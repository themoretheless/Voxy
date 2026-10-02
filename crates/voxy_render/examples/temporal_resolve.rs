//! Analytic GPU reprojection: valid history, disocclusion, offscreen, NaN and reset.
use voxy_render::{TemporalResolve, TemporalResolveInputs, TemporalResolveOptions};
fn texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    bytes: &[u8],
) -> wgpu::Texture {
    let size = wgpu::Extent3d {
        width: 4,
        height: 1,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("temporal fixture"),
        size,
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
            bytes_per_row: Some(u32::try_from(bytes.len()).unwrap()),
            rows_per_image: Some(1),
        },
        size,
    );
    texture
}
#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    println!("Temporal resolve GPU: {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: adapter.limits(),
        ..Default::default()
    }))?;
    let current = texture(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba32Float,
        bytemuck::cast_slice(&[[2.0_f32; 4], [2.0; 4], [2.0; 4], [f32::NAN; 4]]),
    );
    let history = texture(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba32Float,
        bytemuck::cast_slice(&[[10.0_f32; 4], [20.0; 4], [30.0; 4], [40.0; 4]]),
    );
    let motion = texture(
        &device,
        &queue,
        wgpu::TextureFormat::Rg32Float,
        bytemuck::cast_slice(&[[0.25_f32, 0.0], [0.0, 0.0], [0.75, 0.0], [f32::NAN, 0.0]]),
    );
    let expected = texture(
        &device,
        &queue,
        wgpu::TextureFormat::R32Float,
        bytemuck::cast_slice(&[0.5_f32, 0.2, 0.5, 0.5]),
    );
    let depth = texture(
        &device,
        &queue,
        wgpu::TextureFormat::R32Float,
        bytemuck::cast_slice(&[0.5_f32; 4]),
    );
    let resolver = TemporalResolve::new(&device)?;
    let options = TemporalResolveOptions {
        history_weight: 0.5,
        depth_tolerance: 0.001,
        reset_history: false,
    };
    let inputs = || TemporalResolveInputs {
        current: &current,
        motion: &motion,
        history: &history,
        expected_previous_depth: &expected,
        history_depth: &depth,
    };
    let frame = resolver.prepare(inputs(), options)?;
    let mut retained = voxy_render::TemporalHistory::new(&device, 4, 1)?;
    assert!(!retained.valid());
    let initial_output = retained.output().clone();
    let original_history = retained.color().clone();
    let clipped = resolver.prepare_into(inputs(), options, retained.output(), true)?;
    assert_eq!(clipped.output(), &initial_output);
    assert!(!retained.resize(4, 1)?);
    assert!(retained.resize(0, 1).is_err());
    assert_eq!(retained.color(), &original_history);
    for (width, format, usage, mips) in [
        (
            2,
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureUsages::STORAGE_BINDING,
            1,
        ),
        (
            4,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureUsages::STORAGE_BINDING,
            1,
        ),
        (
            4,
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureUsages::COPY_DST,
            1,
        ),
        (
            4,
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureUsages::STORAGE_BINDING,
            2,
        ),
    ] {
        let invalid = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("invalid retained temporal target"),
            size: wgpu::Extent3d {
                width,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: mips,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        assert!(
            resolver
                .prepare_into(inputs(), options, &invalid, true)
                .is_err()
        );
    }
    assert!(
        resolver
            .prepare_into(
                TemporalResolveInputs {
                    current: frame.output(),
                    ..inputs()
                },
                options,
                frame.output(),
                false
            )
            .is_err()
    );
    assert!(
        resolver
            .prepare_into(inputs(), options, inputs().current, false)
            .is_err()
    );
    let dark_history = texture(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba32Float,
        bytemuck::cast_slice(&[[0.0_f32; 4]; 4]),
    );
    let dark = resolver.prepare_clipped(
        TemporalResolveInputs {
            history: &dark_history,
            ..inputs()
        },
        options,
    )?;
    let varied_current = texture(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba32Float,
        bytemuck::cast_slice(&[[2.0_f32; 4], [8.0; 4], [4.0; 4], [f32::NAN; 4]]),
    );
    let varied = resolver.prepare_clipped(
        TemporalResolveInputs {
            current: &varied_current,
            ..inputs()
        },
        options,
    )?;
    let reset = resolver.prepare(
        inputs(),
        TemporalResolveOptions {
            reset_history: true,
            ..options
        },
    )?;
    let partial_current = texture(
        &device,
        &queue,
        wgpu::TextureFormat::Rgba32Float,
        bytemuck::cast_slice(&[
            [f32::NAN, 3.0_f32, 7.0, 1.0],
            [2.0, f32::INFINITY, 5.0, 1.0],
            [-2.0, 4.0, f32::NEG_INFINITY, 1.0],
            [6.0, 8.0, 9.0, f32::NAN],
        ]),
    );
    let partial = resolver.prepare(
        TemporalResolveInputs {
            current: &partial_current,
            ..inputs()
        },
        TemporalResolveOptions {
            reset_history: true,
            ..options
        },
    )?;
    assert!(
        resolver
            .prepare(
                inputs(),
                TemporalResolveOptions {
                    history_weight: f32::NAN,
                    ..options
                }
            )
            .is_err()
    );
    let copy = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("temporal proof"),
        size: 1792,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    retained.encode_depth(&mut encoder, &depth)?;
    let attachment = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("presented depth attachment fixture"),
        size: depth.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    {
        let view = attachment.create_view(&Default::default());
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.25),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    retained.encode_depth_attachment(&mut encoder, &attachment)?;
    for (index, frame) in [frame, reset, clipped, dark, varied, partial]
        .iter()
        .enumerate()
    {
        frame.encode(&mut encoder);
        encoder.copy_texture_to_buffer(
            frame.output().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &copy,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: u64::try_from(index)? * 256,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            frame.output().size(),
        );
    }
    encoder.copy_texture_to_buffer(
        retained.output_depth().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &copy,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 1536,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        retained.output_depth().size(),
    );
    let snapshot =
        voxy_render::ComputeDispatch::copy_buffer(&device, &mut encoder, &copy, 0, 1792)?;
    queue.submit([encoder.finish()]);
    let mut read = snapshot.begin_read();
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let bytes = read.try_read()?.ok_or("temporal read pending")?;
    for pixel in 0..4 {
        let offset = 1536 + pixel * 4;
        assert_eq!(
            f32::from_le_bytes(bytes[offset..offset + 4].try_into()?),
            0.25,
            "retained depth {pixel}"
        );
    }
    for (row, values) in [
        [11.0_f32, 2.0, 2.0, 0.0],
        [2.0, 2.0, 2.0, 0.0],
        [2.0, 2.0, 2.0, 0.0],
        [2.0, 2.0, 2.0, 0.0],
        [5.0, 8.0, 4.0, 0.0],
    ]
    .iter()
    .enumerate()
    {
        for (pixel, expected) in values.iter().enumerate() {
            for channel in 0..4 {
                let offset = row * 256 + pixel * 16 + channel * 4;
                let actual = f32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
                let expected = if channel == 3 { 1.0 } else { *expected };
                assert!(
                    actual.is_finite() && (actual - expected).abs() < 0.0001,
                    "pixel {pixel}, channel {channel}: {actual} != {expected}"
                );
            }
        }
    }
    for (pixel, expected) in [
        [0.0_f32, 3.0, 7.0, 1.0],
        [2.0, 0.0, 5.0, 1.0],
        [0.0, 4.0, 0.0, 1.0],
        [6.0, 8.0, 9.0, 1.0],
    ]
    .iter()
    .enumerate()
    {
        for (channel, expected) in expected.iter().enumerate() {
            let offset = 5 * 256 + pixel * 16 + channel * 4;
            let actual = f32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
            assert!(
                actual.is_finite() && (actual - expected).abs() < 0.0001,
                "partial pixel {pixel}, channel {channel}: {actual} != {expected}"
            );
        }
    }
    // Headless simulation of the caller's successful-presentation notification.
    retained.presented();
    assert!(retained.valid());
    assert_eq!(retained.color(), &initial_output);
    assert_eq!(retained.output(), &original_history);
    assert!(retained.resize(0, 1).is_err());
    assert!(retained.valid());
    retained.reset();
    assert!(!retained.valid());
    assert_eq!(retained.color(), &initial_output);
    retained.presented();
    assert_eq!(retained.color(), &original_history);
    assert!(retained.resize(2, 1)?);
    assert!(!retained.valid());
    println!(
        "TEMPORAL RESOLVE PASS: backward UV, depth rejection, offscreen/NaN, HDR blend, bright/dark history clipping, border range, reset and per-channel NaN/Inf sanitation"
    );
    Ok(())
}
