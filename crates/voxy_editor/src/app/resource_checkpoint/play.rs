//! Full native PlaySession and model resources share one original animation runtime.
//! Authoring history, UI resources, audio and final App publication are outer owners.
#![allow(dead_code)]
use super::*;
use crate::play_session::checkpoint::{
    PlayCheckpointLimits, PlaySessionCheckpoint, PreparedPlaySession,
};
use voxy_gameplay::UiActionCheckpointRegistry;
use voxy_scene::{BehaviorCheckpointRegistry, ComponentRegistry, SceneCheckpointView};
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlayResourcesCheckpoint {
    version: u32,
    resources: ModelResourcesCheckpoint,
    play: PlaySessionCheckpoint,
    gpu: Option<crate::gpu_source_bindings::GpuSourceBindings>,
    accepted_frames: Option<crate::animated_models::checkpoint::AcceptedFrames>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    shared_frames: Vec<(voxy_scene::ObjectId, voxy_scene::ObjectId)>,
}
pub(crate) struct PreparedPlayResources {
    pub resources: PreparedModelResources,
    pub game: PreparedPlaySession,
    pub gpu: Option<crate::gpu_source_bindings::PreparedBindings>,
    pub accepted_frames:
        Option<BTreeMap<NodeId, crate::animated_models::checkpoint::PreparedFrame>>,
}
pub(crate) struct PlayGpuSources {
    pub live: crate::gpu_source_bindings::LiveSources,
    pub accepted: crate::animated_models::checkpoint::AcceptedFrames,
    pub shared_frames: Vec<(voxy_scene::ObjectId, voxy_scene::ObjectId)>,
    pub preview_shared_gpu_owner: Option<voxy_scene::ObjectId>,
}
impl App {
    /// Original Arc identities determine joins; equal values alone never do.
    pub(crate) fn shared_render_frame_roles(
        &self,
        animated: &crate::animated_models::AnimatedModels,
        view: &SceneCheckpointView<'_>,
    ) -> Result<Vec<(voxy_scene::ObjectId, voxy_scene::ObjectId)>, EditorError> {
        let gpu = animated.source_publication_owners()?;
        let mut joins = Vec::new();
        for (runtime, frame) in self.play.animations.checkpoint_frame_roots() {
            if let Some((render, _)) = gpu.iter().find(|(owner, _)| {
                animated
                    .accepted_frame(*owner)
                    .is_some_and(|(_, accepted)| Arc::ptr_eq(accepted, &frame))
            }) {
                joins.push((
                    view.identity(runtime)
                        .cloned()
                        .ok_or("unidentified shared runtime frame")?,
                    view.identity(*render)
                        .cloned()
                        .ok_or("unidentified shared GPU frame")?,
                ));
            }
        }
        Ok(joins)
    }
    pub(crate) fn capture_play_resources(
        &self,
        view: &SceneCheckpointView<'_>,
        registry: &ComponentRegistry,
        compatibility: &str,
        resources: RestoreLimits,
        play_limits: PlayCheckpointLimits,
    ) -> Result<PlayResourcesCheckpoint, EditorError> {
        self.require_model_checkpoint()?;
        if !std::ptr::eq(view.graph, &raw const self.scene) {
            return Err("Play resource view belongs to another scene".into());
        }
        let gpu = self
            .gpu_publication_roots()?
            .map(|live| {
                let graphics = self
                    .graphics
                    .as_ref()
                    .ok_or("Play GPU graphics disappeared")?;
                Ok::<_, EditorError>(PlayGpuSources {
                    shared_frames: self
                        .shared_render_frame_roles(&graphics.animated_models, view)?,
                    preview_shared_gpu_owner: self
                        .preview_render_frame_role(&graphics.animated_models, |node| {
                            view.identity(node).cloned()
                        })?,
                    live,
                    accepted: graphics.animated_models.checkpoint_accepted_frames(
                        |node| view.identity(node).cloned(),
                        play_limits.core.max_bytes,
                    )?,
                })
            })
            .transpose()?;
        self.capture_play_resources_with_gpu(
            view,
            registry,
            compatibility,
            resources,
            play_limits,
            gpu,
        )
    }
    /// Same native capture path with explicit accepted offscreen/host renderer owners.
    pub(crate) fn capture_play_resources_with_gpu(
        &self,
        view: &SceneCheckpointView<'_>,
        registry: &ComponentRegistry,
        compatibility: &str,
        resources: RestoreLimits,
        play_limits: PlayCheckpointLimits,
        gpu_sources: Option<PlayGpuSources>,
    ) -> Result<PlayResourcesCheckpoint, EditorError> {
        self.require_model_checkpoint()?;
        if !std::ptr::eq(view.graph, &raw const self.scene) {
            return Err("Play resource view belongs to another scene".into());
        }
        let (live, accepted_frames, shared_frames, preview_gpu) = match gpu_sources {
            Some(gpu) => (
                Some(gpu.live),
                Some(gpu.accepted),
                gpu.shared_frames,
                gpu.preview_shared_gpu_owner,
            ),
            None => (None, None, Vec::new(), None),
        };
        let gpu = live
            .as_ref()
            .map(|live| {
                crate::gpu_source_bindings::GpuSourceBindings::capture(
                    live,
                    |node| view.identity(node).cloned(),
                    resources.catalog.assets.max_references,
                )
            })
            .transpose()?;
        let models = self.checkpoint_resource_model_roots();
        let sources = std::cell::RefCell::new(self.play.animations.checkpoint_sources(
            play_limits.core.max_bytes,
            resources.catalog.assets.models.animation.max_references,
        )?);
        let play = self
            .play
            .checkpoint(
                view,
                registry,
                compatibility,
                &self.camera,
                |model| {
                    models
                        .iter()
                        .position(|m| Arc::ptr_eq(m, model))
                        .map(|i| format!("model:{i}"))
                },
                |clip, properties| {
                    sources.borrow_mut().intern(
                        clip,
                        properties,
                        resources.catalog.assets.models.animation.max_references,
                    )
                },
                play_limits,
            )
            .map_err(|e| e.to_string())?;
        let retained = live.map(|live| live.publications()).unwrap_or_default();
        let (watcher, delay) = self.require_model_checkpoint()?;
        let preview = self.capture_preview_checkpoint(
            |node| view.identity(node).cloned(),
            play_limits.core.max_bytes,
        )?;
        let saved_resources = self.capture_model_resources_at_barrier(
            resources.catalog,
            watcher,
            delay,
            &retained,
            PreviewCapture {
                checkpoint: preview,
                gpu_owner: preview_gpu,
                runtime_owner: self
                    .preview_runtime_frame_role(|node| view.identity(node).cloned())?,
            },
        )?;
        if saved_resources.playback_sources != sources.into_inner().bindings {
            return Err("Play resource animation sources changed during capture".into());
        }
        let saved = PlayResourcesCheckpoint {
            version: 1,
            resources: saved_resources,
            play,
            gpu,
            accepted_frames,
            shared_frames,
        };
        saved.to_bytes(resources.max_bytes)?;
        Ok(saved)
    }
}
impl PlayResourcesCheckpoint {
    pub(crate) fn to_bytes(&self, max: usize) -> Result<Vec<u8>, EditorError> {
        if self.version != 1 {
            return Err("unsupported Play resource version".into());
        }
        envelope(self, *b"VOXYPGR1", max)
    }
    pub(crate) fn from_bytes(bytes: &[u8], max: usize) -> Result<Self, EditorError> {
        let saved: Self = decode_envelope(bytes, *b"VOXYPGR1", max)?;
        if saved.version != 1 {
            return Err("unsupported Play resource version".into());
        }
        Ok(saved)
    }
    pub(crate) fn prepare(
        &self,
        compatibility: &str,
        registry: &ComponentRegistry,
        behaviors: &BehaviorCheckpointRegistry,
        ui: &UiActionCheckpointRegistry,
        root: &std::path::Path,
        limits: RestoreLimits,
        play_limits: PlayCheckpointLimits,
    ) -> Result<PreparedPlayResources, EditorError> {
        self.to_bytes(limits.max_bytes)?;
        if let Some(preview) = &self.resources.preview {
            preview.to_json(play_limits.core.max_bytes)?;
        }
        let mut resources = self.resources.prepare(root, limits)?;
        let mut game = self
            .play
            .prepare(
                compatibility,
                registry,
                behaviors,
                ui,
                |key| {
                    let index = key.strip_prefix("model:")?.parse::<usize>().ok()?;
                    if format!("model:{index}") != key {
                        return None;
                    }
                    resources.resources.models.get(index).cloned()
                },
                |key| resources.playback_sources.resolve(key),
                play_limits,
            )
            .map_err(|e| e.to_string())?;
        validate_animation_resources(
            &game.play.animations,
            &resources,
            play_limits.core.max_bytes,
            limits.catalog.assets.models.animation.max_references,
        )?;
        let gpu = self
            .gpu
            .as_ref()
            .map(|g| {
                g.prepare(
                    &game.scene.graph,
                    |id| game.scene.resolve(id),
                    &resources.resources.retained,
                    limits.catalog.assets.max_references,
                )
            })
            .transpose()?;
        if gpu.is_none() && !resources.resources.retained.is_empty() {
            return Err("Play retained GPU publications have no roles".into());
        }
        let accepted_frames = self
            .accepted_frames
            .as_ref()
            .map(|saved| {
                saved.prepare(
                    &game.scene.graph,
                    |id| game.scene.resolve(id),
                    gpu.as_ref()
                        .ok_or("Play accepted frames have no GPU roles")?,
                    play_limits.core.max_bytes,
                )
            })
            .transpose()?;
        let mut joined = std::collections::HashSet::new();
        for (runtime, render) in &self.shared_frames {
            if !joined.insert(runtime) {
                return Err("duplicate shared runtime frame role".into());
            }
            let runtime = game
                .scene
                .resolve(runtime)
                .ok_or("missing shared runtime frame role")?;
            let render = game
                .scene
                .resolve(render)
                .ok_or("missing shared GPU frame role")?;
            let accepted = accepted_frames
                .as_ref()
                .and_then(|frames| frames.get(&render))
                .ok_or("shared frame has no accepted GPU owner")?;
            game.play.animations.restore_shared_frame(
                runtime,
                accepted.frame.clone(),
                play_limits.core.max_bytes,
            )?;
        }
        resources.bind_preview(
            &game.scene.graph,
            |id| game.scene.resolve(id),
            &mut game.play.animations,
            accepted_frames.as_ref(),
            play_limits.core.max_bytes,
        )?;
        Ok(PreparedPlayResources {
            resources,
            game,
            gpu,
            accepted_frames,
        })
    }
}

