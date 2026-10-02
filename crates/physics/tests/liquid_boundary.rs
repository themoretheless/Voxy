use physics::liquid::{BoundarySample, Config, Error, Liquid, Material, Particle};
fn material() -> Material {
    Material {
        rest_density: 1.0,
        sound_speed: 1.0,
        viscosity: 0.0,
    }
}
fn particle(x: f64) -> Particle {
    Particle {
        position: [x, 0.0, 0.0],
        velocity: [0.0; 3],
        mass: 1.0,
        material: 0,
    }
}
fn fluid(config: Config) -> Liquid {
    Liquid::new(vec![particle(0.1)], vec![material()], config).unwrap()
}
fn config() -> Config {
    Config {
        gravity: [0.0; 3],
        smoothing_radius: 1.0,
        ..Config::default()
    }
}
#[test]
fn boundary_support_matches_a_mirrored_fluid_neighbor_and_reaction_balances_impulse() {
    let mut wall = fluid(config());
    wall.configure_boundaries(vec![BoundarySample {
        position: [-0.1, 0.0, 0.0],
        volume: 1.0,
    }])
    .unwrap();
    let mut mirror = Liquid::new(
        vec![particle(0.1), particle(-0.1)],
        vec![material()],
        config(),
    )
    .unwrap();
    let diagnostic = wall.boundary_diagnostics().unwrap();
    let reference = mirror.boundary_diagnostics().unwrap();
    assert!((diagnostic.particle_densities[0] - reference.particle_densities[0]).abs() < 1e-12);
    wall.step(0.001, None).unwrap();
    mirror.step(0.001, None).unwrap();
    assert!(wall.particles()[0].velocity[0] > 0.0);
    assert!((wall.particles()[0].velocity[0] - mirror.particles()[0].velocity[0]).abs() < 1e-12);
    assert!(
        (wall.particles()[0].mass * wall.particles()[0].velocity[0]
            + 0.001 * diagnostic.reaction_forces[0][0])
            .abs()
            < 1e-12
    );
}
#[test]
fn support_is_local_and_does_not_change_fluid_mass_or_configuration_on_query() {
    let mut wall = fluid(config());
    let initial = wall.boundary_diagnostics().unwrap();
    wall.configure_boundaries(vec![BoundarySample {
        position: [-2.0, 0.0, 0.0],
        volume: 1.0,
    }])
    .unwrap();
    let before = wall.clone();
    let diagnostic = wall.boundary_diagnostics().unwrap();
    assert_eq!(wall, before);
    assert_eq!(diagnostic.particle_densities, initial.particle_densities);
    assert!(
        diagnostic.reaction_forces[0]
            .iter()
            .all(|value| value.abs() < 1e-12)
    );
    wall.step(0.01, None).unwrap();
    assert!((wall.mass() - 1.0).abs() < 1e-12);
}
#[test]
fn invalid_configuration_and_work_exhaustion_are_atomic() {
    let mut wall = fluid(Config {
        max_neighbor_checks: 1,
        max_particles: 2,
        ..config()
    });
    let before = wall.clone();
    assert_eq!(
        wall.configure_boundaries(vec![BoundarySample {
            position: [0.0; 3],
            volume: -1.0
        }]),
        Err(Error::InvalidBoundary)
    );
    assert_eq!(wall, before);
    wall.configure_boundaries(vec![
        BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 1.0
        };
        2
    ])
    .unwrap();
    let before = wall.clone();
    assert_eq!(wall.step(0.01, None), Err(Error::BoundaryBudget));
    assert_eq!(wall, before);
    let mut pairs = fluid(Config {
        max_pairs: 1,
        ..config()
    });
    pairs
        .configure_boundaries(vec![
            BoundarySample {
                position: [-0.1, 0.0, 0.0],
                volume: 1.0
            };
            2
        ])
        .unwrap();
    let before = pairs.clone();
    assert_eq!(pairs.step(0.01, None), Err(Error::PairBudget));
    assert_eq!(pairs, before);
}
#[test]
fn sampled_half_space_converges_to_half_the_normalized_kernel_volume() {
    let mut liquid = Liquid::new(vec![particle(0.0)], vec![material()], config()).unwrap();
    let baseline = liquid.boundary_diagnostics().unwrap().particle_densities[0];
    let mut errors = Vec::new();
    for spacing in [0.2, 0.1] {
        let samples = BoundarySample::box_grid([-1.0; 3], [0.0, 1.0, 1.0], spacing, 5000).unwrap();
        let volume: f64 = samples.iter().map(|sample| sample.volume).sum();
        assert!((volume - 4.0).abs() < 1e-10);
        liquid.configure_boundaries(samples).unwrap();
        let support = liquid.boundary_diagnostics().unwrap().particle_densities[0] - baseline;
        errors.push((support - 0.5).abs());
    }
    assert!(errors[1] < errors[0]);
    assert!(errors[1] < 0.001);
    assert_eq!(
        BoundarySample::box_grid([-1.0; 3], [1.0; 3], 0.001, 100),
        Err(Error::BoundaryBudget)
    );
}
fn thermal_wall(temperature: f64) -> Liquid {
    let mut liquid = fluid(config());
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 1.0,
        }])
        .unwrap();
    liquid
        .configure_transport(
            vec![physics::liquid::LiquidField {
                temperature,
                concentration: 0.0,
            }],
            vec![physics::liquid::TransportMaterial {
                specific_heat: 1.0,
                conductivity: 0.0,
                ..physics::liquid::TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid.set_pressure_work(true).unwrap();
    liquid
}
fn thermal_energy(liquid: &Liquid) -> f64 {
    liquid.transport_totals().unwrap().unwrap().0
        + liquid
            .particles()
            .iter()
            .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>()
}
#[test]
fn boundary_pressure_converts_thermal_energy_into_motion_without_energy_creation() {
    let mut liquid = thermal_wall(100.0);
    let before = thermal_energy(&liquid);
    for _ in 0..100 {
        liquid.step(0.001, None).unwrap();
    }
    assert!((thermal_energy(&liquid) - before).abs() < 1e-9);
    assert!(liquid.fields().unwrap()[0].temperature < 100.0);
    assert!(liquid.particles()[0].velocity[0] > 0.0);
}
#[test]
fn insufficient_boundary_pressure_energy_rolls_back_motion_and_fields() {
    let mut liquid = thermal_wall(0.0);
    let before = liquid.clone();
    assert_eq!(liquid.step(0.001, None), Err(Error::NumericalFailure));
    assert_eq!(liquid, before);
}
#[test]
fn boundary_pressure_can_draw_work_from_latent_heat_on_the_plateau() {
    let mut liquid = thermal_wall(100.0);
    liquid
        .configure_phase_change(
            vec![Some(physics::liquid::PhaseChange {
                temperature: 100.0,
                latent_heat: 10.0,
                high_phase: material(),
            })],
            vec![0.5],
        )
        .unwrap();
    let before = thermal_energy(&liquid);
    liquid.step(0.001, None).unwrap();
    assert!((thermal_energy(&liquid) - before).abs() < 1e-10);
    assert!(liquid.phase_fractions().unwrap()[0] < 0.5);
    assert!((liquid.fields().unwrap()[0].temperature - 100.0).abs() < 1e-12);
}
#[test]
fn approaching_a_pressurized_boundary_heats_the_fluid() {
    let mut liquid = thermal_wall(100.0);
    let mut incoming = liquid.particles()[0];
    incoming.velocity = [-1.0, 0.0, 0.0];
    liquid
        .exchange_particles(
            &[0],
            &[physics::liquid::ParticleInput {
                particle: incoming,
                field: Some(physics::liquid::LiquidField {
                    temperature: 100.0,
                    concentration: 0.0,
                }),
                phase_fraction: None,
            }],
        )
        .unwrap();
    let before = thermal_energy(&liquid);
    liquid.step(0.001, None).unwrap();
    assert!(liquid.fields().unwrap()[0].temperature > 100.0);
    assert!((thermal_energy(&liquid) - before).abs() < 1e-10);
}
#[test]
fn interior_boundary_pressure_and_viscous_heat_share_one_energy_balance() {
    let mut first = particle(0.1);
    first.velocity = [1.0, 0.0, 0.0];
    let mut second = particle(0.3);
    second.mass = 2.0;
    second.velocity = [-0.5, 0.0, 0.0];
    let mut liquid = Liquid::new(
        vec![first, second],
        vec![Material {
            viscosity: 1.0,
            ..material()
        }],
        config(),
    )
    .unwrap();
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 1.0,
        }])
        .unwrap();
    liquid
        .configure_transport(
            vec![
                physics::liquid::LiquidField {
                    temperature: 100.0,
                    concentration: 0.0
                };
                2
            ],
            vec![physics::liquid::TransportMaterial {
                specific_heat: 1.0,
                conductivity: 0.0,
                ..physics::liquid::TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid.set_pressure_work(true).unwrap();
    liquid.set_viscous_heating(true).unwrap();
    let before = thermal_energy(&liquid);
    for _ in 0..100 {
        liquid.step(0.001, None).unwrap();
    }
    assert!((thermal_energy(&liquid) - before).abs() < 1e-8);
}
#[test]
fn distant_boundary_geometry_does_not_consume_local_neighbor_budget() {
    let mut liquid = fluid(Config {
        max_neighbor_checks: 1,
        ..config()
    });
    let mut samples = vec![BoundarySample {
        position: [-0.1, 0.0, 0.0],
        volume: 1.0,
    }];
    for offset in 0..1000 {
        samples.push(BoundarySample {
            position: [10.0 + f64::from(offset), 0.0, 0.0],
            volume: 1.0,
        });
    }
    liquid.configure_boundaries(samples).unwrap();
    let diagnostic = liquid.boundary_diagnostics().unwrap();
    assert!(diagnostic.reaction_forces[0][0] < 0.0);
    assert!(
        diagnostic.reaction_forces[1..]
            .iter()
            .all(|force| force.iter().all(|component| component.abs() < 1e-12))
    );
    liquid.step(0.001, None).unwrap();
}
#[test]
fn grid_search_matches_full_kernel_sum_across_cell_boundaries() {
    let mut liquid = fluid(config());
    let samples = vec![
        BoundarySample {
            position: [0.6, 0.2, 0.0],
            volume: 0.2,
        },
        BoundarySample {
            position: [-0.3, -0.1, 0.2],
            volume: 0.1,
        },
        BoundarySample {
            position: [1.05, 0.0, 0.0],
            volume: 0.1,
        },
        BoundarySample {
            position: [-2.0, 0.0, 0.0],
            volume: 2.0,
        },
    ];
    let baseline = liquid.boundary_diagnostics().unwrap().particle_densities[0];
    let mut expected = baseline;
    for sample in &samples {
        let radius_squared: f64 = sample
            .position
            .iter()
            .zip(liquid.particles()[0].position)
            .map(|(a, b)| (a - b).powi(2))
            .sum();
        expected += sample.volume * 315.0 / (64.0 * std::f64::consts::PI)
            * (1.0 - radius_squared).max(0.0).powi(3);
    }
    liquid.configure_boundaries(samples).unwrap();
    assert!(
        (liquid.boundary_diagnostics().unwrap().particle_densities[0] - expected).abs() < 1e-12
    );
    let before = liquid.clone();
    assert_eq!(
        liquid.configure_boundaries(vec![BoundarySample {
            position: [1e100, 0.0, 0.0],
            volume: 1.0
        }]),
        Err(Error::NumericalFailure)
    );
    assert_eq!(liquid, before);
}
#[test]
fn boundary_viscosity_damps_tangential_motion_and_deposits_lost_energy_as_heat() {
    let mut moving = particle(0.1);
    moving.velocity = [0.0, 1.0, 0.0];
    let mut liquid = Liquid::new(
        vec![moving],
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 1.0,
            viscosity: 100.0,
        }],
        config(),
    )
    .unwrap();
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 0.1,
        }])
        .unwrap();
    liquid
        .configure_transport(
            vec![physics::liquid::LiquidField {
                temperature: 10.0,
                concentration: 0.0,
            }],
            vec![physics::liquid::TransportMaterial {
                specific_heat: 1.0,
                conductivity: 0.0,
                ..physics::liquid::TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid.set_viscous_heating(true).unwrap();
    let before = thermal_energy(&liquid);
    liquid.step(0.01, None).unwrap();
    assert!(liquid.particles()[0].velocity[1] < 1.0);
    assert!(liquid.fields().unwrap()[0].temperature > 10.0);
    assert!((thermal_energy(&liquid) - before).abs() < 1e-10);
    assert!(liquid.particles()[0].velocity[0].abs() < 1e-12);
}
#[test]
fn explicit_wall_viscosity_reaction_balances_the_lost_particle_momentum() {
    let mut moving = particle(0.1);
    moving.velocity = [0.0, 1.0, 0.0];
    let mut liquid = Liquid::new(
        vec![moving],
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 1.0,
            viscosity: 100.0,
        }],
        config(),
    )
    .unwrap();
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 0.1,
        }])
        .unwrap();
    let diagnostic = liquid.boundary_diagnostics().unwrap();
    assert!(diagnostic.viscous_reaction_forces[0][1] > 0.0);
    liquid.step(0.001, None).unwrap();
    let impulse = moving.mass * (liquid.particles()[0].velocity[1] - moving.velocity[1]);
    assert!((impulse + 0.001 * diagnostic.viscous_reaction_forces[0][1]).abs() < 1e-12);
}
#[test]
fn boundary_viscosity_substep_exhaustion_restores_already_deposited_heat() {
    let mut moving = particle(0.1);
    moving.velocity = [0.0, 1.0, 0.0];
    let mut liquid = Liquid::new(
        vec![moving],
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 1.0,
            viscosity: 1e5,
        }],
        Config {
            max_substeps: 1,
            ..config()
        },
    )
    .unwrap();
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 0.1,
        }])
        .unwrap();
    liquid
        .configure_transport(
            vec![physics::liquid::LiquidField {
                temperature: 10.0,
                concentration: 0.0,
            }],
            vec![physics::liquid::TransportMaterial {
                specific_heat: 1.0,
                conductivity: 0.0,
                ..physics::liquid::TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid.set_viscous_heating(true).unwrap();
    let before = liquid.clone();
    let mut short_step = before.clone();
    short_step.step(0.0001, None).unwrap();
    assert!(short_step.fields().unwrap()[0].temperature > 10.0);
    assert_eq!(liquid.step(0.01, None), Err(Error::SubstepBudget));
    assert_eq!(liquid, before);
}
fn driven_wall(samples: Vec<BoundarySample>, velocities: Vec<[f64; 3]>) -> Liquid {
    let mut liquid = Liquid::new(
        vec![particle(0.1)],
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 1.0,
            viscosity: 100.0,
        }],
        config(),
    )
    .unwrap();
    liquid.configure_boundaries(samples).unwrap();
    liquid.configure_boundary_velocities(velocities).unwrap();
    liquid
        .configure_transport(
            vec![physics::liquid::LiquidField {
                temperature: 10.0,
                concentration: 0.0,
            }],
            vec![physics::liquid::TransportMaterial {
                specific_heat: 1.0,
                conductivity: 0.0,
                ..physics::liquid::TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid.set_viscous_heating(true).unwrap();
    liquid
}
#[test]
fn prescribed_wall_motion_supplies_mechanical_work_and_viscous_heat() {
    let mut liquid = driven_wall(
        vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 0.1,
        }],
        vec![[0.0, 2.0, 0.0]],
    );
    let before = thermal_energy(&liquid);
    let reaction = liquid.boundary_diagnostics().unwrap();
    assert!(reaction.viscous_reaction_forces[0][1] < 0.0);
    liquid.step(0.01, None).unwrap();
    let particle = liquid.particles()[0];
    let supplied_work = 2.0 * particle.mass * particle.velocity[1];
    assert!(particle.velocity[1] > 0.0 && particle.velocity[1] < 2.0);
    assert!(liquid.fields().unwrap()[0].temperature > 10.0);
    assert!((thermal_energy(&liquid) - before - supplied_work).abs() < 1e-10);
}
#[test]
fn opposing_wall_velocities_heat_fluid_without_net_translation() {
    let mut liquid = driven_wall(
        vec![
            BoundarySample {
                position: [-0.1, 0.0, 0.0],
                volume: 0.1,
            },
            BoundarySample {
                position: [0.3, 0.0, 0.0],
                volume: 0.1,
            },
        ],
        vec![[0.0, 2.0, 0.0], [0.0, -2.0, 0.0]],
    );
    let diagnostic = liquid.boundary_diagnostics().unwrap();
    let supplied_work = -(2.0 * diagnostic.viscous_reaction_forces[0][1]
        - 2.0 * diagnostic.viscous_reaction_forces[1][1])
        * 0.01;
    let before = thermal_energy(&liquid);
    liquid.step(0.01, None).unwrap();
    assert!((thermal_energy(&liquid) - before - supplied_work).abs() < 1e-10);
    assert!(
        liquid.particles()[0]
            .velocity
            .iter()
            .all(|speed| speed.abs() < 1e-12)
    );
    assert!(liquid.fields().unwrap()[0].temperature > 10.0);
}
#[test]
fn invalid_surface_velocity_is_atomic_and_geometry_replacement_resets_motion() {
    let sample = BoundarySample {
        position: [-0.1, 0.0, 0.0],
        volume: 0.1,
    };
    let mut liquid = driven_wall(vec![sample], vec![[0.0, 2.0, 0.0]]);
    let before = liquid.clone();
    assert_eq!(
        liquid.configure_boundary_velocities(vec![]),
        Err(Error::InvalidBoundary)
    );
    assert_eq!(
        liquid.configure_boundary_velocities(vec![[f64::NAN, 0.0, 0.0]]),
        Err(Error::InvalidBoundary)
    );
    assert_eq!(liquid, before);
    liquid.configure_boundaries(vec![sample]).unwrap();
    liquid.step(0.01, None).unwrap();
    assert!(
        liquid.particles()[0]
            .velocity
            .iter()
            .all(|speed| speed.abs() < 1e-12)
    );
}
#[test]
fn fast_prescribed_surface_restricts_substep_displacement_and_rolls_back_on_budget() {
    let mut liquid = Liquid::new(
        vec![particle(0.1)],
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 1.0,
            viscosity: 100.0,
        }],
        Config {
            max_substeps: 1,
            ..config()
        },
    )
    .unwrap();
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 0.1,
        }])
        .unwrap();
    liquid
        .configure_boundary_velocities(vec![[0.0, 1e6, 0.0]])
        .unwrap();
    liquid
        .configure_transport(
            vec![physics::liquid::LiquidField {
                temperature: 10.0,
                concentration: 0.0,
            }],
            vec![physics::liquid::TransportMaterial {
                specific_heat: 1.0,
                conductivity: 0.0,
                ..physics::liquid::TransportMaterial::default()
            }],
        )
        .unwrap();
    liquid.set_viscous_heating(true).unwrap();
    let before = liquid.clone();
    assert_eq!(liquid.step(0.01, None), Err(Error::SubstepBudget));
    assert_eq!(liquid, before);
}
#[test]
fn boundary_translation_rebuilds_index_and_preserves_surface_velocity() {
    let mut liquid = driven_wall(
        vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 0.1,
        }],
        vec![[0.0, 2.0, 0.0]],
    );
    let before_density = liquid.boundary_diagnostics().unwrap().particle_densities[0];
    let particles = liquid.particles().to_vec();
    let fields = liquid.fields().unwrap().to_vec();
    liquid.translate_boundaries([0.1, 0.0, 0.0]).unwrap();
    let moved = liquid.boundary_diagnostics().unwrap();
    assert!(moved.particle_densities[0] > before_density);
    assert!(moved.viscous_reaction_forces[0][1] < 0.0);
    assert_eq!(liquid.particles(), particles);
    assert_eq!(liquid.fields().unwrap(), fields);
    liquid.translate_boundaries([10.0, 0.0, 0.0]).unwrap();
    let far = liquid.boundary_diagnostics().unwrap();
    assert!(
        far.viscous_reaction_forces[0]
            .iter()
            .all(|force| force.abs() < 1e-12)
    );
    liquid.translate_boundaries([-10.1, 0.0, 0.0]).unwrap();
    assert!(
        (liquid.boundary_diagnostics().unwrap().particle_densities[0] - before_density).abs()
            < 1e-10
    );
}
#[test]
fn unrepresentable_boundary_translation_preserves_geometry_index_and_velocity() {
    let mut liquid = driven_wall(
        vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 0.1,
        }],
        vec![[0.0, 2.0, 0.0]],
    );
    let before = liquid.clone();
    assert_eq!(
        liquid.translate_boundaries([f64::NAN, 0.0, 0.0]),
        Err(Error::InvalidBoundary)
    );
    assert_eq!(liquid, before);
    assert_eq!(
        liquid.translate_boundaries([1e100, 0.0, 0.0]),
        Err(Error::NumericalFailure)
    );
    assert_eq!(liquid, before);
}

