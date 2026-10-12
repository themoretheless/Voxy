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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClosestFeature {
    Vertex(usize),
    Edge(usize, usize),
    Face,
}
fn closest_triangle(p: V, t: &Triangle) -> V {
    closest_triangle_feature(p, t).0
}
// Preserve the geometric feature selected by the closest-point region tests.
// A face normal alone does not define the distance sign at edges or vertices.
fn closest_triangle_feature(p: V, t: &Triangle) -> (V, ClosestFeature) {
    let [a, b, c] = t.p;
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0. && d2 <= 0. {
        return (a, ClosestFeature::Vertex(0));
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0. && d4 <= d3 {
        return (b, ClosestFeature::Vertex(1));
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0. && d1 >= 0. && d3 <= 0. {
        return (add(a, mul(ab, d1 / (d1 - d3))), ClosestFeature::Edge(0, 1));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0. && d5 <= d6 {
        return (c, ClosestFeature::Vertex(2));
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return (add(a, mul(ac, d2 / (d2 - d6))), ClosestFeature::Edge(0, 2));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        return (
            add(b, mul(sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6)))),
            ClosestFeature::Edge(1, 2),
        );
    }
    let denom = 1. / (va + vb + vc);
    (
        add(a, add(mul(ab, vb * denom), mul(ac, vc * denom))),
        ClosestFeature::Face,
    )
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
    if aa == 0. {
        let t = if cc > 0. { (ee / cc).clamp(0., 1.) } else { 0. };
        return (0., t, a, add(c, mul(v, t)));
    }
    if cc == 0. {
        let s = (-dd / aa).clamp(0., 1.);
        return (s, 0., add(a, mul(u, s)), c);
    }
    // |u x v|^2 is the same determinant as aa*cc-bb*bb, without
    // subtracting almost equal squared lengths for near-parallel segments.
    // The matching cross-product numerator retains their interior minimum.
    let uv = cross(u, v);
    let den = dot(uv, uv);
    let mut s = if den > 0. {
        (dot(cross(v, w), uv) / den).clamp(0., 1.)
    } else {
        0.
    };
    let mut t = (bb * s + ee) / cc;
    if t < 0. {
        t = 0.;
        s = (-dd / aa).clamp(0., 1.);
    } else if t > 1. {
        t = 1.;
        s = ((bb - dd) / aa).clamp(0., 1.);
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
            let (q, feature) = closest_triangle_feature(p, tri);
            if matches!(feature, ClosestFeature::Face) {
                // This is an interior plane/segment intersection, hence zero
                // separation. Reprojecting the same intersection introduces
                // ULP noise which makes flat trajectory minima pick unrelated
                // times and segment weights. Edge/vertex near misses retain
                // their actual distinct closest points below.
                return (s, p, p);
            }
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
// Clip the projection of a segment to the closed triangular face. Each
// half-plane is linear in its segment coordinate; normal displacement does
// not affect the cross-product test. No distance tolerance enlarges the face.
fn projected_face_interval(a: V, b: V, tri: &Triangle) -> Option<(f64, f64)> {
    let mut lo = 0f64;
    let mut hi = 1f64;
    for edge in 0..3 {
        let origin = tri.p[edge];
        let direction = sub(tri.p[(edge + 1) % 3], origin);
        let left = dot(cross(direction, sub(a, origin)), tri.normal);
        let right = dot(cross(direction, sub(b, origin)), tri.normal);
        if left < 0. && right < 0. {
            return None;
        }
        if left < 0. {
            lo = lo.max(left / (left - right));
        }
        if right < 0. {
            hi = hi.min(left / (left - right));
        }
        if lo > hi {
            return None;
        }
    }
    Some((lo, hi))
}
fn project(
    rod: &mut HairRod,
    i: usize,
    t: f64,
    normal: V,
    penetration: f64,
    surface_velocity: Option<V>,
) -> bool {
    if i == 0 && t < 0.15 {
        return false;
    }
    let a = 1. - t;
    let b = t;
    let denom = rod.inv_mass[i] * a * a + rod.inv_mass[i + 1] * b * b;
    if denom < 1e-30 {
        return false;
    }
    let max_weight = (rod.inv_mass[i] * a).max(rod.inv_mass[i + 1] * b);
    let impulse = (penetration / denom).min(rod.lengths[i] * 0.2 / max_weight.max(1e-30));
    let correction_a = mul(normal, impulse * rod.inv_mass[i] * a);
    let correction_b = mul(normal, impulse * rod.inv_mass[i + 1] * b);
    rod.x[i] = add(rod.x[i], correction_a);
    rod.x[i + 1] = add(rod.x[i + 1], correction_b);
    // Accumulate the applied generalized contact impulse, including fixed
    // endpoint weights and the displacement cap, rather than raw penetration.
    if let Some(velocity) = surface_velocity {
        rod.record_surface_response(i, correction_a, velocity);
        rod.record_surface_response(i + 1, correction_b, velocity);
    }
    true
}
fn contact_weight(rod: &HairRod, i: usize, t: f64) -> f64 {
    if i == 0 && t < 0.15 {
        return 0.;
    }
    rod.inv_mass[i] * (1. - t).powi(2) + rod.inv_mass[i + 1] * t.powi(2)
}
fn pair_impulse(
    rods: &[HairRod],
    a: (usize, usize, f64),
    b: (usize, usize, f64),
    depth: f64,
) -> f64 {
    let weight = contact_weight(&rods[a.0], a.1, a.2) + contact_weight(&rods[b.0], b.1, b.2);
    if weight < 1e-30 {
        return 0.;
    }
    let mut impulse = depth.max(0.) / weight;
    // One shared multiplier preserves equal/opposite generalized impulses.
    // Independently clamping either strand changes their momentum balance.
    for (rod, segment, fraction) in [a, b] {
        let rod = &rods[rod];
        if contact_weight(rod, segment, fraction) == 0. {
            continue;
        }
        let maximum =
            (rod.inv_mass[segment] * (1. - fraction)).max(rod.inv_mass[segment + 1] * fraction);
        impulse = impulse.min(rod.lengths[segment] * 0.2 / maximum);
    }
    impulse
}
#[derive(Clone)]
pub(super) struct StrandResponse {
    pub(super) a: (usize, usize, f64),
    pub(super) b: (usize, usize, f64),
    pub(super) normal: V,
    pub(super) impulse: f64,
}
pub(super) fn finish_strand_contacts(rods: &mut [HairRod], responses: &[StrandResponse], dt: f64) {
    finish_strand_contacts_with_diagnostics(rods, responses, dt, 0, None);
}
pub(super) fn finish_strand_contacts_with_diagnostics(
    rods: &mut [HairRod],
    responses: &[StrandResponse],
    dt: f64,
    substep: usize,
    mut diagnostics: Option<&mut Vec<super::HairFrictionDiagnostic>>,
) {
    for response in responses {
        let (ra, ia, s) = response.a;
        let (rb, ib, t) = response.b;
        let wa = contact_weight(&rods[ra], ia, s);
        let wb = contact_weight(&rods[rb], ib, t);
        if wa + wb < 1e-30 {
            continue;
        }
        let va = add(
            mul(rods[ra].velocity[ia], 1. - s),
            mul(rods[ra].velocity[ia + 1], s),
        );
        let vb = add(
            mul(rods[rb].velocity[ib], 1. - t),
            mul(rods[rb].velocity[ib + 1], t),
        );
        let relative = sub(va, vb);
        let normal_speed = dot(relative, response.normal);
        let normal_impulse = (-normal_speed / (wa + wb)).max(0.);
        let tangent = sub(relative, mul(response.normal, normal_speed));
        let tangent_speed = len(tangent);
        let friction = (rods[ra].material.friction * rods[rb].material.friction).sqrt();
        let tangent_impulse =
            (friction * (response.impulse / dt + normal_impulse)).min(tangent_speed / (wa + wb));
        let impulse = sub(
            mul(response.normal, normal_impulse),
            mul(tangent, tangent_impulse / tangent_speed.max(1e-30)),
        );
        if let Some(diagnostics) = diagnostics.as_mut() {
            diagnostics.push(super::HairFrictionDiagnostic {
                substep,
                a: response.a,
                b: response.b,
                normal: response.normal,
                position_impulse: response.impulse,
                relative_velocity: relative,
                normal_speed,
                tangent_speed,
                mobility_a: wa,
                mobility_b: wb,
                friction,
                normal_impulse,
                tangent_impulse,
                applied_impulse: impulse,
            });
        }

        for (rod, segment, fraction, sign) in [(ra, ia, s, 1.), (rb, ib, t, -1.)] {
            if contact_weight(&rods[rod], segment, fraction) == 0. {
                continue;
            }
            for (point, weight) in [(segment, 1. - fraction), (segment + 1, fraction)] {
                let change = mul(impulse, sign * weight * rods[rod].inv_mass[point]);
                rods[rod].velocity[point] = add(rods[rod].velocity[point], change);
            }
        }
    }
}
// A segment can intersect several faces at numerically zero distance. Choose
// its first intersection, with a topology key for coincident shared features,
// rather than allowing roundoff in the distance to select a distant face.
fn segment_contact_precedes(
    distance: f64,
    t: f64,
    key: [usize; 3],
    old_distance: f64,
    old_t: f64,
    old_key: [usize; 3],
    length: f64,
) -> bool {
    const ZERO_DISTANCE: f64 = 1e-10;
    if distance <= ZERO_DISTANCE && old_distance <= ZERO_DISTANCE {
        if (t - old_t).abs() * length <= ZERO_DISTANCE {
            key < old_key
        } else {
            t < old_t
        }
    } else {
        distance < old_distance || (distance == old_distance && key < old_key)
    }
}
// Feature pseudonormals classify the distance sign. Away from zero distance,
// the metric gradient follows the closest-point displacement, also from inside.
fn distance_gradient(delta: V, distance: f64, signed: f64, fallback: V, epsilon: f64) -> V {
    if distance < epsilon {
        fallback
    } else {
        mul(delta, signed.signum() / distance)
    }
}
// A face-interior metric gradient is exactly its outward geometric normal.
// Reconstructing it from two rounded world points adds tangential force,
// especially when the surface gap is tiny or the world origin is distant.
// Edges/vertices retain the signed metric displacement (not a face normal).
fn triangle_distance_gradient(
    triangle: &Triangle,
    point: V,
    closest: V,
    signed: f64,
    fallback: V,
    epsilon: f64,
) -> V {
    if signed.abs() >= epsilon
        && matches!(
            closest_triangle_feature(point, triangle).1,
            ClosestFeature::Face
        )
    {
        triangle.normal
    } else {
        distance_gradient(sub(point, closest), signed.abs(), signed, fallback, epsilon)
    }
}
pub(super) fn mesh_contacts(rod: &mut HairRod, meshes: &[TriangleMesh], radius: f64) {
    gather_mesh_contacts(rod, meshes, radius, true);
}
// Approximate the deepest signed-distance witness in one interior interval.
// Coarse samples bracket candidate minima; bounded refinement retains every
// sampled candidate, including the old midpoint. This is constraint discovery,
// not a global segment-clearance certificate for arbitrary nonconvex meshes.
fn deepest_interval_witness(
    mesh: &TriangleMesh,
    a: V,
    b: V,
    lo: f64,
    hi: f64,
) -> Result<(f64, f64, V), &'static str> {
    let evaluate = |t: f64| {
        mesh.signed_distance_closed(add(mul(a, 1. - t), mul(b, t)))
            .map(|(d, n)| (t, d, n))
    };
    let samples = (0..=8)
        .map(|i| evaluate(lo + (hi - lo) * i as f64 / 8.))
        .collect::<Result<Vec<_>, _>>()?;
    let segment_length = len(sub(b, a));
    let mut best = samples[4];
    for value in &samples {
        if value.1 < best.1 {
            best = *value;
        }
    }
    for i in 0..=8 {
        // A deepest endpoint sample can hide an even deeper medial peak in
        // the last sampling cell. Refine those boundary cells as well as
        // interior brackets; retain the endpoint itself in `best`.
        let (start, end) = if i == 0 {
            if samples[0].1 > samples[1].1 {
                continue;
            }
            (0, 1)
        } else if i == 8 {
            if samples[8].1 > samples[7].1 {
                continue;
            }
            (7, 8)
        } else {
            if samples[i].1 > samples[i - 1].1 || samples[i].1 > samples[i + 1].1 {
                continue;
            }
            (i - 1, i + 1)
        };
        let mut left = samples[start].0;
        let mut right = samples[end].0;
        let ratio = (5f64.sqrt() - 1.) * 0.5;
        let mut x = evaluate(right - ratio * (right - left))?;
        let mut y = evaluate(left + ratio * (right - left))?;
        for _ in 0..80 {
            for value in [x, y] {
                if value.1 < best.1 {
                    best = value;
                }
            }
            // Discovery precision must be finer than the 1e-10 m contact
            // admission gate, including long/scaled segments. Keep a bounded
            // search, but stop by physical interval width rather than count.
            if (right - left) * segment_length <= 1e-12 {
                break;
            }
            if x.1 <= y.1 {
                right = y.0;
                y = x;
                x = evaluate(right - ratio * (right - left))?;
            } else {
                left = x.0;
                x = y;
                y = evaluate(left + ratio * (right - left))?;
            }
        }
    }
    let position = add(mul(a, 1. - best.0), mul(b, best.0));
    let mut nearest = (f64::INFINITY, 0, [0.; 3]);
    mesh.nearest(position, 0, &mut nearest);
    // Feature pseudonormals determine sign, not the metric derivative at
    // concave edges. Use the actual closest-point distance gradient.
    best.2 = triangle_distance_gradient(
        &mesh.triangles[nearest.1],
        position,
        nearest.2,
        best.1,
        best.2,
        1e-10,
    );
    Ok(best)
}
// At a medial minimum of signed distance along a segment, combine the
// two observed one-sided metric gradients so their segment derivative is zero.
// Return unit force direction AND its original gradient magnitude separately.
fn interior_envelope_gradient(
    mesh: &TriangleMesh,
    a: V,
    b: V,
    t: f64,
    lo: f64,
    hi: f64,
    normal: V,
    velocity: V,
) -> (V, f64, V) {
    let delta = (hi - lo) * 1e-6;
    if t - delta <= lo || t + delta >= hi {
        return (normal, 1., velocity);
    }
    let probe = |fraction: f64| -> Option<(V, V)> {
        let p = add(mul(a, 1. - fraction), mul(b, fraction));
        let (signed, pseudo) = mesh.signed_distance_closed(p).ok()?;
        let mut nearest = (f64::INFINITY, 0, [0.; 3]);
        mesh.nearest(p, 0, &mut nearest);
        Some((
            triangle_distance_gradient(
                &mesh.triangles[nearest.1],
                p,
                nearest.2,
                signed,
                pseudo,
                1e-10,
            ),
            mesh.surface_velocity(nearest.1, nearest.2),
        ))
    };
    let (Some((left, vl)), Some((right, vr))) = (probe(t - delta), probe(t + delta)) else {
        return (normal, 1., velocity);
    };
    let segment = sub(b, a);
    let dl = dot(left, segment);
    let dr = dot(right, segment);
    if !(dl < 0. && dr > 0.) {
        return (normal, 1., velocity);
    }
    let w = dr / (dr - dl);
    let g = add(mul(left, w), mul(right, 1. - w));
    let scale = len(g);
    // A zero generalized gradient needs a nonlocal escape, not division by zero.
    if !scale.is_finite() || scale < 1e-12 || scale > 1. + 1e-12 {
        return (normal, 1., velocity);
    }
    let direction = mul(g, 1. / scale);
    let blended = add(mul(vl, w), mul(vr, 1. - w));
    let normal_speed = (w * dot(left, vl) + (1. - w) * dot(right, vr)) / scale;
    let velocity = add(
        blended,
        mul(direction, normal_speed - dot(direction, blended)),
    );
    (direction, scale, velocity)
}
pub(super) fn refresh_mesh_constraints(rod: &mut HairRod, meshes: &[TriangleMesh], radius: f64) {
    gather_mesh_contacts(rod, meshes, radius, false);
}
fn gather_mesh_contacts(
    rod: &mut HairRod,
    meshes: &[TriangleMesh],
    radius: f64,
    project_positions: bool,
) {
    const CONTACT_TOLERANCE: f64 = 1e-10;
    let query_radius = radius + CONTACT_TOLERANCE;
    rod.contacts
        .retain(|contact| !matches!(contact.source, ContactSource::Mesh(_)));
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
                .is_some_and(|bound| bound.excludes(&[rod.x[i]], query_radius))
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
            let contact_normal = mesh.contact_normal(best.1, rod.x[i]);
            let delta = sub(rod.x[i], best.2);
            let distance = best.0.sqrt();
            let signed = if dot(delta, contact_normal) < 0.0 {
                -distance
            } else {
                distance
            };
            rod.clearance_cache[mesh_index][i] = Some(Clearance {
                position: rod.x[i],
                distance: signed,
            });
            if signed <= query_radius {
                let normal = triangle_distance_gradient(
                    &mesh.triangles[best.1],
                    rod.x[i],
                    best.2,
                    signed,
                    contact_normal,
                    1e-12,
                );
                let depth = radius - signed;
                let target = add(rod.x[i], mul(normal, depth));
                let contact =
                    rod.record_point_contact(i, normal, target, ContactSource::Mesh(mesh_index));
                rod.contacts[contact].surface_velocity = mesh.surface_velocity(best.1, best.2);
                let correction = mul(normal, depth.max(0.).min(rod.lengths[i - 1] * 0.2));
                if project_positions {
                    rod.x[i] = add(rod.x[i], correction);
                    rod.record_surface_response(
                        i,
                        correction,
                        mesh.surface_velocity(best.1, best.2),
                    );
                }
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
                    .is_some_and(|bound| bound.excludes(&swept, query_radius))
            }) {
                continue;
            }
            // Swept bounds include previous endpoints; catches triangles crossed this substep.
            let min = std::array::from_fn(|j| {
                a[j].min(b[j]).min(rod.old_x[i][j]).min(rod.old_x[i + 1][j]) - query_radius
            });
            let max = std::array::from_fn(|j| {
                a[j].max(b[j]).max(rod.old_x[i][j]).max(rod.old_x[i + 1][j]) + query_radius
            });
            candidates.clear();
            mesh.query(min, max, 0, &mut candidates);
            let mut contact: Option<(f64, f64, V, f64, [usize; 3], V)> = None;
            let mut crossings = Vec::new();
            for &id in &candidates {
                let tri = &mesh.triangles[id];
                let (t, p, q) = segment_triangle(a, b, tri);
                // Follicle overlap is prescribed. Do not let an allowed root
                // contact hide a later collision on the free part of the strand.
                if i == 0 && t < 0.15 {
                    continue;
                }
                let contact_normal = mesh.contact_normal(id, p);
                let delta = sub(p, q);
                let distance = len(delta);
                if mesh.feature_normals.is_some() && distance < 1e-10 {
                    crossings.push(t);
                }
                let signed = if dot(delta, contact_normal) < 0. {
                    -distance
                } else {
                    distance
                };
                if distance > query_radius {
                    continue;
                }
                // Choose nearest surface, not every tessellation face independently.
                let normal = triangle_distance_gradient(tri, p, q, signed, contact_normal, 1e-10);
                if contact.as_ref().is_none_or(|c| {
                    segment_contact_precedes(distance, t, tri.ids, c.0, c.1, c.4, len(sub(b, a)))
                }) {
                    contact = Some((
                        distance,
                        t,
                        normal,
                        signed,
                        tri.ids,
                        mesh.surface_velocity(id, q),
                    ));
                }
            }
            // A segment with exterior endpoints can traverse a closed body.
            // Its boundary intersection has zero distance, hiding the actual
            // penetration. Add interior witnesses between surface crossings;
            // these discover constraints, never certify continuous clearance.
            // A segment entirely inside the volume has no boundary crossings.
            // It still needs an interior depth witness; endpoint constraints
            // alone can miss a deeper penetration in the middle.
            let free_start = if i == 0 { 0.15 } else { 0. };
            let inside_endpoint = mesh.feature_normals.is_some()
                && [free_start, 1.].into_iter().any(|t| {
                    let p = add(mul(a, 1. - t), mul(b, t));
                    let node = if t == 0. {
                        Some(i)
                    } else if t == 1. {
                        Some(i + 1)
                    } else {
                        None
                    };
                    if let Some(bound) =
                        node.and_then(|point| rod.clearance_cache[mesh_index][point])
                    {
                        if bound.excludes(&[p], query_radius) {
                            return false;
                        }
                        // Reuse the exact signed endpoint query above only at the
                        // same position. Moved/projected points must be re-queried.
                        if bound.position == p {
                            return bound.distance < -CONTACT_TOLERANCE;
                        }
                    }
                    mesh.signed_distance_closed(p)
                        .is_ok_and(|(distance, _)| distance < -CONTACT_TOLERANCE)
                });
            if !crossings.is_empty() || inside_endpoint {
                crossings.extend([if i == 0 { 0.15 } else { 0.0 }, 1.0]);
                crossings.retain(|t| *t >= if i == 0 { 0.15 } else { 0.0 });
                crossings.sort_by(f64::total_cmp);
                crossings.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
                for interval in crossings.windows(2) {
                    if let Ok((t, signed, normal)) =
                        deepest_interval_witness(mesh, a, b, interval[0], interval[1])
                    {
                        let position = add(mul(a, 1. - t), mul(b, t));
                        if signed >= -CONTACT_TOLERANCE {
                            continue;
                        }
                        let mut nearest = (f64::INFINITY, 0, [0.; 3]);
                        mesh.nearest(position, 0, &mut nearest);
                        let velocity = mesh.surface_velocity(nearest.1, nearest.2);
                        let (normal, scale, velocity) = interior_envelope_gradient(
                            mesh,
                            a,
                            b,
                            t,
                            interval[0],
                            interval[1],
                            normal,
                            velocity,
                        );
                        let depth = (radius - signed) / scale;
                        let index = rod.record_contact(
                            i,
                            t,
                            normal,
                            add(position, mul(normal, depth)),
                            ContactSource::Mesh(mesh_index),
                        );
                        rod.contacts[index].metric_scale = scale;
                        rod.contacts[index].surface_velocity = velocity;
                        if project_positions {
                            project(rod, i, t, normal, depth, Some(velocity));
                        }
                    }
                }
            }
            // A flat face supports an interval, not one arbitrary closest
            // point. A tiny tilt can switch the single witness between its
            // ends and cause a finite change in the velocity Jacobian. Keep
            // both supported endpoints as original physical constraints.
            // The tolerance only triggers discovery: each endpoint must still
            // be inside the exact projected face, within the original query
            // radius, and have this face as its globally nearest triangle.
            // Inspect every overlapping face: restricting the interval to the
            // selected minimum loses support across a tessellation boundary.
            if contact.is_some() {
                for &id in &candidates {
                    let tri = &mesh.triangles[id];
                    if dot(sub(b, a), tri.normal).abs() <= CONTACT_TOLERANCE {
                        if let Some((lo, hi)) = projected_face_interval(a, b, tri) {
                            for t in [lo.max(free_start), hi] {
                                if t < free_start || t > hi {
                                    continue;
                                }
                                let position = add(mul(a, 1. - t), mul(b, t));
                                let q = closest_triangle(position, tri);
                                let delta = sub(position, q);
                                let distance = len(delta);
                                if distance > query_radius {
                                    continue;
                                }
                                let mut nearest = (f64::INFINITY, 0, [0.; 3]);
                                mesh.nearest(position, 0, &mut nearest);
                                if nearest.1 != id {
                                    continue;
                                }
                                let pseudo = mesh.contact_normal(id, position);
                                let signed = if dot(delta, pseudo) < 0. {
                                    -distance
                                } else {
                                    distance
                                };
                                let normal = triangle_distance_gradient(
                                    tri, position, q, signed, pseudo, 1e-12,
                                );
                                let index = rod.record_contact(
                                    i,
                                    t,
                                    normal,
                                    add(position, mul(normal, radius - signed)),
                                    ContactSource::Mesh(mesh_index),
                                );
                                rod.contacts[index].surface_velocity = mesh.surface_velocity(id, q);
                            }
                        }
                    }
                }
            }
            if let Some((_, t, normal, signed, _, velocity)) = contact {
                let depth = radius - signed;
                let position = add(mul(a, 1. - t), mul(b, t));
                let contact = rod.record_contact(
                    i,
                    t,
                    normal,
                    add(position, mul(normal, depth)),
                    ContactSource::Mesh(mesh_index),
                );
                rod.contacts[contact].surface_velocity = velocity;
                if depth > 0. && project_positions {
                    project(rod, i, t, normal, depth, Some(velocity));
                }
            }
        }
    }
    rod.contact_candidates = candidates;
}
// An injective 96-bit cell key retains signed coordinates while avoiding
// the array hash's length prefix in the self-contact broad phase.
fn cell_key(cell: [i32; 3]) -> (u64, u32) {
    (
        (u64::from(cell[0] as u32) << 32) | u64::from(cell[1] as u32),
        cell[2] as u32,
    )
}

