use super::*;
use crate::BoxCollider;
use glam::{Quat, Vec3};
use voxy_scene::{SceneGraph, Transform};
fn setup() -> (SceneGraph, voxy_scene::NodeId, FootContactInput) {
    let mut scene = SceneGraph::new(4);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(
            owner,
            BoxCollider {
                half_extents: [2., 0.1, 2.],
            },
        )
        .unwrap();
    (
        scene,
        owner,
        FootContactInput {
            sole: DVec3::new(0.3, f64::from(0.1_f32), 0.2),
            up: DVec3::Y,
            grounded: true,
            plant: true,
        },
    )
}
fn budget() -> SupportQueryBudget {
    SupportQueryBudget::new(1024).unwrap()
}
#[test]
fn contact_holds_surface_point_through_animation_drift_and_platform_transform() {
    let (mut scene, owner, mut input) = setup();
    let settings = FootContactSettings::default();
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let planted = FootContactState::default()
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert_eq!(planted.status, FootContactStatus::Planted);
    let point = planted.contact.unwrap().position;
    input.sole.x += 0.2;
    let held = planted
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert_eq!(held.contact.unwrap().position, point);
    let moved = Transform {
        translation: Vec3::new(0.1, 0.02, 0.),
        rotation: Quat::from_rotation_y(0.2),
        ..Default::default()
    };
    scene.set_local(owner, moved).unwrap();
    let matrix = scene.world_matrix(owner).unwrap().as_dmat4();
    let expected = matrix.transform_point3(point);
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let held = held
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert!(held.contact.unwrap().position.abs_diff_eq(expected, 1e-14));
    assert_eq!(held.state.anchor(), planted.state.anchor());
}
#[test]
fn release_hysteresis_prevents_replanting_until_swing_and_landing_rearms() {
    let (scene, _, mut input) = setup();
    let settings = FootContactSettings::default();
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let state = FootContactState::default()
        .prepare(&world, settings, input, &mut budget())
        .unwrap()
        .state;
    input.sole.x += 0.36;
    let released = state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert_eq!(released.status, FootContactStatus::Released);
    input.sole.x -= 0.36;
    let blocked = released
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert_eq!(blocked.status, FootContactStatus::AwaitingSwing);
    input.plant = false;
    let swing = blocked
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    input.plant = true;
    let replanted = swing
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert_eq!(replanted.status, FootContactStatus::Planted);
    input.grounded = false;
    let airborne = replanted
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert!(airborne.contact.is_none());
    input.grounded = true;
    assert_eq!(
        airborne
            .state
            .prepare(&world, settings, input, &mut budget())
            .unwrap()
            .status,
        FootContactStatus::Planted
    );
}
#[test]
fn missing_or_steep_support_releases_instead_of_transferring_a_planted_foot() {
    let (mut scene, owner, input) = setup();
    let settings = FootContactSettings {
        release_distance: 10.,
        ..Default::default()
    };
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    let state = FootContactState::default()
        .prepare(&world, settings, input, &mut budget())
        .unwrap()
        .state;
    scene
        .set_local(
            owner,
            Transform {
                rotation: Quat::from_rotation_z(1.),
                ..Default::default()
            },
        )
        .unwrap();
    let tilted = SupportWorld::from_scene(&scene, 1).unwrap();
    assert_eq!(
        state
            .prepare(&tilted, settings, input, &mut budget())
            .unwrap()
            .status,
        FootContactStatus::Released
    );
    scene.remove_subtree(owner).unwrap();
    let empty = SupportWorld::from_scene(&scene, 1).unwrap();
    assert_eq!(
        state
            .prepare(&empty, settings, input, &mut budget())
            .unwrap()
            .status,
        FootContactStatus::Released
    );
}
#[test]
fn acquisition_distance_validation_and_failed_budget_leave_published_state_intact() {
    let (scene, _, mut input) = setup();
    let settings = FootContactSettings::default();
    let world = SupportWorld::from_scene(&scene, 1).unwrap();
    input.sole.y += 0.15;
    let searching = FootContactState::default()
        .prepare(&world, settings, input, &mut budget())
        .unwrap();
    assert_eq!(searching.status, FootContactStatus::Searching);
    input.sole.y -= 0.15;
    let state = searching
        .state
        .prepare(&world, settings, input, &mut budget())
        .unwrap()
        .state;
    assert_eq!(
        state.prepare(
            &world,
            settings,
            input,
            &mut SupportQueryBudget::new(0).unwrap()
        ),
        Err(PhysicsError::SweepBudget)
    );
    assert!(state.anchor().is_some());
    let invalid = FootContactSettings {
        release_distance: 0.01,
        ..settings
    };
    let mut limit = budget();
    assert_eq!(
        state.prepare(&world, invalid, input, &mut limit),
        Err(PhysicsError::InvalidMotion)
    );
    assert_eq!(limit.remaining(), 1024);
}
