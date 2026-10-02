//! Configurable heterogeneous liquid mixture and rheology.
use super::{
    Error, Liquid, Material, ShearThinning, SpeciesProperties, Thixotropy, ViscosityBlend,
};

/// Components are low-, high- and medium-viscosity liquids, in that order.
/// Scalar structural memory is independent of elastic stress.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluidMixtureProfile {
    pub components: [Material; 3],
    pub viscosity_blend: ViscosityBlend,
    pub shear_thinning: ShearThinning,
    pub thixotropy: Thixotropy,
}
impl FluidMixtureProfile {
    /// Illustrative SI coefficients for solver demonstrations.
    pub const DEMO: Self = Self {
        components: [
            Material {
                rest_density: 1000.0,
                sound_speed: 20.0,
                viscosity: 0.001,
            },
            Material {
                rest_density: 1050.0,
                sound_speed: 20.0,
                viscosity: 2.0,
            },
            Material {
                rest_density: 1050.0,
                sound_speed: 20.0,
                viscosity: 0.3,
            },
        ],
        viscosity_blend: ViscosityBlend::Logarithmic,
        shear_thinning: ShearThinning {
            reference_rate: 1.0,
            flow_index: 0.5,
            minimum_rate: 0.01,
            minimum_viscosity: 0.0001,
            maximum_viscosity: 100.0,
        },
        thixotropy: Thixotropy {
            recovery_rate: 0.1,
            breakdown: 0.5,
            broken_viscosity_ratio: 0.2,
            broken_yield_ratio: 0.2,
        },
    };
}
impl Liquid {
    /// Sets the complete three-component schema, mixture properties and common
    /// rheology for all material slots. Supply local mass fractions and structural
    /// fractions per particle. Existing transport controls temperature and diffusion.
    /// # Errors
    /// Missing transport, existing coupled species schema, invalid coefficients or
    /// particle rows, incompatible properties, or numerical failure. Atomic.
    pub fn configure_fluid_mixture(
        &mut self,
        profile: FluidMixtureProfile,
        fractions: Vec<[f64; 3]>,
        structure: Vec<f64>,
    ) -> Result<(), Error> {
        let mut candidate = self.clone();
        candidate.configure_species(
            vec![
                "low_viscosity".into(),
                "high_viscosity".into(),
                "medium_viscosity".into(),
            ],
            fractions.into_iter().map(Vec::from).collect(),
        )?;
        candidate.configure_species_properties(Some(SpeciesProperties {
            components: profile.components.to_vec(),
            viscosity: profile.viscosity_blend,
        }))?;
        candidate.configure_shear_thinning(vec![
            Some(profile.shear_thinning);
            candidate.materials.len()
        ])?;
        candidate.configure_thixotropy(
            vec![Some(profile.thixotropy); candidate.materials.len()],
            structure,
        )?;
        *self = candidate;
        Ok(())
    }
}
