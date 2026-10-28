//! Cohesion, inter-material overlap penalties and container-wall response.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceInteractionCheckpoint {
    pub version: u32,
    pub materials: Vec<[u64; 3]>,
    pub strengths: Vec<u64>,
    pub interfaces: Vec<([usize; 2], u64)>,
    pub wall_adhesion: Vec<Option<[u64; 2]>>,
}
impl Liquid {
    #[must_use]
    pub fn surface_checkpoint(&self) -> SurfaceInteractionCheckpoint {
        SurfaceInteractionCheckpoint {
            version: 1,
            materials: self
                .materials
                .iter()
                .map(|m| [m.rest_density, m.sound_speed, m.viscosity].map(f64::to_bits))
                .collect(),
            strengths: self.surface_strengths.iter().map(|v| v.to_bits()).collect(),
            interfaces: self
                .interface_penalties
                .iter()
                .map(|((a, b), v)| ([*a, *b], v.to_bits()))
                .collect(),
            wall_adhesion: self
                .wall_adhesion
                .iter()
                .map(|a| a.map(|a| [a.acceleration, a.range].map(f64::to_bits)))
                .collect(),
        }
    }
    /// Atomically replaces this configuration, removing obsolete interface entries.
    /// Container geometry is supplied by the caller, as in ordinary stepping.
    /// # Errors
    /// Rejects limits/material mismatch, invalid coefficients and noncanonical pair keys.
    pub fn restore_surface_checkpoint(
        &mut self,
        saved: &SurfaceInteractionCheckpoint,
        max_particles: usize,
        max_materials: usize,
        max_interfaces: usize,
    ) -> Result<(), Error> {
        if saved.version != 1
            || self.particles.len() > max_particles
            || saved.materials.len() > max_materials
            || saved.materials.len() != self.materials.len()
            || saved.strengths.len() != self.materials.len()
            || saved.wall_adhesion.len() != self.materials.len()
            || saved.interfaces.len() > max_interfaces
            || saved
                .materials
                .iter()
                .zip(&self.materials)
                .any(|(bits, m)| {
                    *bits != [m.rest_density, m.sound_speed, m.viscosity].map(f64::to_bits)
                })
        {
            return Err(Error::InvalidSurfaceStrength);
        }
        let mut keys = std::collections::BTreeSet::new();
        for (pair, bits) in &saved.interfaces {
            if pair[0] >= pair[1] || !keys.insert(*pair) || !positive(f64::from_bits(*bits)) {
                return Err(Error::InvalidSurfaceStrength);
            }
        }
        let mut candidate = self.clone();
        candidate.interface_penalties.clear();
        for material in 0..saved.materials.len() {
            candidate.set_surface_strength(material, f64::from_bits(saved.strengths[material]))?;
            candidate.set_wall_adhesion(
                material,
                saved.wall_adhesion[material].map(|bits| WallAdhesion {
                    acceleration: f64::from_bits(bits[0]),
                    range: f64::from_bits(bits[1]),
                }),
            )?;
        }
        for ([first, second], bits) in &saved.interfaces {
            candidate.set_interface_penalty(*first, *second, f64::from_bits(*bits))?;
        }
        *self = candidate;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_surface_response_fields_restore_and_continue_with_container_walls() {
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
                    position: [-0.92, 0., 0.],
                    velocity: [0.; 3],
                    mass: 1.,
                    material: 0,
                },
                Particle {
                    position: [-0.84, 0., 0.],
                    velocity: [0.; 3],
                    mass: 1.,
                    material: 1,
                },
            ],
            vec![Material::WATER; 3],
            config,
        )
        .unwrap();
        original.set_surface_strength(0, 1e-7).unwrap();
        original.set_surface_strength(1, 2e-7).unwrap();
        original.set_interface_penalty(0, 1, 1e-6).unwrap();
        original
            .set_wall_adhesion(
                0,
                Some(WallAdhesion {
                    acceleration: 2.,
                    range: 0.15,
                }),
            )
            .unwrap();
        original
            .set_wall_adhesion(
                1,
                Some(WallAdhesion {
                    acceleration: 1.5,
                    range: 0.25,
                }),
            )
            .unwrap();
        let walls = Container {
            min: [-1.; 3],
            max: [1.; 3],
            restitution: 0.,
            friction: 0.,
        };
        assert!(original.interface_energy().unwrap() > 0.);
        for _ in 0..7 {
            original.step(0.0001, Some(walls)).unwrap();
        }
        assert_ne!(
            original.particles()[0].position[0].to_bits(),
            (-0.92_f64).to_bits()
        );
        let saved = original.surface_checkpoint();
        let limits = LiquidCheckpointLimits {
            max_particles: 8,
            max_materials: 3,
            max_pairs: 16,
            max_neighbor_checks: 64,
            max_substeps: 256,
        };
        let mut restored = original.particle_checkpoint().prepare_base(limits).unwrap();
        restored.set_interface_penalty(0, 2, 4e-6).unwrap();
        restored
            .restore_surface_checkpoint(&saved, 8, 3, 3)
            .unwrap();
        assert_eq!(restored.surface_checkpoint(), saved);
        for _ in 0..120 {
            assert_eq!(
                original.step(0.0001, Some(walls)).unwrap(),
                restored.step(0.0001, Some(walls)).unwrap()
            );
            assert_eq!(
                original.particle_checkpoint(),
                restored.particle_checkpoint()
            );
            assert_eq!(original.surface_checkpoint(), restored.surface_checkpoint());
            assert_eq!(
                original.interface_energy().unwrap().to_bits(),
                restored.interface_energy().unwrap().to_bits()
            );
        }
        let before = restored.clone();
        let mut bad = saved.clone();
        bad.interfaces.push(bad.interfaces[0]);
        assert!(restored.restore_surface_checkpoint(&bad, 8, 3, 3).is_err());
        assert_eq!(restored, before);
        let mut bad = saved.clone();
        bad.wall_adhesion[0].as_mut().unwrap()[1] = 0;
        assert!(restored.restore_surface_checkpoint(&bad, 8, 3, 3).is_err());
        assert_eq!(restored, before);
        let mut bad = saved.clone();
        bad.strengths[0] = f64::NAN.to_bits();
        assert!(restored.restore_surface_checkpoint(&bad, 8, 3, 3).is_err());
        assert_eq!(restored, before);
        assert!(
            restored
                .restore_surface_checkpoint(&saved, 8, 3, 0)
                .is_err()
        );
    }
}
