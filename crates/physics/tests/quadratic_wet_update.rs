use physics::{
    biomechanics::Material as Elastic,
    moisture::{Calibration, Cell, Properties},
    plasticity::{
        Material,
        mesh::{FiniteQuadraticDynamics, QuadraticBody},
    },
};
fn calibration() -> Calibration {
    let dry = Properties {
        young_pa: 1e5,
        poisson: 0.3,
        yield_pa: 1e9,
        hardening_pa: 0.,
        hardness_pa: 1e8,
        wear_coefficient: 1e-3,
    };
    Calibration::new(
        dry,
        Properties {
            young_pa: 5e4,
            ..dry
        },
    )
    .unwrap()
}
fn fixture(pinned: bool, deformed: bool) -> FiniteQuadraticDynamics {
    let mut body = QuadraticBody::from_linear(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![([0, 1, 2, 3], Material::new(1e5, 0.3, 1e9, 0.).unwrap())],
    )
    .unwrap();
    if deformed {
        let prescribed: Vec<_> = body
            .positions()
            .iter()
            .map(|p| [Some(0.01 * p[0]), Some(0.), Some(0.)])
            .collect();
        assert!(
            body.equilibrate(&vec![[0.; 3]; 10], &prescribed, 1, 1e-9)
                .unwrap()
                .converged
        );
    }
    let mut supports = vec![false; 10];
    supports[0] = pinned;
    let velocity = (0..10)
        .map(|i| {
            if deformed || (pinned && i == 0) {
                [0.; 3]
            } else {
                [1., 0., 0.]
            }
        })
        .collect();
    FiniteQuadraticDynamics::new(
        body,
        vec![Elastic::from_young_poisson(1e5, 0.3).unwrap()],
        &[6.],
        velocity,
        &supports,
    )
    .unwrap()
}
#[test]
fn water_entrainment_and_drying_conserve_mass_momentum_and_energy() {
    for incoming in [0., 3.] {
        let mut body = fixture(false, false);
        let old = body.energy().unwrap();
        let r = body
            .apply_moisture(
                &[Cell {
                    capacity_kg: 1.,
                    water_kg: 1.,
                }],
                &[1.],
                &[calibration()],
                &vec![[incoming, 0., 0.]; 10],
            )
            .unwrap();
        let new = body.energy().unwrap();
        assert!((new.mass_kg - 2.).abs() < 1e-12);
        assert!((r.water_mass_change_kg - 1.).abs() < 1e-12);
        for v in body.velocities() {
            assert!((v[0] - (1. + incoming) / 2.).abs() < 1e-12);
        }
        assert!((new.momentum_kg_m_s[0] - old.momentum_kg_m_s[0] - incoming).abs() < 1e-12);
        assert!((r.carried_water_kinetic_j - 0.5 * incoming * incoming).abs() < 1e-12);
        assert!((r.kinetic_transfer_loss_j - 0.25 * (incoming - 1.).powi(2)).abs() < 1e-12);
        assert!(r.energy_defect_j.abs() < 1e-12);
        let velocities = body.velocities().to_vec();
        let r = body
            .apply_moisture(
                &[Cell {
                    capacity_kg: 1.,
                    water_kg: 0.,
                }],
                &[1.],
                &[calibration()],
                &vec![[100., 0., 0.]; 10],
            )
            .unwrap();
        for (a, b) in body.velocities().iter().zip(velocities) {
            for i in 0..3 {
                assert!((a[i] - b[i]).abs() < 1e-12);
            }
        }
        assert!((r.water_mass_change_kg + 1.).abs() < 1e-12);
        assert!((r.water_momentum_kg_m_s[0] + (1. + incoming) / 2.).abs() < 1e-12);
        assert!(r.kinetic_transfer_loss_j.abs() < 1e-12);
    }
}
#[test]
fn elastic_parameter_work_and_support_impulse_are_explicit() {
    let mut body = fixture(false, true);
    let initial = body.energy().unwrap();
    let r = body
        .apply_moisture(
            &[Cell {
                capacity_kg: 1.,
                water_kg: 1.,
            }],
            &[1.],
            &[calibration()],
            &vec![[0.; 3]; 10],
        )
        .unwrap();
    assert!((body.energy().unwrap().elastic_j - 0.5 * initial.elastic_j).abs() < 1e-9);
    assert!((r.elastic_parameter_work_j + 0.5 * initial.elastic_j).abs() < 1e-9);
    body.step(1e-5, [0.; 3], 1e-5).unwrap();
    let mut supported = fixture(true, false);
    let initial = supported.energy().unwrap();
    let r = supported
        .apply_moisture(
            &[Cell {
                capacity_kg: 1.,
                water_kg: 1.,
            }],
            &[1.],
            &[calibration()],
            &vec![[3., 0., 0.]; 10],
        )
        .unwrap();
    assert_eq!(supported.velocities()[0], [0.; 3]);
    let support: f64 = r.support_impulse_n_s.iter().map(|p| p[0]).sum();
    assert!(support.abs() > 1e-8);
    assert!(
        (supported.energy().unwrap().momentum_kg_m_s[0]
            - initial.momentum_kg_m_s[0]
            - r.water_momentum_kg_m_s[0]
            - support)
            .abs()
            < 1e-10
    );
}
#[test]
fn invalid_mass_remap_is_atomic() {
    let mut body = fixture(false, false);
    let positions = body.positions().to_vec();
    let velocities = body.velocities().to_vec();
    let mass = body.energy().unwrap().mass_kg;
    assert!(
        body.apply_moisture(
            &[Cell {
                capacity_kg: 1.,
                water_kg: 1.
            }],
            &[2.],
            &[calibration()],
            &vec![[0.; 3]; 10]
        )
        .is_err()
    );
    assert_eq!(body.positions(), positions);
    assert_eq!(body.velocities(), velocities);
    assert_eq!(body.energy().unwrap().mass_kg, mass);
}

