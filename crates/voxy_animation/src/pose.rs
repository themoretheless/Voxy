//! Local skeletal poses, skin-palette evaluation, and zero-allocation pose blending.

use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

use crate::{
    AnimationError, Joint, MAX_JOINTS, Skeleton, SoaPose, Transform, lerp_vec3, rigs_match,
    usable_linear,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    pub(crate) rig: Arc<[Joint]>,
    pub(crate) local: Vec<Transform>,
}

impl Pose {
    #[must_use]
    pub fn local(&self) -> &[Transform] {
        &self.local
    }

    /// Converts this AoS [`Pose`] into a contiguous [`SoaPose`].
    #[must_use]
    pub fn to_soa(&self) -> SoaPose {
        SoaPose::from_pose(self)
    }

    /// Updates one local translation, preserving its rotation and scale.
    /// Invalid joints or nonfinite translations leave the pose unchanged.
    pub fn set_joint_translation(
        &mut self,
        index: usize,
        translation: Vec3,
    ) -> Result<(), AnimationError> {
        let Some(current) = self.local.get(index).copied() else {
            return Err(AnimationError::InvalidPose(index));
        };
        let replacement = Transform {
            translation,
            ..current
        };
        if !replacement.is_valid() {
            return Err(AnimationError::InvalidPose(index));
        }
        self.local[index] = replacement;
        Ok(())
    }

    /// Updates one local scale, preserving translation and rotation.
    /// Invalid joints, nonfinite or singular scales leave the pose unchanged.
    pub fn set_joint_scale(&mut self, index: usize, scale: Vec3) -> Result<(), AnimationError> {
        let Some(current) = self.local.get(index).copied() else {
            return Err(AnimationError::InvalidPose(index));
        };
        let replacement = Transform { scale, ..current };
        if !replacement.is_valid() {
            return Err(AnimationError::InvalidPose(index));
        }
        self.local[index] = replacement;
        Ok(())
    }

    /// Updates one local rotation, preserving its translation and scale.
    /// # Errors
    /// Rejects an unknown joint or a nonfinite/nonunit quaternion without changing the pose.
    pub fn set_joint_rotation(
        &mut self,
        index: usize,
        rotation: Quat,
    ) -> Result<(), AnimationError> {
        let Some(current) = self.local.get(index).copied() else {
            return Err(AnimationError::InvalidPose(index));
        };
        let replacement = Transform {
            rotation,
            ..current
        };
        if !replacement.is_valid() {
            return Err(AnimationError::InvalidPose(index));
        }
        self.local[index] = replacement;
        Ok(())
    }

    /// Produces `global_joint * inverse_bind` matrices into reusable buffers.
    ///
    /// # Errors
    /// Rejects a pose created for a different skeleton, invalid blend data,
    /// or overflowing global/palette matrices.
    pub fn skin_matrices_into(
        &self,
        skeleton: &Skeleton,
        global: &mut Vec<Mat4>,
        skin: &mut Vec<Mat4>,
    ) -> Result<(), AnimationError> {
        if self.local.len() != skeleton.joints.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        if !rigs_match(&self.rig, &skeleton.joints) {
            return Err(AnimationError::SkeletonMismatch);
        }
        global.clear();
        skin.clear();
        global.reserve(self.local.len());
        skin.reserve(self.local.len());
        for (index, (local, joint)) in self.local.iter().zip(skeleton.joints.iter()).enumerate() {
            if !local.is_valid() {
                return Err(AnimationError::InvalidPose(index));
            }
            let matrix = joint.parent.map_or_else(
                || local.matrix(),
                |parent| global[usize::from(parent)] * local.matrix(),
            );
            let palette = matrix * joint.inverse_bind;
            if !usable_linear(matrix.as_dmat4()) || !usable_linear(palette.as_dmat4()) {
                return Err(AnimationError::InvalidPose(index));
            }
            global.push(matrix);
            skin.push(palette);
        }
        Ok(())
    }

    pub(crate) fn skin_matrices_vec_into(
        &self,
        skeleton: &Skeleton,
        skin: &mut Vec<Mat4>,
    ) -> Result<(), AnimationError> {
        if self.local.len() != skeleton.joints.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        if !rigs_match(&self.rig, &skeleton.joints) {
            return Err(AnimationError::SkeletonMismatch);
        }
        let mut global = [Mat4::IDENTITY; MAX_JOINTS];
        skin.clear();
        skin.reserve(self.local.len());
        for (index, (local, joint)) in self.local.iter().zip(skeleton.joints.iter()).enumerate() {
            if !local.is_valid() {
                return Err(AnimationError::InvalidPose(index));
            }
            let matrix = joint.parent.map_or_else(
                || local.matrix(),
                |parent| global[usize::from(parent)] * local.matrix(),
            );
            let palette = matrix * joint.inverse_bind;
            if !usable_linear(matrix.as_dmat4()) || !usable_linear(palette.as_dmat4()) {
                return Err(AnimationError::InvalidPose(index));
            }
            global[index] = matrix;
            skin.push(palette);
        }
        Ok(())
    }

    /// Produces `global_joint * inverse_bind` matrices in skeleton order.
    ///
    /// # Errors
    ///
    /// Rejects a pose created for a different skeleton, invalid blend data,
    /// or overflowing global/palette matrices.
    pub fn skin_matrices(&self, skeleton: &Skeleton) -> Result<Vec<Mat4>, AnimationError> {
        let mut skin = Vec::with_capacity(self.local.len());
        self.skin_matrices_vec_into(skeleton, &mut skin)?;
        Ok(skin)
    }

    /// Blends two local poses with clamped weight and shortest-path quaternion interpolation.
    ///
    /// # Errors
    ///
    /// Rejects different pose lengths, a non-finite weight, or invalid input/result TRS.
    pub fn blend(a: &Self, b: &Self, weight: f32) -> Result<Self, AnimationError> {
        let mut out = Self {
            rig: a.rig.clone(),
            local: Vec::with_capacity(a.local.len()),
        };
        Self::blend_into(a, b, weight, &mut out)?;
        Ok(out)
    }

    /// Blends two local poses directly into `out`, reusing `out`'s heap capacity.
    ///
    /// # Errors
    ///
    /// Rejects different pose lengths, a non-finite weight, or invalid input/result TRS.
    pub fn blend_into(
        a: &Self,
        b: &Self,
        weight: f32,
        out: &mut Self,
    ) -> Result<(), AnimationError> {
        if a.local.len() != b.local.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        if !rigs_match(&a.rig, &b.rig) {
            return Err(AnimationError::SkeletonMismatch);
        }
        if !weight.is_finite() {
            return Err(AnimationError::InvalidBlendWeight);
        }
        let weight = weight.clamp(0.0, 1.0);
        if !Arc::ptr_eq(&out.rig, &a.rig) {
            out.rig = a.rig.clone();
        }
        out.local.clear();
        out.local.reserve(a.local.len());
        for (index, (a, b)) in a.local.iter().zip(&b.local).enumerate() {
            if !a.is_valid() || !b.is_valid() {
                return Err(AnimationError::InvalidPose(index));
            }
            let value = Transform {
                translation: lerp_vec3(a.translation, b.translation, weight),
                rotation: a.rotation.slerp(b.rotation, weight).normalize(),
                scale: lerp_vec3(a.scale, b.scale, weight),
            };
            if !value.is_valid() {
                return Err(AnimationError::InvalidPose(index));
            }
            out.local.push(value);
        }
        Ok(())
    }
}
