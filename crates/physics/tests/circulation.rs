use physics::circulation::*;
fn compartment(volume: f64, unstressed: f64, elastance: f64) -> Compartment {
    Compartment {
        initial_volume_m3: volume,
        unstressed_volume_m3: unstressed,
        initial_elastance_pa_per_m3: elastance,
        initial_external_pressure_pa: 0.,
    }
}
fn vessel(valve: bool) -> Vessel {
    Vessel {
        from: 0,
        to: 1,
        resistance: 2e5,
        quadratic_resistance: 0.,
        inertance: 0.,
        valve,
    }
}
fn pair(valve: bool) -> Circulation {
    Circulation::new(
        vec![
            compartment(0.0012, 0.001, 1e6),
            compartment(0.001, 0.001, 1e6),
        ],
        vec![vessel(valve)],
    )
    .unwrap()
}
#[test]
fn closed_two_compartments_match_discrete_rc_exchange_and_conserve_volume() {
    let mut blood = pair(false);
    let total = blood.total_volume();
    let mut difference = 200.;
    let dt = 0.02;
    for _ in 0..50 {
        difference /= 1. + dt / 0.1;
        let old_energy = blood.elastic_energy().unwrap();
        let report = blood.step(dt, &[1e6; 2], &[0.; 2], 32, 1e-14).unwrap();
        assert!((blood.pressures()[0] - blood.pressures()[1] - difference).abs() < 1e-8);
        assert!((blood.total_volume() - total).abs() < 1e-12);
        assert!(old_energy - blood.elastic_energy().unwrap() + 1e-10 >= report.resistive_loss_j);
    }
}
#[test]
fn ideal_valve_blocks_reverse_pressure_and_allows_forward_flow() {
    let mut blood = pair(true);
    let old = blood.volumes().to_vec();
    blood.step(0.1, &[1e6; 2], &[0., 500.], 32, 1e-14).unwrap();
    assert_eq!(blood.flows()[0], 0.);
    assert_eq!(blood.volumes(), old);
    blood.step(0.1, &[1e6; 2], &[500., 0.], 32, 1e-14).unwrap();
    assert!(blood.flows()[0] > 0.);
    assert!(blood.volumes()[0] < old[0]);
}
#[test]
fn inertial_and_quadratic_flow_satisfy_discrete_momentum_equation() {
    let edge = Vessel {
        quadratic_resistance: 5e9,
        inertance: 2e4,
        ..vessel(false)
    };
    let mut blood = Circulation::new(
        vec![
            compartment(0.0012, 0.001, 1e6),
            compartment(0.001, 0.001, 1e6),
        ],
        vec![edge],
    )
    .unwrap();
    let mut old_flow = 0.;
    for _ in 0..100 {
        let old_energy =
            blood.elastic_energy().unwrap() + 0.5 * edge.inertance * old_flow * old_flow;
        let report = blood.step(0.005, &[1e6; 2], &[0.; 2], 32, 1e-14).unwrap();
        let flow = blood.flows()[0];
        let pressure = blood.pressures()[0] - blood.pressures()[1];
        let expected = edge.resistance * flow
            + edge.quadratic_resistance * flow.abs() * flow
            + edge.inertance * (flow - old_flow) / 0.005;
        assert!(
            (expected - pressure).abs() < 1e-7,
            "{expected} vs {pressure}"
        );
        assert!(report.resistive_loss_j >= 0. && report.flow_kinetic_energy_j >= 0.);
        assert!(
            old_energy - blood.elastic_energy().unwrap() - report.flow_kinetic_energy_j + 1e-10
                >= report.resistive_loss_j
        );
        old_flow = flow;
    }
}
#[test]
fn gauge_invariance_and_negative_bidirectional_flow() {
    let mut a = pair(false);
    let mut b = pair(false);
    a.step(0.1, &[1e6; 2], &[0., 500.], 32, 1e-14).unwrap();
    b.step(0.1, &[1e6; 2], &[2000., 2500.], 32, 1e-14).unwrap();
    assert!(a.flows()[0] < 0.);
    for i in 0..2 {
        assert!((a.volumes()[i] - b.volumes()[i]).abs() < 1e-12);
    }
}
fn pump() -> Circulation {
    Circulation::new(
        vec![
            compartment(0.00012, 0.00002, 1e7),
            compartment(0.0007, 0.0006, 1e8),
            compartment(0.00418, 0.00318, 5e5),
        ],
        vec![
            Vessel {
                from: 0,
                to: 1,
                resistance: 1e6,
                quadratic_resistance: 5e10,
                inertance: 1e4,
                valve: true,
            },
            Vessel {
                from: 1,
                to: 2,
                resistance: 1.6e8,
                quadratic_resistance: 0.,
                inertance: 0.,
                valve: false,
            },
            Vessel {
                from: 2,
                to: 0,
                resistance: 1e6,
                quadratic_resistance: 5e10,
                inertance: 1e4,
                valve: true,
            },
        ],
    )
    .unwrap()
}
#[test]
fn pulsatile_chamber_drives_closed_loop_without_creating_blood() {
    let mut blood = pump();
    let total = blood.total_volume();
    let mut transferred = 0.;
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    for step in 1..=2000 {
        let t = f64::from(step) * 0.005;
        let phase = (t % 0.8) / 0.8;
        let activation = if phase < 0.4 {
            (std::f64::consts::PI * phase / 0.4).sin().powi(2)
        } else {
            0.
        };
        let report = blood
            .step(
                0.005,
                &[1e7 + 2e8 * activation, 1e8, 5e5],
                &[0.; 3],
                32,
                1e-13,
            )
            .unwrap();
        assert!(report.residual_m3 <= 1e-13);
        assert!((blood.total_volume() - total).abs() < 1e-10);
        assert!(blood.flows()[0] >= 0. && blood.flows()[2] >= 0.);
        if step > 1600 {
            transferred += 0.005 * blood.flows()[0];
            low = low.min(blood.volumes()[0]);
            high = high.max(blood.volumes()[0]);
        }
    }
    assert!(transferred > 0.00005);
    assert!(high - low > 0.00001);
}
#[test]
fn invalid_input_and_nonconvergence_preserve_all_state() {
    let mut blood = pump();
    let volumes = blood.volumes().to_vec();
    let flows = blood.flows().to_vec();
    let pressure = blood.pressures().to_vec();
    assert!(
        blood
            .step(0.1, &[2e8, 1e8, 5e5], &[0.; 3], 1, 1e-20)
            .is_err()
    );
    assert_eq!(volumes, blood.volumes());
    assert_eq!(flows, blood.flows());
    assert_eq!(pressure, blood.pressures());
    assert!(
        blood
            .step(f64::NAN, &[1e7, 1e8, 5e5], &[0.; 3], 32, 1e-13)
            .is_err()
    );
    assert_eq!(volumes, blood.volumes());
    assert!(
        Circulation::new(
            vec![compartment(0., 0., 1e6), compartment(0.001, 0., 1e6)],
            vec![vessel(false)]
        )
        .is_err()
    );
    assert!(Circulation::new(vec![compartment(0.001, 0., 1e6); 3], vec![vessel(false)]).is_err());
}

