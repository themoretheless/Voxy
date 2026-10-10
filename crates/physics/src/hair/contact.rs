use super::{HairRod, ContactSource, math::*};
use std::collections::HashMap;
#[path="contact_continuous.rs"]
mod continuous;
#[path="contact_triangle_sweep.rs"]
mod triangle_sweep;
#[path="contact_polynomial.rs"]
mod polynomial;
pub use triangle_sweep::{TriangleMotion,sweep_capsule_triangle};
pub(super) use triangle_sweep::trajectory_contact;
pub use continuous::{CapsuleMotion,CapsuleSweepOptions,CapsuleSweep,sweep_capsules,swept_capsule_pairs,swept_capsule_contacts};
#[path = "contact_features.rs"]
mod features;
#[path = "velocity_contacts.rs"]
mod velocity_contacts;
pub(super) use velocity_contacts::{admit_staged_strands,reconcile_elastic_contact_positions_with_solver,advance_swept_strands,reconcile_swept_contact_owners,strand_fraction,recover_friction_pressure,stabilize_contact_velocities,reconcile_contact_positions,stabilize_contact_velocities_with_solver,reconcile_contact_positions_with_solver};
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
    pub(super) fn replay_geometry(&self)->(Vec<V>,Vec<V>,Vec<V>,Vec<[usize;3]>,bool) {
        let n=self.triangles.iter().flat_map(|t|t.ids).max().unwrap()+1;
        let mut current=vec![[0.;3];n];let mut previous=current.clone();let mut velocity=current.clone();
        for t in &self.triangles {for i in 0..3 {current[t.ids[i]]=t.p[i];previous[t.ids[i]]=t.previous_p[i];velocity[t.ids[i]]=t.velocity[i];}}
        (current,previous,velocity,self.triangles.iter().map(|t|t.ids).collect(),self.feature_normals.is_some())
    }
    pub(super) fn restore_replay_motion(&mut self,previous:&[V],velocity:&[V]) {
        self.motion_bounds.take();
        for t in &mut self.triangles {t.previous_p=t.ids.map(|i|previous[i]);t.velocity=t.ids.map(|i|velocity[i]);}
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
                velocity: [[0.;3];3],
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
        let triangle=&self.triangles[face];
        match &self.feature_normals {
            Some(normals) => normals.normal(face,closest_triangle_feature(point,triangle).1,triangle.normal),
            None => triangle.normal,
        }
    }
    /// Returns signed distance and closest-feature normal after explicit enable.
    /// Positive distance is outside an outward-oriented closed surface.
    pub fn signed_distance_closed(&self, point: V) -> Result<(f64, V), &'static str> {
        if !finite(point) { return Err("invalid surface query"); }
        self.feature_normals.as_ref().ok_or("closed feature normals are not enabled")?;
        let mut best = (f64::INFINITY, 0, [0.;3]);
        self.nearest(point, 0, &mut best);
        let normal = self.contact_normal(best.1, point);
        let distance = best.0.sqrt();
        Ok((if dot(sub(point,best.2),normal)<0. {-distance} else {distance}, normal))
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
                if distance < best.0 || (distance==best.0 && self.triangles[i].ids<self.triangles[best.1].ids) {
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
        if self.feature_normals.as_ref().is_some_and(|normals| !normals.validate_refit(&self.triangles, vertices)) {
            return Err("animated collider splits welded topology or degenerates a face");
        }
        self.motion_bounds.take();
        for t in &mut self.triangles {
            t.previous_p = t.p;
            t.velocity = [[0.;3];3];
            t.p = t.ids.map(|i| vertices[i]);
            t.normal = unit(cross(sub(t.p[1], t.p[0]), sub(t.p[2], t.p[0])));
            t.min = std::array::from_fn(|a| t.p.iter().map(|p| p[a]).fold(f64::INFINITY, f64::min));
            t.max =
                std::array::from_fn(|a| t.p.iter().map(|p| p[a]).fold(f64::NEG_INFINITY, f64::max));
        }
        if let Some(normals) = &mut self.feature_normals { normals.refresh(&self.triangles); }
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
        if !dt.is_finite() || dt <= 0. || self.triangles.iter().any(|triangle| {
            triangle.ids.iter().zip(triangle.p).any(|(&id,previous)| {
                vertices.get(id).is_none_or(|position| !finite(mul(sub(*position,previous),1./dt)))
            })
        }) {return Err("invalid animated collider motion");}
        self.refit(vertices)?;
        for triangle in &mut self.triangles {
            triangle.velocity=std::array::from_fn(|i|mul(sub(triangle.p[i],triangle.previous_p[i]),1./dt));
        }
        self.motion_dt = Some(dt);
        Ok(())
    }
    pub(super) fn motion_duration(&self) -> Option<f64> {self.motion_dt}
    /// Immutable collider motion restricted to an authoritative time interval.
    /// Preserves material velocities and gives continuous queries matching
    /// start/end vertices instead of the full frame's previous pose.
    pub fn motion_interval(&self, start:f64, end:f64)->Result<Self, &'static str> {
        if !start.is_finite() || !end.is_finite() || start<0. || end>1. || start>=end {
            return Err("invalid collider motion interval");
        }
        let mut sample=self.clone();
        sample.motion_bounds.take();
        if let Some(duration)=self.motion_dt {
            sample.sample_motion(self,end)?;
            for (sample,source) in sample.triangles.iter_mut().zip(&self.triangles) {
                let positions=std::array::from_fn(|i|add(mul(source.previous_p[i],1.-start),mul(source.p[i],start)));
                if positions.iter().any(|p|!finite(*p)) || len(cross(sub(positions[1],positions[0]),sub(positions[2],positions[0])))<1e-14 {
                    return Err("collider motion degenerates an intermediate face");
                }
                sample.previous_p=positions;
            }
            let h=duration*(end-start);
            if !h.is_finite() || h<=0. {return Err("invalid collider motion interval duration");}
            sample.motion_dt=Some(h);
        } else {
            for face in &mut sample.triangles {face.previous_p=face.p;face.velocity=[[0.;3];3];}
        }
        Ok(sample)
    }
    // Always sample authoritative endpoints, never the preceding sampled pose.
    pub(super) fn sample_motion(&mut self, source: &Self, fraction: f64) -> Result<(), &'static str> {
        if source.motion_dt.is_none() {return Ok(());}
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {return Err("invalid collider motion fraction");}
        let count=source.triangles.iter().flat_map(|t|t.ids).max().unwrap()+1;
        let mut vertices=vec![[0.;3];count];
        for triangle in &source.triangles {
            let positions=std::array::from_fn::<_,3,_>(|i|add(mul(triangle.previous_p[i],1.-fraction),mul(triangle.p[i],fraction)));
            if positions.iter().any(|p|!finite(*p)) || len(cross(sub(positions[1],positions[0]),sub(positions[2],positions[0])))<1e-14 {
                return Err("collider motion degenerates an intermediate face");
            }
            for (id,position) in triangle.ids.into_iter().zip(positions) {vertices[id]=position;}
        }
        self.refit(&vertices)?;
        for (sample,original) in self.triangles.iter_mut().zip(&source.triangles) {
            sample.previous_p=original.previous_p;
            sample.velocity=original.velocity;
        }
        self.motion_dt=source.motion_dt;
        Ok(())
    }
    fn surface_velocity(&self, face: usize, point: V) -> V {
        if self.motion_dt.is_none() {return [0.;3];}
        let triangle=&self.triangles[face];
        let (point,feature)=closest_triangle_feature(point,triangle);
        let weights=match feature {
            ClosestFeature::Vertex(i)=>std::array::from_fn(|j|if i==j {1.} else {0.}),
            ClosestFeature::Edge(a,b)=>{
                let edge=sub(triangle.p[b],triangle.p[a]);
                let t=(dot(sub(point,triangle.p[a]),edge)/dot(edge,edge)).clamp(0.,1.);
                std::array::from_fn(|i|if i==a {1.-t} else if i==b {t} else {0.})
            }
            ClosestFeature::Face=>{
                let ab=sub(triangle.p[1],triangle.p[0]);let ac=sub(triangle.p[2],triangle.p[0]);
                let relative=sub(point,triangle.p[0]);let area=len(cross(ab,ac));
                let b=dot(cross(relative,ac),triangle.normal)/area;
                let c=dot(cross(ab,relative),triangle.normal)/area;
                [1.-b-c,b,c]
            }
        };
        std::array::from_fn(|axis|(0..3).map(|i|weights[i]*triangle.velocity[i][axis]).sum())
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
            t.velocity = t.velocity.map(|v|std::array::from_fn(|a|dot([m[a][0],m[a][1],m[a][2]],v)));
            t.normal = unit(cross(sub(t.p[1], t.p[0]), sub(t.p[2], t.p[0])));
            t.min = std::array::from_fn(|a| t.p.iter().map(|p| p[a]).fold(f64::INFINITY, f64::min));
            t.max =
                std::array::from_fn(|a| t.p.iter().map(|p| p[a]).fold(f64::NEG_INFINITY, f64::max));
        }
        if let Some(normals) = &mut result.feature_normals { normals.refresh(&result.triangles); }
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
        return (add(a, mul(ab, d1 / (d1 - d3))), ClosestFeature::Edge(0,1));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0. && d5 <= d6 {
        return (c, ClosestFeature::Vertex(2));
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0. && d2 >= 0. && d6 <= 0. {
        return (add(a, mul(ac, d2 / (d2 - d6))), ClosestFeature::Edge(0,2));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0. && (d4 - d3) >= 0. && (d5 - d6) >= 0. {
        return (add(b, mul(sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6)))), ClosestFeature::Edge(1,2));
    }
    let denom = 1. / (va + vb + vc);
    (add(a, add(mul(ab, vb * denom), mul(ac, vc * denom))), ClosestFeature::Face)
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
            let (q,feature) = closest_triangle_feature(p, tri);
            if matches!(feature,ClosestFeature::Face) {
                // This is an interior plane/segment intersection, hence zero
                // separation. Reprojecting the same intersection introduces
                // ULP noise which makes flat trajectory minima pick unrelated
                // times and segment weights. Edge/vertex near misses retain
                // their actual distinct closest points below.
                return (s,p,p);
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
fn projected_face_interval(a:V,b:V,tri:&Triangle)->Option<(f64,f64)> {
    let mut lo=0f64;let mut hi=1f64;
    for edge in 0..3 {
        let origin=tri.p[edge];let direction=sub(tri.p[(edge+1)%3],origin);
        let left=dot(cross(direction,sub(a,origin)),tri.normal);
        let right=dot(cross(direction,sub(b,origin)),tri.normal);
        if left<0. && right<0. {return None;}
        if left<0. {lo=lo.max(left/(left-right));}
        if right<0. {hi=hi.min(left/(left-right));}
        if lo>hi {return None;}
    }
    Some((lo,hi))
}
fn project(rod: &mut HairRod, i: usize, t: f64, normal: V, penetration: f64, surface_velocity: Option<V>) -> bool {
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
    if let Some(velocity)=surface_velocity {
        rod.record_surface_response(i,correction_a,velocity);
        rod.record_surface_response(i+1,correction_b,velocity);
    }
    true
}
fn contact_weight(rod: &HairRod, i: usize, t: f64) -> f64 {
    if i == 0 && t < 0.15 { return 0.; }
    rod.inv_mass[i] * (1.-t).powi(2) + rod.inv_mass[i+1] * t.powi(2)
}
fn pair_impulse(rods: &[HairRod], a: (usize, usize, f64), b: (usize, usize, f64), depth: f64) -> f64 {
    let weight = contact_weight(&rods[a.0],a.1,a.2) + contact_weight(&rods[b.0],b.1,b.2);
    if weight < 1e-30 { return 0.; }
    let mut impulse = depth.max(0.) / weight;
    // One shared multiplier preserves equal/opposite generalized impulses.
    // Independently clamping either strand changes their momentum balance.
    for (rod, segment, fraction) in [a,b] {
        let rod = &rods[rod];
        if contact_weight(rod,segment,fraction) == 0. { continue; }
        let maximum = (rod.inv_mass[segment]*(1.-fraction)).max(rod.inv_mass[segment+1]*fraction);
        impulse = impulse.min(rod.lengths[segment]*0.2/maximum);
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
    finish_strand_contacts_with_diagnostics(rods,responses,dt,0,None);
}
pub(super) fn finish_strand_contacts_with_diagnostics(rods:&mut [HairRod],responses:&[StrandResponse],dt:f64,substep:usize,mut diagnostics:Option<&mut Vec<super::HairFrictionDiagnostic>>) {
    for response in responses {
        let (ra,ia,s)=response.a; let (rb,ib,t)=response.b;
        let wa=contact_weight(&rods[ra],ia,s);let wb=contact_weight(&rods[rb],ib,t);
        if wa+wb<1e-30 {continue;}
        let va=add(mul(rods[ra].velocity[ia],1.-s),mul(rods[ra].velocity[ia+1],s));
        let vb=add(mul(rods[rb].velocity[ib],1.-t),mul(rods[rb].velocity[ib+1],t));
        let relative=sub(va,vb);let normal_speed=dot(relative,response.normal);
        let normal_impulse=(-normal_speed/(wa+wb)).max(0.);
        let tangent=sub(relative,mul(response.normal,normal_speed));
        let tangent_speed=len(tangent);
        let friction=(rods[ra].material.friction*rods[rb].material.friction).sqrt();
        let tangent_impulse=(friction*(response.impulse/dt+normal_impulse)).min(tangent_speed/(wa+wb));
        let impulse=sub(mul(response.normal,normal_impulse),mul(tangent,tangent_impulse/tangent_speed.max(1e-30)));
        if let Some(diagnostics)=diagnostics.as_mut() {diagnostics.push(super::HairFrictionDiagnostic {substep,a:response.a,b:response.b,normal:response.normal,position_impulse:response.impulse,relative_velocity:relative,normal_speed,tangent_speed,mobility_a:wa,mobility_b:wb,friction,normal_impulse,tangent_impulse,applied_impulse:impulse});}

        for (rod,segment,fraction,sign) in [(ra,ia,s,1.),(rb,ib,t,-1.)] {
            if contact_weight(&rods[rod],segment,fraction)==0. {continue;}
            for (point,weight) in [(segment,1.-fraction),(segment+1,fraction)] {
                let change=mul(impulse,sign*weight*rods[rod].inv_mass[point]);
                rods[rod].velocity[point]=add(rods[rod].velocity[point],change);
            }
        }
    }
}
// A segment can intersect several faces at numerically zero distance. Choose
// its first intersection, with a topology key for coincident shared features,
// rather than allowing roundoff in the distance to select a distant face.
fn segment_contact_precedes(distance:f64,t:f64,key:[usize;3],old_distance:f64,old_t:f64,old_key:[usize;3],length:f64)->bool {
    const ZERO_DISTANCE:f64=1e-10;
    if distance<=ZERO_DISTANCE && old_distance<=ZERO_DISTANCE {
        if (t-old_t).abs()*length<=ZERO_DISTANCE {key<old_key} else {t<old_t}
    } else {distance<old_distance || (distance==old_distance && key<old_key)}
}
// Feature pseudonormals classify the distance sign. Away from zero distance,
// the metric gradient follows the closest-point displacement, also from inside.
fn distance_gradient(delta: V, distance: f64, signed: f64, fallback: V, epsilon: f64) -> V {
    if distance<epsilon {fallback} else {mul(delta,signed.signum()/distance)}
}
// A face-interior metric gradient is exactly its outward geometric normal.
// Reconstructing it from two rounded world points adds tangential force,
// especially when the surface gap is tiny or the world origin is distant.
// Edges/vertices retain the signed metric displacement (not a face normal).
fn triangle_distance_gradient(triangle:&Triangle,point:V,closest:V,signed:f64,fallback:V,epsilon:f64)->V {
    if signed.abs()>=epsilon && matches!(closest_triangle_feature(point,triangle).1,ClosestFeature::Face) {
        triangle.normal
    } else {distance_gradient(sub(point,closest),signed.abs(),signed,fallback,epsilon)}
}
pub(super) fn mesh_contacts(rod: &mut HairRod, meshes: &[TriangleMesh], radius: f64) {
    gather_mesh_contacts(rod,meshes,radius,true);
}
// Approximate the deepest signed-distance witness in one interior interval.
// Coarse samples bracket candidate minima; bounded refinement retains every
// sampled candidate, including the old midpoint. This is constraint discovery,
// not a global segment-clearance certificate for arbitrary nonconvex meshes.
fn deepest_interval_witness(mesh:&TriangleMesh,a:V,b:V,lo:f64,hi:f64)->Result<(f64,f64,V), &'static str> {
    let evaluate=|t:f64|mesh.signed_distance_closed(add(mul(a,1.-t),mul(b,t))).map(|(d,n)|(t,d,n));
    let samples=(0..=8).map(|i|evaluate(lo+(hi-lo)*i as f64/8.)).collect::<Result<Vec<_>,_>>()?;
    let segment_length=len(sub(b,a));
    let mut best=samples[4];
    for value in &samples {if value.1<best.1 {best=*value;}}
    for i in 0..=8 {
        // A deepest endpoint sample can hide an even deeper medial peak in
        // the last sampling cell. Refine those boundary cells as well as
        // interior brackets; retain the endpoint itself in `best`.
        let (start,end)=if i==0 {
            if samples[0].1>samples[1].1 {continue;}(0,1)
        } else if i==8 {
            if samples[8].1>samples[7].1 {continue;}(7,8)
        } else {
            if samples[i].1>samples[i-1].1 || samples[i].1>samples[i+1].1 {continue;}(i-1,i+1)
        };
        let mut left=samples[start].0;let mut right=samples[end].0;
        let ratio=(5f64.sqrt()-1.)*0.5;
        let mut x=evaluate(right-ratio*(right-left))?;
        let mut y=evaluate(left+ratio*(right-left))?;
        for _ in 0..80 {
            for value in [x,y] {if value.1<best.1 {best=value;}}
            // Discovery precision must be finer than the 1e-10 m contact
            // admission gate, including long/scaled segments. Keep a bounded
            // search, but stop by physical interval width rather than count.
            if (right-left)*segment_length<=1e-12 {break;}
            if x.1<=y.1 {right=y.0;y=x;x=evaluate(right-ratio*(right-left))?;}
            else {left=x.0;x=y;y=evaluate(left+ratio*(right-left))?;}
        }
    }
    let position=add(mul(a,1.-best.0),mul(b,best.0));
    let mut nearest=(f64::INFINITY,0,[0.;3]);
    mesh.nearest(position,0,&mut nearest);
    // Feature pseudonormals determine sign, not the metric derivative at
    // concave edges. Use the actual closest-point distance gradient.
    best.2=triangle_distance_gradient(&mesh.triangles[nearest.1],position,nearest.2,best.1,best.2,1e-10);
    Ok(best)
}
// At a medial minimum of signed distance along a segment, combine the
// two observed one-sided metric gradients so their segment derivative is zero.
// Return unit force direction AND its original gradient magnitude separately.
fn interior_envelope_gradient(mesh:&TriangleMesh,a:V,b:V,t:f64,lo:f64,hi:f64,normal:V,velocity:V)->(V,f64,V) {
    let delta=(hi-lo)*1e-6;
    if t-delta<=lo || t+delta>=hi {return (normal,1.,velocity);}
    let probe=|fraction:f64|->Option<(V,V)> {
        let p=add(mul(a,1.-fraction),mul(b,fraction));
        let (signed,pseudo)=mesh.signed_distance_closed(p).ok()?;
        let mut nearest=(f64::INFINITY,0,[0.;3]);mesh.nearest(p,0,&mut nearest);
        Some((triangle_distance_gradient(&mesh.triangles[nearest.1],p,nearest.2,signed,pseudo,1e-10),mesh.surface_velocity(nearest.1,nearest.2)))
    };
    let (Some((left,vl)),Some((right,vr)))=(probe(t-delta),probe(t+delta)) else {return (normal,1.,velocity);};
    let segment=sub(b,a);let dl=dot(left,segment);let dr=dot(right,segment);
    if !(dl<0. && dr>0.) {return (normal,1.,velocity);}
    let w=dr/(dr-dl);
    let g=add(mul(left,w),mul(right,1.-w));let scale=len(g);
    // A zero generalized gradient needs a nonlocal escape, not division by zero.
    if !scale.is_finite() || scale<1e-12 || scale>1.+1e-12 {return (normal,1.,velocity);}
    let direction=mul(g,1./scale);
    let blended=add(mul(vl,w),mul(vr,1.-w));
    let normal_speed=(w*dot(left,vl)+(1.-w)*dot(right,vr))/scale;
    let velocity=add(blended,mul(direction,normal_speed-dot(direction,blended)));
    (direction,scale,velocity)
}
pub(super) fn refresh_mesh_constraints(rod: &mut HairRod, meshes: &[TriangleMesh], radius: f64) {
    gather_mesh_contacts(rod,meshes,radius,false);
}
fn gather_mesh_contacts(rod: &mut HairRod, meshes: &[TriangleMesh], radius: f64, project_positions: bool) {
    const CONTACT_TOLERANCE:f64=1e-10;
    let query_radius=radius+CONTACT_TOLERANCE;
    rod.contacts.retain(|contact| !matches!(contact.source,ContactSource::Mesh(_)));
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
                let normal=triangle_distance_gradient(&mesh.triangles[best.1],rod.x[i],best.2,signed,contact_normal,1e-12);
                let depth=radius-signed;
                let target=add(rod.x[i],mul(normal,depth));
                let contact=rod.record_point_contact(i,normal,target,ContactSource::Mesh(mesh_index));
                rod.contacts[contact].surface_velocity=mesh.surface_velocity(best.1,best.2);
                let correction=mul(normal,depth.max(0.).min(rod.lengths[i-1]*0.2));
                if project_positions {
                    rod.x[i]=add(rod.x[i],correction);
                    rod.record_surface_response(i,correction,mesh.surface_velocity(best.1,best.2));
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
            let mut contact: Option<(f64, f64, V, f64, [usize;3], V)> = None;
            let mut crossings = Vec::new();
            for &id in &candidates {
                let tri = &mesh.triangles[id];
                let (t, p, q) = segment_triangle(a, b, tri);
                // Follicle overlap is prescribed. Do not let an allowed root
                // contact hide a later collision on the free part of the strand.
                if i==0 && t<0.15 {continue;}
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
                let normal = triangle_distance_gradient(tri,p,q,signed,contact_normal,1e-10);
                if contact.as_ref().is_none_or(|c| segment_contact_precedes(distance,t,tri.ids,c.0,c.1,c.4,len(sub(b,a)))) {
                    contact = Some((distance,t,normal,signed,tri.ids,mesh.surface_velocity(id,q)));
                }
            }
            // A segment with exterior endpoints can traverse a closed body.
            // Its boundary intersection has zero distance, hiding the actual
            // penetration. Add interior witnesses between surface crossings;
            // these discover constraints, never certify continuous clearance.
            // A segment entirely inside the volume has no boundary crossings.
            // It still needs an interior depth witness; endpoint constraints
            // alone can miss a deeper penetration in the middle.
            let free_start=if i==0 {0.15} else {0.};
            let inside_endpoint=mesh.feature_normals.is_some() && [free_start,1.].into_iter().any(|t| {
                let p=add(mul(a,1.-t),mul(b,t));
                let node=if t==0. {Some(i)} else if t==1. {Some(i+1)} else {None};
                if let Some(bound)=node.and_then(|point|rod.clearance_cache[mesh_index][point]) {
                    if bound.excludes(&[p],query_radius) {return false;}
                    // Reuse the exact signed endpoint query above only at the
                    // same position. Moved/projected points must be re-queried.
                    if bound.position==p {return bound.distance < -CONTACT_TOLERANCE;}
                }
                mesh.signed_distance_closed(p).is_ok_and(|(distance,_)|distance < -CONTACT_TOLERANCE)
            });
            if !crossings.is_empty() || inside_endpoint {
                crossings.extend([if i == 0 {0.15} else {0.0}, 1.0]);
                crossings.retain(|t| *t >= if i == 0 {0.15} else {0.0});
                crossings.sort_by(f64::total_cmp);
                crossings.dedup_by(|a,b| (*a-*b).abs() < 1e-12);
                for interval in crossings.windows(2) {
                    if let Ok((t,signed,normal)) = deepest_interval_witness(mesh,a,b,interval[0],interval[1]) {
                        let position = add(mul(a,1.-t),mul(b,t));
                        if signed >= -CONTACT_TOLERANCE {continue;}
                        let mut nearest = (f64::INFINITY,0,[0.;3]);
                        mesh.nearest(position,0,&mut nearest);
                        let velocity = mesh.surface_velocity(nearest.1,nearest.2);
                        let (normal,scale,velocity)=interior_envelope_gradient(mesh,a,b,t,interval[0],interval[1],normal,velocity);
                        let depth = (radius-signed)/scale;
                        let index = rod.record_contact(i,t,normal,add(position,mul(normal,depth)),ContactSource::Mesh(mesh_index));
                        rod.contacts[index].metric_scale=scale;
                        rod.contacts[index].surface_velocity = velocity;
                        if project_positions {project(rod,i,t,normal,depth,Some(velocity));}
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
                let tri=&mesh.triangles[id];
                if dot(sub(b,a),tri.normal).abs()<=CONTACT_TOLERANCE {
                    if let Some((lo,hi))=projected_face_interval(a,b,tri) {
                        for t in [lo.max(free_start),hi] {
                            if t<free_start || t>hi {continue;}
                            let position=add(mul(a,1.-t),mul(b,t));
                            let q=closest_triangle(position,tri);
                            let delta=sub(position,q);let distance=len(delta);
                            if distance>query_radius {continue;}
                            let mut nearest=(f64::INFINITY,0,[0.;3]);
                            mesh.nearest(position,0,&mut nearest);
                            if nearest.1!=id {continue;}
                            let pseudo=mesh.contact_normal(id,position);
                            let signed=if dot(delta,pseudo)<0. {-distance} else {distance};
                            let normal=triangle_distance_gradient(tri,position,q,signed,pseudo,1e-12);
                            let index=rod.record_contact(i,t,normal,add(position,mul(normal,radius-signed)),ContactSource::Mesh(mesh_index));
                            rod.contacts[index].surface_velocity=mesh.surface_velocity(id,q);
                        }
                    }
                }
            }
            }
            if let Some((_,t,normal,signed,_,velocity))=contact {
                let depth=radius-signed;
                let position=add(mul(a,1.-t),mul(b,t));
                let contact=rod.record_contact(i,t,normal,add(position,mul(normal,depth)),ContactSource::Mesh(mesh_index));
                rod.contacts[contact].surface_velocity=velocity;
                if depth>0. && project_positions {project(rod,i,t,normal,depth,Some(velocity));}
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

// Preserve lexicographic pair order while sorting compact integer keys.
fn sort_contact_pairs(pairs: &mut [(SegmentId, SegmentId)]) {
    if pairs.len() < 128 { pairs.sort_unstable(); return; }
    let (mut rod_max, mut segment_max) = (0usize, 0usize);
    for &(a,b) in pairs.iter() {
        rod_max = rod_max.max(a.0).max(b.0);
        segment_max = segment_max.max(a.1).max(b.1);
    }
    let rod_bits = (usize::BITS - rod_max.leading_zeros()).max(1);
    let segment_bits = (usize::BITS - segment_max.leading_zeros()).max(1);
    if 2*(rod_bits+segment_bits) > 64 { pairs.sort_unstable(); return; }
    let mut keys: Vec<u64> = pairs.iter().map(|&(a,b)| {
        (((((a.0 as u64) << segment_bits) | a.1 as u64) << rod_bits | b.0 as u64) << segment_bits) | b.1 as u64
    }).collect();
    keys.sort_unstable();
    let rod_mask = (1u64 << rod_bits)-1;
    let segment_mask = (1u64 << segment_bits)-1;
    for (pair, mut key) in pairs.iter_mut().zip(keys) {
        let ib = (key & segment_mask) as usize; key >>= segment_bits;
        let rb = (key & rod_mask) as usize; key >>= rod_bits;
        let ia = (key & segment_mask) as usize; key >>= segment_bits;
        *pair = ((key as usize, ia), (rb, ib));
    }
}

fn unique_cell_pairs(grid: &ContactGrid) -> Vec<(SegmentId, SegmentId)> {
    unique_cell_pairs_filtered(grid,|_|true)
}
fn unique_cell_pairs_filtered(grid: &ContactGrid, mut keep:impl FnMut((SegmentId,SegmentId))->bool) -> Vec<(SegmentId, SegmentId)> {
    let mut pairs = Vec::with_capacity(grid.len().saturating_mul(2));
    for (&key, entries) in grid {
        for a in 0..entries.len() {
            for b in a + 1..entries.len() {
                let owner = std::array::from_fn(|axis| entries[a].1[axis].max(entries[b].1[axis]));
                if cell_key(owner) != key { continue; }
                let mut pair = (entries[a].0, entries[b].0);
                if pair.0 > pair.1 { pair = (pair.1, pair.0); }
                if pair.0.0 == pair.1.0 && pair.0.1.abs_diff(pair.1.1) <= 2 { continue; }
                if !keep(pair) {continue;}
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
        let cell = [(key.0 >> 32) as u32 as i32, key.0 as u32 as i32, key.1 as i32];
        for a in 0..entries.len() {
            for b in a + 1..entries.len() {
                let mut pair = (entries[a].0, entries[b].0);
                if pair.0.0 == pair.1.0 && pair.0.1.abs_diff(pair.1.1) <= 2 { continue; }
                // Reject duplicate cells axis by axis before packing a new key.
                // The first shared integer cell remains the sole pair owner.
                if (0..3).any(|axis| entries[a].1[axis].max(entries[b].1[axis]) != cell[axis]) { continue; }
                if pair.0 > pair.1 { pair = (pair.1, pair.0); }
                pairs.push(pair);
            }
        }
    }
    pairs.sort_unstable();
    pairs
}

pub(super) fn self_contacts(rods: &mut [HairRod], radius: f64) -> Vec<StrandResponse> {
    gather_strand_contacts(rods,radius,true)
}
/// Query witnesses must be refreshed before use. A pair owns one clearance
/// budget; splitting its positional targets does not split that budget.
pub(super) fn strand_geometry_admitted(rods:&[HairRod],responses:&[StrandResponse],radius:f64)->Result<bool, &'static str> {
    for response in responses {
        let (ra,ia,s)=response.a;let (rb,ib,t)=response.b;
        let a=add(mul(rods[ra].x[ia],1.-s),mul(rods[ra].x[ia+1],s));
        let b=add(mul(rods[rb].x[ib],1.-t),mul(rods[rb].x[ib+1],t));
        let gap=len(sub(a,b))-2.*radius;
        if !gap.is_finite() {return Err("joint strand contact geometry overflow");}
        if gap < -1e-10 {return Ok(false);}
    }
    Ok(true)
}
/// Re-query the current geometry without moving it. Historical projections supply
/// friction load only; old normals and released pairs must not constrain velocity.
pub(super) fn refresh_strand_responses(rods:&mut [HairRod],radius:f64,history:&[StrandResponse])->Vec<StrandResponse> {
    let mut current=gather_strand_contacts(rods,radius,false);
    if history.is_empty() {return current;}
    let mut witnesses:std::collections::BTreeMap<((usize,usize),(usize,usize)),Vec<usize>>=std::collections::BTreeMap::new();
    for (index,response) in current.iter().enumerate() {
        witnesses.entry(((response.a.0,response.a.1),(response.b.0,response.b.1))).or_default().push(index);
    }
    // A historical projection supplies one pressure budget, not a new copy
    // for every point of the current manifold. Preserve unchanged witnesses
    // exactly; transfer a moved/released witness to the closest current point
    // of the same segment pair. This is a friction history hint; the coupled
    // pressure recovery solve owns newly solved physical contact reactions.
    for old in history {
        let Some(indices)=witnesses.get(&((old.a.0,old.a.1),(old.b.0,old.b.1))) else {continue;};
        let a=sub(rods[old.a.0].x[old.a.1+1],rods[old.a.0].x[old.a.1]);
        let b=sub(rods[old.b.0].x[old.b.1+1],rods[old.b.0].x[old.b.1]);
        let metric=|index:usize| {
            let new=&current[index];
            (new.a.2-old.a.2).powi(2)*dot(a,a)+(new.b.2-old.b.2).powi(2)*dot(b,b)
        };
        if let Some(&index)=indices.iter().min_by(|&&a,&&b|metric(a).total_cmp(&metric(b))) {
            current[index].impulse+=dot(old.normal,current[index].normal).max(0.)*old.impulse;
        }
    }
    current
}
// Scale before normalizing: squaring a nonzero separation can underflow,
// but that must not erase its geometric direction.
fn normalized_contact_direction(vector:V)->Option<V> {
    if !finite(vector) {return None;}
    let scale=vector.into_iter().map(f64::abs).fold(0f64,f64::max);
    if scale==0. {return None;}
    Some(unit(vector.map(|value|value/scale)))
}
fn feature_strand_normal(a:&HairRod,ia:usize,s:f64,b:&HairRod,ib:usize,t:f64,delta:V)->V {
    let ta=normalized_contact_direction(sub(a.x[ia+1],a.x[ia]));
    let tb=normalized_contact_direction(sub(b.x[ib+1],b.x[ib]));
    let interior_a=s>0. && s<1.;let interior_b=t>0. && t<1.;
    if interior_a && interior_b {
        if let (Some(ta),Some(tb))=(ta,tb) {
            let perpendicular=cross(ta,tb);
            if len(perpendicular)>64.*f64::EPSILON {
                // Interior closest-point separation is perpendicular to both
                // segments. Subtracting rounded projected points can corrupt
                // its direction at tiny gaps or large world offsets; use their original tangents.
                let normal=unit(perpendicular);
                let side=normalized_contact_direction(delta).map_or(0.,|direction|dot(direction,normal));
                if side>0. {return normal;}
                if side<0. {return mul(normal,-1.);}
                return coincident_strand_normal(a,ia,s,b,ib,t);
            }
        }
    }
    let mut direction=delta;
    for (interior,tangent) in [(interior_a,ta),(interior_b,tb)] {
        if interior {
            if let Some(tangent)=tangent {direction=cross(tangent,cross(direction,tangent));}
        }
    }
    normalized_contact_direction(direction).unwrap_or_else(||coincident_strand_normal(a,ia,s,b,ib,t))
}
fn coincident_strand_normal(a:&HairRod,ia:usize,s:f64,b:&HairRod,ib:usize,t:f64)->V {
    let ta=normalized_contact_direction(sub(a.x[ia+1],a.x[ia]));
    let tb=normalized_contact_direction(sub(b.x[ib+1],b.x[ib]));
    let old_a=add(mul(a.old_x[ia],1.-s),mul(a.old_x[ia+1],s));
    let old_b=add(mul(b.old_x[ib],1.-t),mul(b.old_x[ib+1],t));
    let history=normalized_contact_direction(sub(old_a,old_b));
    if let (Some(ta),Some(tb))=(ta,tb) {
        let cross_direction=cross(ta,tb);
        // Only a resolvable angular difference defines a unique cross normal.
        // This dimensionless roundoff threshold does not change contact gaps
        // or admission. Parallel cases use the covariant material frame.
        if len(cross_direction)>64.*f64::EPSILON {
            let mut normal=unit(cross_direction);
            if history.is_some_and(|old|dot(old,normal)<0.) {normal=mul(normal,-1.);}
            return normal;
        }
    }
    let tangent=ta.or(tb);
    let transverse=|direction:V| {
        if let Some(tangent)=tangent {cross(tangent,cross(direction,tangent))} else {direction}
    };
    if let Some(history)=history {
        let direction=transverse(history);
        if len(direction)>64.*f64::EPSILON {
            if let Some(normal)=normalized_contact_direction(direction) {return normal;}
        }
    }
    for basis in [[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]] {
        let direction=transverse(rotate(a.q[ia],basis));
        if len(direction)>64.*f64::EPSILON {
            if let Some(normal)=normalized_contact_direction(direction) {return normal;}
        }
    }
    // Invalid/collapsed states have no reliable material direction. Keep a
    // finite transverse geometric direction; physical state admission still
    // owns rejection of the invalid pose, not this query helper.
    for basis in [[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]] {
        if let Some(normal)=normalized_contact_direction(transverse(basis)) {return normal;}
    }
    [1.,0.,0.]
}
fn gather_strand_contacts(rods: &mut [HairRod], radius: f64, project_positions:bool) -> Vec<StrandResponse> {
    gather_strand_contacts_impl(rods,radius,project_positions,!project_positions)
}
#[cfg(test)]
pub(super) fn reference_strand_refresh(rods:&mut [HairRod],radius:f64)->Vec<StrandResponse> {
    gather_strand_contacts_impl(rods,radius,false,false)
}
fn gather_strand_contacts_impl(rods: &mut [HairRod], radius: f64, project_positions:bool,filter_before_sort:bool) -> Vec<StrandResponse> {
    let mut responses=Vec::new();
    const CONTACT_TOLERANCE: f64 = 1e-10;
    let query_radius = radius + CONTACT_TOLERANCE * 0.5;
    for rod in rods.iter_mut() {rod.contacts.retain(|contact| matches!(contact.source,ContactSource::Mesh(_)));}
    // Segment AABBs, not just particles, enter the spatial hash. Adjacent segments are excluded.
    let cell = 0.008f64.max(radius * 4.);
    let total_segments: usize = rods.iter().map(|rod| rod.x.len().saturating_sub(1)).sum();
    let mut grid: ContactGrid = HashMap::with_capacity(total_segments.saturating_mul(2));
    let mut oversized=Vec::new();
    for (r, rod) in rods.iter().enumerate() {
        for (i, p) in rod.x.windows(2).enumerate() {
            let min: [i32; 3] =
                std::array::from_fn(|a| ((p[0][a].min(p[1][a]) - query_radius) / cell).floor() as i32);
            let max: [i32; 3] =
                std::array::from_fn(|a| ((p[0][a].max(p[1][a]) + query_radius) / cell).floor() as i32);
            // Keep hash insertion bounded without discarding physical pairs.
            // Oversized segments use the existing AABB hierarchy below.
            if (0..3).any(|a| i64::from(max[a]) - i64::from(min[a]) > 32) {
                oversized.push((r,i));
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
    let query_separation=radius*2.+CONTACT_TOLERANCE;
    // A refresh does not move geometry: discard disjoint capsules before
    // allocation/sorting, preserving exactly the same surviving pair order.
    // Sequential projection can move later pairs into range, so it retains
    // the original broad candidate list and checks live bounds below.
    let mut pairs=if !filter_before_sort {unique_cell_pairs(&grid)} else {
        unique_cell_pairs_filtered(&grid,|((ra,ia),(rb,ib))| {
            !(0..3).any(|axis| {
                let a0=rods[ra].x[ia][axis];let a1=rods[ra].x[ia+1][axis];
                let b0=rods[rb].x[ib][axis];let b1=rods[rb].x[ib+1][axis];
                a0.min(a1)-query_separation>b0.max(b1) || b0.min(b1)-query_separation>a0.max(a1)
            })
        })
    };
    if !oversized.is_empty() {
        let segments:Vec<_>=rods.iter().enumerate().flat_map(|(r,rod)|(0..rod.x.len()-1).map(move|i|(r,i))).collect();
        let mut append=|a:SegmentId,b:SegmentId| {
            if a.0==b.0 && a.1.abs_diff(b.1)<=2 {return;}
            pairs.push(if a<b {(a,b)} else {(b,a)});
        };
        if project_positions {
            // Sequential projections can create later contacts. Preserve all
            // oversized candidates here, then check live capsule bounds in
            // the deterministic loop, rather than pruning stale geometry.
            for &a in &oversized {for &b in &segments {append(a,b);}}
        } else {
            let bounds:Vec<_>=segments.iter().map(|&(r,i)| {
                let a=rods[r].x[i];let b=rods[r].x[i+1];
                (std::array::from_fn(|axis|a[axis].min(b[axis])-query_radius),
                 std::array::from_fn(|axis|a[axis].max(b[axis])+query_radius))
            }).collect();
            for (a,b) in continuous::bound_pairs(&bounds) {
                let a=segments[a];let b=segments[b];
                if oversized.binary_search(&a).is_ok() || oversized.binary_search(&b).is_ok() {append(a,b);}
            }
        }
        pairs.sort_unstable();pairs.dedup();
    }
    for ((ra, ia), (rb, ib)) in pairs {
        let separation = radius * 2.;
        let query_separation = separation + CONTACT_TOLERANCE;
        // Grid cells are broader than a fibre. Reject disjoint current capsule
        // bounds before the more expensive segment-distance calculation.
        if !filter_before_sort && (0..3).any(|axis| {
            let a0 = rods[ra].x[ia][axis];
            let a1 = rods[ra].x[ia + 1][axis];
            let b0 = rods[rb].x[ib][axis];
            let b1 = rods[rb].x[ib + 1][axis];
            a0.min(a1) - query_separation > b0.max(b1) || b0.min(b1) - query_separation > a0.max(a1)
        }) {
            continue;
        }
        let a=rods[ra].x[ia];let b=rods[ra].x[ia+1];
        let c=rods[rb].x[ib];let d=rods[rb].x[ib+1];
        let (s,t,p,q)=segment_pair(a,b,c,d);
        if len(sub(p,q))>query_separation {continue;}
        // A parallel capsule pair has an interval of closest points. The
        // minimum alone cannot constrain relative velocity at its other end.
        // Endpoint-to-segment projections expose both interval endpoints and
        // preserve valid endpoint contacts for oblique finite segments too.
        let project_fraction=|point:V,origin:V,end:V| {
            let direction=sub(end,origin);let square=dot(direction,direction);
            if square==0. {0.} else {(dot(sub(point,origin),direction)/square).clamp(0.,1.)}
        };
        let witnesses=[(s,t),(0.,project_fraction(a,c,d)),(1.,project_fraction(b,c,d)),
            (project_fraction(c,a,b),0.),(project_fraction(d,a,b),1.)];
        for (witness,&(s,t)) in witnesses.iter().enumerate() {
            if witnesses[..witness].contains(&(s,t)) {continue;}
            // Legacy sequential projection changes the live endpoints. Recheck
            // each secondary witness against that updated geometry, so the same
            // initial penetration is not projected repeatedly.
            let (p,q)=if witness==0 {(p,q)} else {
                (add(rods[ra].x[ia],mul(sub(rods[ra].x[ia+1],rods[ra].x[ia]),s)),
                 add(rods[rb].x[ib],mul(sub(rods[rb].x[ib+1],rods[rb].x[ib]),t)))
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
            let normal = if (s==0. || s==1.) && (t==0. || t==1.) && distance>1e-12 {
                mul(delta,1./distance)
            } else {
                feature_strand_normal(&rods[ra],ia,s,&rods[rb],ib,t,delta)
            };
            let wa = contact_weight(&rods[ra],ia,s);
            let wb = contact_weight(&rods[rb],ib,t);
            if wa + wb < 1e-30 {
                continue;
            }
            let depth = separation - distance;
            let impulse = if project_positions {pair_impulse(rods,(ra,ia,s),(rb,ib,t),depth)} else {0.};
            // Keep the physical separation target from before projection. The
            // displacement limiter controls this iteration, not the contact gap:
            // an incomplete projection must still enter the structural solve.
            responses.push(StrandResponse {a:(ra,ia,s),b:(rb,ib,t),normal,impulse});
            let target_a=add(p,mul(normal,depth*wa/(wa+wb)));
            let target_b=add(q,mul(normal,-depth*wb/(wa+wb)));
            if wa > 0. && (!project_positions || project(&mut rods[ra],ia,s,normal,impulse*wa,None)) {
                rods[ra].record_contact(ia,s,normal,target_a,ContactSource::Strand {other_rod:rb,other_segment:ib});
            }
            if wb > 0. && (!project_positions || project(&mut rods[rb],ib,t,mul(normal,-1.),impulse*wb,None)) {
                rods[rb].record_contact(ib,t,mul(normal,-1.),target_b,ContactSource::Strand {other_rod:ra,other_segment:ia});
            }

        }
    }
    responses
}

#[cfg(test)]
mod query_tests {
    use super::*;
    #[test]
    fn pair_clearance_is_not_two_independent_target_tolerances() {
        let radius=40e-6;
        let half=(2.*radius-1.45e-10)*0.5;
        let mut rods=vec![
            HairRod::new(vec![[-0.001,0.,0.],[-half,0.01,0.],[-half,0.02,0.]],super::super::HairMaterial::default()).unwrap(),
            HairRod::new(vec![[0.001,0.,0.],[half,0.01,0.],[half,0.02,0.]],super::super::HairMaterial::default()).unwrap(),
        ];
        let roots=[rods[0].x[0],rods[1].x[0]];
        let pairs=refresh_strand_responses(&mut rods,radius,&[]);
        assert!(!pairs.is_empty());
        for rod in &rods {
            for contact in &rod.contacts {
                let p=add(mul(rod.x[contact.segment],1.-contact.fraction),mul(rod.x[contact.segment+1],contact.fraction));
                assert!(dot(sub(p,contact.target),contact.normal)>=-1e-10);
            }
        }
        assert!(!strand_geometry_admitted(&rods,&pairs,radius).unwrap());
        super::super::HairSystem::reconcile_positions(&mut rods,&[],1./240.,radius,true,&mut Vec::new(),None).unwrap();
        let pairs=refresh_strand_responses(&mut rods,radius,&[]);
        assert!(strand_geometry_admitted(&rods,&pairs,radius).unwrap());
        assert_eq!([rods[0].x[0],rods[1].x[0]],roots);
    }
    #[test]
    fn animated_surface_velocity_tracks_material_points_and_invalid_refits_are_atomic() {
        let points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]];
        let moved=[[0.,0.,0.],[1.,0.,1.],[0.,1.,2.]];
        let mut mesh=TriangleMesh::new(&points,&[[0,1,2]]).unwrap();
        mesh.refit_with_timestep(&moved,0.5).unwrap();
        for (point,expected) in [([0.2,0.3,0.8],1.6),([0.5,0.,0.5],1.),(moved[1],2.)] {
            assert!(len(sub(mesh.surface_velocity(0,point),[0.,0.,expected]))<1e-12);
        }
        let old=mesh.surface_velocity(0,[0.2,0.3,0.8]);
        assert!(mesh.refit_with_timestep(&points,0.).is_err());
        assert_eq!(mesh.surface_velocity(0,[0.2,0.3,0.8]),old);
        assert_eq!(mesh.triangles[0].p,moved);
        mesh.refit(&moved).unwrap();
        assert_eq!(mesh.surface_velocity(0,[0.2,0.3,0.8]),[0.;3],"untimed refit reused stale motion");
    }
    #[test]
    fn sampled_motion_uses_full_interval_velocity_and_original_endpoints() {
        let points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]];
        let mut source=TriangleMesh::new(&points,&[[0,1,2]]).unwrap();
        source.refit_with_timestep(&points.map(|p|add(p,[0.,0.,2.])),0.5).unwrap();
        let mut sample=source.clone();
        for fraction in [0.25,0.75,1.] {
            sample.sample_motion(&source,fraction).unwrap();
            assert_eq!(sample.triangles[0].p[0],[0.,0.,2.*fraction]);
            assert_eq!(sample.surface_velocity(0,[0.2,0.3,2.*fraction]),[0.,0.,4.]);
        }
        assert_eq!(source.triangles[0].p[0],[0.,0.,2.]);
        assert_eq!(source.triangles[0].previous_p[0],points[0]);
    }
    #[test]
    fn interval_motion_aligns_previous_vertices_duration_and_velocity() {
        let points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]];
        let mut source=TriangleMesh::new(&points,&[[0,1,2]]).unwrap();
        source.refit_with_timestep(&points.map(|p|add(p,[0.,0.,2.])),0.5).unwrap();
        let before=format!("{source:?}");
        let a=source.motion_interval(0.25,0.5).unwrap();
        let b=source.motion_interval(0.5,0.75).unwrap();
        assert_eq!(a.triangles[0].previous_p[0],[0.,0.,0.5]);
        assert_eq!(a.triangles[0].p[0],[0.,0.,1.]);
        assert_eq!(a.motion_duration(),Some(0.125));
        assert_eq!(a.triangles[0].p,b.triangles[0].previous_p);
        assert_eq!(a.surface_velocity(0,[0.2,0.3,1.]),[0.,0.,4.]);
        for (lo,hi) in [(0.5,0.5),(0.75,0.25),(-0.1,0.5),(0.,1.1),(f64::NAN,1.)] {
            assert!(source.motion_interval(lo,hi).is_err());
        }
        assert_eq!(format!("{source:?}"),before);
    }
    #[test]
    fn untimed_refit_is_stationary_in_continuous_queries() {
        let points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]];
        let mut source=TriangleMesh::new(&points,&[[0,1,2]]).unwrap();
        source.refit(&points.map(|p|add(p,[0.,0.,2.]))).unwrap();
        let sample=source.motion_interval(0.,1.).unwrap();
        assert_eq!(sample.triangles[0].previous_p,sample.triangles[0].p);
        assert_eq!(sample.motion_duration(),None);
    }
    #[test]
    fn collapsed_intermediate_motion_is_rejected_atomically() {
        let points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]];
        let mut source=TriangleMesh::new(&points,&[[0,1,2]]).unwrap();
        source.refit_with_timestep(&[[0.,0.,0.],[-1.,0.,0.],[0.,-1.,0.]],0.5).unwrap();
        let mut sample=source.clone();
        assert!(sample.sample_motion(&source,0.5).is_err());
        assert_eq!(sample.triangles[0].p,source.triangles[0].p);
        assert_eq!(sample.triangles[0].velocity,source.triangles[0].velocity);
    }
    #[test]
    fn moving_surface_response_preserves_common_translation() {
        let mut rod=HairRod::new(vec![[0.,0.,0.],[0.,0.01,0.],[0.,0.02,0.]],super::super::HairMaterial::default()).unwrap();
        let dt=1./240.;let velocity=[5.,-2.,3.];
        for (old,current) in rod.old_x.iter_mut().zip(&rod.x) {*old=sub(*current,mul(velocity,dt));}
        rod.record_surface_response(1,[0.,1e-4,0.],velocity);
        rod.finish(dt);
        assert!(len(sub(rod.velocity[1],velocity))<1e-12,"common surface motion generated friction: {:?}",rod.velocity[1]);
    }
    #[test]
    fn released_strand_contacts_do_not_apply_historical_velocity_impulses() {
        let make=|x|HairRod::new(vec![[x,0.,0.],[x,0.01,0.],[x,0.02,0.]],super::super::HairMaterial::default()).unwrap();
        let mut rods=vec![make(0.),make(60e-6)];
        let history=self_contacts(&mut rods,40e-6);
        assert!(!history.is_empty());
        for point in &mut rods[1].x {point[0]+=0.01;}
        rods[0].velocity.fill([1.,2.,0.]);rods[1].velocity.fill([-1.,0.,0.]);
        let before=rods.clone();
        let current=refresh_strand_responses(&mut rods,40e-6,&history);
        assert!(current.is_empty());
        finish_strand_contacts(&mut rods,&current,1./240.);
        for (actual,expected) in rods.iter().zip(&before) {
            assert_eq!(actual.x,expected.x);assert_eq!(actual.velocity,expected.velocity);
            assert!(actual.contacts.iter().all(|c|matches!(c.source,ContactSource::Mesh(_))));
        }
    }
    #[test]
    fn refreshed_strand_contacts_use_current_geometry_without_projecting() {
        let make=|x|HairRod::new(vec![[x,0.,0.],[x,0.01,0.],[x,0.02,0.]],super::super::HairMaterial::default()).unwrap();
        let mut rods=vec![make(0.),make(80e-6)];
        let history=[StrandResponse {a:(0,1,0.5),b:(1,1,0.5),normal:[0.,1.,0.],impulse:1e-6}];
        let before=rods.clone();let current=refresh_strand_responses(&mut rods,40e-6,&history);
        assert!(!current.is_empty());
        for response in current {
            assert!(response.normal[0] < -0.99);assert_eq!(response.impulse,0.,"orthogonal historical load must not become friction");
        }
        for (actual,expected) in rods.iter().zip(before) {assert_eq!(actual.x,expected.x);}
    }
    #[test]
    fn current_strand_manifold_transfers_projection_load_once_per_support() {
        let make=|x|HairRod::new(vec![[x,0.,0.],[x,0.01,0.],[x,0.02,0.]],super::super::HairMaterial::default()).unwrap();
        let mut rods=vec![make(0.),make(80e-6)];
        let history=[
            StrandResponse {a:(0,1,0.2),b:(1,1,0.2),normal:[-1.,0.,0.],impulse:1e-6},
            StrandResponse {a:(0,1,0.8),b:(1,1,0.8),normal:[-1.,0.,0.],impulse:2e-6},
        ];
        let current=refresh_strand_responses(&mut rods,40e-6,&history);
        let pair=current.iter().filter(|r|r.a.1==1 && r.b.1==1).collect::<Vec<_>>();
        assert_eq!(pair.len(),2,"parallel finite segments need both support endpoints");
        for (fraction,pressure) in [(0.,1e-6),(1.,2e-6)] {
            let support=pair.iter().find(|r|r.a.2==fraction && r.b.2==fraction).unwrap();
            assert!((support.impulse-pressure).abs()<1e-20);
            assert!(len(sub(support.normal,[-1.,0.,0.]))<1e-15);
        }
        assert!((pair.iter().map(|r|r.impulse).sum::<f64>()-3e-6).abs()<1e-20);
    }
    #[test]
    fn nonlinear_joint_contact_requeries_pairs_created_by_a_solved_increment() {
        let make=|x|HairRod::new(vec![[x,0.,0.],[x,0.01,0.],[x,0.02,0.]],super::super::HairMaterial::default()).unwrap();
        let initial=vec![make(0.),make(20e-6),make(120e-6)];let radius=40e-6;let dt=1./240.;
        let penetration=|rods:&[HairRod],responses:&[StrandResponse]|responses.iter().map(|r| {
            let (a,i,s)=r.a;let (b,j,t)=r.b;
            let pa=add(mul(rods[a].x[i],1.-s),mul(rods[a].x[i+1],s));
            let pb=add(mul(rods[b].x[j],1.-t),mul(rods[b].x[j+1],t));
            2.*radius-dot(sub(pa,pb),r.normal)
        }).fold(0.,f64::max);
        let mut once=initial.clone();let mut contacts=refresh_strand_responses(&mut once,radius,&[]);
        assert!(reconcile_contact_positions(&mut once,&mut contacts,dt,radius).unwrap());
        let newly_detected=refresh_strand_responses(&mut once,radius,&[]);
        assert!(penetration(&once,&newly_detected)>1e-9,"fixture must expose a new contact after a solved tangent step");
        let mut actual=initial.clone();let mut history=Vec::new();
        super::super::HairSystem::reconcile_positions(&mut actual,&[],dt,radius,true,&mut history,None).unwrap();
        let current=refresh_strand_responses(&mut actual,radius,&[]);
        assert!(penetration(&actual,&current)<=1e-10);
        for (after,before) in actual.iter().zip(initial) {
            assert_eq!(after.x[0],before.x[0]);assert_eq!(after.q[0],before.q[0]);
            assert_eq!(after.velocity,before.velocity);
        }
    }
    #[test]
    fn strand_velocity_response_preserves_common_translation() {
        let make=|x|HairRod::new(vec![[x,0.,0.],[x,0.01,0.],[x,0.02,0.]],super::super::HairMaterial::default()).unwrap();
        let mut rods=vec![make(0.),make(80e-6)];
        for rod in &mut rods {rod.velocity.fill([5.,-2.,3.]);}
        let response=StrandResponse {a:(0,1,0.5),b:(1,1,0.5),normal:[1.,0.,0.],impulse:1e-8};
        finish_strand_contacts(&mut rods,&[response],1./240.);
        assert!(rods.iter().all(|rod|rod.velocity.iter().all(|v|*v==[5.,-2.,3.])));
    }
    #[test]
    fn strand_velocity_response_conserves_momentum_dissipates_energy_and_is_galilean_invariant() {
        let make=|length|HairRod::new(vec![[0.,0.,0.],[0.,length,0.],[0.,2.*length,0.]],super::super::HairMaterial::default()).unwrap();
        let mut rods=vec![make(0.01),make(0.02)];
        rods[0].velocity.fill([-1.,2.,0.]);rods[1].velocity.fill([1.,0.,0.]);
        let before=rods.clone();let mut boosted=rods.clone();let boost=[7.,-3.,2.];
        for rod in &mut boosted {for v in &mut rod.velocity {*v=add(*v,boost);}}
        let response=StrandResponse {a:(0,1,0.5),b:(1,1,0.5),normal:[1.,0.,0.],impulse:0.};
        finish_strand_contacts(&mut rods,std::slice::from_ref(&response),1./240.);
        finish_strand_contacts(&mut boosted,&[response],1./240.);
        let mut momentum=[0.;3];let mut before_energy=0.;let mut after_energy=0.;let mut scale=0.;
        for ((old,rod),shifted) in before.iter().zip(&rods).zip(&boosted) {
            for i in 1..rod.x.len() {
                let mass=1./rod.inv_mass[i];
                let delta=mul(sub(rod.velocity[i],old.velocity[i]),mass);
                momentum=add(momentum,delta);scale+=len(delta);
                before_energy+=0.5*mass*dot(old.velocity[i],old.velocity[i]);
                after_energy+=0.5*mass*dot(rod.velocity[i],rod.velocity[i]);
                assert!(len(sub(sub(shifted.velocity[i],rod.velocity[i]),boost))<1e-12);
            }
        }
        assert!(len(momentum)<scale*1e-12);
        assert!(after_energy<before_energy);
    }
    #[test]
    fn limited_strand_projection_preserves_unresolved_penetration_in_constraints() {
        let material=super::super::HairMaterial::default();
        let short=HairRod::new(vec![[0.,-0.002,0.],[0.,-0.001,0.],[0.,0.,0.]],material).unwrap();
        let other=HairRod::new(vec![[0.,-0.002,0.005],[0.,-0.001,0.005],[0.,0.,0.005]],material).unwrap();
        let mut rods=vec![short,other];
        self_contacts(&mut rods,0.01);
        assert!(rods[0].contacts.iter().any(|contact| {
            if !matches!(contact.source,ContactSource::Strand {other_rod:1,other_segment:1}) {return false;}
            let position=add(mul(rods[0].x[contact.segment],1.-contact.fraction),mul(rods[0].x[contact.segment+1],contact.fraction));
            dot(sub(position,contact.target),contact.normal) < -1e-4
        }),"limited projection erased its remaining penetration: {:?}",rods[0].contacts);
    }
    #[test]
    fn capped_pair_projection_preserves_free_particle_momentum() {
        let make = |length| HairRod::new(vec![[0.,0.,0.],[0.,length,0.],[0.,2.*length,0.]],super::super::HairMaterial::default()).unwrap();
        let mut rods=vec![make(0.001),make(0.1)];
        let before:Vec<_>=rods.iter().map(|rod|rod.x.clone()).collect();
        let fraction=0.5;
        let impulse=pair_impulse(&rods,(0,1,fraction),(1,1,fraction),10.);
        for (index,sign) in [(0,1.),(1,-1.)] {
            let weight=contact_weight(&rods[index],1,fraction);
            assert!(project(&mut rods[index],1,fraction,[sign,0.,0.],impulse*weight,None));
        }
        let mut momentum=[0.;3];let mut magnitude=0.;
        for (rod,positions) in rods.iter().zip(before) {
            for i in 1..rod.x.len() {
                let change=mul(sub(rod.x[i],positions[i]),1./rod.inv_mass[i]);
                momentum=add(momentum,change);magnitude+=len(change);
                assert!(len(sub(rod.x[i],positions[i]))<=0.2*rod.lengths[1]*(1.+1e-12));
            }
        }
        assert!(len(momentum)<=magnitude*1e-12,"capped contact created net momentum: {momentum:?}");
        assert!(magnitude>0.);
    }
    #[test]
    fn near_parallel_segments_preserve_the_interior_minimum_across_scales() {
        for scale in [1e-4, 1., 1e4] {
            let a=[0.;3];
            let b=[scale,0.,0.];
            let c=[0.,-scale*5e-11,scale*1e-4];
            let d=[scale,scale*5e-11,scale*1e-4];
            for (a,b,c,d) in [(a,b,c,d),(c,d,a,b),(b,a,c,d),(a,b,d,c)] {
                let (s,t,p,q)=segment_pair(a,b,c,d);
                assert!((s-0.5).abs()<1e-12 && (t-0.5).abs()<1e-12,"lost interior minimum: {s}, {t}");
                assert!((len(sub(p,q))/scale-1e-4).abs()<1e-15);
            }
        }
    }
    #[test]
    fn point_segments_project_onto_the_other_segment_in_both_orders() {
        let a=[0.,0.,0.];let b=[1.,0.,0.];let p=[0.5,0.25,0.];
        assert_eq!(segment_pair(a,b,p,p),(0.5,0.,[0.5,0.,0.],p));
        assert_eq!(segment_pair(p,p,a,b),(0.,0.5,p,[0.5,0.,0.]));
        assert_eq!(segment_pair(p,p,a,a),(0.,0.,p,a));
    }
    #[test]
    fn separated_strands_retain_boundary_planes_without_projection_or_adhesion() {
        let radius = 40e-6;
        let separation = 2. * radius + 5e-11;
        let make = |x| HairRod::new(vec![[x,0.,0.],[x,0.,0.01],[x,0.,0.02]],super::super::HairMaterial::default()).unwrap();
        let mut rods = vec![make(0.),make(separation)];
        let before: Vec<_> = rods.iter().map(|rod|rod.x.clone()).collect();
        self_contacts(&mut rods,radius);
        for (rod,positions) in rods.iter().zip(before) {
            assert_eq!(rod.x,positions,"separated strands received a projection");
            assert!(rod.normals.iter().all(|normal|*normal==[0.;3]),"separated strands received friction impulses");
            assert!(!rod.contacts.is_empty(),"boundary contact was discarded");
            for contact in &rod.contacts {
                let position=add(mul(rod.x[contact.segment],1.-contact.fraction),mul(rod.x[contact.segment+1],contact.fraction));
                assert!(dot(sub(position,contact.target),contact.normal)>0.,"separated plane became penetrating");
            }
        }
    }
    #[test]
    fn nearby_geometric_planes_do_not_replace_distinct_contacts() {
        let mut rod=HairRod::new(vec![[0.,0.,0.],[0.,0.,0.01],[0.,0.,0.02]],super::super::HairMaterial::default()).unwrap();
        let normal_a=[1.,0.,0.];
        let normal_b=[(1f64-1e-14).sqrt(),1e-7,0.];
        let target=[0.,0.,0.01];
        rod.record_contact(1,0.,normal_a,target,ContactSource::Mesh(0));
        let original=rod.contacts[0].clone();
        rod.record_contact(1,0.,normal_b,target,ContactSource::Mesh(0));
        let probe=[-5e-10,0.01,0.01];
        assert!(original.physical_gap(probe)< -1e-10);
        assert!(rod.contacts.iter().any(|row|row.physical_gap(probe)< -1e-10),
            "nearby contact replaced a plane that excludes the probe");
        assert_eq!(rod.contacts.len(),2);
        // A tangentially shifted witness describes exactly the same plane.
        rod.record_contact(1,0.,normal_a,[0.,0.02,0.01],ContactSource::Mesh(0));
        assert_eq!(rod.contacts.len(),2,"exact plane alias duplicated");
        // A scaled envelope witness is a distinct physical Jacobian.
        rod.contacts[0].metric_scale=0.25;
        rod.record_contact(1,0.,normal_a,target,ContactSource::Mesh(0));
        assert_eq!(rod.contacts.len(),3,"scaled witness was overwritten");
    }
    #[test]
    fn shared_endpoint_contact_is_unique_and_collision_owners_expire_separately() {
        let mut rod=HairRod::new(vec![[0.,0.,0.],[0.,0.,1.],[0.,0.,2.]],super::super::HairMaterial::default()).unwrap();
        rod.record_point_contact(1,[1.,0.,0.],[0.,0.,1.],ContactSource::Mesh(0));
        rod.record_contact(0,1.,[1.,0.,0.],[0.,0.,1.],ContactSource::Mesh(0));
        assert_eq!(rod.contacts.len(),1,"node and capsule counted the shared endpoint twice");
        rod.record_contact(1,0.5,[0.,1.,0.],[0.,0.,1.5],ContactSource::Strand {other_rod:1,other_segment:0});
        mesh_contacts(&mut rod,&[],40e-6);
        assert_eq!(rod.contacts.len(),1);
        assert!(matches!(rod.contacts[0].source,ContactSource::Strand {..}));
        self_contacts(std::slice::from_mut(&mut rod),40e-6);
        assert!(rod.contacts.is_empty(),"obsolete strand constraints survived their owner refresh");
    }
    #[test]
    fn closest_point_regions_preserve_face_edge_and_vertex_identity() {
        let mesh=TriangleMesh::new(&[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]],&[[0,1,2]]).unwrap();
        let tri=&mesh.triangles[0];
        let cases=[
            ([-1.,-1.,0.2],[0.,0.,0.],ClosestFeature::Vertex(0)),
            ([2.,-1.,0.2],[1.,0.,0.],ClosestFeature::Vertex(1)),
            ([-1.,2.,0.2],[0.,1.,0.],ClosestFeature::Vertex(2)),
            ([0.5,-1.,0.2],[0.5,0.,0.],ClosestFeature::Edge(0,1)),
            ([-1.,0.5,0.2],[0.,0.5,0.],ClosestFeature::Edge(0,2)),
            ([1.,1.,0.2],[0.5,0.5,0.],ClosestFeature::Edge(1,2)),
            ([0.25,0.25,0.2],[0.25,0.25,0.],ClosestFeature::Face),
        ];
        for (query,point,feature) in cases {
            let actual=closest_triangle_feature(query,tri);
            assert_eq!(actual,(point,feature));
            assert_eq!(closest_triangle(query,tri),point);
        }
    }
    #[test]
    fn accumulated_contact_direction_matches_applied_weighted_corrections() {
        let mut rod=HairRod::new(vec![[0.,0.,0.],[0.,0.,1.],[0.,0.,2.]],super::super::HairMaterial::default()).unwrap();
        let initial=rod.x.clone();
        project(&mut rod,0,0.2,[1.,0.,0.],0.1,Some([0.;3]));
        project(&mut rod,1,0.8,[0.,1.,0.],0.8,Some([0.;3]));
        for i in 0..rod.x.len() {
            let applied=sub(rod.x[i],initial[i]);
            assert!(len(sub(applied,rod.normals[i]))<1e-14,"point {i}: {:?} != {:?}",applied,rod.normals[i]);
        }
        assert_eq!(rod.normals[0],[0.;3]);
        assert!(rod.x[1][0]>0.19 && rod.x[1][1]>0.);
    }
    #[test]
    fn allowed_follicle_contact_does_not_hide_a_later_surface_crossing() {
        let mesh=TriangleMesh::new(&[[-2.,-2.,0.],[2.,-2.,0.],[0.,2.,0.],[-2.,-2.,0.5],[2.,-2.,0.5],[0.,2.,0.5]],&[[0,1,2],[3,4,5]]).unwrap();
        let mut rod=HairRod::new(vec![[0.,0.,0.],[0.,0.,1.],[0.,0.,2.]],super::super::HairMaterial::default()).unwrap();
        mesh_contacts(&mut rod,std::slice::from_ref(&mesh),40e-6);
        assert_eq!(rod.x[0],[0.;3]);
        assert!((rod.x[1][2]-(1.+80e-6)).abs()<1e-12,"later contact was hidden: {:?}",rod.x);
    }
    #[test]
    fn multiple_intersections_choose_first_and_shared_features_use_topology() {
        assert!(segment_contact_precedes(5e-16,0.2,[8,9,10],0.,0.8,[0,1,2],1.));
        assert!(!segment_contact_precedes(0.,0.8,[0,1,2],5e-16,0.2,[8,9,10],1.));
        assert!(segment_contact_precedes(1e-16,0.2+1e-12,[0,1,2],0.,0.2,[8,9,10],1.));
        assert!(!segment_contact_precedes(0.,0.2,[8,9,10],1e-16,0.2+1e-12,[0,1,2],1.));
        assert!(segment_contact_precedes(1e-5,0.8,[8,9,10],2e-5,0.2,[0,1,2],1.));
    }
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
    fn interior_peak_near_endpoint_is_not_hidden_by_endpoint_sample() {
        let peak=0.2/(3f64.sqrt()+1.);
        for scale in [1e-3,1.,1e3] {
            let points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]].map(|p|mul(p,scale));
            let mut mesh=TriangleMesh::new(&points,&[[0,2,1],[0,1,3],[0,3,2],[1,2,3]]).unwrap();
            mesh.enable_closed_feature_normals().unwrap();
            let a=mul([0.,0.4,0.4],scale);let b=mul([peak/0.99,0.4,0.4],scale);
            for (a,b,expected_t) in [(a,b,0.99),(b,a,0.01)] {
                let (t,signed,_)=deepest_interval_witness(&mesh,a,b,0.,1.).unwrap();
                assert!((signed/scale+peak).abs()<1e-8,"missed analytic interior depth: {signed} at {t}");
                assert!((signed+peak*scale).abs()<2e-12,"witness precision is insufficient for the 1e-10 m contact gate");
                assert!((t-expected_t).abs()<1e-6,"interior ridge was replaced by endpoint");
            }
        }
    }

    #[test]
    fn medial_envelope_gradient_matches_analytic_tetrahedron_derivative() {
        let points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]];
        let mut mesh=TriangleMesh::new(&points,&[[0,2,1],[0,1,3],[0,3,2],[1,2,3]]).unwrap();
        mesh.enable_closed_feature_normals().unwrap();
        let a=[0.,0.4,0.4];let b=[0.2,0.4,0.4];
        let (t,_,normal)=deepest_interval_witness(&mesh,a,b,0.,1.).unwrap();
        let (direction,scale,_)=interior_envelope_gradient(&mesh,a,b,t,0.,1.,normal,[0.;3]);
        let gradient=mul(direction,scale);
        let expected=1./(3f64.sqrt()+1.);
        assert!(gradient[0].abs()<1e-10);
        assert!((gradient[1]-expected).abs()<1e-8 && (gradient[2]-expected).abs()<1e-8);
        let h=1e-5;
        let shifted=|dy|deepest_interval_witness(&mesh,add(a,[0.,dy,0.]),add(b,[0.,dy,0.]),0.,1.).unwrap().1;
        let derivative=(shifted(h)-shifted(-h))/(2.*h);
        assert!((derivative-gradient[1]).abs()<1e-4);
    }

    #[test]
    fn interior_witness_finds_deeper_contact_than_crossing_midpoint() {
        for scale in [1e-3,1.,1e3] {
            let points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]].map(|p|mul(p,scale));
            let mut mesh=TriangleMesh::new(&points,&[[0,2,1],[0,1,3],[0,3,2],[1,2,3]]).unwrap();
            mesh.enable_closed_feature_normals().unwrap();
            let a=mul([0.,0.2,0.2],scale);let b=mul([0.6,0.2,0.2],scale);
            let (_,depth,_)=deepest_interval_witness(&mesh,a,b,0.,1.).unwrap();
            let midpoint=mesh.signed_distance_closed(mul(add(a,b),0.5)).unwrap().0;
            assert!((depth/scale+0.2).abs()<1e-8,"tetrahedron has analytic maximum interior depth 0.2");
            assert!(depth<midpoint-0.02*scale);
        }
    }

    #[test]
    fn fully_interior_segment_discovers_depth_without_boundary_crossings() {
        let points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]];
        let mut mesh=TriangleMesh::new(&points,&[[0,2,1],[0,1,3],[0,3,2],[1,2,3]]).unwrap();
        mesh.enable_closed_feature_normals().unwrap();
        let mut rod=HairRod::new(vec![[0.05,0.2,0.2],[0.1,0.2,0.2],[0.5,0.2,0.2]],Default::default()).unwrap();
        let before=rod.x.clone();
        refresh_mesh_constraints(&mut rod,&[mesh],40e-6);
        assert_eq!(rod.x,before);
        assert!(rod.contacts.iter().any(|c| {
            let p=add(mul(rod.x[c.segment],1.-c.fraction),mul(rod.x[c.segment+1],c.fraction));
            c.segment==1 && c.fraction>0. && c.fraction<1. && dot(sub(p,c.target),c.normal) < -0.199
        }),"wholly interior segment must expose analytic interior depth 0.2");
    }

    #[test]
    fn crossing_segment_recovers_interior_depth_with_exterior_endpoints() {
        let points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]];
        let mut mesh=TriangleMesh::new(&points,&[[0,2,1],[0,1,3],[0,3,2],[1,2,3]]).unwrap();
        mesh.enable_closed_feature_normals().unwrap();
        let mut rod=HairRod::new(vec![[-0.5,0.2,0.2],[-0.1,0.2,0.2],[0.9,0.2,0.2]],Default::default()).unwrap();
        let before=rod.x.clone();
        refresh_mesh_constraints(&mut rod,&[mesh],40e-6);
        assert_eq!(rod.x,before,"constraint discovery must not move the rod");
        assert!(rod.contacts.iter().any(|c| {
            let p=add(mul(rod.x[c.segment],1.-c.fraction),mul(rod.x[c.segment+1],c.fraction));
            dot(sub(p,c.target),c.normal) < -0.1
        }),"surface intersection must expose interior penetration, not just fiber radius");
    }

    #[test]
    fn feature_normals_classify_faces_edges_vertices_and_refresh_after_motion() {
        let points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]];
        let faces=[[0,2,1],[0,1,3],[0,3,2],[1,2,3]];
        let mut mesh=TriangleMesh::new(&points,&faces).unwrap();
        assert!(mesh.signed_distance_closed([0.1;3]).is_err());
        mesh.enable_closed_feature_normals().unwrap();
        for (point,inside) in [([0.1;3],true),([-0.1;3],false),([-0.1,-0.1,0.3],false),([0.3,0.3,-0.1],false)] {
            let (distance,normal)=mesh.signed_distance_closed(point).unwrap();
            assert_eq!(distance<0.,inside);
            assert!((len(normal)-1.).abs()<1e-14);
            assert_eq!(mesh.contains_closed_surface(point).unwrap(),Some(inside));
        }
        let (_,vertex_normal)=mesh.signed_distance_closed([-0.1;3]).unwrap();
        assert!(len(sub(vertex_normal,unit([-1.;3])))<1e-14);
        let (_,edge_normal)=mesh.signed_distance_closed([-0.1,-0.1,0.3]).unwrap();
        assert!(len(sub(edge_normal,unit([-1.,-1.,0.])))<1e-14);
        let moved=points.map(|point|add(point,[2.,3.,4.]));
        mesh.refit(&moved).unwrap();
        assert!(mesh.signed_distance_closed([2.1,3.1,4.1]).unwrap().0<0.);
        let transformed=mesh.transformed([[1.,0.,0.,-2.],[0.,1.,0.,-3.],[0.,0.,1.,-4.]]).unwrap();
        assert!(transformed.signed_distance_closed([0.1;3]).unwrap().0<0.);
    }

    #[test]
    fn feature_topology_welds_seams_and_rejects_open_or_split_surfaces() {
        let points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]];
        let faces=[[0,2,1],[0,1,3],[0,3,2],[1,2,3]];
        let mut seam_points:Vec<V>=faces.iter().flat_map(|face|face.map(|i|points[i])).collect();
        let seam_faces=[[0,1,2],[3,4,5],[6,7,8],[9,10,11]];
        let mut mesh=TriangleMesh::new(&seam_points,&seam_faces).unwrap();
        mesh.enable_closed_feature_normals().unwrap();
        let previous=mesh.signed_distance_closed([-0.1;3]).unwrap();
        seam_points[0][0]+=0.01;
        assert!(mesh.refit(&seam_points).is_err());
        assert_eq!(mesh.signed_distance_closed([-0.1;3]).unwrap(),previous);
        let mut open=TriangleMesh::new(&points,&faces[..3]).unwrap();
        assert!(open.enable_closed_feature_normals().is_err());
        let mut reversed=faces;reversed[0].swap(1,2);
        let mut inconsistent=TriangleMesh::new(&points,&reversed).unwrap();
        assert!(inconsistent.enable_closed_feature_normals().is_err());
    }
    #[test]
    fn concave_edge_recovery_uses_metric_gradient_instead_of_sign_pseudonormal() {
        let outline=[[0.,0.],[2.,0.],[2.,1.],[1.,1.],[1.,2.],[0.,2.]];
        let points:Vec<V>=[-1.,1.].into_iter().flat_map(|z|outline.map(|[x,y]|[x,y,z])).collect();
        let cap=[[0,1,3],[1,2,3],[0,3,5],[3,4,5]];
        let mut faces:Vec<[usize;3]>=cap.into_iter().map(|[a,b,c]|[c,b,a]).collect();
        faces.extend(cap.map(|[a,b,c]|[a+6,b+6,c+6]));
        for a in 0..6 {let b=(a+1)%6;faces.extend([[a,b,b+6],[a,b+6,a+6]]);}
        let mut mesh=TriangleMesh::new(&points,&faces).unwrap();
        mesh.enable_closed_feature_normals().unwrap();
        let point=[0.9,0.8,0.];
        let (signed,pseudonormal)=mesh.signed_distance_closed(point).unwrap();
        let (closest,_)=mesh.closest_surface(point).unwrap();
        assert!(len(sub(closest,[1.,1.,0.]))<1e-14);
        assert!(signed<0.);
        let radius=0.02;
        let old_projection=add(point,mul(pseudonormal,radius-signed));
        assert!(mesh.signed_distance_closed(old_projection).unwrap().0<0.,"fixture must expose the old inward recovery");
        let gradient=distance_gradient(sub(point,closest),signed.abs(),signed,pseudonormal,1e-12);
        let projected=add(point,mul(gradient,radius-signed));
        assert!(mesh.signed_distance_closed(projected).unwrap().0>0.,"metric recovery must exit the solid");
        assert!(len(sub(gradient,unit([0.1,0.2,0.])))<1e-14);
        let (_,witness_signed,witness_normal)=deepest_interval_witness(&mesh,point,point,0.,1.).unwrap();
        assert!((witness_signed-signed).abs()<1e-14);
        assert!(len(sub(witness_normal,gradient))<1e-14,"interior witness must use the metric gradient too");
    }
    #[test]
    fn feature_admission_rejects_touching_vertex_fans_and_collapsed_refits() {
        let points=[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]];
        let outer=[[0,2,1],[0,1,3],[0,3,2],[1,2,3]];
        let mut touching_points=points.to_vec();
        touching_points.extend([[0.,0.,0.],[-1.,0.,0.],[0.,-1.,0.],[0.,0.,-1.]]);
        let mut faces=outer.to_vec();
        faces.extend(outer.map(|[a,b,c]|[a+4,c+4,b+4]));
        let mut touching=TriangleMesh::new(&touching_points,&faces).unwrap();
        assert_eq!(touching.enable_closed_feature_normals(),Err("feature normals require manifold vertex fans"));
        let mut mesh=TriangleMesh::new(&points,&outer).unwrap();
        mesh.enable_closed_feature_normals().unwrap();
        let before=mesh.signed_distance_closed([-0.1;3]).unwrap();
        let mut collapsed=points;collapsed[1]=collapsed[0];
        assert!(mesh.refit(&collapsed).is_err());
        assert_eq!(mesh.signed_distance_closed([-0.1;3]).unwrap(),before);
    }
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
        let mut mesh = TriangleMesh::new(&points, &faces).unwrap();
        mesh.enable_closed_feature_normals().unwrap();
        assert!(mesh.signed_distance_closed([0.1;3]).unwrap().0<0.);
        assert!(mesh.signed_distance_closed([0.35;3]).unwrap().0>0.);
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
    #[ignore = "paired broad-phase benchmark"]
    fn owner_filter_profile() {
        let mut grid = ContactGrid::new();
        for rod in 0..469usize { for segment in 0..20usize {
            let min = [(rod % 19) as i32 - 9, segment as i32 - 10, (rod / 19) as i32 - 12];
            for x in min[0]..=min[0]+1 { for y in min[1]..=min[1]+2 { for z in min[2]..=min[2]+1 {
                grid.entry(cell_key([x,y,z])).or_default().push(((rod, segment), min));
            } } }
        } }
        let baseline = || {
            let mut pairs = Vec::new();
            for (&key, entries) in &grid { for a in 0..entries.len() { for b in a+1..entries.len() {
                let owner = std::array::from_fn(|axis| entries[a].1[axis].max(entries[b].1[axis]));
                if cell_key(owner) != key { continue; }
                let mut pair = (entries[a].0, entries[b].0);
                if pair.0 > pair.1 { pair = (pair.1, pair.0); }
                if pair.0.0 == pair.1.0 && pair.0.1.abs_diff(pair.1.1) <= 2 { continue; }
                pairs.push(pair);
            } } }
            pairs.sort_unstable(); pairs
        };
        let reference = baseline();
        assert_eq!(unique_cell_pairs(&grid), reference);
        assert_eq!(candidate_cell_pairs(&grid), reference);
        let mut times = [Vec::new(), Vec::new()];
        for repeat in 0..12 { for mode in [repeat % 2, 1-repeat % 2] {
            let started = std::time::Instant::now();
            let result = if mode == 0 { baseline() } else { unique_cell_pairs(&grid) };
            times[mode].push(started.elapsed().as_secs_f64()*1000.);
            assert_eq!(result, reference);
            std::hint::black_box(result);
        } }
        for samples in &mut times { samples.sort_by(f64::total_cmp); }
        eprintln!("PACKED SORT pairs={} baseline_median_ms={:.3} candidate_median_ms={:.3}",
            reference.len(), times[0][6], times[1][6]);
    }
    #[test]
    fn packed_sort_preserves_lexicographic_order_and_large_id_fallback() {
        for size in [0, 1, 127, 128, 4096] {
            for large in [false, true] {
                let mut pairs: Vec<_> = (0..size).map(|i| {
                    let rod = if large { usize::MAX-i } else { i*37%469 };
                    ((rod, i*13%20), (i*71%469, i*7%20))
                }).collect();
                let mut reference = pairs.clone(); reference.sort_unstable();
                sort_contact_pairs(&mut pairs);
                assert_eq!(pairs, reference);
            }
        }
        let mut zeros = vec![((0,0),(0,0)); 128];
        sort_contact_pairs(&mut zeros);
        assert!(zeros.iter().all(|p| *p == ((0,0),(0,0))));
    }
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
        assert_eq!(candidate_cell_pairs(&grid), reference);
    }
}

