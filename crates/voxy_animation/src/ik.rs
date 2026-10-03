//! Rig-bound analytical IK. Contact acquisition and foot-lock state belong to callers.
use super::{AnimationError, AnimatorFrame, Joint, Pose, Skeleton, rigs_match};
use glam::{DMat3, DMat4, DQuat, DVec3, Quat, Vec3};
use std::sync::Arc;

/// Immutable direct root/middle/tip chain bound to an exact skeleton layout.
#[derive(Clone, Debug)]
pub struct TwoBoneChain {
    rig: Arc<[Joint]>,
    joints: [usize; 3],
}
/// All targets use model coordinates, before the actor's world transform.
#[derive(Clone, Copy, Debug)]
pub struct TwoBoneTarget {
    pub position: Vec3,
    pub pole: Vec3,
    /// Proper global orientation after accounting for the tip's signed scale.
    pub rotation: Option<Quat>,
    /// Local-rotation blend weight in [0,1], not a linear endpoint interpolation.
    pub weight: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoBoneResult {
    pub tip_position: Vec3,
    /// Full-strength target lies outside the chain's geometric reach.
    pub reach_clamped: bool,
}
impl TwoBoneChain {
    /// Compiles topology once; does not retain mutable playback state.
    /// # Errors
    /// Requires distinct, directly connected parent-before-child joints.
    pub fn new(skeleton: &Skeleton, joints: [u16; 3]) -> Result<Self, AnimationError> {
        let [root, middle, tip] = joints.map(usize::from);
        if root >= middle
            || middle >= tip
            || tip >= skeleton.joints.len()
            || skeleton.joints[middle].parent != Some(joints[0])
            || skeleton.joints[tip].parent != Some(joints[1])
        {
            return Err(AnimationError::InvalidIkChain);
        }
        Ok(Self {
            rig: skeleton.joints.clone(),
            joints: [root, middle, tip],
        })
    }
}
fn globals(pose: &Pose) -> Result<Vec<DMat4>, AnimationError> {
    let mut result: Vec<DMat4> = Vec::with_capacity(pose.local.len());
    for (i, (local, joint)) in pose.local.iter().zip(pose.rig.iter()).enumerate() {
        if !local.is_valid() {
            return Err(AnimationError::InvalidPose(i));
        }
        let local = DMat4::from_scale_rotation_translation(
            local.scale.as_dvec3(),
            DQuat::from_array(local.rotation.to_array().map(f64::from)).normalize(),
            local.translation.as_dvec3(),
        );
        let world = joint
            .parent
            .map_or(local, |parent| result[usize::from(parent)] * local);
        if !world.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        result.push(world);
    }
    Ok(result)
}
// An orthogonal signed scale maps rotations through a proper pseudovector basis.
fn sign_basis(scale: Vec3) -> DQuat {
    let signs = scale.signum().as_dvec3();
    DQuat::from_mat3(&DMat3::from_diagonal(signs * signs.x * signs.y * signs.z)).normalize()
}
fn parent_basis(pose: &Pose, joint: usize) -> Result<DQuat, AnimationError> {
    let mut result = DQuat::IDENTITY;
    let mut parent = pose.rig[joint].parent;
    while let Some(index) = parent {
        let local = pose.local[usize::from(index)];
        let scale = local.scale.abs();
        if scale.x != scale.y || scale.x != scale.z {
            return Err(AnimationError::UnsupportedIkScale);
        }
        let reflection = sign_basis(local.scale);
        let rotation = DQuat::from_array(local.rotation.to_array().map(f64::from)).normalize();
        result = (rotation * reflection * result).normalize();
        parent = pose.rig[usize::from(index)].parent;
    }
    Ok(result)
}
fn perpendicular(direction: DVec3, preferred: DVec3) -> DVec3 {
    let transverse = preferred - direction * direction.dot(preferred);
    if transverse.length() > 1e-12 * preferred.length() {
        return transverse.normalize();
    }
    let axis = if direction.x.abs() <= direction.y.abs() && direction.x.abs() <= direction.z.abs() {
        DVec3::X
    } else if direction.y.abs() <= direction.z.abs() {
        DVec3::Y
    } else {
        DVec3::Z
    };
    (axis - direction * axis.dot(direction)).normalize()
}
fn between(from: DVec3, to: DVec3, plane: DVec3) -> Result<DQuat, AnimationError> {
    if !from.is_finite() || !to.is_finite() || from.length() == 0. || to.length() == 0. {
        return Err(AnimationError::NumericalOverflow);
    }
    let from = from.normalize();
    let to = to.normalize();
    let cross = from.cross(to);
    let dot = from.dot(to).clamp(-1., 1.);
    let rotation = if cross.length() < 1e-14 {
        if dot >= 0. {
            DQuat::IDENTITY
        } else {
            DQuat::from_axis_angle(perpendicular(from, plane), std::f64::consts::PI)
        }
    } else {
        DQuat::from_xyzw(cross.x, cross.y, cross.z, 1. + dot).normalize()
    };
    if !rotation.is_finite() || !rotation.is_normalized() {
        return Err(AnimationError::NumericalOverflow);
    }
    Ok(rotation)
}
fn rotate(pose: &mut Pose, joint: usize, delta: DQuat) -> Result<(), AnimationError> {
    let parent = parent_basis(pose, joint)?;
    let old = DQuat::from_array(pose.local[joint].rotation.to_array().map(f64::from)).normalize();
    let rotation = (parent.conjugate() * delta * parent * old).normalize();
    pose.set_joint_rotation(
        joint,
        Quat::from_array(rotation.to_array().map(|v| v as f32)).normalize(),
    )
}
impl Pose {
    /// Solves a two-link chain without changing any local translations or scales.
    /// Pole degeneracy uses the existing knee plane, then a deterministic axis.
    /// # Errors
    /// Rejects foreign rigs, invalid targets, collapsed links, nonuniform ancestors
    /// and overflowing palettes. The source pose is never mutated on failure.
    pub fn solve_two_bone(
        &self,
        chain: &TwoBoneChain,
        target: TwoBoneTarget,
    ) -> Result<(Self, TwoBoneResult), AnimationError> {
        if !rigs_match(&self.rig, &chain.rig) {
            return Err(AnimationError::SkeletonMismatch);
        }
        if !target.position.is_finite()
            || !target.pole.is_finite()
            || !target.weight.is_finite()
            || !(0. ..=1.).contains(&target.weight)
            || target
                .rotation
                .is_some_and(|q| !q.is_finite() || !q.is_normalized())
        {
            return Err(AnimationError::InvalidIkTarget);
        }
        let rig = Skeleton {
            joints: chain.rig.clone(),
        };
        self.skin_matrices(&rig)?;
        let [root, middle, tip] = chain.joints;
        parent_basis(self, tip)?;
        let world = globals(self)?;
        let start = world[root].w_axis.truncate();
        let knee = world[middle].w_axis.truncate();
        let end = world[tip].w_axis.truncate();
        let first = (knee - start).length();
        let second = (end - knee).length();
        if !first.is_finite() || !second.is_finite() || first == 0. || second == 0. {
            return Err(AnimationError::DegenerateIkChain);
        }
        let offset = target.position.as_dvec3() - start;
        let distance = offset.length();
        let direction = if distance > 0. {
            offset / distance
        } else if (end - start).length() > 0. {
            (end - start).normalize()
        } else {
            (knee - start).normalize()
        };
        let pole = target.pole.as_dvec3() - start;
        let projection = pole - direction * pole.dot(direction);
        let bend = if projection.length() > 1e-12 * pole.length() {
            projection.normalize()
        } else {
            perpendicular(direction, knee - start)
        };
        let size = first.max(second);
        let (a, b) = (first / size, second / size);
        let requested = distance / size;
        let d = requested.clamp((a - b).abs(), a + b);
        let x = if d == 0. {
            0.
        } else {
            ((a - b) * (a + b) + d * d) / (2. * d)
        };
        let height = ((a - x) * (a + x)).max(0.).sqrt();
        let desired_knee = start + direction * (x * size) + bend * (height * size);
        let desired_end = start + direction * (d * size);
        let plane = direction.cross(bend);
        let mut solved = self.clone();
        rotate(
            &mut solved,
            root,
            between(knee - start, desired_knee - start, plane)?,
        )?;
        let updated = globals(&solved)?;
        let knee = updated[middle].w_axis.truncate();
        let end = updated[tip].w_axis.truncate();
        rotate(
            &mut solved,
            middle,
            between(end - knee, desired_end - knee, plane)?,
        )?;
        if let Some(rotation) = target.rotation {
            let parent = parent_basis(&solved, tip)?;
            let target = DQuat::from_array(rotation.to_array().map(f64::from)).normalize();
            let local =
                (parent.conjugate() * target * sign_basis(solved.local[tip].scale).conjugate())
                    .normalize();
            // Preserve signed tip scale while targeting the proper global orientation.
            solved.set_joint_rotation(
                tip,
                Quat::from_array(local.to_array().map(|v| v as f32)).normalize(),
            )?;
        }
        for joint in [root, middle, tip] {
            let old = self.local[joint].rotation;
            let new = solved.local[joint].rotation;
            solved.set_joint_rotation(
                joint,
                if target.weight == 0. {
                    old
                } else if target.weight == 1. {
                    new
                } else {
                    old.slerp(new, target.weight).normalize()
                },
            )?;
        }
        solved.skin_matrices(&rig)?;
        let position = globals(&solved)?[tip].w_axis.truncate().as_vec3();
        if !position.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok((
            solved,
            TwoBoneResult {
                tip_position: position,
                reach_clamped: requested != d,
            },
        ))
    }
}
impl AnimatorFrame {
    /// Rebuilds the palette after IK, preserving extracted motion and clock metadata.
    /// # Errors
    /// Rejects foreign skeletons and all pose/IK validation failures before returning a frame.
    pub fn with_two_bone_ik(
        mut self,
        skeleton: &Skeleton,
        chain: &TwoBoneChain,
        target: TwoBoneTarget,
    ) -> Result<(Self, TwoBoneResult), AnimationError> {
        self.pose.skin_matrices(skeleton)?;
        let (pose, report) = self.pose.solve_two_bone(chain, target)?;
        let palette = pose.skin_matrices(skeleton)?;
        self.pose = pose;
        self.skin_matrices = palette;
        Ok((self, report))
    }
}
#[cfg(test)]
mod tests;
