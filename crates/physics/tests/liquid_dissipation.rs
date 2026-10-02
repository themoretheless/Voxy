use physics::liquid::{
    Config, Error, Liquid, LiquidField, Material, Particle, PhaseChange, TransportMaterial,
};
fn fluid(phase: bool) -> Liquid {
    let material = Material {
        rest_density: 1000.0,
        sound_speed: 2.0,
        viscosity: 100.0,
    };
    let mut fluid = Liquid::new(
        vec![
            Particle {
                position: [-0.1, 0.0, 0.0],
                velocity: [1.0, 0.0, 0.0],
                mass: 1.0,
                material: 0,
            },
            Particle {
                position: [0.1, 0.0, 0.0],
                velocity: [-0.5, 0.0, 0.0],
                mass: 2.0,
                material: 0,
            },
        ],
        vec![material],
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
                    temperature: 10.0,
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
    if phase {
        fluid
            .configure_phase_change(
                vec![Some(PhaseChange {
                    temperature: 10.0,
                    latent_heat: 10.0,
                    high_phase: material,
                })],
                vec![0.5; 2],
            )
            .unwrap();
    }
    fluid.set_viscous_heating(true).unwrap();
    fluid
}
fn kinetic(fluid: &Liquid) -> f64 {
    fluid
        .particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum()
}
fn energy(fluid: &Liquid) -> f64 {
    kinetic(fluid) + fluid.transport_totals().unwrap().unwrap().0
}
#[test]
fn viscosity_conserves_kinetic_plus_thermal_energy_and_pair_momentum() {
    let mut fluid = fluid(false);
    let initial = energy(&fluid);
    for _ in 0..10 {
        fluid.step(0.01, None).unwrap();
    }
    assert!(kinetic(&fluid) < 0.01);
    assert!((energy(&fluid) - initial).abs() < 1e-10);
    let momentum: f64 = fluid
        .particles()
        .iter()
        .map(|p| p.mass * p.velocity[0])
        .sum();
    assert!(momentum.abs() < 1e-12);
    assert!(fluid.fields().unwrap().iter().all(|f| f.temperature > 10.0));
}
#[test]
fn viscous_loss_can_change_phase_fraction_without_raising_temperature() {
    let mut fluid = fluid(true);
    let initial = energy(&fluid);
    fluid.step(0.01, None).unwrap();
    assert!((energy(&fluid) - initial).abs() < 1e-10);
    assert!(fluid.phase_fractions().unwrap().iter().all(|f| *f > 0.5));
    assert!(
        fluid
            .fields()
            .unwrap()
            .iter()
            .all(|f| (f.temperature - 10.0).abs() < 1e-12)
    );
}
#[test]
fn disabling_heating_retains_previous_viscosity_mode_and_requires_fields() {
    let mut fluid = fluid(false);
    fluid.set_viscous_heating(false).unwrap();
    fluid.step(0.01, None).unwrap();
    assert!(kinetic(&fluid) < 0.75);
    assert!(
        fluid
            .fields()
            .unwrap()
            .iter()
            .all(|f| (f.temperature - 10.0).abs() < 1e-12)
    );
    let mut bare = Liquid::new(
        fluid.particles().to_vec(),
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    let before = bare.clone();
    assert_eq!(bare.set_viscous_heating(true), Err(Error::InvalidTransport));
    assert_eq!(bare, before);
}
#[test]
fn failure_after_heating_rolls_back_both_velocity_and_enthalpy() {
    let mut fluid = fluid(false);
    fluid
        .configure_property_response(vec![Some(physics::liquid::PropertyResponse {
            reference_temperature: 10.0,
            thermal_expansion: -100.0,
            viscosity_temperature_rate: 0.0,
            solute: None,
        })])
        .unwrap();
    let before = fluid.clone();
    assert_eq!(fluid.step(0.01, None), Err(Error::InvalidPropertyResponse));
    assert_eq!(fluid, before);
}

fn angular(fluid: &Liquid) -> [f64; 3] {
    std::array::from_fn(|a| {
        fluid
            .particles()
            .iter()
            .map(|p| {
                let b = (a + 1) % 3;
                let c = (a + 2) % 3;
                p.mass * (p.position[b] * p.velocity[c] - p.position[c] * p.velocity[b])
            })
            .sum()
    })
}
fn oblique_fluid(heating: bool) -> Liquid {
    let mut fluid = Liquid::new(
        vec![
            Particle {
                position: [-0.1, 0.2, 0.05],
                velocity: [0.3, -0.2, 0.1],
                mass: 1.0,
                material: 0,
            },
            Particle {
                position: [0.1, -0.05, 0.1],
                velocity: [-0.4, 0.1, 0.6],
                mass: 2.0,
                material: 0,
            },
            Particle {
                position: [0.0, 0.0, -0.2],
                velocity: [0.2, 0.3, -0.4],
                mass: 0.7,
                material: 0,
            },
        ],
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 2.0,
            viscosity: 10.0,
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
                    temperature: 10.0,
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
    fluid.set_viscous_heating(heating).unwrap();
    fluid
}
#[test]
fn oblique_many_particle_viscosity_preserves_angular_momentum_and_heats() {
    let mut fluid = oblique_fluid(true);
    let initial_energy = energy(&fluid);
    let initial_angular = angular(&fluid);
    let initial_kinetic = kinetic(&fluid);
    for _ in 0..20 {
        fluid.step(0.001, None).unwrap();
    }
    assert!(kinetic(&fluid) < initial_kinetic);
    assert!((energy(&fluid) - initial_energy).abs() < 1e-10);
    for (a, value) in initial_angular.into_iter().enumerate() {
        assert!((angular(&fluid)[a] - value).abs() < 1e-12);
    }
}
#[test]
fn explicit_oblique_viscosity_preserves_angular_momentum_without_heating() {
    let mut fluid = oblique_fluid(false);
    let initial = angular(&fluid);
    let thermal = fluid.transport_totals().unwrap().unwrap().0;
    for _ in 0..20 {
        fluid.step(0.001, None).unwrap();
    }
    for (a, value) in initial.into_iter().enumerate() {
        assert!((angular(&fluid)[a] - value).abs() < 1e-12);
    }
    assert!((fluid.transport_totals().unwrap().unwrap().0 - thermal).abs() < 1e-12);
}
#[test]
fn rigid_spin_has_no_internal_viscous_force_or_spurious_heating() {
    let mut fluid = Liquid::new(
        vec![
            Particle {
                position: [-0.1, 0.0, 0.0],
                velocity: [0.0, -0.1, 0.0],
                mass: 1.0,
                material: 0,
            },
            Particle {
                position: [0.1, 0.0, 0.0],
                velocity: [0.0, 0.1, 0.0],
                mass: 2.0,
                material: 0,
            },
            Particle {
                position: [0.0, 0.2, 0.0],
                velocity: [-0.2, 0.0, 0.0],
                mass: 0.7,
                material: 0,
            },
        ],
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 2.0,
            viscosity: 100.0,
        }],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 1.0,
            ..Config::default()
        },
    )
    .unwrap();
    let diagnostic = fluid.diagnostics().unwrap();
    assert!(
        diagnostic
            .accelerations
            .iter()
            .flatten()
            .all(|a| a.abs() < 1e-12)
    );
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 10.0,
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
    fluid.set_viscous_heating(true).unwrap();
    let before = fluid.particles().to_vec();
    let thermal = fluid.transport_totals().unwrap().unwrap().0;
    fluid.step(1e-5, None).unwrap();
    for (p, original) in fluid.particles().iter().zip(before) {
        for a in 0..3 {
            assert!((p.velocity[a] - original.velocity[a]).abs() < 1e-12);
        }
    }
    assert!((fluid.transport_totals().unwrap().unwrap().0 - thermal).abs() < 1e-12);
}
#[test]
fn central_viscosity_matches_interior_divergence_free_quadratic_shear() {
    let spacing: f64 = 0.1;
    let mut particles = Vec::new();
    let mut center = 0;
    for x in -8..=8 {
        for y in -8..=8 {
            for z in -8..=8 {
                if x == 0 && y == 0 && z == 0 {
                    center = particles.len();
                }
                let position = [
                    f64::from(x) * spacing,
                    f64::from(y) * spacing,
                    f64::from(z) * spacing,
                ];
                particles.push(Particle {
                    position,
                    velocity: [0.0, position[0].powi(2), 0.0],
                    mass: 1000.0 * spacing.powi(3),
                    material: 0,
                });
            }
        }
    }
    let fluid = Liquid::new(
        particles,
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 2.0,
            viscosity: 10.0,
        }],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 0.4,
            max_pairs: 2_000_000,
            max_neighbor_checks: 20_000_000,
            ..Config::default()
        },
    )
    .unwrap();
    let diagnostic = fluid.diagnostics().unwrap();
    let acceleration = diagnostic.accelerations[center];
    // mu/rho * laplacian(v_y) = (10/1000)*2 = 0.02.
    assert!(
        (acceleration[1] - 0.02).abs() < 0.002,
        "actual acceleration {acceleration:?}"
    );
    assert!(acceleration[0].abs() < 1e-10 && acceleration[2].abs() < 1e-10);
}

