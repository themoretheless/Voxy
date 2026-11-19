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
/// Scene and character controller state captured at the same fixed tick.
#[derive(Clone, Debug)]
pub struct CharacterReplicationSnapshot {
    pub scene: voxy_scene::TickSnapshot,
    pub physics: Option<voxy_gameplay::CharacterPhysicsCheckpoint>,
    pub input: voxy_input::InputCheckpoint,
}

impl CharacterReplicationSnapshot {
    pub const HEADER: [u8; 5] = *b"VXSC\x01";

    /// Writes the scene frame followed by a bounded controller/input payload.
    ///
    /// # Errors
    /// Reports invalid state, payload budget or stream failures.
    pub fn write_frame(
        &self,
        writer: &mut impl std::io::Write,
        max_bytes: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let physics = self
            .physics
            .as_ref()
            .map(|value| value.to_json(max_bytes))
            .transpose()?;
        let input = voxy_gameplay::encode_input_checkpoint(&self.input, max_bytes, 32, 64)?;
        let payload = serde_json::to_vec(&(physics, input))?;
        if payload.len() > max_bytes {
            return Err("character replication payload exceeds byte budget".into());
        }
        let length = u32::try_from(payload.len())?;
        self.scene.write_frame(writer, max_bytes)?;
        writer.write_all(&length.to_be_bytes())?;
        writer.write_all(&payload)?;
        Ok(())
    }

    /// Reads and validates both frames into a detached snapshot.
    ///
    /// # Errors
    /// Rejects incomplete or invalid state and oversized payloads.
    pub fn read_frame(
        reader: &mut impl std::io::Read,
        max_bytes: usize,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let scene = voxy_scene::TickSnapshot::read_frame(reader, max_bytes)?;
        let mut length = [0; 4];
        reader.read_exact(&mut length)?;
        let length = u32::from_be_bytes(length) as usize;
        if length == 0 || length > max_bytes {
            return Err("invalid character replication payload length".into());
        }
        let mut payload = Vec::new();
        payload.try_reserve_exact(length)?;
        payload.resize(length, 0);
        reader.read_exact(&mut payload)?;
        let (physics, input): (Option<String>, String) = serde_json::from_slice(&payload)?;
        let physics = physics
            .as_deref()
            .map(|text| voxy_gameplay::CharacterPhysicsCheckpoint::from_json(text, max_bytes, 128))
            .transpose()?;
        let input = voxy_gameplay::decode_input_checkpoint(&input, max_bytes, 32, 64)?;
        Ok(Self {
            scene,
            physics,
            input,
        })
    }

    /// Requests the local server's controller snapshot and its applied input acknowledgement.
    ///
    /// # Errors
    /// Rejects malformed acknowledgements and snapshot or stream failures.
    pub fn request(
        stream: &mut (impl std::io::Read + std::io::Write),
        max_bytes: usize,
    ) -> Result<(u64, u64, Self), Box<dyn std::error::Error>> {
        stream.write_all(&Self::HEADER)?;
        stream.flush()?;
        let mut header = [0; 5];
        stream.read_exact(&mut header)?;
        if header != Self::HEADER {
            return Err("unknown character replication reply".into());
        }
        let mut sequence = [0; 8];
        let mut tick = [0; 8];
        stream.read_exact(&mut sequence)?;
        stream.read_exact(&mut tick)?;
        let sequence = u64::from_be_bytes(sequence);
        let tick = u64::from_be_bytes(tick);
        let snapshot = Self::read_frame(stream, max_bytes)?;
        if tick > snapshot.scene.tick || (sequence == 0) != (tick == 0) {
            return Err("invalid character input acknowledgement".into());
        }
        Ok((sequence, tick, snapshot))
    }

