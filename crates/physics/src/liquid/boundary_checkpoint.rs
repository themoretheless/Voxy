//! Persistent boundary geometry/policy; the spatial grid is rebuilt by native admission.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiquidBoundaryCheckpoint {
    pub version: u32,
    pub smoothing_radius: u64,
    pub samples: Vec<[u64; 4]>,
    pub velocities: Vec<[u64; 3]>,
    pub static_pressure_extrapolation: bool,
    pub reflecting_box: Option<[[u64; 3]; 2]>,
    pub no_slip: bool,
}
impl Liquid {
    /// Captures only between accepted public steps, not an in-flight body transaction.
    /// # Errors
    /// Rejects active temporary boundary-body coupling.
    pub fn boundary_checkpoint(&self) -> Result<LiquidBoundaryCheckpoint, Error> {
        if self.boundary_coupling.is_some() {
            return Err(Error::InvalidBoundary);
        }
        Ok(LiquidBoundaryCheckpoint {
            version: 1,
            smoothing_radius: self.config.smoothing_radius.to_bits(),
            samples: self
                .boundaries
                .iter()
                .map(|s| [s.position[0], s.position[1], s.position[2], s.volume].map(f64::to_bits))
                .collect(),
            velocities: self
                .boundary_velocities
                .iter()
                .map(|v| v.map(f64::to_bits))
                .collect(),
            static_pressure_extrapolation: self.static_pressure_extrapolation,
            reflecting_box: self
                .reflecting_box
                .map(|b| [b.min.map(f64::to_bits), b.max.map(f64::to_bits)]),
            no_slip: self.reflecting_no_slip == reflecting_box::WallViscosity::NoSlip,
        })
    }
    /// Replaces geometry/velocities/policies atomically, preserving native sample order.
    /// A matching kernel radius is required to rebuild the same grid and neighbors.
    /// # Errors
    /// Rejects limits, incompatible/in-flight state and invalid geometry/policies.
    pub fn restore_boundary_checkpoint(
        &mut self,
        saved: &LiquidBoundaryCheckpoint,
        max_particles: usize,
        max_samples: usize,
    ) -> Result<(), Error> {
        if saved.version != 1
            || saved.smoothing_radius != self.config.smoothing_radius.to_bits()
            || self.particles.len() > max_particles
            || saved.samples.len() > max_samples
            || saved.velocities.len() != saved.samples.len()
            || self.boundary_coupling.is_some()
        {
            return Err(Error::InvalidBoundary);
        }
        let samples = saved
            .samples
            .iter()
            .map(|bits| {
                let [x, y, z, volume] = bits.map(f64::from_bits);
                BoundarySample {
                    position: [x, y, z],
                    volume,
                }
            })
            .collect();
        let velocities = saved
            .velocities
            .iter()
            .map(|v| v.map(f64::from_bits))
            .collect();
        let bounds = saved.reflecting_box.map(|b| ReflectingBox {
            min: b[0].map(f64::from_bits),
            max: b[1].map(f64::from_bits),
        });
        let mut candidate = self.clone();
        candidate.set_reflecting_box(None)?;
        candidate.set_static_pressure_extrapolation(false)?;
        candidate.configure_boundaries(samples)?;
        candidate.configure_boundary_velocities(velocities)?;
        candidate.set_static_pressure_extrapolation(saved.static_pressure_extrapolation)?;
        candidate.set_reflecting_box(bounds)?;
        candidate.set_reflecting_no_slip(saved.no_slip)?;
        candidate.validate_gas_geometry()?;
        *self = candidate;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sampled_and_reflecting_boundaries_rebuild_grid_and_continue_exactly() {
        for mode in 0..4 {
            let sampled = mode < 2;
            let config = Config {
                smoothing_radius: 1.,
                particle_radius: 0.05,
                gravity: [0., -0.1, 0.],
                max_particles: 8,
                max_pairs: 64,
                max_neighbor_checks: 256,
                max_substeps: 256,
            };
            let mut original = Liquid::new(
                vec![Particle {
                    position: [if sampled { 0.1 } else { -1.8 }, 0., 0.],
                    velocity: [0., 0.01, 0.],
                    mass: 1.,
                    material: 0,
                }],
                vec![Material {
                    rest_density: 1.,
                    sound_speed: 1.,
                    viscosity: 0.01,
                }],
                config,
            )
            .unwrap();
            let bounds = ReflectingBox {
                min: [-2.; 3],
                max: [2.; 3],
            };
            let walls = Container {
                min: bounds.min,
                max: bounds.max,
                restitution: 0.,
                friction: 0.,
            };
            if sampled {
                original
                    .configure_boundaries(vec![BoundarySample {
                        position: [-0.1, 0., 0.],
                        volume: 1.,
                    }])
                    .unwrap();
                original.translate_boundaries([0.01, 0.005, 0.]).unwrap();
                if mode == 0 {
                    original
                        .configure_boundary_velocities(vec![[0., 0.2, 0.]])
                        .unwrap();
                } else {
                    original.set_static_pressure_extrapolation(true).unwrap();
                }
            } else {
                original.set_reflecting_box(Some(bounds)).unwrap();
                original.set_reflecting_no_slip(mode == 3).unwrap();
            }
            for _ in 0..7 {
                original.step(0.0001, Some(walls)).unwrap();
            }
            let saved = original.boundary_checkpoint().unwrap();
            let limits = LiquidCheckpointLimits {
                max_particles: 8,
                max_materials: 1,
                max_pairs: 64,
                max_neighbor_checks: 256,
                max_substeps: 256,
            };
            let mut restored = original.particle_checkpoint().prepare_base(limits).unwrap();
            if sampled {
                restored.set_reflecting_box(Some(bounds)).unwrap();
            } else {
                restored
                    .configure_boundaries(vec![BoundarySample {
                        position: [-0.1, 0., 0.],
                        volume: 1.,
                    }])
                    .unwrap();
                restored.set_static_pressure_extrapolation(true).unwrap();
            }
            restored.restore_boundary_checkpoint(&saved, 8, 8).unwrap();
            assert_eq!(restored.boundary_checkpoint().unwrap(), saved);
            assert_eq!(original.boundary_grid, restored.boundary_grid);
            for _ in 0..120 {
                assert_eq!(
                    original.step(0.0001, Some(walls)).unwrap(),
                    restored.step(0.0001, Some(walls)).unwrap()
                );
                assert_eq!(
                    original.particle_checkpoint(),
                    restored.particle_checkpoint()
                );
                assert_eq!(
                    original.boundary_checkpoint().unwrap(),
                    restored.boundary_checkpoint().unwrap()
                );
                assert_eq!(original.boundary_grid, restored.boundary_grid);
                if sampled {
                    assert_eq!(
                        original.boundary_diagnostics().unwrap(),
                        restored.boundary_diagnostics().unwrap()
                    );
                } else {
                    assert_eq!(
                        original.reflecting_diagnostics().unwrap(),
                        restored.reflecting_diagnostics().unwrap()
                    );
                }
            }
            let before = restored.clone();
            let mut bad = saved.clone();
            bad.version = 2;
            assert!(restored.restore_boundary_checkpoint(&bad, 8, 8).is_err());
            assert_eq!(restored, before);
            let mut bad = saved.clone();
            if sampled {
                bad.velocities[0][0] = f64::NAN.to_bits();
            } else {
                bad.reflecting_box.as_mut().unwrap()[1] = bad.reflecting_box.unwrap()[0];
            }
            assert!(restored.restore_boundary_checkpoint(&bad, 8, 8).is_err());
            assert_eq!(restored, before);
            if sampled {
                assert!(restored.restore_boundary_checkpoint(&saved, 8, 0).is_err());
            }
        }
    }
}
