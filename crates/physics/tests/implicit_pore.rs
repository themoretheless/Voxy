use physics::biomechanics::*;
fn specimen() -> Body {
    let mut body = Body::new(
        vec![
            [0., 0., 0.],
            [0.01, 0., 0.],
            [0., 0.01, 0.],
            [0.006, 0.002, 0.012],
            [0.001, 0.005, -0.007],
        ],
        vec![true, true, true, false, false],
        vec![
            (
                [0, 1, 2, 3],
                Material::from_young_poisson(8000., 0.3).unwrap(),
            ),
            (
                [0, 1, 2, 4],
                Material::from_young_poisson(8000., 0.3).unwrap(),
            ),
        ],
    )
    .unwrap();
    let fluids = body
        .stresses_at(body.positions())
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let v = s.reference_volume_m3;
            PoreFluid {
                reference_fluid_volume_m3: 0.5 * v,
                fluid_volume_m3: 0.5 * v + if i == 0 { v * 0.001 } else { 0. },
                storage_m3_per_pa: v / 100_000.,
                biot_coefficient: 0.8,
            }
        })
        .collect();
    body.set_cell_pore_fluids(fluids).unwrap();
    body
}
fn config() -> ImplicitPoreConfig {
    ImplicitPoreConfig {
        outer_iterations: 200,
        pressure_tolerance_pa: 1e-6,
        relaxation: 0.5,
        solid_iterations: 4000,
        solid_tolerance_n: 1e-9,
    }
}
fn permeability() -> Vec<Matrix> {
    vec![[[1e-12, 0., 0.], [0., 1e-12, 0.], [0., 0., 1e-12]]; 2]
}
#[test]
fn coupled_backward_euler_satisfies_force_storage_and_inventory() {
    let mut body = specimen();
    let reference = body.positions().to_vec();
    let old: Vec<_> = body
        .cell_pore_fluids()
        .iter()
        .map(|f| f.fluid_volume_m3)
        .collect();
    let initial_energy = body.evaluate(body.positions()).unwrap().0;
    let report = body
        .implicit_cell_pore_step(&permeability(), 0.001, 0.1, config())
        .unwrap();
    assert!(report.solid.converged && report.solid.residual_n <= 1e-9);
    assert!(report.pressure_residual_pa <= 1e-6 && report.solid.min_j > 0.);
    assert_ne!(reference, body.positions());
    for ((f, old), q) in body
        .cell_pore_fluids()
        .iter()
        .zip(&old)
        .zip(&report.flow.cell_outflows_m3_per_s)
    {
        assert!(
            (f.fluid_volume_m3 - old + 0.1 * q).abs()
                <= 1.01e-6 * f.storage_m3_per_pa + 64. * f64::EPSILON * old
        );
    }
    assert!(
        (body
            .cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .sum::<f64>()
            - old.iter().sum::<f64>())
        .abs()
            < 1e-12 * old.iter().sum::<f64>()
    );
    assert!(body.evaluate(body.positions()).unwrap().0 < initial_energy);
}
#[test]
fn failed_coupling_preserves_geometry_and_local_storage() {
    let mut body = specimen();
    let positions = body.positions().to_vec();
    let fluids: Vec<_> = body
        .cell_pore_fluids()
        .iter()
        .map(|f| f.fluid_volume_m3)
        .collect();
    let before = body.evaluate(&positions).unwrap();
    let mut options = config();
    options.outer_iterations = 1;
    assert!(
        body.implicit_cell_pore_step(&permeability(), 0.001, 0.1, options)
            .is_err()
    );
    assert_eq!(body.positions(), positions);
    assert_eq!(body.evaluate(&positions).unwrap(), before);
    assert_eq!(
        body.cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .collect::<Vec<_>>(),
        fluids
    );
    assert!(
        body.implicit_cell_pore_step(&[], 0.001, 0.1, config())
            .is_err()
    );
    assert_eq!(body.positions(), positions);
}

#[test]
fn joint_step_conserves_protein_and_rolls_back_after_transport_failure() {
    let mut body = specimen();
    let mut protein: Vec<_> = body
        .cell_pore_fluids()
        .iter()
        .map(|f| 10. * f.fluid_volume_m3)
        .collect();
    let total: f64 = protein.iter().sum();
    body.implicit_cell_pore_protein_step(&mut protein, &permeability(), 0.001, 0.1, config())
        .unwrap();
    assert!((protein.iter().sum::<f64>() - total).abs() < 1e-12 * total);
    for (mass, fluid) in protein.iter().zip(body.cell_pore_fluids()) {
        assert!(*mass >= 0.);
        assert!((mass / fluid.fluid_volume_m3 - 10.).abs() < 1e-8);
    }
    let mut body = specimen();
    let before = body.clone();
    // The FEM/storage phase succeeds, but the subsequent protein phase rejects
    // this inventory. Its already-computed geometry must not escape the trial.
    let mut invalid = vec![-1., 1e-9];
    let original = invalid.clone();
    assert!(
        body.implicit_cell_pore_protein_step(&mut invalid, &permeability(), 0.001, 0.1, config())
            .is_err()
    );
    assert_eq!(invalid, original);
    assert_eq!(body.positions(), before.positions());
    assert_eq!(
        body.evaluate(body.positions()).unwrap(),
        before.evaluate(before.positions()).unwrap()
    );
    assert_eq!(
        body.cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .collect::<Vec<_>>(),
        before
            .cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .collect::<Vec<_>>()
    );
}

