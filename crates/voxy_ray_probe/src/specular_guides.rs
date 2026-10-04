#[path = "planar_temporal.rs"]
mod planar_temporal;
use voxy_render::{
    MirrorSurfaceSample, PrimaryMotionPass, PrimarySurfaceJob, RadianceComposition,
    RayReconstructionGuides, RayScene, ReconstructionDistancePass, ReconstructionGuideMesh,
    ReconstructionGuidePass, ReconstructionGuideVertex, ReconstructionMaterial,
    SpecularDistancePipeline, SurfaceLightingJob, SurfacePointLight, SurfaceReflectionJob,
    SurfaceReflectionOptions,
};

pub fn probe(
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    probe_case(device, adapter, queue, false)?;
    probe_case(device, adapter, queue, true)
}

fn probe_case(
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    queue: &wgpu::Queue,
    occluded: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let scene = RayScene::new(
        device,
        &[[-10.0, -10.0, 3.0], [10.0, -10.0, 3.0], [0.0, 10.0, 3.0]],
    )?;
    let (job, colors, expected) = reference_reflection(device, &scene)?;
    let guides = RayReconstructionGuides::new(device, adapter, 4, 4)?;
    let pass = ReconstructionGuidePass::for_guides(device, &guides);
    let camera = primary_camera();
    let inputs = pass.primary_inputs(device, camera, [0.0, 0.0, 3.0], &guides)?;
    let mesh = primary_mesh(device)?;
    let depth = primary_depth(device, &guides);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    scene.build(&mut encoder);
    pass.encode(
        &mut encoder,
        &guides,
        &depth.create_view(&wgpu::TextureViewDescriptor::default()),
        &inputs,
        &[&mesh],
    )?;
    pass.encode_material_f0(
        &mut encoder,
        &guides,
        &depth.create_view(&wgpu::TextureViewDescriptor::default()),
        &inputs,
        &[&mesh],
    )?;
    encode_geometry_ids(device, &pass, &guides, &depth, &inputs, &mut encoder)?;
    let (surface, primary) = encode_primary(
        device,
        &depth,
        guides.normal_roughness(),
        camera,
        &mut encoder,
    )?;
    let motion = encode_motion(
        device,
        queue,
        &surface,
        camera,
        guides.object_ids(),
        &mut encoder,
    )?;
    let mut rough_proofs = Vec::new();
    for roughness in [0.2, 0.5, 1.0] {
        for seed in [0, 17, 31] {
            let proof = rough_gpu(
                device,
                queue,
                &scene,
                &depth,
                guides.material_f0(),
                &mut encoder,
                (roughness, seed),
            )?;
            rough_proofs.push((proof, roughness, seed));
        }
    }
    let ray_frame = voxy_render::RayLightingFrame::new(
        device,
        &scene,
        voxy_render::RayLightingInputs {
            depth: &depth,
            normal_roughness: guides.normal_roughness(),
            diffuse: guides.diffuse_albedo(),
            mirror_f0: guides.material_f0(),
            view_projection: camera,
            clear_depth: 1.0,
        },
        SurfacePointLight {
            position: [0.0, 0.0, if occluded { 4.0 } else { 2.0 }],
            intensity: [10.0; 3],
            bias: 0.001,
        },
        &[[4.0, 1.0, 0.5, 1.0]],
        SurfaceReflectionOptions {
            material: ReconstructionMaterial::new([0.0; 3], 1.0, 0.0)?,
            camera: [0.0, 0.0, 3.0],
            bias: 0.001,
            maximum_distance: 10.0,
        },
    )?;
    ray_frame.encode(&mut encoder);
    let planar_proof = planar_temporal::encode(device, queue, &ray_frame, camera, &mut encoder)?;
    ReconstructionDistancePass::new(device, &guides, ray_frame.reflection_distance())?
        .encode(&mut encoder);
    job.encode(&mut encoder); // Independent CPU-sample reference, after the GPU lighting chain.
    verify_distances(
        device,
        queue,
        encoder,
        [
            job.output(),
            guides.specular_hit_distance(),
            job.incident_radiance(),
            ray_frame.output(),
            ray_frame.reflection_distance(),
            ray_frame.reflected_radiance(),
            guides.material_f0(),
        ],
        &expected,
        &colors,
        occluded,
    )?;
    planar_temporal::verify(device, &planar_proof, camera)?;
    for (proof, roughness, seed) in &rough_proofs {
        verify_rough_gpu(device, proof, *roughness, *seed)?;
    }
    verify_primary(device, &primary)?;
    verify_motion(device, &motion)?;
    println!(
        "GPU primary reflection -> HDR composition / material MRT: all 16 world-space distances and material-weighted HDR colors and composed ray-shadowed point light match; occluded={occluded}"
    );
    Ok(())
}

