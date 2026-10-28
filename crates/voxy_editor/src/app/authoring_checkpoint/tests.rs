use super::*;
fn limits() -> HistoryCheckpointLimits {
    HistoryCheckpointLimits {
        max_versions: 64,
        max_history_bytes: scene_limits::HISTORY_BYTES,
        max_scene_capacity: scene_limits::OBJECTS,
        max_archive_bytes: 128 * 1024 * 1024,
        max_edits: 4,
        max_draft_objects: scene_limits::OBJECTS * 2,
    }
}
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "voxy-authoring-history-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn view(app: &App) -> SceneCheckpointView<'_> {
    SceneCheckpointView::new(
        &app.scene,
        app.object_ids
            .iter()
            .cloned()
            .zip(app.instances.iter().copied()),
    )
    .unwrap()
}
fn capture(app: &App) -> String {
    app.authoring_history_checkpoint(&view(app), limits())
        .unwrap()
        .to_json(limits().max_archive_bytes)
        .unwrap()
}
fn restore(source: &App, target: &mut App) {
    let source_view = view(source);
    let registry = &source.authoring.authoring_project.registry;
    let simulation = voxy_scene::SceneSimulation::new(
        &source.scene,
        voxy_scene::SimulationLimits {
            fixed_step: 0.01,
            max_steps: 4,
            max_behaviors: 4,
            max_commands: 4,
        },
    )
    .unwrap();
    let scene_record = simulation
        .checkpoint_view(&source_view, registry, limits().max_archive_bytes)
        .unwrap();
    let (loaded, _) = scene_record
        .prepare(
            registry,
            &voxy_scene::BehaviorCheckpointRegistry::default(),
            voxy_scene::SimulationLimits {
                fixed_step: 0.01,
                max_steps: 4,
                max_behaviors: 4,
                max_commands: 4,
            },
            scene_limits::OBJECTS,
            limits().max_archive_bytes,
        )
        .unwrap();
    let saved = AuthoringHistoryCheckpoint::from_json(&capture(source), limits().max_archive_bytes)
        .unwrap();
    let prepared = saved
        .prepare(&loaded.checkpoint_view(), registry, limits())
        .unwrap();
    assert_eq!(prepared.scene_id, loaded.graph.identity());
    // The enclosing App transaction will own this commit. This test installs the
    // exact staged scene and authoring slots only after both preparations pass.
    if let Some(metadata) = prepared.metadata {
        metadata.install(&mut target.authoring);
    }
    target.scene = loaded.graph;
    target.object_ids = prepared.object_ids;
    target.instances = prepared.instances;
    target.selected = prepared.selected;
    target.authoring.next_object_id = prepared.next_object_id;
    target.authoring.history = prepared.history;
    target.drag = prepared.drag;
}
#[test]
fn real_app_history_order_selection_and_active_drag_rebind_and_continue_120_edits() {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("project"), "History").unwrap();
    let mut source = App::from_project(&path).unwrap();
    let mut target = App::from_project(&path).unwrap();
    for _ in 0..3 {
        source.edit_key(KeyCode::KeyD).unwrap();
    }
    source.instances.swap(0, 2);
    source.object_ids.swap(0, 2);
    source.selected = 1;
    source
        .authoring
        .history
        .as_mut()
        .unwrap()
        .commit_auxiliary(
            serde_json::json!({"settings":{"roughness":0.375}}),
            &source.authoring.authoring_project.registry,
        )
        .unwrap();
    source
        .begin_drag(Vec2::new(33.5, 42.25), Vec2::new(800., 600.))
        .unwrap();
    let selected = source.instances[source.selected];
    let mut local = source.scene.local(selected).unwrap();
    local.translation.x += 1.25;
    source.scene.set_local(selected, local).unwrap();
    source.drag.as_mut().unwrap().axis = DragAxis::X;
    source.drag.as_mut().unwrap().viewport_origin = Vec2::new(12.5, -0.0);
    let before = capture(&source);
    restore(&source, &mut target);
    assert_ne!(source.scene.identity(), target.scene.identity());
    assert_ne!(source.instances, target.instances);
    assert_eq!(source.object_ids, target.object_ids);
    assert_eq!(source.selected, target.selected);
    assert_eq!(before, capture(&target));
    source.finish_drag(true).unwrap();
    target.finish_drag(true).unwrap();
    assert_eq!(capture(&source), capture(&target));
    for step in 0..120 {
        for app in [&mut source, &mut target] {
            match step % 4 {
                0 => {
                    app.edit_key(KeyCode::ArrowLeft).unwrap();
                }
                1 => {
                    app.authoring.step_history(true).unwrap();
                    app.restore_authoring().unwrap();
                }
                2 => {
                    app.authoring.step_history(false).unwrap();
                    app.restore_authoring().unwrap();
                }
                _ => {
                    app.authoring
                        .history
                        .as_mut()
                        .unwrap()
                        .commit_auxiliary(
                            serde_json::json!({"settings":step}),
                            &app.authoring.authoring_project.registry,
                        )
                        .unwrap();
                }
            }
        }
        assert_eq!(capture(&source), capture(&target), "step {step}");
    }
    source.stop_workers().unwrap();
    target.stop_workers().unwrap();
}
#[test]
fn stale_drag_stays_stale_and_failed_preparation_preserves_the_original_app() {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("project"), "Stale").unwrap();
    let mut source = App::from_project(&path).unwrap();
    let mut target = App::from_project(&path).unwrap();
    source.edit_key(KeyCode::KeyD).unwrap();
    source
        .begin_drag(Vec2::new(20., 30.), Vec2::new(800., 600.))
        .unwrap();
    source
        .authoring
        .history
        .as_mut()
        .unwrap()
        .refresh_metadata(
            serde_json::json!({"revision":1}),
            &source.authoring.authoring_project.registry,
        )
        .unwrap();
    restore(&source, &mut target);
    assert!(source.finish_drag(true).is_err());
    assert!(target.finish_drag(true).is_err());
    let before = capture(&source);
    let saved = AuthoringHistoryCheckpoint::from_json(&before, limits().max_archive_bytes).unwrap();
    for case in 0..4 {
        let mut bad = saved.clone();
        match case {
            0 => bad.selected = bad.order.len(),
            1 => bad.order.push(bad.order[0].clone()),
            2 => bad.order[0] = ObjectId("missing-owner".into()),
            3 => bad.version = 4,
            _ => unreachable!(),
        }
        assert!(
            bad.prepare(
                &view(&source),
                &source.authoring.authoring_project.registry,
                limits()
            )
            .is_err()
        );
    }
    assert_eq!(capture(&source), before);
    source.stop_workers().unwrap();
    target.stop_workers().unwrap();
}

