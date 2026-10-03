//! Playback state belongs to a model owner, never to the shared imported asset.
use std::sync::Arc;
use voxy_animation::{Animator, AnimatorFrame};
use voxy_render::ModelAsset;

/// Authored playback selection. `None` keeps the bind pose; zero speed pauses.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelAnimation {
    pub clip: Option<usize>,
    pub speed: f32,
}
impl Default for ModelAnimation {
    fn default() -> Self {
        Self {
            clip: Some(0),
            speed: 1.0,
        }
    }
}

impl ModelAnimation {
    pub(crate) fn validate(&self, clip_count: Option<usize>) -> Result<(), &'static str> {
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
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ModelPlayback {
    model: Arc<ModelAsset>,
    animator: Option<Animator>,
}
impl ModelPlayback {
    pub(crate) fn new(model: Arc<ModelAsset>, settings: ModelAnimation) -> Result<Self, String> {
        settings.validate(Some(model.animations.len()))?;
        let animator = match settings.clip {
            Some(index) => {
                let clip = model
                    .animations
                    .get(index)
                    .ok_or("invalid model animation clip")?;
                let mut animator = Animator::new(clip.clone());
                animator
                    .set_speed(settings.speed)
                    .map_err(|error| error.to_string())?;
                Some(animator)
            }
            None => None,
        };
        Ok(Self { model, animator })
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
    pub(crate) fn advance_with<T>(
        &mut self,
        dt: f32,
        publish: impl FnOnce(&ModelAsset, &AnimatorFrame) -> Result<T, String>,
    ) -> Result<T, String> {
        if !dt.is_finite() || !(0.0..=1.0).contains(&dt) {
            return Err("invalid model animation timestep".into());
        }
        let mut candidate = self.animator.clone();
        let frame = if let Some(animator) = &mut candidate {
            animator
                .advance(&self.model.skeleton, dt)
                .map_err(|error| error.to_string())?
        } else {
            let pose = self.model.skeleton.bind_pose();
            let skin_matrices = pose
                .skin_matrices(&self.model.skeleton)
                .map_err(|error| error.to_string())?;
            AnimatorFrame {
                pose,
                skin_matrices,
                root_motion: glam::Vec3::ZERO,
                transition_weight: 1.0,
            }
        };
        let result = publish(&self.model, &frame)?;
        self.animator = candidate;
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
    fn bind_selection_and_invalid_settings_are_explicit() {
        let model = model();
        assert!(
            ModelPlayback::new(
                model.clone(),
                ModelAnimation {
                    clip: Some(usize::MAX),
                    speed: 1.0
                }
            )
            .is_err()
        );
        assert!(
            ModelPlayback::new(
                model.clone(),
                ModelAnimation {
                    clip: None,
                    speed: f32::NAN
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