fn boundary_body(mass: f64) -> physics::liquid::TranslatingBody {
    physics::liquid::TranslatingBody {
        position: [0.0; 3],
        velocity: [0.0; 3],
        mass,
    }
}
fn supported_fluid() -> Liquid {
    let mut liquid = fluid(config());
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 1.0,
        }])
        .unwrap();
    liquid
}
fn attach_thermal(liquid: &mut Liquid, temperature: f64) {
    liquid
        .configure_transport(
            vec![
                physics::liquid::LiquidField {
                    temperature,
                    concentration: 0.0
                };
                liquid.particles().len()
            ],
            vec![physics::liquid::TransportMaterial {
                specific_heat: 1.0,
                conductivity: 0.0,
                ..physics::liquid::TransportMaterial::default()
            }],
        )
        .unwrap();
}
fn body_energy(body: physics::liquid::TranslatingBody) -> f64 {
    0.5 * body.mass * body.velocity.iter().map(|v| v * v).sum::<f64>()
}
#[test]
fn sampled_body_pressure_recoils_and_moves_samples_with_body() {
    let mut liquid = supported_fluid();
    let force = liquid.boundary_diagnostics().unwrap().reaction_forces[0];
    let mut body = boundary_body(3.0);
    let report = liquid.step_with_boundary_body(0.001, &mut body).unwrap();
    assert_eq!(report.fluid.substeps, 1);
    let p = liquid.particles()[0];
    for (a, expected_force) in force.into_iter().enumerate() {
        assert!((p.mass * p.velocity[a] + body.mass * body.velocity[a]).abs() < 1e-12);
        assert!((report.pressure_impulse[a] - expected_force * 0.001).abs() < 1e-12);
        assert!((body.position[a] - body.velocity[a] * 0.001).abs() < 1e-12);
    }
    // Compare actual sample coordinates by removing exactly the returned translation.
    liquid
        .translate_boundaries(body.position.map(|v| -v))
        .unwrap();
    liquid
        .configure_boundary_velocities(vec![[0.0; 3]])
        .unwrap();
    let mut expected = Liquid::new(vec![p], vec![material()], config()).unwrap();
    expected
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 1.0,
        }])
        .unwrap();
    assert_eq!(
        liquid.boundary_diagnostics().unwrap(),
        expected.boundary_diagnostics().unwrap()
    );
}
#[test]
fn sampled_body_pressure_work_conserves_combined_thermal_and_kinetic_energy() {
    let mut liquid = supported_fluid();
    attach_thermal(&mut liquid, 10.0);
    liquid.set_pressure_work(true).unwrap();
    let mut body = boundary_body(0.3);
    body.velocity = [0.3, -0.2, 0.1];
    let initial = thermal_energy(&liquid) + body_energy(body);
    let report = liquid.step_with_boundary_body(0.02, &mut body).unwrap();
    assert!(report.pressure_work.abs() > 1e-8);
    assert!((thermal_energy(&liquid) + body_energy(body) - initial).abs() < 1e-10);
    let p = liquid.particles()[0];
    assert!((p.velocity[0] + body.mass * body.velocity[0] - 0.09).abs() < 1e-12);
}
#[test]
fn sampled_body_viscosity_heats_only_relative_kinetic_energy() {
    let mut p = particle(0.1);
    p.velocity = [0.0, 1.0, 0.0];
    let mut liquid = Liquid::new(
        vec![p],
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 1.0,
            viscosity: 100.0,
        }],
        config(),
    )
    .unwrap();
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 0.1,
        }])
        .unwrap();
    attach_thermal(&mut liquid, 10.0);
    liquid.set_viscous_heating(true).unwrap();
    let mut body = boundary_body(0.1);
    let initial = thermal_energy(&liquid);
    let report = liquid.step_with_boundary_body(0.01, &mut body).unwrap();
    assert!(report.viscous_heat > 0.0);
    assert!(body.velocity[1] > 0.0);
    assert!((liquid.particles()[0].velocity[1] + body.mass * body.velocity[1] - 1.0).abs() < 1e-12);
    assert!((thermal_energy(&liquid) + body_energy(body) - initial).abs() < 1e-10);
    assert!((report.viscous_heat - (liquid.fields().unwrap()[0].temperature - 10.0)).abs() < 1e-12);
    assert!(liquid.particles()[0].velocity[1] >= body.velocity[1]);
}
#[test]
fn sampled_body_substep_failure_restores_geometry_fields_and_body() {
    let mut liquid = fluid(Config {
        max_substeps: 1,
        ..config()
    });
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 1.0,
        }])
        .unwrap();
    attach_thermal(&mut liquid, 10.0);
    liquid.set_pressure_work(true).unwrap();
    let mut body = boundary_body(0.1);
    body.velocity = [100.0, 0.0, 0.0];
    let before = liquid.clone();
    let before_body = body;
    assert_eq!(
        liquid.step_with_boundary_body(0.01, &mut body),
        Err(Error::SubstepBudget)
    );
    assert_eq!(liquid, before);
    assert_eq!(body, before_body);
}
#[test]
fn empty_fluid_still_advances_sampled_body_under_gravity() {
    let mut liquid = Liquid::new(
        vec![],
        vec![material()],
        Config {
            gravity: [0.0, -10.0, 0.0],
            ..config()
        },
    )
    .unwrap();
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [0.0; 3],
            volume: 0.1,
        }])
        .unwrap();
    let mut body = boundary_body(1.0);
    liquid.step_with_boundary_body(0.01, &mut body).unwrap();
    assert!((body.velocity[1] + 0.1).abs() < 1e-12);
    assert!((body.position[1] + 0.001).abs() < 1e-12);
}

