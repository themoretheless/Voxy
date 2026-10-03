//! Playback state belongs to a model owner, never to the shared imported asset.
use std::sync::Arc;
use voxy_animation::{Animator, AnimatorFrame};
use voxy_render::ModelAsset;

/// Authored playback selection. `None` keeps the bind pose; zero speed pauses.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelAnimation {
    pub clip: Option<usize>,
    pub speed: f32,
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
            speed: 1.0,
            root_motion_joint: 0,
            root_motion_bone: String::new(),
            root_motion_axes: [false; 3],
            root_motion_rotation: false,
        }
    }
}

impl ModelAnimation {
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
        if !self.speed.is_finite() || !(0.0..=8.0).contains(&self.speed) {
            return Err("invalid model animation speed");
        }
        if self
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
pub(crate) struct ModelPlayback {
    model: Arc<ModelAsset>,
    animator: Option<Animator>,
    root_motion_joint: u16,
    contact_interval: Option<voxy_animation::AnimationPhaseInterval>,
}
impl ModelPlayback {
    pub(crate) fn new(model: Arc<ModelAsset>, settings: ModelAnimation) -> Result<Self, String> {
        settings.validate(
            Some(model.animations.len()),
            Some(model.skeleton.joints().len()),
        )?;
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
        let interval = self.animator.as_ref().map(|animator| animator.phase_interval(dt))
            .transpose().map_err(|error| error.to_string())?;
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
}
