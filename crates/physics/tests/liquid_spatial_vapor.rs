use physics::liquid::*;
fn interface() -> VaporInterface {
    VaporInterface {
        curve: SaturationCurve {
            reference_temperature: 10.,
            reference_pressure: 10.,
            latent_heat: 100.,
            vapor_gas_constant: 1.,
            min_temperature: 5.,
            max_temperature: 20.,
        },
        area: 0.1,
        accommodation: 0.01,
    }
}
fn fixture(shared: bool) -> (Liquid, FiniteDropletGasGrid) {
    let particles = [0.25, if shared { 0.75 } else { 1.25 }].map(|x| Particle {
        position: [x, 0.5, 0.5],
        velocity: [2., 0., 0.],
        mass: 1.,
        material: 0,
    });
    let mut liquid = Liquid::new(
        particles.to_vec(),
        vec![Material::WATER],
        Config {
            gravity: [0.; 3],
            ..Config::default()
        },
    )
    .unwrap();
    liquid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 10.,
                    concentration: 0.
                };
                2
            ],
            vec![TransportMaterial {
                specific_heat: 2.,
                conductivity: 0.,
                diffusivity: 0.,
                mixing_group: 0,
            }],
        )
        .unwrap();
    let gas = FiniteDropletGasGrid::new(
        [0.; 3],
        [1.; 3],
        [2, 1, 1],
        vec![
            VaporCell {
                mass: 0.1,
                volume: 1.,
                temperature: 10.,
                velocity: [-1., 1., 0.],
                specific_heat_cv: 1.
            };
            2
        ],
    )
    .unwrap();
    (liquid, gas)
}
fn totals(liquid: &Liquid, gas: &FiniteDropletGasGrid) -> (f64, [f64; 3], f64) {
    let g = gas.totals().unwrap();
    let mut momentum = g.momentum;
    let mut energy = g.thermal_energy
        + g.kinetic_energy
        + 100. * g.mass
        + liquid.transport_totals().unwrap().unwrap().0;
    for p in liquid.particles() {
        for k in 0..3 {
            momentum[k] += p.mass * p.velocity[k];
        }
        energy += 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>();
    }
    (liquid.mass() + g.mass, momentum, energy)
}
#[test]
fn spatial_evaporation_matches_ordered_pairs_and_conserves_global_inventory() {
    for shared in [false, true] {
        let (mut liquid, mut gas) = fixture(shared);
        let mut reference = liquid.clone();
        let mut cells = gas.cells().to_vec();
        let before = totals(&liquid, &gas);
        let mut transferred = 0.;
        for i in 0..2 {
            let cell = if shared { 0 } else { i };
            transferred += reference
                .exchange_vapor(
                    i,
                    &mut cells[cell],
                    interface(),
                    0.01,
                    VaporExchangeAccuracy::default(),
                )
                .unwrap();
        }
        let report = liquid
            .exchange_vapor_grid(
                &mut gas,
                &[(0, interface()), (1, interface())],
                0.01,
                VaporExchangeAccuracy::default(),
                2,
            )
            .unwrap();
        assert!(report.vapor_mass_change_kg > 0.);
        assert_eq!(report.vapor_mass_change_kg, transferred);
        assert_eq!(report.affected_cells, if shared { 1 } else { 2 });
        assert_eq!(format!("{liquid:?}"), format!("{reference:?}"));
        assert_eq!(gas.cells(), cells);
        let after = totals(&liquid, &gas);
        assert!((after.0 - before.0).abs() < 1e-12);
        for k in 0..3 {
            assert!((after.1[k] - before.1[k]).abs() < 1e-12);
        }
        assert!((after.2 - before.2).abs() < 1e-9);
    }
}
#[test]
fn late_thermodynamic_failure_and_duplicate_selection_preserve_every_owner() {
    let (mut liquid, mut gas) = fixture(false);
    let before = format!("{liquid:?}");
    let old_gas = gas.clone();
    let mut invalid = interface();
    invalid.curve.vapor_gas_constant = 2.;
    for selection in [
        vec![(0, interface()), (1, invalid)],
        vec![(0, interface()), (0, interface())],
    ] {
        assert!(
            liquid
                .exchange_vapor_grid(
                    &mut gas,
                    &selection,
                    0.01,
                    VaporExchangeAccuracy::default(),
                    2
                )
                .is_err()
        );
        assert_eq!(format!("{liquid:?}"), before);
        assert_eq!(gas, old_gas);
    }
    assert!(matches!(
        liquid.exchange_vapor_grid(
            &mut gas,
            &[(0, interface()), (1, interface())],
            0.01,
            VaporExchangeAccuracy::default(),
            1
        ),
        Err(Error::PairBudget)
    ));
    assert_eq!(format!("{liquid:?}"), before);
    assert_eq!(gas, old_gas);
}

