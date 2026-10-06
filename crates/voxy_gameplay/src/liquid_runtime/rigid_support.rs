//! Query-local affine SAT feature keys and contact-time supporting branches.
use crate::convex::{AffineBox, AxisFeature, axis_direction};
use glam::{DQuat, DVec3};
use physics::{
    contact::{ContactBody, NormalContact, NormalSupport, SupportPlane},
    liquid::Error,
};
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
