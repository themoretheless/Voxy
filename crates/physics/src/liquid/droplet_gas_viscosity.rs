//! Dissipative Newtonian stress relaxation on Cartesian gas cells.
use super::{Error, FiniteDropletGasGrid, finite, positive};

/// Boundaries of the viscous stress operator, independent of Euler wall fluxes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GasGridViscosityBoundary {
    Periodic,
    /// No external viscous force or work. This supplies neither no-slip nor
    /// impermeability; an Euler wall operator must impose the latter separately.
    TractionFree,
}
#[derive(Clone, Copy, Debug)]
pub struct GasGridViscosityControl {
    /// Constant dynamic shear viscosity, Pa s.
    pub shear_viscosity: f64,
    /// Constant bulk viscosity, Pa s; zero uses the Stokes hypothesis.
    pub bulk_viscosity: f64,
    /// Accuracy cap on dt times a bound for the assembled relaxation operator.
    pub max_relaxation_number: f64,
    pub max_substeps: usize,
    pub max_block_steps: usize,
    /// Viscous traction boundaries. No no-slip or moving wall is supplied here.
    pub boundaries: [GasGridViscosityBoundary; 3],
}
impl Default for GasGridViscosityControl {
    fn default() -> Self {
        Self {
            shear_viscosity: 1.8e-5,
            bulk_viscosity: 0.0,
            max_relaxation_number: 0.25,
            max_substeps: 4096,
            max_block_steps: 1_000_000,
            boundaries: [GasGridViscosityBoundary::TractionFree; 3],
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GasGridViscosityReport {
    pub substeps: usize,
    pub block_steps: usize,
    /// Kinetic energy converted to internal heat, not external energy input.
    pub dissipated_heat: f64,
}
type Matrix = [[f64; 12]; 12];

fn direction(key: usize, axis: usize) -> f64 {
    match (key / 3usize.pow(axis as u32)) % 3 {
        1 => 1.0,
        2 => -1.0,
        _ => 0.0,
    }
}
fn gradient(v: [[f64; 3]; 4], mask: usize, spacing: [f64; 3]) -> [[f64; 3]; 3] {
    let mut g = [[0.0; 3]; 3];
    for axis in 0..3 {
        if direction(mask, axis) != 0.0 {
            for component in 0..3 {
                g[component][axis] = direction(mask, axis)
                    * (v[axis + 1][component] - v[0][component])
                    / spacing[axis];
            }
        }
    }
    g
}
fn stress(g: [[f64; 3]; 3], mu: f64, bulk: f64) -> ([[f64; 3]; 3], f64) {
    let divergence = g[0][0] + g[1][1] + g[2][2];
    let mut tau = [[0.0; 3]; 3];
    let mut dissipation = bulk * divergence * divergence;
    for i in 0..3 {
        for j in 0..3 {
            let dev = 0.5 * (g[i][j] + g[j][i]) - if i == j { divergence / 3.0 } else { 0.0 };
            tau[i][j] = 2.0 * mu * dev + if i == j { bulk * divergence } else { 0.0 };
            dissipation += 2.0 * mu * dev * dev;
        }
    }
    (tau, dissipation)
}
fn stiffness(mask: usize, spacing: [f64; 3], volume: f64, mu: f64, bulk: f64) -> Matrix {
    let mut k = [[0.0; 12]; 12];
    for column in 0..12 {
        let mut unit = [[0.0; 3]; 4];
        unit[column / 3][column % 3] = 1.0;
        let (tau, _) = stress(gradient(unit, mask, spacing), mu, bulk);
        for axis in 0..3 {
            if direction(mask, axis) != 0.0 {
                for component in 0..3 {
                    let force =
                        direction(mask, axis) * volume * tau[component][axis] / spacing[axis];
                    k[component][column] -= force;
                    k[3 * (axis + 1) + component][column] += force;
                }
            }
        }
    }
    k
}
fn links(
    grid: &FiniteDropletGasGrid,
    i: usize,
    boundary: [GasGridViscosityBoundary; 3],
) -> ([usize; 4], usize) {
    let stride = [1, grid.shape[0], grid.shape[0] * grid.shape[1]];
    let mut nodes = [i; 4];
    let mut mask = 0;
    for axis in 0..3 {
        let coordinate = (i / stride[axis]) % grid.shape[axis];
        let j = if coordinate + 1 < grid.shape[axis] {
            Some(i + stride[axis])
        } else if grid.shape[axis] > 1 && boundary[axis] == GasGridViscosityBoundary::Periodic {
            Some(i - (grid.shape[axis] - 1) * stride[axis])
        } else if grid.shape[axis] > 1 {
            Some(i - stride[axis])
        } else {
            None
        };
        if let Some(j) = j {
            nodes[axis + 1] = j;
            let backward = coordinate + 1 == grid.shape[axis]
                && boundary[axis] == GasGridViscosityBoundary::TractionFree;
            mask += (if backward { 2 } else { 1 }) * 3usize.pow(axis as u32);
        }
    }
    (nodes, mask)
}
fn solve(mut a: Matrix, mut rhs: [f64; 12]) -> Result<[f64; 12], Error> {
    // Cholesky factorization of I + dt/2 M^-1/2 K M^-1/2.
    for i in 0..12 {
        for j in 0..=i {
            let sum = a[i][j] - (0..j).map(|k| a[i][k] * a[j][k]).sum::<f64>();
            a[i][j] = if i == j {
                if !positive(sum) {
                    return Err(Error::NumericalFailure);
                }
                sum.sqrt()
            } else {
                sum / a[j][j]
            };
        }
    }
    for i in 0..12 {
        rhs[i] = (rhs[i] - (0..i).map(|j| a[i][j] * rhs[j]).sum::<f64>()) / a[i][i];
    }
    for i in (0..12).rev() {
        rhs[i] = (rhs[i] - (i + 1..12).map(|j| a[j][i] * rhs[j]).sum::<f64>()) / a[i][i];
    }
    if rhs.iter().any(|v| !v.is_finite()) {
        return Err(Error::NumericalFailure);
    }
    Ok(rhs)
}
impl FiniteDropletGasGrid {
    /// Relaxes the full symmetric deviatoric Newtonian stress and optional bulk
    /// stress. One-sided Cartesian velocity gradients define a positive strain
    /// dissipation functional; its negative adjoint supplies conservative forces.
    /// Four-cell blocks use implicit midpoint, composed forward/reverse half steps.
    /// Each block deposits its nonnegative viscous heat in its gradient cell.
    /// Mass, volume and cv stay fixed. Atomic, including late budget failures.
    /// The block time splitting and one-sided mixed derivatives require refinement.
    pub fn relax_viscosity(
        &mut self,
        dt: f64,
        control: GasGridViscosityControl,
    ) -> Result<GasGridViscosityReport, Error> {
        if !positive(dt)
            || !control.shear_viscosity.is_finite()
            || control.shear_viscosity < 0.0
            || !control.bulk_viscosity.is_finite()
            || control.bulk_viscosity < 0.0
            || !positive(control.max_relaxation_number)
            || control.max_substeps == 0
            || control.max_block_steps == 0
        {
            return Err(Error::InvalidConfig);
        }
        if control.shear_viscosity == 0.0 && control.bulk_viscosity == 0.0 {
            return Ok(Default::default());
        }
        let before = self.totals()?;
        let volume = self.spacing.iter().product::<f64>();
        let matrices: [Matrix; 27] = std::array::from_fn(|mask| {
            stiffness(
                mask,
                self.spacing,
                volume,
                control.shear_viscosity,
                control.bulk_viscosity,
            )
        });
        if matrices.iter().flatten().flatten().any(|v| !v.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        let mut rates = vec![[0.0; 3]; self.cells.len()];
        let mut blocks = 0;
        for i in 0..self.cells.len() {
            let (nodes, mask) = links(self, i, control.boundaries);
            if mask == 0 {
                continue;
            }
            blocks += 1;
            if blocks > control.max_block_steps / 2 {
                return Err(Error::PairBudget);
            }
            for row in 0..12 {
                if row / 3 > 0 && direction(mask, row / 3 - 1) == 0.0 {
                    continue;
                }
                rates[nodes[row / 3]][row % 3] +=
                    matrices[mask][row].iter().map(|v| v.abs()).sum::<f64>()
                        / self.cells[nodes[row / 3]].mass;
            }
        }
        if blocks == 0 {
            return Ok(Default::default());
        }
        let rate = rates.iter().flatten().copied().fold(0.0, f64::max);
        if !rate.is_finite() {
            return Err(Error::NumericalFailure);
        }
        if rate == 0.0 {
            return Ok(Default::default());
        }
        let mut candidate = self.clone();
        let mut remaining = dt;
        let mut report = GasGridViscosityReport::default();
        while remaining > 0.0 {
            if report.substeps == control.max_substeps {
                return Err(Error::SubstepBudget);
            }
            let step = remaining.min(control.max_relaxation_number / rate);
            if !positive(step) || remaining - step == remaining {
                return Err(Error::NumericalFailure);
            }
            let indices = (0..self.cells.len()).chain((0..self.cells.len()).rev());
            for i in indices {
                let (nodes, mask) = links(&candidate, i, control.boundaries);
                if mask == 0 {
                    continue;
                }
                if report.block_steps == control.max_block_steps {
                    return Err(Error::PairBudget);
                }
                let v: [[f64; 3]; 4] =
                    std::array::from_fn(|node| candidate.cells[nodes[node]].velocity);
                let root_mass: [f64; 4] =
                    std::array::from_fn(|node| candidate.cells[nodes[node]].mass.sqrt());
                let (tau, _) = stress(
                    gradient(v, mask, self.spacing),
                    control.shear_viscosity,
                    control.bulk_viscosity,
                );
                let mut force = [[0.0; 3]; 4];
                for axis in 0..3 {
                    if direction(mask, axis) != 0.0 {
                        for component in 0..3 {
                            let f = direction(mask, axis) * volume * tau[component][axis]
                                / self.spacing[axis];
                            force[0][component] += f;
                            force[axis + 1][component] -= f;
                        }
                    }
                }
                let half = 0.5 * step;
                let a = std::array::from_fn(|row| {
                    std::array::from_fn(|col| {
                        (if row == col { 1.0 } else { 0.0 })
                            + 0.5 * half * matrices[mask][row][col]
                                / root_mass[row / 3]
                                / root_mass[col / 3]
                    })
                });
                let rhs =
                    std::array::from_fn(|row| half * force[row / 3][row % 3] / root_mass[row / 3]);
                let delta = solve(a, rhs)?;
                let midpoint = std::array::from_fn(|node| {
                    std::array::from_fn(|component| {
                        v[node][component] + 0.5 * delta[3 * node + component] / root_mass[node]
                    })
                });
                let (_, density_heat) = stress(
                    gradient(midpoint, mask, self.spacing),
                    control.shear_viscosity,
                    control.bulk_viscosity,
                );
                let heat = half * volume * density_heat;
                if !heat.is_finite() || heat < 0.0 {
                    return Err(Error::NumericalFailure);
                }
                for node in 0..4 {
                    if node > 0 && direction(mask, node - 1) == 0.0 {
                        continue;
                    }
                    let velocity = std::array::from_fn(|component| {
                        v[node][component] + delta[3 * node + component] / root_mass[node]
                    });
                    if !finite(velocity) {
                        return Err(Error::NumericalFailure);
                    }
                    candidate.cells[nodes[node]].velocity = velocity;
                }
                let cell = &mut candidate.cells[i];
                cell.temperature += heat / cell.mass / cell.specific_heat_cv;
                if !positive(cell.temperature) {
                    return Err(Error::NumericalFailure);
                }
                report.dissipated_heat += heat;
                report.block_steps += 1;
            }
            remaining = (remaining - step).max(0.0);
            report.substeps += 1;
        }
        let after = candidate.totals()?;
        let energy_scale = (before.thermal_energy + before.kinetic_energy).max(1.0);
        let momentum_scale = self
            .cells
            .iter()
            .map(|c| c.mass * c.velocity.iter().map(|v| v.abs()).sum::<f64>())
            .sum::<f64>()
            .max(1.0);
        if !report.dissipated_heat.is_finite()
            || (after.kinetic_energy + after.thermal_energy
                - before.kinetic_energy
                - before.thermal_energy)
                .abs()
                > 4096.0 * f64::EPSILON * energy_scale
            || (after.kinetic_energy - before.kinetic_energy + report.dissipated_heat).abs()
                > 4096.0 * f64::EPSILON * before.kinetic_energy.max(1.0)
            || (0..3).any(|k| {
                (after.momentum[k] - before.momentum[k]).abs()
                    > 4096.0 * f64::EPSILON * momentum_scale
            })
        {
            return Err(Error::NumericalFailure);
        }
        *self = candidate;
        Ok(report)
    }
}
