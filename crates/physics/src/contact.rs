//! Point-contact impulse mechanics, shared by particles and finite rigid bodies.
//! Geometry, event timing and persistent ownership remain with their callers.
use crate::{astrophysics_spin::Spin, gravity::Body};
type Vector = [f64; 3];
mod reaction;
pub use reaction::{
    ContactWrench, NetworkReaction, NetworkReactionRate, NetworkSupport, NormalReaction,
    NormalSupport, ReactionConfig, ReactionRateConfig, SupportMotion, SupportPlane,
    normal_gap_acceleration, normal_gap_jerk, resolve_normal_reaction_network,
    resolve_normal_reaction_rate_network, resolve_normal_reactions,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    NumericalFailure,
    Budget,
}
/// A contact-time snapshot. Position is the center of mass, not a scene pivot.
/// A point particle has no intrinsic spin. Rigid inertia uses the existing Spin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactBody {
    pub motion: Body,
    pub spin: Option<Spin>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalImpulse {
    /// World impulse applied to the first body, opposite to the second/boundary.
    pub impulse: Vector,
    pub dissipated_energy: f64,
    pub relative_normal_speed: f64,
    pub inverse_effective_mass: f64,
}
fn finite(v: Vector) -> bool {
    v.iter().all(|v| v.is_finite())
}
fn dot(a: Vector, b: Vector) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn cross(a: Vector, b: Vector) -> Vector {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|k| a[k] - b[k])
}
impl ContactBody {
    fn validate(self) -> Result<(), Error> {
        if !self.motion.mass.is_finite()
            || self.motion.mass <= 0.
            || !finite(self.motion.position)
            || !finite(self.motion.velocity)
        {
            return Err(Error::InvalidInput);
        }
        if let Some(spin) = self.spin {
            spin.energy().map_err(|_| Error::InvalidInput)?;
        }
        Ok(())
    }
    /// Velocity of the material point at the given world coordinate.
    pub fn point_velocity(self, point: Vector) -> Result<Vector, Error> {
        self.validate()?;
        if !finite(point) {
            return Err(Error::InvalidInput);
        }
        let mut velocity = self.motion.velocity;
        if let Some(spin) = self.spin {
            let rotation = cross(
                spin.angular_velocity()
                    .map_err(|_| Error::NumericalFailure)?,
                sub(point, self.motion.position),
            );
            for k in 0..3 {
                velocity[k] += rotation[k];
            }
        }
        if !finite(velocity) {
            return Err(Error::NumericalFailure);
        }
        Ok(velocity)
    }
    /// Kinetic energy including intrinsic rigid rotation.
    pub fn energy(self) -> Result<f64, Error> {
        self.validate()?;
        let energy = self
            .motion
            .velocity
            .iter()
            .map(|v| (0.5 * self.motion.mass * v) * v)
            .sum::<f64>()
            + self
                .spin
                .map_or(Ok(0.), Spin::energy)
                .map_err(|_| Error::NumericalFailure)?;
        if !energy.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(energy)
    }
    fn inverse_contact_mass(self, point: Vector, normal: Vector) -> Result<f64, Error> {
        let mut inverse = 1. / self.motion.mass;
        if let Some(spin) = self.spin {
            let arm = cross(sub(point, self.motion.position), normal);
            let response = spin
                .inverse_inertia(arm)
                .map_err(|_| Error::NumericalFailure)?;
            inverse += dot(arm, response);
        }
        if !inverse.is_finite() || inverse <= 0. {
            return Err(Error::NumericalFailure);
        }
        Ok(inverse)
    }
    /// Apply a world impulse at a world point; failure preserves the snapshot.
    pub fn apply_point_impulse(&mut self, point: Vector, impulse: Vector) -> Result<(), Error> {
        self.validate()?;
        if !finite(point) || !finite(impulse) {
            return Err(Error::InvalidInput);
        }
        let mut candidate = *self;
        for k in 0..3 {
            candidate.motion.velocity[k] += impulse[k] / candidate.motion.mass;
        }
        if let Some(spin) = &mut candidate.spin {
            let torque = cross(sub(point, candidate.motion.position), impulse);
            for k in 0..3 {
                spin.angular_momentum[k] += torque[k];
            }
        }
        candidate.energy()?;
        *self = candidate;
        Ok(())
    }
}
/// Frictionless normal response at one shared point. Normal points toward first.
/// A missing second body denotes a stationary infinite-mass boundary.
/// Geometry must certify the point and normal; this function does not detect hits.
/// Separating/tangent contacts produce zero impulse. No heat is deposited.
pub fn normal_impulse(
    first: &ContactBody,
    second: Option<&ContactBody>,
    point: Vector,
    normal: Vector,
    restitution: f64,
) -> Result<NormalImpulse, Error> {
    let norm = normal[0].hypot(normal[1]).hypot(normal[2]);
    if !finite(point)
        || !norm.is_finite()
        || (norm - 1.).abs() > 5e-11
        || !restitution.is_finite()
        || !(0. ..=1.).contains(&restitution)
    {
        return Err(Error::InvalidInput);
    }
    let normal = normal.map(|v| v / norm);
    let velocity = first.point_velocity(point)?;
    let other = second.map_or(Ok([0.; 3]), |b| b.point_velocity(point))?;
    let speed = dot(sub(velocity, other), normal);
    let inverse = first.inverse_contact_mass(point, normal)?
        + second.map_or(Ok(0.), |b| b.inverse_contact_mass(point, normal))?;
    if !speed.is_finite() || !inverse.is_finite() || inverse <= 0. {
        return Err(Error::NumericalFailure);
    }
    let approaching = speed.min(0.);
    let strength = -(1. + restitution) * approaching / inverse;
    let result = NormalImpulse {
        impulse: normal.map(|v| v * strength),
        dissipated_energy: 0.5
            * (approaching / inverse)
            * approaching
            * (1. - restitution * restitution),
        relative_normal_speed: speed,
        inverse_effective_mass: inverse,
    };
    if !finite(result.impulse) || !result.dissipated_energy.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok(result)
}
/// Equal and opposite point impulses, committed together after full validation.
pub fn resolve_normal_impact(
    first: &mut ContactBody,
    second: Option<&mut ContactBody>,
    point: Vector,
    normal: Vector,
    restitution: f64,
) -> Result<NormalImpulse, Error> {
    let report = normal_impulse(first, second.as_deref(), point, normal, restitution)?;
    let mut a = *first;
    let mut b = second.as_deref().copied();
    a.apply_point_impulse(point, report.impulse)?;
    if let Some(body) = &mut b {
        body.apply_point_impulse(point, report.impulse.map(|v| -v))?;
    }
    *first = a;
    if let Some(second) = second {
        *second = b.expect("second staged");
    }
    Ok(report)
}

