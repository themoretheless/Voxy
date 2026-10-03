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
    settings: ModelAnimation,
    playback: ModelPlayback,
    frame: Arc<AnimatorFrame>,
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
            let settings = scene
                .component::<ModelAnimation>(owner)
                .map_err(|e| e.to_string())?
                .cloned()
                .unwrap_or(ModelAnimation {
                    clip: (!model.animations.is_empty()).then_some(0),
                    ..Default::default()
                });
            let current = next.owners.get(&owner);
            let mut playback = if current.is_none_or(|old| {
                !Arc::ptr_eq(&old.model, model) || old.settings.clip != settings.clip
            }) {
                ModelPlayback::new(model.clone(), settings.clone())?
            } else {
                let mut playback = current.unwrap().playback.clone();
                playback.set_speed(settings.speed)?;
                playback.set_root_motion_joint(settings.resolve_motion_joint(model)?)?;
                playback
            };
            let (mut frame, trajectory) = playback.advance_with_motion(
                dt,
                settings.root_motion_rotation,
                settings.root_motion_axes,
                |_, frame, path| Ok((frame.clone(), path.cloned())),
            )?;
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
                    constant_parent_basis(model, &settings, frame.root_motion_joint)?;
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
            let frame = Arc::new(frame);
            next.owners.insert(
                owner,
                Owner {
                    model: model.clone(),
                    settings,
                    playback,
                    frame,
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
