//! Exercises the production native publication path on a real offscreen device.
use super::*;
use voxy_gameplay::{UiElement, UiText};
use voxy_scene::{SceneGraph, Transform};

#[test]
#[ignore = "requires a real GPU and two local TrueType fonts"]
#[allow(clippy::too_many_lines)]
fn gpu_font_reload_retains_last_good_and_recovers_through_actual_worker_and_watcher() {
    let first = std::fs::read("/System/Library/Fonts/Supplemental/Arial.ttf").unwrap();
    let second = std::fs::read("/System/Library/Fonts/Supplemental/Courier New.ttf").unwrap();
    assert_ne!(first, second);
    let root = std::env::temp_dir().join(format!(
        "voxy-gpu-font-reload-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("font.ttf"), &first).unwrap();
    let project = AuthoringProject::new(&root, &crate::InputRecipe::Direct).unwrap();
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(
            owner,
            UiElement {
                origin: [0.0; 2],
                size: [1.0; 2],
                color: [0.1, 0.2, 0.3, 1.0],
                layer: 0,
                enabled: true,
                action: Some("menu".into()),
                text: Some(UiText {
                    font: "font.ttf".into(),
                    content: "Hello 123".into(),
                    size: 28.0,
                    color: [1.0; 4],
                }),
            },
        )
        .unwrap();
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    let info = adapter.get_info();
    assert_ne!(
        info.device_type,
        wgpu::DeviceType::Cpu,
        "hardware GPU required: {info:?}"
    );
    println!("UI CHECKPOINT GPU {info:?}");
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = voxy_render::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut ui = UiLive::default();
    let mut cache = crate::gpu_model::ResidencyCache::with_budget(32 * 1024 * 1024);
    let mut draws = Vec::<GpuDraw>::new();
    let mut presented = false;
    let mut failures = Vec::<String>::new();
    macro_rules! until {
        ($condition:expr) => {{
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            loop {
                cache.geometry_live = draws
                    .iter()
                    .map(|draw| draw.geometry.allocation_bytes())
                    .sum();
                if let Err(error) = ui.poll_gpu(
                    &scene,
                    &project,
                    [128.0; 2],
                    &renderer,
                    &device,
                    &queue,
                    &mut cache,
                    &mut draws,
                    &mut presented,
                ) {
                    failures.push(error.to_string());
                }
                if $condition {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "UI reload timed out: {failures:?}"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }};
    }
    until!(draws.len() == 2);
    assert!(failures.is_empty());
    ui.frame_presented();
    let visible = ui.presented().unwrap().clone();
    let original_text = Arc::clone(ui.presented_text.as_ref().unwrap());
    let original_run = original_text.value().runs[0].run.checkpoint();
    let original = Arc::clone(draws[1].texture.as_ref().unwrap());
    let archive_limits = crate::ui_text::checkpoint::UiTextArchiveLimits {
        imported: voxy_assets::ImportedArchiveLimits {
            max_inputs: 9,
            max_input_bytes: 16 * 1024 * 1024,
            max_archive_bytes: 128 * 1024 * 1024,
        },
        max_payload_bytes: 64 * 1024 * 1024,
        max_fonts: 8,
        max_runs: 128,
        max_pixels: 16 * 1024 * 1024,
        max_glyphs_per_run: 1024,
        max_total_glyphs: 8192,
        font: voxy_text::FontLimits {
            max_font_bytes: 4 * 1024 * 1024,
            max_glyph_pixels: 16384,
            max_size: 128.,
        },
    };
    let bytes = crate::ui_text::checkpoint::encode(
        &original_text,
        |node| (node == owner).then(|| voxy_scene::ObjectId("label".into())),
        archive_limits,
    )
    .unwrap();
    std::fs::remove_file(root.join("font.ttf")).unwrap();
    let restored = crate::ui_text::checkpoint::decode(
        &bytes,
        &scene,
        |id| (id.0 == "label").then_some(owner),
        archive_limits,
    )
    .unwrap();
    assert_eq!(restored.value().runs[0].run.checkpoint(), original_run);
    let restored_meshes = crate::ui_draw::build(&visible, &restored.value().runs).unwrap();
    let restored_draws =
        upload_on(&renderer, &device, &queue, &mut cache, &restored_meshes).unwrap();
    assert_eq!(restored_draws.len(), draws.len());
    assert!(Arc::ptr_eq(
        &original,
        restored_draws[1].texture.as_ref().unwrap()
    ));
    assert_eq!(
        restored_draws[1].geometry.index_count(),
        draws[1].geometry.index_count()
    );
    drop(restored_draws);
    let registry = voxy_scene::ComponentRegistry::default();
    let mut history = voxy_scene::SceneDocument {
        version: 1,
        objects: Vec::new(),
    }
    .load(&registry, 2)
    .unwrap();
    let retired = history.graph.spawn(None, Transform::default()).unwrap();
    history
        .identify(retired, voxy_scene::ObjectId("label".into()))
        .unwrap();
    history.graph.remove_subtree(retired).unwrap();
    let policy = voxy_scene::SimulationLimits {
        fixed_step: 0.01,
        max_steps: 4,
        max_behaviors: 4,
        max_commands: 4,
    };
    let simulation = voxy_scene::SceneSimulation::new(&history.graph, policy).unwrap();
    let record = simulation.checkpoint(&history, &registry, 100000).unwrap();
    let (history, _) = record
        .prepare(
            &registry,
            &voxy_scene::BehaviorCheckpointRegistry::default(),
            policy,
            2,
            100000,
        )
        .unwrap();
    let view = history.checkpoint_view();
    let historical_text =
        crate::ui_text::checkpoint::decode_historical(&bytes, &view, archive_limits).unwrap();
    let old_layout = visible
        .checkpoint(
            |node| (node == owner).then(|| voxy_scene::ObjectId("label".into())),
            100000,
        )
        .unwrap();
    let old_layout = old_layout.prepare_historical(&view, 128, 100000).unwrap();
    assert!(history.graph.local(old_layout.elements[0].owner).is_err());
    let history_meshes = crate::ui_draw::build(&old_layout, &historical_text.value().runs).unwrap();
    let history_draws = upload_on(&renderer, &device, &queue, &mut cache, &history_meshes).unwrap();
    assert!(Arc::ptr_eq(
        &original,
        history_draws[1].texture.as_ref().unwrap()
    ));
    assert_eq!(history.graph.len(), 0);
    drop(history_draws);
    println!("UI ARCHIVE GPU RESTORED without font source, including retired owner");
    std::fs::write(root.join("font.ttf"), b"broken font").unwrap();
    until!(!failures.is_empty());
    assert!(failures[0].contains("font"), "{failures:?}");
    assert!(Arc::ptr_eq(&original, draws[1].texture.as_ref().unwrap()));
    assert_eq!(ui.presented(), Some(&visible));
    assert!(Arc::ptr_eq(
        &original_text,
        ui.published_text.as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(
        &original_text,
        ui.presented_text.as_ref().unwrap()
    ));
    assert!(draws[1].geometry.index_count() > 0);
    std::fs::write(root.join("font.ttf"), &second).unwrap();
    until!(!Arc::ptr_eq(&original, draws[1].texture.as_ref().unwrap()));
    assert_eq!(ui.published.as_ref(), Some(&visible));
    let second_text = Arc::clone(ui.published_text.as_ref().unwrap());
    assert!(!Arc::ptr_eq(&original_text, &second_text));
    assert_ne!(original_run, second_text.value().runs[0].run.checkpoint());
    assert!(Arc::ptr_eq(
        &original_text,
        ui.presented_text.as_ref().unwrap()
    ));
    // Save the real two-generation owner before acknowledging the new frame.
    let bundle_limits = publications::UiPublicationLimits {
        max_bytes: 256 * 1024 * 1024,
        max_layout_bytes: 100000,
        max_elements: 128,
        max_input_bytes: 32 * 1024 * 1024,
        max_pixels: 32 * 1024 * 1024,
        max_glyphs: 16384,
        max_runs: 256,
        text: archive_limits,
    };
    let live_view = voxy_scene::SceneCheckpointView::new(
        &scene,
        [(voxy_scene::ObjectId("label".into()), owner)],
    )
    .unwrap();
    let bundle = ui
        .publication_checkpoint(&live_view, bundle_limits)
        .unwrap();
    let bundle_bytes = bundle.to_bytes(bundle_limits).unwrap();
    std::fs::write(root.join("ui.bundle"), &bundle_bytes).unwrap();
    std::fs::remove_file(root.join("font.ttf")).unwrap();
    let loaded_bundle = publications::UiPublicationsCheckpoint::from_bytes(
        &std::fs::read(root.join("ui.bundle")).unwrap(),
        bundle_limits,
    )
    .unwrap();
    let prepared_bundle = loaded_bundle.prepare(&live_view, bundle_limits).unwrap();
    assert_eq!(
        prepared_bundle
            .checkpoint(&live_view, bundle_limits)
            .unwrap()
            .to_bytes(bundle_limits)
            .unwrap(),
        bundle_bytes
    );
    let (new_meshes, old_meshes) = prepared_bundle.meshes().unwrap();
    let new_draws = upload_on(&renderer, &device, &queue, &mut cache, &new_meshes).unwrap();
    let retained_geometry = cache.geometry_live;
    cache.geometry_live += new_draws
        .iter()
        .map(|draw| draw.geometry.allocation_bytes())
        .sum::<u64>();
    let old_draws = upload_on(&renderer, &device, &queue, &mut cache, &old_meshes).unwrap();
    cache.geometry_live = retained_geometry;
    assert!(Arc::ptr_eq(
        new_draws[1].texture.as_ref().unwrap(),
        draws[1].texture.as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(
        old_draws[1].texture.as_ref().unwrap(),
        &original
    ));
    assert_eq!(
        new_draws[1].geometry.index_count(),
        draws[1].geometry.index_count()
    );
    assert!(Arc::ptr_eq(
        ui.published_text.as_ref().unwrap(),
        &second_text
    ));
    assert!(Arc::ptr_eq(
        ui.presented_text.as_ref().unwrap(),
        &original_text
    ));
    drop((new_draws, old_draws));
    println!("UI BUNDLE GPU RESTORED both font generations without source or presentation ACK");
    std::fs::write(root.join("font.ttf"), &second).unwrap();
    ui.frame_presented();
    assert!(Arc::ptr_eq(
        &second_text,
        ui.presented_text.as_ref().unwrap()
    ));
    std::fs::write(root.join("font.ttf"), &first).unwrap();
    until!(Arc::ptr_eq(&original, draws[1].texture.as_ref().unwrap()));
    assert_eq!(
        ui.published_text.as_ref().unwrap().value().runs[0]
            .run
            .checkpoint(),
        original_run
    );
    assert!(Arc::ptr_eq(
        &second_text,
        ui.presented_text.as_ref().unwrap()
    ));
    ui.frame_presented();
    assert!(Arc::ptr_eq(
        ui.published_text.as_ref().unwrap(),
        ui.presented_text.as_ref().unwrap()
    ));
    // A save starts with an actual outstanding one-shot UI import. Even if the
    // worker finishes quickly, ownership/completion must be drained normally.
    ui.attempted = None;
    ui.poll_gpu(
        &scene,
        &project,
        [128.; 2],
        &renderer,
        &device,
        &queue,
        &mut cache,
        &mut draws,
        &mut presented,
    )
    .unwrap();
    assert!(ui.worker.is_some());
    ui.begin_checkpoint().unwrap();
    assert!(ui.begin_checkpoint().is_err());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let saved_reload = loop {
        ui.poll_gpu(
            &scene,
            &project,
            [128.; 2],
            &renderer,
            &device,
            &queue,
            &mut cache,
            &mut draws,
            &mut presented,
        )
        .unwrap();
        if let Some(saved) = ui.advance_checkpoint().unwrap() {
            break saved;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "UI import save drain timed out"
        );
        std::thread::yield_now();
    };
    assert!(ui.worker.is_none() && ui.retired.is_empty() && !ui.scanning);
    let saved_reload_json = saved_reload.to_json(100000).unwrap();
    let frozen_text = ui.published_text.as_ref().unwrap().clone();
    let frozen_texture = draws[1].texture.as_ref().unwrap().clone();
    let live_limits = checkpoint::UiLiveCheckpointLimits {
        max_bytes: 300 * 1024 * 1024,
        max_reload_bytes: 1024 * 1024,
        publications: bundle_limits,
    };
    let live_bytes = ui
        .checkpoint(&live_view, live_limits)
        .unwrap()
        .to_bytes(live_limits)
        .unwrap();
    std::fs::write(root.join("ui.live"), &live_bytes).unwrap();
    std::fs::remove_file(root.join("font.ttf")).unwrap();
    let loaded_live = checkpoint::UiLiveCheckpoint::from_bytes(
        &std::fs::read(root.join("ui.live")).unwrap(),
        live_limits,
    )
    .unwrap();
    let prepared_live = loaded_live
        .prepare(&live_view, &project, cache.state(), live_limits)
        .unwrap();
    let (loaded_meshes, _) = prepared_live.publications.meshes().unwrap();
    let loaded_draws = upload_on(&renderer, &device, &queue, &mut cache, &loaded_meshes).unwrap();
    assert!(Arc::ptr_eq(
        loaded_draws[1].texture.as_ref().unwrap(),
        &frozen_texture
    ));
    assert!(prepared_live.reload.watcher.is_some());
    prepared_live.close().unwrap().join().unwrap();
    drop(loaded_draws);
    for _ in 0..10 {
        // A different viewport does not start a new import while the save fence
        // is held; file changes remain observable against the saved baseline.
        ui.poll_gpu(
            &scene,
            &project,
            [256.; 2],
            &renderer,
            &device,
            &queue,
            &mut cache,
            &mut draws,
            &mut presented,
        )
        .unwrap();
        assert_eq!(
            ui.advance_checkpoint()
                .unwrap()
                .unwrap()
                .to_json(100000)
                .unwrap(),
            saved_reload_json
        );
        assert!(ui.worker.is_none() && !ui.scanning);
    }
    assert!(Arc::ptr_eq(
        ui.published_text.as_ref().unwrap(),
        &frozen_text
    ));
    assert!(Arc::ptr_eq(
        draws[1].texture.as_ref().unwrap(),
        &frozen_texture
    ));
    assert_eq!(
        ui.checkpoint(&live_view, live_limits)
            .unwrap()
            .to_bytes(live_limits)
            .unwrap(),
        live_bytes
    );
    assert!(ui.resume_after_checkpoint().unwrap());
    let previous_failures = failures.len();
    until!(failures.len() > previous_failures && ui.worker.is_none());
    assert!(Arc::ptr_eq(
        ui.published_text.as_ref().unwrap(),
        &frozen_text
    ));
    assert!(Arc::ptr_eq(
        draws[1].texture.as_ref().unwrap(),
        &frozen_texture
    ));
    // A failed drained import is also a saveable last-good state.
    ui.begin_checkpoint().unwrap();
    let failed_reload = loop {
        ui.poll_gpu(
            &scene,
            &project,
            [128.; 2],
            &renderer,
            &device,
            &queue,
            &mut cache,
            &mut draws,
            &mut presented,
        )
        .unwrap();
        if let Some(saved) = ui.advance_checkpoint().unwrap() {
            break saved;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "failed UI save drain timed out"
        );
        std::thread::yield_now();
    };
    let prepared_reload = failed_reload.prepare(&project, cache.state()).unwrap();
    assert!(prepared_reload.deferred.is_some());
    prepared_reload.close().unwrap().join().unwrap();
    assert!(ui.resume_after_checkpoint().unwrap());
    std::fs::write(root.join("font.ttf"), &first).unwrap();
    until!(ui.deferred.is_none() && ui.ready());
    println!(
        "UI SAVE BARRIER GPU drained import, froze dispatch, retained failed last-good and recovered"
    );
    for handle in ui.close() {
        handle.join().unwrap();
    }
    assert!(pollster::block_on(scope.pop()).is_none());
    std::fs::remove_dir_all(root).unwrap();
}
