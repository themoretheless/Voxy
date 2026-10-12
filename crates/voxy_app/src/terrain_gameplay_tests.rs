use super::*;
use voxy_world::{CollisionShape, Sample, VoxelView};

#[test]
fn scripted_movement_recovers_after_focus_clear() {
    let mut input = InputState::default();
    apply_autopilot_controls(&mut input, 200);
    assert!(input.take_move().local.length() > 0.9);
    input.clear_keyboard();
    assert_eq!(input.take_move().local, glam::Vec2::ZERO);
    apply_autopilot_controls(&mut input, 200);
    let movement = input.take_move().local;
    assert!(movement.x < 0.0 && movement.y > 0.0);
    apply_autopilot_controls(&mut input, 241);
    assert_eq!(input.take_move().local, glam::Vec2::Y);
    apply_autopilot_controls(&mut input, 600);
    assert_eq!(input.take_move().local, glam::Vec2::ZERO);
}

#[test]
fn pending_water_conflict_preserves_new_activations_and_retries_sources() {
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0).unwrap();
    let states = bootstrap_water_states(&scene.world).unwrap();
    let source = VoxelPos {
        x: 16,
        y: 18,
        z: 16,
    };
    let new = VoxelPos { x: 0, y: 20, z: 0 };
    let plan = step_water(
        &scene.world,
        scene.world.registry(),
        states,
        &[source],
        EditSource::Simulation,
        WaterBudget::default(),
    )
    .unwrap();
    scene
        .world
        .commit(voxy_world::EditTxn {
            source: EditSource::Simulation,
            expected: vec![],
            writes: vec![voxy_world::VoxelWrite {
                pos: new,
                block: states.0[0],
            }],
        })
        .unwrap();
    let mut active = vec![new, new];
    let mut dirty = BTreeSet::new();
    assert_eq!(
        publish_water_plan(
            &mut scene.world,
            &mut active,
            &mut dirty,
            vec![source],
            plan
        )
        .unwrap(),
        WaterPublication::Retry
    );
    assert!(dirty.is_empty());
    assert_eq!(active, [new, source]);
    assert_eq!(scene.world.sample(source), Sample::Loaded(states.0[7]));
    let fresh = step_water(
        &scene.world,
        scene.world.registry(),
        states,
        &[source],
        EditSource::Simulation,
        WaterBudget::default(),
    )
    .unwrap();
    active.retain(|&pos| pos != source);
    assert_eq!(
        publish_water_plan(
            &mut scene.world,
            &mut active,
            &mut dirty,
            vec![source],
            fresh
        )
        .unwrap(),
        WaterPublication::Committed
    );
    assert!(dirty.contains(&voxy_core::split_voxel(source).0));
    assert!(active.contains(&new));
    assert_eq!(
        scene.world.sample(source),
        Sample::Loaded(voxy_world::BlockStateId::AIR)
    );
    assert_eq!(
        publish_water_plan(
            &mut scene.world,
            &mut active,
            &mut dirty,
            vec![source],
            WaterPlan::Settled
        )
        .unwrap(),
        WaterPublication::Settled
    );
    assert!(active.contains(&new));
}

#[test]
fn autopilot_releases_existing_procedural_water_through_destruction() {
    let scene = build_procedural_scene(0x56_4f_58_59, 1).unwrap();
    let states = bootstrap_water_states(&scene.world).unwrap();
    let cells = terrain_water_cells(&scene, states).unwrap();
    let bed = natural_water_bed(&scene.world, states, &cells).unwrap();
    let source = VoxelPos {
        y: bed.y + 1,
        ..bed
    };
    let initial = scene.world.sample(source);
    assert!(matches!(initial, Sample::Loaded(block) if states.0.contains(&block)));
    let mut app = VoxyApp {
        world: Some(scene.world),
        water_states: Some(states),
        autopilot_water_bed: Some(bed),
        ..VoxyApp::default()
    };
    app.release_autopilot_water().unwrap();
    let world = app.world.as_ref().unwrap();
    assert_eq!(world.sample(source), initial);
    assert_eq!(
        world.sample(bed),
        Sample::Loaded(voxy_world::BlockStateId::AIR)
    );
    assert!(app.water_active.contains(&source));
    let WaterPlan::Transaction { edit, .. } = step_water(
        world,
        world.registry(),
        states,
        &app.water_active,
        EditSource::Simulation,
        WaterBudget::default(),
    )
    .unwrap() else {
        panic!("existing water must flow into the destroyed bed");
    };
    assert!(
        edit.writes
            .iter()
            .any(|write| write.pos == bed && states.0.contains(&write.block))
    );
}

