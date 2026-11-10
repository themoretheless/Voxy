use physics::biomechanics::*;
use physics::lymph::FluidSpace;
fn setup(solutes: usize, duplicate: bool) -> Result<PorePerfusion, &'static str> {
    setup_with_support(solutes, duplicate, false)
}
fn setup_with_support(
    solutes: usize,
    duplicate: bool,
    free: bool,
) -> Result<PorePerfusion, &'static str> {
    setup_with_solver(solutes, duplicate, free, 10000)
}
fn setup_with_solver(
    solutes: usize,
    duplicate: bool,
    free: bool,
    iterations: usize,
) -> Result<PorePerfusion, &'static str> {
    let mut body = Body::new(
        vec![[0., 0., 0.], [0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]],
        vec![true, true, true, !free],
        vec![([0, 1, 2, 3], Material::from_young_poisson(8000., 0.3)?)],
    )?;
    body.set_cell_pore_fluids(vec![PoreFluid {
        reference_fluid_volume_m3: 1e-7,
        fluid_volume_m3: 1e-7,
        storage_m3_per_pa: 1e-9,
        biot_coefficient: 0.8,
    }])?;
    let reservoir = FluidSpace {
        reference_volume_m3: 1e-6,
        initial_volume_m3: 1e-6,
        reference_pressure_pa: 100.,
        compliance_m3_per_pa: 1e-8,
        initial_protein_kg: 1e-5,
        oncotic_pa_per_kg_m3: 0.,
    };
    let port = PerfusionPort {
        tissue_cell: 0,
        reservoir: 0,
        reservoir_to_tissue: true,
        hydraulic_m3_per_pa_s: 1e-11,
        reflection: 0.,
        protein_permeability_m3_per_s: 0.,
    };
    PorePerfusion::new(
        body,
        vec![[[1e-12, 0., 0.], [0., 1e-12, 0.], [0., 0., 1e-12]]],
        0.001,
        vec![
            PerfusionCellSolute {
                protein_kg: 1e-6,
                oncotic_pa_per_kg_m3: 0.
            };
            solutes
        ],
        vec![reservoir],
        if duplicate {
            vec![port, port]
        } else {
            vec![port]
        },
        iterations,
        1e-11,
    )
}
#[test]
fn finite_reservoir_matches_closed_form_and_port_ledger() {
    let mut p = setup(1, false).unwrap();
    let initial = p.network().volumes().to_vec();
    let water = p.network().total_volume();
    let protein = p.network().total_protein();
    let report = p.step(0.1, 0.001).unwrap();
    // With every solid node fixed, two linear compliances have exact relaxation.
    let rate: f64 = 1e-11 * (1. / 1e-9 + 1. / 1e-8);
    let exact = 1e-11 * 100. / rate * (-(-rate * 0.1).exp_m1());
    let delta = p.network().volumes()[0] - initial[0];
    assert!((delta / exact - 1.).abs() < 6e-6);
    let mut fine = setup(1, false).unwrap();
    fine.step(0.1, 0.0005).unwrap();
    let fine_delta = fine.network().volumes()[0] - initial[0];
    let error_ratio = (delta - exact).abs() / (fine_delta - exact).abs();
    assert!((error_ratio - 2.).abs() < 0.01);

    assert!((delta - report.transferred_volume_m3[p.port_edges()[0]]).abs() < 1e-20);
    assert!((p.network().volumes()[1] - initial[1] + delta).abs() < 1e-20);
    assert!((p.network().total_volume() - water).abs() < 1e-20);
    assert!((p.network().total_protein() - protein).abs() < 1e-19);
    for (v, m) in p
        .network()
        .volumes()
        .iter()
        .zip(p.network().protein_masses())
    {
        assert!((m / v - 10.).abs() < 1e-10);
    }
}
#[test]
fn invalid_steps_and_store_maps_preserve_state() {
    assert!(setup(0, false).is_err());
    assert!(setup(2, false).is_err());
    assert!(setup(1, true).is_err());
    let mut p = setup(1, false).unwrap();
    let volumes = p.network().volumes().to_vec();
    let protein = p.network().protein_masses().to_vec();
    let positions = p.body().positions().to_vec();
    assert!(p.step(-1., 0.01).is_err());
    assert_eq!(volumes, p.network().volumes());
    assert_eq!(protein, p.network().protein_masses());
    assert_eq!(positions, p.body().positions());
}

