//! Integration proof consuming actual production mirror ray hits/radiance.
use voxy_render::{
    PlanarReflectionCameras, PlanarReflectionPipeline, RayLightingFrame,
    ReflectionCorrespondencePipeline, TemporalResolve, TemporalResolveInputs,
    TemporalResolveOptions,
};
pub fn encode(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    frame: &RayLightingFrame,
    camera: glam::Mat4,
    encoder: &mut wgpu::CommandEncoder,
) -> Result<wgpu::Buffer, Box<dyn std::error::Error>> {
    let reflected = frame.reflection_job();
    let correspondence = ReflectionCorrespondencePipeline::new(device)?;
    let mesh = voxy_render::SkinnedMesh::new(
        [[-10.0, -10.0, 3.0], [10.0, -10.0, 3.0], [0.0, 10.0, 3.0]]
            .map(|position| voxy_render::SkinnedVertex {
                position,
                normal: [0.0, 0.0, 1.0],
                uv: [0.0; 2],
                joints: [0; 4],
                weights: [65535, 0, 0, 0],
            })
            .to_vec(),
        vec![0, 1, 2],
        1,
    )?;
    let mut poses = voxy_render::SkinnedMotionHistory::new(mesh);
    let prior = poses.prepare_frame(
        &[glam::Mat4::from_translation(glam::Vec3::X)],
        glam::Mat4::IDENTITY,
        [1.0; 4],
    )?;
    assert!(prior.previous_reflection_triangles([0; 3], 0)?.is_empty());
    // Model the caller's successful-present acknowledgment for the prior pose.
    // This fixture is offscreen and does not claim native presentation occurred.
    poses.presented_frame(&prior)?;
    let skipped = poses.prepare_frame(
        &[glam::Mat4::from_translation(glam::Vec3::X * 7.0)],
        glam::Mat4::IDENTITY,
        [1.0; 4],
    )?;
    let candidate = poses.prepare_frame(&[glam::Mat4::IDENTITY], glam::Mat4::IDENTITY, [1.0; 4])?;
    let triangles = candidate.previous_reflection_triangles([0; 3], 0)?;
    assert_eq!(triangles[0].vertices[0], [-9.0, -10.0, 3.0, 1.0]);
    assert_eq!(
        skipped.previous_reflection_triangles([0; 3], 0)?[0].vertices,
        triangles[0].vertices
    );
    let previous = correspondence.prepare(reflected, &triangles, false)?;
    let reset = correspondence.prepare(reflected, &triangles, true)?;
    let pipeline = PlanarReflectionPipeline::new(device)?;
    let options = PlanarReflectionCameras {
        cameras: [camera; 2],
        planes: [[0.0, 0.0, 1.0, -0.5]; 2],
    };
    let guide = pipeline.prepare_reflection(reflected, &previous, options)?;
    let reset_guide = pipeline.prepare_reflection(reflected, &reset, options)?;
    let texture = |format| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ray-produced planar history fixture"),
            size: reflected.radiance().size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    };
    let history = texture(wgpu::TextureFormat::Rgba32Float);
    let depth = texture(wgpu::TextureFormat::R32Float);
    let clip = camera * glam::Vec4::new(0.0, 0.0, -2.0, 1.0);
    let virtual_depth = clip.z / clip.w;
    for (target, values, stride) in [
        (
            &history,
            (0..16)
                .flat_map(|i| {
                    [
                        1.0 + (i % 4) as f32,
                        2.0 + (i % 4) as f32,
                        3.0 + (i % 4) as f32,
                        1.0,
                    ]
                })
                .collect::<Vec<_>>(),
            16,
        ),
        (&depth, vec![virtual_depth; 16], 4),
    ] {
        let bytes: Vec<u8> = values.into_iter().flat_map(f32::to_le_bytes).collect();
        queue.write_texture(
            target.as_image_copy(),
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * stride),
                rows_per_image: Some(4),
            },
            target.size(),
        );
    }
    let resolver = TemporalResolve::new(device)?;
    let resolve = |motion, expected_previous_depth| {
        resolver.prepare(
            TemporalResolveInputs {
                current: reflected.radiance(),
                motion,
                history: &history,
                expected_previous_depth,
                history_depth: &depth,
            },
            TemporalResolveOptions {
                history_weight: 0.5,
                depth_tolerance: 0.001,
                reset_history: false,
            },
        )
    };
    let regular = resolve(guide.motion(), guide.expected_previous_depth())?;
    let rejected = resolve(reset_guide.motion(), reset_guide.expected_previous_depth())?;
    let retained = voxy_render::TemporalHistory::new(device, 4, 4)?;
    previous.encode(encoder);
    reset.encode(encoder);
    guide.encode(encoder);
    reset_guide.encode(encoder);
    regular.encode(encoder);
    rejected.encode(encoder);
    retained.encode_depth(encoder, guide.current_depth())?;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("actual mirror temporal proof"),
        size: 9216,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    for (block, texture) in [
        guide.motion(),
        guide.expected_previous_depth(),
        guide.current_depth(),
        reflected.radiance(),
        regular.output(),
        rejected.output(),
        reset_guide.expected_previous_depth(),
        retained.output_depth(),
    ]
    .into_iter()
    .enumerate()
    {
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: block as u64 * 1024,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(4),
                },
            },
            texture.size(),
        );
    }
    let pipeline = voxy_render::PlanarTemporalPipeline::new(device)?;
    let mut history = voxy_render::TemporalHistory::new(device, 4, 4)?;
    let old_color = history.color().clone();
    let settings = TemporalResolveOptions {
        history_weight: 0.5,
        depth_tolerance: 0.001,
        reset_history: false,
    };
    let unencoded = pipeline.prepare(
        reflected,
        &triangles,
        options,
        &mut history,
        settings,
        false,
    )?;
    assert!(matches!(
        unencoded.finish(voxy_render::RenderOutcome::Presented),
        Err(voxy_render::PlanarTemporalError::InvalidLifecycle)
    ));
    assert!(!history.valid());
    assert_eq!(history.color(), &old_color);
    let mut cancelled = pipeline.prepare(
        reflected,
        &triangles,
        options,
        &mut history,
        settings,
        false,
    )?;
    cancelled.encode(encoder)?;
    assert!(matches!(
        cancelled.encode(encoder),
        Err(voxy_render::PlanarTemporalError::InvalidLifecycle)
    ));
    assert!(!cancelled.finish(voxy_render::RenderOutcome::SkippedOccluded)?);
    assert!(!history.valid());
    assert_eq!(history.color(), &old_color);
    let mut accepted = pipeline.prepare(
        reflected,
        &triangles,
        options,
        &mut history,
        settings,
        false,
    )?;
    accepted.encode(encoder)?;
    encoder.copy_texture_to_buffer(
        accepted.output().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 8192,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        accepted.output().size(),
    );
    let accepted_color = accepted.output().clone();
    // Modeled successful presentation outcome for this offscreen fixture only.
    assert!(accepted.finish(voxy_render::RenderOutcome::Presented)?);
    assert!(history.valid());
    assert_eq!(history.color(), &accepted_color);
    let retained_color = history.color().clone();
    drop(pipeline.prepare(
        reflected,
        &triangles,
        options,
        &mut history,
        settings,
        false,
    )?);
    assert!(history.valid());
    assert_eq!(history.color(), &retained_color);
    Ok(readback)
}
pub fn verify(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    camera: glam::Mat4,
) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(std::time::Duration::from_secs(10)),
    })?;
    rx.recv_timeout(std::time::Duration::from_secs(10))??;
    let mapped = buffer.slice(..).get_mapped_range()?;
    let clip = camera * glam::Vec4::new(0.0, 0.0, -2.0, 1.0);
    let expected_depth = clip.z / clip.w;
    for y in 0..4 {
        for x in 0..4 {
            let read = |block: usize, stride: usize, channel: usize| {
                let offset = block * 1024 + y * 256 + x * stride + channel * 4;
                f32::from_le_bytes(mapped[offset..offset + 4].try_into().expect("four bytes"))
            };
            for (actual, expected) in [
                (read(0, 8, 0), if x < 3 { 0.25 } else { 0.0 }),
                (read(0, 8, 1), 0.0),
                (read(1, 4, 0), if x < 3 { expected_depth } else { 0.0 }),
                (read(2, 4, 0), expected_depth),
                (read(6, 4, 0), 0.0),
                (read(7, 4, 0), expected_depth),
            ] {
                assert!(
                    actual.is_finite() && (actual - expected).abs() < 1e-5,
                    "ray planar guide ({x},{y}): {actual} != {expected}"
                );
            }
            for c in 0..3 {
                let offset = 3072 + y * 256 + x * 8 + c * 2;
                let current = half::f16::from_bits(u16::from_le_bytes(
                    mapped[offset..offset + 2].try_into()?,
                ))
                .to_f32();
                let expected_color = if x < 3 {
                    (current + 1.0 + (x + 1 + c) as f32) * 0.5
                } else {
                    current
                };
                assert!(
                    (read(4, 16, c) - expected_color).abs() < 1e-5,
                    "actual reflected temporal blend differs"
                );
                assert!(
                    (read(5, 16, c) - current).abs() < 1e-5,
                    "reset accepted reflection history"
                );
                assert!(
                    (read(8, 16, c) - current).abs() < 1e-5,
                    "first wrapped frame sampled invalid history"
                );
            }
        }
    }
    drop(mapped);
    buffer.unmap();
    println!(
        "RAY PLANAR TEMPORAL PASS: 16 actual ray-produced mirror hits/radiance, prior geometry -> one-pixel backward UV/history lookup, virtual depths -> temporal blend, reset rejection, pending depth copy and ordered-frame lifecycle; modeled outcomes, no presentation proof"
    );
    Ok(())
}
