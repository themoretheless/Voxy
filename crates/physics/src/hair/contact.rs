use super::{HairRod, math::*};
use std::collections::HashMap;
/// An exact surface query bounds clearance inside the ball around its query point.
/// The distance function is 1-Lipschitz; the certificate expires at the next step.
#[derive(Clone, Copy, Debug)]
pub(super) struct Clearance {
    position: V,
    distance: f64,
}
impl Clearance {
    fn excludes(&self, points: &[V], radius: f64) -> bool {
        self.distance
            > radius
                + points
                    .iter()
                    .map(|p| len(sub(*p, self.position)))
                    .fold(0., f64::max)
    }
}
#[derive(Clone, Debug)]
struct Triangle {
    ids: [usize; 3],
    p: [V; 3],
    normal: V,
    min: V,
    max: V,
}
#[derive(Clone, Debug)]
struct Node {
    min: V,
    max: V,
    children: Option<[usize; 2]>,
    range: std::ops::Range<usize>,
}
/// Closed, consistently outward-wound mesh with a segment-query BVH.
#[derive(Clone, Debug)]
pub struct TriangleMesh {
    triangles: Vec<Triangle>,
    nodes: Vec<Node>,
}
impl TriangleMesh {
    pub fn new(vertices: &[V], indices: &[[usize; 3]]) -> Result<Self, &'static str> {
        if vertices.iter().any(|p| !finite(*p))
            || indices.iter().flatten().any(|i| *i >= vertices.len())
        {
            return Err("invalid hair collision mesh");
        }
        let mut triangles = Vec::new();
        for face in indices {
            let p = face.map(|i| vertices[i]);
            let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
            if len(n) < 1e-14 {
                continue;
            }
            triangles.push(Triangle {
                ids: *face,
                p,
                normal: unit(n),
                min: std::array::from_fn(|a| p.iter().map(|p| p[a]).fold(f64::INFINITY, f64::min)),
                max: std::array::from_fn(|a| {
                    p.iter().map(|p| p[a]).fold(f64::NEG_INFINITY, f64::max)
                }),
            });
        }
        if triangles.is_empty() {
            return Err("empty hair collision mesh");
        }
        let mut mesh = Self {
            triangles,
            nodes: Vec::new(),
        };
        mesh.build(0..mesh.triangles.len());
        Ok(mesh)
    }
    /// Returns the closest surface position and its outward face normal.
    /// # Errors
    /// Rejects nonfinite query positions.
    pub fn closest_surface(&self, point: V) -> Result<(V, V), &'static str> {
        if !finite(point) {
            return Err("invalid surface query");
        }
        let mut best = (f64::INFINITY, 0, [0.; 3]);
        self.nearest(point, 0, &mut best);
        Ok((best.2, self.triangles[best.1].normal))
    }
    /// Classifies a point using oriented ray crossings through the existing BVH.
    /// The caller must supply a closed, consistently oriented surface (including
    /// reversed cavity shells). Construction alone does not validate topology.
    /// Returns `None` at numerically ambiguous edge, vertex or parallel hits;
    /// callers must use their robust winding classifier in that case.
    /// # Errors
    /// Rejects nonfinite query positions.
    pub fn contains_closed_surface(&self, point: V) -> Result<Option<bool>, &'static str> {
        if !finite(point) {
            return Err("invalid surface query");
        }
        let direction = [1.0, 0.3713906763541037, 0.5291121672691435];
        Ok(self.oriented_crossings(point, direction, 0).map(|n| n != 0))
    }

    fn oriented_crossings(&self, point: V, direction: V, node: usize) -> Option<i64> {
        let n = &self.nodes[node];
        let mut entry = 0.0_f64;
        let mut exit = f64::INFINITY;
        for axis in 0..3 {
            let padding = 1e-12 * (n.max[axis] - n.min[axis]).abs().max(1.0);
            entry = entry.max((n.min[axis] - padding - point[axis]) / direction[axis]);
            exit = exit.min((n.max[axis] + padding - point[axis]) / direction[axis]);
        }
        if entry > exit {
            return Some(0);
        }
        if let Some([a, b]) = n.children {
            return Some(
                self.oriented_crossings(point, direction, a)?
                    + self.oriented_crossings(point, direction, b)?,
            );
        }
        let mut crossings = 0;
        for triangle in &self.triangles[n.range.clone()] {
            let edge_a = sub(triangle.p[1], triangle.p[0]);
            let edge_b = sub(triangle.p[2], triangle.p[0]);
            let h = cross(direction, edge_b);
            let determinant = dot(edge_a, h);
            let scale = len(edge_a) * len(edge_b) * len(direction);
            if !determinant.is_finite() || determinant.abs() <= 1e-12 * scale {
                return None;
            }
            let relative = sub(point, triangle.p[0]);
            let u = dot(relative, h) / determinant;
            let q = cross(relative, edge_a);
            let v = dot(direction, q) / determinant;
            let t = dot(edge_b, q) / determinant;
            if !u.is_finite() || !v.is_finite() || !t.is_finite() {
                return None;
            }
            let tolerance = 1e-10;
            if u < -tolerance || v < -tolerance || u + v > 1.0 + tolerance || t < -tolerance {
                continue;
            }
            if u <= tolerance || v <= tolerance || 1.0 - u - v <= tolerance || t <= tolerance {
                return None;
            }
            crossings += if dot(triangle.normal, direction) > 0.0 {
                1
            } else {
                -1
            };
        }
        Some(crossings)
    }

    /// Closest position with a barycentrically interpolated supplied vertex normal.
    /// This is a shading-normal query, not a signed-distance certificate.
    pub fn closest_surface_interpolated(
        &self,
        point: V,
        normals: &[V],
    ) -> Result<(V, V), &'static str> {
        if !finite(point) {
            return Err("invalid surface query");
        }
        let mut best = (f64::INFINITY, 0, [0.; 3]);
        self.nearest(point, 0, &mut best);
        let triangle = &self.triangles[best.1];
        if triangle
            .ids
            .iter()
            .any(|&i| i >= normals.len() || !finite(normals[i]))
        {
            return Err("invalid surface normals");
        }
        let a = sub(triangle.p[1], triangle.p[0]);
        let b = sub(triangle.p[2], triangle.p[0]);
        let q = sub(best.2, triangle.p[0]);
        let aa = dot(a, a);
        let ab = dot(a, b);
        let bb = dot(b, b);
        let denominator = aa * bb - ab * ab;
        if denominator <= 0. {
            return Ok((best.2, triangle.normal));
        }
        let v = (bb * dot(q, a) - ab * dot(q, b)) / denominator;
        let w = (aa * dot(q, b) - ab * dot(q, a)) / denominator;
        let weights = [1. - v - w, v, w];
        let normal: V = std::array::from_fn(|k| {
            (0..3)
                .map(|i| normals[triangle.ids[i]][k] * weights[i])
                .sum()
        });
        Ok((
            best.2,
            if len(normal) > 1e-12 {
                unit(normal)
            } else {
                triangle.normal
            },
        ))
    }
    fn build(&mut self, range: std::ops::Range<usize>) -> usize {
        let min = std::array::from_fn(|a| {
            self.triangles[range.clone()]
                .iter()
                .map(|t| t.min[a])
                .fold(f64::INFINITY, f64::min)
        });
        let max = std::array::from_fn(|a| {
            self.triangles[range.clone()]
                .iter()
                .map(|t| t.max[a])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        let id = self.nodes.len();
        self.nodes.push(Node {
            min,
            max,
            children: None,
            range: range.clone(),
        });
        if range.len() > 8 {
            let axis = (0..3)
                .max_by(|a, b| (max[*a] - min[*a]).total_cmp(&(max[*b] - min[*b])))
                .unwrap();
            self.triangles[range.clone()].sort_unstable_by(|a, b| {
                (a.min[axis] + a.max[axis]).total_cmp(&(b.min[axis] + b.max[axis]))
            });
            let mid = range.start + range.len() / 2;
            let a = self.build(range.start..mid);
            let b = self.build(mid..range.end);
            self.nodes[id].children = Some([a, b]);
        }
        id
    }
    fn nearest(&self, p: V, node: usize, best: &mut (f64, usize, V)) {
        let n = &self.nodes[node];
        let distance: f64 = (0..3)
            .map(|a| (n.min[a] - p[a]).max(0.0).max(p[a] - n.max[a]).powi(2))
            .sum();
        if distance > best.0 {
            return;
        }
        if let Some([a, b]) = n.children {
            let box_distance = |id: usize| -> f64 {
                let n = &self.nodes[id];
                (0..3)
                    .map(|a| (n.min[a] - p[a]).max(0.0).max(p[a] - n.max[a]).powi(2))
                    .sum()
            };
            let order = if box_distance(a) < box_distance(b) {
                [a, b]
            } else {
                [b, a]
            };
            for child in order {
                self.nearest(p, child, best);
            }
        } else {
            for i in n.range.clone() {
                let q = closest_triangle(p, &self.triangles[i]);
                let delta = sub(p, q);
                let distance = dot(delta, delta);
                if distance < best.0 {
                    *best = (distance, i, q);
                }
            }
        }
    }
    fn query(&self, min: V, max: V, node: usize, output: &mut Vec<usize>) {
        let n = &self.nodes[node];
        if (0..3).any(|a| min[a] > n.max[a] || max[a] < n.min[a]) {
            return;
        }
        if let Some(children) = n.children {
            for c in children {
                self.query(min, max, c, output);
            }
        } else {
            output.extend(n.range.clone().filter(|&i| {
                let triangle = &self.triangles[i];
                (0..3).all(|a| min[a] <= triangle.max[a] && max[a] >= triangle.min[a])
            }));
        }
    }
    /// First finite-triangle intersection along a nonzero segment.
    /// Returns segment fraction, position and outward face normal. This query
    /// does not certify that the mesh encloses a solid.
    pub fn first_segment_hit(&self, a: V, b: V) -> Result<Option<(f64, V, V)>, &'static str> {
        if !finite(a) || !finite(b) || len(sub(b, a)) <= 1e-12 {
            return Err("invalid surface segment");
        }
        let min = std::array::from_fn(|k| a[k].min(b[k]));
        let max = std::array::from_fn(|k| a[k].max(b[k]));
        let mut candidates = Vec::new();
        self.query(min, max, 0, &mut candidates);
        let direction = sub(b, a);
        let mut best: Option<(f64, V, V)> = None;
        for id in candidates {
            let tri = &self.triangles[id];
            let denominator = dot(direction, tri.normal);
            if denominator.abs() < 1e-20 {
                continue;
            }
            let fraction = dot(sub(tri.p[0], a), tri.normal) / denominator;
            if !(0.0..=1.0).contains(&fraction) || best.as_ref().is_some_and(|v| fraction >= v.0) {
                continue;
            }
            let point = add(a, mul(direction, fraction));
            if len(sub(point, closest_triangle(point, tri))) <= 1e-9 {
                best = Some((fraction, point, tri.normal));
            }
        }
        Ok(best)
    }
    /// Nearest capsule contact against finite triangles; does not classify inside/outside.
    pub fn capsule_contact(
        &self,
        a: V,
        b: V,
        radius: f64,
        forward: V,
    ) -> Result<Option<(f64, V)>, &'static str> {
        if !finite(a) || !finite(b) || !finite(forward) || !radius.is_finite() || radius <= 0. {
            return Err("invalid capsule query");
        }
        let min = std::array::from_fn(|k| a[k].min(b[k]) - radius);
        let max = std::array::from_fn(|k| a[k].max(b[k]) + radius);
        let mut candidates = Vec::new();
        self.query(min, max, 0, &mut candidates);
        let mut best: Option<(f64, f64, V)> = None;
        for id in candidates {
            let tri = &self.triangles[id];
            let (t, p, q) = segment_triangle(a, b, tri);
            let delta = sub(p, q);
            let distance = len(delta);
            if distance >= radius {
                continue;
            }
            let normal = if distance > 1e-10 {
                mul(delta, 1. / distance)
            } else {
                if dot(tri.normal, forward) < 0. {
                    mul(tri.normal, -1.)
                } else {
                    tri.normal
                }
            };
            if best.as_ref().is_none_or(|c| distance < c.0) {
                best = Some((distance, t, mul(normal, radius - distance)));
            }
        }
        Ok(best.map(|(_, t, correction)| (t, correction)))
    }
    /// Update the animated surface and refit the existing BVH without rebuilding it.
    pub fn refit(&mut self, vertices: &[V]) -> Result<(), &'static str> {
        if vertices.iter().any(|p| !finite(*p))
            || self
                .triangles
                .iter()
                .any(|t| t.ids.iter().any(|i| *i >= vertices.len()))
        {
            return Err("invalid animated hair collider");
        }
        for t in &mut self.triangles {
            t.p = t.ids.map(|i| vertices[i]);
            t.normal = unit(cross(sub(t.p[1], t.p[0]), sub(t.p[2], t.p[0])));
            t.min = std::array::from_fn(|a| t.p.iter().map(|p| p[a]).fold(f64::INFINITY, f64::min));
            t.max =
                std::array::from_fn(|a| t.p.iter().map(|p| p[a]).fold(f64::NEG_INFINITY, f64::max));
        }
        for i in (0..self.nodes.len()).rev() {
            if let Some([a, b]) = self.nodes[i].children {
                self.nodes[i].min =
                    std::array::from_fn(|j| self.nodes[a].min[j].min(self.nodes[b].min[j]));
                self.nodes[i].max =
                    std::array::from_fn(|j| self.nodes[a].max[j].max(self.nodes[b].max[j]));
            } else {
                let range = self.nodes[i].range.clone();
                self.nodes[i].min = std::array::from_fn(|a| {
                    self.triangles[range.clone()]
                        .iter()
                        .map(|t| t.min[a])
                        .fold(f64::INFINITY, f64::min)
                });
                self.nodes[i].max = std::array::from_fn(|a| {
                    self.triangles[range.clone()]
                        .iter()
                        .map(|t| t.max[a])
                        .fold(f64::NEG_INFINITY, f64::max)
                });
            }
        }
        Ok(())
    }
    /// Transform a cached mesh rigidly without rebuilding topology. Matrix is row-major 3x4.
    pub fn transformed(&self, m: [[f64; 4]; 3]) -> Result<Self, &'static str> {
        if m.iter().flatten().any(|x| !x.is_finite()) {
            return Err("invalid collision transform");
        }
        let transform =
            |p: V| std::array::from_fn(|a| dot([m[a][0], m[a][1], m[a][2]], p) + m[a][3]);
        let mut result = self.clone();
        for t in &mut result.triangles {
            t.p = t.p.map(transform);
            t.normal = unit(cross(sub(t.p[1], t.p[0]), sub(t.p[2], t.p[0])));
            t.min = std::array::from_fn(|a| t.p.iter().map(|p| p[a]).fold(f64::INFINITY, f64::min));
            t.max =
                std::array::from_fn(|a| t.p.iter().map(|p| p[a]).fold(f64::NEG_INFINITY, f64::max));
        }
        for i in (0..result.nodes.len()).rev() {
            let range = result.nodes[i].range.clone();
            if let Some([a, b]) = result.nodes[i].children {
                result.nodes[i].min =
                    std::array::from_fn(|j| result.nodes[a].min[j].min(result.nodes[b].min[j]));
                result.nodes[i].max =
                    std::array::from_fn(|j| result.nodes[a].max[j].max(result.nodes[b].max[j]));
            } else {
                result.nodes[i].min = std::array::from_fn(|a| {
                    result.triangles[range.clone()]
                        .iter()
                        .map(|t| t.min[a])
                        .fold(f64::INFINITY, f64::min)
                });
                result.nodes[i].max = std::array::from_fn(|a| {
                    result.triangles[range.clone()]
                        .iter()
                        .map(|t| t.max[a])
                        .fold(f64::NEG_INFINITY, f64::max)
                });
            }
        }
        Ok(result)
    }
}
fn closest_triangle(p: V, t: &Triangle) -> V {
    let [a, b, c] = t.p;
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0. && d2 <= 0. {
        return a;
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0. && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        return add(a, mul(ab, d1 / (d1 - d3)));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0. && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return add(a, mul(ac, d2 / (d2 - d6)));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        return add(b, mul(sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6))));
    }
    let denom = 1. / (va + vb + vc);
    add(a, add(mul(ab, vb * denom), mul(ac, vc * denom)))
}
fn segment_pair(a: V, b: V, c: V, d: V) -> (f64, f64, V, V) {
    let u = sub(b, a);
    let v = sub(d, c);
    let w = sub(a, c);
    let aa = dot(u, u);
    let bb = dot(u, v);
    let cc = dot(v, v);
    let dd = dot(u, w);
    let ee = dot(v, w);
    let den = aa * cc - bb * bb;
    let mut s = if den > 1e-25 {
        ((bb * ee - cc * dd) / den).clamp(0., 1.)
    } else {
        0.
    };
    let mut t = (bb * s + ee) / cc.max(1e-30);
    if t < 0. {
        t = 0.;
        s = (-dd / aa.max(1e-30)).clamp(0., 1.);
    } else if t > 1. {
        t = 1.;
        s = ((bb - dd) / aa.max(1e-30)).clamp(0., 1.);
    }
    (s, t, add(a, mul(u, s)), add(c, mul(v, t)))
}
fn segment_triangle(a: V, b: V, tri: &Triangle) -> (f64, V, V) {
    let pa = closest_triangle(a, tri);
    let pb = closest_triangle(b, tri);
    let mut best = if len(sub(a, pa)) < len(sub(b, pb)) {
        (0., a, pa)
    } else {
        (1., b, pb)
    };
    let denominator = dot(sub(b, a), tri.normal);
    if denominator.abs() > 1e-20 {
        let s = dot(sub(tri.p[0], a), tri.normal) / denominator;
        if (0.0..=1.0).contains(&s) {
            let p = add(a, mul(sub(b, a), s));
            let q = closest_triangle(p, tri);
            if len(sub(p, q)) < 1e-9 {
                return (s, p, q);
            }
        }
    }
    for i in 0..3 {
        let (s, _, p, q) = segment_pair(a, b, tri.p[i], tri.p[(i + 1) % 3]);
        if len(sub(p, q)) < len(sub(best.1, best.2)) {
            best = (s, p, q);
        }
    }
    best
}
fn project(rod: &mut HairRod, i: usize, t: f64, normal: V, penetration: f64) {
    if i == 0 && t < 0.15 {
        return;
    }
    let a = 1. - t;
    let b = t;
    let denom = rod.inv_mass[i] * a * a + rod.inv_mass[i + 1] * b * b;
    if denom < 1e-30 {
        return;
    }
    let max_weight = (rod.inv_mass[i] * a).max(rod.inv_mass[i + 1] * b);
    let impulse = (penetration / denom).min(rod.lengths[i] * 0.2 / max_weight.max(1e-30));
    rod.x[i] = add(rod.x[i], mul(normal, impulse * rod.inv_mass[i] * a));
    rod.x[i + 1] = add(rod.x[i + 1], mul(normal, impulse * rod.inv_mass[i + 1] * b));
    if rod.inv_mass[i] > 0. {
        rod.normals[i] = add(rod.normals[i], mul(normal, penetration * a));
    }
    rod.normals[i + 1] = add(rod.normals[i + 1], mul(normal, penetration * b));
    rod.contact_targets[i] = rod.x[i];
    rod.contact_targets[i + 1] = rod.x[i + 1];
}
pub(super) fn mesh_contacts(rod: &mut HairRod, meshes: &[TriangleMesh], radius: f64) {
    // Closest-surface projection also recovers nodes already inside the mesh,
    // which a narrow AABB overlap query alone cannot find.
    rod.nearest_cache.resize_with(meshes.len(), Vec::new);
    for cache in &mut rod.nearest_cache {
        cache.resize(rod.x.len(), usize::MAX);
    }
    rod.clearance_cache.resize_with(meshes.len(), Vec::new);
    for cache in &mut rod.clearance_cache {
        cache.resize(rod.x.len(), None);
    }
    for i in 1..rod.x.len() {
        for (mesh_index, mesh) in meshes.iter().enumerate() {
            if rod.clearance_cache[mesh_index][i]
                .is_some_and(|bound| bound.excludes(&[rod.x[i]], radius))
            {
                continue;
            }
            let cached = rod.nearest_cache[mesh_index][i];
            let mut best = if let Some(triangle) = mesh.triangles.get(cached) {
                let point = closest_triangle(rod.x[i], triangle);
                let delta = sub(rod.x[i], point);
                (dot(delta, delta), cached, point)
            } else {
                (f64::INFINITY, 0, [0.0; 3])
            };
            mesh.nearest(rod.x[i], 0, &mut best);
            rod.nearest_cache[mesh_index][i] = best.1;
            let tri = &mesh.triangles[best.1];
            let delta = sub(rod.x[i], best.2);
            let distance = best.0.sqrt();
            let signed = if dot(delta, tri.normal) < 0.0 {
                -distance
            } else {
                distance
            };
            rod.clearance_cache[mesh_index][i] = Some(Clearance {
                position: rod.x[i],
                distance: signed,
            });
            if signed < radius {
                let normal = if signed <= 0.0 || distance < 1e-12 {
                    tri.normal
                } else {
                    mul(delta, 1.0 / distance)
                };
                let correction = mul(normal, (radius - signed).min(rod.lengths[i - 1] * 0.2));
                rod.x[i] = add(rod.x[i], correction);
                rod.normals[i] = add(rod.normals[i], correction);
                rod.contact_targets[i] = rod.x[i];
            }
        }
    }
    let mut candidates = std::mem::take(&mut rod.contact_candidates);
    for i in 0..rod.lengths.len() {
        for (mesh_index, mesh) in meshes.iter().enumerate() {
            let a = rod.x[i];
            let b = rod.x[i + 1];
            let swept = [a, b, rod.old_x[i], rod.old_x[i + 1]];
            if [i, i + 1].into_iter().any(|point| {
                rod.clearance_cache[mesh_index][point]
                    .is_some_and(|bound| bound.excludes(&swept, radius))
            }) {
                continue;
            }
            // Swept bounds include previous endpoints; catches triangles crossed this substep.
            let min = std::array::from_fn(|j| {
                a[j].min(b[j]).min(rod.old_x[i][j]).min(rod.old_x[i + 1][j]) - radius
            });
            let max = std::array::from_fn(|j| {
                a[j].max(b[j]).max(rod.old_x[i][j]).max(rod.old_x[i + 1][j]) + radius
            });
            candidates.clear();
            mesh.query(min, max, 0, &mut candidates);
            let mut contact: Option<(f64, f64, V)> = None;
            for &id in &candidates {
                let tri = &mesh.triangles[id];
                let (t, p, q) = segment_triangle(a, b, tri);
                let delta = sub(p, q);
                let distance = len(delta);
                let signed = if dot(delta, tri.normal) < 0. {
                    -distance
                } else {
                    distance
                };
                if distance > radius {
                    continue;
                }
                // Choose nearest surface, not every tessellation face independently.
                let normal = if signed <= 0. || distance < 1e-10 {
                    tri.normal
                } else {
                    mul(delta, 1. / distance)
                };
                if contact.as_ref().is_none_or(|c| distance < c.0) {
                    contact = Some((distance, t, mul(normal, radius - distance)));
                }
            }
            if let Some((_, t, correction)) = contact {
                let depth = len(correction);
                if depth > 0. {
                    project(rod, i, t, unit(correction), depth);
                }
            }
        }
    }
    rod.contact_candidates = candidates;
}
// An injective 96-bit cell key retains signed coordinates while avoiding
// the array hash's length prefix in the self-contact broad phase.
fn cell_key(cell: [i32; 3]) -> (u64, u32) {
    ((u64::from(cell[0] as u32) << 32) | u64::from(cell[1] as u32), cell[2] as u32)
}