/// Geometry-owned contact point and normal, pointing toward the first body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalContact {
    pub point: Vector,
    pub normal: Vector,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManifoldConfig {
    pub max_sweeps: usize,
    pub velocity_tolerance: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ManifoldImpulse {
    pub impulses: Vec<Vector>,
    pub sweeps: usize,
    pub velocity_residual: f64,
    /// Actual floating kinetic-energy change; no heat deposition is inferred.
    pub kinetic_energy_change: f64,
}

/// Frictionless, inelastic normal manifold for two rigid snapshots or a fixed
/// boundary. Projected coordinate minimization includes translation and spin.
/// All points must be admitted by geometry. Failure leaves both bodies unchanged.
/// This is a velocity solve, not positional correction or a persistent cache.
pub fn resolve_normal_manifold(
    first: &mut ContactBody,
    second: Option<&mut ContactBody>,
    contacts: &[NormalContact],
    config: ManifoldConfig,
) -> Result<ManifoldImpulse, Error> {
    if contacts.len() > 128 {
        return Err(Error::InvalidInput);
    }
    solve_normal_constraints(
        first,
        second,
        contacts,
        config,
        &[0.; 128][..contacts.len()],
        true,
    )
}

/// One geometry-admitted contact in a caller-owned body snapshot array.
/// A missing second body denotes a fixed world boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NetworkContact {
    pub first: usize,
    pub second: Option<usize>,
    pub contact: NormalContact,
}

