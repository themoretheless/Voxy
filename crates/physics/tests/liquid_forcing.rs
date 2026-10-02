#![allow(clippy::float_cmp)] // Exactly representable impulses and unchanged positions.
use physics::liquid::{Config, Error, Liquid, Material, Particle};
fn fluid() -> Liquid {
    Liquid::new(
        vec![
            Particle {
                position: [0.0; 3],
                velocity: [1.0, 0.0, 0.0],
                mass: 2.0,
                material: 0,
            },
            Particle {
                position: [2.0, 0.0, 0.0],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            },
        ],
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap()
}
fn kinetic(fluid: &Liquid) -> f64 {
    fluid
        .particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum()
}
#[test]
fn forcing_work_matches_kinetic_change_and_impulse_matches_momentum_change() {
    let mut fluid = fluid();
    let before = kinetic(&fluid);
    let report = fluid
        .apply_impulses(&[[2.0, 0.0, 0.0], [0.0, 3.0, 0.0]])
        .unwrap();
    assert!((kinetic(&fluid) - before - report.work).abs() < 1e-12);
    assert_eq!(report.impulse, [2.0, 3.0, 0.0]);
    assert_eq!(fluid.particles()[0].velocity, [2.0, 0.0, 0.0]);
    assert_eq!(fluid.particles()[0].position, [0.0; 3]);
}
#[test]
fn braking_has_negative_work_and_force_duration_scales_impulse() {
    let mut fluid = fluid();
    let report = fluid
        .apply_forces(0.5, &[[-4.0, 0.0, 0.0], [0.0; 3]])
        .unwrap();
    assert!((report.work + 1.0).abs() < 1e-12);
    assert!(fluid.particles()[0].velocity[0].abs() < 1e-12);
    fluid.step(0.01, None).unwrap();
    assert!(fluid.particles()[0].position[0].abs() < 1e-12);
}
#[test]
fn wrong_counts_nonfinite_and_overflow_are_atomic() {
    let mut fluid = fluid();
    let before = fluid.clone();
    assert_eq!(fluid.apply_impulses(&[]), Err(Error::InvalidParticle));
    assert_eq!(
        fluid.apply_impulses(&[[0.0; 3], [f64::NAN, 0.0, 0.0]]),
        Err(Error::InvalidParticle)
    );
    assert_eq!(
        fluid.apply_impulses(&[[1.0, 0.0, 0.0], [f64::MAX, 0.0, 0.0]]),
        Err(Error::NumericalFailure)
    );
    assert_eq!(
        fluid.apply_forces(2.0, &[[0.0; 3], [f64::MAX, 0.0, 0.0]]),
        Err(Error::NumericalFailure)
    );
    assert_eq!(
        fluid.apply_forces(0.0, &[[0.0; 3]; 2]),
        Err(Error::InvalidTimeStep)
    );
    assert_eq!(fluid, before);
}
