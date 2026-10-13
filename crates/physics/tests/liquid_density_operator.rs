use physics::liquid::{Formulation, LiquidDensityOperator, Material, Particle};

#[test]
fn accelerated_density_step_admits_whole_results_and_preserves_failure_state() {
    use physics::liquid::{Config, Liquid};
    let particles = vec![
        Particle {
            position: [0.; 3],
            velocity: [0.1, 0., 0.],
            mass: 1.,
            material: 0,
        },
        Particle {
            position: [0.1, 0., 0.],
            velocity: [0.; 3],
            mass: 2.,
            material: 0,
        },
    ];
    let config = Config {
        smoothing_radius: 1.,
        gravity: [0.; 3],
        ..Config::default()
    };
    let initial = Liquid::new(particles, vec![Material::WATER], config).unwrap();
    let mut native = initial.clone();
    native.step(1e-4, None).unwrap();
    let mut accelerated = initial.clone();
    let mut exact =
        |operator: &LiquidDensityOperator<'_>| Some(operator.evaluate_native().unwrap());
    let (_, counts) = accelerated
        .step_with_density_backend(1e-4, None, &mut exact, 0., 0.)
        .unwrap();
    assert_eq!(accelerated, native);
    assert!(counts.admitted > 0);
    assert_eq!(counts.native_fallbacks, 0);
    for invalid in [None, Some(vec![f64::NAN; 2]), Some(vec![1.])] {
        let mut rejected = initial.clone();
        let mut backend = |_: &LiquidDensityOperator<'_>| invalid.clone();
        let (_, counts) = rejected
            .step_with_density_backend(1e-4, None, &mut backend, 0., 0.)
            .unwrap();
        assert_eq!(rejected, native);
        assert_eq!(counts.admitted, 0);
        assert_eq!(counts.calls, counts.native_fallbacks);
    }
    let mut limited = Liquid::new(
        initial.particles().to_vec(),
        vec![Material::WATER],
        Config {
            max_substeps: 1,
            ..config
        },
    )
    .unwrap();
    let before = limited.clone();
    assert!(
        limited
            .step_with_density_backend(0.1, None, &mut exact, 0., 0.)
            .is_err()
    );
    assert_eq!(limited, before);
}

#[test]
fn density_owner_admits_current_mixture_and_rejects_stale_or_corrupt_candidates() {
    let mut particles = [
        Particle {
            position: [0.; 3],
            velocity: [0.; 3],
            mass: 1.,
            material: 0,
        },
        Particle {
            position: [0.; 3],
            velocity: [0.; 3],
            mass: 2.,
            material: 1,
        },
    ];
    let properties = [
        Material::WATER,
        Material {
            rest_density: 500.,
            ..Material::WATER
        },
    ];
    for formulation in [
        Formulation::MassDensity,
        Formulation::RestVolume,
        Formulation::RestVolumeWendland,
    ] {
        let operator = LiquidDensityOperator {
            particles: &particles,
            pairs: &[(0, 1)],
            properties: &properties,
            formulation,
            smoothing_radius: 1.,
            carrier_droplets: None,
        };
        let density = operator.evaluate_native().unwrap();
        let kernel = if formulation == Formulation::RestVolumeWendland {
            21. / (2. * std::f64::consts::PI)
        } else {
            315. / (64. * std::f64::consts::PI)
        };
        let factors = if formulation == Formulation::MassDensity {
            [3., 3.]
        } else {
            [5., 2.5]
        };
        for (rho, factor) in density.iter().zip(factors) {
            assert!((rho - factor * kernel).abs() < 1e-13);
        }
        operator.admit(&density, 0., 0.).unwrap();
        for value in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(operator.admit(&[density[0], value], 0., 0.).is_err());
        }
        assert!(operator.admit(&density[..1], 0., 0.).is_err());
        let droplets = LiquidDensityOperator {
            carrier_droplets: Some(&[true, false]),
            ..operator
        };
        let overridden = droplets.evaluate_native().unwrap();
        assert_eq!(overridden[0], 1000.);
        assert_eq!(overridden[1], density[1]);
    }
    let old = LiquidDensityOperator {
        particles: &particles,
        pairs: &[(0, 1)],
        properties: &properties,
        formulation: Formulation::MassDensity,
        smoothing_radius: 1.,
        carrier_droplets: None,
    }
    .evaluate_native()
    .unwrap();
    particles[1].position[0] = 0.5;
    let current = LiquidDensityOperator {
        particles: &particles,
        pairs: &[(0, 1)],
        properties: &properties,
        formulation: Formulation::MassDensity,
        smoothing_radius: 1.,
        carrier_droplets: None,
    };
    assert!(current.admit(&old, 0., 0.).is_err());
    for pairs in [
        &[(0, 0)][..],
        &[(0, 2)][..],
        &[(0, 1), (0, 1)][..],
        &[(1, 0)][..],
    ] {
        assert!(
            LiquidDensityOperator { pairs, ..current }
                .validate()
                .is_err()
        );
    }
}

