//! Geometric LOD verification independent of simplifier costs and GPU ownership.

/// Exact denominator used by the integer barycentric witness format.
pub const LOD_BARYCENTRIC_DENOMINATOR: u32 = 1 << 24;

/// Triangle surface in its own vertex domain, in common object-space units.
#[derive(Clone, Copy, Debug)]
pub struct LodSurface<'a> {
    pub positions: &'a [[f32; 3]],
    pub indices: &'a [u32],
}

/// For one source triangle, maps its three corners into one target triangle.
/// Each row is nonnegative integer barycentric weights summing to 2^24.
/// Using one target triangle makes the interpolated correspondence valid over
/// the entire source triangle, including its interior and edges.
#[derive(Clone, Copy, Debug)]
pub struct LodTriangleWitness {
    pub target_triangle: usize,
    pub weights: [[u32; 3]; 3],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LodCertificateError {
    InvalidSurface,
    InvalidWitness,
}

/// Verifies a symmetric surface-distance upper bound from triangle witnesses.
/// Both directions are required: one-sided coverage could miss removed islands
/// or added geometry. No closest-point claim is trusted from the producer.
/// Positions are f32; dyadic products are exact in f64 and subsequent arithmetic
/// is rounded outward. This bounds geometry only, not UVs, normals, materials,
/// topology, winding, animation or image similarity. Work is linear in witnesses.
/// # Errors
/// Rejects empty/nonfinite surfaces, invalid indices, missing triangle witnesses,
/// foreign target triangles and weights that do not sum to the denominator.
pub fn certify_lod_error(
    source: LodSurface<'_>,
    approximation: LodSurface<'_>,
    source_to_approximation: &[LodTriangleWitness],
    approximation_to_source: &[LodTriangleWitness],
) -> Result<f64, LodCertificateError> {
    validate_surface(source)?;
    validate_surface(approximation)?;
    let forward = directional_bound(source, approximation, source_to_approximation)?;
    let reverse = directional_bound(approximation, source, approximation_to_source)?;
    Ok(forward.max(reverse))
}

pub(crate) fn validate_surface(surface: LodSurface<'_>) -> Result<(), LodCertificateError> {
    if surface.positions.is_empty()
        || surface.indices.is_empty()
        || !surface.indices.len().is_multiple_of(3)
        || surface.positions.iter().flatten().any(|p| !p.is_finite())
        || surface
            .indices
            .iter()
            .any(|i| *i as usize >= surface.positions.len())
    {
        return Err(LodCertificateError::InvalidSurface);
    }
    Ok(())
}

pub(crate) fn directional_bound(
    source: LodSurface<'_>,
    target: LodSurface<'_>,
    witnesses: &[LodTriangleWitness],
) -> Result<f64, LodCertificateError> {
    if witnesses.len() != source.indices.len() / 3 {
        return Err(LodCertificateError::InvalidWitness);
    }
    let mut bound = 0.0_f64;
    for (triangle, witness) in source.indices.chunks_exact(3).zip(witnesses) {
        let start = witness
            .target_triangle
            .checked_mul(3)
            .ok_or(LodCertificateError::InvalidWitness)?;
        let end = start
            .checked_add(3)
            .ok_or(LodCertificateError::InvalidWitness)?;
        let mapped = target
            .indices
            .get(start..end)
            .ok_or(LodCertificateError::InvalidWitness)?;
        for (corner, weights) in triangle.iter().zip(witness.weights) {
            if weights.iter().map(|w| u64::from(*w)).sum::<u64>()
                != u64::from(LOD_BARYCENTRIC_DENOMINATOR)
            {
                return Err(LodCertificateError::InvalidWitness);
            }
            let point = source.positions[*corner as usize];
            let mut squared = 0.0_f64;
            for (axis, coordinate) in point.into_iter().enumerate() {
                let mut projected = Interval::exact(0.0);
                for (index, weight) in mapped.iter().zip(weights) {
                    // f32 significand <=24 bits, integer <=25 bits; product fits
                    // in f64. Division by a power of two is also exact here.
                    let term = f64::from(target.positions[*index as usize][axis])
                        * f64::from(weight)
                        / f64::from(LOD_BARYCENTRIC_DENOMINATOR);
                    projected = projected.add(Interval::exact(term));
                }
                let delta = projected.subtract_exact(f64::from(coordinate));
                let magnitude = delta.low.abs().max(delta.high.abs());
                let axis_squared = if magnitude == 0.0 {
                    0.0
                } else {
                    (magnitude * magnitude).next_up()
                };
                squared = add_up(squared, axis_squared);
            }
            let distance = if squared == 0.0 {
                0.0
            } else {
                squared.sqrt().next_up()
            };
            bound = bound.max(distance);
        }
    }
    Ok(bound)
}

/// Proxy points are only used for distance evaluation; coverage uses exact
/// dyadic coordinates. The returned radius bounds f32 proxy quantization.
#[allow(clippy::cast_possible_truncation)] // Quantization is explicitly bounded below.
pub(crate) fn barycentric_proxy(triangle: [[f32; 3]; 3], weights: [u32; 3]) -> ([f32; 3], f64) {
    let mut point = [0.0_f32; 3];
    let mut squared = 0.0;
    for (axis, coordinate) in point.iter_mut().enumerate() {
        let mut interval = Interval::exact(0.0);
        let mut approximate = 0.0;
        for (vertex, weight) in triangle.into_iter().zip(weights) {
            let term = f64::from(vertex[axis]) * f64::from(weight)
                / f64::from(LOD_BARYCENTRIC_DENOMINATOR);
            interval = interval.add(Interval::exact(term));
            approximate += term;
        }
        *coordinate = approximate as f32;
        let delta = interval.subtract_exact(f64::from(*coordinate));
        let magnitude = delta.low.abs().max(delta.high.abs());
        let axis_squared = if magnitude == 0.0 {
            0.0
        } else {
            (magnitude * magnitude).next_up()
        };
        squared = add_up(squared, axis_squared);
    }
    let radius = if squared == 0.0 {
        0.0
    } else {
        squared.sqrt().next_up()
    };
    (point, radius)
}

#[derive(Clone, Copy)]
struct Interval {
    low: f64,
    high: f64,
}
impl Interval {
    fn exact(value: f64) -> Self {
        Self {
            low: value,
            high: value,
        }
    }
    fn add(self, other: Self) -> Self {
        if other.low == 0.0 && other.high == 0.0 {
            return self;
        }
        if self.low == 0.0 && self.high == 0.0 {
            return other;
        }
        Self {
            low: (self.low + other.low).next_down(),
            high: (self.high + other.high).next_up(),
        }
    }
    #[allow(clippy::float_cmp)] // Singleton interval identity; tolerance would underbound.
    fn subtract_exact(self, value: f64) -> Self {
        if self.low == value && self.high == value {
            return Self::exact(0.0);
        }
        Self {
            low: (self.low - value).next_down(),
            high: (self.high - value).next_up(),
        }
    }
}
fn add_up(a: f64, b: f64) -> f64 {
    if a == 0.0 {
        b
    } else if b == 0.0 {
        a
    } else {
        (a + b).next_up()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const D: u32 = LOD_BARYCENTRIC_DENOMINATOR;
    const IDENTITY: LodTriangleWitness = LodTriangleWitness {
        target_triangle: 0,
        weights: [[D, 0, 0], [0, D, 0], [0, 0, D]],
    };
    const TRIANGLE: [[f32; 3]; 3] = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
    const INDICES: [u32; 3] = [0, 1, 2];
    fn surface(positions: &[[f32; 3]]) -> LodSurface<'_> {
        LodSurface {
            positions,
            indices: &INDICES,
        }
    }
    #[test]
    fn identical_and_translated_surfaces() {
        let base = surface(&TRIANGLE);
        assert_eq!(
            certify_lod_error(base, base, &[IDENTITY], &[IDENTITY]),
            Ok(0.0)
        );
        let shifted = TRIANGLE.map(|[x, y, z]| [x, y, z + 2.]);
        let bound = certify_lod_error(base, surface(&shifted), &[IDENTITY], &[IDENTITY]).unwrap();
        assert!((2.0..2.000_000_000_001).contains(&bound));
    }
    #[test]
    fn square_to_triangle_bounds_removed_interior() {
        let positions = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]];
        let square = LodSurface {
            positions: &positions,
            indices: &[0, 1, 3, 0, 3, 2],
        };
        let triangle = LodSurface {
            positions: &positions,
            indices: &[0, 1, 2],
        };
        let midpoint = [0, D / 2, D / 2];
        let forward = [
            LodTriangleWitness {
                target_triangle: 0,
                weights: [[D, 0, 0], [0, D, 0], midpoint],
            },
            LodTriangleWitness {
                target_triangle: 0,
                weights: [[D, 0, 0], midpoint, [0, 0, D]],
            },
        ];
        let reverse = [LodTriangleWitness {
            target_triangle: 0,
            weights: [[D, 0, 0], [0, D, 0], [D / 2, 0, D / 2]],
        }];
        let bound = certify_lod_error(square, triangle, &forward, &reverse).unwrap();
        assert!(bound >= 0.5_f64.sqrt());
        assert!(bound < 0.708);
    }
    #[test]
    fn rejects_untrusted_or_incomplete_witnesses() {
        let base = surface(&TRIANGLE);
        assert_eq!(
            certify_lod_error(base, base, &[], &[IDENTITY]),
            Err(LodCertificateError::InvalidWitness)
        );
        let mut witness = IDENTITY;
        witness.weights[0][0] = u32::MAX;
        assert_eq!(
            certify_lod_error(base, base, &[witness], &[IDENTITY]),
            Err(LodCertificateError::InvalidWitness)
        );
        witness = IDENTITY;
        witness.target_triangle = usize::MAX;
        assert_eq!(
            certify_lod_error(base, base, &[witness], &[IDENTITY]),
            Err(LodCertificateError::InvalidWitness)
        );
        let bad = LodSurface {
            positions: &TRIANGLE,
            indices: &[0, 1, 3],
        };
        assert_eq!(
            certify_lod_error(base, bad, &[IDENTITY], &[IDENTITY]),
            Err(LodCertificateError::InvalidSurface)
        );
    }
    #[test]
    fn reverse_direction_detects_removed_island() {
        let positions = [
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 10.],
            [1., 0., 10.],
            [0., 1., 10.],
        ];
        let islands = LodSurface {
            positions: &positions,
            indices: &[0, 1, 2, 3, 4, 5],
        };
        let retained = surface(&TRIANGLE);
        assert_eq!(
            certify_lod_error(retained, islands, &[IDENTITY], &[IDENTITY]),
            Err(LodCertificateError::InvalidWitness)
        );
        let bound =
            certify_lod_error(retained, islands, &[IDENTITY], &[IDENTITY, IDENTITY]).unwrap();
        assert!((10.0..10.000_000_000_001).contains(&bound));
    }
    #[test]
    fn fractional_witness_and_extreme_finite_coordinates() {
        let base = surface(&TRIANGLE);
        let witness = LodTriangleWitness {
            target_triangle: 0,
            weights: [[D / 2, D / 2, 0]; 3],
        };
        let bound = certify_lod_error(base, base, &[witness], &[witness]).unwrap();
        assert!(bound >= 1.25_f64.sqrt());
        let large = TRIANGLE.map(|_| [f32::MAX; 3]);
        let small = TRIANGLE.map(|_| [-f32::MAX; 3]);
        let bound =
            certify_lod_error(surface(&large), surface(&small), &[IDENTITY], &[IDENTITY]).unwrap();
        assert!(bound.is_finite() && bound >= 2.0 * f64::from(f32::MAX) * 3.0_f64.sqrt());
    }
}
