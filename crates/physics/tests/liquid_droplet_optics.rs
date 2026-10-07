use physics::liquid::*;
fn fixture() -> (Liquid, FiniteDropletGasGrid) {
    let particles = [0.25, 1.25, 0.75, 3.].map(|x| Particle {
        position: [x, 0.5, 0.5],
        velocity: [0.; 3],
        mass: 1.,
        material: 0,
    });
    let mut liquid =
        Liquid::new(particles.to_vec(), vec![Material::WATER], Config::default()).unwrap();
    liquid
        .configure_droplet_population(Some(vec![true, true, false, true]))
        .unwrap();
    let gas = FiniteDropletGasGrid::new(
        [0.; 3],
        [1.; 3],
        [2, 1, 1],
        vec![
            VaporCell {
                mass: 0.1,
                volume: 1.,
                temperature: 300.,
                velocity: [0.; 3],
                specific_heat_cv: 718.
            };
            2
        ],
    )
    .unwrap();
    (liquid, gas)
}
#[test]
fn physical_cross_sections_produce_analytic_optical_depth_and_transmission() {
    let (liquid, gas) = fixture();
    let before = format!("{liquid:?}");
    let old_gas = gas.clone();
    let field = liquid
        .droplet_extinction_grid(&gas, &[0.1, 0.2, 0.5, 0.1], 2.)
        .unwrap();
    assert_eq!(field.included_droplets, 2);
    assert_eq!(field.outside_droplets, 1);
    let expected = 2. * std::f64::consts::PI * (0.1_f64.powi(2) + 0.2_f64.powi(2));
    for (a, b) in [
        ([-1., 0.5, 0.5], [3., 0.5, 0.5]),
        ([3., 0.5, 0.5], [-1., 0.5, 0.5]),
    ] {
        assert!((field.optical_depth_segment(a, b).unwrap() - expected).abs() < 1e-14);
        assert!((field.transmittance_segment(a, b).unwrap() - (-expected).exp()).abs() < 1e-14);
    }
    assert_eq!(
        field
            .optical_depth_segment([0., 2., 0.], [2., 2., 0.])
            .unwrap(),
        0.
    );
    assert_eq!(field.optical_depth_segment([0.5; 3], [0.5; 3]).unwrap(), 0.);
    assert_eq!(
        field
            .optical_depth_segment([0., 1., 0.5], [2., 1., 0.5])
            .unwrap(),
        0.
    );
    let along_shared_face = field
        .optical_depth_segment([1., 0., 0.5], [1., 1., 0.5])
        .unwrap();
    assert!((along_shared_face - 2. * std::f64::consts::PI * 0.2_f64.powi(2)).abs() < 1e-14);
    let diagonal = field
        .optical_depth_segment([0., 0., 0.5], [2., 1., 0.5])
        .unwrap();
    assert!((diagonal - expected * 5_f64.sqrt() / 2.).abs() < 1e-14);
    assert_eq!(format!("{liquid:?}"), before);
    assert_eq!(gas, old_gas);
}
#[test]
fn invalid_optical_inputs_reject_and_transparent_efficiency_remains_transparent() {
    let (liquid, gas) = fixture();
    for q in [-1., f64::NAN, f64::INFINITY] {
        assert!(liquid.droplet_extinction_grid(&gas, &[0.1; 4], q).is_err());
    }
    assert!(liquid.droplet_extinction_grid(&gas, &[0.1; 3], 2.).is_err());
    assert!(
        liquid
            .droplet_extinction_grid(&gas, &[f64::MAX; 4], 2.)
            .is_err()
    );
    let field = liquid.droplet_extinction_grid(&gas, &[0.1; 4], 0.).unwrap();
    assert_eq!(
        field
            .transmittance_segment([-1., 0.5, 0.5], [3., 0.5, 0.5])
            .unwrap(),
        1.
    );
    assert!(
        field
            .optical_depth_segment([f64::NAN, 0., 0.], [0.; 3])
            .is_err()
    );
}

