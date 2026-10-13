//! Model-space rotation transport includes unmapped source ancestors, as in
//! Godot RetargetModifier3D's global-pose path. Scene/resource ownership remains
//! with the canonical binding. Translation/scale keep its explicit local policy.
use super::*;
use glam::DQuat;

// Compose authored proper rotation frames, retaining signed-scale handedness.
// Magnitudes remain in the transported scale channels; no matrix is decomposed
// and no near-unit scale is silently rounded. This is not a polar/shear frame.
fn parent_rotation(pose: &Pose, joint: usize) -> Result<DQuat, AnimationError> {
    let mut result = DQuat::IDENTITY;
    let mut parent = pose.rig[joint].parent;
    while let Some(index) = parent {
        let local = pose.local[usize::from(index)];
        result =
            (local.rotation.as_dquat().normalize() * crate::ik::sign_basis(local.scale) * result)
                .normalize();
        parent = pose.rig[usize::from(index)].parent;
    }
    if !result.is_finite() {
        return Err(AnimationError::NumericalOverflow);
    }
    Ok(result)
}
fn model_rotation(pose: &Pose, joint: usize) -> Result<DQuat, AnimationError> {
    Ok((parent_rotation(pose, joint)?
        * pose.local[joint].rotation.as_dquat().normalize()
        * crate::ik::sign_basis(pose.local[joint].scale))
    .normalize())
}

