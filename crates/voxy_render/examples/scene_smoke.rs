//! Real GPU pixel evidence: geometry, depth rejection, texture sampling and overlay blending.
use glam::{Mat4, Vec2, Vec3};
use voxy_render::{
    DEFAULT_SCENE_SHADER, GraphicsOptions, SceneCamera, SceneDraw, SceneMesh, SceneProjection,
    SceneRenderer,
};
use voxy_render::{Sprite, SpriteBatch};
use voxy_scene::{SceneGraph, Transform};

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = match std::env::var("VOXY_SCENE_BACKEND").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("auto") => voxy_render::GraphicsBackend::Auto,
        Ok("vulkan") => voxy_render::GraphicsBackend::Vulkan,
        Ok("gl") => voxy_render::GraphicsBackend::OpenGl,
        Ok("metal") => voxy_render::GraphicsBackend::Metal,
        Ok("dx12") => voxy_render::GraphicsBackend::DirectX12,
        _ => return Err("VOXY_SCENE_BACKEND expects auto|vulkan|gl|metal|dx12".into()),
    };
    let instance = GraphicsOptions {
        backend,
        ..Default::default()
    }
    .create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Rendering on {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let msaa = std::env::args().any(|arg| arg == "--msaa");
    let mut renderer = if msaa {
        SceneRenderer::new_msaa4(&device, wgpu::TextureFormat::Rgba8Unorm)
    } else {
        SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm)
    };
    let imported = voxy_render::ObjAsset::parse(
        include_str!("assets/quad.obj"),
        voxy_render::ObjLimits::default(),
    )?;
    let mut vertices = imported.mesh.vertices().to_vec();
    for vertex in &mut vertices {
        vertex.color = [1.0, 0.0, 0.0, 1.0];
        vertex.uv = [1.25, 0.5];
    }
    let mut near = renderer.upload_mesh(&device, &imported.mesh)?;
    near.update(
        &queue,
        &SceneMesh::new(vertices, imported.mesh.indices().to_vec())?,
    )?;
    let too_large = SceneMesh::new(vec![imported.mesh.vertices()[0]; 5], vec![0, 1, 2])?;
    assert_eq!(
        near.update(&queue, &too_large),
        Err(voxy_render::SceneError::GeometryCapacityExceeded)
    );
    let far = renderer.upload_mesh(&device, &SceneMesh::quad([0.0, 0.0, 1.0, 1.0]))?;
    let mut sprites = SpriteBatch::new(2);
    sprites.push(Sprite {
        color: [0.0, 1.0, 0.0, 0.5],
        ..Default::default()
    })?;
    let mut overlay = renderer.reserve_geometry(&device, 8, 12)?;
    overlay.update(&queue, &sprites.mesh()?)?;
    assert_eq!(overlay.index_count(), 6);
    overlay.clear();
    assert_eq!(overlay.index_count(), 0);
    sprites.push(Sprite {
        center: Vec2::new(2.4, 0.0),
        size: Vec2::splat(0.5),
        color: [0.0, 0.0, 1.0, 1.0],
        ..Default::default()
    })?;
    overlay.update(&queue, &sprites.mesh()?)?;
    assert_eq!(overlay.index_count(), 12);
    let image = voxy_render::ImageAsset::decode(
        include_bytes!("assets/tint.png"),
        voxy_render::ImageLimits::default(),
    )?;
    let material = renderer.upload_image(
        &device,
        &queue,
        &image,
        voxy_render::TextureSampling::default(),
    )?;
    let repeat_material = renderer.upload_texture_with_sampling(
        &device,
        &queue,
        2,
        1,
        &[255, 128, 255, 255, 0, 0, 0, 255],
        voxy_render::TextureSampling {
            wrap_u: voxy_render::TextureWrap::Repeat,
            ..Default::default()
        },
    )?;
    let perspective = SceneCamera {
        eye: Vec3::ZERO,
        target: Vec3::NEG_Z,
        up: Vec3::Y,
        projection: SceneProjection::Perspective {
            vertical_fov: std::f32::consts::FRAC_PI_2,
            aspect: 1.0,
            near: 0.1,
            far: 10.0,
        },
    }
    .view_projection()?;
    let orthographic = SceneCamera {
        eye: Vec3::Z,
        target: Vec3::ZERO,
        up: Vec3::Y,
        projection: SceneProjection::Orthographic {
            left: -1.0,
            right: 1.0,
            bottom: -1.0,
            top: 1.0,
            near: 0.0,
            far: 2.0,
        },
    }
    .view_projection()?;
    let mut scene = SceneGraph::new(3);
    let root = scene.spawn(
        None,
        Transform {
            translation: Vec3::new(0.0, 0.0, -1.0),
            ..Default::default()
        },
    )?;
    let near_node = scene.spawn(
        Some(root),
        Transform {
            translation: Vec3::new(0.0, 0.0, -1.0),
            ..Default::default()
        },
    )?;
    let far_node = scene.spawn(
        Some(root),
        Transform {
            translation: Vec3::new(0.0, 0.0, -3.0),
            ..Default::default()
        },
    )?;
    let near_transform =
        renderer.create_transform(&device, perspective * scene.world_matrix(near_node)?)?;
    let far_transform =
        renderer.create_transform(&device, perspective * scene.world_matrix(far_node)?)?;
    let overlay_transform = renderer.create_transform(
        &device,
        orthographic
            * Mat4::from_scale_rotation_translation(
                Vec3::new(0.25, 0.25, 1.0),
                glam::Quat::IDENTITY,
                Vec3::new(0.0, 0.0, 0.9),
            ),
    )?;
    let color = target(
        &device,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        1,
    );
    let depth = target(
        &device,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        1,
    );
    let multisample = msaa.then(|| {
        (
            target(
                &device,
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
                4,
            ),
            target(
                &device,
                wgpu::TextureFormat::Depth32Float,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
                4,
            ),
        )
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scene pixel readback"),
        size: 64 * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let shader_test = std::env::args().any(|arg| arg == "--shader-test");
    if shader_test {
        let (other_device, other_queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
        let other_scope = other_device.push_error_scope(wgpu::ErrorFilter::Validation);
        assert!(
            pollster::block_on(renderer.reload_shader(&other_device, DEFAULT_SCENE_SHADER))
                .is_err()
        );
        assert_eq!(renderer.shader_revision(), 0);
        assert!(matches!(
            renderer.upload_mesh(&other_device, &imported.mesh),
            Err(voxy_render::SceneError::DeviceMismatch)
        ));
        assert!(matches!(
            renderer.reserve_geometry(&other_device, 3, 3),
            Err(voxy_render::SceneError::DeviceMismatch)
        ));
        assert!(matches!(
            renderer.create_transform(&other_device, Mat4::IDENTITY),
            Err(voxy_render::SceneError::DeviceMismatch)
        ));
        assert!(matches!(
            renderer.upload_texture(&other_device, &other_queue, 1, 1, &[255; 4]),
            Err(voxy_render::SceneError::DeviceMismatch)
        ));
        assert!(pollster::block_on(other_scope.pop()).is_none());
        // Resources were created before reload: their bind groups must survive.
        assert!(!pollster::block_on(
            renderer.reload_shader(&device, DEFAULT_SCENE_SHADER)
        )?);
        let changed = DEFAULT_SCENE_SHADER.replace(
            "return textureSample(image, image_sampler, in.uv) * in.color;",
            "return (textureSample(image, image_sampler, in.uv) * in.color).bgra;",
        );
        assert!(pollster::block_on(
            renderer.reload_shader(&device, &changed)
        )?);
        assert_eq!(renderer.shader_revision(), 1);
        assert!(!pollster::block_on(
            renderer.reload_shader(&device, &changed)
        )?);
        for invalid in [
            "not valid WGSL".to_owned(),
            changed.replace("fn fs_main", "fn missing_fragment"),
            changed.replace("@group(1) @binding(0)", "@group(2) @binding(0)"),
        ] {
            let error = pollster::block_on(renderer.reload_shader(&device, &invalid))
                .expect_err("invalid source or ABI must not be installed");
            assert!(!error.to_string().is_empty());
            assert_eq!(renderer.shader_revision(), 1);
        }
        println!(
            "PASS: shader replacement, unchanged-source cache and rejection of syntax/entrypoint/binding errors"
        );
    }
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let draws = [
        SceneDraw {
            geometry: &overlay,
            texture: &material,
            transform: &overlay_transform,
            overlay: true,
        },
        SceneDraw {
            geometry: &near,
            texture: &repeat_material,
            transform: &near_transform,
            overlay: false,
        },
        SceneDraw {
            geometry: &far,
            texture: &material,
            transform: &far_transform,
            overlay: false,
        },
    ];
    let output = color.create_view(&wgpu::TextureViewDescriptor::default());
    if let Some((ms_color, ms_depth)) = &multisample {
        renderer.encode_msaa4(
            &mut encoder,
            &ms_color.create_view(&wgpu::TextureViewDescriptor::default()),
            &ms_depth.create_view(&wgpu::TextureViewDescriptor::default()),
            &output,
            wgpu::Color::BLACK,
            &draws,
        )?;
    } else {
        renderer.encode(
            &mut encoder,
            &output,
            &depth.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::Color::BLACK,
            &draws,
        );
    }
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
                bytes_per_row: Some(256),
                rows_per_image: Some(64),
            },
        },
        color.size(),
    );
    queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    receiver.recv()??;
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    let pixels = readback.slice(..).get_mapped_range()?;
    let pixel = |x: usize, y: usize| &pixels[y * 256 + x * 4..y * 256 + x * 4 + 4];
    assert_eq!(pixel(2, 2), [0, 0, 0, 255], "clear color");
    assert_eq!(
        pixel(25, 32),
        if shader_test {
            [0, 0, 255, 255]
        } else {
            [255, 0, 0, 255]
        },
        "near mesh must occlude later far mesh"
    );
    assert_eq!(
        pixel(51, 32),
        if shader_test {
            [255, 0, 0, 255]
        } else {
            [0, 0, 255, 255]
        },
        "second sprite in shared batch"
    );
    let center = pixel(32, 32);
    assert!(
        center[if shader_test { 2 } else { 0 }].abs_diff(128) <= 1
            && center[1].abs_diff(28) <= 1
            && center[if shader_test { 0 } else { 2 }] == 0
            && center[3] == 255,
        "overlay blend: {center:?}"
    );
    let path = std::env::args()
        .skip(1)
        .find(|arg| arg != "--shader-test" && arg != "--msaa")
        .unwrap_or_else(|| {
            if shader_test {
                "/tmp/voxy-shader-smoke.ppm".into()
            } else {
                "/tmp/voxy-scene-smoke.ppm".into()
            }
        });
    let mut ppm = b"P6\n64 64\n255\n".to_vec();
    for texel in pixels.chunks_exact(4) {
        ppm.extend_from_slice(&texel[..3]);
    }
    std::fs::write(&path, ppm)?;
    drop(pixels);
    readback.unmap();
    println!("PASS: clear, indexed mesh, depth occlusion, textured RGBA and overlay alpha; {path}");
    Ok(())
}

fn target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
    sample_count: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene smoke target"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}
