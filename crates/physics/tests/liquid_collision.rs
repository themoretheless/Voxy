use physics::liquid::{Config, ContactConfig, Error, Liquid, Material, Particle};
use physics::{AnchoredAabb, CollisionWorld, SweepResult, sweep_box};
// Deliberately no voxel/world dependency: arbitrary non-grid static boxes.
struct Boxes(Vec<([f64; 3], [f64; 3])>);

impl CollisionWorld for Boxes {
    type Obstacle = usize;
    type Error = &'static str;

    fn sweep_aabb(
        &self,
        body: AnchoredAabb,
        displacement: [f64; 3],
        budget: usize,
    ) -> Result<SweepResult<usize>, Self::Error> {
        if self.0.len() > budget {
            return Err("budget");
        }
        #[allow(clippy::cast_precision_loss)]
        let origin = [
            body.anchor.x as f64,
            body.anchor.y as f64,
            body.anchor.z as f64,
        ];
        let mut nearest = SweepResult {
            fraction: 1.0,
            normal: [0; 3],
            obstacle: None,
        };
        for (id, (min, max)) in self.0.iter().enumerate() {
            let min = std::array::from_fn(|axis| min[axis] - origin[axis]);
            let max = std::array::from_fn(|axis| max[axis] - origin[axis]);
            if let Some((fraction, normal)) = sweep_box(body, displacement, min, max)
                && (fraction < nearest.fraction || nearest.obstacle.is_none())
            {
                nearest = SweepResult {
                    fraction,
                    normal,
                    obstacle: Some(id),
                };
            }
        }
        Ok(nearest)
    }
}

