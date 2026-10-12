use super::*;
use voxy_animation::{
    RootRigidApproximation, RootRigidFadeFieldInterval, RootRigidFieldInterval, RootRigidPath,
    RootRigidTransform, RootRigidTwist,
};
#[test]
fn positive_directional_error_requires_a_proven_initial_gap_including_numeric_radius() {
    let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
    let floor = AffineBox {
        center: DVec3::Y * (-0.5),
        edges: [DVec3::X * 4., DVec3::Y * 0.5, DVec3::Z * 4.],
    };
    let nominal = RootRigidTwist {
        linear: DVec3::X * 0.3,
        angular: DVec3::ZERO,
    };
    let path = RootRigidPath::from_twists(&[(nominal, 1.)], 1).unwrap();
    let source_path = RootRigidPath::from_twists(
        &[(
            RootRigidTwist {
                linear: nominal.linear - DVec3::Y * 0.002,
                ..nominal
            },
            1.,
        )],
        1,
    )
    .unwrap();
    let source = RootRigidFieldInterval::from_span(
        &source_path.spans()[0],
        [0., 1.],
        [0., 1.],
        RootRigidTransform::IDENTITY,
        1.,
    )
    .unwrap()
    .unwrap();
    let field = RootRigidFadeFieldInterval::new(source, source, [0., 1.], [0., 1.]).unwrap();
    let approximation = RootRigidApproximation {
        path,
        origin_error_bound: 0.05,
        angular_error_bound: 0.1,
    };
    let proof = approximation
        .path
        .enclose_fade_coordinate_error(&[field], 1, 1)
        .unwrap()
        .unwrap();
    assert!(proof.error_bound() >= 0.002);
    for scale in [-2., 1.] {
        let query = |height, certificate| {
            sweep_rigid_approximation_with_coordinate_certificate(
                DVec3::Y * (0.125 + height),
                edges,
                DQuat::IDENTITY,
                &approximation,
                DQuat::IDENTITY,
                DVec3::ZERO,
                scale,
                0.001,
                &[floor],
                256,
                &mut 4096,
                certificate,
            )
            .unwrap()
        };
        assert!(query(0.01, Some(&proof)).complete);
        assert!(!query(0.001, Some(&proof)).complete);
        assert!(!query(0.01, None).complete);
        assert!(
            proof.enclosed_scaled_error_bound(scale, 0.001).unwrap() >= 0.002 * scale.abs() + 0.001
        );
    }
    assert!(proof.enclosed_scaled_error_bound(1., -0.001).is_err());
}
