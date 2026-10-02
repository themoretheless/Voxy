//! GPU static radiance history acceptance; readback is verification only.
use voxy_render::{RadianceAccumulationFrame, RadianceAccumulator};
fn source(device: &wgpu::Device, queue: &wgpu::Queue, value: f32) -> wgpu::Texture {
    let size = wgpu::Extent3d {
        width: 17,
        height: 9,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("radiance fixture"),
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
        bytemuck::cast_slice(&[[value; 4]; 153]),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(272),
            rows_per_image: Some(9),
        },
        size,
    );
    texture
}
fn capture(
    frame: &RadianceAccumulationFrame,
    encoder: &mut wgpu::CommandEncoder,
    buffer: &wgpu::Buffer,
    index: u64,
) {
    frame.encode(encoder);
    capture_texture(frame.output(), encoder, buffer, index);
}
fn capture_texture(
    texture: &wgpu::Texture,
    encoder: &mut wgpu::CommandEncoder,
    buffer: &wgpu::Buffer,
    index: u64,
) {
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer,
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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await?;
        println!("Accumulation adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await?;
        let engine = RadianceAccumulator::new(&device)?;
        let one = source(&device, &queue, 1.0);
        let three = source(&device, &queue, 3.0);
        let bright = source(&device, &queue, 100.0);
        let black = source(&device, &queue, 0.0);
        let invalid = source(&device, &queue, f32::NAN);
        let first = engine.prepare(&one, None)?;
        let skipped = engine.prepare(&bright, Some(&first))?;
        let next = engine.prepare(&three, Some(&first))?;
        let null = engine.prepare(&black, Some(&next))?;
        let bad = engine.prepare(&invalid, Some(&next))?;
        let reset = engine.prepare(&three, None)?;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("accumulation acceptance"),
            size: 3840,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for (i, frame) in [&first, &skipped, &next, &null, &bad, &reset]
            .into_iter()
            .enumerate()
        {
            capture(frame, &mut encoder, &buffer, u64::try_from(i)?);
        }
        let mut long = engine.prepare(&one, None)?;
        long.encode(&mut encoder);
        for _ in 0..64 {
            let candidate = engine.prepare(&three, Some(&long))?;
            candidate.encode(&mut encoder);
            long = candidate;
        }
        assert_eq!(long.samples(), 65);
        capture(&long, &mut encoder, &buffer, 6);
        let maximum = source(&device, &queue, f32::MAX);
        let mut extreme = engine.prepare(&maximum, None)?;
        extreme.encode(&mut encoder);
        for _ in 0..36 {
            let candidate = engine.prepare(&maximum, Some(&extreme))?;
            candidate.encode(&mut encoder);
            extreme = candidate;
        }
        capture(&extreme, &mut encoder, &buffer, 7);
        // The referenced first frame must be encoded before its consumer.
        let initial = engine.prepare(&maximum, None)?;
        initial.encode(&mut encoder);
        let half = engine.prepare(&black, Some(&initial))?;
        capture(&half, &mut encoder, &buffer, 8);
        let hdr = engine.prepare(&maximum, Some(&first))?;
        capture(&hdr, &mut encoder, &buffer, 9);
        let large = source(&device, &queue, 131_008.0);
        let composition = voxy_render::HdrCompositionPipeline::new(&device)?;
        for (index, a, b) in [
            (10, &one, &three),
            (11, &large, &bright),
            (12, &maximum, &maximum),
        ] {
            let composed = composition.create_job(a, b)?;
            composed.encode(&mut encoder);
            capture_texture(composed.output(), &mut encoder, &buffer, index);
        }
        let initial_sum = composition.create_job(&one, &one)?;
        initial_sum.encode(&mut encoder);
        let old_output = initial_sum.output().clone();
        let reused = composition.create_job_reusing(initial_sum, &one, &three)?;
        assert_eq!(reused.output(), &old_output, "HDR output was not reused");
        reused.encode(&mut encoder);
        capture_texture(reused.output(), &mut encoder, &buffer, 13);
        let input_alias = reused.output().clone();
        let unaliased = composition.create_job_reusing(reused, &input_alias, &three)?;
        assert_ne!(
            unaliased.output(),
            &input_alias,
            "HDR output aliases its input"
        );
        unaliased.encode(&mut encoder);
        capture_texture(unaliased.output(), &mut encoder, &buffer, 14);
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let bytes = buffer.slice(..).get_mapped_range()?;
        verify(&bytes)?;
        drop(bytes);
        buffer.unmap();
        println!(
            "RADIANCE ACCUMULATION PASS: skipped candidate, reset, null/invalid zero contributions, 65 samples, alpha one"
        );
        Ok(())
    })
}

fn verify(bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    for (i, expected) in [1.0, 50.5, 2.0, 4.0 / 3.0, 4.0 / 3.0, 3.0, 193.0 / 65.0]
        .into_iter()
        .enumerate()
    {
        for c in 0..4 {
            let offset = i * 256 + c * 4;
            let actual = f32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
            let target = if c == 3 { 1.0 } else { expected };
            assert!(
                actual.is_finite() && (actual - target).abs() < 0.000_01,
                "radiance history {i} channel {c}: {actual} != {target}"
            );
        }
    }
    for (index, expected) in [(7, 1.0_f64), (8, 0.5), (9, 0.5)] {
        let offset = index * 256;
        let actual = f32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
        let ratio = f64::from(actual) / f64::from(f32::MAX);
        assert!(
            actual.is_finite() && (ratio - expected).abs() < 0.000_01,
            "HDR accumulation {index}: {actual}, ratio {ratio} != {expected}"
        );
    }
    for (index, expected) in [
        (10, 4.0_f64),
        (11, 131_108.0),
        (12, f64::from(f32::MAX)),
        (13, 4.0),
        (14, 7.0),
    ] {
        let offset = index * 256;
        let actual = f32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
        assert!(
            actual.is_finite() && (f64::from(actual) / expected - 1.0).abs() < 0.000_01,
            "wide HDR composition {index}: {actual} != {expected}"
        );
    }
    Ok(())
}
