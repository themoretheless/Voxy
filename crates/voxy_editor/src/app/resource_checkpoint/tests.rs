use super::*;
fn limits() -> import::catalog_checkpoint::Limits {
    import::catalog_checkpoint::Limits {
        assets: import::asset_checkpoint::EditorAssetCheckpointLimits::default(),
        inputs: voxy_assets::ImportedArchiveLimits {
            max_inputs: 34,
            max_input_bytes: 32 * 1024 * 1024,
            max_archive_bytes: 64 * 1024 * 1024,
        },
        max_bytes: 128 * 1024 * 1024,
        max_catalog_assets: 256,
        max_pending: 4,
        max_source_outputs: 256,
        max_source_edges: 8192,
    }
}
fn await_barrier(app: &mut App) {
    let until = Instant::now() + Duration::from_secs(10);
    while !app.poll_model_checkpoint().unwrap() {
        assert!(Instant::now() < until, "model checkpoint timed out");
        std::thread::yield_now();
    }
}
fn restore_limits() -> RestoreLimits {
    RestoreLimits {
        catalog: limits(),
        max_bytes: 256 * 1024 * 1024,
        max_outputs: 128,
        max_queued: 4096,
        max_sources: 257,
        max_file_bytes: crate::import::SOURCE_BYTES,
    }
}
fn complete_restored(prepared: &mut PreparedModelResources, id: &AssetId) {
    let ticket = prepared.resources.catalog.request(id.clone()).unwrap();
    prepared.imports.as_mut().unwrap().submit(&ticket).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(completion) = prepared.imports.as_mut().unwrap().try_result().unwrap() {
            prepared
                .resources
                .catalog
                .complete_observed(
                    &mut prepared.resources.sources,
                    &completion.ticket,
                    completion.result,
                )
                .unwrap();
            break;
        }
        assert!(Instant::now() < until, "restored import timed out");
        std::thread::yield_now();
    }
}
#[test]
fn app_fence_drains_import_and_scan_keeps_queue_and_resumes() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    assert!(app.capture_model_resources(limits()).is_err());
    app.tick().unwrap(); // Starts the real file-backed import worker.
    assert!(app.imports.as_ref().unwrap().is_busy());
    app.begin_model_checkpoint().unwrap();
    assert!(app.begin_model_checkpoint().is_err());
    await_barrier(&mut app);
    let id = app.id.clone().unwrap();
    let prior = app.catalog.snapshot(&id).unwrap();
    app.reload.insert(id.clone());
    app.last_scan = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
    for _ in 0..5 {
        app.tick().unwrap();
    }
    assert!(!app.scan_active);
    assert!(!app.imports.as_ref().unwrap().is_busy());
    assert!(app.reload.contains(&id));
    let saved = app.capture_model_resources(limits()).unwrap();
    assert_eq!(saved.queued, vec![id.clone()]);
    assert_eq!(saved.current, vec![id.clone()]);
    assert!(saved.manifest.is_none());
    let json = serde_json::to_vec(&saved).unwrap();
    let decoded: ModelResourcesCheckpoint = serde_json::from_slice(&json).unwrap();
    let restored = import::catalog_checkpoint::prepare(&decoded.catalog, limits()).unwrap();
    assert_eq!(
        restored
            .catalog
            .snapshot(&id)
            .unwrap()
            .inputs()
            .observations(),
        prior.inputs().observations()
    );
    assert!(app.resume_model_checkpoint().unwrap());
    app.tick().unwrap();
    assert!(app.imports.as_ref().unwrap().is_busy());
    assert!(!app.reload.contains(&id));
    app.begin_model_checkpoint().unwrap();
    await_barrier(&mut app);
    assert!(app.resume_model_checkpoint().unwrap());
    app.last_scan = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
    app.poll_sources().unwrap();
    assert!(app.scan_active);
    app.begin_model_checkpoint().unwrap();
    await_barrier(&mut app);
    assert!(!app.scan_active);
    assert!(
        app.capture_model_resources(limits())
            .unwrap()
            .watcher
            .is_some()
    );
    app.resume_model_checkpoint().unwrap();
    app.stop_workers().unwrap();
}
#[test]
fn cancelling_requested_snapshot_consumes_it_before_new_scans() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.begin_model_checkpoint().unwrap();
    app.watcher.as_mut().unwrap().request_checkpoint().unwrap();
    app.resource_checkpoint = Some(Barrier::Requested);
    let until = Instant::now() + Duration::from_secs(10);
    while !app.resume_model_checkpoint().unwrap() {
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    assert!(app.resource_checkpoint.is_none());
    app.last_scan = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
    app.poll_sources().unwrap();
    assert!(app.scan_active);
    app.stop_workers().unwrap();
}