#[test]
fn deforming_finite_reservoir_joint_step_conserves_and_rolls_back() {
    let mut body = specimen();
    let mut protein: Vec<_> = body
        .cell_pore_fluids()
        .iter()
        .map(|f| 10. * f.fluid_volume_m3)
        .collect();
    let mut reservoir = PoreReservoir {
        reference_volume_m3: 2e-9,
        reference_pressure_pa: 0.,
        compliance_m3_per_pa: 1e-11,
        fluid_volume_m3: 3.5e-9,
        protein_kg: 7e-8,
    };
    let initial_water = body
        .cell_pore_fluids()
        .iter()
        .map(|f| f.fluid_volume_m3)
        .sum::<f64>()
        + reservoir.fluid_volume_m3;
    let initial_protein = protein.iter().sum::<f64>() + reservoir.protein_kg;
    let initial_positions = body.positions().to_vec();
    let initial_energy = body.evaluate(body.positions()).unwrap().0
        + 0.5 * reservoir.compliance_m3_per_pa * reservoir.pressure_pa().unwrap().powi(2);
    let report = body
        .implicit_cell_pore_reservoir_step(
            &mut protein,
            &mut reservoir,
            &[[0, 1, 3]],
            &permeability(),
            0.001,
            0.1,
            config(),
        )
        .unwrap();
    assert!(report.solid.converged && report.pressure_residual_pa <= 1e-6);
    assert_ne!(body.positions(), initial_positions);
    assert!(reservoir.pressure_pa().unwrap() < 150.);
    assert!(
        (body
            .cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .sum::<f64>()
            + reservoir.fluid_volume_m3
            - initial_water)
            .abs()
            < 1e-12 * initial_water
    );
    assert!(
        (protein.iter().sum::<f64>() + reservoir.protein_kg - initial_protein).abs()
            < 1e-12 * initial_protein
    );
    let final_energy = body.evaluate(body.positions()).unwrap().0
        + 0.5 * reservoir.compliance_m3_per_pa * reservoir.pressure_pa().unwrap().powi(2);
    assert!(final_energy < initial_energy);
    // Continue the accepted deforming state: a single-step balance would miss
    // accidentally reusing reference inventories or the initial compartment.
    let mut previous_energy = final_energy;
    for _ in 0..3 {
        let report = body
            .implicit_cell_pore_reservoir_step(
                &mut protein,
                &mut reservoir,
                &[[0, 1, 3]],
                &permeability(),
                0.001,
                0.1,
                config(),
            )
            .unwrap();
        assert!(report.solid.converged && report.pressure_residual_pa <= 1e-6);
        let water = body
            .cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .sum::<f64>()
            + reservoir.fluid_volume_m3;
        assert!((water - initial_water).abs() < 1e-12 * initial_water);
        assert!(
            (protein.iter().sum::<f64>() + reservoir.protein_kg - initial_protein).abs()
                < 1e-12 * initial_protein
        );
        assert!(protein.iter().all(|m| m.is_finite() && *m >= 0.));
        let energy = body.evaluate(body.positions()).unwrap().0
            + 0.5 * reservoir.compliance_m3_per_pa * reservoir.pressure_pa().unwrap().powi(2);
        assert!(energy <= previous_energy);
        previous_energy = energy;
    }
    protein[0] = -1.;
    let before = (
        body.positions().to_vec(),
        body.evaluate(body.positions()).unwrap(),
        protein.clone(),
        reservoir,
        body.cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .collect::<Vec<_>>(),
    );
    assert!(
        body.implicit_cell_pore_reservoir_step(
            &mut protein,
            &mut reservoir,
            &[[0, 1, 3]],
            &permeability(),
            0.001,
            0.1,
            config()
        )
        .is_err()
    );
    assert_eq!(
        (
            body.positions().to_vec(),
            body.evaluate(body.positions()).unwrap(),
            protein,
            reservoir,
            body.cell_pore_fluids()
                .iter()
                .map(|f| f.fluid_volume_m3)
                .collect::<Vec<_>>()
        ),
        before
    );
}

#[test]
fn prescribed_exterior_pressure_drives_deformation_and_conservative_protein() {
    let mut body = specimen();
    let mut protein: Vec<_> = body
        .cell_pore_fluids()
        .iter()
        .map(|f| 10. * f.fluid_volume_m3)
        .collect();
    let old_water = body
        .cell_pore_fluids()
        .iter()
        .map(|f| f.fluid_volume_m3)
        .sum::<f64>();
    let old_mass = protein.iter().sum::<f64>();
    let positions = body.positions().to_vec();
    let dt = 0.1;
    let report = body
        .implicit_cell_pore_boundary_protein_step(
            &mut protein,
            &[([0, 1, 3], 150.)],
            10.,
            &permeability(),
            0.001,
            dt,
            config(),
        )
        .unwrap();
    assert_ne!(body.positions(), positions);
    assert!(report.solid.converged && report.pressure_residual_pa <= 1e-6);
    let gain = body
        .cell_pore_fluids()
        .iter()
        .map(|f| f.fluid_volume_m3)
        .sum::<f64>()
        - old_water;
    assert!(gain > 0.);
    let flux_gain = -dt * report.flow.cell_outflows_m3_per_s.iter().sum::<f64>();
    let storage = body
        .cell_pore_fluids()
        .iter()
        .map(|f| f.storage_m3_per_pa)
        .sum::<f64>();
    assert!((gain - flux_gain).abs() < 1.1e-6 * storage);
    assert!((protein.iter().sum::<f64>() - old_mass - 10. * flux_gain).abs() < 1e-12 * old_mass);
    let before = (
        body.positions().to_vec(),
        body.evaluate(body.positions()).unwrap(),
        protein.clone(),
    );
    assert!(
        body.implicit_cell_pore_boundary_protein_step(
            &mut protein,
            &[([0, 1, 3], 150.)],
            -1.,
            &permeability(),
            0.001,
            dt,
            config()
        )
        .is_err()
    );
    assert_eq!(
        before,
        (
            body.positions().to_vec(),
            body.evaluate(body.positions()).unwrap(),
            protein
        )
    );
}

