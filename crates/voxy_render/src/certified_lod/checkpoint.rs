//! Proof-carrying accepted LOD resources; error values are exact receipts only.
use super::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, thiserror::Error)]
pub enum CertifiedLodCheckpointError {
    #[error("invalid certified LOD checkpoint or compiler receipt")]
    Invalid,
    #[error("certified LOD checkpoint exceeds policy")]
    Capacity,
    #[error("native LOD certification failed: {0:?}")]
    Certificate(CertifiedLodError),
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
}
type Error = CertifiedLodCheckpointError;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Triangle {
    target: usize,
    weights: [[u32; 3]; 3],
}
impl From<&LodTriangleWitness> for Triangle {
    fn from(v: &LodTriangleWitness) -> Self {
        let LodTriangleWitness {
            target_triangle,
            weights,
        } = v;
        Self {
            target: *target_triangle,
            weights: *weights,
        }
    }
}
impl Triangle {
    fn native(&self) -> LodTriangleWitness {
        LodTriangleWitness {
            target_triangle: self.target,
            weights: self.weights,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Subdivision {
    depth: u8,
    cells: Vec<Triangle>,
}
impl From<&crate::LodSubdivisionWitness> for Subdivision {
    fn from(v: &crate::LodSubdivisionWitness) -> Self {
        let crate::LodSubdivisionWitness { depth, cells } = v;
        Self {
            depth: *depth,
            cells: cells.iter().map(Triangle::from).collect(),
        }
    }
}
impl Subdivision {
    fn native(&self) -> crate::LodSubdivisionWitness {
        crate::LodSubdivisionWitness {
            depth: self.depth,
            cells: self.cells.iter().map(Triangle::native).collect(),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Proofs {
    Whole(Vec<(Vec<Triangle>, Vec<Triangle>)>),
    Subdivided(Vec<(Vec<Subdivision>, Vec<Subdivision>)>),
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Level {
    indices: Vec<u32>,
    error: u64,
    count: u32,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CertifiedLodCheckpoint {
    version: u32,
    positions: Vec<[u32; 3]>,
    levels: Vec<Level>,
    index_bytes: u64,
    proofs: Proofs,
}
fn add(a: &mut usize, n: usize) -> Result<(), Error> {
    *a = a.checked_add(n).ok_or(Error::Capacity)?;
    Ok(())
}
impl CertifiedLodIndexSet {
    pub fn checkpoint(
        &self,
        limits: crate::LodArchiveLimits,
    ) -> Result<CertifiedLodCheckpoint, Error> {
        let Self {
            positions,
            indices,
            verification,
        } = self;
        // Admit retained stream counts before cloning their records.
        let mut count = 0;
        let mut cells = 0;
        if positions.len() > limits.positions || indices.levels().len() > limits.levels {
            return Err(Error::Capacity);
        }
        for level in 0..indices.levels().len() {
            add(
                &mut count,
                indices.indices(level).ok_or(Error::Invalid)?.len(),
            )?;
        }
        match verification {
            Verification::Whole(proofs) => {
                for p in proofs {
                    add(&mut cells, p.forward.len())?;
                    add(&mut cells, p.reverse.len())?;
                }
            }
            Verification::Subdivided(proofs) => {
                for p in proofs {
                    for s in p.forward.iter().chain(&p.reverse) {
                        add(&mut cells, s.cells.len())?;
                    }
                }
            }
        }
        if count > limits.indices || cells > limits.cells {
            return Err(Error::Capacity);
        }
        let proofs = match verification {
            Verification::Whole(proofs) => Proofs::Whole(
                proofs
                    .iter()
                    .map(|p| {
                        (
                            p.forward.iter().map(Triangle::from).collect(),
                            p.reverse.iter().map(Triangle::from).collect(),
                        )
                    })
                    .collect(),
            ),
            Verification::Subdivided(proofs) => Proofs::Subdivided(
                proofs
                    .iter()
                    .map(|p| {
                        (
                            p.forward.iter().map(Subdivision::from).collect(),
                            p.reverse.iter().map(Subdivision::from).collect(),
                        )
                    })
                    .collect(),
            ),
        };
        let levels = indices
            .levels()
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let crate::LodLevel {
                    object_error,
                    index_count,
                } = v;
                Ok(Level {
                    indices: indices.indices(i).ok_or(Error::Invalid)?.to_vec(),
                    error: object_error.to_bits(),
                    count: *index_count,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let data = CertifiedLodCheckpoint {
            version: 1,
            positions: positions.iter().map(|v| v.map(f32::to_bits)).collect(),
            levels,
            index_bytes: indices.index_bytes(),
            proofs,
        };
        data.admit(limits)?;
        data.to_json(limits.bytes)?;
        Ok(data)
    }
}
enum NativeVariants {
    Whole(Vec<CertifiedLodVariant>),
    Subdivided(Vec<CertifiedLodSubdivisionVariant>),
}
impl CertifiedLodCheckpoint {
    fn admit(&self, limits: crate::LodArchiveLimits) -> Result<(), Error> {
        if self.version != 1 || self.levels.is_empty() {
            return Err(Error::Invalid);
        }
        if self.positions.len() > limits.positions || self.levels.len() > limits.levels {
            return Err(Error::Capacity);
        }
        let mut indices = 0;
        for level in &self.levels {
            add(&mut indices, level.indices.len())?;
        }
        let mut cells = 0;
        let proof_count = match &self.proofs {
            Proofs::Whole(proofs) => {
                for (a, b) in proofs {
                    add(&mut cells, a.len())?;
                    add(&mut cells, b.len())?;
                }
                proofs.len()
            }
            Proofs::Subdivided(proofs) => {
                for (a, b) in proofs {
                    // A stored depth must have exactly the native uniform cell coverage.
                    for v in a.iter().chain(b) {
                        let n = crate::lod_subdivision::cell_count(v.depth)
                            .map_err(|_| Error::Invalid)?;
                        if n != v.cells.len() {
                            return Err(Error::Invalid);
                        }
                        add(&mut cells, n)?;
                    }
                }
                proofs.len()
            }
        };
        if proof_count.checked_add(1) != Some(self.levels.len()) {
            return Err(Error::Invalid);
        }
        if indices > limits.indices || cells > limits.cells {
            return Err(Error::Capacity);
        }
        Ok(())
    }
    pub fn to_json(&self, max_bytes: usize) -> Result<String, Error> {
        struct Writer {
            data: Vec<u8>,
            limit: usize,
        }
        impl std::io::Write for Writer {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if self
                    .data
                    .len()
                    .checked_add(bytes.len())
                    .is_none_or(|n| n > self.limit)
                {
                    return Err(std::io::Error::other("LOD checkpoint byte cap"));
                }
                self.data.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut w = Writer {
            data: Vec::new(),
            limit: max_bytes,
        };
        serde_json::to_writer(&mut w, self)?;
        String::from_utf8(w.data).map_err(|_| Error::Invalid)
    }
    pub fn from_json(json: &str, max_bytes: usize) -> Result<Self, Error> {
        if json.len() > max_bytes {
            return Err(Error::Capacity);
        }
        Ok(serde_json::from_str(json)?)
    }
    fn native_variants(&self) -> NativeVariants {
        match &self.proofs {
            Proofs::Whole(proofs) => NativeVariants::Whole(
                proofs
                    .iter()
                    .zip(self.levels.iter().skip(1))
                    .map(|((a, b), level)| CertifiedLodVariant {
                        indices: level.indices.clone(),
                        source_to_variant: a.iter().map(Triangle::native).collect(),
                        variant_to_source: b.iter().map(Triangle::native).collect(),
                    })
                    .collect(),
            ),
            Proofs::Subdivided(proofs) => NativeVariants::Subdivided(
                proofs
                    .iter()
                    .zip(self.levels.iter().skip(1))
                    .map(|((a, b), level)| CertifiedLodSubdivisionVariant {
                        indices: level.indices.clone(),
                        source_to_variant: a.iter().map(Subdivision::native).collect(),
                        variant_to_source: b.iter().map(Subdivision::native).collect(),
                    })
                    .collect(),
            ),
        }
    }
    pub(crate) fn prepare_skin_source(
        &self,
        mesh: std::sync::Arc<crate::SkinnedMesh>,
        limits: crate::LodArchiveLimits,
    ) -> Result<crate::SkinnedLodMesh, crate::SkinnedLodCheckpointError> {
        use crate::SkinnedLodCheckpointError as SkinError;
        let certified = self.prepare(limits).map_err(SkinError::Certificate)?;
        if !certified.matches_positions(mesh.vertices().iter().map(|v| v.position))
            || certified.indices().indices(0) != Some(mesh.indices())
        {
            return Err(SkinError::Invalid);
        }
        drop(certified);
        match self.native_variants() {
            NativeVariants::Whole(variants) => crate::SkinnedLodMesh::new(mesh, variants),
            NativeVariants::Subdivided(variants) => {
                crate::SkinnedLodMesh::new_subdivided(mesh, variants)
            }
        }
        .map_err(SkinError::Native)
    }
    pub fn prepare(&self, limits: crate::LodArchiveLimits) -> Result<CertifiedLodIndexSet, Error> {
        self.admit(limits)?;
        self.to_json(limits.bytes)?;
        let positions = self
            .positions
            .iter()
            .map(|v| v.map(f32::from_bits))
            .collect();
        let base = self.levels[0].indices.clone();
        let result = match self.native_variants() {
            NativeVariants::Whole(variants) => CertifiedLodIndexSet::new(positions, base, variants),
            NativeVariants::Subdivided(variants) => {
                CertifiedLodIndexSet::new_subdivided(positions, base, variants)
            }
        }
        .map_err(Error::Certificate)?;
        if result.checkpoint(limits)? != *self {
            return Err(Error::Invalid);
        }
        Ok(result)
    }
}
#[cfg(test)]
mod tests;
