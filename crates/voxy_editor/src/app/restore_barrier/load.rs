//! Frame-polled file/CPU/GPU load command using the original checkpoint codecs.
use super::*;
use editor::{
    EditorCheckpoint, PreparedEditorCheckpoint, ReadyEditorCheckpoint, ReadyEditorInstallation,
};
use save::EditorSavePolicy;
use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll, Waker},
    thread::JoinHandle,
};
pub(crate) struct Request {
    path: PathBuf,
    root: PathBuf,
    policy: EditorSavePolicy,
    fence: Option<Arc<()>>,
    cancelled: bool,
}
pub(crate) struct LoadContext {
    request: Box<Request>,
    bindings: Vec<(ObjectId, NodeId)>,
    registry: Arc<voxy_scene::ComponentRegistry>,
}
type ModelsCheck = Pin<Box<dyn Future<Output = Result<ReadyEditorCheckpoint, EditorError>>>>;
type UiCheck = Pin<Box<dyn Future<Output = Result<ReadyEditorInstallation, EditorError>>>>;
pub(crate) enum PendingEditorLoad {
    Reading {
        request: Box<Request>,
        worker: JoinHandle<Result<EditorCheckpoint, String>>,
    },
    Fencing {
        request: Box<Request>,
        saved: Box<EditorCheckpoint>,
    },
    Device {
        request: Box<Request>,
        saved: Box<EditorCheckpoint>,
        rate: u32,
        started: Instant,
    },
    Preparing {
        context: Box<LoadContext>,
        worker: JoinHandle<Result<PreparedEditorCheckpoint, String>>,
    },
    Models {
        context: Box<LoadContext>,
        device: wgpu::Device,
        checks: ModelsCheck,
    },
    Ui {
        context: Box<LoadContext>,
        device: wgpu::Device,
        checks: UiCheck,
    },
    Audio {
        context: Box<LoadContext>,
        candidate: Box<ReadyEditorInstallation>,
    },
    Resuming {
        error: String,
        fence: Arc<()>,
    },
    Cancelling {
        workers: Vec<JoinHandle<()>>,
        errors: Vec<String>,
    },
    Retiring {
        path: PathBuf,
        workers: Vec<JoinHandle<()>>,
        errors: Vec<String>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LoadStatus {
    pub label: &'static str,
    pub cancellable: bool,
}
impl App {
    pub(crate) fn editor_load_status(&self) -> Option<LoadStatus> {
        let pending = self.editor_load.as_ref()?;
        let cancelled = match pending {
            PendingEditorLoad::Reading { request, .. }
            | PendingEditorLoad::Fencing { request, .. }
            | PendingEditorLoad::Device { request, .. } => request.cancelled,
            PendingEditorLoad::Preparing { context, .. }
            | PendingEditorLoad::Models { context, .. }
            | PendingEditorLoad::Ui { context, .. }
            | PendingEditorLoad::Audio { context, .. } => context.request.cancelled,
            _ => false,
        };
        let label = if cancelled {
            "Cancelling load..."
        } else {
            match pending {
                PendingEditorLoad::Reading { .. } => "Reading scene...",
                PendingEditorLoad::Fencing { .. } => "Finishing current frame...",
                PendingEditorLoad::Device { .. } => "Opening audio output...",
                PendingEditorLoad::Preparing { .. } => "Preparing scene...",
                PendingEditorLoad::Models { .. } => "Uploading models...",
                PendingEditorLoad::Ui { .. } => "Preparing interface...",
                PendingEditorLoad::Audio { .. } => "Preparing audio output...",
                PendingEditorLoad::Resuming { .. } => "Recovering current scene...",
                PendingEditorLoad::Cancelling { .. } => "Cancelling load...",
                PendingEditorLoad::Retiring { .. } => "Finishing scene load...",
            }
        };
        Some(LoadStatus {
            label,
            cancellable: !cancelled
                && matches!(
                    pending,
                    PendingEditorLoad::Reading { .. }
                        | PendingEditorLoad::Fencing { .. }
                        | PendingEditorLoad::Device { .. }
                        | PendingEditorLoad::Preparing { .. }
                        | PendingEditorLoad::Models { .. }
                        | PendingEditorLoad::Ui { .. }
                        | PendingEditorLoad::Audio { .. }
                ),
        })
    }
    /// Returns true while a load owns Escape, including during cancellation cleanup.
    pub(crate) fn editor_load_escape(&mut self, key: KeyCode) -> bool {
        if key != KeyCode::Escape || self.editor_load.is_none() {
            return false;
        }
        self.cancel_editor_file_load();
        self.panel_cache = None;
        true
    }
    pub(crate) fn request_editor_file_load(
        &mut self,
        path: PathBuf,
        root: PathBuf,
        policy: EditorSavePolicy,
    ) -> Result<(), EditorError> {
        if self.editor_load.is_some()
            || self.editor_save.is_some()
            || self.restore_barrier.is_some()
            || policy.compatibility.is_empty()
        {
            return Err(
                "editor file operation is already pending or compatibility is empty".into(),
            );
        }
        let destination = path.clone();
        let limits = policy.file;
        let worker = std::thread::Builder::new()
            .name("voxy-editor-read".into())
            .spawn(move || {
                let saved = EditorCheckpoint::read_file(&destination, limits)
                    .map_err(|error| error.to_string())?;
                // Validate the nested PCM archive off-frame, before any source
                // fence or native device is acquired. Later metadata lookup is O(1).
                saved
                    .game
                    .audio
                    .validate_native_output(limits.game.max_audio_bytes)
                    .map_err(|error| error.to_string())?;
                Ok(saved)
            })?;
        self.snapshot_error = None;
        self.panel_cache = None;
        self.editor_load = Some(PendingEditorLoad::Reading {
            request: Box::new(Request {
                path,
                root,
                policy,
                fence: None,
                cancelled: false,
            }),
            worker,
        });
        Ok(())
    }
    /// Cancellation is cooperative: outstanding work settles without publication.
    pub(crate) fn cancel_editor_file_load(&mut self) -> bool {
        let Some(request) = self
            .editor_load
            .as_mut()
            .and_then(PendingEditorLoad::request_mut)
        else {
            return false;
        };
        if request.cancelled {
            return false;
        }
        request.cancelled = true;
        true
    }
    /// Call after source-frame resource adoption/presentation, not before it.
    /// GPU polling is nonblocking; only already-finished threads are joined.
    pub(crate) fn poll_editor_file_load(&mut self) -> Result<Option<PathBuf>, EditorError> {
        let Some(mut pending) = self.editor_load.take() else {
            return Ok(None);
        };
        // Release only this operation's source fence, even if another operation
        // has acquired a new one while cancelled work was still settling.
        if let Some(request) = pending.request_mut() {
            if request.cancelled {
                if let Some(fence) = &request.fence {
                    if self
                        .restore_barrier
                        .as_ref()
                        .is_some_and(|barrier| Arc::ptr_eq(&barrier.epoch, fence))
                    {
                        match self.resume_restore_barrier() {
                            Ok(true) => request.fence = None,
                            Ok(false) => {
                                self.editor_load = Some(pending);
                                return Ok(None);
                            }
                            Err(error) => {
                                self.editor_load = Some(pending);
                                return Err(error);
                            }
                        }
                    } else {
                        request.fence = None;
                    }
                }
            }
        }
        let mut pending = match pending {
            PendingEditorLoad::Resuming { error, fence } => {
                if !self
                    .restore_barrier
                    .as_ref()
                    .is_some_and(|barrier| Arc::ptr_eq(&barrier.epoch, &fence))
                {
                    return Err(error.into());
                }
                match self.resume_restore_barrier() {
                    Ok(true) => return Err(error.into()),
                    Ok(false) => {
                        self.editor_load = Some(PendingEditorLoad::Resuming { error, fence })
                    }
                    Err(release) => {
                        self.editor_load = Some(PendingEditorLoad::Resuming { error, fence });
                        return Err(release);
                    }
                }
                return Ok(None);
            }
            PendingEditorLoad::Cancelling {
                workers,
                mut errors,
            } => {
                let mut waiting = Vec::new();
                for worker in workers {
                    if worker.is_finished() {
                        record_join(worker, &mut errors);
                    } else {
                        waiting.push(worker);
                    }
                }
                if waiting.is_empty() {
                    return if errors.is_empty() {
                        Ok(None)
                    } else {
                        Err(format!(
                            "editor load cancelled; cleanup failed: {}",
                            errors.join("; ")
                        )
                        .into())
                    };
                }
                self.editor_load = Some(PendingEditorLoad::Cancelling {
                    workers: waiting,
                    errors,
                });
                return Ok(None);
            }
            PendingEditorLoad::Retiring {
                path,
                workers,
                mut errors,
            } => {
                let mut waiting = Vec::new();
                for worker in workers {
                    if worker.is_finished() {
                        record_join(worker, &mut errors);
                    } else {
                        waiting.push(worker);
                    }
                }
                if waiting.is_empty() {
                    return if errors.is_empty() {
                        Ok(Some(path))
                    } else {
                        Err(format!(
                            "editor snapshot installed; retirement failed: {}",
                            errors.join("; ")
                        )
                        .into())
                    };
                }
                self.editor_load = Some(PendingEditorLoad::Retiring {
                    path,
                    workers: waiting,
                    errors,
                });
                return Ok(None);
            }
            pending => pending,
        };
        let fence = pending
            .request_mut()
            .and_then(|request| request.fence.clone());
        if fence.as_ref().is_some_and(|fence| {
            !self
                .restore_barrier
                .as_ref()
                .is_some_and(|barrier| Arc::ptr_eq(&barrier.epoch, fence))
        }) {
            return Err("editor load source fence changed".into());
        }
        let next = (|| -> Result<PendingEditorLoad, EditorError> {
            Ok(match pending {
                PendingEditorLoad::Reading {
                    mut request,
                    worker,
                } => {
                    if !worker.is_finished() {
                        return Ok(PendingEditorLoad::Reading { request, worker });
                    }
                    let saved = join_job(worker);
                    if request.cancelled {
                        return Ok(PendingEditorLoad::Cancelling {
                            workers: Vec::new(),
                            errors: Vec::new(),
                        });
                    }
                    let saved = saved?;
                    // A corrupt or oversized file never acquires a source fence.
                    self.begin_restore_barrier(request.policy.file.game.max_audio_bytes)?;
                    request.fence = Some(self.restore_barrier.as_ref().unwrap().epoch.clone());
                    PendingEditorLoad::Fencing {
                        request,
                        saved: Box::new(saved),
                    }
                }
                PendingEditorLoad::Fencing { request, saved } => {
                    if request.cancelled {
                        return Ok(PendingEditorLoad::Cancelling {
                            workers: Vec::new(),
                            errors: Vec::new(),
                        });
                    }
                    if !self.restore_audio_frozen() {
                        return Ok(PendingEditorLoad::Fencing { request, saved });
                    }
                    let native_enabled = self.audio.output_mode()
                        == audio_session::AudioOutputMode::Native
                        || self.window.is_some();
                    let rate = native_enabled
                        .then(|| saved.game.audio.playback_rate())
                        .flatten();
                    if self.audio.checkpoint_output_connection()?.is_none()
                        && let Some(rate) = rate
                    {
                        self.audio
                            .begin_checkpoint_output_startup(self.window.is_some())?;
                        return Ok(PendingEditorLoad::Device {
                            request,
                            saved,
                            rate,
                            started: Instant::now(),
                        });
                    }
                    let bindings: Vec<_> = self
                        .object_ids
                        .iter()
                        .cloned()
                        .zip(self.instances.iter().copied())
                        .collect();
                    let mut registry = self.authoring.authoring_project.registry.as_ref().clone();
                    crate::register_game_checkpoint_components(&mut registry)?;
                    let registry = Arc::new(registry);
                    let policy = &request.policy;
                    let origin = self.observe_coordinated_origin(
                        &bindings,
                        &registry,
                        &policy.compatibility,
                        policy.resources,
                        policy.play,
                        policy.file.game.ui,
                    )?;
                    let output = self.audio.checkpoint_output_connection()?;
                    let behaviors = self.behavior_checkpoints.clone();
                    let ui_actions = self.ui_action_checkpoints.clone();
                    let prepare_registry = registry.clone();
                    let root = request.root.clone();
                    let policy = request.policy.clone();
                    let cache = self
                        .graphics
                        .as_ref()
                        .map_or((0, 0, 0, 0), |graphics| graphics.residency_cache.state());
                    let worker = std::thread::Builder::new()
                        .name("voxy-editor-prepare".into())
                        .spawn(move || {
                            saved
                                .prepare_guarded_with_output(
                                    CoordinatedPrepare {
                                        compatibility: &policy.compatibility,
                                        registry: &prepare_registry,
                                        behaviors: &behaviors,
                                        ui_actions: &ui_actions,
                                        root: &root,
                                        resources: policy.resources,
                                        play: policy.play,
                                        ui: policy.file.game.ui,
                                        max_audio_bytes: policy.file.game.max_audio_bytes,
                                        restored_ui_cache: cache,
                                    },
                                    policy.file.history,
                                    origin,
                                    output.as_ref(),
                                    native_enabled,
                                )
                                .map_err(|error| error.to_string())
                        })?;
                    PendingEditorLoad::Preparing {
                        context: Box::new(LoadContext {
                            request,
                            bindings,
                            registry,
                        }),
                        worker,
                    }
                }
                PendingEditorLoad::Device {
                    request,
                    saved,
                    rate,
                    started,
                } => {
                    if request.cancelled {
                        return Ok(PendingEditorLoad::Cancelling {
                            workers: Vec::new(),
                            errors: Vec::new(),
                        });
                    }
                    if started.elapsed() > Duration::from_secs(20) {
                        return Err("native restoration device startup timed out".into());
                    }
                    let identities: BTreeMap<_, _> = self
                        .instances
                        .iter()
                        .copied()
                        .zip(self.object_ids.iter().cloned())
                        .collect();
                    if !self.audio.poll_checkpoint_output_startup(
                        rate,
                        request.policy.file.game.max_audio_bytes,
                        |node| identities.get(&node).cloned(),
                    )? {
                        return Ok(PendingEditorLoad::Device {
                            request,
                            saved,
                            rate,
                            started,
                        });
                    }
                    PendingEditorLoad::Fencing { request, saved }
                }
                PendingEditorLoad::Preparing { context, worker } => {
                    if !worker.is_finished() {
                        return Ok(PendingEditorLoad::Preparing { context, worker });
                    }
                    let candidate = join_job(worker);
                    if context.request.cancelled {
                        return Ok(PendingEditorLoad::Cancelling {
                            workers: candidate
                                .map_or_else(|_| Vec::new(), |candidate| candidate.game.discard()),
                            errors: Vec::new(),
                        });
                    }
                    let candidate = candidate?;
                    let graphics = self
                        .graphics
                        .as_ref()
                        .ok_or("editor load requires graphics")?;
                    let pending = candidate.stage_gpu(crate::gpu_model::batch::UploadTarget {
                        renderer: &graphics.renderer,
                        device: &graphics.device,
                        queue: &graphics.queue,
                        cache: &graphics.residency_cache,
                        live_geometry: graphics.geometry_bytes(),
                        replaced_geometry: graphics.checkpoint_replaced_geometry(),
                    })?;
                    PendingEditorLoad::Models {
                        context,
                        device: graphics.device.clone(),
                        checks: Box::pin(pending.validate()),
                    }
                }
                PendingEditorLoad::Models {
                    context,
                    device,
                    mut checks,
                } => {
                    device
                        .poll(wgpu::PollType::Poll)
                        .map_err(|error| error.to_string())?;
                    match checks
                        .as_mut()
                        .poll(&mut Context::from_waker(Waker::noop()))
                    {
                        Poll::Pending => PendingEditorLoad::Models {
                            context,
                            device,
                            checks,
                        },
                        Poll::Ready(candidate) => {
                            if context.request.cancelled {
                                return Ok(PendingEditorLoad::Cancelling {
                                    workers: candidate.map_or_else(
                                        |_| Vec::new(),
                                        |candidate| candidate.game.discard(),
                                    ),
                                    errors: Vec::new(),
                                });
                            }
                            let pending = self.stage_editor_checkpoint_install(
                                candidate?,
                                &context.bindings,
                                &context.registry,
                                &context.request.policy.compatibility,
                            )?;
                            PendingEditorLoad::Ui {
                                context,
                                device,
                                checks: Box::pin(pending.validate()),
                            }
                        }
                    }
                }
                PendingEditorLoad::Ui {
                    context,
                    device,
                    mut checks,
                } => {
                    device
                        .poll(wgpu::PollType::Poll)
                        .map_err(|error| error.to_string())?;
                    match checks
                        .as_mut()
                        .poll(&mut Context::from_waker(Waker::noop()))
                    {
                        Poll::Pending => PendingEditorLoad::Ui {
                            context,
                            device,
                            checks,
                        },
                        Poll::Ready(candidate) => {
                            if context.request.cancelled {
                                return Ok(PendingEditorLoad::Cancelling {
                                    workers: candidate.map_or_else(
                                        |_| Vec::new(),
                                        |candidate| candidate.discard(),
                                    ),
                                    errors: Vec::new(),
                                });
                            }
                            PendingEditorLoad::Audio {
                                context,
                                candidate: Box::new(candidate?),
                            }
                        }
                    }
                }
                PendingEditorLoad::Audio {
                    context,
                    mut candidate,
                } => {
                    if context.request.cancelled {
                        return Ok(PendingEditorLoad::Cancelling {
                            workers: candidate.discard(),
                            errors: Vec::new(),
                        });
                    }
                    if !candidate.poll_audio_ready(
                        self,
                        &context.bindings,
                        &context.registry,
                        &context.request.policy.compatibility,
                    )? {
                        return Ok(PendingEditorLoad::Audio { context, candidate });
                    }
                    let retired = self.publish_editor_checkpoint_install(
                        *candidate,
                        &context.bindings,
                        &context.registry,
                        &context.request.policy.compatibility,
                    )?;
                    let (workers, errors) = retired.release();
                    self.play.simulation_time = Instant::now();
                    PendingEditorLoad::Retiring {
                        path: context.request.path,
                        workers,
                        errors,
                    }
                }
                PendingEditorLoad::Resuming { .. }
                | PendingEditorLoad::Cancelling { .. }
                | PendingEditorLoad::Retiring { .. } => {
                    unreachable!()
                }
            })
        })();
        match next {
            Ok(pending) => self.editor_load = Some(pending),
            Err(error) if fence.is_some() => {
                self.editor_load = Some(PendingEditorLoad::Resuming {
                    error: error.to_string(),
                    fence: fence.unwrap(),
                })
            }
            Err(error) => return Err(error),
        }
        Ok(None)
    }
    /// Shutdown can wait outside the frame loop; uninstalled candidates are closed.
    pub(crate) fn finish_editor_file_load(&mut self) -> Vec<String> {
        let mut errors = Vec::new();
        match self.editor_load.take() {
            Some(PendingEditorLoad::Reading { worker, .. }) => {
                if let Err(error) = join_job(worker) {
                    errors.push(error);
                }
            }
            Some(PendingEditorLoad::Preparing { worker, .. }) => match join_job(worker) {
                Ok(prepared) => {
                    for worker in prepared.game.discard() {
                        record_join(worker, &mut errors);
                    }
                }
                Err(error) => errors.push(error),
            },
            Some(PendingEditorLoad::Retiring {
                workers,
                errors: retired,
                ..
            }) => {
                errors.extend(retired);
                for worker in workers {
                    record_join(worker, &mut errors);
                }
            }
            Some(PendingEditorLoad::Audio { candidate, .. }) => {
                for worker in candidate.discard() {
                    record_join(worker, &mut errors);
                }
            }
            Some(PendingEditorLoad::Resuming { error, .. }) => errors.push(error),
            Some(PendingEditorLoad::Cancelling {
                workers,
                errors: cleanup,
            }) => {
                errors.extend(cleanup);
                for worker in workers {
                    record_join(worker, &mut errors);
                }
            }
            // Dropping owned GPU futures closes their detached IO channels.
            _ => {}
        }
        errors
    }
}
fn join_job<T>(worker: JoinHandle<Result<T, String>>) -> Result<T, String> {
    worker.join().map_err(|payload| {
        format!(
            "editor load worker panicked: {}",
            crate::panic_message(payload.as_ref())
        )
    })?
}
fn record_join(worker: JoinHandle<()>, errors: &mut Vec<String>) {
    if let Err(payload) = worker.join() {
        errors.push(format!(
            "retired load worker panicked: {}",
            crate::panic_message(payload.as_ref())
        ));
    }
}
impl PendingEditorLoad {
    fn request_mut(&mut self) -> Option<&mut Request> {
        match self {
            Self::Reading { request, .. }
            | Self::Fencing { request, .. }
            | Self::Device { request, .. } => Some(request),
            Self::Preparing { context, .. }
            | Self::Models { context, .. }
            | Self::Ui { context, .. }
            | Self::Audio { context, .. } => Some(&mut context.request),
            _ => None,
        }
    }
}
impl std::fmt::Debug for PendingEditorLoad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Reading { .. } => "Reading",
            Self::Fencing { .. } => "Fencing",
            Self::Device { .. } => "Device",
            Self::Preparing { .. } => "Preparing",
            Self::Models { .. } => "Models",
            Self::Ui { .. } => "Ui",
            Self::Audio { .. } => "Audio",
            Self::Resuming { .. } => "Resuming",
            Self::Cancelling { .. } => "Cancelling",
            Self::Retiring { .. } => "Retiring",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn drive(app: &mut App) -> Result<PathBuf, EditorError> {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if app.restore_barrier.is_some() {
                let ids: BTreeMap<_, _> = app
                    .instances
                    .iter()
                    .copied()
                    .zip(app.object_ids.iter().cloned())
                    .collect();
                app.poll_restore_barrier(|node| ids.get(&node).cloned())
                    .map_err(|error| error.to_string())?;
            }
            if let Some(path) = app.poll_editor_file_load()? {
                return Ok(path);
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    #[test]
    fn corrupt_and_oversized_reads_never_freeze_the_source_or_allow_another_job() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = project::create(&temp.0.join("project"), "Invalid load").unwrap();
        let mut app = App::from_project(&project).unwrap();
        let path = temp.0.join("bad.voxyedr");
        std::fs::write(&path, b"invalid checkpoint").unwrap();
        let source = app.scene.change_token();
        for oversized in [false, true] {
            let mut policy = save::tests::policy();
            if oversized {
                policy.file.max_bytes = 1;
            }
            app.request_editor_file_load(path.clone(), temp.0.clone(), policy.clone())
                .unwrap();
            assert!(app.restore_barrier.is_none());
            assert!(
                app.request_editor_file_load(path.clone(), temp.0.clone(), policy.clone())
                    .is_err()
            );
            assert!(app.request_editor_file_save(path.clone(), policy).is_err());
            assert!(drive(&mut app).is_err());
            assert!(
                app.editor_load.is_none()
                    && app.restore_barrier.is_none()
                    && app.resource_checkpoint.is_none()
            );
            assert_eq!(app.scene.change_token(), source);
            assert_eq!(std::fs::read(&path).unwrap(), b"invalid checkpoint");
        }
        app.stop_workers().unwrap();
    }
    #[test]
    fn prepare_failure_after_valid_read_releases_the_original_fences() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = project::create(&temp.0.join("project"), "Prepare failure").unwrap();
        let mut app = App::from_project(&project).unwrap();
        let path = temp.0.join("snapshot.voxyedr");
        let policy = save::tests::policy();
        app.request_editor_file_save(path.clone(), policy.clone())
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        while app.editor_save.is_some() {
            if app.restore_barrier.is_some() {
                app.poll_restore_barrier(|_| None).unwrap();
            }
            app.poll_editor_file_save().unwrap();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let before = std::fs::read(&path).unwrap();
        let source = app.scene.change_token();
        let mut rejected = policy;
        rejected.compatibility = "incompatible-project".into();
        app.request_editor_file_load(path.clone(), temp.0.clone(), rejected)
            .unwrap();
        let error = drive(&mut app).err().unwrap();
        assert!(error.to_string().contains("incompatible Play checkpoint"));
        assert_eq!(app.scene.change_token(), source);
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(
            app.editor_load.is_none()
                && app.restore_barrier.is_none()
                && app.resource_checkpoint.is_none()
        );
        app.stop_workers().unwrap();
    }
    #[test]
    fn cancellation_releases_only_its_own_fence_before_worker_finishes() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = project::create(&temp.0.join("project"), "Cancel prepare").unwrap();
        let mut app = App::from_project(&project).unwrap();
        app.begin_restore_barrier(1024).unwrap();
        let fence = app.restore_barrier.as_ref().unwrap().epoch.clone();
        let (release, pending) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            pending.recv_timeout(Duration::from_secs(10)).unwrap();
            Err("cancelled preparation".to_owned())
        });
        app.editor_load = Some(PendingEditorLoad::Preparing {
            context: Box::new(LoadContext {
                request: Box::new(Request {
                    path: temp.0.join("unused"),
                    root: temp.0.clone(),
                    policy: save::tests::policy(),
                    fence: Some(fence.clone()),
                    cancelled: false,
                }),
                bindings: Vec::new(),
                registry: app.authoring.authoring_project.registry.clone(),
            }),
            worker,
        });
        let source = app.scene.change_token();
        assert!(app.cancel_editor_file_load());
        assert!(!app.cancel_editor_file_load());
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.restore_barrier.is_some() {
            assert!(app.poll_editor_file_load().unwrap().is_none());
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(
            matches!(&app.editor_load, Some(PendingEditorLoad::Preparing { worker, .. }) if !worker.is_finished())
        );
        // A late cancelled result must not release a subsequent operation's fence.
        app.begin_restore_barrier(1024).unwrap();
        let foreign = app.restore_barrier.as_ref().unwrap().epoch.clone();
        assert!(!Arc::ptr_eq(&fence, &foreign));
        release.send(()).unwrap();
        while app.editor_load.is_some() {
            assert!(app.poll_editor_file_load().unwrap().is_none());
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(Arc::ptr_eq(
            &app.restore_barrier.as_ref().unwrap().epoch,
            &foreign
        ));
        assert_eq!(app.scene.change_token(), source);
        while !app.resume_restore_barrier().unwrap() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(!app.cancel_editor_file_load());
        app.stop_workers().unwrap();
    }
    #[test]
    fn cancelled_reader_never_fences_or_publishes_and_keeps_edits() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = project::create(&temp.0.join("project"), "Cancel read").unwrap();
        let mut app = App::from_project(&project).unwrap();
        let (release, pending) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            pending.recv_timeout(Duration::from_secs(10)).unwrap();
            Err("late read failure".to_owned())
        });
        app.editor_load = Some(PendingEditorLoad::Reading {
            request: Box::new(Request {
                path: temp.0.join("unused"),
                root: temp.0.clone(),
                policy: save::tests::policy(),
                fence: None,
                cancelled: false,
            }),
            worker,
        });
        let mut panels = crate::panels::Panels::new().unwrap();
        panels.load_status = app.editor_load_status();
        assert_eq!(panels.load_status.unwrap().label, "Reading scene...");
        let document = app.panel_document().unwrap();
        panels
            .build(
                &document,
                0,
                Vec2::new(1024.0, 640.0),
                false,
                None,
                0,
                false,
                InspectorMode::Transform,
                "Texture: none",
                "Prefab",
                None,
            )
            .unwrap();
        assert!(
            panels
                .regions
                .iter()
                .any(|(_, action)| *action == crate::panels::Action::CancelLoad)
        );
        assert!(!panels.regions.iter().any(|(_, action)| matches!(
            action,
            crate::panels::Action::Save | crate::panels::Action::Load
        )));
        assert!(!app.editor_load_escape(KeyCode::KeyD));
        // The presented button goes through the same action adapter as a click.
        app.panel_action(crate::panels::Action::CancelLoad).unwrap();
        assert!(!app.cancel_editor_file_load());
        let status = app.editor_load_status().unwrap();
        assert_eq!(status.label, "Cancelling load...");
        assert!(!status.cancellable);
        panels.load_status = Some(status);
        panels
            .build(
                &document,
                0,
                Vec2::new(1024.0, 640.0),
                false,
                None,
                0,
                false,
                InspectorMode::Transform,
                "Texture: none",
                "Prefab",
                None,
            )
            .unwrap();
        assert!(
            !panels
                .regions
                .iter()
                .any(|(_, action)| *action == crate::panels::Action::CancelLoad)
        );
        // Escape remains consumed throughout cleanup, even with a field draft.
        assert!(app.editor_load_escape(KeyCode::Escape));
        assert!(app.poll_editor_file_load().unwrap().is_none());
        assert!(
            matches!(&app.editor_load, Some(PendingEditorLoad::Reading { worker, .. }) if !worker.is_finished())
        );
        app.edit_key(KeyCode::KeyD).unwrap();
        let source = app.scene.change_token();
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.editor_load.is_some() {
            assert!(app.poll_editor_file_load().unwrap().is_none());
            assert!(app.restore_barrier.is_none());
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(app.scene.change_token(), source);
        assert_eq!(app.instances.len(), 1);
        assert!(app.editor_load_status().is_none());
        assert!(!app.editor_load_escape(KeyCode::Escape));
        // A stale button event after completion must have no authoring effects.
        app.panel_action(crate::panels::Action::CancelLoad).unwrap();
        assert_eq!(app.scene.change_token(), source);
        app.stop_workers().unwrap();
    }
    #[test]
    fn reader_poll_returns_without_waiting_for_a_blocked_worker() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = project::create(&temp.0.join("project"), "Blocked reader").unwrap();
        let mut app = App::from_project(&project).unwrap();
        let (release, pending) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            pending
                .recv_timeout(Duration::from_secs(5))
                .map_err(|error| error.to_string())?;
            Err("delayed read failure".to_owned())
        });
        app.editor_load = Some(PendingEditorLoad::Reading {
            request: Box::new(Request {
                path: temp.0.join("unused"),
                root: temp.0.clone(),
                policy: save::tests::policy(),
                fence: None,
                cancelled: false,
            }),
            worker,
        });
        assert!(app.poll_editor_file_load().unwrap().is_none());
        assert!(
            matches!(&app.editor_load, Some(PendingEditorLoad::Reading { worker, .. }) if !worker.is_finished())
        );
        assert!(app.restore_barrier.is_none());
        app.edit_key(KeyCode::KeyD).unwrap();
        release.send(()).unwrap();
        assert!(
            drive(&mut app)
                .err()
                .unwrap()
                .to_string()
                .contains("delayed read failure")
        );
        assert_eq!(app.instances.len(), 1);
        app.stop_workers().unwrap();
    }
}
