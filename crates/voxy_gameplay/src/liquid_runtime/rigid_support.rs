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

/// Conservative range of a nominal cubic displacement over the full prefix.
fn polynomial_range(v: f64, a: f64, j: f64, duration: f64) -> Result<(f64, f64), Error> {
    if j != 0. {
        // The cubic Bezier hull covers every prefix, including hidden excursions.
        // Retain the tighter existing quadratic extrema for constant forces.
        let controls = [
            0.,
            v * (duration / 3.),
            (a * (duration / 6.) + v * (2. / 3.)) * duration,
            ((j * (duration / 6.) + a * 0.5) * duration + v) * duration,
        ];
        if controls.iter().any(|x| !x.is_finite()) {
            return Err(Error::NumericalFailure);
        }
        return Ok((
            controls.into_iter().fold(0., f64::min),
            controls.into_iter().fold(0., f64::max),
        ));
    }
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
/// Projected change of an affine support radius on a fixed world normal.
fn angular_normal_excursion(
    path: &physics::rigid_motion::RigidMotion,
    radius: f64,
    normal: DVec3,
) -> Result<f64, Error> {
    let bound = path.rotation().map_or(0., |rotation| {
        rotation
            .segments()
            .iter()
            .map(|s| {
                DVec3::from_array(s.arc.angular_velocity())
                    .cross(normal)
                    .length()
                    * radius
                    * (s.end_s - s.start_s)
            })
            .sum::<f64>()
    });
    if !bound.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok(bound * (1. + 512. * f64::EPSILON))
}
/// Change of rotating affine rows at the actual common application arm.
/// Relative COM displacement is bounded across the complete cubic interval.
fn angular_arm_excursion(
    path: &physics::rigid_motion::RigidMotion,
    arm: DVec3,
    displacement: f64,
    row: Option<DVec3>,
) -> Result<f64, Error> {
    let mut bound = 0.;
    if let Some(rotation) = path.rotation() {
        let initial_q = DQuat::from_array(
            path.initial()
                .spin
                .ok_or(Error::InvalidCollision)?
                .orientation,
        );
        let local_row = row.map(|r| initial_q.conjugate() * r);
        for segment in rotation.segments() {
            let w = DVec3::from_array(segment.arc.angular_velocity());
            let mut speed = w.cross(arm).length() + w.length() * displacement;
            if let Some(local) = local_row {
                let q = DQuat::from_array(
                    path.sample(segment.start_s)
                        .map_err(|_| Error::NumericalFailure)?
                        .spin
                        .ok_or(Error::InvalidCollision)?
                        .orientation,
                );
                speed = speed.min(w.cross(q * local).length() * (arm.length() + displacement));
            }
            bound += speed * (segment.end_s - segment.start_s);
        }
    }
    if !bound.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok(bound * (1. + 512. * f64::EPSILON))
}

/// Admit a common moving-point reaction model against actual nominal motion.
/// The whole cubic COM interval and every spin arc are bounded, not just
/// endpoints. Reciprocal COM torque follows the same moving world point.
/// Rotation is bounded in the emitting normal and each affine ownership row.
/// Changing material points still require a different reaction point model.
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
    let velocity = DVec3::from_array(a.motion.velocity) - DVec3::from_array(b.motion.velocity);
    let acceleration =
        DVec3::from_array(first.acceleration()) - DVec3::from_array(second.acceleration());
    let jerk = DVec3::from_array(first.jerk()) - DVec3::from_array(second.jerk());
    let duration = first.duration();
    let mut max_error: f64 = 0.;
    for point in points {
        let token = FeatureKey::decode(point.feature.ok_or(Error::CollisionBackend)?)?;
        let normal = DVec3::from_array(point.support.contact.normal);
        let p = DVec3::from_array(point.support.contact.point);
        let angular = angular_normal_excursion(first, ra, normal)?
            + angular_normal_excursion(second, rb, normal)?;
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
        let (lo, hi) = polynomial_range(
            velocity.dot(normal),
            acceleration.dot(normal),
            jerk.dot(normal),
            duration,
        )?;
        let gap = (sa.center - sb.center).dot(normal) - sa.radius(normal) - sb.radius(normal);
        let error = if point.carrying_reaction {
            (gap + lo).abs().max((gap + hi).abs()) + angular
        } else {
            (-(gap + lo) + angular).max(0.)
        };
        max_error = max_error.max(error);
        if error > tolerance {
            return Err(Error::CollisionBudget);
        }
        if !point.carrying_reaction {
            continue;
        }
        // Core now evolves the reciprocal COM torque at this same moving
        // world point. Tangential translation is admitted through ownership,
        // not rejected merely because the two COM application arms differ.
        // Model application points follow first COM with a fixed world arm.
        // Verify ownership on both complete affine volumes over the interval.
        for (owner, path, center, v, acc, j) in [
            (sa, first, ca, DVec3::ZERO, DVec3::ZERO, DVec3::ZERO),
            (sb, second, cb, velocity, acceleration, jerk),
        ] {
            let mut displacement2: f64 = 0.;
            for k in 0..3 {
                let (lo, hi) = polynomial_range(v[k], acc[k], j[k], duration)?;
                displacement2 = displacement2.hypot(lo.abs().max(hi.abs()));
            }
            let inverse =
                glam::DMat3::from_cols(owner.edges[0], owner.edges[1], owner.edges[2]).inverse();
            if !inverse.is_finite() {
                return Err(Error::InvalidCollision);
            }
            let initial = inverse * (p - owner.center);
            for k in 0..3 {
                let row = inverse.transpose().col(k);
                let angular = angular_arm_excursion(
                    path,
                    p - center,
                    displacement2,
                    Some(row / row.length()),
                )?;
                let (lo, hi) = polynomial_range(row.dot(v), row.dot(acc), row.dot(j), duration)?;
                let extent = (initial[k] + lo).abs().max((initial[k] + hi).abs());
                max_error = max_error.max(((extent - 1.) / row.length() + angular).max(0.));
                if extent + row.length() * angular > 1. + row.length() * tolerance {
                    return Err(Error::CollisionBudget);
                }
            }
        }
    }
    Ok(max_error)
}

