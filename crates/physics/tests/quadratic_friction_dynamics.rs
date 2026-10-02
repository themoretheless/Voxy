use physics::{
    biomechanics::Material as Elastic,
    plasticity::{
        Material,
        mesh::{
            FiniteQuadraticDynamics, QuadraticAdvanceLimits, QuadraticBody, QuadraticPlaneContact,
            QuadraticPlaneFriction,
        },
    },
};
fn body() -> FiniteQuadraticDynamics {
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
    let mut body = FiniteQuadraticDynamics::new(
        mesh,
        vec![Elastic::from_young_poisson(1e5, 0.3).unwrap()],
        &[1000.],
        vec![[1., 0., 0.]; 10],
        &[false; 10],
    )
    .unwrap();
    assert!(
        body.set_plane_friction(Some(QuadraticPlaneFriction::new(0.4, 0.01).unwrap()))
            .is_err()
    );
    body.set_plane_contact(Some(
        QuadraticPlaneContact::new([0., 1., 0.], 0., 1e6).unwrap(),
    ))
    .unwrap();
    body.set_plane_friction(Some(QuadraticPlaneFriction::new(0.4, 0.01).unwrap()))
        .unwrap();
    body
}
fn slide(dt: f64, steps: usize) -> (FiniteQuadraticDynamics, f64) {
    let mut body = body();
    let initial = body.energy().unwrap();
    let mut worst = 0_f64;
    let mut previous = 0.;
    for _ in 0..steps {
        body.step(dt, [0.; 3], 1e-6).unwrap();
        let e = body.energy().unwrap();
        assert!(e.friction_dissipated_j >= previous);
        previous = e.friction_dissipated_j;
        assert!(
            (e.momentum_kg_m_s[0] - initial.momentum_kg_m_s[0] - e.friction_impulse_n_s[0]).abs()
                < 1e-10
        );
        worst = worst.max(
            (e.kinetic_j + e.elastic_j + e.contact_j + e.friction_dissipated_j
                - initial.kinetic_j
                - initial.contact_j)
                .abs(),
        );
    }
    let e = body.energy().unwrap();
    assert!(e.friction_dissipated_j > 0. && e.momentum_kg_m_s[0] < initial.momentum_kg_m_s[0]);
    (body, worst)
}
#[test]
fn sliding_dissipates_balances_impulse_and_converges() {
    let (coarse, ce) = slide(1e-5, 200);
    let (fine, fe) = slide(5e-6, 400);
    let (reference, _) = slide(2.5e-6, 800);
    let error = |a: &FiniteQuadraticDynamics| {
        a.velocities()
            .iter()
            .zip(reference.velocities())
            .flat_map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a - b).abs()))
            .fold(0_f64, f64::max)
    };
    let coarse_error = error(&coarse);
    let fine_error = error(&fine);
    println!(
        "sliding: velocity errors {coarse_error:e}, {fine_error:e}; energy envelopes {ce:e}, {fe:e}; dissipation {}",
        fine.energy().unwrap().friction_dissipated_j
    );
    assert!(fine_error < coarse_error * 0.7 && fine_error < 0.001);
    assert!(fe < ce && ce < 1e-5);
}
#[test]
fn rejected_friction_kick_and_interval_preserve_all_diagnostics() {
    let mut body = body();
    let original = body.clone();
    assert!(body.step(0.0001, [0.; 3], 1e-30).is_err());
    assert_eq!(body.positions(), original.positions());
    assert_eq!(body.velocities(), original.velocities());
    assert_eq!(body.energy().unwrap().friction_dissipated_j, 0.);
    assert_eq!(body.energy().unwrap().friction_impulse_n_s, [0.; 3]);
    body.set_plane_friction(Some(QuadraticPlaneFriction::new(1e8, 0.01).unwrap()))
        .unwrap();
    let positions = body.positions().to_vec();
    let velocities = body.velocities().to_vec();
    let initial = body.energy().unwrap();
    assert_eq!(
        body.step(0.001, [0.; 3], 1.).unwrap_err(),
        "quadratic friction kick energy increase"
    );
    assert_eq!(body.positions(), positions);
    assert_eq!(body.velocities(), velocities);
    assert_eq!(
        body.energy().unwrap().friction_dissipated_j,
        initial.friction_dissipated_j
    );
    assert_eq!(
        body.energy().unwrap().friction_impulse_n_s,
        initial.friction_impulse_n_s
    );
    assert_eq!(
        body.advance_loaded(
            0.001,
            &[[0.; 3]; 10],
            [0.; 3],
            QuadraticAdvanceLimits {
                minimum_dt_s: 0.001,
                maximum_dt_s: 0.001,
                max_attempts: 10,
                energy_tolerance_j: 1.
            }
        )
        .unwrap_err(),
        "quadratic adaptive minimum timestep reached"
    );
    assert_eq!(body.positions(), positions);
    assert_eq!(body.velocities(), velocities);
    body.set_plane_friction(None).unwrap();
    body.step(1e-5, [0.; 3], 1.).unwrap();
    assert_eq!(
        body.energy().unwrap().friction_dissipated_j,
        initial.friction_dissipated_j
    );
}

#[test]
fn supported_friction_reactions_and_disabling_preserve_dissipation_history() {
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
    let mut velocities = vec![[1., 0., 0.]; 10];
    velocities[0] = [0.; 3];
    let mut pins = [false; 10];
    pins[0] = true;
    let mut body = FiniteQuadraticDynamics::new(
        mesh,
        vec![Elastic::from_young_poisson(1e5, 0.3).unwrap()],
        &[1000.],
        velocities,
        &pins,
    )
    .unwrap();
    body.set_plane_contact(Some(
        QuadraticPlaneContact::new([0., 1., 0.], 0., 1e6).unwrap(),
    ))
    .unwrap();
    body.set_plane_friction(Some(QuadraticPlaneFriction::new(0.4, 0.01).unwrap()))
        .unwrap();
    let reactions = body.support_reactions(&[[0.; 3]; 10], [0.; 3]).unwrap();
    assert!(reactions[0].iter().any(|f| f.abs() > 1e-6));
    assert!(reactions[1..].iter().flatten().all(|f| f.abs() < 1e-8));
    body.step(1e-5, [0.; 3], 1e-6).unwrap();
    assert_eq!(body.velocities()[0], [0.; 3]);
    let e = body.energy().unwrap();
    assert!(e.friction_dissipated_j > 0.);
    body.set_plane_friction(None).unwrap();
    body.step(1e-5, [0.; 3], 1e-6).unwrap();
    assert_eq!(
        body.energy().unwrap().friction_dissipated_j,
        e.friction_dissipated_j
    );
    assert_eq!(
        body.energy().unwrap().friction_impulse_n_s,
        e.friction_impulse_n_s
    );
}
