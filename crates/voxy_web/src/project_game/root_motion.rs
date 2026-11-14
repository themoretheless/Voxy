//! Authored root motion uses the same fixed parent basis as native Play.
use super::{
    ClipOverride, EventBatch, ProjectGame, ProjectGraphics, ProjectPlayback, RetargetPlayback,
    SampledProjectPose, StagedClip, clip_event_track, error,
};
use glam::{Mat4, Vec3};
use std::collections::BTreeMap;
use voxy_animation::{
    RootRigidCertifiedFadeInterval, RootRigidEnclosure, RootRigidFadePlan, RootRigidTransform,
    RootUniformScaleEnclosure,
};
use voxy_gameplay::{
    AcceptedCharacterPose, CharacterBody, CharacterCertifiedFadeMotion, CharacterPhysics,
};
use voxy_input::InputMap;
use voxy_model::{ModelAnimation, ModelAsset};
use voxy_scene::{LoadedScene, NodeId, SceneGraph};
use wasm_bindgen::JsValue;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RootBasis {
    pub matrix: Mat4,
    pub rotation: glam::DQuat,
    pub scale: f64,
}

pub(super) fn root_basis(
    scene: &SceneGraph,
    owner: NodeId,
    model: &ModelAsset,
    settings: &ModelAnimation,
    retarget: Option<&RetargetPlayback>,
) -> Result<Option<RootBasis>, JsValue> {
    if !settings.root_motion_rotation && !settings.root_motion_axes.into_iter().any(|axis| axis) {
        return Ok(None);
    }
    if scene
        .component::<CharacterBody>(owner)
        .map_err(error)?
        .is_none()
    {
        return Err(error(
            "root motion requires a CharacterBody on the model owner",
        ));
    }
    let joint = settings.resolve_motion_joint(model).map_err(error)?;
    let (matrix, rotation, scale) = if let Some(retarget) = retarget {
        if settings.root_motion_rotation {
            return Err(error(
                "browser retargeted rotational root motion is not supported",
            ));
        }
        let target_joint = retarget.binding.target_motion_joint(joint).map_err(error)?;
        voxy_model::retarget_parent_basis(
            &retarget.target,
            model,
            &retarget.profile,
            target_joint,
            false,
        )
        .map_err(error)?
    } else {
        voxy_model::constant_parent_basis(model, settings, joint).map_err(error)?
    };
    Ok(Some(RootBasis {
        matrix,
        rotation,
        scale,
    }))
}

pub(super) fn extract(
    frame: voxy_animation::AnimatorFrame,
    model: &ModelAsset,
    settings: &ModelAnimation,
) -> Result<(voxy_animation::AnimatorFrame, Vec3), voxy_animation::AnimationError> {
    let (frame, translation) =
        frame.into_in_place_translation(&model.skeleton, settings.root_motion_axes)?;
    let frame = if settings.root_motion_rotation {
        frame.without_root_rotation(&model.skeleton)?
    } else {
        frame
    };
    Ok((frame, translation))
}

pub(super) fn display_frame(
    mut frame: voxy_animation::AnimatorFrame,
    source: &ModelAsset,
    settings: &ModelAnimation,
    retarget: Option<&RetargetPlayback>,
) -> Result<(voxy_animation::AnimatorFrame, Vec3), voxy_animation::AnimationError> {
    let extracts =
        settings.root_motion_rotation || settings.root_motion_axes.into_iter().any(|axis| axis);
    let target = if let Some(retarget) = retarget {
        frame = if extracts {
            retarget.binding.apply_frame(&frame)?
        } else {
            retarget.binding.apply_pose_frame(&frame)?
        };
        retarget.target.as_ref()
    } else {
        source
    };
    if extracts {
        extract(frame, target, settings)
    } else {
        Ok((frame, Vec3::ZERO))
    }
}

fn world_displacement(
    scene: &SceneGraph,
    owner: NodeId,
    basis: Mat4,
    displacement: Vec3,
) -> Result<Vec3, String> {
    let world = scene
        .world_matrix(owner)
        .map_err(|error| error.to_string())?;
    let displacement = world.transform_vector3(basis.transform_vector3(displacement));
    if !displacement.is_finite() {
        return Err("root motion coordinate conversion overflow".into());
    }
    Ok(displacement)
}

