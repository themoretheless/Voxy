//! Local authoring metadata; paths/receipts retain trusted host-path semantics.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Metadata {
    settings: BTreeMap<String, [u8; 32]>,
    scene_path: Option<std::path::PathBuf>,
    revision: Option<crate::scene_revision::SceneRevisionCheckpoint>,
    prefabs: Vec<String>,
    choice: usize,
    seek: Option<(ObjectId, [u32; 4])>,
    #[serde(default)]
    source: Option<Vec<u8>>,
}
#[derive(Debug)]
enum SourceRestore {
    Unspecified,
    Replace(Option<ImportedAsset<crate::prefab_authoring::AuthoredScene>>),
}
#[derive(Debug)]
pub(crate) struct PreparedMetadata {
    settings: BTreeMap<String, [u8; 32]>,
    scene_path: Option<std::path::PathBuf>,
    revision: Option<crate::scene_revision::SceneRevision>,
    prefabs: Vec<AssetId>,
    choice: usize,
    seek: Option<(NodeId, [f32; 4])>,
    source: SourceRestore,
}
impl Metadata {
    pub(super) fn capture(
        app: &App,
        view: &SceneCheckpointView<'_>,
        max: usize,
    ) -> Result<Self, EditorError> {
        Ok(Self {
            source: app
                .authoring
                .authoring_source
                .as_ref()
                .map(|value| {
                    super::source::capture(value, &app.authoring.authoring_project.registry, max)
                })
                .transpose()?,
            settings: app.authoring.settings_written.clone(),
            scene_path: app.authoring.scene_path.clone(),
            revision: app
                .authoring
                .scene_revision
                .as_ref()
                .map(crate::scene_revision::SceneRevision::checkpoint),
            prefabs: app
                .authoring
                .prefab_assets
                .iter()
                .map(|id| id.0.clone())
                .collect(),
            choice: app.authoring.prefab_choice,
            seek: app
                .authoring
                .preview_seek
                .map(|(node, rect)| {
                    Ok::<_, EditorError>((
                        view.reserved_identity(node)
                            .cloned()
                            .ok_or("unidentified pending preview owner")?,
                        rect.map(f32::to_bits),
                    ))
                })
                .transpose()?,
        })
    }
    pub(super) fn prepare(
        &self,
        view: &SceneCheckpointView<'_>,
        registry: &ComponentRegistry,
        max: usize,
        source_declared: bool,
    ) -> Result<PreparedMetadata, EditorError> {
        // The enclosing checkpoint has already enforced its full archive-byte
        // policy; asset inventory size is independent of scene draft capacity.
        if (self.prefabs.is_empty() && self.choice != 0)
            || (!self.prefabs.is_empty() && self.choice >= self.prefabs.len())
            || self.prefabs.iter().any(String::is_empty)
        {
            return Err("invalid authoring metadata budget or selection".into());
        }
        if !source_declared && self.source.is_some() {
            return Err("legacy authoring source was not declared".into());
        }
        let source = if source_declared {
            SourceRestore::Replace(
                self.source
                    .as_ref()
                    .map(|bytes| super::source::prepare(bytes, registry, max))
                    .transpose()?,
            )
        } else {
            SourceRestore::Unspecified
        };
        let revision = self
            .revision
            .as_ref()
            .map(|r| r.prepare(self.scene_path.as_deref()))
            .transpose()?;
        let seek = self
            .seek
            .as_ref()
            .map(|(id, rect)| {
                let rect = rect.map(f32::from_bits);
                if !rect.iter().all(|v| v.is_finite()) {
                    return Err(EditorError::from("invalid pending preview rectangle"));
                }
                Ok((
                    view.reserved_node(id)
                        .ok_or("missing pending preview owner")?,
                    rect,
                ))
            })
            .transpose()?;
        Ok(PreparedMetadata {
            source,
            settings: self.settings.clone(),
            scene_path: self.scene_path.clone(),
            revision,
            prefabs: self.prefabs.iter().cloned().map(AssetId).collect(),
            choice: self.choice,
            seek,
        })
    }
}
impl PreparedMetadata {
    pub(crate) fn install(self, target: &mut crate::authoring_session::AuthoringSession) {
        if let SourceRestore::Replace(source) = self.source {
            target.authoring_source = source;
        }
        target.settings_written = self.settings;
        target.scene_path = self.scene_path;
        target.scene_revision = self.revision;
        target.prefab_assets = self.prefabs;
        target.prefab_choice = self.choice;
        target.preview_seek = self.seek;
    }
}
