use physics::liquid::{
    Config, Liquid, LiquidField, Material, Particle, ParticleInput, TransportMaterial,
};
fn fluid(materials: bool) -> Liquid {
    let mut state = Liquid::new(
        vec![
            Particle {
                position: [0.0; 3],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            },
            Particle {
                position: [0.4, 0.0, 0.0],
                velocity: [0.0; 3],
                mass: 3.0,
                material: usize::from(materials),
            },
        ],
        vec![
            Material {
                viscosity: 0.0,
                sound_speed: 1.0,
                ..Material::WATER
            };
            if materials { 2 } else { 1 }
        ],
        Config {
            smoothing_radius: 1.0,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    state
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.0
                };
                2
            ],
            vec![
                TransportMaterial {
                    specific_heat: 1.0,
                    conductivity: 0.0,
                    diffusivity: 2.0,
                    mixing_group: 0
                };
                if materials { 2 } else { 1 }
            ],
        )
        .unwrap();
    state
        .configure_species(
            vec!["fuel".into(), "oxidizer".into(), "carrier".into()],
            vec![vec![1.0, 0.0, 0.0], vec![0.0, 0.25, 0.75]],
        )
        .unwrap();
    state
}
#[test]
fn complete_species_diffusion_is_analytic_positive_and_mass_conserving() {
    for symmetric in [false, true] {
        let mut state = fluid(false);
        let before = state.species_totals().unwrap().unwrap();
        let thermal = state.transport_totals().unwrap().unwrap().0;
        if symmetric {
            state.set_viscous_heating(true).unwrap();
            state.step_symmetric_free(0.03).unwrap();
        } else {
            state.step(0.03, None).unwrap();
        }
        let decay = (-2.0_f64 * 0.6 * 0.03).exp();
        let initial = [[1.0, 0.0, 0.0], [0.0, 0.25, 0.75]];
        let rows = state.species_fractions().unwrap();
        for component in 0..3 {
            let equilibrium = before[component] / 4.0;
            for particle in 0..2 {
                let expected = equilibrium + (initial[particle][component] - equilibrium) * decay;
                assert!((rows[particle][component] - expected).abs() < 1e-12);
            }
        }
        for row in rows {
            assert!((row.iter().sum::<f64>() - 1.0).abs() < 1e-12);
            assert!(row.iter().all(|v| (0.0..=1.0).contains(v)));
        }
        for (a, b) in before.iter().zip(state.species_totals().unwrap().unwrap()) {
            assert!((a - b).abs() < 1e-12);
        }
        assert!((state.transport_totals().unwrap().unwrap().0 - thermal).abs() < 1e-12);
    }
}
#[test]
fn mixing_groups_block_and_enable_complete_vector_diffusion() {
    let mut state = fluid(true);
    let initial = state.species_fractions().unwrap().to_vec();
    state.step(0.03, None).unwrap();
    assert_eq!(state.species_fractions().unwrap(), initial);
    state
        .configure_transport(
            state.fields().unwrap().to_vec(),
            vec![
                TransportMaterial {
                    specific_heat: 1.0,
                    conductivity: 0.0,
                    diffusivity: 2.0,
                    mixing_group: 7
                };
                2
            ],
        )
        .unwrap();
    assert_eq!(state.species_fractions().unwrap(), initial);
    state.step(0.03, None).unwrap();
    assert_ne!(state.species_fractions().unwrap(), initial);
    let expected = [1.0, 0.75, 2.25];
    for (actual, expected) in state
        .species_totals()
        .unwrap()
        .unwrap()
        .iter()
        .zip(expected)
    {
        assert!((actual - expected).abs() < 1e-12);
    }
}
#[test]
fn species_sources_drains_and_emission_preserve_component_masses() {
    use physics::liquid::{EmissionPulse, PulsedEmitter};
    let mut state = fluid(false);
    let source = ParticleInput {
        particle: Particle {
            position: [2.0, 0.0, 0.0],
            velocity: [0.0; 3],
            mass: 2.0,
            material: 0,
        },
        field: Some(LiquidField {
            temperature: 300.0,
            concentration: 0.0,
        }),
        phase_fraction: None,
    };
    state
        .exchange_particles_with_species(&[0], &[source], &[vec![0.2, 0.3, 0.5]])
        .unwrap();
    let expected = [0.4, 1.35, 3.25];
    for (actual, expected) in state
        .species_totals()
        .unwrap()
        .unwrap()
        .iter()
        .zip(expected)
    {
        assert!((actual - expected).abs() < 1e-12);
    }
    let mut emitter = PulsedEmitter::new(
        vec![EmissionPulse {
            start: 0.0,
            duration: 1.0,
            volume: 0.0025,
            speed: 0.0,
        }],
        source,
    )
    .unwrap();
    emitter.particle_volume = 0.001;
    emitter.density = 1000.0;
    let emitted = emitter
        .advance_with_species(&mut state, 1.0, &[0.4, 0.4, 0.2])
        .unwrap();
    assert!((emitted.added.mass - 2.5).abs() < 1e-12);
    for (actual, expected) in state
        .species_totals()
        .unwrap()
        .unwrap()
        .iter()
        .zip([1.4, 2.35, 3.75])
    {
        assert!((actual - expected).abs() < 1e-12);
    }
    state.exchange_particles(&[1], &[]).unwrap();
    for (actual, expected) in state
        .species_totals()
        .unwrap()
        .unwrap()
        .iter()
        .zip([1.0, 1.75, 2.75])
    {
        assert!((actual - expected).abs() < 1e-12);
    }
}
#[test]
fn invalid_compositions_sources_and_failed_motion_roll_back_complete_state() {
    let mut state = fluid(false);
    let before = state.clone();
    assert!(
        state
            .configure_species(vec!["a".into(), "a".into()], vec![vec![1.0, 0.0]; 2])
            .is_err()
    );
    assert_eq!(state, before);
    assert!(
        state
            .configure_species(vec!["a".into(), "b".into()], vec![vec![0.5, 0.6]; 2])
            .is_err()
    );
    assert_eq!(state, before);
    let source = ParticleInput {
        particle: state.particles()[0],
        field: Some(state.fields().unwrap()[0]),
        phase_fraction: None,
    };
    assert!(state.exchange_particles(&[], &[source]).is_err());
    assert_eq!(state, before);
    assert!(
        state
            .exchange_particles_with_species(&[], &[source], &[vec![f64::NAN, 0.0, 1.0]])
            .is_err()
    );
    assert_eq!(state, before);
    let mut emitter = physics::liquid::PulsedEmitter::new(
        vec![physics::liquid::EmissionPulse {
            start: 0.0,
            duration: 1.0,
            volume: 1e-8,
            speed: 1.0,
        }],
        source,
    )
    .unwrap();
    let emitter_before = emitter.clone();
    assert!(
        emitter
            .advance_with_species(&mut state, 1.0, &[0.4, 0.4, 0.4])
            .is_err()
    );
    assert_eq!(emitter, emitter_before);
    assert_eq!(state, before);
    assert!(emitter.advance(&mut state, 1.0).is_err());
    assert_eq!(emitter, emitter_before);
    assert_eq!(state, before);
    assert!(
        state
            .step_with_world(0.03, None, &FailingWorld, Default::default())
            .is_err()
    );
    assert_eq!(state, before);
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
        Err("deliberate failure")
    }
}

