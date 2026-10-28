// Test-only frame producers; the production renderer receives accepted frames.
use crate::EditorError;
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
    fn new(renderer: &SceneRenderer) -> Result<Self, EditorError> {
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
    ) -> Vec<(NodeId, EditorError)> {
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
            voxy_model::ModelLimits::default(),
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
        publication: None,
        settings: ModelAnimation {
            speed,
            ..ModelAnimation::default()
        },
    }
}

#[test]
#[ignore = "requires real GPU; retained CPU texture publication and pixel readback"]
fn retained_texture_publications_survive_reload_pose_failure_and_source_deletion() {
    use voxy_assets::{
        AssetCatalog, AssetId, FileInputs, ImportInputs, SourceDependencies, SourcePath,
    };
    let root = std::env::temp_dir().join(format!("voxy-retained-texture-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("mesh.glb"),
        include_bytes!("../../../voxy_render/examples/assets/animated-triangle.glb"),
    )
    .unwrap();
    let red = [220, 20, 20, 255];
    let green = [20, 220, 20, 255];
    std::fs::write(root.join("texture.rgba"), red).unwrap();
    let provider = FileInputs::new(&root).unwrap();
    let mut inputs = ImportInputs::new(4, 4 * 1024 * 1024);
    let mut asset = crate::import::load(
        &SourcePath::new("mesh.glb").unwrap(),
        &provider,
        &mut inputs,
    )
    .unwrap();
    let observed = inputs
        .read(AssetId("texture.rgba".into()), |id, max| {
            provider.read(id, max)
        })
        .unwrap();
    asset.images = vec![
        voxy_render::ImageAsset::from_rgba(1, 1, observed.bytes.to_vec(), Default::default())
            .unwrap(),
    ];
    Arc::make_mut(asset.animated.as_mut().unwrap()).primitives[0].base_color_texture =
        Some(voxy_model::ModelTexture {
            image: 0,
            sampling: Default::default(),
            use_mips: true,
        });
    let asset = inputs
        .finish(asset, |id, max| provider.read(id, max))
        .unwrap();
    let id = AssetId("model".into());
    let mut catalog = AssetCatalog::new(4, 1).unwrap();
    let mut sources = SourceDependencies::new(4, 16);
    let ticket = catalog.request(id.clone()).unwrap();
    catalog
        .complete_observed(&mut sources, &ticket, Ok(asset))
        .unwrap();
    let old = catalog.snapshot(&id).unwrap();
    let weak = Arc::downgrade(&old);
    let model = old.value().animated.as_ref().unwrap().clone();
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("RETAINED TEXTURE GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut cache = crate::gpu_model::ResidencyCache::default();
    let mut old_gpu =
        crate::ModelGraphics::upload_on(&renderer, &device, &queue, old.value(), &mut cache)
            .unwrap();
    old_gpu.publication = Some(old.clone());
    let mut scene = voxy_scene::SceneGraph::new(4);
    let owner = scene.spawn(None, Default::default()).unwrap();
    let frame = test_frame(&model, ModelAnimation::default(), 0.1);
    let make_request = |gpu: &crate::ModelGraphics, frame| Request {
        owner,
        publication: gpu.publication.clone(),
        model: model.clone(),
        settings: Default::default(),
        frame: Some(frame),
        lod: None,
        textures: gpu.animated_textures.clone(),
        texture_storage: gpu._images.clone(),
    };
    let mut render = AnimatedModels::new(&renderer).unwrap();
    assert!(
        render
            .synchronize(
                &renderer,
                &device,
                &queue,
                vec![make_request(&old_gpu, frame.clone())],
                1,
                old_gpu.geometry_bytes(),
                64 << 20
            )
            .is_empty()
    );
    let matrix = glam::Mat4::from_scale(glam::Vec3::splat(0.7));
    let before = pixels_with_texture(
        &renderer,
        &device,
        &queue,
        render.geometries(owner).unwrap().collect(),
        matrix,
        render.texture(owner, 0),
    );
    assert!(before.chunks_exact(4).any(|p| p[0] > p[1]));
    std::fs::write(root.join("texture.rgba"), green).unwrap();
    let mut replacement = old.value().clone();
    replacement.images[0] =
        voxy_render::ImageAsset::from_rgba(1, 1, green.to_vec(), Default::default()).unwrap();
    let mut inputs = ImportInputs::new(4, 4 * 1024 * 1024);
    for source in old.inputs().observations().keys() {
        inputs
            .read(source.clone(), |id, max| provider.read(id, max))
            .unwrap();
    }
    let replacement = inputs
        .finish(replacement, |id, max| provider.read(id, max))
        .unwrap();
    let ticket = catalog.request(id.clone()).unwrap();
    catalog
        .complete_observed(&mut sources, &ticket, Ok(replacement))
        .unwrap();
    let current = catalog.snapshot(&id).unwrap();
    let mut current_gpu =
        crate::ModelGraphics::upload_on(&renderer, &device, &queue, current.value(), &mut cache)
            .unwrap();
    current_gpu.publication = Some(current.clone());
    let green_pixels = pixels_with_texture(
        &renderer,
        &device,
        &queue,
        render.geometries(owner).unwrap().collect(),
        matrix,
        current_gpu.animated_textures[0].as_deref(),
    );
    assert_ne!(before, green_pixels);
    let mut invalid = frame.as_ref().clone();
    invalid.skin_matrices[0] = glam::Mat4::ZERO;
    assert_eq!(
        render
            .synchronize(
                &renderer,
                &device,
                &queue,
                vec![make_request(&current_gpu, Arc::new(invalid))],
                2,
                current_gpu.geometry_bytes(),
                64 << 20
            )
            .len(),
        1
    );
    drop(old_gpu);
    drop(old);
    assert!(weak.upgrade().is_some());
    assert_eq!(
        before,
        pixels_with_texture(
            &renderer,
            &device,
            &queue,
            render.geometries(owner).unwrap().collect(),
            matrix,
            render.texture(owner, 0)
        )
    );
    let live = crate::gpu_source_bindings::LiveSources {
        models: vec![(id.clone(), current_gpu.publication.clone().unwrap())],
        owners: render.source_publication_owners().unwrap(),
    };
    let bindings = crate::gpu_source_bindings::GpuSourceBindings::capture(
        &live,
        |node| (node == owner).then(|| voxy_scene::ObjectId("actor".into())),
        4,
    )
    .unwrap();
    let accepted = render
        .checkpoint_accepted_frames(
            |node| (node == owner).then(|| voxy_scene::ObjectId("actor".into())),
            4 * 1024 * 1024,
        )
        .unwrap();
    assert!(
        render
            .checkpoint_accepted_frames(|_| None, 4 * 1024 * 1024)
            .is_err()
    );
    assert!(
        render
            .checkpoint_accepted_frames(|_| Some(voxy_scene::ObjectId("actor".into())), 1,)
            .is_err()
    );
    let retained = live.publications();
    drop(live);
    std::fs::remove_dir_all(&root).unwrap();
    let limits = crate::import::catalog_checkpoint::Limits {
        assets: Default::default(),
        inputs: voxy_assets::ImportedArchiveLimits {
            max_inputs: 4,
            max_input_bytes: 4 * 1024 * 1024,
            max_archive_bytes: 32 * 1024 * 1024,
        },
        max_bytes: 64 * 1024 * 1024,
        max_catalog_assets: 4,
        max_pending: 1,
        max_source_outputs: 4,
        max_source_edges: 16,
    };
    let bytes = crate::import::catalog_checkpoint::capture_with_retained(
        &catalog,
        &sources,
        &[],
        &[],
        &[],
        &retained,
        limits,
    )
    .unwrap();
    crate::import::asset_checkpoint::retain_evidence("retained-textures.voxyeic", &bytes);
    let file = std::env::temp_dir().join(format!(
        "voxy-retained-textures-{}.voxyeic",
        std::process::id()
    ));
    let package = serde_json::to_vec(&(bindings, accepted, &bytes)).unwrap();
    crate::import::asset_checkpoint::retain_evidence("retained-texture-roles.json", &package);
    std::fs::write(&file, &package).unwrap();
    let package = std::fs::read(&file).unwrap();
    let (bindings, accepted, read): (
        crate::gpu_source_bindings::GpuSourceBindings,
        checkpoint::AcceptedFrames,
        Vec<u8>,
    ) = serde_json::from_slice(&package).unwrap();
    std::fs::remove_file(file).unwrap();
    let loaded = crate::import::catalog_checkpoint::prepare(&read, limits).unwrap();
    assert!(Arc::ptr_eq(
        &loaded.retained[0],
        &loaded.catalog.snapshot(&id).unwrap()
    ));
    let mut target = voxy_scene::SceneGraph::new(4);
    let _unused = target.spawn(None, Default::default()).unwrap();
    let new_owner = target.spawn(None, Default::default()).unwrap();
    let roles = bindings
        .prepare(
            &target,
            |id| (id.0 == "actor").then_some(new_owner),
            &loaded.retained,
            4,
        )
        .unwrap();
    assert_eq!(roles.scene, target.identity());
    assert!(Arc::ptr_eq(
        &roles.models[&id],
        &loaded.catalog.snapshot(&id).unwrap()
    ));
    assert_eq!(loaded.retained[1].value().images[0].rgba(), red);
    assert_eq!(loaded.retained[0].value().images[0].rgba(), green);
    assert_eq!(
        &*loaded.retained[1].inputs().observations()[&AssetId("texture.rgba".into())]
            .as_ref()
            .unwrap()
            .bytes,
        red
    );
    let prepared_frames = accepted
        .prepare(
            &target,
            |id| (id.0 == "actor").then_some(new_owner),
            &roles,
            4 * 1024 * 1024,
        )
        .unwrap();
    assert!(
        accepted
            .prepare(&target, |_| None, &roles, 4 * 1024 * 1024)
            .is_err()
    );
    let foreign = voxy_scene::SceneGraph::new(4);
    assert!(
        accepted
            .prepare(&foreign, |_| Some(new_owner), &roles, 4 * 1024 * 1024)
            .is_err()
    );
    let accepted_state = &prepared_frames[&new_owner];
    assert_eq!(accepted_state.ticks, render.owners[&owner].ticks);
    assert_eq!(accepted_state.settings, render.owners[&owner].settings);
    assert_eq!(
        accepted_state.frame.skin_matrices,
        render.owners[&owner].frame.skin_matrices
    );
    let live_geometry = current_gpu.geometry_bytes() + render.allocation_bytes();
    let old_cache_state = cache.state();
    let old_cache_images = cache.live_bytes();
    let old_cache_budget = cache.geometry_budget;
    cache.geometry_budget = live_geometry;
    assert!(
        crate::gpu_model::batch::UploadTarget {
            renderer: &renderer,
            device: &device,
            queue: &queue,
            cache: &cache,
            live_geometry,
            replaced_geometry: live_geometry,
        }
        .stage_restore(&target, &roles, prepared_frames)
        .is_err()
    );
    cache.geometry_budget = old_cache_budget;
    assert_eq!(cache.state(), old_cache_state);
    assert_eq!(cache.live_bytes(), old_cache_images);
    let prepared_frames = accepted
        .prepare(
            &target,
            |id| (id.0 == "actor").then_some(new_owner),
            &roles,
            4 * 1024 * 1024,
        )
        .unwrap();
    let pending = crate::gpu_model::batch::UploadTarget {
        renderer: &renderer,
        device: &device,
        queue: &queue,
        cache: &cache,
        live_geometry,
        replaced_geometry: live_geometry,
    }
    .stage_restore(&target, &roles, prepared_frames)
    .unwrap();
    let restored_gpu = pollster::block_on(pending.validate()).unwrap();
    assert_eq!(restored_gpu.scene, target.identity());
    assert!(Arc::ptr_eq(
        restored_gpu.models[&id].publication.as_ref().unwrap(),
        &roles.models[&id]
    ));
    assert_eq!(cache.state(), old_cache_state);
    assert_eq!(cache.live_bytes(), old_cache_images);
    assert_eq!(
        restored_gpu.cache.geometry_live,
        restored_gpu
            .models
            .values()
            .map(crate::ModelGraphics::geometry_bytes)
            .sum::<u64>()
            + restored_gpu.animated.allocation_bytes()
    );
    let restored_render = &restored_gpu.animated;
    let restored_pixels = pixels_with_texture(
        &renderer,
        &device,
        &queue,
        restored_render.geometries(new_owner).unwrap().collect(),
        matrix,
        restored_render.texture(new_owner, 0),
    );
    assert_eq!(before, restored_pixels);
    for (name, bytes) in [
        ("texture-reference.rgba", &before),
        ("texture-current.rgba", &green_pixels),
        ("texture-restored.rgba", &restored_pixels),
    ] {
        crate::import::asset_checkpoint::retain_evidence(name, bytes);
    }
    assert!(
        render
            .synchronize(
                &renderer,
                &device,
                &queue,
                vec![make_request(&current_gpu, frame)],
                3,
                current_gpu.geometry_bytes(),
                64 << 20
            )
            .is_empty()
    );
    assert!(Arc::ptr_eq(
        &render.source_publications().unwrap()[0],
        &current
    ));
    assert_eq!(
        green_pixels,
        pixels_with_texture(
            &renderer,
            &device,
            &queue,
            render.geometries(owner).unwrap().collect(),
            matrix,
            render.texture(owner, 0)
        )
    );
    drop(retained);
    assert!(weak.upgrade().is_none());
    render.clear();
    assert!(weak.upgrade().is_none());
    assert!(pollster::block_on(scope.pop()).is_none());
}
pub(crate) fn pixels(
    renderer: &SceneRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    geometries: Vec<&SceneGeometry>,
    matrix: glam::Mat4,
) -> Vec<u8> {
    pixels_with_texture(renderer, device, queue, geometries, matrix, None)
}
fn pixels_with_texture(
    renderer: &SceneRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    geometries: Vec<&SceneGeometry>,
    matrix: glam::Mat4,
    material: Option<&voxy_render::SceneTexture>,
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
    let texture = material.unwrap_or(&texture);
    let transform = renderer.create_transform(device, matrix).unwrap();
    let draws: Vec<_> = geometries
        .into_iter()
        .map(|geometry| voxy_render::SceneDraw {
            geometry,
            texture,
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
        Primitive::Baked(_) => panic!("GPU skin expected"),
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
                None,
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
    model.primitives.push(voxy_model::ModelPrimitive {
        geometry: ModelGeometry::Static(bind[0].clone()),
        color: [1.; 4],
        base_color_texture: None,
    });
    let model = Arc::new(model);
    let mut runtime = FrameFixture {
        render: AnimatedModels {
            authoring_frames: Default::default(),
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
                source_to_variant: vec![witness, witness],
                variant_to_source: vec![witness],
            }],
        )
        .unwrap(),
    );
    model.primitives[0].geometry = ModelGeometry::Skinned(mesh);
    let model = Arc::new(model);
    let publication = Arc::new(
        voxy_assets::ImportInputs::new(0, 0)
            .finish(
                crate::import::EditorAsset {
                    mesh: model
                        .scene_meshes_frame(&model.bind_frame().unwrap())
                        .unwrap()[0]
                        .clone(),
                    lod: None,
                    animated: Some(model.clone()),
                    skinned_lod: Some(lod.clone()),
                    nodes: Vec::new(),
                    images: Vec::new(),
                },
                |_, _| unreachable!(),
            )
            .unwrap(),
    );
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
            publication: Some(publication.clone()),
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
        let accepted = owners
            .checkpoint_accepted_frames(
                |_| Some(voxy_scene::ObjectId("actor".into())),
                4 * 1024 * 1024,
            )
            .unwrap();
        let packet = serde_json::to_vec(&accepted).unwrap();
        let accepted: checkpoint::AcceptedFrames = serde_json::from_slice(&packet).unwrap();
        let asset_limits = crate::import::asset_checkpoint::EditorAssetCheckpointLimits::default();
        let input_limits = voxy_assets::ImportedArchiveLimits {
            max_inputs: 4,
            max_input_bytes: 4 * 1024 * 1024,
            max_archive_bytes: 64 * 1024 * 1024,
        };
        let source_bytes = crate::import::publication_checkpoint::capture(
            std::slice::from_ref(&publication),
            asset_limits,
            input_limits,
            64 * 1024 * 1024,
        )
        .unwrap();
        let loaded_publication = crate::import::publication_checkpoint::prepare(
            &source_bytes,
            asset_limits,
            input_limits,
            64 * 1024 * 1024,
        )
        .unwrap()
        .remove(0);
        crate::import::asset_checkpoint::retain_evidence(
            &format!(
                "accepted-lod-source-{}.voxyeip",
                if cpu { "cpu" } else { "gpu" }
            ),
            &source_bytes,
        );
        let roles = crate::gpu_source_bindings::PreparedBindings {
            scene: graph.identity(),
            models: Default::default(),
            owners: [(owner, loaded_publication.clone())].into(),
        };
        let loaded = accepted
            .prepare(&graph, |_| Some(owner), &roles, 4 * 1024 * 1024)
            .unwrap();
        let accepted_lod = loaded[&owner].lod.as_ref().unwrap();
        assert!(Arc::ptr_eq(
            &accepted_lod.source,
            loaded_publication.value().skinned_lod.as_ref().unwrap()
        ));
        assert!(!Arc::ptr_eq(&accepted_lod.source, &lod));
        assert_eq!(accepted_lod.history, [(0, 1), (1, 0)].into());
        assert_eq!(accepted_lod.resident, vec![1]);
        let proof = accepted_lod.prepared.as_ref().unwrap();
        assert_eq!(proof.model(), world);
        assert_eq!(proof.positions(), cached.positions());
        assert_eq!(proof.levels(), cached.levels());
        if !cpu {
            let cache = crate::gpu_model::ResidencyCache::default();
            let frames = accepted
                .prepare(&graph, |_| Some(owner), &roles, 4 * 1024 * 1024)
                .unwrap();
            let pending = crate::gpu_model::batch::UploadTarget {
                renderer: &renderer,
                device: &device,
                queue: &queue,
                cache: &cache,
                live_geometry: owners.allocation_bytes(),
                replaced_geometry: owners.allocation_bytes(),
            }
            .stage_restore(&graph, &roles, frames)
            .unwrap();
            let ready = pollster::block_on(pending.validate()).unwrap();
            assert!(ready.models.is_empty());
            assert_eq!(ready.cache.geometry_live, ready.animated.allocation_bytes());
            assert_eq!(
                ready.animated.owners[&owner].lod_history,
                owners.owners[&owner].lod_history
            );
            for view in [0, 1] {
                let original_pixels = pixels(
                    &renderer,
                    &device,
                    &queue,
                    owners.geometries_for_view(owner, view).unwrap().collect(),
                    glam::Mat4::IDENTITY,
                );
                let actual = pixels(
                    &renderer,
                    &device,
                    &queue,
                    ready
                        .animated
                        .geometries_for_view(owner, view)
                        .unwrap()
                        .collect(),
                    glam::Mat4::IDENTITY,
                );
                assert_eq!(original_pixels, actual);
                crate::import::asset_checkpoint::retain_evidence(
                    &format!("composed-lod-view-{view}.rgba"),
                    &actual,
                );
            }
        }
        let mut restored = AnimatedModels::new(&renderer).unwrap();
        if cpu {
            restored.skinner = None;
        }
        let saved_frame = &loaded[&owner];
        assert!(
            restored
                .synchronize(
                    &renderer,
                    &device,
                    &queue,
                    vec![Request {
                        owner,
                        publication: Some(loaded_publication.clone()),
                        model: saved_frame.model.clone(),
                        settings: saved_frame.settings.clone(),
                        frame: Some(saved_frame.frame.clone()),
                        lod: Some(accepted_lod.source.clone()),
                        textures: vec![],
                        texture_storage: vec![],
                    }],
                    saved_frame.ticks,
                    0,
                    8192
                )
                .is_empty()
        );
        let recover_lod = || {
            accepted
                .prepare(&graph, |_| Some(owner), &roles, 4 * 1024 * 1024)
                .unwrap()
                .remove(&owner)
                .unwrap()
                .lod
                .unwrap()
        };
        let base = restored.allocation_bytes();
        assert!(
            restored
                .restore_accepted_lod(&renderer, &device, owner, recover_lod(), 0, base)
                .is_err()
        );
        assert_eq!(restored.allocation_bytes(), base);
        assert!(restored.owners[&owner].lod_history.is_empty());
        restored
            .restore_accepted_lod(&renderer, &device, owner, recover_lod(), 0, 8192)
            .unwrap();
        assert_eq!(
            restored.owners[&owner].lod_history,
            owners.owners[&owner].lod_history
        );
        assert_eq!(
            restored.owners[&owner]
                .prepared_lod
                .as_ref()
                .unwrap()
                .model(),
            world
        );
        for view in [0, 1] {
            assert_eq!(
                restored.lod_selection(owner, view),
                owners.lod_selection(owner, view)
            );
            let original_pixels = pixels(
                &renderer,
                &device,
                &queue,
                owners.geometries_for_view(owner, view).unwrap().collect(),
                glam::Mat4::IDENTITY,
            );
            let restored_pixels = pixels(
                &renderer,
                &device,
                &queue,
                restored.geometries_for_view(owner, view).unwrap().collect(),
                glam::Mat4::IDENTITY,
            );
            assert_eq!(original_pixels, restored_pixels);
            crate::import::asset_checkpoint::retain_evidence(
                &format!(
                    "lod-{}-view-{view}-reference.rgba",
                    if cpu { "cpu" } else { "gpu" }
                ),
                &original_pixels,
            );
            crate::import::asset_checkpoint::retain_evidence(
                &format!(
                    "lod-{}-view-{view}-restored.rgba",
                    if cpu { "cpu" } else { "gpu" }
                ),
                &restored_pixels,
            );
        }
        crate::import::asset_checkpoint::retain_evidence(
            &format!("accepted-lod-{}.json", if cpu { "cpu" } else { "gpu" }),
            &packet,
        );
        let mut bad: serde_json::Value = serde_json::from_slice(&packet).unwrap();
        bad["owners"][0]["lod"]["resident"] = serde_json::json!([]);
        let bad: checkpoint::AcceptedFrames = serde_json::from_value(bad).unwrap();
        assert!(
            bad.prepare(&graph, |_| Some(owner), &roles, 4 * 1024 * 1024)
                .is_err()
        );
        let mut bad: serde_json::Value = serde_json::from_slice(&packet).unwrap();
        bad["owners"][0]["lod"]["prepared"]["positions"][0][0] = serde_json::json!(0);
        let bad: checkpoint::AcceptedFrames = serde_json::from_value(bad).unwrap();
        assert!(
            bad.prepare(&graph, |_| Some(owner), &roles, 4 * 1024 * 1024)
                .is_err()
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
    model.primitives[0].base_color_texture = Some(voxy_model::ModelTexture {
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
        std::ptr::from_ref(runtime.texture(owner, 0).unwrap()),
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
        std::ptr::from_ref(runtime.texture(owner, 0).unwrap()),
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
        Primitive::Baked(_) => panic!("GPU skin expected"),
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
            voxy_model::ModelLimits::default(),
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
        Primitive::Baked(_) => panic!("GPU skin expected"),
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
    planted_foot_gpu_acceptance(false, false);
}
#[test]
#[ignore = "requires a graphics adapter"]
fn retargeted_foot_gpu_matches_independent_locked_geometry_and_clears_resources() {
    planted_foot_gpu_acceptance(true, false);
}
#[test]
#[ignore = "requires a graphics adapter"]
fn authored_roll_foot_gpu_matches_independent_pivot_geometry_and_clears_resources() {
    planted_foot_gpu_acceptance(false, true);
}
fn planted_foot_gpu_acceptance(retarget: bool, roll: bool) {
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
        voxy_model::ModelLimits::default(),
    )
    .unwrap();
    if retarget {
        model.animations.clear();
    }
    if roll {
        let mut joints = model.skeleton.joints().to_vec();
        joints[2].bind_local.rotation = glam::Quat::from_rotation_x(0.3);
        model.skeleton = voxy_animation::Skeleton::new(joints).unwrap();
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
                    orientation: if roll {
                        crate::FootOrientation::Authored
                    } else {
                        crate::FootOrientation::AlignSupport
                    },
                    pole: [1., 0., 0.],
                    plant: true,
                    weight: 1.,
                    contact: Default::default(),
                    contact_curve: vec![],
                    clip_contact_curves: Default::default(),
                    clip_pivot_keys: Default::default(),
                    support_geometry: None,
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
            Arc::new(ModelAsset::parse(&bytes, &[], voxy_model::ModelLimits::default()).unwrap());
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
        Primitive::Baked(_) => panic!("GPU skin expected"),
    };
    assert_eq!(source(first), source(second));
    // Independent expected world points from authored triangle and locked x=.03.
    // In the roll case the initial sole pivot has z=-.1*sin(.3), and the
    // authored triangle rolls about it. No solved pose, palette or CPU skinning
    // helper enters this reference.
    let reference_point = |x: f32, y: f32| {
        if roll {
            [x, y * 0.3f32.cos(), (y - 0.1) * 0.3f32.sin()]
        } else {
            [x, y, 0.]
        }
    };
    let mesh = voxy_render::SceneMesh::new(
        vec![
            voxy_render::SceneVertex {
                position: reference_point(-0.22, 0.),
                uv: [0., 0.],
                color: model.primitives[0].color,
            },
            voxy_render::SceneVertex {
                position: reference_point(0.28, 0.),
                uv: [0., 0.],
                color: model.primitives[0].color,
            },
            voxy_render::SceneVertex {
                position: reference_point(0.03, 0.4),
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
        "VOXY_FOOT_GPU retarget={retarget} authored_roll={roll} cpu_pixels_equal=true uncorrected_differs=true sources=1 bytes={bytes} stop_bytes=0"
    );
}

#[test]
#[ignore = "requires physical GPU; ordinary owner morph updates, CPU fallback and last-good resource acceptance"]
fn gpu_morph_weight_only_updates_and_failed_replacement_retains_last_good()
-> Result<(), Box<dyn std::error::Error>> {
    morph_weight_only_owner_acceptance(false)
}

#[test]
#[ignore = "requires physical GPU; unskinned rigid morph node transforms, CPU parity and resource retention"]
fn gpu_rigid_morph_without_source_skin_uses_common_owner() -> Result<(), Box<dyn std::error::Error>>
{
    morph_weight_only_owner_acceptance(true)
}

fn morph_weight_only_owner_acceptance(rigid: bool) -> Result<(), Box<dyn std::error::Error>> {
    let temp = crate::locomotion_tests::Temp::new();
    let (_, source) = if rigid {
        crate::locomotion_tests::rigid_morph_project(&temp)?
    } else {
        crate::locomotion_tests::morph_project(&temp)?
    };
    if rigid {
        assert_eq!(gltf::Gltf::from_slice(&source)?.skins().count(), 0);
    }
    let model = Arc::new(ModelAsset::parse_animated(
        &source,
        &[],
        voxy_model::ModelLimits::default(),
    )?);
    let graphics = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(graphics.request_adapter(&Default::default()))?;
    eprintln!("ordinary morph owner adapter: {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut scene = voxy_scene::SceneGraph::new(1);
    let owner = scene.spawn(None, Default::default())?;
    let mut gpu = AnimatedModels::new(&renderer)?;
    let mut cpu = AnimatedModels::new(&renderer)?;
    cpu.skinner = None;
    let settings = ModelAnimation {
        clip: None,
        ..Default::default()
    };
    let first = Arc::new(model.bind_frame()?);
    let update = |render: &mut AnimatedModels,
                  model: Arc<ModelAsset>,
                  frame: Arc<voxy_animation::AnimatorFrame>,
                  tick,
                  budget| {
        render.update(
            &renderer,
            &device,
            &queue,
            owner,
            model,
            settings.clone(),
            Some(frame),
            None,
            Vec::new(),
            Vec::new(),
            None,
            tick,
            0,
            budget,
        )
    };
    update(&mut gpu, model.clone(), first.clone(), 1, 1 << 20)?;
    assert!(matches!(
        gpu.owners[&owner].primitives[0],
        Primitive::Skin { .. }
    ));
    let matrix = glam::Mat4::from_translation(glam::Vec3::new(-0.4, -0.4, 0.5))
        * glam::Mat4::from_scale(glam::Vec3::splat(0.5));
    let before = pixels(
        &renderer,
        &device,
        &queue,
        vec![gpu.owners[&owner].primitives[0].geometry()],
        matrix,
    );
    let mut changed = (*first).clone();
    changed.properties = voxy_animation::PropertySet::new(
        first
            .properties
            .entries()
            .iter()
            .enumerate()
            .map(|(i, (name, _))| {
                (
                    name.clone(),
                    voxy_animation::PropertyValue::Float([1.5, -0.25][i]),
                )
            })
            .collect(),
    )?;
    let changed = Arc::new(changed);
    assert_eq!(changed.pose, first.pose);
    update(&mut gpu, model.clone(), changed.clone(), 2, 1 << 20)?;
    update(&mut cpu, model.clone(), changed.clone(), 2, 1 << 20)?;
    let after = pixels(
        &renderer,
        &device,
        &queue,
        vec![gpu.owners[&owner].primitives[0].geometry()],
        matrix,
    );
    let reference = pixels(
        &renderer,
        &device,
        &queue,
        vec![cpu.owners[&owner].primitives[0].geometry()],
        matrix,
    );
    assert_ne!(before, after);
    assert_eq!(after, reference);
    assert_eq!(gpu.owners[&owner].frame.properties, changed.properties);
    let mut invalid = (*changed).clone();
    invalid.properties = Default::default();
    assert!(update(&mut gpu, model.clone(), Arc::new(invalid), 3, 1 << 20).is_err());
    let bytes = gpu.allocation_bytes();
    assert!(
        update(
            &mut gpu,
            Arc::new((*model).clone()),
            changed.clone(),
            3,
            bytes
        )
        .is_err()
    );
    assert_eq!(gpu.allocation_bytes(), bytes);
    assert!(Arc::ptr_eq(&gpu.owners[&owner].model, &model));
    assert!(Arc::ptr_eq(&gpu.owners[&owner].frame, &changed));
    let retained = pixels(
        &renderer,
        &device,
        &queue,
        vec![gpu.owners[&owner].primitives[0].geometry()],
        matrix,
    );
    assert_eq!(retained, after);
    gpu.clear();
    cpu.clear();
    assert_eq!(gpu.counts(), (0, 0, 0));
    assert!(pollster::block_on(scope.pop()).is_none());
    println!(
        "VOXY_MORPH_OWNER_GPU weight_only_updates=true cpu_pixels_equal=true last_good_retained=true stop_owners=0 fps_qualified=false"
    );
    if rigid {
        println!(
            "VOXY_RIGID_MORPH_OWNER_GPU source_skins=0 gpu=true mirrored_nonuniform_node=true cpu_pixels_equal=true last_good_retained=true fps_qualified=false"
        );
    }
    Ok(())
}

#[test]
#[ignore = "requires physical GPU and VOXY_MORPH_UPSTREAM_GLB; ordinary imported owner with all original primitives"]
fn gpu_original_morph_owner_updates_all_primitives() -> Result<(), Box<dyn std::error::Error>> {
    let source_path = std::env::var_os("VOXY_MORPH_UPSTREAM_GLB")
        .ok_or("set VOXY_MORPH_UPSTREAM_GLB to the pinned original asset")?;
    let bytes = std::fs::read(source_path)?;
    let temp = crate::locomotion_tests::Temp::new();
    let (path, _) = crate::locomotion_tests::morph_project(&temp)?;
    let root = path.parent().unwrap();
    std::fs::write(root.join("rig.glb"), &bytes)?;
    let scene_path = root.join("main.scene.json");
    let mut scene: serde_json::Value = serde_json::from_slice(&std::fs::read(&scene_path)?)?;
    scene["objects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|o| o["id"] == "visual")
        .unwrap()["components"]["editor.model-locomotion.v1"]["falling"] = "Jump".into();
    std::fs::write(scene_path, serde_json::to_vec(&scene)?)?;
    let mut app = crate::App::from_project(&path)?;
    app.start_standalone()?;
    let model = app
        .catalog
        .snapshot(&voxy_assets::AssetId("rig".into()))
        .unwrap()
        .value()
        .animated
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(
        (
            model.mesh_instance_count(),
            model.primitives.len(),
            model.morph_primitive_count()
        ),
        (14, 19, 3)
    );
    let owner = app.instances[1];
    let graphics = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(graphics.request_adapter(&Default::default()))?;
    eprintln!("ordinary original morph adapter: {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut render = AnimatedModels::new(&renderer)?;
    let settings = app
        .scene
        .component::<ModelAnimation>(owner)?
        .unwrap()
        .clone();
    let mut checked = 0;
    let mut maximum = 0f64;
    for tick in 1..=3 {
        app.advance_game(1. / 60.)?;
        let frame = app.play.animations.frame(owner, &model).unwrap();
        render.update(
            &renderer,
            &device,
            &queue,
            owner,
            model.clone(),
            settings.clone(),
            Some(frame.clone()),
            None,
            Vec::new(),
            Vec::new(),
            None,
            tick,
            0,
            64 << 20,
        )?;
        let state = &render.owners[&owner];
        assert_eq!(state.primitives.len(), 19);
        assert!(
            state
                .primitives
                .iter()
                .all(|p| matches!(p, Primitive::Skin { .. }))
        );
        let expected = model.scene_meshes_frame(&frame)?;
        let sizes = expected
            .iter()
            .map(|mesh| mesh.vertices().len() as u64 * 36)
            .collect::<Vec<_>>();
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ordinary original full owner readback"),
            size: sizes.iter().sum(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        let mut offset = 0;
        for (primitive, size) in state.primitives.iter().zip(&sizes) {
            encoder.copy_buffer_to_buffer(
                &primitive.geometry().vertices,
                0,
                &staging,
                offset,
                *size,
            );
            offset += size;
        }
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let mapped = staging.slice(..).get_mapped_range()?;
        let actual = mapped
            .chunks_exact(4)
            .map(|bytes| f32::from_le_bytes(bytes.try_into().expect("one scalar")))
            .collect::<Vec<_>>();
        let mut first = 0;
        for (mesh, size) in expected.iter().zip(&sizes) {
            let end = first + *size as usize / 4;
            for (gpu, cpu) in actual[first..end].chunks_exact(9).zip(mesh.vertices()) {
                for axis in 0..3 {
                    let error = (f64::from(gpu[axis]) - f64::from(cpu.position[axis])).abs();
                    assert!(error.is_finite() && error <= 1e-5);
                    maximum = maximum.max(error);
                }
                checked += 1;
            }
            first = end;
        }
        drop(mapped);
        staging.unmap();
    }
    assert_eq!(checked, 7214 * 3);
    render.clear();
    assert_eq!(render.counts(), (0, 0, 0));
    app.stop_workers()?;
    assert!(pollster::block_on(scope.pop()).is_none());
    println!(
        "VOXY_ORIGINAL_MORPH_OWNER_GPU {}",
        serde_json::json!({"primitives":19,"morph_primitives":3,"frames":3,"checked_vertices":checked,"max_position_error":maximum,"ordinary_import_and_playback":true,"all_primitives_gpu":true,"native_image_verified":false,"fps_qualified":false})
    );
    Ok(())
}

#[test]
fn authoring_override_frames_are_cached_per_owner_revision_and_settings()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = crate::locomotion_tests::Temp::new();
    let (_, source) = crate::locomotion_tests::morph_project(&temp)?;
    let glb = gltf::binary::Glb::from_slice(&source)?;
    let mut document: serde_json::Value = serde_json::from_slice(&glb.json)?;
    let node = document["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node.get("mesh").is_some())
        .unwrap();
    document["nodes"][node]["name"] = "EditableMorph".into();
    let source = gltf::binary::Glb {
        header: glb.header,
        json: serde_json::to_vec(&document)?.into(),
        bin: glb.bin,
    }
    .to_vec()?;
    let model = Arc::new(ModelAsset::parse_animated(
        &source,
        &[],
        voxy_model::ModelLimits::default(),
    )?);
    let mut render = AnimatedModels {
        authoring_frames: Default::default(),
        skinner: None,
        owners: HashMap::new(),
        sources: HashMap::new(),
    };
    let mut scene = voxy_scene::SceneGraph::new(2);
    let owner = scene.spawn(None, Default::default())?;
    let other = scene.spawn(None, Default::default())?;
    let mut settings = ModelAnimation {
        morph_weights: [("EditableMorph".into(), vec![1.25, -0.75])].into(),
        ..Default::default()
    };
    let first = render.authoring_frame(owner, &model, &settings)?;
    assert_eq!(model.frame_weights(&first)?, vec![vec![1.25, -0.75]]);
    assert_eq!(first.skin_matrices, model.bind_frame()?.skin_matrices);
    assert!(Arc::ptr_eq(
        &first,
        &render.authoring_frame(owner, &model, &settings)?
    ));
    assert!(!Arc::ptr_eq(
        &first,
        &render.authoring_frame(other, &model, &settings)?
    ));
    settings
        .morph_weights
        .insert("EditableMorph".into(), vec![0.25, 0.5]);
    let edited = render.authoring_frame(owner, &model, &settings)?;
    assert!(!Arc::ptr_eq(&first, &edited));
    assert_eq!(model.frame_weights(&edited)?, vec![vec![0.25, 0.5]]);
    settings
        .morph_weights
        .insert("EditableMorph".into(), vec![f32::NAN, 0.]);
    assert!(render.authoring_frame(owner, &model, &settings).is_err());
    assert!(Arc::ptr_eq(
        &render.authoring_frames.borrow()[&owner].frame,
        &edited
    ));
    settings
        .morph_weights
        .insert("EditableMorph".into(), vec![0.25, 0.5]);
    let revision = Arc::new((*model).clone());
    assert!(!Arc::ptr_eq(
        &edited,
        &render.authoring_frame(owner, &revision, &settings)?
    ));
    render.clear();
    assert!(render.authoring_frames.borrow().is_empty());
    Ok(())
}