#[test]
fn spatial_condensation_returns_mass_to_liquid_without_creating_energy() {
    let (mut liquid, _) = fixture(false);
    let mut gas = FiniteDropletGasGrid::new(
        [0.; 3],
        [1.; 3],
        [2, 1, 1],
        vec![
            VaporCell {
                mass: 1.2,
                volume: 1.,
                temperature: 10.,
                velocity: [-1., 1., 0.],
                specific_heat_cv: 1.
            };
            2
        ],
    )
    .unwrap();
    let before = totals(&liquid, &gas);
    let report = liquid
        .exchange_vapor_grid(
            &mut gas,
            &[(0, interface()), (1, interface())],
            0.01,
            VaporExchangeAccuracy::default(),
            2,
        )
        .unwrap();
    assert!(report.vapor_mass_change_kg < 0.);
    assert!(liquid.mass() > 2.);
    let after = totals(&liquid, &gas);
    assert!((after.0 - before.0).abs() < 1e-12);
    for k in 0..3 {
        assert!((after.1[k] - before.1[k]).abs() < 1e-12);
    }
    assert!((after.2 - before.2).abs() < 1e-9);
}

fn flow_control() -> GasGridFlowControl {
    GasGridFlowControl {
        gas_constant: 1.,
        boundaries: [GasGridBoundary::Periodic; 3],
        ..Default::default()
    }
}
fn heat_control() -> GasGridHeatControl {
    GasGridHeatControl {
        conductivity: 0.1,
        boundaries: [GasGridBoundary::Periodic; 3],
        ..Default::default()
    }
}
#[test]
fn transported_vapor_feedback_conserves_global_inventory_over_many_steps() {
    let (mut liquid, initial) = fixture(false);
    let mut cells = initial.cells().to_vec();
    cells[1].temperature = 12.;
    cells[1].mass = 0.2;
    let mut gas = FiniteDropletGasGrid::new([0.; 3], [1.; 3], [2, 1, 1], cells).unwrap();
    let before = totals(&liquid, &gas);
    let mut net = 0.;
    let mut max_energy = 0_f64;
    for _ in 0..50 {
        let (phase, flow, heat) = liquid
            .exchange_vapor_grid_with_transport(
                &mut gas,
                &[(0, interface()), (1, interface())],
                0.001,
                SpatialVaporTransportControl {
                    accuracy: VaporExchangeAccuracy::default(),
                    max_exchanges: 2,
                    flow: flow_control(),
                    heat: heat_control(),
                },
            )
            .unwrap();
        assert!(flow.face_updates > 0);
        assert!(heat.pair_steps > 0);
        net += phase.vapor_mass_change_kg;
        let after = totals(&liquid, &gas);
        assert!((after.0 - before.0).abs() < 1e-12);
        for k in 0..3 {
            assert!((after.1[k] - before.1[k]).abs() < 1e-12);
        }
        max_energy = max_energy.max((after.2 - before.2).abs());
        assert!(max_energy < 1e-9);
    }
    assert!(net > 0.);
    eprintln!(
        "SPATIAL_VAPOR_TRANSPORT steps=50 net_vapor_kg={net:.17e} max_energy_error_j={max_energy:.17e}"
    );
}
#[test]
fn late_phase_failure_rolls_back_prior_gas_transport_and_conduction() {
    let (mut liquid, mut gas) = fixture(false);
    let mut cells = gas.cells().to_vec();
    cells[1].temperature = 12.;
    cells[1].mass = 0.2;
    gas = FiniteDropletGasGrid::new([0.; 3], [1.; 3], [2, 1, 1], cells).unwrap();
    let before = format!("{liquid:?}");
    let old_gas = gas.clone();
    // Physical flow and conduction succeed; incompatible cp rejects during phase exchange.
    let mut bad = interface();
    bad.curve.vapor_gas_constant = 2.;
    let mut flow = flow_control();
    flow.gas_constant = 2.;
    let mut staged_gas = gas.clone();
    staged_gas.advance_euler(0.001, flow).unwrap();
    staged_gas.conduct_heat(0.001, heat_control()).unwrap();
    assert_ne!(
        staged_gas, old_gas,
        "the preceding gas stages must change physical state"
    );
    assert!(
        liquid
            .exchange_vapor_grid_with_transport(
                &mut gas,
                &[(0, bad)],
                0.001,
                SpatialVaporTransportControl {
                    accuracy: VaporExchangeAccuracy::default(),
                    max_exchanges: 2,
                    flow,
                    heat: heat_control()
                }
            )
            .is_err()
    );
    assert_eq!(format!("{liquid:?}"), before);
    assert_eq!(gas, old_gas);
}