#[cfg(test)]
#[path="segment_pair_fixture_tests.rs"]
mod segment_pair_fixture_tests;

#[cfg(test)]
#[path="friction_diagnostics_tests.rs"]
mod friction_diagnostics_tests;

#[test]
fn interior_face_crossing_has_zero_separation_without_projected_roundoff() {
    let p=[[0.13,-0.27,0.19],[1.17,0.31,0.41],[0.23,1.29,0.73]];
    let normal=unit(cross(sub(p[1],p[0]),sub(p[2],p[0])));
    let tri=Triangle {ids:[0,1,2],p,previous_p:p,velocity:[[0.;3];3],normal,min:[0.;3],max:[0.;3]};
    let center=mul(add(add(p[0],p[1]),p[2]),1./3.);
    for shift in [-1e-15,0.,1e-15] {
        let center=add(center,[shift,0.,0.]);
        let a=sub(center,mul(normal,0.2));let b=add(center,mul(normal,0.2));
        let (fraction,x,y)=segment_triangle(a,b,&tri);
        assert!((fraction-0.5).abs()<1e-12);
        assert_eq!(x,y,"a genuine interior crossing acquired a spurious positive distance");
    }
    // A plane crossing outside the face must retain its real separation.
    let outside=add(p[1],sub(p[1],p[0]));
    let (_,x,y)=segment_triangle(sub(outside,normal),add(outside,normal),&tri);
    assert!(len(sub(x,y))>0.1);
}