#[test]
fn single_scattering_converges_to_independent_slab_integrals_and_hg_moments() {
    let (liquid, gas) = fixture();
    // Prescribed identical cross sections give a uniform two-metre slab.
    let field = liquid.droplet_extinction_grid(&gas, &[0.2; 4], 2.).unwrap();
    let before = field.clone();
    let start = [-1., 0.5, 0.5];
    let end = [3., 0.5, 0.5];
    let tau = 4. * std::f64::consts::PI * 0.2_f64.powi(2);
    for direction in [[-1., 0., 0.], [1., 0., 0.]] {
        let light = DirectionalScatteringLight {
            direction_to_light: direction,
            irradiance_rgb: [4., 2., 1.],
            single_scattering_albedo: 0.8,
            asymmetry: 0.,
        };
        let integral = if direction[0] < 0. {
            0.5 * (1. - (-2. * tau).exp())
        } else {
            tau * (-tau).exp()
        };
        let expected = light
            .irradiance_rgb
            .map(|v| v * 0.8 / (4. * std::f64::consts::PI) * integral);

        for samples in [4, 8, 16, 32, 64, 128, 256] {
            let actual = field
                .directional_scattered_radiance_segment(start, end, light, samples, 10000)
                .unwrap();
            let error = (actual[0] - expected[0]).abs();
            println!(
                "SINGLE SCATTERING REFINEMENT direction={} samples={samples} error={error:.17e}",
                direction[0]
            );
            assert!(error < 1e-13);
            assert!(actual.iter().all(|v| v.is_finite() && *v >= 0.));
        }

        assert_eq!(
            field
                .directional_scattered_radiance_segment([0.; 3], [0.; 3], light, 8, 100)
                .unwrap(),
            [0.; 3]
        );
        assert_eq!(
            field
                .directional_scattered_radiance_segment(
                    [-1., 2., 0.5],
                    [3., 2., 0.5],
                    light,
                    8,
                    100
                )
                .unwrap(),
            [0.; 3]
        );
        assert_eq!(
            field
                .directional_scattered_radiance_segment(
                    start,
                    end,
                    DirectionalScatteringLight {
                        single_scattering_albedo: 0.,
                        ..light
                    },
                    64,
                    1000
                )
                .unwrap(),
            [0.; 3]
        );
        assert!(
            field
                .directional_scattered_radiance_segment(start, end, light, 64, 255)
                .is_err()
        );
    }
    for g in [-0.5, 0., 0.5] {
        let light = DirectionalScatteringLight {
            direction_to_light: [1., 0., 0.],
            irradiance_rgb: [1.; 3],
            single_scattering_albedo: 1.,
            asymmetry: g,
        };
        let n = 16384;
        let mut mass = 0.;
        let mut moment = 0.;
        for i in 0..n {
            let cosine = -1. + 2. * (i as f64 + 0.5) / n as f64;
            let weight = light.phase(cosine).unwrap() * 4. * std::f64::consts::PI / n as f64;
            mass += weight;
            moment += weight * cosine;
        }
        assert!((mass - 1.).abs() < 1e-7);
        assert!((moment - g).abs() < 1e-7);
    }
    assert_eq!(field, before);
}

