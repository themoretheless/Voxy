//! CPU strand showcase, rendered through the existing scene pipeline.
use physics::strand::{SphereCollider, Strand, StrandConfig};
use voxy_render::{SceneMesh, SceneVertex};
#[derive(Debug)]
pub(crate) struct StrandsDemo {
    strands: Vec<Strand>,
    roots: Vec<[f64; 3]>,
    time: f64,
    accumulator: f64,
}
impl StrandsDemo {
    pub(crate) fn new() -> Self {
        let mut strands = Vec::new();
        let mut roots = Vec::new();
        for i in 0..80 {
            let root = [
                -2.5 + f64::from(i % 20) * 0.13,
                -1.0,
                f64::from(i / 20) * 0.12,
            ];
            let height = 0.65 + f64::from(i % 7) * 0.045;
            let points = (0..7)
                .map(|j| [root[0], root[1] + height * f64::from(j) / 6.0, root[2]])
                .collect();
            strands.push(
                Strand::new(
                    points,
                    StrandConfig {
                        stiffness: 100.0,
                        ..Default::default()
                    },
                )
                .expect("valid grass"),
            );
            roots.push(root);
        }
        for i in 0..28 {
            let x = 0.65 + f64::from(i) * 0.035;
            let root = [x, 1.2, 0.0];
            let points = (0..12)
                .map(|j| [x + f64::from(j) * 0.035, 1.2 - f64::from(j) * 0.13, 0.0])
                .collect();
            strands.push(
                Strand::new(
                    points,
                    StrandConfig {
                        stiffness: 2.0,
                        radius: 0.015,
                        ..Default::default()
                    },
                )
                .expect("valid hair"),
            );
            roots.push(root);
        }
        Self {
            strands,
            roots,
            time: 0.0,
            accumulator: 0.0,
        }
    }
    fn spheres(&self) -> [SphereCollider; 2] {
        [
            SphereCollider {
                center: [-1.3 + 0.9 * (self.time * 0.9).sin(), -0.55, 0.18],
                radius: 0.3,
            },
            SphereCollider {
                center: [1.1 + 0.2 * (self.time * 1.4).sin(), 0.88, 0.0],
                radius: 0.3,
            },
        ]
    }
    pub(crate) fn advance(&mut self, dt: f64) -> Result<(), &'static str> {
        self.accumulator += dt.min(0.1);
        while self.accumulator >= 1.0 / 240.0 {
            self.time += 1.0 / 240.0;
            let colliders = self.spheres();
            for (i, s) in self.strands.iter_mut().enumerate() {
                let mut root = self.roots[i];
                if i >= 80 {
                    root[0] += 0.2 * (self.time * 1.4).sin();
                }
                let wind = 4.0 * (self.time * 1.8).sin() + 2.0 * (self.time * 3.7).sin();
                s.step(
                    1.0 / 240.0,
                    root,
                    [wind, -9.81, 0.6 * (self.time * 2.0).sin()],
                    &colliders,
                )?;
            }
            self.accumulator -= 1.0 / 240.0;
        }
        Ok(())
    }
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn mesh(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut quad = |points: [[f64; 3]; 4], color: [f32; 4]| {
            let base = vertices.len() as u32;
            for p in points {
                vertices.push(SceneVertex {
                    position: p.map(|v| v as f32),
                    uv: [0.0; 2],
                    color,
                });
            }
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        };
        quad(
            [
                [-2.8, -1.02, 0.5],
                [0.15, -1.02, 0.5],
                [0.15, -1.15, 0.5],
                [-2.8, -1.15, 0.5],
            ],
            [0.22, 0.15, 0.08, 1.0],
        );
        for (i, s) in self.strands.iter().enumerate() {
            let color = if i < 80 {
                [0.15, 0.75, 0.25, 1.0]
            } else {
                [0.8, 0.38, 0.09, 1.0]
            };
            for pair in s.positions().windows(2) {
                let a = pair[0];
                let b = pair[1];
                let width = if i < 80 { 0.012 } else { 0.008 };
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let len = dx.hypot(dy).max(1e-9);
                let x = -dy / len * width;
                let y = dx / len * width;
                quad(
                    [
                        [a[0] + x, a[1] + y, a[2]],
                        [a[0] - x, a[1] - y, a[2]],
                        [b[0] - x, b[1] - y, b[2]],
                        [b[0] + x, b[1] + y, b[2]],
                    ],
                    color,
                );
            }
        }
        // Filled discs show the sphere silhouette in the frontal demonstration.
        for c in self.spheres() {
            for j in 0..32 {
                let a = f64::from(j) * std::f64::consts::TAU / 32.0;
                let b = f64::from(j + 1) * std::f64::consts::TAU / 32.0;
                let p = c.center;
                quad(
                    [
                        p,
                        [
                            p[0] + c.radius * a.cos(),
                            p[1] + c.radius * a.sin(),
                            p[2] + 0.02,
                        ],
                        [
                            p[0] + c.radius * b.cos(),
                            p[1] + c.radius * b.sin(),
                            p[2] + 0.02,
                        ],
                        p,
                    ],
                    [0.35, 0.5, 0.8, 1.0],
                );
            }
        }
        SceneMesh::new(vertices, indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animated_mesh_keeps_its_capacity_over_ten_seconds() {
        let mut demo = StrandsDemo::new();
        let initial = demo.mesh().unwrap();
        for _ in 0..100 {
            demo.advance(0.1).unwrap();
            let mesh = demo.mesh().unwrap();
            assert_eq!(mesh.vertices().len(), initial.vertices().len());
            assert_eq!(mesh.indices().len(), initial.indices().len());
        }
        assert!(demo.time > 9.9);
    }
}
