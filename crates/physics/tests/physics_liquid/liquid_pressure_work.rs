use physics::liquid::{Config, Error, Liquid, LiquidField, Material, Particle, TransportMaterial};
fn fluid(expanding: bool, temperature: f64) -> Liquid {
    let sign = if expanding { -1.0 } else { 1.0 };
    let mut fluid = Liquid::new(
        vec![
            Particle {
                position: [-0.1, 0.0, 0.0],
                velocity: [sign, 0.0, 0.0],
                mass: 1.0,
                material: 0,
            },
            Particle {
                position: [0.1, 0.0, 0.0],
                velocity: [-0.5 * sign, 0.0, 0.0],
                mass: 2.0,
                material: 0,
            },
        ],
        vec![Material {
            rest_density: 1.0,
            sound_speed: 1.0,
            viscosity: 0.0,
        }],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 1.0,
            ..Config::default()
        },
    )
    .unwrap();
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature,
                    concentration: 0.0
                };
                2
            ],
            vec![TransportMaterial {
                specific_heat: 1.0,
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    fluid.set_pressure_work(true).unwrap();
    fluid
}
fn energy(fluid: &Liquid) -> f64 {
    fluid.transport_totals().unwrap().unwrap().0
        + fluid
            .particles()
            .iter()
            .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>()
}
#[test]
fn compression_heats_and_expansion_cools_without_losing_total_energy() {
    for expanding in [false, true] {
        let mut fluid = fluid(expanding, 100.0);
        let before = energy(&fluid);
        let heat = fluid.transport_totals().unwrap().unwrap().0;
        fluid.step(0.001, None).unwrap();
        let difference = fluid.transport_totals().unwrap().unwrap().0 - heat;
        assert!(if expanding {
            difference < 0.0
        } else {
            difference > 0.0
        });
        assert!((energy(&fluid) - before).abs() < 1e-10);
        let momentum: f64 = fluid
            .particles()
            .iter()
            .map(|p| p.mass * p.velocity[0])
            .sum();
        assert!(momentum.abs() < 1e-12);
    }
}
#[test]
fn repeated_pressure_work_conserves_energy_through_the_motion() {
    let mut fluid = fluid(false, 100.0);
    let before = energy(&fluid);
    for _ in 0..100 {
        fluid.step(0.001, None).unwrap();
    }
    assert!((energy(&fluid) - before).abs() < 1e-9);
}
#[test]
fn insufficient_thermal_energy_rolls_back_pressure_kicks() {
    let mut fluid = fluid(true, 0.0);
    let before = fluid.clone();
    assert_eq!(fluid.step(0.001, None), Err(Error::NumericalFailure));
    assert_eq!(fluid, before);
}
#[test]
fn disabled_pressure_work_retains_the_previous_isothermal_mode() {
    let mut fluid = fluid(false, 100.0);
    fluid.set_pressure_work(false).unwrap();
    fluid.step(0.001, None).unwrap();
    assert!(
        fluid
            .fields()
            .unwrap()
            .iter()
            .all(|field| (field.temperature - 100.0).abs() < 1e-12)
    );
    let mut bare = Liquid::new(vec![], vec![Material::WATER], Config::default()).unwrap();
    assert_eq!(bare.set_pressure_work(true), Err(Error::InvalidTransport));
}
#[test]
fn pressure_work_and_viscous_heat_conserve_energy_together_in_a_cloud() {
    let mut fluid = Liquid::new(
        vec![
            Particle {
                position: [-0.2, 0.0, 0.0],
                velocity: [1.0, 0.0, 0.0],
                mass: 1.0,
                material: 0,
            },
            Particle {
                position: [0.0, 0.0, 0.0],
                velocity: [0.0; 3],
                mass: 2.0,
                material: 0,
            },
            Particle {
                position: [0.2, 0.0, 0.0],
                velocity: [-1.0, 0.0, 0.0],
                mass: 3.0,
                material: 0,
            },
        ],
        vec![Material {
            rest_density: 1.0,
            sound_speed: 1.0,
            viscosity: 1.0,
        }],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 1.0,
            ..Config::default()
        },
    )
    .unwrap();
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 100.0,
                    concentration: 0.0
                };
                3
            ],
            vec![TransportMaterial {
                specific_heat: 1.0,
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    fluid.set_pressure_work(true).unwrap();
    fluid.set_viscous_heating(true).unwrap();
    let before = energy(&fluid);
    for _ in 0..20 {
        fluid.step(0.001, None).unwrap();
    }
    assert!((energy(&fluid) - before).abs() < 1e-8);
    let momentum: f64 = fluid
        .particles()
        .iter()
        .map(|p| p.mass * p.velocity[0])
        .sum();
    assert!((momentum + 2.0).abs() < 1e-10);
}
#[test]
fn pressure_work_is_stored_in_latent_heat_on_the_phase_plateau() {
    for expanding in [false, true] {
        let mut liquid = fluid(expanding, 100.0);
        liquid
            .configure_phase_change(
                vec![Some(physics::liquid::PhaseChange {
                    temperature: 100.0,
                    latent_heat: 10.0,
                    high_phase: Material {
                        rest_density: 1.0,
                        sound_speed: 1.0,
                        viscosity: 0.0,
                    },
                })],
                vec![0.5; 2],
            )
            .unwrap();
        let before = energy(&liquid);
        liquid.step(0.001, None).unwrap();
        assert!((energy(&liquid) - before).abs() < 1e-10);
        assert!(
            liquid
                .fields()
                .unwrap()
                .iter()
                .all(|field| (field.temperature - 100.0).abs() < 1e-12)
        );
        assert!(
            liquid
                .phase_fractions()
                .unwrap()
                .iter()
                .all(|fraction| if expanding {
                    *fraction < 0.5
                } else {
                    *fraction > 0.5
                })
        );
    }
}
#[test]
fn thermal_conduction_and_temperature_response_keep_the_pressure_energy_balance() {
    let mut liquid = fluid(false, 100.0);
    liquid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 100.0,
                    concentration: 0.0,
                },
                LiquidField {
                    temperature: 120.0,
                    concentration: 0.0,
                },
            ],
            vec![TransportMaterial {
                specific_heat: 1.0,
                conductivity: 10.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid
        .configure_property_response(vec![Some(physics::liquid::PropertyResponse {
            reference_temperature: 100.0,
            thermal_expansion: 0.001,
            viscosity_temperature_rate: 0.0,
            solute: None,
        })])
        .unwrap();
    let before = energy(&liquid);
    for _ in 0..100 {
        liquid.step(0.001, None).unwrap();
    }
    assert!((energy(&liquid) - before).abs() < 1e-8);
    assert!(
        liquid.fields().unwrap()[1].temperature - liquid.fields().unwrap()[0].temperature < 20.0
    );
}

#[test]
fn volume_pressure_work_uses_the_same_force_and_conserves_total_energy() {
    use physics::liquid::{
        Config, Formulation, LiquidField, Material, Particle, TransportMaterial,
    };
    for formulation in [Formulation::RestVolume, Formulation::RestVolumeWendland] {
        let mut fluid = Liquid::new(
            vec![
                Particle {
                    position: [0.0; 3],
                    velocity: [0.2, 0.1, 0.0],
                    mass: 1.0,
                    material: 0,
                },
                Particle {
                    position: [0.2, 0.1, 0.0],
                    velocity: [-0.1, 0.0, 0.0],
                    mass: 1.1,
                    material: 1,
                },
            ],
            vec![
                Material {
                    rest_density: 1.0,
                    sound_speed: 1.0,
                    viscosity: 0.0,
                },
                Material {
                    rest_density: 0.8,
                    sound_speed: 1.0,
                    viscosity: 0.0,
                },
            ],
            Config {
                smoothing_radius: 1.0,
                gravity: [0.0; 3],
                ..Config::default()
            },
        )
        .unwrap();
        fluid.set_formulation(formulation);
        fluid
            .configure_transport(
                vec![
                    LiquidField {
                        temperature: 10.0,
                        concentration: 0.0
                    };
                    2
                ],
                vec![
                    TransportMaterial {
                        specific_heat: 1.0,
                        conductivity: 0.0,
                        ..TransportMaterial::default()
                    };
                    2
                ],
            )
            .unwrap();
        let total = |fluid: &Liquid| {
            fluid.transport_totals().unwrap().unwrap().0
                + fluid
                    .particles()
                    .iter()
                    .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
                    .sum::<f64>()
        };
        let initial = total(&fluid);
        let mut explicit = fluid.clone();
        fluid.set_pressure_work(true).unwrap();
        explicit.step(0.001, None).unwrap();
        fluid.step(0.001, None).unwrap();
        for (a, b) in fluid.particles().iter().zip(explicit.particles()) {
            for axis in 0..3 {
                assert!((a.velocity[axis] - b.velocity[axis]).abs() < 1e-12);
            }
        }
        for _ in 0..20 {
            fluid.step(0.001, None).unwrap();
        }
        assert!((total(&fluid) - initial).abs() < 1e-11);
    }
}
