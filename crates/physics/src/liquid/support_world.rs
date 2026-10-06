//! Read-only geometry assembly for coupled rigid support reactions.
//! The result belongs to the supplied snapshots, not to a future interval.
use super::{DynamicWorldConfig, Error, Liquid, LiquidBodyWorld, RigidSupportPoint, finite};
use crate::contact::{
    ContactBody, ContactWrench, NetworkReaction, NetworkSupport, ReactionConfig,
    resolve_normal_reaction_network,
};

#[derive(Clone, Debug, PartialEq)]
pub struct RigidWorldReactions {
    pub supports: Vec<NetworkSupport>,
    /// Same-order geometry metadata required for finite support admission.
    pub geometry: Vec<RigidSupportPoint>,
    /// None means geometry proved that no supporting patches exist.
    pub reaction: Option<NetworkReaction>,
    /// Opposite force carried by the fixed environment, in newtons, not impulse.
    pub environment_force: [f64; 3],
    /// Number of pair/environment callback invocations. Internal shape queries
    /// are bounded separately by contact.max_candidates on each invocation.
    pub queries: usize,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportedWorldConfig {
    pub reaction: ReactionConfig,
    /// Enable affine reaction forces with this normal jerk tolerance.
    /// None retains the constant-reaction integration mode.
    pub reaction_jerk_tolerance: Option<f64>,
    /// Accepted constant-reaction intervals are also geometry-admitted. This is
    /// a maximum integration interval, not a contact stiffness or rest threshold.
    pub max_interval_s: f64,
    pub min_interval_s: f64,
    /// Maximum nominal contact gap/ownership error admitted by geometry, metres.
    /// Errors are measured and reported; physical states are never snapped.
    pub max_geometry_error_m: f64,
    pub max_intervals: usize,
}
impl Default for SupportedWorldConfig {
    fn default() -> Self {
        Self {
            reaction: ReactionConfig {
                max_sweeps: 8192,
                acceleration_tolerance: 1e-12,
                normal_velocity_tolerance: 1e-10,
            },
            reaction_jerk_tolerance: None,
            max_interval_s: 0.01,
            min_interval_s: 1e-12,
            max_geometry_error_m: 1e-10,
            max_intervals: 4096,
        }
    }
}
impl SupportedWorldConfig {
    pub fn validate(self) -> Result<(), Error> {
        let r = self.reaction;
        if self
            .reaction_jerk_tolerance
            .is_some_and(|t| !t.is_finite() || t <= 0.)
            || !self.max_interval_s.is_finite()
            || !self.min_interval_s.is_finite()
            || self.min_interval_s <= 0.
            || self.max_interval_s < self.min_interval_s
            || !self.max_geometry_error_m.is_finite()
            || self.max_geometry_error_m < 0.
            || self.max_intervals == 0
            || r.max_sweeps == 0
            || !r.acceleration_tolerance.is_finite()
            || r.acceleration_tolerance <= 0.
            || !r.normal_velocity_tolerance.is_finite()
            || r.normal_velocity_tolerance <= 0.
        {
            Err(Error::InvalidCollision)
        } else {
            Ok(())
        }
    }
}

pub(super) fn world_wrenches(
    bodies: &[ContactBody],
    additional: &[ContactWrench],
    gravity: [f64; 3],
) -> Result<Vec<ContactWrench>, Error> {
    if bodies.len() != additional.len()
        || bodies.iter().zip(additional).any(|(body, wrench)| {
            !finite(wrench.force)
                || !finite(wrench.torque)
                || (body.spin.is_none() && wrench.torque != [0.; 3])
        })
    {
        return Err(Error::InvalidCollision);
    }
    let forces: Vec<_> = bodies
        .iter()
        .zip(additional)
        .map(|(body, wrench)| ContactWrench {
            force: std::array::from_fn(|k| wrench.force[k] + body.motion.mass * gravity[k]),
            torque: wrench.torque,
        })
        .collect();
    if forces.iter().any(|w| !finite(w.force)) {
        return Err(Error::NumericalFailure);
    }
    Ok(forces)
}

impl Liquid {
    /// Read-only supporting reaction assembly under this world's configured
    /// gravity plus the supplied additional per-body COM loads. Uses the same
    /// load composition as step_with_rigid_body_forces; no time is advanced.
    pub fn rigid_world_reactions(
        &self,
        bodies: &[ContactBody],
        world: &impl LiquidBodyWorld,
        additional: &[ContactWrench],
        limits: DynamicWorldConfig,
        config: ReactionConfig,
    ) -> Result<RigidWorldReactions, Error> {
        let external = world_wrenches(bodies, additional, self.config.gravity)?;
        resolve_rigid_world_reactions(bodies, world, &external, limits, config)
    }
}

/// Gather every pair and fixed-environment patch and solve their reactions as
/// one indexed network. No velocity kick, position correction or state write is
/// made. Geometry failure, global/patch limits and nonconvergence reject the
/// complete result. This is the instantaneous force assembly needed before
/// constrained trajectory integration; it does not certify persistent branches.
pub fn resolve_rigid_world_reactions(
    bodies: &[ContactBody],
    world: &impl LiquidBodyWorld,
    external: &[ContactWrench],
    limits: DynamicWorldConfig,
    config: ReactionConfig,
) -> Result<RigidWorldReactions, Error> {
    assemble_reactions(bodies, world, external, limits, config, false, 0.)
}

/// Closing points are left to the existing impact loop. After an impact, fresh
/// snapshot assembly determines which zero-speed points can carry reactions.
pub(super) fn assemble_reactions(
    bodies: &[ContactBody],
    world: &impl LiquidBodyWorld,
    external: &[ContactWrench],
    limits: DynamicWorldConfig,
    config: ReactionConfig,
    omit_approaching: bool,
    geometry_error_m: f64,
) -> Result<RigidWorldReactions, Error> {
    limits.contact.validate()?;
    if bodies.is_empty()
        || bodies.len() > 128
        || bodies.len() != external.len()
        || limits.max_contacts == 0
        || limits.max_queries == 0
        || config.max_sweeps == 0
        || !config.acceleration_tolerance.is_finite()
        || config.acceleration_tolerance <= 0.
        || !config.normal_velocity_tolerance.is_finite()
        || config.normal_velocity_tolerance <= 0.
        || bodies.iter().zip(external).any(|(body, wrench)| {
            body.energy().is_err()
                || !finite(wrench.force)
                || !finite(wrench.torque)
                || (body.spin.is_none() && wrench.torque != [0.; 3])
        })
    {
        return Err(Error::InvalidCollision);
    }
    let mut supports = Vec::new();
    let mut geometry = Vec::new();
    let mut queries = 0;
    for i in 0..bodies.len() {
        for second in (i + 1..bodies.len())
            .map(Some)
            .chain(world.has_environment().then_some(None))
        {
            if queries >= limits.max_queries {
                return Err(Error::CollisionBudget);
            }
            queries += 1;
            let patch = if let Some(j) = second {
                world.rigid_pair_support_contacts_with_error(
                    i,
                    &bodies[i],
                    j,
                    &bodies[j],
                    limits.contact.max_candidates,
                    geometry_error_m,
                )?
            } else {
                world.rigid_environment_support_contacts_with_error(
                    i,
                    &bodies[i],
                    limits.contact.max_candidates,
                    geometry_error_m,
                )?
            };
            if patch.len() > limits.max_contacts.min(128).saturating_sub(supports.len()) {
                return Err(Error::CollisionBudget);
            }
            let speed = |point: &RigidSupportPoint| -> Result<f64, Error> {
                let p = point.support.contact.point;
                let a = bodies[i]
                    .point_velocity(p)
                    .map_err(|_| Error::InvalidCollision)?;
                let b = second
                    .map_or(Ok([0.; 3]), |j| bodies[j].point_velocity(p))
                    .map_err(|_| Error::InvalidCollision)?;
                let n = point.support.contact.normal;
                let norm = n[0].hypot(n[1]).hypot(n[2]);
                if !norm.is_finite() || norm <= 0. {
                    return Err(Error::InvalidCollision);
                }
                let speed: f64 = (0..3).map(|k| (a[k] - b[k]) * (n[k] / norm)).sum();
                if !speed.is_finite() {
                    Err(Error::InvalidCollision)
                } else {
                    Ok(speed)
                }
            };
            let mut closing = Vec::new();
            if omit_approaching {
                for point in &patch {
                    if speed(point)? < -config.normal_velocity_tolerance {
                        let key = point.feature.map(|f| f >> 8);
                        if !closing.contains(&key) {
                            closing.push(key);
                        }
                    }
                }
            }
            for point in patch {
                if !point.tolerance_m.is_finite()
                    || point.tolerance_m < 0.
                    || !point.admission_error_m.is_finite()
                    || point.admission_error_m < 0.
                    || point.admission_error_m > geometry_error_m
                {
                    return Err(Error::InvalidCollision);
                }
                // One closing point puts its entire shape pair back on the
                // impact path; outgoing vertices cannot hide that collision.
                if closing.contains(&point.feature.map(|f| f >> 8)) {
                    continue;
                }
                supports.push(NetworkSupport {
                    first: i,
                    second,
                    support: point.support,
                });
                geometry.push(point);
            }
        }
    }
    let reaction = if supports.is_empty() {
        None
    } else {
        Some(
            resolve_normal_reaction_network(bodies, &supports, external, config).map_err(|e| {
                match e {
                    crate::contact::Error::Budget => Error::CollisionBudget,
                    crate::contact::Error::NumericalFailure => Error::NumericalFailure,
                    _ => Error::InvalidCollision,
                }
            })?,
        )
    };
    let mut environment_force = [0.; 3];
    if let Some(reaction) = &reaction {
        for (entry, force) in supports.iter().zip(&reaction.forces) {
            if entry.second.is_none() {
                for k in 0..3 {
                    environment_force[k] -= force[k];
                }
            }
        }
    }
    if !finite(environment_force) {
        return Err(Error::NumericalFailure);
    }
    Ok(RigidWorldReactions {
        supports,
        geometry,
        reaction,
        environment_force,
        queries,
    })
}

/// Read-only derivative of the assembled world network under constant gravity.
#[derive(Clone, Debug, PartialEq)]
pub struct RigidWorldReactionRates {
    pub reactions: RigidWorldReactions,
    pub rate: Option<crate::contact::NetworkReactionRate>,
    /// Opposite fixed-world force derivative, newtons per second, not impulse.
    pub environment_force_rate: [f64; 3],
}
impl Liquid {
    /// Use the same snapshot geometry, owner indices and coupled baseline solve.
    /// Common point motion is supplied by geometry; the compatibility default
    /// follows first COM. Supported stepping retains its frozen-arm convention.
    /// Edge Rate branches need a geometry second derivative and reject here.
    pub fn rigid_world_reaction_rates(
        &self,
        bodies: &[ContactBody],
        world: &impl LiquidBodyWorld,
        additional: &[ContactWrench],
        additional_rate: &[ContactWrench],
        limits: DynamicWorldConfig,
        config: crate::contact::ReactionRateConfig,
    ) -> Result<RigidWorldReactionRates, Error> {
        if additional_rate.len() != bodies.len()
            || !config.jerk_tolerance.is_finite()
            || config.jerk_tolerance <= 0.
            || bodies.iter().zip(additional_rate).any(|(b, r)| {
                !finite(r.force) || !finite(r.torque) || (b.spin.is_none() && r.torque != [0.; 3])
            })
        {
            return Err(Error::InvalidCollision);
        }
        let external = world_wrenches(bodies, additional, self.config.gravity)?;
        let reactions =
            resolve_rigid_world_reactions(bodies, world, &external, limits, config.reaction)?;
        let mut environment_force_rate = [0.; 3];
        let rate = if let Some(baseline) = &reactions.reaction {
            let total: Vec<_> = external
                .iter()
                .zip(&baseline.wrenches)
                .map(|(a, b)| ContactWrench {
                    force: std::array::from_fn(|k| a.force[k] + b.force[k]),
                    torque: std::array::from_fn(|k| a.torque[k] + b.torque[k]),
                })
                .collect();
            let motion = reactions
                .supports
                .iter()
                .zip(&reactions.geometry)
                .map(|(s, geometry)| {
                    Ok(crate::contact::SupportMotion {
                        point_velocity: world
                            .rigid_support_point_velocity(bodies, *s, *geometry)?,
                        normal_acceleration: world
                            .rigid_support_normal_acceleration(bodies, &total, *s, *geometry)?,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            let rate = crate::contact::resolve_rate_from_baseline(
                bodies,
                &reactions.supports,
                &external,
                additional_rate,
                &motion,
                config,
                baseline.clone(),
            )
            .map_err(|e| match e {
                crate::contact::Error::Budget => Error::CollisionBudget,
                crate::contact::Error::InvalidInput => Error::InvalidCollision,
                _ => Error::NumericalFailure,
            })?;
            for (s, f) in reactions.supports.iter().zip(&rate.forces_rate) {
                if s.second.is_none() {
                    for k in 0..3 {
                        environment_force_rate[k] -= f[k];
                    }
                }
            }
            if !finite(environment_force_rate) {
                return Err(Error::NumericalFailure);
            }
            Some(rate)
        } else {
            None
        };
        Ok(RigidWorldReactionRates {
            reactions,
            rate,
            environment_force_rate,
        })
    }
}
