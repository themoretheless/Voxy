use super::{BreedAnatomy, Ear, Species};
use glam::Vec3;
#[derive(Default, Debug)]
pub struct AnatomySurface {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    /// Interior pinna vertices, for region-specific skin and grooming.
    pub inner_ear: Vec<bool>,
    /// Continuous transition from the furred rim to exposed pinna skin.
    pub inner_ear_weight: Vec<f32>,
    pub(super) volumes: Vec<Vec<Section>>,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Section {
    pub(super) p: Vec3,
    pub(super) w: f32,
    pub(super) d: f32,
}
fn s(x: f32, y: f32, z: f32, w: f32, d: f32) -> Section {
    Section {
        p: Vec3::new(x, y, z),
        w,
        d,
    }
}
impl AnatomySurface {
    fn loft(&mut self, sections: &[Section]) {
        const N: usize = 16;
        let delta = sections.last().unwrap().p - sections[0].p;
        let longitudinal_axis = if delta.y.abs() > delta.z.abs() { 1 } else { 2 };
        let start = self.positions.len();
        let mut rings = Vec::new();
        for k in 0..sections.len() - 1 {
            for step in 0..6 {
                let t = step as f32 / 6.;
                let a = sections[k.saturating_sub(1)];
                let b = sections[k];
                let c = sections[k + 1];
                let d = sections[(k + 2).min(sections.len() - 1)];
                let spline = |a: Vec3, b: Vec3, c: Vec3, d: Vec3| {
                    (b * 2.
                        + (c - a) * t
                        + (a * 2. - b * 5. + c * 4. - d) * t * t
                        + (-a + b * 3. - c * 3. + d) * t * t * t)
                        * 0.5
                };
                let mut p = spline(a.p, b.p, c.p, d.p);
                // Keep the sweep coordinate monotone even when Catmull tangents
                // overshoot a short terminal neck span. Other coordinates and
                // section radii retain their smooth authored interpolation.
                p[longitudinal_axis] =
                    b.p[longitudinal_axis] + (c.p[longitudinal_axis] - b.p[longitudinal_axis]) * t;
                let r = spline(
                    Vec3::new(a.w, a.d, 0.),
                    Vec3::new(b.w, b.d, 0.),
                    Vec3::new(c.w, c.d, 0.),
                    Vec3::new(d.w, d.d, 0.),
                );
                rings.push(Section {
                    p,
                    w: r.x.max(0.0005),
                    d: r.y.max(0.0005),
                });
            }
        }
        rings.push(*sections.last().unwrap());
        self.volumes.push(rings.clone());
        for (i, r) in rings.iter().enumerate() {
            let tangent = (rings[(i + 1).min(rings.len() - 1)].p - rings[i.saturating_sub(1)].p)
                .normalize_or_zero();
            let right = (Vec3::X - tangent * tangent.x).normalize_or_zero();
            let up = tangent.cross(right).normalize_or_zero();
            for j in 0..N {
                let angle = j as f32 * std::f32::consts::TAU / N as f32;
                self.positions
                    .push((r.p + right * r.w * angle.cos() + up * r.d * angle.sin()).to_array());
            }
        }
        for i in 0..rings.len() - 1 {
            for j in 0..N {
                let a = (start + i * N + j) as u32;
                let b = (start + i * N + (j + 1) % N) as u32;
                let c = a + N as u32;
                let d = b + N as u32;
                self.indices.extend([a, b, c, b, d, c]);
            }
        }
        for (ring, reverse) in [(0, true), (rings.len() - 1, false)] {
            let center = self.positions.len() as u32;
            self.positions.push(rings[ring].p.to_array());
            for j in 0..N {
                let a = (start + ring * N + j) as u32;
                let b = (start + ring * N + (j + 1) % N) as u32;
                self.indices.extend(if reverse {
                    [center, b, a]
                } else {
                    [center, a, b]
                });
            }
        }
    }
    fn finish(&mut self) {
        let mut normals = vec![Vec3::ZERO; self.positions.len()];
        for tri in self.indices.chunks_exact(3) {
            let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
            let n = (Vec3::from(self.positions[b]) - Vec3::from(self.positions[a]))
                .cross(Vec3::from(self.positions[c]) - Vec3::from(self.positions[a]));
            for i in [a, b, c] {
                normals[i] += n;
            }
        }
        self.normals = normals
            .into_iter()
            .map(|n| n.normalize_or_zero().to_array())
            .collect();
    }
}
pub(super) fn build(a: BreedAnatomy) -> Result<AnatomySurface, String> {
    let mut mesh = AnatomySurface::default();
    let h = a.shoulder_height;
    let l = a.trunk_length;
    let w = a.rib_half_width;
    let f = l * 0.36;
    let b = -l * 0.36;
    let d = a.rib_depth;
    mesh.loft(&[
        // Carry the croup behind the hip joint before closing the body. Closing
        // at the joint leaves a notch beneath the tail and an exposed thigh.
        s(0., a.rump_height - d * 0.40, -l * 0.55, w * 0.12, d * 0.14),
        s(0., a.rump_height - d * 0.40, -l * 0.44, w * 0.65, d * 0.34),
        s(0., a.rump_height - d * 0.40, b, w * 0.88, d * 0.40),
        s(
            0.,
            h - d * 0.40,
            -l * 0.08,
            w * 0.78,
            d * if a.breed.species() == Species::Cat {
                0.43
            } else {
                0.34
            },
        ),
        s(0., h - d * 0.49, l * 0.17, w, d * 0.49),
        s(0., h - d * 0.42, f, w * 0.84, d * 0.42),
        s(0., h - d * 0.28, l * 0.40, w * 0.18, d * 0.18),
    ]);
    let origin = a.head_origin();
    let neck_z = origin.z;
    let head_y = origin.y;
    mesh.loft(&[
        s(0., h - d * 0.56, f - l * 0.07, w * 0.48, d * 0.18),
        s(
            0.,
            h - a.neck_length * 0.05,
            f + a.neck_length * 0.12,
            w * 0.57,
            d * 0.27,
        ),
        s(
            0.,
            head_y,
            neck_z,
            a.skull_width * 0.43,
            a.head_length * 0.28,
        ),
    ]);
    let skull = a.head_length - a.muzzle_length;
    let z = neck_z;
    let hw = a.skull_width * 0.5;
    if a.breed == super::Breed::MaineCoon {
        // Distinct low cranial vault, zygomatic cheeks, brow, nasal bridge
        // and deep compact muzzle. A sphere followed by a second sphere
        // produces the rejected toy silhouette even under dense grooming.
        mesh.loft(&[
            s(0., head_y, z - skull * 0.30, hw * 0.45, skull * 0.34),
            s(0., head_y, z, hw * 0.88, skull * 0.46),
            s(
                0.,
                head_y - skull * 0.04,
                z + skull * 0.46,
                hw,
                skull * 0.44,
            ),
            s(
                0.,
                head_y - skull * 0.09,
                z + skull * 0.80,
                hw * 0.82,
                skull * 0.34,
            ),
            s(
                0.,
                head_y - skull * 0.25,
                z + skull * 1.02,
                hw * 0.50,
                skull * 0.22,
            ),
            s(
                0.,
                head_y - skull * 0.27,
                z + a.head_length * 0.82,
                hw * 0.40,
                skull * 0.21,
            ),
            s(
                0.,
                head_y - skull * 0.25,
                z + a.head_length * 0.89,
                hw * 0.34,
                skull * 0.17,
            ),
        ]);
    } else if a.breed == super::Breed::BritishShorthair {
        // Broad cheeks and a short deep muzzle remain distinct from the
        // tapered domestic head and the longer Maine Coon bridge.
        mesh.loft(&[
            s(0., head_y, z - skull * 0.28, hw * 0.45, skull * 0.36),
            s(0., head_y, z, hw * 0.94, skull * 0.55),
            s(
                0.,
                head_y - skull * 0.02,
                z + skull * 0.56,
                hw,
                skull * 0.52,
            ),
            s(
                0.,
                head_y - skull * 0.08,
                z + skull * 0.92,
                hw * 0.84,
                skull * 0.40,
            ),
            s(
                0.,
                head_y - skull * 0.22,
                z + a.head_length * 0.86,
                hw * 0.45,
                skull * 0.25,
            ),
            s(
                0.,
                head_y - skull * 0.25,
                z + a.head_length,
                hw * 0.21,
                skull * 0.13,
            ),
        ]);
    } else if a.breed.species() == Species::Cat {
        // Short feline face: brow/nasal bridge, paired whisker pads and a
        // separate small nasal planum, rather than a canine muzzle cylinder.
        mesh.loft(&[
            s(0.0, head_y, z - skull * 0.28, hw * 0.35, skull * 0.30),
            s(0.0, head_y, z, hw * 0.87, skull * 0.51),
            s(
                0.0,
                head_y - skull * 0.02,
                z + skull * 0.62,
                hw,
                skull * 0.49,
            ),
            s(
                0.0,
                head_y - skull * 0.10,
                z + skull * 0.94,
                hw * 0.72,
                skull * 0.37,
            ),
            s(
                0.0,
                head_y - skull * 0.24,
                z + a.head_length * 0.86,
                hw * 0.38,
                skull * 0.22,
            ),
            s(
                0.0,
                head_y - skull * 0.25,
                z + a.head_length,
                hw * 0.21,
                skull * 0.13,
            ),
        ]);
    } else {
        let broad_muzzle = a.breed == super::Breed::Labrador;
        mesh.loft(&[
            s(0., head_y, z - skull * 0.30, hw * 0.40, skull * 0.35),
            s(
                0.,
                head_y,
                z,
                hw,
                skull * if broad_muzzle { 0.60 } else { 0.45 },
            ),
            s(
                0.,
                head_y - skull * 0.03,
                z + skull * 0.64,
                hw * if broad_muzzle { 0.96 } else { 0.85 },
                skull * if broad_muzzle { 0.51 } else { 0.38 },
            ),
            s(
                0.,
                head_y - skull * 0.20,
                z + skull,
                hw * if broad_muzzle { 0.70 } else { 0.53 },
                skull * 0.29,
            ),
            s(
                0.,
                head_y - skull * 0.23,
                z + a.head_length * 0.94,
                hw * if broad_muzzle { 0.60 } else { 0.39 },
                skull * 0.24,
            ),
            s(
                0.,
                head_y - skull * 0.22,
                z + a.head_length,
                hw * if broad_muzzle { 0.48 } else { 0.30 },
                skull * 0.19,
            ),
        ]);
    }
    if a.breed.species() == Species::Cat {
        // Paired whisker pads beneath the nasal bridge give a feline muzzle
        // its rounded lower outline without lengthening the nose.
        for side in [-1.0, 1.0] {
            mesh.loft(&[
                s(
                    side * hw * 0.22,
                    head_y - skull * 0.27,
                    z + a.head_length * 0.62,
                    hw * 0.16,
                    skull * 0.12,
                ),
                s(
                    side * hw * 0.22,
                    head_y - skull * 0.28,
                    z + a.head_length * 0.73,
                    hw * 0.17,
                    skull * 0.13,
                ),
                s(
                    side * hw * 0.16,
                    head_y - skull * 0.30,
                    z + a.head_length * 0.84,
                    hw * 0.08,
                    skull * 0.07,
                ),
            ]);
        }
    }
    for side in [-1., 1.] {
        let x = side * w * 0.68;
        let paw = a.paw_half_width;
        mesh.loft(&[
            s(x * 0.75, h - d * 0.32, f, paw * 1.05, d * 0.17),
            s(x, h * 0.52, f - h * 0.025, paw * 1.05, paw * 1.25),
            s(x, h * 0.19, f + 0.012, paw * 0.49, paw * 0.67),
            s(x, paw * 1.1, f + 0.015, paw * 0.72, paw * 0.83),
        ]);
        mesh.loft(&[
            s(x * 0.80, a.rump_height - d * 0.32, b, paw * 1.50, d * 0.23),
            s(x, h * 0.57, b + l * 0.075, paw * 1.4, paw * 1.9),
            s(x, h * 0.25, b - l * 0.045, paw * 0.65, paw * 0.82),
            s(x, paw * 1.1, b, paw * 0.65, paw * 0.70),
        ]);
        for foot_z in [f, b] {
            mesh.loft(&[
                s(x, paw * 0.75, foot_z - paw * 0.55, paw * 0.65, paw * 0.58),
                s(x, paw * 0.75, foot_z + paw * 0.25, paw, paw * 0.72),
                s(x, paw * 0.60, foot_z + paw * 0.80, paw * 0.75, paw * 0.48),
            ]);
            for digit in 0..4 {
                let dx = (digit as f32 - 1.5) * paw * 0.42;
                let tip = a.digit_tip(side, foot_z > 0., digit);
                mesh.loft(&[
                    s(
                        x + dx,
                        paw * 0.52,
                        foot_z + paw * 0.45,
                        paw * 0.23,
                        paw * 0.31,
                    ),
                    s(
                        x + dx,
                        paw * 0.51,
                        foot_z + paw * 1.00,
                        paw * 0.22,
                        paw * 0.34,
                    ),
                    s(tip.x, tip.y, tip.z, paw * 0.15, paw * 0.26),
                ]);
            }
        }
        let ex = side
            * hw
            * if matches!(a.ears, Ear::Folded { .. }) {
                1.05
            } else {
                0.78
            };
        match a.ears {
            Ear::Upright { .. } => {}
            Ear::Folded { length: e } => mesh.loft(&[
                s(ex, head_y + skull * 0.25, z, hw * 0.42, e * 0.17),
                s(
                    ex + side * e * 0.24,
                    head_y + skull * 0.08,
                    z + 0.003,
                    hw * 0.48,
                    e * 0.09,
                ),
                s(
                    ex + side * e * 0.16,
                    head_y - e * 0.65,
                    z + 0.025,
                    hw * 0.18,
                    e * 0.045,
                ),
            ]),
        }
    }
    let t = a.tail_length;
    mesh.loft(&[
        s(0., a.rump_height - d * 0.12, -l * 0.43, w * 0.23, w * 0.23),
        s(
            0.,
            a.rump_height - d * 0.30,
            -l * 0.50 - t * 0.25,
            w * 0.18,
            w * 0.18,
        ),
        s(
            t * 0.09,
            a.rump_height
                - d * if a.breed.species() == Species::Cat {
                    0.30
                } else {
                    0.12
                }
                - t * if a.breed.species() == Species::Cat {
                    0.40
                } else {
                    0.15
                },
            -l * 0.5 - t * 0.65,
            w * 0.09,
            w * 0.09,
        ),
        s(
            t * 0.14,
            a.rump_height
                - d * if a.breed.species() == Species::Cat {
                    0.46
                } else {
                    0.12
                }
                - t * if a.breed.species() == Species::Cat {
                    0.45
                } else {
                    0.25
                },
            -l * 0.5 - t * 0.82,
            0.001,
            0.001,
        ),
    ]);
    mesh.finish();
    super::ears::append(&mut mesh, a);
    if mesh.positions.iter().flatten().any(|v| !v.is_finite()) {
        return Err("nonfinite anatomy surface".into());
    }
    Ok(mesh)
}
