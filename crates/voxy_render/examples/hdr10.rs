//! HDR10 PQ/gamut GPU readback against a double-precision primary-derived oracle.
use voxy_render::TextureBlit;

#[allow(clippy::too_many_lines)]
fn verify(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    gl: bool,
    packed: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let size = wgpu::Extent3d {
        width: 8,
        height: 1,
        depth_or_array_layers: 1,
    };
    let descriptor = wgpu::TextureDescriptor {
        label: Some("HDR10 source"),
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
        [0.0_f32, 0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0, 1.0],
        [10.0, 10.0, 10.0, 1.0],
        [100.0, 100.0, 100.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [f32::MAX, -1.0, 0.0, 1.0],
    ];
    queue.write_texture(
        input.as_image_copy(),
        bytemuck::cast_slice(&colors),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(128),
            rows_per_image: Some(1),
        },
        size,
    );
    // GL downlevel permits RGBA16 render targets; native backends retain f32 precision.
    let format = if packed {
        wgpu::TextureFormat::Rgb10a2Unorm
    } else if gl {
        wgpu::TextureFormat::Rgba16Float
    } else {
        wgpu::TextureFormat::Rgba32Float
    };
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("HDR10 output"),
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        ..descriptor
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("HDR10 readback"),
        size: 512,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    for invalid in [0.0, -1.0, f32::NAN, f32::INFINITY, 10001.0] {
        assert!(TextureBlit::hdr10(device, format, invalid).is_none());
    }
    assert!(TextureBlit::hdr10(device, wgpu::TextureFormat::Rgba8UnormSrgb, 100.0).is_none());
    let base = TextureBlit::hdr10(device, format, 100.0).ok_or("base rejected")?;
    let exposures = [100.0, 203.0];
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    for (frame, exposure) in exposures.into_iter().enumerate() {
        let blit = base
            .with_sdr_white_nits(device, exposure)
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
                let actual = if packed {
                    let offset = frame * 256 + pixel * 4;
                    let word = u32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
                    let (shift, mask) = if channel == 3 {
                        (30, 3_u32)
                    } else {
                        (channel * 10, 1023_u32)
                    };
                    f32::from(u16::try_from((word >> shift) & mask)?)
                        / f32::from(u16::try_from(mask)?)
                } else if gl {
                    let offset = frame * 256 + (pixel * 4 + channel) * 2;
                    half::f16::from_bits(u16::from_le_bytes(bytes[offset..offset + 2].try_into()?))
                        .to_f32()
                } else {
                    let offset = frame * 256 + (pixel * 4 + channel) * 4;
                    f32::from_le_bytes(bytes[offset..offset + 4].try_into()?)
                };
                let expected = if channel == 3 {
                    f64::from(*value)
                } else {
                    let input = glam::DVec3::from_array([
                        f64::from(color[0].max(0.0)),
                        f64::from(color[1].max(0.0)),
                        f64::from(color[2].max(0.0)),
                    ]);
                    let wide = conversion() * input;
                    pq((wide[channel] * f64::from(exposure)).clamp(0.0, 10000.0))
                };
                let tolerance = if packed {
                    1.0 / 1023.0 + 0.00005
                } else if gl {
                    0.0005
                } else {
                    0.00005
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
        "HDR10 PASS: black, 100/1000/10000 nits, BT.709 primaries, clipping, alpha, batched white snapshots"
    );
    Ok(())
}

fn xyz(x: f64, y: f64) -> glam::DVec3 {
    glam::DVec3::new(x / y, 1.0, (1.0 - x - y) / y)
}
fn primaries(red: [f64; 2], green: [f64; 2], blue: [f64; 2]) -> glam::DMat3 {
    let matrix = glam::DMat3::from_cols(
        xyz(red[0], red[1]),
        xyz(green[0], green[1]),
        xyz(blue[0], blue[1]),
    );
    let scale = matrix.inverse() * xyz(0.3127, 0.3290);
    matrix * glam::DMat3::from_diagonal(scale)
}
fn conversion() -> glam::DMat3 {
    // Derive independently from BT.709 and BT.2020 xy primaries plus D65 white.
    let narrow = primaries([0.64, 0.33], [0.30, 0.60], [0.15, 0.06]);
    let wide = primaries([0.708, 0.292], [0.170, 0.797], [0.131, 0.046]);
    wide.inverse() * narrow
}
fn pq(nits: f64) -> f64 {
    let p = (nits / 10000.0).powf(2610.0 / 16384.0);
    ((3424.0 / 4096.0 + 2413.0 / 128.0 * p) / (1.0 + 2392.0 / 128.0 * p)).powf(2523.0 / 32.0)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await?;
        let info = adapter.get_info();
        println!("HDR10 adapter: {info:?}");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await?;
        verify(&device, &queue, info.backend == wgpu::Backend::Gl, false)?;
        verify(&device, &queue, info.backend == wgpu::Backend::Gl, true)
    })
}
