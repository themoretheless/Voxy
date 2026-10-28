use super::*;
use crate::{GeometryArchiveLimits, GeometryCheckpointError, SceneMeshCheckpoint};
impl SceneMesh {
    #[must_use]
    pub fn checkpoint(&self) -> SceneMeshCheckpoint {
        let Self {
            vertices,
            indices,
            validated: _,
            material_coordinates,
            authored_normals,
            material_parameters,
        } = self;
        // validated is a process-local proof, not archive authority. Restore
        // independently validates every source stream through native constructors.
        SceneMeshCheckpoint {
            version: 1,
            vertices: vertices
                .iter()
                .map(|value| {
                    let SceneVertex {
                        position,
                        uv,
                        color,
                    } = value;
                    let mut words = [0; 9];
                    words[..3].copy_from_slice(&position.map(f32::to_bits));
                    words[3..5].copy_from_slice(&uv.map(f32::to_bits));
                    words[5..].copy_from_slice(&color.map(f32::to_bits));
                    words
                })
                .collect(),
            indices: indices.clone(),
            material_parameters: material_parameters.map(f32::to_bits),
            normals: authored_normals
                .as_ref()
                .map(|v| v.iter().map(|v| v.map(f32::to_bits)).collect()),
            material_coordinates: material_coordinates
                .as_ref()
                .map(|v| v.iter().map(|v| v.map(f32::to_bits)).collect()),
        }
    }
}
impl SceneMeshCheckpoint {
    pub fn prepare(
        &self,
        limits: GeometryArchiveLimits,
    ) -> Result<SceneMesh, GeometryCheckpointError> {
        self.encoded_size(limits)?;
        let vertices = self
            .vertices
            .iter()
            .map(|words| {
                let values = words.map(f32::from_bits);
                SceneVertex {
                    position: [values[0], values[1], values[2]],
                    uv: [values[3], values[4]],
                    color: [values[5], values[6], values[7], values[8]],
                }
            })
            .collect();
        let mut mesh = SceneMesh::new(vertices, self.indices.clone())?
            .with_material_parameters(self.material_parameters.map(f32::from_bits))?;
        if let Some(values) = &self.normals {
            mesh = mesh.with_normals(values.iter().map(|v| v.map(f32::from_bits)).collect())?;
        }
        if let Some(values) = &self.material_coordinates {
            mesh = mesh.with_material_coordinates(
                values.iter().map(|v| v.map(f32::from_bits)).collect(),
            )?;
        }
        Ok(mesh)
    }
}