#[test]
fn cancelling_restored_drag_keeps_history_and_late_bad_gesture_does_not_consume_live_edit() {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("project"), "Cancel").unwrap();
    let mut source = App::from_project(&path).unwrap();
    let mut target = App::from_project(&path).unwrap();
    source.edit_key(KeyCode::KeyD).unwrap();
    let original = source.scene.local(source.instances[0]).unwrap();
    source
        .begin_drag(Vec2::new(12.25, 20.5), Vec2::new(800., 600.))
        .unwrap();
    let mut moved = original;
    moved.translation.x += 0.75;
    source.scene.set_local(source.instances[0], moved).unwrap();
    let before = capture(&source);
    let saved = AuthoringHistoryCheckpoint::from_json(&before, limits().max_archive_bytes).unwrap();
    for case in 0..5 {
        let mut bad = saved.clone();
        match case {
            0 => bad.drag.as_mut().unwrap().axis = 4,
            1 => bad.drag.as_mut().unwrap().viewport[0] = f32::NAN.to_bits(),
            2 => bad.drag.as_mut().unwrap().rotation = [0; 4],
            3 => bad.drag.as_mut().unwrap().node = ObjectId("missing-late-gesture-owner".into()),
            4 => bad.drag = None,
            _ => unreachable!(),
        }
        assert!(
            bad.prepare(
                &view(&source),
                &source.authoring.authoring_project.registry,
                limits()
            )
            .is_err()
        );
        assert_eq!(capture(&source), before);
    }
    restore(&source, &mut target);
    let history_before = source
        .authoring
        .history
        .as_ref()
        .unwrap()
        .checkpoint(&[], limits())
        .unwrap()
        .to_json(limits().max_archive_bytes)
        .unwrap();
    source.finish_drag(false).unwrap();
    target.finish_drag(false).unwrap();
    for app in [&source, &target] {
        assert_eq!(
            app.scene
                .local(app.instances[0])
                .unwrap()
                .translation
                .to_array()
                .map(f32::to_bits),
            original.translation.to_array().map(f32::to_bits)
        );
        assert_eq!(
            app.authoring
                .history
                .as_ref()
                .unwrap()
                .checkpoint(&[], limits())
                .unwrap()
                .to_json(limits().max_archive_bytes)
                .unwrap(),
            history_before
        );
    }
    assert_eq!(capture(&source), capture(&target));
    source.stop_workers().unwrap();
    target.stop_workers().unwrap();
}

