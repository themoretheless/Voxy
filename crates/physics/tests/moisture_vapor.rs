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