#[test]
fn flat_face_contact_preserves_both_ends_of_the_supported_segment() {
    let mesh=TriangleMesh::new(&[[-1.,0.,0.],[1.,0.,0.],[0.,1.,0.]],&[[0,1,2]]).unwrap();
    for tilt in [-1e-12,0.,1e-12] {
        let radius=40e-6;
        let mut rod=HairRod::new(vec![[-3.,0.5,radius],[-2.,0.5,radius-tilt],[2.,0.5,radius+tilt]],Default::default()).unwrap();
        let before=rod.x.clone();
        refresh_mesh_constraints(&mut rod,std::slice::from_ref(&mesh),radius);
        assert_eq!(rod.x,before,"discovery must not move the rod");
        for fraction in [0.375,0.625] {
            assert!(rod.contacts.iter().any(|c|c.segment==1 && (c.fraction-fraction).abs()<1e-12),
                "flat face must retain both support endpoints at tilt {tilt}: {:?}",rod.contacts);
        }
    }
}

#[test]
fn flat_face_velocity_response_admits_both_support_endpoints() {
    let mesh=TriangleMesh::new(&[[-1.,0.,0.],[1.,0.,0.],[0.,1.,0.]],&[[0,1,2]]).unwrap();
    let radius=40e-6;let dt=1./240.;
    for tilt in [-1e-12,0.,1e-12] {
        for direction in [-1.,1.] {
            let mut rod=HairRod::new(vec![[-3.,0.5,radius],[-2.,0.5,radius-tilt],[2.,0.5,radius+tilt]],Default::default()).unwrap();
            refresh_mesh_constraints(&mut rod,std::slice::from_ref(&mesh),radius);
            rod.velocity[1]=[0.,0.,direction];rod.velocity[2]=[0.,0.,-direction];
            stabilize_contact_velocities(std::slice::from_mut(&mut rod),&[],dt,radius).unwrap();
            // Independently check both geometric support endpoints, including
            // the one the old single-witness discovery omitted.
            for t in [0.375,0.625] {
                let p=add(mul(rod.x[1],1.-t),mul(rod.x[2],t));
                let velocity=add(mul(rod.velocity[1],1.-t),mul(rod.velocity[2],t));
                let bound=(-(p[2]-radius)/dt).min(0.);
                assert!(velocity[2]>=bound-1e-9,"missing supported endpoint: tilt={tilt} t={t} velocity={velocity:?} bound={bound}");
            }
        }
    }
}

