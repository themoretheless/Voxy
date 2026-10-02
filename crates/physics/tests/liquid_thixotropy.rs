use physics::liquid::{
    Config, Liquid, LiquidField, Material, Particle, ParticleInput, Thixotropy, TransportMaterial,
    WhippedCreamProfile,
};

fn model() -> Thixotropy {
    Thixotropy {
        recovery_rate: 2.0,
        breakdown: 3.0,
        broken_viscosity_ratio: 0.2,
        broken_yield_ratio: 0.0,
    }
}
fn cloud(rate: f64, spin: bool) -> Liquid {
    let mut particles = Vec::new();
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                let position = [f64::from(x) * 0.1, f64::from(y) * 0.1, f64::from(z) * 0.1];
                particles.push(Particle {
                    position,
                    velocity: if spin {
                        [-rate * position[1], rate * position[0], 0.0]
                    } else {
                        [rate * position[1], 0.0, 0.0]
                    },
                    mass: 0.5,
                    material: 0,
                });
            }
        }
    }
    Liquid::new(
        particles,
        vec![WhippedCreamProfile::DEMO.material],
        Config {
            smoothing_radius: 1.0,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap()
}
#[test]
fn constant_shear_structure_is_analytic_bounded_and_partition_independent() {
    let mut fluid = cloud(4.0, false);
    fluid
        .configure_thixotropy(vec![Some(model())], vec![0.9; 27])
        .unwrap();
    let before = fluid.particles().to_vec();
    let mut partitioned = fluid.clone();
    fluid.relax_structure(0.75).unwrap();
    for _ in 0..75 {
        partitioned.relax_structure(0.01).unwrap();
    }
    let equilibrium = 2.0 / 14.0;
    let expected = equilibrium + (0.9 - equilibrium) * (-14.0_f64 * 0.75).exp();
    for (value, split) in fluid
        .structure_fractions()
        .iter()
        .zip(partitioned.structure_fractions())
    {
        assert!((value - expected).abs() < 1e-12);
        assert!((value - split).abs() < 1e-12);
        assert!((0.0..=1.0).contains(value));
    }
    assert_eq!(fluid.particles(), before);
    let expected_viscosity = 5.0 * (0.2 + 0.8 * expected);
    for property in fluid.effective_materials().unwrap() {
        assert!((property.viscosity - expected_viscosity).abs() < 1e-12);
    }
}
#[test]
fn rest_and_rigid_spin_recover_without_regularization_induced_breakdown() {
    for spin in [false, true] {
        let mut fluid = cloud(if spin { 20.0 } else { 0.0 }, spin);
        fluid
            .configure_herschel_bulkley(&[Some(WhippedCreamProfile::DEMO.rheology)])
            .unwrap();
        fluid
            .configure_thixotropy(vec![Some(model())], vec![0.0; 27])
            .unwrap();
        fluid.relax_structure(0.5).unwrap();
        let expected = 1.0 - (-1.0_f64).exp();
        for value in fluid.structure_fractions() {
            assert!((value - expected).abs() < 1e-12);
        }
        if !spin {
            let expected_mu = 50.0 * (0.2 + 0.8 * expected) + 3000.0 * expected;
            for property in fluid.effective_materials().unwrap() {
                assert!((property.viscosity - expected_mu).abs() < 1e-9);
            }
        }
    }
}
#[test]
fn ordinary_and_symmetric_steps_evolve_structure_without_creating_heat_at_rest() {
    for symmetric in [false, true] {
        let mut fluid = Liquid::new(
            vec![Particle {
                position: [0.0; 3],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            }],
            vec![Material::WATER],
            Config {
                smoothing_radius: 1.0,
                gravity: [0.0; 3],
                ..Config::default()
            },
        )
        .unwrap();
        fluid
            .configure_transport(
                vec![LiquidField {
                    temperature: 280.0,
                    concentration: 0.0,
                }],
                vec![TransportMaterial::default()],
            )
            .unwrap();
        fluid.set_viscous_heating(true).unwrap();
        fluid
            .configure_thixotropy(vec![Some(model())], vec![0.2])
            .unwrap();
        let energy = fluid.transport_totals().unwrap().unwrap().0;
        if symmetric {
            fluid.step_symmetric_free(0.05).unwrap();
        } else {
            fluid.step(0.05, None).unwrap();
        }
        let expected = 1.0 - 0.8 * (-0.1_f64).exp();
        assert!((fluid.structure_fractions()[0] - expected).abs() < 1e-12);
        assert!((fluid.transport_totals().unwrap().unwrap().0 - energy).abs() < 1e-12);
        assert_eq!(fluid.particles()[0].velocity, [0.0; 3]);
    }
}
#[test]
fn structural_memory_follows_survivors_and_new_sources_start_recovered() {
    let mut fluid = cloud(0.0, false);
    let fractions: Vec<_> = (0..27).map(|i| f64::from(i) / 27.0).collect();
    fluid
        .configure_thixotropy(vec![Some(model())], fractions.clone())
        .unwrap();
    let input = ParticleInput {
        particle: fluid.particles()[0],
        field: None,
        phase_fraction: None,
    };
    fluid.exchange_particles(&[2, 0], &[input]).unwrap();
    let mut expected: Vec<_> = fractions
        .iter()
        .enumerate()
        .filter_map(|(i, v)| (i != 0 && i != 2).then_some(*v))
        .collect();
    expected.push(1.0);
    assert_eq!(fluid.structure_fractions(), expected);
}
#[test]
fn invalid_kinetics_and_failed_flow_preserve_structure_and_complete_state() {
    let mut fluid = cloud(1.0, false);
    fluid
        .configure_thixotropy(vec![Some(model())], vec![0.8; 27])
        .unwrap();
    let before = fluid.clone();
    assert!(
        fluid
            .configure_thixotropy(
                vec![Some(Thixotropy {
                    recovery_rate: f64::NAN,
                    ..model()
                })],
                vec![0.8; 27]
            )
            .is_err()
    );
    assert_eq!(fluid, before);
    assert!(fluid.relax_structure(f64::NAN).is_err());
    assert_eq!(fluid, before);
    assert!(fluid.step(0.2, None).is_err());
    assert_eq!(fluid, before);
    // Failure occurs after the current substep has computed its kinetic candidate.
    assert!(
        fluid
            .step_with_world(0.05, None, &FailingWorld, Default::default())
            .is_err()
    );
    assert_eq!(fluid, before);
}
struct FailingWorld;
impl physics::CollisionWorld for FailingWorld {
    type Obstacle = usize;
    type Error = &'static str;
    fn sweep_aabb(
        &self,
        _: physics::AnchoredAabb,
        _: [f64; 3],
        _: usize,
    ) -> Result<physics::SweepResult<usize>, Self::Error> {
        Err("deliberate backend failure")
    }
}

#[test]
fn evolving_shear_structure_changes_damping_and_preserves_heat_and_momentum() {
    let mut fluid = cloud(4.0, false);
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 280.0,
                    concentration: 0.0
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
    fluid.set_viscous_heating(true).unwrap();
    let mut recovered = fluid.clone();
    fluid
        .configure_thixotropy(vec![Some(model())], vec![0.0; 27])
        .unwrap();
    let kinetic = |state: &Liquid| {
        state
            .particles()
            .iter()
            .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>()
    };
    let initial = kinetic(&fluid);
    let before = fluid.particles().to_vec();
    let heat_before = fluid.transport_totals().unwrap().unwrap().0;
    for _ in 0..20 {
        fluid.relax_viscosity_midpoint(0.005).unwrap();
        recovered.relax_viscosity_midpoint(0.005).unwrap();
    }
    let loss = initial - kinetic(&fluid);
    let heat = fluid.transport_totals().unwrap().unwrap().0 - heat_before;
    assert!(loss > 0.0);
    assert!((heat - loss).abs() < 1e-7);
    assert!(kinetic(&fluid) > kinetic(&recovered));
    assert!(
        fluid
            .structure_fractions()
            .iter()
            .all(|v| *v > 0.0 && *v < 1.0)
    );
    for axis in 0..3 {
        let b = (axis + 1) % 3;
        let c = (axis + 2) % 3;
        let momentum: f64 = fluid
            .particles()
            .iter()
            .zip(&before)
            .map(|(p, q)| p.mass * (p.velocity[axis] - q.velocity[axis]))
            .sum();
        let angular: f64 = fluid
            .particles()
            .iter()
            .zip(&before)
            .map(|(p, q)| {
                p.mass
                    * (p.position[b] * (p.velocity[c] - q.velocity[c])
                        - p.position[c] * (p.velocity[b] - q.velocity[b]))
            })
            .sum();
        assert!(momentum.abs() < 1e-12 && angular.abs() < 1e-12);
    }
}

#[test]
fn coupled_structure_and_viscosity_refine_toward_the_same_solution() {
    let mut initial = cloud(4.0, false);
    initial
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 280.0,
                    concentration: 0.0
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
    initial.set_viscous_heating(true).unwrap();
    initial
        .configure_thixotropy(vec![Some(model())], vec![0.1; 27])
        .unwrap();
    let run = |steps: u32| {
        let mut state = initial.clone();
        for _ in 0..steps {
            state
                .relax_viscosity_midpoint(0.2 / f64::from(steps))
                .unwrap();
        }
        state
    };
    let reference = run(1024);
    let finer = run(2048);
    let error = |state: &Liquid, target: &Liquid| {
        let velocity = state
            .particles()
            .iter()
            .zip(target.particles())
            .flat_map(|(p, q)| {
                p.velocity
                    .into_iter()
                    .zip(q.velocity)
                    .map(|(v, w)| (v - w).abs())
            })
            .fold(0.0_f64, f64::max);
        let structure = state
            .structure_fractions()
            .iter()
            .zip(target.structure_fractions())
            .map(|(v, w)| (v - w).abs())
            .fold(0.0_f64, f64::max);
        (velocity, structure)
    };
    let reference_gap = error(&reference, &finer);
    let mut previous = None;
    for steps in [8, 16, 32] {
        let errors = error(&run(steps), &finer);
        eprintln!(
            "thixotropy refinement: steps={steps}, velocity_error={}, structure_error={}",
            errors.0, errors.1
        );
        assert!(errors.0 > 100.0 * reference_gap.0 && errors.1 > 100.0 * reference_gap.1);
        if let Some((velocity, structure)) = previous {
            assert!(velocity / errors.0 > 3.5 && structure / errors.1 > 3.5);
        }
        previous = Some(errors);
    }
}

#[test]
fn advecting_pressurized_structured_flow_refines_in_all_evolved_variables() {
    use physics::liquid::ViscousIntegrator;
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
    let particles = positions
        .into_iter()
        .map(|p| Particle {
            position: p,
            mass: 1.3,
            material: 0,
            velocity: [
                0.3 * p[0] + 0.4 * p[1] + 0.2 * p[2],
                0.4 * p[0] + 0.1 * p[1] - 0.1 * p[2],
                0.2 * p[0] - 0.1 * p[1] - 0.2 * p[2],
            ],
        })
        .collect();
    let mut initial = Liquid::new(
        particles,
        vec![Material {
            rest_density: 5.0,
            sound_speed: 2.0,
            viscosity: 0.1,
        }],
        Config {
            smoothing_radius: 1.0,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    initial
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.0
                };
                8
            ],
            vec![TransportMaterial {
                // Numerical capacity keeps reference energy small enough to resolve
                // tiny mechanical transfers without a large sensible-energy baseline.
                specific_heat: 1.0,
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    initial
        .configure_thixotropy(vec![Some(model())], vec![0.3; 8])
        .unwrap();
    initial.set_viscous_heating(true).unwrap();
    initial.set_pressure_work(true).unwrap();
    initial
        .set_viscous_integrator(ViscousIntegrator::Midpoint)
        .unwrap();
    let total = |state: &Liquid| {
        state.transport_totals().unwrap().unwrap().0
            + state
                .particles()
                .iter()
                .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
                .sum::<f64>()
    };
    let initial_energy = total(&initial);
    let run = |steps: u32, symmetric: bool| {
        let mut state = initial.clone();
        for _ in 0..steps {
            let dt = 0.02 / f64::from(steps);
            let stats = if symmetric {
                state.step_symmetric_free(dt).unwrap()
            } else {
                state.step(dt, None).unwrap()
            };
            assert_eq!(stats.substeps, 1);
        }
        assert!(
            (total(&state) - initial_energy).abs() < 1e-9,
            "steps={steps}, symmetric={symmetric}, energy_drift={}",
            total(&state) - initial_energy
        );
        state
    };
    let errors = |state: &Liquid, target: &Liquid| {
        let mut result = [0.0_f64; 3];
        for (p, q) in state.particles().iter().zip(target.particles()) {
            for axis in 0..3 {
                result[0] = result[0].max((p.position[axis] - q.position[axis]).abs());
                result[1] = result[1].max((p.velocity[axis] - q.velocity[axis]).abs());
            }
        }
        for (a, b) in state
            .structure_fractions()
            .iter()
            .zip(target.structure_fractions())
        {
            result[2] = result[2].max((a - b).abs());
        }
        result
    };
    for symmetric in [false, true] {
        let reference = run(512, symmetric);
        let finer = run(1024, symmetric);
        let reference_gap = errors(&reference, &finer);
        let mut previous: Option<[f64; 3]> = None;
        for steps in [8, 16, 32] {
            let error = errors(&run(steps, symmetric), &finer);
            eprintln!(
                "structured flow refinement: symmetric={symmetric}, steps={steps}, position={}, velocity={}, structure={}",
                error[0], error[1], error[2]
            );
            for axis in 0..3 {
                assert!(error[axis] > 10.0 * reference_gap[axis]);
                if let Some(previous) = previous {
                    let ratio = previous[axis] / error[axis];
                    assert!(
                        ratio > if symmetric { 3.5 } else { 1.7 },
                        "symmetric={symmetric}, variable={axis}, ratio={ratio}"
                    );
                }
            }
            previous = Some(error);
        }
    }
}
