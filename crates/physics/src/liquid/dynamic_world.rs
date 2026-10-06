use super::{ContactConfig, Error, Liquid, Particle, StepStats, finite, positive};

/// Translation of a fixed collision template. Rotation is constrained.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TranslatingBody {
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub mass: f64,
}
/// Translational collision body with constant specific heat and uniform temperature.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermalTranslatingBody {
    pub mechanics: TranslatingBody,
    /// J/(kg K) in SI; finite and positive.
    pub specific_heat: f64,
    /// Absolute temperature, finite and nonnegative.
    pub temperature: f64,
}
impl ThermalTranslatingBody {
    /// Sensible thermal energy relative to zero temperature.
    /// # Errors
    /// Invalid temperature/capacity or unrepresentable energy.
    pub fn thermal_energy(&self) -> Result<f64, Error> {
        let capacity = self.mechanics.mass * self.specific_heat;
        if !positive(self.mechanics.mass)
            || !positive(self.specific_heat)
            || !positive(capacity)
            || !self.temperature.is_finite()
            || self.temperature < 0.0
        {
            return Err(Error::InvalidTransport);
        }
        let energy = capacity * self.temperature;
        if !energy.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(energy)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicWorldConfig {
    pub contact: ContactConfig,
    /// Global event and query budgets for the entire outer step.
    pub max_contacts: usize,
    pub max_queries: usize,
}
impl Default for DynamicWorldConfig {
    fn default() -> Self {
        Self {
            contact: ContactConfig::default(),
            max_contacts: 100_000,
            max_queries: 4_000_000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicWorldReport {
    pub fluid: StepStats,
    pub contacts: usize,
    pub queries: usize,
    /// Contact losses are reported, not converted into fluid heat.
    pub dissipated_energy: f64,
}
/// Finite translational recoil plus a user-selected impact heat partition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicImpactReport {
    pub dynamics: DynamicWorldReport,
    pub fluid_heat: f64,
    /// Heat assigned to the body; thermal-body stepping also updates its temperature.
    pub body_heat: f64,
}
/// Read-only static surroundings for a finite translating geometry template.
/// The backend owns the body's collision shape and sweeps it exactly.
pub trait DynamicLiquidEnvironment {
    /// # Errors
    /// Invalid geometry or exhausted query budgets.
    fn sweep_particle(
        &self,
        center: [f64; 3],
        radius: f64,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<super::GeometryHit, Error>;
    /// # Errors
    /// Invalid geometry or exhausted query budgets.
    fn sweep_body(
        &self,
        body: &TranslatingBody,
        displacement: [f64; 3],
        max_candidates: usize,
    ) -> Result<super::GeometryHit, Error>;
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicEnvironmentReport {
    pub dynamics: DynamicWorldReport,
    /// Impulse received by the static surroundings (opposite moving participants).
    pub environment_impulse: [f64; 3],
}

impl Liquid {
    /// Advances finite-body collisions and stores the body share of impact heat
    /// in its uniform temperature. Specific heat is constant; no body phase change,
    /// conductive liquid/body exchange or thermal mechanical response is applied.
    /// # Errors
    /// Invalid capacity/temperature, collision/constitutive failures or overflow.
    /// Both complete fluid and thermal-body state remain unchanged on any error.
    pub fn step_symmetric_with_thermal_body(
        &mut self,
        dt: f64,
        body: &mut ThermalTranslatingBody,
        world: &impl crate::CollisionWorld,
        config: DynamicWorldConfig,
        fluid_heat_fraction: f64,
    ) -> Result<DynamicImpactReport, Error> {
        let initial_energy = body.thermal_energy()?;
        let capacity = body.mechanics.mass * body.specific_heat;
        let mut fluid_candidate = self.clone();
        let mut body_candidate = *body;
        let report = fluid_candidate.step_symmetric_with_dynamic_world_heating(
            dt,
            &mut body_candidate.mechanics,
            world,
            config,
            fluid_heat_fraction,
        )?;
        let energy = initial_energy + report.body_heat;
        body_candidate.temperature = energy / capacity;
        if !energy.is_finite() || !body_candidate.temperature.is_finite() {
            return Err(Error::NumericalFailure);
        }
        body_candidate.thermal_energy()?;
        let fluid_energy = fluid_candidate
            .transport_totals()?
            .ok_or(Error::InvalidTransport)?
            .0;
        if !(energy + fluid_energy).is_finite() {
            return Err(Error::NumericalFailure);
        }
        *self = fluid_candidate;
        *body = body_candidate;
        Ok(report)
    }
    /// Symmetric fluid stepping and swept finite-mass translational collisions.
    /// Impact loss is assigned to the contacting fluid particle and body ledger.
    /// No body rotation, wall heat capacity or sampled-boundary pressure recoil.
    /// # Errors
    /// Invalid body/configuration/heat fraction, backend or budget failure, invalid
    /// thermal response or overflow. Both fluid and body remain unchanged on error.
    pub fn step_symmetric_with_dynamic_world_heating(
        &mut self,
        dt: f64,
        body: &mut TranslatingBody,
        world: &impl crate::CollisionWorld,
        config: DynamicWorldConfig,
        fluid_heat_fraction: f64,
    ) -> Result<DynamicImpactReport, Error> {
        config.contact.validate()?;
        if !finite(body.position)
            || !finite(body.velocity)
            || !positive(body.mass)
            || config.max_contacts == 0
            || config.max_queries == 0
        {
            return Err(Error::InvalidCollision);
        }
        if !(0.0..=1.0).contains(&fluid_heat_fraction) {
            return Err(Error::InvalidTransport);
        }
        let mut fluid_candidate = self.clone();
        let mut body_candidate = *body;
        let mut ledger = ContactLedger {
            particle_loss: vec![0.0; self.particles.len()],
            ..ContactLedger::default()
        };
        let gravity = self.config.gravity;
        let radius = self.config.particle_radius;
        let empty = self.particles.is_empty();
        let mut fluid_heat = 0.0;
        let mut body_heat = 0.0;
        let fluid = fluid_candidate.advance_symmetric_with_mover(dt, |state, time| {
            for (v, g) in body_candidate.velocity.iter_mut().zip(gravity) {
                *v += 0.5 * time * g;
            }
            ledger.particle_loss.fill(0.0);
            sweep_particles(
                &mut state.particles,
                &mut body_candidate,
                radius,
                time,
                world,
                config,
                &mut ledger,
            )?;
            for (v, g) in body_candidate.velocity.iter_mut().zip(gravity) {
                *v += 0.5 * time * g;
            }
            if !finite(body_candidate.velocity) {
                return Err(Error::NumericalFailure);
            }
            let heat: Vec<_> = ledger
                .particle_loss
                .iter()
                .map(|loss| fluid_heat_fraction * loss)
                .collect();
            fluid_heat += heat.iter().sum::<f64>();
            body_heat += (1.0 - fluid_heat_fraction) * ledger.particle_loss.iter().sum::<f64>();
            if !fluid_heat.is_finite() || !body_heat.is_finite() {
                return Err(Error::NumericalFailure);
            }
            state.add_heat(&heat)
        })?;
        if empty {
            for (axis, g) in gravity.iter().enumerate() {
                body_candidate.position[axis] +=
                    body_candidate.velocity[axis] * dt + 0.5 * g * dt * dt;
                body_candidate.velocity[axis] += g * dt;
            }
            if !finite(body_candidate.position) || !finite(body_candidate.velocity) {
                return Err(Error::NumericalFailure);
            }
        }
        *self = fluid_candidate;
        *body = body_candidate;
        Ok(DynamicImpactReport {
            dynamics: DynamicWorldReport {
                fluid,
                contacts: ledger.contacts,
                queries: ledger.queries,
                dissipated_energy: ledger.loss,
            },
            fluid_heat,
            body_heat,
        })
    }
    /// Finite-mass translational recoil against arbitrary unit-normal geometry.
    /// Geometry is a body-local collision template; body.position translates it.
    /// # Errors
    /// Invalid body/settings, overlap, geometry failure or event budgets roll back
    /// both entire fluid and body. Rotation and sampled boundary recoil are absent.
    pub fn step_with_dynamic_geometry(
        &mut self,
        dt: f64,
        body: &mut TranslatingBody,
        geometry: &impl super::LiquidGeometry,
        config: DynamicWorldConfig,
    ) -> Result<DynamicWorldReport, Error> {
        self.step_dynamic_geometry(dt, body, geometry, config, None)
            .map(|r| r.dynamics)
    }

    /// Finite-body and static-environment contacts on one earliest-event timeline.
    /// # Errors
    /// Any geometry, event or numerical failure restores fluid and body together.
    pub fn step_with_dynamic_geometry_and_environment(
        &mut self,
        dt: f64,
        body: &mut TranslatingBody,
        geometry: &impl super::LiquidGeometry,
        environment: &impl DynamicLiquidEnvironment,
        config: DynamicWorldConfig,
    ) -> Result<DynamicEnvironmentReport, Error> {
        self.step_dynamic_geometry(dt, body, geometry, config, Some(environment))
    }

    fn step_dynamic_geometry(
        &mut self,
        dt: f64,
        body: &mut TranslatingBody,
        geometry: &impl super::LiquidGeometry,
        config: DynamicWorldConfig,
        environment: Option<&dyn DynamicLiquidEnvironment>,
    ) -> Result<DynamicEnvironmentReport, Error> {
        config.contact.validate()?;
        if !finite(body.position)
            || !finite(body.velocity)
            || !positive(body.mass)
            || config.max_contacts == 0
            || config.max_queries == 0
        {
            return Err(Error::InvalidCollision);
        }
        let mut fluid_candidate = self.clone();
        let mut candidate = *body;
        let mut ledger = ContactLedger::default();
        let radius = self.config.particle_radius;
        let gravity = self.config.gravity;
        let empty = self.particles.is_empty();
        let fluid = fluid_candidate.advance(dt, None, |particles, time| {
            for axis in 0..3 {
                candidate.velocity[axis] += gravity[axis] * time;
            }
            sweep_particles_query(
                particles,
                &mut candidate,
                time,
                radius,
                config,
                &mut ledger,
                |p, body, remaining| {
                    let center = std::array::from_fn(|a| p.position[a] - body.position[a]);
                    let displacement =
                        std::array::from_fn(|a| (p.velocity[a] - body.velocity[a]) * remaining);
                    if !finite(center) || !finite(displacement) {
                        return Err(Error::NumericalFailure);
                    }
                    match geometry
                        .sweep(center, radius, displacement, config.contact.max_candidates)
                        .map_err(|_| Error::CollisionBackend)?
                    {
                        super::GeometryHit::Clear => Ok(None),
                        super::GeometryHit::Overlap => Err(Error::InitialOverlap),
                        super::GeometryHit::Contact { fraction, normal } => {
                            let length2: f64 = normal.iter().map(|v| v * v).sum();
                            let speed: f64 = (0..3)
                                .map(|a| (p.velocity[a] - body.velocity[a]) * normal[a])
                                .sum();
                            if !fraction.is_finite()
                                || !(0. ..=1.).contains(&fraction)
                                || !length2.is_finite()
                                || (length2 - 1.).abs() > 1e-10
                                || !speed.is_finite()
                                || speed >= 0.
                            {
                                return Err(Error::InvalidCollision);
                            }
                            Ok(Some((fraction, normal)))
                        }
                    }
                },
                environment,
            )
        })?;
        if empty {
            for axis in 0..3 {
                candidate.velocity[axis] += gravity[axis] * dt;
            }
            sweep_particles_query(
                &mut [],
                &mut candidate,
                dt,
                radius,
                config,
                &mut ledger,
                |_, _, _| Ok(None),
                environment,
            )?;
        }
        *self = fluid_candidate;
        *body = candidate;
        Ok(DynamicEnvironmentReport {
            dynamics: DynamicWorldReport {
                fluid,
                contacts: ledger.contacts,
                queries: ledger.queries,
                dissipated_energy: ledger.loss,
            },
            environment_impulse: ledger.environment_impulse,
        })
    }

    /// Swept two-way collisions against one translating finite-mass collision template.
    /// The backend stays in template coordinates; `body.position` supplies its translation.
    /// SPH boundary samples remain independent environment samples, not body-owned samples.
    /// `contact.max_candidates` bounds each backend query; the global limits bound events.
    /// # Errors
    /// Invalid settings, overlap, backend failure or exhausted budgets. Fluid and body
    /// are both unchanged on error. Body rotation and sampled boundary recoil are absent.
    pub fn step_with_dynamic_world(
        &mut self,
        dt: f64,
        body: &mut TranslatingBody,
        world: &impl crate::CollisionWorld,
        config: DynamicWorldConfig,
    ) -> Result<DynamicWorldReport, Error> {
        config.contact.validate()?;
        if !finite(body.position)
            || !finite(body.velocity)
            || !positive(body.mass)
            || config.max_contacts == 0
            || config.max_queries == 0
        {
            return Err(Error::InvalidCollision);
        }
        let mut candidate = *body;
        let mut ledger = ContactLedger::default();
        let radius = self.config.particle_radius;
        let gravity = self.config.gravity;
        let empty = self.particles.is_empty();
        let fluid = self.advance(dt, None, |particles, time| {
            for (axis, acceleration) in gravity.into_iter().enumerate() {
                candidate.velocity[axis] += acceleration * time;
            }
            sweep_particles(
                particles,
                &mut candidate,
                radius,
                time,
                world,
                config,
                &mut ledger,
            )
        })?;
        if empty {
            for (a, g) in gravity.into_iter().enumerate() {
                candidate.velocity[a] += g * dt;
            }
            // Validate before committing the already-empty fluid state.
            drift(&mut [], &mut candidate, dt)?;
        }
        *body = candidate;
        Ok(DynamicWorldReport {
            fluid,
            contacts: ledger.contacts,
            queries: ledger.queries,
            dissipated_energy: ledger.loss,
        })
    }
}
fn drift(particles: &mut [Particle], body: &mut TranslatingBody, dt: f64) -> Result<(), Error> {
    for p in particles {
        for a in 0..3 {
            p.position[a] += p.velocity[a] * dt;
        }
        if !finite(p.position) {
            return Err(Error::NumericalFailure);
        }
    }
    for a in 0..3 {
        body.position[a] += body.velocity[a] * dt;
    }
    if !finite(body.position) || !finite(body.velocity) {
        return Err(Error::NumericalFailure);
    }
    Ok(())
}

fn query(
    p: &Particle,
    body: &TranslatingBody,
    radius: f64,
    remaining: f64,
    world: &impl crate::CollisionWorld,
    budget: usize,
) -> Result<Option<(f64, [f64; 3])>, Error> {
    let center: [f64; 3] = std::array::from_fn(|a| p.position[a] - body.position[a]);
    let displacement = std::array::from_fn(|a| (p.velocity[a] - body.velocity[a]) * remaining);
    if !finite(center) || !finite(displacement) {
        return Err(Error::NumericalFailure);
    }
    let hit = world
        .sweep_aabb(
            crate::AnchoredAabb {
                anchor: crate::Origin::default(),
                min: center.map(|x| x - radius),
                max: center.map(|x| x + radius),
            },
            displacement,
            budget,
        )
        .map_err(|_| Error::CollisionBackend)?;
    if !hit.fraction.is_finite() || !(0.0..=1.0).contains(&hit.fraction) {
        return Err(Error::InvalidCollision);
    }
    let count = hit.normal.iter().filter(|&&n| n != 0).count();
    if count == 0 {
        if hit.obstacle.is_some() {
            return Err(Error::InitialOverlap);
        }
        if hit.fraction < 1.0 {
            return Err(Error::InvalidCollision);
        }
        return Ok(None);
    }
    if count != 1 || hit.obstacle.is_none() || hit.normal.iter().any(|&n| !(-1..=1).contains(&n)) {
        return Err(Error::InvalidCollision);
    }
    let axis = hit
        .normal
        .iter()
        .position(|&n| n != 0)
        .ok_or(Error::InvalidCollision)?;
    let normal = f64::from(hit.normal[axis]);
    if (p.velocity[axis] - body.velocity[axis]) * normal >= 0.0 {
        return Err(Error::InvalidCollision);
    }
    Ok(Some((hit.fraction, hit.normal.map(f64::from))))
}

#[derive(Default)]
pub(super) struct ContactLedger {
    pub contacts: usize,
    pub queries: usize,
    pub loss: f64,
    pub particle_loss: Vec<f64>,
    pub(super) environment_impulse: [f64; 3],
}
pub(super) fn sweep_particles(
    particles: &mut [Particle],
    body: &mut TranslatingBody,
    radius: f64,
    time: f64,
    world: &impl crate::CollisionWorld,
    config: DynamicWorldConfig,
    ledger: &mut ContactLedger,
) -> Result<(), Error> {
    sweep_particles_query(
        particles,
        body,
        time,
        radius,
        config,
        ledger,
        |p, body, remaining| {
            query(
                p,
                body,
                radius,
                remaining,
                world,
                config.contact.max_candidates,
            )
        },
        None,
    )
}
fn sweep_particles_query(
    particles: &mut [Particle],
    body: &mut TranslatingBody,
    time: f64,
    radius: f64,
    config: DynamicWorldConfig,
    ledger: &mut ContactLedger,
    query: impl FnMut(&Particle, &TranslatingBody, f64) -> Result<Option<(f64, [f64; 3])>, Error>,
    environment: Option<&dyn DynamicLiquidEnvironment>,
) -> Result<(), Error> {
    use std::cell::RefCell;
    struct Single<'a, F> {
        query: RefCell<F>,
        environment: Option<&'a dyn DynamicLiquidEnvironment>,
    }
    impl<F: FnMut(&Particle, &TranslatingBody, f64) -> Result<Option<(f64, [f64; 3])>, Error>>
        super::LiquidBodyWorld for Single<'_, F>
    {
        fn sweep_particle_body(
            &self,
            p: &Particle,
            _: f64,
            _: usize,
            b: &TranslatingBody,
            dt: f64,
            _: usize,
        ) -> Result<super::GeometryHit, Error> {
            Ok(self.query.borrow_mut()(p, b, dt)?
                .map_or(super::GeometryHit::Clear, |(fraction, normal)| {
                    super::GeometryHit::Contact { fraction, normal }
                }))
        }
        fn sweep_body_pair(
            &self,
            _: usize,
            _: &TranslatingBody,
            _: usize,
            _: &TranslatingBody,
            _: f64,
            _: usize,
        ) -> Result<super::GeometryHit, Error> {
            Ok(super::GeometryHit::Clear)
        }
        fn sweep_particle_environment(
            &self,
            p: &Particle,
            radius: f64,
            dt: f64,
            budget: usize,
        ) -> Result<super::GeometryHit, Error> {
            self.environment.unwrap().sweep_particle(
                p.position,
                radius,
                p.velocity.map(|v| v * dt),
                budget,
            )
        }
        fn sweep_body_environment(
            &self,
            _: usize,
            b: &TranslatingBody,
            dt: f64,
            budget: usize,
        ) -> Result<super::GeometryHit, Error> {
            self.environment
                .unwrap()
                .sweep_body(b, b.velocity.map(|v| v * dt), budget)
        }
        fn has_environment(&self) -> bool {
            self.environment.is_some()
        }
    }
    super::multi_body::solve(
        particles,
        std::slice::from_mut(body),
        radius,
        time,
        config,
        ledger,
        &Single {
            query: RefCell::new(query),
            environment,
        },
    )
}
