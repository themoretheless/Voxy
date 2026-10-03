use super::*;
use glam::{DVec3, Quat, Vec3};
use voxy_scene::Transform;
fn budget() -> SupportQueryBudget {
    SupportQueryBudget::new(1024).unwrap()
}
fn down(origin: DVec3) -> SupportProbe {
    SupportProbe {
        origin,
        direction: -DVec3::Y,
        up: DVec3::Y,
        max_distance: 10.,
        min_up_dot: 0.7,
    }
}
fn platform(scene: &mut SceneGraph, transform: Transform, half: [f32; 3]) -> NodeId {
    let owner = scene.spawn(None, transform).unwrap();
    scene
        .insert_component(owner, BoxCollider { half_extents: half })
        .unwrap();
    owner
}
#[test]
fn nearest_contact_and_equal_distance_ties_use_stable_owner_identity() {
    let mut scene = SceneGraph::new(5);
    let first = platform(&mut scene, Default::default(), [2., 0.1, 2.]);
    let _duplicate = platform(&mut scene, Default::default(), [2., 0.1, 2.]);
    let lower = platform(
        &mut scene,
        Transform {
            translation: Vec3::Y * -2.,
            ..Default::default()
        },
        [4., 0.1, 4.],
    );
    let world = SupportWorld::from_scene(&scene, 3).unwrap();
    let mut budget = budget();
    let contact = world.probe(down(DVec3::Y), &mut budget).unwrap().unwrap();
    assert_eq!(contact.anchor.owner(), first);
    assert!(
        contact
            .position
            .abs_diff_eq(DVec3::Y * f64::from(0.1_f32), 1e-14)
    );
    assert!(contact.normal.abs_diff_eq(DVec3::Y, 1e-14));
    assert!((contact.distance - (1. - f64::from(0.1_f32))).abs() < 1e-14);
    assert_eq!(budget.remaining(), 1021);
    let other = world
        .probe(down(DVec3::new(3., 1., 0.)), &mut budget)
        .unwrap()
        .unwrap();
    assert_eq!(other.anchor.owner(), lower);
    assert_eq!(budget.remaining(), 1018);
}
#[test]
fn sheared_reflected_support_normals_and_local_anchors_follow_parent_transforms() {
    for sy in [-0.5, 0.5] {
        let mut scene = SceneGraph::new(3);
        let parent = scene
            .spawn(
                None,
                Transform {
                    rotation: Quat::from_rotation_z(0.3),
                    scale: Vec3::new(2., 1., 1.),
                    ..Default::default()
                },
            )
            .unwrap();
        let owner = scene
            .spawn(
                Some(parent),
                Transform {
                    translation: Vec3::new(0.2, 0.1, 0.3),
                    rotation: Quat::from_rotation_z(-0.2),
                    scale: Vec3::new(-1., sy, 2.),
                },
            )
            .unwrap();
        scene
            .insert_component(
                owner,
                BoxCollider {
                    half_extents: [2., 0.2, 2.],
                },
            )
            .unwrap();
        let matrix = scene.world_matrix(owner).unwrap();
        let side = sy.signum();
        let local = Vec3::new(0.2, side * 0.2, 0.1);
        // Evaluate the face point in double from independently read affine columns.
        let point = matrix.w_axis.truncate().as_dvec3()
            + matrix.x_axis.truncate().as_dvec3() * f64::from(local.x)
            + matrix.y_axis.truncate().as_dvec3() * f64::from(local.y)
            + matrix.z_axis.truncate().as_dvec3() * f64::from(local.z);
        let expected = matrix
            .z_axis
            .truncate()
            .as_dvec3()
            .cross(matrix.x_axis.truncate().as_dvec3())
            .normalize()
            * f64::from(matrix.determinant().signum() * side);
        let world = SupportWorld::from_scene(&scene, 1).unwrap();
        let contact = world
            .probe(down(point + DVec3::Y), &mut budget())
            .unwrap()
            .unwrap();
        assert!(
            contact.position.abs_diff_eq(point, 1e-12),
            "{contact:?} {point:?}"
        );
        assert!(contact.normal.abs_diff_eq(expected, 1e-12));
        assert!((contact.distance - 1.).abs() < 1e-12);
        let mut changed = scene.local(parent).unwrap();
        changed.translation = Vec3::new(3., 1., -2.);
        changed.rotation = Quat::from_rotation_y(0.4) * changed.rotation;
        scene.set_local(parent, changed).unwrap();
        let next = SupportWorld::from_scene(&scene, 1).unwrap();
        let moved = next
            .resolve(contact.anchor, &mut budget())
            .unwrap()
            .unwrap();
        let matrix = scene.world_matrix(owner).unwrap();
        let expected_point = matrix.w_axis.truncate().as_dvec3()
            + matrix.x_axis.truncate().as_dvec3() * f64::from(local.x)
            + matrix.y_axis.truncate().as_dvec3() * f64::from(local.y)
            + matrix.z_axis.truncate().as_dvec3() * f64::from(local.z);
        assert!(moved.position.abs_diff_eq(expected_point, 1e-12));
        let normal = matrix
            .z_axis
            .truncate()
            .as_dvec3()
            .cross(matrix.x_axis.truncate().as_dvec3())
            .normalize()
            * f64::from(matrix.determinant().signum() * side);
        assert!(moved.normal.abs_diff_eq(normal, 1e-12));
        // An old immutable snapshot stays authoritative for its own query interval.
        assert_eq!(
            world
                .resolve(contact.anchor, &mut budget())
                .unwrap()
                .unwrap()
                .position,
            contact.position
        );
    }
}
#[test]
fn unwalkable_first_hit_occludes_lower_ground_and_inside_origins_do_not_plant() {
    let mut scene = SceneGraph::new(4);
    let slope = platform(
        &mut scene,
        Transform {
            rotation: Quat::from_rotation_z(1.),
            ..Default::default()
        },
        [2., 0.1, 2.],
    );
    platform(
        &mut scene,
        Transform {
            translation: Vec3::Y * -3.,
            ..Default::default()
        },
        [4., 0.1, 4.],
    );
    let world = SupportWorld::from_scene(&scene, 2).unwrap();
    assert!(
        world
            .probe(down(DVec3::Y * 3.), &mut budget())
            .unwrap()
            .is_none()
    );
    let probe = SupportProbe {
        min_up_dot: 0.4,
        ..down(DVec3::Y * 3.)
    };
    assert_eq!(
        world
            .probe(probe, &mut budget())
            .unwrap()
            .unwrap()
            .anchor
            .owner(),
        slope
    );
    assert!(
        world
            .probe(down(DVec3::ZERO), &mut budget())
            .unwrap()
            .is_none()
    );
}
#[test]
fn touching_faces_endpoint_hits_parallel_misses_and_corner_normal_ties_are_defined() {
    let mut scene = SceneGraph::new(2);
    platform(&mut scene, Default::default(), [1.; 3]);
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let touch = world
        .probe(
            SupportProbe {
                max_distance: 0.,
                ..down(DVec3::Y)
            },
            &mut budget(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(touch.distance, 0.);
    assert_eq!(touch.normal, DVec3::Y);
    let exact = world
        .probe(
            SupportProbe {
                max_distance: 1.,
                ..down(DVec3::Y * 2.)
            },
            &mut budget(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(exact.distance, 1.);
    assert!(
        world
            .probe(down(DVec3::new(2., 2., 0.)), &mut budget())
            .unwrap()
            .is_none()
    );
    let corner = world
        .probe(
            SupportProbe {
                origin: DVec3::new(2., 2., 0.),
                direction: DVec3::new(-1., -1., 0.).normalize(),
                ..down(DVec3::Y * 2.)
            },
            &mut budget(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(corner.normal, DVec3::Y);
    assert!(corner.position.abs_diff_eq(DVec3::new(1., 1., 0.), 1e-14));
}
#[test]
fn removal_generation_reuse_activity_dimensions_and_foreign_scenes_release_or_reject_anchors() {
    let mut scene = SceneGraph::new(3);
    let owner = platform(&mut scene, Default::default(), [1.; 3]);
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let contact = world
        .probe(down(DVec3::Y * 2.), &mut budget())
        .unwrap()
        .unwrap();
    scene.set_active(owner, false).unwrap();
    assert!(
        SupportWorld::from_scene(&scene, 1)
            .unwrap()
            .resolve(contact.anchor, &mut budget())
            .unwrap()
            .is_none()
    );
    scene.set_active(owner, true).unwrap();
    scene
        .component_mut::<BoxCollider>(owner)
        .unwrap()
        .unwrap()
        .half_extents[1] = 0.5;
    assert!(
        SupportWorld::from_scene(&scene, 1)
            .unwrap()
            .resolve(contact.anchor, &mut budget())
            .unwrap()
            .is_none()
    );
    scene.remove_subtree(owner).unwrap();
    let recycled = platform(&mut scene, Default::default(), [1.; 3]);
    assert_ne!(recycled, owner);
    assert!(
        SupportWorld::from_scene(&scene, 1)
            .unwrap()
            .resolve(contact.anchor, &mut budget())
            .unwrap()
            .is_none()
    );
    let mut foreign = SceneGraph::new(2);
    platform(&mut foreign, Default::default(), [1.; 3]);
    let mut queries = budget();
    let before = queries.remaining();
    assert_eq!(
        SupportWorld::from_scene(&foreign, 1)
            .unwrap()
            .resolve(contact.anchor, &mut queries)
            .unwrap_err(),
        PhysicsError::InvalidMotion
    );
    assert_eq!(queries.remaining(), before);
}
#[test]
fn invalid_input_capacity_and_budget_failures_do_not_return_partial_ground() {
    let mut scene = SceneGraph::new(3);
    platform(&mut scene, Default::default(), [1.; 3]);
    platform(
        &mut scene,
        Transform {
            translation: Vec3::X * 5.,
            ..Default::default()
        },
        [1.; 3],
    );
    assert_eq!(
        SupportWorld::from_scene(&scene, 1).unwrap_err(),
        PhysicsError::Capacity
    );
    let world = SupportWorld::from_scene(&scene, 2).unwrap();
    let mut limited = SupportQueryBudget::new(1).unwrap();
    assert_eq!(
        world.probe(down(DVec3::Y * 2.), &mut limited).unwrap_err(),
        PhysicsError::SweepBudget
    );
    assert_eq!(limited.remaining(), 0);
    let base = down(DVec3::Y * 2.);
    for invalid in [
        SupportProbe {
            origin: DVec3::splat(f64::NAN),
            ..base
        },
        SupportProbe {
            direction: DVec3::ZERO,
            ..base
        },
        SupportProbe {
            up: DVec3::ZERO,
            ..base
        },
        SupportProbe {
            max_distance: f64::INFINITY,
            ..base
        },
        SupportProbe {
            min_up_dot: 1.1,
            ..base
        },
    ] {
        let mut queries = budget();
        assert_eq!(
            world.probe(invalid, &mut queries).unwrap_err(),
            PhysicsError::InvalidMotion
        );
        assert_eq!(queries.remaining(), 1024);
    }
    let owner = scene.active_components::<BoxCollider>().next().unwrap().0;
    scene
        .component_mut::<BoxCollider>(owner)
        .unwrap()
        .unwrap()
        .half_extents = [0.; 3];
    assert_eq!(
        SupportWorld::from_scene(&scene, 2).unwrap_err(),
        PhysicsError::InvalidBody
    );
}

#[test]
fn shared_collision_snapshot_keeps_geometry_extents_and_scene_identity_from_one_revision() {
    let mut scene = SceneGraph::new(2);
    let owner = platform(&mut scene, Default::default(), [1., 0.1, 1.]);
    let shared = static_world(&scene).unwrap();
    scene
        .component_mut::<BoxCollider>(owner)
        .unwrap()
        .unwrap()
        .half_extents[1] = 0.5;
    let frozen = SupportWorld::from_static_world(&shared).unwrap();
    let old = frozen
        .probe(down(DVec3::Y), &mut budget())
        .unwrap()
        .unwrap();
    assert!((old.position.y - f64::from(0.1_f32)).abs() < 1e-14);
    assert_eq!(
        frozen
            .resolve(old.anchor, &mut budget())
            .unwrap()
            .unwrap()
            .position,
        old.position
    );
    let current = SupportWorld::from_scene(&scene, 1).unwrap();
    assert!(
        current
            .resolve(old.anchor, &mut budget())
            .unwrap()
            .is_none()
    );
    assert_eq!(
        current
            .probe(down(DVec3::Y), &mut budget())
            .unwrap()
            .unwrap()
            .position
            .y,
        0.5
    );
}
#[test]
fn support_queries_reuse_character_physics_dynamic_parent_restrictions() {
    use crate::{CharacterBody, CharacterPhysics};
    let mut scene = SceneGraph::new(3);
    let body = scene.spawn(None, Default::default()).unwrap();
    scene
        .insert_component(body, CharacterBody::default())
        .unwrap();
    let child = scene.spawn(Some(body), Default::default()).unwrap();
    scene
        .insert_component(child, BoxCollider::default())
        .unwrap();
    let expected = CharacterPhysics::new(&scene, 1, 1)
        .validate(&scene)
        .unwrap_err();
    assert_eq!(expected, PhysicsError::UnsupportedDynamicParent);
    assert_eq!(SupportWorld::from_scene(&scene, 1).unwrap_err(), expected);
    scene.remove_subtree(child).unwrap();
    scene
        .insert_component(body, BoxCollider::default())
        .unwrap();
    assert_eq!(
        SupportWorld::from_scene(&scene, 1).unwrap_err(),
        PhysicsError::InvalidBody
    );
}

#[test]
fn extreme_finite_inverse_face_vectors_keep_unit_normals_instead_of_zero_or_nan() {
    for magnitude in [1e-300, 1e300] {
        let normal = normal_direction(DVec3::new(magnitude, magnitude, 0.)).unwrap();
        assert!(normal.abs_diff_eq(
            DVec3::new(
                std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2,
                0.
            ),
            1e-14
        ));
        assert!((normal.length_squared() - 1.).abs() < 1e-14);
    }
    assert_eq!(
        normal_direction(DVec3::ZERO).unwrap_err(),
        PhysicsError::UnsupportedTransform
    );
}

#[test]
fn a_one_ulp_thickness_change_on_a_wide_platform_releases_the_old_face_anchor() {
    let mut scene = SceneGraph::new(2);
    let thickness = 1e-6_f32;
    let owner = platform(&mut scene, Default::default(), [1e4, thickness, 1.]);
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let contact = world.probe(down(DVec3::Y), &mut budget()).unwrap().unwrap();
    scene
        .component_mut::<BoxCollider>(owner)
        .unwrap()
        .unwrap()
        .half_extents[1] = f32::from_bits(thickness.to_bits() + 1);
    assert!(
        SupportWorld::from_scene(&scene, 1)
            .unwrap()
            .resolve(contact.anchor, &mut budget())
            .unwrap()
            .is_none()
    );
}