fn verify_distances(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    textures: [&wgpu::Texture; 7],
    expected: &[f32],
    colors: &[[f32; 4]],
    occluded: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ray/guide distance readback"),
        size: 7168,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    for (index, texture) in textures.into_iter().enumerate() {
        if index < 2 && texture.format() != wgpu::TextureFormat::R32Float {
            return Err("this proof requires R32Float guide distances".into());
        }
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: u64::try_from(index)? * 1024,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(4),
                },
            },
            texture.size(),
        );
    }
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
    let mapped = readback.slice(..).get_mapped_range()?;
    for plane in mapped[..2048].chunks_exact(1024) {
        for (bytes, expected) in plane
            .chunks_exact(256)
            .flat_map(|row| row[..16].chunks_exact(4))
            .zip(expected)
        {
            let actual = f32::from_le_bytes(bytes.try_into()?);
            if !actual.is_finite() || (actual - expected).abs() > 0.0001 {
                return Err(format!("ray/guide distance mismatch: {actual} != {expected}").into());
            }
        }
    }
    verify_color(&mapped[2048..3072], colors)?;
    let combined: Vec<_> = colors
        .iter()
        .enumerate()
        .map(|(index, c)| {
            let x = f32::from(u16::try_from(index % 4).unwrap()) * 0.5 - 0.75;
            let y = 0.75 - f32::from(u16::try_from(index / 4).unwrap()) * 0.5;
            let squared = x * x + y * y + 2.25;
            let irradiance = if occluded {
                0.0
            } else {
                15.0 / (std::f32::consts::PI * squared * squared.sqrt())
            };
            let base = primary_base(x, y);
            [
                c[0] + base[0] * 0.5 * irradiance,
                c[1] + base[1] * 0.5 * irradiance,
                c[2] + base[2] * 0.5 * irradiance,
                1.0,
            ]
        })
        .collect();
    verify_color(&mapped[3072..4096], &combined)?;
    verify_color(&mapped[5120..6144], colors)?;
    for (bytes, expected) in mapped[4096..5120]
        .chunks_exact(256)
        .flat_map(|row| row[..16].chunks_exact(4))
        .zip(expected)
    {
        let actual = f32::from_le_bytes(bytes.try_into()?);
        if !actual.is_finite() || (actual - expected).abs() > 0.0001 {
            return Err(format!("GPU primary reflection distance {actual} != {expected}").into());
        }
    }
    let f0: Vec<_> = (0_u16..16)
        .map(|i| {
            let base = primary_base(f32::from(i % 4) * 0.5 - 0.75, 0.75 - f32::from(i / 4) * 0.5);
            [
                0.02 + base[0] * 0.5,
                0.02 + base[1] * 0.5,
                0.02 + base[2] * 0.5,
                1.0,
            ]
        })
        .collect();
    verify_color(&mapped[6144..], &f0)?;
    drop(mapped);
    readback.unmap();
    Ok(())
}

fn verify_color(plane: &[u8], colors: &[[f32; 4]]) -> Result<(), Box<dyn std::error::Error>> {
    for (pixel, expected) in plane
        .chunks_exact(256)
        .flat_map(|row| row[..32].chunks_exact(8))
        .zip(colors)
    {
        for (bytes, expected) in pixel.chunks_exact(2).zip(expected) {
            let actual = half::f16::from_bits(u16::from_le_bytes(bytes.try_into()?)).to_f32();
            if !actual.is_finite() || (actual - expected).abs() > 0.003 {
                return Err(format!("mirror color mismatch {actual} != {expected}").into());
            }
        }
    }
    Ok(())
}

fn encode_primary(
    device: &wgpu::Device,
    depth: &wgpu::Texture,
    normals: &wgpu::Texture,
    camera: glam::Mat4,
    encoder: &mut wgpu::CommandEncoder,
) -> Result<(PrimarySurfaceJob, wgpu::Buffer), Box<dyn std::error::Error>> {
    let reconstruction = voxy_render::PrimarySurfacePipeline::new(device)?;
    let initial = reconstruction.prepare(depth, normals, glam::Mat4::IDENTITY, 1.0)?;
    let storage = initial.output().clone();
    initial.encode(encoder);
    let job = reconstruction.prepare_reusing(initial, depth, normals, camera, 1.0)?;
    assert_eq!(job.output(), &storage, "primary storage was not reused");
    let resized_depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("primary resize depth"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let resized_normals = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("primary resize normals"),
        size: resized_depth.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let resize_source = reconstruction.prepare(depth, normals, camera, 1.0)?;
    let previous_storage = resize_source.output().clone();
    let resized = reconstruction.prepare_reusing(
        resize_source,
        &resized_depth,
        &resized_normals,
        camera,
        1.0,
    )?;
    assert_ne!(
        resized.output(),
        &previous_storage,
        "resize reused incompatible storage"
    );
    assert_eq!(resized.dimensions(), [1, 1]);
    assert_eq!(resized.output().size(), 32);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("primary surface proof"),
        size: 512,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    job.encode(encoder);
    encoder.copy_buffer_to_buffer(job.output(), 0, &readback, 0, 512);
    Ok((job, readback))
}

fn verify_primary(
    device: &wgpu::Device,
    readback: &wgpu::Buffer,
) -> Result<(), Box<dyn std::error::Error>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(std::time::Duration::from_secs(5)),
    })?;
    receiver.recv_timeout(std::time::Duration::from_secs(1))??;
    let mapped = readback.slice(..).get_mapped_range()?;
    for (index, sample) in mapped.chunks_exact(32).enumerate() {
        let x = f32::from(u16::try_from(index % 4)?) * 0.5 - 0.75;
        let y = 0.75 - f32::from(u16::try_from(index / 4)?) * 0.5;
        for (bytes, expected) in sample
            .chunks_exact(4)
            .zip([x, y, 0.5, 1.0, 0.0, 0.0, 1.0, 0.0])
        {
            let actual = f32::from_le_bytes(bytes.try_into()?);
            if !actual.is_finite() || (actual - expected).abs() > 0.0001 {
                return Err(
                    format!("primary surface reconstruction {actual} != {expected}").into(),
                );
            }
        }
    }
    drop(mapped);
    readback.unmap();
    println!("Primary depth -> world position/normal: all 16 GPU samples match");
    Ok(())
}