#[test]
fn finite_supply_water_and_mechanics_commit_together_or_roll_back() {
    use physics::moisture::{Body, WaterSupply};
    let mut solid = fixture(false, false);
    let mut water = Body::new(
        vec![Cell {
            capacity_kg: 1.,
            water_kg: 0.,
        }],
        vec![],
    )
    .unwrap();
    let mut supplies = [WaterSupply {
        cell: 0,
        water_kg: 0.1,
        conductance_kg_s: 1.,
    }];
    let (transfer, mechanics) = solid
        .advance_moisture_supplies(
            10.,
            &mut water,
            &mut supplies,
            &[1.],
            &[calibration()],
            &vec![[0.; 3]; 10],
        )
        .unwrap();
    assert_eq!(supplies[0].water_kg, 0.);
    assert!((water.cells()[0].water_kg - 0.1).abs() < 1e-12);
    assert!((transfer.supplied_water_kg[0] - 0.1).abs() < 1e-12);
    assert!((solid.energy().unwrap().mass_kg - 1.1).abs() < 1e-12);
    for v in solid.velocities() {
        assert!((v[0] - 1. / 1.1).abs() < 1e-12);
    }
    assert!((mechanics.water_mass_change_kg - 0.1).abs() < 1e-12);
    let velocity = solid.velocities().to_vec();
    let mass = solid.energy().unwrap().mass_kg;
    supplies[0].water_kg = 0.1;
    // Moisture would consume water, but the mechanical velocity mapping is invalid.
    assert!(
        solid
            .advance_moisture_supplies(10., &mut water, &mut supplies, &[1.], &[calibration()], &[])
            .is_err()
    );
    assert_eq!(supplies[0].water_kg, 0.1);
    assert!((water.cells()[0].water_kg - 0.1).abs() < 1e-12);
    assert_eq!(solid.velocities(), velocity);
    assert_eq!(solid.energy().unwrap().mass_kg, mass);
}

