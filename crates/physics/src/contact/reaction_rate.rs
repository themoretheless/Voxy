//! Local tangent evolution of the existing unilateral reaction network.
//! Rates are signed on loaded branches; zero-pressure branches remain unilateral.
use super::super::solve_normal_network_constraints_with_bounds;
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportMotion {
    /// Geometry-owned world velocity of the common application point.
    pub point_velocity: Vector,
    /// Required only for SupportPlane::Rate. Must preserve the unit-normal
    /// identity n dot n'' = -|n'|². Other owners derive n'' from their dynamics.
    pub normal_acceleration: Option<Vector>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactionRateConfig {
    pub reaction: ReactionConfig,
    /// Tangent complementarity tolerance, in metres per second cubed.
    pub jerk_tolerance: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkReactionRate {
    pub baseline: NetworkReaction,
    /// World force derivatives, including rotation of the supporting normals.
    pub forces_rate: Vec<Vector>,
    /// COM wrench derivatives including motion of the common application arms.
    pub wrenches_rate: Vec<ContactWrench>,
    pub normal_jerks: Vec<f64>,
    pub sweeps: usize,
    pub jerk_residual: f64,
    /// Earliest zero pressure in the linear strength model; infinity if none.
    /// This is a branch limit, not a geometry or nonlinear interval certificate.
    pub positive_until_s: f64,
}
fn scale(v: Vector, a: f64) -> Vector {
    v.map(|x| x * a)
}
fn normal_motion(
    first: ContactBody,
    second: Option<ContactBody>,
    support: NormalSupport,
    first_load: ContactWrench,
    second_load: Option<ContactWrench>,
    motion: SupportMotion,
) -> Result<(Vector, Vector, Vector), Error> {
    let n = unit_normal(support.contact.normal)?;
    let (rate, acceleration) = match support.plane {
        SupportPlane::World => ([0.; 3], [0.; 3]),
        SupportPlane::First | SupportPlane::Second => {
            let (owner, load) = if support.plane == SupportPlane::First {
                (first, first_load)
            } else {
                (
                    second.ok_or(Error::InvalidInput)?,
                    second_load.unwrap_or_default(),
                )
            };
            let (_, omega, alpha) = free_rates(owner, load)?;
            let rate = cross(omega, n);
            (rate, add(cross(alpha, n), cross(omega, rate)))
        }
        SupportPlane::Rate { normal_rate } => {
            let acceleration = motion.normal_acceleration.ok_or(Error::InvalidInput)?;
            let error = dot(n, acceleration) + dot(normal_rate, normal_rate);
            let magnitude = 1.
                + acceleration[0]
                    .hypot(acceleration[1])
                    .hypot(acceleration[2])
                + dot(normal_rate, normal_rate);
            if !finite(acceleration)
                || !error.is_finite()
                || !magnitude.is_finite()
                || error.abs() > 256. * f64::EPSILON * magnitude
            {
                return Err(Error::InvalidInput);
            }
            (normal_rate, acceleration)
        }
    };
    if !matches!(support.plane, SupportPlane::Rate { .. }) && motion.normal_acceleration.is_some() {
        return Err(Error::InvalidInput);
    }
    if !finite(motion.point_velocity) || !finite(rate) || !finite(acceleration) {
        return Err(Error::InvalidInput);
    }
    Ok((n, rate, acceleration))
}
/// Directional derivative of normal_gap_acceleration along physical body rates
/// and the supplied geometry point/normal motion. Wrench rates are world COM
/// derivatives, not force derivatives at a frozen material point.
pub fn normal_gap_jerk(
    first: &ContactBody,
    second: Option<&ContactBody>,
    support: NormalSupport,
    first_load: ContactWrench,
    second_load: Option<ContactWrench>,
    first_load_rate: ContactWrench,
    second_load_rate: Option<ContactWrench>,
    motion: SupportMotion,
) -> Result<f64, Error> {
    normal_gap_acceleration(first, second, support, first_load, second_load)?;
    if second.is_none() && second_load_rate.is_some() {
        return Err(Error::InvalidInput);
    }
    let (n, nd, ndd) = normal_motion(
        *first,
        second.copied(),
        support,
        first_load,
        second_load,
        motion,
    )?;
    let material = |body: ContactBody,
                    load: ContactWrench,
                    rate: ContactWrench|
     -> Result<(Vector, Vector, Vector, Vector), Error> {
        if !finite(rate.force)
            || !finite(rate.torque)
            || (body.spin.is_none() && rate.torque != [0.; 3])
        {
            return Err(Error::InvalidInput);
        }
        let (a, w, alpha) = free_rates(body, load)?;
        let adot = rate.force.map(|f| f / body.motion.mass);
        let alpha_dot = if let Some(spin) = body.spin {
            let gyroscopic = sub(load.torque, cross(w, spin.angular_momentum));
            let rhs = sub(
                sub(
                    sub(rate.torque, cross(alpha, spin.angular_momentum)),
                    cross(w, load.torque),
                ),
                cross(w, gyroscopic),
            );
            add(
                cross(w, alpha),
                spin.inverse_inertia(rhs)
                    .map_err(|_| Error::NumericalFailure)?,
            )
        } else {
            [0.; 3]
        };
        let r = sub(support.contact.point, body.motion.position);
        let rd = sub(motion.point_velocity, body.motion.velocity);
        let acceleration = add(add(a, cross(alpha, r)), cross(w, cross(w, r)));
        let jerk = add(
            add(add(adot, cross(alpha_dot, r)), cross(alpha, rd)),
            add(
                cross(alpha, cross(w, r)),
                cross(w, add(cross(alpha, r), cross(w, rd))),
            ),
        );
        let velocity = add(body.motion.velocity, cross(w, r));
        let velocity_rate = add(add(a, cross(alpha, r)), cross(w, rd));
        if [acceleration, jerk, velocity, velocity_rate]
            .iter()
            .any(|v| !finite(*v))
        {
            return Err(Error::NumericalFailure);
        }
        Ok((acceleration, jerk, velocity, velocity_rate))
    };
    let a = material(*first, first_load, first_load_rate)?;
    let b = second
        .map(|body| {
            material(
                *body,
                second_load.unwrap_or_default(),
                second_load_rate.unwrap_or_default(),
            )
        })
        .transpose()?
        .unwrap_or(([0.; 3], [0.; 3], [0.; 3], [0.; 3]));
    let result = dot(nd, sub(a.0, b.0))
        + dot(n, sub(a.1, b.1))
        + 2. * dot(ndd, sub(a.2, b.2))
        + 2. * dot(nd, sub(a.3, b.3));
    if !result.is_finite() {
        Err(Error::NumericalFailure)
    } else {
        Ok(result)
    }
}
fn accumulate_rate(
    wrench: &mut ContactWrench,
    body: ContactBody,
    point: Vector,
    point_velocity: Vector,
    force: Vector,
    force_rate: Vector,
) -> Result<(), Error> {
    accumulate(wrench, body, point, force_rate)?;
    wrench.torque = add(
        wrench.torque,
        cross(sub(point_velocity, body.motion.velocity), force),
    );
    if !finite(wrench.torque) {
        return Err(Error::NumericalFailure);
    }
    if body.spin.is_none() && wrench.torque != [0.; 3] {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
/// Solve pressure derivatives with the same indexed mass/inertia kernel.
/// The baseline unilateral solve determines the tangent cone. Loaded points
/// admit signed pressure rates; unloaded points admit only nonnegative births.
/// Moving arms, gyroscopic derivatives and rotating normals enter the bias.
/// This is a local tangent solve, not finite-time constrained integration.
pub fn resolve_normal_reaction_rate_network(
    bodies: &[ContactBody],
    contacts: &[NetworkSupport],
    external: &[ContactWrench],
    external_rate: &[ContactWrench],
    motion: &[SupportMotion],
    config: ReactionRateConfig,
) -> Result<NetworkReactionRate, Error> {
    if external_rate.len() != bodies.len()
        || motion.len() != contacts.len()
        || !config.jerk_tolerance.is_finite()
        || config.jerk_tolerance <= 0.
    {
        return Err(Error::InvalidInput);
    }
    if bodies.iter().zip(external_rate).any(|(body, rate)| {
        !finite(rate.force)
            || !finite(rate.torque)
            || (body.spin.is_none() && rate.torque != [0.; 3])
    }) {
        return Err(Error::InvalidInput);
    }
    let baseline = resolve_normal_reaction_network(bodies, contacts, external, config.reaction)?;
    resolve_rate_from_baseline(
        bodies,
        contacts,
        external,
        external_rate,
        motion,
        config,
        baseline,
    )
}

pub(crate) fn resolve_rate_from_baseline(
    bodies: &[ContactBody],
    contacts: &[NetworkSupport],
    external: &[ContactWrench],
    external_rate: &[ContactWrench],
    motion: &[SupportMotion],
    config: ReactionRateConfig,
    baseline: NetworkReaction,
) -> Result<NetworkReactionRate, Error> {
    resolve_rate_from_baseline_interval(
        bodies,
        contacts,
        external,
        external_rate,
        motion,
        config,
        baseline,
        None,
    )
}

pub(crate) fn resolve_rate_from_baseline_interval(
    bodies: &[ContactBody],
    contacts: &[NetworkSupport],
    external: &[ContactWrench],
    external_rate: &[ContactWrench],
    motion: &[SupportMotion],
    config: ReactionRateConfig,
    baseline: NetworkReaction,
    duration: Option<f64>,
) -> Result<NetworkReactionRate, Error> {
    if duration.is_some_and(|t| !t.is_finite() || t <= 0.)
        || external.len() != bodies.len()
        || external_rate.len() != bodies.len()
        || motion.len() != contacts.len()
        || baseline.forces.len() != contacts.len()
        || baseline.normal_accelerations.len() != contacts.len()
        || baseline.wrenches.len() != bodies.len()
        || !config.jerk_tolerance.is_finite()
        || config.jerk_tolerance <= 0.
        || bodies.iter().zip(external_rate).any(|(b, r)| {
            !finite(r.force) || !finite(r.torque) || (b.spin.is_none() && r.torque != [0.; 3])
        })
    {
        return Err(Error::InvalidInput);
    }
    let total: Vec<_> = external
        .iter()
        .zip(&baseline.wrenches)
        .map(|(a, b)| add_wrench(*a, *b))
        .collect();
    let mut geometry_rates = vec![ContactWrench::default(); bodies.len()];
    let mut normals = Vec::new();
    let mut forces_rate = Vec::new();
    for (i, entry) in contacts.iter().enumerate() {
        let (n, nd, _) = normal_motion(
            bodies[entry.first],
            entry.second.map(|j| bodies[j]),
            entry.support,
            total[entry.first],
            entry.second.map(|j| total[j]),
            motion[i],
        )?;
        let fr = scale(nd, dot(n, baseline.forces[i]));
        normals.push(n);
        forces_rate.push(fr);
        accumulate_rate(
            &mut geometry_rates[entry.first],
            bodies[entry.first],
            entry.support.contact.point,
            motion[i].point_velocity,
            baseline.forces[i],
            fr,
        )?;
        if let Some(j) = entry.second {
            accumulate_rate(
                &mut geometry_rates[j],
                bodies[j],
                entry.support.contact.point,
                motion[i].point_velocity,
                scale(baseline.forces[i], -1.),
                scale(fr, -1.),
            )?;
        }
    }
    let total_rate: Vec<_> = external_rate
        .iter()
        .zip(&geometry_rates)
        .map(|(a, b)| add_wrench(*a, *b))
        .collect();
    let mut rates: Vec<_> = bodies
        .iter()
        .zip(&total_rate)
        .map(|(body, wrench)| {
            let mut state = *body;
            state.motion.velocity = wrench.force.map(|f| f / body.motion.mass);
            if let Some(spin) = &mut state.spin {
                spin.angular_momentum = wrench.torque;
            }
            state
        })
        .collect();
    let mut active = Vec::new();
    let mut indices = Vec::new();
    let mut biases = Vec::new();
    let mut lower = Vec::new();
    for (i, entry) in contacts.iter().enumerate() {
        let strength = dot(normals[i], baseline.forces[i]);
        let speed = dot(
            normals[i],
            sub(
                bodies[entry.first].point_velocity(entry.support.contact.point)?,
                entry.second.map_or(Ok([0.; 3]), |j| {
                    bodies[j].point_velocity(entry.support.contact.point)
                })?,
            ),
        );
        if strength == 0.
            && (speed > config.reaction.normal_velocity_tolerance
                || baseline.normal_accelerations[i] > config.reaction.acceleration_tolerance)
        {
            continue;
        }
        let jerk = normal_gap_jerk(
            &bodies[entry.first],
            entry.second.map(|j| &bodies[j]),
            entry.support,
            total[entry.first],
            entry.second.map(|j| total[j]),
            total_rate[entry.first],
            entry.second.map(|j| total_rate[j]),
            motion[i],
        )?;
        let linear = dot(
            normals[i],
            sub(
                rates[entry.first].point_velocity(entry.support.contact.point)?,
                entry.second.map_or(Ok([0.; 3]), |j| {
                    rates[j].point_velocity(entry.support.contact.point)
                })?,
            ),
        );
        active.push(NetworkContact {
            first: entry.first,
            second: entry.second,
            contact: NormalContact {
                normal: normals[i],
                ..entry.support.contact
            },
        });
        indices.push(i);
        biases.push(jerk - linear);
        lower.push(if strength > 0. {
            duration.map_or(f64::NEG_INFINITY, |t| (-strength / t).next_up())
        } else {
            0.
        });
    }
    let (sweeps, mut residual) = if active.is_empty() {
        (0, 0.)
    } else {
        let solved = solve_normal_network_constraints_with_bounds(
            &mut rates,
            &active,
            ManifoldConfig {
                max_sweeps: config.reaction.max_sweeps,
                velocity_tolerance: config.jerk_tolerance,
            },
            &biases,
            false,
            Some(&lower),
        )?;
        for (i, rate) in indices.iter().zip(solved.impulses) {
            forces_rate[*i] = add(forces_rate[*i], rate);
        }
        (solved.sweeps, solved.velocity_residual)
    };
    let mut wrenches_rate = vec![ContactWrench::default(); bodies.len()];
    let mut positive_until = f64::INFINITY;
    for (i, entry) in contacts.iter().enumerate() {
        accumulate_rate(
            &mut wrenches_rate[entry.first],
            bodies[entry.first],
            entry.support.contact.point,
            motion[i].point_velocity,
            baseline.forces[i],
            forces_rate[i],
        )?;
        if let Some(j) = entry.second {
            accumulate_rate(
                &mut wrenches_rate[j],
                bodies[j],
                entry.support.contact.point,
                motion[i].point_velocity,
                scale(baseline.forces[i], -1.),
                scale(forces_rate[i], -1.),
            )?;
        }
        let pressure = dot(normals[i], baseline.forces[i]);
        let rate = dot(normals[i], forces_rate[i]);
        if rate < 0. {
            positive_until = positive_until.min((pressure / (-rate)).max(0.));
        }
    }
    let total_rate: Vec<_> = external_rate
        .iter()
        .zip(&wrenches_rate)
        .map(|(a, b)| add_wrench(*a, *b))
        .collect();
    let jerks = contacts
        .iter()
        .enumerate()
        .map(|(i, e)| {
            normal_gap_jerk(
                &bodies[e.first],
                e.second.map(|j| &bodies[j]),
                e.support,
                total[e.first],
                e.second.map(|j| total[j]),
                total_rate[e.first],
                e.second.map(|j| total_rate[j]),
                motion[i],
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    for i in indices {
        let loaded =
            dot(normals[i], baseline.forces[i]) > 0. || dot(normals[i], forces_rate[i]) > 0.;
        residual = residual.max(if loaded {
            jerks[i].abs()
        } else {
            (-jerks[i]).max(0.)
        });
    }
    if residual > config.jerk_tolerance {
        return Err(Error::Budget);
    }
    Ok(NetworkReactionRate {
        baseline,
        forces_rate,
        wrenches_rate,
        normal_jerks: jerks,
        sweeps,
        jerk_residual: residual,
        positive_until_s: positive_until,
    })
}

#[cfg(test)]
mod interval_tests {
    use super::*;
    #[test]
    fn interval_bounds_preserve_true_unload_time_and_reject_crossing_it() {
        let bodies = [ContactBody {
            motion: crate::gravity::Body {
                position: [0.; 3],
                velocity: [0.; 3],
                mass: 1.,
            },
            spin: None,
        }];
        let supports = [NetworkSupport {
            first: 0,
            second: None,
            support: NormalSupport {
                contact: NormalContact {
                    point: [0.; 3],
                    normal: [0., 1., 0.],
                },
                plane: SupportPlane::World,
            },
        }];
        let external = [ContactWrench {
            force: [0., -10., 0.],
            torque: [0.; 3],
        }];
        let rate = [ContactWrench {
            force: [0., 20., 0.],
            torque: [0.; 3],
        }];
        let cfg = ReactionRateConfig {
            reaction: ReactionConfig {
                max_sweeps: 128,
                acceleration_tolerance: 1e-11,
                normal_velocity_tolerance: 1e-10,
            },
            jerk_tolerance: 1e-10,
        };
        let baseline =
            resolve_normal_reaction_network(&bodies, &supports, &external, cfg.reaction).unwrap();
        let motion = [SupportMotion {
            point_velocity: [0.; 3],
            normal_acceleration: None,
        }];
        let safe = resolve_rate_from_baseline_interval(
            &bodies,
            &supports,
            &external,
            &rate,
            &motion,
            cfg,
            baseline.clone(),
            Some(0.25),
        )
        .unwrap();
        assert!((safe.forces_rate[0][1] + 20.).abs() < 1e-12);
        assert!((safe.positive_until_s - 0.5).abs() < 1e-12);
        assert_eq!(
            resolve_rate_from_baseline_interval(
                &bodies,
                &supports,
                &external,
                &rate,
                &motion,
                cfg,
                baseline.clone(),
                Some(1.)
            )
            .unwrap_err(),
            Error::Budget
        );
        assert!(
            resolve_rate_from_baseline_interval(
                &bodies,
                &supports,
                &external,
                &rate,
                &motion,
                cfg,
                baseline,
                Some(f64::NAN)
            )
            .is_err()
        );
    }
}
