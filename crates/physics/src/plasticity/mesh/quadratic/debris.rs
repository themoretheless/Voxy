//! Rigid debris extraction from a fractured quadratic body: each connected
//! fragment becomes a convex piece with consistent-mass centre, inertia,
//! momentum and spin. The deformable body is not mutated; the caller decides
//! which pieces leave the continuum and which nodes stay.

use super::fragments::{project_rotation, summarize};
use super::{QuadraticBody, Vec3, dot};
use crate::math::{cross, norm, sub};

/// One rigid piece released from a fractured body.
#[derive(Clone, Debug)]
pub struct DebrisPiece {
    pub nodes: Vec<usize>,
    /// Convex hull vertex positions, in the body's frame.
    pub hull: Vec<Vec3>,
    /// Outward-facing hull triangles indexing `hull`.
    pub hull_faces: Vec<[usize; 3]>,
    pub mass_kg: f64,
    /// Full inertia tensor about `center_m`.
    pub inertia_kg_m2: [[f64; 3]; 3],
    pub center_m: Vec3,
    pub velocity_m_s: Vec3,
    pub angular_velocity_rad_s: Vec3,
    /// Rigid translational plus rotational kinetic energy carried by the piece.
    pub kinetic_j: f64,
    /// Residual deformable kinetic energy not representable rigidly.
    pub deformation_kinetic_j: f64,
}

/// Outcome of extracting rigid pieces from a body.
#[derive(Clone, Debug)]
pub struct DebrisRelease {
    pub pieces: Vec<DebrisPiece>,
    /// Nodes of fragments below the mass threshold, left to the continuum.
    pub retained_nodes: Vec<usize>,
    /// Sum of rigid kinetic energy across released pieces.
    pub kinetic_j: f64,
    /// Sum of residual deformable kinetic energy across released pieces.
    pub deformation_kinetic_j: f64,
    /// Area of fully broken cohesive faces separating released pieces.
    pub surface_created_m2: f64,
}

impl QuadraticBody {
    /// Extracts rigid pieces for fragments with mass at least `min_mass_kg`.
    /// Mass, linear momentum and angular momentum of every released piece
    /// equal the consistent-mass fragment integrals exactly.
    /// # Errors
    /// Invalid densities/velocities, singular fragment inertia, degenerate
    /// hull geometry or a nonfinite threshold.
    pub fn release_debris(
        &self,
        densities: &[f64],
        velocities: &[Vec3],
        min_mass_kg: f64,
    ) -> Result<DebrisRelease, &'static str> {
        if !min_mass_kg.is_finite() || min_mass_kg < 0. {
            return Err("invalid debris mass threshold");
        }
        if velocities.len() != self.positions.len()
            || velocities.iter().flatten().any(|v| !v.is_finite())
        {
            return Err("invalid debris velocities");
        }
        let mass = self.consistent_mass(densities)?;
        let groups = self.fragment_nodes();
        let mut piece_of_node = vec![usize::MAX; self.positions.len()];
        let mut pieces = Vec::new();
        let mut retained_nodes = Vec::new();
        for nodes in groups {
            let fragment = summarize(nodes, &mass, &self.positions, velocities)?;
            if fragment.mass_kg < min_mass_kg {
                retained_nodes.extend(fragment.nodes);
                continue;
            }
            let rotation = project_rotation(fragment, &mass, &self.positions, velocities)?;
            let points: Vec<Vec3> = rotation
                .fragment
                .nodes
                .iter()
                .map(|&i| self.positions[i])
                .collect();
            let (hull, hull_faces) = convex_hull(&points)?;
            for &node in &rotation.fragment.nodes {
                piece_of_node[node] = pieces.len();
            }
            pieces.push(DebrisPiece {
                nodes: rotation.fragment.nodes,
                hull,
                hull_faces,
                mass_kg: rotation.fragment.mass_kg,
                inertia_kg_m2: rotation.inertia_kg_m2,
                center_m: rotation.fragment.center_m,
                velocity_m_s: rotation.fragment.velocity_m_s,
                angular_velocity_rad_s: rotation.angular_velocity_rad_s,
                kinetic_j: rotation.translation_kinetic_j + rotation.rotation_kinetic_j,
                deformation_kinetic_j: rotation.deformation_kinetic_j,
            });
        }
        let mut surface_created_m2 = 0.;
        for face in &self.interfaces {
            if !face.is_fully_broken() {
                continue;
            }
            let (minus, plus) = face.sides();
            let a = piece_of_node[minus[0]];
            let b = piece_of_node[plus[0]];
            if a != usize::MAX && b != usize::MAX && a != b {
                surface_created_m2 += face.area_m2();
            }
        }
        let kinetic_j = pieces.iter().map(|p| p.kinetic_j).sum::<f64>();
        let deformation_kinetic_j = pieces.iter().map(|p| p.deformation_kinetic_j).sum::<f64>();
        if !kinetic_j.is_finite()
            || !deformation_kinetic_j.is_finite()
            || !surface_created_m2.is_finite()
        {
            return Err("debris release overflow");
        }
        Ok(DebrisRelease {
            pieces,
            retained_nodes,
            kinetic_j,
            deformation_kinetic_j,
            surface_created_m2,
        })
    }
}

