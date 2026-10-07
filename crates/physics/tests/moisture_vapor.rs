use physics::moisture::{Body, Cell, VaporLink, VaporReservoir};
#[test]
fn closed_drying_and_condensation_match_two_capacity_implicit_solution() {
    for (material, vapor_mass) in [(1., 0.), (0., 0.5)] {
        let mut body = Body::new(
            vec![Cell {
                capacity_kg: 1.,
                water_kg: material,
            }],
            vec![],
        )
        .unwrap();
        let mut vapor = VaporReservoir::new(0.5, vapor_mass, 2e6, 3e6).unwrap();
        let energy = vapor.accounted_energy_j();
        let mass = material + vapor_mass;
        let dt = 0.2;
        let g = 0.1;
        let expected = dt * g * (material - vapor_mass / 0.5) / (1. + dt * g * (1. + 1. / 0.5));
        let report = body
            .advance_vapor(
                dt,
                &mut vapor,
                &[VaporLink {
                    material_cell: 0,
                    conductance_kg_s: g,
                }],
            )
            .unwrap();
        assert!((report.vapor_water_change_kg - expected).abs() < 1e-14);
        assert!((body.cells()[0].water_kg + vapor.water_kg() - mass).abs() < 1e-14);
        assert!((vapor.accounted_energy_j() - energy).abs() < 1e-8);
        assert!((report.latent_exchange_j - 2e6 * expected).abs() < 1e-8);
        assert!((vapor.thermal_j() - (3e6 - 2e6 * expected)).abs() < 1e-8);
    }
}
#[test]
fn insufficient_heat_and_invalid_links_leave_both_owners_unchanged() {
    let mut body = Body::new(
        vec![Cell {
            capacity_kg: 1.,
            water_kg: 1.,
        }],
        vec![],
    )
    .unwrap();
    let mut vapor = VaporReservoir::new(0.5, 0., 2e6, 0.).unwrap();
    let link = VaporLink {
        material_cell: 0,
        conductance_kg_s: 0.1,
    };
    assert!(body.advance_vapor(1., &mut vapor, &[link]).is_err());
    assert_eq!(body.cells()[0].water_kg, 1.);
    assert_eq!(vapor.water_kg(), 0.);
    assert_eq!(vapor.thermal_j(), 0.);
    assert!(body.advance_vapor(1., &mut vapor, &[link, link]).is_err());
    assert_eq!(body.cells()[0].water_kg, 1.);
}
#[test]
fn long_closed_exchange_equilibrates_without_losing_water_or_latent_energy() {
    let mut body = Body::new(
        vec![Cell {
            capacity_kg: 1.,
            water_kg: 1.,
        }],
        vec![],
    )
    .unwrap();
    let mut vapor = VaporReservoir::new(0.5, 0., 2e6, 3e6).unwrap();
    let energy = vapor.accounted_energy_j();
    for _ in 0..1000 {
        body.advance_vapor(
            0.1,
            &mut vapor,
            &[VaporLink {
                material_cell: 0,
                conductance_kg_s: 0.1,
            }],
        )
        .unwrap();
    }
    assert!((body.cells()[0].water_kg + vapor.water_kg() - 1.).abs() < 1e-12);
    assert!((body.cells()[0].water_kg - vapor.activity()).abs() < 1e-12);
    assert!((vapor.accounted_energy_j() - energy).abs() < 1e-6);
}

