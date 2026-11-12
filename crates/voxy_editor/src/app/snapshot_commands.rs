//! Explicit full-state snapshot commands, separate from authored scene files.
use crate::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SnapshotCommand {
    Save,
    Open,
}
#[derive(Debug)]
pub(crate) struct SnapshotDraft {
    command: SnapshotCommand,
    path: String,
    select_all: bool,
    scene: voxy_scene::SceneId,
}
impl App {
    pub(crate) fn snapshot_escape(&mut self, key: KeyCode) -> bool {
        if self.editor_load_escape(key) {
            return true;
        }
        if key == KeyCode::Escape
            && self
                .snapshot_error
                .take_if(|_| self.snapshot_draft.is_none())
                .is_some()
        {
            self.panel_cache = None;
            self.update_edit_title();
            return true;
        }
        false
    }

    pub(crate) fn report_snapshot_error(&mut self, error: impl std::fmt::Display) {
        let error = error.to_string();
        eprintln!("SNAPSHOT OPERATION FAILED: {error}");
        if let Some(window) = &self.window {
            window.set_title(&format!("Voxy — snapshot: {error}"));
        }
        self.snapshot_error = Some(error);
        self.panel_cache = None;
    }
    pub(crate) fn poll_snapshot_load_command(&mut self) {
        match self.poll_editor_file_load() {
            Ok(Some(path)) => println!("EDITOR SNAPSHOT LOADED {}", path.display()),
            Ok(None) => {}
            Err(error) => self.report_snapshot_error(error),
        }
    }
    pub(crate) fn poll_snapshot_save_command(&mut self) {
        match self.poll_editor_file_save() {
            Ok(Some(path)) => println!("EDITOR SNAPSHOT SAVED {}", path.display()),
            Ok(None) => {}
            Err(error) => self.report_snapshot_error(error),
        }
    }
    pub(crate) fn snapshot_failure_notice(&self) -> Option<panels::ResourceNotice> {
        let error = self.snapshot_error.as_ref()?;
        Some(panels::ResourceNotice {
            lines: [
                "Snapshot operation failed".into(),
                error.chars().take(160).collect(),
                "Esc dismisses; Ctrl/Cmd+Shift+S saves, +O opens".into(),
            ],
            retry: false,
            relink: false,
            editing: false,
        })
    }

