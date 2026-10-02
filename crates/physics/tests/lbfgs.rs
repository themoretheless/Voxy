use physics::biomechanics::*;
fn body() -> Body {
    Body::new(
        vec![[0., 0., 0.], [0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]],
        vec![true, true, true, false],
        vec![(
            [0, 1, 2, 3],
            Material::from_young_poisson(8000., 0.3).unwrap(),
        )],
    )
    .unwrap()
}
#[test]
fn diagnostic_import_preserves_reference_and_is_atomic() {
    let mut b = body();
    b.set_force(3, [0.001, 0., 0.]).unwrap();
    let reference = b.rest_positions().to_vec();
    let mut x = reference.clone();
    x[3][0] += 0.0001;
    let expected = b.evaluate(&x).unwrap();
    b.restore_diagnostic_positions(&x).unwrap();
    assert_eq!(b.rest_positions(), reference);
    assert_eq!(b.evaluate(b.positions()).unwrap(), expected);
    let mut bad = x.clone();
    bad[0][0] += 0.0001;
    assert!(b.restore_diagnostic_positions(&bad).is_err());
    assert_eq!(b.positions(), x);
    assert!(b.restore_diagnostic_positions(&[]).is_err());
    bad = x.clone();
    bad[3][2] = f64::NAN;
    assert!(b.restore_diagnostic_positions(&bad).is_err());
    assert_eq!(b.positions(), x);
}
#[test]
fn lbfgs_matches_elastic_contact_equilibrium_and_recovers_without_rebasing() {
    let mut cg = body();
    cg.add_tissue_gaps(&[TissueGap {
        nodes: [0, 3],
        minimum_distance_m: 0.003,
        activation_gap_m: 0.0069,
        stiffness_n_m: 10.,
    }])
    .unwrap();
    cg.set_force(3, [0., 0., -0.03]).unwrap();
    let mut lbfgs = cg.clone();
    assert!(cg.equilibrate(100000, 1e-10).unwrap().converged);
    let report = lbfgs.equilibrate_lbfgs(100000, 1e-10).unwrap();
    assert!(report.converged && report.min_j > 0.);
    for (a, b) in cg.positions().iter().zip(lbfgs.positions()) {
        for k in 0..3 {
            assert!((a[k] - b[k]).abs() < 1e-8);
        }
    }
    let reference = lbfgs.rest_positions().to_vec();
    lbfgs.set_force(3, [0.; 3]).unwrap();
    assert!(lbfgs.equilibrate_lbfgs(100000, 1e-10).unwrap().converged);
    for (a, b) in reference.iter().zip(lbfgs.positions()) {
        for k in 0..3 {
            assert!((a[k] - b[k]).abs() < 1e-8);
        }
    }
    assert_eq!(lbfgs.tissue_gaps().len(), 1);
}
#[test]
fn invalid_controls_preserve_geometry_and_zero_force_rest_converges() {
    let mut b = body();
    let before = b.positions().to_vec();
    assert!(b.equilibrate_lbfgs(0, 1e-8).is_err());
    assert!(b.equilibrate_lbfgs(1, f64::NAN).is_err());
    assert_eq!(before, b.positions());
    let report = b.equilibrate_lbfgs(100, 1e-10).unwrap();
    assert!(report.converged && report.iterations == 0);
}

