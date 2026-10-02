//! Configurable viscosity profiles and deterministic, volume-accounted pulse emission.
//! Presets are illustrative numerical controls.
use super::{Error, Liquid, Material, ParticleInput, ShearThinning, finite, norm, positive};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViscosityProfile {
    pub material: Material,
    pub shear_thinning: ShearThinning,
}
impl ViscosityProfile {
    pub const FLUID_DEMO: Self = Self::preset(0.01, 0.9);
    pub const VISCOUS_DEMO: Self = Self::preset(0.1, 0.7);
    pub const THICK_DEMO: Self = Self::preset(1.0, 0.5);
    const fn preset(viscosity: f64, flow_index: f64) -> Self {
        Self {
            material: Material {
                rest_density: 1000.0,
                sound_speed: 20.0,
                viscosity,
            },
            shear_thinning: ShearThinning {
                reference_rate: 1.0,
                flow_index,
                minimum_rate: 0.01,
                minimum_viscosity: viscosity * 0.01,
                maximum_viscosity: viscosity * 100.0,
            },
        }
    }
}

/// Constant-volume-rate pulse. Time is seconds and volume is cubic metres in SI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmissionPulse {
    pub start: f64,
    pub duration: f64,
    pub volume: f64,
    pub speed: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PulsedEmitter {
    pulses: Vec<EmissionPulse>,
    elapsed: f64,
    emitted_samples: u32,
    /// Source position, material and transported fields. Mass/velocity are replaced.
    pub template: ParticleInput,
    pub direction: [f64; 3],
    /// Explicit source density; must match the desired material calibration.
    pub density: f64,
    /// Upper bound on each emitted particle's volume.
    pub particle_volume: f64,
    /// Circular aperture radius, perpendicular to direction. Zero keeps a point source.
    /// Area sampling does not guarantee minimum particle separation.
    pub nozzle_radius: f64,
}
impl PulsedEmitter {
    /// # Errors
    /// Rejects negative times/volumes, nonpositive durations and invalid speeds.
    pub fn new(pulses: Vec<EmissionPulse>, template: ParticleInput) -> Result<Self, Error> {
        if pulses.iter().any(|p| {
            !p.start.is_finite()
                || p.start < 0.0
                || !positive(p.duration)
                || !(p.start + p.duration).is_finite()
                || !p.volume.is_finite()
                || p.volume < 0.0
                || !p.speed.is_finite()
                || p.speed < 0.0
        }) {
            return Err(Error::InvalidConfig);
        }
        Ok(Self {
            pulses,
            elapsed: 0.0,
            emitted_samples: 0,
            template,
            direction: [0.0, 1.0, 0.0],
            density: 1000.0,
            particle_volume: 1e-8,
            nozzle_radius: 0.0,
        })
    }
    #[must_use]
    pub fn elapsed(&self) -> f64 {
        self.elapsed
    }

    /// Emits the overlap of every pulse with this interval; failure rolls back time and fluid.
    /// Partial final particles preserve volume rather than rounding it away.
    /// # Errors
    /// Invalid settings/time, overflow, source validation or the liquid particle budget.
    pub fn advance(
        &mut self,
        liquid: &mut Liquid,
        dt: f64,
    ) -> Result<super::ParticleExchange, Error> {
        self.advance_composed(liquid, dt, None)
    }
    /// Emits source particles with a complete species mass-fraction vector.
    /// # Errors
    /// Usual emission errors or missing/invalid composition configuration.
    /// Both elapsed source time and the complete fluid roll back together.
    pub fn advance_with_species(
        &mut self,
        liquid: &mut Liquid,
        dt: f64,
        composition: &[f64],
    ) -> Result<super::ParticleExchange, Error> {
        self.advance_composed(liquid, dt, Some(composition))
    }
    fn advance_composed(
        &mut self,
        liquid: &mut Liquid,
        dt: f64,
        composition: Option<&[f64]>,
    ) -> Result<super::ParticleExchange, Error> {
        if let Some(row) = composition {
            let count = liquid.species_names().ok_or(Error::InvalidTransport)?.len();
            super::species::validate_row(row, count)?;
        }
        let end = self.elapsed + dt;
        let length = norm(self.direction);
        if !positive(dt)
            || !end.is_finite()
            || end <= self.elapsed
            || !finite(self.direction)
            || !positive(length)
            || !positive(self.density)
            || !positive(self.particle_volume)
            || !self.nozzle_radius.is_finite()
            || self.nozzle_radius < 0.0
        {
            return Err(Error::InvalidConfig);
        }
        let mut add = Vec::new();
        let mut samples = self.emitted_samples;
        let direction = self.direction.map(|v| v / length);
        let (tangent, bitangent) = aperture_frame(direction);
        let available = liquid.config.max_particles - liquid.particles.len();
        for pulse in &self.pulses {
            let overlap =
                (end.min(pulse.start + pulse.duration) - self.elapsed.max(pulse.start)).max(0.0);
            let mut volume = pulse.volume * (overlap / pulse.duration);
            if !volume.is_finite() {
                return Err(Error::NumericalFailure);
            }
            while volume > 0.0 {
                if add.len() >= available {
                    return Err(Error::ParticleBudget);
                }
                let part = volume.min(self.particle_volume);
                let mut source = self.template;
                source.particle.mass = part * self.density;
                source.particle.velocity = self.direction.map(|v| v / length * pulse.speed);
                if self.nozzle_radius > 0.0 {
                    samples = samples.checked_add(1).ok_or(Error::NumericalFailure)?;
                    // Base-2 radical inverse gives uniform disk area; golden-angle azimuth.
                    let radius = self.nozzle_radius
                        * (f64::from(samples.reverse_bits()) / 4_294_967_296.0).sqrt();
                    let angle = f64::from(samples) * std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
                    for axis in 0..3 {
                        source.particle.position[axis] +=
                            radius * (angle.cos() * tangent[axis] + angle.sin() * bitangent[axis]);
                    }
                }
                add.push(source);
                let remaining = volume - part;
                if remaining >= volume {
                    return Err(Error::NumericalFailure);
                }
                volume = remaining;
            }
        }
        let ledger = if let Some(row) = composition {
            let rows: Vec<_> = add.iter().map(|_| row.to_vec()).collect();
            liquid.exchange_particles_with_species(&[], &add, &rows)?
        } else {
            liquid.exchange_particles(&[], &add)?
        };
        self.elapsed = end;
        self.emitted_samples = samples;
        Ok(ledger)
    }
}