#[test]
fn manifest_recipe_and_original_dependency_bytes_survive_source_deletion() {
    let root = std::env::temp_dir().join(format!("voxy-app-model-fence-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("quad.obj"),
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
    )
    .unwrap();
    std::fs::write(
        root.join("assets.json"),
        br#"{"version":1,"assets":[{"asset":"mesh","source":"quad.obj"}]}"#,
    )
    .unwrap();
    let mut app =
        App::from_manifest(&root.join("assets.json"), AssetId("mesh".into()), false).unwrap();
    app.tick().unwrap();
    app.begin_model_checkpoint().unwrap();
    await_barrier(&mut app);
    std::fs::remove_file(root.join("assets.json")).unwrap();
    std::fs::remove_file(root.join("quad.obj")).unwrap();
    let saved = app.capture_model_resources(limits()).unwrap();
    assert_eq!(saved.manifest.as_deref(), Some("assets.json"));
    assert_eq!(saved.outputs, vec![AssetId("mesh".into())]);
    let restored = import::catalog_checkpoint::prepare(&saved.catalog, limits()).unwrap();
    let model = restored.catalog.snapshot(&AssetId("mesh".into())).unwrap();
    let ids: Vec<_> = model
        .inputs()
        .observations()
        .keys()
        .map(|id| id.0.as_str())
        .collect();
    assert_eq!(ids, vec!["assets.json", "quad.obj"]);
    let live_original = app.catalog.snapshot(&AssetId("mesh".into())).unwrap();
    let bytes = saved.to_bytes(restore_limits().max_bytes).unwrap();
    import::asset_checkpoint::retain_evidence("model-resources.voxyemr", &bytes);
    std::fs::write(root.join("resources.voxyemr"), &bytes).unwrap();
    let bytes = std::fs::read(root.join("resources.voxyemr")).unwrap();
    let decoded = ModelResourcesCheckpoint::from_bytes(&bytes, restore_limits().max_bytes).unwrap();
    let mut prepared = decoded.prepare(&root, restore_limits()).unwrap();
    assert!(matches!(&prepared.recipe, InputRecipe::Manifest(id) if id.0 == "assets.json"));
    assert_eq!(prepared.current, Some(AssetId("mesh".into())));
    assert_eq!(prepared.outputs, app.available);
    assert_eq!(prepared.queued, app.reload);
    assert_eq!(prepared.delay, saved.delay);
    assert!(!prepared.imports.as_ref().unwrap().is_busy());
    let watcher = prepared.watcher.as_mut().unwrap();
    watcher.request_checkpoint().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(baseline) = watcher.try_checkpoint().unwrap() {
            assert_eq!(baseline, saved.watcher.as_ref().unwrap().0);
            break;
        }
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let id = AssetId("mesh".into());
    let original = prepared.resources.catalog.snapshot(&id).unwrap();
    complete_restored(&mut prepared, &id);
    assert!(matches!(
        prepared.resources.catalog.status(&id),
        Some(voxy_assets::AssetStatus::Failed(_))
    ));
    assert!(Arc::ptr_eq(
        &original,
        &prepared.resources.catalog.snapshot(&id).unwrap()
    ));
    // A later explicit reload uses the saved manifest recipe and current files.
    std::fs::write(root.join("new.obj"), "v 0 0 0\nv 2 0 0\nv 0 2 0\nf 1 2 3\n").unwrap();
    std::fs::write(
        root.join("assets.json"),
        br#"{"version":1,"assets":[{"asset":"mesh","source":"new.obj"}]}"#,
    )
    .unwrap();
    complete_restored(&mut prepared, &id);
    assert_eq!(
        prepared.resources.catalog.status(&id),
        Some(&voxy_assets::AssetStatus::Ready)
    );
    let replacement = prepared.resources.catalog.snapshot(&id).unwrap();
    assert!(
        replacement
            .inputs()
            .observations()
            .contains_key(&AssetId("new.obj".into()))
    );
    assert!(
        !replacement
            .inputs()
            .observations()
            .contains_key(&AssetId("quad.obj".into()))
    );
    assert!(Arc::ptr_eq(
        &live_original,
        &app.catalog.snapshot(&id).unwrap()
    ));
    prepared.imports.take().unwrap().close().join().unwrap();
    prepared.watcher.take().unwrap().close().join().unwrap();
    app.stop_workers().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_restore_rejects_and_stopped_workers_stay_stopped() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    app.begin_model_checkpoint().unwrap();
    await_barrier(&mut app);
    let mut saved = app.capture_model_resources(limits()).unwrap();
    let bytes = saved.to_bytes(restore_limits().max_bytes).unwrap();
    assert!(ModelResourcesCheckpoint::from_bytes(&bytes, bytes.len() - 1).is_err());
    let mut corrupt = bytes.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(ModelResourcesCheckpoint::from_bytes(&corrupt, restore_limits().max_bytes).is_err());
    let root = fixture.parent().unwrap();
    saved.outputs.push(saved.outputs[0].clone());
    assert!(saved.prepare(root, restore_limits()).is_err());
    saved.outputs.pop();
    saved.delay = Duration::from_secs(1);
    assert!(saved.prepare(root, restore_limits()).is_err());
    saved.delay = Duration::ZERO;
    saved.manifest = Some("../escape.json".into());
    assert!(saved.prepare(root, restore_limits()).is_err());
    saved.manifest = None;
    saved.watcher.as_mut().unwrap().0.cursor = usize::MAX;
    assert!(saved.prepare(root, restore_limits()).is_err());
    assert!(app.resource_checkpoint.is_some());
    assert_eq!(app.reload.len(), 1);
    app.stop_workers().unwrap();
    // The capture fence remains ready, but producer presence is captured now.
    let stopped = app.capture_model_resources(limits()).unwrap();
    assert!(!stopped.import_worker);
    assert!(stopped.watcher.is_none());
    let prepared = stopped.prepare(root, restore_limits()).unwrap();
    assert!(prepared.imports.is_none() && prepared.watcher.is_none());
}

#[test]
fn inactive_runtime_retains_old_model_after_catalog_replacement_and_resumes_120_frames() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/animated-triangle.glb");
    let mut app = App::new(&fixture, false).unwrap();
    app.tick().unwrap();
    app.begin_model_checkpoint().unwrap();
    await_barrier(&mut app);
    let id = app.id.clone().unwrap();
    let owner = app.instances[0];
    app.scene
        .insert_component(owner, ModelAnimation::default())
        .unwrap();
    let publication = app.catalog.snapshot(&id).unwrap();
    let old = publication.value().animated.as_ref().unwrap().clone();
    let old_models = BTreeMap::from([(id.clone(), old.clone())]);
    app.play.animations = app
        .play
        .animations
        .prepare(&app.scene, &old_models, 1. / 60.)
        .unwrap();
    app.scene.set_active(owner, false).unwrap();
    let mut replacement = publication.value().clone();
    replacement.animated = Some(Arc::new(old.as_ref().clone()));
    let ticket = app.catalog.request(id.clone()).unwrap();
    let replacement = voxy_assets::ImportInputs::new(0, 0)
        .finish(replacement, |_, _| unreachable!())
        .unwrap();
    app.catalog
        .complete_observed(&mut app.sources, &ticket, Ok(replacement))
        .unwrap();
    let saved_runtime = app
        .play
        .animations
        .checkpoint(
            &app.scene,
            |_| Some(voxy_scene::ObjectId("owner".into())),
            |model| Arc::ptr_eq(model, &old).then(|| "retired".into()),
            |clip, _| {
                old.animations
                    .iter()
                    .position(|c| Arc::ptr_eq(c, clip))
                    .map(|i| i.to_string())
            },
            4 * 1024 * 1024,
        )
        .unwrap();
    let saved = app.capture_model_resources(limits()).unwrap();
    let bytes = saved.to_bytes(restore_limits().max_bytes).unwrap();
    import::asset_checkpoint::retain_evidence("retired-animation-resources.voxyemr", &bytes);
    let saved = ModelResourcesCheckpoint::from_bytes(&bytes, restore_limits().max_bytes).unwrap();
    let prepared = import::catalog_checkpoint::prepare(&saved.catalog, limits()).unwrap();
    assert_eq!(prepared.models.len(), 1);
    let restored_model = prepared.models[0].clone();
    let current = prepared.catalog.snapshot(&id).unwrap();
    assert!(!Arc::ptr_eq(
        &restored_model,
        current.value().animated.as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(
        &restored_model.animations[0],
        &current.value().animated.as_ref().unwrap().animations[0]
    ));
    let mut restored = saved_runtime
        .prepare(
            &app.scene,
            |_| Some(owner),
            |key| (key == "retired").then(|| restored_model.clone()),
            |key| {
                key.parse::<usize>().ok().and_then(|i| {
                    restored_model.animations.get(i).cloned().map(|clip| {
                        (
                            clip,
                            restored_model.weight_clips().get(i).cloned().map(Arc::new),
                        )
                    })
                })
            },
            4 * 1024 * 1024,
        )
        .unwrap();
    let restored_current = current.value().animated.as_ref().unwrap().clone();
    let original_current = app
        .catalog
        .snapshot(&id)
        .unwrap()
        .value()
        .animated
        .as_ref()
        .unwrap()
        .clone();
    let current_models = BTreeMap::from([(id.clone(), original_current.clone())]);
    let models = BTreeMap::from([(id, restored_current.clone())]);
    assert!(Arc::ptr_eq(
        &restored.checkpoint_model_roots()[0],
        &restored_model
    ));
    assert_eq!(
        app.play
            .animations
            .frame(owner, &old)
            .unwrap()
            .pose
            .checkpoint(),
        restored
            .frame(owner, &restored_model)
            .unwrap()
            .pose
            .checkpoint()
    );
    app.scene.set_active(owner, true).unwrap();
    for _ in 0..120 {
        app.play.animations = app
            .play
            .animations
            .prepare(&app.scene, &current_models, 1. / 120.)
            .unwrap();
        restored = restored.prepare(&app.scene, &models, 1. / 120.).unwrap();
        assert_eq!(
            app.play
                .animations
                .frame(owner, &original_current)
                .unwrap()
                .pose
                .checkpoint(),
            restored
                .frame(owner, &restored_current)
                .unwrap()
                .pose
                .checkpoint()
        );
    }
    app.stop_workers().unwrap();
}

#[test]
fn reloaded_clip_fade_restores_from_resource_file_with_native_source_resolver() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/animated-triangle.glb");
    let mut app = App::new(&fixture, false).unwrap();
    app.tick().unwrap();
    app.begin_model_checkpoint().unwrap();
    await_barrier(&mut app);
    let id = app.id.clone().unwrap();
    let owner = app.instances[0];
    app.scene
        .insert_component(
            owner,
            ModelAnimation {
                transition_seconds: 0.5,
                ..Default::default()
            },
        )
        .unwrap();
    let old = app.catalog.snapshot(&id).unwrap();
    let old_model = old.value().animated.as_ref().unwrap().clone();
    let old_models = BTreeMap::from([(id.clone(), old_model.clone())]);
    app.play.animations = app
        .play
        .animations
        .prepare(&app.scene, &old_models, 0.1)
        .unwrap();
    let mut model = old_model.as_ref().clone();
    let mut tracks = vec![voxy_animation::JointTrack::default(); model.skeleton.joints().len()];
    tracks[0].translations = vec![voxy_animation::Vec3Key {
        time: 0.,
        value: Vec3::new(8., 0., 0.),
    }];
    model.animations[0] = Arc::new(
        voxy_animation::AnimationClip::new(
            old_model.animations[0].name(),
            1.,
            voxy_animation::Playback::Loop,
            tracks,
            &model.skeleton,
        )
        .unwrap(),
    );
    let model = Arc::new(model);
    let mut asset = old.value().clone();
    asset.animated = Some(model.clone());
    let ticket = app.catalog.request(id.clone()).unwrap();
    let asset = voxy_assets::ImportInputs::new(0, 0)
        .finish(asset, |_, _| unreachable!())
        .unwrap();
    app.catalog
        .complete_observed(&mut app.sources, &ticket, Ok(asset))
        .unwrap();
    let secondary_id = AssetId("secondary".into());
    let second = app.scene.spawn(None, Default::default()).unwrap();
    app.scene
        .insert_component(
            second,
            ModelInstance {
                asset: secondary_id.clone(),
            },
        )
        .unwrap();
    app.scene
        .insert_component(second, ModelAnimation::default())
        .unwrap();
    let ticket = app.catalog.request(secondary_id.clone()).unwrap();
    let secondary = voxy_assets::ImportInputs::new(0, 0)
        .finish(old.value().clone(), |_, _| unreachable!())
        .unwrap();
    app.catalog
        .complete_observed(&mut app.sources, &ticket, Ok(secondary))
        .unwrap();
    app.available.insert(secondary_id.clone());
    let models = BTreeMap::from([
        (id.clone(), model.clone()),
        (secondary_id.clone(), old_model.clone()),
    ]);
    app.play.animations = app
        .play
        .animations
        .prepare(&app.scene, &models, 0.05)
        .unwrap();
    let frame = app.play.animations.frame(owner, &model).unwrap();
    assert!(frame.transition_weight > 0. && frame.transition_weight < 1.);
    app.resume_model_checkpoint().unwrap();
    assert!(
        app.capture_model_animation_checkpoint(
            |_| panic!("queried live runtime before save fence"),
            limits(),
            4 * 1024 * 1024
        )
        .is_err()
    );
    app.begin_model_checkpoint().unwrap();
    await_barrier(&mut app);
    let source_roots = app
        .play
        .animations
        .checkpoint_sources(4 * 1024 * 1024, 4096)
        .unwrap();
    assert!(
        source_roots
            .clips
            .iter()
            .any(|c| Arc::ptr_eq(c, &old_model.animations[0]))
    );
    assert!(
        source_roots
            .clips
            .iter()
            .any(|c| Arc::ptr_eq(c, &model.animations[0]))
    );
    let saved = app
        .capture_model_animation_checkpoint(
            |node| {
                Some(voxy_scene::ObjectId(
                    if node == owner {
                        "owner"
                    } else if node == second {
                        "second"
                    } else {
                        return None;
                    }
                    .into(),
                ))
            },
            limits(),
            4 * 1024 * 1024,
        )
        .unwrap();
    assert_eq!(saved.resources.playback_sources, source_roots.bindings);
    let bytes = saved.to_bytes(restore_limits().max_bytes).unwrap();
    import::asset_checkpoint::retain_evidence("active-fade-state.voxymac", &bytes);
    let file =
        std::env::temp_dir().join(format!("voxy-active-fade-{}.voxymac", std::process::id()));
    std::fs::write(&file, &bytes).unwrap();
    let bytes = std::fs::read(&file).unwrap();
    std::fs::remove_file(file).unwrap();
    let mut decoded =
        ModelAnimationCheckpoint::from_bytes(&bytes, restore_limits().max_bytes).unwrap();
    decoded
        .resources
        .playback_sources
        .push(decoded.resources.playback_sources[0].clone());
    assert!(
        decoded
            .prepare(
                &app.scene,
                |_| Some(owner),
                fixture.parent().unwrap(),
                restore_limits(),
                4 * 1024 * 1024
            )
            .is_err()
    );
    decoded.resources.playback_sources.pop();
    // Stable IDs may reorder, but two owners may never collapse into one node.
    assert!(
        decoded
            .prepare(
                &app.scene,
                |_| Some(owner),
                fixture.parent().unwrap(),
                restore_limits(),
                4 * 1024 * 1024
            )
            .is_err()
    );
    let mut target = SceneGraph::new(8);
    let new_second = target
        .spawn(None, app.scene.local(second).unwrap())
        .unwrap();
    let new_owner = target.spawn(None, app.scene.local(owner).unwrap()).unwrap();
    assert_ne!(owner, new_owner);
    for (a, b) in [(owner, new_owner), (second, new_second)] {
        target
            .insert_component(
                b,
                app.scene
                    .component::<ModelInstance>(a)
                    .unwrap()
                    .unwrap()
                    .clone(),
            )
            .unwrap();
        target
            .insert_component(
                b,
                app.scene
                    .component::<ModelAnimation>(a)
                    .unwrap()
                    .unwrap()
                    .clone(),
            )
            .unwrap();
    }
    let unit = decoded
        .prepare(
            &target,
            |key| match key.0.as_str() {
                "owner" => Some(new_owner),
                "second" => Some(new_second),
                _ => None,
            },
            fixture.parent().unwrap(),
            restore_limits(),
            4 * 1024 * 1024,
        )
        .unwrap();
    let mut prepared = unit.resources;
    let mut restored = unit.animations;
    let restored_model = prepared
        .resources
        .catalog
        .snapshot(&id)
        .unwrap()
        .value()
        .animated
        .as_ref()
        .unwrap()
        .clone();
    let restored_second = prepared
        .resources
        .catalog
        .snapshot(&secondary_id)
        .unwrap()
        .value()
        .animated
        .as_ref()
        .unwrap()
        .clone();
    let restored_models = BTreeMap::from([
        (id, restored_model.clone()),
        (secondary_id, restored_second.clone()),
    ]);
    for _ in 0..120 {
        app.play.animations = app
            .play
            .animations
            .prepare(&app.scene, &models, 1. / 120.)
            .unwrap();
        restored = restored
            .prepare(&target, &restored_models, 1. / 120.)
            .unwrap();
        let a = app.play.animations.frame(owner, &model).unwrap();
        let b = restored.frame(new_owner, &restored_model).unwrap();
        assert_eq!(a.pose.checkpoint(), b.pose.checkpoint());
        assert_eq!(
            app.play
                .animations
                .frame(second, &old_model)
                .unwrap()
                .pose
                .checkpoint(),
            restored
                .frame(new_second, &restored_second)
                .unwrap()
                .pose
                .checkpoint()
        );
        assert_eq!(a.transition_weight.to_bits(), b.transition_weight.to_bits());
        assert_eq!(
            app.play.animations.clip_phase(owner).unwrap().to_bits(),
            restored.clip_phase(new_owner).unwrap().to_bits()
        );
    }
    assert_eq!(
        restored
            .frame(new_owner, &restored_model)
            .unwrap()
            .transition_weight,
        1.
    );
    prepared.imports.take().unwrap().close().join().unwrap();
    prepared.watcher.take().unwrap().close().join().unwrap();
    app.stop_workers().unwrap();
}

