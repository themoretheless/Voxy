//! Bounded global closest-point search on a current quadratic triangle.
use super::{QuadraticFace, Vec3, dot, sub};
use std::{cmp::Ordering, collections::BinaryHeap};
#[derive(Clone, Copy, Debug)]
pub struct QuadraticClosestLimits {
    pub distance_tolerance_m: f64,
    pub max_patches: usize,
    pub max_depth: u8,
}
#[derive(Clone, Debug)]
pub struct QuadraticClosestPoint {
    pub barycentric: Vec3,
    pub shape_weights: [f64; 6],
    pub point_m: Vec3,
    pub normal: Vec3,
    pub distance_m: f64,
    /// Bezier convex-hull/AABB lower bound, subject to floating point roundoff.
    /// This is not an interval-certified bound.
    pub lower_distance_m: f64,
    pub converged: bool,
    pub patches: usize,
}
#[derive(Clone)]
struct Patch {
    triangle: [Vec3; 3],
    lower: f64,
    depth: u8,
}
impl PartialEq for Patch {
    fn eq(&self, rhs: &Self) -> bool {
        self.lower.total_cmp(&rhs.lower) == Ordering::Equal
    }
}
impl Eq for Patch {}
impl PartialOrd for Patch {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}
impl Ord for Patch {
    fn cmp(&self, rhs: &Self) -> Ordering {
        rhs.lower.total_cmp(&self.lower)
    }
}
fn norm(v: Vec3) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}
fn midpoint(a: Vec3, b: Vec3) -> Vec3 {
    std::array::from_fn(|i| a[i].midpoint(b[i]))
}
fn value(points: &[Vec3; 6], l: Vec3) -> Vec3 {
    let (n, _, _) = super::cohesive::basis(l);
    std::array::from_fn(|axis| points.iter().zip(n).map(|(p, w)| p[axis] * w).sum())
}
fn lower(points: &[Vec3; 6], tri: [Vec3; 3]) -> Result<f64, &'static str> {
    let vertices = tri.map(|l| value(points, l));
    let mut controls = [[0.; 3]; 6];
    controls[..3].copy_from_slice(&vertices);
    for (k, (a, b)) in [(0, 1), (1, 2), (0, 2)].into_iter().enumerate() {
        let mid = value(points, midpoint(tri[a], tri[b]));
        controls[3 + k] =
            std::array::from_fn(|i| 2. * mid[i] - vertices[a][i].midpoint(vertices[b][i]));
    }
    if controls.iter().flatten().any(|x| !x.is_finite()) {
        return Err("quadratic closest-point bound overflow");
    }
    let aabb = norm(std::array::from_fn(|axis| {
        let min = controls
            .iter()
            .map(|p| p[axis])
            .fold(f64::INFINITY, f64::min);
        let max = controls
            .iter()
            .map(|p| p[axis])
            .fold(f64::NEG_INFINITY, f64::max);
        if min > 0. {
            min
        } else if max < 0. {
            max
        } else {
            0.
        }
    }));
    // Every Bezier surface point lies in the convex hull of these controls.
    // Any unit support direction therefore gives a valid separating-plane
    // distance lower bound. A local feasible minimum supplies a useful direction;
    // its global optimality is not assumed by this bound.
    let center = std::array::from_fn(|i| (tri[0][i] + tri[1][i] + tri[2][i]) / 3.);
    let mut bound = aabb;
    for candidate in [
        value(points, center),
        value(points, improve(points, center)),
    ] {
        let length = norm(candidate);
        if length > 0. && length.is_finite() {
            let direction = candidate.map(|x| x / length);
            let support = controls
                .iter()
                .map(|p| dot(*p, direction))
                .fold(f64::INFINITY, f64::min)
                .max(0.);
            bound = bound.max(support);
        }
    }
    Ok(bound)
}
// Projection onto the barycentric simplex. Used only to obtain a feasible upper
// bound; the subdivision search independently checks the global distance gap.
fn project(l: Vec3) -> Vec3 {
    let mut sorted = l;
    sorted.sort_by(|a, b| b.total_cmp(a));
    let mut sum = 0.;
    let mut theta = 0.;
    for (i, &x) in sorted.iter().enumerate() {
        sum += x;
        let candidate = (sum - 1.) / f64::from(u32::try_from(i + 1).unwrap());
        if x > candidate {
            theta = candidate;
        }
    }
    let mut result = l.map(|x| (x - theta).max(0.));
    let sum: f64 = result.iter().sum();
    result = result.map(|x| x / sum);
    result
}
fn improve(points: &[Vec3; 6], mut barycentric: Vec3) -> Vec3 {
    for _ in 0..24 {
        let (_, du, dv) = super::cohesive::basis(barycentric);
        let interpolate = |n: [f64; 6]| -> Vec3 {
            std::array::from_fn(|axis| points.iter().zip(n).map(|(point, w)| point[axis] * w).sum())
        };
        let tangent_u = interpolate(du);
        let tangent_v = interpolate(dv);
        let point = value(points, barycentric);
        let aa = dot(tangent_u, tangent_u);
        let ab = dot(tangent_u, tangent_v);
        let bb = dot(tangent_v, tangent_v);
        let det = aa * bb - ab * ab;
        if !det.is_finite() || det <= 64. * f64::EPSILON * aa * bb {
            break;
        }
        let ga = dot(tangent_u, point);
        let gb = dot(tangent_v, point);
        let step_u = (bb * ga - ab * gb) / det;
        let step_v = (aa * gb - ab * ga) / det;
        let mut accepted = None;
        for k in 0..16 {
            let fraction = 2_f64.powi(-k);
            let next = project([
                barycentric[0] + fraction * (step_u + step_v),
                barycentric[1] - fraction * step_u,
                barycentric[2] - fraction * step_v,
            ]);
            if norm(value(points, next)) < norm(point) {
                accepted = Some(next);
                break;
            }
        }
        let Some(next) = accepted else {
            break;
        };
        barycentric = next;
    }
    barycentric
}
fn finish(
    points: &[Vec3; 6],
    best_l: Vec3,
    query: Vec3,
    best: f64,
    bound: f64,
    tolerance: f64,
    patches: usize,
) -> Result<QuadraticClosestPoint, &'static str> {
    let (shape_weights, du, dv) = super::cohesive::basis(best_l);
    let interpolate = |n: [f64; 6]| -> Vec3 {
        std::array::from_fn(|axis| points.iter().zip(n).map(|(p, w)| p[axis] * w).sum())
    };
    let a = interpolate(du);
    let b = interpolate(dv);
    let scale = norm(a).max(norm(b));
    let product = crate::plasticity::mesh::cross(a.map(|x| x / scale), b.map(|x| x / scale));
    let magnitude = norm(product);
    if !best.is_finite()
        || !bound.is_finite()
        || !magnitude.is_finite()
        || magnitude <= 64. * f64::EPSILON
    {
        return Err("singular quadratic closest-point geometry");
    }
    let relative = value(points, best_l);
    let point_m = std::array::from_fn(|i| query[i] + relative[i]);
    if point_m.iter().any(|x| !x.is_finite()) {
        return Err("quadratic closest-point overflow");
    }
    Ok(QuadraticClosestPoint {
        barycentric: best_l,
        shape_weights,
        point_m,
        normal: product.map(|x| x / magnitude),
        distance_m: best,
        lower_distance_m: bound,
        converged: best - bound <= tolerance,
        patches,
    })
}
impl QuadraticFace {
    /// Search the entire current T6 parameter triangle, including edges/corners.
    /// Node order is corners then midpoints 01,12,02. No history is mutated.
    /// The returned upper/lower distance gap is authoritative for convergence;
    /// a depleted patch/depth budget returns `converged=false` with its gap.
    /// # Errors
    /// Invalid indices, nonfinite geometry/query, invalid limits, overflow or
    /// a singular tangent frame at the reported closest point.
    pub fn closest_point_at(
        &self,
        positions: &[Vec3],
        query: Vec3,
        limits: QuadraticClosestLimits,
    ) -> Result<QuadraticClosestPoint, &'static str> {
        if query.iter().any(|x| !x.is_finite())
            || !limits.distance_tolerance_m.is_finite()
            || limits.distance_tolerance_m <= 0.
            || limits.max_patches == 0
            || limits.max_patches > 65536
            || limits.max_depth > 24
        {
            return Err("invalid quadratic closest-point query");
        }
        let mut points = [[0.; 3]; 6];
        for (i, &node) in self.nodes.iter().enumerate() {
            let p = *positions
                .get(node)
                .ok_or("invalid quadratic closest-point node")?;
            if p.iter().any(|x| !x.is_finite()) {
                return Err("invalid quadratic closest-point position");
            }
            points[i] = sub(p, query);
        }
        if points.iter().flatten().any(|x| !x.is_finite()) {
            return Err("quadratic closest-point overflow");
        }
        let mut best_l = [1. / 3.; 3];
        let mut best = norm(value(&points, best_l));
        let mut consider = |l: Vec3| {
            let distance = norm(value(&points, l));
            if distance < best {
                best = distance;
                best_l = l;
            }
        };
        let root = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        for l in root {
            consider(l);
        }
        consider(improve(&points, [1. / 3.; 3]));
        let mut heap = BinaryHeap::new();
        heap.push(Patch {
            triangle: root,
            lower: lower(&points, root)?,
            depth: 0,
        });
        let mut terminal_lower = f64::INFINITY;
        let mut patches = 0;
        loop {
            let bound = heap
                .peek()
                .map_or(terminal_lower, |p| p.lower.min(terminal_lower))
                .min(best);
            if best - bound <= limits.distance_tolerance_m || patches >= limits.max_patches {
                break;
            }
            let Some(patch) = heap.pop() else {
                break;
            };
            patches += 1;
            if patch.lower >= best {
                continue;
            }
            let [a, b, c] = patch.triangle;
            let center = std::array::from_fn(|i| (a[i] + b[i] + c[i]) / 3.);
            for l in [a, b, c, center, improve(&points, center)] {
                let distance = norm(value(&points, l));
                if distance < best {
                    best = distance;
                    best_l = l;
                }
            }
            if patch.depth == limits.max_depth {
                terminal_lower = terminal_lower.min(patch.lower);
                continue;
            }
            let ab = midpoint(a, b);
            let bc = midpoint(b, c);
            let ac = midpoint(a, c);
            for triangle in [[a, ab, ac], [ab, b, bc], [ac, bc, c], [ab, bc, ac]] {
                let bound = lower(&points, triangle)?;
                if bound < best {
                    heap.push(Patch {
                        triangle,
                        lower: bound,
                        depth: patch.depth + 1,
                    });
                }
            }
        }
        let bound = heap
            .peek()
            .map_or(terminal_lower, |p| p.lower.min(terminal_lower))
            .min(best);
        finish(
            &points,
            best_l,
            query,
            best,
            bound,
            limits.distance_tolerance_m,
            patches,
        )
    }
}