#[test]
fn simultaneous_vascular_tissue_step_matches_pressure_and_conserves_both_inventories() {
    use physics::circulation::{Circulation, Compartment, Vessel};
    let mut body = specimen();
    let mut blood = Circulation::new(
        vec![
            Compartment {
                unstressed_volume_m3: 9e-9,
                initial_volume_m3: 1e-8,
                initial_elastance_pa_per_m3: 1e11,
                initial_external_pressure_pa: 0.,
            },
            Compartment {
                unstressed_volume_m3: 9.8e-9,
                initial_volume_m3: 1e-8,
                initial_elastance_pa_per_m3: 1e11,
                initial_external_pressure_pa: 0.,
            },
        ],
        vec![Vessel {
            from: 0,
            to: 1,
            resistance: 1e12,
            quadratic_resistance: 0.,
            inertance: 0.,
            valve: false,
        }],
    )
    .unwrap();
    let mut tissue_mass: Vec<_> = body
        .cell_pore_fluids()
        .iter()
        .map(|f| 10. * f.fluid_volume_m3)
        .collect();
    let mut blood_mass = vec![2e-7, 1e-7];
    let water = body
        .cell_pore_fluids()
        .iter()
        .map(|f| f.fluid_volume_m3)
        .sum::<f64>()
        + blood.total_volume();
    let protein = tissue_mass.iter().chain(&blood_mass).sum::<f64>();
    let options = VascularPoreConfig {
        iterations: 100,
        relaxation: 0.5,
        pressure_tolerance_pa: 1e-6,
        concentration_tolerance_kg_per_m3: 1e-7,
        tissue: config(),
        circulation_iterations: 32,
        circulation_tolerance_m3: 1e-20,
    };
    for _ in 0..3 {
        let report = body
            .implicit_vascular_pore_step(
                &mut tissue_mass,
                &mut blood,
                &mut blood_mass,
                0,
                &[[0, 1, 3]],
                &permeability(),
                0.001,
                0.01,
                &[1e11; 2],
                &[0.; 2],
                options,
            )
            .unwrap();
        assert!(report.pressure_residual_pa <= 1e-6);
        assert!(report.concentration_residual_kg_per_m3 <= 1e-7);
        assert!(report.tissue.solid.converged);
        assert!(
            (body
                .cell_pore_fluids()
                .iter()
                .map(|f| f.fluid_volume_m3)
                .sum::<f64>()
                + blood.total_volume()
                - water)
                .abs()
                < 1e-10 * water
        );
        assert!(
            (tissue_mass.iter().chain(&blood_mass).sum::<f64>() - protein).abs() < 1e-12 * protein
        );
    }
    let before = (
        body.positions().to_vec(),
        body.evaluate(body.positions()).unwrap(),
        tissue_mass.clone(),
        blood.volumes().to_vec(),
        blood.pressures().to_vec(),
        blood.flows().to_vec(),
        blood_mass.clone(),
    );
    let mut failed = options;
    failed.iterations = 1;
    let mut observations = Vec::new();
    assert!(
        body.implicit_vascular_pore_step_with_observer(
            &mut tissue_mass,
            &mut blood,
            &mut blood_mass,
            0,
            &[[0, 1, 3]],
            &permeability(),
            0.001,
            0.01,
            &[1e11; 2],
            &[500., 0.],
            failed,
            |iteration, pressure, concentration| observations.push((
                iteration,
                pressure,
                concentration
            ))
        )
        .is_err()
    );
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].0, 1);
    assert!(observations[0].1.is_finite() && observations[0].1 > failed.pressure_tolerance_pa);
    assert!(observations[0].2.is_finite() && observations[0].2 >= 0.);
    assert_eq!(
        before,
        (
            body.positions().to_vec(),
            body.evaluate(body.positions()).unwrap(),
            tissue_mass,
            blood.volumes().to_vec(),
            blood.pressures().to_vec(),
            blood.flows().to_vec(),
            blood_mass
        )
    );
}

#[test]
fn simultaneous_fixed_tissue_matches_independent_three_storage_linear_system() {
    use physics::circulation::{Circulation, Compartment, Vessel};
    let mut body = Body::new(
        vec![[0., 0., 0.], [0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]],
        vec![true; 4],
        vec![(
            [0, 1, 2, 3],
            Material::from_young_poisson(8000., 0.3).unwrap(),
        )],
    )
    .unwrap();
    let volume = body.stresses_at(body.positions()).unwrap()[0].reference_volume_m3;
    let storage = volume / 100_000.;
    body.set_cell_pore_fluids(vec![PoreFluid {
        reference_fluid_volume_m3: 0.5 * volume,
        fluid_volume_m3: 0.5 * volume + 20. * storage,
        storage_m3_per_pa: storage,
        biot_coefficient: 0.8,
    }])
    .unwrap();
    let k = vec![permeability()[0]];
    let port = [0, 1, 3];
    let g = body
        .deformed_darcy_with_boundaries(&k, 0.001, &[(port, 0.)])
        .unwrap()
        .response(&[1.])
        .unwrap()
        .cell_outflows_m3_per_s[0];
    let c = 1e-11;
    let dt = 0.01;
    let h = 1e-12;
    let mut a = [
        [storage + dt * g, -dt * g, 0.],
        [-dt * g, c + dt * g + dt * h, -dt * h],
        [0., -dt * h, c + dt * h],
    ];
    let mut rhs = [20. * storage, 100. * c, 20. * c];
    for i in 0..3 {
        for j in i + 1..3 {
            let factor = a[j][i] / a[i][i];
            for column in i..3 {
                a[j][column] -= factor * a[i][column];
            }
            rhs[j] -= factor * rhs[i];
        }
    }
    let mut expected = [0.; 3];
    for i in (0..3).rev() {
        expected[i] = (rhs[i] - (i + 1..3).map(|j| a[i][j] * expected[j]).sum::<f64>()) / a[i][i];
    }
    let mut blood = Circulation::new(
        vec![
            Compartment {
                unstressed_volume_m3: 9e-9,
                initial_volume_m3: 1e-8,
                initial_elastance_pa_per_m3: 1. / c,
                initial_external_pressure_pa: 0.,
            },
            Compartment {
                unstressed_volume_m3: 9.8e-9,
                initial_volume_m3: 1e-8,
                initial_elastance_pa_per_m3: 1. / c,
                initial_external_pressure_pa: 0.,
            },
        ],
        vec![Vessel {
            from: 0,
            to: 1,
            resistance: 1. / h,
            quadratic_resistance: 0.,
            inertance: 0.,
            valve: false,
        }],
    )
    .unwrap();
    let mut tm = vec![10. * body.cell_pore_fluids()[0].fluid_volume_m3];
    let mut bm = vec![1e-7; 2];
    body.implicit_vascular_pore_step(
        &mut tm,
        &mut blood,
        &mut bm,
        0,
        &[port],
        &k,
        0.001,
        dt,
        &[1. / c; 2],
        &[0.; 2],
        VascularPoreConfig {
            iterations: 100,
            relaxation: 0.5,
            pressure_tolerance_pa: 1e-6,
            concentration_tolerance_kg_per_m3: 1e-7,
            tissue: config(),
            circulation_iterations: 32,
            circulation_tolerance_m3: 1e-20,
        },
    )
    .unwrap();
    let actual = [
        body.cell_pore_response_at(body.positions()).unwrap().0[0],
        blood.pressures()[0],
        blood.pressures()[1],
    ];
    for (a, b) in actual.iter().zip(expected) {
        assert!((a - b).abs() < 3e-6, "{actual:?} vs {expected:?}");
    }
    for (m, v) in tm
        .iter()
        .zip(body.cell_pore_fluids().iter().map(|f| f.fluid_volume_m3))
    {
        assert!((m / v - 10.).abs() < 1e-7);
    }
    for (m, v) in bm.iter().zip(blood.volumes()) {
        assert!((m / v - 10.).abs() < 1e-7);
    }
}

