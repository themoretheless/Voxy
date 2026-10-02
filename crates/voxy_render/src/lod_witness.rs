use crate::{
    LOD_BARYCENTRIC_DENOMINATOR, LodCertificateError, LodSurface, LodTriangleWitness,
    certify_lod_error,
};

/// Producer output for the surfaces supplied at generation time. Construct a
/// `CertifiedLodIndexSet` to bind it to owned immutable positions and indices.
#[derive(Debug)]
pub struct LodWitnesses {
    pub source_to_approximation: Vec<LodTriangleWitness>,
    pub approximation_to_source: Vec<LodTriangleWitness>,
    pub object_error: f64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LodWitnessError {
    Certificate(LodCertificateError),
    WorkBudgetExceeded,
}

/// Produces conservative bidirectional witnesses using a bounded all-pairs search.
/// For each source triangle, chooses one target triangle minimizing the largest
/// witnessed corner distance. Point projections are only proposals; quantized
/// weights and the resulting bound are checked by the independent verifier.
/// Degenerate/thin target triangles can give looser bounds, never uncertified ones.
/// `max_triangle_pairs` covers both directions and is checked before allocation.
/// This is an offline reference producer; it is not a scalable spatial accelerator.
/// # Errors
/// Rejects invalid surfaces, overflowing/excessive pair counts or invalid witnesses.
pub fn generate_lod_witnesses(
    source: LodSurface<'_>,
    approximation: LodSurface<'_>,
    max_triangle_pairs: u64,
) -> Result<LodWitnesses, LodWitnessError> {
    crate::lod_certificate::validate_surface(source).map_err(LodWitnessError::Certificate)?;
    crate::lod_certificate::validate_surface(approximation)
        .map_err(LodWitnessError::Certificate)?;
    let pairs = u64::try_from(source.indices.len() / 3)
        .ok()
        .and_then(|a| {
            u64::try_from(approximation.indices.len() / 3)
                .ok()
                .and_then(|b| a.checked_mul(b))
        })
        .and_then(|pairs| pairs.checked_mul(2))
        .ok_or(LodWitnessError::WorkBudgetExceeded)?;
    if pairs > max_triangle_pairs {
        return Err(LodWitnessError::WorkBudgetExceeded);
    }
    let forward = directional_witnesses(source, approximation);
    let reverse = directional_witnesses(approximation, source);
    let object_error = certify_lod_error(source, approximation, &forward, &reverse)
        .map_err(LodWitnessError::Certificate)?;
    Ok(LodWitnesses {
        source_to_approximation: forward,
        approximation_to_source: reverse,
        object_error,
    })
}

fn directional_witnesses(
    source: LodSurface<'_>,
    target: LodSurface<'_>,
) -> Vec<LodTriangleWitness> {
    source
        .indices
        .chunks_exact(3)
        .map(|triangle| {
            let points = triangle_points(triangle, source.positions);
            best_witness(points, target)
        })
        .collect()
}

pub(crate) fn best_witness(points: [[f64; 3]; 3], target: LodSurface<'_>) -> LodTriangleWitness {
    let mut best = LodTriangleWitness {
        target_triangle: 0,
        weights: [[0; 3]; 3],
    };
    let mut best_distance = f64::INFINITY;
    for (index, triangle) in target.indices.chunks_exact(3).enumerate() {
        let target_points = triangle_points(triangle, target.positions);
        let weights = points.map(|point| closest_weights(point, target_points));
        let distance = points
            .into_iter()
            .zip(weights)
            .map(|(point, weights)| squared_distance(point, weighted_point(target_points, weights)))
            .fold(0.0_f64, f64::max);
        if distance < best_distance {
            best_distance = distance;
            best = LodTriangleWitness {
                target_triangle: index,
                weights,
            };
        }
    }
    best
}

type Point = [f64; 3];
fn triangle_points(triangle: &[u32], positions: &[[f32; 3]]) -> [Point; 3] {
    std::array::from_fn(|i| positions[triangle[i] as usize].map(f64::from))
}
fn subtract(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: Point, b: Point) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}
fn squared_distance(a: Point, b: Point) -> f64 {
    let d = subtract(a, b);
    dot(d, d)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Deliberate floor after [0, 2^24] clamping.
fn quantize(weights: [f64; 3]) -> [u32; 3] {
    let denominator = LOD_BARYCENTRIC_DENOMINATOR;
    let a = (weights[0].clamp(0.0, 1.0) * f64::from(denominator)).floor() as u32;
    let b =
        ((weights[1].clamp(0.0, 1.0) * f64::from(denominator)).floor() as u32).min(denominator - a);
    [a, b, denominator - a - b]
}
fn weighted_point(triangle: [Point; 3], weights: [u32; 3]) -> Point {
    std::array::from_fn(|axis| {
        triangle
            .into_iter()
            .zip(weights)
            .map(|(point, weight)| {
                point[axis] * f64::from(weight) / f64::from(LOD_BARYCENTRIC_DENOMINATOR)
            })
            .sum()
    })
}

pub(crate) fn closest_weights(point: Point, triangle: [Point; 3]) -> [u32; 3] {
    let mut best = [LOD_BARYCENTRIC_DENOMINATOR, 0, 0];
    let mut distance = squared_distance(point, triangle[0]);
    let mut consider = |weights| {
        let weights = quantize(weights);
        let candidate = squared_distance(point, weighted_point(triangle, weights));
        if candidate < distance {
            distance = candidate;
            best = weights;
        }
    };
    for (a, b) in [(0, 1), (1, 2), (2, 0)] {
        let edge = subtract(triangle[b], triangle[a]);
        let length = dot(edge, edge);
        let t = if length > 0.0 {
            (dot(subtract(point, triangle[a]), edge) / length).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let mut weights = [0.0; 3];
        weights[a] = 1.0 - t;
        weights[b] = t;
        consider(weights);
    }
    let ab = subtract(triangle[1], triangle[0]);
    let ac = subtract(triangle[2], triangle[0]);
    let ap = subtract(point, triangle[0]);
    let aa = dot(ab, ab);
    let cc = dot(ac, ac);
    let cross = dot(ab, ac);
    let determinant = aa * cc - cross * cross;
    if determinant > 0.0 {
        let u = (dot(ap, ab) * cc - dot(ap, ac) * cross) / determinant;
        let v = (dot(ap, ac) * aa - dot(ap, ab) * cross) / determinant;
        if u >= 0.0 && v >= 0.0 && u + v <= 1.0 {
            consider([1.0 - u - v, u, v]);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_square_triangle_bound_with_exact_work_limit() {
        let positions = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]];
        let square = LodSurface {
            positions: &positions,
            indices: &[0, 1, 3, 0, 3, 2],
        };
        let triangle = LodSurface {
            positions: &positions,
            indices: &[0, 1, 2],
        };
        assert!(matches!(
            generate_lod_witnesses(square, triangle, 3),
            Err(LodWitnessError::WorkBudgetExceeded)
        ));
        let generated = generate_lod_witnesses(square, triangle, 4).unwrap();
        assert!(generated.object_error >= 0.5_f64.sqrt() && generated.object_error < 0.708);
        assert_eq!(generated.source_to_approximation.len(), 2);
        assert_eq!(generated.approximation_to_source.len(), 1);
        let artifact = crate::CertifiedLodIndexSet::new(
            positions.to_vec(),
            square.indices.to_vec(),
            vec![crate::CertifiedLodVariant {
                indices: triangle.indices.to_vec(),
                source_to_variant: generated.source_to_approximation,
                variant_to_source: generated.approximation_to_source,
            }],
        )
        .unwrap();
        assert_eq!(artifact.indices().levels()[1].index_count, 3);
        assert!(artifact.indices().levels()[1].object_error >= 0.5_f64.sqrt());
    }
    #[test]
    fn exact_identity_degenerate_and_removed_island() {
        let positions = [
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 10.],
            [1., 0., 10.],
            [0., 1., 10.],
        ];
        let base = LodSurface {
            positions: &positions,
            indices: &[0, 1, 2],
        };
        assert_eq!(
            generate_lod_witnesses(base, base, 2).unwrap().object_error,
            0.0
        );
        let islands = LodSurface {
            positions: &positions,
            indices: &[0, 1, 2, 3, 4, 5],
        };
        assert!(
            generate_lod_witnesses(base, islands, 4)
                .unwrap()
                .object_error
                >= 10.0
        );
        let degenerate = LodSurface {
            positions: &positions,
            indices: &[0, 0, 0],
        };
        let result = generate_lod_witnesses(base, degenerate, 2).unwrap();
        assert!(result.object_error >= 1.0 && result.object_error < 1.001);
    }
    #[test]
    fn thin_and_extreme_candidates_still_pass_the_verifier() {
        let large = [
            [f32::MAX, f32::MAX, 0.],
            [f32::MAX, -f32::MAX, 0.],
            [-f32::MAX, f32::MAX, 0.],
        ];
        let thin = [[1., 0., 1.], [-1., 0., 1.], [0., f32::MIN_POSITIVE, 1.]];
        let source = LodSurface {
            positions: &large,
            indices: &[0, 1, 2],
        };
        let approximation = LodSurface {
            positions: &thin,
            indices: &[0, 1, 2],
        };
        let generated = generate_lod_witnesses(source, approximation, 2).unwrap();
        assert!(generated.object_error.is_finite());
        assert!(generated.object_error >= f64::from(f32::MAX));
        let verified = certify_lod_error(
            source,
            approximation,
            &generated.source_to_approximation,
            &generated.approximation_to_source,
        )
        .unwrap();
        assert_eq!(generated.object_error.to_bits(), verified.to_bits());
    }
}
