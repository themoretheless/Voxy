use physics::gravity::{Body, Error, Gravity};

fn body(mass: f64, x: f64) -> Body {
    Body {
        mass,
        position: [x, 0.0, 0.0],
        velocity: [0.0; 3],
    }
}

fn gravity() -> Gravity {
    Gravity {
        constant: 1.0,
        ..Gravity::default()
    }
}

#[test]
fn diagnostics_match_softened_potential_and_external_field() {
    let mut bodies = [body(2.0, -1.0), body(3.0, 1.0)];
    bodies[0].velocity = [0.0, 2.0, 0.0];
    let model = Gravity {
        softening: 1.0,
        uniform_acceleration: [1.0, 0.0, 0.0],
        ..gravity()
    };
    let d = model.diagnostics(&bodies).unwrap();
    assert!((d.total_mass - 5.0).abs() < 1e-12);
    assert!((d.kinetic_energy - 4.0).abs() < 1e-12);
    assert!((d.potential_energy + 1.0 + 6.0 / 5.0_f64.sqrt()).abs() < 1e-12);
    for (actual, expected) in d.linear_momentum.into_iter().zip([0.0, 4.0, 0.0]) {
        assert!((actual - expected).abs() < 1e-12);
    }
    for (actual, expected) in d.angular_momentum.into_iter().zip([0.0, 0.0, -4.0]) {
        assert!((actual - expected).abs() < 1e-12);
    }
}

#[test]
fn inverse_square_and_equal_opposite_force() {
    let bodies = [body(2.0, 0.0), body(3.0, 2.0)];
    let a = gravity().accelerations(&bodies).unwrap();
    assert_eq!(a, [[0.75, 0.0, 0.0], [-0.5, 0.0, 0.0]]);
    assert!((bodies[0].mass * a[0][0] + bodies[1].mass * a[1][0]).abs() < 1e-15);
    let distant = gravity()
        .accelerations(&[body(2.0, 0.0), body(3.0, 4.0)])
        .unwrap();
    assert!((distant[0][0] - a[0][0] / 4.0).abs() < 1e-15);
}

#[test]
fn uniform_field_matches_analytic_free_fall() {
    let field = Gravity {
        constant: 0.0,
        uniform_acceleration: [0.0, -9.81, 0.0],
        ..Gravity::default()
    };
    let mut bodies = [body(1.0, 0.0), body(100.0, 0.0)];
    for _ in 0..100 {
        field.step(&mut bodies, 0.01).unwrap();
    }
    for b in bodies {
        assert!((b.position[1] + 4.905).abs() < 1e-12);
        assert!((b.velocity[1] + 9.81).abs() < 1e-12);
    }
}

#[test]
fn circular_binary_orbit_preserves_radius_energy_and_momentum() {
    let mut bodies = [body(1.0, -0.5), body(1.0, 0.5)];
    let speed = (0.5_f64).sqrt();
    bodies[0].velocity[1] = -speed;
    bodies[1].velocity[1] = speed;
    for _ in 0..100_000 {
        gravity().step(&mut bodies, 0.001).unwrap();
        let distance = bodies[0]
            .position
            .iter()
            .zip(bodies[1].position)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        let kinetic = bodies
            .iter()
            .flat_map(|b| b.velocity)
            .map(|v| 0.5 * v * v)
            .sum::<f64>();
        assert!((distance - 1.0).abs() < 2e-6);
        assert!((kinetic - 1.0 / distance + 0.5).abs() < 1e-10);
        for k in 0..3 {
            assert!((bodies[0].velocity[k] + bodies[1].velocity[k]).abs() < 1e-12);
            assert!((bodies[0].position[k] + bodies[1].position[k]).abs() < 1e-12);
        }
    }
}

#[test]
fn singular_invalid_and_overflow_steps_are_atomic() {
    let mut bodies = [body(1.0, 0.0), body(1.0, 0.0)];
    let initial = bodies;
    assert_eq!(gravity().step(&mut bodies, 0.1), Err(Error::SingularPair));
    assert_eq!(bodies, initial);
    assert_eq!(
        gravity().step(&mut bodies, f64::NAN),
        Err(Error::InvalidInput)
    );
    assert_eq!(bodies, initial);
    let softened = Gravity {
        softening: 0.1,
        ..gravity()
    };
    softened.step(&mut bodies, 0.1).unwrap();
    assert_eq!(bodies, initial);
    bodies[0].velocity[0] = f64::MAX;
    let initial = bodies;
    assert_eq!(
        softened.step(&mut bodies, 10.0),
        Err(Error::NumericalOverflow)
    );
    assert_eq!(bodies, initial);
}
