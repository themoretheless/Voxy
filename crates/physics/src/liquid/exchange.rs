//! Atomic whole-particle sources and drains with an explicit conservation ledger.
use super::{Error, Liquid, LiquidField, Particle, finite, positive};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleInput {
    pub particle: Particle,
    /// Required exactly when thermal transport is enabled.
    pub field: Option<LiquidField>,
    /// Required exactly when a phase model is enabled, including zero for inert materials.
    pub phase_fraction: Option<f64>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ExchangeTotals {
    pub mass: f64,
    pub momentum: [f64; 3],
    pub kinetic_energy: f64,
    /// Polymer free energy transported out with removed particles; separate from heat.
    pub polymer_energy: f64,
    /// Present exactly when transport is enabled; includes latent heat.
    pub thermal_energy: Option<f64>,
    pub dissolved_mass: Option<f64>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleExchange {
    pub added: ExchangeTotals,
    pub removed: ExchangeTotals,
}
impl Liquid {
    /// Atomically removes indexed particles and appends source particles in input order.
    /// Removed indices refer to the pre-edit array; survivors retain order. All transported
    /// fields and phase fractions follow their particle. Whole particles only, no resampling.
    /// # Errors
    /// Duplicate/invalid indices, invalid particle/field/phase, particle-budget excess,
    /// overflow or invalid resulting constitutive properties. No partial edits are committed.
    pub fn exchange_particles(
        &mut self,
        remove: &[usize],
        add: &[ParticleInput],
    ) -> Result<ParticleExchange, Error> {
        self.exchange_particles_impl(remove, add, None, None)
    }
    /// Adds particles with complete mass-fraction rows in configured species order.
    /// Surviving particles retain composition. Requires configured species transport.
    /// # Errors
    /// Same errors as ordinary exchange, plus missing/invalid source compositions.
    /// The particle, thermal, phase and species edits are committed together.
    pub fn exchange_particles_with_species(
        &mut self,
        remove: &[usize],
        add: &[ParticleInput],
        compositions: &[Vec<f64>],
    ) -> Result<ParticleExchange, Error> {
        self.exchange_particles_impl(remove, add, Some(compositions), None)
    }
    /// Adds sources with prescribed absolute pressures when saturation is configured.
    /// Optional compositions are required exactly when species transport is configured.
    /// # Errors
    /// Invalid pressure/composition/source rows or the ordinary exchange errors. Atomic.
    pub fn exchange_particles_at_pressure(
        &mut self,
        remove: &[usize],
        add: &[ParticleInput],
        pressures: &[f64],
        compositions: Option<&[Vec<f64>]>,
    ) -> Result<ParticleExchange, Error> {
        self.exchange_particles_impl(remove, add, compositions, Some(pressures))
    }
    fn exchange_particles_impl(
        &mut self,
        remove: &[usize],
        add: &[ParticleInput],
        compositions: Option<&[Vec<f64>]>,
        pressures: Option<&[f64]>,
    ) -> Result<ParticleExchange, Error> {
        if remove.len() > self.particles.len() || add.len() > self.config.max_particles {
            return Err(Error::ParticleBudget);
        }
        if remove.is_empty() && add.is_empty() {
            self.validate_source_pressures(0, pressures)?;
            self.validate_source_species(0, compositions)?;
            let empty = self.exchange_totals(std::iter::empty())?;
            return Ok(ParticleExchange {
                added: empty,
                removed: empty,
            });
        }
        let mut mask = vec![false; self.particles.len()];
        for &index in remove {
            let entry = mask.get_mut(index).ok_or(Error::InvalidParticle)?;
            if *entry {
                return Err(Error::InvalidParticle);
            }
            *entry = true;
        }
        let survivors = self.particles.len() - remove.len();
        if add.len() > self.config.max_particles - survivors {
            return Err(Error::ParticleBudget);
        }
        self.validate_source_pressures(add.len(), pressures)?;
        self.validate_source_species(add.len(), compositions)?;
        let removed = self.exchange_totals(remove.iter().copied())?;
        let mut candidate = self.clone();
        candidate.particles = self
            .particles
            .iter()
            .enumerate()
            .filter_map(|(index, particle)| (!mask[index]).then_some(*particle))
            .collect();
        candidate.droplet_population = self.droplet_population.as_ref().map(|flags| {
            flags
                .iter()
                .enumerate()
                .filter_map(|(i, v)| (!mask[i]).then_some(*v))
                .collect()
        });
        candidate.structure = self
            .structure
            .iter()
            .enumerate()
            .filter_map(|(i, value)| (!mask[i]).then_some(*value))
            .collect();
        candidate.conformation = self
            .conformation
            .iter()
            .enumerate()
            .filter_map(|(i, c)| (!mask[i]).then_some(*c))
            .collect();
        candidate.retain_transport(&mask);
        for (source_index, source) in add.iter().enumerate() {
            let particle = source.particle;
            self.validate_source_particle(&particle)?;
            match (&mut candidate.transport, source.field) {
                (Some(transport), Some(field)) => {
                    if !positive(
                        particle.mass * transport.materials[particle.material].specific_heat,
                    ) {
                        return Err(Error::NumericalFailure);
                    }
                    if !field.temperature.is_finite()
                        || field.temperature < 0.0
                        || !field.concentration.is_finite()
                        || !(0.0..=1.0).contains(&field.concentration)
                    {
                        return Err(Error::InvalidTransport);
                    }
                    transport.fields.push(field);
                    if let Some(species) = &mut transport.species {
                        let rows = compositions.ok_or(Error::InvalidTransport)?;
                        species.fractions.push(rows[source_index].clone());
                    }
                    match (&mut transport.phase, source.phase_fraction) {
                        (Some(phase), Some(fraction)) => {
                            phase.fractions.push(fraction);
                            if let Some(saturation) = &mut phase.saturation {
                                let pressure =
                                    pressures.ok_or(Error::InvalidPhaseChange)?[source_index];
                                saturation.pressures.push(pressure);
                                saturation.temperatures.push(
                                    saturation.curves[particle.material]
                                        .map(|c| c.temperature(pressure))
                                        .transpose()?,
                                );
                            }
                        }
                        (None, None) => {}
                        _ => return Err(Error::InvalidPhaseChange),
                    }
                }
                (None, None) if source.phase_fraction.is_none() => {}
                _ => return Err(Error::InvalidTransport),
            }
            candidate.particles.push(particle);
            candidate.structure.push(1.0);
            candidate.conformation.push(super::viscoelastic::IDENTITY);
            if let Some(flags) = &mut candidate.droplet_population {
                flags.push(false);
            }
        }
        candidate.validate_exchange_phases()?;
        candidate.validate_reflecting_positions(&candidate.particles)?;
        candidate.evaluate_materials(&candidate.particles, candidate.transport.as_ref())?;
        let added = candidate.exchange_totals(survivors..candidate.particles.len())?;
        candidate.exchange_totals(0..candidate.particles.len())?;
        *self = candidate;
        Ok(ParticleExchange { added, removed })
    }
    fn retain_transport(&mut self, mask: &[bool]) {
        if let Some(transport) = &mut self.transport {
            if let Some(species) = &mut transport.species {
                species.fractions = species
                    .fractions
                    .iter()
                    .enumerate()
                    .filter_map(|(index, row)| (!mask[index]).then_some(row.clone()))
                    .collect();
            }
            transport.fields = transport
                .fields
                .iter()
                .enumerate()
                .filter_map(|(index, field)| (!mask[index]).then_some(*field))
                .collect();
            if let Some(phase) = &mut transport.phase {
                if let Some(saturation) = &mut phase.saturation {
                    saturation.pressures = saturation
                        .pressures
                        .iter()
                        .enumerate()
                        .filter_map(|(i, value)| (!mask[i]).then_some(*value))
                        .collect();
                    saturation.temperatures = saturation
                        .temperatures
                        .iter()
                        .enumerate()
                        .filter_map(|(i, value)| (!mask[i]).then_some(*value))
                        .collect();
                }
                phase.fractions = phase
                    .fractions
                    .iter()
                    .enumerate()
                    .filter_map(|(index, fraction)| (!mask[index]).then_some(*fraction))
                    .collect();
            }
        }
    }
    fn validate_source_pressures(
        &self,
        added: usize,
        pressures: Option<&[f64]>,
    ) -> Result<(), Error> {
        let enabled = self
            .transport
            .as_ref()
            .and_then(|t| t.phase.as_ref())
            .is_some_and(|p| p.saturation.is_some());
        match (enabled, pressures) {
            (true, Some(rows)) if rows.len() == added && rows.iter().all(|p| positive(*p)) => {
                Ok(())
            }
            (_, None) if !enabled || added == 0 => Ok(()),
            _ => Err(Error::InvalidPhaseChange),
        }
    }
    #[allow(clippy::float_cmp)] // Pure phases must be exact outside their local plateau.
    fn validate_exchange_phases(&mut self) -> Result<(), Error> {
        if let Some(phase) = self
            .transport
            .as_ref()
            .and_then(|transport| transport.phase.as_ref())
        {
            if phase.saturation.is_some() {
                let fields = self.transport.as_ref().ok_or(Error::InvalidTransport)?;
                for (i, p) in self.particles.iter().enumerate() {
                    let fraction = phase.fractions[i];
                    let temperature = fields.fields[i].temperature;
                    if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
                        return Err(Error::InvalidPhaseChange);
                    }
                    if let Some(model) = phase.model(i, p.material) {
                        if (temperature < model.temperature && fraction != 0.0)
                            || (temperature > model.temperature && fraction != 1.0)
                        {
                            return Err(Error::InvalidPhaseChange);
                        }
                    } else if fraction != 0.0 {
                        return Err(Error::InvalidPhaseChange);
                    }
                }
            } else {
                self.configure_phase_change(phase.models.clone(), phase.fractions.clone())?;
            }
        }
        Ok(())
    }
    fn validate_source_species(
        &self,
        added: usize,
        compositions: Option<&[Vec<f64>]>,
    ) -> Result<(), Error> {
        let species = self.transport.as_ref().and_then(|t| t.species.as_ref());
        match (species, compositions) {
            (Some(species), Some(rows)) if rows.len() == added => {
                for row in rows {
                    super::species::validate_row(row, species.names.len())?;
                }
            }
            (Some(_), None) if added == 0 => {}
            (None, None) => {}
            _ => return Err(Error::InvalidTransport),
        }
        Ok(())
    }
    fn validate_source_particle(&self, particle: &Particle) -> Result<(), Error> {
        if !finite(particle.position)
            || !finite(particle.velocity)
            || !positive(particle.mass)
            || particle.material >= self.materials.len()
        {
            return Err(Error::InvalidParticle);
        }
        Ok(())
    }
    fn exchange_totals(
        &self,
        indices: impl Iterator<Item = usize>,
    ) -> Result<ExchangeTotals, Error> {
        let mut total = ExchangeTotals {
            thermal_energy: self.transport.as_ref().map(|_| 0.0),
            dissolved_mass: self.transport.as_ref().map(|_| 0.0),
            ..ExchangeTotals::default()
        };
        let properties = self.evaluate_materials(&self.particles, self.transport.as_ref())?;
        for index in indices {
            let particle = self.particles[index];
            total.polymer_energy +=
                self.polymer_particle_energy(index, &particle, properties[index])?;
            total.mass += particle.mass;
            for (axis, momentum) in total.momentum.iter_mut().enumerate() {
                *momentum += particle.mass * particle.velocity[axis];
            }
            total.kinetic_energy += 0.5
                * particle.mass
                * particle
                    .velocity
                    .iter()
                    .map(|speed| speed * speed)
                    .sum::<f64>();
            if let Some(transport) = &self.transport {
                *total
                    .thermal_energy
                    .as_mut()
                    .ok_or(Error::InvalidTransport)? +=
                    transport.energy(&particle, &transport.fields[index], index)?;
                *total
                    .dissolved_mass
                    .as_mut()
                    .ok_or(Error::InvalidTransport)? +=
                    particle.mass * transport.fields[index].concentration;
            }
        }
        if !total.mass.is_finite()
            || !finite(total.momentum)
            || !total.kinetic_energy.is_finite()
            || !total.polymer_energy.is_finite()
            || total.thermal_energy.is_some_and(|value| !value.is_finite())
            || total.dissolved_mass.is_some_and(|value| !value.is_finite())
        {
            return Err(Error::NumericalFailure);
        }
        Ok(total)
    }
}
