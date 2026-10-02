use super::{Error, Liquid, Particle, norm, pairs, sub};
use std::f64::consts::PI;
impl Liquid {
    /// Sets a symmetric compact overlap penalty between two different materials.
    /// Zero disables it. This numerical coefficient is not calibrated SI tension.
    /// It does not exchange particle material IDs, mass or solute composition.
    /// # Errors
    /// Unknown/equal material indices, negative or nonfinite coefficient; no mutation.
    pub fn set_interface_penalty(
        &mut self,
        first: usize,
        second: usize,
        stiffness: f64,
    ) -> Result<(), Error> {
        if first >= self.materials.len()
            || second >= self.materials.len()
            || first == second
            || !stiffness.is_finite()
            || stiffness < 0.0
        {
            return Err(Error::InvalidSurfaceStrength);
        }
        let key = (first.min(second), first.max(second));
        if stiffness > 0.0 {
            self.interface_penalties.insert(key, stiffness);
        } else {
            self.interface_penalties.remove(&key);
        }
        Ok(())
    }
    fn interface_coefficient(&self, a: Particle, b: Particle) -> f64 {
        let strength = self
            .interface_penalties
            .get(&(a.material.min(b.material), a.material.max(b.material)))
            .copied()
            .unwrap_or(0.0);
        if strength == 0.0 {
            return 0.0;
        }
        strength * a.mass * b.mass * 45.0 / (PI * self.config.smoothing_radius.powi(7))
    }

    pub(super) fn interface_force(&self, a: Particle, b: Particle, radius: f64) -> f64 {
        self.interface_coefficient(a, b)
            * radius
            * (self.config.smoothing_radius - radius).max(0.0).powi(2)
    }
    /// Potential energy of the configured overlap penalties, independent of density.
    /// Pressure, cohesion and thermal energy are not included.
    /// # Errors
    /// Neighbor budget excess or numerical overflow. The query is read-only.
    pub fn interface_energy(&self) -> Result<f64, Error> {
        let neighbors = pairs(
            &self.particles,
            self.config.smoothing_radius,
            self.config.max_pairs,
            self.config.max_neighbor_checks,
        )?;
        let mut energy = 0.0;
        let support = self.config.smoothing_radius;
        for (i, j) in neighbors {
            let a = self.particles[i];
            let b = self.particles[j];
            let gap = (support - norm(sub(a.position, b.position))).max(0.0);
            energy += self.interface_coefficient(a, b) * gap.powi(3) * (support / 3.0 - gap / 4.0);
        }
        if !energy.is_finite() || energy < 0.0 {
            return Err(Error::NumericalFailure);
        }
        Ok(energy)
    }
}