#[test]
fn thermal_exchange_cools_or_heats_and_domain_failure_is_atomic() {
    use physics::{liquid::SaturationCurve, moisture::ThermalVapor};
    let curve = SaturationCurve {
        reference_temperature: 300.,
        reference_pressure: 3500.,
        latent_heat: 2.4e6,
        vapor_gas_constant: 461.,
        min_temperature: 280.,
        max_temperature: 320.,
    };
    for (material, gas_mass) in [(0.01, 0.), (0., 0.01)] {
        let mut body = Body::new(
            vec![Cell {
                capacity_kg: 0.01,
                water_kg: material,
            }],
            vec![],
        )
        .unwrap();
        let mut gas = ThermalVapor::new(300., 1000., 1., gas_mass, curve).unwrap();
        let energy = gas.accounted_energy_j();
        let transfer = body
            .advance_thermal_vapor(
                0.1,
                &mut gas,
                &[VaporLink {
                    material_cell: 0,
                    conductance_kg_s: 0.001,
                }],
            )
            .unwrap();
        assert!((gas.temperature_k() - (300. - transfer.latent_exchange_j / 1000.)).abs() < 1e-12);
        assert!((gas.accounted_energy_j() - energy).abs() < 1e-8);
        assert!((body.cells()[0].water_kg + gas.water_kg() - material - gas_mass).abs() < 1e-14);
        assert_eq!(gas.temperature_k() < 300., material > 0.);
    }
    let mut body = Body::new(
        vec![Cell {
            capacity_kg: 0.01,
            water_kg: 0.01,
        }],
        vec![],
    )
    .unwrap();
    let mut gas = ThermalVapor::new(280., 1000., 1., 0., curve).unwrap();
    let before = format!("{body:?}{gas:?}");
    assert!(
        body.advance_thermal_vapor(
            1.,
            &mut gas,
            &[VaporLink {
                material_cell: 0,
                conductance_kg_s: 0.001
            }]
        )
        .is_err()
    );
    assert_eq!(before, format!("{body:?}{gas:?}"));
}

#[test]
fn material_vapor_conduction_matches_exact_two_store_solution() {
    use physics::{
        liquid::SaturationCurve,
        moisture::{MaterialThermalStore, ThermalVapor},
    };
    let curve = SaturationCurve {
        reference_temperature: 300.,
        reference_pressure: 3500.,
        latent_heat: 2.4e6,
        vapor_gas_constant: 461.,
        min_temperature: 280.,
        max_temperature: 320.,
    };
    let mut gas = ThermalVapor::new(290., 1000., 1., 0.001, curve).unwrap();
    let mut solid = MaterialThermalStore::new(2000., 310.).unwrap();
    let before = gas.accounted_energy_j() + solid.energy_j();
    let expected = (20. / (1. / 1000. + 1. / 2000.)) * (1. - (-0.3_f64).exp());
    let heat = gas.exchange_material_heat(2., &mut solid, 100.).unwrap();
    assert!((heat - expected).abs() < 1e-9);
    assert!((gas.temperature_k() - (290. + heat / 1000.)).abs() < 1e-12);
    assert!((solid.temperature_k() - (310. - heat / 2000.)).abs() < 1e-12);
    assert!((gas.accounted_energy_j() + solid.energy_j() - before).abs() < 1e-8);
    let snapshot = format!("{gas:?}{solid:?}");
    assert!(gas.exchange_material_heat(1., &mut solid, -1.).is_err());
    assert_eq!(snapshot, format!("{gas:?}{solid:?}"));
}

#[test]
fn closed_enthalpy_transfer_changes_heat_capacities_and_preserves_energy() {
    use physics::{
        liquid::SaturationCurve,
        moisture::{MaterialThermalStore, ThermalVapor},
    };
    let curve = SaturationCurve {
        reference_temperature: 300.,
        reference_pressure: 3500.,
        latent_heat: 2.4e6,
        vapor_gas_constant: 461.,
        min_temperature: 280.,
        max_temperature: 320.,
    };
    for (water, mv) in [(0.01, 0.), (0., 0.01)] {
        let mut body = Body::new(
            vec![Cell {
                capacity_kg: 0.01,
                water_kg: water,
            }],
            vec![],
        )
        .unwrap();
        let mut gas = ThermalVapor::new(300., 1000., 1., mv, curve).unwrap();
        let mut material = MaterialThermalStore::new(2000., 300.).unwrap();
        let before = gas.accounted_energy_j() + material.energy_j();
        let r = body
            .advance_enthalpy_vapor(
                0.1,
                &mut gas,
                &mut material,
                4200.,
                &[VaporLink {
                    material_cell: 0,
                    conductance_kg_s: 0.001,
                }],
            )
            .unwrap();
        let dm = r.vapor_water_change_kg;
        assert!((gas.temperature_k() - 300.).abs() < 1e-12);
        assert!(
            (material.temperature_k()
                - (600000. - dm * (4200. * 300. + 2.4e6)) / (2000. - dm * 4200.))
                .abs()
                < 1e-12
        );
        assert!((gas.accounted_energy_j() + material.energy_j() - before).abs() < 1e-8);
        assert!((body.cells()[0].water_kg + gas.water_kg() - water - mv).abs() < 1e-14);
        let snapshot = format!("{body:?}{gas:?}{material:?}");
        assert!(
            body.advance_enthalpy_vapor(
                0.1,
                &mut gas,
                &mut material,
                f64::MAX,
                &[VaporLink {
                    material_cell: 0,
                    conductance_kg_s: 0.001
                }]
            )
            .is_err()
        );
        assert_eq!(snapshot, format!("{body:?}{gas:?}{material:?}"));
    }
}

