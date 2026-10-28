//! Compose original authoring history with the coordinated running game candidate.
use super::*;
use crate::app::authoring_checkpoint::{AuthoringHistoryCheckpoint, PreparedAuthoringHistory};
use voxy_scene::{ComponentRegistry, HistoryCheckpointLimits, SceneCheckpointView};
pub(crate) struct EditorCheckpoint {
    pub game: CoordinatedPlayCheckpoint,
    pub authoring: AuthoringHistoryCheckpoint,
}
pub(crate) struct PreparedEditorCheckpoint {
    pub game: PreparedCoordinatedPlay,
    pub authoring: PreparedAuthoringHistory,
}
impl App {
    pub(crate) fn editor_checkpoint(
        &self,
        view: &SceneCheckpointView<'_>,
        registry: &ComponentRegistry,
        compatibility: &str,
        resources: crate::app::resource_checkpoint::RestoreLimits,
        play: crate::play_session::checkpoint::PlayCheckpointLimits,
        ui: crate::ui_live::checkpoint::UiLiveCheckpointLimits,
        history: HistoryCheckpointLimits,
    ) -> Result<EditorCheckpoint, EditorError> {
        let game =
            self.coordinated_play_checkpoint(view, registry, compatibility, resources, play, ui)?;
        let authoring = self.authoring_history_checkpoint(view, history)?;
        Ok(EditorCheckpoint { game, authoring })
    }
}
impl EditorCheckpoint {
    /// Failure of history preparation drops only the detached game candidate and
    /// closes its workers; it never changes the original App/history pair.
    pub(crate) fn prepare(
        &self,
        options: CoordinatedPrepare<'_>,
        limits: HistoryCheckpointLimits,
    ) -> Result<PreparedEditorCheckpoint, EditorError> {
        self.prepare_with_output(options, limits, None, false)
    }
    fn prepare_with_output(
        &self,
        options: CoordinatedPrepare<'_>,
        limits: HistoryCheckpointLimits,
        output: Option<&voxy_audio_device::OutputConnection>,
        native_enabled: bool,
    ) -> Result<PreparedEditorCheckpoint, EditorError> {
        let registry = options.registry;
        self.authoring.to_json(limits.max_archive_bytes)?;
        let game = self
            .game
            .prepare_with_output(options, output, native_enabled)?;
        let authoring = self.authoring.prepare(
            &game
                .game
                .as_ref()
                .ok_or("missing prepared game")?
                .game
                .scene
                .checkpoint_view(),
            registry,
            limits,
        )?;
        Ok(PreparedEditorCheckpoint { game, authoring })
    }
}

