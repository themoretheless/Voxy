use crate::{
    LOD_BARYCENTRIC_DENOMINATOR as D, LodCertificateError, LodSurface, LodTriangleWitness,
};

/// Complete uniform dyadic subdivision of one original triangle. Cell order is
/// canonical: three corner children followed by the central child, recursively.
/// Missing/extra cells are rejected; callers cannot supply arbitrary cell shapes.
#[derive(Clone, Debug)]
pub struct LodSubdivisionWitness {
    pub depth: u8,
    pub cells: Vec<LodTriangleWitness>,
}

#[derive(Debug)]
pub struct LodSubdivisionWitnesses {
    pub source_to_approximation: Vec<LodSubdivisionWitness>,
    pub approximation_to_source: Vec<LodSubdivisionWitness>,
    pub object_error: f64,
}

/// Bounded reference producer using complete uniform subdivisions in both directions.
/// Pair work includes every subdivision cell and is admitted before allocation.
/// # Errors
/// Rejects invalid surfaces/depth or excessive/overflowing pair work.
pub fn generate_subdivided_lod_witnesses(
    source: LodSurface<'_>,
    approximation: LodSurface<'_>,
    depth: u8,
    max_triangle_pairs: u64,
) -> Result<LodSubdivisionWitnesses, crate::LodWitnessError> {
    use crate::LodWitnessError as Error;
    crate::lod_certificate::validate_surface(source).map_err(Error::Certificate)?;
    crate::lod_certificate::validate_surface(approximation).map_err(Error::Certificate)?;
    let count = cell_count(depth).map_err(Error::Certificate)?;
    let pairs = u64::try_from(source.indices.len() / 3)
        .ok()
        .and_then(|a| {
            u64::try_from(approximation.indices.len() / 3)
                .ok()
                .and_then(|b| a.checked_mul(b))
        })
        .and_then(|pairs| pairs.checked_mul(2))
        .and_then(|pairs| {
            u64::try_from(count)
                .ok()
                .and_then(|cells| pairs.checked_mul(cells))
        })
        .ok_or(Error::WorkBudgetExceeded)?;
    if pairs > max_triangle_pairs {
        return Err(Error::WorkBudgetExceeded);
    }
    let forward = produce(source, approximation, depth, count);
    let reverse = produce(approximation, source, depth, count);
    let object_error = certify_subdivided_lod_error(source, approximation, &forward, &reverse)
        .map_err(Error::Certificate)?;
    Ok(LodSubdivisionWitnesses {
        source_to_approximation: forward,
        approximation_to_source: reverse,
        object_error,
    })
}
fn produce(
    source: LodSurface<'_>,
    target: LodSurface<'_>,
    depth: u8,
    count: usize,
) -> Vec<LodSubdivisionWitness> {
    source
        .indices
        .chunks_exact(3)
        .map(|indices| {
            let triangle = std::array::from_fn(|i| source.positions[indices[i] as usize]);
            let cells = (0..count)
                .map(|index| {
                    let points = cell_weights(depth, index).map(|weights| {
                        crate::lod_certificate::barycentric_proxy(triangle, weights)
                            .0
                            .map(f64::from)
                    });
                    crate::lod_witness::best_witness(points, target)
                })
                .collect();
            LodSubdivisionWitness { depth, cells }
        })
        .collect()
}

/// Verifies both surface directions with independently reconstructed coverage.
/// Every source triangle has exactly 4^depth cells; each maps into one target
/// triangle. f32 proxy quantization is bounded separately and added outward.
/// Depth is limited to 12, preserving exact 24-bit dyadic barycentric coordinates.
/// This bounds geometry only; it does not certify topology or material attributes.
/// # Errors
/// Rejects invalid surfaces, excessive depth, incomplete coverage and invalid witnesses.
pub fn certify_subdivided_lod_error(
    source: LodSurface<'_>,
    approximation: LodSurface<'_>,
    forward: &[LodSubdivisionWitness],
    reverse: &[LodSubdivisionWitness],
) -> Result<f64, LodCertificateError> {
    crate::lod_certificate::validate_surface(source)?;
    crate::lod_certificate::validate_surface(approximation)?;
    Ok(
        directional(source, approximation, forward)?.max(directional(
            approximation,
            source,
            reverse,
        )?),
    )
}

