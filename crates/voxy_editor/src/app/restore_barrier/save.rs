//! Ordered snapshot capture and background atomic file writing. Frame polling never
//! waits for the writer; source fences resume once the immutable snapshot is owned.
use super::*;
use editor::EditorFileLimits;
use std::{path::PathBuf, thread::JoinHandle};
#[derive(Clone)]
pub(crate) struct EditorSavePolicy {
    pub compatibility: String,
    pub resources: crate::app::resource_checkpoint::RestoreLimits,
    pub play: crate::play_session::checkpoint::PlayCheckpointLimits,
    pub file: EditorFileLimits,
}
pub(crate) enum PendingEditorSave {
    Capturing {
        path: PathBuf,
        policy: Box<EditorSavePolicy>,
        fence: Arc<()>,
    },
    Writing {
        path: PathBuf,
        worker: JoinHandle<Result<(), String>>,
        fence: Arc<()>,
    },
    Resuming {
        result: Result<PathBuf, String>,
        fence: Arc<()>,
    },
}
impl App {
    pub(crate) fn editor_file_status(&self) -> Option<super::load::LoadStatus> {
        self.editor_load_status().or_else(|| {
            self.editor_save
                .as_ref()
                .map(|pending| super::load::LoadStatus {
                    label: match pending {
                        PendingEditorSave::Capturing { .. } => "Capturing snapshot...",
                        PendingEditorSave::Writing { .. } => "Writing snapshot...",
                        PendingEditorSave::Resuming { .. } => "Finishing save...",
                    },
                    cancellable: false,
                })
        })
    }

    pub(crate) fn request_editor_file_save(
        &mut self,
        path: PathBuf,
        policy: EditorSavePolicy,
    ) -> Result<(), EditorError> {
        if self.editor_save.is_some()
            || self.editor_load.is_some()
            || policy.compatibility.is_empty()
        {
            return Err("editor save is already pending or compatibility is empty".into());
        }
        self.begin_restore_barrier(policy.file.game.max_audio_bytes)?;
        self.snapshot_error = None;
        self.panel_cache = None;
        self.editor_save = Some(PendingEditorSave::Capturing {
            path,
            policy: Box::new(policy),
            fence: self.restore_barrier.as_ref().unwrap().epoch.clone(),
        });
        Ok(())
    }
    /// Invoked after the ordinary frame has progressed the original restore fence.
    pub(crate) fn poll_editor_file_save(&mut self) -> Result<Option<PathBuf>, EditorError> {
        let Some(pending) = self.editor_save.take() else {
            return Ok(None);
        };
        self.editor_save = Some(match pending {
            PendingEditorSave::Capturing {
                path,
                policy,
                fence,
            } => {
                if !self
                    .restore_barrier
                    .as_ref()
                    .is_some_and(|barrier| Arc::ptr_eq(&barrier.epoch, &fence))
                {
                    self.editor_save = Some(PendingEditorSave::Resuming {
                        result: Err("editor save barrier changed before capture".into()),
                        fence,
                    });
                    return Ok(None);
                }
                if !self.restore_audio_frozen() {
                    self.editor_save = Some(PendingEditorSave::Capturing {
                        path,
                        policy,
                        fence,
                    });
                    return Ok(None);
                }
                let capture = (|| {
                    let mut registry = self.authoring.authoring_project.registry.as_ref().clone();
                    crate::register_game_checkpoint_components(&mut registry)?;
                    let view = voxy_scene::SceneCheckpointView::new(
                        &self.scene,
                        self.object_ids
                            .iter()
                            .cloned()
                            .zip(self.instances.iter().copied()),
                    )?;
                    self.editor_checkpoint(
                        &view,
                        &registry,
                        &policy.compatibility,
                        policy.resources,
                        policy.play,
                        policy.file.game.ui,
                        policy.file.history,
                    )
                })();
                match capture {
                    Err(error) => PendingEditorSave::Resuming {
                        result: Err(error.to_string()),
                        fence,
                    },
                    Ok(saved) => {
                        let destination = path.clone();
                        let limits = policy.file;
                        match std::thread::Builder::new()
                            .name("voxy-editor-save".into())
                            .spawn(move || {
                                saved
                                    .save_file(&destination, limits)
                                    .map_err(|error| error.to_string())
                            }) {
                            Ok(worker) => PendingEditorSave::Writing {
                                path,
                                worker,
                                fence,
                            },
                            Err(error) => PendingEditorSave::Resuming {
                                result: Err(error.to_string()),
                                fence,
                            },
                        }
                    }
                }
            }
            PendingEditorSave::Writing {
                path,
                worker,
                fence,
            } => {
                // The worker owns all snapshot bytes. Release only our boundary;
                // serialization, fsync and rename must not pause the live game.
                if let Err(error) = self.resume_editor_save_fence(&fence) {
                    self.editor_save = Some(PendingEditorSave::Writing {
                        path,
                        worker,
                        fence,
                    });
                    return Err(error);
                }
                if !worker.is_finished() {
                    self.editor_save = Some(PendingEditorSave::Writing {
                        path,
                        worker,
                        fence,
                    });
                    return Ok(None);
                }
                PendingEditorSave::Resuming {
                    result: join_writer(worker).map(|()| path),
                    fence,
                }
            }
            PendingEditorSave::Resuming { result, fence } => {
                match self.resume_editor_save_fence(&fence) {
                    Ok(true) => return result.map(Some).map_err(Into::into),
                    Ok(false) => {}
                    Err(error) => {
                        self.editor_save = Some(PendingEditorSave::Resuming { result, fence });
                        return Err(error);
                    }
                }
                PendingEditorSave::Resuming { result, fence }
            }
        });
        Ok(None)
    }
    fn resume_editor_save_fence(&mut self, fence: &Arc<()>) -> Result<bool, EditorError> {
        if !self
            .restore_barrier
            .as_ref()
            .is_some_and(|barrier| Arc::ptr_eq(&barrier.epoch, fence))
        {
            // An absent or newer barrier belongs to another operation.
            return Ok(true);
        }
        self.resume_restore_barrier()
    }
    /// Shutdown is outside frame-critical polling: finish any already-started write.
    pub(crate) fn finish_editor_file_save(&mut self) -> Vec<String> {
        match self.editor_save.take() {
            Some(PendingEditorSave::Writing { worker, .. }) => {
                join_writer(worker).err().into_iter().collect()
            }
            Some(PendingEditorSave::Resuming {
                result: Err(error), ..
            }) => vec![error],
            Some(PendingEditorSave::Capturing { .. }) => {
                vec!["editor save cancelled before snapshot capture".into()]
            }
            _ => Vec::new(),
        }
    }
}
fn join_writer(worker: JoinHandle<Result<(), String>>) -> Result<(), String> {
    worker.join().map_err(|payload| {
        format!(
            "editor save worker panicked: {}",
            crate::panic_message(payload.as_ref())
        )
    })?
}

