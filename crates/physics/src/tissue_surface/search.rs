//! Prepared immutable tetrahedral point location; published bindings share geometry.
use super::*;
use crate::spatial_bounds::BoundsIndex;
/// Reusable rest-space point location and binding for one immutable volume.
/// Shared-face ties retain source cell order; no nearest-cell fallback is used.
#[derive(Debug)]
pub struct TetrahedralEmbedding {
    rest: Arc<[Point]>,
    cells: Arc<[[usize; 4]]>,
    prepared: Vec<([usize; 4], Point, Point, Point, f64)>,
    bounds: BoundsIndex,
}
impl TetrahedralEmbedding {
    /// # Errors
    /// Invalid/nonfinite geometry, indices or degenerate cells.
    pub fn new(rest: &[Point], cells: &[[usize; 4]]) -> Result<Self, &'static str> {
        if rest.is_empty() || cells.is_empty() || rest.iter().flatten().any(|x| !x.is_finite()) {
            return Err("invalid embedding positions");
        }
        let mut prepared = Vec::with_capacity(cells.len());
        for &ids in cells {
            if ids.iter().any(|&i| i >= rest.len()) {
                return Err("invalid embedding index");
            }
            let a = sub(rest[ids[1]], rest[ids[0]]);
            let b = sub(rest[ids[2]], rest[ids[0]]);
            let c = sub(rest[ids[3]], rest[ids[0]]);
            let det = determinant(a, b, c);
            let scale = a
                .iter()
                .chain(&b)
                .chain(&c)
                .fold(0.0_f64, |s, x| s.max(x.abs()));
            if !det.is_finite() || scale == 0.0 || det.abs() <= 1e-12 * scale.powi(3) {
                return Err("degenerate embedding cell");
            }
            prepared.push((ids, a, b, c, det));
        }
        let bounds = prepared
            .iter()
            .enumerate()
            .map(|(index, (ids, a, b, c, det))| {
                let scale = a
                    .iter()
                    .chain(b)
                    .chain(c)
                    .map(|v| v.abs())
                    .fold(0., f64::max);
                let conditioning = scale.powi(3) / det.abs();
                let low: Point = std::array::from_fn(|axis| {
                    ids.iter()
                        .map(|&node| rest[node][axis])
                        .fold(f64::INFINITY, f64::min)
                });
                let high: Point = std::array::from_fn(|axis| {
                    ids.iter()
                        .map(|&node| rest[node][axis])
                        .fold(f64::NEG_INFINITY, f64::max)
                });
                // Broad-phase inflation only. Barycentric admission below is unchanged.
                let pad: Point = std::array::from_fn(|axis| {
                    4. * (1e-10 + 128. * f64::EPSILON * conditioning) * (high[axis] - low[axis])
                        + 16. * f64::EPSILON * low[axis].abs().max(high[axis].abs())
                });
                (
                    index,
                    std::array::from_fn(|axis| low[axis] - pad[axis]),
                    std::array::from_fn(|axis| high[axis] + pad[axis]),
                )
            })
            .collect();
        Ok(Self {
            rest: rest.into(),
            cells: cells.into(),
            prepared,
            bounds: BoundsIndex::new(bounds),
        })
    }
    /// # Errors
    /// Nonfinite query positions. Uses the same admission as published bindings.
    pub fn contains(&self, point: Point) -> Result<bool, &'static str> {
        if point.iter().any(|x| !x.is_finite()) {
            return Err("invalid embedding positions");
        }
        Ok(self.find(point, &mut Vec::new()).is_some())
    }
    /// Batch point membership, reusing query storage across the entire batch.
    /// # Errors
    /// Any nonfinite query rejects the whole batch.
    pub fn contains_points(&self, points: &[Point]) -> Result<Vec<bool>, &'static str> {
        if points.iter().flatten().any(|x| !x.is_finite()) {
            return Err("invalid embedding positions");
        }
        let mut candidates = Vec::new();
        Ok(points
            .iter()
            .map(|&point| self.find(point, &mut candidates).is_some())
            .collect())
    }
    fn find(&self, point: Point, candidates: &mut Vec<usize>) -> Option<Binding> {
        candidates.clear();
        self.bounds.query_point(point, candidates);
        candidates.sort_unstable();
        let mut found = None;
        for &cell in candidates.iter() {
            let (indices, a, b, c, det) = self.prepared[cell];
            let q = sub(point, self.rest[indices[0]]);
            let mut weights = [
                0.0,
                determinant(q, b, c) / det,
                determinant(a, q, c) / det,
                determinant(a, b, q) / det,
            ];
            weights[0] = 1.0 - weights[1] - weights[2] - weights[3];
            if weights
                .iter()
                .all(|&w| w.is_finite() && (-1e-10..=1.0 + 1e-10).contains(&w))
            {
                // Remove boundary roundoff without permitting visible extrapolation.
                for w in &mut weights {
                    *w = w.clamp(0.0, 1.0);
                }
                let sum: f64 = weights.iter().sum();
                for w in &mut weights {
                    *w /= sum;
                }
                found = Some(Binding { indices, weights });
                break;
            }
        }
        found
    }
    /// # Errors
    /// Invalid ownership/positions or exterior tissue-owned vertices.
    pub fn bind_relative(
        &self,
        surface: &[Point],
        tissue_owned: &[bool],
    ) -> Result<EmbeddedSurface, &'static str> {
        if surface.len() != tissue_owned.len() {
            return Err("invalid relative embedding ownership");
        }
        if surface.iter().flatten().any(|x| !x.is_finite()) {
            return Err("invalid embedding positions");
        }
        let mut bindings = Vec::with_capacity(surface.len());
        let mut candidates = Vec::new();
        for (&point, &owned) in surface.iter().zip(tissue_owned) {
            bindings.push(if owned {
                Some(
                    self.find(point, &mut candidates)
                        .ok_or("surface vertex outside tetrahedral mesh")?,
                )
            } else {
                None
            });
        }
        Ok(EmbeddedSurface {
            bindings,
            vertex_count: self.rest.len(),
            rest: self.rest.clone(),
            cells: self.cells.clone(),
            surface: surface.into(),
        })
    }
}