type SegmentId = (usize, usize);
type ContactGrid = HashMap<(u64, u32), Vec<(SegmentId, [i32; 3])>>;

fn unique_cell_pairs(grid: &ContactGrid) -> Vec<(SegmentId, SegmentId)> {
    let mut pairs = Vec::new();
    for (&key, entries) in grid {
        for a in 0..entries.len() {
            for b in a + 1..entries.len() {
                // Intersecting integer AABBs have a unique lowest shared cell.
                // Emit here only, avoiding repeated HashSet insertions elsewhere.
                let owner = std::array::from_fn(|axis| entries[a].1[axis].max(entries[b].1[axis]));
                if cell_key(owner) != key { continue; }
                let mut pair = (entries[a].0, entries[b].0);
                if pair.0 > pair.1 { pair = (pair.1, pair.0); }
                if pair.0.0 == pair.1.0 && pair.0.1.abs_diff(pair.1.1) <= 2 { continue; }
                pairs.push(pair);
            }
        }
    }
    pairs.sort_unstable();
    pairs
}

pub(super) fn self_contacts(rods: &mut [HairRod], radius: f64) {
    // Segment AABBs, not just particles, enter the spatial hash. Adjacent segments are excluded.
    let cell = 0.008f64.max(radius * 4.);
    let mut grid: ContactGrid = HashMap::new();
    for (r, rod) in rods.iter().enumerate() {
        for (i, p) in rod.x.windows(2).enumerate() {
            let min: [i32; 3] =
                std::array::from_fn(|a| ((p[0][a].min(p[1][a]) - radius) / cell).floor() as i32);
            let max: [i32; 3] =
                std::array::from_fn(|a| ((p[0][a].max(p[1][a]) + radius) / cell).floor() as i32);
            // Bound malformed/overstretched segments to avoid an unbounded hash insertion loop.
            if (0..3).any(|a| i64::from(max[a]) - i64::from(min[a]) > 32) {
                continue;
            }
            for x in min[0]..=max[0] {
                for y in min[1]..=max[1] {
                    for z in min[2]..=max[2] {
                        grid.entry(cell_key([x, y, z])).or_default().push(((r, i), min));
                    }
                }
            }
        }
    }
    let pairs = unique_cell_pairs(&grid);
    for ((ra, ia), (rb, ib)) in pairs {
        let separation = radius * 2.;
        // Grid cells are broader than a fibre. Reject disjoint current capsule
        // bounds before the more expensive segment-distance calculation.
        if (0..3).any(|axis| {
            let a0 = rods[ra].x[ia][axis];
            let a1 = rods[ra].x[ia + 1][axis];
            let b0 = rods[rb].x[ib][axis];
            let b1 = rods[rb].x[ib + 1][axis];
            a0.min(a1) - separation > b0.max(b1) || b0.min(b1) - separation > a0.max(a1)
        }) {
            continue;
        }
        let (s, t, p, q) = segment_pair(
            rods[ra].x[ia],
            rods[ra].x[ia + 1],
            rods[rb].x[ib],
            rods[rb].x[ib + 1],
        );
        let delta = sub(p, q);
        let distance = len(delta);
        if distance >= separation {
            continue;
        }
        // Shared/nearby follicles are allowed to overlap at the pinned boundary.
        if ia == 0 && ib == 0 && s < 0.2 && t < 0.2 {
            continue;
        }
        let normal = if distance > 1e-12 {
            mul(delta, 1. / distance)
        } else {
            let n = cross(
                sub(rods[ra].x[ia + 1], rods[ra].x[ia]),
                sub(rods[rb].x[ib + 1], rods[rb].x[ib]),
            );
            if len(n) > 1e-12 {
                unit(n)
            } else {
                [1., 0., 0.]
            }
        };
        let wa = rods[ra].inv_mass[ia] * (1. - s).powi(2) + rods[ra].inv_mass[ia + 1] * s * s;
        let wb = rods[rb].inv_mass[ib] * (1. - t).powi(2) + rods[rb].inv_mass[ib + 1] * t * t;
        if wa + wb < 1e-30 {
            continue;
        }
        let depth = separation - distance;
        if wa > 0. {
            project(&mut rods[ra], ia, s, normal, depth * wa / (wa + wb));
        }
        if wb > 0. {
            project(
                &mut rods[rb],
                ib,
                t,
                mul(normal, -1.),
                depth * wb / (wa + wb),
            );
        }
    }
}