#[test]
fn combined_bundle_resolves_gpu_material_roles_and_rejects_missing_role_metadata() {
    exercise_combined_bundle(false);
}
#[test]
#[ignore = "requires real GPU; combined native model/runtime restore transaction"]
fn combined_bundle_stages_original_cpu_resources_runtime_and_gpu_together() {
    exercise_combined_bundle(true);
}
fn exercise_combined_bundle(with_gpu: bool) {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/animated-triangle.glb");
    let mut app = App::new(&fixture, false).unwrap();
    app.tick().unwrap();
    app.begin_model_checkpoint().unwrap();
    await_barrier(&mut app);
    let id = app.id.clone().unwrap();
    let owner = app.instances[0];
    let previous = app.catalog.snapshot(&id).unwrap();
    let mut value = previous.value().clone();
    value.images.push(
        voxy_render::ImageAsset::from_rgba(1, 1, vec![0, 255, 0, 255], Default::default()).unwrap(),
    );
    let value = voxy_assets::ImportInputs::new(0, 0)
        .finish(value, |_, _| unreachable!())
        .unwrap();
    let ticket = app.catalog.request(id.clone()).unwrap();
    app.catalog
        .complete_observed(&mut app.sources, &ticket, Ok(value))
        .unwrap();
    let current = app.catalog.snapshot(&id).unwrap();
    app.scene
        .insert_component(owner, ModelAnimation::default())
        .unwrap();
    let models = BTreeMap::from([(
        id.clone(),
        current.value().animated.as_ref().unwrap().clone(),
    )]);
    app.play.animations = app
        .play
        .animations
        .prepare(&app.scene, &models, 0.05)
        .unwrap();

    let roots = crate::gpu_source_bindings::LiveSources {
        models: vec![(id.clone(), current.clone())],
        owners: vec![(owner, previous)],
    };
    let mut saved = app
        .capture_model_animation_with_sources(
            |node| (node == owner).then(|| voxy_scene::ObjectId("actor".into())),
            limits(),
            4 * 1024 * 1024,
            Some(roots),
        )
        .unwrap();
    // Explicit CPU fixture for an accepted render frame, composed through the native envelope.
    let model = current.value().animated.as_ref().unwrap();
    let frame = model.bind_frame().unwrap();
    saved.accepted_frames = Some(
        serde_json::from_value(serde_json::json!({
            "version": 1,
            "owners": [{
                "id": voxy_scene::ObjectId("actor".into()),
                "settings": ModelAnimation::default(),
                "ticks": 731,
                "frame": frame.checkpoint(&model.skeleton, 4 * 1024 * 1024).unwrap(),
            }],
        }))
        .unwrap(),
    );
    let bytes = saved.to_bytes(restore_limits().max_bytes).unwrap();
    import::asset_checkpoint::retain_evidence("gpu-source-role-bundle.voxymac", &bytes);
    let mut saved =
        ModelAnimationCheckpoint::from_bytes(&bytes, restore_limits().max_bytes).unwrap();
    let mut target = SceneGraph::new(4);
    target.spawn(None, Default::default()).unwrap();
    let new_owner = target.spawn(None, Default::default()).unwrap();
    target
        .insert_component(new_owner, ModelInstance { asset: id.clone() })
        .unwrap();
    target
        .insert_component(new_owner, ModelAnimation::default())
        .unwrap();

    let mut unit = saved
        .prepare(
            &target,
            |key| (key.0 == "actor").then_some(new_owner),
            fixture.parent().unwrap(),
            restore_limits(),
            4 * 1024 * 1024,
        )
        .unwrap();
    let accepted = &unit.accepted_frames.as_ref().unwrap()[&new_owner];
    assert_eq!(accepted.ticks, 731);
    assert_eq!(accepted.frame.skin_matrices, frame.skin_matrices);
    let roles = unit.gpu.as_ref().unwrap();
    assert_eq!(roles.scene, target.identity());
    assert!(Arc::ptr_eq(
        &roles.models[&id],
        &unit.resources.resources.catalog.snapshot(&id).unwrap()
    ));
    assert!(!Arc::ptr_eq(&roles.models[&id], &roles.owners[&new_owner]));
    assert_eq!(roles.owners[&new_owner].value().images.len(), 0);
    assert_eq!(roles.models[&id].value().images.len(), 1);
    assert!(
        saved
            .prepare(
                &target,
                |_| None,
                fixture.parent().unwrap(),
                restore_limits(),
                4 * 1024 * 1024
            )
            .is_err()
    );
    saved.gpu = None;
    assert!(
        saved
            .prepare(
                &target,
                |_| Some(new_owner),
                fixture.parent().unwrap(),
                restore_limits(),
                4 * 1024 * 1024
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(&current, &app.catalog.snapshot(&id).unwrap()));
    if with_gpu {
        let instance = voxy_render::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
        println!("CPU/GPU BUNDLE {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let cache = gpu_model::ResidencyCache::default();
        let target_gpu = || gpu_model::batch::UploadTarget {
            renderer: &renderer,
            device: &device,
            queue: &queue,
            cache: &cache,
            live_geometry: 0,
            replaced_geometry: 0,
        };
        let expected_catalog = unit.resources.resources.catalog.snapshot(&id).unwrap();
        let expected_owner = unit.gpu.as_ref().unwrap().owners[&new_owner].clone();
        let pending = unit.stage_gpu_for_app(&app, &target, target_gpu()).unwrap();
        let mut ready = pollster::block_on(pending.validate()).unwrap();
        ready
            .gpu
            .validate_destination(&target, &target_gpu())
            .unwrap();
        ready.validate_scene_authoring(&app).unwrap();
        ready.validate_model_origin(&app).unwrap();
        assert!(Arc::ptr_eq(
            &ready.resources.resources.catalog.snapshot(&id).unwrap(),
            &expected_catalog
        ));
        assert!(Arc::ptr_eq(
            ready.gpu.models[&id].publication.as_ref().unwrap(),
            &expected_catalog
        ));
        assert!(Arc::ptr_eq(
            &ready.gpu.animated.source_publication_owners().unwrap()[0].1,
            &expected_owner
        ));
        assert_eq!(
            ready
                .gpu
                .animated
                .pose_signature(new_owner)
                .unwrap()
                .unwrap(),
            frame
                .skin_matrices
                .iter()
                .map(|m| m.to_cols_array().map(f32::to_bits))
                .collect::<Vec<_>>()
        );
        assert_eq!(ready.animations.checkpoint_model_roots().len(), 1);
        let restored_model = ready.resources.resources.models[0].clone();
        assert!(Arc::ptr_eq(
            &restored_model,
            expected_catalog.value().animated.as_ref().unwrap()
        ));
        let accepted_pose = ready
            .gpu
            .animated
            .pose_signature(new_owner)
            .unwrap()
            .unwrap();
        let simulated = ready.animations.frame(new_owner, &restored_model).unwrap();
        assert_ne!(
            accepted_pose,
            simulated
                .skin_matrices
                .iter()
                .map(|m| m.to_cols_array().map(f32::to_bits))
                .collect::<Vec<_>>()
        );
        let restored_models = BTreeMap::from([(id.clone(), restored_model.clone())]);
        for _ in 0..120 {
            app.play.animations = app
                .play
                .animations
                .prepare(&app.scene, &models, 1. / 120.)
                .unwrap();
            ready.animations = ready
                .animations
                .prepare(&target, &restored_models, 1. / 120.)
                .unwrap();
            assert_eq!(
                app.play.animations.clip_phase(owner).unwrap().to_bits(),
                ready.animations.clip_phase(new_owner).unwrap().to_bits()
            );
            let original = app.play.animations.frame(owner, model).unwrap();
            let restored = ready.animations.frame(new_owner, &restored_model).unwrap();
            assert_eq!(original.skin_matrices, restored.skin_matrices);
            assert_eq!(original.properties, restored.properties);
        }
        assert_eq!(
            ready
                .gpu
                .animated
                .pose_signature(new_owner)
                .unwrap()
                .unwrap(),
            accepted_pose
        );

        assert!(Arc::ptr_eq(&current, &app.catalog.snapshot(&id).unwrap()));
        ready.validate_scene_authoring(&app).unwrap();
        assert!(ready.validate_model_origin(&app).is_err());
        app.scene
            .set_name(owner, "changed during preparation")
            .unwrap();
        assert!(ready.validate_scene_authoring(&app).is_err());
        ready
            .resources
            .imports
            .take()
            .unwrap()
            .close()
            .join()
            .unwrap();
        ready
            .resources
            .watcher
            .take()
            .unwrap()
            .close()
            .join()
            .unwrap();
        app.stop_workers().unwrap();
        return;
    }
    unit.resources
        .imports
        .take()
        .unwrap()
        .close()
        .join()
        .unwrap();
    unit.resources
        .watcher
        .take()
        .unwrap()
        .close()
        .join()
        .unwrap();
    app.stop_workers().unwrap();
}

#[test]
fn removed_manifest_model_keeps_authoring_history_and_archived_last_good_owner() {
    let root = std::env::temp_dir().join(format!(
        "voxy-removed-model-save-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("mesh.obj"),
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
    )
    .unwrap();
    std::fs::write(
        root.join("assets.json"),
        br#"{"version":1,"assets":[{"asset":"mesh","source":"mesh.obj"}]}"#,
    )
    .unwrap();
    let id = AssetId("mesh".into());
    let mut app = App::from_manifest(&root.join("assets.json"), id.clone(), false).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while app.catalog.snapshot(&id).is_none() {
        app.tick().unwrap();
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    app.begin_model_checkpoint().unwrap();
    await_barrier(&mut app);
    app.resume_model_checkpoint().unwrap();
    let original = app.authoring_document().unwrap();
    std::fs::write(root.join("assets.json"), br#"{"version":1,"assets":[]}"#).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while app.available.contains(&id) {
        app.tick().unwrap();
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(app.available.is_empty());
    let last_good = app.catalog.snapshot(&id).unwrap();
    app.validate_authoring_document(&original).unwrap();
    let mut unknown = original.clone();
    unknown.objects[0].components.insert(
        "editor.model.v1".into(),
        serde_json::json!("never-imported"),
    );
    assert!(app.validate_authoring_document(&unknown).is_err());
    app.edit_key(KeyCode::ArrowRight).unwrap();
    let edited = app.authoring_document().unwrap();
    assert_ne!(edited, original);
    app.history_key(KeyCode::KeyZ).unwrap();
    assert_eq!(app.authoring_document().unwrap(), original);
    app.history_key(KeyCode::KeyY).unwrap();
    assert_eq!(app.authoring_document().unwrap(), edited);
    app.authoring.scene_path = Some(root.join("scene.json"));
    app.save_authoring().unwrap();
    app.edit_key(KeyCode::ArrowRight).unwrap();
    assert_ne!(app.authoring_document().unwrap(), edited);
    app.load_authoring().unwrap();
    assert_eq!(app.authoring_document().unwrap(), edited);
    assert!(Arc::ptr_eq(&last_good, &app.catalog.snapshot(&id).unwrap()));
    app.begin_model_checkpoint().unwrap();
    await_barrier(&mut app);
    let saved = app.capture_model_resources(limits()).unwrap();
    assert!(saved.outputs.is_empty());
    let bytes = saved.to_bytes(restore_limits().max_bytes).unwrap();
    std::fs::remove_file(root.join("assets.json")).unwrap();
    std::fs::remove_file(root.join("mesh.obj")).unwrap();
    let mut decoded =
        ModelResourcesCheckpoint::from_bytes(&bytes, restore_limits().max_bytes).unwrap();
    decoded.current = vec![AssetId("never-imported".into())];
    assert!(decoded.prepare(&root, restore_limits()).is_err());
    decoded.current = vec![id.clone()];
    let mut restored = decoded.prepare(&root, restore_limits()).unwrap();
    assert!(restored.outputs.is_empty());
    assert_eq!(restored.current.as_ref(), Some(&id));
    let asset = restored.resources.catalog.snapshot(&id).unwrap();
    assert!(!Arc::ptr_eq(&last_good, &asset));
    assert_eq!(
        asset.value().mesh.vertices(),
        last_good.value().mesh.vertices()
    );
    if let Some(worker) = restored.imports.take() {
        worker.close().join().unwrap();
    }
    if let Some(worker) = restored.watcher.take() {
        worker.close().join().unwrap();
    }
    app.stop_workers().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

fn watcher_policy_fixture() -> (App, ModelResourcesCheckpoint, std::path::PathBuf) {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    let id = app.id.clone().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while app.catalog.snapshot(&id).is_none() {
        app.tick().unwrap();
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    app.begin_model_checkpoint().unwrap();
    await_barrier(&mut app);
    let saved = app.capture_model_resources(limits()).unwrap();
    (app, saved, fixture.parent().unwrap().to_owned())
}
fn close_policy_candidate(mut candidate: PreparedModelResources) {
    if let Some(worker) = candidate.imports.take() {
        worker.close().join().unwrap();
    }
    if let Some(worker) = candidate.watcher.take() {
        worker.close().join().unwrap();
    }
}
#[test]
fn restore_rejects_watcher_policy_that_cannot_cover_accepted_model_sources() {
    let (mut app, saved, root) = watcher_policy_fixture();
    let original = saved.to_bytes(restore_limits().max_bytes).unwrap();
    let absent_root = std::env::temp_dir().join(format!(
        "voxy-absent-policy-root-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    assert!(!absent_root.exists());
    for too_few_files in [true, false] {
        let mut bad =
            ModelResourcesCheckpoint::from_bytes(&original, restore_limits().max_bytes).unwrap();
        let watcher = &mut bad.watcher.as_mut().unwrap().0;
        if too_few_files {
            watcher.max_sources = 0;
            watcher.sources.clear();
            watcher.cursor = 0;
        } else {
            watcher.max_file_bytes = 1;
        }
        let bytes = bad.to_bytes(restore_limits().max_bytes).unwrap();
        let decoded =
            ModelResourcesCheckpoint::from_bytes(&bytes, restore_limits().max_bytes).unwrap();
        let error = match decoded.prepare(&root, restore_limits()) {
            Err(error) => error,
            Ok(accepted) => {
                close_policy_candidate(accepted);
                panic!("accepted unwatchable model inputs");
            }
        };
        assert!(error.to_string().contains("restored watcher"));
        let error = match decoded.prepare(&absent_root, restore_limits()) {
            Err(error) => error,
            Ok(accepted) => {
                close_policy_candidate(accepted);
                panic!("accepted missing provider root");
            }
        };
        assert!(
            error.to_string().contains("restored watcher"),
            "policy validation ran after provider IO"
        );
    }
    assert!(app.catalog.snapshot(app.id.as_ref().unwrap()).is_some());
    app.stop_workers().unwrap();
}
#[test]
fn restore_preserves_exact_boundary_watcher_policy_and_allows_explicit_offline_owners() {
    let (mut app, mut saved, root) = watcher_policy_fixture();
    let id = app.id.as_ref().unwrap();
    let published = app.catalog.snapshot(id).unwrap();
    let size = published
        .inputs()
        .observations()
        .values()
        .map(|input| input.as_ref().unwrap().bytes.len())
        .max()
        .unwrap();
    saved.watcher.as_mut().unwrap().0.max_file_bytes = size;
    let mut restored = saved.prepare(&root, restore_limits()).unwrap();
    let watcher = restored.watcher.as_mut().unwrap();
    assert_eq!(watcher.source_byte_capacity(), size);
    watcher.request_checkpoint().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(policy) = watcher.try_checkpoint().unwrap() {
            assert_eq!(policy, saved.watcher.as_ref().unwrap().0);
            break;
        }
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    close_policy_candidate(restored);
    saved.watcher = None;
    saved.import_worker = false;
    let offline = saved
        .prepare(&root.join("nonexistent-provider"), restore_limits())
        .unwrap();
    assert!(offline.imports.is_none() && offline.watcher.is_none());
    assert!(offline.resources.catalog.snapshot(id).is_some());
    app.stop_workers().unwrap();
}
#[test]
fn empty_manifest_restore_reserves_watcher_slot_without_model_publications() {
    let temp = crate::project_prefab_tests::Temp::new();
    let path = project::create(&temp.0.join("empty"), "Empty restore").unwrap();
    let mut app = App::from_project(&path).unwrap();
    app.begin_model_checkpoint().unwrap();
    await_barrier(&mut app);
    let mut saved = app.capture_model_resources(limits()).unwrap();
    assert!(app.catalog.checkpoint(|_| None).unwrap().entries.is_empty());
    assert!(app.required_cpu_model_assets().is_empty());
    let watcher = &mut saved.watcher.as_mut().unwrap().0;
    watcher.sources.clear();
    watcher.cursor = 0;
    watcher.max_sources = 0;
    let error = match saved.prepare(path.parent().unwrap(), restore_limits()) {
        Err(error) => error,
        Ok(accepted) => {
            close_policy_candidate(accepted);
            panic!("empty manifest had no watcher capacity");
        }
    };
    assert!(error.to_string().contains("restored watcher source budget"));
    app.stop_workers().unwrap();
}

#[test]
fn preview_models_join_original_resource_graph_without_source_reimport() {
    let temp = crate::project_prefab_tests::Temp::new();
    let source = temp.0.join("source");
    std::fs::create_dir(&source).unwrap();
    let path = source.join("rig.glb");
    std::fs::write(
        &path,
        include_bytes!("../../../../voxy_render/examples/assets/animated-triangle.glb"),
    )
    .unwrap();
    let mut app = App::new(&path, false).unwrap();
    let id = app.id.clone().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while app.catalog.snapshot(&id).is_none() {
        app.tick().unwrap();
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    app.panel_action(panels::Action::Animation).unwrap();
    let phase = f64::from_bits(0.25_f64.to_bits() + 1);
    app.preview_animation_phase(phase, false).unwrap();
    let original = app
        .authoring
        .animation_preview
        .as_ref()
        .unwrap()
        .model_roots();
    assert_eq!(original.len(), 1);
    assert!(app.play.animations.checkpoint_model_roots().is_empty());
    app.begin_model_checkpoint().unwrap();
    await_barrier(&mut app);
    let saved = app.capture_model_resources(limits()).unwrap();
    assert!(
        serde_json::to_value(&saved).unwrap()["preview"].is_object(),
        "native resource archive omitted accepted preview and phase"
    );
    let original_preview = saved
        .preview
        .as_ref()
        .unwrap()
        .to_json(4 * 1024 * 1024)
        .unwrap();
    let bytes = saved.to_bytes(restore_limits().max_bytes).unwrap();
    let combined = app
        .capture_model_animation_checkpoint(
            |node| {
                app.instances
                    .iter()
                    .position(|candidate| *candidate == node)
                    .map(|i| app.object_ids[i].clone())
            },
            limits(),
            4 * 1024 * 1024,
        )
        .unwrap();
    let combined_bytes = combined.to_bytes(restore_limits().max_bytes).unwrap();
    app.stop_workers().unwrap();
    std::fs::remove_dir_all(source).unwrap();
    let target = temp.0.join("empty-target");
    std::fs::create_dir(&target).unwrap();
    let saved = ModelResourcesCheckpoint::from_bytes(&bytes, restore_limits().max_bytes).unwrap();
    let restored = saved.prepare(&target, restore_limits()).unwrap();
    assert_eq!(
        restored.resources.models.len(),
        1,
        "native resource graph omitted preview-only model root"
    );
    assert_eq!(restored.preview_models.len(), 1);
    let root = &restored.resources.models[0];
    assert!(Arc::ptr_eq(root, &restored.preview_models[0]));
    assert!(!Arc::ptr_eq(root, &original[0]));
    let publication = restored.resources.catalog.snapshot(&id).unwrap();
    assert!(
        Arc::ptr_eq(root, publication.value().animated.as_ref().unwrap()),
        "preview root lost original catalog/model alias"
    );
    close_policy_candidate(restored);
    for roles in [vec![1], vec![0, 0], vec![0, 1, 2]] {
        let mut invalid =
            ModelResourcesCheckpoint::from_bytes(&bytes, restore_limits().max_bytes).unwrap();
        invalid.preview_models = roles;
        assert!(invalid.prepare(&target, restore_limits()).is_err());
    }
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Default::default()).unwrap();
    let combined =
        ModelAnimationCheckpoint::from_bytes(&combined_bytes, restore_limits().max_bytes).unwrap();
    let prepared = combined
        .prepare(
            &scene,
            |_| Some(owner),
            &target,
            restore_limits(),
            4 * 1024 * 1024,
        )
        .unwrap();
    assert_eq!(prepared.resources.preview_models.len(), 1);
    assert!(prepared.animations.checkpoint_model_roots().is_empty());
    let preview = prepared.resources.preview.as_ref().unwrap();
    assert_eq!(preview.phase.to_bits(), phase.to_bits());
    assert_eq!(preview.owner, owner);
    let view =
        voxy_scene::SceneCheckpointView::new(&scene, [(app.object_ids[0].clone(), owner)]).unwrap();
    let encoded = preview
        .checkpoint(
            &view,
            |model| {
                prepared
                    .resources
                    .resources
                    .models
                    .iter()
                    .position(|root| Arc::ptr_eq(root, model))
                    .map(|index| format!("model:{index}"))
            },
            4 * 1024 * 1024,
        )
        .unwrap();
    assert_eq!(encoded.to_json(4 * 1024 * 1024).unwrap(), original_preview);
    close_policy_candidate(prepared.resources);
    let mut missing =
        ModelAnimationCheckpoint::from_bytes(&combined_bytes, restore_limits().max_bytes).unwrap();
    missing.resources.preview = None;
    assert!(
        missing
            .prepare(
                &scene,
                |_| Some(owner),
                &target,
                restore_limits(),
                4 * 1024 * 1024
            )
            .is_err()
    );
    assert!(
        combined
            .prepare(&scene, |_| None, &target, restore_limits(), 4 * 1024 * 1024)
            .is_err()
    );
    let mut wrong_shared =
        ModelAnimationCheckpoint::from_bytes(&combined_bytes, restore_limits().max_bytes).unwrap();
    wrong_shared.resources.preview_shared_gpu_owner =
        Some(voxy_scene::ObjectId("missing-gpu".into()));
    assert!(
        wrong_shared
            .prepare(
                &scene,
                |_| Some(owner),
                &target,
                restore_limits(),
                4 * 1024 * 1024
            )
            .is_err()
    );
    let mut invalid =
        ModelAnimationCheckpoint::from_bytes(&combined_bytes, restore_limits().max_bytes).unwrap();
    invalid.resources.preview_models.clear();
    assert!(
        invalid
            .prepare(
                &scene,
                |_| Some(owner),
                &target,
                restore_limits(),
                4 * 1024 * 1024
            )
            .is_err(),
        "preview-only root admitted without an owning role"
    );
}

#[test]
fn preview_runtime_aliases_restore_without_gpu_and_keep_equal_frames_independent() {
    for shared in [true, false] {
        let temp = crate::project_prefab_tests::Temp::new();
        let source = temp.0.join("source");
        std::fs::create_dir(&source).unwrap();
        let path = source.join("rig.glb");
        std::fs::write(
            &path,
            include_bytes!("../../../../voxy_render/examples/assets/animated-triangle.glb"),
        )
        .unwrap();
        let mut app = App::new(&path, false).unwrap();
        let id = app.id.clone().unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while app.catalog.snapshot(&id).is_none() {
            app.tick().unwrap();
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        let owner = app.instances[0];
        let model = app
            .catalog
            .snapshot(&id)
            .unwrap()
            .value()
            .animated
            .as_ref()
            .unwrap()
            .clone();
        app.scene
            .insert_component(
                owner,
                ModelAnimation {
                    clip: None,
                    ..Default::default()
                },
            )
            .unwrap();
        app.play.animations = crate::animation_runtime::AnimationRuntime::default()
            .prepare_wall(&app.scene, &[(id.clone(), model.clone())].into(), 1. / 60.)
            .unwrap();
        let phase = f64::from_bits(0.25_f64.to_bits() + 1);
        app.preview_animation_phase(phase, false).unwrap();
        let runtime_frame = app.play.animations.frame(owner, &model).unwrap();
        if shared {
            app.authoring
                .animation_preview
                .as_mut()
                .unwrap()
                .restore_shared_frame(runtime_frame.clone(), 4 * 1024 * 1024)
                .unwrap();
        }
        assert_eq!(
            Arc::ptr_eq(
                app.authoring
                    .animation_preview
                    .as_ref()
                    .unwrap()
                    .accepted_frame(),
                &runtime_frame
            ),
            shared
        );
        app.scene.set_active(owner, false).unwrap();
        let roles: std::collections::HashMap<_, _> = app
            .instances
            .iter()
            .copied()
            .zip(app.object_ids.iter().cloned())
            .collect();
        app.begin_model_checkpoint().unwrap();
        await_barrier(&mut app);
        let saved = app
            .capture_model_animation_checkpoint(
                |node| roles.get(&node).cloned(),
                limits(),
                4 * 1024 * 1024,
            )
            .unwrap();
        assert_eq!(
            serde_json::to_value(&saved).unwrap()["resources"]
                .get("preview_shared_runtime_owner")
                .is_some(),
            shared
        );
        let bytes = saved.to_bytes(restore_limits().max_bytes).unwrap();
        let document = app.authoring_document().unwrap();
        let serial = app.play.animations.serial();
        app.stop_workers().unwrap();
        std::fs::remove_dir_all(source).unwrap();
        let target = temp.0.join("target");
        std::fs::create_dir(&target).unwrap();
        let registry = editor_component_registry().unwrap();
        let scene = document.load(&registry, scene_limits::OBJECTS).unwrap();
        let saved =
            ModelAnimationCheckpoint::from_bytes(&bytes, restore_limits().max_bytes).unwrap();
        let prepared = saved
            .prepare(
                &scene.graph,
                |id| scene.resolve(id),
                &target,
                restore_limits(),
                4 * 1024 * 1024,
            )
            .unwrap();
        let preview = prepared.resources.preview.as_ref().unwrap();
        let restored_model = &prepared.resources.preview_models[0];
        let restored_frame = prepared
            .animations
            .frame(preview.owner, restored_model)
            .unwrap();
        assert_eq!(
            Arc::ptr_eq(preview.accepted_frame(), &restored_frame),
            shared,
            "preview/runtime frame identity differs after source-free restore"
        );
        assert_eq!(preview.phase.to_bits(), phase.to_bits());
        assert_eq!(prepared.animations.serial(), serial);
        assert_eq!(
            preview
                .accepted_frame()
                .checkpoint(&restored_model.skeleton, 4 * 1024 * 1024)
                .unwrap()
                .to_json(4 * 1024 * 1024)
                .unwrap(),
            runtime_frame
                .checkpoint(&model.skeleton, 4 * 1024 * 1024)
                .unwrap()
                .to_json(4 * 1024 * 1024)
                .unwrap()
        );
        close_policy_candidate(prepared.resources);
        let load = |saved: &ModelAnimationCheckpoint| {
            saved.prepare(
                &scene.graph,
                |id| scene.resolve(id),
                &target,
                restore_limits(),
                4 * 1024 * 1024,
            )
        };
        let mut missing =
            ModelAnimationCheckpoint::from_bytes(&bytes, restore_limits().max_bytes).unwrap();
        missing.resources.preview_shared_runtime_owner =
            Some(voxy_scene::ObjectId("missing-runtime".into()));
        assert!(load(&missing).is_err());
        if shared {
            let mut value = serde_json::to_value(&saved).unwrap();
            value["runtime"]["owners"] = serde_json::json!([]);
            let absent: ModelAnimationCheckpoint = serde_json::from_value(value).unwrap();
            assert!(
                load(&absent).is_err(),
                "preview role resolved to a scene node without runtime frame"
            );
            let mut value = serde_json::to_value(&saved).unwrap();
            value["runtime"]["owners"][0]["frame"]["transition_weight"] =
                serde_json::json!(0.5_f32.to_bits());
            let mismatch: ModelAnimationCheckpoint = serde_json::from_value(value).unwrap();
            let error = match load(&mismatch) {
                Err(error) => error,
                Ok(_) => panic!("different preview/runtime frame states joined"),
            };
            assert!(
                error
                    .to_string()
                    .contains("shared preview frame contents differ"),
                "{error}"
            );
        }
    }
}
