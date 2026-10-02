//! Absolute-temperature viscosity response, composed before shear thinning.
use super::{Error, Liquid, Material, Particle, positive, transport::Transport};
const GAS_CONSTANT: f64 = 8.314_462_618_153_24;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrheniusViscosity {
    /// Kelvin. The current phase/solute reference viscosity is defined at this temperature.
    pub reference_temperature: f64,
    /// Joules per mole. Nonnegative activation energy.
    pub activation_energy: f64,
}
impl ArrheniusViscosity {
    /// Illustrative midpoint of the reported 33.8–40.9 kJ/mol commercial milk range.
    /// This is not a measured fit for one brand.
    pub const CONDENSED_MILK_DEMO: Self = Self {
        reference_temperature: 298.15,
        activation_energy: 37_000.0,
    };
}
impl Liquid {
    /// Configures absolute-temperature viscosity responses per material.
    /// Combines with phase/solute coefficients and optional shear thinning. Cannot
    /// coexist with a nonzero exponential temperature rate in `PropertyResponse`.
    /// # Errors
    /// Requires thermal fields, valid model count, positive Kelvin temperatures,
    /// nonnegative finite activation energies and finite evaluated viscosities.
    /// Any failure preserves the prior configuration.
    pub fn configure_arrhenius_viscosity(
        &mut self,
        models: Vec<Option<ArrheniusViscosity>>,
    ) -> Result<(), Error> {
        if models.len() != self.materials.len()
            || self.transport.is_none()
            || models.iter().flatten().any(|m| {
                !positive(m.reference_temperature)
                    || !m.activation_energy.is_finite()
                    || m.activation_energy < 0.0
            })
            || models
                .iter()
                .zip(&self.property_responses)
                .any(|(m, p)| m.is_some() && p.is_some_and(|p| p.viscosity_temperature_rate != 0.0))
        {
            return Err(Error::InvalidPropertyResponse);
        }
        let mut candidate = self.clone();
        candidate.arrhenius_viscosity = models;
        candidate.effective_materials()?;
        self.arrhenius_viscosity = candidate.arrhenius_viscosity;
        Ok(())
    }
    pub(super) fn apply_arrhenius_viscosity(
        &self,
        particles: &[Particle],
        properties: &mut [Material],
        transport: Option<&Transport>,
    ) -> Result<(), Error> {
        if self.arrhenius_viscosity.iter().all(Option::is_none) {
            return Ok(());
        }
        let fields = transport.ok_or(Error::InvalidPropertyResponse)?;
        for (index, (particle, property)) in particles.iter().zip(properties).enumerate() {
            let Some(model) = self.arrhenius_viscosity[particle.material] else {
                continue;
            };
            let temperature = fields.fields[index].temperature;
            if !positive(temperature) {
                return Err(Error::InvalidPropertyResponse);
            }
            let exponent = model.activation_energy / GAS_CONSTANT
                * (1.0 / temperature - 1.0 / model.reference_temperature);
            let viscosity = property.viscosity * exponent.exp();
            if !viscosity.is_finite() || viscosity < 0.0 {
                return Err(Error::NumericalFailure);
            }
            property.viscosity = viscosity;
        }
        Ok(())
    }
}
