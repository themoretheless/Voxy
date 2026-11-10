use physics::biomechanics::*;
fn specimen() -> Body {
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
fn external_follower_pressure_has_consistent_energy_force_and_recovers() {
    let mut body = specimen();
    let plain = body.clone();
    body.add_gauge_pressure_cavity(Cavity {
        faces: body.surface(),
        pressure_pa: -20.,
    })
    .unwrap();
    let x = body.positions().to_vec();
    let (e, g) = body.evaluate(&x).unwrap();
    let (base, base_g) = plain.evaluate(&x).unwrap();
    assert!((e - base - 20. * 1e-6 / 6.).abs() < 1e-15);
    for i in 0..x.len() {
        for k in 0..3 {
            let mut plus = x.clone();
            let mut minus = x.clone();
            let h = 1e-7;
            plus[i][k] += h;
            minus[i][k] -= h;
            let ep = body.evaluate(&plus).unwrap().0 - plain.evaluate(&plus).unwrap().0;
            let em = body.evaluate(&minus).unwrap().0 - plain.evaluate(&minus).unwrap().0;
            assert!(((ep - em) / (2. * h) - (g[i][k] - base_g[i][k])).abs() < 1e-10);
        }
    }
    assert!(body.equilibrate_lbfgs(10000, 1e-10).unwrap().converged);
    assert!(body.positions()[3][2] < x[3][2]);
    body.set_gauge_pressure(0, 0.).unwrap();
    assert!(body.equilibrate_lbfgs(10000, 1e-10).unwrap().converged);
    assert!((body.positions()[3][2] - x[3][2]).abs() < 1e-8);
    assert!(body.set_pressure(0, -20.).is_err());
    assert!(body.set_gauge_pressure(0, f64::NAN).is_err());
    let before = body.cavities().len();
    assert!(
        body.add_gauge_pressure_cavity(Cavity {
            faces: vec![[0, 1, 2]],
            pressure_pa: -20.
        })
        .is_err()
    );
    assert_eq!(body.cavities().len(), before);
}

#[test]
fn signed_pressure_survives_assembly_and_has_no_internal_resultant() {
    let mut compressed = specimen();
    compressed
        .add_gauge_pressure_cavity(Cavity {
            faces: compressed.surface(),
            pressure_pa: -37.,
        })
        .unwrap();
    let plain = specimen();
    let assembly = Body::assemble_tissues(&[compressed.clone(), plain.clone()]).unwrap();
    assert_eq!(assembly.body.cavities()[0].pressure_pa, -37.);
    let (ea, ga) = assembly.body.evaluate(assembly.body.positions()).unwrap();
    let (ec, gc) = compressed.evaluate(compressed.positions()).unwrap();
    let (ep, gp) = plain.evaluate(plain.positions()).unwrap();
    assert!((ea - ec - ep).abs() < 1e-15);
    for (actual, expected) in ga.iter().zip(gc.iter().chain(&gp)) {
        for k in 0..3 {
            assert!((actual[k] - expected[k]).abs() < 1e-14);
        }
    }
    let mut deformed = compressed.positions().to_vec();
    deformed[3] = [0.001, 0.002, 0.008];
    let (_, loaded) = compressed.evaluate(&deformed).unwrap();
    let (_, unloaded) = plain.evaluate(&deformed).unwrap();
    let mut net = [0.; 3];
    let mut torque = [0.; 3];
    for ((x, a), b) in deformed.iter().zip(loaded).zip(unloaded) {
        let force: [f64; 3] = std::array::from_fn(|k| b[k] - a[k]);
        for k in 0..3 {
            net[k] += force[k];
        }
        torque[0] += x[1] * force[2] - x[2] * force[1];
        torque[1] += x[2] * force[0] - x[0] * force[2];
        torque[2] += x[0] * force[1] - x[1] * force[0];
    }
    assert!(net.iter().all(|v| v.abs() < 1e-13));
    assert!(torque.iter().all(|v| v.abs() < 1e-15));
}
