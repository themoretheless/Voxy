//! Opt-in experimental ray-query verification. Does not enable experiments in games.
use voxy_render::{GraphicsOptions, RayScene, RaySceneError};
use wgpu::util::DeviceExt;
#[cfg(feature = "face-demo")]
mod face;
mod options;
mod primary_background;
mod specular;
mod specular_guides;

// This probe explicitly acknowledges wgpu's experimental API risk. The engine
// library remains unsafe_code=forbid. Only this token creation is exempted.
#[allow(unsafe_code)]
fn experimental_features() -> wgpu::ExperimentalFeatures {
    // SAFETY: the purpose of this isolated probe is to exercise experimental ray
    // queries on controlled geometry; implementation bugs may still cause UB.
    unsafe { wgpu::ExperimentalFeatures::enabled() }
}
const SHADER: &str = r"
enable wgpu_ray_query;
@group(0) @binding(0) var scene: acceleration_structure;
@group(0) @binding(1) var<storage, read_write> result: array<vec4<f32>>;
@compute @workgroup_size(1)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    var query: ray_query;
    rayQueryInitialize(&query, scene, RayDesc(0u, 255u, 0.001, 10.0,
        vec3<f32>(f32(id.x) * 3.0, 0.0, 1.0), vec3<f32>(0.0, 0.0, -1.0)));
    rayQueryProceed(&query);
    let hit = rayQueryGetCommittedIntersection(&query);
    result[id.x] = vec4<f32>(f32(hit.kind), hit.t, hit.barycentrics);
}";
#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = options::Options::parse(std::env::args().skip(1))?;
    #[cfg(not(feature = "face-demo"))]
    if options.face {
        return Err("face probe requires the face-demo Cargo feature".into());
    }
    let instance = GraphicsOptions {
        backend: options.backend,
        ..Default::default()
    }
    .create_instance();
    let adapter = if options.require_nvidia {
        pollster::block_on(instance.enumerate_adapters(options.backend.backends()))
            .into_iter()
            .find(|adapter| {
                let info = adapter.get_info();
                info.vendor == 0x10de
                    && matches!(
                        info.device_type,
                        wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
                    )
                    && adapter
                        .features()
                        .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
            })
            .ok_or("no physical NVIDIA adapter supports ray queries on the selected backend")?
    } else {
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?
    };
    let info = adapter.get_info();
    if !adapter
        .features()
        .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
    {
        return Err("adapter does not support ray queries".into());
    }
    println!("RAY GPU: {info:?}");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: wgpu::Features::EXPERIMENTAL_RAY_QUERY,
        required_limits: adapter.limits(),
        experimental_features: experimental_features(),
        ..Default::default()
    }))?;
    if options.rough_image {
        return specular_guides::render_rough_image(&device, &adapter, &queue);
    }
    #[cfg(feature = "face-demo")]
    if options.face {
        return face::render(&device, &queue);
    }
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    assert!(matches!(
        RayScene::new(&device, &[]),
        Err(RaySceneError::InvalidGeometry)
    ));
    assert!(matches!(
        RayScene::new(&device, &[[0.0; 3]; 2]),
        Err(RaySceneError::InvalidGeometry)
    ));
    assert!(matches!(
        RayScene::new(&device, &[[f32::NAN; 3]; 3]),
        Err(RaySceneError::InvalidGeometry)
    ));
    let mesh = voxy_render::SceneMesh::new(
        [
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [0.0, 1.0, 0.0],
            [100.0, 100.0, 100.0],
        ]
        .into_iter()
        .map(|position| voxy_render::SceneVertex {
            position,
            uv: [0.0; 2],
            color: [1.0; 4],
        })
        .collect(),
        vec![0, 1, 2],
    )?;
    let mut scene = RayScene::from_scene_mesh(&device, &mesh, 2)?;
    assert_eq!(scene.triangle_count(), 1);
    let output = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ray results"),
        contents: &[0; 32],
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ray readback"),
        size: 32,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ray-query smoke"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("ray query"),
        layout: None,
        module: &module,
        entry_point: Some("cs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let mut changed_vertices = mesh.vertices().to_vec();
    for vertex in &mut changed_vertices {
        vertex.position[2] -= 0.25;
    }
    for position in [[10.0, 10.0, 0.0], [11.0, 10.0, 0.0], [10.0, 11.0, 0.0]] {
        changed_vertices.push(voxy_render::SceneVertex {
            position,
            uv: [0.0; 2],
            color: [1.0; 4],
        });
    }
    let changed_mesh = voxy_render::SceneMesh::new(changed_vertices, vec![0, 1, 2, 4, 5, 6])?;
    let skinned = voxy_render::SkinnedMesh::new(
        mesh.vertices()
            .iter()
            .map(|vertex| voxy_render::SkinnedVertex {
                position: vertex.position,
                normal: [0.0, 0.0, 1.0],
                uv: vertex.uv,
                joints: [0, 1, 0, 0],
                weights: [32768, 32767, 0, 0],
            })
            .collect(),
        mesh.indices().to_vec(),
        2,
    )?;
    let mut cached_binding = None;
    for phase in 0..11 {
        if phase == 1 {
            scene.set_transform([1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., -0.5])?;
        } else if phase == 2 {
            scene.set_transform([1., 0., 0., 3., 0., 1., 0., 0., 0., 0., 1., 0.])?;
        } else if phase == 3 {
            assert_eq!(
                scene.set_transform([0.; 12]),
                Err(RaySceneError::InvalidTransform)
            );
        }
        if phase == 4 {
            scene.set_instances(&[
                voxy_render::RayInstanceUpdate {
                    index: 0,
                    transform: [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.],
                    custom_index: 0,
                    mask: 255,
                },
                voxy_render::RayInstanceUpdate {
                    index: 1,
                    transform: [1., 0., 0., 3., 0., 1., 0., 0., 0., 0., 1., 0.],
                    custom_index: 7,
                    mask: 255,
                },
            ])?;
            assert!(
                scene
                    .set_instances(&[
                        voxy_render::RayInstanceUpdate {
                            index: 0,
                            transform: [1., 0., 0., 9., 0., 1., 0., 0., 0., 0., 1., 0.],
                            custom_index: 0,
                            mask: 255
                        },
                        voxy_render::RayInstanceUpdate {
                            index: 1,
                            transform: [0.; 12],
                            custom_index: 7,
                            mask: 255
                        },
                    ])
                    .is_err()
            );
            assert_eq!(scene.remove_instance(2), Err(RaySceneError::Capacity));
            assert_eq!(
                scene.set_instance(1, [0.; 12], 0x0100_0000, 255),
                Err(RaySceneError::Capacity)
            );
            scene.replace_scene_mesh(&mesh)?;
        } else if phase == 5 {
            scene.set_instance(1, [1., 0., 0., 3., 0., 1., 0., 0., 0., 0., 1., 0.], 7, 0)?;
        } else if phase == 6 {
            scene.remove_instance(0)?;
            scene.replace_scene_mesh(&mesh)?;
        }
        if phase == 7 {
            scene.set_instance(0, [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.], 0, 255)?;
            scene.replace_scene_mesh(&changed_mesh)?;
            assert_eq!(scene.triangle_count(), 2);
        } else if phase == 8 {
            scene.replace_scene_mesh(&mesh)?;
            assert_eq!(scene.triangle_count(), 1);
        }
        if phase == 9 {
            let posed = skinned.posed_scene_mesh(
                &[
                    glam::Mat4::IDENTITY,
                    glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, -0.5)),
                ],
                glam::Mat4::IDENTITY,
                [1.0; 4],
            )?;
            scene.replace_scene_mesh(&posed)?;
        } else if phase == 10 {
            scene.replace_scene_mesh(&mesh)?;
        }
        let expected_revision = match phase {
            0..=3 => 0,
            4..=5 => 1,
            6 => 2,
            7 => 3,
            8 => 4,
            9 => 5,
            _ => 6,
        };
        assert_eq!(scene.geometry_revision(), expected_revision);
        if cached_binding
            .as_ref()
            .is_none_or(|(revision, _)| *revision != scene.geometry_revision())
        {
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ray scene"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: scene.binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output.as_entire_binding(),
                    },
                ],
            });
            cached_binding = Some((scene.geometry_revision(), bind));
        }
        let bind = &cached_binding.as_ref().ok_or("missing ray binding")?.1;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        if matches!(phase, 0 | 4 | 6 | 7 | 8 | 9 | 10) {
            scene.build(&mut encoder);
        } else {
            scene.build_instances(&mut encoder);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, bind, &[]);
            pass.dispatch_workgroups(2, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, 32);
        queue.submit([encoder.finish()]);
        let (send, recv) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = send.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        recv.recv()??;
        let bytes = readback.slice(..).get_mapped_range()?;
        let values: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
            .collect();
        for ray in 0..2 {
            let offset = ray * 4;
            let should_hit = match phase {
                0 | 1 | 5 | 7 | 8 | 9 | 10 => ray == 0,
                2 | 3 => ray == 1,
                4 => true,
                _ => false,
            };
            if should_hit {
                let distance = match phase {
                    1 => 1.5,
                    7 | 9 => 1.25,
                    _ => 1.0,
                };
                assert!(
                    values[offset] > 0.0,
                    "phase {phase}: expected hit {values:?}"
                );
                assert!(
                    (values[offset + 1] - distance).abs() < 1e-4,
                    "phase {phase}: distance {values:?}"
                );
                assert!(
                    (values[offset + 2] - 0.25).abs() < 1e-4
                        && (values[offset + 3] - 0.5).abs() < 1e-4,
                    "phase {phase}: barycentrics {values:?}"
                );
            } else {
                assert!(
                    values[offset].abs() < f32::EPSILON,
                    "phase {phase}: expected miss {values:?}"
                );
            }
        }
        drop(bytes);
        readback.unmap();
    }
    visibility_probe(&device, &queue, &mut scene)?;
    shadow_image(&device, &queue, &scene)?;
    specular::probe(&device, &queue)?;
    specular_guides::probe(&device, &adapter, &queue)?;
    primary_background::probe(&device, &adapter, &queue)?;
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    println!(
        "RAY SMOKE PASS: TLAS transforms, two shared-BLAS instances, masks/removal, invalid update retention"
    );
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn visibility_probe(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &mut RayScene,
) -> Result<(), Box<dyn std::error::Error>> {
    scene.set_transform([1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.])?;
    let segments = [
        voxy_render::RaySegment::new([0., 0., 1.], [0., 0., -1.], 0.001)?,
        voxy_render::RaySegment::new([3., 0., 1.], [3., 0., -1.], 0.001)?,
        voxy_render::RaySegment::new([0., 0., 1.], [0., 0., 0.5], 0.001)?,
        voxy_render::RaySegment::new([0., 0., 0.], [0., 0., 1.], 0.001)?,
        voxy_render::RaySegment::new([0., 0., 0.0005], [0., 0., -1.], 0.001)?,
        voxy_render::RaySegment::new([0., 0., 0.002], [0., 0., -1.], 0.001)?,
        voxy_render::RaySegment::new([0., 0., 1.], [0., 0., -0.0005], 0.001)?,
        voxy_render::RaySegment::new([0., 0., 1.], [0., 0., -0.002], 0.001)?,
    ];
    let mut expected = voxy_render::cpu_segment_visibility(
        &[[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
        &segments,
    )?;
    assert_eq!(expected, [0, 1, 1, 1, 1, 0, 1, 0], "endpoint bias contract");
    let reversed: Vec<_> = segments.iter().copied().rev().collect();
    expected.extend(expected.clone().into_iter().rev());
    let visibility_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let visibility = voxy_render::RayVisibilityPipeline::new(device)?;
    let job = visibility.create_job(scene, &segments)?;
    let other = visibility.create_job(scene, &reversed)?;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("visibility readback"),
        size: 64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    scene.build_instances(&mut encoder);
    job.encode(&mut encoder);
    other.encode(&mut encoder);
    encoder.copy_buffer_to_buffer(job.output(), 0, &readback, 0, 32);
    encoder.copy_buffer_to_buffer(other.output(), 0, &readback, 32, 32);
    queue.submit([encoder.finish()]);
    let (send, recv) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = send.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    recv.recv()??;
    if let Some(error) = pollster::block_on(visibility_scope.pop()) {
        return Err(error.into());
    }
    let mapped = readback.slice(..).get_mapped_range()?;
    let values: Vec<_> = mapped
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    assert_eq!(
        values, expected,
        "CPU/GPU visibility including endpoint bias"
    );
    drop(mapped);
    readback.unmap();
    println!("SHADOW VISIBILITY PASS: blocked, unobstructed and finite light segment");
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn shadow_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &RayScene,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut segments = Vec::new();
    for y in 0..64 {
        for x in 0..64 {
            let px = (x as f32 + 0.5) / 8.0 - 4.0;
            let py = (y as f32 + 0.5) / 8.0 - 4.0;
            segments.push(voxy_render::RaySegment::new(
                [px, py, -1.0],
                [0., 0., 1.0],
                0.001,
            )?);
        }
    }
    let cpu_visibility = voxy_render::cpu_segment_visibility(
        &[[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
        &segments,
    )?;
    let job = voxy_render::RayVisibilityPipeline::new(device)?.create_job(scene, &segments)?;
    let image = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ray-traced shadow plane"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("point light composition"),
        source: wgpu::ShaderSource::Wgsl(
            r"
@group(0) @binding(0) var<storage, read> visibility: array<u32>;
@group(0) @binding(1) var image: texture_storage_2d<rgba8unorm, write>;
@compute @workgroup_size(8,8)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(image);
    if id.x >= size.x || id.y >= size.y { return; }
    let p = (vec2<f32>(id.xy) + 0.5) / 8.0 - 4.0;
    let diffuse = 2.0 / length(vec3<f32>(p, 2.0));
    let light = 0.1 + 0.9 * diffuse * f32(visibility[id.y * size.x + id.x]);
    textureStore(image, id.xy, vec4<f32>(vec3<f32>(0.8,0.6,0.4) * light, 1.0));
}"
            .into(),
        ),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("shadow plane lighting"),
        layout: None,
        module: &shader,
        entry_point: Some("cs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let view = image.create_view(&wgpu::TextureViewDescriptor::default());
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("shadow composition"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: job.output().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
        ],
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("shadow pixels"),
        size: 64 * 256,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    job.encode(&mut encoder);
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(8, 8, 1);
    }
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &image,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(64),
            },
        },
        image.size(),
    );
    queue.submit([encoder.finish()]);
    let (send, recv) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = send.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    recv.recv()??;
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    let pixels = readback.slice(..).get_mapped_range()?;
    let center = &pixels[32 * 256 + 32 * 4..32 * 256 + 32 * 4 + 4];
    for (actual, expected) in center.iter().zip([20_u8, 15, 10, 255]) {
        assert!(actual.abs_diff(expected) <= 1, "shadow center {center:?}");
    }
    let corner = &pixels[..4];
    let coordinate = -3.9375_f32;
    let light = 0.1 + 0.9 * 2.0 / (2.0 * coordinate * coordinate + 4.0).sqrt();
    for (actual, base) in corner[..3].iter().zip([0.8, 0.6, 0.4]) {
        assert!(
            actual.abs_diff((base * light * 255.0).round() as u8) <= 1,
            "lit corner {corner:?}"
        );
    }
    for (index, pixel) in pixels.chunks_exact(4).enumerate() {
        let x = (index % 64) as f32;
        let y = (index / 64) as f32;
        let px = (x + 0.5) / 8.0 - 4.0;
        let py = (y + 0.5) / 8.0 - 4.0;
        let diffuse = 2.0 / (px * px + py * py + 4.0).sqrt();
        let light = 0.1 + 0.9 * diffuse * cpu_visibility[index] as f32;
        for (actual, base) in pixel[..3].iter().zip([0.8, 0.6, 0.4]) {
            assert!(
                actual.abs_diff((base * light * 255.0).round() as u8) <= 1,
                "CPU/GPU shadow mismatch at {index}: {pixel:?}"
            );
        }
    }
    let mut ppm = b"P6\n64 64\n255\n".to_vec();
    for pixel in pixels.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    std::fs::write("/tmp/voxy-ray-shadow.ppm", ppm)?;
    drop(pixels);
    readback.unmap();
    println!(
        "SHADOW IMAGE PASS: ray visibility + point-light diffuse/ambient; /tmp/voxy-ray-shadow.ppm"
    );
    Ok(())
}
