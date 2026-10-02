use physics::biomechanics::{Body, IDENTITY, Material, Matrix, Stress};
fn close(a: f64, b: f64, tolerance: f64) {
    assert!((a - b).abs() <= tolerance, "{a} != {b}");
}
fn multiply(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}
fn transpose(a: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| a[j][i]))
}
#[test]
fn analytical_uniaxial_shear_and_hydrostatic_states() {
    let axial = Stress::from_cauchy([[100., 0., 0.], [0., 0., 0.], [0., 0., 0.]]).unwrap();
    close(axial.von_mises_pa, 100., 1e-12);
    close(axial.max_shear_pa, 50., 1e-12);
    close(axial.pressure_pa, -100. / 3., 1e-12);
    close(axial.yield_utilization(80.).unwrap(), 1.25, 1e-12);
    let shear = Stress::from_cauchy([[0., 40., 0.], [40., 0., 0.], [0., 0., 0.]]).unwrap();
    close(shear.von_mises_pa, 3_f64.sqrt() * 40., 1e-12);
    for (a, b) in shear.principal_pa.into_iter().zip([40., 0., -40.]) {
        close(a, b, 1e-12);
    }
    for value in [-200., 0., 200., 1e200, 1e-200] {
        let s = Stress::from_cauchy(IDENTITY.map(|r| r.map(|v| v * value))).unwrap();
        close(s.pressure_pa, -value, value.abs() * 1e-14);
        close(s.von_mises_pa, 0., 0.);
        assert_eq!(s.principal_pa, [value; 3]);
    }
}
#[test]
fn invariants_and_spatial_stress_rotate_with_body() {
    let r = [[0.36, -0.8, 0.48], [0.48, 0.6, 0.64], [-0.8, 0., 0.6]];
    let sigma = [[170., 30., -45.], [30., -80., 25.], [-45., 25., 110.]];
    let a = Stress::from_cauchy(sigma).unwrap();
    let b = Stress::from_cauchy(multiply(multiply(r, sigma), transpose(r))).unwrap();
    for (x, y) in a.principal_pa.into_iter().zip(b.principal_pa) {
        close(x, y, 1e-10);
    }
    close(a.von_mises_pa, b.von_mises_pa, 1e-10);
    let m = Material::from_young_poisson(2e6, 0.3).unwrap();
    let f = [[1.1, 0.2, 0.], [0., 0.94, 0.1], [0., 0., 1.03]];
    let original = m.stress(f, 0.).unwrap();
    let rotated = m.stress(multiply(r, f), 0.).unwrap();
    let expected = multiply(multiply(r, original.cauchy_pa), transpose(r));
    for (x, y) in rotated
        .cauchy_pa
        .iter()
        .flatten()
        .zip(expected.iter().flatten())
    {
        close(*x, *y, 1e-8);
    }
    close(original.von_mises_pa, rotated.von_mises_pa, 1e-8);
    assert!(m.stress(r, 0.).unwrap().von_mises_pa < 1e-8);
}
#[test]
fn young_poisson_small_strain_and_affine_tetra_patch() {
    let young = 2e6;
    let nu = 0.3;
    let m = Material::from_young_poisson(young, nu).unwrap();
    let eps = 1e-7;
    let f = [
        [1. + eps, 0., 0.],
        [0., 1. - nu * eps, 0.],
        [0., 0., 1. - nu * eps],
    ];
    let s = m.stress(f, 0.).unwrap();
    close(s.cauchy_pa[0][0] / eps, young, 1.);
    close(s.cauchy_pa[1][1] / eps, 0., 1.);
    let rest = vec![[0., 0., 0.], [0.02, 0., 0.], [0., 0.03, 0.], [0., 0., 0.04]];
    let b = Body::new(rest.clone(), vec![false; 4], vec![([0, 1, 2, 3], m)]).unwrap();
    let positions: Vec<_> = rest
        .iter()
        .map(|p| std::array::from_fn(|i| (0..3).map(|j| f[i][j] * p[j]).sum::<f64>() + 0.1))
        .collect();
    let e = b.stresses_at(&positions).unwrap()[0];
    close(e.reference_volume_m3, 0.02 * 0.03 * 0.04 / 6., 1e-18);
    for (x, y) in e
        .stress
        .cauchy_pa
        .iter()
        .flatten()
        .zip(s.cauchy_pa.iter().flatten())
    {
        close(*x, *y, 1e-8);
    }
    let mut inverted = rest.clone();
    inverted.swap(1, 2);
    assert!(b.stresses_at(&inverted).is_err());
    assert_eq!(b.positions(), rest);
}
#[test]
fn invalid_inputs_and_extreme_stresses_fail_explicitly() {
    for (e, nu) in [
        (0., 0.3),
        (1., 0.5),
        (1., -1.),
        (f64::INFINITY, 0.),
        (1., f64::NAN),
    ] {
        assert!(Material::from_young_poisson(e, nu).is_err());
    }
    assert!(Stress::from_cauchy([[0., 1., 0.], [0., 0., 0.], [0., 0., 0.]]).is_err());
    assert!(Stress::from_cauchy([[f64::NAN, 0., 0.], [0., 0., 0.], [0., 0., 0.]]).is_err());
    assert!(Stress::from_cauchy([[f64::MAX, 0., 0.], [0., -f64::MAX, 0.], [0., 0., 0.]]).is_err());
    let s = Stress::from_cauchy(IDENTITY).unwrap();
    for strength in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(s.yield_utilization(strength).is_err());
    }
}

