use glam::Mat4;
use voxy_render::{
    PrimaryMotionPass, PrimarySurfaceJob, RayReconstructionGuides, ReconstructionGuideMesh,
    ReconstructionGuidePass, ReconstructionGuideVertex,
};

// Partial coverage exercises the reserved background ID and invalid primary pixels.
#[allow(clippy::too_many_lines)]
pub fn probe(
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    let guides = RayReconstructionGuides::new(device, adapter, 4, 4)?;
    let pass = ReconstructionGuidePass::for_guides(device, &guides);
    let inputs = pass.primary_inputs(device, Mat4::IDENTITY, [0.0, 0.0, 3.0], &guides)?;
    let vertices = [
        [-1.0, -1.0, 0.5],
        [0.0, -1.0, 0.5],
        [0.0, 1.0, 0.5],
        [-1.0, -1.0, 0.5],
        [0.0, 1.0, 0.5],
        [-1.0, 1.0, 0.5],
    ]
    .map(|position| ReconstructionGuideVertex::new(position, [0.0, 0.0, 1.0], [0.5; 3], 0.0, 0.0));
    let vertices = vertices.into_iter().collect::<Result<Vec<_>, _>>()?;
    let mesh = ReconstructionGuideMesh::upload(device, &vertices)?;
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("partial coverage depth"),
        size: guides.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    let primary = PrimarySurfaceJob::new(
        device,
        &depth,
        guides.normal_roughness(),
        Mat4::IDENTITY,
        1.0,
    )?;
    let motion = PrimaryMotionPass::for_objects(
        device,
        &primary,
        [Mat4::IDENTITY; 2],
        &[[
            Mat4::IDENTITY,
            Mat4::from_translation(glam::Vec3::new(0.2, 0.0, 0.0)),
        ]],
        guides.object_ids(),
        false,
    )?;
    let previous_camera = Mat4::from_translation(glam::Vec3::new(-0.2, 0.1, 0.0));
    let camera_motion =
        PrimaryMotionPass::new(device, &primary, Mat4::IDENTITY, previous_camera, false)?;
    let reset_motion = PrimaryMotionPass::for_objects(
        device,
        &primary,
        [Mat4::IDENTITY, previous_camera],
        &[[Mat4::IDENTITY, Mat4::from_translation(glam::Vec3::X)]],
        guides.object_ids(),
        true,
    )?;
    let combined_motion = PrimaryMotionPass::for_objects(
        device,
        &primary,
        [Mat4::IDENTITY, previous_camera],
        &[[
            Mat4::IDENTITY,
            Mat4::from_translation(glam::Vec3::new(0.2, 0.0, 0.0)),
        ]],
        guides.object_ids(),
        false,
    )?;
    let affine_motion = PrimaryMotionPass::for_objects(
        device,
        &primary,
        [Mat4::IDENTITY; 2],
        &[[
            Mat4::from_scale(glam::Vec3::new(2.0, 0.5, 1.0)),
            Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2)
                * Mat4::from_scale(glam::Vec3::new(0.5, 2.0, 1.0)),
        ]],
        guides.object_ids(),
        false,
    )?;
    let skinned_vertices = [
        [-1.0, -1.0, 0.5],
        [0.0, -1.0, 0.5],
        [0.0, 1.0, 0.5],
        [-1.0, -1.0, 0.5],
        [0.0, 1.0, 0.5],
        [-1.0, 1.0, 0.5],
    ]
    .map(|position| voxy_render::SkinnedVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        uv: [0.0; 2],
        joints: [0; 4],
        weights: [65535, 0, 0, 0],
    });
    let skin = voxy_render::SkinnedMesh::new(skinned_vertices.to_vec(), (0..6).collect(), 1)?;
    let previous_joint = Mat4::from_cols(
        glam::Vec4::new(1.0, 0.1, 0.0, 0.0),
        glam::Vec4::new(0.2, 1.0, 0.0, 0.0),
        glam::Vec4::Z,
        glam::Vec4::W,
    );
    let mut skin_history = voxy_render::SkinnedMotionHistory::new(skin);
    skin_history.presented(&[previous_joint], Mat4::IDENTITY)?;
    // Preparing a skipped intermediate pose must not replace the presented one.
    skin_history.prepare(&[Mat4::from_translation(glam::Vec3::X)], Mat4::IDENTITY)?;
    let skin_frame = skin_history.prepare(&[Mat4::IDENTITY], Mat4::IDENTITY)?;
    assert!(skin_frame.history_valid);
    let correspondence_vertices = skin_frame.vertices;
    let correspondence = voxy_render::PreviousPositionPass::new(
        device,
        &depth,
        Mat4::IDENTITY,
        &correspondence_vertices,
    )?;
    let deformation_motion = PrimaryMotionPass::for_previous_positions(
        device,
        &primary,
        [Mat4::IDENTITY; 2],
        correspondence.output(),
        false,
    )?;
    let raster_motion = voxy_render::RasterMotionPass::new(
        device,
        &depth,
        [Mat4::IDENTITY; 2],
        &correspondence_vertices,
        false,
    )?;
    let depth_motion = voxy_render::DepthMotionPass::new(
        device,
        &depth,
        [Mat4::IDENTITY, previous_camera],
        1.0,
        false,
    )?;
    let depth_reset = voxy_render::DepthMotionPass::new(
        device,
        &depth,
        [Mat4::IDENTITY, previous_camera],
        1.0,
        true,
    )?;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("partial coverage verification"),
        size: 10752,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    pass.encode(&mut encoder, &guides, &view, &inputs, &[&mesh])?;
    pass.encode_object_ids(device, &mut encoder, &guides, &view, &inputs, &[(&mesh, 0)])?;
    primary.encode(&mut encoder);
    motion.encode(&mut encoder);
    camera_motion.encode(&mut encoder);
    reset_motion.encode(&mut encoder);
    combined_motion.encode(&mut encoder);
    affine_motion.encode(&mut encoder);
    correspondence.encode(&mut encoder);
    deformation_motion.encode(&mut encoder);
    raster_motion.encode(&mut encoder);
    depth_motion.encode(&mut encoder);
    depth_reset.encode(&mut encoder);
    for (texture, offset) in [
        (guides.object_ids(), 0),
        (motion.output(), 1024),
        (camera_motion.output(), 2048),
        (reset_motion.output(), 3072),
        (combined_motion.output(), 4096),
        (affine_motion.output(), 5120),
        (deformation_motion.output(), 6144),
        (raster_motion.output(), 7168),
        (depth_motion.output(), 8192),
        (depth_reset.output(), 9216),
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
    encoder.copy_buffer_to_buffer(primary.output(), 0, &readback, 10240, 512);
    let index = queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(index),
        timeout: Some(std::time::Duration::from_secs(5)),
    })?;
    receiver.recv_timeout(std::time::Duration::from_secs(1))??;
    let bytes = readback.slice(..).get_mapped_range()?;
    for pixel in 0..16 {
        let column = pixel % 4;
        let row = pixel / 4;
        let offset = row * 256 + column * 4;
        let id = u32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
        let foreground = column < 2;
        assert_eq!(id, if foreground { 0 } else { u32::MAX });
        let values: Vec<_> = bytes[10240 + pixel * 32..10240 + (pixel + 1) * 32]
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
            .collect();
        if foreground {
            let x = [-0.75, -0.25][column];
            let y = [0.75, 0.25, -0.25, -0.75][row];
            assert_eq!(values, [x, y, 0.5, 1.0, 0.0, 0.0, 1.0, 0.0]);
        } else {
            assert_eq!(values, [0.0; 8]);
        }
        let x = [-0.75, -0.25, 0.25, 0.75][column];
        let y = [0.75, 0.25, -0.25, -0.75][row];
        for (plane, expected) in [
            (1024, [0.1, 0.0]),
            (2048, [-0.1, -0.05]),
            (3072, [0.0, 0.0]),
            (4096, [0.0, -0.05]),
            (5120, [(-4.0 * y - x) * 0.5, (y - 0.25 * x) * 0.5]),
            (6144, [0.1 * y, -0.05 * x]),
            (7168, [0.1 * y, -0.05 * x]),
            (8192, [-0.1, -0.05]),
            (9216, [0.0, 0.0]),
        ] {
            for (component, expected) in expected.into_iter().enumerate() {
                let start = plane + offset + component * 2;
                let value =
                    half::f16::from_bits(u16::from_le_bytes(bytes[start..start + 2].try_into()?))
                        .to_f32();
                let expected = if foreground { expected } else { 0.0 };
                assert!(
                    value.is_finite()
                        && (value - expected).abs()
                            < if plane == 6144 && adapter.get_info().backend == wgpu::Backend::Gl {
                                0.0005
                            } else {
                                0.0001
                            },
                    "pixel {pixel}, plane {plane}, component {component}: {value} != {expected}"
                );
            }
        }
    }
    drop(bytes);
    readback.unmap();
    println!(
        "PRIMARY BACKGROUND PASS: reserved IDs, invalid surfaces, camera/object motion and reset with zero background motion"
    );
    Ok(())
}
