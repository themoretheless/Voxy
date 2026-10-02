use physics::plasticity::{
    Material,
    mesh::{Body, DynamicBody},
};
fn body(yield_pa: f64) -> Body {
    Body::new(
        vec![[0.; 3], [0.1, 0., 0.], [0., 0.1, 0.], [0., 0., 0.1]],
        vec![(
            [0, 1, 2, 3],
            Material::new(1e6, 0.3, yield_pa, 1000.).unwrap(),
        )],
    )
    .unwrap()
}
fn supports() -> [[Option<f64>; 3]; 4] {
    [
        [Some(0.); 3],
        [None, Some(0.), Some(0.)],
        [Some(0.); 3],
        [Some(0.); 3],
    ]
}
fn close(a: f64, b: f64, tolerance: f64) {
    assert!((a - b).abs() < tolerance, "{a} != {b}");
}
#[test]
fn lumped_mass_and_uniform_free_fall_match_analytical_motion_and_work() {
    let mut dynamic = DynamicBody::new(body(1e12), &[1000.], vec![[0.; 3]; 4]).unwrap();
    let initial = dynamic.body().positions().to_vec();
    let mass = 1000. * 0.001 / 6.;
    close(dynamic.diagnostics().unwrap().mass_kg, mass, 1e-14);
    let mut work = 0.;
    let dt = 0.001;
    for step in 1..=100 {
        let report = dynamic
            .step(
                dt,
                &[[0.; 3]; 4],
                [0., -9.81, 0.],
                &[[None; 3]; 4],
                20,
                1e-8,
                1e-9,
            )
            .unwrap();
        assert!(report.converged, "{report:?}");
        work += report.external_work_j;
        let time = f64::from(step) * dt;
        for (i, p) in dynamic.body().positions().iter().enumerate() {
            close(p[1], initial[i][1] - 0.5 * 9.81 * time * time, 1e-10);
            close(dynamic.velocities()[i][1], -9.81 * time, 1e-9);
        }
    }
    let diagnostics = dynamic.diagnostics().unwrap();
    close(diagnostics.momentum_kg_m_s[1], -mass * 9.81 * 0.1, 1e-10);
    close(work, diagnostics.kinetic_j + diagnostics.elastic_j, 1e-10);
}
fn oscillator(steps_per_period: u32, steps: u32) -> (f64, f64, f64) {
    let mut velocity = vec![[0.; 3]; 4];
    velocity[1][0] = 0.1;
    let mut dynamic = DynamicBody::new(body(1e12), &[1000.], velocity).unwrap();
    let constrained = 1e6 * (1. - 0.3) / ((1. + 0.3) * (1. - 2. * 0.3));
    let stiffness = constrained * (0.1 * 0.1 / 6.) / 0.1;
    let omega = (stiffness / dynamic.masses()[1]).sqrt();
    let period = 2. * std::f64::consts::PI / omega;
    let dt = period / f64::from(steps_per_period);
    let initial = dynamic.diagnostics().unwrap().kinetic_j;
    for _ in 0..steps {
        let step = dynamic
            .step(dt, &[[0.; 3]; 4], [0.; 3], &supports(), 10, 1e-8, 1e-9)
            .unwrap();
        assert!(step.converged, "{step:?}");
        let energy = dynamic.diagnostics().unwrap();
        assert!(((energy.kinetic_j + energy.elastic_j) / initial - 1.).abs() < 1e-8);
    }
    let x = dynamic.body().positions()[1][0] - 0.1;
    let time = f64::from(steps) * dt;
    let numerical_omega = 2. / dt * (omega * dt / 2.).atan();
    close(x, 0.1 / omega * (numerical_omega * time).sin(), 1e-10);
    close(
        dynamic.velocities()[1][0],
        0.1 * (numerical_omega * time).cos(),
        1e-7,
    );
    (x, 0.1 / omega * (omega * time).sin(), time)
}
#[test]
fn elastic_oscillation_preserves_energy_without_numerical_damping() {
    oscillator(40, 1000);
}
#[test]
fn oscillator_phase_error_converges_at_second_order() {
    let (coarse, analytic, time) = oscillator(24, 27);
    let (fine, analytic_fine, time_fine) = oscillator(48, 54);
    close(time, time_fine, 1e-14);
    close(analytic, analytic_fine, 1e-14);
    let ratio = ((coarse - analytic) / (fine - analytic_fine)).abs();
    assert!((3.5..4.5).contains(&ratio), "{ratio}");
}
#[test]
fn failed_plastic_step_invalid_inputs_and_inversion_preserve_all_state() {
    let mut dynamic = DynamicBody::new(body(100.), &[1000.], vec![[0.; 3]; 4]).unwrap();
    let positions = dynamic.body().positions().to_vec();
    let velocity = dynamic.velocities().to_vec();
    let states = dynamic.body().states();
    let mut force = [[0.; 3]; 4];
    force[1][0] = 10.;
    let failed = dynamic
        .step(0.001, &force, [0.; 3], &supports(), 1, 1e-8, 1e-9)
        .unwrap();
    assert!(!failed.converged);
    assert_eq!(dynamic.body().positions(), positions);
    assert_eq!(dynamic.velocities(), velocity);
    assert_eq!(dynamic.body().states(), states);
    assert!(
        dynamic
            .step(
                f64::MIN_POSITIVE,
                &force,
                [0.; 3],
                &supports(),
                10,
                1e-8,
                1e-9
            )
            .is_err()
    );
    assert_eq!(dynamic.body().positions(), positions);
    assert_eq!(dynamic.velocities(), velocity);
    let mut initial = vec![[0.; 3]; 4];
    initial[1][0] = -200.;
    let mut inverted = DynamicBody::new(body(1e12), &[1000.], initial.clone()).unwrap();
    assert!(
        inverted
            .step(0.001, &[[0.; 3]; 4], [0.; 3], &supports(), 10, 1e-8, 1e-9)
            .is_err()
    );
    assert_eq!(inverted.body().positions(), positions);
    assert_eq!(inverted.velocities(), initial);
}

