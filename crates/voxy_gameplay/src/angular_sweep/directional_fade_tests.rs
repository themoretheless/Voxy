use super::*;
use voxy_animation::{
    RootRigidApproximation, RootRigidFadeFieldInterval, RootRigidFieldInterval, RootRigidPath,
    RootRigidTransform, RootRigidTwist,
};
#[test]
fn directional_fade_proof_admits_floor_contact_but_preserves_wall_and_evaluation_margin() {
    let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
    let floor = AffineBox {
        center: DVec3::Y * (-0.5),
        edges: [DVec3::X * 4., DVec3::Y * 0.5, DVec3::Z * 4.],
    };
    let wall = AffineBox {
        center: DVec3::X,
        edges: [DVec3::X * 0.125, DVec3::Y * 2., DVec3::Z * 2.],
    };
    let path = RootRigidPath::from_twists(
        &[(
            RootRigidTwist {
                linear: DVec3::X * 1.2,
                angular: DVec3::ZERO,
            },
            1.,
        )],
        1,
    )
    .unwrap();
    let approximation = RootRigidApproximation {
        path,
        origin_error_bound: 0.02,
        angular_error_bound: 0.1,
    };
    let span = &approximation.path.spans()[0];
    let source = RootRigidFieldInterval::from_span(
        span,
        [0., 1.],
        [0., 1.],
        RootRigidTransform::IDENTITY,
        1.,
    )
    .unwrap()
    .unwrap();
    let field = RootRigidFadeFieldInterval::new(source, source, [0., 1.], [0., 1.]).unwrap();
    let proof = approximation
        .path
        .enclose_fade_coordinate_error(&[field], 1, 1)
        .unwrap()
        .unwrap();
    assert_eq!(proof.error_bound(), 0.);
    let query = |boxes: &[AffineBox], evaluation, certificate| {
        sweep_rigid_approximation_with_coordinate_certificate(
            DVec3::Y * 0.125,
            edges,
            DQuat::IDENTITY,
            &approximation,
            DQuat::IDENTITY,
            DVec3::ZERO,
            1.,
            evaluation,
            boxes,
            256,
            &mut 4096,
            certificate,
        )
    };
    assert!(query(&[floor], 0., Some(&proof)).unwrap().complete);
    let wall_hit = query(&[floor, wall], 0., Some(&proof)).unwrap();
    assert!(
        !wall_hit.complete && wall_hit.path_fraction > 0. && wall_hit.path_fraction < 0.75 / 1.2
    );
    assert!(!query(&[floor], 0., None).unwrap().complete);
    assert!(!query(&[floor], 0.001, Some(&proof)).unwrap().complete);
    let other = approximation.path.clone();
    let wrong = other
        .enclose_fade_coordinate_error(&[field], 1, 1)
        .unwrap()
        .unwrap();
    assert!(query(&[floor], 0., Some(&wrong)).is_err());
}