    pub(crate) fn begin_snapshot_path(
        &mut self,
        command: SnapshotCommand,
    ) -> Result<(), EditorError> {
        if self.standalone
            || self.editor_load.is_some()
            || self.editor_save.is_some()
            || self.restore_barrier.is_some()
            || self.resource_checkpoint.is_some()
            || self.retarget_draft.is_some()
            || self.model_relink_draft.is_some()
            || self.field.is_some()
        {
            return Err("finish the current edit or file operation first".into());
        }
        let path = self
            .authoring
            .scene_path
            .as_ref()
            .and_then(|path| path.file_stem())
            .and_then(|stem| stem.to_str())
            .map_or_else(
                || "snapshot.voxyedr".into(),
                |stem| format!("{stem}.voxyedr"),
            );
        self.snapshot_draft = Some(SnapshotDraft {
            command,
            path,
            select_all: true,
            scene: self.scene.identity(),
        });
        self.panel_cache = None;
        self.update_edit_title();
        Ok(())
    }
    pub(crate) fn snapshot_shortcut(&mut self, key: KeyCode) -> Result<bool, EditorError> {
        if self.standalone
            || !self.modifiers.shift_key()
            || !(self.modifiers.super_key() || self.modifiers.control_key())
        {
            return Ok(false);
        }
        let command = match key {
            KeyCode::KeyS => SnapshotCommand::Save,
            KeyCode::KeyO => SnapshotCommand::Open,
            _ => return Ok(false),
        };
        self.begin_snapshot_path(command)?;
        Ok(true)
    }
    pub(crate) fn snapshot_path_key(
        &mut self,
        key: KeyCode,
        text: Option<&str>,
    ) -> Result<(), EditorError> {
        let Some(draft) = &mut self.snapshot_draft else {
            return Ok(());
        };
        if key == KeyCode::Escape || draft.scene != self.scene.identity() {
            self.snapshot_draft = None;
            self.panel_cache = None;
            self.update_edit_title();
            return Ok(());
        }
        if key == KeyCode::KeyA && (self.modifiers.super_key() || self.modifiers.control_key()) {
            draft.select_all = true;
            self.panel_cache = None;
            return Ok(());
        }
        if key == KeyCode::Enter {
            let path = std::path::Path::new(&draft.path);
            if path
                .extension()
                .is_none_or(|extension| extension != "voxyedr")
            {
                return Err("snapshot path must end with .voxyedr".into());
            }
            let root = self.authoring.authoring_project.root().to_owned();
            let path = if path.is_absolute() {
                path.to_owned()
            } else {
                root.join(path)
            };
            let command = draft.command;
            let policy = app::restore_barrier::save::EditorSavePolicy::desktop();
            match command {
                SnapshotCommand::Save => self.request_editor_file_save(path, policy)?,
                SnapshotCommand::Open => self.request_editor_file_load(path, root, policy)?,
            }
            self.snapshot_draft = None;
        } else if key == KeyCode::Backspace {
            if draft.select_all {
                draft.path.clear();
            } else {
                draft.path.pop();
            }
            draft.select_all = false;
        } else if let Some(text) = text {
            let retained = if draft.select_all {
                0
            } else {
                draft.path.len()
            };
            if text.chars().any(char::is_control) || retained.saturating_add(text.len()) > 4096 {
                return Err(
                    "snapshot path contains control characters or exceeds 4096 bytes".into(),
                );
            }
            if draft.select_all {
                draft.path.clear();
            }
            draft.select_all = false;
            draft.path.push_str(text);
        }
        self.panel_cache = None;
        self.update_edit_title();
        Ok(())
    }
    pub(crate) fn snapshot_path_notice(&self) -> Option<panels::ResourceNotice> {
        let draft = self.snapshot_draft.as_ref()?;
        Some(panels::ResourceNotice {
            lines: [
                match draft.command {
                    SnapshotCommand::Save => "Save full snapshot",
                    SnapshotCommand::Open => "Open full snapshot",
                }
                .into(),
                format!("Path: {}|", draft.path),
                "Project-relative or absolute .voxyedr; Enter confirms, Esc cancels".into(),
            ],
            retry: false,
            relink: false,
            editing: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shortcut_path_admission_and_background_save_use_desktop_policy() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = project::create(&temp.0.join("project"), "Snapshot commands").unwrap();
        let mut app = App::from_project(&project).unwrap();
        let source = app.scene.change_token();
        app.modifiers =
            winit::keyboard::ModifiersState::CONTROL | winit::keyboard::ModifiersState::SHIFT;
        app.authoring_input_key(KeyCode::KeyS, None).unwrap();
        assert!(app.snapshot_path_notice().unwrap().lines[0].contains("Save"));
        assert!(app.editor_save.is_none());
        app.snapshot_draft.as_mut().unwrap().path = "project.voxy".into();
        assert!(app.authoring_input_key(KeyCode::Enter, None).is_err());
        assert!(app.editor_save.is_none() && app.restore_barrier.is_none());
        app.snapshot_draft.as_mut().unwrap().path = "state.voxyedr".into();
        let before = app.snapshot_draft.as_ref().unwrap().path.clone();
        assert!(app.snapshot_path_key(KeyCode::Period, Some("\n")).is_err());
        assert_eq!(app.snapshot_draft.as_ref().unwrap().path, before);
        app.authoring_input_key(KeyCode::Enter, None).unwrap();
        assert!(app.snapshot_draft.is_none() && app.editor_save.is_some());
        let deadline = Instant::now() + Duration::from_secs(15);
        while app.editor_save.is_some() {
            if app.restore_barrier.is_some() {
                app.poll_restore_barrier(|_| None).unwrap();
            }
            app.poll_editor_file_save().unwrap();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let path = project.parent().unwrap().join("state.voxyedr");
        assert!(path.is_file());
        let policy = app::restore_barrier::save::EditorSavePolicy::desktop();
        let saved =
            app::restore_barrier::editor::EditorCheckpoint::read_file(&path, policy.file).unwrap();
        assert!(!saved.to_bytes(policy.file).unwrap().is_empty());
        assert_eq!(app.scene.change_token(), source);
        app.authoring_input_key(KeyCode::KeyO, None).unwrap();
        app.authoring_input_key(KeyCode::Escape, None).unwrap();
        assert!(app.snapshot_draft.is_none() && app.editor_load.is_none());
        assert_eq!(app.scene.change_token(), source);
        app.stop_workers().unwrap();
    }
    fn settle(app: &mut App) {
        let deadline = Instant::now() + Duration::from_secs(15);
        while app.editor_save.is_some() || app.editor_load.is_some() {
            if app.restore_barrier.is_some() {
                let ids: BTreeMap<_, _> = app
                    .instances
                    .iter()
                    .copied()
                    .zip(app.object_ids.iter().cloned())
                    .collect();
                app.poll_restore_barrier(|node| ids.get(&node).cloned())
                    .unwrap();
            }
            app.poll_snapshot_save_command();
            app.poll_snapshot_load_command();
            assert!(
                app.error.is_none(),
                "file failure leaked into fatal runtime state"
            );
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    #[test]
    fn command_failures_keep_editor_alive_and_allow_edit_save_and_retry() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = project::create(&temp.0.join("project"), "Recoverable errors").unwrap();
        let mut app = App::from_project(&project).unwrap();
        let bad = temp.0.join("corrupt.voxyedr");
        std::fs::write(&bad, b"corrupt snapshot").unwrap();
        let source = app.scene.change_token();
        let policy = app::restore_barrier::save::EditorSavePolicy::desktop();
        app.request_editor_file_load(bad.clone(), temp.0.clone(), policy.clone())
            .unwrap();
        settle(&mut app);
        assert!(app.snapshot_error.is_some());
        assert!(app.selected_model_notice().unwrap().lines[0].contains("failed"));
        assert_eq!(app.scene.change_token(), source);
        assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
        assert_eq!(std::fs::read(&bad).unwrap(), b"corrupt snapshot");
        app.authoring_input_key(KeyCode::Escape, None).unwrap();
        assert!(app.snapshot_error.is_none());
        assert_eq!(app.scene.change_token(), source);
        // Authoring is still usable after the failed read.
        app.edit_key(KeyCode::KeyD).unwrap();
        let edited = app.scene.change_token();
        let missing_parent = temp.0.join("missing/snapshot.voxyedr");
        app.request_editor_file_save(missing_parent.clone(), policy.clone())
            .unwrap();
        assert!(app.snapshot_error.is_none());
        settle(&mut app);
        assert!(app.snapshot_error.is_some());
        assert!(app.error.is_none());
        assert_eq!(app.scene.change_token(), edited);
        assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
        assert!(!missing_parent.exists());
        let good = temp.0.join("good.voxyedr");
        app.request_editor_file_save(good.clone(), policy.clone())
            .unwrap();
        settle(&mut app);
        assert!(good.is_file() && app.snapshot_error.is_none() && app.error.is_none());
        assert_eq!(app.scene.change_token(), edited);
        // The original failure does not suppress a later independent diagnostic.
        app.request_editor_file_load(bad, temp.0.clone(), policy)
            .unwrap();
        settle(&mut app);
        assert!(app.snapshot_error.is_some());
        assert!(app.error.is_none());
        app.stop_workers().unwrap();
    }
    #[test]
    fn path_entry_stays_inert_after_scene_replacement_or_cancel() {
        let temp = crate::project_prefab_tests::Temp::new();
        let project = project::create(&temp.0.join("project"), "Path identity").unwrap();
        let mut app = App::from_project(&project).unwrap();
        app.begin_snapshot_path(SnapshotCommand::Open).unwrap();
        app.snapshot_path_key(KeyCode::Backspace, None).unwrap();
        app.snapshot_path_key(KeyCode::KeyA, Some("й")).unwrap();
        assert!(app.snapshot_draft.as_ref().unwrap().path.ends_with('й'));
        app.snapshot_path_key(KeyCode::Backspace, None).unwrap();
        assert!(!app.snapshot_draft.as_ref().unwrap().path.ends_with('й'));
        assert!(app.begin_snapshot_path(SnapshotCommand::Save).is_ok());
        app.snapshot_draft.as_mut().unwrap().scene = voxy_scene::SceneGraph::new(1).identity();
        app.snapshot_path_key(KeyCode::Enter, None).unwrap();
        assert!(
            app.snapshot_draft.is_none() && app.editor_load.is_none() && app.editor_save.is_none()
        );
        app.stop_workers().unwrap();
    }
}
