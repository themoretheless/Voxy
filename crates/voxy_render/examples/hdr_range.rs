//! Float readback of low-light precision and HDR exposure overflow.
use voxy_render::TextureBlit;

#[allow(clippy::too_many_lines)]
fn verify(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    gl: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let size = wgpu::Extent3d {
        width: 4,
        height: 1,
        depth_or_array_layers: 1,
    };
    let descriptor = wgpu::TextureDescriptor {
        label: Some("HDR range source"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    };
    let input = device.create_texture(&descriptor);
    let colors = [
        [0.000_000_08_f32, 0.000_01, -1.0, 0.25],
        [0.25, 1.0, 4.0, 0.5],
        [65504.0, f32::MAX, 0.0, 0.75],
        [0.000_001, 0.001, 16.0, 1.0],
    ];
    queue.write_texture(
        input.as_image_copy(),
        bytemuck::cast_slice(&colors),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(64),
            rows_per_image: Some(1),
        },
        size,
    );
    // GL downlevel permits RGBA16 render targets; native backends retain f32 precision.
    let format = if gl {
        wgpu::TextureFormat::Rgba16Float
    } else {
        wgpu::TextureFormat::Rgba32Float
    };
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("HDR range output"),
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        ..descriptor
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("HDR range readback"),
        size: 768,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let base = TextureBlit::tone_mapped(device, format, 1.0).ok_or("base rejected")?;
    let exposures = [1.0, f32::MAX, 1e-30];
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    for (frame, exposure) in exposures.into_iter().enumerate() {
        let blit = base
            .with_exposure(device, exposure)
            .ok_or("exposure rejected")?;
        blit.encode(
            device,
            &mut encoder,
            &input.create_view(&wgpu::TextureViewDescriptor::default()),
            &output.create_view(&wgpu::TextureViewDescriptor::default()),
        );
        encoder.copy_texture_to_buffer(
            output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: u64::try_from(frame)? * 256,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            size,
        );
    }
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let bytes = readback.slice(..).get_mapped_range()?;
    for (frame, exposure) in exposures.into_iter().enumerate() {
        for (pixel, color) in colors.iter().enumerate() {
            for (channel, value) in color.iter().enumerate() {
                let actual = if gl {
                    let offset = frame * 256 + (pixel * 4 + channel) * 2;
                    half::f16::from_bits(u16::from_le_bytes(bytes[offset..offset + 2].try_into()?))
                        .to_f32()
                } else {
                    let offset = frame * 256 + (pixel * 4 + channel) * 4;
                    f32::from_le_bytes(bytes[offset..offset + 4].try_into()?)
                };
                let exposed = f64::from(value.max(0.0)) * f64::from(exposure);
                let expected = if channel == 3 {
                    f64::from(*value)
                } else {
                    exposed / (1.0 + exposed)
                };
                let tolerance = if gl {
                    0.0005
                } else {
                    expected.abs() * 0.000_002 + 1e-35
                };
                assert!(
                    actual.is_finite() && (f64::from(actual) - expected).abs() <= tolerance,
                    "HDR pixel {pixel} channel {channel} exposure {exposure}: {actual} != {expected}"
                );
            }
        }
    }
    drop(bytes);
    readback.unmap();
    println!(
        "HDR RANGE PASS: low-light precision, >1 radiance, exposure overflow, alpha, independent snapshots in one submission"
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await?;
        let info = adapter.get_info();
        println!("HDR adapter: {info:?}");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await?;
        verify(&device, &queue, info.backend == wgpu::Backend::Gl)?;
        resolve_half(&device, &queue)
    })
}

#[allow(clippy::too_many_lines)]
fn resolve_half(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    let descriptor = wgpu::TextureDescriptor {
        label: Some("mipmapped wide HDR to SDK half-float proof"),
        size: wgpu::Extent3d {
            width: 4,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 2,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    };
    let input = device.create_texture(&descriptor);
    let colors = [
        [131_008.0_f32, 1.0, 65504.0, 0.5],
        [f32::MAX, 0.01, -1.0, 1.0],
        [12.0, 34.0, 56.0, 0.0],
        [0.125, 0.25, 0.5, 0.25],
    ];
    queue.write_texture(
        input.as_image_copy(),
        bytemuck::cast_slice(&colors),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(64),
            rows_per_image: Some(1),
        },
        input.size(),
    );
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            mip_level: 1,
            ..input.as_image_copy()
        },
        bytemuck::cast_slice(&[[-100.0_f32, 17.0, 23.0, 0.0]]),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let pipeline = voxy_render::HdrHalfResolvePipeline::new(device)?;
    reject_resolve_inputs(device, &pipeline);
    let resolve = pipeline.prepare(&input)?;
    let output = resolve.output();
    assert_eq!(output.size(), input.size());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("half resolve readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    resolve.encode(&mut encoder);
    encoder.copy_texture_to_buffer(
        output.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        output.size(),
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let bytes = buffer.slice(..).get_mapped_range()?;
    let expected = [
        [65504.0_f32, 1.0, 65504.0, 0.5],
        [65504.0, 0.01, 0.0, 1.0],
        [12.0, 34.0, 56.0, 0.0],
        [0.125, 0.25, 0.5, 0.25],
    ];
    for (pixel, channels) in expected.into_iter().enumerate() {
        for (channel, expected) in channels.into_iter().enumerate() {
            let offset = pixel * 8 + channel * 2;
            let actual =
                half::f16::from_bits(u16::from_le_bytes(bytes[offset..offset + 2].try_into()?))
                    .to_f32();
            assert!(
                actual.is_finite() && (actual - expected).abs() <= expected.abs() * 0.001 + 1e-6,
                "HDR half resolve pixel {pixel} channel {channel}: {actual} != {expected}"
            );
        }
    }
    drop(bytes);
    buffer.unmap();
    println!(
        "HDR HALF RESOLVE PASS: linear values, range clipping, negative RGB clamp, alpha preservation"
    );
    Ok(())
}

fn reject_resolve_inputs(device: &wgpu::Device, pipeline: &voxy_render::HdrHalfResolvePipeline) {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    for (format, usage, layers) in [
        (
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::TEXTURE_BINDING,
            1,
        ),
        (
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureUsages::COPY_DST,
            1,
        ),
        (
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureUsages::TEXTURE_BINDING,
            2,
        ),
    ] {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rejected HDR resolve input"),
            size: wgpu::Extent3d {
                width: 2,
                height: 1,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        assert!(matches!(
            pipeline.prepare(&texture),
            Err(voxy_render::RaySceneError::InvalidGeometry)
        ));
    }
    assert!(
        pollster::block_on(scope.pop()).is_none(),
        "resolve rejection emitted GPU validation errors"
    );
}
