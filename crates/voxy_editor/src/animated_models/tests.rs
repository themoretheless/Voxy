use super::*;

fn fixture() -> Arc<ModelAsset> {
    Arc::new(
        ModelAsset::parse(
            include_bytes!("../../../voxy_render/examples/assets/animated-triangle.glb"),
            &[],
            voxy_render::ModelLimits::default(),
        )
        .unwrap(),
    )
}
fn request(owner: NodeId, model: &Arc<ModelAsset>, speed: f32) -> Request {
    Request {
        owner,
        model: model.clone(),
        settings: ModelAnimation {
            speed,
            ..ModelAnimation::default()
        },
    }
}
fn pixels(
    renderer: &SceneRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    geometries: Vec<&SceneGeometry>,
    matrix: glam::Mat4,
) -> Vec<u8> {
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
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
    let texture = renderer
        .upload_texture(device, queue, 1, 1, &[255; 4])
        .unwrap();
    let transform = renderer.create_transform(device, matrix).unwrap();
    let draws: Vec<_> = geometries
        .into_iter()
        .map(|geometry| voxy_render::SceneDraw {
            geometry,
            texture: &texture,
            transform: &transform,
            overlay: true,
        })
        .collect();
    let mut encoder = device.create_command_encoder(&Default::default());
    renderer
        .encode_views(
            &mut encoder,
            &color.create_view(&Default::default()),
            &depth.create_view(&Default::default()),
            wgpu::Color::BLACK,
            &[voxy_render::SceneView {
                viewport: [0, 0, 64, 64],
                draws: &draws,
            }],
        )
        .unwrap();
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256 * 64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        color.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(64),
            },
        },
        wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
    );
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let result = readback.slice(..).get_mapped_range().unwrap().to_vec();
    readback.unmap();
    result
}
#[test]
#[ignore = "requires real GPU; editor owner playback and scene draw"]
fn gpu_playback_shares_sources_matches_cpu_and_preserves_failed_revision() {
    let gpu = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(gpu.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut runtime = AnimatedModels::new(&renderer).unwrap();
    assert!(runtime.skinner.is_some());
    let mut scene = voxy_scene::SceneGraph::new(4);
    let first = scene.spawn(None, voxy_scene::Transform::default()).unwrap();
    let second = scene.spawn(None, voxy_scene::Transform::default()).unwrap();
    let model = fixture();
    assert!(
        runtime
            .synchronize(
                &renderer,
                &device,
                &queue,
                vec![request(first, &model, 1.), request(second, &model, 0.)],
                0,
                0,
                65536
            )
            .is_empty()
    );
    let source = |owner: NodeId| match &runtime.owners[&owner].primitives[0] {
        Primitive::Skin { source, .. } => Arc::as_ptr(source),
        _ => panic!("GPU skin expected"),
    };
    assert_eq!(source(first), source(second));
    let initial_bytes = runtime.allocation_bytes();
    assert!(
        runtime
            .synchronize(
                &renderer,
                &device,
                &queue,
                vec![request(first, &model, 1.), request(second, &model, 0.)],
                8,
                0,
                65536
            )
            .is_empty()
    );
    assert_eq!(runtime.allocation_bytes(), initial_bytes);
    let expected = model
        .scene_meshes(&model.sample_pose(Some(0), 8. / 60.).unwrap())
        .unwrap();
    let geometry = renderer.upload_mesh(&device, &expected[0]).unwrap();
    let mut min = glam::Vec3::splat(f32::INFINITY);
    let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
    for v in expected[0].vertices() {
        let p = glam::Vec3::from_array(v.position);
        min = min.min(p);
        max = max.max(p);
    }
    let matrix = glam::Mat4::from_translation(glam::Vec3::new(0., 0., 0.5))
        * glam::Mat4::from_scale(glam::Vec3::splat(1.5 / (max - min).max_element()))
        * glam::Mat4::from_translation(-(min + max) * 0.5);
    let moving = pixels(
        &renderer,
        &device,
        &queue,
        runtime.geometries(first).unwrap().collect(),
        matrix,
    );
    assert_eq!(
        moving,
        pixels(&renderer, &device, &queue, vec![&geometry], matrix)
    );
    assert!(moving.chunks_exact(4).filter(|p| p[0] > 0).count() > 50);
    assert_ne!(
        moving,
        pixels(
            &renderer,
            &device,
            &queue,
            runtime.geometries(second).unwrap().collect(),
            matrix
        )
    );
    // No extra fixed steps: repeated views/presents do not consume animation time.
    assert!(
        runtime
            .synchronize(
                &renderer,
                &device,
                &queue,
                vec![request(first, &model, 1.), request(second, &model, 0.)],
                8,
                0,
                65536
            )
            .is_empty()
    );
    assert_eq!(
        moving,
        pixels(
            &renderer,
            &device,
            &queue,
            runtime.geometries(first).unwrap().collect(),
            matrix
        )
    );
    // Pause/resume changes the existing owner clock without reallocating or resetting its pose.
    assert!(
        runtime
            .synchronize(
                &renderer,
                &device,
                &queue,
                vec![request(first, &model, 0.), request(second, &model, 0.)],
                8,
                0,
                initial_bytes
            )
            .is_empty()
    );
    assert_eq!(runtime.allocation_bytes(), initial_bytes);
    assert_eq!(
        moving,
        pixels(
            &renderer,
            &device,
            &queue,
            runtime.geometries(first).unwrap().collect(),
            matrix
        )
    );
    assert!(
        runtime
            .synchronize(
                &renderer,
                &device,
                &queue,
                vec![request(first, &model, 1.), request(second, &model, 0.)],
                8,
                0,
                initial_bytes
            )
            .is_empty()
    );
    assert_eq!(
        moving,
        pixels(
            &renderer,
            &device,
            &queue,
            runtime.geometries(first).unwrap().collect(),
            matrix
        )
    );
    // Replacement must admit alongside the still-live previous revision.
    let revised = Arc::new((*model).clone());
    assert!(
        runtime
            .update(
                &renderer,
                &device,
                &queue,
                first,
                revised,
                ModelAnimation::default(),
                9,
                0,
                initial_bytes
            )
            .is_err()
    );
    assert!(Arc::ptr_eq(&runtime.owners[&first].model, &model));
    assert_eq!(runtime.owners[&first].ticks, 8);
    assert_eq!(
        moving,
        pixels(
            &renderer,
            &device,
            &queue,
            runtime.geometries(first).unwrap().collect(),
            matrix
        )
    );
    assert!(
        runtime
            .synchronize(
                &renderer,
                &device,
                &queue,
                vec![request(first, &model, 1.)],
                8,
                0,
                65536
            )
            .is_empty()
    );
    assert!(runtime.geometries(second).is_none());
    assert!(runtime.allocation_bytes() < initial_bytes);
    runtime.clear();
    assert_eq!(runtime.allocation_bytes(), 0);
    assert!(runtime.sources.is_empty());
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "requires GPU upload; explicit CPU deformation fallback"]
fn cpu_fallback_supports_mixed_primitives_and_removal() {
    let gpu = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(gpu.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut model = (*fixture()).clone();
    let bind = model.scene_meshes(&model.skeleton.bind_pose()).unwrap();
    model.primitives.push(voxy_render::ModelPrimitive {
        geometry: ModelGeometry::Static(bind[0].clone()),
        color: [1.; 4],
    });
    let model = Arc::new(model);
    let mut runtime = AnimatedModels {
        skinner: None,
        owners: HashMap::new(),
        sources: HashMap::new(),
    };
    let mut scene = voxy_scene::SceneGraph::new(1);
    let owner = scene.spawn(None, voxy_scene::Transform::default()).unwrap();
    assert!(
        runtime
            .synchronize(
                &renderer,
                &device,
                &queue,
                vec![request(owner, &model, 1.)],
                0,
                0,
                65536
            )
            .is_empty()
    );
    assert_eq!(runtime.geometries(owner).unwrap().count(), 2);
    assert!(
        runtime
            .synchronize(
                &renderer,
                &device,
                &queue,
                vec![request(owner, &model, 1.)],
                8,
                0,
                65536
            )
            .is_empty()
    );
    assert!(runtime.sources.is_empty());
    assert!(
        runtime.owners[&owner]
            .primitives
            .iter()
            .all(|p| matches!(p, Primitive::Baked(_)))
    );
    assert!(
        runtime
            .synchronize(&renderer, &device, &queue, Vec::new(), 8, 0, 65536)
            .is_empty()
    );
    assert_eq!(runtime.allocation_bytes(), 0);
    assert!(pollster::block_on(scope.pop()).is_none());
}
