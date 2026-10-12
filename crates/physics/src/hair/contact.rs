use super::{ContactSource, HairRod, math::*};
use std::collections::HashMap;
#[path = "contact_continuous.rs"]
mod continuous;
#[path = "contact_polynomial.rs"]
mod polynomial;
#[path = "contact_triangle_sweep.rs"]
mod triangle_sweep;
pub use continuous::{
    CapsuleMotion, CapsuleSweep, CapsuleSweepOptions, sweep_capsules, swept_capsule_contacts,
    swept_capsule_pairs,
};
pub(super) use triangle_sweep::trajectory_contact;
pub use triangle_sweep::{TriangleMotion, sweep_capsule_triangle};
#[path = "contact_features.rs"]
mod features;
#[path = "velocity_contacts.rs"]
mod velocity_contacts;
pub(super) use velocity_contacts::{
    admit_staged_strands, advance_swept_strands, reconcile_contact_positions,
    reconcile_contact_positions_with_solver, reconcile_elastic_contact_positions_with_solver,
    reconcile_swept_contact_owners, recover_friction_pressure, stabilize_contact_velocities,
    stabilize_contact_velocities_with_solver, strand_fraction,
};
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
    previous_p: [V; 3],
    velocity: [V; 3],
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
    // Immutable derived data shared by collider clones; every geometry writer invalidates it.
    motion_bounds: std::sync::OnceLock<std::sync::Arc<triangle_sweep::MotionBounds>>,
    feature_normals: Option<features::FeatureNormals>,
    motion_dt: Option<f64>,
}
#[path = "contact_groom.rs"]
mod groom;
impl TriangleMesh {
    pub(super) fn replay_geometry(&self) -> (Vec<V>, Vec<V>, Vec<V>, Vec<[usize; 3]>, bool) {
        let n = self.triangles.iter().flat_map(|t| t.ids).max().unwrap() + 1;
        let mut current = vec![[0.; 3]; n];
        let mut previous = current.clone();
        let mut velocity = current.clone();
        for t in &self.triangles {
            for i in 0..3 {
                current[t.ids[i]] = t.p[i];
                previous[t.ids[i]] = t.previous_p[i];
                velocity[t.ids[i]] = t.velocity[i];
            }
        }
        (
            current,
            previous,
            velocity,
            self.triangles.iter().map(|t| t.ids).collect(),
            self.feature_normals.is_some(),
        )
    }
    #[allow(dead_code)] // Replay support kept for the hair contact diagnostics tooling.
    pub(super) fn restore_replay_motion(&mut self, previous: &[V], velocity: &[V]) {
        self.motion_bounds.take();
        for t in &mut self.triangles {
            t.previous_p = t.ids.map(|i| previous[i]);
            t.velocity = t.ids.map(|i| velocity[i]);
        }
    }
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
                previous_p: p,
                velocity: [[0.; 3]; 3],
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
            motion_bounds: std::sync::OnceLock::new(),
            feature_normals: None,
            motion_dt: None,
        };
        mesh.build(0..mesh.triangles.len());
        Ok(mesh)
    }
    /// Enables angle-weighted feature normals for a closed oriented collider.
    /// Geometrically coincident seams are welded once. Edge closure and winding
    /// and vertex fans are validated; callers must still exclude geometric
    /// self-intersections, which this check does not certify.
    pub fn enable_closed_feature_normals(&mut self) -> Result<(), &'static str> {
        let normals = features::FeatureNormals::new(&self.triangles)?;
        self.feature_normals = Some(normals);
        Ok(())
    }
    fn contact_normal(&self, face: usize, point: V) -> V {
        let triangle = &self.triangles[face];
        match &self.feature_normals {
            Some(normals) => normals.normal(
                face,
                closest_triangle_feature(point, triangle).1,
                triangle.normal,
            ),
            None => triangle.normal,
        }
    }
    /// Returns signed distance and closest-feature normal after explicit enable.
    /// Positive distance is outside an outward-oriented closed surface.
    pub fn signed_distance_closed(&self, point: V) -> Result<(f64, V), &'static str> {
        if !finite(point) {
            return Err("invalid surface query");
        }
        self.feature_normals
            .as_ref()
            .ok_or("closed feature normals are not enabled")?;
        let mut best = (f64::INFINITY, 0, [0.; 3]);
        self.nearest(point, 0, &mut best);
        let normal = self.contact_normal(best.1, point);
        let distance = best.0.sqrt();
        Ok((
            if dot(sub(point, best.2), normal) < 0. {
                -distance
            } else {
                distance
            },
            normal,
        ))
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
                if distance < best.0
                    || (distance == best.0 && self.triangles[i].ids < self.triangles[best.1].ids)
                {
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
        if self
            .feature_normals
            .as_ref()
            .is_some_and(|normals| !normals.validate_refit(&self.triangles, vertices))
        {
            return Err("animated collider splits welded topology or degenerates a face");
        }
        self.motion_bounds.take();
        for t in &mut self.triangles {
            t.previous_p = t.p;
            t.velocity = [[0.; 3]; 3];
            t.p = t.ids.map(|i| vertices[i]);
            t.normal = unit(cross(sub(t.p[1], t.p[0]), sub(t.p[2], t.p[0])));
            t.min = std::array::from_fn(|a| t.p.iter().map(|p| p[a]).fold(f64::INFINITY, f64::min));
            t.max =
                std::array::from_fn(|a| t.p.iter().map(|p| p[a]).fold(f64::NEG_INFINITY, f64::max));
        }
        if let Some(normals) = &mut self.feature_normals {
            normals.refresh(&self.triangles);
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
        self.motion_dt = None;
        Ok(())
    }
    /// Refit an animated collider and retain material-point velocities over dt.
    /// Invalid duration or overflowing motion leaves the old collider intact.
    pub fn refit_with_timestep(&mut self, vertices: &[V], dt: f64) -> Result<(), &'static str> {
        if !dt.is_finite()
            || dt <= 0.
            || self.triangles.iter().any(|triangle| {
                triangle.ids.iter().zip(triangle.p).any(|(&id, previous)| {
                    vertices
                        .get(id)
                        .is_none_or(|position| !finite(mul(sub(*position, previous), 1. / dt)))
                })
            })
        {
            return Err("invalid animated collider motion");
        }
        self.refit(vertices)?;
        for triangle in &mut self.triangles {
            triangle.velocity =
                std::array::from_fn(|i| mul(sub(triangle.p[i], triangle.previous_p[i]), 1. / dt));
        }
        self.motion_dt = Some(dt);
        Ok(())
    }
    pub(super) fn motion_duration(&self) -> Option<f64> {
        self.motion_dt
    }
    /// Immutable collider motion restricted to an authoritative time interval.
    /// Preserves material velocities and gives continuous queries matching
    /// start/end vertices instead of the full frame's previous pose.
    pub fn motion_interval(&self, start: f64, end: f64) -> Result<Self, &'static str> {
        if !start.is_finite() || !end.is_finite() || start < 0. || end > 1. || start >= end {
            return Err("invalid collider motion interval");
        }
        let mut sample = self.clone();
        sample.motion_bounds.take();
        if let Some(duration) = self.motion_dt {
            sample.sample_motion(self, end)?;
            for (sample, source) in sample.triangles.iter_mut().zip(&self.triangles) {
                let positions = std::array::from_fn(|i| {
                    add(
                        mul(source.previous_p[i], 1. - start),
                        mul(source.p[i], start),
                    )
                });
                if positions.iter().any(|p| !finite(*p))
                    || len(cross(
                        sub(positions[1], positions[0]),
                        sub(positions[2], positions[0]),
                    )) < 1e-14
                {
                    return Err("collider motion degenerates an intermediate face");
                }
                sample.previous_p = positions;
            }
            let h = duration * (end - start);
            if !h.is_finite() || h <= 0. {
                return Err("invalid collider motion interval duration");
            }
            sample.motion_dt = Some(h);
        } else {
            for face in &mut sample.triangles {
                face.previous_p = face.p;
                face.velocity = [[0.; 3]; 3];
            }
        }
        Ok(sample)
    }
    // Always sample authoritative endpoints, never the preceding sampled pose.
    pub(super) fn sample_motion(
        &mut self,
        source: &Self,
        fraction: f64,
    ) -> Result<(), &'static str> {
        if source.motion_dt.is_none() {
            return Ok(());
        }
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
            return Err("invalid collider motion fraction");
        }
        let count = source.triangles.iter().flat_map(|t| t.ids).max().unwrap() + 1;
        let mut vertices = vec![[0.; 3]; count];
        for triangle in &source.triangles {
            let positions = std::array::from_fn::<_, 3, _>(|i| {
                add(
                    mul(triangle.previous_p[i], 1. - fraction),
                    mul(triangle.p[i], fraction),
                )
            });
            if positions.iter().any(|p| !finite(*p))
                || len(cross(
                    sub(positions[1], positions[0]),
                    sub(positions[2], positions[0]),
                )) < 1e-14
            {
                return Err("collider motion degenerates an intermediate face");
            }
            for (id, position) in triangle.ids.into_iter().zip(positions) {
                vertices[id] = position;
            }
        }
        self.refit(&vertices)?;
        for (sample, original) in self.triangles.iter_mut().zip(&source.triangles) {
            sample.previous_p = original.previous_p;
            sample.velocity = original.velocity;
        }
        self.motion_dt = source.motion_dt;
        Ok(())
    }
    fn surface_velocity(&self, face: usize, point: V) -> V {
        if self.motion_dt.is_none() {
            return [0.; 3];
        }
        let triangle = &self.triangles[face];
        let (point, feature) = closest_triangle_feature(point, triangle);
        let weights = match feature {
            ClosestFeature::Vertex(i) => std::array::from_fn(|j| if i == j { 1. } else { 0. }),
            ClosestFeature::Edge(a, b) => {
                let edge = sub(triangle.p[b], triangle.p[a]);
                let t = (dot(sub(point, triangle.p[a]), edge) / dot(edge, edge)).clamp(0., 1.);
                std::array::from_fn(|i| {
                    if i == a {
                        1. - t
                    } else if i == b {
                        t
                    } else {
                        0.
                    }
                })
            }
            ClosestFeature::Face => {
                let ab = sub(triangle.p[1], triangle.p[0]);
                let ac = sub(triangle.p[2], triangle.p[0]);
                let relative = sub(point, triangle.p[0]);
                let area = len(cross(ab, ac));
                let b = dot(cross(relative, ac), triangle.normal) / area;
                let c = dot(cross(ab, relative), triangle.normal) / area;
                [1. - b - c, b, c]
            }
        };
        std::array::from_fn(|axis| {
            (0..3)
                .map(|i| weights[i] * triangle.velocity[i][axis])
                .sum()
        })
    }
    /// Transform a cached mesh rigidly without rebuilding topology. Matrix is row-major 3x4.
    pub fn transformed(&self, m: [[f64; 4]; 3]) -> Result<Self, &'static str> {
        if m.iter().flatten().any(|x| !x.is_finite()) {
            return Err("invalid collision transform");
        }
        let transform =
            |p: V| std::array::from_fn(|a| dot([m[a][0], m[a][1], m[a][2]], p) + m[a][3]);
        let mut result = self.clone();
        result.motion_bounds.take();
        for t in &mut result.triangles {
            t.p = t.p.map(transform);
            t.previous_p = t.previous_p.map(transform);
            t.velocity = t
                .velocity
                .map(|v| std::array::from_fn(|a| dot([m[a][0], m[a][1], m[a][2]], v)));
            t.normal = unit(cross(sub(t.p[1], t.p[0]), sub(t.p[2], t.p[0])));
            t.min = std::array::from_fn(|a| t.p.iter().map(|p| p[a]).fold(f64::INFINITY, f64::min));
            t.max =
                std::array::from_fn(|a| t.p.iter().map(|p| p[a]).fold(f64::NEG_INFINITY, f64::max));
        }
        if let Some(normals) = &mut result.feature_normals {
            normals.refresh(&result.triangles);
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
mod closest;
mod grid;
mod mesh_contacts;
mod strand_contacts;
mod strand_responses;
use closest::*;
use grid::*;
use mesh_contacts::*;
pub(in crate::hair) use mesh_contacts::{mesh_contacts, refresh_mesh_constraints};
#[cfg(test)]
pub(in crate::hair) use strand_contacts::reference_strand_refresh;
pub(in crate::hair) use strand_contacts::{
    refresh_strand_responses, self_contacts, strand_geometry_admitted,
};
pub(in crate::hair) use strand_responses::{
    StrandResponse, finish_strand_contacts, finish_strand_contacts_with_diagnostics,
};

#[cfg(test)]
mod query_tests;

#[cfg(test)]
mod first_surface_segment_tests;

#[cfg(test)]
mod closed_surface_tests;

#[cfg(test)]
mod cell_key_tests;

#[cfg(test)]
mod unique_pair_tests;

#[cfg(test)]
#[path = "segment_pair_fixture_tests.rs"]
mod segment_pair_fixture_tests;

#[cfg(test)]
#[path = "friction_diagnostics_tests.rs"]
mod friction_diagnostics_tests;
#[cfg(test)]
mod surface_tests;
