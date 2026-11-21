use crate::*;
use voxy_gameplay::{BoxCollider, FollowCamera, PhysicsJoint, PhysicsJointKind};
use voxy_scene::FogVolume;

fn quad_fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../voxy_render/examples/assets/quad.obj")
}

#[test]
fn multi_selection_drag_moves_roots_once_and_undoes_and_deletes_atomically() {
    let mut app = App::new(&quad_fixture(), false).unwrap();
    app.edit_key(KeyCode::KeyD).unwrap(); // idx 1
    app.edit_key(KeyCode::KeyD).unwrap(); // idx 2
    assert_eq!(app.instances.len(), 3);

    let root = app.instances[0];
    let child = app.instances[1];
    let sibling = app.instances[2];
    app.scene.reparent(child, Some(root)).unwrap();
    app.scene
        .set_local(
            child,
            Transform {
                translation: Vec3::new(0.0, 2.0, 0.0),
                ..Transform::default()
            },
        )
        .unwrap();
    app.scene
        .set_local(
            sibling,
            Transform {
                translation: Vec3::new(-1.0, 0.0, 0.5),
                ..Transform::default()
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();
    let baseline = app.authoring_document().unwrap();

    // Select all 3 (root + child + sibling).
    app.modifiers = winit::keyboard::ModifiersState::CONTROL;
    app.edit_key(KeyCode::KeyA).unwrap();
    app.modifiers = winit::keyboard::ModifiersState::empty();
    assert_eq!(app.selected_indices(), vec![0, 1, 2]);

    // Only root & sibling are transform roots; child must stay fixed in root's local frame.
    let child_local_before = app.scene.local(child).unwrap().translation;
    let viewport = Vec2::splat(600.0);
    let cursor = viewport * 0.5;
    app.begin_drag(cursor, viewport).unwrap();
    assert_eq!(app.drag.as_ref().unwrap().companions.len(), 1);
    let vp = app.camera.matrix(viewport).unwrap();
    let project = |point| {
        let p = vp.project_point3(point);
        Vec2::new(p.x + 1.0, 1.0 - p.y) * viewport * 0.5
    };
    let origin_world = app.drag.as_ref().unwrap().world_origin;
    let screen_axis = project(origin_world + Vec3::X * 0.25) - project(origin_world);
    app.drag.as_mut().unwrap().axis = DragAxis::X;
    app.preview_drag(cursor + screen_axis * 2.0).unwrap();
    app.finish_drag(true).unwrap();

    assert!((app.scene.local(child).unwrap().translation - child_local_before).length() < 1e-5);
    assert!(
        (app.scene.local(root).unwrap().translation.x - baseline.objects[0].translation[0] - 0.5)
            .abs()
            < 1e-3
    );
    assert!(
        (app.scene.local(sibling).unwrap().translation.x
            - baseline.objects[2].translation[0]
            - 0.5)
            .abs()
            < 1e-3
    );

    // Single undo restores both root and sibling.
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), baseline);
    app.edit_key(KeyCode::KeyY).unwrap();
    assert_ne!(app.authoring_document().unwrap(), baseline);

    // Delete root & sibling (and covered child) in one step.
    app.modifiers = winit::keyboard::ModifiersState::CONTROL;
    app.edit_key(KeyCode::KeyA).unwrap();
    app.modifiers = winit::keyboard::ModifiersState::empty();
    app.edit_key(KeyCode::Delete).unwrap();
    assert!(app.instances.is_empty());
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.instances.len(), 3);
    app.stop_workers().unwrap();
}

