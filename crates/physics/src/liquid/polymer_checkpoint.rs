//! Maxwell configuration, per-particle memory and accumulated relaxation heat.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolymerCheckpoint {
    pub version: u32,
    pub laws: Vec<Option<[u64; 2]>>,
    pub conformations: Vec<[[u64; 3]; 3]>,
    pub particle_materials: Vec<usize>,
    pub heat: u64,
}
impl Liquid {
    #[must_use]
    pub fn polymer_checkpoint(&self) -> PolymerCheckpoint {
        PolymerCheckpoint {
            version: 1,
            laws: self
                .maxwell_fluids
                .iter()
                .map(|law| law.map(|m| [m.modulus, m.relaxation_time].map(f64::to_bits)))
                .collect(),
            conformations: self
                .conformation
                .iter()
                .map(|c| c.map(|row| row.map(f64::to_bits)))
                .collect(),
            particle_materials: self.particles.iter().map(|p| p.material).collect(),
            heat: self.polymer_heat.to_bits(),
        }
    }
    /// Applies this participant to the matching restored particle layout atomically.
    /// # Errors
    /// Rejects incompatible particle/material ordering, invalid laws/memory/heat and limits.
    pub fn restore_polymer_checkpoint(
        &mut self,
        saved: &PolymerCheckpoint,
        max_particles: usize,
        max_materials: usize,
    ) -> Result<(), Error> {
        self.restore_polymer_checkpoint_inner(saved, max_particles, max_materials, true)
    }
    pub(in crate::liquid) fn stage_polymer_checkpoint(
        &mut self,
        saved: &PolymerCheckpoint,
        max_particles: usize,
        max_materials: usize,
    ) -> Result<(), Error> {
        self.restore_polymer_checkpoint_inner(saved, max_particles, max_materials, false)
    }
    fn restore_polymer_checkpoint_inner(
        &mut self,
        saved: &PolymerCheckpoint,
        max_particles: usize,
        max_materials: usize,
        evaluate: bool,
    ) -> Result<(), Error> {
        let heat = f64::from_bits(saved.heat);
        if saved.version != 1
            || !heat.is_finite()
            || heat < 0.
            || saved.laws.len() != self.materials.len()
            || saved.laws.len() > max_materials
            || saved.conformations.len() != self.particles.len()
            || saved.conformations.len() > max_particles
            || saved.particle_materials.len() != self.particles.len()
            || saved
                .particle_materials
                .iter()
                .zip(&self.particles)
                .any(|(m, p)| *m != p.material)
        {
            return Err(Error::InvalidConfig);
        }
        let mut candidate = self.clone();
        candidate.conformation.fill(IDENTITY);
        for (material, law) in saved.laws.iter().enumerate() {
            candidate.set_maxwell_fluid(
                material,
                law.map(|bits| MaxwellFluid {
                    modulus: f64::from_bits(bits[0]),
                    relaxation_time: f64::from_bits(bits[1]),
                }),
            )?;
        }
        candidate.set_conformations(
            saved
                .conformations
                .iter()
                .map(|c| c.map(|row| row.map(f64::from_bits)))
                .collect(),
        )?;
        candidate.polymer_heat = heat;
        if evaluate {
            candidate.polymer_energy()?;
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
    fn particle_and_polymer_blocks_rebuild_stressed_fluid_and_continue_exactly() {
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
                    position: [-0.04, 0., -0.],
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
            .set_maxwell_fluid(
                0,
                Some(MaxwellFluid {
                    modulus: 5.,
                    relaxation_time: 0.2,
                }),
            )
            .unwrap();
        original
            .set_conformations(vec![[[1.2, 0., 0.], [0., 0.9, 0.], [0., 0., 1.]]; 2])
            .unwrap();
        for _ in 0..7 {
            original.step(0.0001, None).unwrap();
        }
        let particles = original.particle_checkpoint();
        let memory = original.polymer_checkpoint();
        assert!(f64::from_bits(memory.heat) > 0.);
        let limits = LiquidCheckpointLimits {
            max_particles: 8,
            max_materials: 1,
            max_pairs: 16,
            max_neighbor_checks: 64,
            max_substeps: 256,
        };
        let mut restored = particles.prepare_base(limits).unwrap();
        restored.restore_polymer_checkpoint(&memory, 8, 1).unwrap();
        assert_eq!(restored.particle_checkpoint(), particles);
        assert_eq!(restored.polymer_checkpoint(), memory);
        for _ in 0..120 {
            assert_eq!(
                original.step(0.0001, None).unwrap(),
                restored.step(0.0001, None).unwrap()
            );
            assert_eq!(
                original.particle_checkpoint(),
                restored.particle_checkpoint()
            );
            assert_eq!(original.polymer_checkpoint(), restored.polymer_checkpoint());
        }
        let before = restored.clone();
        let mut bad = memory.clone();
        bad.conformations[0][0][0] = (-1_f64).to_bits();
        assert!(restored.restore_polymer_checkpoint(&bad, 8, 1).is_err());
        assert_eq!(restored, before);
        assert!(
            particles
                .prepare_base(LiquidCheckpointLimits {
                    max_particles: 1,
                    ..limits
                })
                .is_err()
        );
        let mut bad = particles;
        bad.particles[0].material = 1;
        assert!(bad.prepare_base(limits).is_err());
    }
}