#[path = "support/cohesive.rs"]
mod support;
fn fracture_run(dt: f64) -> (f64, f64) {
    let (body, _, _) = support::fixture(
        1,
        physics::cohesive::Material::new(1e9, 1e10, 1e5, 10.).unwrap(),
    );
    let mut velocities = vec![[0.; 3]; body.positions().len()];
    for (i, v) in velocities.iter_mut().enumerate() {
        v[0] = if i < 8 { -0.5 } else { 0.5 };
    }
    let mut dynamic = DynamicBody::new(body, &[1000.; 12], velocities).unwrap();
    let initial = dynamic.diagnostics().unwrap().kinetic_j;
    assert_eq!(dynamic.fragments().unwrap().len(), 1);
    let mut time = 0.;
    while time < 0.0016 - dt / 2. {
        let result = dynamic
            .step(
                dt,
                &[[0.; 3]; 16],
                [0.; 3],
                &[[None; 3]; 16],
                30,
                1e-5,
                1e-3,
            )
            .unwrap();
        assert!(result.converged, "dt={dt}, t={time}, {result:?}");
        time += dt;
    }
    let diagnostics = dynamic.diagnostics().unwrap();
    close(diagnostics.fracture_dissipated_j, 0.1, 1e-8);
    assert!(
        dynamic
            .body()
            .interface_reports()
            .unwrap()
            .iter()
            .all(|r| r.quadrature.iter().all(|q| q.damage > 1. - 1e-10))
    );
    for momentum in diagnostics.momentum_kg_m_s {
        close(momentum, 0., 1e-8);
    }
    let fragments = dynamic.fragments().unwrap();
    assert_eq!(fragments.len(), 2);
    for fragment in &fragments {
        close(fragment.mass_kg, 1., 1e-12);
        assert_eq!(fragment.nodes.len(), 8);
    }
    assert!(fragments[0].velocity_m_s[0] < 0. && fragments[1].velocity_m_s[0] > 0.);
    close(
        fragments[0].momentum_kg_m_s[0] + fragments[1].momentum_kg_m_s[0],
        0.,
        1e-8,
    );
    let energy = diagnostics.kinetic_j
        + diagnostics.elastic_j
        + diagnostics.interface_stored_j
        + diagnostics.fracture_dissipated_j;
    ((energy - initial).abs(), diagnostics.kinetic_j)
}
#[test]
fn dynamic_fracture_consumes_toughness_and_moving_fragments_preserve_momentum() {
    let (coarse, _) = fracture_run(2e-5);
    let (medium, _) = fracture_run(1e-5);
    let (fine, kinetic) = fracture_run(5e-6);
    println!(
        "dynamic fracture energy defects: {coarse:.6e}, {medium:.6e}, {fine:.6e} J; final kinetic {kinetic:.9} J"
    );
    assert!(fine < coarse && fine < medium, "{coarse}, {medium}, {fine}");
    assert!(fine < 1e-4);
    assert!(kinetic > 0.1);
}