type SegmentId = (usize, usize);
type ContactGrid = HashMap<(u64, u32), Vec<(SegmentId, [i32; 3])>>;

// Preserve lexicographic pair order while sorting compact integer keys.
fn sort_contact_pairs(pairs: &mut [(SegmentId, SegmentId)]) {
    if pairs.len() < 128 {
        pairs.sort_unstable();
        return;
    }
    let (mut rod_max, mut segment_max) = (0usize, 0usize);
    for &(a, b) in pairs.iter() {
        rod_max = rod_max.max(a.0).max(b.0);
        segment_max = segment_max.max(a.1).max(b.1);
    }
    let rod_bits = (usize::BITS - rod_max.leading_zeros()).max(1);
    let segment_bits = (usize::BITS - segment_max.leading_zeros()).max(1);
    if 2 * (rod_bits + segment_bits) > 64 {
        pairs.sort_unstable();
        return;
    }
    let mut keys: Vec<u64> = pairs
        .iter()
        .map(|&(a, b)| {
            (((((a.0 as u64) << segment_bits) | a.1 as u64) << rod_bits | b.0 as u64)
                << segment_bits)
                | b.1 as u64
        })
        .collect();
    keys.sort_unstable();
    let rod_mask = (1u64 << rod_bits) - 1;
    let segment_mask = (1u64 << segment_bits) - 1;
    for (pair, mut key) in pairs.iter_mut().zip(keys) {
        let ib = (key & segment_mask) as usize;
        key >>= segment_bits;
        let rb = (key & rod_mask) as usize;
        key >>= rod_bits;
        let ia = (key & segment_mask) as usize;
        key >>= segment_bits;
        *pair = ((key as usize, ia), (rb, ib));
    }
}

