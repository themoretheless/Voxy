//! Detached CPU publication graph; catalog tickets and GPU commit stay with App.
#![allow(dead_code)]
use super::{EditorAsset, asset_checkpoint::*};
use crate::EditorError;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use voxy_assets::{ImportedArchiveLimits, ImportedAsset, InputError};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Publications {
    version: u32,
    references: Vec<usize>,
    resources: Vec<u8>,
    originals: Vec<Vec<u8>>,
}
pub(crate) struct PreparedPublications {
    pub publications: Vec<Arc<ImportedAsset<EditorAsset>>>,
    pub models: Vec<Arc<voxy_model::ModelAsset>>,
    pub clips: Vec<Arc<voxy_animation::AnimationClip>>,
    pub properties: Vec<voxy_animation::PropertyClip>,
}
fn invalid() -> EditorError {
    "invalid editor imported publication graph".into()
}
fn native(error: impl std::fmt::Debug) -> EditorError {
    format!("editor imported publication graph: {error:?}").into()
}
pub(crate) fn capture(
    publications: &[Arc<ImportedAsset<EditorAsset>>],
    assets: EditorAssetCheckpointLimits,
    inputs: ImportedArchiveLimits,
    max_bytes: usize,
) -> Result<Vec<u8>, EditorError> {
    capture_with_models(publications, &[], assets, inputs, max_bytes)
}
pub(crate) fn capture_with_models(
    publications: &[Arc<ImportedAsset<EditorAsset>>],
    models: &[Arc<voxy_model::ModelAsset>],
    assets: EditorAssetCheckpointLimits,
    inputs: ImportedArchiveLimits,
    max_bytes: usize,
) -> Result<Vec<u8>, EditorError> {
    capture_with_sources(publications, models, &[], &[], assets, inputs, max_bytes)
}
pub(crate) fn capture_with_sources(
    publications: &[Arc<ImportedAsset<EditorAsset>>],
    models: &[Arc<voxy_model::ModelAsset>],
    clips: &[Arc<voxy_animation::AnimationClip>],
    properties: &[voxy_animation::PropertyClip],
    assets: EditorAssetCheckpointLimits,
    inputs: ImportedArchiveLimits,
    max_bytes: usize,
) -> Result<Vec<u8>, EditorError> {
    if publications.len() > assets.max_references {
        return Err(invalid());
    }
    let mut owners = HashMap::new();
    let mut references = Vec::new();
    let mut resources = Vec::new();
    let mut originals = Vec::new();
    for publication in publications {
        let key = Arc::as_ptr(publication);
        let index = if let Some(index) = owners.get(&key) {
            *index
        } else {
            let index = resources.len();
            if index >= assets.max_assets {
                return Err(invalid());
            }
            owners.insert(key, index);
            resources.push(Arc::new(publication.value().clone()));
            originals.push(
                publication
                    .to_checkpoint_bytes(|_| Ok((index as u64).to_le_bytes().to_vec()), inputs)
                    .map_err(native)?,
            );
            index
        };
        references.push(index);
    }
    let resources =
        EditorAssetCheckpoint::capture_with_sources(&resources, models, clips, properties, assets)?
            .to_bytes(assets)?;
    let body = serde_json::to_vec(&Publications {
        version: 1,
        references,
        resources,
        originals,
    })?;
    if body.len().checked_add(40).is_none_or(|n| n > max_bytes) {
        return Err(invalid());
    }
    let mut bytes = Vec::with_capacity(40 + body.len());
    bytes.extend_from_slice(b"VOXYEIP1");
    bytes.extend_from_slice(blake3::hash(&body).as_bytes());
    bytes.extend_from_slice(&body);
    Ok(bytes)
}
pub(crate) fn prepare(
    bytes: &[u8],
    assets: EditorAssetCheckpointLimits,
    inputs: ImportedArchiveLimits,
    max_bytes: usize,
) -> Result<Vec<Arc<ImportedAsset<EditorAsset>>>, EditorError> {
    let prepared = prepare_with_models(bytes, assets, inputs, max_bytes)?;
    if !prepared.models.is_empty() || !prepared.clips.is_empty() || !prepared.properties.is_empty()
    {
        return Err("publication graph requires external model restoration".into());
    }
    Ok(prepared.publications)
}
pub(crate) fn prepare_with_models(
    bytes: &[u8],
    assets: EditorAssetCheckpointLimits,
    inputs: ImportedArchiveLimits,
    max_bytes: usize,
) -> Result<PreparedPublications, EditorError> {
    if bytes.len() < 40 || bytes.len() > max_bytes || &bytes[..8] != b"VOXYEIP1" {
        return Err(invalid());
    }
    let body = &bytes[40..];
    if blake3::hash(body).as_bytes() != &bytes[8..40] {
        return Err(invalid());
    }
    let wire: Publications = serde_json::from_slice(body)?;
    if wire.version != 1
        || wire.references.len() > assets.max_references
        || wire.originals.len() > assets.max_assets
    {
        return Err(invalid());
    }
    // Require canonical first-use IDs and reject unused publication payloads.
    let mut next = 0;
    for &index in &wire.references {
        if index > next || index >= wire.originals.len() {
            return Err(invalid());
        }
        if index == next {
            next += 1;
        }
    }
    if next != wire.originals.len() {
        return Err(invalid());
    }
    let prepared =
        EditorAssetCheckpoint::from_bytes(&wire.resources, assets)?.prepare_with_models(assets)?;
    let resources = prepared.assets;
    if resources.len() != wire.originals.len() {
        return Err(invalid());
    }
    let mut resource_owners = std::collections::HashSet::new();
    if resources
        .iter()
        .any(|resource| !resource_owners.insert(Arc::as_ptr(resource)))
    {
        return Err(invalid());
    }
    let mut publications = Vec::with_capacity(resources.len());
    for (index, (original, resource)) in wire.originals.iter().zip(&resources).enumerate() {
        let publication = ImportedAsset::from_checkpoint_bytes(
            original,
            |payload| {
                if payload != (index as u64).to_le_bytes() {
                    return Err(InputError::Read(
                        "publication resource slot mismatch".into(),
                    ));
                }
                Ok(resource.as_ref().clone())
            },
            inputs,
        )
        .map_err(native)?;
        publications.push(Arc::new(publication));
    }
    let publications = wire
        .references
        .iter()
        .map(|&i| publications[i].clone())
        .collect();
    Ok(PreparedPublications {
        publications,
        models: prepared.models,
        clips: prepared.clips,
        properties: prepared.properties,
    })
}

#[cfg(test)]
mod tests;
