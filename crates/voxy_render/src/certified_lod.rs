use crate::{
    LodCertificateError, LodError, LodIndexSet, LodSurface, LodTriangleWitness, certify_lod_error,
};

/// Importer input; the constructor verifies witnesses and derives errors itself.
#[derive(Debug)]
pub struct CertifiedLodVariant {
    pub indices: Vec<u32>,
    pub source_to_variant: Vec<LodTriangleWitness>,
    pub variant_to_source: Vec<LodTriangleWitness>,
}

/// Complete subdivision witnesses for an immutable shared-vertex variant.
#[derive(Debug)]
pub struct CertifiedLodSubdivisionVariant {
    pub indices: Vec<u32>,
    pub source_to_variant: Vec<crate::LodSubdivisionWitness>,
    pub variant_to_source: Vec<crate::LodSubdivisionWitness>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertifiedLodError {
    Certificate(LodCertificateError),
    Levels(LodError),
}

/// Verified immutable geometry errors bound to exact position bits and indices.
/// Verification witnesses are discarded after successful construction.
#[derive(Debug)]
pub struct CertifiedLodIndexSet {
    positions: Box<[[f32; 3]]>,
    indices: LodIndexSet,
}
impl CertifiedLodIndexSet {
    /// Verifies complete subdivided coverage directly against the original mesh.
    /// Each published error is a conservative monotonic envelope of verified bounds.
    /// # Errors
    /// Rejects invalid surfaces, incomplete cells, excessive depth and increasing counts.
    pub fn new_subdivided(
        positions: Vec<[f32; 3]>,
        base: Vec<u32>,
        variants: Vec<CertifiedLodSubdivisionVariant>,
    ) -> Result<Self, CertifiedLodError> {
        validate_base(&positions, &base)?;
        let mut levels = vec![(0.0, base)];
        let mut error = 0.0_f64;
        for variant in variants {
            let measured = crate::certify_subdivided_lod_error(
                LodSurface {
                    positions: &positions,
                    indices: &levels[0].1,
                },
                LodSurface {
                    positions: &positions,
                    indices: &variant.indices,
                },
                &variant.source_to_variant,
                &variant.variant_to_source,
            )
            .map_err(CertifiedLodError::Certificate)?;
            error = error.max(measured);
            levels.push((error, variant.indices));
        }
        finish(positions, levels)
    }
    /// Verifies every variant against the original surface, never a preceding
    /// approximation. Errors form a conservative monotonic envelope of verified
    /// bounds. This does not certify UVs, normals, colors, materials or animation.
    /// # Errors
    /// Rejects invalid surfaces/witnesses or increasing triangle counts.
    pub fn new(
        positions: Vec<[f32; 3]>,
        base: Vec<u32>,
        variants: Vec<CertifiedLodVariant>,
    ) -> Result<Self, CertifiedLodError> {
        validate_base(&positions, &base)?;
        let mut levels = vec![(0.0, base)];
        let mut error = 0.0_f64;
        for variant in variants {
            let measured = certify_lod_error(
                LodSurface {
                    positions: &positions,
                    indices: &levels[0].1,
                },
                LodSurface {
                    positions: &positions,
                    indices: &variant.indices,
                },
                &variant.source_to_variant,
                &variant.variant_to_source,
            )
            .map_err(CertifiedLodError::Certificate)?;
            error = error.max(measured);
            levels.push((error, variant.indices));
        }
        finish(positions, levels)
    }
    #[must_use]
    pub fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }
    #[must_use]
    pub fn indices(&self) -> &LodIndexSet {
        &self.indices
    }
    pub(crate) fn matches_positions(&self, positions: impl Iterator<Item = [f32; 3]>) -> bool {
        let mut positions = positions;
        self.positions.iter().all(|expected| {
            positions
                .next()
                .is_some_and(|actual| actual.map(f32::to_bits) == expected.map(f32::to_bits))
        }) && positions.next().is_none()
    }
}

fn validate_base(positions: &[[f32; 3]], base: &[u32]) -> Result<(), CertifiedLodError> {
    if positions.iter().flatten().any(|p| !p.is_finite()) {
        return Err(CertifiedLodError::Certificate(
            LodCertificateError::InvalidSurface,
        ));
    }
    LodIndexSet::new(positions.len(), vec![(0.0, base.to_vec())])
        .map_err(CertifiedLodError::Levels)?;
    Ok(())
}
fn finish(
    positions: Vec<[f32; 3]>,
    levels: Vec<(f64, Vec<u32>)>,
) -> Result<CertifiedLodIndexSet, CertifiedLodError> {
    let indices = LodIndexSet::new(positions.len(), levels).map_err(CertifiedLodError::Levels)?;
    Ok(CertifiedLodIndexSet {
        positions: positions.into_boxed_slice(),
        indices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LOD_BARYCENTRIC_DENOMINATOR as D;
    const POSITIONS: [[f32; 3]; 4] = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]];
    fn variant() -> CertifiedLodVariant {
        CertifiedLodVariant {
            indices: vec![0, 1, 2],
            source_to_variant: vec![
                LodTriangleWitness {
                    target_triangle: 0,
                    weights: [[D, 0, 0], [0, D, 0], [0, D / 2, D / 2]],
                },
                LodTriangleWitness {
                    target_triangle: 0,
                    weights: [[D, 0, 0], [0, D / 2, D / 2], [0, 0, D]],
                },
            ],
            variant_to_source: vec![LodTriangleWitness {
                target_triangle: 0,
                weights: [[D, 0, 0], [0, D, 0], [D / 2, 0, D / 2]],
            }],
        }
    }
    #[test]
    fn derives_bound_and_binds_exact_source_bits() {
        let artifact =
            CertifiedLodIndexSet::new(POSITIONS.to_vec(), vec![0, 1, 3, 0, 3, 2], vec![variant()])
                .unwrap();
        let levels = artifact.indices().levels();
        assert_eq!(levels[0].index_count, 6);
        assert_eq!(levels[1].index_count, 3);
        assert!(levels[1].object_error >= 0.5_f64.sqrt());
        assert!(levels[1].object_error < 0.708);
        assert!(artifact.matches_positions(POSITIONS.into_iter()));
        let mut changed = POSITIONS;
        changed[3][2] = 1.0;
        assert!(!artifact.matches_positions(changed.into_iter()));
        changed = POSITIONS;
        changed[0][0] = -0.0;
        assert!(!artifact.matches_positions(changed.into_iter()));
        assert!(!artifact.matches_positions(POSITIONS[..3].iter().copied()));
    }
    #[test]
    fn rejects_unverified_artifacts_and_invalid_base() {
        let mut broken = variant();
        broken.variant_to_source.clear();
        assert!(matches!(
            CertifiedLodIndexSet::new(POSITIONS.to_vec(), vec![0, 1, 3, 0, 3, 2], vec![broken]),
            Err(CertifiedLodError::Certificate(
                LodCertificateError::InvalidWitness
            ))
        ));
        assert!(CertifiedLodIndexSet::new(POSITIONS.to_vec(), vec![], vec![]).is_err());
        let mut nonfinite = POSITIONS;
        nonfinite[0][0] = f32::NAN;
        assert!(CertifiedLodIndexSet::new(nonfinite.to_vec(), vec![0, 1, 2], vec![]).is_err());
    }
}
