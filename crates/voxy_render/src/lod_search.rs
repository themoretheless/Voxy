use crate::{
    LodSubdivisionWitness, LodSubdivisionWitnesses, LodSurface, LodTriangleWitness, LodWitnessError,
};

/// Independent work/storage limits for the indexed offline producer.
#[derive(Clone, Copy, Debug)]
pub struct LodSearchBudget {
    pub indexed_triangles: u64,
    pub output_cells: u64,
    pub triangle_tests: u64,
    pub node_visits: u64,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct LodSearchWork {
    pub indexed_triangles: u64,
    pub output_cells: u64,
    pub triangle_tests: u64,
    pub node_visits: u64,
}

/// Generates complete subdivided witnesses using temporary triangle BVHs.
/// Conservative AABB lower bounds prune against verified candidate upper bounds.
/// Final surface verification remains independent of spatial search. Storage
/// cardinalities are admitted before index construction; query work is charged
/// across both directions. Failure publishes no partial witness result.
/// # Errors
/// Rejects invalid geometry/depth or exhausted storage/query budgets.
pub fn generate_indexed_lod_witnesses(
    source: LodSurface<'_>,
    approximation: LodSurface<'_>,
    depth: u8,
    budget: LodSearchBudget,
) -> Result<(LodSubdivisionWitnesses, LodSearchWork), LodWitnessError> {
    crate::lod_certificate::validate_surface(source).map_err(LodWitnessError::Certificate)?;
    crate::lod_certificate::validate_surface(approximation)
        .map_err(LodWitnessError::Certificate)?;
    let count = crate::lod_subdivision::cell_count(depth).map_err(LodWitnessError::Certificate)?;
    let indexed = u64::try_from(source.indices.len() / 3)
        .ok()
        .and_then(|a| {
            u64::try_from(approximation.indices.len() / 3)
                .ok()
                .and_then(|b| a.checked_add(b))
        })
        .ok_or(LodWitnessError::WorkBudgetExceeded)?;
    let output = u64::try_from(count)
        .ok()
        .and_then(|count| indexed.checked_mul(count))
        .ok_or(LodWitnessError::WorkBudgetExceeded)?;
    if indexed > budget.indexed_triangles || output > budget.output_cells {
        return Err(LodWitnessError::WorkBudgetExceeded);
    }
    let source_index = Index::new(source);
    let approximation_index = Index::new(approximation);
    let mut work = LodSearchWork {
        indexed_triangles: indexed,
        output_cells: output,
        ..LodSearchWork::default()
    };
    let forward = produce(
        source,
        &approximation_index,
        depth,
        count,
        budget,
        &mut work,
    )?;
    let reverse = produce(
        approximation,
        &source_index,
        depth,
        count,
        budget,
        &mut work,
    )?;
    let object_error =
        crate::certify_subdivided_lod_error(source, approximation, &forward, &reverse)
            .map_err(LodWitnessError::Certificate)?;
    Ok((
        LodSubdivisionWitnesses {
            source_to_approximation: forward,
            approximation_to_source: reverse,
            object_error,
        },
        work,
    ))
}

fn produce(
    source: LodSurface<'_>,
    target: &Index<'_>,
    depth: u8,
    count: usize,
    budget: LodSearchBudget,
    work: &mut LodSearchWork,
) -> Result<Vec<LodSubdivisionWitness>, LodWitnessError> {
    source
        .indices
        .chunks_exact(3)
        .map(|indices| {
            let triangle = std::array::from_fn(|i| source.positions[indices[i] as usize]);
            let cells = (0..count)
                .map(|index| {
                    let points =
                        crate::lod_subdivision::cell_weights(depth, index).map(|weights| {
                            crate::lod_certificate::barycentric_proxy(triangle, weights).0
                        });
                    target.search(points, budget, work)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(LodSubdivisionWitness { depth, cells })
        })
        .collect()
}

#[derive(Clone, Copy)]
struct Bounds {
    low: [f64; 3],
    high: [f64; 3],
}
struct Node {
    bounds: Bounds,
    range: std::ops::Range<usize>,
    children: Option<[usize; 2]>,
}
struct Index<'a> {
    surface: LodSurface<'a>,
    order: Vec<usize>,
    nodes: Vec<Node>,
}
impl<'a> Index<'a> {
    fn new(surface: LodSurface<'a>) -> Self {
        let mut index = Self {
            surface,
            order: (0..surface.indices.len() / 3).collect(),
            nodes: Vec::new(),
        };
        index.build(0, index.order.len());
        index
    }
    fn triangle(&self, index: usize) -> [[f32; 3]; 3] {
        std::array::from_fn(|i| {
            self.surface.positions[self.surface.indices[index * 3 + i] as usize]
        })
    }
    fn build(&mut self, start: usize, end: usize) -> usize {
        let mut bounds = Bounds {
            low: [f64::INFINITY; 3],
            high: [f64::NEG_INFINITY; 3],
        };
        for index in &self.order[start..end] {
            for point in self.triangle(*index) {
                for (axis, value) in point.into_iter().enumerate() {
                    bounds.low[axis] = bounds.low[axis].min(f64::from(value));
                    bounds.high[axis] = bounds.high[axis].max(f64::from(value));
                }
            }
        }
        let node = self.nodes.len();
        self.nodes.push(Node {
            bounds,
            range: start..end,
            children: None,
        });
        if end - start > 4 {
            let axis = (0..3)
                .max_by(|a, b| {
                    (bounds.high[*a] - bounds.low[*a])
                        .total_cmp(&(bounds.high[*b] - bounds.low[*b]))
                })
                .unwrap_or(0);
            let surface = self.surface;
            let center = |index: usize| {
                (0..3)
                    .map(|corner| {
                        f64::from(
                            surface.positions[surface.indices[index * 3 + corner] as usize][axis],
                        )
                    })
                    .sum::<f64>()
            };
            let middle = start + (end - start) / 2;
            self.order[start..end].select_nth_unstable_by(middle - start, |a, b| {
                center(*a).total_cmp(&center(*b)).then(a.cmp(b))
            });
            let left = self.build(start, middle);
            let right = self.build(middle, end);
            self.nodes[node].children = Some([left, right]);
        }
        node
    }
    fn search(
        &self,
        points: [[f32; 3]; 3],
        budget: LodSearchBudget,
        work: &mut LodSearchWork,
    ) -> Result<LodTriangleWitness, LodWitnessError> {
        let mut stack = vec![0];
        let mut best = None;
        let mut upper = f64::INFINITY;
        while let Some(index) = stack.pop() {
            charge(&mut work.node_visits, budget.node_visits)?;
            let node = &self.nodes[index];
            if lower_squared(node.bounds, points) > (upper * upper).next_up() {
                continue;
            }
            if let Some([a, b]) = node.children {
                let da = lower_squared(self.nodes[a].bounds, points);
                let db = lower_squared(self.nodes[b].bounds, points);
                if da <= db {
                    stack.extend([b, a]);
                } else {
                    stack.extend([a, b]);
                }
            } else {
                for triangle in &self.order[node.range.clone()] {
                    charge(&mut work.triangle_tests, budget.triangle_tests)?;
                    let target = self.triangle(*triangle).map(|point| point.map(f64::from));
                    let witness = LodTriangleWitness {
                        target_triangle: *triangle,
                        weights: points.map(|point| {
                            crate::lod_witness::closest_weights(point.map(f64::from), target)
                        }),
                    };
                    let bound = crate::lod_certificate::directional_bound(
                        LodSurface {
                            positions: &points,
                            indices: &[0, 1, 2],
                        },
                        self.surface,
                        std::slice::from_ref(&witness),
                    )
                    .map_err(LodWitnessError::Certificate)?;
                    if bound < upper
                        || (bound.to_bits() == upper.to_bits()
                            && best.as_ref().is_some_and(|old: &LodTriangleWitness| {
                                *triangle < old.target_triangle
                            }))
                    {
                        upper = bound;
                        best = Some(witness);
                    }
                }
            }
        }
        best.ok_or(LodWitnessError::WorkBudgetExceeded)
    }
}

fn charge(counter: &mut u64, limit: u64) -> Result<(), LodWitnessError> {
    if *counter >= limit {
        return Err(LodWitnessError::WorkBudgetExceeded);
    }
    *counter += 1;
    Ok(())
}
fn lower_squared(bounds: Bounds, points: [[f32; 3]; 3]) -> f64 {
    box_lower_squared(bounds.low, bounds.high, points)
}
pub(crate) fn box_lower_squared(low: [f64; 3], high: [f64; 3], points: [[f32; 3]; 3]) -> f64 {
    points
        .into_iter()
        .map(|point| {
            point
                .into_iter()
                .enumerate()
                .fold(0.0_f64, |sum, (axis, value)| {
                    let value = f64::from(value);
                    let delta = if value < low[axis] {
                        (low[axis] - value).next_down().max(0.0)
                    } else if value > high[axis] {
                        (value - high[axis]).next_down().max(0.0)
                    } else {
                        0.0
                    };
                    let squared = (delta * delta).next_down().max(0.0);
                    (sum + squared).next_down().max(0.0)
                })
        })
        .fold(0.0_f64, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indexed_search_prunes_without_changing_flat_geometry_bound() {
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
        let budget = LodSearchBudget {
            indexed_triangles: 40,
            output_cells: 160,
            triangle_tests: 2048,
            node_visits: 10000,
        };
        let (generated, work) =
            generate_indexed_lod_witnesses(source, approximation, 1, budget).unwrap();
        assert!(generated.object_error < 1e-12);
        assert!(work.triangle_tests < 2048);
        assert_eq!(work.output_cells, 160);
        for blocked in [
            LodSearchBudget {
                indexed_triangles: 39,
                ..budget
            },
            LodSearchBudget {
                output_cells: 159,
                ..budget
            },
            LodSearchBudget {
                triangle_tests: 0,
                ..budget
            },
            LodSearchBudget {
                node_visits: 0,
                ..budget
            },
            LodSearchBudget {
                triangle_tests: work.triangle_tests - 1,
                ..budget
            },
            LodSearchBudget {
                node_visits: work.node_visits - 1,
                ..budget
            },
        ] {
            assert!(matches!(
                generate_indexed_lod_witnesses(source, approximation, 1, blocked),
                Err(LodWitnessError::WorkBudgetExceeded)
            ));
        }
        let exact = LodSearchBudget {
            triangle_tests: work.triangle_tests,
            node_visits: work.node_visits,
            ..budget
        };
        assert!(generate_indexed_lod_witnesses(source, approximation, 1, exact).is_ok());
    }
    #[test]
    fn indexed_search_keeps_removed_island_error() {
        let mut positions = Vec::new();
        for island in 0_u8..12 {
            let x = f32::from(island) * 10.;
            positions.extend([[x, 0., 0.], [x + 1., 0., 0.], [x, 1., 0.]]);
        }
        let source_indices: Vec<_> = (0_u32..36).collect();
        let approximation_indices: Vec<_> = source_indices
            .iter()
            .copied()
            .filter(|i| !(15..18).contains(i))
            .collect();
        let source = LodSurface {
            positions: &positions,
            indices: &source_indices,
        };
        let approximation = LodSurface {
            positions: &positions,
            indices: &approximation_indices,
        };
        let budget = LodSearchBudget {
            indexed_triangles: 23,
            output_cells: 23,
            triangle_tests: 264,
            node_visits: 1000,
        };
        let (generated, work) =
            generate_indexed_lod_witnesses(source, approximation, 0, budget).unwrap();
        assert!(generated.object_error >= 9.0 && generated.object_error < 10.001);
        assert!(work.triangle_tests < 264);
    }
}