#[test]
fn pipe_coefficients_scale_with_geometry_and_validate_fluid_inputs() {
    let wide = Vessel::rigid_pipe(0, 1, 0.1, 0.002, 0.004, 1060., 0.5).unwrap();
    let narrow = Vessel::rigid_pipe(0, 1, 0.1, 0.001, 0.004, 1060., 0.5).unwrap();
    assert!((narrow.resistance / wide.resistance - 16.).abs() < 1e-10);
    assert!((narrow.inertance / wide.inertance - 4.).abs() < 1e-10);
    assert!((narrow.quadratic_resistance / wide.quadratic_resistance - 16.).abs() < 1e-10);
    assert!(Vessel::rigid_pipe(0, 1, 0.1, 0., 0.004, 1060., 0.5).is_err());
    assert!(Vessel::rigid_pipe(0, 1, 0.1, 0.001, 0.004, f64::NAN, 0.5).is_err());
}

#[test]
fn external_exchange_changes_pressure_and_obeys_open_volume_balance() {
    let mut blood = pair(false);
    let dt = 0.02;
    for exchange in [[-1e-5, 2e-6], [3e-6, -5e-6], [0., 0.]] {
        let old = blood.volumes().to_vec();
        let drive = 1e6 * (old[0] + exchange[0] - old[1] - exchange[1]);
        let expected_q = drive / (2e5 + dt * 2e6);
        let report = blood
            .step_with_exchange(dt, &[1e6; 2], &[0.; 2], &exchange, 32, 1e-14)
            .unwrap();
        assert!((blood.flows()[0] - expected_q).abs() < 1e-12);
        for i in 0..2 {
            let internal = if i == 0 {
                -dt * expected_q
            } else {
                dt * expected_q
            };
            assert!((blood.volumes()[i] - old[i] - exchange[i] - internal).abs() < 1e-14);
            assert!((blood.pressures()[i] - 1e6 * (blood.volumes()[i] - 0.001)).abs() < 1e-8);
        }
        assert!(report.volume_drift_m3.abs() < 1e-14);
        assert!(
            (blood.total_volume() - old.iter().sum::<f64>() - exchange.iter().sum::<f64>()).abs()
                < 1e-14
        );
    }
    let before = (
        blood.volumes().to_vec(),
        blood.pressures().to_vec(),
        blood.flows().to_vec(),
    );
    for exchange in [vec![f64::NAN, 0.], vec![-1., 0.], vec![0.]] {
        assert!(
            blood
                .step_with_exchange(dt, &[1e6; 2], &[0.; 2], &exchange, 32, 1e-14)
                .is_err()
        );
    }
    // A valid staged exchange followed by a rejected circulation solve must
    // likewise leave all accepted circuit state untouched.
    assert!(
        blood
            .step_with_exchange(-dt, &[1e6; 2], &[0.; 2], &[1e-5, 0.], 32, 1e-14)
            .is_err()
    );
    assert_eq!(
        before,
        (
            blood.volumes().to_vec(),
            blood.pressures().to_vec(),
            blood.flows().to_vec()
        )
    );
}