#[test]
fn pair_force_backend_is_whole_batch_admitted_and_failed_step_is_atomic() {
    use physics::liquid::{
        Config, Liquid, LiquidComputeBackend, LiquidPairForceInput, LiquidPairForceResult,
    };
    struct Backend {
        bad: bool,
    }
    impl LiquidComputeBackend for Backend {
        fn candidate(&mut self, _: &LiquidDensityOperator<'_>) -> Option<Vec<f64>> {
            None
        }
        fn supports_pair_forces(&self) -> bool {
            true
        }
        fn pair_force_candidates(
            &mut self,
            inputs: &[LiquidPairForceInput],
        ) -> Option<Vec<LiquidPairForceResult>> {
            let mut results = inputs
                .iter()
                .map(|p| p.evaluate_native().unwrap())
                .collect::<Vec<_>>();
            if self.bad {
                results.last_mut().unwrap().force[1] = f64::NAN;
            }
            Some(results)
        }
    }
    let particles = (0..3)
        .map(|i| Particle {
            position: [i as f64 * 0.01, 0., 0.],
            velocity: [0.1 * i as f64, 0., 0.],
            mass: 0.001,
            material: 0,
        })
        .collect();
    let config = Config {
        gravity: [0.; 3],
        ..Config::default()
    };
    let initial = Liquid::new(particles, vec![Material::WATER], config).unwrap();
    let mut native = initial.clone();
    native.step(1e-5, None).unwrap();
    for bad in [false, true] {
        let mut actual = initial.clone();
        let (_, stats) = actual
            .step_with_compute_backend(1e-5, None, &mut Backend { bad }, 0., 0.)
            .unwrap();
        assert_eq!(actual, native);
        assert!(stats.pair_force_calls > 0);
        if bad {
            assert_eq!(stats.pair_force_admitted, 0);
            assert_eq!(stats.pair_force_native_fallbacks, stats.pair_force_calls);
        } else {
            assert_eq!(stats.pair_force_native_fallbacks, 0);
            assert_eq!(stats.pair_force_admitted, stats.pair_force_calls);
        }
    }
    let mut limited = Liquid::new(
        initial.particles().to_vec(),
        vec![Material::WATER],
        Config {
            max_substeps: 1,
            ..config
        },
    )
    .unwrap();
    let before = limited.clone();
    assert!(
        limited
            .step_with_compute_backend(0.1, None, &mut Backend { bad: false }, 0., 0.)
            .is_err()
    );
    assert_eq!(limited, before);
}

