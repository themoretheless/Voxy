//! Shared triangle/swept-triangle broad phase; narrow phase remains caller-owned.
type V = [f64; 3];
#[derive(Clone, Copy, Debug)]
pub(crate) struct TriangleBounds {
    lo: V,
    hi: V,
    center: V,
}
impl TriangleBounds {
    fn points<const N: usize>(points: [V; N]) -> Self {
        Self {
            lo: std::array::from_fn(|axis| {
                points.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min)
            }),
            hi: std::array::from_fn(|axis| {
                points
                    .iter()
                    .map(|p| p[axis])
                    .fold(f64::NEG_INFINITY, f64::max)
            }),
            center: std::array::from_fn(|axis| points.iter().map(|p| p[axis] / N as f64).sum()),
        }
    }
    pub(crate) fn triangle(points: [V; 3]) -> Self {
        Self::points(points)
    }
    pub(crate) fn swept(start: [V; 3], end: [V; 3]) -> Self {
        Self::points(std::array::from_fn::<_, 6, _>(|i| {
            if i < 3 { start[i] } else { end[i - 3] }
        }))
    }
}
#[derive(Clone, Debug)]
struct Node {
    lo: V,
    hi: V,
    coordinate_scale: f64,
    children: Option<(Box<Node>, Box<Node>)>,
    ids: Vec<usize>,
}
impl Node {
    fn build(bounds: &[TriangleBounds], mut ids: Vec<usize>) -> Self {
        let lo = std::array::from_fn(|axis| {
            ids.iter()
                .map(|&i| bounds[i].lo[axis])
                .fold(f64::INFINITY, f64::min)
        });
        let hi = std::array::from_fn(|axis| {
            ids.iter()
                .map(|&i| bounds[i].hi[axis])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        let coordinate_scale = lo
            .iter()
            .chain(&hi)
            .map(|v| v.abs())
            .fold(1e-12_f64, f64::max);
        if ids.len() <= 8 {
            return Self {
                lo,
                hi,
                coordinate_scale,
                children: None,
                ids,
            };
        }
        let axis = (0..3)
            .max_by(|&a, &b| (hi[a] - lo[a]).total_cmp(&(hi[b] - lo[b])))
            .unwrap();
        ids.sort_by(|&a, &b| bounds[a].center[axis].total_cmp(&bounds[b].center[axis]));
        let right = ids.split_off(ids.len() / 2);
        Self {
            lo,
            hi,
            coordinate_scale,
            ids: Vec::new(),
            children: Some((
                Box::new(Self::build(bounds, ids)),
                Box::new(Self::build(bounds, right)),
            )),
        }
    }
    fn refit(&mut self, bounds: &[TriangleBounds]) {
        if let Some((a, b)) = &mut self.children {
            a.refit(bounds);
            b.refit(bounds);
            self.lo = std::array::from_fn(|axis| a.lo[axis].min(b.lo[axis]));
            self.hi = std::array::from_fn(|axis| a.hi[axis].max(b.hi[axis]));
        } else {
            self.lo = std::array::from_fn(|axis| {
                self.ids
                    .iter()
                    .map(|&i| bounds[i].lo[axis])
                    .fold(f64::INFINITY, f64::min)
            });
            self.hi = std::array::from_fn(|axis| {
                self.ids
                    .iter()
                    .map(|&i| bounds[i].hi[axis])
                    .fold(f64::NEG_INFINITY, f64::max)
            });
        }
        self.coordinate_scale = self
            .lo
            .iter()
            .chain(&self.hi)
            .map(|v| v.abs())
            .fold(1e-12_f64, f64::max);
    }
    fn query<const PAD: bool>(
        &self,
        bounds: TriangleBounds,
        gap: f64,
        query_scale: f64,
        out: &mut Vec<usize>,
    ) {
        let padding = if PAD {
            64. * f64::EPSILON * self.coordinate_scale.max(query_scale)
        } else {
            0.
        };
        let margin = gap + padding;
        for axis in 0..3 {
            if bounds.lo[axis] > self.hi[axis] + margin || bounds.hi[axis] < self.lo[axis] - margin
            {
                return;
            }
        }
        if let Some((a, b)) = &self.children {
            a.query::<PAD>(bounds, gap, query_scale, out);
            b.query::<PAD>(bounds, gap, query_scale, out);
        } else {
            out.extend(&self.ids);
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct TriangleIndex {
    tree: Node,
}
impl TriangleIndex {
    pub(crate) fn new(triangles: &[[V; 3]]) -> Self {
        Self::from_bounds(
            &triangles
                .iter()
                .copied()
                .map(TriangleBounds::triangle)
                .collect::<Vec<_>>(),
        )
    }
    pub(crate) fn from_bounds(bounds: &[TriangleBounds]) -> Self {
        Self {
            tree: Node::build(bounds, (0..bounds.len()).collect()),
        }
    }
    pub(crate) fn refit(&mut self, triangles: &[[V; 3]]) {
        self.refit_bounds(
            &triangles
                .iter()
                .copied()
                .map(TriangleBounds::triangle)
                .collect::<Vec<_>>(),
        );
    }
    pub(crate) fn refit_bounds(&mut self, bounds: &[TriangleBounds]) {
        self.tree.refit(bounds);
    }
    // Preserve thin-film traversal semantics; its narrow phase owns precision.
    pub(crate) fn query(&self, triangle: [V; 3], gap: f64, out: &mut Vec<usize>) {
        self.tree
            .query::<false>(TriangleBounds::triangle(triangle), gap, 0., out);
    }
    pub(crate) fn query_conservative(
        &self,
        bounds: TriangleBounds,
        gap: f64,
        out: &mut Vec<usize>,
    ) {
        let query_scale = bounds
            .lo
            .iter()
            .chain(&bounds.hi)
            .map(|v| v.abs())
            .fold(1e-12_f64, f64::max);
        self.tree.query::<true>(bounds, gap, query_scale, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn overlaps(a: TriangleBounds, b: TriangleBounds, gap: f64) -> bool {
        (0..3).all(|axis| a.lo[axis] <= b.hi[axis] + gap && a.hi[axis] >= b.lo[axis] - gap)
    }
    #[test]
    fn static_and_swept_refits_never_drop_brute_force_candidates() {
        let mut seed = 7_u64;
        let mut random = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            (seed >> 11) as f64 / ((1_u64 << 53) as f64)
        };
        let start: Vec<[[f64; 3]; 3]> = (0..257)
            .map(|_| {
                let center: V = std::array::from_fn(|_| 20. * random() - 10.);
                std::array::from_fn(|_| std::array::from_fn(|axis| center[axis] + random()))
            })
            .collect();
        let end: Vec<_> = start
            .iter()
            .map(|triangle| triangle.map(|point| point.map(|v| v + 8. * random() - 4.)))
            .collect();
        let mut index = TriangleIndex::new(&start);
        for phase in 0..3 {
            let bounds: Vec<_> = start
                .iter()
                .zip(&end)
                .map(|(&a, &b)| match phase {
                    0 => TriangleBounds::triangle(a),
                    1 => TriangleBounds::triangle(b),
                    _ => TriangleBounds::swept(a, b),
                })
                .collect();
            index.refit_bounds(&bounds);
            for _ in 0..1000 {
                let a = std::array::from_fn(|_| std::array::from_fn(|_| 24. * random() - 12.));
                let b = a.map(|p| p.map(|v| v + random()));
                let query = if phase == 2 {
                    TriangleBounds::swept(a, b)
                } else {
                    TriangleBounds::triangle(a)
                };
                let gap = 0.1 * random();
                let mut candidates = Vec::new();
                index.query_conservative(query, gap, &mut candidates);
                candidates.sort_unstable();
                assert!(candidates.windows(2).all(|pair| pair[0] < pair[1]));
                for (id, &bound) in bounds.iter().enumerate() {
                    if overlaps(query, bound, gap) {
                        assert!(
                            candidates.binary_search(&id).is_ok(),
                            "phase {phase}, triangle {id}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn localized_query_prunes_large_mesh_and_sweep_keeps_crossing() {
        let triangles: Vec<_> = (0..4096)
            .map(|i| {
                let x = i as f64 * 2.;
                [[x, 0., 0.], [x + 0.5, 0., 0.], [x, 0.5, 0.]]
            })
            .collect();
        let index = TriangleIndex::new(&triangles);
        let mut candidates = Vec::new();
        index.query_conservative(
            TriangleBounds::triangle(triangles[2048]),
            0.01,
            &mut candidates,
        );
        assert!(candidates.contains(&2048));
        assert!(candidates.len() <= 16);
        let moving = TriangleBounds::swept(triangles[0], triangles[4095]);
        candidates.clear();
        index.query_conservative(moving, 0., &mut candidates);
        assert_eq!(candidates.len(), 4096);
    }
}

#[cfg(test)]
mod cached_scale_tests {
    use super::*;
    fn original_query(node: &Node, bounds: TriangleBounds, gap: f64, out: &mut Vec<usize>) {
        let padding = 64.
            * f64::EPSILON
            * node
                .lo
                .iter()
                .chain(&node.hi)
                .chain(&bounds.lo)
                .chain(&bounds.hi)
                .map(|v| v.abs())
                .fold(1e-12_f64, f64::max);
        let margin = gap + padding;
        if (0..3).any(|axis| {
            bounds.lo[axis] > node.hi[axis] + margin || bounds.hi[axis] < node.lo[axis] - margin
        }) {
            return;
        }
        if let Some((a, b)) = &node.children {
            original_query(a, bounds, gap, out);
            original_query(b, bounds, gap, out);
        } else {
            out.extend(&node.ids);
        }
    }
    fn fixture(offset: f64) -> Vec<[V; 3]> {
        (0..4096)
            .map(|i| {
                let x = (i % 64) as f64 * 0.1 + offset;
                let y = (i / 64) as f64 * 0.1 - offset;
                [[x, y, 0.], [x + 0.05, y, 0.], [x, y + 0.05, 0.]]
            })
            .collect()
    }
    #[test]
    fn cached_padding_preserves_original_candidate_order_after_refit() {
        let mut index = TriangleIndex::new(&fixture(0.));
        for offset in [0., 1e12, -1e12] {
            index.refit(&fixture(offset));
            for i in 0..1000 {
                let x = (i % 64) as f64 * 0.1 + offset;
                let y = (i / 64) as f64 * 0.1 - offset;
                let bounds = TriangleBounds::triangle([[x, y, 0.001]; 3]);
                for gap in [0., 0.001, 0.1] {
                    let mut expected = Vec::new();
                    original_query(&index.tree, bounds, gap, &mut expected);
                    let mut actual = Vec::new();
                    index.query_conservative(bounds, gap, &mut actual);
                    assert_eq!(actual, expected, "offset={offset}, query={i}, gap={gap}");
                }
            }
        }
    }
    #[test]
    #[ignore = "manual microbenchmark; candidate equivalence has a separate test"]
    fn benchmark_cached_conservative_padding() {
        let index = TriangleIndex::new(&fixture(0.));
        let queries: Vec<_> = (0..1024)
            .map(|i| {
                let x = (i % 64) as f64 * 0.1;
                let y = (i / 64) as f64 * 0.1;
                TriangleBounds::triangle([[x, y, 0.001]; 3])
            })
            .collect();
        for cached in [false, true, false, true] {
            let start = std::time::Instant::now();
            let mut out = Vec::new();
            let mut count = 0;
            for _ in 0..100 {
                for &bounds in &queries {
                    out.clear();
                    if cached {
                        index.query_conservative(bounds, 0.002, &mut out);
                    } else {
                        original_query(&index.tree, bounds, 0.002, &mut out);
                    }
                    count += std::hint::black_box(out.len());
                }
            }
            eprintln!(
                "cached={cached} queries=102400 candidates={count} elapsed_ns={}",
                start.elapsed().as_nanos()
            );
        }
    }
}