/// Guard of the original running or idle PlaySession and its model participant.
/// UI resources, audio devices and host/window state remain outer owners.
pub(crate) struct PlayResourceGuard {
    scene: crate::app::restore_guard::SceneAuthoringGuard,
    models: crate::app::restore_guard::ModelRuntimeGuard,
    frame_epoch: Instant,
    digest: blake3::Hash,
    running: bool,
    resources: RestoreLimits,
    play: PlayCheckpointLimits,
}
impl App {
    pub(crate) fn observe_play_resources(
        &self,
        view: &SceneCheckpointView<'_>,
        registry: &ComponentRegistry,
        compatibility: &str,
        resources: RestoreLimits,
        play: PlayCheckpointLimits,
    ) -> Result<(PlayResourcesCheckpoint, PlayResourceGuard), EditorError> {
        let saved = self.capture_play_resources(view, registry, compatibility, resources, play)?;
        let digest = blake3::hash(&saved.to_bytes(resources.max_bytes)?);
        Ok((
            saved,
            PlayResourceGuard {
                scene: crate::app::restore_guard::SceneAuthoringGuard::capture(self),
                models: crate::app::restore_guard::ModelRuntimeGuard::capture(self)?,
                frame_epoch: self.play.simulation_time,
                digest,
                running: true,
                resources,
                play,
            },
        ))
    }
}
impl PlayResourceGuard {
    pub(crate) fn validate(
        &self,
        app: &App,
        view: &SceneCheckpointView<'_>,
        registry: &ComponentRegistry,
        compatibility: &str,
    ) -> Result<(), EditorError> {
        self.scene.validate(app)?;
        self.models.validate(app)?;
        // Idle rendering updates this host timer without advancing a simulation.
        if self.running && self.frame_epoch != app.play.simulation_time {
            return Err("Play host pacing epoch changed during restore preparation".into());
        }
        let digest = if self.running {
            let current = app.capture_play_resources(
                view,
                registry,
                compatibility,
                self.resources,
                self.play,
            )?;
            blake3::hash(&current.to_bytes(self.resources.max_bytes)?)
        } else {
            app.idle_play_fingerprint(view, registry, compatibility, self.resources, self.play)?
        };
        if digest != self.digest {
            return Err("Play runtime state changed during restore preparation".into());
        }
        Ok(())
    }
}

