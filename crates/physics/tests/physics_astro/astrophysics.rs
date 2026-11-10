use physics::astrophysics::{Conic, Error, OrbitalState, orbit, propagate};

fn conic(eccentricity: f64) -> Conic {
    Conic {
        periapsis: 1.0,
        eccentricity,
        inclination: 0.4,
        ascending_node: 0.7,
        argument_periapsis: 0.3,
        true_anomaly: 0.2,
    }
}
fn near(a: f64, b: f64, tolerance: f64) {
    assert!((a - b).abs() < tolerance, "{a} != {b}");
}

#[test]
fn circle_has_known_period_and_quarter_turn() {
    let state = OrbitalState {
        position: [1.0, 0.0, 0.0],
        velocity: [0.0, 1.0, 0.0],
    };
    let parameters = orbit(state, 1.0).unwrap();
    near(parameters.eccentricity, 0.0, 1e-12);
    near(parameters.period.unwrap(), std::f64::consts::TAU, 1e-12);
    let quarter = propagate(state, 1.0, std::f64::consts::FRAC_PI_2).unwrap();
    near(quarter.position[0], 0.0, 1e-11);
    near(quarter.position[1], 1.0, 1e-11);
    near(quarter.velocity[0], -1.0, 1e-11);
    near(quarter.velocity[1], 0.0, 1e-11);
    let repeated = propagate(state, 1.0, 100.0 * std::f64::consts::TAU).unwrap();
    for k in 0..3 {
        near(repeated.position[k], state.position[k], 1e-9);
        near(repeated.velocity[k], state.velocity[k], 1e-9);
    }
}

#[test]
fn all_three_conics_preserve_invariants_and_reverse_time() {
    for eccentricity in [0.0, 0.7, 0.9999, 1.0, 1.5, 3.0] {
        let initial = conic(eccentricity).state(1.0).unwrap();
        let invariants = orbit(initial, 1.0).unwrap();
        near(invariants.eccentricity, eccentricity, 1e-12);
        near(invariants.periapsis, 1.0, 1e-12);
        for dt in [0.1, 1.0, 5.0, -3.0] {
            let next = propagate(initial, 1.0, dt).unwrap();
            let diagnostics = orbit(next, 1.0).unwrap();
            near(
                diagnostics.specific_energy,
                invariants.specific_energy,
                1e-10,
            );
            near(diagnostics.eccentricity, eccentricity, 1e-10);
            let reversed = propagate(next, 1.0, -dt).unwrap();
            for k in 0..3 {
                near(
                    diagnostics.angular_momentum[k],
                    invariants.angular_momentum[k],
                    1e-10,
                );
                near(reversed.position[k], initial.position[k], 1e-9);
                near(reversed.velocity[k], initial.velocity[k], 1e-9);
            }
        }
    }
}

#[test]
fn eccentric_ellipse_returns_to_periapsis() {
    let initial = Conic {
        true_anomaly: 0.0,
        ..conic(0.9)
    }
    .state(1.0)
    .unwrap();
    let parameters = orbit(initial, 1.0).unwrap();
    near(parameters.apoapsis.unwrap(), 19.0, 1e-10);
    let next = propagate(initial, 1.0, parameters.period.unwrap()).unwrap();
    for k in 0..3 {
        near(next.position[k], initial.position[k], 1e-8);
        near(next.velocity[k], initial.velocity[k], 1e-8);
    }
}

#[test]
fn hyperbolic_branch_and_invalid_states_are_rejected() {
    assert_eq!(
        Conic {
            true_anomaly: std::f64::consts::PI,
            ..conic(2.0)
        }
        .state(1.0),
        Err(Error::InvalidInput)
    );
    let state = OrbitalState {
        position: [1.0, 0.0, 0.0],
        velocity: [0.0, 1.0, 0.0],
    };
    assert_eq!(propagate(state, 0.0, 1.0), Err(Error::InvalidInput));
    assert_eq!(propagate(state, 1.0, f64::NAN), Err(Error::InvalidInput));
    assert_eq!(
        propagate(
            OrbitalState {
                velocity: [0.0; 3],
                ..state
            },
            1.0,
            1.0
        ),
        Err(Error::DegenerateOrbit)
    );
    assert_eq!(propagate(state, 1.0, 0.0).unwrap(), state);
}