#[cfg(test)]
mod cubic_range_tests {
    use super::*;
    #[test]
    fn cubic_support_range_covers_hidden_excursions_and_clipped_prefixes() {
        for (v, a, j) in [(0., 36., -72.), (1., -6., 6.), (0., 0., 36.)] {
            for duration in [0.01, 0.5, 1.] {
                let (lo, hi) = polynomial_range(v, a, j, duration).unwrap();
                for i in 0..=128 {
                    let t = duration * i as f64 / 128.;
                    let x = ((j * t / 6. + a / 2.) * t + v) * t;
                    assert!(x >= lo - 1e-14 && x <= hi + 1e-14);
                }
            }
        }
        // This path returns to the starting plane but penetrates inside the step.
        let (lo, hi) = polynomial_range(1., -6., 12., 1.).unwrap();
        assert!(lo < 0. && hi > 0.);
        assert!(polynomial_range(0., 0., f64::MAX, 10.).is_err());
    }
}

/// Differentiate the selected edge cross product twice in world coordinates.
pub(super) fn edge_normal_acceleration(
    first: ContactBody,
    first_shape: AffineBox,
    second: Option<ContactBody>,
    second_shape: AffineBox,
    feature: AxisFeature,
    normal: [f64; 3],
    normal_rate: [f64; 3],
    first_load: physics::contact::ContactWrench,
    second_load: physics::contact::ContactWrench,
) -> Result<[f64; 3], Error> {
    let AxisFeature::Edges(i, j) = feature else {
        return Err(Error::InvalidCollision);
    };
    let edge = |body: Option<ContactBody>,
                vector: DVec3,
                load: physics::contact::ContactWrench|
     -> Result<(DVec3, DVec3, DVec3), Error> {
        let Some(spin) = body.and_then(|b| b.spin) else {
            return Ok((axis_direction(vector), DVec3::ZERO, DVec3::ZERO));
        };
        let q = DQuat::from_array(spin.orientation);
        let e = axis_direction(rotate_vector(q, vector));
        let w = DVec3::from_array(
            spin.angular_velocity()
                .map_err(|_| Error::NumericalFailure)?,
        );
        let alpha = DVec3::from_array(
            spin.angular_acceleration(load.torque)
                .map_err(|_| Error::NumericalFailure)?,
        );
        let rate = w.cross(e);
        Ok((e, rate, alpha.cross(e) + w.cross(rate)))
    };
    let (a, ad, add) = edge(Some(first), first_shape.edges[i as usize], first_load)?;
    let (b, bd, bdd) = edge(second, second_shape.edges[j as usize], second_load)?;
    let u = a.cross(b);
    let length = u.length();
    let n = DVec3::from_array(normal);
    let nd = DVec3::from_array(normal_rate);
    let sign = if u.dot(n) < 0. { -1. } else { 1. };
    let ud = (ad.cross(b) + a.cross(bd)) * sign;
    let udd = (add.cross(b) + 2. * ad.cross(bd) + a.cross(bdd)) * sign;
    if !length.is_finite() || length <= 0. {
        return Err(Error::InvalidCollision);
    }
    let ndd = (udd - n * n.dot(udd) - 2. * n.dot(ud) * nd) / length - n * nd.length_squared();
    if !ndd.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok(ndd.to_array())
}

