use physics::liquid::{
    Config, Error, Liquid, LiquidField, Material, Particle, PropertyResponse, TransportMaterial,
};
fn system(material: Material, temperature: f64) -> Liquid {
    let mut liquid = Liquid::new(
        vec![
            Particle {
                position: [-0.1, 0.0, 0.0],
                velocity: [1.0, 0.0, 0.0],
                mass: 1.0,
                material: 0,
            },
            Particle {
                position: [0.1, 0.0, 0.0],
                velocity: [-1.0, 0.0, 0.0],
                mass: 1.0,
                material: 0,
            },
        ],
        vec![material],
        Config {
            smoothing_radius: 1.0,
            particle_radius: 0.05,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    liquid
        .configure_transport(
            vec![
                LiquidField {
                    temperature,
                    concentration: 0.0
                };
                2
            ],
            vec![TransportMaterial {
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid
}
fn response() -> PropertyResponse {
    PropertyResponse {
        reference_temperature: 300.0,
        thermal_expansion: 0.001,
        viscosity_temperature_rate: 0.01,
        solute: None,
    }
}
#[test]
fn hot_liquid_expands_and_is_less_viscous_in_actual_force_integration() {
    let material = Material {
        viscosity: 10.0,
        ..Material::WATER
    };
    let mut cold = system(material, 300.0);
    let mut hot = system(material, 400.0);
    cold.configure_property_response(vec![Some(response())])
        .unwrap();
    hot.configure_property_response(vec![Some(response())])
        .unwrap();
    let effective = hot.effective_materials().unwrap()[0];
    assert!((effective.rest_density - 1000.0 / 1.1).abs() < 1e-10);
    assert!((effective.viscosity - 10.0 * (-1.0_f64).exp()).abs() < 1e-12);
    cold.step(0.01, None).unwrap();
    hot.step(0.01, None).unwrap();
    let relative = |l: &Liquid| (l.particles()[0].velocity[0] - l.particles()[1].velocity[0]).abs();
    assert!(relative(&hot) > relative(&cold));
    for liquid in [&hot, &cold] {
        assert!(
            (liquid.particles()[0].velocity[0] + liquid.particles()[1].velocity[0]).abs() < 1e-12
        );
    }
}
#[test]
fn mass_fraction_blend_uses_specific_volume_and_affects_viscosity() {
    let mut liquid = system(Material::WATER, 300.0);
    liquid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.5
                };
                2
            ],
            vec![TransportMaterial::default()],
        )
        .unwrap();
    liquid
        .configure_property_response(vec![Some(PropertyResponse {
            solute: Some(Material::OIL),
            ..response()
        })])
        .unwrap();
    let effective = liquid.effective_materials().unwrap()[0];
    assert!((effective.rest_density - 1.0 / (0.5 / 1000.0 + 0.5 / 800.0)).abs() < 1e-10);
    assert!((effective.viscosity - 0.0505).abs() < 1e-12);
}
#[test]
fn temperature_dependent_density_changes_pressure_force() {
    let material = Material {
        rest_density: 2.0,
        sound_speed: 5.0,
        viscosity: 0.0,
    };
    let mut cold = system(material, 300.0);
    let mut hot = system(material, 400.0);
    let response = PropertyResponse {
        thermal_expansion: 0.01,
        ..response()
    };
    cold.configure_property_response(vec![Some(response)])
        .unwrap();
    hot.configure_property_response(vec![Some(response)])
        .unwrap();
    cold.step(0.001, None).unwrap();
    hot.step(0.001, None).unwrap();
    assert!(hot.particles()[0].velocity[0] < cold.particles()[0].velocity[0]);
}
#[test]
fn invalid_response_and_incompatible_field_replacement_are_atomic() {
    let mut liquid = system(Material::WATER, 300.0);
    let before = liquid.clone();
    assert_eq!(
        liquid.configure_property_response(vec![Some(PropertyResponse {
            thermal_expansion: f64::NAN,
            ..response()
        })]),
        Err(Error::InvalidPropertyResponse)
    );
    assert_eq!(liquid, before);
    liquid
        .configure_property_response(vec![Some(PropertyResponse {
            thermal_expansion: 0.1,
            ..response()
        })])
        .unwrap();
    let before = liquid.clone();
    assert_eq!(
        liquid.configure_transport(
            vec![
                LiquidField {
                    temperature: 200.0,
                    concentration: 0.0
                };
                2
            ],
            vec![TransportMaterial::default()]
        ),
        Err(Error::InvalidPropertyResponse)
    );
    assert_eq!(liquid, before);
}

#[test]
fn heat_exchange_outside_constitutive_domain_rolls_back_the_entire_step() {
    let mut second = Particle {
        position: [0.1, 0.0, 0.0],
        velocity: [0.0; 3],
        mass: 1.0,
        material: 1,
    };
    let first = Particle {
        position: [-0.1, 0.0, 0.0],
        material: 0,
        ..second
    };
    second.material = 1;
    let mut liquid = Liquid::new(
        vec![first, second],
        vec![Material::WATER, Material::WATER],
        Config {
            smoothing_radius: 1.0,
            particle_radius: 0.05,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    liquid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.0,
                },
                LiquidField {
                    temperature: 600.0,
                    concentration: 0.0,
                },
            ],
            vec![
                TransportMaterial {
                    specific_heat: 1.0,
                    conductivity: 1e12,
                    ..TransportMaterial::default()
                };
                2
            ],
        )
        .unwrap();
    liquid
        .configure_property_response(vec![
            Some(PropertyResponse {
                thermal_expansion: -0.01,
                ..response()
            }),
            None,
        ])
        .unwrap();
    let before = liquid.clone();
    assert_eq!(
        liquid.step(0.001, None),
        Err(Error::InvalidPropertyResponse)
    );
    assert_eq!(liquid, before);
}

#[test]
fn buoyancy_uses_temperature_adjusted_carrier_density() {
    use physics::liquid::{BuoyancyConfig, FloatingBody, FluidLayer};
    let mut liquid = system(Material::WATER, 400.0);
    liquid
        .configure_property_response(vec![Some(response())])
        .unwrap();
    let mut sphere = FloatingBody {
        position: [0.0; 3],
        velocity: [0.0; 3],
        mass: 2.0,
        radius: 0.1,
    };
    let report = liquid
        .couple_floating_body(
            &mut sphere,
            &[FluidLayer {
                bottom: -1.0,
                top: 1.0,
                material: 0,
            }],
            0.001,
            BuoyancyConfig::default(),
        )
        .unwrap();
    let expected = 4.0 * std::f64::consts::PI / 3.0 * 0.1_f64.powi(3) * 1000.0 / 1.1;
    assert!((report.displaced_mass - expected).abs() < 1e-10);
}
#[test]
fn response_configuration_validates_the_current_phase_not_only_the_base_material() {
    let mut liquid = system(
        Material {
            rest_density: 1000.0,
            sound_speed: 2.0,
            viscosity: 0.0,
        },
        100.0,
    );
    liquid
        .configure_phase_change(
            vec![Some(physics::liquid::PhaseChange {
                temperature: 100.0,
                latent_heat: 10.0,
                high_phase: Material {
                    rest_density: 800.0,
                    sound_speed: 2.0,
                    viscosity: 1e308,
                },
            })],
            vec![1.0; 2],
        )
        .unwrap();
    let before = liquid.clone();
    assert_eq!(
        liquid.configure_property_response(vec![Some(PropertyResponse {
            reference_temperature: 0.0,
            thermal_expansion: 0.0,
            viscosity_temperature_rate: -0.01,
            solute: None
        })]),
        Err(Error::NumericalFailure)
    );
    assert_eq!(liquid, before);
}

#[test]
fn symmetric_heated_advection_updates_temperature_dependent_viscosity_and_expansion() {
    use physics::liquid::ViscousIntegrator;
    let mut liquid = system(
        Material {
            rest_density: 1000.0,
            sound_speed: 1.0,
            viscosity: 10.0,
        },
        300.0,
    );
    liquid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.0
                };
                2
            ],
            vec![TransportMaterial {
                specific_heat: 0.1,
                conductivity: 0.0,
                diffusivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid.set_viscous_heating(true).unwrap();
    liquid
        .set_viscous_integrator(ViscousIntegrator::Symmetric)
        .unwrap();
    let mut frozen = liquid.clone();
    liquid
        .configure_property_response(vec![Some(PropertyResponse {
            reference_temperature: 300.0,
            thermal_expansion: 0.001,
            viscosity_temperature_rate: 0.1,
            solute: None,
        })])
        .unwrap();
    let kinetic = |f: &Liquid| {
        f.particles()
            .iter()
            .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>()
    };
    let initial = kinetic(&liquid) + liquid.transport_totals().unwrap().unwrap().0;
    for _ in 0..200 {
        liquid.step(0.0001, None).unwrap();
        frozen.step(0.0001, None).unwrap();
        assert!(
            (kinetic(&liquid) + liquid.transport_totals().unwrap().unwrap().0 - initial).abs()
                < 1e-10
        );
    }
    for (field, material) in liquid
        .fields()
        .unwrap()
        .iter()
        .zip(liquid.effective_materials().unwrap())
    {
        assert!(field.temperature > 300.0);
        let delta = field.temperature - 300.0;
        assert!((material.viscosity - 10.0 * (-0.1 * delta).exp()).abs() < 1e-12);
        assert!((material.rest_density - 1000.0 / (1.0 + 0.001 * delta)).abs() < 1e-10);
    }
    assert!(kinetic(&liquid) > kinetic(&frozen));
    assert!(liquid.effective_materials().unwrap()[0].viscosity < 9.0);
}

#[test]
fn condensed_milk_demo_thins_under_shear_but_not_rigid_rotation() {
    use physics::liquid::ShearThinning;
    let make = |rate: f64, rotation: bool| {
        let mut particles = Vec::new();
        for x in 0..3 {
            for y in 0..3 {
                for z in 0..3 {
                    let position = [x, y, z].map(|i| f64::from(i) * 0.1);
                    particles.push(Particle {
                        position,
                        velocity: if rotation {
                            [-rate * position[1], rate * position[0], 0.0]
                        } else {
                            [rate * position[1], 0.0, 0.0]
                        },
                        mass: 1.3,
                        material: 0,
                    });
                }
            }
        }
        let mut f = Liquid::new(
            particles,
            vec![Material::CONDENSED_MILK_DEMO],
            Config {
                smoothing_radius: 0.3,
                gravity: [0.0; 3],
                ..Config::default()
            },
        )
        .unwrap();
        f.configure_shear_thinning(vec![Some(ShearThinning::CONDENSED_MILK_DEMO)])
            .unwrap();
        f
    };
    let slow = make(0.1, false).effective_materials().unwrap();
    let fast = make(10.0, false).effective_materials().unwrap();
    let resting = make(0.0, false).effective_materials().unwrap();
    let spinning = make(10.0, true).effective_materials().unwrap();
    for ((slow, fast), (rest, spin)) in slow.iter().zip(fast).zip(resting.iter().zip(spinning)) {
        assert!(slow.viscosity > fast.viscosity);
        assert!((rest.viscosity - spin.viscosity).abs() < 1e-10);
        assert!((rest.viscosity - 10.0 * 0.01_f64.powf(-0.25)).abs() < 1e-10);
    }
    let mut f = make(1.0, false);
    let before = f.clone();
    assert!(
        f.configure_shear_thinning(vec![Some(ShearThinning {
            flow_index: 0.0,
            ..ShearThinning::CONDENSED_MILK_DEMO
        })])
        .is_err()
    );
    assert_eq!(f, before);
}

#[test]
fn shear_thinning_advection_and_heating_preserve_discrete_energy() {
    use physics::liquid::{ShearThinning, ViscousIntegrator};
    let mut liquid = system(Material::CONDENSED_MILK_DEMO, 300.0);
    liquid
        .configure_shear_thinning(vec![Some(ShearThinning::CONDENSED_MILK_DEMO)])
        .unwrap();
    liquid.set_viscous_heating(true).unwrap();
    liquid
        .set_viscous_integrator(ViscousIntegrator::Symmetric)
        .unwrap();
    let total = |f: &Liquid| {
        f.transport_totals().unwrap().unwrap().0
            + f.particles()
                .iter()
                .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
                .sum::<f64>()
    };
    let initial = total(&liquid);
    for _ in 0..20 {
        liquid.step(0.0001, None).unwrap();
        assert!((total(&liquid) - initial).abs() < 1e-8);
    }
    assert!(liquid.fields().unwrap()[0].temperature > 300.0);
    assert!(liquid.particles()[0].velocity[0] < 1.0);
}

#[test]
fn arrhenius_condensed_milk_temperature_response_composes_with_shear_thinning() {
    use physics::liquid::{ArrheniusViscosity, ShearThinning};
    let make = |temperature: f64| {
        let mut f = system(Material::CONDENSED_MILK_DEMO, temperature);
        f.configure_arrhenius_viscosity(vec![Some(ArrheniusViscosity::CONDENSED_MILK_DEMO)])
            .unwrap();
        f.configure_shear_thinning(vec![Some(ShearThinning::CONDENSED_MILK_DEMO)])
            .unwrap();
        f
    };
    let cold = make(278.15);
    let reference = make(298.15);
    let hot = make(328.15);
    let ratio = |temperature: f64| {
        (37_000.0 / 8.314_462_618_153_24 * (1.0 / temperature - 1.0 / 298.15)).exp()
    };
    for ((a, b), c) in cold
        .effective_materials()
        .unwrap()
        .iter()
        .zip(reference.effective_materials().unwrap())
        .zip(hot.effective_materials().unwrap())
    {
        assert!(a.viscosity > b.viscosity && b.viscosity > c.viscosity);
        assert!((a.viscosity / b.viscosity - ratio(278.15)).abs() < 1e-12);
        assert!((c.viscosity / b.viscosity - ratio(328.15)).abs() < 1e-12);
        assert!((a.rest_density - c.rest_density).abs() < 1e-12);
    }
}

#[test]
fn arrhenius_response_rejects_duplicate_temperature_laws_and_invalid_cooling_atomically() {
    use physics::liquid::ArrheniusViscosity;
    let mut f = system(Material::CONDENSED_MILK_DEMO, 300.0);
    f.configure_property_response(vec![Some(response())])
        .unwrap();
    let before = f.clone();
    assert_eq!(
        f.configure_arrhenius_viscosity(vec![Some(ArrheniusViscosity::CONDENSED_MILK_DEMO)]),
        Err(Error::InvalidPropertyResponse)
    );
    assert_eq!(f, before);
    f.configure_property_response(vec![None]).unwrap();
    f.configure_arrhenius_viscosity(vec![Some(ArrheniusViscosity::CONDENSED_MILK_DEMO)])
        .unwrap();
    let before = f.clone();
    assert_eq!(
        f.configure_property_response(vec![Some(response())]),
        Err(Error::InvalidPropertyResponse)
    );
    assert_eq!(f, before);
    assert_eq!(
        f.exchange_reservoir_heat(1.0, 0.0, &[1e100; 2]),
        Err(Error::InvalidPropertyResponse)
    );
    assert_eq!(f, before);
    assert_eq!(
        f.configure_transport(
            vec![
                LiquidField {
                    temperature: 0.0,
                    concentration: 0.0
                };
                2
            ],
            vec![TransportMaterial::default()]
        ),
        Err(Error::InvalidPropertyResponse)
    );
    assert_eq!(f, before);
}

#[test]
fn shear_rate_estimate_reproduces_unit_affine_shear_away_from_cloud_edges() {
    use physics::liquid::ShearThinning;
    let mut particles = Vec::new();
    for x in 0..14 {
        for y in 0..14 {
            for z in 0..14 {
                let position = [x, y, z].map(|a| (f64::from(a) + 0.5) * 0.1);
                particles.push(Particle {
                    position,
                    velocity: [position[1], 0.0, 0.0],
                    mass: 1.3,
                    material: 0,
                });
            }
        }
    }
    let mut fluid = Liquid::new(
        particles,
        vec![Material::CONDENSED_MILK_DEMO],
        Config {
            smoothing_radius: 0.25,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    fluid
        .configure_shear_thinning(vec![Some(ShearThinning::CONDENSED_MILK_DEMO)])
        .unwrap();
    let mut max_error = 0.0_f64;
    let mut count = 0;
    for (p, m) in fluid
        .particles()
        .iter()
        .zip(fluid.effective_materials().unwrap())
    {
        if p.position.iter().any(|x| *x <= 0.5 || *x >= 0.9) {
            continue;
        }
        // Invert the unconstrained power law to inspect its estimated strain rate.
        let rate = (m.viscosity / 10.0).powf(-4.0);
        max_error = max_error.max((rate - 1.0).abs());
        count += 1;
    }
    eprintln!("affine shear: {count} interior samples, maximum rate error {max_error}");
    assert!(count > 0);
    assert!(max_error < 1e-10);
}

#[test]
fn affine_strain_fit_is_exact_on_irregular_cloud_and_ignores_spin_and_translation() {
    use physics::liquid::ShearThinning;
    let positions = [
        [0.1, 0.2, 0.3],
        [-0.3, 0.2, 0.1],
        [0.2, -0.1, 0.1],
        [0.1, 0.1, -0.2],
        [-0.1, -0.2, -0.1],
        [0.3, -0.2, 0.2],
        [-0.2, 0.1, -0.3],
        [0.2, 0.3, -0.1],
    ];
    let make = |spin: f64, translation: [f64; 3]| {
        let particles = positions
            .into_iter()
            .map(|p| Particle {
                position: p,
                mass: 1.3,
                material: 0,
                velocity: [
                    0.3 * p[0] + 0.4 * p[1] + 0.2 * p[2] - spin * p[1] + translation[0],
                    0.4 * p[0] + 0.1 * p[1] - 0.1 * p[2] + spin * p[0] + translation[1],
                    0.2 * p[0] - 0.1 * p[1] - 0.2 * p[2] + translation[2],
                ],
            })
            .collect();
        let mut f = Liquid::new(
            particles,
            vec![Material::CONDENSED_MILK_DEMO],
            Config {
                smoothing_radius: 1.0,
                gravity: [0.0; 3],
                ..Config::default()
            },
        )
        .unwrap();
        f.configure_shear_thinning(vec![Some(ShearThinning::CONDENSED_MILK_DEMO)])
            .unwrap();
        f
    };
    let expected = 1.12_f64.sqrt();
    for f in [make(0.0, [0.0; 3]), make(2.0, [3.0, -4.0, 5.0])] {
        for m in f.effective_materials().unwrap() {
            assert!(((m.viscosity / 10.0).powf(-4.0) - expected).abs() < 1e-10);
        }
    }
}
