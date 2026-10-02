use physics::{biomechanics::*, lymph::*};
fn specimen(free: bool) -> Body {
    let x = vec![
        [0., 0., 0.],
        [0.01, 0., 0.],
        [0., 0.01, 0.],
        [0.01 / 3., 0.01 / 3., 0.01],
        [0.01 / 3., 0.01 / 3., -0.01],
    ];
    let m = Material {
        shear_pa: 1000.,
        bulk_pa: 10_000.,
        fibers: vec![],
    };
    let mut body = Body::new(
        x,
        vec![true, true, true, !free, !free],
        vec![([0, 1, 2, 3], m.clone()), ([0, 1, 2, 4], m)],
    )
    .unwrap();
    let vf = body.reference_volume() / 4.;
    let store = |extra| PoreFluid {
        reference_fluid_volume_m3: vf,
        fluid_volume_m3: vf + extra,
        biot_coefficient: 0.8,
        storage_m3_per_pa: 1e-11,
    };
    body.set_cell_pore_fluids(vec![store(1e-9), store(0.)])
        .unwrap();
    body
}
fn network(body: &Body) -> LymphNetwork {
    let spaces = body
        .cell_pore_fluids()
        .iter()
        .map(|p| FluidSpace {
            reference_volume_m3: p.reference_fluid_volume_m3,
            initial_volume_m3: p.fluid_volume_m3,
            initial_protein_kg: 10. * p.fluid_volume_m3,
            reference_pressure_pa: 0.,
            compliance_m3_per_pa: p.storage_m3_per_pa,
            oncotic_pa_per_kg_m3: 0.,
        })
        .collect();
    let e = body
        .reference_darcy_interface(0, 1, [1e-12; 2], 0.001)
        .unwrap();
    LymphNetwork::new(spaces, vec![e]).unwrap()
}
#[test]
fn local_pore_energy_gradient_and_total_stress() {
    let body = specimen(true);
    let mut x = body.positions().to_vec();
    x[3][2] += 1e-4;
    x[4][0] += 2e-4;
    let (_, g) = body.evaluate(&x).unwrap();
    for i in 0..x.len() {
        for j in 0..3 {
            let mut a = x.clone();
            let mut b = x.clone();
            a[i][j] += 1e-8;
            b[i][j] -= 1e-8;
            let fd = (body.evaluate(&a).unwrap().0 - body.evaluate(&b).unwrap().0) / 2e-8;
            assert!((fd - g[i][j]).abs() < 1e-7, "{i},{j}");
        }
    }
    let stresses = body.stresses_at(body.rest_positions()).unwrap();
    assert!((stresses[0].stress.cauchy_pa[0][0] + 80.).abs() < 1e-9);
    assert!(stresses[1].stress.cauchy_pa[0][0].abs() < 1e-9);
    assert!(body.pore_response_at(body.positions()).is_err());
}
#[test]
fn darcy_half_cell_resistances_geometry_and_scaling() {
    let body = specimen(false);
    let e = body
        .reference_darcy_interface(0, 1, [1e-12; 2], 0.001)
        .unwrap();
    assert!((e.hydraulic_m3_per_pa_s / 1e-11 - 1.).abs() < 1e-12);
    let half = body
        .reference_darcy_interface(0, 1, [1e-12, 1e-12 / 3.], 0.001)
        .unwrap();
    assert!((half.hydraulic_m3_per_pa_s / e.hydraulic_m3_per_pa_s - 0.5).abs() < 1e-12);
    let blocked = body
        .reference_darcy_interface(0, 1, [0., 1e-12], 0.001)
        .unwrap();
    assert_eq!(blocked.hydraulic_m3_per_pa_s, 0.);
    assert!(
        body.reference_darcy_interface(0, 0, [1e-12; 2], 0.001)
            .is_err()
    );
    let mut x = body.rest_positions().to_vec();
    x[3][0] += 0.001;
    let m = Material {
        shear_pa: 1000.,
        bulk_pa: 10_000.,
        fibers: vec![],
    };
    let oblique = Body::new(
        x,
        vec![true; 5],
        vec![([0, 1, 2, 3], m.clone()), ([0, 1, 2, 4], m)],
    )
    .unwrap();
    assert!(
        oblique
            .reference_darcy_interface(0, 1, [1e-12; 2], 0.001)
            .is_err()
    );
}
#[test]
fn fixed_skeleton_diffusion_matches_discrete_and_continuous_limits() {
    fn run(h: f64) -> f64 {
        let body = specimen(false);
        let mut net = network(&body);
        let v = net.total_volume();
        let m = net.total_protein();
        let mut tissue = CellPoreTissue::new(body, vec![0, 1], 4000, 1e-9).unwrap();
        tissue.step(&mut net, 1., h).unwrap();
        let p = net.pressures();
        let difference = p[0] - p[1];
        let discrete = 100. * (1. - 2. * h).powf((1. / h).round());
        assert!((difference - discrete).abs() < 1e-8);
        assert!((net.total_volume() - v).abs() < 1e-20);
        assert!((net.total_protein() - m).abs() < 1e-18);
        (difference - 100. * (-2_f64).exp()).abs()
    }
    assert!(run(0.001) < run(0.01) * 0.11);
}
#[test]
fn spatial_pressure_transfer_changes_local_geometry_and_keeps_inventory() {
    let body = specimen(true);
    let mut net = network(&body);
    let total = net.total_volume();
    let initial = net.volumes().to_vec();
    let mut tissue = CellPoreTissue::new(body, vec![0, 1], 4000, 1e-9).unwrap();
    let report = tissue.step(&mut net, 0.2, 0.01).unwrap();
    assert!(report.transferred_volume_m3[0] > 0.);
    assert!(net.volumes()[0] < initial[0] && net.volumes()[1] > initial[1]);
    assert!(net.pressures()[0] > net.pressures()[1] && net.pressures()[1] > 0.);
    let body = tissue.body();
    assert!(body.positions()[3][2] > 0.01 && body.positions()[4][2] < -0.01);
    let (p, _) = body.cell_pore_response_at(body.positions()).unwrap();
    assert_eq!(p, net.pressures());
    for (f, v) in body.cell_pore_fluids().iter().zip(net.volumes()) {
        assert_eq!(f.fluid_volume_m3, *v);
    }
    assert!((net.total_volume() - total).abs() < 1e-20);
}
#[test]
fn invalid_assignment_and_failed_coupling_preserve_cell_state() {
    let mut body = specimen(true);
    let old = body.cell_pore_fluids().to_vec();
    assert!(body.set_cell_pore_fluids(vec![old[0]]).is_err());
    assert_eq!(body.cell_pore_fluids().len(), 2);
    assert!(CellPoreTissue::new(body.clone(), vec![0, 0], 4000, 1e-9).is_err());
    let mut net = network(&body);
    let v = net.volumes().to_vec();
    let x = body.positions().to_vec();
    let mut tissue = CellPoreTissue::new(body, vec![0, 1], 1, 1e-15).unwrap();
    assert!(tissue.step(&mut net, 1., 0.1).is_err());
    assert_eq!(net.volumes(), v);
    assert_eq!(tissue.body().positions(), x);
    for (a, b) in old.iter().zip(tissue.body().cell_pore_fluids()) {
        assert_eq!(a.fluid_volume_m3, b.fluid_volume_m3);
    }
}