#[test]
fn scattering_calibration_rejects_invalid_inputs_without_mutating_the_field() {
    let (liquid, gas) = fixture();
    let field = liquid.droplet_extinction_grid(&gas, &[0.2; 4], 2.).unwrap();
    let before = field.clone();
    let light = DirectionalScatteringLight {
        direction_to_light: [1., 0., 0.],
        irradiance_rgb: [1.; 3],
        single_scattering_albedo: 0.8,
        asymmetry: 0.3,
    };
    for bad in [
        DirectionalScatteringLight {
            direction_to_light: [0.; 3],
            ..light
        },
        DirectionalScatteringLight {
            irradiance_rgb: [-1.; 3],
            ..light
        },
        DirectionalScatteringLight {
            irradiance_rgb: [f64::NAN; 3],
            ..light
        },
        DirectionalScatteringLight {
            single_scattering_albedo: 1.1,
            ..light
        },
        DirectionalScatteringLight {
            asymmetry: -1.,
            ..light
        },
        DirectionalScatteringLight {
            asymmetry: 1.,
            ..light
        },
    ] {
        assert!(
            field
                .directional_scattered_radiance_segment(
                    [-1., 0.5, 0.5],
                    [3., 0.5, 0.5],
                    bad,
                    32,
                    1000
                )
                .is_err()
        );
    }
    let diagonal_light = DirectionalScatteringLight {
        direction_to_light: [1., 1., 0.],
        ..light
    };
    let diagonal = field
        .directional_scattered_radiance_segment(
            [-1., -1., 0.5],
            [2., 2., 0.5],
            diagonal_light,
            32,
            1000,
        )
        .unwrap();
    for magnitude in [f64::MAX, f64::from_bits(1)] {
        let scaled = DirectionalScatteringLight {
            direction_to_light: [magnitude, magnitude, 0.],
            ..diagonal_light
        };
        assert_eq!(
            field
                .directional_scattered_radiance_segment(
                    [-1., -1., 0.5],
                    [2., 2., 0.5],
                    scaled,
                    32,
                    1000
                )
                .unwrap(),
            diagonal
        );
    }
    let transparent = liquid.droplet_extinction_grid(&gas, &[0.2; 4], 0.).unwrap();
    let intense = DirectionalScatteringLight {
        irradiance_rgb: [f64::MAX; 3],
        asymmetry: 0.9,
        single_scattering_albedo: 1.,
        ..light
    };
    assert_eq!(
        transparent
            .directional_scattered_radiance_segment(
                [-1., 0.5, 0.5],
                [3., 0.5, 0.5],
                intense,
                32,
                1000
            )
            .unwrap(),
        [0.; 3]
    );
    assert!(light.phase(1.1).is_err());
    assert_eq!(field, before);
}

#[test]
fn endpoint_transport_matches_dense_slabs_and_refines_heterogeneous_shadows() {
    let (liquid, gas) = fixture();
    let light = DirectionalScatteringLight {
        direction_to_light: [-1., 0., 0.],
        irradiance_rgb: [4., 2., 1.],
        single_scattering_albedo: 0.8,
        asymmetry: 0.,
    };
    for sigma in [0.5, 10., 100., 10000.] {
        let field = liquid
            .droplet_extinction_grid(&gas, &[(sigma / (2. * std::f64::consts::PI)).sqrt(); 4], 2.)
            .unwrap();
        let tau = 2. * sigma;
        for direction in [-1., 1.] {
            let source = DirectionalScatteringLight {
                direction_to_light: [direction, 0., 0.],
                ..light
            };
            let integral = if direction < 0. {
                0.5 * (1. - (-2. * tau).exp())
            } else {
                tau * (-tau).exp()
            };
            let expected = 4. * 0.8 / (4. * std::f64::consts::PI) * integral;
            for samples in [1, 2, 8, 32, 128] {
                let value = field
                    .directional_scattered_radiance_segment(
                        [-1., 0.5, 0.5],
                        [3., 0.5, 0.5],
                        source,
                        samples,
                        10000,
                    )
                    .unwrap()[0];
                assert!(
                    (value - expected).abs() < 1e-12,
                    "sigma={sigma} direction={direction} samples={samples} actual={value} expected={expected}"
                );
            }
        }
    }
    let field = liquid
        .droplet_extinction_grid(&gas, &[0.1, 0.2, 0.1, 0.1], 2.)
        .unwrap();
    let source = DirectionalScatteringLight {
        direction_to_light: [-1., 0.7, 0.3],
        ..light
    };
    let start = [-1., 0.4, 0.3];
    let end = [3., 0.7, 0.7];
    let reference = field
        .directional_scattered_radiance_segment(start, end, source, 4096, 25000)
        .unwrap()[0];
    let mut first = 0.;
    let mut last = 0.;
    for samples in [4, 8, 16, 32, 64, 128] {
        let actual = field
            .directional_scattered_radiance_segment(start, end, source, samples, 25000)
            .unwrap()[0];
        let error = (actual - reference).abs();
        if samples == 4 {
            first = error;
        }
        last = error;
        println!("ENDPOINT SCATTERING HETEROGENEOUS samples={samples} error={error:.17e}");
    }
    assert!(last < first / 20. && last < 1e-5);
}