fn primary_camera() -> glam::Mat4 {
    let view = glam::camera::rh::view::look_at_mat4(
        glam::Vec3::new(0.0, 0.0, 3.0),
        glam::Vec3::ZERO,
        glam::Vec3::Y,
    );
    let projection =
        glam::camera::rh::proj::directx::perspective(2.0 * (1.0_f32 / 2.5).atan(), 1.0, 0.1, 10.0);
    projection * view
}

fn primary_mesh(
    device: &wgpu::Device,
) -> Result<ReconstructionGuideMesh, Box<dyn std::error::Error>> {
    let vertices = [[-1.0, -1.0, 0.5], [3.0, -1.0, 0.5], [-1.0, 3.0, 0.5]]
        .into_iter()
        .zip([[0.8, 0.4, 0.2], [0.2, 0.8, 0.4], [0.4, 0.2, 0.8]])
        .map(|(position, base)| {
            ReconstructionGuideVertex::new(position, [0.0, 0.0, 1.0], base, 0.5, 0.0)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ReconstructionGuideMesh::upload(device, &vertices)?)
}

fn primary_depth(device: &wgpu::Device, guides: &RayReconstructionGuides) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("specular guide depth"),
        size: guides.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

// Analytic barycentric interpolation on the constant-depth primary triangle.
fn primary_base(x: f32, y: f32) -> [f32; 3] {
    let right = (x + 1.0) * 0.25;
    let top = (y + 1.0) * 0.25;
    let origin = 1.0 - right - top;
    [
        0.8 * origin + 0.2 * right + 0.4 * top,
        0.4 * origin + 0.8 * right + 0.2 * top,
        0.2 * origin + 0.4 * right + 0.8 * top,
    ]
}

fn encode_motion(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    primary: &PrimarySurfaceJob,
    camera: glam::Mat4,
    object_ids: &wgpu::Texture,
    encoder: &mut wgpu::CommandEncoder,
) -> Result<wgpu::Buffer, Box<dyn std::error::Error>> {
    let previous = camera * glam::Mat4::from_translation(glam::Vec3::new(-0.2, 0.1, 0.0));
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("primary camera motion proof"),
        size: 6144,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    for (reset, offset) in [(false, 0), (true, 1024)] {
        let pass = PrimaryMotionPass::new(device, primary, camera, previous, reset)?;
        pass.encode(encoder);
        encoder.copy_texture_to_buffer(
            pass.output().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(4),
                },
            },
            pass.output().size(),
        );
    }
    let models = [
        glam::Mat4::from_translation(glam::Vec3::new(0.3, 0.0, 0.0)),
        glam::Mat4::from_translation(glam::Vec3::new(0.1, 0.0, 0.0)),
    ];
    for (reset, offset) in [(false, 2048), (true, 3072)] {
        let pass = PrimaryMotionPass::for_object(device, primary, [camera, camera], models, reset)?;
        pass.encode(encoder);
        encoder.copy_texture_to_buffer(
            pass.output().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(4),
                },
            },
            pass.output().size(),
        );
    }
    encode_object_map_motion(device, queue, primary, camera, encoder, &readback)?;
    encode_raster_motion(device, primary, camera, object_ids, encoder, &readback)?;
    Ok(readback)
}
fn verify_motion(
    device: &wgpu::Device,
    readback: &wgpu::Buffer,
) -> Result<(), Box<dyn std::error::Error>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(std::time::Duration::from_secs(5)),
    })?;
    receiver.recv_timeout(std::time::Duration::from_secs(1))??;
    let mapped = readback.slice(..).get_mapped_range()?;
    for (plane_index, (plane, expected)) in mapped
        .chunks_exact(1024)
        .zip([
            [-0.1_f32, -0.05],
            [0.0, 0.0],
            [-0.1, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
        ])
        .enumerate()
    {
        for (pixel_index, pixel) in plane
            .chunks_exact(256)
            .flat_map(|row| row[..16].chunks_exact(4))
            .enumerate()
        {
            let expected = if plane_index == 5 {
                if pixel_index % 4 < 2 {
                    [-0.1, 0.0]
                } else {
                    [0.1, 0.0]
                }
            } else if plane_index == 4 {
                [[-0.1, 0.0], [-0.1, 0.0], [0.1, 0.0], [0.0, 0.0]][pixel_index % 4]
            } else {
                expected
            };
            for (bytes, expected) in pixel.chunks_exact(2).zip(expected) {
                let actual = half::f16::from_bits(u16::from_le_bytes(bytes.try_into()?)).to_f32();
                if !actual.is_finite() || (actual - expected).abs() > 0.0001 {
                    return Err(format!("primary UV motion {actual} != {expected}").into());
                }
            }
        }
    }
    drop(mapped);
    readback.unmap();
    println!(
        "Primary static camera RG16 UV motion: camera/object translation and history reset match all pixels"
    );
    Ok(())
}

fn encode_object_map_motion(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    primary: &PrimarySurfaceJob,
    camera: glam::Mat4,
    encoder: &mut wgpu::CommandEncoder,
    readback: &wgpu::Buffer,
) -> Result<(), Box<dyn std::error::Error>> {
    let ids = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("motion object ID proof"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Uint,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let data = [0_u32, 0, 1, 99].repeat(4);
    queue.write_texture(
        ids.as_image_copy(),
        &data
            .iter()
            .flat_map(|id| id.to_le_bytes())
            .collect::<Vec<_>>(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16),
            rows_per_image: Some(4),
        },
        ids.size(),
    );
    let translation = |x| glam::Mat4::from_translation(glam::Vec3::new(x, 0.0, 0.0));
    let models = [
        [translation(0.3), translation(0.1)],
        [translation(-0.1), translation(0.1)],
    ];
    let pass =
        PrimaryMotionPass::for_objects(device, primary, [camera, camera], &models, &ids, false)?;
    pass.encode(encoder);
    encoder.copy_texture_to_buffer(
        pass.output().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 4096,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        pass.output().size(),
    );
    Ok(())
}

