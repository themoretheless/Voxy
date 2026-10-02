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
