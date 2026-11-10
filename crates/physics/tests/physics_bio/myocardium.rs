use physics::biomechanics::{ExponentialTerm, IDENTITY, Myocardium};
fn material() -> Myocardium {
    let term = |scale_pa, exponent| ExponentialTerm { scale_pa, exponent };
    Myocardium {
        matrix: term(700., 7.),
        fiber: term(18000., 12.),
        sheet: term(2500., 5.),
        fiber_sheet: term(150., 10.),
        bulk_pa: 1e6,
        fiber_direction: [1., 0., 0.],
        sheet_direction: [0., 1., 0.],
        active_tension_pa: 5000.,
    }
}
#[test]
fn orthotropic_energy_derivative_including_activation_and_shear() {
    let m = material();
    for f in [
        [[1.12, 0.08, 0.02], [0.03, 0.94, 0.], [0., 0.04, 0.98]],
        [[0.91, 0.12, 0.], [0., 1.07, 0.03], [0., 0., 1.02]],
    ] {
        for activation in [0., 0.7] {
            let r = m.response(f, activation).unwrap();
            for i in 0..3 {
                for k in 0..3 {
                    let mut plus = f;
                    let mut minus = f;
                    plus[i][k] += 1e-6;
                    minus[i][k] -= 1e-6;
                    let derivative = (m.response(plus, activation).unwrap().energy_density
                        - m.response(minus, activation).unwrap().energy_density)
                        / 2e-6;
                    let analytic = r.first_piola[i][k];
                    assert!(
                        (analytic - derivative).abs() < 1e-3 + analytic.abs() * 1e-6,
                        "{i},{k}: {analytic} vs {derivative}"
                    );
                }
            }
        }
    }
}
#[test]
fn reference_rotation_and_orthotropy() {
    let m = material();
    let rest = m.response(IDENTITY, 0.).unwrap();
    assert!(rest.energy_density.abs() < 1e-10);
    assert!(rest.first_piola.iter().flatten().all(|v| v.abs() < 1e-10));
    let rotation = [[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]];
    let rotated = m.response(rotation, 0.).unwrap();
    assert!(rotated.energy_density.abs() < 1e-10);
    let stretch = [
        [1.1, 0., 0.],
        [0., 1. / 1.1f64.sqrt(), 0.],
        [0., 0., 1. / 1.1f64.sqrt()],
    ];
    let transverse = [
        [1. / 1.1f64.sqrt(), 0., 0.],
        [0., 1.1, 0.],
        [0., 0., 1. / 1.1f64.sqrt()],
    ];
    assert!(
        m.response(stretch, 0.).unwrap().energy_density
            > m.response(transverse, 0.).unwrap().energy_density
    );
    let rotated_stretch = [
        [0., -stretch[1][1], 0.],
        [stretch[0][0], 0., 0.],
        [0., 0., stretch[2][2]],
    ];
    let a = m.response(stretch, 0.5).unwrap();
    let b = m.response(rotated_stretch, 0.5).unwrap();
    assert!((a.energy_density - b.energy_density).abs() < 1e-8);
    for k in 0..3 {
        assert!((b.first_piola[0][k] + a.first_piola[1][k]).abs() < 1e-8);
        assert!((b.first_piola[1][k] - a.first_piola[0][k]).abs() < 1e-8);
    }
}
#[test]
fn invalid_frames_inversion_and_overflow_rejected() {
    let mut m = material();
    m.sheet_direction = m.fiber_direction;
    assert!(m.response(IDENTITY, 0.).is_err());
    assert!(
        material()
            .response([[-1., 0., 0.], [0., 1., 0.], [0., 0., 1.]], 0.)
            .is_err()
    );
    assert!(
        material()
            .response([[100., 0., 0.], [0., 0.1, 0.], [0., 0., 0.1]], 0.)
            .is_err()
    );
    assert!(material().response(IDENTITY, f64::NAN).is_err());
}
#[test]
fn cardiac_fem_gradient_and_spatial_stress_use_assigned_law() {
    use physics::biomechanics::{Body, Material};
    let points = vec![[0., 0., 0.], [0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]];
    let mut body = Body::new(
        points.clone(),
        vec![true; 4],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 1000.,
                bulk_pa: 1e5,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    body.set_myocardium(0, material()).unwrap();
    body.set_activation(0, 0.4).unwrap();
    let f = [[1.08, 0.04, 0.], [0., 0.96, 0.], [0., 0., 1.01]];
    let deformed: Vec<_> = points
        .iter()
        .map(|p| std::array::from_fn(|i| (0..3).map(|k| f[i][k] * p[k]).sum()))
        .collect();
    let (energy, gradient) = body.evaluate(&deformed).unwrap();
    let r = material().response(f, 0.4).unwrap();
    assert!((energy - r.energy_density * 1e-6 / 6.).abs() < 1e-12);
    for i in 0..4 {
        for k in 0..3 {
            let mut plus = deformed.clone();
            let mut minus = deformed.clone();
            plus[i][k] += 1e-8;
            minus[i][k] -= 1e-8;
            let fd = (body.evaluate(&plus).unwrap().0 - body.evaluate(&minus).unwrap().0) / 2e-8;
            assert!((fd - gradient[i][k]).abs() < 1e-6);
        }
    }
    let stress = body.stresses_at(&deformed).unwrap()[0].stress.cauchy_pa;
    for i in 0..3 {
        for k in 0..3 {
            let expected =
                (0..3).map(|a| r.first_piola[i][a] * f[k][a]).sum::<f64>() / r.volume_ratio;
            assert!((stress[i][k] - expected).abs() < 1e-6);
        }
    }
}
#[test]
fn cardiac_activation_contracts_a_loaded_fem_element_at_equilibrium() {
    use physics::biomechanics::{Body, Material};
    let mut body = Body::new(
        vec![[0., 0., 0.], [0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]],
        vec![true, false, true, true],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 1000.,
                bulk_pa: 1e5,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    body.set_myocardium(0, material()).unwrap();
    body.set_activation(0, 0.1).unwrap();
    let report = body.equilibrate(2000, 1e-7).unwrap();
    assert!(report.residual_n < 1e-7, "{report:?}");
    assert!(body.positions()[1][0] < 0.01);
    assert!(body.positions()[1][0] > 0.009);
    let first_contracted = body.positions()[1][0];
    for _ in 0..3 {
        body.set_activation(0, 0.).unwrap();
        let relaxed = body.equilibrate(2000, 1e-7).unwrap();
        assert!(relaxed.converged, "{relaxed:?}");
        assert!((body.positions()[1][0] - 0.01).abs() < 1e-9);
        let (energy, _) = body.evaluate(body.positions()).unwrap();
        assert!(energy.abs() < 1e-12);
        body.set_activation(0, 0.1).unwrap();
        let active = body.equilibrate(2000, 1e-7).unwrap();
        assert!(active.converged, "{active:?}");
        assert!((body.positions()[1][0] - first_contracted).abs() < 1e-9);
    }
}

#[test]
fn batch_material_field_matches_individual_assignments_and_is_atomic() {
    use physics::biomechanics::{Body, Material};
    let base = Body::new(
        vec![
            [0., 0., 0.],
            [0.01, 0., 0.],
            [0., 0.01, 0.],
            [0., 0., 0.01],
            [0., 0., -0.01],
        ],
        vec![true, true, true, false, false],
        vec![
            (
                [0, 1, 2, 3],
                Material::from_young_poisson(8000., 0.3).unwrap(),
            ),
            (
                [0, 1, 2, 4],
                Material::from_young_poisson(8000., 0.3).unwrap(),
            ),
        ],
    )
    .unwrap();
    let mut sequential = base.clone();
    let mut batch = base.clone();
    let law = material();
    sequential.set_myocardium(0, law).unwrap();
    sequential.set_myocardium(1, law).unwrap();
    batch.set_myocardium_batch(&[(0, law), (1, law)]).unwrap();
    let mut x = base.positions().to_vec();
    x[3][0] += 0.0001;
    x[4][2] -= 0.0001;
    assert_eq!(
        sequential.evaluate(&x).unwrap(),
        batch.evaluate(&x).unwrap()
    );
    let before = batch.evaluate(&x).unwrap();
    let mut invalid = law;
    invalid.bulk_pa = -1.;
    assert!(
        batch
            .set_myocardium_batch(&[(0, law), (1, invalid)])
            .is_err()
    );
    assert_eq!(before, batch.evaluate(&x).unwrap());
    assert!(batch.set_myocardium_batch(&[(0, law), (0, law)]).is_err());
    assert!(batch.set_myocardium_batch(&[(0, law), (2, law)]).is_err());
    assert_eq!(before, batch.evaluate(&x).unwrap());
}

#[test]
fn near_rest_myocardium_preserves_quadratic_shear_energy() {
    let m = material();
    for gamma in [1e-6, 1e-8, 1e-10] {
        let f = [[1., gamma, 0.], [0., 1., 0.], [0., 0., 1.]];
        let response = m.response(f, 0.).unwrap();
        let expected = 0.5 * (m.matrix.scale_pa + m.fiber_sheet.scale_pa) * gamma * gamma;
        assert!((response.energy_density / expected - 1.).abs() < 1e-8);
        assert!(
            (response.first_piola[0][1] / ((m.matrix.scale_pa + m.fiber_sheet.scale_pa) * gamma)
                - 1.)
                .abs()
                < 1e-8
        );
    }
}

#[test]
fn cardiac_field_preserves_neighboring_viscoelastic_memory() {
    use physics::biomechanics::{Body, Material, MaxwellBranch, OgdenTerm, ViscoelasticOgden};
    let mut law = ViscoelasticOgden::new(
        vec![OgdenTerm {
            shear_pa: 400.,
            exponent: 2.,
        }],
        5000.,
        vec![MaxwellBranch {
            shear_pa: 700.,
            relaxation_seconds: 0.2,
        }],
    )
    .unwrap();
    let strain = [[1.08, 0.03, 0.], [0., 0.96, 0.], [0., 0., 1.]];
    law.advance(strain, 0.1).unwrap();
    let expected = law.response(IDENTITY, 0.).unwrap();
    assert!(expected.first_piola.iter().flatten().any(|v| v.abs() > 1.));
    let mut body = Body::new(
        vec![
            [0., 0., 0.],
            [0.01, 0., 0.],
            [0., 0.01, 0.],
            [0., 0., 0.01],
            [0., 0., -0.01],
        ],
        vec![true; 5],
        vec![
            (
                [0, 1, 2, 3],
                Material::from_young_poisson(8000., 0.3).unwrap(),
            ),
            (
                [0, 1, 2, 4],
                Material::from_young_poisson(8000., 0.3).unwrap(),
            ),
        ],
    )
    .unwrap();
    body.set_viscoelastic_ogden(1, law).unwrap();
    body.set_myocardium_batch(&[(0, material())]).unwrap();
    let retained = body.elements()[1].response(IDENTITY).unwrap();
    assert_eq!(retained.energy_density, expected.energy_density);
    assert_eq!(retained.first_piola, expected.first_piola);
    // Evolving the composite advances the untouched branch once, not on assignment.
    let mut reference = body.clone();
    body.set_myocardium_batch(&[(0, material())]).unwrap();
    body.relax_step(0.05, 2000, 1e-7).unwrap();
    reference.relax_step(0.05, 2000, 1e-7).unwrap();
    assert_eq!(
        body.elements()[1].response(IDENTITY).unwrap().first_piola,
        reference.elements()[1]
            .response(IDENTITY)
            .unwrap()
            .first_piola
    );
    assert_ne!(
        body.elements()[1].response(IDENTITY).unwrap().first_piola,
        expected.first_piola
    );
}
