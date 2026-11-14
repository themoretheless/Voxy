//! Detect resolvable positive-volume overlap with tetrahedral separating axes.
//! Floating-point admission, not an exact-predicate geometric certificate.
use super::*;
use crate::spatial_bounds::BoundsIndex;

impl TetraMesh {
    pub(super) fn reject_overlapping_cells(&self) -> Result<(), &'static str> {
        let bounds: Vec<_> = self
            .cells
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                let low: Vec3 = std::array::from_fn(|axis| {
                    cell.iter()
                        .map(|&i| self.points[i][axis])
                        .fold(f64::INFINITY, f64::min)
                });
                let high: Vec3 = std::array::from_fn(|axis| {
                    cell.iter()
                        .map(|&i| self.points[i][axis])
                        .fold(f64::NEG_INFINITY, f64::max)
                });
                (index, low, high)
            })
            .collect();
        let tree = BoundsIndex::new(bounds);
        let mut candidates = 0usize;
        for i in 0..self.cells.len() {
            let cell = self.cells[i];
            let low: Vec3 = std::array::from_fn(|axis| {
                cell.iter()
                    .map(|&node| self.points[node][axis])
                    .fold(f64::INFINITY, f64::min)
            });
            let high: Vec3 = std::array::from_fn(|axis| {
                cell.iter()
                    .map(|&node| self.points[node][axis])
                    .fold(f64::NEG_INFINITY, f64::max)
            });
            let mut nearby = Vec::new();
            tree.query_overlapping_after(i, low, high, &mut nearby);
            for j in nearby {
                candidates += 1;
                if candidates > 4_000_000 {
                    return Err("tetrahedral overlap candidate limit");
                }
                // The already validated opposite interface orientation proves
                // that cells sharing a complete face occupy opposite halfspaces.
                if self.cells[i]
                    .iter()
                    .filter(|node| self.cells[j].contains(node))
                    .count()
                    == 3
                {
                    continue;
                }
                let a =
                    self.cells[i].map(|node| sub(self.points[node], self.points[self.cells[i][0]]));
                let b =
                    self.cells[j].map(|node| sub(self.points[node], self.points[self.cells[i][0]]));
                if positive_overlap(a, b)? {
                    return Err("overlapping tetrahedral cells");
                }
            }
        }
        Ok(())
    }
}
fn positive_overlap(a: [Vec3; 4], b: [Vec3; 4]) -> Result<bool, &'static str> {
    let edges = |p: [Vec3; 4]| {
        [[0, 1], [0, 2], [0, 3], [1, 2], [1, 3], [2, 3]].map(|[i, j]| sub(p[j], p[i]))
    };
    let ea = edges(a);
    let eb = edges(b);
    let mut axes = Vec::with_capacity(44);
    for p in [a, b] {
        for [i, j, k] in [[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]] {
            axes.push(cross(sub(p[j], p[i]), sub(p[k], p[i])));
        }
    }
    for x in ea {
        for y in eb {
            axes.push(cross(x, y));
        }
    }
    for axis in axes {
        let scale = axis.iter().map(|v| v.abs()).fold(0., f64::max);
        if !scale.is_finite() {
            return Err("nonfinite tetrahedral overlap predicate");
        }
        if scale == 0. {
            continue;
        }
        let axis = axis.map(|v| v / scale);
        let pa = a.map(|v| dot(v, axis));
        let pb = b.map(|v| dot(v, axis));
        if pa.iter().chain(&pb).any(|v| !v.is_finite()) {
            return Err("nonfinite tetrahedral overlap predicate");
        }
        let low = |p: [f64; 4]| p.into_iter().fold(f64::INFINITY, f64::min);
        let high = |p: [f64; 4]| p.into_iter().fold(f64::NEG_INFINITY, f64::max);
        // Relative roundoff allowance only, no fixed world-unit clearance.
        // Overlap below this band is unresolved and not claimed as detected.
        // Bound dot-product rounding using the terms actually projected.
        // A wide x/y extent must not erase resolvable overlap on a thin z axis.
        let projected_terms = a
            .iter()
            .chain(&b)
            .map(|point| (0..3).map(|i| (point[i] * axis[i]).abs()).sum::<f64>())
            .fold(0., f64::max);
        if !projected_terms.is_finite() {
            return Err("nonfinite tetrahedral overlap predicate");
        }
        let roundoff = 64. * f64::EPSILON * projected_terms;
        if high(pa).min(high(pb)) - low(pa).max(low(pb)) <= roundoff {
            return Ok(false);
        }
    }
    Ok(true)
}
