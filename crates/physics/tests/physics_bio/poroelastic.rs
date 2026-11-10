use physics::{biomechanics::*, lymph::*};
fn specimen() -> Body {
    let points = vec![[0., 0., 0.], [0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]];
    let material = Material {
        shear_pa: 1000.,
        bulk_pa: 10_000.,
        fibers: vec![],
    };
    let mut body = Body::new(
        points,
        vec![true, true, true, false],
        vec![([0, 1, 2, 3], material)],
    )
    .unwrap();
    let fluid = body.reference_volume() * 0.5;
    body.set_pore_fluid(PoreFluid {
        reference_fluid_volume_m3: fluid,
        fluid_volume_m3: fluid,
        biot_coefficient: 0.8,
        storage_m3_per_pa: 1e-11,
    })
    .unwrap();
    body
}
#[test]
fn pore_energy_gradient_and_total_cauchy_stress_agree() {
    let mut body = specimen();
    let mut pore = body.pore_fluid().unwrap();
    pore.fluid_volume_m3 += 1e-9;
    body.set_pore_fluid(pore).unwrap();
    let mut x = body.positions().to_vec();
    x[3] = [0.001, 0.0003, 0.0101];
    let (_, gradient) = body.evaluate(&x).unwrap();
    for i in 0..4 {
        for j in 0..3 {
            let mut a = x.clone();
            let mut b = x.clone();
            a[i][j] += 1e-8;
            b[i][j] -= 1e-8;
            let fd = (body.evaluate(&a).unwrap().0 - body.evaluate(&b).unwrap().0) / 2e-8;
            assert!(
                (fd - gradient[i][j]).abs() < 1e-7,
                "{i},{j}: {fd} {}",
                gradient[i][j]
            );
        }
    }
    let rest = body.rest_positions();
    let p = body.pore_response_at(rest).unwrap().0;
    let stress = body.stresses_at(rest).unwrap()[0].stress.cauchy_pa;
    for i in 0..3 {
        for j in 0..3 {
            assert!((stress[i][j] - if i == j { -0.8 * p } else { 0. }).abs() < 1e-9);
        }
    }
}
#[test]
fn retained_fluid_swells_tissue_and_satisfies_storage_balance() {
    let mut body = specimen();
    let mut pore = body.pore_fluid().unwrap();
    pore.fluid_volume_m3 += 1e-9;
    body.set_pore_fluid(pore).unwrap();
    let before = body.pore_response_at(body.positions()).unwrap().0;
    let report = body.equilibrate(4000, 1e-9).unwrap();
    assert!(report.converged, "{report:?}");
    let v = body.volume_at(body.positions()).unwrap();
    let p = body.pore_response_at(body.positions()).unwrap().0;
    assert!(v > body.reference_volume() && p > 0. && p < before);
    let content = 0.8 * (v - body.reference_volume()) + 1e-11 * p;
    assert!((content - 1e-9).abs() < 1e-20);
}
fn network(body: &Body) -> LymphNetwork {
    let v = body.pore_fluid().unwrap().fluid_volume_m3;
    let space = |volume, p| FluidSpace {
        reference_volume_m3: volume,
        initial_volume_m3: volume,
        initial_protein_kg: volume * 10.,
        reference_pressure_pa: p,
        compliance_m3_per_pa: 1e-11,
        oncotic_pa_per_kg_m3: 0.,
    };
    LymphNetwork::new(
        vec![space(1e-6, 100.), space(v, 0.)],
        vec![Exchange {
            from: 0,
            to: 1,
            hydraulic_m3_per_pa_s: 1e-12,
            reflection: 0.,
            protein_permeability_m3_per_s: 0.,
            pump_head_pa: 0.,
            valve: false,
        }],
    )
    .unwrap()
}
#[test]
fn fluid_exchange_and_deformation_are_bidirectionally_coupled() {
    let body = specimen();
    let mut net = network(&body);
    let total = net.total_volume();
    let protein = net.total_protein();
    let mut tissue = PoreTissue::new(body, 1, 4000, 1e-9).unwrap();
    tissue.step(&mut net, 0.2, 0.02).unwrap();
    let body = tissue.body();
    let pore = body.pore_fluid().unwrap();
    assert!(body.volume_at(body.positions()).unwrap() > body.reference_volume());
    assert_eq!(pore.fluid_volume_m3, net.volumes()[1]);
    assert_eq!(
        net.pressures()[1],
        body.pore_response_at(body.positions()).unwrap().0
    );
    assert!(net.pressures()[1] > 0.);
    assert!((net.total_volume() - total).abs() < 1e-20);
    assert!((net.total_protein() - protein).abs() < 1e-19);
    // Solid compliance absorbs fluid: pressure is below the fixed-geometry storage value.
    assert!(
        net.pressures()[1]
            < (pore.fluid_volume_m3 - pore.reference_fluid_volume_m3) / pore.storage_m3_per_pa
    );
}
#[test]
fn coupled_failure_and_bad_pressure_callback_roll_back_everything() {
    let body = specimen();
    let mut net = network(&body);
    let mut tissue = PoreTissue::new(body, 1, 1, 1e-15).unwrap();
    let x = tissue.body().positions().to_vec();
    let v = net.volumes().to_vec();
    let m = net.protein_masses().to_vec();
    assert!(tissue.step(&mut net, 1., 0.1).is_err());
    assert_eq!(tissue.body().positions(), x);
    assert_eq!(net.volumes(), v);
    assert_eq!(net.protein_masses(), m);
    assert!(
        net.step_with_pressure_law(1., 0.1, |_, _| Ok(vec![0.]))
            .is_err()
    );
    assert_eq!(net.volumes(), v);
}
