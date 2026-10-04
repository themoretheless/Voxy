//! Playback state belongs to a model owner, never to the shared imported asset.
use std::sync::Arc;
use voxy_animation::{Animator, AnimatorFrame};
use voxy_render::ModelAsset;

/// Authored playback selection. `None` keeps the bind pose; zero speed pauses.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelAnimation {
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
    pub(crate) fn resolve_clip(&self, model: &ModelAsset) -> Result<Option<usize>, String> {
        if self.clip_name.is_empty() {
            return Ok(self.clip);
        }
        let mut matches = model
            .animations
            .iter()
            .enumerate()
            .filter(|(_, clip)| clip.name() == self.clip_name);
        let index = matches.next().ok_or("named animation clip is missing")?.0;
        if matches.next().is_some() {
            return Err("named animation clip is ambiguous".into());
        }
        Ok(Some(index))
    }
    pub(crate) fn resolve_motion_joint(&self, model: &ModelAsset) -> Result<u16, String> {
        if self.root_motion_bone.is_empty() {
            Ok(self.root_motion_joint)
        } else {
            model
                .resolve_joint_name(&self.root_motion_bone)
                .map_err(|error| error.to_string())
        }
    }

    pub(crate) fn validate(
        &self,
        clip_count: Option<usize>,
        joint_count: Option<usize>,
    ) -> Result<(), &'static str> {
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

#[derive(Clone, Debug)]
pub(crate) struct SourceContactInterval {
    pub clip: Arc<voxy_animation::AnimationClip>,
    pub phase: voxy_animation::AnimationPhaseInterval,
    pub active_tick_fraction: f64,
}
#[derive(Clone, Debug)]
pub(crate) struct ModelPlayback {
    model: Arc<ModelAsset>,
    animator: Option<Animator>,
    root_motion_joint: u16,
    contact_interval: Option<voxy_animation::AnimationPhaseInterval>,
    source_contact_interval: Option<SourceContactInterval>,
    frozen_source_tick: Option<(Arc<voxy_animation::Pose>, f64)>,
}
/// Asset-bound immutable preparation; publication remains in the owner tick.
#[derive(Debug)]
pub(crate) struct PreparedModelFade {
    model: Arc<ModelAsset>,
    plan: voxy_animation::RootRigidFadePlan,
}
impl PreparedModelFade {
    /// Original-source compiler with the same asset/animator receipt owner.
    /// Numerical body/publication admission is still a separate obligation.
    pub(crate) fn bind_original_sources_common_similarity(
        self,
        owner: voxy_scene::NodeId,
        common: voxy_animation::RootRigidEnclosure,
        scale: voxy_animation::RootUniformScaleEnclosure,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<PreparedModelFadeMotion, String> {
        let motion = self
            .plan
            .integrate_original_sources_common_similarity(
                common,
                scale,
                origin_tolerance,
                angular_tolerance,
                max_spans,
            )
            .map_err(|error| error.to_string())?;
        Ok(PreparedModelFadeMotion {
            owner,
            prepared: self,
            motion,
        })
    }

    pub(crate) fn bind_motion(
        self,
        owner: voxy_scene::NodeId,
        source_frame: Option<voxy_animation::RootRigidTransform>,
        target_frame: voxy_animation::RootRigidTransform,
        origin_tolerance: f64,
        angular_tolerance: f64,
        max_spans: usize,
    ) -> Result<PreparedModelFadeMotion, String> {
        let motion = self
            .plan
            .integrate_certified_tick(
                source_frame,
                target_frame,
                origin_tolerance,
                angular_tolerance,
                max_spans,
            )
            .map_err(|error| error.to_string())?;
        Ok(PreparedModelFadeMotion {
            owner,
            prepared: self,
            motion,
        })
    }
    pub(crate) fn accepted_frame(
        &self,
        current: &ModelPlayback,
        accepted_wall_seconds: f64,
    ) -> Result<(Animator, AnimatorFrame), String> {
        let animator = current
            .animator
            .as_ref()
            .ok_or("fade animator disappeared")?;
        if !Arc::ptr_eq(&self.model, &current.model) || !self.plan.matches_animator(animator) {
            return Err("staged fade model or animator changed".into());
        }
        self.plan
            .prepare_accepted_frame(&self.model.skeleton, accepted_wall_seconds)
            .map_err(|error| error.to_string())
    }
}
#[derive(Debug)]
pub(crate) struct PreparedModelFadeMotion {
    owner: voxy_scene::NodeId,
    prepared: PreparedModelFade,
    motion: voxy_animation::RootRigidCertifiedFadeInterval,
}
impl PreparedModelFadeMotion {
    /// Stages playback and contact travel for the same accepted physical prefix.
    /// The returned owner state is published only after the physics transaction.
    pub(crate) fn accepted_playback(
        &self,
        current: &ModelPlayback,
        receipt: &voxy_gameplay::AppliedCharacterTrajectoryMotion,
    ) -> Result<(ModelPlayback, AnimatorFrame), String> {
        let (animator, frame) = self.accepted_frame(current, receipt)?;
        let wall = self
            .motion
            .accepted_wall_time(
                receipt.completed_spans,
                receipt.span_fraction,
                receipt.complete,
            )
            .map_err(|error| error.to_string())?;
        let initial = current
            .animator
            .as_ref()
            .ok_or("fade animator disappeared")?;
        let contact_interval = initial
            .phase_interval_wall(wall)
            .map_err(|error| error.to_string())?;
        let source = initial
            .source_phase_interval_wall(wall)
            .map_err(|error| error.to_string())?
            .map(|source| SourceContactInterval {
                clip: source.clip.clone(),
                phase: source.phase,
                active_tick_fraction: source.active_tick_fraction,
            });
        let frozen = initial
            .frozen_source_tick_wall(wall)
            .map_err(|error| error.to_string())?
            .map(|source| (source.snapshot.clone(), source.active_tick_fraction));
        let mut candidate = current.clone();
        candidate.animator = Some(animator);
        candidate.contact_interval = Some(contact_interval);
        candidate.source_contact_interval = source;
        candidate.frozen_source_tick = frozen;
        Ok((candidate, frame))
    }

