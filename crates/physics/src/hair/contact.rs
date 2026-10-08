use super::{HairRod, ContactSource, math::*};
use std::collections::HashMap;
#[path = "contact_features.rs"]
mod features;
#[path = "velocity_contacts.rs"]
mod velocity_contacts;
pub(super) use velocity_contacts::{stabilize_contact_velocities,reconcile_contact_positions};
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
    feature_normals: Option<features::FeatureNormals>,
    motion_dt: Option<f64>,
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
pub(super) struct StrandResponse {
    a: (usize, usize, f64),
    b: (usize, usize, f64),
    normal: V,
    impulse: f64,
}
pub(super) fn finish_strand_contacts(rods: &mut [HairRod], responses: &[StrandResponse], dt: f64) {
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
pub(super) fn mesh_contacts(rod: &mut HairRod, meshes: &[TriangleMesh], radius: f64) {
    gather_mesh_contacts(rod,meshes,radius,true);
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
                let normal=distance_gradient(delta,distance,signed,contact_normal,1e-12);
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
            for &id in &candidates {
                let tri = &mesh.triangles[id];
                let (t, p, q) = segment_triangle(a, b, tri);
                // Follicle overlap is prescribed. Do not let an allowed root
                // contact hide a later collision on the free part of the strand.
                if i==0 && t<0.15 {continue;}
                let contact_normal = mesh.contact_normal(id, p);
                let delta = sub(p, q);
                let distance = len(delta);
                let signed = if dot(delta, contact_normal) < 0. {
                    -distance
                } else {
                    distance
                };
                if distance > query_radius {
                    continue;
                }
                // Choose nearest surface, not every tessellation face independently.
                let normal = distance_gradient(delta,distance,signed,contact_normal,1e-10);
                if contact.as_ref().is_none_or(|c| segment_contact_precedes(distance,t,tri.ids,c.0,c.1,c.4,len(sub(b,a)))) {
                    contact = Some((distance,t,normal,signed,tri.ids,mesh.surface_velocity(id,q)));
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
    let mut pairs = Vec::new();
    for (&key, entries) in grid {
        for a in 0..entries.len() {
            for b in a + 1..entries.len() {
                let owner = std::array::from_fn(|axis| entries[a].1[axis].max(entries[b].1[axis]));
                if cell_key(owner) != key { continue; }
                let mut pair = (entries[a].0, entries[b].0);
                if pair.0 > pair.1 { pair = (pair.1, pair.0); }
                if pair.0.0 == pair.1.0 && pair.0.1.abs_diff(pair.1.1) <= 2 { continue; }
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
/// Re-query the current geometry without moving it. Historical projections supply
/// friction load only; old normals and released pairs must not constrain velocity.
pub(super) fn refresh_strand_responses(rods:&mut [HairRod],radius:f64,history:&[StrandResponse])->Vec<StrandResponse> {
    let mut loads:std::collections::BTreeMap<((usize,usize),(usize,usize)),Vec<(V,f64)>>=std::collections::BTreeMap::new();
    for response in history {
        loads.entry(((response.a.0,response.a.1),(response.b.0,response.b.1))).or_default().push((response.normal,response.impulse));
    }
    let mut current=gather_strand_contacts(rods,radius,false);
    for response in &mut current {
        response.impulse=loads.get(&((response.a.0,response.a.1),(response.b.0,response.b.1)))
            .map_or(0.,|loads|loads.iter().map(|(normal,impulse)|dot(*normal,response.normal).max(0.)*impulse).sum());
    }
    current
}
fn gather_strand_contacts(rods: &mut [HairRod], radius: f64, project_positions:bool) -> Vec<StrandResponse> {
    let mut responses=Vec::new();
    const CONTACT_TOLERANCE: f64 = 1e-10;
    let query_radius = radius + CONTACT_TOLERANCE * 0.5;
    for rod in rods.iter_mut() {rod.contacts.retain(|contact| matches!(contact.source,ContactSource::Mesh(_)));}
    // Segment AABBs, not just particles, enter the spatial hash. Adjacent segments are excluded.
    let cell = 0.008f64.max(radius * 4.);
    let mut grid: ContactGrid = HashMap::new();
    for (r, rod) in rods.iter().enumerate() {
        for (i, p) in rod.x.windows(2).enumerate() {
            let min: [i32; 3] =
                std::array::from_fn(|a| ((p[0][a].min(p[1][a]) - query_radius) / cell).floor() as i32);
            let max: [i32; 3] =
                std::array::from_fn(|a| ((p[0][a].max(p[1][a]) + query_radius) / cell).floor() as i32);
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
        let query_separation = separation + CONTACT_TOLERANCE;
        // Grid cells are broader than a fibre. Reject disjoint current capsule
        // bounds before the more expensive segment-distance calculation.
        if (0..3).any(|axis| {
            let a0 = rods[ra].x[ia][axis];
            let a1 = rods[ra].x[ia + 1][axis];
            let b0 = rods[rb].x[ib][axis];
            let b1 = rods[rb].x[ib + 1][axis];
            a0.min(a1) - query_separation > b0.max(b1) || b0.min(b1) - query_separation > a0.max(a1)
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
        if distance > query_separation {
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
    responses
}

#[cfg(test)]
mod query_tests {
    use super::*;
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
    fn current_strand_manifold_combines_projection_load_without_duplicate_velocity_constraints() {
        let make=|x|HairRod::new(vec![[x,0.,0.],[x,0.01,0.],[x,0.02,0.]],super::super::HairMaterial::default()).unwrap();
        let mut rods=vec![make(0.),make(80e-6)];
        let history=[
            StrandResponse {a:(0,1,0.2),b:(1,1,0.2),normal:[-1.,0.,0.],impulse:1e-6},
            StrandResponse {a:(0,1,0.8),b:(1,1,0.8),normal:[-1.,0.,0.],impulse:2e-6},
        ];
        let current=refresh_strand_responses(&mut rods,40e-6,&history);
        let pair=current.iter().filter(|r|r.a.1==1 && r.b.1==1).collect::<Vec<_>>();
        assert_eq!(pair.len(),1);
        assert!((pair[0].impulse-3e-6).abs()<1e-20);
        assert!(len(sub(pair[0].normal,[-1.,0.,0.]))<1e-15);
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
