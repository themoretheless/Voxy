//! Homogeneous mixture coefficients from complete transported mass fractions.
use super::{Error, Liquid, Material, positive};

/// Application-selected viscosity closure, without calibration implied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViscosityBlend {
    /// `mu = sum(Y_i * mu_i)`, permits inviscid components.
    Linear,
    /// `ln(mu) = sum(Y_i * ln(mu_i))`, requires positive end-member viscosities.
    Logarithmic,
}
/// Additive specific volume and volume-weighted bulk compliance (Wood closure).
/// End members are ordered like the species schema. No excess volume/heat of mixing.
#[derive(Clone, Debug, PartialEq)]
pub struct SpeciesProperties {
    pub components: Vec<Material>,
    pub viscosity: ViscosityBlend,
}
impl SpeciesProperties {
    fn validate(&self, count: usize) -> Result<(), Error> {
        if self.components.len() != count
            || self.components.iter().any(|m| {
                !positive(m.rest_density)
                    || !positive(m.sound_speed)
                    || !m.viscosity.is_finite()
                    || m.viscosity < 0.0
                    || (self.viscosity == ViscosityBlend::Logarithmic && m.viscosity == 0.0)
            })
        {
            return Err(Error::InvalidPropertyResponse);
        }
        Ok(())
    }
    /// Computes mixture coefficients before phase, thermal and rheological responses.
    /// # Errors
    /// Invalid model/composition or unrepresentable coefficients.
    pub fn evaluate(&self, fractions: &[f64]) -> Result<Material, Error> {
        self.validate(fractions.len())?;
        super::species::validate_row(fractions, self.components.len())?;
        let mut volume = 0.0;
        let mut compliance = 0.0;
        let mut viscosity = 0.0;
        for (y, m) in fractions.iter().zip(&self.components) {
            volume += y / m.rest_density;
            let inverse = (1.0 / m.rest_density) / m.sound_speed;
            compliance += y * inverse * inverse;
            viscosity += y * match self.viscosity {
                ViscosityBlend::Linear => m.viscosity,
                ViscosityBlend::Logarithmic => m.viscosity.ln(),
            };
        }
        let result = Material {
            rest_density: 1.0 / volume,
            sound_speed: volume / compliance.sqrt(),
            viscosity: match self.viscosity {
                ViscosityBlend::Linear => viscosity,
                ViscosityBlend::Logarithmic => viscosity.exp(),
            },
        };
        if !positive(result.rest_density)
            || !positive(result.sound_speed)
            || !result.viscosity.is_finite()
            || result.viscosity < 0.0
        {
            return Err(Error::NumericalFailure);
        }
        Ok(result)
    }
}
impl Liquid {
    /// Enables complete-species mechanical blending. Clearing restores material bases.
    /// Composition is applied before phase, thermal expansion and rheology. Gas EOS
    /// still overrides density/sound speed with its configured constant R and gamma.
    /// Heat capacity and chemical reference energies are not changed by this operation.
    /// # Errors
    /// Missing species, invalid end members/count, conflicting scalar solute response,
    /// or invalid resulting mechanical coefficients. Atomic.
    pub fn configure_species_properties(
        &mut self,
        properties: Option<SpeciesProperties>,
    ) -> Result<(), Error> {
        let species = self
            .transport
            .as_ref()
            .and_then(|t| t.species.as_ref())
            .ok_or(Error::InvalidTransport)?;
        if let Some(p) = &properties {
            p.validate(species.names.len())?;
            if self
                .property_responses
                .iter()
                .flatten()
                .any(|r| r.solute.is_some())
            {
                return Err(Error::InvalidPropertyResponse);
            }
        }
        let mut candidate = self.clone();
        candidate
            .transport
            .as_mut()
            .and_then(|t| t.species.as_mut())
            .ok_or(Error::InvalidTransport)?
            .properties = properties;
        candidate.effective_materials()?;
        *self = candidate;
        Ok(())
    }
}
