//! Optional mechanical property response to transported temperature and solute fraction.
use super::{Error, Liquid, LiquidField, Material, Particle, positive, transport::Transport};

/// Application-supplied constitutive coefficients, not calibrated substance presets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropertyResponse {
    pub reference_temperature: f64,
    /// Volumetric expansion per temperature unit: `rho = rho_ref / (1 + beta * delta_T)`.
    pub thermal_expansion: f64,
    /// `viscosity = mixed_viscosity * exp(-rate * delta_T)`.
    pub viscosity_temperature_rate: f64,
    /// Dissolved end-member properties at concentration one. None keeps the base properties.
    pub solute: Option<Material>,
}
impl PropertyResponse {
    fn validate(self) -> Result<(), Error> {
        if !self.reference_temperature.is_finite()
            || self.reference_temperature < 0.0
            || !self.thermal_expansion.is_finite()
            || !self.viscosity_temperature_rate.is_finite()
            || self.solute.is_some_and(|m| {
                !positive(m.rest_density)
                    || !positive(m.sound_speed)
                    || !m.viscosity.is_finite()
                    || m.viscosity < 0.0
            })
        {
            return Err(Error::InvalidPropertyResponse);
        }
        Ok(())
    }
    pub(super) fn evaluate(self, base: Material, field: LiquidField) -> Result<Material, Error> {
        let end = self.solute.unwrap_or(base);
        let fraction = field.concentration;
        let temperature = field.temperature - self.reference_temperature;
        let expansion = 1.0 + self.thermal_expansion * temperature;
        if !positive(expansion) {
            return Err(Error::InvalidPropertyResponse);
        }
        // Mass-fraction blending uses additive specific volumes rather than additive densities.
        let density =
            1.0 / ((1.0 - fraction) / base.rest_density + fraction / end.rest_density) / expansion;
        let viscosity = ((1.0 - fraction) * base.viscosity + fraction * end.viscosity)
            * (-self.viscosity_temperature_rate * temperature).exp();
        let sound_speed = (1.0 - fraction) * base.sound_speed + fraction * end.sound_speed;
        if !positive(density) || !positive(sound_speed) || !viscosity.is_finite() || viscosity < 0.0
        {
            return Err(Error::NumericalFailure);
        }
        Ok(Material {
            rest_density: density,
            viscosity,
            sound_speed,
        })
    }
}
impl Liquid {
    /// Enables per-material mechanical responses to transported fields.
    /// # Errors
    /// Rejects missing fields, wrong material count, invalid coefficients or a response
    /// outside its numerical domain, without changing the configured response.
    pub fn configure_property_response(
        &mut self,
        responses: Vec<Option<PropertyResponse>>,
    ) -> Result<(), Error> {
        if responses.len() != self.materials.len()
            || responses
                .iter()
                .zip(&self.arrhenius_viscosity)
                .any(|(p, m)| m.is_some() && p.is_some_and(|p| p.viscosity_temperature_rate != 0.0))
        {
            return Err(Error::InvalidPropertyResponse);
        }
        if self
            .transport
            .as_ref()
            .and_then(|t| t.species.as_ref())
            .is_some_and(|s| s.properties.is_some())
            && responses.iter().flatten().any(|r| r.solute.is_some())
        {
            return Err(Error::InvalidPropertyResponse);
        }
        for response in responses.iter().flatten() {
            response.validate()?;
        }
        let Some(fields) = &self.transport else {
            return Err(Error::InvalidPropertyResponse);
        };
        for (index, (particle, field)) in self.particles.iter().zip(&fields.fields).enumerate() {
            if let Some(response) = responses[particle.material] {
                let phase = fields.phase_material(
                    index,
                    particle.material,
                    self.materials[particle.material],
                )?;
                response.evaluate(phase, *field)?;
            }
        }
        let mut candidate = self.clone();
        candidate.property_responses = responses;
        candidate.effective_materials()?;
        self.property_responses = candidate.property_responses;
        Ok(())
    }
    /// Returns current effective coefficients for each particle, including field responses.
    /// # Errors
    /// Rejects numerical overflow or temperatures outside the configured constitutive domain.
    pub fn effective_materials(&self) -> Result<Vec<Material>, Error> {
        self.evaluate_materials(&self.particles, self.transport.as_ref())
    }
    pub(super) fn evaluate_materials(
        &self,
        particles: &[Particle],
        transport: Option<&Transport>,
    ) -> Result<Vec<Material>, Error> {
        let mut properties: Vec<Material> = particles
            .iter()
            .enumerate()
            .map(|(i, particle)| {
                let base = if let Some(species) = transport.and_then(|t| t.species.as_ref()) {
                    if let Some(properties) = &species.properties {
                        properties.evaluate(&species.fractions[i])?
                    } else {
                        self.materials[particle.material]
                    }
                } else {
                    self.materials[particle.material]
                };
                let base = if let Some(fields) = transport {
                    fields.phase_material(i, particle.material, base)?
                } else {
                    base
                };
                if let Some(response) = self.property_responses[particle.material] {
                    let fields = transport.ok_or(Error::InvalidPropertyResponse)?;
                    response.evaluate(base, fields.fields[i])
                } else {
                    Ok(base)
                }
            })
            .collect::<Result<_, Error>>()?;
        self.apply_arrhenius_viscosity(particles, &mut properties, transport)?;
        self.apply_structure_viscosity(particles, &mut properties)?;
        self.apply_shear_thinning(particles, &mut properties)?;
        self.apply_gas_properties(particles, &mut properties, transport)?;
        Ok(properties)
    }
}