fn aperture_frame(direction: [f64; 3]) -> ([f64; 3], [f64; 3]) {
    // Cross with the least-aligned coordinate axis to avoid a vanishing tangent.
    let axis = (0..3)
        .min_by(|a, b| direction[*a].abs().total_cmp(&direction[*b].abs()))
        .unwrap_or(0);
    let mut reference = [0.0; 3];
    reference[axis] = 1.0;
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let tangent = cross(direction, reference);
    let length = norm(tangent);
    let tangent = tangent.map(|v| v / length);
    (tangent, cross(direction, tangent))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquid::{Config, Particle};
    #[test]
    fn circular_aperture_samples_area_in_the_normal_plane_and_rolls_back() {
        let template = ParticleInput {
            particle: Particle {
                position: [4.0, 5.0, 6.0],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            },
            field: None,
            phase_fraction: None,
        };
        let particle_volume = 2.0_f64.powi(-30);
        let mut source = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.0,
                duration: 1.0,
                volume: 128.0 * particle_volume,
                speed: 2.0,
            }],
            template,
        )
        .unwrap();
        source.particle_volume = particle_volume;
        source.nozzle_radius = 0.1;
        source.direction = [1.0, 2.0, 3.0];
        let mut fluid = Liquid::new(vec![], vec![Material::WATER], Config::default()).unwrap();
        let mut point = source.clone();
        point.nozzle_radius = 0.0;
        let mut point_fluid = fluid.clone();
        let point_ledger = point.advance(&mut point_fluid, 1.0).unwrap();
        let ledger = source.advance(&mut fluid, 1.0).unwrap();
        assert_eq!(ledger, point_ledger);
        assert_eq!(fluid.particles.len(), 128);
        let direction = source.direction.map(|v| v / 14.0_f64.sqrt());
        let mut mean_squared_radius = 0.0;
        for (i, particle) in fluid.particles.iter().enumerate() {
            let delta = super::super::sub(particle.position, template.particle.position);
            let radius = norm(delta);
            assert!(radius <= source.nozzle_radius);
            let normal_distance: f64 = delta.iter().zip(direction).map(|(v, n)| v * n).sum();
            assert!(normal_distance.abs() < 1e-14);
            mean_squared_radius += radius * radius / 128.0;
            for other in &fluid.particles[..i] {
                assert!(norm(super::super::sub(particle.position, other.position)) > 1e-7);
            }
        }
        assert!((mean_squared_radius / source.nozzle_radius.powi(2) - 0.5).abs() < 0.01);
        let mut failed = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.0,
                duration: 1.0,
                volume: 128.0 * particle_volume,
                speed: 2.0,
            }],
            template,
        )
        .unwrap();
        failed.nozzle_radius = 0.1;
        failed.template.particle.material = 99;
        let before = failed.clone();
        let before_fluid = fluid.clone();
        assert!(failed.advance(&mut fluid, 1.0).is_err());
        assert_eq!(failed, before);
        assert_eq!(fluid, before_fluid);
    }
    #[test]
    fn pulses_conserve_volume_momentum_and_rollback() {
        let template = ParticleInput {
            particle: Particle {
                position: [0.0; 3],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            },
            field: None,
            phase_fraction: None,
        };
        let mut emitter = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.1,
                duration: 0.2,
                volume: 3e-8,
                speed: 2.0,
            }],
            template,
        )
        .unwrap();
        let mut liquid = Liquid::new(vec![], vec![Material::WATER], Config::default()).unwrap();
        emitter.advance(&mut liquid, 0.1).unwrap();
        assert!(liquid.particles.is_empty());
        let first = emitter.advance(&mut liquid, 0.1).unwrap();
        let second = emitter.advance(&mut liquid, 0.2).unwrap();
        assert!((first.added.mass + second.added.mass - 3e-5).abs() < 1e-18);
        assert!((first.added.momentum[1] + second.added.momentum[1] - 6e-5).abs() < 1e-18);
        let before = emitter.clone();
        let fluid_before = liquid.clone();
        assert!(emitter.advance(&mut liquid, f64::NAN).is_err());
        assert_eq!(emitter, before);
        assert_eq!(liquid, fluid_before);
        let mut limited = Liquid::new(
            vec![],
            vec![Material::WATER],
            Config {
                max_particles: 1,
                ..Config::default()
            },
        )
        .unwrap();
        let mut emitter = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.0,
                duration: 1.0,
                volume: 3e-8,
                speed: 2.0,
            }],
            template,
        )
        .unwrap();
        assert_eq!(
            emitter.advance(&mut limited, 1.0),
            Err(Error::ParticleBudget)
        );
        assert_eq!(emitter.elapsed(), 0.0);
        assert!(limited.particles.is_empty());
    }
}