#[test]
fn sampled_body_multimaterial_pressure_preserves_total_momentum_over_substeps() {
    let mut second = particle(0.3);
    second.mass = 0.8;
    second.material = 1;
    second.velocity = [0.1, -0.2, 0.3];
    let mut liquid = Liquid::new(
        vec![particle(0.1), second],
        vec![
            material(),
            Material {
                rest_density: 0.8,
                sound_speed: 1.5,
                viscosity: 0.2,
            },
        ],
        config(),
    )
    .unwrap();
    liquid
        .configure_boundaries(vec![
            BoundarySample {
                position: [-0.1, 0.0, 0.0],
                volume: 1.0,
            },
            BoundarySample {
                position: [-0.1, 0.3, 0.0],
                volume: 0.5,
            },
        ])
        .unwrap();
    let mut body = boundary_body(0.2);
    body.velocity = [-0.1, 0.2, 0.0];
    let initial: [f64; 3] = std::array::from_fn(|a| {
        body.mass * body.velocity[a]
            + liquid
                .particles()
                .iter()
                .map(|p| p.mass * p.velocity[a])
                .sum::<f64>()
    });
    let initial_body = body;
    let report = liquid.step_with_boundary_body(0.1, &mut body).unwrap();
    assert!(report.fluid.substeps > 1);
    for (a, expected) in initial.into_iter().enumerate() {
        let momentum = body.mass * body.velocity[a]
            + liquid
                .particles()
                .iter()
                .map(|p| p.mass * p.velocity[a])
                .sum::<f64>();
        assert!((momentum - expected).abs() < 1e-11);
        assert!(
            (body.mass * (body.velocity[a] - initial_body.velocity[a])
                - report.pressure_impulse[a]
                - report.viscous_impulse[a])
                .abs()
                < 1e-11
        );
    }
}