fn transported_state(steps: usize) -> Vec<f64> {
    let (mut liquid, initial) = fixture(false);
    let mut cells = initial.cells().to_vec();
    cells[1].temperature = 12.;
    cells[1].mass = 0.2;
    let mut gas = FiniteDropletGasGrid::new([0.; 3], [1.; 3], [2, 1, 1], cells).unwrap();
    for _ in 0..steps {
        liquid
            .exchange_vapor_grid_with_transport(
                &mut gas,
                &[(0, interface()), (1, interface())],
                0.05 / steps as f64,
                SpatialVaporTransportControl {
                    accuracy: VaporExchangeAccuracy {
                        relative_tolerance: 1e-9,
                        mass_tolerance: 1e-14,
                        temperature_tolerance: 1e-9,
                        max_attempts: 4096,
                    },
                    max_exchanges: 2,
                    flow: flow_control(),
                    heat: heat_control(),
                },
            )
            .unwrap();
    }
    let mut state = Vec::new();
    for (p, field) in liquid.particles().iter().zip(liquid.fields().unwrap()) {
        state.push(p.mass);
        state.extend(p.velocity);
        state.push(field.temperature);
    }
    for cell in gas.cells() {
        state.push(cell.mass);
        state.extend(cell.velocity);
        state.push(cell.temperature);
    }
    state
}
#[test]
fn coupled_vapor_transport_refines_physical_state_with_time_partition() {
    let reference = transported_state(800);
    let mut errors = Vec::new();
    for steps in [25, 50, 100, 200] {
        let state = transported_state(steps);
        let error = state
            .iter()
            .zip(&reference)
            .map(|(a, b)| (a - b).abs() / b.abs().max(1.))
            .sum::<f64>();
        assert!(error.is_finite() && error > 0.);
        errors.push(error);
        eprintln!("VAPOR_TEMPORAL_REFINEMENT steps={steps} normalized_l1_state_error={error:.17e}");
    }
    for pair in errors.windows(2) {
        assert!(
            pair[1] < 0.7 * pair[0],
            "physical state must approach fine reference: {errors:?}"
        );
    }
}
