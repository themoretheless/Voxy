use crate::{
    CertifiedLodError, CertifiedLodIndexSet, CertifiedLodSubdivisionVariant, LodSubdivisionWitness,
    LodTriangleWitness,
};

const MAGIC: &[u8; 8] = b"VOXYLCD1";
/// Cumulative limits, including both witness directions and all index variants.
#[derive(Clone, Copy, Debug)]
pub struct LodArchiveLimits {
    pub bytes: usize,
    pub positions: usize,
    pub levels: usize,
    pub indices: usize,
    pub cells: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LodArchiveError {
    InvalidFormat,
    BudgetExceeded,
    Certificate(CertifiedLodError),
}

/// Encodes a little-endian certificate proposal. No error scalar is serialized;
/// loading derives it from geometry and complete witness coverage. Encoding is
/// not certification; semantic validation is mandatory during decoding.
/// # Errors
/// Rejects exhausted limits and counts/triangle IDs outside the u32 wire format.
pub fn encode_lod_archive(
    positions: &[[f32; 3]],
    base: &[u32],
    variants: &[CertifiedLodSubdivisionVariant],
    limits: LodArchiveLimits,
) -> Result<Vec<u8>, LodArchiveError> {
    admit(positions.len(), limits.positions)?;
    admit(
        variants
            .len()
            .checked_add(1)
            .ok_or(LodArchiveError::BudgetExceeded)?,
        limits.levels,
    )?;
    let mut writer = Writer {
        bytes: Vec::new(),
        limit: limits.bytes,
    };
    writer.push(MAGIC)?;
    writer.count(positions.len())?;
    for point in positions {
        for value in point {
            writer.integer(value.to_bits())?;
        }
    }
    let mut indices = 0;
    writer.indices(base, &mut indices, limits.indices)?;
    writer.count(variants.len())?;
    let mut cells = 0;
    for variant in variants {
        writer.indices(&variant.indices, &mut indices, limits.indices)?;
        writer.witnesses(&variant.source_to_variant, &mut cells, limits.cells)?;
        writer.witnesses(&variant.variant_to_source, &mut cells, limits.cells)?;
    }
    Ok(writer.bytes)
}

/// Loads bounded data and re-verifies geometry before publishing an immutable set.
/// Payload lengths are checked before allocation, counts are cumulative, and
/// trailing bytes/version mismatches reject. No stored optimizer/error claim is trusted.
/// # Errors
/// Rejects invalid/truncated payloads, exhausted limits and failed geometry proofs.
pub fn decode_lod_archive(
    bytes: &[u8],
    limits: LodArchiveLimits,
) -> Result<CertifiedLodIndexSet, LodArchiveError> {
    let proposal = decode_proposal(bytes, limits)?;
    CertifiedLodIndexSet::new_subdivided(proposal.positions, proposal.base, proposal.variants)
        .map_err(LodArchiveError::Certificate)
}

struct Proposal {
    positions: Vec<[f32; 3]>,
    base: Vec<u32>,
    variants: Vec<CertifiedLodSubdivisionVariant>,
}
fn decode_proposal(bytes: &[u8], limits: LodArchiveLimits) -> Result<Proposal, LodArchiveError> {
    admit(bytes.len(), limits.bytes)?;
    admit(1, limits.levels)?;
    let mut reader = Reader { bytes, cursor: 0 };
    if reader.take(8)? != MAGIC {
        return Err(LodArchiveError::InvalidFormat);
    }
    let count = reader.count(12, &mut 0, limits.positions)?;
    let positions = (0..count)
        .map(|_| {
            Ok([
                f32::from_bits(reader.integer()?),
                f32::from_bits(reader.integer()?),
                f32::from_bits(reader.integer()?),
            ])
        })
        .collect::<Result<Vec<_>, LodArchiveError>>()?;
    let mut indices = 0;
    let base = reader.indices(&mut indices, limits.indices)?;
    let count = reader.count(12, &mut 0, limits.levels - 1)?;
    let mut cells = 0;
    let variants = (0..count)
        .map(|_| {
            let variant = reader.indices(&mut indices, limits.indices)?;
            let forward = reader.witnesses(base.len() / 3, &mut cells, limits.cells)?;
            let reverse = reader.witnesses(variant.len() / 3, &mut cells, limits.cells)?;
            Ok(CertifiedLodSubdivisionVariant {
                indices: variant,
                source_to_variant: forward,
                variant_to_source: reverse,
            })
        })
        .collect::<Result<Vec<_>, LodArchiveError>>()?;
    if reader.cursor != bytes.len() {
        return Err(LodArchiveError::InvalidFormat);
    }
    Ok(Proposal {
        positions,
        base,
        variants,
    })
}

#[derive(Debug)]
pub enum SkinnedLodArchiveError {
    Archive(LodArchiveError),
    SourceMismatch,
    Quality(crate::SkinnedLodError),
}
impl std::fmt::Display for SkinnedLodArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "skeletal LOD archive error: {self:?}")
    }
}
impl std::error::Error for SkinnedLodArchiveError {}