#[test]
fn normal_backend_rejects_partial_corrupt_and_stale_vectors_as_a_whole() {
    use physics::liquid::{Config, Liquid, LiquidComputeBackend, LiquidNormalOperator};
    struct Backend {
        mode: usize,
    }
    impl LiquidComputeBackend for Backend {
        fn candidate(&mut self, _: &LiquidDensityOperator<'_>) -> Option<Vec<f64>> {
            None
        }
        fn supports_normals(&self) -> bool {
            true
        }
        fn normal_candidates(
            &mut self,
            operator: &LiquidNormalOperator<'_>,
        ) -> Option<Vec<[f64; 3]>> {
            let mut result = operator.evaluate_native().unwrap();
            match self.mode {
                1 => {
                    result.pop();
                }
                2 => {
                    result.last_mut().unwrap()[0] = f64::NAN;
                }
                3 => {
                    result.last_mut().unwrap()[0] += 1.;
                }
                _ => {}
            }
            Some(result)
        }
    }
    let config = Config {
        gravity: [0.; 3],
        ..Config::default()
    };
    let particles = (0..3)
        .map(|i| Particle {
            position: [i as f64 * 0.01, 0., 0.],
            velocity: [0.01 * i as f64, 0., 0.],
            mass: 0.001,
            material: 0,
        })
        .collect();
    let initial = Liquid::new(particles, vec![Material::WATER], config).unwrap();
    let mut native = initial.clone();
    native.step(1e-5, None).unwrap();
    for mode in 0..4 {
        let mut actual = initial.clone();
        let (_, stats) = actual
            .step_with_compute_backend(1e-5, None, &mut Backend { mode }, 0., 0.)
            .unwrap();
        assert_eq!(actual, native);
        assert!(stats.normal_calls > 0);
        if mode == 0 {
            assert_eq!(stats.normal_admitted, stats.normal_calls);
            assert_eq!(stats.normal_native_fallbacks, 0);
        } else {
            assert_eq!(stats.normal_admitted, 0);
            assert_eq!(stats.normal_native_fallbacks, stats.normal_calls);
        }
    }
    let mut limited = Liquid::new(
        initial.particles().to_vec(),
        vec![Material::WATER],
        Config {
            max_substeps: 1,
            ..config
        },
    )
    .unwrap();
    let before = limited.clone();
    assert!(
        limited
            .step_with_compute_backend(0.1, None, &mut Backend { mode: 0 }, 0., 0.)
            .is_err()
    );
    assert_eq!(limited, before);
    let particles = initial.particles();
    let densities = vec![1000.; particles.len()];
    let operator = LiquidNormalOperator {
        particles,
        pairs: &[(0, 1)],
        densities: &densities,
        smoothing_radius: 0.2,
    };
    let stale = operator.evaluate_native().unwrap();
    let changed = LiquidNormalOperator {
        smoothing_radius: 0.1,
        ..operator
    };
    assert!(changed.admit(&stale, 0., 0.).is_err());
}

#[test]
fn accumulation_backend_rejects_whole_corrupt_batch_and_preserves_late_failure() {
    use physics::liquid::{
        Config, Liquid, LiquidAccumulationOperator, LiquidAccumulationResult, LiquidComputeBackend,
    };
    struct Backend {
        mode: usize,
    }
    impl LiquidComputeBackend for Backend {
        fn candidate(&mut self, _: &LiquidDensityOperator<'_>) -> Option<Vec<f64>> {
            None
        }
        fn supports_accumulation(&self) -> bool {
            true
        }
        fn accumulation_candidate(
            &mut self,
            operator: &LiquidAccumulationOperator<'_>,
        ) -> Option<LiquidAccumulationResult> {
            let mut result = operator.evaluate_native().unwrap();
            match self.mode {
                1 => {
                    result.accelerations.pop();
                }
                2 => {
                    result.accelerations.last_mut().unwrap()[0] = f64::NAN;
                }
                3 => {
                    result.viscous_rates.last_mut().map(|v| *v = -1.);
                }
                4 => {
                    result.accelerations.last_mut().unwrap()[0] += 1.;
                }
                _ => {}
            }
            Some(result)
        }
    }
    let config = Config {
        gravity: [0., -9.81, 0.],
        ..Config::default()
    };
    let particles = (0..3)
        .map(|i| Particle {
            position: [i as f64 * 0.01, 0., 0.],
            velocity: [0.01 * i as f64, 0., 0.],
            mass: 0.001 * (1. + i as f64),
            material: 0,
        })
        .collect();
    let initial = Liquid::new(particles, vec![Material::WATER], config).unwrap();
    let mut native = initial.clone();
    native.step(1e-5, None).unwrap();
    for mode in 0..5 {
        let mut actual = initial.clone();
        let (_, stats) = actual
            .step_with_compute_backend(1e-5, None, &mut Backend { mode }, 0., 0.)
            .unwrap();
        assert_eq!(actual, native);
        assert!(stats.accumulation_calls > 0);
        if mode == 0 {
            assert_eq!(stats.accumulation_admitted, stats.accumulation_calls);
            assert_eq!(stats.accumulation_native_fallbacks, 0);
        } else {
            assert_eq!(stats.accumulation_admitted, 0);
            assert_eq!(
                stats.accumulation_native_fallbacks,
                stats.accumulation_calls
            );
        }
    }
    let mut limited = Liquid::new(
        initial.particles().to_vec(),
        vec![Material::WATER],
        Config {
            max_substeps: 1,
            ..config
        },
    )
    .unwrap();
    let before = limited.clone();
    assert!(
        limited
            .step_with_compute_backend(0.1, None, &mut Backend { mode: 0 }, 0., 0.)
            .is_err()
    );
    assert_eq!(limited, before);
}

