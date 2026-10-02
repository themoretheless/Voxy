//! Finite-volume viscous exchange with prescribed planar box walls.
use super::{
    Error, FiniteDropletGasGrid, GasGridViscosityBoundary, GasGridViscosityControl,
    GasGridViscosityReport, finite, positive,
};

#[derive(Clone, Copy, Debug)]
pub struct GasGridWallViscosityControl {
    pub shear_viscosity: f64,
    pub bulk_viscosity: f64,
    /// Prescribed uniform tangential velocities, x-/x+/y-/y+/z-/z+.
    /// None disables a face. Normal wall motion requires moving geometry and
    /// is rejected. Wall inertia and wall temperature are not evolved.
    pub walls: [Option<[f64; 3]>; 6],
    pub max_relaxation_number: f64,
    pub max_substeps: usize,
    pub max_face_steps: usize,
}
impl Default for GasGridWallViscosityControl {
    fn default() -> Self {
        Self {
            shear_viscosity: 1.8e-5,
            bulk_viscosity: 0.0,
            walls: [Some([0.0; 3]); 6],
            max_relaxation_number: 0.25,
            max_substeps: 4096,
            max_face_steps: 1_000_000,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GasGridWallViscosityReport {
    pub substeps: usize,
    pub face_steps: usize,
    /// Impulse received by the prescribed walls, opposite the gas change.
    pub wall_impulse: [f64; 3],
    pub face_impulses: [[f64; 3]; 6],
    /// Mechanical energy supplied to the gas by prescribed tangential motion.
    pub work_on_gas: f64,
    /// Viscous conversion to internal heat; not additional external energy.
    pub dissipated_heat: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GasGridViscousReport {
    pub interior: GasGridViscosityReport,
    pub walls: GasGridWallViscosityReport,
}
impl FiniteDropletGasGrid {
    /// Atomic wall-half/interior-full/wall-half composition with a shared pair
    /// of viscosity coefficients. Enabled walls cannot lie on periodic axes.
    /// Wall budgets cover both half stages together; interior budgets cover
    /// the full interior stage. Reports separate wall work from internal heat.
    pub fn relax_viscosity_with_walls(
        &mut self,
        dt: f64,
        interior: GasGridViscosityControl,
        walls: GasGridWallViscosityControl,
    ) -> Result<GasGridViscousReport, Error> {
        if interior.shear_viscosity != walls.shear_viscosity
            || interior.bulk_viscosity != walls.bulk_viscosity
            || (0..6).any(|face| {
                walls.walls[face].is_some()
                    && interior.boundaries[face / 2] == GasGridViscosityBoundary::Periodic
            })
        {
            return Err(Error::InvalidConfig);
        }
        let mut candidate = self.clone();
        let first = candidate.relax_wall_viscosity(0.5 * dt, walls)?;
        let stress = candidate.relax_viscosity(dt, interior)?;
        let mut remainder = walls;
        remainder.max_substeps -= first.substeps;
        remainder.max_face_steps -= first.face_steps;
        if remainder.max_substeps == 0 {
            return Err(Error::SubstepBudget);
        }
        if remainder.max_face_steps == 0 {
            return Err(Error::PairBudget);
        }
        let second = candidate.relax_wall_viscosity(0.5 * dt, remainder)?;
        let mut combined = first;
        combined.substeps += second.substeps;
        combined.face_steps += second.face_steps;
        combined.work_on_gas += second.work_on_gas;
        combined.dissipated_heat += second.dissipated_heat;
        for k in 0..3 {
            combined.wall_impulse[k] += second.wall_impulse[k];
            for face in 0..6 {
                combined.face_impulses[face][k] += second.face_impulses[face][k];
            }
        }
        if !combined.work_on_gas.is_finite()
            || !combined.dissipated_heat.is_finite()
            || !finite(combined.wall_impulse)
            || combined.face_impulses.iter().any(|v| !finite(*v))
        {
            return Err(Error::NumericalFailure);
        }
        *self = candidate;
        Ok(GasGridViscousReport {
            interior: stress,
            walls: combined,
        })
    }
    /// Relaxes gas velocities toward selected box wall velocities. Each face
    /// uses center-to-wall distance h/2, tangential conductance mu*A/(h/2),
    /// and normal conductance (4mu/3+bulk)*A/(h/2). The uniform planar wall
    /// condition supplies no tangential derivatives of prescribed wall velocity.
    /// Exact face exchanges run in symmetric forward/reverse half sweeps.
    /// Their nonnegative viscous heat is deposited in the adjacent gas cell.
    /// Total gas energy changes by reported wall work; gas plus wall impulse
    /// is conserved. Mass, geometry and cv are fixed. Whole-grid atomic.
    /// Use with the interior stress operator; this does not move walls or cells.
    pub fn relax_wall_viscosity(
        &mut self,
        dt: f64,
        control: GasGridWallViscosityControl,
    ) -> Result<GasGridWallViscosityReport, Error> {
        if !positive(dt)
            || !control.shear_viscosity.is_finite()
            || control.shear_viscosity < 0.0
            || !control.bulk_viscosity.is_finite()
            || control.bulk_viscosity < 0.0
            || !positive(control.max_relaxation_number)
            || control.max_substeps == 0
            || control.max_face_steps == 0
            || control
                .walls
                .iter()
                .enumerate()
                .any(|(face, wall)| wall.is_some_and(|v| !finite(v) || v[face / 2] != 0.0))
        {
            return Err(Error::InvalidConfig);
        }
        if control.shear_viscosity == 0.0 && control.bulk_viscosity == 0.0 {
            return Ok(Default::default());
        }
        let before = self.totals()?;
        let volume = self.spacing.iter().product::<f64>();
        let stride = [1, self.shape[0], self.shape[0] * self.shape[1]];
        let normal = (4.0 / 3.0) * control.shear_viscosity + control.bulk_viscosity;
        if !normal.is_finite() {
            return Err(Error::NumericalFailure);
        }
        let mut faces = Vec::new();
        let mut rates = vec![[0.0; 3]; self.cells.len()];
        for i in 0..self.cells.len() {
            for face in 0..6 {
                let axis = face / 2;
                let coordinate = (i / stride[axis]) % self.shape[axis];
                let boundary = if face % 2 == 0 {
                    coordinate == 0
                } else {
                    coordinate + 1 == self.shape[axis]
                };
                if !boundary || control.walls[face].is_none() {
                    continue;
                }
                if faces.len() >= control.max_face_steps / 2 {
                    return Err(Error::PairBudget);
                }
                let rate: [f64; 3] = std::array::from_fn(|k| {
                    let viscosity = if k == axis {
                        normal
                    } else {
                        control.shear_viscosity
                    };
                    (viscosity / self.spacing[axis]) * (volume / self.spacing[axis]) * 2.0
                        / self.cells[i].mass
                });
                if rate.iter().any(|v| !v.is_finite() || *v < 0.0) {
                    return Err(Error::NumericalFailure);
                }
                for k in 0..3 {
                    rates[i][k] += rate[k];
                }
                faces.push((i, face, rate));
            }
        }
        let max_rate = rates.iter().flatten().copied().fold(0.0, f64::max);
        if !max_rate.is_finite() {
            return Err(Error::NumericalFailure);
        }
        if max_rate == 0.0 {
            return Ok(Default::default());
        }
        let mut candidate = self.clone();
        let mut report = GasGridWallViscosityReport::default();
        let mut remaining = dt;
        while remaining > 0.0 {
            if report.substeps == control.max_substeps {
                return Err(Error::SubstepBudget);
            }
            let step = remaining.min(control.max_relaxation_number / max_rate);
            if !positive(step) || remaining - step == remaining {
                return Err(Error::NumericalFailure);
            }
            for &(i, face, rate) in faces.iter().chain(faces.iter().rev()) {
                if report.face_steps == control.max_face_steps {
                    return Err(Error::PairBudget);
                }
                let wall = control.walls[face].ok_or(Error::InvalidConfig)?;
                let cell = &mut candidate.cells[i];
                let mut heat = 0.0;
                for k in 0..3 {
                    if rate[k] == 0.0 {
                        continue;
                    }
                    let relative = cell.velocity[k] - wall[k];
                    let exchange = -(-rate[k] * (0.5 * step)).exp_m1();
                    let heat_fraction = -(-rate[k] * step).exp_m1();
                    let delta = -relative * exchange;
                    let impulse = cell.mass * delta;
                    heat += (0.5 * cell.mass * relative) * relative * heat_fraction;
                    cell.velocity[k] += delta;
                    report.wall_impulse[k] -= impulse;
                    report.face_impulses[face][k] -= impulse;
                    report.work_on_gas += wall[k] * impulse;
                }
                if !heat.is_finite() || heat < 0.0 || !finite(cell.velocity) {
                    return Err(Error::NumericalFailure);
                }
                cell.temperature += heat / cell.mass / cell.specific_heat_cv;
                if !positive(cell.temperature) {
                    return Err(Error::NumericalFailure);
                }
                report.dissipated_heat += heat;
                report.face_steps += 1;
            }
            remaining = (remaining - step).max(0.0);
            report.substeps += 1;
        }
        let after = candidate.totals()?;
        let mechanical_scale =
            (before.kinetic_energy + report.work_on_gas.abs() + report.dissipated_heat).max(1.0);
        let energy_scale = (before.thermal_energy + mechanical_scale).max(1.0);
        let momentum_scale = self
            .cells
            .iter()
            .map(|c| c.mass * c.velocity.iter().map(|v| v.abs()).sum::<f64>())
            .sum::<f64>()
            + report.wall_impulse.iter().map(|v| v.abs()).sum::<f64>();
        if !energy_scale.is_finite()
            || !mechanical_scale.is_finite()
            || !momentum_scale.is_finite()
            || !report.work_on_gas.is_finite()
            || !report.dissipated_heat.is_finite()
            || !finite(report.wall_impulse)
            || report.face_impulses.iter().any(|v| !finite(*v))
            || (after.kinetic_energy - before.kinetic_energy + report.dissipated_heat
                - report.work_on_gas)
                .abs()
                > 4096.0 * f64::EPSILON * mechanical_scale
            || (after.kinetic_energy + after.thermal_energy
                - before.kinetic_energy
                - before.thermal_energy
                - report.work_on_gas)
                .abs()
                > 4096.0 * f64::EPSILON * energy_scale
            || (0..3).any(|k| {
                (after.momentum[k] - before.momentum[k] + report.wall_impulse[k]).abs()
                    > 4096.0 * f64::EPSILON * momentum_scale.max(1.0)
            })
        {
            return Err(Error::NumericalFailure);
        }
        *self = candidate;
        Ok(report)
    }
}
