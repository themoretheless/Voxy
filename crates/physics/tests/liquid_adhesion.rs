// Exact comparisons verify unchanged values and invariant particle masses.
#![allow(clippy::float_cmp)]
use physics::liquid::{Config, Container, Error, Liquid, Material, Particle, WallAdhesion};

fn fluid(x: f64) -> Liquid {
    Liquid::new(
        vec![Particle {
            position: [x, 0.0, 0.0],
            velocity: [0.0; 3],
            mass: 1.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 0.2,
            particle_radius: 0.05,
            ..Config::default()
        },
    )
    .unwrap()
}
fn walls() -> Container {
    Container {
        min: [-1.0; 3],
        max: [1.0; 3],
        restitution: 0.0,
        friction: 0.0,
    }
}
fn enable(liquid: &mut Liquid) {
    liquid
        .set_wall_adhesion(
            0,
            Some(WallAdhesion {
                acceleration: 2.0,
                range: 0.25,
            }),
        )
        .unwrap();
}
#[test]
fn wall_potential_is_local_symmetric_and_preserves_mass() {
    let mut left = fluid(-0.85);
    let mut right = fluid(0.85);
    let mut middle = fluid(0.0);
    for liquid in [&mut left, &mut right, &mut middle] {
        enable(liquid);
        liquid.step(0.01, Some(walls())).unwrap();
        assert_eq!(liquid.mass(), 1.0);
    }
    assert!(left.particles()[0].velocity[0] < 0.0);
    assert!((left.particles()[0].velocity[0] + right.particles()[0].velocity[0]).abs() < 1e-12);
    assert_eq!(middle.particles()[0].velocity, [0.0; 3]);
    assert_eq!(left.particles()[0].velocity[1], 0.0);
}
#[test]
fn adhesion_reaches_wall_without_penetration_and_disable_removes_force() {
    let mut liquid = fluid(-0.85);
    enable(&mut liquid);
    for _ in 0..100 {
        liquid.step(0.01, Some(walls())).unwrap();
    }
    assert!((liquid.particles()[0].position[0] + 0.95).abs() < 1e-12);
    assert_eq!(liquid.particles()[0].velocity[0], 0.0);
    let mut disabled = fluid(-0.85);
    enable(&mut disabled);
    disabled.set_wall_adhesion(0, None).unwrap();
    disabled.step(0.1, Some(walls())).unwrap();
    assert_eq!(disabled.particles()[0].position[0], -0.85);
}
#[test]
fn invalid_configuration_and_substep_exhaustion_are_atomic() {
    let mut liquid = fluid(-0.85);
    let before = liquid.clone();
    assert_eq!(
        liquid.set_wall_adhesion(
            0,
            Some(WallAdhesion {
                acceleration: f64::NAN,
                range: 0.1
            })
        ),
        Err(Error::InvalidWallAdhesion)
    );
    assert_eq!(liquid, before);
    liquid
        .set_wall_adhesion(
            0,
            Some(WallAdhesion {
                acceleration: 1e12,
                range: 0.1,
            }),
        )
        .unwrap();
    let before = liquid.clone();
    assert_eq!(liquid.step(0.1, Some(walls())), Err(Error::SubstepBudget));
    assert_eq!(liquid, before);
}

#[test]
fn material_selection_and_absent_container_do_not_apply_unwanted_forces() {
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [-0.85, 0.0, 0.0],
            velocity: [0.0; 3],
            mass: 1.0,
            material: 1,
        }],
        vec![Material::WATER, Material::OIL],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 0.2,
            particle_radius: 0.05,
            ..Config::default()
        },
    )
    .unwrap();
    enable(&mut liquid);
    liquid.step(0.01, Some(walls())).unwrap();
    assert_eq!(liquid.particles()[0].velocity, [0.0; 3]);
    let mut free = fluid(-0.85);
    enable(&mut free);
    free.step(0.01, None).unwrap();
    assert_eq!(free.particles()[0].velocity, [0.0; 3]);
}
