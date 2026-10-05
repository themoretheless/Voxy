use physics::biomechanics::{
    Body, InertialBody, Material, MaxwellBranch, OgdenTerm, SupportTarget, ViscoelasticOgden,
};
fn law() -> ViscoelasticOgden {
    ViscoelasticOgden::new(
        vec![OgdenTerm {
            shear_pa: 100.,
            exponent: 2.,
        }],
        1000.,
        vec![MaxwellBranch {
            shear_pa: 700.,
            relaxation_seconds: 0.2,
        }],
    )
    .unwrap()
}
#[test]
fn exact_relaxation_matches_analytic_energy_and_subdivision() {
    let shear: f64 = 0.2;
    let f = [[1., shear, 0.], [0., 1., 0.], [0., 0., 1.]];
    let initial_branch = 700. * (shear.powi(2) / 2. + shear.powi(4) / 6.);
    let expected_heat = initial_branch * (1. - (-2_f64 * 0.3 / 0.2).exp());
    let mut full = law();
    let before = full.response(f, 0.).unwrap().energy_density;
    assert!((full.maxwell_energy_density(f).unwrap() - initial_branch).abs() < 1e-10);
    let heat = full.relax_exact(f, 0.3).unwrap();
    assert!((heat - expected_heat).abs() < 1e-10);
    assert!((full.maxwell_energy_density(f).unwrap() + heat - initial_branch).abs() < 1e-10);
    assert!((full.response(f, 0.).unwrap().energy_density + heat - before).abs() < 1e-10);
    let mut split = law();
    let sum: f64 = (0..10).map(|_| split.relax_exact(f, 0.03).unwrap()).sum();
    assert!((sum - heat).abs() < 1e-10);
    assert!(
        (split.response(f, 0.).unwrap().energy_density
            - full.response(f, 0.).unwrap().energy_density)
            .abs()
            < 1e-10
    );
    let rotated = [[0., -1., 0.], [1., shear, 0.], [0., 0., 1.]];
    assert!(
        (full.response(rotated, 0.).unwrap().energy_density
            - full.response(f, 0.).unwrap().energy_density)
            .abs()
            < 1e-10
    );
    let snapshot = format!("{full:?}");
    assert!(full.relax_exact(f, f64::NAN).is_err());
    assert!(full.relax_exact([[0.; 3]; 3], 0.1).is_err());
    assert_eq!(snapshot, format!("{full:?}"));
}
fn specimen() -> InertialBody {
    let mut body = Body::new(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![true; 4],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 100.,
                bulk_pa: 1000.,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    body.set_viscoelastic_ogden(0, law()).unwrap();
    assert!(InertialBody::new_with_fixed_supports(body.clone(), &[1.], vec![[0.; 3]; 4]).is_err());
    InertialBody::new_viscoelastic_with_supports(body, &[1.], vec![[0.; 3]; 4]).unwrap()
}
#[test]
fn driven_shear_then_hold_balances_work_heat_and_rollback() {
    let mut body = specimen();
    let before = body.diagnostics().unwrap();
    let mut work = 0.;
    let mut heat = 0.;
    for step in 1..=100 {
        let shear = 0.001 * f64::from(step.min(20));
        let points = [[0.; 3], [1., 0., 0.], [shear, 1., 0.], [0., 0., 1.]];
        let targets: Vec<_> = points
            .into_iter()
            .enumerate()
            .map(|(node, position_m)| SupportTarget { node, position_m })
            .collect();
        let receipt = body.step_viscoelastic(Some(&targets), 0.01, 1e-6).unwrap();
        assert!(receipt.viscous_heat_j >= 0.);
        work += receipt.support.support_work_j;
        heat += receipt.viscous_heat_j;
    }
    let after = body.diagnostics().unwrap();
    assert!(
        (after.kinetic_j + after.potential_j - before.kinetic_j - before.potential_j - work + heat)
            .abs()
            < 1e-5
    );
    assert!(heat > 0.);
    let snapshot = format!("{body:?}");
    assert!(body.step(0.01, 1e-6).is_err());
    assert!(body.step_viscoelastic(Some(&[]), 0.01, 1e-6).is_err());
    assert_eq!(snapshot, format!("{body:?}"));
    let energy = body.diagnostics().unwrap().potential_j;
    let receipt = body.step_viscoelastic(None, 0.1, 1e-8).unwrap();
    assert!(receipt.viscous_heat_j > 0.);
    assert!(body.diagnostics().unwrap().potential_j < energy);
}

#[test]
fn free_node_moves_and_failed_driving_preserves_material_history() {
    let mut body = Body::new(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![true, true, true, false],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 100.,
                bulk_pa: 1000.,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    body.set_viscoelastic_ogden(0, law()).unwrap();
    let mut body =
        InertialBody::new_viscoelastic_with_supports(body, &[1.], vec![[0.; 3]; 4]).unwrap();
    let targets: Vec<_> = body.body().positions()[..3]
        .iter()
        .enumerate()
        .map(|(node, p)| SupportTarget {
            node,
            position_m: [p[0] + 0.0001, p[1], p[2]],
        })
        .collect();
    body.step_viscoelastic(Some(&targets), 0.001, 1e-7).unwrap();
    assert!(body.velocities()[3][0].abs() > 1e-8);
    let snapshot = format!("{body:?}");
    assert!(
        body.step_with_support_targets(&targets, 0.001, 1e-7)
            .is_err()
    );
    assert_eq!(snapshot, format!("{body:?}"));
    let inverted: Vec<_> = body.body().positions()[..3]
        .iter()
        .enumerate()
        .map(|(node, p)| SupportTarget {
            node,
            position_m: [p[0], p[1], 2.],
        })
        .collect();
    assert!(body.step_viscoelastic(Some(&inverted), 0.001, 1.).is_err());
    assert_eq!(snapshot, format!("{body:?}"));
}

#[test]
fn unforced_viscoelastic_motion_converges_under_time_refinement() {
    let mut endpoints = Vec::new();
    let mut defects = Vec::new();
    for steps in [25, 50, 100] {
        let mut body = Body::new(
            vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            vec![true, true, true, false],
            vec![(
                [0, 1, 2, 3],
                Material {
                    shear_pa: 100.,
                    bulk_pa: 1000.,
                    fibers: vec![],
                },
            )],
        )
        .unwrap();
        body.set_viscoelastic_ogden(0, law()).unwrap();
        let mut velocities = vec![[0.; 3]; 4];
        velocities[3] = [0.01, 0., 0.];
        let mut body =
            InertialBody::new_viscoelastic_with_supports(body, &[1.], velocities).unwrap();
        let initial = body.diagnostics().unwrap();
        let mut heat = 0.;
        let mut defect = 0.;
        for _ in 0..steps {
            let receipt = body
                .step_viscoelastic(None, 0.1 / f64::from(steps), 1e-6)
                .unwrap();
            heat += receipt.viscous_heat_j;
            defect += receipt.total_energy_defect_j;
        }
        let final_energy = body.diagnostics().unwrap();
        assert!(heat > 0.);
        assert!(
            (final_energy.kinetic_j + final_energy.potential_j
                - initial.kinetic_j
                - initial.potential_j
                + heat
                - defect)
                .abs()
                < 1e-10
        );
        endpoints.push((body.body().positions()[3], body.velocities()[3]));
        defects.push(defect.abs());
    }
    let distance = |a: &([f64; 3], [f64; 3]), b: &([f64; 3], [f64; 3])| -> f64 {
        a.0.iter()
            .chain(&a.1)
            .zip(b.0.iter().chain(&b.1))
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    assert!(
        distance(&endpoints[1], &endpoints[2]) < 0.35 * distance(&endpoints[0], &endpoints[1]),
        "endpoints={endpoints:?}"
    );
    assert!(
        defects[1] < 0.35 * defects[0] && defects[2] < 0.35 * defects[1],
        "defects={defects:?}"
    );
}

#[test]
fn hgo_stress_memory_is_not_admitted_as_passive_maxwell_dynamics() {
    use physics::biomechanics::{HgoMaterial, ViscoelasticHgo};
    let mut body = Body::new(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![true, true, true, false],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 100.,
                bulk_pa: 1000.,
                fibers: vec![],
            },
        )],
    )
    .unwrap();
    let hgo = ViscoelasticHgo::new(
        HgoMaterial {
            shear_pa: 100.,
            bulk_pa: 1000.,
            fibers: vec![],
        },
        &[(0.2, 0.5)],
    )
    .unwrap();
    body.set_viscoelastic_hgo_batch(&[(0, hgo)]).unwrap();
    assert!(InertialBody::new_with_fixed_supports(body.clone(), &[1.], vec![[0.; 3]; 4]).is_err());
    assert!(InertialBody::new_viscoelastic_with_supports(body, &[1.], vec![[0.; 3]; 4]).is_err());
}

