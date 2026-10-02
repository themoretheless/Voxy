//! Grey LTE spherical transfer using straight chords and projected annulus
//! quadrature. Static absorption/emission; isotropic ambient intensity, no scattering.
use crate::astrophysics_radiation::{LinearSourceLayer, trace_linear_sources};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shell {
    pub outer_radius: f64,
    pub absorption: f64,
    pub temperature: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Rates {
    /// Net deposited thermal power per shell in W; negative when cooling.
    pub heating: Vec<f64>,
    /// Net outward luminosity minus ambient irradiation, W.
    pub luminosity: f64,
    pub segments: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    NumericalOverflow,
    BudgetExceeded,
    Opacity(crate::astrophysics_opacity::Error),
}
/// Integrate full chords from the far exterior to the near exterior. Every
/// radial annulus gets samples, including the innermost core. Cost O(N² Q),
/// Q=`rays_per_annulus`; `max_segments` strictly bounds transfer work.
/// # Errors
/// Invalid shell geometry/parameters, overflow or exhausted work budget.
pub fn transfer(
    shells: &[Shell],
    ambient: f64,
    rays_per_annulus: usize,
    max_segments: usize,
) -> Result<Rates, Error> {
    let sources = shells
        .iter()
        .map(|s| {
            crate::astrophysics_radiation::blackbody(s.temperature).map_err(|_| Error::InvalidInput)
        })
        .collect::<Result<Vec<_>, _>>()?;
    transfer_sources(
        shells,
        &sources,
        ambient,
        rays_per_annulus,
        max_segments,
        false,
        SpectralSurface::CellSource,
    )
}
fn transfer_sources(
    shells: &[Shell],
    sources: &[f64],
    ambient: f64,
    rays_per_annulus: usize,
    max_segments: usize,
    linear_sources: bool,
    surface: SpectralSurface,
) -> Result<Rates, Error> {
    if shells.is_empty() || !ambient.is_finite() || ambient < 0.0 || rays_per_annulus == 0 {
        return Err(Error::InvalidInput);
    }
    let mut previous = 0.0;
    for shell in shells {
        if ![shell.outer_radius, shell.absorption, shell.temperature]
            .into_iter()
            .all(f64::is_finite)
            || shell.outer_radius <= previous
            || shell.absorption < 0.0
            || shell.temperature < 0.0
            || !shell.outer_radius.powi(2).is_finite()
        {
            return Err(Error::InvalidInput);
        }
        previous = shell.outer_radius;
    }
    let required = shells
        .len()
        .checked_mul(shells.len() + 1)
        .and_then(|n| n.checked_mul(rays_per_annulus))
        .ok_or(Error::BudgetExceeded)?;
    if required > max_segments {
        return Err(Error::BudgetExceeded);
    }
    let quadrature_count = u32::try_from(rays_per_annulus).map_err(|_| Error::InvalidInput)?;
    let mut result = Rates {
        heating: vec![0.0; shells.len()],
        luminosity: 0.0,
        segments: 0,
    };
    let mut face_sources = Vec::with_capacity(shells.len() + 1);
    face_sources.push(sources[0]);
    for i in 1..shells.len() {
        let left_inner = if i == 1 {
            0.0
        } else {
            shells[i - 2].outer_radius
        };
        let left_center = 0.5 * (left_inner + shells[i - 1].outer_radius);
        let right_center = 0.5 * (shells[i - 1].outer_radius + shells[i].outer_radius);
        let w = (shells[i - 1].outer_radius - left_center) / (right_center - left_center);
        face_sources.push((1.0 - w) * sources[i - 1] + w * sources[i]);
    }
    let last = shells.len() - 1;
    let outer_source = sources[last];
    let boundary_source = if linear_sources && surface == SpectralSurface::EddingtonApproximation {
        let inner_radius = if last == 0 {
            0.0
        } else {
            shells[last - 1].outer_radius
        };
        let depth = shells[last].absorption * (shells[last].outer_radius - inner_radius);
        if !depth.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        // B(tau)=3 F/(4 pi)*(tau+2/3), center tau=alpha*dr/2.
        // Extend this grey closure independently to each frequency bin. It is
        // a numerical boundary approximation, not a calibrated non-grey atmosphere.
        let weight = 1.0 / (1.0 + 0.75 * depth);
        if outer_source == ambient {
            ambient
        } else {
            weight * outer_source + (1.0 - weight) * ambient
        }
    } else {
        outer_source
    };
    face_sources.push(boundary_source);
    let mut inner: f64 = 0.0;
    for (annulus, shell) in shells.iter().enumerate() {
        let outer = shell.outer_radius;
        let area =
            std::f64::consts::PI * (outer - inner) * (outer + inner) / f64::from(quadrature_count);
        let weight = 4.0 * std::f64::consts::PI * area;
        for sample in 0..quadrature_count {
            let fraction = (f64::from(sample) + 0.5) / f64::from(quadrature_count);
            let impact_squared = inner * inner + fraction * (outer - inner) * (outer + inner);
            let mut layers = Vec::with_capacity(2 * (shells.len() - annulus));
            let mut owners = Vec::with_capacity(layers.capacity());
            for i in (annulus..shells.len()).rev() {
                let a = if i == 0 {
                    0.0
                } else {
                    shells[i - 1].outer_radius
                };
                let b = shells[i].outer_radius;
                let far = (b * b - impact_squared).max(0.0).sqrt();
                let near = (a * a - impact_squared).max(0.0).sqrt();
                let length = far - near;
                if linear_sources {
                    let closest = a.max(impact_squared.sqrt());
                    let center = 0.5 * (a + b);
                    if closest < center {
                        let middle = (center * center - impact_squared).sqrt();
                        layers.push(LinearSourceLayer {
                            length: far - middle,
                            absorption: shells[i].absorption,
                            source_start: face_sources[i + 1],
                            source_end: sources[i],
                        });
                        owners.push(i);
                        let w = (closest - a) / (center - a);
                        layers.push(LinearSourceLayer {
                            length: middle - near,
                            absorption: shells[i].absorption,
                            source_start: sources[i],
                            source_end: (1.0 - w) * face_sources[i] + w * sources[i],
                        });
                        owners.push(i);
                    } else {
                        let w = (closest - center) / (b - center);
                        layers.push(LinearSourceLayer {
                            length,
                            absorption: shells[i].absorption,
                            source_start: face_sources[i + 1],
                            source_end: (1.0 - w) * sources[i] + w * face_sources[i + 1],
                        });
                        owners.push(i);
                    }
                } else {
                    layers.push(LinearSourceLayer {
                        length,
                        absorption: shells[i].absorption,
                        source_start: sources[i],
                        source_end: sources[i],
                    });
                    owners.push(i);
                }
            }
            let half = layers.len();
            for i in (0..half).rev() {
                let layer = layers[i];
                layers.push(LinearSourceLayer {
                    source_start: layer.source_end,
                    source_end: layer.source_start,
                    ..layer
                });
                owners.push(owners[i]);
            }
            if layers.len() > max_segments - result.segments {
                return Err(Error::BudgetExceeded);
            }
            let ray = trace_linear_sources(ambient, &layers, layers.len())
                .map_err(|_| Error::NumericalOverflow)?;
            result.segments += layers.len();
            if linear_sources {
                let mut deposited_sum = 0.0;
                let mut compensation = 0.0;
                // Mirrored segments have equal optical depth and opposite source
                // slopes. Cancel those source terms algebraically before summing
                // the small physical deposition; don't subtract their huge powers.
                for i in 0..half {
                    let mirror = layers.len() - 1 - i;
                    let fraction = -(-layers[i].length * layers[i].absorption).exp_m1();
                    let power = fraction
                        * (ray.incident_source_offsets[i] + ray.incident_source_offsets[mirror]);
                    result.heating[owners[i]] += weight * power;
                    let corrected = power - compensation;
                    let sum = deposited_sum + corrected;
                    compensation = (sum - deposited_sum) - corrected;
                    deposited_sum = sum;
                }
                // The same exact formal solution telescopes to outgoing minus
                // incident intensity. Sum its small paired deposits instead of
                // subtracting nearly equal outgoing/incident/source intensities.
                result.luminosity -= weight * deposited_sum;
            } else {
                result.luminosity += weight * (ray.intensity - ambient);
                for (owner, power) in owners.into_iter().zip(ray.deposited) {
                    result.heating[owner] += weight * power;
                }
            }
        }
        inner = outer;
    }
    if !result.luminosity.is_finite() || !result.heating.iter().all(|p| p.is_finite()) {
        return Err(Error::NumericalOverflow);
    }
    Ok(result)
}
impl crate::astrophysics_spherical::Sphere {
    /// Derive shells from Euler density/internal energy and return spherical
    /// radiative power. SI: cv in J/kg/K, opacity in m²/kg, ambient W/m²/sr.
    /// # Errors
    /// Invalid gas/parameters, transfer budget exhaustion or overflow.
    pub fn radiation_rates(
        &self,
        specific_heat: f64,
        opacity: f64,
        ambient: f64,
        rays_per_annulus: usize,
        max_segments: usize,
    ) -> Result<Rates, Error> {
        self.radiation_rates_eos(
            specific_heat,
            opacity,
            ambient,
            rays_per_annulus,
            max_segments,
            None,
            None,
        )
    }
    /// Radiation rates from the gas + trapped radiation EOS at fixed composition.
    /// # Errors
    /// Invalid EOS/gas state, transfer parameters, overflow or work budget.
    pub fn radiation_rates_ionized(
        &self,
        mixture: crate::astrophysics_eos::Mixture,
        opacity: f64,
        ambient: f64,
        rays_per_annulus: usize,
        max_segments: usize,
    ) -> Result<Rates, Error> {
        self.radiation_rates_eos(
            1.5 * mixture.specific_gas_constant(),
            opacity,
            ambient,
            rays_per_annulus,
            max_segments,
            Some(mixture),
            None,
        )
    }
    #[allow(clippy::too_many_arguments)] // Shared transfer controls plus local/uniform EOS.
    fn radiation_rates_eos(
        &self,
        specific_heat: f64,
        opacity: f64,
        ambient: f64,
        rays_per_annulus: usize,
        max_segments: usize,
        mixture: Option<crate::astrophysics_eos::Mixture>,
        local: Option<&[crate::astrophysics_eos::Mixture]>,
    ) -> Result<Rates, Error> {
        self.totals().map_err(|_| Error::InvalidInput)?;
        if !specific_heat.is_finite()
            || specific_heat <= 0.0
            || !opacity.is_finite()
            || opacity < 0.0
        {
            return Err(Error::InvalidInput);
        }
        let mut radius = 0.0;
        let mut shells = Vec::with_capacity(self.cells.len());
        for (i, cell) in self.cells.iter().enumerate() {
            radius += self.spacing;
            let eos = local.map_or(mixture, |rows| Some(rows[i]));
            let temperature = temperature_eos(*cell, self.gamma, specific_heat, eos)?;
            shells.push(Shell {
                outer_radius: radius,
                absorption: opacity * cell.density,
                temperature,
            });
        }
        transfer(&shells, ambient, rays_per_annulus, max_segments)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heating {
    pub specific_heat: f64,
    pub opacity: f64,
    /// Isotropic incident intensity in W/m²/sr (use `blackbody` for a temperature).
    pub ambient: f64,
    pub rays_per_annulus: usize,
    /// Total ray-segment budget across all thermal substeps.
    pub max_segments: usize,
    pub max_step: f64,
    pub max_steps: usize,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exchange {
    pub steps: usize,
    pub segments: usize,
    pub escaped_energy: f64,
}
impl crate::astrophysics_spherical::Sphere {
    /// Heat/cool fixed-density spherical gas from the full-chord radiation field.
    /// Momentum is unchanged. Adaptive explicit update, no temperature floors.
    /// Full failure rolls back state; escaped energy is signed and measured in J.
    /// # Errors
    /// Invalid inputs, budget exhaustion, numerical/nonphysical evolution.
    pub fn radiate(&mut self, dt: f64, settings: Heating) -> Result<Exchange, Error> {
        self.radiate_eos(dt, settings, None)
    }
    /// Heat/cool with the gas + trapped radiation EOS. `Heating.specific_heat` is
    /// replaced by the mixture's gas heat capacity for the adaptive bound;
    /// temperature uses total EOS energy, including radiation exactly once.
    /// # Errors
    /// Same atomic validation/budget failures as `radiate`, plus EOS errors.
    pub fn radiate_ionized(
        &mut self,
        dt: f64,
        mut settings: Heating,
        mixture: crate::astrophysics_eos::Mixture,
    ) -> Result<Exchange, Error> {
        settings.specific_heat = 1.5 * mixture.specific_gas_constant();
        self.radiate_eos(dt, settings, Some(mixture))
    }
    pub(crate) fn radiate_eos(
        &mut self,
        dt: f64,
        settings: Heating,
        mixture: Option<crate::astrophysics_eos::Mixture>,
    ) -> Result<Exchange, Error> {
        self.radiate_local(dt, settings, mixture, None, None)
    }
    /// Radiation thermal step using the current composition in every shell.
    /// # Errors
    /// Invalid composition, EOS, thermal state or exhausted work budget.
    pub fn radiate_composition(
        &mut self,
        dt: f64,
        settings: Heating,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
    ) -> Result<Exchange, Error> {
        if fractions.len() != self.cells.len() {
            return Err(Error::InvalidInput);
        }
        let local = fractions
            .iter()
            .map(|row| network.mixture(row).map_err(|_| Error::InvalidInput))
            .collect::<Result<Vec<_>, _>>()?;
        self.radiate_local(dt, settings, None, Some(&local), None)
    }
    /// Thermal radiation with a supplied grey absorption table in SI rho/T.
    /// `settings.opacity` is superseded by the table. Full failure rolls back.
    /// # Errors
    /// Wrong opacity kind, outside table domain, EOS or work-budget failure.
    pub fn radiate_tabulated(
        &mut self,
        dt: f64,
        settings: Heating,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        table: &crate::astrophysics_opacity::Table,
    ) -> Result<Exchange, Error> {
        if table.kind != crate::astrophysics_opacity::Kind::GreyAbsorption
            || fractions.len() != self.cells.len()
        {
            return Err(Error::InvalidInput);
        }
        let local = fractions
            .iter()
            .map(|row| network.mixture(row).map_err(|_| Error::InvalidInput))
            .collect::<Result<Vec<_>, _>>()?;
        self.radiate_local(dt, settings, None, Some(&local), Some(table))
    }
    #[allow(clippy::too_many_lines)] // Keep thermal budget and transactional update together.
    fn radiate_local(
        &mut self,
        dt: f64,
        mut settings: Heating,
        mixture: Option<crate::astrophysics_eos::Mixture>,
        local: Option<&[crate::astrophysics_eos::Mixture]>,
        table: Option<&crate::astrophysics_opacity::Table>,
    ) -> Result<Exchange, Error> {
        if let Some(mixture) = mixture {
            settings.specific_heat = 1.5 * mixture.specific_gas_constant();
        }
        if let Some(local) = local {
            settings.specific_heat = local
                .iter()
                .map(|m| 1.5 * m.specific_gas_constant())
                .fold(f64::INFINITY, f64::min);
        }
        if let Some(table) = table {
            settings.opacity = table.maximum_opacity();
            self.validate_table_states(settings.specific_heat, mixture, local, table)?;
        }
        if !dt.is_finite() || dt < 0.0 || !settings.max_step.is_finite() || settings.max_step <= 0.0
        {
            return Err(Error::InvalidInput);
        }
        self.totals().map_err(|_| Error::InvalidInput)?;
        if ![settings.specific_heat, settings.opacity, settings.ambient]
            .into_iter()
            .all(f64::is_finite)
            || settings.specific_heat <= 0.0
            || settings.opacity < 0.0
            || settings.ambient < 0.0
            || settings.rays_per_annulus == 0
        {
            return Err(Error::InvalidInput);
        }
        let mut volumes = Vec::with_capacity(self.cells.len());
        let mut inner: f64 = 0.0;
        for _ in &self.cells {
            let outer = inner + self.spacing;
            volumes.push(
                4.0 * std::f64::consts::PI / 3.0
                    * self.spacing
                    * (outer * outer + outer * inner + inner * inner),
            );
            inner = outer;
        }
        let mut next = self.clone();
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
            let rates = if let Some(table) = table {
                next.table_rates(
                    Heating {
                        max_segments: settings.max_segments - exchange.segments,
                        ..settings
                    },
                    mixture,
                    local,
                    table,
                )?
            } else {
                next.radiation_rates_eos(
                    settings.specific_heat,
                    settings.opacity,
                    settings.ambient,
                    settings.rays_per_annulus,
                    settings.max_segments - exchange.segments,
                    mixture,
                    local,
                )?
            };
            let mut h = remaining.min(settings.max_step);
            let mut hottest: f64 = 0.0;
            for (i, ((cell, volume), power)) in next
                .cells
                .iter()
                .zip(&volumes)
                .zip(&rates.heating)
                .enumerate()
            {
                let thermal = cell
                    .pressure(next.gamma)
                    .map_err(|_| Error::NumericalOverflow)?
                    / (next.gamma - 1.0);
                let temperature = temperature_eos(
                    *cell,
                    next.gamma,
                    settings.specific_heat,
                    local.map_or(mixture, |rows| Some(rows[i])),
                )?;
                hottest = hottest.max(temperature);
                if power.abs() > 0.0 {
                    h = h.min(0.05 * thermal * volume / power.abs());
                    if let Some(table) = table {
                        // Gas-only cv is a lower bound for the trapped-radiation EOS.
                        h = h.min(
                            0.05 * temperature * cell.density * settings.specific_heat * volume
                                / power.abs()
                                / (1.0 + table.maximum_temperature_slope()),
                        );
                    }
                }
            }
            if settings.opacity > 0.0 && hottest > 0.0 {
                h = h.min(
                    0.05 * settings.specific_heat
                        / (16.0
                            * crate::astrophysics_thermal::STEFAN_BOLTZMANN
                            * settings.opacity
                            * hottest.powi(3)),
                );
            }
            if !h.is_finite() || h <= 0.0 || remaining - h >= remaining {
                return Err(Error::NumericalOverflow);
            }
            for ((cell, volume), power) in next.cells.iter_mut().zip(&volumes).zip(rates.heating) {
                cell.energy += h * power / volume;
                cell.pressure(next.gamma)
                    .map_err(|_| Error::NumericalOverflow)?;
            }
            exchange.escaped_energy += h * rates.luminosity;
            exchange.segments += rates.segments;
            exchange.steps += 1;
            remaining -= h;
        }
        if let Some(table) = table {
            next.validate_table_states(settings.specific_heat, mixture, local, table)?;
        }
        next.energy().map_err(|_| Error::NumericalOverflow)?;
        if !exchange.escaped_energy.is_finite() {
            return Err(Error::NumericalOverflow);
        }
        *self = next;
        Ok(exchange)
    }
}

fn temperature_eos(
    cell: crate::astrophysics_gas::Cell,
    gamma: f64,
    specific_heat: f64,
    mixture: Option<crate::astrophysics_eos::Mixture>,
) -> Result<f64, Error> {
    let internal = cell.pressure(gamma).map_err(|_| Error::InvalidInput)? / (gamma - 1.0);
    if let Some(mixture) = mixture {
        mixture
            .temperature(cell.density, internal)
            .map_err(|_| Error::NumericalOverflow)
    } else {
        Ok(internal / cell.density / specific_heat)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactiveSettings {
    pub hydro_max_step: f64,
    pub hydro_steps: usize,
    pub burn: crate::astrophysics_nuclear::Budget,
    pub radiation: Heating,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReactiveError {
    Dynamics(crate::astrophysics_spherical::Error),
    Radiation(Error),
}
#[derive(Clone, Debug, PartialEq)]
pub struct ReactiveExchange {
    pub dynamics: crate::astrophysics_spherical::ReactiveExchange,
    pub radiation: Exchange,
}
impl crate::astrophysics_spherical::Sphere {
    /// Half radiative thermal step, reactive dynamics, half radiative step.
    /// Uses the local composition before and after burning. All state commits
    /// atomically; both radiation halves share step and ray budgets.
    /// # Errors
    /// Any radiation, EOS, transport, reaction or work-budget failure.
    pub fn step_reactive_radiating(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        settings: ReactiveSettings,
    ) -> Result<ReactiveExchange, ReactiveError> {
        self.step_reactive_radiating_impl(fractions, network, dt, settings, None, None, None)
    }
    /// Reactive radiation/dynamics with grey absorption evaluated from a table.
    /// The same table applies before and after burning; caller supplies data for
    /// the intended composition model. All phases commit atomically.
    /// # Errors
    /// Outside table domain, wrong kind, or any coupled-state/work failure.
    pub fn step_reactive_tabulated(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        settings: ReactiveSettings,
        table: &crate::astrophysics_opacity::Table,
    ) -> Result<ReactiveExchange, ReactiveError> {
        self.step_reactive_radiating_impl(fractions, network, dt, settings, Some(table), None, None)
    }
    /// Coupled radiation, burning and dynamics with a fixed matter reservoir.
    /// Photon irradiation is still supplied explicitly by `settings.radiation`.
    /// # Errors
    /// Any table, radiation, exterior, reaction, EOS or budget failure.
    pub fn step_reactive_radiating_exterior(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        settings: ReactiveSettings,
        exterior: &crate::astrophysics_spherical::CompositionExterior,
        table: Option<&crate::astrophysics_opacity::Table>,
    ) -> Result<ReactiveExchange, ReactiveError> {
        self.step_reactive_radiating_impl(
            fractions,
            network,
            dt,
            settings,
            table,
            Some(exterior),
            None,
        )
    }
    /// Radiation and burning around equilibrium-preserving spherical transport.
    /// # Errors
    /// Any local physical/domain error or shared work-budget failure.
    #[allow(clippy::too_many_arguments)]
    pub fn step_reactive_radiating_balanced(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        settings: ReactiveSettings,
        reference: &crate::astrophysics_spherical::HydrostaticReference,
        exterior: Option<&crate::astrophysics_spherical::CompositionExterior>,
        table: Option<&crate::astrophysics_opacity::Table>,
    ) -> Result<ReactiveExchange, ReactiveError> {
        self.step_reactive_radiating_impl(
            fractions,
            network,
            dt,
            settings,
            table,
            exterior,
            Some(reference),
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn step_reactive_radiating_impl(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        settings: ReactiveSettings,
        table: Option<&crate::astrophysics_opacity::Table>,
        exterior: Option<&crate::astrophysics_spherical::CompositionExterior>,
        reference: Option<&crate::astrophysics_spherical::HydrostaticReference>,
    ) -> Result<ReactiveExchange, ReactiveError> {
        self.step_reactive_radiating_spectrum_impl(
            fractions, network, dt, settings, table, exterior, reference, None,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn step_reactive_radiating_spectrum_impl(
        &mut self,
        fractions: &mut [Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        settings: ReactiveSettings,
        table: Option<&crate::astrophysics_opacity::Table>,
        exterior: Option<&crate::astrophysics_spherical::CompositionExterior>,
        reference: Option<&crate::astrophysics_spherical::HydrostaticReference>,
        spectrum: Option<FreeFreeSpectrum>,
    ) -> Result<ReactiveExchange, ReactiveError> {
        let thermal = |sphere: &mut Self, dt, settings, rows: &[Vec<f64>]| {
            if let Some(spectrum) = spectrum {
                sphere.radiate_free_free(dt, settings, rows, network, spectrum)
            } else if let Some(table) = table {
                sphere.radiate_tabulated(dt, settings, rows, network, table)
            } else {
                sphere.radiate_composition(dt, settings, rows, network)
            }
        };
        let mut next = self.clone();
        let mut rows = fractions.to_vec();
        let first = thermal(&mut next, dt / 2.0, settings.radiation, &rows)
            .map_err(ReactiveError::Radiation)?;
        let dynamics = if let Some(reference) = reference {
            next.step_reactive_balanced(
                &mut rows,
                network,
                dt,
                crate::astrophysics_spherical::ReactiveBudget {
                    hydro_max_step: settings.hydro_max_step,
                    hydro_steps: settings.hydro_steps,
                    burn: settings.burn,
                },
                reference,
                exterior,
            )
        } else if let Some(exterior) = exterior {
            next.step_reactive_exterior(
                &mut rows,
                network,
                dt,
                crate::astrophysics_spherical::ReactiveBudget {
                    hydro_max_step: settings.hydro_max_step,
                    hydro_steps: settings.hydro_steps,
                    burn: settings.burn,
                },
                exterior,
            )
        } else {
            next.step_reactive(
                &mut rows,
                network,
                dt,
                settings.hydro_max_step,
                settings.hydro_steps,
                settings.burn,
            )
        }
        .map_err(ReactiveError::Dynamics)?;
        let second = thermal(
            &mut next,
            dt / 2.0,
            Heating {
                max_steps: settings.radiation.max_steps - first.steps,
                max_segments: settings.radiation.max_segments - first.segments,
                ..settings.radiation
            },
            &rows,
        )
        .map_err(ReactiveError::Radiation)?;
        let radiation = Exchange {
            steps: first.steps + second.steps,
            segments: first.segments + second.segments,
            escaped_energy: first.escaped_energy + second.escaped_energy,
        };
        if !radiation.escaped_energy.is_finite() {
            return Err(ReactiveError::Radiation(Error::NumericalOverflow));
        }
        *self = next;
        fractions.clone_from_slice(&rows);
        Ok(ReactiveExchange {
            dynamics,
            radiation,
        })
    }
}

impl crate::astrophysics_spherical::Sphere {
    fn table_shells(
        &self,
        cv: f64,
        mixture: Option<crate::astrophysics_eos::Mixture>,
        local: Option<&[crate::astrophysics_eos::Mixture]>,
        table: &crate::astrophysics_opacity::Table,
    ) -> Result<Vec<Shell>, Error> {
        self.totals().map_err(|_| Error::InvalidInput)?;
        let mut radius = 0.0;
        self.cells
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let temperature =
                    temperature_eos(*cell, self.gamma, cv, local.map_or(mixture, |m| Some(m[i])))?;
                let opacity = table
                    .at(cell.density, temperature)
                    .map_err(Error::Opacity)?
                    .opacity;
                Ok(Shell {
                    outer_radius: {
                        radius += self.spacing;
                        radius
                    },
                    absorption: opacity * cell.density,
                    temperature,
                })
            })
            .collect()
    }
    fn validate_table_states(
        &self,
        cv: f64,
        mixture: Option<crate::astrophysics_eos::Mixture>,
        local: Option<&[crate::astrophysics_eos::Mixture]>,
        table: &crate::astrophysics_opacity::Table,
    ) -> Result<(), Error> {
        self.table_shells(cv, mixture, local, table).map(|_| ())
    }
    fn table_rates(
        &self,
        settings: Heating,
        mixture: Option<crate::astrophysics_eos::Mixture>,
        local: Option<&[crate::astrophysics_eos::Mixture]>,
        table: &crate::astrophysics_opacity::Table,
    ) -> Result<Rates, Error> {
        transfer(
            &self.table_shells(settings.specific_heat, mixture, local, table)?,
            settings.ambient,
            settings.rays_per_annulus,
            settings.max_segments,
        )
    }
}

/// Radius where radial optical depth measured inward from the outer boundary
/// reaches `target_depth`. Returns None if the entire radius is optically thin
/// at that threshold. Exact for the piecewise-constant shell absorption model.
/// This is a radial diagnostic, not a ray-image formation radius.
/// # Errors
/// Empty/unordered geometry, nonpositive target or invalid/overflowing opacity.
pub fn optical_depth_radius(shells: &[Shell], target_depth: f64) -> Result<Option<f64>, Error> {
    if shells.is_empty() || !target_depth.is_finite() || target_depth <= 0.0 {
        return Err(Error::InvalidInput);
    }
    let mut inner = 0.0;
    for shell in shells {
        if !shell.outer_radius.is_finite()
            || shell.outer_radius <= inner
            || !shell.absorption.is_finite()
            || shell.absorption < 0.0
        {
            return Err(Error::InvalidInput);
        }
        inner = shell.outer_radius;
    }
    let mut depth = 0.0;
    let mut radius = None;
    for i in (0..shells.len()).rev() {
        let shell = shells[i];
        let inner = if i == 0 {
            0.0
        } else {
            shells[i - 1].outer_radius
        };
        let increment = shell.absorption * (shell.outer_radius - inner);
        if !increment.is_finite() || !(depth + increment).is_finite() {
            return Err(Error::NumericalOverflow);
        }
        if radius.is_none() && shell.absorption > 0.0 && depth + increment >= target_depth {
            radius = Some(
                (shell.outer_radius - (target_depth - depth) / shell.absorption)
                    .clamp(inner, shell.outer_radius),
            );
        }
        depth += increment;
    }
    Ok(radius)
}

impl crate::astrophysics_spherical::Sphere {
    /// Radial optical-depth diagnostic using current local EOS temperatures.
    /// A supplied grey absorption table overrides scalar opacity (m²/kg).
    /// The table represents a fixed composition approximation; the EOS uses
    /// each shell's actual composition. No state mutation or transfer solve.
    /// # Errors
    /// Invalid EOS/composition, wrong opacity kind, domain or geometry failure.
    pub fn composition_optical_depth_radius(
        &self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        opacity: f64,
        table: Option<&crate::astrophysics_opacity::Table>,
        target_depth: f64,
    ) -> Result<Option<f64>, Error> {
        if fractions.len() != self.cells.len() {
            return Err(Error::InvalidInput);
        }
        let local = fractions
            .iter()
            .map(|row| network.mixture(row).map_err(|_| Error::InvalidInput))
            .collect::<Result<Vec<_>, _>>()?;
        let shells = if let Some(table) = table {
            if table.kind != crate::astrophysics_opacity::Kind::GreyAbsorption {
                return Err(Error::InvalidInput);
            }
            self.table_shells(1.0, None, Some(&local), table)?
        } else {
            if !opacity.is_finite() || opacity < 0.0 {
                return Err(Error::InvalidInput);
            }
            self.totals().map_err(|_| Error::InvalidInput)?;
            let mut radius = 0.0;
            self.cells
                .iter()
                .zip(local)
                .map(|(cell, mixture)| {
                    radius += self.spacing;
                    Ok(Shell {
                        outer_radius: radius,
                        absorption: opacity * cell.density,
                        temperature: temperature_eos(*cell, self.gamma, 1.0, Some(mixture))?,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?
        };
        optical_depth_radius(&shells, target_depth)
    }
}

/// Blackbody-equivalent temperature defined by L=4*pi*R²*sigma*T⁴.
/// The supplied nonnegative luminosity must represent outward emission, not a
/// signed net irradiation loss. This definition does not impose a spectrum.
/// # Errors
/// Negative/nonfinite luminosity, nonpositive radius, or numerical overflow.
pub fn effective_temperature(luminosity: f64, radius: f64) -> Result<f64, Error> {
    if !luminosity.is_finite() || luminosity < 0.0 || !radius.is_finite() || radius <= 0.0 {
        return Err(Error::InvalidInput);
    }
    if luminosity == 0.0 {
        return Ok(0.0);
    }
    let temperature = ((luminosity.ln()
        - (4.0 * std::f64::consts::PI).ln()
        - 2.0 * radius.ln()
        - crate::astrophysics_thermal::STEFAN_BOLTZMANN.ln())
        / 4.0)
        .exp();
    if !temperature.is_finite() || temperature <= 0.0 {
        return Err(Error::NumericalOverflow);
    }
    Ok(temperature)
}
impl crate::astrophysics_spherical::Sphere {
    /// Instantaneous signed net luminosity and shell heating using local EOS.
    /// Ray segments are reported explicitly; no gas state or time is advanced.
    /// A grey absorption table overrides scalar opacity. Ambient is intensity.
    /// # Errors
    /// Invalid composition, opacity kind/domain, ray budget or transfer failure.
    pub fn composition_radiation_rates(
        &self,
        settings: Heating,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        table: Option<&crate::astrophysics_opacity::Table>,
    ) -> Result<Rates, Error> {
        if fractions.len() != self.cells.len() {
            return Err(Error::InvalidInput);
        }
        let local = fractions
            .iter()
            .map(|row| network.mixture(row).map_err(|_| Error::InvalidInput))
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(table) = table {
            if table.kind != crate::astrophysics_opacity::Kind::GreyAbsorption {
                return Err(Error::InvalidInput);
            }
            self.table_rates(settings, None, Some(&local), table)
        } else {
            self.radiation_rates_eos(
                1.0,
                settings.opacity,
                settings.ambient,
                settings.rays_per_annulus,
                settings.max_segments,
                None,
                Some(&local),
            )
        }
    }
}

/// Instantaneous thermal balance at fixed density and composition.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermalRates {
    /// Deposited nuclear thermal power per shell, W.
    pub nuclear: Vec<f64>,
    /// Escaping neutrino power per shell, W.
    pub neutrinos: Vec<f64>,
    /// Net radiation deposition per shell, W (negative for cooling).
    pub radiation: Vec<f64>,
    /// Nuclear plus radiation deposition per shell, W.
    pub net: Vec<f64>,
    /// Net outward photon luminosity, W.
    pub luminosity: f64,
    pub segments: usize,
    pub fit_evaluations: usize,
}
impl ThermalRates {
    /// Largest shell imbalance relative to its heating and cooling powers.
    /// Zero means local thermal balance; one means unopposed heating/cooling.
    /// Empty, inconsistent or nonfinite public data are rejected.
    pub fn relative_imbalance(&self) -> Result<f64, Error> {
        let n = self.net.len();
        if n == 0 || self.nuclear.len() != n || self.radiation.len() != n {
            return Err(Error::InvalidInput);
        }
        let mut maximum = 0.0_f64;
        for i in 0..n {
            let heating = self.nuclear[i];
            let radiation = self.radiation[i];
            let net = self.net[i];
            if ![heating, radiation, net].into_iter().all(f64::is_finite) {
                return Err(Error::InvalidInput);
            }
            // Scaling before addition avoids overflow for large finite powers.
            let scale = heating.abs().max(radiation.abs());
            let residual = if scale == 0.0 {
                if net != 0.0 {
                    return Err(Error::InvalidInput);
                }
                0.0
            } else {
                let expected = heating / scale + radiation / scale;
                if (net / scale - expected).abs() > 1e-12 {
                    return Err(Error::InvalidInput);
                }
                (net / scale).abs() / (heating.abs() / scale + radiation.abs() / scale)
            };
            maximum = maximum.max(residual);
        }
        Ok(maximum)
    }
}
impl crate::astrophysics_spherical::Sphere {
    /// Read-only local nuclear/radiative thermal balance with explicit budgets.
    /// This excludes compression, advection and mechanical gravity work.
    /// # Errors
    /// Invalid gas/composition, reaction/opacity domain, overflow or work budget.
    pub fn thermal_rates(
        &self,
        settings: Heating,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        table: Option<&crate::astrophysics_opacity::Table>,
        fit_budget: usize,
    ) -> Result<ThermalRates, ReactiveError> {
        let radiation = self
            .composition_radiation_rates(settings, fractions, network, table)
            .map_err(ReactiveError::Radiation)?;
        self.thermal_rates_from_radiation(radiation, fractions, network, fit_budget)
    }
    fn thermal_rates_from_radiation(
        &self,
        radiation: Rates,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        fit_budget: usize,
    ) -> Result<ThermalRates, ReactiveError> {
        let mut result = ThermalRates {
            nuclear: Vec::with_capacity(self.cells.len()),
            neutrinos: Vec::with_capacity(self.cells.len()),
            radiation: radiation.heating,
            net: Vec::with_capacity(self.cells.len()),
            luminosity: radiation.luminosity,
            segments: radiation.segments,
            fit_evaluations: 0,
        };
        let mut inner = 0.0_f64;
        for (i, (cell, row)) in self.cells.iter().zip(fractions).enumerate() {
            let mixture = network.mixture(row).map_err(|e| {
                ReactiveError::Dynamics(crate::astrophysics_spherical::Error::Nuclear(e))
            })?;
            let internal = cell.energy - 0.5 * cell.momentum.powi(2) / cell.density;
            let temperature = mixture.temperature(cell.density, internal).map_err(|e| {
                ReactiveError::Dynamics(crate::astrophysics_spherical::Error::Eos(e))
            })?;
            let rates = network
                .rates(
                    row,
                    cell.density,
                    temperature,
                    fit_budget - result.fit_evaluations,
                )
                .map_err(|e| {
                    ReactiveError::Dynamics(crate::astrophysics_spherical::Error::Nuclear(e))
                })?;
            let outer = inner + self.spacing;
            let mass = cell.density * 4.0 * std::f64::consts::PI / 3.0
                * self.spacing
                * (inner * inner + inner * outer + outer * outer);
            let nuclear = mass * rates.deposited_power;
            let neutrinos = mass * rates.escaped_neutrino_power;
            let net = nuclear + result.radiation[i];
            if ![nuclear, neutrinos, net].into_iter().all(f64::is_finite) {
                return Err(ReactiveError::Radiation(Error::NumericalOverflow));
            }
            result.nuclear.push(nuclear);
            result.neutrinos.push(neutrinos);
            result.net.push(net);
            result.fit_evaluations += rates.fit_evaluations;
            inner = outer;
        }
        Ok(result)
    }
}

/// Fixed-density, fixed-composition thermal root search. Temperature limits must
/// lie inside the supplied EOS, reaction and opacity domains.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermalSearch {
    pub min_temperature: f64,
    pub max_temperature: f64,
    pub relative_tolerance: f64,
    pub max_sweeps: usize,
    pub max_evaluations: usize,
    pub fit_evaluations: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ThermalEquilibrium {
    pub rates: ThermalRates,
    pub sweeps: usize,
    pub evaluations: usize,
    pub segments: usize,
    pub fit_evaluations: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ThermalSearchError {
    Physics(ReactiveError),
    InvalidInput,
    BudgetExceeded,
    /// The selected shell has no sign-changing bracket at the current profile.
    Unbracketed {
        shell: usize,
    },
    NoConvergence,
}
fn thermal_shell_residual(rates: &ThermalRates, shell: usize) -> f64 {
    let scale = rates.nuclear[shell].abs().max(rates.radiation[shell].abs());
    if scale == 0.0 {
        0.0
    } else {
        (rates.net[shell] / scale).abs()
            / (rates.nuclear[shell].abs() / scale + rates.radiation[shell].abs() / scale)
    }
}
impl crate::astrophysics_spherical::Sphere {
    /// Find local nuclear/radiative balance using sequential bracketed shell
    /// solves in log temperature. Density, momentum and composition stay fixed.
    /// This changes pressure and does not enforce hydrostatic equilibrium or
    /// establish thermal stability. Multiple roots need not be found; all shell
    /// roots must be bracketed within the caller's temperature interval.
    /// Radiation segment and reaction-fit budgets apply to the whole search.
    /// The sphere is committed only after the complete profile meets tolerance.
    pub fn equilibrate_thermal(
        &mut self,
        settings: Heating,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
        table: Option<&crate::astrophysics_opacity::Table>,
        search: ThermalSearch,
    ) -> Result<ThermalEquilibrium, ThermalSearchError> {
        use ThermalSearchError as E;
        if ![
            search.min_temperature,
            search.max_temperature,
            search.relative_tolerance,
        ]
        .into_iter()
        .all(f64::is_finite)
            || search.min_temperature <= 0.0
            || search.max_temperature <= search.min_temperature
            || search.relative_tolerance <= 0.0
            || search.relative_tolerance >= 1.0
            || search.max_sweeps == 0
            || search.max_evaluations == 0
            || self.cells.is_empty()
            || fractions.len() != self.cells.len()
        {
            return Err(E::InvalidInput);
        }
        let mut candidate = self.clone();
        let mut evaluations = 0usize;
        let mut segments = 0usize;
        let mut fits = 0usize;
        let mut evaluate = |sphere: &Self| -> Result<ThermalRates, E> {
            if evaluations >= search.max_evaluations {
                return Err(E::BudgetExceeded);
            }
            let mut remaining = settings;
            remaining.max_segments = settings.max_segments - segments;
            let rates = sphere
                .thermal_rates(
                    remaining,
                    fractions,
                    network,
                    table,
                    search.fit_evaluations - fits,
                )
                .map_err(E::Physics)?;
            evaluations += 1;
            segments += rates.segments;
            fits += rates.fit_evaluations;
            Ok(rates)
        };
        let mut rates = evaluate(&candidate)?;
        let mut mixtures = Vec::with_capacity(candidate.cells.len());
        for (cell, row) in candidate.cells.iter().zip(fractions) {
            let mixture = network.mixture(row).map_err(|e| {
                E::Physics(ReactiveError::Dynamics(
                    crate::astrophysics_spherical::Error::Nuclear(e),
                ))
            })?;
            let temperature = mixture
                .temperature(
                    cell.density,
                    cell.energy - 0.5 * cell.momentum.powi(2) / cell.density,
                )
                .map_err(|e| {
                    E::Physics(ReactiveError::Dynamics(
                        crate::astrophysics_spherical::Error::Eos(e),
                    ))
                })?;
            if temperature < search.min_temperature || temperature > search.max_temperature {
                return Err(E::InvalidInput);
            }
            mixtures.push(mixture);
        }
        let set_temperature =
            |sphere: &mut Self, shell: usize, temperature: f64| -> Result<(), E> {
                let cell = &mut sphere.cells[shell];
                let state = mixtures[shell].at(cell.density, temperature).map_err(|e| {
                    E::Physics(ReactiveError::Dynamics(
                        crate::astrophysics_spherical::Error::Eos(e),
                    ))
                })?;
                cell.energy =
                    state.internal_energy_density + 0.5 * cell.momentum.powi(2) / cell.density;
                Ok(())
            };
        for sweep in 0..=search.max_sweeps {
            if rates
                .relative_imbalance()
                .map_err(|e| E::Physics(ReactiveError::Radiation(e)))?
                <= search.relative_tolerance
            {
                // End the closure borrow before reporting cumulative counters.
                *self = candidate;
                return Ok(ThermalEquilibrium {
                    rates,
                    sweeps: sweep,
                    evaluations,
                    segments,
                    fit_evaluations: fits,
                });
            }
            if sweep == search.max_sweeps {
                return Err(E::NoConvergence);
            }
            for shell in 0..candidate.cells.len() {
                let mut lo = search.min_temperature.ln();
                let mut hi = search.max_temperature.ln();
                set_temperature(&mut candidate, shell, search.min_temperature)?;
                let low = evaluate(&candidate)?;
                if thermal_shell_residual(&low, shell) <= search.relative_tolerance {
                    continue;
                }
                let mut low_sign = low.net[shell].is_sign_positive();
                set_temperature(&mut candidate, shell, search.max_temperature)?;
                let high = evaluate(&candidate)?;
                if thermal_shell_residual(&high, shell) <= search.relative_tolerance {
                    continue;
                }
                if low_sign == high.net[shell].is_sign_positive() {
                    return Err(E::Unbracketed { shell });
                }
                loop {
                    let middle = lo + 0.5 * (hi - lo);
                    if middle == lo || middle == hi {
                        return Err(E::NoConvergence);
                    }
                    set_temperature(&mut candidate, shell, middle.exp())?;
                    let trial = evaluate(&candidate)?;
                    if thermal_shell_residual(&trial, shell) <= search.relative_tolerance {
                        break;
                    }
                    let sign = trial.net[shell].is_sign_positive();
                    if sign == low_sign {
                        lo = middle;
                        low_sign = sign;
                    } else {
                        hi = middle;
                    }
                }
            }
            rates = evaluate(&candidate)?;
        }
        Err(E::NoConvergence)
    }
}

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
    pub absorption: crate::astrophysics_opacity::FreeFree,
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
    let x = 6.626_070_15e-34 * frequency / (crate::astrophysics_eos::BOLTZMANN * temperature);
    if x > 700.0 {
        return Ok(0.0);
    }
    let value = 2.0 * 6.626_070_15e-34 * frequency.powi(3) * width
        / crate::astrophysics_eos::LIGHT_SPEED.powi(2)
        / x.exp_m1();
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(Error::NumericalOverflow)
    }
}
impl crate::astrophysics_spherical::Sphere {
    /// Frequency-dependent free-free chord transport with local composition.
    /// Returns finite-band heating and luminosity in W. Frequency/angular
    /// convergence and adequacy of the chosen band are caller responsibilities.
    /// Work budget includes every frequency bin, shell chord and angular sample.
    pub fn free_free_radiation_rates(
        &self,
        fractions: &[Vec<f64>],
        network: &crate::astrophysics_nuclear::Network,
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
                    .map(|(x, nucleus)| crate::astrophysics_eos::Species {
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
        network: &crate::astrophysics_nuclear::Network,
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

impl crate::astrophysics_spherical::Sphere {
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
        network: &crate::astrophysics_nuclear::Network,
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
                    .map(|(x, nucleus)| crate::astrophysics_eos::Species {
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
                if eos.sound_speed_squared >= crate::astrophysics_eos::LIGHT_SPEED.powi(2) {
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
        network: &crate::astrophysics_nuclear::Network,
        dt: f64,
        settings: ReactiveSettings,
        spectrum: FreeFreeSpectrum,
        reference: Option<&crate::astrophysics_spherical::HydrostaticReference>,
        exterior: Option<&crate::astrophysics_spherical::CompositionExterior>,
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
