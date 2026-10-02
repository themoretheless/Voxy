//! Imports a GLB, skins two poses, renders on a real GPU and checks pixel motion.
use glam::{Mat4, Vec3};
use voxy_render::{GraphicsOptions, ModelAsset, ModelLimits, SceneDraw, SceneRenderer};

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let asset = ModelAsset::parse(
        include_bytes!("assets/animated-triangle.glb"),
        &[],
        ModelLimits::default(),
    )?;
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Model smoke on {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let first = asset.scene_meshes(&asset.skeleton.bind_pose())?;
    let mut geometry = renderer.upload_mesh(&device, &first[0])?;
    let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let transform = renderer.create_transform(
        &device,
        Mat4::from_translation(Vec3::new(-0.6, -0.3, 0.5)) * Mat4::from_scale(Vec3::splat(0.5)),
    )?;
    let color = target(
        &device,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = target(
        &device,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("model readback"),
        size: 64 * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut centers = Vec::new();
    let mut frames = Vec::new();
    for time in [0., 0.5] {
        let pose = asset.animations[0].sample(&asset.skeleton, time);
        geometry.update(&queue, &asset.scene_meshes(&pose)?[0])?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        renderer.encode(
            &mut encoder,
            &color.create_view(&wgpu::TextureViewDescriptor::default()),
            &depth.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::Color::BLACK,
            &[SceneDraw {
                geometry: &geometry,
                texture: &texture,
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
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let pixels = readback.slice(..).get_mapped_range()?;
        let (mut count, mut sum) = (0u32, 0u32);
        for (index, pixel) in pixels.chunks_exact(4).enumerate() {
            if pixel[0] > 200 {
                count += 1;
                sum += u32::try_from(index % 64)?;
            }
        }
        assert!(count > 50, "animated triangle missing");
        centers.push(f64::from(sum) / f64::from(count));
        frames.push(pixels.to_vec());
        drop(pixels);
        readback.unmap();
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    assert!(
        (centers[1] - centers[0] - 16.).abs() < 1.,
        "unexpected motion: {centers:?}"
    );
    let mut pixels = Vec::new();
    for y in 0..64 {
        for frame in &frames {
            pixels.extend_from_slice(&frame[y * 256..(y + 1) * 256]);
        }
    }
    image::save_buffer(
        "/tmp/voxy-model-smoke.png",
        &pixels,
        128,
        64,
        image::ColorType::Rgba8,
    )?;
    println!(
        "PASS: imported GLB animation moved rendered triangle by 16 pixels; /tmp/voxy-model-smoke.png"
    );
    Ok(())
}
fn target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("model target"),
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
}