#[test]
fn opposing_water_fluxes_transport_sensible_heat_even_with_zero_net_mass() {
    use physics::{
        liquid::SaturationCurve,
        moisture::{MaterialThermalStore, ThermalVapor},
    };
    let curve = SaturationCurve {
        reference_temperature: 300.,
        reference_pressure: 3500.,
        latent_heat: 2.4e6,
        vapor_gas_constant: 461.,
        min_temperature: 280.,
        max_temperature: 320.,
    };
    let mv = curve.pressure(300.).unwrap() / 461. / 300. * 0.5;
    let mut gas = ThermalVapor::new(300., 1000., 1., mv, curve).unwrap();
    let mut material = MaterialThermalStore::new(2000., 310.).unwrap();
    let mut body = Body::new(
        vec![
            Cell {
                capacity_kg: 0.01,
                water_kg: 0.01,
            },
            Cell {
                capacity_kg: 0.01,
                water_kg: 0.,
            },
        ],
        vec![],
    )
    .unwrap();
    let energy = gas.accounted_energy_j() + material.energy_j();
    let r = body
        .advance_enthalpy_vapor(
            0.1,
            &mut gas,
            &mut material,
            4200.,
            &[
                VaporLink {
                    material_cell: 0,
                    conductance_kg_s: 0.001,
                },
                VaporLink {
                    material_cell: 1,
                    conductance_kg_s: 0.001,
                },
            ],
        )
        .unwrap();
    assert!(r.vapor_water_change_kg.abs() < 1e-14);
    let flux = 0.0001 * 0.5 / (1. + 0.0001 / 0.01);
    assert!((gas.temperature_k() - (300. + flux * 4200. * 10. / 1000.)).abs() < 1e-12);
    assert!(material.temperature_k() < 310.);
    assert!((gas.accounted_energy_j() + material.energy_j() - energy).abs() < 1e-8);
}

#[test]
fn heated_mass_exchange_conserves_long_run_and_refines_in_time() {
    use physics::{
        liquid::SaturationCurve,
        moisture::{MaterialThermalStore, ThermalVapor},
    };
    fn run(steps: usize) -> [f64; 3] {
        let curve = SaturationCurve {
            reference_temperature: 300.,
            reference_pressure: 3500.,
            latent_heat: 2.4e6,
            vapor_gas_constant: 461.,
            min_temperature: 280.,
            max_temperature: 320.,
        };
        let mut gas = ThermalVapor::new(295., 1000., 1., 0., curve).unwrap();
        let mut material = MaterialThermalStore::new(2000., 310.).unwrap();
        let mut body = Body::new(
            vec![Cell {
                capacity_kg: 0.01,
                water_kg: 0.01,
            }],
            vec![],
        )
        .unwrap();
        let energy = gas.accounted_energy_j() + material.energy_j();
        let snapshot = format!("{body:?}{gas:?}{material:?}");
        assert!(
            body.advance_heated_vapor(
                0.1,
                &mut gas,
                &mut material,
                f64::MAX,
                100.,
                &[VaporLink {
                    material_cell: 0,
                    conductance_kg_s: 0.001
                }]
            )
            .is_err()
        );
        assert_eq!(snapshot, format!("{body:?}{gas:?}{material:?}"));
        for _ in 0..steps {
            body.advance_heated_vapor(
                1. / steps as f64,
                &mut gas,
                &mut material,
                4200.,
                100.,
                &[VaporLink {
                    material_cell: 0,
                    conductance_kg_s: 0.001,
                }],
            )
            .unwrap();
            assert!((body.cells()[0].water_kg + gas.water_kg() - 0.01).abs() < 1e-12);
            assert!((gas.accounted_energy_j() + material.energy_j() - energy).abs() < 1e-6);
        }
        [
            gas.water_kg(),
            gas.temperature_k(),
            material.temperature_k(),
        ]
    }
    let reference = run(1024);
    let coarse = run(8);
    let medium = run(16);
    let fine = run(32);
    for axis in 0..3 {
        let ec = (coarse[axis] - reference[axis]).abs();
        let em = (medium[axis] - reference[axis]).abs();
        let ef = (fine[axis] - reference[axis]).abs();
        assert!(
            em < 0.7 * ec && ef < 0.7 * em,
            "axis {axis}: {ec} {em} {ef}"
        );
    }
}