#[test]
fn complete_composition_advects_with_gas_without_creating_thermal_energy() {
    use physics::liquid::IdealGas;
    let eos = IdealGas {
        gas_constant: 1.0,
        heat_capacity_ratio: 1.4,
    };
    let mut state = Liquid::new(
        vec![Particle {
            position: [0.0; 3],
            velocity: [0.5, 0.1, 0.0],
            mass: 1.0,
            material: 0,
        }],
        vec![Material {
            viscosity: 0.0,
            ..Material::WATER
        }],
        Config {
            smoothing_radius: 1.0,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    state
        .configure_transport(
            vec![LiquidField {
                temperature: 300.0,
                concentration: 0.0,
            }],
            vec![TransportMaterial {
                specific_heat: eos.specific_heat_cv().unwrap(),
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )
        .unwrap();
    state.configure_gas_equations(vec![Some(eos)]).unwrap();
    state
        .configure_species(
            vec!["fuel".into(), "oxygen".into(), "products".into()],
            vec![vec![0.2, 0.3, 0.5]],
        )
        .unwrap();
    let before = state.species_fractions().unwrap().to_vec();
    let thermal = state.transport_totals().unwrap().unwrap().0;
    state.step_symmetric_free(0.01).unwrap();
    assert_eq!(state.species_fractions().unwrap(), before);
    assert!((state.particles()[0].position[0] - 0.005).abs() < 1e-12);
    assert!((state.particles()[0].position[1] - 0.001).abs() < 1e-12);
    assert!((state.transport_totals().unwrap().unwrap().0 - thermal).abs() < 1e-10);
}