type ReflectionReference = (voxy_render::SpecularDistanceJob, Vec<[f32; 4]>, Vec<f32>);
fn reference_reflection(
    device: &wgpu::Device,
    scene: &RayScene,
) -> Result<ReflectionReference, Box<dyn std::error::Error>> {
    let mut rays = Vec::new();
    let mut colors = Vec::new();
    let mut expected = Vec::new();
    for y in 0_u16..4 {
        for x in 0_u16..4 {
            let px = f32::from(x) * 0.5 - 0.75;
            let py = 0.75 - f32::from(y) * 0.5;
            let material = ReconstructionMaterial::new(primary_base(px, py), 0.5, 0.0)?;
            rays.push(MirrorSurfaceSample::new(
                &material,
                [px, py, 0.5],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 3.0],
                0.001,
                10.0,
            )?);
            let weight = material.mirror_throughput([0.0, 0.0, 1.0], [-px, -py, 2.5])?;
            colors.push([weight[0] * 4.0, weight[1], weight[2] * 0.5, 1.0]);
            expected.push((px * px + py * py + 6.25).sqrt());
        }
    }
    let job = SpecularDistancePipeline::new(device)?.create_mirror_job(
        scene,
        [4, 4],
        &rays,
        &[[4.0, 1.0, 0.5, 1.0]],
    )?;
    Ok((job, colors, expected))
}