#[test]
fn tessellated_flat_face_retains_support_across_both_triangles() {
    for (u,v,n) in [([1.,0.,0.],[0.,1.,0.],[0.,0.,1.]),([0.6,0.8,0.],[-0.48,0.36,0.8],[0.64,-0.48,0.6])] {
      for scale in [0.01,1.,100.] {
        let radius=40e-6;let dt=1./240.;let origin=[0.13,-0.27,0.19];
        let point=|x,y,z|add(origin,add(mul(u,x*scale),add(mul(v,y*scale),mul(n,z))));
        let points=[point(-1.,-1.,0.),point(1.,-1.,0.),point(1.,1.,0.),point(-1.,1.,0.)];
        for faces in [[[0,1,2],[0,2,3]],[[0,2,3],[0,1,2]]] {
            let mesh=TriangleMesh::new(&points,&faces).unwrap();
            for direction in [-1.,1.] {
                let mut rod=HairRod::new(vec![point(-3.,0.,radius),point(-2.,0.,radius),point(2.,0.,radius)],Default::default()).unwrap();
                let before=rod.x.clone();
                refresh_mesh_constraints(&mut rod,std::slice::from_ref(&mesh),radius);
                assert_eq!(rod.x,before,"contact discovery changed geometry");
                for t in [0.25,0.75] {
                    assert!(rod.contacts.iter().any(|c|c.segment==1 && (c.fraction-t).abs()<1e-12),"lost surface support at {t}, scale={scale}, normal={n:?}: {:?}",rod.contacts);
                }
                rod.velocity[1]=mul(n,direction);rod.velocity[2]=mul(n,-direction);
                stabilize_contact_velocities(std::slice::from_mut(&mut rod),&[],dt,radius).unwrap();
                for t in [0.25,0.75] {
                    let velocity=add(mul(rod.velocity[1],1.-t),mul(rod.velocity[2],t));
                    let position=add(mul(rod.x[1],1.-t),mul(rod.x[2],t));
                    let bound=(-(dot(sub(position,origin),n)-radius)/dt).min(0.);
                    assert!(dot(velocity,n)>=bound-1e-9,"lost velocity support at {t}, scale={scale}: {velocity:?}");
                }
            }
        }
      }
    }
}

