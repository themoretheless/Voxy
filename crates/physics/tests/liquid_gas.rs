use physics::liquid::{
    Config, IdealGas, Liquid, LiquidField, Material, Particle, PhaseChange, TransportMaterial,
};
fn gas() -> IdealGas {
    IdealGas {
        gas_constant: 1.0,
        heat_capacity_ratio: 1.4,
    }
}
fn state(reference_density: f64, inward_speed: f64) -> Liquid {
    let mut fluid = Liquid::new(
        vec![
            Particle {
                position: [-0.2, 0.0, 0.0],
                velocity: [inward_speed, 0.0, 0.0],
                mass: 1.0,
                material: 0,
            },
            Particle {
                position: [0.2, 0.0, 0.0],
                velocity: [-inward_speed, 0.0, 0.0],
                mass: 1.0,
                material: 0,
            },
        ],
        vec![Material {
            rest_density: reference_density,
            sound_speed: 1.0,
            viscosity: 0.0,
        }],
        Config {
            smoothing_radius: 1.0,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.0
                };
                2
            ],
            vec![TransportMaterial {
                specific_heat: gas().specific_heat_cv().unwrap(),
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    fluid.configure_gas_equations(vec![Some(gas())]).unwrap();
    fluid
}
fn kinetic(fluid: &Liquid) -> f64 {
    fluid
        .particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum()
}
#[test]
fn ideal_gas_pressure_and_acoustic_speed_follow_temperature() {
    let air = IdealGas {
        gas_constant: 287.0,
        heat_capacity_ratio: 1.4,
    };
    assert!((air.pressure(1.2, 300.0).unwrap() - 103320.0).abs() < 1e-8);
    let mut fluid = state(1000.0, 0.0);
    let expected = (1.4_f64 * 300.0).sqrt();
    for property in fluid.effective_materials().unwrap() {
        assert!((property.sound_speed - expected).abs() < 1e-12);
    }
    fluid
        .exchange_reservoir_heat(0.01, 600.0, &[100.0; 2])
        .unwrap();
    for (property, field) in fluid
        .effective_materials()
        .unwrap()
        .iter()
        .zip(fluid.fields().unwrap())
    {
        assert!((property.sound_speed.powi(2) - 1.4 * field.temperature).abs() < 1e-10);
    }
    assert!(air.pressure(0.0, 300.0).is_err());
    assert!(air.pressure(1.0, 0.0).is_err());
}
#[test]
fn expansion_cools_and_compression_heats_with_total_energy_and_momentum_balance() {
    for (speed, symmetric) in [(0.0, false), (0.0, true), (1.0, false), (1.0, true)] {
        let mut fluid = state(1000.0, speed);
        let density_before = fluid.effective_materials().unwrap()[0].rest_density;
        let thermal = fluid.transport_totals().unwrap().unwrap().0;
        let initial = thermal + kinetic(&fluid);
        if symmetric {
            fluid.step_symmetric_free(1e-4).unwrap();
        } else {
            fluid.step(1e-4, None).unwrap();
        }
        let after = fluid.transport_totals().unwrap().unwrap().0;
        assert!((initial - after - kinetic(&fluid)).abs() < 1e-10);
        if speed == 0.0 {
            assert!(after < thermal);
            assert!(fluid.effective_materials().unwrap()[0].rest_density < density_before);
            assert!(fluid.particles()[0].velocity[0] < 0.0);
        } else {
            assert!(after > thermal);
        }
        assert!(
            (fluid.particles()[0].velocity[0] + fluid.particles()[1].velocity[0]).abs() < 1e-12
        );
    }
}
#[test]
fn gas_pressure_is_independent_of_liquid_reference_density() {
    let mut first = state(1.0, 0.0);
    let mut second = state(1000.0, 0.0);
    first.step_symmetric_free(0.001).unwrap();
    second.step_symmetric_free(0.001).unwrap();
    assert_eq!(first.particles(), second.particles());
    assert_eq!(first.fields(), second.fields());
}
#[test]
fn incompatible_heat_capacity_and_latent_eos_are_rejected_atomically() {
    let mut fluid = state(1.0, 0.0);
    let before = fluid.clone();
    assert!(
        fluid
            .configure_transport(
                vec![
                    LiquidField {
                        temperature: 300.0,
                        concentration: 0.0
                    };
                    2
                ],
                vec![TransportMaterial::default()]
            )
            .is_err()
    );
    assert_eq!(fluid, before);
    assert!(
        fluid
            .configure_gas_equations(vec![Some(IdealGas {
                heat_capacity_ratio: 1.0,
                ..gas()
            })])
            .is_err()
    );
    assert_eq!(fluid, before);
    assert!(
        fluid
            .configure_phase_change(
                vec![Some(PhaseChange {
                    temperature: 300.0,
                    latent_heat: 10.0,
                    high_phase: Material::WATER
                })],
                vec![0.0; 2]
            )
            .is_err()
    );
    assert_eq!(fluid, before);
    assert!(fluid.set_pressure_work(false).is_err());
    assert!(fluid.set_viscous_heating(false).is_err());
    assert_eq!(fluid, before);
}

fn reflecting_state(
    particles: Vec<Particle>,
    temperatures: Vec<f64>,
    bounds: physics::liquid::ReflectingBox,
) -> Liquid {
    reflecting_state_with_gravity(particles, temperatures, bounds, [0.0; 3])
}
fn reflecting_state_with_gravity(
    particles: Vec<Particle>,
    temperatures: Vec<f64>,
    bounds: physics::liquid::ReflectingBox,
    gravity: [f64; 3],
) -> Liquid {
    let mut fluid = Liquid::new(
        particles,
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 1.0,
            viscosity: 0.0,
        }],
        Config {
            smoothing_radius: 1.0,
            gravity,
            ..Config::default()
        },
    )
    .unwrap();
    fluid
        .configure_transport(
            temperatures
                .into_iter()
                .map(|temperature| LiquidField {
                    temperature,
                    concentration: 0.0,
                })
                .collect(),
            vec![TransportMaterial {
                specific_heat: gas().specific_heat_cv().unwrap(),
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    fluid.configure_gas_equations(vec![Some(gas())]).unwrap();
    fluid.set_reflecting_box(Some(bounds)).unwrap();
    fluid
}
#[test]
fn gas_corner_and_face_loads_match_isentropic_internal_energy_gradients() {
    use physics::liquid::ReflectingBox;
    let bounds = ReflectingBox {
        min: [0.0; 3],
        max: [3.0; 3],
    };
    let particles = vec![
        Particle {
            position: [0.12, 0.18, 0.2],
            velocity: [0.0; 3],
            mass: 1.3,
            material: 0,
        },
        Particle {
            position: [0.27, 0.31, 0.08],
            velocity: [0.0; 3],
            mass: 0.9,
            material: 0,
        },
        Particle {
            position: [0.65, 0.23, 0.33],
            velocity: [0.0; 3],
            mass: 1.1,
            material: 0,
        },
    ];
    let initial = reflecting_state(particles.clone(), vec![300.0; 3], bounds);
    let density = initial.diagnostics().unwrap().densities;
    let energy = |particles: Vec<Particle>, bounds: ReflectingBox| {
        let mut fluid = reflecting_state(particles, vec![300.0; 3], bounds);
        let densities = fluid.diagnostics().unwrap().densities;
        let fields = densities
            .iter()
            .zip(&density)
            .map(|(rho, reference)| LiquidField {
                temperature: 300.0 * (rho / reference).powf(gas().heat_capacity_ratio - 1.0),
                concentration: 0.0,
            })
            .collect();
        fluid
            .configure_transport(
                fields,
                vec![TransportMaterial {
                    specific_heat: gas().specific_heat_cv().unwrap(),
                    conductivity: 0.0,
                    ..TransportMaterial::default()
                }],
            )
            .unwrap();
        fluid.transport_totals().unwrap().unwrap().0
    };
    let diagnostics = initial.diagnostics().unwrap();
    let reflections = initial.reflecting_diagnostics().unwrap();
    let epsilon = 1e-6;
    for i in 0..particles.len() {
        for axis in 0..3 {
            let mut plus = particles.clone();
            let mut minus = particles.clone();
            plus[i].position[axis] += epsilon;
            minus[i].position[axis] -= epsilon;
            let gradient = (energy(plus, bounds) - energy(minus, bounds)) / (2.0 * epsilon);
            let force = particles[i].mass * diagnostics.accelerations[i][axis];
            assert!(
                (gradient + force).abs() < 1e-6 * force.abs().max(1.0),
                "particle={i},axis={axis},force={force},gradient={gradient}"
            );
        }
    }
    for face in 0..6 {
        let axis = face / 2;
        let mut plus = bounds;
        let mut minus = bounds;
        if face % 2 == 0 {
            plus.min[axis] += epsilon;
            minus.min[axis] -= epsilon;
        } else {
            plus.max[axis] += epsilon;
            minus.max[axis] -= epsilon;
        }
        let gradient =
            (energy(particles.clone(), plus) - energy(particles.clone(), minus)) / (2.0 * epsilon);
        let force = reflections.reaction_forces[face][axis];
        assert!(
            (gradient + force).abs() < 1e-6 * force.abs().max(1.0),
            "face={face},force={force},gradient={gradient}"
        );
    }
}
#[test]
fn gas_wall_work_preserves_total_energy_and_fixture_force_balance() {
    use physics::liquid::ReflectingBox;
    for symmetric in [false, true] {
        let particles = vec![
            Particle {
                position: [0.2, 0.25, 0.3],
                velocity: [-0.1, 0.05, 0.0],
                mass: 1.3,
                material: 0,
            },
            Particle {
                position: [0.4, 0.28, 0.22],
                velocity: [0.1, -0.02, 0.01],
                mass: 0.9,
                material: 0,
            },
        ];
        let mut fluid = reflecting_state(
            particles,
            vec![300.0; 2],
            ReflectingBox {
                min: [0.0; 3],
                max: [3.0; 3],
            },
        );
        let forces = fluid.diagnostics().unwrap();
        let reactions = fluid.reflecting_diagnostics().unwrap();
        for axis in 0..3 {
            let momentum_rate: f64 = fluid
                .particles()
                .iter()
                .zip(&forces.accelerations)
                .map(|(p, a)| p.mass * a[axis])
                .sum();
            let wall_rate: f64 = reactions
                .reaction_forces
                .iter()
                .map(|force| force[axis])
                .sum();
            assert!((momentum_rate + wall_rate).abs() < 1e-10);
        }
        let total = kinetic(&fluid) + fluid.transport_totals().unwrap().unwrap().0;
        if symmetric {
            fluid.step_symmetric_free(1e-4).unwrap();
        } else {
            fluid.step(1e-4, None).unwrap();
        }
        assert!(
            (kinetic(&fluid) + fluid.transport_totals().unwrap().unwrap().0 - total).abs() < 1e-10
        );
        let diagnostics = fluid.diagnostics().unwrap();
        for (material, rho) in fluid
            .effective_materials()
            .unwrap()
            .iter()
            .zip(diagnostics.densities)
        {
            assert!((material.rest_density - rho).abs() < 1e-12);
        }
    }
}

#[test]
fn gas_wall_adiabatic_entropy_error_decreases_under_time_refinement() {
    use physics::liquid::ReflectingBox;
    let initial = reflecting_state(
        vec![Particle {
            position: [0.2, 1.5, 1.5],
            velocity: [-1.0, 0.0, 0.0],
            mass: 1.0,
            material: 0,
        }],
        vec![300.0],
        ReflectingBox {
            min: [0.0; 3],
            max: [3.0; 3],
        },
    );
    let density = initial.diagnostics().unwrap().densities[0];
    let total = kinetic(&initial) + initial.transport_totals().unwrap().unwrap().0;
    for symmetric in [false, true] {
        let mut previous: Option<f64> = None;
        for steps in [8, 16, 32] {
            let mut fluid = initial.clone();
            for _ in 0..steps {
                if symmetric {
                    fluid.step_symmetric_free(0.01 / f64::from(steps)).unwrap();
                } else {
                    fluid.step(0.01 / f64::from(steps), None).unwrap();
                }
            }
            let rho = fluid.diagnostics().unwrap().densities[0];
            let expected = 300.0 * (rho / density).powf(gas().heat_capacity_ratio - 1.0);
            let error = (fluid.fields().unwrap()[0].temperature - expected).abs();
            let drift = kinetic(&fluid) + fluid.transport_totals().unwrap().unwrap().0 - total;
            eprintln!(
                "gas wall adiabat: symmetric={symmetric}, steps={steps}, temperature_error={error}, energy_drift={drift}"
            );
            assert!(error > 1e-10 && drift.abs() < 1e-10);
            if let Some(previous) = previous {
                assert!(previous / error > if symmetric { 3.5 } else { 1.7 });
            }
            previous = Some(error);
        }
    }
}

#[test]
fn multi_particle_gas_corner_flow_has_second_order_temperature_and_motion() {
    use physics::liquid::ReflectingBox;
    let particles = vec![
        Particle {
            position: [0.2, 0.25, 0.3],
            velocity: [-0.1, 0.05, 0.0],
            mass: 1.3,
            material: 0,
        },
        Particle {
            position: [0.4, 0.28, 0.22],
            velocity: [0.1, -0.02, 0.01],
            mass: 0.9,
            material: 0,
        },
        Particle {
            position: [0.3, 0.55, 0.4],
            velocity: [-0.03, 0.1, -0.02],
            mass: 1.1,
            material: 0,
        },
    ];
    let initial = reflecting_state(
        particles,
        vec![280.0, 300.0, 320.0],
        ReflectingBox {
            min: [0.0; 3],
            max: [3.0; 3],
        },
    );
    let total = kinetic(&initial) + initial.transport_totals().unwrap().unwrap().0;
    let run = |steps: u32| {
        let mut fluid = initial.clone();
        for _ in 0..steps {
            assert_eq!(
                fluid
                    .step_symmetric_free(0.001 / f64::from(steps))
                    .unwrap()
                    .substeps,
                1
            );
        }
        assert!(
            (kinetic(&fluid) + fluid.transport_totals().unwrap().unwrap().0 - total).abs() < 1e-8
        );
        fluid
    };
    let errors = |fluid: &Liquid, target: &Liquid| {
        let mut result = [0.0_f64; 3];
        for (p, q) in fluid.particles().iter().zip(target.particles()) {
            for axis in 0..3 {
                result[0] = result[0].max((p.position[axis] - q.position[axis]).abs());
                result[1] = result[1].max((p.velocity[axis] - q.velocity[axis]).abs());
            }
        }
        for (f, g) in fluid.fields().unwrap().iter().zip(target.fields().unwrap()) {
            result[2] = result[2].max((f.temperature - g.temperature).abs());
        }
        result
    };
    let reference = run(512);
    let finer = run(1024);
    let uncertainty = errors(&reference, &finer);
    let mut previous: Option<[f64; 3]> = None;
    for steps in [8, 16, 32] {
        let error = errors(&run(steps), &finer);
        eprintln!(
            "gas corner refinement: steps={steps}, position_error={}, velocity_error={}, temperature_error={}",
            error[0], error[1], error[2]
        );
        for variable in 0..3 {
            assert!(error[variable] > 100.0 * uncertainty[variable]);
            if let Some(previous) = previous {
                assert!(previous[variable] / error[variable] > 3.5);
            }
        }
        previous = Some(error);
    }
}

#[test]
fn gas_gravity_and_wall_pressure_refine_adiabatic_and_potential_energy_balance() {
    use physics::liquid::ReflectingBox;
    let gravity = [-100.0, 0.0, 0.0];
    let initial = reflecting_state_with_gravity(
        vec![Particle {
            position: [0.2, 1.5, 1.5],
            velocity: [-1.0, 0.0, 0.0],
            mass: 1.0,
            material: 0,
        }],
        vec![300.0],
        ReflectingBox {
            min: [0.0; 3],
            max: [3.0; 3],
        },
        gravity,
    );
    let initial_density = initial.diagnostics().unwrap().densities[0];
    let total = |state: &Liquid| {
        kinetic(state) + state.transport_totals().unwrap().unwrap().0
            - state
                .particles()
                .iter()
                .map(|p| {
                    p.mass
                        * p.position
                            .iter()
                            .zip(gravity)
                            .map(|(x, g)| x * g)
                            .sum::<f64>()
                })
                .sum::<f64>()
    };
    let initial_energy = total(&initial);
    let mut previous: Option<[f64; 2]> = None;
    for steps in [8, 16, 32] {
        let mut fluid = initial.clone();
        for _ in 0..steps {
            assert_eq!(
                fluid
                    .step_symmetric_free(0.01 / f64::from(steps))
                    .unwrap()
                    .substeps,
                1
            );
        }
        let rho = fluid.diagnostics().unwrap().densities[0];
        let expected = 300.0 * (rho / initial_density).powf(gas().heat_capacity_ratio - 1.0);
        let error = [
            (fluid.fields().unwrap()[0].temperature - expected).abs(),
            (total(&fluid) - initial_energy).abs(),
        ];
        eprintln!(
            "gas gravity refinement: steps={steps}, temperature_error={}, total_energy_error={}",
            error[0], error[1]
        );
        for variable in 0..2 {
            assert!(error[variable] > 1e-10);
            if let Some(previous) = previous {
                assert!(previous[variable] / error[variable] > 3.5);
            }
        }
        previous = Some(error);
    }
}

#[test]
fn isolated_gas_particle_falls_ballistically_without_gravity_heating() {
    use physics::liquid::ReflectingBox;
    let gravity = [2.0, -9.81, 3.0];
    let initial_velocity = [0.1, -0.2, 0.05];
    let initial_position = [1.5; 3];
    let mut fluid = reflecting_state_with_gravity(
        vec![Particle {
            position: initial_position,
            velocity: initial_velocity,
            mass: 1.0,
            material: 0,
        }],
        vec![300.0],
        ReflectingBox {
            min: [0.0; 3],
            max: [3.0; 3],
        },
        gravity,
    );
    let thermal = fluid.transport_totals().unwrap().unwrap().0;
    let dt = 0.05;
    let stats = fluid.step_symmetric_free(dt).unwrap();
    assert!(stats.substeps > 1);
    for axis in 0..3 {
        assert!(
            (fluid.particles()[0].velocity[axis] - (initial_velocity[axis] + gravity[axis] * dt))
                .abs()
                < 1e-12
        );
        assert!(
            (fluid.particles()[0].position[axis]
                - (initial_position[axis]
                    + initial_velocity[axis] * dt
                    + 0.5 * gravity[axis] * dt * dt))
                .abs()
                < 1e-12
        );
    }
    assert!((fluid.fields().unwrap()[0].temperature - 300.0).abs() < 1e-10);
    assert!((fluid.transport_totals().unwrap().unwrap().0 - thermal).abs() < 1e-10);
}