/// Resolve simultaneous frictionless inelastic contacts sharing arbitrary bodies.
/// All state changes are staged; any failed admission or convergence preserves
/// the complete body array. Geometry and contact identities remain caller-owned.
pub fn resolve_normal_contact_network(
    bodies: &mut [ContactBody],
    contacts: &[NetworkContact],
    config: ManifoldConfig,
) -> Result<ManifoldImpulse, Error> {
    solve_normal_network_constraints(
        bodies,
        contacts,
        config,
        &[0.; 128][..contacts.len().min(128)],
        true,
    )
}

fn solve_normal_constraints(
    first: &mut ContactBody,
    second: Option<&mut ContactBody>,
    contacts: &[NormalContact],
    config: ManifoldConfig,
    biases: &[f64],
    enforce_energy: bool,
) -> Result<ManifoldImpulse, Error> {
    let mut bodies = vec![*first];
    if let Some(body) = second.as_deref() {
        bodies.push(*body);
    }
    let indexed: Vec<_> = contacts
        .iter()
        .map(|contact| NetworkContact {
            first: 0,
            second: (bodies.len() == 2).then_some(1),
            contact: *contact,
        })
        .collect();
    let report =
        solve_normal_network_constraints(&mut bodies, &indexed, config, biases, enforce_energy)?;
    *first = bodies[0];
    if let Some(body) = second {
        *body = bodies[1];
    }
    Ok(report)
}

fn solve_normal_network_constraints(
    bodies: &mut [ContactBody],
    contacts: &[NetworkContact],
    config: ManifoldConfig,
    biases: &[f64],
    enforce_energy: bool,
) -> Result<ManifoldImpulse, Error> {
    solve_normal_network_constraints_with_bounds(
        bodies,
        contacts,
        config,
        biases,
        enforce_energy,
        None,
    )
}

