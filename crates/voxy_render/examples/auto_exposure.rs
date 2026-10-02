//! GPU exposure + tone mapping; CPU reads below are verification only.
use voxy_render::{AutoExposure, AutoExposureFrame, ExposureSettings, TextureBlit};
fn texture(device: &wgpu::Device, queue: &wgpu::Queue, values: &[[f32; 4]]) -> wgpu::Texture {
    let size = wgpu::Extent3d {
        width: 17,
        height: 9,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("exposure HDR fixture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        bytemuck::cast_slice(values),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(17 * 16),
            rows_per_image: Some(9),
        },
        size,
    );
    texture
}
fn exposure_copy(
    encoder: &mut wgpu::CommandEncoder,
    frame: &AutoExposureFrame,
    readback: &wgpu::Buffer,
    index: u64,
) {
    frame.encode(encoder);
    encoder.copy_buffer_to_buffer(frame.output(), 0, readback, index * 256, 16);
}
#[allow(clippy::too_many_lines)]
fn verify(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<(), Box<dyn std::error::Error>> {
    let pixels: Vec<_> = (0..153)
        .map(|index| {
            [
                [0.0; 4],
                [1.0; 4],
                [4.0; 4],
                [f32::NAN; 4],
                [f32::INFINITY; 4],
                [-1.0; 4],
                [4.0, 0.0, 0.0, 1.0],
                [0.0, 1.0, 0.0, 1.0],
                [0.0, 0.0, 16.0, 1.0],
                [0.5, 0.5, 0.5, f32::NAN],
            ][index % 10]
        })
        .collect();
    let valid: Vec<_> = pixels
        .iter()
        .filter_map(|p| {
            if !p[..3].iter().all(|v| v.is_finite()) {
                return None;
            }
            let luminance = f64::from(p[0].max(0.0)) * 0.2126
                + f64::from(p[1].max(0.0)) * 0.7152
                + f64::from(p[2].max(0.0)) * 0.0722;
            (luminance > 0.0).then_some(luminance)
        })
        .collect();
    let count = f64::from(u32::try_from(valid.len())?);
    let geometric = (valid.iter().map(|v| v.ln()).sum::<f64>() / count).exp();
    let expected_first = 0.18 / geometric;
    let mixed = texture(device, queue, &pixels);
    let bright = texture(device, queue, &[[4.0; 4]; 153]);
    let dim = texture(device, queue, &[[0.25; 4]; 153]);
    let black = texture(device, queue, &[[0.0; 4]; 153]);
    let invalid = texture(device, queue, &[[f32::NAN; 4]; 153]);
    let engine = AutoExposure::new(device)?;
    let settings = ExposureSettings::default();
    assert!(engine.prepare(&mixed, None, settings, f32::NAN).is_err());
    assert!(
        engine
            .prepare(
                &mixed,
                None,
                ExposureSettings {
                    minimum: 0.0,
                    ..settings
                },
                1.0
            )
            .is_err()
    );
    let first = engine.prepare(&mixed, None, settings, 0.0)?;
    let skipped = engine.prepare(&bright, Some(&first), settings, 1.0)?;
    let next = engine.prepare(&dim, Some(&first), settings, 0.5)?;
    let held = engine.prepare(&black, Some(&first), settings, 1.0)?;
    let reset = engine.prepare(&dim, None, settings, 0.0)?;
    let bad = engine.prepare(&invalid, None, settings, 1.0)?;
    let stopped = engine.prepare(&dim, Some(&first), settings, 0.0)?;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("exposure proof readback"),
        size: 4864,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    for (index, frame) in [&first, &skipped, &next, &held, &reset, &bad, &stopped]
        .into_iter()
        .enumerate()
    {
        exposure_copy(&mut encoder, frame, &readback, u64::try_from(index)?);
    }
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("auto exposed display"),
        size: dim.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let blit = TextureBlit::tone_mapped(device, output.format(), 1.0)
        .ok_or("tone map rejected")?
        .with_auto_exposure(&reset)
        .ok_or("GPU exposure rejected")?;
    blit.encode(
        device,
        &mut encoder,
        &dim.create_view(&wgpu::TextureViewDescriptor::default()),
        &output.create_view(&wgpu::TextureViewDescriptor::default()),
    );
    encoder.copy_texture_to_buffer(
        output.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 1792,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    for (pass, offset) in [
        (
            TextureBlit::linear_exposed(device, output.format(), 1.0)
                .ok_or("linear exposure rejected")?,
            2048,
        ),
        (
            TextureBlit::hdr10(device, output.format(), 203.0).ok_or("PQ exposure rejected")?,
            2304,
        ),
    ] {
        let pass = pass
            .with_auto_exposure(&reset)
            .ok_or("HDR GPU exposure rejected")?;
        pass.encode(
            device,
            &mut encoder,
            &dim.create_view(&wgpu::TextureViewDescriptor::default()),
            &output.create_view(&wgpu::TextureViewDescriptor::default()),
        );
        encoder.copy_texture_to_buffer(
            output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }
    let left = texture(device, queue, &[[1.0; 4]; 153]);
    let stereo = engine.prepare_stereo([&left, &bright], None, settings, 0.0)?;
    let one_valid = engine.prepare_stereo([&left, &black], None, settings, 0.0)?;
    let stereo_held = engine.prepare_stereo([&black, &invalid], Some(&stereo), settings, 1.0)?;
    for (index, frame) in [(10, &stereo), (11, &one_valid), (12, &stereo_held)] {
        exposure_copy(&mut encoder, frame, &readback, index);
    }
    // A submitted-but-unpresented candidate must not become stereo history.
    let stereo_skipped = engine.prepare_stereo([&bright, &bright], Some(&stereo), settings, 1.0)?;
    let stereo_next = engine.prepare_stereo([&dim, &dim], Some(&stereo), settings, 0.5)?;
    let stereo_reset = engine.prepare_stereo([&dim, &dim], None, settings, 0.0)?;
    let stereo_stopped = engine.prepare_stereo([&dim, &dim], Some(&stereo), settings, 0.0)?;
    for (index, frame) in [
        (15, &stereo_skipped),
        (16, &stereo_next),
        (17, &stereo_reset),
        (18, &stereo_stopped),
    ] {
        exposure_copy(&mut encoder, frame, &readback, index);
    }
    let mismatched = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("mismatched stereo eye"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    assert!(
        engine
            .prepare_stereo([&left, &mismatched], Some(&stereo), settings, 1.0)
            .is_err()
    );
    let shared_display = TextureBlit::linear_exposed(device, output.format(), 1.0)
        .ok_or("linear exposure unavailable")?
        .with_auto_exposure(&stereo)
        .ok_or("stereo exposure unavailable")?;
    for (index, eye) in [(13, &left), (14, &bright)] {
        shared_display.encode(
            device,
            &mut encoder,
            &eye.create_view(&wgpu::TextureViewDescriptor::default()),
            &output.create_view(&wgpu::TextureViewDescriptor::default()),
        );
        encoder.copy_texture_to_buffer(
            output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: index * 256,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
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
    let expected = [
        expected_first,
        expected_first + (0.045 - expected_first) * (1.0 - (-3.0_f64).exp()),
        expected_first + (0.72 - expected_first) * (1.0 - (-0.5_f64).exp()),
        expected_first,
        0.72,
        1.0,
        expected_first,
    ];
    for (index, expected) in expected.into_iter().enumerate() {
        let offset = index * 256;
        let actual = f32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
        assert!(
            actual.is_finite() && (f64::from(actual) - expected).abs() < 0.00001,
            "exposure {index}: {actual} != {expected}"
        );
    }
    for channel in 0..3 {
        let offset = 1792 + channel * 2;
        let actual =
            half::f16::from_bits(u16::from_le_bytes(bytes[offset..offset + 2].try_into()?))
                .to_f32();
        assert!((actual - 0.18 / 1.18).abs() < 0.0002);
    }
    let p = (36.54_f64 / 10000.0).powf(2610.0 / 16384.0);
    let pq_expected =
        ((3424.0 / 4096.0 + 2413.0 / 128.0 * p) / (1.0 + 2392.0 / 128.0 * p)).powf(2523.0 / 32.0);
    for (base, expected) in [(2048, 0.18), (2304, pq_expected)] {
        for channel in 0..3 {
            let offset = base + channel * 2;
            let actual =
                half::f16::from_bits(u16::from_le_bytes(bytes[offset..offset + 2].try_into()?))
                    .to_f32();
            assert!(
                (f64::from(actual) - expected).abs() < 0.0005,
                "HDR auto exposure: {actual} != {expected}"
            );
        }
    }
    for (index, expected) in [(10, 0.09), (11, 0.18), (12, 0.09)] {
        let offset = index * 256;
        let actual = f32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
        assert!(
            (actual - expected).abs() < 0.00001,
            "stereo exposure {actual} != {expected}"
        );
    }
    for (index, expected) in [(13, 0.09), (14, 0.36)] {
        let offset = index * 256;
        let actual =
            half::f16::from_bits(u16::from_le_bytes(bytes[offset..offset + 2].try_into()?))
                .to_f32();
        assert!(
            (actual - expected).abs() < 0.0005,
            "stereo display {actual} != {expected}"
        );
    }
    for (index, expected) in [
        (15, 0.09 + (0.045 - 0.09) * (1.0 - (-3.0_f64).exp())),
        (16, 0.09 + (0.72 - 0.09) * (1.0 - (-0.5_f64).exp())),
        (17, 0.72),
        (18, 0.09),
    ] {
        let offset = index * 256;
        let actual = f32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
        assert!(
            (f64::from(actual) - expected).abs() < 0.00001,
            "stereo temporal exposure {actual} != {expected}"
        );
    }
    drop(bytes);
    readback.unmap();
    println!(
        "AUTO EXPOSURE PASS: partial tiles, invalid/black exclusion, skipped candidate, adaptation/reset, direct GPU SDR/scRGB/PQ composition, shared stereo exposure"
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
        println!("Exposure adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await?;
        verify(&device, &queue)?;
        let (foreign, foreign_queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await?;
        let blit = TextureBlit::tone_mapped(&device, wgpu::TextureFormat::Rgba8Unorm, 1.0)
            .ok_or("tone mapper")?;
        assert!(blit.with_exposure(&device, 2.0).is_some());
        assert!(blit.with_exposure(&foreign, 2.0).is_none());
        let pq = TextureBlit::hdr10(&device, wgpu::TextureFormat::Rgb10a2Unorm, 100.0)
            .ok_or("PQ mapper")?;
        assert!(pq.with_sdr_white_nits(&device, 200.0).is_some());
        assert!(pq.with_sdr_white_nits(&foreign, 200.0).is_none());
        let foreign_texture = texture(&foreign, &foreign_queue, &[[1.0; 4]; 153]);
        let foreign_exposure = AutoExposure::new(&foreign)?.prepare(
            &foreign_texture,
            None,
            ExposureSettings::default(),
            0.0,
        )?;
        let target = voxy_render::ProcessedColorTarget::new(&foreign, 17, 9, false)?;
        let source = foreign_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            foreign.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        assert!(matches!(
            blit.encode_checked(&foreign, &mut encoder, &source, target.view()),
            Err(voxy_render::SceneError::DeviceMismatch)
        ));
        foreign_queue.submit([encoder.finish()]);
        assert!(blit.with_auto_exposure(&foreign_exposure).is_none());
        assert!(pq.with_auto_exposure(&foreign_exposure).is_none());
        println!(
            "DISPLAY OWNERSHIP PASS: same-device snapshots accepted, foreign exposure/PQ/auto-exposure rejected"
        );
        Ok(())
    })
}