// Observation only: this is not another durable game/runtime representation.
#[derive(serde::Serialize)]
struct IdleObservation<'a> {
    compatibility: &'a str,
    scene: voxy_scene::SceneDocument,
    models: ModelAnimationCheckpoint,
    ticks: u64,
    camera: &'a crate::camera::ViewportCamera,
    angular: Option<voxy_gameplay::AngularMotionCheckpoint>,
    follow: Option<crate::follow_camera::FollowRuntimeCheckpoint>,
    optics: Vec<Option<[u32; 4]>>,
    liquid: Option<Vec<u8>>,
    physics: Option<voxy_gameplay::CharacterPhysicsCheckpoint>,
    input: String,
    ui: Option<voxy_gameplay::UiActionCheckpoint>,
}
impl App {
    /// Captures a source observation for loading a game from either editor or Play.
    pub(crate) fn observe_play_restore_origin(
        &self,
        view: &SceneCheckpointView<'_>,
        registry: &ComponentRegistry,
        compatibility: &str,
        resources: RestoreLimits,
        play: PlayCheckpointLimits,
    ) -> Result<PlayResourceGuard, EditorError> {
        if self.play.playing.is_some() || self.play.simulation.is_some() {
            return self
                .observe_play_resources(view, registry, compatibility, resources, play)
                .map(|(_, guard)| guard);
        }
        let digest = self.idle_play_fingerprint(view, registry, compatibility, resources, play)?;
        Ok(PlayResourceGuard {
            scene: crate::app::restore_guard::SceneAuthoringGuard::capture(self),
            models: crate::app::restore_guard::ModelRuntimeGuard::capture(self)?,
            frame_epoch: self.play.simulation_time,
            digest,
            running: false,
            resources,
            play,
        })
    }
    fn idle_play_fingerprint(
        &self,
        view: &SceneCheckpointView<'_>,
        registry: &ComponentRegistry,
        compatibility: &str,
        resources: RestoreLimits,
        limits: PlayCheckpointLimits,
    ) -> Result<blake3::Hash, EditorError> {
        self.require_model_checkpoint()?;
        if !std::ptr::eq(view.graph, &raw const self.scene)
            || compatibility.is_empty()
            || compatibility.len() > 1024
            || !self.camera.valid()
        {
            return Err("invalid idle Play source observation".into());
        }
        let crate::play_session::PlaySession {
            playing,
            simulation,
            simulation_time: _,
            animations: _,
            angular_motion,
            simulation_ticks,
            follow_camera,
            liquid_optics,
            liquid,
            physics,
            player_input,
            ui_actions,
        } = &self.play;
        if playing.is_some() || simulation.is_some() {
            return Err("idle Play source mode changed during restore preparation".into());
        }
        let identify = |node| view.identity(node).cloned();
        let record = IdleObservation {
            compatibility,
            scene: view.capture(registry)?,
            models: self.capture_model_animation_checkpoint(
                identify,
                resources.catalog,
                limits.core.max_bytes,
            )?,
            ticks: *simulation_ticks,
            camera: &self.camera,
            angular: angular_motion
                .as_ref()
                .map(voxy_gameplay::AngularMotionBatch::checkpoint),
            follow: follow_camera
                .as_ref()
                .map(|follow| follow.checkpoint(&self.camera, identify))
                .transpose()?,
            optics: liquid_optics
                .iter()
                .map(|v| v.map(|v| v.map(f32::to_bits)))
                .collect(),
            liquid: liquid
                .as_ref()
                .map(|liquid| {
                    liquid
                        .checkpoint(&self.scene, identify)
                        .and_then(|saved| saved.to_bytes(limits.liquid_wire.max_bytes))
                })
                .transpose()?,
            physics: physics
                .as_ref()
                .map(|physics| physics.checkpoint(&self.scene, identify))
                .transpose()?,
            input: voxy_gameplay::encode_input_checkpoint(
                &player_input.checkpoint(),
                limits.core.max_bytes,
                limits.core.max_actions,
                limits.core.max_bindings,
            )
            .map_err(|e| e.to_string())?,
            ui: ui_actions
                .as_ref()
                .map(|ui| ui.checkpoint(&self.scene, identify, limits.ui))
                .transpose()?,
        };
        let bytes = serde_json::to_vec(&record)?;
        if bytes.len() > resources.max_bytes {
            return Err("idle Play source observation exceeds policy".into());
        }
        Ok(blake3::hash(&bytes))
    }
}