fn liquid(velocity: [f64; 3]) -> Liquid {
    Liquid::new(
        vec![Particle {
            position: [-0.5, 0.0, 0.0],
            velocity,
            mass: 1.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            smoothing_radius: 1.0,
            particle_radius: 0.05,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap()
}
#[test]
fn fast_particle_cannot_cross_a_thin_internal_wall() {
    let mut liquid = liquid([100.0, 0.0, 0.0]);
    let wall = Boxes(vec![([0.0, -1.0, -1.0], [0.001, 1.0, 1.0])]);
    liquid
        .step_with_world(0.05, None, &wall, ContactConfig::default())
        .unwrap();
    assert!((liquid.particles()[0].position[0] + 0.05).abs() < 1e-10);
    assert!(liquid.particles()[0].velocity[0].abs() < 1e-12);
}
#[test]
fn bounce_uses_remaining_time_and_friction_changes_only_tangent_speed() {
    let wall = Boxes(vec![([0.0, -1.0, -1.0], [0.001, 1.0, 1.0])]);
    let mut elastic = liquid([100.0, 0.0, 0.0]);
    elastic
        .step_with_world(
            0.01,
            None,
            &wall,
            ContactConfig {
                restitution: 1.0,
                ..ContactConfig::default()
            },
        )
        .unwrap();
    assert!((elastic.particles()[0].position[0] + 0.6).abs() < 1e-10);
    assert!((elastic.particles()[0].velocity[0] + 100.0).abs() < 1e-12);
    let mut friction = liquid([100.0, 10.0, 0.0]);
    friction
        .step_with_world(
            0.01,
            None,
            &wall,
            ContactConfig {
                friction: 0.5,
                ..ContactConfig::default()
            },
        )
        .unwrap();
    assert!((friction.particles()[0].velocity[1] - 5.0).abs() < 1e-12);
}
#[test]
fn overlap_and_candidate_failures_preserve_state() {
    let mut liquid = liquid([1.0, 0.0, 0.0]);
    let before = liquid.clone();
    let overlapping = Boxes(vec![([-0.6, -1.0, -1.0], [-0.4, 1.0, 1.0])]);
    assert_eq!(
        liquid.step_with_world(0.01, None, &overlapping, ContactConfig::default()),
        Err(Error::InitialOverlap)
    );
    assert_eq!(liquid, before);
    let boxes = Boxes(vec![
        ([0.0, -1.0, -1.0], [0.1, 1.0, 1.0]),
        ([1.0, -1.0, -1.0], [1.1, 1.0, 1.0]),
    ]);
    assert_eq!(
        liquid.step_with_world(
            0.01,
            None,
            &boxes,
            ContactConfig {
                max_candidates: 1,
                ..ContactConfig::default()
            }
        ),
        Err(Error::CollisionBackend)
    );
    assert_eq!(liquid, before);
}
#[test]
fn backend_failure_after_successful_substep_is_atomic() {
    struct FailsAfterOne(std::cell::Cell<usize>);
    impl CollisionWorld for FailsAfterOne {
        type Obstacle = ();
        type Error = ();
        fn sweep_aabb(
            &self,
            _: AnchoredAabb,
            _: [f64; 3],
            _: usize,
        ) -> Result<SweepResult<()>, ()> {
            let count = self.0.get();
            self.0.set(count + 1);
            if count > 0 {
                return Err(());
            }
            Ok(SweepResult {
                fraction: 1.0,
                normal: [0; 3],
                obstacle: None,
            })
        }
    }
    let mut liquid = liquid([100.0, 0.0, 0.0]);
    let before = liquid.clone();
    assert_eq!(
        liquid.step_with_world(
            0.01,
            None,
            &FailsAfterOne(std::cell::Cell::new(0)),
            ContactConfig::default()
        ),
        Err(Error::CollisionBackend)
    );
    assert_eq!(liquid, before);
}

#[test]
fn contact_limit_counts_impacts_and_exhaustion_is_atomic() {
    let wall = Boxes(vec![([0.0, -1.0, -1.0], [0.001, 1.0, 1.0])]);
    let mut one = liquid([100.0, 0.0, 0.0]);
    one.step_with_world(
        0.01,
        None,
        &wall,
        ContactConfig {
            max_contacts: 1,
            ..ContactConfig::default()
        },
    )
    .unwrap();
    let walls = Boxes(vec![
        ([-0.6, -1.0, -1.0], [-0.55, 1.0, 1.0]),
        ([-0.45, -1.0, -1.0], [-0.4, 1.0, 1.0]),
    ]);
    let mut trapped = Liquid::new(
        vec![Particle {
            position: [-0.5, 0.0, 0.0],
            velocity: [100.0, 0.0, 0.0],
            mass: 1.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            smoothing_radius: 1.0,
            particle_radius: 0.01,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    let before = trapped.clone();
    assert_eq!(
        trapped.step_with_world(
            0.002,
            None,
            &walls,
            ContactConfig {
                restitution: 1.0,
                max_contacts: 1,
                ..ContactConfig::default()
            }
        ),
        Err(Error::CollisionBudget)
    );
    assert_eq!(trapped, before);
}
#[test]
fn translating_thin_wall_sweeps_and_pushes_a_stationary_particle() {
    let mut liquid = liquid([0.0; 3]);
    let wall = Boxes(vec![([-1.0, -1.0, -1.0], [-0.999, 1.0, 1.0])]);
    liquid
        .step_with_translating_world(0.01, [100.0, 0.0, 0.0], &wall, ContactConfig::default())
        .unwrap();
    assert!((liquid.particles()[0].velocity[0] - 100.0).abs() < 1e-9);
    assert!(liquid.particles()[0].position[0] >= 0.051 - 1e-10);
}
#[test]
fn translating_world_preserves_galilean_motion_without_contacts() {
    let mut liquid = liquid([3.0, 4.0, 0.0]);
    let empty = Boxes(vec![]);
    liquid
        .step_with_translating_world(0.01, [100.0, -20.0, 2.0], &empty, ContactConfig::default())
        .unwrap();
    assert!((liquid.particles()[0].position[0] + 0.47).abs() < 1e-12);
    assert!((liquid.particles()[0].position[1] - 0.04).abs() < 1e-12);
    assert!((liquid.particles()[0].velocity[0] - 3.0).abs() < 1e-12);
}
#[test]
fn translating_world_failure_restores_particles_and_boundary_geometry() {
    let mut liquid = liquid([0.0; 3]);
    liquid
        .configure_boundaries(vec![physics::liquid::BoundarySample {
            position: [-1.0, 0.0, 0.0],
            volume: 0.001,
        }])
        .unwrap();
    let before = liquid.clone();
    let overlapping = Boxes(vec![([-0.6, -1.0, -1.0], [-0.4, 1.0, 1.0])]);
    assert_eq!(
        liquid.step_with_translating_world(
            0.01,
            [100.0, 0.0, 0.0],
            &overlapping,
            ContactConfig::default()
        ),
        Err(Error::InitialOverlap)
    );
    assert_eq!(liquid, before);
}
#[test]
fn translating_sph_boundaries_and_viscous_heat_match_the_comoving_solution() {
    let mut moving = liquid([0.0; 3]);
    moving
        .configure_boundaries(vec![physics::liquid::BoundarySample {
            position: [-0.9, 0.0, 0.0],
            volume: 0.001,
        }])
        .unwrap();
    moving
        .configure_transport(
            vec![physics::liquid::LiquidField {
                temperature: 10.0,
                concentration: 0.0,
            }],
            vec![physics::liquid::TransportMaterial {
                specific_heat: 1.0,
                conductivity: 0.0,
                ..physics::liquid::TransportMaterial::default()
            }],
        )
        .unwrap();
    moving.set_viscous_heating(true).unwrap();
    let mut comoving = moving.clone();
    comoving.apply_impulses(&[[0.0, -2.0, 0.0]]).unwrap();
    let world = Boxes(vec![]);
    comoving
        .step_with_world(0.01, None, &world, ContactConfig::default())
        .unwrap();
    moving
        .step_with_translating_world(0.01, [0.0, 2.0, 0.0], &world, ContactConfig::default())
        .unwrap();
    assert!(
        (moving.particles()[0].position[1] - comoving.particles()[0].position[1] - 0.02).abs()
            < 1e-12
    );
    assert!(
        (moving.particles()[0].velocity[1] - comoving.particles()[0].velocity[1] - 2.0).abs()
            < 1e-12
    );
    assert!(
        (moving.fields().unwrap()[0].temperature - comoving.fields().unwrap()[0].temperature).abs()
            < 1e-12
    );
    let laboratory = moving.boundary_diagnostics().unwrap();
    let relative = comoving.boundary_diagnostics().unwrap();
    assert!(
        (laboratory.viscous_reaction_forces[0][1] - relative.viscous_reaction_forces[0][1]).abs()
            < 1e-12
    );
}
#[test]
fn elastic_moving_wall_work_matches_the_laboratory_kinetic_energy_gain() {
    let mut liquid = liquid([0.0; 3]);
    let wall = Boxes(vec![([-1.0, -1.0, -1.0], [-0.999, 1.0, 1.0])]);
    let report = liquid
        .step_with_translating_world_report(
            0.01,
            [100.0, 0.0, 0.0],
            &wall,
            ContactConfig {
                restitution: 1.0,
                ..ContactConfig::default()
            },
        )
        .unwrap();
    let particle = liquid.particles()[0];
    let kinetic = 0.5
        * particle.mass
        * particle
            .velocity
            .iter()
            .map(|speed| speed * speed)
            .sum::<f64>();
    assert!((report.impulse[0] - 200.0).abs() < 1e-9);
    assert!((report.drive_work - kinetic).abs() < 1e-7);
}
#[test]
fn inelastic_moving_wall_reports_drive_work_including_contact_loss() {
    let mut liquid = liquid([0.0; 3]);
    let wall = Boxes(vec![([-1.0, -1.0, -1.0], [-0.999, 1.0, 1.0])]);
    let report = liquid
        .step_with_translating_world_report(
            0.01,
            [100.0, 0.0, 0.0],
            &wall,
            ContactConfig::default(),
        )
        .unwrap();
    let particle = liquid.particles()[0];
    let kinetic = 0.5 * particle.mass * particle.velocity[0].powi(2);
    assert!((report.drive_work - 10_000.0).abs() < 1e-7);
    assert!((report.drive_work - kinetic - 5000.0).abs() < 1e-7);
}
#[test]
fn moving_world_transfer_excludes_gravity_when_no_geometry_is_hit() {
    let source = liquid([0.0; 3]);
    let mut liquid = Liquid::new(
        source.particles().to_vec(),
        vec![Material::WATER],
        Config {
            smoothing_radius: 1.0,
            particle_radius: 0.05,
            ..Config::default()
        },
    )
    .unwrap();
    let report = liquid
        .step_with_translating_world_report(
            0.01,
            [0.0, 10.0, 0.0],
            &Boxes(vec![]),
            ContactConfig::default(),
        )
        .unwrap();
    assert!(report.impulse.iter().all(|value| value.abs() < 1e-10));
    assert!(report.drive_work.abs() < 1e-9);
}
#[test]
fn overflow_in_moving_world_work_rolls_back_completed_contacts() {
    let mut liquid = liquid([0.0; 3]);
    let before = liquid.clone();
    let wall = Boxes(vec![([-1.0, -1.0, -1.0], [-0.999, 1.0, 1.0])]);
    assert_eq!(
        liquid.step_with_translating_world_report(
            1e-160,
            [1e160, 0.0, 0.0],
            &wall,
            ContactConfig {
                restitution: 1.0,
                ..ContactConfig::default()
            }
        ),
        Err(Error::NumericalFailure)
    );
    assert_eq!(liquid, before);
}

#[test]
fn finite_mass_template_elastic_contact_conserves_momentum_and_energy() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody};
    let wall = Boxes(vec![([0.0, -1.0, -1.0], [0.001, 1.0, 1.0])]);
    let mut fluid = liquid([100.0, 0.0, 0.0]);
    let mut body = TranslatingBody {
        position: [0.0; 3],
        velocity: [0.0; 3],
        mass: 3.0,
    };
    let report = fluid
        .step_with_dynamic_world(
            0.01,
            &mut body,
            &wall,
            DynamicWorldConfig {
                contact: ContactConfig {
                    restitution: 1.0,
                    ..ContactConfig::default()
                },
                ..DynamicWorldConfig::default()
            },
        )
        .unwrap();
    let v = fluid.particles()[0].velocity[0];
    assert!((v + 50.0).abs() < 1e-10);
    assert!((body.velocity[0] - 50.0).abs() < 1e-10);
    assert!((v + 3.0 * body.velocity[0] - 100.0).abs() < 1e-10);
    assert!((0.5 * v * v + 1.5 * body.velocity[0].powi(2) - 5000.0).abs() < 1e-8);
    assert_eq!(report.contacts, 1);
    assert!(report.dissipated_energy.abs() < 1e-12);
    assert!(fluid.particles()[0].position[0] < body.position[0] - 0.05);
}

#[test]
fn finite_mass_moving_template_transfers_momentum_and_reports_friction_loss() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody};
    let wall = Boxes(vec![([0.0, -1.0, -1.0], [0.001, 1.0, 1.0])]);
    let mut fluid = liquid([0.0, 10.0, 0.0]);
    let mut body = TranslatingBody {
        position: [0.0; 3],
        velocity: [-100.0, 0.0, 0.0],
        mass: 3.0,
    };
    let initial_energy = 15050.0;
    let report = fluid
        .step_with_dynamic_world(
            0.01,
            &mut body,
            &wall,
            DynamicWorldConfig {
                contact: ContactConfig {
                    friction: 1.0,
                    ..ContactConfig::default()
                },
                ..DynamicWorldConfig::default()
            },
        )
        .unwrap();
    let v = fluid.particles()[0].velocity;
    assert!((v[0] + 75.0).abs() < 1e-9);
    assert!((v[1] - 2.5).abs() < 1e-9);
    assert!((v[0] + 3.0 * body.velocity[0] + 300.0).abs() < 1e-9);
    assert!((v[1] + 3.0 * body.velocity[1] - 10.0).abs() < 1e-9);
    let energy = 0.5 * v.iter().map(|x| x * x).sum::<f64>()
        + 1.5 * body.velocity.iter().map(|x| x * x).sum::<f64>();
    assert!((energy + report.dissipated_energy - initial_energy).abs() < 1e-8);
}

#[test]
fn finite_mass_template_query_failure_rolls_back_after_contact() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody};
    let wall = Boxes(vec![([0.0, -1.0, -1.0], [0.001, 1.0, 1.0])]);
    let mut fluid = liquid([100.0, 0.0, 0.0]);
    let mut body = TranslatingBody {
        position: [0.0; 3],
        velocity: [0.0; 3],
        mass: 3.0,
    };
    let before = fluid.clone();
    let body_before = body;
    let result = fluid.step_with_dynamic_world(
        0.01,
        &mut body,
        &wall,
        DynamicWorldConfig {
            max_queries: 3,
            ..DynamicWorldConfig::default()
        },
    );
    assert_eq!(result, Err(Error::CollisionBudget));
    assert_eq!(fluid, before);
    assert_eq!(body, body_before);
}