    pub(crate) fn accepted_frame(
        &self,
        current: &ModelPlayback,
        receipt: &voxy_gameplay::AppliedCharacterTrajectoryMotion,
    ) -> Result<(Animator, AnimatorFrame), String> {
        if receipt.owner != self.owner
            || !receipt.matches_rigid_trajectory(&self.motion.approximation().path)
        {
            return Err("accepted fade receipt belongs to another motion".into());
        }
        let time = self
            .motion
            .accepted_wall_time(
                receipt.completed_spans,
                receipt.span_fraction,
                receipt.complete,
            )
            .map_err(|error| error.to_string())?;
        self.prepared.accepted_frame(current, time)
    }
    pub(crate) fn request(
        &self,
        basis: glam::DQuat,
        origin: glam::Vec3,
        scale: f64,
        coordinate_axis: usize,
        evaluation_radius: f64,
    ) -> voxy_gameplay::CharacterCertifiedFadeMotion<'_> {
        voxy_gameplay::CharacterCertifiedFadeMotion {
            owner: self.owner,
            fade: &self.motion,
            basis,
            origin,
            scale,
            coordinate_axis,
            evaluation_radius,
            evaluation_axes: None,
        }
    }
}
impl ModelPlayback {
    pub(crate) fn reference_at_current_phase(
        &self,
        axes: [bool; 3],
        current_root_to_body: voxy_animation::RootRigidEnclosure,
        scale: voxy_animation::RootUniformScaleEnclosure,
    ) -> Result<voxy_animation::RootRigidEnclosure, String> {
        let animator = self
            .animator
            .as_ref()
            .ok_or("root reference needs a playing clip")?;
        let factor = match animator
            .root_rigid_source_phase_factor_enclosure(axes)
            .map_err(|e| e.to_string())?
        {
            Some(source) => source,
            // Ordinary playback retains unsupported STEP channels. Such moving
            // channels cannot enter the original-source fade compiler.
            None => voxy_animation::RootRigidEnclosure::from_transform(
                animator
                    .root_rigid_phase_factor(axes)
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?,
        };
        let factor = factor
            .with_translation_scale_enclosed(scale)
            .map_err(|e| e.to_string())?;
        current_root_to_body
            .compose(&factor.inverse().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }

    pub(crate) fn prepare_certified_fade(
        &self,
        dt: f32,
        axes: [bool; 3],
        max_spans: usize,
    ) -> Result<Option<PreparedModelFade>, String> {
        self.prepare_certified_fade_wall(f64::from(dt), axes, max_spans)
    }
    pub(crate) fn prepare_certified_fade_wall(
        &self,
        dt: f64,
        axes: [bool; 3],
        max_spans: usize,
    ) -> Result<Option<PreparedModelFade>, String> {
        let plan = self
            .animator
            .as_ref()
            .map(|animator| {
                animator.prepare_root_rigid_fade_wall(&self.model.skeleton, dt, axes, max_spans)
            })
            .transpose()
            .map_err(|error| error.to_string())?
            .flatten();
        Ok(plan.map(|plan| PreparedModelFade {
            model: self.model.clone(),
            plan,
        }))
    }

    pub(crate) fn can_rebind(&self, model: &ModelAsset, allow_reorder: bool) -> bool {
        self.model.skeleton.joints() == model.skeleton.joints()
            && self.model.animations.len() == model.animations.len()
            && (self
                .model
                .animations
                .iter()
                .zip(&model.animations)
                .all(|(old, new)| old.has_same_authored_animation(new))
                || (allow_reorder
                    && self.model.animations.iter().all(|old| {
                        let mut matches = model
                            .animations
                            .iter()
                            .filter(|new| new.name() == old.name());
                        matches
                            .next()
                            .is_some_and(|new| old.has_same_authored_animation(new))
                            && matches.next().is_none()
                            && self
                                .model
                                .animations
                                .iter()
                                .filter(|clip| clip.name() == old.name())
                                .count()
                                == 1
                    })))
    }
    pub(crate) fn rebind(&mut self, model: Arc<ModelAsset>) {
        self.model = model;
    }
    pub(crate) fn new(
        model: Arc<ModelAsset>,
        mut settings: ModelAnimation,
    ) -> Result<Self, String> {
        settings.validate(
            Some(model.animations.len()),
            Some(model.skeleton.joints().len()),
        )?;
        settings.clip = settings.resolve_clip(&model)?;
        let root_motion_joint = settings.resolve_motion_joint(&model)?;
        let animator = match settings.clip {
            Some(index) => {
                let clip = model
                    .animations
                    .get(index)
                    .ok_or("invalid model animation clip")?;
                if !clip.is_compatible_with(&model.skeleton) {
                    return Err("animation clip belongs to an incompatible rig".into());
                }
                let mut animator = Animator::new(clip.clone());
                animator
                    .set_speed(settings.speed)
                    .map_err(|error| error.to_string())?;
                animator
                    .set_root_motion_joint(root_motion_joint)
                    .map_err(|error| error.to_string())?;
                Some(animator)
            }
            None => None,
        };
        Ok(Self {
            model,
            animator,
            root_motion_joint,
            contact_interval: None,
            source_contact_interval: None,
            frozen_source_tick: None,
        })
    }

    pub(crate) fn set_root_motion_joint(&mut self, joint: u16) -> Result<(), String> {
        if usize::from(joint) >= self.model.skeleton.joints().len() {
            return Err("invalid model animation motion joint".into());
        }
        if let Some(animator) = &mut self.animator {
            animator
                .set_root_motion_joint(joint)
                .map_err(|error| error.to_string())?;
        }
        self.root_motion_joint = joint;
        Ok(())
    }

    pub(crate) fn pose_blend_phases(&self) -> Option<voxy_animation::PoseBlendPhases<'_>> {
        self.animator.as_ref().map(Animator::pose_blend_phases)
    }

    pub(crate) fn has_transition(&self) -> bool {
        self.animator
            .as_ref()
            .is_some_and(|animator| animator.pose_blend_phases().source.is_some())
    }

    pub(crate) fn reload_clip(&mut self, index: usize, duration: f32) -> Result<(), String> {
        let clip = self
            .model
            .animations
            .get(index)
            .ok_or("invalid reload clip")?;
        let animator = self
            .animator
            .as_mut()
            .ok_or("bind pose has no reload phase")?;
        animator
            .transition_to_at_phase(clip.clone(), duration, animator.normalized_phase())
            .map_err(|error| error.to_string())
    }

    pub(crate) fn transition_to_clip(&mut self, index: usize, duration: f32) -> Result<(), String> {
        let clip = self
            .model
            .animations
            .get(index)
            .ok_or("invalid transition clip")?;
        self.animator
            .as_mut()
            .ok_or("bind pose has no playing transition source")?
            .transition_to(clip.clone(), duration)
            .map_err(|e| e.to_string())
    }

    pub(crate) fn frozen_source_tick(&self) -> Option<&(Arc<voxy_animation::Pose>, f64)> {
        self.frozen_source_tick.as_ref()
    }
    pub(crate) fn source_contact_interval(&self) -> Option<&SourceContactInterval> {
        self.source_contact_interval.as_ref()
    }

    pub(crate) fn contact_interval(&self) -> Option<voxy_animation::AnimationPhaseInterval> {
        self.contact_interval
    }
    pub(crate) fn contact_phase(&self) -> Option<f64> {
        self.animator.as_ref().map(Animator::normalized_phase)
    }

    pub(crate) fn set_speed(&mut self, speed: f32) -> Result<(), String> {
        if !speed.is_finite() || !(0.0..=8.0).contains(&speed) {
            return Err("invalid model animation speed".into());
        }
        if let Some(animator) = &mut self.animator {
            animator
                .set_speed(speed)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// Publishes the clock only after the consumer accepts the validated frame.
    /// The consumer must preflight its own writes before committing resources.
    #[cfg(test)]
    pub(crate) fn advance_with<T>(
        &mut self,
        dt: f32,
        publish: impl FnOnce(&ModelAsset, &AnimatorFrame) -> Result<T, String>,
    ) -> Result<T, String> {
        self.advance_with_motion(dt, false, [false; 3], |model, frame, _| {
            publish(model, frame)
        })
    }

    pub(crate) fn advance_with_motion<T>(
        &mut self,
        dt: f32,
        rotation: bool,
        axes: [bool; 3],
        publish: impl FnOnce(
            &ModelAsset,
            &AnimatorFrame,
            Option<&voxy_animation::RootRigidPath>,
        ) -> Result<T, String>,
    ) -> Result<T, String> {
        if !dt.is_finite() || !(0.0..=1.0).contains(&dt) {
            return Err("invalid model animation timestep".into());
        }
        let interval = self
            .animator
            .as_ref()
            .map(|animator| animator.phase_interval(dt))
            .transpose()
            .map_err(|error| error.to_string())?;
        let source_interval = self
            .animator
            .as_ref()
            .map(|animator| animator.source_phase_interval(dt))
            .transpose()
            .map_err(|error| error.to_string())?
            .flatten()
            .map(|source| SourceContactInterval {
                clip: source.clip.clone(),
                phase: source.phase,
                active_tick_fraction: source.active_tick_fraction,
            });
        let frozen_source_tick = self
            .animator
            .as_ref()
            .map(|animator| animator.frozen_source_tick(dt))
            .transpose()
            .map_err(|e| e.to_string())?
            .flatten()
            .map(|source| (source.snapshot.clone(), source.active_tick_fraction));
        let mut candidate = self.animator.clone();
        let mut path = None;
        let frame = if let Some(animator) = &mut candidate {
            if rotation {
                let (frame, rotation) = animator
                    .advance_with_root_rigid_motion(
                        &self.model.skeleton,
                        dt,
                        axes,
                        voxy_animation::MAX_ROOT_ROTATION_SPANS,
                    )
                    .map_err(|error| error.to_string())?;
                path = Some(rotation);
                frame
            } else {
                animator
                    .advance(&self.model.skeleton, dt)
                    .map_err(|error| error.to_string())?
            }
        } else {
            let pose = self.model.skeleton.bind_pose();
            let skin_matrices = pose
                .skin_matrices(&self.model.skeleton)
                .map_err(|error| error.to_string())?;
            AnimatorFrame {
                pose,
                skin_matrices,
                root_motion: glam::Vec3::ZERO,
                root_motion_joint: self.root_motion_joint,
                transition_weight: 1.0,
            }
        };
        let result = publish(&self.model, &frame, path.as_ref())?;
        self.animator = candidate;
        self.contact_interval = interval;
        self.source_contact_interval = source_interval;
        self.frozen_source_tick = frozen_source_tick;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn model() -> Arc<ModelAsset> {
        let glb = include_bytes!("../../voxy_render/examples/assets/animated-triangle.glb");
        Arc::new(ModelAsset::parse(glb, &[], voxy_render::ModelLimits::default()).unwrap())
    }
    #[test]
    fn shared_asset_has_independent_owner_clocks_and_failed_publication_retries() {
        let model = model();
        let mut first = ModelPlayback::new(model.clone(), ModelAnimation::default()).unwrap();
        let mut other = ModelPlayback::new(
            model,
            ModelAnimation {
                speed: 0.0,
                ..ModelAnimation::default()
            },
        )
        .unwrap();
        let take = |_: &ModelAsset, frame: &AnimatorFrame| Ok(frame.pose.clone());
        let paused = other.advance_with(0.5, take).unwrap();
        let moving = first.advance_with(0.5, take).unwrap();
        assert_ne!(paused, moving);
        let mut control = first.clone();
        assert!(
            first
                .advance_with(0.25, |_, _| Err::<(), _>("GPU admission rejected".into()))
                .is_err()
        );
        assert_eq!(
            first.advance_with(0.25, take).unwrap(),
            control.advance_with(0.25, take).unwrap()
        );
        assert_eq!(other.advance_with(0.5, take).unwrap(), paused);
    }
    #[test]
    fn foreign_clip_is_rejected_before_owner_creation_or_publication() {
        let mut asset = model();
        let mut joints = asset.skeleton.joints().to_vec();
        joints[0].name = Arc::from("foreign root");
        let foreign = voxy_animation::Skeleton::new(joints).unwrap();
        let clip = voxy_animation::AnimationClip::new(
            "foreign",
            1.,
            voxy_animation::Playback::Loop,
            vec![voxy_animation::JointTrack::default(); foreign.joints().len()],
            &foreign,
        )
        .unwrap();
        Arc::make_mut(&mut asset).animations = vec![Arc::new(clip)];
        assert!(asset.sample_pose(Some(0), 0.).is_err());
        assert!(ModelPlayback::new(asset, ModelAnimation::default()).is_err());
    }

    #[test]
    fn cubic_invalid_pose_never_reaches_publication_and_preserves_owner_clock() {
        use voxy_animation::{
            AnimationClip, Interpolation, JointTangents, JointTrack, Playback, QuatKey,
            TrackInterpolation,
        };
        let mut model = model();
        let count = model.skeleton.joints().len();
        let mut tracks = vec![JointTrack::default(); count];
        tracks[0].rotations = vec![
            QuatKey {
                time: 0.0,
                value: glam::Quat::IDENTITY,
            },
            QuatKey {
                time: 2.0,
                value: -glam::Quat::IDENTITY,
            },
        ];
        let mut modes = vec![TrackInterpolation::default(); count];
        modes[0].rotation = Interpolation::CubicSpline;
        let mut tangents = vec![JointTangents::default(); count];
        tangents[0].rotation = vec![[glam::Vec4::ZERO; 2]; 2];
        let clip = AnimationClip::new_with_tangents(
            "invalid middle",
            2.0,
            Playback::Clamp,
            tracks,
            modes,
            tangents,
            &model.skeleton,
        )
        .unwrap();
        Arc::make_mut(&mut model).animations = vec![Arc::new(clip)];
        assert!(model.sample_pose(Some(0), 1.0).is_err());
        let mut playback = ModelPlayback::new(model, ModelAnimation::default()).unwrap();
        playback.advance_with(0.25, |_, _| Ok(())).unwrap();
        let mut control = playback.clone();
        let mut published = false;
        assert!(
            playback
                .advance_with(0.75, |_, _| {
                    published = true;
                    Ok(())
                })
                .is_err()
        );
        assert!(!published);
        let pose = playback
            .advance_with(0.25, |_, f| Ok(f.pose.clone()))
            .unwrap();
        assert_eq!(
            pose,
            control
                .advance_with(0.25, |_, f| Ok(f.pose.clone()))
                .unwrap()
        );
    }

    #[test]
    fn named_selection_resolves_imported_names_without_resetting_time() {
        let model = model();
        let name = model
            .joint_names()
            .iter()
            .flatten()
            .next()
            .unwrap()
            .to_string();
        let settings = ModelAnimation {
            root_motion_bone: name.clone(),
            ..Default::default()
        };
        let joint = settings.resolve_motion_joint(&model).unwrap();
        let mut playback = ModelPlayback::new(model.clone(), ModelAnimation::default()).unwrap();
        playback.advance_with(0.25, |_, _| Ok(())).unwrap();
        let mut control = playback.clone();
        playback.set_root_motion_joint(joint).unwrap();
        let actual = playback
            .advance_with(0.25, |_, frame| Ok(frame.clone()))
            .unwrap();
        assert_eq!(
            actual.pose,
            control
                .advance_with(0.25, |_, frame| Ok(frame.pose.clone()))
                .unwrap()
        );
        assert_eq!(actual.root_motion_joint, joint);
        let encoded = serde_json::to_string(&settings).unwrap();
        assert_eq!(
            serde_json::from_str::<ModelAnimation>(&encoded).unwrap(),
            settings
        );
        let mut bad = settings.clone();
        bad.root_motion_bone = "missing bone".into();
        assert!(ModelPlayback::new(model, bad).is_err());
    }

    #[test]
    fn motion_joint_settings_keep_old_scenes_and_owner_clock_compatible() {
        let old: ModelAnimation = serde_json::from_str(r#"{"clip":0,"speed":1.0}"#).unwrap();
        assert_eq!(old.root_motion_joint, 0);
        let model = model();
        let settings = ModelAnimation {
            root_motion_joint: 1,
            ..ModelAnimation::default()
        };
        let encoded = serde_json::to_string(&settings).unwrap();
        assert_eq!(
            serde_json::from_str::<ModelAnimation>(&encoded).unwrap(),
            settings
        );
        let mut playback = ModelPlayback::new(model.clone(), old.clone()).unwrap();
        playback.advance_with(0.25, |_, _| Ok(())).unwrap();
        let mut control = playback.clone();
        playback.set_root_motion_joint(1).unwrap();
        assert!(playback.set_root_motion_joint(u16::MAX).is_err());
        let actual = playback.advance_with(0.25, |_, f| Ok(f.clone())).unwrap();
        let expected = control.advance_with(0.25, |_, f| Ok(f.clone())).unwrap();
        assert_eq!(actual.root_motion_joint, 1);
        assert_eq!(actual.pose, expected.pose);
        assert!(
            ModelPlayback::new(
                model.clone(),
                ModelAnimation {
                    root_motion_joint: 255,
                    ..old
                }
            )
            .is_err()
        );
        let mut bind = ModelPlayback::new(
            model,
            ModelAnimation {
                clip: None,
                ..settings
            },
        )
        .unwrap();
        let frame = bind.advance_with(0.25, |_, f| Ok(f.clone())).unwrap();
        assert_eq!(frame.root_motion_joint, 1);
        assert_eq!(frame.root_motion, glam::Vec3::ZERO);
    }

    #[test]
    fn bind_selection_and_invalid_settings_are_explicit() {
        let model = model();
        assert!(
            ModelPlayback::new(
                model.clone(),
                ModelAnimation {
                    clip: Some(usize::MAX),
                    speed: 1.0,
                    ..ModelAnimation::default()
                }
            )
            .is_err()
        );
        assert!(
            ModelPlayback::new(
                model.clone(),
                ModelAnimation {
                    clip: None,
                    speed: f32::NAN,
                    ..ModelAnimation::default()
                }
            )
            .is_err()
        );
        let bind = model.skeleton.bind_pose();
        let mut playback = ModelPlayback::new(
            model,
            ModelAnimation {
                clip: None,
                speed: 1.0,
                ..ModelAnimation::default()
            },
        )
        .unwrap();
        assert!(playback.advance_with(f32::NAN, |_, _| Ok(())).is_err());
        assert_eq!(
            playback
                .advance_with(1.0, |_, frame| Ok(frame.pose.clone()))
                .unwrap(),
            bind
        );
    }
    #[test]
    fn source_contact_interval_survives_fade_completion_and_failed_publication() {
        let mut asset = (*model()).clone();
        let source = asset.animations[0].clone();
        asset.animations.push(Arc::new(
            voxy_animation::AnimationClip::new(
                "target",
                1.,
                voxy_animation::Playback::Loop,
                vec![voxy_animation::JointTrack::default(); asset.skeleton.joints().len()],
                &asset.skeleton,
            )
            .unwrap(),
        ));
        let mut playback = ModelPlayback::new(Arc::new(asset), ModelAnimation::default()).unwrap();
        playback.advance_with(0.25, |_, _| Ok(())).unwrap();
        playback.transition_to_clip(1, 0.25).unwrap();
        assert!(
            playback
                .advance_with(0.5, |_, _| Err::<(), _>("reject".into()))
                .is_err()
        );
        assert!(playback.source_contact_interval().is_none());
        assert_eq!(playback.contact_phase(), Some(0.));
        assert_eq!(
            playback
                .advance_with(0.5, |_, frame| Ok(frame.transition_weight))
                .unwrap(),
            1.
        );
        assert!(!playback.has_transition());
        let held = playback.source_contact_interval().unwrap().clone();
        assert!(Arc::ptr_eq(&held.clip, &source));
        assert_eq!(held.phase.start, 0.25);
        assert_eq!(held.phase.end, 0.5);
        assert_eq!(held.active_tick_fraction, 0.5);
        assert!(
            playback
                .advance_with(0.1, |_, _| Err::<(), _>("reject".into()))
                .is_err()
        );
        let retained = playback.source_contact_interval().unwrap();
        assert!(Arc::ptr_eq(&held.clip, &retained.clip));
        assert_eq!(retained.phase, held.phase);
        assert_eq!(retained.active_tick_fraction, held.active_tick_fraction);
        playback.advance_with(0.1, |_, _| Ok(())).unwrap();
        assert!(playback.source_contact_interval().is_none());
    }

    #[test]
    fn frozen_source_tick_survives_completion_and_rejection_then_releases_snapshot() {
        let mut asset = (*model()).clone();
        asset.animations.push(Arc::new(
            voxy_animation::AnimationClip::new(
                "target",
                1.,
                voxy_animation::Playback::Loop,
                vec![voxy_animation::JointTrack::default(); asset.skeleton.joints().len()],
                &asset.skeleton,
            )
            .unwrap(),
        ));
        let mut playback = ModelPlayback::new(Arc::new(asset), ModelAnimation::default()).unwrap();
        playback.advance_with(0.1, |_, _| Ok(())).unwrap();
        playback.transition_to_clip(1, 0.5).unwrap();
        playback.advance_with(0.1, |_, _| Ok(())).unwrap();
        playback.transition_to_clip(0, 0.25).unwrap();
        assert!(
            playback
                .advance_with(0.5, |_, _| Err::<(), _>("reject".into()))
                .is_err()
        );
        assert!(playback.frozen_source_tick().is_none());
        assert_eq!(playback.contact_phase(), Some(0.));
        playback.advance_with(0.5, |_, _| Ok(())).unwrap();
        assert!(!playback.has_transition());
        assert_eq!(playback.frozen_source_tick().unwrap().1, 0.5);
        let snapshot = Arc::downgrade(&playback.frozen_source_tick().unwrap().0);
        assert!(
            playback
                .advance_with(0.1, |_, _| Err::<(), _>("reject".into()))
                .is_err()
        );
        assert_eq!(playback.frozen_source_tick().unwrap().1, 0.5);
        assert!(snapshot.upgrade().is_some());
        playback.advance_with(0.1, |_, _| Ok(())).unwrap();
        assert!(playback.frozen_source_tick().is_none());
        assert!(snapshot.upgrade().is_none());
    }
}

#[cfg(test)]
mod accepted_fade_tests {
    use super::*;
    #[test]
    fn asset_bound_fade_rejects_rebind_and_changed_clock_before_pose_preparation() {
        let glb = include_bytes!("../../voxy_render/examples/assets/animated-triangle.glb");
        let asset =
            Arc::new(ModelAsset::parse(glb, &[], voxy_render::ModelLimits::default()).unwrap());
        let mut playback = ModelPlayback::new(asset.clone(), ModelAnimation::default()).unwrap();
        playback.transition_to_clip(0, 0.125).unwrap();
        let staged = playback
            .prepare_certified_fade(0.25, [true; 3], 256)
            .unwrap()
            .unwrap();
        assert!(staged.accepted_frame(&playback, 0.0625).is_ok());
        assert!(staged.accepted_frame(&playback, 0.5).is_err());
        let mut changed = playback.clone();
        changed.set_speed(0.5).unwrap();
        assert!(staged.accepted_frame(&changed, 0.0625).is_err());
        let replacement =
            Arc::new(ModelAsset::parse(glb, &[], voxy_render::ModelLimits::default()).unwrap());
        changed = playback.clone();
        changed.rebind(replacement);
        assert!(staged.accepted_frame(&changed, 0.0625).is_err());
        assert!(staged.accepted_frame(&playback, 0.).is_ok());
    }
}

#[cfg(test)]
mod physical_fade_receipt_tests {
    use super::*;
    #[test]
    fn physical_receipt_prepares_only_bound_fade_and_failed_preparation_rolls_back() {
        exercise_physical_fade_receipt(false);
    }
    #[test]
    fn original_source_fade_preserves_asset_receipt_and_transactional_rollback() {
        exercise_physical_fade_receipt(true);
    }
    fn exercise_physical_fade_receipt(original_sources: bool) {
        use voxy_animation::RootRigidTransform;
        use voxy_gameplay::{CharacterBody, CharacterPhysics, CharacterTickError, player_input};
        use voxy_scene::{SceneGraph, Transform};
        let glb = include_bytes!("../../voxy_render/examples/assets/animated-triangle.glb");
        let mut model = ModelAsset::parse(glb, &[], voxy_render::ModelLimits::default()).unwrap();
        if original_sources {
            use voxy_animation::{
                AnimationClip, Interpolation, JointTangents, JointTrack, Playback, QuatKey,
                TrackInterpolation, Vec3Key,
            };
            let count = model.skeleton.joints().len();
            let mut tracks = vec![JointTrack::default(); count];
            tracks[0].translations = vec![
                Vec3Key {
                    time: 0.,
                    value: glam::Vec3::ZERO,
                },
                Vec3Key {
                    time: 1.,
                    value: glam::Vec3::X,
                },
            ];
            tracks[0].rotations = vec![
                QuatKey {
                    time: 0.,
                    value: glam::Quat::IDENTITY,
                },
                QuatKey {
                    time: 1.,
                    value: glam::Quat::IDENTITY,
                },
            ];
            let mut modes = vec![TrackInterpolation::default(); count];
            modes[0].rotation = Interpolation::CubicSpline;
            let mut tangents = vec![JointTangents::default(); count];
            tangents[0].rotation = vec![[glam::Vec4::ZERO; 2]; 2];
            model.animations = vec![Arc::new(
                AnimationClip::new_with_tangents(
                    "source receipt",
                    1.,
                    Playback::Loop,
                    tracks,
                    modes,
                    tangents,
                    &model.skeleton,
                )
                .unwrap(),
            )];
        }
        let model = Arc::new(model);
        let mut playback = ModelPlayback::new(model, ModelAnimation::default()).unwrap();
        playback.transition_to_clip(0, 0.125).unwrap();
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                owner,
                CharacterBody {
                    gravity: 0.,
                    speed: 0.,
                    ..Default::default()
                },
            )
            .unwrap();
        let dt = 0.0625;
        let build = || {
            let prepared = playback
                .prepare_certified_fade(dt, [true; 3], 256)
                .unwrap()
                .unwrap();
            if original_sources {
                prepared
                    .bind_original_sources_common_similarity(
                        owner,
                        voxy_animation::RootRigidEnclosure::IDENTITY,
                        voxy_animation::RootUniformScaleEnclosure::from_scale(1.).unwrap(),
                        0.01,
                        0.01,
                        4096,
                    )
                    .unwrap()
            } else {
                prepared
                    .bind_motion(
                        owner,
                        Some(RootRigidTransform::IDENTITY),
                        RootRigidTransform::IDENTITY,
                        0.01,
                        0.01,
                        4096,
                    )
                    .unwrap()
            }
        };
        let staged = build();
        let other = build();
        let request = staged.request(glam::DQuat::IDENTITY, glam::Vec3::ZERO, 1., 1, 0.);
        let mut physics = CharacterPhysics::new(&scene, 1, 0);
        let mut input = player_input().unwrap();
        let before = scene.local(owner).unwrap();
        let result = physics.fixed_step_with_certified_fade_preparation(
            &mut scene,
            &mut input,
            f64::from(dt),
            &[request],
            |preview, _| {
                staged.accepted_frame(&playback, &preview.motions[0])?;
                assert!(
                    other
                        .accepted_frame(&playback, &preview.motions[0])
                        .is_err()
                );
                Err::<(), String>("palette rejected".into())
            },
        );
        assert_eq!(
            result.unwrap_err(),
            CharacterTickError::Preparation("palette rejected".into())
        );
        assert_eq!(scene.local(owner).unwrap(), before);
        assert!(physics.state(&scene, owner).unwrap().is_none());
        let (receipts, (candidate, frame)) = physics
            .fixed_step_with_certified_fade_preparation(
                &mut scene,
                &mut input,
                f64::from(dt),
                &[request],
                |preview, _| staged.accepted_playback(&playback, &preview.motions[0]),
            )
            .unwrap();
        assert!(receipts[0].complete);
        if original_sources {
            assert_eq!(
                scene.local(owner).unwrap().translation.x - before.translation.x,
                dt
            );
        }
        assert!(frame.transition_weight > 0.);
        assert!(
            candidate.contact_interval().unwrap().end > candidate.contact_interval().unwrap().start
        );
        assert_eq!(
            candidate.contact_interval().unwrap().end,
            candidate.contact_phase().unwrap()
        );
        assert!(candidate.source_contact_interval().is_some());
        assert!(playback.contact_interval().is_none());
        playback = candidate;
        assert!(staged.accepted_frame(&playback, &receipts[0]).is_err());
    }
}