#[test]
fn wet_cohesive_update_breaks_dynamic_fragment_without_resetting_work() {
    use physics::moisture::{CohesiveCalibration, CohesiveProperties};
    let dry = CohesiveProperties {
        stiffness_pa_m: 1e6,
        closure_pa_m: 1e7,
        peak_pa: 1000.,
        fracture_j_m2: 10.,
    };
    let calibration = CohesiveCalibration::new(
        dry,
        CohesiveProperties {
            peak_pa: 500.,
            fracture_j_m2: 2.5,
            ..dry
        },
    )
    .unwrap();
    let mut body = QuadraticBody::from_linear_with_cohesive_faces(
        vec![
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0., 0., -1.],
        ],
        vec![
            ([0, 1, 2, 3], Material::new(1e5, 0.3, 1e9, 0.).unwrap()),
            ([0, 2, 1, 4], Material::new(1e5, 0.3, 1e9, 0.).unwrap()),
        ],
        calibration.at(0.).unwrap(),
    )
    .unwrap()
    .body;
    let n = body.positions().len();
    let prescribed = (0..n)
        .map(|i| [Some(0.), Some(0.), Some(if i < 10 { 0.0125 } else { 0. })])
        .collect::<Vec<_>>();
    assert!(
        body.equilibrate(&vec![[0.; 3]; n], &prescribed, 1, 1e-9)
            .unwrap()
            .converged
    );
    let mut dynamics = FiniteQuadraticDynamics::new(
        body,
        vec![Elastic::from_young_poisson(1e5, 0.3).unwrap(); 2],
        &[1.; 2],
        vec![[0.; 3]; n],
        &vec![false; n],
    )
    .unwrap();
    let before = dynamics.energy().unwrap();
    let mut supplied = dynamics.clone();
    let mut network = physics::moisture::Body::new(
        vec![
            Cell {
                capacity_kg: 0.001,
                water_kg: 0.
            };
            2
        ],
        vec![],
    )
    .unwrap();
    let mut sources = [
        physics::moisture::WaterSupply {
            cell: 0,
            water_kg: 0.001,
            conductance_kg_s: 1.,
        },
        physics::moisture::WaterSupply {
            cell: 1,
            water_kg: 0.001,
            conductance_kg_s: 1.,
        },
    ];
    let bulk_laws = [crate::calibration(); 2];
    assert!(
        supplied
            .advance_moisture_supplies_with_cohesion(
                1.,
                &mut network,
                &mut sources,
                &[1. / 6.; 2],
                &bulk_laws,
                &vec![[0.; 3]; n],
                &[],
                &[0.5]
            )
            .is_err()
    );
    assert_eq!(network.cells()[0].water_kg, 0.);
    assert_eq!(sources[0].water_kg, 0.001);
    assert_eq!(supplied.energy().unwrap().mass_kg, before.mass_kg);
    let (transfer, bulk, fracture) = supplied
        .advance_moisture_supplies_with_cohesion(
            1.,
            &mut network,
            &mut sources,
            &[1. / 6.; 2],
            &bulk_laws,
            &vec![[0.; 3]; n],
            &[calibration],
            &[0.5],
        )
        .unwrap();
    assert_eq!(fracture.fragments_before, 1);
    assert_eq!(fracture.fragments_after, 2);
    let material_water: f64 = network.cells().iter().map(|c| c.water_kg).sum();
    assert!(
        (material_water + sources.iter().map(|s| s.water_kg).sum::<f64>() - 0.002).abs() < 1e-14
    );
    assert!(
        (bulk.water_mass_change_kg - transfer.supplied_water_kg.iter().sum::<f64>()).abs() < 1e-12
    );
    assert!((supplied.energy().unwrap().mass_kg - before.mass_kg - material_water).abs() < 1e-12);
    let water = [Cell {
        capacity_kg: 0.001,
        water_kg: 0.001,
    }; 2];
    let bulk_calibration = [crate::calibration(); 2];
    let asymmetric = [
        Cell {
            capacity_kg: 1.,
            water_kg: 0.,
        },
        Cell {
            capacity_kg: 1.,
            water_kg: 1.,
        },
    ];
    let minus = dynamics
        .cohesive_cell_saturations(&asymmetric, &[1.])
        .unwrap()[0];
    let plus = dynamics
        .cohesive_cell_saturations(&asymmetric, &[0.])
        .unwrap()[0];
    assert_eq!(minus + plus, 1.);
    assert_eq!(
        dynamics
            .cohesive_cell_saturations(&asymmetric, &[0.25])
            .unwrap()[0],
        0.25 * minus + 0.75 * plus
    );
    assert!(
        dynamics
            .cohesive_cell_saturations(&asymmetric, &[1.1])
            .is_err()
    );

    assert_eq!(
        dynamics.cohesive_cell_saturations(&water, &[0.5]).unwrap(),
        vec![1.]
    );
    let (bulk, report) = dynamics
        .apply_moisture_with_cohesion(
            &water,
            &[1. / 6.; 2],
            &bulk_calibration,
            &vec![[0.; 3]; n],
            &[calibration],
            &[0.5],
        )
        .unwrap();
    assert!((bulk.water_mass_change_kg - 0.002).abs() < 1e-12);
    assert_eq!(report.fragments_before, 1);
    assert_eq!(report.fragments_after, 2);
    let after = dynamics.energy().unwrap();
    assert!((after.fracture_dissipated_j - before.fracture_dissipated_j).abs() < 1e-12);
    assert!(
        (after.cohesive_stored_j - before.cohesive_stored_j - report.total_parameter_work_j).abs()
            < 1e-12
    );
    assert!(
        dynamics
            .apply_cohesive_moisture(&[0.], &[calibration])
            .is_err()
    );
    assert!((dynamics.energy().unwrap().cohesive_stored_j - after.cohesive_stored_j).abs() < 1e-12);
    let dry_water = [Cell {
        capacity_kg: 0.001,
        water_kg: 0.,
    }; 2];
    assert!(
        dynamics
            .apply_moisture_with_cohesion(
                &dry_water,
                &[1. / 6.; 2],
                &bulk_calibration,
                &vec![[0.; 3]; n],
                &[calibration],
                &[0.5]
            )
            .is_err()
    );
    assert_eq!(dynamics.energy().unwrap().mass_kg, after.mass_kg);
    assert_eq!(
        dynamics.energy().unwrap().fracture_dissipated_j,
        after.fracture_dissipated_j
    );
}

