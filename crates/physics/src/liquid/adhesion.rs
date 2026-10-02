use super::{Container, Error, Liquid, Particle, finite, positive};

/// Numerical attraction to the six static container walls.
/// This is a wall potential, not a calibrated contact-angle law.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallAdhesion {
    /// Maximum inward acceleration toward each wall, in m/s².
    pub acceleration: f64,
    /// Range measured from the particle's collision surface, in metres.
    pub range: f64,
}

impl Liquid {
    /// Sets a material's container-wall attraction; `None` disables it.
    /// # Errors
    /// Invalid material, nonfinite/negative strength or nonpositive range.
    /// Configuration failure leaves the simulation unchanged.
    pub fn set_wall_adhesion(
        &mut self,
        material: usize,
        adhesion: Option<WallAdhesion>,
    ) -> Result<(), Error> {
        if material >= self.materials.len()
            || adhesion.is_some_and(|value| {
                !positive(value.range)
                    || !value.acceleration.is_finite()
                    || value.acceleration < 0.0
            })
        {
            return Err(Error::InvalidWallAdhesion);
        }
        self.wall_adhesion[material] = adhesion;
        Ok(())
    }

    pub(super) fn apply_wall_adhesion(
        &self,
        particles: &[Particle],
        walls: Container,
        accelerations: &mut [[f64; 3]],
    ) -> Result<(), Error> {
        for (particle, acceleration) in particles.iter().zip(accelerations) {
            let Some(adhesion) = self.wall_adhesion[particle.material] else {
                continue;
            };
            for (axis, component) in acceleration.iter_mut().enumerate() {
                let lower_gap =
                    particle.position[axis] - walls.min[axis] - self.config.particle_radius;
                let upper_gap =
                    walls.max[axis] - self.config.particle_radius - particle.position[axis];
                // Compact, continuous force with zero slope at the support boundary.
                let weight = |gap: f64| (1.0 - gap.max(0.0) / adhesion.range).max(0.0).powi(2);
                *component += adhesion.acceleration * (weight(upper_gap) - weight(lower_gap));
            }
            if !finite(*acceleration) {
                return Err(Error::NumericalFailure);
            }
        }
        Ok(())
    }
}