#[test]
fn tessellated_contact_preserves_affine_surface_velocity_on_each_face() {
    let dt=1./240.;let radius=40e-6;
    let current=[[-1.,-1.,0.],[1.,-1.,0.],[1.,1.,0.],[-1.,1.,0.]];
    let mut previous=current;previous[3][2]=-4.*dt;
    for faces in [[[0,1,2],[0,2,3]],[[0,2,3],[0,1,2]]] {
        let mut mesh=TriangleMesh::new(&previous,&faces).unwrap();
        mesh.refit_with_timestep(&current,dt).unwrap();
        let mut rod=HairRod::new(vec![[-3.,0.,radius],[-2.,0.,radius],[2.,0.,radius]],Default::default()).unwrap();
        refresh_mesh_constraints(&mut rod,std::slice::from_ref(&mesh),radius);
        stabilize_contact_velocities(std::slice::from_mut(&mut rod),&[],dt,radius).unwrap();
        for (t,surface_speed) in [(0.25,2.),(0.5,0.),(0.75,0.)] {
            let velocity=add(mul(rod.velocity[1],1.-t),mul(rod.velocity[2],t));
            assert!(velocity[2]>=surface_speed-1e-9,"lost moving face at {t}: {velocity:?}, surface={surface_speed}");
        }
    }
}

