use physics::secondary_motion::{Config, SecondaryMotion};
#[test]
fn exact_constant_force_is_step_independent_for_all_damping_regimes() {
    for damping_ratio in [0.0, 0.16, 1.0, 2.0] {
        let mut coarse = SecondaryMotion::new(Config {
            frequency: 4.0,
            damping_ratio,
        })
        .unwrap();
        let mut fine = coarse.clone();
        for _ in 0..10 {
            coarse.step(0.1, [2., -5., 1.], [0., -9.81, 0.]).unwrap();
        }
        for _ in 0..1000 {
            fine.step(0.001, [2., -5., 1.], [0., -9.81, 0.]).unwrap();
        }
        for (a, b) in coarse.offset().into_iter().zip(fine.offset()) {
            assert!((a - b).abs() < 1e-12);
        }
        for (a, b) in coarse.velocity().into_iter().zip(fine.velocity()) {
            assert!((a - b).abs() < 1e-11);
        }
    }
}
#[test]
fn inertia_sag_and_settling() {
    let mut spring = SecondaryMotion::new(Config::default()).unwrap();
    spring.step(0.01, [0., 10., 0.], [0.; 3]).unwrap();
    assert!(spring.offset()[1] < 0.0);
    for _ in 0..1000 {
        spring.step(0.01, [0.; 3], [0., -9.81, 0.]).unwrap();
    }
    let equilibrium = -9.81 / (std::f64::consts::TAU * 4.0).powi(2);
    assert!((spring.offset()[1] - equilibrium).abs() < 1e-10);
    assert!(spring.velocity()[1].abs() < 1e-10);
}
#[test]
fn invalid_inputs_and_reconfiguration_are_atomic() {
    let mut spring = SecondaryMotion::new(Config::default()).unwrap();
    spring.step(0.01, [1.; 3], [0.; 3]).unwrap();
    let before = (spring.offset(), spring.velocity());
    assert!(spring.step(f64::NAN, [0.; 3], [0.; 3]).is_err());
    assert!(
        spring
            .configure(Config {
                frequency: 0.,
                damping_ratio: 1.
            })
            .is_err()
    );
    assert_eq!(before, (spring.offset(), spring.velocity()));
    spring
        .configure(Config {
            frequency: 6.,
            damping_ratio: 1.,
        })
        .unwrap();
    assert_eq!(before, (spring.offset(), spring.velocity()));
    spring.reset();
    assert_eq!(spring.offset(), [0.; 3]);
}
#[test]
fn hardening_reduces_static_deflection_and_contacts_stop_penetration() {
    use physics::secondary_motion::ContactPlane;
    let mut linear = SecondaryMotion::new(Config::default()).unwrap();
    let mut hard = linear.clone();
    for _ in 0..2000 {
        linear
            .step_nonlinear(0.005, [0.; 3], [0., -80., 0.], 0.0, &[])
            .unwrap();
        hard.step_nonlinear(0.005, [0.; 3], [0., -80., 0.], 2000.0, &[])
            .unwrap();
    }
    assert!(hard.offset()[1].abs() < linear.offset()[1].abs() * 0.6);
    let plane = ContactPlane {
        normal: [0., 1., 0.],
        limit: -0.02,
        friction: 0.5,
    };
    for _ in 0..1000 {
        hard.step_nonlinear(0.005, [0.; 3], [0., -80., 0.], 2000.0, &[plane])
            .unwrap();
        assert!(hard.offset()[1] >= -0.02 - 1e-10);
    }
    assert!(hard.velocity()[1].abs() < 1e-10);
    for _ in 0..2000 {
        hard.step_nonlinear(0.005, [0.; 3], [0.; 3], 2000.0, &[plane])
            .unwrap();
    }
    assert!(hard.offset()[1].abs() < 1e-10);
}
#[test]
fn incompatible_contacts_rollback_and_stiff_material_stays_finite() {
    use physics::secondary_motion::ContactPlane;
    let mut motion = SecondaryMotion::new(Config {
        frequency: 1000.0,
        damping_ratio: 0.2,
    })
    .unwrap();
    for _ in 0..100 {
        motion
            .step_nonlinear(0.1, [5.; 3], [0.; 3], 1e6, &[])
            .unwrap();
    }
    let old = (motion.offset(), motion.velocity());
    let contacts = [
        ContactPlane {
            normal: [1., 0., 0.],
            limit: 1.,
            friction: 0.,
        },
        ContactPlane {
            normal: [-1., 0., 0.],
            limit: 1.,
            friction: 0.,
        },
    ];
    assert!(
        motion
            .step_nonlinear(0.01, [0.; 3], [0.; 3], 1.0, &contacts)
            .is_err()
    );
    assert_eq!(old, (motion.offset(), motion.velocity()));
}
