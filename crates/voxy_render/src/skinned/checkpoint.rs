mod set;
use super::*;
use crate::{
    GeometryArchiveLimits, GeometryCheckpointError, SkinnedMeshCheckpoint, SkinnedVertexCheckpoint,
};
impl From<&SkinnedVertex> for SkinnedVertexCheckpoint {
    fn from(value: &SkinnedVertex) -> Self {
        let SkinnedVertex {
            position,
            normal,
            uv,
            joints,
            weights,
        } = value;
        let mut attributes = [0; 8];
        attributes[..3].copy_from_slice(&position.map(f32::to_bits));
        attributes[3..6].copy_from_slice(&normal.map(f32::to_bits));
        attributes[6..].copy_from_slice(&uv.map(f32::to_bits));
        SkinnedVertexCheckpoint {
            attributes,
            joints: *joints,
            weights: *weights,
        }
    }
}
impl SkinnedMesh {
    #[must_use]
    pub fn checkpoint(&self) -> SkinnedMeshCheckpoint {
        let Self {
            vertices,
            indices,
            joint_count,
            coherent_weights: _,
            max_position_component: _,
            groups: _,
        } = self;
        // Admission/cache metadata is derived from these immutable exact streams.
        SkinnedMeshCheckpoint {
            version: 1,
            vertices: vertices.iter().map(SkinnedVertexCheckpoint::from).collect(),
            indices: indices.to_vec(),
            joint_count: *joint_count,
        }
    }
}
impl SkinnedMeshCheckpoint {
    pub fn prepare(
        &self,
        limits: GeometryArchiveLimits,
    ) -> Result<SkinnedMesh, GeometryCheckpointError> {
        self.encoded_size(limits)?;
        let vertices = self
            .vertices
            .iter()
            .map(|value| {
                let v = value.attributes.map(f32::from_bits);
                SkinnedVertex {
                    position: [v[0], v[1], v[2]],
                    normal: [v[3], v[4], v[5]],
                    uv: [v[6], v[7]],
                    joints: value.joints,
                    weights: value.weights,
                }
            })
            .collect();
        let mesh = SkinnedMesh::new(vertices, self.indices.clone(), self.joint_count)
            .map_err(GeometryCheckpointError::Skin)?;
        // Zero-weight joint canonicalization must never silently rewrite archived
        // accepted attributes. Weights remain exact UNORM16 integers throughout.
        if mesh.vertices().len() != self.vertices.len()
            || mesh
                .vertices()
                .iter()
                .zip(&self.vertices)
                .any(|(value, expected)| SkinnedVertexCheckpoint::from(value) != *expected)
        {
            return Err(GeometryCheckpointError::Invalid);
        }
        Ok(mesh)
    }
}
