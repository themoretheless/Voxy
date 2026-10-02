//! Parameterized adult anatomical fold specimens, not fitted anatomical meshes.
use super::{Point, Skin, SkinMaterial};
/// A mechanically continuous fold and explicit support/loading identities.
#[derive(Clone, Debug)]
pub struct TissueFold {
    pub skin: Skin,
    pub root_nodes: Vec<usize>,
    /// Distal fold crest, not a prescribed kinematic trajectory.
    pub crest_nodes: Vec<usize>,
}
#[derive(Clone, Copy, Debug)]
pub struct PrepuceGeometry {
    pub inner_radius_m: f64,
    pub outer_radius_m: f64,
    pub length_m: f64,
    pub sectors: usize,
    /// Rows per inner sheet, rounded distal fold and outer sheet.
    pub rows_per_section: usize,
}
impl PrepuceGeometry {
    /// Connected inner/outer sheets joined by a rounded distal fold; two basal
    /// rings fixed. Z is the longitudinal axis; material follows the circumference.
    /// # Errors
    /// Invalid geometry/resolution, excessively thick shell, material/mesh error.
    pub fn build(self, material: SkinMaterial) -> Result<TissueFold, &'static str> {
        let h = (self.outer_radius_m - self.inner_radius_m) / 2.;
        if [self.inner_radius_m, self.outer_radius_m, self.length_m]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.)
            || h <= 0.
            || self.length_m <= h
            || !(8..=128).contains(&self.sectors)
            || !(2..=32).contains(&self.rows_per_section)
            || material.layers.iter().map(|l| l.thickness).sum::<f64>() >= 2. * h
        {
            return Err("invalid prepuce fold geometry");
        }
        let n = self.rows_per_section;
        let rows = 3 * n;
        let index = |row: usize, a: usize| row * self.sectors + a % self.sectors;
        let mut points = Vec::new();
        for row in 0..=rows {
            let (r, z) = if row <= n {
                (
                    self.inner_radius_m,
                    (self.length_m - h) * row as f64 / n as f64,
                )
            } else if row <= 2 * n {
                let phi = std::f64::consts::PI * (row - n) as f64 / n as f64;
                (
                    (self.inner_radius_m + self.outer_radius_m) / 2. - h * phi.cos(),
                    self.length_m - h + h * phi.sin(),
                )
            } else {
                (
                    self.outer_radius_m,
                    (self.length_m - h) * (rows - row) as f64 / n as f64,
                )
            };
            for a in 0..self.sectors {
                let angle = std::f64::consts::TAU * a as f64 / self.sectors as f64;
                points.push([r * angle.cos(), r * angle.sin(), z]);
            }
        }
        let mut faces = Vec::new();
        let mut directions = Vec::new();
        for row in 0..rows {
            for a in 0..self.sectors {
                let [u, v, w, q] = [
                    index(row, a),
                    index(row, a + 1),
                    index(row + 1, a),
                    index(row + 1, a + 1),
                ];
                let direction = std::array::from_fn(|k| points[v][k] - points[u][k]);
                faces.extend([[u, v, q], [u, q, w]]);
                directions.extend([direction; 2]);
            }
        }
        let roots: Vec<_> = (0..self.sectors)
            .flat_map(|a| [index(0, a), index(rows, a)])
            .collect();
        let crest = (0..self.sectors).map(|a| index(n + n / 2, a)).collect();
        Ok(TissueFold {
            skin: Skin::new(points, faces, &roots, material, directions)?,
            root_nodes: roots,
            crest_nodes: crest,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct LabiaMinoraGeometry {
    pub length_m: f64,
    pub fold_width_m: f64,
    pub fold_height_m: f64,
    /// Distance between the two fold center lines.
    pub separation_m: f64,
    pub longitudinal_segments: usize,
    pub transverse_segments: usize,
}
impl LabiaMinoraGeometry {
    /// Two separate mirrored folds, each fixed along its basal perimeter.
    /// Y is longitudinal, X lateral, Z outward. Each retains its own material history.
    /// # Errors
    /// Invalid dimensions/resolution, overlapping rest folds or material/mesh error.
    pub fn build(self, material: SkinMaterial) -> Result<[TissueFold; 2], &'static str> {
        if [
            self.length_m,
            self.fold_width_m,
            self.fold_height_m,
            self.separation_m,
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
            || self.separation_m <= self.fold_width_m
            || !(2..=64).contains(&self.longitudinal_segments)
            || !(2..=32).contains(&self.transverse_segments)
        {
            return Err("invalid labia minora fold geometry");
        }
        let build = |side: f64| {
            let ny = self.longitudinal_segments;
            let nx = self.transverse_segments;
            let index = |y: usize, x: usize| y * (nx + 1) + x;
            let mut points: Vec<Point> = Vec::new();
            let mut roots = Vec::new();
            let mut crest = Vec::new();
            for y in 0..=ny {
                for x in 0..=nx {
                    let u = x as f64 / nx as f64;
                    let v = y as f64 / ny as f64;
                    points.push([
                        side * (self.separation_m / 2. + self.fold_width_m * (u - 0.5)),
                        self.length_m * (v - 0.5),
                        self.fold_height_m
                            * (std::f64::consts::PI * u).sin()
                            * (std::f64::consts::PI * v).sin(),
                    ]);
                    if x == 0 || x == nx || y == 0 || y == ny {
                        roots.push(index(y, x));
                    }
                    if x == nx / 2 && y > 0 && y < ny {
                        crest.push(index(y, x));
                    }
                }
            }
            let mut faces = Vec::new();
            for y in 0..ny {
                for x in 0..nx {
                    let [u, v, w, q] = [
                        index(y, x),
                        index(y, x + 1),
                        index(y + 1, x),
                        index(y + 1, x + 1),
                    ];
                    if side > 0. {
                        faces.extend([[u, v, q], [u, q, w]]);
                    } else {
                        faces.extend([[u, q, v], [u, w, q]]);
                    }
                }
            }
            let directions = vec![[0., 1., 0.]; faces.len()];
            Ok(TissueFold {
                skin: Skin::new(points, faces, &roots, material.clone(), directions)?,
                root_nodes: roots,
                crest_nodes: crest,
            })
        };
        Ok([build(-1.)?, build(1.)?])
    }
}
