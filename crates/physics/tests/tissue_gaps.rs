use physics::biomechanics::*;
fn tetra(offset: f64, pinned: bool) -> Body {
    Body::new(
        vec![
            [offset, 0., 0.],
            [offset + 0.01, 0., 0.],
            [offset, 0.01, 0.],
            [offset, 0., 0.01],
        ],
        vec![pinned, pinned, pinned, false],
        vec![(
            [0, 1, 2, 3],
            Material::from_young_poisson(8000., 0.3).unwrap(),
        )],
    )
    .unwrap()
}
fn gap() -> TissueGap {
    TissueGap {
        nodes: [0, 3],
        minimum_distance_m: 0.003,
        activation_gap_m: 0.01,
        stiffness_n_m: 10.,
    }
}
#[test]
fn barrier_energy_gradient_is_objective_and_internal_forces_balance() {
    let plain = tetra(0., false);
    let mut body = plain.clone();
    body.add_tissue_gaps(&[gap()]).unwrap();
    let mut x = body.positions().to_vec();
    x[3][2] = 0.008;
    let (e, g) = body.evaluate(&x).unwrap();
    let (e0, g0) = plain.evaluate(&x).unwrap();
    let clearance = 0.008 - 0.003;
    let offset: f64 = clearance - 0.01;
    let expected = -10. * offset.powi(2) * (clearance / 0.01).ln();
    assert!((e - e0 - expected).abs() < 1e-15);
    for i in 0..4 {
        for k in 0..3 {
            let mut xp = x.clone();
            let mut xm = x.clone();
            xp[i][k] += 1e-8;
            xm[i][k] -= 1e-8;
            let fd = (body.evaluate(&xp).unwrap().0 - body.evaluate(&xm).unwrap().0) / 2e-8;
            assert!((fd - g[i][k]).abs() < 1e-7);
        }
    }
    for k in 0..3 {
        assert!((0..4).map(|i| g[i][k] - g0[i][k]).sum::<f64>().abs() < 1e-14);
    }
    let transformed: Vec<_> = x
        .iter()
        .map(|p| [-p[1] + 0.1, p[0] - 0.2, p[2] + 0.05])
        .collect();
    let difference =
        body.evaluate(&transformed).unwrap().0 - plain.evaluate(&transformed).unwrap().0;
    assert!((difference - expected).abs() < 1e-14);
    assert!(g[3][2] - g0[3][2] < 0.); // repulsive force is minus gradient
}
#[test]
fn gaps_install_atomically_and_survive_assembly() {
    let mut body = tetra(0., true);
    body.add_tissue_gaps(&[gap()]).unwrap();
    let energy = body.evaluate(body.positions()).unwrap().0;
    assert!(body.add_tissue_gaps(&[gap()]).is_err());
    assert_eq!(body.tissue_gaps().len(), 1);
    assert_eq!(body.evaluate(body.positions()).unwrap().0, energy);
    let combined = Body::assemble_tissues(&[body.clone(), body]).unwrap().body;
    assert_eq!(combined.tissue_gaps()[1].nodes, [4, 7]);
    assert!((combined.evaluate(combined.positions()).unwrap().0 - 2. * energy).abs() < 1e-14);
}
#[test]
fn compressed_equilibrium_keeps_clearance_and_resists_loading() {
    let mut plain = tetra(0., true);
    plain.set_force(3, [0., 0., -0.01]).unwrap();
    let mut body = plain.clone();
    body.add_tissue_gaps(&[gap()]).unwrap();
    assert!(plain.equilibrate(100000, 1e-9).unwrap().converged);
    let report = body.equilibrate(100000, 1e-9).unwrap();
    assert!(report.converged && report.min_j > 0.);
    assert!(body.positions()[3][2] > plain.positions()[3][2]);
    assert!(body.positions()[3][2] > gap().minimum_distance_m);
}
#[test]
fn inertial_path_cannot_jump_through_a_selected_pair() {
    let mut body = Body::assemble_tissues(&[tetra(0., false), tetra(0.03, false)])
        .unwrap()
        .body;
    body.add_tissue_gaps(&[TissueGap {
        nodes: [3, 7],
        minimum_distance_m: 0.005,
        activation_gap_m: 0.001,
        stiffness_n_m: 10.,
    }])
    .unwrap();
    let velocity = (0..8)
        .map(|i| [if i < 4 { 0.03 } else { -0.03 }, 0., 0.])
        .collect();
    let mut inertia = InertialBody::new(body, &[1000.; 2], velocity).unwrap();
    let positions = inertia.body().positions().to_vec();
    let velocities = inertia.velocities().to_vec();
    assert_eq!(
        inertia.step(1., 1.).unwrap_err(),
        "inertial tissue gap path crossing"
    );
    assert_eq!(positions, inertia.body().positions());
    assert_eq!(velocities, inertia.velocities());
}

#[test]
fn removal_of_compression_recovers_reference_without_resetting_contact() {
    let mut body = tetra(0., true);
    let reference = body.positions().to_vec();
    body.add_tissue_gaps(&[TissueGap {
        activation_gap_m: 0.0069,
        ..gap()
    }])
    .unwrap();
    body.set_force(3, [0., 0., -0.03]).unwrap();
    assert!(body.equilibrate(100000, 1e-10).unwrap().converged);
    assert!(body.positions()[3][2] < reference[3][2]);
    body.set_force(3, [0.; 3]).unwrap();
    assert!(body.equilibrate(100000, 1e-10).unwrap().converged);
    for (a, b) in body.positions().iter().zip(reference) {
        for k in 0..3 {
            assert!((a[k] - b[k]).abs() < 1e-8);
        }
    }
    assert_eq!(body.tissue_gaps().len(), 1);
}