/// Same indexed mass/inertia operator for nonnegative reactions and their
/// signed tangent rates. A finite lower bound marks a unilateral rate branch;
/// negative infinity marks an already loaded branch with a signed derivative.
fn solve_normal_network_constraints_with_bounds(
    bodies: &mut [ContactBody],
    contacts: &[NetworkContact],
    config: ManifoldConfig,
    biases: &[f64],
    enforce_energy: bool,
    lower_bounds: Option<&[f64]>,
) -> Result<ManifoldImpulse, Error> {
    if lower_bounds.is_some_and(|bounds| {
        bounds.len() != contacts.len() || bounds.iter().any(|x| *x != 0. && *x != f64::NEG_INFINITY)
    }) {
        return Err(Error::InvalidInput);
    }
    let lower = |i: usize| lower_bounds.map_or(0., |bounds| bounds[i]);
    if bodies.is_empty()
        || bodies.len() > 128
        || biases.len() != contacts.len()
        || biases.iter().any(|v| !v.is_finite())
        || contacts.is_empty()
        || contacts.len() > 128
        || config.max_sweeps == 0
        || !config.velocity_tolerance.is_finite()
        || config.velocity_tolerance <= 0.
    {
        return Err(Error::InvalidInput);
    }
    let mut staged = bodies.to_vec();
    let mut components: Vec<_> = (0..bodies.len()).collect();
    let root = |parents: &[usize], mut k: usize| {
        while parents[k] != k {
            k = parents[k];
        }
        k
    };
    let mut points = Vec::with_capacity(contacts.len());
    let mut inverse = Vec::with_capacity(contacts.len());
    for indexed in contacts {
        if indexed.first >= staged.len()
            || indexed
                .second
                .is_some_and(|j| j >= staged.len() || j == indexed.first)
        {
            return Err(Error::InvalidInput);
        }
        if let Some(second) = indexed.second {
            let first_root = root(&components, indexed.first);
            let second_root = root(&components, second);
            components[second_root] = first_root;
        }
        let contact = indexed.contact;
        let report = normal_impulse(
            &staged[indexed.first],
            indexed.second.map(|j| &staged[j]),
            contact.point,
            contact.normal,
            0.,
        )?;
        let norm = contact.normal[0]
            .hypot(contact.normal[1])
            .hypot(contact.normal[2]);
        points.push(NetworkContact {
            contact: NormalContact {
                point: contact.point,
                normal: contact.normal.map(|v| v / norm),
            },
            ..*indexed
        });
        inverse.push(report.inverse_effective_mass);
    }
    // Disconnected components retain independent energy guards and loss.
    // An unrelated high-energy body must not hide a small collision's loss.
    let energy = |states: &[ContactBody]| -> Result<Vec<f64>, Error> {
        let mut sums = vec![0.; states.len()];
        for (k, body) in states.iter().enumerate() {
            let group = root(&components, k);
            sums[group] += body.energy()?;
            if !sums[group].is_finite() {
                return Err(Error::NumericalFailure);
            }
        }
        Ok(sums)
    };
    let before = energy(&staged)?;
    let speed =
        |states: &[ContactBody], indexed: NetworkContact, index: usize| -> Result<f64, Error> {
            let contact = indexed.contact;
            let relative = sub(
                states[indexed.first].point_velocity(contact.point)?,
                indexed
                    .second
                    .map_or(Ok([0.; 3]), |j| states[j].point_velocity(contact.point))?,
            );
            let value = dot(relative, contact.normal) + biases[index];
            if !value.is_finite() {
                return Err(Error::NumericalFailure);
            }
            Ok(value)
        };
    let apply =
        |states: &mut [ContactBody], indexed: NetworkContact, delta: f64| -> Result<(), Error> {
            let impulse = indexed.contact.normal.map(|n| n * delta);
            states[indexed.first].apply_point_impulse(indexed.contact.point, impulse)?;
            if let Some(j) = indexed.second {
                states[j].apply_point_impulse(indexed.contact.point, impulse.map(|v| -v))?;
            }
            Ok(())
        };
    // Pair block minimization resolves strongly coupled face points without
    // the slow alternating scalar impulses of a nearly singular contact patch.
    // Diagonal/duplicate singular blocks retain scalar coordinate updates.
    let coupling = |body: ContactBody, i: NormalContact, j: NormalContact| -> Result<f64, Error> {
        let mut value = dot(i.normal, j.normal) / body.motion.mass;
        if let Some(spin) = body.spin {
            let ri = cross(sub(i.point, body.motion.position), i.normal);
            let rj = cross(sub(j.point, body.motion.position), j.normal);
            value += dot(
                ri,
                spin.inverse_inertia(rj)
                    .map_err(|_| Error::NumericalFailure)?,
            );
        }
        if !value.is_finite() {
            return Err(Error::NumericalFailure);
        }
        Ok(value)
    };
    let mut cross_mass = vec![vec![0.; points.len()]; points.len()];
    for i in 0..points.len() {
        for j in (i + 1)..points.len() {
            let participants =
                |point: NetworkContact| [Some((point.first, 1.)), point.second.map(|j| (j, -1.))];
            let mut value = 0.;
            for (body, sign_i) in participants(points[i]).into_iter().flatten() {
                for (other, sign_j) in participants(points[j]).into_iter().flatten() {
                    if body == other {
                        value += sign_i
                            * sign_j
                            * coupling(staged[body], points[i].contact, points[j].contact)?;
                    }
                }
            }
            if !value.is_finite() {
                return Err(Error::NumericalFailure);
            }
            cross_mass[i][j] = value;
        }
    }
    let mut strengths = vec![0.; points.len()];
    for sweep in 1..=config.max_sweeps {
        for (index, contact) in points.iter().copied().enumerate() {
            let next = (strengths[index] - speed(&staged, contact, index)? / inverse[index])
                .max(lower(index));
            if !next.is_finite() {
                return Err(Error::NumericalFailure);
            }
            let delta = next - strengths[index];
            apply(&mut staged, contact, delta)?;
            strengths[index] = next;
        }
        for i in 0..points.len() {
            for j in (i + 1)..points.len() {
                let si = inverse[i].sqrt();
                let sj = inverse[j].sqrt();
                let correlation = (cross_mass[i][j] / si) / sj;
                if !correlation.is_finite() || correlation.abs() > 1. + 256. * f64::EPSILON {
                    return Err(Error::NumericalFailure);
                }
                let correlation = correlation.clamp(-1., 1.);
                let determinant = 1. - correlation * correlation;
                if determinant <= 256. * f64::EPSILON {
                    continue;
                }
                let u = strengths[i] * si;
                let v = strengths[j] * sj;
                let ri = u + correlation * v - speed(&staged, points[i], i)? / si;
                let rj = v + correlation * u - speed(&staged, points[j], j)? / sj;
                if !ri.is_finite() || !rj.is_finite() {
                    return Err(Error::NumericalFailure);
                }
                let cost = |u: f64, v: f64| {
                    0.5 * u * u + 0.5 * v * v + correlation * u * v - ri * u - rj * v
                };
                let mut best = if lower_bounds.is_none() {
                    (0., 0., 0.)
                } else {
                    (0., 0., f64::INFINITY)
                };
                let candidates = [
                    (
                        ri.max(lower(i) * si),
                        if lower(j).is_finite() {
                            lower(j) * sj
                        } else {
                            f64::NAN
                        },
                    ),
                    (
                        if lower(i).is_finite() {
                            lower(i) * si
                        } else {
                            f64::NAN
                        },
                        rj.max(lower(j) * sj),
                    ),
                    (
                        (ri - correlation * rj) / determinant,
                        (rj - correlation * ri) / determinant,
                    ),
                ];
                // Finite bounds here are zero; these edge candidates preserve
                // the old unilateral block solve exactly when no rates are used.
                for (x, y) in candidates {
                    if x >= lower(i) * si && y >= lower(j) * sj && x.is_finite() && y.is_finite() {
                        let value = cost(x, y);
                        if value.is_finite() && value < best.2 {
                            best = (x, y, value);
                        }
                    }
                }
                for (index, next) in [(i, best.0 / si), (j, best.1 / sj)] {
                    if !next.is_finite() {
                        return Err(Error::NumericalFailure);
                    }
                    let delta = next - strengths[index];
                    apply(&mut staged, points[index], delta)?;
                    strengths[index] = next;
                }
            }
        }
        let mut residual = 0_f64;
        for (index, contact) in points.iter().copied().enumerate() {
            let velocity = speed(&staged, contact, index)?;
            residual = residual.max(if strengths[index] > lower(index) {
                velocity.abs()
            } else {
                (-velocity).max(0.)
            });
        }
        if residual <= config.velocity_tolerance {
            let after = energy(&staged)?;
            let mut change = 0.;
            for (old, new) in before.iter().zip(after) {
                let delta = new - old;
                if !delta.is_finite()
                    || (enforce_energy && delta > 128. * f64::EPSILON * (1. + old))
                {
                    return Err(Error::NumericalFailure);
                }
                change += delta;
            }
            if !change.is_finite() {
                return Err(Error::NumericalFailure);
            }
            let report = ManifoldImpulse {
                impulses: points
                    .iter()
                    .zip(strengths.iter())
                    .map(|(contact, strength)| contact.contact.normal.map(|n| n * strength))
                    .collect(),
                sweeps: sweep,
                velocity_residual: residual,
                kinetic_energy_change: change,
            };
            bodies.copy_from_slice(&staged);
            return Ok(report);
        }
    }
    Err(Error::Budget)
}

pub(crate) use reaction::resolve_rate_from_baseline;