const HEADER: usize = 56;
const MAGIC: &[u8; 8] = b"VOXYEDR1";
#[derive(Clone, Copy)]
pub(crate) struct EditorFileLimits {
    pub max_bytes: usize,
    pub game: super::file::CoordinatedFileLimits,
    pub history: HistoryCheckpointLimits,
}
impl EditorCheckpoint {
    pub(crate) fn to_bytes(&self, limits: EditorFileLimits) -> Result<Vec<u8>, EditorError> {
        let remaining = limits
            .max_bytes
            .checked_sub(HEADER)
            .ok_or("editor file byte budget")?;
        let mut game_limits = limits.game;
        game_limits.max_bytes = game_limits.max_bytes.min(remaining);
        let game = self.game.to_bytes(game_limits)?;
        let remaining = remaining
            .checked_sub(game.len())
            .ok_or("editor file byte budget")?;
        if remaining == 0 {
            return Err("editor file byte budget".into());
        }
        let history = self
            .authoring
            .to_json(limits.history.max_archive_bytes.min(remaining))?;
        let mut output = Vec::with_capacity(HEADER + game.len() + history.len());
        output.extend_from_slice(MAGIC);
        output.extend_from_slice(&u64::try_from(game.len())?.to_le_bytes());
        output.extend_from_slice(&u64::try_from(history.len())?.to_le_bytes());
        let mut hash = blake3::Hasher::new();
        hash.update(&output[8..]);
        hash.update(&game);
        hash.update(history.as_bytes());
        output.extend_from_slice(hash.finalize().as_bytes());
        output.extend_from_slice(&game);
        output.extend_from_slice(history.as_bytes());
        Ok(output)
    }
    pub(crate) fn from_bytes(bytes: &[u8], limits: EditorFileLimits) -> Result<Self, EditorError> {
        if bytes.len() < HEADER || bytes.len() > limits.max_bytes || &bytes[..8] != MAGIC {
            return Err("invalid editor file header or budget".into());
        }
        let length = |start| -> Result<usize, EditorError> {
            Ok(usize::try_from(u64::from_le_bytes(
                bytes[start..start + 8]
                    .try_into()
                    .map_err(|_| "invalid editor section length")?,
            ))?)
        };
        let game_len = length(8)?;
        let history_len = length(16)?;
        if game_len > limits.game.max_bytes
            || history_len > limits.history.max_archive_bytes
            || HEADER
                .checked_add(game_len)
                .and_then(|v| v.checked_add(history_len))
                != Some(bytes.len())
        {
            return Err("editor section byte budget or length".into());
        }
        let mut hash = blake3::Hasher::new();
        hash.update(&bytes[8..24]);
        hash.update(&bytes[HEADER..]);
        if hash.finalize().as_bytes() != &bytes[24..HEADER] {
            return Err("corrupt editor file".into());
        }
        let split = HEADER + game_len;
        Ok(Self {
            game: CoordinatedPlayCheckpoint::from_bytes(&bytes[HEADER..split], limits.game)?,
            authoring: AuthoringHistoryCheckpoint::from_json(
                std::str::from_utf8(&bytes[split..])?,
                limits.history.max_archive_bytes,
            )?,
        })
    }
    pub(crate) fn save_file(
        &self,
        path: &std::path::Path,
        limits: EditorFileLimits,
    ) -> Result<(), EditorError> {
        let bytes = self.to_bytes(limits)?;
        voxy_assets::save_atomic_file(path, &bytes, limits.max_bytes).map_err(|e| e.to_string())?;
        Ok(())
    }
    pub(crate) fn read_file(
        path: &std::path::Path,
        limits: EditorFileLimits,
    ) -> Result<Self, EditorError> {
        use std::io::Read;
        let file = std::fs::File::open(path)?;
        let max = u64::try_from(limits.max_bytes)?;
        if file.metadata()?.len() > max {
            return Err("editor file byte budget".into());
        }
        let mut bytes = Vec::new();
        file.take(max.checked_add(1).ok_or("editor read limit overflow")?)
            .read_to_end(&mut bytes)?;
        Self::from_bytes(&bytes, limits)
    }
}

impl EditorCheckpoint {
    pub(crate) fn prepare_guarded(
        &self,
        options: CoordinatedPrepare<'_>,
        limits: HistoryCheckpointLimits,
        origin: CoordinatedOrigin,
    ) -> Result<PreparedEditorCheckpoint, EditorError> {
        self.prepare_guarded_with_output(options, limits, origin, None, false)
    }
    pub(crate) fn prepare_guarded_with_output(
        &self,
        options: CoordinatedPrepare<'_>,
        limits: HistoryCheckpointLimits,
        origin: CoordinatedOrigin,
        output: Option<&voxy_audio_device::OutputConnection>,
        native_enabled: bool,
    ) -> Result<PreparedEditorCheckpoint, EditorError> {
        let mut candidate = self.prepare_with_output(options, limits, output, native_enabled)?;
        candidate.game.origin = Some(origin);
        Ok(candidate)
    }
}
impl PreparedEditorCheckpoint {
    pub(crate) fn validate_origin(
        &self,
        app: &mut App,
        bindings: &[(ObjectId, NodeId)],
        registry: &ComponentRegistry,
        compatibility: &str,
    ) -> Result<(), EditorError> {
        let scene = &self
            .game
            .game
            .as_ref()
            .ok_or("missing prepared editor game")?
            .game
            .scene
            .graph;
        if self.authoring.scene_id != scene.identity() {
            return Err("prepared editor history belongs to another scene".into());
        }
        self.game
            .validate_origin(app, bindings, registry, compatibility)
    }
}