#[test]
fn unrelated_large_gravity_potential_cannot_hide_or_block_maxwell_heat() {
    let mut body = specimen();
    let targets: Vec<_> = body
        .body()
        .positions()
        .iter()
        .enumerate()
        .map(|(node, p)| SupportTarget {
            node,
            position_m: [p[0] + 0.001 * p[1], p[1], p[2]],
        })
        .collect();
    body.step_viscoelastic(Some(&targets), 0.1, 1e-6).unwrap();
    body.step_viscoelastic(Some(&targets), 0.01, 1e-8).unwrap();
    let mut reference = body.clone();
    body.set_uniform_acceleration([1e16, 0., 0.]).unwrap();
    let before = body.body().evaluate(body.body().positions()).unwrap().0;
    let rounded_total_before = body.diagnostics().unwrap().potential_j;
    let receipt = body.step_viscoelastic(None, 0.01, 1e-10).unwrap();
    let normal = reference.step_viscoelastic(None, 0.01, 1e-10).unwrap();
    assert!(receipt.viscous_heat_j > 1e-6);
    // A guard based on subtracting these rounded global potentials would fail.
    let rounded_total_after = body.diagnostics().unwrap().potential_j;
    assert!((rounded_total_after - rounded_total_before + receipt.viscous_heat_j).abs() > 1e-10);
    assert_eq!(receipt.viscous_heat_j, normal.viscous_heat_j);
    let after = body.body().evaluate(body.body().positions()).unwrap().0;
    assert!((after - before + receipt.viscous_heat_j).abs() < 1e-12);
    assert_eq!(body.body().positions(), reference.body().positions());
    assert_eq!(body.velocities(), reference.velocities());
    assert_eq!(
        after,
        reference
            .body()
            .evaluate(reference.body().positions())
            .unwrap()
            .0
    );
}