#[test]
fn whole_wet_dynamic_interval_rolls_back_water_if_motion_fails() {
    use physics::{
        moisture::{Body, WaterSupply},
        plasticity::mesh::QuadraticAdvanceLimits,
    };
    let mut solid = fixture(false, false);
    let mut water = Body::new(
        vec![Cell {
            capacity_kg: 1.,
            water_kg: 0.,
        }],
        vec![],
    )
    .unwrap();
    let mut supplies = [WaterSupply {
        cell: 0,
        water_kg: 0.1,
        conductance_kg_s: 1.,
    }];
    let limits = QuadraticAdvanceLimits {
        minimum_dt_s: 1e-5,
        maximum_dt_s: 0.01,
        max_attempts: 100,
        energy_tolerance_j: 1e-8,
    };
    let positions = solid.positions().to_vec();
    assert!(
        solid
            .advance_wet_loaded(
                0.01,
                &mut water,
                &mut supplies,
                &[1.],
                &[calibration()],
                &[[0.; 3]; 10],
                &[],
                &[],
                &[],
                [0.; 3],
                limits
            )
            .is_err()
    );
    assert_eq!(solid.positions(), positions);
    assert_eq!(water.cells()[0].water_kg, 0.);
    assert_eq!(supplies[0].water_kg, 0.1);
    let report = solid
        .advance_wet_loaded(
            0.01,
            &mut water,
            &mut supplies,
            &[1.],
            &[calibration()],
            &[[0.; 3]; 10],
            &[],
            &[],
            &[[0.; 3]; 10],
            [0.; 3],
            limits,
        )
        .unwrap();
    assert!(!report.motion.substeps.is_empty());
    assert!((report.motion.substeps.iter().map(|s| s.dt_s).sum::<f64>() - 0.01).abs() < 1e-14);
    let mass = solid.energy().unwrap().mass_kg;
    assert!((water.cells()[0].water_kg + supplies[0].water_kg - 0.1).abs() < 1e-14);
    assert!((mass - 1. - water.cells()[0].water_kg).abs() < 1e-12);
    for (old, new) in positions.iter().zip(solid.positions()) {
        assert!((new[0] - old[0] - 0.01 / mass).abs() < 1e-12);
    }
}