fn unique_cell_pairs(grid: &ContactGrid) -> Vec<(SegmentId, SegmentId)> {
    unique_cell_pairs_filtered(grid, |_| true)
}
fn unique_cell_pairs_filtered(
    grid: &ContactGrid,
    mut keep: impl FnMut((SegmentId, SegmentId)) -> bool,
) -> Vec<(SegmentId, SegmentId)> {
    let mut pairs = Vec::with_capacity(grid.len().saturating_mul(2));
    for (&key, entries) in grid {
        for a in 0..entries.len() {
            for b in a + 1..entries.len() {
                let owner = std::array::from_fn(|axis| entries[a].1[axis].max(entries[b].1[axis]));
                if cell_key(owner) != key {
                    continue;
                }
                let mut pair = (entries[a].0, entries[b].0);
                if pair.0 > pair.1 {
                    pair = (pair.1, pair.0);
                }
                if pair.0.0 == pair.1.0 && pair.0.1.abs_diff(pair.1.1) <= 2 {
                    continue;
                }
                if !keep(pair) {
                    continue;
                }
                pairs.push(pair);
            }
        }
    }
    sort_contact_pairs(&mut pairs);
    pairs
}

#[cfg(test)]
fn candidate_cell_pairs(grid: &ContactGrid) -> Vec<(SegmentId, SegmentId)> {
    let mut pairs = Vec::new();
    for (&key, entries) in grid {
        let cell = [
            (key.0 >> 32) as u32 as i32,
            key.0 as u32 as i32,
            key.1 as i32,
        ];
        for a in 0..entries.len() {
            for b in a + 1..entries.len() {
                let mut pair = (entries[a].0, entries[b].0);
                if pair.0.0 == pair.1.0 && pair.0.1.abs_diff(pair.1.1) <= 2 {
                    continue;
                }
                // Reject duplicate cells axis by axis before packing a new key.
                // The first shared integer cell remains the sole pair owner.
                if (0..3).any(|axis| entries[a].1[axis].max(entries[b].1[axis]) != cell[axis]) {
                    continue;
                }
                if pair.0 > pair.1 {
                    pair = (pair.1, pair.0);
                }
                pairs.push(pair);
            }
        }
    }
    pairs.sort_unstable();
    pairs
}

