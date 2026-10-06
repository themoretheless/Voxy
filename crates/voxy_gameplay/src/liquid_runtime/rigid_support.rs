//! Query-local affine SAT feature keys and contact-time supporting branches.
use crate::convex::{AffineBox, AxisFeature, axis_direction, rotate_vector};
use glam::{DQuat, DVec3};
use physics::{
    contact::{ContactBody, NormalContact, NormalSupport, SupportPlane},
    liquid::{ContactWitness, Error},
};
/// Pose-only support query in the second shape's rigid frame. The selected SAT
/// feature is replayed by the same branch/patch callbacks used after impacts.
pub(super) fn snapshot_with_error(
    first: ContactBody,
    first_shape: AffineBox,
    second: Option<ContactBody>,
    second_shape: AffineBox,
    error_m: f64,
) -> Result<Vec<(ContactWitness, [f64; 3], AxisFeature)>, Error> {
    first.energy().map_err(|_| Error::InvalidCollision)?;
    if let Some(second) = second {
        second.energy().map_err(|_| Error::InvalidCollision)?;
    }
    let qa = first
        .spin
        .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
    let qb = second
        .and_then(|b| b.spin)
        .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
    let ca = DVec3::from_array(first.motion.position);
    let cb = second.map_or(DVec3::ZERO, |b| DVec3::from_array(b.motion.position));
    let local = AffineBox {
        center: rotate_vector(
            qb.conjugate(),
            ca - cb + rotate_vector(qa, first_shape.center),
        ),
        edges: first_shape
            .edges
            .map(|e| rotate_vector(qb.conjugate(), rotate_vector(qa, e))),
    };
    let contacts = if error_m == 0. {
        second_shape.support_contacts(&local)
    } else {
        second_shape.support_contacts_with_error(&local, error_m)
    };
    let contacts = contacts.map_err(|e| {
        if matches!(e, crate::PhysicsError::InitialOverlap) {
            Error::InitialOverlap
        } else {
            Error::InvalidCollision
        }
    })?;
    contacts
        .into_iter()
        .map(|(contact, feature)| {
            let point = cb + rotate_vector(qb, contact.point);
            let normal = rotate_vector(qb, contact.normal);
            let tolerance_m = contact.tolerance
                + 32. * f64::EPSILON * (1. + cb.abs().max_element() + point.abs().max_element());
            if !point.is_finite() || !normal.is_finite() || !tolerance_m.is_finite() {
                return Err(Error::InvalidCollision);
            }
            Ok((
                ContactWitness {
                    point: point.to_array(),
                    tolerance_m,
                },
                normal.to_array(),
                feature,
            ))
        })
        .collect()
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FeatureKey {
    pub first: usize,
    pub second: usize,
    pub axis: AxisFeature,
}
impl FeatureKey {
    pub fn encode(self) -> Result<u64, Error> {
        const MASK: usize = (1 << 28) - 1;
        if self.first > MASK || self.second > MASK {
            return Err(Error::CollisionBudget);
        }
        Ok((self.axis.code() as u64) | ((self.first as u64) << 8) | ((self.second as u64) << 36))
    }
    pub fn decode(token: u64) -> Result<Self, Error> {
        Ok(Self {
            first: ((token >> 8) & ((1 << 28) - 1)) as usize,
            second: (token >> 36) as usize,
            axis: AxisFeature::from_code(token as u8).ok_or(Error::InvalidCollision)?,
        })
    }
}
/// Re-evaluate the selected feature on post-impact momentum at the admitted pose.
/// Shape/axis identities come from the query; a normal is never used to choose them.
pub(super) fn plane(
    first: ContactBody,
    first_shape: AffineBox,
    second: Option<ContactBody>,
    second_shape: AffineBox,
    feature: AxisFeature,
    normal: [f64; 3],
) -> Result<SupportPlane, Error> {
    first.energy().map_err(|_| Error::InvalidCollision)?;
    if let Some(body) = second {
        body.energy().map_err(|_| Error::InvalidCollision)?;
    }
    let qa = first
        .spin
        .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
    let qb = second
        .and_then(|b| b.spin)
        .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
    if (qa.length_squared() - 1.).abs() > 64. * f64::EPSILON
        || (qb.length_squared() - 1.).abs() > 64. * f64::EPSILON
    {
        return Err(Error::InvalidCollision);
    }
    // Replay the query's second-body frame, preserving cross-product conditioning.
    let a = first_shape
        .edges
        .map(|edge| axis_direction(qb.conjugate() * (qa * edge)));
    let b = second_shape.edges.map(axis_direction);
    let raw = match feature {
        AxisFeature::BodyFace(k) => a[((k + 1) % 3) as usize].cross(a[((k + 2) % 3) as usize]),
        AxisFeature::ObstacleFace(k) => b[((k + 1) % 3) as usize].cross(b[((k + 2) % 3) as usize]),
        AxisFeature::Edges(i, j) => a[i as usize].cross(b[j as usize]),
    };
    let magnitude = raw.x.hypot(raw.y).hypot(raw.z);
    let n = DVec3::from_array(normal);
    if !raw.is_finite()
        || magnitude <= 0.
        || !magnitude.is_finite()
        || !n.is_finite()
        || (n.length_squared() - 1.).abs() > 5e-11
    {
        return Err(Error::InvalidCollision);
    }
    let local = axis_direction(raw);
    let expected = axis_direction(qb * local);
    let sign = if expected.dot(n) < 0. { -1. } else { 1. };
    let guard = 1024. * f64::EPSILON / magnitude;
    // Reject a direction whose source cannot be resolved within a meaningful
    // floating angle guard; do not reinterpret a nearly parallel edge pair.
    if !guard.is_finite() || guard >= 0.25 || (expected * sign - n).abs().max_element() > guard {
        return Err(Error::InvalidCollision);
    }
    Ok(match feature {
        AxisFeature::BodyFace(_) => SupportPlane::First,
        AxisFeature::ObstacleFace(_) => {
            if second.is_some() {
                SupportPlane::Second
            } else {
                SupportPlane::World
            }
        }
        AxisFeature::Edges(i, j) => {
            let omega = |body: Option<ContactBody>| -> Result<DVec3, Error> {
                body.and_then(|b| b.spin).map_or(Ok(DVec3::ZERO), |spin| {
                    spin.angular_velocity()
                        .map(DVec3::from_array)
                        .map_err(|_| Error::CollisionBackend)
                })
            };
            let wa = omega(Some(first))?;
            let wb = omega(second)?;
            let relative = qb.conjugate() * (wa - wb);
            let derivative = relative.cross(a[i as usize]).cross(b[j as usize]);
            let local_rate = (derivative - local * local.dot(derivative)) / magnitude;
            let mut rate = wb.cross(n) + qb * (local_rate * sign);
            rate -= n * n.dot(rate);
            if !rate.is_finite() {
                return Err(Error::CollisionBackend);
            }
            SupportPlane::Rate {
                normal_rate: rate.to_array(),
            }
        }
    })
}
pub(super) fn attach(contacts: Vec<NormalContact>, plane: SupportPlane) -> Vec<NormalSupport> {
    contacts
        .into_iter()
        .map(|contact| NormalSupport { contact, plane })
        .collect()
}

/// Range of a nominal constant-acceleration displacement over the full prefix.
fn polynomial_range(v: f64, a: f64, duration: f64) -> Result<(f64, f64), Error> {
    let end = (a * (0.5 * duration) + v) * duration;
    let mut lo = end.min(0.);
    let mut hi = end.max(0.);
    if a != 0. {
        let t = -v / a;
        if t > 0. && t < duration {
            let value = (a * (0.5 * t) + v) * t;
            lo = lo.min(value);
            hi = hi.max(value);
        }
    }
    if !lo.is_finite() || !hi.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok((lo, hi))
}
fn angular_excursion(path: &physics::rigid_motion::RigidMotion, radius: f64) -> Result<f64, Error> {
    let angle = path.rotation().map_or(0., |rotation| {
        rotation
            .segments()
            .iter()
            .map(|s| DVec3::from_array(s.arc.angular_velocity()).length() * (s.end_s - s.start_s))
            .sum::<f64>()
    });
    let result = angle * radius * (1. + 512. * f64::EPSILON);
    if !result.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok(result)
}

/// Admit a constant world-arm reaction model against actual nominal motion.
/// The whole parabolic COM interval and every spin arc are bounded, not just
/// endpoints. Significant evolving rotations or finite-body sliding require a
/// moving-wrench/branch model and reject; small numerical rotation is retained.
/// Acceptance uses the emitting geometry witness plus floating pose evaluation
/// allowance, never a user-chosen rest speed, position snap or force clamp.
pub(super) fn admit_motion(
    first: &physics::rigid_motion::RigidMotion,
    first_shape: AffineBox,
    second: &physics::rigid_motion::RigidMotion,
    second_shape: AffineBox,
    fixed: bool,
    points: &[physics::liquid::RigidSupportPoint],
) -> Result<f64, Error> {
    if first.duration() != second.duration() || points.is_empty() {
        return Err(Error::InvalidCollision);
    }
    let a = first.initial();
    let b = second.initial();
    let qa = a
        .spin
        .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
    let qb = b
        .spin
        .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
    let ca = DVec3::from_array(a.motion.position);
    let cb = DVec3::from_array(b.motion.position);
    let shape = |body: DVec3, q: DQuat, template: AffineBox| AffineBox {
        center: body + rotate_vector(q, template.center),
        edges: template.edges.map(|e| rotate_vector(q, e)),
    };
    let sa = shape(ca, qa, first_shape);
    let sb = shape(cb, qb, second_shape);
    let ra =
        first_shape.center.length() + first_shape.edges.iter().map(|e| e.length()).sum::<f64>();
    let rb =
        second_shape.center.length() + second_shape.edges.iter().map(|e| e.length()).sum::<f64>();
    let angular = angular_excursion(first, ra)? + angular_excursion(second, rb)?;
    let velocity = DVec3::from_array(a.motion.velocity) - DVec3::from_array(b.motion.velocity);
    let acceleration =
        DVec3::from_array(first.acceleration()) - DVec3::from_array(second.acceleration());
    let duration = first.duration();
    let mut max_error: f64 = 0.;
    for point in points {
        let token = FeatureKey::decode(point.feature.ok_or(Error::CollisionBackend)?)?;
        let normal = DVec3::from_array(point.support.contact.normal);
        let p = DVec3::from_array(point.support.contact.point);
        if !point.tolerance_m.is_finite() || point.tolerance_m < 0. || !p.is_finite() {
            return Err(Error::InvalidCollision);
        }
        let expected = plane(
            a,
            first_shape,
            (!fixed).then_some(b),
            second_shape,
            token.axis,
            normal.to_array(),
        )?;
        if expected != point.support.plane {
            return Err(Error::InvalidCollision);
        }
        let tolerance = point.admission_error_m
            + 256.
                * f64::EPSILON
                * (1.
                    + ca.abs().max_element()
                    + cb.abs().max_element()
                    + p.abs().max_element()
                    + ra
                    + rb);
        let (lo, hi) = polynomial_range(velocity.dot(normal), acceleration.dot(normal), duration)?;
        let gap = (sa.center - sb.center).dot(normal) - sa.radius(normal) - sb.radius(normal);
        let error = if point.carrying_reaction {
            (gap + lo).abs().max((gap + hi).abs()) + angular
        } else {
            (-(gap + lo) + angular).max(0.)
        };
        max_error = max_error.max(error);
        if (point.carrying_reaction && angular > tolerance) || error > tolerance {
            return Err(Error::CollisionBudget);
        }
        if !point.carrying_reaction {
            continue;
        }
        // A finite reciprocal pair must retain coincident application points.
        // Otherwise frozen COM arms would introduce an unowned couple.
        if !fixed {
            let mut bound = DVec3::ZERO;
            for k in 0..3 {
                let (lo, hi) = polynomial_range(velocity[k], acceleration[k], duration)?;
                bound[k] = lo.abs().max(hi.abs());
            }
            if bound.length() + angular > tolerance {
                return Err(Error::CollisionBudget);
            }
            max_error = max_error.max(bound.length() + angular);
        }
        // Model application points follow first COM with a fixed world arm.
        // Verify ownership on both complete affine volumes over the interval.
        for (owner, v, acc) in [(sa, DVec3::ZERO, DVec3::ZERO), (sb, velocity, acceleration)] {
            let inverse =
                glam::DMat3::from_cols(owner.edges[0], owner.edges[1], owner.edges[2]).inverse();
            if !inverse.is_finite() {
                return Err(Error::InvalidCollision);
            }
            let initial = inverse * (p - owner.center);
            for k in 0..3 {
                let row = inverse.transpose().col(k);
                let (lo, hi) = polynomial_range(row.dot(v), row.dot(acc), duration)?;
                let extent = (initial[k] + lo).abs().max((initial[k] + hi).abs());
                max_error = max_error.max((extent - 1.).max(0.) / row.length() + angular);
                if extent + row.length() * angular > 1. + row.length() * tolerance {
                    return Err(Error::CollisionBudget);
                }
            }
        }
    }
    Ok(max_error)
}
