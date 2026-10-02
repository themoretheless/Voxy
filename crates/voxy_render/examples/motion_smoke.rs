//! Real GPU motion-vector readback in `RGBA16Float`.
use glam::{Mat4, Vec3};
use voxy_render::{GraphicsOptions, MotionMatrices, SceneDraw, SceneMesh, SceneRenderer};

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = match std::env::var("VOXY_MOTION_BACKEND").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("auto") => voxy_render::GraphicsBackend::Auto,
        Ok("metal") => voxy_render::GraphicsBackend::Metal,
        Ok("vulkan") => voxy_render::GraphicsBackend::Vulkan,
        Ok("gl") => voxy_render::GraphicsBackend::OpenGl,
        Ok("dx12") => voxy_render::GraphicsBackend::DirectX12,
        _ => return Err("VOXY_MOTION_BACKEND expects auto|metal|vulkan|gl|dx12".into()),
    };
    let instance = GraphicsOptions {
        backend,
        ..Default::default()
    }
    .create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Motion GPU: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba16Float);
    pollster::block_on(renderer.reload_shader(&device, voxy_render::MOTION_SCENE_SHADER))?;
    let mesh = renderer.upload_mesh(&device, &SceneMesh::quad([1.0; 4]))?;
    let image = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let transform = renderer.create_transform(&device, Mat4::IDENTITY)?;
    let target = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("motion smoke target"),
            size: wgpu::Extent3d {
                width: 32,
                height: 32,
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
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("motion readback"),
        size: 32 * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let moved = Mat4::from_translation(Vec3::new(0.25, 0.25, 0.0));
    let perspective =
        glam::camera::rh::proj::directx::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 10.0);
    let previous_perspective = perspective * Mat4::from_translation(Vec3::new(0.0, 0.0, -2.0));
    let current_perspective = perspective * Mat4::from_translation(Vec3::new(0.25, 0.25, -2.0));
    let behind_camera = perspective * Mat4::from_translation(Vec3::Z);
    for (current, previous, valid, expected) in [
        (Mat4::IDENTITY, Mat4::IDENTITY, true, [0_u16, 0, 0, 0x3c00]),
        (moved, Mat4::IDENTITY, true, [0xb000, 0x3000, 0, 0x3c00]),
        (moved, Mat4::IDENTITY, false, [0, 0, 0, 0x3c00]),
        (
            current_perspective,
            previous_perspective,
            true,
            [0xac00, 0x2c00, 0, 0x3c00],
        ),
        (
            current_perspective,
            current_perspective,
            true,
            [0, 0, 0, 0x3c00],
        ),
        (current_perspective, behind_camera, true, [0, 0, 0, 0x3c00]),
    ] {
        transform.update_motion(
            &queue,
            MotionMatrices {
                current,
                previous,
                history_valid: valid,
            },
        )?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        renderer.encode(
            &mut encoder,
            &color.create_view(&wgpu::TextureViewDescriptor::default()),
            &depth.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::Color::TRANSPARENT,
            &[SceneDraw {
                geometry: &mesh,
                texture: &image,
                transform: &transform,
                overlay: false,
            }],
        );
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
                    bytes_per_row: Some(256),
                    rows_per_image: Some(32),
                },
            },
            color.size(),
        );
        queue.submit([encoder.finish()]);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        receiver.recv()??;
        let pixels = buffer.slice(..).get_mapped_range()?;
        let offset = 16 * 256 + 16 * 8;
        for (channel, expected) in expected.into_iter().enumerate() {
            let index = offset + channel * 2;
            let actual = u16::from_le_bytes([pixels[index], pixels[index + 1]]);
            // IEEE half signed zero is equivalent to positive zero.
            assert!(
                actual == expected || (expected == 0 && actual == 0x8000),
                "motion channel {channel}: got {actual:#x}, expected {expected:#x}"
            );
        }
        drop(pixels);
        buffer.unmap();
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    println!(
        "PASS: orthographic/perspective motion, stationary/reset zero, previous-behind-camera zero"
    );
    Ok(())
}