#[test]
fn vapor_drying_updates_solid_mass_and_rolls_back_mechanical_failure() {
    use physics::moisture::{Body, VaporLink, VaporReservoir};
    let mut solid = fixture(false, false);
    solid
        .apply_moisture(
            &[Cell {
                capacity_kg: 1.,
                water_kg: 1.,
            }],
            &[1.],
            &[calibration()],
            &vec![[1., 0., 0.]; 10],
        )
        .unwrap();
    let mut water = Body::new(
        vec![Cell {
            capacity_kg: 1.,
            water_kg: 1.,
        }],
        vec![],
    )
    .unwrap();
    let mut vapor = VaporReservoir::new(0.5, 0., 2e6, 3e6).unwrap();
    let links = [VaporLink {
        material_cell: 0,
        conductance_kg_s: 0.1,
    }];
    let before = solid.energy().unwrap();
    let (transfer, bulk, _) = solid
        .advance_moisture_vapor_with_cohesion(
            0.2,
            &mut water,
            &mut vapor,
            &links,
            &[1.],
            &[calibration()],
            &vec![[0.; 3]; 10],
            &[],
            &[],
        )
        .unwrap();
    let after = solid.energy().unwrap();
    assert!(transfer.vapor_water_change_kg > 0.);
    assert!((after.mass_kg + vapor.water_kg() - 2.).abs() < 1e-12);
    assert!((after.mass_kg - before.mass_kg + transfer.vapor_water_change_kg).abs() < 1e-12);
    assert!(
        (after.momentum_kg_m_s[0] - before.momentum_kg_m_s[0] - bulk.water_momentum_kg_m_s[0])
            .abs()
            < 1e-12
    );
    assert!(bulk.energy_defect_j.abs() < 1e-12);
    let snapshot = format!("{solid:?}{water:?}{vapor:?}");
    // Transport would succeed; invalid mechanical velocity dimensions must undo it.
    assert!(
        solid
            .advance_moisture_vapor_with_cohesion(
                0.2,
                &mut water,
                &mut vapor,
                &links,
                &[1.],
                &[calibration()],
                &[],
                &[],
                &[]
            )
            .is_err()
    );
    assert_eq!(snapshot, format!("{solid:?}{water:?}{vapor:?}"));
}

