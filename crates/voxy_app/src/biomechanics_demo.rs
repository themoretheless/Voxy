//! Load-controlled quasistatic FEM specimens; solver residual is visible.
use physics::biomechanics::{Body, Equilibrium, penile_chambers, sphincter_layers};
use voxy_render::{SceneMesh, SceneVertex};
#[derive(Debug)]
pub(crate) struct BiomechanicsDemo {
    bodies: Vec<Body>,
    surfaces: Vec<Vec<[usize; 3]>>,
    reports: Vec<Equilibrium>,
    phase: usize,
    hold: f64,
}
impl BiomechanicsDemo {
    pub(crate) fn new() -> Result<Self, &'static str> {
        let mut bodies: Vec<_> = penile_chambers()?.into_iter().collect();
        bodies.push(sphincter_layers()?);
        let surfaces = bodies.iter().map(Body::surface).collect();
        let reports = bodies
            .iter_mut()
            .map(|b| b.equilibrate(1, 1e-4))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            bodies,
            surfaces,
            reports,
            phase: 0,
            hold: 0.,
        })
    }
    pub(crate) fn advance(&mut self, dt: f64) -> Result<(), &'static str> {
        self.hold += dt.clamp(0., 0.1);
        if self.hold > 2. && self.reports.iter().all(|r| r.converged) {
            self.phase = (self.phase + 1) % 4;
            self.hold = 0.;
        }
        for (i, b) in self.bodies.iter_mut().enumerate() {
            b.set_pressure(
                0,
                if i < 2 && matches!(self.phase, 1 | 2) {
                    8000.
                } else {
                    0.
                },
            )?;

            if i == 2 {
                for index in 0..b.elements().len() {
                    let inner = b.elements()[index].region == 0;
                    let activation = match self.phase {
                        1 if inner => 0.3,
                        2 => {
                            if inner {
                                0.3
                            } else {
                                0.5
                            }
                        }
                        _ => 0.,
                    };
                    b.set_activation(index, activation)?;
                }
            }
            self.reports[i] = b.equilibrate(64, 1e-4)?;
        }
        Ok(())
    }
    pub(crate) fn title(&self) -> String {
        let residual = self.reports.iter().map(|r| r.residual_n).fold(0., f64::max);
        let converged = self.reports.iter().all(|r| r.converged);
        format!(
            "Voxy FEM | {} | p={} kPa | residual={residual:.2e} N {} | parameters unvalidated | Space: pause",
            [
                "unloaded",
                "pressure + IAS",
                "pressure + IAS/EAS",
                "release"
            ][self.phase],
            if matches!(self.phase, 1 | 2) { 8 } else { 0 },
            if converged { "equilibrium" } else { "solving" }
        )
    }
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn mesh(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for (i, b) in self.bodies.iter().enumerate() {
            let map = |p: [f64; 3]| -> [f32; 3] {
                if i < 2 {
                    [
                        (p[2] * 20. - 1.9) as f32,
                        (p[1] * 20. + if i == 0 { 0.28 } else { -0.28 }) as f32,
                        (p[0] * 20.) as f32,
                    ]
                } else {
                    [
                        (p[0] * 35. + 1.05) as f32,
                        (p[1] * 35.) as f32,
                        ((p[2] - 0.0125) * 35.) as f32,
                    ]
                }
            };
            for face in &self.surfaces[i] {
                let points = face.map(|j| map(b.positions()[j]));
                let origin = points[0];
                let edge_u = std::array::from_fn::<_, 3, _>(|k| points[1][k] - origin[k]);
                let edge_v = std::array::from_fn::<_, 3, _>(|k| points[2][k] - origin[k]);
                let normal = [
                    edge_u[1] * edge_v[2] - edge_u[2] * edge_v[1],
                    edge_u[2] * edge_v[0] - edge_u[0] * edge_v[2],
                    edge_u[0] * edge_v[1] - edge_u[1] * edge_v[0],
                ];
                let len = normal.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-9);
                let light = 0.4
                    + 0.6 * ((normal[0] * 0.3 + normal[1] * 0.4 + normal[2] * 0.866) / len).abs();
                let radius = face
                    .iter()
                    .map(|j| b.rest_positions()[*j][0].hypot(b.rest_positions()[*j][1]))
                    .sum::<f64>()
                    / 3.;
                let color = if i < 2 {
                    [0.35, 0.7, 0.95]
                } else if radius < 0.0115 {
                    [0.3, 0.95, 0.65]
                } else {
                    [0.95, 0.65, 0.3]
                };
                let base = vertices.len() as u32;
                for position in points {
                    vertices.push(SceneVertex {
                        position,
                        uv: [0.; 2],
                        color: [color[0] * light, color[1] * light, color[2] * light, 1.],
                    });
                }
                indices.extend([base, base + 1, base + 2]);
            }
        }
        SceneMesh::new(vertices, indices)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_loaded_specimens_without_inversion() {
        let mut demo = BiomechanicsDemo::new().unwrap();
        demo.phase = 2;
        for _ in 0..8 {
            demo.advance(0.1).unwrap();
        }
        assert!(
            demo.reports
                .iter()
                .all(|r| r.min_j > 0.9 && r.residual_n.is_finite())
        );
        assert!(!demo.mesh().unwrap().vertices().is_empty());
        assert!(demo.title().contains("p=8 kPa"));
    }
    #[test]
    fn all_load_stages_reach_equilibrium() {
        let mut demo = BiomechanicsDemo::new().unwrap();
        for phase in 1..=3 {
            demo.phase = phase;
            demo.hold = 0.;
            let mut converged = false;
            for _ in 0..600 {
                demo.advance(0.).unwrap();
                if demo.reports.iter().all(|r| r.converged) {
                    converged = true;
                    break;
                }
            }
            assert!(converged, "phase {phase}: {:?}", demo.reports);
            assert!(demo.reports.iter().all(|r| r.min_j > 0.9));
        }
    }
}
