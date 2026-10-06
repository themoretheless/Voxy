//! Instantaneous normal reactions for a material point against a rotating plane.
//! Geometry supplies the admitted point and normal; interval evolution stays with
//! the caller. No position correction, guessed stiffness or heat is introduced.
use super::{
    ContactBody, Error, ManifoldConfig, NetworkContact, NormalContact, Vector, cross, dot, finite,
    solve_normal_network_constraints, sub,
};

/// Explicit owner of the supporting normal. Geometry, not the contact solver,
/// chooses the face/feature branch and its owner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SupportPlane {
    First,
    Second,
    World,
    /// Geometry-owned derivative of the unit normal, in inverse seconds.
    /// Supports edge/edge features whose normal belongs to neither body alone.
    Rate {
        normal_rate: Vector,
    },
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalSupport {
    pub contact: NormalContact,
    pub plane: SupportPlane,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ContactWrench {
    /// World force at COM, in newtons.
    pub force: Vector,
    /// World torque about COM, in newton metres.
    pub torque: Vector,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactionConfig {
    pub max_sweeps: usize,
    /// Complementarity tolerance in metres per second squared.
    pub acceleration_tolerance: f64,
    /// Only contacts within this normal-speed tolerance can carry a reaction.
    /// A faster approaching point requires an impact solve first.
    pub normal_velocity_tolerance: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NormalReaction {
    /// World force on the first body at each input contact; inactive points are zero.
    pub forces: Vec<Vector>,
    pub first_wrench: ContactWrench,
    pub second_wrench: Option<ContactWrench>,
    /// Supporting-plane gap accelerations after reactions, in input order.
    pub normal_accelerations: Vec<f64>,
    pub sweeps: usize,
    pub acceleration_residual: f64,
    /// Sum f dot (v_first(point)-v_second(point)), in watts. This is
    /// instantaneous constraint power, not integrated work or dissipated heat.
    pub instantaneous_power: f64,
}
fn add(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|k| a[k] + b[k])
}
fn free_rates(body: ContactBody, wrench: ContactWrench) -> Result<(Vector, Vector, Vector), Error> {
    body.validate()?;
    if !finite(wrench.force)
        || !finite(wrench.torque)
        || (body.spin.is_none() && wrench.torque != [0.; 3])
    {
        return Err(Error::InvalidInput);
    }
    let acceleration = wrench.force.map(|f| f / body.motion.mass);
    let (omega, alpha) = if let Some(spin) = body.spin {
        let omega = spin
            .angular_velocity()
            .map_err(|_| Error::NumericalFailure)?;
        let alpha = spin
            .inverse_inertia(sub(wrench.torque, cross(omega, spin.angular_momentum)))
            .map_err(|_| Error::NumericalFailure)?;
        (omega, alpha)
    } else {
        ([0.; 3], [0.; 3])
    };
    if !finite(acceleration) || !finite(omega) || !finite(alpha) {
        return Err(Error::NumericalFailure);
    }
    Ok((acceleration, omega, alpha))
}
fn unit_normal(normal: Vector) -> Result<Vector, Error> {
    let norm = normal[0].hypot(normal[1]).hypot(normal[2]);
    if !finite(normal) || !norm.is_finite() || norm <= 0. {
        return Err(Error::InvalidInput);
    }
    Ok(normal.map(|n| n / norm))
}
/// Second derivative of the normal gap on the supplied supporting-plane branch.
/// Body-owned planes rotate with their explicit owner; World holds the normal
/// fixed and follows both material points. A Second plane requires a second body.
/// Includes gyroscopic acceleration, centrifugal and rotating-plane Coriolis terms.
/// # Errors
/// Invalid snapshots, points, normals, external wrenches or nonfinite arithmetic.
pub fn normal_gap_acceleration(
    first: &ContactBody,
    second: Option<&ContactBody>,
    support: NormalSupport,
    first_external: ContactWrench,
    second_external: Option<ContactWrench>,
) -> Result<f64, Error> {
    let contact = support.contact;
    if !finite(contact.point) || (second.is_none() && second_external.is_some()) {
        return Err(Error::InvalidInput);
    }
    let n = unit_normal(contact.normal)?;
    let first_rates = free_rates(*first, first_external)?;
    let second_rates = second
        .map(|body| free_rates(*body, second_external.unwrap_or_default()))
        .transpose()?;
    let rate = match support.plane {
        SupportPlane::First => cross(first_rates.1, n),
        SupportPlane::Second => cross(second_rates.ok_or(Error::InvalidInput)?.1, n),
        SupportPlane::World => [0.; 3],
        SupportPlane::Rate { normal_rate } => {
            let tangent = dot(n, normal_rate);
            let scale = normal_rate.iter().map(|v| v.abs()).fold(1., f64::max);
            if !finite(normal_rate)
                || !tangent.is_finite()
                || tangent.abs() > 128. * f64::EPSILON * scale
            {
                return Err(Error::InvalidInput);
            }
            normal_rate
        }
    };
    let material = |body: ContactBody, rates: (Vector, Vector, Vector)| {
        let (acceleration, omega, alpha) = rates;
        let arm = sub(contact.point, body.motion.position);
        add(
            add(acceleration, cross(alpha, arm)),
            cross(omega, cross(omega, arm)),
        )
    };
    let acceleration = sub(
        material(*first, first_rates),
        second.map_or([0.; 3], |body| material(*body, second_rates.unwrap())),
    );
    let relative = sub(
        first.point_velocity(contact.point)?,
        second.map_or(Ok([0.; 3]), |body| body.point_velocity(contact.point))?,
    );
    // Both material points coincide at the admitted contact. The n'' dot gap
    // term is zero, leaving material accelerations and 2 n' dot relative velocity.
    let result = dot(n, acceleration) + 2. * dot(rate, relative);
    if !finite(acceleration) || !finite(rate) || !result.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok(result)
}
fn virtual_rates(body: ContactBody, wrench: ContactWrench) -> Result<ContactBody, Error> {
    let (linear, omega, _) = free_rates(body, wrench)?;
    let mut result = body;
    result.motion.velocity = linear;
    if let Some(spin) = &mut result.spin {
        // This represents angular acceleration through the existing inverse
        // inertia operator. These rate snapshots never replace physical states.
        spin.angular_momentum = sub(wrench.torque, cross(omega, spin.angular_momentum));
    }
    Ok(result)
}
fn accumulate(
    wrench: &mut ContactWrench,
    body: ContactBody,
    point: Vector,
    force: Vector,
) -> Result<(), Error> {
    wrench.force = add(wrench.force, force);
    wrench.torque = add(
        wrench.torque,
        cross(sub(point, body.motion.position), force),
    );
    if !finite(wrench.force) || !finite(wrench.torque) {
        return Err(Error::NumericalFailure);
    }
    if body.spin.is_none() && wrench.torque != [0.; 3] {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
/// Solve nonnegative normal reactions at an admitted resting patch. Geometry
/// must explicitly identify the supporting plane owner for each normal.
/// Outgoing points carry zero reaction; approaching points must be resolved first.
/// Reuses the same contact-mass/block complementarity solver as normal impacts.
/// Inputs stay immutable, including on late solve failure. This is an instantaneous
/// acceleration solve, not an accepted constrained trajectory over a finite step.
/// A no-Spin point body cannot absorb an off-center intrinsic reaction torque.
pub fn resolve_normal_reactions(
    first: &ContactBody,
    second: Option<&ContactBody>,
    contacts: &[NormalSupport],
    first_external: ContactWrench,
    second_external: Option<ContactWrench>,
    config: ReactionConfig,
) -> Result<NormalReaction, Error> {
    if second.is_none() && second_external.is_some() {
        return Err(Error::InvalidInput);
    }
    let mut bodies = vec![*first];
    let mut external = vec![first_external];
    if let Some(body) = second {
        bodies.push(*body);
        external.push(second_external.unwrap_or_default());
    }
    let indexed: Vec<_> = contacts
        .iter()
        .map(|support| NetworkSupport {
            first: 0,
            second: second.map(|_| 1),
            support: *support,
        })
        .collect();
    let result = resolve_normal_reaction_network(&bodies, &indexed, &external, config)?;
    Ok(NormalReaction {
        forces: result.forces,
        first_wrench: result.wrenches[0],
        second_wrench: second.map(|_| result.wrenches[1]),
        normal_accelerations: result.normal_accelerations,
        sweeps: result.sweeps,
        acceleration_residual: result.acceleration_residual,
        instantaneous_power: result.instantaneous_power,
    })
}

/// Geometry-admitted support between indexed caller-owned bodies, or the world.
/// Plane ownership is relative to this support's first and second body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NetworkSupport {
    pub first: usize,
    pub second: Option<usize>,
    pub support: NormalSupport,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkReaction {
    /// World forces on the first participant of each input support.
    pub forces: Vec<Vector>,
    /// Total reciprocal reaction force and COM torque for each input body.
    pub wrenches: Vec<ContactWrench>,
    pub normal_accelerations: Vec<f64>,
    pub sweeps: usize,
    pub acceleration_residual: f64,
    /// Instantaneous sum of force times relative point velocity, in watts.
    pub instantaneous_power: f64,
}

/// Solve simultaneous nonnegative reactions for a complete admitted support
/// network. Shared bodies couple every incident constraint through their mass
/// and world inertia. Inputs are read-only; no pairwise force superposition or
/// physical-state replacement is performed. This remains an instantaneous solve,
/// not a geometry branch or finite-interval trajectory certificate.
pub fn resolve_normal_reaction_network(
    bodies: &[ContactBody],
    contacts: &[NetworkSupport],
    external: &[ContactWrench],
    config: ReactionConfig,
) -> Result<NetworkReaction, Error> {
    if bodies.is_empty()
        || bodies.len() > 128
        || external.len() != bodies.len()
        || contacts.is_empty()
        || contacts.len() > 128
        || config.max_sweeps == 0
        || !config.acceleration_tolerance.is_finite()
        || config.acceleration_tolerance <= 0.
        || !config.normal_velocity_tolerance.is_finite()
        || config.normal_velocity_tolerance <= 0.
    {
        return Err(Error::InvalidInput);
    }
    let mut rates = bodies
        .iter()
        .zip(external)
        .map(|(body, wrench)| virtual_rates(*body, *wrench))
        .collect::<Result<Vec<_>, _>>()?;
    let mut active = Vec::new();
    let mut indices = Vec::new();
    let mut biases = Vec::new();
    for (index, entry) in contacts.iter().enumerate() {
        if entry.first >= bodies.len()
            || entry
                .second
                .is_some_and(|j| j >= bodies.len() || j == entry.first)
        {
            return Err(Error::InvalidInput);
        }
        let first = &bodies[entry.first];
        let second = entry.second.map(|j| &bodies[j]);
        let support = entry.support;
        let normal = unit_normal(support.contact.normal)?;
        let contact = NormalContact {
            normal,
            ..support.contact
        };
        let relative = sub(
            first.point_velocity(contact.point)?,
            second.map_or(Ok([0.; 3]), |body| body.point_velocity(contact.point))?,
        );
        let speed = dot(normal, relative);
        if !speed.is_finite() {
            return Err(Error::NumericalFailure);
        }
        if speed < -config.normal_velocity_tolerance {
            return Err(Error::InvalidInput);
        }
        let acceleration = normal_gap_acceleration(
            first,
            second,
            NormalSupport {
                contact,
                plane: support.plane,
            },
            external[entry.first],
            entry.second.map(|j| external[j]),
        )?;
        if speed > config.normal_velocity_tolerance {
            continue;
        }
        let linear = dot(
            normal,
            sub(
                rates[entry.first].point_velocity(contact.point)?,
                entry
                    .second
                    .map_or(Ok([0.; 3]), |j| rates[j].point_velocity(contact.point))?,
            ),
        );
        let bias = acceleration - linear;
        if !bias.is_finite() {
            return Err(Error::NumericalFailure);
        }
        active.push(NetworkContact {
            first: entry.first,
            second: entry.second,
            contact,
        });
        indices.push(index);
        biases.push(bias);
    }
    let mut forces = vec![[0.; 3]; contacts.len()];
    let (sweeps, residual) = if active.is_empty() {
        (0, 0.)
    } else {
        let report = solve_normal_network_constraints(
            &mut rates,
            &active,
            ManifoldConfig {
                max_sweeps: config.max_sweeps,
                velocity_tolerance: config.acceleration_tolerance,
            },
            &biases,
            false,
        )?;
        for (index, force) in indices.iter().zip(report.impulses) {
            forces[*index] = force;
        }
        (report.sweeps, report.velocity_residual)
    };
    let mut wrenches = vec![ContactWrench::default(); bodies.len()];
    let mut power = 0.;
    for (entry, force) in contacts.iter().zip(&forces) {
        let contact = entry.support.contact;
        accumulate(
            &mut wrenches[entry.first],
            bodies[entry.first],
            contact.point,
            *force,
        )?;
        if let Some(j) = entry.second {
            accumulate(
                &mut wrenches[j],
                bodies[j],
                contact.point,
                force.map(|f| -f),
            )?;
        }
        let relative = sub(
            bodies[entry.first].point_velocity(contact.point)?,
            entry
                .second
                .map_or(Ok([0.; 3]), |j| bodies[j].point_velocity(contact.point))?,
        );
        power += dot(*force, relative);
    }
    let normal_accelerations = contacts
        .iter()
        .map(|entry| {
            normal_gap_acceleration(
                &bodies[entry.first],
                entry.second.map(|j| &bodies[j]),
                entry.support,
                add_wrench(external[entry.first], wrenches[entry.first]),
                entry.second.map(|j| add_wrench(external[j], wrenches[j])),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !power.is_finite() {
        return Err(Error::NumericalFailure);
    }
    let mut checked_residual = residual;
    for index in indices {
        let force = forces[index];
        let strength = force[0].hypot(force[1]).hypot(force[2]);
        checked_residual = checked_residual.max(if strength > 0. {
            normal_accelerations[index].abs()
        } else {
            (-normal_accelerations[index]).max(0.)
        });
    }
    if checked_residual > config.acceleration_tolerance {
        return Err(Error::Budget);
    }
    Ok(NetworkReaction {
        forces,
        wrenches,
        normal_accelerations,
        sweeps,
        acceleration_residual: checked_residual,
        instantaneous_power: power,
    })
}
fn add_wrench(a: ContactWrench, b: ContactWrench) -> ContactWrench {
    ContactWrench {
        force: add(a.force, b.force),
        torque: add(a.torque, b.torque),
    }
}

#[path = "reaction_rate.rs"]
mod rate;
pub use rate::{
    NetworkReactionRate, ReactionRateConfig, SupportMotion, normal_gap_jerk,
    resolve_normal_reaction_rate_network,
};

pub(crate) use rate::resolve_rate_from_baseline;