pub(crate) struct PendingEditorCheckpoint {
    game: PendingCoordinatedPlay,
    authoring: PreparedAuthoringHistory,
}
pub(crate) struct ReadyEditorCheckpoint {
    pub game: ReadyCoordinatedPlay,
    pub authoring: PreparedAuthoringHistory,
}
impl PreparedEditorCheckpoint {
    pub(crate) fn stage_gpu(
        self,
        target: crate::gpu_model::batch::UploadTarget<'_>,
    ) -> Result<PendingEditorCheckpoint, EditorError> {
        let scene = &self
            .game
            .game
            .as_ref()
            .ok_or("missing prepared editor game")?
            .game
            .scene
            .graph;
        if self.authoring.scene_id != scene.identity() {
            return Err("prepared editor history belongs to another scene".into());
        }
        Ok(PendingEditorCheckpoint {
            game: self.game.stage_gpu(target)?,
            authoring: self.authoring,
        })
    }
}
impl PendingEditorCheckpoint {
    pub(crate) async fn validate(self) -> Result<ReadyEditorCheckpoint, EditorError> {
        let game = Box::pin(self.game.validate()).await?;
        if self.authoring.scene_id != game.game.game.scene.graph.identity() {
            return Err("prepared editor history belongs to another scene".into());
        }
        Ok(ReadyEditorCheckpoint {
            game,
            authoring: self.authoring,
        })
    }
    pub(crate) fn discard(self) -> Vec<std::thread::JoinHandle<()>> {
        self.game.discard()
    }
}
impl ReadyEditorCheckpoint {
    pub(crate) fn validate_origin(
        &self,
        app: &mut App,
        bindings: &[(ObjectId, NodeId)],
        registry: &ComponentRegistry,
        compatibility: &str,
        target: &crate::gpu_model::batch::UploadTarget<'_>,
    ) -> Result<(), EditorError> {
        if self.authoring.scene_id != self.game.game.game.scene.graph.identity() {
            return Err("prepared editor history belongs to another scene".into());
        }
        self.game
            .validate_origin(app, bindings, registry, compatibility, target)
    }
    pub(crate) fn discard(self) -> Vec<std::thread::JoinHandle<()>> {
        self.game.discard()
    }
}

