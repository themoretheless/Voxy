//! Playback descriptors shared by editor and browser projects.
use crate::{ModelAsset, ModelError, fail};
use voxy_animation::AnimatorFrame;

/// Marker on the selected clip's normalized authored timeline.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelAnimationEvent {
    pub name: String,
    pub phase: f64,
}

/// Authored playback selection. `None` keeps the bind pose; zero speed pauses.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelAnimation {
    /// Persistent absolute overrides keyed by exact unique authored node name.
    /// Empty preserves imported defaults and animated weight channels.
    #[serde(default)]
    pub morph_weights: std::collections::BTreeMap<String, Vec<f32>>,
    #[serde(default)]
    pub events: Vec<ModelAnimationEvent>,
    pub clip: Option<usize>,
    /// Exact unique clip name; empty retains legacy numeric selection.
    #[serde(default)]
    pub clip_name: String,
    pub speed: f32,
    /// Clip-switch fade duration in seconds. Zero retains immediate switching.
    #[serde(default)]
    pub transition_seconds: f32,
    #[serde(default)]
    pub root_motion_joint: u16,
    /// Exact unique authored node name. Empty preserves legacy numeric selection.
    #[serde(default)]
    pub root_motion_bone: String,
    #[serde(default)]
    pub root_motion_axes: [bool; 3],
    #[serde(default)]
    pub root_motion_rotation: bool,
}
impl Default for ModelAnimation {
    fn default() -> Self {
        Self {
            morph_weights: Default::default(),
            events: Vec::new(),
            clip: Some(0),
            clip_name: String::new(),
            speed: 1.0,
            transition_seconds: 0.0,
            root_motion_joint: 0,
            root_motion_bone: String::new(),
            root_motion_axes: [false; 3],
            root_motion_rotation: false,
        }
    }
}

impl ModelAnimation {
    pub fn apply_morph_weights(
        &self,
        model: &ModelAsset,
        frame: &mut AnimatorFrame,
    ) -> Result<(), ModelError> {
        self.validate(None, None).map_err(fail)?;
        if self.morph_weights.is_empty() {
            return Ok(());
        }
        let mut candidate = frame.clone();
        for (name, weights) in &self.morph_weights {
            let node = model.resolve_morph_node_name(name)?;
            model.set_frame_morph_weights(&mut candidate, node, weights)?;
        }
        *frame = candidate;
        Ok(())
    }

    pub fn resolve_clip(&self, model: &ModelAsset) -> Result<Option<usize>, ModelError> {
        if self.clip_name.is_empty() {
            return Ok(self.clip);
        }
        let mut matches = model
            .animations
            .iter()
            .enumerate()
            .filter(|(_, clip)| clip.name() == self.clip_name);
        let index = matches
            .next()
            .ok_or_else(|| fail("named animation clip is missing"))?
            .0;
        if matches.next().is_some() {
            return Err(fail("named animation clip is ambiguous"));
        }
        Ok(Some(index))
    }
    pub fn resolve_motion_joint(&self, model: &ModelAsset) -> Result<u16, ModelError> {
        if self.root_motion_bone.is_empty() {
            Ok(self.root_motion_joint)
        } else {
            model.resolve_joint_name(&self.root_motion_bone)
        }
    }