#[test]
fn joint_protein_step_uses_accepted_volume_and_reversing_donor() {
    for external in [[0., 0.], [0., 500.]] {
        let mut blood = pair(false);
        let mut mass = vec![2e-5, 5e-6];
        let old = mass.clone();
        let total: f64 = mass.iter().sum();
        let dt = 0.1;
        blood
            .step_with_protein(&mut mass, dt, &[1e6; 2], &external, 32, 1e-14)
            .unwrap();
        let q = blood.flows()[0];
        let (donor, receiver) = if q >= 0. { (0, 1) } else { (1, 0) };
        let expected = old[donor] / (1. + dt * q.abs() / blood.volumes()[donor]);
        assert!((mass[donor] - expected).abs() < 1e-14 * total);
        assert!((mass[receiver] - old[receiver] - old[donor] + expected).abs() < 1e-14 * total);
        assert!((mass.iter().sum::<f64>() - total).abs() < 1e-14 * total);
    }
    let mut blood = pair(false);
    let mut mass: Vec<_> = blood.volumes().iter().map(|v| 10. * v).collect();
    for _ in 0..8 {
        blood
            .step_with_protein(&mut mass, 0.1, &[1e6; 2], &[0.; 2], 32, 1e-14)
            .unwrap();
        for (m, v) in mass.iter().zip(blood.volumes()) {
            assert!((m / v - 10.).abs() < 1e-9);
        }
    }
    let before = (
        blood.volumes().to_vec(),
        blood.pressures().to_vec(),
        blood.flows().to_vec(),
    );
    let mut bad = vec![-1., 0.];
    let original = bad.clone();
    assert!(
        blood
            .step_with_protein(&mut bad, 0.1, &[1e6; 2], &[500., 0.], 32, 1e-14)
            .is_err()
    );
    assert_eq!(bad, original);
    assert_eq!(
        before,
        (
            blood.volumes().to_vec(),
            blood.pressures().to_vec(),
            blood.flows().to_vec()
        )
    );
}