#[test]
fn load_controlled_equilibrium_matches_force_balance_and_constrained_modulus() {
    // Single constant-strain tetra: lateral stretch is constrained by three pins.
    // Its effective axial area is V/L = L^2/6; it is not a rectangular bar.
    let length = 0.1;
    let load = 1.;
    let material = Material::from_young_poisson(2e6, 0.3).unwrap();
    let modulus = material.bulk_pa + 4. * material.shear_pa / 3.;
    let area = length * length / 6.;
    let mut body = Body::new(
        vec![
            [0.; 3],
            [length, 0., 0.],
            [0., length, 0.],
            [0., 0., length],
        ],
        vec![true, false, true, true],
        vec![([0, 1, 2, 3], material)],
    )
    .unwrap();
    body.set_force(1, [load, 0., 0.]).unwrap();
    let result = body.equilibrate(2000, 5e-6).unwrap();
    assert!(result.converged, "{result:?}");
    let extension = body.positions()[1][0] - length;
    let analytic = load * length / (area * modulus);
    assert!((extension / analytic - 1.).abs() < 5e-4);
    let stress = body.stresses_at(body.positions()).unwrap()[0].stress;
    close(stress.cauchy_pa[0][0], load / area, 0.003);
    let (_, gradient) = body.evaluate(body.positions()).unwrap();
    for axis in 0..3 {
        close(
            gradient.iter().map(|p| p[axis]).sum(),
            if axis == 0 { -load } else { 0. },
            1e-10,
        );
    }
}

#[test]
fn normal_strength_detects_hydrostatic_failure_and_asymmetric_strengths() {
    for (p, expected) in [(30., 1.5), (-30., 0.15), (0., 0.)] {
        let stress = Stress::from_cauchy(IDENTITY.map(|r| r.map(|v| v * p))).unwrap();
        close(stress.yield_utilization(20.).unwrap(), 0., 0.);
        close(
            stress.normal_strength_utilization(20., 200.).unwrap(),
            expected,
            1e-14,
        );
    }
    let sigma = [[0., 40., 0.], [40., 0., 0.], [0., 0., 0.]];
    let stress = Stress::from_cauchy(sigma).unwrap();
    close(
        stress.normal_strength_utilization(20., 200.).unwrap(),
        2.,
        1e-14,
    );
    let r = [[0.36, -0.8, 0.48], [0.48, 0.6, 0.64], [-0.8, 0., 0.6]];
    let rotated = Stress::from_cauchy(multiply(multiply(r, sigma), transpose(r))).unwrap();
    close(
        rotated.normal_strength_utilization(20., 200.).unwrap(),
        2.,
        1e-12,
    );
    for bad in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(stress.normal_strength_utilization(bad, 200.).is_err());
        assert!(stress.normal_strength_utilization(20., bad).is_err());
    }
    assert!(
        stress
            .normal_strength_utilization(f64::MIN_POSITIVE, 200.)
            .is_err()
    );
}

#[test]
fn principal_directions_reconstruct_stress_and_rotate_as_unoriented_axes() {
    let sigma = [[170., 30., -45.], [30., -80., 25.], [-45., 25., 110.]];
    let rotation = [[0.36, -0.8, 0.48], [0.48, 0.6, 0.64], [-0.8, 0., 0.6]];
    let original = Stress::from_cauchy(sigma).unwrap();
    let rotated =
        Stress::from_cauchy(multiply(multiply(rotation, sigma), transpose(rotation))).unwrap();
    for scale in [1e-200, 1., 1e200] {
        for input in [
            sigma,
            [[0., 40., 0.], [40., 0., 0.], [0., 0., 0.]],
            IDENTITY,
            [[0.; 3]; 3],
        ] {
            let stress = Stress::from_cauchy(input.map(|r| r.map(|v| v * scale))).unwrap();
            let axes = stress.principal_directions;
            let orthogonal = multiply(transpose(axes), axes);
            for i in 0..3 {
                for j in 0..3 {
                    close(orthogonal[i][j], IDENTITY[i][j], 1e-12);
                    let reconstructed: f64 = (0..3)
                        .map(|k| axes[i][k] * (stress.principal_pa[k] / scale) * axes[j][k])
                        .sum();
                    close(reconstructed, input[i][j], 1e-10);
                }
            }
        }
    }
    let expected = multiply(rotation, original.principal_directions);
    for k in 0..3 {
        let alignment: f64 = (0..3)
            .map(|i| expected[i][k] * rotated.principal_directions[i][k])
            .sum();
        close(alignment.abs(), 1., 1e-12);
    }
}
