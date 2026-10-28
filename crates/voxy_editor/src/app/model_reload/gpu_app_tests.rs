//! Native App import/watch/publication with real GPU resources and no window.
use super::*;
use voxy_render::{SceneMesh, SceneVertex};
#[test]
#[ignore = "requires real GPU; full App model watcher/reload without presentation"]
fn offscreen_app_sampler_reload_keeps_cpu_gpu_atomic_and_recovers_failed_source() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("OFFSCREEN APP GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let temp = crate::project_prefab_tests::Temp::new();
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/scene3d/assembly.glb"),
    )
    .unwrap();
    let json_length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let mut spec: serde_json::Value = serde_json::from_slice(&bytes[20..20 + json_length]).unwrap();
    let binary = &bytes[28 + json_length..];
    spec["samplers"] =
        serde_json::json!([{"minFilter":9987,"magFilter":9729,"wrapS":10497,"wrapT":10497}]);
    spec["textures"][0]["sampler"] = 0.into();
    let root = temp.0.join("game");
    let project = project::create(&root, "GPU reload package").unwrap();
    let source = root.join("assembly.glb");
    crate::scene3d_tests::write_glb(&source, &spec, binary);
    std::fs::write(
        root.join("assets.json"),
        br#"{"version":1,"assets":[{"asset":"assembly.glb","source":"assembly.glb"}]}"#,
    )
    .unwrap();
    let mut app = App::from_project(&project).unwrap();
    app.edit_key(KeyCode::KeyD).unwrap();
    let id = AssetId("assembly.glb".into());
    await_state(&mut app, |app| app.catalog.snapshot(&id).is_some());
    let before = app.catalog.snapshot(&id).unwrap();
    app.expand_model().unwrap();
    app.edit_key(KeyCode::ArrowRight).unwrap();
    app.camera.legacy = false;
    app.camera.perspective = true;
    app.edit_key(KeyCode::KeyG).unwrap();
    let fog = app
        .scene
        .spawn(
            None,
            Transform {
                translation: Vec3::splat(-2.),
                ..Default::default()
            },
        )
        .unwrap();
    app.scene.set_name(fog, "Fog volume").unwrap();
    app.scene
        .insert_component(
            fog,
            voxy_scene::FogVolume {
                size: [4.; 3],
                ..Default::default()
            },
        )
        .unwrap();
    app.instances.push(fog);
    app.object_ids
        .push(voxy_scene::ObjectId("fog-volume".into()));
    let light_owner = app.instances[0];
    app.scene
        .insert_component(
            light_owner,
            DirectionalLight {
                direction: [1., 1., 1.],
                intensity: 4.,
            },
        )
        .unwrap();
    app.scene
        .insert_component(light_owner, crate::DirectionalShadow::default())
        .unwrap();
    let casters: Vec<_> = app
        .scene
        .components::<ModelPart>()
        .filter(|(_, part)| part.node != u32::MAX)
        .map(|(owner, _)| owner)
        .collect();
    for owner in casters {
        app.scene
            .insert_component(owner, crate::OpaqueShadowCaster::default())
            .unwrap();
    }
    let moving_owner = app
        .scene
        .components::<ModelPart>()
        .find(|(_, part)| part.node == 1)
        .unwrap()
        .0;
    app.scene
        .insert_component(
            moving_owner,
            voxy_gameplay::AngularMotion {
                axis: [0., 1., 0.],
                radians_per_second: 6.,
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();
    app.save_authoring().unwrap();
    let document = app.authoring_document().unwrap();
    app.graphics = Some(
        Graphics::new(
            adapter.clone(),
            device.clone(),
            queue.clone(),
            SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm),
            None,
            None,
            (256 * 1024 * 1024, 256 * 1024 * 1024),
        )
        .unwrap(),
    );
    await_state(&mut app, |app| {
        app.graphics.as_ref().unwrap().models.contains_key(&id)
    });
    assert!(app.window.is_none());
    assert!(app.graphics.as_ref().unwrap().host.is_none());
    let graphics = app.graphics.as_ref().unwrap();
    let mesh = SceneMesh::new(
        vec![
            SceneVertex {
                position: [-1., 1., 0.],
                uv: [0., 0.],
                color: [1.; 4],
            },
            SceneVertex {
                position: [-1., -1., 0.],
                uv: [0., 1.],
                color: [1.; 4],
            },
            SceneVertex {
                position: [1., -1., 0.],
                uv: [1., 1.],
                color: [1.; 4],
            },
            SceneVertex {
                position: [1., 1., 0.],
                uv: [1., 0.],
                color: [1.; 4],
            },
        ],
        vec![0, 1, 2, 0, 2, 3],
    )
    .unwrap();
    let geometry = graphics.renderer.upload_mesh(&device, &mesh).unwrap();
    let transform = graphics
        .renderer
        .create_transform(&device, Mat4::IDENTITY)
        .unwrap();
    let render = |app: &App| {
        let graphics = app.graphics.as_ref().unwrap();
        crate::gpu_model::sampler_tests::pixels(
            &graphics.renderer,
            &device,
            &queue,
            &geometry,
            &texture(app, &id),
            &transform,
        )
    };
    let old_texture = texture(&app, &id);
    let original = render(&app);
    let old_residency = app.graphics.as_ref().unwrap().residency_cache.live_bytes();
    spec["samplers"][0] =
        serde_json::json!({"minFilter":9728,"magFilter":9728,"wrapS":33071,"wrapT":33071});
    crate::scene3d_tests::write_glb(&source, &spec, binary);
    let mut observed_pending = false;
    let until = Instant::now() + Duration::from_secs(10);
    while Arc::ptr_eq(&before, &app.catalog.snapshot(&id).unwrap()) {
        app.tick().unwrap();
        if app.pending_model.is_some() {
            observed_pending = true;
            assert!(Arc::ptr_eq(&before, &app.catalog.snapshot(&id).unwrap()));
            assert!(Arc::ptr_eq(
                &before,
                app.graphics.as_ref().unwrap().models[&id]
                    .publication
                    .as_ref()
                    .unwrap()
            ));
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(observed_pending);
    let accepted = app.catalog.snapshot(&id).unwrap();
    assert!(Arc::ptr_eq(
        &accepted,
        app.graphics.as_ref().unwrap().models[&id]
            .publication
            .as_ref()
            .unwrap()
    ));
    let new_texture = texture(&app, &id);
    assert!(!Arc::ptr_eq(&old_texture, &new_texture));
    assert!(std::ptr::eq(old_texture.texture(), new_texture.texture()));
    assert_eq!(
        app.graphics.as_ref().unwrap().residency_cache.live_bytes(),
        old_residency
    );
    let changed = render(&app);
    assert_ne!(original, changed);
    assert_eq!(app.authoring_document().unwrap(), document);
    std::fs::write(&source, b"broken").unwrap();
    await_state(&mut app, |app| {
        matches!(
            app.catalog.status(&id),
            Some(voxy_assets::AssetStatus::Failed(_))
        )
    });
    assert!(Arc::ptr_eq(&accepted, &app.catalog.snapshot(&id).unwrap()));
    assert!(Arc::ptr_eq(
        &accepted,
        app.graphics.as_ref().unwrap().models[&id]
            .publication
            .as_ref()
            .unwrap()
    ));
    assert_eq!(changed, render(&app));
    crate::scene3d_tests::write_glb(&source, &spec, binary);
    await_state(&mut app, |app| {
        !Arc::ptr_eq(&accepted, &app.catalog.snapshot(&id).unwrap())
    });
    let recovered = app.catalog.snapshot(&id).unwrap();
    assert!(Arc::ptr_eq(
        &recovered,
        app.graphics.as_ref().unwrap().models[&id]
            .publication
            .as_ref()
            .unwrap()
    ));
    assert_eq!(changed, render(&app));
    assert_eq!(app.authoring_document().unwrap(), document);
    assert!(app.pending_model.is_none());

    // Valid input that cannot be admitted must not strand the native import.
    let geometry_limit = app
        .graphics
        .as_ref()
        .unwrap()
        .residency_cache
        .geometry_budget;
    app.graphics
        .as_mut()
        .unwrap()
        .residency_cache
        .geometry_budget = 0;
    let cache_state = app.graphics.as_ref().unwrap().residency_cache.state();
    let cache_owner = app.graphics.as_ref().unwrap().residency_cache.identity();
    let geometry_owner =
        std::ptr::from_ref(app.graphics.as_ref().unwrap().models[&id].geometry.base());
    let retained_texture = texture(&app, &id);
    spec["samplers"][0] =
        serde_json::json!({"minFilter":9987,"magFilter":9729,"wrapS":10497,"wrapT":10497});
    crate::scene3d_tests::write_glb(&source, &spec, binary);
    let retry_source = std::fs::read(&source).unwrap();
    await_state(
        &mut app,
        |app| matches!(app.catalog.status(&id), Some(voxy_assets::AssetStatus::Failed(error)) if error.contains("GPU geometry budget exceeded")),
    );
    assert_eq!(app.catalog.pending(), 0);
    assert!(app.pending_model.is_none());
    assert!(Arc::ptr_eq(&recovered, &app.catalog.snapshot(&id).unwrap()));
    let graphics = app.graphics.as_ref().unwrap();
    assert!(Arc::ptr_eq(
        &recovered,
        graphics.models[&id].publication.as_ref().unwrap()
    ));
    assert!(std::ptr::eq(
        geometry_owner,
        graphics.models[&id].geometry.base()
    ));
    assert_eq!(graphics.residency_cache.state(), cache_state);
    assert!(Arc::ptr_eq(
        &graphics.residency_cache.identity(),
        &cache_owner
    ));
    assert!(Arc::ptr_eq(&retained_texture, &texture(&app, &id)));
    assert_eq!(changed, render(&app));
    app.begin_model_checkpoint().unwrap();
    let until = Instant::now() + Duration::from_secs(10);
    while !app.poll_model_checkpoint().unwrap() {
        assert!(
            Instant::now() < until,
            "GPU refusal stranded the model checkpoint"
        );
        std::thread::yield_now();
    }
    app.resume_model_checkpoint().unwrap();
    app.graphics
        .as_mut()
        .unwrap()
        .residency_cache
        .geometry_budget = geometry_limit;
    assert!(app.can_retry_selected_model());
    app.panel_action(panels::Action::RetryModel).unwrap();
    await_state(&mut app, |app| {
        !Arc::ptr_eq(&recovered, &app.catalog.snapshot(&id).unwrap())
    });
    let retried = app.catalog.snapshot(&id).unwrap();
    assert!(Arc::ptr_eq(
        &retried,
        app.graphics.as_ref().unwrap().models[&id]
            .publication
            .as_ref()
            .unwrap()
    ));
    assert_eq!(std::fs::read(&source).unwrap(), retry_source);
    assert_eq!(original, render(&app));
    assert_eq!(app.authoring_document().unwrap(), document);
    assert_eq!(app.catalog.pending(), 0);
    assert!(app.pending_model.is_none());
    app.start_standalone().unwrap();
    let before_frame = app.frames;
    let before_ticks = app.play.simulation_ticks;
    app.play.simulation_time = Instant::now()
        .checked_sub(Duration::from_millis(100))
        .unwrap();
    let before_wall_time = app.play.simulation_time;
    let viewport_pixels = scene_pixels(&mut app);
    assert_eq!(
        app.play.simulation_ticks, before_ticks,
        "offscreen render advanced gameplay clock"
    );
    assert_eq!(app.play.simulation_time, before_wall_time);
    let fog_owner = app
        .scene
        .components::<voxy_scene::FogVolume>()
        .next()
        .unwrap()
        .0;
    app.scene
        .component_mut::<voxy_scene::FogVolume>(fog_owner)
        .unwrap()
        .unwrap()
        .enabled = false;
    let without_fog = scene_pixels(&mut app);
    assert_ne!(
        viewport_pixels, without_fog,
        "fog was omitted from offscreen scene"
    );
    app.scene
        .component_mut::<voxy_scene::FogVolume>(fog_owner)
        .unwrap()
        .unwrap()
        .enabled = true;
    assert_eq!(scene_pixels(&mut app), viewport_pixels);
    let light_owner = app
        .scene
        .components::<crate::DirectionalShadow>()
        .next()
        .unwrap()
        .0;
    app.scene
        .component_mut::<crate::DirectionalShadow>(light_owner)
        .unwrap()
        .unwrap()
        .enabled = false;
    let without_shadows = scene_pixels(&mut app);
    assert_ne!(
        viewport_pixels, without_shadows,
        "opaque shadows were omitted from fog transport"
    );
    app.scene
        .component_mut::<crate::DirectionalShadow>(light_owner)
        .unwrap()
        .unwrap()
        .enabled = true;
    assert_eq!(scene_pixels(&mut app), viewport_pixels);
    let casters: Vec<_> = app
        .scene
        .components::<crate::OpaqueShadowCaster>()
        .map(|(owner, _)| owner)
        .collect();
    assert!(!casters.is_empty());
    for owner in &casters {
        app.scene
            .component_mut::<crate::OpaqueShadowCaster>(*owner)
            .unwrap()
            .unwrap()
            .enabled = false;
    }
    let without_casters = scene_pixels(&mut app);
    assert_ne!(
        viewport_pixels, without_casters,
        "shadow map did not depend on scene geometry"
    );
    for owner in casters {
        app.scene
            .component_mut::<crate::OpaqueShadowCaster>(owner)
            .unwrap()
            .unwrap()
            .enabled = true;
    }
    assert_eq!(scene_pixels(&mut app), viewport_pixels);
    assert_eq!(
        app.frames, before_frame,
        "offscreen submission acknowledged presentation"
    );
    assert!(!app.graphics.as_ref().unwrap().ui_presented);
    assert!(
        viewport_pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[..3] != [0, 0, 0])
            .count()
            > 16,
        "scene frame is empty"
    );
    let moving_owner = app
        .scene
        .components::<voxy_gameplay::AngularMotion>()
        .next()
        .unwrap()
        .0;
    let world_before = app.scene.world_matrix(moving_owner).unwrap();
    app.advance_game(4.0 / 60.0).unwrap();
    assert_eq!(app.play.simulation_ticks, before_ticks + 4);
    assert_ne!(app.scene.world_matrix(moving_owner).unwrap(), world_before);
    let moved_pixels = scene_pixels(&mut app);
    assert_ne!(
        moved_pixels, viewport_pixels,
        "fixed-step movement did not reach the rendered frame"
    );
    assert_eq!(app.play.simulation_ticks, before_ticks + 4);
    app.stop_workers().unwrap();

    // Build from the repaired project, then deny all fallback to its source tree.
    let package = temp.0.join("game.vpak");
    export_project_package(&project, &package).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert!(!source.exists() && !project.exists());
    let materialized = packaged_game::MaterializedPackage::open(&package).unwrap();
    let mut restored = App::from_project(&materialized.project_path().unwrap()).unwrap();
    restored.graphics = Some(
        Graphics::new(
            adapter.clone(),
            device.clone(),
            queue.clone(),
            SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm),
            None,
            None,
            (256 * 1024 * 1024, 256 * 1024 * 1024),
        )
        .unwrap(),
    );
    await_state(&mut restored, |app| {
        app.graphics.as_ref().unwrap().models.contains_key(&id) && app.pending_model.is_none()
    });
    let fresh = restored.catalog.snapshot(&id).unwrap();
    assert!(!Arc::ptr_eq(&fresh, &retried));
    assert!(Arc::ptr_eq(
        &fresh,
        restored.graphics.as_ref().unwrap().models[&id]
            .publication
            .as_ref()
            .unwrap()
    ));
    assert_eq!(
        fresh.inputs().observations(),
        retried.inputs().observations()
    );
    assert!(!Arc::ptr_eq(
        &restored
            .graphics
            .as_ref()
            .unwrap()
            .residency_cache
            .identity(),
        &app.graphics.as_ref().unwrap().residency_cache.identity()
    ));
    assert!(!std::ptr::eq(
        texture(&restored, &id).texture(),
        texture(&app, &id).texture()
    ));
    assert_eq!(original, render(&restored));
    assert_eq!(restored.authoring_document().unwrap(), document);
    assert!(restored.window.is_none() && restored.graphics.as_ref().unwrap().host.is_none());
    restored.start_standalone().unwrap();
    let before_frame = restored.frames;
    assert_eq!(scene_pixels(&mut restored), viewport_pixels);
    assert_eq!(restored.frames, before_frame);
    assert!(!restored.graphics.as_ref().unwrap().ui_presented);
    restored.advance_game(4.0 / 60.0).unwrap();
    assert_eq!(scene_pixels(&mut restored), moved_pixels);
    assert_eq!(restored.play.simulation_ticks, 4);
    restored.stop_workers().unwrap();
    // A fresh fog/shadow owner first renders at the moved pose: stale maps from
    // the earlier pose in either running App cannot satisfy this oracle.
    let mut oracle = App::from_project(&materialized.project_path().unwrap()).unwrap();
    oracle.graphics = Some(
        Graphics::new(
            adapter.clone(),
            device.clone(),
            queue.clone(),
            SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm),
            None,
            None,
            (256 * 1024 * 1024, 256 * 1024 * 1024),
        )
        .unwrap(),
    );
    await_state(&mut oracle, |app| {
        app.graphics.as_ref().unwrap().models.contains_key(&id) && app.pending_model.is_none()
    });
    oracle.start_standalone().unwrap();
    oracle.advance_game(4.0 / 60.0).unwrap();
    assert!(oracle.graphics.as_ref().unwrap().fog.is_none());
    assert_eq!(scene_pixels(&mut oracle), moved_pixels);
    assert_eq!(oracle.play.simulation_ticks, 4);
    oracle.stop_workers().unwrap();
    run_packaged_game(&package, ViewportMode::GameCheck).unwrap();
    assert!(pollster::block_on(scope.pop()).is_none());
}
fn texture(app: &App, id: &AssetId) -> Arc<SceneTexture> {
    app.graphics.as_ref().unwrap().models[id]
        .parts
        .values()
        .find_map(|part| part.texture.clone())
        .unwrap()
}
fn await_state(app: &mut App, ready: impl Fn(&App) -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        app.tick().unwrap();
        if ready(app) {
            return;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn scene_pixels(app: &mut App) -> Vec<u8> {
    let graphics = app.graphics.as_ref().unwrap();
    let device = graphics.device.clone();
    let queue = graphics.queue.clone();
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("native scene frame"),
            size: wgpu::Extent3d {
                width: 128,
                height: 128,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = target(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let color_view = color.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    app.draw_frame(Some(voxy_render::SceneViewTargets {
        color: &color_view,
        depth: &depth_view,
        xray_depth: None,
        resolve: None,
        overlay_depth: &depth_view,
    }))
    .unwrap();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("native scene readback"),
        size: 128 * 128 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &color,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
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
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let bytes = buffer.slice(..).get_mapped_range().unwrap().to_vec();
    buffer.unmap();
    bytes
}

#[test]
#[ignore = "requires real GPU; native optical liquid viewport and source-free package"]
fn offscreen_optical_liquid_uses_native_runtime_and_matches_packaged_frame() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("OFFSCREEN LIQUID APP GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let temp = crate::project_prefab_tests::Temp::new();
    let root = temp.0.join("game");
    let project = project::create(&root, "Optical liquid").unwrap();
    std::fs::write(
        root.join("background.obj"),
        "v -1 -1 0\nv 1 -1 0\nv 1 1 0\nv -1 1 0\nf 1 2 3\nf 1 3 4\n",
    )
    .unwrap();
    std::fs::write(
        root.join("liquid.json"),
        br#"{"rest_density":1000,"sound_speed":20,"viscosity":0.001,"optics":[3,0.5,0.2,1.333]}"#,
    )
    .unwrap();
    std::fs::write(root.join("assets.json"), br#"{"version":1,"assets":[{"asset":"background","source":"background.obj"},{"asset":"liquid","source":"liquid.json"}]}"#).unwrap();
    let mut app = App::from_project(&project).unwrap();
    app.edit_key(KeyCode::KeyD).unwrap();
    app.camera.legacy = false;
    app.camera.perspective = true;
    app.edit_key(KeyCode::KeyG).unwrap();
    let mut document = app.authoring_document().unwrap();
    let mut jet = document.objects[0].clone();
    jet.id = voxy_scene::ObjectId("jet".into());
    jet.name = "Liquid source".into();
    jet.translation = [0., 0., 0.9];
    jet.components.clear();
    jet.components.insert(
        "game.liquid-source.v1".into(),
        serde_json::to_value(voxy_gameplay::LiquidSource {
            pulses: vec![voxy_gameplay::LiquidPulse {
                start_s: 0.,
                duration_s: 1. / 60.,
                volume_m3: 0.008,
                speed_m_s: 0.,
            }],
            density_kg_m3: 1000.,
            particle_volume_m3: 0.008,
            nozzle_radius_m: 0.,
            direction: [0., 0., -1.],
            material_asset: "liquid".into(),
        })
        .unwrap(),
    );
    document.objects.push(jet);
    app.authoring
        .history
        .as_mut()
        .unwrap()
        .commit(document.clone(), &app.authoring.authoring_project.registry)
        .unwrap();
    app.restore_authoring().unwrap();
    app.save_authoring().unwrap();
    app.graphics = Some(
        Graphics::new(
            adapter.clone(),
            device.clone(),
            queue.clone(),
            SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm),
            None,
            None,
            (256 * 1024 * 1024, 256 * 1024 * 1024),
        )
        .unwrap(),
    );
    app.start_standalone().unwrap();
    await_state(&mut app, |app| {
        app.graphics
            .as_ref()
            .unwrap()
            .models
            .contains_key(&AssetId("background".into()))
    });
    let empty = scene_pixels(&mut app);
    assert!(
        app.play
            .liquid
            .as_ref()
            .unwrap()
            .liquid()
            .particles()
            .is_empty()
    );
    app.advance_game(1. / 60.).unwrap();
    assert_eq!(
        app.play.liquid.as_ref().unwrap().liquid().particles().len(),
        1
    );
    let state = |app: &App| {
        app.play
            .liquid
            .as_ref()
            .unwrap()
            .liquid()
            .particles()
            .iter()
            .map(|p| {
                (
                    p.position.map(f64::to_bits),
                    p.velocity.map(f64::to_bits),
                    p.mass.to_bits(),
                    p.material,
                )
            })
            .collect::<Vec<_>>()
    };
    let accepted_state = state(&app);
    let wet = scene_pixels(&mut app);
    assert_ne!(wet, empty, "optical fluid made no change to scene pixels");
    assert_eq!(state(&app), accepted_state);
    assert_eq!(app.play.simulation_ticks, 1);
    assert_eq!(app.graphics.as_ref().unwrap().optical_liquids.len(), 1);
    assert!(app.graphics.as_ref().unwrap().optical_liquids[&0].allocation_bytes() > 0);
    assert_eq!(app.frames, 0);
    app.stop_workers().unwrap();
    let package = temp.0.join("liquid.vpak");
    export_project_package(&project, &package).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    let materialized = packaged_game::MaterializedPackage::open(&package).unwrap();
    let mut restored = App::from_project(&materialized.project_path().unwrap()).unwrap();
    restored.graphics = Some(
        Graphics::new(
            adapter.clone(),
            device.clone(),
            queue.clone(),
            SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm),
            None,
            None,
            (256 * 1024 * 1024, 256 * 1024 * 1024),
        )
        .unwrap(),
    );
    restored.start_standalone().unwrap();
    await_state(&mut restored, |app| {
        app.graphics
            .as_ref()
            .unwrap()
            .models
            .contains_key(&AssetId("background".into()))
    });
    assert_eq!(scene_pixels(&mut restored), empty);
    restored.advance_game(1. / 60.).unwrap();
    assert_eq!(state(&restored), accepted_state);
    assert_eq!(scene_pixels(&mut restored), wet);
    assert_eq!(restored.play.simulation_ticks, 1);
    assert_eq!(restored.frames, 0);
    assert!(restored.window.is_none() && restored.graphics.as_ref().unwrap().host.is_none());
    restored.stop_workers().unwrap();
    run_packaged_game(&package, ViewportMode::GameCheck).unwrap();
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "requires real GPU; 720 native animated scene frames and source-free replay"]
fn offscreen_nested_locomotion_cycle_matches_packaged_gpu_frames() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("LOCOMOTION CYCLE GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let fixture = crate::locomotion_tests::Temp::new();
    let project = crate::locomotion_tests::nested_morph_project(&fixture).unwrap();
    let root = project.parent().unwrap();
    let output_temp = crate::project_prefab_tests::Temp::new();
    let output = output_temp.0.join("gpu-cycle.vpak");
    let mut app = App::from_project(&project).unwrap();
    app.camera.legacy = false;
    app.camera.perspective = false;
    app.camera.distance = 8.;
    app.camera.target = Vec3::new(3., 0.5, 0.5);
    app.edit_key(KeyCode::KeyG).unwrap();
    app.save_authoring().unwrap();
    let attach = |app: &mut App| {
        app.graphics = Some(
            Graphics::new(
                adapter.clone(),
                device.clone(),
                queue.clone(),
                SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm),
                None,
                None,
                (256 * 1024 * 1024, 256 * 1024 * 1024),
            )
            .unwrap(),
        );
        await_state(app, |app| {
            app.graphics
                .as_ref()
                .unwrap()
                .models
                .contains_key(&AssetId("rig".into()))
        });
        // Settle the initial watcher inventory and its ordinary native reloads
        // before starting a controlled fixed-step comparison.
        for _ in 0..2 {
            app.last_scan = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
            await_state(app, |app| {
                !app.scan_active && app.last_scan.elapsed() < Duration::from_millis(200)
            });
            await_state(app, |app| {
                app.pending_model.is_none()
                    && app.reload.is_empty()
                    && app
                        .imports
                        .as_ref()
                        .is_none_or(|imports| !imports.is_busy())
            });
        }
        app.start_standalone().unwrap();
    };
    attach(&mut app);
    let owner = app.scene.components::<ModelLocomotion>().next().unwrap().0;
    let visible = scene_pixels(&mut app);
    app.scene.set_active(owner, false).unwrap();
    let hidden = scene_pixels(&mut app);
    assert!(
        !app.graphics
            .as_ref()
            .unwrap()
            .models
            .contains_key(&AssetId("rig".into())),
        "hidden model should release its GPU residency"
    );
    assert!(app.catalog.snapshot(&AssetId("rig".into())).is_some());
    app.scene.set_active(owner, true).unwrap();
    assert_ne!(visible, hidden, "animated fixture is not visible");
    let mut cpu_reference = Some(
        animated_models::AnimatedModels::new_cpu_reference(
            &app.graphics.as_ref().unwrap().renderer,
        )
        .unwrap(),
    );
    let mut step = |app: &mut App, tick: usize| {
        let phase = tick % 180;
        if phase == 20 {
            app.game_key(KeyCode::ArrowRight, ElementState::Pressed)
                .unwrap();
        }
        if phase == 40 {
            app.game_key(KeyCode::Space, ElementState::Pressed).unwrap();
            app.game_key(KeyCode::Space, ElementState::Released)
                .unwrap();
        }
        if phase == 120 {
            app.game_key(KeyCode::ArrowRight, ElementState::Released)
                .unwrap();
        }
        app.advance_game(1. / 60.).unwrap();
        let ticks = app.play.simulation_ticks;
        let frames = app.frames;
        let pixels = scene_pixels(app);
        let owner = app.scene.components::<ModelLocomotion>().next().unwrap().0;
        assert!(
            app.graphics
                .as_ref()
                .unwrap()
                .animated_models
                .geometry_inputs_for_view(owner, 0)
                .unwrap()
                .all(|input| input.gpu_deformed)
        );
        let gpu = std::mem::replace(
            &mut app.graphics.as_mut().unwrap().animated_models,
            cpu_reference.take().unwrap(),
        );
        let reference_pixels = scene_pixels(app);
        assert!(
            app.graphics
                .as_ref()
                .unwrap()
                .animated_models
                .geometry_inputs_for_view(owner, 0)
                .unwrap()
                .all(|input| !input.gpu_deformed)
        );
        cpu_reference = Some(std::mem::replace(
            &mut app.graphics.as_mut().unwrap().animated_models,
            gpu,
        ));
        let differences = pixels
            .iter()
            .zip(&reference_pixels)
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(
            differences, 0,
            "GPU deformation differs from CPU pixel oracle at tick {tick}"
        );
        assert_eq!(app.play.simulation_ticks, ticks);
        assert_eq!(app.frames, frames);
        pixels
    };
    let runtime_state = |app: &App| {
        let owner = app.scene.components::<ModelLocomotion>().next().unwrap().0;
        let published = app.catalog.snapshot(&AssetId("rig".into())).unwrap();
        let model = published.value().animated.as_ref().unwrap();
        format!(
            "{:?} {:?} {:?}",
            app.play.animations.clip_phase(owner),
            app.play.animations.frame(owner, model),
            app.authoring_document().unwrap()
        )
    };
    let mut expected = Vec::new();
    let mut states = Vec::new();
    let mut stationary_worlds = Vec::new();
    let mut clips = std::collections::BTreeSet::new();
    for tick in 0..720 {
        expected.push(step(&mut app, tick));
        states.push(runtime_state(&app));
        if tick == 5 || tick == 18 {
            stationary_worlds.push(app.scene.world_matrix(owner).unwrap());
        }
        clips.insert(app.play.animations.clip_name(owner).unwrap().to_owned());
        assert!(
            app.graphics
                .as_ref()
                .unwrap()
                .animated_models
                .pose_evidence(owner)
                .is_some()
        );
    }
    for name in ["Idle", "Walking", "Jump", "Fall"] {
        assert!(clips.contains(name));
    }
    assert_eq!(stationary_worlds[0], stationary_worlds[1]);
    assert_ne!(
        expected[5], expected[18],
        "deformation must change pixels while the model world transform is fixed"
    );
    assert!(expected.windows(2).any(|frames| frames[0] != frames[1]));
    assert_eq!(app.play.simulation_ticks, 720);
    let final_scene = app.authoring_document().unwrap();
    app.stop_workers().unwrap();
    export_project_package(&project, &output).unwrap();
    std::fs::remove_dir_all(root).unwrap();
    let package = packaged_game::MaterializedPackage::open(&output).unwrap();
    let mut restored = App::from_project(&package.project_path().unwrap()).unwrap();
    attach(&mut restored);
    assert_eq!(scene_pixels(&mut restored), visible);
    for (tick, pixels) in expected.iter().enumerate() {
        let actual = step(&mut restored, tick);
        assert_eq!(
            runtime_state(&restored),
            states[tick],
            "CPU state differs at tick {tick}"
        );
        let mismatches = actual.iter().zip(pixels).filter(|(a, b)| a != b).count();
        assert_eq!(mismatches, 0, "GPU frame differs at tick {tick}");
    }
    assert_eq!(restored.play.simulation_ticks, 720);
    assert_eq!(restored.authoring_document().unwrap(), final_scene);
    restored.stop_workers().unwrap();
    assert!(pollster::block_on(scope.pop()).is_none());
    println!(
        "GPU CYCLE PASS ticks=720 packaged_ticks=720 exact_rgba_frames=720 cpu_oracle_frames=1440"
    );
}

#[test]
#[ignore = "requires real GPU; source-free preview frame/phase restore and GPU Arc sharing"]
fn offscreen_preview_snapshot_restores_exact_phase_frame_and_gpu_identity() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("PREVIEW SNAPSHOT GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    for (independent, bridge) in [(false, false), (true, false), (false, true)] {
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
        app.graphics = Some(
            Graphics::new(
                adapter.clone(),
                device.clone(),
                queue.clone(),
                SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm),
                None,
                None,
                (256 * 1024 * 1024, 256 * 1024 * 1024),
            )
            .unwrap(),
        );
        let id = app.id.clone().unwrap();
        await_state(&mut app, |app| {
            app.graphics.as_ref().unwrap().models.contains_key(&id)
        });
        app.panel_action(panels::Action::Animation).unwrap();
        app.begin_model_checkpoint().unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while !app.poll_model_checkpoint().unwrap() {
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        if bridge {
            app.scene
                .insert_component(
                    app.instances[0],
                    ModelAnimation {
                        clip: None,
                        ..Default::default()
                    },
                )
                .unwrap();
            let model = app
                .catalog
                .snapshot(&id)
                .unwrap()
                .value()
                .animated
                .as_ref()
                .unwrap()
                .clone();
            app.play.animations = crate::animation_runtime::AnimationRuntime::default()
                .prepare_wall(&app.scene, &[(id.clone(), model)].into(), 1. / 60.)
                .unwrap();
        }
        let phase = f64::from_bits(0.25_f64.to_bits() + 1);
        app.preview_animation_phase(phase, false).unwrap();
        if bridge {
            let model = app
                .catalog
                .snapshot(&id)
                .unwrap()
                .value()
                .animated
                .as_ref()
                .unwrap()
                .clone();
            let frame = app.play.animations.frame(app.instances[0], &model).unwrap();
            app.authoring
                .animation_preview
                .as_mut()
                .unwrap()
                .restore_shared_frame(frame, 4 * 1024 * 1024)
                .unwrap();
        }
        let _ = scene_pixels(&mut app);
        let owner = app.instances[0];
        let preview_frame = app
            .authoring
            .animation_preview
            .as_ref()
            .unwrap()
            .accepted_frame()
            .clone();
        assert!(Arc::ptr_eq(
            app.graphics
                .as_ref()
                .unwrap()
                .animated_models
                .accepted_frame(owner)
                .unwrap()
                .1,
            &preview_frame
        ));
        if independent {
            let settings = app
                .scene
                .component::<ModelAnimation>(owner)
                .unwrap()
                .unwrap()
                .clone();
            let graphics = app.graphics.as_mut().unwrap();
            let other_live = graphics
                .geometry_bytes()
                .saturating_sub(graphics.animated_models.allocation_bytes());
            let resource = &graphics.models[&id];
            graphics
                .animated_models
                .restore_accepted_owner(
                    &graphics.renderer,
                    &device,
                    &queue,
                    animated_models::Request {
                        publication: resource.publication.clone(),
                        owner,
                        model: resource.animated_model.as_ref().unwrap().clone(),
                        settings,
                        frame: Some(Arc::new((*preview_frame).clone())),
                        lod: resource.animated_lod.clone(),
                        textures: resource.animated_textures.clone(),
                        texture_storage: resource._images.clone(),
                    },
                    0,
                    other_live,
                    graphics.residency_cache.geometry_budget,
                )
                .unwrap();
        }
        let roles: std::collections::HashMap<_, _> = app
            .instances
            .iter()
            .copied()
            .zip(app.object_ids.iter().cloned())
            .collect();
        let policy = crate::play_session::checkpoint::tests::resource_restore_limits();
        let max = 4 * 1024 * 1024;
        let original_preview = app
            .capture_preview_checkpoint(|node| roles.get(&node).cloned(), max)
            .unwrap()
            .unwrap()
            .to_json(max)
            .unwrap();
        let saved = app
            .capture_model_animation_checkpoint(
                |node| roles.get(&node).cloned(),
                policy.catalog,
                max,
            )
            .unwrap();
        let metadata = serde_json::to_value(&saved).unwrap();
        assert_eq!(
            metadata["resources"]
                .get("preview_shared_gpu_owner")
                .is_some(),
            !independent
        );
        assert_eq!(
            metadata["resources"]
                .get("preview_shared_runtime_owner")
                .is_some(),
            bridge
        );
        let serial = app.play.animations.serial();
        let bytes = saved.to_bytes(policy.max_bytes).unwrap();
        let document = app.authoring_document().unwrap();
        app.stop_workers().unwrap();
        std::fs::remove_dir_all(source).unwrap();
        let target_root = temp.0.join("empty-target");
        std::fs::create_dir(&target_root).unwrap();
        let graphics = app.graphics.as_ref().unwrap();
        let reference = crate::animated_models::tests::pixels(
            &graphics.renderer,
            &device,
            &queue,
            graphics
                .animated_models
                .geometries(owner)
                .unwrap()
                .collect(),
            glam::Mat4::IDENTITY,
        );
        assert!(
            reference
                .chunks_exact(4)
                .any(|pixel| pixel[..3].iter().any(|v| *v != 0))
        );
        let registry = editor_component_registry().unwrap();
        let scene = document.load(&registry, scene_limits::OBJECTS).unwrap();
        let saved = crate::app::resource_checkpoint::ModelAnimationCheckpoint::from_bytes(
            &bytes,
            policy.max_bytes,
        )
        .unwrap();
        let prepared = saved
            .prepare(
                &scene.graph,
                |id| scene.resolve(id),
                &target_root,
                policy,
                max,
            )
            .unwrap();
        let cache = crate::gpu_model::ResidencyCache::default();
        let pending = prepared
            .stage_gpu(
                &scene.graph,
                crate::gpu_model::batch::UploadTarget {
                    renderer: &graphics.renderer,
                    device: &device,
                    queue: &queue,
                    cache: &cache,
                    live_geometry: graphics.geometry_bytes(),
                    replaced_geometry: 0,
                },
            )
            .unwrap();
        let mut ready = pollster::block_on(pending.validate()).unwrap();
        let preview = ready.resources.preview.as_ref().unwrap();
        let new_owner = scene.resolve(&roles[&owner]).unwrap();
        assert_eq!(preview.owner, new_owner);
        assert_eq!(preview.phase.to_bits(), phase.to_bits());
        let gpu_frame = ready.gpu.animated.accepted_frame(new_owner).unwrap().1;
        assert_eq!(
            Arc::ptr_eq(preview.accepted_frame(), gpu_frame),
            !independent
        );
        let restored = preview
            .checkpoint(
                &scene.checkpoint_view(),
                |model| {
                    ready
                        .resources
                        .resources
                        .models
                        .iter()
                        .position(|root| Arc::ptr_eq(root, model))
                        .map(|index| format!("model:{index}"))
                },
                max,
            )
            .unwrap()
            .to_json(max)
            .unwrap();
        assert_eq!(restored, original_preview);
        let actual = crate::animated_models::tests::pixels(
            &graphics.renderer,
            &device,
            &queue,
            ready.gpu.animated.geometries(new_owner).unwrap().collect(),
            glam::Mat4::IDENTITY,
        );
        assert_eq!(actual, reference);
        assert_eq!(ready.animations.serial(), serial);
        if bridge {
            let model = &ready.resources.preview_models[0];
            let frame = ready.animations.frame(new_owner, model).unwrap();
            assert!(Arc::ptr_eq(
                &frame,
                ready.resources.preview.as_ref().unwrap().accepted_frame()
            ));
            assert!(Arc::ptr_eq(
                &frame,
                ready.gpu.animated.accepted_frame(new_owner).unwrap().1
            ));
        }
        if let Some(worker) = ready.resources.imports.take() {
            worker.close().join().unwrap();
        }
        if let Some(worker) = ready.resources.watcher.take() {
            worker.close().join().unwrap();
        }
        println!(
            "PREVIEW SNAPSHOT PASS independent={independent} bridge={bridge} exact_phase_frame_pixels=true"
        );
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "requires real GPU; original editor file/history and coordinated GPU staging"]
fn offscreen_editor_file_keeps_history_preview_and_gpu_in_one_scene() {
    qualify_editor_file_install(false, false, EditorFileAudio::Offline);
}

#[test]
#[ignore = "requires real GPU; running App restoration with nonempty UI and offline audio"]
fn offscreen_running_editor_file_installs_ui_audio_and_continues_game() {
    qualify_editor_file_install(true, false, EditorFileAudio::Offline);
}

#[test]
#[ignore = "requires real GPU and Unicode font; running App restoration with accepted text atlases"]
fn offscreen_running_editor_file_installs_text_font_and_continues_game() {
    qualify_editor_file_install(true, true, EditorFileAudio::Offline);
}

#[test]
#[ignore = "requires real GPU; native output policy without an opened device, no window"]
fn offscreen_native_idle_editor_file_keeps_policy_and_restores_models_history() {
    qualify_editor_file_install(false, false, EditorFileAudio::NativeIdle);
}

#[test]
#[ignore = "real GPU and default native audio output; offline-to-native conversion, zero gain, no window"]
fn offscreen_offline_playback_archive_restores_native_output_without_sources() {
    qualify_editor_file_install(true, false, EditorFileAudio::ConvertNative);
}

enum EditorFileAudio {
    Offline,
    NativeIdle,
    ConvertNative,
}

fn qualify_editor_file_install(running: bool, with_text: bool, audio_mode: EditorFileAudio) {
    let (native_idle, convert_native) = match audio_mode {
        EditorFileAudio::Offline => (false, false),
        EditorFileAudio::NativeIdle => (true, false),
        EditorFileAudio::ConvertNative => (false, true),
    };
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("EDITOR HISTORY GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let temp = crate::project_prefab_tests::Temp::new();
    let root = temp.0.join("source");
    let project = project::create(&root, "Editor GPU history").unwrap();
    std::fs::write(
        root.join("rig.glb"),
        include_bytes!("../../../../voxy_render/examples/assets/animated-triangle.glb"),
    )
    .unwrap();
    let mut manifest =
        serde_json::json!({"version":1,"assets":[{"asset":"rig","source":"rig.glb"}]});
    if running {
        std::fs::write(
            root.join("tone.wav"),
            crate::audio_session::checkpoint::tests::wav(),
        )
        .unwrap();
        manifest["assets"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"asset":"tone.wav","source":"tone.wav"}));
    }
    if with_text {
        let font = [
            "/System/Library/Fonts/Supplemental/Arial.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "C:/Windows/Fonts/arial.ttf",
        ]
        .iter()
        .find_map(|path| std::fs::read(path).ok())
        .expect("Unicode TrueType font fixture");
        println!(
            "EDITOR UI FONT bytes={} blake3={}",
            font.len(),
            blake3::hash(&font)
        );
        std::fs::write(root.join("font.ttf"), font).unwrap();
        manifest["assets"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"asset":"font.ttf","source":"font.ttf"}));
    }
    std::fs::write(
        root.join("assets.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let mut app = App::from_project(&project).unwrap();
    if native_idle {
        app.audio
            .set_output_mode(crate::audio_session::AudioOutputMode::Native);
    }
    app.graphics = Some(
        Graphics::new(
            adapter,
            device.clone(),
            queue.clone(),
            SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm),
            None,
            None,
            (256 * 1024 * 1024, 256 * 1024 * 1024),
        )
        .unwrap(),
    );
    app.edit_key(KeyCode::KeyD).unwrap();
    let id = AssetId("rig".into());
    await_state(&mut app, |app| {
        app.graphics.as_ref().unwrap().models.contains_key(&id)
    });
    app.panel_action(panels::Action::Animation).unwrap();
    if running {
        let owner = app.instances[0];
        app.scene
            .insert_component(
                owner,
                voxy_gameplay::AudioSource {
                    asset: "tone.wav".into(),
                    import_settings: None,
                    bus: 0,
                    gain: if convert_native { 0. } else { 0.8 },
                    looping: true,
                    spatial: false,
                    near: 1.,
                    far: 5.,
                },
            )
            .unwrap();
        app.scene
            .insert_component(
                owner,
                voxy_gameplay::UiElement {
                    origin: [0.1, 0.2],
                    size: if with_text { [0.8, 0.5] } else { [0.3, 0.15] },
                    color: [0.2, 0.4, 0.6, 1.],
                    layer: 0,
                    enabled: true,
                    action: None,
                    text: with_text.then(|| voxy_gameplay::UiText {
                        font: "font.ttf".into(),
                        content: "Voxy Привет é".into(),
                        size: 12.,
                        color: [1.; 4],
                    }),
                },
            )
            .unwrap();
        app.commit_authoring().unwrap();
    }
    app.edit_key(KeyCode::KeyD).unwrap();
    assert!(app.authoring.step_history(true).unwrap());
    app.restore_authoring().unwrap();
    let phase = f64::from_bits(0.25_f64.to_bits() + 1);
    if running {
        app.toggle_play().unwrap();
        await_state(&mut app, |app| {
            app.play.playing.is_some() && app.audio.play().is_some()
        });
        app.advance_game(7. / 60.).unwrap();
    } else {
        app.preview_animation_phase(phase, false).unwrap();
    }
    let _ = scene_pixels(&mut app);
    if running {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !app.ui_live.ready() {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(!app.graphics.as_ref().unwrap().ui_draws.is_empty());
    }
    let bindings: Vec<_> = app
        .object_ids
        .iter()
        .cloned()
        .zip(app.instances.iter().copied())
        .collect();
    app.begin_restore_barrier(8 * 1024 * 1024).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !app
        .poll_restore_barrier(|node| {
            bindings
                .iter()
                .find(|(_, n)| *n == node)
                .map(|(id, _)| id.clone())
        })
        .unwrap()
    {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let mut registry = editor_component_registry().unwrap();
    register_game_checkpoint_components(&mut registry).unwrap();
    let registry = Arc::new(registry);
    let resources = crate::play_session::checkpoint::tests::resource_restore_limits();
    let mut play = crate::play_session::checkpoint::tests::limits();
    play.core.scene_capacity = scene_limits::OBJECTS;
    play.core.max_bodies = 128;
    play.core.max_colliders = 128;
    play.max_angular_motions = 128;
    play.ui.max_events = 128;
    play.ui.max_handlers = 128;
    play.ui.max_commands_per_event = 128;

    play.core.simulation = voxy_scene::SimulationLimits {
        fixed_step: 1. / 60.,
        max_steps: 8,
        max_behaviors: 128,
        max_commands: 128,
    };
    let mut ui = crate::ui_live::checkpoint::tests::limits();
    ui.max_bytes = 96 * 1024 * 1024;
    ui.max_reload_bytes = 1024 * 1024;
    ui.publications.max_bytes = 64 * 1024 * 1024;
    ui.publications.max_input_bytes = 32 * 1024 * 1024;
    ui.publications.text.imported.max_inputs = 9;
    ui.publications.text.imported.max_input_bytes = 16 * 1024 * 1024;
    ui.publications.text.imported.max_archive_bytes = 32 * 1024 * 1024;
    ui.publications.text.max_payload_bytes = 64 * 1024 * 1024;
    ui.publications.max_pixels = 32 * 1024 * 1024;
    ui.publications.max_glyphs = 16384;
    ui.publications.max_runs = 256;
    ui.publications.text.max_fonts = 8;
    ui.publications.text.max_runs = 128;
    ui.publications.text.max_pixels = 16 * 1024 * 1024;
    ui.publications.text.max_glyphs_per_run = 1024;
    ui.publications.text.max_total_glyphs = 8192;
    ui.publications.text.font = voxy_text::FontLimits {
        max_font_bytes: 4 * 1024 * 1024,
        max_glyph_pixels: 16384,
        max_size: 128.,
    };

    let history = voxy_scene::HistoryCheckpointLimits {
        max_versions: 64,
        max_history_bytes: scene_limits::HISTORY_BYTES,
        max_scene_capacity: scene_limits::OBJECTS,
        max_archive_bytes: 128 * 1024 * 1024,
        max_edits: 4,
        max_draft_objects: scene_limits::OBJECTS * 2,
    };
    let view = voxy_scene::SceneCheckpointView::new(&app.scene, bindings.clone()).unwrap();
    let saved = app
        .editor_checkpoint(
            &view,
            &registry,
            "editor-gpu-v1",
            resources,
            play,
            ui,
            history,
        )
        .unwrap();
    let file_limits = crate::app::restore_barrier::editor::EditorFileLimits {
        max_bytes: 512 * 1024 * 1024,
        game: crate::app::restore_barrier::file::CoordinatedFileLimits {
            max_bytes: 384 * 1024 * 1024,
            max_game_bytes: resources.max_bytes,
            ui,
            max_audio_bytes: 8 * 1024 * 1024,
            max_input_bytes: 100000,
        },
        history,
    };
    let target_root = temp.0.join("target");
    std::fs::create_dir(&target_root).unwrap();
    let file = target_root.join("editor.voxyedr");
    saved.save_file(&file, file_limits).unwrap();
    let saved =
        crate::app::restore_barrier::editor::EditorCheckpoint::read_file(&file, file_limits)
            .unwrap();
    let original_preview = app
        .capture_preview_checkpoint(|node| view.identity(node).cloned(), play.core.max_bytes)
        .unwrap()
        .map(|preview| preview.to_json(play.core.max_bytes).unwrap());
    let original_ui = app
        .ui_live
        .publication_checkpoint(&view, ui.publications)
        .unwrap()
        .to_bytes(ui.publications)
        .unwrap();
    let original_ui_pixels = with_text.then(|| {
        let graphics = app.graphics.as_ref().unwrap();
        assert!(graphics.ui_draws.iter().any(|draw| draw.texture.is_some()));
        let pixels = crate::ui_live::checkpoint_pixels(
            &graphics.renderer,
            &device,
            &queue,
            &graphics.ui_draws,
        );
        assert!(
            pixels
                .chunks_exact(4)
                .any(|pixel| pixel[..3].iter().all(|channel| *channel > 200))
        );
        pixels
    });
    let original_audio: serde_json::Value =
        serde_json::from_str(&saved.game.audio.to_json(8 * 1024 * 1024).unwrap()).unwrap();
    let audio_frames = app.audio.play().map(|audio| audio.frames);
    let original_ticks = app.play.simulation_ticks;
    if running {
        assert!(audio_frames.unwrap() > 0);
        if !convert_native {
            assert!(app.audio.play().unwrap().peak > 0.);
        }
        assert!(!app.graphics.as_ref().unwrap().ui_draws.is_empty());
    }
    drop(view);
    std::fs::remove_dir_all(&root).unwrap();
    let graphics = app.graphics.as_ref().unwrap();
    let live_geometry = graphics.geometry_bytes();
    let original_pixels = crate::animated_models::tests::pixels(
        &graphics.renderer,
        &device,
        &queue,
        graphics
            .animated_models
            .geometries(app.instances[0])
            .unwrap()
            .collect(),
        glam::Mat4::IDENTITY,
    );
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let cache = crate::gpu_model::ResidencyCache::default();
    let initial_cache = cache.state();
    let target = || crate::gpu_model::batch::UploadTarget {
        renderer: &renderer,
        device: &device,
        queue: &queue,
        cache: &cache,
        live_geometry,
        replaced_geometry: 0,
    };
    let behaviors = app.behavior_checkpoints.clone();
    let ui_actions = app.ui_action_checkpoints.clone();
    let options = || crate::app::restore_barrier::CoordinatedPrepare {
        compatibility: "editor-gpu-v1",
        registry: &registry,
        behaviors: &behaviors,
        ui_actions: &ui_actions,
        root: &target_root,
        resources,
        play,
        ui,
        max_audio_bytes: 8 * 1024 * 1024,
        restored_ui_cache: (0, 0, 0, 0),
    };
    for wrong_scene in [true, false] {
        let origin = app
            .observe_coordinated_origin(&bindings, &registry, "editor-gpu-v1", resources, play, ui)
            .unwrap();
        let mut candidate = saved.prepare_guarded(options(), history, origin).unwrap();
        if wrong_scene {
            candidate.authoring.scene_id = app.scene.identity();
            assert!(candidate.stage_gpu(target()).is_err());
        } else {
            for worker in candidate.stage_gpu(target()).unwrap().discard() {
                worker.join().unwrap();
            }
        }
        assert_eq!(cache.state(), initial_cache);
    }
    let origin = app
        .observe_coordinated_origin(&bindings, &registry, "editor-gpu-v1", resources, play, ui)
        .unwrap();
    let candidate = saved.prepare_guarded(options(), history, origin).unwrap();
    let pending = candidate.stage_gpu(target()).unwrap();
    let mut ready = pollster::block_on(pending.validate()).unwrap();
    ready
        .validate_origin(&mut app, &bindings, &registry, "editor-gpu-v1", &target())
        .unwrap();
    let target_scene = ready.authoring.scene_id;
    ready.authoring.scene_id = app.scene.identity();
    assert!(
        ready
            .validate_origin(&mut app, &bindings, &registry, "editor-gpu-v1", &target())
            .is_err()
    );
    ready.authoring.scene_id = target_scene;
    ready
        .validate_origin(&mut app, &bindings, &registry, "editor-gpu-v1", &target())
        .unwrap();
    let scene = &ready.game.game.game.scene;
    assert_eq!(ready.authoring.scene_id, scene.graph.identity());
    assert_eq!(ready.authoring.object_ids, app.object_ids);
    assert_eq!(ready.authoring.selected, app.selected);
    for (id, node) in ready
        .authoring
        .object_ids
        .iter()
        .zip(&ready.authoring.instances)
    {
        assert_eq!(scene.resolve(id), Some(*node));
    }

    assert_ne!(scene.graph.identity(), app.scene.identity());
    assert_eq!(ready.game.game.game.play.playing.is_some(), running);
    let owner = scene.resolve(&app.object_ids[app.selected]).unwrap();
    let restored_preview = ready.game.game.resources.preview.as_ref().map(|preview| {
        assert_eq!(preview.phase.to_bits(), phase.to_bits());
        assert_eq!(preview.owner, owner);
        preview
            .checkpoint(
                &scene.checkpoint_view(),
                |model| {
                    ready
                        .game
                        .game
                        .resources
                        .resources
                        .models
                        .iter()
                        .position(|root| Arc::ptr_eq(root, model))
                        .map(|i| format!("model:{i}"))
                },
                play.core.max_bytes,
            )
            .unwrap()
            .to_json(play.core.max_bytes)
            .unwrap()
    });
    assert_eq!(restored_preview, original_preview);
    let actual = crate::animated_models::tests::pixels(
        &renderer,
        &device,
        &queue,
        ready
            .game
            .game
            .gpu
            .animated
            .geometries(owner)
            .unwrap()
            .collect(),
        glam::Mat4::IDENTITY,
    );
    assert_eq!(original_pixels, actual);
    assert!(
        actual
            .chunks_exact(4)
            .any(|p| p[..3].iter().any(|v| *v != 0))
    );
    for redo in [true, false, false, true, true] {
        let original = app.authoring.history.as_mut().unwrap();
        let restored = ready.authoring.history.as_mut().unwrap();
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
            serde_json::to_value(original.checkpoint(&[], history).unwrap()).unwrap(),
            serde_json::to_value(restored.checkpoint(&[], history).unwrap()).unwrap()
        );
    }
    assert!(
        ready
            .validate_origin(&mut app, &bindings, &registry, "editor-gpu-v1", &target())
            .is_err()
    );
    for worker in ready.discard() {
        worker.join().unwrap();
    }
    assert_eq!(cache.state(), initial_cache);
    let source_registry = app.authoring.authoring_project.registry.clone();
    let source_scene = app.scene.change_token();
    let source_history = app.authoring.history.as_ref().unwrap().change_token();
    let source_cache = app.graphics.as_ref().unwrap().residency_cache.state();
    for outcome in 0..4 {
        let origin = app
            .observe_coordinated_origin(&bindings, &registry, "editor-gpu-v1", resources, play, ui)
            .unwrap();
        let prepared = saved.prepare_guarded(options(), history, origin).unwrap();
        let graphics = app.graphics.as_ref().unwrap();
        let pending = prepared
            .stage_gpu(crate::gpu_model::batch::UploadTarget {
                renderer: &graphics.renderer,
                device: &graphics.device,
                queue: &graphics.queue,
                cache: &graphics.residency_cache,
                live_geometry: graphics.geometry_bytes(),
                replaced_geometry: graphics.checkpoint_replaced_geometry(),
            })
            .unwrap();
        let mut candidate = pollster::block_on(pending.validate()).unwrap();
        if outcome != 3 {
            if outcome == 0 {
                candidate.authoring.scene_id = app.scene.identity();
                assert!(
                    pollster::block_on(Box::pin(app.install_editor_checkpoint(
                        candidate,
                        &bindings,
                        &registry,
                        "editor-gpu-v1",
                    )))
                    .is_err()
                );
            } else {
                let pending = app
                    .stage_editor_checkpoint_install(
                        candidate,
                        &bindings,
                        &registry,
                        "editor-gpu-v1",
                    )
                    .unwrap();
                if outcome == 1 {
                    // App is available while the owned GPU candidate is pending.
                    let before = app.play.player_input.checkpoint();
                    app.play
                        .player_input
                        .event(voxy_gameplay::RIGHT, 0.625)
                        .unwrap();
                    let ready = pollster::block_on(Box::pin(pending.validate())).unwrap();
                    let error = app
                        .publish_editor_checkpoint_install(
                            ready,
                            &bindings,
                            &registry,
                            "editor-gpu-v1",
                        )
                        .err()
                        .expect("changed source input must reject publication");
                    assert!(error.to_string().contains("Play runtime state changed"));
                    app.play.player_input.restore_checkpoint(&before).unwrap();
                } else {
                    for worker in pending.discard() {
                        worker.join().unwrap();
                    }
                }
            }
            assert_eq!(app.scene.change_token(), source_scene);
            assert_eq!(
                app.authoring.history.as_ref().unwrap().change_token(),
                source_history
            );
            assert_eq!(
                app.graphics.as_ref().unwrap().residency_cache.state(),
                source_cache
            );
            assert!(app.restore_play_frozen());
        } else {
            let target_scene = candidate.authoring.scene_id;
            let expected_history = serde_json::to_value(
                candidate
                    .authoring
                    .history
                    .as_ref()
                    .unwrap()
                    .checkpoint(&[], history)
                    .unwrap(),
            )
            .unwrap();
            let publication_start = Instant::now();
            let retired = pollster::block_on(Box::pin(app.install_editor_checkpoint(
                candidate,
                &bindings,
                &registry,
                "editor-gpu-v1",
            )))
            .unwrap();
            assert!(retired.finish().is_empty());
            assert!(app.play.simulation_time >= publication_start);
            assert_eq!(app.scene.identity(), target_scene);
            assert_eq!(app.play.playing.is_some(), running);
            assert_eq!(app.play.simulation.is_some(), running);
            assert!(!app.restore_play_frozen());
            assert!(app.resource_checkpoint.is_none());
            assert!(Arc::ptr_eq(
                &source_registry,
                &app.authoring.authoring_project.registry
            ));
            assert_eq!(
                serde_json::to_value(
                    app.authoring
                        .history
                        .as_ref()
                        .unwrap()
                        .checkpoint(&[], history)
                        .unwrap()
                )
                .unwrap(),
                expected_history
            );
            assert!(
                app.authoring_document()
                    .unwrap()
                    .objects
                    .iter()
                    .all(|object| !object.components.contains_key("runtime.model-instance.v1"))
            );
            let graphics = app.graphics.as_ref().unwrap();
            let actual = crate::animated_models::tests::pixels(
                &graphics.renderer,
                &device,
                &queue,
                graphics
                    .animated_models
                    .geometries(app.instances[0])
                    .unwrap()
                    .collect(),
                glam::Mat4::IDENTITY,
            );
            assert_eq!(actual, original_pixels);
            let fresh_bindings: Vec<_> = app
                .object_ids
                .iter()
                .cloned()
                .zip(app.instances.iter().copied())
                .collect();
            let view =
                voxy_scene::SceneCheckpointView::new(&app.scene, fresh_bindings.clone()).unwrap();
            assert_eq!(
                app.ui_live
                    .publication_checkpoint(&view, ui.publications)
                    .unwrap()
                    .to_bytes(ui.publications)
                    .unwrap(),
                original_ui
            );
            if let Some(expected) = &original_ui_pixels {
                let graphics = app.graphics.as_ref().unwrap();
                assert!(graphics.ui_draws.iter().any(|draw| draw.texture.is_some()));
                let actual = crate::ui_live::checkpoint_pixels(
                    &graphics.renderer,
                    &device,
                    &queue,
                    &graphics.ui_draws,
                );
                assert_eq!(&actual, expected);
            }
            if running {
                assert!(app.authoring.animation_preview.is_none());
                assert_eq!(
                    capture_offline_audio_playback(&mut app.audio, &fresh_bindings),
                    original_audio["play"]["playback"]
                );
                let mut reference_audio = crate::audio_session::AudioSession::default();
                let prepared_audio = saved
                    .game
                    .audio
                    .prepare(
                        &app.scene,
                        &app.authoring.authoring_project,
                        |id| {
                            fresh_bindings
                                .iter()
                                .find(|(candidate, _)| candidate == id)
                                .map(|(_, node)| *node)
                        },
                        8 * 1024 * 1024,
                    )
                    .unwrap();
                reference_audio
                    .checkpoint_publication()
                    .unwrap()
                    .publish(prepared_audio);
                assert_eq!(app.audio.play().unwrap().frames, audio_frames.unwrap());
                assert!(!app.graphics.as_ref().unwrap().ui_draws.is_empty());
                let ticks = app.play.simulation_ticks;
                let frames = app.audio.play().unwrap().frames;
                for _ in 0..120 {
                    app.advance_game(1. / 60.).unwrap();
                    reference_audio
                        .play_mut()
                        .unwrap()
                        .advance(&app.scene, 1)
                        .unwrap();
                    assert_eq!(
                        app.audio.play().unwrap().frames,
                        reference_audio.play().unwrap().frames
                    );
                    assert_eq!(
                        app.audio.play().unwrap().peak.to_bits(),
                        reference_audio.play().unwrap().peak.to_bits()
                    );
                    let _ = scene_pixels(&mut app);
                    reference_audio
                        .play_mut()
                        .unwrap()
                        .poll_reload(&app.scene)
                        .unwrap();
                }
                assert_eq!(
                    capture_offline_audio_playback(&mut app.audio, &fresh_bindings),
                    capture_offline_audio_playback(&mut reference_audio, &fresh_bindings)
                );
                if let Some(expected) = &original_ui_pixels {
                    let graphics = app.graphics.as_ref().unwrap();
                    let actual = crate::ui_live::checkpoint_pixels(
                        &graphics.renderer,
                        &device,
                        &queue,
                        &graphics.ui_draws,
                    );
                    assert_eq!(&actual, expected);
                }
                reference_audio.close_play_audio();
                assert!(reference_audio.join_workers().is_empty());
                assert_eq!(app.play.simulation_ticks, ticks + 120);
                assert_eq!(app.audio.play().unwrap().frames, frames + 96000);
                app.toggle_play().unwrap();
                assert!(app.play.playing.is_none() && app.play.simulation.is_none());
                assert!(app.audio.play().is_none());
            } else {
                assert_eq!(
                    app.authoring
                        .animation_preview
                        .as_ref()
                        .unwrap()
                        .phase
                        .to_bits(),
                    phase.to_bits()
                );
            }
            let _ = scene_pixels(&mut app);
        }
    }
    // Cancel real GPU validation at both pre-publication stages. The source
    // authoring state must survive, including after detached workers settle.
    for cancel_ui in [false, true] {
        let source = app.scene.identity();
        let token = app.scene.change_token();
        app.request_editor_file_load(
            file.clone(),
            target_root.clone(),
            crate::app::restore_barrier::save::EditorSavePolicy {
                compatibility: "editor-gpu-v1".into(),
                resources,
                play,
                file: file_limits,
            },
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let reached = if cancel_ui {
                matches!(
                    app.editor_load,
                    Some(crate::app::restore_barrier::load::PendingEditorLoad::Ui { .. })
                )
            } else {
                matches!(
                    app.editor_load,
                    Some(crate::app::restore_barrier::load::PendingEditorLoad::Models { .. })
                )
            };
            if reached {
                break;
            }
            let _ = scene_pixels(&mut app);
            assert!(
                app.editor_load.is_some(),
                "load completed before cancellation stage"
            );
            assert!(
                Instant::now() < deadline,
                "cancellation stage timed out: {:?}",
                app.editor_load
            );
            std::thread::yield_now();
        }
        assert!(app.cancel_editor_file_load());
        assert!(!app.cancel_editor_file_load());
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(
                Instant::now() < deadline,
                "cancellation did not settle: {:?}",
                app.editor_load
            );
            std::thread::yield_now();
        }
        assert_eq!(app.scene.identity(), source);
        assert_eq!(app.scene.change_token(), token);
        assert!(
            (app.error.is_none() && app.snapshot_error.is_none()),
            "cancelled load failed: {:?}",
            app.error
        );
        assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
    }
    // Exercise the frame-polled command itself, after the direct install checks.
    let source_scene = app.scene.identity();
    let request_start = Instant::now();
    app.request_editor_file_load(
        file.clone(),
        target_root.clone(),
        crate::app::restore_barrier::save::EditorSavePolicy {
            compatibility: "editor-gpu-v1".into(),
            resources,
            play,
            file: file_limits,
        },
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.editor_load.is_some() {
        let _ = scene_pixels(&mut app);
        assert!(
            Instant::now() < deadline,
            "frame-polled load did not settle: {:?}",
            app.editor_load
        );
        std::thread::yield_now();
    }
    assert_ne!(app.scene.identity(), source_scene);
    assert!(
        (app.error.is_none() && app.snapshot_error.is_none()),
        "frame load failed: {:?}",
        app.error
    );
    assert_eq!(app.play.playing.is_some(), running);
    assert_eq!(app.play.simulation_ticks, original_ticks);
    assert_eq!(app.audio.play().map(|audio| audio.frames), audio_frames);
    assert!(app.play.simulation_time >= request_start);
    assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
    let graphics = app.graphics.as_ref().unwrap();
    let actual = crate::animated_models::tests::pixels(
        &graphics.renderer,
        &device,
        &queue,
        graphics
            .animated_models
            .geometries(app.instances[0])
            .unwrap()
            .collect(),
        glam::Mat4::IDENTITY,
    );
    assert_eq!(actual, original_pixels);
    if let Some(expected) = &original_ui_pixels {
        assert_eq!(
            &crate::ui_live::checkpoint_pixels(
                &graphics.renderer,
                &device,
                &queue,
                &graphics.ui_draws
            ),
            expected
        );
    }
    // Exercise the actual desktop command profile with accepted font/audio/model
    // resources, rather than the deliberately restricted fixture policy above.
    if with_text {
        use crate::app::snapshot_commands::SnapshotCommand;
        let desktop_file = target_root.join("desktop.voxyedr");
        let source = app.scene.identity();
        let token = app.scene.change_token();
        app.begin_snapshot_path(SnapshotCommand::Save).unwrap();
        app.snapshot_path_key(KeyCode::KeyA, Some(desktop_file.to_str().unwrap()))
            .unwrap();
        app.snapshot_path_key(KeyCode::Enter, None).unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        while app.editor_save.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline, "desktop save did not settle");
            assert!(
                (app.error.is_none() && app.snapshot_error.is_none()),
                "desktop save failed: {:?}",
                app.error
            );
            std::thread::yield_now();
        }
        assert!(desktop_file.is_file());
        assert_eq!(app.scene.change_token(), token);
        app.begin_snapshot_path(SnapshotCommand::Open).unwrap();
        app.snapshot_path_key(KeyCode::KeyA, Some(desktop_file.to_str().unwrap()))
            .unwrap();
        app.snapshot_path_key(KeyCode::Enter, None).unwrap();
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(
                Instant::now() < deadline,
                "desktop open did not settle: {:?}",
                app.editor_load
            );
            assert!(
                (app.error.is_none() && app.snapshot_error.is_none()),
                "desktop open failed: {:?}",
                app.error
            );
            std::thread::yield_now();
        }
        assert_ne!(app.scene.identity(), source);
        assert_eq!(app.play.simulation_ticks, original_ticks);
        assert_eq!(app.audio.play().map(|audio| audio.frames), audio_frames);
        let graphics = app.graphics.as_ref().unwrap();
        assert_eq!(
            crate::ui_live::checkpoint_pixels(
                &graphics.renderer,
                &device,
                &queue,
                &graphics.ui_draws
            ),
            original_ui_pixels.clone().unwrap()
        );
        assert_eq!(
            crate::animated_models::tests::pixels(
                &graphics.renderer,
                &device,
                &queue,
                graphics
                    .animated_models
                    .geometries(app.instances[0])
                    .unwrap()
                    .collect(),
                glam::Mat4::IDENTITY
            ),
            original_pixels
        );
        let corrupt_file = target_root.join("corrupt.voxyedr");
        std::fs::write(&corrupt_file, b"corrupt snapshot").unwrap();
        for (command, path) in [
            (SnapshotCommand::Open, corrupt_file),
            (
                SnapshotCommand::Save,
                target_root.join("missing/snapshot.voxyedr"),
            ),
        ] {
            let source = app.scene.identity();
            let token = app.scene.change_token();
            app.begin_snapshot_path(command).unwrap();
            app.snapshot_path_key(KeyCode::KeyA, Some(path.to_str().unwrap()))
                .unwrap();
            app.snapshot_path_key(KeyCode::Enter, None).unwrap();
            assert!(app.snapshot_error.is_none());
            let deadline = Instant::now() + Duration::from_secs(30);
            while app.editor_load.is_some() || app.editor_save.is_some() {
                let _ = scene_pixels(&mut app);
                assert!(
                    app.error.is_none(),
                    "file failure became fatal: {:?}",
                    app.error
                );
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
            assert!(app.snapshot_error.is_some());
            assert_eq!(app.scene.identity(), source);
            assert_eq!(app.scene.change_token(), token);
            assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
            // Normal source frames keep rendering after a recoverable failure.
            let _ = scene_pixels(&mut app);
            assert!(app.error.is_none());
        }
        app.begin_snapshot_path(SnapshotCommand::Open).unwrap();
        app.snapshot_path_key(KeyCode::KeyA, Some(desktop_file.to_str().unwrap()))
            .unwrap();
        app.snapshot_path_key(KeyCode::Enter, None).unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(app.error.is_none() && app.snapshot_error.is_none());
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(app.snapshot_error.is_none());
        assert_eq!(app.play.simulation_ticks, original_ticks);
        assert_eq!(app.audio.play().map(|audio| audio.frames), audio_frames);
    }
    if convert_native {
        use crate::app::restore_barrier::load::PendingEditorLoad;
        app.audio
            .set_output_mode(crate::audio_session::AudioOutputMode::Native);
        assert!(app.audio.play().unwrap().output_connection().is_none());
        let conversion_policy = crate::app::restore_barrier::save::EditorSavePolicy {
            compatibility: "editor-gpu-v1".into(),
            resources,
            play,
            file: file_limits,
        };
        let deadline = Instant::now() + Duration::from_secs(30);
        for cancel_device in [true, false] {
            let source = app.scene.change_token();
            let bindings = app
                .object_ids
                .iter()
                .cloned()
                .zip(app.instances.iter().copied())
                .collect::<Vec<_>>();
            let before = capture_offline_audio_playback(&mut app.audio, &bindings);
            app.request_editor_file_load(
                file.clone(),
                target_root.clone(),
                conversion_policy.clone(),
            )
            .unwrap();
            loop {
                let _ = scene_pixels(&mut app);
                assert!(
                    Instant::now() < deadline,
                    "conversion timeout: {:?}",
                    app.editor_load
                );
                assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
                let reached = if cancel_device {
                    matches!(app.editor_load, Some(PendingEditorLoad::Device { .. }))
                } else {
                    matches!(app.editor_load, Some(PendingEditorLoad::Audio { .. }))
                };
                if reached {
                    break;
                }
                assert!(app.editor_load.is_some());
            }
            if !cancel_device {
                let _ = scene_pixels(&mut app);
                assert!(matches!(
                    app.editor_load,
                    Some(PendingEditorLoad::Audio { .. })
                ));
            }
            assert!(app.cancel_editor_file_load());
            while app.editor_load.is_some() {
                let _ = scene_pixels(&mut app);
                assert!(Instant::now() < deadline);
            }
            while app.audio.checkpoint_admission().is_err() {
                app.tick().unwrap();
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
            assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
            assert_eq!(app.scene.change_token(), source);
            assert!(app.audio.play().unwrap().output_connection().is_none());
            let after = capture_offline_audio_playback(&mut app.audio, &bindings);
            assert_eq!(before["runtime"], after["runtime"]);
            assert_eq!(before["clock"], after["clock"]);
            assert_eq!(before["pcm"], after["pcm"]);
        }
        app.request_editor_file_load(file.clone(), target_root.clone(), conversion_policy.clone())
            .unwrap();
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline);
            assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
        }
        assert_eq!(app.play.simulation_ticks, original_ticks);
        assert_eq!(app.audio.play().unwrap().frames, audio_frames.unwrap());
        let output = app.audio.play().unwrap().output_connection().unwrap();
        assert_eq!(output.sample_rate(), 48000);
        assert_eq!(output.stats().errors, 0);
        let graphics = app.graphics.as_ref().unwrap();
        assert_eq!(
            crate::animated_models::tests::pixels(
                &graphics.renderer,
                &device,
                &queue,
                graphics
                    .animated_models
                    .geometries(app.instances[0])
                    .unwrap()
                    .collect(),
                glam::Mat4::IDENTITY,
            ),
            original_pixels
        );
        assert!(!graphics.ui_draws.is_empty());
        for _ in 0..120 {
            app.advance_game(1. / 60.).unwrap();
            let _ = scene_pixels(&mut app);
        }
        assert_eq!(app.play.simulation_ticks, original_ticks + 120);
        assert!(app.audio.play().unwrap().frames > audio_frames.unwrap());
        for from_idle in [false, true] {
            let previous = app
                .audio
                .play()
                .unwrap()
                .output_connection()
                .unwrap()
                .clone();
            if from_idle {
                app.toggle_play().unwrap();
                while app.audio.checkpoint_admission().is_err() {
                    app.tick().unwrap();
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
            }
            app.request_editor_file_load(
                file.clone(),
                target_root.clone(),
                conversion_policy.clone(),
            )
            .unwrap();
            while app.editor_load.is_some() {
                let _ = scene_pixels(&mut app);
                assert!(Instant::now() < deadline);
                assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
            }
            let output = app.audio.play().unwrap().output_connection().unwrap();
            assert_eq!(output.shares_output(&previous), !from_idle);
            assert_eq!(output.stats().errors, 0);
            assert_eq!(app.play.simulation_ticks, original_ticks);
            assert_eq!(app.audio.play().unwrap().frames, audio_frames.unwrap());
            for _ in 0..120 {
                app.advance_game(1. / 60.).unwrap();
                let _ = scene_pixels(&mut app);
            }
            assert_eq!(app.play.simulation_ticks, original_ticks + 120);
            assert!(app.audio.play().unwrap().frames > audio_frames.unwrap());
        }
        println!(
            "OFFLINE-TO-NATIVE source-free archive, source cancellation exact, existing/new output reuse, three 120-frame continuations PASS"
        );
    }
    if native_idle {
        assert_eq!(
            app.audio.output_mode(),
            crate::audio_session::AudioOutputMode::Native
        );
        assert!(app.audio.checkpoint_output_connection().unwrap().is_none());
    }
    app.stop_workers().unwrap();
    assert!(pollster::block_on(scope.pop()).is_none());
}

