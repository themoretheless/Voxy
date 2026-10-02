use super::{ContactConfig, Error, Liquid, StepStats, finite};
/// Net mechanical transfer from the prescribed translating geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TranslatingWorldReport {
    pub fluid: StepStats,
    pub impulse: [f64; 3],
    /// Work supplied by the geometry drive: velocity dot external impulse.
    pub drive_work: f64,
}
impl Liquid {
    /// Advances against a whole collision world translating with constant velocity.
    /// The backend describes geometry at the start of this call. All configured SPH
    /// boundaries share this translation; surface velocities are set to this common motion.
    /// Positions/velocities return in the laboratory frame and samples move by velocity*dt.
    /// Backend geometry is read-only: its owner must update it before the next call.
    /// No rotation, acceleration, static second world or rigid-body recoil is modelled.
    /// # Errors
    /// Nonfinite motion, collision/step limits or overflow leave all fluid/geometry unchanged.
    pub fn step_with_translating_world(
        &mut self,
        dt: f64,
        velocity: [f64; 3],
        world: &impl crate::CollisionWorld,
        contact: ContactConfig,
    ) -> Result<StepStats, Error> {
        self.step_with_translating_world_report(dt, velocity, world, contact)
            .map(|report| report.fluid)
    }
    /// Advances as `step_with_translating_world` and reports geometry impulse and drive work.
    /// Gravity impulse is subtracted from the net fluid momentum change. Collision dissipation
    /// is not converted to heat; drive work is not a total-energy conservation claim.
    /// # Errors
    /// Also rejects overflow in the reported transfer; all state changes remain atomic.
    pub fn step_with_translating_world_report(
        &mut self,
        dt: f64,
        velocity: [f64; 3],
        world: &impl crate::CollisionWorld,
        contact: ContactConfig,
    ) -> Result<TranslatingWorldReport, Error> {
        if !finite(velocity) {
            return Err(Error::InvalidCollision);
        }
        let mut candidate = self.clone();
        candidate.configure_boundary_velocities(vec![[0.0; 3]; candidate.boundaries.len()])?;
        for particle in &mut candidate.particles {
            particle.velocity =
                std::array::from_fn(|axis| particle.velocity[axis] - velocity[axis]);
            if !finite(particle.velocity) {
                return Err(Error::NumericalFailure);
            }
        }
        let stats = candidate.step_with_world(dt, None, world, contact)?;
        let displacement = velocity.map(|component| component * dt);
        if !finite(displacement) {
            return Err(Error::NumericalFailure);
        }
        for particle in &mut candidate.particles {
            for axis in 0..3 {
                particle.position[axis] += displacement[axis];
                particle.velocity[axis] += velocity[axis];
            }
            if !finite(particle.position) || !finite(particle.velocity) {
                return Err(Error::NumericalFailure);
            }
        }
        candidate.translate_boundaries(displacement)?;
        candidate.configure_boundary_velocities(vec![velocity; candidate.boundaries.len()])?;
        let mut impulse = [0.0; 3];
        for (before, after) in self.particles.iter().zip(&candidate.particles) {
            for (axis, component) in impulse.iter_mut().enumerate() {
                *component += before.mass
                    * (after.velocity[axis]
                        - before.velocity[axis]
                        - self.config.gravity[axis] * dt);
            }
        }
        let drive_work: f64 = impulse
            .into_iter()
            .zip(velocity)
            .map(|(momentum, speed)| momentum * speed)
            .sum();
        if !finite(impulse) || !drive_work.is_finite() {
            return Err(Error::NumericalFailure);
        }
        *self = candidate;
        Ok(TranslatingWorldReport {
            fluid: stats,
            impulse,
            drive_work,
        })
    }
}