impl ProjectGame {
    pub(super) fn authored_root_playbacks(
        loaded: &LoadedScene,
        graphics: &ProjectGraphics,
    ) -> Result<BTreeMap<NodeId, ClipOverride>, JsValue> {
        let mut playbacks = BTreeMap::new();
        for instance in &graphics.instances {
            let Some(playback) = &instance.playback else {
                continue;
            };
            let Some(basis) = playback.root_basis else {
                continue;
            };
            let Some(index) = playback.settings.clip else {
                continue;
            };
            let settings = playback.settings.clone();
            let target = graphics.models[&instance.asset]
                .animation
                .as_ref()
                .ok_or_else(|| error("missing root motion model"))?
                .clone();
            let retarget = playback.retarget.clone();
            let model = retarget
                .as_ref()
                .map_or(&target, |retarget| &retarget.source)
                .clone();
            let mut animator = model.animator(index).map_err(error)?;
            animator.set_speed(settings.speed).map_err(error)?;
            animator
                .set_root_motion_joint(settings.resolve_motion_joint(&model).map_err(error)?)
                .map_err(error)?;
            let pose = SampledProjectPose::from_frame(
                &target,
                ProjectPlayback::sample(&target, &settings, 0, retarget.as_ref())?,
                0,
            )
            .map_err(error)?;
            let identity = loaded
                .identity(instance.owner)
                .ok_or_else(|| error("unidentified root motion owner"))?
                .0
                .clone();
            let track = (!settings.events.is_empty())
                .then(|| clip_event_track(&settings, &model.animations[index]))
                .transpose()
                .map_err(error)?;
            playbacks.insert(
                instance.owner,
                ClipOverride {
                    model,
                    retarget,
                    animator,
                    settings,
                    identity,
                    track,
                    pose,
                    root_basis: Some(basis),
                    root_reference: None,
                },
            );
        }
        Ok(playbacks)
    }
}

impl ClipOverride {
    pub(super) fn stage_tick(
        &self,
        batch: &mut EventBatch,
        tick: u64,
        dt: f64,
    ) -> Result<StagedClip, String> {
        let mut animator = self.animator.clone();
        if self.settings.root_motion_rotation
            && self.settings.speed > 0.0
            && let Some(reference) = self.root_reference
            && let Some(plan) = animator
                .prepare_root_rigid_fade_wall(
                    &self.model.skeleton,
                    dt,
                    self.settings.root_motion_axes,
                    voxy_animation::MAX_ROOT_ROTATION_SPANS,
                )
                .map_err(|error| error.to_string())?
        {
            let motion = plan
                .integrate_original_sources_common_similarity(
                    reference.authored_to_body,
                    reference.scale,
                    1e-4,
                    1e-4,
                    voxy_animation::MAX_ROOT_ROTATION_SPANS,
                )
                .map_err(|error| format!("root fade integration at tick {tick}: {error}"))?;
            return Ok(StagedClip {
                animator,
                pose: self.pose.clone(),
                displacement: None,
                trajectory: None,
                fade: Some(PreparedFade { plan, motion }),
                reference: Some(reference),
            });
        }
        self.stage_events(batch, tick, dt)?;
        let (frame, path) = if self.settings.root_motion_rotation {
            let (frame, path) = animator
                .advance_with_root_rigid_motion_wall(
                    &self.model.skeleton,
                    dt,
                    self.settings.root_motion_axes,
                    voxy_animation::MAX_ROOT_ROTATION_SPANS,
                )
                .map_err(|error| error.to_string())?;
            (frame, Some(path))
        } else {
            (
                animator
                    .advance_wall(&self.model.skeleton, dt)
                    .map_err(|error| error.to_string())?,
                None,
            )
        };
        let (mut frame, extracted) =
            display_frame(frame, &self.model, &self.settings, self.retarget.as_ref())
                .map_err(|error| error.to_string())?;
        let (displacement, trajectory) = match (self.root_basis, path) {
            (Some(basis), Some(path)) => (None, Some((basis, path))),
            (Some(basis), None) => (Some((basis.matrix, extracted)), None),
            (None, _) => (None, None),
        };
        let model = self
            .retarget
            .as_ref()
            .map_or(&self.model, |retarget| &retarget.target);
        self.settings
            .apply_morph_weights(model, &mut frame)
            .map_err(|error| error.to_string())?;
        let pose = SampledProjectPose::from_frame(model, frame, tick)
            .map_err(|error| error.to_string())?;
        Ok(StagedClip {
            animator,
            pose,
            displacement,
            trajectory,
            fade: None,
            reference: self.root_reference,
        })
    }