#[test]
fn a_step_that_skips_the_cohesive_peak_is_rejected_by_energy_even_at_zero_force_residual() {
    let (body, _, _) = support::fixture(
        1,
        physics::cohesive::Material::new(1e9, 1e10, 1e5, 10.).unwrap(),
    );
    let positions = body.positions().to_vec();
    let interface_states = body.interface_states();
    let mut velocities = vec![[0.; 3]; 16];
    for (i, v) in velocities.iter_mut().enumerate() {
        v[0] = if i < 8 { -0.5 } else { 0.5 };
    }
    let mut dynamic = DynamicBody::new(body, &[1000.; 12], velocities.clone()).unwrap();
    let report = dynamic
        .step(
            0.0002,
            &[[0.; 3]; 16],
            [0.; 3],
            &[[None; 3]; 16],
            30,
            1e-5,
            1e-4,
        )
        .unwrap();
    assert!(!report.converged);
    assert!(report.residual_n < 1e-5);
    assert!(report.energy_defect_j.unwrap() > 0.09);
    assert_eq!(dynamic.body().positions(), positions);
    assert_eq!(dynamic.velocities(), velocities);
    assert_eq!(dynamic.body().interface_states(), interface_states);
    assert_eq!(dynamic.fragments().unwrap().len(), 1);
}

#[test]
fn adaptive_free_interval_refines_fracture_and_rolls_back_work_limit() {
    let (body, _, _) = support::fixture(
        1,
        physics::cohesive::Material::new(1e9, 1e10, 1e5, 10.).unwrap(),
    );
    let mut velocities = vec![[0.; 3]; 16];
    for (i, v) in velocities.iter_mut().enumerate() {
        v[0] = if i < 8 { -0.5 } else { 0.5 };
    }
    let mut dynamic = DynamicBody::new(body, &[1000.; 12], velocities).unwrap();
    let before = dynamic.clone();
    let limits = physics::plasticity::mesh::AdvanceLimits {
        minimum_dt_s: 1e-8,
        max_steps: 1,
        max_iterations: 30,
        force_tolerance_n: 1e-5,
        energy_tolerance_j: 1e-4,
    };
    assert!(
        dynamic
            .advance_free(0.0002, &[[0.; 3]; 16], [0.; 3], limits)
            .is_err()
    );
    assert_eq!(dynamic.body().positions(), before.body().positions());
    assert_eq!(dynamic.velocities(), before.velocities());
    assert_eq!(
        dynamic.body().interface_states(),
        before.body().interface_states()
    );
    let reports = dynamic
        .advance_free(
            0.0002,
            &[[0.; 3]; 16],
            [0.; 3],
            physics::plasticity::mesh::AdvanceLimits {
                max_steps: 4096,
                ..limits
            },
        )
        .unwrap();
    assert!(reports.len() > 1);
    let defect: f64 = reports
        .iter()
        .map(|r| r.energy_defect_j.unwrap().abs())
        .sum();
    assert!(defect <= limits.energy_tolerance_j);
    assert!(dynamic.body().positions()[0][0] < before.body().positions()[0][0]);
    for p in dynamic.diagnostics().unwrap().momentum_kg_m_s {
        close(p, 0., 1e-8);
    }
}
