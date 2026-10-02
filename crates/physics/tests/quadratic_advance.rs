use physics::{
    biomechanics::Material as Elastic,
    plasticity::{
        Material,
        mesh::{
            FiniteQuadraticDynamics, QuadraticAdvanceLimits, QuadraticBody, QuadraticDynamics,
            QuadraticPlaneContact,
        },
    },
};
fn mesh(yield_pa: f64) -> QuadraticBody {
    QuadraticBody::from_linear(
        vec![
            [0., 0.01, 0.],
            [0.1, 0.01, 0.],
            [0., 0.11, 0.],
            [0., 0.01, 0.1],
        ],
        vec![(
            [0, 1, 2, 3],
            Material::new(1e5, 0.3, yield_pa, 1000.).unwrap(),
        )],
    )
    .unwrap()
}
fn impact() -> FiniteQuadraticDynamics {
    let mut body = FiniteQuadraticDynamics::new(
        mesh(1e9),
        vec![Elastic::from_young_poisson(1e5, 0.3).unwrap()],
        &[1000.],
        vec![[0., -1., 0.]; 10],
        &[false; 10],
    )
    .unwrap();
    body.set_plane_contact(Some(
        QuadraticPlaneContact::new([0., 1., 0.], 0., 1e6).unwrap(),
    ))
    .unwrap();
    body
}
#[test]
fn adaptive_impact_rejects_large_step_and_matches_refined_reference() {
    let mut adaptive = impact();
    let original = adaptive.clone();
    assert!(adaptive.step(0.02, [0.; 3], 1e-7).is_err());
    assert_eq!(adaptive.positions(), original.positions());
    let report = adaptive
        .advance_loaded(
            0.02,
            &[[0.; 3]; 10],
            [0.; 3],
            QuadraticAdvanceLimits {
                minimum_dt_s: 1e-8,
                maximum_dt_s: 0.02,
                max_attempts: 10000,
                energy_tolerance_j: 1e-7,
            },
        )
        .unwrap();
    assert!(report.attempts > report.substeps.len());
    assert!((report.substeps.iter().map(|step| step.dt_s).sum::<f64>() - 0.02).abs() < 1e-14);
    let absolute: f64 = report
        .substeps
        .iter()
        .map(|step| step.energy_defect_j.abs())
        .sum();
    assert_eq!(absolute, report.absolute_energy_defect_j);
    assert!(absolute <= 1e-7);
    for step in &report.substeps {
        assert!(step.energy_defect_j.abs() <= 1e-7 * (step.dt_s / 0.02));
    }
    let mut reference = original;
    for _ in 0..2000 {
        reference.step(1e-5, [0.; 3], 1e-5).unwrap();
    }
    let mut position_error = 0_f64;
    let mut velocity_error = 0_f64;
    for (a, b) in adaptive.positions().iter().zip(reference.positions()) {
        for axis in 0..3 {
            position_error = position_error.max((a[axis] - b[axis]).abs());
        }
    }
    for (a, b) in adaptive.velocities().iter().zip(reference.velocities()) {
        for axis in 0..3 {
            velocity_error = velocity_error.max((a[axis] - b[axis]).abs());
        }
    }
    println!(
        "adaptive impact: attempts={}, accepted={}, absolute energy defect={absolute:e}, position error={position_error:e}, velocity error={velocity_error:e}",
        report.attempts,
        report.substeps.len()
    );
    assert!(position_error < 1e-5 && velocity_error < 0.002);
}
#[test]
fn failed_interval_rolls_back_accepted_plastic_substeps() {
    let mesh = mesh(100.);
    let velocities = mesh
        .positions()
        .iter()
        .map(|p| [10. * p[0], 0., 0.])
        .collect();
    let mut body = QuadraticDynamics::new(mesh, &[1000.], velocities).unwrap();
    let positions = body.body().positions().to_vec();
    let velocities = body.velocities().to_vec();
    let states = body.body().states();
    let mut proof = body.clone();
    proof.step(0.001, [0.; 3], 10.).unwrap();
    assert!(proof.energy().unwrap().dissipated_j > 0.);
    let error = body
        .advance_loaded(
            0.002,
            &[[0.; 3]; 10],
            [0.; 3],
            QuadraticAdvanceLimits {
                minimum_dt_s: 1e-8,
                maximum_dt_s: 0.001,
                max_attempts: 1,
                energy_tolerance_j: 100.,
            },
        )
        .unwrap_err();
    assert_eq!(error, "quadratic adaptive attempt limit reached");
    assert_eq!(body.body().positions(), positions);
    assert_eq!(body.velocities(), velocities);
    assert_eq!(body.body().states(), states);
}
#[test]
fn maximum_step_caps_sampling_and_invalid_limits_preserve_state() {
    let mut body = QuadraticDynamics::new(mesh(1e9), &[1000.], vec![[0.; 3]; 10]).unwrap();
    let limits = QuadraticAdvanceLimits {
        minimum_dt_s: 1e-8,
        maximum_dt_s: 0.001,
        max_attempts: 100,
        energy_tolerance_j: 1e-8,
    };
    let report = body
        .advance_loaded(0.004, &[[0.; 3]; 10], [0., -9.81, 0.], limits)
        .unwrap();
    assert_eq!(report.substeps.len(), 4);
    assert!(report.substeps.iter().all(|s| s.dt_s <= 0.001));
    assert!((body.velocities()[0][1] + 9.81 * 0.004).abs() < 1e-10);
    let positions = body.body().positions().to_vec();
    for invalid in [
        QuadraticAdvanceLimits {
            max_attempts: 0,
            ..limits
        },
        QuadraticAdvanceLimits {
            minimum_dt_s: 0.002,
            ..limits
        },
        QuadraticAdvanceLimits {
            energy_tolerance_j: f64::NAN,
            ..limits
        },
    ] {
        assert!(
            body.advance_loaded(0.004, &[[0.; 3]; 10], [0.; 3], invalid)
                .is_err()
        );
        assert_eq!(body.body().positions(), positions);
    }
}

