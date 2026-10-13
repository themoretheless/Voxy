//! Joint fixed-radius stellar structure solve: shell hydrostatic pressure and
//! instantaneous nuclear/radiative balance. No composition evolution, convection,
//! mass constraint or stability claim. Surface pressure is prescribed; mass is
//! an output. The gas must initially be stationary.
use crate::{
    astrophysics_eos::Mixture,
    astrophysics_nuclear::Network,
    astrophysics_opacity::Table,
    astrophysics_spherical::Sphere,
    astrophysics_spherical_radiation::{Heating, ReactiveError, ThermalRates},
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Search {
    pub min_density: f64,
    pub max_density: f64,
    pub min_temperature: f64,
    pub max_temperature: f64,
    pub relative_tolerance: f64,
    /// Absolute shell net-power tolerance in W; zero requests purely relative balance.
    pub absolute_power_tolerance: f64,
    pub iterations: usize,
    pub evaluations: usize,
    pub fit_evaluations: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Equilibrium {
    pub thermal: ThermalRates,
    /// Maximum absolute log ratio of EOS to required hydrostatic pressure.
    pub hydrostatic_residual: f64,
    /// Thermal residual normalized with the explicit absolute power floor.
    pub thermal_residual: f64,
    pub maximum_net_power: f64,
    pub iterations: usize,
    pub evaluations: usize,
    pub segments: usize,
    pub fit_evaluations: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    InvalidInput,
    Physics(ReactiveError),
    BudgetExceeded,
    NoConvergence,
}
fn eos_error(e: crate::astrophysics_eos::Error) -> Error {
    Error::Physics(ReactiveError::Dynamics(
        crate::astrophysics_spherical::Error::Eos(e),
    ))
}
fn dynamics_error(e: crate::astrophysics_spherical::Error) -> Error {
    Error::Physics(ReactiveError::Dynamics(e))
}
fn state(template: &Sphere, mixtures: &[Mixture], x: &[f64]) -> Result<(Sphere, Vec<f64>), Error> {
    let n = mixtures.len();
    let mut sphere = template.clone();
    let mut pressures = Vec::with_capacity(n);
    for i in 0..n {
        let rho = x[i].exp();
        let temperature = x[n + i].exp();
        let gas = mixtures[i].at(rho, temperature).map_err(eos_error)?;
        if gas.sound_speed_squared >= 299_792_458.0_f64.powi(2) {
            return Err(Error::InvalidInput);
        }
        sphere.cells[i].density = rho;
        sphere.cells[i].energy = gas.internal_energy_density;
        pressures.push(gas.gas_pressure + gas.radiation_pressure);
    }
    Ok((sphere, pressures))
}
fn norm(values: &[f64]) -> f64 {
    values.iter().fold(0.0_f64, |m, x| m.max(x.abs()))
}
// Freeze thermal power scales within a Newton iteration. Differentiating the
// normalized ratio itself makes its Jacobian almost zero when cooling dominates
// heating (the residual then saturates near -1), even though net power varies.
fn scaled_trial(
    residual: &[f64],
    trial: &ThermalRates,
    base: &ThermalRates,
    n: usize,
    power_floor: f64,
) -> Result<Vec<f64>, Error> {
    let mut result = residual.to_vec();
    for i in 0..n {
        let scale = base.nuclear[i]
            .abs()
            .max(base.radiation[i].abs())
            .max(power_floor);
        result[n + i] = if scale == 0.0 {
            if trial.net[i] != 0.0 {
                return Err(Error::NoConvergence);
            }
            0.0
        } else {
            (trial.net[i] / scale)
                / (base.nuclear[i].abs() / scale + base.radiation[i].abs() / scale)
                    .max(power_floor / scale)
        };
        if !result[n + i].is_finite() {
            return Err(Error::NoConvergence);
        }
    }
    Ok(result)
}
/// Dense Gaussian elimination with row pivoting. Matrix is a numerical
/// Jacobian of dimensionless residuals with respect to logarithmic primitives.
fn solve(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Result<Vec<f64>, Error> {
    let n = rhs.len();
    for column in 0..n {
        let pivot = (column..n)
            .max_by(|&a, &b| matrix[a][column].abs().total_cmp(&matrix[b][column].abs()))
            .unwrap();
        if matrix[pivot][column].abs() < 1e-14 {
            return Err(Error::NoConvergence);
        }
        matrix.swap(column, pivot);
        rhs.swap(column, pivot);
        for row in column + 1..n {
            let factor = matrix[row][column] / matrix[column][column];
            for j in column + 1..n {
                matrix[row][j] -= factor * matrix[column][j];
            }
            rhs[row] -= factor * rhs[column];
        }
    }
    let mut solution = vec![0.0; n];
    for i in (0..n).rev() {
        let sum: f64 = (i + 1..n).map(|j| matrix[i][j] * solution[j]).sum();
        solution[i] = (rhs[i] - sum) / matrix[i][i];
        if !solution[i].is_finite() {
            return Err(Error::NoConvergence);
        }
    }
    Ok(solution)
}
impl Sphere {
    /// Damped Newton solve of coupled hydrostatic and thermal residuals in log
    /// density and temperature. Fixed grid/radius, composition and surface
    /// pressure determine the resulting mass; input mass is not conserved.
    /// All trial physics evaluations share ray/reaction budgets. A failed search
    /// never changes the sphere. A converged root need not be dynamically stable.
    /// # Errors
    /// Invalid bounds/stationarity, physics domain, work budget, singular
    /// Jacobian or failure to converge within the requested limits.
    pub fn equilibrate_stellar(
        &mut self,
        fractions: &[Vec<f64>],
        network: &Network,
        surface_pressure: f64,
        heating: Heating,
        table: Option<&Table>,
        search: Search,
    ) -> Result<Equilibrium, Error> {
        self.equilibrate_stellar_impl(
            fractions,
            network,
            surface_pressure,
            heating,
            table,
            search,
            None,
        )
    }
    /// Joint stellar solve using frequency-dependent physical free-free transfer.
    /// The explicit frequency band, quadrature and all work budgets apply to
    /// every trial state. The same mass/radius and stability limits apply.
    pub fn equilibrate_stellar_free_free(
        &mut self,
        fractions: &[Vec<f64>],
        network: &Network,
        surface_pressure: f64,
        spectrum: crate::astrophysics_spherical_radiation::FreeFreeSpectrum,
        rays_per_annulus: usize,
        max_segments: usize,
        search: Search,
    ) -> Result<Equilibrium, Error> {
        let heating = Heating {
            specific_heat: 1.0,
            opacity: 0.0,
            ambient: 0.0,
            rays_per_annulus,
            max_segments,
            max_step: 1.0,
            max_steps: 1,
        };
        self.equilibrate_stellar_impl(
            fractions,
            network,
            surface_pressure,
            heating,
            None,
            search,
            Some(spectrum),
        )
    }
    fn equilibrate_stellar_impl(
        &mut self,
        fractions: &[Vec<f64>],
        network: &Network,
        surface_pressure: f64,
        heating: Heating,
        table: Option<&Table>,
        search: Search,
        spectrum: Option<crate::astrophysics_spherical_radiation::FreeFreeSpectrum>,
    ) -> Result<Equilibrium, Error> {
        let n = self.cells.len();
        if n == 0
            || fractions.len() != n
            || self.cells.iter().any(|c| c.momentum != 0.0)
            || ![
                search.min_density,
                search.max_density,
                search.min_temperature,
                search.max_temperature,
                search.relative_tolerance,
                search.absolute_power_tolerance,
                surface_pressure,
            ]
            .into_iter()
            .all(f64::is_finite)
            || search.min_density <= 0.0
            || search.max_density <= search.min_density
            || search.min_temperature <= 0.0
            || search.max_temperature <= search.min_temperature
            || search.relative_tolerance <= 0.0
            || search.relative_tolerance >= 1.0
            || search.absolute_power_tolerance < 0.0
            || !(search.absolute_power_tolerance / search.relative_tolerance).is_finite()
            || surface_pressure <= 0.0
            || search.iterations == 0
            || search.evaluations == 0
        {
            return Err(Error::InvalidInput);
        }
        let mixtures = fractions
            .iter()
            .map(|r| {
                network
                    .mixture(r)
                    .map_err(|e| dynamics_error(crate::astrophysics_spherical::Error::Nuclear(e)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut x = Vec::with_capacity(2 * n);
        let mut temperatures = Vec::with_capacity(n);
        for (cell, mixture) in self.cells.iter().zip(&mixtures) {
            let temperature = mixture
                .temperature(cell.density, cell.energy)
                .map_err(eos_error)?;
            if cell.density < search.min_density
                || cell.density > search.max_density
                || temperature < search.min_temperature
                || temperature > search.max_temperature
            {
                return Err(Error::InvalidInput);
            }
            x.push(cell.density.ln());
            temperatures.push(temperature.ln());
        }
        x.extend(temperatures);
        let lower: Vec<_> = (0..2 * n)
            .map(|i| {
                if i < n {
                    search.min_density.ln()
                } else {
                    search.min_temperature.ln()
                }
            })
            .collect();
        let upper: Vec<_> = (0..2 * n)
            .map(|i| {
                if i < n {
                    search.max_density.ln()
                } else {
                    search.max_temperature.ln()
                }
            })
            .collect();
        let power_floor = search.absolute_power_tolerance / search.relative_tolerance;
        let mut evaluations = 0usize;
        let mut segments = 0usize;
        let mut fits = 0usize;
        let mut evaluate = |point: &[f64]| -> Result<(Sphere, ThermalRates, Vec<f64>), Error> {
            if evaluations >= search.evaluations {
                return Err(Error::BudgetExceeded);
            }
            let (sphere, actual_pressure) = state(self, &mixtures, point)?;
            let expected = sphere
                .hydrostatic_pressures(surface_pressure)
                .map_err(dynamics_error)?;
            let mut remaining = heating;
            remaining.max_segments = heating.max_segments - segments;
            let rates = if let Some(spectrum) = spectrum {
                sphere.thermal_rates_free_free(
                    fractions,
                    network,
                    spectrum,
                    heating.rays_per_annulus,
                    remaining.max_segments,
                    search.fit_evaluations - fits,
                )
            } else {
                sphere.thermal_rates(
                    remaining,
                    fractions,
                    network,
                    table,
                    search.fit_evaluations - fits,
                )
            }
            .map_err(Error::Physics)?;
            evaluations += 1;
            segments += rates.segments;
            fits += rates.fit_evaluations;
            let mut residual: Vec<f64> = actual_pressure
                .iter()
                .zip(&expected.cells)
                .map(|(p, target)| (p / target).ln())
                .collect();
            for i in 0..n {
                let scale = rates.nuclear[i]
                    .abs()
                    .max(rates.radiation[i].abs())
                    .max(power_floor);
                residual.push(if scale == 0.0 {
                    0.0
                } else {
                    (rates.net[i] / scale)
                        / (rates.nuclear[i].abs() / scale + rates.radiation[i].abs() / scale)
                            .max(power_floor / scale)
                });
            }
            if residual.iter().any(|v| !v.is_finite()) {
                return Err(Error::NoConvergence);
            }
            Ok((sphere, rates, residual))
        };
        let mut current = evaluate(&x)?;
        for iteration in 0..=search.iterations {
            let maximum = norm(&current.2);
            if maximum <= search.relative_tolerance {
                let hydrostatic_residual = norm(&current.2[..n]);
                *self = current.0;
                return Ok(Equilibrium {
                    hydrostatic_residual,
                    thermal_residual: norm(&current.2[n..]),
                    maximum_net_power: norm(&current.1.net),
                    thermal: current.1,
                    iterations: iteration,
                    evaluations,
                    segments,
                    fit_evaluations: fits,
                });
            }
            if iteration == search.iterations {
                return Err(Error::NoConvergence);
            }
            let mut jacobian = vec![vec![0.0; 2 * n]; 2 * n];
            for j in 0..2 * n {
                let delta = if x[j] + 1e-3 <= upper[j] { 1e-3 } else { -1e-3 };
                if x[j] + delta < lower[j] {
                    return Err(Error::NoConvergence);
                }
                let mut perturbed = x.clone();
                perturbed[j] += delta;
                let trial = evaluate(&perturbed)?;
                let scaled = scaled_trial(&trial.2, &trial.1, &current.1, n, power_floor)?;
                for (i, row) in jacobian.iter_mut().enumerate() {
                    row[j] = (scaled[i] - current.2[i]) / delta;
                }
            }
            // Newton first; regularized least-squares directions provide a
            // descent fallback when the coupled Jacobian is ill-conditioned.
            let mut directions = Vec::new();
            if let Ok(direction) = solve(jacobian.clone(), current.2.iter().map(|r| -r).collect()) {
                directions.push(direction);
            }
            let mut normal = vec![vec![0.0; 2 * n]; 2 * n];
            let mut gradient = vec![0.0; 2 * n];
            for i in 0..2 * n {
                for row in 0..2 * n {
                    gradient[i] -= jacobian[row][i] * current.2[row];
                    for j in 0..2 * n {
                        normal[i][j] += jacobian[row][i] * jacobian[row][j];
                    }
                }
            }
            let diagonal = (0..2 * n).map(|i| normal[i][i]).fold(0.0_f64, f64::max);
            for regularization in [1e-6, 1e-3, 1.0, 100.0] {
                let mut damped = normal.clone();
                for i in 0..2 * n {
                    damped[i][i] += regularization * diagonal;
                }
                if let Ok(direction) = solve(damped, gradient.clone()) {
                    directions.push(direction);
                }
            }
            let merit = |values: &[f64]| values.iter().map(|v| v * v).sum::<f64>();
            let initial_merit = merit(&current.2);
            let mut accepted = None;
            for direction in directions {
                let mut damping = (0.25 / norm(&direction)).min(1.0);
                for _ in 0..24 {
                    let trial_x: Vec<_> = x
                        .iter()
                        .zip(&direction)
                        .map(|(a, d)| a + damping * d)
                        .collect();
                    if trial_x
                        .iter()
                        .enumerate()
                        .all(|(i, v)| *v >= lower[i] && *v <= upper[i])
                    {
                        let trial = evaluate(&trial_x)?;
                        let scaled = scaled_trial(&trial.2, &trial.1, &current.1, n, power_floor)?;
                        if merit(&scaled) < initial_merit {
                            accepted = Some((trial_x, trial));
                            break;
                        }
                    }
                    damping *= 0.5;
                }
                if accepted.is_some() {
                    break;
                }
            }
            let (next_x, next) = accepted.ok_or(Error::NoConvergence)?;
            x = next_x;
            current = next;
        }
        Err(Error::NoConvergence)
    }
}
