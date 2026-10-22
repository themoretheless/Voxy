//! Immutable scene import archive. Scene nodes are not skeleton joints.
//! Keep authored material/extension data for explicit renderer capability checks.
use crate::ModelError;
use glam::{DMat4, DQuat, DVec3};
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct SceneImportLimits {
    pub source_bytes: usize,
    pub nodes: usize,
    pub vertices: usize,
    pub indices: usize,
}
impl Default for SceneImportLimits {
    fn default() -> Self {
        Self {
            source_bytes: 512 * 1024 * 1024,
            nodes: 100_000,
            vertices: 8_000_000,
            indices: 32_000_000,
        }
    }
}
#[derive(Clone, Debug)]
pub struct SceneArchiveNode {
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub mesh: Option<usize>,
    pub skin: Option<usize>,
    pub local: DMat4,
    pub world: DMat4,
}
/// Invalid authored tangents are retained for repair, never exposed as render-ready.
#[derive(Clone, Debug)]
pub enum SceneArchiveTangents {
    Finite(Vec<[f32; 4]>),
    NeedsRepair {
        source: Vec<[f32; 4]>,
        invalid_vertices: Vec<usize>,
    },
}
#[derive(Clone, Debug)]
pub struct SceneArchivePrimitive {
    pub positions: Vec<[f32; 3]>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub tangents: Option<SceneArchiveTangents>,
    pub texcoords: Vec<(u32, Vec<[f32; 2]>)>,
    pub indices: Vec<u32>,
    pub material: Option<usize>,
    pub source: serde_json::Value,
}
#[derive(Clone, Debug)]
pub struct SceneArchive {
    pub document: serde_json::Value,
    pub buffers: Vec<Arc<[u8]>>,
    pub nodes: Vec<SceneArchiveNode>,
    pub meshes: Vec<Vec<SceneArchivePrimitive>>,
    pub scene_roots: Vec<Vec<usize>>,
    pub default_scene: Option<usize>,
}
fn fail(s: impl Into<String>) -> ModelError {
    ModelError(s.into())
}
fn numbers<const N: usize>(
    v: Option<&serde_json::Value>,
    default: [f64; N],
) -> Result<[f64; N], ModelError> {
    let Some(v) = v else { return Ok(default) };
    let a = v.as_array().ok_or_else(|| fail("invalid node transform"))?;
    if a.len() != N {
        return Err(fail("invalid node transform width"));
    }
    let mut out = [0.; N];
    for (x, v) in out.iter_mut().zip(a) {
        *x = v
            .as_f64()
            .filter(|x| x.is_finite())
            .ok_or_else(|| fail("nonfinite node transform"))?;
    }
    Ok(out)
}
impl SceneArchive {
    /// Buffers are supplied by the asset owner; this parser performs no file/network IO.
    /// Extension payloads remain authored data, never silently replaced by defaults.
    pub fn decode(
        source: &[u8],
        buffers: Vec<Arc<[u8]>>,
        limits: SceneImportLimits,
    ) -> Result<Self, ModelError> {
        let bytes = buffers
            .iter()
            .try_fold(source.len(), |n, b| n.checked_add(b.len()))
            .ok_or_else(|| fail("scene byte overflow"))?;
        if bytes > limits.source_bytes {
            return Err(fail("scene byte limit exceeded"));
        }
        let document: serde_json::Value =
            serde_json::from_slice(source).map_err(|e| fail(e.to_string()))?;
        let gltf = gltf::Gltf::from_slice(source).map_err(|e| fail(e.to_string()))?;
        if gltf.nodes().count() > limits.nodes || gltf.buffers().count() != buffers.len() {
            return Err(fail("scene node or buffer count limit"));
        }
        for buffer in gltf.buffers() {
            if buffers[buffer.index()].len() < buffer.length() {
                return Err(fail("truncated scene buffer"));
            }
        }
        for view in gltf.views() {
            if view
                .offset()
                .checked_add(view.length())
                .is_none_or(|end| end > buffers[view.buffer().index()].len())
            {
                return Err(fail("scene buffer view out of range"));
            }
        }
        for accessor in gltf.accessors() {
            if accessor.sparse().is_some() {
                return Err(fail("sparse scene accessor unsupported; source retained"));
            }
            let view = accessor
                .view()
                .ok_or_else(|| fail("scene accessor lacks a buffer view"))?;
            let stride = view.stride().unwrap_or(accessor.size());
            let end = accessor
                .count()
                .saturating_sub(1)
                .checked_mul(stride)
                .and_then(|n| n.checked_add(accessor.offset()))
                .and_then(|n| {
                    n.checked_add(if accessor.count() == 0 {
                        0
                    } else {
                        accessor.size()
                    })
                });
            if stride < accessor.size() || end.is_none_or(|n| n > view.length()) {
                return Err(fail("scene accessor out of buffer bounds"));
            }
        }
        let mut nodes = Vec::new();
        for node in gltf.nodes() {
            let raw = &document["nodes"][node.index()];
            let local = if raw.get("matrix").is_some() {
                DMat4::from_cols_array(&numbers(
                    raw.get("matrix"),
                    DMat4::IDENTITY.to_cols_array(),
                )?)
            } else {
                let translation = DVec3::from_array(numbers(raw.get("translation"), [0.; 3])?);
                let rotation = DQuat::from_array(numbers(raw.get("rotation"), [0., 0., 0., 1.])?);
                let scale = DVec3::from_array(numbers(raw.get("scale"), [1.; 3])?);
                if (rotation.length_squared() - 1.).abs() > 1e-5 {
                    return Err(fail("invalid scene node quaternion"));
                }
                DMat4::from_scale_rotation_translation(scale, rotation, translation)
            };
            nodes.push(SceneArchiveNode {
                parent: None,
                children: node.children().map(|v| v.index()).collect(),
                mesh: node.mesh().map(|v| v.index()),
                skin: node.skin().map(|v| v.index()),
                local,
                world: DMat4::IDENTITY,
            });
        }
        for parent in 0..nodes.len() {
            for child in nodes[parent].children.clone() {
                if child >= nodes.len() || nodes[child].parent.replace(parent).is_some() {
                    return Err(fail("scene node has multiple parents"));
                }
            }
        }
        let mut visited = vec![false; nodes.len()];
        let mut stack = (0..nodes.len())
            .filter(|&i| nodes[i].parent.is_none())
            .map(|i| (i, DMat4::IDENTITY))
            .collect::<Vec<_>>();
        while let Some((id, parent)) = stack.pop() {
            if visited[id] {
                return Err(fail("scene hierarchy cycle"));
            }
            visited[id] = true;
            let world = parent * nodes[id].local;
            if !world.is_finite() {
                return Err(fail("scene world transform overflow"));
            }
            nodes[id].world = world;
            for &child in &nodes[id].children {
                stack.push((child, world));
            }
        }
        if visited.iter().any(|v| !*v) {
            return Err(fail("scene hierarchy cycle"));
        }
        let mut meshes = Vec::new();
        let mut vertex_count = 0usize;
        let mut index_count = 0usize;
        for mesh in gltf.meshes() {
            let mut primitives = Vec::new();
            for primitive in mesh.primitives() {
                if primitive.mode() != gltf::mesh::Mode::Triangles {
                    return Err(fail(
                        "scene primitive mode unsupported; geometry was not dropped",
                    ));
                }
                let reader = primitive.reader(|buffer| Some(buffers[buffer.index()].as_ref()));
                let positions = reader
                    .read_positions()
                    .ok_or_else(|| fail("scene primitive lacks positions"))?
                    .collect::<Vec<_>>();
                vertex_count = vertex_count
                    .checked_add(positions.len())
                    .ok_or_else(|| fail("scene vertex count overflow"))?;
                if vertex_count > limits.vertices {
                    return Err(fail("scene vertex limit exceeded"));
                }
                if positions.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(fail("nonfinite scene position"));
                }
                let indices = reader
                    .read_indices()
                    .map(|v| v.into_u32().collect::<Vec<_>>())
                    .unwrap_or_else(|| (0..positions.len() as u32).collect());
                index_count = index_count
                    .checked_add(indices.len())
                    .ok_or_else(|| fail("scene index count overflow"))?;
                if index_count > limits.indices
                    || indices.len() % 3 != 0
                    || indices.iter().any(|&i| i as usize >= positions.len())
                {
                    return Err(fail("invalid scene indices or limit"));
                }
                let normals = reader.read_normals().map(|v| v.collect::<Vec<_>>());
                let tangents = reader.read_tangents().map(|v| v.collect::<Vec<_>>());
                if normals.as_ref().is_some_and(|v| {
                    v.len() != positions.len() || v.iter().flatten().any(|x| !x.is_finite())
                }) || tangents
                    .as_ref()
                    .is_some_and(|v| v.len() != positions.len())
                {
                    return Err(fail("invalid scene normals or tangents"));
                }
                let tangents = tangents.map(|values| {
                    let invalid_vertices = values
                        .iter()
                        .enumerate()
                        .filter_map(|(i, v)| v.iter().any(|x| !x.is_finite()).then_some(i))
                        .collect::<Vec<_>>();
                    if invalid_vertices.is_empty() {
                        SceneArchiveTangents::Finite(values)
                    } else {
                        SceneArchiveTangents::NeedsRepair {
                            source: values,
                            invalid_vertices,
                        }
                    }
                });
                let mut texcoords = Vec::new();
                for (semantic, _) in primitive.attributes() {
                    if let gltf::Semantic::TexCoords(set) = semantic {
                        let values = reader
                            .read_tex_coords(set)
                            .ok_or_else(|| fail("invalid scene UV accessor"))?
                            .into_f32()
                            .collect::<Vec<_>>();
                        if values.len() != positions.len()
                            || values.iter().flatten().any(|v| !v.is_finite())
                        {
                            return Err(fail("invalid scene UV values"));
                        }
                        texcoords.push((set, values));
                    }
                }
                primitives.push(SceneArchivePrimitive {
                    positions,
                    normals,
                    tangents,
                    texcoords,
                    indices,
                    material: primitive.material().index(),
                    source: document["meshes"][mesh.index()]["primitives"][primitive.index()]
                        .clone(),
                });
            }
            meshes.push(primitives);
        }
        let scene_roots = gltf
            .scenes()
            .map(|s| s.nodes().map(|n| n.index()).collect())
            .collect();
        let default_scene = gltf.default_scene().map(|v| v.index());
        Ok(Self {
            document,
            buffers,
            nodes,
            meshes,
            scene_roots,
            default_scene,
        })
    }
}
