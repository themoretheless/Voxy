#![allow(clippy::float_cmp)]
use physics::liquid::{
    Config, Error, Formulation, Liquid, LiquidField, Material, Particle, ReflectingBox,
    TransportMaterial,
};
fn bounds() -> ReflectingBox {
    ReflectingBox {
        min: [0.0; 3],
        max: [3.0; 3],
    }
}
fn config() -> Config {
    Config {
        smoothing_radius: 1.0,
        gravity: [0.0; 3],
        ..Config::default()
    }
}
fn materials() -> Vec<Material> {
    vec![
        Material {
            rest_density: 0.8,
            sound_speed: 1.2,
            viscosity: 0.0,
        },
        Material {
            rest_density: 1.1,
            sound_speed: 0.9,
            viscosity: 0.0,
        },
    ]
}
fn particles() -> Vec<Particle> {
    vec![
        Particle {
            position: [0.12, 0.18, 0.2],
            velocity: [0.0; 3],
            mass: 1.3,
            material: 0,
        },
        Particle {
            position: [0.27, 0.31, 0.08],
            velocity: [0.0; 3],
            mass: 0.7,
            material: 1,
        },
        Particle {
            position: [2.85, 0.2, 2.9],
            velocity: [0.0; 3],
            mass: 1.1,
            material: 1,
        },
    ]
}
fn make(p: Vec<Particle>, b: ReflectingBox, formulation: Formulation) -> Liquid {
    let mut liquid = Liquid::new(p, materials(), config()).unwrap();
    liquid.set_formulation(formulation);
    liquid.set_reflecting_box(Some(b)).unwrap();
    liquid
}
fn compression(liquid: &Liquid) -> f64 {
    liquid
        .particles()
        .iter()
        .zip(liquid.diagnostics().unwrap().densities)
        .map(|(p, rho)| {
            let m = materials()[p.material];
            let x = (rho / m.rest_density - 1.0).max(0.0);
            p.mass * m.sound_speed.powi(2) * (x.ln_1p() - x / (1.0 + x))
        })
        .sum()
}
fn kinetic(liquid: &Liquid) -> f64 {
    liquid
        .particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum()
}
#[test]
fn reflected_corner_forces_and_face_reactions_are_energy_gradients() {
    for formulation in [Formulation::RestVolume, Formulation::RestVolumeWendland] {
        let particles = particles();
        let fluid = make(particles.clone(), bounds(), formulation);
        let accelerations = fluid.diagnostics().unwrap().accelerations;
        let walls = fluid.reflecting_diagnostics().unwrap();
        let epsilon = 1e-6;
        for (i, values) in accelerations.iter().enumerate() {
            for (axis, &a) in values.iter().enumerate() {
                let mut plus = particles.clone();
                let mut minus = particles.clone();
                plus[i].position[axis] += epsilon;
                minus[i].position[axis] -= epsilon;
                let gradient = (compression(&make(plus, bounds(), formulation))
                    - compression(&make(minus, bounds(), formulation)))
                    / (2.0 * epsilon);
                let force = particles[i].mass * a;
                assert!((force + gradient).abs() < 1e-7 * force.abs().max(1.0));
            }
        }
        for face in 0..6 {
            let axis = face / 2;
            let mut plus = bounds();
            let mut minus = bounds();
            if face % 2 == 0 {
                plus.min[axis] += epsilon;
                minus.min[axis] -= epsilon;
            } else {
                plus.max[axis] += epsilon;
                minus.max[axis] -= epsilon;
            }
            let gradient = (compression(&make(particles.clone(), plus, formulation))
                - compression(&make(particles.clone(), minus, formulation)))
                / (2.0 * epsilon);
            let force = walls.reaction_forces[face][axis];
            assert!((force + gradient).abs() < 1e-7 * force.abs().max(1.0));
        }
    }
}
#[test]
fn pressure_impulses_and_torque_balance_the_stationary_fixture() {
    let mut liquid = make(particles(), bounds(), Formulation::RestVolumeWendland);
    let walls = liquid.reflecting_diagnostics().unwrap();
    let dt = 1e-5;
    assert_eq!(liquid.step(dt, None).unwrap().substeps, 1);
    for axis in 0..3 {
        let momentum: f64 = liquid
            .particles()
            .iter()
            .map(|p| p.mass * p.velocity[axis])
            .sum();
        let reaction: f64 = walls.reaction_forces.iter().map(|f| f[axis]).sum();
        assert!((momentum + dt * reaction).abs() < 1e-12);
        let b = (axis + 1) % 3;
        let c = (axis + 2) % 3;
        let angular: f64 = liquid
            .particles()
            .iter()
            .map(|p| p.mass * (p.position[b] * p.velocity[c] - p.position[c] * p.velocity[b]))
            .sum();
        assert!((angular + dt * walls.reaction_torque_about_origin[axis]).abs() < 1e-12);
    }
}
#[test]
fn mechanical_energy_error_decreases_with_time_step() {
    let mut errors = Vec::new();
    for steps in [25, 50, 100] {
        let mut fluid = make(particles(), bounds(), Formulation::RestVolumeWendland);
        let initial = compression(&fluid) + kinetic(&fluid);
        for _ in 0..steps {
            fluid.step(0.05 / f64::from(steps), None).unwrap();
        }
        errors.push(((compression(&fluid) + kinetic(&fluid)) / initial - 1.0).abs());
    }
    eprintln!("reflecting energy errors at dt .002/.001/.0005: {errors:?}");
    assert!(errors[0] < 0.01);
    assert!(errors[1] < 0.65 * errors[0]);
    assert!(errors[2] < 0.65 * errors[1]);
}
#[test]
fn reflected_pressure_work_matches_explicit_kicks_and_balances_heat() {
    let mut fluid = make(particles(), bounds(), Formulation::RestVolumeWendland);
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 100.0,
                    concentration: 0.0
                };
                3
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
    let total = |fluid: &Liquid| fluid.transport_totals().unwrap().unwrap().0 + kinetic(fluid);
    let initial = total(&fluid);
    let mut explicit = fluid.clone();
    fluid.set_pressure_work(true).unwrap();
    explicit.step(0.001, None).unwrap();
    fluid.step(0.001, None).unwrap();
    for (p, q) in fluid.particles().iter().zip(explicit.particles()) {
        for (v, w) in p.velocity.iter().zip(q.velocity) {
            assert!((v - w).abs() < 1e-12);
        }
    }
    for _ in 0..100 {
        fluid.step(0.001, None).unwrap();
    }
    assert!((total(&fluid) - initial).abs() < 1e-10);
}
#[test]
fn reflected_geometry_and_budget_failures_are_atomic() {
    let mut fluid = make(particles(), bounds(), Formulation::RestVolumeWendland);
    let before = fluid.clone();
    assert_eq!(
        fluid.set_reflecting_box(Some(ReflectingBox {
            min: [0.0; 3],
            max: [0.5; 3]
        })),
        Err(Error::InvalidBoundary)
    );
    assert_eq!(fluid, before);
    assert_eq!(
        fluid.set_static_pressure_extrapolation(true),
        Err(Error::InvalidBoundary)
    );
    assert_eq!(fluid, before);
    assert_eq!(
        fluid.configure_boundaries(vec![physics::liquid::BoundarySample {
            position: [0.0; 3],
            volume: 1.0
        }]),
        Err(Error::InvalidBoundary)
    );
    assert_eq!(fluid, before);
    let mut outside = particles()[0];
    outside.position[0] = -0.1;
    assert_eq!(
        fluid.exchange_particles(
            &[0],
            &[physics::liquid::ParticleInput {
                particle: outside,
                field: None,
                phase_fraction: None
            }]
        ),
        Err(Error::InvalidBoundary)
    );
    assert_eq!(fluid, before);
    for (pairs, checks, error) in [
        (1, 4000, Error::PairBudget),
        (1000, 7, Error::NeighborBudget),
    ] {
        let mut limited = Liquid::new(
            particles(),
            materials(),
            Config {
                max_pairs: pairs,
                max_neighbor_checks: checks,
                ..config()
            },
        )
        .unwrap();
        limited.set_formulation(Formulation::RestVolumeWendland);
        limited.set_reflecting_box(Some(bounds())).unwrap();
        let original = limited.clone();
        assert_eq!(limited.step(0.001, None), Err(error));
        assert_eq!(limited, original);
    }
}

