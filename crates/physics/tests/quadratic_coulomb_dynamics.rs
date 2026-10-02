use physics::{
    biomechanics::Material as Elastic,
    plasticity::{
        Material,
        mesh::{
            FiniteQuadraticDynamics, QuadraticBody, QuadraticPlaneContact, QuadraticPlaneCoulomb,
        },
    },
};
fn fixture(ratio: f64) -> (FiniteQuadraticDynamics, Vec<[f64; 3]>) {
    let mesh = QuadraticBody::from_linear(
        vec![
            [0., -0.001, 0.],
            [0.1, -0.001, 0.],
            [0., 0.099, 0.],
            [0., -0.001, 0.1],
        ],
        vec![([0, 1, 2, 3], Material::new(1e5, 0.3, 1e9, 0.).unwrap())],
    )
    .unwrap();
    let plane = QuadraticPlaneContact::new([0., 1., 0.], 0., 1e6).unwrap();
    let normal = mesh.plane_contact_at(mesh.positions(), plane).unwrap();
    // Balance the exact reference-area normal nodal forces. Tangential loading
    // is distributed by the same surface pressure, not equally per node.
    let loads = normal
        .gradient_n
        .iter()
        .map(|g| [-ratio * g[1], g[1], 0.])
        .collect();
    let mut body = FiniteQuadraticDynamics::new(
        mesh,
        vec![Elastic::from_young_poisson(1e5, 0.3).unwrap()],
        &[1000.],
        vec![[0.; 3]; 10],
        &[false; 10],
    )
    .unwrap();
    body.set_plane_contact(Some(plane)).unwrap();
    body.set_plane_coulomb(Some(
        QuadraticPlaneCoulomb::new(0.5, 1e-10, 5000, 4096).unwrap(),
    ))
    .unwrap();
    (body, loads)
}
#[test]
fn below_limit_load_sticks_before_drift_and_balances_wall_impulse() {
    let (mut body, loads) = fixture(0.25);
    let positions = body.positions().to_vec();
    let initial = body.energy().unwrap();
    let total_force: f64 = loads.iter().map(|f| f[0]).sum();
    for step in 1..=20 {
        body.step_loaded(0.001, &loads, [0.; 3], 1e-8)
            .unwrap_or_else(|error| panic!("step={step}, error={error}"));
        let e = body.energy().unwrap();
        assert!(
            (e.momentum_kg_m_s[0]
                - total_force * f64::from(step) * 0.001
                - e.friction_impulse_n_s[0])
                .abs()
                < 1e-9
        );
        assert!(body.velocities().iter().flatten().all(|v| v.abs() < 1e-8));
        let report = body.last_coulomb_impulse().unwrap();
        assert!(report.residual_m_s <= 1e-10);
    }
    let movement = body
        .positions()
        .iter()
        .zip(&positions)
        .flat_map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a - b).abs()))
        .fold(0_f64, f64::max);
    println!(
        "static hold maximum movement={movement:e}, numerical dissipation={:e}",
        body.energy().unwrap().friction_numerical_j
    );
    assert!(movement < 1e-9);
    assert!(body.energy().unwrap().friction_dissipated_j < 1e-12);
    assert!(body.energy().unwrap().friction_numerical_j > initial.friction_numerical_j);
    assert!(body.support_reactions(&loads, [0.; 3]).is_err());
}
#[test]
fn above_limit_load_slides_and_failed_solve_rolls_back() {
    let (mut body, loads) = fixture(0.75);
    let initial = body.energy().unwrap();
    let initial_positions = body.positions().to_vec();
    for _ in 0..20 {
        body.step_loaded(1e-5, &loads, [0.; 3], 1e-7).unwrap();
    }
    let e = body.energy().unwrap();
    assert!(e.momentum_kg_m_s[0] > initial.momentum_kg_m_s[0] && e.friction_dissipated_j > 0.);
    assert!(body.positions()[1][0] > 0.1);
    let work: f64 = loads
        .iter()
        .zip(body.positions().iter().zip(&initial_positions))
        .map(|(f, (p, q))| {
            f.iter()
                .zip(p.iter().zip(q))
                .map(|(f, (p, q))| f * (p - q))
                .sum::<f64>()
        })
        .sum();
    let defect = e.kinetic_j + e.elastic_j + e.contact_j + e.friction_dissipated_j
        - initial.kinetic_j
        - initial.contact_j
        - work;
    assert!(defect.abs() < 2e-6);
    let last_impulse = body
        .last_coulomb_impulse()
        .unwrap()
        .nodal_impulse_n_s
        .clone();
    let positions = body.positions().to_vec();
    let velocities = body.velocities().to_vec();
    body.set_plane_coulomb(Some(
        QuadraticPlaneCoulomb::new(0.5, 1e-15, 1, 4096).unwrap(),
    ))
    .unwrap();
    assert!(body.step_loaded(0.001, &loads, [0.; 3], 1e-8).is_err());
    assert_eq!(body.positions(), positions);
    assert_eq!(body.velocities(), velocities);
    assert_eq!(
        body.energy().unwrap().friction_dissipated_j,
        e.friction_dissipated_j
    );
    assert_eq!(
        body.energy().unwrap().friction_numerical_j,
        e.friction_numerical_j
    );
    assert_eq!(
        body.energy().unwrap().friction_impulse_n_s,
        e.friction_impulse_n_s
    );
    assert_eq!(
        body.last_coulomb_impulse().unwrap().nodal_impulse_n_s,
        last_impulse
    );
}

