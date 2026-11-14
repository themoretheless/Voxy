//! Explicit idealized urogenital specimens; material constants are caller supplied.
use super::{Body, Material, Vec3, elliptical_tube, elliptical_tube_axial, tube};
/// Four radial regions: mucosa, longitudinal muscle, circular muscle, outer tissue.
/// Fiber directions use tube templates: X circumferential, Y radial, Z longitudinal.
#[derive(Clone, Debug)]
pub struct UrogenitalWallGeometry {
    pub radii_m: [f64; 5],
    pub length_m: f64,
    pub sectors: usize,
    pub segments: usize,
}
impl UrogenitalWallGeometry {
    /// Female urethral wall. Outer material may represent the striated sphincter
    /// for a selected segment; this does not model its measured axial distribution.
    /// # Errors
    /// Invalid dimensions, materials or mesh.
    pub fn urethra(&self, materials: [Material; 4]) -> Result<Body, &'static str> {
        tube(
            &self.radii_m,
            self.length_m,
            self.sectors,
            self.segments,
            &materials,
            true,
        )
    }
    /// Vaginal wall with separately activatable circular/longitudinal regions.
    /// This open circular reference tube omits a collapsed slit and rugae.
    /// # Errors
    /// Invalid dimensions, materials or mesh.
    pub fn vagina(&self, materials: [Material; 4]) -> Result<Body, &'static str> {
        tube(
            &self.radii_m,
            self.length_m,
            self.sectors,
            self.segments,
            &materials,
            true,
        )
    }
    /// Oval lumen with four homothetic wall layers, in an unstressed reference state.
    /// Axis scales multiply every radius along X/Y. Does not model closed contact
    /// or rugae; the lumen retains a finite gap and idealized pressure end caps.
    /// # Errors
    /// Invalid scales, geometry or material parameters.
    pub fn oval_wall(
        &self,
        axis_scales: [f64; 2],
        materials: [Material; 4],
    ) -> Result<Body, &'static str> {
        elliptical_tube(
            &self.radii_m,
            self.length_m,
            self.sectors,
            self.segments,
            &materials,
            true,
            axis_scales,
        )
    }
    /// Independently specified materials for every axial segment's four layers.
    /// Region IDs `4 * segment + layer` permit localized muscle drives. Shared
    /// interface nodes keep the solid mesh continuous across material boundaries.
    /// No measured anatomical material distribution is assumed.
    /// # Errors
    /// Profile count mismatch or invalid geometry/material parameters.
    pub fn axial_wall(
        &self,
        axis_scales: [f64; 2],
        profiles: &[[Material; 4]],
    ) -> Result<Body, &'static str> {
        let profiles: Vec<_> = profiles.iter().map(|p| p.to_vec()).collect();
        elliptical_tube_axial(
            &self.radii_m,
            self.length_m,
            self.sectors,
            self.segments,
            &profiles,
            true,
            axis_scales,
        )
    }
    /// Subdivide each physical wall layer without moving its boundaries or
    /// changing material/activation region IDs (four regions per axial segment).
    pub fn axial_wall_refined(
        &self,
        axis_scales: [f64; 2],
        profiles: &[[Material; 4]],
        subdivisions: usize,
    ) -> Result<Body, &'static str> {
        if !(1..=3).contains(&subdivisions) || profiles.len() != self.segments {
            return Err("invalid wall radial refinement");
        }
        let mut radii = Vec::new();
        for layer in 0..4 {
            for k in 0..subdivisions {
                radii.push(
                    self.radii_m[layer]
                        + (self.radii_m[layer + 1] - self.radii_m[layer]) * k as f64
                            / subdivisions as f64,
                );
            }
        }
        radii.push(self.radii_m[4]);
        let refined: Vec<Vec<Material>> = profiles
            .iter()
            .map(|profile| {
                profile
                    .iter()
                    .flat_map(|material| std::iter::repeat_n(material.clone(), subdivisions))
                    .collect()
            })
            .collect();
        let mut body = elliptical_tube_axial(
            &radii,
            self.length_m,
            self.sectors,
            self.segments,
            &refined,
            true,
            axis_scales,
        )?;
        for element in &mut body.elements {
            element.region /= subdivisions;
        }
        Ok(body)
    }
}
#[derive(Clone, Debug)]
pub struct ClitoralComplex {
    /// Each continuous curved body includes a crus and a distal corpus.
    pub corpora_crura: [Body; 2],
    pub glans: Body,
    pub vestibular_bulbs: [Body; 2],
}
#[derive(Clone, Copy, Debug)]
pub struct ClitoralGeometry {
    pub corpus_radius_m: f64,
    pub crus_length_m: f64,
    pub body_length_m: f64,
    pub root_half_separation_m: f64,
    pub body_half_separation_m: f64,
    pub glans_radii_m: Vec3,
    pub bulb_radii_m: Vec3,
    pub bulb_half_separation_m: f64,
    pub sectors: usize,
    pub segments: usize,
}
impl ClitoralGeometry {
    /// Separate mechanical substructures in a common local coordinate system.
    /// Corpus/crus mesh continuity is real; bonding to glans/bulbs is not implied.
    /// # Errors
    /// Invalid dimensions/resolution/material, intersecting paired reference cores
    /// or degenerate cells. No penile material values are silently substituted.
    pub fn build(
        self,
        corpus: Material,
        glans: Material,
        bulb: Material,
    ) -> Result<ClitoralComplex, &'static str> {
        let dimensions = [
            self.corpus_radius_m,
            self.crus_length_m,
            self.body_length_m,
            self.root_half_separation_m,
            self.body_half_separation_m,
            self.bulb_half_separation_m,
        ];
        if dimensions
            .iter()
            .chain(self.glans_radii_m.iter())
            .chain(self.bulb_radii_m.iter())
            .any(|x| !x.is_finite() || *x <= 0.)
            || self.root_half_separation_m <= self.corpus_radius_m
            || self.body_half_separation_m <= self.corpus_radius_m
            || self.bulb_half_separation_m <= self.bulb_radii_m[0]
            || !(8..=64).contains(&self.sectors)
            || !(2..=32).contains(&self.segments)
        {
            return Err("invalid clitoral geometry");
        }
        let build_corpus = |side: f64| {
            let n = 2 * self.segments;
            let index = |row: usize, a: usize| row * (self.sectors + 1) + 1 + a % self.sectors;
            let center = |row: usize| row * (self.sectors + 1);
            let mut points = Vec::new();
            for row in 0..=n {
                let t = (row as f64 / self.segments as f64).min(1.);
                let blend = t * t * (3. - 2. * t);
                let x = side
                    * (self.root_half_separation_m
                        + (self.body_half_separation_m - self.root_half_separation_m) * blend);
                let z = if row <= self.segments {
                    self.crus_length_m * t
                } else {
                    self.crus_length_m
                        + self.body_length_m * (row - self.segments) as f64 / self.segments as f64
                };
                points.push([x, 0., z]);
                for a in 0..self.sectors {
                    let phi = std::f64::consts::TAU * a as f64 / self.sectors as f64;
                    points.push([
                        x + self.corpus_radius_m * phi.cos(),
                        self.corpus_radius_m * phi.sin(),
                        z,
                    ]);
                }
            }
            let mut cells = Vec::new();
            for row in 0..n {
                let dx = points[center(row + 1)][0] - points[center(row)][0];
                let dz = points[center(row + 1)][2] - points[center(row)][2];
                let norm = dx.hypot(dz);
                let (tx, tz) = (dx / norm, dz / norm);
                let mut segment_material = corpus.clone();
                for fiber in &mut segment_material.fibers {
                    let [a, b, c] = fiber.direction;
                    fiber.direction = [a * tz + c * tx, b, -a * tx + c * tz];
                }
                for a in 0..self.sectors {
                    let [u, v, w, p, q, r] = [
                        center(row),
                        index(row, a),
                        index(row, a + 1),
                        center(row + 1),
                        index(row + 1, a),
                        index(row + 1, a + 1),
                    ];
                    // Consistent triangular-prism subdivision; both base circles held at roots.
                    for nodes in [[u, v, w, r], [u, v, q, r], [u, p, q, r]] {
                        cells.push((nodes, segment_material.clone()));
                    }
                }
            }
            let pins = (0..points.len()).map(|i| i < self.sectors + 1).collect();
            Body::new(points, pins, cells)
        };
        let z = self.crus_length_m + self.body_length_m + self.glans_radii_m[2];
        Ok(ClitoralComplex {
            corpora_crura: [build_corpus(-1.)?, build_corpus(1.)?],
            glans: ellipsoid(
                [0., 0., z],
                self.glans_radii_m,
                self.sectors,
                self.segments,
                glans,
            )?,
            vestibular_bulbs: [
                ellipsoid(
                    [-self.bulb_half_separation_m, 0., self.crus_length_m / 2.],
                    self.bulb_radii_m,
                    self.sectors,
                    self.segments,
                    bulb.clone(),
                )?,
                ellipsoid(
                    [self.bulb_half_separation_m, 0., self.crus_length_m / 2.],
                    self.bulb_radii_m,
                    self.sectors,
                    self.segments,
                    bulb,
                )?,
            ],
        })
    }
}
pub(super) fn ellipsoid(
    center: Vec3,
    radii: Vec3,
    sectors: usize,
    rings: usize,
    material: Material,
) -> Result<Body, &'static str> {
    let mut points = vec![[center[0], center[1], center[2] - radii[2]]];
    let index = |row: usize, a: usize| 1 + row * sectors + a % sectors;
    for row in 0..2 * rings - 1 {
        let theta = std::f64::consts::PI * (row + 1) as f64 / (2 * rings) as f64;
        for a in 0..sectors {
            let phi = std::f64::consts::TAU * a as f64 / sectors as f64;
            points.push([
                center[0] + radii[0] * theta.sin() * phi.cos(),
                center[1] + radii[1] * theta.sin() * phi.sin(),
                center[2] - radii[2] * theta.cos(),
            ]);
        }
    }
    let north = points.len();
    points.push([center[0], center[1], center[2] + radii[2]]);
    let seed = points.len();
    points.push(center);
    let mut faces = Vec::new();
    for a in 0..sectors {
        faces.push([0, index(0, a + 1), index(0, a)]);
        faces.push([north, index(2 * rings - 2, a), index(2 * rings - 2, a + 1)]);
    }
    for row in 0..2 * rings - 2 {
        for a in 0..sectors {
            let [u, v, w, q] = [
                index(row, a),
                index(row, a + 1),
                index(row + 1, a),
                index(row + 1, a + 1),
            ];
            faces.extend([[u, v, q], [u, q, w]]);
        }
    }
    let pins = (0..points.len())
        .map(|i| i == 0 || (i >= 1 && i <= sectors))
        .collect();
    Body::new(
        points,
        pins,
        faces
            .into_iter()
            .map(|f| ([seed, f[0], f[1], f[2]], material.clone()))
            .collect(),
    )
}