fn collision_sample_material() -> Material {
    Material {
        rest_density: 1.0,
        sound_speed: 1.0,
        viscosity: 0.1,
    }
}
fn sampled_collision_fluid() -> Liquid {
    use physics::liquid::{BoundarySample, LiquidField, TransportMaterial};
    let initial = liquid([100.0, 2.0, 0.0]);
    let mut fluid = Liquid::new(
        initial.particles().to_vec(),
        vec![collision_sample_material()],
        Config {
            smoothing_radius: 1.0,
            particle_radius: 0.05,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    fluid
        .configure_boundaries(vec![BoundarySample {
            position: [0.0005, 0.0, 0.0],
            volume: 0.01,
        }])
        .unwrap();
    fluid
        .configure_transport(
            vec![LiquidField {
                temperature: 10000.0,
                concentration: 0.0,
            }],
            vec![TransportMaterial {
                specific_heat: 1.0,
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    fluid.set_pressure_work(true).unwrap();
    fluid.set_viscous_heating(true).unwrap();
    fluid
}
fn coupled_energy(fluid: &Liquid, body: physics::liquid::TranslatingBody) -> f64 {
    fluid
        .particles()
        .iter()
        .zip(fluid.fields().unwrap())
        .map(|(p, f)| {
            p.mass * (f.temperature + 0.5 * p.velocity.iter().map(|v| v * v).sum::<f64>())
        })
        .sum::<f64>()
        + 0.5 * body.mass * body.velocity.iter().map(|v| v * v).sum::<f64>()
}
#[test]
fn sampled_pressure_and_swept_contacts_share_momentum_and_energy_balance() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody};
    let wall = Boxes(vec![([0.0, -1.0, -1.0], [0.001, 1.0, 1.0])]);
    let mut fluid = sampled_collision_fluid();
    let mut body = TranslatingBody {
        position: [0.0; 3],
        velocity: [0.0; 3],
        mass: 3.0,
    };
    let initial = coupled_energy(&fluid, body);
    let report = fluid
        .step_with_boundary_world(
            0.01,
            &mut body,
            &wall,
            DynamicWorldConfig {
                contact: ContactConfig {
                    restitution: 0.5,
                    friction: 0.2,
                    ..ContactConfig::default()
                },
                ..DynamicWorldConfig::default()
            },
        )
        .unwrap();
    assert!(report.contacts > 0);
    assert!(report.boundary.pressure_impulse[0].abs() > 1e-8);
    assert!(report.boundary.viscous_heat > 0.0);
    let p = fluid.particles()[0];
    assert!((p.velocity[0] + body.mass * body.velocity[0] - 100.0).abs() < 1e-9);
    assert!((p.velocity[1] + body.mass * body.velocity[1] - 2.0).abs() < 1e-9);
    assert!((coupled_energy(&fluid, body) + report.dissipated_energy - initial).abs() < 1e-7);
    assert!(p.position[0] <= body.position[0] - 0.05);
    // Final boundary follows the actual contact-altered body trajectory.
    let mut expected = Liquid::new(
        vec![p],
        vec![collision_sample_material()],
        Config {
            smoothing_radius: 1.0,
            particle_radius: 0.05,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    expected
        .configure_boundaries(vec![physics::liquid::BoundarySample {
            position: [
                0.0005 + body.position[0],
                body.position[1],
                body.position[2],
            ],
            volume: 0.01,
        }])
        .unwrap();
    expected
        .configure_boundary_velocities(vec![body.velocity])
        .unwrap();
    let actual = fluid.boundary_diagnostics().unwrap();
    let expected = expected.boundary_diagnostics().unwrap();
    assert!((actual.particle_densities[0] - expected.particle_densities[0]).abs() < 1e-10);
}
#[test]
fn coupled_boundary_collision_failure_rolls_back_samples_heat_and_body() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody};
    let wall = Boxes(vec![([0.0, -1.0, -1.0], [0.001, 1.0, 1.0])]);
    let mut fluid = sampled_collision_fluid();
    let mut body = TranslatingBody {
        position: [0.0; 3],
        velocity: [0.0; 3],
        mass: 3.0,
    };
    let before = fluid.clone();
    let before_body = body;
    assert_eq!(
        fluid.step_with_boundary_world(
            0.01,
            &mut body,
            &wall,
            DynamicWorldConfig {
                max_queries: 3,
                ..DynamicWorldConfig::default()
            }
        ),
        Err(Error::CollisionBudget)
    );
    assert_eq!(fluid, before);
    assert_eq!(body, before_body);
}
#[test]
fn coupled_mode_without_samples_matches_dynamic_collision_mode() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody};
    let wall = Boxes(vec![([0.0, -1.0, -1.0], [0.001, 1.0, 1.0])]);
    let mut first = liquid([100.0, 0.0, 0.0]);
    let mut second = first.clone();
    let mut first_body = TranslatingBody {
        position: [0.0; 3],
        velocity: [0.0; 3],
        mass: 3.0,
    };
    let mut second_body = first_body;
    first
        .step_with_boundary_world(0.01, &mut first_body, &wall, DynamicWorldConfig::default())
        .unwrap();
    second
        .step_with_dynamic_world(0.01, &mut second_body, &wall, DynamicWorldConfig::default())
        .unwrap();
    for a in 0..3 {
        assert!(
            (first.particles()[0].position[a] - second.particles()[0].position[a]).abs() < 1e-10
        );
        assert!((first_body.position[a] - second_body.position[a]).abs() < 1e-10);
        assert!((first_body.velocity[a] - second_body.velocity[a]).abs() < 1e-10);
    }
}

#[test]
fn unified_boundary_world_applies_gravity_once_to_each_mass() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody};
    let mut fluid = Liquid::new(
        vec![Particle {
            position: [-0.5, 0.0, 0.0],
            velocity: [0.0; 3],
            mass: 1.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            smoothing_radius: 1.0,
            gravity: [0.0, -10.0, 0.0],
            ..Config::default()
        },
    )
    .unwrap();
    fluid
        .configure_boundaries(vec![physics::liquid::BoundarySample {
            position: [5.0, 0.0, 0.0],
            volume: 0.01,
        }])
        .unwrap();
    let mut body = TranslatingBody {
        position: [0.0; 3],
        velocity: [0.0; 3],
        mass: 3.0,
    };
    let report = fluid
        .step_with_boundary_world(
            0.01,
            &mut body,
            &Boxes(vec![]),
            DynamicWorldConfig::default(),
        )
        .unwrap();
    assert_eq!(report.contacts, 0);
    assert!((fluid.particles()[0].velocity[1] + 0.1).abs() < 1e-12);
    assert!((body.velocity[1] + 0.1).abs() < 1e-12);
    assert!((fluid.particles()[0].position[1] - body.position[1]).abs() < 1e-12);
}

