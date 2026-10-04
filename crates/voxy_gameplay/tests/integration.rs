use glam::{Quat, Vec3};
use voxy_gameplay::{
    BoxCollider, CharacterBody, CharacterPhysics, JUMP, PhysicsError, RIGHT, player_input,
};
use voxy_scene::{
    ComponentRegistry, NodeId, ObjectId, SceneCommand, SceneDocument, SceneGraph, SceneObject,
    SceneSimulation, SimulationLimits, Transform,
};
fn at(position: Vec3) -> Transform {
    Transform {
        translation: position,
        ..Transform::default()
    }
}
fn fixture() -> (SceneGraph, NodeId, NodeId) {
    let mut scene = SceneGraph::new(8);
    let level = scene.spawn(None, Transform::default()).unwrap();
    let floor = scene
        .spawn(Some(level), at(Vec3::new(0.0, -0.2, 0.0)))
        .unwrap();
    scene
        .insert_component(
            floor,
            BoxCollider {
                half_extents: [2.0, 0.1, 2.0],
            },
        )
        .unwrap();
    let player = scene.spawn(Some(level), Transform::default()).unwrap();
    scene
        .insert_component(player, CharacterBody::default())
        .unwrap();
    (scene, level, player)
}
fn simulation(scene: &SceneGraph) -> SceneSimulation {
    SceneSimulation::new(
        scene,
        SimulationLimits {
            fixed_step: 1.0 / 60.0,
            max_steps: 8,
            max_behaviors: 4,
            max_commands: 8,
        },
    )
    .unwrap()
}
#[test]
fn quick_tap_survives_render_only_frames_and_is_consumed_once_during_catch_up() {
    let (mut scene, _, player) = fixture();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    for _ in 0..120 {
        physics
            .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
            .unwrap();
    }
    assert!(physics.state(&scene, player).unwrap().unwrap().grounded);
    let initial_height = scene.local(player).unwrap().translation.y;
    let mut simulation = simulation(&scene);
    let plan = voxy_gameplay::character_schedule().unwrap();
    input.event(JUMP, 1.0).unwrap();
    input.event(JUMP, 0.0).unwrap();
    for _ in 0..2 {
        let frame = simulation
            .advance_scoped(&mut scene, 0.004, plan, |system, scene, dt| {
                physics.run_scoped_system(system, scene, &mut input, dt)
            })
            .unwrap();
        assert_eq!(frame.time.steps, 0);
        assert!(input.state("jump").unwrap().pressed);
    }
    let mut edges = Vec::new();
    let frame = simulation
        .advance_scoped(&mut scene, 0.05, plan, |system, scene, dt| {
            if system == "character.step" {
                edges.push(input.state("jump").unwrap().pressed);
            }
            physics.run_scoped_system(system, scene, &mut input, dt)
        })
        .unwrap();
    assert_eq!(frame.time.steps, 3);
    assert_eq!(edges, [true, false, false]);
    assert!(physics.state(&scene, player).unwrap().unwrap().velocity[1] > 0.0);
    assert!(scene.local(player).unwrap().translation.y > initial_height + 0.02);
    assert!(!input.state("jump").unwrap().pressed);
}
#[test]
fn swept_motion_hits_wall_without_tunneling_and_focus_cancels_intent() {
    let (mut scene, _, player) = fixture();
    let wall = scene.spawn(None, at(Vec3::new(0.4, 0.0, 0.0))).unwrap();
    scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [0.05, 2.0, 2.0],
            },
        )
        .unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                speed: 100.0,
                gravity: 0.0,
                ..CharacterBody::default()
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    input.event(RIGHT, 1.0).unwrap();
    physics
        .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
        .unwrap();
    let position = scene.world_matrix(player).unwrap().w_axis.x;
    assert!(position > 0.29 && position < 0.31);
    input.event(JUMP, 1.0).unwrap();
    input.set_focused(false);
    assert!(!input.state("jump").unwrap().pressed);
    assert!(!input.state("move_x").unwrap().held);
    physics
        .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
        .unwrap();
    assert!((scene.world_matrix(player).unwrap().w_axis.x - position).abs() < 1e-5);
    input.set_focused(true);
    assert!(!input.state("move_x").unwrap().held);
    input.event(RIGHT, 1.0).unwrap();
    input.disconnect(0);
    assert!(!input.state("move_x").unwrap().held);
}
#[test]
fn parent_activity_deletion_component_removal_and_reused_generation_reconcile() {
    let (mut scene, level, player) = fixture();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    input.event(RIGHT, 1.0).unwrap();
    physics
        .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
        .unwrap();
    let before = scene.local(player).unwrap();
    scene.set_active(level, false).unwrap();
    physics
        .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
        .unwrap();
    assert_eq!(scene.local(player).unwrap(), before);
    scene.set_active(level, true).unwrap();
    physics
        .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
        .unwrap();
    assert!(scene.local(player).unwrap().translation.x > before.translation.x);
    scene.remove_component::<CharacterBody>(player).unwrap();
    physics.synchronize(&scene).unwrap();
    assert_eq!(physics.body_count(), 0);
    scene
        .insert_component(player, CharacterBody::default())
        .unwrap();
    physics
        .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
        .unwrap();
    let mut simulation = simulation(&scene);
    simulation
        .commands()
        .push(SceneCommand::RemoveSubtree(level))
        .unwrap();
    simulation.advance(&mut scene, 0.0).unwrap();
    physics.synchronize(&scene).unwrap();
    assert_eq!(physics.body_count(), 0);
    assert!(physics.state(&scene, player).is_err());
    let reused = scene.spawn(None, Transform::default()).unwrap();
    assert_ne!(reused, player);
    scene
        .insert_component(
            reused,
            CharacterBody {
                gravity: 0.0,
                ..CharacterBody::default()
            },
        )
        .unwrap();
    input.event(RIGHT, 0.0).unwrap();
    physics
        .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
        .unwrap();
    assert!(
        physics
            .state(&scene, reused)
            .unwrap()
            .unwrap()
            .velocity
            .iter()
            .all(|value| value.abs() < f64::EPSILON)
    );
}
#[test]
fn translated_parent_teleport_resets_velocity_and_affine_dynamic_ancestors_fail() {
    let (mut scene, level, player) = fixture();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    physics
        .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
        .unwrap();
    scene
        .set_local(level, at(Vec3::new(1.0, 2.0, 3.0)))
        .unwrap();
    physics
        .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
        .unwrap();
    let state = physics.state(&scene, player).unwrap().unwrap();
    assert!((state.velocity[1] + 2.4 / 60.0).abs() < 1e-10);
    let before = scene.local(player).unwrap();
    scene
        .set_local(
            level,
            Transform {
                rotation: Quat::from_rotation_z(0.2),
                ..Transform::default()
            },
        )
        .unwrap();
    assert_eq!(
        physics.fixed_step(&mut scene, &mut input, 1.0 / 60.0),
        Err(PhysicsError::UnsupportedTransform)
    );
    assert_eq!(scene.local(player).unwrap(), before);
    scene.set_local(level, Transform::default()).unwrap();
    scene
        .insert_component(level, CharacterBody::default())
        .unwrap();
    assert_eq!(
        physics.validate(&scene),
        Err(PhysicsError::UnsupportedDynamicParent)
    );
}
#[test]
fn failure_in_later_body_preserves_all_poses_runtime_state_and_input() {
    let mut scene = SceneGraph::new(2);
    let a = scene.spawn(None, Transform::default()).unwrap();
    let b = scene
        .spawn(None, at(Vec3::new(999_999.0, 0.0, 0.0)))
        .unwrap();
    for owner in [a, b] {
        scene
            .insert_component(
                owner,
                CharacterBody {
                    speed: 1000.0,
                    gravity: 0.0,
                    ..CharacterBody::default()
                },
            )
            .unwrap();
    }
    let mut physics = CharacterPhysics::new(&scene, 2, 0);
    let mut input = player_input().unwrap();
    input.event(RIGHT, 1.0).unwrap();
    let before = scene.local(a).unwrap();
    assert_eq!(
        physics.fixed_step(&mut scene, &mut input, 0.1),
        Err(PhysicsError::CoordinateRange)
    );
    assert_eq!(scene.local(a).unwrap(), before);
    assert_eq!(physics.body_count(), 0);
    assert!(input.state("move_x").unwrap().pressed);
    assert_eq!(
        physics.fixed_step(&mut scene, &mut input, f64::NAN),
        Err(PhysicsError::InvalidStep)
    );
}
#[test]
fn durable_descriptors_reload_with_new_runtime_identity_and_no_velocity() {
    let mut registry = ComponentRegistry::default();
    voxy_gameplay::register_components(&mut registry).unwrap();
    let object = SceneObject {
        id: ObjectId("player".into()),
        parent: None,
        name: "Player".into(),
        active: true,
        translation: [0.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0; 3],
        components: std::collections::BTreeMap::from([(
            "game.character.v1".into(),
            serde_json::json!({"half_extents": [0.05, 0.05, 0.05], "speed": 0.6, "gravity": -2.4, "jump_speed": 0.9}),
        )]),
    };
    let document = SceneDocument {
        version: 1,
        objects: vec![object],
    };
    let mut loaded = document.load(&registry, 4).unwrap();
    let owner = loaded.resolve(&ObjectId("player".into())).unwrap();
    let mut physics = CharacterPhysics::new(&loaded.graph, 4, 4);
    let mut input = player_input().unwrap();
    input.event(RIGHT, 1.0).unwrap();
    physics
        .fixed_step(&mut loaded.graph, &mut input, 1.0 / 60.0)
        .unwrap();
    let captured = loaded.capture(&registry).unwrap();
    assert_eq!(captured.objects[0].components.len(), 1);
    let mut restored = captured.load(&registry, 4).unwrap();
    let replacement = restored.resolve(&ObjectId("player".into())).unwrap();
    assert_ne!(replacement, owner);
    assert_ne!(loaded.graph.identity(), restored.graph.identity());
    assert!(
        physics
            .fixed_step(&mut restored.graph, &mut input, 1.0 / 60.0)
            .is_err()
    );
    assert_eq!(physics.body_count(), 1);
    let restored_physics = CharacterPhysics::new(&restored.graph, 4, 4);
    assert!(
        restored_physics
            .state(&restored.graph, replacement)
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        CharacterPhysics::new(&loaded.graph, 0, 0).validate(&loaded.graph),
        Err(PhysicsError::Capacity)
    ));
}

#[test]
fn initial_penetration_is_an_explicit_error_without_partial_publication() {
    let (mut scene, _, player) = fixture();
    scene
        .set_local(player, at(Vec3::new(0.0, -0.2, 0.0)))
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    assert_eq!(
        physics.validate_start(&scene),
        Err(PhysicsError::InitialOverlap)
    );
    let before = scene.local(player).unwrap();
    assert_eq!(
        physics.fixed_step(&mut scene, &mut input, 1.0 / 60.0),
        Err(PhysicsError::InitialOverlap)
    );
    assert_eq!(scene.local(player).unwrap(), before);
    assert_eq!(physics.body_count(), 0);
}

#[test]
fn grounded_tangent_motion_is_not_blocked_by_time_zero_floor_contact() {
    let (mut scene, _, player) = fixture();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    for _ in 0..120 {
        physics
            .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
            .unwrap();
    }
    let before = scene.local(player).unwrap();
    input.event(RIGHT, 1.0).unwrap();
    for _ in 0..10 {
        physics
            .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
            .unwrap();
    }
    let after = scene.local(player).unwrap();
    assert!((after.translation.x - before.translation.x - 0.1).abs() < 1e-5);
    assert!((after.translation.y - before.translation.y).abs() < 1e-5);
    assert!(physics.state(&scene, player).unwrap().unwrap().grounded);
}

