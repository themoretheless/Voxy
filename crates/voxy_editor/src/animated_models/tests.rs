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
        lod: None,
        textures: vec![],
        texture_storage: vec![],
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
                None,
                vec![],
                vec![],
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
        base_color_texture: None,
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

#[test]
#[ignore = "requires GPU; animated editor LOD camera and budget publication"]
fn animated_lod_views_budget_and_cpu_fallback() {
    let gpu = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(gpu.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut model = (*fixture()).clone();
    let ModelGeometry::Skinned(original) = &model.primitives[0].geometry else {
        panic!("skin required");
    };
    let indices = original.indices().to_vec();
    let mesh = voxy_render::SkinnedMesh::new(
        original.vertices().to_vec(),
        [indices.clone(), indices.clone()].concat(),
        original.joint_count(),
    )
    .unwrap();
    let d = voxy_render::LOD_BARYCENTRIC_DENOMINATOR;
    let witness = voxy_render::LodTriangleWitness {
        target_triangle: 0,
        weights: [[d, 0, 0], [0, d, 0], [0, 0, d]],
    };
    let lod = Arc::new(
        voxy_render::SkinnedLodMesh::new(
            Arc::new(mesh.clone()),
            vec![voxy_render::CertifiedLodVariant {
                indices,
                source_to_variant: vec![witness.clone(), witness.clone()],
                variant_to_source: vec![witness],
            }],
        )
        .unwrap(),
    );
    model.primitives[0].geometry = ModelGeometry::Skinned(mesh);
    let model = Arc::new(model);
    let mut graph = voxy_scene::SceneGraph::new(8);
    let owner = graph.spawn(None, voxy_scene::Transform::default()).unwrap();
    let far = voxy_render::SceneCamera {
        eye: glam::Vec3::new(0., 0., 10.),
        target: glam::Vec3::ZERO,
        up: glam::Vec3::Y,
        projection: voxy_render::SceneProjection::Orthographic {
            left: -100.,
            right: 100.,
            bottom: -100.,
            top: 100.,
            near: 0.1,
            far: 100.,
        },
    };
    let near = voxy_render::SceneCamera {
        eye: glam::Vec3::new(0., 0., 0.05),
        projection: voxy_render::SceneProjection::Perspective {
            vertical_fov: 1.,
            aspect: 1.,
            near: 0.1,
            far: 100.,
        },
        ..far
    };
    for cpu in [false, true] {
        let mut owners = AnimatedModels::new(&renderer).unwrap();
        if cpu {
            owners.skinner = None;
        }
        let make_request = || Request {
            owner,
            model: model.clone(),
            settings: ModelAnimation::default(),
            lod: Some(lod.clone()),
            textures: vec![],
            texture_storage: vec![],
        };
        assert!(
            owners
                .synchronize(&renderer, &device, &queue, vec![make_request()], 0, 0, 8192)
                .is_empty()
        );
        let base_bytes = owners.allocation_bytes();
        assert!(
            owners
                .select_lod(
                    &renderer,
                    &device,
                    owner,
                    0,
                    Some(far),
                    glam::Mat4::IDENTITY,
                    [100, 100],
                    0,
                    base_bytes
                )
                .is_err()
        );
        assert!(!owners.owners[&owner].lod_history.contains_key(&0));
        let certificate_positions = owners.owners[&owner]
            .prepared_lod
            .as_ref()
            .unwrap()
            .positions()
            .as_ptr();
        owners
            .select_lod(
                &renderer,
                &device,
                owner,
                0,
                Some(far),
                glam::Mat4::IDENTITY,
                [100, 100],
                0,
                8192,
            )
            .unwrap();
        owners
            .select_lod(
                &renderer,
                &device,
                owner,
                1,
                Some(near),
                glam::Mat4::IDENTITY,
                [100, 100],
                0,
                8192,
            )
            .unwrap();
        assert_eq!(
            owners.owners[&owner]
                .prepared_lod
                .as_ref()
                .unwrap()
                .positions()
                .as_ptr(),
            certificate_positions
        );
        // Rejected camera/world requests must not replace the accepted cache.
        let world = glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::new(1.2, 0.8, 2.),
            glam::Quat::from_rotation_y(0.3),
            glam::Vec3::new(1., 0., 0.),
        );
        let invalid_world = glam::Mat4::from_scale(glam::Vec3::splat(f32::NAN));
        for (matrix, viewport) in [(world, [0, 100]), (invalid_world, [100, 100])] {
            assert!(
                owners
                    .select_lod(
                        &renderer,
                        &device,
                        owner,
                        0,
                        Some(far),
                        matrix,
                        viewport,
                        0,
                        8192
                    )
                    .is_err()
            );
            let cached = owners.owners[&owner].prepared_lod.as_ref().unwrap();
            assert_eq!(cached.model(), glam::Mat4::IDENTITY);
            assert_eq!(cached.positions().as_ptr(), certificate_positions);
        }
        owners
            .select_lod(
                &renderer,
                &device,
                owner,
                0,
                Some(far),
                world,
                [100, 100],
                0,
                8192,
            )
            .unwrap();
        let cached = owners.owners[&owner].prepared_lod.as_ref().unwrap();
        assert_eq!(cached.model(), world);
        let reference = lod.prepare(cached.joints(), world).unwrap();
        assert_eq!(cached.positions(), reference.positions());
        assert_eq!(cached.levels(), reference.levels());
        owners
            .select_lod(
                &renderer,
                &device,
                owner,
                0,
                Some(far),
                glam::Mat4::IDENTITY,
                [100, 100],
                0,
                8192,
            )
            .unwrap();
        assert_eq!(owners.owners[&owner].lod_history[&0], 1);
        assert_eq!(owners.owners[&owner].lod_history[&1], 0);
        assert_eq!(
            owners
                .geometries_for_view(owner, 0)
                .unwrap()
                .next()
                .unwrap()
                .index_count(),
            3
        );
        assert_eq!(
            owners
                .geometries_for_view(owner, 1)
                .unwrap()
                .next()
                .unwrap()
                .index_count(),
            6
        );
        if !cpu {
            assert_eq!(owners.allocation_bytes(), base_bytes + 12);
        }
        let accepted_bytes = owners.allocation_bytes();
        assert!(
            owners
                .select_lod(
                    &renderer,
                    &device,
                    owner,
                    0,
                    Some(far),
                    glam::Mat4::IDENTITY,
                    [0, 100],
                    0,
                    8192
                )
                .is_err()
        );
        assert_eq!(owners.allocation_bytes(), accepted_bytes);
        assert_eq!(owners.owners[&owner].lod_history[&0], 1);
        // One camera returning to base must not evict a level another camera uses.
        owners
            .select_lod(
                &renderer,
                &device,
                owner,
                1,
                Some(far),
                glam::Mat4::IDENTITY,
                [100, 100],
                0,
                8192,
            )
            .unwrap();
        owners
            .select_lod(
                &renderer,
                &device,
                owner,
                0,
                Some(near),
                glam::Mat4::IDENTITY,
                [100, 100],
                0,
                8192,
            )
            .unwrap();
        assert_eq!(owners.evict_unused_lod(), 0);
        assert_eq!(owners.allocation_bytes(), accepted_bytes);
        // Closing the remaining reduced-detail view makes its level disposable.
        owners.retain_lod_views(&[0]);
        assert_eq!(owners.allocation_bytes(), base_bytes);
        assert!(!owners.owners[&owner].lod_history.contains_key(&1));
        assert_eq!(
            owners
                .geometries_for_view(owner, 0)
                .unwrap()
                .next()
                .unwrap()
                .index_count(),
            6
        );
        // Re-admission at the exact prior peak succeeds after unused residency retires.
        owners
            .select_lod(
                &renderer,
                &device,
                owner,
                0,
                Some(far),
                glam::Mat4::IDENTITY,
                [100, 100],
                0,
                accepted_bytes,
            )
            .unwrap();
        assert_eq!(owners.allocation_bytes(), accepted_bytes);
        assert!(
            owners
                .synchronize(&renderer, &device, &queue, vec![make_request()], 8, 0, 8192)
                .is_empty()
        );
        owners
            .select_lod(
                &renderer,
                &device,
                owner,
                0,
                Some(far),
                glam::Mat4::IDENTITY,
                [100, 100],
                0,
                8192,
            )
            .unwrap();
        let frame = owners
            .owners
            .get_mut(&owner)
            .unwrap()
            .playback
            .advance_with(0., |_, frame| Ok(frame.clone()))
            .unwrap();
        let baked = lod
            .prepare(&frame.skin_matrices, glam::Mat4::IDENTITY)
            .unwrap()
            .posed_scene_mesh(1, model.primitives[0].color)
            .unwrap();
        let mut min = glam::Vec3::splat(f32::INFINITY);
        let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
        for vertex in baked.vertices() {
            let p = glam::Vec3::from_array(vertex.position);
            min = min.min(p);
            max = max.max(p);
        }
        let matrix = glam::Mat4::from_translation(glam::Vec3::new(0., 0., 0.5))
            * glam::Mat4::from_scale(glam::Vec3::splat(1.5 / (max - min).max_element()))
            * glam::Mat4::from_translation(-(min + max) * 0.5);
        let actual = pixels(
            &renderer,
            &device,
            &queue,
            owners.geometries_for_view(owner, 0).unwrap().collect(),
            matrix,
        );
        assert!(actual.chunks_exact(4).filter(|pixel| pixel[0] > 0).count() > 50);
        let reference = renderer.upload_mesh(&device, &baked).unwrap();
        assert_eq!(
            actual,
            pixels(&renderer, &device, &queue, vec![&reference], matrix)
        );
        owners.clear();
        assert_eq!(owners.allocation_bytes(), 0);
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "requires GPU; accepted material revision and residency storage lifetime"]
fn gpu_material_revision_failure_preserves_texture_storage() {
    let gpu = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(gpu.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut model = (*fixture()).clone();
    model.primitives[0].base_color_texture = Some(voxy_render::ModelTexture {
        image: 0,
        sampling: voxy_render::TextureSampling::default(),
        use_mips: false,
    });
    let model = Arc::new(model);
    let revised = Arc::new((*model).clone());
    let mut graph = voxy_scene::SceneGraph::new(8);
    let owner = graph.spawn(None, voxy_scene::Transform::default()).unwrap();
    let storage = Arc::new(
        renderer
            .upload_texture(&device, &queue, 1, 1, &[255, 0, 0, 255])
            .unwrap(),
    );
    let weak_storage = Arc::downgrade(&storage);
    let red = Arc::new(
        renderer
            .texture_binding(
                &device,
                &storage,
                voxy_render::TextureSampling::default(),
                1,
            )
            .unwrap(),
    );
    let weak_red = Arc::downgrade(&red);
    let blue = Arc::new(
        renderer
            .upload_texture(&device, &queue, 1, 1, &[0, 0, 255, 255])
            .unwrap(),
    );
    let mut runtime = AnimatedModels::new(&renderer).unwrap();
    let mut original = request(owner, &model, 1.);
    original.textures = vec![Some(red.clone())];
    original.texture_storage = vec![storage.clone()];
    assert!(
        runtime
            .synchronize(&renderer, &device, &queue, vec![original], 0, 0, 8192)
            .is_empty()
    );
    drop(storage);
    drop(red);
    assert!(weak_storage.upgrade().is_some());
    let replacement = || {
        let mut next = request(owner, &revised, 1.);
        next.textures = vec![Some(blue.clone())];
        next.texture_storage = vec![blue.clone()];
        next
    };
    assert_eq!(
        runtime
            .synchronize(&renderer, &device, &queue, vec![replacement()], 8, 0, 0)
            .len(),
        1
    );
    assert!(Arc::ptr_eq(&runtime.owners[&owner].model, &model));
    assert_eq!(runtime.owners[&owner].ticks, 0);
    assert_eq!(
        runtime.texture(owner, 0).unwrap() as *const _,
        weak_red.as_ptr()
    );
    assert!(weak_storage.upgrade().is_some());
    let missing = request(owner, &revised, 1.);
    assert_eq!(
        runtime
            .synchronize(&renderer, &device, &queue, vec![missing], 8, 0, 8192)
            .len(),
        1
    );
    assert!(weak_storage.upgrade().is_some());
    assert!(
        runtime
            .synchronize(&renderer, &device, &queue, vec![replacement()], 8, 0, 8192)
            .is_empty()
    );
    assert!(Arc::ptr_eq(&runtime.owners[&owner].model, &revised));
    assert_eq!(
        runtime.texture(owner, 0).unwrap() as *const _,
        Arc::as_ptr(&blue)
    );
    assert!(weak_storage.upgrade().is_none());
    assert!(weak_red.upgrade().is_none());
    runtime.clear();
    assert!(runtime.texture(owner, 0).is_none());
    assert!(pollster::block_on(scope.pop()).is_none());
}
