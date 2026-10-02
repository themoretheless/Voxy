// Exact equality intentionally checks unchanged masses and exactly zero states.
#![allow(clippy::float_cmp)]
use physics::liquid::{Config, Container, Error, Liquid, Material, Particle};

fn particle(x: f64, mass: f64, velocity: f64) -> Particle {
    Particle {
        position: [x, 0.0, 0.0],
        velocity: [velocity, 0.0, 0.0],
        mass,
        material: 0,
    }
}
fn config() -> Config {
    Config {
        smoothing_radius: 1.0,
        particle_radius: 0.05,
        gravity: [0.0; 3],
        ..Config::default()
    }
}
fn material(viscosity: f64) -> Material {
    Material {
        rest_density: 1000.0,
        sound_speed: 5.0,
        viscosity,
    }
}
fn momentum(liquid: &Liquid) -> [f64; 3] {
    std::array::from_fn(|axis| {
        liquid
            .particles()
            .iter()
            .map(|p| p.mass * p.velocity[axis])
            .sum()
    })
}
#[test]
fn unequal_mass_pressure_pair_preserves_momentum() {
    let m = Material {
        rest_density: 1.0,
        sound_speed: 5.0,
        viscosity: 0.0,
    };
    let mut liquid = Liquid::new(
        vec![particle(-0.1, 1.0, 0.0), particle(0.1, 2.0, 0.0)],
        vec![m],
        config(),
    )
    .unwrap();
    liquid.step(0.01, None).unwrap();
    assert!(liquid.particles()[0].velocity[0] < 0.0);
    assert!(liquid.particles()[1].velocity[0] > 0.0);
    assert!(momentum(&liquid)[0].abs() < 1e-12);
    assert_eq!(liquid.mass(), 3.0);
}
#[test]
fn viscosity_damps_relative_motion_without_losing_momentum() {
    let initial = vec![particle(-0.1, 1.0, 1.0), particle(0.1, 2.0, -0.5)];
    let mut inviscid = Liquid::new(initial.clone(), vec![material(0.0)], config()).unwrap();
    let mut viscous = Liquid::new(initial, vec![material(10.0)], config()).unwrap();
    inviscid.step(0.02, None).unwrap();
    viscous.step(0.02, None).unwrap();
    let relative = |l: &Liquid| (l.particles()[0].velocity[0] - l.particles()[1].velocity[0]).abs();
    assert!(relative(&viscous) < relative(&inviscid));
    assert!(momentum(&viscous)[0].abs() < 1e-12);
}
#[test]
fn gravity_and_walls_bound_particles_without_mass_loss() {
    let mut liquid = Liquid::new(
        vec![particle(0.0, 1.0, 0.0)],
        vec![Material::WATER],
        Config {
            gravity: [0.0, -9.81, 0.0],
            ..config()
        },
    )
    .unwrap();
    let container = Container {
        min: [-1.0; 3],
        max: [1.0; 3],
        restitution: 0.0,
        friction: 0.1,
    };
    for _ in 0..200 {
        liquid.step(0.01, Some(container)).unwrap();
    }
    assert!((liquid.particles()[0].position[1] + 0.95).abs() < 1e-10);
    assert_eq!(liquid.particles()[0].velocity[1], 0.0);
    assert_eq!(liquid.mass(), 1.0);
}
#[test]
fn failures_are_atomic_including_after_a_completed_substep() {
    let initial = vec![
        particle(0.0, 1.0, 0.0),
        particle(0.1, 1.0, 0.0),
        particle(0.2, 1.0, 0.0),
    ];
    let mut liquid = Liquid::new(
        initial.clone(),
        vec![material(0.0)],
        Config {
            max_pairs: 1,
            ..config()
        },
    )
    .unwrap();
    let before = liquid.clone();
    assert_eq!(liquid.step(0.01, None), Err(Error::PairBudget));
    assert_eq!(liquid, before);
    let mut liquid = Liquid::new(
        initial,
        vec![material(0.0)],
        Config {
            max_substeps: 1,
            ..config()
        },
    )
    .unwrap();
    let before = liquid.clone();
    assert_eq!(liquid.step(0.1, None), Err(Error::SubstepBudget));
    assert_eq!(liquid, before);
    assert_eq!(liquid.step(f64::NAN, None), Err(Error::InvalidTimeStep));
    assert_eq!(liquid, before);
}
#[test]
fn different_materials_keep_individual_mass_and_exchange_momentum() {
    let mut oil = particle(0.1, 0.8, -1.0);
    oil.material = 1;
    let mut liquid = Liquid::new(
        vec![particle(-0.1, 1.0, 0.8), oil],
        vec![Material::WATER, Material::OIL],
        config(),
    )
    .unwrap();
    for _ in 0..10 {
        liquid.step(0.001, None).unwrap();
    }
    assert!(momentum(&liquid)[0].abs() < 1e-12);
    assert_eq!(liquid.particles()[0].mass, 1.0);
    assert_eq!(liquid.particles()[1].mass, 0.8);
    assert_eq!(liquid.particles()[1].material, 1);
}

