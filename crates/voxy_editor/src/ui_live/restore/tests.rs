use super::*;
use crate::ui_live::checkpoint::{UiLiveCheckpoint, UiLiveCheckpointLimits};
use voxy_gameplay::{UiElement, UiText};
use voxy_scene::{ObjectId, SceneCheckpointView, Transform};

#[test]
fn restored_history_is_not_native_input_or_focus_until_frame_ack() {
    let snapshot = SceneUiSnapshot {
        viewport: [128.; 2],
        elements: Vec::new(),
    };
    let mut ui = UiLive {
        attempted: Some(snapshot.clone()),
        published: Some(snapshot.clone()),
        presented: Some(snapshot.clone()),
        awaiting_first_presentation: true,
        ..UiLive::default()
    };
    assert!(ui.ready());
    assert!(ui.presented().is_none());
    assert!(ui.focus_snapshot().is_none());
    assert_eq!(ui.presented.as_ref(), Some(&snapshot));
    ui.frame_presented();
    assert_eq!(ui.presented(), Some(&snapshot));
    assert_eq!(ui.focus_snapshot(), Some(&snapshot));
    assert!(ui.close().is_empty());
    assert!(!ui.awaiting_first_presentation);
}
#[test]
fn upload_index_fork_does_not_change_live_policies_or_accounting() {
    let mut cache = ResidencyCache::with_budget(1000);
    cache.geometry_live = 25;
    let before = cache.state();
    let mut candidate = cache.fork_for_upload();
    assert_eq!(candidate.state(), before);
    candidate.budget = 50;
    candidate.geometry_live = 200;
    candidate.geometry_budget = 400;
    assert_eq!(cache.state(), before);
}

pub(crate) fn pixels(
    renderer: &voxy_render::SceneRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    draws: &[GpuDraw],
) -> Vec<u8> {
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("UI restore pixels"),
        size: wgpu::Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("UI restore depth"),
        size: color.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let white = renderer
        .upload_texture(device, queue, 1, 1, &[255; 4])
        .unwrap();
    let transform = renderer
        .create_transform(device, glam::Mat4::IDENTITY)
        .unwrap();
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("UI restore readback"),
        size: 128 * 128 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    let calls: Vec<_> = draws
        .iter()
        .map(|draw| voxy_render::SceneDraw {
            geometry: &draw.geometry,
            texture: draw.texture.as_deref().unwrap_or(&white),
            transform: &transform,
            overlay: true,
        })
        .collect();
    renderer.encode(
        &mut encoder,
        &color.create_view(&Default::default()),
        &depth.create_view(&Default::default()),
        wgpu::Color::BLACK,
        &calls,
    );
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &color,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(512),
                rows_per_image: Some(128),
            },
        },
        color.size(),
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let result = readback.slice(..).get_mapped_range().unwrap().to_vec();
    readback.unmap();
    result
}
fn limits() -> UiLiveCheckpointLimits {
    UiLiveCheckpointLimits {
        max_bytes: 300 * 1024 * 1024,
        max_reload_bytes: 1024 * 1024,
        publications: publications::UiPublicationLimits {
            max_bytes: 256 * 1024 * 1024,
            max_layout_bytes: 100000,
            max_elements: 128,
            max_input_bytes: 32 * 1024 * 1024,
            max_pixels: 32 * 1024 * 1024,
            max_glyphs: 16384,
            max_runs: 256,
            text: ui_text::checkpoint::UiTextArchiveLimits {
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
            },
        },
    }
}

