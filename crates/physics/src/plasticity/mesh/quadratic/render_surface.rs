//! Current curved T6 surface triangles for rendering accepted FEM geometry.
use super::{QuadraticBody, QuadraticFace, Vec3};
#[derive(Clone, Copy, Debug)]
pub struct QuadraticSurfaceVertex {
    pub position_m: Vec3,
    pub normal: Vec3,
    pub barycentric: Vec3,
}
#[derive(Clone, Debug)]
pub struct QuadraticSurfaceTriangle {
    pub vertices: [QuadraticSurfaceVertex; 3],
    pub face: QuadraticFace,
    pub component: usize,
}
impl QuadraticBody {
    /// Tessellate currently exposed T6 faces with analytic current normals.
    /// Geometry uses the same interpolation and exposure rule as contact queries.
    /// Uniform four-way subdivision; max_triangles is checked before allocation.
    /// This is a render approximation, not a collision surface replacement.
    pub fn surface_triangles(
        &self,
        subdivision_depth: u8,
        max_triangles: usize,
    ) -> Result<Vec<QuadraticSurfaceTriangle>, &'static str> {
        if subdivision_depth > 8 || max_triangles == 0 {
            return Err("invalid quadratic render surface limits");
        }
        let faces = self.exposed_faces_at(&self.positions)?;
        let count = faces
            .len()
            .checked_mul(4_usize.pow(u32::from(subdivision_depth)))
            .ok_or("quadratic render surface count overflow")?;
        if count > max_triangles {
            return Err("quadratic render surface triangle limit");
        }
        let mut owner = vec![usize::MAX; self.positions.len()];
        for (i, nodes) in self.fragment_nodes().iter().enumerate() {
            for &n in nodes {
                owner[n] = i;
            }
        }
        let mut patches = vec![[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]];
        let midpoint = |a: Vec3, b: Vec3| std::array::from_fn(|i| a[i].midpoint(b[i]));
        for _ in 0..subdivision_depth {
            patches = patches
                .into_iter()
                .flat_map(|[a, b, c]| {
                    let ab = midpoint(a, b);
                    let bc = midpoint(b, c);
                    let ac = midpoint(a, c);
                    [[a, ab, ac], [ab, b, bc], [ac, bc, c], [ab, bc, ac]]
                })
                .collect();
        }
        let mut result = Vec::with_capacity(count);
        for face in faces {
            let origin = self.positions[face.nodes[0]];
            let points: [Vec3; 6] = face
                .nodes
                .map(|n| std::array::from_fn(|a| self.positions[n][a] - origin[a]));
            let vertex = |barycentric: Vec3| -> Result<QuadraticSurfaceVertex, &'static str> {
                let (shape, du, dv) = super::cohesive::basis(barycentric);
                let interpolate = |w: [f64; 6]| -> Vec3 {
                    std::array::from_fn(|a| points.iter().zip(w).map(|(p, w)| p[a] * w).sum())
                };
                let position_m = std::array::from_fn(|a| origin[a] + interpolate(shape)[a]);
                let u = interpolate(du);
                let v = interpolate(dv);
                let norm = |p: Vec3| p[0].hypot(p[1]).hypot(p[2]);
                let scale = norm(u).max(norm(v));
                let cross =
                    crate::plasticity::mesh::cross(u.map(|x| x / scale), v.map(|x| x / scale));
                let length = norm(cross);
                if !length.is_finite()
                    || length <= 64. * f64::EPSILON
                    || position_m.iter().any(|x| !x.is_finite())
                {
                    return Err("singular quadratic render surface geometry");
                }
                Ok(QuadraticSurfaceVertex {
                    position_m,
                    normal: cross.map(|x| x / length),
                    barycentric,
                })
            };
            for patch in &patches {
                result.push(QuadraticSurfaceTriangle {
                    vertices: [vertex(patch[0])?, vertex(patch[1])?, vertex(patch[2])?],
                    component: owner[face.nodes[0]],
                    face,
                });
            }
        }
        Ok(result)
    }
}