#[test]
fn sampled_body_pressure_work_is_invariant_under_uniform_velocity_shift() {
    let mut stationary = supported_fluid();
    attach_thermal(&mut stationary, 10.0);
    stationary.set_pressure_work(true).unwrap();
    stationary.set_viscous_heating(true).unwrap();
    let shift = [2.0, -3.0, 1.0];
    let mut p = particle(0.1);
    p.velocity = shift;
    let mut moving = Liquid::new(vec![p], vec![material()], config()).unwrap();
    moving
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 1.0,
        }])
        .unwrap();
    attach_thermal(&mut moving, 10.0);
    moving.set_pressure_work(true).unwrap();
    moving.set_viscous_heating(true).unwrap();
    let mut fixed_body = boundary_body(0.3);
    let mut moving_body = boundary_body(0.3);
    moving_body.velocity = shift;
    let reference = stationary
        .step_with_boundary_body(0.001, &mut fixed_body)
        .unwrap();
    let result = moving
        .step_with_boundary_body(0.001, &mut moving_body)
        .unwrap();
    assert_eq!(reference.fluid.substeps, 1);
    assert_eq!(result.fluid.substeps, 1);
    for (a, speed) in shift.into_iter().enumerate() {
        assert!(
            (moving.particles()[0].velocity[a] - stationary.particles()[0].velocity[a] - speed)
                .abs()
                < 1e-12
        );
        assert!((moving_body.velocity[a] - fixed_body.velocity[a] - speed).abs() < 1e-12);
        assert!(
            (moving.particles()[0].position[a]
                - stationary.particles()[0].position[a]
                - speed * 0.001)
                .abs()
                < 1e-12
        );
    }
    assert!((result.pressure_work - reference.pressure_work).abs() < 1e-12);
    assert!(
        (moving.fields().unwrap()[0].temperature - stationary.fields().unwrap()[0].temperature)
            .abs()
            < 1e-12
    );
}