impl std::fmt::Debug for PendingEditorSave {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Capturing { path, .. } => f.debug_tuple("Capturing").field(path).finish(),
            Self::Writing { path, .. } => f.debug_tuple("Writing").field(path).finish(),
            Self::Resuming { result, .. } => f.debug_tuple("Resuming").field(result).finish(),
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct UserComponent {
        exact: u64,
    }
    pub(crate) fn policy() -> EditorSavePolicy {
        let resources = crate::play_session::checkpoint::tests::resource_restore_limits();
        let mut play = crate::play_session::checkpoint::tests::limits();
        play.core.scene_capacity = scene_limits::OBJECTS;
        let ui = crate::ui_live::checkpoint::tests::limits();
        EditorSavePolicy {
            compatibility: "editor-save-job-v1".into(),
            resources,
            play,
            file: EditorFileLimits {
                max_bytes: 512 * 1024 * 1024,
                game: crate::app::restore_barrier::file::CoordinatedFileLimits {
                    max_bytes: 384 * 1024 * 1024,
                    max_game_bytes: resources.max_bytes,
                    ui,
                    max_audio_bytes: 8 * 1024 * 1024,
                    max_input_bytes: 100000,
                },
                history: voxy_scene::HistoryCheckpointLimits {
                    max_versions: 64,
                    max_history_bytes: scene_limits::HISTORY_BYTES,
                    max_scene_capacity: scene_limits::OBJECTS,
                    max_archive_bytes: 128 * 1024 * 1024,
                    max_edits: 4,
                    max_draft_objects: scene_limits::OBJECTS * 2,
                },
            },
        }
    }
    fn advance(app: &mut App) -> Result<Option<PathBuf>, EditorError> {
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
        app.poll_editor_file_save()
    }
    fn finish(app: &mut App) -> Result<PathBuf, EditorError> {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(path) = advance(app)? {
                return Ok(path);
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    #[test]
    fn background_save_preserves_custom_components_and_releases_all_fences() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = project::create(&temp.0.join("project"), "Background save").unwrap();
        let mut registry = editor_component_registry().unwrap();
        registry.register::<UserComponent>("game.user.v1").unwrap();
        let mut app = App::from_project_with_registry(&project, registry).unwrap();
        app.edit_key(KeyCode::KeyD).unwrap();
        app.scene
            .insert_component(
                app.instances[0],
                UserComponent {
                    exact: 9_007_199_254_740_993,
                },
            )
            .unwrap();
        app.commit_authoring().unwrap();
        let source_registry = app.authoring.authoring_project.registry.clone();
        let path = temp.0.join("snapshot.voxyedr");
        let policy = policy();
        app.request_editor_file_save(path.clone(), policy.clone())
            .unwrap();
        assert!(
            app.request_editor_file_save(path.clone(), policy.clone())
                .is_err()
        );
        assert_eq!(finish(&mut app).unwrap(), path);
        assert!(
            app.editor_save.is_none()
                && app.restore_barrier.is_none()
                && app.resource_checkpoint.is_none()
        );
        assert!(Arc::ptr_eq(
            &source_registry,
            &app.authoring.authoring_project.registry
        ));
        let saved = editor::EditorCheckpoint::read_file(&path, policy.file).unwrap();
        let mut registry = source_registry.as_ref().clone();
        crate::register_game_checkpoint_components(&mut registry).unwrap();
        let registry = Arc::new(registry);
        let target = temp.0.join("target");
        std::fs::create_dir(&target).unwrap();
        let prepared = saved
            .prepare(
                CoordinatedPrepare {
                    compatibility: &policy.compatibility,
                    registry: &registry,
                    behaviors: &app.behavior_checkpoints,
                    ui_actions: &app.ui_action_checkpoints,
                    root: &target,
                    resources: policy.resources,
                    play: policy.play,
                    ui: policy.file.game.ui,
                    max_audio_bytes: policy.file.game.max_audio_bytes,
                    restored_ui_cache: (0, 0, 0, 0),
                },
                policy.file.history,
            )
            .unwrap();
        let scene = &prepared.game.game.as_ref().unwrap().game.scene;
        let owner = scene.resolve(&app.object_ids[0]).unwrap();
        assert_eq!(
            scene
                .graph
                .component::<UserComponent>(owner)
                .unwrap()
                .unwrap()
                .exact,
            9_007_199_254_740_993
        );
        for worker in prepared.game.discard() {
            worker.join().unwrap();
        }
        app.stop_workers().unwrap();
    }
    #[test]
    fn failed_capture_and_failed_writer_preserve_existing_file_and_resume_source() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = project::create(&temp.0.join("project"), "Failed background save").unwrap();
        let mut app = App::from_project(&project).unwrap();
        let path = temp.0.join("snapshot.voxyedr");
        std::fs::write(&path, b"prior file").unwrap();
        let scene = app.scene.change_token();
        for capture in [true, false] {
            let mut limits = policy();
            if capture {
                limits.play.core.max_bytes = 1;
            } else {
                limits.file.max_bytes = 1;
            }
            app.request_editor_file_save(path.clone(), limits).unwrap();
            assert!(finish(&mut app).is_err());
            assert_eq!(std::fs::read(&path).unwrap(), b"prior file");
            assert_eq!(app.scene.change_token(), scene);
            assert!(
                app.editor_save.is_none()
                    && app.restore_barrier.is_none()
                    && app.resource_checkpoint.is_none()
            );
        }
        app.stop_workers().unwrap();
    }
    #[test]
    fn frame_poll_returns_while_the_file_worker_is_deliberately_blocked() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = project::create(&temp.0.join("project"), "Blocked writer").unwrap();
        let mut app = App::from_project(&project).unwrap();
        app.toggle_play().unwrap();
        app.advance_game(0.05).unwrap();
        let source_ticks = app.play.simulation_ticks;
        let path = temp.0.join("snapshot.voxyedr");
        app.request_editor_file_save(path.clone(), EditorSavePolicy::desktop())
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        while !matches!(app.editor_save, Some(PendingEditorSave::Writing { .. })) {
            advance(&mut app).unwrap();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let Some(PendingEditorSave::Writing { worker, fence, .. }) = app.editor_save.take() else {
            unreachable!()
        };
        join_writer(worker).unwrap();
        let (release, pending) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            pending
                .recv_timeout(Duration::from_secs(5))
                .map_err(|error| error.to_string())
        });
        app.editor_save = Some(PendingEditorSave::Writing {
            path: path.clone(),
            worker,
            fence,
        });
        // A model-release failure must retain both the writer and audio fence.
        let checkpoint = app
            .resource_checkpoint
            .replace(crate::app::resource_checkpoint::Barrier::Requested);
        let watcher = app.watcher.take();
        let error = app.poll_editor_file_save().unwrap_err();
        assert!(error.to_string().contains("model watcher disappeared"));
        assert!(app.restore_play_frozen());
        assert!(app.restore_barrier.as_ref().unwrap().audio);
        assert!(
            matches!(&app.editor_save, Some(PendingEditorSave::Writing { worker, .. }) if !worker.is_finished())
        );
        app.resource_checkpoint = checkpoint;
        app.watcher = watcher;
        assert!(app.poll_editor_file_save().unwrap().is_none());
        assert!(
            matches!(&app.editor_save, Some(PendingEditorSave::Writing { worker, .. }) if !worker.is_finished())
        );
        while app.restore_barrier.is_some() {
            app.poll_editor_file_save().unwrap();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(app.resource_checkpoint.is_none());
        app.advance_game(0.05).unwrap();
        assert!(app.play.simulation_ticks > source_ticks);
        // Authoring remains usable while the file worker is still blocked.
        app.edit_key(KeyCode::KeyD).unwrap();
        assert!(
            matches!(&app.editor_save, Some(PendingEditorSave::Writing { worker, .. }) if !worker.is_finished())
        );
        release.send(()).unwrap();
        assert_eq!(finish(&mut app).unwrap(), path);
        assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
        app.stop_workers().unwrap();
    }
    #[test]
    fn late_save_stages_never_release_a_newer_restore_barrier() {
        for stage in 0..3 {
            let temp = crate::project_prefab_tests::Temp::new();
            let project = project::create(&temp.0.join("project"), "Save fence ownership").unwrap();
            let mut app = App::from_project(&project).unwrap();
            let path = temp.0.join("snapshot.voxyedr");
            app.request_editor_file_save(path.clone(), policy())
                .unwrap();
            let Some(PendingEditorSave::Capturing { fence, policy, .. }) = app.editor_save.take()
            else {
                unreachable!()
            };
            let deadline = Instant::now() + Duration::from_secs(15);
            while !app.resume_restore_barrier().unwrap() {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
            app.begin_restore_barrier(policy.file.game.max_audio_bytes)
                .unwrap();
            let newer = app.restore_barrier.as_ref().unwrap().epoch.clone();
            app.editor_save = Some(match stage {
                0 => PendingEditorSave::Capturing {
                    path,
                    policy,
                    fence,
                },
                1 => PendingEditorSave::Writing {
                    path,
                    worker: std::thread::spawn(|| Ok(())),
                    fence,
                },
                _ => PendingEditorSave::Resuming {
                    result: Ok(path),
                    fence,
                },
            });
            let result = loop {
                let result = app.poll_editor_file_save();
                assert!(Arc::ptr_eq(
                    &app.restore_barrier.as_ref().unwrap().epoch,
                    &newer
                ));
                assert!(app.resource_checkpoint.is_some());
                if !matches!(result, Ok(None)) {
                    break result;
                }
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            };
            if stage == 0 {
                assert!(
                    result
                        .unwrap_err()
                        .to_string()
                        .contains("barrier changed before capture")
                );
            } else {
                assert!(result.unwrap().is_some());
            }
            assert!(app.editor_save.is_none());
            while !app.resume_restore_barrier().unwrap() {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
            app.stop_workers().unwrap();
        }
    }
    #[test]
    fn native_idle_empty_project_saves_before_and_during_play_without_a_device() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = project::create(&temp.0.join("project"), "Native idle project").unwrap();
        let mut app = App::from_project(&project).unwrap();
        app.audio
            .set_output_mode(audio_session::AudioOutputMode::Native);
        let policy = EditorSavePolicy::desktop();
        for playing in [false, true] {
            if playing {
                app.toggle_play().unwrap();
                app.advance_game(0.05).unwrap();
            }
            let path = temp.0.join(format!("native-idle-{playing}.voxyedr"));
            app.request_editor_file_save(path.clone(), policy.clone())
                .unwrap();
            assert_eq!(finish(&mut app).unwrap(), path);
            let saved = editor::EditorCheckpoint::read_file(&path, policy.file).unwrap();
            let audio: serde_json::Value = serde_json::from_str(
                &saved
                    .game
                    .audio
                    .to_json(policy.file.game.max_audio_bytes)
                    .unwrap(),
            )
            .unwrap();
            assert!(audio["play"].is_null());
            assert_eq!(
                app.audio.output_mode(),
                audio_session::AudioOutputMode::Native
            );
            assert!(app.audio.checkpoint_output_connection().unwrap().is_none());
            assert_eq!(app.play.playing.is_some(), playing);
            assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
        }
        app.stop_workers().unwrap();
    }
    #[test]
    fn shutdown_finishes_a_started_atomic_write() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = project::create(&temp.0.join("project"), "Shutdown save").unwrap();
        let mut app = App::from_project(&project).unwrap();
        let path = temp.0.join("snapshot.voxyedr");
        let policy = policy();
        app.request_editor_file_save(path.clone(), policy.clone())
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        while !matches!(app.editor_save, Some(PendingEditorSave::Writing { .. })) {
            advance(&mut app).unwrap();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        app.stop_workers().unwrap();
        editor::EditorCheckpoint::read_file(&path, policy.file).unwrap();
    }
}
