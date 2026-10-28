//! Skeletal LOD source data. Every candidate pose still gets native recertification.
use super::*;
use crate::{
    CertifiedLodCheckpoint, CertifiedLodCheckpointError, GeometryArchiveLimits,
    GeometryCheckpointError, LodArchiveLimits, SkinnedMeshSetCheckpoint, SkinnedMeshSetLimits,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug)]
pub struct SkinnedLodCheckpointLimits {
    pub max_bytes: usize,
    pub geometry: GeometryArchiveLimits,
    pub lod: LodArchiveLimits,
}
impl Default for SkinnedLodCheckpointLimits {
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            geometry: GeometryArchiveLimits::default(),
            lod: LodArchiveLimits {
                bytes: 128 * 1024 * 1024,
                positions: 1_000_000,
                levels: 64,
                indices: 3_000_000,
                cells: 1_000_000,
            },
        }
    }
}
impl SkinnedLodCheckpointLimits {
    fn mesh(self) -> SkinnedMeshSetLimits {
        SkinnedMeshSetLimits {
            geometry: GeometryArchiveLimits {
                max_bytes: self.geometry.max_bytes.min(self.max_bytes),
                ..self.geometry
            },
            max_meshes: 1,
            max_vertex_buffers: 1,
            max_index_buffers: 1,
            max_groups: 1,
            max_native_vertices: self.geometry.max_vertices,
            max_native_indices: self.geometry.max_indices,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum SkinnedLodCheckpointError {
    #[error("invalid skeletal LOD checkpoint or source binding")]
    Invalid,
    #[error("skeletal LOD checkpoint exceeds policy")]
    Capacity,
    #[error(transparent)]
    Geometry(#[from] GeometryCheckpointError),
    #[error(transparent)]
    Certificate(#[from] CertifiedLodCheckpointError),
    #[error("native skeletal LOD verification failed: {0:?}")]
    Native(SkinnedLodError),
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
}
type Error = SkinnedLodCheckpointError;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkinnedLodCheckpoint {
    version: u32,
    mesh: Vec<u8>,
    certificate: CertifiedLodCheckpoint,
}
impl SkinnedLodMesh {
    pub fn checkpoint(
        &self,
        limits: SkinnedLodCheckpointLimits,
    ) -> Result<SkinnedLodCheckpoint, Error> {
        let Source { mesh, variants } = self.0.as_ref();
        if mesh.vertices().len() > limits.lod.positions
            || variants
                .len()
                .checked_add(1)
                .is_none_or(|n| n > limits.lod.levels)
        {
            return Err(Error::Capacity);
        }
        let mut indices = mesh.indices().len();
        let mut cells = 0usize;
        for variant in variants {
            indices = indices
                .checked_add(variant.indices.len())
                .ok_or(Error::Capacity)?;
            match &variant.witnesses {
                Witnesses::Whole(a, b) => {
                    cells = cells
                        .checked_add(a.len())
                        .and_then(|n| n.checked_add(b.len()))
                        .ok_or(Error::Capacity)?;
                }
                Witnesses::Subdivided(a, b) => {
                    for triangle in a.iter().chain(b) {
                        cells = cells
                            .checked_add(triangle.cells.len())
                            .ok_or(Error::Capacity)?;
                    }
                }
            }
            if indices > limits.lod.indices || cells > limits.lod.cells {
                return Err(Error::Capacity);
            }
        }
        if indices > limits.lod.indices {
            return Err(Error::Capacity);
        }
        let geometry =
            SkinnedMeshSetCheckpoint::capture(std::slice::from_ref(mesh.as_ref()), limits.mesh())?
                .to_bytes(limits.mesh())?;
        // Native source constructors produce one proof kind for the whole source.
        // Empty sources have no stored discriminator and use the canonical Whole form.
        let subdivided = variants
            .first()
            .is_some_and(|v| matches!(v.witnesses, Witnesses::Subdivided(..)));
        let positions = mesh.vertices().iter().map(|v| v.position).collect();
        let base = mesh.indices().to_vec();
        let certificate = if subdivided {
            let proposals = variants
                .iter()
                .map(|v| {
                    let Variant { indices, witnesses } = v;
                    let Witnesses::Subdivided(forward, reverse) = witnesses else {
                        return Err(Error::Invalid);
                    };
                    Ok(CertifiedLodSubdivisionVariant {
                        indices: indices.clone(),
                        source_to_variant: forward.clone(),
                        variant_to_source: reverse.clone(),
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            CertifiedLodIndexSet::new_subdivided(positions, base, proposals)
        } else {
            let proposals = variants
                .iter()
                .map(|v| {
                    let Variant { indices, witnesses } = v;
                    let Witnesses::Whole(forward, reverse) = witnesses else {
                        return Err(Error::Invalid);
                    };
                    Ok(CertifiedLodVariant {
                        indices: indices.clone(),
                        source_to_variant: forward.clone(),
                        variant_to_source: reverse.clone(),
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            CertifiedLodIndexSet::new(positions, base, proposals)
        }
        .map_err(|e| Error::Certificate(CertifiedLodCheckpointError::Certificate(e)))?
        .checkpoint(limits.lod)?;
        let result = SkinnedLodCheckpoint {
            version: 1,
            mesh: geometry,
            certificate,
        };
        result.to_json(limits.max_bytes)?;
        Ok(result)
    }
}
impl SkinnedLodCheckpoint {
    pub fn to_json(&self, max_bytes: usize) -> Result<String, Error> {
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
                    return Err(std::io::Error::other("skeletal LOD checkpoint byte cap"));
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
        serde_json::to_writer(&mut w, self)?;
        String::from_utf8(w.bytes).map_err(|_| Error::Invalid)
    }
    pub fn from_json(json: &str, max_bytes: usize) -> Result<Self, Error> {
        if json.len() > max_bytes {
            return Err(Error::Capacity);
        }
        Ok(serde_json::from_str(json)?)
    }
    fn admit(&self, limits: SkinnedLodCheckpointLimits) -> Result<(), Error> {
        if self.version != 1 {
            return Err(Error::Invalid);
        }
        if self.mesh.len() > limits.mesh().geometry.max_bytes {
            return Err(Error::Capacity);
        }
        self.to_json(limits.max_bytes)?;
        Ok(())
    }
    pub fn prepare(&self, limits: SkinnedLodCheckpointLimits) -> Result<SkinnedLodMesh, Error> {
        self.admit(limits)?;
        let mut meshes = SkinnedMeshSetCheckpoint::from_bytes(&self.mesh, limits.mesh())?
            .prepare(limits.mesh())?;
        if meshes.len() != 1 {
            return Err(Error::Invalid);
        }
        self.prepare_with_mesh(Arc::new(meshes.remove(0)), limits)
    }
    /// Preserve the enclosing model's mesh Arc instead of allocating an unrelated source.
    /// The caller supplies the explicit resource binding; all source/cache words must match.
    pub fn prepare_with_mesh(
        &self,
        mesh: Arc<SkinnedMesh>,
        limits: SkinnedLodCheckpointLimits,
    ) -> Result<SkinnedLodMesh, Error> {
        self.admit(limits)?;
        let actual =
            SkinnedMeshSetCheckpoint::capture(std::slice::from_ref(mesh.as_ref()), limits.mesh())?
                .to_bytes(limits.mesh())?;
        if actual != self.mesh {
            return Err(Error::Invalid);
        }
        let result = self.certificate.prepare_skin_source(mesh, limits.lod)?;
        if result.checkpoint(limits)? != *self {
            return Err(Error::Invalid);
        }
        Ok(result)
    }
}
#[cfg(test)]
mod tests;
