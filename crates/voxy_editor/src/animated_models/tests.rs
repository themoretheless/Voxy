// Test-only frame producers; the production renderer receives accepted frames.
fn test_frame(
    model: &Arc<ModelAsset>,
    settings: ModelAnimation,
    time: f32,
) -> Arc<voxy_animation::AnimatorFrame> {
    let mut playback = crate::model_playback::ModelPlayback::new(model.clone(), settings).unwrap();
    Arc::new(
        playback
            .advance_with(time, |_, frame| Ok(frame.clone()))
            .unwrap(),
    )
}
struct FrameFixture {
    render: AnimatedModels,
    clocks: HashMap<
        NodeId,
        (
            Arc<ModelAsset>,
            ModelAnimation,
            crate::model_playback::ModelPlayback,
            u64,
        ),
    >,
}
impl std::ops::Deref for FrameFixture {
    type Target = AnimatedModels;
    fn deref(&self) -> &Self::Target {
        &self.render
    }
}
impl std::ops::DerefMut for FrameFixture {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.render
    }
}
impl FrameFixture {
    fn new(renderer: &SceneRenderer) -> Result<Self, String> {
        Ok(Self {
            render: AnimatedModels::new(renderer)?,
            clocks: HashMap::new(),
        })
    }
    fn synchronize_fixture(
        &mut self,
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mut requests: Vec<Request>,
        ticks: u64,
        live: u64,
        budget: u64,
    ) -> Vec<(NodeId, String)> {
        let active: HashSet<_> = requests.iter().map(|request| request.owner).collect();
        self.clocks.retain(|owner, _| active.contains(owner));
        let mut candidates = Vec::new();
        for request in &mut requests {
            let previous = self.clocks.get(&request.owner);
            let reset = previous.is_none_or(|old| {
                !Arc::ptr_eq(&old.0, &request.model) || old.1.clip != request.settings.clip
            });
            let mut playback = if reset {
                crate::model_playback::ModelPlayback::new(
                    request.model.clone(),
                    request.settings.clone(),
                )
                .unwrap()
            } else {
                let mut playback = previous.unwrap().2.clone();
                playback.set_speed(request.settings.speed).unwrap();
                playback
                    .set_root_motion_joint(
                        request
                            .settings
                            .resolve_motion_joint(&request.model)
                            .unwrap(),
                    )
                    .unwrap();
                playback
            };
            let steps = if reset {
                0
            } else {
                ticks.saturating_sub(previous.unwrap().3).min(8)
            };
            let mut frame = playback
                .advance_with(0., |_, frame| Ok(frame.clone()))
                .unwrap();
            for _ in 0..steps {
                frame = playback
                    .advance_with(1. / 60., |_, frame| Ok(frame.clone()))
                    .unwrap();
            }
            request.frame = Some(Arc::new(frame));
            candidates.push((
                request.owner,
                (
                    request.model.clone(),
                    request.settings.clone(),
                    playback,
                    ticks,
                ),
            ));
        }
        let errors = self
            .render
            .synchronize(renderer, device, queue, requests, ticks, live, budget);
        for (owner, candidate) in candidates {
            if !errors.iter().any(|(failed, _)| *failed == owner) {
                self.clocks.insert(owner, candidate);
            }
        }
        errors
    }
}

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
        frame: None,
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
    let mut runtime = FrameFixture::new(&renderer).unwrap();
    assert!(runtime.skinner.is_some());
    let mut scene = voxy_scene::SceneGraph::new(4);
    let first = scene.spawn(None, voxy_scene::Transform::default()).unwrap();
    let second = scene.spawn(None, voxy_scene::Transform::default()).unwrap();
    let model = fixture();
    assert!(
        runtime
            .synchronize_fixture(
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
            .synchronize_fixture(
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
    // Waiting for a fixed-tick frame retains GPU geometry and its accepted version.
    let accepted_frame = runtime.owners[&first].frame.clone();
    assert!(
        runtime
            .render
            .synchronize(
                &renderer,
                &device,
                &queue,
                vec![request(first, &model, 1.), request(second, &model, 0.)],
                999,
                0,
                initial_bytes
            )
            .is_empty()
    );
    assert!(Arc::ptr_eq(&accepted_frame, &runtime.owners[&first].frame));
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
    // No extra fixed steps: repeated views/presents do not consume animation time.
    assert!(
        runtime
            .synchronize_fixture(
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
            .synchronize_fixture(
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
            .synchronize_fixture(
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
                revised.clone(),
                ModelAnimation::default(),
                Some(test_frame(&revised, ModelAnimation::default(), 0.)),
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
            .synchronize_fixture(
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
    let mut runtime = FrameFixture {
        render: AnimatedModels {
            skinner: None,
            owners: HashMap::new(),
            sources: HashMap::new(),
        },
        clocks: HashMap::new(),
    };
    let mut scene = voxy_scene::SceneGraph::new(1);
    let owner = scene.spawn(None, voxy_scene::Transform::default()).unwrap();
    assert!(
        runtime
            .synchronize_fixture(
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
            .synchronize_fixture(
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
            .synchronize_fixture(&renderer, &device, &queue, Vec::new(), 8, 0, 65536)
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
        let mut owners = FrameFixture::new(&renderer).unwrap();
        if cpu {
            owners.skinner = None;
        }
        let make_request = || Request {
            owner,
            frame: None,
            model: model.clone(),
            settings: ModelAnimation::default(),
            lod: Some(lod.clone()),
            textures: vec![],
            texture_storage: vec![],
        };
        assert!(
            owners
                .synchronize_fixture(&renderer, &device, &queue, vec![make_request()], 0, 0, 8192)
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
        for view in [0, 1] {
            let selected = owners
                .geometry_inputs_for_view(owner, view)
                .unwrap()
                .next()
                .unwrap();
            assert_eq!(selected.gpu_deformed, !cpu);
            assert!(std::ptr::eq(
                selected.geometry,
                owners
                    .geometries_for_view(owner, view)
                    .unwrap()
                    .next()
                    .unwrap()
            ));
        }
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
                .synchronize_fixture(&renderer, &device, &queue, vec![make_request()], 8, 0, 8192)
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
        let frame = owners.owners[&owner].frame.clone();
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
    let mut runtime = FrameFixture::new(&renderer).unwrap();
    let mut original = request(owner, &model, 1.);
    original.textures = vec![Some(red.clone())];
    original.texture_storage = vec![storage.clone()];
    assert!(
        runtime
            .synchronize_fixture(&renderer, &device, &queue, vec![original], 0, 0, 8192)
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
            .synchronize_fixture(&renderer, &device, &queue, vec![replacement()], 8, 0, 0)
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
            .synchronize_fixture(&renderer, &device, &queue, vec![missing], 8, 0, 8192)
            .len(),
        1
    );
    assert!(weak_storage.upgrade().is_some());
    assert!(
        runtime
            .synchronize_fixture(&renderer, &device, &queue, vec![replacement()], 8, 0, 8192)
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

#[test]
#[ignore = "requires real GPU; fixed-tick hierarchy owners at runtime capacity"]
fn fixed_tick_hierarchy_owners_share_gpu_source_at_capacity() {
    let gpu = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(gpu.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let model = fixture();
    let asset = voxy_assets::AssetId("rig".into());
    let models = std::collections::BTreeMap::from([(asset.clone(), model.clone())]);
    let mut scene = voxy_scene::SceneGraph::new(crate::scene_limits::OBJECTS);
    let mut owners = Vec::new();
    for _ in 0..crate::animation_runtime::MAX_OWNERS {
        let owner = scene.spawn(None, Default::default()).unwrap();
        scene
            .insert_component(
                owner,
                crate::ModelInstance {
                    asset: asset.clone(),
                },
            )
            .unwrap();
        scene
            .insert_component(owner, crate::ModelPart { node: u32::MAX })
            .unwrap();
        owners.push(owner);
        for node in 0..2 {
            let part = scene.spawn(Some(owner), Default::default()).unwrap();
            scene
                .insert_component(
                    part,
                    crate::ModelInstance {
                        asset: asset.clone(),
                    },
                )
                .unwrap();
            scene
                .insert_component(part, crate::ModelPart { node })
                .unwrap();
        }
    }
    let playback = crate::animation_runtime::AnimationRuntime::default()
        .prepare(&scene, &models, 1. / 60.)
        .unwrap();
    let requests = || {
        owners
            .iter()
            .map(|&owner| {
                let mut request = request(owner, &model, 1.);
                request.frame = playback.frame(owner, &model);
                request
            })
            .collect()
    };
    let mut render = AnimatedModels::new(&renderer).unwrap();
    assert!(render.skinner.is_some());
    assert!(
        render
            .synchronize(
                &renderer,
                &device,
                &queue,
                requests(),
                playback.serial(),
                0,
                1_048_576
            )
            .is_empty()
    );
    assert_eq!(render.counts(), (128, 128, 1));
    let bytes = render.allocation_bytes();
    let source = match &render.owners[&owners[0]].primitives[0] {
        Primitive::Skin { source, .. } => Arc::as_ptr(source),
        _ => panic!("GPU skin expected"),
    };
    for &owner in &owners {
        let Primitive::Skin { source: actual, .. } = &render.owners[&owner].primitives[0] else {
            panic!("GPU skin expected")
        };
        assert_eq!(Arc::as_ptr(actual), source);
        assert!(Arc::ptr_eq(
            &render.owners[&owner].frame,
            &playback.frame(owner, &model).unwrap()
        ));
    }
    let frame = playback.frame(owners[0], &model).unwrap();
    let baked = model.scene_meshes(&frame.pose).unwrap();
    let reference = renderer.upload_mesh(&device, &baked[0]).unwrap();
    let matrix = glam::Mat4::from_translation(glam::Vec3::new(-0.1, 0., 0.5));
    let expected = pixels(&renderer, &device, &queue, vec![&reference], matrix);
    assert!(
        expected
            .chunks_exact(4)
            .any(|pixel| pixel[..3] != [0, 0, 0])
    );
    let draws = owners
        .iter()
        .flat_map(|&owner| render.geometries(owner).unwrap())
        .collect();
    assert_eq!(pixels(&renderer, &device, &queue, draws, matrix), expected);
    assert!(
        render
            .synchronize(
                &renderer,
                &device,
                &queue,
                requests(),
                playback.serial(),
                0,
                1_048_576
            )
            .is_empty()
    );
    assert_eq!(render.allocation_bytes(), bytes);
    render.clear();
    assert_eq!(render.allocation_bytes(), 0);
    assert_eq!(render.counts(), (0, 0, 0));
    assert!(pollster::block_on(scope.pop()).is_none());
    println!(
        "VOXY_HIERARCHY_CAPACITY_GPU owners=128 parts=256 sources=1 bytes={bytes} cpu_pixels_equal=true stop_bytes=0"
    );
}

#[test]
#[ignore = "requires real GPU; curved character turn and in-place skin palette"]
fn curved_root_rotation_collision_renders_once_with_in_place_gpu_palette() {
    rig_trajectory_gpu(false);
}
#[test]
#[ignore = "requires real GPU; composed root path and in-place skin palette"]
fn composed_root_motion_collision_renders_once_with_in_place_gpu_palette() {
    rig_trajectory_gpu(true);
}
fn rig_trajectory_gpu(composed: bool) {
    use glam::{Mat4, Quat, Vec3};
    use voxy_gameplay::{BoxCollider, CharacterBody, CharacterPhysics};
    let gpu = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(gpu.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let model = Arc::new(
        ModelAsset::parse(
            if composed {
                include_bytes!("../../../voxy_render/examples/assets/root-moving-turn.glb")
                    .as_slice()
            } else {
                include_bytes!("../../../voxy_render/examples/assets/root-pivot-turn.glb")
                    .as_slice()
            },
            &[],
            voxy_render::ModelLimits::default(),
        )
        .unwrap(),
    );
    let mut scene = voxy_scene::SceneGraph::new(4);
    let first = scene.spawn(None, Default::default()).unwrap();
    let second = scene.spawn(None, Default::default()).unwrap();
    scene
        .insert_component(
            first,
            CharacterBody {
                half_extents: [0.4, 0.1, 0.02],
                speed: 0.,
                gravity: 0.,
                ..Default::default()
            },
        )
        .unwrap();
    let wall = scene
        .spawn(
            None,
            voxy_scene::Transform {
                translation: Vec3::Z * 0.25,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            wall,
            BoxCollider {
                half_extents: [2., 2., 0.02],
            },
        )
        .unwrap();
    let asset = voxy_assets::AssetId("turn".into());
    for owner in [first, second] {
        scene
            .insert_component(
                owner,
                crate::ModelInstance {
                    asset: asset.clone(),
                },
            )
            .unwrap();
        scene
            .insert_component(
                owner,
                ModelAnimation {
                    root_motion_rotation: owner == first,
                    root_motion_axes: [composed && owner == first; 3],
                    root_motion_bone: "root".into(),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    let models = std::collections::BTreeMap::from([(asset, model.clone())]);
    let candidate = crate::animation_runtime::AnimationRuntime::default()
        .prepare(&scene, &models, 1. / 60.)
        .unwrap();
    let authored = candidate.frame(second, &model).unwrap();
    let accepted = candidate.frame(first, &model).unwrap();
    // The independent CPU reference starts from bind, not the extracted frame.
    assert_eq!(accepted.pose, model.skeleton.bind_pose());
    let mut physics = CharacterPhysics::new(&scene, 1, 1);
    let mut input = voxy_gameplay::player_input().unwrap();
    let receipt = physics
        .fixed_step_with_motion_and_rigid_trajectories(
            &mut scene,
            &mut input,
            1. / 60.,
            candidate.motions(),
            &candidate.trajectories(),
        )
        .unwrap()[0];
    let (angle, _) = crate::animation_smoke::rotation_contact(composed);
    assert!(!receipt.complete);
    assert!(
        scene
            .local(first)
            .unwrap()
            .rotation
            .abs_diff_eq(Quat::from_rotation_y(angle as f32), 1e-6)
    );
    let mut render = AnimatedModels::new(&renderer).unwrap();
    assert!(render.skinner.is_some());
    let requests = || {
        let mut a = request(first, &model, 1.);
        a.frame = Some(accepted.clone());
        let mut b = request(second, &model, 1.);
        b.frame = Some(authored.clone());
        vec![a, b]
    };
    assert!(
        render
            .synchronize(&renderer, &device, &queue, requests(), 1, 0, 65536)
            .is_empty()
    );
    assert_eq!(render.counts(), (2, 2, 1));
    let source = |owner| match &render.owners[&owner].primitives[0] {
        Primitive::Skin { source, .. } => Arc::as_ptr(source),
        _ => panic!("GPU skin expected"),
    };
    assert_eq!(source(first), source(second));
    assert!(Arc::ptr_eq(&render.owners[&first].frame, &accepted));
    let reference_mesh = model.scene_meshes(&model.skeleton.bind_pose()).unwrap();
    let reference = renderer.upload_mesh(&device, &reference_mesh[0]).unwrap();
    let matrix =
        Mat4::from_translation(Vec3::new(-0.1, 0., 0.5)) * scene.world_matrix(first).unwrap();
    let actual = pixels(
        &renderer,
        &device,
        &queue,
        render.geometries(first).unwrap().collect(),
        matrix,
    );
    assert!(
        actual
            .chunks_exact(4)
            .filter(|p| p[..3] != [0, 0, 0])
            .count()
            > 50
    );
    assert_eq!(
        actual,
        pixels(&renderer, &device, &queue, vec![&reference], matrix)
    );
    assert_ne!(
        actual,
        pixels(
            &renderer,
            &device,
            &queue,
            render.geometries(second).unwrap().collect(),
            matrix
        )
    );
    let bytes = render.allocation_bytes();
    assert!(
        render
            .synchronize(&renderer, &device, &queue, requests(), 2, 0, 65536)
            .is_empty()
    );
    assert_eq!(render.allocation_bytes(), bytes);
    physics
        .fixed_step(&mut scene, &mut input, 1. / 60.)
        .unwrap();
    assert_eq!(
        matrix,
        Mat4::from_translation(Vec3::new(-0.1, 0., 0.5)) * scene.world_matrix(first).unwrap()
    );
    render.clear();
    assert_eq!(render.counts(), (0, 0, 0));
    assert_eq!(render.allocation_bytes(), 0);
    assert!(pollster::block_on(scope.pop()).is_none());
    println!(
        "VOXY_CURVED_ROOT_GPU composed={composed} accepted_angle={angle} cpu_pixels_equal=true double_rotation_differs=true sources=1 bytes={bytes} stop_bytes=0"
    );
}

#[test]
#[ignore = "requires a graphics adapter"]
fn planted_foot_gpu_matches_independent_locked_geometry_and_clears_resources() {
    planted_foot_gpu_acceptance(false);
}
#[test]
#[ignore = "requires a graphics adapter"]
fn retargeted_foot_gpu_matches_independent_locked_geometry_and_clears_resources() {
    planted_foot_gpu_acceptance(true);
}
fn planted_foot_gpu_acceptance(retarget: bool) {
    use glam::{Mat4, Vec3};
    use voxy_gameplay::{BoxCollider, CharacterBody, CharacterPhysics};
    let gpu = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(gpu.request_adapter(&Default::default())).unwrap();
    println!("VOXY_FOOT_GPU_ADAPTER {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut model = ModelAsset::parse(
        include_bytes!("../../../voxy_render/examples/assets/foot-contact.glb"),
        &[],
        voxy_render::ModelLimits::default(),
    )
    .unwrap();
    if retarget {
        model.animations.clear();
    }
    let model = Arc::new(model);
    let mut scene = voxy_scene::SceneGraph::new(4);
    let first = scene
        .spawn(
            None,
            voxy_scene::Transform {
                translation: Vec3::Y,
                ..Default::default()
            },
        )
        .unwrap();
    let second = scene.spawn(None, Default::default()).unwrap();
    let floor = scene
        .spawn(
            None,
            voxy_scene::Transform {
                translation: -Vec3::Y * 0.1,
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            floor,
            BoxCollider {
                half_extents: [4., 0.1, 4.],
            },
        )
        .unwrap();
    scene
        .insert_component(
            first,
            CharacterBody {
                half_extents: [0.1, 1., 0.1],
                ..Default::default()
            },
        )
        .unwrap();
    scene
        .insert_component(
            first,
            crate::ModelFootPlacement {
                feet: vec![crate::FootBinding {
                    bones: ["hip".into(), "knee".into(), "foot".into()],
                    sole_offset: [0., -0.1, 0.],
                    sole_up: [0., 1., 0.],
                    pole: [1., 0., 0.],
                    plant: true,
                    weight: 1.,
                    contact: Default::default(),
                    contact_curve: vec![],
                    clip_contact_curves: Default::default(),
                }],
            },
        )
        .unwrap();
    let asset = voxy_assets::AssetId("foot".into());
    for owner in [first, second] {
        scene
            .insert_component(
                owner,
                crate::ModelInstance {
                    asset: asset.clone(),
                },
            )
            .unwrap();
        scene
            .insert_component(
                owner,
                ModelAnimation {
                    clip: None,
                    ..Default::default()
                },
            )
            .unwrap();
    }
    let mut models = std::collections::BTreeMap::from([(asset, model.clone())]);
    if retarget {
        let glb = gltf::binary::Glb::from_slice(include_bytes!(
            "../../../voxy_render/examples/assets/foot-contact.glb"
        ))
        .unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&glb.json).unwrap();
        for (index, name) in ["sourceHip", "sourceKnee", "sourceFoot"].iter().enumerate() {
            json["nodes"][index]["name"] = serde_json::json!(name);
        }
        json["nodes"][0]["translation"] = serde_json::json!([0., 0.6, 0.]);
        let mut bin = glb.bin.unwrap().into_owned();
        let view = json["accessors"][5]["bufferView"].as_u64().unwrap() as usize;
        let offset = json["bufferViews"][view]["byteOffset"].as_u64().unwrap() as usize;
        for key in 0..2 {
            let at = offset + key * 12 + 4;
            let value =
                f32::from_le_bytes(bin[at..at + 4].try_into().unwrap()) + 0.1 + key as f32 * 0.05;
            bin[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        let bytes = gltf::binary::Glb {
            header: glb.header,
            json: serde_json::to_vec(&json).unwrap().into(),
            bin: Some(bin.into()),
        }
        .to_vec()
        .unwrap();
        let source =
            Arc::new(ModelAsset::parse(&bytes, &[], voxy_render::ModelLimits::default()).unwrap());
        models.insert(voxy_assets::AssetId("source".into()), source);
        scene
            .insert_component(
                first,
                crate::ModelRetarget {
                    source: "source".into(),
                    joints: [
                        ("sourceHip", "hip"),
                        ("sourceKnee", "knee"),
                        ("sourceFoot", "foot"),
                    ]
                    .into_iter()
                    .map(|(a, b)| crate::RetargetJointProfile {
                        source: a.into(),
                        target: b.into(),
                        rotation_basis: glam::Quat::IDENTITY.to_array(),
                        translation_basis: glam::Quat::IDENTITY.to_array(),
                        translation_scale: 1.,
                    })
                    .collect(),
                },
            )
            .unwrap();
        scene
            .insert_component(
                first,
                ModelAnimation {
                    clip_name: "move".into(),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    let mut runtime = crate::animation_runtime::AnimationRuntime::default();
    let mut physics = CharacterPhysics::new(&scene, 1, 1);
    let mut input = voxy_gameplay::player_input().unwrap();
    for _ in 0..4 {
        let candidate = runtime.prepare(&scene, &models, 1. / 60.).unwrap();
        runtime = physics
            .fixed_step_with_preparation(
                &mut scene,
                &mut input,
                1. / 60.,
                &[(first, Vec3::X * 0.03)],
                &[],
                |preview, budget| candidate.clone().prepare_accepted_pose(preview, budget),
            )
            .unwrap()
            .1;
    }
    let frame = runtime.frame(first, &model).unwrap();
    if retarget {
        assert!((frame.pose.local()[0].translation.y - (0.5 + 0.05 * 4. / 60.)).abs() < 1e-6);
        assert!(model.animations.is_empty());
    }
    let authored = runtime.frame(second, &model).unwrap();

    let mut render = AnimatedModels::new(&renderer).unwrap();
    assert!(render.skinner.is_some());
    let requests = || {
        let mut a = request(first, &model, 1.);
        a.frame = Some(frame.clone());
        let mut b = request(second, &model, 1.);
        b.frame = Some(authored.clone());
        vec![a, b]
    };
    assert!(
        render
            .synchronize(&renderer, &device, &queue, requests(), 1, 0, 65536)
            .is_empty()
    );
    assert_eq!(render.counts(), (2, 2, 1));
    let source = |owner| match &render.owners[&owner].primitives[0] {
        Primitive::Skin { source, .. } => Arc::as_ptr(source),
        _ => panic!("GPU skin expected"),
    };
    assert_eq!(source(first), source(second));
    // Independent expected world points from authored triangle and locked x=.03.
    // No solved pose, skin palette or CPU skinning helper enters this reference.
    let mesh = voxy_render::SceneMesh::new(
        vec![
            voxy_render::SceneVertex {
                position: [-0.22, 0., 0.],
                uv: [0., 0.],
                color: model.primitives[0].color,
            },
            voxy_render::SceneVertex {
                position: [0.28, 0., 0.],
                uv: [0., 0.],
                color: model.primitives[0].color,
            },
            voxy_render::SceneVertex {
                position: [0.03, 0.4, 0.],
                uv: [0., 0.],
                color: model.primitives[0].color,
            },
        ],
        vec![0, 1, 2],
    )
    .unwrap();
    let reference = renderer.upload_mesh(&device, &mesh).unwrap();
    let view = Mat4::from_scale_rotation_translation(
        Vec3::splat(2.),
        glam::Quat::IDENTITY,
        Vec3::new(0., -0.4, 0.5),
    );
    let matrix = view * scene.world_matrix(first).unwrap();
    let actual = pixels(
        &renderer,
        &device,
        &queue,
        render.geometries(first).unwrap().collect(),
        matrix,
    );
    assert!(
        actual
            .chunks_exact(4)
            .filter(|p| p[..3] != [0, 0, 0])
            .count()
            > 100
    );
    assert!(
        actual == pixels(&renderer, &device, &queue, vec![&reference], view),
        "locked geometry pixels differ"
    );
    assert_ne!(
        actual,
        pixels(
            &renderer,
            &device,
            &queue,
            render.geometries(second).unwrap().collect(),
            matrix
        )
    );
    let bytes = render.allocation_bytes();
    assert!(
        render
            .synchronize(&renderer, &device, &queue, requests(), 2, 0, 65536)
            .is_empty()
    );
    assert_eq!(render.allocation_bytes(), bytes);
    render.clear();
    assert_eq!(render.allocation_bytes(), 0);
    assert_eq!(render.counts(), (0, 0, 0));
    assert!(pollster::block_on(scope.pop()).is_none());
    println!(
        "VOXY_FOOT_GPU retarget={retarget} cpu_pixels_equal=true uncorrected_differs=true sources=1 bytes={bytes} stop_bytes=0"
    );
}