#[test]
fn effective_thermal_capacity_cannot_hide_negative_dry_capacity() {
    use physics::{
        liquid::SaturationCurve,
        moisture::{MaterialThermalStore, ThermalVapor},
    };
    let curve = SaturationCurve {
        reference_temperature: 300.,
        reference_pressure: 3500.,
        latent_heat: 2.4e6,
        vapor_gas_constant: 461.,
        min_temperature: 280.,
        max_temperature: 320.,
    };
    for (cg, cm) in [(10., 1000.), (1000., 10.)] {
        let mut gas = ThermalVapor::new(300., cg, 1., 0.01, curve).unwrap();
        let mut solid = MaterialThermalStore::new(cm, 300.).unwrap();
        let mut body = Body::new(
            vec![Cell {
                capacity_kg: 0.01,
                water_kg: 0.01,
            }],
            vec![],
        )
        .unwrap();
        let snapshot = format!("{body:?}{gas:?}{solid:?}");
        assert!(
            body.advance_enthalpy_vapor(
                0.1,
                &mut gas,
                &mut solid,
                4200.,
                &[VaporLink {
                    material_cell: 0,
                    conductance_kg_s: 0.001
                }]
            )
            .is_err()
        );
        assert_eq!(snapshot, format!("{body:?}{gas:?}{solid:?}"));
    }
}

#[test]
fn heat_deposition_is_bounded_atomic_and_energy_accounted() {
    use physics::moisture::MaterialThermalStore;
    let mut store = MaterialThermalStore::new(2000., 300.).unwrap();
    let initial = store.energy_j();
    store.deposit_heat(1000.).unwrap();
    assert_eq!(store.temperature_k(), 300.5);
    assert!((store.energy_j() - initial - 1000.).abs() < 1e-10);
    for heat in [-1., f64::INFINITY, f64::MIN_POSITIVE] {
        let before = format!("{store:?}");
        assert!(store.deposit_heat(heat).is_err());
        assert_eq!(before, format!("{store:?}"));
    }
}