#[test]
fn crossing_a_reflecting_domain_rolls_back_motion_and_thermal_fields() {
    let mut initial = particles();
    initial[0].velocity[0] = -100.0;
    let mut fluid = make(initial, bounds(), Formulation::RestVolumeWendland);
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 100.0,
                    concentration: 0.0
                };
                3
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
    fluid.set_pressure_work(true).unwrap();
    let before = fluid.clone();
    assert_eq!(fluid.step(0.01, None), Err(Error::InvalidBoundary));
    assert_eq!(fluid, before);
}

#[test]
fn central_wall_self_image_damps_normal_velocity_and_deposits_exact_heat() {
    let p = Particle {
        position: [0.2, 1.5, 1.5],
        velocity: [0.2, 0.7, -0.3],
        mass: 0.1,
        material: 0,
    };
    let mut fluid = Liquid::new(
        vec![p],
        vec![Material {
            rest_density: 1000.0,
            sound_speed: 1.0,
            viscosity: 10.0,
        }],
        config(),
    )
    .unwrap();
    fluid.set_formulation(Formulation::RestVolumeWendland);
    fluid.set_reflecting_box(Some(bounds())).unwrap();
    fluid.set_reflecting_no_slip(true).unwrap();
    let rho = fluid.diagnostics().unwrap().densities[0];
    let c = 2.5 * p.mass * p.mass * 10.0 * 45.0 / std::f64::consts::PI * (1.0 - 0.4) / (rho * rho);
    let dt = 1e-7;
    let explicit = fluid.diagnostics().unwrap().accelerations[0];
    assert!((explicit[0] + 4.0 * c * p.velocity[0] / p.mass).abs() < 1e-9);
    assert_eq!(explicit[1], 0.0);
    assert_eq!(explicit[2], 0.0);
    fluid
        .configure_transport(
            vec![LiquidField {
                temperature: 300.0,
                concentration: 0.0,
            }],
            vec![TransportMaterial {
                specific_heat: 2.0,
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    fluid.set_viscous_heating(true).unwrap();
    let before = kinetic(&fluid) + 0.2 * 300.0;
    let stats = fluid.step(dt, None).unwrap();
    assert_eq!(stats.substeps, 1);
    assert!(
        (fluid.particles()[0].velocity[0] - p.velocity[0] * (-4.0 * c / p.mass * dt).exp()).abs()
            < 1e-12
    );
    assert_eq!(fluid.particles()[0].velocity[1], p.velocity[1]);
    assert_eq!(fluid.particles()[0].velocity[2], p.velocity[2]);
    assert!(
        (kinetic(&fluid) + 0.2 * fluid.fields().unwrap()[0].temperature - before).abs() < 1e-12
    );
}

#[test]
fn corner_image_relaxation_conserves_heat_and_cannot_increase_kinetic_energy() {
    let mut ps = particles();
    for (speed, p) in [0.1, 0.2, 0.3].into_iter().zip(&mut ps) {
        p.velocity = [speed, -0.08, 0.05];
    }
    let mut ms = materials();
    for m in &mut ms {
        m.rest_density = 1e6;
        m.viscosity = 0.01;
    }
    let mut fluid = Liquid::new(ps, ms, config()).unwrap();
    fluid.set_formulation(Formulation::RestVolumeWendland);
    fluid.set_reflecting_box(Some(bounds())).unwrap();
    fluid.set_reflecting_no_slip(true).unwrap();
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.0
                };
                3
            ],
            vec![
                TransportMaterial {
                    specific_heat: 2.0,
                    conductivity: 0.0,
                    ..TransportMaterial::default()
                };
                2
            ],
        )
        .unwrap();
    fluid.set_viscous_heating(true).unwrap();
    let total = |f: &Liquid| {
        kinetic(f)
            + f.particles()
                .iter()
                .zip(f.fields().unwrap())
                .map(|(p, t)| p.mass * 2.0 * t.temperature)
                .sum::<f64>()
    };
    let initial = total(&fluid);
    for _ in 0..20 {
        let before = kinetic(&fluid);
        fluid.step(0.0001, None).unwrap();
        assert!(kinetic(&fluid) <= before + 1e-13);
        assert!((total(&fluid) - initial).abs() < 1e-10);
    }
}

