#![allow(unsafe_code)]
use voxy_streamline::{
    CameraConstants, DlssQuality, DlssRenderSize, PerspectiveFrame, StreamlineRuntime,
    dx12::{SuperResolutionResources, WgpuQueue},
};

pub fn evaluate(
    runtime: &mut StreamlineRuntime,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), String> {
    for (first_index, quality, width, height, hdr) in [
        (0, DlssQuality::Quality, 1024, 768, false),
        (3, DlssQuality::Quality, 800, 600, false),
        (6, DlssQuality::Dlaa, 800, 600, false),
        (9, DlssQuality::Quality, 800, 600, true),
    ] {
        evaluate_configuration(
            runtime,
            device,
            queue,
            quality,
            DlssRenderSize { width, height },
            first_index,
            hdr,
        )?;
    }
    Ok(())
}

fn evaluate_configuration(
    runtime: &mut StreamlineRuntime,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    quality: DlssQuality,
    output_size: DlssRenderSize,
    first_index: u32,
    hdr: bool,
) -> Result<(), String> {
    let input = runtime
        .configure_dlss(0, quality, output_size, hdr)
        .map_err(|error| format!("DLSS configure: {error:?}"))?;
    if input.width == 0 || input.height == 0 {
        return Err("SDK returned empty DLSS input size".into());
    }
    if quality == DlssQuality::Dlaa && input != output_size {
        return Err(format!(
            "DLAA input size differs from output: {input:?} vs {output_size:?}"
        ));
    }
    let format = if hdr {
        wgpu::TextureFormat::Rgba16Float
    } else {
        wgpu::TextureFormat::Rgba8Unorm
    };
    let color = texture(device, input, format, false);
    let depth = texture(device, input, wgpu::TextureFormat::Depth32Float, false);
    let motion = texture(device, input, wgpu::TextureFormat::Rgba16Float, false);
    let output = texture(device, output_size, format, true);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    clear_color(
        &mut encoder,
        &color,
        wgpu::Color {
            r: if hdr { 4.0 } else { 0.2 },
            g: if hdr { 1.0 } else { 0.4 },
            b: if hdr { 0.5 } else { 0.6 },
            a: 1.0,
        },
    );
    clear_color(&mut encoder, &motion, wgpu::Color::TRANSPARENT);
    clear_color(&mut encoder, &output, wgpu::Color::TRANSPARENT);
    {
        let view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("DLSS probe depth"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.5),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
    }
    queue.submit([encoder.finish()]);
    // SAFETY: Device remains live; access to its native queue is serialized.
    let native = unsafe { WgpuQueue::from_wgpu(device) }
        .map_err(|error| format!("DLSS queue: {error:?}"))?;
    for index in first_index..first_index + 3 {
        // SAFETY: Same live registered device; preceding native/readback work completed.
        let resources =
            unsafe { SuperResolutionResources::import(&color, &depth, &motion, &output) }
                .map_err(|error| format!("DLSS import: {error:?}"))?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        // Erase the preceding output, so a skipped SDK write cannot pass by reusing it.
        clear_color(&mut encoder, &output, wgpu::Color::TRANSPARENT);
        resources
            .prepare(&mut encoder)
            .map_err(|error| format!("DLSS prepare: {error:?}"))?;
        queue.submit([encoder.finish()]);
        evaluate_frame(runtime, &native, resources, index)?;
        verify_output(device, queue, &output)?;
        println!(
            "DLSS SR frame {index}: RGB readback passed, reset={}",
            index % 3 != 1
        );
    }
    println!(
        "DLSS {quality:?} HDR={hdr} reset/history/reset sequence passed: {}x{} -> {}x{}",
        input.width, input.height, output_size.width, output_size.height
    );
    Ok(())
}

fn evaluate_frame(
    runtime: &mut StreamlineRuntime,
    native: &WgpuQueue,
    resources: SuperResolutionResources,
    index: u32,
) -> Result<(), String> {
    let projection = glam::camera::rh::proj::directx::perspective(1.0, 4.0 / 3.0, 0.1, 100.0);
    let constants = CameraConstants::from_perspective(&PerspectiveFrame {
        projection,
        view: glam::Mat4::IDENTITY,
        previous_view_projection: (index % 3 == 1).then_some(projection),
        near_plane: 0.1,
        far_plane: 100.0,
        fov: 1.0,
        aspect: 4.0 / 3.0,
        jitter_pixels: [0.0; 2],
        motion_scale: [1.0; 2],
        reset: index % 3 != 1,
    })
    .map_err(|error| format!("DLSS camera: {error:?}"))?;
    let mut recorder = native
        .recorder()
        .map_err(|error| format!("DLSS recorder: {error:?}"))?;
    let mut frame = runtime
        .begin_frame(Some(index))
        .map_err(|error| format!("DLSS token: {error:?}"))?;
    // SAFETY: Prepared textures, current/history camera and viewport match this SDK frame.
    if let Err(error) = unsafe { resources.evaluate(&mut recorder, &mut frame, 0, &constants) } {
        std::mem::forget(recorder);
        std::mem::forget(resources);
        return Err(format!("DLSS evaluate: {error:?}"));
    }
    // SAFETY: Streamline manages tagged resource states; output returns to tagged UAV state.
    unsafe { recorder.uav_barrier(resources.textures().output) };
    let commands = match recorder.finish() {
        Ok(commands) => commands,
        Err(error) => {
            std::mem::forget(resources);
            return Err(format!("DLSS close: {error:?}"));
        }
    };
    // SAFETY: Matching registered device/queue; all native inputs retained to completion.
    let submission = unsafe { resources.submit(native, commands) }
        .map_err(|error| format!("DLSS submit: {error:?}"))?;
    super::wait_submission(&submission)?;
    drop(submission);
    drop(frame);
    Ok(())
}

fn verify_output(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    output: &wgpu::Texture,
) -> Result<(), String> {
    let size = output.size();
    let hdr = output.format() == wgpu::TextureFormat::Rgba16Float;
    let pixel_bytes = if hdr { 8 } else { 4 };
    let row_bytes = (size.width * pixel_bytes).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("DLSS output readback"),
        size: u64::from(row_bytes) * u64::from(size.height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        output.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: Some(size.height),
            },
        },
        size,
    );
    let submission = queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(5)),
        })
        .map_err(|error| format!("DLSS readback poll: {error}"))?;
    receiver
        .recv_timeout(std::time::Duration::from_secs(1))
        .map_err(|error| format!("DLSS readback callback: {error}"))?
        .map_err(|error| format!("DLSS readback map: {error}"))?;
    let mapped = buffer
        .slice(..)
        .get_mapped_range()
        .map_err(|error| format!("DLSS readback range: {error}"))?;
    let result =
        super::sr_pixels::validate_output(&mapped, row_bytes, size.width, size.height, hdr);
    drop(mapped);
    buffer.unmap();
    result
}

fn texture(
    device: &wgpu::Device,
    size: DlssRenderSize,
    format: wgpu::TextureFormat,
    output: bool,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("DLSS SR SDK probe"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | if output {
                wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC
            } else {
                wgpu::TextureUsages::empty()
            },
        view_formats: &[],
    })
}

fn clear_color(encoder: &mut wgpu::CommandEncoder, texture: &wgpu::Texture, color: wgpu::Color) {
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("DLSS probe clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(color),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
}