#[test]
fn inverted_large_trial_can_recover_by_subdivision() {
    let mesh = mesh(1e9);
    let velocities = mesh
        .positions()
        .iter()
        .map(|p| [-20. * p[0], 0., 0.])
        .collect();
    let mut body = FiniteQuadraticDynamics::new(
        mesh,
        vec![Elastic::from_young_poisson(1e5, 0.3).unwrap()],
        &[1000.],
        velocities,
        &[false; 10],
    )
    .unwrap();
    let error = body.step(0.1, [0.; 3], 1.).unwrap_err();
    assert_eq!(error, "inverted quadratic integration point");
    let before_positions = body.positions().to_vec();
    let before_velocities = body.velocities().to_vec();
    let rejected = body
        .advance_loaded(
            0.1,
            &[[0.; 3]; 10],
            [0.; 3],
            QuadraticAdvanceLimits {
                minimum_dt_s: 0.1,
                maximum_dt_s: 0.1,
                max_attempts: 10,
                energy_tolerance_j: 1e-5,
            },
        )
        .unwrap_err();
    assert_eq!(rejected, "quadratic adaptive minimum timestep reached");
    assert_eq!(body.positions(), before_positions);
    assert_eq!(body.velocities(), before_velocities);
    let report = body
        .advance_loaded(
            0.1,
            &[[0.; 3]; 10],
            [0.; 3],
            QuadraticAdvanceLimits {
                minimum_dt_s: 1e-7,
                maximum_dt_s: 0.1,
                max_attempts: 30000,
                energy_tolerance_j: 1e-5,
            },
        )
        .unwrap();
    assert!(report.attempts > report.substeps.len());
    assert!(report.absolute_energy_defect_j <= 1e-5);
    assert!((report.substeps.iter().map(|s| s.dt_s).sum::<f64>() - 0.1).abs() < 1e-12);
    body.stresses().unwrap();
}
