//! Conservative Fourier heat conduction on finite Cartesian gas cells.
use super::{Error, FiniteDropletGasGrid, GasGridBoundary, positive};
#[derive(Clone, Copy, Debug)]
pub struct GasGridHeatControl {
    /// W/(m K), prescribed constant nonnegative conductivity.
    pub conductivity: f64,
    /// Accuracy limit on step*sum(face conductance)/cell heat capacity.
    pub max_exchange_number: f64,
    pub max_substeps: usize,
    pub max_pair_steps: usize,
    /// Reflecting boundaries are insulated for this thermal operator.
    pub boundaries: [GasGridBoundary; 3],
}
impl Default for GasGridHeatControl {
    fn default() -> Self {
        Self {
            conductivity: 0.026,
            max_exchange_number: 0.25,
            max_substeps: 4096,
            max_pair_steps: 1_000_000,
            boundaries: [GasGridBoundary::Reflecting; 3],
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GasGridHeatReport {
    pub substeps: usize,
    pub pair_steps: usize,
    /// Sum of the magnitudes of modeled internal pair heat exchanges (J).
    /// This is redistribution, not external heat added to the gas.
    pub absolute_transferred_heat: f64,
}
impl FiniteDropletGasGrid {
    /// Evolves Fourier conduction with face G=kappa*A/distance and actual m*cv.
    /// Exact capacitive pair exchanges are composed in symmetric forward/reverse
    /// half steps. Temperatures remain within the original range; full conduction
    /// accuracy still requires time and mesh refinement. Mass, velocity and cv
    /// stay fixed. No boundary heat is supplied. Atomic over every gas cell.
    pub fn conduct_heat(
        &mut self,
        dt: f64,
        control: GasGridHeatControl,
    ) -> Result<GasGridHeatReport, Error> {
        if !positive(dt)
            || !control.conductivity.is_finite()
            || control.conductivity < 0.0
            || !positive(control.max_exchange_number)
            || control.max_substeps == 0
            || control.max_pair_steps == 0
        {
            return Err(Error::InvalidConfig);
        }
        if control.conductivity == 0.0 {
            return Ok(Default::default());
        }
        let before = self.totals()?;
        let capacities: Vec<_> = self
            .cells
            .iter()
            .map(|c| c.mass * c.specific_heat_cv)
            .collect();
        if capacities.iter().any(|v| !positive(*v)) {
            return Err(Error::NumericalFailure);
        }
        let volume = self.spacing.iter().product::<f64>();
        let stride = [1, self.shape[0], self.shape[0] * self.shape[1]];
        let mut edges = Vec::new();
        let mut rates = vec![0.0; self.cells.len()];
        for i in 0..self.cells.len() {
            for axis in 0..3 {
                let coordinate = (i / stride[axis]) % self.shape[axis];
                let j = if coordinate + 1 < self.shape[axis] {
                    Some(i + stride[axis])
                } else if control.boundaries[axis] == GasGridBoundary::Periodic
                    && self.shape[axis] > 1
                {
                    Some(i - (self.shape[axis] - 1) * stride[axis])
                } else {
                    None
                };
                if let Some(j) = j {
                    let conductance =
                        (control.conductivity / self.spacing[axis]) * (volume / self.spacing[axis]);
                    if !positive(conductance) {
                        return Err(Error::NumericalFailure);
                    }
                    if edges.len() >= control.max_pair_steps / 2 {
                        return Err(Error::PairBudget);
                    }
                    edges.push((i, j, conductance));
                    rates[i] += conductance / capacities[i];
                    rates[j] += conductance / capacities[j];
                }
            }
        }
        if edges.is_empty() {
            return Ok(Default::default());
        }
        if rates.iter().any(|v| !v.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        let rate = rates.into_iter().fold(0.0, f64::max);
        let mut remaining = dt;
        let mut candidate = self.clone();
        let mut report = GasGridHeatReport::default();
        while remaining > 0.0 {
            if report.substeps == control.max_substeps {
                return Err(Error::SubstepBudget);
            }
            let step = (control.max_exchange_number / rate).min(remaining);
            if !positive(step) || remaining - step == remaining {
                return Err(Error::NumericalFailure);
            }
            for &(i, j, conductance) in edges.iter().chain(edges.iter().rev()) {
                if report.pair_steps == control.max_pair_steps {
                    return Err(Error::PairBudget);
                }
                let a = candidate.cells[i].temperature;
                let b = candidate.cells[j].temperature;
                let small = capacities[i].min(capacities[j]);
                let large = capacities[i].max(capacities[j]);
                let reduced = small / (1.0 + small / large);
                if !positive(reduced) {
                    return Err(Error::NumericalFailure);
                }
                let decay = -(-conductance / reduced * (0.5 * step)).exp_m1();
                let wa = (reduced / capacities[i]) * decay;
                let wb = (reduced / capacities[j]) * decay;
                let left = (1.0 - wa) * a + wa * b;
                let right = (1.0 - wb) * b + wb * a;
                let transferred = reduced * (b - a) * decay;
                if !positive(left) || !positive(right) || !transferred.is_finite() {
                    return Err(Error::NumericalFailure);
                }
                candidate.cells[i].temperature = left;
                candidate.cells[j].temperature = right;
                report.absolute_transferred_heat += transferred.abs();
                report.pair_steps += 1;
            }
            remaining = (remaining - step).max(0.0);
            report.substeps += 1;
        }
        let after = candidate.totals()?;
        if !report.absolute_transferred_heat.is_finite()
            || (after.thermal_energy - before.thermal_energy).abs()
                > 2048.0 * f64::EPSILON * before.thermal_energy.max(1.0)
        {
            return Err(Error::NumericalFailure);
        }
        *self = candidate;
        Ok(report)
    }
}