/// Uses the same bounded archive parser, retaining verified coverage for future
/// poses. Exact rest-position bits and base indices must match the skeletal mesh.
/// # Errors
/// Rejects archive quotas/format/proof failures or source identity mismatch.
pub fn decode_skinned_lod_archive(
    mesh: std::sync::Arc<crate::SkinnedMesh>,
    bytes: &[u8],
    limits: LodArchiveLimits,
) -> Result<crate::SkinnedLodMesh, SkinnedLodArchiveError> {
    let proposal = decode_proposal(bytes, limits).map_err(SkinnedLodArchiveError::Archive)?;
    if proposal.positions.len() != mesh.vertices().len()
        || proposal.base != mesh.indices()
        || proposal
            .positions
            .iter()
            .zip(mesh.vertices())
            .any(|(point, vertex)| point.map(f32::to_bits) != vertex.position.map(f32::to_bits))
    {
        return Err(SkinnedLodArchiveError::SourceMismatch);
    }
    crate::SkinnedLodMesh::new_subdivided(mesh, proposal.variants)
        .map_err(SkinnedLodArchiveError::Quality)
}
fn admit(count: usize, limit: usize) -> Result<(), LodArchiveError> {
    if count > limit {
        Err(LodArchiveError::BudgetExceeded)
    } else {
        Ok(())
    }
}
fn charge(total: &mut usize, count: usize, limit: usize) -> Result<(), LodArchiveError> {
    let next = total
        .checked_add(count)
        .ok_or(LodArchiveError::BudgetExceeded)?;
    admit(next, limit)?;
    *total = next;
    Ok(())
}
struct Writer {
    bytes: Vec<u8>,
    limit: usize,
}
impl Writer {
    fn push(&mut self, bytes: &[u8]) -> Result<(), LodArchiveError> {
        admit(
            self.bytes
                .len()
                .checked_add(bytes.len())
                .ok_or(LodArchiveError::BudgetExceeded)?,
            self.limit,
        )?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn integer(&mut self, value: u32) -> Result<(), LodArchiveError> {
        self.push(&value.to_le_bytes())
    }
    fn count(&mut self, value: usize) -> Result<(), LodArchiveError> {
        self.integer(u32::try_from(value).map_err(|_| LodArchiveError::InvalidFormat)?)
    }
    fn indices(
        &mut self,
        indices: &[u32],
        total: &mut usize,
        limit: usize,
    ) -> Result<(), LodArchiveError> {
        charge(total, indices.len(), limit)?;
        self.count(indices.len())?;
        for index in indices {
            self.integer(*index)?;
        }
        Ok(())
    }
    fn witnesses(
        &mut self,
        witnesses: &[LodSubdivisionWitness],
        total: &mut usize,
        limit: usize,
    ) -> Result<(), LodArchiveError> {
        self.count(witnesses.len())?;
        for witness in witnesses {
            charge(total, witness.cells.len(), limit)?;
            self.push(&[witness.depth])?;
            self.count(witness.cells.len())?;
            for cell in &witness.cells {
                self.count(cell.target_triangle)?;
                for weights in cell.weights {
                    for weight in weights {
                        self.integer(weight)?;
                    }
                }
            }
        }
        Ok(())
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}
impl Reader<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8], LodArchiveError> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or(LodArchiveError::InvalidFormat)?;
        let result = self
            .bytes
            .get(self.cursor..end)
            .ok_or(LodArchiveError::InvalidFormat)?;
        self.cursor = end;
        Ok(result)
    }
    fn integer(&mut self) -> Result<u32, LodArchiveError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| LodArchiveError::InvalidFormat)?,
        ))
    }
    fn count(
        &mut self,
        minimum_bytes: usize,
        total: &mut usize,
        limit: usize,
    ) -> Result<usize, LodArchiveError> {
        let count = usize::try_from(self.integer()?).map_err(|_| LodArchiveError::InvalidFormat)?;
        charge(total, count, limit)?;
        let bytes = count
            .checked_mul(minimum_bytes)
            .ok_or(LodArchiveError::InvalidFormat)?;
        if bytes > self.bytes.len() - self.cursor {
            return Err(LodArchiveError::InvalidFormat);
        }
        Ok(count)
    }
    fn indices(&mut self, total: &mut usize, limit: usize) -> Result<Vec<u32>, LodArchiveError> {
        let count = self.count(4, total, limit)?;
        (0..count).map(|_| self.integer()).collect()
    }
    fn witnesses(
        &mut self,
        expected: usize,
        total: &mut usize,
        limit: usize,
    ) -> Result<Vec<LodSubdivisionWitness>, LodArchiveError> {
        let count = self.count(5, &mut 0, expected)?;
        if count != expected {
            return Err(LodArchiveError::InvalidFormat);
        }
        (0..count)
            .map(|_| {
                let depth = self.take(1)?[0];
                let expected = crate::lod_subdivision::cell_count(depth)
                    .map_err(|_| LodArchiveError::InvalidFormat)?;
                let count = self.count(40, total, limit)?;
                if count != expected {
                    return Err(LodArchiveError::InvalidFormat);
                }
                let cells = (0..count)
                    .map(|_| {
                        let target_triangle = usize::try_from(self.integer()?)
                            .map_err(|_| LodArchiveError::InvalidFormat)?;
                        let mut weights = [[0; 3]; 3];
                        for row in &mut weights {
                            for weight in row {
                                *weight = self.integer()?;
                            }
                        }
                        Ok(LodTriangleWitness {
                            target_triangle,
                            weights,
                        })
                    })
                    .collect::<Result<Vec<_>, LodArchiveError>>()?;
                Ok(LodSubdivisionWitness { depth, cells })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const LIMITS: LodArchiveLimits = LodArchiveLimits {
        bytes: 4096,
        positions: 3,
        levels: 2,
        indices: 6,
        cells: 2,
    };
    fn archive() -> Vec<u8> {
        let positions = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let surface = crate::LodSurface {
            positions: &positions,
            indices: &[0, 1, 2],
        };
        let generated = crate::generate_subdivided_lod_witnesses(surface, surface, 0, 2).unwrap();
        encode_lod_archive(
            &positions,
            surface.indices,
            &[CertifiedLodSubdivisionVariant {
                indices: vec![0, 1, 2],
                source_to_variant: generated.source_to_approximation,
                variant_to_source: generated.approximation_to_source,
            }],
            LIMITS,
        )
        .unwrap()
    }
    #[test]
    fn roundtrip_reverifies_and_preserves_source_bits() {
        let bytes = archive();
        let restored = decode_lod_archive(&bytes, LIMITS).unwrap();
        assert_eq!(
            restored.positions()[1].map(f32::to_bits),
            [1.0_f32.to_bits(), 0, 0]
        );
        assert_eq!(
            restored.indices().levels()[1].object_error.to_bits(),
            0.0_f64.to_bits()
        );
        assert_eq!(restored.indices().indices(1), Some([0, 1, 2].as_slice()));
    }
    #[test]
    fn rejects_truncation_trailing_version_and_forged_counts() {
        let bytes = archive();
        for end in 0..bytes.len() {
            assert!(decode_lod_archive(&bytes[..end], LIMITS).is_err());
        }
        let mut changed = bytes.clone();
        changed.push(0);
        assert!(decode_lod_archive(&changed, LIMITS).is_err());
        changed = bytes.clone();
        changed[7] = b'2';
        assert_eq!(
            decode_lod_archive(&changed, LIMITS).unwrap_err(),
            LodArchiveError::InvalidFormat
        );
        changed = bytes;
        changed[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            decode_lod_archive(&changed, LIMITS).unwrap_err(),
            LodArchiveError::BudgetExceeded
        );
    }
    #[test]
    fn rejects_semantic_corruption_and_cumulative_budget_overruns() {
        let bytes = archive();
        let mut changed = bytes.clone();
        changed[12..16].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
        assert!(matches!(
            decode_lod_archive(&changed, LIMITS),
            Err(LodArchiveError::Certificate(_))
        ));
        changed = bytes.clone();
        let end = changed.len();
        changed[end - 4..].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            decode_lod_archive(&changed, LIMITS),
            Err(LodArchiveError::Certificate(_))
        ));
        for limits in [
            LodArchiveLimits {
                bytes: bytes.len() - 1,
                ..LIMITS
            },
            LodArchiveLimits {
                positions: 2,
                ..LIMITS
            },
            LodArchiveLimits {
                levels: 1,
                ..LIMITS
            },
            LodArchiveLimits {
                indices: 5,
                ..LIMITS
            },
            LodArchiveLimits { cells: 1, ..LIMITS },
        ] {
            assert_eq!(
                decode_lod_archive(&bytes, limits).unwrap_err(),
                LodArchiveError::BudgetExceeded
            );
        }
    }
    #[test]
    fn encoder_and_decoder_enforce_exact_cumulative_limits() {
        let positions = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let surface = crate::LodSurface {
            positions: &positions,
            indices: &[0, 1, 2],
        };
        let generated = crate::generate_subdivided_lod_witnesses(surface, surface, 0, 2).unwrap();
        let variants = [CertifiedLodSubdivisionVariant {
            indices: vec![0, 1, 2],
            source_to_variant: generated.source_to_approximation,
            variant_to_source: generated.approximation_to_source,
        }];
        let bytes = encode_lod_archive(&positions, surface.indices, &variants, LIMITS).unwrap();
        let exact = LodArchiveLimits {
            bytes: bytes.len(),
            ..LIMITS
        };
        assert!(decode_lod_archive(&bytes, exact).is_ok());
        assert_eq!(
            encode_lod_archive(&positions, surface.indices, &variants, exact).unwrap(),
            bytes
        );
        for blocked in [
            LodArchiveLimits {
                bytes: bytes.len() - 1,
                ..exact
            },
            LodArchiveLimits {
                indices: 5,
                ..exact
            },
            LodArchiveLimits { cells: 1, ..exact },
        ] {
            assert_eq!(
                encode_lod_archive(&positions, surface.indices, &variants, blocked).unwrap_err(),
                LodArchiveError::BudgetExceeded
            );
        }
    }
}
