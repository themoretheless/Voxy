//! Spatially selected finite-cell gas feedback; gas transport is separate.
use super::{Error, FiniteDropletDragReport, Liquid, VaporCell, finite, norm, positive};
#[derive(Clone, Debug, PartialEq)]
pub struct FiniteDropletGasGrid {
    pub(super) origin: [f64; 3],
    pub(super) spacing: [f64; 3],
    pub(super) shape: [usize; 3],
    pub(super) cells: Vec<VaporCell>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GasGridTotals {
    pub mass: f64,
    pub volume: f64,
    pub momentum: [f64; 3],
    pub kinetic_energy: f64,
    pub thermal_energy: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpatialDropletDragReport {
    pub drag: FiniteDropletDragReport,
    pub affected_cells: usize,
    pub outside_droplets: usize,
}
impl FiniteDropletGasGrid {
    /// Cartesian cells in x-fastest order. Origin is included, far box faces excluded.
    /// Every supplied cell volume must match spacing product. At most 1,000,000 cells.
    pub fn new(
        origin: [f64; 3],
        spacing: [f64; 3],
        shape: [usize; 3],
        cells: Vec<VaporCell>,
    ) -> Result<Self, Error> {
        let count = shape
            .iter()
            .try_fold(1usize, |a, &b| a.checked_mul(b))
            .ok_or(Error::InvalidConfig)?;
        let volume = spacing.iter().product::<f64>();
        if !finite(origin)
            || spacing.iter().any(|v| !positive(*v))
            || !positive(volume)
            || count == 0
            || count > 1_000_000
            || cells.len() != count
        {
            return Err(Error::InvalidConfig);
        }
        for k in 0..3 {
            let upper = origin[k] + spacing[k] * shape[k] as f64;
            if !upper.is_finite() || upper <= origin[k] {
                return Err(Error::InvalidConfig);
            }
            for i in 0..shape[k] {
                if origin[k] + spacing[k] * (i + 1) as f64 <= origin[k] + spacing[k] * i as f64 {
                    return Err(Error::InvalidConfig);
                }
            }
        }
        for cell in &cells {
            if !positive(cell.mass)
                || !positive(cell.temperature)
                || !positive(cell.specific_heat_cv)
                || !finite(cell.velocity)
                || (cell.volume - volume).abs() > 1e-12 * volume
                || !positive(cell.volume)
            {
                return Err(Error::InvalidConfig);
            }
            cell.energy(1.0)?;
        }
        let grid = Self {
            origin,
            spacing,
            shape,
            cells,
        };
        grid.totals()?;
        Ok(grid)
    }
    pub fn cells(&self) -> &[VaporCell] {
        &self.cells
    }
    pub fn origin(&self) -> [f64; 3] {
        self.origin
    }
    pub fn spacing(&self) -> [f64; 3] {
        self.spacing
    }
    pub fn shape(&self) -> [usize; 3] {
        self.shape
    }
    /// Containing center cell; outside the finite gas region means no gas exchange.
    pub fn cell_index(&self, point: [f64; 3]) -> Result<Option<usize>, Error> {
        if !finite(point) {
            return Err(Error::InvalidParticle);
        }
        let mut ijk = [0usize; 3];
        for k in 0..3 {
            let upper = self.origin[k] + self.spacing[k] * self.shape[k] as f64;
            if point[k] < self.origin[k] || point[k] >= upper {
                return Ok(None);
            }
            let coordinate = (point[k] - self.origin[k]) / self.spacing[k];
            if !coordinate.is_finite() {
                return Err(Error::NumericalFailure);
            }
            ijk[k] = (coordinate.floor() as usize).min(self.shape[k] - 1);
        }
        Ok(Some(
            ijk[0] + self.shape[0] * (ijk[1] + self.shape[1] * ijk[2]),
        ))
    }
    pub fn totals(&self) -> Result<GasGridTotals, Error> {
        let mut totals = GasGridTotals::default();
        for c in &self.cells {
            totals.mass += c.mass;
            totals.volume += c.volume;
            totals.kinetic_energy += 0.5 * c.mass * norm(c.velocity).powi(2);
            totals.thermal_energy += c.mass * c.specific_heat_cv * c.temperature;
            for k in 0..3 {
                totals.momentum[k] += c.mass * c.velocity[k];
            }
        }
        if !finite(totals.momentum)
            || [
                totals.mass,
                totals.volume,
                totals.kinetic_energy,
                totals.thermal_energy,
            ]
            .iter()
            .any(|v| !v.is_finite())
        {
            return Err(Error::NumericalFailure);
        }
        Ok(totals)
    }
}
impl Liquid {
    /// Symmetric reduced-mass drag with the gas cell containing each marked center.
    /// Membership is frozen for this drag stage; unmarked and outside drops stay
    /// unchanged. No gas flow between cells, face interpolation or sphere-volume
    /// overlap is inferred. Positions and both masses remain fixed. Atomic over
    /// the complete liquid and every gas cell, including late heating failures.
    pub fn exchange_marked_droplet_drag_grid(
        &mut self,
        dt: f64,
        grid: &mut FiniteDropletGasGrid,
        radii: &[f64],
        cd: f64,
    ) -> Result<SpatialDropletDragReport, Error> {
        let flags = self
            .droplet_population
            .as_ref()
            .ok_or(Error::InvalidConfig)?;
        if !positive(dt)
            || radii.len() != self.particles.len()
            || radii.iter().any(|r| !positive(*r))
            || !cd.is_finite()
            || cd < 0.0
        {
            return Err(Error::InvalidConfig);
        }
        let mut selected = Vec::new();
        let mut report = SpatialDropletDragReport::default();
        let mut affected = vec![false; grid.cells.len()];
        for (i, p) in self.particles.iter().enumerate() {
            if !flags[i] {
                continue;
            }
            if let Some(cell) = grid.cell_index(p.position)? {
                selected.push((i, cell));
                affected[cell] = true;
            } else {
                report.outside_droplets += 1;
            }
        }
        report.drag.pair_steps = selected.len().checked_mul(2).ok_or(Error::PairBudget)?;
        if report.drag.pair_steps > self.config.max_neighbor_checks {
            return Err(Error::PairBudget);
        }
        report.affected_cells = affected.iter().filter(|v| **v).count();
        let mut candidate = self.clone();
        let mut cells = grid.cells.clone();
        let mut heat = vec![0.0; cells.len()];
        for &(i, cell) in selected.iter().chain(selected.iter().rev()) {
            let (impulse, dissipation) = super::droplet_finite_gas::finite_pair(
                &mut candidate.particles[i],
                &mut cells[cell],
                0.5 * dt,
                radii[i],
                cd,
            )?;
            heat[cell] += dissipation;
            report.drag.dissipated_heat += dissipation;
            for k in 0..3 {
                report.drag.gas_impulse[k] += impulse[k];
            }
        }
        let mut gas_scale = 0.0;
        for i in 0..cells.len() {
            if !affected[i] {
                continue;
            }
            let old = grid.cells[i];
            let cell = &mut cells[i];
            let capacity = cell.mass * cell.specific_heat_cv;
            if !positive(capacity) || !heat[i].is_finite() {
                return Err(Error::NumericalFailure);
            }
            cell.temperature += heat[i] / capacity;
            if !positive(cell.temperature) || (heat[i] > 0.0 && cell.temperature == old.temperature)
            {
                return Err(Error::NumericalFailure);
            }
            report.drag.gas_kinetic_energy_change += 0.5
                * cell.mass
                * (0..3)
                    .map(|k| {
                        (cell.velocity[k] + old.velocity[k]) * (cell.velocity[k] - old.velocity[k])
                    })
                    .sum::<f64>();
            gas_scale += 0.5 * old.mass * norm(old.velocity).powi(2);
            cell.energy(1.0)?;
        }
        let liquid_change: f64 = selected
            .iter()
            .map(|&(i, _)| {
                let a = self.particles[i];
                let b = candidate.particles[i];
                0.5 * a.mass
                    * (0..3)
                        .map(|k| (a.velocity[k] + b.velocity[k]) * (b.velocity[k] - a.velocity[k]))
                        .sum::<f64>()
            })
            .sum();
        let scale = gas_scale
            + selected
                .iter()
                .map(|&(i, _)| {
                    let p = self.particles[i];
                    0.5 * p.mass * norm(p.velocity).powi(2)
                })
                .sum::<f64>();
        if !finite(report.drag.gas_impulse)
            || !report.drag.dissipated_heat.is_finite()
            || !report.drag.gas_kinetic_energy_change.is_finite()
            || !liquid_change.is_finite()
            || !scale.is_finite()
            || (liquid_change + report.drag.dissipated_heat + report.drag.gas_kinetic_energy_change)
                .abs()
                > 512.0 * f64::EPSILON * scale.max(1.0)
        {
            return Err(Error::NumericalFailure);
        }
        candidate.effective_materials()?;
        let staged = FiniteDropletGasGrid {
            cells,
            origin: grid.origin,
            spacing: grid.spacing,
            shape: grid.shape,
        };
        staged.totals()?;
        *self = candidate;
        *grid = staged;
        Ok(report)
    }
}