#[test]
fn joint_force_pipeline_never_publishes_partially_admitted_state() {
    use physics::liquid::{
        Config, Liquid, LiquidComputeBackend, LiquidForcePipelineOperator,
        LiquidForcePipelineResult,
    };
    struct Backend {
        mode: usize,
    }
    impl LiquidComputeBackend for Backend {
        fn candidate(&mut self, _: &LiquidDensityOperator<'_>) -> Option<Vec<f64>> {
            None
        }
        fn supports_force_pipeline(&self) -> bool {
            true
        }
        fn force_pipeline_candidate(
            &mut self,
            operator: &LiquidForcePipelineOperator<'_>,
        ) -> Option<LiquidForcePipelineResult> {
            let mut result = operator.evaluate_native().unwrap();
            match self.mode {
                1 => {
                    result.forces.last_mut().unwrap().force[0] = f64::NAN;
                }
                2 => {
                    result.accumulation.accelerations.last_mut().unwrap()[0] += 1.;
                }
                3 => {
                    result.forces.pop();
                }
                _ => {}
            }
            Some(result)
        }
    }
    let config = Config {
        gravity: [0., -9.81, 0.],
        ..Config::default()
    };
    let particles = (0..3)
        .map(|i| Particle {
            position: [i as f64 * 0.01, 0., 0.],
            velocity: [0.01 * i as f64, 0., 0.],
            mass: 0.001 * (1. + i as f64),
            material: 0,
        })
        .collect();
    let initial = Liquid::new(particles, vec![Material::WATER], config).unwrap();
    let mut native = initial.clone();
    native.step(1e-5, None).unwrap();
    for mode in 0..4 {
        let mut actual = initial.clone();
        let (_, stats) = actual
            .step_with_compute_backend(1e-5, None, &mut Backend { mode }, 0., 0.)
            .unwrap();
        assert_eq!(actual, native);
        assert!(stats.force_pipeline_calls > 0);
        if mode == 0 {
            assert_eq!(stats.force_pipeline_admitted, stats.force_pipeline_calls);
            assert_eq!(stats.force_pipeline_native_fallbacks, 0);
        } else {
            assert_eq!(stats.force_pipeline_admitted, 0);
            assert_eq!(
                stats.force_pipeline_native_fallbacks,
                stats.force_pipeline_calls
            );
        }
    }
    let mut limited = Liquid::new(
        initial.particles().to_vec(),
        vec![Material::WATER],
        Config {
            max_substeps: 1,
            ..config
        },
    )
    .unwrap();
    let before = limited.clone();
    assert!(
        limited
            .step_with_compute_backend(0.1, None, &mut Backend { mode: 0 }, 0., 0.)
            .is_err()
    );
    assert_eq!(limited, before);
}

