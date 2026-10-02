//! Idealized volumetric labia majora cores; skin/core coupling is not implied.
use super::{Body, Material};
#[derive(Clone, Debug)]
pub struct LabialPad {
    pub body: Body,
    pub base_nodes: Vec<usize>,
    pub apex_node: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct LabiaMajoraGeometry {
    pub length_m: f64,
    pub width_m: f64,
    pub height_m: f64,
    pub separation_m: f64,
    pub sectors: usize,
    pub latitude_rings: usize,
}
impl LabiaMajoraGeometry {
    /// Two mirrored half-ellipsoid solid pads with fully fixed basal disk.
    /// Y is longitudinal, X lateral, Z outward. Material is explicit and must
    /// represent the selected adipose/core experiment; no fitted defaults.
    /// # Errors
    /// Invalid geometry, overlap, resolution or tetrahedral constitutive failure.
    pub fn build(self, material: Material) -> Result<[LabialPad; 2], &'static str> {
        if [
            self.length_m,
            self.width_m,
            self.height_m,
            self.separation_m,
        ]
        .iter()
        .any(|x| !x.is_finite() || *x <= 0.)
            || self.separation_m <= self.width_m
            || !(8..=128).contains(&self.sectors)
            || !(2..=32).contains(&self.latitude_rings)
        {
            return Err("invalid labia majora pad geometry");
        }
        let build = |side: f64| {
            let center_x = side * self.separation_m / 2.;
            let index = |r: usize, a: usize| r * self.sectors + a % self.sectors;
            let mut points = Vec::new();
            for ring in 0..self.latitude_rings {
                let theta =
                    std::f64::consts::FRAC_PI_2 * (1. - ring as f64 / self.latitude_rings as f64);
                for sector in 0..self.sectors {
                    let phi = std::f64::consts::TAU * sector as f64 / self.sectors as f64;
                    points.push([
                        center_x + side * self.width_m / 2. * theta.sin() * phi.cos(),
                        self.length_m / 2. * theta.sin() * phi.sin(),
                        if ring == 0 {
                            0.
                        } else {
                            self.height_m * theta.cos()
                        },
                    ]);
                }
            }
            let apex = points.len();
            points.push([center_x, 0., self.height_m]);
            let base_center = points.len();
            points.push([center_x, 0., 0.]);
            let seed = points.len();
            points.push([center_x, 0., self.height_m / 3.]);
            let mut faces = Vec::new();
            for ring in 0..self.latitude_rings - 1 {
                for a in 0..self.sectors {
                    let [u, v, w, q] = [
                        index(ring, a),
                        index(ring, a + 1),
                        index(ring + 1, a),
                        index(ring + 1, a + 1),
                    ];
                    faces.extend([[u, v, q], [u, q, w]]);
                }
            }
            for a in 0..self.sectors {
                faces.push([
                    index(self.latitude_rings - 1, a),
                    index(self.latitude_rings - 1, a + 1),
                    apex,
                ]);
                faces.push([index(0, a + 1), index(0, a), base_center]);
            }
            let roots: Vec<_> = (0..self.sectors).chain([base_center]).collect();
            let pins = (0..points.len())
                .map(|i| i < self.sectors || i == base_center)
                .collect();
            let cells = faces
                .into_iter()
                .map(|f| ([seed, f[0], f[1], f[2]], material.clone()))
                .collect();
            Ok(LabialPad {
                body: Body::new(points, pins, cells)?,
                base_nodes: roots,
                apex_node: apex,
            })
        };
        Ok([build(-1.)?, build(1.)?])
    }
}