struct EditorInstallationState {
    outer: PreparedCoordinatedPlay,
    resources: crate::app::resource_checkpoint::PreparedModelResources,
    game: crate::play_session::checkpoint::PreparedPlaySession,
    authoring: PreparedAuthoringHistory,
}
pub(crate) struct PendingEditorInstallation {
    state: EditorInstallationState,
    publication: crate::ui_live::PendingDetachedModelsUi,
}
pub(crate) struct ReadyEditorInstallation {
    state: EditorInstallationState,
    publication: crate::ui_live::ReadyDetachedModelsUi,
}
impl PendingEditorInstallation {
    pub(crate) async fn validate(self) -> Result<ReadyEditorInstallation, EditorError> {
        let publication = Box::pin(self.publication.validate())
            .await
            .map_err(|rejected| rejected.ui.error)?;
        Ok(ReadyEditorInstallation {
            state: self.state,
            publication,
        })
    }
    pub(crate) fn discard(mut self) -> Vec<std::thread::JoinHandle<()>> {
        let mut workers = retire_staged_models(&mut self.state.resources);
        workers.extend(self.publication.discard());
        workers.extend(self.state.outer.discard());
        workers
    }
}
impl ReadyEditorInstallation {
    pub(crate) fn poll_audio_ready(
        &mut self,
        app: &mut App,
        bindings: &[(ObjectId, NodeId)],
        registry: &ComponentRegistry,
        compatibility: &str,
    ) -> Result<bool, EditorError> {
        self.state
            .outer
            .validate_origin(app, bindings, registry, compatibility)?;
        match self
            .state
            .outer
            .audio
            .as_mut()
            .ok_or("missing prepared audio")?
        {
            PreparedCoordinatedAudio::Offline(candidate) => {
                candidate.validate_output_mode(app.audio.output_mode())?;
                Ok(true)
            }
            PreparedCoordinatedAudio::Native(candidate) => app.audio.poll_native_checkpoint_ready(
                candidate,
                &self.state.game.scene.graph,
                self.state
                    .outer
                    .project
                    .as_ref()
                    .ok_or("missing prepared project")?,
                |node| {
                    bindings
                        .iter()
                        .find(|(_, n)| *n == node)
                        .map(|(id, _)| id.clone())
                },
            ),
        }
    }
    pub(crate) fn discard(mut self) -> Vec<std::thread::JoinHandle<()>> {
        let mut workers = retire_staged_models(&mut self.state.resources);
        workers.extend(self.publication.discard());
        workers.extend(self.state.outer.discard());
        workers
    }
}
impl App {
    /// Convenience for exclusive callers. The event loop can instead retain the
    /// owned pending installation and poll its GPU checks without borrowing App.
    pub(crate) async fn install_editor_checkpoint(
        &mut self,
        candidate: ReadyEditorCheckpoint,
        bindings: &[(ObjectId, NodeId)],
        registry: &ComponentRegistry,
        compatibility: &str,
    ) -> Result<RetiredEditorCheckpoint, EditorError> {
        let pending =
            self.stage_editor_checkpoint_install(candidate, bindings, registry, compatibility)?;
        let mut ready = Box::pin(pending.validate()).await?;
        std::future::poll_fn(|context| {
            match ready.poll_audio_ready(self, bindings, registry, compatibility) {
                Ok(true) => std::task::Poll::Ready(Ok(())),
                Ok(false) => {
                    context.waker().wake_by_ref();
                    std::task::Poll::Pending
                }
                Err(error) => std::task::Poll::Ready(Err(error)),
            }
        })
        .await?;
        self.publish_editor_checkpoint_install(ready, bindings, registry, compatibility)
    }
    pub(crate) fn stage_editor_checkpoint_install(
        &mut self,
        candidate: ReadyEditorCheckpoint,
        bindings: &[(ObjectId, NodeId)],
        registry: &ComponentRegistry,
        compatibility: &str,
    ) -> Result<PendingEditorInstallation, EditorError> {
        if candidate.authoring.scene_id != candidate.game.game.game.scene.graph.identity() {
            return Err("prepared editor history belongs to another scene".into());
        }
        candidate
            .game
            .outer
            .validate_origin(self, bindings, registry, compatibility)?;
        let ReadyEditorCheckpoint { game, authoring } = candidate;
        let ReadyCoordinatedPlay { mut outer, game } = game;
        if outer.project.is_none() || outer.audio.is_none() || outer.ui_input.is_none() {
            return Err("incomplete editor installation candidate".into());
        }
        let ui = outer.ui.take().ok_or("missing prepared UI")?;
        let graphics = self
            .graphics
            .as_mut()
            .ok_or("editor restore requires graphics")?;
        game.gpu.validate_destination(
            &game.game.scene.graph,
            &crate::gpu_model::batch::UploadTarget {
                renderer: &graphics.renderer,
                device: &graphics.device,
                queue: &graphics.queue,
                cache: &graphics.residency_cache,
                live_geometry: graphics.geometry_bytes(),
                replaced_geometry: graphics.checkpoint_replaced_geometry(),
            },
        )?;
        let permit = self.ui_live.checkpoint_publication_at_barrier(graphics)?;
        let publication = permit
            .stage_after_models(ui, &game.game.scene.graph, game.gpu)
            .map_err(|rejected| rejected.ui.error)?
            .detach_at_barrier()
            .map_err(|rejected| rejected.ui.error)?;
        Ok(PendingEditorInstallation {
            state: EditorInstallationState {
                outer,
                resources: game.resources,
                game: game.game,
                authoring,
            },
            publication,
        })
    }
    pub(crate) fn publish_editor_checkpoint_install(
        &mut self,
        ready: ReadyEditorInstallation,
        bindings: &[(ObjectId, NodeId)],
        registry: &ComponentRegistry,
        compatibility: &str,
    ) -> Result<RetiredEditorCheckpoint, EditorError> {
        if ready.state.authoring.scene_id != ready.state.game.scene.graph.identity() {
            return Err("prepared editor history belongs to another scene".into());
        }
        ready
            .state
            .outer
            .validate_origin(self, bindings, registry, compatibility)?;
        let max_audio = self
            .restore_barrier
            .as_ref()
            .ok_or("missing restore barrier")?
            .max_audio_bytes;
        let game = ready.state;
        let mut outer = game.outer;
        let authoring = game.authoring;
        let mut project = outer.project.take().ok_or("missing prepared project")?;
        project.registry = self.authoring.authoring_project.registry.clone();
        let audio = outer.audio.take().ok_or("missing prepared audio")?;
        if let PreparedCoordinatedAudio::Offline(candidate) = &audio {
            candidate.validate_output_mode(self.audio.output_mode())?;
        }
        let ui_input = outer.ui_input.take().ok_or("missing prepared UI input")?;
        let restored_scan = Instant::now()
            .checked_sub(Duration::from_millis(200).saturating_sub(game.resources.delay))
            .ok_or("model poll delay cannot be restored")?;
        let graphics = self
            .graphics
            .as_mut()
            .ok_or("editor restore requires graphics")?;
        ready.publication.validate_destination(
            &game.game.scene.graph,
            &crate::gpu_model::batch::UploadTarget {
                renderer: &graphics.renderer,
                device: &graphics.device,
                queue: &graphics.queue,
                cache: &graphics.residency_cache,
                live_geometry: graphics.geometry_bytes(),
                replaced_geometry: graphics.checkpoint_replaced_geometry(),
            },
        )?;
        let (models, mut retired, committed_audio) = {
            let (offline_audio, mut native_audio) = match audio {
                PreparedCoordinatedAudio::Offline(audio) => (Some(audio), None),
                PreparedCoordinatedAudio::Native(audio) => (None, Some(audio)),
            };
            let identify = |node| {
                bindings
                    .iter()
                    .find(|(_, n)| *n == node)
                    .map(|(id, _)| id.clone())
            };
            enum Permit<'a, 'b, 'c> {
                Offline(crate::audio_session::checkpoint::AudioPublicationPermit<'a>),
                Native(crate::audio_session::checkpoint::NativeAudioPublicationPermit<'a, 'b, 'c>),
            }
            let audio_permit = match &mut native_audio {
                Some(audio) => Permit::Native(
                    self.audio
                        .native_checkpoint_publication(
                            audio,
                            &game.game.scene.graph,
                            &project,
                            identify,
                        )?
                        .ok_or("native output publication is not quiescent")?,
                ),
                None => Permit::Offline(
                    self.audio
                        .checkpoint_publication_at_barrier(identify, max_audio)?,
                ),
            };
            let ui_permit = self.ui_live.checkpoint_publication_at_barrier(graphics)?;
            let publication = ready
                .publication
                .rebind_at_barrier(ui_permit, &game.game.scene.graph)
                .map_err(|rejected| rejected.ui.error)?;
            // Source and destination admission just completed under one exclusive
            // borrow. No fallible operations or awaits occur below this point.
            let (models, retired) = publication.publish();
            let committed_audio = match audio_permit {
                Permit::Offline(permit) => {
                    permit.publish(*offline_audio.expect("admitted offline audio"));
                    None
                }
                Permit::Native(permit) => Some(permit.publish()),
            };
            (models, retired, committed_audio)
        };
        if let Some(worker) = self.imports.take() {
            retired.push(worker.close());
        }
        if let Some(worker) = self.watcher.take() {
            retired.push(worker.close());
        }
        let mut resources = game.resources;
        let crate::app::authoring_checkpoint::PreparedAuthoringHistory {
            scene_id: _,
            metadata,
            history,
            object_ids,
            instances,
            selected,
            next_object_id,
            drag,
        } = authoring;
        let mut restored_authoring = crate::authoring_session::AuthoringSession {
            preview_seek: None,
            animation_preview: resources.preview.take(),
            history,
            settings_written: BTreeMap::new(),
            scene_path: None,
            scene_revision: None,
            authoring_project: project,
            prefab_assets: Vec::new(),
            prefab_choice: 0,
            authoring_source: None,
            next_object_id,
        };
        if let Some(metadata) = metadata {
            metadata.install(&mut restored_authoring);
        }
        self.authoring = restored_authoring;
        let old_scene = std::mem::replace(&mut self.scene, game.game.scene.graph);
        let old_play = std::mem::replace(&mut self.play, game.game.play);
        // Preparation/GPU wait time is not elapsed game time. Native saved
        // accumulators remain intact; only the unsaved host pacing epoch changes.
        self.play.simulation_time = Instant::now();
        self.camera = game.game.camera;
        self.object_ids = object_ids;
        self.instances = instances;
        self.selected = selected;
        self.drag = drag;
        self.catalog = resources.resources.catalog;
        self.sources = resources.resources.sources;
        self.id = resources.current;
        self.import_recipe = resources.recipe;
        self.available = resources.outputs;
        self.reload = resources.queued;
        self.imports = resources.imports.take();
        self.watcher = resources.watcher.take();
        self.last_scan = restored_scan;
        self.scan_active = false;
        self.last_publication = None;
        self.game_ui_input = ui_input;
        graphics.models = models.models;
        graphics.animated_models = models.animated;
        graphics.residency_errors.clear();
        graphics.transforms.clear();
        graphics.lod_history.clear();
        graphics.lod_pending.clear();
        graphics.optical_liquids.clear();
        graphics.liquid_revision = None;
        graphics.liquid_geometry = None;
        graphics.liquid_transforms.clear();
        graphics.gizmo_transforms.clear();
        graphics.fog = None;
        self.extraction = SceneExtraction::new(scene_limits::OBJECTS);
        self.frame_styles = None;
        self.secondary_camera = None;
        self.active_view = 0;
        self.cursor = None;
        self.camera_drag = None;
        self.keyboard_device = None;
        self.inspector = InspectorMode::Transform;
        self.panel_cache = None;
        self.field = None;
        self.component_edit = None;
        self.retarget_draft = None;
        self.retarget_picker = None;
        self.tree_scroll = 0;
        self.parenting = None;
        self.follow_pick = None;
        self.model_relink_draft = None;
        self.model_relink_error = None;
        self.error = None;
        self.resource_checkpoint = None;
        self.restore_barrier = None;
        if let Some(panels) = &mut self.panels {
            panels.invalidate_presentation();
        }
        self.trace.pending = true;
        retired.extend(
            committed_audio
                .map(|audio| audio.finish())
                .unwrap_or_default(),
        );
        retired.extend(self.audio.take_retired_workers());
        Ok(RetiredEditorCheckpoint {
            scene: old_scene,
            play: old_play,
            workers: retired,
        })
    }
}

/// Stop callbacks see only the old scene, never the newly installed graph.
/// Release callbacks at the publication boundary; join closed workers separately.
#[must_use = "retire the replaced simulation and join its closed workers"]
pub(crate) struct RetiredEditorCheckpoint {
    scene: SceneGraph,
    play: crate::play_session::PlaySession,
    workers: Vec<std::thread::JoinHandle<()>>,
}
impl RetiredEditorCheckpoint {
    /// Dispatch old lifecycle callbacks without joining any IO thread.
    pub(crate) fn release(mut self) -> (Vec<std::thread::JoinHandle<()>>, Vec<String>) {
        let mut errors = Vec::new();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.play.stop_simulation(&mut self.scene)
        })) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => errors.push(format!("retired simulation: {error}")),
            Err(payload) => errors.push(format!(
                "retired simulation panicked: {}",
                crate::panic_message(payload.as_ref())
            )),
        }
        (self.workers, errors)
    }
    pub(crate) fn finish(self) -> Vec<String> {
        let (workers, mut errors) = self.release();
        for worker in workers {
            if let Err(payload) = worker.join() {
                errors.push(format!(
                    "retired worker panicked: {}",
                    crate::panic_message(payload.as_ref())
                ));
            }
        }
        errors
    }
}