fn encode_geometry_ids(
    device: &wgpu::Device,
    pass: &ReconstructionGuidePass,
    guides: &RayReconstructionGuides,
    depth: &wgpu::Texture,
    inputs: &voxy_render::ReconstructionGuideInputs,
    encoder: &mut wgpu::CommandEncoder,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut meshes = Vec::new();
    for [left, right] in [[-1.0, 0.0], [0.0, 1.0]] {
        let positions = [
            [left, -1.0, 0.5],
            [right, -1.0, 0.5],
            [left, 1.0, 0.5],
            [right, -1.0, 0.5],
            [right, 1.0, 0.5],
            [left, 1.0, 0.5],
        ];
        let vertices = positions
            .into_iter()
            .map(|p| {
                ReconstructionGuideVertex::new(
                    p,
                    [0.0, 0.0, 1.0],
                    primary_base(p[0], p[1]),
                    0.5,
                    0.0,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        meshes.push(ReconstructionGuideMesh::upload(device, &vertices)?);
    }
    for depth_z in [0.4, 0.6] {
        let vertices = [
            [-10.0, -10.0, depth_z],
            [10.0, -10.0, depth_z],
            [0.0, 10.0, depth_z],
        ]
        .map(|position| {
            ReconstructionGuideVertex::new(position, [0.0, 0.0, 1.0], [0.5; 3], 0.5, 0.0)
        })
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
        meshes.push(ReconstructionGuideMesh::upload(device, &vertices)?);
    }
    assert!(
        pass.encode_object_ids(
            device,
            encoder,
            guides,
            &depth.create_view(&wgpu::TextureViewDescriptor::default()),
            inputs,
            &[(&meshes[0], u32::MAX)]
        )
        .is_err()
    );
    pass.encode_object_ids(
        device,
        encoder,
        guides,
        &depth.create_view(&wgpu::TextureViewDescriptor::default()),
        inputs,
        &[
            (&meshes[0], 0),
            (&meshes[1], 1),
            (&meshes[2], 77),
            (&meshes[3], 88),
        ],
    )?;
    Ok(())
}
fn encode_raster_motion(
    device: &wgpu::Device,
    primary: &PrimarySurfaceJob,
    camera: glam::Mat4,
    ids: &wgpu::Texture,
    encoder: &mut wgpu::CommandEncoder,
    readback: &wgpu::Buffer,
) -> Result<(), Box<dyn std::error::Error>> {
    let translation = |x| glam::Mat4::from_translation(glam::Vec3::new(x, 0.0, 0.0));
    let models = [
        [translation(0.3), translation(0.1)],
        [translation(-0.1), translation(0.1)],
    ];
    let pass =
        PrimaryMotionPass::for_objects(device, primary, [camera, camera], &models, ids, false)?;
    pass.encode(encoder);
    encoder.copy_texture_to_buffer(
        pass.output().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 5120,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        pass.output().size(),
    );
    Ok(())
}

fn rough_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &RayScene,
    depth: &wgpu::Texture,
    f0: &wgpu::Texture,
    encoder: &mut wgpu::CommandEncoder,
    case: (f32, u32),
) -> Result<wgpu::Buffer, Box<dyn std::error::Error>> {
    let normals = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rough GPU fixture"),
        size: depth.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let data: Vec<u8> = [0.0_f32, 0.0, 1.0, case.0]
        .into_iter()
        .cycle()
        .take(64)
        .flat_map(f32::to_le_bytes)
        .collect();
    queue.write_texture(
        normals.as_image_copy(),
        &data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(64),
            rows_per_image: Some(4),
        },
        depth.size(),
    );
    let primary = PrimarySurfaceJob::new(device, depth, &normals, primary_camera(), 1.0)?;
    primary.encode(encoder);
    let pipeline = voxy_render::GgxReflectionPipeline::new(device)?;
    let initial_reflection = rough_job(&pipeline, scene, &primary, f0, case.1.wrapping_add(1))?;
    initial_reflection.encode(encoder);
    let old_radiance = initial_reflection.radiance().clone();
    let old_distance = initial_reflection.distance().clone();
    let old_hits = initial_reflection.hits().clone();
    let job = pipeline.create_job_reusing(
        scene,
        voxy_render::GgxReflectionInputs {
            primary: &primary,
            emission: &[[4.0, 1.0, 0.5, 1.0]],
            options: SurfaceReflectionOptions {
                material: ReconstructionMaterial::new([0.0; 3], 1.0, 0.0)?,
                camera: [0.0, 0.0, 3.0],
                bias: 0.001,
                maximum_distance: 10.0,
            },
            material_f0: f0,
            seed: case.1,
        },
        initial_reflection,
    )?;
    assert_eq!(job.radiance(), &old_radiance, "GGX radiance was not reused");
    assert_eq!(job.distance(), &old_distance, "GGX distance was not reused");
    assert_eq!(job.hits(), &old_hits, "GGX hit storage was not reused");
    job.encode(encoder);
    let accumulator = voxy_render::RadianceAccumulator::new(device)?;
    let mut mean = accumulator.prepare(job.radiance(), None)?;
    mean.encode(encoder);
    for step in 1..8_u32 {
        let sample = rough_job(&pipeline, scene, &primary, f0, case.1.wrapping_add(step))?;
        sample.encode(encoder);
        let candidate = accumulator.prepare(sample.radiance(), Some(&mean))?;
        candidate.encode(encoder);
        mean = candidate;
    }
    assert_eq!(mean.samples(), 8);
    let lighting_pipeline = voxy_render::GgxLightingPipeline::new(device)?;
    let initial_direct = lighting_pipeline.create_job(
        scene,
        &primary,
        f0,
        f0,
        [0.0, 0.0, 3.0],
        SurfacePointLight {
            position: [0.0, 0.0, 2.0],
            intensity: [0.0; 3],
            bias: 0.001,
        },
    )?;
    initial_direct.encode(encoder);
    let old_direct = initial_direct.output().clone();
    let direct = lighting_pipeline.create_job_reusing(
        scene,
        voxy_render::GgxLightingInputs {
            primary: &primary,
            diffuse: f0,
            f0,
            camera: [0.0, 0.0, 3.0],
            light: SurfacePointLight {
                position: [0.0, 0.0, 2.0],
                intensity: [1.0, 2.0, 3.0],
                bias: 0.001,
            },
        },
        initial_direct,
    )?;
    assert_eq!(
        direct.output(),
        &old_direct,
        "GGX direct storage was not reused"
    );
    direct.encode(encoder);
    let shadowed = lighting_pipeline.create_job(
        scene,
        &primary,
        f0,
        f0,
        [0.0, 0.0, 3.0],
        SurfacePointLight {
            position: [0.0, 0.0, 4.0],
            intensity: [1.0, 2.0, 3.0],
            bias: 0.001,
        },
    )?;
    shadowed.encode(encoder);
    let lights = lighting_pipeline.create_lights(
        scene,
        &primary,
        f0,
        f0,
        [0.0, 0.0, 3.0],
        &[
            SurfacePointLight {
                position: [0.0, 0.0, 2.0],
                intensity: [1.0, 2.0, 3.0],
                bias: 0.001,
            },
            SurfacePointLight {
                position: [0.75, -0.25, 2.0],
                intensity: [3.0, 1.0, 2.0],
                bias: 0.001,
            },
        ],
    )?;
    lights.encode(encoder);
    let combined = RadianceComposition::new_hdr(device, lights.output(), mean.output())?;
    combined.encode(encoder);
    let resolve_pipeline = voxy_render::HdrHalfResolvePipeline::new(device)?;
    let resolved = resolve_pipeline.prepare(combined.output())?;
    resolved.encode(encoder);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rough GPU proof"),
        size: 8704,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        job.radiance().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        job.radiance().size(),
    );
    encoder.copy_texture_to_buffer(
        job.distance().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 1024,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        job.distance().size(),
    );
    encoder.copy_texture_to_buffer(
        mean.output().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 2048,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        mean.output().size(),
    );
    encoder.copy_texture_to_buffer(
        direct.output().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 3072,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        direct.output().size(),
    );
    encoder.copy_texture_to_buffer(
        shadowed.output().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 4096,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        shadowed.output().size(),
    );
    encoder.copy_texture_to_buffer(
        combined.output().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 5120,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        combined.output().size(),
    );
    encoder.copy_texture_to_buffer(
        resolved.output().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 6144,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        resolved.output().size(),
    );
    encoder.copy_buffer_to_buffer(job.hits(), 0, &readback, 7168, 16 * 48);
    let correspondence = voxy_render::ReflectionCorrespondencePipeline::new(device)?;
    let previous_triangle = voxy_render::PreviousReflectionTriangle {
        identity: [0; 4],
        vertices: [
            [-10.0, -10.0, 2.0, 1.0],
            [12.0, -10.0, 2.5, 1.0],
            [0.0, 12.0, 4.0, 1.0],
        ],
    };
    let mut decoy = previous_triangle;
    decoy.identity = [1, 0, 0, 0];
    let previous = correspondence.prepare(&job, &[decoy, previous_triangle], false)?;
    let reset = correspondence.prepare(&job, &[previous_triangle], true)?;
    let unknown = correspondence.prepare(&job, &[decoy], false)?;
    assert!(
        correspondence
            .prepare(&job, &[previous_triangle, previous_triangle], false)
            .is_err()
    );
    let mut malformed = previous_triangle;
    malformed.vertices[0][0] = f32::NAN;
    assert!(correspondence.prepare(&job, &[malformed], false).is_err());
    for (output, offset) in [(&previous, 7936), (&reset, 8192), (&unknown, 8448)] {
        output.encode(encoder);
        encoder.copy_buffer_to_buffer(output.positions(), 0, &readback, offset, 256);
    }

    Ok(readback)
}
fn verify_rough_gpu(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    roughness: f32,
    seed: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let mapped = buffer.slice(..).get_mapped_range()?;
    let mut positive = 0;
    for y in 0..4_u16 {
        for x in 0..4_u16 {
            let index = u32::from(y) * 4 + u32::from(x);
            let px = f32::from(x) * 0.5 - 0.75;
            let py = 0.75 - f32::from(y) * 0.5;
            let base = primary_base(px, py);
            let f0 = base.map(|v| half::f16::from_f32(0.02 + v * 0.5).to_f32());
            let (expected, distance) = rough_reference(px, py, index, f0, roughness, seed)?;
            let record_offset = 7168 + usize::try_from(index)? * 48;
            let words: [u32; 12] = std::array::from_fn(|i| {
                let offset = record_offset + i * 4;
                u32::from_le_bytes(
                    mapped[offset..offset + 4]
                        .try_into()
                        .expect("four-byte word"),
                )
            });
            let hit = voxy_render::ReflectionHit {
                position_distance: std::array::from_fn(|i| f32::from_bits(words[i])),
                identity: std::array::from_fn(|i| words[i + 4]),
                barycentrics_valid: std::array::from_fn(|i| f32::from_bits(words[i + 8])),
            };
            if distance == 0.0 {
                assert_eq!(hit.barycentrics_valid, [0.0; 4], "miss retained old hit");
                assert_eq!(hit.position_distance, [0.0; 4]);
                assert_eq!(hit.identity, [0; 4]);
            } else {
                assert_eq!(hit.barycentrics_valid[3], 1.0);
                assert_eq!(hit.identity[0], 0);
                assert_eq!(hit.identity[2..], [0, 0]);
                assert!((f64::from(hit.position_distance[3]) - distance).abs() < 0.001);
                let b = hit.barycentrics_valid[0];
                let c = hit.barycentrics_valid[1];
                assert!(b >= -1.0e-5 && c >= -1.0e-5 && b + c <= 1.00001);
                let reconstructed = [-10.0 + 20.0 * b + 10.0 * c, -10.0 + 20.0 * c, 3.0];
                for (actual, expected) in hit.position_distance[..3].iter().zip(reconstructed) {
                    assert!(
                        actual.is_finite() && (*actual - expected).abs() < 0.001,
                        "reflected hit world position != triangle barycentric position"
                    );
                }
            }
            let read_position = |base: usize| -> [f32; 4] {
                std::array::from_fn(|i| {
                    let offset =
                        base + usize::try_from(index).expect("small pixel index") * 16 + i * 4;
                    f32::from_le_bytes(mapped[offset..offset + 4].try_into().expect("four bytes"))
                })
            };
            let previous = read_position(7936);
            assert_eq!(
                read_position(8192),
                [0.0; 4],
                "reset retained correspondence"
            );
            assert_eq!(
                read_position(8448),
                [0.0; 4],
                "foreign identity matched geometry"
            );
            if distance == 0.0 {
                assert_eq!(previous, [0.0; 4]);
            } else {
                let b = hit.barycentrics_valid[0];
                let c = hit.barycentrics_valid[1];
                let expected = [
                    -10.0 + 22.0 * b + 10.0 * c,
                    -10.0 + 22.0 * c,
                    2.0 + 0.5 * b + 2.0 * c,
                ];
                assert_eq!(previous[3], 1.0);
                for (actual, expected) in previous[..3].iter().zip(expected) {
                    assert!(
                        (*actual - expected).abs() < 0.001,
                        "previous deformed reflection point mismatch"
                    );
                }
            }
            let mut mean_expected = [0.0; 3];
            for step in 0..8_u32 {
                let (sample, _) =
                    rough_reference(px, py, index, f0, roughness, seed.wrapping_add(step))?;
                for (mean, value) in mean_expected.iter_mut().zip(sample) {
                    *mean += value / 8.0;
                }
            }
            let offset = usize::from(y) * 256 + usize::from(x) * 16;
            for (bytes, expected) in mapped[offset..offset + 12].chunks_exact(4).zip(expected) {
                let actual = f32::from_le_bytes(bytes.try_into()?);
                assert!(
                    actual.is_finite() && (f64::from(actual) - expected).abs() < 0.003,
                    "GPU GGX pixel {index}: {actual} != {expected}"
                );
                if actual > 0.0 {
                    positive += 1;
                }
            }
            let offset = 2048 + usize::from(y) * 256 + usize::from(x) * 16;
            for (channel, bytes) in mapped[offset..offset + 12].chunks_exact(4).enumerate() {
                let actual = f32::from_le_bytes(bytes.try_into()?);
                let mean_expected = mean_expected[channel];
                assert!(
                    actual.is_finite() && (f64::from(actual) - mean_expected).abs() < 0.003,
                    "GPU accumulated GGX {index}: {actual} != {mean_expected}"
                );
            }
            let expected_direct = verify_direct_ggx(&mapped, x, y, px, py, f0, roughness)?;
            let second =
                direct_reference(px, py, f0, roughness, [0.75, -0.25, 2.0], [3.0, 1.0, 2.0])?;
            let offset = 5120 + usize::from(y) * 256 + usize::from(x) * 16;
            for (channel, bytes) in mapped[offset..offset + 12].chunks_exact(4).enumerate() {
                let actual = f32::from_le_bytes(bytes.try_into()?);
                let expected =
                    f64::from(expected_direct[channel] + second[channel]) + mean_expected[channel];
                assert!(
                    actual.is_finite() && (f64::from(actual) - expected).abs() < 0.006,
                    "two-light/reflection HDR pixel {index} channel {channel}: {actual} != {expected}"
                );
                let half_offset = 6144 + usize::from(y) * 256 + usize::from(x) * 8 + channel * 2;
                let resolved = half::f16::from_bits(u16::from_le_bytes(
                    mapped[half_offset..half_offset + 2].try_into()?,
                ))
                .to_f32();
                let expected = expected.clamp(0.0, 65504.0);
                assert!(
                    resolved.is_finite()
                        && (f64::from(resolved) - expected).abs() < 0.006 + expected * 0.001,
                    "resolved HDR pixel {index} channel {channel}: {resolved} != {expected}"
                );
            }
            let alpha = 6144 + usize::from(y) * 256 + usize::from(x) * 8 + 6;
            assert_eq!(
                u16::from_le_bytes(mapped[alpha..alpha + 2].try_into()?),
                half::f16::ONE.to_bits()
            );
            let offset = 1024 + usize::from(y) * 256 + usize::from(x) * 4;
            let actual = f32::from_le_bytes(mapped[offset..offset + 4].try_into()?);
            assert!(
                actual.is_finite() && (f64::from(actual) - distance).abs() < 0.001,
                "GPU GGX distance {index}: {actual} != {distance}"
            );
        }
    }
    assert!(positive > 0, "GPU rough branch returned only black");
    drop(mapped);
    buffer.unmap();
    println!(
        "GPU primary GGX sampling: roughness={roughness}, seed={seed}: 16 pixels and eight-sample means match CPU GGX, direct lighting/opaque shadows, analytic triangle distance and reflected-hit correspondence"
    );
    Ok(())
}

