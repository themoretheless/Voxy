use super::{Error, Liquid, finite, positive};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ForcingReport {
    pub impulse: [f64; 3],
    /// Signed change in kinetic energy, equal to work of this discrete impulse.
    pub work: f64,
}
impl Liquid {
    /// Applies one external impulse to each particle without advancing time or position.
    /// Mechanical work is reported; thermal fields are unchanged. The caller supplies
    /// impulses in mass*velocity units, including mass when applying acceleration.
    /// # Errors
    /// Wrong count, nonfinite impulses or overflowing velocities/work. Failure is atomic.
    pub fn apply_impulses(&mut self, impulses: &[[f64; 3]]) -> Result<ForcingReport, Error> {
        if impulses.len() != self.particles.len() || impulses.iter().any(|value| !finite(*value)) {
            return Err(Error::InvalidParticle);
        }
        let mut velocities = Vec::with_capacity(self.particles.len());
        let mut report = ForcingReport::default();
        for (particle, impulse) in self.particles.iter().zip(impulses) {
            let mut velocity = particle.velocity;
            for axis in 0..3 {
                let delta = impulse[axis] / particle.mass;
                velocity[axis] += delta;
                report.impulse[axis] += impulse[axis];
                report.work += impulse[axis] * (particle.velocity[axis] + 0.5 * delta);
            }
            if !finite(velocity) || !finite(report.impulse) || !report.work.is_finite() {
                return Err(Error::NumericalFailure);
            }
            velocities.push(velocity);
        }
        for (particle, velocity) in self.particles.iter_mut().zip(velocities) {
            particle.velocity = velocity;
        }
        Ok(report)
    }
    /// Applies force*time as an external velocity kick; does not advance the fluid.
    /// Pair this with `step` or `step_with_world` to integrate positions and internal forces.
    /// # Errors
    /// Nonpositive/nonfinite duration, wrong force count, nonfinite forces or numerical overflow.
    pub fn apply_forces(
        &mut self,
        duration: f64,
        forces: &[[f64; 3]],
    ) -> Result<ForcingReport, Error> {
        if !positive(duration) {
            return Err(Error::InvalidTimeStep);
        }
        if forces.len() != self.particles.len() || forces.iter().any(|value| !finite(*value)) {
            return Err(Error::InvalidParticle);
        }
        let impulses: Vec<_> = forces
            .iter()
            .map(|force| force.map(|component| component * duration))
            .collect();
        if impulses.iter().any(|value| !finite(*value)) {
            return Err(Error::NumericalFailure);
        }
        self.apply_impulses(&impulses)
    }
}
