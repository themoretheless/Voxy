use super::*;

#[test]
fn stationary_coordinate_survives_actual_cache_reframe_pivot_and_body_update() {
    let actors = [
        DQuat::IDENTITY,
        DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5),
        DQuat::from_xyzw(0.123, 0.123, -0.7, 0.7).normalize(),
        DQuat::from_xyzw(0.5, -0.5, 0.5, 0.5),
    ];
    let bases = [
        DQuat::IDENTITY,
        DQuat::from_xyzw(0.5, 0.5, 0.5, 0.5),
        DQuat::from_rotation_y(0.37),
        DQuat::from_xyzw(0.123, 0., 0.7, 0.).normalize(),
    ];
    let origin = DVec3::new(0.123, -0.731, 0.417);
    let edges = [DVec3::X * 0.125, DVec3::Y * 0.25, DVec3::Z * 0.5];
    let mut cases = 0;
    for orientation in actors {
        for basis in bases {
            for world_axis in 0..3 {
                let Some((actor_axis, actor_sign)) =
                    rotation_coordinate_preimage(orientation, world_axis)
                else {
                    continue;
                };
                let Some((source_axis, _)) = rotation_coordinate_preimage(basis, actor_axis) else {
                    continue;
                };
                let mut angular = DVec3::ZERO;
                angular[source_axis] = 0.7;
                let mut linear = DVec3::new(0.3, -0.2, 0.1);
                linear[source_axis] = 0.;
                let path = voxy_animation::RootRigidPath::from_twists(
                    &[(voxy_animation::RootRigidTwist { linear, angular }, 0.5)],
                    1,
                )
                .unwrap();
                let cache = path.prepare_screw_enclosures(1).unwrap();
                if !exact_stationary_physical_projection(&cache, orientation, basis, world_axis) {
                    continue;
                }
                for fraction in [0., 0.1, 0.3, 0.5, 0.9, 1.] {
                    let pose = cache.sample_evaluated(0, fraction).unwrap().0;
                    let rotation = reframe_rotation(basis, pose.rotation);
                    let body_rotation = (orientation * rotation).normalize();
                    assert_eq!(
                        rotation_coordinate_preimage(body_rotation, world_axis),
                        Some((actor_axis, actor_sign))
                    );
                    for scale in [-2., 1.] {
                        let displacement = rotate_vector(
                            orientation,
                            scale * rotate_vector(basis, pose.translation) + origin
                                - rotate_vector(rotation, origin),
                        );
                        assert_eq!(displacement[world_axis], 0.);
                        for edge in edges {
                            assert_eq!(
                                rotate_vector(body_rotation, edge)[world_axis],
                                rotate_vector(orientation, edge)[world_axis]
                            );
                        }
                    }
                }
                cases += 1;
            }
        }
    }
    assert!(cases >= 20, "only {cases} admitted frame cases");
}

#[test]
fn zero_requested_allowance_still_reserves_derived_body_error_before_wall() {
    let approximation = voxy_animation::RootRigidApproximation {
        path: voxy_animation::RootRigidPath::from_twists(
            &[(
                voxy_animation::RootRigidTwist {
                    linear: DVec3::X * 0.25,
                    angular: DVec3::ZERO,
                },
                1.,
            )],
            1,
        )
        .unwrap(),
        origin_error_bound: 0.,
        angular_error_bound: 0.,
    };
    let center = DVec3::ZERO;
    let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
    let cache = approximation.path.prepare_screw_enclosures(1).unwrap();
    let (_, radius) = cache
        .physical_body_selection_error_bounds(
            center,
            edges,
            DQuat::IDENTITY,
            DVec3::ZERO,
            DQuat::IDENTITY,
            1.,
        )
        .unwrap();
    assert!(radius > 0. && radius < 1e-6);
    let wall = AffineBox {
        center: DVec3::X * (0.5 + radius * 0.25),
        edges,
    };
    assert!(wall.center.x > 0.5);
    let run = |allowance, queries: &mut usize| {
        sweep_rigid_approximation(
            center,
            edges,
            DQuat::IDENTITY,
            &approximation,
            DQuat::IDENTITY,
            DVec3::ZERO,
            1.,
            allowance,
            &[wall],
            256,
            queries,
        )
    };
    let vertices = rigid_vertex_enclosures(edges, DQuat::IDENTITY, DVec3::ZERO, 1.).unwrap();
    let speeds = cache.point_speed_bounds(&vertices, 1.).unwrap();
    let control = sweep_rigid_path_with_bounds(
        center,
        edges,
        DQuat::IDENTITY,
        &approximation.path,
        DQuat::IDENTITY,
        DVec3::ZERO,
        1.,
        0.,
        &[wall],
        256,
        &mut 4096,
        Some(&speeds),
        Some(&cache),
        None,
        [false; 3],
    )
    .unwrap();
    assert!(
        control.complete,
        "zero-clearance control must reach the endpoint"
    );
    let hit = run(0., &mut 4096).unwrap();
    assert!(!hit.complete);
    assert!(hit.displacement.x < 0.25 && hit.displacement.x > 0.249);
    println!(
        "AUTOMATIC_PHYSICAL_MARGIN {:?}",
        (radius, wall.center.x, hit.displacement.x)
    );
    for invalid in [f64::NAN, f64::INFINITY, -1.] {
        assert!(matches!(
            run(invalid, &mut 4096),
            Err(PhysicsError::InvalidMotion)
        ));
    }
    assert_eq!(add_evaluation_allowances(0., 0.).unwrap(), 0.);
    assert!(add_evaluation_allowances(0.25, 0.5).unwrap() >= 0.75);
    assert!(add_evaluation_allowances(f64::MAX, f64::MAX).is_err());
}

#[test]
fn moving_projection_and_unproved_basis_do_not_get_zero_error() {
    let path = voxy_animation::RootRigidPath::from_twists(
        &[(
            voxy_animation::RootRigidTwist {
                linear: DVec3::Y * 0.01,
                angular: DVec3::Y * 0.7,
            },
            0.5,
        )],
        1,
    )
    .unwrap();
    let cache = path.prepare_screw_enclosures(1).unwrap();
    assert!(!exact_stationary_physical_projection(
        &cache,
        DQuat::IDENTITY,
        DQuat::IDENTITY,
        1
    ));
    assert!(!exact_stationary_physical_projection(
        &cache,
        DQuat::from_rotation_x(0.37),
        DQuat::IDENTITY,
        1
    ));
}
