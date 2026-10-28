//! Shear/yield law and evolving per-particle structural memory.
use super::super::Thixotropy;
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RheologyCheckpoint {
    pub version: u32,
    pub shear: Vec<Option<[u64; 5]>>,
    pub yield_stresses: Vec<u64>,
    pub thixotropy: Vec<Option<[u64; 4]>>,
    pub structure: Vec<u64>,
    pub particle_materials: Vec<usize>,
}
impl Liquid {
    #[must_use]
    pub fn rheology_checkpoint(&self) -> RheologyCheckpoint {
        RheologyCheckpoint {
            version: 1,
            shear: self
                .shear_thinning
                .iter()
                .map(|m| {
                    m.map(|m| {
                        [
                            m.reference_rate,
                            m.flow_index,
                            m.minimum_rate,
                            m.minimum_viscosity,
                            m.maximum_viscosity,
                        ]
                        .map(f64::to_bits)
                    })
                })
                .collect(),
            yield_stresses: self.yield_stresses.iter().map(|v| v.to_bits()).collect(),
            thixotropy: self
                .thixotropy
                .iter()
                .map(|m| {
                    m.map(|m| {
                        [
                            m.recovery_rate,
                            m.breakdown,
                            m.broken_viscosity_ratio,
                            m.broken_yield_ratio,
                        ]
                        .map(f64::to_bits)
                    })
                })
                .collect(),
            structure: self.structure.iter().map(|v| v.to_bits()).collect(),
            particle_materials: self.particles.iter().map(|p| p.material).collect(),
        }
    }
    /// Restores this block into the matching particle layout as one transaction.
    /// Other material/transport/constitutive participants remain caller-owned stages.
    /// # Errors
    /// Rejects incompatible layout, coefficients, structure fractions, limits or evaluation overflow.
    pub fn restore_rheology_checkpoint(
        &mut self,
        saved: &RheologyCheckpoint,
        max_particles: usize,
        max_materials: usize,
    ) -> Result<(), Error> {
        self.restore_rheology_checkpoint_inner(saved, max_particles, max_materials, true)
    }
    pub(in crate::liquid) fn stage_rheology_checkpoint(
        &mut self,
        saved: &RheologyCheckpoint,
        max_particles: usize,
        max_materials: usize,
    ) -> Result<(), Error> {
        self.restore_rheology_checkpoint_inner(saved, max_particles, max_materials, false)
    }
    fn restore_rheology_checkpoint_inner(
        &mut self,
        saved: &RheologyCheckpoint,
        max_particles: usize,
        max_materials: usize,
        evaluate: bool,
    ) -> Result<(), Error> {
        if saved.version != 1
            || saved.shear.len() > max_materials
            || saved.shear.len() != self.materials.len()
            || saved.yield_stresses.len() != self.materials.len()
            || saved.thixotropy.len() != self.materials.len()
            || saved.structure.len() > max_particles
            || saved.structure.len() != self.particles.len()
            || saved.particle_materials.len() != self.particles.len()
            || saved
                .particle_materials
                .iter()
                .zip(&self.particles)
                .any(|(m, p)| *m != p.material)
        {
            return Err(Error::InvalidPropertyResponse);
        }
        let shear: Vec<_> = saved
            .shear
            .iter()
            .map(|m| {
                m.map(|bits| {
                    let [
                        reference_rate,
                        flow_index,
                        minimum_rate,
                        minimum_viscosity,
                        maximum_viscosity,
                    ] = bits.map(f64::from_bits);
                    ShearThinning {
                        reference_rate,
                        flow_index,
                        minimum_rate,
                        minimum_viscosity,
                        maximum_viscosity,
                    }
                })
            })
            .collect();
        let thixotropy: Vec<_> = saved
            .thixotropy
            .iter()
            .map(|m| {
                m.map(|bits| {
                    let [
                        recovery_rate,
                        breakdown,
                        broken_viscosity_ratio,
                        broken_yield_ratio,
                    ] = bits.map(f64::from_bits);
                    Thixotropy {
                        recovery_rate,
                        breakdown,
                        broken_viscosity_ratio,
                        broken_yield_ratio,
                    }
                })
            })
            .collect();
        let yields: Vec<_> = saved
            .yield_stresses
            .iter()
            .copied()
            .map(f64::from_bits)
            .collect();
        let structure: Vec<_> = saved
            .structure
            .iter()
            .copied()
            .map(f64::from_bits)
            .collect();
        if shear.iter().flatten().any(|m| !m.valid())
            || thixotropy.iter().flatten().any(|m| !m.valid())
            || yields
                .iter()
                .zip(&shear)
                .any(|(v, m)| !v.is_finite() || *v < 0. || (m.is_none() && *v != 0.))
            || structure
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(Error::InvalidPropertyResponse);
        }
        let mut candidate = self.clone();
        candidate.shear_thinning = shear;
        candidate.yield_stresses = yields;
        candidate.thixotropy = thixotropy;
        candidate.structure = structure;
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
    use crate::liquid::{Config, LiquidCheckpointLimits};
    #[test]
    fn structural_viscosity_history_rebuilds_and_continues_without_reset() {
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
                    velocity: [0., 0.2, 0.],
                    mass: 1.,
                    material: 0,
                },
                Particle {
                    position: [0.04, 0., 0.],
                    velocity: [0., -0.2, 0.],
                    mass: 1.,
                    material: 0,
                },
            ],
            vec![Material {
                viscosity: 0.1,
                ..Material::WATER
            }],
            config,
        )
        .unwrap();
        original
            .configure_herschel_bulkley(&[Some(HerschelBulkley {
                shear_thinning: ShearThinning {
                    reference_rate: 1.,
                    flow_index: 0.6,
                    minimum_rate: 0.01,
                    minimum_viscosity: 0.001,
                    maximum_viscosity: 1.,
                },
                yield_stress: 0.1,
            })])
            .unwrap();
        original
            .configure_thixotropy(
                vec![Some(Thixotropy {
                    recovery_rate: 0.4,
                    breakdown: 0.8,
                    broken_viscosity_ratio: 0.1,
                    broken_yield_ratio: 0.2,
                })],
                vec![0.3, 0.7],
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
            original.structure_fractions()[0].to_bits(),
            0.3_f64.to_bits()
        );
        let limits = LiquidCheckpointLimits {
            max_particles: 8,
            max_materials: 1,
            max_pairs: 16,
            max_neighbor_checks: 64,
            max_substeps: 256,
        };
        let saved = original.rheology_checkpoint();
        let polymer = original.polymer_checkpoint();
        let mut restored = original.particle_checkpoint().prepare_base(limits).unwrap();
        restored.restore_rheology_checkpoint(&saved, 8, 1).unwrap();
        restored.restore_polymer_checkpoint(&polymer, 8, 1).unwrap();
        for _ in 0..120 {
            assert_eq!(
                original.step(0.0001, None).unwrap(),
                restored.step(0.0001, None).unwrap()
            );
            assert_eq!(
                original.particle_checkpoint(),
                restored.particle_checkpoint()
            );
            assert_eq!(
                original.rheology_checkpoint(),
                restored.rheology_checkpoint()
            );
            assert_eq!(original.polymer_checkpoint(), restored.polymer_checkpoint());
        }
        let before = restored.clone();
        let mut bad = saved.clone();
        bad.structure[0] = 1.1_f64.to_bits();
        assert!(restored.restore_rheology_checkpoint(&bad, 8, 1).is_err());
        assert_eq!(restored, before);
        let mut bad = saved.clone();
        bad.yield_stresses[0] = f64::NAN.to_bits();
        assert!(restored.restore_rheology_checkpoint(&bad, 8, 1).is_err());
        assert_eq!(restored, before);
        assert!(restored.restore_rheology_checkpoint(&saved, 8, 0).is_err());
    }
}