fn rotating_body() -> physics::liquid::RotatingBody {
    physics::liquid::RotatingBody {
        translation: boundary_body(3.0),
        angular_velocity: [0.0; 3],
        moment_of_inertia: 0.1,
    }
}
fn rotating_energy(liquid: &Liquid, body: physics::liquid::RotatingBody) -> f64 {
    thermal_energy(liquid)
        + body_energy(body.translation)
        + 0.5 * body.moment_of_inertia * body.angular_velocity.iter().map(|w| w * w).sum::<f64>()
}
fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn angular_momentum(liquid: &Liquid, body: physics::liquid::RotatingBody) -> [f64; 3] {
    let orbital = cross3(
        body.translation.position,
        body.translation.velocity.map(|v| v * body.translation.mass),
    );
    std::array::from_fn(|a| {
        orbital[a]
            + body.moment_of_inertia * body.angular_velocity[a]
            + liquid
                .particles()
                .iter()
                .map(|p| cross3(p.position, p.velocity.map(|v| v * p.mass))[a])
                .sum::<f64>()
    })
}
#[test]
fn off_center_sample_pressure_spins_body_and_preserves_angular_energy_balance() {
    for formulation in [
        physics::liquid::Formulation::MassDensity,
        physics::liquid::Formulation::RestVolume,
        physics::liquid::Formulation::RestVolumeWendland,
    ] {
        let mut p = particle(0.1);
        p.position[1] = 0.2;
        let mut liquid = Liquid::new(vec![p], vec![material()], config()).unwrap();
        liquid
            .configure_boundaries(vec![BoundarySample {
                position: [-0.1, 0.2, 0.0],
                volume: 1.0,
            }])
            .unwrap();
        liquid.set_formulation(formulation);
        attach_thermal(&mut liquid, 10.0);
        liquid.set_pressure_work(true).unwrap();
        let mut body = rotating_body();
        let energy = rotating_energy(&liquid, body);
        let report = liquid
            .step_with_rotating_boundary(0.001, &mut body)
            .unwrap();
        assert!(body.angular_velocity[2] > 0.0);
        assert_eq!(report.boundary.fluid.substeps, 1);
        assert!((rotating_energy(&liquid, body) - energy).abs() < 1e-10);
        for momentum in angular_momentum(&liquid, body) {
            assert!(momentum.abs() < 1e-12);
        }
        assert!(
            (body.moment_of_inertia * body.angular_velocity[2] - report.angular_impulse[2]).abs()
                < 1e-12
        );
        let angle = body.angular_velocity[2] * 0.001;
        let expected = [
            -0.1 * angle.cos() - 0.2 * angle.sin() + body.translation.position[0],
            -0.1 * angle.sin() + 0.2 * angle.cos() + body.translation.position[1],
            0.0,
        ];
        for (a, value) in expected.into_iter().enumerate() {
            assert!((liquid.boundary_samples()[0].position[a] - value).abs() < 1e-12);
        }
    }
}

