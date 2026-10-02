//! Browser event-loop acceptance of retained HDR resolve and depth rejection.
use super::browser::{error, yield_browser};
use wasm_bindgen::JsValue;

pub(crate) async fn validate(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<u32, JsValue> {
    let texture = |format, bytes: &[u8], stride| {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("browser temporal acceptance"),
            size: wgpu::Extent3d {
                width: 4,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(1),
            },
            texture.size(),
        );
        texture
    };
    let floats = |values: &[f32]| {
        values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>()
    };
    let color = |value| {
        texture(
            wgpu::TextureFormat::Rgba32Float,
            &floats(
                &(0..4)
                    .flat_map(|_| [value, value, value, 1.0])
                    .collect::<Vec<_>>(),
            ),
            64,
        )
    };
    let bright = color(10.0);
    let current = color(2.0);
    let motion = texture(wgpu::TextureFormat::Rg16Float, &[0; 16], 16);
    let depth = texture(wgpu::TextureFormat::R32Float, &floats(&[0.5; 4]), 16);
    let wrong = texture(wgpu::TextureFormat::R32Float, &floats(&[0.75; 4]), 16);
    let resolver = voxy_render::TemporalResolve::new(device).map_err(error)?;
    let mut history = voxy_render::TemporalHistory::new(device, 4, 1).map_err(error)?;
    let mut checked = 0;
    for (index, expected) in [10.0_f32, 6.0, 2.0, 2.0, 2.0].into_iter().enumerate() {
        if index == 4 {
            history.reset();
        }
        let frame = history
            .prepare_resolve(
                &resolver,
                if index == 0 { &bright } else { &current },
                &motion,
                if index == 3 { &wrong } else { &depth },
                voxy_render::TemporalResolveOptions {
                    history_weight: 0.5,
                    depth_tolerance: 0.001,
                    reset_history: false,
                },
                index == 2,
            )
            .map_err(error)?;
        let copy = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("browser temporal result"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        history.encode_depth(&mut encoder, &depth).map_err(error)?;
        frame.encode(&mut encoder);
        encoder.copy_texture_to_buffer(
            frame.output().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &copy,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            frame.output().size(),
        );
        let dispatch =
            voxy_render::ComputeDispatch::copy_buffer(device, &mut encoder, &copy, 0, 256)
                .map_err(error)?;
        queue.submit([encoder.finish()]);
        let mut pending = dispatch.begin_read();
        let deadline = js_sys::Date::now() + 30_000.0;
        let bytes = loop {
            if let Some(bytes) = pending.try_read().map_err(error)? {
                break bytes;
            }
            if js_sys::Date::now() >= deadline {
                return Err(error("temporal readback timed out"));
            }
            yield_browser().await?;
        };
        for pixel in 0..4 {
            for channel in 0..4 {
                let offset = pixel * 16 + channel * 4;
                let value =
                    f32::from_le_bytes(bytes[offset..offset + 4].try_into().map_err(error)?);
                let reference = if channel == 3 { 1.0 } else { expected };
                if !value.is_finite() || (value - reference).abs() > 0.00001 {
                    return Err(error(format!(
                        "temporal case {index} pixel {pixel} channel {channel}: {value} != {reference}"
                    )));
                }
                checked += 1;
            }
        }
        // Controlled caller-confirmed commit: this fixture has no surface presentation.
        // Keep intermediate candidates uncommitted to test retained input.
        if index == 0 || index == 4 {
            history.presented();
        }
    }
    Ok(checked)
}