#[test]
fn distinct_exterior_ports_use_their_own_inflow_and_accepted_outflow_concentration() {
    for pressures in [[150., -150.], [-150., 150.]] {
        let mut body = specimen();
        let mut protein: Vec<_> = body
            .cell_pore_fluids()
            .iter()
            .map(|f| 10. * f.fluid_volume_m3)
            .collect();
        let old = protein.iter().sum::<f64>();
        // Reordered vertex identities exercise the canonical port lookup.
        let boundaries = [([3, 0, 1], pressures[0]), ([4, 1, 0], pressures[1])];
        let dt = 0.1;
        let report = body
            .implicit_cell_pore_boundary_protein_step_with_concentrations(
                &mut protein,
                &boundaries,
                &[20., 5.],
                &permeability(),
                0.001,
                dt,
                config(),
            )
            .unwrap();
        let model = body
            .deformed_darcy_with_boundaries(&permeability(), 0.001, &boundaries)
            .unwrap();
        let mut ledger = 0.;
        let mut inflow = false;
        let mut outflow = false;
        for (face, q) in model.faces().iter().zip(&report.flow.face_flows_m3_per_s) {
            if face.neighbor.is_some() {
                continue;
            }
            let concentration = if *q < 0. {
                inflow = true;
                if face.owner == 0 { 20. } else { 5. }
            } else {
                outflow = true;
                protein[face.owner] / body.cell_pore_fluids()[face.owner].fluid_volume_m3
            };
            ledger -= dt * q * concentration;
        }
        assert!(inflow && outflow);
        assert!((protein.iter().sum::<f64>() - old - ledger).abs() < 1e-12 * old);
        let before = (
            body.positions().to_vec(),
            body.evaluate(body.positions()).unwrap(),
            protein.clone(),
        );
        for concentrations in [vec![20.], vec![20., f64::NAN]] {
            assert!(
                body.implicit_cell_pore_boundary_protein_step_with_concentrations(
                    &mut protein,
                    &boundaries,
                    &concentrations,
                    &permeability(),
                    0.001,
                    dt,
                    config()
                )
                .is_err()
            );
        }
        assert_eq!(
            before,
            (
                body.positions().to_vec(),
                body.evaluate(body.positions()).unwrap(),
                protein
            )
        );
    }
}

