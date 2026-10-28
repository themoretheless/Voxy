//! Temperature/concentration transport block. Phase, species and mechanical
//! response laws remain separate blocks required by a complete liquid snapshot.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalMaterialCheckpoint {
    pub values: [u64; 3],
    pub mixing_group: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalFieldCheckpoint {
    pub version: u32,
    pub fields: Vec<[u64; 2]>,
    pub materials: Vec<ThermalMaterialCheckpoint>,
    pub particle_materials: Vec<usize>,
}
impl Liquid {
    #[must_use]
    pub fn thermal_checkpoint(&self) -> Option<ThermalFieldCheckpoint> {
        self.transport
            .as_ref()
            .map(|transport| ThermalFieldCheckpoint {
                version: 1,
                fields: transport
                    .fields
                    .iter()
                    .map(|f| [f.temperature, f.concentration].map(f64::to_bits))
                    .collect(),
                materials: transport
                    .materials
                    .iter()
                    .map(|m| ThermalMaterialCheckpoint {
                        values: [m.specific_heat, m.conductivity, m.diffusivity].map(f64::to_bits),
                        mixing_group: m.mixing_group,
                    })
                    .collect(),
                particle_materials: self.particles.iter().map(|p| p.material).collect(),
            })
    }
    /// Rebuilds transported fields through the existing atomic configuration API.
    /// Restore phase/species and response participants in the outer staged transaction.
    /// Existing phase state is rejected by configure_transport rather than erased.
    /// # Errors
    /// Rejects layout, version, capacity, invalid fields/coefficient data and overflow.
    pub fn restore_thermal_checkpoint(
        &mut self,
        saved: &ThermalFieldCheckpoint,
        max_particles: usize,
        max_materials: usize,
    ) -> Result<(), Error> {
        self.restore_thermal_checkpoint_inner(saved, max_particles, max_materials, true)
    }
    pub(in crate::liquid) fn stage_thermal_checkpoint(
        &mut self,
        saved: &ThermalFieldCheckpoint,
        max_particles: usize,
        max_materials: usize,
    ) -> Result<(), Error> {
        self.restore_thermal_checkpoint_inner(saved, max_particles, max_materials, false)
    }
    fn restore_thermal_checkpoint_inner(
        &mut self,
        saved: &ThermalFieldCheckpoint,
        max_particles: usize,
        max_materials: usize,
        evaluate: bool,
    ) -> Result<(), Error> {
        if saved.version != 1
            || saved.fields.len() != self.particles.len()
            || saved.fields.len() > max_particles
            || saved.materials.len() != self.materials.len()
            || saved.materials.len() > max_materials
            || saved.particle_materials.len() != self.particles.len()
            || saved
                .particle_materials
                .iter()
                .zip(&self.particles)
                .any(|(m, p)| *m != p.material)
        {
            return Err(Error::InvalidTransport);
        }
        let fields = saved
            .fields
            .iter()
            .map(|f| LiquidField {
                temperature: f64::from_bits(f[0]),
                concentration: f64::from_bits(f[1]),
            })
            .collect();
        let materials = saved
            .materials
            .iter()
            .map(|m| TransportMaterial {
                specific_heat: f64::from_bits(m.values[0]),
                conductivity: f64::from_bits(m.values[1]),
                diffusivity: f64::from_bits(m.values[2]),
                mixing_group: m.mixing_group,
            })
            .collect();
        if evaluate {
            self.configure_transport(fields, materials)
        } else {
            super::validate_checkpoint_arrays(
                &fields,
                &materials,
                self.particles.len(),
                self.materials.len(),
            )?;
            self.transport = Some(Transport {
                fields,
                materials,
                phase: None,
                species: None,
            });
            Ok(())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquid::{Config, LiquidCheckpointLimits, Material, Particle};
    #[test]
    fn transported_heat_and_solute_continue_from_exact_restored_fields() {
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
        assert!(original.thermal_checkpoint().is_none());
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
                    mixing_group: 3,
                }],
            )
            .unwrap();
        original
            .set_maxwell_fluid(
                0,
                Some(crate::liquid::MaxwellFluid {
                    modulus: 2.,
                    relaxation_time: 0.2,
                }),
            )
            .unwrap();
        original
            .set_conformations(vec![[[1.1, 0., 0.], [0., 0.95, 0.], [0., 0., 1.]]; 2])
            .unwrap();
        for _ in 0..7 {
            original.step(0.0001, None).unwrap();
        }
        assert_ne!(
            original.fields().unwrap()[0].temperature.to_bits(),
            300_f64.to_bits()
        );
        assert_ne!(
            original.fields().unwrap()[0].concentration.to_bits(),
            0.2_f64.to_bits()
        );
        let saved = original.thermal_checkpoint().unwrap();
        let polymer = original.polymer_checkpoint();
        let limits = LiquidCheckpointLimits {
            max_particles: 8,
            max_materials: 1,
            max_pairs: 16,
            max_neighbor_checks: 64,
            max_substeps: 256,
        };
        let mut restored = original.particle_checkpoint().prepare_base(limits).unwrap();
        restored.restore_thermal_checkpoint(&saved, 8, 1).unwrap();
        restored.restore_polymer_checkpoint(&polymer, 8, 1).unwrap();
        assert_eq!(restored.thermal_checkpoint(), Some(saved.clone()));
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
            assert_eq!(original.polymer_checkpoint(), restored.polymer_checkpoint());
            let a = original.transport_totals().unwrap().unwrap();
            let b = restored.transport_totals().unwrap().unwrap();
            assert_eq!([a.0, a.1].map(f64::to_bits), [b.0, b.1].map(f64::to_bits));
        }
        let before = restored.clone();
        let mut bad = saved.clone();
        bad.fields[0][0] = f64::NAN.to_bits();
        assert!(restored.restore_thermal_checkpoint(&bad, 8, 1).is_err());
        assert_eq!(restored, before);
        let mut bad = saved.clone();
        bad.fields[0][1] = 1.1_f64.to_bits();
        assert!(restored.restore_thermal_checkpoint(&bad, 8, 1).is_err());
        assert_eq!(restored, before);
        let mut bad = saved.clone();
        bad.materials[0].values[0] = 0;
        assert!(restored.restore_thermal_checkpoint(&bad, 8, 1).is_err());
        assert_eq!(restored, before);
        assert!(restored.restore_thermal_checkpoint(&saved, 1, 1).is_err());
    }
}
