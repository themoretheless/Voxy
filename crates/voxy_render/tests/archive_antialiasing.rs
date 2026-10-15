//! Validate the complete HDR/sky/AA shader, including native Metal translation.
fn source() -> String {
    concat!(
        include_str!("../src/display_transform.wgsl"),
        include_str!("../src/archive_environment.wgsl"),
        include_str!("../src/archive_post.wgsl")
    )
    .into()
}

#[test]
fn archive_antialiasing_shader_is_valid() {
    let module = naga::front::wgsl::parse_str(&source()).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn archive_antialiasing_compiles_on_metal_sdr_and_hdr() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    let (device, _) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("archive spatial AA Metal regression"),
        source: wgpu::ShaderSource::Wgsl(source().into()),
    });
    for format in [
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba16Float,
    ] {
        let _pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("archive spatial AA SDR/HDR"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[cfg(target_os = "macos")]
#[test]
fn archive_antialiasing_filters_silhouettes_on_metal() {
    use glam::{Mat4, Vec3};
    use voxy_render::archive_scene::*;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let vertices = [[-0.8, -0.7, 0.], [0.9, -0.6, 0.], [-0.35, 0.9, 0.]]
        .map(|position| ArchiveVertex {
            position,
            normal: [0., 0., 1.],
            tangent: [1., 0., 0., 1.],
            uv: [0., 0.],
        })
        .to_vec();
    let mut renderer = ArchiveSceneRenderer::new_with_images(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba8Unorm,
        64,
        64,
        vec![],
        vec![ArchiveMaterialSource {
            parameters: ArchiveMaterial {
                diffuse: [0., 0., 0., 1.],
                specular_gloss: [0.; 4],
                emissive_normal: [0.; 4],
                surface: [0., 1., 0., -1.],
                workflow: [1., 0., 0., 0.],
            },
            images: [None; 4],
            double_sided: true,
        }],
        vec![ArchiveMeshSource {
            vertices,
            indices: vec![0, 1, 2],
            instances: vec![ArchiveInstance::new(Mat4::IDENTITY)],
            material: 0,
            center: Vec3::ZERO,
        }],
        &[],
        Vec3::new(0.3, 0.9, 0.1).normalize(),
        [1.; 4],
        (Vec3::splat(-1.), Vec3::splat(1.)),
    )
    .unwrap();
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("AA diagonal silhouette"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let mut images = Vec::new();
    for mode in [
        voxy_render::ArchiveAntialiasing::Off,
        voxy_render::ArchiveAntialiasing::Spatial,
        voxy_render::ArchiveAntialiasing::Temporal,
        voxy_render::ArchiveAntialiasing::MetalFx,
    ] {
        eprintln!("Metal polygon AA mode {mode:?}");
        renderer.set_antialiasing(mode).unwrap();
        // Updating display headroom must preserve the selected AA state.
        renderer.set_display_headroom(1.).unwrap();
        for frame in 0..16 {
            renderer.draw(
                &output.create_view(&Default::default()),
                Vec3::new(frame as f32 * 0.001, 0., 2.),
                -Vec3::Z,
            );
        }
        if let Some(motion) = renderer.temporal_motion_texture().cloned() {
            renderer.draw(
                &output.create_view(&Default::default()),
                Vec3::new(0.015, 0., 2.),
                -Vec3::Z,
            );
            let data = read_64x64(&device, &queue, &motion);
            assert!(
                data.chunks_exact(2)
                    .all(|p| half::f16::from_bits(u16::from_le_bytes([p[0], p[1]]))
                        .to_f32()
                        .abs()
                        < 0.0001),
                "static-camera motion must exclude jitter in {mode:?}"
            );
        }
        if mode == voxy_render::ArchiveAntialiasing::Temporal {
            for _ in 0..16 {
                renderer.draw(
                    &output.create_view(&Default::default()),
                    Vec3::new(0.015, 0., 2.),
                    -Vec3::Z,
                );
            }
            let mut previous = read_64x64(&device, &queue, &output);
            let mut change = 0_u64;
            for _ in 0..8 {
                renderer.draw(
                    &output.create_view(&Default::default()),
                    Vec3::new(0.015, 0., 2.),
                    -Vec3::Z,
                );
                let current = read_64x64(&device, &queue, &output);
                change += current
                    .iter()
                    .zip(&previous)
                    .enumerate()
                    .filter(|(i, _)| i % 4 != 3)
                    .map(|(_, (&a, &b))| u64::from(a.abs_diff(b)))
                    .sum::<u64>();
                previous = current;
            }
            let mean = change as f64 / (64. * 64. * 3. * 8.);
            assert!(
                mean < 0.2,
                "static TAA must stabilize despite jitter: mean delta {mean}"
            );
        }
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 64 * 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(64),
                },
            },
            output.size(),
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).unwrap();
        });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().unwrap();
        images.push(readback.slice(..).get_mapped_range().unwrap().to_vec());
        readback.unmap();
    }
    let changed = images[0]
        .chunks_exact(4)
        .zip(images[1].chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        changed > 20,
        "AA must filter the diagonal silhouette: {changed}"
    );
    assert!(
        changed < 1024,
        "AA must preserve most interior pixels: {changed}"
    );
    assert!(
        images
            .iter()
            .all(|image| image.chunks_exact(4).all(|p| p[3] == 255))
    );
    for image in &images[2..] {
        assert!(
            image.chunks_exact(4).any(|p| p[0] > 20),
            "temporal output must contain the sky"
        );
        assert!(
            image.chunks_exact(4).any(|p| p[0] < 10),
            "temporal output must contain the polygon"
        );
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
fn polygon_temporal_shader_and_jitter_are_valid() {
    let module =
        naga::front::wgsl::parse_str(concat!(include_str!("../src/display_transform.wgsl"), include_str!("../src/archive_temporal.wgsl"))).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
    let points: Vec<_> = (0..8).map(voxy_render::halton_jitter).collect();
    assert!(points.iter().all(|p| p.x.abs() <= 0.5 && p.y.abs() <= 0.5));
    assert_eq!(voxy_render::halton_jitter(0), voxy_render::halton_jitter(8));
    assert_ne!(points[0], points[1]);
}

#[cfg(target_os = "macos")]
fn read_64x64(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 64 * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(64),
            },
        },
        texture.size(),
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let result = buffer.slice(..).get_mapped_range().unwrap().to_vec();
    buffer.unmap();
    result
}

