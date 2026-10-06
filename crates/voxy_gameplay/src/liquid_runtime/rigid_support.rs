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

fn rotating_cubic_dot_range(
    normal: DVec3,
    omega: DVec3,
    r: DVec3,
    v: DVec3,
    a: DVec3,
    j: DVec3,
    h: f64,
) -> Result<(f64, f64), Error> {
    let nd = omega.cross(normal);
    let ndd = omega.cross(nd);
    let nddd = omega.cross(ndd);
    let g = normal.dot(r);
    let gd = nd.dot(r) + normal.dot(v);
    let gdd = ndd.dot(r) + 2. * nd.dot(v) + normal.dot(a);
    let gddd = nddd.dot(r) + 3. * ndd.dot(v) + 3. * nd.dot(a) + normal.dot(j);
    let (lo, hi) = polynomial_range(gd, gdd, gddd, h)?;
    let speed = omega.length();
    let remainder = if speed == 0. {
        0.
    } else {
        let direction = omega / speed;
        let perpendicular = |x: DVec3| (x - direction * direction.dot(x)).length();
        let nr = perpendicular(normal);
        let rr = perpendicular(r)
            + perpendicular(v) * h
            + perpendicular(a) * h * h / 2.
            + perpendicular(j) * h * h * h / 6.;
        let vr = perpendicular(v) + perpendicular(a) * h + perpendicular(j) * h * h / 2.;
        let ar = perpendicular(a) + perpendicular(j) * h;
        nr * (speed.powi(4) * rr
            + 4. * speed.powi(3) * vr
            + 6. * speed.powi(2) * ar
            + 4. * speed * perpendicular(j))
            * h.powi(4)
            / 24.
    };
    let guard = 1024.
        * f64::EPSILON
        * (1. + r.length() + v.length() * h + a.length() * h * h + j.length() * h * h * h);
    let lower = g + lo - remainder - guard;
    let upper = g + hi + remainder + guard;
    if !lower.is_finite() || !upper.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok((lower, upper))
}

/// Bound a cubic common point in one rotating affine slab over every accepted
/// spin arc. Third-order cancellation is retained; the fourth derivative has
/// an analytic remainder bound, so endpoint agreement is insufficient.
fn slab_range(
    path: &physics::rigid_motion::RigidMotion,
    shape: AffineBox,
    axis: usize,
    point: DVec3,
    motion: physics::liquid::RigidSupportPointMotion,
) -> Result<(f64, f64), Error> {
    let inverse = glam::DMat3::from_cols(shape.edges[0], shape.edges[1], shape.edges[2]).inverse();
    if !inverse.is_finite() {
        return Err(Error::InvalidCollision);
    }
    let row = inverse.transpose().col(axis);
    let length = row.length();
    if !length.is_finite() || length <= 0. || !shape.center.is_finite() {
        return Err(Error::InvalidCollision);
    }
    let local = row / length;
    let offset = local.dot(shape.center);
    let intervals: Vec<_> = path.rotation().map_or_else(
        || vec![(0., path.duration(), DVec3::ZERO)],
        |rotation| {
            rotation
                .segments()
                .iter()
                .map(|segment| {
                    (
                        segment.start_s,
                        segment.end_s,
                        DVec3::from_array(segment.arc.angular_velocity()),
                    )
                })
                .collect()
        },
    );
    let pv = DVec3::from_array(motion.velocity);
    let pa = DVec3::from_array(motion.acceleration);
    let pj = DVec3::from_array(motion.jerk);
    let mut lower = f64::INFINITY;
    let mut upper = f64::NEG_INFINITY;
    for (start, end, omega) in intervals {
        let state = path.sample(start).map_err(|_| Error::NumericalFailure)?;
        let q = state
            .spin
            .map_or(DQuat::IDENTITY, |spin| DQuat::from_array(spin.orientation));
        let normal = rotate_vector(q, local);
        let r = point + pv * start + pa * (start * start / 2.) + pj * (start * start * start / 6.)
            - DVec3::from_array(state.motion.position);
        let v =
            pv + pa * start + pj * (start * start / 2.) - DVec3::from_array(state.motion.velocity);
        let a = pa + pj * start
            - DVec3::from_array(path.acceleration())
            - DVec3::from_array(path.jerk()) * start;
        let j = pj - DVec3::from_array(path.jerk());
        let (lo, hi) = rotating_cubic_dot_range(normal, omega, r, v, a, j, end - start)?;
        let guard = 1024. * f64::EPSILON * offset.abs();
        lower = lower.min(lo - offset - guard);
        upper = upper.max(hi - offset + guard);
    }
    if !lower.is_finite() || !upper.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok((lower, upper))
}

/// Dot product of two vectors rotating at constant world angular velocities.
/// Its fourth derivative is bounded by |delta omega|*(|wa|+|wb|)^3.
fn rotating_pair_dot_range(
    normal: DVec3,
    wn: DVec3,
    vector: DVec3,
    wv: DVec3,
    h: f64,
) -> Result<(f64, f64), Error> {
    let guard = 1024. * f64::EPSILON * (1. + vector.length() * normal.length());
    let value = normal.dot(vector);
    if wn == wv {
        return Ok((value - guard, value + guard));
    }
    let mut n = [normal; 4];
    let mut v = [vector; 4];
    for k in 1..4 {
        n[k] = wn.cross(n[k - 1]);
        v[k] = wv.cross(v[k - 1]);
    }
    let delta = wv - wn;
    // Differentiate g'=n dot (delta omega cross v), preserving common
    // rotation cancellation before floating evaluation.
    let first = n[0].dot(delta.cross(v[0]));
    let second = n[1].dot(delta.cross(v[0])) + n[0].dot(delta.cross(v[1]));
    let third = n[2].dot(delta.cross(v[0]))
        + 2. * n[1].dot(delta.cross(v[1]))
        + n[0].dot(delta.cross(v[2]));
    let (lo, hi) = polynomial_range(first, second, third, h)?;
    let broad =
        delta.length() * (wn.length() + wv.length()).powi(3) * normal.length() * vector.length();
    let cross_bound = |value: DVec3| {
        value.cross(delta).length()
            + delta.length() * (wn.cross(value).length() * h).min(2. * value.length())
    };
    let projected = cross_bound(n[3]) * v[0].length()
        + 3. * cross_bound(n[2]) * v[1].length()
        + 3. * cross_bound(n[1]) * v[2].length()
        + cross_bound(n[0]) * v[3].length();
    let remainder = broad.min(projected) * h.powi(4) / 24.;
    let result = (
        value + lo - remainder - guard,
        value + hi + remainder + guard,
    );
    if !result.0.is_finite() || !result.1.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok(result)
}

fn arc_omega(path: &physics::rigid_motion::RigidMotion, time: f64) -> Result<DVec3, Error> {
    path.rotation().map_or(Ok(DVec3::ZERO), |rotation| {
        let segments = rotation.segments();
        let index = segments.partition_point(|segment| segment.end_s <= time);
        segments
            .get(index)
            .map(|segment| DVec3::from_array(segment.arc.angular_velocity()))
            .ok_or(Error::InvalidCollision)
    })
}

