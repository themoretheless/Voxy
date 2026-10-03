//! Fixed-tick animation authority. Rendering receives immutable accepted frames.
use crate::{ModelAnimation, ModelInstance, ModelPart, model_playback::ModelPlayback};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
use voxy_animation::AnimatorFrame;
use voxy_assets::AssetId;
use voxy_render::ModelAsset;
use voxy_scene::{NodeId, SceneGraph, SceneId};

pub(super) const MAX_OWNERS: usize = 128;

/// The editor adds its playback authority to the existing character transaction.
pub(super) fn schedule() -> Result<&'static voxy_scene::SchedulePlan, voxy_scene::ScheduleError> {
    static PLAN: std::sync::OnceLock<Result<voxy_scene::SchedulePlan, voxy_scene::ScheduleError>> =
        std::sync::OnceLock::new();
    PLAN.get_or_init(|| {
        voxy_gameplay::gameplay_schedule()?.with_resource_access(
            "character.step",
            voxy_scene::SystemAccess {
                resource: "animation.playback".into(),
                write: true,
            },
        )
    })
    .as_ref()
    .map_err(Clone::clone)
}

#[derive(Clone, Debug)]
struct Owner {
    model: Arc<ModelAsset>,
    animation_model: Arc<ModelAsset>,
    retarget_profile: Option<crate::ModelRetarget>,
    retarget_binding: Option<voxy_animation::RetargetBinding>,
    settings: ModelAnimation,
    playback: ModelPlayback,
    frame: Arc<AnimatorFrame>,
    feet: Option<crate::foot_placement::FootRuntime>,
}
#[derive(Clone, Debug, Default)]
pub(super) struct AnimationRuntime {
    scene: Option<SceneId>,
    owners: HashMap<NodeId, Owner>,
    serial: u64,
    motions: Vec<(NodeId, glam::Vec3)>,
    rotations: Vec<(
        NodeId,
        voxy_animation::RootRigidPath,
        glam::DQuat,
        glam::Vec3,
        f64,
    )>,
}
impl AnimationRuntime {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
    pub(super) fn motions(&self) -> &[(NodeId, glam::Vec3)] {
        &self.motions
    }
    pub(super) fn trajectories(&self) -> Vec<voxy_gameplay::CharacterRigidTrajectoryMotion<'_>> {
        self.rotations
            .iter()
            .map(|(owner, trajectory, basis, origin, scale)| {
                voxy_gameplay::CharacterRigidTrajectoryMotion {
                    owner: *owner,
                    trajectory,
                    basis: *basis,
                    origin: *origin,
                    scale: *scale,
                }
            })
            .collect()
    }
    pub(super) fn has_foot_placement(&self) -> bool {
        self.owners.values().any(|owner| owner.feet.is_some())
    }
    pub(super) fn correct_feet(mut self, preview: &voxy_gameplay::CharacterTickPreview,
        budget: &mut voxy_gameplay::SupportQueryBudget) -> Result<Self, String> {
        for accepted in &preview.characters {
            let Some(owner) = self.owners.get_mut(&accepted.owner) else { continue; };
            let Some(feet) = &mut owner.feet else { continue; };
            owner.frame = Arc::new(feet.correct_at_phase(&owner.model, (*owner.frame).clone(),
                accepted.world_matrix.as_dmat4(), accepted.grounded, preview, budget, owner.playback.pose_blend_phases().map(|phase| phase.target.clip.name()), owner.playback.contact_phase(), owner.playback.contact_interval(), owner.playback.pose_blend_phases(), owner.playback.source_contact_interval(), owner.playback.frozen_source_tick())?);
        }
        Ok(self)
    }
    pub(super) fn clip_name(&self, owner: NodeId) -> Option<&str> {
        Some(self.owners.get(&owner)?.playback.pose_blend_phases()?.target.clip.name())
    }
    pub(super) fn clip_phase(&self, owner: NodeId) -> Option<f64> {
        self.owners.get(&owner)?.playback.contact_phase()
    }
    pub(super) fn serial(&self) -> u64 {
        self.serial
    }
    pub(super) fn frame(
        &self,
        owner: NodeId,
        model: &Arc<ModelAsset>,
    ) -> Option<Arc<AnimatorFrame>> {
        let state = self.owners.get(&owner)?;
        Arc::ptr_eq(&state.model, model).then(|| state.frame.clone())
    }
    pub(super) fn synchronize(&mut self, scene: &SceneGraph) -> Result<(), String> {
        if self.scene.is_some_and(|id| id != scene.identity()) {
            return Err("foreign animation scene".into());
        }
        self.owners.retain(|owner, _| {
            scene
                .component::<ModelInstance>(*owner)
                .ok()
                .flatten()
                .is_some()
                && scene
                    .component::<ModelPart>(*owner)
                    .ok()
                    .flatten()
                    .is_none_or(|part| part.node == u32::MAX)
        });
        Ok(())
    }
    /// Stage one fixed tick; publish the returned owner only after physics accepts.
    pub(super) fn prepare(
        &self,
        scene: &SceneGraph,
        models: &BTreeMap<AssetId, Arc<ModelAsset>>,
        dt: f32,
    ) -> Result<Self, String> {
        if !dt.is_finite() || dt <= 0.0 || dt > 0.1 {
            return Err("invalid fixed animation timestep".into());
        }
        let mut next = self.clone();
        next.motions.clear();
        next.rotations.clear();
        let mut rotation_spans = 0_usize;
        next.synchronize(scene)?;
        next.scene = Some(scene.identity());
        // Imported hierarchy parts reference the shared model but own no clocks.
        // Inactive logical owners count too: their retained playback consumes storage.
        let owner_count = scene
            .components::<ModelInstance>()
            .filter(|(owner, instance)| {
                (models.contains_key(&instance.asset) || next.owners.contains_key(owner))
                    && scene
                        .component::<ModelPart>(*owner)
                        .ok()
                        .flatten()
                        .is_none_or(|part| part.node == u32::MAX)
            })
            .count();
        if owner_count > MAX_OWNERS {
            return Err("animation owner capacity exceeded".into());
        }
        for (owner, instance) in scene.active_components::<ModelInstance>() {
            if scene
                .component::<ModelPart>(owner)
                .map_err(|e| e.to_string())?
                .is_some_and(|part| part.node != u32::MAX)
            {
                continue;
            }
            let Some(model) = models.get(&instance.asset) else {
                continue;
            };
            let profile = scene.component::<crate::ModelRetarget>(owner).map_err(|e|e.to_string())?.cloned();
            let animation_model = if let Some(profile) = &profile {
                models.get(&AssetId(profile.source.clone())).ok_or("retarget source is not loaded")?
            } else { model };
            let current = next.owners.get(&owner);
            let same_binding = current.is_some_and(|old| old.retarget_profile == profile
                && old.model.skeleton.joints() == model.skeleton.joints());
            let binding = if let Some(profile) = &profile {
                if let Some(old) = current.filter(|old| same_binding && Arc::ptr_eq(&old.animation_model, animation_model)) {
                    old.retarget_binding.clone()
                } else { Some(profile.compile_models(animation_model, model)?) }
            } else { None };
            let mut settings = scene
                .component::<ModelAnimation>(owner)
                .map_err(|e| e.to_string())?
                .cloned()
                .unwrap_or(ModelAnimation {
                    clip: (!animation_model.animations.is_empty()).then_some(0),
                    ..Default::default()
                });
            settings.validate(Some(animation_model.animations.len()), Some(animation_model.skeleton.joints().len()))?;
            settings.clip = settings.resolve_clip(animation_model)?;

            let named_identity = current.is_some_and(|old| !settings.clip_name.is_empty()
                && old.settings.clip_name == settings.clip_name);
            let same_clip = current.is_some_and(|old| old.settings.clip == settings.clip || named_identity);
            let compatible_model = current.is_some_and(|old|
                same_binding && (Arc::ptr_eq(&old.animation_model, animation_model) || old.playback.can_rebind(animation_model, named_identity)));
            let reload_transition = same_binding && !compatible_model && settings.transition_seconds > 0.
                && current.is_some_and(|old| same_clip
                    && old.animation_model.skeleton.joints() == animation_model.skeleton.joints()
                    && old.settings.clip.zip(settings.clip).is_some_and(|(old_index,index)| old.animation_model.animations.get(old_index)
                        .zip(animation_model.animations.get(index)).is_some_and(|(a,b)| a.name() == b.name())));
            let mut playback = if current.is_none_or(|old| {
                !(compatible_model || reload_transition)
                    || (!same_clip
                        && (settings.transition_seconds == 0. || old.settings.clip.is_none() || settings.clip.is_none()))
            }) {
                ModelPlayback::new(animation_model.clone(), settings.clone())?
            } else {
                let mut playback = current.unwrap().playback.clone();
                playback.rebind(animation_model.clone());
                if reload_transition {
                    playback.reload_clip(settings.clip.ok_or("missing reload target")?,settings.transition_seconds)?;
                }
                if !same_clip {
                    playback.transition_to_clip(settings.clip.ok_or("missing transition target")?, settings.transition_seconds)?;
                }
                playback.set_speed(settings.speed)?;
                playback.set_root_motion_joint(settings.resolve_motion_joint(animation_model)?)?;
                playback
            };
            let source_axes = if settings.root_motion_rotation {
                if let Some(binding) = &binding {
                    binding.source_root_motion_axes(settings.resolve_motion_joint(animation_model)?, settings.root_motion_axes)
                        .map_err(|e|e.to_string())?
                } else { settings.root_motion_axes }
            } else { settings.root_motion_axes };
            let (mut frame, mut trajectory) = playback.advance_with_motion(
                dt,
                settings.root_motion_rotation,
                source_axes,
                |_, frame, path| Ok((frame.clone(), path.cloned())),
            )?;
            if let Some(binding) = &binding {
                if let Some(path) = trajectory.as_ref() {
                    trajectory = Some(binding.apply_root_path(path, frame.root_motion_joint, settings.root_motion_axes)
                        .map_err(|e|e.to_string())?);
                }
                frame = binding.apply_frame(&frame).map_err(|e|e.to_string())?;
            }
            if settings.root_motion_rotation
                || settings.root_motion_axes.into_iter().any(|axis| axis)
            {
                if scene
                    .component::<voxy_gameplay::CharacterBody>(owner)
                    .map_err(|error| error.to_string())?
                    .is_none()
                {
                    return Err("root motion requires a CharacterBody on the model owner".into());
                }
                let (basis, rotation_basis, scale) =
                    if let Some(profile) = &profile {
                        retarget_parent_basis(model, animation_model, profile, frame.root_motion_joint, settings.root_motion_rotation)?
                    } else { constant_parent_basis(model, &settings, frame.root_motion_joint)? };
                let (in_place, displacement) = frame
                    .into_in_place_translation(&model.skeleton, settings.root_motion_axes)
                    .map_err(|error| error.to_string())?;
                frame = in_place;
                let world = scene
                    .world_matrix(owner)
                    .map_err(|error| error.to_string())?;
                let displacement = world.transform_vector3(basis.transform_vector3(displacement));
                if !basis.is_finite() || !displacement.is_finite() {
                    return Err("root motion coordinate conversion overflow".into());
                }
                if !settings.root_motion_rotation {
                    next.motions.push((owner, displacement));
                }
                if settings.root_motion_rotation {
                    let origin = basis.w_axis.truncate();
                    if !origin.is_finite() {
                        return Err("root trajectory origin conversion overflow".into());
                    }
                    frame = frame
                        .without_root_rotation(&model.skeleton)
                        .map_err(|error| error.to_string())?;
                    if let Some(trajectory) = trajectory {
                        rotation_spans = rotation_spans
                            .checked_add(trajectory.spans().len())
                            .ok_or("root rotation span overflow")?;
                        if rotation_spans > voxy_gameplay::MAX_CHARACTER_TRAJECTORY_SPANS {
                            return Err(
                                "aggregate root rotation trajectory capacity exceeded".into()
                            );
                        }
                        next.rotations
                            .push((owner, trajectory, rotation_basis, origin, scale));
                    }
                }
            }
            let feet = if let Some(foot_settings) = scene.component::<crate::ModelFootPlacement>(owner)
                .map_err(|e| e.to_string())?.filter(|settings| !settings.feet.is_empty()) {
                if scene.component::<voxy_gameplay::CharacterBody>(owner).map_err(|e| e.to_string())?.is_none() {
                    return Err("foot placement requires a CharacterBody on the model owner".into());
                }
                if let Some(feet) = current.filter(|_| (compatible_model || reload_transition)
                    && (same_clip || playback.has_transition()
                        || playback.source_contact_interval().is_some() || playback.frozen_source_tick().is_some())).and_then(|old| old.feet.as_ref())
                    .filter(|feet| feet.matches(foot_settings)) {
                    Some(feet.clone())
                } else { Some(crate::foot_placement::FootRuntime::new_with_clips(model, foot_settings.clone(), &animation_model.animations)?) }
            } else { None };
            let frame = Arc::new(frame);
            next.owners.insert(
                owner,
                Owner {
                    model: model.clone(),
                    animation_model: animation_model.clone(),
                    retarget_profile: profile,
                    retarget_binding: binding,
                    settings,
                    playback,
                    frame,
                    feet,
                },
            );
        }
        next.serial = next
            .serial
            .checked_add(1)
            .ok_or("animation tick overflow")?;
        Ok(next)
    }
}