#[test]
fn rotational_viscosity_transfers_spin_and_heats_with_conserved_angular_momentum() {
    let mut liquid = Liquid::new(
        vec![particle(0.1)],
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 1.0,
            viscosity: 100.0,
        }],
        config(),
    )
    .unwrap();
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.0, 0.0],
            volume: 0.1,
        }])
        .unwrap();
    attach_thermal(&mut liquid, 10.0);
    liquid.set_viscous_heating(true).unwrap();
    let mut body = rotating_body();
    body.angular_velocity = [0.0, 0.0, 1.0];
    let energy = rotating_energy(&liquid, body);
    let initial = angular_momentum(&liquid, body);
    let report = liquid.step_with_rotating_boundary(0.01, &mut body).unwrap();
    assert!(liquid.particles()[0].velocity[1] > 0.0);
    assert!(body.angular_velocity[2] < 1.0);
    assert!(report.boundary.viscous_heat > 0.0);
    assert!((rotating_energy(&liquid, body) - energy).abs() < 1e-10);
    for (a, value) in initial.into_iter().enumerate() {
        assert!((angular_momentum(&liquid, body)[a] - value).abs() < 1e-12);
    }
}
#[test]
fn rotating_geometry_preserves_volume_and_radius_without_particles() {
    let mut liquid = Liquid::new(vec![], vec![material()], config()).unwrap();
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [0.3, 0.4, 0.0],
            volume: 0.1,
        }])
        .unwrap();
    let mut body = rotating_body();
    body.angular_velocity = [0.0, 0.0, 5.0];
    let report = liquid.step_with_rotating_boundary(0.1, &mut body).unwrap();
    assert!(report.boundary.fluid.substeps >= 2);
    let sample = liquid.boundary_samples()[0];
    assert!((sample.position[0] - (0.3 * 0.5_f64.cos() - 0.4 * 0.5_f64.sin())).abs() < 1e-12);
    assert!((sample.position.iter().map(|v| v * v).sum::<f64>() - 0.25).abs() < 1e-12);
    assert!((sample.volume - 0.1).abs() < 1e-12);
}
#[test]
fn rotational_failure_rolls_back_orientation_and_thermal_fields() {
    let mut liquid = fluid(Config {
        max_substeps: 1,
        ..config()
    });
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.2, 0.0],
            volume: 1.0,
        }])
        .unwrap();
    attach_thermal(&mut liquid, 10.0);
    liquid.set_pressure_work(true).unwrap();
    let mut body = rotating_body();
    body.angular_velocity = [0.0, 0.0, 100.0];
    let before = liquid.clone();
    let old = body;
    assert_eq!(
        liquid.step_with_rotating_boundary(0.01, &mut body),
        Err(Error::SubstepBudget)
    );
    assert_eq!(liquid, before);
    assert_eq!(body, old);
}

#[test]
fn three_dimensional_rotational_viscosity_conserves_pair_energy_and_momenta() {
    let mut p = particle(0.1);
    p.position = [0.1, 0.2, 0.3];
    p.velocity = [-0.1, 0.3, -0.2];
    let mut liquid = Liquid::new(
        vec![p],
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 1.0,
            viscosity: 100.0,
        }],
        config(),
    )
    .unwrap();
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.2, 0.3],
            volume: 0.1,
        }])
        .unwrap();
    attach_thermal(&mut liquid, 10.0);
    liquid.set_viscous_heating(true).unwrap();
    let mut body = rotating_body();
    body.angular_velocity = [0.7, -0.4, 1.0];
    body.translation.velocity = [0.2, -0.1, 0.3];
    let energy = rotating_energy(&liquid, body);
    let angular = angular_momentum(&liquid, body);
    let momentum: [f64; 3] = std::array::from_fn(|a| {
        p.mass * p.velocity[a] + body.translation.mass * body.translation.velocity[a]
    });
    let report = liquid.step_with_rotating_boundary(0.02, &mut body).unwrap();
    assert!(report.boundary.viscous_heat > 0.0);
    assert!((rotating_energy(&liquid, body) - energy).abs() < 1e-10);
    for a in 0..3 {
        assert!((angular_momentum(&liquid, body)[a] - angular[a]).abs() < 1e-12);
        let p = liquid.particles()[0];
        assert!(
            (p.mass * p.velocity[a] + body.translation.mass * body.translation.velocity[a]
                - momentum[a])
                .abs()
                < 1e-12
        );
    }
}

