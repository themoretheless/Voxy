//! GPU planar virtual-point reprojection and temporal integration fixture.
use wgpu::util::DeviceExt;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(run())
}
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let backend = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => voxy_render::GraphicsBackend::Auto,
        [flag, value] if flag == "--backend" => match value.as_str() {
            "metal" => voxy_render::GraphicsBackend::Metal,
            "vulkan" => voxy_render::GraphicsBackend::Vulkan,
            "gl" => voxy_render::GraphicsBackend::OpenGl,
            "dx12" => voxy_render::GraphicsBackend::DirectX12,
            _ => return Err("expected --backend metal|vulkan|gl|dx12".into()),
        },
        _ => return Err("expected --backend metal|vulkan|gl|dx12".into()),
    };
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
    descriptor.backends = backend.backends();
    let instance = wgpu::Instance::new(descriptor);
    let adapter = instance.request_adapter(&Default::default()).await?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            ..Default::default()
        })
        .await?;
    println!("PLANAR GPU: {:?}", adapter.get_info());
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let [width, height] = [9u32, 3u32];
    let mut hits = Vec::new();
    let mut previous = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let px = ((x as f32 + 0.5) / width as f32) * 2.0 - 1.0;
            let py = 1.0 - ((y as f32 + 0.5) / height as f32) * 2.0;
            hits.push(voxy_render::ReflectionHit {
                position_distance: [px + if x == 3 { 0.1 } else { 0.0 }, py, -0.5, 1.0],
                identity: [0, 0, 0, y * width + x],
                barycentrics_valid: [0.0, 0.0, 0.0, if x == 2 { 0.0 } else { 1.0 }],
            });
            previous.push([px + 0.2, py, -0.7, if x == 1 { 0.0 } else { 1.0 }]);
        }
    }
    let buffer = |data: &[u8]| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("controlled planar reprojection input"),
            contents: data,
            usage: wgpu::BufferUsages::STORAGE,
        })
    };
    let hits = buffer(bytemuck::cast_slice(&hits));
    let previous_triangles = previous
        .iter()
        .enumerate()
        .filter(|(i, _)| i % width as usize != 1)
        .map(|(i, p)| voxy_render::PreviousReflectionTriangle {
            identity: [0, 0, 0, i as u32],
            vertices: [*p; 3],
        })
        .rev()
        .collect::<Vec<_>>();
    let correspondence = voxy_render::ReflectionCorrespondencePipeline::new(&device)?;
    let previous =
        correspondence.prepare_hits(&hits, [width, height], &previous_triangles, false)?;
    let pipeline = voxy_render::PlanarReflectionPipeline::new(&device)?;
    let options = voxy_render::PlanarReflectionCameras {
        cameras: [
            glam::Mat4::IDENTITY,
            glam::Mat4::from_translation(glam::Vec3::new(0.1, 0.0, 0.0)),
        ],
        planes: [[0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 1.0, -0.1]],
    };
    let job = pipeline.prepare(&hits, previous.positions(), [width, height], options)?;
    let motion_stride = if job.motion().format() == wgpu::TextureFormat::Rgba32Float {
        16
    } else {
        8
    };
    let texture = |format| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("planar temporal fixture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    };
    let current = texture(wgpu::TextureFormat::Rgba32Float);
    let history = texture(wgpu::TextureFormat::Rgba32Float);
    let history_depth = texture(wgpu::TextureFormat::R32Float);
    for (target, values, stride) in [
        (&current, vec![2.0f32; 108], 16),
        (&history, vec![1.0; 108], 16),
        (&history_depth, vec![0.9; 27], 4),
    ] {
        queue.write_texture(
            target.as_image_copy(),
            bytemuck::cast_slice(&values),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * stride),
                rows_per_image: Some(height),
            },
            target.size(),
        );
    }
    let resolver = voxy_render::TemporalResolve::new(&device)?;
    let resolved = resolver.prepare(
        voxy_render::TemporalResolveInputs {
            current: &current,
            motion: job.motion(),
            history: &history,
            expected_previous_depth: job.expected_previous_depth(),
            history_depth: &history_depth,
        },
        voxy_render::TemporalResolveOptions {
            history_weight: 0.5,
            depth_tolerance: 0.001,
            reset_history: false,
        },
    )?;
    let retained = voxy_render::TemporalHistory::new(&device, width, height)?;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("planar guide/color readback"),
        size: 3840,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    previous.encode(&mut encoder);
    job.encode(&mut encoder);
    resolved.encode(&mut encoder);
    retained.encode_depth(&mut encoder, job.current_depth())?;
    for (index, target) in [
        job.motion(),
        job.expected_previous_depth(),
        job.current_depth(),
        resolved.output(),
        retained.output_depth(),
    ]
    .into_iter()
    .enumerate()
    {
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: index as u64 * 768,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(height),
                },
            },
            target.size(),
        );
    }
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(std::time::Duration::from_secs(10)),
    })?;
    rx.recv_timeout(std::time::Duration::from_secs(10))??;
    if let Some(error) = scope.pop().await {
        return Err(error.to_string().into());
    }
    let mapped = readback.slice(..).get_mapped_range()?;
    let read = |block: usize, x: usize, y: usize, stride: usize, channel: usize| {
        let offset = block * 768 + y * 256 + x * stride + channel * 4;
        f32::from_le_bytes(
            mapped[offset..offset + 4]
                .try_into()
                .expect("four-byte float"),
        )
    };
    for y in 0..3 {
        for x in 0..9 {
            let correspondence = ![1, 2, 3, 8].contains(&x);
            let valid_current = ![2, 3].contains(&x);
            for (actual, expected) in [
                (
                    read(0, x, y, motion_stride, 0),
                    if correspondence { 0.15 } else { 0.0 },
                ),
                (read(0, x, y, motion_stride, 1), 0.0),
                (read(1, x, y, 4, 0), if correspondence { 0.9 } else { 0.0 }),
                (read(2, x, y, 4, 0), if valid_current { 0.5 } else { 0.0 }),
                (read(4, x, y, 4, 0), if valid_current { 0.5 } else { 0.0 }),
            ] {
                assert!(
                    actual.is_finite() && (actual - expected).abs() < 1e-5,
                    "guide ({x},{y}): {actual} != {expected}"
                );
            }
            for c in 0..3 {
                let actual = read(3, x, y, 16, c);
                let expected = if correspondence { 1.5 } else { 2.0 };
                assert!(
                    (actual - expected).abs() < 1e-5,
                    "temporal ({x},{y}): {actual} != {expected}"
                );
            }
        }
    }
    drop(mapped);
    readback.unmap();
    println!(
        "PLANAR REFLECTION PASS: 27 GPU pixels, geometry/camera/mirror motion, invalid correspondence, UV bounds, virtual depths, temporal blending and pending history depth copy; controlled inputs, no presentation proof"
    );
    Ok(())
}
