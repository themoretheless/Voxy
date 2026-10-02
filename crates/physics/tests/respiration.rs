use physics::respiration::*;
fn linear(compliance: f64) -> LungUnit {
    LungUnit {
        reference_volume_m3: 0.001,
        reference_transpulmonary_pa: 500.,
        recoil: Recoil::Linear {
            compliance_m3_per_pa: compliance,
        },
    }
}
fn single() -> RespiratoryNetwork {
    RespiratoryNetwork::new(
        vec![None, Some(linear(1e-6))],
        vec![Airway {
            from: 0,
            to: 1,
            resistance_pa_s_per_m3: 2e5,
        }],
        vec![0.; 2],
        vec![0., -500.],
    )
    .unwrap()
}
#[test]
fn single_compartment_matches_implicit_rc_solution_and_volume_balance() {
    let mut network = single();
    let dt = 0.02;
    let tau = 0.2;
    let mut pressure = 0.;
    for _ in 0..30 {
        pressure = (pressure + dt / tau * 100.) / (1. + dt / tau);
        let step = network.step(dt, 100., &[0., -500.], 32, 1e-13).unwrap();
        assert!((network.pressures()[1] - pressure).abs() < 1e-8);
        assert!((network.volumes()[1] - (0.001 + 1e-6 * pressure)).abs() < 1e-12);
        assert!((step.volume_change_m3 - step.mouth_volume_m3).abs() < 1e-12);
        assert!(step.airway_loss_j >= 0.);
    }
}
#[test]
fn negative_pleural_pressure_inflates_and_expiration_reverses_flow() {
    let mut network = single();
    let step = network.step(0.1, 0., &[0., -600.], 32, 1e-13).unwrap();
    assert!(step.mouth_volume_m3 > 0.);
    assert!(network.pressures()[1] < 0.);
    assert!(network.volumes()[1] > 0.001);
    let step = network.step(0.1, 0., &[0., -500.], 32, 1e-13).unwrap();
    assert!(step.mouth_volume_m3 < 0.);
}
fn branched(logarithmic: bool) -> RespiratoryNetwork {
    let unit = if logarithmic {
        LungUnit {
            reference_volume_m3: 0.001,
            reference_transpulmonary_pa: 500.,
            recoil: Recoil::Logarithmic { scale_pa: 1200. },
        }
    } else {
        linear(1e-6)
    };
    RespiratoryNetwork::new(
        vec![None, None, Some(unit), Some(unit)],
        vec![
            Airway {
                from: 0,
                to: 1,
                resistance_pa_s_per_m3: 1e5,
            },
            Airway {
                from: 1,
                to: 2,
                resistance_pa_s_per_m3: 1e5,
            },
            Airway {
                from: 1,
                to: 3,
                resistance_pa_s_per_m3: 8e5,
            },
        ],
        vec![0.; 4],
        vec![0., 0., -500., -500.],
    )
    .unwrap()
}
#[test]
fn junction_conservation_and_regional_airway_resistance_affect_filling() {
    let mut network = branched(false);
    for _ in 0..50 {
        let step = network
            .step(0.02, 150., &[0., 0., -500., -500.], 32, 1e-13)
            .unwrap();
        let flow = network.flows();
        assert!((flow[0] - flow[1] - flow[2]).abs() < 1e-10);
        assert!((step.volume_change_m3 - step.mouth_volume_m3).abs() < 3e-12);
        assert!(network.volumes()[2] > network.volumes()[3]);
    }
}
#[test]
fn nonlinear_compartments_conserve_volume_under_cyclic_pleural_forcing() {
    let mut network = branched(true);
    for i in 0..300 {
        let pleural = -500. - 180. * (f64::from(i) * 0.02).sin();
        let step = network
            .step(0.02, 0., &[0., 0., pleural, pleural + 20.], 32, 1e-13)
            .unwrap();
        assert!((step.volume_change_m3 - step.mouth_volume_m3).abs() < 3e-12);
        assert!(step.residual_m3 < 1e-13);
        assert!(network.volumes()[2] > 0. && network.volumes()[3] > 0.);
        let pressures = network.transpulmonary_pressures();
        for k in [2, 3] {
            let expected = 500. + 1200. * (network.volumes()[k] / 0.001).ln();
            assert!((pressures[k] - expected).abs() < 1e-8);
        }
    }
}
#[test]
fn pressure_gauge_shift_does_not_change_mechanics() {
    let mut first = single();
    let mut second = RespiratoryNetwork::new(
        vec![None, Some(linear(1e-6))],
        vec![Airway {
            from: 0,
            to: 1,
            resistance_pa_s_per_m3: 2e5,
        }],
        vec![2000.; 2],
        vec![2000., 1500.],
    )
    .unwrap();
    for _ in 0..10 {
        first.step(0.1, 100., &[0., -600.], 32, 1e-13).unwrap();
        second.step(0.1, 2100., &[2000., 1400.], 32, 1e-13).unwrap();
        assert!((first.volumes()[1] - second.volumes()[1]).abs() < 1e-12);
        assert!((first.flows()[0] - second.flows()[0]).abs() < 1e-10);
    }
}
#[test]
fn invalid_network_and_nonconverged_step_are_rejected_atomically() {
    assert!(Airway::poiseuille(0, 1, 0.1, 0., 1.8e-5).is_err());
    let a = Airway::poiseuille(0, 1, 0.1, 0.002, 1.8e-5).unwrap();
    assert!(a.resistance_pa_s_per_m3 > 0.);
    let narrow = Airway::poiseuille(0, 1, 0.1, 0.001, 1.8e-5).unwrap();
    assert!((narrow.resistance_pa_s_per_m3 / a.resistance_pa_s_per_m3 - 16.).abs() < 1e-10);
    assert!(
        RespiratoryNetwork::new(
            vec![None, Some(linear(1e-6)), None],
            vec![a],
            vec![0.; 3],
            vec![0., -500., 0.]
        )
        .is_err()
    );
    let mut network = branched(true);
    let volumes = network.volumes().to_vec();
    let pressure = network.pressures().to_vec();
    assert!(
        network
            .step(0.1, 1000., &[0., 0., -1000., -1000.], 1, 1e-18)
            .is_err()
    );
    assert_eq!(volumes, network.volumes());
    assert_eq!(pressure, network.pressures());
    assert!(network.step(f64::NAN, 0., &[0.; 4], 32, 1e-13).is_err());
    assert_eq!(volumes, network.volumes());
}