#[test]
#[allow(clippy::float_cmp)] // Frozen viscous stages must preserve positions exactly.
fn whipped_cream_yield_term_increases_damping_and_conserves_heat_and_momentum() {
    use physics::liquid::WhippedCreamProfile;
    let profile = WhippedCreamProfile::DEMO;
    let mut particles = Vec::new();
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                particles.push(Particle {
                    position: [f64::from(x) * 0.1, f64::from(y) * 0.1, f64::from(z) * 0.1],
                    velocity: [0.1 * f64::from(y), 0.0, 0.0],
                    mass: 0.5,
                    material: 0,
                });
            }
        }
    }
    let mut cream = Liquid::new(
        particles,
        vec![profile.material],
        Config {
            smoothing_radius: 0.5,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    cream
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 280.0,
                    concentration: 0.0,
                };
                27
            ],
            vec![TransportMaterial {
                specific_heat: 2000.0,
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    cream
        .configure_herschel_bulkley(&[Some(profile.rheology)])
        .unwrap();
    let mut no_yield = cream.clone();
    no_yield
        .configure_shear_thinning(vec![Some(profile.rheology.shear_thinning)])
        .unwrap();
    let initial_kinetic = kinetic(&cream);
    let positions: Vec<_> = cream.particles().iter().map(|p| p.position).collect();
    for _ in 0..20 {
        cream.relax_viscosity_symmetric(0.01).unwrap();
        no_yield.relax_viscosity_symmetric(0.01).unwrap();
    }
    let loss = initial_kinetic - kinetic(&cream);
    eprintln!(
        "cream relaxation: initial_ke={initial_kinetic}, final_ke={}, no_yield_ke={}, loss={loss}",
        kinetic(&cream),
        kinetic(&no_yield)
    );
    assert!(loss > 0.0);
    assert!(kinetic(&cream) < kinetic(&no_yield));
    let heat: f64 = cream
        .particles()
        .iter()
        .zip(cream.fields().unwrap())
        .map(|(p, f)| p.mass * 2000.0 * (f.temperature - 280.0))
        .sum();
    assert!(
        (heat - loss).abs() < 2e-6 * loss,
        "heat={heat}, loss={loss}"
    );
    let mut momentum = [0.0; 3];
    let mut angular = [0.0; 3];
    for (p, position) in cream.particles().iter().zip(positions) {
        assert_eq!(p.position, position);
        for (total, speed) in momentum.iter_mut().zip(p.velocity) {
            *total += p.mass * speed;
        }
        angular[0] += p.mass * (p.position[1] * p.velocity[2] - p.position[2] * p.velocity[1]);
        angular[1] += p.mass * (p.position[2] * p.velocity[0] - p.position[0] * p.velocity[2]);
        angular[2] += p.mass * (p.position[0] * p.velocity[1] - p.position[1] * p.velocity[0]);
    }
    assert!(momentum.iter().all(|v| v.abs() < 1e-12));
    // Initial z angular momentum is -sum(m*y*v_x)=-0.09.
    assert!(angular[0].abs() < 1e-12 && angular[1].abs() < 1e-12);
    assert!((angular[2] + 0.09).abs() < 1e-12);
}

#[test]
fn exact_heated_viscosity_does_not_require_explicit_diffusion_substeps() {
    let mut exact = fluid(false);
    // This coefficient would require many explicit diffusion stability steps.
    let mut particles = exact.particles().to_vec();
    particles[0].velocity = [1.0, 0.0, 0.0];
    particles[1].velocity = [-0.5, 0.0, 0.0];
    exact = Liquid::new(
        particles,
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 2.0,
            viscosity: 1e8,
        }],
        Config {
            smoothing_radius: 1.0,
            gravity: [0.0; 3],
            max_substeps: 1,
            ..Config::default()
        },
    )
    .unwrap();
    exact
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 10.0,
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
    let mut explicit = exact.clone();
    let before = explicit.clone();
    assert_eq!(explicit.step(0.01, None), Err(Error::SubstepBudget));
    assert_eq!(explicit, before);
    exact.set_viscous_heating(true).unwrap();
    let initial = kinetic(&exact);
    let result = exact.step(0.01, None).unwrap();
    assert_eq!(result.substeps, 1);
    assert!(kinetic(&exact) < initial);
    let heat: f64 = exact
        .particles()
        .iter()
        .zip(exact.fields().unwrap())
        .map(|(p, f)| p.mass * (f.temperature - 10.0))
        .sum();
    assert!((heat + kinetic(&exact) - initial).abs() < 1e-12);
    let momentum: f64 = exact
        .particles()
        .iter()
        .map(|p| p.mass * p.velocity[0])
        .sum();
    assert!(momentum.abs() < 1e-12);
}