fn merged_arc_cuts(
    first: &physics::rigid_motion::RigidMotion,
    second: &physics::rigid_motion::RigidMotion,
) -> Vec<f64> {
    let mut cuts = vec![0., first.duration()];
    for path in [first, second] {
        if let Some(rotation) = path.rotation() {
            cuts.extend(rotation.segments().iter().map(|segment| segment.end_s));
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    cuts
}

const BINOMIAL: [[f64; 5]; 5] = [
    [1., 0., 0., 0., 0.],
    [1., 1., 0., 0., 0.],
    [1., 2., 1., 0., 0.],
    [1., 3., 3., 1., 0.],
    [1., 4., 6., 4., 1.],
];
fn cross_derivatives(a: DVec3, wa: DVec3, b: DVec3, wb: DVec3) -> [DVec3; 5] {
    let mut ad = [a; 5];
    let mut bd = [b; 5];
    for k in 1..5 {
        ad[k] = wa.cross(ad[k - 1]);
        bd[k] = wb.cross(bd[k - 1]);
    }
    std::array::from_fn(|k| {
        (0..=k)
            .map(|i| ad[i].cross(bd[k - i]) * BINOMIAL[k][i])
            .sum()
    })
}

/// Lower/upper magnitude of the unnormalized cross product, including
/// interior poles. A static normalized direction does not bypass this check.
fn cross_magnitude_range(
    a: DVec3,
    wa: DVec3,
    b: DVec3,
    wb: DVec3,
    h: f64,
) -> Result<(f64, f64), Error> {
    let c = a.cross(b);
    let magnitude = c.x.hypot(c.y).hypot(c.z);
    let guard = 2048. * f64::EPSILON * (a.length() * b.length() + magnitude);
    if wa == wb {
        if magnitude <= guard {
            return Err(Error::CollisionBudget);
        }
        return Ok((magnitude - guard, magnitude + guard));
    }
    let derivatives = cross_derivatives(a, wa, b, wb);
    let mut displacement = 0_f64;
    for k in 0..3 {
        let (lo, hi) =
            polynomial_range(derivatives[1][k], derivatives[2][k], derivatives[3][k], h)?;
        displacement = displacement.hypot(lo.abs().max(hi.abs()));
    }
    let remainder = (wa.length() + wb.length()).powi(4) * a.length() * b.length() * h.powi(4) / 24.;
    let direct_lo = (magnitude - displacement - remainder - guard).max(0.);
    let direct_hi = magnitude + displacement + remainder + guard;
    let (lo, hi) = rotating_pair_dot_range(a, wa, b, wb, h)?;
    // |a cross b|²=|a|²|b|²-(a dot b)². The norm-product guard
    // is subtracted before using this independent lower bound.
    let product = a.length_squared() * b.length_squared();
    let norm_guard = 4096. * f64::EPSILON * product;
    let cosine_lo = (product - norm_guard - lo.abs().max(hi.abs()).powi(2))
        .max(0.)
        .sqrt();
    let lower = direct_lo.max(cosine_lo);
    let upper = direct_hi.min((product + norm_guard).sqrt());
    if !lower.is_finite() || !upper.is_finite() {
        return Err(Error::NumericalFailure);
    }
    if lower <= 0. {
        return Err(Error::CollisionBudget);
    }
    Ok((lower, upper))
}

fn cross_cubic_dot_range(
    a: DVec3,
    wa: DVec3,
    b: DVec3,
    wb: DVec3,
    r: DVec3,
    v: DVec3,
    acc: DVec3,
    j: DVec3,
    h: f64,
    sign: f64,
) -> Result<(f64, f64), Error> {
    if wa == wb {
        return rotating_cubic_dot_range(a.cross(b) * sign, wa, r, v, acc, j, h);
    }
    let c = cross_derivatives(a, wa, b, wb).map(|value| value * sign);
    let value = c[0].dot(r);
    let first = c[1].dot(r) + c[0].dot(v);
    let second = c[2].dot(r) + 2. * c[1].dot(v) + c[0].dot(acc);
    let third = c[3].dot(r) + 3. * c[2].dot(v) + 3. * c[1].dot(acc) + c[0].dot(j);
    let (lo, hi) = polynomial_range(first, second, third, h)?;
    let speed = wa.length() + wb.length();
    let rr = r.length() + v.length() * h + acc.length() * h * h / 2. + j.length() * h * h * h / 6.;
    let vr = v.length() + acc.length() * h + j.length() * h * h / 2.;
    let ar = acc.length() + j.length() * h;
    let remainder = a.length()
        * b.length()
        * (speed.powi(4) * rr
            + 4. * speed.powi(3) * vr
            + 6. * speed.powi(2) * ar
            + 4. * speed * j.length())
        * h.powi(4)
        / 24.;
    let guard = 2048.
        * f64::EPSILON
        * (1. + rr + first.abs() * h + second.abs() * h * h + third.abs() * h * h * h);
    let result = (
        value + lo - remainder - guard,
        value + hi + remainder + guard,
    );
    if !result.0.is_finite() || !result.1.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok(result)
}

/// Triple products reduce to a rotating-pair dot because affine centers and
/// edges follow one of the two owners. This retains rigid-frame cancellation.
fn cross_attached_dot_range(
    a: DVec3,
    wa: DVec3,
    b: DVec3,
    wb: DVec3,
    vector: DVec3,
    first: bool,
    h: f64,
    sign: f64,
) -> Result<(f64, f64), Error> {
    let (lo, hi) = if first {
        rotating_pair_dot_range(vector.cross(a), wa, b, wb, h)?
    } else {
        rotating_pair_dot_range(b.cross(vector), wb, a, wa, h)?
    };
    Ok(if sign > 0. { (lo, hi) } else { (-hi, -lo) })
}

fn edge_gap_range(
    first: &physics::rigid_motion::RigidMotion,
    shape: AffineBox,
    second: &physics::rigid_motion::RigidMotion,
    obstacle: AffineBox,
    feature: AxisFeature,
    normal: DVec3,
) -> Result<Option<(f64, f64)>, Error> {
    let AxisFeature::Edges(i, j) = feature else {
        return Ok(None);
    };
    let ea = axis_direction(*shape.edges.get(i as usize).ok_or(Error::InvalidCollision)?);
    let eb = axis_direction(
        *obstacle
            .edges
            .get(j as usize)
            .ok_or(Error::InvalidCollision)?,
    );
    let q = |body: ContactBody| {
        body.spin
            .map_or(DQuat::IDENTITY, |spin| DQuat::from_array(spin.orientation))
    };
    let initial =
        rotate_vector(q(first.initial()), ea).cross(rotate_vector(q(second.initial()), eb));
    let sign = if initial.dot(normal) < 0. { -1. } else { 1. };
    let mut lower = f64::INFINITY;
    let mut upper = f64::NEG_INFINITY;
    let mut fixed = true;
    for interval in merged_arc_cuts(first, second).windows(2) {
        let start = interval[0];
        let h = interval[1] - start;
        if h <= 0. {
            continue;
        }
        let ba = first.sample(start).map_err(|_| Error::NumericalFailure)?;
        let bb = second.sample(start).map_err(|_| Error::NumericalFailure)?;
        let qa = q(ba);
        let qb = q(bb);
        let wa = arc_omega(first, start)?;
        let wb = arc_omega(second, start)?;
        let a = rotate_vector(qa, ea);
        let b = rotate_vector(qb, eb);
        let raw = a.cross(b);
        fixed &= wa.cross(raw) == DVec3::ZERO && wb.cross(raw) == DVec3::ZERO;
        let (mag_lo, mag_hi) = cross_magnitude_range(a, wa, b, wb, h)?;
        let r = DVec3::from_array(ba.motion.position) - DVec3::from_array(bb.motion.position);
        let v = DVec3::from_array(ba.motion.velocity) - DVec3::from_array(bb.motion.velocity);
        let jerk = DVec3::from_array(first.jerk()) - DVec3::from_array(second.jerk());
        let acc = DVec3::from_array(first.acceleration())
            - DVec3::from_array(second.acceleration())
            + jerk * start;
        let (mut lo, mut hi) = cross_cubic_dot_range(a, wa, b, wb, r, v, acc, jerk, h, sign)?;
        let (cl, ch) =
            cross_attached_dot_range(a, wa, b, wb, rotate_vector(qa, shape.center), true, h, sign)?;
        lo += cl;
        hi += ch;
        let (cl, ch) = cross_attached_dot_range(
            a,
            wa,
            b,
            wb,
            rotate_vector(qb, obstacle.center),
            false,
            h,
            sign,
        )?;
        lo -= ch;
        hi -= cl;
        for (rotation, template, owner) in [(qa, shape, true), (qb, obstacle, false)] {
            for edge in template.edges {
                let (a, b) = cross_attached_dot_range(
                    a,
                    wa,
                    b,
                    wb,
                    rotate_vector(rotation, edge),
                    owner,
                    h,
                    sign,
                )?;
                let abs_lo = if a <= 0. && b >= 0. {
                    0.
                } else {
                    a.abs().min(b.abs())
                };
                lo -= a.abs().max(b.abs());
                hi -= abs_lo;
            }
        }
        // Divide only after proving the denominator strictly positive over
        // the whole interval; all four interval corners cover either sign.
        let values = [lo / mag_lo, lo / mag_hi, hi / mag_lo, hi / mag_hi];
        lower = lower.min(values.iter().copied().fold(f64::INFINITY, f64::min));
        upper = upper.max(values.iter().copied().fold(f64::NEG_INFINITY, f64::max));
    }
    if !lower.is_finite() || !upper.is_finite() {
        return Err(Error::NumericalFailure);
    }
    if fixed {
        Ok(None)
    } else {
        Ok(Some((lower, upper)))
    }
}

/// Whole-interval SAT gap on the rotating face owner, with both arc timelines
/// merged. Edge-cross normal ownership still requires its own normalized bound.
fn face_gap_range(
    first: &physics::rigid_motion::RigidMotion,
    shape: AffineBox,
    second: &physics::rigid_motion::RigidMotion,
    obstacle: AffineBox,
    feature: AxisFeature,
    normal: DVec3,
) -> Result<Option<(f64, f64)>, Error> {
    let owner = match feature {
        AxisFeature::BodyFace(_) => first,
        AxisFeature::ObstacleFace(_) => second,
        AxisFeature::Edges(_, _) => {
            return edge_gap_range(first, shape, second, obstacle, feature, normal);
        }
    };
    // A structurally stationary normal retains the qualified fixed-axis
    // cubic gap and projected-radius bound, including zero-budget release.
    if owner.rotation().is_none_or(|rotation| {
        rotation.segments().iter().all(|segment| {
            DVec3::from_array(segment.arc.angular_velocity()).cross(normal) == DVec3::ZERO
        })
    }) {
        return Ok(None);
    }
    let initial_q = owner
        .initial()
        .spin
        .map_or(DQuat::IDENTITY, |spin| DQuat::from_array(spin.orientation));
    let local = rotate_vector(initial_q.conjugate(), normal);
    let cuts = merged_arc_cuts(first, second);
    let mut lower = f64::INFINITY;
    let mut upper = f64::NEG_INFINITY;
    for interval in cuts.windows(2) {
        let start = interval[0];
        let h = interval[1] - start;
        if h <= 0. {
            continue;
        }
        let a = first.sample(start).map_err(|_| Error::NumericalFailure)?;
        let b = second.sample(start).map_err(|_| Error::NumericalFailure)?;
        let qa = a
            .spin
            .map_or(DQuat::IDENTITY, |spin| DQuat::from_array(spin.orientation));
        let qb = b
            .spin
            .map_or(DQuat::IDENTITY, |spin| DQuat::from_array(spin.orientation));
        let owner_q = match feature {
            AxisFeature::BodyFace(_) => qa,
            _ => qb,
        };
        let n = rotate_vector(owner_q, local);
        let wa = arc_omega(first, start)?;
        let wb = arc_omega(second, start)?;
        let wn = match feature {
            AxisFeature::BodyFace(_) => wa,
            _ => wb,
        };
        let r = DVec3::from_array(a.motion.position) - DVec3::from_array(b.motion.position);
        let v = DVec3::from_array(a.motion.velocity) - DVec3::from_array(b.motion.velocity);
        let j = DVec3::from_array(first.jerk()) - DVec3::from_array(second.jerk());
        let acc = DVec3::from_array(first.acceleration())
            - DVec3::from_array(second.acceleration())
            + j * start;
        let (mut lo, mut hi) = rotating_cubic_dot_range(n, wn, r, v, acc, j, h)?;
        let (cl, ch) = rotating_pair_dot_range(n, wn, rotate_vector(qa, shape.center), wa, h)?;
        lo += cl;
        hi += ch;
        let (cl, ch) = rotating_pair_dot_range(n, wn, rotate_vector(qb, obstacle.center), wb, h)?;
        lo -= ch;
        hi -= cl;
        for (q, w, template) in [(qa, wa, shape), (qb, wb, obstacle)] {
            for edge in template.edges {
                let (a, b) = rotating_pair_dot_range(n, wn, rotate_vector(q, edge), w, h)?;
                let abs_lo = if a <= 0. && b >= 0. {
                    0.
                } else {
                    a.abs().min(b.abs())
                };
                let abs_hi = a.abs().max(b.abs());
                lo -= abs_hi;
                hi -= abs_lo;
            }
        }
        lower = lower.min(lo);
        upper = upper.max(hi);
    }
    if !lower.is_finite() || !upper.is_finite() {
        return Err(Error::NumericalFailure);
    }
    Ok(Some((lower, upper)))
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
    admit_point_motion(
        first,
        first_shape,
        second,
        second_shape,
        fixed,
        points,
        &vec![None; points.len()],
    )
}

pub(super) fn admit_point_motion(
    first: &physics::rigid_motion::RigidMotion,
    first_shape: AffineBox,
    second: &physics::rigid_motion::RigidMotion,
    second_shape: AffineBox,
    fixed: bool,
    points: &[physics::liquid::RigidSupportPoint],
    motion: &[Option<physics::liquid::RigidSupportPointMotion>],
) -> Result<f64, Error> {
    if points.len() != motion.len() {
        return Err(Error::InvalidCollision);
    }
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
    for (index, point) in points.iter().enumerate() {
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
        let error = if let Some((lo, hi)) =
            face_gap_range(first, first_shape, second, second_shape, token.axis, normal)?
        {
            if point.carrying_reaction {
                lo.abs().max(hi.abs())
            } else {
                (-lo).max(0.)
            }
        } else {
            let angular = angular_normal_excursion(first, ra, normal)?
                + angular_normal_excursion(second, rb, normal)?;
            let (lo, hi) = polynomial_range(
                velocity.dot(normal),
                acceleration.dot(normal),
                jerk.dot(normal),
                duration,
            )?;
            let gap = (sa.center - sb.center).dot(normal) - sa.radius(normal) - sb.radius(normal);
            if point.carrying_reaction {
                (gap + lo).abs().max((gap + hi).abs()) + angular
            } else {
                (-(gap + lo) + angular).max(0.)
            }
        };
        max_error = max_error.max(error);
        if error > tolerance {
            return Err(Error::CollisionBudget);
        }
        if !point.carrying_reaction {
            continue;
        }
        if let Some(model) = motion[index] {
            if model
                .velocity
                .iter()
                .chain(&model.acceleration)
                .chain(&model.jerk)
                .any(|v| !v.is_finite())
            {
                return Err(Error::InvalidCollision);
            }
            for (path, template, initial_shape) in
                [(first, first_shape, sa), (second, second_shape, sb)]
            {
                let inverse = glam::DMat3::from_cols(
                    initial_shape.edges[0],
                    initial_shape.edges[1],
                    initial_shape.edges[2],
                )
                .inverse();
                if !inverse.is_finite() {
                    return Err(Error::InvalidCollision);
                }
                let initial = inverse * (p - initial_shape.center);
                for axis in 0..3 {
                    let length = inverse.transpose().col(axis).length();
                    let extent = 1. / length;
                    let (lo, hi) = slab_range(path, template, axis, p, model)?;
                    let mut excursion = (-extent - lo).max(hi - extent).max(0.);
                    // Preserve every active initial plane of this clipped
                    // vertex, in addition to staying inside both volumes.
                    if (initial[axis].abs() - 1.).abs() / length
                        <= point.tolerance_m
                            + point.admission_error_m
                            + 1024. * f64::EPSILON * (1. + p.length())
                    {
                        let plane = initial[axis].signum() * extent;
                        excursion = excursion.max((lo - plane).abs().max((hi - plane).abs()));
                    }
                    max_error = max_error.max(excursion);
                    if excursion > tolerance {
                        return Err(Error::CollisionBudget);
                    }
                }
            }
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
    solve_point_constraints(
        first,
        first_shape,
        second,
        second_shape,
        point,
        tolerance_m,
        true,
        |_, body, _, velocity| Ok(body.map_or(0., |_| velocity)),
    )
}

/// Candidate second derivative from the same independent active-plane basis.
/// The factor two accounts for both the rotating plane and sliding along it.
/// Redundant plane rates may carry pressure-solver residual. Their actual
/// distance is bounded by whole-interval admission, not forced to agree exactly.
pub(super) fn point_acceleration(
    first: ContactBody,
    first_shape: AffineBox,
    second: Option<ContactBody>,
    second_shape: AffineBox,
    point: [f64; 3],
    tolerance_m: f64,
    loads: [physics::contact::ContactWrench; 2],
) -> Result<[f64; 3], Error> {
    if loads.iter().any(|load| {
        load.force
            .iter()
            .chain(&load.torque)
            .any(|x| !x.is_finite())
    }) {
        return Err(Error::InvalidCollision);
    }
    let velocity = DVec3::from_array(point_velocity(
        first,
        first_shape,
        second,
        second_shape,
        point,
        tolerance_m,
    )?);
    let p = DVec3::from_array(point);
    // Each shape contributes all its active planes through one load owner.
    solve_point_constraints(
        first,
        first_shape,
        second,
        second_shape,
        point,
        tolerance_m,
        false,
        |owner, body, normal, _| {
            let load = loads[owner];
            let Some(body) = body else {
                return Ok(0.);
            };
            if body.spin.is_none() && load.torque != [0.; 3] {
                return Err(Error::InvalidCollision);
            }
            let material = DVec3::from_array(
                body.point_velocity(point)
                    .map_err(|_| Error::InvalidCollision)?,
            );
            let arm = p - DVec3::from_array(body.motion.position);
            let (omega, alpha) = if let Some(spin) = body.spin {
                (
                    DVec3::from_array(
                        spin.angular_velocity()
                            .map_err(|_| Error::InvalidCollision)?,
                    ),
                    DVec3::from_array(
                        spin.angular_acceleration(load.torque)
                            .map_err(|_| Error::InvalidCollision)?,
                    ),
                )
            } else {
                (DVec3::ZERO, DVec3::ZERO)
            };
            let acceleration = DVec3::from_array(load.force) / body.motion.mass
                + alpha.cross(arm)
                + omega.cross(omega.cross(arm));
            let value =
                normal.dot(acceleration) + 2. * omega.cross(normal).dot(material - velocity);
            if !value.is_finite() {
                return Err(Error::InvalidCollision);
            }
            Ok(value)
        },
    )
}

/// Candidate third derivative of the same active-plane basis. Required by cubic
/// common-point trajectories; not a certificate of finite branch persistence.
pub(super) fn point_jerk(
    first: ContactBody,
    first_shape: AffineBox,
    second: Option<ContactBody>,
    second_shape: AffineBox,
    point: [f64; 3],
    tolerance_m: f64,
    loads: [physics::contact::ContactWrench; 2],
    rates: [physics::contact::ContactWrench; 2],
) -> Result<[f64; 3], Error> {
    if rates.iter().any(|load| {
        load.force
            .iter()
            .chain(&load.torque)
            .any(|v| !v.is_finite())
    }) {
        return Err(Error::InvalidCollision);
    }
    let velocity = DVec3::from_array(point_velocity(
        first,
        first_shape,
        second,
        second_shape,
        point,
        tolerance_m,
    )?);
    let acceleration = DVec3::from_array(point_acceleration(
        first,
        first_shape,
        second,
        second_shape,
        point,
        tolerance_m,
        loads,
    )?);
    let p = DVec3::from_array(point);
    solve_point_constraints(
        first,
        first_shape,
        second,
        second_shape,
        point,
        tolerance_m,
        false,
        |owner, body, normal, _| {
            let Some(body) = body else {
                return Ok(0.);
            };
            let load = loads[owner];
            let rate = rates[owner];
            if body.spin.is_none() && rate.torque != [0.; 3] {
                return Err(Error::InvalidCollision);
            }
            let (omega, alpha, beta) = if let Some(spin) = body.spin {
                (
                    DVec3::from_array(
                        spin.angular_velocity()
                            .map_err(|_| Error::InvalidCollision)?,
                    ),
                    DVec3::from_array(
                        spin.angular_acceleration(load.torque)
                            .map_err(|_| Error::InvalidCollision)?,
                    ),
                    DVec3::from_array(
                        spin.angular_jerk(load.torque, rate.torque)
                            .map_err(|_| Error::InvalidCollision)?,
                    ),
                )
            } else {
                (DVec3::ZERO, DVec3::ZERO, DVec3::ZERO)
            };
            let nd = omega.cross(normal);
            let ndd = alpha.cross(normal) + omega.cross(nd);
            let nddd = beta.cross(normal)
                + 2. * alpha.cross(nd)
                + omega.cross(alpha.cross(normal))
                + omega.cross(omega.cross(nd));
            let value = normal.dot(DVec3::from_array(rate.force) / body.motion.mass)
                - 3. * nd.dot(acceleration - DVec3::from_array(load.force) / body.motion.mass)
                - 3. * ndd.dot(velocity - DVec3::from_array(body.motion.velocity))
                - nddd.dot(p - DVec3::from_array(body.motion.position));
            if !value.is_finite() {
                return Err(Error::InvalidCollision);
            }
            Ok(value)
        },
    )
}

fn solve_point_constraints(
    first: ContactBody,
    first_shape: AffineBox,
    second: Option<ContactBody>,
    second_shape: AffineBox,
    point: [f64; 3],
    tolerance_m: f64,
    require_coincident_rates: bool,
    mut rate: impl FnMut(usize, Option<ContactBody>, DVec3, f64) -> Result<f64, Error>,
) -> Result<[f64; 3], Error> {
    first.energy().map_err(|_| Error::InvalidCollision)?;
    if let Some(second) = second {
        second.energy().map_err(|_| Error::InvalidCollision)?;
    }
    let point = DVec3::from_array(point);
    if !point.is_finite() || !tolerance_m.is_finite() || tolerance_m < 0. {
        return Err(Error::InvalidCollision);
    }
    let mut constraints = Vec::with_capacity(6);
    for (owner, (body, shape)) in [(Some(first), first_shape), (second, second_shape)]
        .into_iter()
        .enumerate()
    {
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
                constraints.push((normal, rate(owner, body, normal, normal.dot(velocity))?));
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
                    && (!require_coincident_rates
                        || constraints
                            .iter()
                            .all(|(normal, rate)| (normal.dot(velocity) - rate).abs() <= guard))
                {
                    return Ok(velocity.to_array());
                }
            }
        }
    }
    Err(Error::CollisionBackend)
}

#[cfg(test)]
mod point_velocity_tests {
    use super::*;

    fn body(center: DVec3, velocity: DVec3) -> ContactBody {
        ContactBody {
            motion: physics::gravity::Body {
                position: center.to_array(),
                velocity: velocity.to_array(),
                mass: 1.,
            },
            spin: None,
        }
    }
    fn box_shape(x: f64, y: f64, z: f64) -> AffineBox {
        AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * x, DVec3::Y * y, DVec3::Z * z],
        }
    }

    #[test]
    fn rotating_corner_velocity_matches_independent_finite_difference() {
        let mut first = body(DVec3::Y, DVec3::ZERO);
        first.spin = Some(physics::astrophysics_spin::Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0., 0.3, 0.],
            inertia: [1.; 3],
        });
        let floor = AffineBox {
            center: DVec3::Y * -0.5,
            ..box_shape(4., 0.5, 4.)
        };
        let actual = DVec3::from_array(
            point_velocity(
                first,
                box_shape(1., 1., 1.),
                None,
                floor,
                [1., 0., 1.],
                1e-12,
            )
            .unwrap(),
        );
        assert!((actual - DVec3::new(0.3, 0., -0.3)).length() < 1e-13);
        for h in [1e-3, 1e-4] {
            let corner =
                |t: f64| DVec3::Y + DQuat::from_rotation_y(0.3 * t) * DVec3::new(1., -1., 1.);
            assert!(((corner(h) - corner(-h)) / (2. * h) - actual).length() < 1e-8);
        }
    }

    #[test]
    fn clipped_vertex_follows_both_bodies_instead_of_one_material_owner() {
        let first = body(DVec3::Y, DVec3::new(0.4, 0., 7.));
        let second = body(-DVec3::Y, DVec3::new(8., 0., -0.2));
        let point = [1., 0., 0.5];
        let actual = DVec3::from_array(
            point_velocity(
                first,
                box_shape(1., 1., 1.),
                Some(second),
                box_shape(2., 1., 0.5),
                point,
                1e-12,
            )
            .unwrap(),
        );
        let vertex = |t: f64| {
            DVec3::new(
                (1. + 0.4 * t).min(2. + 8. * t),
                0.,
                (1. + 7. * t).min(0.5 - 0.2 * t),
            )
        };
        let h = 1e-4;
        assert!(((vertex(h) - vertex(-h)) / (2. * h) - actual).length() < 1e-11);
        assert!((actual - DVec3::from_array(first.point_velocity(point).unwrap())).length() > 1.);
        assert!((actual - DVec3::from_array(second.point_velocity(point).unwrap())).length() > 1.);
        let swapped = point_velocity(
            second,
            box_shape(2., 1., 0.5),
            Some(first),
            box_shape(1., 1., 1.),
            point,
            1e-12,
        )
        .unwrap();
        assert!((actual - DVec3::from_array(swapped)).length() < 1e-13);
        // Oblique slabs exercise inverse-matrix rows independently of the
        // axis-aligned fixture. A shared affine map preserves intersection.
        let map = glam::DMat3::from_quat(DQuat::from_rotation_z(0.7))
            * glam::DMat3::from_cols(
                DVec3::new(1.2, 0.1, 0.),
                DVec3::new(0.2, 0.8, 0.1),
                DVec3::new(-0.1, 0.3, 1.1),
            );
        let shift = DVec3::new(4., -3., 2.);
        let transformed_body = |b: ContactBody| {
            body(
                map * DVec3::from_array(b.motion.position) + shift,
                map * DVec3::from_array(b.motion.velocity),
            )
        };
        let transformed_shape = |shape: AffineBox| AffineBox {
            center: map * shape.center,
            edges: shape.edges.map(|edge| map * edge),
        };
        let transformed = DVec3::from_array(
            point_velocity(
                transformed_body(first),
                transformed_shape(box_shape(1., 1., 1.)),
                Some(transformed_body(second)),
                transformed_shape(box_shape(2., 1., 0.5)),
                (map * DVec3::from_array(point) + shift).to_array(),
                1e-12,
            )
            .unwrap(),
        );
        let reference = (map * vertex(h) + shift - (map * vertex(-h) + shift)) / (2. * h);
        assert!((reference - transformed).length() < 1e-10);
    }

    #[test]
    fn clipped_acceleration_includes_sliding_on_a_rotating_plane() {
        let mut first = body(DVec3::Y, DVec3::ZERO);
        first.spin = Some(physics::astrophysics_spin::Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0., 0.3, 0.],
            inertia: [1.; 3],
        });
        let second = body(-DVec3::Y, DVec3::ZERO);
        let point = [1., 0., 0.5];
        let loads = [physics::contact::ContactWrench::default(); 2];
        let acceleration = DVec3::from_array(
            point_acceleration(
                first,
                box_shape(1., 1., 1.),
                Some(second),
                box_shape(2., 1., 0.5),
                point,
                1e-12,
                loads,
            )
            .unwrap(),
        );
        assert!((acceleration - DVec3::new(0.09, 0., 0.)).length() < 1e-13);
        let vertex = |t: f64| DVec3::new((1. + 0.5 * (0.3 * t).sin()) / (0.3 * t).cos(), 0., 0.5);
        for h in [1e-3, 3e-4] {
            let independent = (vertex(h) - 2. * vertex(0.) + vertex(-h)) / (h * h);
            assert!((independent - acceleration).length() < 1e-7);
        }
        let swapped = point_acceleration(
            second,
            box_shape(2., 1., 0.5),
            Some(first),
            box_shape(1., 1., 1.),
            point,
            1e-12,
            loads,
        )
        .unwrap();
        assert!((acceleration - DVec3::from_array(swapped)).length() < 1e-13);
        first.spin.as_mut().unwrap().angular_momentum = [0.; 3];
        let loaded = [
            physics::contact::ContactWrench {
                force: [2., 0., 0.],
                torque: [0., 0.4, 0.],
            },
            physics::contact::ContactWrench {
                force: [0., 0., -0.6],
                torque: [0.; 3],
            },
        ];
        let actual = DVec3::from_array(
            point_acceleration(
                first,
                box_shape(1., 1., 1.),
                Some(second),
                box_shape(2., 1., 0.5),
                point,
                1e-12,
                loaded,
            )
            .unwrap(),
        );
        let vertex = |t: f64| {
            let angle = 0.2 * t * t;
            let z = 0.5 - 0.3 * t * t;
            DVec3::new(t * t + (1. + z * angle.sin()) / angle.cos(), 0., z)
        };
        let h = 3e-4;
        assert!(((vertex(h) - 2. * vertex(0.) + vertex(-h)) / (h * h) - actual).length() < 1e-6);
        assert!((actual - DVec3::new(2.2, 0., -0.6)).length() < 1e-13);
        let mut invalid = loads;
        invalid[0].force[0] = f64::NAN;
        assert_eq!(
            point_acceleration(
                first,
                box_shape(1., 1., 1.),
                Some(second),
                box_shape(2., 1., 0.5),
                point,
                1e-12,
                invalid
            ),
            Err(Error::InvalidCollision)
        );
    }

    #[test]
    fn clipped_jerk_matches_independent_plane_intersection() {
        let mut first = body(DVec3::Y, DVec3::ZERO);
        first.spin = Some(physics::astrophysics_spin::Spin {
            orientation: [0., 0., 0., 1.],
            angular_momentum: [0., 0.3, 0.],
            inertia: [1.; 3],
        });
        let second = body(-DVec3::Y, DVec3::ZERO);
        let loads = [physics::contact::ContactWrench::default(); 2];
        let rates = loads;
        let actual = DVec3::from_array(
            point_jerk(
                first,
                box_shape(1., 1., 1.),
                Some(second),
                box_shape(2., 1., 0.5),
                [1., 0., 0.5],
                1e-12,
                loads,
                rates,
            )
            .unwrap(),
        );
        let vertex = |t: f64| DVec3::new((1. + 0.5 * (0.3 * t).sin()) / (0.3 * t).cos(), 0., 0.5);
        let h = 0.01;
        let independent = (vertex(2. * h) - 2. * vertex(h) + 2. * vertex(-h) - vertex(-2. * h))
            / (2. * h * h * h);
        assert!((actual - DVec3::new(0.027, 0., 0.)).length() < 1e-13);
        assert!((independent - actual).length() < 1e-6);
        first.spin.as_mut().unwrap().angular_momentum = [0.; 3];
        let rates = [
            physics::contact::ContactWrench {
                force: [0.6, 0., 0.],
                torque: [0., 0.4, 0.],
            },
            physics::contact::ContactWrench {
                force: [0., 0., -0.2],
                torque: [0.; 3],
            },
        ];
        let actual = DVec3::from_array(
            point_jerk(
                first,
                box_shape(1., 1., 1.),
                Some(second),
                box_shape(2., 1., 0.5),
                [1., 0., 0.5],
                1e-12,
                loads,
                rates,
            )
            .unwrap(),
        );
        assert!((actual - DVec3::new(0.8, 0., -0.2)).length() < 1e-13);
    }

    #[test]
    fn topology_kinks_and_unowned_points_are_rejected() {
        let shape = box_shape(1., 1., 1.);
        let stationary = body(DVec3::ZERO, DVec3::ZERO);
        let moving = body(DVec3::ZERO, DVec3::X);
        assert_eq!(
            point_velocity(stationary, shape, Some(moving), shape, [1., 1., 1.], 1e-12),
            Err(Error::CollisionBackend)
        );
        assert_eq!(
            point_velocity(stationary, shape, None, shape, [0.; 3], 1e-12),
            Err(Error::CollisionBackend)
        );
        assert_eq!(
            point_velocity(stationary, shape, None, shape, [2., 0., 0.], 1e-12),
            Err(Error::InvalidCollision)
        );
        let mut invalid = stationary;
        invalid.motion.mass = 0.;
        assert_eq!(
            point_velocity(invalid, shape, None, shape, [1.; 3], 1e-12),
            Err(Error::InvalidCollision)
        );
    }
}

