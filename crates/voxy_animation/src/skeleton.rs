//! Skeleton hierarchy, bind-local transforms, and canonical inverse-bind validation.

use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

use crate::{AnimationError, MAX_JOINTS, Pose, SoaSkeleton, linear_rank};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    #[must_use]
    pub fn matrix(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub(crate) fn is_valid(self) -> bool {
        self.translation.is_finite()
            && self.rotation.is_finite()
            && self.rotation.is_normalized()
            && self.scale.is_finite()
            && self.scale.abs().min_element() > 0.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Joint {
    pub name: Arc<str>,
    pub parent: Option<u16>,
    pub bind_local: Transform,
    pub inverse_bind: Mat4,
}

pub(crate) fn usable_linear(matrix: glam::DMat4) -> bool {
    linear_rank::usable(matrix)
}

// Shared immutable layout is the fast path; exact structural equality also
// permits independently reconstructed copies without hashes or process IDs.
pub(crate) fn rigs_match(a: &Arc<[Joint]>, b: &Arc<[Joint]>) -> bool {
    Arc::ptr_eq(a, b) || a.as_ref() == b.as_ref()
}

#[derive(Clone, Debug)]
pub struct Skeleton {
    pub(crate) joints: Arc<[Joint]>,
}

impl Skeleton {
    /// Creates a parent-before-child skeleton suitable for linear pose evaluation.
    /// Affine inverse binds are stored in canonical homogeneous form (w=1).
    ///
    /// # Errors
    ///
    /// Rejects empty/oversized skeletons, duplicate/empty names, invalid transforms, non-finite
    /// inverse bind matrices, non-affine/singular inverse binds, and parents that
    /// do not precede their child.
    pub fn new(mut joints: Vec<Joint>) -> Result<Self, AnimationError> {
        if joints.is_empty() || joints.len() > MAX_JOINTS {
            return Err(AnimationError::InvalidJointCount(joints.len()));
        }
        for index in 0..joints.len() {
            let joint = &joints[index];
            if joint.name.is_empty()
                || joints[..index]
                    .iter()
                    .any(|candidate| candidate.name == joint.name)
            {
                return Err(AnimationError::InvalidJointName(index));
            }
            let inverse = joint.inverse_bind;
            if !joint.bind_local.is_valid()
                || !inverse.is_finite()
                || inverse.x_axis.w != 0.
                || inverse.y_axis.w != 0.
                || inverse.z_axis.w != 0.
                || inverse.w_axis.w == 0.
            {
                return Err(AnimationError::InvalidJointTransform(index));
            }
            // An affine homogeneous map may have row (0,0,0,h). Canonicalize
            // by dividing every column by h, preserving the represented map.
            // Check the source row first so division cannot erase perspective.
            let h = inverse.w_axis.w;
            let inverse = Mat4::from_cols(
                inverse.x_axis / h,
                inverse.y_axis / h,
                inverse.z_axis / h,
                inverse.w_axis / h,
            );
            if !usable_linear(inverse.as_dmat4()) {
                return Err(AnimationError::InvalidJointTransform(index));
            }
            if let Some(parent) = joint.parent
                && usize::from(parent) >= index
            {
                return Err(AnimationError::InvalidParent {
                    joint: index,
                    parent,
                });
            }
            joints[index].inverse_bind = inverse;
        }
        let skeleton = Self {
            joints: joints.into(),
        };
        // Use the same evaluator as runtime: finite local transforms can still
        // overflow along a hierarchy or when multiplied by inverse binds.
        skeleton
            .bind_pose64()
            .skin_matrices(&skeleton)
            .map_err(|error| match error {
                AnimationError::InvalidPose(index) => AnimationError::InvalidJointTransform(index),
                other => other,
            })?;
        Ok(skeleton)
    }

    #[must_use]
    pub fn joints(&self) -> &[Joint] {
        &self.joints
    }

    #[must_use]
    pub fn bind_pose(&self) -> Pose {
        Pose {
            rig: self.joints.clone(),
            local: self.joints.iter().map(|joint| joint.bind_local).collect(),
        }
    }

    /// Creates a Structure-of-Arrays (SoA) skeleton layout for cache-friendly pose evaluation.
    #[must_use]
    pub fn to_soa(&self) -> SoaSkeleton {
        SoaSkeleton::from_skeleton(self)
    }
}