#[test]
fn clipboard_copy_paste_remaps_internal_object_references_and_offsets_sequentially() {
    let mut app = App::new(&quad_fixture(), false).unwrap();
    app.edit_key(KeyCode::KeyD).unwrap(); // child camera/joint holder
    let root_node = app.instances[0];
    let child_node = app.instances[1];
    let root_id = app.object_ids[0].clone();
    app.scene.reparent(child_node, Some(root_node)).unwrap();
    app.scene
        .insert_component(
            child_node,
            FollowCamera {
                target: root_id.clone(),
                offset: [0.0, 2.0, 5.0],
                half_life: 0.2,
            },
        )
        .unwrap();
    app.scene
        .insert_component(
            child_node,
            PhysicsJoint {
                target: root_id.clone(),
                local_anchor: [0.0; 3],
                target_anchor: [0.0; 3],
                kind: PhysicsJointKind::Fixed {
                    relative_rotation: [0.0, 0.0, 0.0, 1.0],
                },
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();

    // Select both parent and child; copy must deduplicate subtree and paste once per Ctrl+V.
    app.modifiers = winit::keyboard::ModifiersState::CONTROL;
    app.edit_key(KeyCode::KeyA).unwrap();
    app.edit_key(KeyCode::KeyC).unwrap();
    app.edit_key(KeyCode::KeyV).unwrap();
    app.modifiers = winit::keyboard::ModifiersState::empty();

    assert_eq!(app.instances.len(), 4);
    let pasted_root_id = app.object_ids[2].clone();
    let pasted_child = app.instances[3];
    assert_eq!(
        app.scene.parent(pasted_child).unwrap(),
        Some(app.instances[2])
    );
    assert_eq!(
        app.scene
            .component::<FollowCamera>(pasted_child)
            .unwrap()
            .unwrap()
            .target,
        pasted_root_id
    );
    assert_eq!(
        app.scene
            .component::<PhysicsJoint>(pasted_child)
            .unwrap()
            .unwrap()
            .target,
        pasted_root_id
    );
    assert!(
        (app.scene.local(app.instances[2]).unwrap().translation.x
            - app.scene.local(app.instances[0]).unwrap().translation.x
            - 0.25)
            .abs()
            < 1e-6
    );
    assert_eq!(app.selected_indices(), vec![2]);

    // Second paste increments offset to 0.50 and undo removes one paste step at a time.
    app.modifiers = winit::keyboard::ModifiersState::CONTROL;
    app.edit_key(KeyCode::KeyV).unwrap();
    app.modifiers = winit::keyboard::ModifiersState::empty();
    assert_eq!(app.instances.len(), 6);
    assert!(
        (app.scene.local(app.instances[4]).unwrap().translation.x
            - app.scene.local(app.instances[0]).unwrap().translation.x
            - 0.50)
            .abs()
            < 1e-6
    );
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.instances.len(), 4);
    app.stop_workers().unwrap();
}

#[test]
fn gizmo_modes_and_snapping_support_world_grid_rotate_scale_and_checkpoint() {
    let mut app = App::new(&quad_fixture(), false).unwrap();
    app.scene
        .set_local(
            app.instances[0],
            Transform {
                translation: Vec3::new(0.13, 0.0, 0.5),
                ..Transform::default()
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();

    // Enable WorldGrid snap (N toggles snap, Backslash selects WorldGrid).
    app.edit_key(KeyCode::KeyN).unwrap();
    app.edit_key(KeyCode::Backslash).unwrap();
    assert!(app.gizmo_snap.enabled);
    assert_eq!(app.gizmo_snap.space, GizmoSnapSpace::WorldGrid);

    let viewport = Vec2::splat(600.0);
    let cursor = viewport * 0.5;
    app.begin_drag(cursor, viewport).unwrap();
    app.drag.as_mut().unwrap().axis = DragAxis::X;
    app.preview_drag(cursor + Vec2::new(15.0, 0.0)).unwrap();
    let snapped_x = app.scene.local(app.instances[0]).unwrap().translation.x;
    assert!(((snapped_x / 0.25).round() * 0.25 - snapped_x).abs() < 1e-5);
    app.finish_drag(false).unwrap();

    // Rotation mode (E) with 15-degree snap active.
    app.edit_key(KeyCode::KeyE).unwrap();
    assert_eq!(app.gizmo_mode, GizmoMode::Rotate);
    app.begin_drag(cursor, viewport).unwrap();
    app.drag.as_mut().unwrap().axis = DragAxis::Y;
    // 50 px horizontal delta * 0.015 = 0.75 rad -> snaps to multiple of pi/12 (0.2618 rad) -> 3 * 15° = 45° (pi/4).
    app.preview_drag(cursor + Vec2::new(50.0, 0.0)).unwrap();
    let (axis, angle) = app
        .scene
        .local(app.instances[0])
        .unwrap()
        .rotation
        .to_axis_angle();
    let _ = axis;
    let step_15 = std::f32::consts::FRAC_PI_6 * 0.5;
    let rounded = (angle / step_15).round() * step_15;
    assert!(
        (angle - rounded).abs() < 1e-4,
        "angle={angle} rounded={rounded}"
    );

    // Verify AuthoringHistoryCheckpoint round-trip preserves Rotate mode + companion drags + extra_selected.
    app.finish_drag(true).unwrap();
    app.edit_key(KeyCode::KeyD).unwrap();
    app.modifiers = winit::keyboard::ModifiersState::CONTROL;
    app.edit_key(KeyCode::KeyA).unwrap();
    app.modifiers = winit::keyboard::ModifiersState::empty();
    app.begin_drag(cursor, viewport).unwrap();
    let view = voxy_scene::SceneCheckpointView::new(
        &app.scene,
        app.object_ids
            .iter()
            .cloned()
            .zip(app.instances.iter().copied()),
    )
    .unwrap();
    let limits = voxy_scene::HistoryCheckpointLimits {
        max_versions: 64,
        max_history_bytes: scene_limits::HISTORY_BYTES,
        max_scene_capacity: scene_limits::OBJECTS,
        max_archive_bytes: 64 * 1024 * 1024,
        max_edits: 4,
        max_draft_objects: scene_limits::OBJECTS * 2,
    };
    let json = app
        .authoring_history_checkpoint(&view, limits)
        .unwrap()
        .to_json(limits.max_archive_bytes)
        .unwrap();
    let prepared = crate::app::authoring_checkpoint::AuthoringHistoryCheckpoint::from_json(
        &json,
        limits.max_archive_bytes,
    )
    .unwrap()
    .prepare(&view, &app.authoring.authoring_project.registry, limits)
    .unwrap();
    assert_eq!(prepared.extra_selected, BTreeSet::from([0]));
    assert_eq!(prepared.drag.as_ref().unwrap().mode, GizmoMode::Rotate);
    assert_eq!(prepared.drag.as_ref().unwrap().companions.len(), 1);
    app.finish_drag(false).unwrap();

    // Scale mode (T) with 0.25 snap active and undo.
    app.select_only(0);
    app.edit_key(KeyCode::KeyT).unwrap();
    assert_eq!(app.gizmo_mode, GizmoMode::Scale);
    app.begin_drag(cursor, viewport).unwrap();
    app.drag.as_mut().unwrap().axis = DragAxis::X;
    app.preview_drag(cursor + Vec2::new(62.0, 0.0)).unwrap();
    let scaled_x = app.scene.local(app.instances[0]).unwrap().scale.x;
    assert!((scaled_x - 1.5).abs() < 1e-5, "scaled_x={scaled_x}");
    app.finish_drag(true).unwrap();
    app.edit_key(KeyCode::KeyZ).unwrap();
    assert!((app.scene.local(app.instances[0]).unwrap().scale.x - 1.0).abs() < 1e-5);
    app.stop_workers().unwrap();
}

#[test]
fn focus_key_frames_full_hierarchy_non_mesh_volumes_and_multi_selection() {
    let mut app = App::new(&quad_fixture(), false).unwrap();
    app.camera.legacy = false;
    let model_id = app.id.clone().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.catalog.snapshot(&model_id).is_none() {
        assert!(Instant::now() < deadline, "quad model import timed out");
        app.tick().unwrap();
        std::thread::sleep(Duration::from_millis(5));
    }
    // Empty root + active distant child model + inactive huge child collider.
    let empty_root = app.scene.spawn(None, Transform::default()).unwrap();
    app.scene.set_name(empty_root, "EmptyRoot").unwrap();
    app.instances.push(empty_root);
    app.object_ids.push(ObjectId("object-1".into()));

    let model_child = app.instances[0];
    app.scene.reparent(model_child, Some(empty_root)).unwrap();
    app.scene
        .set_local(
            model_child,
            Transform {
                translation: Vec3::new(12.0, 1.0, 0.0),
                ..Transform::default()
            },
        )
        .unwrap();

    let inactive_child = app
        .scene
        .spawn(
            Some(empty_root),
            Transform {
                translation: Vec3::new(-40.0, 0.0, 0.0),
                ..Transform::default()
            },
        )
        .unwrap();
    app.scene.set_active(inactive_child, false).unwrap();
    app.scene
        .insert_component(
            inactive_child,
            BoxCollider {
                half_extents: [10.0, 10.0, 10.0],
            },
        )
        .unwrap();
    app.instances.push(inactive_child);
    app.object_ids.push(ObjectId("object-2".into()));
    app.commit_authoring().unwrap();

    // Selecting empty_root focuses the active model_child around x=12, ignoring inactive_child.
    let root_index = app.instances.iter().position(|&n| n == empty_root).unwrap();
    app.select_only(root_index);
    app.edit_key(KeyCode::KeyF).unwrap();
    assert!(
        (app.camera.target.x - 12.0).abs() < 1.5,
        "target={:?}",
        app.camera.target
    );

    // Node with only FogVolume (no ModelInstance): KeyF frames full [0, size] bounds.
    let fog_node = app
        .scene
        .spawn(
            None,
            Transform {
                translation: Vec3::new(-6.0, -2.0, 0.0),
                ..Transform::default()
            },
        )
        .unwrap();
    app.scene
        .insert_component(
            fog_node,
            FogVolume {
                size: [4.0, 2.0, 6.0],
                ..FogVolume::default()
            },
        )
        .unwrap();
    let fog_index = app.instances.len();
    app.instances.push(fog_node);
    app.object_ids.push(ObjectId("object-3".into()));
    app.select_only(fog_index);
    app.edit_key(KeyCode::KeyF).unwrap();
    assert!((app.camera.target - Vec3::new(-4.0, -1.0, 3.0)).length() < 1e-4);

    // Multi-selection framing both distant active model (x=12) and fog volume (x=-6..-2):
    app.select_only(0);
    app.toggle_selected_index(fog_index);
    app.edit_key(KeyCode::KeyF).unwrap();
    let vp = app.camera.matrix(Vec2::new(800.0, 600.0)).unwrap();
    for world_pt in [
        Vec3::new(12.0, 1.0, 0.0),
        Vec3::new(-6.0, -2.0, 0.0),
        Vec3::new(-2.0, 0.0, 6.0),
    ] {
        let ndc = vp.project_point3(world_pt);
        assert!(
            ndc.is_finite()
                && ndc.x.abs() <= 0.9
                && ndc.y.abs() <= 0.9
                && (0.0..=1.0).contains(&ndc.z),
            "point {world_pt:?} projected outside safe NDC: {ndc:?}"
        );
    }
    app.stop_workers().unwrap();
}