#[test]
fn maxwell_heat_updates_cell_temperature_and_failures_preserve_both_owners() {
    let mut body = specimen();
    let before = format!("{body:?}");
    for (cp, t) in [
        (vec![], vec![300.]),
        (vec![0.], vec![300.]),
        (vec![2.], vec![f64::NAN]),
    ] {
        assert!(body.enable_maxwell_thermal(&cp, &t).is_err());
        assert_eq!(before, format!("{body:?}"));
    }
    body.enable_maxwell_thermal(&[2.], &[300.]).unwrap();
    let configured = format!("{body:?}");
    assert!(body.enable_maxwell_thermal(&[2.], &[310.]).is_err());
    assert_eq!(configured, format!("{body:?}"));
    let initial = body.diagnostics().unwrap();
    let mut work = 0.;
    let mut heat = 0.;
    let mut defects = 0.;
    for step in 1..=50 {
        let shear = 0.001 * f64::from(step.min(20));
        let points = [[0.; 3], [1., 0., 0.], [shear, 1., 0.], [0., 0., 1.]];
        let targets: Vec<_> = points
            .into_iter()
            .enumerate()
            .map(|(node, position_m)| SupportTarget { node, position_m })
            .collect();
        let receipt = body.step_viscoelastic(Some(&targets), 0.01, 1e-6).unwrap();
        work += receipt.support.support_work_j;
        heat += receipt.viscous_heat_j;
        defects += receipt.total_energy_defect_j;
    }
    let stored = body.maxwell_sensible_energy_j().unwrap()[0];
    let temperature = body.maxwell_temperatures_kelvin().unwrap()[0];
    assert!(stored > 0. && temperature > 300.);
    assert!((stored - heat).abs() < 1e-12);
    // Unit tetra density=1 gives mass 1/6 kg; cp=2 gives capacity 1/3 J/K.
    assert!((temperature - 300. - stored * 3.).abs() < 1e-12);
    let final_state = body.diagnostics().unwrap();
    assert!(
        (final_state.kinetic_j + final_state.potential_j + stored
            - initial.kinetic_j
            - initial.potential_j
            - work
            - defects)
            .abs()
            < 1e-10
    );
    let snapshot = format!("{body:?}");
    assert!(body.step_viscoelastic(Some(&[]), 0.01, 1e-6).is_err());
    assert_eq!(snapshot, format!("{body:?}"));
}

#[test]
fn late_temperature_overflow_rolls_back_mechanics_memory_and_heat() {
    let mut body = specimen();
    body.enable_maxwell_thermal(&[1e-320], &[300.]).unwrap();
    let snapshot = format!("{body:?}");
    let targets: Vec<_> = body
        .body()
        .positions()
        .iter()
        .enumerate()
        .map(|(node, p)| SupportTarget {
            node,
            position_m: [p[0] + 0.001 * p[1], p[1], p[2]],
        })
        .collect();
    assert_eq!(
        body.step_viscoelastic(Some(&targets), 0.01, 1e-6)
            .unwrap_err(),
        "unrepresentable Maxwell thermal inventory"
    );
    assert_eq!(snapshot, format!("{body:?}"));
    assert_eq!(body.maxwell_sensible_energy_j().unwrap(), vec![0.]);
    assert_eq!(body.maxwell_temperatures_kelvin().unwrap(), vec![300.]);
}