#[test]
fn supported_density_normal_pipeline_preserves_wall_images_and_atomic_failure() {
    use physics::liquid::{
        BoundarySample, Config, Liquid, LiquidComputeBackend, LiquidDensityNormalOperator,
        LiquidDensityNormalResult, ReflectingBox,
    };
    struct Backend {
        mode: usize,
        support_seen: bool,
    }
    impl LiquidComputeBackend for Backend {
        fn candidate(&mut self, _: &LiquidDensityOperator<'_>) -> Option<Vec<f64>> {
            panic!("joint capability must bypass separate density callback")
        }
        fn supports_density_normal_pressure(&self) -> bool {
            true
        }
        fn supports_density_normals(&self) -> bool {
            true
        }
        fn density_normal_candidate(
            &mut self,
            operator: &LiquidDensityNormalOperator<'_>,
        ) -> Option<LiquidDensityNormalResult> {
            self.support_seen |= !operator.support.is_empty();
            let mut result = operator.evaluate_native().unwrap();
            match self.mode {
                1 => {
                    result.densities[0] = f64::NAN;
                }
                2 => {
                    result.normals.last_mut().unwrap()[0] += 1.;
                }
                3 => {
                    result.normals.pop();
                }
                4 => {
                    result.pressures.as_mut().unwrap()[0] = f64::NAN;
                }
                5 => {
                    result.pressures = None;
                }
                _ => {}
            }
            Some(result)
        }
    }
    let config = Config {
        gravity: [0.; 3],
        smoothing_radius: 1.,
        ..Config::default()
    };
    for formulation in [
        Formulation::MassDensity,
        Formulation::RestVolume,
        Formulation::RestVolumeWendland,
    ] {
        for reflected in [false, true] {
            if reflected && formulation == Formulation::MassDensity {
                continue;
            }
            let particles = (0..3)
                .map(|i| Particle {
                    position: [0.12 + i as f64 * 0.04, 0.15, 0.2],
                    velocity: [0.01 * i as f64, 0., 0.],
                    mass: 0.1,
                    material: 0,
                })
                .collect();
            let mut initial = Liquid::new(
                particles,
                vec![Material {
                    rest_density: 1.,
                    sound_speed: 1.,
                    viscosity: 0.01,
                }],
                config,
            )
            .unwrap();
            initial.set_formulation(formulation);
            if !reflected {
                initial
                    .configure_boundaries(vec![BoundarySample {
                        position: [-0.1, 0.15, 0.2],
                        volume: 0.1,
                    }])
                    .unwrap();
            }
            if reflected {
                initial
                    .set_reflecting_box(Some(ReflectingBox {
                        min: [0.; 3],
                        max: [3.; 3],
                    }))
                    .unwrap();
            }
            let mut native = initial.clone();
            native.step(1e-5, None).unwrap();
            for mode in 0..6 {
                let mut actual = initial.clone();
                let mut backend = Backend {
                    mode,
                    support_seen: false,
                };
                let (_, stats) = actual
                    .step_with_compute_backend(1e-5, None, &mut backend, 0., 0.)
                    .unwrap();
                assert_eq!(actual, native);
                assert!(backend.support_seen);
                assert!(stats.density_normal_calls > 0);
                if mode == 0 {
                    assert_eq!(
                        stats.density_normal_pressure_admitted,
                        stats.density_normal_calls
                    );
                    assert_eq!(stats.density_normal_admitted, stats.density_normal_calls);
                    assert_eq!(stats.density_normal_native_fallbacks, 0);
                } else {
                    assert_eq!(stats.density_normal_admitted, 0);
                    assert_eq!(
                        stats.density_normal_native_fallbacks,
                        stats.density_normal_calls
                    );
                }
            }
            let mut moving = initial.particles().to_vec();
            for p in &mut moving {
                p.velocity[0] += 100.;
            }
            let mut limited = Liquid::new(
                moving,
                vec![Material {
                    rest_density: 1.,
                    sound_speed: 1.,
                    viscosity: 0.01,
                }],
                Config {
                    max_substeps: 1,
                    ..config
                },
            )
            .unwrap();
            limited.set_formulation(formulation);
            if reflected {
                limited
                    .set_reflecting_box(Some(ReflectingBox {
                        min: [0.; 3],
                        max: [3.; 3],
                    }))
                    .unwrap();
            } else {
                limited
                    .configure_boundaries(vec![BoundarySample {
                        position: [-0.1, 0.15, 0.2],
                        volume: 0.1,
                    }])
                    .unwrap();
            }
            let before = limited.clone();
            let mut backend = Backend {
                mode: 0,
                support_seen: false,
            };
            assert!(
                limited
                    .step_with_compute_backend(0.1, None, &mut backend, 0., 0.)
                    .is_err()
            );
            assert!(backend.support_seen);
            assert_eq!(limited, before);
        }
    }
}