#[test]
fn adaptive_thermal_drying_converges_to_independent_closed_reservoir_ode_and_rolls_back() {
    use physics::{
        liquid::SaturationCurve,
        moisture::{ThermalVapor, ThermalVaporAccuracy},
    };
    let curve = SaturationCurve {
        reference_temperature: 300.,
        reference_pressure: 3500.,
        latent_heat: 2.4e6,
        vapor_gas_constant: 461.,
        min_temperature: 280.,
        max_temperature: 320.,
    };
    for (initial_water, initial_vapor) in [(0.01, 0.), (0., 0.005)] {
        let total_water = initial_water + initial_vapor;
        let initial = || {
            (
                Body::new(
                    vec![Cell {
                        capacity_kg: 0.01,
                        water_kg: initial_water,
                    }],
                    vec![],
                )
                .unwrap(),
                ThermalVapor::new(300., 1000., 1., initial_vapor, curve).unwrap(),
            )
        };
        let links = [VaporLink {
            material_cell: 0,
            conductance_kg_s: 0.001,
        }];
        // Independent RK4 integration of the continuous activity/latent-energy ODE.
        let rate = |water: f64| {
            let temperature = 300. - curve.latent_heat * (water - initial_vapor) / 1000.;
            let capacity =
                curve.pressure(temperature).unwrap() / (curve.vapor_gas_constant * temperature);
            0.001 * ((total_water - water) / 0.01 - water / capacity)
        };
        let h = 0.5 / 4096.;
        let mut expected = initial_vapor;
        for _ in 0..4096 {
            let a = rate(expected);
            let b = rate(expected + h * a / 2.);
            let c = rate(expected + h * b / 2.);
            let d = rate(expected + h * c);
            expected += h * (a + 2. * b + 2. * c + d) / 6.;
        }
        let mut previous = f64::INFINITY;
        for tolerance in [1e-7, 1e-9, 1e-11] {
            let (mut body, mut gas) = initial();
            let energy = gas.accounted_energy_j();
            let report = body
                .advance_thermal_vapor_adaptive(
                    0.5,
                    &mut gas,
                    &links,
                    ThermalVaporAccuracy {
                        relative_tolerance: 0.,
                        mass_tolerance_kg: tolerance,
                        temperature_tolerance_k: 1e-3,
                        max_attempts: 10000,
                    },
                )
                .unwrap();
            let error = (gas.water_kg() - expected).abs();
            eprintln!(
                "ADAPTIVE_MOISTURE initial_material={initial_water} tolerance={tolerance:e} mass_error={error:e}"
            );
            assert!(
                error < previous / 2.,
                "errors did not converge: {error}, {previous}"
            );
            previous = error;
            assert!((body.cells()[0].water_kg + gas.water_kg() - total_water).abs() < 1e-14);
            assert!((gas.accounted_energy_j() - energy).abs() < 1e-8);
            assert!(
                (gas.temperature_k()
                    - (300. - curve.latent_heat * (gas.water_kg() - initial_vapor) / 1000.))
                    .abs()
                    < 1e-10
            );
            assert!(report.mass_defect_kg.abs() < 1e-14);
            assert!(report.energy_defect_j.abs() < 1e-8);
        }
        assert!(previous < 1e-7);
        let (mut body, mut gas) = initial();
        let before = format!("{body:?}{gas:?}");
        assert!(
            body.advance_thermal_vapor_adaptive(
                0.5,
                &mut gas,
                &links,
                ThermalVaporAccuracy {
                    relative_tolerance: 0.,
                    mass_tolerance_kg: 1e-16,
                    temperature_tolerance_k: 1e-12,
                    max_attempts: 1
                }
            )
            .is_err()
        );
        assert_eq!(before, format!("{body:?}{gas:?}"));
        assert!(
            body.advance_thermal_vapor_adaptive(
                0.5,
                &mut gas,
                &[links[0], links[0]],
                ThermalVaporAccuracy::default()
            )
            .is_err()
        );
        assert_eq!(before, format!("{body:?}{gas:?}"));
    }
}

