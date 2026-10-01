use physics::{
    Origin,
    astrophysics::gravity_gradient_torque,
    astrophysics_spin::Spin,
    gravity::Gravity,
    gravity_field::{GravityField, NewtonianField, Source},
};
fn field(sources: &[Source]) -> NewtonianField<'_> {
    NewtonianField {
        gravity: Gravity {
            constant: 1.0,
            ..Default::default()
        },
        sources,
    }
}
fn near(a: f64, b: f64, tolerance: f64) {
    assert!((a - b).abs() < tolerance, "{a} != {b}");
}
#[test]
fn gradient_matches_finite_difference_inside_outside_and_with_softening() {
    let sources = [Source {
        anchor: Origin::default(),
        position: [0.0; 3],
        mass: 8.0,
        radius: 2.0,
    }];
    for point in [[0.2, 0.4, 0.7], [3.0, 4.0, 2.0]] {
        for softening in [0.0, 0.5] {
            let mut field = field(&sources);
            field.gravity.softening = softening;
            let tensor = field.tidal_tensor(Origin::default(), point).unwrap();
            for axis in 0..3 {
                let mut plus = point;
                let mut minus = point;
                plus[axis] += 1e-5;
                minus[axis] -= 1e-5;
                let a = field.acceleration(Origin::default(), plus).unwrap();
                let b = field.acceleration(Origin::default(), minus).unwrap();
                for row in 0..3 {
                    near(tensor[row][axis], (a[row] - b[row]) / 2e-5, 1e-9);
                }
            }
        }
    }
}
#[test]
fn point_tides_stretch_radially_and_compress_transversely() {
    let sources = [Source {
        anchor: Origin::default(),
        position: [0.0; 3],
        mass: 8.0,
        radius: 0.0,
    }];
    let tensor = field(&sources)
        .tidal_tensor(Origin::default(), [2.0, 0.0, 0.0])
        .unwrap();
    near(tensor[0][0], 2.0, 1e-12);
    near(tensor[1][1], -1.0, 1e-12);
    near(tensor[2][2], -1.0, 1e-12);
    near(tensor[0][0] + tensor[1][1] + tensor[2][2], 0.0, 1e-12);
    let diagonal = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]];
    let tilted = [[0.5, 1.5, 0.0], [1.5, 0.5, 0.0], [0.0, 0.0, -1.0]];
    let torque = gravity_gradient_torque(tilted, diagonal).unwrap();
    near(torque[2], 1.5, 1e-12);
    let spherical = [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 2.0]];
    for value in gravity_gradient_torque(tilted, spherical).unwrap() {
        near(value, 0.0, 1e-12);
    }
}
#[test]
fn spherical_spin_matches_analytic_rotation_and_torque() {
    let mut spin = Spin {
        orientation: [0.0, 0.0, 0.0, 1.0],
        angular_momentum: [0.0, 0.0, 2.0],
        inertia: [2.0; 3],
    };
    spin.step([0.0; 3], 1.0).unwrap();
    near(spin.orientation[2], 0.5_f64.sin(), 1e-12);
    near(spin.orientation[3], 0.5_f64.cos(), 1e-12);
    near(spin.energy().unwrap(), 1.0, 1e-12);
    spin.step([0.0, 0.0, 2.0], 0.1).unwrap();
    near(spin.angular_momentum[2], 2.2, 1e-12);
    let before = spin;
    assert!(spin.step([f64::NAN, 0.0, 0.0], 0.1).is_err());
    assert_eq!(spin, before);
}
#[test]
fn asymmetric_free_rotation_preserves_momentum_and_bounded_energy() {
    let mut spin = Spin {
        orientation: [0.0, 0.0, 0.0, 1.0],
        angular_momentum: [0.4, 0.7, 1.2],
        inertia: [1.0, 2.0, 3.0],
    };
    let energy = spin.energy().unwrap();
    let momentum = spin.angular_momentum;
    for _ in 0..10_000 {
        spin.step([0.0; 3], 0.001).unwrap();
        near(spin.energy().unwrap(), energy, 2e-6);
        for (actual, expected) in spin.angular_momentum.into_iter().zip(momentum) {
            near(actual, expected, 1e-12);
        }
        near(
            spin.orientation.iter().map(|v| v * v).sum::<f64>(),
            1.0,
            1e-12,
        );
    }
}

#[test]
fn gradient_torque_matches_forces_on_a_small_extended_body() {
    let sources = [Source {
        anchor: Origin::default(),
        position: [0.0; 3],
        mass: 8.0,
        radius: 0.0,
    }];
    let field = field(&sources);
    let center = [10.0, 0.0, 0.0];
    let separation = 0.001_f64;
    let diagonal = separation * separation;
    // Two unit masses on a 45-degree rod plus a tiny isotropic inertia.
    let inertia = [
        [diagonal + 1e-9, -diagonal, 0.0],
        [-diagonal, diagonal + 1e-9, 0.0],
        [0.0, 0.0, 2.0 * diagonal + 1e-9],
    ];
    let approximate = gravity_gradient_torque(
        field.tidal_tensor(Origin::default(), center).unwrap(),
        inertia,
    )
    .unwrap();
    let mut exact = 0.0;
    for sign in [-1.0, 1.0] {
        let offset = [
            sign * separation / std::f64::consts::SQRT_2,
            sign * separation / std::f64::consts::SQRT_2,
            0.0,
        ];
        let position = std::array::from_fn(|k| center[k] + offset[k]);
        let force = field.acceleration(Origin::default(), position).unwrap();
        exact += offset[0] * force[1] - offset[1] * force[0];
    }
    near(approximate[2], exact, 1e-14);
}
