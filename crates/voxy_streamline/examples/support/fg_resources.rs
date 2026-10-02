//! GPU/lease preparation acceptance; no SDK tagging or generated frames.
pub fn verify(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<(), String> {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let result = verify_resources(device, queue);
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(format!(
            "FG scene GPU validation: {error}; readback result: {result:?}"
        ));
    }
    result?;
    println!(
        "FG SCENE RESOURCES PASS: 16 HDR channels, owned half resolve, distinct color/render extents, presentation/reset retained; no SDK generation"
    );
    Ok(())
}

#[allow(unsafe_code, clippy::too_many_lines)]
fn verify_resources(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<(), String> {
    use voxy_streamline::scene_dx12::SceneFrameGeneration;
    let create = |width, format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("FG scene resource probe"),
            size: wgpu::Extent3d {
                width,
                height: 1,
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
    let depth = create(
        2,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let motion = create(
        2,
        wgpu::TextureFormat::Rg16Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    );
    let color = create(
        4,
        wgpu::TextureFormat::Rgba32Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    );
    let pixels = [
        [0.25_f32, 1.0, 4.0, 0.5],
        [131008.0, -2.0, 0.0, 1.0],
        [0.0001, 0.1, 2.0, 0.25],
        [8.0, 16.0, 32.0, 0.75],
    ];
    let bytes: Vec<u8> = pixels
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    queue.write_texture(
        color.as_image_copy(),
        &bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(64),
            rows_per_image: Some(1),
        },
        color.size(),
    );
    queue.write_texture(
        motion.as_image_copy(),
        &[0; 8],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(8),
            rows_per_image: Some(1),
        },
        motion.size(),
    );
    let mut encoder = device.create_command_encoder(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    {
        let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("FG initialized opaque depth"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.5),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    queue.submit([encoder.finish()]);
    let pipeline = voxy_render::HdrHalfResolvePipeline::new(device)
        .map_err(|e| format!("FG resolve pipeline: {e:?}"))?;
    let frame = voxy_render::TemporalFrame {
        presentation_id: 17,
        reset_history: true,
        color: &color,
        depth: &depth,
        motion: &motion,
    };
    // SAFETY: All textures/pipeline share this DX12 device. Producers precede
    // preparation on this queue, owners remain live through completion, and
    // no SDK tags/evaluation or presentation is performed by this probe.
    let candidate =
        unsafe { SceneFrameGeneration::import_wide_radiance(&frame, &color, &pipeline) }
            .map_err(|e| format!("FG scene import: {e:?}"))?;
    if candidate.presentation_id() != 17
        || !candidate.reset_history()
        || candidate.hudless_color().size() != color.size()
        || candidate.hudless_color().format() != wgpu::TextureFormat::Rgba16Float
    {
        return Err("FG scene identity, reset or output extent/format changed".into());
    }
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("FG scene resolved color readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    candidate
        .prepare(&mut encoder)
        .map_err(|e| format!("FG prepare: {e:?}"))?;
    encoder.copy_texture_to_buffer(
        candidate.hudless_color().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        color.size(),
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
    if let Err(error) = device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(std::time::Duration::from_secs(5)),
    }) {
        // Completion is unproven; preserve the imported owners conservatively.
        std::mem::forget(candidate);
        return Err(format!("FG resource completion: {error}"));
    }
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let mapped = readback
        .slice(..)
        .get_mapped_range()
        .map_err(|e| e.to_string())?;
    for (index, input) in pixels.iter().flatten().enumerate() {
        let actual = half::f16::from_bits(u16::from_le_bytes([
            mapped[index * 2],
            mapped[index * 2 + 1],
        ]))
        .to_f32();
        let expected = if index % 4 == 3 {
            *input
        } else {
            input.clamp(0.0, 65504.0)
        };
        if !actual.is_finite() || (actual - expected).abs() > expected.abs() * 0.001 + 1e-6 {
            return Err(format!(
                "FG resolved channel {index}: {actual} != {expected}"
            ));
        }
    }
    drop(mapped);
    readback.unmap();
    Ok(())
}