#[test]
fn viscous_fixture_loads_match_velocity_gradient_and_particle_impulses() {
    let mut ps = particles();
    for (speed, p) in [0.1, 0.2, 0.3].into_iter().zip(&mut ps) {
        p.velocity = [speed, -0.08, 0.05];
    }
    let mut ms = materials();
    for m in &mut ms {
        m.rest_density = 1e6;
        m.viscosity = 0.01;
    }
    let make_drag = |particles: Vec<Particle>| {
        let mut f = Liquid::new(particles, ms.clone(), config()).unwrap();
        f.set_formulation(Formulation::RestVolumeWendland);
        f.set_reflecting_box(Some(bounds())).unwrap();
        f.set_reflecting_no_slip(true).unwrap();
        f
    };
    let mut fluid = make_drag(ps.clone());
    let loads = fluid.reflecting_viscous_diagnostics().unwrap();
    assert!(loads.dissipated_power > 0.0);
    let mut slip = fluid.clone();
    slip.set_reflecting_no_slip(false).unwrap();
    let off = slip.reflecting_viscous_diagnostics().unwrap();
    assert_eq!(off.dissipated_power, 0.0);
    let before = fluid.diagnostics().unwrap().accelerations;
    let baseline = slip.diagnostics().unwrap().accelerations;
    for (index, p) in ps.iter().enumerate() {
        for axis in 0..3 {
            let epsilon = 1e-6;
            let mut plus = ps.clone();
            plus[index].velocity[axis] += epsilon;
            let mut minus = ps.clone();
            minus[index].velocity[axis] -= epsilon;
            let derivative = (make_drag(plus)
                .reflecting_viscous_diagnostics()
                .unwrap()
                .dissipated_power
                - make_drag(minus)
                    .reflecting_viscous_diagnostics()
                    .unwrap()
                    .dissipated_power)
                / (4.0 * epsilon);
            assert!((loads.accelerations[index][axis] * p.mass + derivative).abs() < 1e-9);
            assert!(
                (before[index][axis] - baseline[index][axis] - loads.accelerations[index][axis])
                    .abs()
                    < 1e-10
            );
        }
    }
    let momentum = |f: &Liquid| -> [f64; 3] {
        std::array::from_fn(|a| f.particles().iter().map(|p| p.mass * p.velocity[a]).sum())
    };
    let angular = |f: &Liquid| -> [f64; 3] {
        std::array::from_fn(|a| {
            f.particles()
                .iter()
                .map(|p| {
                    p.mass
                        * (p.position[(a + 1) % 3] * p.velocity[(a + 2) % 3]
                            - p.position[(a + 2) % 3] * p.velocity[(a + 1) % 3])
                })
                .sum()
        })
    };
    let initial = momentum(&fluid);
    let spin = angular(&fluid);
    let dt = 1e-7;
    assert_eq!(fluid.step(dt, None).unwrap().substeps, 1);
    for a in 0..3 {
        assert!((momentum(&fluid)[a] - initial[a] + dt * loads.reaction_force[a]).abs() < 1e-12);
        assert!(
            (angular(&fluid)[a] - spin[a] + dt * loads.reaction_torque_about_origin[a]).abs()
                < 1e-12
        );
    }
}