#[test]
fn symmetric_ccd_stops_or_bounces_and_preserves_free_flight() {
    use physics::liquid::{LiquidField, TransportMaterial};
    let prepare = |velocity| {
        let mut state = liquid(velocity);
        state
            .configure_transport(
                vec![LiquidField {
                    temperature: 300.0,
                    concentration: 0.0,
                }],
                vec![TransportMaterial {
                    conductivity: 0.0,
                    ..TransportMaterial::default()
                }],
            )
            .unwrap();
        state.set_viscous_heating(true).unwrap();
        state
    };
    let wall = Boxes(vec![([0.0, -1.0, -1.0], [0.001, 1.0, 1.0])]);
    for restitution in [0.0, 1.0] {
        let mut state = prepare([100.0, 0.0, 0.0]);
        state
            .step_symmetric_with_world(
                0.05,
                &wall,
                ContactConfig {
                    restitution,
                    ..ContactConfig::default()
                },
            )
            .unwrap();
        let expected_position = if restitution < 0.5 { -0.05 } else { -4.6 };
        let expected_velocity = if restitution < 0.5 { 0.0 } else { -100.0 };
        assert!((state.particles()[0].position[0] - expected_position).abs() < 1e-9);
        assert!((state.particles()[0].velocity[0] - expected_velocity).abs() < 1e-12);
        assert!((state.fields().unwrap()[0].temperature - 300.0).abs() < 1e-12);
    }
    let mut with_world = prepare([1.0, 2.0, 3.0]);
    let mut free = with_world.clone();
    free.step_symmetric_free(0.01).unwrap();
    with_world
        .step_symmetric_with_world(0.01, &Boxes(vec![]), ContactConfig::default())
        .unwrap();
    assert_eq!(with_world, free);
    let before = with_world.clone();
    assert!(
        with_world
            .step_symmetric_with_world(
                0.01,
                &wall,
                ContactConfig {
                    max_candidates: 0,
                    ..ContactConfig::default()
                }
            )
            .is_err()
    );
    assert_eq!(with_world, before);
    let mut overlap = prepare([1.0, 0.0, 0.0]);
    let inside = Boxes(vec![([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0])]);
    let before = overlap.clone();
    assert!(
        overlap
            .step_symmetric_with_world(0.01, &inside, ContactConfig::default())
            .is_err()
    );
    assert_eq!(overlap, before);
}