fn threshold_transition(dt: f64, steps: usize) -> FiniteQuadraticDynamics {
    let (mut body, below) = fixture(0.49);
    let original = body.positions().to_vec();
    for _ in 0..steps {
        body.step_loaded(dt, &below, [0.; 3], 1e-8).unwrap();
    }
    let held_displacement = body
        .positions()
        .iter()
        .zip(&original)
        .flat_map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a - b).abs()))
        .fold(0_f64, f64::max);
    assert!(held_displacement < 1e-9);
    assert!(body.energy().unwrap().friction_dissipated_j < 1e-12);
    let above: Vec<_> = below
        .iter()
        .map(|f| [f[0] * 0.51 / 0.49, f[1], f[2]])
        .collect();
    let initial = body.energy().unwrap();
    let applied_force: f64 = above.iter().map(|f| f[0]).sum();
    for _ in 0..steps {
        body.step_loaded(dt, &above, [0.; 3], 1e-8).unwrap();
    }
    let after = body.energy().unwrap();
    assert!(after.momentum_kg_m_s[0] > initial.momentum_kg_m_s[0] + 1e-6);
    assert!(after.friction_dissipated_j > initial.friction_dissipated_j);
    assert!(
        (after.momentum_kg_m_s[0]
            - initial.momentum_kg_m_s[0]
            - applied_force * dt * f64::from(u32::try_from(steps).unwrap())
            - (after.friction_impulse_n_s[0] - initial.friction_impulse_n_s[0]))
            .abs()
            < 1e-9
    );
    body
}
#[test]
fn threshold_transition_is_resolved_and_velocity_refines() {
    let coarse = threshold_transition(1e-5, 40);
    let fine = threshold_transition(5e-6, 80);
    let reference = threshold_transition(2.5e-6, 160);
    let error = |body: &FiniteQuadraticDynamics| {
        body.velocities()
            .iter()
            .zip(reference.velocities())
            .flat_map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a - b).abs()))
            .fold(0_f64, f64::max)
    };
    let coarse_error = error(&coarse);
    let fine_error = error(&fine);
    println!("Coulomb threshold velocity errors: {coarse_error:e}, {fine_error:e}");
    assert!(fine_error < coarse_error * 0.7 && fine_error < 1e-5);
}
