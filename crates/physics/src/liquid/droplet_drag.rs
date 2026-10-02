//! Exact constant-coefficient quadratic drag against a prescribed uniform gas.
use super::{Error, Liquid, finite, norm, positive, sub};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropletGas {
    pub velocity: [f64; 3],
    pub density: f64,
    /// Explicit coefficient using projected spherical area pi*r². Requires
    /// calibration for Reynolds number/shape; no universal sphere Cd is assumed.
    pub drag_coefficient: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DropletDragReport {
    pub gas_impulse: [f64; 3],
    /// Relative-motion kinetic energy converted to heat in the external gas ledger.
    pub dissipated_heat: f64,
    /// Gas velocity dotted with its impulse. Can be negative when gas accelerates
    /// droplets. Particle lab-frame KE loss equals heat plus this signed work.
    pub gas_work: f64,
}
impl Liquid {
    /// Applies only drag, holding geometry, thermal fields and mass fixed. Radii
    /// are explicit physical projected radii in particle order, not kernel radii.
    /// For constant Cd, rho and gas velocity, relative velocity evolves exactly as
    /// `w_new=w/(1+rho*Cd*pi*r²*|w|*dt/(2*m))`. Advection/gravity remain separate.
    ///
    /// The prescribed gas receives reported impulse, heat and work; its finite
    /// dynamics are not solved. Dense-cloud shielding, lift, deformation and a
    /// Reynolds-dependent Cd law are absent. Use only on particles treated as drops.
    /// # Errors
    /// Invalid controls/radii/time, overflow or invalid resulting constitutive state.
    /// Atomic for the entire liquid.
    pub fn apply_droplet_drag(
        &mut self,
        dt: f64,
        gas: DropletGas,
        radii: &[f64],
    ) -> Result<DropletDragReport, Error> {
        self.apply_droplet_drag_impl(dt, gas, radii, false)
    }
    /// Drag only on explicitly marked droplets. Ordinary SPH/source samples are
    /// unchanged; a population mask is required. Gas ledgers contain marked drops only.
    pub fn apply_marked_droplet_drag(
        &mut self,
        dt: f64,
        gas: DropletGas,
        radii: &[f64],
    ) -> Result<DropletDragReport, Error> {
        if self.droplet_population.is_none() {
            return Err(Error::InvalidConfig);
        }
        self.apply_droplet_drag_impl(dt, gas, radii, true)
    }
    fn apply_droplet_drag_impl(
        &mut self,
        dt: f64,
        gas: DropletGas,
        radii: &[f64],
        marked_only: bool,
    ) -> Result<DropletDragReport, Error> {
        if !positive(dt)
            || !finite(gas.velocity)
            || !positive(gas.density)
            || !gas.drag_coefficient.is_finite()
            || gas.drag_coefficient < 0.0
            || radii.len() != self.particles.len()
            || radii.iter().any(|r| !positive(*r))
        {
            return Err(Error::InvalidConfig);
        }
        let mut candidate = self.clone();
        let mut report = DropletDragReport::default();
        for (i, (particle, radius)) in candidate.particles.iter_mut().zip(radii).enumerate() {
            if marked_only
                && !self
                    .droplet_population
                    .as_ref()
                    .ok_or(Error::InvalidConfig)?[i]
            {
                continue;
            }
            let relative = sub(particle.velocity, gas.velocity);
            let speed = norm(relative);
            let coefficient =
                0.5 * gas.density * gas.drag_coefficient * std::f64::consts::PI * radius * radius
                    / particle.mass;
            let decay = coefficient * speed * dt;
            if !speed.is_finite() || !decay.is_finite() {
                return Err(Error::NumericalFailure);
            }
            let next = relative.map(|v| v / (1.0 + decay));
            let loss = decay / (1.0 + decay);
            let heat = 0.5 * particle.mass * speed * speed * (loss * (2.0 - loss));
            for axis in 0..3 {
                let impulse = particle.mass * (relative[axis] - next[axis]);
                report.gas_impulse[axis] += impulse;
                report.gas_work += gas.velocity[axis] * impulse;
                particle.velocity[axis] = gas.velocity[axis] + next[axis];
            }
            report.dissipated_heat += heat;
            if !finite(particle.velocity) {
                return Err(Error::NumericalFailure);
            }
        }
        if !finite(report.gas_impulse)
            || !report.dissipated_heat.is_finite()
            || !report.gas_work.is_finite()
        {
            return Err(Error::NumericalFailure);
        }
        candidate.effective_materials()?;
        *self = candidate;
        Ok(report)
    }
}
