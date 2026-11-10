use physics::biomechanics::*;
fn material() -> Material {
    Material {
        shear_pa: 12_000.,
        bulk_pa: 120_000.,
        fibers: vec![Fiber {
            direction: [1., 0., 0.],
            stiffness_pa: 8000.,
            exponent: 6.,
            active_pa: 10_000.,
        }],
    }
}
#[test]
fn analytic_stress_matches_energy_gradient_and_rigid_rotation() {
    let m = material();
    let f = [[1.12, 0.08, 0.], [0.02, 0.97, 0.04], [0., 0., 1.01]];
    let r = m.response(f, 0.4).unwrap();
    for i in 0..3 {
        for j in 0..3 {
            let mut plus = f;
            let mut minus = f;
            plus[i][j] += 1e-6;
            minus[i][j] -= 1e-6;
            let derivative = (m.response(plus, 0.4).unwrap().energy_density
                - m.response(minus, 0.4).unwrap().energy_density)
                / 2e-6;
            assert!(
                (derivative - r.first_piola[i][j]).abs() < 1e-4,
                "stress derivative {i} {j}"
            );
        }
    }
    let rest = m.response(IDENTITY, 0.).unwrap();
    assert!(rest.energy_density.abs() < 1e-10);
    let rotation = [[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]];
    let rotated = m.response(rotation, 0.).unwrap();
    assert!(rotated.energy_density.abs() < 1e-10);
    assert!(
        rotated
            .first_piola
            .iter()
            .flatten()
            .all(|x| x.abs() < 1e-10)
    );
    let rf: Matrix = std::array::from_fn(|i| {
        std::array::from_fn(|j| (0..3).map(|k| rotation[i][k] * f[k][j]).sum())
    });
    assert!((m.response(rf, 0.4).unwrap().energy_density - r.energy_density).abs() < 1e-9);
}
#[test]
fn matrix_small_strain_shear_is_physical_and_fibers_are_directional() {
    let mut m = material();
    m.fibers.clear();
    let mut f = IDENTITY;
    f[0][1] = 1e-4;
    assert!((m.response(f, 0.).unwrap().first_piola[0][1] / 1e-4 - m.shear_pa).abs() < 1e-5);
    let m = material();
    let mut x = IDENTITY;
    let mut y = IDENTITY;
    x[0][0] = 1.15;
    y[1][1] = 1.15;
    assert!(m.response(x, 0.).unwrap().energy_density > m.response(y, 0.).unwrap().energy_density);
}
#[test]
fn assembled_pressure_energy_gradient_and_force_balance() {
    let mut b = tube(&[0.008, 0.011], 0.025, 12, 2, &[material()], true).unwrap();
    b.set_pressure(0, 2000.).unwrap();
    let (_, g) = b.evaluate(b.positions()).unwrap();
    let sum: Vec3 = std::array::from_fn(|k| g.iter().map(|p| p[k]).sum());
    assert!(sum.iter().all(|x| x.abs() < 1e-10));
    for i in [0, 15, 38, 60] {
        for k in 0..3 {
            let mut plus = b.positions().to_vec();
            let mut minus = plus.clone();
            plus[i][k] += 1e-8;
            minus[i][k] -= 1e-8;
            let de = (b.evaluate(&plus).unwrap().0 - b.evaluate(&minus).unwrap().0) / 2e-8;
            assert!(
                (de - g[i][k]).abs() < 1e-6,
                "nodal derivative {i} {k}: {de} != {}",
                g[i][k]
            );
        }
    }
    let mut shifted = b.positions().to_vec();
    for p in &mut shifted {
        p[0] += 0.125;
        p[1] -= 0.03;
    }
    assert!((b.evaluate(&shifted).unwrap().0 - b.evaluate(b.positions()).unwrap().0).abs() < 1e-10);
}
#[test]
fn pressure_expands_and_muscle_activation_contracts() {
    let radius = |b: &Body| {
        b.positions().iter().map(|p| p[0].hypot(p[1])).sum::<f64>() / b.positions().len() as f64
    };
    let mut passive = material();
    passive.fibers.clear();
    let mut b = tube(&[0.008, 0.011], 0.025, 12, 2, &[passive], false).unwrap();
    let initial = radius(&b);
    b.set_pressure(0, 1000.).unwrap();
    let report = b.equilibrate(4000, 1e-5).unwrap();
    println!("pressure report {report:?}");
    assert!(report.converged);
    assert!(radius(&b) > initial * 1.01);
    assert!(report.min_j > 0.95);
    let mut ring = tube(&[0.008, 0.011], 0.025, 12, 2, &[material()], false).unwrap();
    let initial = radius(&ring);
    for i in 0..ring.elements().len() {
        ring.set_activation(i, 0.5).unwrap();
    }
    let report = ring.equilibrate(6000, 1e-5).unwrap();
    println!("active report {report:?}");
    assert!(report.converged);
    assert!(radius(&ring) < initial * 0.99);
    assert!(report.min_j > 0.9);
}
#[test]
fn rejects_inversion_open_pressure_and_invalid_load_atomically() {
    let mut b = tube(&[0.008, 0.011], 0.025, 12, 2, &[material()], false).unwrap();
    let old = b.positions().to_vec();
    assert!(b.set_pressure(0, f64::NAN).is_err());
    assert!(b.set_activation(0, 2.).is_err());
    assert!(b.set_force(0, [f64::NAN, 0., 0.]).is_err());
    assert!(b.equilibrate(0, 1e-5).is_err());
    assert_eq!(b.positions(), old);
    assert!(
        b.add_cavity(Cavity {
            faces: vec![[0, 1, 2]],
            pressure_pa: 1000.
        })
        .is_err()
    );
    let mut inverted = IDENTITY;
    inverted[0][0] = -1.;
    assert!(material().response(inverted, 0.).is_err());
}
#[test]
fn specimens_have_noninverted_connected_volume_and_bonded_layers() {
    for mut b in penile_chambers()
        .unwrap()
        .into_iter()
        .chain([sphincter_layers().unwrap()])
    {
        let r = b.equilibrate(1, 1e-8).unwrap();
        assert!(r.converged);
        assert!((r.min_j - 1.).abs() < 1e-10);
        assert!(!b.surface().is_empty());
        assert_eq!(b.cavities().len(), 1);
    }
}