#[test]
fn dam_break_remains_finite_with_exact_mass_and_wall_bounds() {
    let mut particles = Vec::new();
    let spacing: f64 = 0.08;
    for x in 0..4 {
        for y in 0..4 {
            for z in 0..3 {
                particles.push(Particle {
                    position: [
                        -0.7 + f64::from(x) * spacing,
                        0.1 + f64::from(y) * spacing,
                        -0.08 + f64::from(z) * spacing,
                    ],
                    velocity: [0.0; 3],
                    mass: 1000.0 * spacing.powi(3),
                    material: 0,
                });
            }
        }
    }
    let mut liquid = Liquid::new(
        particles,
        vec![Material::WATER],
        Config {
            smoothing_radius: 0.16,
            particle_radius: 0.03,
            ..Config::default()
        },
    )
    .unwrap();
    let container = Container {
        min: [-1.0, 0.0, -0.3],
        max: [1.0, 1.0, 0.3],
        restitution: 0.0,
        friction: 0.02,
    };
    let mass = liquid.mass();
    for _ in 0..240 {
        liquid.step(1.0 / 120.0, Some(container)).unwrap();
        assert_eq!(liquid.mass(), mass);
        for p in liquid.particles() {
            for axis in 0..3 {
                assert!(p.velocity[axis].is_finite());
                assert!(p.position[axis] >= container.min[axis] + 0.03);
                assert!(p.position[axis] <= container.max[axis] - 0.03);
            }
        }
    }
    let width = liquid
        .particles()
        .iter()
        .map(|p| p.position[0])
        .fold(f64::NEG_INFINITY, f64::max)
        - liquid
            .particles()
            .iter()
            .map(|p| p.position[0])
            .fold(f64::INFINITY, f64::min);
    assert!(width > 0.4);
}

#[test]
fn isolated_free_fall_converges_as_time_step_is_refined() {
    let run = |dt: f64, steps| {
        let mut liquid = Liquid::new(
            vec![particle(0.0, 1.0, 0.0)],
            vec![material(0.0)],
            Config {
                gravity: [0.0, -9.81, 0.0],
                ..config()
            },
        )
        .unwrap();
        for _ in 0..steps {
            liquid.step(dt, None).unwrap();
        }
        liquid.particles()[0]
    };
    let coarse = run(0.01, 20);
    let fine = run(0.005, 40);
    let exact = -0.5 * 9.81 * 0.2 * 0.2;
    assert!((fine.position[1] - exact).abs() < (coarse.position[1] - exact).abs());
    assert!((fine.velocity[1] + 9.81 * 0.2).abs() < 1e-12);
}

#[test]
fn surface_tension_pulls_a_pair_together_and_preserves_unequal_mass_momentum() {
    let initial = vec![particle(-0.3, 1000.0, 0.0), particle(0.3, 2000.0, 0.0)];
    let mut reference = Liquid::new(
        initial.clone(),
        vec![Material {
            rest_density: 10000.0,
            ..material(0.0)
        }],
        config(),
    )
    .unwrap();
    let mut surface = Liquid::new(
        initial,
        vec![Material {
            rest_density: 10000.0,
            ..material(0.0)
        }],
        config(),
    )
    .unwrap();
    surface.set_surface_strength(0, 0.05).unwrap();
    reference.step(0.01, None).unwrap();
    surface.step(0.01, None).unwrap();
    assert!(surface.particles()[0].velocity[0] > 0.0);
    assert!(surface.particles()[1].velocity[0] < 0.0);
    let width = |l: &Liquid| l.particles()[1].position[0] - l.particles()[0].position[0];
    assert!(width(&surface) < width(&reference));
    assert!(momentum(&surface)[0].abs() < 1e-12);
    let before = surface.clone();
    assert_eq!(
        surface.set_surface_strength(0, f64::NAN),
        Err(Error::InvalidSurfaceStrength)
    );
    assert_eq!(surface, before);
}

