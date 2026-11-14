//! Rounded geometry shared by continuum FEM and compliant tissue dynamics.
use super::super::{Vec3, columns, det, sub};
use super::TetraMesh;
use std::collections::BTreeMap;
impl TetraMesh {
    /// Inscribed ellipsoid with conforming radial tetrahedra, in metres.
    /// The center is node 0; nodes 1..=6 remain +X, -X, +Y, -Y, +Z, -Z.
    /// Geometry contains no mass, material, pins, or simulation state.
    /// # Errors
    /// Rejects nonfinite dimensions, nonpositive radii, refinement above three,
    /// overflow, and cells outside the existing mesh validity bounds.
    pub fn ellipsoid(center: Vec3, radii: Vec3, refinement: u32) -> Result<Self, &'static str> {
        if center.iter().any(|v| !v.is_finite())
            || radii.iter().any(|r| !r.is_finite() || *r <= 0.)
            || refinement > 3
        {
            return Err("invalid tetrahedral ellipsoid");
        }
        let mut unit = vec![
            [0.; 3],
            [1., 0., 0.],
            [-1., 0., 0.],
            [0., 1., 0.],
            [0., -1., 0.],
            [0., 0., 1.],
            [0., 0., -1.],
        ];
        let mut faces = Vec::new();
        for a in [1, 2] {
            for b in [3, 4] {
                for c in [5, 6] {
                    faces.push([a, b, c]);
                }
            }
        }
        for _ in 0..refinement {
            let mut midpoint = BTreeMap::new();
            let mut refined = Vec::with_capacity(faces.len() * 4);
            for [a, b, c] in faces {
                let mut edge = |a: usize, b: usize| {
                    *midpoint.entry((a.min(b), a.max(b))).or_insert_with(|| {
                        let p: Vec3 = std::array::from_fn(|axis| unit[a][axis] + unit[b][axis]);
                        let length = p.iter().map(|v| v * v).sum::<f64>().sqrt();
                        let index = unit.len();
                        unit.push(p.map(|v| v / length));
                        index
                    })
                };
                let (ab, bc, ca) = (edge(a, b), edge(b, c), edge(c, a));
                refined.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
            }
            faces = refined;
        }
        let positions: Vec<Vec3> = unit
            .iter()
            .map(|p| std::array::from_fn(|axis| center[axis] + radii[axis] * p[axis]))
            .collect();
        if positions.iter().flatten().any(|v| !v.is_finite()) {
            return Err("tissue ellipsoid geometry overflow");
        }
        let mut cells: Vec<_> = faces.into_iter().map(|[a, b, c]| [0, a, b, c]).collect();

        for cell in &mut cells {
            let [a, b, c, d] = cell.map(|i| positions[i]);
            if det(columns(sub(b, a), sub(c, a), sub(d, a))) < 0. {
                cell.swap(1, 2);
            }
        }
        let boundary = cells.iter().map(|&[_, b, c, d]| [b, c, d]).collect();
        let mesh = Self {
            points: positions,
            cells,
            boundary,
        };
        mesh.validate_topology()?;
        Ok(mesh)
    }
}