#[test]
fn second_order_transport_converges_to_exact_inventory_and_conserves_ledger() {
    let start = setup(1, false).unwrap();
    let mut errors = Vec::new();
    let rate: f64 = 1e-11 * (1. / 1e-9 + 1. / 1e-8);
    let exact = 1e-11 * 100. / rate * (-(-rate * 10.).exp_m1());
    for step in [1., 0.5, 0.25] {
        let mut p = start.clone();
        let report = p.step_second_order(10., step).unwrap();
        let delta = p.network().volumes()[0] - start.network().volumes()[0];
        errors.push((delta - exact).abs());
        let amplification = 1. - rate * step + 0.5 * (rate * step).powi(2);
        let discrete = 1e-11 * 100. / rate * (1. - amplification.powi((10. / step) as i32));
        assert!((delta - discrete).abs() < 2e-20);

        assert!((delta - report.transferred_volume_m3[p.port_edges()[0]]).abs() < 1e-20);
        assert!((p.network().total_volume() - start.network().total_volume()).abs() < 1e-20);
        assert!((p.network().total_protein() - start.network().total_protein()).abs() < 1e-19);
    }
    for e in errors.windows(2) {
        assert!((e[0] / e[1] - 4.).abs() < 0.05, "errors={errors:?}");
    }
    assert!(errors[0] / exact < 3e-5);
}
#[test]
fn second_order_elastic_coupling_converges_with_deformation_and_rolls_back() {
    let start = setup_with_support(1, false, true).unwrap();
    let mut inventories = Vec::new();
    let mut heights = Vec::new();
    for step in [1., 0.5, 0.25] {
        let mut p = start.clone();
        p.step_second_order(10., step).unwrap();
        inventories.push(p.network().volumes()[0]);
        heights.push(p.body().positions()[3][2]);
        assert!(p.body().stresses_at(p.body().positions()).unwrap()[0].volume_ratio > 1.);
        assert_eq!(
            p.body().cell_pore_fluids()[0].fluid_volume_m3,
            p.network().volumes()[0]
        );
        let before = p.clone();
        assert!(p.step_second_order(1., 0.).is_err());
        assert_eq!(p.network().volumes(), before.network().volumes());
        assert_eq!(
            p.network().protein_masses(),
            before.network().protein_masses()
        );
        assert_eq!(p.body().positions(), before.body().positions());
    }
    for values in [inventories, heights] {
        let ratio = (values[0] - values[1]) / (values[1] - values[2]);
        assert!(
            (ratio - 4.).abs() < 0.15,
            "ratio={ratio}, values={values:?}"
        );
    }
}

#[test]
fn failed_second_order_predictor_preserves_body_and_transport() {
    let mut p = setup_with_solver(1, false, true, 1).unwrap();
    let before = p.clone();
    assert_eq!(
        p.step_second_order(10., 1.).unwrap_err(),
        "mixed pore FEM nonconvergence"
    );
    assert_eq!(p.body().positions(), before.body().positions());
    assert_eq!(
        p.body().cell_pore_fluids()[0].fluid_volume_m3,
        before.body().cell_pore_fluids()[0].fluid_volume_m3
    );
    assert_eq!(p.network().volumes(), before.network().volumes());
    assert_eq!(
        p.network().protein_masses(),
        before.network().protein_masses()
    );
    assert_eq!(p.network().pressures(), before.network().pressures());
}

fn adaptive_config() -> AdaptiveTissueExchangeConfig {
    AdaptiveTissueExchangeConfig {
        exchange: physics::lymph::AdaptiveExchangeConfig {
            relative_tolerance: 0.,
            absolute_volume_tolerance_m3: 1e-14,
            absolute_protein_tolerance_kg: 1e-13,
            min_step_seconds: 1e-6,
            max_step_seconds: 10.,
            max_trials: 1000,
        },
        absolute_position_tolerance_m: 1e-10,
    }
}
#[test]
fn adaptive_perfusion_rejects_coarse_steps_and_preserves_joint_ledger() {
    let mut p = setup_with_support(1, false, true).unwrap();
    let initial = p.clone();
    let report = p.step_adaptive(10., adaptive_config()).unwrap();
    assert!(report.accepted_steps > 1 && report.rejected_steps > 0);
    assert!(report.max_accepted_error_ratio <= 1.);
    let delta = p.network().volumes()[0] - initial.network().volumes()[0];
    assert!((delta - report.exchange.transferred_volume_m3[p.port_edges()[0]]).abs() < 1e-20);
    assert!((p.network().total_volume() - initial.network().total_volume()).abs() < 1e-20);
    assert!((p.network().total_protein() - initial.network().total_protein()).abs() < 1e-19);
    let mut reference = initial.clone();
    reference.step_second_order(10., 0.025).unwrap();
    assert!((p.network().volumes()[0] - reference.network().volumes()[0]).abs() < 5e-13);
    assert!((p.body().positions()[3][2] - reference.body().positions()[3][2]).abs() < 5e-9);
}
#[test]
fn adaptive_perfusion_budget_failure_rolls_back_complete_interval() {
    let mut p = setup_with_support(1, false, true).unwrap();
    let before = p.clone();
    let mut config = adaptive_config();
    config.exchange.max_step_seconds = 0.01;
    config.exchange.max_trials = 1;
    assert!(p.step_adaptive(10., config).is_err());
    assert_eq!(p.body().positions(), before.body().positions());
    assert_eq!(p.network().volumes(), before.network().volumes());
    assert_eq!(
        p.network().protein_masses(),
        before.network().protein_masses()
    );
    assert_eq!(p.network().pressures(), before.network().pressures());
}
