//! Symmetric free-flight operator splitting with midpoint nonlinear viscous relaxation.
use super::{ContactConfig, Error, Liquid, StepStats, finite, move_with_world, pairs, positive};
/// Fixture collision exchange over both half drifts and all substeps.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImpactExchange {
    /// Kinetic loss measured relative to the fixture.
    pub dissipated_energy: f64,
    /// Work supplied to the liquid by prescribed fixture translation.
    pub drive_work: f64,
    pub fluid_heat: f64,
    /// Assigned to the external fixture ledger, not a finite wall temperature.
    pub fixture_energy: f64,
    pub fixture_impulse: [f64; 3],
    pub fixture_angular_impulse_about_origin: [f64; 3],
}
impl Liquid {
    /// Symmetric drift/thermal/kick/viscosity/kick/thermal/drift splitting.
    /// Requires heated viscosity. No collision geometry, sampled walls or finite bodies.
    /// Static reflecting images are supported, but crossing their bounds fails atomically.
    /// Smooth, uncoupled fixed-coefficient thermal and mechanical operators permit
    /// second-order splitting; constitutive feedback/phase transitions need separate checks.
    /// This is not a general second-order EOS or collision integrator.
    /// # Errors
    /// Invalid time, missing heated transport, unsupported boundaries, exhausted budgets
    /// or invalid intermediate state. Failure preserves the complete liquid.
    pub fn step_symmetric_free(&mut self, dt: f64) -> Result<StepStats, Error> {
        self.advance_symmetric_with_mover(dt, |state, time| {
            for particle in &mut state.particles {
                for axis in 0..3 {
                    particle.position[axis] += time * particle.velocity[axis];
                }
            }
            Ok(())
        })
    }
    /// Symmetric fluid splitting with static-world CCD during both half drifts.
    /// Collision limits apply separately per particle and half drift. Impact energy
    /// is not deposited into heat; nonsmooth contacts need independent accuracy checks.
    /// # Errors
    /// Same constitutive requirements as `step_symmetric_free`, plus valid contact
    /// settings, valid backend sweeps and no initial overlap. All failures roll back.
    pub fn step_symmetric_with_world(
        &mut self,
        dt: f64,
        world: &impl crate::CollisionWorld,
        contact: ContactConfig,
    ) -> Result<StepStats, Error> {
        contact.validate()?;
        let radius = self.config.particle_radius;
        self.advance_symmetric_with_mover(dt, |state, time| {
            for particle in &mut state.particles {
                move_with_world(particle, time, radius, world, contact)?;
            }
            Ok(())
        })
    }
    /// Static-world CCD with an explicit partition of lost kinetic energy.
    /// `fluid_heat_fraction` is in [0,1]; the remainder belongs to the fixture ledger.
    /// # Errors
    /// Invalid heat/contact settings, backend failures, constitutive heating domain
    /// failures or overflow. The whole fluid state is rolled back on every error.
    pub fn step_symmetric_with_world_heating(
        &mut self,
        dt: f64,
        world: &impl crate::CollisionWorld,
        contact: ContactConfig,
        fluid_heat_fraction: f64,
    ) -> Result<(StepStats, ImpactExchange), Error> {
        self.symmetric_impact_in_frame(dt, world, contact, fluid_heat_fraction, [0.0; 3])
    }
    /// Prescribed constant-velocity collision world with relative-frame impact heating.
    /// The backend describes geometry at the start of the call; the owner must move
    /// it before the next call. Returned fluid coordinates and fixture torque use
    /// the laboratory frame. No wall recoil, acceleration or rotation is solved.
    /// # Errors
    /// Nonfinite frame velocity, reflecting/sampled walls or finite bodies, collision
    /// failures and constitutive overflow. All fluid changes are atomic.
    pub fn step_symmetric_with_translating_world_heating(
        &mut self,
        dt: f64,
        velocity: [f64; 3],
        world: &impl crate::CollisionWorld,
        contact: ContactConfig,
        fluid_heat_fraction: f64,
    ) -> Result<(StepStats, ImpactExchange), Error> {
        if !finite(velocity) {
            return Err(Error::InvalidCollision);
        }
        if self.reflecting_box.is_some() {
            return Err(Error::InvalidBoundary);
        }
        let mut candidate = self.clone();
        for particle in &mut candidate.particles {
            for (v, wall) in particle.velocity.iter_mut().zip(velocity) {
                *v -= wall;
            }
            if !finite(particle.velocity) {
                return Err(Error::NumericalFailure);
            }
        }
        let report = candidate.symmetric_impact_in_frame(
            dt,
            world,
            contact,
            fluid_heat_fraction,
            velocity,
        )?;
        for particle in &mut candidate.particles {
            for (axis, speed) in velocity.iter().enumerate() {
                particle.position[axis] += speed * dt;
                particle.velocity[axis] += speed;
            }
            if !finite(particle.position) || !finite(particle.velocity) {
                return Err(Error::NumericalFailure);
            }
        }
        *self = candidate;
        Ok(report)
    }
    fn symmetric_impact_in_frame(
        &mut self,
        dt: f64,
        world: &impl crate::CollisionWorld,
        contact: ContactConfig,
        fluid_heat_fraction: f64,
        frame_velocity: [f64; 3],
    ) -> Result<(StepStats, ImpactExchange), Error> {
        contact.validate()?;
        if !(0.0..=1.0).contains(&fluid_heat_fraction) {
            return Err(Error::InvalidTransport);
        }
        let radius = self.config.particle_radius;
        let mut exchange = ImpactExchange::default();
        let mut elapsed = 0.0;
        let stats = self.advance_symmetric_with_mover(dt, |state, time| {
            let mut heat = vec![0.0; state.particles.len()];
            for (particle, energy) in state.particles.iter_mut().zip(&mut heat) {
                let before = *particle;
                move_with_world(particle, time, radius, world, contact)?;
                let kinetic = |p: super::Particle| {
                    0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>()
                };
                let initial = kinetic(before);
                let final_energy = kinetic(*particle);
                let loss = initial - final_energy;
                if !loss.is_finite() || loss < -16.0 * f64::EPSILON * initial.max(final_energy) {
                    return Err(Error::NumericalFailure);
                }
                let loss = loss.max(0.0);
                *energy = fluid_heat_fraction * loss;
                exchange.dissipated_energy += loss;
                exchange.fluid_heat += *energy;
                exchange.fixture_energy += (1.0 - fluid_heat_fraction) * loss;
                for axis in 0..3 {
                    exchange.fixture_impulse[axis] +=
                        particle.mass * (before.velocity[axis] - particle.velocity[axis]);
                    let b = (axis + 1) % 3;
                    let c = (axis + 2) % 3;
                    let initial_position = before.position;
                    let before_b = initial_position[b] + frame_velocity[b] * elapsed;
                    let before_c = initial_position[c] + frame_velocity[c] * elapsed;
                    let after_b = particle.position[b] + frame_velocity[b] * (elapsed + time);
                    let after_c = particle.position[c] + frame_velocity[c] * (elapsed + time);
                    exchange.fixture_angular_impulse_about_origin[axis] += particle.mass
                        * (before_b * (before.velocity[c] + frame_velocity[c])
                            - before_c * (before.velocity[b] + frame_velocity[b])
                            - after_b * (particle.velocity[c] + frame_velocity[c])
                            + after_c * (particle.velocity[b] + frame_velocity[b]));
                }
            }
            exchange.drive_work = -exchange
                .fixture_impulse
                .iter()
                .zip(frame_velocity)
                .map(|(p, v)| p * v)
                .sum::<f64>();
            elapsed += time;
            if !exchange.drive_work.is_finite()
                || !elapsed.is_finite()
                || !exchange.dissipated_energy.is_finite()
                || !exchange.fluid_heat.is_finite()
                || !exchange.fixture_energy.is_finite()
                || !finite(exchange.fixture_impulse)
                || !finite(exchange.fixture_angular_impulse_about_origin)
            {
                return Err(Error::NumericalFailure);
            }
            state.add_heat(&heat)
        })?;
        Ok((stats, exchange))
    }
    pub(super) fn advance_symmetric_with_mover(
        &mut self,
        dt: f64,
        mut mover: impl FnMut(&mut Self, f64) -> Result<(), Error>,
    ) -> Result<StepStats, Error> {
        if !positive(dt) || dt > 0.1 {
            return Err(Error::InvalidTimeStep);
        }
        if !self.viscous_heating
            || self.transport.is_none()
            || self.maxwell_fluids.iter().any(Option::is_some)
        {
            return Err(Error::InvalidTransport);
        }
        if !self.boundaries.is_empty() || self.boundary_coupling.is_some() {
            return Err(Error::InvalidBoundary);
        }
        let mut candidate = self.clone();
        let mut remaining = dt;
        let mut stats = StepStats {
            substeps: 0,
            neighbor_pairs: 0,
            max_density_ratio: 0.0,
        };
        while remaining > 0.0 && !candidate.particles.is_empty() {
            if stats.substeps >= candidate.config.max_substeps {
                return Err(Error::SubstepBudget);
            }
            let neighbors = candidate.split_pairs()?;
            let properties = candidate.effective_materials()?;
            let (density, acceleration, _) =
                candidate.forces(&candidate.particles, &neighbors, &properties)?;
            let mut step = remaining;
            for (i, particle) in candidate.particles.iter().enumerate() {
                step = step.min(candidate.particle_time_limit(
                    particle,
                    properties[i],
                    acceleration[i],
                    None,
                    0.0,
                ));
                stats.max_density_ratio = stats
                    .max_density_ratio
                    .max(density[i] / properties[i].rest_density);
            }
            if !positive(step) {
                return Err(Error::NumericalFailure);
            }
            mover(&mut candidate, 0.5 * step)?;
            candidate.validate_split_positions()?;
            candidate.split_thermal(0.5 * step, false)?;
            candidate.split_kick(0.5 * step)?;
            let viscous = candidate.relax_viscosity_midpoint(step)?;
            candidate.split_kick(0.5 * step)?;
            candidate.split_thermal(0.5 * step, true)?;
            mover(&mut candidate, 0.5 * step)?;
            candidate.validate_split_positions()?;
            candidate.effective_materials()?;
            stats.neighbor_pairs = stats
                .neighbor_pairs
                .max(neighbors.len())
                .max(viscous.neighbor_pairs);
            stats.substeps += 1;
            let rest = (remaining - step).max(0.0);
            if rest >= remaining {
                return Err(Error::NumericalFailure);
            }
            remaining = rest;
        }
        *self = candidate;
        Ok(stats)
    }
    fn split_pairs(&self) -> Result<Vec<(usize, usize)>, Error> {
        pairs(
            &self.particles,
            self.config.smoothing_radius,
            self.config.max_pairs,
            self.config.max_neighbor_checks,
        )
    }
    fn validate_split_positions(&self) -> Result<(), Error> {
        if self
            .particles
            .iter()
            .any(|p| !finite(p.position) || !finite(p.velocity))
        {
            return Err(Error::NumericalFailure);
        }
        self.validate_reflecting_positions(&self.particles)
    }
    fn split_thermal(&mut self, dt: f64, reverse: bool) -> Result<(), Error> {
        let mut neighbors = self.split_pairs()?;
        if reverse {
            neighbors.reverse();
        }
        let properties = self.effective_materials()?;
        let mut fields = self.transport.clone().ok_or(Error::InvalidTransport)?;
        fields.advance(
            &self.particles,
            &neighbors,
            &properties,
            self.config.smoothing_radius,
            dt,
        )?;
        self.evaluate_materials(&self.particles, Some(&fields))?;
        self.transport = Some(fields);
        Ok(())
    }
    fn split_kick(&mut self, dt: f64) -> Result<(), Error> {
        if self.gas_active() {
            return self.split_gas_kick(dt);
        }
        let neighbors = self.split_pairs()?;
        let properties = self.effective_materials()?;
        let (density, mut acceleration, _) =
            self.forces(&self.particles, &neighbors, &properties)?;
        let mut particles = self.particles.clone();
        let mut fields = self.transport.clone().ok_or(Error::InvalidTransport)?;
        if self.pressure_work {
            let pressure = self.exchange_pressure_work(
                &mut particles,
                &neighbors,
                &properties,
                &density,
                &mut fields,
                dt,
            )?;
            for (a, p) in acceleration.iter_mut().zip(pressure) {
                for axis in 0..3 {
                    a[axis] -= p[axis];
                }
            }
        }
        for (particle, a) in particles.iter_mut().zip(acceleration) {
            for (v, force) in particle.velocity.iter_mut().zip(a) {
                *v += dt * force;
            }
            if !finite(particle.velocity) {
                return Err(Error::NumericalFailure);
            }
        }
        self.evaluate_materials(&particles, Some(&fields))?;
        self.particles = particles;
        self.transport = Some(fields);
        Ok(())
    }
    fn split_gas_kick(&mut self, dt: f64) -> Result<(), Error> {
        let neighbors = self.split_pairs()?;
        let mut particles = self.particles.clone();
        let mut fields = self.transport.clone().ok_or(Error::InvalidTransport)?;
        // External acceleration changes velocity before pressure performs work.
        // Symmetric half kicks prevent its cross-work from entering internal energy.
        self.gas_external_kick(&mut particles, &fields, &neighbors, 0.5 * dt)?;
        let properties = self.evaluate_materials(&particles, Some(&fields))?;
        let density = self.forces(&particles, &neighbors, &properties)?.0;
        let mut predicted_particles = particles.clone();
        let mut predicted_fields = fields.clone();
        self.exchange_gas_pressure_work(
            &mut predicted_particles,
            &neighbors,
            &properties,
            &density,
            &mut predicted_fields,
            0.5 * dt,
        )?;
        let midpoint = self.evaluate_materials(&predicted_particles, Some(&predicted_fields))?;
        self.exchange_gas_pressure_work(
            &mut particles,
            &neighbors,
            &midpoint,
            &density,
            &mut fields,
            dt,
        )?;
        self.gas_external_kick(&mut particles, &fields, &neighbors, 0.5 * dt)?;
        self.evaluate_materials(&particles, Some(&fields))?;
        self.particles = particles;
        self.transport = Some(fields);
        Ok(())
    }
    fn gas_external_kick(
        &self,
        particles: &mut [super::Particle],
        fields: &super::transport::Transport,
        neighbors: &[(usize, usize)],
        dt: f64,
    ) -> Result<(), Error> {
        let properties = self.evaluate_materials(particles, Some(fields))?;
        let (density, total, _) = self.forces(particles, neighbors, &properties)?;
        let pressure =
            self.gas_pressure_accelerations(particles, neighbors, &properties, &density)?;
        for ((particle, acceleration), pressure) in particles.iter_mut().zip(total).zip(pressure) {
            for ((velocity, total), pressure) in
                particle.velocity.iter_mut().zip(acceleration).zip(pressure)
            {
                *velocity += dt * (total - pressure);
            }
            if !finite(particle.velocity) {
                return Err(Error::NumericalFailure);
            }
        }
        Ok(())
    }
}
