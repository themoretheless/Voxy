//! Shared earliest-event solver for fluid, translating/rotating bodies and fixed walls.
use super::dynamic_world::ContactLedger;
use super::{
    DynamicEnvironmentReport, DynamicWorldConfig, DynamicWorldReport, Error, GeometryHit, Liquid,
    Particle, TranslatingBody, finite, positive,
};

#[path = "multi_body/world_trait.rs"]
mod world_trait;
pub use world_trait::{
    BodyGeometryHit, ContactWitness, LiquidBodyWorld, RigidGeometryHit, RigidSupportPoint,
    RigidSupportPointMotion, SupportedGeometryHit,
};
#[path = "multi_body/support_participants.rs"]
mod support_participants;
use support_participants::SupportParticipants;

#[path = "multi_body/rigid_events.rs"]
mod rigid_events;
use rigid_events::{CollisionEvent, resolve_rigid_events};

#[derive(Clone, Copy)]
struct Node {
    spin: Option<crate::rotation::Spin>,
    position: [f64; 3],
    velocity: [f64; 3],
    mass: f64,
}
impl Node {
    fn contact(self) -> crate::contact::ContactBody {
        crate::contact::ContactBody {
            motion: crate::gravity::Body {
                mass: self.mass,
                position: self.position,
                velocity: self.velocity,
            },
            spin: self.spin,
        }
    }
    fn update(&mut self, body: crate::contact::ContactBody) {
        self.position = body.motion.position;
        self.velocity = body.motion.velocity;
        self.spin = body.spin;
    }
}

/// Body work accompanies the existing environment report without changing its
/// legacy DTO. Includes configured gravity and additional world COM wrenches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigidWorldReport {
    pub world: DynamicEnvironmentReport,
    pub external_work: f64,
    /// Work supplied to finite bodies by moving environment impacts.
    pub boundary_impact_work: f64,
    /// Nominal rotation integration and floating work discrepancy, not heat.
    pub integration_energy_residual: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportedWorldReport {
    pub rigid: RigidWorldReport,
    /// Signed work of reactions on the admitted nominal trajectories, not heat.
    pub reaction_work: f64,
    pub environment_reaction_impulse: [f64; 3],
    /// Reaction angular impulse on bodies and participating particles about the origin.
    /// Includes orbital COM motion and intrinsic COM torque; excludes impacts.
    pub reaction_angular_impulse: [f64; 3],
    /// Opposite moment transmitted to fixed geometry by support reactions.
    /// Uses the same admitted common-point motion as finite-body reactions.
    pub environment_reaction_angular_impulse: [f64; 3],
    /// Signed participant plus fixed-environment reaction angular impulse.
    /// Retains error of the admitted point model; never converted to heat.
    pub reaction_angular_balance_residual: [f64; 3],

    pub supported_intervals: usize,
    pub support_points: usize,
    pub max_support_error_m: f64,
}
#[derive(Default)]
struct RigidWork {
    external: f64,
    residual: f64,
    reaction: f64,
    environment_reaction_impulse: [f64; 3],
    reaction_angular_impulse: [f64; 3],
    environment_reaction_angular_impulse: [f64; 3],
    supported_intervals: usize,
    support_points: usize,
    max_support_error_m: f64,
    mixed_support: MixedSupportState,
}

#[derive(Default)]
struct MixedSupportState {
    particles: std::collections::BTreeSet<usize>,
    previous: std::collections::BTreeMap<(usize, usize), (Vec<RigidSupportPoint>, f64)>,
}