fn verify_direct_ggx(
    mapped: &[u8],
    x: u16,
    y: u16,
    px: f32,
    py: f32,
    f0: [f32; 3],
    roughness: f32,
) -> Result<[f32; 3], Box<dyn std::error::Error>> {
    let expected_direct =
        direct_reference(px, py, f0, roughness, [0.0, 0.0, 2.0], [1.0, 2.0, 3.0])?;
    let offset = 3072 + usize::from(y) * 256 + usize::from(x) * 16;
    for (channel, bytes) in mapped[offset..offset + 12].chunks_exact(4).enumerate() {
        let expected = expected_direct[channel];
        let actual = f32::from_le_bytes(bytes.try_into()?);
        assert!(
            actual.is_finite() && (actual - expected).abs() < 0.003,
            "GPU direct GGX ({x},{y}) channel {channel}: {actual} != {expected}"
        );
    }
    let offset = 4096 + usize::from(y) * 256 + usize::from(x) * 16;
    for (channel, bytes) in mapped[offset..offset + 16].chunks_exact(4).enumerate() {
        let actual = f32::from_le_bytes(bytes.try_into()?);
        let expected = if channel == 3 { 1.0 } else { 0.0 };
        assert!(
            actual.is_finite() && (actual - expected).abs() < 1e-6,
            "shadowed GGX ({x},{y}) channel {channel}: {actual} != {expected}"
        );
    }
    Ok(expected_direct)
}