#[test]
fn independent_vascular_ports_preserve_local_and_joint_water_protein_ledgers() {
    use physics::circulation::{Circulation, Compartment, Vessel};
    let mut body = specimen();
    let mut blood = Circulation::new(
        vec![
            Compartment {
                unstressed_volume_m3: 9e-9,
                initial_volume_m3: 1e-8,
                initial_elastance_pa_per_m3: 1e11,
                initial_external_pressure_pa: 0.,
            },
            Compartment {
                unstressed_volume_m3: 9.8e-9,
                initial_volume_m3: 1e-8,
                initial_elastance_pa_per_m3: 1e11,
                initial_external_pressure_pa: -170.,
            },
        ],
        vec![Vessel {
            from: 0,
            to: 1,
            resistance: 1e12,
            quadratic_resistance: 0.,
            inertance: 0.,
            valve: false,
        }],
    )
    .unwrap();
    let mut tissue_mass: Vec<_> = body
        .cell_pore_fluids()
        .iter()
        .map(|f| 10. * f.fluid_volume_m3)
        .collect();
    let mut blood_mass = vec![2e-7, 5e-8];
    let water = body
        .cell_pore_fluids()
        .iter()
        .map(|f| f.fluid_volume_m3)
        .sum::<f64>()
        + blood.total_volume();
    let protein = tissue_mass.iter().chain(&blood_mass).sum::<f64>();
    let options = VascularPoreConfig {
        iterations: 100,
        relaxation: 0.5,
        pressure_tolerance_pa: 1e-6,
        concentration_tolerance_kg_per_m3: 1e-7,
        tissue: config(),
        circulation_iterations: 32,
        circulation_tolerance_m3: 1e-20,
    };
    let ports = [
        VascularPorePort {
            nodes: [3, 0, 1],
            compartment: 0,
        },
        VascularPorePort {
            nodes: [4, 1, 0],
            compartment: 1,
        },
    ];
    for _ in 0..3 {
        let old_volumes = blood.volumes().to_vec();
        let old_mass = blood_mass.clone();
        let dt = 0.01;
        let report = body
            .implicit_vascular_pore_ports_step_with_observer(
                &mut tissue_mass,
                &mut blood,
                &mut blood_mass,
                &ports,
                &permeability(),
                0.001,
                dt,
                &[1e11; 2],
                &[0., -170.],
                options,
                |_, _, _| {},
            )
            .unwrap();
        assert!(report.pressure_residual_pa <= options.pressure_tolerance_pa);
        assert!(
            report.concentration_residual_kg_per_m3 <= options.concentration_tolerance_kg_per_m3
        );
        let boundaries: Vec<_> = ports
            .iter()
            .map(|p| (p.nodes, blood.pressures()[p.compartment]))
            .collect();
        let model = body
            .deformed_darcy_with_boundaries(&permeability(), 0.001, &boundaries)
            .unwrap();
        let mut dv = [0.; 2];
        let mut dm = [0.; 2];
        for (face, q) in model
            .faces()
            .iter()
            .zip(&report.tissue.flow.face_flows_m3_per_s)
        {
            if face.neighbor.is_some() {
                continue;
            }
            let compartment = face.owner;
            if compartment == 0 {
                assert!(*q < 0.);
            } else {
                assert!(*q > 0.);
            }
            let concentration = if *q < 0. {
                blood_mass[compartment] / blood.volumes()[compartment]
            } else {
                tissue_mass[face.owner] / body.cell_pore_fluids()[face.owner].fluid_volume_m3
            };
            dv[compartment] += dt * q;
            dm[compartment] += dt * q * concentration;
        }
        let q = blood.flows()[0];
        let donor = if q >= 0. { 0 } else { 1 };
        let transported_mass = dt * q * blood_mass[donor] / blood.volumes()[donor];
        for i in 0..2 {
            let sign = if i == 0 { -1. } else { 1. };
            assert!((blood.volumes()[i] - old_volumes[i] - dv[i] - sign * dt * q).abs() < 1e-19);
            assert!(
                (blood_mass[i] - old_mass[i] - dm[i] - sign * transported_mass).abs()
                    < 1e-12 * protein
            );
        }
        assert!(
            (body
                .cell_pore_fluids()
                .iter()
                .map(|f| f.fluid_volume_m3)
                .sum::<f64>()
                + blood.total_volume()
                - water)
                .abs()
                < 1e-10 * water
        );
        assert!(
            (tissue_mass.iter().chain(&blood_mass).sum::<f64>() - protein).abs() < 1e-12 * protein
        );
    }
    let before = (
        body.positions().to_vec(),
        body.evaluate(body.positions()).unwrap(),
        tissue_mass.clone(),
        blood.volumes().to_vec(),
        blood.pressures().to_vec(),
        blood.flows().to_vec(),
        blood_mass.clone(),
    );
    let duplicate = [
        ports[0],
        VascularPorePort {
            nodes: [0, 1, 3],
            compartment: 1,
        },
    ];
    assert!(
        body.implicit_vascular_pore_ports_step_with_observer(
            &mut tissue_mass,
            &mut blood,
            &mut blood_mass,
            &duplicate,
            &permeability(),
            0.001,
            0.01,
            &[1e11; 2],
            &[0., -170.],
            options,
            |_, _, _| {}
        )
        .is_err()
    );
    assert_eq!(
        before,
        (
            body.positions().to_vec(),
            body.evaluate(body.positions()).unwrap(),
            tissue_mass,
            blood.volumes().to_vec(),
            blood.pressures().to_vec(),
            blood.flows().to_vec(),
            blood_mass
        )
    );
}

#[test]
fn hydraulic_interfaces_limit_deforming_vascular_exchange_and_preserve_balances() {
    use physics::circulation::{Circulation, Compartment, Vessel};
    let mut body = specimen();
    let mut blood = Circulation::new(
        vec![
            Compartment {
                unstressed_volume_m3: 9e-9,
                initial_volume_m3: 1e-8,
                initial_elastance_pa_per_m3: 1e11,
                initial_external_pressure_pa: 0.,
            },
            Compartment {
                unstressed_volume_m3: 9.8e-9,
                initial_volume_m3: 1e-8,
                initial_elastance_pa_per_m3: 1e11,
                initial_external_pressure_pa: -170.,
            },
        ],
        vec![Vessel {
            from: 0,
            to: 1,
            resistance: 1e12,
            quadratic_resistance: 0.,
            inertance: 0.,
            valve: false,
        }],
    )
    .unwrap();
    let mut tissue_mass: Vec<_> = body
        .cell_pore_fluids()
        .iter()
        .map(|f| 10. * f.fluid_volume_m3)
        .collect();
    let mut blood_mass = vec![2e-7, 5e-8];
    let water = body
        .cell_pore_fluids()
        .iter()
        .map(|f| f.fluid_volume_m3)
        .sum::<f64>()
        + blood.total_volume();
    let protein = tissue_mass.iter().chain(&blood_mass).sum::<f64>();
    let options = VascularPoreConfig {
        iterations: 100,
        relaxation: 0.5,
        pressure_tolerance_pa: 1e-6,
        concentration_tolerance_kg_per_m3: 1e-7,
        tissue: config(),
        circulation_iterations: 32,
        circulation_tolerance_m3: 1e-20,
    };
    let ports = [
        VascularPorePort {
            nodes: [3, 0, 1],
            compartment: 0,
        },
        VascularPorePort {
            nodes: [4, 1, 0],
            compartment: 1,
        },
    ];
    let original_body = body.clone();
    let original_blood = blood.clone();
    let original_tm = tissue_mass.clone();
    let original_bm = blood_mass.clone();
    let mut baseline = Vec::new();
    for resistance in [0., 1e15] {
        body = original_body.clone();
        blood = original_blood.clone();
        tissue_mass.clone_from(&original_tm);
        blood_mass.clone_from(&original_bm);
        let resistances: Vec<_> = ports.iter().map(|p| (p.nodes, resistance)).collect();
        let report = body
            .implicit_vascular_pore_ports_step_with_resistances_and_observer(
                &mut tissue_mass,
                &mut blood,
                &mut blood_mass,
                &ports,
                &resistances,
                &permeability(),
                0.001,
                0.01,
                &[1e11; 2],
                &[0., -170.],
                options,
                |_, _, _| {},
            )
            .unwrap();
        let boundaries: Vec<_> = ports
            .iter()
            .map(|p| (p.nodes, blood.pressures()[p.compartment]))
            .collect();
        let model = body
            .deformed_darcy_with_boundaries(&permeability(), 0.001, &boundaries)
            .unwrap();
        let flows: Vec<_> = model
            .faces()
            .iter()
            .zip(&report.tissue.flow.face_flows_m3_per_s)
            .filter(|(f, _)| f.neighbor.is_none())
            .map(|(_, q)| q.abs())
            .collect();
        if resistance == 0. {
            baseline = flows;
        } else {
            assert!(flows.iter().zip(&baseline).all(|(q, old)| *q < 0.01 * old));
        }
        assert!(
            report.tissue.solid.converged
                && report.pressure_residual_pa <= options.pressure_tolerance_pa
        );
        assert!(
            (body
                .cell_pore_fluids()
                .iter()
                .map(|f| f.fluid_volume_m3)
                .sum::<f64>()
                + blood.total_volume()
                - water)
                .abs()
                < 1e-10 * water
        );
        assert!(
            (tissue_mass.iter().chain(&blood_mass).sum::<f64>() - protein).abs() < 1e-12 * protein
        );
    }
    let before = (
        body.positions().to_vec(),
        body.evaluate(body.positions()).unwrap(),
        tissue_mass.clone(),
        blood.volumes().to_vec(),
        blood.pressures().to_vec(),
        blood.flows().to_vec(),
        blood_mass.clone(),
    );
    assert!(
        body.implicit_vascular_pore_ports_step_with_resistances_and_observer(
            &mut tissue_mass,
            &mut blood,
            &mut blood_mass,
            &ports,
            &[(ports[0].nodes, -1.)],
            &permeability(),
            0.001,
            0.01,
            &[1e11; 2],
            &[0., -170.],
            options,
            |_, _, _| {}
        )
        .is_err()
    );
    assert_eq!(
        before,
        (
            body.positions().to_vec(),
            body.evaluate(body.positions()).unwrap(),
            tissue_mass,
            blood.volumes().to_vec(),
            blood.pressures().to_vec(),
            blood.flows().to_vec(),
            blood_mass
        )
    );
}

