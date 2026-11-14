//! Persisted, explicit retarget authoring data; compiled rig bindings stay immutable.
use crate::{ModelAsset, ModelError, fail};
use std::sync::Arc;
use voxy_animation::{RetargetBinding, RetargetJoint, Skeleton};

fn identity_basis() -> [f32; 4] {
    [0., 0., 0., 1.]
}
fn unit_scale() -> f32 {
    1.
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetargetJointProfile {
    pub source: String,
    pub target: String,
    #[serde(default = "identity_basis")]
    pub rotation_basis: [f32; 4],
    #[serde(default = "identity_basis")]
    pub translation_basis: [f32; 4],
    #[serde(default = "unit_scale")]
    pub translation_scale: f32,
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRetarget {
    pub source: String,
    pub joints: Vec<RetargetJointProfile>,
}
impl ModelRetarget {
    /// Validates authored data even while asynchronous model imports are pending.
    pub fn validate(&self) -> Result<(), ModelError> {
        let valid_name =
            |name: &str| !name.is_empty() && name.len() <= 1024 && !name.contains('\0');
        if !valid_name(&self.source)
            || self.joints.is_empty()
            || self.joints.len() > voxy_animation::MAX_JOINTS
        {
            return Err(fail("invalid retarget source or joint capacity"));
        }
        let mut source_names = std::collections::BTreeSet::new();
        let mut target_names = std::collections::BTreeSet::new();
        for joint in &self.joints {
            let rotation = glam::Quat::from_array(joint.rotation_basis);
            let translation = glam::Quat::from_array(joint.translation_basis);
            if !valid_name(&joint.source)
                || !valid_name(&joint.target)
                || !source_names.insert(&joint.source)
                || !target_names.insert(&joint.target)
                || !rotation.is_finite()
                || !rotation.is_normalized()
                || !translation.is_finite()
                || !translation.is_normalized()
                || !joint.translation_scale.is_finite()
                || !(0. ..=1e6).contains(&joint.translation_scale)
            {
                return Err(fail("invalid retarget joint profile"));
            }
        }
        Ok(())
    }
    /// Resolves public imported bone names before binding canonical skeleton names.
    pub fn compile_models(
        &self,
        source: &ModelAsset,
        target: &ModelAsset,
    ) -> Result<RetargetBinding, ModelError> {
        self.validate()?;
        if source.has_morph() || target.has_morph() {
            return Err(fail("morph retargeting needs an explicit shape mapping"));
        }
        let mapping = self
            .joints
            .iter()
            .map(|joint| {
                let source_index = source.resolve_joint_name(&joint.source)?;
                let target_index = target.resolve_joint_name(&joint.target)?;
                Ok(RetargetJoint {
                    source: source.skeleton.joints()[usize::from(source_index)]
                        .name
                        .clone(),
                    target: target.skeleton.joints()[usize::from(target_index)]
                        .name
                        .clone(),
                    rotation_basis: glam::Quat::from_array(joint.rotation_basis),
                    translation_basis: glam::Quat::from_array(joint.translation_basis),
                    translation_scale: joint.translation_scale,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        RetargetBinding::new(&source.skeleton, &target.skeleton, &mapping)
            .map_err(|error| fail(error.to_string()))
    }
    pub fn compile(
        &self,
        source: &Skeleton,
        target: &Skeleton,
    ) -> Result<RetargetBinding, ModelError> {
        self.validate()?;
        let mapping: Vec<_> = self
            .joints
            .iter()
            .map(|joint| RetargetJoint {
                source: Arc::from(joint.source.as_str()),
                target: Arc::from(joint.target.as_str()),
                rotation_basis: glam::Quat::from_array(joint.rotation_basis),
                translation_basis: glam::Quat::from_array(joint.translation_basis),
                translation_scale: joint.translation_scale,
            })
            .collect();
        RetargetBinding::new(source, target, &mapping).map_err(|error| fail(error.to_string()))
    }
}

use crate::{ModelAnimation, constant_parent_basis};
// A mapped ancestor must retain its bind transform across source clips and fades.
// Unmapped target ancestors are bind pose by the retarget contract.
pub fn retarget_parent_basis(
    target: &ModelAsset,
    source: &ModelAsset,
    profile: &crate::ModelRetarget,
    root: u16,
    rotation: bool,
) -> Result<(glam::Mat4, glam::DQuat, f64), ModelError> {
    validate_retarget_motion_parents(target, source, profile, root)?;
    constant_parent_basis(
        target,
        &ModelAnimation {
            clip: None,
            root_motion_rotation: rotation,
            ..Default::default()
        },
        root,
    )
}

pub fn validate_retarget_motion_parents(
    target: &ModelAsset,
    source: &ModelAsset,
    profile: &crate::ModelRetarget,
    root: u16,
) -> Result<(), ModelError> {
    let mut parent = target
        .skeleton
        .joints()
        .get(usize::from(root))
        .ok_or_else(|| fail("invalid retarget root motion joint"))?
        .parent;
    while let Some(index) = parent {
        for mapping in &profile.joints {
            if target.resolve_joint_name(&mapping.target)? == index {
                let source_index = usize::from(source.resolve_joint_name(&mapping.source)?);
                if source
                    .animations
                    .iter()
                    .any(|clip| !clip.joint_uses_bind_pose(source_index))
                {
                    return Err(fail(
                        "retarget motion parent is animated; interval basis is unproved",
                    ));
                }
            }
        }
        parent = target.skeleton.joints()[usize::from(index)].parent;
    }
    Ok(())
}