fn capture_offline_audio_playback(
    audio: &mut crate::audio_session::AudioSession,
    bindings: &[(voxy_scene::ObjectId, voxy_scene::NodeId)],
) -> serde_json::Value {
    audio.request_checkpoint().unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let saved = loop {
        if let Some(saved) = audio
            .poll_checkpoint(
                |node| {
                    bindings
                        .iter()
                        .find(|(_, candidate)| *candidate == node)
                        .map(|(id, _)| id.clone())
                },
                8 * 1024 * 1024,
            )
            .unwrap()
        {
            break saved;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    };
    while !audio.resume_checkpoint().unwrap() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let value: serde_json::Value =
        serde_json::from_str(&saved.to_json(8 * 1024 * 1024).unwrap()).unwrap();
    value["play"]["playback"].clone()
}

#[test]
#[ignore = "real GPU and default native audio device; zero gain, no window"]
fn offscreen_native_editor_file_commits_scene_ui_input_and_audio_together() {
    qualify_native_editor_file_transition(false, false);
}

#[test]
#[ignore = "real GPU and default native audio device; zero gain, no window"]
fn offscreen_native_playback_loads_idle_scene_and_retires_output_atomically() {
    qualify_native_editor_file_transition(true, false);
}

#[test]
#[ignore = "real GPU and default native audio output; exact offline queue continuation, zero gain, no window"]
fn offscreen_native_archive_preserves_pcm_when_loading_offline_and_back() {
    qualify_native_editor_file_transition(true, true);
}

fn qualify_native_editor_file_transition(active_to_idle: bool, native_to_offline: bool) {
    use crate::app::restore_barrier::{
        editor::EditorCheckpoint, load::PendingEditorLoad, save::EditorSavePolicy,
    };
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("NATIVE EDITOR GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let temp = crate::project_prefab_tests::Temp::new();
    let root = temp.0.join("source");
    let project = project::create(&root, "Native coordinated editor").unwrap();
    std::fs::write(
        root.join("rig.glb"),
        include_bytes!("../../../../voxy_render/examples/assets/animated-triangle.glb"),
    )
    .unwrap();
    std::fs::write(
        root.join("tone.wav"),
        crate::audio_session::checkpoint::tests::wav(),
    )
    .unwrap();
    std::fs::write(root.join("assets.json"), br#"{"version":1,"assets":[{"asset":"rig","source":"rig.glb"},{"asset":"tone.wav","source":"tone.wav"}]}"#).unwrap();
    let mut app = App::from_project(&project).unwrap();
    app.graphics = Some(
        Graphics::new(
            adapter,
            device.clone(),
            queue.clone(),
            SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm),
            None,
            None,
            (256 * 1024 * 1024, 256 * 1024 * 1024),
        )
        .unwrap(),
    );
    app.edit_key(KeyCode::KeyD).unwrap();
    let asset = AssetId("rig".into());
    await_state(&mut app, |app| {
        app.graphics.as_ref().unwrap().models.contains_key(&asset)
    });
    let owner = app.instances[0];
    app.scene
        .insert_component(
            owner,
            voxy_gameplay::AudioSource {
                asset: "tone.wav".into(),
                import_settings: None,
                bus: 0,
                gain: 0.,
                looping: true,
                spatial: false,
                near: 1.,
                far: 5.,
            },
        )
        .unwrap();
    app.scene
        .insert_component(
            owner,
            voxy_gameplay::UiElement {
                origin: [0.1, 0.2],
                size: [0.3, 0.15],
                color: [0.2, 0.4, 0.6, 1.],
                layer: 0,
                enabled: true,
                action: Some("native-test".into()),
                text: None,
            },
        )
        .unwrap();
    app.commit_authoring().unwrap();
    app.audio
        .set_output_mode(crate::audio_session::AudioOutputMode::Native);
    let before_play_idle = if active_to_idle {
        let file = temp.0.join("before-play-idle.voxyedr");
        app.request_editor_file_save(file.clone(), EditorSavePolicy::desktop())
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        while app.editor_save.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline);
            assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
        }
        Some(file)
    } else {
        None
    };
    app.toggle_play().unwrap();
    await_state(&mut app, |app| {
        app.play.playing.is_some() && app.audio.play().is_some()
    });
    app.advance_game(7. / 60.).unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let _ = scene_pixels(&mut app);
        if app.ui_live.ready() && !app.graphics.as_ref().unwrap().ui_draws.is_empty() {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let ticks = app.play.simulation_ticks;
    let frames = app.audio.play().unwrap().frames;
    let rate = app
        .audio
        .play()
        .unwrap()
        .output_connection()
        .unwrap()
        .sample_rate();
    println!("NATIVE EDITOR output rate={rate}, gain=0, ticks={ticks}, frames={frames}");
    let graphics = app.graphics.as_ref().unwrap();
    let ui_pixels =
        crate::ui_live::checkpoint_pixels(&graphics.renderer, &device, &queue, &graphics.ui_draws);
    let model_pixels = crate::animated_models::tests::pixels(
        &graphics.renderer,
        &device,
        &queue,
        graphics
            .animated_models
            .geometries(app.instances[0])
            .unwrap()
            .collect(),
        glam::Mat4::IDENTITY,
    );
    let policy = EditorSavePolicy::desktop();
    let target = temp.0.join("target");
    std::fs::create_dir(&target).unwrap();
    let file = target.join("native.voxyedr");
    app.request_editor_file_save(file.clone(), policy.clone())
        .unwrap();
    while app.editor_save.is_some() {
        let _ = scene_pixels(&mut app);
        assert!(Instant::now() < deadline, "native save timeout");
        assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
    }
    let saved = EditorCheckpoint::read_file(&file, policy.file).unwrap();
    let audio_json = saved
        .game
        .audio
        .to_json(policy.file.game.max_audio_bytes)
        .unwrap();
    assert!(audio_json.contains("\"output\""));
    std::fs::remove_file(root.join("rig.glb")).unwrap();
    std::fs::remove_file(root.join("tone.wav")).unwrap();
    app.advance_game(2. / 60.).unwrap();
    assert!(app.play.simulation_ticks > ticks);
    let source = app.scene.identity();
    // Cancellation after issuing the native resume must preserve the original
    // scene and playback, even while the backend acknowledgement is in flight.
    app.request_editor_file_load(file.clone(), target.clone(), policy.clone())
        .unwrap();
    loop {
        let _ = scene_pixels(&mut app);
        assert!(
            Instant::now() < deadline,
            "native audio readiness timeout: {:?}",
            app.editor_load
        );
        assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
        if matches!(app.editor_load, Some(PendingEditorLoad::Audio { .. })) {
            break;
        }
        assert!(
            app.editor_load.is_some(),
            "load ended before native readiness"
        );
    }
    let token = app.scene.change_token();
    let source_frames = app.audio.play().unwrap().frames;
    let _ = scene_pixels(&mut app);
    assert!(matches!(
        app.editor_load,
        Some(PendingEditorLoad::Audio { .. })
    ));
    assert!(app.cancel_editor_file_load());
    while app.editor_load.is_some() {
        let _ = scene_pixels(&mut app);
        assert!(Instant::now() < deadline);
    }
    assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
    assert_eq!(app.scene.identity(), source);
    assert_eq!(app.scene.change_token(), token);
    assert_eq!(app.audio.play().unwrap().frames, source_frames);
    assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
    // Actual file/CPU/GPU/UI/device stages now commit without any source files.
    for _ in 0..2 {
        let source = app.scene.identity();
        app.request_editor_file_load(file.clone(), target.clone(), policy.clone())
            .unwrap();
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(
                Instant::now() < deadline,
                "native load timeout: {:?}",
                app.editor_load
            );
            assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
        }
        assert_ne!(app.scene.identity(), source);
        assert!(app.error.is_none() && app.snapshot_error.is_none());
        assert!(app.play.playing.is_some() && app.play.simulation.is_some());
        assert_eq!(app.play.simulation_ticks, ticks);
        assert_eq!(app.audio.play().unwrap().frames, frames);
        assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
        let output = app.audio.play().unwrap().output_connection().unwrap();
        assert_eq!(output.sample_rate(), rate);
        assert_eq!(output.stats().errors, 0);
        let bindings = app
            .object_ids
            .iter()
            .cloned()
            .zip(app.instances.iter().copied())
            .collect::<Vec<_>>();
        let view = voxy_scene::SceneCheckpointView::new(&app.scene, bindings).unwrap();
        let max_input = policy.file.game.ui.publications.max_layout_bytes;
        assert_eq!(
            app.game_ui_input
                .checkpoint_historical(&view, max_input)
                .unwrap()
                .to_json(max_input)
                .unwrap(),
            saved.game.ui_input.to_json(max_input).unwrap(),
        );
        let graphics = app.graphics.as_ref().unwrap();
        assert_eq!(
            crate::animated_models::tests::pixels(
                &graphics.renderer,
                &device,
                &queue,
                graphics
                    .animated_models
                    .geometries(app.instances[0])
                    .unwrap()
                    .collect(),
                glam::Mat4::IDENTITY,
            ),
            model_pixels
        );
        assert_eq!(
            crate::ui_live::checkpoint_pixels(
                &graphics.renderer,
                &device,
                &queue,
                &graphics.ui_draws
            ),
            ui_pixels
        );
        assert!(
            !graphics
                .animated_models
                .geometries(app.instances[0])
                .unwrap()
                .collect::<Vec<_>>()
                .is_empty()
        );
        for _ in 0..120 {
            app.advance_game(1. / 60.).unwrap();
            let _ = scene_pixels(&mut app);
        }
        assert_eq!(app.play.simulation_ticks, ticks + 120);
        assert!(app.audio.play().unwrap().frames > frames);
    }
    if let Some(idle_file) = before_play_idle {
        // The source is still playing on the actual native output. Preparation
        // and cancellation must not stop it before final world publication.
        app.request_editor_file_load(idle_file.clone(), target.clone(), policy.clone())
            .unwrap();
        loop {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline);
            assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
            if matches!(app.editor_load, Some(PendingEditorLoad::Audio { .. })) {
                break;
            }
            assert!(app.editor_load.is_some());
        }
        let original = app.scene.change_token();
        let original_frames = app.audio.play().unwrap().frames;
        let _ = scene_pixels(&mut app);
        assert!(matches!(
            app.editor_load,
            Some(PendingEditorLoad::Audio { .. })
        ));
        assert!(app.cancel_editor_file_load());
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline);
        }
        assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
        assert_eq!(app.scene.change_token(), original);
        assert_eq!(app.audio.play().unwrap().frames, original_frames);
        let source = app.scene.identity();
        app.request_editor_file_load(idle_file, target.clone(), policy.clone())
            .unwrap();
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline);
            assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
        }
        assert_ne!(app.scene.identity(), source);
        println!(
            "NATIVE ACTIVE-TO-IDLE cancel preserved source; committed idle scene and queued output retirement"
        );
    } else {
        app.toggle_play().unwrap();
    }
    assert!(app.play.playing.is_none() && app.audio.play().is_none());
    // Stop closes the old real device asynchronously. Admission must wait for
    // that owner to retire, then idle save/load must keep native policy intact.
    while app.audio.checkpoint_admission().is_err() {
        app.tick().unwrap();
        assert!(
            Instant::now() < deadline,
            "stopped native output did not retire"
        );
        std::thread::yield_now();
    }
    let idle_file = target.join("native-idle.voxyedr");
    app.request_editor_file_save(idle_file.clone(), policy.clone())
        .unwrap();
    while app.editor_save.is_some() {
        let _ = scene_pixels(&mut app);
        assert!(Instant::now() < deadline);
        assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
    }
    let idle = EditorCheckpoint::read_file(&idle_file, policy.file).unwrap();
    let idle_audio: serde_json::Value = serde_json::from_str(
        &idle
            .game
            .audio
            .to_json(policy.file.game.max_audio_bytes)
            .unwrap(),
    )
    .unwrap();
    assert!(idle_audio["play"].is_null());
    let source = app.scene.identity();
    app.request_editor_file_load(idle_file, target.clone(), policy.clone())
        .unwrap();
    while app.editor_load.is_some() {
        let _ = scene_pixels(&mut app);
        assert!(Instant::now() < deadline);
        assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
    }
    assert_ne!(app.scene.identity(), source);
    assert!(app.play.playing.is_none() && app.audio.play().is_none());
    assert_eq!(
        app.audio.output_mode(),
        crate::audio_session::AudioOutputMode::Native
    );
    assert!(app.audio.checkpoint_output_connection().unwrap().is_none());
    if active_to_idle {
        // A valid outer editor file with corrupt nested PCM must be rejected by
        // the read worker before acquiring source fences or opening any device.
        let mut corrupt = EditorCheckpoint::read_file(&file, policy.file).unwrap();
        let mut audio: serde_json::Value = serde_json::from_str(
            &corrupt
                .game
                .audio
                .to_json(policy.file.game.max_audio_bytes)
                .unwrap(),
        )
        .unwrap();
        audio["play"]["playback"]["output"][0] = serde_json::json!(0);
        corrupt.game.audio = crate::audio_session::checkpoint::AudioSessionCheckpoint::from_json(
            &serde_json::to_string(&audio).unwrap(),
            policy.file.game.max_audio_bytes,
        )
        .unwrap();
        let corrupt_file = target.join("corrupt-native-output.voxyedr");
        corrupt.save_file(&corrupt_file, policy.file).unwrap();
        let source = app.scene.change_token();
        app.request_editor_file_load(corrupt_file, target.clone(), policy.clone())
            .unwrap();
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline);
            assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
            assert!(app.audio.checkpoint_output_connection().unwrap().is_none());
        }
        assert!(app.snapshot_error.is_some());
        assert_eq!(app.scene.change_token(), source);
        assert!(app.audio.checkpoint_admission().is_ok());
        // A later source-admission failure must also retire an already opened,
        // paused provisional device and restore the same idle source.
        let source = app.scene.change_token();
        let mut tight = policy.clone();
        tight.resources.max_bytes = 1;
        app.request_editor_file_load(file.clone(), target.clone(), tight)
            .unwrap();
        let mut saw_device = false;
        let mut saw_ready_device = false;
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline);
            saw_device |= matches!(app.editor_load, Some(PendingEditorLoad::Device { .. }));
            saw_ready_device |=
                saw_device && matches!(app.editor_load, Some(PendingEditorLoad::Fencing { .. }));
        }
        assert!(saw_device && saw_ready_device);
        assert!(app.snapshot_error.is_some());
        assert_eq!(app.scene.change_token(), source);
        assert!(app.play.playing.is_none() && app.audio.play().is_none());
        assert!(app.restore_barrier.is_none() && app.resource_checkpoint.is_none());
        while app.audio.checkpoint_admission().is_err() {
            app.tick().unwrap();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        for cancel_device in [true, false] {
            let source = app.scene.change_token();
            app.request_editor_file_load(file.clone(), target.clone(), policy.clone())
                .unwrap();
            loop {
                let _ = scene_pixels(&mut app);
                assert!(
                    Instant::now() < deadline,
                    "startup stage timeout: {:?}",
                    app.editor_load
                );
                assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
                let reached = if cancel_device {
                    matches!(app.editor_load, Some(PendingEditorLoad::Device { .. }))
                } else {
                    matches!(app.editor_load, Some(PendingEditorLoad::Audio { .. }))
                };
                if reached {
                    break;
                }
                assert!(
                    app.editor_load.is_some(),
                    "startup ended before cancel stage"
                );
            }
            if !cancel_device {
                let _ = scene_pixels(&mut app); // Begin resume while new PCM is still held.
                assert!(matches!(
                    app.editor_load,
                    Some(PendingEditorLoad::Audio { .. })
                ));
            }
            assert!(app.cancel_editor_file_load());
            while app.editor_load.is_some() {
                let _ = scene_pixels(&mut app);
                assert!(Instant::now() < deadline);
            }
            assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
            assert_eq!(app.scene.change_token(), source);
            assert!(app.play.playing.is_none() && app.audio.play().is_none());
            while app.audio.checkpoint_admission().is_err() {
                app.tick().unwrap();
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
        }
        let source = app.scene.identity();
        app.request_editor_file_load(file.clone(), target.clone(), policy.clone())
            .unwrap();
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(
                Instant::now() < deadline,
                "idle-to-native timeout: {:?}",
                app.editor_load
            );
            assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
        }
        assert_ne!(app.scene.identity(), source);
        assert!(app.play.playing.is_some() && app.play.simulation.is_some());
        assert_eq!(app.play.simulation_ticks, ticks);
        assert_eq!(app.audio.play().unwrap().frames, frames);
        let output = app.audio.play().unwrap().output_connection().unwrap();
        assert_eq!(output.sample_rate(), rate);
        assert_eq!(output.stats().errors, 0);
        let graphics = app.graphics.as_ref().unwrap();
        assert_eq!(
            crate::ui_live::checkpoint_pixels(
                &graphics.renderer,
                &device,
                &queue,
                &graphics.ui_draws
            ),
            ui_pixels
        );
        assert_eq!(
            crate::animated_models::tests::pixels(
                &graphics.renderer,
                &device,
                &queue,
                graphics
                    .animated_models
                    .geometries(app.instances[0])
                    .unwrap()
                    .collect(),
                glam::Mat4::IDENTITY,
            ),
            model_pixels
        );
        for _ in 0..120 {
            app.advance_game(1. / 60.).unwrap();
            let _ = scene_pixels(&mut app);
        }
        assert_eq!(app.play.simulation_ticks, ticks + 120);
        assert!(app.audio.play().unwrap().frames > frames);
        println!(
            "IDLE-TO-NATIVE source-free device/Audio cancellation, exact counters/pixels and 120-frame continuation PASS"
        );
    }
    if native_to_offline {
        let expected: serde_json::Value = serde_json::from_str(&audio_json).unwrap();
        let output_bytes: Vec<u8> =
            serde_json::from_value(expected["play"]["playback"]["output"].clone()).unwrap();
        let pending = voxy_audio_device::OutputCheckpoint::from_bytes(
            &output_bytes,
            voxy_audio_device::OutputCheckpointLimits {
                max_bytes: policy.file.game.max_audio_bytes,
                max_queue_frames: 4096,
                max_pump_frames: 256,
                sample_rate: rate,
            },
        )
        .unwrap();
        let pending_frames =
            pending.queue.frames.len() + pending.pump.block.len() - pending.pump.cursor;
        let original_connection = app
            .audio
            .play()
            .unwrap()
            .output_connection()
            .unwrap()
            .clone();
        app.audio
            .set_output_mode(crate::audio_session::AudioOutputMode::Offline);
        let source = app.scene.change_token();
        let source_frames = app.audio.play().unwrap().frames;
        app.request_editor_file_load(file.clone(), target.clone(), policy.clone())
            .unwrap();
        loop {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline);
            assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
            if matches!(app.editor_load, Some(PendingEditorLoad::Audio { .. })) {
                break;
            }
            assert!(app.editor_load.is_some());
        }
        let _ = scene_pixels(&mut app);
        assert!(matches!(
            app.editor_load,
            Some(PendingEditorLoad::Audio { .. })
        ));
        assert!(app.cancel_editor_file_load());
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline);
        }
        assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
        assert_eq!(app.scene.change_token(), source);
        assert_eq!(app.audio.play().unwrap().frames, source_frames);
        assert!(
            app.audio
                .play()
                .unwrap()
                .output_connection()
                .unwrap()
                .shares_output(&original_connection)
        );
        app.request_editor_file_load(file.clone(), target.clone(), policy.clone())
            .unwrap();
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline);
            assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
        }
        while app.audio.checkpoint_admission().is_err() {
            app.tick().unwrap();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(app.play.playing.is_some());
        assert!(app.audio.play().unwrap().output_connection().is_none());
        assert_eq!(app.play.simulation_ticks, ticks);
        assert_eq!(app.audio.play().unwrap().frames, frames);
        let bindings = app
            .object_ids
            .iter()
            .cloned()
            .zip(app.instances.iter().copied())
            .collect::<Vec<_>>();
        let actual = capture_offline_audio_playback(&mut app.audio, &bindings);
        for field in ["runtime", "clock", "pcm", "output"] {
            assert_eq!(actual[field], expected["play"]["playback"][field]);
        }
        for _ in 0..120 {
            app.advance_game(1. / 60.).unwrap();
            let _ = scene_pixels(&mut app);
        }
        assert_eq!(app.play.simulation_ticks, ticks + 120);
        assert_eq!(
            app.audio.play().unwrap().frames,
            frames + 96000 - pending_frames as u64
        );
        // A native archive also loads on an already logical/offline source
        // without opening hardware; it still retains the archived PCM prefix.
        app.request_editor_file_load(file.clone(), target.clone(), policy.clone())
            .unwrap();
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline);
            assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
        }
        assert!(app.audio.play().unwrap().output_connection().is_none());
        assert!(app.audio.checkpoint_output_connection().unwrap().is_none());
        assert_eq!(app.audio.play().unwrap().frames, frames);
        app.audio
            .set_output_mode(crate::audio_session::AudioOutputMode::Native);
        app.request_editor_file_load(file.clone(), target.clone(), policy.clone())
            .unwrap();
        while app.editor_load.is_some() {
            let _ = scene_pixels(&mut app);
            assert!(Instant::now() < deadline);
            assert!(app.snapshot_error.is_none(), "{:?}", app.snapshot_error);
        }
        assert!(app.audio.play().unwrap().output_connection().is_some());
        assert_eq!(app.audio.play().unwrap().frames, frames);
        println!(
            "NATIVE-TO-OFFLINE cancellation preserves source; exact pending PCM retained; 120-frame budget avoids replay; logical reload and native return PASS"
        );
    }
    app.stop_workers().unwrap();
    assert!(pollster::block_on(scope.pop()).is_none());
    println!(
        "NATIVE EDITOR save/cancel/two source-free loads/120-frame continuations/Stop/idle-save-load PASS"
    );
}
