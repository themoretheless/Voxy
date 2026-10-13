use super::*;
use voxy_animation::{
    RootRigidCertifiedFadeInterval, RootRigidMappedPath, RootRigidPath, RootRigidTransform,
    RootRigidTwist,
};
#[test]
fn completion_keeps_floor_contact_and_wall_stops_motion_after_fade() {
    let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
    let floor = AffineBox {
        center: DVec3::Y * (-0.5),
        edges: [DVec3::X * 8., DVec3::Y * 0.5, DVec3::Z * 8.],
    };
    let wall = AffineBox {
        center: DVec3::X,
        edges: [DVec3::X * 0.125, DVec3::Y * 2., DVec3::Z * 8.],
    };
    let target = RootRigidPath::from_twists(
        &[(
            RootRigidTwist {
                linear: DVec3::X * 0.1,
                angular: DVec3::Y * 0.1,
            },
            0.25,
        )],
        1,
    )
    .unwrap();
    let tail = RootRigidPath::from_twists(
        &[
            (
                RootRigidTwist {
                    linear: DVec3::X * 0.5,
                    angular: DVec3::Y * 0.1,
                },
                1.,
            ),
            (
                RootRigidTwist {
                    linear: DVec3::X,
                    angular: DVec3::Y * 0.1,
                },
                2.,
            ),
        ],
        2,
    )
    .unwrap();
    let mapped = RootRigidMappedPath::new(&target, RootRigidTransform::IDENTITY, 1.).unwrap();
    let completion = RootRigidMappedPath::from_enclosed_frame(
        &tail,
        target.continuous_end_enclosure(1).unwrap(),
        1.,
    )
    .unwrap();
    let tick = RootRigidCertifiedFadeInterval::integrate_paths_with_completion(
        None,
        mapped,
        [0., 1.],
        0.25,
        Some((completion, 1.)),
        0.02,
        0.01,
        4096,
    )
    .unwrap();
    let query = |boxes: &[AffineBox], evaluation| {
        sweep_certified_rigid_fade(
            DVec3::Y * 0.125,
            edges,
            DQuat::IDENTITY,
            &tick,
            1,
            DQuat::IDENTITY,
            DVec3::ZERO,
            1.,
            evaluation,
            boxes,
            8192,
            &mut 65536,
        )
        .unwrap()
    };
    let free = query(&[floor], 0.);
    assert!(free.complete);
    assert_eq!(free.displacement.y, 0.);
    assert!(free.displacement.x > 1.);
    let blocked = query(&[floor, wall], 0.);
    assert!(!blocked.complete);
    assert!(blocked.path_fraction > 0.25 && blocked.path_fraction < 1.);
    assert!(blocked.displacement.x < 0.75);
    assert_eq!(blocked.displacement.y, 0.);
    assert!(!query(&[floor], 0.001).complete);
    let certificate = tick.coordinate_certificate(1, 4096).unwrap().unwrap();
    let axis_query = |axes| {
        sweep_rigid_approximation_with_axis_errors(
            DVec3::Y * 0.125,
            edges,
            DQuat::IDENTITY,
            tick.approximation(),
            DQuat::IDENTITY,
            DVec3::ZERO,
            1.,
            0.002,
            axes,
            &[floor],
            8192,
            &mut 65536,
            Some(&certificate),
        )
    };
    assert!(axis_query([0.001, 0., 0.001]).unwrap().complete);
    assert!(!axis_query([0.001, 0.001, 0.]).unwrap().complete);
    assert!(axis_query([0., -0.001, 0.]).is_err());
    assert!(axis_query([0., f64::NAN, 0.]).is_err());
    assert!(axis_query([0.003, 0., 0.]).is_err());
}
