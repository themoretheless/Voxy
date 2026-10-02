//! Conservative Cartesian ideal-gas Euler flow with Rusanov face fluxes.
use super::{Error, FiniteDropletGasGrid, VaporCell, finite, positive};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GasGridBoundary {
    Periodic,
    Reflecting,
}
#[derive(Clone, Copy, Debug)]
pub struct GasGridFlowControl {
    pub gas_constant: f64,
    pub courant: f64,
    pub max_substeps: usize,
    pub max_face_updates: usize,
    pub boundaries: [GasGridBoundary; 3],
}
impl Default for GasGridFlowControl {
    fn default() -> Self {
        Self {
            gas_constant: 287.0,
            courant: 0.4,
            max_substeps: 4096,
            max_face_updates: 1_000_000,
            boundaries: [GasGridBoundary::Reflecting; 3],
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GasGridFlowReport {
    pub substeps: usize,
    pub face_updates: usize,
    /// Impulse received by stationary reflecting walls, opposite the gas change.
    pub wall_impulse: [f64; 3],
}
#[derive(Clone, Copy)]
struct State {
    u: [f64; 5],
    velocity: [f64; 3],
    pressure: f64,
    sound: f64,
}
fn state(cell: VaporCell, r: f64, cv: f64) -> Result<State, Error> {
    let density = cell.mass / cell.volume;
    let pressure = density * r * cell.temperature;
    let sound = ((1.0 + r / cv) * r * cell.temperature).sqrt();
    let kinetic = 0.5 * density * cell.velocity.iter().map(|v| v * v).sum::<f64>();
    let u = [
        density,
        density * cell.velocity[0],
        density * cell.velocity[1],
        density * cell.velocity[2],
        density * cv * cell.temperature + kinetic,
    ];
    if !positive(density)
        || !positive(pressure)
        || !positive(sound)
        || u.iter().any(|v| !v.is_finite())
    {
        return Err(Error::NumericalFailure);
    }
    Ok(State {
        u,
        velocity: cell.velocity,
        pressure,
        sound,
    })
}
fn flux(s: State, axis: usize) -> [f64; 5] {
    let mut f = s.u.map(|u| u * s.velocity[axis]);
    f[axis + 1] += s.pressure;
    f[4] += s.pressure * s.velocity[axis];
    f
}
fn riemann(a: State, b: State, axis: usize) -> Result<[f64; 5], Error> {
    let speed = (a.velocity[axis].abs() + a.sound).max(b.velocity[axis].abs() + b.sound);
    let fa = flux(a, axis);
    let fb = flux(b, axis);
    let f = std::array::from_fn(|k| 0.5 * fa[k] + 0.5 * fb[k] - 0.5 * speed * (b.u[k] - a.u[k]));
    if f.iter().any(|v| !v.is_finite()) {
        return Err(Error::NumericalFailure);
    }
    Ok(f)
}
fn reflected(mut s: State, axis: usize) -> State {
    s.velocity[axis] = -s.velocity[axis];
    s.u[axis + 1] = -s.u[axis + 1];
    s
}
impl FiniteDropletGasGrid {
    /// Advances inviscid ideal-gas mass, momentum and total thermal+kinetic energy.
    /// Uniform cv is required; gamma=1+R/cv. Adaptive unsplit CFL substeps use
    /// first-order Rusanov fluxes on all three axes. Periodic faces conserve total
    /// momentum; stationary reflecting walls return their reaction impulse.
    /// No viscosity, conduction, gravity or external heat is applied. Atomic.
    pub fn advance_euler(
        &mut self,
        dt: f64,
        control: GasGridFlowControl,
    ) -> Result<GasGridFlowReport, Error> {
        if !positive(dt)
            || !positive(control.gas_constant)
            || !positive(control.courant)
            || control.courant > 0.4
            || control.max_substeps == 0
            || control.max_face_updates == 0
        {
            return Err(Error::InvalidConfig);
        }
        let cv = self.cells[0].specific_heat_cv;
        if self.cells.iter().any(|c| c.specific_heat_cv != cv) {
            return Err(Error::InvalidConfig);
        }
        let before = self.totals()?;
        let mut candidate = self.clone();
        let mut report = GasGridFlowReport::default();
        let mut remaining = dt;
        let geometric_volume = self.spacing.iter().product::<f64>();
        let stride = [1, self.shape[0], self.shape[0] * self.shape[1]];
        while remaining > 0.0 {
            if report.substeps == control.max_substeps {
                return Err(Error::SubstepBudget);
            }
            let states: Vec<_> = candidate
                .cells
                .iter()
                .map(|c| state(*c, control.gas_constant, cv))
                .collect::<Result<_, _>>()?;
            let rate = states
                .iter()
                .zip(&candidate.cells)
                .map(|(s, cell)| {
                    (0..3)
                        .filter(|&k| {
                            self.shape[k] > 1
                                || control.boundaries[k] == GasGridBoundary::Reflecting
                        })
                        .map(|k| (s.velocity[k].abs() + s.sound) / self.spacing[k])
                        .sum::<f64>()
                        * geometric_volume
                        / cell.volume
                })
                .fold(0.0, f64::max);
            if rate == 0.0 {
                return Ok(report);
            }
            let step = (control.courant / rate).min(remaining);
            if !positive(step) || remaining - step == remaining {
                return Err(Error::NumericalFailure);
            }
            // A constant pressure flux has zero divergence on the closed/periodic
            // grid and zero net wall impulse. Remove it to avoid subtracting large
            // ambient-pressure impulses when resolving small gas velocities.
            let reference_pressure = states[0].pressure;
            let mut next: Vec<_> = states.iter().map(|s| s.u).collect();
            for i in 0..states.len() {
                for axis in 0..3 {
                    let coordinate = (i / stride[axis]) % self.shape[axis];
                    let area_time = step * geometric_volume / self.spacing[axis];
                    let factor = area_time / self.cells[i].volume;
                    let neighbor = if coordinate + 1 < self.shape[axis] {
                        Some(i + stride[axis])
                    } else if control.boundaries[axis] == GasGridBoundary::Periodic
                        && self.shape[axis] > 1
                    {
                        Some(i - (self.shape[axis] - 1) * stride[axis])
                    } else {
                        None
                    };
                    if let Some(j) = neighbor {
                        charge(&mut report, control.max_face_updates)?;
                        let mut f = riemann(states[i], states[j], axis)?;
                        f[axis + 1] -= reference_pressure;
                        for k in 0..5 {
                            next[i][k] -= factor * f[k];
                            next[j][k] += area_time / self.cells[j].volume * f[k];
                        }
                    } else if coordinate + 1 == self.shape[axis]
                        && control.boundaries[axis] == GasGridBoundary::Reflecting
                    {
                        charge(&mut report, control.max_face_updates)?;
                        let mut f = riemann(states[i], reflected(states[i], axis), axis)?;
                        f[axis + 1] -= reference_pressure;
                        for k in 0..5 {
                            next[i][k] -= factor * f[k];
                        }
                        for k in 0..3 {
                            report.wall_impulse[k] += area_time * f[k + 1];
                        }
                    }
                    if coordinate == 0 && control.boundaries[axis] == GasGridBoundary::Reflecting {
                        charge(&mut report, control.max_face_updates)?;
                        let mut f = riemann(reflected(states[i], axis), states[i], axis)?;
                        f[axis + 1] -= reference_pressure;
                        for k in 0..5 {
                            next[i][k] += factor * f[k];
                        }
                        for k in 0..3 {
                            report.wall_impulse[k] -= area_time * f[k + 1];
                        }
                    }
                }
            }
            for (cell, u) in candidate.cells.iter_mut().zip(next) {
                if u.iter().any(|v| !v.is_finite()) || !positive(u[0]) {
                    return Err(Error::NumericalFailure);
                }
                let velocity = [u[1] / u[0], u[2] / u[0], u[3] / u[0]];
                let internal = u[4] - 0.5 * u[0] * velocity.iter().map(|v| v * v).sum::<f64>();
                let temperature = internal / u[0] / cv;
                let mass = u[0] * cell.volume;
                if !positive(temperature) || !positive(mass) || !finite(velocity) {
                    return Err(Error::NumericalFailure);
                }
                cell.mass = mass;
                cell.velocity = velocity;
                cell.temperature = temperature;
                cell.energy(1.0)?;
            }
            report.substeps += 1;
            remaining = (remaining - step).max(0.0);
        }
        let after = candidate.totals()?;
        let energy = before.thermal_energy + before.kinetic_energy;
        if (after.mass - before.mass).abs() > 2048.0 * f64::EPSILON * before.mass.max(1.0)
            || (after.thermal_energy + after.kinetic_energy - energy).abs()
                > 2048.0 * f64::EPSILON * energy.max(1.0)
            || !finite(report.wall_impulse)
            || (0..3).any(|k| {
                (after.momentum[k] + report.wall_impulse[k] - before.momentum[k]).abs()
                    > 2048.0
                        * f64::EPSILON
                        * (before.momentum[k].abs() + report.wall_impulse[k].abs()).max(1.0)
            })
        {
            return Err(Error::NumericalFailure);
        }
        *self = candidate;
        Ok(report)
    }
}
fn charge(report: &mut GasGridFlowReport, max: usize) -> Result<(), Error> {
    if report.face_updates == max {
        return Err(Error::PairBudget);
    }
    report.face_updates += 1;
    Ok(())
}
