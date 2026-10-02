//! Perspective and affine-object temporal correspondence GPU acceptance.
use glam::{Mat4, Vec3};
use voxy_render::{PrimaryMotionPass, PrimarySurfaceJob};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: adapter.limits(),
        ..Default::default()
    }))?;
    println!("Perspective motion GPU: {:?}", adapter.get_info());
    let texture = |format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("perspective correspondence fixture"),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let depth = texture(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let normal = texture(
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let normals: Vec<u16> = (0..16)
        .flat_map(|_| [0, 0, half::f16::ONE.to_bits(), 0])
        .collect();
    queue.write_texture(
        normal.as_image_copy(),
        bytemuck::cast_slice(&normals),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(32),
            rows_per_image: Some(4),
        },
        normal.size(),
    );
    let ids = texture(
        wgpu::TextureFormat::R32Uint,
        wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let id_values: Vec<u32> = (0..16)
        .map(|i| if i % 4 == 3 { 99 } else { (i % 4) / 2 })
        .collect();
    queue.write_texture(
        ids.as_image_copy(),
        bytemuck::cast_slice(&id_values),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16),
            rows_per_image: Some(4),
        },
        ids.size(),
    );
    let camera =
        glam::camera::rh::proj::directx::perspective(std::f32::consts::FRAC_PI_2, 1.0, 1.0, 10.0);
    let primary = PrimarySurfaceJob::new(&device, &depth, &normal, camera, 1.0)?;
    let models = [
        [
            Mat4::from_scale(Vec3::new(1.5, 0.75, 1.0)),
            Mat4::from_translation(Vec3::new(0.125, -0.25, -0.5)) * Mat4::from_rotation_z(0.3),
        ],
        [
            Mat4::from_rotation_z(-0.2),
            Mat4::from_translation(Vec3::new(-0.25, 0.125, -1.0))
                * Mat4::from_scale(Vec3::new(0.7, 1.2, 1.0)),
        ],
    ];
    let passes = [
        PrimaryMotionPass::new(&device, &primary, camera, camera, false)?,
        PrimaryMotionPass::for_objects(&device, &primary, [camera; 2], &models, &ids, false)?,
        PrimaryMotionPass::for_objects(&device, &primary, [camera; 2], &models, &ids, true)?,
    ];
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("perspective motion readback"),
        size: 6144,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let view = depth.create_view(&Default::default());
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(5.0 / 9.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
    }
    primary.encode(&mut encoder);
    for (index, pass) in passes.iter().enumerate() {
        pass.encode(&mut encoder);
        for (slot, source) in [pass.output(), pass.previous_depth()]
            .into_iter()
            .enumerate()
        {
            encoder.copy_texture_to_buffer(
                source.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: (index * 2048 + slot * 1024) as u64,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(4),
                    },
                },
                source.size(),
            );
        }
    }
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let bytes = readback.slice(..).get_mapped_range()?;
    for index in 0..3 {
        for row in 0..4 {
            for column in 0..4 {
                let uv = [(column as f32 + 0.5) / 4.0, (row as f32 + 0.5) / 4.0];
                // Independent pinhole plane reference: 90-degree field, z=-2.
                let world = Vec3::new((uv[0] * 2.0 - 1.0) * 2.0, (1.0 - uv[1] * 2.0) * 2.0, -2.0);
                let previous = if index == 1 && column != 3 {
                    let model = models[column / 2];
                    model[1].transform_point3(model[0].inverse().transform_point3(world))
                } else {
                    world
                };
                let rejected = index == 2 || (index == 1 && column == 3);
                let expected = if rejected {
                    [0.0; 3]
                } else {
                    [
                        (previous.x / -previous.z + 1.0) * 0.5 - uv[0],
                        (1.0 - previous.y / -previous.z) * 0.5 - uv[1],
                        10.0 / 9.0 + 10.0 / (9.0 * previous.z),
                    ]
                };
                let offset = index * 2048 + row * 256 + column * 4;
                let actual = [
                    half::f16::from_bits(u16::from_le_bytes(bytes[offset..offset + 2].try_into()?))
                        .to_f32(),
                    half::f16::from_bits(u16::from_le_bytes(
                        bytes[offset + 2..offset + 4].try_into()?,
                    ))
                    .to_f32(),
                    f32::from_le_bytes(bytes[offset + 1024..offset + 1028].try_into()?),
                ];
                for channel in 0..3 {
                    assert!(
                        actual[channel].is_finite()
                            && (actual[channel] - expected[channel]).abs() < 0.0003,
                        "case={index} pixel={column},{row} channel={channel}: {} != {}",
                        actual[channel],
                        expected[channel]
                    );
                }
            }
        }
    }
    println!(
        "PERSPECTIVE MOTION PASS: 144 UV/depth references, affine rotation/nonuniform scale, unknown IDs and reset"
    );
    Ok(())
}