/// Compiled model-space rotation transport for different bone hierarchies.
/// This is pose-only: no root-motion certificate from the local-channel binding
/// is exposed, because omitted animated ancestors can change the root motion.
#[derive(Clone, Debug)]
pub struct ModelRotationRetarget {
    binding: RetargetBinding,
    source_rest: Vec<DQuat>,
    target_reference: Vec<DQuat>,
    model_basis: DQuat,
    target_order: Vec<usize>,
}
impl RetargetBinding {
    /// Compiles model-space rotations using an explicitly authored conversion
    /// between source and target model axes. Source bind motion maps to the
    /// compiled target reference pose. Frames compose authored quaternions and
    /// scale handedness; scale magnitudes are retained, never rounded to uniform.
    /// This does not extract polar rotations from sheared model matrices.
    /// Per-joint `rotation_basis` belongs to local-channel transport; this mode
    /// uses `model_basis` and compiled source/target model reference frames.
    /// # Errors
    /// Rejects invalid axis conversion and palettes.
    pub fn compile_model_rotations(
        self,
        model_basis: Quat,
    ) -> Result<ModelRotationRetarget, AnimationError> {
        if !model_basis.is_finite() || !model_basis.is_normalized() {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        let source = Pose {
            rig: self.source.clone(),
            local: self.source.iter().map(|joint| joint.bind_local).collect(),
        };
        source.skin_matrices(&Skeleton {
            joints: self.source.clone(),
        })?;
        // Use exactly the canonical unmapped channels published by transport.
        let target = self.transport_pose(&source)?;
        target.skin_matrices(&Skeleton {
            joints: self.target.clone(),
        })?;
        let mut source_rest = Vec::with_capacity(self.joints.len());
        let mut target_reference = Vec::with_capacity(self.joints.len());
        for joint in &self.joints {
            source_rest.push(model_rotation(&source, joint.source)?);
            target_reference.push(model_rotation(&target, joint.target)?);
        }
        let mut target_order: Vec<_> = (0..self.joints.len()).collect();
        target_order.sort_by_key(|&index| self.joints[index].target);
        Ok(ModelRotationRetarget {
            binding: self,
            source_rest,
            target_reference,
            model_basis: model_basis.as_dquat().normalize(),
            target_order,
        })
    }
}
impl ModelRotationRetarget {
    /// Converts a source bone-local reference point to its mapped target bone.
    /// This authoring operation uses full reference affine frames and an explicit
    /// positive model-space size ratio, not translation-channel or rotation deltas.
    /// It does not infer a sole surface or preserve animated end-effector positions.
    /// # Errors
    /// Rejects unmapped joints, nonfinite points, invalid sizing, singular target
    /// reference frames and coordinate overflow. The binding is never mutated.
    pub fn map_reference_local_point(
        &self,
        source_joint: u16,
        point: glam::DVec3,
        size_ratio: f64,
    ) -> Result<(u16, glam::DVec3), AnimationError> {
        if !point.is_finite()
            || !size_ratio.is_finite()
            || !(0. ..=1e6).contains(&size_ratio)
            || size_ratio == 0.
        {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        let joint = self
            .binding
            .joints
            .iter()
            .find(|joint| joint.source == usize::from(source_joint))
            .ok_or(AnimationError::InvalidRetargetBinding)?;
        let source = Pose {
            rig: self.binding.source.clone(),
            local: self
                .binding
                .source
                .iter()
                .map(|joint| joint.bind_local)
                .collect(),
        };
        let target = self.binding.transport_pose(&source)?;
        let global = |pose: &Pose, index: usize| {
            let mut matrix = glam::DMat4::IDENTITY;
            let mut current = Some(index);
            while let Some(i) = current {
                let local = pose.local[i];
                matrix = glam::DMat4::from_scale_rotation_translation(
                    local.scale.as_dvec3(),
                    local.rotation.as_dquat().normalize(),
                    local.translation.as_dvec3(),
                ) * matrix;
                current = pose.rig[i].parent.map(usize::from);
            }
            matrix
        };
        let source_frame = global(&source, joint.source);
        let inverse_target = global(&target, joint.target).inverse();
        if !source_frame.is_finite() || !inverse_target.is_finite() {
            return Err(AnimationError::InvalidRetargetBinding);
        }
        let mapped = inverse_target.transform_vector3(
            self.model_basis * source_frame.transform_vector3(point) * size_ratio,
        );
        if !mapped.is_finite() {
            return Err(AnimationError::NumericalOverflow);
        }
        Ok((joint.target as u16, mapped))
    }

    /// Preserves the model-space rotational contribution of every source
    /// ancestor, including bones absent from the target map. Target local
    /// rotations are solved parent-before-child against the already solved pose.
    /// Local translation/scale channels retain the canonical binding's policy;
    /// this does not copy global positions or promise planted end effectors.
    /// # Errors
    /// Rejects foreign/invalid poses and palette overflow.
    /// No input, binding or source/target rig is mutated on failure.
    pub fn apply_pose(&self, source: &Pose) -> Result<Pose, AnimationError> {
        self.binding.validate_source_pose(source)?;
        source.skin_matrices(&Skeleton {
            joints: self.binding.source.clone(),
        })?;
        let mut target = self.binding.transport_pose(source)?;
        for &index in &self.target_order {
            let joint = &self.binding.joints[index];
            let delta = (model_rotation(source, joint.source)?
                * self.source_rest[index].conjugate())
            .normalize();
            let desired = (self.model_basis
                * delta
                * self.model_basis.conjugate()
                * self.target_reference[index])
                .normalize();
            let local = (parent_rotation(&target, joint.target)?.conjugate()
                * desired
                * crate::ik::sign_basis(target.local[joint.target].scale).conjugate())
            .normalize();
            target.set_joint_rotation(
                joint.target,
                Quat::from_array(local.to_array().map(|v| v as f32)).normalize(),
            )?;
        }
        target.skin_matrices(&Skeleton {
            joints: self.binding.target.clone(),
        })?;
        Ok(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Mat4, Vec3};
    fn rig(names: &[&str]) -> Skeleton {
        Skeleton::new(
            names
                .iter()
                .enumerate()
                .map(|(i, name)| Joint {
                    name: Arc::from(*name),
                    parent: if i == 0 { None } else { Some((i - 1) as u16) },
                    bind_local: Transform {
                        translation: if i == 0 { Vec3::ZERO } else { Vec3::Y },
                        ..Transform::IDENTITY
                    },
                    inverse_bind: Mat4::IDENTITY,
                })
                .collect(),
        )
        .unwrap()
    }
    fn mapping() -> Vec<RetargetJoint> {
        // Intentionally child-first: compilation must establish target order.
        [("arm", "target.arm"), ("root", "target.root")]
            .map(|(source, target)| RetargetJoint {
                source: source.into(),
                target: target.into(),
                rotation_basis: Quat::IDENTITY,
                translation_basis: Quat::IDENTITY,
                translation_scale: 0.,
            })
            .to_vec()
    }
    #[test]
    fn omitted_animated_parent_survives_model_transport_and_calibration() {
        let source = rig(&["root", "shoulder", "arm"]);
        let target = rig(&["target.root", "target.arm"]);
        let mut reference = target.bind_pose();
        reference
            .set_joint_rotation(1, Quat::from_rotation_z(0.7))
            .unwrap();
        let local = RetargetBinding::new(&source, &target, &mapping())
            .unwrap()
            .with_target_reference_pose(&reference)
            .unwrap();
        let basis = Quat::from_rotation_y(0.5);
        let model = local.clone().compile_model_rotations(basis).unwrap();
        let mut pose = source.bind_pose();
        pose.set_joint_rotation(0, Quat::from_rotation_y(0.2))
            .unwrap();
        pose.set_joint_rotation(1, Quat::from_rotation_z(0.4))
            .unwrap();
        pose.set_joint_rotation(2, Quat::from_rotation_x(0.3))
            .unwrap();
        let original = pose.clone();
        let result = model.apply_pose(&pose).unwrap();
        let b = basis.as_dquat().normalize();
        let expected = b
            * model_rotation(&pose, 2).unwrap()
            * b.conjugate()
            * model_rotation(&reference, 1).unwrap();
        assert!(model_rotation(&result, 1).unwrap().dot(expected).abs() > 1. - 1e-12);
        assert!(
            model_rotation(&local.apply_pose(&pose).unwrap(), 1)
                .unwrap()
                .dot(expected)
                .abs()
                < 0.999
        );
        assert_eq!(pose, original);
        assert_eq!(
            result.local()[1].translation,
            target.bind_pose().local()[1].translation
        );
        result.skin_matrices(&target).unwrap();
        assert_eq!(model.apply_pose(&source.bind_pose()).unwrap(), reference);
        assert!(model.apply_pose(&target.bind_pose()).is_err());
    }
    #[test]
    fn signed_nonuniform_scales_preserve_authored_frames_without_rounding() {
        let mut source = rig(&["root", "shoulder", "arm"]);
        let mut joints = source.joints().to_vec();
        joints[0].bind_local.scale = Vec3::new(-2., 2., 2.);
        source = Skeleton::new(joints).unwrap();
        let target = rig(&["target.root", "target.arm"]);
        let model = RetargetBinding::new(&source, &target, &mapping())
            .unwrap()
            .compile_model_rotations(Quat::IDENTITY)
            .unwrap();
        let mut pose = source.bind_pose();
        pose.set_joint_rotation(1, Quat::from_rotation_z(0.4))
            .unwrap();
        let result = model.apply_pose(&pose).unwrap();
        let expected = model_rotation(&pose, 2).unwrap()
            * model_rotation(&source.bind_pose(), 2).unwrap().conjugate();
        assert!(model_rotation(&result, 1).unwrap().dot(expected).abs() > 1. - 1e-12);
        let before = pose.clone();
        pose.local[1].scale = Vec3::new(1., 2., 1.);
        let scaled = model.apply_pose(&pose).unwrap();
        assert!(
            model_rotation(&scaled, 1)
                .unwrap()
                .dot(model_rotation(&result, 1).unwrap())
                .abs()
                > 1. - 1e-12
        );
        assert_eq!(pose.local[1].scale, Vec3::new(1., 2., 1.));
        assert_eq!(model.apply_pose(&before).unwrap(), result);
        assert!(
            RetargetBinding::new(&source, &target, &mapping())
                .unwrap()
                .compile_model_rotations(Quat::from_xyzw(0., 0., 0., 2.))
                .is_err()
        );
    }
}

#[cfg(test)]
mod reference_point_tests {
    use super::*;
    use glam::{DVec3, Mat4, Vec3};
    fn rig(rotation: Quat, scale: Vec3) -> Skeleton {
        Skeleton::new(vec![Joint {
            name: Arc::from("ankle"),
            parent: None,
            bind_local: Transform {
                translation: Vec3::new(3., 4., 5.),
                rotation,
                scale,
            },
            inverse_bind: Mat4::IDENTITY,
        }])
        .unwrap()
    }
    #[test]
    fn reference_point_mapping_accounts_for_bone_axes_scales_and_explicit_sizing() {
        let source = rig(Quat::from_rotation_z(0.4), Vec3::new(1., 2., 0.5));
        let target = rig(Quat::from_rotation_x(-0.7), Vec3::new(-2., 3., 1.));
        let model_basis = Quat::from_rotation_y(0.6);
        let binding = RetargetBinding::new(
            &source,
            &target,
            &[RetargetJoint {
                source: Arc::from("ankle"),
                target: Arc::from("ankle"),
                rotation_basis: Quat::IDENTITY,
                translation_basis: Quat::IDENTITY,
                translation_scale: 1.,
            }],
        )
        .unwrap()
        .compile_model_rotations(model_basis)
        .unwrap();
        let point = DVec3::new(0., 0.1733, 0.02);
        let (joint, mapped) = binding.map_reference_local_point(0, point, 1.4).unwrap();
        assert_eq!(joint, 0);
        let source_local = source.joints()[0].bind_local;
        let target_local = target.joints()[0].bind_local;
        let expected = model_basis.as_dquat().normalize()
            * source_local.rotation.as_dquat().normalize()
            * (source_local.scale.as_dvec3() * point)
            * 1.4;
        let actual =
            target_local.rotation.as_dquat().normalize() * (target_local.scale.as_dvec3() * mapped);
        assert!(actual.distance(expected) < 1e-12);
        assert_eq!(
            binding
                .map_reference_local_point(0, DVec3::ZERO, 1.)
                .unwrap()
                .1,
            DVec3::ZERO
        );
        for ratio in [0., -1., f64::NAN, f64::INFINITY, 1e6 + 1.] {
            assert!(binding.map_reference_local_point(0, point, ratio).is_err());
        }
        assert!(binding.map_reference_local_point(1, point, 1.).is_err());
        assert!(
            binding
                .map_reference_local_point(0, DVec3::splat(f64::NAN), 1.)
                .is_err()
        );
        assert_eq!(
            binding.map_reference_local_point(0, point, 1.4).unwrap().1,
            mapped
        );
    }
}