fn direct_reference(
    px: f32,
    py: f32,
    f0: [f32; 3],
    roughness: f32,
    point: [f32; 3],
    intensity: [f32; 3],
) -> Result<[f32; 3], Box<dyn std::error::Error>> {
    let delta = [point[0] - px, point[1] - py, point[2] - 0.5];
    let distance_squared = delta.iter().map(|v| v * v).sum::<f32>();
    let cosine = delta[2].max(0.0) / distance_squared.sqrt();
    let material = ReconstructionMaterial::new(f0, 1.0, roughness)?;
    let specular = material.ggx_reflection([0.0, 0.0, 1.0], [-px, -py, 2.5], delta)?;
    Ok(std::array::from_fn(|channel| {
        (f0[channel] / std::f32::consts::PI + specular[channel]) * intensity[channel] * cosine
            / distance_squared
    }))
}

fn rough_hash(mut x: u32) -> u32 {
    x = (x ^ (x >> 16)).wrapping_mul(0x7feb352d);
    x = (x ^ (x >> 15)).wrapping_mul(0x846ca68b);
    x ^ (x >> 16)
}

fn rough_job(
    pipeline: &voxy_render::GgxReflectionPipeline,
    scene: &RayScene,
    primary: &PrimarySurfaceJob,
    f0: &wgpu::Texture,
    seed: u32,
) -> Result<SurfaceReflectionJob, voxy_render::RaySceneError> {
    pipeline.create_job(
        scene,
        primary,
        &[[4.0, 1.0, 0.5, 1.0]],
        SurfaceReflectionOptions {
            material: ReconstructionMaterial::new([0.0; 3], 1.0, 0.0)
                .map_err(|_| voxy_render::RaySceneError::InvalidGeometry)?,
            camera: [0.0, 0.0, 3.0],
            bias: 0.001,
            maximum_distance: 10.0,
        },
        f0,
        seed,
    )
}
fn rough_reference(
    px: f32,
    py: f32,
    index: u32,
    f0: [f32; 3],
    roughness: f32,
    seed: u32,
) -> Result<([f64; 3], f64), Box<dyn std::error::Error>> {
    let key = rough_hash(index ^ rough_hash(seed.wrapping_add(0x9e3779b9)));
    #[allow(clippy::cast_precision_loss)]
    let u = [
        (key >> 8) as f32 / 16777216.0,
        (rough_hash(key) >> 8) as f32 / 16777216.0,
    ];
    let sample = ReconstructionMaterial::new(f0, 1.0, roughness)?.sample_ggx_reflection(
        [0.0, 0.0, 1.0],
        [-px, -py, 2.5],
        u,
    )?;
    let mut expected = [0.0; 3];
    let mut distance = 0.0;
    if let Some(sample) = sample {
        let d = sample.direction.map(f64::from);
        let t = 2.5 / d[2];
        let hx = f64::from(px) + t * d[0];
        let hy = f64::from(py) + t * d[1];
        if (0.001..=10.0).contains(&t)
            && (-10.0..=10.0).contains(&hy)
            && hx.abs() <= (10.0 - hy) * 0.5
        {
            distance = t;
            expected = std::array::from_fn(|c| sample.throughput[c] * [4.0, 1.0, 0.5][c]);
        }
    }
    Ok((expected, distance))
}

