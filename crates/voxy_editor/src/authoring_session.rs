//! Owns authoring project, persistent history and source revisions.
use super::{prefab_authoring, scene_revision};
use std::collections::BTreeMap;
use voxy_assets::{AssetId, ImportedAsset};
use voxy_scene::SceneHistory;
#[derive(Debug)]
pub(super) struct AuthoringSession {
    pub(super) history: Option<SceneHistory>,
    pub(super) settings_written: BTreeMap<String, [u8; 32]>,
    pub(super) scene_path: Option<std::path::PathBuf>,
    pub(super) scene_revision: Option<scene_revision::SceneRevision>,
    pub(super) authoring_project: prefab_authoring::AuthoringProject,
    pub(super) prefab_assets: Vec<AssetId>,
    pub(super) prefab_choice: usize,
    pub(super) authoring_source: Option<ImportedAsset<prefab_authoring::AuthoredScene>>,
    pub(super) next_object_id: u64,
}

impl AuthoringSession {
    pub(super) fn step_history(&mut self, undo: bool) -> Result<bool, &'static str> {
        let history = self.history.as_mut().ok_or("missing history")?;
        Ok(if undo { history.undo() } else { history.redo() })
    }
}