#[test]
fn parallel_strand_contact_preserves_tip_velocity_support() {
    let radius=40e-6;let dt=1./240.;
    for (axis,normal) in [([0.,1.,0.],[1.,0.,0.]),([0.6,0.8,0.],[0.64,-0.48,0.6])] {
      for scale in [0.01,1.,100.] {
        for offset in [0.,0.005] {
          for reversed in [false,true] {
            let lengths=if reversed {[0.03,0.02,0.01]} else {[0.,0.01,0.02]};
            let points_a=[0.,0.01,0.02].map(|y|mul(axis,y*scale));
            let points_b=lengths.map(|y|add(mul(axis,(y+offset)*scale),mul(normal,2.*radius)));
            let mut rods=vec![HairRod::new(points_a.to_vec(),Default::default()).unwrap(),HairRod::new(points_b.to_vec(),Default::default()).unwrap()];
            let before:Vec<_>=rods.iter().map(|r|r.x.clone()).collect();
            let responses=refresh_strand_responses(&mut rods,radius,&[]);
            for (rod,positions) in rods.iter().zip(before) {assert_eq!(rod.x,positions);}
            rods[0].velocity[2]=normal;
            rods[1].velocity[1]=mul(normal,-1.);rods[1].velocity[2]=mul(normal,-1.);
            stabilize_contact_velocities(&mut rods,&responses,dt,radius).unwrap();
            // The support interval is known independently from the projected
            // witness list, including partial overlap and reversed segments.
            for y in [0.01+offset,0.02] {
                let a=(y-0.01)/0.01;
                let b=(y-offset-lengths[1])/(lengths[2]-lengths[1]);
                let va=add(mul(rods[0].velocity[1],1.-a),mul(rods[0].velocity[2],a));
                let vb=add(mul(rods[1].velocity[1],1.-b),mul(rods[1].velocity[2],b));
                let pa=add(mul(rods[0].x[1],1.-a),mul(rods[0].x[2],a));
                let pb=add(mul(rods[1].x[1],1.-b),mul(rods[1].x[2],b));
                let permitted=(len(sub(pa,pb))-2.*radius).max(0.)/dt;
                let closing=dot(sub(va,vb),normal);
                assert!(closing<=permitted+1e-9,"parallel support approaches unchecked: closing={closing}, scale={scale}, reversed={reversed}, offset={offset}");
            }
          }
        }
      }
    }
}

