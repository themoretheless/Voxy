//! Complete CPU EditorAsset graph. Import observations/catalog generations and
//! native CPU/GPU publication belong to the enclosing App transaction.
#![allow(dead_code)]
use super::node_checkpoint::{ImportedNodesCheckpoint, NodeCheckpointLimits};
use super::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use voxy_model::{ModelAsset, ModelAssetCheckpoint, ModelAssetCheckpointLimits};
use voxy_render::{
    CertifiedLodCheckpoint, GeometryArchiveLimits, LodArchiveLimits, SceneMeshCheckpoint,
    SkinnedLodCheckpoint, SkinnedLodCheckpointLimits, SkinnedLodMesh, SkinnedMeshSetCheckpoint,
    SkinnedMeshSetLimits,
};
#[derive(Clone, Copy, Debug)]
pub(crate) struct EditorAssetCheckpointLimits {
    pub max_bytes: usize,
    pub max_metadata_bytes: usize,
    pub max_assets: usize,
    pub max_references: usize,
    pub max_nodes: usize,
    pub max_images: usize,
    pub max_pixel_bytes: u64,
    pub max_vertices: usize,
    pub max_indices: usize,
    pub geometry: GeometryArchiveLimits,
    pub models: ModelAssetCheckpointLimits,
    pub nodes: NodeCheckpointLimits,
    pub lod: LodArchiveLimits,
    pub skinned: SkinnedLodCheckpointLimits,
    pub skins: SkinnedMeshSetLimits,
}
impl Default for EditorAssetCheckpointLimits {
    fn default() -> Self {
        Self {
            max_bytes: 512 * 1024 * 1024,
            max_metadata_bytes: 16 * 1024 * 1024,
            max_assets: 1024,
            max_references: 4096,
            max_nodes: 65536,
            max_images: 4096,
            max_pixel_bytes: 256 * 1024 * 1024,
            max_vertices: 4_000_000,
            max_indices: 12_000_000,
            geometry: GeometryArchiveLimits::default(),
            models: ModelAssetCheckpointLimits::default(),
            nodes: NodeCheckpointLimits::default(),
            lod: LodArchiveLimits {
                bytes: 128 * 1024 * 1024,
                positions: 1_000_000,
                levels: 64,
                indices: 3_000_000,
                cells: 1_000_000,
            },
            skinned: SkinnedLodCheckpointLimits::default(),
            skins: SkinnedMeshSetLimits {
                geometry: GeometryArchiveLimits {
                    max_vertices: 4_000_000,
                    max_indices: 12_000_000,
                    ..GeometryArchiveLimits::default()
                },
                max_meshes: 8192,
                max_vertex_buffers: 8192,
                max_index_buffers: 8192,
                max_groups: 8192,
                max_native_vertices: 8_000_000,
                max_native_indices: 24_000_000,
            },
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Asset {
    mesh: usize,
    nodes: usize,
    model: Option<usize>,
    lod: Option<usize>,
    skinned: Option<usize>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    chunk: usize,
    mesh: usize,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    references: Vec<usize>,
    assets: Vec<Asset>,
    lods: Vec<usize>,
    sources: Vec<Source>,
    allocations: Vec<usize>,
    mesh_arcs: Vec<usize>,
    model_skins: usize,
    #[serde(default, skip_serializing_if = "zero")]
    external_models: usize,
}
// Serde's skip predicate receives the serialized field by reference.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn zero(value: &usize) -> bool {
    *value == 0
}
pub(crate) struct PreparedEditorAssets {
    pub assets: Vec<Arc<EditorAsset>>,
    pub models: Vec<Arc<ModelAsset>>,
    pub clips: Vec<Arc<voxy_animation::AnimationClip>>,
    pub properties: Vec<voxy_animation::PropertyClip>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EditorAssetCheckpoint {
    manifest: Manifest,
    chunks: Vec<Vec<u8>>,
}
fn bad() -> EditorError {
    "invalid editor asset graph checkpoint".into()
}
fn cap() -> EditorError {
    "editor asset graph checkpoint exceeds policy".into()
}
fn native(e: impl std::fmt::Display) -> EditorError {
    format!("editor asset graph: {e}").into()
}
fn add(v: &mut usize, n: usize, max: usize) -> Result<(), EditorError> {
    *v = v.checked_add(n).ok_or_else(cap)?;
    if *v > max {
        return Err(cap());
    }
    Ok(())
}
fn rooted(ids: impl Iterator<Item = usize>, n: usize) -> Result<(), EditorError> {
    let mut seen = vec![false; n];
    let mut next = 0;
    for id in ids {
        let slot = seen.get_mut(id).ok_or_else(bad)?;
        if !*slot {
            if id != next {
                return Err(bad());
            }
            *slot = true;
            next += 1;
        }
    }
    if next != n {
        return Err(bad());
    }
    Ok(())
}
struct Chunks {
    values: Vec<Vec<u8>>,
    bytes: usize,
}
impl Chunks {
    fn push(
        &mut self,
        bytes: Vec<u8>,
        limits: EditorAssetCheckpointLimits,
    ) -> Result<usize, EditorError> {
        add(&mut self.bytes, bytes.len(), limits.max_bytes)?;
        add(&mut self.bytes, 8, limits.max_bytes)?;
        let id = self.values.len();
        self.values.push(bytes);
        Ok(id)
    }
}
#[derive(Default)]
struct Totals {
    nodes: usize,
    images: usize,
    pixels: u64,
    vertices: usize,
    indices: usize,
}
impl Totals {
    fn admit(
        &mut self,
        asset: &EditorAsset,
        limits: EditorAssetCheckpointLimits,
    ) -> Result<(), EditorError> {
        add(&mut self.nodes, asset.nodes.len(), limits.max_nodes)?;
        add(&mut self.images, asset.images.len(), limits.max_images)?;
        for image in &asset.images {
            self.pixels = self
                .pixels
                .checked_add(image.rgba().len() as u64)
                .ok_or_else(cap)?;
            if self.pixels > limits.max_pixel_bytes {
                return Err(cap());
            }
        }
        for mesh in
            std::iter::once(&asset.mesh).chain(asset.nodes.iter().filter_map(|n| n.mesh.as_ref()))
        {
            add(
                &mut self.vertices,
                mesh.vertices().len(),
                limits.max_vertices,
            )?;
            add(&mut self.indices, mesh.indices().len(), limits.max_indices)?;
        }
        Ok(())
    }
}
fn metadata(m: &Manifest, max: usize) -> Result<Vec<u8>, EditorError> {
    struct Writer {
        bytes: Vec<u8>,
        limit: usize,
    }
    impl std::io::Write for Writer {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            if self
                .bytes
                .len()
                .checked_add(b.len())
                .is_none_or(|n| n > self.limit)
            {
                return Err(std::io::Error::other("asset metadata byte cap"));
            }
            self.bytes.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut w = Writer {
        bytes: Vec::new(),
        limit: max,
    };
    serde_json::to_writer(&mut w, m).map_err(native)?;
    Ok(w.bytes)
}
impl EditorAssetCheckpoint {
    pub fn capture(
        assets: &[Arc<EditorAsset>],
        limits: EditorAssetCheckpointLimits,
    ) -> Result<Self, EditorError> {
        Self::capture_with_models(assets, &[], limits)
    }
    pub fn capture_with_models(
        assets: &[Arc<EditorAsset>],
        external_models: &[Arc<ModelAsset>],
        limits: EditorAssetCheckpointLimits,
    ) -> Result<Self, EditorError> {
        Self::capture_with_sources(assets, external_models, &[], &[], limits)
    }
    pub fn capture_with_sources(
        assets: &[Arc<EditorAsset>],
        external_models: &[Arc<ModelAsset>],
        clips: &[Arc<voxy_animation::AnimationClip>],
        properties: &[voxy_animation::PropertyClip],
        limits: EditorAssetCheckpointLimits,
    ) -> Result<Self, EditorError> {
        if assets.len() > limits.max_references {
            return Err(cap());
        }
        let mut unique = Vec::new();
        let mut asset_ids = HashMap::new();
        let mut references = Vec::new();
        let mut totals = Totals::default();
        for asset in assets {
            let ptr = Arc::as_ptr(asset) as usize;
            let id = if let Some(id) = asset_ids.get(&ptr) {
                *id
            } else {
                let id = unique.len();
                if id >= limits.max_assets {
                    return Err(cap());
                }
                totals.admit(asset, limits)?;
                unique.push(asset);
                asset_ids.insert(ptr, id);
                id
            };
            references.push(id);
        }
        let mut models: Vec<_> = unique.iter().filter_map(|a| a.animated.clone()).collect();
        if models
            .len()
            .checked_add(external_models.len())
            .is_none_or(|n| n > limits.models.max_references)
        {
            return Err(cap());
        }
        models.extend(external_models.iter().cloned());
        let mut skins =
            ModelAssetCheckpoint::source_skin_meshes(&models, limits.models).map_err(native)?;
        let mut manifest = Manifest {
            version: 1,
            references,
            assets: Vec::new(),
            lods: Vec::new(),
            sources: Vec::new(),
            allocations: Vec::new(),
            mesh_arcs: Vec::new(),
            model_skins: skins.len(),
            external_models: external_models.len(),
        };
        let mut chunks = Chunks {
            values: vec![Vec::new(), Vec::new()],
            bytes: 16,
        };
        let mut static_ids = HashMap::new();
        let mut allocation_ids = HashMap::new();
        let mut mesh_ids = HashMap::new();
        let mut source_values: Vec<&SkinnedLodMesh> = Vec::new();
        let mut next_model = 0;
        for asset in unique {
            let EditorAsset {
                mesh,
                lod,
                animated,
                skinned_lod,
                nodes,
                images,
            } = asset.as_ref();
            let mesh = chunks.push(
                mesh.checkpoint()
                    .to_bytes(limits.geometry)
                    .map_err(native)?,
                limits,
            )?;
            let nodes = chunks.push(
                ImportedNodesCheckpoint::capture(nodes, images, limits.nodes)?
                    .to_json(limits.nodes.max_bytes)?
                    .into_bytes(),
                limits,
            )?;
            let model = animated.as_ref().map(|_| {
                let id = next_model;
                next_model += 1;
                id
            });
            let lod = lod
                .as_ref()
                .map(|lod| -> Result<usize, EditorError> {
                    let ptr = Arc::as_ptr(lod) as usize;
                    if let Some(id) = static_ids.get(&ptr) {
                        return Ok(*id);
                    }
                    let id = manifest.lods.len();
                    let chunk = chunks.push(
                        lod.checkpoint(limits.lod)
                            .map_err(native)?
                            .to_json(limits.lod.bytes)
                            .map_err(native)?
                            .into_bytes(),
                        limits,
                    )?;
                    manifest.lods.push(chunk);
                    static_ids.insert(ptr, id);
                    Ok(id)
                })
                .transpose()?;
            let skinned = skinned_lod
                .as_ref()
                .map(|lod| -> Result<usize, EditorError> {
                    let ptr = Arc::as_ptr(lod) as usize;
                    if let Some(id) = allocation_ids.get(&ptr) {
                        return Ok(*id);
                    }
                    let source = if let Some(id) =
                        source_values.iter().position(|v| v.shares_source_with(lod))
                    {
                        id
                    } else {
                        let handle = lod.source_mesh();
                        let ptr = Arc::as_ptr(handle) as usize;
                        let mesh = if let Some(id) = mesh_ids.get(&ptr) {
                            *id
                        } else {
                            let id = manifest.mesh_arcs.len();
                            manifest.mesh_arcs.push(skins.len());
                            skins.push(handle.as_ref().clone());
                            mesh_ids.insert(ptr, id);
                            id
                        };
                        let chunk = chunks.push(
                            lod.checkpoint(limits.skinned)
                                .map_err(native)?
                                .to_json(limits.skinned.max_bytes)
                                .map_err(native)?
                                .into_bytes(),
                            limits,
                        )?;
                        let id = manifest.sources.len();
                        manifest.sources.push(Source { chunk, mesh });
                        source_values.push(lod);
                        id
                    };
                    let id = manifest.allocations.len();
                    manifest.allocations.push(source);
                    allocation_ids.insert(ptr, id);
                    Ok(id)
                })
                .transpose()?;
            manifest.assets.push(Asset {
                mesh,
                nodes,
                model,
                lod,
                skinned,
            });
        }
        let model_bytes =
            ModelAssetCheckpoint::capture_with_sources(&models, clips, properties, limits.models)
                .map_err(native)?
                .to_bytes(limits.models)
                .map_err(native)?;
        let skin_bytes = SkinnedMeshSetCheckpoint::capture(&skins, limits.skins)
            .map_err(native)?
            .to_bytes(limits.skins)
            .map_err(native)?;
        for (index, bytes) in [model_bytes, skin_bytes].into_iter().enumerate() {
            add(&mut chunks.bytes, bytes.len(), limits.max_bytes)?;
            chunks.values[index] = bytes;
        }
        let result = Self {
            manifest,
            chunks: chunks.values,
        };
        result.admit(limits)?;
        result.size(limits)?;
        Ok(result)
    }
    fn admit(&self, limits: EditorAssetCheckpointLimits) -> Result<(), EditorError> {
        let m = &self.manifest;
        if m.version != 1 || self.chunks.len() < 2 {
            return Err(bad());
        }
        if m.assets.len() > limits.max_assets
            || m.external_models > limits.models.max_references
            || m.references.len() > limits.max_references
            || m.lods.len() > limits.max_assets
            || m.sources.len() > limits.max_assets
            || m.allocations.len() > limits.max_assets
            || m.mesh_arcs.len() > limits.max_assets
        {
            return Err(cap());
        }
        rooted(m.references.iter().copied(), m.assets.len())?;
        rooted(m.assets.iter().filter_map(|a| a.lod), m.lods.len())?;
        rooted(
            m.assets.iter().filter_map(|a| a.skinned),
            m.allocations.len(),
        )?;
        rooted(m.allocations.iter().copied(), m.sources.len())?;
        rooted(m.sources.iter().map(|s| s.mesh), m.mesh_arcs.len())?;
        let mut next_model = 0;
        let mut chunks = std::collections::BTreeSet::new();
        let mut own = |id: usize| -> Result<(), EditorError> {
            if id < 2 || id >= self.chunks.len() || !chunks.insert(id) {
                return Err(bad());
            }
            Ok(())
        };
        for a in &m.assets {
            own(a.mesh)?;
            own(a.nodes)?;
            if let Some(id) = a.model {
                if id != next_model {
                    return Err(bad());
                }
                next_model += 1;
            }
        }
        for id in &m.lods {
            own(*id)?;
        }
        for source in &m.sources {
            own(source.chunk)?;
        }
        if chunks.len().checked_add(2) != Some(self.chunks.len()) {
            return Err(bad());
        }
        for (index, id) in m.mesh_arcs.iter().enumerate() {
            if m.model_skins.checked_add(index) != Some(*id) {
                return Err(bad());
            }
        }
        if m.model_skins > limits.models.skins.max_meshes {
            return Err(cap());
        }
        Ok(())
    }
    fn size(&self, limits: EditorAssetCheckpointLimits) -> Result<(Vec<u8>, usize), EditorError> {
        let meta = metadata(&self.manifest, limits.max_metadata_bytes)?;
        let mut size = 64usize.checked_add(meta.len()).ok_or_else(cap)?;
        for chunk in &self.chunks {
            add(&mut size, 8, limits.max_bytes)?;
            add(&mut size, chunk.len(), limits.max_bytes)?;
        }
        if size > limits.max_bytes {
            return Err(cap());
        }
        Ok((meta, size))
    }
    pub fn to_bytes(&self, limits: EditorAssetCheckpointLimits) -> Result<Vec<u8>, EditorError> {
        self.admit(limits)?;
        let (meta, size) = self.size(limits)?;
        let mut bytes = Vec::with_capacity(size);
        bytes.resize(48, 0);
        bytes.extend_from_slice(&(meta.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&meta);
        bytes.extend_from_slice(&(self.chunks.len() as u64).to_le_bytes());
        for chunk in &self.chunks {
            bytes.extend_from_slice(&(chunk.len() as u64).to_le_bytes());
            bytes.extend_from_slice(chunk);
        }
        bytes[..8].copy_from_slice(b"VOXYEDA1");
        bytes[8..16].copy_from_slice(&((size - 48) as u64).to_le_bytes());
        let hash = blake3::hash(&bytes[48..]);
        bytes[16..48].copy_from_slice(hash.as_bytes());
        Ok(bytes)
    }
    pub fn from_bytes(
        bytes: &[u8],
        limits: EditorAssetCheckpointLimits,
    ) -> Result<Self, EditorError> {
        if bytes.len() > limits.max_bytes {
            return Err(cap());
        }
        struct Reader<'a> {
            bytes: &'a [u8],
            offset: usize,
        }
        impl<'a> Reader<'a> {
            fn take(&mut self, n: usize) -> Result<&'a [u8], EditorError> {
                let end = self.offset.checked_add(n).ok_or_else(bad)?;
                let b = self.bytes.get(self.offset..end).ok_or_else(bad)?;
                self.offset = end;
                Ok(b)
            }
            fn count(&mut self) -> Result<usize, EditorError> {
                usize::try_from(u64::from_le_bytes(
                    self.take(8)?.try_into().map_err(|_| bad())?,
                ))
                .map_err(|_| cap())
            }
        }
        let mut r = Reader { bytes, offset: 0 };
        if r.take(8)? != b"VOXYEDA1" {
            return Err(bad());
        }
        let n = r.count()?;
        let hash = r.take(32)?;
        if n != bytes.len() - 48 || blake3::hash(&bytes[48..]).as_bytes() != hash {
            return Err(bad());
        }
        let n = r.count()?;
        if n > limits.max_metadata_bytes {
            return Err(cap());
        }
        let manifest = serde_json::from_slice(r.take(n)?).map_err(native)?;
        let n = r.count()?;
        if n > limits
            .max_assets
            .checked_mul(4)
            .and_then(|n| n.checked_add(2))
            .ok_or_else(cap)?
            || n.checked_mul(8).is_none_or(|n| n > bytes.len() - r.offset)
        {
            return Err(cap());
        }
        let mut chunks = Vec::new();
        for _ in 0..n {
            let n = r.count()?;
            chunks.push(r.take(n)?.to_vec());
        }
        if r.offset != bytes.len() {
            return Err(bad());
        }
        let result = Self { manifest, chunks };
        result.admit(limits)?;
        Ok(result)
    }
    pub fn prepare(
        &self,
        limits: EditorAssetCheckpointLimits,
    ) -> Result<Vec<Arc<EditorAsset>>, EditorError> {
        if self.manifest.external_models != 0 {
            return Err("editor asset graph requires external model restoration".into());
        }
        let prepared = self.prepare_with_models(limits)?;
        if !prepared.clips.is_empty() || !prepared.properties.is_empty() {
            return Err("editor asset graph requires playback source restoration".into());
        }
        Ok(prepared.assets)
    }
    pub fn prepare_with_models(
        &self,
        limits: EditorAssetCheckpointLimits,
    ) -> Result<PreparedEditorAssets, EditorError> {
        self.admit(limits)?;
        self.size(limits)?;
        let m = &self.manifest;
        let record =
            SkinnedMeshSetCheckpoint::from_bytes(&self.chunks[1], limits.skins).map_err(native)?;
        if m.model_skins.checked_add(m.mesh_arcs.len()) != Some(record.meshes.len()) {
            return Err(bad());
        }
        let skins = record.prepare(limits.skins).map_err(native)?;
        let prepared = ModelAssetCheckpoint::from_bytes(&self.chunks[0], limits.models)
            .map_err(native)?
            .prepare_sources_with_skin_meshes(limits.models, &skins[..m.model_skins])
            .map_err(native)?;
        let models = prepared.models;
        let model_prefix = m.assets.iter().filter(|a| a.model.is_some()).count();
        if model_prefix.checked_add(m.external_models) != Some(models.len()) {
            return Err(bad());
        }
        let mesh_arcs: Vec<_> = m
            .mesh_arcs
            .iter()
            .map(|id| skins.get(*id).cloned().map(Arc::new).ok_or_else(bad))
            .collect::<Result<_, _>>()?;
        let mut sources = Vec::new();
        for source in &m.sources {
            sources.push(
                SkinnedLodCheckpoint::from_json(
                    std::str::from_utf8(&self.chunks[source.chunk]).map_err(native)?,
                    limits.skinned.max_bytes,
                )
                .map_err(native)?
                .prepare_with_mesh(
                    mesh_arcs.get(source.mesh).cloned().ok_or_else(bad)?,
                    limits.skinned,
                )
                .map_err(native)?,
            );
        }
        let allocations: Vec<_> = m
            .allocations
            .iter()
            .map(|id| sources.get(*id).cloned().map(Arc::new).ok_or_else(bad))
            .collect::<Result<_, _>>()?;
        let mut lods = Vec::new();
        for chunk in &m.lods {
            lods.push(Arc::new(
                CertifiedLodCheckpoint::from_json(
                    std::str::from_utf8(&self.chunks[*chunk]).map_err(native)?,
                    limits.lod.bytes,
                )
                .map_err(native)?
                .prepare(limits.lod)
                .map_err(native)?,
            ));
        }
        let mut assets = Vec::new();
        let mut totals = Totals::default();
        for a in &m.assets {
            let mesh = SceneMeshCheckpoint::from_bytes(&self.chunks[a.mesh], limits.geometry)
                .map_err(native)?
                .prepare(limits.geometry)
                .map_err(native)?;
            let nodes = ImportedNodesCheckpoint::from_json(
                std::str::from_utf8(&self.chunks[a.nodes]).map_err(native)?,
                limits.nodes.max_bytes,
            )?
            .prepare(limits.nodes)?;
            let lod = a
                .lod
                .map(|id| lods.get(id).cloned().ok_or_else(bad))
                .transpose()?;
            if let Some(lod) = &lod {
                voxy_render::SceneRenderer::certified_lod_mesh_allocation_bytes(&mesh, lod)
                    .map_err(|e| native(format!("static LOD binding: {e:?}")))?;
            }
            let animated = a
                .model
                .map(|id| models.get(id).cloned().ok_or_else(bad))
                .transpose()?;
            if let Some(model) = &animated {
                if model.primitives.iter().any(|p| {
                    p.base_color_texture
                        .is_some_and(|t| t.image >= nodes.images.len())
                }) {
                    return Err(bad());
                }
            }
            let skinned_lod = a
                .skinned
                .map(|id| allocations.get(id).cloned().ok_or_else(bad))
                .transpose()?;
            let asset = EditorAsset {
                mesh,
                lod,
                animated,
                skinned_lod,
                nodes: nodes.nodes,
                images: nodes.images,
            };
            totals.admit(&asset, limits)?;
            assets.push(Arc::new(asset));
        }
        let result = m
            .references
            .iter()
            .map(|id| assets.get(*id).cloned().ok_or_else(bad))
            .collect::<Result<Vec<_>, _>>()?;
        let external_models = models[model_prefix..].to_vec();
        if Self::capture_with_sources(
            &result,
            &external_models,
            &prepared.clips,
            &prepared.properties,
            limits,
        )? != *self
        {
            return Err(bad());
        }
        Ok(PreparedEditorAssets {
            assets: result,
            models: external_models,
            clips: prepared.clips,
            properties: prepared.properties,
        })
    }
}
#[cfg(test)]
pub(crate) fn retain_evidence(name: &str, bytes: &[u8]) {
    if let Some(output) = std::env::var_os("VOXY_EDITOR_ASSET_EVIDENCE_DIR") {
        let output = std::path::PathBuf::from(output);
        std::fs::create_dir_all(&output).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(name))
            .unwrap();
        std::io::Write::write_all(&mut file, bytes).unwrap();
    }
}
#[cfg(test)]
mod tests;