#[test]
fn original_pressure_laws_cover_compression_gas_and_current_density_admission() {
    use physics::liquid::{LiquidPressureLaw, LiquidPressureOperator};
    let condensed = LiquidPressureLaw {
        rest_density: 1000.,
        sound_speed: 10.,
        heat_capacity_ratio: None,
    };
    let gas = LiquidPressureLaw {
        heat_capacity_ratio: Some(1.4),
        ..condensed
    };
    for (density, expected) in [(500., 0.), (1000., 0.), (1500., 50000.)] {
        assert_eq!(condensed.evaluate_native(density).unwrap(), expected);
        assert_eq!(
            gas.evaluate_native(density).unwrap(),
            density * (100. / 1.4)
        );
    }
    let densities = [500., 1000., 1500.];
    let laws = [condensed; 3];
    let operator = LiquidPressureOperator {
        densities: &densities,
        laws: &laws,
    };
    let exact = operator.evaluate_native().unwrap();
    operator.admit(&exact, 0., 0.).unwrap();
    for bad in [
        vec![0., 0.],
        vec![0., f64::NAN, 50000.],
        vec![0., -1., 50000.],
        vec![0., 0., 1.],
    ] {
        assert!(operator.admit(&bad, 0., 0.).is_err());
    }
    assert!(
        LiquidPressureOperator {
            densities: &[500., 1000., 1600.],
            laws: &laws
        }
        .admit(&exact, 0., 0.)
        .is_err()
    );
}

