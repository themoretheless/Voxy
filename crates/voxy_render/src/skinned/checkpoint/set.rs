use super::*;
use crate::{SkinnedMeshSetCheckpoint, SkinnedMeshSetEntry, SkinnedMeshSetLimits};
use std::collections::HashMap;
fn group_receipt(group: &InfluenceGroups) -> [u8; 32] {
    let InfluenceGroups {
        keys,
        families,
        has_bad_normal,
        max_position,
        bad_normal,
    } = group;
    let mut hash = blake3::Hasher::new();
    hash.update(b"voxy-native-influence-groups-v1");
    hash.update(&(keys.len() as u64).to_le_bytes());
    for (j, w) in keys {
        for v in j.iter().chain(w) {
            hash.update(&v.to_le_bytes());
        }
    }
    hash.update(&(families.len() as u64).to_le_bytes());
    for f in families {
        for v in f {
            hash.update(&v.to_le_bytes());
        }
    }
    hash.update(&[u8::from(*has_bad_normal)]);
    hash.update(&(max_position.len() as u64).to_le_bytes());
    for v in max_position {
        hash.update(&v.to_bits().to_le_bytes());
    }
    hash.update(&(bad_normal.len() as u64).to_le_bytes());
    for v in bad_normal {
        hash.update(&[u8::from(*v)]);
    }
    *hash.finalize().as_bytes()
}
impl SkinnedMeshSetCheckpoint {
    pub fn capture(
        meshes: &[SkinnedMesh],
        limits: SkinnedMeshSetLimits,
    ) -> Result<Self, GeometryCheckpointError> {
        if meshes.len() > limits.max_meshes {
            return Err(GeometryCheckpointError::Capacity);
        }
        let mut out = Self {
            version: 1,
            vertices: Vec::new(),
            indices: Vec::new(),
            groups: Vec::new(),
            meshes: Vec::new(),
        };
        let mut vertex_ids = HashMap::new();
        let mut index_ids = HashMap::new();
        let mut group_ids = HashMap::new();
        let mut total_vertices = 0usize;
        let mut total_indices = 0usize;
        for mesh in meshes {
            let SkinnedMesh {
                vertices,
                indices,
                joint_count,
                coherent_weights,
                max_position_component,
                groups,
            } = mesh;
            let vptr = Arc::as_ptr(vertices).cast::<SkinnedVertex>() as usize;
            let vertices = if let Some(id) = vertex_ids.get(&vptr) {
                *id
            } else {
                total_vertices = total_vertices
                    .checked_add(vertices.len())
                    .ok_or(GeometryCheckpointError::Capacity)?;
                if total_vertices > limits.geometry.max_vertices
                    || out.vertices.len() >= limits.max_vertex_buffers
                {
                    return Err(GeometryCheckpointError::Capacity);
                }
                let id = out.vertices.len();
                out.vertices
                    .push(vertices.iter().map(SkinnedVertexCheckpoint::from).collect());
                vertex_ids.insert(vptr, id);
                id
            };
            let iptr = Arc::as_ptr(indices).cast::<u32>() as usize;
            let indices = if let Some(id) = index_ids.get(&iptr) {
                *id
            } else {
                total_indices = total_indices
                    .checked_add(indices.len())
                    .ok_or(GeometryCheckpointError::Capacity)?;
                if total_indices > limits.geometry.max_indices
                    || out.indices.len() >= limits.max_index_buffers
                {
                    return Err(GeometryCheckpointError::Capacity);
                }
                let id = out.indices.len();
                out.indices.push(indices.to_vec());
                index_ids.insert(iptr, id);
                id
            };
            let groups = groups
                .as_ref()
                .map(|g| {
                    let ptr = Arc::as_ptr(g) as usize;
                    if let Some(id) = group_ids.get(&ptr) {
                        return Ok(*id);
                    }
                    if out.groups.len() >= limits.max_groups {
                        return Err(GeometryCheckpointError::Capacity);
                    }
                    let id = out.groups.len();
                    out.groups.push(group_receipt(g));
                    group_ids.insert(ptr, id);
                    Ok(id)
                })
                .transpose()?;
            out.meshes.push(SkinnedMeshSetEntry {
                vertices,
                indices,
                joint_count: *joint_count,
                coherent_weights: *coherent_weights,
                max_position: max_position_component.to_bits(),
                groups,
            });
        }
        out.admit(limits)?;
        Ok(out)
    }
    pub fn prepare(
        &self,
        limits: SkinnedMeshSetLimits,
    ) -> Result<Vec<SkinnedMesh>, GeometryCheckpointError> {
        self.admit(limits)?;
        let mut vertices: Vec<Option<Arc<[SkinnedVertex]>>> = vec![None; self.vertices.len()];
        let mut indices: Vec<Option<Arc<[u32]>>> = vec![None; self.indices.len()];
        let mut groups: Vec<Option<Arc<InfluenceGroups>>> = vec![None; self.groups.len()];
        let mut meshes = Vec::new();
        let mut prototypes: HashMap<(usize, u16, Option<usize>), SkinnedMesh> = HashMap::new();
        let mut completed: HashMap<SkinnedMeshSetEntry, SkinnedMesh> = HashMap::new();
        for entry in &self.meshes {
            if let Some(mesh) = completed.get(entry) {
                meshes.push(mesh.clone());
                continue;
            }
            let key = (entry.vertices, entry.joint_count, entry.groups);
            if let Some(prototype) = prototypes.get(&key) {
                if prototype.coherent_weights != entry.coherent_weights
                    || prototype.max_position_component.to_bits() != entry.max_position
                {
                    return Err(GeometryCheckpointError::Invalid);
                }
                let stream = indices[entry.indices]
                    .get_or_insert_with(|| Arc::from(self.indices[entry.indices].clone()))
                    .clone();
                let mesh = prototype
                    .with_shared_indices(stream)
                    .map_err(GeometryCheckpointError::Skin)?;
                completed.insert(entry.clone(), mesh.clone());
                meshes.push(mesh);
                continue;
            }
            let checkpoint = SkinnedMeshCheckpoint {
                version: 1,
                vertices: self.vertices[entry.vertices].clone(),
                indices: self.indices[entry.indices].clone(),
                joint_count: entry.joint_count,
            };
            let mut mesh = checkpoint.prepare(limits.geometry)?;
            if mesh.coherent_weights != entry.coherent_weights
                || mesh.max_position_component.to_bits() != entry.max_position
            {
                return Err(GeometryCheckpointError::Invalid);
            }
            match (&mesh.groups, entry.groups) {
                (None, None) => {}
                (Some(group), Some(id)) => {
                    if group_receipt(group) != self.groups[id] {
                        return Err(GeometryCheckpointError::Invalid);
                    }
                    mesh.groups = Some(groups[id].get_or_insert_with(|| group.clone()).clone());
                }
                _ => return Err(GeometryCheckpointError::Invalid),
            }
            mesh.vertices = vertices[entry.vertices]
                .get_or_insert_with(|| mesh.vertices.clone())
                .clone();
            mesh.indices = indices[entry.indices]
                .get_or_insert_with(|| mesh.indices.clone())
                .clone();
            prototypes.insert(key, mesh.clone());
            completed.insert(entry.clone(), mesh.clone());
            meshes.push(mesh);
        }
        if Self::capture(&meshes, limits)? != *self {
            return Err(GeometryCheckpointError::Invalid);
        }
        Ok(meshes)
    }
}
#[cfg(test)]
mod tests;
