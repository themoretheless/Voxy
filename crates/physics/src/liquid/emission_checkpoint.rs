//! Lossless pulse-source state. Mutable source settings retain their normal
//! advance-time admission semantics; the liquid world is a separate participant.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PulsedEmitterCheckpoint {
    pub version: u32,
    pub pulses: Vec<[u64; 4]>,
    pub elapsed: u64,
    pub emitted_samples: u32,
    pub particle: [u64; 7],
    pub material: usize,
    pub field: Option<[u64; 2]>,
    pub phase_fraction: Option<u64>,
    pub direction: [u64; 3],
    pub source_velocity: [u64; 3],
    pub density: u64,
    pub particle_volume: u64,
    pub nozzle_radius: u64,
}
impl PulsedEmitter {
    #[must_use]
    pub fn checkpoint(&self) -> PulsedEmitterCheckpoint {
        let p = self.template.particle;
        PulsedEmitterCheckpoint {
            version: 1,
            pulses: self
                .pulses
                .iter()
                .map(|p| [p.start, p.duration, p.volume, p.speed].map(f64::to_bits))
                .collect(),
            elapsed: self.elapsed.to_bits(),
            emitted_samples: self.emitted_samples,
            particle: [
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
            field: self
                .template
                .field
                .map(|f| [f.temperature, f.concentration].map(f64::to_bits)),
            phase_fraction: self.template.phase_fraction.map(f64::to_bits),
            direction: self.direction.map(f64::to_bits),
            source_velocity: self.source_velocity.map(f64::to_bits),
            density: self.density.to_bits(),
            particle_volume: self.particle_volume.to_bits(),
            nozzle_radius: self.nozzle_radius.to_bits(),
        }
    }
}
impl PulsedEmitterCheckpoint {
    /// # Errors
    /// Rejects pulse count/version, invalid immutable pulses or nonfinite/negative clock.
    /// Public mutable settings and template fields are checked at normal emission time.
    pub fn prepare(&self, max_pulses: usize) -> Result<PulsedEmitter, Error> {
        let elapsed = f64::from_bits(self.elapsed);
        if self.version != 1
            || self.pulses.len() > max_pulses
            || !elapsed.is_finite()
            || elapsed < 0.
        {
            return Err(Error::InvalidConfig);
        }
        let pulses = self
            .pulses
            .iter()
            .map(|bits| {
                let [start, duration, volume, speed] = bits.map(f64::from_bits);
                EmissionPulse {
                    start,
                    duration,
                    volume,
                    speed,
                }
            })
            .collect();
        let p = self.particle.map(f64::from_bits);
        let template = ParticleInput {
            particle: super::super::Particle {
                position: [p[0], p[1], p[2]],
                velocity: [p[3], p[4], p[5]],
                mass: p[6],
                material: self.material,
            },
            field: self.field.map(|f| super::super::LiquidField {
                temperature: f64::from_bits(f[0]),
                concentration: f64::from_bits(f[1]),
            }),
            phase_fraction: self.phase_fraction.map(f64::from_bits),
        };
        let mut source = PulsedEmitter::new(pulses, template)?;
        source.elapsed = elapsed;
        source.emitted_samples = self.emitted_samples;
        source.direction = self.direction.map(f64::from_bits);
        source.source_velocity = self.source_velocity.map(f64::from_bits);
        source.density = f64::from_bits(self.density);
        source.particle_volume = f64::from_bits(self.particle_volume);
        source.nozzle_radius = f64::from_bits(self.nozzle_radius);
        Ok(source)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquid::{Config, Particle};
    #[test]
    fn restored_nozzle_sample_sequence_and_source_history_match_following_emission() {
        let mut fluid = Liquid::new(Vec::new(), vec![Material::WATER], Config::default()).unwrap();
        let mut source = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.,
                duration: 1.,
                volume: 3e-7,
                speed: 0.8,
            }],
            ParticleInput {
                particle: Particle {
                    position: [0.1, -0., 0.3],
                    velocity: [0.; 3],
                    mass: 1.,
                    material: 0,
                },
                field: None,
                phase_fraction: None,
            },
        )
        .unwrap();
        source.nozzle_radius = 0.03;
        source.direction = [1e300, 2e300, -1e300];
        for _ in 0..7 {
            source.advance(&mut fluid, 0.013).unwrap();
        }
        let saved = source.checkpoint();
        assert!(saved.emitted_samples > 0);
        let mut restored = saved.prepare(1).unwrap();
        // Liquid checkpoint is not implemented here; clone the same accepted fluid
        // to isolate the source participant's continuation contract.
        let mut loaded_fluid = fluid.clone();
        assert_eq!(restored.checkpoint(), saved);
        for frame in 0..120 {
            let dt = [0.003, 0.019, 0.027][frame % 3];
            let velocity = [f64::from(frame as u16) * 0.01, -0., 0.2];
            source.source_velocity = velocity;
            restored.source_velocity = velocity;
            assert_eq!(
                source.advance(&mut fluid, dt).unwrap(),
                restored.advance(&mut loaded_fluid, dt).unwrap()
            );
            assert_eq!(source.checkpoint(), restored.checkpoint());
            assert_eq!(fluid, loaded_fluid);
            for (a, b) in fluid.particles().iter().zip(loaded_fluid.particles()) {
                assert_eq!(a.position.map(f64::to_bits), b.position.map(f64::to_bits));
                assert_eq!(a.velocity.map(f64::to_bits), b.velocity.map(f64::to_bits));
                assert_eq!(a.mass.to_bits(), b.mass.to_bits());
            }
        }
        assert!(saved.prepare(0).is_err());
        let mut bad = saved.clone();
        bad.elapsed = f64::NAN.to_bits();
        assert!(bad.prepare(1).is_err());
        let mut bad = saved.clone();
        bad.pulses[0][1] = 0;
        assert!(bad.prepare(1).is_err());
        // A mutable invalid setting remains pending rather than being silently repaired.
        let mut pending = saved;
        pending.nozzle_radius = (-1_f64).to_bits();
        let mut pending_source = pending.prepare(1).unwrap();
        assert!(pending_source.advance(&mut loaded_fluid, 0.01).is_err());
        assert_eq!(pending_source.checkpoint(), pending);
    }
}
