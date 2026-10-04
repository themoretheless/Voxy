//! Explicit local-channel transport between immutable rig bindings.
use super::{
    AnimationError, AnimatorFrame, Joint, MAX_JOINTS, Pose, Skeleton, Transform, rigs_match,
};
use crate::exact_quaternion as exact_basis;
use glam::Quat;
use std::sync::Arc;

/// Axis corrections are authored, never inferred from bone names or lengths.
#[derive(Clone, Debug)]
pub struct RetargetJoint {
    pub source: Arc<str>,
    pub target: Arc<str>,
    /// Conjugates the animated rotation relative to the source bind rotation.
    pub rotation_basis: Quat,
    /// Converts parent-local translation deltas and extracted root displacement.
    pub translation_basis: Quat,
    pub translation_scale: f32,
}
#[derive(Clone, Debug)]
struct BoundJoint {
    source: usize,
    target: usize,
    rotation_basis: Quat,
    translation_basis: Quat,
    translation_scale: f32,
    /// Compiled exact coefficient pattern; absent for pose-only mappings whose
    /// translation and rotation channels do not share a rigid similarity.
    root_basis_nonzero: Option<[[bool; 3]; 3]>,
}
/// Compiled one-to-one channel mapping. Unmapped target joints retain bind pose.
/// This transports local authored channels, not world-space end-effector goals.
#[derive(Clone, Debug)]
pub struct RetargetBinding {
    source: Arc<[Joint]>,
    target: Arc<[Joint]>,
    joints: Vec<BoundJoint>,
}
/// Original-source to target-parent root frame. Applying this similarity to
/// source points accounts for both bind offsets and translation units. A
/// relative rigid trajectory is conjugated by it; it is not a target pose.
#[derive(Clone, Copy, Debug)]
pub struct RetargetRootSimilarity {
    pub target_joint: u16,
    pub source_axes: [bool; 3],
    pub frame: crate::RootRigidEnclosure,
    pub scale: crate::RootUniformScaleEnclosure,
}
impl RetargetRootSimilarity {
    /// Maps the target-parent frame into a caller-qualified static ancestor
    /// similarity. Rotation and signed uniform scale compose without a matrix
    /// cast or a rounded offset replacing the original interval proof.
    /// # Errors
    /// Rejects noninvertible parent scales and nonfinite interval operations.
    pub fn in_parent(
        self,
        parent: crate::RootRigidEnclosure,
        parent_scale: crate::RootUniformScaleEnclosure,
    ) -> Result<Self, AnimationError> {
        Ok(Self {
            frame: parent.compose(&self.frame.with_translation_scale_enclosed(parent_scale)?)?,
            scale: parent_scale.multiplied(self.scale)?,
            ..self
        })
    }
}
impl RetargetBinding {
    /// # Errors
    /// Rejects empty/oversized maps, absent names, repeated source/target ownership,
    /// nonunit axis corrections and nonfinite/out-of-range translation scale.
    pub fn new(
        source: &Skeleton,
        target: &Skeleton,
        mapping: &[RetargetJoint],
    ) -> Result<Self, AnimationError> {
        if mapping.is_empty() || mapping.len() > MAX_JOINTS {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        let mut joints: Vec<BoundJoint> = Vec::with_capacity(mapping.len());
        for entry in mapping {
            if !entry.rotation_basis.is_finite()
                || !entry.rotation_basis.is_normalized()
                || !entry.translation_basis.is_finite()
                || !entry.translation_basis.is_normalized()
                || !entry.translation_scale.is_finite()
                || !(0. ..=1e6).contains(&entry.translation_scale)
            {
                return Err(AnimationError::InvalidRetargetBinding);
            }
            let source_index = source
                .joints
                .iter()
                .position(|joint| joint.name == entry.source)
                .ok_or(AnimationError::InvalidRetargetBinding)?;
            let target_index = target
                .joints
                .iter()
                .position(|joint| joint.name == entry.target)
                .ok_or(AnimationError::InvalidRetargetBinding)?;
            if joints
                .iter()
                .any(|joint| joint.source == source_index || joint.target == target_index)
            {
                return Err(AnimationError::InvalidRetargetBinding);
            }
            joints.push(BoundJoint {
                source: source_index,
                target: target_index,
                rotation_basis: entry.rotation_basis,
                translation_basis: entry.translation_basis,
                translation_scale: entry.translation_scale,
                root_basis_nonzero: exact_basis::coherent(
                    target.joints[target_index].bind_local.rotation,
                    entry.rotation_basis,
                    source.joints[source_index].bind_local.rotation,
                    entry.translation_basis,
                )
                .then(|| exact_basis::rotation_nonzero(entry.translation_basis)),
            });
        }
        Ok(Self {
            source: source.joints.clone(),
            target: target.joints.clone(),
            joints,
        })
    }
    /// Builds a target-bound candidate without modifying either rig or source pose.
    /// Relative scale channels preserve target bind proportions and signed scales.
    /// # Errors
    /// Rejects a foreign rig, invalid source channels or target palette overflow.
    pub fn apply_pose(&self, pose: &Pose) -> Result<Pose, AnimationError> {
        let result = self.transport_pose(pose)?;
        result.skin_matrices(&Skeleton {
            joints: self.target.clone(),
        })?;
        Ok(result)
    }
    fn transport_pose(&self, pose: &Pose) -> Result<Pose, AnimationError> {
        self.validate_source_pose(pose)?;
        let mut local: Vec<Transform> = self.target.iter().map(|joint| joint.bind_local).collect();
        for joint in &self.joints {
            let animated = pose.local[joint.source];
            let source = self.source[joint.source].bind_local;
            let target = self.target[joint.target].bind_local;
            let delta = source.rotation.inverse() * animated.rotation;
            local[joint.target] = Transform {
                translation: (target.translation.as_dvec3()
                    + joint.translation_basis.as_dquat()
                        * (animated.translation.as_dvec3() - source.translation.as_dvec3())
                        * f64::from(joint.translation_scale))
                .as_vec3(),
                rotation: (target.rotation
                    * joint.rotation_basis
                    * delta
                    * joint.rotation_basis.inverse())
                .normalize(),
                scale: (target.scale.as_dvec3() * animated.scale.as_dvec3()
                    / source.scale.as_dvec3())
                .as_vec3(),
            };
        }
        Ok(Pose {
            rig: self.target.clone(),
            local,
        })
    }
    fn validate_source_pose(&self, pose: &Pose) -> Result<(), AnimationError> {
        if !rigs_match(&pose.rig, &self.source) {
            return Err(AnimationError::SkeletonMismatch);
        }
        if pose.local.len() != self.source.len() {
            return Err(AnimationError::PoseCountMismatch);
        }
        for (index, local) in pose.local.iter().enumerate() {
            if !local.is_valid() {
                return Err(AnimationError::InvalidPose(index));
            }
        }
        Ok(())
    }
    fn mapped_joint(&self, source_joint: u16) -> Result<&BoundJoint, AnimationError> {
        self.joints
            .iter()
            .find(|joint| joint.source == usize::from(source_joint))
            .ok_or(AnimationError::InvalidRetargetBinding)
    }
    fn rigid_motion_joint(&self, source_joint: u16) -> Result<(&BoundJoint, Quat), AnimationError> {
        let joint = self.mapped_joint(source_joint)?;
        if joint.root_basis_nonzero.is_none() {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        Ok((joint, joint.translation_basis))
    }
    /// Encloses the ideal translation and unit rotation of one mapped joint
    /// from an accepted stored source pose. Scale channels are not included.
    /// Normalizes original quaternion inputs in real arithmetic; downstream
    /// point discrepancy includes actual f32 retarget arithmetic and publication.
    /// This is a pose certificate, not a uniform trajectory/source sampling cap.
    /// # Errors
    /// Rejects foreign/invalid poses, absent mappings and interval overflow.
    pub fn joint_rigid_pose_enclosure(
        &self,
        pose: &Pose,
        source_joint: u16,
    ) -> Result<(u16, crate::RootRigidEnclosure), AnimationError> {
        use crate::{RootRigidEnclosure, RootRigidTransform};
        self.validate_source_pose(pose)?;
        let joint = self.mapped_joint(source_joint)?;
        let animated = pose.local[joint.source];
        self.joint_rigid_field_enclosure(
            source_joint,
            RootRigidEnclosure::from_transform(RootRigidTransform {
                translation: animated.translation.as_dvec3(),
                rotation: animated.rotation.as_dquat(),
            })?,
        )
    }
    /// Transports every ideal local pose represented by a caller-qualified
    /// source enclosure, including a whole continuous interval. This input is
    /// an absolute local pose, not a relative root-motion factor. The caller
    /// must retain its source rig/joint identity and temporal coverage proof.
    /// Scale channels and actual floating evaluation error remain separate.
    /// # Errors
    /// Rejects absent mappings and overflowing enclosure arithmetic.
    pub fn joint_rigid_field_enclosure(
        &self,
        source_joint: u16,
        animated: crate::RootRigidEnclosure,
    ) -> Result<(u16, crate::RootRigidEnclosure), AnimationError> {
        use crate::{RootRigidEnclosure, RootRigidTransform};
        let joint = self.mapped_joint(source_joint)?;
        let source = self.source[joint.source].bind_local;
        let target = self.target[joint.target].bind_local;
        let rotation = |q: Quat| {
            RootRigidEnclosure::from_transform(RootRigidTransform {
                rotation: q.as_dquat(),
                ..RootRigidTransform::IDENTITY
            })
        };
        let translation = |v: glam::Vec3| {
            RootRigidEnclosure::from_transform(RootRigidTransform {
                translation: v.as_dvec3(),
                ..RootRigidTransform::IDENTITY
            })
        };
        let correction = rotation(joint.rotation_basis)?;
        let unit_rotation = rotation(target.rotation)?
            .compose(&correction)?
            .compose(&rotation(source.rotation)?.inverse()?)?
            .compose(&RootRigidEnclosure::IDENTITY.with_rotation_from(animated))?
            .compose(&correction.inverse()?)?;
        let position = if joint.translation_scale == 0. {
            translation(target.translation)?
        } else {
            let delta = animated
                .with_rotation_from(RootRigidEnclosure::IDENTITY)
                .compose(&translation(-source.translation)?)?
                .with_translation_scale(f64::from(joint.translation_scale))?;
            translation(target.translation)?
                .compose(&rotation(joint.translation_basis)?)?
                .compose(&delta)?
        };
        Ok((
            u16::try_from(joint.target).map_err(|_| AnimationError::InvalidRetargetBinding)?,
            position.with_rotation_from(unit_rotation),
        ))
    }
    /// Transports an original clip's absolute local pose interval, retaining
    /// source rig identity and initial/bind offsets. Scale channels are excluded.
    /// # Errors
    /// Rejects foreign clips, absent mappings, invalid intervals and overflow.
    /// Returns None when the source cannot supply a continuous pose enclosure.
    pub fn joint_rigid_clip_interval_enclosure(
        &self,
        clip: &crate::AnimationClip,
        source_joint: u16,
        times: [f64; 2],
    ) -> Result<Option<(u16, crate::RootRigidEnclosure)>, AnimationError> {
        if !rigs_match(&clip.rig, &self.source) {
            return Err(AnimationError::SkeletonMismatch);
        }
        self.mapped_joint(source_joint)?;
        clip.joint_rigid_pose_interval_enclosure(source_joint, times)?
            .map(|source| self.joint_rigid_field_enclosure(source_joint, source))
            .transpose()
    }
    /// Uniform same-time translation discrepancy over an incoming source domain.
    /// source_error must bound source evaluation against those original positions;
    /// zero qualifies retarget arithmetic alone on exact stored source positions.
    /// Includes the fixed raw-vs-unit basis difference, actual f64 subtraction,
    /// glam rotation, scale/addition and final f32 publication. Temporal variation
    /// of the position domain is not itself counted as error. Rotation/scale
    /// channels and ancestor/scene arithmetic are separate.
    /// # Errors
    /// Rejects absent mappings, invalid domains/errors and nonfinite publication.
    pub fn joint_translation_evaluation_error_bounds(
        &self,
        source_joint: u16,
        position: [[f64; 2]; 3],
        source_error: [f64; 3],
    ) -> Result<(u16, ([f64; 3], f64)), AnimationError> {
        let joint = self.mapped_joint(source_joint)?;
        let bounds = crate::RootRigidEnclosure::retarget_translation_evaluation_error(
            position,
            source_error,
            self.source[joint.source].bind_local.translation.as_dvec3(),
            self.target[joint.target].bind_local.translation.as_dvec3(),
            joint.translation_basis.as_dquat(),
            f64::from(joint.translation_scale),
        )?;
        Ok((
            u16::try_from(joint.target).map_err(|_| AnimationError::InvalidRetargetBinding)?,
            bounds,
        ))
    }
    /// Uniform component discrepancy of the actual f32 retarget rotation from
    /// its ideal unit-quaternion chain. source_error must qualify the incoming
    /// animated quaternion against its same-time ideal unit source. Covers raw
    /// fixed key norms, four Hamilton products and final f32 normalization.
    /// Does not qualify source sampling, scale or ancestor/scene arithmetic.
    /// # Errors
    /// Rejects absent mappings, invalid errors and insufficient norm separation.
    pub fn joint_rotation_evaluation_error_bounds(
        &self,
        source_joint: u16,
        source_error: [f64; 4],
    ) -> Result<(u16, [f64; 4]), AnimationError> {
        let joint = self.mapped_joint(source_joint)?;
        let error = crate::RootRigidEnclosure::retarget_rotation_evaluation_error(
            self.target[joint.target].bind_local.rotation.to_array(),
            joint.rotation_basis.to_array(),
            self.source[joint.source].bind_local.rotation.to_array(),
            source_error,
        )?;
        Ok((
            u16::try_from(joint.target).map_err(|_| AnimationError::InvalidRetargetBinding)?,
            error,
        ))
    }
    /// Qualifies held/default/STEP and stationary LINEAR source sampling and actual retarget rotation
    /// together, retaining the clip's full source-rig identity. None requires
    /// moving source interpolation or key-partition qualification.
    /// # Errors
    /// Rejects foreign clips, absent mappings and invalid source intervals.
    pub fn joint_rotation_clip_sample_error_bounds(
        &self,
        clip: &crate::AnimationClip,
        source_joint: u16,
        times: [f32; 2],
    ) -> Result<Option<(u16, [f64; 4])>, AnimationError> {
        if !rigs_match(&clip.rig, &self.source) {
            return Err(AnimationError::SkeletonMismatch);
        }
        self.mapped_joint(source_joint)?;
        clip.joint_rotation_sample_error_bounds(source_joint, times)?
            .map(|error| self.joint_rotation_evaluation_error_bounds(source_joint, error))
            .transpose()
    }
    /// Resolves the source mask corresponding to target parent-local axes.
    /// # Errors
    /// Rejects mappings whose selected subspace needs a nondiagonal source mask.
    /// Tests exact zero coefficients, never an angular tolerance or axis snapping.
    pub fn source_root_motion_axes(
        &self,
        source_joint: u16,
        target_axes: [bool; 3],
    ) -> Result<[bool; 3], AnimationError> {
        let (joint, _) = self.rigid_motion_joint(source_joint)?;
        let nonzero = joint
            .root_basis_nonzero
            .ok_or(AnimationError::InvalidRetargetBinding)?;
        let mut source_axes = [false; 3];
        for axis in 0..3 {
            let mut selected = None;
            for row in 0..3 {
                if nonzero[row][axis] {
                    if selected.is_some_and(|flag| flag != target_axes[row]) {
                        return Err(AnimationError::InvalidRetargetBinding);
                    }
                    selected = Some(target_axes[row]);
                }
            }
            source_axes[axis] = selected.ok_or(AnimationError::InvalidRetargetBinding)?;
        }
        Ok(source_axes)
    }
    /// Transports a selected joint's ordered rigid trajectory into target coordinates.
    /// The source path must be generated with `source_root_motion_axes` for this mask.
    /// # Errors
    /// Requires a common rigid basis and a representable target/source axis mask.
    pub fn apply_root_path(
        &self,
        path: &crate::RootRigidPath,
        source_joint: u16,
        axes: [bool; 3],
    ) -> Result<crate::RootRigidPath, AnimationError> {
        self.source_root_motion_axes(source_joint, axes)?;
        let (joint, basis) = self.rigid_motion_joint(source_joint)?;
        let source = self.source[joint.source].bind_local;
        let target = self.target[joint.target].bind_local;
        let scale = f64::from(joint.translation_scale);
        let basis = basis.as_dquat().normalize();
        let offset =
            target.translation.as_dvec3() - scale * (basis * source.translation.as_dvec3());
        path.transformed(basis, scale, offset)
    }
    /// Encloses the common similarity from original bind values before any
    /// quaternion normalization, offset calculation or matrix rounding.
    /// # Errors
    /// Requires coherent rigid channels, a representable mask and nonzero scale.
    pub fn root_similarity_enclosure(
        &self,
        source_joint: u16,
        target_axes: [bool; 3],
    ) -> Result<RetargetRootSimilarity, AnimationError> {
        use crate::{RootRigidEnclosure, RootRigidTransform, RootUniformScaleEnclosure};
        let source_axes = self.source_root_motion_axes(source_joint, target_axes)?;
        let (joint, basis) = self.rigid_motion_joint(source_joint)?;
        if joint.translation_scale == 0. {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        let scale = RootUniformScaleEnclosure::from_scale(f64::from(joint.translation_scale))?;
        let outer = RootRigidEnclosure::from_transform(RootRigidTransform {
            translation: self.target[joint.target].bind_local.translation.as_dvec3(),
            rotation: basis.as_dquat(),
        })?;
        let source_origin = RootRigidEnclosure::from_transform(RootRigidTransform {
            translation: -self.source[joint.source].bind_local.translation.as_dvec3(),
            ..RootRigidTransform::IDENTITY
        })?
        .with_translation_scale_enclosed(scale)?;
        Ok(RetargetRootSimilarity {
            target_joint: u16::try_from(joint.target)
                .map_err(|_| AnimationError::InvalidRetargetBinding)?,
            source_axes,
            frame: outer.compose(&source_origin)?,
            scale,
        })
    }
    /// Transfers only pose channels. The returned frame has no extracted motion.
    /// Use this when root motion consumption is disabled; no mapped root is required.
    pub fn apply_pose_frame(&self, frame: &AnimatorFrame) -> Result<AnimatorFrame, AnimationError> {
        if !frame.transition_weight.is_finite() || !(0. ..=1.).contains(&frame.transition_weight) {
            return Err(AnimationError::InvalidBlendWeight);
        }
        let pose = self.transport_pose(&frame.pose)?;
        let skin_matrices = pose.skin_matrices(&Skeleton {
            joints: self.target.clone(),
        })?;
        Ok(AnimatorFrame {
            pose,
            skin_matrices,
            root_motion: glam::Vec3::ZERO,
            root_motion_joint: 0,
            transition_weight: frame.transition_weight,
        })
    }
    /// Transfers pose and extracted parent-local root displacement as one candidate.
    /// No root motion is inferred from the difference between bind translations.
    /// # Errors
    /// Also requires a mapped selected motion joint and valid frame metadata.
    pub fn apply_frame(&self, frame: &AnimatorFrame) -> Result<AnimatorFrame, AnimationError> {
        if !frame.root_motion.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        if !frame.transition_weight.is_finite() || !(0. ..=1.).contains(&frame.transition_weight) {
            return Err(AnimationError::InvalidBlendWeight);
        }
        let root = self
            .joints
            .iter()
            .find(|joint| joint.source == usize::from(frame.root_motion_joint))
            .ok_or(AnimationError::InvalidRetargetBinding)?;
        let pose = self.transport_pose(&frame.pose)?;
        let root_motion = (root.translation_basis.as_dquat()
            * frame.root_motion.as_dvec3()
            * f64::from(root.translation_scale))
        .as_vec3();
        if !root_motion.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        let skin_matrices = pose.skin_matrices(&Skeleton {
            joints: self.target.clone(),
        })?;
        Ok(AnimatorFrame {
            pose,
            skin_matrices,
            root_motion,
            root_motion_joint: root.target as u16,
            transition_weight: frame.transition_weight,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnimationClip, Animator, JointTrack, Playback, QuatKey, Vec3Key};
    use glam::{Mat4, Vec3};
    fn rigs() -> (Skeleton, Skeleton) {
        let source = Skeleton::new(vec![Joint {
            name: "source.root".into(),
            parent: None,
            bind_local: Transform {
                translation: Vec3::X,
                rotation: Quat::from_rotation_y(0.3),
                scale: Vec3::new(1., 2., -1.),
            },
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap();
        let root = Transform {
            translation: Vec3::Y * 10.,
            rotation: Quat::from_rotation_z(0.5),
            scale: Vec3::new(-2., 3., 1.),
        };
        let helper = Transform {
            translation: Vec3::new(0.5, 0.5, 0.),
            ..Transform::IDENTITY
        };
        let target = Skeleton::new(vec![
            Joint {
                name: "decoration".into(),
                parent: None,
                bind_local: Transform::IDENTITY,
                inverse_bind: Mat4::IDENTITY,
            },
            Joint {
                name: "target.root".into(),
                parent: None,
                bind_local: root,
                inverse_bind: root.matrix().inverse(),
            },
            Joint {
                name: "helper".into(),
                parent: Some(1),
                bind_local: helper,
                inverse_bind: (root.matrix() * helper.matrix()).inverse(),
            },
        ])
        .unwrap();
        (source, target)
    }
    fn entry() -> RetargetJoint {
        RetargetJoint {
            source: "source.root".into(),
            target: "target.root".into(),
            rotation_basis: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            translation_basis: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            translation_scale: 2.,
        }
    }
    #[test]
    fn pose_only_partial_mapping_does_not_require_the_unused_motion_joint() {
        let (source, target) = rigs();
        let mut joints = source.joints().to_vec();
        let mut arm = joints[0].clone();
        arm.name = "arm".into();
        arm.parent = Some(0);
        joints.push(arm);
        let source = Skeleton::new(joints).unwrap();
        let mut mapping = entry();
        mapping.source = "arm".into();
        let binding = RetargetBinding::new(&source, &target, &[mapping]).unwrap();
        let mut pose = source.bind_pose();
        pose.local[1].translation += Vec3::X;
        let frame = AnimatorFrame {
            skin_matrices: pose.skin_matrices(&source).unwrap(),
            pose,
            root_motion: Vec3::X,
            root_motion_joint: 0,
            transition_weight: 0.4,
        };
        assert!(binding.apply_frame(&frame).is_err());
        let converted = binding.apply_pose_frame(&frame).unwrap();
        assert!((converted.pose.local()[1].translation - (Vec3::Y * 12.)).length() < 1e-5);
        assert_eq!(converted.root_motion, Vec3::ZERO);
        assert_eq!(converted.transition_weight, 0.4);
        assert_eq!(converted.pose.local()[0], target.joints()[0].bind_local);
    }
    #[test]
    fn bind_relative_channels_preserve_target_proportions_and_drive_its_palette() {
        let (source, target) = rigs();
        let binding = RetargetBinding::new(&source, &target, &[entry()]).unwrap();
        let bind = source.joints()[0].bind_local;
        let clip = AnimationClip::new(
            "motion",
            1.,
            Playback::Clamp,
            vec![JointTrack {
                translations: vec![
                    Vec3Key {
                        time: 0.,
                        value: bind.translation,
                    },
                    Vec3Key {
                        time: 1.,
                        value: bind.translation + Vec3::X,
                    },
                ],
                rotations: vec![
                    QuatKey {
                        time: 0.,
                        value: bind.rotation,
                    },
                    QuatKey {
                        time: 1.,
                        value: bind.rotation * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                    },
                ],
                scales: vec![
                    Vec3Key {
                        time: 0.,
                        value: bind.scale,
                    },
                    Vec3Key {
                        time: 1.,
                        value: bind.scale * Vec3::new(2., 0.5, 1.),
                    },
                ],
            }],
            &source,
        )
        .unwrap();
        let mut animator = Animator::new(Arc::new(clip));
        let original = animator.advance(&source, 0.5).unwrap();
        let snapshot = original.clone();
        let converted = binding.apply_frame(&original).unwrap();
        assert_eq!(original.pose, snapshot.pose);
        assert_eq!(original.root_motion, snapshot.root_motion);
        assert_eq!(converted.root_motion_joint, 1);
        assert!(converted.root_motion.abs_diff_eq(Vec3::Y, 1e-6));
        assert!(
            converted.pose.local()[1]
                .translation
                .abs_diff_eq(Vec3::Y * 11., 1e-6)
        );
        assert!(
            converted.pose.local()[1]
                .scale
                .abs_diff_eq(Vec3::new(-3., 2.25, 1.), 1e-6)
        );
        let expected_rotation =
            Quat::from_rotation_z(0.5) * Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
        assert!(
            converted.pose.local()[1]
                .rotation
                .abs_diff_eq(expected_rotation, 1e-6)
        );
        for index in [0, 2] {
            assert_eq!(
                converted.pose.local()[index],
                target.joints()[index].bind_local
            );
        }
        let expected = Mat4::from_scale_rotation_translation(
            Vec3::new(-3., 2.25, 1.),
            expected_rotation,
            Vec3::Y * 11.,
        );
        assert!(
            converted.skin_matrices[1]
                .abs_diff_eq(expected * target.joints()[1].inverse_bind, 3e-6)
        );
        assert!(converted.skin_matrices[2].abs_diff_eq(
            expected * target.joints()[2].bind_local.matrix() * target.joints()[2].inverse_bind,
            3e-6
        ));
        assert!(converted.pose.skin_matrices(&source).is_err());
    }
    #[test]
    fn bind_pose_and_independently_reconstructed_source_are_admitted() {
        let (source, target) = rigs();
        let binding = RetargetBinding::new(&source, &target, &[entry()]).unwrap();
        let rebuilt = Skeleton::new(source.joints().to_vec()).unwrap();
        let converted = binding.apply_pose(&rebuilt.bind_pose()).unwrap();
        for (actual, expected) in converted.local().iter().zip(target.joints()) {
            assert!(
                actual
                    .matrix()
                    .abs_diff_eq(expected.bind_local.matrix(), 2e-6)
            );
        }
    }
    #[test]
    fn malformed_mapping_and_foreign_pose_are_rejected_without_mutation() {
        let (source, target) = rigs();
        assert!(RetargetBinding::new(&source, &target, &[]).is_err());
        assert!(RetargetBinding::new(&source, &target, &[entry(), entry()]).is_err());
        let mut bad = entry();
        bad.source = "absent".into();
        assert!(RetargetBinding::new(&source, &target, &[bad]).is_err());
        for scale in [f32::NAN, -1., 1e7] {
            let mut bad = entry();
            bad.translation_scale = scale;
            assert!(RetargetBinding::new(&source, &target, &[bad]).is_err());
        }
        let mut bad = entry();
        bad.rotation_basis = Quat::from_xyzw(0., 0., 0., 2.);
        assert!(RetargetBinding::new(&source, &target, &[bad]).is_err());
        let binding = RetargetBinding::new(&source, &target, &[entry()]).unwrap();
        let pose = target.bind_pose();
        let snapshot = pose.clone();
        assert!(binding.apply_pose(&pose).is_err());
        assert_eq!(pose, snapshot);
        let mut invalid = source.bind_pose();
        invalid.local[0].scale = Vec3::ZERO;
        assert!(binding.apply_pose(&invalid).is_err());
        let frame = AnimatorFrame {
            pose: source.bind_pose(),
            skin_matrices: vec![],
            root_motion: Vec3::ZERO,
            root_motion_joint: 1,
            transition_weight: 1.,
        };
        assert!(binding.apply_frame(&frame).is_err());
    }
    #[test]
    fn overflowing_transport_rejects_without_mutating_input_or_poisoning_binding() {
        let (source, target) = rigs();
        let mut mapping = entry();
        mapping.translation_scale = 1e6;
        let binding = RetargetBinding::new(&source, &target, &[mapping]).unwrap();
        let mut pose = source.bind_pose();
        pose.local[0].translation = Vec3::splat(f32::MAX);
        let snapshot = pose.clone();
        assert!(binding.apply_pose(&pose).is_err());
        assert_eq!(pose, snapshot);
        let mut frame = AnimatorFrame {
            pose: source.bind_pose(),
            skin_matrices: vec![],
            root_motion: Vec3::splat(f32::MAX),
            root_motion_joint: 0,
            transition_weight: 1.,
        };
        assert!(binding.apply_frame(&frame).is_err());
        frame.root_motion = Vec3::ZERO;
        assert_eq!(binding.apply_frame(&frame).unwrap().root_motion, Vec3::ZERO);
    }
    #[test]
    fn held_original_rotation_sample_error_feeds_retarget_with_source_identity() {
        let rig = |name: &str, rotation| {
            Skeleton::new(vec![Joint {
                name: name.into(),
                parent: None,
                bind_local: Transform {
                    rotation,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            }])
            .unwrap()
        };
        let h = core::f32::consts::FRAC_1_SQRT_2;
        let rotations = [
            Quat::IDENTITY,
            Quat::from_xyzw(0., 0., h, h),
            Quat::from_euler(glam::EulerRot::XYZ, 0.2, -0.4, 0.7),
            Quat::from_xyzw(0., 0., 0., 1_f32.next_up()),
        ];
        for source_q in rotations {
            let source = rig("s", source_q);
            for correction in rotations {
                let target_q = -correction;
                let target = rig("t", target_q);
                let binding = RetargetBinding::new(
                    &source,
                    &target,
                    &[RetargetJoint {
                        source: "s".into(),
                        target: "t".into(),
                        rotation_basis: correction,
                        translation_basis: Quat::IDENTITY,
                        translation_scale: 1.,
                    }],
                )
                .unwrap();
                let tracks = vec![crate::JointTrack {
                    rotations: vec![
                        crate::QuatKey {
                            time: 0.25,
                            value: source_q,
                        },
                        crate::QuatKey {
                            time: 0.75,
                            value: correction,
                        },
                    ],
                    ..crate::JointTrack::default()
                }];
                let clip = crate::AnimationClip::new_with_interpolation(
                    "step",
                    1.,
                    crate::Playback::Clamp,
                    tracks.clone(),
                    vec![crate::TrackInterpolation {
                        rotation: crate::Interpolation::Step,
                        ..crate::TrackInterpolation::default()
                    }],
                    &source,
                )
                .unwrap();
                for times in [[0., 0.1], [0.25, 0.5], [0.75, 1.]] {
                    let source_error = clip
                        .joint_rotation_sample_error_bounds(0, times)
                        .unwrap()
                        .unwrap();
                    let (_, caps) = binding
                        .joint_rotation_clip_sample_error_bounds(&clip, 0, times)
                        .unwrap()
                        .unwrap();
                    let pose = clip.sample(&source, times[0]);
                    let animated = pose.local()[0].rotation;
                    let actual = binding.apply_pose(&pose).unwrap().local()[0].rotation;
                    println!(
                        "held_rotation_error_reference={{\"source_bind\":{:?},\"target_bind\":{:?},\"correction\":{:?},\"animated\":{:?},\"source_error\":{:?},\"actual\":{:?},\"caps\":{:?}}}",
                        source_q.to_array(),
                        target_q.to_array(),
                        correction.to_array(),
                        animated.to_array(),
                        source_error,
                        actual.to_array(),
                        caps
                    );
                }
                assert_eq!(
                    binding
                        .joint_rotation_clip_sample_error_bounds(&clip, 0, [0.5, 0.75])
                        .unwrap(),
                    None
                );
                let linear = crate::AnimationClip::new(
                    "linear",
                    1.,
                    crate::Playback::Clamp,
                    tracks,
                    &source,
                )
                .unwrap();
                let stationary = source_q
                    .as_dquat()
                    .normalize()
                    .dot(correction.as_dquat().normalize())
                    .abs()
                    > 1. - 1e-12;
                assert_eq!(
                    binding
                        .joint_rotation_clip_sample_error_bounds(&linear, 0, [0.25, 0.5])
                        .unwrap()
                        .is_some(),
                    stationary
                );
                assert!(
                    binding
                        .joint_rotation_clip_sample_error_bounds(&linear, 0, [0., 0.1])
                        .unwrap()
                        .is_some()
                );
                let fallback = crate::AnimationClip::new(
                    "fallback",
                    1.,
                    crate::Playback::Clamp,
                    vec![crate::JointTrack::default()],
                    &source,
                )
                .unwrap();
                assert!(
                    binding
                        .joint_rotation_clip_sample_error_bounds(&fallback, 0, [0., 1.])
                        .unwrap()
                        .is_some()
                );
                let foreign = crate::AnimationClip::new(
                    "foreign",
                    1.,
                    crate::Playback::Clamp,
                    vec![crate::JointTrack::default()],
                    &rig("foreign", source_q),
                )
                .unwrap();
                assert!(
                    binding
                        .joint_rotation_clip_sample_error_bounds(&foreign, 0, [0., 1.])
                        .is_err()
                );
                assert!(
                    binding
                        .joint_rotation_clip_sample_error_bounds(&clip, 1, [0., 1.])
                        .is_err()
                );
                assert!(
                    clip.joint_rotation_sample_error_bounds(0, [0., f32::NAN])
                        .is_err()
                );
                assert!(
                    clip.joint_rotation_sample_error_bounds(0, [0.5, 0.25])
                        .is_err()
                );
            }
        }
    }
    #[test]
    fn original_linear_translation_sample_errors_feed_retarget_caps() {
        let rig = |name: &str, translation| {
            Skeleton::new(vec![Joint {
                name: name.into(),
                parent: None,
                bind_local: Transform {
                    translation,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            }])
            .unwrap()
        };
        let source = rig("s", Vec3::new(0.6, -0.2, 0.4));
        let target = rig("t", Vec3::new(-1., 2., 3.));
        let basis = Quat::from_euler(glam::EulerRot::XYZ, 0.2, -0.4, 0.7);
        for keys in [[0., 1.], [0.1, 0.9], [0.125, 0.625]] {
            for (from, to) in [
                (Vec3::new(4., -2., 1.), Vec3::new(-8., 9., -9.)),
                (Vec3::new(1e6, -1e6, 0.1), Vec3::new(-1e6, 1e6, -0.2)),
                (
                    Vec3::new(1e-30, -1e-30, 0.),
                    Vec3::new(-2e-30, 3e-30, 1e-30),
                ),
            ] {
                let clip = crate::AnimationClip::new(
                    "source",
                    keys[1],
                    crate::Playback::Clamp,
                    vec![crate::JointTrack {
                        translations: vec![
                            crate::Vec3Key {
                                time: keys[0],
                                value: from,
                            },
                            crate::Vec3Key {
                                time: keys[1],
                                value: to,
                            },
                        ],
                        ..crate::JointTrack::default()
                    }],
                    &source,
                )
                .unwrap();
                let source_caps = clip
                    .joint_translation_sample_error_bounds(0, keys)
                    .unwrap()
                    .unwrap();
                let domain = core::array::from_fn(|i| {
                    [f64::from(from[i].min(to[i])), f64::from(from[i].max(to[i]))]
                });
                for scale in [2., 1e6] {
                    let binding = RetargetBinding::new(
                        &source,
                        &target,
                        &[RetargetJoint {
                            source: "s".into(),
                            target: "t".into(),
                            rotation_basis: Quat::IDENTITY,
                            translation_basis: basis,
                            translation_scale: scale,
                        }],
                    )
                    .unwrap();
                    let (_, (caps, radius)) = binding
                        .joint_translation_evaluation_error_bounds(0, domain, source_caps)
                        .unwrap();
                    for step in 0..=16 {
                        let time = if step == 16 {
                            keys[1]
                        } else {
                            keys[0] + (keys[1] - keys[0]) * (step as f32 / 16.)
                        };
                        let pose = clip.sample(&source, time);
                        let actual_source = pose.local()[0].translation;
                        let actual = binding.apply_pose(&pose).unwrap().local()[0].translation;
                        println!(
                            "source_linear_sample_reference={{\"from\":{:?},\"to\":{:?},\"keys\":{:?},\"time\":{:?},\"source_actual\":{:?},\"source_caps\":{:?},\"source_bind\":{:?},\"target_bind\":{:?},\"basis\":{:?},\"scale\":{:?},\"actual\":{:?},\"caps\":{:?},\"radius\":{:?}}}",
                            from.to_array(),
                            to.to_array(),
                            keys,
                            time,
                            actual_source.to_array(),
                            source_caps,
                            source.joints()[0].bind_local.translation.to_array(),
                            target.joints()[0].bind_local.translation.to_array(),
                            basis.to_array(),
                            scale,
                            actual.to_array(),
                            caps,
                            radius
                        );
                    }
                }
                assert!(clip.joint_translation_sample_error_bounds(1, keys).is_err());
                assert!(
                    clip.joint_translation_sample_error_bounds(0, [keys[1], keys[0]])
                        .is_err()
                );
                assert!(
                    clip.joint_translation_sample_error_bounds(0, [0., f32::NAN])
                        .is_err()
                );
                assert!(
                    clip.joint_translation_sample_error_bounds(0, [-1., keys[1]])
                        .is_err()
                );
                assert_eq!(
                    clip.joint_translation_sample_error_bounds(0, [keys[1]; 2])
                        .unwrap(),
                    Some([0.; 3])
                );
                if keys[0] > 0. {
                    assert_eq!(
                        clip.joint_translation_sample_error_bounds(0, [0., keys[0]])
                            .unwrap(),
                        Some([0.; 3])
                    );
                    assert_eq!(
                        clip.joint_translation_sample_error_bounds(0, [0., keys[1]])
                            .unwrap(),
                        None
                    );
                }
            }
        }
    }
    #[test]
    fn uniform_rotation_caps_cover_actual_f32_chain_and_reject_unqualified_error() {
        let rig = |name: &str, rotation| {
            Skeleton::new(vec![Joint {
                name: name.into(),
                parent: None,
                bind_local: Transform {
                    rotation,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            }])
            .unwrap()
        };
        let h = core::f32::consts::FRAC_1_SQRT_2;
        let rotations = [
            Quat::IDENTITY,
            Quat::from_xyzw(0., 0., h, h),
            Quat::from_euler(glam::EulerRot::XYZ, 0.2, -0.4, 0.7),
            Quat::from_xyzw(0., 0., 0., 1_f32.next_up()),
        ];
        for source_q in rotations {
            for target_q in rotations {
                let source = rig("s", source_q);
                let target = rig("t", target_q);
                for correction in rotations {
                    let binding = RetargetBinding::new(
                        &source,
                        &target,
                        &[RetargetJoint {
                            source: "s".into(),
                            target: "t".into(),
                            rotation_basis: correction,
                            translation_basis: Quat::IDENTITY,
                            translation_scale: 1.,
                        }],
                    )
                    .unwrap();
                    for animated in rotations {
                        let ideal_input = animated.as_dquat().normalize();
                        let incoming = core::array::from_fn(|i| {
                            (animated.as_dquat().to_array()[i] - ideal_input.to_array()[i]).abs()
                                + 1e-15
                        });
                        let (joint, caps) = binding
                            .joint_rotation_evaluation_error_bounds(0, incoming)
                            .unwrap();
                        assert_eq!(joint, 0);
                        assert!(caps.iter().all(|v| *v > 0. && *v < 0.001));
                        let mut pose = source.bind_pose();
                        pose.local[0].rotation = animated;
                        let actual = binding.apply_pose(&pose).unwrap().local()[0]
                            .rotation
                            .as_dquat();
                        let c = correction.as_dquat().normalize();
                        let ideal = (target_q.as_dquat().normalize()
                            * c
                            * (source_q.as_dquat().normalize().conjugate() * ideal_input)
                            * c.conjugate())
                        .normalize();
                        for i in 0..4 {
                            assert!((actual.to_array()[i] - ideal.to_array()[i]).abs() <= caps[i]);
                        }
                        println!(
                            "retarget_rotation_error_reference={{\"source_bind\":{:?},\"target_bind\":{:?},\"correction\":{:?},\"animated\":{:?},\"source_error\":{:?},\"actual\":{:?},\"caps\":{:?}}}",
                            source_q.to_array(),
                            target_q.to_array(),
                            correction.to_array(),
                            animated.to_array(),
                            incoming,
                            actual.to_array(),
                            caps
                        );
                    }
                    assert!(
                        binding
                            .joint_rotation_evaluation_error_bounds(1, [0.; 4])
                            .is_err()
                    );
                    for invalid in [[-1.; 4], [f64::NAN; 4], [f64::INFINITY; 4], [1.; 4]] {
                        assert!(
                            binding
                                .joint_rotation_evaluation_error_bounds(0, invalid)
                                .is_err()
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn uniform_translation_rounding_caps_exclude_motion_width_and_include_source_error() {
        let rig = |name: &str, translation| {
            Skeleton::new(vec![Joint {
                name: name.into(),
                parent: None,
                bind_local: Transform {
                    translation,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            }])
            .unwrap()
        };
        let source = rig("s", Vec3::new(0.6, -0.2, 0.4));
        let target = rig("t", Vec3::new(-1., 2., 3.));
        let h = core::f32::consts::FRAC_1_SQRT_2;
        let arbitrary = Quat::from_euler(glam::EulerRot::XYZ, 0.2, -0.4, 0.7);
        let bases = [
            Quat::IDENTITY,
            Quat::from_xyzw(0., 0., h, h),
            Quat::from_rotation_y(0.3),
            arbitrary,
            -arbitrary,
            Quat::from_xyzw(0., 0., 0., 1_f32.next_up()),
        ];
        let domain = [[-10., 10.]; 3];
        for basis in bases {
            for scale in [0., 2., 1e6] {
                let binding = RetargetBinding::new(
                    &source,
                    &target,
                    &[RetargetJoint {
                        source: "s".into(),
                        target: "t".into(),
                        rotation_basis: Quat::IDENTITY,
                        translation_basis: basis,
                        translation_scale: scale,
                    }],
                )
                .unwrap();
                for incoming in [[0.; 3], [0.03125, 0.0625, 0.125]] {
                    let (joint, (axes, radius)) = binding
                        .joint_translation_evaluation_error_bounds(0, domain, incoming)
                        .unwrap();
                    assert_eq!(joint, 0);
                    assert!(axes.iter().all(|v| v.is_finite() && *v >= 0.) && radius.is_finite());
                    if scale == 0. {
                        assert_eq!((axes, radius), ([0.; 3], 0.));
                    }
                    if incoming == [0.; 3] {
                        assert!(radius < if scale <= 2. { 1e-4 } else { 100. });
                    }
                    for position in [Vec3::new(-8., -4., 2.), Vec3::ZERO, Vec3::new(8., 9., -9.)] {
                        let mut pose = source.bind_pose();
                        pose.local[0].translation =
                            position + glam::DVec3::from_array(incoming).as_vec3();
                        let actual = binding.apply_pose(&pose).unwrap().local()[0].translation;
                        println!(
                            "retarget_translation_error_reference={{\"basis\":{:?},\"scale\":{scale:?},\"source_bind\":{:?},\"target_bind\":{:?},\"source_input\":{:?},\"source_error\":{incoming:?},\"evaluated\":{:?},\"axes\":{axes:?},\"radius\":{radius:?}}}",
                            basis.to_array(),
                            source.joints()[0].bind_local.translation.to_array(),
                            target.joints()[0].bind_local.translation.to_array(),
                            position.to_array(),
                            actual.to_array()
                        );
                    }
                }
                assert!(
                    binding
                        .joint_translation_evaluation_error_bounds(1, domain, [0.; 3])
                        .is_err()
                );
                for bad in [
                    [[1., -1.]; 3],
                    [[f64::NAN, 1.]; 3],
                    [[0., f64::INFINITY]; 3],
                ] {
                    assert!(
                        binding
                            .joint_translation_evaluation_error_bounds(0, bad, [0.; 3])
                            .is_err()
                    );
                }
                assert!(
                    binding
                        .joint_translation_evaluation_error_bounds(0, domain, [-1., 0., 0.])
                        .is_err()
                );
                assert!(
                    binding
                        .joint_translation_evaluation_error_bounds(0, domain, [f64::NAN, 0., 0.])
                        .is_err()
                );
                if scale == 1e6 {
                    assert!(
                        binding
                            .joint_translation_evaluation_error_bounds(
                                0,
                                [[f64::from(f32::MAX); 2]; 3],
                                [0.; 3]
                            )
                            .is_err()
                    );
                }
            }
        }
    }
    #[test]
    fn original_clip_interval_retains_nonbind_origin_and_initial_rotation() {
        let source = Skeleton::new(vec![Joint {
            name: "s".into(),
            parent: None,
            bind_local: Transform {
                translation: Vec3::new(0.6, -0.2, 0.4),
                rotation: Quat::from_xyzw(1., 0., 0., 0.),
                ..Transform::IDENTITY
            },
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap();
        let target = Skeleton::new(vec![Joint {
            name: "t".into(),
            parent: None,
            bind_local: Transform {
                translation: Vec3::new(-1., 2., 3.),
                rotation: Quat::from_rotation_z(-0.4),
                ..Transform::IDENTITY
            },
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap();
        let initial_position = Vec3::new(4., -2., 1.);
        let delta = Vec3::new(-0.5, 0.25, 0.75);
        let initial_rotation = Quat::from_xyzw(0.5, 0.5, 0.5, 0.5);
        let h = core::f32::consts::FRAC_1_SQRT_2;
        let clip = AnimationClip::new(
            "original",
            1.,
            Playback::Clamp,
            vec![JointTrack {
                translations: vec![
                    Vec3Key {
                        time: 0.,
                        value: initial_position,
                    },
                    Vec3Key {
                        time: 1.,
                        value: initial_position + delta,
                    },
                ],
                rotations: vec![
                    QuatKey {
                        time: 0.,
                        value: initial_rotation,
                    },
                    QuatKey {
                        time: 1.,
                        value: Quat::from_xyzw(h, h, 0., 0.),
                    },
                ],
                ..Default::default()
            }],
            &source,
        )
        .unwrap();
        let correction = Quat::from_euler(glam::EulerRot::XYZ, 0.3, -0.1, 0.7);
        let basis = Quat::from_rotation_y(0.3);
        let mut saw_actual_outside_ideal_field = false;
        for scale in [0., 2., 1e6] {
            let binding = RetargetBinding::new(
                &source,
                &target,
                &[RetargetJoint {
                    source: "s".into(),
                    target: "t".into(),
                    rotation_basis: correction,
                    translation_basis: basis,
                    translation_scale: scale,
                }],
            )
            .unwrap();
            for times in [[0.125, 0.25], [0.5, 0.625]] {
                let (joint, field) = binding
                    .joint_rigid_clip_interval_enclosure(&clip, 0, times)
                    .unwrap()
                    .unwrap();
                assert_eq!(joint, 0);
                for point in [
                    Vec3::ZERO,
                    Vec3::X,
                    Vec3::new(0.25, -0.5, 0.75),
                    Vec3::new(-4., 5., -6.),
                ] {
                    let image = field
                        .transform_point_box_bounds(point.as_dvec3().to_array().map(|v| [v, v]))
                        .unwrap();
                    for fraction in [0., 0.25, 0.5, 0.75, 1.] {
                        let phase = times[0] + fraction * (times[1] - times[0]);
                        let source_pose = clip.try_sample(&source, phase as f32).unwrap();
                        let pose = binding.apply_pose(&source_pose).unwrap();
                        let actual = pose.local()[0].matrix().transform_point3(point);
                        if image.iter().zip(actual.to_array()).any(|(bounds, value)| {
                            f64::from(value) < bounds[0] || f64::from(value) > bounds[1]
                        }) {
                            saw_actual_outside_ideal_field = true;
                            let stored = binding
                                .joint_rigid_pose_enclosure(&source_pose, 0)
                                .unwrap()
                                .1;
                            let (_, error) = stored
                                .enclosed_point_evaluation_error(
                                    point.as_dvec3().to_array().map(|v| [v, v]),
                                    1.,
                                    actual.as_dvec3(),
                                )
                                .unwrap();
                            assert!(error > 0. && error.is_finite());
                        }
                    }
                    println!(
                        "retarget_absolute_interval_reference={{\"target_rotation\":{:?},\"correction\":{:?},\"basis\":{:?},\"target_translation\":{:?},\"source_bind_translation\":{:?},\"source_initial_translation\":{:?},\"source_bind_rotation\":{:?},\"source_initial_rotation\":{:?},\"end_translation\":{:?},\"times\":{times:?},\"scale\":{scale:?},\"point\":{:?},\"image\":{image:?}}}",
                        target.joints()[0].bind_local.rotation.to_array(),
                        correction.to_array(),
                        basis.to_array(),
                        target.joints()[0].bind_local.translation.to_array(),
                        source.joints()[0].bind_local.translation.to_array(),
                        initial_position.to_array(),
                        source.joints()[0].bind_local.rotation.to_array(),
                        initial_rotation.to_array(),
                        delta.to_array(),
                        point.to_array()
                    );
                }
            }
            let foreign = AnimationClip::new(
                "foreign",
                1.,
                Playback::Clamp,
                vec![JointTrack::default()],
                &target,
            )
            .unwrap();
            assert!(
                binding
                    .joint_rigid_clip_interval_enclosure(&foreign, 0, [0.125, 0.25])
                    .is_err()
            );
            assert!(
                binding
                    .joint_rigid_clip_interval_enclosure(&clip, 1, [0.125, 0.25])
                    .is_err()
            );
            assert!(
                binding
                    .joint_rigid_clip_interval_enclosure(&clip, 0, [0.25, 0.125])
                    .is_err()
            );
            let held = AnimationClip::new(
                "bind",
                1.,
                Playback::Clamp,
                vec![JointTrack::default()],
                &source,
            )
            .unwrap();
            let (_, field) = binding
                .joint_rigid_clip_interval_enclosure(&held, 0, [0.125, 0.25])
                .unwrap()
                .unwrap();
            let expected = binding
                .joint_rigid_pose_enclosure(&source.bind_pose(), 0)
                .unwrap()
                .1;
            for (bounds, pose_bounds) in field
                .translation_bounds()
                .into_iter()
                .zip(expected.translation_bounds())
            {
                assert!(bounds[0] <= pose_bounds[1] && pose_bounds[0] <= bounds[1]);
            }
        }
        assert!(saw_actual_outside_ideal_field);
    }
    #[test]
    fn continuous_source_interval_transports_independent_retarget_channels() {
        let source = Skeleton::new(vec![Joint {
            name: "s".into(),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap();
        let target = Skeleton::new(vec![Joint {
            name: "t".into(),
            parent: None,
            bind_local: Transform {
                translation: Vec3::new(-1., 2., 3.),
                rotation: Quat::from_rotation_z(-0.4),
                ..Transform::IDENTITY
            },
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap();
        let h = core::f32::consts::FRAC_1_SQRT_2;
        let end_position = Vec3::new(-0.5, 0.25, 0.75);
        let clip = AnimationClip::new(
            "source",
            1.,
            Playback::Clamp,
            vec![JointTrack {
                translations: vec![
                    Vec3Key {
                        time: 0.,
                        value: Vec3::ZERO,
                    },
                    Vec3Key {
                        time: 1.,
                        value: end_position,
                    },
                ],
                rotations: vec![
                    QuatKey {
                        time: 0.,
                        value: Quat::IDENTITY,
                    },
                    QuatKey {
                        time: 1.,
                        value: Quat::from_xyzw(0., h, 0., h),
                    },
                ],
                ..Default::default()
            }],
            &source,
        )
        .unwrap();
        let curve = clip.root_rigid_curve(0).unwrap();
        let correction = Quat::from_euler(glam::EulerRot::XYZ, 0.3, -0.1, 0.7);
        let basis = Quat::from_rotation_y(0.3);
        for scale in [0., 2., 1e6] {
            let binding = RetargetBinding::new(
                &source,
                &target,
                &[RetargetJoint {
                    source: "s".into(),
                    target: "t".into(),
                    rotation_basis: correction,
                    translation_basis: basis,
                    translation_scale: scale,
                }],
            )
            .unwrap();
            for times in [[0.125, 0.25], [0.5, 0.625]] {
                // Here, and only because bind/initial translation are zero and
                // initial rotation is identity, this root factor is the actual
                // absolute local source pose throughout the key interval.
                let original = curve
                    .source_phase_interval_enclosure(times, [true; 3])
                    .unwrap()
                    .unwrap();
                let (_, field) = binding.joint_rigid_field_enclosure(0, original).unwrap();
                for point in [
                    Vec3::ZERO,
                    Vec3::X,
                    Vec3::new(0.25, -0.5, 0.75),
                    Vec3::new(-4., 5., -6.),
                ] {
                    let image = field
                        .transform_point_box_bounds(point.as_dvec3().to_array().map(|v| [v, v]))
                        .unwrap();
                    for fraction in [0., 0.25, 0.5, 0.75, 1.] {
                        let phase = times[0] + fraction * (times[1] - times[0]);
                        let actual = binding
                            .apply_pose(&clip.try_sample(&source, phase as f32).unwrap())
                            .unwrap();
                        let evaluated = actual.local()[0].matrix().transform_point3(point);
                        for (range, value) in image.iter().zip(evaluated.to_array()) {
                            assert!(range[0] <= f64::from(value) && f64::from(value) <= range[1]);
                        }
                    }
                    println!(
                        "retarget_interval_reference={{\"target_rotation\":{:?},\"correction\":{:?},\"basis\":{:?},\"target_translation\":{:?},\"end_translation\":{:?},\"times\":{times:?},\"scale\":{scale:?},\"point\":{:?},\"image\":{image:?}}}",
                        target.joints()[0].bind_local.rotation.to_array(),
                        correction.to_array(),
                        basis.to_array(),
                        target.joints()[0].bind_local.translation.to_array(),
                        end_position.to_array(),
                        point.to_array()
                    );
                }
            }
        }
    }
    #[test]
    fn joint_pose_certificate_covers_actual_f32_retarget_and_matrix_evaluation() {
        let rig = |name: &str, translation, rotation| {
            Skeleton::new(vec![Joint {
                name: name.into(),
                parent: None,
                bind_local: Transform {
                    translation,
                    rotation,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            }])
            .unwrap()
        };
        let source = rig("s", Vec3::new(0.6, -0.2, 0.4), Quat::from_rotation_x(0.2));
        let target = rig("t", Vec3::new(-1., 2., 3.), Quat::from_rotation_z(-0.4));
        let correction = Quat::from_euler(glam::EulerRot::XYZ, 0.3, -0.1, 0.7);
        let h = core::f32::consts::FRAC_1_SQRT_2;
        for basis in [Quat::from_xyzw(0., 0., h, h), Quat::from_rotation_y(0.3)] {
            for scale in [0., 2., 1e6] {
                let binding = RetargetBinding::new(
                    &source,
                    &target,
                    &[RetargetJoint {
                        source: "s".into(),
                        target: "t".into(),
                        rotation_basis: correction,
                        translation_basis: basis,
                        translation_scale: scale,
                    }],
                )
                .unwrap();
                // This pose certificate also supports independent channel bases.
                assert!(binding.root_similarity_enclosure(0, [true; 3]).is_err());
                for phase in [0., 0.125, 0.5, 1.] {
                    let mut pose = source.bind_pose();
                    pose.local[0].translation += Vec3::new(phase, phase * 0.3, -phase * 0.7);
                    pose.local[0].rotation =
                        (pose.local[0].rotation * Quat::from_rotation_y(phase * 0.8)).normalize();
                    let actual = binding.apply_pose(&pose).unwrap();
                    let (joint, ideal) = binding.joint_rigid_pose_enclosure(&pose, 0).unwrap();
                    assert_eq!(joint, 0);
                    for point in [
                        Vec3::ZERO,
                        Vec3::X,
                        Vec3::new(0.25, -0.5, 0.75),
                        Vec3::new(-4., 5., -6.),
                    ] {
                        let point_box = point.as_dvec3().to_array().map(|v| [v, v]);
                        let image = ideal.transform_point_box_bounds(point_box).unwrap();
                        let evaluated = actual.local()[0].matrix().transform_point3(point);
                        let (axes, radius) = ideal
                            .enclosed_point_evaluation_error(point_box, 1., evaluated.as_dvec3())
                            .unwrap();
                        assert!(
                            axes.iter().all(|v| v.is_finite() && *v >= 0.) && radius.is_finite()
                        );
                        println!(
                            "retarget_pose_reference={{\"source_rotation\":{:?},\"target_rotation\":{:?},\"correction\":{:?},\"basis\":{:?},\"animated_rotation\":{:?},\"source_translation\":{:?},\"target_translation\":{:?},\"animated_translation\":{:?},\"scale\":{scale:?},\"point\":{:?},\"evaluated\":{:?},\"image\":{image:?},\"axes\":{axes:?},\"radius\":{radius:?}}}",
                            source.joints()[0].bind_local.rotation.to_array(),
                            target.joints()[0].bind_local.rotation.to_array(),
                            correction.to_array(),
                            basis.to_array(),
                            pose.local()[0].rotation.to_array(),
                            source.joints()[0].bind_local.translation.to_array(),
                            target.joints()[0].bind_local.translation.to_array(),
                            pose.local()[0].translation.to_array(),
                            point.to_array(),
                            evaluated.to_array()
                        );
                    }
                }
                assert!(
                    binding
                        .joint_rigid_pose_enclosure(&target.bind_pose(), 0)
                        .is_err()
                );
                assert!(
                    binding
                        .joint_rigid_pose_enclosure(&source.bind_pose(), 1)
                        .is_err()
                );
                let mut invalid = source.bind_pose();
                invalid.local[0].rotation = Quat::from_xyzw(f32::NAN, 0., 0., 1.);
                assert!(binding.joint_rigid_pose_enclosure(&invalid, 0).is_err());
            }
        }
    }
    #[test]
    fn root_similarity_composes_signed_parent_frames_without_matrix_rounding() {
        use crate::{RootRigidEnclosure, RootRigidTransform, RootUniformScaleEnclosure};
        let rig = |name: &str, translation| {
            Skeleton::new(vec![Joint {
                name: name.into(),
                parent: None,
                bind_local: Transform {
                    translation,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            }])
            .unwrap()
        };
        let source = rig("s", Vec3::new(1., 2., 3.));
        let target = rig("t", Vec3::new(4., 5., 6.));
        let cycle = Quat::from_xyzw(0.5, 0.5, 0.5, 0.5);
        let binding = RetargetBinding::new(
            &source,
            &target,
            &[RetargetJoint {
                source: "s".into(),
                target: "t".into(),
                rotation_basis: cycle,
                translation_basis: cycle,
                translation_scale: 2.,
            }],
        )
        .unwrap();
        let root = binding.root_similarity_enclosure(0, [true; 3]).unwrap();
        let parent = RootRigidEnclosure::from_transform(RootRigidTransform {
            translation: glam::DVec3::new(7., 8., 9.),
            rotation: cycle.as_dquat(),
        })
        .unwrap();
        let ancestor = RootRigidEnclosure::from_transform(RootRigidTransform {
            translation: glam::DVec3::new(10., 11., 12.),
            rotation: glam::DQuat::from_xyzw(0., 0., 1., 0.),
        })
        .unwrap();
        for parent_scale in [0.5, -3.] {
            let combined = root
                .in_parent(
                    parent,
                    RootUniformScaleEnclosure::from_scale(parent_scale).unwrap(),
                )
                .unwrap()
                .in_parent(
                    ancestor,
                    RootUniformScaleEnclosure::from_scale(-2.).unwrap(),
                )
                .unwrap();
            assert!(combined.scale.bounds()[0] <= -4. * parent_scale);
            assert!(combined.scale.bounds()[1] >= -4. * parent_scale);
            for point in [
                glam::DVec3::ZERO,
                glam::DVec3::new(1., 2., 3.),
                glam::DVec3::new(-4., 5., -6.),
            ] {
                // Independently authored exact signed coordinate permutations.
                let delta = point - glam::DVec3::new(1., 2., 3.);
                let target =
                    glam::DVec3::new(4., 5., 6.) + 2. * glam::DVec3::new(delta.z, delta.x, delta.y);
                let parent = glam::DVec3::new(7., 8., 9.)
                    + parent_scale * glam::DVec3::new(target.z, target.x, target.y);
                let expected = glam::DVec3::new(10., 11., 12.)
                    - 2. * glam::DVec3::new(-parent.x, -parent.y, parent.z);
                let bounds = combined
                    .frame
                    .similarity_point_box_bounds_enclosed(
                        point.to_array().map(|v| [v, v]),
                        combined.scale,
                    )
                    .unwrap();
                for (range, value) in bounds.iter().zip(expected.to_array()) {
                    assert!(range[0] <= value && value <= range[1]);
                }
            }
        }
        assert!(
            root.in_parent(parent, RootUniformScaleEnclosure::from_scale(0.).unwrap())
                .is_err()
        );
    }
    #[test]
    fn root_similarity_encloses_original_bind_offsets_and_unit_conversion() {
        let make_rig = |name: &str, translation| {
            Skeleton::new(vec![Joint {
                name: name.into(),
                parent: None,
                bind_local: Transform {
                    translation,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            }])
            .unwrap()
        };
        let source_position = Vec3::new(0.6, -0.2, 0.4);
        let target_position = Vec3::new(-1., 2., 3.);
        let source = make_rig("s", source_position);
        let target = make_rig("t", target_position);
        let h = core::f32::consts::FRAC_1_SQRT_2;
        let bases = [
            Quat::IDENTITY,
            Quat::from_xyzw(0., 0., h, h),
            Quat::from_xyzw(0.5, 0.5, 0.5, 0.5),
            Quat::from_rotation_y(0.3),
            Quat::from_euler(glam::EulerRot::XYZ, 0.2, -0.4, 0.7),
        ];
        for basis in bases {
            for scale in [0.25_f32, 2., 1e6] {
                let binding = RetargetBinding::new(
                    &source,
                    &target,
                    &[RetargetJoint {
                        source: "s".into(),
                        target: "t".into(),
                        rotation_basis: basis,
                        translation_basis: basis,
                        translation_scale: scale,
                    }],
                )
                .unwrap();
                let similarity = binding.root_similarity_enclosure(0, [true; 3]).unwrap();
                assert_eq!(similarity.target_joint, 0);
                assert_eq!(similarity.source_axes, [true; 3]);
                assert_eq!(similarity.scale.bounds(), [f64::from(scale); 2]);
                for point in [
                    source_position.as_dvec3(),
                    glam::DVec3::ZERO,
                    glam::DVec3::new(0.1, -0.8, 1.3),
                    glam::DVec3::new(-4., 5., -6.),
                ] {
                    let bounds = similarity
                        .frame
                        .similarity_point_box_bounds_enclosed(
                            point.to_array().map(|value| [value, value]),
                            similarity.scale,
                        )
                        .unwrap();
                    if point == source_position.as_dvec3() {
                        for (bound, expected) in bounds.iter().zip(target_position.to_array()) {
                            assert!(
                                bound[0] <= f64::from(expected) && f64::from(expected) <= bound[1]
                            );
                        }
                    }
                    println!(
                        "retarget_similarity_reference={{\"basis\":{:?},\"scale\":{scale:?},\"source\":{:?},\"target\":{:?},\"point\":{:?},\"bounds\":{bounds:?}}}",
                        basis.to_array(),
                        source_position.to_array(),
                        target_position.to_array(),
                        point.to_array()
                    );
                }
            }
        }
        let zero = RetargetBinding::new(
            &source,
            &target,
            &[RetargetJoint {
                source: "s".into(),
                target: "t".into(),
                rotation_basis: Quat::IDENTITY,
                translation_basis: Quat::IDENTITY,
                translation_scale: 0.,
            }],
        )
        .unwrap();
        zero.apply_pose(&source.bind_pose()).unwrap();
        assert!(zero.root_similarity_enclosure(0, [true; 3]).is_err());
    }
    #[test]
    fn quarter_turn_masks_use_original_components_and_reject_rounded_coherence() {
        let rig = |name: &str, rotation| {
            Skeleton::new(vec![Joint {
                name: name.into(),
                parent: None,
                bind_local: Transform {
                    rotation,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            }])
            .unwrap()
        };
        let binding = |source: &Skeleton, target: &Skeleton, rotation_basis, translation_basis| {
            RetargetBinding::new(
                source,
                target,
                &[RetargetJoint {
                    source: "s".into(),
                    target: "t".into(),
                    rotation_basis,
                    translation_basis,
                    translation_scale: 1.,
                }],
            )
            .unwrap()
        };
        let source = rig("s", Quat::IDENTITY);
        let target = rig("t", Quat::IDENTITY);
        let h = core::f32::consts::FRAC_1_SQRT_2;
        let q = Quat::from_xyzw(0., 0., h, h);
        for basis in [q, -q] {
            let mapping = binding(&source, &target, q, basis);
            for mask in 0..8 {
                let target_axes = std::array::from_fn(|axis| mask & (1 << axis) != 0);
                assert_eq!(
                    mapping.source_root_motion_axes(0, target_axes).unwrap(),
                    [target_axes[1], target_axes[0], target_axes[2]]
                );
            }
        }
        let tiny = f32::from_bits(1);
        let source = rig("s", Quat::from_xyzw(tiny, 0., 0., 1.));
        let target = rig("t", Quat::from_xyzw(tiny, tiny, 0., 1.));
        let rounded = (target.joints()[0].bind_local.rotation
            * source.joints()[0].bind_local.rotation.conjugate())
        .normalize();
        let mapping = binding(&source, &target, Quat::IDENTITY, rounded);
        // Pose-only mapping remains valid, but a common rigid trajectory cannot
        // certify the two different exact bases as one similarity transform.
        mapping.apply_pose(&source.bind_pose()).unwrap();
        assert!(mapping.source_root_motion_axes(0, [true; 3]).is_err());
    }
    #[test]
    fn partial_masks_route_between_axes_and_match_independently_authored_target_curves() {
        let rig = |name: &str, translation: Vec3| {
            Skeleton::new(vec![Joint {
                name: name.into(),
                parent: None,
                bind_local: Transform {
                    translation,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            }])
            .unwrap()
        };
        let source_bind = Vec3::new(0.6, 0.2, 0.4);
        let target_bind = Vec3::new(1., 2., 3.);
        let source = rig("s", source_bind);
        let target = rig("t", target_bind);
        let basis = Quat::from_xyzw(0.5, 0.5, 0.5, 0.5);
        let make_binding = |basis| {
            RetargetBinding::new(
                &source,
                &target,
                &[RetargetJoint {
                    source: "s".into(),
                    target: "t".into(),
                    rotation_basis: basis,
                    translation_basis: basis,
                    translation_scale: 2.,
                }],
            )
            .unwrap()
        };
        let binding = make_binding(basis);
        assert_eq!(
            binding
                .source_root_motion_axes(0, [true, false, false])
                .unwrap(),
            [false, false, true]
        );
        let positions = [
            source_bind + Vec3::new(0.1, 0.2, 0.3),
            source_bind + Vec3::new(0.5, 0.5, 0.5),
        ];
        let rotations = [Quat::IDENTITY, Quat::from_rotation_y(0.8)];
        let make_clip = |rig: &Skeleton, positions: [Vec3; 2], rotations: [Quat; 2]| {
            AnimationClip::new(
                "turn",
                1.,
                Playback::Loop,
                vec![JointTrack {
                    translations: positions
                        .into_iter()
                        .enumerate()
                        .map(|(i, value)| Vec3Key {
                            time: i as f32,
                            value,
                        })
                        .collect(),
                    rotations: rotations
                        .into_iter()
                        .enumerate()
                        .map(|(i, value)| QuatKey {
                            time: i as f32,
                            value,
                        })
                        .collect(),
                    ..Default::default()
                }],
                rig,
            )
            .unwrap()
        };
        let source_clip = make_clip(&source, positions, rotations);
        let target_clip = make_clip(
            &target,
            positions.map(|p| target_bind + 2. * (basis * (p - source_bind))),
            rotations.map(|q| (basis * q * basis.conjugate()).normalize()),
        );
        for mask in 0..8 {
            let target_axes = std::array::from_fn(|i| mask & (1 << i) != 0);
            let source_axes = binding.source_root_motion_axes(0, target_axes).unwrap();
            let path = source_clip
                .root_rigid_curve(0)
                .unwrap()
                .path(0.13, 1.2, source_axes, 256)
                .unwrap();
            let transported = binding.apply_root_path(&path, 0, target_axes).unwrap();
            let reference = target_clip
                .root_rigid_curve(0)
                .unwrap()
                .path(0.13, 1.2, target_axes, 256)
                .unwrap();
            assert_eq!(transported.spans().len(), reference.spans().len());
            for (a, b) in transported.spans().iter().zip(reference.spans()) {
                for i in 0..=100 {
                    let a = a.sample(i as f64 / 100.).unwrap();
                    let b = b.sample(i as f64 / 100.).unwrap();
                    assert!(
                        a.translation.abs_diff_eq(b.translation, 1e-6),
                        "{target_axes:?} {a:?} {b:?}"
                    );
                    for axis in [glam::DVec3::X, glam::DVec3::Y, glam::DVec3::Z] {
                        assert!((a.rotation * axis).abs_diff_eq(b.rotation * axis, 1e-6));
                    }
                }
            }
        }
        let planar = make_binding(Quat::from_rotation_y(0.3));
        assert_eq!(
            planar
                .source_root_motion_axes(0, [true, false, true])
                .unwrap(),
            [true, false, true]
        );
        assert!(
            planar
                .source_root_motion_axes(0, [true, false, false])
                .is_err()
        );
        assert!(binding.source_root_motion_axes(1, [true; 3]).is_err());
    }
}