    fn stage_events(&self, batch: &mut EventBatch, tick: u64, dt: f64) -> Result<(), String> {
        if let Some(track) = &self.track {
            let interval = self
                .animator
                .phase_interval_wall(dt)
                .map_err(|error| error.to_string())?;
            let occurrences = track.preview_target_tick(&self.animator, dt, batch.count_left())?;
            batch.append(
                &self.identity,
                track.clip().name(),
                tick,
                interval.start,
                interval.end,
                occurrences,
            )?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RootReference {
    scale: RootUniformScaleEnclosure,
    authored_to_world: RootRigidEnclosure,
    authored_to_body: RootRigidEnclosure,
}

impl RootReference {
    fn accept(
        previous: Option<Self>,
        owner: &ClipOverride,
        animator: &voxy_animation::Animator,
        pose: &AcceptedCharacterPose,
    ) -> Result<Self, String> {
        let body_to_world = RootRigidEnclosure::from_transform(RootRigidTransform {
            translation: pose.physical_center,
            rotation: pose.physical_rotation,
        })
        .map_err(|error| error.to_string())?;
        let (authored_to_world, scale) = if let Some(previous) = previous {
            (previous.authored_to_world, previous.scale)
        } else {
            let joint = owner
                .settings
                .resolve_motion_joint(&owner.model)
                .map_err(|error| error.to_string())?;
            let (parent, scale) =
                voxy_model::constant_parent_similarity_enclosure(&owner.model, joint)
                    .map_err(|error| error.to_string())?;
            let axes = owner.settings.root_motion_axes;
            let factor = match animator
                .root_rigid_source_phase_factor_enclosure(axes)
                .map_err(|error| error.to_string())?
            {
                Some(factor) => factor,
                None => RootRigidEnclosure::from_transform(
                    animator
                        .root_rigid_phase_factor(axes)
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())?,
            };
            let inverse = factor
                .with_translation_scale_enclosed(scale)
                .and_then(|factor| factor.inverse())
                .map_err(|error| error.to_string())?;
            let authored_to_world = parent
                .compose(&inverse)
                .and_then(|anchor| body_to_world.compose(&anchor))
                .map_err(|error| error.to_string())?;
            (authored_to_world, scale)
        };
        // Reproject the fixed world enclosure directly to avoid interval widening
        // from repeatedly composing the previous body and its inverse.
        let authored_to_body = body_to_world
            .inverse()
            .and_then(|world_to_body| world_to_body.compose(&authored_to_world))
            .map_err(|error| error.to_string())?;
        Ok(Self {
            scale,
            authored_to_world,
            authored_to_body,
        })
    }
}

pub(super) struct PreparedFade {
    plan: RootRigidFadePlan,
    motion: RootRigidCertifiedFadeInterval,
}

impl PreparedFade {
    fn coordinate_axis(&self, preferred: Option<usize>) -> Result<usize, String> {
        if let Some(axis) = preferred {
            return Ok(axis);
        }
        let mut best = None;
        for axis in [1, 0, 2] {
            if let Some(proof) = self
                .motion
                .coordinate_certificate(axis, 4096)
                .map_err(|error| error.to_string())?
            {
                let bound = proof.error_bound();
                if best.is_none_or(|(_, old)| bound < old) {
                    best = Some((axis, bound));
                }
            }
        }
        Ok(best.map_or(1, |(axis, _)| axis))
    }
}

pub(super) struct TickAnimations<'a> {
    pub staged: &'a mut [(NodeId, StagedClip)],
    pub owners: &'a BTreeMap<NodeId, ClipOverride>,
    pub batch: &'a mut EventBatch,
    pub tick: u64,
}

pub(super) fn apply_character_motion(
    physics: &mut CharacterPhysics,
    scene: &mut SceneGraph,
    input: &mut InputMap,
    dt: f64,
    animations: TickAnimations<'_>,
) -> Result<(), String> {
    let mut motion = Vec::new();
    let mut paths = Vec::new();
    let mut fades = Vec::new();
    let mut spans = 0;
    for (owner, state) in animations.staged.iter() {
        if let Some((basis, displacement)) = state.displacement {
            motion.push((
                *owner,
                world_displacement(scene, *owner, basis, displacement)?,
            ));
        }
        if let Some((basis, path)) = &state.trajectory {
            spans += path.spans().len();
            paths.push(voxy_gameplay::CharacterRigidTrajectoryMotion {
                owner: *owner,
                trajectory: path,
                scale: basis.scale,
                basis: basis.rotation,
                origin: basis.matrix.w_axis.truncate(),
            });
        }
        if let Some(fade) = &state.fade {
            spans += fade.motion.approximation().path.spans().len();
            let orientation = match physics
                .accepted_pose(scene, *owner)
                .map_err(|error| error.to_string())?
            {
                Some(pose) => pose.physical_rotation,
                None => scene
                    .local(*owner)
                    .map_err(|error| error.to_string())?
                    .rotation
                    .as_dquat()
                    .normalize(),
            };
            let preferred =
                voxy_gameplay::rigid_source_coordinate_axis(orientation, glam::DQuat::IDENTITY, 1);
            fades.push(CharacterCertifiedFadeMotion {
                owner: *owner,
                fade: &fade.motion,
                basis: glam::DQuat::IDENTITY,
                origin: Vec3::ZERO,
                scale: 1.,
                coordinate_axis: fade.coordinate_axis(preferred)?,
                evaluation_radius: 0.,
                evaluation_axes: None,
            });
        }
        if spans > voxy_gameplay::MAX_CHARACTER_TRAJECTORY_SPANS {
            return Err("aggregate root rotation trajectory capacity exceeded".into());
        }
    }
    if paths.is_empty() && fades.is_empty() {
        return physics
            .fixed_step_with_motion(scene, input, dt, &motion)
            .map(|_| ())
            .map_err(|error| error.to_string());
    }
    let (_, accepted) = physics
        .fixed_step_with_mixed_certified_fade_preparation(
            scene,
            input,
            dt,
            &motion,
            &paths,
            &fades,
            |preview, _| {
                let mut accepted = Vec::new();
                for (owner, staged) in animations.staged.iter() {
                    let current = &animations.owners[owner];
                    if !current.settings.root_motion_rotation {
                        continue;
                    }
                    let body = preview
                        .characters
                        .iter()
                        .find(|pose| pose.owner == *owner)
                        .ok_or("missing accepted root motion pose")?;
                    let playback = if let Some(fade) = &staged.fade {
                        let receipt = preview
                            .motions
                            .iter()
                            .find(|receipt| receipt.owner == *owner)
                            .ok_or("missing accepted fade receipt")?;
                        if !receipt.matches_rigid_trajectory(&fade.motion.approximation().path) {
                            return Err(
                                "accepted fade receipt belongs to another trajectory".into()
                            );
                        }
                        let wall = fade
                            .motion
                            .accepted_wall_time(
                                receipt.completed_spans,
                                receipt.span_fraction,
                                receipt.complete,
                            )
                            .map_err(|error| error.to_string())?;
                        let (animator, frame) = fade
                            .plan
                            .prepare_accepted_frame(&current.model.skeleton, wall)
                            .map_err(|error| error.to_string())?;
                        let (mut frame, _) = extract(frame, &current.model, &current.settings)
                            .map_err(|error| error.to_string())?;
                        current
                            .settings
                            .apply_morph_weights(&current.model, &mut frame)
                            .map_err(|error| error.to_string())?;
                        let pose =
                            SampledProjectPose::from_frame(&current.model, frame, animations.tick)
                                .map_err(|error| error.to_string())?;
                        current.stage_events(animations.batch, animations.tick, wall)?;
                        Some((animator, pose))
                    } else {
                        None
                    };
                    let animator = playback
                        .as_ref()
                        .map_or(&staged.animator, |(animator, _)| animator);
                    let reference =
                        RootReference::accept(staged.reference, current, animator, body)?;
                    accepted.push((playback, reference));
                }
                Ok::<_, String>(accepted)
            },
        )
        .map_err(|error| format!("{error:?}"))?;
    for (staged, (playback, reference)) in animations
        .staged
        .iter_mut()
        .filter(|(owner, _)| animations.owners[owner].settings.root_motion_rotation)
        .map(|(_, staged)| staged)
        .zip(accepted)
    {
        if let Some((animator, pose)) = playback {
            staged.animator = animator;
            staged.pose = pose;
        }
        staged.reference = Some(reference);
    }
    Ok(())
}