#[test]
fn symmetric_collision_backend_failure_after_drift_rolls_back_heat_and_motion() {
    struct FailingWorld(std::cell::Cell<usize>);
    impl CollisionWorld for FailingWorld {
        type Obstacle = ();
        type Error = ();
        fn sweep_aabb(
            &self,
            _: AnchoredAabb,
            _: [f64; 3],
            _: usize,
        ) -> Result<SweepResult<()>, ()> {
            let count = self.0.get();
            self.0.set(count + 1);
            if count >= 1 {
                return Err(());
            }
            Ok(SweepResult {
                fraction: 1.0,
                normal: [0; 3],
                obstacle: None,
            })
        }
    }
    use physics::liquid::{LiquidField, TransportMaterial};
    let mut state = liquid([1.0, 0.0, 0.0]);
    state
        .configure_transport(
            vec![LiquidField {
                temperature: 300.0,
                concentration: 0.0,
            }],
            vec![TransportMaterial {
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    state.set_viscous_heating(true).unwrap();
    let before = state.clone();
    let backend = FailingWorld(std::cell::Cell::new(0));
    assert_eq!(
        state.step_symmetric_with_world(0.01, &backend, ContactConfig::default()),
        Err(Error::CollisionBackend)
    );
    assert!(backend.0.get() >= 2);
    assert_eq!(state, before);
}

fn heated_impact_liquid() -> Liquid {
    use physics::liquid::{LiquidField, TransportMaterial};
    let mut state = liquid([100.0, 20.0, 0.0]);
    state
        .configure_transport(
            vec![LiquidField {
                temperature: 300.0,
                concentration: 0.0,
            }],
            vec![TransportMaterial {
                specific_heat: 100.0,
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    state.set_viscous_heating(true).unwrap();
    state
}
#[test]
fn symmetric_impact_heat_partition_and_fixture_impulse_balance() {
    let wall = Boxes(vec![([0.0, -3.0, -3.0], [0.001, 3.0, 3.0])]);
    for restitution in [0.0, 0.5, 1.0] {
        for fraction in [0.0, 0.25, 1.0] {
            let mut state = heated_impact_liquid();
            let (_, exchange) = state
                .step_symmetric_with_world_heating(
                    0.05,
                    &wall,
                    ContactConfig {
                        restitution,
                        ..ContactConfig::default()
                    },
                    fraction,
                )
                .unwrap();
            let expected_loss = 5000.0 * (1.0 - restitution * restitution);
            assert!((exchange.dissipated_energy - expected_loss).abs() < 1e-8);
            assert!((exchange.fluid_heat - fraction * expected_loss).abs() < 1e-8);
            assert!((exchange.fixture_energy - (1.0 - fraction) * expected_loss).abs() < 1e-8);
            let particle = state.particles()[0];
            let kinetic: f64 =
                0.5 * particle.mass * particle.velocity.iter().map(|v| v * v).sum::<f64>();
            let thermal = state.transport_totals().unwrap().unwrap().0;
            assert!((kinetic + thermal + exchange.fixture_energy - 35200.0).abs() < 1e-8);
            assert!(
                (exchange.fixture_impulse[0] + particle.mass * particle.velocity[0] - 100.0).abs()
                    < 1e-10
            );
            let final_angular = particle.mass
                * (particle.position[0] * particle.velocity[1]
                    - particle.position[1] * particle.velocity[0]);
            assert!(
                (exchange.fixture_angular_impulse_about_origin[2] + final_angular + 10.0).abs()
                    < 1e-8
            );
        }
    }
}
#[test]
fn impact_heat_enters_latent_plateau_and_property_failure_rolls_back() {
    use physics::liquid::{PhaseChange, PropertyResponse};
    let wall = Boxes(vec![([0.0, -3.0, -3.0], [0.001, 3.0, 3.0])]);
    let mut phase = heated_impact_liquid();
    phase
        .configure_phase_change(
            vec![Some(PhaseChange {
                temperature: 300.0,
                latent_heat: 5000.0,
                high_phase: Material::WATER,
            })],
            vec![0.0],
        )
        .unwrap();
    let (_, exchange) = phase
        .step_symmetric_with_world_heating(0.05, &wall, ContactConfig::default(), 0.5)
        .unwrap();
    assert!((exchange.fluid_heat - 2500.0).abs() < 1e-9);
    assert!((phase.fields().unwrap()[0].temperature - 300.0).abs() < 1e-12);
    assert!((phase.phase_fractions().unwrap()[0] - 0.5).abs() < 1e-12);
    let mut invalid = heated_impact_liquid();
    invalid
        .configure_property_response(vec![Some(PropertyResponse {
            reference_temperature: 300.0,
            thermal_expansion: -1.0,
            viscosity_temperature_rate: 0.0,
            solute: None,
        })])
        .unwrap();
    let before = invalid.clone();
    assert!(
        invalid
            .step_symmetric_with_world_heating(0.05, &wall, ContactConfig::default(), 1.0)
            .is_err()
    );
    assert_eq!(invalid, before);
}

#[test]
fn translating_symmetric_impact_heat_is_galilean_invariant_and_work_balances() {
    let wall = Boxes(vec![([0.0, -3.0, -3.0], [0.001, 3.0, 3.0])]);
    let velocity = [20.0, -3.0, 5.0];
    for restitution in [0.0, 1.0] {
        let mut static_state = heated_impact_liquid();
        let template = static_state.particles()[0];
        let moving_particle = Particle {
            velocity: std::array::from_fn(|a| template.velocity[a] + velocity[a]),
            ..template
        };
        let mut moving = Liquid::new(
            vec![moving_particle],
            vec![Material::WATER],
            Config {
                smoothing_radius: 1.0,
                particle_radius: 0.05,
                gravity: [0.0; 3],
                ..Config::default()
            },
        )
        .unwrap();
        use physics::liquid::{LiquidField, TransportMaterial};
        moving
            .configure_transport(
                vec![LiquidField {
                    temperature: 300.0,
                    concentration: 0.0,
                }],
                vec![TransportMaterial {
                    specific_heat: 100.0,
                    conductivity: 0.0,
                    ..TransportMaterial::default()
                }],
            )
            .unwrap();
        moving.set_viscous_heating(true).unwrap();
        let initial_energy =
            30000.0 + 0.5 * moving_particle.velocity.iter().map(|v| v * v).sum::<f64>();
        let contact = ContactConfig {
            restitution,
            ..ContactConfig::default()
        };
        let (_, stationary) = static_state
            .step_symmetric_with_world_heating(0.05, &wall, contact, 0.25)
            .unwrap();
        let (_, exchange) = moving
            .step_symmetric_with_translating_world_heating(0.05, velocity, &wall, contact, 0.25)
            .unwrap();
        assert!((stationary.fluid_heat - exchange.fluid_heat).abs() < 1e-8);
        for (axis, speed) in velocity.iter().enumerate() {
            let actual = moving.particles()[0];
            let baseline = static_state.particles()[0];
            assert!((actual.position[axis] - baseline.position[axis] - speed * 0.05).abs() < 1e-9);
            assert!((actual.velocity[axis] - baseline.velocity[axis] - speed).abs() < 1e-12);
            let b = (axis + 1) % 3;
            let c = (axis + 2) % 3;
            let before = moving_particle.position[b] * moving_particle.velocity[c]
                - moving_particle.position[c] * moving_particle.velocity[b];
            let after =
                actual.position[b] * actual.velocity[c] - actual.position[c] * actual.velocity[b];
            assert!(
                (exchange.fixture_angular_impulse_about_origin[axis] + after - before).abs() < 1e-8
            );
        }
        let final_energy = moving.transport_totals().unwrap().unwrap().0
            + 0.5
                * moving.particles()[0]
                    .velocity
                    .iter()
                    .map(|v| v * v)
                    .sum::<f64>();
        assert!(
            (final_energy + exchange.fixture_energy - initial_energy - exchange.drive_work).abs()
                < 1e-8
        );
        assert!((exchange.drive_work + 20.0 * exchange.fixture_impulse[0]).abs() < 1e-8);
    }
}

#[test]
fn translating_elastic_wall_can_supply_lab_energy_without_negative_heat() {
    use physics::liquid::ParticleInput;
    let mut state = heated_impact_liquid();
    let stationary = ParticleInput {
        particle: Particle {
            velocity: [0.0; 3],
            ..state.particles()[0]
        },
        field: Some(state.fields().unwrap()[0]),
        phase_fraction: None,
    };
    state.exchange_particles(&[0], &[stationary]).unwrap();
    let wall = Boxes(vec![([0.0, -3.0, -3.0], [0.001, 3.0, 3.0])]);
    let (_, exchange) = state
        .step_symmetric_with_translating_world_heating(
            0.05,
            [-20.0, 0.0, 0.0],
            &wall,
            ContactConfig {
                restitution: 1.0,
                ..ContactConfig::default()
            },
            1.0,
        )
        .unwrap();
    assert!((state.particles()[0].velocity[0] + 40.0).abs() < 1e-10);
    assert!((exchange.drive_work - 800.0).abs() < 1e-8);
    assert!(exchange.fluid_heat.abs() < 1e-10);
    assert!(exchange.dissipated_energy.abs() < 1e-10);
    assert!((state.fields().unwrap()[0].temperature - 300.0).abs() < 1e-12);
}

#[test]
fn symmetric_finite_body_recoil_heat_and_total_momentum_balance() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody};
    let wall = Boxes(vec![([0.0, -3.0, -3.0], [0.001, 3.0, 3.0])]);
    for restitution in [0.0, 0.5, 1.0] {
        let mut fluid = heated_impact_liquid();
        let mut body = TranslatingBody {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 3.0,
        };
        let report = fluid
            .step_symmetric_with_dynamic_world_heating(
                0.05,
                &mut body,
                &wall,
                DynamicWorldConfig {
                    contact: ContactConfig {
                        restitution,
                        ..ContactConfig::default()
                    },
                    ..DynamicWorldConfig::default()
                },
                0.25,
            )
            .unwrap();
        let particle = fluid.particles()[0];
        assert!((particle.velocity[0] - 25.0 * (1.0 - 3.0 * restitution)).abs() < 1e-9);
        assert!((body.velocity[0] - 25.0 * (1.0 + restitution)).abs() < 1e-9);
        let loss = 3750.0 * (1.0 - restitution * restitution);
        assert!((report.dynamics.dissipated_energy - loss).abs() < 1e-8);
        assert!((report.fluid_heat - 0.25 * loss).abs() < 1e-8);
        assert!((report.body_heat - 0.75 * loss).abs() < 1e-8);
        let kinetic = |mass: f64, velocity: [f64; 3]| {
            0.5 * mass * velocity.iter().map(|v| v * v).sum::<f64>()
        };
        let total = kinetic(particle.mass, particle.velocity)
            + kinetic(body.mass, body.velocity)
            + fluid.transport_totals().unwrap().unwrap().0
            + report.body_heat;
        assert!((total - 35200.0).abs() < 1e-8);
        for (axis, before) in [100.0, 20.0, 0.0].iter().enumerate() {
            assert!(
                (particle.mass * particle.velocity[axis] + body.mass * body.velocity[axis]
                    - before)
                    .abs()
                    < 1e-9
            );
        }
        assert!(report.dynamics.contacts > 0);
    }
}
#[test]
fn symmetric_body_and_fluid_rollback_on_impact_heating_domain_failure() {
    use physics::liquid::{DynamicWorldConfig, PropertyResponse, TranslatingBody};
    let mut fluid = heated_impact_liquid();
    fluid
        .configure_property_response(vec![Some(PropertyResponse {
            reference_temperature: 300.0,
            thermal_expansion: -1.0,
            viscosity_temperature_rate: 0.0,
            solute: None,
        })])
        .unwrap();
    let mut body = TranslatingBody {
        position: [0.0; 3],
        velocity: [0.0; 3],
        mass: 3.0,
    };
    let before_fluid = fluid.clone();
    let before_body = body;
    let wall = Boxes(vec![([0.0, -3.0, -3.0], [0.001, 3.0, 3.0])]);
    assert!(
        fluid
            .step_symmetric_with_dynamic_world_heating(
                0.05,
                &mut body,
                &wall,
                DynamicWorldConfig::default(),
                1.0
            )
            .is_err()
    );
    assert_eq!(fluid, before_fluid);
    assert_eq!(body, before_body);
}

#[test]
fn symmetric_empty_fluid_body_follows_exact_constant_gravity() {
    use physics::liquid::{DynamicWorldConfig, TranslatingBody, TransportMaterial};
    let mut fluid = Liquid::new(
        vec![],
        vec![Material::WATER],
        Config {
            smoothing_radius: 1.0,
            ..Config::default()
        },
    )
    .unwrap();
    fluid
        .configure_transport(vec![], vec![TransportMaterial::default()])
        .unwrap();
    fluid.set_viscous_heating(true).unwrap();
    let initial = TranslatingBody {
        position: [0.1, 0.2, 0.3],
        velocity: [1.0, 2.0, 3.0],
        mass: 3.0,
    };
    let mut body = initial;
    let dt = 0.01;
    let report = fluid
        .step_symmetric_with_dynamic_world_heating(
            dt,
            &mut body,
            &Boxes(vec![]),
            DynamicWorldConfig::default(),
            0.5,
        )
        .unwrap();
    for (axis, g) in [0.0, -9.81, 0.0].iter().enumerate() {
        assert!(
            (body.position[axis]
                - initial.position[axis]
                - initial.velocity[axis] * dt
                - 0.5 * g * dt * dt)
                .abs()
                < 1e-12
        );
        assert!((body.velocity[axis] - initial.velocity[axis] - g * dt).abs() < 1e-12);
    }
    assert_eq!(report.dynamics.contacts, 0);
    assert_eq!(report.dynamics.queries, 0);
    assert!(report.fluid_heat.abs() < 1e-12 && report.body_heat.abs() < 1e-12);
}

#[test]
fn finite_thermal_body_temperature_and_coupled_energy_balance() {
    use physics::liquid::{DynamicWorldConfig, ThermalTranslatingBody, TranslatingBody};
    let wall = Boxes(vec![([0.0, -3.0, -3.0], [0.001, 3.0, 3.0])]);
    for fraction in [0.0, 0.25, 1.0] {
        let mut fluid = heated_impact_liquid();
        let mut body = ThermalTranslatingBody {
            mechanics: TranslatingBody {
                position: [0.0; 3],
                velocity: [0.0; 3],
                mass: 3.0,
            },
            specific_heat: 500.0,
            temperature: 290.0,
        };
        let initial_body = body.thermal_energy().unwrap();
        let report = fluid
            .step_symmetric_with_thermal_body(
                0.05,
                &mut body,
                &wall,
                DynamicWorldConfig::default(),
                fraction,
            )
            .unwrap();
        assert!((body.temperature - 290.0 - (1.0 - fraction) * 3750.0 / 1500.0).abs() < 1e-12);
        assert!((body.thermal_energy().unwrap() - initial_body - report.body_heat).abs() < 1e-8);
        let kinetic = |mass: f64, velocity: [f64; 3]| {
            0.5 * mass * velocity.iter().map(|v| v * v).sum::<f64>()
        };
        let particle = fluid.particles()[0];
        let final_energy = fluid.transport_totals().unwrap().unwrap().0
            + body.thermal_energy().unwrap()
            + kinetic(particle.mass, particle.velocity)
            + kinetic(body.mechanics.mass, body.mechanics.velocity);
        assert!((final_energy - 470200.0).abs() < 1e-8);
    }
}
#[test]
fn thermal_body_overflow_after_recoil_rolls_back_both_complete_states() {
    use physics::liquid::{DynamicWorldConfig, ThermalTranslatingBody, TranslatingBody};
    let mut fluid = heated_impact_liquid();
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 3.0,
        },
        specific_heat: 1e-308,
        temperature: 300.0,
    };
    let before_fluid = fluid.clone();
    let before_body = body;
    let wall = Boxes(vec![([0.0, -3.0, -3.0], [0.001, 3.0, 3.0])]);
    assert_eq!(
        fluid.step_symmetric_with_thermal_body(
            0.05,
            &mut body,
            &wall,
            DynamicWorldConfig::default(),
            0.0
        ),
        Err(Error::NumericalFailure)
    );
    assert_eq!(fluid, before_fluid);
    assert_eq!(body, before_body);
}

#[test]
fn combined_body_fluid_thermal_overflow_is_atomic() {
    use physics::liquid::{
        DynamicWorldConfig, LiquidField, ThermalTranslatingBody, TranslatingBody, TransportMaterial,
    };
    let mut fluid = heated_impact_liquid();
    let energy = 0.6 * f64::MAX;
    fluid
        .configure_transport(
            vec![LiquidField {
                temperature: energy / 100.0,
                concentration: 0.0,
            }],
            vec![TransportMaterial {
                specific_heat: 100.0,
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 3.0,
        },
        specific_heat: 500.0,
        temperature: energy / 1500.0,
    };
    assert!(body.thermal_energy().unwrap().is_finite());
    assert!(fluid.transport_totals().unwrap().unwrap().0.is_finite());
    let before_fluid = fluid.clone();
    let before_body = body;
    assert_eq!(
        fluid.step_symmetric_with_thermal_body(
            0.001,
            &mut body,
            &Boxes(vec![]),
            DynamicWorldConfig::default(),
            0.5
        ),
        Err(Error::NumericalFailure)
    );
    assert_eq!(fluid, before_fluid);
    assert_eq!(body, before_body);
}

#[test]
fn finite_body_conduction_matches_two_capacity_solution_and_preserves_motion() {
    use physics::liquid::{ThermalTranslatingBody, TranslatingBody};
    let mut fluid = heated_impact_liquid();
    let before = fluid.particles().to_vec();
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.0; 3],
            velocity: [1.0; 3],
            mass: 3.0,
        },
        specific_heat: 500.0,
        temperature: 290.0,
    };
    let mechanics = body.mechanics;
    let dt: f64 = 0.1;
    let conductance: f64 = 100.0;
    let decay = (-conductance * (1.0 / 100.0 + 1.0 / 1500.0) * dt).exp();
    let equilibrium = 290.625;
    let heat = fluid
        .exchange_body_heat(dt, &mut body, &[conductance])
        .unwrap();
    assert!(
        (fluid.fields().unwrap()[0].temperature - (equilibrium + (300.0 - equilibrium) * decay))
            .abs()
            < 1e-12
    );
    assert!((body.temperature - (equilibrium + (290.0 - equilibrium) * decay)).abs() < 1e-12);
    assert!(heat < 0.0);
    assert!(
        (fluid.transport_totals().unwrap().unwrap().0 + body.thermal_energy().unwrap() - 465000.0)
            .abs()
            < 1e-8
    );
    assert_eq!(fluid.particles(), before);
    assert_eq!(body.mechanics, mechanics);
}
#[test]
fn finite_body_conduction_accounts_for_latent_heat_and_rolls_back_invalid_properties() {
    use physics::liquid::{PhaseChange, PropertyResponse, ThermalTranslatingBody, TranslatingBody};
    let make_body = || ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 3.0,
        },
        specific_heat: 500.0,
        temperature: 310.0,
    };
    let mut fluid = heated_impact_liquid();
    fluid
        .configure_phase_change(
            vec![Some(PhaseChange {
                temperature: 300.0,
                latent_heat: 5000.0,
                high_phase: Material::WATER,
            })],
            vec![0.0],
        )
        .unwrap();
    let mut body = make_body();
    let heat = fluid.exchange_body_heat(1.0, &mut body, &[100.0]).unwrap();
    let expected_heat = 15000.0 * (1.0 - (-100.0_f64 / 1500.0).exp());
    assert!((heat - expected_heat).abs() < 1e-9);
    assert!((fluid.phase_fractions().unwrap()[0] - expected_heat / 5000.0).abs() < 1e-12);
    assert!((fluid.fields().unwrap()[0].temperature - 300.0).abs() < 1e-12);
    assert!((body.temperature - (310.0 - expected_heat / 1500.0)).abs() < 1e-12);
    assert!(
        (fluid.transport_totals().unwrap().unwrap().0 + body.thermal_energy().unwrap() - 495000.0)
            .abs()
            < 1e-8
    );
    fluid.exchange_body_heat(1.0, &mut body, &[1e300]).unwrap();
    assert!((body.temperature - 306.25).abs() < 1e-10);
    assert!((fluid.fields().unwrap()[0].temperature - 306.25).abs() < 1e-10);
    let mut invalid = heated_impact_liquid();
    invalid
        .configure_property_response(vec![Some(PropertyResponse {
            reference_temperature: 300.0,
            thermal_expansion: -1.0,
            viscosity_temperature_rate: 0.0,
            solute: None,
        })])
        .unwrap();
    let mut body = make_body();
    let before = invalid.clone();
    let body_before = body;
    assert!(
        invalid
            .exchange_body_heat(1.0, &mut body, &[1e300])
            .is_err()
    );
    assert_eq!(invalid, before);
    assert_eq!(body, body_before);
}

