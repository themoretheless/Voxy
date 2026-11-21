//! Periodic scene autosave sidecar journal and crash-recovery detection/restoration.

use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use voxy_scene::{
    ComponentRegistry, DocumentError, HistoryChangeToken, PrefabLimits, PrefabSceneDocument,
    SceneDocument, SceneHistory,
};

const JOURNAL_VERSION: u32 = 1;
const AUTOSAVE_SUFFIX: &str = ".voxy-autosave.json";

/// Returns the deterministic sidecar journal path for an authored `.scene.json` file.
#[must_use]
pub fn autosave_path_for(scene_path: &Path) -> PathBuf {
    let file_name = scene_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("scene.json");
    scene_path.with_file_name(format!("{file_name}{AUTOSAVE_SUFFIX}"))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AutosavePayload {
    Plain {
        document: SceneDocument,
    },
    Composed {
        source: PrefabSceneDocument,
        expanded: SceneDocument,
        dependencies: BTreeMap<String, PrefabSceneDocument>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutosaveJournal {
    pub version: u32,
    pub scene_file_name: String,
    pub base_scene_blake3: Option<String>,
    pub saved_at_unix_ms: u64,
    pub payload: AutosavePayload,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PendingCrashRecovery {
    pub journal_path: PathBuf,
    pub journal: AutosaveJournal,
    pub base_matches_disk: bool,
}

impl PendingCrashRecovery {
    #[must_use]
    pub fn document(&self) -> &SceneDocument {
        match &self.journal.payload {
            AutosavePayload::Plain { document } => document,
            AutosavePayload::Composed { expanded, .. } => expanded,
        }
    }

    #[must_use]
    pub fn object_count(&self) -> usize {
        self.document().objects.len()
    }

    #[must_use]
    pub fn is_prefab_composed(&self) -> bool {
        matches!(self.journal.payload, AutosavePayload::Composed { .. })
    }
}

#[derive(Debug)]
pub(super) struct AutosaveState {
    pub saved_history_token: Option<HistoryChangeToken>,
    pub saved_content_digest: Option<[u8; 32]>,
    pub autosaved_history_token: Option<HistoryChangeToken>,
    pub last_autosave: Instant,
    pub interval: Duration,
    pub pending_recovery: Option<PendingCrashRecovery>,
    pub recovery_error: Option<String>,
}

impl Default for AutosaveState {
    fn default() -> Self {
        Self::from_env()
    }
}

impl AutosaveState {
    #[must_use]
    pub fn from_env() -> Self {
        let interval = std::env::var("VOXY_AUTOSAVE_INTERVAL_MS")
            .ok()
            .and_then(|raw| raw.parse::<u64>().ok())
            .map_or_else(
                || Duration::from_secs(5),
                |ms| Duration::from_millis(ms.clamp(10, 3_600_000)),
            );
        Self {
            saved_history_token: None,
            saved_content_digest: None,
            autosaved_history_token: None,
            last_autosave: Instant::now(),
            interval,
            pending_recovery: None,
            recovery_error: None,
        }
    }
}

pub(crate) fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

pub(crate) fn file_blake3_hex(path: &Path, max_bytes: usize) -> Result<Option<String>, String> {
    use std::io::Read as _;
    if !path.is_file() {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| format!("open {}: {e}", path.display()))?
        .take((max_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    if bytes.len() > max_bytes {
        return Err(format!("scene file exceeds {max_bytes} bytes"));
    }
    Ok(Some(blake3::hash(&bytes).to_hex().to_string()))
}

pub(crate) fn state_digest(history: Option<&SceneHistory>) -> Option<[u8; 32]> {
    let history = history?;
    let bytes = serde_json::to_vec(&(history.current(), history.metadata())).ok()?;
    Some(*blake3::hash(&bytes).as_bytes())
}

pub(crate) fn capture_payload(
    document: SceneDocument,
    metadata: &serde_json::Value,
    registry: &ComponentRegistry,
) -> Result<AutosavePayload, Box<dyn std::error::Error>> {
    if metadata.is_null() {
        return Ok(AutosavePayload::Plain { document });
    }
    let snapshot: prefab_authoring::AuthoredScene = serde_json::from_value(metadata.clone())?;
    let baseline =
        snapshot
            .source
            .instance_baseline(registry, scene_limits::PREFABS, |asset| {
                snapshot.dependencies.get(asset).cloned().ok_or_else(|| {
                    DocumentError::Invalid(format!("missing historical prefab {asset}"))
                })
            })?;
    let source =
        snapshot
            .source
            .capture_edits(&baseline, &document, registry, scene_limits::OBJECTS)?;
    Ok(AutosavePayload::Composed {
        source,
        expanded: document,
        dependencies: snapshot.dependencies,
    })
}

pub(crate) fn write_journal(
    scene_path: &Path,
    journal: &AutosaveJournal,
    max_bytes: usize,
) -> Result<PathBuf, String> {
    let journal_path = autosave_path_for(scene_path);
    let bytes = serde_json::to_vec_pretty(journal).map_err(|e| format!("encode autosave: {e}"))?;
    voxy_assets::save_atomic_file(&journal_path, &bytes, max_bytes)
        .map_err(|e| format!("write autosave {}: {e}", journal_path.display()))?;
    Ok(journal_path)
}

pub(crate) fn remove_journal(scene_path: &Path) -> Result<bool, String> {
    let journal_path = autosave_path_for(scene_path);
    match std::fs::remove_file(&journal_path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "remove autosave {}: {error}",
            journal_path.display()
        )),
    }
}

pub(crate) fn inspect_journal(
    scene_path: &Path,
    registry: &ComponentRegistry,
    max_objects: usize,
    prefab_limits: PrefabLimits,
    max_bytes: usize,
) -> Result<Option<PendingCrashRecovery>, String> {
    use std::io::Read as _;
    let journal_path = autosave_path_for(scene_path);
    if !journal_path.is_file() {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    std::fs::File::open(&journal_path)
        .map_err(|e| format!("open autosave {}: {e}", journal_path.display()))?
        .take((max_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("read autosave {}: {e}", journal_path.display()))?;
    if bytes.len() > max_bytes {
        return Err(format!("autosave journal exceeds {max_bytes} bytes"));
    }
    let text = std::str::from_utf8(&bytes).map_err(|e| format!("autosave UTF-8: {e}"))?;
    let journal: AutosaveJournal =
        serde_json::from_str(text).map_err(|e| format!("parse autosave: {e}"))?;
    if journal.version != JOURNAL_VERSION {
        return Err(format!("unsupported autosave version {}", journal.version));
    }
    match &journal.payload {
        AutosavePayload::Plain { document } => {
            document
                .load(registry, max_objects)
                .map_err(|e| format!("validate autosave scene: {e}"))?;
            if let Ok(disk_doc) =
                voxy_scene::read_scene_file_migrated(scene_path, max_bytes, registry)
                    .map(|(doc, _)| doc)
                && &disk_doc == document
            {
                let _ = remove_journal(scene_path);
                return Ok(None);
            }
        }
        AutosavePayload::Composed {
            source,
            expanded,
            dependencies,
        } => {
            let reexpanded = source
                .expand(registry, prefab_limits, |asset| {
                    dependencies.get(asset).cloned().ok_or_else(|| {
                        DocumentError::Invalid(format!("missing autosave prefab {asset}"))
                    })
                })
                .map_err(|e| format!("expand autosave prefab scene: {e}"))?;
            if &reexpanded.document != expanded {
                return Err("autosave composed scene does not match expanded snapshot".into());
            }
            expanded
                .load(registry, max_objects)
                .map_err(|e| format!("validate autosave composed scene: {e}"))?;
            if let Ok((disk_prefab, _)) =
                voxy_scene::read_prefab_scene_file_migrated(scene_path, max_bytes, registry)
                && &disk_prefab == source
            {
                let _ = remove_journal(scene_path);
                return Ok(None);
            }
        }
    }
    let disk_digest = file_blake3_hex(scene_path, max_bytes)?;
    let base_matches_disk = journal.base_scene_blake3 == disk_digest;
    Ok(Some(PendingCrashRecovery {
        journal_path,
        journal,
        base_matches_disk,
    }))
}

impl App {
    pub(crate) fn sync_saved_autosave_baseline(&mut self) {
        let token = self
            .authoring
            .history
            .as_ref()
            .map(SceneHistory::change_token);
        let digest = state_digest(self.authoring.history.as_ref());
        self.authoring
            .autosave
            .saved_history_token
            .clone_from(&token);
        self.authoring.autosave.saved_content_digest = digest;
        self.authoring.autosave.autosaved_history_token = token;
        self.authoring.autosave.last_autosave = Instant::now();
    }

    pub(crate) fn detect_crash_recovery_on_open(&mut self) {
        self.authoring.autosave.pending_recovery = None;
        self.authoring.autosave.recovery_error = None;
        if self.standalone || self.model_package.is_some() {
            return;
        }
        let Some(scene_path) = &self.authoring.scene_path else {
            return;
        };
        match inspect_journal(
            scene_path,
            &self.authoring.authoring_project.registry,
            scene_limits::OBJECTS,
            scene_limits::PREFABS,
            scene_limits::DOCUMENT_BYTES,
        ) {
            Ok(recovery) => {
                self.authoring.autosave.pending_recovery = recovery;
                self.panel_cache = None;
            }
            Err(error) => {
                eprintln!("VOXY AUTOSAVE CORRUPT: {error}");
                self.authoring.autosave.recovery_error = Some(error);
                self.panel_cache = None;
            }
        }
    }

    pub(crate) fn poll_autosave(&mut self) {
        if self.standalone
            || self.model_package.is_some()
            || self.play.playing.is_some()
            || self.drag.is_some()
            || self.restore_barrier.is_some()
            || self.resource_checkpoint.is_some()
            || self.authoring.autosave.pending_recovery.is_some()
            || self.smoke_deadline.is_some()
        {
            return;
        }
        let Some(scene_path) = self.authoring.scene_path.clone() else {
            return;
        };
        let Some(history) = &self.authoring.history else {
            return;
        };
        let current_token = history.change_token();
        if self.authoring.autosave.saved_history_token.as_ref() == Some(&current_token) {
            return;
        }
        let current_digest = state_digest(Some(history));
        if current_digest.is_some()
            && current_digest == self.authoring.autosave.saved_content_digest
        {
            let _ = remove_journal(&scene_path);
            self.authoring.autosave.saved_history_token = Some(current_token.clone());
            self.authoring.autosave.autosaved_history_token = Some(current_token);
            self.authoring.autosave.last_autosave = Instant::now();
            return;
        }
        if self.authoring.autosave.autosaved_history_token.as_ref() == Some(&current_token) {
            return;
        }
        if self.authoring.autosave.last_autosave.elapsed() < self.authoring.autosave.interval {
            return;
        }
        if let Err(error) = self.flush_autosave_now() {
            eprintln!("VOXY AUTOSAVE FAILED: {error}");
            self.authoring.autosave.last_autosave = Instant::now();
        }
    }

    pub(crate) fn flush_autosave_now(
        &mut self,
    ) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
        if self.standalone || self.model_package.is_some() {
            return Ok(None);
        }
        let Some(scene_path) = self.authoring.scene_path.clone() else {
            return Ok(None);
        };
        let history = self.authoring.history.as_ref().ok_or("missing history")?;
        let token = history.change_token();
        let edited = if self.play.playing.is_some() || self.drag.is_some() {
            history.current().clone()
        } else {
            self.authoring_document()?
        };
        let registry = &self.authoring.authoring_project.registry;
        let payload = capture_payload(edited, history.metadata(), registry)?;
        let base_scene_blake3 = file_blake3_hex(&scene_path, scene_limits::DOCUMENT_BYTES)
            .map_err(EditorError::from)?;
        let scene_file_name = scene_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("scene.json")
            .to_owned();
        let journal = AutosaveJournal {
            version: JOURNAL_VERSION,
            scene_file_name,
            base_scene_blake3,
            saved_at_unix_ms: current_unix_ms(),
            payload,
        };
        let written = write_journal(&scene_path, &journal, scene_limits::DOCUMENT_BYTES)
            .map_err(EditorError::from)?;
        self.authoring.autosave.autosaved_history_token = Some(token);
        self.authoring.autosave.last_autosave = Instant::now();
        Ok(Some(written))
    }

    pub(crate) fn recover_autosave(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_some() {
            return Err("stop Play mode before recovering autosave".into());
        }
        self.finish_drag(false)?;
        let Some(recovery) = self.authoring.autosave.pending_recovery.take() else {
            return Err("no pending crash recovery journal".into());
        };
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let registry = self.authoring.authoring_project.registry.clone();
            let document = recovery.document().clone();
            let next_id = self.validate_authoring_document_policy(&document, true)?;
            match recovery.journal.payload {
                AutosavePayload::Plain { document } => {
                    self.authoring
                        .history
                        .as_mut()
                        .ok_or("missing history")?
                        .commit(document, &registry)?;
                }
                AutosavePayload::Composed {
                    source,
                    expanded,
                    dependencies,
                } => {
                    let metadata = serde_json::to_value(prefab_authoring::AuthoredScene {
                        source,
                        expanded: expanded.clone(),
                        dependencies,
                    })?;
                    self.authoring
                        .history
                        .as_mut()
                        .ok_or("missing history")?
                        .commit_with_metadata(expanded, metadata, &registry)?;
                }
            }
            self.restore_authoring()?;
            self.authoring.next_object_id = self.authoring.next_object_id.max(next_id);
            let token = self
                .authoring
                .history
                .as_ref()
                .map(SceneHistory::change_token);
            self.authoring.autosave.autosaved_history_token = token;
            self.authoring.autosave.recovery_error = None;
            self.panel_cache = None;
            if let Some(window) = &self.window {
                window.set_title("Voxy — crash autosave recovered");
            }
            Ok(())
        })();
        if result.is_err() {
            // Retain recovery journal if validation/installation failed.
            if let Some(scene_path) = &self.authoring.scene_path {
                self.authoring.autosave.pending_recovery = inspect_journal(
                    scene_path,
                    &self.authoring.authoring_project.registry,
                    scene_limits::OBJECTS,
                    scene_limits::PREFABS,
                    scene_limits::DOCUMENT_BYTES,
                )
                .ok()
                .flatten();
            }
        }
        result
    }

    pub(crate) fn discard_autosave(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.authoring.autosave.pending_recovery = None;
        self.authoring.autosave.recovery_error = None;
        if let Some(scene_path) = &self.authoring.scene_path {
            remove_journal(scene_path).map_err(EditorError::from)?;
        }
        self.panel_cache = None;
        self.update_edit_title();
        Ok(())
    }

    pub(crate) fn autosave_notice(&self) -> Option<panels::ResourceNotice> {
        if let Some(recovery) = &self.authoring.autosave.pending_recovery {
            let detail = if recovery.base_matches_disk {
                format!(
                    "Unsaved session found ({} objects{})",
                    recovery.object_count(),
                    if recovery.is_prefab_composed() {
                        ", prefab"
                    } else {
                        ""
                    }
                )
            } else {
                format!(
                    "Disk file changed since autosave ({} objects)",
                    recovery.object_count()
                )
            };
            return Some(panels::ResourceNotice {
                lines: [
                    format!("Crash recovery: {}", recovery.journal.scene_file_name),
                    detail,
                    "Click Recover (or F11) to restore, or Discard".into(),
                ],
                retry: false,
                relink: false,
                editing: false,
                recovery: true,
                discard_recovery: true,
            });
        }
        if let Some(error) = &self.authoring.autosave.recovery_error {
            return Some(panels::ResourceNotice {
                lines: [
                    "Corrupt autosave journal detected".into(),
                    error.chars().take(120).collect(),
                    "Click Discard to remove the corrupt journal".into(),
                ],
                retry: false,
                relink: false,
                editing: false,
                recovery: false,
                discard_recovery: true,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_scene_autosave_survives_simulated_crash_recovers_undoes_and_cleans_on_save() {
        let temp = crate::project_prefab_tests::Temp::new();
        let proj = crate::project::create(&temp.0.join("proj"), "AutosavePlain").unwrap();
        let scene_path = temp.0.join("proj/main.scene.json");

        {
            let mut app = App::from_project(&proj).unwrap();
            app.authoring.autosave.interval = Duration::ZERO;
            app.edit_key(KeyCode::KeyD).unwrap();
            let count_after_edit = app.instances.len();
            assert_eq!(count_after_edit, 1);
            // Poll tick triggers atomic sidecar autosave without saving main.scene.json.
            app.tick().unwrap();
            let sidecar = autosave_path_for(&scene_path);
            assert!(sidecar.is_file());
            // Drop without CloseRequested or save_authoring -> simulates crash!
            app.stop_workers().unwrap();
        }

        // Reopen project: crash recovery must be detected automatically.
        let mut reopened = App::from_project(&proj).unwrap();
        assert_eq!(reopened.instances.len(), 0);
        assert!(reopened.authoring.autosave.pending_recovery.is_some());
        let notice = reopened.selected_model_notice().unwrap();
        assert!(notice.recovery && notice.discard_recovery);

        // Recover (equivalent to F11 / Recover button), then verify Undo -> Redo -> Save.
        reopened.edit_key(KeyCode::F11).unwrap();
        assert!(reopened.authoring.autosave.pending_recovery.is_none());
        assert_eq!(reopened.instances.len(), 1);

        // Undo brings back the on-disk clean state, redo brings back recovered state.
        reopened.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(reopened.instances.len(), 0);
        reopened.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(reopened.instances.len(), 1);

        // Saving commits recovered scene to main.scene.json and clears the sidecar.
        reopened.save_authoring().unwrap();
        assert!(!autosave_path_for(&scene_path).exists());
        reopened.stop_workers().unwrap();

        let mut clean = App::from_project(&proj).unwrap();
        assert_eq!(clean.instances.len(), 1);
        assert!(clean.authoring.autosave.pending_recovery.is_none());
        clean.stop_workers().unwrap();
    }

    #[test]
    fn composed_prefab_autosave_recovers_without_flattening_and_corrupt_sidecar_discards() {
        let temp = crate::project_prefab_tests::Temp::new();
        let proj = crate::project::create(&temp.0.join("proj"), "AutosavePrefab").unwrap();
        let root = temp.0.join("proj");
        std::fs::write(
            root.join("assets.json"),
            r#"{"version":1,"assets":[{"asset":"prop","source":"prop.prefab"}]}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("prop.prefab"),
            serde_json::to_string_pretty(&serde_json::json!({
                "version": 1,
                "objects": [{
                    "id": "root",
                    "parent": null,
                    "name": "Prop Root",
                    "active": true,
                    "translation": [0.0, 0.0, 0.0],
                    "rotation": [0.0, 0.0, 0.0, 1.0],
                    "scale": [1.0, 1.0, 1.0],
                    "components": {}
                }],
                "instances": []
            }))
            .unwrap(),
        )
        .unwrap();
        let main_scene = root.join("main.scene.json");
        std::fs::write(
            &main_scene,
            serde_json::to_string_pretty(&serde_json::json!({
                "version": 1,
                "objects": [],
                "instances": [{
                    "id": "inst-1",
                    "asset": "prop",
                    "parent": null,
                    "overrides": {}
                }]
            }))
            .unwrap(),
        )
        .unwrap();

        {
            let mut app = App::from_project(&proj).unwrap();
            app.authoring.autosave.interval = Duration::ZERO;
            assert!(!app.authoring.history.as_ref().unwrap().metadata().is_null());
            app.edit_key(KeyCode::ArrowRight).unwrap();
            app.tick().unwrap();
            assert!(autosave_path_for(&main_scene).is_file());
            app.stop_workers().unwrap();
        }

        let mut reopened = App::from_project(&proj).unwrap();
        assert!(
            reopened
                .authoring
                .autosave
                .pending_recovery
                .as_ref()
                .is_some_and(PendingCrashRecovery::is_prefab_composed)
        );
        reopened.recover_autosave().unwrap();
        reopened.save_authoring().unwrap();
        assert!(!autosave_path_for(&main_scene).exists());
        let saved_prefab: PrefabSceneDocument =
            serde_json::from_str(&std::fs::read_to_string(&main_scene).unwrap()).unwrap();
        assert_eq!(saved_prefab.instances.len(), 1);
        assert!(!saved_prefab.instances[0].overrides.is_empty());
        reopened.stop_workers().unwrap();

        // Corrupt sidecar must not block opening and must be discardable.
        std::fs::write(autosave_path_for(&main_scene), b"{corrupt-json").unwrap();
        let mut with_corrupt = App::from_project(&proj).unwrap();
        assert!(with_corrupt.authoring.autosave.pending_recovery.is_none());
        assert!(with_corrupt.authoring.autosave.recovery_error.is_some());
        let notice = with_corrupt.selected_model_notice().unwrap();
        assert!(!notice.recovery && notice.discard_recovery);
        with_corrupt.discard_autosave().unwrap();
        assert!(!autosave_path_for(&main_scene).exists());
        assert!(with_corrupt.authoring.autosave.recovery_error.is_none());
        with_corrupt.stop_workers().unwrap();
    }
}
