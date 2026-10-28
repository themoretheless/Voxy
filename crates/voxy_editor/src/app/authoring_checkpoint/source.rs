//! Original imported authoring publication, including accepted input observations.
use super::*;
use crate::prefab_authoring::AuthoredScene;
use voxy_assets::{ImportedArchiveLimits, InputError};
fn policy(max: usize) -> ImportedArchiveLimits {
    ImportedArchiveLimits {
        max_inputs: 130,
        max_input_bytes: 16 * 1024 * 1024,
        max_archive_bytes: max,
    }
}
fn invalid(error: impl std::fmt::Display) -> InputError {
    InputError::Read(error.to_string())
}
fn validate(value: &AuthoredScene, registry: &ComponentRegistry) -> Result<(), InputError> {
    let expanded = value
        .source
        .expand(registry, scene_limits::PREFABS, |id| {
            value.dependencies.get(id).cloned().ok_or_else(|| {
                voxy_scene::DocumentError::Invalid("missing archived prefab dependency".into())
            })
        })
        .map_err(invalid)?;
    if expanded.dependencies.iter().ne(value.dependencies.keys())
        || serde_json::to_vec(&expanded.document).map_err(invalid)?
            != serde_json::to_vec(&value.expanded).map_err(invalid)?
    {
        return Err(invalid("archived authoring expansion differs"));
    }
    Ok(())
}
pub(super) fn capture(
    source: &ImportedAsset<AuthoredScene>,
    registry: &ComponentRegistry,
    max: usize,
) -> Result<Vec<u8>, EditorError> {
    Ok(source
        .to_checkpoint_bytes(
            |value| {
                validate(value, registry)?;
                serde_json::to_vec(value).map_err(invalid)
            },
            policy(max),
        )
        .map_err(|e| format!("authoring source archive: {e:?}"))?)
}
pub(super) fn prepare(
    bytes: &[u8],
    registry: &ComponentRegistry,
    max: usize,
) -> Result<ImportedAsset<AuthoredScene>, EditorError> {
    Ok(ImportedAsset::from_checkpoint_bytes(
        bytes,
        |payload| {
            let value: AuthoredScene = serde_json::from_slice(payload).map_err(invalid)?;
            validate(&value, registry)?;
            Ok(value)
        },
        policy(max),
    )
    .map_err(|e| format!("authoring source archive: {e:?}"))?)
}

impl App {
    pub(crate) fn authoring_source_fingerprint(
        &self,
        max: usize,
    ) -> Result<Option<blake3::Hash>, EditorError> {
        self.authoring
            .authoring_source
            .as_ref()
            .map(|source| {
                capture(source, &self.authoring.authoring_project.registry, max)
                    .map(|bytes| blake3::hash(&bytes))
            })
            .transpose()
    }
}
