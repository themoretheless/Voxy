use super::{Error, Liquid, Material, Particle, finite, sub, transport::Transport};
/// Pair composition used by the heated viscosity stage of ordinary stepping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViscousIntegrator {
    Sequential,
    Symmetric,
    /// Symmetric pair relaxation with constitutive coefficients predicted at half time.
    Midpoint,
}
/// Integrated balance of a frozen-position viscous stage with stationary fixtures.
#[derive(Clone, Debug, PartialEq)]
pub struct ViscousRelaxationStats {
    pub neighbor_pairs: usize,
    pub kinetic_energy_loss: f64,
    pub fixture_impulse: [f64; 3],
    pub fixture_angular_impulse_about_origin: [f64; 3],
}
impl Liquid {
    #[must_use]
    pub fn viscous_integrator(&self) -> ViscousIntegrator {
        self.viscous_integrator
    }
    /// Selects the pair composition when viscous heating is enabled.
    /// This does not change the explicit unheated operator or the order of the full step.
    /// # Errors
    /// Non-sequential modes require no sampled walls or finite body coupling.
    pub fn set_viscous_integrator(&mut self, integrator: ViscousIntegrator) -> Result<(), Error> {
        if integrator != ViscousIntegrator::Sequential
            && (!self.boundaries.is_empty() || self.boundary_coupling.is_some())
        {
            return Err(Error::InvalidBoundary);
        }
        self.viscous_integrator = integrator;
        Ok(())
    }

