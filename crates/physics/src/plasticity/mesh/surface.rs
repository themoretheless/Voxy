//! Current external and fully fractured tetrahedral surfaces.
use super::{Body, Vec3, cross, dot, sub};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug)]
pub struct SurfaceFace {
    /// Winding follows the outward normal of the owning cell.
    pub nodes: [usize; 3],
    pub cell: usize,
    pub normal: Vec3,
    pub area_m2: f64,
}
impl Body {
    /// Topological external surfaces plus both sides of fully broken interfaces.
    /// Partially bonded interfaces remain internal. This does not determine fluid
    /// access, occlusion, contact closure or self-intersection of crack surfaces.
    /// Geometry is sampled at accepted current positions.
    /// # Errors
    /// Nonmanifold mesh faces, degenerate geometry or diagnostic overflow.
    pub fn exposed_faces(&self) -> Result<Vec<SurfaceFace>, &'static str> {
        let mut hidden = BTreeSet::new();
        for interface in self.interface_reports()? {
            if interface.quadrature.iter().any(|q| q.damage < 1.) {
                for mut nodes in [interface.minus, interface.plus] {
                    nodes.sort_unstable();
                    hidden.insert(nodes);
                }
            }
        }
        let mut faces = BTreeMap::<[usize; 3], Vec<(usize, usize)>>::new();
        for (cell, element) in self.elements.iter().enumerate() {
            for opposite in 0..4 {
                let mut nodes = [0; 3];
                let mut index = 0;
                for (local, &node) in element.nodes.iter().enumerate() {
                    if local != opposite {
                        nodes[index] = node;
                        index += 1;
                    }
                }
                nodes.sort_unstable();
                let owners = faces.entry(nodes).or_default();
                owners.push((cell, element.nodes[opposite]));
                if owners.len() > 2 {
                    return Err("nonmanifold solid surface");
                }
            }
        }
        let mut result = Vec::new();
        for (mut nodes, owners) in faces {
            if owners.len() != 1 || hidden.contains(&nodes) {
                continue;
            }
            let (cell, opposite) = owners[0];
            let [a, b, c] = nodes.map(|n| self.positions[n]);
            let edges = [sub(b, a), sub(c, a)];
            let scale = edges.iter().flatten().fold(0_f64, |m, v| m.max(v.abs()));
            if !scale.is_finite() || scale <= 0. {
                return Err("invalid surface scale");
            }
            let product = cross(edges[0].map(|v| v / scale), edges[1].map(|v| v / scale));
            let norm = product.iter().fold(0_f64, |m, v| m.hypot(*v));
            let area = 0.5 * norm * scale * scale;
            if norm <= 64. * f64::EPSILON || !area.is_finite() || area <= 0. {
                return Err("degenerate solid surface");
            }
            let mut normal = product.map(|v| v / norm);
            let side = dot(sub(self.positions[opposite], a).map(|v| v / scale), normal);
            if !side.is_finite() || side.abs() <= 64. * f64::EPSILON {
                return Err("invalid surface orientation");
            }
            if side > 0. {
                normal = normal.map(|v| -v);
                nodes.swap(1, 2);
            }
            result.push(SurfaceFace {
                nodes,
                cell,
                normal,
                area_m2: area,
            });
        }
        Ok(result)
    }
    /// Integrate constant pressure per exposed triangle, in `exposed_faces` order.
    /// Positive pressure acts inward; negative values give outward suction.
    /// Uses current geometry and equal linear shape-function weights, area/3.
    /// These are sampled loads: the equilibrium solver does not differentiate
    /// follower-pressure geometry. Recompute at the desired sampling time.
    /// # Errors
    /// Invalid pressure count, nonfinite values or force overflow.
    pub fn pressure_loads(&self, pressure_pa: &[f64]) -> Result<Vec<Vec3>, &'static str> {
        let faces = self.exposed_faces()?;
        if pressure_pa.len() != faces.len() || pressure_pa.iter().any(|p| !p.is_finite()) {
            return Err("invalid exposed surface pressure");
        }
        let mut loads = vec![[0.; 3]; self.positions.len()];
        for (face, &pressure) in faces.iter().zip(pressure_pa) {
            let traction = face.normal.map(|n| -n * pressure * (face.area_m2 / 3.));
            for &node in &face.nodes {
                for (force, component) in loads[node].iter_mut().zip(traction) {
                    *force += component;
                }
            }
        }
        if loads.iter().flatten().any(|f| !f.is_finite()) {
            return Err("pressure load overflow");
        }
        Ok(loads)
    }
}