#[test]
fn pulsatile_closed_vascular_loop_conserves_protein() {
    let mut blood = Circulation::new(
        vec![
            compartment(0.0012, 0.0005, 1e6),
            compartment(0.001, 0.0005, 1e6),
            compartment(0.0008, 0.0005, 1e6),
        ],
        (0..3)
            .map(|i| Vessel {
                from: i,
                to: (i + 1) % 3,
                ..vessel(false)
            })
            .collect(),
    )
    .unwrap();
    let mut protein = vec![1e-5, 0., 3e-5];
    let total = protein.iter().sum::<f64>();
    let mut reversed = false;
    for step in 0..200 {
        let external: Vec<_> = (0..3)
            .map(|i| 150. * (step as f64 * 0.1 + i as f64 * 2.).sin())
            .collect();
        blood
            .step_with_protein(&mut protein, 0.02, &[1e6; 3], &external, 32, 1e-14)
            .unwrap();
        reversed |= blood.flows().iter().any(|q| *q < 0.);
        assert!(protein.iter().all(|m| m.is_finite() && *m >= 0.));
        assert!((protein.iter().sum::<f64>() - total).abs() < 1e-11 * total);
    }
    assert!(reversed);
}

#[test]
fn joint_external_water_and_protein_exchange_preserves_concentration_and_ledgers() {
    let mut blood = pair(false);
    let mut mass: Vec<_> = blood.volumes().iter().map(|v| 10. * v).collect();
    let initial_water = blood.total_volume();
    let initial_mass = mass.iter().sum::<f64>();
    let mut exchanged_water = 0.;
    let mut exchanged_mass = 0.;
    for step in 0..20 {
        let water = if step % 2 == 0 {
            [-1e-5, 5e-6]
        } else {
            [2e-6, -3e-6]
        };
        let protein = water.map(|v| 10. * v);
        let report = blood
            .step_with_protein_exchange(
                &mut mass, 0.02, &[1e6; 2], &[0.; 2], &water, &protein, 32, 1e-14,
            )
            .unwrap();
        exchanged_water += water.iter().sum::<f64>();
        exchanged_mass += protein.iter().sum::<f64>();
        assert!(report.volume_drift_m3.abs() < 1e-14);
        assert!((blood.total_volume() - initial_water - exchanged_water).abs() < 1e-14);
        assert!(
            (mass.iter().sum::<f64>() - initial_mass - exchanged_mass).abs() < 1e-12 * initial_mass
        );
        for (m, v) in mass.iter().zip(blood.volumes()) {
            assert!((m / v - 10.).abs() < 1e-9);
        }
    }
    let before = (
        blood.volumes().to_vec(),
        blood.pressures().to_vec(),
        blood.flows().to_vec(),
    );
    let original = mass.clone();
    assert!(
        blood
            .step_with_protein_exchange(
                &mut mass,
                0.02,
                &[1e6; 2],
                &[0.; 2],
                &[0.; 2],
                &[-1., 0.],
                32,
                1e-14
            )
            .is_err()
    );
    assert_eq!(mass, original);
    // Individual inventories are finite; their sum overflows only in the
    // protein phase, after a valid volume/flow solve has been staged.
    let mut huge = vec![1e308; 2];
    assert!(
        blood
            .step_with_protein_exchange(
                &mut huge,
                0.02,
                &[1e6; 2],
                &[100., 0.],
                &[1e-5, 0.],
                &[0.; 2],
                32,
                1e-14
            )
            .is_err()
    );
    assert_eq!(huge, vec![1e308; 2]);
    assert_eq!(
        before,
        (
            blood.volumes().to_vec(),
            blood.pressures().to_vec(),
            blood.flows().to_vec()
        )
    );
}
