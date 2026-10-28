//! Ordered model/UI drain followed by game freeze and native audio-owner fence.
//! Presentation/input ownership and native output-device queue transfer are outer gates.
#![allow(dead_code)]
use crate::*;
use crate::{
    audio_session::checkpoint::AudioSessionCheckpoint,
    ui_live::reload_checkpoint::UiReloadCheckpoint,
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Draining,
    Audio,
    Ready,
    Releasing,
}
#[derive(Debug)]
pub(crate) struct RestoreBarrier {
    epoch: Arc<()>,
    phase: Phase,
    frozen: bool,
    models: bool,
    ui: bool,
    audio: bool,
    max_audio_bytes: usize,
    ui_snapshot: Option<UiReloadCheckpoint>,
    audio_snapshot: Option<AudioSessionCheckpoint>,
}
impl App {
    pub(crate) fn restore_audio_frozen(&self) -> bool {
        self.restore_barrier
            .as_ref()
            .is_some_and(|b| b.phase == Phase::Ready && b.audio)
    }
    pub(crate) fn restore_play_frozen(&self) -> bool {
        self.restore_barrier.as_ref().is_some_and(|b| b.frozen)
    }
    pub(crate) fn begin_restore_barrier(
        &mut self,
        max_audio_bytes: usize,
    ) -> Result<(), EditorError> {
        if max_audio_bytes == 0
            || self.restore_barrier.is_some()
            || self.resource_checkpoint.is_some()
        {
            return Err("restoration barrier already held or invalid policy".into());
        }
        // This check is read-only. Native audio must not partially acquire other fences.
        self.audio.checkpoint_admission()?;
        self.ui_live.begin_checkpoint()?;
        if let Err(error) = self.begin_model_checkpoint() {
            self.ui_live.resume_after_checkpoint()?;
            return Err(error);
        }
        self.restore_barrier = Some(RestoreBarrier {
            epoch: Arc::new(()),
            phase: Phase::Draining,
            frozen: false,
            models: true,
            ui: true,
            audio: false,
            max_audio_bytes,
            ui_snapshot: None,
            audio_snapshot: None,
        });
        Ok(())
    }
    /// Call after the normal frame's game/UI processing, or explicitly headless.
    /// Never waits for an import, watcher or GPU validation future.
    pub(crate) fn poll_restore_barrier(
        &mut self,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let phase = self
            .restore_barrier
            .as_ref()
            .ok_or("restore barrier not requested")?
            .phase;
        if phase == Phase::Releasing {
            return Ok(false);
        }
        if phase == Phase::Ready {
            return Ok(true);
        }
        if phase == Phase::Draining {
            let models = self.poll_model_checkpoint()?;
            let ui = if let Some(graphics) = &mut self.graphics {
                let size = self.window.as_ref().map_or([1., 1.], |w| {
                    let s = w.inner_size();
                    [s.width.max(1) as f32, s.height.max(1) as f32]
                });
                self.ui_live.poll_checkpoint(
                    &self.scene,
                    &self.authoring.authoring_project,
                    size,
                    graphics,
                )?
            } else {
                self.ui_live.advance_checkpoint()?
            };
            if let Some(saved) = ui {
                self.restore_barrier.as_mut().unwrap().ui_snapshot = Some(saved);
            }
            if !models || self.restore_barrier.as_ref().unwrap().ui_snapshot.is_none() {
                return Ok(false);
            }
            // The last normal game frame has adopted drained model descriptors.
            let state = self.restore_barrier.as_mut().unwrap();
            state.phase = Phase::Audio;
            state.frozen = true;
            self.audio.request_checkpoint()?;
            self.restore_barrier.as_mut().unwrap().audio = true;
        } else {
            // Audio reload completion still needs the normal owner-side poll.
            self.tick()?;
        }
        let max = self.restore_barrier.as_ref().unwrap().max_audio_bytes;
        let Some(audio) = self.audio.poll_checkpoint(identify, max)? else {
            return Ok(false);
        };
        let state = self.restore_barrier.as_mut().unwrap();
        state.audio_snapshot = Some(audio);
        state.phase = Phase::Ready;
        Ok(true)
    }
    pub(crate) fn restore_barrier_snapshots(
        &self,
    ) -> Result<(&UiReloadCheckpoint, &AudioSessionCheckpoint), EditorError> {
        let state = self
            .restore_barrier
            .as_ref()
            .filter(|b| b.phase == Phase::Ready)
            .ok_or("restore barrier is not ready")?;
        Ok((
            state
                .ui_snapshot
                .as_ref()
                .ok_or("missing UI barrier snapshot")?,
            state
                .audio_snapshot
                .as_ref()
                .ok_or("missing audio barrier snapshot")?,
        ))
    }
    /// Consume every issued watcher fence before unfreezing. Input remains owned
    /// by the existing maps/queues; this does not drain or discard queued events.
    pub(crate) fn resume_restore_barrier(&mut self) -> Result<bool, EditorError> {
        let Some(state) = self.restore_barrier.as_mut() else {
            return Ok(true);
        };
        state.phase = Phase::Releasing;
        let (models, ui, audio) = (state.models, state.ui, state.audio);
        if ui && self.ui_live.resume_after_checkpoint()? {
            self.restore_barrier.as_mut().unwrap().ui = false;
        }
        if models && self.resume_model_checkpoint()? {
            self.restore_barrier.as_mut().unwrap().models = false;
        }
        // Audio must remain held while another owner can still delay/fail release.
        // Native output may consume its queue as soon as its owner resumes.
        let state = self.restore_barrier.as_ref().unwrap();
        if state.models || state.ui {
            return Ok(false);
        }
        if audio && self.audio.resume_checkpoint()? {
            self.restore_barrier.as_mut().unwrap().audio = false;
        }
        let state = self.restore_barrier.as_ref().unwrap();
        if state.audio {
            return Ok(false);
        }
        let frozen = state.frozen;
        self.restore_barrier = None;
        // Paused wall time must not turn into catch-up ticks on cancellation.
        if frozen {
            self.play.simulation_time = Instant::now();
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests;

/// One immutable capture boundary, delegating all participant formats to their
/// native codecs. Authoring and final host publication remain separate owners.
pub(crate) struct CoordinatedPlayCheckpoint {
    pub game: crate::app::resource_checkpoint::play::PlayResourcesCheckpoint,
    pub ui: crate::ui_live::checkpoint::UiLiveCheckpoint,
    pub audio: AudioSessionCheckpoint,
    pub ui_input: crate::game_ui_input::checkpoint::GameUiInputCheckpoint,
}
impl App {
    pub(crate) fn coordinated_play_checkpoint(
        &self,
        view: &voxy_scene::SceneCheckpointView<'_>,
        registry: &voxy_scene::ComponentRegistry,
        compatibility: &str,
        resources: crate::app::resource_checkpoint::RestoreLimits,
        play: crate::play_session::checkpoint::PlayCheckpointLimits,
        ui: crate::ui_live::checkpoint::UiLiveCheckpointLimits,
    ) -> Result<CoordinatedPlayCheckpoint, EditorError> {
        // Require the completed common boundary, not merely independent model
        // and UI fences. These immutable captures cannot interleave a game step.
        let (_, audio) = self.restore_barrier_snapshots()?;
        let game = self.capture_play_resources(view, registry, compatibility, resources, play)?;
        let ui_input = self
            .game_ui_input
            .checkpoint_historical(view, ui.publications.max_layout_bytes)?;
        let ui = self.ui_live.checkpoint(view, ui)?;
        Ok(CoordinatedPlayCheckpoint {
            game,
            ui,
            audio: audio.clone(),
            ui_input,
        })
    }
}

pub(crate) struct CoordinatedPrepare<'a> {
    pub compatibility: &'a str,
    pub registry: &'a Arc<voxy_scene::ComponentRegistry>,
    pub behaviors: &'a voxy_scene::BehaviorCheckpointRegistry,
    pub ui_actions: &'a voxy_gameplay::UiActionCheckpointRegistry,
    pub root: &'a std::path::Path,
    pub resources: crate::app::resource_checkpoint::RestoreLimits,
    pub play: crate::play_session::checkpoint::PlayCheckpointLimits,
    pub ui: crate::ui_live::checkpoint::UiLiveCheckpointLimits,
    pub max_audio_bytes: usize,
    pub restored_ui_cache: (u64, u64, u64, u64),
}
/// Candidate owners remain detached. Options allow the eventual infallible host
/// publication to transfer each owner without bypassing cleanup of the others.
pub(crate) struct PreparedCoordinatedPlay {
    origin: Option<CoordinatedOrigin>,
    pub game: Option<crate::app::resource_checkpoint::play::PreparedPlayResources>,
    pub project: Option<prefab_authoring::AuthoringProject>,
    pub ui: Option<crate::ui_live::checkpoint::PreparedUiLive>,
    pub audio: Option<PreparedCoordinatedAudio>,
    pub ui_input: Option<crate::game_ui_input::GameUiInput>,
}
pub(crate) enum PreparedCoordinatedAudio {
    Offline(Box<crate::audio_session::checkpoint::PreparedAudioSession>),
    Native(Box<crate::audio_session::checkpoint::PreparedNativeAudioSession>),
}
impl PreparedCoordinatedAudio {
    fn discard(self) -> Vec<std::thread::JoinHandle<()>> {
        match self {
            Self::Offline(audio) => audio.discard(),
            Self::Native(audio) => audio.discard(),
        }
    }
}
impl PreparedCoordinatedPlay {
    fn retire(&mut self) -> Vec<std::thread::JoinHandle<()>> {
        let mut workers = Vec::new();
        if let Some(game) = &mut self.game {
            if let Some(imports) = game.resources.imports.take() {
                workers.push(imports.close());
            }
            if let Some(watcher) = game.resources.watcher.take() {
                workers.push(watcher.close());
            }
        }
        if let Some(ui) = self.ui.take() {
            workers.extend(ui.close());
        }
        if let Some(audio) = self.audio.take() {
            workers.extend(audio.discard());
        }
        workers
    }
    /// Join outside frame-critical code; dropping is nonblocking and closes IO.
    pub(crate) fn discard(mut self) -> Vec<std::thread::JoinHandle<()>> {
        self.retire()
    }
}
impl Drop for PreparedCoordinatedPlay {
    fn drop(&mut self) {
        drop(self.retire());
    }
}
impl CoordinatedPlayCheckpoint {
    pub(crate) fn prepare(
        &self,
        options: CoordinatedPrepare<'_>,
    ) -> Result<PreparedCoordinatedPlay, EditorError> {
        self.prepare_with_output(options, None, false)
    }
    fn prepare_with_output(
        &self,
        options: CoordinatedPrepare<'_>,
        output: Option<&voxy_audio_device::OutputConnection>,
        native_enabled: bool,
    ) -> Result<PreparedCoordinatedPlay, EditorError> {
        // Validate cheap participant encodings before spawning model workers.
        self.ui.to_bytes(options.ui)?;
        self.audio.to_json(options.max_audio_bytes)?;
        self.ui_input
            .to_json(options.ui.publications.max_layout_bytes)?;
        let game = self.game.prepare(
            options.compatibility,
            options.registry,
            options.behaviors,
            options.ui_actions,
            options.root,
            options.resources,
            options.play,
        )?;
        let mut candidate = PreparedCoordinatedPlay {
            origin: None,
            game: Some(game),
            project: None,
            ui: None,
            audio: None,
            ui_input: None,
        };
        let game = candidate.game.as_ref().unwrap();
        candidate.project = Some(
            prefab_authoring::AuthoringProject::with_registry(
                options.root,
                &game.resources.recipe,
                options.registry.clone(),
            )
            .map_err(|e| e.to_string())?,
        );
        let project = candidate.project.as_ref().unwrap();
        let scene = &game.game.scene;
        candidate.ui_input = Some(self.ui_input.prepare_historical(
            &scene.checkpoint_view(),
            options.ui.publications.max_elements,
            options.ui.publications.max_layout_bytes,
        )?);
        candidate.ui = Some(self.ui.prepare(
            &scene.checkpoint_view(),
            project,
            options.restored_ui_cache,
            options.ui,
        )?);
        candidate.audio = Some(match output {
            Some(output) if !native_enabled => PreparedCoordinatedAudio::Native(Box::new(
                self.audio.prepare_offline_replacing_native(
                    &scene.graph,
                    project,
                    |id| scene.resolve(id),
                    options.max_audio_bytes,
                    output,
                )?,
            )),
            Some(output) => PreparedCoordinatedAudio::Native(Box::new(self.audio.prepare_native(
                &scene.graph,
                project,
                |id| scene.resolve(id),
                options.max_audio_bytes,
                output,
            )?)),
            None => PreparedCoordinatedAudio::Offline(Box::new(self.audio.prepare(
                &scene.graph,
                project,
                |id| scene.resolve(id),
                options.max_audio_bytes,
            )?)),
        });
        Ok(candidate)
    }
}

/// Freshness of the source session, separate from the archived target game.
pub(crate) struct CoordinatedOrigin {
    epoch: Arc<()>,
    game: Arc<crate::app::resource_checkpoint::play::PlayResourceGuard>,
    ui: blake3::Hash,
    audio: blake3::Hash,
    ui_limits: crate::ui_live::checkpoint::UiLiveCheckpointLimits,
    authoring_source: Option<blake3::Hash>,
    authoring_source_bytes: usize,
}
impl App {
    fn coordinated_outer_fingerprint(
        &mut self,
        bindings: &[(ObjectId, NodeId)],
        ui_limits: crate::ui_live::checkpoint::UiLiveCheckpointLimits,
    ) -> Result<(blake3::Hash, blake3::Hash), EditorError> {
        self.restore_barrier_snapshots()?;
        let max = self.restore_barrier.as_ref().unwrap().max_audio_bytes;
        // Observe the current native owner, not the cached snapshot captured
        // when the barrier first became ready. Pending edits must invalidate.
        let audio = self
            .audio
            .poll_checkpoint(
                |node| {
                    bindings
                        .iter()
                        .find(|(_, n)| *n == node)
                        .map(|(id, _)| id.clone())
                },
                max,
            )?
            .ok_or("audio checkpoint is no longer ready")?;
        let mut audio_hash = blake3::Hasher::new();
        audio_hash.update(audio.to_json(max)?.as_bytes());
        // Output policy is unsaved host state. Even an idle mode switch must
        // invalidate an outstanding candidate without starting a device.
        audio_hash.update(&[match self.audio.output_mode() {
            audio_session::AudioOutputMode::Offline => 0,
            audio_session::AudioOutputMode::Native => 1,
        }]);
        let audio = audio_hash.finalize();
        let view = voxy_scene::SceneCheckpointView::new(&self.scene, bindings.iter().cloned())?;
        let ui = blake3::hash(
            &self
                .ui_live
                .checkpoint(&view, ui_limits)?
                .to_bytes(ui_limits)?,
        );
        let input = self
            .game_ui_input
            .checkpoint_historical(&view, ui_limits.publications.max_layout_bytes)?;
        let input = blake3::hash(
            input
                .to_json(ui_limits.publications.max_layout_bytes)?
                .as_bytes(),
        );
        let mut combined = blake3::Hasher::new();
        combined.update(ui.as_bytes());
        combined.update(input.as_bytes());
        Ok((combined.finalize(), audio))
    }
    pub(crate) fn observe_coordinated_origin(
        &mut self,
        bindings: &[(ObjectId, NodeId)],
        registry: &voxy_scene::ComponentRegistry,
        compatibility: &str,
        resources: crate::app::resource_checkpoint::RestoreLimits,
        play: crate::play_session::checkpoint::PlayCheckpointLimits,
        ui_limits: crate::ui_live::checkpoint::UiLiveCheckpointLimits,
    ) -> Result<CoordinatedOrigin, EditorError> {
        let (ui, audio) = self.coordinated_outer_fingerprint(bindings, ui_limits)?;
        let view = voxy_scene::SceneCheckpointView::new(&self.scene, bindings.iter().cloned())?;
        let game =
            self.observe_play_restore_origin(&view, registry, compatibility, resources, play)?;
        let authoring_source = self.authoring_source_fingerprint(resources.max_bytes)?;
        Ok(CoordinatedOrigin {
            epoch: self.restore_barrier.as_ref().unwrap().epoch.clone(),
            game: Arc::new(game),
            ui,
            audio,
            ui_limits,
            authoring_source,
            authoring_source_bytes: resources.max_bytes,
        })
    }
}
impl CoordinatedOrigin {
    pub(crate) fn validate(
        &self,
        app: &mut App,
        bindings: &[(ObjectId, NodeId)],
        registry: &voxy_scene::ComponentRegistry,
        compatibility: &str,
    ) -> Result<(), EditorError> {
        if !app
            .restore_barrier
            .as_ref()
            .is_some_and(|b| Arc::ptr_eq(&b.epoch, &self.epoch))
        {
            return Err("coordinated restore barrier epoch changed".into());
        }
        if app.authoring_source_fingerprint(self.authoring_source_bytes)? != self.authoring_source {
            return Err("imported authoring source changed during restore preparation".into());
        }
        let (ui, audio) = app.coordinated_outer_fingerprint(bindings, self.ui_limits)?;
        if ui != self.ui || audio != self.audio {
            return Err("UI or audio source changed during restore preparation".into());
        }
        let view = voxy_scene::SceneCheckpointView::new(&app.scene, bindings.iter().cloned())?;
        self.game.validate(app, &view, registry, compatibility)
    }
}

impl CoordinatedPlayCheckpoint {
    pub(crate) fn prepare_guarded(
        &self,
        options: CoordinatedPrepare<'_>,
        origin: CoordinatedOrigin,
    ) -> Result<PreparedCoordinatedPlay, EditorError> {
        let mut candidate = self.prepare(options)?;
        candidate.origin = Some(origin);
        Ok(candidate)
    }
}
impl PreparedCoordinatedPlay {
    pub(crate) fn validate_origin(
        &self,
        app: &mut App,
        bindings: &[(ObjectId, NodeId)],
        registry: &voxy_scene::ComponentRegistry,
        compatibility: &str,
    ) -> Result<(), EditorError> {
        self.origin
            .as_ref()
            .ok_or("candidate has no source observation")?
            .validate(app, bindings, registry, compatibility)
    }
}

pub(crate) mod file;

pub(crate) mod editor;
pub(crate) mod load;
pub(crate) mod save;

/// Detached outer owners stay together while the original game uploads are pending.
pub(crate) struct PendingCoordinatedPlay {
    outer: PreparedCoordinatedPlay,
    game: crate::app::resource_checkpoint::play::PendingPlayResources,
}
pub(crate) struct ReadyCoordinatedPlay {
    pub outer: PreparedCoordinatedPlay,
    pub game: crate::app::resource_checkpoint::play::ReadyPlayResources,
}
impl PreparedCoordinatedPlay {
    pub(crate) fn stage_gpu(
        mut self,
        target: crate::gpu_model::batch::UploadTarget<'_>,
    ) -> Result<PendingCoordinatedPlay, EditorError> {
        let origin = self
            .origin
            .as_ref()
            .ok_or("candidate has no source observation")?;
        if self.project.is_none()
            || self.ui.is_none()
            || self.audio.is_none()
            || self.ui_input.is_none()
        {
            return Err("incomplete coordinated restore candidate".into());
        }
        let guard = origin.game.clone();
        let game = self.game.take().ok_or("missing prepared game")?;
        let game = game.stage_gpu_shared(target, guard)?;
        Ok(PendingCoordinatedPlay { outer: self, game })
    }
}
impl PendingCoordinatedPlay {
    pub(crate) async fn validate(self) -> Result<ReadyCoordinatedPlay, EditorError> {
        let game = self.game.validate().await?;
        Ok(ReadyCoordinatedPlay {
            outer: self.outer,
            game,
        })
    }
}
impl ReadyCoordinatedPlay {
    /// Check all original source owners and destination GPU immediately before commit.
    pub(crate) fn validate_origin(
        &self,
        app: &mut App,
        bindings: &[(ObjectId, NodeId)],
        registry: &voxy_scene::ComponentRegistry,
        compatibility: &str,
        target: &crate::gpu_model::batch::UploadTarget<'_>,
    ) -> Result<(), EditorError> {
        self.outer
            .validate_origin(app, bindings, registry, compatibility)?;
        self.game
            .gpu
            .validate_destination(&self.game.game.scene.graph, target)?;
        Ok(())
    }
}

fn retire_staged_models(
    resources: &mut crate::app::resource_checkpoint::PreparedModelResources,
) -> Vec<std::thread::JoinHandle<()>> {
    let mut workers = Vec::new();
    if let Some(imports) = resources.imports.take() {
        workers.push(imports.close());
    }
    if let Some(watcher) = resources.watcher.take() {
        workers.push(watcher.close());
    }
    workers
}
impl PendingCoordinatedPlay {
    pub(crate) fn discard(self) -> Vec<std::thread::JoinHandle<()>> {
        let mut workers = self.game.discard();
        workers.extend(self.outer.discard());
        workers
    }
}
impl ReadyCoordinatedPlay {
    pub(crate) fn discard(mut self) -> Vec<std::thread::JoinHandle<()>> {
        let mut workers = retire_staged_models(&mut self.game.resources);
        workers.extend(self.outer.discard());
        workers
    }
}

mod policy;
