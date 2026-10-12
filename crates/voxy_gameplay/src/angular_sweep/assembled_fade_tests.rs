use super::*;
use voxy_animation::{
    RootRigidCertifiedFadeInterval, RootRigidMappedField, RootRigidPath, RootRigidTransform,
    RootRigidTwist,
};
#[test]
fn original_source_fade_assembly_and_collision_query_preserve_floor_and_wall() {
    let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
    let floor = AffineBox {
        center: DVec3::Y * (-0.5),
        edges: [DVec3::X * 4., DVec3::Y * 0.5, DVec3::Z * 4.],
    };
    let wall = AffineBox {
        center: DVec3::X,
        edges: [DVec3::X * 0.125, DVec3::Y * 2., DVec3::Z * 2.],
    };
    for speed in [0.3, 3.] {
        let source = RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::X * speed,
                    angular: DVec3::Y * 0.4,
                },
                1.,
            )],
            1,
        )
        .unwrap();
        let field = RootRigidMappedField::new(
            &source.spans()[0],
            [0., 1.],
            RootRigidTransform::IDENTITY,
            1.,
        )
        .unwrap();
        let fade =
            RootRigidCertifiedFadeInterval::integrate(None, field, [0., 1.], 1., 0.01, 0.01, 4096)
                .unwrap();
        assert!(fade.approximation().origin_error_bound > 0.);
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
            256,
            &mut 16384,
        )
        .unwrap();
        assert!(hit.complete);
        assert_eq!(hit.displacement.y, 0.);
        if speed == 3. {
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
                &[floor, wall],
                256,
                &mut 16384,
            )
            .unwrap();
            assert!(!hit.complete && hit.path_fraction > 0.);
        }
    }
}
