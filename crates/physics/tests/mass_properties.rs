use physics::mass_properties::{UniformBoxMass, from_boxes};
fn part(mass: f64, center: [f64; 3], half: [f64; 3]) -> UniformBoxMass {
    UniformBoxMass {
        mass,
        center,
        half_edges: [[half[0], 0., 0.], [0., half[1], 0.], [0., 0., half[2]]],
    }
}
#[test]
fn homogeneous_cuboid_matches_analytic_si_inertia() {
    let p = from_boxes(&[part(3., [1., 2., 3.], [1., 2., 3.])], 1).unwrap();
    assert_eq!(p.mass, 3.);
    assert_eq!(p.center, [1., 2., 3.]);
    for (i, expected) in [13., 10., 5.].into_iter().enumerate() {
        assert!((p.inertia[i][i] - expected).abs() < 1e-12);
    }
    assert!((p.principal_moments[0] - 13.).abs() < 1e-12);
}
#[test]
fn additive_offsets_obey_parallel_axis_and_large_translation_invariance() {
    for origin in [0., 1e15] {
        let p = from_boxes(
            &[
                part(2., [origin, 0., 0.], [0.1; 3]),
                part(1., [origin + 3., 0., 0.], [0.1; 3]),
            ],
            2,
        )
        .unwrap();
        assert_eq!(p.center, [origin + 1., 0., 0.]);
        assert!((p.inertia[0][0] - 0.02).abs() < 1e-12);
        assert!((p.inertia[1][1] - 6.02).abs() < 1e-12);
        assert!((p.inertia[2][2] - 6.02).abs() < 1e-12);
    }
}
#[test]
fn sheared_volume_has_full_tensor_and_proper_principal_frame() {
    let p = from_boxes(
        &[UniformBoxMass {
            mass: 3.,
            center: [0.; 3],
            half_edges: [[1., 0., 0.], [0.5, 2., 0.], [0., 0., 3.]],
        }],
        1,
    )
    .unwrap();
    let expected = [[13., -1., 0.], [-1., 10.25, 0.], [0., 0., 5.25]];
    for i in 0..3 {
        for j in 0..3 {
            assert!((p.inertia[i][j] - expected[i][j]).abs() < 1e-12);
        }
    }
    let a = p.principal_axes;
    let determinant = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
    assert!((determinant - 1.).abs() < 1e-12);
    for i in 0..3 {
        for j in 0..3 {
            let reconstructed: f64 = (0..3)
                .map(|k| a[i][k] * p.principal_moments[k] * a[j][k])
                .sum();
            assert!((reconstructed - expected[i][j]).abs() < 1e-12);
        }
    }
}
#[test]
fn explicit_overlapping_constituents_are_additive_and_extreme_scales_stay_representable() {
    let one = part(1., [0.; 3], [1.; 3]);
    let p = from_boxes(&[one, one], 2).unwrap();
    assert_eq!(p.mass, 2.);
    assert!((p.inertia[0][0] - 4. / 3.).abs() < 1e-12);
    let p = from_boxes(&[part(1e-300, [0.; 3], [1e200; 3])], 1).unwrap();
    assert!((p.inertia[0][0] / (2e100 / 3.) - 1.).abs() < 1e-12);
}
#[test]
fn invalid_mass_rank_overflow_and_budget_reject_without_fallback() {
    assert!(from_boxes(&[], 1).is_err());
    assert!(from_boxes(&[part(1., [0.; 3], [1.; 3])], 0).is_err());
    assert!(from_boxes(&[part(-1., [0.; 3], [1.; 3])], 1).is_err());
    assert!(from_boxes(&[part(1., [0.; 3], [0., 1., 1.])], 1).is_err());
    assert!(from_boxes(&[part(1e300, [0.; 3], [1e200; 3])], 1).is_err());
    assert!(from_boxes(&[part(1., [f64::NAN; 3], [1.; 3])], 1).is_err());
}