/// Incremental convex hull. Returns unique hull vertices and outward CCW
/// triangles. Coplanar or degenerate inputs are rejected.
/// # Errors
/// Fewer than four non-coplanar points or nonfinite coordinates.
pub fn convex_hull(points: &[Vec3]) -> Result<(Vec<Vec3>, Vec<[usize; 3]>), &'static str> {
    if points.len() < 4 || points.iter().flatten().any(|v| !v.is_finite()) {
        return Err("convex hull needs at least four finite points");
    }
    let scale = points
        .iter()
        .flatten()
        .fold(0_f64, |m, v| m.max(v.abs()))
        .max(f64::MIN_POSITIVE);
    let eps = 1e-10 * scale;
    // Initial tetrahedron: extreme pair, farthest from the line, farthest from the plane.
    let (mut a, mut b) = (0, 0);
    let mut best = 0.;
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            let d = norm(sub(points[i], points[j]));
            if d > best {
                best = d;
                a = i;
                b = j;
            }
        }
    }
    if best <= eps {
        return Err("convex hull points coincide");
    }
    let ab = sub(points[b], points[a]);
    let mut c = usize::MAX;
    best = 0.;
    for (i, p) in points.iter().enumerate() {
        let d = norm(cross(ab, sub(*p, points[a])));
        if d > best {
            best = d;
            c = i;
        }
    }
    if c == usize::MAX || best <= eps * norm(ab) {
        return Err("convex hull points are collinear");
    }
    let normal = cross(ab, sub(points[c], points[a]));
    let mut d = usize::MAX;
    best = 0.;
    for (i, p) in points.iter().enumerate() {
        let h = dot(normal, sub(*p, points[a])).abs();
        if h > best {
            best = h;
            d = i;
        }
    }
    if d == usize::MAX || best <= eps * norm(normal) {
        return Err("convex hull points are coplanar");
    }
    let mut faces: Vec<[usize; 3]> = if dot(normal, sub(points[d], points[a])) > 0. {
        vec![[a, c, b], [a, b, d], [b, c, d], [c, a, d]]
    } else {
        vec![[a, b, c], [a, d, b], [b, d, c], [c, d, a]]
    };
    let outward = |face: &[usize; 3], p: Vec3| -> f64 {
        let n = cross(
            sub(points[face[1]], points[face[0]]),
            sub(points[face[2]], points[face[0]]),
        );
        dot(n, sub(p, points[face[0]])) / norm(n).max(f64::MIN_POSITIVE)
    };
    for (i, p) in points.iter().enumerate() {
        if [a, b, c, d].contains(&i) {
            continue;
        }
        let visible: Vec<usize> = (0..faces.len())
            .filter(|&f| outward(&faces[f], *p) > eps)
            .collect();
        if visible.is_empty() {
            continue;
        }
        // Horizon edges appear in exactly one visible face.
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for &f in &visible {
            let [x, y, z] = faces[f];
            for e in [(x, y), (y, z), (z, x)] {
                if let Some(pos) = edges.iter().position(|&(u, v)| (u, v) == (e.1, e.0)) {
                    edges.swap_remove(pos);
                } else {
                    edges.push(e);
                }
            }
        }
        let mut keep: Vec<[usize; 3]> = faces
            .iter()
            .enumerate()
            .filter(|(f, _)| !visible.contains(f))
            .map(|(_, face)| *face)
            .collect();
        for (u, v) in edges {
            keep.push([u, v, i]);
        }
        faces = keep;
    }
    let mut used: Vec<usize> = faces.iter().flatten().copied().collect();
    used.sort_unstable();
    used.dedup();
    let remap = |index: usize| used.binary_search(&index).unwrap_or(0);
    let hull: Vec<Vec3> = used.iter().map(|&i| points[i]).collect();
    let hull_faces: Vec<[usize; 3]> = faces.iter().map(|f| f.map(remap)).collect();
    Ok((hull, hull_faces))
}

/// Signed distance of `point` to the hull surface (negative inside), using
/// the maximum face-plane distance; exact outside only near faces.
#[must_use]
pub fn hull_signed_distance(hull: &[Vec3], faces: &[[usize; 3]], point: Vec3) -> f64 {
    faces
        .iter()
        .map(|&[a, b, c]| {
            let n = cross(sub(hull[b], hull[a]), sub(hull[c], hull[a]));
            dot(n, sub(point, hull[a])) / norm(n).max(f64::MIN_POSITIVE)
        })
        .fold(f64::NEG_INFINITY, f64::max)
}
