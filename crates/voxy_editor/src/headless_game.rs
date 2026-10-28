//! Embeddable game sessions use the same app owner and input adapters as native Play.
use crate::{App, AudioOutputMode, GameHost};
use std::path::Path;
use winit::{event::ElementState, keyboard::KeyCode};

/// Accepted animation state from the same immutable frame used by rendering.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct AnimationSnapshot {
    pub clip_name: String,
    pub phase: f64,
    pub transition_weight: f32,
    pub skin_matrices: Vec<[f32; 16]>,
    pub morph_weights: Vec<Vec<f32>>,
}
/// Window-free simulation with offline PCM mixing. This does not measure render FPS.
/// The package storage stays alive until the app's workers have stopped.
pub struct HeadlessGame {
    app: App,
    _package: Option<crate::packaged_game::MaterializedPackage>,
}
impl std::fmt::Debug for HeadlessGame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeadlessGame")
            .field("fixed_ticks", &self.fixed_ticks())
            .finish_non_exhaustive()
    }
}
impl HeadlessGame {
    /// Start a saved project using its normal imports and fixed-step game lifecycle.
    /// # Errors
    /// Reports invalid projects, missing components/assets and host setup failures.
    pub fn open_project(path: &Path, host: GameHost) -> Result<Self, Box<dyn std::error::Error>> {
        let mut app = App::from_project_with_registry(path, host.components)?;
        app.behavior_setup = host.behaviors;
        app.behavior_checkpoints = host.behavior_checkpoints;
        app.ui_action_checkpoints = host.ui_action_checkpoints;
        app.ui_action_setup = host.ui_actions;
        app.audio.set_output_mode(AudioOutputMode::Offline);
        app.start_standalone()?;
        Ok(Self {
            app,
            _package: None,
        })
    }
    /// Start a verified project package without access to its original source tree.
    /// # Errors
    /// Reports package corruption, incompatible launch data and game setup errors.
    pub fn open_package(path: &Path, host: GameHost) -> Result<Self, Box<dyn std::error::Error>> {
        let package = crate::packaged_game::MaterializedPackage::open(path)?;
        let mut session = Self::open_project(&package.project_path()?, host)?;
        session._package = Some(package);
        Ok(session)
    }
    /// Capture opted-in Rust behaviors using this session's durable object IDs.
    /// This does not capture scene/clock/physics or other participants.
    /// # Errors
    /// Rejects unsupported behavior types, missing identities and byte limits.
    pub fn behavior_checkpoint(
        &self,
        max_bytes: usize,
    ) -> Result<voxy_scene::BehaviorCheckpoint, Box<dyn std::error::Error>> {
        let simulation = self
            .app
            .play
            .simulation
            .as_ref()
            .ok_or("game simulation is not running")?;
        let identities: std::collections::HashMap<_, _> = self
            .app
            .instances
            .iter()
            .copied()
            .zip(self.app.object_ids.iter().cloned())
            .collect();
        Ok(simulation.behavior_checkpoint(
            &self.app.scene,
            |node| identities.get(&node).cloned(),
            max_bytes,
        )?)
    }
    /// Restore the behavior participant with the host's executable codecs into
    /// an empty runner. Stage a new session for whole-game load; live behaviors
    /// are deliberately not replaced by this low-level operation.
    /// # Errors
    /// Rejects nonempty runners, unknown codecs, invalid data/owners and budgets.
    pub fn restore_behavior_checkpoint(
        &mut self,
        saved: &voxy_scene::BehaviorCheckpoint,
        max_bytes: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let identities: std::collections::HashMap<_, _> = self
            .app
            .object_ids
            .iter()
            .cloned()
            .zip(self.app.instances.iter().copied())
            .collect();
        let simulation = self
            .app
            .play
            .simulation
            .as_mut()
            .ok_or("game simulation is not running")?;
        simulation.restore_behavior_checkpoint(
            &self.app.scene,
            saved,
            &self.app.behavior_checkpoints,
            |id| identities.get(id).copied(),
            max_bytes,
        )?;
        Ok(())
    }

