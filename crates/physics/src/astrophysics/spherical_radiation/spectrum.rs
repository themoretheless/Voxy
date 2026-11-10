//! Frequency-dependent free-free spherical radiation transfer and thermal evolution.
use super::{
    Error, Exchange, Heating, Rates, ReactiveError, ReactiveExchange, ReactiveSettings, Shell,
    ThermalRates, transfer_sources,
};

/// Explicit finite frequency band and logarithmic midpoint quadrature for
/// physical free-free transfer. No frequency-tail extrapolation or scattering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpectralSurface {
    /// Surface source equals the outermost cell source.
    CellSource,
    /// Per-bin extension of the grey Eddington surface closure for unresolved
    /// optically thick surface cells. Requires convergence checks; not a non-grey
    /// atmosphere model or an observed-star calibration.
    EddingtonApproximation,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FreeFreeSpectrum {
    pub surface: SpectralSurface,
    pub absorption: crate::astrophysics::opacity::FreeFree,
    pub min_frequency: f64,
    pub max_frequency: f64,
    pub bins: usize,
    /// Blackbody exterior irradiation temperature, K; zero means no irradiation.
    pub ambient_temperature: f64,
}
impl FreeFreeSpectrum {
    fn validate(self) -> Result<(), Error> {
        if ![
            self.min_frequency,
            self.max_frequency,
            self.ambient_temperature,
        ]
        .into_iter()
        .all(f64::is_finite)
            || self.min_frequency <= 0.0
            || self.max_frequency <= self.min_frequency
            || self.ambient_temperature < 0.0
            || self.bins == 0
            || u32::try_from(self.bins).is_err()
        {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}
fn planck_bin(temperature: f64, frequency: f64, width: f64) -> Result<f64, Error> {
    if temperature == 0.0 {
        return Ok(0.0);
    }
    let x = 6.626_070_15e-34 * frequency / (crate::astrophysics::eos::BOLTZMANN * temperature);
    if x > 700.0 {
        return Ok(0.0);
    }
    let value = 2.0 * 6.626_070_15e-34 * frequency.powi(3) * width
        / crate::astrophysics::eos::LIGHT_SPEED.powi(2)
        / x.exp_m1();
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(Error::NumericalOverflow)
    }
}
impl crate::astrophysics::spherical::Sphere {
    /// Frequency-dependent free-free chord transport with local composition.
    /// Returns finite-band heating and luminosity in W. Frequency/angular
    /// convergence and adequacy of the chosen band are caller responsibilities.
    /// Work budget includes every frequency bin, shell chord and angular sample.
    pub fn free_free_radiation_rates(
        &self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics::nuclear::Network,
        spectrum: FreeFreeSpectrum,
        rays_per_annulus: usize,
        max_segments: usize,
    ) -> Result<Rates, Error> {
        if fractions.len() != self.cells.len()
            || self.cells.is_empty()
            || ![
                spectrum.min_frequency,
                spectrum.max_frequency,
                spectrum.ambient_temperature,
            ]
            .into_iter()
            .all(f64::is_finite)
            || spectrum.min_frequency <= 0.0
            || spectrum.max_frequency <= spectrum.min_frequency
            || spectrum.ambient_temperature < 0.0
            || spectrum.bins == 0
            || rays_per_annulus == 0
        {
            return Err(Error::InvalidInput);
        }
        self.totals().map_err(|_| Error::InvalidInput)?;
        let n = self.cells.len();
        let required = n
            .checked_mul(n + 1)
            .and_then(|v| v.checked_mul(rays_per_annulus))
            .and_then(|v| v.checked_mul(spectrum.bins))
            .ok_or(Error::BudgetExceeded)?;
        if required > max_segments {
            return Err(Error::BudgetExceeded);
        }
        let bins = u32::try_from(spectrum.bins).map_err(|_| Error::InvalidInput)?;
        let mut temperatures = Vec::with_capacity(n);
        let mut species = Vec::with_capacity(n);
        for (cell, row) in self.cells.iter().zip(fractions) {
            let mixture = network.mixture(row).map_err(|_| Error::InvalidInput)?;
            temperatures.push(
                mixture
                    .temperature(
                        cell.density,
                        cell.energy - 0.5 * cell.momentum.powi(2) / cell.density,
                    )
                    .map_err(|_| Error::InvalidInput)?,
            );
            species.push(
                row.iter()
                    .zip(&network.nuclei)
                    .map(|(x, nucleus)| crate::astrophysics::eos::Species {
                        mass_fraction: *x,
                        mass_number: nucleus.mass_number,
                        nuclear_charge: nucleus.charge,
                    })
                    .collect::<Vec<_>>(),
            );
        }
        let log_min = spectrum.min_frequency.ln();
        let delta = (spectrum.max_frequency.ln() - log_min) / f64::from(bins);
        let mut result = Rates {
            heating: vec![0.0; n],
            luminosity: 0.0,
            segments: 0,
        };
        for bin in 0..bins {
            let lo = (log_min + f64::from(bin) * delta).exp();
            let hi = (log_min + f64::from(bin + 1) * delta).exp();
            let frequency = (log_min + (f64::from(bin) + 0.5) * delta).exp();
            let width = hi - lo;
            let mut radius = 0.0;
            let mut shells = Vec::with_capacity(n);
            let mut sources = Vec::with_capacity(n);
            for i in 0..n {
                radius += self.spacing;
                let opacity = spectrum
                    .absorption
                    .spectral(
                        self.cells[i].density,
                        temperatures[i],
                        frequency,
                        &species[i],
                    )
                    .map_err(Error::Opacity)?;
                shells.push(Shell {
                    outer_radius: radius,
                    absorption: opacity * self.cells[i].density,
                    temperature: temperatures[i],
                });
                sources.push(planck_bin(temperatures[i], frequency, width)?);
            }
            let ambient = planck_bin(spectrum.ambient_temperature, frequency, width)?;
            let rates = transfer_sources(
                &shells,
                &sources,
                ambient,
                rays_per_annulus,
                max_segments - result.segments,
                true,
                spectrum.surface,
            )?;
            result.luminosity += rates.luminosity;
            result.segments += rates.segments;
            for (sum, power) in result.heating.iter_mut().zip(rates.heating) {
                *sum += power;
            }
        }
        if !result.luminosity.is_finite() || result.heating.iter().any(|v| !v.is_finite()) {
            return Err(Error::NumericalOverflow);
        }
        Ok(result)
    }
    /// Nuclear and frequency-dependent free-free radiative thermal powers.
    pub fn thermal_rates_free_free(
        &self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics::nuclear::Network,
        spectrum: FreeFreeSpectrum,
        rays_per_annulus: usize,
        max_segments: usize,
        fit_budget: usize,
    ) -> Result<ThermalRates, ReactiveError> {
        let radiation = self
            .free_free_radiation_rates(fractions, network, spectrum, rays_per_annulus, max_segments)
            .map_err(ReactiveError::Radiation)?;
        self.thermal_rates_from_radiation(radiation, fractions, network, fit_budget)
    }
}

impl crate::astrophysics::spherical::Sphere {
    /// Evolve internal energy with finite-band physical free-free transfer.
    /// Density, momentum and composition are fixed. Only ray count, cumulative
    /// segment/step budgets and max_step are used from Heating; spectrum supplies
    /// absorption and blackbody irradiation. Explicit updates limit both energy
    /// and temperature changes; callers must verify timestep convergence.
    /// # Errors
    /// Invalid inputs, opacity/EOS domain, exhausted budget or overflow. Atomic.
    pub fn radiate_free_free(
        &mut self,
        dt: f64,
        settings: Heating,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics::nuclear::Network,
        spectrum: FreeFreeSpectrum,
    ) -> Result<Exchange, Error> {
        if !dt.is_finite()
            || dt < 0.0
            || !settings.max_step.is_finite()
            || settings.max_step <= 0.0
            || settings.rays_per_annulus == 0
            || fractions.len() != self.cells.len()
        {
            return Err(Error::InvalidInput);
        }
        spectrum.validate()?;
        self.totals().map_err(|_| Error::InvalidInput)?;
        let mixtures = fractions
            .iter()
            .map(|row| network.mixture(row).map_err(|_| Error::InvalidInput))
            .collect::<Result<Vec<_>, _>>()?;
        let species = fractions
            .iter()
            .map(|row| {
                row.iter()
                    .zip(&network.nuclei)
                    .map(|(x, nucleus)| crate::astrophysics::eos::Species {
                        mass_fraction: *x,
                        mass_number: nucleus.mass_number,
                        nuclear_charge: nucleus.charge,
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut volumes = Vec::with_capacity(self.cells.len());
        let mut inner = 0.0;
        for _ in &self.cells {
            let outer = inner + self.spacing;
            volumes.push(
                4.0 * std::f64::consts::PI / 3.0
                    * self.spacing
                    * (inner * inner + inner * outer + outer * outer),
            );
            inner = outer;
        }
        let validate = |sphere: &Self| -> Result<(), Error> {
            for (i, cell) in sphere.cells.iter().enumerate() {
                let temperature = mixtures[i]
                    .temperature(
                        cell.density,
                        cell.energy - 0.5 * cell.momentum.powi(2) / cell.density,
                    )
                    .map_err(|_| Error::InvalidInput)?;
                spectrum
                    .absorption
                    .planck(cell.density, temperature, &species[i])
                    .map_err(Error::Opacity)?;
                let eos = mixtures[i]
                    .at(cell.density, temperature)
                    .map_err(|_| Error::InvalidInput)?;
                if eos.sound_speed_squared >= crate::astrophysics::eos::LIGHT_SPEED.powi(2) {
                    return Err(Error::InvalidInput);
                }
            }
            Ok(())
        };
        let mut next = self.clone();
        validate(&next)?;
        let mut remaining = dt;
        let mut exchange = Exchange {
            steps: 0,
            segments: 0,
            escaped_energy: 0.0,
        };
        while remaining > 0.0 {
            if exchange.steps >= settings.max_steps {
                return Err(Error::BudgetExceeded);
            }
            let rates = next.free_free_radiation_rates(
                fractions,
                network,
                spectrum,
                settings.rays_per_annulus,
                settings.max_segments - exchange.segments,
            )?;
            let mut h = remaining.min(settings.max_step);
            for (i, cell) in next.cells.iter().enumerate() {
                let internal = cell.energy - 0.5 * cell.momentum.powi(2) / cell.density;
                let temperature = mixtures[i]
                    .temperature(cell.density, internal)
                    .map_err(|_| Error::InvalidInput)?;
                let eos = mixtures[i]
                    .at(cell.density, temperature)
                    .map_err(|_| Error::InvalidInput)?;
                let power = rates.heating[i].abs();
                if power > 0.0 {
                    h = h.min(0.01 * internal * volumes[i] / power);
                    // Free-free mean varies as T^-3.5; keep primitive changes small.
                    h = h.min(
                        0.01 * temperature * cell.density * eos.specific_heat_cv * volumes[i]
                            / power
                            / 4.5,
                    );
                }
            }
            if !h.is_finite() || h <= 0.0 || remaining - h == remaining {
                return Err(Error::NumericalOverflow);
            }
            for (i, cell) in next.cells.iter_mut().enumerate() {
                cell.energy += h * rates.heating[i] / volumes[i];
            }
            validate(&next)?;
            exchange.steps += 1;
            exchange.segments += rates.segments;
            exchange.escaped_energy += h * rates.luminosity;
            if !exchange.escaped_energy.is_finite() {
                return Err(Error::NumericalOverflow);
            }
            remaining = (remaining - h).max(0.0);
        }
        *self = next;
        Ok(exchange)
    }
    /// Frequency-dependent free-free thermal steps around reactive dynamics.
    /// Optional hydrostatic reference and matter reservoir use the same transport
    /// paths as the grey solver. Composition refreshes after burning/advection.
    /// Both thermal halves share budgets; all phases commit together.
    pub fn step_reactive_free_free(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics::nuclear::Network,
        dt: f64,
        settings: ReactiveSettings,
        spectrum: FreeFreeSpectrum,
        reference: Option<&crate::astrophysics::spherical::HydrostaticReference>,
        exterior: Option<&crate::astrophysics::spherical::CompositionExterior>,
    ) -> Result<ReactiveExchange, ReactiveError> {
        self.step_reactive_radiating_spectrum_impl(
            fractions,
            network,
            dt,
            settings,
            None,
            exterior,
            reference,
            Some(spectrum),
        )
    }
}
