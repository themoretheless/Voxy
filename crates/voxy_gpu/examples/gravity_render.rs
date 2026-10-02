//! Compute and rendering share resident body storage; only final pixels are mapped.
#[path = "support/native_backend.rs"]
mod native_backend;
use voxy_gpu::{GravityBody, GravityBudget, GravityParameters, GravityProgram};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (device, queue) = gpu()?;
    let program = pollster::block_on(GravityProgram::new(&device, GravityBudget::default()))?;
    let job = program.create_job(
        &device,
        &[GravityBody {
            mass: 1.0,
            position: [-0.5, 0.0, 0.0],
            velocity: [1.0, 0.0, 0.0],
        }],
        GravityParameters {
            constant: 0.0,
            softening: 0.0,
            uniform_acceleration: [0.0; 3],
            dt: 1.0 / 64.0,
        },
    )?;
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let drawing = pollster::block_on(voxy_gpu::GravityView::new(
        &device,
        &job,
        wgpu::TextureFormat::Rgba8Unorm,
    ))?;
    let render_buffer = plain_buffer(&device);
    verify_buffer_rejections(&device, &render_buffer);
    let independent = pollster::block_on(voxy_gpu::GravityView::from_buffer(
        &device,
        &render_buffer,
        1,
        wgpu::TextureFormat::Rgba8Unorm,
    ))?;
    let (texture, pixels) = targets(&device);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    for (steps, expected_x) in [(0, 16_usize), (32, 32), (32, 48)] {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        if steps > 0 {
            job.encode_steps(&mut encoder, steps)?;
        }
        if steps == 0 {
            drawing.encode(&mut encoder, &view);
        } else {
            encoder.copy_buffer_to_buffer(job.buffer(), 0, &render_buffer, 0, 64);
            independent.encode(&mut encoder, &view);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::default(),
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &pixels,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(64),
                },
            },
            texture.size(),
        );
        queue.submit([encoder.finish()]);
        verify_pixels(&device, &pixels, expected_x, steps)?;
    }
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    println!(
        "PASS: initial and two evolved frames; compute-to-vertex storage visibility, old positions cleared; independent render-only ABI and invalid buffer rejection; no body readback"
    );
    Ok(())
}

fn targets(device: &wgpu::Device) -> (wgpu::Texture, wgpu::Buffer) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let pixels = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 64 * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    (texture, pixels)
}

fn verify_buffer_rejections(device: &wgpu::Device, buffer: &wgpu::Buffer) {
    for count in [0, 2, u32::MAX] {
        assert!(
            pollster::block_on(voxy_gpu::GravityView::from_buffer(
                device,
                buffer,
                count,
                wgpu::TextureFormat::Rgba8Unorm,
            ))
            .is_err()
        );
    }
    let wrong_usage = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 64,
        usage: wgpu::BufferUsages::VERTEX,
        mapped_at_creation: false,
    });
    assert!(
        pollster::block_on(voxy_gpu::GravityView::from_buffer(
            device,
            &wrong_usage,
            1,
            wgpu::TextureFormat::Rgba8Unorm,
        ))
        .is_err()
    );
}

fn verify_pixels(
    device: &wgpu::Device,
    pixels: &wgpu::Buffer,
    expected_x: usize,
    steps: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    pixels
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    receiver.recv()??;
    let data = pixels.slice(..).get_mapped_range()?;
    for x in [16_usize, 32, 48] {
        let offset = 32 * 256 + x * 4;
        let expected = if x == expected_x {
            [0, 255, 0, 255]
        } else {
            [0, 0, 0, 255]
        };
        assert_eq!(
            &data[offset..offset + 4],
            &expected,
            "pixel x={x}, steps={steps}"
        );
    }
    drop(data);
    pixels.unmap();
    Ok(())
}

fn plain_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("independent render-only gravity ABI"),
        size: 64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn gpu() -> Result<(wgpu::Device, wgpu::Queue), Box<dyn std::error::Error + Send + Sync>> {
    let options = native_backend::parse(std::env::args().skip(1))?;
    let instance = options.create_instance();
    let adapter = pollster::block_on(options.adapter(&instance))?;
    println!("Gravity render: {:?}", adapter.get_info());
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    Ok((device, queue))
}