#[cfg(test)]
mod point_interval_tests {
    use super::*;
    use physics::liquid::{RigidSupportPoint, RigidSupportPointMotion};
    fn box_shape(x: f64, y: f64, z: f64) -> AffineBox {
        AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * x, DVec3::Y * y, DVec3::Z * z],
        }
    }
    fn path(duration: f64, yaw: f64) -> physics::rigid_motion::RigidMotion {
        ContactBody {
            motion: physics::gravity::Body {
                position: [0., 1., 0.],
                velocity: [0.; 3],
                mass: 1.,
            },
            spin: Some(physics::astrophysics_spin::Spin {
                orientation: [0., 0., 0., 1.],
                angular_momentum: [0., yaw, 0.],
                inertia: [1.; 3],
            }),
        }
        .prepare_motion(
            [0.; 3],
            [0.; 3],
            duration,
            physics::spin_path::Config {
                max_angular_error_rad: 1e-6,
                min_step_s: 1e-9,
                max_arcs: 10000,
                max_trials: 30000,
            },
        )
        .unwrap()
    }
    fn wall(duration: f64) -> physics::rigid_motion::RigidMotion {
        ContactBody {
            motion: physics::gravity::Body {
                position: [0.; 3],
                velocity: [0.; 3],
                mass: 1.,
            },
            spin: None,
        }
        .prepare_motion(
            [0.; 3],
            [0.; 3],
            duration,
            physics::spin_path::Config {
                max_angular_error_rad: 1e-6,
                min_step_s: 1e-9,
                max_arcs: 10000,
                max_trials: 30000,
            },
        )
        .unwrap()
    }
    fn point(p: [f64; 3]) -> RigidSupportPoint {
        RigidSupportPoint {
            support: NormalSupport {
                contact: NormalContact {
                    point: p,
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
        }
    }
    #[test]
    fn cubic_corner_bound_covers_every_prefix_and_admits_small_yaw() {
        let shape = box_shape(1., 1., 1.);
        let floor = AffineBox {
            center: DVec3::Y * -0.5,
            ..box_shape(4., 0.5, 4.)
        };
        let p = DVec3::new(1., 0., 1.);
        let model = RigidSupportPointMotion {
            velocity: [0.3, 0., -0.3],
            acceleration: [-0.09, 0., -0.09],
            jerk: [-0.027, 0., 0.027],
        };
        let motion = path(0.01, 0.3);
        for axis in 0..3 {
            let (lo, hi) = slab_range(&motion, shape, axis, p, model).unwrap();
            for i in 0..=128 {
                let t = motion.duration() * i as f64 / 128.;
                let state = motion.sample(t).unwrap();
                let q = DQuat::from_array(state.spin.unwrap().orientation);
                let actual = p
                    + DVec3::from_array(model.velocity) * t
                    + DVec3::from_array(model.acceleration) * t * t / 2.
                    + DVec3::from_array(model.jerk) * t * t * t / 6.;
                let coordinate =
                    (q.conjugate() * (actual - DVec3::from_array(state.motion.position)))[axis];
                assert!(
                    coordinate >= lo && coordinate <= hi,
                    "axis={axis} time={t} coordinate={coordinate} range={lo}..{hi}"
                );
            }
        }
        assert!(
            admit_point_motion(
                &motion,
                shape,
                &wall(0.01),
                floor,
                true,
                &[point(p.to_array())],
                &[Some(model)]
            )
            .unwrap()
                < 1e-10
        );
        assert_eq!(
            admit_point_motion(
                &path(0.5, 0.3),
                shape,
                &wall(0.5),
                floor,
                true,
                &[point(p.to_array())],
                &[Some(model)]
            ),
            Err(Error::CollisionBudget)
        );
    }
    #[test]
    fn point_that_returns_inside_at_endpoint_but_leaves_patch_is_rejected() {
        let motion = path(1., 0.);
        let shape = box_shape(1., 1., 1.);
        let floor = AffineBox {
            center: DVec3::Y * -0.5,
            ..box_shape(4., 0.5, 4.)
        };
        // x(0)=x(1)=0, but the extrema exceed the face's [-1,1] span.
        let model = RigidSupportPointMotion {
            velocity: [12., 0., 0.],
            acceleration: [-72., 0., 0.],
            jerk: [144., 0., 0.],
        };
        assert_eq!(
            admit_point_motion(
                &motion,
                shape,
                &wall(1.),
                floor,
                true,
                &[point([0.; 3])],
                &[Some(model)]
            ),
            Err(Error::CollisionBudget)
        );
    }
    #[test]
    fn every_accelerated_spin_arc_is_covered_and_redundant_plane_drift_rejects() {
        let initial = path(0.1, 0.3).initial();
        let motion = initial
            .prepare_motion(
                [0.; 3],
                [0., 0.2, 0.],
                0.1,
                physics::spin_path::Config {
                    max_angular_error_rad: 1e-6,
                    min_step_s: 1e-9,
                    max_arcs: 10000,
                    max_trials: 30000,
                },
            )
            .unwrap();
        assert!(motion.rotation().unwrap().segments().len() > 1);
        let shape = box_shape(1., 1., 1.);
        let p = DVec3::new(1., 0., 1.);
        let model = RigidSupportPointMotion {
            velocity: [0.3, 0., -0.3],
            acceleration: [0.11, 0., -0.29],
            jerk: [-0.207, 0., -0.153],
        };
        for axis in 0..3 {
            let (lo, hi) = slab_range(&motion, shape, axis, p, model).unwrap();
            for i in 0..=512 {
                let t = motion.duration() * i as f64 / 512.;
                let state = motion.sample(t).unwrap();
                let q = DQuat::from_array(state.spin.unwrap().orientation);
                let moved = p
                    + DVec3::from_array(model.velocity) * t
                    + DVec3::from_array(model.acceleration) * t * t / 2.
                    + DVec3::from_array(model.jerk) * t * t * t / 6.;
                let coordinate =
                    (q.conjugate() * (moved - DVec3::from_array(state.motion.position)))[axis];
                assert!(coordinate >= lo && coordinate <= hi);
            }
        }
        let floor = AffineBox {
            center: DVec3::Y * -0.5,
            ..box_shape(4., 0.5, 4.)
        };
        let still = path(0.01, 0.);
        let loads = [
            physics::contact::ContactWrench {
                force: [0., 1., 0.],
                torque: [0.; 3],
            },
            physics::contact::ContactWrench::default(),
        ];
        // An independent-plane candidate is allowed to carry a redundant-plane
        // residual. It does not bypass the all-plane interval certificate.
        let acceleration = point_acceleration(
            still.initial(),
            shape,
            None,
            floor,
            p.to_array(),
            1e-12,
            loads,
        )
        .unwrap();
        assert_eq!(acceleration, [0., 1., 0.]);
        let drift = RigidSupportPointMotion {
            velocity: [0.; 3],
            acceleration,
            jerk: [0.; 3],
        };
        assert_eq!(
            admit_point_motion(
                &still,
                shape,
                &wall(0.01),
                floor,
                true,
                &[point(p.to_array())],
                &[Some(drift)]
            ),
            Err(Error::CollisionBudget)
        );
    }
    #[test]
    fn interval_bound_rejects_unrepresentable_affine_normalization() {
        let motion = path(0.01, 0.);
        let model = RigidSupportPointMotion {
            velocity: [0.; 3],
            acceleration: [0.; 3],
            jerk: [0.; 3],
        };
        assert_eq!(
            slab_range(&motion, box_shape(1e-308, 1., 1.), 0, DVec3::ZERO, model),
            Err(Error::InvalidCollision)
        );
        let invalid = AffineBox {
            center: DVec3::new(f64::INFINITY, 0., 0.),
            ..box_shape(1., 1., 1.)
        };
        assert_eq!(
            slab_range(&motion, invalid, 0, DVec3::ZERO, model),
            Err(Error::InvalidCollision)
        );
    }
}

#[cfg(test)]
mod moving_face_gap_tests {
    use super::*;
    fn cube() -> AffineBox {
        AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125],
        }
    }
    fn path(center: f64, dt: f64, torque: [f64; 3]) -> physics::rigid_motion::RigidMotion {
        ContactBody {
            motion: physics::gravity::Body {
                position: [0., center, 0.],
                velocity: [-center, 0., 0.],
                mass: 1.,
            },
            spin: Some(physics::astrophysics_spin::Spin {
                orientation: [0., 0., 0., 1.],
                angular_momentum: [0., 0., 1.],
                inertia: [1.; 3],
            }),
        }
        .prepare_affine_motion(
            [0., -center, 0.],
            [center, 0., 0.],
            physics::astrophysics_spin::TorquePolynomial::constant(torque),
            dt,
            physics::spin_path::Config {
                max_angular_error_rad: 1e-6,
                min_step_s: 1e-9,
                max_arcs: 10000,
                max_trials: 30000,
            },
        )
        .unwrap()
    }
    #[test]
    fn two_rotating_vector_bound_covers_independent_rodrigues_samples() {
        let n = DVec3::new(0.3, 0.5, -0.2).normalize();
        let v = DVec3::new(0.4, -0.8, 0.2);
        let wa = DVec3::new(0.7, -0.3, 0.4);
        for wb in [wa, DVec3::new(-0.2, 0.5, 0.1), DVec3::ZERO] {
            for duration in [0.01, 0.1, 0.4] {
                let (lo, hi) = rotating_pair_dot_range(n, wa, v, wb, duration).unwrap();
                for i in 0..=128 {
                    let t = duration * i as f64 / 128.;
                    let actual = (DQuat::from_scaled_axis(wa * t) * n)
                        .dot(DQuat::from_scaled_axis(wb * t) * v);
                    assert!(actual >= lo && actual <= hi);
                }
            }
        }
    }
    #[test]
    fn moving_face_admits_corotation_that_a_frozen_axis_would_reject() {
        let duration = 0.004;
        let first = path(0.125, duration, [0.; 3]);
        let second = path(-0.125, duration, [0.; 3]);
        let shape = cube();
        let (lo, hi) = face_gap_range(
            &first,
            shape,
            &second,
            shape,
            AxisFeature::BodyFace(1),
            DVec3::Y,
        )
        .unwrap()
        .unwrap();
        for i in 0..=128 {
            let t = duration * i as f64 / 128.;
            let n = DQuat::from_rotation_z(t) * DVec3::Y;
            let separation =
                DVec3::new(-0.25 * t + 0.25 * t * t * t / 6., 0.25 - 0.125 * t * t, 0.);
            let actual = n.dot(separation) - 0.25;
            assert!(actual >= lo && actual <= hi);
        }
        let end = first.sample(duration).unwrap();
        let other = second.sample(duration).unwrap();
        let q = DQuat::from_rotation_z(duration);
        let rotated = AffineBox {
            center: DVec3::ZERO,
            edges: shape.edges.map(|edge| q * edge),
        };
        let frozen_gap = (DVec3::from_array(end.motion.position)
            - DVec3::from_array(other.motion.position))
        .dot(DVec3::Y)
            - 2. * rotated.radius(DVec3::Y);
        assert!(frozen_gap < -1e-4);
        let point = physics::liquid::RigidSupportPoint {
            support: NormalSupport {
                contact: NormalContact {
                    point: [0.125, 0., 0.125],
                    normal: [0., 1., 0.],
                },
                plane: SupportPlane::First,
            },
            feature: Some(
                FeatureKey {
                    first: 0,
                    second: 0,
                    axis: AxisFeature::BodyFace(1),
                }
                .encode()
                .unwrap(),
            ),
            tolerance_m: 1e-12,
            admission_error_m: 1e-10,
            carrying_reaction: true,
        };
        let model = physics::liquid::RigidSupportPointMotion {
            velocity: [0., 0.125, 0.],
            acceleration: [-0.125, 0., 0.],
            jerk: [0., -0.125, 0.],
        };
        assert!(
            admit_point_motion(
                &first,
                shape,
                &second,
                shape,
                false,
                &[point],
                &[Some(model)]
            )
            .unwrap()
                <= 1e-10
        );
    }
    #[test]
    fn moving_gap_covers_both_independent_accelerated_arc_timelines() {
        let first = path(0.125, 0.1, [0., 0.2, 0.]);
        let second = path(-0.125, 0.1, [0.1, 0., 0.]);
        assert!(first.rotation().unwrap().segments().len() > 1);
        assert!(second.rotation().unwrap().segments().len() > 1);
        let shape = cube();
        for feature in [AxisFeature::BodyFace(1), AxisFeature::ObstacleFace(1)] {
            let (lo, hi) = face_gap_range(&first, shape, &second, shape, feature, DVec3::Y)
                .unwrap()
                .unwrap();
            for i in 0..=512 {
                let t = 0.1 * i as f64 / 512.;
                let a = first.sample(t).unwrap();
                let b = second.sample(t).unwrap();
                let qa = DQuat::from_array(a.spin.unwrap().orientation);
                let qb = DQuat::from_array(b.spin.unwrap().orientation);
                let n = if matches!(feature, AxisFeature::BodyFace(_)) {
                    qa * DVec3::Y
                } else {
                    qb * DVec3::Y
                };
                let actual = n.dot(
                    DVec3::from_array(a.motion.position) - DVec3::from_array(b.motion.position),
                ) - shape
                    .edges
                    .iter()
                    .map(|edge| n.dot(qa * (*edge)).abs())
                    .sum::<f64>()
                    - shape
                        .edges
                        .iter()
                        .map(|edge| n.dot(qb * (*edge)).abs())
                        .sum::<f64>();
                assert!(
                    actual >= lo && actual <= hi,
                    "time={t} gap={actual} bounds={lo}..{hi}"
                );
            }
        }
    }
}

