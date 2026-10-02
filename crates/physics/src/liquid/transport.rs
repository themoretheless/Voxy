//! Conservative pair exchange of heat and a dissolved mass fraction.
use super::{Error, Liquid, Particle, finite, norm, positive, sub};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiquidField {
    /// Absolute temperature (K in an SI simulation).
    pub temperature: f64,
    /// Dissolved mass fraction in [0,1]. It moves with the owning particle.
    pub concentration: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransportMaterial {
    /// Energy capacity per unit mass and temperature; gas EOS requires cv.
    pub specific_heat: f64,
    pub conductivity: f64,
    pub diffusivity: f64,
    /// Nonzero matching groups permit dissolved fraction exchange across material IDs.
    /// Equal material IDs always permit diffusion; group zero keeps other IDs separate.
    pub mixing_group: u32,
}
impl Default for TransportMaterial {
    fn default() -> Self {
        Self {
            specific_heat: 4184.0,
            conductivity: 0.6,
            diffusivity: 0.0,
            mixing_group: 0,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Transport {
    pub fields: Vec<LiquidField>,
    pub materials: Vec<TransportMaterial>,
    pub phase: Option<super::phase::PhaseState>,
    pub species: Option<super::species::SpeciesState>,
}
impl Liquid {
    /// Enables transported temperature and one dissolved species on existing particles.
    /// Configuring again replaces the fields; it does not reset particle motion.
    /// # Errors
    /// Rejects array length mismatches, negative/nonfinite temperatures or coefficients,
    /// invalid concentrations and nonpositive heat capacities without modifying state.
    pub fn configure_transport(
        &mut self,
        fields: Vec<LiquidField>,
        materials: Vec<TransportMaterial>,
    ) -> Result<(), Error> {
        if self
            .transport
            .as_ref()
            .is_some_and(|transport| transport.phase.is_some())
        {
            return Err(Error::InvalidPhaseChange);
        }
        if fields.len() != self.particles.len()
            || materials.len() != self.materials.len()
            || fields.iter().any(|f| {
                !f.temperature.is_finite()
                    || f.temperature < 0.0
                    || !f.concentration.is_finite()
                    || !(0.0..=1.0).contains(&f.concentration)
            })
            || materials.iter().any(|m| {
                !positive(m.specific_heat)
                    || !m.conductivity.is_finite()
                    || m.conductivity < 0.0
                    || !m.diffusivity.is_finite()
                    || m.diffusivity < 0.0
            })
        {
            return Err(Error::InvalidTransport);
        }
        let mut total_heat = 0.0;
        for (particle, field) in self.particles.iter().zip(&fields) {
            let capacity = particle.mass * materials[particle.material].specific_heat;
            total_heat += capacity * field.temperature;
            if !positive(capacity) || !total_heat.is_finite() {
                return Err(Error::NumericalFailure);
            }
        }
        for (particle, field) in self.particles.iter().zip(&fields) {
            if let Some(response) = self.property_responses[particle.material] {
                response.evaluate(self.materials[particle.material], *field)?;
            }
        }
        let candidate = Transport {
            fields,
            materials,
            phase: None,
            species: self.transport.as_ref().and_then(|t| t.species.clone()),
        };
        for (i, p) in self.particles.iter().enumerate() {
            candidate.energy(p, &candidate.fields[i], i)?;
        }
        self.evaluate_materials(&self.particles, Some(&candidate))?;
        self.transport = Some(candidate);
        Ok(())
    }
    #[must_use]
    pub fn fields(&self) -> Option<&[LiquidField]> {
        self.transport.as_ref().map(|t| t.fields.as_slice())
    }
    /// Returns total thermal energy and dissolved mass, when transport is enabled.
    /// # Errors
    /// Rejects numerical overflow. Absence of configured fields returns `None`.
    pub fn transport_totals(&self) -> Result<Option<(f64, f64)>, Error> {
        let Some(transport) = &self.transport else {
            return Ok(None);
        };
        let mut heat = 0.0;
        let mut dissolved = 0.0;
        for (index, (particle, field)) in self.particles.iter().zip(&transport.fields).enumerate() {
            heat += transport.energy(particle, field, index)?;
            dissolved += particle.mass * field.concentration;
        }
        if !heat.is_finite() || !dissolved.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(Some((heat, dissolved)))
    }
}
impl Transport {
    pub(super) fn advance(
        &mut self,
        particles: &[Particle],
        pairs: &[(usize, usize)],
        densities: &[super::Material],
        h: f64,
        dt: f64,
    ) -> Result<(), Error> {
        self.react(particles, 0.5 * dt)?;
        for &(i, j) in pairs {
            let a = particles[i];
            let b = particles[j];
            let first = self.materials[a.material];
            let second = self.materials[b.material];
            let shape = (1.0 - norm(sub(a.position, b.position)) / h).max(0.0);
            let capacity_a = a.mass * self.specific_heat(i, a.material)?;
            let capacity_b = b.mass * self.specific_heat(j, b.material)?;
            if !positive(capacity_a) || !positive(capacity_b) {
                return Err(Error::NumericalFailure);
            }
            let volume_a = a.mass / densities[i].rest_density;
            let volume_b = b.mass / densities[j].rest_density;
            // Effective conductivity times a representative area / separation scale.
            let heat_conductance = harmonic(first.conductivity, second.conductivity)
                * volume_a.min(volume_b).powf(2.0 / 3.0)
                / h
                * shape;
            if self.phase.as_ref().is_some_and(|phase| {
                phase.models[a.material].is_some() || phase.models[b.material].is_some()
            }) {
                self.exchange_phase_heat(i, j, particles, heat_conductance, dt)?;
            } else {
                let (left, right) = exchange(
                    self.fields[i].temperature,
                    self.fields[j].temperature,
                    capacity_a,
                    capacity_b,
                    heat_conductance,
                    dt,
                )?;
                self.fields[i].temperature = left;
                self.fields[j].temperature = right;
            }
            if a.material == b.material
                || (first.mixing_group != 0 && first.mixing_group == second.mixing_group)
            {
                let diffusion = harmonic(first.diffusivity, second.diffusivity);
                let conductance = diffusion / (1.0 / a.mass + 1.0 / b.mass) / (h * h) * shape;
                let (left, right) = exchange(
                    self.fields[i].concentration,
                    self.fields[j].concentration,
                    a.mass,
                    b.mass,
                    conductance,
                    dt,
                )?;
                self.fields[i].concentration = left;
                self.fields[j].concentration = right;
                self.exchange_species([i, j], particles, conductance, dt)?;
            }
        }
        self.react(particles, 0.5 * dt)?;
        if let Some(species) = &self.species {
            species.validate(particles.len())?;
        }
        if self.fields.iter().any(|f| {
            !finite([f.temperature, f.concentration, 0.0])
                || f.temperature < 0.0
                || !(0.0..=1.0).contains(&f.concentration)
        }) {
            return Err(Error::NumericalFailure);
        }
        Ok(())
    }
}
fn harmonic(a: f64, b: f64) -> f64 {
    if a == 0.0 || b == 0.0 {
        0.0
    } else {
        let min = a.min(b);
        2.0 * (min / (1.0 + min / a.max(b)))
    }
}
pub(super) fn exchange(
    a: f64,
    b: f64,
    ca: f64,
    cb: f64,
    conductance: f64,
    dt: f64,
) -> Result<(f64, f64), Error> {
    if !conductance.is_finite() || conductance < 0.0 {
        return Err(Error::NumericalFailure);
    }
    let reduced = 1.0 / (1.0 / ca + 1.0 / cb);
    let decay = -(-conductance / reduced * dt).exp_m1();
    // Convex combinations avoid overshoot even at extremely fast exchange rates.
    let left = a + (b - a) * (reduced / ca) * decay;
    let right = b + (a - b) * (reduced / cb) * decay;
    if !left.is_finite() || !right.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok((left, right))
}

impl Liquid {
    pub(super) fn advance_thermal(
        &self,
        particles: &mut [Particle],
        pairs: &[(usize, usize)],
        properties: &[super::Material],
        density: &[f64],
        fields: &mut Transport,
        step: f64,
    ) -> Result<Option<Vec<[f64; 3]>>, Error> {
        fields.advance(
            particles,
            pairs,
            properties,
            self.config.smoothing_radius,
            step,
        )?;
        let pressure_acceleration = if self.pressure_work {
            Some(self.exchange_pressure_work(particles, pairs, properties, density, fields, step)?)
        } else {
            None
        };
        if self.viscous_heating {
            match self.viscous_integrator {
                super::ViscousIntegrator::Sequential => {
                    self.dissipate_viscosity(particles, pairs, properties, density, fields, step)?;
                }
                super::ViscousIntegrator::Symmetric => {
                    self.dissipate_symmetric_viscosity(
                        particles, pairs, properties, density, fields, step,
                    )?;
                }
                super::ViscousIntegrator::Midpoint => {
                    // Predict from the current intermediate state, after thermal/pressure work.
                    // Discard predictor heating and apply the full damping from this state.
                    let mut predictor = self.clone();
                    predictor.particles = particles.to_vec();
                    predictor.transport = Some(fields.clone());
                    let start_properties = predictor.effective_materials()?;
                    let start_density = predictor.forces(particles, pairs, &start_properties)?.0;
                    let mut predicted_particles = particles.to_vec();
                    let mut predicted_fields = fields.clone();
                    predictor.dissipate_symmetric_viscosity(
                        &mut predicted_particles,
                        pairs,
                        &start_properties,
                        &start_density,
                        &mut predicted_fields,
                        0.5 * step,
                    )?;
                    let midpoint_properties = predictor
                        .evaluate_materials(&predicted_particles, Some(&predicted_fields))?;
                    let midpoint_density =
                        predictor.forces(particles, pairs, &midpoint_properties)?.0;
                    self.dissipate_symmetric_viscosity(
                        particles,
                        pairs,
                        &midpoint_properties,
                        &midpoint_density,
                        fields,
                        step,
                    )?;
                }
            }
        }
        // Validate the final field domain even on the last integration substep.
        self.evaluate_materials(particles, Some(fields))?;
        Ok(pressure_acceleration)
    }
}