impl Graphics {
    /// Scene allocations retired by a complete editor snapshot installation.
    /// Host gizmos/panels and the separately staged UI remain in the base inventory.
    pub(crate) fn checkpoint_replaced_geometry(&self) -> u64 {
        self.models
            .values()
            .map(ModelGraphics::geometry_bytes)
            .sum::<u64>()
            + self.animated_models.allocation_bytes()
            + self
                .fog
                .as_ref()
                .map_or(0, fog_draw::FogDraw::allocation_bytes)
            + self
                .optical_liquids
                .values()
                .map(voxy_render::ScreenSpaceFluidRenderer::allocation_bytes)
                .sum::<u64>()
            + self
                .liquid_geometry
                .as_ref()
                .map_or(0, SceneGeometry::allocation_bytes)
    }
}

#[cfg(test)]
mod retirement_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Debug)]
    struct StopProbe {
        scene: voxy_scene::SceneId,
        calls: Arc<AtomicUsize>,
        panic: bool,
    }
    impl voxy_scene::Behavior for StopProbe {
        fn on_destroy(&mut self, scene: &mut SceneGraph, owner: NodeId) {
            assert_eq!(scene.identity(), self.scene);
            assert!(scene.local(owner).is_ok());
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert!(!self.panic, "retired callback failure");
        }
    }
    #[test]
    fn retired_simulation_uses_its_old_scene_and_joins_workers_even_after_callback_panic() {
        for panic in [false, true] {
            let mut scene = SceneGraph::new(4);
            let owner = scene.spawn(None, Default::default()).unwrap();
            let calls = Arc::new(AtomicUsize::new(0));
            let mut simulation = voxy_scene::SceneSimulation::new(
                &scene,
                crate::play_session::checkpoint::tests::limits()
                    .core
                    .simulation,
            )
            .unwrap();
            let identity = scene.identity();
            simulation
                .attach(
                    &mut scene,
                    owner,
                    StopProbe {
                        scene: identity,
                        calls: calls.clone(),
                        panic,
                    },
                )
                .unwrap();
            let play = crate::play_session::PlaySession {
                playing: None,
                simulation_time: Instant::now(),
                simulation: Some(simulation),
                animations: Default::default(),
                angular_motion: None,
                simulation_ticks: 0,
                follow_camera: None,
                liquid_optics: Vec::new(),
                liquid: None,
                physics: None,
                player_input: voxy_gameplay::player_input().unwrap(),
                ui_actions: None,
            };
            let joined = Arc::new(AtomicUsize::new(0));
            let completed = joined.clone();
            let worker = std::thread::spawn(move || {
                completed.fetch_add(1, Ordering::SeqCst);
            });
            let retired = RetiredEditorCheckpoint {
                scene,
                play,
                workers: vec![worker],
            };
            let errors = retired.finish();
            assert_eq!(errors.is_empty(), !panic);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert_eq!(joined.load(Ordering::SeqCst), 1);
        }
    }
}