#[test]
fn coordinated_play_and_history_prepare_share_scene_and_preserve_redo() {
    let temp = Temp::new();
    let project = project::create(&temp.0.join("source"), "Coordinated history").unwrap();
    let mut source = App::from_project(&project).unwrap();
    source.edit_key(KeyCode::KeyD).unwrap();
    source.edit_key(KeyCode::KeyD).unwrap();
    assert!(source.authoring.step_history(true).unwrap());
    source.restore_authoring().unwrap();
    source.toggle_play().unwrap();
    source.begin_restore_barrier(8 * 1024 * 1024).unwrap();
    let identities: BTreeMap<_, _> = source
        .instances
        .iter()
        .copied()
        .zip(source.object_ids.iter().cloned())
        .collect();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !source
        .poll_restore_barrier(|node| identities.get(&node).cloned())
        .unwrap()
    {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let registry = source.authoring.authoring_project.registry.clone();
    let mut play = crate::play_session::checkpoint::tests::limits();
    play.core.simulation = SimulationLimits {
        fixed_step: 1. / 60.,
        max_steps: 8,
        max_behaviors: 128,
        max_commands: 128,
    };
    play.core.scene_capacity = scene_limits::OBJECTS;
    play.max_angular_motions = 128;
    play.ui.max_events = 128;
    play.ui.max_handlers = 128;
    play.ui.max_commands_per_event = 128;
    let resources = crate::play_session::checkpoint::tests::resource_restore_limits();
    let ui = crate::ui_live::checkpoint::tests::limits();
    let mut saved = source
        .editor_checkpoint(
            &view(&source),
            &registry,
            "editor-history-v1",
            resources,
            play,
            ui,
            limits(),
        )
        .unwrap();
    let target = temp.0.join("target");
    std::fs::create_dir(&target).unwrap();
    let file_limits = crate::app::restore_barrier::editor::EditorFileLimits {
        max_bytes: 512 * 1024 * 1024,
        game: crate::app::restore_barrier::file::CoordinatedFileLimits {
            max_bytes: 384 * 1024 * 1024,
            max_game_bytes: resources.max_bytes,
            ui,
            max_audio_bytes: 8 * 1024 * 1024,
            max_input_bytes: 100000,
        },
        history: limits(),
    };
    let file = target.join("editor.voxyedr");
    saved.save_file(&file, file_limits).unwrap();
    let bytes = std::fs::read(&file).unwrap();
    saved = crate::app::restore_barrier::editor::EditorCheckpoint::read_file(&file, file_limits)
        .unwrap();
    assert_eq!(saved.to_bytes(file_limits).unwrap(), bytes);
    let mut small = file_limits;
    small.max_bytes = bytes.len() - 1;
    assert!(saved.save_file(&file, small).is_err());
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    assert!(
        crate::app::restore_barrier::editor::EditorCheckpoint::read_file(&file, small).is_err()
    );
    let mut damaged = bytes.clone();
    *damaged.last_mut().unwrap() ^= 1;
    assert!(
        crate::app::restore_barrier::editor::EditorCheckpoint::from_bytes(&damaged, file_limits)
            .is_err()
    );
    let mut bad_length = bytes.clone();
    bad_length[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(
        crate::app::restore_barrier::editor::EditorCheckpoint::from_bytes(&bad_length, file_limits)
            .is_err()
    );
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(
        crate::app::restore_barrier::editor::EditorCheckpoint::from_bytes(&trailing, file_limits)
            .is_err()
    );
    if let Some(directory) = std::env::var_os("VOXY_EDITOR_HISTORY_EVIDENCE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("editor.voxyedr"), &bytes).unwrap();
    }
    let behavior_codecs = source.behavior_checkpoints.clone();
    let ui_codecs = source.ui_action_checkpoints.clone();
    let options = || crate::app::restore_barrier::CoordinatedPrepare {
        compatibility: "editor-history-v1",
        registry: &registry,
        behaviors: &behavior_codecs,
        ui_actions: &ui_codecs,
        root: &target,
        resources,
        play,
        ui,
        max_audio_bytes: 8 * 1024 * 1024,
        restored_ui_cache: (0, 0, 0, 0),
    };
    let mut prepared = saved.prepare(options(), limits()).unwrap();
    let before = capture(&source);
    let order = saved.authoring.order.clone();
    saved.authoring.order[0] = ObjectId("missing-authoring-owner".into());
    assert!(saved.prepare(options(), limits()).is_err());
    saved.authoring.order = order;
    assert_eq!(capture(&source), before);
    assert!(source.restore_play_frozen());
    let bindings: Vec<_> = source
        .object_ids
        .iter()
        .cloned()
        .zip(source.instances.iter().copied())
        .collect();
    let origin = source
        .observe_coordinated_origin(
            &bindings,
            &registry,
            "editor-history-v1",
            resources,
            play,
            ui,
        )
        .unwrap();
    assert!(
        prepared
            .validate_origin(&mut source, &bindings, &registry, "editor-history-v1")
            .is_err()
    );
    let mut guarded = saved.prepare_guarded(options(), limits(), origin).unwrap();
    let target_scene = guarded.authoring.scene_id;
    guarded.authoring.scene_id = source.scene.identity();
    assert!(
        guarded
            .validate_origin(&mut source, &bindings, &registry, "editor-history-v1")
            .is_err()
    );
    guarded.authoring.scene_id = target_scene;
    guarded
        .validate_origin(&mut source, &bindings, &registry, "editor-history-v1")
        .unwrap();
    let source_scene = source.scene.change_token();
    let source_history = source.authoring.history.as_ref().unwrap().change_token();
    let original_source = source.authoring.authoring_source.take();
    assert!(original_source.is_some());
    assert!(
        guarded
            .validate_origin(&mut source, &bindings, &registry, "editor-history-v1")
            .is_err()
    );
    let different: voxy_scene::PrefabSceneDocument = serde_json::from_value(serde_json::json!({
        "version":1,"objects":[{"id":"archive-only","parent":null,"name":"new import",
            "active":true,"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{}}],
        "instances":[]
    })).unwrap();
    source.authoring.authoring_source = Some(
        source
            .authoring
            .authoring_project
            .prepare(different, None)
            .unwrap(),
    );
    assert!(
        guarded
            .validate_origin(&mut source, &bindings, &registry, "editor-history-v1")
            .is_err()
    );
    assert_eq!(source_scene, source.scene.change_token());
    assert_eq!(
        source_history,
        source.authoring.history.as_ref().unwrap().change_token()
    );
    source.authoring.authoring_source = original_source;
    guarded
        .validate_origin(&mut source, &bindings, &registry, "editor-history-v1")
        .unwrap();
    assert!(source.authoring.history.as_mut().unwrap().redo());
    assert!(
        guarded
            .validate_origin(&mut source, &bindings, &registry, "editor-history-v1")
            .is_err()
    );
    assert!(source.authoring.history.as_mut().unwrap().undo());
    // Reverting values does not resurrect the old native history epoch.
    assert!(
        guarded
            .validate_origin(&mut source, &bindings, &registry, "editor-history-v1")
            .is_err()
    );
    for worker in guarded.game.discard() {
        worker.join().unwrap();
    }
    let game = prepared.game.game.as_ref().unwrap();
    assert_eq!(
        prepared.authoring.scene_id,
        game.game.scene.graph.identity()
    );
    assert_ne!(prepared.authoring.scene_id, source.scene.identity());
    assert_eq!(prepared.authoring.object_ids, source.object_ids);
    assert_eq!(prepared.authoring.selected, source.selected);
    assert_ne!(prepared.authoring.instances, source.instances);
    for redo in [true, false, false, true, true] {
        let original = source.authoring.history.as_mut().unwrap();
        let restored = prepared.authoring.history.as_mut().unwrap();
        assert_eq!(
            if redo {
                original.redo()
            } else {
                original.undo()
            },
            if redo {
                restored.redo()
            } else {
                restored.undo()
            }
        );
        assert_eq!(
            serde_json::to_value(original.checkpoint(&[], limits()).unwrap()).unwrap(),
            serde_json::to_value(restored.checkpoint(&[], limits()).unwrap()).unwrap()
        );
    }
    for worker in prepared.game.discard() {
        worker.join().unwrap();
    }
    while !source.resume_restore_barrier().unwrap() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    source.stop_workers().unwrap();
}

#[test]
fn metadata_roundtrip_rebinds_seek_and_retains_original_file_receipt_without_io() {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("project"), "Metadata").unwrap();
    let mut source = App::from_project(&path).unwrap();
    let mut target = App::from_project(&path).unwrap();
    source.edit_key(KeyCode::KeyD).unwrap();
    source
        .authoring
        .settings_written
        .insert("material.settings".into(), [13; 32]);
    source.authoring.prefab_assets = vec![AssetId("props.prefab".into())];
    source.authoring.prefab_choice = 0;
    source.authoring.preview_seek = Some((source.instances[0], [-0., 2.5, 640.125, 480.]));
    let registry = source.authoring.authoring_project.registry.clone();
    let document = view(&source).capture(&registry).unwrap();
    let receipt = temp.0.join("external.scene");
    let bytes = document.to_json().unwrap().into_bytes();
    std::fs::write(&receipt, &bytes).unwrap();
    source.authoring.scene_path = Some(receipt.clone());
    source.authoring.scene_revision =
        Some(scene_revision::SceneRevision::written(&receipt, &document).unwrap());
    std::fs::remove_file(&receipt).unwrap();
    restore(&source, &mut target);
    assert_eq!(capture(&source), capture(&target));
    assert_ne!(
        source.authoring.preview_seek.unwrap().0,
        target.authoring.preview_seek.unwrap().0
    );
    assert_eq!(
        target.authoring.preview_seek.unwrap().1.map(f32::to_bits),
        [-0_f32, 2.5, 640.125, 480.].map(f32::to_bits)
    );
    assert!(
        target
            .authoring
            .scene_revision
            .as_ref()
            .unwrap()
            .validate(&receipt)
            .is_err()
    );
    std::fs::write(&receipt, &bytes).unwrap();
    target
        .authoring
        .scene_revision
        .as_ref()
        .unwrap()
        .validate(&receipt)
        .unwrap();
    let mut changed = bytes;
    changed.push(b' ');
    std::fs::write(&receipt, changed).unwrap();
    assert!(
        target
            .authoring
            .scene_revision
            .as_ref()
            .unwrap()
            .validate(&receipt)
            .is_err()
    );
    let mut legacy: serde_json::Value = serde_json::from_str(&capture(&source)).unwrap();
    legacy["version"] = 1.into();
    legacy.as_object_mut().unwrap().remove("metadata");
    let loaded =
        AuthoringHistoryCheckpoint::from_json(&legacy.to_string(), limits().max_archive_bytes)
            .unwrap();
    assert!(
        loaded
            .prepare(&view(&target), &registry, limits())
            .unwrap()
            .metadata
            .is_none()
    );
    legacy["version"] = 2.into();
    assert!(
        AuthoringHistoryCheckpoint::from_json(&legacy.to_string(), limits().max_archive_bytes)
            .unwrap()
            .prepare(&view(&target), &registry, limits())
            .is_err()
    );
    let mut mismatch: serde_json::Value = serde_json::from_str(&capture(&source)).unwrap();
    mismatch["metadata"]["scene_path"] = "different.scene".into();
    assert!(
        AuthoringHistoryCheckpoint::from_json(&mismatch.to_string(), limits().max_archive_bytes)
            .unwrap()
            .prepare(&view(&target), &registry, limits())
            .is_err()
    );
    source.stop_workers().unwrap();
    target.stop_workers().unwrap();
}

#[test]
fn imported_authoring_source_archive_restores_nested_dependencies_without_files() {
    let temp = Temp::new();
    let project_path = project::create(&temp.0.join("project"), "Source archive").unwrap();
    let mut source = App::from_project(&project_path).unwrap();
    let mut target = App::from_project(&project_path).unwrap();
    let provider = prefab_authoring::AuthoringProject::new(&temp.0, &InputRecipe::Direct).unwrap();
    let leaf = serde_json::json!({"version":1,"objects":[{
        "id":"leaf","parent":null,"name":"original leaf","active":true,
        "translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{}
    }],"instances":[]});
    std::fs::write(temp.0.join("leaf.prefab"), leaf.to_string()).unwrap();
    let nested = serde_json::json!({"version":1,"objects":[],"instances":[{
        "id":"nested","asset":"leaf.prefab","parent":null,"overrides":{}
    }]});
    std::fs::write(temp.0.join("nested.prefab"), nested.to_string()).unwrap();
    let root = serde_json::json!({"version":1,"objects":[],"instances":[{
        "id":"root","asset":"nested.prefab","parent":null,"overrides":{}
    }]});
    let root_path = temp.0.join("root.scene");
    std::fs::write(&root_path, root.to_string()).unwrap();
    source.authoring.authoring_source = Some(provider.load(&root_path).unwrap());
    let original = source.authoring.authoring_source.as_ref().unwrap();
    let payload = serde_json::to_value(original.value()).unwrap();
    let observations = original.inputs().observations().clone();
    assert_eq!(original.value().dependencies.len(), 2);
    for missing_dependency in [false, true] {
        let forged = original
            .to_checkpoint_bytes(
                |value| {
                    let mut value = value.clone();
                    if missing_dependency {
                        value.dependencies.remove("leaf.prefab");
                    } else {
                        value.expanded.objects[0].name = "different expansion".into();
                    }
                    serde_json::to_vec(&value)
                        .map_err(|e| voxy_assets::InputError::Read(e.to_string()))
                },
                voxy_assets::ImportedArchiveLimits {
                    max_inputs: 130,
                    max_input_bytes: 16 * 1024 * 1024,
                    max_archive_bytes: limits().max_archive_bytes,
                },
            )
            .unwrap();
        assert!(
            super::source::prepare(
                &forged,
                &source.authoring.authoring_project.registry,
                limits().max_archive_bytes
            )
            .is_err()
        );
    }

    for name in ["leaf.prefab", "nested.prefab", "root.scene"] {
        std::fs::remove_file(temp.0.join(name)).unwrap();
    }
    restore(&source, &mut target);
    let restored = target.authoring.authoring_source.as_ref().unwrap();
    assert_eq!(serde_json::to_value(restored.value()).unwrap(), payload);
    assert_eq!(restored.inputs().observations(), &observations);
    assert_eq!(capture(&source), capture(&target));
    let mut damaged: serde_json::Value = serde_json::from_str(&capture(&source)).unwrap();
    let archive = damaged["metadata"]["source"].as_array_mut().unwrap();
    *archive.last_mut().unwrap() = serde_json::json!(255);
    assert!(
        AuthoringHistoryCheckpoint::from_json(&damaged.to_string(), limits().max_archive_bytes)
            .unwrap()
            .prepare(
                &view(&target),
                &target.authoring.authoring_project.registry,
                limits()
            )
            .is_err()
    );
    // Legacy v2 does not erase an imported source it never archived.
    damaged = serde_json::from_str(&capture(&source)).unwrap();
    damaged["version"] = 2.into();
    damaged["metadata"]
        .as_object_mut()
        .unwrap()
        .remove("source");
    let legacy =
        AuthoringHistoryCheckpoint::from_json(&damaged.to_string(), limits().max_archive_bytes)
            .unwrap();
    legacy
        .prepare(
            &view(&target),
            &target.authoring.authoring_project.registry,
            limits(),
        )
        .unwrap()
        .metadata
        .unwrap()
        .install(&mut target.authoring);
    assert_eq!(
        serde_json::to_value(target.authoring.authoring_source.as_ref().unwrap().value()).unwrap(),
        payload
    );
    source.authoring.authoring_source = None;
    let explicit_absence =
        AuthoringHistoryCheckpoint::from_json(&capture(&source), limits().max_archive_bytes)
            .unwrap();
    explicit_absence
        .prepare(
            &view(&target),
            &target.authoring.authoring_project.registry,
            limits(),
        )
        .unwrap()
        .metadata
        .unwrap()
        .install(&mut target.authoring);
    assert!(target.authoring.authoring_source.is_none());
    source.stop_workers().unwrap();
    target.stop_workers().unwrap();
}

#[test]
fn idle_editor_file_restores_scene_and_history_without_play() {
    qualify_idle_editor_file(false);
}

#[test]
fn idle_editor_file_retains_preview_after_source_deletion() {
    qualify_idle_editor_file(true);
}

fn qualify_idle_editor_file(with_preview: bool) {
    let temp = Temp::new();
    let project = project::create(&temp.0.join("source"), "Coordinated history").unwrap();
    if with_preview {
        std::fs::write(
            temp.0.join("source/rig.glb"),
            include_bytes!("../../../../voxy_render/examples/assets/animated-triangle.glb"),
        )
        .unwrap();
        std::fs::write(
            temp.0.join("source/assets.json"),
            br#"{"version":1,"assets":[{"asset":"rig","source":"rig.glb"}]}"#,
        )
        .unwrap();
    }
    let mut source = App::from_project(&project).unwrap();
    source.edit_key(KeyCode::KeyD).unwrap();
    if with_preview {
        let deadline = Instant::now() + Duration::from_secs(15);
        while source
            .catalog
            .snapshot(&voxy_assets::AssetId("rig".into()))
            .is_none()
        {
            source.tick().unwrap();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        source.panel_action(panels::Action::Animation).unwrap();
    }
    source.edit_key(KeyCode::KeyD).unwrap();
    assert!(source.authoring.step_history(true).unwrap());
    source.restore_authoring().unwrap();
    let phase = f64::from_bits(0.25_f64.to_bits() + 1);
    if with_preview {
        source.preview_animation_phase(phase, false).unwrap();
    }
    assert!(source.play.playing.is_none() && source.play.simulation.is_none());
    source.begin_restore_barrier(8 * 1024 * 1024).unwrap();
    let identities: BTreeMap<_, _> = source
        .instances
        .iter()
        .copied()
        .zip(source.object_ids.iter().cloned())
        .collect();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !source
        .poll_restore_barrier(|node| identities.get(&node).cloned())
        .unwrap()
    {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let mut checkpoint_registry = crate::editor_component_registry().unwrap();
    crate::register_game_checkpoint_components(&mut checkpoint_registry).unwrap();
    let registry = Arc::new(checkpoint_registry);
    let mut play = crate::play_session::checkpoint::tests::limits();
    play.core.simulation = SimulationLimits {
        fixed_step: 1. / 60.,
        max_steps: 8,
        max_behaviors: 128,
        max_commands: 128,
    };
    play.core.scene_capacity = scene_limits::OBJECTS;
    play.max_angular_motions = 128;
    play.ui.max_events = 128;
    play.ui.max_handlers = 128;
    play.ui.max_commands_per_event = 128;
    let resources = crate::play_session::checkpoint::tests::resource_restore_limits();
    let ui = crate::ui_live::checkpoint::tests::limits();
    let mut saved = source
        .editor_checkpoint(
            &view(&source),
            &registry,
            "editor-history-v1",
            resources,
            play,
            ui,
            limits(),
        )
        .unwrap();
    let target = temp.0.join("target");
    std::fs::create_dir(&target).unwrap();
    let file_limits = crate::app::restore_barrier::editor::EditorFileLimits {
        max_bytes: 512 * 1024 * 1024,
        game: crate::app::restore_barrier::file::CoordinatedFileLimits {
            max_bytes: 384 * 1024 * 1024,
            max_game_bytes: resources.max_bytes,
            ui,
            max_audio_bytes: 8 * 1024 * 1024,
            max_input_bytes: 100000,
        },
        history: limits(),
    };
    let file = target.join("editor.voxyedr");
    saved.save_file(&file, file_limits).unwrap();
    let bytes = std::fs::read(&file).unwrap();
    saved = crate::app::restore_barrier::editor::EditorCheckpoint::read_file(&file, file_limits)
        .unwrap();
    assert_eq!(saved.to_bytes(file_limits).unwrap(), bytes);
    let behavior_codecs = source.behavior_checkpoints.clone();
    let ui_codecs = source.ui_action_checkpoints.clone();
    let options = || crate::app::restore_barrier::CoordinatedPrepare {
        compatibility: "editor-history-v1",
        registry: &registry,
        behaviors: &behavior_codecs,
        ui_actions: &ui_codecs,
        root: &target,
        resources,
        play,
        ui,
        max_audio_bytes: 8 * 1024 * 1024,
        restored_ui_cache: (0, 0, 0, 0),
    };
    let original_preview = source.authoring.animation_preview.as_ref().map(|preview| {
        preview
            .checkpoint(&view(&source), |_| Some("model:0".into()), 8 * 1024 * 1024)
            .unwrap()
            .to_json(8 * 1024 * 1024)
            .unwrap()
    });
    if with_preview {
        std::fs::remove_dir_all(temp.0.join("source")).unwrap();
    }
    let mut prepared = saved.prepare(options(), limits()).unwrap();
    let game = prepared.game.game.as_ref().unwrap();
    assert!(game.game.play.playing.is_none());
    assert!(game.game.play.simulation.is_none());
    let restored_preview = game.resources.preview.as_ref().map(|preview| {
        assert_eq!(preview.phase.to_bits(), phase.to_bits());
        assert_eq!(
            game.game.scene.identity(preview.owner),
            Some(&source.object_ids[source.selected])
        );
        preview
            .checkpoint(
                &game.game.scene.checkpoint_view(),
                |_| Some("model:0".into()),
                8 * 1024 * 1024,
            )
            .unwrap()
            .to_json(8 * 1024 * 1024)
            .unwrap()
    });
    assert_eq!(restored_preview, original_preview);
    assert_eq!(
        prepared.authoring.scene_id,
        game.game.scene.graph.identity()
    );
    assert_eq!(prepared.authoring.object_ids, source.object_ids);
    for redo in [true, false, false, true, true] {
        let original = source.authoring.history.as_mut().unwrap();
        let restored = prepared.authoring.history.as_mut().unwrap();
        assert_eq!(
            if redo {
                original.redo()
            } else {
                original.undo()
            },
            if redo {
                restored.redo()
            } else {
                restored.undo()
            }
        );
        assert_eq!(
            serde_json::to_value(original.checkpoint(&[], limits()).unwrap()).unwrap(),
            serde_json::to_value(restored.checkpoint(&[], limits()).unwrap()).unwrap()
        );
    }
    for worker in prepared.game.discard() {
        worker.join().unwrap();
    }
    if !with_preview {
        let resume_deadline = Instant::now() + Duration::from_secs(15);
        while !source.resume_restore_barrier().unwrap() {
            assert!(Instant::now() < resume_deadline);
            std::thread::yield_now();
        }
    }
    source.stop_workers().unwrap();
}