#[test]
fn strand_manifold_refresh_preserves_historical_pressure_without_duplication() {
    let radius=40e-6;
    let mut rods=vec![
        HairRod::new(vec![[0.,0.,0.],[0.,0.01,0.],[0.,0.02,0.]],Default::default()).unwrap(),
        HairRod::new(vec![[2.*radius,0.,0.],[2.*radius,0.01,0.],[2.*radius,0.02,0.]],Default::default()).unwrap(),
    ];
    let mut history=refresh_strand_responses(&mut rods,radius,&[]);
    for (i,response) in history.iter_mut().enumerate() {response.impulse=(i+1) as f64*1e-9;}
    let current=refresh_strand_responses(&mut rods,radius,&history);
    assert_eq!(current.len(),history.len());
    for (old,new) in history.iter().zip(&current) {
        assert_eq!((old.a,old.b),(new.a,new.b));
        assert!((old.impulse-new.impulse).abs()<=old.impulse*1e-12,"history load was copied between support points: old={} new={}",old.impulse,new.impulse);
    }
}

#[test]
fn coincident_parallel_contacts_use_transverse_material_direction() {
    for axis in [[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]] {
        let rod=HairRod::new([0.,0.01,0.02].map(|t|mul(axis,t)).to_vec(),Default::default()).unwrap();
        let mut rods=vec![rod.clone(),rod];
        let contacts=refresh_strand_responses(&mut rods,40e-6,&[]);
        assert!(!contacts.is_empty());
        for contact in contacts {
            assert!((len(contact.normal)-1.).abs()<1e-12);
            assert!(dot(contact.normal,axis).abs()<1e-12,"coincident contact pushed along the fibre: axis={axis:?}, normal={:?}",contact.normal);
        }
    }
}