    pub fn validate(
        &self,
        clip_count: Option<usize>,
        joint_count: Option<usize>,
    ) -> Result<(), &'static str> {
        let mut weight_count = 0usize;
        for (name, weights) in &self.morph_weights {
            weight_count = weight_count.saturating_add(weights.len());
            if name.is_empty()
                || name.len() > 1024
                || name.contains('\0')
                || weights.is_empty()
                || weights.iter().any(|weight| !weight.is_finite())
                || weight_count > voxy_animation::MAX_PROPERTY_TRACKS
            {
                return Err("invalid authored morph weights");
            }
        }
        if self.events.len() > 4096
            || self.events.iter().any(|event| {
                event.name.is_empty()
                    || event.name.len() > 1024
                    || event.name.contains('\0')
                    || !event.phase.is_finite()
                    || !(0. ..=1.).contains(&event.phase)
            })
        {
            return Err("invalid model animation events");
        }
        if !self.transition_seconds.is_finite() || !(0. ..=60.).contains(&self.transition_seconds) {
            return Err("invalid model animation transition duration");
        }
        if !self.speed.is_finite() || !(0.0..=8.0).contains(&self.speed) {
            return Err("invalid model animation speed");
        }
        if self.clip_name.len() > 1024 || self.clip_name.contains('\0') {
            return Err("invalid model animation clip name");
        }
        if self.clip_name.is_empty()
            && self
                .clip
                .zip(clip_count)
                .is_some_and(|(index, count)| index >= count)
        {
            return Err("invalid model animation clip");
        }
        if self.root_motion_bone.len() > 1024 || self.root_motion_bone.contains('\0') {
            return Err("invalid model animation motion bone name");
        }
        let joint = usize::from(self.root_motion_joint);
        if joint >= voxy_animation::MAX_JOINTS
            || (self.root_motion_bone.is_empty()
                && joint_count.is_some_and(|count| joint >= count && (count != 0 || joint != 0)))
        {
            return Err("invalid model animation motion joint");
        }
        Ok(())
    }
}

// Only immutable clip admission proofs establish an ancestor for the whole interval.
// For rotation, a uniform signed scale is orthogonal up to magnitude; its axis
// conversion uses the proper pseudovector basis det(S)*S, including reflections.
/// Resolves the fixed parent basis used to convert extracted root motion.
/// # Errors
/// Rejects unknown joints, moving ancestors and invalid rotation bases.
pub fn constant_parent_basis(
    model: &ModelAsset,
    settings: &ModelAnimation,
    root: u16,
) -> Result<(glam::Mat4, glam::DQuat, f64), ModelError> {
    let mut matrix = glam::Mat4::IDENTITY;
    let mut rotation = glam::DQuat::IDENTITY;
    let mut uniform_scale = 1_f64;
    let mut parent = model
        .skeleton
        .joints()
        .get(usize::from(root))
        .ok_or_else(|| fail("invalid root motion joint"))?
        .parent;
    let clip = settings
        .clip
        .map(|index| {
            model
                .animations
                .get(index)
                .ok_or_else(|| fail("invalid root motion clip"))
        })
        .transpose()?;
    while let Some(index) = parent {
        let index = usize::from(index);
        let joint = &model.skeleton.joints()[index];
        let transform = if let Some(clip) = clip {
            clip.constant_joint_transform(index).ok_or_else(|| {
                fail("root motion parent is moving or unproved; choose a fixed locomotion root")
            })?
        } else {
            joint.bind_local
        };
        if settings.root_motion_rotation {
            let scale = transform.scale.abs();
            if scale.x != scale.y || scale.x != scale.z {
                return Err(fail(
                    "A parent of the selected motion bone has nonuniform scale; root rotation requires uniform scale",
                ));
            }
            let signs = transform.scale.signum().as_dvec3();
            let determinant = signs.x * signs.y * signs.z;
            uniform_scale *= f64::from(scale.x) * determinant;
            let reflection =
                glam::DQuat::from_mat3(&glam::DMat3::from_diagonal(signs * determinant))
                    .normalize();
            let authored =
                glam::DQuat::from_array(transform.rotation.to_array().map(f64::from)).normalize();
            rotation = (authored * reflection * rotation).normalize();
        }
        matrix = transform.matrix() * matrix;
        parent = joint.parent;
    }
    if !matrix.is_finite()
        || !rotation.is_finite()
        || !uniform_scale.is_finite()
        || uniform_scale == 0.
    {
        return Err(fail("root motion parent conversion overflow"));
    }
    Ok((matrix, rotation, uniform_scale))
}