#[test]
fn selective_vascular_membranes_retain_or_diffuse_protein_with_joint_conservation() {
    use physics::circulation::{Circulation, Compartment, Vessel};
    let mut body = specimen();
    let mut blood = Circulation::new(
        vec![
            Compartment {
                unstressed_volume_m3: 9e-9,
                initial_volume_m3: 1e-8,
                initial_elastance_pa_per_m3: 1e11,
                initial_external_pressure_pa: 0.,
            },
            Compartment {
                unstressed_volume_m3: 9.8e-9,
                initial_volume_m3: 1e-8,
                initial_elastance_pa_per_m3: 1e11,
                initial_external_pressure_pa: -170.,
            },
        ],
        vec![Vessel {
            from: 0,
            to: 1,
            resistance: 1e12,
            quadratic_resistance: 0.,
            inertance: 0.,
            valve: false,
        }],
    )
    .unwrap();
    let mut tissue_mass: Vec<_> = body
        .cell_pore_fluids()
        .iter()
        .map(|f| 10. * f.fluid_volume_m3)
        .collect();
    let mut blood_mass = vec![2e-7, 5e-8];
    let water = body
        .cell_pore_fluids()
        .iter()
        .map(|f| f.fluid_volume_m3)
        .sum::<f64>()
        + blood.total_volume();
    let protein = tissue_mass.iter().chain(&blood_mass).sum::<f64>();
    let options = VascularPoreConfig {
        iterations: 100,
        relaxation: 0.5,
        pressure_tolerance_pa: 1e-6,
        concentration_tolerance_kg_per_m3: 1e-7,
        tissue: config(),
        circulation_iterations: 32,
        circulation_tolerance_m3: 1e-20,
    };
    let ports = [
        VascularPorePort {
            nodes: [3, 0, 1],
            compartment: 0,
        },
        VascularPorePort {
            nodes: [4, 1, 0],
            compartment: 1,
        },
    ];
    let initial_body = body.clone();
    let initial_blood = blood.clone();
    let initial_tm = tissue_mass.clone();
    let initial_bm = blood_mass.clone();
    let tissue_total = tissue_mass.iter().sum::<f64>();
    for diffusion in [0., 1e-12] {
        body = initial_body.clone();
        blood = initial_blood.clone();
        tissue_mass.clone_from(&initial_tm);
        blood_mass.clone_from(&initial_bm);
        let membranes: Vec<_> = ports.iter().map(|p| (p.nodes, 1., diffusion)).collect();
        for _ in 0..3 {
            let report = body
                .implicit_vascular_pore_ports_step_with_membranes_and_observer(
                    &mut tissue_mass,
                    &mut blood,
                    &mut blood_mass,
                    &ports,
                    &[],
                    &membranes,
                    &permeability(),
                    0.001,
                    0.01,
                    &[1e11; 2],
                    &[0., -170.],
                    options,
                    |_, _, _| {},
                )
                .unwrap();
            assert!(report.tissue.solid.converged);
            assert!(
                (tissue_mass.iter().chain(&blood_mass).sum::<f64>() - protein).abs()
                    < 1e-12 * protein
            );
            assert!(
                (body
                    .cell_pore_fluids()
                    .iter()
                    .map(|f| f.fluid_volume_m3)
                    .sum::<f64>()
                    + blood.total_volume()
                    - water)
                    .abs()
                    < 1e-10 * water
            );
            assert!(
                tissue_mass
                    .iter()
                    .chain(&blood_mass)
                    .all(|m| m.is_finite() && *m >= 0.)
            );
        }
        if diffusion == 0. {
            assert!((tissue_mass.iter().sum::<f64>() - tissue_total).abs() < 1e-12 * tissue_total);
        } else {
            assert!(tissue_mass.iter().sum::<f64>() > tissue_total);
        }
    }
    let before = (
        body.positions().to_vec(),
        body.evaluate(body.positions()).unwrap(),
        tissue_mass.clone(),
        blood.volumes().to_vec(),
        blood.pressures().to_vec(),
        blood.flows().to_vec(),
        blood_mass.clone(),
    );
    assert!(
        body.implicit_vascular_pore_ports_step_with_membranes_and_observer(
            &mut tissue_mass,
            &mut blood,
            &mut blood_mass,
            &ports,
            &[],
            &[(ports[0].nodes, 1.1, 0.)],
            &permeability(),
            0.001,
            0.01,
            &[1e11; 2],
            &[0., -170.],
            options,
            |_, _, _| {}
        )
        .is_err()
    );
    assert_eq!(
        before,
        (
            body.positions().to_vec(),
            body.evaluate(body.positions()).unwrap(),
            tissue_mass,
            blood.volumes().to_vec(),
            blood.pressures().to_vec(),
            blood.flows().to_vec(),
            blood_mass
        )
    );
}