#[test]
fn rotated_scaled_static_boxes_use_exact_sweeps_and_runtime_overlap_recovery() {
    let mut scene = SceneGraph::new(4);
    let wall = scene
        .spawn(
            None,
            Transform {
                rotation: Quat::from_rotation_y(0.6),
                scale: Vec3::new(0.2, 1., 1.),
                ..Transform::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [0.1, 1., 1.],
            },
        )
        .unwrap();
    let player = scene.spawn(None, at(Vec3::new(-1., 0., 0.))).unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                gravity: 0.,
                speed: 20.,
                ..CharacterBody::default()
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 2, 2).with_depenetration(true);
    let mut input = player_input().unwrap();
    input.event(RIGHT, 1.).unwrap();
    physics.fixed_step(&mut scene, &mut input, 0.1).unwrap();
    let center = scene.local(player).unwrap().translation;
    assert!(center.x > -1. && center.x < 0.5); // 2m sweep cannot tunnel through the thin rotated wall
    assert!(center.z.abs() > 0.01); // motion slides along its continuous normal
    input.event(RIGHT, 0.).unwrap();
    scene.set_local(player, at(Vec3::ZERO)).unwrap();
    physics.fixed_step(&mut scene, &mut input, 0.01).unwrap();
    assert!(scene.local(player).unwrap().translation.length() > 0.05);
    assert!(
        physics
            .state(&scene, player)
            .unwrap()
            .unwrap()
            .velocity
            .iter()
            .all(|v| v.abs() < 1e-10)
    );
    let before = scene.local(player).unwrap();
    physics.fixed_step(&mut scene, &mut input, 0.01).unwrap();
    assert!(
        scene
            .local(player)
            .unwrap()
            .translation
            .distance(before.translation)
            < 1e-5
    );
}

#[test]
fn bounded_recovery_failure_preserves_every_pose_and_input_edge() {
    let mut scene = SceneGraph::new(5);
    for x in [-0.1, 0.1] {
        let wall = scene.spawn(None, at(Vec3::new(x, 0., 0.))).unwrap();
        scene
            .insert_component(
                wall,
                BoxCollider {
                    half_extents: [0.1, 1., 1.],
                },
            )
            .unwrap();
    }
    let safe = scene.spawn(None, at(Vec3::new(-2., 0., 0.))).unwrap();
    let wedged = scene.spawn(None, Transform::default()).unwrap();
    for owner in [safe, wedged] {
        scene
            .insert_component(
                owner,
                CharacterBody {
                    gravity: 0.,
                    ..CharacterBody::default()
                },
            )
            .unwrap();
    }
    let before = [scene.local(safe).unwrap(), scene.local(wedged).unwrap()];
    let mut physics = CharacterPhysics::new(&scene, 2, 2).with_depenetration(true);
    let mut input = player_input().unwrap();
    input.event(RIGHT, 1.).unwrap();
    input.event(JUMP, 1.).unwrap();
    assert_eq!(
        physics.fixed_step(&mut scene, &mut input, 0.01),
        Err(PhysicsError::InitialOverlap)
    );
    assert_eq!(
        [scene.local(safe).unwrap(), scene.local(wedged).unwrap()],
        before
    );
    assert_eq!(physics.body_count(), 0);
    assert!(input.state("jump").unwrap().pressed);
}

#[test]
fn unknown_scheduled_system_preserves_scene_physics_and_input() {
    let (mut scene, _, player) = fixture();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    input.event(JUMP, 1.0).unwrap();
    let before = scene.local(player).unwrap();
    assert_eq!(
        physics.run_scheduled_system("character.typo", &mut scene, &mut input, 1.0 / 60.0),
        Err(PhysicsError::UnknownSystem)
    );
    assert_eq!(scene.local(player).unwrap(), before);
    assert_eq!(physics.body_count(), 0);
    assert!(input.state("jump").unwrap().pressed);
}

#[test]
fn character_step_read_only_grant_preserves_pose_and_pending_input() {
    use voxy_scene::{SchedulePlan, SystemAccess, SystemSpec};
    let (mut scene, _, player) = fixture();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    input.event(JUMP, 1.0).unwrap();
    let before = scene.local(player).unwrap();
    let plan = SchedulePlan::build(
        &[SystemSpec {
            name: "character.step".into(),
            phase: 0,
            after: vec![],
            access: vec![
                SystemAccess {
                    resource: "scene".into(),
                    write: false,
                },
                SystemAccess {
                    resource: "character.physics".into(),
                    write: true,
                },
                SystemAccess {
                    resource: "player.input".into(),
                    write: true,
                },
            ],
        }],
        1,
    )
    .unwrap();
    let failure = plan
        .run_scene(&mut scene, |name, access| {
            physics.run_scoped_system(name, access, &mut input, 1.0 / 60.0)
        })
        .unwrap_err();
    assert_eq!(failure.error, PhysicsError::AccessDenied);
    assert_eq!(scene.local(player).unwrap(), before);
    assert_eq!(physics.body_count(), 0);
    assert!(input.state("jump").unwrap().pressed);
}

#[test]
fn character_domain_denials_preserve_live_physics_pose_and_input() {
    use voxy_scene::{SchedulePlan, SystemAccess, SystemSpec};
    for (domain, read_only) in [
        ("character.physics", false),
        ("character.physics", true),
        ("player.input", false),
        ("player.input", true),
    ] {
        let (mut scene, _, player) = fixture();
        let mut physics = CharacterPhysics::new(&scene, 4, 4);
        let mut input = player_input().unwrap();
        physics
            .fixed_step(&mut scene, &mut input, 1.0 / 60.0)
            .unwrap();
        let before_state = physics.state(&scene, player).unwrap().unwrap();
        let before_pose = scene.local(player).unwrap();
        input.event(JUMP, 1.0).unwrap();
        let mut access: Vec<_> = ["scene", "character.physics", "player.input"]
            .iter()
            .filter(|resource| **resource != domain)
            .map(|resource| SystemAccess {
                resource: (*resource).into(),
                write: true,
            })
            .collect();
        if read_only {
            access.push(SystemAccess {
                resource: domain.into(),
                write: false,
            });
        }
        let plan = SchedulePlan::build(
            &[SystemSpec {
                name: "character.step".into(),
                phase: 0,
                after: vec![],
                access,
            }],
            1,
        )
        .unwrap();
        let failure = plan
            .run_scene(&mut scene, |name, access| {
                physics.run_scoped_system(name, access, &mut input, 1.0 / 60.0)
            })
            .unwrap_err();
        assert_eq!(failure.error, PhysicsError::AccessDenied);
        assert_eq!(scene.local(player).unwrap(), before_pose);
        let after = physics.state(&scene, player).unwrap().unwrap();
        assert_eq!(after.velocity, before_state.velocity);
        assert_eq!(after.grounded, before_state.grounded);
        assert_eq!(physics.body_count(), 1);
        assert!(input.state("jump").unwrap().pressed);
    }
}

fn motion_fixture() -> (SceneGraph, NodeId) {
    let mut scene = SceneGraph::new(8);
    let player = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                gravity: 0.0,
                speed: 0.0,
                ..Default::default()
            },
        )
        .unwrap();
    (scene, player)
}

#[test]
fn animation_displacement_sweeps_slides_and_is_consumed_once() {
    let (mut scene, player) = motion_fixture();
    let wall = scene.spawn(None, at(Vec3::X)).unwrap();
    scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [0.05, 5.0, 5.0],
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    let applied = physics
        .fixed_step_with_motion(
            &mut scene,
            &mut input,
            0.02,
            &[(player, Vec3::new(2.0, 0.0, 0.25))],
        )
        .unwrap();
    let center = scene.local(player).unwrap().translation;
    assert!((center.x - 0.9).abs() < 1e-5, "{center:?}");
    assert!((center.z - 0.25).abs() < 1e-5);
    assert_eq!(applied.len(), 1);
    assert!(applied[0].1.abs_diff_eq(center, 1e-5));
    physics.fixed_step(&mut scene, &mut input, 0.02).unwrap();
    assert!(
        scene
            .local(player)
            .unwrap()
            .translation
            .abs_diff_eq(center, 1e-5)
    );
    let blocked = physics
        .fixed_step_with_motion(&mut scene, &mut input, 0.02, &[(player, Vec3::X)])
        .unwrap();
    assert!(blocked[0].1.length() < 1e-5);
}