/// Compiles a static ancestor similarity from stored TRS values before matrix rounding.
/// # Errors
/// Rejects moving or incompatible ancestors and invalid similarities.
pub fn constant_parent_similarity_enclosure(
    model: &ModelAsset,
    root: u16,
) -> Result<
    (
        voxy_animation::RootRigidEnclosure,
        voxy_animation::RootUniformScaleEnclosure,
    ),
    ModelError,
> {
    static_parent_similarity_enclosure(model, root, |index, joint| {
        let transform = model
            .animations
            .first()
            .map_or(Some(joint.bind_local), |clip| {
                clip.constant_joint_transform(index)
            })
            .ok_or_else(|| fail("root parent reference is animated"))?;
        if model
            .animations
            .iter()
            .any(|clip| clip.constant_joint_transform(index) != Some(transform))
        {
            return Err(fail("root parent reference differs between clips"));
        }
        Ok(transform)
    })
}

pub fn static_parent_similarity_enclosure(
    model: &ModelAsset,
    root: u16,
    mut transform_at: impl FnMut(
        usize,
        &voxy_animation::Joint,
    ) -> Result<voxy_animation::Transform, ModelError>,
) -> Result<
    (
        voxy_animation::RootRigidEnclosure,
        voxy_animation::RootUniformScaleEnclosure,
    ),
    ModelError,
> {
    use voxy_animation::{RootRigidEnclosure, RootRigidTransform, RootUniformScaleEnclosure};
    let mut frame = RootRigidEnclosure::IDENTITY;
    let mut scale = RootUniformScaleEnclosure::ONE;
    let mut parent = model
        .skeleton
        .joints()
        .get(usize::from(root))
        .ok_or_else(|| fail("invalid root parent reference joint"))?
        .parent;
    while let Some(index) = parent {
        let index = usize::from(index);
        let joint = &model.skeleton.joints()[index];
        let transform = transform_at(index, joint)?;
        let magnitude = transform.scale.abs();
        if magnitude.x == 0. || magnitude.x != magnitude.y || magnitude.x != magnitude.z {
            return Err(fail("root parent reference requires nonzero uniform scale"));
        }
        let signs = transform.scale.signum();
        let determinant = signs.x * signs.y * signs.z;
        let proper = signs * determinant;
        let reflection = if proper == glam::Vec3::ONE {
            glam::DQuat::IDENTITY
        } else if proper.x == 1. {
            glam::DQuat::from_xyzw(1., 0., 0., 0.)
        } else if proper.y == 1. {
            glam::DQuat::from_xyzw(0., 1., 0., 0.)
        } else {
            glam::DQuat::from_xyzw(0., 0., 1., 0.)
        };
        let authored = RootRigidEnclosure::from_transform(RootRigidTransform {
            translation: transform.translation.as_dvec3(),
            rotation: glam::DQuat::from_array(transform.rotation.to_array().map(f64::from)),
        })
        .map_err(|error| fail(error.to_string()))?;
        let reflection = RootRigidEnclosure::from_transform(RootRigidTransform {
            rotation: reflection,
            ..RootRigidTransform::IDENTITY
        })
        .map_err(|error| fail(error.to_string()))?;
        let outer = authored
            .compose(&reflection)
            .map_err(|error| fail(error.to_string()))?;
        let outer_scale =
            RootUniformScaleEnclosure::from_scale(f64::from(magnitude.x) * f64::from(determinant))
                .map_err(|error| fail(error.to_string()))?;
        frame = frame
            .with_translation_scale_enclosed(outer_scale)
            .and_then(|inner| outer.compose(&inner))
            .map_err(|error| fail(error.to_string()))?;
        scale = outer_scale
            .multiplied(scale)
            .map_err(|error| fail(error.to_string()))?;
        parent = joint.parent;
    }
    Ok((frame, scale))
}