pub(super) fn self_contacts(rods: &mut [HairRod], radius: f64) -> Vec<StrandResponse> {
    gather_strand_contacts(rods, radius, true)
}
/// Query witnesses must be refreshed before use. A pair owns one clearance
/// budget; splitting its positional targets does not split that budget.
pub(super) fn strand_geometry_admitted(
    rods: &[HairRod],
    responses: &[StrandResponse],
    radius: f64,
) -> Result<bool, &'static str> {
    for response in responses {
        let (ra, ia, s) = response.a;
        let (rb, ib, t) = response.b;
        let a = add(mul(rods[ra].x[ia], 1. - s), mul(rods[ra].x[ia + 1], s));
        let b = add(mul(rods[rb].x[ib], 1. - t), mul(rods[rb].x[ib + 1], t));
        let gap = len(sub(a, b)) - 2. * radius;
        if !gap.is_finite() {
            return Err("joint strand contact geometry overflow");
        }
        if gap < -1e-10 {
            return Ok(false);
        }
    }
    Ok(true)
}
/// Re-query the current geometry without moving it. Historical projections supply
/// friction load only; old normals and released pairs must not constrain velocity.
pub(super) fn refresh_strand_responses(
    rods: &mut [HairRod],
    radius: f64,
    history: &[StrandResponse],
) -> Vec<StrandResponse> {
    let mut current = gather_strand_contacts(rods, radius, false);
    if history.is_empty() {
        return current;
    }
    let mut witnesses: std::collections::BTreeMap<((usize, usize), (usize, usize)), Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, response) in current.iter().enumerate() {
        witnesses
            .entry(((response.a.0, response.a.1), (response.b.0, response.b.1)))
            .or_default()
            .push(index);
    }
    // A historical projection supplies one pressure budget, not a new copy
    // for every point of the current manifold. Preserve unchanged witnesses
    // exactly; transfer a moved/released witness to the closest current point
    // of the same segment pair. This is a friction history hint; the coupled
    // pressure recovery solve owns newly solved physical contact reactions.
    for old in history {
        let Some(indices) = witnesses.get(&((old.a.0, old.a.1), (old.b.0, old.b.1))) else {
            continue;
        };
        let a = sub(rods[old.a.0].x[old.a.1 + 1], rods[old.a.0].x[old.a.1]);
        let b = sub(rods[old.b.0].x[old.b.1 + 1], rods[old.b.0].x[old.b.1]);
        let metric = |index: usize| {
            let new = &current[index];
            (new.a.2 - old.a.2).powi(2) * dot(a, a) + (new.b.2 - old.b.2).powi(2) * dot(b, b)
        };
        if let Some(&index) = indices
            .iter()
            .min_by(|&&a, &&b| metric(a).total_cmp(&metric(b)))
        {
            current[index].impulse += dot(old.normal, current[index].normal).max(0.) * old.impulse;
        }
    }
    current
}
// Scale before normalizing: squaring a nonzero separation can underflow,
// but that must not erase its geometric direction.
fn normalized_contact_direction(vector: V) -> Option<V> {
    if !finite(vector) {
        return None;
    }
    let scale = vector.into_iter().map(f64::abs).fold(0f64, f64::max);
    if scale == 0. {
        return None;
    }
    Some(unit(vector.map(|value| value / scale)))
}
fn feature_strand_normal(
    a: &HairRod,
    ia: usize,
    s: f64,
    b: &HairRod,
    ib: usize,
    t: f64,
    delta: V,
) -> V {
    let ta = normalized_contact_direction(sub(a.x[ia + 1], a.x[ia]));
    let tb = normalized_contact_direction(sub(b.x[ib + 1], b.x[ib]));
    let interior_a = s > 0. && s < 1.;
    let interior_b = t > 0. && t < 1.;
    if interior_a && interior_b {
        if let (Some(ta), Some(tb)) = (ta, tb) {
            let perpendicular = cross(ta, tb);
            if len(perpendicular) > 64. * f64::EPSILON {
                // Interior closest-point separation is perpendicular to both
                // segments. Subtracting rounded projected points can corrupt
                // its direction at tiny gaps or large world offsets; use their original tangents.
                let normal = unit(perpendicular);
                let side = normalized_contact_direction(delta)
                    .map_or(0., |direction| dot(direction, normal));
                if side > 0. {
                    return normal;
                }
                if side < 0. {
                    return mul(normal, -1.);
                }
                return coincident_strand_normal(a, ia, s, b, ib, t);
            }
        }
    }
    let mut direction = delta;
    for (interior, tangent) in [(interior_a, ta), (interior_b, tb)] {
        if interior {
            if let Some(tangent) = tangent {
                direction = cross(tangent, cross(direction, tangent));
            }
        }
    }
    normalized_contact_direction(direction)
        .unwrap_or_else(|| coincident_strand_normal(a, ia, s, b, ib, t))
}
fn coincident_strand_normal(a: &HairRod, ia: usize, s: f64, b: &HairRod, ib: usize, t: f64) -> V {
    let ta = normalized_contact_direction(sub(a.x[ia + 1], a.x[ia]));
    let tb = normalized_contact_direction(sub(b.x[ib + 1], b.x[ib]));
    let old_a = add(mul(a.old_x[ia], 1. - s), mul(a.old_x[ia + 1], s));
    let old_b = add(mul(b.old_x[ib], 1. - t), mul(b.old_x[ib + 1], t));
    let history = normalized_contact_direction(sub(old_a, old_b));
    if let (Some(ta), Some(tb)) = (ta, tb) {
        let cross_direction = cross(ta, tb);
        // Only a resolvable angular difference defines a unique cross normal.
        // This dimensionless roundoff threshold does not change contact gaps
        // or admission. Parallel cases use the covariant material frame.
        if len(cross_direction) > 64. * f64::EPSILON {
            let mut normal = unit(cross_direction);
            if history.is_some_and(|old| dot(old, normal) < 0.) {
                normal = mul(normal, -1.);
            }
            return normal;
        }
    }
    let tangent = ta.or(tb);
    let transverse = |direction: V| {
        if let Some(tangent) = tangent {
            cross(tangent, cross(direction, tangent))
        } else {
            direction
        }
    };
    if let Some(history) = history {
        let direction = transverse(history);
        if len(direction) > 64. * f64::EPSILON {
            if let Some(normal) = normalized_contact_direction(direction) {
                return normal;
            }
        }
    }
    for basis in [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]] {
        let direction = transverse(rotate(a.q[ia], basis));
        if len(direction) > 64. * f64::EPSILON {
            if let Some(normal) = normalized_contact_direction(direction) {
                return normal;
            }
        }
    }
    // Invalid/collapsed states have no reliable material direction. Keep a
    // finite transverse geometric direction; physical state admission still
    // owns rejection of the invalid pose, not this query helper.
    for basis in [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]] {
        if let Some(normal) = normalized_contact_direction(transverse(basis)) {
            return normal;
        }
    }
    [1., 0., 0.]
}
fn gather_strand_contacts(
    rods: &mut [HairRod],
    radius: f64,
    project_positions: bool,
) -> Vec<StrandResponse> {
    gather_strand_contacts_impl(rods, radius, project_positions, !project_positions)
}
#[cfg(test)]
pub(super) fn reference_strand_refresh(rods: &mut [HairRod], radius: f64) -> Vec<StrandResponse> {
    gather_strand_contacts_impl(rods, radius, false, false)
}
fn gather_strand_contacts_impl(
    rods: &mut [HairRod],
    radius: f64,
    project_positions: bool,
    filter_before_sort: bool,
) -> Vec<StrandResponse> {
    let mut responses = Vec::new();
    const CONTACT_TOLERANCE: f64 = 1e-10;
    let query_radius = radius + CONTACT_TOLERANCE * 0.5;
    for rod in rods.iter_mut() {
        rod.contacts
            .retain(|contact| matches!(contact.source, ContactSource::Mesh(_)));
    }
    // Segment AABBs, not just particles, enter the spatial hash. Adjacent segments are excluded.
    let cell = 0.008f64.max(radius * 4.);
    let total_segments: usize = rods.iter().map(|rod| rod.x.len().saturating_sub(1)).sum();
    let mut grid: ContactGrid = HashMap::with_capacity(total_segments.saturating_mul(2));
    let mut oversized = Vec::new();
    for (r, rod) in rods.iter().enumerate() {
        for (i, p) in rod.x.windows(2).enumerate() {
            let min: [i32; 3] = std::array::from_fn(|a| {
                ((p[0][a].min(p[1][a]) - query_radius) / cell).floor() as i32
            });
            let max: [i32; 3] = std::array::from_fn(|a| {
                ((p[0][a].max(p[1][a]) + query_radius) / cell).floor() as i32
            });
            // Keep hash insertion bounded without discarding physical pairs.
            // Oversized segments use the existing AABB hierarchy below.
            if (0..3).any(|a| i64::from(max[a]) - i64::from(min[a]) > 32) {
                oversized.push((r, i));
                continue;
            }
            for x in min[0]..=max[0] {
                for y in min[1]..=max[1] {
                    for z in min[2]..=max[2] {
                        grid.entry(cell_key([x, y, z]))
                            .or_default()
                            .push(((r, i), min));
                    }
                }
            }
        }
    }
    let query_separation = radius * 2. + CONTACT_TOLERANCE;
    // A refresh does not move geometry: discard disjoint capsules before
    // allocation/sorting, preserving exactly the same surviving pair order.
    // Sequential projection can move later pairs into range, so it retains
    // the original broad candidate list and checks live bounds below.
    let mut pairs = if !filter_before_sort {
        unique_cell_pairs(&grid)
    } else {
        unique_cell_pairs_filtered(&grid, |((ra, ia), (rb, ib))| {
            !(0..3).any(|axis| {
                let a0 = rods[ra].x[ia][axis];
                let a1 = rods[ra].x[ia + 1][axis];
                let b0 = rods[rb].x[ib][axis];
                let b1 = rods[rb].x[ib + 1][axis];
                a0.min(a1) - query_separation > b0.max(b1)
                    || b0.min(b1) - query_separation > a0.max(a1)
            })
        })
    };
    if !oversized.is_empty() {
        let segments: Vec<_> = rods
            .iter()
            .enumerate()
            .flat_map(|(r, rod)| (0..rod.x.len() - 1).map(move |i| (r, i)))
            .collect();
        let mut append = |a: SegmentId, b: SegmentId| {
            if a.0 == b.0 && a.1.abs_diff(b.1) <= 2 {
                return;
            }
            pairs.push(if a < b { (a, b) } else { (b, a) });
        };
        if project_positions {
            // Sequential projections can create later contacts. Preserve all
            // oversized candidates here, then check live capsule bounds in
            // the deterministic loop, rather than pruning stale geometry.
            for &a in &oversized {
                for &b in &segments {
                    append(a, b);
                }
            }
        } else {
            let bounds: Vec<_> = segments
                .iter()
                .map(|&(r, i)| {
                    let a = rods[r].x[i];
                    let b = rods[r].x[i + 1];
                    (
                        std::array::from_fn(|axis| a[axis].min(b[axis]) - query_radius),
                        std::array::from_fn(|axis| a[axis].max(b[axis]) + query_radius),
                    )
                })
                .collect();
            for (a, b) in continuous::bound_pairs(&bounds) {
                let a = segments[a];
                let b = segments[b];
                if oversized.binary_search(&a).is_ok() || oversized.binary_search(&b).is_ok() {
                    append(a, b);
                }
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
    }
    for ((ra, ia), (rb, ib)) in pairs {
        let separation = radius * 2.;
        let query_separation = separation + CONTACT_TOLERANCE;
        // Grid cells are broader than a fibre. Reject disjoint current capsule
        // bounds before the more expensive segment-distance calculation.
        if !filter_before_sort
            && (0..3).any(|axis| {
                let a0 = rods[ra].x[ia][axis];
                let a1 = rods[ra].x[ia + 1][axis];
                let b0 = rods[rb].x[ib][axis];
                let b1 = rods[rb].x[ib + 1][axis];
                a0.min(a1) - query_separation > b0.max(b1)
                    || b0.min(b1) - query_separation > a0.max(a1)
            })
        {
            continue;
        }
        let a = rods[ra].x[ia];
        let b = rods[ra].x[ia + 1];
        let c = rods[rb].x[ib];
        let d = rods[rb].x[ib + 1];
        let (s, t, p, q) = segment_pair(a, b, c, d);
        if len(sub(p, q)) > query_separation {
            continue;
        }
        // A parallel capsule pair has an interval of closest points. The
        // minimum alone cannot constrain relative velocity at its other end.
        // Endpoint-to-segment projections expose both interval endpoints and
        // preserve valid endpoint contacts for oblique finite segments too.
        let project_fraction = |point: V, origin: V, end: V| {
            let direction = sub(end, origin);
            let square = dot(direction, direction);
            if square == 0. {
                0.
            } else {
                (dot(sub(point, origin), direction) / square).clamp(0., 1.)
            }
        };
        let witnesses = [
            (s, t),
            (0., project_fraction(a, c, d)),
            (1., project_fraction(b, c, d)),
            (project_fraction(c, a, b), 0.),
            (project_fraction(d, a, b), 1.),
        ];
        for (witness, &(s, t)) in witnesses.iter().enumerate() {
            if witnesses[..witness].contains(&(s, t)) {
                continue;
            }
            // Legacy sequential projection changes the live endpoints. Recheck
            // each secondary witness against that updated geometry, so the same
            // initial penetration is not projected repeatedly.
            let (p, q) = if witness == 0 {
                (p, q)
            } else {
                (
                    add(
                        rods[ra].x[ia],
                        mul(sub(rods[ra].x[ia + 1], rods[ra].x[ia]), s),
                    ),
                    add(
                        rods[rb].x[ib],
                        mul(sub(rods[rb].x[ib + 1], rods[rb].x[ib]), t),
                    ),
                )
            };
            let delta = sub(p, q);
            let distance = len(delta);
            if distance > query_separation {
                continue;
            }
            // Shared/nearby follicles are allowed to overlap at the pinned boundary.
            if ia == 0 && ib == 0 && s < 0.2 && t < 0.2 {
                continue;
            }
            // Interior witnesses require a normal perpendicular to their
            // segment tangents even at ordinary gaps. Subtracting projected
            // world points adds tangential roundoff after large translations.
            // Endpoint/endpoint witnesses retain their separation direction.
            let normal = if (s == 0. || s == 1.) && (t == 0. || t == 1.) && distance > 1e-12 {
                mul(delta, 1. / distance)
            } else {
                feature_strand_normal(&rods[ra], ia, s, &rods[rb], ib, t, delta)
            };
            let wa = contact_weight(&rods[ra], ia, s);
            let wb = contact_weight(&rods[rb], ib, t);
            if wa + wb < 1e-30 {
                continue;
            }
            let depth = separation - distance;
            let impulse = if project_positions {
                pair_impulse(rods, (ra, ia, s), (rb, ib, t), depth)
            } else {
                0.
            };
            // Keep the physical separation target from before projection. The
            // displacement limiter controls this iteration, not the contact gap:
            // an incomplete projection must still enter the structural solve.
            responses.push(StrandResponse {
                a: (ra, ia, s),
                b: (rb, ib, t),
                normal,
                impulse,
            });
            let target_a = add(p, mul(normal, depth * wa / (wa + wb)));
            let target_b = add(q, mul(normal, -depth * wb / (wa + wb)));
            if wa > 0.
                && (!project_positions || project(&mut rods[ra], ia, s, normal, impulse * wa, None))
            {
                rods[ra].record_contact(
                    ia,
                    s,
                    normal,
                    target_a,
                    ContactSource::Strand {
                        other_rod: rb,
                        other_segment: ib,
                    },
                );
            }
            if wb > 0.
                && (!project_positions
                    || project(&mut rods[rb], ib, t, mul(normal, -1.), impulse * wb, None))
            {
                rods[rb].record_contact(
                    ib,
                    t,
                    mul(normal, -1.),
                    target_b,
                    ContactSource::Strand {
                        other_rod: ra,
                        other_segment: ia,
                    },
                );
            }
        }
    }
    responses
}

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
