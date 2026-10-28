//! Imported-node/image participant for the pending EditorAsset graph coordinator.
#![allow(dead_code)]
use super::*;
use serde::{Deserialize, Serialize};
use voxy_render::{GeometryArchiveLimits, ImageCheckpoint, SceneMeshCheckpoint};
#[derive(Clone, Copy, Debug)]
pub(crate) struct NodeCheckpointLimits {
    pub max_bytes: usize,
    pub max_nodes: usize,
    pub max_images: usize,
    pub max_string_bytes: usize,
    pub max_vertices: usize,
    pub max_indices: usize,
    pub max_pixel_bytes: u64,
    pub geometry: GeometryArchiveLimits,
}
impl Default for NodeCheckpointLimits {
    fn default() -> Self {
        Self {
            max_bytes: 512 * 1024 * 1024,
            max_nodes: 65536,
            max_images: 4096,
            max_string_bytes: 8 * 1024 * 1024,
            max_vertices: 4_000_000,
            max_indices: 12_000_000,
            max_pixel_bytes: 256 * 1024 * 1024,
            geometry: GeometryArchiveLimits::default(),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sampling {
    wrap: [u8; 2],
    filter: [u8; 2],
    mip: Option<u8>,
    anisotropy: u16,
}
fn wrap(v: TextureWrap) -> u8 {
    match v {
        TextureWrap::Clamp => 0,
        TextureWrap::Repeat => 1,
        TextureWrap::Mirror => 2,
    }
}
fn filter(v: TextureFilter) -> u8 {
    match v {
        TextureFilter::Nearest => 0,
        TextureFilter::Linear => 1,
    }
}
fn decode_wrap(v: u8) -> Result<TextureWrap, EditorError> {
    match v {
        0 => Ok(TextureWrap::Clamp),
        1 => Ok(TextureWrap::Repeat),
        2 => Ok(TextureWrap::Mirror),
        _ => Err(bad()),
    }
}
fn decode_filter(v: u8) -> Result<TextureFilter, EditorError> {
    match v {
        0 => Ok(TextureFilter::Nearest),
        1 => Ok(TextureFilter::Linear),
        _ => Err(bad()),
    }
}
impl Sampling {
    fn capture(v: TextureSampling) -> Self {
        let TextureSampling {
            wrap_u,
            wrap_v,
            min_filter,
            mag_filter,
            mipmap_filter,
            anisotropy,
        } = v;
        Self {
            wrap: [wrap(wrap_u), wrap(wrap_v)],
            filter: [filter(min_filter), filter(mag_filter)],
            mip: mipmap_filter.map(filter),
            anisotropy,
        }
    }
    fn prepare(&self) -> Result<TextureSampling, EditorError> {
        Ok(TextureSampling {
            wrap_u: decode_wrap(self.wrap[0])?,
            wrap_v: decode_wrap(self.wrap[1])?,
            min_filter: decode_filter(self.filter[0])?,
            mag_filter: decode_filter(self.filter[1])?,
            mipmap_filter: self.mip.map(decode_filter).transpose()?,
            anisotropy: self.anisotropy,
        })
    }
}
fn transform(v: Transform) -> [u32; 10] {
    let Transform {
        translation,
        rotation,
        scale,
    } = v;
    let t = translation.to_array();
    let r = rotation.to_array();
    let s = scale.to_array();
    [t[0], t[1], t[2], r[0], r[1], r[2], r[3], s[0], s[1], s[2]].map(f32::to_bits)
}
fn decode_transform(v: [u32; 10]) -> Result<Transform, EditorError> {
    let v = v.map(f32::from_bits);
    let local = Transform {
        translation: Vec3::new(v[0], v[1], v[2]),
        rotation: glam::Quat::from_array([v[3], v[4], v[5], v[6]]),
        scale: Vec3::new(v[7], v[8], v[9]),
    };
    local
        .matrix()
        .map_err(|e| format!("node checkpoint transform: {e}"))?;
    Ok(local)
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Node {
    name: String,
    parent: Option<usize>,
    local: [u32; 10],
    mesh: Option<Vec<u8>>,
    image: Option<usize>,
    sampling: Sampling,
    use_mips: bool,
    geometry_key: Option<(usize, usize)>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImportedNodesCheckpoint {
    version: u32,
    nodes: Vec<Node>,
    images: Vec<Vec<u8>>,
}
pub(crate) struct PreparedImportedNodes {
    pub nodes: Vec<ImportedNode>,
    pub images: Vec<ImageAsset>,
}
fn bad() -> EditorError {
    "invalid imported node/image checkpoint".into()
}
fn cap() -> EditorError {
    "imported node/image checkpoint exceeds policy".into()
}
fn native(e: impl std::fmt::Display) -> EditorError {
    format!("node/image checkpoint: {e}").into()
}
fn add(a: &mut usize, n: usize, max: usize) -> Result<(), EditorError> {
    *a = a.checked_add(n).ok_or_else(cap)?;
    if *a > max {
        return Err(cap());
    }
    Ok(())
}
impl ImportedNodesCheckpoint {
    pub fn capture(
        nodes: &[ImportedNode],
        images: &[ImageAsset],
        limits: NodeCheckpointLimits,
    ) -> Result<Self, EditorError> {
        if nodes.len() > limits.max_nodes || images.len() > limits.max_images {
            return Err(cap());
        }
        let mut saved = Vec::new();
        let mut strings = 0;
        let mut vertices = 0;
        let mut indices = 0;
        for node in nodes {
            let ImportedNode {
                name,
                parent,
                local,
                mesh,
                image,
                sampling,
                use_mips,
                geometry_key,
            } = node;
            add(&mut strings, name.len(), limits.max_string_bytes)?;
            let mesh = mesh
                .as_ref()
                .map(|m| {
                    add(&mut vertices, m.vertices().len(), limits.max_vertices)?;
                    add(&mut indices, m.indices().len(), limits.max_indices)?;
                    m.checkpoint().to_bytes(limits.geometry).map_err(native)
                })
                .transpose()?;
            saved.push(Node {
                name: name.clone(),
                parent: *parent,
                local: transform(*local),
                mesh,
                image: *image,
                sampling: Sampling::capture(*sampling),
                use_mips: *use_mips,
                geometry_key: *geometry_key,
            });
        }
        let mut pixels = 0u64;
        let mut saved_images = Vec::new();
        for image in images {
            pixels = pixels
                .checked_add(image.rgba().len() as u64)
                .ok_or_else(cap)?;
            if pixels > limits.max_pixel_bytes {
                return Err(cap());
            }
            saved_images.push(
                image
                    .checkpoint()
                    .to_bytes(limits.geometry)
                    .map_err(native)?,
            );
        }
        let record = Self {
            version: 1,
            nodes: saved,
            images: saved_images,
        };
        record.admit(limits)?;
        record.to_json(limits.max_bytes)?;
        Ok(record)
    }
    fn admit(&self, limits: NodeCheckpointLimits) -> Result<(), EditorError> {
        if self.version != 1 {
            return Err(bad());
        }
        if self.nodes.len() > limits.max_nodes || self.images.len() > limits.max_images {
            return Err(cap());
        }
        let mut strings = 0;
        let mut geometry = std::collections::BTreeMap::new();
        for (index, node) in self.nodes.iter().enumerate() {
            add(&mut strings, node.name.len(), limits.max_string_bytes)?;
            if node.parent.is_some_and(|p| p >= index)
                || node.image.is_some_and(|i| i >= self.images.len())
            {
                return Err(bad());
            }
            if let Some(mesh) = &node.mesh {
                let key = node.geometry_key.ok_or_else(bad)?;
                if let Some(previous) = geometry.insert(key, mesh.as_slice()) {
                    if previous != mesh.as_slice() {
                        return Err(bad());
                    }
                }
            }
            decode_transform(node.local)?;
            node.sampling.prepare()?;
        }
        Ok(())
    }
    pub fn to_json(&self, max_bytes: usize) -> Result<String, EditorError> {
        struct Writer {
            bytes: Vec<u8>,
            limit: usize,
        }
        impl std::io::Write for Writer {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if self
                    .bytes
                    .len()
                    .checked_add(bytes.len())
                    .is_none_or(|n| n > self.limit)
                {
                    return Err(std::io::Error::other("node checkpoint byte cap"));
                }
                self.bytes.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut w = Writer {
            bytes: Vec::new(),
            limit: max_bytes,
        };
        serde_json::to_writer(&mut w, self).map_err(native)?;
        String::from_utf8(w.bytes).map_err(native)
    }
    pub fn from_json(json: &str, max_bytes: usize) -> Result<Self, EditorError> {
        if json.len() > max_bytes {
            return Err(cap());
        }
        serde_json::from_str(json).map_err(native)
    }
    pub fn prepare(
        &self,
        limits: NodeCheckpointLimits,
    ) -> Result<PreparedImportedNodes, EditorError> {
        self.admit(limits)?;
        self.to_json(limits.max_bytes)?;
        // Parse and admit aggregate source counts before constructing native resources.
        let mut records = Vec::new();
        let mut vertices = 0;
        let mut indices = 0;
        for node in &self.nodes {
            records.push(
                node.mesh
                    .as_ref()
                    .map(|bytes| {
                        let record = SceneMeshCheckpoint::from_bytes(bytes, limits.geometry)
                            .map_err(native)?;
                        add(&mut vertices, record.vertices.len(), limits.max_vertices)?;
                        add(&mut indices, record.indices.len(), limits.max_indices)?;
                        Ok::<_, EditorError>(record)
                    })
                    .transpose()?,
            );
        }
        let mut image_records = Vec::new();
        let mut pixels = 0u64;
        for bytes in &self.images {
            let record = ImageCheckpoint::from_bytes(bytes, limits.geometry).map_err(native)?;
            pixels = pixels
                .checked_add(record.rgba.len() as u64)
                .ok_or_else(cap)?;
            if pixels > limits.max_pixel_bytes {
                return Err(cap());
            }
            image_records.push(record);
        }
        let images = image_records
            .iter()
            .map(|r| r.prepare(limits.geometry).map_err(native))
            .collect::<Result<Vec<_>, _>>()?;
        let mut nodes = Vec::new();
        for (node, mesh) in self.nodes.iter().zip(records) {
            nodes.push(ImportedNode {
                name: node.name.clone(),
                parent: node.parent,
                local: decode_transform(node.local)?,
                mesh: mesh
                    .map(|r| r.prepare(limits.geometry).map_err(native))
                    .transpose()?,
                image: node.image,
                sampling: node.sampling.prepare()?,
                use_mips: node.use_mips,
                geometry_key: node.geometry_key,
            });
        }
        if Self::capture(&nodes, &images, limits)? != *self {
            return Err(bad());
        }
        Ok(PreparedImportedNodes { nodes, images })
    }
}
#[cfg(test)]
mod tests;
