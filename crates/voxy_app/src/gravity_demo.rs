//! Native showcase using the same finite-body simulation as physics clients.
use physics::gravity::{Body, Gravity};
use physics::gravity_spheres::{Simulation, Sphere};
use voxy_render::{SceneMesh, SceneVertex};

#[derive(Debug, Default)]
struct PlanetProof {
    inverted: bool,
    jumped: bool,
    landed: bool,
}

#[derive(Debug)]
pub(crate) struct GravityDemo {
    bodies: Vec<Sphere>,
    solver: Simulation,
    previous: Vec<[f64; 3]>,
    trails: Vec<std::collections::VecDeque<[f64; 3]>>,
    accumulator: f64,
    pub(crate) contacts: usize,
    pub(crate) steps: usize,
    collision: bool,
    walker: Option<physics::gravity_character::State>,
    jump_pending: bool,
    pub(crate) planet_steps: usize,
    proof: PlanetProof,
}
impl GravityDemo {
    pub(crate) fn new(collision: bool) -> Self {
        let mut bodies = vec![
            Sphere {
                body: Body {
                    mass: 1.0,
                    position: [-1.5, 0.0, 0.0],
                    velocity: [0.0, -(1.0_f64 / 6.0).sqrt(), 0.0],
                },
                radius: 0.3,
                angular_velocity: [0.0; 3],
            },
            Sphere {
                body: Body {
                    mass: 1.0,
                    position: [1.5, 0.0, 0.0],
                    velocity: [0.0, (1.0_f64 / 6.0).sqrt(), 0.0],
                },
                radius: 0.3,
                angular_velocity: [0.0; 3],
            },
        ];
        if collision {
            bodies[0].body.position = [-2.0, 0.0, 0.0];
            bodies[1].body.position = [2.0, 0.0, 0.0];
            bodies[0].body.velocity = [3.0, 0.0, 0.0];
            bodies[1].body.velocity = [-3.0, 0.0, 0.0];
            bodies[0].body.mass = 2.0;
            bodies[0].radius = 0.5;
            bodies[1].radius = 0.5;
        }
        Self {
            previous: bodies.iter().map(|b| b.body.position).collect(),
            trails: bodies
                .iter()
                .map(|b| std::collections::VecDeque::from(vec![b.body.position; 240]))
                .collect(),
            bodies,
            solver: Simulation {
                gravity: Gravity {
                    constant: 1.0,
                    ..Gravity::default()
                },
                restitution: 0.9,
                friction: 0.2,
                ..Simulation::default()
            },
            accumulator: 0.0,
            contacts: 0,
            steps: 0,
            collision,
            walker: None,
            jump_pending: false,
            planet_steps: 0,
            proof: PlanetProof::default(),
        }
    }
    pub(crate) fn reset(&mut self, collision: bool) {
        *self = Self::new(collision);
    }
    pub(crate) fn restart(&mut self) {
        if self.walker.is_some() {
            self.planet();
        } else {
            self.reset(self.collision);
        }
    }
    pub(crate) fn planet(&mut self) {
        use physics::gravity_character::State;
        self.reset(false);
        self.bodies[0].body = Body {
            mass: 50.0,
            position: [0.0; 3],
            velocity: [0.0; 3],
        };
        self.bodies[0].radius = 2.0;
        self.bodies[1].body = Body {
            mass: 1.0,
            position: [0.0, 2.2, 0.0],
            velocity: [0.0; 3],
        };
        self.bodies[1].radius = 0.2;
        for (k, b) in self.bodies.iter().enumerate() {
            self.previous[k] = b.body.position;
            self.trails[k] = std::collections::VecDeque::from(vec![b.body.position; 240]);
        }
        self.walker = Some(State {
            anchor: physics::Origin::default(),
            center: [0.0, 2.2, 0.0],
            radius: 0.2,
            velocity: [0.0; 3],
            up: [0.0, 1.0, 0.0],
            grounded: true,
        });
    }
    pub(crate) fn jump(&mut self) {
        self.jump_pending = true;
    }
    pub(crate) fn planet_verified(&self) -> bool {
        self.proof.inverted && self.proof.jumped && self.proof.landed
    }
    pub(crate) fn advance(&mut self, dt: f64) -> Result<(), String> {
        const STEP: f64 = 1.0 / 240.0;
        self.accumulator += dt.min(0.1);
        while self.accumulator >= STEP {
            for (p, b) in self.previous.iter_mut().zip(&self.bodies) {
                *p = b.body.position;
            }
            if let Some(walker) = &mut self.walker {
                use physics::gravity_character::{
                    self as controller, Config, Input, SphereWorld, SurfaceSphere,
                };
                use physics::gravity_field::{NewtonianField, Source};
                let surfaces = [SurfaceSphere {
                    anchor: physics::Origin::default(),
                    center: [0.0; 3],
                    radius: 2.0,
                }];
                let sources = [Source {
                    anchor: physics::Origin::default(),
                    position: [0.0; 3],
                    mass: 50.0,
                    radius: 2.0,
                }];
                let input = Input {
                    tangent_velocity: Some([walker.up[1] * 1.5, -walker.up[0] * 1.5, 0.0]),
                    jump_pressed: self.jump_pending || self.planet_steps % 1200 == 600,
                };
                self.jump_pending = false;
                let report = controller::step(
                    &SphereWorld {
                        surfaces: &surfaces,
                    },
                    &NewtonianField {
                        gravity: self.solver.gravity,
                        sources: &sources,
                    },
                    walker,
                    input,
                    STEP,
                    Config::default(),
                )
                .map_err(|e| format!("planet character: {e:?}"))?;
                self.contacts += report.contacts.len();
                self.planet_steps += 1;
                self.proof.inverted |= walker.up[1] < -0.9;
                self.proof.jumped |= report.jumped;
                self.proof.landed |= self.proof.jumped && walker.grounded;
                #[allow(clippy::cast_precision_loss)]
                let origin = [
                    walker.anchor.x as f64,
                    walker.anchor.y as f64,
                    walker.anchor.z as f64,
                ];
                self.bodies[1].body.position =
                    std::array::from_fn(|k| origin[k] + walker.center[k]);
                self.bodies[1].body.velocity = walker.velocity;
            } else {
                let report = self
                    .solver
                    .step(&mut self.bodies, STEP)
                    .map_err(|e| format!("gravity: {e:?}"))?;
                self.contacts += report.contacts;
            }
            self.steps += 1;
            for (trail, b) in self.trails.iter_mut().zip(&self.bodies) {
                trail.pop_front();
                trail.push_back(b.body.position);
            }
            self.accumulator -= STEP;
        }
        Ok(())
    }
    pub(crate) fn title(&self) -> String {
        format!(
            "Voxy gravity | {} | contacts {} | 1: orbit  2: collision  3: planet  J: jump  R: reset  Space: pause",
            if self.walker.is_some() {
                "planet walk"
            } else if self.collision {
                "collision"
            } else {
                "binary orbit"
            },
            self.contacts
        )
    }
    pub(crate) fn camera_distance(&self, aspect: f32) -> f32 {
        #[allow(clippy::cast_possible_truncation)]
        let extent = self
            .bodies
            .iter()
            .map(|s| s.body.position[0].abs().max(s.body.position[1].abs()) + s.radius)
            .fold(2.0_f64, f64::max) as f32;
        (extent * 1.3 / (27.5_f32.to_radians().tan() * aspect.min(1.0))).max(9.0)
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub(crate) fn mesh(&self) -> Result<SceneMesh, voxy_render::SceneError> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let colors = [[0.25, 0.65, 1.0, 1.0], [1.0, 0.55, 0.15, 1.0]];
        let alpha = self.accumulator * 240.0;
        // UV spheres, with per-face shading so their volume is visible.
        for (index, s) in self.bodies.iter().enumerate() {
            let position: [f64; 3] = std::array::from_fn(|k| {
                self.previous[index][k] * (1.0 - alpha) + s.body.position[k] * alpha
            });
            let base = vertices.len() as u32;
            for latitude in 0..=12 {
                let phi = f64::from(latitude) * std::f64::consts::PI / 12.0;
                for longitude in 0..=24 {
                    let theta = f64::from(longitude) * std::f64::consts::TAU / 24.0;
                    let normal = [phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin()];
                    let shade = (0.35
                        + 0.65 * (normal[0] * 0.3 + normal[1] * 0.6 + normal[2] * 0.7).max(0.0))
                        as f32;
                    let mut color = colors[index];
                    for c in &mut color[..3] {
                        *c *= shade;
                    }
                    vertices.push(SceneVertex {
                        position: std::array::from_fn(|k| {
                            (position[k] + s.radius * normal[k]) as f32
                        }),
                        uv: [0.0; 2],
                        color,
                    });
                }
            }
            for latitude in 0..12 {
                for longitude in 0..24 {
                    let a = base + latitude * 25 + longitude;
                    let b = a + 25;
                    indices.extend([a, a + 1, b, b, a + 1, b + 1]);
                }
            }
            for j in 0..239 {
                let a = self.trails[index][j];
                let b = self.trails[index][j + 1];
                let delta = [b[0] - a[0], b[1] - a[1]];
                let length = delta[0].hypot(delta[1]).max(1e-12);
                let width = 0.012;
                let offset = [-delta[1] / length * width, delta[0] / length * width];
                let base = vertices.len() as u32;
                let mut color = colors[index];
                for c in &mut color[..3] {
                    *c *= 0.25 + 0.65 * j as f32 / 239.0;
                }
                for p in [
                    [a[0] + offset[0], a[1] + offset[1], -0.04],
                    [a[0] - offset[0], a[1] - offset[1], -0.04],
                    [b[0] - offset[0], b[1] - offset[1], -0.04],
                    [b[0] + offset[0], b[1] + offset[1], -0.04],
                ] {
                    vertices.push(SceneVertex {
                        position: p.map(|v| v as f32),
                        uv: [0.0; 2],
                        color,
                    });
                }
                indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
        SceneMesh::new(vertices, indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planet_showcase_walks_inverted_and_jumps() {
        let mut demo = GravityDemo::new(false);
        demo.planet();
        let initial = demo.mesh().unwrap();
        let mut inverted = false;
        let mut airborne = false;
        let mut landed = false;
        for _ in 0..200 {
            demo.advance(0.1).unwrap();
            let walker = demo.walker.unwrap();
            inverted |= walker.up[1] < -0.9;
            airborne |= !walker.grounded;
            landed |= airborne && walker.grounded;
            let mesh = demo.mesh().unwrap();
            assert_eq!(mesh.vertices().len(), initial.vertices().len());
            assert_eq!(mesh.indices().len(), initial.indices().len());
        }
        assert!(inverted && airborne && landed);
    }
    #[test]
    fn showcase_runs_both_modes_with_stable_mesh_capacity() {
        for collision in [false, true] {
            let mut demo = GravityDemo::new(collision);
            let initial = demo.mesh().unwrap();
            for _ in 0..100 {
                demo.advance(0.1).unwrap();
                let mesh = demo.mesh().unwrap();
                assert_eq!(mesh.vertices().len(), initial.vertices().len());
                assert_eq!(mesh.indices().len(), initial.indices().len());
            }
            if collision {
                assert!(demo.contacts > 0);
            } else {
                assert_eq!(demo.contacts, 0);
            }
            assert!(demo.steps >= 2399);
        }
    }
}
