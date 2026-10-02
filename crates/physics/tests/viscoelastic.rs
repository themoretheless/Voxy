use physics::biomechanics::{
    Body, IDENTITY, Material, MaxwellBranch, OgdenTerm, ViscoelasticOgden,
};
fn material() -> ViscoelasticOgden {
    ViscoelasticOgden::new(
        vec![
            OgdenTerm {
                shear_pa: 300.,
                exponent: -4.,
            },
            OgdenTerm {
                shear_pa: 100.,
                exponent: 2.,
            },
        ],
        5000.,
        vec![
            MaxwellBranch {
                shear_pa: 700.,
                relaxation_seconds: 0.2,
            },
            MaxwellBranch {
                shear_pa: 400.,
                relaxation_seconds: 2.,
            },
        ],
    )
    .unwrap()
}
#[test]
fn spectral_ogden_and_incremental_maxwell_forces_match_energy_derivatives() {
    let mut m = material();
    m.advance(
        [
            [1.06, 0.05, 0.],
            [0., 0.97, 0.],
            [0., 0., 1. / (1.06 * 0.97)],
        ],
        0.1,
    )
    .unwrap();
    for f in [
        [[1.13, 0.11, 0.], [0.02, 0.95, 0.03], [0., 0.02, 0.99]],
        [[0.88, 0., 0.], [0., 1.05, 0.], [0., 0., 1.1]],
        IDENTITY,
    ] {
        for dt in [0., 0.03, 0.5] {
            let r = m.response(f, dt).unwrap();
            for i in 0..3 {
                for k in 0..3 {
                    let mut a = f;
                    let mut b = f;
                    a[i][k] += 1e-6;
                    b[i][k] -= 1e-6;
                    let fd = (m.response(a, dt).unwrap().energy_density
                        - m.response(b, dt).unwrap().energy_density)
                        / 2e-6;
                    assert!(
                        (r.first_piola[i][k] - fd).abs() < 1e-4,
                        "{i},{k}: {} vs {fd}",
                        r.first_piola[i][k]
                    );
                }
            }
        }
    }
}
#[test]
fn rigid_motion_objectivity_and_neo_hookean_limit() {
    let m = ViscoelasticOgden::new(
        vec![OgdenTerm {
            shear_pa: 1000.,
            exponent: 2.,
        }],
        10000.,
        vec![],
    )
    .unwrap();
    let base = Material {
        shear_pa: 1000.,
        bulk_pa: 10000.,
        fibers: vec![],
    };
    let f = [[1.1, 0.04, 0.], [0., 0.96, 0.05], [0., 0., 0.98]];
    let a = m.response(f, 0.).unwrap();
    let b = base.response(f, 0.).unwrap();
    assert!((a.energy_density - b.energy_density).abs() < 1e-9);
    for i in 0..3 {
        for k in 0..3 {
            assert!((a.first_piola[i][k] - b.first_piola[i][k]).abs() < 1e-8);
        }
    }
    let rotated = [[0., -0.96, -0.05], [1.1, 0.04, 0.], [0., 0., 0.98]];
    let r = m.response(rotated, 0.).unwrap();
    assert!((r.energy_density - a.energy_density).abs() < 1e-9);
    for k in 0..3 {
        assert!((r.first_piola[0][k] + a.first_piola[1][k]).abs() < 1e-8);
        assert!((r.first_piola[1][k] - a.first_piola[0][k]).abs() < 1e-8);
    }
    let rest = m.response(IDENTITY, 0.).unwrap();
    assert!(rest.energy_density.abs() < 1e-10);
    assert!(rest.first_piola.iter().flatten().all(|v| v.abs() < 1e-10));
}
#[test]
fn prescribed_strain_relaxes_with_discrete_maxwell_time_and_dissipates() {
    let mut m = ViscoelasticOgden::new(
        vec![OgdenTerm {
            shear_pa: 100.,
            exponent: 2.,
        }],
        10000.,
        vec![MaxwellBranch {
            shear_pa: 900.,
            relaxation_seconds: 0.2,
        }],
    )
    .unwrap();
    let equilibrium = ViscoelasticOgden::new(
        vec![OgdenTerm {
            shear_pa: 100.,
            exponent: 2.,
        }],
        10000.,
        vec![],
    )
    .unwrap();
    let f = [[1., 0.2, 0.], [0., 1., 0.], [0., 0., 1.]];
    let eq = equilibrium.response(f, 0.).unwrap().first_piola[0][1];
    let initial = m.response(f, 0.).unwrap().first_piola[0][1];
    let mut old_energy = m.response(f, 0.).unwrap().energy_density;
    for step in 1..=20 {
        let d = m.advance(f, 0.02).unwrap();
        assert!(d > 0.);
        let r = m.response(f, 0.).unwrap();
        let expected = eq + (initial - eq) * (1f64 / 1.1).powi(step);
        assert!((r.first_piola[0][1] - expected).abs() < 1e-9);
        assert!(
            old_energy - r.energy_density + 1e-10 >= d,
            "dissipation exceeds stored-energy loss"
        );
        old_energy = r.energy_density;
    }
}
fn body() -> Body {
    let mut b = Body::new(
        vec![[0., 0., 0.], [0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]],
        vec![true, false, true, true],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 400.,
                bulk_pa: 5000.,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    b.set_viscoelastic_ogden(0, material()).unwrap();
    b.set_force(1, [0.01, 0., 0.]).unwrap();
    b
}
#[test]
fn coupled_fem_creeps_under_constant_load_and_failed_step_is_atomic() {
    let mut b = body();
    b.relax_step(0.02, 4000, 1e-8).unwrap();
    let initial = b.positions()[1][0];
    for _ in 0..15 {
        b.relax_step(0.1, 4000, 1e-8).unwrap();
    }
    assert!(b.positions()[1][0] > initial + 1e-6);
    let stress = b.stresses_at(b.positions()).unwrap()[0].stress;
    assert!(stress.cauchy_pa.iter().flatten().all(|v| v.is_finite()));
    let mut b = body();
    let positions = b.positions().to_vec();
    let before = b.evaluate(&positions).unwrap();
    assert!(b.relax_step(0.1, 1, 1e-14).is_err());
    assert_eq!(positions, b.positions());
    assert_eq!(before, b.evaluate(&positions).unwrap());
    let f = [[1.1, 0.05, 0.], [0., 1., 0.], [0., 0., 1.]];
    let original = b.elements()[0].response(f).unwrap();
    assert!(b.relax_step(f64::NAN, 4000, 1e-8).is_err());
    assert_eq!(
        original.first_piola,
        b.elements()[0].response(f).unwrap().first_piola
    );
    assert!(b.set_activation(0, 0.1).is_err());
}
#[test]
fn input_validation_inversion_and_history_update_are_atomic() {
    assert!(
        ViscoelasticOgden::new(
            vec![OgdenTerm {
                shear_pa: 100.,
                exponent: 0.
            }],
            1000.,
            vec![]
        )
        .is_err()
    );
    let mut m = material();
    let f = [[1., 0.1, 0.], [0., 1., 0.], [0., 0., 1.]];
    let before = m.response(f, 0.).unwrap().first_piola;
    assert!(
        m.advance([[-1., 0., 0.], [0., 1., 0.], [0., 0., 1.]], 0.1)
            .is_err()
    );
    assert!(m.advance(f, 0.).is_err());
    assert_eq!(before, m.response(f, 0.).unwrap().first_piola);
}
#[test]
fn populated_reference_memory_rotates_objectively() {
    let mut m = material();
    let held = [[1.08, 0.12, 0.], [0., 0.96, 0.03], [0., 0., 1.]];
    m.advance(held, 0.3).unwrap();
    let f = [[1.1, 0.1, 0.], [0.02, 0.97, 0.], [0., 0.01, 0.99]];
    let rotated = [[-0.02, -0.97, 0.], [1.1, 0.1, 0.], [0., 0.01, 0.99]];
    for dt in [0., 0.1] {
        let a = m.response(f, dt).unwrap();
        let b = m.response(rotated, dt).unwrap();
        assert!((a.energy_density - b.energy_density).abs() < 1e-8);
        for k in 0..3 {
            assert!((a.first_piola[0][k] - b.first_piola[1][k]).abs() < 1e-7);
            assert!((a.first_piola[1][k] + b.first_piola[0][k]).abs() < 1e-7);
        }
    }
}
#[test]
fn ogden_spectral_energy_matches_exact_incompressible_uniaxial_expression() {
    let m = ViscoelasticOgden::new(
        vec![OgdenTerm {
            shear_pa: 300.,
            exponent: -4.,
        }],
        5000.,
        vec![],
    )
    .unwrap();
    for stretch in [0.8_f64, 0.9, 1., 1.1, 1.2] {
        let transverse = stretch.sqrt().recip();
        let f = [
            [stretch, 0., 0.],
            [0., transverse, 0.],
            [0., 0., transverse],
        ];
        let exact = 600. / 16. * (stretch.powf(-4.) + 2. * stretch.powf(2.) - 3.);
        assert!((m.response(f, 0.).unwrap().energy_density - exact).abs() < 1e-9);
    }
    let stretch = |l: f64| {
        [
            [l, 0., 0.],
            [0., l.sqrt().recip(), 0.],
            [0., 0., l.sqrt().recip()],
        ]
    };
    assert!(
        m.response(stretch(0.9), 0.).unwrap().energy_density
            > m.response(stretch(1.1), 0.).unwrap().energy_density
    );
}
#[test]
fn spectral_energy_does_not_lose_small_shear_near_repeated_eigenvalues() {
    let m = ViscoelasticOgden::new(
        vec![OgdenTerm {
            shear_pa: 2000.,
            exponent: 2.,
        }],
        20000.,
        vec![],
    )
    .unwrap();
    for shear in [1e-8_f64, 1e-6, 1e-4] {
        let mut f = IDENTITY;
        f[0][1] = shear;
        let energy = m.response(f, 0.).unwrap().energy_density;
        let expected = 1000. * shear * shear;
        assert!(
            (energy - expected).abs() < expected * 1e-6,
            "{shear}: {energy} vs {expected}"
        );
    }
}

#[test]
fn lbfgs_physical_steps_match_cg_memory_and_failed_step_is_atomic() {
    let mut cg = body();
    let mut lbfgs = cg.clone();
    let reference = cg.rest_positions().to_vec();
    for dt in [0.02, 0.1, 0.1, 0.2] {
        assert!(cg.relax_step(dt, 4000, 1e-8).unwrap().converged);
        let r = lbfgs
            .relax_step_lbfgs_states(dt, 4000, 1e-8, |_, _, _, _, _, _, _| {})
            .unwrap();
        assert!(r.converged);
        let committed_gradient = lbfgs.evaluate(lbfgs.positions()).unwrap().1;
        let residual = committed_gradient[1]
            .iter()
            .map(|g| g * g)
            .sum::<f64>()
            .sqrt();
        assert!(
            residual <= 1e-8,
            "committed residual={residual} reported={}",
            r.residual_n
        );
        for (a, b) in cg.positions().iter().zip(lbfgs.positions()) {
            for k in 0..3 {
                assert!((a[k] - b[k]).abs() < 1e-8);
            }
        }
        let f = [[1.1, 0.05, 0.], [0., 1., 0.], [0., 0., 1.]];
        let a = cg.elements()[0].response(f).unwrap();
        let b = lbfgs.elements()[0].response(f).unwrap();
        for i in 0..3 {
            for k in 0..3 {
                assert!((a.first_piola[i][k] - b.first_piola[i][k]).abs() < 0.01);
            }
        }
        assert_eq!(lbfgs.rest_positions(), reference);
    }
    let mut b = body();
    let x = b.positions().to_vec();
    let e = b.evaluate(&x).unwrap();
    let f = [[1.1, 0.05, 0.], [0., 1., 0.], [0., 0., 1.]];
    let response = b.elements()[0].response(f).unwrap();
    assert!(
        b.relax_step_lbfgs_states(0.1, 1, 1e-14, |_, _, _, _, _, _, _| {})
            .is_err()
    );
    assert_eq!(b.positions(), x);
    assert_eq!(b.evaluate(&x).unwrap(), e);
    assert_eq!(
        b.elements()[0].response(f).unwrap().first_piola,
        response.first_piola
    );
}
#[test]
fn batch_assignment_matches_single_and_rejects_invalid_profile_atomically() {
    let mut single = body();
    let mut batch = single.clone();
    single.set_viscoelastic_ogden(0, material()).unwrap();
    batch
        .set_viscoelastic_ogden_batch(&[(0, material())])
        .unwrap();
    assert_eq!(
        single.evaluate(single.positions()).unwrap(),
        batch.evaluate(batch.positions()).unwrap()
    );
    let before = batch.evaluate(batch.positions()).unwrap();
    assert!(
        batch
            .set_viscoelastic_ogden_batch(&[(0, material()), (0, material())])
            .is_err()
    );
    assert!(
        batch
            .set_viscoelastic_ogden_batch(&[(1, material())])
            .is_err()
    );
    assert_eq!(batch.evaluate(batch.positions()).unwrap(), before);
}