#[test]
fn frozen_viscous_stage_preserves_geometry_and_reports_integrated_fixture_balance() {
    let mut ps = particles();
    for (speed, p) in [0.1, 0.2, 0.3].into_iter().zip(&mut ps) {
        p.velocity = [speed, -0.08, 0.05];
    }
    let mut ms = materials();
    for m in &mut ms {
        m.viscosity = 0.01;
    }
    let mut fluid = Liquid::new(
        ps.clone(),
        ms,
        Config {
            gravity: [0.0, -9.81, 0.0],
            ..config()
        },
    )
    .unwrap();
    fluid.set_formulation(Formulation::RestVolumeWendland);
    fluid.set_reflecting_box(Some(bounds())).unwrap();
    fluid.set_reflecting_no_slip(true).unwrap();
    let before = fluid.clone();
    assert_eq!(fluid.relax_viscosity(0.01), Err(Error::InvalidTransport));
    assert_eq!(fluid, before);
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.1
                };
                3
            ],
            vec![
                TransportMaterial {
                    specific_heat: 2.0,
                    conductivity: 1.0,
                    diffusivity: 1.0,
                    ..TransportMaterial::default()
                };
                2
            ],
        )
        .unwrap();
    let before = fluid.clone();
    assert_eq!(fluid.relax_viscosity(f64::NAN), Err(Error::InvalidTimeStep));
    assert_eq!(fluid, before);
    let initial_heat = fluid.transport_totals().unwrap().unwrap().0;
    let initial_kinetic = kinetic(&fluid);
    let stats = fluid.relax_viscosity(0.01).unwrap();
    assert!(stats.kinetic_energy_loss > 0.0);
    assert!((initial_kinetic - kinetic(&fluid) - stats.kinetic_energy_loss).abs() < 1e-12);
    assert!(
        (fluid.transport_totals().unwrap().unwrap().0 - initial_heat - stats.kinetic_energy_loss)
            .abs()
            < 1e-10
    );
    let mut impulse = [0.0; 3];
    let mut torque = [0.0; 3];
    for (old, new) in ps.iter().zip(fluid.particles()) {
        assert_eq!(old.position, new.position);
        for a in 0..3 {
            let b = (a + 1) % 3;
            let c = (a + 2) % 3;
            impulse[a] += old.mass * (old.velocity[a] - new.velocity[a]);
            torque[a] += old.mass
                * (old.position[b] * (old.velocity[c] - new.velocity[c])
                    - old.position[c] * (old.velocity[b] - new.velocity[b]));
        }
    }
    for a in 0..3 {
        assert!((impulse[a] - stats.fixture_impulse[a]).abs() < 1e-12);
        assert!((torque[a] - stats.fixture_angular_impulse_about_origin[a]).abs() < 1e-12);
    }
}

