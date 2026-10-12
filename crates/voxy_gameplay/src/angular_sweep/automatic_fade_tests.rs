use super::*;
use voxy_animation::{
    RootRigidCertifiedFadeInterval, RootRigidMappedPath, RootRigidPath, RootRigidTransform,
    RootRigidTwist,
};
#[test]
fn automatically_discovered_key_guard_preserves_floor_during_translating_yaw_fade() {
    let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
    let floor = AffineBox {
        center: DVec3::Y * (-0.5),
        edges: [DVec3::X * 4., DVec3::Y * 0.5, DVec3::Z * 4.],
    };
    let target = RootRigidPath::from_twists(
        &[
            (
                RootRigidTwist {
                    linear: DVec3::X * 0.3,
                    angular: DVec3::Y * 0.4,
                },
                1.,
            ),
            (
                RootRigidTwist {
                    linear: DVec3::X * 0.6,
                    angular: DVec3::Y * 0.2,
                },
                2.,
            ),
        ],
        2,
    )
    .unwrap();
    let target = RootRigidMappedPath::new(&target, RootRigidTransform::IDENTITY, 1.).unwrap();
    let fade = RootRigidCertifiedFadeInterval::integrate_paths(
        None,
        target,
        [0., 1.],
        1.,
        0.01,
        0.01,
        4096,
    )
    .unwrap();
    let hit = sweep_certified_rigid_fade(
        DVec3::Y * 0.125,
        edges,
        DQuat::IDENTITY,
        &fade,
        1,
        DQuat::IDENTITY,
        DVec3::ZERO,
        1.,
        0.,
        &[floor],
        512,
        &mut 16384,
    )
    .unwrap();
    assert!(hit.complete);
    assert_eq!(hit.displacement.y, 0.);
    assert!(hit.displacement.x > 0.);
}