#[cfg(test)]
mod edge_derivative_tests {
    use super::*;
    #[test]
    fn edge_normal_second_derivative_matches_independent_rotations() {
        let make = |q: DQuat, momentum| ContactBody {
            motion: physics::gravity::Body {
                position: [0.; 3],
                velocity: [0.; 3],
                mass: 2.,
            },
            spin: Some(physics::astrophysics_spin::Spin {
                orientation: q.to_array(),
                angular_momentum: momentum,
                inertia: [1., 2., 3.],
            }),
        };
        let first = make(DQuat::from_rotation_y(0.2), [0.4, -0.3, 0.7]);
        let second = make(DQuat::from_rotation_x(0.3), [-0.2, 0.6, 0.5]);
        let shape = AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X, DVec3::Y, DVec3::Z],
        };
        let load_a = physics::contact::ContactWrench {
            force: [0.; 3],
            torque: [0.3, -0.4, 0.2],
        };
        let load_b = physics::contact::ContactWrench {
            force: [0.; 3],
            torque: [-0.1, 0.2, 0.5],
        };
        let rotation = |body: ContactBody, load: physics::contact::ContactWrench, t: f64| {
            let spin = body.spin.unwrap();
            let w = DVec3::from_array(spin.angular_velocity().unwrap());
            let alpha = DVec3::from_array(spin.angular_acceleration(load.torque).unwrap());
            DQuat::from_scaled_axis(w * t + alpha * (t * t / 2.))
                * DQuat::from_array(spin.orientation)
        };
        let normal_at = |t: f64| {
            (rotation(first, load_a, t) * DVec3::X)
                .cross(rotation(second, load_b, t) * DVec3::Z)
                .normalize()
        };
        let n = normal_at(0.);
        let feature = AxisFeature::Edges(0, 2);
        let SupportPlane::Rate { normal_rate } =
            plane(first, shape, Some(second), shape, feature, n.to_array()).unwrap()
        else {
            panic!("wrong owner");
        };
        let analytic = DVec3::from_array(
            edge_normal_acceleration(
                first,
                shape,
                Some(second),
                shape,
                feature,
                n.to_array(),
                normal_rate,
                load_a,
                load_b,
            )
            .unwrap(),
        );
        assert!((n.dot(analytic) + DVec3::from_array(normal_rate).length_squared()).abs() < 1e-13);
        for h in [1e-3, 1e-4] {
            let independent = (normal_at(h) - 2. * n + normal_at(-h)) / (h * h);
            assert!((independent - analytic).length() < 1e-6);
            assert!(
                ((normal_at(h) - normal_at(-h)) / (2. * h) - DVec3::from_array(normal_rate))
                    .length()
                    < 1e-6
            );
        }
    }
}