#[test]
fn frozen_viscous_stage_converges_under_time_refinement() {
    let mut ps = particles();
    for (speed, p) in [0.1, 0.2, 0.3].into_iter().zip(&mut ps) {
        p.velocity = [speed, -0.08, 0.05];
    }
    let mut ms = materials();
    for m in &mut ms {
        m.viscosity = 0.1;
    }
    let mut fluid = Liquid::new(ps, ms, config()).unwrap();
    fluid.set_formulation(Formulation::RestVolumeWendland);
    fluid.set_reflecting_box(Some(bounds())).unwrap();
    fluid.set_reflecting_no_slip(true).unwrap();
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.0
                };
                3
            ],
            vec![
                TransportMaterial {
                    specific_heat: 2.0,
                    conductivity: 0.0,
                    ..TransportMaterial::default()
                };
                2
            ],
        )
        .unwrap();
    let run = |steps: u32| {
        let mut f = fluid.clone();
        for _ in 0..steps {
            f.relax_viscosity(0.05 / f64::from(steps)).unwrap();
        }
        f
    };
    let reference = run(256);
    let error = |f: Liquid| {
        f.particles()
            .iter()
            .zip(reference.particles())
            .map(|(p, q)| {
                p.velocity
                    .iter()
                    .zip(q.velocity)
                    .map(|(v, w)| (v - w).powi(2))
                    .sum::<f64>()
            })
            .sum::<f64>()
            .sqrt()
    };
    let coarse = error(run(4));
    let medium = error(run(8));
    let fine = error(run(16));
    assert!(coarse > 1e-8);
    assert!(medium < 0.7 * coarse);
    assert!(fine < 0.7 * medium);
}

