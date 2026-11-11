//! Runtime clip changes retain their CPU playback across GPU replacement.
use super::{
    ClipOverride, DOCUMENT_BYTES, ProjectGame, ProjectPlayback, SampledProjectPose,
    animation_phase_at_tick, clip_event_track, error,
};
use voxy_model::ModelAnimation;
use wasm_bindgen::JsValue;

impl ProjectGame {
    pub(crate) fn transition_animation(
        &mut self,
        identity: &str,
        json: &str,
    ) -> Result<(), JsValue> {
        if json.len() > DOCUMENT_BYTES {
            return Err(error("animation settings exceed document budget"));
        }
        let mut settings: ModelAnimation = serde_json::from_str(json).map_err(error)?;
        let owner = self
            .loaded
            .resolve(&voxy_scene::ObjectId(identity.to_owned()))
            .ok_or_else(|| error("unknown animation owner"))?;
        let instance = self
            .graphics
            .instances
            .iter()
            .find(|instance| instance.owner == owner)
            .ok_or_else(|| error("animation owner has no model instance"))?;
        let playback = instance
            .playback
            .as_ref()
            .ok_or_else(|| error("animation owner has no animated model"))?;
        let previous = self.clip_overrides.get(&owner);
        let retarget = previous
            .and_then(|state| state.retarget.as_ref())
            .or(playback.retarget.as_ref())
            .cloned();
        let display_model = retarget
            .as_ref()
            .map(|profile| &profile.target)
            .or(self.graphics.models[&instance.asset].animation.as_ref())
            .ok_or_else(|| error("missing displayed animation model"))?
            .clone();
        let model = previous
            .map(|state| &state.model)
            .or(retarget.as_ref().map(|profile| &profile.source))
            .unwrap_or(&display_model)
            .clone();
        settings
            .validate(
                Some(model.animations.len()),
                Some(model.skeleton.joints().len()),
            )
            .map_err(error)?;
        let next = settings
            .resolve_clip(&model)
            .map_err(error)?
            .ok_or_else(|| error("transition requires a target clip"))?;
        settings.clip = Some(next);
        let root_basis = super::root_motion::root_basis(
            &self.loaded.graph,
            owner,
            &model,
            &settings,
            retarget.as_ref(),
        )?;
        let source_settings = previous.map_or(&playback.settings, |state| &state.settings);
        let source_basis = previous.map_or(playback.root_basis, |state| state.root_basis);
        let root_joint = if root_basis.is_some() {
            Some(settings.resolve_motion_joint(&model).map_err(error)?)
        } else {
            None
        };
        let source_root_joint = if source_basis.is_some() {
            Some(
                source_settings
                    .resolve_motion_joint(&model)
                    .map_err(error)?,
            )
        } else {
            None
        };
        let same_extraction = root_basis == source_basis
            && settings.root_motion_rotation == source_settings.root_motion_rotation
            && settings.root_motion_axes == source_settings.root_motion_axes
            && root_joint == source_root_joint;
        let root_reference = previous
            .filter(|_| same_extraction)
            .and_then(|state| state.root_reference);
        if settings.transition_seconds > 0.0 && !same_extraction {
            return Err(error(
                "root motion transition requires the same extraction axes, joint and parent basis",
            ));
        }
        if settings.transition_seconds > 0.0
            && settings.root_motion_rotation
            && root_reference.is_none()
        {
            return Err(error(
                "rotational fade requires a physically accepted root reference",
            ));
        }
        let target = model
            .animations
            .get(next)
            .ok_or_else(|| error("invalid target clip"))?;
        let mut animator = if let Some(state) = previous {
            state.animator.clone()
        } else if let Some(initial) = playback.settings.clip {
            let clip = &model.animations[initial];
            let mut animator = model.animator(initial).map_err(error)?;
            let phase = animation_phase_at_tick(
                self.animation_ticks,
                f64::from(playback.settings.speed),
                f64::from(clip.duration()),
            )
            .fract();
            animator
                .transition_to_at_phase(clip.clone(), 0.0, phase)
                .map_err(error)?;
            animator
        } else if settings.transition_seconds == 0.0 {
            model.animator(next).map_err(error)?
        } else {
            return Err(error("bind pose has no playing transition source"));
        };
        animator
            .transition_to(target.clone(), settings.transition_seconds)
            .map_err(error)?;
        animator.set_speed(settings.speed).map_err(error)?;
        if let Some(joint) = root_joint {
            animator.set_root_motion_joint(joint).map_err(error)?;
        }
        let frame = animator.advance_wall(&model.skeleton, 0.0).map_err(error)?;
        let (mut frame, _) =
            super::root_motion::display_frame(frame, &model, &settings, retarget.as_ref())
                .map_err(error)?;
        settings
            .apply_morph_weights(&display_model, &mut frame)
            .map_err(error)?;
        let pose = if settings.transition_seconds > 0.0
            && settings.morph_weights == source_settings.morph_weights
        {
            // Preserve the admitted source exactly at the fade boundary. Blending
            // at zero can still renormalize a quaternion and change raster pixels.
            if let Some(state) = previous {
                state.pose.clone()
            } else {
                let source = ProjectPlayback::sample(
                    &display_model,
                    source_settings,
                    self.animation_ticks,
                    retarget.as_ref(),
                )?;
                SampledProjectPose::from_frame(&display_model, source, self.animation_ticks)
                    .map_err(error)?
            }
        } else {
            SampledProjectPose::from_frame(&display_model, frame, self.animation_ticks)
                .map_err(error)?
        };
        if let Some(skinner) = &self.graphics.skinner {
            for (skin, weights) in playback.skins.iter().zip(&pose.weights) {
                if let Some(skin) = skin {
                    if weights.is_empty() {
                        skinner.prepare_pose(skin, &pose.joints).map_err(error)?;
                    } else {
                        skinner
                            .prepare_morph_pose(skin, &pose.joints, weights)
                            .map_err(error)?;
                    }
                }
            }
        }
        let track = (!settings.events.is_empty())
            .then(|| clip_event_track(&settings, target))
            .transpose()
            .map_err(error)?;
        self.clip_overrides.insert(
            owner,
            ClipOverride {
                model,
                retarget,
                animator,
                settings,
                identity: identity.to_owned(),
                track,
                pose,
                root_basis,
                root_reference,
            },
        );
        self.graphics.last_skin_tick = None;
        Ok(())
    }
}
