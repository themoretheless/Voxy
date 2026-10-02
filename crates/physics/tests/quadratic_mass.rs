use physics::plasticity::{Material, mesh::QuadraticBody};
fn body() -> QuadraticBody {
    QuadraticBody::new(
        vec![
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0.5, 0., 0.],
            [0.5, 0.5, 0.],
            [0., 0.5, 0.],
            [0., 0., 0.5],
            [0.5, 0., 0.5],
            [0., 0.5, 0.5],
        ],
        vec![(
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            Material::new(1e6, 0., 1e9, 0.).unwrap(),
        )],
    )
    .unwrap()
}
#[test]
fn consistent_mass_matches_independent_degree_four_quadrature_and_is_positive_definite() {
    let mass = body().consistent_mass(&[1000.]).unwrap();
    let gauss: [(f64, f64); 4] = [
        (0.06943184420297371, 0.17392742256872693),
        (0.33000947820757187, 0.32607257743127307),
        (0.6699905217924281, 0.32607257743127307),
        (0.9305681557970262, 0.17392742256872693),
    ];
    let mut integrated = [[0.; 10]; 10];
    for (r, wr) in gauss {
        for (s, ws) in gauss {
            for (t, wt) in gauss {
                let x = r;
                let y = (1. - r) * s;
                let z = (1. - r) * (1. - s) * t;
                let l = [1. - x - y - z, x, y, z];
                let mut shape = [0.; 10];
                for i in 0..4 {
                    shape[i] = l[i] * (2. * l[i] - 1.);
                }
                for (edge, (a, b)) in [(0, 1), (1, 2), (0, 2), (0, 3), (1, 3), (2, 3)]
                    .into_iter()
                    .enumerate()
                {
                    shape[4 + edge] = 4. * l[a] * l[b];
                }
                let weight = 1000. * wr * ws * wt * (1. - r).powi(2) * (1. - s);
                for i in 0..10 {
                    for j in 0..10 {
                        integrated[i][j] += weight * shape[i] * shape[j];
                    }
                }
            }
        }
    }
    for i in 0..10 {
        for j in 0..10 {
            assert!((mass[i][j] - integrated[i][j]).abs() < 1e-10);
        }
    }
    let total: f64 = mass.iter().flatten().sum();
    assert!((total - 1000. / 6.).abs() < 1e-10);
    // The matrix is SPD although corner row sums are negative. Naive row-sum
    // lumping is therefore inappropriate for this element.
    assert!(mass[0].iter().sum::<f64>() < 0.);
    let mut factor = vec![vec![0_f64; 10]; 10];
    for i in 0..10 {
        for j in 0..=i {
            let value = mass[i][j] - (0..j).map(|k| factor[i][k] * factor[j][k]).sum::<f64>();
            factor[i][j] = if i == j {
                assert!(value > 0.);
                value.sqrt()
            } else {
                value / factor[j][j]
            };
        }
    }
}
#[test]
fn affine_velocity_kinetic_energy_and_total_momentum_are_exact() {
    let body = body();
    let mass = body.consistent_mass(&[1000.]).unwrap();
    let v: Vec<_> = body.positions().iter().map(|p| p[0]).collect();
    let momentum: f64 = (0..10)
        .map(|i| (0..10).map(|j| mass[i][j] * v[j]).sum::<f64>())
        .sum();
    let kinetic: f64 = 0.5
        * (0..10)
            .map(|i| (0..10).map(|j| v[i] * mass[i][j] * v[j]).sum::<f64>())
            .sum::<f64>();
    assert!((momentum - 1000. / 24.).abs() < 1e-10);
    assert!((kinetic - 1000. / 120.).abs() < 1e-10);
    for density in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(body.consistent_mass(&[density]).is_err());
    }
    assert!(body.consistent_mass(&[]).is_err());
}

#[test]
fn cached_consistent_inertia_recovers_general_and_uniform_acceleration() {
    let body = body();
    let mass = body.consistent_mass(&[1000.]).unwrap();
    let inertia = body.consistent_inertia(&[1000.]).unwrap();
    for uniform in [false, true] {
        let expected: Vec<[f64; 3]> = body
            .positions()
            .iter()
            .map(|p| {
                if uniform {
                    [1., -9.81, 0.]
                } else {
                    [p[0] + 2. * p[1], p[2] - p[0], 3. * p[1] + 1.]
                }
            })
            .collect();
        let forces: Vec<[f64; 3]> = (0..10)
            .map(|i| {
                std::array::from_fn(|axis| (0..10).map(|j| mass[i][j] * expected[j][axis]).sum())
            })
            .collect();
        let actual = inertia.accelerations(&forces).unwrap();
        for (a, b) in actual.iter().flatten().zip(expected.iter().flatten()) {
            assert!((a - b).abs() < 1e-10);
        }
        // Reusing the factor must not modify it or accumulate solve state.
        assert_eq!(actual, inertia.accelerations(&forces).unwrap());
    }
    assert!(inertia.accelerations(&[[0.; 3]; 9]).is_err());
    assert!(inertia.accelerations(&[[f64::NAN; 3]; 10]).is_err());
}
