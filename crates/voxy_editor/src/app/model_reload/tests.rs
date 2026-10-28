use super::*;
fn candidate() -> ImportedAsset<import::EditorAsset> {
    let vertices = [[-0.5, -0.5, 0.], [0.5, -0.5, 0.], [0., 0.5, 0.]].map(|position| {
        voxy_render::SceneVertex {
            position,
            uv: [0.; 2],
            color: [1.; 4],
        }
    });
    let mesh = voxy_render::SceneMesh::new(vertices.to_vec(), vec![0, 1, 2]).unwrap();
    let asset = import::EditorAsset {
        mesh,
        lod: None,
        animated: None,
        skinned_lod: None,
        nodes: vec![],
        images: vec![],
    };
    voxy_assets::ImportInputs::new(0, 0)
        .finish(asset, |_, _| unreachable!())
        .unwrap()
}
#[test]
#[ignore = "requires real GPU; native reload transaction without a window"]
fn native_transaction_switches_cpu_gpu_together_and_rejects_stale_or_failed_work() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("NATIVE MODEL RELOAD GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut app = App::new(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj"),
        false,
    )
    .unwrap();
    app.tick().unwrap();
    app.begin_model_checkpoint().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while !app.poll_model_checkpoint().unwrap() {
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let id = app.id.clone().unwrap();
    let old = app.catalog.snapshot(&id).unwrap();
    let mut cache = gpu_model::ResidencyCache::default();
    let old_model =
        ModelGraphics::upload_on(&renderer, &device, &queue, old.value(), &mut cache).unwrap();
    let old_bytes = old_model.geometry_bytes();
    cache.geometry_live = old_bytes;
    let mut models = BTreeMap::from([(id.clone(), old_model)]);
    let mut errors = BTreeMap::new();
    macro_rules! target {
        () => {
            UploadTarget {
                renderer: &renderer,
                device: &device,
                queue: &queue,
                cache: &cache,
                live_geometry: old_bytes,
                replaced_geometry: old_bytes,
            }
        };
    }
    macro_rules! destination {
        () => {
            Some(GpuDestination {
                models: &mut models,
                cache: &mut cache,
                errors: &mut errors,
            })
        };
    }
    let ticket = app.catalog.request(id.clone()).unwrap();
    let asset = candidate();
    let new_bytes = ModelGraphics::geometry_estimate(asset.value());
    assert_ne!(new_bytes, old_bytes);
    let pending = target!()
        .stage(BTreeMap::from([(id.clone(), Arc::new(asset))]))
        .unwrap_or_else(|e| panic!("{}", e.error));
    assert!(Arc::ptr_eq(&old, &app.catalog.snapshot(&id).unwrap()));
    assert_eq!(models[&id].geometry_bytes(), old_bytes);
    let mut ready =
        pollster::block_on(pending.validate()).unwrap_or_else(|e| panic!("{}", e.error));
    let source = take_source(std::mem::take(&mut ready.assets), &id).unwrap();
    assert!(
        complete_transaction(
            &mut app.catalog,
            &mut app.sources,
            &ticket,
            Ok(source),
            Some(ready),
            destination!(),
            None
        )
        .unwrap()
    );
    assert!(!Arc::ptr_eq(&old, &app.catalog.snapshot(&id).unwrap()));
    assert_eq!(models[&id].geometry_bytes(), new_bytes);
    assert!(Arc::ptr_eq(
        models[&id].publication.as_ref().unwrap(),
        &app.catalog.snapshot(&id).unwrap()
    ));
    assert_eq!(cache.geometry_live, new_bytes);
    let accepted = app.catalog.snapshot(&id).unwrap();
    let cache_before = cache.state();
    let stale = app.catalog.request(id.clone()).unwrap();
    let pending = UploadTarget {
        live_geometry: new_bytes,
        replaced_geometry: new_bytes,
        ..target!()
    }
    .stage(BTreeMap::from([(id.clone(), Arc::new(candidate()))]))
    .unwrap_or_else(|e| panic!("{}", e.error));
    let mut ready =
        pollster::block_on(pending.validate()).unwrap_or_else(|e| panic!("{}", e.error));
    let current = app.catalog.request(id.clone()).unwrap();
    let source = take_source(std::mem::take(&mut ready.assets), &id).unwrap();
    assert_eq!(
        complete_transaction(
            &mut app.catalog,
            &mut app.sources,
            &stale,
            Ok(source),
            Some(ready),
            destination!(),
            None
        ),
        Err(PublicationError::Asset(AssetError::StaleTicket))
    );
    assert_eq!(cache.state(), cache_before);
    assert!(Arc::ptr_eq(&accepted, &app.catalog.snapshot(&id).unwrap()));
    assert!(
        !complete_transaction(
            &mut app.catalog,
            &mut app.sources,
            &current,
            Err(candidate().into_failed("GPU validation failed".into())),
            None,
            destination!(),
            None
        )
        .unwrap()
    );
    assert_eq!(cache.state(), cache_before);
    assert_eq!(models[&id].geometry_bytes(), new_bytes);
    assert!(Arc::ptr_eq(&accepted, &app.catalog.snapshot(&id).unwrap()));
    for (edges, policy) in [
        (0, None),
        (
            512,
            Some(SourceWatchAdmission {
                max_sources: 257,
                max_file_bytes: 0,
                required_source: None,
            }),
        ),
        (
            512,
            Some(SourceWatchAdmission {
                max_sources: 1,
                max_file_bytes: import::SOURCE_BYTES,
                required_source: None,
            }),
        ),
    ] {
        app.sources = SourceDependencies::new(128, edges);
        app.sources
            .record(
                id.clone(),
                accepted.inputs(),
                voxy_assets::ImportOutcome::Published,
            )
            .unwrap();
        let source_token = app.sources.change_token();
        let cache_identity = cache.identity();
        let ticket = app.catalog.request(id.clone()).unwrap();
        let pending = UploadTarget {
            live_geometry: new_bytes,
            replaced_geometry: new_bytes,
            ..target!()
        }
        .stage(BTreeMap::from([(
            id.clone(),
            Arc::new(candidate_with_two_sources(&id)),
        )]))
        .unwrap_or_else(|e| panic!("{}", e.error));
        let mut ready =
            pollster::block_on(pending.validate()).unwrap_or_else(|e| panic!("{}", e.error));
        let source = take_source(std::mem::take(&mut ready.assets), &id).unwrap();
        assert!(
            !complete_transaction(
                &mut app.catalog,
                &mut app.sources,
                &ticket,
                Ok(source),
                Some(ready),
                destination!(),
                policy
            )
            .unwrap()
        );
        assert_eq!(app.catalog.pending(), 0);
        assert!(
            matches!(app.catalog.status(&id), Some(voxy_assets::AssetStatus::Failed(error)) if error.contains("dependency budget"))
        );
        assert!(Arc::ptr_eq(&accepted, &app.catalog.snapshot(&id).unwrap()));
        assert!(Arc::ptr_eq(
            models[&id].publication.as_ref().unwrap(),
            &accepted
        ));
        assert_eq!(models[&id].geometry_bytes(), new_bytes);
        assert_eq!(cache.state(), cache_before);
        assert!(Arc::ptr_eq(&cache.identity(), &cache_identity));
        assert_eq!(app.sources.change_token(), source_token);
    }
    let ticket = app.catalog.request(id.clone()).unwrap();
    let source = Arc::new(candidate());
    let assets = BTreeMap::from([(id.clone(), source.clone())]);
    app.pending_model = Some(Pending {
        ticket,
        source,
        device: device.clone(),
        cache: cache.state(),
        cache_identity: cache.identity(),
        geometry: new_bytes,
        checks: Box::pin(async move {
            Err(RejectedModelBatch {
                assets,
                error: "injected deferred GPU failure".into(),
            })
        }),
    });
    app.poll_pending_model().unwrap();
    assert!(app.pending_model.is_none());
    assert_eq!(app.catalog.pending(), 0);
    assert!(Arc::ptr_eq(&accepted, &app.catalog.snapshot(&id).unwrap()));
    let ticket = app.catalog.request(id.clone()).unwrap();
    let source = Arc::new(candidate());
    let retained = source.clone();
    app.resume_model_checkpoint().unwrap();
    app.begin_model_checkpoint().unwrap();
    app.pending_model = Some(Pending {
        ticket,
        source,
        device: device.clone(),
        cache: cache.state(),
        cache_identity: cache.identity(),
        geometry: new_bytes,
        checks: Box::pin(async move {
            let _retained = retained;
            std::future::pending::<Result<ReadyModelBatch, RejectedModelBatch>>().await
        }),
    });
    assert!(!app.poll_model_checkpoint().unwrap());
    app.stop_workers().unwrap();
    assert_eq!(app.catalog.pending(), 0);
    assert!(Arc::ptr_eq(&accepted, &app.catalog.snapshot(&id).unwrap()));
}