#[test]
fn coupled_body_conduction_and_recoil_count_each_heat_transfer_once() {
    use physics::liquid::{DynamicWorldConfig, ThermalTranslatingBody, TranslatingBody};
    let mut fluid = heated_impact_liquid();
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 3.0,
        },
        specific_heat: 500.0,
        temperature: 290.0,
    };
    let initial_body = body.thermal_energy().unwrap();
    let wall = Boxes(vec![([0.0, -3.0, -3.0], [0.001, 3.0, 3.0])]);
    let report = fluid
        .step_symmetric_with_thermal_body_conduction(
            0.05,
            &mut body,
            &wall,
            DynamicWorldConfig::default(),
            0.25,
            &[100.0],
        )
        .unwrap();
    assert!(report.conductive_heat < 0.0);
    assert!(
        (body.thermal_energy().unwrap() - initial_body - report.impacts.body_heat
            + report.conductive_heat)
            .abs()
            < 1e-8
    );
    let kinetic =
        |mass: f64, velocity: [f64; 3]| 0.5 * mass * velocity.iter().map(|v| v * v).sum::<f64>();
    let p = fluid.particles()[0];
    let total = kinetic(p.mass, p.velocity)
        + kinetic(body.mechanics.mass, body.mechanics.velocity)
        + fluid.transport_totals().unwrap().unwrap().0
        + body.thermal_energy().unwrap();
    assert!((total - 470200.0).abs() < 1e-8);
}