#[cfg(test)]
mod query_tests {
    use super::*;
    #[test]
    fn clearance_skips_match_full_queries_when_particles_cross_surface() {
        let mesh = TriangleMesh::new(
            &[[-1., 0., -1.], [1., 0., -1.], [1., 0., 1.], [-1., 0., 1.]],
            &[[0, 2, 1], [0, 3, 2]],
        )
        .unwrap();
        let mut cached = HairRod::new(
            vec![[0., 0.05, 0.], [0.01, 0.04, 0.], [0.02, 0.03, 0.]],
            super::super::HairMaterial::default(),
        )
        .unwrap();
        let mut full = cached.clone();
        for i in 0..100 {
            let y = (i as f64 * 0.17).sin() * 0.04;
            for rod in [&mut cached, &mut full] {
                rod.old_x.clone_from(&rod.x);
                rod.x[1][1] = y;
                rod.x[2][1] = y + 0.005;
            }
            // Independent reference starts with fresh query storage on every call.
            full.contact_candidates = Vec::new();
            for bounds in &mut full.clearance_cache {
                bounds.fill(None);
            }
            mesh_contacts(&mut cached, std::slice::from_ref(&mesh), 0.001);
            mesh_contacts(&mut full, std::slice::from_ref(&mesh), 0.001);
            assert_eq!(cached.x, full.x);
            assert_eq!(cached.normals, full.normals);
        }
    }
    #[test]
    fn warm_start_remains_exact_after_surface_refit_and_far_motion() {
        let vertices: Vec<V> = (0..=8)
            .flat_map(|y| (0..=8).map(move |x| [x as f64 * 0.1, y as f64 * 0.1, 0.]))
            .collect();
        let indices: Vec<[usize; 3]> = (0..8)
            .flat_map(|y| {
                (0..8).flat_map(move |x| {
                    let a = y * 9 + x;
                    [[a, a + 1, a + 9], [a + 1, a + 10, a + 9]]
                })
            })
            .collect();
        let mut mesh = TriangleMesh::new(&vertices, &indices).unwrap();
        let moved: Vec<V> = vertices
            .iter()
            .map(|p| [p[0] + 1., p[1] - 2., p[2] + 0.3 * p[0]])
            .collect();
        mesh.refit(&moved).unwrap();
        let mut cached = 0;
        for sample in 0..100 {
            let p = [
                (sample * 37 % 101) as f64 * 0.03 - 0.5,
                (sample * 19 % 101) as f64 * 0.04 - 3.,
                0.5,
            ];
            let q = closest_triangle(p, &mesh.triangles[cached]);
            let delta = sub(p, q);
            let mut best = (dot(delta, delta), cached, q);
            mesh.nearest(p, 0, &mut best);
            let brute = mesh
                .triangles
                .iter()
                .map(|t| {
                    let q = closest_triangle(p, t);
                    let d = sub(p, q);
                    dot(d, d)
                })
                .fold(f64::INFINITY, f64::min);
            assert!((best.0 - brute).abs() < 1e-12);
            cached = best.1;
        }
    }
    #[test]
    fn capsule_query_detects_interior_contact_and_rejects_distant_plane_extension() {
        let mesh = TriangleMesh::new(&[[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]], &[[0, 1, 2]])
            .unwrap();
        let (_, correction) = mesh
            .capsule_contact([-0.5, 0., 0.05], [0.5, 0., 0.05], 0.1, [0., 0., 1.])
            .unwrap()
            .unwrap();
        assert!((correction[2] - 0.05).abs() < 1e-12);
        assert!(
            mesh.capsule_contact([3., 0., 0.05], [4., 0., 0.05], 0.1, [0., 0., 1.])
                .unwrap()
                .is_none()
        );
        let (t, correction) = mesh
            .capsule_contact([0., 0., -0.2], [0., 0., 0.2], 0.1, [0., 0., 1.])
            .unwrap()
            .unwrap();
        assert!((t - 0.5).abs() < 1e-12 && (correction[2] - 0.1).abs() < 1e-12);
    }
    #[test]
    fn leaf_query_contains_exactly_overlapping_triangle_bounds() {
        let vertices = [
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [4., 4., 0.],
            [5., 4., 0.],
            [4., 5., 0.],
        ];
        let mesh = TriangleMesh::new(&vertices, &[[0, 1, 2], [3, 4, 5]]).unwrap();
        let mut result = Vec::new();
        mesh.query([-0.1, -0.1, -0.1], [1.1, 1.1, 0.1], 0, &mut result);
        assert_eq!(result.len(), 1);
        assert_eq!(mesh.triangles[result[0]].ids, [0, 1, 2]);
    }
}