// A mapped ancestor must retain its bind transform across source clips and fades.
// Unmapped target ancestors are bind pose by the retarget contract.
fn retarget_parent_basis(target: &ModelAsset, source: &ModelAsset, profile: &crate::ModelRetarget, root: u16, rotation: bool)
    -> Result<(glam::Mat4, glam::DQuat, f64), String> {
    let mut parent = target.skeleton.joints()[usize::from(root)].parent;
    while let Some(index) = parent {
        for mapping in &profile.joints {
            if target.resolve_joint_name(&mapping.target).map_err(|e|e.to_string())? == index {
                let source_index = usize::from(source.resolve_joint_name(&mapping.source).map_err(|e|e.to_string())?);
                if source.animations.iter().any(|clip| !clip.joint_uses_bind_pose(source_index)) {
                    return Err("retarget motion parent is animated; interval basis is unproved".into());
                }
            }
        }
        parent = target.skeleton.joints()[usize::from(index)].parent;
    }
    constant_parent_basis(target, &ModelAnimation { clip: None, root_motion_rotation:rotation, ..Default::default() }, root)
}

// Only immutable clip admission proofs establish an ancestor for the whole interval.
// For rotation, a uniform signed scale is orthogonal up to magnitude; its axis
// conversion uses the proper pseudovector basis det(S)*S, including reflections.
fn constant_parent_basis(
    model: &ModelAsset,
    settings: &ModelAnimation,
    root: u16,
) -> Result<(glam::Mat4, glam::DQuat, f64), String> {
    let mut matrix = glam::Mat4::IDENTITY;
    let mut rotation = glam::DQuat::IDENTITY;
    let mut uniform_scale = 1_f64;
    let mut parent = model.skeleton.joints()[usize::from(root)].parent;
    while let Some(index) = parent {
        let index = usize::from(index);
        let joint = &model.skeleton.joints()[index];
        let transform = if let Some(clip) = settings.clip {
            model.animations[clip]
                .constant_joint_transform(index)
                .ok_or("root motion parent is moving or unproved; choose a fixed locomotion root")?
        } else {
            joint.bind_local
        };
        if settings.root_motion_rotation {
            let scale = transform.scale.abs();
            if scale.x != scale.y || scale.x != scale.z {
                return Err("A parent of the selected motion bone has nonuniform scale; root rotation requires uniform scale".into());
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
        return Err("root motion parent conversion overflow".into());
    }
    Ok((matrix, rotation, uniform_scale))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod rotation_tests;
