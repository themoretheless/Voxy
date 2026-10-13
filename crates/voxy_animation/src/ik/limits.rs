//! Swing/twist joint limits relative to the bind local rotation. The twist
//! axis is the bind-space direction to the joint's first child (or +Y).

use glam::{DQuat, DVec3, Quat};

use crate::{AnimationError, Pose};

/// Cone half-angle for swing and a signed twist range, in radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointLimit {
    pub swing: f32,
    pub twist: [f32; 2],
}

impl JointLimit {
    /// # Errors
    /// Rejects nonfinite values, a negative swing, a swing above π, or an
    /// inverted or oversized twist range.
    pub fn new(swing: f32, twist: [f32; 2]) -> Result<Self, AnimationError> {
        let ok = swing.is_finite()
            && (0.0..=std::f32::consts::PI).contains(&swing)
            && twist
                .iter()
                .all(|t| t.is_finite() && t.abs() <= std::f32::consts::PI)
            && twist[0] <= twist[1];
        if ok {
            Ok(Self { swing, twist })
        } else {
            Err(AnimationError::InvalidJointLimit)
        }
    }

    /// Clamps `rotation` (a delta from the bind local rotation) so its swing
    /// about `axis` stays inside the cone and its twist inside the range.
    #[must_use]
    pub fn clamp_delta(&self, rotation: DQuat, axis: DVec3) -> DQuat {
        let (swing, twist) = swing_twist(rotation, axis);
        let swing = clamp_angle(swing, f64::from(self.swing));
        let twist_angle = signed_angle(twist, axis);
        let clamped = twist_angle.clamp(f64::from(self.twist[0]), f64::from(self.twist[1]));
        let twist = DQuat::from_axis_angle(axis, clamped);
        (swing * twist).normalize()
    }
}

/// Decomposes `q = swing * twist` with `twist` about `axis`.
pub(crate) fn swing_twist(q: DQuat, axis: DVec3) -> (DQuat, DQuat) {
    let r = DVec3::new(q.x, q.y, q.z);
    let projection = axis * r.dot(axis);
    let twist = DQuat::from_xyzw(projection.x, projection.y, projection.z, q.w);
    let twist = if twist.length_squared() < 1e-24 {
        DQuat::IDENTITY
    } else {
        twist.normalize()
    };
    let swing = (q * twist.conjugate()).normalize();
    (swing, twist)
}

fn signed_angle(twist: DQuat, axis: DVec3) -> f64 {
    let (a, angle) = twist.to_axis_angle();
    let angle = if a.dot(axis) < 0.0 { -angle } else { angle };
    // Fold into (-π, π].
    let folded = (angle + std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI)
        - std::f64::consts::PI;
    if folded <= -std::f64::consts::PI {
        std::f64::consts::PI
    } else {
        folded
    }
}

fn clamp_angle(q: DQuat, max: f64) -> DQuat {
    let (axis, angle) = q.to_axis_angle();
    let angle = if angle > std::f64::consts::PI {
        angle - 2.0 * std::f64::consts::PI
    } else {
        angle
    };
    if angle.abs() <= max || angle == 0.0 {
        q
    } else {
        DQuat::from_axis_angle(axis, max.copysign(angle))
    }
}

/// Bind-space twist axis for `joint`: direction to its first child, else +Y.
pub(crate) fn twist_axis(pose: &Pose, joint: usize) -> DVec3 {
    let child = pose
        .rig
        .iter()
        .position(|j| j.parent == Some(joint as u16))
        .map(|c| pose.rig[c].bind_local.translation.as_dvec3());
    match child {
        Some(d) if d.length() > 1e-12 => d.normalize(),
        _ => DVec3::Y,
    }
}

/// Applies `limit` to the joint's local rotation relative to its bind local
/// rotation; a `None` limit leaves the pose unchanged.
pub(crate) fn apply_limit(
    pose: &mut Pose,
    joint: usize,
    limit: Option<&JointLimit>,
) -> Result<(), AnimationError> {
    let Some(limit) = limit else {
        return Ok(());
    };
    let bind = DQuat::from_array(
        pose.rig[joint]
            .bind_local
            .rotation
            .to_array()
            .map(f64::from),
    )
    .normalize();
    let local = DQuat::from_array(pose.local[joint].rotation.to_array().map(f64::from)).normalize();
    let delta = (bind.conjugate() * local).normalize();
    let clamped = limit.clamp_delta(delta, twist_axis(pose, joint));
    let rotation = (bind * clamped).normalize();
    pose.set_joint_rotation(
        joint,
        Quat::from_array(rotation.to_array().map(|v| v as f32)).normalize(),
    )
}

/// Delta from bind as (swing angle, twist angle) for tests and diagnostics.
#[must_use]
pub fn swing_twist_angles(pose: &Pose, joint: usize) -> (f64, f64) {
    let bind = DQuat::from_array(
        pose.rig[joint]
            .bind_local
            .rotation
            .to_array()
            .map(f64::from),
    )
    .normalize();
    let local = DQuat::from_array(pose.local[joint].rotation.to_array().map(f64::from)).normalize();
    let delta = (bind.conjugate() * local).normalize();
    let axis = twist_axis(pose, joint);
    let (swing, twist) = swing_twist(delta, axis);
    let (_, swing_angle) = swing.to_axis_angle();
    let swing_angle = if swing_angle > std::f64::consts::PI {
        2.0 * std::f64::consts::PI - swing_angle
    } else {
        swing_angle
    };
    (swing_angle, signed_angle(twist, axis))
}