#[cfg(test)]
mod moving_edge_gap_tests {
    use super::*;
    fn cube() -> AffineBox {
        AffineBox {
            center: DVec3::ZERO,
            edges: [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125],
        }
    }
    fn path(
        center: DVec3,
        velocity: DVec3,
        force: DVec3,
        rate: DVec3,
        momentum: DVec3,
        torque: DVec3,
        dt: f64,
    ) -> physics::rigid_motion::RigidMotion {
        ContactBody {
            motion: physics::gravity::Body {
                position: center.to_array(),
                velocity: velocity.to_array(),
                mass: 1.,
            },
            spin: Some(physics::astrophysics_spin::Spin {
                orientation: [0., 0., 0., 1.],
                angular_momentum: momentum.to_array(),
                inertia: [1.; 3],
            }),
        }
        .prepare_affine_motion(
            force.to_array(),
            rate.to_array(),
            physics::astrophysics_spin::TorquePolynomial::constant(torque.to_array()),
            dt,
            physics::spin_path::Config {
                max_angular_error_rad: 1e-6,
                min_step_s: 1e-9,
                max_arcs: 10000,
                max_trials: 30000,
            },
        )
        .unwrap()
    }
    #[test]
    fn normalized_edge_gap_covers_independent_samples_on_both_arc_timelines() {
        let first = path(
            DVec3::new(0.3, 0.1, 0.2),
            DVec3::new(0.1, -0.2, 0.3),
            DVec3::new(0.2, 0.1, -0.1),
            DVec3::new(0.1, -0.1, 0.2),
            DVec3::new(0.3, 0.7, -0.2),
            DVec3::new(0.1, 0.03, -0.05),
            0.04,
        );
        let second = path(
            DVec3::new(-0.2, 0.1, -0.2),
            DVec3::new(-0.2, 0.15, -0.1),
            DVec3::new(-0.1, 0.3, 0.05),
            DVec3::new(-0.05, 0.1, -0.1),
            DVec3::new(-0.4, 0.2, 0.6),
            DVec3::new(0., 0.1, 0.08),
            0.04,
        );
        let shape = AffineBox {
            center: DVec3::new(0.02, -0.01, 0.03),
            ..cube()
        };
        let other = AffineBox {
            center: DVec3::new(-0.01, 0.02, -0.03),
            ..cube()
        };
        assert!(first.rotation().unwrap().segments().len() > 1);
        assert!(second.rotation().unwrap().segments().len() > 1);
        let (lo, hi) = edge_gap_range(
            &first,
            shape,
            &second,
            other,
            AxisFeature::Edges(0, 2),
            -DVec3::Y,
        )
        .unwrap()
        .unwrap();
        for i in 0..=512 {
            let t = 0.04 * i as f64 / 512.;
            let a = first.sample(t).unwrap();
            let b = second.sample(t).unwrap();
            let qa = DQuat::from_array(a.spin.unwrap().orientation);
            let qb = DQuat::from_array(b.spin.unwrap().orientation);
            let n = (qa * DVec3::X).cross(qb * DVec3::Z).normalize();
            let actual = n.dot(
                DVec3::from_array(a.motion.position) + qa * shape.center
                    - DVec3::from_array(b.motion.position)
                    - qb * other.center,
            ) - shape
                .edges
                .iter()
                .map(|edge| n.dot(qa * (*edge)).abs())
                .sum::<f64>()
                - other
                    .edges
                    .iter()
                    .map(|edge| n.dot(qb * (*edge)).abs())
                    .sum::<f64>();
            assert!(
                actual >= lo && actual <= hi,
                "t={t} gap={actual} range={lo}..{hi}"
            );
        }
    }
    #[test]
    fn normalized_edge_bound_admits_corotating_touch_and_reciprocal_owner() {
        let dt = 0.004;
        let first = path(
            DVec3::Y * 0.125,
            -DVec3::X * 0.125,
            -DVec3::Y * 0.125,
            DVec3::X * 0.125,
            DVec3::Z,
            DVec3::ZERO,
            dt,
        );
        let second = path(
            -DVec3::Y * 0.125,
            DVec3::X * 0.125,
            DVec3::Y * 0.125,
            -DVec3::X * 0.125,
            DVec3::Z,
            DVec3::ZERO,
            dt,
        );
        let shape = cube();
        let feature = AxisFeature::Edges(0, 2);
        let support = NormalSupport {
            contact: NormalContact {
                point: [0.125, 0., 0.125],
                normal: [0., 1., 0.],
            },
            plane: plane(
                first.initial(),
                shape,
                Some(second.initial()),
                shape,
                feature,
                [0., 1., 0.],
            )
            .unwrap(),
        };
        let geometry = physics::liquid::RigidSupportPoint {
            support,
            feature: Some(
                FeatureKey {
                    first: 0,
                    second: 0,
                    axis: feature,
                }
                .encode()
                .unwrap(),
            ),
            tolerance_m: 1e-12,
            admission_error_m: 1e-10,
            carrying_reaction: true,
        };
        let model = physics::liquid::RigidSupportPointMotion {
            velocity: [0., 0.125, 0.],
            acceleration: [-0.125, 0., 0.],
            jerk: [0., -0.125, 0.],
        };
        assert!(
            admit_point_motion(
                &first,
                shape,
                &second,
                shape,
                false,
                &[geometry],
                &[Some(model)]
            )
            .unwrap()
                <= 1e-10
        );
        let (lo, hi) = edge_gap_range(&first, shape, &second, shape, feature, DVec3::Y)
            .unwrap()
            .unwrap();
        let (rl, rh) = edge_gap_range(
            &second,
            shape,
            &first,
            shape,
            AxisFeature::Edges(2, 0),
            -DVec3::Y,
        )
        .unwrap()
        .unwrap();
        assert!((lo - rl).abs() < 1e-12 && (hi - rh).abs() < 1e-12);
    }
    #[test]
    fn interior_parallel_pole_rejects_even_with_fixed_direction_and_valid_endpoints() {
        let dt = 1.8;
        let first = path(
            DVec3::Z * 0.125,
            DVec3::ZERO,
            DVec3::ZERO,
            DVec3::ZERO,
            DVec3::Z,
            DVec3::ZERO,
            dt,
        );
        let second = path(
            -DVec3::Z * 0.125,
            DVec3::ZERO,
            DVec3::ZERO,
            DVec3::ZERO,
            DVec3::ZERO,
            DVec3::ZERO,
            dt,
        );
        assert!(DVec3::X.cross(DVec3::Y).length() > 0.1);
        assert!(
            (DQuat::from_rotation_z(dt) * DVec3::X)
                .cross(DVec3::Y)
                .length()
                > 0.1
        );
        assert!(
            (DQuat::from_rotation_z(std::f64::consts::FRAC_PI_2) * DVec3::X)
                .cross(DVec3::Y)
                .length()
                < 1e-14
        );
        assert_eq!(
            edge_gap_range(
                &first,
                cube(),
                &second,
                cube(),
                AxisFeature::Edges(0, 1),
                DVec3::Z
            ),
            Err(Error::CollisionBudget)
        );
    }
}