#[test]
fn adaptive_receipt_reports_complete_time_and_temperature_floor_rejects_atomically() {
    use physics::{
        liquid::SaturationCurve,
        moisture::{ThermalVapor, ThermalVaporAccuracy},
    };
    let curve = SaturationCurve {
        reference_temperature: 300.,
        reference_pressure: 3500.,
        latent_heat: 2.4e6,
        vapor_gas_constant: 461.,
        min_temperature: 280.,
        max_temperature: 320.,
    };
    let initial = |temperature| {
        (
            Body::new(
                vec![Cell {
                    capacity_kg: 0.01,
                    water_kg: 0.01,
                }],
                vec![],
            )
            .unwrap(),
            ThermalVapor::new(temperature, 1000., 1., 0., curve).unwrap(),
        )
    };
    let link = [VaporLink {
        material_cell: 0,
        conductance_kg_s: 0.001,
    }];
    let accuracy = ThermalVaporAccuracy {
        relative_tolerance: 0.,
        mass_tolerance_kg: 1e-11,
        temperature_tolerance_k: 1e-6,
        max_attempts: 10000,
    };
    let (mut body, mut gas) = initial(300.);
    let (mut reference, mut reference_gas) = initial(300.);
    let transfer = reference
        .advance_thermal_vapor_adaptive(0.5, &mut reference_gas, &link, accuracy)
        .unwrap();
    let report = body
        .advance_thermal_vapor_adaptive_with_receipt(0.5, &mut gas, &link, accuracy)
        .unwrap();
    assert_eq!(
        format!("{body:?}{gas:?}"),
        format!("{reference:?}{reference_gas:?}")
    );
    assert_eq!(format!("{:?}", report.transfer), format!("{transfer:?}"));
    assert_eq!(report.elapsed_s, 0.5);
    assert!(report.accepted_intervals > 1);
    assert!(report.rejected_attempts > 0);
    assert_eq!(
        report.attempts,
        report.accepted_intervals + report.rejected_attempts
    );
    assert!(report.attempts <= accuracy.max_attempts);
    assert!(report.minimum_interval_s > 0. && report.minimum_interval_s < 0.5);
    eprintln!("THERMAL_VAPOR_RECEIPT {report:?}");
    let (mut body, mut gas) = initial(280.);
    let before = format!("{body:?}{gas:?}");
    assert!(
        body.advance_thermal_vapor_adaptive_with_receipt(
            0.5,
            &mut gas,
            &link,
            ThermalVaporAccuracy {
                max_attempts: 32,
                ..accuracy
            }
        )
        .is_err()
    );
    assert_eq!(before, format!("{body:?}{gas:?}"));
    // A zero-flux interval admits the entire duration without invented work.
    let (mut body, mut gas) = initial(300.);
    let report = body
        .advance_thermal_vapor_adaptive_with_receipt(0.5, &mut gas, &[], accuracy)
        .unwrap();
    assert_eq!(report.attempts, 1);
    assert_eq!(report.accepted_intervals, 1);
    assert_eq!(report.rejected_attempts, 0);
    assert_eq!(report.minimum_interval_s, 0.5);
    assert_eq!(report.transfer.vapor_water_change_kg, 0.);
    assert_eq!(report.transfer.latent_exchange_j, 0.);
}

#[test]
fn prepared_multicell_thermal_network_matches_independent_ode_with_opposing_fluxes() {
    use physics::{
        liquid::SaturationCurve,
        moisture::{Link, ThermalVapor, ThermalVaporAccuracy},
    };
    let curve = SaturationCurve {
        reference_temperature: 300.,
        reference_pressure: 3500.,
        latent_heat: 2.4e6,
        vapor_gas_constant: 461.,
        min_temperature: 280.,
        max_temperature: 320.,
    };
    let capacities = [0.01, 0.02, 0.03, 0.04];
    let initial = [0., 0.01, 0.025, 0.002, 0.003];
    let internal = [
        Link {
            cells: [0, 1],
            conductance_kg_s: 0.0002,
        },
        Link {
            cells: [1, 2],
            conductance_kg_s: 0.0003,
        },
        Link {
            cells: [2, 3],
            conductance_kg_s: 0.0001,
        },
    ];
    let links: Vec<_> = (0..4)
        .map(|i| VaporLink {
            material_cell: i,
            conductance_kg_s: 0.0001 * (i + 1) as f64,
        })
        .collect();
    let rate = |state: [f64; 5]| {
        let temperature = 300. - curve.latent_heat * (state[4] - initial[4]) / 1000.;
        let gas_capacity =
            curve.pressure(temperature).unwrap() / (curve.vapor_gas_constant * temperature);
        let mut derivative = [0.; 5];
        for link in internal {
            let [a, b] = link.cells;
            let flow =
                link.conductance_kg_s * (state[a] / capacities[a] - state[b] / capacities[b]);
            derivative[a] -= flow;
            derivative[b] += flow;
        }
        for link in &links {
            let i = link.material_cell;
            let flow = link.conductance_kg_s * (state[i] / capacities[i] - state[4] / gas_capacity);
            derivative[i] -= flow;
            derivative[4] += flow;
        }
        derivative
    };
    let mut expected = initial;
    let h = 0.5 / 4096.;
    for _ in 0..4096 {
        let a = rate(expected);
        let b = rate(std::array::from_fn(|i| expected[i] + h * a[i] / 2.));
        let c = rate(std::array::from_fn(|i| expected[i] + h * b[i] / 2.));
        let d = rate(std::array::from_fn(|i| expected[i] + h * c[i]));
        expected =
            std::array::from_fn(|i| expected[i] + h * (a[i] + 2. * b[i] + 2. * c[i] + d[i]) / 6.);
    }
    let mut body = Body::new(
        (0..4)
            .map(|i| Cell {
                capacity_kg: capacities[i],
                water_kg: initial[i],
            })
            .collect(),
        internal.to_vec(),
    )
    .unwrap();
    let mut gas = ThermalVapor::new(300., 1000., 1., initial[4], curve).unwrap();
    let report = body
        .advance_thermal_vapor_adaptive_with_receipt(
            0.5,
            &mut gas,
            &links,
            ThermalVaporAccuracy {
                relative_tolerance: 0.,
                mass_tolerance_kg: 1e-11,
                temperature_tolerance_k: 1e-6,
                max_attempts: 20000,
            },
        )
        .unwrap();
    let actual: [f64; 5] = std::array::from_fn(|i| {
        if i < 4 {
            body.cells()[i].water_kg
        } else {
            gas.water_kg()
        }
    });
    let error = actual
        .iter()
        .zip(expected)
        .map(|(a, b)| (a - b).abs())
        .fold(0., f64::max);
    assert!(error < 2e-7, "multi-cell ODE error {error}");
    assert!(report.transfer.material_water_change_kg[0] > 0.);
    assert!(report.transfer.material_water_change_kg[2] < 0.);
    assert!((actual.iter().sum::<f64>() - initial.iter().sum::<f64>()).abs() < 1e-13);
    assert!(report.transfer.energy_defect_j.abs() < 1e-8);
    eprintln!(
        "PREPARED_MULTICELL max_mass_error_kg={error:e} attempts={} accepted={}",
        report.attempts, report.accepted_intervals
    );
}