#[cfg(test)]
mod first_surface_segment_tests {
    use super::*;
    #[test]
    fn segment_hits_finite_surface_and_reports_distance_and_orientation() {
        let points = [
            [-1., -1., 0.],
            [1., -1., 0.],
            [0., 1., 0.],
            [-1., -1., -0.002],
            [0., 1., -0.002],
            [1., -1., -0.002],
        ];
        let mesh = TriangleMesh::new(&points, &[[0, 1, 2], [3, 4, 5]]).unwrap();
        let (fraction, point, normal) = mesh
            .first_segment_hit([0., 0., -0.00002], [0., 0., -0.01])
            .unwrap()
            .unwrap();
        assert!((fraction * 0.00998 - 0.00198).abs() < 1e-12);
        assert!((point[2] + 0.002).abs() < 1e-12 && normal[2] < -0.99);
        assert!(
            mesh.first_segment_hit([2., 0., -0.00002], [2., 0., -0.01])
                .unwrap()
                .is_none()
        );
        assert!(mesh.first_segment_hit([0.; 3], [0.; 3]).is_err());
    }
}

#[cfg(test)]
mod closed_surface_tests {
    use super::*;

    #[test]
    fn oriented_crossings_preserve_nested_cavities() {
        let mut points = vec![[0., 0., 0.], [3., 0., 0.], [0., 3., 0.], [0., 0., 3.]];
        let outer = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
        let mut faces = outer.to_vec();
        points.extend([
            [0.3, 0.3, 0.3],
            [0.6, 0.3, 0.3],
            [0.3, 0.6, 0.3],
            [0.3, 0.3, 0.6],
        ]);
        faces.extend(outer.map(|[a, b, c]| [a + 4, c + 4, b + 4]));
        let mesh = TriangleMesh::new(&points, &faces).unwrap();
        assert_eq!(
            mesh.contains_closed_surface([0.1, 0.1, 0.1]).unwrap(),
            Some(true)
        );
        assert_eq!(
            mesh.contains_closed_surface([0.35, 0.35, 0.35]).unwrap(),
            Some(false)
        );
        assert_eq!(
            mesh.contains_closed_surface([2., 2., 2.]).unwrap(),
            Some(false)
        );
    }