#[test]
fn bent_oval_wall_completes_contact_load_release_cycle() {
    let passive = Material {
        shear_pa: 500.,
        bulk_pa: 5000.,
        fibers: vec![],
    };
    let muscle = |direction| Material {
        fibers: vec![Fiber {
            direction,
            stiffness_pa: 50.,
            exponent: 2.,
            active_pa: 1000.,
        }],
        ..passive.clone()
    };
    let mut profiles = vec![
        [
            passive.clone(),
            muscle([0., 0., 1.]),
            muscle([1., 0., 0.]),
            passive.clone()
        ];
        3
    ];
    profiles[1][3] = muscle([1., 0., 0.]);
    let mut body = UrogenitalWallGeometry {
        radii_m: [0.0015, 0.0018, 0.0022, 0.0026, 0.003],
        length_m: 0.02,
        sectors: 8,
        segments: 3,
    }
    .axial_wall([1., 0.6], &profiles)
    .unwrap();
    let reference = body.positions().to_vec();
    let pairs: Vec<_> = (1..=3).map(|row| [row * 40 + 2, row * 40 + 6]).collect();
    let gaps: Vec<_> = pairs
        .iter()
        .map(|&nodes| TissueGap {
            nodes,
            minimum_distance_m: 0.00018,
            activation_gap_m: 0.001539,
            stiffness_n_m: 10.,
        })
        .collect();
    body.add_tissue_gaps(&gaps).unwrap();
    for factor in [1., 0.75, 0.5, 0.25, 0.] {
        for &[a, b] in &pairs {
            body.set_force(a, [0., -0.002 * factor, 0.]).unwrap();
            body.set_force(b, [0., 0.002 * factor, 0.]).unwrap();
        }
        let report = body.equilibrate_lbfgs(100000, 1e-7).unwrap();
        assert!(
            report.converged && report.min_j > 0.,
            "factor={factor} residual={}",
            report.residual_n
        );
        for &[a, b] in &pairs {
            let distance = (0..3)
                .map(|k| (body.positions()[a][k] - body.positions()[b][k]).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(distance > 0.00018);
        }
        assert_eq!(&body.positions()[..40], &reference[..40]);
    }
    assert_eq!(body.tissue_gaps().len(), 3);
    for (a, b) in reference.iter().zip(body.positions()) {
        assert!((0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt() < 5e-6);
    }
}

#[test]
fn observer_reports_accepted_iterates_without_changing_equilibrium() {
    let mut plain = body();
    plain.set_force(3, [0.001, 0., -0.01]).unwrap();
    let mut observed = plain.clone();
    let mut stepped = plain.clone();
    let expected = plain.equilibrate_lbfgs(10000, 1e-10).unwrap();
    let mut samples = Vec::new();
    let actual = observed
        .equilibrate_lbfgs_observed(10000, 1e-10, |i, e, r| {
            samples.push((i, e, r));
        })
        .unwrap();
    assert!(actual.converged);
    assert_eq!(observed.positions(), plain.positions());
    assert_eq!(actual.iterations, expected.iterations);
    assert_eq!(actual.residual_n, expected.residual_n);
    assert_eq!(samples.len(), actual.iterations + 1);
    for (i, (iteration, energy, residual)) in samples.iter().enumerate() {
        assert_eq!(*iteration, i);
        assert!(energy.is_finite() && residual.is_finite());
    }
    assert_eq!(samples.last().unwrap().2, actual.residual_n);
    let mut invalid_observations = 0;
    assert!(
        observed
            .equilibrate_lbfgs_observed(0, 1e-10, |_, _, _| {
                invalid_observations += 1;
            })
            .is_err()
    );
    assert_eq!(invalid_observations, 0);
    let mut steps = Vec::new();
    let report = stepped
        .equilibrate_lbfgs_steps(10000, 1e-10, |i, e, r, s, b, d| {
            steps.push((i, e, r, s, b, d));
        })
        .unwrap();
    assert_eq!(stepped.positions(), plain.positions());
    assert_eq!(report.residual_n, expected.residual_n);
    assert_eq!(steps.len(), samples.len());
    for (step, sample) in steps.iter().zip(samples) {
        assert_eq!((step.0, step.1, step.2), sample);
        assert!(step.5.is_finite() && step.5 >= 0.);
        if step.0 == 0 {
            assert_eq!((step.3, step.4, step.5), (0., 0, 0.));
        } else {
            assert_eq!(step.3, 0.5_f64.powi(step.4 as i32));
        }
    }
}

#[test]
fn accepted_coordinate_observer_is_bit_exact_and_reports_actual_energy() {
    let mut plain = body();
    plain.set_force(3, [0.001, 0.0005, 0.]).unwrap();
    let oracle = plain.clone();
    let mut observed = plain.clone();
    let mut snapshots = Vec::new();
    let a = plain
        .equilibrate_lbfgs_steps(2000, 1e-10, |_, _, _, _, _, _| {})
        .unwrap();
    let b = observed
        .equilibrate_lbfgs_states(2000, 1e-10, |i, e, r, _, _, _, x| {
            assert_eq!(oracle.evaluate(x).unwrap().0, e);
            assert_eq!(&x[..3], &oracle.positions()[..3]);
            snapshots.push((i, r, x.to_vec()));
        })
        .unwrap();
    assert_eq!(a.iterations, b.iterations);
    assert_eq!(a.residual_n, b.residual_n);
    assert_eq!(plain.positions(), observed.positions());
    assert_eq!(snapshots.last().unwrap().2, observed.positions());
    assert_eq!(snapshots.len(), b.iterations + 1);
}

#[test]
fn inactive_contact_preconditioner_is_bit_exact() {
    let mut plain = body();
    plain.set_force(3, [0.001, 0.0005, 0.]).unwrap();
    plain
        .set_surface_contact_law(SurfaceContactLaw::ExperimentalPrimitiveSum)
        .unwrap();
    let mut conditioned = plain.clone();
    let a = plain.equilibrate_lbfgs(2000, 1e-10).unwrap();
    let b = conditioned
        .equilibrate_lbfgs_preconditioned_states(2000, 1e-10, true, |_, _, _, _, _, _, _| {})
        .unwrap();
    assert_eq!(a.iterations, b.iterations);
    assert_eq!(a.residual_n, b.residual_n);
    assert_eq!(plain.positions(), conditioned.positions());
}

#[test]
fn equilibrium_rejects_iteration_exhaustion_without_committing() {
    let mut b = body();
    b.set_force(3, [0.01, 0.005, -0.02]).unwrap();
    let before = b.positions().to_vec();
    let reference = b.rest_positions().to_vec();
    let initial = b.evaluate(&before).unwrap();
    let mut diagnostic = b.clone();
    let report = diagnostic
        .equilibrate_lbfgs_observed(1, 1e-12, |_, _, _| {})
        .unwrap();
    assert!(!report.converged && report.residual_n > 1e-12);
    assert_ne!(diagnostic.positions(), before);
    assert_eq!(
        b.equilibrate_lbfgs(1, 1e-12).unwrap_err(),
        "L-BFGS equilibrium did not converge"
    );
    assert_eq!(b.positions(), before);
    assert_eq!(b.rest_positions(), reference);
    assert_eq!(b.evaluate(b.positions()).unwrap(), initial);
    let accepted = b.equilibrate_lbfgs(100000, 1e-10).unwrap();
    assert!(accepted.converged && accepted.residual_n <= 1e-10);
    assert_eq!(b.positions(), diagnostic_equilibrium_positions());
}

fn diagnostic_equilibrium_positions() -> Vec<[f64; 3]> {
    let mut b = body();
    b.set_force(3, [0.01, 0.005, -0.02]).unwrap();
    assert!(
        b.equilibrate_lbfgs_observed(100000, 1e-10, |_, _, _| {})
            .unwrap()
            .converged
    );
    b.positions().to_vec()
}
