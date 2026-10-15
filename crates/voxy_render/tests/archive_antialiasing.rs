//! Validate the complete HDR/sky/AA shader, including native Metal translation.
fn source() -> String {
    concat!(
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
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    let (device, _) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("archive spatial AA Metal regression"),
        source: wgpu::ShaderSource::Wgsl(source().into()),
    });
    for format in [wgpu::TextureFormat::Bgra8UnormSrgb, wgpu::TextureFormat::Rgba16Float] {
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
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let vertices = [[-0.8, -0.7, 0.], [0.9, -0.6, 0.], [-0.35, 0.9, 0.]]
        .map(|position| ArchiveVertex {
            position, normal: [0., 0., 1.], tangent: [1., 0., 0., 1.], uv: [0., 0.],
        }).to_vec();
    let mut renderer = ArchiveSceneRenderer::new_with_images(
        &device, &queue, wgpu::TextureFormat::Rgba8Unorm, 64, 64, vec![],
        vec![ArchiveMaterialSource {
            parameters: ArchiveMaterial {
                diffuse: [0., 0., 0., 1.], specular_gloss: [0.; 4],
                emissive_normal: [0.; 4], surface: [0., 1., 0., -1.], workflow: [1., 0., 0., 0.],
            }, images: [None; 4], double_sided: true,
        }],
        vec![ArchiveMeshSource { vertices, indices: vec![0, 1, 2],
            instances: vec![ArchiveInstance::new(Mat4::IDENTITY)], material: 0, center: Vec3::ZERO }],
        &[], Vec3::Y, [1.; 4], (Vec3::splat(-1.), Vec3::splat(1.)),
    ).unwrap();
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("AA diagonal silhouette"),
        size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let mut images = Vec::new();
    for enabled in [false, true] {
        renderer.set_spatial_antialiasing(enabled);
        // Updating display headroom must preserve the selected AA state.
        renderer.set_display_headroom(1.).unwrap();
        renderer.draw(&output.create_view(&Default::default()), Vec3::new(0., 0., 2.), -Vec3::Z);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None, size: 64 * 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            output.as_image_copy(),
            wgpu::TexelCopyBufferInfo { buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(256), rows_per_image: Some(64) } },
            output.size(),
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| { tx.send(r).unwrap(); });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        rx.recv().unwrap().unwrap();
        images.push(readback.slice(..).get_mapped_range().unwrap().to_vec());
        readback.unmap();
    }
    let changed = images[0].chunks_exact(4).zip(images[1].chunks_exact(4))
        .filter(|(a,b)| a != b).count();
    assert!(changed > 20, "AA must filter the diagonal silhouette: {changed}");
    assert!(changed < 1024, "AA must preserve most interior pixels: {changed}");
    assert!(images[1].chunks_exact(4).all(|p| p[3] == 255));
    assert!(pollster::block_on(scope.pop()).is_none());
}