    #[test]
    fn surface_vertex_is_ambiguous_and_nonfinite_queries_are_rejected() {
        let mesh = TriangleMesh::new(
            &[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]],
        )
        .unwrap();
        assert_eq!(mesh.contains_closed_surface([0., 0., 0.]).unwrap(), None);
        assert!(mesh.contains_closed_surface([f64::NAN, 0., 0.]).is_err());
    }
}

#[cfg(test)]
mod cell_key_tests {
    use super::cell_key;
    #[test]
    fn signed_cell_coordinates_round_trip_without_aliasing() {
        let values = [i32::MIN, -65_536, -1, 0, 1, 65_536, i32::MAX];
        let mut keys = std::collections::HashSet::new();
        for x in values { for y in values { for z in values {
            let key = cell_key([x, y, z]);
            assert!(keys.insert(key));
            assert_eq!([(key.0 >> 32) as u32 as i32, key.0 as u32 as i32, key.1 as i32], [x, y, z]);
        } } }
    }
}

#[cfg(test)]
mod unique_pair_tests {
    use super::*;
    #[test]
    fn canonical_cells_preserve_hashset_candidates_exactly() {
        let mut grid = ContactGrid::new();
        for id in 0..48 {
            let min = [(id % 5) as i32 - 3, (id % 7) as i32 - 4, (id % 3) as i32 - 2];
            let max = [min[0] + 3, min[1] + 2, min[2] + 1];
            for x in min[0]..=max[0] { for y in min[1]..=max[1] { for z in min[2]..=max[2] {
                grid.entry(cell_key([x,y,z])).or_default().push(((id / 8, id % 8), min));
            } } }
        }
        let mut reference = std::collections::HashSet::new();
        for entries in grid.values() {
            for a in 0..entries.len() { for b in a+1..entries.len() {
                let mut pair = (entries[a].0, entries[b].0);
                if pair.0 > pair.1 { pair = (pair.1, pair.0); }
                if pair.0.0 == pair.1.0 && pair.0.1.abs_diff(pair.1.1) <= 2 { continue; }
                reference.insert(pair);
            } }
        }
        let mut reference: Vec<_> = reference.into_iter().collect();
        reference.sort_unstable();
        assert!(!reference.is_empty());
        assert_eq!(unique_cell_pairs(&grid), reference);
    }
}
