//! Actual GPU pixels for shared UI layout, overlays, resize and DPI scaling.
use glam::{Mat4, Vec2};
use voxy_render::{SceneDraw, SceneRenderer, Sprite, SpriteBatch};
use voxy_ui::{Axis, HitRegion, LayoutItem, Length, PointerRouter, WidgetId, layout_linear};
#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let material = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
    let transform = renderer.create_transform(&device, Mat4::IDENTITY)?;
    let items = [
        LayoutItem {
            id: WidgetId(1),
            length: Length::Fixed(20.0),
            enabled: true,
        },
        LayoutItem {
            id: WidgetId(2),
            length: Length::Flex(1.0),
            enabled: true,
        },
    ];
    for (logical_width, scale) in [(64_u32, 1_u32), (64, 2), (128, 1)] {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let viewport = Vec2::new(f32::from(u16::try_from(logical_width)?), 64.0);
        let mut regions = layout_linear(
            [0.0; 2],
            viewport.to_array(),
            Axis::Horizontal,
            4.0,
            4.0,
            &items,
            3,
        )?;
        let overlay = HitRegion {
            id: WidgetId(3),
            origin: [16.0; 2],
            size: [20.0, 16.0],
            enabled: false,
        };
        regions.push(
            overlay
                .clipped([20.0, 18.0], [12.0, 10.0])?
                .ok_or("empty overlay clip")?,
        );
        let mut batch = SpriteBatch::new(3);
        for (region, color) in regions.iter().zip([
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 1.0],
        ]) {
            batch.push(Sprite::from_logical_rect(
                Vec2::from_array(region.origin),
                Vec2::from_array(region.size),
                viewport,
                color,
            )?)?;
        }
        let geometry = renderer.upload_mesh(&device, &batch.mesh()?)?;
        let width = logical_width * scale;
        let height = 64 * scale;
        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let target = |format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("UI pixel target"),
                size: extent,
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
        let row_bytes = (width * 4).div_ceil(256) * 256;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI readback"),
            size: u64::from(row_bytes) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        renderer.encode(
            &mut encoder,
            &color.create_view(&wgpu::TextureViewDescriptor::default()),
            &depth.create_view(&wgpu::TextureViewDescriptor::default()),
            wgpu::Color::BLACK,
            &[SceneDraw {
                geometry: &geometry,
                texture: &material,
                transform: &transform,
                overlay: true,
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
                    bytes_per_row: Some(row_bytes),
                    rows_per_image: Some(height),
                },
            },
            extent,
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
        let pixel = |x: u32, y: u32| {
            let index = usize::try_from(y * scale * row_bytes + x * scale * 4).unwrap();
            &pixels[index..index + 4]
        };
        assert_eq!(pixel(2, 2), [0, 0, 0, 255]);
        assert_eq!(pixel(8, 8), [255, 0, 0, 255]);
        assert_eq!(pixel(40, 8), [0, 255, 0, 255]);
        assert_eq!(pixel(20, 20), [0, 0, 255, 255]);
        assert_eq!(pixel(18, 20), [255, 0, 0, 255], "outside left clip");
        assert_eq!(pixel(34, 20), [0, 255, 0, 255], "outside right clip");
        let mut pointer = PointerRouter::new(3);
        pointer.set_regions(&regions)?;
        pointer.move_to(Some([20.0; 2]));
        assert!(pointer.press().consumed);
        assert_eq!(pointer.captured(), None);
        let mut rgba = Vec::with_capacity(usize::try_from(width * height * 4)?);
        for row in pixels.chunks_exact(usize::try_from(row_bytes)?) {
            rgba.extend_from_slice(&row[..usize::try_from(width * 4)?]);
        }
        image::save_buffer(
            format!("/tmp/voxy-ui-{logical_width}-{scale}.png"),
            &rgba,
            width,
            height,
            image::ColorType::Rgba8,
        )?;
        drop(pixels);
        readback.unmap();
    }
    println!(
        "UI PIXELS PASS: GPU colors, top-left coordinates, painter overlay, matching disabled hit occlusion, resize and 2x DPI"
    );
    Ok(())
}
