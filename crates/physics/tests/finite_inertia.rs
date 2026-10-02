use physics::biomechanics::{Body, InertialBody, Material};
fn spinning() -> InertialBody {
    let positions = vec![[0.; 3], [0.1, 0., 0.], [0., 0.1, 0.], [0., 0., 0.1]];
    let velocities = positions
        .iter()
        .map(|p| [-10. * (p[1] - 0.025), 10. * (p[0] - 0.025), 0.])
        .collect();
    let body = Body::new(
        positions,
        vec![false; 4],
        vec![(
            [0, 1, 2, 3],
            Material::from_young_poisson(1e5, 0.3).unwrap(),
        )],
    )
    .unwrap();
    InertialBody::new(body, &[1000.], velocities).unwrap()
}
fn run(dt: f64, steps: u32) -> (InertialBody, f64) {
    let mut body = spinning();
    let before = body.diagnostics().unwrap();
    let initial = before.kinetic_j + before.potential_j;
    let mut worst = 0_f64;
    for _ in 0..steps {
        body.step(dt, 1.).unwrap();
        let now = body.diagnostics().unwrap();
        worst = worst.max((now.kinetic_j + now.potential_j - initial).abs() / initial);
        for axis in 0..3 {
            assert!((now.momentum_kg_m_s[axis] - before.momentum_kg_m_s[axis]).abs() < 1e-10);
            assert!(
                (now.angular_momentum_kg_m2_s[axis] - before.angular_momentum_kg_m2_s[axis]).abs()
                    < 1e-10
            );
        }
    }
    (body, worst)
}
#[test]
fn finite_rotation_preserves_angular_momentum_and_energy_error_converges() {
    let (_, coarse) = run(0.0004, 250);
    let (body, fine) = run(0.0002, 500);
    println!("relative energy envelopes: coarse={coarse:e}, fine={fine:e}");
    assert!(fine < 0.01);
    assert!(coarse / fine > 3. && coarse / fine < 5.);
    let p = body.body().positions()[1];
    let initial = [0.075, -0.025];
    let current = [p[0] - 0.025, p[1] - 0.025];
    let angle = (initial[0] * current[1] - initial[1] * current[0])
        .atan2(initial[0] * current[0] + initial[1] * current[1]);
    assert!(angle > 0.5, "rotation={angle}");
}
#[test]
fn verlet_is_time_reversible_and_invalid_or_unstable_steps_are_atomic() {
    let original = spinning();
    let (body, _) = run(0.0001, 200);
    let velocities = body.velocities().iter().map(|v| v.map(|x| -x)).collect();
    let mut reversed = InertialBody::new(body.body().clone(), &[1000.], velocities).unwrap();
    for _ in 0..200 {
        reversed.step(0.0001, 1.).unwrap();
    }
    for (a, b) in reversed
        .body()
        .positions()
        .iter()
        .flatten()
        .zip(original.body().positions().iter().flatten())
    {
        assert!((a - b).abs() < 1e-10);
    }
    for (a, b) in reversed
        .velocities()
        .iter()
        .flatten()
        .zip(original.velocities().iter().flatten())
    {
        assert!((a + b).abs() < 1e-9);
    }
    let before_positions = reversed.body().positions().to_vec();
    let before_velocities = reversed.velocities().to_vec();
    assert!(reversed.step(f64::NAN, 1.).is_err());
    assert!(reversed.step(0.01, 1e-12).is_err());
    assert_eq!(reversed.body().positions(), before_positions);
    assert_eq!(reversed.velocities(), before_velocities);
}

#[test]
fn dynamic_trajectory_is_covariant_under_rigid_coordinate_rotation_and_translation() {
    let mut original = spinning();
    let rotation = [[0.36, -0.8, 0.48], [0.48, 0.6, 0.64], [-0.8, 0., 0.6]];
    let rotate = |v: [f64; 3]| -> [f64; 3] {
        std::array::from_fn(|i| (0..3).map(|j| rotation[i][j] * v[j]).sum())
    };
    let offset = [0.2, -0.3, 0.1];
    let transformed_positions = original
        .body()
        .positions()
        .iter()
        .map(|&p| {
            let r = rotate(p);
            std::array::from_fn(|i| r[i] + offset[i])
        })
        .collect();
    let transformed_velocities = original.velocities().iter().map(|&v| rotate(v)).collect();
    let body = Body::new(
        transformed_positions,
        vec![false; 4],
        vec![(
            [0, 1, 2, 3],
            Material::from_young_poisson(1e5, 0.3).unwrap(),
        )],
    )
    .unwrap();
    let mut transformed = InertialBody::new(body, &[1000.], transformed_velocities).unwrap();
    for _ in 0..500 {
        original.step(0.0002, 1.).unwrap();
        transformed.step(0.0002, 1.).unwrap();
    }
    for (a, b) in original
        .body()
        .positions()
        .iter()
        .zip(transformed.body().positions())
    {
        let expected = rotate(*a);
        for axis in 0..3 {
            assert!((expected[axis] + offset[axis] - b[axis]).abs() < 1e-10);
        }
    }
    for (a, b) in original.velocities().iter().zip(transformed.velocities()) {
        let expected = rotate(*a);
        for axis in 0..3 {
            assert!((expected[axis] - b[axis]).abs() < 1e-9);
        }
    }
}

#[test]
fn uniform_acceleration_matches_ballistic_motion_and_potential_work() {
    let source = spinning();
    let mut body = InertialBody::new(source.body().clone(), &[1000.], vec![[0.; 3]; 4]).unwrap();
    assert_eq!(body.set_uniform_acceleration([0., -9.81, 0.]).unwrap(), 0.);
    let initial = body.body().positions().to_vec();
    for step in 1..=100 {
        body.step(0.001, 1e-10).unwrap();
        let time = f64::from(step) * 0.001;
        for (i, p) in body.body().positions().iter().enumerate() {
            assert!((p[1] - initial[i][1] + 0.5 * 9.81 * time * time).abs() < 1e-10);
            assert!((body.velocities()[i][1] + 9.81 * time).abs() < 1e-9);
        }
    }
    let energy = body.diagnostics().unwrap();
    assert!((energy.kinetic_j + energy.potential_j).abs() < 1e-10);
    let positions = body.body().positions().to_vec();
    assert!(body.set_uniform_acceleration([0., f64::NAN, 0.]).is_err());
    assert_eq!(body.body().positions(), positions);
    assert_eq!(body.diagnostics().unwrap().potential_j, energy.potential_j);
    let parameter_work = body.set_uniform_acceleration([0.; 3]).unwrap();
    assert!((parameter_work + energy.potential_j).abs() < 1e-10);
}