#[test]
fn symmetric_viscous_stage_has_second_order_frozen_time_convergence() {
    let mut ps = particles();
    for (speed, p) in [0.1, 0.2, 0.3].into_iter().zip(&mut ps) {
        p.velocity = [speed, -0.08, 0.05];
    }
    let mut ms = materials();
    for m in &mut ms {
        m.viscosity = 0.1;
    }
    let mut fluid = Liquid::new(ps, ms, config()).unwrap();
    fluid.set_formulation(Formulation::RestVolumeWendland);
    fluid.set_reflecting_box(Some(bounds())).unwrap();
    fluid.set_reflecting_no_slip(true).unwrap();
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.0
                };
                3
            ],
            vec![
                TransportMaterial {
                    specific_heat: 2.0,
                    conductivity: 0.0,
                    ..TransportMaterial::default()
                };
                2
            ],
        )
        .unwrap();
    let run = |steps: u32| {
        let mut f = fluid.clone();
        for _ in 0..steps {
            f.relax_viscosity_symmetric(0.5 / f64::from(steps)).unwrap();
        }
        f
    };
    let reference = run(256);
    let error = |f: Liquid| {
        f.particles()
            .iter()
            .zip(reference.particles())
            .map(|(p, q)| {
                p.velocity
                    .iter()
                    .zip(q.velocity)
                    .map(|(v, w)| (v - w).powi(2))
                    .sum::<f64>()
            })
            .sum::<f64>()
            .sqrt()
    };
    let coarse = error(run(4));
    let medium = error(run(8));
    let fine = error(run(16));
    eprintln!("symmetric errors: {coarse}, {medium}, {fine}");
    assert!(coarse > 1e-8);
    assert!(medium < 0.35 * coarse);
    assert!(fine < 0.35 * medium);
}

#[test]
fn ordinary_step_uses_selected_symmetric_viscosity_and_advects_relaxed_velocity() {
    use physics::liquid::ViscousIntegrator;
    let mut ps = particles();
    for (speed, p) in [0.1, 0.2, 0.3].into_iter().zip(&mut ps) {
        p.velocity = [speed, -0.08, 0.05];
    }
    let mut ms = materials();
    for m in &mut ms {
        m.rest_density = 1e6;
        m.viscosity = 0.1;
    }
    let mut fluid = Liquid::new(ps.clone(), ms, config()).unwrap();
    assert_eq!(fluid.viscous_integrator(), ViscousIntegrator::Sequential);
    fluid.set_formulation(Formulation::RestVolumeWendland);
    fluid.set_reflecting_box(Some(bounds())).unwrap();
    fluid.set_reflecting_no_slip(true).unwrap();
    fluid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.0
                };
                3
            ],
            vec![
                TransportMaterial {
                    specific_heat: 2.0,
                    conductivity: 0.0,
                    ..TransportMaterial::default()
                };
                2
            ],
        )
        .unwrap();
    fluid.set_viscous_heating(true).unwrap();
    fluid
        .set_viscous_integrator(ViscousIntegrator::Symmetric)
        .unwrap();
    let mut stage = fluid.clone();
    let dt = 0.0001;
    stage.relax_viscosity_symmetric(dt).unwrap();
    assert_eq!(fluid.step(dt, None).unwrap().substeps, 1);
    for ((p, q), old) in fluid.particles().iter().zip(stage.particles()).zip(ps) {
        for axis in 0..3 {
            assert!((p.velocity[axis] - q.velocity[axis]).abs() < 1e-12);
            assert!((p.position[axis] - old.position[axis] - dt * q.velocity[axis]).abs() < 1e-12);
        }
    }
    assert_eq!(fluid.fields(), stage.fields());
}

#[test]
fn symmetric_integrator_rejects_sampled_walls_atomically_in_both_configuration_orders() {
    use physics::liquid::{BoundarySample, ViscousIntegrator};
    let sample = BoundarySample {
        position: [0.0; 3],
        volume: 0.1,
    };
    let mut fluid = Liquid::new(particles(), materials(), config()).unwrap();
    fluid.configure_boundaries(vec![sample]).unwrap();
    let before = fluid.clone();
    assert_eq!(
        fluid.set_viscous_integrator(ViscousIntegrator::Symmetric),
        Err(Error::InvalidBoundary)
    );
    assert_eq!(fluid, before);
    fluid.configure_boundaries(vec![]).unwrap();
    fluid
        .set_viscous_integrator(ViscousIntegrator::Symmetric)
        .unwrap();
    let before = fluid.clone();
    assert_eq!(
        fluid.configure_boundaries(vec![sample]),
        Err(Error::InvalidBoundary)
    );
    assert_eq!(fluid, before);
}
