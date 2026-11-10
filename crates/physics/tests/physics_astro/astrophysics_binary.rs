use physics::{
    astrophysics::OrbitalState,
    astrophysics_binary::{Binary, Body, StepError, Tide},
    astrophysics_spin::Spin,
};
fn body(mass: f64, radius: f64) -> Body {
    Body {
        mass,
        radius,
        spin: Spin {
            orientation: [0.0, 0.0, 0.0, 1.0],
            angular_momentum: [0.0; 3],
            inertia: [0.4 * mass * radius * radius; 3],
        },
        tide: Tide::default(),
        heat: 0.0,
    }
}
fn binary() -> Binary {
    Binary {
        relative: OrbitalState {
            position: [2.0, 0.0, 0.0],
            velocity: [0.0, (11.0_f64 / 2.0).sqrt(), 0.0],
        },
        primary: body(10.0, 0.3),
        secondary: body(1.0, 0.5),
    }
}
fn near(a: f64, b: f64, tolerance: f64) {
    assert!((a - b).abs() < tolerance, "{a} != {b}");
}
#[test]
fn figure_torque_has_orbital_backreaction() {
    let mut system = binary();
    let angle = 0.3_f64;
    system.secondary.spin.orientation = [0.0, 0.0, angle.sin(), angle.cos()];
    system.secondary.spin.inertia = [0.03, 0.07, 0.08];
    system.secondary.spin.angular_momentum = [0.01, 0.02, 0.03];
    let initial = system.diagnostics(1.0).unwrap();
    for _ in 0..10_000 {
        system.step(1.0, 0.001, 0.001, 1).unwrap();
        let actual = system.diagnostics(1.0).unwrap();
        for k in 0..3 {
            near(
                actual.angular_momentum[k],
                initial.angular_momentum[k],
                1e-10,
            );
        }
        near(actual.mechanical_energy, initial.mechanical_energy, 3e-6);
    }
    assert!((system.secondary.spin.angular_momentum[2] - 0.03).abs() > 1e-4);
}
#[test]
fn dissipative_tides_exchange_spin_orbit_and_deposit_positive_heat() {
    let mut system = binary();
    system.secondary.tide = Tide {
        love_number: 0.5,
        time_lag: 0.01,
    };
    system.secondary.spin.angular_momentum = [0.0, 0.0, 0.4];
    let initial = system.diagnostics(1.0).unwrap();
    let initial_spin = system.secondary.spin.angular_momentum[2];
    let mut previous_heat = 0.0;
    for _ in 0..10_000 {
        system.step(1.0, 0.001, 0.001, 1).unwrap();
        let actual = system.diagnostics(1.0).unwrap();
        assert!(actual.heat >= previous_heat);
        previous_heat = actual.heat;
        for k in 0..3 {
            near(
                actual.angular_momentum[k],
                initial.angular_momentum[k],
                1e-10,
            );
        }
        near(
            actual.mechanical_energy + actual.heat,
            initial.mechanical_energy + initial.heat,
            3e-6,
        );
    }
    assert!(system.secondary.heat > 0.001);
    assert!(system.secondary.spin.angular_momentum[2] < initial_spin);
}
#[test]
fn synchronous_circular_pair_has_no_initial_tidal_heating() {
    let mut system = binary();
    let orbital_speed = system.relative.velocity[1] / 2.0;
    for b in [&mut system.primary, &mut system.secondary] {
        b.spin.angular_momentum[2] = b.spin.inertia[2] * orbital_speed;
        b.tide = Tide {
            love_number: 0.5,
            time_lag: 0.01,
        };
    }
    system.step(1.0, 1e-6, 1e-6, 1).unwrap();
    assert!(system.primary.heat + system.secondary.heat < 1e-20);
}
#[test]
fn contact_and_budget_failures_are_atomic_even_after_substeps() {
    let mut system = binary();
    let before = system;
    assert_eq!(
        system.step(1.0, 0.01, 0.001, 1),
        Err(StepError::BudgetExceeded)
    );
    assert_eq!(system, before);
    system.relative.velocity = [-100.0, 0.0, 0.0];
    let before = system;
    assert_eq!(system.step(1.0, 0.04, 0.04, 1), Err(StepError::Contact));
    assert_eq!(system, before);
    system.secondary.heat = -1.0;
    let before = system;
    assert!(system.step(1.0, 0.01, 0.01, 1).is_err());
    assert_eq!(system, before);
}
