//! Complete native ModelAsset resource graph. GPU/LOD/images/import observations
//! belong to the enclosing EditorAsset owner, not this filesystem-free model.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use voxy_animation::{
    AnimationAssetCheckpoint, AnimationAssetLimits, PropertyAssetCheckpoint, PropertyAssetLimits,
};
use voxy_render::{SkinnedMeshSetCheckpoint, SkinnedMeshSetLimits};

#[derive(Clone, Copy, Debug)]
pub struct ModelAssetCheckpointLimits {
    pub max_bytes: usize,
    pub max_metadata_bytes: usize,
    pub max_models: usize,
    pub max_references: usize,
    pub max_morph_owners: usize,
    /// Combined main primitives and morph source bindings.
    pub max_primitives: usize,
    pub max_names: usize,
    pub max_string_bytes: usize,
    pub animation: AnimationAssetLimits,
    pub property: PropertyAssetLimits,
    pub morph: MorphArchiveLimits,
    pub skins: SkinnedMeshSetLimits,
}
impl Default for ModelAssetCheckpointLimits {
    fn default() -> Self {
        Self {
            max_bytes: 512 * 1024 * 1024,
            max_metadata_bytes: 16 * 1024 * 1024,
            max_models: 1024,
            max_references: 4096,
            max_morph_owners: 1024,
            max_primitives: 4096,
            max_names: 65536,
            max_string_bytes: 8 * 1024 * 1024,
            animation: AnimationAssetLimits {
                max_references: 65536,
                ..AnimationAssetLimits::default()
            },
            property: PropertyAssetLimits {
                max_tracks: 131072,
                ..PropertyAssetLimits::default()
            },
            morph: MorphArchiveLimits::default(),
            skins: SkinnedMeshSetLimits::default(),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Primitive {
    chunk: usize,
    skin: Option<usize>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    node: usize,
    name: Option<usize>,
    geometry: Primitive,
    defaults: Vec<u32>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Morph {
    defaults: usize,
    sources: Vec<usize>,
    skeleton: Option<usize>,
    primitives: Vec<Option<Binding>>,
    clips: Vec<usize>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    primitives: Vec<Primitive>,
    skeleton: usize,
    animations: Vec<usize>,
    joint_names: Vec<Option<usize>>,
    mesh_joint: usize,
    mesh_instances: usize,
    skinned: bool,
    morph: usize,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    names: usize,
    models: Vec<Model>,
    morphs: Vec<Morph>,
    references: Vec<usize>,
    #[serde(default, skip_serializing_if = "zero")]
    external_clips: usize,
    #[serde(default, skip_serializing_if = "zero")]
    external_properties: usize,
}
// Serde's skip predicate borrows its field.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn zero(value: &usize) -> bool {
    *value == 0
}
/// Model graph plus explicitly retained playback sources. PropertyClip owns its
/// value; enclosing owners must separately preserve Arc<PropertyClip> identities.
#[derive(Debug)]
pub struct PreparedModelSources {
    pub models: Vec<Arc<ModelAsset>>,
    pub clips: Vec<Arc<voxy_animation::AnimationClip>>,
    pub properties: Vec<voxy_animation::PropertyClip>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelAssetCheckpoint {
    manifest: Manifest,
    chunks: Vec<Vec<u8>>,
}
const HEADER: usize = 48;
fn invalid() -> ModelError {
    fail("invalid model resource checkpoint")
}
fn capacity() -> ModelError {
    fail("model resource checkpoint exceeds policy")
}
fn resource(error: impl std::fmt::Display) -> ModelError {
    fail(error.to_string())
}
fn rooted_ids(ids: impl Iterator<Item = usize>, total: usize) -> Result<(), ModelError> {
    let mut seen = vec![false; total];
    let mut next = 0;
    for id in ids {
        let slot = seen.get_mut(id).ok_or_else(invalid)?;
        if !*slot {
            if id != next {
                return Err(invalid());
            }
            *slot = true;
            next += 1;
        }
    }
    if next != total {
        return Err(invalid());
    }
    Ok(())
}
fn ids(start: usize, count: usize) -> Vec<usize> {
    (start..start + count).collect()
}
struct Names {
    values: Vec<Arc<str>>,
    ids: HashMap<usize, usize>,
    bytes: usize,
}
impl Names {
    fn add(
        &mut self,
        value: &Arc<str>,
        limits: ModelAssetCheckpointLimits,
    ) -> Result<usize, ModelError> {
        let key = Arc::as_ptr(value).cast::<u8>() as usize;
        if let Some(id) = self.ids.get(&key) {
            return Ok(*id);
        }
        self.bytes = self.bytes.checked_add(value.len()).ok_or_else(capacity)?;
        if self.bytes > limits.max_string_bytes || self.values.len() >= limits.max_names {
            return Err(capacity());
        }
        let id = self.values.len();
        self.values.push(value.clone());
        self.ids.insert(key, id);
        Ok(id)
    }
}
struct Chunks {
    values: Vec<Vec<u8>>,
    bytes: usize,
}
impl Chunks {
    fn push(
        &mut self,
        value: Vec<u8>,
        limits: ModelAssetCheckpointLimits,
    ) -> Result<usize, ModelError> {
        self.bytes = self
            .bytes
            .checked_add(8)
            .and_then(|v| v.checked_add(value.len()))
            .ok_or_else(capacity)?;
        if self.bytes > limits.max_bytes {
            return Err(capacity());
        }
        let id = self.values.len();
        self.values.push(value);
        Ok(id)
    }
}
fn skin(geometry: &ModelGeometry, skins: &mut Vec<SkinnedMesh>) -> Option<usize> {
    match geometry {
        ModelGeometry::Static(_) => None,
        ModelGeometry::Skinned(mesh) => {
            let id = skins.len();
            skins.push(mesh.clone());
            Some(id)
        }
    }
}
fn geometry_matches(
    geometry: &mut ModelGeometry,
    index: Option<usize>,
    skins: &[SkinnedMesh],
) -> Result<(), ModelError> {
    match (geometry, index) {
        (ModelGeometry::Static(_), None) => Ok(()),
        (ModelGeometry::Skinned(mesh), Some(id)) => {
            let shared = skins.get(id).ok_or_else(invalid)?;
            if mesh.checkpoint() != shared.checkpoint() {
                return Err(invalid());
            }
            *mesh = shared.clone();
            Ok(())
        }
        _ => Err(invalid()),
    }
}
fn encode_metadata(manifest: &Manifest, max: usize) -> Result<Vec<u8>, ModelError> {
    struct Writer {
        bytes: Vec<u8>,
        max: usize,
    }
    impl std::io::Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self
                .bytes
                .len()
                .checked_add(bytes.len())
                .is_none_or(|n| n > self.max)
            {
                return Err(std::io::Error::other("model metadata byte cap"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Writer {
        bytes: Vec::new(),
        max,
    };
    serde_json::to_writer(&mut writer, manifest).map_err(resource)?;
    Ok(writer.bytes)
}
impl ModelAssetCheckpoint {
    /// Return live immutable skin values in the same canonical order used by
    /// capture. No content-based interning; repeated model/morph owners are visited once.
    pub fn source_skin_meshes(
        models: &[Arc<ModelAsset>],
        limits: ModelAssetCheckpointLimits,
    ) -> Result<Vec<SkinnedMesh>, ModelError> {
        if models.len() > limits.max_references {
            return Err(capacity());
        }
        let mut seen_models = std::collections::HashSet::new();
        let mut seen_morphs = std::collections::HashSet::new();
        let mut morphs = Vec::new();
        let mut skins = Vec::new();
        let mut count = 0usize;
        for model in models {
            if !seen_models.insert(Arc::as_ptr(model) as usize) {
                continue;
            }
            if seen_models.len() > limits.max_models {
                return Err(capacity());
            }
            count = count
                .checked_add(model.primitives.len())
                .ok_or_else(capacity)?;
            if count > limits.max_primitives {
                return Err(capacity());
            }
            for primitive in &model.primitives {
                skin(&primitive.geometry, &mut skins);
            }
            if seen_morphs.insert(Arc::as_ptr(&model.morph) as usize) {
                if seen_morphs.len() > limits.max_morph_owners {
                    return Err(capacity());
                }
                morphs.push(&model.morph);
            }
        }
        for morph in morphs {
            count = count
                .checked_add(morph.primitives.len())
                .ok_or_else(capacity)?;
            if count > limits.max_primitives {
                return Err(capacity());
            }
            for binding in morph.primitives.iter().flatten() {
                skin(&binding.geometry.source.geometry, &mut skins);
            }
        }
        SkinnedMeshSetCheckpoint::capture(&skins, limits.skins).map_err(resource)?;
        Ok(skins)
    }
    /// Capture at the enclosing owner's resource-query barrier. No live model is changed.
    pub fn capture(
        models: &[Arc<ModelAsset>],
        limits: ModelAssetCheckpointLimits,
    ) -> Result<Self, ModelError> {
        Self::capture_with_sources(models, &[], &[], limits)
    }
    /// Captures standalone playback sources in the same rig, clip, property and
    /// name graph as model-owned sources. No content-based deduplication occurs.
    pub fn capture_with_sources(
        models: &[Arc<ModelAsset>],
        clips: &[Arc<voxy_animation::AnimationClip>],
        properties: &[voxy_animation::PropertyClip],
        limits: ModelAssetCheckpointLimits,
    ) -> Result<Self, ModelError> {
        if models.len() > limits.max_references {
            return Err(capacity());
        }
        let mut source_models = Vec::new();
        let mut model_ids = HashMap::new();
        let mut references = Vec::new();
        for model in models {
            let ptr = Arc::as_ptr(model) as usize;
            let id = if let Some(id) = model_ids.get(&ptr) {
                *id
            } else {
                let id = source_models.len();
                if id >= limits.max_models {
                    return Err(capacity());
                }
                source_models.push(model);
                model_ids.insert(ptr, id);
                id
            };
            references.push(id);
        }
        let mut source_morphs = Vec::new();
        let mut morph_ids = HashMap::new();
        for model in &source_models {
            let ptr = Arc::as_ptr(&model.morph) as usize;
            if let std::collections::hash_map::Entry::Vacant(entry) = morph_ids.entry(ptr) {
                let id = source_morphs.len();
                if id >= limits.max_morph_owners {
                    return Err(capacity());
                }
                source_morphs.push(&model.morph);
                entry.insert(id);
            }
        }
        let mut names = Names {
            values: Vec::new(),
            ids: HashMap::new(),
            bytes: 0,
        };
        let mut skeletons = Vec::new();
        let mut animation_clips = Vec::new();
        let mut property_clips = Vec::new();
        let mut sets = Vec::new();
        let mut chunks = Chunks {
            values: vec![Vec::new(), Vec::new(), Vec::new()],
            bytes: 24,
        };
        let mut skins = Vec::new();
        let mut count = 0usize;
        let mut manifest = Manifest {
            version: 1,
            names: 0,
            models: Vec::new(),
            morphs: Vec::new(),
            references,
            external_clips: clips.len(),
            external_properties: properties.len(),
        };
        for model in source_models {
            let ModelAsset {
                primitives,
                skeleton,
                animations,
                joint_names,
                mesh_joint,
                mesh_instances,
                skinned,
                morph,
            } = model.as_ref();
            if *mesh_joint >= skeleton.joints().len()
                || joint_names.len() != skeleton.joints().len()
                || *mesh_instances == 0
            {
                return Err(invalid());
            }
            count = count.checked_add(primitives.len()).ok_or_else(capacity)?;
            if count > limits.max_primitives {
                return Err(capacity());
            }
            let mut saved_primitives = Vec::new();
            for p in primitives {
                let chunk = chunks.push(p.checkpoint(limits.morph)?.as_bytes().to_vec(), limits)?;
                saved_primitives.push(Primitive {
                    chunk,
                    skin: skin(&p.geometry, &mut skins),
                });
            }
            if skeletons.len() >= limits.animation.max_references
                || animation_clips
                    .len()
                    .checked_add(animations.len())
                    .is_none_or(|n| n > limits.animation.max_references)
            {
                return Err(capacity());
            }
            let rig = skeletons.len();
            skeletons.push(skeleton.clone());
            let clips = ids(animation_clips.len(), animations.len());
            animation_clips.extend(animations.iter().cloned());
            let joint_names = joint_names
                .iter()
                .map(|n| n.as_ref().map(|n| names.add(n, limits)).transpose())
                .collect::<Result<Vec<_>, _>>()?;
            manifest.models.push(Model {
                primitives: saved_primitives,
                skeleton: rig,
                animations: clips,
                joint_names,
                mesh_joint: *mesh_joint,
                mesh_instances: *mesh_instances,
                skinned: *skinned,
                morph: *morph_ids
                    .get(&(Arc::as_ptr(morph) as usize))
                    .ok_or_else(invalid)?,
            });
        }
        for data in source_morphs {
            let morph::MorphImportData {
                defaults,
                sources,
                skeleton,
                primitives,
                clips,
            } = data.as_ref();
            count = count.checked_add(primitives.len()).ok_or_else(capacity)?;
            if count > limits.max_primitives {
                return Err(capacity());
            }
            if sets.len() >= limits.property.max_sets
                || animation_clips
                    .len()
                    .checked_add(sources.len())
                    .is_none_or(|n| n > limits.animation.max_references)
                || property_clips
                    .len()
                    .checked_add(clips.len())
                    .is_none_or(|n| n > limits.property.max_clips)
                || (skeleton.is_some() && skeletons.len() >= limits.animation.max_references)
            {
                return Err(capacity());
            }
            let defaults_id = sets.len();
            sets.push(defaults.clone());
            let sources_ids = ids(animation_clips.len(), sources.len());
            animation_clips.extend(sources.iter().cloned());
            let skeleton = if let Some(rig) = skeleton {
                let id = skeletons.len();
                skeletons.push(rig.clone());
                Some(id)
            } else {
                None
            };
            let clips_ids = ids(property_clips.len(), clips.len());
            property_clips.extend(clips.iter().cloned());
            let mut saved = Vec::new();
            for binding in primitives {
                saved.push(if let Some(b) = binding {
                    let morph::MorphBinding {
                        node,
                        name,
                        geometry,
                        defaults,
                    } = b;
                    let chunk = chunks.push(
                        geometry.checkpoint(limits.morph)?.as_bytes().to_vec(),
                        limits,
                    )?;
                    Some(Binding {
                        node: *node,
                        name: name.as_ref().map(|n| names.add(n, limits)).transpose()?,
                        geometry: Primitive {
                            chunk,
                            skin: skin(&geometry.source.geometry, &mut skins),
                        },
                        defaults: defaults.iter().map(|v| v.to_bits()).collect(),
                    })
                } else {
                    None
                });
            }
            manifest.morphs.push(Morph {
                defaults: defaults_id,
                sources: sources_ids,
                skeleton,
                primitives: saved,
                clips: clips_ids,
            });
        }
        if animation_clips
            .len()
            .checked_add(clips.len())
            .is_none_or(|n| n > limits.animation.max_references)
            || property_clips
                .len()
                .checked_add(properties.len())
                .is_none_or(|n| n > limits.property.max_clips)
        {
            return Err(capacity());
        }
        animation_clips.extend(clips.iter().cloned());
        property_clips.extend(properties.iter().cloned());
        for name in AnimationAssetCheckpoint::source_names(&skeletons, &animation_clips)
            .into_iter()
            .chain(PropertyAssetCheckpoint::source_names(
                &property_clips,
                &sets,
            ))
        {
            names.add(&name, limits)?;
        }
        manifest.names = names.values.len();
        let animation = AnimationAssetCheckpoint::capture_with_names(
            &skeletons,
            &animation_clips,
            &names.values,
            limits.animation,
        )
        .map_err(resource)?
        .to_json(limits.animation.max_bytes)
        .map_err(resource)?
        .into_bytes();
        let property = PropertyAssetCheckpoint::capture_with_names(
            &property_clips,
            &sets,
            &names.values,
            limits.property,
        )
        .map_err(resource)?
        .to_json(limits.property.max_bytes)
        .map_err(resource)?
        .into_bytes();
        let skins = SkinnedMeshSetCheckpoint::capture(&skins, limits.skins)
            .map_err(resource)?
            .to_bytes(limits.skins)
            .map_err(resource)?;
        for (i, value) in [animation, property, skins].into_iter().enumerate() {
            chunks.bytes = chunks.bytes.checked_add(value.len()).ok_or_else(capacity)?;
            if chunks.bytes > limits.max_bytes {
                return Err(capacity());
            }
            chunks.values[i] = value;
        }
        let result = Self {
            manifest,
            chunks: chunks.values,
        };
        result.admit(limits)?;
        result.encoded_size(limits)?;
        Ok(result)
    }
    fn admit(&self, limits: ModelAssetCheckpointLimits) -> Result<(), ModelError> {
        let m = &self.manifest;
        if m.version != 1
            || self.chunks.len() < 3
            || m.references.iter().any(|id| *id >= m.models.len())
        {
            return Err(invalid());
        }
        if m.models.len() > limits.max_models
            || m.external_clips > limits.animation.max_references
            || m.external_properties > limits.property.max_clips
            || m.references.len() > limits.max_references
            || m.morphs.len() > limits.max_morph_owners
            || m.names > limits.max_names
        {
            return Err(capacity());
        }
        rooted_ids(m.references.iter().copied(), m.models.len())?;
        rooted_ids(m.models.iter().map(|model| model.morph), m.morphs.len())?;
        let mut count = 0usize;
        let mut next_chunk = 3usize;
        for model in &m.models {
            if model.morph >= m.morphs.len()
                || model.joint_names.iter().flatten().any(|id| *id >= m.names)
            {
                return Err(invalid());
            }
            if model.joint_names.len() > voxy_animation::MAX_JOINTS
                || model.animations.len() > limits.animation.max_references
            {
                return Err(capacity());
            }
            count = count
                .checked_add(model.primitives.len())
                .ok_or_else(capacity)?;
            for p in &model.primitives {
                if p.chunk != next_chunk || p.chunk >= self.chunks.len() {
                    return Err(invalid());
                }
                next_chunk = next_chunk.checked_add(1).ok_or_else(capacity)?;
            }
        }
        for data in &m.morphs {
            if data.sources.len() != data.clips.len() {
                return Err(invalid());
            }
            if data.sources.len() > limits.animation.max_references
                || data.clips.len() > limits.property.max_clips
            {
                return Err(capacity());
            }
            count = count
                .checked_add(data.primitives.len())
                .ok_or_else(capacity)?;
            for b in data.primitives.iter().flatten() {
                if b.name.is_some_and(|n| n >= m.names)
                    || b.geometry.chunk != next_chunk
                    || b.geometry.chunk >= self.chunks.len()
                {
                    return Err(invalid());
                }
                next_chunk = next_chunk.checked_add(1).ok_or_else(capacity)?;
                if b.defaults.len() > voxy_render::MAX_MORPH_TARGETS {
                    return Err(capacity());
                }
            }
        }
        if next_chunk != self.chunks.len() {
            return Err(invalid());
        }
        if count > limits.max_primitives
            || self.chunks.len() > limits.max_primitives.checked_add(3).ok_or_else(capacity)?
        {
            return Err(capacity());
        }
        Ok(())
    }
    fn encoded_size(
        &self,
        limits: ModelAssetCheckpointLimits,
    ) -> Result<(Vec<u8>, usize), ModelError> {
        let metadata = encode_metadata(&self.manifest, limits.max_metadata_bytes)?;
        let mut size = HEADER
            .checked_add(16)
            .and_then(|v| v.checked_add(metadata.len()))
            .ok_or_else(capacity)?;
        for chunk in &self.chunks {
            size = size
                .checked_add(8)
                .and_then(|v| v.checked_add(chunk.len()))
                .ok_or_else(capacity)?;
        }
        if size > limits.max_bytes {
            return Err(capacity());
        }
        Ok((metadata, size))
    }
    pub fn to_bytes(&self, limits: ModelAssetCheckpointLimits) -> Result<Vec<u8>, ModelError> {
        self.admit(limits)?;
        let (metadata, size) = self.encoded_size(limits)?;
        let mut bytes = Vec::with_capacity(size);
        bytes.resize(HEADER, 0);
        bytes.extend_from_slice(&(metadata.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&metadata);
        bytes.extend_from_slice(&(self.chunks.len() as u64).to_le_bytes());
        for chunk in &self.chunks {
            bytes.extend_from_slice(&(chunk.len() as u64).to_le_bytes());
            bytes.extend_from_slice(chunk);
        }
        bytes[..8].copy_from_slice(b"VOXYMDL1");
        bytes[8..16].copy_from_slice(&((size - HEADER) as u64).to_le_bytes());
        let digest = blake3::hash(&bytes[HEADER..]);
        bytes[16..48].copy_from_slice(digest.as_bytes());
        Ok(bytes)
    }
    pub fn from_bytes(
        bytes: &[u8],
        limits: ModelAssetCheckpointLimits,
    ) -> Result<Self, ModelError> {
        if bytes.len() > limits.max_bytes {
            return Err(capacity());
        }
        struct Reader<'a> {
            bytes: &'a [u8],
            offset: usize,
        }
        impl<'a> Reader<'a> {
            fn take(&mut self, n: usize) -> Result<&'a [u8], ModelError> {
                let end = self.offset.checked_add(n).ok_or_else(invalid)?;
                let value = self.bytes.get(self.offset..end).ok_or_else(invalid)?;
                self.offset = end;
                Ok(value)
            }
            fn count(&mut self) -> Result<usize, ModelError> {
                usize::try_from(u64::from_le_bytes(
                    self.take(8)?.try_into().map_err(|_| invalid())?,
                ))
                .map_err(|_| capacity())
            }
        }
        let mut r = Reader { bytes, offset: 0 };
        if r.take(8)? != b"VOXYMDL1" {
            return Err(invalid());
        }
        let n = r.count()?;
        let digest = r.take(32)?;
        if n != bytes.len() - HEADER || blake3::hash(&bytes[HEADER..]).as_bytes() != digest {
            return Err(invalid());
        }
        let n = r.count()?;
        if n > limits.max_metadata_bytes {
            return Err(capacity());
        }
        let manifest = serde_json::from_slice(r.take(n)?).map_err(resource)?;
        let n = r.count()?;
        if n > limits.max_primitives.checked_add(3).ok_or_else(capacity)?
            || n.checked_mul(8).is_none_or(|v| v > bytes.len() - r.offset)
        {
            return Err(capacity());
        }
        let mut chunks = Vec::new();
        for _ in 0..n {
            let size = r.count()?;
            chunks.push(r.take(size)?.to_vec());
        }
        if r.offset != bytes.len() {
            return Err(invalid());
        }
        let result = Self { manifest, chunks };
        result.admit(limits)?;
        Ok(result)
    }
    /// Pure preparation. Publishing this graph into the editor remains the caller's transaction.
    pub fn prepare(
        &self,
        limits: ModelAssetCheckpointLimits,
    ) -> Result<Vec<Arc<ModelAsset>>, ModelError> {
        self.reject_external_sources()?;
        Ok(self.prepare_inner(limits, None)?.models)
    }
    /// Bind the enclosing resource graph's explicit skin collection. Both source
    /// words and the original local buffer/cache alias topology must match.
    pub fn prepare_with_skin_meshes(
        &self,
        limits: ModelAssetCheckpointLimits,
        skins: &[SkinnedMesh],
    ) -> Result<Vec<Arc<ModelAsset>>, ModelError> {
        self.reject_external_sources()?;
        Ok(self.prepare_inner(limits, Some(skins))?.models)
    }
    fn reject_external_sources(&self) -> Result<(), ModelError> {
        if self.manifest.external_clips != 0 || self.manifest.external_properties != 0 {
            return Err(fail("model graph requires playback source restoration"));
        }
        Ok(())
    }
    /// Restores model and standalone playback roots together without a resolver.
    pub fn prepare_sources(
        &self,
        limits: ModelAssetCheckpointLimits,
    ) -> Result<PreparedModelSources, ModelError> {
        self.prepare_inner(limits, None)
    }
    /// Uses the enclosing graph's skin buffers while retaining playback roots.
    pub fn prepare_sources_with_skin_meshes(
        &self,
        limits: ModelAssetCheckpointLimits,
        skins: &[SkinnedMesh],
    ) -> Result<PreparedModelSources, ModelError> {
        self.prepare_inner(limits, Some(skins))
    }
    fn prepare_inner(
        &self,
        limits: ModelAssetCheckpointLimits,
        external: Option<&[SkinnedMesh]>,
    ) -> Result<PreparedModelSources, ModelError> {
        self.admit(limits)?;
        self.encoded_size(limits)?;
        let animation = AnimationAssetCheckpoint::from_json(
            std::str::from_utf8(&self.chunks[0]).map_err(resource)?,
            limits.animation.max_bytes,
        )
        .map_err(resource)?
        .prepare(limits.animation)
        .map_err(resource)?;
        if animation.names.len() != self.manifest.names {
            return Err(invalid());
        }
        let mut seen_names = std::collections::HashSet::new();
        let mut string_bytes = 0usize;
        for name in &animation.names {
            if !seen_names.insert(Arc::as_ptr(name).cast::<u8>() as usize) {
                return Err(invalid());
            }
            string_bytes = string_bytes.checked_add(name.len()).ok_or_else(capacity)?;
            if string_bytes > limits.max_string_bytes {
                return Err(capacity());
            }
        }
        let property = PropertyAssetCheckpoint::from_json(
            std::str::from_utf8(&self.chunks[1]).map_err(resource)?,
            limits.property.max_bytes,
        )
        .map_err(resource)?
        .prepare_with_names(limits.property, &animation.names)
        .map_err(resource)?;
        let skin_record = SkinnedMeshSetCheckpoint::from_bytes(&self.chunks[2], limits.skins)
            .map_err(resource)?;
        let skins = if let Some(skins) = external {
            if SkinnedMeshSetCheckpoint::capture(skins, limits.skins).map_err(resource)?
                != skin_record
            {
                return Err(invalid());
            }
            skins.to_vec()
        } else {
            skin_record.prepare(limits.skins).map_err(resource)?
        };
        let name = |id: usize| animation.names.get(id).cloned().ok_or_else(invalid);
        let mut morphs = Vec::new();
        for record in &self.manifest.morphs {
            let mut primitives = Vec::new();
            for b in &record.primitives {
                primitives.push(if let Some(b) = b {
                    let mut geometry = MorphGeometryCheckpoint::from_bytes(
                        &self.chunks[b.geometry.chunk],
                        limits.morph,
                    )?
                    .prepare(limits.morph)?;
                    geometry_matches(&mut geometry.source.geometry, b.geometry.skin, &skins)?;
                    let defaults: Vec<_> = b.defaults.iter().map(|v| f32::from_bits(*v)).collect();
                    if defaults.len() != geometry.targets().len()
                        || defaults.iter().any(|v| !v.is_finite())
                    {
                        return Err(invalid());
                    }
                    Some(morph::MorphBinding {
                        node: b.node,
                        name: b.name.map(name).transpose()?,
                        geometry,
                        defaults,
                    })
                } else {
                    None
                });
            }
            let sources = record
                .sources
                .iter()
                .map(|id| animation.clips.get(*id).cloned().ok_or_else(invalid))
                .collect::<Result<Vec<_>, _>>()?;
            let skeleton = record
                .skeleton
                .map(|id| animation.skeletons.get(id).cloned().ok_or_else(invalid))
                .transpose()?;
            let clips = record
                .clips
                .iter()
                .map(|id| property.clips.get(*id).cloned().ok_or_else(invalid))
                .collect::<Result<Vec<_>, _>>()?;
            morphs.push(Arc::new(morph::MorphImportData {
                defaults: property
                    .sets
                    .get(record.defaults)
                    .cloned()
                    .ok_or_else(invalid)?,
                sources,
                skeleton,
                primitives,
                clips,
            }));
        }
        let mut models = Vec::new();
        for record in &self.manifest.models {
            let mut primitives = Vec::new();
            for p in &record.primitives {
                let mut primitive =
                    ModelPrimitiveCheckpoint::from_bytes(&self.chunks[p.chunk], limits.morph)?
                        .prepare(limits.morph)?;
                geometry_matches(&mut primitive.geometry, p.skin, &skins)?;
                primitives.push(primitive);
            }
            let skeleton = animation
                .skeletons
                .get(record.skeleton)
                .cloned()
                .ok_or_else(invalid)?;
            if record.mesh_joint >= skeleton.joints().len()
                || record.joint_names.len() != skeleton.joints().len()
                || record.mesh_instances == 0
            {
                return Err(invalid());
            }
            let animations = record
                .animations
                .iter()
                .map(|id| animation.clips.get(*id).cloned().ok_or_else(invalid))
                .collect::<Result<Vec<_>, _>>()?;
            let joint_names = record
                .joint_names
                .iter()
                .map(|id| id.map(name).transpose())
                .collect::<Result<Vec<_>, _>>()?;
            models.push(Arc::new(ModelAsset {
                primitives,
                skeleton,
                animations,
                joint_names,
                mesh_joint: record.mesh_joint,
                mesh_instances: record.mesh_instances,
                skinned: record.skinned,
                morph: morphs.get(record.morph).cloned().ok_or_else(invalid)?,
            }));
        }
        let result = self
            .manifest
            .references
            .iter()
            .map(|id| models.get(*id).cloned().ok_or_else(invalid))
            .collect::<Result<Vec<_>, _>>()?;
        let clip_start = animation
            .clips
            .len()
            .checked_sub(self.manifest.external_clips)
            .ok_or_else(invalid)?;
        let property_start = property
            .clips
            .len()
            .checked_sub(self.manifest.external_properties)
            .ok_or_else(invalid)?;
        let clips = animation.clips[clip_start..].to_vec();
        let properties = property.clips[property_start..].to_vec();
        if Self::capture_with_sources(&result, &clips, &properties, limits)? != *self {
            return Err(invalid());
        }
        Ok(PreparedModelSources {
            models: result,
            clips,
            properties,
        })
    }
}
#[cfg(test)]
mod tests;