pub(crate) struct PendingPlayResources {
    resources: PreparedModelResources,
    game: PreparedPlaySession,
    gpu: crate::gpu_model::restore::PendingModels,
    origin: Arc<PlayResourceGuard>,
}
pub(crate) struct ReadyPlayResources {
    pub resources: PreparedModelResources,
    pub game: PreparedPlaySession,
    pub gpu: crate::gpu_model::restore::ReadyModels,
    origin: Arc<PlayResourceGuard>,
}
impl PreparedPlayResources {
    pub(crate) fn stage_gpu(
        self,
        target: crate::gpu_model::batch::UploadTarget<'_>,
        origin: PlayResourceGuard,
    ) -> Result<PendingPlayResources, EditorError> {
        self.stage_gpu_shared(target, Arc::new(origin))
    }
    pub(crate) fn stage_gpu_shared(
        self,
        target: crate::gpu_model::batch::UploadTarget<'_>,
        origin: Arc<PlayResourceGuard>,
    ) -> Result<PendingPlayResources, EditorError> {
        let Self {
            resources,
            game,
            gpu,
            accepted_frames,
        } = self;
        let bindings = gpu.ok_or("full Play bundle has no GPU roles")?;
        let gpu = target.stage_restore(
            &game.scene.graph,
            &bindings,
            accepted_frames.unwrap_or_default(),
        )?;
        Ok(PendingPlayResources {
            resources,
            game,
            gpu,
            origin,
        })
    }
}
impl PendingPlayResources {
    pub(crate) async fn validate(self) -> Result<ReadyPlayResources, EditorError> {
        let gpu = self.gpu.validate().await?;
        Ok(ReadyPlayResources {
            resources: self.resources,
            game: self.game,
            gpu,
            origin: self.origin,
        })
    }
}
impl ReadyPlayResources {
    /// All checks run synchronously immediately before the enclosing exclusive commit.
    /// Outer authoring/UI/audio/host participants still need their own admission.
    pub(crate) fn validate_origin(
        &self,
        app: &App,
        view: &SceneCheckpointView<'_>,
        registry: &ComponentRegistry,
        compatibility: &str,
        target: &crate::gpu_model::batch::UploadTarget<'_>,
    ) -> Result<(), EditorError> {
        self.origin.validate(app, view, registry, compatibility)?;
        self.gpu
            .validate_destination(&self.game.scene.graph, target)?;
        Ok(())
    }
}

impl PendingPlayResources {
    pub(crate) fn discard(mut self) -> Vec<std::thread::JoinHandle<()>> {
        let mut workers = Vec::new();
        if let Some(imports) = self.resources.imports.take() {
            workers.push(imports.close());
        }
        if let Some(watcher) = self.resources.watcher.take() {
            workers.push(watcher.close());
        }
        workers
    }
}
