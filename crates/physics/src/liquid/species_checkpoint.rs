//! Complete current species owner: schema, fractions, chemistry, mixture mechanics
//! and heat capacities. Restoring stored temperatures must not remap their energy.
use super::super::{
    Material, Reaction, ReactionAccuracy, ReactionKinetics, SpeciesProperties, ViscosityBlend,
};
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct SpeciesCheckpointLimits {
    pub max_particles: usize,
    pub max_materials: usize,
    pub max_species: usize,
    pub max_name_bytes: usize,
    pub max_reaction_attempts: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactionCheckpoint {
    pub indices: [usize; 3],
    pub coefficients: [u64; 2],
    pub chemical_energies: Vec<u64>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactionAccuracyCheckpoint {
    pub tolerances: [u64; 3],
    pub max_attempts: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpeciesPropertiesCheckpoint {
    pub components: Vec<[u64; 3]>,
    pub viscosity_blend: u8,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpeciesCheckpoint {
    pub version: u32,
    pub names: Vec<String>,
    pub fractions: Vec<Vec<u64>>,
    pub particle_materials: Vec<usize>,
    pub reaction: Option<ReactionCheckpoint>,
    pub kinetics: Option<[u64; 2]>,
    pub accuracy: Option<ReactionAccuracyCheckpoint>,
    pub properties: Option<SpeciesPropertiesCheckpoint>,
    pub heat_capacities: Option<Vec<u64>>,
}
impl Liquid {
    #[must_use]
    pub fn species_checkpoint(&self) -> Option<SpeciesCheckpoint> {
        let s = self.transport.as_ref()?.species.as_ref()?;
        Some(SpeciesCheckpoint {
            version: 1,
            names: s.names.clone(),
            fractions: s
                .fractions
                .iter()
                .map(|row| row.iter().map(|v| v.to_bits()).collect())
                .collect(),
            particle_materials: self.particles.iter().map(|p| p.material).collect(),
            reaction: s.reaction.as_ref().map(|r| ReactionCheckpoint {
                indices: [r.fuel, r.oxidizer, r.product],
                coefficients: [r.oxidizer_ratio, r.rate].map(f64::to_bits),
                chemical_energies: r.chemical_energies.iter().map(|v| v.to_bits()).collect(),
            }),
            kinetics: s
                .kinetics
                .map(|k| [k.activation_temperature, k.ignition_temperature].map(f64::to_bits)),
            accuracy: s.reaction_accuracy.map(|a| ReactionAccuracyCheckpoint {
                tolerances: [
                    a.relative_tolerance,
                    a.fraction_tolerance,
                    a.temperature_tolerance,
                ]
                .map(f64::to_bits),
                max_attempts: a.max_attempts,
            }),
            properties: s.properties.as_ref().map(|p| SpeciesPropertiesCheckpoint {
                components: p
                    .components
                    .iter()
                    .map(|m| [m.rest_density, m.sound_speed, m.viscosity].map(f64::to_bits))
                    .collect(),
                viscosity_blend: match p.viscosity {
                    ViscosityBlend::Linear => 0,
                    ViscosityBlend::Logarithmic => 1,
                },
            }),
            heat_capacities: s
                .heat_capacities
                .as_ref()
                .map(|v| v.iter().map(|c| c.to_bits()).collect()),
        })
    }
    /// Applies the complete species block to matching staged particles and fields.
    /// No heat-capacity configuration remap is run: stored fields already include it.
    /// # Errors
    /// Rejects limits, layout/schema/fractions, invalid chemistry/mixtures and overflow.
    pub fn restore_species_checkpoint(
        &mut self,
        saved: &SpeciesCheckpoint,
        limits: SpeciesCheckpointLimits,
    ) -> Result<(), Error> {
        self.restore_species_checkpoint_inner(saved, limits, true)
    }
    pub(in crate::liquid) fn stage_species_checkpoint(
        &mut self,
        saved: &SpeciesCheckpoint,
        limits: SpeciesCheckpointLimits,
    ) -> Result<(), Error> {
        self.restore_species_checkpoint_inner(saved, limits, false)
    }
    fn restore_species_checkpoint_inner(
        &mut self,
        saved: &SpeciesCheckpoint,
        limits: SpeciesCheckpointLimits,
        evaluate: bool,
    ) -> Result<(), Error> {
        let count = saved.names.len();
        let name_bytes = saved
            .names
            .iter()
            .try_fold(0_usize, |sum, n| sum.checked_add(n.len()))
            .ok_or(Error::InvalidTransport)?;
        if saved.version != 1
            || count > limits.max_species
            || count > 64
            || name_bytes > limits.max_name_bytes
            || self.materials.len() > limits.max_materials
            || saved.fractions.len() > limits.max_particles
            || saved.fractions.len() != self.particles.len()
            || saved.fractions.iter().any(|row| row.len() != count)
            || saved
                .heat_capacities
                .as_ref()
                .is_some_and(|v| v.len() != count)
            || saved
                .reaction
                .as_ref()
                .is_some_and(|r| r.chemical_energies.len() != count)
            || saved
                .properties
                .as_ref()
                .is_some_and(|p| p.components.len() != count)
            || saved.particle_materials.len() != self.particles.len()
            || saved
                .particle_materials
                .iter()
                .zip(&self.particles)
                .any(|(m, p)| *m != p.material)
            || saved
                .accuracy
                .as_ref()
                .is_some_and(|a| a.max_attempts > limits.max_reaction_attempts)
            || (saved.reaction.is_none() && (saved.kinetics.is_some() || saved.accuracy.is_some()))
            || (saved.kinetics.is_none() && saved.accuracy.is_some())
        {
            return Err(Error::InvalidTransport);
        }
        let capacities = saved
            .heat_capacities
            .as_ref()
            .map(|v| v.iter().copied().map(f64::from_bits).collect::<Vec<_>>());
        if capacities.as_ref().is_some_and(|v| {
            v.len() != count || v.iter().any(|c| !positive(*c)) || self.gas_active()
        }) {
            return Err(Error::InvalidTransport);
        }
        let mut candidate = self.clone();
        candidate
            .transport
            .as_mut()
            .ok_or(Error::InvalidTransport)?
            .species = None;
        candidate.configure_species(
            saved.names.clone(),
            saved
                .fractions
                .iter()
                .map(|r| r.iter().copied().map(f64::from_bits).collect())
                .collect(),
        )?;
        candidate
            .transport
            .as_mut()
            .and_then(|t| t.species.as_mut())
            .ok_or(Error::InvalidTransport)?
            .heat_capacities = capacities;
        let reaction = saved.reaction.as_ref().map(|r| {
            let [fuel, oxidizer, product] = r.indices;
            let [oxidizer_ratio, rate] = r.coefficients.map(f64::from_bits);
            Reaction {
                fuel,
                oxidizer,
                product,
                oxidizer_ratio,
                rate,
                chemical_energies: r
                    .chemical_energies
                    .iter()
                    .copied()
                    .map(f64::from_bits)
                    .collect(),
            }
        });
        candidate.configure_reaction(reaction)?;
        if saved.reaction.is_some() {
            candidate.configure_reaction_kinetics(saved.kinetics.map(|bits| ReactionKinetics {
                activation_temperature: f64::from_bits(bits[0]),
                ignition_temperature: f64::from_bits(bits[1]),
            }))?;
            if saved.kinetics.is_some() {
                candidate.configure_reaction_accuracy(saved.accuracy.as_ref().map(|a| {
                    let [
                        relative_tolerance,
                        fraction_tolerance,
                        temperature_tolerance,
                    ] = a.tolerances.map(f64::from_bits);
                    ReactionAccuracy {
                        relative_tolerance,
                        fraction_tolerance,
                        temperature_tolerance,
                        max_attempts: a.max_attempts,
                    }
                }))?;
            }
        }
        let properties = saved
            .properties
            .as_ref()
            .map(|p| -> Result<SpeciesProperties, Error> {
                let viscosity = match p.viscosity_blend {
                    0 => ViscosityBlend::Linear,
                    1 => ViscosityBlend::Logarithmic,
                    _ => return Err(Error::InvalidPropertyResponse),
                };
                if p.components.len() != count {
                    return Err(Error::InvalidPropertyResponse);
                }
                Ok(SpeciesProperties {
                    viscosity,
                    components: p
                        .components
                        .iter()
                        .map(|m| {
                            let [rest_density, sound_speed, viscosity] = m.map(f64::from_bits);
                            Material {
                                rest_density,
                                sound_speed,
                                viscosity,
                            }
                        })
                        .collect(),
                })
            })
            .transpose()?;
        if evaluate {
            candidate.configure_species_properties(properties)?;
            candidate.transport_totals()?;
            candidate.species_totals()?;
            candidate.chemical_energy()?;
        } else {
            if let Some(p) = &properties {
                p.validate(count)?;
                if candidate
                    .property_responses
                    .iter()
                    .flatten()
                    .any(|r| r.solute.is_some())
                {
                    return Err(Error::InvalidPropertyResponse);
                }
            }
            candidate
                .transport
                .as_mut()
                .and_then(|t| t.species.as_mut())
                .ok_or(Error::InvalidTransport)?
                .properties = properties;
        }
        *self = candidate;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquid::{Config, LiquidCheckpointLimits, LiquidField, Particle, TransportMaterial};
    #[test]
    fn composition_chemistry_and_mixture_heat_restore_without_remapping_temperature() {
        for thermal_kinetics in [false, true] {
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
                        velocity: [0.; 3],
                        mass: 1.,
                        material: 0,
                    },
                    Particle {
                        position: [0.04, 0., 0.],
                        velocity: [0.; 3],
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
                            temperature: 500.,
                            concentration: 0.2,
                        },
                        LiquidField {
                            temperature: 510.,
                            concentration: 0.8,
                        },
                    ],
                    vec![TransportMaterial {
                        specific_heat: 4184.,
                        conductivity: 10.,
                        diffusivity: 0.001,
                        mixing_group: 1,
                    }],
                )
                .unwrap();
            original
                .configure_species(
                    vec!["fuel".into(), "oxidizer".into(), "product".into()],
                    vec![vec![0.3, 0.2, 0.5], vec![0.1, 0.4, 0.5]],
                )
                .unwrap();
            if thermal_kinetics {
                original
                    .configure_species_heat_capacities(Some(vec![3000., 3500., 4000.]))
                    .unwrap();
                original
                    .configure_species_properties(Some(SpeciesProperties {
                        components: vec![Material::WATER; 3],
                        viscosity: ViscosityBlend::Logarithmic,
                    }))
                    .unwrap();
            }
            original
                .configure_reaction(Some(Reaction {
                    fuel: 0,
                    oxidizer: 1,
                    product: 2,
                    oxidizer_ratio: 1.,
                    rate: 0.2,
                    chemical_energies: vec![1000., 500., 0.],
                }))
                .unwrap();
            if thermal_kinetics {
                original
                    .configure_reaction_kinetics(Some(ReactionKinetics {
                        activation_temperature: 100.,
                        ignition_temperature: 300.,
                    }))
                    .unwrap();
                original
                    .configure_reaction_accuracy(Some(ReactionAccuracy {
                        max_attempts: 256,
                        ..ReactionAccuracy::default()
                    }))
                    .unwrap();
            }
            for _ in 0..7 {
                original.step(0.0001, None).unwrap();
            }
            assert_ne!(
                original.species_fractions().unwrap()[0][0].to_bits(),
                0.3_f64.to_bits()
            );
            let saved = original.species_checkpoint().unwrap();
            let thermal = original.thermal_checkpoint().unwrap();
            let base = LiquidCheckpointLimits {
                max_particles: 8,
                max_materials: 1,
                max_pairs: 16,
                max_neighbor_checks: 64,
                max_substeps: 256,
            };
            let limits = SpeciesCheckpointLimits {
                max_particles: 8,
                max_materials: 1,
                max_species: 3,
                max_name_bytes: 64,
                max_reaction_attempts: 4096,
            };
            let mut restored = original.particle_checkpoint().prepare_base(base).unwrap();
            restored.restore_thermal_checkpoint(&thermal, 8, 1).unwrap();
            restored.restore_species_checkpoint(&saved, limits).unwrap();
            assert_eq!(restored.thermal_checkpoint(), Some(thermal));
            assert_eq!(restored.species_checkpoint(), Some(saved.clone()));
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
                assert_eq!(original.species_checkpoint(), restored.species_checkpoint());
                assert_eq!(
                    original
                        .species_totals()
                        .unwrap()
                        .unwrap()
                        .iter()
                        .map(|v| v.to_bits())
                        .collect::<Vec<_>>(),
                    restored
                        .species_totals()
                        .unwrap()
                        .unwrap()
                        .iter()
                        .map(|v| v.to_bits())
                        .collect::<Vec<_>>()
                );
                assert_eq!(
                    original.chemical_energy().unwrap().unwrap().to_bits(),
                    restored.chemical_energy().unwrap().unwrap().to_bits()
                );
            }
            let before = restored.clone();
            let mut bad = saved.clone();
            bad.fractions[0][0] = 2_f64.to_bits();
            assert!(restored.restore_species_checkpoint(&bad, limits).is_err());
            assert_eq!(restored, before);
            let mut bad = saved.clone();
            bad.reaction.as_mut().unwrap().indices[2] = 0;
            assert!(restored.restore_species_checkpoint(&bad, limits).is_err());
            assert_eq!(restored, before);
            assert!(
                restored
                    .restore_species_checkpoint(
                        &saved,
                        SpeciesCheckpointLimits {
                            max_species: 2,
                            ..limits
                        }
                    )
                    .is_err()
            );
        }
    }
}
