//! Exact source streams, identity tables and derived-cache receipts for a native
//! mesh set. Cached proofs are rebuilt and checked, never trusted from the file.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct SkinnedMeshSetLimits {
    pub geometry: GeometryArchiveLimits,
    pub max_meshes: usize,
    pub max_vertex_buffers: usize,
    pub max_index_buffers: usize,
    pub max_groups: usize,
    /// Aggregate source vertices across distinct native reconstruction jobs.
    pub max_native_vertices: usize,
    /// Aggregate indices across distinct vertex/rig/group/index combinations.
    pub max_native_indices: usize,
}
impl Default for SkinnedMeshSetLimits {
    fn default() -> Self {
        Self {
            geometry: GeometryArchiveLimits::default(),
            max_meshes: 4096,
            max_vertex_buffers: 4096,
            max_index_buffers: 4096,
            max_groups: 4096,
            max_native_vertices: 4_000_000,
            max_native_indices: 12_000_000,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SkinnedMeshSetEntry {
    pub vertices: usize,
    pub indices: usize,
    pub joint_count: u16,
    pub coherent_weights: bool,
    pub max_position: u32,
    pub groups: Option<usize>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkinnedMeshSetCheckpoint {
    pub version: u32,
    pub vertices: Vec<Vec<SkinnedVertexCheckpoint>>,
    pub indices: Vec<Vec<u32>>,
    pub groups: Vec<[u8; 32]>,
    pub meshes: Vec<SkinnedMeshSetEntry>,
}
impl SkinnedMeshSetCheckpoint {
    pub(crate) fn admit(&self, limits: SkinnedMeshSetLimits) -> Result<usize, Error> {
        if self.version != 1 {
            return Err(Error::Invalid);
        }
        if self.meshes.len() > limits.max_meshes
            || self.vertices.len() > limits.max_vertex_buffers
            || self.indices.len() > limits.max_index_buffers
            || self.groups.len() > limits.max_groups
        {
            return Err(Error::Capacity);
        }
        let vertices = self
            .vertices
            .iter()
            .try_fold(0usize, |a, v| a.checked_add(v.len()).ok_or(Error::Capacity))?;
        let indices = self
            .indices
            .iter()
            .try_fold(0usize, |a, v| a.checked_add(v.len()).ok_or(Error::Capacity))?;
        if vertices > limits.geometry.max_vertices || indices > limits.geometry.max_indices {
            return Err(Error::Capacity);
        }
        let mut native_vertices = 0usize;
        let mut native_indices = 0usize;
        let mut vertex_jobs = std::collections::HashSet::new();
        let mut index_jobs = std::collections::HashSet::new();
        for m in &self.meshes {
            if m.vertices >= self.vertices.len()
                || m.indices >= self.indices.len()
                || m.groups.is_some_and(|g| g >= self.groups.len())
                || m.joint_count == 0
                || usize::from(m.joint_count) > crate::MAX_SKIN_JOINTS
            {
                return Err(Error::Invalid);
            }
            if usize::from(m.joint_count) > limits.geometry.max_joints {
                return Err(Error::Capacity);
            }
            if vertex_jobs.insert((m.vertices, m.joint_count, m.groups)) {
                native_vertices = native_vertices
                    .checked_add(self.vertices[m.vertices].len())
                    .ok_or(Error::Capacity)?;
            }
            if index_jobs.insert((m.vertices, m.indices, m.joint_count, m.groups)) {
                native_indices = native_indices
                    .checked_add(self.indices[m.indices].len())
                    .ok_or(Error::Capacity)?;
            }
            if native_vertices > limits.max_native_vertices
                || native_indices > limits.max_native_indices
            {
                return Err(Error::Capacity);
            }
        }
        encoded_size(
            85,
            &[
                (self.vertices.len(), 8),
                (vertices, 48),
                (self.indices.len(), 8),
                (indices, 4),
                (self.groups.len(), 32),
                (self.meshes.len(), 31),
            ],
            limits.geometry,
        )
    }
    pub fn to_bytes(&self, limits: SkinnedMeshSetLimits) -> Result<Vec<u8>, Error> {
        let mut w = Writer::new(4, self.admit(limits)?)?;
        w.words(&[self.version])?;
        w.count(self.vertices.len())?;
        for stream in &self.vertices {
            w.count(stream.len())?;
            for v in stream {
                w.words(&v.attributes)?;
                for word in v.joints.into_iter().chain(v.weights) {
                    w.put(&word.to_le_bytes())?;
                }
            }
        }
        w.count(self.indices.len())?;
        for indices in &self.indices {
            w.indices(indices)?;
        }
        w.count(self.groups.len())?;
        for receipt in &self.groups {
            w.put(receipt)?;
        }
        w.count(self.meshes.len())?;
        for m in &self.meshes {
            w.count(m.vertices)?;
            w.count(m.indices)?;
            w.put(&m.joint_count.to_le_bytes())?;
            w.put(&[u8::from(m.coherent_weights)])?;
            w.words(&[m.max_position])?;
            w.put(
                &match m.groups {
                    None => u64::MAX,
                    Some(g) => u64::try_from(g).map_err(|_| Error::Capacity)?,
                }
                .to_le_bytes(),
            )?;
        }
        w.finish()
    }
    pub fn from_bytes(bytes: &[u8], limits: SkinnedMeshSetLimits) -> Result<Self, Error> {
        let mut r = Reader::new(bytes, 4, limits.geometry)?;
        let version = r.u32()?;
        if version != 1 {
            return Err(Error::Invalid);
        }
        let count = r.count(limits.max_vertex_buffers, 8)?;
        let mut vertices = Vec::new();
        let mut remaining = limits.geometry.max_vertices;
        for _ in 0..count {
            let n = r.count(remaining, 48)?;
            remaining -= n;
            let mut stream = Vec::with_capacity(n);
            for _ in 0..n {
                let attributes = r.words()?;
                let mut joints = [0; 4];
                let mut weights = [0; 4];
                for v in &mut joints {
                    *v = r.u16()?;
                }
                for v in &mut weights {
                    *v = r.u16()?;
                }
                stream.push(SkinnedVertexCheckpoint {
                    attributes,
                    joints,
                    weights,
                });
            }
            vertices.push(stream);
        }
        let count = r.count(limits.max_index_buffers, 8)?;
        let mut indices = Vec::new();
        let mut remaining = limits.geometry.max_indices;
        for _ in 0..count {
            let stream = r.indices(remaining)?;
            remaining -= stream.len();
            indices.push(stream);
        }
        let count = r.count(limits.max_groups, 32)?;
        let mut groups = Vec::new();
        for _ in 0..count {
            groups.push(r.take(32)?.try_into().map_err(|_| Error::Invalid)?);
        }
        let count = r.count(limits.max_meshes, 31)?;
        let mut meshes = Vec::new();
        for _ in 0..count {
            // Reference indices use the same checked u64 representation as counts.
            let read_id = |r: &mut Reader<'_>| {
                usize::try_from(u64::from_le_bytes(
                    r.take(8)?.try_into().map_err(|_| Error::Invalid)?,
                ))
                .map_err(|_| Error::Capacity)
            };
            let vertices = read_id(&mut r)?;
            let indices = read_id(&mut r)?;
            let joint_count = r.u16()?;
            let coherent_weights = match r.take(1)?[0] {
                0 => false,
                1 => true,
                _ => return Err(Error::Invalid),
            };
            let max_position = r.u32()?;
            let group = u64::from_le_bytes(r.take(8)?.try_into().map_err(|_| Error::Invalid)?);
            let groups = if group == u64::MAX {
                None
            } else {
                Some(usize::try_from(group).map_err(|_| Error::Capacity)?)
            };
            meshes.push(SkinnedMeshSetEntry {
                vertices,
                indices,
                joint_count,
                coherent_weights,
                max_position,
                groups,
            });
        }
        r.end()?;
        let result = Self {
            version,
            vertices,
            indices,
            groups,
            meshes,
        };
        result.admit(limits)?;
        Ok(result)
    }
}