/// Render a static accumulated GGX reflection image using GPU primary surfaces.
pub fn render_rough_image(
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    let size = 128_u32;
    let scene = RayScene::new(
        device,
        &[[-2.0, -2.0, 3.0], [2.0, -2.0, 3.0], [0.0, 2.0, 3.0]],
    )?;
    let guides = RayReconstructionGuides::new(device, adapter, size, size)?;
    let pass = ReconstructionGuidePass::for_guides(device, &guides);
    let camera = primary_camera();
    let inputs = pass.primary_inputs(device, camera, [0.0, 0.0, 3.0], &guides)?;
    let vertices = [[-1.0, -1.0, 0.5], [3.0, -1.0, 0.5], [-1.0, 3.0, 0.5]]
        .into_iter()
        .zip([0.05, 1.0, 0.6])
        .map(|(p, r)| ReconstructionGuideVertex::new(p, [0.0, 0.0, 1.0], [0.8, 0.4, 0.2], 1.0, r))
        .collect::<Result<Vec<_>, _>>()?;
    let mesh = ReconstructionGuideMesh::upload(device, &vertices)?;
    let depth = primary_depth(device, &guides);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    scene.build(&mut encoder);
    pass.encode(
        &mut encoder,
        &guides,
        &depth.create_view(&wgpu::TextureViewDescriptor::default()),
        &inputs,
        &[&mesh],
    )?;
    pass.encode_material_f0(
        &mut encoder,
        &guides,
        &depth.create_view(&wgpu::TextureViewDescriptor::default()),
        &inputs,
        &[&mesh],
    )?;
    let primary = PrimarySurfaceJob::new(device, &depth, guides.normal_roughness(), camera, 1.0)?;
    primary.encode(&mut encoder);
    let pipeline = voxy_render::GgxReflectionPipeline::new(device)?;
    let accumulator = voxy_render::RadianceAccumulator::new(device)?;
    let mut history = None;
    for seed in 0..64 {
        let sample = rough_job(&pipeline, &scene, &primary, guides.material_f0(), seed)?;
        sample.encode(&mut encoder);
        let candidate = accumulator.prepare(sample.radiance(), history.as_ref())?;
        candidate.encode(&mut encoder);
        history = Some(candidate);
    }
    let mean = history.ok_or("no accumulated samples")?;
    assert_eq!(mean.samples(), 64);
    let direct = SurfaceLightingJob::with_ggx(
        device,
        &scene,
        &primary,
        guides.diffuse_albedo(),
        guides.material_f0(),
        [0.0, 0.0, 3.0],
        SurfacePointLight {
            position: [0.25, 0.25, 2.0],
            intensity: [1.0, 1.0, 1.0],
            bias: 0.001,
        },
    )?;
    direct.encode(&mut encoder);
    let lighting = RadianceComposition::new_hdr(device, direct.output(), mean.output())?;
    lighting.encode(&mut encoder);
    let display = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("rough reflection display"),
        size: guides.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let blit = voxy_render::TextureBlit::tone_mapped(device, display.format(), 1.0)
        .ok_or("tone mapping unsupported")?;
    blit.encode(
        device,
        &mut encoder,
        &lighting
            .output()
            .create_view(&wgpu::TextureViewDescriptor::default()),
        &display.create_view(&wgpu::TextureViewDescriptor::default()),
    );
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rough image export"),
        size: u64::from(size * size * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        display.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size * 4),
                rows_per_image: Some(size),
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
    let mapped = readback.slice(..).get_mapped_range()?;
    assert!(mapped.chunks_exact(4).any(|pixel| pixel[0] > 0));
    let image =
        image::RgbaImage::from_raw(size, size, mapped.to_vec()).ok_or("bad image dimensions")?;
    let path = std::env::temp_dir().join("voxy-rough-reflections.png");
    image.save(&path)?;
    drop(mapped);
    readback.unmap();
    println!(
        "ROUGH IMAGE PASS: GPU primary/direct GGX/64-sample reflection accumulation/HDR composition/tone map: {}",
        path.display()
    );
    Ok(())
}