#[test]
fn nonzero_subpicometer_strand_separation_retains_its_geometric_normal() {
    for separation in [1e-14,1e-200] {
        let mut rods=vec![
            HairRod::new(vec![[0.,0.,0.],[0.01,0.,0.],[0.02,0.,0.]],Default::default()).unwrap(),
            HairRod::new(vec![[0.,separation,0.],[0.01,separation,0.],[0.02,separation,0.]],Default::default()).unwrap(),
        ];
        let contacts=refresh_strand_responses(&mut rods,40e-6,&[]);
        assert!(!contacts.is_empty());
        for contact in contacts {assert!(dot(contact.normal,[0.,-1.,0.])>1.-1e-12,"nonzero separation lost its normal: {:?}",contact.normal);}
    }
}

#[test]
fn coincident_material_contact_direction_follows_rigid_rotation() {
    let rod=HairRod::new(vec![[0.,0.,0.],[0.01,0.,0.],[0.02,0.,0.]],Default::default()).unwrap();
    let mut baseline=vec![rod.clone(),rod];
    let expected=refresh_strand_responses(&mut baseline,40e-6,&[]);
    let rotation=exp([0.2,-0.4,0.3]);let translation=[0.13,-0.27,0.19];
    let mut transformed=baseline.clone();
    for rod in &mut transformed {
        for point in &mut rod.x {*point=add(rotate(rotation,*point),translation);}
        for point in &mut rod.old_x {*point=add(rotate(rotation,*point),translation);}
        for frame in &mut rod.q {*frame=qunit(qm(rotation,*frame));}
    }
    let actual=refresh_strand_responses(&mut transformed,40e-6,&[]);
    assert_eq!(actual.len(),expected.len());
    for (a,b) in actual.iter().zip(expected) {
        assert!(len(sub(a.normal,rotate(rotation,b.normal)))<1e-12,"contact direction did not follow the material frame");
    }
}

#[test]
fn coincident_strand_contact_retains_the_previous_side_of_separation() {
    for crossing in [false,true] {
        let a=vec![[-0.02,0.,0.],[-0.01,0.,0.],[0.01,0.,0.]];
        let b=if crossing {vec![[0.,-0.02,0.],[0.,-0.01,0.],[0.,0.01,0.]]} else {a.clone()};
        let mut rods=vec![HairRod::new(a,Default::default()).unwrap(),HairRod::new(b,Default::default()).unwrap()];
        for point in &mut rods[1].old_x {point[2]+=1e-4;}
        let contacts=refresh_strand_responses(&mut rods,40e-6,&[]);
        assert!(!contacts.is_empty());
        for contact in contacts {assert!(dot(contact.normal,[0.,0.,-1.])>1.-1e-12,"contact reversed the historical side: {:?}",contact.normal);}
    }
}

#[test]
fn coincident_free_segments_separate_transversely_without_axial_sliding() {
    let radius=40e-6;
    for axis in [[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]] {
        let rod=HairRod::new([0.,0.01,0.02].map(|t|mul(axis,t)).to_vec(),Default::default()).unwrap();
        let mut rods=vec![rod.clone(),rod];
        let before=rods.clone();
        let mut contacts=refresh_strand_responses(&mut rods,radius,&[]);
        assert!(reconcile_contact_positions(&mut rods,&mut contacts,1./240.,radius).unwrap());
        for (old,rod) in before.iter().zip(&rods) {
            assert_eq!(rod.x[0],old.x[0]);
            for (a,b) in rod.x.iter().zip(&old.x) {assert!(dot(sub(*a,*b),axis).abs()<1e-12,"contact escaped by sliding along the fibre");}
        }
        let (_,_,a,b)=segment_pair(rods[0].x[1],rods[0].x[2],rods[1].x[1],rods[1].x[2]);
        assert!(len(sub(a,b))>=2.*radius-1e-10,"free segments still overlap after normal projection");
    }
}

#[test]
fn interior_crossing_contact_normal_is_orthogonal_to_both_segments() {
    let u=[0.6,0.8,0.];let v=[-0.48,0.36,0.8];let n=[0.64,-0.48,0.6];let center=[0.13,-0.27,0.19];
    for separation in [0.,1e-14] {
        let mut rods=vec![
            HairRod::new([-0.04,-0.013,0.017].map(|t|add(center,mul(u,t))).to_vec(),Default::default()).unwrap(),
            HairRod::new([-0.04,-0.011,0.019].map(|t|add(add(center,mul(v,t)),mul(n,separation))).to_vec(),Default::default()).unwrap(),
        ];
        let contacts=refresh_strand_responses(&mut rods,40e-6,&[]);
        let interior=contacts.iter().find(|r|r.a.1==1 && r.b.1==1 && r.a.2>0. && r.a.2<1. && r.b.2>0. && r.b.2<1.).unwrap();
        assert!(dot(interior.normal,u).abs()<1e-12 && dot(interior.normal,v).abs()<1e-12,"closest-point rounding created a tangential normal at separation {separation}: {:?}",interior.normal);
    }
}

#[test]
fn translated_interior_strand_contact_has_no_tangential_force() {
    let u=[0.6,0.8,0.];let v=[-0.48,0.36,0.8];let n=[0.64,-0.48,0.6];
    for center in [[0.13,-0.27,0.19],[1000.,-2000.,3000.]] {
        let mut rods=vec![
            HairRod::new([-0.04,-0.013,0.017].map(|t|add(center,mul(u,t))).to_vec(),Default::default()).unwrap(),
            HairRod::new([-0.04,-0.011,0.019].map(|t|add(add(center,mul(v,t)),mul(n,40e-6))).to_vec(),Default::default()).unwrap(),
        ];
        let contacts=refresh_strand_responses(&mut rods,40e-6,&[]);
        let contact=contacts.iter().find(|r|r.a.1==1 && r.b.1==1 && r.a.2>0. && r.a.2<1. && r.b.2>0. && r.b.2<1.).unwrap();
        for rod in &rods {
            let tangent=unit(sub(rod.x[2],rod.x[1]));
            assert!(dot(contact.normal,tangent).abs()<1e-12,"translated closest-point subtraction introduces tangential force: center={center:?}, normal={:?}, tangent={tangent:?}",contact.normal);
        }
        assert!(dot(contact.normal,n)< -0.999999,"contact normal reversed its geometric side");
    }
}

#[test]
fn translated_face_contacts_do_not_push_along_the_surface() {
    let u=[0.6,0.8,0.];let v=[-0.48,0.36,0.8];let n=[0.64,-0.48,0.6];
    for center in [[0.13,-0.27,0.19],[1000.,-2000.,3000.]] {
        let points=[add(center,mul(u,-0.1)),add(center,mul(u,0.1)),add(center,mul(v,0.1))];
        let mesh=TriangleMesh::new(&points,&[[0,1,2]]).unwrap();
        let face_normal=mesh.triangles[0].normal;
        let edge=unit(sub(points[1],points[0]));
        for gap in [-20e-6,20e-6] {
            let mut rod=HairRod::new([-0.01,0.,0.01].map(|t|add(add(add(center,mul(v,0.03)),mul(u,t)),mul(n,gap))).to_vec(),Default::default()).unwrap();
            assert_eq!(closest_triangle_feature(rod.x[1],&mesh.triangles[0]).1,ClosestFeature::Face);
            let before=rod.x.clone();
            refresh_mesh_constraints(&mut rod,std::slice::from_ref(&mesh),40e-6);
            assert_eq!(rod.x,before,"discovery changed the model");
            assert!(!rod.contacts.is_empty());
            for contact in &rod.contacts {
                assert!(dot(contact.normal,edge).abs()<1e-12,"translated face query created tangential contact: center={center:?}, normal={:?}",contact.normal);
                assert!(dot(contact.normal,face_normal)>1.-1e-12,"face recovery reversed the outward direction");
            }
        }
    }
}

#[test]
fn triangle_feature_gradient_matches_face_edge_and_vertex_distance_derivatives() {
    let mesh=TriangleMesh::new(&[[0.,0.,0.],[1.,0.,0.],[0.,1.,0.]],&[[0,1,2]]).unwrap();
    let tri=&mesh.triangles[0];
    for (point,feature,side) in [
        ([0.2,0.2,0.1],ClosestFeature::Face,1.),
        ([0.2,0.2,-0.1],ClosestFeature::Face,-1.),
        ([0.5,-0.2,0.1],ClosestFeature::Edge(0,1),1.),
        ([-0.2,-0.3,0.1],ClosestFeature::Vertex(0),1.),
    ] {
        let (closest,actual)=closest_triangle_feature(point,tri);assert_eq!(actual,feature);
        let signed=side*len(sub(point,closest));
        let gradient=triangle_distance_gradient(tri,point,closest,signed,tri.normal,1e-12);
        for axis in 0..3 {
            let dt=1e-6;let mut lo=point;let mut hi=point;lo[axis]-=dt;hi[axis]+=dt;
            let distance=|p|side*len(sub(p,closest_triangle(p,tri)));
            let derivative=(distance(hi)-distance(lo))/(2.*dt);
            assert!((gradient[axis]-derivative).abs()<1e-9,"feature metric derivative changed: {feature:?}, axis={axis}");
        }
    }
}
