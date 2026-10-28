//! Exact particle/material/configuration block. Constitutive, transport and boundary
//! histories are separate required blocks when composing a complete liquid save.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct LiquidCheckpointLimits {
    pub max_particles: usize,
    pub max_materials: usize,
    pub max_pairs: usize,
    pub max_neighbor_checks: usize,
    pub max_substeps: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParticleCheckpoint {
    pub values: [u64; 7],
    pub material: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiquidParticleCheckpoint {
    pub version: u32,
    pub particles: Vec<ParticleCheckpoint>,
    pub materials: Vec<[u64; 3]>,
    pub configuration: [u64; 5],
    pub budgets: [usize; 4],
}
impl Liquid {
    #[must_use]
    pub fn particle_checkpoint(&self) -> LiquidParticleCheckpoint {
        LiquidParticleCheckpoint {
            version: 1,
            particles: self
                .particles
                .iter()
                .map(|p| ParticleCheckpoint {
                    values: [
                        p.position[0],
                        p.position[1],
                        p.position[2],
                        p.velocity[0],
                        p.velocity[1],
                        p.velocity[2],
                        p.mass,
                    ]
                    .map(f64::to_bits),
                    material: p.material,
                })
                .collect(),
            materials: self
                .materials
                .iter()
                .map(|m| [m.rest_density, m.sound_speed, m.viscosity].map(f64::to_bits))
                .collect(),
            configuration: [
                self.config.smoothing_radius,
                self.config.particle_radius,
                self.config.gravity[0],
                self.config.gravity[1],
                self.config.gravity[2],
            ]
            .map(f64::to_bits),
            budgets: [
                self.config.max_particles,
                self.config.max_pairs,
                self.config.max_neighbor_checks,
                self.config.max_substeps,
            ],
        }
    }
}
impl LiquidParticleCheckpoint {
    /// Rebuilds only the particle/configuration base; other active owner blocks must
    /// be restored before this candidate is published as a complete liquid world.
    /// # Errors
    /// Rejects malformed physics data, versions and all saved policies above caller limits.
    pub fn prepare_base(&self, limits: LiquidCheckpointLimits) -> Result<Liquid, Error> {
        let [max_particles, max_pairs, max_neighbor_checks, max_substeps] = self.budgets;
        if max_particles > limits.max_particles
            || self.particles.len() > limits.max_particles
            || self.materials.len() > limits.max_materials
            || max_pairs > limits.max_pairs
            || max_neighbor_checks > limits.max_neighbor_checks
            || max_substeps > limits.max_substeps
        {
            return Err(Error::ParticleBudget);
        }
        if self.version != 1 {
            return Err(Error::InvalidConfig);
        }
        let [smoothing_radius, particle_radius, gx, gy, gz] =
            self.configuration.map(f64::from_bits);
        let config = Config {
            smoothing_radius,
            particle_radius,
            gravity: [gx, gy, gz],
            max_particles,
            max_pairs,
            max_neighbor_checks,
            max_substeps,
        };
        let materials = self
            .materials
            .iter()
            .map(|bits| {
                let [rest_density, sound_speed, viscosity] = bits.map(f64::from_bits);
                Material {
                    rest_density,
                    sound_speed,
                    viscosity,
                }
            })
            .collect();
        let particles = self
            .particles
            .iter()
            .map(|p| {
                let v = p.values.map(f64::from_bits);
                Particle {
                    position: [v[0], v[1], v[2]],
                    velocity: [v[3], v[4], v[5]],
                    mass: v[6],
                    material: p.material,
                }
            })
            .collect();
        Liquid::new(particles, materials, config)
    }
}
