//! Shared engine MCP session state, path safety, and outcome envelopes.

use super::play_package::PlayWorker;
use crate::{EditorCamera, project::OpenProject, scene_limits};
use serde_json::{Value, json};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use voxy_mcp::ContentBlock;
use voxy_scene::{ComponentRegistry, SceneDocument, SceneHistory};

pub(crate) const MAX_DOC_BYTES: usize = scene_limits::DOCUMENT_BYTES;
pub(crate) const SCENE_CAPACITY: usize = scene_limits::OBJECTS;

#[derive(Debug)]
pub(crate) struct ProjectSession {
    pub project: OpenProject,
    pub scene_path: PathBuf,
    pub scene_rel: PathBuf,
    pub history: SceneHistory,
    pub saved_scene: SceneDocument,
    pub disk_bytes: Vec<u8>,
    pub is_prefab_composed: bool,
    pub revision: u64,
    pub camera_override: Option<EditorCamera>,
}

impl ProjectSession {
    pub(crate) fn current_scene(&self) -> &SceneDocument {
        self.history.current()
    }

    pub(crate) fn dirty(&self) -> bool {
        self.history.current() != &self.saved_scene
    }

    pub(crate) fn scene_hash(&self) -> Result<String, String> {
        let bytes = serde_json::to_vec(self.history.current()).map_err(|e| e.to_string())?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }

    pub(crate) fn manifest_path(&self) -> PathBuf {
        self.project
            .root
            .join(&self.project.descriptor.asset_manifest)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RenderedFrame {
    pub png: Vec<u8>,
}

pub(crate) struct HeadlessGpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl std::fmt::Debug for HeadlessGpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeadlessGpu").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) struct EngineState {
    pub registry: Arc<ComponentRegistry>,
    pub project: Option<ProjectSession>,
    pub play: Option<PlayWorker>,
    pub latest_render: Option<RenderedFrame>,
    pub gpu: Option<HeadlessGpu>,
    pub capture_seq: u64,
}

impl EngineState {
    pub(crate) fn new(registry: Arc<ComponentRegistry>) -> Self {
        Self {
            registry,
            project: None,
            play: None,
            latest_render: None,
            gpu: None,
            capture_seq: 0,
        }
    }

    pub(crate) fn require_project(&self) -> Result<&ProjectSession, String> {
        self.project
            .as_ref()
            .ok_or_else(|| "no project open (call project_open or project_create first)".to_owned())
    }

    pub(crate) fn require_project_mut(&mut self) -> Result<&mut ProjectSession, String> {
        self.project
            .as_mut()
            .ok_or_else(|| "no project open (call project_open or project_create first)".to_owned())
    }

    pub(crate) fn require_play(&self) -> Result<&PlayWorker, String> {
        self.play
            .as_ref()
            .ok_or_else(|| "no play session running (call play_start first)".to_owned())
    }
}

#[derive(Debug)]
pub(crate) struct ToolOutcome {
    pub structured: Value,
    pub extra_content: Vec<ContentBlock>,
    pub updated_uris: Vec<&'static str>,
    pub list_changed: bool,
}

impl ToolOutcome {
    pub(crate) fn ok(structured: Value) -> Self {
        Self {
            structured,
            extra_content: Vec::new(),
            updated_uris: Vec::new(),
            list_changed: false,
        }
    }

    pub(crate) fn with_updates(mut self, uris: &[&'static str]) -> Self {
        self.updated_uris.extend_from_slice(uris);
        self
    }

    pub(crate) fn with_list_changed(mut self) -> Self {
        self.list_changed = true;
        self
    }

    pub(crate) fn with_block(mut self, block: ContentBlock) -> Self {
        self.extra_content.push(block);
        self
    }
}

pub(crate) fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    use std::io::Read as _;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| format!("open {}: {e}", path.display()))?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    if bytes.len() > limit {
        return Err(format!("{} exceeds {limit} bytes", path.display()));
    }
    Ok(bytes)
}

pub(crate) fn resolve_project_rel(
    root: &Path,
    rel: &str,
    must_exist: bool,
) -> Result<PathBuf, String> {
    let rel_path = Path::new(rel);
    if rel_path.as_os_str().is_empty()
        || rel_path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!(
            "expected a normalized project-relative path: {rel}"
        ));
    }
    let candidate = root.join(rel_path);
    if must_exist {
        let canonical = candidate
            .canonicalize()
            .map_err(|e| format!("resolve {}: {e}", rel_path.display()))?;
        if !canonical.starts_with(root) {
            return Err(format!("path escapes project root: {rel}"));
        }
        return Ok(canonical);
    }
    Ok(candidate)
}

pub(crate) fn json_merge_patch(target: &mut Value, patch: Value) {
    if let Value::Object(patch_map) = patch {
        if !target.is_object() {
            *target = json!({});
        }
        if let Value::Object(target_map) = target {
            for (k, v) in patch_map {
                if v.is_null() {
                    target_map.remove(&k);
                } else {
                    let entry = target_map.entry(k).or_insert(Value::Null);
                    json_merge_patch(entry, v);
                }
            }
        }
    } else {
        *target = patch;
    }
}