pub(crate) fn cell_count(depth: u8) -> Result<usize, LodCertificateError> {
    if depth > 12 {
        return Err(LodCertificateError::InvalidWitness);
    }
    Ok(4_usize.pow(u32::from(depth)))
}
pub(crate) fn cell_weights(depth: u8, index: usize) -> [[u32; 3]; 3] {
    let mut triangle = [[D, 0, 0], [0, D, 0], [0, 0, D]];
    for level in (0..depth).rev() {
        let midpoint =
            |a: usize, b: usize| std::array::from_fn(|i| triangle[a][i].midpoint(triangle[b][i]));
        let ab = midpoint(0, 1);
        let bc = midpoint(1, 2);
        let ca = midpoint(2, 0);
        triangle = match (index >> (2 * level)) & 3 {
            0 => [triangle[0], ab, ca],
            1 => [ab, triangle[1], bc],
            2 => [ca, bc, triangle[2]],
            _ => [ab, bc, ca],
        };
    }
    triangle
}
fn directional(
    source: LodSurface<'_>,
    target: LodSurface<'_>,
    witnesses: &[LodSubdivisionWitness],
) -> Result<f64, LodCertificateError> {
    if witnesses.len() != source.indices.len() / 3 {
        return Err(LodCertificateError::InvalidWitness);
    }
    let mut bound = 0.0_f64;
    for (indices, witness) in source.indices.chunks_exact(3).zip(witnesses) {
        let count = cell_count(witness.depth)?;
        if witness.cells.len() != count {
            return Err(LodCertificateError::InvalidWitness);
        }
        let triangle = std::array::from_fn(|i| source.positions[indices[i] as usize]);
        for (index, cell) in witness.cells.iter().enumerate() {
            let proxies = cell_weights(witness.depth, index)
                .map(|weights| crate::lod_certificate::barycentric_proxy(triangle, weights));
            let positions = proxies.map(|(point, _)| point);
            let radius = proxies
                .into_iter()
                .map(|(_, radius)| radius)
                .fold(0.0_f64, f64::max);
            let projected = crate::lod_certificate::directional_bound(
                LodSurface {
                    positions: &positions,
                    indices: &[0, 1, 2],
                },
                target,
                std::slice::from_ref(cell),
            )?;
            let total = if radius == 0.0 {
                projected
            } else if projected == 0.0 {
                radius
            } else {
                (projected + radius).next_up()
            };
            bound = bound.max(total);
        }
    }
    Ok(bound)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subdivision_still_detects_removed_islands() {
        let positions = [
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 10.],
            [1., 0., 10.],
            [0., 1., 10.],
        ];
        let source = LodSurface {
            positions: &positions,
            indices: &[0, 1, 2, 3, 4, 5],
        };
        let approximation = LodSurface {
            positions: &positions,
            indices: &[0, 1, 2],
        };
        let generated = generate_subdivided_lod_witnesses(source, approximation, 1, 16).unwrap();
        assert!(generated.object_error >= 10.0 && generated.object_error < 10.001);
    }
    #[test]
    fn subdivision_removes_flat_retriangulation_overestimate() {
        let mut positions = Vec::new();
        for y in 0_u8..=4 {
            for x in 0_u8..=4 {
                positions.push([f32::from(x) / 4., f32::from(y) / 4., 0.]);
            }
        }
        let indices = |stride: usize| {
            let mut indices = Vec::new();
            for y in (0..4).step_by(stride) {
                for x in (0..4).step_by(stride) {
                    let a = y * 5 + x;
                    let b = a + stride;
                    let c = a + 5 * stride;
                    let d = c + stride;
                    indices.extend([a, b, d, a, d, c].map(|i| u32::try_from(i).unwrap()));
                }
            }
            indices
        };
        let fine = indices(1);
        let coarse = indices(2);
        let source = LodSurface {
            positions: &positions,
            indices: &fine,
        };
        let approximation = LodSurface {
            positions: &positions,
            indices: &coarse,
        };
        let original = crate::generate_lod_witnesses(source, approximation, 512).unwrap();
        assert!(original.object_error > 0.24);
        assert!(matches!(
            generate_subdivided_lod_witnesses(source, approximation, 1, 2047),
            Err(crate::LodWitnessError::WorkBudgetExceeded)
        ));
        let refined = generate_subdivided_lod_witnesses(source, approximation, 1, 2048).unwrap();
        assert!(refined.object_error < 1e-12);
        let artifact = crate::CertifiedLodIndexSet::new_subdivided(
            positions,
            fine,
            vec![crate::CertifiedLodSubdivisionVariant {
                indices: coarse,
                source_to_variant: refined.source_to_approximation,
                variant_to_source: refined.approximation_to_source,
            }],
        )
        .unwrap();
        assert!(artifact.indices().levels()[1].object_error < 1e-12);
        assert_eq!(artifact.indices().levels()[1].index_count, 24);
    }
    #[test]
    fn rejects_holes_extra_cells_and_excessive_depth() {
        let positions = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let surface = LodSurface {
            positions: &positions,
            indices: &[0, 1, 2],
        };
        let cell = LodTriangleWitness {
            target_triangle: 0,
            weights: [[D, 0, 0], [0, D, 0], [0, 0, D]],
        };
        for (depth, count) in [(1, 3), (1, 5), (13, 1)] {
            let witness = [LodSubdivisionWitness {
                depth,
                cells: vec![cell; count],
            }];
            assert_eq!(
                certify_subdivided_lod_error(surface, surface, &witness, &witness),
                Err(LodCertificateError::InvalidWitness)
            );
        }
    }
    #[test]
    fn canonical_subdivision_covers_the_whole_triangle() {
        let positions = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let surface = LodSurface {
            positions: &positions,
            indices: &[0, 1, 2],
        };
        for depth in 0..=3 {
            let cells = (0..cell_count(depth).unwrap())
                .map(|index| LodTriangleWitness {
                    target_triangle: 0,
                    weights: cell_weights(depth, index),
                })
                .collect();
            let witnesses = [LodSubdivisionWitness { depth, cells }];
            let error =
                certify_subdivided_lod_error(surface, surface, &witnesses, &witnesses).unwrap();
            assert!(error < 1e-14);
        }
    }
}