#[test]
fn animation_displacement_collides_vertically_and_with_affine_walls() {
    for direction in [-1.0, 1.0] {
        let (mut scene, player) = motion_fixture();
        let obstacle = scene.spawn(None, at(Vec3::Y * direction)).unwrap();
        scene
            .insert_component(
                obstacle,
                BoxCollider {
                    half_extents: [5.0, 0.05, 5.0],
                },
            )
            .unwrap();
        let mut physics = CharacterPhysics::new(&scene, 4, 4);
        let mut input = player_input().unwrap();
        let applied = physics
            .fixed_step_with_motion(
                &mut scene,
                &mut input,
                0.02,
                &[(player, Vec3::Y * direction * 2.0)],
            )
            .unwrap();
        assert!(
            (applied[0].1.y - direction * 0.9).abs() < 1e-5,
            "{applied:?}"
        );
        assert_eq!(
            physics.state(&scene, player).unwrap().unwrap().grounded,
            direction < 0.0
        );
    }
    let (mut scene, player) = motion_fixture();
    let wall = scene
        .spawn(
            None,
            Transform {
                translation: Vec3::X,
                rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_4),
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [0.05, 5.0, 5.0],
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let applied = physics
        .fixed_step_with_motion(
            &mut scene,
            &mut player_input().unwrap(),
            0.02,
            &[(player, Vec3::X * 2.0)],
        )
        .unwrap();
    assert!(applied[0].1.x < 1.5 && applied[0].1.z > 0.4, "{applied:?}");
}

#[test]
fn invalid_animation_motion_preserves_all_owners_and_pending_input() {
    let (mut scene, player) = motion_fixture();
    let other = scene.spawn(None, at(Vec3::Z)).unwrap();
    scene
        .insert_component(
            other,
            CharacterBody {
                gravity: 0.0,
                ..Default::default()
            },
        )
        .unwrap();
    let non_character = scene.spawn(None, Transform::default()).unwrap();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    physics.fixed_step(&mut scene, &mut input, 0.02).unwrap();
    input.event(JUMP, 1.0).unwrap();
    let before = scene.local(player).unwrap();
    let before_other = scene.local(other).unwrap();
    let state = physics.state(&scene, player).unwrap();
    for motions in [
        vec![(player, Vec3::X), (other, Vec3::splat(f32::NAN))],
        vec![(player, Vec3::X), (player, Vec3::Z)],
        vec![(non_character, Vec3::ZERO)],
        vec![(player, Vec3::X * 1e7)],
    ] {
        assert_eq!(
            physics.fixed_step_with_motion(&mut scene, &mut input, 0.02, &motions),
            Err(PhysicsError::InvalidMotion)
        );
        assert_eq!(scene.local(player).unwrap(), before);
        assert_eq!(scene.local(other).unwrap(), before_other);
        assert_eq!(physics.state(&scene, player).unwrap(), state);
        assert!(input.state("jump").unwrap().pressed);
    }
    physics
        .fixed_step_with_motion(&mut scene, &mut input, 0.02, &[(player, Vec3::X)])
        .unwrap();
    assert!(!input.state("jump").unwrap().pressed);
}

#[test]
fn looped_animation_drives_collisions_without_duplicate_pose_translation() {
    use std::sync::Arc;
    use voxy_animation::{AnimationClip, Animator, Joint, JointTrack, Playback, Skeleton, Vec3Key};
    let rig = Skeleton::new(vec![Joint {
        name: "locomotion".into(),
        parent: None,
        bind_local: voxy_animation::Transform::IDENTITY,
        inverse_bind: glam::Mat4::IDENTITY,
    }])
    .unwrap();
    let clip = Arc::new(
        AnimationClip::new(
            "walk",
            1.0,
            Playback::Loop,
            vec![JointTrack {
                translations: vec![
                    Vec3Key {
                        time: 0.0,
                        value: Vec3::ZERO,
                    },
                    Vec3Key {
                        time: 1.0,
                        value: Vec3::X * 10.0,
                    },
                ],
                ..Default::default()
            }],
            &rig,
        )
        .unwrap(),
    );
    let mut animator = Animator::new(clip);
    let (mut scene, player) = motion_fixture();
    let wall = scene.spawn(None, at(Vec3::X)).unwrap();
    scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [0.05, 5.0, 5.0],
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    let mut requested = 0.0;
    let mut accepted = 0.0;
    for _ in 0..125 {
        let mut candidate = animator.clone();
        let frame = candidate.advance(&rig, 0.02).unwrap();
        let (frame, motion) = frame
            .into_in_place_translation(&rig, [true, false, true])
            .unwrap();
        assert_eq!(frame.pose.local()[0].translation, Vec3::ZERO);
        assert_eq!(frame.skin_matrices[0], glam::Mat4::IDENTITY);
        // This fixture's locomotion parent and model-to-world basis are identity.
        let applied = physics
            .fixed_step_with_motion(&mut scene, &mut input, 0.02, &[(player, motion)])
            .unwrap();
        animator = candidate;
        requested += motion.x;
        accepted += applied[0].1.x;
    }
    assert!((requested - 25.0).abs() < 1e-3, "{requested}");
    assert!((accepted - 0.9).abs() < 1e-5, "{accepted}");
    assert!((scene.local(player).unwrap().translation.x - 0.9).abs() < 1e-5);
    let mut candidate = animator.clone();
    let pending = candidate.advance(&rig, 0.02).unwrap().root_motion;
    assert_eq!(
        physics.fixed_step_with_motion(
            &mut scene,
            &mut input,
            0.02,
            &[(player, pending), (player, pending)]
        ),
        Err(PhysicsError::InvalidMotion)
    );
    // Failed physics publication leaves the animation owner untouched for retry.
    assert_eq!(animator.advance(&rig, 0.02).unwrap().root_motion, pending);
}

#[test]
fn horizontal_animation_during_jump_does_not_snap_back_to_floor() {
    let (mut scene, player) = motion_fixture();
    let mut descriptor = *scene.component::<CharacterBody>(player).unwrap().unwrap();
    descriptor.jump_speed = 0.1;
    scene.insert_component(player, descriptor).unwrap();
    let floor = scene.spawn(None, at(Vec3::new(0.0, -0.1, 0.0))).unwrap();
    scene
        .insert_component(
            floor,
            BoxCollider {
                half_extents: [5.0, 0.05, 5.0],
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    physics
        .fixed_step_with_motion(&mut scene, &mut input, 0.02, &[(player, -Vec3::Y * 0.01)])
        .unwrap();
    assert!(physics.state(&scene, player).unwrap().unwrap().grounded);
    input.event(JUMP, 1.0).unwrap();
    physics
        .fixed_step_with_motion(&mut scene, &mut input, 0.02, &[(player, Vec3::X * 0.1)])
        .unwrap();
    let state = physics.state(&scene, player).unwrap().unwrap();
    assert!(!state.grounded);
    assert!((state.velocity[1] - 0.1).abs() < 1e-6);
    assert!((scene.local(player).unwrap().translation.y - 0.002).abs() < 1e-6);
}

#[test]
fn animation_ceiling_contact_removes_persistent_upward_velocity() {
    let (mut scene, player) = motion_fixture();
    let mut descriptor = *scene.component::<CharacterBody>(player).unwrap().unwrap();
    descriptor.jump_speed = 0.1;
    scene.insert_component(player, descriptor).unwrap();
    for (y, half) in [(-0.1, [5.0, 0.05, 5.0]), (0.15, [5.0, 0.05, 5.0])] {
        let obstacle = scene.spawn(None, at(Vec3::Y * y)).unwrap();
        scene
            .insert_component(obstacle, BoxCollider { half_extents: half })
            .unwrap();
    }
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    physics
        .fixed_step_with_motion(&mut scene, &mut input, 0.02, &[(player, -Vec3::Y * 0.01)])
        .unwrap();
    input.event(JUMP, 1.0).unwrap();
    physics
        .fixed_step_with_motion(&mut scene, &mut input, 0.02, &[(player, Vec3::Y)])
        .unwrap();
    let state = physics.state(&scene, player).unwrap().unwrap();
    assert!(!state.grounded);
    assert_eq!(state.velocity[1], 0.0);
    assert!((scene.local(player).unwrap().translation.y - 0.05).abs() < 1e-6);
}

#[test]
fn oriented_character_ignores_empty_enclosing_aabb_corner_and_slides_at_true_support() {
    let mut scene = SceneGraph::new(4);
    let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
    let player = scene
        .spawn(
            None,
            Transform {
                rotation,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                half_extents: [0.4, 0.1, 0.05],
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    let corner = scene.spawn(None, at(Vec3::new(0.3, 0., 0.3))).unwrap();
    scene
        .insert_component(
            corner,
            BoxCollider {
                half_extents: [0.02; 3],
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 1, 2);
    physics.validate_start(&scene).unwrap();
    let mut input = player_input().unwrap();
    physics
        .fixed_step(&mut scene, &mut input, 1. / 60.)
        .unwrap();
    assert_eq!(scene.local(player).unwrap().translation, Vec3::ZERO);
    scene.remove_subtree(corner).unwrap();
    let wall = scene.spawn(None, at(Vec3::X)).unwrap();
    scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [0.1, 1., 1.],
            },
        )
        .unwrap();
    let requested = Vec3::new(2., 0., 0.25);
    let applied = physics
        .fixed_step_with_motion(&mut scene, &mut input, 1. / 60., &[(player, requested)])
        .unwrap();
    let support = (rotation * Vec3::X * 0.4).x.abs() + (rotation * Vec3::Z * 0.05).x.abs();
    let expected_x = 1. - 0.1 - support;
    let position = scene.local(player).unwrap().translation;
    assert!((position.x - expected_x).abs() < 1e-6, "{position:?}");
    assert!((position.z - 0.25).abs() < 1e-6);
    assert!(applied[0].1.abs_diff_eq(position, 1e-6));
    assert_eq!(scene.local(player).unwrap().rotation, rotation);
    let state = physics.state(&scene, player).unwrap().unwrap();
    assert!(((state.body.max[0] - state.body.min[0]) * 0.5 - f64::from(support)).abs() < 1e-7);
    assert!(!state.grounded);
}

#[test]
fn tilted_character_floor_contact_jump_and_recovery_use_rotated_shape() {
    let mut scene = SceneGraph::new(4);
    let rotation = Quat::from_rotation_z(0.3);
    let player = scene
        .spawn(
            None,
            Transform {
                rotation,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                half_extents: [0.4, 0.1, 0.05],
                speed: 0.,
                gravity: 0.,
                jump_speed: 2.,
                ..Default::default()
            },
        )
        .unwrap();
    let floor = scene.spawn(None, at(Vec3::new(0., -1.1, 0.))).unwrap();
    scene
        .insert_component(
            floor,
            BoxCollider {
                half_extents: [2., 0.1, 2.],
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 1, 1);
    let mut input = player_input().unwrap();
    let displacement = Vec3::Y * -2.;
    physics
        .fixed_step_with_motion(&mut scene, &mut input, 1. / 60., &[(player, displacement)])
        .unwrap();
    let support = (rotation * Vec3::X * 0.4).y.abs() + (rotation * Vec3::Y * 0.1).y.abs();
    let grounded = scene.local(player).unwrap().translation;
    assert!((grounded.y - (-1. + support)).abs() < 1e-6);
    assert!(physics.state(&scene, player).unwrap().unwrap().grounded);
    input.event(JUMP, 1.).unwrap();
    physics
        .fixed_step_with_motion(&mut scene, &mut input, 0.01, &[(player, Vec3::X * 0.01)])
        .unwrap();
    assert!(scene.local(player).unwrap().translation.y > grounded.y + 0.019);
    assert!(physics.state(&scene, player).unwrap().unwrap().velocity[1] > 1.99);
    // A teleported overlap rejects atomically; the opt-in recovery uses the OBB.
    scene
        .set_local(
            player,
            Transform {
                translation: Vec3::new(0., -0.95, 0.),
                rotation,
                ..Default::default()
            },
        )
        .unwrap();
    let before = scene.local(player).unwrap();
    input.event(JUMP, 0.).unwrap();
    input.event(JUMP, 1.).unwrap();
    assert_eq!(
        physics.fixed_step(&mut scene, &mut input, 0.01),
        Err(PhysicsError::InitialOverlap)
    );
    assert_eq!(scene.local(player).unwrap(), before);
    assert!(input.state("jump").unwrap().pressed);
    let mut recovering = CharacterPhysics::new(&scene, 1, 1).with_depenetration(true);
    recovering.fixed_step(&mut scene, &mut input, 0.01).unwrap();
    assert!(scene.local(player).unwrap().translation.y >= -1. + support - 1e-6);
    assert_eq!(scene.local(player).unwrap().rotation, rotation);
}

#[test]
fn rigid_motion_preserves_winding_clips_mid_arc_and_retains_accepted_orientation() {
    use voxy_gameplay::CharacterMotion;
    let mut scene = SceneGraph::new(3);
    let player = scene.spawn(None, Default::default()).unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                half_extents: [0.4, 0.1, 0.02],
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    let wall = scene.spawn(None, at(Vec3::Z * 0.25)).unwrap();
    scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [2., 2., 0.02],
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 1, 1);
    let mut input = player_input().unwrap();
    let requested = CharacterMotion {
        owner: player,
        displacement: Vec3::ZERO,
        angular_displacement: Vec3::Y * std::f32::consts::TAU,
    };
    let receipt = physics
        .fixed_step_with_rigid_motion(&mut scene, &mut input, 1. / 60., &[requested])
        .unwrap()[0];
    assert!((0.05..0.2).contains(&receipt.angular_fraction));
    let expected = (0.23_f64 / 0.4_f64.hypot(0.02)).asin() - 0.02_f64.atan2(0.4);
    assert!((f64::from(receipt.angular_displacement.y) - expected).abs() < 1e-6);
    let accepted = scene.local(player).unwrap();
    assert!(
        accepted
            .rotation
            .abs_diff_eq(Quat::from_rotation_y(receipt.angular_displacement.y), 1e-6)
    );
    physics
        .fixed_step(&mut scene, &mut input, 1. / 60.)
        .unwrap();
    assert_eq!(scene.local(player).unwrap().rotation, accepted.rotation);
    let reverse = CharacterMotion {
        angular_displacement: -receipt.angular_displacement,
        ..requested
    };
    assert_eq!(
        physics
            .fixed_step_with_rigid_motion(&mut scene, &mut input, 1. / 60., &[reverse])
            .unwrap()[0]
            .angular_fraction,
        1.
    );
    assert!(
        scene
            .local(player)
            .unwrap()
            .rotation
            .abs_diff_eq(Quat::IDENTITY, 1e-6)
    );
}

#[test]
fn angular_ground_turn_preserves_jump_and_continuing_velocity() {
    use voxy_gameplay::CharacterMotion;
    let (mut scene, _, player) = fixture();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    for _ in 0..60 {
        physics
            .fixed_step(&mut scene, &mut input, 1. / 60.)
            .unwrap();
    }
    assert!(physics.state(&scene, player).unwrap().unwrap().grounded);
    let request = CharacterMotion {
        owner: player,
        displacement: Vec3::ZERO,
        angular_displacement: Vec3::Y * 0.3,
    };
    assert_eq!(
        physics
            .fixed_step_with_rigid_motion(&mut scene, &mut input, 1. / 60., &[request])
            .unwrap()[0]
            .angular_fraction,
        1.
    );
    assert!(physics.state(&scene, player).unwrap().unwrap().grounded);
    input.event(JUMP, 1.).unwrap();
    physics
        .fixed_step_with_rigid_motion(&mut scene, &mut input, 1. / 60., &[request])
        .unwrap();
    let before = physics.state(&scene, player).unwrap().unwrap().velocity[1];
    assert!(before > 0.);
    assert!(!physics.state(&scene, player).unwrap().unwrap().grounded);
    physics
        .fixed_step(&mut scene, &mut input, 1. / 60.)
        .unwrap();
    let after = physics.state(&scene, player).unwrap().unwrap().velocity[1];
    assert!((after - (before - 2.4 / 60.)).abs() < 1e-10);
}

#[test]
fn angular_budget_and_invalid_requests_preserve_pose_state_and_input() {
    use voxy_gameplay::CharacterMotion;
    let mut scene = SceneGraph::new(3);
    let player = scene.spawn(None, Default::default()).unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                half_extents: [0.4, 0.1, 0.02],
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    let wall = scene.spawn(None, at(Vec3::Z * 0.25)).unwrap();
    scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [2., 2., 0.02],
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 1, 1)
        .with_angular_sweep_budget(1)
        .unwrap();
    let mut input = player_input().unwrap();
    physics
        .fixed_step(&mut scene, &mut input, 1. / 60.)
        .unwrap();
    let before = scene.local(player).unwrap();
    let velocity = physics.state(&scene, player).unwrap().unwrap().velocity;
    input.event(JUMP, 1.).unwrap();
    let request = CharacterMotion {
        owner: player,
        displacement: Vec3::X * 0.01,
        angular_displacement: Vec3::Y * std::f32::consts::PI,
    };
    assert_eq!(
        physics
            .fixed_step_with_rigid_motion(&mut scene, &mut input, 1. / 60., &[request])
            .unwrap_err(),
        PhysicsError::SweepBudget
    );
    for invalid in [Vec3::splat(f32::NAN), Vec3::Y * 100.] {
        let request = CharacterMotion {
            angular_displacement: invalid,
            ..request
        };
        assert_eq!(
            physics
                .fixed_step_with_rigid_motion(&mut scene, &mut input, 1. / 60., &[request])
                .unwrap_err(),
            PhysicsError::InvalidMotion
        );
    }
    assert_eq!(
        physics
            .fixed_step_with_rigid_motion(&mut scene, &mut input, 1. / 60., &[request, request])
            .unwrap_err(),
        PhysicsError::InvalidMotion
    );
    assert_eq!(scene.local(player).unwrap(), before);
    assert_eq!(
        physics.state(&scene, player).unwrap().unwrap().velocity,
        velocity
    );
    assert!(input.state("jump").unwrap().pressed);
    assert!(
        CharacterPhysics::new(&scene, 1, 1)
            .with_angular_sweep_budget(0)
            .is_err()
    );
}

#[test]
fn authored_character_rotation_uses_one_writer_and_complete_arc() {
    use voxy_gameplay::{AngularMotion, AngularMotionBatch, bind_authored_behaviors};
    let mut scene = SceneGraph::new(3);
    let player = scene.spawn(None, Default::default()).unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                half_extents: [0.4, 0.1, 0.02],
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            player,
            AngularMotion {
                axis: [0., 1., 0.],
                radians_per_second: std::f64::consts::TAU * 60.,
            },
        )
        .unwrap();
    let wall = scene.spawn(None, at(Vec3::Z * 0.25)).unwrap();
    scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [2., 2., 0.02],
            },
        )
        .unwrap();
    let mut batch = AngularMotionBatch::new(&scene, 1).unwrap();
    let mut legacy = simulation(&scene);
    bind_authored_behaviors(&mut scene, &mut legacy).unwrap();
    batch.fixed_step(&mut scene, 1. / 60.).unwrap();
    legacy.advance(&mut scene, 1. / 60.).unwrap();
    assert_eq!(scene.local(player).unwrap().rotation, Quat::IDENTITY);
    let mut physics = CharacterPhysics::new(&scene, 1, 1);
    let mut input = player_input().unwrap();
    // The legacy translation API must not expose implicit angular receipts.
    assert!(
        physics
            .fixed_step_with_motion(&mut scene, &mut input, 1. / 60., &[])
            .unwrap()
            .is_empty()
    );
    let expected = (0.23 / 0.4_f64.hypot(0.02)).asin() - 0.02_f64.atan2(0.4);
    let accepted = scene.local(player).unwrap().rotation;
    assert!(accepted.abs_diff_eq(Quat::from_rotation_y(expected as f32), 1e-6));
    physics
        .fixed_step(&mut scene, &mut input, 1. / 60.)
        .unwrap();
    assert!(
        scene
            .local(player)
            .unwrap()
            .rotation
            .abs_diff_eq(accepted, 1e-6)
    );
    scene
        .insert_component(
            player,
            AngularMotion {
                axis: [0., 1., 0.],
                radians_per_second: -expected * 60.,
            },
        )
        .unwrap();
    let receipts = physics
        .fixed_step_with_rigid_motion(&mut scene, &mut input, 1. / 60., &[])
        .unwrap();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].angular_fraction, 1.);
    assert!(
        scene
            .local(player)
            .unwrap()
            .rotation
            .abs_diff_eq(Quat::IDENTITY, 1e-6)
    );
}

