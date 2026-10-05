//! Analytic ray-integral checks on a real GPU, including opaque clipping.
use glam::Vec3;
use voxy_render::*;
fn read_centre(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> f32 {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x: 64, y: 64, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let bytes = buffer.slice(..).get_mapped_range().unwrap();
    if texture.format() == wgpu::TextureFormat::Rg32Float {
        f32::from_le_bytes(bytes[..4].try_into().unwrap())
    } else {
        half::f16::from_bits(u16::from_le_bytes([bytes[0], bytes[1]])).to_f32()
    }
}
#[test]
fn sphere_path_scale_additivity_and_opaque_occlusion() {
    let instance = GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .expect("GPU required for optical validation");
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let scene = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 128,
            height: 128,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let camera = SceneCamera {
        eye: Vec3::new(0., 0., 3.),
        target: Vec3::ZERO,
        up: Vec3::Y,
        projection: SceneProjection::Perspective {
            vertical_fov: 45_f32.to_radians(),
            aspect: 1.,
            near: 0.1,
            far: 10.,
        },
    };
    let mut fluid = ScreenSpaceFluidRenderer::new_with_adapter(
        &device,
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        128,
        128,
        8,
    )
    .unwrap();
    let sphere = FluidRenderParticle {
        position_radius: [0., 0., 0., 0.4],
        absorption_ior: [0.1, 0.1, 0.1, 1.333],
    };
    let render = |fluid: &ScreenSpaceFluidRenderer, draws: &[SceneDraw<'_>]| {
        let mut encoder = device.create_command_encoder(&Default::default());
        fluid.encode(
            &scene,
            &mut encoder,
            &output.create_view(&Default::default()),
            wgpu::Color::BLACK,
            draws,
        );
        queue.submit([encoder.finish()]);
        read_centre(&device, &queue, fluid.thickness_texture())
    };
    for filter in [FluidDepthFilter::None, FluidDepthFilter::Bilateral] {
        fluid.update(&queue, camera, &[sphere], 2., filter).unwrap();
        let single = render(&fluid, &[]);
        assert!((single - 0.4).abs() < 0.003, "diameter in metres: {single}");
        fluid
            .update(&queue, camera, &[sphere, sphere], 2., filter)
            .unwrap();
        assert!((render(&fluid, &[]) - 2. * single).abs() < 0.003);
    }
    let texture = scene
        .upload_texture(&device, &queue, 1, 1, &[255; 4])
        .unwrap();
    let transform = scene
        .create_transform(&device, camera.view_projection().unwrap())
        .unwrap();
    let plane = |z| {
        SceneMesh::new(
            [[-1., -1., z], [1., -1., z], [1., 1., z], [-1., 1., z]]
                .map(|position| SceneVertex {
                    position,
                    uv: [0.; 2],
                    color: [1.; 4],
                })
                .to_vec(),
            vec![0, 1, 2, 0, 2, 3],
        )
        .unwrap()
    };
    fluid
        .update(&queue, camera, &[sphere], 2., FluidDepthFilter::Bilateral)
        .unwrap();
    let clip = scene.upload_mesh(&device, &plane(0.)).unwrap();
    let draws = [SceneDraw {
        geometry: &clip,
        texture: &texture,
        transform: &transform,
        overlay: false,
    }];
    let half = render(&fluid, &draws);
    assert!(
        (half - 0.2).abs() < 0.003,
        "partial opaque clipping: {half}"
    );
    let block = scene.upload_mesh(&device, &plane(1.)).unwrap();
    let draws = [SceneDraw {
        geometry: &block,
        texture: &texture,
        transform: &transform,
        overlay: false,
    }];
    assert_eq!(
        render(&fluid, &draws),
        0.0,
        "hidden liquid contributes no optical thickness"
    );
    let mut left = sphere;
    left.position_radius[0] = -0.7;
    let mut right = sphere;
    right.position_radius[0] = 0.7;
    fluid
        .update(
            &queue,
            camera,
            &[left, right],
            2.,
            FluidDepthFilter::Bilateral,
        )
        .unwrap();
    render(&fluid, &[]);
    assert_eq!(
        read_centre(&device, &queue, fluid.depth_radius_texture()),
        0.0,
        "filter must not bridge disconnected droplets"
    );
    fluid
        .update(&queue, camera, &[sphere], 2., FluidDepthFilter::Bilateral)
        .unwrap();
    let mut invalid = sphere;
    invalid.position_radius[3] = f32::NAN;
    assert!(
        fluid
            .update(&queue, camera, &[invalid], 2., FluidDepthFilter::None)
            .is_err()
    );
    assert!(
        (render(&fluid, &[]) - 0.4).abs() < 0.003,
        "rejected updates preserve prior snapshot"
    );
    fluid
        .update(&queue, camera, &[], 2., FluidDepthFilter::None)
        .unwrap();
    assert_eq!(render(&fluid, &[]), 0.0);
    assert!(
        ScreenSpaceFluidRenderer::new_with_adapter(
            &device,
            &adapter,
            wgpu::TextureFormat::Rgba8Unorm,
            0,
            128,
            8
        )
        .is_err()
    );
    assert!(pollster::block_on(scope.pop()).is_none());
}
