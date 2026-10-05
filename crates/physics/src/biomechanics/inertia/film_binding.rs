//! Reference mesh compatibility and stable boundary-cell attachment.
use super::super::{TetraMesh, columns, det, sub};
use super::InertialBody;
use crate::surface_film::{Material, SurfaceFilm, ThermalFilmMixture};
#[derive(Clone, Debug)]
pub struct SolidFilmBinding {
    rest: Vec<[f64; 3]>,
    cells: Vec<[usize; 4]>,
    faces: Vec<[usize; 3]>,
    owners: Vec<usize>,
    internal_faces: Vec<([usize; 3], [usize; 2])>,
}
impl SolidFilmBinding {
    /// Binds the complete audited tetrahedral boundary, without nearest-cell
    /// inference. Compatibility is defined by reference coordinates and cell
    /// order; copies of the same reference mesh remain compatible.
    pub fn new(body: &InertialBody) -> Result<Self, &'static str> {
        let rest = body.body.rest.clone();
        let cells: Vec<_> = body.body.elements.iter().map(|e| e.nodes).collect();
        let mut normalized = cells.clone();
        for cell in &mut normalized {
            let [a, b, c, d] = cell.map(|i| rest[i]);
            if det(columns(sub(b, a), sub(c, a), sub(d, a))) < 0. {
                cell.swap(1, 2);
            }
        }
        let faces = body.body.surface();
        TetraMesh {
            points: rest.clone(),
            cells: normalized,
            boundary: faces.clone(),
        }
        .validate_topology()?;
        if faces.is_empty() {
            return Err("empty solid film boundary");
        }
        let mut face_owners = std::collections::BTreeMap::<[usize; 3], Vec<usize>>::new();
        for (cell, &[a, b, c, d]) in cells.iter().enumerate() {
            for mut key in [[a, b, c], [a, b, d], [a, c, d], [b, c, d]] {
                key.sort_unstable();
                face_owners.entry(key).or_default().push(cell);
            }
        }
        let owners = faces
            .iter()
            .map(|face| {
                let mut key = *face;
                key.sort_unstable();
                let values = face_owners.get(&key).ok_or("missing boundary cell")?;
                if values.len() != 1 {
                    return Err("ambiguous boundary cell");
                }
                Ok(values[0])
            })
            .collect::<Result<Vec<_>, _>>()?;
        let internal_faces = face_owners
            .into_iter()
            .filter_map(|(face, owners)| {
                (owners.len() == 2).then(|| (face, [owners[0], owners[1]]))
            })
            .collect();
        Ok(Self {
            internal_faces,
            rest,
            cells,
            faces,
            owners,
        })
    }
    fn compatible(&self, body: &InertialBody) -> Result<(), &'static str> {
        if self.rest != body.body.rest
            || self.cells.len() != body.body.elements.len()
            || self
                .cells
                .iter()
                .zip(&body.body.elements)
                .any(|(cell, e)| *cell != e.nodes)
        {
            return Err("incompatible solid film reference mesh");
        }
        Ok(())
    }
    pub fn new_film(
        &self,
        body: &InertialBody,
        material: Material,
    ) -> Result<SurfaceFilm, &'static str> {
        self.compatible(body)?;
        SurfaceFilm::new(body.body.positions(), self.faces.clone(), material)
    }
    /// Derives W/K conductances from current physical triangle area and supplied
    /// areal contact resistance (m² K/W). Rejects stale/wrong film geometry.
    pub fn heat_contacts(
        &self,
        body: &InertialBody,
        film: &SurfaceFilm,
        resistance: &[f64],
    ) -> Result<Vec<(usize, usize, f64)>, &'static str> {
        self.compatible(body)?;
        if film.triangles() != self.faces || resistance.len() != self.faces.len() {
            return Err("incompatible bound film topology");
        }
        self.faces
            .iter()
            .enumerate()
            .map(|(cell, face)| {
                if film.cell_points_m(cell)? != face.map(|i| body.body.positions()[i]) {
                    return Err("stale bound film geometry");
                }
                if !resistance[cell].is_finite() || resistance[cell] <= 0. {
                    return Err("invalid film contact resistance");
                }
                let g = film.cell_area_m2(cell)? / resistance[cell];
                if !g.is_finite() || g <= 0. {
                    return Err("unrepresentable film contact conductance");
                }
                Ok((self.owners[cell], cell, g))
            })
            .collect()
    }
    /// Cell-centred two-point conduction on the current tetrahedral geometry.
    /// Conductivity is isotropic W/(m K). This is a monotone network model;
    /// non-orthogonal meshes require a separate consistency correction to obtain
    /// a convergent continuum diffusion discretization.
    pub fn internal_heat_contacts(
        &self,
        body: &InertialBody,
        conductivity: &[f64],
    ) -> Result<Vec<(usize, usize, f64)>, &'static str> {
        self.compatible(body)?;
        if conductivity.len() != self.cells.len()
            || conductivity.iter().any(|k| !k.is_finite() || *k <= 0.)
        {
            return Err("invalid solid conductivity");
        }
        let points = body.body.positions();
        let mut links = Vec::with_capacity(self.internal_faces.len());
        for &(face, owners) in &self.internal_faces {
            let [a, b, c] = face.map(|i| points[i]);
            let u = sub(b, a);
            let v = sub(c, a);
            let normal = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let length = normal[0].hypot(normal[1]).hypot(normal[2]);
            if !length.is_finite() || length <= 0. {
                return Err("invalid solid thermal face");
            }
            let signed_distance = |cell: usize| {
                let mut center = [0.; 3];
                for node in self.cells[cell] {
                    for axis in 0..3 {
                        center[axis] += (points[node][axis] - a[axis]) * 0.25;
                    }
                }
                center
                    .iter()
                    .zip(normal)
                    .map(|(v, n)| v * n / length)
                    .sum::<f64>()
            };
            let i = owners[0];
            let j = owners[1];
            let di = signed_distance(i);
            let dj = signed_distance(j);
            if !di.is_finite()
                || !dj.is_finite()
                || di == 0.
                || dj == 0.
                || di.signum() == dj.signum()
            {
                return Err("invalid solid thermal cell geometry");
            }
            let resistance = di.abs() / conductivity[i] + dj.abs() / conductivity[j];
            let g = (length * 0.5) / resistance;
            if !g.is_finite() || g <= 0. {
                return Err("unrepresentable solid conductance");
            }
            links.push((i, j, g));
        }
        Ok(links)
    }
    /// Closed thermal exchange at matching solid/film geometry. Geometry refresh
    /// is explicit so its separate mechanical work is not silently hidden here.
    pub fn exchange_heat(
        &self,
        body: &mut InertialBody,
        film: &mut ThermalFilmMixture,
        resistance: &[f64],
        dt: f64,
        tolerance: f64,
    ) -> Result<f64, &'static str> {
        let links = self.heat_contacts(body, film.mixture().film(), resistance)?;
        body.exchange_maxwell_film_heat(film, &links, dt, tolerance)
    }
}