#[test]
fn vapor_condensation_accounts_for_incoming_momentum_and_mixing_loss() {
    use physics::moisture::{Body, VaporLink, VaporReservoir};
    let mut solid = fixture(false, false);
    let mut water = Body::new(
        vec![Cell {
            capacity_kg: 1.,
            water_kg: 0.,
        }],
        vec![],
    )
    .unwrap();
    let mut vapor = VaporReservoir::new(0.5, 0.5, 2e6, 0.).unwrap();
    let initial_energy = vapor.accounted_energy_j();
    let (transfer, bulk, _) = solid
        .advance_moisture_vapor_with_cohesion(
            0.2,
            &mut water,
            &mut vapor,
            &[VaporLink {
                material_cell: 0,
                conductance_kg_s: 0.1,
            }],
            &[1.],
            &[calibration()],
            &[[3., 0., 0.]; 10],
            &[],
            &[],
        )
        .unwrap();
    let added = -transfer.vapor_water_change_kg;
    let energy = solid.energy().unwrap();
    assert!(added > 0.);
    assert!((energy.mass_kg + vapor.water_kg() - 1.5).abs() < 1e-12);
    assert!((energy.momentum_kg_m_s[0] - 1. - 3. * added).abs() < 1e-12);
    assert!((bulk.carried_water_kinetic_j - 4.5 * added).abs() < 1e-12);
    assert!((bulk.kinetic_transfer_loss_j - 2. * added / (1. + added)).abs() < 1e-12);
    for velocity in solid.velocities() {
        assert!((velocity[0] - (1. + 3. * added) / (1. + added)).abs() < 1e-12);
    }
    assert!((vapor.thermal_j() - 2e6 * added).abs() < 1e-8);
    assert!((vapor.accounted_energy_j() - initial_energy).abs() < 1e-8);
}

#[test]
fn vapor_motion_interval_commits_translation_or_rolls_back_every_owner() {
    use physics::{
        moisture::{Body, VaporLink, VaporReservoir},
        plasticity::mesh::QuadraticAdvanceLimits,
    };
    let mut solid = fixture(false, false);
    let mut water = Body::new(
        vec![Cell {
            capacity_kg: 1.,
            water_kg: 0.,
        }],
        vec![],
    )
    .unwrap();
    let mut vapor = VaporReservoir::new(0.5, 0.5, 2e6, 0.).unwrap();
    let links = [VaporLink {
        material_cell: 0,
        conductance_kg_s: 0.1,
    }];
    let limits = QuadraticAdvanceLimits {
        minimum_dt_s: 1e-5,
        maximum_dt_s: 0.01,
        max_attempts: 100,
        energy_tolerance_j: 1e-8,
    };
    let snapshot = format!("{solid:?}{water:?}{vapor:?}");
    assert!(
        solid
            .advance_vapor_loaded(
                0.01,
                &mut water,
                &mut vapor,
                &links,
                &[1.],
                &[calibration()],
                &[[0.; 3]; 10],
                &[],
                &[],
                &[],
                [0.; 3],
                limits
            )
            .is_err()
    );
    assert_eq!(snapshot, format!("{solid:?}{water:?}{vapor:?}"));
    let positions = solid.positions().to_vec();
    let (_, _, _, motion) = solid
        .advance_vapor_loaded(
            0.01,
            &mut water,
            &mut vapor,
            &links,
            &[1.],
            &[calibration()],
            &[[0.; 3]; 10],
            &[],
            &[],
            &[[0.; 3]; 10],
            [0.; 3],
            limits,
        )
        .unwrap();
    assert!(!motion.substeps.is_empty());
    let mass = solid.energy().unwrap().mass_kg;
    assert!((mass + vapor.water_kg() - 1.5).abs() < 1e-12);
    for (old, new) in positions.iter().zip(solid.positions()) {
        assert!((new[0] - old[0] - 0.01 / mass).abs() < 1e-12);
    }
}