#[test]
fn authored_axes_are_parent_local_and_conflicting_sources_are_atomic() {
    use voxy_gameplay::{AngularMotion, CharacterMotion};
    let mut scene = SceneGraph::new(1);
    let initial = Quat::from_rotation_z(0.7);
    let player = scene
        .spawn(
            None,
            Transform {
                rotation: initial,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            player,
            AngularMotion {
                axis: [0., 1., 0.],
                radians_per_second: 3.,
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 1, 0);
    let mut input = player_input().unwrap();
    physics.fixed_step(&mut scene, &mut input, 0.1).unwrap();
    assert!(
        scene
            .local(player)
            .unwrap()
            .rotation
            .abs_diff_eq(Quat::from_rotation_y(0.3) * initial, 1e-6)
    );
    let before = scene.local(player).unwrap();
    input.event(JUMP, 1.).unwrap();
    let request = CharacterMotion {
        owner: player,
        displacement: Vec3::X,
        angular_displacement: Vec3::Y,
    };
    assert_eq!(
        physics
            .fixed_step_with_rigid_motion(&mut scene, &mut input, 0.1, &[request])
            .unwrap_err(),
        PhysicsError::InvalidMotion
    );
    assert_eq!(scene.local(player).unwrap(), before);
    assert!(input.state("jump").unwrap().pressed);
    scene
        .insert_component(
            player,
            AngularMotion {
                axis: [0., 1., 0.],
                radians_per_second: 1e6,
            },
        )
        .unwrap();
    assert_eq!(
        physics.fixed_step(&mut scene, &mut input, 0.1).unwrap_err(),
        PhysicsError::InvalidMotion
    );
    assert_eq!(scene.local(player).unwrap(), before);
    assert!(input.state("jump").unwrap().pressed);
}

fn rotation_trajectory(
    values: &[Quat],
    mode: voxy_animation::Interpolation,
    tangents: Vec<[glam::Vec4; 2]>,
) -> voxy_animation::RootRotationPath {
    use voxy_animation::{
        AnimationClip, Joint, JointTangents, JointTrack, Playback, QuatKey, Skeleton,
        TrackInterpolation,
    };
    let rig = Skeleton::new(vec![Joint {
        name: std::sync::Arc::from("root"),
        parent: None,
        bind_local: voxy_animation::Transform::IDENTITY,
        inverse_bind: glam::Mat4::IDENTITY,
    }])
    .unwrap();
    let keys = values
        .iter()
        .enumerate()
        .map(|(index, q)| QuatKey {
            time: index as f32 / (values.len() - 1) as f32,
            value: *q,
        })
        .collect();
    AnimationClip::new_with_tangents(
        "turn",
        1.,
        Playback::Clamp,
        vec![JointTrack {
            rotations: keys,
            ..Default::default()
        }],
        vec![TrackInterpolation {
            rotation: mode,
            ..Default::default()
        }],
        vec![JointTangents {
            rotation: tangents,
            ..Default::default()
        }],
        &rig,
    )
    .unwrap()
    .root_rotation_curve(0)
    .unwrap()
    .path(0., 1., 256)
    .unwrap()
}
fn trajectory_wall_scene() -> (SceneGraph, NodeId) {
    let mut scene = SceneGraph::new(4);
    let player = scene.spawn(None, Default::default()).unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                half_extents: [0.4, 0.1, 0.02],
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    let wall = scene.spawn(None, at(Vec3::Z * 0.25)).unwrap();
    scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [2., 2., 0.02],
            },
        )
        .unwrap();
    (scene, player)
}

#[test]
fn one_tick_curved_trajectory_hits_before_equal_endpoints_and_retains_orientation() {
    use voxy_animation::Interpolation;
    use voxy_gameplay::CharacterTrajectoryMotion;
    let trajectory = rotation_trajectory(
        &[Quat::IDENTITY, Quat::IDENTITY],
        Interpolation::CubicSpline,
        vec![
            [glam::Vec4::ZERO, glam::Vec4::Y * 8.],
            [-glam::Vec4::Y * 8., glam::Vec4::ZERO],
        ],
    );
    assert!(
        trajectory
            .end_rotation()
            .abs_diff_eq(glam::DQuat::IDENTITY, 1e-12)
    );
    let (mut scene, player) = trajectory_wall_scene();
    let mut physics = CharacterPhysics::new(&scene, 1, 1);
    let mut input = player_input().unwrap();
    let request = CharacterTrajectoryMotion {
        owner: player,
        displacement: Vec3::ZERO,
        rotation: &trajectory,
        basis: glam::DQuat::IDENTITY,
        pivot: Vec3::ZERO,
    };
    let receipt = physics
        .fixed_step_with_trajectory_motion(&mut scene, &mut input, 1. / 60., &[request])
        .unwrap()[0];
    let angle = (0.23 / 0.4_f64.hypot(0.02)).asin() - 0.02_f64.atan2(0.4);
    // q(t) = normalize([0, 8t(1-t), 0, 1]); first wall contact is analytic.
    let time = (1. - (1. - 4. * (angle * 0.5).tan() / 8.).sqrt()) * 0.5;
    assert!(!receipt.complete);
    assert!((receipt.path_fraction - time).abs() < 1e-7, "{receipt:?}");
    assert!(receipt.path_fraction <= time);
    assert!(receipt.completed_spans < trajectory.spans().len());
    assert!(
        scene
            .local(player)
            .unwrap()
            .rotation
            .abs_diff_eq(Quat::from_rotation_y(angle as f32), 1e-6)
    );
    let accepted = scene.local(player).unwrap();
    physics
        .fixed_step(&mut scene, &mut input, 1. / 60.)
        .unwrap();
    assert_eq!(scene.local(player).unwrap().rotation, accepted.rotation);
}

#[test]
fn ordered_full_turn_and_final_step_do_not_hide_a_collision() {
    use voxy_animation::Interpolation;
    use voxy_gameplay::CharacterTrajectoryMotion;
    let values: Vec<_> = (0..=4)
        .map(|i| Quat::from_rotation_y(i as f32 * std::f32::consts::FRAC_PI_2))
        .collect();
    for trajectory in [
        rotation_trajectory(&values, Interpolation::Linear, vec![]),
        rotation_trajectory(
            &[Quat::IDENTITY, Quat::from_rotation_y(1.)],
            Interpolation::Step,
            vec![],
        ),
    ] {
        let (mut scene, player) = trajectory_wall_scene();
        let mut physics = CharacterPhysics::new(&scene, 1, 1);
        let mut input = player_input().unwrap();
        let receipt = physics
            .fixed_step_with_trajectory_motion(
                &mut scene,
                &mut input,
                1. / 60.,
                &[CharacterTrajectoryMotion {
                    owner: player,
                    displacement: Vec3::ZERO,
                    rotation: &trajectory,
                    basis: glam::DQuat::IDENTITY,
                    pivot: Vec3::ZERO,
                }],
            )
            .unwrap()[0];
        let angle = (0.23 / 0.4_f64.hypot(0.02)).asin() - 0.02_f64.atan2(0.4);
        assert!(!receipt.complete);
        assert!(
            scene
                .local(player)
                .unwrap()
                .rotation
                .abs_diff_eq(Quat::from_rotation_y(angle as f32), 1e-6)
        );
        if trajectory.spans().last().unwrap().is_step() {
            assert_eq!(receipt.path_fraction, 1.);
            assert_eq!(receipt.completed_spans, 1);
            assert!((receipt.span_fraction - angle).abs() < 1e-7);
        } else {
            assert!(receipt.path_fraction < 0.25);
        }
    }
}