#[test]
fn complete_pipeline_whole_rejection_and_late_failure_preserve_native_state() {
    use physics::liquid::{
        BoundarySample, Config, Liquid, LiquidComputeBackend, LiquidPipelineOperator,
        LiquidPipelineResult,
    };
    struct Backend {
        mode: usize,
        calls: usize,
    }
    impl LiquidComputeBackend for Backend {
        fn candidate(&mut self, _: &LiquidDensityOperator<'_>) -> Option<Vec<f64>> {
            panic!("complete pipeline bypasses individual stages")
        }
        fn supports_pipeline(&self) -> bool {
            true
        }
        fn pipeline_candidate(
            &mut self,
            owner: &LiquidPipelineOperator<'_>,
        ) -> Option<LiquidPipelineResult> {
            self.calls += 1;
            let mut result = owner.evaluate_native().unwrap();
            match self.mode {
                1 => result.fields.densities[0] = f64::NAN,
                2 => result.fields.normals.last_mut().unwrap()[0] += 1.,
                3 => result.fields.pressures = None,
                4 => result.forces.forces.last_mut().unwrap().force[0] = f64::NAN,
                5 => result.forces.accumulation.accelerations[0][0] += 1.,
                6 => {
                    result.forces.forces.pop();
                }
                7 => result.forces.accumulation.viscous_rates[0] = -1.,
                _ => {}
            }
            Some(result)
        }
    }
    let config = Config {
        gravity: [0.; 3],
        smoothing_radius: 1.,
        ..Config::default()
    };
    for formulation in [
        Formulation::MassDensity,
        Formulation::RestVolume,
        Formulation::RestVolumeWendland,
    ] {
        let particles = (0..3)
            .map(|i| Particle {
                position: [0.12 + i as f64 * 0.04, 0.15, 0.2],
                velocity: [0.01 * i as f64, 0., 0.],
                mass: 0.1,
                material: 0,
            })
            .collect();
        let material = Material {
            rest_density: 1.,
            sound_speed: 1.,
            viscosity: 0.01,
        };
        let mut initial = Liquid::new(particles, vec![material], config).unwrap();
        initial.set_formulation(formulation);
        initial
            .configure_boundaries(vec![BoundarySample {
                position: [-0.1, 0.15, 0.2],
                volume: 0.1,
            }])
            .unwrap();
        let mut native = initial.clone();
        native.step(1e-5, None).unwrap();
        for mode in 0..8 {
            let mut backend = Backend { mode, calls: 0 };
            let mut actual = initial.clone();
            let (_, stats) = actual
                .step_with_compute_backend(1e-5, None, &mut backend, 0., 0.)
                .unwrap();
            assert_eq!(actual, native);
            assert_eq!(stats.pipeline_calls, backend.calls);
            assert!(backend.calls > 0);
            if mode == 0 {
                assert_eq!(stats.pipeline_admitted, stats.pipeline_calls);
                assert_eq!(stats.pipeline_native_fallbacks, 0);
            } else {
                assert_eq!(stats.pipeline_admitted, 0);
                assert_eq!(stats.pipeline_native_fallbacks, stats.pipeline_calls);
            }
            assert_eq!(stats.density_normal_calls, 0);
            assert_eq!(stats.force_pipeline_calls, 0);
        }
        let mut fast = initial.particles().to_vec();
        for p in &mut fast {
            p.velocity = [100., 0., 0.];
        }
        let mut limited = Liquid::new(
            fast,
            vec![material],
            Config {
                max_substeps: 1,
                ..config
            },
        )
        .unwrap();
        limited.set_formulation(formulation);
        let before = limited.clone();
        let mut backend = Backend { mode: 0, calls: 0 };
        assert!(
            limited
                .step_with_compute_backend(0.1, None, &mut backend, 0., 0.)
                .is_err()
        );
        assert!(backend.calls > 0);
        assert_eq!(limited, before);
    }
}

#[test]
fn supported_pipeline_rejects_nonfinite_native_reference_with_loose_candidate_density() {
    use physics::liquid::{
        LiquidDensityNormalOperator, LiquidDensityNormalResult, LiquidPressureLaw,
    };
    let coefficient = 315. / (64. * std::f64::consts::PI);
    let particles = [Particle {
        position: [0.; 3],
        velocity: [0.; 3],
        mass: 2.6 / coefficient,
        material: 0,
    }];
    let properties = [Material {
        rest_density: 1.,
        sound_speed: 1e154,
        viscosity: 0.,
    }];
    let law = LiquidPressureLaw {
        rest_density: 1.,
        sound_speed: 1e154,
        heat_capacity_ratio: Some(1.4),
    };
    let owner = LiquidDensityNormalOperator {
        fluid: LiquidDensityOperator {
            particles: &particles,
            pairs: &[],
            properties: &properties,
            formulation: Formulation::MassDensity,
            smoothing_radius: 1.,
            carrier_droplets: None,
        },
        support: &[],
        pressure_laws: Some(&[law]),
    };
    assert!(owner.evaluate_native().is_err());
    let candidate = LiquidDensityNormalResult {
        densities: vec![1.],
        normals: vec![[0.; 3]],
        pressures: Some(vec![law.evaluate_native(1.).unwrap()]),
    };
    assert!(
        owner.admit(&candidate, 2., 0.).is_err(),
        "a finite candidate cannot repair a nonfinite original reference"
    );
}
