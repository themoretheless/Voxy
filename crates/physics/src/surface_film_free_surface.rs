//! Collision snapshot of the reconstructed upper film surface.
use super::{SphereFilmHit, SurfaceFilm, norm};

/// Frozen geometric reconstruction; cell indices match the source film.
/// No liquid inventory is owned here. Rebuild after transport or deposition.
#[derive(Debug)]
pub struct FilmFreeSurface {
    geometry: SurfaceFilm,
    points: Vec<[f64; 3]>,
    substrate: Vec<[[f64; 3]; 3]>,
    side: f64,
}
fn determinant(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0])
}
fn inside_tetrahedron(point: [f64; 3], tetrahedron: [[f64; 3]; 4]) -> Result<bool, &'static str> {
    let edges: [[f64; 3]; 3] =
        std::array::from_fn(|i| super::sub(tetrahedron[i + 1], tetrahedron[0]));
    let relative = super::sub(point, tetrahedron[0]);
    let scale = edges.iter().flatten().fold(0.0_f64, |s, v| s.max(v.abs()));
    if scale == 0.0 {
        return Ok(false);
    }
    let edges = edges.map(|e| e.map(|v| v / scale));
    let relative = relative.map(|v| v / scale);
    if relative.iter().any(|v| !v.is_finite()) {
        return Err("film immersion coordinate overflow");
    }
    let det = determinant(edges[0], edges[1], edges[2]);
    if det == 0.0 {
        return Ok(false);
    }
    let mut weights = [
        determinant(relative, edges[1], edges[2]) / det,
        determinant(edges[0], relative, edges[2]) / det,
        determinant(edges[0], edges[1], relative) / det,
        0.0,
    ];
    weights[3] = 1.0 - weights[..3].iter().sum::<f64>();
    if weights.iter().any(|v| !v.is_finite()) {
        return Err("film immersion barycentric overflow");
    }
    Ok(weights
        .iter()
        .all(|v| *v >= -128.0 * f64::EPSILON && *v <= 1.0 + 128.0 * f64::EPSILON))
}
impl FilmFreeSurface {
    /// Cell containing the point in the reconstructed substrate-to-surface prism.
    /// Each prism is split into three tetrahedra. Dry zero-volume prisms are skipped.
    /// This is center containment, not resolved sphere-volume intersection.
    pub fn immersed_cell(
        &self,
        point: [f64; 3],
        max_checks: usize,
    ) -> Result<Option<usize>, &'static str> {
        if point.iter().any(|v| !v.is_finite()) || max_checks == 0 {
            return Err("invalid film immersion query");
        }
        let mut checks = 0usize;
        for (cell, (base, top)) in self
            .substrate
            .iter()
            .zip(&self.geometry.geometry)
            .enumerate()
        {
            checks += 1;
            if checks > max_checks {
                return Err("film immersion search budget");
            }
            if (0..3).any(|k| {
                base.iter().chain(top).all(|p| p[k] < point[k])
                    || base.iter().chain(top).all(|p| p[k] > point[k])
            }) {
                continue;
            }
            for tet in [
                [base[0], base[1], base[2], top[0]],
                [base[1], base[2], top[0], top[1]],
                [base[2], top[0], top[1], top[2]],
            ] {
                checks += 1;
                if checks > max_checks {
                    return Err("film immersion search budget");
                }
                if inside_tetrahedron(point, tet)? {
                    return Ok(Some(cell));
                }
            }
        }
        Ok(None)
    }
    /// Partial overlap of the reconstructed wet top, with the center on the
    /// declared fluid side of the substrate. A dry coincident top is excluded.
    pub fn overlapping_wet_cell(
        &self,
        center: [f64; 3],
        radius: f64,
    ) -> Result<Option<usize>, &'static str> {
        let Some(hit) = self.geometry.first_sphere_hit(center, center, radius)? else {
            return Ok(None);
        };
        if hit.penetration <= 64.0 * f64::EPSILON * radius {
            return Ok(None);
        }
        let base = self.substrate[hit.cell];
        let top = self.geometry.geometry[hit.cell];
        let a = super::sub(base[1], base[0]);
        let b = super::sub(base[2], base[0]);
        let normal = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        if super::dot(super::sub(center, base[0]), normal) * self.side < 0.0 || base == top {
            return Ok(None);
        }
        Ok(Some(hit.cell))
    }
    pub(crate) fn collision_geometry(&self) -> &SurfaceFilm {
        &self.geometry
    }
    pub fn points(&self) -> &[[f64; 3]] {
        &self.points
    }
    pub fn triangles(&self) -> &[[usize; 3]] {
        &self.geometry.triangles
    }
    pub fn first_sphere_hit(
        &self,
        start: [f64; 3],
        end: [f64; 3],
        radius: f64,
    ) -> Result<Option<SphereFilmHit>, &'static str> {
        self.geometry.first_sphere_hit(start, end, radius)
    }
    pub fn first_closing_sphere_hit(
        &self,
        start: [f64; 3],
        end: [f64; 3],
        radius: f64,
    ) -> Result<Option<SphereFilmHit>, &'static str> {
        self.geometry.first_closing_sphere_hit(start, end, radius)
    }
    pub fn first_accelerated_sphere_hit(
        &self,
        start: [f64; 3],
        velocity: [f64; 3],
        acceleration: [f64; 3],
        dt: f64,
        radius: f64,
        max_feature_checks: usize,
    ) -> Result<Option<SphereFilmHit>, &'static str> {
        self.geometry.first_accelerated_sphere_hit(
            start,
            velocity,
            acceleration,
            dt,
            radius,
            max_feature_checks,
        )
    }
    pub fn first_closing_accelerated_sphere_hit(
        &self,
        start: [f64; 3],
        velocity: [f64; 3],
        acceleration: [f64; 3],
        dt: f64,
        radius: f64,
        max_feature_checks: usize,
    ) -> Result<Option<SphereFilmHit>, &'static str> {
        self.geometry.first_closing_accelerated_sphere_hit(
            start,
            velocity,
            acceleration,
            dt,
            radius,
            max_feature_checks,
        )
    }
}
impl SurfaceFilm {
    /// Builds a piecewise-linear free surface using area-weighted vertex thickness
    /// and vertex normals. `side` must be +1 or -1 relative to triangle winding.
    /// The FV cell volumes remain authoritative and unchanged. This smooth
    /// reconstruction is not an exact extrusion of discontinuous cell heights.
    /// Faces, finite edges and vertices are queried; vertical boundary skirts and
    /// trapped-volume contact are not represented. Geometry is frozen at construction.
    pub fn free_surface(&self, side: f64) -> Result<FilmFreeSurface, &'static str> {
        if side != 1.0 && side != -1.0 {
            return Err("invalid film free-surface side");
        }
        let mut state = self.state();
        let heights = self.vertex_thickness(state.points.len())?;
        let mut normals = vec![[0.0; 3]; state.points.len()];
        for (i, tri) in self.triangles.iter().enumerate() {
            for &v in tri {
                for k in 0..3 {
                    normals[v][k] += self.normals[i][k] * self.area[i];
                }
            }
        }
        for (i, p) in state.points.iter_mut().enumerate() {
            if heights[i] == 0.0 {
                continue;
            }
            let length = norm(normals[i]);
            if !length.is_finite() || length <= 0.0 || !heights[i].is_finite() {
                return Err("undefined film free-surface vertex normal");
            }
            for k in 0..3 {
                p[k] += side * heights[i] * (normals[i][k] / length);
            }
            if p.iter().any(|v| !v.is_finite()) {
                return Err("film free-surface coordinate overflow");
            }
        }
        let geometry = SurfaceFilm::new(&state.points, state.triangles, self.material)?;
        Ok(FilmFreeSurface {
            geometry,
            points: state.points,
            substrate: self.geometry.clone(),
            side,
        })
    }
}