    /// Replays one local-server input pulse or idle step per entry on a detached
    /// scene, using the saved character controller and the static collision world.
    ///
    /// # Errors
    /// Reports invalid scene/controller/input state, unsupported keys and budget failures.
    pub fn replay_character_inputs(
        &self,
        registry: &voxy_scene::ComponentRegistry,
        capacity: usize,
        steps: &[Option<KeyCode>],
        max_steps: usize,
    ) -> Result<voxy_scene::LoadedScene, Box<dyn std::error::Error>> {
        if steps.len() > max_steps {
            return Err("character replay exceeds step budget".into());
        }
        let mut scene = self.scene.document.load(registry, capacity)?;
        let checkpoint = self.physics.as_ref().ok_or("missing character state")?;
        let mut physics = voxy_gameplay::CharacterPhysics::new(&scene.graph, 128, 128);
        physics.restore_checkpoint(&scene.graph, checkpoint, |id| scene.resolve(id))?;
        let mut input = voxy_gameplay::player_input()?;
        input.restore_checkpoint(&self.input)?;
        for &step in steps {
            let control = match step {
                None => None,
                Some(key @ (KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Space)) => {
                    crate::game_control(key)
                }
                Some(_) => return Err("unsupported character replay input".into()),
            };
            if let Some(control) = control {
                input.event(control, 1.0)?;
            }
            physics.fixed_step(&mut scene.graph, &mut input, 1.0 / 60.0)?;
            if let Some(control) = control {
                input.event(control, 0.0)?;
            }
        }
        Ok(scene)
    }
}

/// Window-free simulation with offline PCM mixing. This does not measure render FPS.
pub struct HeadlessGame {
    app: App,
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
        Self::open(path, host, None)
    }

    fn open(
        path: &Path,
        host: GameHost,
        model_package: Option<std::sync::Arc<voxy_assets::ResourcePackage>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut app = match model_package {
            Some(archive) => App::from_packaged_project(path, host.components, archive)?,
            None => App::from_project_with_model_package(path, host.components, None)?,
        };
        app.behavior_setup = host.behaviors;
        app.behavior_checkpoints = host.behavior_checkpoints;
        app.ui_action_checkpoints = host.ui_action_checkpoints;
        app.ui_action_setup = host.ui_actions;
        app.audio.set_output_mode(AudioOutputMode::Offline);
        app.start_standalone()?;
        Ok(Self { app })
    }
    /// Start a verified project package without access to its original source tree.
    /// # Errors
    /// Reports package corruption, incompatible launch data and game setup errors.
    pub fn open_package(path: &Path, host: GameHost) -> Result<Self, Box<dyn std::error::Error>> {
        let archive = crate::packaged_game::read_project_archive(path)?;
        Self::open(path, host, Some(archive))
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
        self.app.set_game_focus(focused);
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

    /// Captures the authoritative scene at the last completed fixed tick.
    ///
    /// # Errors
    /// Reports component serialization, invalid transforms or invalid scene references.
    pub fn replication_snapshot(
        &self,
    ) -> Result<voxy_scene::TickSnapshot, Box<dyn std::error::Error>> {
        Ok(voxy_scene::TickSnapshot::from_document(
            self.fixed_ticks(),
            self.snapshot()?,
        )?)
    }
    /// Captures the character controller and input alongside the accepted scene.
    ///
    /// # Errors
    /// Reports invalid scene data or a character without a durable object ID.
    pub fn character_replication_snapshot(
        &self,
    ) -> Result<CharacterReplicationSnapshot, Box<dyn std::error::Error>> {
        let physics = self
            .app
            .play
            .physics
            .as_ref()
            .map(|physics| {
                physics.checkpoint(&self.app.scene, |node| {
                    self.app
                        .instances
                        .iter()
                        .position(|candidate| *candidate == node)
                        .map(|index| self.app.object_ids[index].clone())
                })
            })
            .transpose()?;
        Ok(CharacterReplicationSnapshot {
            scene: self.replication_snapshot()?,
            physics,
            input: self.input_checkpoint(),
        })
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