#[test]
fn affine_patch_energy_is_exact_and_cavity_geometry_refines() {
    let mut m = material();
    m.fibers.clear();
    let f = [[1.05, 0.03, 0.], [0., 0.97, 0.], [0., 0., 1.02]];
    let density = m.response(f, 0.).unwrap().energy_density;
    let mut previous_error = f64::INFINITY;
    for n in [12, 24, 48] {
        let mut b = tube(&[0.008, 0.011], 0.025, n, 2, &[m.clone()], true).unwrap();
        let transformed: Vec<Vec3> = b
            .positions()
            .iter()
            .map(|p| f.map(|r| (0..3).map(|k| r[k] * p[k]).sum()))
            .collect();
        let polygon_area = n as f64 / 2. * (std::f64::consts::TAU / n as f64).sin();
        let expected_energy =
            polygon_area * (0.011_f64.powi(2) - 0.008_f64.powi(2)) * 0.025 * density;
        assert!((b.evaluate(&transformed).unwrap().0 - expected_energy).abs() < 1e-12);
        b.set_pressure(0, 1.).unwrap();
        let volume = -b.evaluate(b.positions()).unwrap().0;
        let exact = std::f64::consts::PI * 0.008_f64.powi(2) * 0.025;
        let error = (volume - exact).abs();
        assert!(error < previous_error * 0.3);
        previous_error = error;
    }
}

#[test]
fn calibration_recovers_material_and_detects_held_out_error() {
    let mut m = material();
    m.fibers[0].active_pa = 0.;
    let measurements = |stretches: &[f64]| {
        stretches
            .iter()
            .map(|&stretch| TensilePoint {
                stretch,
                nominal_stress_pa: tensile_stress(&m, stretch).unwrap(),
            })
            .collect::<Vec<_>>()
    };
    let train = measurements(&[1., 1.03, 1.08, 1.12, 1.2]);
    let fitted = fit_tensile(&train, m.bulk_pa, m.fibers[0].exponent).unwrap();
    assert!((fitted.shear_pa / m.shear_pa - 1.).abs() < 1e-10);
    assert!((fitted.fibers[0].stiffness_pa / m.fibers[0].stiffness_pa - 1.).abs() < 1e-10);
    let mut validation = measurements(&[1.05, 1.15, 1.25]);
    assert!(tensile_error(&fitted, &validation).unwrap().rmse_pa < 1e-8);
    for p in &mut validation {
        p.nominal_stress_pa += 500.;
    }
    assert!((tensile_error(&fitted, &validation).unwrap().rmse_pa - 500.).abs() < 1e-7);
    let repeated = vec![
        TensilePoint {
            stretch: 1.1,
            nominal_stress_pa: 1000.
        };
        3
    ];
    assert!(fit_tensile(&repeated, 1e5, 6.).is_err());
}

#[test]
fn near_rest_energy_retains_tiny_shear_instead_of_cancelling_to_zero() {
    let m = Material {
        shear_pa: 2000.,
        bulk_pa: 20000.,
        fibers: vec![],
    };
    for shear in [1e-8_f64, 1e-7, 1e-6, 1e-4, 0.01] {
        let mut f = IDENTITY;
        f[0][1] = shear;
        let energy = m.response(f, 0.).unwrap().energy_density;
        let expected = 0.5 * m.shear_pa * shear * shear;
        assert!(
            (energy - expected).abs() <= expected * 1e-8,
            "{shear}: {energy} vs {expected}"
        );
    }
}