fn default_rotation() -> crate::spin_path::Config {
    crate::spin_path::Config {
        max_angular_error_rad: 1e-5,
        min_step_s: 1e-9,
        max_arcs: 1024,
        max_trials: 4096,
    }
}
pub(super) fn solve(
    particles: &mut [Particle],
    bodies: &mut [TranslatingBody],
    radius: f64,
    dt: f64,
    config: DynamicWorldConfig,
    ledger: &mut ContactLedger,
    world: &impl LiquidBodyWorld,
) -> Result<(), Error> {
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
    let forces = vec![crate::contact::ContactWrench::default(); rigid.len()];
    let mut work = RigidWork::default();
    solve_contact(
        particles,
        &mut rigid,
        radius,
        dt,
        config,
        ledger,
        world,
        default_rotation(),
        &forces,
        [0.; 3],
        &mut work,
        None,
        None,
        0.,
    )?;
    for (body, state) in bodies.iter_mut().zip(rigid) {
        body.position = state.motion.position;
        body.velocity = state.motion.velocity;
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
fn solve_contact(
    particles: &mut [Particle],
    bodies: &mut [crate::contact::ContactBody],
    radius: f64,
    dt: f64,
    config: DynamicWorldConfig,
    ledger: &mut ContactLedger,
    world: &impl LiquidBodyWorld,
    rotation: crate::spin_path::Config,
    forces: &[crate::contact::ContactWrench],
    particle_acceleration: [f64; 3],
    work: &mut RigidWork,
    supported: Option<super::SupportedWorldConfig>,
    point_loads: Option<&[Vec<crate::rigid_motion::MaterialPointForce>]>,
    load_offset: f64,
) -> Result<(), Error> {
    if forces.len() != bodies.len() {
        return Err(Error::InvalidCollision);
    }
    let count = particles.len();
    let mut nodes: Vec<_> = particles
        .iter()
        .map(|p| Node {
            spin: None,
            position: p.position,
            velocity: p.velocity,
            mass: p.mass,
        })
        .chain(bodies.iter().map(|b| Node {
            position: b.motion.position,
            velocity: b.motion.velocity,
            mass: b.motion.mass,
            spin: b.spin,
        }))
        .collect();
    let mut remaining = dt;
    let mut horizon = supported.map_or(dt, |c| dt.min(c.max_interval_s));
    let require_witness = bodies.iter().any(|body| body.spin.is_some());
    #[derive(Default)]
    struct BoundaryStall {
        time_s: f64,
        hits: usize,
        moving: Option<([f64; 3], Option<u64>)>,
        fixed: Option<[f64; 3]>,
    }
    let mut stalls = std::collections::HashMap::<usize, BoundaryStall>::new();
    struct ParticleContact {
        event: CollisionEvent,
        time: f64,
        particle_position: [f64; 3],
        body_position: Option<[f64; 3]>,
    }
    let mut prior_particle_contacts = std::collections::HashMap::<usize, ParticleContact>::new();
    let mut moving_boundary_seen = false;
    // Only particles that have hit an angular body need reaction snapshots.
    // Every other particle stays in the ordinary earliest-event search.
    // SPH substeps change velocities and material state between drifts.
    // Positions and node indices still belong to this same transactional tick.
    let MixedSupportState {
        particles: contacted_particles,
        previous: previous_support,
    } = &mut work.mixed_support;
    'intervals: while remaining > 0. {
        let participants = SupportParticipants::new(&nodes, count, contacted_particles, radius);
        let states = &participants.states;
        let particle_slots = participants.radii.len();
        let zero_rates = vec![crate::contact::ContactWrench::default(); bodies.len()];
        let (mut external_snapshot, mut external_rates) = if supported.is_some() {
            if let Some(recipes) = point_loads {
                let points = recipes
                    .iter()
                    .map(|points| {
                        points
                            .iter()
                            .map(|p| p.shifted(load_offset + dt - remaining))
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| Error::NumericalFailure)?;
                super::support_world::material_point_wrenches(
                    &states[particle_slots..],
                    forces,
                    &zero_rates,
                    &points,
                )?
            } else {
                (forces.to_vec(), zero_rates.clone())
            }
        } else {
            (forces.to_vec(), zero_rates.clone())
        };
        if particle_slots > 0 {
            external_snapshot.splice(
                ..0,
                states[..particle_slots]
                    .iter()
                    .map(|state| crate::contact::ContactWrench {
                        force: particle_acceleration.map(|a| a * state.motion.mass),
                        ..Default::default()
                    }),
            );
            external_rates.splice(..0, std::iter::repeat_n(Default::default(), particle_slots));
        }
        let support_report = if let Some(c) = supported {
            if states.is_empty() {
                None
            } else {
                if ledger.queries >= config.max_queries {
                    return Err(Error::CollisionBudget);
                }
                let limits = DynamicWorldConfig {
                    max_queries: config.max_queries - ledger.queries,
                    ..config
                };
                let report = super::support_world::assemble_reactions(
                    states,
                    &participants.radii,
                    world,
                    &external_snapshot,
                    limits,
                    c.reaction,
                    true,
                    c.max_geometry_error_m,
                )?;
                ledger.queries += report.queries;
                Some(report)
            }
        } else {
            None
        };
        if particle_slots > 0 {
            contacted_particles.retain(|&node| {
                support_report.as_ref().is_some_and(|report| {
                    report
                        .supports
                        .iter()
                        .any(|support| participants.node(support.first) == node)
                })
            });
        }
        let active = support_report
            .as_ref()
            .is_some_and(|r| !r.supports.is_empty());
        if !active {
            horizon = remaining;
        }
        if active && work.supported_intervals >= supported.unwrap().max_intervals {
            return Err(Error::CollisionBudget);
        }
        let mut effective = external_snapshot.clone();
        if let Some(reaction) = support_report.as_ref().and_then(|r| r.reaction.as_ref()) {
            for (w, r) in effective.iter_mut().zip(&reaction.wrenches) {
                for k in 0..3 {
                    w.force[k] += r.force[k];
                    w.torque[k] += r.torque[k];
                }
            }
        }
        let initial_point_motion = if let Some(report) = &support_report {
            report
                .supports
                .iter()
                .zip(&report.geometry)
                .map(|(support, geometry)| {
                    participants.point_motion(
                        world,
                        &effective,
                        &external_rates,
                        *support,
                        *geometry,
                    )
                })
                .collect::<Result<Vec<_>, Error>>()?
        } else {
            Vec::new()
        };
        let support_rate = if let (Some(tolerance), Some(report)) = (
            supported.and_then(|c| c.reaction_jerk_tolerance),
            support_report.as_ref(),
        ) {
            if let Some(baseline) = &report.reaction {
                let motion = report
                    .supports
                    .iter()
                    .zip(&report.geometry)
                    .enumerate()
                    .map(|(index, (s, geometry))| {
                        Ok(crate::contact::SupportMotion {
                            point_velocity: initial_point_motion[index]
                                .map_or(states[s.first].motion.velocity, |m| m.velocity),
                            normal_acceleration: participants
                                .normal_acceleration(world, &effective, *s, *geometry)?,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                let rate = match crate::contact::resolve_rate_from_baseline_mixed_interval(
                    states,
                    particle_slots,
                    &report.supports,
                    &external_snapshot,
                    &external_rates,
                    &motion,
                    crate::contact::ReactionRateConfig {
                        reaction: supported.unwrap().reaction,
                        jerk_tolerance: tolerance,
                    },
                    baseline.clone(),
                    Some(horizon),
                ) {
                    Ok(rate) => rate,
                    Err(crate::contact::Error::Budget)
                        if horizon * 0.5 >= supported.unwrap().min_interval_s =>
                    {
                        horizon *= 0.5;
                        continue 'intervals;
                    }
                    Err(crate::contact::Error::Budget) => return Err(Error::CollisionBudget),
                    Err(crate::contact::Error::InvalidInput) => {
                        return Err(Error::InvalidCollision);
                    }
                    Err(_) => return Err(Error::NumericalFailure),
                };
                // A representable final time remainder may be below min_interval_s.
                // Only a pressure branch limit that truncates the interval uses
                // the minimum subdivision budget; integrate the remainder itself.
                if rate.positive_until_s < horizon
                    && rate.positive_until_s < supported.unwrap().min_interval_s
                {
                    return Err(Error::CollisionBudget);
                }
                horizon = horizon.min(rate.positive_until_s);
                Some(rate)
            } else {
                None
            }
        } else {
            None
        };
        let force_rates: Vec<_> = support_rate.as_ref().map_or_else(
            || vec![[0.; 3]; states.len()],
            |rate| rate.wrenches_rate.iter().map(|w| w.force).collect(),
        );
        let mut final_rates = external_rates.clone();
        if let Some(rate) = &support_rate {
            for (total, reaction) in final_rates.iter_mut().zip(&rate.wrenches_rate) {
                for k in 0..3 {
                    total.force[k] += reaction.force[k];
                    total.torque[k] += reaction.torque[k];
                }
            }
        }
        let point_motion = if let Some(report) = &support_report {
            report
                .supports
                .iter()
                .zip(&report.geometry)
                .enumerate()
                .map(|(index, (support, geometry))| {
                    let motion = participants.point_motion(
                        world,
                        &effective,
                        &final_rates,
                        *support,
                        *geometry,
                    )?;
                    match (initial_point_motion[index], motion) {
                        (None, None) => {}
                        (Some(a), Some(b))
                            if a.velocity == b.velocity && a.acceleration == b.acceleration => {}
                        _ => return Err(Error::InvalidCollision),
                    }
                    Ok(motion)
                })
                .collect::<Result<Vec<_>, Error>>()?
        } else {
            Vec::new()
        };
        let mut reaction_torques: Vec<_> = support_report
            .as_ref()
            .and_then(|r| r.reaction.as_ref())
            .map_or_else(
                || vec![crate::rotation::TorquePolynomial::constant([0.; 3]); states.len()],
                |r| {
                    r.wrenches
                        .iter()
                        .map(|w| crate::rotation::TorquePolynomial::constant(w.torque))
                        .collect()
                },
            );
        if let Some(report) = &support_report
            && let Some(reaction) = &report.reaction
        {
            for (index, (support, force)) in
                report.supports.iter().zip(&reaction.forces).enumerate()
            {
                let force_rate = support_rate
                    .as_ref()
                    .map_or([0.; 3], |r| r.forces_rate[index]);
                for (owner, sign) in [(Some(support.first), 1.), (support.second, -1.)] {
                    let Some(owner) = owner.filter(|&owner| owner >= particle_slots) else {
                        continue;
                    };
                    let arm = std::array::from_fn(|k| {
                        support.support.contact.point[k] - states[owner].motion.position[k]
                    });
                    let velocity = std::array::from_fn(|k| {
                        point_motion[index]
                            .map_or(states[support.first].motion.velocity[k], |m| m.velocity[k])
                            - states[owner].motion.velocity[k]
                    });
                    let acceleration = std::array::from_fn(|k| {
                        point_motion[index].map_or(
                            effective[support.first].force[k] / states[support.first].motion.mass,
                            |m| m.acceleration[k],
                        ) - effective[owner].force[k] / states[owner].motion.mass
                    });
                    let jerk = std::array::from_fn(|k| {
                        point_motion[index].map_or(
                            final_rates[support.first].force[k] / states[support.first].motion.mass,
                            |m| m.jerk[k],
                        ) - final_rates[owner].force[k] / states[owner].motion.mass
                    });
                    let law = crate::rotation::TorquePolynomial::moving_affine_arm(
                        arm,
                        velocity,
                        acceleration,
                        jerk,
                        force.map(|f| sign * f),
                        force_rate.map(|f| sign * f),
                    )
                    .map_err(|_| Error::NumericalFailure)?;
                    for k in 0..3 {
                        reaction_torques[owner].rate[k] += law.rate[k];
                        reaction_torques[owner].acceleration[k] += law.acceleration[k];
                        reaction_torques[owner].jerk[k] += law.jerk[k];
                        reaction_torques[owner].snap[k] += law.snap[k];
                    }
                }
            }
        }
        let pair_supports =
            |i: usize,
             j: Option<usize>|
             -> (Vec<RigidSupportPoint>, Vec<Option<RigidSupportPointMotion>>) {
                let Some(report) = &support_report else {
                    return (Vec::new(), Vec::new());
                };
                let Some(reaction) = &report.reaction else {
                    return (Vec::new(), Vec::new());
                };
                report
                    .supports
                    .iter()
                    .zip(&report.geometry)
                    .zip(&reaction.forces)
                    .enumerate()
                    .filter(|(_, ((s, _), _))| {
                        participants.node(s.first) == i
                            && s.second.map(|slot| participants.node(slot)) == j
                    })
                    .map(|(index, ((_, geometry), force))| {
                        let mut point = *geometry;
                        point.carrying_reaction = *force != [0.; 3]
                            || support_rate
                                .as_ref()
                                .is_some_and(|r| r.forces_rate[index] != [0.; 3]);
                        (point, point_motion[index])
                    })
                    .unzip()
            };
        let mut support_error: f64 = 0.;
        let paths: Vec<_> = nodes
            .iter()
            .enumerate()
            .map(|(index, n)| {
                use crate::rigid_motion::MotionLoad;
                let slot = participants.slot(index);
                let external = if index < count {
                    crate::contact::ContactWrench {
                        force: particle_acceleration.map(|a| a * n.mass),
                        ..Default::default()
                    }
                } else {
                    forces[index - count]
                };
                let reaction = slot
                    .and_then(|slot| {
                        support_report
                            .as_ref()?
                            .reaction
                            .as_ref()?
                            .wrenches
                            .get(slot)
                    })
                    .copied()
                    .unwrap_or_default();
                let loads = [
                    MotionLoad {
                        force: external.force,
                        torque: crate::rotation::TorquePolynomial::constant(external.torque),
                        ..MotionLoad::zero()
                    },
                    MotionLoad {
                        force: reaction.force,
                        force_rate: slot.map_or([0.; 3], |slot| force_rates[slot]),
                        torque: slot.map_or_else(
                            || crate::rotation::TorquePolynomial::constant([0.; 3]),
                            |slot| reaction_torques[slot],
                        ),
                    },
                ];
                let prepared = if let (true, Some(point_loads)) = (index >= count, point_loads) {
                    let points = point_loads[index - count]
                        .iter()
                        .map(|point| point.shifted(load_offset + dt - remaining))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| Error::NumericalFailure)?;
                    n.contact()
                        .prepare_material_load_motion(points, loads, horizon, rotation)
                } else {
                    n.contact().prepare_load_motion(loads, horizon, rotation)
                };
                prepared.map_err(|_| Error::NumericalFailure)
            })
            .collect::<Result<_, _>>()?;
        let interval_duration = horizon;
        let elapsed_s = load_offset + (dt - remaining);
        let mut earliest: Option<CollisionEvent> = None;
        let mut simultaneous = Vec::new();
        let mut admit =
            |first: usize, second: Option<usize>, event: RigidGeometryHit| -> Result<(), Error> {
                let hit = event.contact;
                if event.feature.is_some() && !matches!(hit.geometry, GeometryHit::Contact { .. }) {
                    return Err(Error::InvalidCollision);
                }
                if hit.witness.is_some() && !matches!(hit.geometry, GeometryHit::Contact { .. }) {
                    return Err(Error::InvalidCollision);
                }
                let GeometryHit::Contact { fraction, normal } = hit.geometry else {
                    return if matches!(hit.geometry, GeometryHit::Overlap) {
                        Err(Error::InitialOverlap)
                    } else {
                        Ok(())
                    };
                };
                if hit.witness.is_some_and(|w| {
                    !finite(w.point) || !w.tolerance_m.is_finite() || w.tolerance_m < 0.
                }) {
                    return Err(Error::InvalidCollision);
                }
                if require_witness && hit.witness.is_none() {
                    return Err(Error::InvalidCollision);
                }
                let norm = normal[0].hypot(normal[1]).hypot(normal[2]);
                if !fraction.is_finite()
                    || !(0. ..=1.).contains(&fraction)
                    || !norm.is_finite()
                    || (norm - 1.).abs() > 5e-11
                {
                    return Err(Error::InvalidCollision);
                }
                let normal = normal.map(|n| n / norm);
                let time = interval_duration * fraction;
                let point = hit.witness.map_or(nodes[first].position, |w| w.point);
                let a = paths[first]
                    .sample(time)
                    .map_err(|_| Error::NumericalFailure)?;
                let va = a
                    .point_velocity(point)
                    .map_err(|_| Error::NumericalFailure)?;
                let boundary_velocity = if second.is_some() {
                    [0.; 3]
                } else if first >= count {
                    world.rigid_environment_impact_velocity(
                        first - count,
                        event.feature,
                        elapsed_s + time,
                    )?
                } else {
                    world.particle_environment_impact_velocity(event.feature, elapsed_s + time)?
                };
                if !finite(boundary_velocity) {
                    return Err(Error::InvalidCollision);
                }
                if supported.is_some() && boundary_velocity != [0.; 3] {
                    return Err(Error::CollisionBackend);
                }
                let vb = if let Some(index) = second {
                    paths[index]
                        .sample(time)
                        .map_err(|_| Error::NumericalFailure)?
                        .point_velocity(point)
                        .map_err(|_| Error::NumericalFailure)?
                } else {
                    boundary_velocity
                };
                let relative: [f64; 3] = std::array::from_fn(|k| va[k] - vb[k]);
                let speed: f64 = (0..3).map(|a| relative[a] * normal[a]).sum();
                if !speed.is_finite() || speed >= 0. {
                    return Err(Error::InvalidCollision);
                }
                if earliest.is_none_or(|(_, _, old, ..)| fraction < old) {
                    earliest = Some((
                        first,
                        second,
                        fraction,
                        normal,
                        hit.witness,
                        event.feature,
                        boundary_velocity,
                    ));
                    simultaneous.clear();
                }
                if earliest.is_some_and(|(_, _, old, ..)| fraction == old) {
                    simultaneous.push((
                        first,
                        second,
                        fraction,
                        normal,
                        hit.witness,
                        event.feature,
                        boundary_velocity,
                    ));
                }
                Ok(())
            };
        let mut admitted_support = std::collections::BTreeMap::new();
        for (index, original) in particles.iter().enumerate() {
            let p = Particle {
                position: nodes[index].position,
                velocity: nodes[index].velocity,
                ..*original
            };
            for body_index in 0..bodies.len() {
                charge(ledger, config)?;
                let body_node = count + body_index;
                let (supports, point_models) = pair_supports(index, Some(body_node));
                let event = if supports.is_empty() {
                    if let Some((points, error)) = previous_support.get(&(index, body_node)) {
                        world.sweep_particle_body_after_support(
                            &p,
                            radius,
                            &paths[index],
                            body_index,
                            &paths[body_node],
                            points,
                            *error,
                            config.contact.max_candidates,
                        )?
                    } else {
                        world.sweep_particle_motion_body_event(
                            &p,
                            radius,
                            &paths[index],
                            body_index,
                            &paths[body_node],
                            config.contact.max_candidates,
                        )?
                    }
                } else {
                    match world.sweep_supported_particle_body_point_motion(
                        &paths[index],
                        radius,
                        body_index,
                        &paths[body_node],
                        &supports,
                        &point_models,
                        config.contact.max_candidates,
                    ) {
                        Ok(hit) => {
                            if !hit.support_error_m.is_finite() || hit.support_error_m < 0. {
                                return Err(Error::InvalidCollision);
                            }
                            support_error = support_error.max(hit.support_error_m);
                            admitted_support
                                .insert((index, body_node), (supports, hit.support_error_m));
                            hit.event
                        }
                        Err(Error::CollisionBudget)
                            if horizon * 0.5 >= supported.unwrap().min_interval_s =>
                        {
                            horizon *= 0.5;
                            continue 'intervals;
                        }
                        Err(error) => return Err(error),
                    }
                };
                admit(index, Some(body_node), event)?;
            }
            if world.has_environment() {
                charge(ledger, config)?;
                admit(
                    index,
                    None,
                    world.sweep_particle_motion_environment_event_at(
                        &p,
                        radius,
                        &paths[index],
                        elapsed_s,
                        config.contact.max_candidates,
                    )?,
                )?;
            }
        }
        for first in 0..bodies.len() {
            for second in first + 1..bodies.len() {
                charge(ledger, config)?;
                admit(count + first, Some(count + second), {
                    let (supports, point_models) =
                        pair_supports(count + first, Some(count + second));
                    if supports.is_empty() {
                        world.sweep_rigid_pair_event(
                            first,
                            &paths[count + first],
                            second,
                            &paths[count + second],
                            config.contact.max_candidates,
                        )?
                    } else {
                        match world.sweep_supported_rigid_pair_point_motion(
                            first,
                            &paths[count + first],
                            second,
                            &paths[count + second],
                            &supports,
                            &point_models,
                            config.contact.max_candidates,
                        ) {
                            Ok(hit) => {
                                if !hit.support_error_m.is_finite() || hit.support_error_m < 0. {
                                    return Err(Error::InvalidCollision);
                                }
                                support_error = support_error.max(hit.support_error_m);
                                hit.event
                            }
                            Err(Error::CollisionBudget)
                                if horizon * 0.5 >= supported.unwrap().min_interval_s =>
                            {
                                horizon *= 0.5;
                                continue 'intervals;
                            }
                            Err(e) => return Err(e),
                        }
                    }
                })?;
            }
            if world.has_environment() {
                charge(ledger, config)?;
                admit(count + first, None, {
                    let (supports, point_models) = pair_supports(count + first, None);
                    if supports.is_empty() {
                        world.sweep_rigid_environment_event_at(
                            first,
                            &paths[count + first],
                            elapsed_s,
                            config.contact.max_candidates,
                        )?
                    } else {
                        match world.sweep_supported_rigid_environment_point_motion(
                            first,
                            &paths[count + first],
                            &supports,
                            &point_models,
                            config.contact.max_candidates,
                        ) {
                            Ok(hit) => {
                                if !hit.support_error_m.is_finite() || hit.support_error_m < 0. {
                                    return Err(Error::InvalidCollision);
                                }
                                support_error = support_error.max(hit.support_error_m);
                                hit.event
                            }
                            Err(Error::CollisionBudget)
                                if horizon * 0.5 >= supported.unwrap().min_interval_s =>
                            {
                                horizon *= 0.5;
                                continue 'intervals;
                            }
                            Err(e) => return Err(e),
                        }
                    }
                })?;
            }
        }
        let fraction = earliest.map_or(1., |(_, _, f, ..)| f);
        for (index, n) in nodes.iter_mut().enumerate() {
            let time = interval_duration * fraction;
            let slot = participants.slot(index);
            if let Some(slot) = slot {
                let change = paths[index]
                    .work(time)
                    .map_err(|_| Error::NumericalFailure)?;
                let external = if index < count {
                    external_snapshot[slot]
                } else {
                    forces[index - count]
                };
                let (force, torque) = paths[index]
                    .wrench_work(time, external.force, external.torque)
                    .map_err(|_| Error::NumericalFailure)?;
                let reaction = support_report
                    .as_ref()
                    .and_then(|r| r.reaction.as_ref())
                    .map_or(crate::contact::ContactWrench::default(), |r| {
                        r.wrenches[slot]
                    });
                let (rf, rt) = paths[index]
                    .affine_wrench_work(
                        time,
                        reaction.force,
                        force_rates[slot],
                        reaction_torques[slot],
                    )
                    .map_err(|_| Error::NumericalFailure)?;
                let angular = paths[index]
                    .affine_wrench_angular_impulse(
                        time,
                        reaction.force,
                        force_rates[slot],
                        reaction_torques[slot],
                    )
                    .map_err(|_| Error::NumericalFailure)?;
                for k in 0..3 {
                    work.reaction_angular_impulse[k] += angular[k];
                }
                let mut point_work = 0.;
                if let (true, Some(loads)) = (index >= count, point_loads) {
                    for point in &loads[index - count] {
                        let point = point
                            .shifted(load_offset + dt - remaining)
                            .map_err(|_| Error::NumericalFailure)?;
                        point_work += paths[index]
                            .material_point_force_work(
                                time,
                                point.local,
                                point.force,
                                point.force_rate,
                            )
                            .map_err(|_| Error::NumericalFailure)?
                            .total_work;
                    }
                }
                if index >= count {
                    work.external += force + torque + point_work;
                }
                work.reaction += rf + rt;
                work.residual +=
                    change.kinetic_energy_change - force - torque - point_work - rf - rt;
            }
            if slot.is_some() || particle_acceleration != [0.; 3] {
                n.update(
                    paths[index]
                        .sample(time)
                        .map_err(|_| Error::NumericalFailure)?,
                );
            } else {
                for k in 0..3 {
                    n.position[k] += n.velocity[k] * time;
                }
            }
            if !finite(n.position) || !work.external.is_finite() || !work.residual.is_finite() {
                return Err(Error::NumericalFailure);
            }
        }
        let time = interval_duration * fraction;
        if time > 0. {
            *previous_support = admitted_support;
        }

        if active && time > 0. {
            let report = support_report.as_ref().unwrap();
            work.supported_intervals += 1;
            work.support_points = work
                .support_points
                .checked_add(report.supports.len())
                .ok_or(Error::CollisionBudget)?;
            if work.support_points + ledger.contacts > config.max_contacts {
                return Err(Error::CollisionBudget);
            }
            work.max_support_error_m = work.max_support_error_m.max(support_error);
            if let Some(reaction) = &report.reaction {
                for (index, (support, force)) in
                    report.supports.iter().zip(&reaction.forces).enumerate()
                {
                    if support.second.is_some() {
                        continue;
                    }
                    let path = &paths[participants.node(support.first)];
                    let center = path.initial().motion.position;
                    let point = support.support.contact.point;
                    let opposite_force = force.map(|f| -f);
                    let opposite_rate = support_rate
                        .as_ref()
                        .map_or([0.; 3], |r| r.forces_rate[index].map(|f| -f));
                    let law = crate::rotation::TorquePolynomial::moving_affine_arm(
                        std::array::from_fn(|k| point[k] - center[k]),
                        point_motion[index].map_or([0.; 3], |m| {
                            std::array::from_fn(|k| {
                                m.velocity[k] - path.initial().motion.velocity[k]
                            })
                        }),
                        point_motion[index].map_or([0.; 3], |m| {
                            std::array::from_fn(|k| m.acceleration[k] - path.acceleration()[k])
                        }),
                        point_motion[index].map_or([0.; 3], |m| {
                            std::array::from_fn(|k| m.jerk[k] - path.jerk()[k])
                        }),
                        opposite_force,
                        opposite_rate,
                    )
                    .map_err(|_| Error::NumericalFailure)?;
                    let angular = path
                        .affine_wrench_angular_impulse(time, opposite_force, opposite_rate, law)
                        .map_err(|_| Error::NumericalFailure)?;
                    for k in 0..3 {
                        work.environment_reaction_angular_impulse[k] += angular[k];
                    }
                }
            }
            for k in 0..3 {
                let environment_rate: f64 = support_rate.as_ref().map_or(0., |rate| {
                    report
                        .supports
                        .iter()
                        .zip(&rate.forces_rate)
                        .filter(|(s, _)| s.second.is_none())
                        .map(|(_, f)| -f[k])
                        .sum()
                });
                let impulse = (environment_rate * time * 0.5 + report.environment_force[k]) * time;
                work.environment_reaction_impulse[k] += impulse;
                ledger.environment_impulse[k] += impulse;
            }
            if !work.reaction.is_finite()
                || !finite(work.environment_reaction_impulse)
                || !finite(work.reaction_angular_impulse)
                || !finite(work.environment_reaction_angular_impulse)
                || !(0..3).all(|k| {
                    (work.reaction_angular_impulse[k]
                        + work.environment_reaction_angular_impulse[k])
                        .is_finite()
                })
            {
                return Err(Error::NumericalFailure);
            }
        }
        let next = if supported.is_none() {
            remaining * (1. - fraction)
        } else {
            remaining - time
        };
        if time > 0. && next == remaining {
            return Err(Error::CollisionBudget);
        }
        remaining = next.max(0.);
        horizon = supported.map_or(remaining, |c| remaining.min(c.max_interval_s));
        let Some((first, second, _, normal, witness, feature, boundary_velocity)) = earliest else {
            continue;
        };
        moving_boundary_seen |= simultaneous.iter().any(|event| event.6 != [0.; 3]);
        if moving_boundary_seen
            && supported.is_none()
            && config.contact.restitution == 0.
            && config.contact.friction == 0.
        {
            let impact_time = elapsed_s + time;
            if let Some(other) = second {
                stalls.remove(&first);
                stalls.remove(&other);
            } else {
                let stall = stalls.entry(first).or_default();
                if impact_time - stall.time_s > dt * 1e-8 {
                    *stall = BoundaryStall::default();
                }
                stall.time_s = impact_time;
                stall.hits += 1;
                if boundary_velocity == [0.; 3] {
                    stall.fixed = Some(normal);
                } else if crate::math::dot(normal, boundary_velocity) > 0. {
                    stall.moving = Some((normal, feature));
                }
                if stall.hits >= 8
                    && let Some(((moving, moving_feature), fixed)) = stall.moving.zip(stall.fixed)
                    && crate::math::dot(moving, fixed) < -0.99
                {
                    return Err(Error::KinematicConstraint {
                        body: first.checked_sub(count),
                        feature: moving_feature,
                    });
                }
            }
            // Opposing prescribed normal velocities leave no solution for
            // a translating body between a moving face and a fixed face.
            for &(a, b, _, n, _, moving_feature, moving) in &simultaneous {
                if nodes[a].spin.is_some() || b.is_some() || crate::math::dot(n, moving) <= 0. {
                    continue;
                }
                if simultaneous
                    .iter()
                    .any(|&(other, pair, _, fixed_n, _, _, velocity)| {
                        other == a
                            && pair.is_none()
                            && velocity == [0.; 3]
                            && fixed_n == n.map(|v| -v)
                    })
                {
                    return Err(Error::KinematicConstraint {
                        body: a.checked_sub(count),
                        feature: moving_feature,
                    });
                }
            }
        }
        if first < count
            && moving_boundary_seen
            && supported.is_none()
            && config.contact.restitution == 0.
            && config.contact.friction == 0.
            && second.is_none_or(|index| nodes[index].spin.is_none())
        {
            let impact_time = elapsed_s + time;
            let current = earliest.expect("selected event");
            if let Some(prior) = prior_particle_contacts.remove(&first) {
                let (_, other, _, prior_normal, prior_witness, _, prior_velocity) = prior.event;
                let linked = (second.is_some() && other.is_none() && prior_velocity != [0.; 3])
                    || (other.is_some() && second.is_none() && boundary_velocity != [0.; 3]);
                if linked
                    && crate::math::dot(normal, prior_normal) < -0.99
                    && let Some(witness) = prior_witness
                {
                    let elapsed = impact_time - prior.time;
                    let drift = |current: [f64; 3], previous: [f64; 3]| {
                        (0..3)
                            .map(|axis| (current[axis] - previous[axis]).abs())
                            .sum::<f64>()
                    };
                    let particle_drift = drift(nodes[first].position, prior.particle_position);
                    let other_drift = other.zip(prior.body_position).map_or_else(
                        || elapsed * prior_velocity.iter().map(|v| v.abs()).sum::<f64>(),
                        |(index, previous)| drift(nodes[index].position, previous),
                    );
                    // The previously admitted patch is reusable only while
                    // both participants remain within its witness tolerance.
                    if elapsed >= 0. && particle_drift + other_drift <= witness.tolerance_m {
                        resolve_rigid_events(
                            &mut nodes,
                            count,
                            &[prior.event, current],
                            impact_time,
                            world,
                            config,
                            ledger,
                        )?;
                        continue;
                    }
                }
            }
            if second.is_some() || boundary_velocity != [0.; 3] {
                prior_particle_contacts.insert(
                    first,
                    ParticleContact {
                        event: current,
                        time: impact_time,
                        particle_position: nodes[first].position,
                        body_position: second.map(|index| nodes[index].position),
                    },
                );
            }
        }
        if simultaneous.len() > 1
            && config.contact.restitution == 0.
            && config.contact.friction == 0.
            && simultaneous
                .iter()
                .all(|(first, second, ..)| *first >= count && second.is_none_or(|j| j >= count))
        {
            if ledger.contacts + work.support_points + simultaneous.len() > config.max_contacts {
                return Err(Error::CollisionBudget);
            }
            resolve_rigid_events(
                &mut nodes,
                count,
                &simultaneous,
                elapsed_s + time,
                world,
                config,
                ledger,
            )?;
            continue;
        }
        if ledger.contacts + work.support_points >= config.max_contacts {
            return Err(Error::CollisionBudget);
        }
        ledger.contacts += 1;
        let relative: [f64; 3] = std::array::from_fn(|a| {
            nodes[first].velocity[a] - second.map_or(boundary_velocity[a], |s| nodes[s].velocity[a])
        });
        let reduced = second.map_or(nodes[first].mass, |s| {
            1. / (1. / nodes[first].mass + 1. / nodes[s].mass)
        });
        if !positive(reduced) {
            return Err(Error::NumericalFailure);
        }
        let speed: f64 = (0..3).map(|a| relative[a] * normal[a]).sum();
        let tangent: [f64; 3] = std::array::from_fn(|a| relative[a] - speed * normal[a]);
        let first_contact = nodes[first].contact();
        let second_contact = second.map(|s| nodes[s].contact());
        let point = witness.map_or(nodes[first].position, |w| w.point);
        let mut a = first_contact;
        let mut b = second_contact;
        let mixed_patch = first < count
            && supported.is_some()
            && config.contact.friction == 0.
            && world.supports_particle_body_reactions()
            && b.is_some_and(|body| body.spin.is_some());
        let angular_patch = (first >= count || mixed_patch)
            && config.contact.restitution == 0.
            && (a.spin.is_some() || b.is_some_and(|body| body.spin.is_some()));
        let (impulse, loss, boundary_work) = if angular_patch {
            let witness = witness.ok_or(Error::InvalidCollision)?;
            charge(ledger, config)?;
            let contacts = if mixed_patch {
                let index = second.ok_or(Error::InvalidCollision)?;
                world.particle_body_impact_patch(
                    radius,
                    &a,
                    index - count,
                    b.as_ref().ok_or(Error::InvalidCollision)?,
                    witness,
                    normal,
                    feature,
                    config.contact.max_candidates,
                )?
            } else if let Some(index) = second {
                world.rigid_pair_patch_for_feature(
                    first - count,
                    &a,
                    index - count,
                    b.as_ref().ok_or(Error::InvalidCollision)?,
                    witness,
                    normal,
                    feature,
                    config.contact.max_candidates,
                )?
            } else {
                world.rigid_environment_patch_at(
                    first - count,
                    &a,
                    witness,
                    normal,
                    feature,
                    elapsed_s + time,
                    config.contact.max_candidates,
                )?
            };
            if contacts.len() > 128 {
                return Err(Error::CollisionBudget);
            }
            let mut scale = relative.iter().map(|v| v.abs()).fold(1., f64::max);
            for contact in &contacts {
                for velocity in [
                    a.point_velocity(contact.point),
                    b.map_or(Ok([0.; 3]), |body| body.point_velocity(contact.point)),
                ] {
                    let velocity = velocity.map_err(|_| Error::InvalidCollision)?;
                    scale = velocity.iter().map(|v| v.abs()).fold(scale, f64::max);
                }
            }
            let manifold = crate::contact::ManifoldConfig {
                max_sweeps: 10000,
                velocity_tolerance: 128. * f64::EPSILON * scale,
            };
            let report = if boundary_velocity == [0.; 3] {
                crate::contact::resolve_normal_manifold(&mut a, b.as_mut(), &contacts, manifold)
            } else {
                let mut states = [a];
                let indexed: Vec<_> = contacts
                    .iter()
                    .map(|&contact| crate::contact::NetworkContact {
                        first: 0,
                        second: None,
                        contact,
                    })
                    .collect();
                let report =
                    crate::contact::resolve_normal_contact_network_with_boundary_velocities(
                        &mut states,
                        &indexed,
                        &[boundary_velocity; 128][..indexed.len()],
                        manifold,
                    );
                a = states[0];
                report
            }
            .map_err(|e| match e {
                crate::contact::Error::Budget => Error::CollisionBudget,
                crate::contact::Error::InvalidInput => Error::InvalidCollision,
                crate::contact::Error::NumericalFailure => Error::NumericalFailure,
            })?;
            let total = std::array::from_fn(|k| report.impulses.iter().map(|j| j[k]).sum());
            (
                total,
                (report.boundary_work - report.kinetic_energy_change).max(0.),
                report.boundary_work,
            )
        } else {
            let response = if boundary_velocity == [0.; 3] {
                crate::contact::normal_impulse(
                    &a,
                    b.as_ref(),
                    point,
                    normal,
                    config.contact.restitution,
                )
            } else {
                crate::contact::normal_impulse_against_moving_boundary(
                    &a,
                    boundary_velocity,
                    point,
                    normal,
                    config.contact.restitution,
                )
            }
            .map_err(|_| Error::NumericalFailure)?;
            let loss = response.dissipated_energy
                + if config.contact.friction == 0. {
                    0.
                } else {
                    0.5 * reduced
                        * tangent.iter().map(|v| v * v).sum::<f64>()
                        * config.contact.friction
                        * (2. - config.contact.friction)
                };
            let impulse: [f64; 3] = std::array::from_fn(|k| {
                response.impulse[k] - reduced * config.contact.friction * tangent[k]
            });
            a.apply_point_impulse(point, impulse)
                .map_err(|_| Error::NumericalFailure)?;
            if let Some(body) = &mut b {
                body.apply_point_impulse(point, impulse.map(|j| -j))
                    .map_err(|_| Error::NumericalFailure)?;
            }
            let work = (0..3)
                .map(|k| impulse[k] * boundary_velocity[k])
                .sum::<f64>();
            (impulse, loss, work)
        };
        ledger.loss += loss;
        ledger.boundary_impact_work += boundary_work;
        if let Some(total) = ledger.particle_loss.get_mut(first) {
            *total += loss;
        }
        let reference = second.map_or(nodes[first].position, |s| nodes[s].position);
        let epsilon = 64.
            * f64::EPSILON
            * (0..3)
                .map(|k| normal[k].abs() * nodes[first].position[k].abs().max(reference[k].abs()))
                .fold(1., f64::max);
        nodes[first].update(a);
        if supported.is_some()
            && first < count
            && world.supports_particle_body_reactions()
            && config.contact.restitution == 0.
            && config.contact.friction == 0.
            && b.is_some_and(|body| body.spin.is_some())
        {
            contacted_particles.insert(first);
            if contacted_particles.len() + bodies.len() > 128 {
                return Err(Error::CollisionBudget);
            }
        }
        if let Some(index) = second {
            nodes[index].update(b.ok_or(Error::InvalidCollision)?);
        } else {
            for k in 0..3 {
                ledger.environment_impulse[k] -= impulse[k];
            }
        }
        // A unilateral position bias can place a particle inside the other
        // surface when a moving wall is already supporting it. Keep the
        // sampled contact positions in that event loop.
        if first >= count || !moving_boundary_seen {
            let before = nodes[first].position;
            for k in 0..3 {
                nodes[first].position[k] += normal[k] * epsilon;
            }
            if !previous_support.is_empty() {
                // A position bias changes the separation by at most its displacement.
                // Only the just-resolved pair may carry this adjusted certificate.
                let displacement: f64 = nodes[first]
                    .position
                    .iter()
                    .zip(before)
                    .map(|(after, before)| (after - before).abs())
                    .sum();
                previous_support.retain(|&(particle, body), (_, error)| {
                    if particle == first && Some(body) == second {
                        *error = (*error + displacement) * (1. + 8. * f64::EPSILON);
                        true
                    } else {
                        particle != first && body != first
                    }
                });
            }
        }
        if !nodes
            .iter()
            .all(|n| finite(n.velocity) && finite(n.position))
            || !finite(ledger.environment_impulse)
            || !ledger.loss.is_finite()
            || !ledger.boundary_impact_work.is_finite()
        {
            return Err(Error::NumericalFailure);
        }
    }
    for (p, n) in particles.iter_mut().zip(&nodes) {
        p.position = n.position;
        p.velocity = n.velocity;
    }
    for (b, n) in bodies.iter_mut().zip(&nodes[count..]) {
        *b = n.contact();
    }
    Ok(())
}
fn charge(ledger: &mut ContactLedger, config: DynamicWorldConfig) -> Result<(), Error> {
    if ledger.queries >= config.max_queries {
        return Err(Error::CollisionBudget);
    }
    ledger.queries += 1;
    Ok(())
}

#[path = "multi_body/step_entry.rs"]
mod step_entry;
