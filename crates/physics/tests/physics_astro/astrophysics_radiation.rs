use physics::astrophysics_radiation::{Layer, blackbody, trace};
#[test]
fn beer_lambert_and_thin_limit() {
    let l = Layer {
        length: 2.0,
        absorption: 0.3,
        temperature: 0.0,
    };
    let r = trace(100.0, &[l], 1).unwrap();
    assert!((r.intensity - 100.0 * (-0.6_f64).exp()).abs() < 1e-12);
    let r = trace(
        100.0,
        &[Layer {
            absorption: 1e-15,
            ..l
        }],
        1,
    )
    .unwrap();
    assert!((r.deposited[0] / 2e-13 - 1.0).abs() < 1e-14);
}
#[test]
fn lte_and_opaque_limit() {
    let source = blackbody(500.0).unwrap();
    let l = Layer {
        length: 1.0,
        absorption: 1000.0,
        temperature: 500.0,
    };
    let r = trace(0.0, &[l], 1).unwrap();
    assert_eq!(r.intensity, source);
    assert_eq!(r.deposited[0], -source);
    let r = trace(source, &[l], 1).unwrap();
    assert_eq!(r.intensity, source);
    assert_eq!(r.deposited[0], 0.0);
}
#[test]
fn layer_splitting_and_energy_accounting() {
    let l = Layer {
        length: 1.0,
        absorption: 0.5,
        temperature: 500.0,
    };
    let one = trace(123.0, &[l], 1).unwrap();
    let split = trace(123.0, &[Layer { length: 0.01, ..l }; 100], 100).unwrap();
    assert!((one.intensity - split.intensity).abs() < 1e-10);
    let r = trace(
        123.0,
        &[
            l,
            Layer {
                temperature: 250.0,
                ..l
            },
            Layer {
                temperature: 800.0,
                ..l
            },
        ],
        3,
    )
    .unwrap();
    assert!((123.0 - r.intensity - r.deposited.iter().sum::<f64>()).abs() < 1e-10);
}
#[test]
fn invalid_layers_and_budget_fail_without_result() {
    let l = Layer {
        length: 1.0,
        absorption: 0.0,
        temperature: 500.0,
    };
    assert_eq!(trace(123.0, &[l], 1).unwrap().intensity, 123.0);
    assert!(trace(123.0, &[l], 0).is_err());
    assert!(
        trace(
            123.0,
            &[Layer {
                absorption: -1.0,
                ..l
            }],
            1
        )
        .is_err()
    );
    assert!(blackbody(f64::MAX).is_err());
    assert!(trace(f64::NAN, &[], 0).is_err());
}

#[test]
fn linear_source_has_correct_thin_and_diffusion_limits() {
    use physics::astrophysics_radiation::{LinearSourceLayer, trace_linear_sources};
    for depth in [0.0, 1e-12, 1e-4, 1.0, 10.0, 100.0] {
        let layer = LinearSourceLayer {
            length: 1.0,
            absorption: depth,
            source_start: 3.0,
            source_end: 1.0,
        };
        let result = trace_linear_sources(3.0, &[layer], 1).unwrap();
        if depth > 0.0 {
            // I_out = S_end + (S_start-S_end)*(1-exp(-tau))/tau.
            let expected = 1.0 + 2.0 * (-(-depth).exp_m1()) / depth;
            assert!((result.intensity - expected).abs() < 1e-12);
        } else {
            assert_eq!(result.intensity, 3.0);
        }
        assert!((result.intensity + result.deposited[0] - 3.0).abs() < 1e-14);
        if depth >= 10.0 {
            let flux = result.intensity - 1.0;
            assert!((flux * depth / 2.0 - 1.0).abs() < 5e-5);
        }
    }
    let layer = LinearSourceLayer {
        length: 2.0,
        absorption: 0.7,
        source_start: 2.0,
        source_end: 8.0,
    };
    let whole = trace_linear_sources(1.0, &[layer], 1).unwrap();
    let split = trace_linear_sources(
        1.0,
        &[
            LinearSourceLayer {
                length: 1.0,
                source_end: 5.0,
                ..layer
            },
            LinearSourceLayer {
                length: 1.0,
                source_start: 5.0,
                ..layer
            },
        ],
        2,
    )
    .unwrap();
    assert!((whole.intensity - split.intensity).abs() < 1e-14);
    assert!(trace_linear_sources(1.0, &[layer], 0).is_err());
}

#[test]
fn linear_source_preserves_sub_ulp_diffusion_offsets() {
    use physics::astrophysics_radiation::{LinearSourceLayer, trace_linear_sources};
    let result = trace_linear_sources(
        2.0,
        &[
            LinearSourceLayer {
                length: 1.0,
                absorption: 1e20,
                source_start: 2.0,
                source_end: 1.0,
            },
            LinearSourceLayer {
                length: 1.0,
                absorption: 1e20,
                source_start: 1.0,
                source_end: 0.5,
            },
        ],
        2,
    )
    .unwrap();
    assert_eq!(result.intensity, 0.5); // Absolute intensity cannot represent the tiny flux.
    assert!((result.incident_source_offsets[1] / 1e-20 - 1.0).abs() < 1e-14);
    assert!((result.end_source_offset / 5e-21 - 1.0).abs() < 1e-14);
}
