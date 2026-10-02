//! Consistent constant dead traction on straight reference faces.
use super::{QuadraticBody, Vec3, dot, sub};
use crate::plasticity::mesh::cross;
#[derive(Clone, Copy, Debug)]
pub struct QuadraticFace {
    /// Outward-wound corners followed by edge midpoints 01,12,02.
    pub nodes: [usize; 6],
    pub normal: Vec3,
    pub reference_area_m2: f64,
}
impl QuadraticBody {
    /// Reference boundary topology/geometry. This is not a curved-current-surface
    /// or fluid-access query. Shared interior faces are excluded.
    /// # Errors
    /// Nonmanifold faces, invalid topology, degenerate geometry or overflow.
    pub fn reference_faces(&self) -> Result<Vec<QuadraticFace>, &'static str> {
        let edges: std::collections::BTreeMap<_, _> = self.edge_midpoints().into_iter().collect();
        let mut faces = std::collections::BTreeMap::<[usize; 3], Vec<usize>>::new();
        for cell in &self.cells {
            for opposite in 0..4 {
                let mut face = [0; 3];
                let mut i = 0;
                for (local, &node) in cell.nodes[..4].iter().enumerate() {
                    if local != opposite {
                        face[i] = node;
                        i += 1;
                    }
                }
                face.sort_unstable();
                let owners = faces.entry(face).or_default();
                owners.push(cell.nodes[opposite]);
                if owners.len() > 2 {
                    return Err("nonmanifold quadratic boundary face");
                }
            }
        }
        let mut result = Vec::new();
        for (mut face, owners) in faces {
            let [a, b, c] = face.map(|n| self.rest[n]);
            let vectors = [sub(b, a), sub(c, a)];
            let scale = vectors.iter().flatten().fold(0_f64, |m, v| m.max(v.abs()));
            if !scale.is_finite() || scale <= 0. {
                return Err("invalid quadratic surface scale");
            }
            let product = cross(vectors[0].map(|v| v / scale), vectors[1].map(|v| v / scale));
            let norm = product.iter().fold(0_f64, |m, v| m.hypot(*v));
            let area = 0.5 * norm * scale * scale;
            if !area.is_finite() || area <= 0. || norm <= 64. * f64::EPSILON {
                return Err("invalid quadratic reference face");
            }
            let mut normal = product.map(|v| v / norm);
            let side = dot(normal, sub(self.rest[owners[0]], a).map(|v| v / scale));
            if !side.is_finite() || side.abs() <= 64. * f64::EPSILON {
                return Err("invalid quadratic face owner");
            }
            if owners.len() == 2 {
                let other = dot(normal, sub(self.rest[owners[1]], a).map(|v| v / scale));
                if !other.is_finite()
                    || other.abs() <= 64. * f64::EPSILON
                    || side.is_sign_positive() == other.is_sign_positive()
                {
                    return Err("overlapping quadratic face neighbors");
                }
                continue;
            }
            if side > 0. {
                face.swap(1, 2);
                normal = normal.map(|v| -v);
            }
            let mut nodes = [0; 6];
            nodes[..3].copy_from_slice(&face);
            for (edge, (i, j)) in [(0, 1), (1, 2), (0, 2)].into_iter().enumerate() {
                let key = [face[i].min(face[j]), face[i].max(face[j])];
                nodes[3 + edge] = *edges.get(&key).ok_or("missing quadratic face midpoint")?;
            }
            result.push(QuadraticFace {
                nodes,
                normal,
                reference_area_m2: area,
            });
        }
        Ok(result)
    }
    /// Constant nominal traction vectors in N/m² per `reference_faces` triangle.
    /// Forces remain fixed spatial vectors integrated over reference area, not
    /// follower pressure. Exact face integration gives zero corner forces and
    /// area/3 times traction at each midpoint.
    /// # Errors
    /// Invalid count/nonfinite traction, invalid surface or force overflow.
    pub fn traction_loads(&self, traction: &[Vec3]) -> Result<Vec<Vec3>, &'static str> {
        let faces = self.reference_faces()?;
        if traction.len() != faces.len() || traction.iter().flatten().any(|t| !t.is_finite()) {
            return Err("invalid quadratic traction");
        }
        let mut loads = vec![[0.; 3]; self.rest.len()];
        for (face, t) in faces.iter().zip(traction) {
            for &node in &face.nodes[3..] {
                for (axis, &value) in t.iter().enumerate() {
                    loads[node][axis] += value * (face.reference_area_m2 / 3.);
                }
            }
        }
        if loads.iter().flatten().any(|f| !f.is_finite()) {
            return Err("quadratic traction overflow");
        }
        Ok(loads)
    }
}