    /// Enables exact viscous pair relaxation and conversion of lost kinetic energy to heat.
    /// Pressure, surface and collision energy are not included in this conversion.
    /// # Errors
    /// Enabling requires configured thermal fields. Failure leaves state unchanged.
    pub fn set_viscous_heating(&mut self, enabled: bool) -> Result<(), Error> {
        if (!enabled && self.gas_equations.iter().any(Option::is_some))
            || (enabled && self.transport.is_none())
        {
            return Err(Error::InvalidTransport);
        }
        self.viscous_heating = enabled;
        Ok(())
    }
    /// Applies only viscosity and its heating, holding positions fixed.
    /// Each pair is solved exactly at frozen properties; the composed stage is
    /// first order in time, not an exact global viscous solve. No pressure,
    /// gravity, advection, conduction, diffusion or collision is applied.
    /// The usual step already includes viscosity: do not apply both for the same interval.
    /// # Errors
    /// Requires positive finite dt and thermal fields. Moving sampled fixtures and
    /// finite-body coupling are unsupported. All errors preserve the entire liquid.
    pub fn relax_viscosity(&mut self, dt: f64) -> Result<ViscousRelaxationStats, Error> {
        self.relax_viscous_stage(dt, false, false)
    }
    /// Symmetric forward/backward pair composition at frozen geometry/properties.
    /// Second order applies to the frozen-coefficient stage, not temperature-dependent
    /// coefficients or the full fluid evolution. Heat and fixture impulses are recorded.
    /// # Errors
    /// Same requirements as `relax_viscosity`; sampled walls are currently unsupported.
    pub fn relax_viscosity_symmetric(&mut self, dt: f64) -> Result<ViscousRelaxationStats, Error> {
        if !self.boundaries.is_empty() {
            return Err(Error::InvalidBoundary);
        }
        self.relax_viscous_stage(dt, true, false)
    }
    /// Predicts a half-stage, then uses its constitutive coefficients in a full
    /// symmetric stage from the original state. Geometry stays fixed; predictor
    /// heating is discarded. Actual kinetic loss supplies the committed heat.
    /// Smooth constitutive laws admit second-order local time accuracy; floors,
    /// caps and phase transitions can reduce it. This does not advance full flow.
    /// # Errors
    /// Positive finite dt, thermal fields and no sampled walls/finite body are required.
    /// Failure in either predictor or final stage preserves the entire liquid.
    pub fn relax_viscosity_midpoint(&mut self, dt: f64) -> Result<ViscousRelaxationStats, Error> {
        if !self.boundaries.is_empty() {
            return Err(Error::InvalidBoundary);
        }
        self.relax_viscous_stage(dt, true, true)
    }
    fn relax_viscous_stage(
        &mut self,
        dt: f64,
        symmetric: bool,
        midpoint: bool,
    ) -> Result<ViscousRelaxationStats, Error> {
        if self.thixotropy.iter().all(Option::is_none) {
            return self.relax_viscous_stage_inner(dt, symmetric, midpoint);
        }
        let mut candidate = self.clone();
        candidate.advance_structure(0.5 * dt)?;
        let result = candidate.relax_viscous_stage_inner(dt, symmetric, midpoint)?;
        candidate.advance_structure(0.5 * dt)?;
        *self = candidate;
        Ok(result)
    }
    fn relax_viscous_stage_inner(
        &mut self,
        dt: f64,
        symmetric: bool,
        midpoint: bool,
    ) -> Result<ViscousRelaxationStats, Error> {
        if !super::positive(dt) {
            return Err(Error::InvalidTimeStep);
        }
        if self.boundary_coupling.is_some()
            || self
                .boundary_velocities
                .iter()
                .any(|v| v.iter().any(|x| *x != 0.0))
        {
            return Err(Error::InvalidBoundary);
        }
        let mut fields = self.transport.clone().ok_or(Error::InvalidTransport)?;
        let mut next = self.particles.clone();
        let properties = if midpoint {
            let mut predictor = self.clone();
            predictor.relax_viscous_stage_inner(0.5 * dt, true, false)?;
            predictor.effective_materials()?
        } else {
            self.effective_materials()?
        };
        let pairs = super::pairs(
            &next,
            self.config.smoothing_radius,
            self.config.max_pairs,
            self.config.max_neighbor_checks,
        )?;
        let density = self.forces(&next, &pairs, &properties)?.0;
        if symmetric {
            self.dissipate_symmetric_viscosity(
                &mut next,
                &pairs,
                &properties,
                &density,
                &mut fields,
                dt,
            )?;
        } else {
            self.dissipate_viscosity(&mut next, &pairs, &properties, &density, &mut fields, dt)?;
        }
        self.evaluate_materials(&next, Some(&fields))?;
        let mut stats = ViscousRelaxationStats {
            neighbor_pairs: pairs.len(),
            kinetic_energy_loss: 0.0,
            fixture_impulse: [0.0; 3],
            fixture_angular_impulse_about_origin: [0.0; 3],
        };
        for (before, after) in self.particles.iter().zip(&next) {
            let impulse: [f64; 3] =
                std::array::from_fn(|a| before.mass * (before.velocity[a] - after.velocity[a]));
            stats.kinetic_energy_loss += 0.5
                * before.mass
                * (before.velocity.iter().map(|v| v * v).sum::<f64>()
                    - after.velocity.iter().map(|v| v * v).sum::<f64>());
            for axis in 0..3 {
                let b = (axis + 1) % 3;
                let c = (axis + 2) % 3;
                stats.fixture_impulse[axis] += impulse[axis];
                stats.fixture_angular_impulse_about_origin[axis] +=
                    before.position[b] * impulse[c] - before.position[c] * impulse[b];
            }
        }
        if !stats.kinetic_energy_loss.is_finite()
            || !finite(stats.fixture_impulse)
            || !finite(stats.fixture_angular_impulse_about_origin)
        {
            return Err(Error::NumericalFailure);
        }
        self.particles = next;
        self.transport = Some(fields);
        Ok(stats)
    }
    pub(super) fn dissipate_symmetric_viscosity(
        &self,
        particles: &mut [Particle],
        pairs: &[(usize, usize)],
        properties: &[Material],
        density: &[f64],
        fields: &mut Transport,
        dt: f64,
    ) -> Result<(), Error> {
        if !self.boundaries.is_empty() || self.boundary_coupling.is_some() {
            return Err(Error::InvalidBoundary);
        }
        let mut images = self.image_pairs(particles, pairs.len())?;
        self.dissipate_real_viscosity(particles, pairs, properties, density, fields, 0.5 * dt)?;
        self.dissipate_image_pairs(particles, properties, density, &images, fields, 0.5 * dt)?;
        images.reverse();
        self.dissipate_image_pairs(particles, properties, density, &images, fields, 0.5 * dt)?;
        let reverse: Vec<_> = pairs.iter().copied().rev().collect();
        self.dissipate_real_viscosity(particles, &reverse, properties, density, fields, 0.5 * dt)?;
        Ok(())
    }
    pub(super) fn dissipate_viscosity(
        &self,
        particles: &mut [Particle],
        pairs: &[(usize, usize)],
        properties: &[Material],
        density: &[f64],
        fields: &mut Transport,
        dt: f64,
    ) -> Result<(), Error> {
        self.dissipate_real_viscosity(particles, pairs, properties, density, fields, dt)?;
        if self.boundary_coupling.is_none() {
            self.dissipate_boundary_viscosity(
                particles,
                pairs.len(),
                properties,
                density,
                fields,
                dt,
            )?;
        }
        self.dissipate_image_viscosity(particles, properties, density, pairs.len(), fields, dt)?;
        Ok(())
    }
    fn dissipate_real_viscosity(
        &self,
        particles: &mut [Particle],
        pairs: &[(usize, usize)],
        properties: &[Material],
        density: &[f64],
        fields: &mut Transport,
        dt: f64,
    ) -> Result<(), Error> {
        let support = self.config.smoothing_radius;
        for &(i, j) in pairs {
            let first = particles[i];
            let second = particles[j];
            let displacement = sub(first.position, second.position);
            let separation = super::norm(displacement);
            if separation <= 1e-12 {
                continue;
            }
            let direction = displacement.map(|r| r / separation);
            let viscosity = 0.5 * (properties[i].viscosity + properties[j].viscosity);
            let conductance = super::viscous_conductance(
                [first.mass, second.mass],
                viscosity,
                [density[i], density[j]],
                separation,
                support,
            );
            let reduced_mass = 1.0 / (1.0 / first.mass + 1.0 / second.mass);
            let fraction = -(-conductance / reduced_mass * dt).exp_m1();
            let normal_speed: f64 = sub(first.velocity, second.velocity)
                .iter()
                .zip(direction)
                .map(|(v, n)| v * n)
                .sum();
            let relative = direction.map(|n| n * normal_speed);
            let squared_speed = normal_speed.powi(2);
            let heat = 0.5 * reduced_mass * squared_speed * fraction * (2.0 - fraction);
            if !conductance.is_finite() || conductance < 0.0 || !heat.is_finite() {
                return Err(Error::NumericalFailure);
            }
            for (axis, speed) in relative.into_iter().enumerate() {
                let impulse = reduced_mass * fraction * speed;
                particles[i].velocity[axis] -= impulse / first.mass;
                particles[j].velocity[axis] += impulse / second.mass;
            }
            if !finite(particles[i].velocity) || !finite(particles[j].velocity) {
                return Err(Error::NumericalFailure);
            }
            // Symmetric deposition; the capacities determine temperature rises independently.
            for index in [i, j] {
                let energy = fields.energy(&particles[index], &fields.fields[index], index)?;
                fields.set_energy(index, &particles[index], energy + 0.5 * heat)?;
            }
        }
        Ok(())
    }
}