#[test]
fn symmetric_body_heat_second_half_failure_rolls_back_first_half() {
    use physics::liquid::{PropertyResponse, ThermalTranslatingBody, TranslatingBody};
    let mut fluid = heated_impact_liquid();
    fluid
        .configure_property_response(vec![Some(PropertyResponse {
            reference_temperature: 300.0,
            thermal_expansion: -0.2,
            viscosity_temperature_rate: 0.0,
            solute: None,
        })])
        .unwrap();
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 3.0,
        },
        specific_heat: 500.0,
        temperature: 310.0,
    };
    let mut probe = fluid.clone();
    let mut probe_body = body;
    probe
        .exchange_body_heat(0.5, &mut probe_body, &[100.0])
        .unwrap();
    assert!(probe.fields().unwrap()[0].temperature < 305.0);
    let before = fluid.clone();
    let before_body = body;
    assert!(
        fluid
            .exchange_body_heat_symmetric(1.0, &mut body, &[100.0])
            .is_err()
    );
    assert_eq!(fluid, before);
    assert_eq!(body, before_body);
}

#[test]
fn exact_finite_body_heat_crosses_latent_boundaries_independently_of_partition() {
    use physics::liquid::{
        LiquidField, PhaseChange, ThermalTranslatingBody, TranslatingBody, TransportMaterial,
    };
    for heating in [true, false] {
        let mut fluid = heated_impact_liquid();
        fluid
            .configure_transport(
                vec![LiquidField {
                    temperature: if heating { 290.0 } else { 330.0 },
                    concentration: 0.0,
                }],
                vec![TransportMaterial {
                    specific_heat: 100.0,
                    conductivity: 0.0,
                    ..TransportMaterial::default()
                }],
            )
            .unwrap();
        fluid
            .configure_phase_change(
                vec![Some(PhaseChange {
                    temperature: 300.0,
                    latent_heat: 5000.0,
                    high_phase: Material::WATER,
                })],
                vec![if heating { 0.0 } else { 1.0 }],
            )
            .unwrap();
        let mut body = ThermalTranslatingBody {
            mechanics: TranslatingBody {
                position: [0.0; 3],
                velocity: [0.0; 3],
                mass: 3.0,
            },
            specific_heat: 500.0,
            temperature: if heating { 320.0 } else { 280.0 },
        };
        let initial_energy =
            fluid.transport_totals().unwrap().unwrap().0 + body.thermal_energy().unwrap();
        let mut partitioned = fluid.clone();
        let mut small_body = body;
        fluid.exchange_body_heat(0.5, &mut body, &[1000.0]).unwrap();
        for _ in 0..100 {
            partitioned
                .exchange_body_heat(0.005, &mut small_body, &[1000.0])
                .unwrap();
        }
        assert!(
            (fluid.fields().unwrap()[0].temperature - partitioned.fields().unwrap()[0].temperature)
                .abs()
                < 1e-9
        );
        assert!((body.temperature - small_body.temperature).abs() < 1e-9);
        assert!(
            (fluid.phase_fractions().unwrap()[0] - if heating { 1.0 } else { 0.0 }).abs() < 1e-12
        );
        assert!(
            (fluid.transport_totals().unwrap().unwrap().0 + body.thermal_energy().unwrap()
                - initial_energy)
                .abs()
                < 1e-8
        );
    }
}