#[cfg(test)]
mod temporal_coordinates {
    use glam::{Mat4, Vec2, Vec3};
    use voxy_render::halton_jitter;
    fn uv(vp: Mat4, point: glam::Vec4) -> Vec2 {
        let p = vp * point;
        p.truncate().truncate() / p.w * Vec2::new(0.5, -0.5) + Vec2::splat(0.5)
    }
    #[test]
    fn backward_uv_motion_has_correct_translation_sign_and_sky_invariance() {
        let projection =
            glam::camera::rh::proj::directx::perspective(65_f32.to_radians(), 1., 0.05, 1000.);
        let previous =
            projection * glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
        let current = projection
            * glam::camera::rh::view::look_at_mat4(
                Vec3::X * 0.1,
                Vec3::X * 0.1 + Vec3::NEG_Z,
                Vec3::Y,
            );
        let point = glam::Vec4::new(0., 0., -2., 1.);
        let delta = uv(previous, point) - uv(current, point);
        assert!(delta.x > 0.);
        assert!(delta.y.abs() < 1e-6);
        let sky = glam::Vec4::new(0.2, 0.4, -1., 0.);
        assert!((uv(previous, sky) - uv(current, sky)).length() < 1e-6);
    }
    #[test]
    fn jitter_moves_raster_pixels_in_top_left_coordinates() {
        let p = glam::Vec4::new(0., 0., 0.5, 1.);
        let jitter = halton_jitter(2);
        let size = Vec2::new(1920., 1080.);
        let shifted = Mat4::from_translation(Vec3::new(
            2. * jitter.x / size.x,
            -2. * jitter.y / size.y,
            0.,
        ));
        let actual = (uv(shifted, p) - uv(Mat4::IDENTITY, p)) * size;
        assert!((actual - jitter).length() < 0.0001);
    }
}

#[test]
fn temporal_guide_formats_have_exact_memory_admission() {
    let (device, _) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    let budget = voxy_render::ComputeMemoryBudget::configure(&device, 88).unwrap();
    let create = |format| {
        budget.allocate_texture(&wgpu::TextureDescriptor {
            label: Some("temporal guide memory regression"),
            size: wgpu::Extent3d {
                width: 4,
                height: 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 3,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    };
    let motion = create(wgpu::TextureFormat::Rg16Float).unwrap();
    let depth = create(wgpu::TextureFormat::R32Float).unwrap();
    assert_eq!(motion.allocation_bytes(), 44);
    assert_eq!(depth.allocation_bytes(), 44);
    assert_eq!(budget.stats().allocated_bytes, 88);
    assert!(matches!(
        create(wgpu::TextureFormat::R32Float),
        Err(voxy_render::ComputeError::MemoryBudget)
    ));
    drop((motion, depth));
    budget.discard_retired().unwrap();
    assert_eq!(budget.stats().allocated_bytes, 0);
}
