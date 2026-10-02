//! Abstract tissue samples using the shared CPU solver and scene renderer.
use physics::tissue::{Tissue, TissueKind, sample};
use physics::tissue_surface::EmbeddedSurface;
use voxy_render::{SceneMesh, SceneVertex};
#[derive(Debug)]
pub(crate) struct TissueDemo {
    bodies: Vec<Tissue>,
    surfaces: Vec<EmbeddedSurface>,
    body_mode: bool,
    biomechanics: Option<crate::biomechanics_demo::BiomechanicsDemo>,
    time: f64,
    accumulator: f64,
}
impl TissueDemo {
    pub(crate) fn new() -> Self {
        let kinds = [
            TissueKind::Skin,
            TissueKind::Buttock,
            TissueKind::Breast,
            TissueKind::Lip,
            TissueKind::Sphincter,
            TissueKind::Penis,
        ];
        Self {
            bodies: kinds
                .into_iter()
                .enumerate()
                .map(|(i, k)| sample(k, [-2.5 + i as f64, 0.0, 0.0]).expect("valid tissue sample"))
                .collect(),
            surfaces: Vec::new(),
            body_mode: false,
            biomechanics: None,
            time: 0.0,
            accumulator: 0.0,
        }
    }
    pub(crate) fn biomechanics() -> Result<Self, &'static str> {
        let mut demo = Self::new();
        demo.bodies.clear();
        demo.biomechanics = Some(crate::biomechanics_demo::BiomechanicsDemo::new()?);
        Ok(demo)
    }
    pub(crate) fn biomechanics_title(&self) -> Option<String> {
        self.biomechanics.as_ref().map(|b| b.title())
    }
    pub(crate) fn body() -> Self {
        let mut demo = Self::new();
        demo.body_mode = true;
        demo.bodies = [
            TissueKind::Breast,
            TissueKind::Breast,
            TissueKind::Buttock,
            TissueKind::Buttock,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, k)| sample(k, Self::center(i)).unwrap())
        .collect();
        demo.surfaces = demo
            .bodies
            .iter()
            .map(|body| {
                let p = body.positions();
                let mut surface = Vec::new();
                // Subdivide the actual simulation boundary; no axis-extent fitting.
                for a in [1, 2] {
                    for b in [3, 4] {
                        for c in [5, 6] {
                            let point = |u: f64, v: f64| {
                                std::array::from_fn(|k| {
                                    p[a][k] * (1.0 - u - v) + p[b][k] * u + p[c][k] * v
                                })
                            };
                            for i in 0..8 {
                                for j in 0..8 - i {
                                    let u = i as f64 / 8.0;
                                    let v = j as f64 / 8.0;
                                    surface.extend([
                                        point(u, v),
                                        point(u + 0.125, v),
                                        point(u, v + 0.125),
                                    ]);
                                    if i + j < 7 {
                                        surface.extend([
                                            point(u + 0.125, v),
                                            point(u + 0.125, v + 0.125),
                                            point(u, v + 0.125),
                                        ]);
                                    }
                                }
                            }
                        }
                    }
                }
                EmbeddedSurface::bind(p, &body.tetrahedra().collect::<Vec<_>>(), &surface)
                    .expect("boundary lies in tetrahedral mesh")
            })
            .collect();
        demo
    }
    pub(crate) fn is_body(&self) -> bool {
        self.body_mode
    }
    fn center(i: usize) -> [f64; 3] {
        [
            if i % 2 == 0 { -0.23 } else { 0.23 },
            if i < 2 { 0.55 } else { -0.25 },
            if i < 2 { 0.25 } else { -0.23 },
        ]
    }
    fn lift(t: f64) -> f64 {
        // Walk, jump, then rest; twelve-second repeat.
        let t = t % 12.0;
        if t < 4.0 {
            0.045 * (t * std::f64::consts::TAU * 1.8).sin().powi(2)
        } else if t < 8.0 {
            let phase = (t - 4.0) % 2.0;
            if phase < 1.0 {
                0.48 * (std::f64::consts::PI * phase).sin().powi(2)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
    pub(crate) fn advance(&mut self, dt: f64) -> Result<(), &'static str> {
        if let Some(b) = &mut self.biomechanics {
            return b.advance(dt);
        }
        self.accumulator += dt.min(0.1);
        while self.accumulator >= 1.0 / 240.0 {
            self.time += 1.0 / 240.0;
            if self.body_mode {
                let lift = Self::lift(self.time);
                for (i, b) in self.bodies.iter_mut().enumerate() {
                    let c = Self::center(i);
                    b.move_pin(
                        3,
                        [c[0], c[1] + lift + if i < 2 { 0.36 } else { 0.3 }, c[2]],
                    )?;
                    b.move_pin(
                        6,
                        [c[0], c[1] + lift, c[2] - if i < 2 { 0.3 } else { 0.28 }],
                    )?;
                    b.step(1.0 / 240.0, [0.0, -9.81, 0.0], &[], 24)?;
                }
                self.accumulator -= 1.0 / 240.0;
                continue;
            }
            for (i, b) in self.bodies.iter_mut().enumerate() {
                b.set_activation(0.5 + 0.5 * (self.time * 2.0).sin())?;
                let x = -2.5 + i as f64;
                if i == 0 {
                    for j in 20..25 {
                        b.move_pin(
                            j,
                            [
                                x + (j % 5) as f64 * 0.15 - 0.3,
                                0.3,
                                0.12 * (self.time * 3.0).sin(),
                            ],
                        )?;
                    }
                } else if i < 4 {
                    let y = match i {
                        1 => 0.3,
                        2 => 0.36,
                        _ => 0.12,
                    };
                    b.move_pin(3, [x + 0.1 * (self.time * 3.0).sin(), y, 0.0])?;
                    b.move_pin(
                        6,
                        [
                            x,
                            0.0,
                            if i == 1 {
                                -0.28
                            } else if i == 2 {
                                -0.3
                            } else {
                                -0.16
                            },
                        ],
                    )?;
                } else if i == 5 {
                    for j in 0..4 {
                        let y = if j % 2 == 0 { -0.09 } else { 0.09 };
                        let z = if j < 2 { -0.09 } else { 0.09 };
                        b.move_pin(j, [x - 0.35, y + 0.06 * (self.time * 2.0).sin(), z])?;
                    }
                }
                b.step(
                    1.0 / 240.0,
                    if i == 4 {
                        [0.0; 3]
                    } else {
                        [0.0, -2.0, 1.0 * (self.time * 2.0).sin()]
                    },
                    &[],
                    24,
                )?;
            }
            self.accumulator -= 1.0 / 240.0;
        }
        Ok(())
    }
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn mesh(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        if let Some(b) = &self.biomechanics {
            return b.mesh();
        }
        if self.body_mode {
            return self.body_mesh();
        }
        let colors = [
            [0.95, 0.75, 0.5, 1.0],
            [0.95, 0.45, 0.3, 1.0],
            [0.7, 0.55, 0.95, 1.0],
            [0.95, 0.35, 0.65, 1.0],
            [0.3, 0.85, 0.7, 1.0],
            [0.4, 0.65, 0.95, 1.0],
        ];
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for (i, b) in self.bodies.iter().enumerate() {
            for [a, c] in b.edges() {
                let a = b.positions()[a];
                let c = b.positions()[c];
                let dx = c[0] - a[0];
                let dy = c[1] - a[1];
                let length = dx.hypot(dy).max(1e-9);
                let x = -dy / length * 0.008;
                let y = dx / length * 0.008;
                let base = vertices.len() as u32;
                for p in [
                    [a[0] + x, a[1] + y, a[2]],
                    [a[0] - x, a[1] - y, a[2]],
                    [c[0] - x, c[1] - y, c[2]],
                    [c[0] + x, c[1] + y, c[2]],
                ] {
                    vertices.push(SceneVertex {
                        position: p.map(|v| v as f32),
                        uv: [0.0; 2],
                        color: colors[i],
                    });
                }
                indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
        SceneMesh::new(vertices, indices)
    }
    fn body_mesh(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let lift = Self::lift(self.time);
        // Two full mannequins, front and rear, retain a fixed camera and scale.
        for (view, x) in [(1.0, -1.25), (-1.0, 1.25)] {
            let mut ellipsoid = |center: [f64; 3], r: [f64; 3], color: [f32; 4]| {
                let base = vertices.len() as u32;
                for j in 0..=16 {
                    let theta = j as f64 * std::f64::consts::PI / 16.0;
                    for k in 0..=24 {
                        let phi = k as f64 * std::f64::consts::TAU / 24.0;
                        let p = [
                            center[0] + r[0] * theta.sin() * phi.cos(),
                            center[1] + r[1] * theta.cos(),
                            center[2] + r[2] * theta.sin() * phi.sin(),
                        ];
                        vertices.push(SceneVertex {
                            position: [
                                (x + p[0] * view) as f32,
                                (p[1] + lift) as f32,
                                (p[2] * view) as f32,
                            ],
                            uv: [0.0; 2],
                            color,
                        });
                    }
                }
                for j in 0..16 {
                    for k in 0..24 {
                        let a = base + j * 25 + k;
                        indices.extend([a, a + 1, a + 25, a + 1, a + 26, a + 25]);
                    }
                }
            };
            let suit = [0.22, 0.48, 0.67, 1.0];
            ellipsoid([0.0, 0.38, 0.0], [0.39, 0.61, 0.22], suit);
            ellipsoid([0.0, -0.24, 0.0], [0.41, 0.32, 0.25], suit);
            ellipsoid([0.0, 1.24, 0.0], [0.23, 0.28, 0.23], [0.7, 0.73, 0.77, 1.0]);
            for side in [-1.0, 1.0] {
                ellipsoid([side * 0.2, -0.91, 0.0], [0.14, 0.57, 0.15], suit);
                ellipsoid([side * 0.55, 0.35, 0.0], [0.12, 0.54, 0.12], suit);
                ellipsoid([side * 0.2, -1.47, 0.09], [0.15, 0.1, 0.23], suit);
            }
            for (i, (body, surface)) in self.bodies.iter().zip(&self.surfaces).enumerate() {
                let points = surface
                    .deform(body.positions())
                    .expect("validated tissue state");
                let color = if i < 2 {
                    [0.32, 0.67, 0.8, 1.0]
                } else {
                    [0.28, 0.58, 0.73, 1.0]
                };
                for p in points {
                    indices.push(vertices.len() as u32);
                    vertices.push(SceneVertex {
                        position: [(x + p[0] * view) as f32, p[1] as f32, (p[2] * view) as f32],
                        uv: [0.0; 2],
                        color,
                    });
                }
            }
        }
        SceneMesh::new(vertices, indices)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn body_walk_jump_and_settle() {
        let mut d = TissueDemo::body();
        let n = d.mesh().unwrap().vertices().len();
        let mut low = [1e9_f64; 4];
        let mut high = [-1e9_f64; 4];
        for _ in 0..1600 {
            d.advance(1.0 / 240.0).unwrap();
            for (i, body) in d.bodies.iter().enumerate() {
                let p = body.positions();
                let relative = p[0][1] - p[3][1];
                low[i] = low[i].min(relative);
                high[i] = high[i].max(relative);
            }
        }
        for i in 0..4 {
            assert!(
                high[i] - low[i] > 0.005,
                "no secondary motion for region {i}"
            );
        }
        for _ in 0..1000 {
            d.advance(1.0 / 240.0).unwrap();
        }
        let previous: Vec<_> = d.bodies.iter().map(|b| b.positions()[0]).collect();
        d.advance(0.1).unwrap();
        for (body, p) in d.bodies.iter().zip(previous) {
            assert!(
                (body.positions()[0][1] - p[1]).abs() < 0.01,
                "motion did not settle"
            );
        }
        assert_eq!(d.mesh().unwrap().vertices().len(), n);
    }
    #[test]
    fn demo_runs_ten_seconds() {
        let mut d = TissueDemo::new();
        let n = d.mesh().unwrap().vertices().len();
        for _ in 0..100 {
            d.advance(0.1).unwrap();
        }
        assert_eq!(d.mesh().unwrap().vertices().len(), n);
    }
}