#[test]
fn plane_thermal_stencil_tiles_area_and_is_translation_invariant() {
    use physics::liquid::{
        LiquidField, ThermalPlanePatch, ThermalTranslatingBody, TranslatingBody, TransportMaterial,
    };
    for resolution in [1, 2, 4] {
        let spacing = 1.0 / f64::from(resolution);
        let offset = [2.0, 3.0, 4.0];
        let mut particles = Vec::new();
        for y in 0..resolution {
            for z in 0..resolution {
                particles.push(Particle {
                    position: [
                        offset[0] + 0.5,
                        offset[1] + (f64::from(y) + 0.5) * spacing - 0.5,
                        offset[2] + (f64::from(z) + 0.5) * spacing - 0.5,
                    ],
                    velocity: [0.0; 3],
                    mass: 1000.0 * spacing.powi(3),
                    material: 0,
                });
            }
        }
        let count = particles.len();
        let mut fluid = Liquid::new(
            particles,
            vec![Material::WATER],
            Config {
                smoothing_radius: 2.0,
                ..Config::default()
            },
        )
        .unwrap();
        fluid
            .configure_transport(
                vec![
                    LiquidField {
                        temperature: 300.0,
                        concentration: 0.0
                    };
                    count
                ],
                vec![TransportMaterial {
                    conductivity: 10.0,
                    ..TransportMaterial::default()
                }],
            )
            .unwrap();
        let body = ThermalTranslatingBody {
            mechanics: TranslatingBody {
                position: offset,
                velocity: [0.0; 3],
                mass: 3.0,
            },
            specific_heat: 500.0,
            temperature: 290.0,
        };
        let patch = ThermalPlanePatch {
            center: [0.0; 3],
            axis: 0,
            side: 1,
            half_extents: [0.5; 2],
            max_gap: 0.5,
            body_thickness: 0.25,
            body_conductivity: 5.0,
            contact_resistance: 0.0,
        };
        let before = fluid.clone();
        let total: f64 = fluid
            .body_plane_conductances(&body, patch)
            .unwrap()
            .iter()
            .sum();
        // One square metre; 0.5/10 + 0.25/5 = 0.1 m² K/W.
        assert!((total - 10.0).abs() < 1e-12);
        let contact_total: f64 = fluid
            .body_plane_conductances(
                &body,
                ThermalPlanePatch {
                    contact_resistance: 0.1,
                    ..patch
                },
            )
            .unwrap()
            .iter()
            .sum();
        assert!((contact_total - 5.0).abs() < 1e-12);
        assert_eq!(fluid, before);
    }
}
#[test]
fn plane_thermal_stencil_clips_footprints_and_rejects_disconnected_cells() {
    use physics::liquid::{
        LiquidField, ThermalPlanePatch, ThermalTranslatingBody, TranslatingBody, TransportMaterial,
    };
    let mut fluid = Liquid::new(
        [[0.5, 0.5, 0.0], [-0.5, 0.0, 0.0], [1.0, 0.0, 0.0]]
            .into_iter()
            .map(|position| Particle {
                position,
                velocity: [0.0; 3],
                mass: 1000.0,
                material: 0,
            })
            .collect(),
        vec![Material::WATER],
        Config {
            smoothing_radius: 2.0,
            ..Config::default()
        },
    )
    .unwrap();
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.0
                };
                3
            ],
            vec![TransportMaterial {
                conductivity: 10.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    let body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 3.0,
        },
        specific_heat: 500.0,
        temperature: 290.0,
    };
    let patch = ThermalPlanePatch {
        center: [0.0; 3],
        axis: 0,
        side: 1,
        half_extents: [0.5; 2],
        max_gap: 0.0,
        body_thickness: 0.25,
        body_conductivity: 5.0,
        contact_resistance: 0.0,
    };
    let conductances = fluid.body_plane_conductances(&body, patch).unwrap();
    assert!((conductances[0] - 5.0).abs() < 1e-12);
    assert!(conductances[1].abs() < 1e-12 && conductances[2].abs() < 1e-12);
    let insulated = fluid
        .body_plane_conductances(
            &body,
            ThermalPlanePatch {
                body_conductivity: 0.0,
                ..patch
            },
        )
        .unwrap();
    assert!(insulated.iter().all(|g| g.abs() < 1e-12));
}

#[test]
fn moving_thermal_plane_rebuilds_links_and_preserves_energy() {
    use physics::liquid::{
        DynamicWorldConfig, LiquidField, ThermalPlanePatch, ThermalTranslatingBody,
        TranslatingBody, TransportMaterial,
    };
    let mut fluid = Liquid::new(
        vec![Particle {
            position: [0.5, 0.0, 0.0],
            velocity: [20.0, 0.0, 0.0],
            mass: 1.0,
            material: 0,
        }],
        vec![Material {
            rest_density: 1.0,
            sound_speed: 20.0,
            viscosity: 0.0,
        }],
        Config {
            smoothing_radius: 2.0,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    fluid
        .configure_transport(
            vec![LiquidField {
                temperature: 300.0,
                concentration: 0.0,
            }],
            vec![TransportMaterial {
                specific_heat: 100.0,
                conductivity: 10.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    fluid.set_viscous_heating(true).unwrap();
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 3.0,
        },
        specific_heat: 500.0,
        temperature: 290.0,
    };
    let patch = ThermalPlanePatch {
        center: [0.0; 3],
        axis: 0,
        side: 1,
        half_extents: [0.5; 2],
        max_gap: 0.0,
        body_thickness: 0.0,
        body_conductivity: 0.0,
        contact_resistance: 0.0,
    };
    let report = fluid
        .step_symmetric_with_thermal_plane(
            0.05,
            &mut body,
            &Boxes(vec![]),
            DynamicWorldConfig::default(),
            1.0,
            patch,
        )
        .unwrap();
    // G=20 W/K acts for the first half only; motion opens the link.
    let reduced_capacity = 100.0 * 1500.0 / 1600.0;
    let expected = -10.0 * reduced_capacity * (1.0 - (-20.0_f64 * 0.025 / reduced_capacity).exp());
    assert!((report.conductive_heat - expected).abs() < 1e-8);
    assert_eq!(
        fluid.body_plane_conductances(&body, patch).unwrap(),
        vec![0.0]
    );
    assert!(
        (fluid.transport_totals().unwrap().unwrap().0 + body.thermal_energy().unwrap() - 465000.0)
            .abs()
            < 1e-8
    );
    let before = fluid.clone();
    let body_before = body;
    assert!(
        fluid
            .step_symmetric_with_thermal_plane(
                0.05,
                &mut body,
                &Boxes(vec![]),
                DynamicWorldConfig::default(),
                1.0,
                ThermalPlanePatch { axis: 3, ..patch }
            )
            .is_err()
    );
    assert_eq!(fluid, before);
    assert_eq!(body, body_before);
}
