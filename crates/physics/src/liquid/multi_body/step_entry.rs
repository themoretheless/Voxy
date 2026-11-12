use super::{
    ContactLedger, DynamicEnvironmentReport, DynamicWorldConfig, DynamicWorldReport, Error, Liquid,
    LiquidBodyWorld, RigidWork, RigidWorldReport, SupportedWorldReport, TranslatingBody,
    default_rotation, finite, solve_contact,
};

impl Liquid {
    /// Coupled finite-body/body, fluid/body and fixed-world events in one transaction.
    /// Rotation is constrained; backend templates may be affine or compound geometry.
    /// # Errors
    /// Invalid bodies/budgets/backend contacts or numerical failure restore every owner.
    pub fn step_with_body_world(
        &mut self,
        dt: f64,
        bodies: &mut [TranslatingBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
    ) -> Result<DynamicEnvironmentReport, Error> {
        if bodies.len() > max_bodies || max_bodies == 0 {
            return Err(Error::InvalidCollision);
        }
        let mut rigid: Vec<_> = bodies
            .iter()
            .map(|b| crate::contact::ContactBody {
                motion: crate::gravity::Body {
                    position: b.position,
                    velocity: b.velocity,
                    mass: b.mass,
                },
                spin: None,
            })
            .collect();
        let report = self.step_with_rigid_body_world(
            dt,
            &mut rigid,
            world,
            config,
            max_bodies,
            default_rotation(),
        )?;
        for (body, state) in bodies.iter_mut().zip(rigid) {
            body.position = state.motion.position;
            body.velocity = state.motion.velocity;
        }
        Ok(report)
    }

    /// Same event loop with intrinsic spin. Geometry must admit angular paths and
    /// world witnesses. Legacy tangential damping is unsupported for rigid spin.
    pub fn step_with_rigid_body_world(
        &mut self,
        dt: f64,
        bodies: &mut [crate::contact::ContactBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
        rotation: crate::spin_path::Config,
    ) -> Result<DynamicEnvironmentReport, Error> {
        let forces = vec![crate::contact::ContactWrench::default(); bodies.len()];
        self.step_with_rigid_body_forces(dt, bodies, world, config, max_bodies, rotation, &forces)
            .map(|report| report.world)
    }

    /// Same transactional event loop with additional constant world COM forces
    /// and torques for each body. Configured body gravity is applied continuously
    /// once, in addition to these wrenches. Accelerated geometry must be supported
    /// by the backend; linear-only defaults reject rather than sweep a chord.
    #[expect(
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn step_with_rigid_body_forces(
        &mut self,
        dt: f64,
        bodies: &mut [crate::contact::ContactBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
        rotation: crate::spin_path::Config,
        additional: &[crate::contact::ContactWrench],
    ) -> Result<RigidWorldReport, Error> {
        self.step_rigid_forces(
            dt, bodies, world, config, max_bodies, rotation, additional, None, None,
        )
        .map(|report| report.rigid)
    }

    /// Shared event loop with geometry-admitted constant-reaction intervals.
    /// Unsupported evolving branches reject transactionally; velocities and Spin
    /// are sampled from their real net-wrench paths, never clamped to rest.
    #[expect(
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn step_with_supported_rigid_body_forces(
        &mut self,
        dt: f64,
        bodies: &mut [crate::contact::ContactBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
        rotation: crate::spin_path::Config,
        additional: &[crate::contact::ContactWrench],
        supported: super::super::SupportedWorldConfig,
    ) -> Result<SupportedWorldReport, Error> {
        supported.validate()?;
        self.step_rigid_forces(
            dt,
            bodies,
            world,
            config,
            max_bodies,
            rotation,
            additional,
            Some(supported),
            None,
        )
    }

    /// Material-point forces with coupled, geometry-admitted support reactions.
    /// Point recipes begin at this tick's origin and are rebased after every
    /// interval and impact. Unresolved contact evolution rolls back the whole tick.
    #[expect(
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn step_with_supported_rigid_body_point_forces(
        &mut self,
        dt: f64,
        bodies: &mut [crate::contact::ContactBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
        rotation: crate::spin_path::Config,
        additional: &[crate::contact::ContactWrench],
        points: &[Vec<crate::rigid_motion::MaterialPointForce>],
        supported: super::super::SupportedWorldConfig,
    ) -> Result<SupportedWorldReport, Error> {
        supported.validate()?;
        self.step_rigid_forces(
            dt,
            bodies,
            world,
            config,
            max_bodies,
            rotation,
            additional,
            Some(supported),
            Some(points),
        )
    }

    /// Own-body material-point forces in the shared transactional contact loop.
    /// Input recipes start at this tick's origin; supported pressure coupling
    /// is not included by this entry point.
    #[expect(
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    pub fn step_with_rigid_body_point_forces(
        &mut self,
        dt: f64,
        bodies: &mut [crate::contact::ContactBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
        rotation: crate::spin_path::Config,
        additional: &[crate::contact::ContactWrench],
        points: &[Vec<crate::rigid_motion::MaterialPointForce>],
    ) -> Result<RigidWorldReport, Error> {
        self.step_rigid_forces(
            dt,
            bodies,
            world,
            config,
            max_bodies,
            rotation,
            additional,
            None,
            Some(points),
        )
        .map(|report| report.rigid)
    }

    #[expect(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
    fn step_rigid_forces(
        &mut self,
        dt: f64,
        bodies: &mut [crate::contact::ContactBody],
        world: &impl LiquidBodyWorld,
        config: DynamicWorldConfig,
        max_bodies: usize,
        rotation: crate::spin_path::Config,
        additional: &[crate::contact::ContactWrench],
        supported: Option<super::super::SupportedWorldConfig>,
        point_loads: Option<&[Vec<crate::rigid_motion::MaterialPointForce>]>,
    ) -> Result<SupportedWorldReport, Error> {
        if let Some(loads) = point_loads {
            if loads.len() != bodies.len() {
                return Err(Error::InvalidCollision);
            }
            for (points, body) in loads.iter().zip(bodies.iter()) {
                for point in points {
                    point.shifted(dt).map_err(|_| Error::InvalidCollision)?;
                    if body.spin.is_none() && point.local != [0.; 3] {
                        return Err(Error::InvalidCollision);
                    }
                }
            }
        }
        config.contact.validate()?;
        if additional.len() != bodies.len()
            || additional.iter().zip(bodies.iter()).any(|(w, b)| {
                !finite(w.force) || !finite(w.torque) || (b.spin.is_none() && w.torque != [0.; 3])
            })
            || bodies.len() > max_bodies
            || max_bodies == 0
            || config.max_contacts == 0
            || config.max_queries == 0
            || bodies
                .iter()
                .any(|b| b.energy().is_err() || (b.spin.is_some() && config.contact.friction != 0.))
        {
            return Err(Error::InvalidCollision);
        }
        let mut candidate = self.clone();
        let mut bodies_candidate = bodies.to_vec();
        let mut ledger = ContactLedger::default();
        let radius = self.config.particle_radius;
        let gravity = self.config.gravity;
        let forces = super::super::support_world::world_wrenches(bodies, additional, gravity)?;
        let mut work = RigidWork::default();
        let empty = self.particles.is_empty();
        let mut load_elapsed = 0.;
        let fluid = candidate.advance(dt, None, |particles, time| {
            // SPH has already kicked pressure, viscosity and gravity. Retain
            // the first two operators, but put uniform gravity on the same
            // continuous drift paths as the bodies to avoid artificial relative
            // acceleration at a resting fluid/body contact.
            for particle in particles.iter_mut() {
                for k in 0..3 {
                    particle.velocity[k] -= gravity[k] * time;
                }
            }
            solve_contact(
                particles,
                &mut bodies_candidate,
                radius,
                time,
                config,
                &mut ledger,
                world,
                rotation,
                &forces,
                gravity,
                &mut work,
                supported,
                point_loads,
                load_elapsed,
            )?;
            load_elapsed += time;
            Ok(())
        })?;
        if empty {
            solve_contact(
                &mut [],
                &mut bodies_candidate,
                radius,
                dt,
                config,
                &mut ledger,
                world,
                rotation,
                &forces,
                gravity,
                &mut work,
                supported,
                point_loads,
                0.,
            )?;
        }
        *self = candidate;
        bodies.copy_from_slice(&bodies_candidate);
        Ok(SupportedWorldReport {
            reaction_work: work.reaction,
            environment_reaction_impulse: work.environment_reaction_impulse,
            reaction_angular_impulse: work.reaction_angular_impulse,
            environment_reaction_angular_impulse: work.environment_reaction_angular_impulse,
            reaction_angular_balance_residual: std::array::from_fn(|k| {
                work.reaction_angular_impulse[k] + work.environment_reaction_angular_impulse[k]
            }),

            supported_intervals: work.supported_intervals,
            support_points: work.support_points,
            max_support_error_m: work.max_support_error_m,
            rigid: RigidWorldReport {
                external_work: work.external,
                boundary_impact_work: ledger.boundary_impact_work,
                integration_energy_residual: work.residual,
                world: DynamicEnvironmentReport {
                    dynamics: DynamicWorldReport {
                        fluid,
                        contacts: ledger.contacts,
                        queries: ledger.queries,
                        dissipated_energy: ledger.loss,
                    },
                    environment_impulse: ledger.environment_impulse,
                },
            },
        })
    }
}
