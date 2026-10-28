use super::*;
#[test]
#[ignore = "requires real GPU; composed restore late validation and cancellation"]
fn late_gpu_rejection_and_cancellation_preserve_live_cache_and_scope_stack() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("RESTORE GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let outer = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let provider = voxy_assets::FileInputs::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../voxy_render/examples/assets"),
    )
    .unwrap();
    let mut inputs = voxy_assets::ImportInputs::new(4, 4 * 1024 * 1024);
    let asset = crate::import::load(
        &voxy_assets::SourcePath::new("animated-triangle.glb").unwrap(),
        &provider,
        &mut inputs,
    )
    .unwrap();
    let publication = Arc::new(
        inputs
            .finish(asset, |id, max| provider.read(id, max))
            .unwrap(),
    );
    let mut cache = ResidencyCache::default();
    let old = ModelGraphics::upload_on(&renderer, &device, &queue, publication.value(), &mut cache)
        .unwrap();
    let live = old.geometry_bytes();
    let state = cache.state();
    let images = cache.live_bytes();
    let mut scene = SceneGraph::new(4);
    let node = scene.spawn(None, Default::default()).unwrap();
    let bindings = PreparedBindings {
        scene: scene.identity(),
        models: [(voxy_assets::AssetId("model".into()), publication)].into(),
        owners: Default::default(),
    };
    let target = || batch::UploadTarget {
        renderer: &renderer,
        device: &device,
        queue: &queue,
        cache: &cache,
        live_geometry: live,
        replaced_geometry: live,
    };
    let pending = target()
        .stage_restore_with(&scene, &bindings, Default::default(), || {
            let _invalid = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("injected restore validation failure"),
                size: 4,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            Ok(())
        })
        .unwrap();
    assert!(pollster::block_on(pending.validate()).is_err());
    assert_eq!(cache.state(), state);
    assert_eq!(cache.live_bytes(), images);
    let rejected = target().stage_restore_with(&scene, &bindings, Default::default(), || {
        Err("late CPU rejection".into())
    });
    assert!(rejected.is_err());
    assert_eq!(cache.state(), state);
    let pending = target()
        .stage_restore(&scene, &bindings, Default::default())
        .unwrap();
    drop(pending);
    assert_eq!(cache.state(), state);
    let foreign = SceneGraph::new(4);
    assert!(
        target()
            .stage_restore(&foreign, &bindings, Default::default())
            .is_err()
    );
    let pending = target()
        .stage_restore(&scene, &bindings, Default::default())
        .unwrap();
    let ready = pollster::block_on(pending.validate()).unwrap();
    assert_eq!(ready.scene, scene.identity());
    ready.validate_destination(&scene, &target()).unwrap();
    assert!(ready.validate_destination(&foreign, &target()).is_err());
    let replacement = cache.fork_for_upload();
    assert_eq!(replacement.state(), cache.state());
    assert!(
        ready
            .validate_destination(
                &scene,
                &batch::UploadTarget {
                    renderer: &renderer,
                    device: &device,
                    queue: &queue,
                    cache: &replacement,
                    live_geometry: live,
                    replaced_geometry: live,
                }
            )
            .is_err()
    );
    assert!(
        ready
            .validate_destination(
                &scene,
                &batch::UploadTarget {
                    live_geometry: live + 1,
                    ..target()
                }
            )
            .is_err()
    );
    assert!(
        ready
            .validate_destination(
                &scene,
                &batch::UploadTarget {
                    replaced_geometry: 0,
                    ..target()
                }
            )
            .is_err()
    );
    let (other_device, other_queue) =
        pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    assert!(
        ready
            .validate_destination(
                &scene,
                &batch::UploadTarget {
                    device: &other_device,
                    queue: &other_queue,
                    ..target()
                }
            )
            .is_err()
    );

    assert_eq!(
        ready.cache.geometry_live,
        ready
            .models
            .values()
            .map(ModelGraphics::geometry_bytes)
            .sum::<u64>()
    );
    assert_eq!(cache.state(), state);
    let structure = scene.structure_revision();
    scene
        .set_local(
            node,
            voxy_scene::Transform {
                translation: glam::Vec3::X,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(scene.structure_revision(), structure);
    assert!(ready.validate_destination(&scene, &target()).is_err());
    scene.set_local(node, Default::default()).unwrap();
    assert_eq!(scene.structure_revision(), structure);
    assert!(ready.validate_destination(&scene, &target()).is_err());
    assert!(pollster::block_on(outer.pop()).is_none());
    drop(old);
}