#[test]
fn committed_edit_wakes_adjacent_water_without_duplicate_or_unloaded_work() {
    let mut scene = voxy_runtime::build_bootstrap_scene(42, 0).unwrap();
    let states = bootstrap_water_states(&scene.world).unwrap();
    let source = VoxelPos {
        x: 16,
        y: 18,
        z: 16,
    };
    let below = VoxelPos { y: 17, ..source };
    // Insert a support beneath a known full-water source and remove it using
    // the authoritative commit path, as an explosion would do.
    let support = scene
        .world
        .registry()
        .find(&ResourceKey::parse("voxy:stone").unwrap())
        .unwrap();
    scene
        .world
        .commit(voxy_world::EditTxn {
            source: EditSource::Editor,
            expected: vec![],
            writes: vec![voxy_world::VoxelWrite {
                pos: below,
                block: support,
            }],
        })
        .unwrap();
    let receipt = scene
        .world
        .commit(voxy_world::EditTxn {
            source: EditSource::Editor,
            expected: vec![],
            writes: vec![voxy_world::VoxelWrite {
                pos: below,
                block: voxy_world::BlockStateId::AIR,
            }],
        })
        .unwrap();
    let mut active = vec![];
    wake_water_after_edits(&scene.world, states, &receipt.inverse.writes, &mut active);
    assert!(active.contains(&source));
    let previous = active.clone();
    wake_water_after_edits(&scene.world, states, &receipt.inverse.writes, &mut active);
    assert_eq!(active, previous);
    wake_water_after_edits(
        &scene.world,
        states,
        &[voxy_world::VoxelWrite {
            pos: VoxelPos {
                x: i64::MAX,
                y: i64::MIN,
                z: i64::MAX,
            },
            block: voxy_world::BlockStateId::AIR,
        }],
        &mut active,
    );
    assert_eq!(active, previous);
    let plan = step_water(
        &scene.world,
        scene.world.registry(),
        states,
        &active,
        EditSource::Simulation,
        WaterBudget::default(),
    )
    .unwrap();
    let WaterPlan::Transaction { edit, .. } = plan else {
        panic!("woken water did not flow into removed support");
    };
    scene.world.commit(edit).unwrap();
    let Sample::Loaded(block) = scene.world.sample(below) else {
        panic!("destination unloaded");
    };
    assert!(states.0.contains(&block));
}

#[test]
fn procedural_spawn_has_clear_loaded_body_space() {
    let scene = build_procedural_scene(0x56_4f_58_59, 0).unwrap();
    let spawn = terrain_spawn(&scene.world).unwrap();
    for x in [15, 16] {
        for z in [15, 16] {
            for y in spawn.y..spawn.y + 2 {
                let Sample::Loaded(block) = scene.world.sample(VoxelPos { x, y, z }) else {
                    panic!("spawn body is not loaded");
                };
                assert_eq!(
                    scene.world.registry().get(block).unwrap().collision,
                    CollisionShape::Empty
                );
            }
        }
    }
}

#[test]
fn active_water_is_discovered_from_actual_blocks() {
    let scene = voxy_runtime::build_bootstrap_scene(42, 0).unwrap();
    let states = bootstrap_water_states(&scene.world).unwrap();
    let active = terrain_water_cells(&scene, states).unwrap();
    assert!(active.len() >= 81);
    for position in &active {
        let Sample::Loaded(block) = scene.world.sample(*position) else {
            panic!("water is not loaded");
        };
        assert!(states.0.contains(&block));
    }
    for x in 12..=20 {
        for z in 12..=20 {
            assert!(active.contains(&VoxelPos { x, y: 18, z }));
        }
    }
}
