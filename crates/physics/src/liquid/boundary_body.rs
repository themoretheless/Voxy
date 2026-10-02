use super::{
    Error, Liquid, Material, Particle, StepStats, TranslatingBody, finite, norm, positive,
    transport::Transport,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundaryBodyReport {
    pub fluid: StepStats,
    /// Integrated pressure impulse received by the body.
    pub pressure_impulse: [f64; 3],
    /// Integrated viscous impulse received by the body.
    pub viscous_impulse: [f64; 3],
    /// Relative kinetic energy converted into fluid heat (only if enabled).
    pub viscous_heat: f64,
    /// Kinetic work of pressure on fluid plus body (only if enabled).
    pub pressure_work: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Coupling {
    pub body: TranslatingBody,
    pub(super) pressure_impulse: [f64; 3],
    pub(super) viscous_impulse: [f64; 3],
    pub(super) viscous_heat: f64,
    pub(super) pressure_work: f64,
    pub(super) rotation: Option<super::rotation::Rotation>,
}
pub(super) struct BoundaryForces {
    pub(super) pressure: Vec<[f64; 3]>,
    pub(super) rates: Vec<f64>,
}
impl Liquid {
    /// Couples all configured SPH volume samples to one finite-mass translating body.
    /// Sample positions are lab coordinates at the start of the call, and follow the
    /// returned body displacement. Surface velocities are set to body velocity.
    /// Pressure and viscosity transfer equal opposite impulses each adaptive substep.
    /// When enabled, pressure work and viscous heating include body kinetic energy.
    /// There is no rotational degree of freedom or hard collision geometry in this method.
    /// # Errors
    /// Invalid body, budgets or numerical/thermal failure; fluid, samples and body
    /// remain unchanged together. Samples need to fill the solid's kernel support.
    pub fn step_with_boundary_body(
        &mut self,
        dt: f64,
        body: &mut TranslatingBody,
    ) -> Result<BoundaryBodyReport, Error> {
        let mut candidate = self.prepare_boundary_body(*body)?;
        let fluid = candidate.advance(dt, None, |particles, step| {
            for p in particles {
                for a in 0..3 {
                    p.position[a] += p.velocity[a] * step;
                }
            }
            Ok(())
        })?;
        let report = candidate.finish_boundary_body(fluid, body)?;
        *self = candidate;
        Ok(report)
    }
    pub(super) fn prepare_boundary_body(&self, body: TranslatingBody) -> Result<Self, Error> {
        if self.viscous_integrator != super::ViscousIntegrator::Sequential
            || self.reflecting_box.is_some()
            || self.static_pressure_extrapolation
            || !finite(body.position)
            || !finite(body.velocity)
            || !positive(body.mass)
        {
            return Err(Error::InvalidBoundary);
        }
        let mut candidate = self.clone();
        candidate.boundary_coupling = Some(Coupling {
            body,
            rotation: None,
            pressure_impulse: [0.0; 3],
            viscous_impulse: [0.0; 3],
            viscous_heat: 0.0,
            pressure_work: 0.0,
        });
        candidate.configure_boundary_velocities(vec![body.velocity; candidate.boundaries.len()])?;
        Ok(candidate)
    }
    pub(super) fn finish_boundary_body(
        &mut self,
        fluid: StepStats,
        body: &mut TranslatingBody,
    ) -> Result<BoundaryBodyReport, Error> {
        let coupling = self
            .boundary_coupling
            .take()
            .ok_or(Error::InvalidBoundary)?;
        *body = coupling.body;
        Ok(BoundaryBodyReport {
            fluid,
            pressure_impulse: coupling.pressure_impulse,
            viscous_impulse: coupling.viscous_impulse,
            viscous_heat: coupling.viscous_heat,
            pressure_work: coupling.pressure_work,
        })
    }
    /// Combines attached SPH volume samples and swept collision template in each substep.
    /// Template coordinates are relative to `body.position`; samples start in lab coordinates.
    /// # Errors
    /// Same limits as the boundary-body and dynamic-world methods. All fluid fields,
    /// sample geometry and body state roll back together on failure.
    pub fn step_with_boundary_world(
        &mut self,
        dt: f64,
        body: &mut TranslatingBody,
        world: &impl crate::CollisionWorld,
        config: super::DynamicWorldConfig,
    ) -> Result<BoundaryWorldReport, Error> {
        config.contact.validate()?;
        if config.max_contacts == 0 || config.max_queries == 0 {
            return Err(Error::InvalidCollision);
        }
        let mut candidate = self.prepare_boundary_body(*body)?;
        let radius = self.config.particle_radius;
        let mut ledger = super::dynamic_world::ContactLedger::default();
        let fluid = candidate.advance_with_mover(dt, None, |state, particles, time| {
            let coupling = state
                .boundary_coupling
                .as_mut()
                .ok_or(Error::InvalidBoundary)?;
            let initial = coupling.body.position;
            super::dynamic_world::sweep_particles(
                particles,
                &mut coupling.body,
                radius,
                time,
                world,
                config,
                &mut ledger,
            )?;
            let displacement = std::array::from_fn(|a| coupling.body.position[a] - initial[a]);
            state.follow_boundary_body(displacement)
        })?;
        let boundary = candidate.finish_boundary_body(fluid, body)?;
        *self = candidate;
        Ok(BoundaryWorldReport {
            boundary,
            contacts: ledger.contacts,
            queries: ledger.queries,
            dissipated_energy: ledger.loss,
        })
    }
    pub(super) fn drift_boundary_body(&mut self, dt: f64) -> Result<(), Error> {
        let coupling = self
            .boundary_coupling
            .as_mut()
            .ok_or(Error::InvalidBoundary)?;
        let displacement = coupling.body.velocity.map(|v| v * dt);
        for (a, delta) in displacement.into_iter().enumerate() {
            coupling.body.position[a] += delta;
        }
        if !finite(coupling.body.position) {
            return Err(Error::NumericalFailure);
        }
        if coupling.rotation.is_some() {
            self.rotate_boundary_samples(displacement, dt)
        } else {
            self.follow_boundary_body(displacement)
        }
    }
    pub(super) fn follow_boundary_body(&mut self, displacement: [f64; 3]) -> Result<(), Error> {
        let velocity = self
            .boundary_coupling
            .as_ref()
            .ok_or(Error::InvalidBoundary)?
            .body
            .velocity;
        self.translate_boundaries(displacement)?;
        self.configure_boundary_velocities(vec![velocity; self.boundaries.len()])
    }
    pub(super) fn boundary_body_time_limit(
        &self,
        forces: &BoundaryForces,
        particles: &[Particle],
    ) -> Result<f64, Error> {
        let body = self
            .boundary_coupling
            .as_ref()
            .ok_or(Error::InvalidBoundary)?
            .body;
        let limit = forces.time_limit(
            particles,
            body,
            self.config.smoothing_radius,
            self.config.gravity,
        )?;
        Ok(limit.min(self.rotating_boundary_time_limit(forces, particles)?))
    }
    pub(super) fn boundary_body_forces(
        &self,
        particles: &[Particle],
        properties: &[Material],
        density: &[f64],
        fluid_pairs: usize,
    ) -> Result<BoundaryForces, Error> {
        let neighbors = self.boundary_pairs(particles, fluid_pairs)?;
        let mut pressure = vec![[0.0; 3]; particles.len()];
        self.add_boundary_pressure(particles, properties, density, &neighbors, &mut pressure)?;
        let rates = self.boundary_viscous_rates(properties, density, &neighbors)?;
        Ok(BoundaryForces { pressure, rates })
    }
    pub(super) fn advance_boundary_body(
        &mut self,
        particles: &mut [Particle],
        mut fields: Option<&mut Transport>,
        forces: &BoundaryForces,
        dt: f64,
    ) -> Result<(), Error> {
        let mut coupling = self
            .boundary_coupling
            .clone()
            .ok_or(Error::InvalidBoundary)?;
        for a in 0..3 {
            coupling.body.velocity[a] += self.config.gravity[a] * dt;
        }
        for (i, p) in particles.iter_mut().enumerate() {
            let impulse = forces.pressure[i].map(|a| p.mass * a * dt);
            let work = coupled_kick(p, &mut coupling, impulse)?;
            for (a, value) in impulse.into_iter().enumerate() {
                coupling.pressure_impulse[a] -= value;
            }
            if self.pressure_work {
                let thermal = fields.as_deref_mut().ok_or(Error::InvalidTransport)?;
                let energy = thermal.energy(p, &thermal.fields[i], i)?;
                thermal.set_energy(i, p, energy - work)?;
                coupling.pressure_work += work;
            }
        }
        for (i, p) in particles.iter_mut().enumerate() {
            let (impulse, work) = if coupling.rotation.is_some() {
                super::rotation::viscous_kick(
                    p,
                    &mut coupling,
                    forces.rates[i],
                    dt,
                    self.viscous_heating,
                )?
            } else {
                let reduced = 1.0 / (1.0 / p.mass + 1.0 / coupling.body.mass);
                let conductance = p.mass * forces.rates[i];
                let factor = if self.viscous_heating {
                    reduced * -(-conductance / reduced * dt).exp_m1()
                } else {
                    conductance * dt
                };
                if !positive(reduced) || !factor.is_finite() || factor < 0.0 {
                    return Err(Error::NumericalFailure);
                }
                let impulse =
                    std::array::from_fn(|a| -factor * (p.velocity[a] - coupling.body.velocity[a]));
                let work = coupled_kick(p, &mut coupling, impulse)?;
                (impulse, work)
            };
            for (a, value) in impulse.into_iter().enumerate() {
                coupling.viscous_impulse[a] -= value;
            }
            if self.viscous_heating {
                let thermal = fields.as_deref_mut().ok_or(Error::InvalidTransport)?;
                let energy = thermal.energy(p, &thermal.fields[i], i)?;
                thermal.set_energy(i, p, energy - work)?;
                coupling.viscous_heat -= work;
            }
        }
        if !finite(coupling.body.position)
            || !finite(coupling.pressure_impulse)
            || !finite(coupling.viscous_impulse)
            || !coupling.pressure_work.is_finite()
            || !coupling.viscous_heat.is_finite()
        {
            return Err(Error::NumericalFailure);
        }

        self.boundary_coupling = Some(coupling);
        Ok(())
    }
}
impl BoundaryForces {
    pub(super) fn time_limit(
        &self,
        particles: &[Particle],
        body: TranslatingBody,
        support: f64,
        gravity: [f64; 3],
    ) -> Result<f64, Error> {
        let mut limit = 0.25 * support / norm(body.velocity).max(f64::MIN_POSITIVE);
        let mut body_force = [0.0; 3];
        let mut body_rate = 0.0;
        for (i, p) in particles.iter().enumerate() {
            let acceleration = norm(self.pressure[i]);
            if acceleration > 0.0 {
                limit = limit.min(0.25 * (support / acceleration).sqrt());
            }
            body_rate += p.mass * self.rates[i] / body.mass;
            for (a, force) in body_force.iter_mut().enumerate() {
                *force -= p.mass * self.pressure[i][a];
            }
            let rate = self.rates[i] * (1.0 + p.mass / body.mass);
            if rate > 0.0 {
                limit = limit.min(0.25 / rate);
            }
        }
        if body_rate > 0.0 {
            limit = limit.min(0.25 / body_rate);
        }
        let acceleration = norm(std::array::from_fn(|a| {
            body_force[a] / body.mass + gravity[a]
        }));
        if acceleration > 0.0 {
            limit = limit.min(0.25 * (support / acceleration).sqrt());
        }
        if !finite(body_force)
            || !body_rate.is_finite()
            || !acceleration.is_finite()
            || limit.is_nan()
            || limit <= 0.0
        {
            return Err(Error::NumericalFailure);
        }
        Ok(limit)
    }
}
pub(super) fn coupled_kick(
    p: &mut Particle,
    coupling: &mut Coupling,
    impulse: [f64; 3],
) -> Result<f64, Error> {
    let rotational_work = super::rotation::angular_kick(p, coupling, impulse)?;
    Ok(kick(p, &mut coupling.body, impulse)? + rotational_work)
}
/// Equal opposite impulse; work is evaluated in relative coordinates for Galilean invariance.
fn kick(p: &mut Particle, body: &mut TranslatingBody, impulse: [f64; 3]) -> Result<f64, Error> {
    let inverse = 1.0 / p.mass + 1.0 / body.mass;
    let work = impulse
        .iter()
        .enumerate()
        .map(|(a, j)| j * (p.velocity[a] - body.velocity[a] + 0.5 * j * inverse))
        .sum::<f64>();
    for (a, j) in impulse.into_iter().enumerate() {
        p.velocity[a] += j / p.mass;
        body.velocity[a] -= j / body.mass;
    }
    if !work.is_finite() || !finite(p.velocity) || !finite(body.velocity) {
        return Err(Error::NumericalFailure);
    }
    Ok(work)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundaryWorldReport {
    pub boundary: BoundaryBodyReport,
    pub contacts: usize,
    pub queries: usize,
    /// Contact loss; not yet deposited as heat.
    pub dissipated_energy: f64,
}
