use physics::plasticity::{
    Material,
    mesh::{QuadraticBody, QuadraticDynamics},
};
fn body(yield_pa: f64) -> QuadraticBody {
    QuadraticBody::new(
        vec![
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0.5, 0., 0.],
            [0.5, 0.5, 0.],
            [0., 0.5, 0.],
            [0., 0., 0.5],
            [0.5, 0., 0.5],
            [0., 0.5, 0.5],
        ],
        vec![(
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            Material::new(1e6, 0.3, yield_pa, 1000.).unwrap(),
        )],
    )
    .unwrap()
}
#[test]
fn consistent_mass_free_fall_matches_ballistic_motion_and_work() {
    let b = body(1e9);
    let initial = b.positions().to_vec();
    let mut dynamic = QuadraticDynamics::new(b, &[1000.], vec![[0.; 3]; 10]).unwrap();
    for step in 1..=100 {
        dynamic.step(0.001, [0., -9.81, 0.], 1e-8).unwrap();
        let t = f64::from(step) * 0.001;
        for (i, p) in dynamic.body().positions().iter().enumerate() {
            assert!((p[1] - initial[i][1] + 0.5 * 9.81 * t * t).abs() < 1e-10);
            assert!((dynamic.velocities()[i][1] + 9.81 * t).abs() < 1e-9);
        }
    }
    let e = dynamic.energy().unwrap();
    assert!((e.mass_kg - 1000. / 6.).abs() < 1e-10);
    assert!((e.momentum_kg_m_s[1] + e.mass_kg * 0.981).abs() < 1e-8);
    assert!((e.kinetic_j - 0.5 * e.mass_kg * 0.981_f64.powi(2)).abs() < 1e-8);
    assert!(e.elastic_j < 1e-16);
}
fn elastic_run(dt: f64, steps: usize) -> f64 {
    let b = body(1e9);
    let velocities = b.positions().iter().map(|p| [0.1 * p[0], 0., 0.]).collect();
    let mut dynamic = QuadraticDynamics::new(b, &[1000.], velocities).unwrap();
    let initial = dynamic.energy().unwrap();
    let mut worst = 0_f64;
    for _ in 0..steps {
        dynamic.step(dt, [0.; 3], 1.).unwrap();
        let e = dynamic.energy().unwrap();
        worst =
            worst.max((e.kinetic_j + e.elastic_j - initial.kinetic_j).abs() / initial.kinetic_j);
        for axis in 0..3 {
            assert!((e.momentum_kg_m_s[axis] - initial.momentum_kg_m_s[axis]).abs() < 1e-8);
        }
    }
    worst
}
#[test]
fn elastic_energy_error_converges_with_consistent_inertia() {
    let coarse = elastic_run(0.0002, 500);
    let fine = elastic_run(0.0001, 1000);
    println!("quadratic dynamics energy envelopes: {coarse:e}, {fine:e}");
    assert!(fine < 1e-3 && fine < coarse / 2.);
}
#[test]
fn rejected_plastic_step_keeps_geometry_velocity_and_all_histories() {
    let b = body(1.);
    let velocities = b.positions().iter().map(|p| [0.1 * p[0], 0., 0.]).collect();
    let mut dynamic = QuadraticDynamics::new(b, &[1000.], velocities).unwrap();
    let positions = dynamic.body().positions().to_vec();
    let states = dynamic.body().states();
    let velocities = dynamic.velocities().to_vec();
    assert_eq!(
        dynamic.step(0.0001, [0.; 3], 1e-15).unwrap_err(),
        "quadratic dynamic energy defect"
    );
    assert_eq!(dynamic.body().positions(), positions);
    assert_eq!(dynamic.body().states(), states);
    assert_eq!(dynamic.velocities(), velocities);
}

fn plastic_run(dt: f64, steps: usize) -> f64 {
    let b = body(1.);
    let velocities = b.positions().iter().map(|p| [0.1 * p[0], 0., 0.]).collect();
    let mut dynamic = QuadraticDynamics::new(b, &[1000.], velocities).unwrap();
    let initial = dynamic.energy().unwrap();
    for _ in 0..steps {
        dynamic.step(dt, [0.; 3], 1e-5).unwrap();
    }
    let final_state = dynamic.energy().unwrap();
    assert!(final_state.dissipated_j > 0. && final_state.hardening_j > 0.);
    assert!(
        dynamic
            .body()
            .states()
            .iter()
            .flatten()
            .any(|s| s.equivalent_plastic_strain() > 0.)
    );
    (final_state.kinetic_j
        + final_state.elastic_j
        + final_state.hardening_j
        + final_state.dissipated_j
        - initial.kinetic_j)
        .abs()
        / initial.kinetic_j
}
#[test]
fn accepted_plastic_history_accumulates_and_energy_ledger_refines() {
    let coarse = plastic_run(2e-6, 500);
    let fine = plastic_run(1e-6, 1000);
    println!("quadratic plastic energy errors: {coarse:e}, {fine:e}");
    assert!(fine < 1e-3 && fine < coarse);
}

