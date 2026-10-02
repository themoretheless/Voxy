//! Explicit eligibility for droplet coalescence, separate from SPH sampling.
use super::{Error, Liquid};
impl Liquid {
    /// Enables a coalescence population mask. True identifies a prescribed droplet;
    /// false is an ordinary fluid sample. None restores legacy all-particle eligibility.
    /// New exchange/source particles start false; explicit splits and merges become true.
    /// This does not switch off SPH forces or establish a separate droplet drag model.
    pub fn configure_droplet_population(&mut self, flags: Option<Vec<bool>>) -> Result<(), Error> {
        if flags
            .as_ref()
            .is_some_and(|v| v.len() != self.particles.len())
        {
            return Err(Error::InvalidConfig);
        }
        self.droplet_population = flags;
        Ok(())
    }
    pub fn droplet_population(&self) -> Option<&[bool]> {
        self.droplet_population.as_deref()
    }
}

impl Liquid {
    pub(super) fn retain_carrier_pairs(&self, pairs: &mut Vec<(usize, usize)>) {
        if self.carrier_droplet_stage {
            if let Some(flags) = &self.droplet_population {
                pairs.retain(|(i, j)| !flags[*i] && !flags[*j]);
            }
        }
    }
    /// Advances the carrier with SPH, while marked drops receive only gravity and
    /// containing-box collision response. Drag is a separate explicit stage.
    /// Carrier/drop and drop/drop SPH pressure, viscosity and pair transport are
    /// excluded. Uses the existing adaptive semi-implicit advection and preserves
    /// original particle ordering and fields. Sampled/moving/image boundaries and
    /// gas/phase models are not yet supported in this mode. Atomic.
    pub fn step_carrier_and_droplets(
        &mut self,
        dt: f64,
        container: Option<super::Container>,
    ) -> Result<super::StepStats, Error> {
        let flags = self
            .droplet_population
            .as_ref()
            .ok_or(Error::InvalidConfig)?;
        if flags.iter().any(|v| *v)
            && (!self.boundaries.is_empty()
                || self.boundary_coupling.is_some()
                || self.reflecting_box.is_some()
                || self.gas_active()
                || self.phase_fractions().is_some())
        {
            return Err(Error::InvalidConfig);
        }
        let mut candidate = self.clone();
        candidate.carrier_droplet_stage = true;
        let flags = flags.clone();
        let gravity = self.config.gravity;
        let result = candidate.advance(dt, container, |particles, time| {
            for (i, p) in particles.iter_mut().enumerate() {
                for k in 0..3 {
                    p.position[k] += p.velocity[k] * time;
                    if flags[i] {
                        p.position[k] -= 0.5 * gravity[k] * time * time;
                    }
                }
            }
            Ok(())
        })?;
        candidate.carrier_droplet_stage = false;
        *self = candidate;
        Ok(result)
    }
}
