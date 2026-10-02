//! Camera-relative correspondence for worlds exceeding the GL half-float range.
use glam::{Mat4, Vec3};
use voxy_render::{
    PreviousPositionPass, PreviousPositionVertex, PrimaryMotionPass, PrimarySurfaceJob,
    RayReconstructionGuides, ReconstructionGuideMesh, ReconstructionGuidePass,
    ReconstructionGuideVertex,
};
#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: adapter.limits(),
        ..Default::default()
    }))?;
    println!("Relative motion GPU: {:?}", adapter.get_info());
    let origin = Vec3::new(100_000.0, 200_000.0, 0.0);
    let camera = Mat4::from_translation(-origin);
    let depth_camera = Mat4::from_translation(Vec3::new(0.125, 0.125, 0.0)) * camera;
    let guides = RayReconstructionGuides::new(&device, &adapter, 4, 4)?;
    let pass = ReconstructionGuidePass::for_guides(&device, &guides);
    let inputs = pass.primary_inputs(
        &device,
        depth_camera,
        (origin + Vec3::Z * 3.0).to_array(),
        &guides,
    )?;
    let positions = [
        [-1.0, -1.0, 0.5],
        [0.0, -1.0, 0.5],
        [0.0, 1.0, 0.5],
        [-1.0, -1.0, 0.5],
        [0.0, 1.0, 0.5],
        [-1.0, 1.0, 0.5],
    ]
    .map(|p| (origin + Vec3::from_array(p)).to_array());
    let vertices = positions
        .map(|p| ReconstructionGuideVertex::new(p, [0.0, 0.0, 1.0], [0.5; 3], 0.0, 0.0))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let mesh = ReconstructionGuideMesh::upload(&device, &vertices)?;
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("relative motion depth"),
        size: guides.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let paired = positions.map(|current| PreviousPositionVertex {
        current,
        previous: (Vec3::from_array(current) + Vec3::new(0.25, 0.0, 0.125)).to_array(),
    });
    if adapter.get_info().backend == wgpu::Backend::Gl {
        assert!(PreviousPositionPass::new(&device, &depth, depth_camera, &paired).is_err());
    }
    assert!(
        PreviousPositionPass::new_relative(&device, &depth, depth_camera, &paired, [f32::NAN; 3])
            .is_err()
    );
    let correspondence = PreviousPositionPass::new_relative(
        &device,
        &depth,
        depth_camera,
        &paired,
        origin.to_array(),
    )?;
    let primary = PrimarySurfaceJob::new(
        &device,
        &depth,
        guides.normal_roughness(),
        depth_camera,
        1.0,
    )?;
    let motion = PrimaryMotionPass::for_previous_positions_relative(
        &device,
        &primary,
        [camera; 2],
        correspondence.output(),
        correspondence.position_origin(),
        false,
    )?;
    let zoom = voxy_render::DepthMotionPass::with_jittered_depth(
        &device,
        &depth,
        depth_camera,
        [camera, Mat4::from_scale(Vec3::new(2.0, 1.0, 1.0)) * camera],
        1.0,
        false,
    )?;
    let reset_motion = PrimaryMotionPass::for_previous_positions_relative(
        &device,
        &primary,
        [camera; 2],
        correspondence.output(),
        correspondence.position_origin(),
        true,
    )?;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("relative motion readback"),
        size: 4096,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    pass.encode(
        &mut encoder,
        &guides,
        &depth.create_view(&wgpu::TextureViewDescriptor::default()),
        &inputs,
        &[&mesh],
    )?;
    correspondence.encode(&mut encoder);
    primary.encode(&mut encoder);
    motion.encode(&mut encoder);
    reset_motion.encode(&mut encoder);
    zoom.encode(&mut encoder);
    encoder.copy_texture_to_buffer(
        motion.output().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        guides.size(),
    );
    encoder.copy_texture_to_buffer(
        zoom.output().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 1024,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        guides.size(),
    );
    for (offset, texture) in [
        (2048, motion.previous_depth()),
        (3072, reset_motion.previous_depth()),
    ] {
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(4),
                },
            },
            guides.size(),
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
    for row in 0..4 {
        for column in 0..4 {
            let offset = row * 256 + column * 4;
            for (base, expected) in [
                (2048, if column < 2 { 0.625_f32 } else { 0.0 }),
                (3072, 0.0),
            ] {
                let half_depth = motion.previous_depth().format() == wgpu::TextureFormat::R16Float;
                let start = base + row * 256 + column * if half_depth { 2 } else { 4 };
                let value = if half_depth {
                    half::f16::from_bits(u16::from_le_bytes(bytes[start..start + 2].try_into()?))
                        .to_f32()
                } else {
                    f32::from_le_bytes(bytes[start..start + 4].try_into()?)
                };
                assert!(
                    (value - expected).abs() < 0.0001,
                    "previous depth {base}/{row}/{column}: {value} vs {expected}"
                );
            }
            let x = half::f16::from_bits(u16::from_le_bytes(bytes[offset..offset + 2].try_into()?))
                .to_f32();
            let y = half::f16::from_bits(u16::from_le_bytes(
                bytes[offset + 2..offset + 4].try_into()?,
            ))
            .to_f32();
            let expected = if column < 2 { 0.125 } else { 0.0 };
            assert!(
                x.is_finite() && (x - expected).abs() < 0.0001 && y.abs() < 0.0001,
                "{row},{column}: {x},{y}"
            );
            let offset = 1024 + offset;
            let x = half::f16::from_bits(u16::from_le_bytes(bytes[offset..offset + 2].try_into()?))
                .to_f32();
            let y = half::f16::from_bits(u16::from_le_bytes(
                bytes[offset + 2..offset + 4].try_into()?,
            ))
            .to_f32();
            let expected = [-0.4375, -0.1875, 0.0, 0.0][column];
            assert!(
                x.is_finite() && (x - expected).abs() < 0.0001 && y.abs() < 0.0001,
                "jitter zoom {row},{column}: {x},{y}"
            );
        }
    }
    drop(bytes);
    readback.unmap();
    let constant = |format, value: &[f32]| {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("correspondence resolve fixture"),
            size: guides.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(&value.repeat(16)),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some((value.len() * 16) as u32),
                rows_per_image: Some(4),
            },
            guides.size(),
        );
        texture
    };
    let current = constant(wgpu::TextureFormat::Rgba32Float, &[2., 2., 2., 1.]);
    let history = constant(wgpu::TextureFormat::Rgba32Float, &[10., 10., 10., 1.]);
    let good_depth = constant(wgpu::TextureFormat::R32Float, &[0.625]);
    let wrong_depth = constant(wgpu::TextureFormat::R32Float, &[0.5]);
    let resolver = voxy_render::TemporalResolve::new(&device)?;
    for (case, producer, history_depth) in [
        (0, &motion, &good_depth),
        (1, &motion, &wrong_depth),
        (2, &reset_motion, &good_depth),
    ] {
        let frame = resolver.prepare(
            voxy_render::TemporalResolveInputs {
                current: &current,
                motion: producer.output(),
                history: &history,
                expected_previous_depth: producer.previous_depth(),
                history_depth,
            },
            voxy_render::TemporalResolveOptions {
                history_weight: 0.5,
                depth_tolerance: 0.001,
                reset_history: false,
            },
        )?;
        let mut encoder = device.create_command_encoder(&Default::default());
        frame.encode(&mut encoder);
        encoder.copy_texture_to_buffer(
            frame.output().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(4),
                },
            },
            guides.size(),
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let bytes = readback.slice(..).get_mapped_range()?;
        for y in 0..4 {
            for x in 0..4 {
                for c in 0..3 {
                    let offset = y * 256 + x * 16 + c * 4;
                    let actual = f32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
                    let expected = if case == 0 && x < 2 { 6.0_f32 } else { 2.0 };
                    assert!(
                        (actual - expected).abs() < 0.0001,
                        "correspondence resolve {case}/{x}/{y}/{c}: {actual} vs {expected}"
                    );
                }
            }
        }
        drop(bytes);
        readback.unmap();
    }
    let mut owner = voxy_render::TemporalHistory::new(&device, 4, 4)?;
    for step in 0..3 {
        if step == 2 {
            owner.reset();
        }
        let frame = owner.prepare_resolve(
            &resolver,
            if step == 0 { &history } else { &current },
            motion.output(),
            motion.previous_depth(),
            voxy_render::TemporalResolveOptions {
                history_weight: 0.5,
                depth_tolerance: 0.001,
                reset_history: false,
            },
            false,
        )?;
        let mut encoder = device.create_command_encoder(&Default::default());
        frame.encode(&mut encoder);
        owner.encode_depth(&mut encoder, &good_depth)?;
        encoder.copy_texture_to_buffer(
            frame.output().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(4),
                },
            },
            guides.size(),
        );
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let bytes = readback.slice(..).get_mapped_range()?;
        for y in 0..4 {
            for x in 0..4 {
                for c in 0..3 {
                    let offset = y * 256 + x * 16 + c * 4;
                    let actual = f32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
                    let expected = match step {
                        0 => 10.0,
                        1 if x < 2 => 6.0,
                        _ => 2.0,
                    };
                    assert!(
                        (actual - expected).abs() < 0.0001,
                        "history owner {step}/{x}/{y}/{c}: {actual} vs {expected}"
                    );
                }
            }
        }
        drop(bytes);
        readback.unmap();
        if step != 1 {
            owner.presented();
        } // Simulated caller confirmation; failed second frame is not committed.
    }
    println!(
        "RELATIVE MOTION PASS: local correspondence, previous depth, temporal rejection and retained history resolve"
    );
    Ok(())
}