#[test]
fn heated_vapor_and_loaded_body_publish_or_restore_all_four_owners() {
    use physics::{
        liquid::SaturationCurve,
        moisture::{Body, MaterialThermalStore, ThermalVapor, VaporLink},
        plasticity::mesh::QuadraticAdvanceLimits,
    };
    let curve = SaturationCurve {
        reference_temperature: 300.,
        reference_pressure: 3500.,
        latent_heat: 2.4e6,
        vapor_gas_constant: 461.,
        min_temperature: 280.,
        max_temperature: 320.,
    };
    let mut solid = fixture(false, false);
    let mut water = Body::new(
        vec![Cell {
            capacity_kg: 1.,
            water_kg: 0.,
        }],
        vec![],
    )
    .unwrap();
    let mut gas = ThermalVapor::new(300., 1000., 1., 0.01, curve).unwrap();
    let mut thermal = MaterialThermalStore::new(2000., 305.).unwrap();
    let links = [VaporLink {
        material_cell: 0,
        conductance_kg_s: 0.1,
    }];
    let limits = QuadraticAdvanceLimits {
        minimum_dt_s: 1e-5,
        maximum_dt_s: 0.01,
        max_attempts: 100,
        energy_tolerance_j: 1e-8,
    };
    let snapshot = format!("{solid:?}{water:?}{gas:?}{thermal:?}");
    assert!(
        solid
            .advance_heated_vapor_loaded(
                0.01,
                &mut water,
                &mut gas,
                &mut thermal,
                4200.,
                100.,
                &links,
                &[1.],
                &[calibration()],
                &[[0.; 3]; 10],
                &[],
                &[],
                &[],
                [0.; 3],
                limits
            )
            .is_err()
    );
    assert_eq!(snapshot, format!("{solid:?}{water:?}{gas:?}{thermal:?}"));
    let energy = gas.accounted_energy_j() + thermal.energy_j();
    let positions = solid.positions().to_vec();
    let (transfer, _, bulk, _, motion) = solid
        .advance_heated_vapor_loaded(
            0.01,
            &mut water,
            &mut gas,
            &mut thermal,
            4200.,
            100.,
            &links,
            &[1.],
            &[calibration()],
            &[[0.; 3]; 10],
            &[],
            &[],
            &[[0.; 3]; 10],
            [0.; 3],
            limits,
        )
        .unwrap();
    assert!(transfer.vapor_water_change_kg < 0.);
    assert!(!motion.substeps.is_empty());
    let mass = solid.energy().unwrap().mass_kg;
    assert!((mass + gas.water_kg() - 1.01).abs() < 1e-12);
    assert!(
        (gas.accounted_energy_j() + thermal.energy_j() - energy - bulk.kinetic_transfer_loss_j)
            .abs()
            < 1e-8
    );
    assert!(
        (gas.accounted_energy_j() + thermal.energy_j() + solid.energy().unwrap().kinetic_j
            - energy
            - 0.5)
            .abs()
            < 1e-8
    );
    assert!(bulk.energy_defect_j.abs() < 1e-12);
    for (old, new) in positions.iter().zip(solid.positions()) {
        assert!((new[0] - old[0] - 0.01 / mass).abs() < 1e-12);
    }
}

#[test]
fn temperature_changes_elastic_response_with_explicit_parameter_work() {
    use physics::moisture::ThermalCalibration;
    let cold = calibration();
    let hot = Calibration::new(
        Properties {
            young_pa: 5e4,
            ..cold.at(0.).unwrap()
        },
        Properties {
            young_pa: 2.5e4,
            ..cold.at(1.).unwrap()
        },
    )
    .unwrap();
    let law = ThermalCalibration::new(300., 600., cold, hot).unwrap();
    let mut body = fixture(false, true);
    let water = [Cell {
        capacity_kg: 1.,
        water_kg: 0.,
    }];
    let before = body.energy().unwrap();
    let report = body
        .apply_thermal_moisture(&water, &[1.], &[law], &[450.], &[[0.; 3]; 10])
        .unwrap();
    let after = body.energy().unwrap();
    assert!((after.elastic_j - 0.75 * before.elastic_j).abs() < 1e-12);
    assert!((report.elastic_parameter_work_j + 0.25 * before.elastic_j).abs() < 1e-12);
    assert!((after.mass_kg - before.mass_kg).abs() < 1e-12);
    let snapshot = format!("{body:?}");
    assert!(
        body.apply_thermal_moisture(&water, &[1.], &[law], &[601.], &[[0.; 3]; 10])
            .is_err()
    );
    assert_eq!(snapshot, format!("{body:?}"));
    assert!((law.at_temperature(450.).unwrap().at(0.5).unwrap().young_pa - 56250.).abs() < 1e-10);
}
