//! Verify repeated updates to the same GPU transform through actual pixel readback.
use glam::{Mat4, Vec3};
use voxy_render::{GraphicsOptions, ObjAsset, ObjLimits, SceneDraw, SceneRenderer};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    render_frames()
}
#[allow(clippy::too_many_lines)]
fn render_frames() -> Result<(), Box<dyn std::error::Error>> {
    let instance = GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Transform readback on {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let transform =
        renderer.create_transform(&device, Mat4::from_translation(Vec3::new(0.0, 0.0, 0.5)))?;
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
        size: 256 * 1024,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut handle_counts = Vec::new();
    let mut amber_counts = Vec::new();
    let mut centers = Vec::new();
    let mut frames = Vec::new();
    let asset = ObjAsset::parse(
        include_str!("../../voxy_render/examples/assets/quad.obj"),
        ObjLimits {
            source_bytes: 4096,
            attributes: 64,
            vertices: 64,
            triangles: 64,
        },
    )?;
    let geometry = renderer.upload_mesh(&device, &asset.mesh)?;
    let gizmo = renderer.upload_mesh(&device, &voxy_editor::translation_gizmo()?)?;
    let outline = renderer.upload_mesh(&device, &voxy_editor::selection_outline(&asset.mesh)?)?;
    for x in [0.0, 0.5, 0.0] {
        for _ in 0..120 {
            transform.update(&queue, Mat4::from_translation(Vec3::new(x, 0.0, 0.5)))?;
            queue.submit([]);
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            renderer.encode(
                &mut encoder,
                &color.create_view(&wgpu::TextureViewDescriptor::default()),
                &depth.create_view(&wgpu::TextureViewDescriptor::default()),
                wgpu::Color::BLACK,
                &[
                    SceneDraw {
                        geometry: &geometry,
                        texture: &texture,
                        transform: &transform,
                        overlay: false,
                    },
                    SceneDraw {
                        geometry: &outline,
                        texture: &texture,
                        transform: &transform,
                        overlay: true,
                    },
                    SceneDraw {
                        geometry: &gizmo,
                        texture: &texture,
                        transform: &transform,
                        overlay: true,
                    },
                ],
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
                        bytes_per_row: Some(1024),
                        rows_per_image: Some(256),
                    },
                },
                color.size(),
            );
            queue.submit([encoder.finish()]);
        }
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
                sum += u32::try_from(index % 256)?;
            }
        }
        assert!(count > 50, "imported quad missing");
        centers.push(f64::from(sum) / f64::from(count));
        amber_counts.push(
            pixels
                .chunks_exact(4)
                .filter(|pixel| pixel[0] > 240 && pixel[1] > 100 && pixel[1] < 200 && pixel[2] < 20)
                .count(),
        );
        handle_counts.push([
            pixels
                .chunks_exact(4)
                .filter(|pixel| pixel[0] > 240 && pixel[1] < 60 && pixel[2] < 60)
                .count(),
            pixels
                .chunks_exact(4)
                .filter(|pixel| pixel[0] < 60 && pixel[1] > 240 && pixel[2] < 60)
                .count(),
            pixels
                .chunks_exact(4)
                .filter(|pixel| pixel[0] < 60 && pixel[1] < 100 && pixel[2] > 240)
                .count(),
        ]);
        frames.push(pixels.to_vec());
        drop(pixels);
        readback.unmap();
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    assert_eq!(
        frames[0], frames[2],
        "transform reset did not restore pixels"
    );
    assert!(
        (centers[1] - centers[0] - 64.0).abs() < 1.0,
        "GPU transform did not move pixels: {centers:?}"
    );
    assert!(
        amber_counts.iter().all(|count| *count > 20),
        "selection outline missing: {amber_counts:?}"
    );
    assert!(
        handle_counts.iter().flatten().all(|count| *count > 8),
        "gizmo handles missing: {handle_counts:?}"
    );
    println!("GPU GIZMO PASS: RGB handle pixels per pose={handle_counts:?}");
    println!("GPU SELECTION PASS: amber outline pixels per pose={amber_counts:?}");
    println!(
        "GPU TRANSFORM PASS (120 submissions per pose without CPU waits): same buffer update shifted centroid by 64 pixels and reset restored exact image; centers={centers:?}"
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
            width: 256,
            height: 256,
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