#[test]
fn model_completions_cannot_override_watcher_owned_manifest_inventory() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&fixture, false).unwrap();
    let id = app.id.clone().unwrap();
    let original_inventory = app.available.clone();
    app.import_recipe = InputRecipe::Manifest(AssetId("assets.json".into()));
    let manifest = br#"{"version":1,"assets":[{"asset":"new-model","source":"new.obj"}]}"#;
    let observed = || {
        let mut inputs = voxy_assets::ImportInputs::new(1, 65536);
        inputs
            .read(AssetId("assets.json".into()), |_, _| Ok(manifest.to_vec()))
            .unwrap();
        inputs
            .finish(candidate().value().clone(), |_, _| Ok(manifest.to_vec()))
            .unwrap()
    };
    let stale = app.catalog.request(id.clone()).unwrap();
    let current = app.catalog.request(id.clone()).unwrap();
    app.complete_model_reload(&stale, Ok(observed()), None)
        .unwrap();
    assert_eq!(app.available, original_inventory);
    assert!(app.catalog.snapshot(&id).is_none());
    app.complete_model_reload(&current, Ok(observed()), None)
        .unwrap();
    assert_eq!(app.available, original_inventory);
    assert!(app.catalog.snapshot(&id).is_some());
    app.stop_workers().unwrap();
}