#[test]
fn surface_forces_do_not_attract_different_materials() {
    let mut second = particle(0.3, 1.0, 0.0);
    second.material = 1;
    let mut liquid = Liquid::new(
        vec![particle(-0.3, 1.0, 0.0), second],
        vec![material(0.0), material(0.0)],
        config(),
    )
    .unwrap();
    liquid.set_surface_strength(0, 0.05).unwrap();
    liquid.set_surface_strength(1, 0.05).unwrap();
    liquid.step(0.01, None).unwrap();
    assert_eq!(liquid.particles()[0].velocity, [0.0; 3]);
    assert_eq!(liquid.particles()[1].velocity, [0.0; 3]);
}

#[test]
fn stretched_drop_rounds_and_stays_more_compact_than_control() {
    let spacing: f64 = 0.08;
    let mut particles = Vec::new();
    for x in -2..=2 {
        for y in -1..=1 {
            for z in -1..=1 {
                particles.push(Particle {
                    position: [
                        f64::from(x) * spacing,
                        f64::from(y) * spacing,
                        f64::from(z) * spacing,
                    ],
                    velocity: [0.0; 3],
                    mass: 1000.0 * spacing.powi(3),
                    material: 0,
                });
            }
        }
    }
    let mut drop = Liquid::new(
        particles,
        vec![Material {
            viscosity: 5.0,
            ..Material::WATER
        }],
        Config {
            smoothing_radius: 0.16,
            particle_radius: 0.03,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    let spread = |liquid: &Liquid| -> [f64; 3] {
        std::array::from_fn(|axis| {
            (liquid
                .particles()
                .iter()
                .map(|p| p.mass * p.position[axis].powi(2))
                .sum::<f64>()
                / liquid.mass())
            .sqrt()
        })
    };
    let mut control = drop.clone();
    let mass = drop.mass();
    let initial = spread(&drop);
    drop.set_surface_strength(0, 0.05).unwrap();
    for _ in 0..480 {
        drop.step(1.0 / 240.0, None).unwrap();
        control.step(1.0 / 240.0, None).unwrap();
        assert_eq!(drop.mass(), mass);
        for component in momentum(&drop) {
            assert!(component.abs() < 1e-10);
        }
    }
    let final_spread = spread(&drop);
    assert!(final_spread[0] / final_spread[1] < initial[0] / initial[1]);
    assert!(final_spread.iter().sum::<f64>() < spread(&control).iter().sum::<f64>());
}

#[test]
fn asymmetric_surface_forces_preserve_angular_momentum() {
    let particles = vec![
        Particle {
            position: [-0.12, 0.04, 0.03],
            velocity: [0.2, -0.3, 0.1],
            mass: 0.7,
            material: 0,
        },
        Particle {
            position: [0.1, -0.07, 0.08],
            velocity: [-0.1, 0.2, 0.3],
            mass: 1.3,
            material: 0,
        },
        Particle {
            position: [0.03, 0.15, -0.04],
            velocity: [0.3, 0.1, -0.2],
            mass: 0.9,
            material: 0,
        },
        Particle {
            position: [-0.03, -0.02, -0.12],
            velocity: [-0.2, -0.1, 0.2],
            mass: 1.1,
            material: 0,
        },
    ];
    let mut liquid = Liquid::new(particles, vec![material(0.5)], config()).unwrap();
    liquid.set_surface_strength(0, 0.01).unwrap();
    let angular = |fluid: &Liquid| -> [f64; 3] {
        std::array::from_fn(|a| {
            let b = (a + 1) % 3;
            let c = (a + 2) % 3;
            fluid
                .particles()
                .iter()
                .map(|p| p.mass * (p.position[b] * p.velocity[c] - p.position[c] * p.velocity[b]))
                .sum()
        })
    };
    let initial = angular(&liquid);
    let initial_linear = momentum(&liquid);
    let diagnostic = liquid.diagnostics().unwrap();
    let torque: [f64; 3] = std::array::from_fn(|a| {
        let b = (a + 1) % 3;
        let c = (a + 2) % 3;
        liquid
            .particles()
            .iter()
            .zip(&diagnostic.accelerations)
            .map(|(p, force)| p.mass * (p.position[b] * force[c] - p.position[c] * force[b]))
            .sum()
    });
    assert!(
        diagnostic
            .accelerations
            .iter()
            .flatten()
            .any(|a| a.abs() > 1e-6)
    );
    assert!(torque.iter().all(|a| a.abs() < 1e-12));
    for _ in 0..100 {
        liquid.step(0.001, None).unwrap();
    }
    for a in 0..3 {
        assert!((angular(&liquid)[a] - initial[a]).abs() < 1e-11);
        assert!((momentum(&liquid)[a] - initial_linear[a]).abs() < 1e-11);
    }
}

fn interface_fluid(distance: f64) -> Liquid {
    let mut second = particle(distance, 2.0, 0.0);
    second.material = 1;
    let mut fluid = Liquid::new(
        vec![particle(0.0, 1.0, 0.0), second],
        vec![material(0.0), material(0.0)],
        config(),
    )
    .unwrap();
    fluid.set_interface_penalty(0, 1, 0.1).unwrap();
    fluid
}
#[test]
fn interface_penalty_force_matches_potential_gradient_and_separates_materials() {
    let mut fluid = interface_fluid(0.2);
    let initial_energy = fluid.interface_energy().unwrap();
    let force = fluid.diagnostics().unwrap().accelerations[1][0] * 2.0;
    let epsilon = 1e-6;
    let derivative = (interface_fluid(0.2 + epsilon).interface_energy().unwrap()
        - interface_fluid(0.2 - epsilon).interface_energy().unwrap())
        / (2.0 * epsilon);
    assert!((force + derivative).abs() < 1e-9);
    fluid.step(0.02, None).unwrap();
    assert!(fluid.particles()[0].velocity[0] < 0.0 && fluid.particles()[1].velocity[0] > 0.0);
    assert!(fluid.interface_energy().unwrap() < initial_energy);
    assert!(momentum(&fluid).iter().all(|v| v.abs() < 1e-12));
    assert_eq!(fluid.particles()[0].material, 0);
    assert_eq!(fluid.particles()[1].material, 1);
    assert_eq!(fluid.mass(), 3.0);
}
#[test]
fn interface_penalty_is_symmetric_local_and_configuration_is_atomic() {
    let mut fluid = interface_fluid(0.2);
    let initial = fluid.clone();
    for invalid in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(fluid.set_interface_penalty(0, 1, invalid).is_err());
        assert_eq!(fluid, initial);
    }
    assert!(fluid.set_interface_penalty(0, 0, 1.0).is_err());
    assert_eq!(fluid, initial);
    assert!(fluid.set_interface_penalty(0, 2, 1.0).is_err());
    assert_eq!(fluid, initial);
    fluid.set_interface_penalty(1, 0, 0.1).unwrap();
    assert_eq!(fluid, initial);
    fluid.set_interface_penalty(1, 0, 0.0).unwrap();
    assert_eq!(fluid.interface_energy().unwrap(), 0.0);
    assert!(
        fluid
            .diagnostics()
            .unwrap()
            .accelerations
            .iter()
            .flatten()
            .all(|v| v.abs() < 1e-12)
    );
    assert_eq!(interface_fluid(2.0).interface_energy().unwrap(), 0.0);
}

#[test]
fn oblique_interfaces_preserve_linear_and_angular_momentum_over_time() {
    let mut first = particle(-0.1, 1.0, 0.2);
    first.position[1] = 0.1;
    let mut second = particle(0.1, 2.0, -0.1);
    second.position[2] = 0.1;
    second.material = 1;
    let mut third = particle(0.0, 0.7, 0.3);
    third.position[1] = -0.1;
    third.material = 2;
    let mut fluid =
        Liquid::new(vec![first, second, third], vec![material(0.0); 3], config()).unwrap();
    fluid.set_interface_penalty(0, 1, 0.1).unwrap();
    fluid.set_interface_penalty(1, 2, 0.2).unwrap();
    let angular = |fluid: &Liquid| -> [f64; 3] {
        std::array::from_fn(|a| {
            let b = (a + 1) % 3;
            let c = (a + 2) % 3;
            fluid
                .particles()
                .iter()
                .map(|p| p.mass * (p.position[b] * p.velocity[c] - p.position[c] * p.velocity[b]))
                .sum()
        })
    };
    let initial = angular(&fluid);
    let linear = momentum(&fluid);
    let mass = fluid.mass();
    for _ in 0..100 {
        fluid.step(0.001, None).unwrap();
    }
    for a in 0..3 {
        assert!((angular(&fluid)[a] - initial[a]).abs() < 1e-12);
        assert!((momentum(&fluid)[a] - linear[a]).abs() < 1e-12);
    }
    assert_eq!(fluid.mass(), mass);
    assert_eq!(
        fluid
            .particles()
            .iter()
            .map(|p| p.material)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
}

#[test]
fn volume_formulation_preserves_density_jump_for_equal_reference_volumes() {
    use physics::liquid::Formulation;
    let mut second = particle(0.2, 0.8, 0.0);
    second.material = 1;
    let mut fluid = Liquid::new(
        vec![particle(0.0, 1.0, 0.0), second],
        vec![
            material(0.0),
            Material {
                rest_density: 800.0,
                ..material(0.0)
            },
        ],
        config(),
    )
    .unwrap();
    let legacy = fluid.diagnostics().unwrap();
    fluid.set_formulation(Formulation::RestVolume);
    let volume = fluid.diagnostics().unwrap();
    assert!((volume.densities[0] / 1000.0 - volume.densities[1] / 800.0).abs() < 1e-14);
    assert!((legacy.densities[0] / 1000.0 - legacy.densities[1] / 800.0).abs() > 1e-4);
    assert_eq!(fluid.mass(), 1.8);
}
#[test]
fn volume_formulation_symmetric_pressure_preserves_momenta_with_unequal_masses() {
    use physics::liquid::Formulation;
    let mut second = particle(0.2, 0.8, 0.0);
    second.material = 1;
    second.position[1] = 0.1;
    let mut fluid = Liquid::new(
        vec![particle(0.0, 1.0, 0.0), second],
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
        config(),
    )
    .unwrap();
    fluid.set_formulation(Formulation::RestVolume);
    assert!(fluid.diagnostics().unwrap().accelerations[1][0] > 0.0);
    for _ in 0..20 {
        fluid.step(0.001, None).unwrap();
    }
    assert!(momentum(&fluid).iter().all(|v| v.abs() < 1e-12));
    let angular: f64 = fluid
        .particles()
        .iter()
        .map(|p| p.mass * (p.position[0] * p.velocity[1] - p.position[1] * p.velocity[0]))
        .sum();
    assert!(angular.abs() < 1e-12);
}

#[test]
fn volume_pressure_is_compression_energy_gradient_for_unequal_reference_volumes() {
    use physics::liquid::Formulation;
    let materials = vec![
        Material {
            rest_density: 0.8,
            sound_speed: 1.2,
            viscosity: 0.0,
        },
        Material {
            rest_density: 1.7,
            sound_speed: 0.9,
            viscosity: 0.0,
        },
        Material {
            rest_density: 1.1,
            sound_speed: 1.5,
            viscosity: 0.0,
        },
    ];
    let particles = vec![
        Particle {
            position: [-0.2, 0.1, 0.0],
            velocity: [0.0; 3],
            mass: 1.3,
            material: 0,
        },
        Particle {
            position: [0.1, -0.1, 0.2],
            velocity: [0.0; 3],
            mass: 0.7,
            material: 1,
        },
        Particle {
            position: [0.3, 0.2, -0.1],
            velocity: [0.0; 3],
            mass: 1.1,
            material: 2,
        },
    ];
    for formulation in [Formulation::RestVolume, Formulation::RestVolumeWendland] {
        let make = |particles: Vec<Particle>| {
            let mut fluid = Liquid::new(particles, materials.clone(), config()).unwrap();
            fluid.set_formulation(formulation);
            fluid
        };
        // Integrate p/rho^2 independently; no boundary or thermal property changes.
        let energy = |fluid: &Liquid| -> f64 {
            fluid
                .particles()
                .iter()
                .zip(fluid.diagnostics().unwrap().densities)
                .map(|(p, density)| {
                    let m = materials[p.material];
                    let x = (density / m.rest_density - 1.0).max(0.0);
                    p.mass * m.sound_speed.powi(2) * (x.ln_1p() - x / (1.0 + x))
                })
                .sum()
        };
        let fluid = make(particles.clone());
        let forces = fluid.diagnostics().unwrap().accelerations;
        assert!(energy(&fluid) > 0.0);
        for (i, acceleration) in forces.iter().enumerate() {
            for (axis, component) in acceleration.iter().enumerate() {
                let mut plus = particles.clone();
                let mut minus = particles.clone();
                let epsilon = 1e-6;
                plus[i].position[axis] += epsilon;
                minus[i].position[axis] -= epsilon;
                let gradient = (energy(&make(plus)) - energy(&make(minus))) / (2.0 * epsilon);
                let force = particles[i].mass * component;
                assert!(
                    (force + gradient).abs() < 1e-8 * force.abs().max(1.0),
                    "particle {i} axis {axis}: force {force}, gradient {gradient}"
                );
            }
        }
    }
}
