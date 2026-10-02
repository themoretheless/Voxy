//! Reversible scalar structural kinetics coupled to apparent viscosity and yield stress.
use super::{Error, Liquid, Material, Particle, positive};

/// Moore-style kinetics: `dλ/dt = recovery_rate*(1-λ) - breakdown*shear_rate*λ`.
/// λ=1 is fully structured. Coefficients must be calibrated for a material.
/// No elastic stress, structural free energy or bubble population is represented.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Thixotropy {
    /// Rest recovery rate in 1/s; zero disables rebuilding.
    pub recovery_rate: f64,
    /// Dimensionless coefficient multiplying the actual (unfloored) strain rate.
    pub breakdown: f64,
    /// Fraction of fully structured reference viscosity remaining at λ=0.
    pub broken_viscosity_ratio: f64,
    /// Fraction of fully structured yield stress remaining at λ=0.
    pub broken_yield_ratio: f64,
}
impl Thixotropy {
    fn valid(self) -> bool {
        [self.recovery_rate, self.breakdown]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.0)
            && [self.broken_viscosity_ratio, self.broken_yield_ratio]
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    }
    fn evolve(self, structure: f64, rate: f64, dt: f64) -> Result<f64, Error> {
        let destruction = self.breakdown * rate;
        let total = self.recovery_rate + destruction;
        if !total.is_finite() {
            return Err(Error::NumericalFailure);
        }
        if total == 0.0 {
            return Ok(structure);
        }
        let equilibrium = self.recovery_rate / total;
        let change = -(-total * dt).exp_m1();
        let result = structure + (equilibrium - structure) * change;
        if !result.is_finite() || !(0.0..=1.0).contains(&result) {
            return Err(Error::NumericalFailure);
        }
        Ok(result)
    }
}
impl Liquid {
    /// Configures per-material structural kinetics and per-particle λ values.
    /// None disables the response for that material. Replaces configuration atomically.
    /// # Errors
    /// Invalid counts, coefficients, fractions, budgets or constitutive overflow.
    pub fn configure_thixotropy(
        &mut self,
        models: Vec<Option<Thixotropy>>,
        structure: Vec<f64>,
    ) -> Result<(), Error> {
        if models.len() != self.materials.len()
            || structure.len() != self.particles.len()
            || models.iter().flatten().any(|m| !m.valid())
            || structure
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(Error::InvalidPropertyResponse);
        }
        let mut candidate = self.clone();
        candidate.thixotropy = models;
        candidate.structure = structure;
        candidate.effective_materials()?;
        *self = candidate;
        Ok(())
    }
    #[must_use]
    pub fn structure_fractions(&self) -> &[f64] {
        &self.structure
    }

    /// Exact structural evolution at the current frozen strain rates. Positions,
    /// velocities and thermal fields stay fixed. Ordinary steps and isolated viscous
    /// stages already evolve structure; do not apply this again for the same interval.
    /// # Errors
    /// Invalid dt, strain/kinetic overflow, budgets or constitutive failure; atomic.
    pub fn relax_structure(&mut self, dt: f64) -> Result<(), Error> {
        let mut candidate = self.clone();
        candidate.advance_structure(dt)?;
        *self = candidate;
        Ok(())
    }
    pub(super) fn advance_structure(&mut self, dt: f64) -> Result<(), Error> {
        if !positive(dt) {
            return Err(Error::InvalidTimeStep);
        }
        if self.thixotropy.iter().all(Option::is_none) {
            return Ok(());
        }
        self.structure = self.advanced_structure(&self.particles, self.transport.as_ref(), dt)?;
        self.effective_materials()?;
        Ok(())
    }
    pub(super) fn advanced_structure(
        &self,
        particles: &[Particle],
        transport: Option<&super::transport::Transport>,
        dt: f64,
    ) -> Result<Vec<f64>, Error> {
        if !positive(dt) {
            return Err(Error::InvalidTimeStep);
        }
        let properties = self.evaluate_materials(particles, transport)?;
        let rates = self.strain_rates(particles, &properties)?;
        let mut structure = self.structure.clone();
        for (i, particle) in particles.iter().enumerate() {
            if let Some(model) = self.thixotropy[particle.material] {
                structure[i] = model.evolve(structure[i], rates[i], dt)?;
            }
        }
        Ok(structure)
    }
    pub(super) fn apply_structure_viscosity(
        &self,
        particles: &[Particle],
        properties: &mut [Material],
    ) -> Result<(), Error> {
        if particles.len() != self.structure.len() {
            return Err(Error::InvalidPropertyResponse);
        }
        for (i, (particle, property)) in particles.iter().zip(properties).enumerate() {
            if let Some(model) = self.thixotropy[particle.material] {
                let factor = model.broken_viscosity_ratio
                    + (1.0 - model.broken_viscosity_ratio) * self.structure[i];
                property.viscosity *= factor;
                if !property.viscosity.is_finite() {
                    return Err(Error::NumericalFailure);
                }
            }
        }
        Ok(())
    }
    pub(super) fn structure_yield_factor(&self, index: usize) -> f64 {
        self.thixotropy[self.particles[index].material].map_or(1.0, |model| {
            model.broken_yield_ratio + (1.0 - model.broken_yield_ratio) * self.structure[index]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquid::Config;
    #[test]
    fn adaptive_substeps_match_replaying_each_accepted_interval() {
        let mut particles = Vec::new();
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let position = [f64::from(x) * 0.1, f64::from(y) * 0.1, f64::from(z) * 0.1];
                    particles.push(Particle {
                        position,
                        velocity: [
                            0.3 * position[0] + 0.4 * position[1],
                            0.4 * position[0] + 0.1 * position[1],
                            -0.2 * position[2],
                        ],
                        mass: 0.5,
                        material: 0,
                    });
                }
            }
        }
        let mut fluid = Liquid::new(
            particles,
            vec![Material {
                viscosity: 10.0,
                ..Material::WATER
            }],
            Config {
                smoothing_radius: 1.0,
                gravity: [0.0; 3],
                ..Config::default()
            },
        )
        .unwrap();
        fluid
            .configure_thixotropy(
                vec![Some(Thixotropy {
                    recovery_rate: 2.0,
                    breakdown: 3.0,
                    broken_viscosity_ratio: 0.2,
                    broken_yield_ratio: 0.0,
                })],
                vec![0.2; 27],
            )
            .unwrap();
        let mut replay = fluid.clone();
        let mut failed = fluid.clone();
        let before_failure = failed.clone();
        let mut intervals = Vec::new();
        let stats = fluid
            .advance(0.05, None, |particles, time| {
                intervals.push(time);
                for particle in particles {
                    for axis in 0..3 {
                        particle.position[axis] += time * particle.velocity[axis];
                    }
                }
                Ok(())
            })
            .unwrap();
        assert!(stats.substeps > 1);
        let mut calls = 0;
        let error = failed.advance(0.05, None, |particles, time| {
            calls += 1;
            if calls == 2 {
                return Err(Error::CollisionBackend);
            }
            for particle in particles {
                for axis in 0..3 {
                    particle.position[axis] += time * particle.velocity[axis];
                }
            }
            Ok(())
        });
        assert_eq!(calls, 2);
        assert_eq!(error, Err(Error::CollisionBackend));
        assert_eq!(failed, before_failure);
        for interval in intervals {
            assert_eq!(replay.step(interval, None).unwrap().substeps, 1);
        }
        for (p, q) in fluid.particles().iter().zip(replay.particles()) {
            for axis in 0..3 {
                assert!((p.position[axis] - q.position[axis]).abs() < 1e-12);
                assert!((p.velocity[axis] - q.velocity[axis]).abs() < 1e-12);
            }
        }
        for (a, b) in fluid
            .structure_fractions()
            .iter()
            .zip(replay.structure_fractions())
        {
            assert!((a - b).abs() < 1e-12);
        }
    }
}