fn candidate_with_two_sources(root: &AssetId) -> ImportedAsset<import::EditorAsset> {
    let mut inputs = voxy_assets::ImportInputs::new(2, 2);
    for id in [root.clone(), AssetId("new-dependency.bin".into())] {
        inputs.read(id, |_, _| Ok(vec![1])).unwrap();
    }
    inputs
        .finish(candidate().value().clone(), |_, _| Ok(vec![1]))
        .unwrap()
}
#[test]
fn source_budget_failure_settles_ticket_retains_model_and_does_not_block_checkpoint() {
    exercise_source_budget_failure(false);
}
#[test]
fn watcher_capacity_rejects_unwatchable_model_and_settles_ticket_without_graph_changes() {
    exercise_source_budget_failure(true);
}
fn exercise_source_budget_failure(watcher_limited: bool) {
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
    let old = app.catalog.snapshot(&id).unwrap();
    if watcher_limited {
        app.begin_model_checkpoint().unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while !app.poll_model_checkpoint().unwrap() {
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        app.watcher.take().unwrap().close().join().unwrap();
        app.watcher = Some(
            SourcePollWorker::new(
                FileInputs::new(fixture.parent().unwrap()).unwrap(),
                1,
                import::SOURCE_BYTES,
            )
            .unwrap(),
        );
        assert_eq!(app.watcher.as_ref().unwrap().source_capacity(), 1);
        app.resume_model_checkpoint().unwrap();
    }
    let mut sources = SourceDependencies::new(1, if watcher_limited { 512 } else { 1 });
    sources
        .record(
            id.clone(),
            old.inputs(),
            voxy_assets::ImportOutcome::Published,
        )
        .unwrap();
    app.sources = sources;
    let before = app.sources.source_ids();
    let source_token = app.sources.change_token();
    let stale = app.catalog.request(id.clone()).unwrap();
    let ticket = app.catalog.request(id.clone()).unwrap();
    app.complete_model_reload(&stale, Ok(candidate_with_two_sources(&id)), None)
        .unwrap();
    assert_eq!(app.catalog.pending(), 1);
    assert!(matches!(
        app.catalog.status(&id),
        Some(voxy_assets::AssetStatus::Loading)
    ));
    assert!(Arc::ptr_eq(&old, &app.catalog.snapshot(&id).unwrap()));
    assert_eq!(app.sources.change_token(), source_token);
    assert!(app.failed_at.is_none());
    app.complete_model_reload(&ticket, Ok(candidate_with_two_sources(&id)), None)
        .unwrap();
    assert_eq!(app.catalog.pending(), 0);
    assert!(
        matches!(app.catalog.status(&id), Some(voxy_assets::AssetStatus::Failed(error)) if error.contains("dependency budget"))
    );
    assert!(Arc::ptr_eq(&old, &app.catalog.snapshot(&id).unwrap()));
    assert_eq!(app.sources.source_ids(), before);
    assert_eq!(app.sources.change_token(), source_token);
    assert!(app.failed_at.is_some());
    app.begin_model_checkpoint().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while !app.poll_model_checkpoint().unwrap() {
        assert!(
            Instant::now() < until,
            "budget failure left checkpoint waiting on lost import"
        );
        std::thread::yield_now();
    }
    app.stop_workers().unwrap();
}

#[test]
fn watcher_byte_budget_refuses_grown_source_and_recovers_when_valid_source_fits_again() {
    let root = std::env::temp_dir().join(format!(
        "voxy-watch-byte-budget-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let source = root.join("mesh.obj");
    let model = |x| format!("v {x} 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n");
    let original = model(0);
    std::fs::write(&source, &original).unwrap();
    let mut app = App::new(&source, false).unwrap();
    let id = app.id.clone().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while app.catalog.snapshot(&id).is_none() {
        app.tick().unwrap();
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    app.begin_model_checkpoint().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while !app.poll_model_checkpoint().unwrap() {
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let old = app.catalog.snapshot(&id).unwrap();
    let document = app.authoring_document().unwrap();
    app.watcher.take().unwrap().close().join().unwrap();
    app.watcher = Some(
        SourcePollWorker::new_with_observations(
            FileInputs::new(&root).unwrap(),
            1,
            original.len(),
            old.inputs().observations(),
        )
        .unwrap(),
    );
    app.resume_model_checkpoint().unwrap();
    std::fs::write(&source, format!("{}# extra valid source bytes\n", model(2))).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while !matches!(
        app.catalog.status(&id),
        Some(voxy_assets::AssetStatus::Failed(_))
    ) {
        app.tick().unwrap();
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        matches!(app.catalog.status(&id), Some(voxy_assets::AssetStatus::Failed(error)) if error.contains("source-byte limit"))
    );
    assert!(Arc::ptr_eq(&old, &app.catalog.snapshot(&id).unwrap()));
    assert_eq!(app.catalog.pending(), 0);
    assert_eq!(app.authoring_document().unwrap(), document);
    std::fs::write(&source, model(3)).unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while Arc::ptr_eq(&old, &app.catalog.snapshot(&id).unwrap()) {
        app.tick().unwrap();
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        app.catalog.snapshot(&id).unwrap().value().mesh.vertices()[0].position[0],
        3.
    );
    assert_eq!(app.authoring_document().unwrap(), document);
    app.stop_workers().unwrap();
}

#[test]
fn explicit_panel_retry_recovers_unchanged_source_and_respects_checkpoint_fence() {
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../voxy_render/examples/assets/quad.obj");
    let mut app = App::new(&source, false).unwrap();
    let id = app.id.clone().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while app.catalog.snapshot(&id).is_none() {
        app.tick().unwrap();
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let old = app.catalog.snapshot(&id).unwrap();
    let document = app.authoring_document().unwrap();
    assert!(!app.can_retry_selected_model());
    let ticket = app.catalog.request(id.clone()).unwrap();
    app.catalog
        .complete(&ticket, Err("transient read failure".into()))
        .unwrap();
    assert!(app.can_retry_selected_model());
    assert!(app.selected_model_notice().unwrap().retry);
    app.begin_model_checkpoint().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while !app.poll_model_checkpoint().unwrap() {
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    assert!(!app.can_retry_selected_model());
    assert!(!app.selected_model_notice().unwrap().retry);
    assert!(app.panel_action(panels::Action::RetryModel).is_err());
    assert!(matches!(
        app.catalog.status(&id),
        Some(voxy_assets::AssetStatus::Failed(_))
    ));
    assert!(Arc::ptr_eq(&old, &app.catalog.snapshot(&id).unwrap()));
    app.resume_model_checkpoint().unwrap();
    app.panel_action(panels::Action::RetryModel).unwrap();
    assert!(matches!(
        app.catalog.status(&id),
        Some(voxy_assets::AssetStatus::Dirty)
    ));
    assert_eq!(app.reload.iter().filter(|asset| *asset == &id).count(), 1);
    assert!(app.panel_action(panels::Action::RetryModel).is_err());
    assert!(Arc::ptr_eq(&old, &app.catalog.snapshot(&id).unwrap()));
    let until = Instant::now() + Duration::from_secs(10);
    while !matches!(
        app.catalog.status(&id),
        Some(voxy_assets::AssetStatus::Ready)
    ) {
        app.tick().unwrap();
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let new = app.catalog.snapshot(&id).unwrap();
    assert!(!Arc::ptr_eq(&old, &new));
    assert_eq!(old.inputs().observations(), new.inputs().observations());
    assert_eq!(app.authoring_document().unwrap(), document);
    assert!(!app.can_retry_selected_model());
    assert_eq!(app.catalog.pending(), 0);
    app.stop_workers().unwrap();
}