#[test]
fn noncommuting_curved_path_uses_body_basis_and_commits_once() {
    use voxy_animation::Interpolation;
    use voxy_gameplay::CharacterTrajectoryMotion;
    let trajectory = rotation_trajectory(
        &[
            Quat::IDENTITY,
            Quat::from_rotation_x(0.6),
            Quat::from_rotation_y(0.8) * Quat::from_rotation_x(0.6),
        ],
        Interpolation::CubicSpline,
        vec![
            [glam::Vec4::ZERO, glam::Vec4::new(0.4, 0.3, -0.2, 0.1)],
            [
                glam::Vec4::new(-0.3, 0.2, 0.4, 0.1),
                glam::Vec4::new(0.2, -0.4, 0.3, -0.1),
            ],
            [glam::Vec4::new(0.1, 0.3, -0.4, 0.1), glam::Vec4::ZERO],
        ],
    );
    let initial = Quat::from_rotation_z(0.3);
    let basis = glam::DQuat::from_rotation_y(-0.4);
    let mut scene = SceneGraph::new(1);
    let player = scene
        .spawn(
            None,
            Transform {
                rotation: initial,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 1, 0);
    let mut input = player_input().unwrap();
    let receipt = physics
        .fixed_step_with_trajectory_motion(
            &mut scene,
            &mut input,
            1. / 60.,
            &[CharacterTrajectoryMotion {
                owner: player,
                displacement: Vec3::X * 0.05,
                rotation: &trajectory,
                basis,
                pivot: Vec3::ZERO,
            }],
        )
        .unwrap()[0];
    assert!(receipt.complete);
    assert_eq!(receipt.completed_spans, trajectory.spans().len());
    let expected_delta = basis * trajectory.end_rotation() * basis.conjugate();
    assert!(receipt.rotation.abs_diff_eq(expected_delta, 1e-12));
    let expected =
        glam::DQuat::from_array(initial.to_array().map(f64::from)).normalize() * expected_delta;
    assert!(scene.local(player).unwrap().rotation.abs_diff_eq(
        Quat::from_array(expected.to_array().map(|v| v as f32)),
        1e-6
    ));
    assert!((receipt.displacement.x - 0.05).abs() < 1e-7);
    assert!((scene.local(player).unwrap().translation.x - 0.05).abs() < 1e-7);
}

#[test]
fn curved_grounded_yaw_preserves_jump_and_tall_body_avoids_sphere_speed_overestimate() {
    use voxy_animation::Interpolation;
    use voxy_gameplay::CharacterTrajectoryMotion;
    let yaw = rotation_trajectory(
        &[Quat::IDENTITY, Quat::from_rotation_y(0.7)],
        Interpolation::CubicSpline,
        vec![
            [glam::Vec4::ZERO, glam::Vec4::Y * 0.5],
            [glam::Vec4::Y * 0.3, glam::Vec4::ZERO],
        ],
    );
    let (mut scene, _, player) = fixture();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    for _ in 0..60 {
        physics
            .fixed_step(&mut scene, &mut input, 1. / 60.)
            .unwrap();
    }
    let request = CharacterTrajectoryMotion {
        owner: player,
        displacement: Vec3::ZERO,
        rotation: &yaw,
        basis: glam::DQuat::IDENTITY,
        pivot: Vec3::ZERO,
    };
    assert!(
        physics
            .fixed_step_with_trajectory_motion(&mut scene, &mut input, 1. / 60., &[request])
            .unwrap()[0]
            .complete
    );
    assert!(physics.state(&scene, player).unwrap().unwrap().grounded);
    input.event(JUMP, 1.).unwrap();
    assert!(
        physics
            .fixed_step_with_trajectory_motion(&mut scene, &mut input, 1. / 60., &[request])
            .unwrap()[0]
            .complete
    );
    let before = physics.state(&scene, player).unwrap().unwrap().velocity[1];
    assert!(before > 0.);
    assert!(!physics.state(&scene, player).unwrap().unwrap().grounded);
    physics
        .fixed_step(&mut scene, &mut input, 1. / 60.)
        .unwrap();
    assert!(
        (physics.state(&scene, player).unwrap().unwrap().velocity[1] - (before - 2.4 / 60.)).abs()
            < 1e-10
    );
    let (mut scene, player) = trajectory_wall_scene();
    scene
        .insert_component(
            player,
            CharacterBody {
                half_extents: [0.4, 1000., 0.02],
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 1, 1);
    let receipt = physics
        .fixed_step_with_trajectory_motion(
            &mut scene,
            &mut input,
            1. / 60.,
            &[CharacterTrajectoryMotion {
                owner: player,
                ..request
            }],
        )
        .unwrap()[0];
    assert!(!receipt.complete);
    let angle = (0.23 / 0.4_f64.hypot(0.02)).asin() - 0.02_f64.atan2(0.4);
    assert!(
        scene
            .local(player)
            .unwrap()
            .rotation
            .abs_diff_eq(Quat::from_rotation_y(angle as f32), 1e-6)
    );
}

#[test]
fn trajectory_query_failure_after_another_body_stages_motion_is_atomic() {
    use voxy_animation::Interpolation;
    use voxy_gameplay::{AngularMotion, CharacterTrajectoryMotion};
    let trajectory = rotation_trajectory(
        &[Quat::IDENTITY, Quat::from_rotation_y(1.)],
        Interpolation::Linear,
        vec![],
    );
    let (mut scene, player) = trajectory_wall_scene();
    scene.set_local(player, at(Vec3::X * 10.)).unwrap();
    let other = scene.spawn(None, at(Vec3::ZERO)).unwrap();
    scene
        .insert_component(
            other,
            CharacterBody {
                half_extents: [0.4, 0.1, 0.02],
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        scene
            .active_components::<CharacterBody>()
            .map(|(owner, _)| owner)
            .collect::<Vec<_>>(),
        vec![player, other]
    );
    let mut physics = CharacterPhysics::new(&scene, 2, 1)
        .with_angular_trajectory_query_budget(3)
        .unwrap();
    let mut input = player_input().unwrap();
    physics
        .fixed_step(&mut scene, &mut input, 1. / 60.)
        .unwrap();
    let poses = [scene.local(player).unwrap(), scene.local(other).unwrap()];
    let velocities = [
        physics.state(&scene, player).unwrap().unwrap().velocity,
        physics.state(&scene, other).unwrap().unwrap().velocity,
    ];
    input.event(JUMP, 1.).unwrap();
    let request = CharacterTrajectoryMotion {
        owner: player,
        displacement: Vec3::X * 0.01,
        rotation: &trajectory,
        basis: glam::DQuat::IDENTITY,
        pivot: Vec3::ZERO,
    };
    let requests = [
        CharacterTrajectoryMotion {
            owner: other,
            ..request
        },
        request,
    ];
    assert_eq!(
        physics
            .fixed_step_with_trajectory_motion(&mut scene, &mut input, 1. / 60., &requests)
            .unwrap_err(),
        PhysicsError::SweepBudget
    );
    for (index, owner) in [player, other].into_iter().enumerate() {
        assert_eq!(scene.local(owner).unwrap(), poses[index]);
        assert_eq!(
            physics.state(&scene, owner).unwrap().unwrap().velocity,
            velocities[index]
        );
    }
    assert!(input.state("jump").unwrap().pressed);
    scene
        .insert_component(
            player,
            AngularMotion {
                axis: [0., 1., 0.],
                radians_per_second: 1.,
            },
        )
        .unwrap();
    assert_eq!(
        physics
            .fixed_step_with_trajectory_motion(&mut scene, &mut input, 1. / 60., &[request])
            .unwrap_err(),
        PhysicsError::InvalidMotion
    );
    assert_eq!(scene.local(player).unwrap(), poses[0]);
    assert!(input.state("jump").unwrap().pressed);
    assert!(
        CharacterPhysics::new(&scene, 2, 1)
            .with_angular_trajectory_query_budget(0)
            .is_err()
    );
}

#[test]
fn angular_iteration_budget_is_shared_across_ordered_spans() {
    use voxy_animation::Interpolation;
    use voxy_gameplay::CharacterTrajectoryMotion;
    let quarter = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
    let first = rotation_trajectory(&[Quat::IDENTITY, quarter], Interpolation::Linear, vec![]);
    let full = rotation_trajectory(
        &[
            Quat::IDENTITY,
            quarter,
            Quat::from_rotation_y(-std::f32::consts::PI),
        ],
        Interpolation::Linear,
        vec![],
    );
    let fixture = || {
        let mut scene = SceneGraph::new(2);
        let player = scene.spawn(None, Default::default()).unwrap();
        scene
            .insert_component(
                player,
                CharacterBody {
                    half_extents: [0.4, 0.1, 0.02],
                    speed: 0.,
                    gravity: 0.,
                    ..Default::default()
                },
            )
            .unwrap();
        let wall = scene.spawn(None, at(Vec3::new(0.3, 0., 0.3))).unwrap();
        scene
            .insert_component(
                wall,
                BoxCollider {
                    half_extents: [0.005, 0.1, 0.005],
                },
            )
            .unwrap();
        (scene, player)
    };
    let (mut scene, player) = fixture();
    let mut physics = CharacterPhysics::new(&scene, 1, 1);
    let mut input = player_input().unwrap();
    let receipt = physics
        .fixed_step_with_trajectory_motion(
            &mut scene,
            &mut input,
            1. / 60.,
            &[CharacterTrajectoryMotion {
                owner: player,
                displacement: Vec3::ZERO,
                rotation: &first,
                basis: glam::DQuat::IDENTITY,
                pivot: Vec3::ZERO,
            }],
        )
        .unwrap()[0];
    assert!(receipt.complete);
    assert!(receipt.advancement_iterations > 0 && receipt.advancement_iterations < 256);
    assert_eq!(
        receipt.trajectory_queries,
        receipt.advancement_iterations + 2
    );
    let (mut scene, player) = fixture();
    let before = scene.local(player).unwrap();
    let mut physics = CharacterPhysics::new(&scene, 1, 1)
        .with_angular_sweep_budget(receipt.advancement_iterations)
        .unwrap();
    input.event(JUMP, 1.).unwrap();
    let request = CharacterTrajectoryMotion {
        owner: player,
        displacement: Vec3::ZERO,
        rotation: &full,
        basis: glam::DQuat::IDENTITY,
        pivot: Vec3::ZERO,
    };
    assert_eq!(
        physics
            .fixed_step_with_trajectory_motion(&mut scene, &mut input, 1. / 60., &[request])
            .unwrap_err(),
        PhysicsError::SweepBudget
    );
    assert_eq!(scene.local(player).unwrap(), before);
    assert!(physics.state(&scene, player).unwrap().is_none());
    assert!(input.state("jump").unwrap().pressed);
    let mut physics = CharacterPhysics::new(&scene, 1, 1);
    let completed = physics
        .fixed_step_with_trajectory_motion(&mut scene, &mut input, 1. / 60., &[request])
        .unwrap()[0];
    assert!(completed.complete);
    assert!(completed.advancement_iterations > receipt.advancement_iterations);
}

#[test]
fn offset_pivot_curved_turn_hits_on_center_arc_before_equal_endpoints() {
    use voxy_animation::Interpolation;
    use voxy_gameplay::CharacterTrajectoryMotion;
    let trajectory = rotation_trajectory(
        &[Quat::IDENTITY, Quat::IDENTITY],
        Interpolation::CubicSpline,
        vec![
            [glam::Vec4::ZERO, glam::Vec4::Y * 8.],
            [-glam::Vec4::Y * 8., glam::Vec4::ZERO],
        ],
    );
    for sign in [-1_f32, 1.] {
        let (mut scene, player) = trajectory_wall_scene();
        let wall = scene.active_components::<BoxCollider>().next().unwrap().0;
        scene.set_local(wall, at(Vec3::Z * (0.25 * sign))).unwrap();
        let mut physics = CharacterPhysics::new(&scene, 1, 1);
        let mut input = player_input().unwrap();
        let request = CharacterTrajectoryMotion {
            owner: player,
            displacement: Vec3::ZERO,
            rotation: &trajectory,
            basis: glam::DQuat::IDENTITY,
            pivot: Vec3::X * (0.6 * sign),
        };
        let receipt = physics
            .fixed_step_with_trajectory_motion(&mut scene, &mut input, 1. / 60., &[request])
            .unwrap()[0];
        // Pivot at x=.6: the center's z=.6*sin(angle), and the front corner's
        // z=1*sin(angle)+.02*cos(angle). The first contact is independent of sampling.
        let angle = (0.23 / 1_f64.hypot(0.02)).asin() - 0.02_f64.atan2(1.);
        let time = (1. - (1. - 4. * (angle * 0.5).tan() / 8.).sqrt()) * 0.5;
        assert!(!receipt.complete);
        assert!((receipt.path_fraction - time).abs() < 1e-7, "{receipt:?}");
        let pose = scene.local(player).unwrap();
        assert!(
            pose.rotation
                .abs_diff_eq(Quat::from_rotation_y(angle as f32), 1e-6)
        );
        let expected = Vec3::new(
            (0.6 * (1. - angle.cos())) as f32,
            0.,
            (0.6 * angle.sin()) as f32,
        );
        let expected = expected * sign;
        assert!(pose.translation.abs_diff_eq(expected, 1e-7), "{pose:?}");
        assert!(receipt.displacement.abs_diff_eq(expected, 1e-7));
        let body = physics.state(&scene, player).unwrap().unwrap().body;
        let physical = glam::DVec3::new(
            body.anchor.x as f64,
            body.anchor.y as f64,
            body.anchor.z as f64,
        ) + (glam::DVec3::from_array(body.min) + glam::DVec3::from_array(body.max))
            * 0.5;
        assert!(
            (physical + receipt.rotation * request.pivot.as_dvec3())
                .abs_diff_eq(request.pivot.as_dvec3(), 1e-12)
        );
        // Scene transforms are f32: casting and normalizing the quaternion adds a few ULPs.
        let anchor = pose.translation + pose.rotation * request.pivot;
        assert!(
            anchor.abs_diff_eq(request.pivot, 2e-7),
            "pose={pose:?} anchor={anchor:?} receipt={receipt:?}"
        );
        physics
            .fixed_step(&mut scene, &mut input, 1. / 60.)
            .unwrap();
        assert_eq!(scene.local(player).unwrap(), pose);
    }
}

#[test]
fn noncommuting_offset_pivot_uses_body_basis_and_failure_is_atomic() {
    use voxy_animation::Interpolation;
    use voxy_gameplay::CharacterTrajectoryMotion;
    let trajectory = rotation_trajectory(
        &[
            Quat::IDENTITY,
            Quat::from_rotation_x(0.6),
            Quat::from_rotation_z(-0.7) * Quat::from_rotation_y(0.4),
        ],
        Interpolation::CubicSpline,
        vec![[glam::Vec4::ZERO; 2]; 3],
    );
    let mut scene = SceneGraph::new(2);
    let start = Transform {
        translation: Vec3::new(0.2, 0.3, -0.1),
        rotation: Quat::from_rotation_z(0.3),
        ..Default::default()
    };
    let player = scene.spawn(None, start).unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    let basis = glam::DQuat::from_rotation_y(-0.4);
    let pivot = Vec3::new(0.4, 0.7, -0.2);
    let displacement = Vec3::X * 0.05;
    let request = CharacterTrajectoryMotion {
        owner: player,
        displacement,
        rotation: &trajectory,
        basis,
        pivot,
    };
    let mut physics = CharacterPhysics::new(&scene, 1, 0);
    let mut input = player_input().unwrap();
    let receipt = physics
        .fixed_step_with_trajectory_motion(&mut scene, &mut input, 1. / 60., &[request])
        .unwrap()[0];
    let initial = glam::DQuat::from_array(start.rotation.to_array().map(f64::from)).normalize();
    let delta = (basis * trajectory.end_rotation() * basis.conjugate()).normalize();
    let expected =
        displacement.as_dvec3() + initial * (pivot.as_dvec3() - delta * pivot.as_dvec3());
    let pose = scene.local(player).unwrap();
    assert!(receipt.complete);
    assert!(receipt.rotation.abs_diff_eq(delta, 1e-12));
    assert!(
        pose.translation
            .abs_diff_eq(start.translation + expected.as_vec3(), 2e-7)
    );
    assert!((pose.translation + pose.rotation * pivot).abs_diff_eq(
        start.translation + displacement + start.rotation * pivot,
        2e-7
    ));
    // A rejected pivot must leave the already accepted scene, carried velocity and input intact.
    input.event(JUMP, 1.).unwrap();
    let state = physics.state(&scene, player).unwrap().unwrap();
    let invalid = CharacterTrajectoryMotion {
        pivot: Vec3::splat(f32::NAN),
        ..request
    };
    assert_eq!(
        physics
            .fixed_step_with_trajectory_motion(&mut scene, &mut input, 1. / 60., &[invalid])
            .unwrap_err(),
        PhysicsError::InvalidMotion
    );
    assert_eq!(scene.local(player).unwrap(), pose);
    assert_eq!(
        physics.state(&scene, player).unwrap().unwrap().velocity,
        state.velocity
    );
    assert!(input.state("jump").unwrap().pressed);
    let mut limited = physics.with_angular_trajectory_query_budget(1).unwrap();
    assert_eq!(
        limited
            .fixed_step_with_trajectory_motion(&mut scene, &mut input, 1. / 60., &[request])
            .unwrap_err(),
        PhysicsError::SweepBudget
    );
    assert_eq!(scene.local(player).unwrap(), pose);
    assert_eq!(
        limited.state(&scene, player).unwrap().unwrap().velocity,
        state.velocity
    );
    assert!(input.state("jump").unwrap().pressed);
}

fn moving_root_trajectory(axes: [bool; 3], end: f64) -> voxy_animation::RootRigidPath {
    use std::sync::Arc;
    use voxy_animation::{
        AnimationClip, Interpolation, Joint, JointTangents, JointTrack, Playback, QuatKey,
        Skeleton, TrackInterpolation, Transform as BoneTransform, Vec3Key,
    };
    let pivot = Vec3::X * 0.6;
    let skeleton = Skeleton::new(vec![Joint {
        name: Arc::from("root"),
        parent: None,
        bind_local: BoneTransform {
            translation: pivot,
            ..BoneTransform::IDENTITY
        },
        inverse_bind: glam::Mat4::IDENTITY,
    }])
    .unwrap();
    let clip = AnimationClip::new_with_tangents(
        "moving pivot",
        1.,
        Playback::Clamp,
        vec![JointTrack {
            translations: vec![
                Vec3Key {
                    time: 0.,
                    value: pivot,
                },
                Vec3Key {
                    time: 1.,
                    value: pivot,
                },
            ],
            rotations: vec![
                QuatKey {
                    time: 0.,
                    value: Quat::IDENTITY,
                },
                QuatKey {
                    time: 1.,
                    value: Quat::IDENTITY,
                },
            ],
            ..Default::default()
        }],
        vec![TrackInterpolation {
            translation: Interpolation::CubicSpline,
            rotation: Interpolation::CubicSpline,
            ..Default::default()
        }],
        vec![JointTangents {
            translation: vec![[Vec3::ZERO, Vec3::Z * 2.], [-Vec3::Z * 2., Vec3::ZERO]],
            rotation: vec![
                [glam::Vec4::ZERO, glam::Vec4::Y * 8.],
                [-glam::Vec4::Y * 8., glam::Vec4::ZERO],
            ],
            ..Default::default()
        }],
        &skeleton,
    )
    .unwrap();
    clip.root_rigid_curve(0)
        .unwrap()
        .path(0., end, axes, 128)
        .unwrap()
}

#[test]
fn simultaneous_translation_and_turn_clip_the_closed_curve_before_the_wall() {
    use voxy_gameplay::CharacterRigidTrajectoryMotion;
    for axes in [[false; 3], [true; 3]] {
        let path = moving_root_trajectory(axes, 1.);
        assert!(path.end_transform().translation.length() < 1e-12);
        let (mut scene, player) = trajectory_wall_scene();
        let mut physics = CharacterPhysics::new(&scene, 1, 1);
        let mut input = player_input().unwrap();
        let request = CharacterRigidTrajectoryMotion {
            scale: 1.,
            owner: player,
            trajectory: &path,
            basis: glam::DQuat::IDENTITY,
            origin: Vec3::ZERO,
        };
        let receipt = physics
            .fixed_step_with_rigid_trajectories(&mut scene, &mut input, 1. / 60., &[request])
            .unwrap()[0];
        // Independent source equations: pz=2t(1-t), q=normalize(0,8t(1-t),0,1).
        // This is neither a straight endpoint chord nor translation followed by rotation.
        let front = |t: f64| {
            let angle = 2. * (8. * t * (1. - t)).atan();
            let z = 2. * t * (1. - t);
            angle.sin() + 0.02 * angle.cos() + if axes[2] { z } else { z * (1. - angle.cos()) }
        };
        let (mut low, mut high) = (0., 0.2);
        assert!(front(high) > 0.23);
        for _ in 0..80 {
            let mid = (low + high) * 0.5;
            if front(mid) < 0.23 {
                low = mid;
            } else {
                high = mid;
            }
        }
        let time = (low + high) * 0.5;
        assert!(!receipt.complete);
        assert!(
            (receipt.path_fraction - time).abs() < 1e-7,
            "{receipt:?} expected={time}"
        );
        assert!(receipt.path_fraction <= time);
        let angle = 2. * (8. * time * (1. - time)).atan();
        let rotation = glam::DQuat::from_rotation_y(angle);
        let pivot = glam::DVec3::X * 0.6 + glam::DVec3::Z * (2. * time * (1. - time));
        let expected = pivot - rotation * if axes[2] { glam::DVec3::X * 0.6 } else { pivot };
        assert!(receipt.displacement.as_dvec3().abs_diff_eq(expected, 1e-7));
        assert!(receipt.rotation.abs_diff_eq(rotation, 1e-7));
        let pose = scene.local(player).unwrap();
        physics
            .fixed_step(&mut scene, &mut input, 1. / 60.)
            .unwrap();
        assert_eq!(scene.local(player).unwrap(), pose);
    }
}

#[test]
fn composed_trajectory_frame_conversion_and_rejections_preserve_the_atomic_tick() {
    use voxy_gameplay::CharacterRigidTrajectoryMotion;
    let path = moving_root_trajectory([true; 3], 1.);
    let (mut scene, player) = trajectory_wall_scene();
    let start = scene.local(player).unwrap();
    let mut physics = CharacterPhysics::new(&scene, 1, 1)
        .with_angular_trajectory_query_budget(1)
        .unwrap();
    let mut input = player_input().unwrap();
    input.event(JUMP, 1.).unwrap();
    let request = CharacterRigidTrajectoryMotion {
        scale: 1.,
        owner: player,
        trajectory: &path,
        basis: glam::DQuat::IDENTITY,
        origin: Vec3::ZERO,
    };
    assert_eq!(
        physics
            .fixed_step_with_rigid_trajectories(&mut scene, &mut input, 1. / 60., &[request])
            .unwrap_err(),
        PhysicsError::SweepBudget
    );
    assert_eq!(scene.local(player).unwrap(), start);
    assert!(input.state("jump").unwrap().pressed);
    assert_eq!(
        physics
            .fixed_step_with_rigid_trajectories(
                &mut scene,
                &mut input,
                1. / 60.,
                &[request, request]
            )
            .unwrap_err(),
        PhysicsError::InvalidMotion
    );
    assert_eq!(scene.local(player).unwrap(), start);
    assert!(input.state("jump").unwrap().pressed);

    let mut scene = SceneGraph::new(2);
    let initial = Transform {
        translation: Vec3::new(0.2, 0.3, -0.1),
        rotation: Quat::from_rotation_z(0.3),
        ..Default::default()
    };
    let player = scene.spawn(None, initial).unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                gravity: 0.,
                speed: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    // Restrict to a nonidentity endpoint, and independently conjugate the full rigid transform.
    let path = moving_root_trajectory([true; 3], 0.2);
    let basis = glam::DQuat::from_rotation_y(-0.4);
    let origin = Vec3::new(0.4, 0.7, -0.2);
    let request = CharacterRigidTrajectoryMotion {
        scale: 1.,
        owner: player,
        trajectory: &path,
        basis,
        origin,
    };
    let mut physics = CharacterPhysics::new(&scene, 1, 0);
    let receipt = physics
        .fixed_step_with_rigid_trajectories(&mut scene, &mut input, 1. / 60., &[request])
        .unwrap()[0];
    assert!(receipt.complete);
    let q = glam::DQuat::from_rotation_y(2. * (8_f64 * 0.2 * 0.8).atan());
    let bind = f64::from(0.6_f32);
    let p = glam::DVec3::new(bind, 0., 2. * 0.2 * 0.8) - q * glam::DVec3::X * bind;
    let delta = (basis * q * basis.conjugate()).normalize();
    let orientation =
        glam::DQuat::from_array(initial.rotation.to_array().map(f64::from)).normalize();
    let expected = orientation * (basis * p + origin.as_dvec3() - delta * origin.as_dvec3());
    assert!(receipt.displacement.as_dvec3().abs_diff_eq(expected, 1e-7));
    assert!(receipt.rotation.abs_diff_eq(delta, 1e-7));
    assert!(
        scene
            .local(player)
            .unwrap()
            .translation
            .abs_diff_eq((initial.translation.as_dvec3() + expected).as_vec3(), 1e-7),
        "pose={:?} expected={:?} receipt={receipt:?}",
        scene.local(player).unwrap(),
        (initial.translation.as_dvec3() + expected).as_vec3()
    );
}

#[test]
fn moving_pivot_yaw_keeps_a_grounded_tall_body_on_the_floor_without_advancement() {
    use voxy_gameplay::CharacterRigidTrajectoryMotion;
    let path = moving_root_trajectory([false; 3], 1.);
    let mut scene = SceneGraph::new(3);
    let player = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                half_extents: [0.4, 1000., 0.02],
                gravity: 0.,
                speed: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    let floor = scene.spawn(None, at(Vec3::Y * -1000.125)).unwrap();
    scene
        .insert_component(
            floor,
            BoxCollider {
                half_extents: [10., 0.125, 10.],
            },
        )
        .unwrap();
    let mut physics = CharacterPhysics::new(&scene, 1, 1)
        .with_angular_sweep_budget(1)
        .unwrap();
    let mut input = player_input().unwrap();
    let request = CharacterRigidTrajectoryMotion {
        scale: 1.,
        owner: player,
        trajectory: &path,
        basis: glam::DQuat::IDENTITY,
        origin: Vec3::ZERO,
    };
    let receipt = physics
        .fixed_step_with_rigid_trajectories(&mut scene, &mut input, 1. / 60., &[request])
        .unwrap()[0];
    assert!(receipt.complete, "{receipt:?}");
    assert_eq!(receipt.advancement_iterations, 0);
    assert!(receipt.displacement.length() < 1e-6);
}

#[test]
fn reflected_uniform_source_frame_sweeps_translation_and_rotation_together() {
    use voxy_gameplay::CharacterRigidTrajectoryMotion;
    let path = moving_root_trajectory([true; 3], 1.);
    let (mut scene, player) = trajectory_wall_scene();
    let mut physics = CharacterPhysics::new(&scene, 1, 1);
    let mut input = player_input().unwrap();
    let request = CharacterRigidTrajectoryMotion {
        owner: player,
        trajectory: &path,
        scale: -2.,
        basis: glam::DQuat::from_rotation_x(std::f64::consts::PI),
        origin: Vec3::ZERO,
    };
    let receipt = physics
        .fixed_step_with_rigid_trajectories(&mut scene, &mut input, 1. / 60., &[request])
        .unwrap()[0];
    let (mut low, mut high) = (0., 0.1);
    for _ in 0..80 {
        let t = (low + high) * 0.5;
        let angle = 2. * (8_f64 * t * (1. - t)).atan();
        let front = 1.6 * angle.sin() + 4. * t * (1. - t) + 0.02 * angle.cos();
        if front < 0.23 {
            low = t;
        } else {
            high = t;
        }
    }
    let time = (low + high) * 0.5;
    assert!(!receipt.complete);
    assert!((receipt.path_fraction - time).abs() < 1e-7, "{receipt:?}");
    let angle = 2. * (8_f64 * time * (1. - time)).atan();
    assert!(
        receipt
            .rotation
            .abs_diff_eq(glam::DQuat::from_rotation_y(-angle), 1e-7)
    );
    let expected = glam::DVec3::new(
        -1.2 * (1. - angle.cos()),
        0.,
        1.2 * angle.sin() + 4. * time * (1. - time),
    );
    assert!(receipt.displacement.as_dvec3().abs_diff_eq(expected, 1e-7));
    let pose = scene.local(player).unwrap();
    input.event(JUMP, 1.).unwrap();
    for scale in [0., f64::NAN, f64::INFINITY] {
        let invalid = CharacterRigidTrajectoryMotion { scale, ..request };
        assert_eq!(
            physics
                .fixed_step_with_rigid_trajectories(&mut scene, &mut input, 1. / 60., &[invalid])
                .unwrap_err(),
            PhysicsError::InvalidMotion
        );
        assert_eq!(scene.local(player).unwrap(), pose);
        assert!(input.state("jump").unwrap().pressed);
    }
}

#[test]
fn failed_pose_preparation_preserves_physics_scene_and_pending_input() {
    use voxy_gameplay::CharacterTickError;
    let (mut scene, _, player) = fixture();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    for _ in 0..120 {
        physics
            .fixed_step(&mut scene, &mut input, 1. / 60.)
            .unwrap();
    }
    let before = scene.local(player).unwrap();
    let state = physics.state(&scene, player).unwrap().unwrap();
    input.event(JUMP, 1.).unwrap();
    input.event(JUMP, 0.).unwrap();
    let error = physics
        .fixed_step_with_preparation(
            &mut scene,
            &mut input,
            1. / 60.,
            &[(player, Vec3::new(0.1, 0., 0.))],
            &[],
            |preview, _| {
                let accepted = &preview.characters[0];
                assert_eq!(accepted.owner, player);
                assert!(accepted.world_matrix.w_axis.x > before.translation.x);
                assert!(accepted.world_matrix.w_axis.y > before.translation.y);
                Err::<(), _>("IK rejected")
            },
        )
        .unwrap_err();
    assert_eq!(error, CharacterTickError::Preparation("IK rejected"));
    assert_eq!(scene.local(player).unwrap(), before);
    assert_eq!(physics.state(&scene, player).unwrap().unwrap(), state);
    assert!(input.state("jump").unwrap().pressed);
    let (_, candidate) = physics
        .fixed_step_with_preparation(
            &mut scene,
            &mut input,
            1. / 60.,
            &[(player, Vec3::new(0.1, 0., 0.))],
            &[],
            |preview, _| Ok::<_, ()>(preview.characters[0]),
        )
        .unwrap();
    assert_eq!(scene.world_matrix(player).unwrap(), candidate.world_matrix);
    assert_eq!(
        physics.state(&scene, player).unwrap().unwrap().grounded,
        candidate.grounded
    );
    assert!(!input.state("jump").unwrap().pressed);
}

#[test]
fn support_failure_in_preparation_rolls_back_and_physics_failure_skips_preparation() {
    use voxy_gameplay::{CharacterTickError, SupportProbe};
    let (mut scene, _, player) = fixture();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    physics = physics.with_angular_trajectory_query_budget(1).unwrap();
    let before = scene.local(player).unwrap();
    let error = physics
        .fixed_step_with_preparation(
            &mut scene,
            &mut input,
            1. / 60.,
            &[],
            &[],
            |preview, budget| {
                assert_eq!(budget.remaining(), 1);
                let probe = SupportProbe {
                    origin: glam::DVec3::Y,
                    direction: -glam::DVec3::Y,
                    max_distance: 2.,
                    up: glam::DVec3::Y,
                    min_up_dot: 0.7,
                };
                preview.support.probe(probe, budget)?;
                preview.support.probe(probe, budget)
            },
        )
        .unwrap_err();
    assert_eq!(
        error,
        CharacterTickError::Preparation(PhysicsError::SweepBudget)
    );
    assert_eq!(scene.local(player).unwrap(), before);
    assert!(physics.state(&scene, player).unwrap().is_none());
    let error = physics
        .fixed_step_with_preparation(
            &mut scene,
            &mut input,
            f64::NAN,
            &[],
            &[],
            |_, _| -> Result<(), ()> { panic!("invalid physics cannot invoke preparation") },
        )
        .unwrap_err();
    assert_eq!(
        error,
        CharacterTickError::Physics(PhysicsError::InvalidStep)
    );
}

#[test]
fn foot_contact_candidates_publish_only_after_character_tick_accepts() {
    use voxy_gameplay::{
        CharacterTickError, FootContactInput, FootContactSettings, FootContactState,
        FootContactStatus,
    };
    let (mut scene, _, player) = fixture();
    let mut physics = CharacterPhysics::new(&scene, 4, 4);
    let mut input = player_input().unwrap();
    for _ in 0..120 {
        physics
            .fixed_step(&mut scene, &mut input, 1. / 60.)
            .unwrap();
    }
    let published_foot = FootContactState::default();
    let sole = glam::DVec3::new(0.3, -f64::from(0.1_f32), 0.2);
    let settings = FootContactSettings::default();
    let (_, planted) = physics
        .fixed_step_with_preparation(
            &mut scene,
            &mut input,
            1. / 60.,
            &[],
            &[],
            |preview, budget| {
                published_foot.prepare(
                    &preview.support,
                    settings,
                    FootContactInput {
                        sole,
                        up: glam::DVec3::Y,
                        grounded: preview.characters[0].grounded,
                        plant: true,
                    },
                    budget,
                )
            },
        )
        .unwrap();
    assert_eq!(planted.status, FootContactStatus::Planted);
    let published_foot = planted.state;
    let before = scene.local(player).unwrap();
    let error = physics
        .fixed_step_with_preparation(
            &mut scene,
            &mut input,
            1. / 60.,
            &[(player, Vec3::X * 0.1)],
            &[],
            |preview, budget| {
                let candidate = published_foot
                    .prepare(
                        &preview.support,
                        settings,
                        FootContactInput {
                            sole: sole + glam::DVec3::X * 0.4,
                            up: glam::DVec3::Y,
                            grounded: preview.characters[0].grounded,
                            plant: true,
                        },
                        budget,
                    )
                    .unwrap();
                assert_eq!(candidate.status, FootContactStatus::Released);
                Err::<(), _>("later leg IK rejected")
            },
        )
        .unwrap_err();
    assert_eq!(
        error,
        CharacterTickError::Preparation("later leg IK rejected")
    );
    assert_eq!(scene.local(player).unwrap(), before);
    assert_eq!(published_foot.anchor(), planted.state.anchor());
    assert!(published_foot.anchor().is_some());
}

#[test]
fn ordered_screw_trajectory_hits_wall_after_translation_and_budget_failure_is_atomic() {
    use glam::{DQuat, DVec3};
    use voxy_animation::{RootRigidPath, RootRigidTwist};
    use voxy_gameplay::CharacterRigidTrajectoryMotion;
    let angular = DVec3::Y * 2.;
    let pivot = DVec3::X * 0.6;
    let path = RootRigidPath::from_twists(
        &[
            (
                RootRigidTwist {
                    linear: DVec3::X,
                    angular: DVec3::ZERO,
                },
                0.1,
            ),
            (
                RootRigidTwist {
                    linear: -angular.cross(pivot),
                    angular,
                },
                0.5,
            ),
        ],
        2,
    )
    .unwrap();
    let (mut scene, player) = trajectory_wall_scene();
    let before = scene.local(player).unwrap();
    let mut input = player_input().unwrap();
    input.event(JUMP, 1.).unwrap();
    let request = CharacterRigidTrajectoryMotion {
        owner: player,
        trajectory: &path,
        basis: DQuat::IDENTITY,
        origin: Vec3::ZERO,
        scale: 1.,
    };
    let mut limited = CharacterPhysics::new(&scene, 1, 1)
        .with_angular_trajectory_query_budget(1)
        .unwrap();
    assert_eq!(
        limited
            .fixed_step_with_rigid_trajectories(&mut scene, &mut input, 1. / 60., &[request])
            .unwrap_err(),
        PhysicsError::SweepBudget
    );
    assert_eq!(scene.local(player).unwrap(), before);
    assert!(input.state("jump").unwrap().pressed);
    let mut physics = CharacterPhysics::new(&scene, 1, 1);
    let receipt = physics
        .fixed_step_with_rigid_trajectories(&mut scene, &mut input, 1. / 60., &[request])
        .unwrap()[0];
    // Center: (.6-.5*cos(angle),0,.5*sin(angle)); front extent: .4*sin+.02*cos.
    let angle = (0.23 / 0.9_f64.hypot(0.02)).asin() - 0.02_f64.atan2(0.9);
    let fraction = (0.1 + angle / 2.) / 0.6;
    assert!(!receipt.complete);
    assert!(
        (receipt.path_fraction - fraction).abs() < 1e-7,
        "{receipt:?}"
    );
    assert!(receipt.path_fraction <= fraction);
    assert_eq!(receipt.completed_spans, 1);
    assert!(receipt.displacement.as_dvec3().abs_diff_eq(
        DVec3::new(0.6 - 0.5 * angle.cos(), 0., 0.5 * angle.sin()),
        1e-7
    ));
    assert!(
        receipt
            .rotation
            .abs_diff_eq(DQuat::from_rotation_y(angle), 1e-7)
    );
    assert!(!input.state("jump").unwrap().pressed);
    let accepted = scene.local(player).unwrap();
    physics
        .fixed_step(&mut scene, &mut input, 1. / 60.)
        .unwrap();
    assert_eq!(scene.local(player).unwrap(), accepted);
}

#[test]
fn rigid_preparation_rejects_final_contact_narrowing_before_publication() {
    use voxy_animation::{RootRigidPath, RootRigidTwist};
    use voxy_gameplay::{CharacterRigidTrajectoryMotion, CharacterTickError};
    let mut scene = SceneGraph::new(2);
    let wall = scene.spawn(None, at(Vec3::X)).unwrap();
    scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [0.1, 2., 2.],
            },
        )
        .unwrap();
    let player = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                half_extents: [0.1; 3],
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    let path = RootRigidPath::from_twists(
        &[(
            RootRigidTwist {
                linear: glam::DVec3::X * 2.,
                angular: glam::DVec3::ZERO,
            },
            1.,
        )],
        1,
    )
    .unwrap();
    let request = CharacterRigidTrajectoryMotion {
        owner: player,
        trajectory: &path,
        scale: 1.,
        basis: glam::DQuat::IDENTITY,
        origin: Vec3::ZERO,
    };
    let mut physics = CharacterPhysics::new(&scene, 1, 1);
    let mut input = player_input().unwrap();
    input.event(JUMP, 1.).unwrap();
    let before = scene.local(player).unwrap();
    let mut prepared = false;
    let result = physics.fixed_step_with_preparation(
        &mut scene,
        &mut input,
        1. / 60.,
        &[],
        &[request],
        |_, _| {
            prepared = true;
            Ok::<_, ()>(())
        },
    );
    assert!(matches!(result, Err(CharacterTickError::Physics(_))));
    assert!(!prepared);
    assert_eq!(scene.local(player).unwrap(), before);
    assert!(physics.state(&scene, player).unwrap().is_none());
    assert!(input.state("jump").unwrap().pressed);
    let safe = RootRigidPath::from_twists(
        &[(
            RootRigidTwist {
                linear: glam::DVec3::X * 0.2,
                angular: glam::DVec3::ZERO,
            },
            1.,
        )],
        1,
    )
    .unwrap();
    let safe_request = CharacterRigidTrajectoryMotion {
        trajectory: &safe,
        ..request
    };
    let (_, accepted) = physics
        .fixed_step_with_preparation(
            &mut scene,
            &mut input,
            1. / 60.,
            &[],
            &[safe_request],
            |preview, _| {
                prepared = true;
                Ok::<_, ()>(preview.characters[0])
            },
        )
        .unwrap();
    assert!(prepared);
    assert_eq!(scene.world_matrix(player).unwrap(), accepted.world_matrix);
    assert!(!input.state("jump").unwrap().pressed);
}

#[test]
fn certified_fade_transaction_preserves_floor_and_rolls_back_failed_pose_preparation() {
    use voxy_animation::{
        RootRigidCertifiedFadeInterval, RootRigidMappedPath, RootRigidPath, RootRigidTransform,
        RootRigidTwist,
    };
    use voxy_gameplay::{CharacterCertifiedFadeMotion, CharacterTickError};
    let mut scene = SceneGraph::new(2);
    let floor = scene.spawn(None, at(Vec3::Y * (-0.5))).unwrap();
    scene
        .insert_component(
            floor,
            BoxCollider {
                half_extents: [4., 0.5, 4.],
            },
        )
        .unwrap();
    let player = scene.spawn(None, at(Vec3::Y * 0.125)).unwrap();
    scene
        .insert_component(
            player,
            CharacterBody {
                half_extents: [0.125; 3],
                speed: 0.,
                gravity: 0.,
                jump_speed: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    let path = RootRigidPath::from_twists(
        &[(
            RootRigidTwist {
                linear: glam::DVec3::X * 0.3,
                angular: glam::DVec3::Y * 0.4,
            },
            1.,
        )],
        1,
    )
    .unwrap();
    let mapped = RootRigidMappedPath::new(&path, RootRigidTransform::IDENTITY, 1.).unwrap();
    let dt = 0.0625;
    let fade = RootRigidCertifiedFadeInterval::integrate_paths(
        None,
        mapped,
        [0., 1.],
        dt,
        0.01,
        0.01,
        4096,
    )
    .unwrap();
    let request = CharacterCertifiedFadeMotion {
        owner: player,
        fade: &fade,
        scale: 1.,
        basis: glam::DQuat::IDENTITY,
        origin: Vec3::ZERO,
        coordinate_axis: 1,
        evaluation_radius: 0.002,
        evaluation_axes: Some([0.001, 0., 0.001]),
    };
    let mut physics = CharacterPhysics::new(&scene, 1, 1);
    let mut input = player_input().unwrap();
    input.event(JUMP, 1.).unwrap();
    let before = scene.local(player).unwrap();
    for axes in [[0., -0.001, 0.], [0., f64::NAN, 0.], [0.003, 0., 0.]] {
        let invalid = CharacterCertifiedFadeMotion {
            evaluation_axes: Some(axes),
            ..request
        };
        let error = physics
            .fixed_step_with_certified_fade_preparation(
                &mut scene,
                &mut input,
                dt,
                &[invalid],
                |_, _| -> Result<(), ()> { panic!("invalid bounds reached publication") },
            )
            .unwrap_err();
        assert_eq!(
            error,
            CharacterTickError::Physics(PhysicsError::InvalidMotion)
        );
        assert_eq!(scene.local(player).unwrap(), before);
        assert!(physics.state(&scene, player).unwrap().is_none());
        assert!(input.state("jump").unwrap().pressed);
    }
    let result = physics.fixed_step_with_certified_fade_preparation(
        &mut scene,
        &mut input,
        dt,
        &[request],
        |preview, _| {
            assert_eq!(preview.motions.len(), 1);
            assert!(preview.motions[0].complete);
            assert_eq!(preview.characters[0].physical_center.y, 0.125);
            Err::<(), _>("stale rig")
        },
    );
    assert_eq!(
        result.unwrap_err(),
        CharacterTickError::Preparation("stale rig")
    );
    assert_eq!(scene.local(player).unwrap(), before);
    assert!(physics.state(&scene, player).unwrap().is_none());
    assert!(input.state("jump").unwrap().pressed);
    let (receipts, accepted) = physics
        .fixed_step_with_certified_fade_preparation(
            &mut scene,
            &mut input,
            dt,
            &[request],
            |preview, _| Ok::<_, ()>(preview.characters[0]),
        )
        .unwrap();
    assert!(receipts[0].complete);
    assert_eq!(scene.world_matrix(player).unwrap(), accepted.world_matrix);
    assert_eq!(scene.local(player).unwrap().translation.y, 0.125);
    assert!(!input.state("jump").unwrap().pressed);
    let before = scene.local(player).unwrap();
    let error = physics
        .fixed_step_with_certified_fade_preparation(
            &mut scene,
            &mut input,
            dt * 0.5,
            &[request],
            |_, _| Ok::<_, ()>(()),
        )
        .unwrap_err();
    assert_eq!(
        error,
        CharacterTickError::Physics(PhysicsError::InvalidMotion)
    );
    assert_eq!(scene.local(player).unwrap(), before);
}

#[test]
fn mixed_certified_fade_tick_rolls_back_and_retries_all_motion_kinds() {
    use voxy_animation::{
        RootRigidCertifiedFadeInterval, RootRigidMappedPath, RootRigidPath, RootRigidTransform,
        RootRigidTwist,
    };
    use voxy_gameplay::{
        CharacterCertifiedFadeMotion, CharacterRigidTrajectoryMotion, CharacterTickError,
    };
    let mut scene = SceneGraph::new(3);
    let mut owners = Vec::new();
    for x in [0., 2., 4.] {
        let owner = scene.spawn(None, at(Vec3::X * x)).unwrap();
        scene
            .insert_component(
                owner,
                CharacterBody {
                    half_extents: [0.125; 3],
                    speed: 0.,
                    gravity: 0.,
                    jump_speed: 0.,
                    ..Default::default()
                },
            )
            .unwrap();
        owners.push(owner);
    }
    let dt = 0.0625;
    let ordinary = RootRigidPath::from_twists(
        &[(
            RootRigidTwist {
                linear: glam::DVec3::X * 0.25,
                angular: glam::DVec3::ZERO,
            },
            dt,
        )],
        1,
    )
    .unwrap();
    let fade_source = RootRigidPath::from_twists(
        &[(
            RootRigidTwist {
                linear: glam::DVec3::X * 0.5,
                angular: glam::DVec3::Y * 0.25,
            },
            1.,
        )],
        1,
    )
    .unwrap();
    let fade = RootRigidCertifiedFadeInterval::integrate_paths(
        None,
        RootRigidMappedPath::new(&fade_source, RootRigidTransform::IDENTITY, 1.).unwrap(),
        [0., 1.],
        dt,
        0.01,
        0.01,
        4096,
    )
    .unwrap();
    let rigid = CharacterRigidTrajectoryMotion {
        owner: owners[1],
        trajectory: &ordinary,
        scale: 1.,
        basis: glam::DQuat::IDENTITY,
        origin: Vec3::ZERO,
    };
    let certified = CharacterCertifiedFadeMotion {
        owner: owners[2],
        fade: &fade,
        scale: 1.,
        basis: glam::DQuat::IDENTITY,
        origin: Vec3::ZERO,
        coordinate_axis: 1,
        evaluation_radius: 0.,
        evaluation_axes: None,
    };
    let translations = [(owners[0], Vec3::X * 0.125)];
    let mut physics = CharacterPhysics::new(&scene, 3, 0);
    let mut input = player_input().unwrap();
    input.event(JUMP, 1.).unwrap();
    let before: Vec<_> = owners
        .iter()
        .map(|owner| scene.local(*owner).unwrap())
        .collect();
    for owner in &owners {
        assert!(physics.accepted_pose(&scene, *owner).unwrap().is_none());
    }
    let error = physics
        .fixed_step_with_mixed_certified_fade_preparation(
            &mut scene,
            &mut input,
            dt,
            &translations,
            &[rigid],
            &[certified],
            |preview, _| {
                assert_eq!(preview.characters.len(), 3);
                assert_eq!(preview.motions.len(), 2);
                Err::<(), _>("palette failure")
            },
        )
        .unwrap_err();
    assert_eq!(error, CharacterTickError::Preparation("palette failure"));
    for (owner, pose) in owners.iter().zip(&before) {
        assert_eq!(&scene.local(*owner).unwrap(), pose);
        assert!(physics.state(&scene, *owner).unwrap().is_none());
    }
    assert!(input.state("jump").unwrap().pressed);
    let (receipts, accepted) = physics
        .fixed_step_with_mixed_certified_fade_preparation(
            &mut scene,
            &mut input,
            dt,
            &translations,
            &[rigid],
            &[certified],
            |preview, _| Ok::<_, ()>(preview.characters.clone()),
        )
        .unwrap();
    assert_eq!(receipts.len(), 2);
    assert!(receipts.iter().all(|receipt| receipt.complete));
    assert_eq!(scene.local(owners[0]).unwrap().translation.x, 0.125);
    assert!(scene.local(owners[1]).unwrap().translation.x > 2.);
    assert!(scene.local(owners[2]).unwrap().translation.x > 4.);
    for pose in accepted {
        let published = physics.accepted_pose(&scene, pose.owner).unwrap().unwrap();
        assert_eq!(published.world_matrix, pose.world_matrix);
        assert_eq!(published.physical_center, pose.physical_center);
        assert_eq!(published.physical_rotation, pose.physical_rotation);
        assert_eq!(published.velocity, pose.velocity);
        assert_eq!(published.grounded, pose.grounded);
    }
    let published_local = scene.local(owners[2]).unwrap();
    let mut edited = published_local;
    edited.translation.x += 0.5;
    scene.set_local(owners[2], edited).unwrap();
    assert!(physics.accepted_pose(&scene, owners[2]).unwrap().is_none());
    scene.set_local(owners[2], published_local).unwrap();
    assert!(physics.accepted_pose(&scene, owners[2]).unwrap().is_some());
    edited = published_local;
    edited.rotation = glam::Quat::from_rotation_y(0.2);
    scene.set_local(owners[2], edited).unwrap();
    assert!(physics.accepted_pose(&scene, owners[2]).unwrap().is_none());
    scene.set_local(owners[2], published_local).unwrap();
    let descriptor = *scene
        .component::<CharacterBody>(owners[2])
        .unwrap()
        .unwrap();
    scene
        .component_mut::<CharacterBody>(owners[2])
        .unwrap()
        .unwrap()
        .gravity = -1.;
    assert!(physics.accepted_pose(&scene, owners[2]).unwrap().is_none());
    scene.insert_component(owners[2], descriptor).unwrap();
    scene.set_active(owners[2], false).unwrap();
    assert!(physics.accepted_pose(&scene, owners[2]).unwrap().is_none());
    scene.set_active(owners[2], true).unwrap();
    assert!(physics.accepted_pose(&scene, owners[2]).unwrap().is_some());
    assert!(!input.state("jump").unwrap().pressed);
}