#[test]
fn osmotic_vascular_exchange_matches_accepted_pressure_law_and_conserves() {
    use physics::circulation::{Circulation, Compartment, Vessel};
    // Hydro-osmotic equilibrium, then either sign of an isolated protein gradient.
    for (mut blood_pressure, blood_concentration, expected_sign, dt, law) in
        [(100., 20., 0.), (20., 20., 1.), (20., 5., -1.)]
            .into_iter()
            .flat_map(|(p, c, sign)| [0.001, 0.01, 0.1].map(move |dt| (p, c, sign, dt)))
            .flat_map(|(p, c, sign, dt)| {
                [
                    OsmoticPressureLaw {
                        linear: 8.,
                        quadratic: 0.,
                        cubic: 0.,
                    },
                    OsmoticPressureLaw {
                        linear: 8.,
                        quadratic: 0.1,
                        cubic: 0.001,
                    },
                ]
                .map(move |law| (p, c, sign, dt, law))
            })
    {
        // Direct polynomial reference, independent of the law evaluator.
        let pi = |c: f64| law.linear * c + law.quadratic * c * c + law.cubic * c * c * c;
        if expected_sign == 0. {
            blood_pressure = 20. + pi(blood_concentration) - pi(10.);
        }
        let mut body = Body::new(
            vec![[0., 0., 0.], [0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]],
            vec![true; 4],
            vec![(
                [0, 1, 2, 3],
                Material::from_young_poisson(8000., 0.3).unwrap(),
            )],
        )
        .unwrap();
        let v = body.stresses_at(body.positions()).unwrap()[0].reference_volume_m3;
        body.set_cell_pore_fluids(vec![PoreFluid {
            reference_fluid_volume_m3: 0.5 * v,
            fluid_volume_m3: 0.5 * v + 20. * v / 100_000.,
            storage_m3_per_pa: v / 100_000.,
            biot_coefficient: 0.8,
        }])
        .unwrap();
        let mut blood = Circulation::new(
            (0..2)
                .map(|_| Compartment {
                    unstressed_volume_m3: 1e-8 - blood_pressure * 1e-11,
                    initial_volume_m3: 1e-8,
                    initial_elastance_pa_per_m3: 1e11,
                    initial_external_pressure_pa: 0.,
                })
                .collect(),
            vec![Vessel {
                from: 0,
                to: 1,
                resistance: 1e12,
                quadratic_resistance: 0.,
                inertance: 0.,
                valve: false,
            }],
        )
        .unwrap();
        let old_fluid = body.cell_pore_fluids()[0].fluid_volume_m3;
        let mut tm = vec![10. * old_fluid];
        let mut bm = vec![blood_concentration * 1e-8; 2];
        let water = old_fluid + blood.total_volume();
        let mass = tm.iter().chain(&bm).sum::<f64>();
        let k = vec![permeability()[0]];
        let port = [0, 1, 3];
        let mut cfg = VascularPoreConfig {
            iterations: 200,
            relaxation: 0.5,
            pressure_tolerance_pa: 1e-7,
            concentration_tolerance_kg_per_m3: 1e-9,
            tissue: config(),
            circulation_iterations: 32,
            circulation_tolerance_m3: 1e-20,
        };
        cfg.tissue.pressure_tolerance_pa = 1e-9;
        let report = body
            .implicit_vascular_pore_ports_step_with_osmotic_law_and_observer(
                &mut tm,
                &mut blood,
                &mut bm,
                &[VascularPorePort {
                    nodes: port,
                    compartment: 0,
                }],
                &[],
                &[(port, 1., 0.)],
                law,
                &k,
                0.001,
                dt,
                &[1e11; 2],
                &[0.; 2],
                cfg,
                |_, _, _| {},
            )
            .unwrap();
        let now = body.cell_pore_fluids()[0].fluid_volume_m3;
        assert!((now + blood.total_volume() - water).abs() < 1e-12 * water);
        assert!((tm.iter().chain(&bm).sum::<f64>() - mass).abs() < 1e-12 * mass);
        assert!((tm[0] - 10. * old_fluid).abs() < 1e-12 * mass);
        let q = report.tissue.flow.cell_outflows_m3_per_s[0];
        if expected_sign == 0. {
            assert!(q.abs() < 1e-20, "equilibrium flux {q}");
        } else {
            assert!(q * expected_sign > 0., "gradient sign {q}");
        }
        // Independent Darcy response at accepted concentrations checks the feedback,
        // rather than merely checking outer iteration residuals.
        let effective = blood.pressures()[0] - (pi(bm[0] / blood.volumes()[0]) - pi(tm[0] / now));
        let pressure = body.cell_pore_response_at(body.positions()).unwrap().0;
        let expected = body
            .deformed_darcy_with_boundaries(&k, 0.001, &[(port, effective)])
            .unwrap()
            .response(&pressure)
            .unwrap()
            .cell_outflows_m3_per_s[0];
        let conductance = body
            .deformed_darcy_with_boundaries(&k, 0.001, &[(port, 0.)])
            .unwrap()
            .response(&[1.])
            .unwrap()
            .cell_outflows_m3_per_s[0];
        assert!(
            (q - expected).abs() <= conductance * cfg.pressure_tolerance_pa,
            "accepted law {q} vs {expected}"
        );
        // Independently eliminate the two vascular storage equations. x is water
        // transferred tissue -> blood, y is blood 0 -> blood 1 during this step.
        // Full reflection retains tissue protein, while vessel advection moves it.
        let storage = v / 100_000.;
        let a = dt * 1e-12 / 1e-11;
        let state = |x: f64| {
            let y = a * x / (1. + 2. * a);
            let v0 = 1e-8 + x - y;
            let v1 = 1e-8 + y;
            let m = blood_concentration * 1e-8;
            let m0 = if y >= 0. {
                m / (1. + y / v0)
            } else {
                2. * m - m / (1. - y / v1)
            };
            let pt = 20. - x / storage;
            let pb = blood_pressure + (x - y) / 1e-11;
            let drive = pt - pb + (pi(m0 / v0) - pi(10. * old_fluid / (old_fluid - x)));
            (x - dt * conductance * drive, pt, pb, v0, v1, m0)
        };
        let mut low = -0.5e-8;
        let mut high = 0.5 * old_fluid;
        assert!(state(low).0 < 0. && state(high).0 > 0.);
        for _ in 0..100 {
            let mid = 0.5 * (low + high);
            if state(mid).0 > 0. {
                high = mid;
            } else {
                low = mid;
            }
        }
        let x = 0.5 * (low + high);
        let (_, pt, pb, v0, v1, m0) = state(x);
        let volume_tolerance = 4. * storage * cfg.pressure_tolerance_pa;
        assert!(
            (old_fluid - now - x).abs() < volume_tolerance,
            "independent water root: {} vs {x}, dt={dt}",
            old_fluid - now
        );
        assert!((pressure[0] - pt).abs() < 4. * cfg.pressure_tolerance_pa);
        assert!((blood.pressures()[0] - pb).abs() < 4. * cfg.pressure_tolerance_pa);
        assert!((blood.volumes()[0] - v0).abs() < volume_tolerance);
        assert!((blood.volumes()[1] - v1).abs() < volume_tolerance);
        assert!(
            (bm[0] / blood.volumes()[0] - m0 / v0).abs()
                < 4. * cfg.concentration_tolerance_kg_per_m3
        );
        let before = (
            body.positions().to_vec(),
            now,
            tm.clone(),
            blood.volumes().to_vec(),
            blood.pressures().to_vec(),
            blood.flows().to_vec(),
            bm.clone(),
        );
        for invalid in [
            OsmoticPressureLaw {
                linear: 0.,
                quadratic: -1.,
                cubic: 0.,
            },
            OsmoticPressureLaw {
                linear: 0.,
                quadratic: 0.,
                cubic: f64::NAN,
            },
            OsmoticPressureLaw {
                linear: 0.,
                quadratic: f64::MAX,
                cubic: 0.,
            },
        ] {
            assert!(
                body.implicit_vascular_pore_ports_step_with_osmotic_law_and_observer(
                    &mut tm,
                    &mut blood,
                    &mut bm,
                    &[VascularPorePort {
                        nodes: port,
                        compartment: 0
                    }],
                    &[],
                    &[(port, 1., 0.)],
                    invalid,
                    &k,
                    0.001,
                    dt,
                    &[1e11; 2],
                    &[0.; 2],
                    cfg,
                    |_, _, _| {},
                )
                .is_err()
            );
            assert_eq!(
                before,
                (
                    body.positions().to_vec(),
                    body.cell_pore_fluids()[0].fluid_volume_m3,
                    tm.clone(),
                    blood.volumes().to_vec(),
                    blood.pressures().to_vec(),
                    blood.flows().to_vec(),
                    bm.clone()
                )
            );
        }
        for slope in [-1., f64::NAN, 8.] {
            let mut failure = cfg;
            failure.iterations = 1;
            assert!(
                body.implicit_vascular_pore_ports_step_with_osmosis_and_observer(
                    &mut tm,
                    &mut blood,
                    &mut bm,
                    &[VascularPorePort {
                        nodes: port,
                        compartment: 0
                    }],
                    &[],
                    &[(port, 1., 0.)],
                    slope,
                    &k,
                    0.001,
                    dt,
                    &[1e11; 2],
                    &[30.; 2],
                    failure,
                    |_, _, _| {},
                )
                .is_err()
            );
            assert_eq!(
                before,
                (
                    body.positions().to_vec(),
                    body.cell_pore_fluids()[0].fluid_volume_m3,
                    tm.clone(),
                    blood.volumes().to_vec(),
                    blood.pressures().to_vec(),
                    blood.flows().to_vec(),
                    bm.clone()
                )
            );
        }
    }
}

#[test]
fn osmotic_polynomial_checks_units_monotonicity_and_overflow() {
    let law = OsmoticPressureLaw {
        linear: 2.,
        quadratic: 3.,
        cubic: 4.,
    };
    assert_eq!(law.pressure_pa(0.).unwrap(), 0.);
    assert_eq!(law.pressure_pa(2.).unwrap(), 48.);
    let values: Vec<_> = (0..100)
        .map(|i| law.pressure_pa(f64::from(i)).unwrap())
        .collect();
    assert!(values.windows(2).all(|p| p[1] > p[0]));
    assert!(law.pressure_pa(-1.).is_err());
    assert!(law.pressure_pa(f64::MAX).is_err());
    for invalid in [f64::NAN, f64::INFINITY, -1.] {
        for coefficients in [[invalid, 0., 0.], [0., invalid, 0.], [0., 0., invalid]] {
            assert!(
                OsmoticPressureLaw {
                    linear: coefficients[0],
                    quadratic: coefficients[1],
                    cubic: coefficients[2]
                }
                .pressure_pa(0.)
                .is_err()
            );
        }
    }
}
