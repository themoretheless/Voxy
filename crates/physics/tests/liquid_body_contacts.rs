use physics::liquid::{BodyContactConfig, Config, Error, FloatingBody, Liquid, Material, Particle};
fn fluid() -> Liquid {
    Liquid::new(
        vec![Particle {
            position: [-1.0, 0.0, 0.0],
            velocity: [100.0, 0.0, 0.0],
            mass: 1.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 1.0,
            particle_radius: 0.05,
            ..Config::default()
        },
    )
    .unwrap()
}
fn body() -> FloatingBody {
    FloatingBody {
        position: [0.0; 3],
        velocity: [0.0; 3],
        mass: 2.0,
        radius: 0.1,
    }
}
fn kinetic(fluid: &Liquid, bodies: &[FloatingBody]) -> f64 {
    fluid
        .particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum::<f64>()
        + bodies
            .iter()
            .map(|b| 0.5 * b.mass * b.velocity.iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>()
}
#[test]
fn fast_particle_hits_moving_body_with_equal_opposite_impulse_and_no_tunnelling() {
    let mut fluid = fluid();
    let mut bodies = [body()];
    let before = kinetic(&fluid, &bodies);
    let report = fluid
        .step_with_bodies(
            0.02,
            &mut bodies,
            BodyContactConfig {
                restitution: 1.0,
                ..BodyContactConfig::default()
            },
        )
        .unwrap();
    assert_eq!(report.contacts, 1);
    assert!((fluid.particles()[0].velocity[0] + 100.0 / 3.0).abs() < 1e-9);
    assert!((bodies[0].velocity[0] - 200.0 / 3.0).abs() < 1e-9);
    assert!((fluid.particles()[0].velocity[0] + 2.0 * bodies[0].velocity[0] - 100.0).abs() < 1e-9);
    assert!((kinetic(&fluid, &bodies) - before).abs() < 1e-7);
    assert!(bodies[0].position[0] - fluid.particles()[0].position[0] >= 0.15);
}
#[test]
fn inelastic_contact_accounts_for_lost_kinetic_energy() {
    let mut fluid = fluid();
    let mut bodies = [body()];
    let before = kinetic(&fluid, &bodies);
    let report = fluid
        .step_with_bodies(0.02, &mut bodies, BodyContactConfig::default())
        .unwrap();
    assert_eq!(report.contacts, 1);
    assert!((before - kinetic(&fluid, &bodies) - report.dissipated_energy).abs() < 1e-7);
    assert!((fluid.particles()[0].velocity[0] - bodies[0].velocity[0]).abs() < 1e-9);
}
#[test]
fn body_body_collision_works_with_an_empty_fluid() {
    let mut fluid = Liquid::new(
        vec![],
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    let mut bodies = [
        body(),
        FloatingBody {
            position: [1.0, 0.0, 0.0],
            velocity: [-100.0, 0.0, 0.0],
            ..body()
        },
    ];
    let report = fluid
        .step_with_bodies(
            0.02,
            &mut bodies,
            BodyContactConfig {
                restitution: 1.0,
                ..BodyContactConfig::default()
            },
        )
        .unwrap();
    assert_eq!(report.contacts, 1);
    assert!((bodies[0].velocity[0] + 100.0).abs() < 1e-9);
    assert!(bodies[1].velocity[0].abs() < 1e-9);
}
#[test]
fn overlap_and_check_exhaustion_roll_back_fluid_and_bodies() {
    let mut fluid = fluid();
    let initial = fluid.clone();
    let mut overlapping = [FloatingBody {
        position: [-1.0, 0.0, 0.0],
        ..body()
    }];
    let before = overlapping;
    assert_eq!(
        fluid.step_with_bodies(0.01, &mut overlapping, BodyContactConfig::default()),
        Err(Error::InitialOverlap)
    );
    assert_eq!(fluid, initial);
    assert_eq!(overlapping, before);
    let mut bodies = [body()];
    let before = bodies;
    assert_eq!(
        fluid.step_with_bodies(
            0.02,
            &mut bodies,
            BodyContactConfig {
                max_checks: 1,
                ..BodyContactConfig::default()
            }
        ),
        Err(Error::CollisionBudget)
    );
    assert_eq!(fluid, initial);
    assert_eq!(bodies, before);
}
#[test]
fn moving_body_pushes_a_stationary_fluid_particle_locally() {
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 1.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 1.0,
            particle_radius: 0.05,
            ..Config::default()
        },
    )
    .unwrap();
    let mut bodies = [FloatingBody {
        position: [-1.0, 0.0, 0.0],
        velocity: [100.0, 0.0, 0.0],
        ..body()
    }];
    let report = liquid
        .step_with_bodies(
            0.02,
            &mut bodies,
            BodyContactConfig {
                restitution: 1.0,
                ..BodyContactConfig::default()
            },
        )
        .unwrap();
    assert_eq!(report.contacts, 1);
    assert!((liquid.particles()[0].velocity[0] - 400.0 / 3.0).abs() < 1e-9);
    assert!((bodies[0].velocity[0] - 100.0 / 3.0).abs() < 1e-9);
    assert!(liquid.particles()[0].position[0] > bodies[0].position[0] + 0.15);
}
#[test]
fn contact_limit_after_an_actual_body_collision_restores_all_bodies() {
    let mut liquid = Liquid::new(
        vec![],
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    let mut bodies = [
        FloatingBody {
            velocity: [100.0, 0.0, 0.0],
            ..body()
        },
        FloatingBody {
            position: [1.0, 0.0, 0.0],
            ..body()
        },
        FloatingBody {
            position: [2.0, 0.0, 0.0],
            ..body()
        },
    ];
    let before = bodies;
    assert_eq!(
        liquid.step_with_bodies(
            0.02,
            &mut bodies,
            BodyContactConfig {
                restitution: 1.0,
                max_contacts: 1,
                ..BodyContactConfig::default()
            }
        ),
        Err(Error::CollisionBudget)
    );
    assert_eq!(bodies, before);
    let report = liquid
        .step_with_bodies(
            0.02,
            &mut bodies,
            BodyContactConfig {
                restitution: 1.0,
                ..BodyContactConfig::default()
            },
        )
        .unwrap();
    assert_eq!(report.contacts, 2);
}
#[test]
fn oblique_spherical_contact_conserves_angular_momentum() {
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [-1.0, 0.1, 0.0],
            velocity: [100.0, 0.0, 0.0],
            mass: 1.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 1.0,
            particle_radius: 0.05,
            ..Config::default()
        },
    )
    .unwrap();
    let mut bodies = [body()];
    let before = kinetic(&liquid, &bodies);
    liquid
        .step_with_bodies(
            0.02,
            &mut bodies,
            BodyContactConfig {
                restitution: 1.0,
                ..BodyContactConfig::default()
            },
        )
        .unwrap();
    let particle = liquid.particles()[0];
    let sphere = bodies[0];
    let angular = particle.mass
        * (particle.position[0] * particle.velocity[1]
            - particle.position[1] * particle.velocity[0])
        + sphere.mass
            * (sphere.position[0] * sphere.velocity[1] - sphere.position[1] * sphere.velocity[0]);
    assert!((angular + 10.0).abs() < 1e-8);
    assert!(particle.velocity[1] > 0.0 && sphere.velocity[1] < 0.0);
    assert!((particle.mass * particle.velocity[1] + sphere.mass * sphere.velocity[1]).abs() < 1e-9);
    assert!((kinetic(&liquid, &bodies) - before).abs() < 1e-7);
}
#[test]
fn a_fast_near_miss_does_not_generate_a_contact() {
    let mut liquid = fluid();
    let mut bodies = [FloatingBody {
        position: [0.0, 0.3, 0.0],
        ..body()
    }];
    let report = liquid
        .step_with_bodies(0.02, &mut bodies, BodyContactConfig::default())
        .unwrap();
    assert_eq!(report.contacts, 0);
    assert!(bodies[0].velocity.iter().all(|v| v.abs() < 1e-12));
    assert!((liquid.particles()[0].position[0] - 1.0).abs() < 1e-9);
}
