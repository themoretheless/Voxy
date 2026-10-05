//! Isothermal latent-heat transition with conservative pair enthalpy exchange.
use super::{Error, Liquid, LiquidField, Material, Particle, positive, transport::Transport};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhaseChange {
    /// Fixed transition temperature unless an optional saturation curve overrides it.
    pub temperature: f64,
    /// Energy per mass required to convert the low-temperature phase to the high phase.
    pub latent_heat: f64,
    /// Mechanical properties of the high-temperature phase, before field responses.
    pub high_phase: Material,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PhaseState {
    pub models: Vec<Option<PhaseChange>>,
    pub fractions: Vec<f64>,
    pub saturation: Option<super::saturation::SaturationState>,
}
impl Liquid {
    /// Configures one reversible thermal transition per material on existing fields.
    /// Fractions describe the high-temperature phase and must agree with temperature:
    /// zero below transition, one above it, any value in [0,1] on the plateau.
    /// This changes the energy reference; it does not inject heat automatically.
    /// # Errors
    /// Invalid models/fractions, absent transport or nonfinite resulting energy/properties.
    #[allow(clippy::float_cmp)] // Phase endpoints must be exactly pure outside the plateau.
    pub fn configure_phase_change(
        &mut self,
        models: Vec<Option<PhaseChange>>,
        fractions: Vec<f64>,
    ) -> Result<(), Error> {
        if models
            .iter()
            .zip(&self.gas_equations)
            .any(|(phase, gas)| phase.is_some() && gas.is_some())
            || models.len() != self.materials.len()
            || fractions.len() != self.particles.len()
            || models.iter().flatten().any(|model| {
                !positive(model.temperature)
                    || !positive(model.latent_heat)
                    || !positive(model.high_phase.rest_density)
                    || !positive(model.high_phase.sound_speed)
                    || !model.high_phase.viscosity.is_finite()
                    || model.high_phase.viscosity < 0.0
            })
        {
            return Err(Error::InvalidPhaseChange);
        }
        let mut candidate = self.transport.clone().ok_or(Error::InvalidPhaseChange)?;
        for (model, material) in models.iter().zip(&candidate.materials) {
            if model.is_some_and(|model| {
                !(material.specific_heat * model.temperature + model.latent_heat).is_finite()
            }) {
                return Err(Error::InvalidPhaseChange);
            }
        }
        for (index, ((particle, field), fraction)) in self
            .particles
            .iter()
            .zip(&candidate.fields)
            .zip(&fractions)
            .enumerate()
        {
            if !fraction.is_finite() || !(0.0..=1.0).contains(fraction) {
                return Err(Error::InvalidPhaseChange);
            }
            if let Some(model) = models[particle.material] {
                if !(candidate.specific_heat(index, particle.material)? * model.temperature
                    + model.latent_heat)
                    .is_finite()
                {
                    return Err(Error::InvalidPhaseChange);
                }
                if (field.temperature < model.temperature && *fraction != 0.0)
                    || (field.temperature > model.temperature && *fraction != 1.0)
                {
                    return Err(Error::InvalidPhaseChange);
                }
            } else if *fraction != 0.0 {
                return Err(Error::InvalidPhaseChange);
            }
        }
        candidate.phase = if models.iter().any(Option::is_some) {
            Some(PhaseState {
                models,
                fractions,
                saturation: None,
            })
        } else {
            None
        };
        let mut total_energy = 0.0;
        for (index, (particle, field)) in self.particles.iter().zip(&candidate.fields).enumerate() {
            total_energy += candidate.energy(particle, field, index)?;
            if !total_energy.is_finite() {
                return Err(Error::NumericalFailure);
            }
        }
        self.evaluate_materials(&self.particles, Some(&candidate))?;
        self.transport = Some(candidate);
        Ok(())
    }
    #[must_use]
    pub fn phase_fractions(&self) -> Option<&[f64]> {
        self.transport
            .as_ref()?
            .phase
            .as_ref()
            .map(|phase| phase.fractions.as_slice())
    }
    /// Adds signed energy to each particle. Includes latent heat when configured.
    /// # Errors
    /// Missing transport, wrong length, nonfinite energy, negative final enthalpy,
    /// or an invalid mechanical property response. The whole operation is atomic.
    pub fn add_heat(&mut self, energy: &[f64]) -> Result<(), Error> {
        if energy.len() != self.particles.len() || energy.iter().any(|value| !value.is_finite()) {
            return Err(Error::InvalidPhaseChange);
        }
        let mut candidate = self.transport.clone().ok_or(Error::InvalidTransport)?;
        for (index, (particle, delta)) in self.particles.iter().zip(energy).enumerate() {
            let current = candidate.energy(particle, &candidate.fields[index], index)?;
            candidate.set_energy(index, particle, current + delta)?;
        }
        let mut total_energy = 0.0;
        for (index, particle) in self.particles.iter().enumerate() {
            total_energy += candidate.energy(particle, &candidate.fields[index], index)?;
            if !total_energy.is_finite() {
                return Err(Error::NumericalFailure);
            }
        }
        self.evaluate_materials(&self.particles, Some(&candidate))?;
        self.transport = Some(candidate);
        Ok(())
    }
}
impl Transport {
    pub(super) fn energy(
        &self,
        particle: &Particle,
        field: &LiquidField,
        index: usize,
    ) -> Result<f64, Error> {
        let latent = self
            .phase
            .as_ref()
            .and_then(|phase| {
                phase.models[particle.material]
                    .map(|model| model.latent_heat * phase.fractions[index])
            })
            .unwrap_or(0.0);
        let energy = particle.mass
            * (self.specific_heat(index, particle.material)? * field.temperature + latent);
        if !energy.is_finite() || energy < 0.0 {
            return Err(Error::NumericalFailure);
        }
        Ok(energy)
    }
    pub(super) fn decode(
        &self,
        particle: &Particle,
        energy: f64,
        index: usize,
    ) -> Result<(f64, f64), Error> {
        self.decode_capacity(
            particle,
            energy,
            index,
            self.specific_heat(index, particle.material)?,
        )
    }
    pub(super) fn decode_capacity(
        &self,
        particle: &Particle,
        energy: f64,
        index: usize,
        capacity: f64,
    ) -> Result<(f64, f64), Error> {
        if !energy.is_finite() || energy < 0.0 || !positive(capacity) {
            return Err(Error::NumericalFailure);
        }
        let specific = energy / particle.mass;
        let (temperature, fraction) = if let Some(model) = self
            .phase
            .as_ref()
            .and_then(|phase| phase.model(index, particle.material))
        {
            let start = capacity * model.temperature;
            let end = start + model.latent_heat;
            if !end.is_finite() {
                return Err(Error::NumericalFailure);
            }
            if specific < start {
                (specific / capacity, 0.0)
            } else if specific <= end {
                (
                    model.temperature,
                    ((specific - start) / model.latent_heat).clamp(0.0, 1.0),
                )
            } else {
                ((specific - model.latent_heat) / capacity, 1.0)
            }
        } else {
            (specific / capacity, 0.0)
        };
        if !temperature.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok((temperature, fraction))
    }
    pub(super) fn set_energy(
        &mut self,
        index: usize,
        particle: &Particle,
        energy: f64,
    ) -> Result<(), Error> {
        let (temperature, fraction) = self.decode(particle, energy, index)?;
        self.fields[index].temperature = temperature;
        if let Some(phase) = &mut self.phase {
            phase.fractions[index] = fraction;
        }
        Ok(())
    }
    pub(super) fn phase_material(
        &self,
        index: usize,
        material: usize,
        base: Material,
    ) -> Result<Material, Error> {
        let Some(phase) = &self.phase else {
            return Ok(base);
        };
        let Some(model) = phase.models[material] else {
            return Ok(base);
        };
        let fraction = phase.fractions[index];
        let high = model.high_phase;
        let result = Material {
            rest_density: 1.0
                / ((1.0 - fraction) / base.rest_density + fraction / high.rest_density),
            sound_speed: (1.0 - fraction) * base.sound_speed + fraction * high.sound_speed,
            viscosity: (1.0 - fraction) * base.viscosity + fraction * high.viscosity,
        };
        if !positive(result.rest_density)
            || !positive(result.sound_speed)
            || !result.viscosity.is_finite()
        {
            return Err(Error::NumericalFailure);
        }
        Ok(result)
    }
    #[allow(clippy::float_cmp)] // Equal plateau temperatures drive exactly zero conductive heat.
    pub(super) fn exchange_phase_heat(
        &mut self,
        i: usize,
        j: usize,
        particles: &[Particle],
        conductance: f64,
        dt: f64,
    ) -> Result<(), Error> {
        if !conductance.is_finite() || conductance < 0.0 {
            return Err(Error::NumericalFailure);
        }
        if conductance == 0.0 || self.fields[i].temperature == self.fields[j].temperature {
            return Ok(());
        }
        let (hot, cold) = if self.fields[i].temperature > self.fields[j].temperature {
            (i, j)
        } else {
            (j, i)
        };
        let hot_energy = self.energy(&particles[hot], &self.fields[hot], hot)?;
        let cold_energy = self.energy(&particles[cold], &self.fields[cold], cold)?;
        let inverse_rate = (1.0 / conductance) / dt;
        // Solve implicit q = G*dt*(T_hot(E_hot-q)-T_cold(E_cold+q)).
        // Temperature is monotone in enthalpy, including the latent-heat plateau.
        let transfer =
            super::transport::implicit_heat_transfer(0., hot_energy, inverse_rate, |transfer| {
                let hot_temperature = self.decode(&particles[hot], hot_energy - transfer, hot)?.0;
                let cold_temperature = self
                    .decode(&particles[cold], cold_energy + transfer, cold)?
                    .0;
                Ok(hot_temperature - cold_temperature)
            })?;
        self.set_energy(hot, &particles[hot], hot_energy - transfer)?;
        self.set_energy(cold, &particles[cold], cold_energy + transfer)?;
        Ok(())
    }
}

impl PhaseState {
    pub(super) fn model(&self, index: usize, material: usize) -> Option<PhaseChange> {
        self.models[material].map(|mut model| {
            if let Some(temperature) = self.saturation.as_ref().and_then(|s| s.temperatures[index])
            {
                model.temperature = temperature;
            }
            model
        })
    }
}
