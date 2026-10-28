//! Quiescent model catalog and original dependency observations, staged off-line.
#![allow(dead_code)]
use super::{EditorAsset, asset_checkpoint::EditorAssetCheckpointLimits, publication_checkpoint};
use crate::{EditorError, asset_checkpoint_wire as wire};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use voxy_assets::{
    AssetCatalog, AssetCatalogCheckpoint, ImportedArchiveLimits, ImportedAsset, SourceDependencies,
    SourceDependenciesCheckpoint,
};
#[derive(Clone, Copy)]
pub(crate) struct Limits {
    pub assets: EditorAssetCheckpointLimits,
    pub inputs: ImportedArchiveLimits,
    pub max_bytes: usize,
    pub max_catalog_assets: usize,
    pub max_pending: usize,
    pub max_source_outputs: usize,
    pub max_source_edges: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    catalog: wire::CatalogWire,
    #[serde(with = "wire::sources")]
    sources: SourceDependenciesCheckpoint,
    publications: Vec<u8>,
    #[serde(default, skip_serializing_if = "zero")]
    retained: usize,
}
#[allow(clippy::trivially_copy_pass_by_ref)] // Serde skip predicate borrows its field.
fn zero(value: &usize) -> bool {
    *value == 0
}
pub(crate) struct Prepared {
    pub catalog: AssetCatalog<ImportedAsset<EditorAsset>>,
    pub sources: SourceDependencies,
    pub models: Vec<Arc<voxy_model::ModelAsset>>,
    pub clips: Vec<Arc<voxy_animation::AnimationClip>>,
    pub properties: Vec<voxy_animation::PropertyClip>,
    pub retained: Vec<Arc<ImportedAsset<EditorAsset>>>,
}
fn bad() -> EditorError {
    "invalid model catalog checkpoint".into()
}
fn native(e: impl std::fmt::Debug) -> EditorError {
    format!("model catalog checkpoint: {e:?}").into()
}
pub(crate) fn capture(
    catalog: &AssetCatalog<ImportedAsset<EditorAsset>>,
    sources: &SourceDependencies,
    limits: Limits,
) -> Result<Vec<u8>, EditorError> {
    capture_with_models(catalog, sources, &[], limits)
}
pub(crate) fn capture_with_models(
    catalog: &AssetCatalog<ImportedAsset<EditorAsset>>,
    sources: &SourceDependencies,
    models: &[Arc<voxy_model::ModelAsset>],
    limits: Limits,
) -> Result<Vec<u8>, EditorError> {
    capture_with_sources(catalog, sources, models, &[], &[], limits)
}
pub(crate) fn capture_with_sources(
    catalog: &AssetCatalog<ImportedAsset<EditorAsset>>,
    sources: &SourceDependencies,
    models: &[Arc<voxy_model::ModelAsset>],
    clips: &[Arc<voxy_animation::AnimationClip>],
    properties: &[voxy_animation::PropertyClip],
    limits: Limits,
) -> Result<Vec<u8>, EditorError> {
    capture_with_retained(catalog, sources, models, clips, properties, &[], limits)
}
pub(crate) fn capture_with_retained(
    catalog: &AssetCatalog<ImportedAsset<EditorAsset>>,
    sources: &SourceDependencies,
    models: &[Arc<voxy_model::ModelAsset>],
    clips: &[Arc<voxy_animation::AnimationClip>],
    properties: &[voxy_animation::PropertyClip],
    retained: &[Arc<ImportedAsset<EditorAsset>>],
    limits: Limits,
) -> Result<Vec<u8>, EditorError> {
    let mut owners = HashMap::new();
    let mut publications = Vec::new();
    let saved = catalog
        .checkpoint(|publication| {
            let next = publications.len();
            let index = *owners.entry(Arc::as_ptr(publication)).or_insert_with(|| {
                publications.push(publication.clone());
                next
            });
            Some(index.to_string())
        })
        .map_err(native)?;
    let sources = sources.checkpoint();
    validate_join(&saved, &sources, &publications)?;
    // Admission applies to recorded policies, not merely current entry counts.
    saved
        .prepare(
            |key| {
                key.parse::<usize>()
                    .ok()
                    .and_then(|i| publications.get(i).cloned())
            },
            limits.max_catalog_assets,
            limits.max_pending,
        )
        .map_err(native)?;
    sources
        .prepare(limits.max_source_outputs, limits.max_source_edges)
        .map_err(native)?;
    if publications
        .len()
        .checked_add(retained.len())
        .is_none_or(|n| n > limits.assets.max_references)
    {
        return Err(bad());
    }
    publications.extend(retained.iter().cloned());
    let body = serde_json::to_vec(&Record {
        version: 1,
        catalog: saved.try_into()?,
        sources,
        retained: retained.len(),
        publications: publication_checkpoint::capture_with_sources(
            &publications,
            models,
            clips,
            properties,
            limits.assets,
            limits.inputs,
            limits.max_bytes,
        )?,
    })?;
    if body
        .len()
        .checked_add(40)
        .is_none_or(|n| n > limits.max_bytes)
    {
        return Err(bad());
    }
    let mut bytes = b"VOXYEIC1".to_vec();
    bytes.extend_from_slice(blake3::hash(&body).as_bytes());
    bytes.extend_from_slice(&body);
    Ok(bytes)
}
pub(crate) fn prepare(bytes: &[u8], limits: Limits) -> Result<Prepared, EditorError> {
    if bytes.len() < 40 || bytes.len() > limits.max_bytes || &bytes[..8] != b"VOXYEIC1" {
        return Err(bad());
    }
    let body = &bytes[40..];
    if blake3::hash(body).as_bytes() != &bytes[8..40] {
        return Err(bad());
    }
    let saved: Record = serde_json::from_slice(body)?;
    if saved.version != 1 {
        return Err(bad());
    }
    if saved.retained > limits.assets.max_references {
        return Err(bad());
    }
    let catalog: AssetCatalogCheckpoint = saved.catalog.into();
    if catalog.max_assets > limits.max_catalog_assets
        || catalog.max_pending > limits.max_pending
        || catalog.entries.len() > catalog.max_assets
    {
        return Err(bad());
    }
    let sources = saved
        .sources
        .prepare(limits.max_source_outputs, limits.max_source_edges)
        .map_err(native)?;
    let prepared = publication_checkpoint::prepare_with_models(
        &saved.publications,
        limits.assets,
        limits.inputs,
        limits.max_bytes,
    )?;
    let mut publications = prepared.publications;
    let split = publications
        .len()
        .checked_sub(saved.retained)
        .ok_or_else(bad)?;
    let retained = publications.split_off(split);
    validate_join(&catalog, &saved.sources, &publications)?;
    let catalog = catalog
        .prepare(
            |key| {
                key.parse::<usize>()
                    .ok()
                    .and_then(|i| publications.get(i).cloned())
            },
            limits.max_catalog_assets,
            limits.max_pending,
        )
        .map_err(native)?;
    Ok(Prepared {
        catalog,
        sources,
        models: prepared.models,
        clips: prepared.clips,
        properties: prepared.properties,
        retained,
    })
}
fn validate_join(
    catalog: &AssetCatalogCheckpoint,
    sources: &SourceDependenciesCheckpoint,
    publications: &[Arc<ImportedAsset<EditorAsset>>],
) -> Result<(), EditorError> {
    let mut used = std::collections::HashSet::new();
    let mut next = 0;
    for entry in &catalog.entries {
        let reads = sources.outputs.iter().find(|s| s.output == entry.asset);
        if let Some(key) = &entry.publication {
            let index: usize = key.parse().map_err(|_| bad())?;
            if index.to_string() != *key || index > next {
                return Err(bad());
            }
            let publication = publications.get(index).ok_or_else(bad)?;
            if used.insert(index) {
                next += 1;
            }
            let expected: Vec<_> = publication
                .inputs()
                .observations()
                .keys()
                .cloned()
                .collect();
            if reads.is_none_or(|s| s.published != expected) {
                return Err(bad());
            }
        } else if reads.is_some_and(|s| !s.published.is_empty()) {
            return Err(bad());
        }
    }
    if used.len() != publications.len()
        || sources
            .outputs
            .iter()
            .any(|s| !catalog.entries.iter().any(|e| e.asset == s.output))
    {
        return Err(bad());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
