use physics::biomechanics::{CouplingConfig, FemChamber, Fiber, Material, tube};
use physics::circulation::{Circulation, Compartment, PressureVolume, Vessel};
fn specimen() -> FemChamber {
    let material = Material {
        shear_pa: 2000.,
        bulk_pa: 20000.,
        fibers: vec![Fiber {
            direction: [1., 0., 0.],
            stiffness_pa: 1000.,
            exponent: 3.,
            active_pa: 10000.,
        }],
    };
    FemChamber::new(
        tube(&[0.01, 0.014], 0.025, 8, 1, &[material], true).unwrap(),
        0,
        CouplingConfig {
            force_tolerance_n: 1e-8,
            volume_tolerance_m3: 1e-11,
            ..CouplingConfig::default()
        },
    )
    .unwrap()
}
fn circuit(chamber: &FemChamber) -> Circulation {
    let volume = chamber.body().cavity_volume(0).unwrap();
    Circulation::new(
        vec![
            Compartment {
                unstressed_volume_m3: volume,
                initial_volume_m3: volume,
                initial_elastance_pa_per_m3: 1e8,
                initial_external_pressure_pa: 0.,
            },
            Compartment {
                unstressed_volume_m3: 0.,
                initial_volume_m3: 0.001,
                initial_elastance_pa_per_m3: 1e5,
                initial_external_pressure_pa: 0.,
            },
        ],
        vec![Vessel {
            from: 1,
            to: 0,
            resistance: 2e6,
            quadratic_resistance: 0.,
            inertance: 0.,
            valve: false,
        }],
    )
    .unwrap()
}
#[test]
fn cavity_pressure_volume_closes_the_flow_loop_and_deforms_actual_fem_mesh() {
    let mut chamber = specimen();
    let mut blood = circuit(&chamber);
    let before = chamber.body().positions().to_vec();
    let original = chamber.body().cavity_volume(0).unwrap();
    let total = blood.total_volume();
    let report = chamber
        .step(&mut blood, 0.01, &[1e8, 1e5], &[0.; 2], 32)
        .unwrap();
    assert!(report.residual_m3 < 1e-11);
    assert!(blood.volumes()[0] > original);
    assert!((chamber.body().cavity_volume(0).unwrap() - blood.volumes()[0]).abs() < 1e-11);
    assert!((blood.total_volume() - total).abs() < 2e-11);
    assert_ne!(before, chamber.body().positions());
    assert!((chamber.body().cavities()[0].pressure_pa - blood.pressures()[0]).abs() < 1e-10);
    assert!(
        blood.elastic_energy().is_err(),
        "nominal chamber energy must not impersonate FEM energy"
    );
}
#[test]
fn activated_wall_transfers_volume_back_into_the_circuit() {
    let mut chamber = specimen();
    let mut blood = circuit(&chamber);
    let volume = blood.volumes()[0];
    for i in 0..chamber.body().elements().len() {
        chamber.body_mut().set_activation(i, 0.15).unwrap();
    }
    chamber
        .step(&mut blood, 0.01, &[1e8, 1e5], &[0.; 2], 32)
        .unwrap();
    assert!(
        blood.volumes()[0] < volume,
        "active wall did not eject volume"
    );
    assert!(blood.flows()[0] < 0.);
    assert!((chamber.body().cavity_volume(0).unwrap() - blood.volumes()[0]).abs() < 1e-11);
}
#[test]
fn failed_outer_step_preserves_geometry_history_and_blood_state() {
    let mut chamber = specimen();
    let mut blood = circuit(&chamber);
    let positions = chamber.body().positions().to_vec();
    let volumes = blood.volumes().to_vec();
    let pressure = blood.pressures().to_vec();
    assert!(
        chamber
            .step(&mut blood, 0.01, &[1e8, 1e5], &[0.; 2], 0)
            .is_err()
    );
    assert_eq!(positions, chamber.body().positions());
    assert_eq!(volumes, blood.volumes());
    assert_eq!(pressure, blood.pressures());
    assert_eq!(chamber.body().cavities()[0].pressure_pa, 0.);
}
#[test]
fn volume_callback_uses_actual_law_and_rejects_invalid_compliance_atomically() {
    let chamber = specimen();
    let mut blood = circuit(&chamber);
    let volume = blood.volumes()[0];
    blood
        .step_with_volume_response(0.01, &[1e8, 1e5], &[0.; 2], 32, 1e-13, |i, p| {
            Ok(if i == 0 {
                Some(PressureVolume {
                    volume_m3: volume + 1e-8 * p,
                    compliance_m3_per_pa: 1e-8,
                })
            } else {
                None
            })
        })
        .unwrap();
    assert!((blood.volumes()[0] - (volume + 1e-8 * blood.pressures()[0])).abs() < 1e-12);
    let old = blood.volumes().to_vec();
    assert!(
        blood
            .step_with_volume_response(0.01, &[1e8, 1e5], &[0.; 2], 32, 1e-13, |_, _| Ok(Some(
                PressureVolume {
                    volume_m3: 0.001,
                    compliance_m3_per_pa: -1.
                }
            )))
            .is_err()
    );
    assert_eq!(old, blood.volumes());
}
#[test]
fn subsequent_custom_steps_start_from_accepted_pressure_after_contraction() {
    let chamber = specimen();
    let mut blood = circuit(&chamber);
    let reference = blood.volumes()[0];
    for contraction in [0., 2e-6, 2e-6] {
        blood
            .step_with_volume_response(0.01, &[1e8, 1e5], &[0.; 2], 32, 1e-13, |i, p| {
                if i != 0 {
                    return Ok(None);
                }
                if p < 0. {
                    return Err("negative constitutive wall pressure");
                }
                Ok(Some(PressureVolume {
                    volume_m3: reference - contraction + 1e-8 * p,
                    compliance_m3_per_pa: 1e-8,
                }))
            })
            .unwrap();
    }
    assert!(blood.volumes()[0] < reference);
    assert!(blood.pressures()[0] > 0.);
}
#[test]
fn pressure_tangent_trials_do_not_advance_viscous_history_multiple_times() {
    use physics::biomechanics::{IDENTITY, MaxwellBranch, OgdenTerm, ViscoelasticOgden};
    let mut chamber = specimen();
    for i in 0..chamber.body().elements().len() {
        chamber
            .body_mut()
            .set_viscoelastic_ogden(
                i,
                ViscoelasticOgden::new(
                    vec![OgdenTerm {
                        shear_pa: 2000.,
                        exponent: 2.,
                    }],
                    20000.,
                    vec![MaxwellBranch {
                        shear_pa: 1000.,
                        relaxation_seconds: 0.2,
                    }],
                )
                .unwrap(),
            )
            .unwrap();
    }
    let mut blood = circuit(&chamber);
    let old_body = chamber.body().clone();
    chamber
        .step(&mut blood, 0.01, &[1e8, 1e5], &[0.; 2], 32)
        .unwrap();
    let mut reference = old_body;
    reference.set_pressure(0, blood.pressures()[0]).unwrap();
    reference.relax_step(0.01, 4000, 1e-8).unwrap();
    let a = chamber.body().elements()[0]
        .response(IDENTITY)
        .unwrap()
        .first_piola;
    let b = reference.elements()[0]
        .response(IDENTITY)
        .unwrap()
        .first_piola;
    for i in 0..3 {
        for k in 0..3 {
            assert!(
                (a[i][k] - b[i][k]).abs() < 1e-4,
                "branch history differs at {i},{k}: {} vs {}",
                a[i][k],
                b[i][k]
            );
        }
    }
}