fn tensor_body() -> physics::liquid::TensorBody {
    physics::liquid::TensorBody {
        translation: boundary_body(3.0),
        inertia: [[0.2, 0.03, 0.01], [0.03, 0.3, -0.02], [0.01, -0.02, 0.4]],
        orientation: [1.0, 0.0, 0.0, 0.0],
        angular_momentum: [0.0; 3],
    }
}
fn tensor_energy(liquid: &Liquid, body: physics::liquid::TensorBody) -> f64 {
    thermal_energy(liquid) + body_energy(body.translation) + body.rotational_energy().unwrap()
}
fn tensor_angular(liquid: &Liquid, body: physics::liquid::TensorBody) -> [f64; 3] {
    let orbital = cross3(
        body.translation.position,
        body.translation.velocity.map(|v| v * body.translation.mass),
    );
    std::array::from_fn(|a| {
        orbital[a]
            + body.angular_momentum[a]
            + liquid
                .particles()
                .iter()
                .map(|p| cross3(p.position, p.velocity.map(|v| v * p.mass))[a])
                .sum::<f64>()
    })
}
#[test]
fn tensor_mobility_respects_body_orientation_and_off_diagonal_inertia() {
    let mut body = tensor_body();
    body.inertia = [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]];
    let half = 0.5_f64.sqrt();
    body.orientation = [half, 0.0, 0.0, half];
    body.angular_momentum = [1.0, 2.0, 3.0];
    let velocity = body.angular_velocity().unwrap();
    for (a, value) in [1.0 / 3.0, 1.0, 0.75].into_iter().enumerate() {
        assert!((velocity[a] - value).abs() < 1e-12);
    }
    body = tensor_body();
    body.angular_momentum = [0.1, -0.2, 0.3];
    let velocity = body.angular_velocity().unwrap();
    for a in 0..3 {
        let recovered: f64 = body.inertia[a]
            .iter()
            .zip(velocity)
            .map(|(i, w)| i * w)
            .sum();
        assert!((recovered - body.angular_momentum[a]).abs() < 1e-12);
    }
}
#[test]
fn asymmetric_free_rotation_preserves_energy_world_momentum_and_sample_shape() {
    let mut liquid = Liquid::new(vec![], vec![material()], config()).unwrap();
    liquid
        .configure_boundaries(vec![
            BoundarySample {
                position: [0.3, 0.4, 0.2],
                volume: 0.1,
            },
            BoundarySample {
                position: [-0.2, 0.1, 0.4],
                volume: 0.2,
            },
        ])
        .unwrap();
    let mut body = tensor_body();
    body.angular_momentum = [0.04, 0.08, 0.1];
    let original = body;
    let energy = body.rotational_energy().unwrap();
    let velocity = body.angular_velocity().unwrap();
    for _ in 0..200 {
        let report = liquid.step_with_tensor_boundary(0.01, &mut body).unwrap();
        assert!(report.angular_impulse.iter().all(|v| v.abs() < 1e-14));
    }
    assert_eq!(
        body.angular_momentum.map(f64::to_bits),
        original.angular_momentum.map(f64::to_bits)
    );
    assert!((body.rotational_energy().unwrap() - energy).abs() < 1e-11);
    let change: f64 = body
        .angular_velocity()
        .unwrap()
        .iter()
        .zip(velocity)
        .map(|(a, b)| (a - b).powi(2))
        .sum();
    assert!(change > 1e-5);
    assert!((body.orientation.iter().map(|q| q * q).sum::<f64>() - 1.0).abs() < 1e-12);
    let samples = liquid.boundary_samples();
    assert!((samples[0].position.iter().map(|v| v * v).sum::<f64>() - 0.29).abs() < 1e-11);
    let distance: f64 = samples[0]
        .position
        .iter()
        .zip(samples[1].position)
        .map(|(a, b)| (a - b).powi(2))
        .sum();
    assert!((distance - 0.38).abs() < 1e-11);
}
#[test]
fn general_tensor_pressure_work_preserves_combined_energy_and_angular_momentum() {
    let mut p = particle(0.1);
    p.position = [0.1, 0.2, 0.3];
    let mut liquid = Liquid::new(vec![p], vec![material()], config()).unwrap();
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.2, 0.3],
            volume: 1.0,
        }])
        .unwrap();
    attach_thermal(&mut liquid, 10.0);
    liquid.set_pressure_work(true).unwrap();
    let mut body = tensor_body();
    body.angular_momentum = [0.04, 0.08, 0.1];
    let initial = tensor_energy(&liquid, body);
    let angular = tensor_angular(&liquid, body);
    let original = body;
    let report = liquid.step_with_tensor_boundary(0.01, &mut body).unwrap();
    assert!(report.angular_impulse.iter().any(|v| v.abs() > 1e-8));
    assert!((tensor_energy(&liquid, body) - initial).abs() < 1e-10);
    for (a, expected) in angular.into_iter().enumerate() {
        assert!((tensor_angular(&liquid, body)[a] - expected).abs() < 1e-12);
        assert!(
            (body.angular_momentum[a] - original.angular_momentum[a] - report.angular_impulse[a])
                .abs()
                < 1e-12
        );
    }
}
#[test]
fn tensor_viscous_relaxation_heats_with_conserved_energy_and_angular_momentum() {
    let mut p = particle(0.1);
    p.position = [0.1, 0.2, 0.3];
    p.velocity = [0.3, -0.2, 0.1];
    let mut liquid = Liquid::new(
        vec![p],
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 1.0,
            viscosity: 100.0,
        }],
        config(),
    )
    .unwrap();
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [-0.1, 0.2, 0.3],
            volume: 0.1,
        }])
        .unwrap();
    attach_thermal(&mut liquid, 10.0);
    liquid.set_viscous_heating(true).unwrap();
    let mut body = tensor_body();
    body.angular_momentum = [0.07, -0.08, 0.2];
    let initial = tensor_energy(&liquid, body);
    let angular = tensor_angular(&liquid, body);
    let report = liquid.step_with_tensor_boundary(0.02, &mut body).unwrap();
    assert!(report.boundary.viscous_heat > 0.0);
    assert!((tensor_energy(&liquid, body) - initial).abs() < 1e-10);
    for (a, value) in angular.into_iter().enumerate() {
        assert!((tensor_angular(&liquid, body)[a] - value).abs() < 1e-12);
    }
}
#[test]
fn invalid_tensor_and_non_unit_orientation_leave_every_state_unchanged() {
    let mut liquid = supported_fluid();
    let original = liquid.clone();
    let mut body = tensor_body();
    body.inertia[0][0] = -0.1;
    let before = body;
    assert_eq!(
        liquid.step_with_tensor_boundary(0.01, &mut body),
        Err(Error::InvalidBoundary)
    );
    assert_eq!(liquid, original);
    assert_eq!(body, before);
    body = tensor_body();
    body.inertia[0][1] = 1.0;
    let before = body;
    assert_eq!(
        liquid.step_with_tensor_boundary(0.01, &mut body),
        Err(Error::InvalidBoundary)
    );
    assert_eq!(liquid, original);
    assert_eq!(body, before);
    body = tensor_body();
    body.orientation = [2.0, 0.0, 0.0, 0.0];
    let before = body;
    assert_eq!(
        liquid.step_with_tensor_boundary(0.01, &mut body),
        Err(Error::InvalidBoundary)
    );
    assert_eq!(liquid, original);
    assert_eq!(body, before);
}