#[test]
#[ignore = "requires real GPU and local Unicode TrueType font; offscreen readback"]
fn gpu_restore_rolls_back_late_failures_and_publishes_exact_pixels_only_after_validation() {
    exercise_gpu_restore(false, false, false);
}
#[test]
#[ignore = "requires real GPU and local Unicode TrueType font; coordinated frozen UI readback"]
fn gpu_restore_at_completed_barrier_preserves_fence_on_fault_and_discard() {
    exercise_gpu_restore(true, false, false);
}
#[test]
#[ignore = "requires real GPU and local font; combined model/UI cache and peak admission"]
fn gpu_model_and_ui_restore_share_cache_and_admit_combined_peak() {
    exercise_gpu_restore(true, true, false);
}
#[test]
#[ignore = "requires real GPU and font; coordinated file with nonempty UI and audio"]
fn coordinated_file_restores_real_text_and_audio_without_source_inputs() {
    exercise_gpu_restore(true, true, true);
}
fn exercise_gpu_restore(held_barrier: bool, with_models: bool, with_file: bool) {
    let root = std::env::temp_dir().join(format!(
        "voxy-ui-restore-{}-{}",
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
    let font = [
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "C:/Windows/Fonts/arial.ttf",
    ]
    .iter()
    .find_map(|p| std::fs::read(p).ok())
    .expect("Unicode TrueType font fixture");
    std::fs::write(root.join("font.ttf"), font).unwrap();
    let project = AuthoringProject::new(&root, &crate::InputRecipe::Direct).unwrap();
    let mut scene = SceneGraph::new(4);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    let mut element = UiElement {
        origin: [0.1; 2],
        size: [0.8; 2],
        color: [0.2, 0.3, 0.4, 1.],
        layer: 0,
        enabled: true,
        action: Some("menu".into()),
        text: Some(UiText {
            font: "font.ttf".into(),
            content: "Old office".into(),
            size: 18.,
            color: [1.; 4],
        }),
    };
    scene.insert_component(owner, element.clone()).unwrap();
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let info = adapter.get_info();
    assert_ne!(info.device_type, wgpu::DeviceType::Cpu);
    println!("UI ATOMIC RESTORE GPU {info:?}");
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = voxy_render::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut ui = UiLive::default();
    let mut cache = ResidencyCache::with_budget(32 * 1024 * 1024);
    let mut draws = Vec::new();
    let mut displayed = false;
    macro_rules! until {
        ($condition:expr) => {{
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            loop {
                cache.geometry_live = draws
                    .iter()
                    .map(|draw: &GpuDraw| draw.geometry.allocation_bytes())
                    .sum();
                ui.poll_gpu(
                    &scene,
                    &project,
                    [128.; 2],
                    &renderer,
                    &device,
                    &queue,
                    &mut cache,
                    &mut draws,
                    &mut displayed,
                )
                .unwrap();
                if $condition {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "UI publication timed out"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }};
    }
    until!(ui.ready());
    let old_pixels = pixels(&renderer, &device, &queue, &draws);
    ui.frame_presented();
    element.text.as_mut().unwrap().content = "New WORLD".into();
    scene.insert_component(owner, element.clone()).unwrap();
    until!(ui.published != ui.presented && ui.ready());
    let saved_pixels = pixels(&renderer, &device, &queue, &draws);
    assert_ne!(saved_pixels, old_pixels);
    ui.begin_checkpoint().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        ui.poll_gpu(
            &scene,
            &project,
            [128.; 2],
            &renderer,
            &device,
            &queue,
            &mut cache,
            &mut draws,
            &mut displayed,
        )
        .unwrap();
        if ui.advance_checkpoint().unwrap().is_some() {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
    let view = SceneCheckpointView::new(&scene, [(ObjectId("label".into()), owner)]).unwrap();
    let bytes = ui
        .checkpoint(&view, limits())
        .unwrap()
        .to_bytes(limits())
        .unwrap();
    std::fs::write(root.join("ui.live"), &bytes).unwrap();
    assert!(ui.resume_after_checkpoint().unwrap());
    let coordinated_limits = if with_file {
        use crate::app::restore_barrier::file::CoordinatedFileLimits;
        std::fs::write(
            root.join("quad.obj"),
            include_bytes!("../../../../voxy_render/examples/assets/quad.obj"),
        )
        .unwrap();
        std::fs::write(
            root.join("tone.wav"),
            crate::audio_session::checkpoint::tests::wav(),
        )
        .unwrap();
        let mut app = crate::App::new(&root.join("quad.obj"), false).unwrap();
        app.tick().unwrap();
        app.begin_model_checkpoint().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while !app.poll_model_checkpoint().unwrap() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        app.resume_model_checkpoint().unwrap();
        app.scene = std::mem::replace(&mut scene, SceneGraph::new(4));
        app.object_ids = vec![ObjectId("label".into())];
        app.instances = vec![owner];
        app.scene
            .insert_component(
                owner,
                voxy_gameplay::AudioSource {
                    asset: "tone.wav".into(),
                    import_settings: None,
                    bus: 0,
                    gain: 0.8,
                    looping: true,
                    spatial: false,
                    near: 1.,
                    far: 5.,
                },
            )
            .unwrap();
        let play_limits = crate::play_session::checkpoint::tests::limits();
        let registry = app.authoring.authoring_project.registry.clone();
        let view =
            SceneCheckpointView::new(&app.scene, [(ObjectId("label".into()), owner)]).unwrap();
        app.play.playing = Some(view.capture(&registry).unwrap());
        app.play.simulation = Some(
            voxy_scene::SceneSimulation::new(&app.scene, play_limits.core.simulation).unwrap(),
        );
        app.audio.set_play(
            crate::audio_play::AudioPlay::prepare(
                &app.scene,
                &app.authoring.authoring_project,
                None,
            )
            .unwrap(),
        );
        app.audio
            .play_mut()
            .unwrap()
            .advance(&app.scene, 3)
            .unwrap();
        app.ui_live = std::mem::take(&mut ui);
        app.begin_restore_barrier(8 * 1024 * 1024).unwrap();
        while !app
            .poll_restore_barrier(|node| (node == owner).then(|| ObjectId("label".into())))
            .unwrap()
        {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        let view =
            SceneCheckpointView::new(&app.scene, [(ObjectId("label".into()), owner)]).unwrap();
        let resources = crate::play_session::checkpoint::tests::resource_restore_limits();
        let checkpoint = app
            .coordinated_play_checkpoint(
                &view,
                &registry,
                "ui-file-v1",
                resources,
                play_limits,
                limits(),
            )
            .unwrap();
        let policy = CoordinatedFileLimits {
            max_bytes: 512 * 1024 * 1024,
            max_game_bytes: resources.max_bytes,
            ui: limits(),
            max_audio_bytes: 8 * 1024 * 1024,
            max_input_bytes: 100000,
        };
        checkpoint
            .save_file(&root.join("game.voxycgr"), policy)
            .unwrap();
        if let Some(directory) = std::env::var_os("VOXY_UI_RESTORE_EVIDENCE_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::copy(root.join("game.voxycgr"), directory.join("game.voxycgr")).unwrap();
        }
        while !app.resume_restore_barrier().unwrap() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        ui = std::mem::take(&mut app.ui_live);
        app.play.stop_simulation(&mut app.scene).unwrap();
        scene = std::mem::replace(&mut app.scene, SceneGraph::new(4));
        app.stop_workers().unwrap();
        std::fs::remove_file(root.join("quad.obj")).unwrap();
        std::fs::remove_file(root.join("tone.wav")).unwrap();
        Some(policy)
    } else {
        None
    };
    let saved_element = element.clone();
    element.text.as_mut().unwrap().content = "Other live UI".into();
    scene.insert_component(owner, element).unwrap();
    until!(
        ui.published.as_ref().unwrap().elements[0]
            .descriptor
            .text
            .as_ref()
            .unwrap()
            .content
            == "Other live UI"
            && ui.ready()
    );
    ui.frame_presented();
    displayed = true;
    let target_pixels = pixels(&renderer, &device, &queue, &draws);
    assert_ne!(target_pixels, saved_pixels);
    if held_barrier {
        // Drain while the source scene and its font still exist. A watcher
        // request already issued by poll_gpu needs normal owner polling.
        ui.begin_checkpoint().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            ui.poll_gpu(
                &scene,
                &project,
                [128.; 2],
                &renderer,
                &device,
                &queue,
                &mut cache,
                &mut draws,
                &mut displayed,
            )
            .unwrap();
            if ui.advance_checkpoint().unwrap().is_some() {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    // The surrounding scene participant has restored its descriptor. The live
    // UI still displays its previous publication until the atomic UI commit.
    scene.insert_component(owner, saved_element).unwrap();
    let view = SceneCheckpointView::new(&scene, [(ObjectId("label".into()), owner)]).unwrap();
    std::fs::remove_file(root.join("font.ttf")).unwrap();
    let saved = if let Some(policy) = coordinated_limits {
        let checkpoint = crate::app::restore_barrier::CoordinatedPlayCheckpoint::read_file(
            &root.join("game.voxycgr"),
            policy,
        )
        .unwrap();
        assert!(
            serde_json::from_str::<serde_json::Value>(
                &checkpoint.audio.to_json(policy.max_audio_bytes).unwrap()
            )
            .unwrap()["play"]
                .is_object()
        );
        let target = root.join("restored-project");
        std::fs::create_dir(&target).unwrap();
        let behaviors = voxy_scene::BehaviorCheckpointRegistry::default();
        let actions = voxy_gameplay::UiActionCheckpointRegistry::default();
        let mut group = checkpoint
            .prepare(crate::app::restore_barrier::CoordinatedPrepare {
                compatibility: "ui-file-v1",
                registry: &project.registry,
                behaviors: &behaviors,
                ui_actions: &actions,
                root: &target,
                resources: crate::play_session::checkpoint::tests::resource_restore_limits(),
                play: crate::play_session::checkpoint::tests::limits(),
                ui: limits(),
                max_audio_bytes: policy.max_audio_bytes,
                restored_ui_cache: cache.state(),
            })
            .unwrap();
        assert!(group.ui.is_some() && group.ui_input.is_some());
        let restored_scene = &group.game.as_ref().unwrap().game.scene;
        assert_ne!(restored_scene.graph.identity(), scene.identity());
        let mut restored_audio = crate::audio_session::AudioSession::default();
        restored_audio.checkpoint_publication().unwrap().publish(
            match group.audio.take().unwrap() {
                crate::app::restore_barrier::PreparedCoordinatedAudio::Offline(audio) => *audio,
                crate::app::restore_barrier::PreparedCoordinatedAudio::Native(_) => {
                    panic!("offline fixture")
                }
            },
        );
        assert!(restored_audio.play().unwrap().frames > 0);
        restored_audio.request_checkpoint().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let audio = loop {
            if let Some(saved) = restored_audio
                .poll_checkpoint(
                    |node| restored_scene.identity(node).cloned(),
                    policy.max_audio_bytes,
                )
                .unwrap()
            {
                break saved;
            }
            restored_audio
                .play_mut()
                .unwrap()
                .poll_reload(&restored_scene.graph)
                .unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        };
        assert_eq!(
            serde_json::to_value(audio).unwrap()["play"]["playback"],
            serde_json::to_value(&checkpoint.audio).unwrap()["play"]["playback"]
        );
        restored_audio.close_play_audio();
        assert!(restored_audio.join_workers().is_empty());
        for worker in group.discard() {
            worker.join().unwrap();
        }
        assert!(std::fs::read_dir(&target).unwrap().next().is_none());
        checkpoint.ui
    } else {
        UiLiveCheckpoint::from_bytes(&std::fs::read(root.join("ui.live")).unwrap(), limits())
            .unwrap()
    };
    let ring = ui_draw::FocusRing {
        owner,
        viewport: [128.; 2],
        clip: [10., 10., 70., 30.],
    };
    cache.geometry_live = draws
        .iter()
        .map(|draw| draw.geometry.allocation_bytes())
        .sum();
    let mut focus = upload_on(
        &renderer,
        &device,
        &queue,
        &mut cache,
        &[ring.mesh().unwrap()],
    )
    .unwrap();
    let mut focus_key = Some(ring);
    let mut focus_deferred = None;
    let inventory = draws
        .iter()
        .chain(&focus)
        .map(|draw| draw.geometry.allocation_bytes())
        .sum::<u64>();
    cache.geometry_live = inventory;
    let before_cache = cache.state();
    let before_images = cache.live_bytes();
    let before_text = ui.published_text.as_ref().unwrap().clone();
    let before_texture = draws[1].texture.as_ref().unwrap().clone();
    macro_rules! target {
        () => {
            UiGraphicsTarget {
                cache: &mut cache,
                draws: &mut draws,
                focus: &mut focus,
                focus_key: &mut focus_key,
                focus_deferred: &mut focus_deferred,
                presented: &mut displayed,
                geometry_live: inventory,
            }
        };
    }
    if held_barrier {
        let ready_barrier = ui.checkpoint.take();
        assert!(
            ui.publication_on_mode(target!(), &renderer, &device, &queue, true)
                .is_err()
        );
        ui.checkpoint = Some(reload_checkpoint::Barrier::Draining);
        assert!(
            ui.publication_on_mode(target!(), &renderer, &device, &queue, true)
                .is_err()
        );
        assert!(matches!(
            ui.checkpoint,
            Some(reload_checkpoint::Barrier::Draining)
        ));
        ui.checkpoint = ready_barrier;
    }
    if held_barrier {
        assert!(
            ui.publication_on(target!(), &renderer, &device, &queue)
                .is_err()
        );
        assert!(matches!(
            ui.checkpoint,
            Some(reload_checkpoint::Barrier::Ready(_))
        ));
    }
    macro_rules! permit {
        () => {
            ui.publication_on_mode(target!(), &renderer, &device, &queue, held_barrier)
                .unwrap()
        };
    }
    macro_rules! intact {
        () => {{
            if held_barrier {
                assert!(matches!(
                    ui.checkpoint,
                    Some(reload_checkpoint::Barrier::Ready(_))
                ));
            }
            assert_eq!(cache.state(), before_cache);
            assert_eq!(cache.live_bytes(), before_images);
            assert!(Arc::ptr_eq(
                ui.published_text.as_ref().unwrap(),
                &before_text
            ));
            assert!(Arc::ptr_eq(
                draws[1].texture.as_ref().unwrap(),
                &before_texture
            ));
            assert!(focus_key.is_some() && !focus.is_empty() && displayed);
            assert_eq!(pixels(&renderer, &device, &queue, &draws), target_pixels);
        }};
    }
    let mut prepared = saved
        .prepare(&view, &project, cache.state(), limits())
        .unwrap();
    let original_geometry_budget = cache.geometry_budget;
    cache.geometry_budget = inventory;
    let rejected = match permit!().stage(prepared) {
        Err(rejected) => rejected,
        Ok(_) => panic!("replacement peak geometry must be admitted"),
    };
    assert!(rejected.error.contains("geometry peak budget"));
    prepared = *rejected.prepared;
    cache.geometry_budget = original_geometry_budget;
    intact!();
    let original_image_budget = cache.budget;
    cache.budget = before_images;
    let rejected = match permit!().stage(prepared) {
        Err(rejected) => rejected,
        Ok(_) => panic!("replacement peak images must be admitted"),
    };
    assert!(rejected.error.contains("image budget"));
    prepared = *rejected.prepared;
    cache.budget = original_image_budget;
    intact!();
    let rejected =
        match permit!().stage_with(prepared, || Err("late cross-owner validation".into())) {
            Err(rejected) => rejected,
            Ok(_) => panic!("late rejection must fail"),
        };
    assert!(rejected.error.contains("late cross-owner"));
    prepared = *rejected.prepared;
    intact!();
    let pending = permit!()
        .stage_with(prepared, || {
            let _invalid = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("injected late GPU validation fault"),
                size: 4,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            Ok(())
        })
        .unwrap_or_else(|rejected| panic!("unexpected staging rejection: {}", rejected.error));
    let rejected = match pollster::block_on(pending.validate()) {
        Err(rejected) => rejected,
        Ok(_) => panic!("actual GPU validation must reject"),
    };
    assert!(rejected.error.contains("Buffer") || rejected.error.contains("buffer"));
    prepared = *rejected.prepared;
    intact!();
    // Validated-but-discarded and pending-but-discarded candidates also leave
    // native resources untouched, including the weak cache index epoch.
    let pending = permit!()
        .stage(prepared)
        .unwrap_or_else(|e| panic!("{}", e.error));
    let ready = pollster::block_on(pending.validate()).unwrap_or_else(|e| panic!("{}", e.error));
    ready.discard().unwrap().join().unwrap();
    intact!();
    let prepared = saved
        .prepare(&view, &project, cache.state(), limits())
        .unwrap();
    permit!()
        .stage(prepared)
        .unwrap_or_else(|e| panic!("{}", e.error))
        .discard()
        .unwrap()
        .join()
        .unwrap();
    intact!();
    let release = if !held_barrier {
        // Keep a real active import owned by the destination until publication.
        let (release, waiting) = std::sync::mpsc::channel();
        let mut worker = voxy_assets::AssetImportWorker::new(
            project.input_provider().unwrap(),
            1,
            1,
            move |_, _, _| {
                waiting.recv().unwrap();
                Err::<ui_text::UiTextPreparation, _>("retired import".to_string())
            },
        )
        .unwrap();
        let mut catalog = voxy_assets::AssetCatalog::<
            voxy_assets::ImportedAsset<ui_text::UiTextPreparation>,
        >::new(1, 1)
        .unwrap();
        worker
            .submit(&catalog.request(AssetId("game-ui-text".into())).unwrap())
            .unwrap();
        ui.worker = Some(worker);
        Some(release)
    } else {
        None
    };
    let prepared = saved
        .prepare(&view, &project, cache.state(), limits())
        .unwrap();
    let pending = permit!()
        .stage(prepared)
        .unwrap_or_else(|e| panic!("{}", e.error));
    let ready = pollster::block_on(pending.validate()).unwrap_or_else(|e| panic!("{}", e.error));
    let retired = ready.publish();
    if let Some(release) = release {
        assert!(retired.len() >= 2 && retired.iter().any(|handle| !handle.is_finished()));
        release.send(()).unwrap();
    } else {
        assert!(!retired.is_empty());
        assert!(ui.checkpoint.is_none());
    }
    for handle in retired {
        handle.join().unwrap();
    }
    assert!(focus.is_empty() && focus_key.is_none() && focus_deferred.is_none() && !displayed);
    assert_eq!(
        cache.geometry_live,
        draws
            .iter()
            .map(|draw| draw.geometry.allocation_bytes())
            .sum::<u64>()
    );
    assert!(ui.worker.is_none());
    assert!(ui.presented().is_none() && ui.focus_snapshot().is_none());
    assert!(
        ui.presented.is_some(),
        "CPU visible history must not be discarded"
    );
    let restored_pixels = pixels(&renderer, &device, &queue, &draws);
    assert_eq!(restored_pixels, saved_pixels);
    if let Some(directory) = std::env::var_os("VOXY_UI_RESTORE_EVIDENCE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let mut records = serde_json::Map::new();
        for (name, bytes) in [
            ("saved-reference", &saved_pixels),
            ("target-before", &target_pixels),
            ("restored-after", &restored_pixels),
        ] {
            std::fs::write(directory.join(format!("{name}.rgba")), bytes).unwrap();
            records.insert(name.into(), serde_json::json!({"blake3": blake3::hash(bytes).to_hex().to_string(), "bytes": bytes.len()}));
        }
        records.insert("dimensions".into(), serde_json::json!([128, 128]));
        records.insert("adapter".into(), serde_json::json!(format!("{info:?}")));
        std::fs::write(
            directory.join("readback.json"),
            serde_json::to_vec_pretty(&records).unwrap(),
        )
        .unwrap();
    }
    ui.frame_presented();
    let mut input = crate::game_ui_input::GameUiInput::new();
    assert!(
        input
            .admit_presented(&scene, [128.; 2], ui.presented())
            .unwrap()
    );
    if with_models {
        let provider = voxy_assets::FileInputs::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../voxy_render/examples/assets"),
        )
        .unwrap();
        let mut inputs = voxy_assets::ImportInputs::new(4, 4 * 1024 * 1024);
        let mut asset = crate::import::load(
            &voxy_assets::SourcePath::new("animated-triangle.glb").unwrap(),
            &provider,
            &mut inputs,
        )
        .unwrap();
        asset.images.push(
            voxy_render::ImageAsset::from_rgba(
                1,
                1,
                vec![11, 191, 73, 255],
                voxy_render::ImageLimits::default(),
            )
            .unwrap(),
        );
        asset.nodes.push(crate::import::ImportedNode {
            name: "textured-part".into(),
            parent: None,
            local: Transform::default(),
            mesh: Some(asset.mesh.clone()),
            image: Some(0),
            sampling: voxy_render::TextureSampling::default(),
            use_mips: true,
            geometry_key: Some((0, 0)),
        });
        let original_image_bytes = cache.live_bytes();
        let publication = Arc::new(
            inputs
                .finish(asset, |id, max| provider.read(id, max))
                .unwrap(),
        );
        let bindings = crate::gpu_source_bindings::PreparedBindings {
            scene: scene.identity(),
            models: [(AssetId("model".into()), publication)].into(),
            owners: Default::default(),
        };
        let inventory = draws
            .iter()
            .map(|draw| draw.geometry.allocation_bytes())
            .sum::<u64>();
        cache.geometry_live = inventory;
        ui.begin_checkpoint().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while ui.advance_checkpoint().unwrap().is_none() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        macro_rules! models {
            () => {
                pollster::block_on(
                    crate::gpu_model::batch::UploadTarget {
                        renderer: &renderer,
                        device: &device,
                        queue: &queue,
                        cache: &cache,
                        live_geometry: inventory,
                        replaced_geometry: 0,
                    }
                    .stage_restore(&scene, &bindings, Default::default())
                    .unwrap()
                    .validate(),
                )
                .unwrap()
            };
        }
        macro_rules! shared_permit {
            () => {
                ui.publication_on_mode(
                    UiGraphicsTarget {
                        cache: &mut cache,
                        draws: &mut draws,
                        focus: &mut focus,
                        focus_key: &mut focus_key,
                        focus_deferred: &mut focus_deferred,
                        presented: &mut displayed,
                        geometry_live: inventory,
                    },
                    &renderer,
                    &device,
                    &queue,
                    true,
                )
                .unwrap()
            };
        }
        let mut foreign = SceneGraph::new(4);
        let foreign_owner = foreign.spawn(None, Transform::default()).unwrap();
        foreign
            .insert_component(
                foreign_owner,
                scene.components::<UiElement>().next().unwrap().1.clone(),
            )
            .unwrap();
        let foreign_view =
            SceneCheckpointView::new(&foreign, [(ObjectId("label".into()), foreign_owner)])
                .unwrap();
        let foreign_ui = saved
            .prepare(&foreign_view, &project, cache.state(), limits())
            .unwrap();
        let model_stage = models!();
        let rejected = match shared_permit!().stage_after_models(foreign_ui, &scene, model_stage) {
            Err(rejected) => rejected,
            Ok(_) => panic!("foreign target UI must reject before upload"),
        };
        assert!(rejected.ui.error.contains("target scene"));
        rejected.ui.discard().unwrap().join().unwrap();
        drop(rejected.models);
        let model_stage = models!();
        let combined_limit = model_stage.cache.geometry_live + inventory - 1;
        drop(model_stage);
        let budget = cache.geometry_budget;
        cache.geometry_budget = combined_limit;
        let model_stage = models!();
        let state = cache.state();
        let prepared = saved
            .prepare(&view, &project, cache.state(), limits())
            .unwrap();
        let rejected = match shared_permit!().stage_after_models(prepared, &scene, model_stage) {
            Err(rejected) => rejected,
            Ok(_) => panic!("combined old + models + UI peak must reject"),
        };
        assert!(rejected.ui.error.contains("geometry peak budget"));
        rejected.ui.discard().unwrap().join().unwrap();
        drop(rejected.models);
        assert_eq!(cache.state(), state);
        assert!(matches!(
            ui.checkpoint,
            Some(reload_checkpoint::Barrier::Ready(_))
        ));
        assert_eq!(pixels(&renderer, &device, &queue, &draws), saved_pixels);
        cache.geometry_budget = budget;
        let models = models!();
        let fork = cache.fork_for_upload();
        let original_cache = std::mem::replace(&mut cache, fork);
        let prepared = saved
            .prepare(&view, &project, cache.state(), limits())
            .unwrap();
        let rejected = match shared_permit!().stage_after_models(prepared, &scene, models) {
            Err(rejected) => rejected,
            Ok(_) => panic!("different original cache identity must reject"),
        };
        assert!(rejected.ui.error.contains("destination changed"));
        cache = original_cache;
        let pending = shared_permit!()
            .stage_after_models(*rejected.ui.prepared, &scene, *rejected.models)
            .unwrap_or_else(|e| panic!("{}", e.ui.error));
        let ready =
            pollster::block_on(pending.validate()).unwrap_or_else(|e| panic!("{}", e.ui.error));
        ready.discard().unwrap().join().unwrap();
        assert!(matches!(
            ui.checkpoint,
            Some(reload_checkpoint::Barrier::Ready(_))
        ));
        assert_eq!(pixels(&renderer, &device, &queue, &draws), saved_pixels);
        let models = models!();
        let expected_model_bytes = models.cache.geometry_live - inventory;
        let prepared = saved
            .prepare(&view, &project, cache.state(), limits())
            .unwrap();
        let pending = shared_permit!()
            .stage_after_models(prepared, &scene, models)
            .unwrap_or_else(|e| panic!("{}", e.ui.error));
        let ready =
            pollster::block_on(pending.validate()).unwrap_or_else(|e| panic!("{}", e.ui.error));
        let (models, retired) = ready.publish();
        for worker in retired {
            worker.join().unwrap();
        }
        assert_eq!(models.scene, scene.identity());
        assert_eq!(models.models.len(), 1);
        assert_eq!(cache.live_bytes(), original_image_bytes + 4);
        assert_eq!(models.animated.allocation_bytes(), 0);
        assert_eq!(
            cache.geometry_live,
            expected_model_bytes
                + draws
                    .iter()
                    .map(|draw| draw.geometry.allocation_bytes())
                    .sum::<u64>()
        );
        assert_eq!(pixels(&renderer, &device, &queue, &draws), saved_pixels);
        assert!(ui.presented().is_none() && ui.focus_snapshot().is_none());
        assert!(ui.checkpoint.is_none());
        ui.frame_presented();
        assert!(ui.presented().is_some());
        if let Some(directory) = std::env::var_os("VOXY_UI_RESTORE_EVIDENCE_DIR") {
            let directory = std::path::PathBuf::from(directory);
            let combined_pixels = pixels(&renderer, &device, &queue, &draws);
            assert_eq!(combined_pixels, saved_pixels);
            std::fs::write(directory.join("combined-final.rgba"), combined_pixels).unwrap();
            std::fs::write(
                directory.join("combined-accounting.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "source_ui_geometry": inventory,
                    "new_model_geometry": expected_model_bytes,
                    "final_geometry": cache.geometry_live,
                    "rejected_geometry_limit": combined_limit,
                    "source_image_bytes": original_image_bytes,
                    "final_image_bytes": cache.live_bytes(),
                    "model_texture_bytes": 4,
                }))
                .unwrap(),
            )
            .unwrap();
        }
        println!(
            "COMBINED MODEL/UI cache retains model geometry and exact UI pixels; old+new peak enforced"
        );
    }
    for handle in ui.close() {
        handle.join().unwrap();
    }
    assert!(pollster::block_on(errors.pop()).is_none());
    println!(
        "UI ATOMIC RESTORE exact RGBA readback; late CPU/GPU faults and discard preserve live UI/cache; old owners retired"
    );
}

#[test]
fn detached_observation_detects_first_presentation_without_layout_changes() {
    let snapshot = SceneUiSnapshot {
        viewport: [128.; 2],
        elements: Vec::new(),
    };
    let mut ui = UiLive {
        attempted: Some(snapshot.clone()),
        published: Some(snapshot.clone()),
        presented: Some(snapshot),
        awaiting_first_presentation: true,
        ..UiLive::default()
    };
    let observed = PublicationObservation::capture(&ui);
    assert!(observed.matches(&ui));
    ui.frame_presented();
    assert_eq!(observed.published, ui.published);
    assert_eq!(observed.presented, ui.presented);
    assert!(!observed.matches(&ui));
}
