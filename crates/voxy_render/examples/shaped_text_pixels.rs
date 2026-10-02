//! GPU glyph alpha/baseline/atlas pixel proof with a runtime font fixture.
use glam::{Mat4, Vec2};
use voxy_render::{SceneDraw, SceneRenderer, Sprite, SpriteBatch};
use voxy_text::{FontLimits, RunDirection, RunOptions, TextFont};
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let path = std::env::args()
        .nth(1)
        .ok_or("pass a local TrueType font path")?;
    let font = TextFont::parse(
        &std::fs::read(path)?,
        FontLimits {
            max_font_bytes: 8 * 1024 * 1024,
            max_glyph_pixels: 4096,
            max_size: 64.0,
        },
    )?;
    let run = font.prepare(
        "AV é Жg",
        [8.0, 48.0],
        RunOptions {
            size: 32.0,
            direction: RunDirection::Guess,
            max_text_bytes: 128,
            max_glyphs: 32,
            atlas_size: [256, 64],
            max_atlas_pixels: 16384,
        },
    )?;
    let rgba: Vec<_> = run
        .atlas()
        .alpha()
        .iter()
        .flat_map(|alpha| [255, 255, 255, *alpha])
        .collect();
    let material = renderer.upload_texture(&device, &queue, 256, 64, &rgba)?;
    let transform = renderer.create_transform(&device, Mat4::IDENTITY)?;
    for (logical_width, scale) in [(256_u32, 1_u32), (256, 2)] {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let viewport = Vec2::new(f32::from(u16::try_from(logical_width)?), 64.0);
        let mut batch = SpriteBatch::new(run.glyphs().len());
        let coordinate = |value: usize| -> Result<f32, std::num::TryFromIntError> {
            Ok(f32::from(u16::try_from(value)?))
        };
        for glyph in run.glyphs() {
            let region = glyph.region;
            let mut sprite = Sprite::from_logical_rect(
                Vec2::from_array(glyph.origin),
                Vec2::new(coordinate(region.size[0])?, coordinate(region.size[1])?),
                viewport,
                [1.0; 4],
            )?;
            sprite.uv_min = Vec2::new(
                coordinate(region.origin[0])? / 256.0,
                coordinate(region.origin[1])? / 64.0,
            );
            sprite.uv_max = Vec2::new(
                coordinate(region.origin[0] + region.size[0])? / 256.0,
                coordinate(region.origin[1] + region.size[1])? / 64.0,
            );
            batch.push(sprite)?;
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
        let bright = pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[0] > 128 && pixel[1] > 128 && pixel[2] > 128)
            .count();
        assert!(bright > 500 * usize::try_from(scale * scale)?);
        assert_eq!(pixel(240, 40), [0, 0, 0, 255]);
        let mut rgba = Vec::with_capacity(usize::try_from(width * height * 4)?);
        for row in pixels.chunks_exact(usize::try_from(row_bytes)?) {
            rgba.extend_from_slice(&row[..usize::try_from(width * 4)?]);
        }
        image::save_buffer(
            format!("/tmp/voxy-shaped-text-{scale}.png"),
            &rgba,
            width,
            height,
            image::ColorType::Rgba8,
        )?;
        drop(pixels);
        readback.unmap();
    }
    println!(
        "SHAPED TEXT GPU PASS: immutable prepared run renders kerning/accent/Cyrillic/descender at 1x/2x"
    );
    Ok(())
}