#[test]
fn exact_nonzero_activity_equilibrium_preserves_all_inventories() {
    for n in [1, 3, 129] {
        let cells: Vec<_> = (0..n)
            .map(|i| {
                let capacity_kg = if i % 2 == 0 { 0.1 } else { 0.3 };
                Cell {
                    capacity_kg,
                    water_kg: capacity_kg * 0.5,
                }
            })
            .collect();
        let mut body = Body::new(cells, vec![]).unwrap();
        let mut vapor = VaporReservoir::new(0.75, 0.375, 2.4e6, 1e6).unwrap();
        let before = format!("{body:?} {vapor:?}");
        let links: Vec<_> = (0..n)
            .map(|material_cell| VaporLink {
                material_cell,
                conductance_kg_s: 0.01,
            })
            .collect();
        for _ in 0..10 {
            let report = body.advance_vapor(1., &mut vapor, &links).unwrap();
            assert!(report.material_water_change_kg.iter().all(|&x| x == 0.));
            assert_eq!(report.vapor_water_change_kg, 0.);
            assert_eq!(report.latent_exchange_j, 0.);
            assert_eq!(report.mass_defect_kg, 0.);
            assert_eq!(report.energy_defect_j, 0.);
            assert_eq!(format!("{body:?} {vapor:?}"), before);
        }
    }
}

#[test]
fn equilibrium_does_not_hide_invalid_operator_or_small_physical_flux() {
    let mut body = Body::new(
        vec![Cell {
            capacity_kg: 1.,
            water_kg: 0.5,
        }],
        vec![],
    )
    .unwrap();
    let mut vapor = VaporReservoir::new(1., 0.5, 2.4e6, 1e6).unwrap();
    let before = format!("{body:?} {vapor:?}");
    let link = VaporLink {
        material_cell: 0,
        conductance_kg_s: f64::MAX,
    };
    assert!(body.advance_vapor(2., &mut vapor, &[link]).is_err());
    assert_eq!(format!("{body:?} {vapor:?}"), before);
    assert!(body.advance_vapor(0., &mut vapor, &[]).is_err());
    assert_eq!(format!("{body:?} {vapor:?}"), before);
    let mut vapor = VaporReservoir::new(1., 0.5 - 1e-6, 2.4e6, 1e6).unwrap();
    let report = body
        .advance_vapor(
            1.,
            &mut vapor,
            &[VaporLink {
                conductance_kg_s: 0.1,
                ..link
            }],
        )
        .unwrap();
    let expected = 1e-7 / 1.2;
    assert!(report.vapor_water_change_kg > 0.);
    assert!((report.vapor_water_change_kg - expected).abs() < 1e-15);
}
