//! Coupled temperature/solute mechanical response configuration.
use super::super::ArrheniusViscosity;
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyResponseCheckpoint {
    pub coefficients: [u64; 3],
    pub solute: Option<[u64; 3]>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldResponseCheckpoint {
    pub version: u32,
    pub materials: Vec<[u64; 3]>,
    pub properties: Vec<Option<PropertyResponseCheckpoint>>,
    pub arrhenius: Vec<Option<[u64; 2]>>,
}
impl Liquid {
    #[must_use]
    pub fn field_response_checkpoint(&self) -> FieldResponseCheckpoint {
        FieldResponseCheckpoint {
            version: 1,
            materials: self
                .materials
                .iter()
                .map(|m| [m.rest_density, m.sound_speed, m.viscosity].map(f64::to_bits))
                .collect(),
            properties: self
                .property_responses
                .iter()
                .map(|p| {
                    p.map(|p| PropertyResponseCheckpoint {
                        coefficients: [
                            p.reference_temperature,
                            p.thermal_expansion,
                            p.viscosity_temperature_rate,
                        ]
                        .map(f64::to_bits),
                        solute: p.solute.map(|m| {
                            [m.rest_density, m.sound_speed, m.viscosity].map(f64::to_bits)
                        }),
                    })
                })
                .collect(),
            arrhenius: self
                .arrhenius_viscosity
                .iter()
                .map(|m| {
                    m.map(|m| [m.reference_temperature, m.activation_energy].map(f64::to_bits))
                })
                .collect(),
        }
    }
    /// Atomically replaces both linked response arrays after particle/thermal stages.
    /// # Errors
    /// Rejects material identity/layout, limits, incompatible laws or evaluation overflow.
    pub fn restore_field_response_checkpoint(
        &mut self,
        saved: &FieldResponseCheckpoint,
        max_particles: usize,
        max_materials: usize,
    ) -> Result<(), Error> {
        self.restore_field_response_checkpoint_inner(saved, max_particles, max_materials, true)
    }
    pub(in crate::liquid) fn stage_field_response_checkpoint(
        &mut self,
        saved: &FieldResponseCheckpoint,
        max_particles: usize,
        max_materials: usize,
    ) -> Result<(), Error> {
        self.restore_field_response_checkpoint_inner(saved, max_particles, max_materials, false)
    }
    fn restore_field_response_checkpoint_inner(
        &mut self,
        saved: &FieldResponseCheckpoint,
        max_particles: usize,
        max_materials: usize,
        evaluate: bool,
    ) -> Result<(), Error> {
        if saved.version != 1
            || self.particles.len() > max_particles
            || saved.materials.len() > max_materials
            || saved.materials.len() != self.materials.len()
            || saved.properties.len() != self.materials.len()
            || saved.arrhenius.len() != self.materials.len()
            || saved
                .materials
                .iter()
                .zip(&self.materials)
                .any(|(bits, m)| {
                    *bits != [m.rest_density, m.sound_speed, m.viscosity].map(f64::to_bits)
                })
        {
            return Err(Error::InvalidPropertyResponse);
        }
        let properties: Vec<_> = saved
            .properties
            .iter()
            .map(|p| {
                p.as_ref().map(|p| {
                    let [
                        reference_temperature,
                        thermal_expansion,
                        viscosity_temperature_rate,
                    ] = p.coefficients.map(f64::from_bits);
                    PropertyResponse {
                        reference_temperature,
                        thermal_expansion,
                        viscosity_temperature_rate,
                        solute: p.solute.map(|bits| {
                            let [rest_density, sound_speed, viscosity] = bits.map(f64::from_bits);
                            Material {
                                rest_density,
                                sound_speed,
                                viscosity,
                            }
                        }),
                    }
                })
            })
            .collect();
        let arrhenius: Vec<_> = saved
            .arrhenius
            .iter()
            .map(|m| {
                m.map(|bits| ArrheniusViscosity {
                    reference_temperature: f64::from_bits(bits[0]),
                    activation_energy: f64::from_bits(bits[1]),
                })
            })
            .collect();
        for p in properties.iter().flatten() {
            p.validate()?;
        }
        for m in arrhenius.iter().flatten() {
            m.validate()?;
        }
        if self.transport.is_none()
            && (properties.iter().any(Option::is_some) || arrhenius.iter().any(Option::is_some))
            || properties
                .iter()
                .zip(&arrhenius)
                .any(|(p, a)| a.is_some() && p.is_some_and(|p| p.viscosity_temperature_rate != 0.))
            || self
                .transport
                .as_ref()
                .and_then(|t| t.species.as_ref())
                .is_some_and(|s| s.properties.is_some())
                && properties.iter().flatten().any(|p| p.solute.is_some())
        {
            return Err(Error::InvalidPropertyResponse);
        }
        let mut candidate = self.clone();
        candidate.property_responses = properties;
        candidate.arrhenius_viscosity = arrhenius;
        if evaluate {
            candidate.effective_materials()?;
        }
        *self = candidate;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquid::{Config, LiquidCheckpointLimits, LiquidField, TransportMaterial};
    #[test]
    fn coupled_field_responses_restore_without_invalid_intermediate_configuration() {
        for activated in [false, true] {
            let config = Config {
                gravity: [0.; 3],
                max_particles: 8,
                max_pairs: 16,
                max_neighbor_checks: 64,
                max_substeps: 256,
                ..Config::default()
            };
            let mut original = Liquid::new(
                vec![
                    Particle {
                        position: [-0.04, 0., 0.],
                        velocity: [0.1, 0., 0.],
                        mass: 1.,
                        material: 0,
                    },
                    Particle {
                        position: [0.04, 0., 0.],
                        velocity: [-0.1, 0., 0.],
                        mass: 1.,
                        material: 0,
                    },
                ],
                vec![Material::WATER],
                config,
            )
            .unwrap();
            original
                .configure_transport(
                    vec![
                        LiquidField {
                            temperature: 300.,
                            concentration: 0.2,
                        },
                        LiquidField {
                            temperature: 330.,
                            concentration: 0.8,
                        },
                    ],
                    vec![TransportMaterial {
                        specific_heat: 4184.,
                        conductivity: 100.,
                        diffusivity: 0.001,
                        mixing_group: 1,
                    }],
                )
                .unwrap();
            let law = PropertyResponse {
                reference_temperature: 300.,
                thermal_expansion: 0.0001,
                viscosity_temperature_rate: if activated { 0. } else { 0.02 },
                solute: Some(Material {
                    rest_density: 900.,
                    sound_speed: 18.,
                    viscosity: 0.005,
                }),
            };
            original
                .configure_property_response(vec![Some(law)])
                .unwrap();
            if activated {
                original
                    .configure_arrhenius_viscosity(vec![Some(ArrheniusViscosity {
                        reference_temperature: 300.,
                        activation_energy: 20000.,
                    })])
                    .unwrap();
            }
            for _ in 0..7 {
                original.step(0.0001, None).unwrap();
            }
            let saved = original.field_response_checkpoint();
            let limits = LiquidCheckpointLimits {
                max_particles: 8,
                max_materials: 1,
                max_pairs: 16,
                max_neighbor_checks: 64,
                max_substeps: 256,
            };
            let mut restored = original.particle_checkpoint().prepare_base(limits).unwrap();
            restored
                .restore_thermal_checkpoint(&original.thermal_checkpoint().unwrap(), 8, 1)
                .unwrap();
            // Seed the opposite viscosity family: sequential public setters would
            // reject an intermediate conflict, while the final saved pair is valid.
            if activated {
                restored
                    .configure_property_response(vec![Some(PropertyResponse {
                        viscosity_temperature_rate: 0.01,
                        ..law
                    })])
                    .unwrap();
            } else {
                restored
                    .configure_arrhenius_viscosity(vec![Some(ArrheniusViscosity {
                        reference_temperature: 300.,
                        activation_energy: 1000.,
                    })])
                    .unwrap();
            }
            restored
                .restore_field_response_checkpoint(&saved, 8, 1)
                .unwrap();
            for _ in 0..120 {
                assert_eq!(
                    original.step(0.0001, None).unwrap(),
                    restored.step(0.0001, None).unwrap()
                );
                assert_eq!(
                    original.particle_checkpoint(),
                    restored.particle_checkpoint()
                );
                assert_eq!(original.thermal_checkpoint(), restored.thermal_checkpoint());
                assert_eq!(
                    original.field_response_checkpoint(),
                    restored.field_response_checkpoint()
                );
                let words = |fluid: &Liquid| {
                    fluid
                        .effective_materials()
                        .unwrap()
                        .iter()
                        .map(|m| [m.rest_density, m.sound_speed, m.viscosity].map(f64::to_bits))
                        .collect::<Vec<_>>()
                };
                assert_eq!(words(&original), words(&restored));
            }
            let before = restored.clone();
            let mut bad = saved.clone();
            bad.properties[0].as_mut().unwrap().coefficients[0] = f64::NAN.to_bits();
            assert!(
                restored
                    .restore_field_response_checkpoint(&bad, 8, 1)
                    .is_err()
            );
            assert_eq!(restored, before);
            let mut bad = saved.clone();
            bad.arrhenius[0] = Some([300_f64.to_bits(), 1000_f64.to_bits()]);
            bad.properties[0].as_mut().unwrap().coefficients[2] = 0.02_f64.to_bits();
            assert!(
                restored
                    .restore_field_response_checkpoint(&bad, 8, 1)
                    .is_err()
            );
            assert_eq!(restored, before);
            assert!(
                restored
                    .restore_field_response_checkpoint(&saved, 8, 0)
                    .is_err()
            );
        }
    }
}