#[cfg(test)]
mod projected_rotation_tests {
    use super::*;
    #[test]
    fn projected_gap_and_arm_bounds_cover_all_nominal_prefixes() {
        let body = ContactBody {
            motion: physics::gravity::Body {
                position: [0.; 3],
                velocity: [0.; 3],
                mass: 1.,
            },
            spin: Some(physics::astrophysics_spin::Spin {
                orientation: [0., 0., 0., 1.],
                angular_momentum: [0.3, 0.7, -0.4],
                inertia: [1.; 3],
            }),
        };
        let path = body
            .prepare_motion(
                [0.; 3],
                [0.; 3],
                0.25,
                physics::spin_path::Config {
                    max_angular_error_rad: 1e-5,
                    min_step_s: 1e-9,
                    max_arcs: 10000,
                    max_trials: 30000,
                },
            )
            .unwrap();
        let shape = AffineBox {
            center: DVec3::new(0.1, -0.2, 0.3),
            edges: [DVec3::X * 0.2, DVec3::Y * 0.3, DVec3::Z * 0.4],
        };
        let radius = shape.center.length() + shape.edges.iter().map(|e| e.length()).sum::<f64>();
        let n = DVec3::Y;
        let gap_bound = angular_normal_excursion(&path, radius, n).unwrap();
        let arm = DVec3::new(0.15, 0.12, -0.17);
        let displacement = DVec3::new(0.025, 0.0125, 0.0046875).length();
        let arm_bound = angular_arm_excursion(&path, arm, displacement, None).unwrap();
        let inverse =
            glam::DMat3::from_cols(shape.edges[0], shape.edges[1], shape.edges[2]).inverse();
        for i in 0..=128 {
            let t = 0.25 * i as f64 / 128.;
            let q = DQuat::from_array(path.sample(t).unwrap().spin.unwrap().orientation);
            let moved = AffineBox {
                center: q * shape.center,
                edges: shape.edges.map(|e| q * e),
            };
            let support = moved.center.dot(n) + moved.radius(n);
            let initial = shape.center.dot(n) + shape.radius(n);
            assert!((support - initial).abs() <= gap_bound + 1e-14);
            let r = arm + DVec3::new(0.1 * t, 0.2 * t * t, -0.3 * t * t * t);
            let actual = inverse * (q.conjugate() * r - shape.center);
            let frozen = inverse * (r - shape.center);
            for k in 0..3 {
                assert!(
                    (actual[k] - frozen[k]).abs()
                        <= inverse.transpose().col(k).length() * arm_bound + 1e-14
                );
            }
        }
        let mut yaw = body;
        yaw.spin.as_mut().unwrap().angular_momentum = [0., 1., 0.];
        let yaw = yaw
            .prepare_motion(
                [0.; 3],
                [0.; 3],
                0.25,
                physics::spin_path::Config {
                    max_angular_error_rad: 1e-5,
                    min_step_s: 1e-9,
                    max_arcs: 10000,
                    max_trials: 30000,
                },
            )
            .unwrap();
        assert_eq!(
            angular_normal_excursion(&yaw, radius, DVec3::Y).unwrap(),
            0.
        );
        assert_eq!(
            angular_arm_excursion(&yaw, DVec3::Y * 0.3, 0., None).unwrap(),
            0.
        );
        // A frozen common point inside the rotating face remains owned,
        // although it is not on the spin axis. The normal row is invariant.
        let cube = AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125],
        };
        let floor = AffineBox {
            center: DVec3::Y * -0.25,
            edges: [DVec3::X, DVec3::Y * 0.125, DVec3::Z],
        };
        let fixed = ContactBody {
            spin: None,
            ..yaw.initial()
        }
        .prepare_motion(
            [0.; 3],
            [0.; 3],
            0.25,
            physics::spin_path::Config {
                max_angular_error_rad: 1e-5,
                min_step_s: 1e-9,
                max_arcs: 10000,
                max_trials: 30000,
            },
        )
        .unwrap();
        let point = physics::liquid::RigidSupportPoint {
            support: NormalSupport {
                contact: NormalContact {
                    point: [0.05, -0.125, 0.],
                    normal: [0., 1., 0.],
                },
                plane: SupportPlane::World,
            },
            feature: Some(
                FeatureKey {
                    first: 0,
                    second: 0,
                    axis: AxisFeature::ObstacleFace(1),
                }
                .encode()
                .unwrap(),
            ),
            tolerance_m: 1e-12,
            admission_error_m: 1e-10,
            carrying_reaction: true,
        };
        assert!(admit_motion(&yaw, cube, &fixed, floor, true, &[point]).unwrap() <= 1e-10);
    }
}