    /// Capture opted-in callback state and queued UI events. Scene, pointer/focus,
    /// commands and other owners belong to the enclosing whole-game transaction.
    pub fn ui_action_checkpoint(
        &self,
        limits: voxy_gameplay::UiActionCheckpointLimits,
    ) -> Result<Option<voxy_gameplay::UiActionCheckpoint>, Box<dyn std::error::Error>> {
        let identities: std::collections::HashMap<_, _> = self
            .app
            .instances
            .iter()
            .copied()
            .zip(self.app.object_ids.iter().cloned())
            .collect();
        self.app
            .play
            .ui_actions
            .as_ref()
            .map(|actions| {
                actions
                    .checkpoint(
                        &self.app.scene,
                        |node| identities.get(&node).cloned(),
                        limits,
                    )
                    .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)
            })
            .transpose()
    }
    /// Stage new callbacks using the host codecs, then replace only this participant.
    /// Errors preserve current callbacks and queued events. This does not load a game.
    pub fn restore_ui_action_checkpoint(
        &mut self,
        saved: &voxy_gameplay::UiActionCheckpoint,
        limits: voxy_gameplay::UiActionCheckpointLimits,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.app.play.simulation.is_none() {
            return Err("game simulation is not running".into());
        }
        let identities: std::collections::HashMap<_, _> = self
            .app
            .object_ids
            .iter()
            .cloned()
            .zip(self.app.instances.iter().copied())
            .collect();
        let candidate = self.app.ui_action_checkpoints.prepare(
            &self.app.scene,
            saved,
            |id| identities.get(id).copied(),
            limits,
        )?;
        self.app.play.ui_actions = Some(candidate);
        Ok(())
    }

    /// Capture the gameplay input participant, including pending presses/releases.
    /// UI pointer capture/focus and other game state are separate participants.
    #[must_use]
    pub fn input_checkpoint(&self) -> voxy_input::InputCheckpoint {
        self.app.play.player_input.checkpoint()
    }
    /// Restore only gameplay input; this is not a whole-session load operation.
    /// Physical control identifiers require a compatible platform adapter.
    /// # Errors
    /// Rejects corrupt data or capacity failures without changing current input.
    pub fn restore_input_checkpoint(
        &mut self,
        saved: &voxy_input::InputCheckpoint,
    ) -> Result<(), voxy_input::InputError> {
        self.app.play.player_input.restore_checkpoint(saved)
    }

    /// Deliver a key through game UI focus first, then through the gameplay map.
    /// Releases always clear previous gameplay bindings, matching native input.
    /// # Errors
    /// Reports invalid input bindings or failed UI action admission.
    pub fn key(&mut self, key: KeyCode, pressed: bool) -> Result<(), Box<dyn std::error::Error>> {
        let state = if pressed {
            ElementState::Pressed
        } else {
            ElementState::Released
        };
        let consumed = self.app.game_ui_key(key, state, false)?;
        if !consumed || !pressed {
            self.app.game_key(key, state)?;
        }
        Ok(())
    }
    /// Deliver a UI pointer edge in logical viewport coordinates.
    /// # Errors
    /// Rejects nonfinite coordinates, invalid viewport sizes and UI action errors.
    pub fn pointer(
        &mut self,
        point: [f32; 2],
        viewport: [f32; 2],
        pressed: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if point.iter().chain(viewport.iter()).any(|v| !v.is_finite())
            || viewport.iter().any(|v| *v <= 0.)
        {
            return Err("invalid headless pointer coordinates".into());
        }
        self.app
            .game_ui_pointer(point.into(), viewport.into(), pressed)
    }
    /// Losing focus releases gameplay input and cancels incomplete UI activation.
    pub fn set_focused(&mut self, focused: bool) {
        self.app.play.player_input.set_focused(focused);
        if !focused {
            self.app.game_ui_input.cancel();
        }
    }
    /// Advance through the existing bounded fixed-step and animation schedule.
    /// # Errors
    /// Rejects invalid elapsed time and reports simulation/import/runtime failures.
    pub fn advance(&mut self, elapsed: f64) -> Result<(), Box<dyn std::error::Error>> {
        self.app.advance_game(elapsed)
    }
    /// Capture the current registered components and transforms using durable IDs.
    /// # Errors
    /// Reports component serialization or invalid scene references.
    pub fn snapshot(&self) -> Result<voxy_scene::SceneDocument, Box<dyn std::error::Error>> {
        Ok(self.app.authoring_document()?)
    }
    /// Inspect an accepted skeletal frame by durable scene object ID.
    /// # Errors
    /// Reports unknown owners, invalid scene handles or unavailable model imports.
    pub fn animation_snapshot(
        &self,
        id: &voxy_scene::ObjectId,
    ) -> Result<Option<AnimationSnapshot>, Box<dyn std::error::Error>> {
        let owner = self
            .app
            .object_ids
            .iter()
            .zip(&self.app.instances)
            .find_map(|(candidate, node)| (candidate == id).then_some(*node))
            .ok_or("unknown animation owner")?;
        let instance = self
            .app
            .scene
            .component::<crate::ModelInstance>(owner)?
            .ok_or("animation owner has no model")?;
        let asset = self
            .app
            .catalog
            .snapshot(&instance.asset)
            .ok_or("animation model is unavailable")?;
        let Some(model) = asset.value().animated.as_ref() else {
            return Ok(None);
        };
        let Some(frame) = self.app.play.animations.frame(owner, model) else {
            return Ok(None);
        };
        let Some(name) = self.app.play.animations.clip_name(owner) else {
            return Ok(None);
        };
        let Some(phase) = self.app.play.animations.clip_phase(owner) else {
            return Ok(None);
        };
        Ok(Some(AnimationSnapshot {
            clip_name: name.into(),
            phase,
            transition_weight: frame.transition_weight,
            morph_weights: model.frame_weights(&frame)?,
            skin_matrices: frame
                .skin_matrices
                .iter()
                .map(glam::Mat4::to_cols_array)
                .collect(),
        }))
    }
    /// Current presentation view, including follow smoothing of interpolated poses.
    #[must_use]
    pub fn presentation_camera(&self) -> crate::EditorCamera {
        self.app.camera.clone()
    }
    #[must_use]
    pub fn fixed_ticks(&self) -> u64 {
        self.app.play.simulation_ticks
    }
    /// Offline mixed frame count and peak, when the scene has an audio session.
    #[must_use]
    pub fn audio_stats(&self) -> Option<(u64, f32)> {
        self.app
            .audio
            .play()
            .map(|audio| (audio.frames, audio.peak))
    }
    /// Stop behaviors and all resource workers before releasing package storage.
    /// # Errors
    /// Reports lifecycle, audio and worker shutdown failures.
    pub fn stop(mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.app.stop_workers()
    }
}

#[cfg(test)]
#[path = "headless_game_tests.rs"]
mod tests;