#[test]
fn tensor_torque_free_drift_matches_euler_acceleration() {
    let mut liquid = Liquid::new(vec![], vec![material()], config()).unwrap();
    let mut body = tensor_body();
    body.angular_momentum = [0.04, 0.08, 0.1];
    let before = body.angular_velocity().unwrap();
    let torque = cross3(before, body.angular_momentum).map(|v| -v);
    let mut acceleration_body = body;
    acceleration_body.angular_momentum = torque;
    let expected = acceleration_body.angular_velocity().unwrap();
    liquid.step_with_tensor_boundary(1e-6, &mut body).unwrap();
    let after = body.angular_velocity().unwrap();
    for a in 0..3 {
        assert!(((after[a] - before[a]) / 1e-6 - expected[a]).abs() < 1e-7);
    }
}
#[test]
fn tensor_orientation_converges_when_time_step_is_halved() {
    fn integrate(step: f64, count: usize) -> [f64; 4] {
        let mut liquid = Liquid::new(vec![], vec![material()], config()).unwrap();
        let mut body = tensor_body();
        body.angular_momentum = [0.04, 0.08, 0.1];
        for _ in 0..count {
            liquid.step_with_tensor_boundary(step, &mut body).unwrap();
        }
        body.orientation
    }
    let reference = integrate(0.00125, 320);
    let error = |q: [f64; 4]| {
        q.iter()
            .zip(reference)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    let coarse = error(integrate(0.04, 10));
    let fine = error(integrate(0.02, 20));
    assert!(coarse > 3.5 * fine);
    assert!(fine > 3.5 * error(integrate(0.01, 40)));
}
#[test]
fn tensor_substep_failure_restores_orientation_and_sample_geometry() {
    let mut liquid = Liquid::new(
        vec![],
        vec![material()],
        Config {
            max_substeps: 1,
            ..config()
        },
    )
    .unwrap();
    liquid
        .configure_boundaries(vec![BoundarySample {
            position: [0.3, 0.4, 0.2],
            volume: 0.1,
        }])
        .unwrap();
    let mut body = tensor_body();
    body.angular_momentum = [1.0, 2.0, 3.0];
    let before = liquid.clone();
    let previous = body;
    assert_eq!(
        liquid.step_with_tensor_boundary(0.1, &mut body),
        Err(Error::SubstepBudget)
    );
    assert_eq!(liquid, before);
    assert_eq!(body, previous);
}

#[test]
fn volume_wall_and_fluid_forces_differentiate_the_same_compression_energy() {
    use physics::liquid::Formulation;
    let particles = vec![
        Particle {
            position: [0.1, 0.2, -0.1],
            mass: 1.3,
            ..particle(0.0)
        },
        Particle {
            position: [0.3, -0.1, 0.2],
            mass: 0.7,
            ..particle(0.0)
        },
    ];
    let samples = vec![
        BoundarySample {
            position: [-0.2, 0.0, 0.0],
            volume: 0.9,
        },
        BoundarySample {
            position: [0.0, -0.3, -0.1],
            volume: 0.4,
        },
    ];
    for formulation in [Formulation::RestVolume, Formulation::RestVolumeWendland] {
        let make = |particles: Vec<Particle>, samples: Vec<BoundarySample>| {
            let mut fluid = Liquid::new(particles, vec![material()], config()).unwrap();
            fluid.set_formulation(formulation);
            fluid.configure_boundaries(samples).unwrap();
            fluid
        };
        let energy = |fluid: &Liquid| -> f64 {
            fluid
                .particles()
                .iter()
                .zip(fluid.diagnostics().unwrap().densities)
                .map(|(p, density)| {
                    let x = (density / material().rest_density - 1.0).max(0.0);
                    p.mass * material().sound_speed.powi(2) * (x.ln_1p() - x / (1.0 + x))
                })
                .sum()
        };
        let fluid = make(particles.clone(), samples.clone());
        let acceleration = fluid.diagnostics().unwrap().accelerations;
        let reactions = fluid.boundary_diagnostics().unwrap().reaction_forces;
        let epsilon = 1e-6;
        for (i, values) in acceleration.iter().enumerate() {
            for (axis, &value) in values.iter().enumerate() {
                let mut plus = particles.clone();
                let mut minus = particles.clone();
                plus[i].position[axis] += epsilon;
                minus[i].position[axis] -= epsilon;
                let gradient = (energy(&make(plus, samples.clone()))
                    - energy(&make(minus, samples.clone())))
                    / (2.0 * epsilon);
                assert!((particles[i].mass * value + gradient).abs() < 1e-8);
            }
        }
        for (i, values) in reactions.iter().enumerate() {
            for (axis, &value) in values.iter().enumerate() {
                let mut plus = samples.clone();
                let mut minus = samples.clone();
                plus[i].position[axis] += epsilon;
                minus[i].position[axis] -= epsilon;
                let gradient = (energy(&make(particles.clone(), plus))
                    - energy(&make(particles.clone(), minus)))
                    / (2.0 * epsilon);
                assert!((value + gradient).abs() < 1e-8);
            }
        }
    }
}

#[test]
fn extrapolated_static_pressure_preserves_reaction_and_thermal_work_balance() {
    for formulation in [
        physics::liquid::Formulation::MassDensity,
        physics::liquid::Formulation::RestVolume,
        physics::liquid::Formulation::RestVolumeWendland,
    ] {
        let mut liquid = thermal_wall(100.0);
        liquid.set_formulation(formulation);
        liquid.set_static_pressure_extrapolation(true).unwrap();
        let pressure = liquid.static_wall_pressures().unwrap();
        assert_eq!(pressure.len(), 1);
        assert!(pressure[0].pressure > 0.0);
        let reaction = liquid.boundary_diagnostics().unwrap().reaction_forces[0];
        let initial = thermal_energy(&liquid);
        liquid.step(0.001, None).unwrap();
        assert!((thermal_energy(&liquid) - initial).abs() < 1e-10);
        for (axis, force) in reaction.into_iter().enumerate() {
            assert!(
                (liquid.particles()[0].mass * liquid.particles()[0].velocity[axis] + force * 0.001)
                    .abs()
                    < 1e-12
            );
        }
        let before = liquid.clone();
        assert_eq!(
            liquid.configure_boundary_velocities(vec![[0.1, 0.0, 0.0]]),
            Err(Error::InvalidBoundary)
        );
        assert_eq!(liquid, before);
        let mut body = rotating_body().translation;
        let original_body = body;
        assert_eq!(
            liquid.step_with_boundary_body(0.001, &mut body),
            Err(Error::InvalidBoundary)
        );
        assert_eq!(liquid, before);
        assert_eq!(body, original_body);
        let mut moving = before.clone();
        moving.set_static_pressure_extrapolation(false).unwrap();
        moving
            .configure_boundary_velocities(vec![[0.1, 0.0, 0.0]])
            .unwrap();
        let original = moving.clone();
        assert_eq!(
            moving.set_static_pressure_extrapolation(true),
            Err(Error::InvalidBoundary)
        );
        assert_eq!(moving, original);
    }
}

#[test]
fn wall_extrapolation_is_invariant_to_coincident_particle_mass_splitting() {
    for formulation in [
        physics::liquid::Formulation::MassDensity,
        physics::liquid::Formulation::RestVolume,
        physics::liquid::Formulation::RestVolumeWendland,
    ] {
        let mut whole = fluid(config());
        whole.set_formulation(formulation);
        let mut half = particle(0.1);
        half.mass = 0.5;
        let mut split = Liquid::new(vec![half; 2], vec![material()], config()).unwrap();
        split.set_formulation(formulation);
        for liquid in [&mut whole, &mut split] {
            liquid
                .configure_boundaries(vec![BoundarySample {
                    position: [-0.1, 0.0, 0.0],
                    volume: 1.0,
                }])
                .unwrap();
            liquid.set_static_pressure_extrapolation(true).unwrap();
        }
        let a = whole.static_wall_pressures().unwrap();
        let b = split.static_wall_pressures().unwrap();
        assert!((a[0].pressure - b[0].pressure).abs() < 1e-12);
        assert!((a[0].density - b[0].density).abs() < 1e-12);
        let a = whole.boundary_diagnostics().unwrap().reaction_forces[0];
        let b = split.boundary_diagnostics().unwrap().reaction_forces[0];
        for (first, second) in a.into_iter().zip(b) {
            assert!((first - second).abs() < 1e-12);
        }
    }
}