/// Differentiate the active slab constraints defining a clipped patch vertex.
/// Coincident constraints must agree; topology kinks are not assigned an
/// arbitrary material owner. Shapes are principal-body templates, except fixed
/// environment geometry, whose coordinates are already world-oriented.
pub(super) fn point_velocity(
    first: ContactBody,
    first_shape: AffineBox,
    second: Option<ContactBody>,
    second_shape: AffineBox,
    point: [f64; 3],
    tolerance_m: f64,
) -> Result<[f64; 3], Error> {
    let point = DVec3::from_array(point);
    if !point.is_finite() || !tolerance_m.is_finite() || tolerance_m < 0. {
        return Err(Error::InvalidCollision);
    }
    let mut constraints = Vec::with_capacity(6);
    for (body, shape) in [(Some(first), first_shape), (second, second_shape)] {
        let q = body
            .and_then(|b| b.spin)
            .map_or(DQuat::IDENTITY, |s| DQuat::from_array(s.orientation));
        let center = body.map_or(shape.center, |b| {
            DVec3::from_array(b.motion.position) + rotate_vector(q, shape.center)
        });
        let edges = if body.is_some() {
            shape.edges.map(|edge| rotate_vector(q, edge))
        } else {
            shape.edges
        };
        let inverse = glam::DMat3::from_cols(edges[0], edges[1], edges[2]).inverse();
        if !inverse.is_finite() {
            return Err(Error::InvalidCollision);
        }
        let coordinates = inverse * (point - center);
        let rows = inverse.transpose().to_cols_array();
        let velocity = body
            .map_or(Ok([0.; 3]), |b| b.point_velocity(point.to_array()))
            .map_err(|_| Error::InvalidCollision)?;
        let velocity = DVec3::from_array(velocity);
        for k in 0..3 {
            let row = DVec3::new(rows[k * 3], rows[k * 3 + 1], rows[k * 3 + 2]);
            let length = row.length();
            let gap = (coordinates[k].abs() - 1.) / length;
            let guard = tolerance_m
                + 1024. * f64::EPSILON * (1. + edges[k].length() + (point - center).length());
            if !gap.is_finite() || gap > guard {
                return Err(Error::InvalidCollision);
            }
            if gap.abs() <= guard {
                let normal = row / length;
                constraints.push((normal, normal.dot(velocity)));
            }
        }
    }
    for i in 0..constraints.len() {
        for j in i + 1..constraints.len() {
            for k in j + 1..constraints.len() {
                let matrix =
                    glam::DMat3::from_cols(constraints[i].0, constraints[j].0, constraints[k].0)
                        .transpose();
                let determinant = matrix.determinant().abs();
                if determinant < 1e-10 {
                    continue;
                }
                let velocity = matrix.inverse()
                    * DVec3::new(constraints[i].1, constraints[j].1, constraints[k].1);
                let guard = 4096. * f64::EPSILON * (1. + velocity.length()) / determinant;
                if velocity.is_finite()
                    && constraints
                        .iter()
                        .all(|(normal, rate)| (normal.dot(velocity) - rate).abs() <= guard)
                {
                    return Ok(velocity.to_array());
                }
            }
        }
    }
    Err(Error::CollisionBackend)
}
