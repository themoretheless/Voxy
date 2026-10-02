//! GPU glyph alpha/baseline/atlas pixel proof with a runtime font fixture.
use glam::{Mat4, Vec2};
use voxy_render::{SceneDraw, SceneRenderer, Sprite, SpriteBatch};
use voxy_text::{FontLimits, GlyphAtlas, RasterFont};
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
    let font = RasterFont::parse(
        &std::fs::read(path)?,
        FontLimits {
            max_font_bytes: 8 * 1024 * 1024,
            max_glyph_pixels: 4096,
            max_size: 64.0,
        },
    )?;
    let mut atlas = GlyphAtlas::new(128, 64, 8192, 8)?;
    let mut glyphs = Vec::new();
    let mut pen = 8_i32;
    for character in ['A', 'Ж', 'g'] {
        let glyph = font.rasterize(character, 32.0)?;
        let region = atlas.insert(&glyph)?.ok_or("empty visible glyph")?;
        let origin = [
            usize::try_from(pen + glyph.bearing[0])?,
            usize::try_from(48 - glyph.bearing[1] - i32::try_from(glyph.height)?)?,
        ];
        pen += glyph.advance.ceil() as i32 + 4;
        glyphs.push((glyph, region, origin));
    }
    let rgba: Vec<_> = atlas
        .alpha()
        .iter()
        .flat_map(|alpha| [255, 255, 255, *alpha])
        .collect();
    let material = renderer.upload_texture(&device, &queue, 128, 64, &rgba)?;
    let transform = renderer.create_transform(&device, Mat4::IDENTITY)?;
    for (logical_width, scale) in [(128_u32, 1_u32), (128, 2)] {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let viewport = Vec2::new(f32::from(u16::try_from(logical_width)?), 64.0);
        let mut batch = SpriteBatch::new(3);
        let coordinate = |value: usize| -> Result<f32, std::num::TryFromIntError> {
            Ok(f32::from(u16::try_from(value)?))
        };
        for (glyph, region, origin) in &glyphs {
            let mut sprite = Sprite::from_logical_rect(
                Vec2::new(coordinate(origin[0])?, coordinate(origin[1])?),
                Vec2::new(coordinate(glyph.width)?, coordinate(glyph.height)?),
                viewport,
                [1.0; 4],
            )?;
            sprite.uv_min = Vec2::new(
                coordinate(region.origin[0])? / 128.0,
                coordinate(region.origin[1])? / 64.0,
            );
            sprite.uv_max = Vec2::new(
                coordinate(region.origin[0] + region.size[0])? / 128.0,
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
        let mut checked = 0;
        for (glyph, _, origin) in &glyphs {
            for y in 0..glyph.height {
                for x in 0..glyph.width {
                    let actual =
                        pixel(u32::try_from(origin[0] + x)?, u32::try_from(origin[1] + y)?);
                    let alpha = glyph.alpha[y * glyph.width + x];
                    assert!(
                        actual[..3]
                            .iter()
                            .all(|channel| channel.abs_diff(alpha) <= 1),
                        "glyph coverage mismatch at {x},{y}: {actual:?}, alpha={alpha}"
                    );
                    assert_eq!(actual[3], 255);
                    checked += 1;
                }
            }
        }
        assert!(checked > 1000);
        let mut rgba = Vec::with_capacity(usize::try_from(width * height * 4)?);
        for row in pixels.chunks_exact(usize::try_from(row_bytes)?) {
            rgba.extend_from_slice(&row[..usize::try_from(width * 4)?]);
        }
        image::save_buffer(
            format!("/tmp/voxy-text-{scale}.png"),
            &rgba,
            width,
            height,
            image::ColorType::Rgba8,
        )?;
        drop(pixels);
        readback.unmap();
    }
    println!(
        "TEXT PIXELS PASS: Latin/Cyrillic/descender baseline, atlas UVs and alpha coverage verified on GPU at 1x/2x"
    );
    Ok(())
}