#[test]
fn fixed_supports_recover_consistent_mass_oscillator_frequency() {
    let b = body(1e9);
    let initial = b.positions().to_vec();
    let mut pinned = vec![true; 10];
    pinned[1] = false;
    let mut velocities = vec![[0.; 3]; 10];
    velocities[1][0] = 0.001;
    let mut dynamic = QuadraticDynamics::new_supported(b, &[1000.], velocities, &pinned).unwrap();
    let modulus: f64 = 1e6 * (1. - 0.3) / ((1. + 0.3) * (1. - 2. * 0.3));
    let stiffness: f64 = modulus / 10.;
    let mass = 1000. / 420.;
    let omega = (stiffness / mass).sqrt();
    let dt = 0.0001;
    let theta = 2. * (omega * dt / 2.).asin();
    for _ in 0..1000 {
        dynamic.step(dt, [0.; 3], 1e-8).unwrap();
    }
    let expected = 0.001 * dt * (1000. * theta).sin() / theta.sin();
    assert!((dynamic.body().positions()[1][0] - 1. - expected).abs() < 1e-10);
    assert!((dynamic.velocities()[1][0] - 0.001 * (1000. * theta).cos()).abs() < 1e-9);
    for (i, &p) in pinned.iter().enumerate() {
        if p {
            assert_eq!(dynamic.body().positions()[i], initial[i]);
            assert_eq!(dynamic.velocities()[i], [0.; 3]);
        }
    }
}
#[test]
fn constant_nodal_force_drives_supported_oscillator_and_invalid_loads_roll_back() {
    let b = body(1e9);
    let mut pinned = vec![true; 10];
    pinned[1] = false;
    let mut dynamic =
        QuadraticDynamics::new_supported(b, &[1000.], vec![[0.; 3]; 10], &pinned).unwrap();
    let mut loads = vec![[0.; 3]; 10];
    loads[1][0] = 1.;
    let stiffness: f64 = (1e6 * (1. - 0.3) / ((1. + 0.3) * (1. - 2. * 0.3))) / 10.;
    let omega = (stiffness / (1000. / 420.)).sqrt();
    let dt = 0.0001;
    let theta = 2. * (omega * dt / 2.).asin();
    for _ in 0..1000 {
        dynamic.step_loaded(dt, &loads, [0.; 3], 1e-8).unwrap();
    }
    let expected = (1. - (1000. * theta).cos()) / stiffness;
    assert!((dynamic.body().positions()[1][0] - 1. - expected).abs() < 1e-10);
    let positions = dynamic.body().positions().to_vec();
    let velocities = dynamic.velocities().to_vec();
    let history = dynamic.body().states();
    loads[0][0] = f64::NAN;
    assert!(dynamic.step_loaded(dt, &loads, [0.; 3], 1e-8).is_err());
    assert_eq!(dynamic.body().positions(), positions);
    assert_eq!(dynamic.velocities(), velocities);
    assert_eq!(dynamic.body().states(), history);
}

#[test]
fn constraint_impulse_includes_consistent_mass_coupling_and_balances_momentum() {
    let b = body(1e9);
    let mut pinned = vec![true; 10];
    pinned[1] = false;
    let mut dynamic =
        QuadraticDynamics::new_supported(b, &[1000.], vec![[0.; 3]; 10], &pinned).unwrap();
    let mut loads = vec![[0.; 3]; 10];
    loads[1][0] = 1.;
    let acceleration = [0., -9.81, 0.];
    let dt = 0.0001;
    for _ in 0..500 {
        let before = dynamic.energy().unwrap();
        let old = dynamic.support_reactions(&loads, acceleration).unwrap();
        assert!(old[1].iter().all(|r| r.abs() < 1e-8));
        dynamic.step_loaded(dt, &loads, acceleration, 1e-7).unwrap();
        let after = dynamic.energy().unwrap();
        let new = dynamic.support_reactions(&loads, acceleration).unwrap();
        for axis in 0..3 {
            let reaction: f64 = old
                .iter()
                .zip(&new)
                .map(|(a, b)| 0.5 * (a[axis] + b[axis]))
                .sum();
            let external: f64 =
                loads.iter().map(|f| f[axis]).sum::<f64>() + before.mass_kg * acceleration[axis];
            let impulse = dt * (reaction + external);
            assert!(
                (after.momentum_kg_m_s[axis] - before.momentum_kg_m_s[axis] - impulse).abs() < 1e-9
            );
        }
    }
    assert!(
        dynamic
            .support_reactions(&loads[..9], acceleration)
            .is_err()
    );
}
