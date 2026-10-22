//! Actual full groom: compute extraction/scatter and the draw material in one submission.
use super::FemaleDemo;
use glam::{Mat4, Vec3};
use voxy_render::{SceneDraw, SceneRenderer, SceneView};

#[test]
#[ignore = "full-density actual groom GPU material pixel qualification"]
fn resident_groom_draw_matches_cpu_on_two_poses() {
    pollster::block_on(async {
        let mut demo = FemaleDemo::new().unwrap();
        demo.enable_physical_stance().unwrap();
        demo.resident_surface_light = true;
        let instance = voxy_render::GraphicsOptions::default().create_instance();
        let adapter = instance.request_adapter(&Default::default()).await.unwrap();
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        eprintln!("GROOM PIXELS adapter={:?}", adapter.get_info());
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        renderer.enable_msaa4(&device).await.unwrap();
        let texture = renderer
            .upload_texture(&device, &queue, 1, 1, &[255; 4])
            .unwrap();
        let shader = std::env::var("VOXY_GROOM_SHADER_OVERRIDE")
            .ok()
            .map(|path| std::fs::read_to_string(path).expect("explicit diagnostic shader path"));
        let program = voxy_render::ComputeProgram::new(
            &device,
            shader
                .as_deref()
                .unwrap_or(voxy_render::FIBER_SURFACE_SHADER),
        )
        .await
        .unwrap();
        // Test-only mechanism isolation; the default remains the production material.
        let material_override = std::env::var("VOXY_GROOM_MATERIAL_OVERRIDE")
            .ok()
            .map(|path| std::fs::read_to_string(path).expect("explicit diagnostic material path"));
        let material = material_override
            .as_deref()
            .unwrap_or(include_str!("female_hair_material_tangent.wgsl"));
        let mut input = demo.gpu_hair_surface_input().unwrap();
        assert_eq!(input.vertex_count(), 2_363_760);
        let job = program.create_job(&device, input.bytes()).unwrap();
        let initial_mesh = demo.mesh_with_full_hair().unwrap();
        let base = initial_mesh
            .vertices()
            .iter()
            .position(|v| v.uv[0] == -7.)
            .unwrap();
        let (_, indices) = crate::full_model_worker::partition_hair(&initial_mesh).unwrap();
        // Poison initial hair positions: a skipped compute/scatter must not match CPU.
        let mut poisoned = initial_mesh.vertices().to_vec();
        for v in &mut poisoned[base..base + input.vertex_count() as usize] {
            v.position = [100.; 3];
        }
        let poison_mesh = voxy_render::SceneMesh::new(poisoned, initial_mesh.indices().to_vec())
            .unwrap()
            .with_normals(initial_mesh.authored_normals().unwrap().to_vec())
            .unwrap();
        let mut gpu = renderer
            .upload_compute_tangent_mesh(
                &device,
                &poison_mesh,
                &vec![[1., 0., 0., 1.]; poison_mesh.vertices().len()],
            )
            .unwrap();
        gpu.update_index_partition(&queue, &indices).unwrap();
        renderer
            .set_geometry_opaque_shader(&device, &mut gpu, material)
            .await
            .unwrap();
        let transfer = voxy_render::FiberSurfaceTransfer::new_with_tangents(
            &device,
            &gpu,
            job.buffer(),
            &input,
            base as u32,
        )
        .await
        .unwrap();
        const WIDTH: u32 = 512;
        const HEIGHT: u32 = 512;
        let make_texture = |format, sample_count, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("actual groom material audit"),
                size: wgpu::Extent3d {
                    width: WIDTH,
                    height: HEIGHT,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let color = make_texture(
            wgpu::TextureFormat::Rgba8Unorm,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let depth = make_texture(
            wgpu::TextureFormat::Depth32Float,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let msaa_color = make_texture(
            wgpu::TextureFormat::Rgba8Unorm,
            4,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let msaa_depth = make_texture(
            wgpu::TextureFormat::Depth32Float,
            4,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let image_bytes = u64::from(WIDTH * HEIGHT * 4);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("groom comparison pixels"),
            size: image_bytes * 2,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut prior_cpu_pixels = None;
        let mut maximum_across_poses = 0u8;
        for tick in [1, 31] {
            while demo.steps < tick {
                demo.advance(crate::female_physical_stance::STEP).unwrap();
            }
            let mesh = demo.mesh_with_full_hair().unwrap();
            let mut cpu = renderer
                .upload_tangent_mesh(
                    &device,
                    &mesh,
                    &demo.gpu_hair_vertex_tangents(&mesh).unwrap(),
                )
                .unwrap();
            cpu.update_index_partition(&queue, &indices).unwrap();
            renderer
                .set_geometry_opaque_shader(&device, &mut cpu, material)
                .await
                .unwrap();
            let frames = demo.gpu_hair_surface_frames();
            queue.write_buffer(job.buffer(), 32, input.replace_frames(&frames).unwrap());
            let offset = input.transform_byte_offset();
            queue.write_buffer(
                job.buffer(),
                offset,
                input
                    .replace_transform(demo.gpu_hair_surface_transform())
                    .unwrap(),
            );
            let points = &mesh.vertices()[base..base + input.vertex_count() as usize];
            let min = points.iter().fold(Vec3::splat(f32::INFINITY), |a, v| {
                a.min(Vec3::from(v.position))
            });
            let max = points.iter().fold(Vec3::splat(f32::NEG_INFINITY), |a, v| {
                a.max(Vec3::from(v.position))
            });
            // Fixed camera so that the second pose must change the reference image.
            let center = Vec3::new(0., 0.71, 0.04);
            let eye = center + Vec3::new(0.35, 0.05, 0.9);
            let transform = renderer
                .create_transform(
                    &device,
                    glam::camera::rh::proj::directx::orthographic(
                        -0.28, 0.28, -0.28, 0.28, 0.01, 3.,
                    ) * glam::camera::rh::view::look_at_mat4(eye, center, Vec3::Y),
                )
                .unwrap();
            transform.update_view_position(&queue, eye).unwrap();
            let mut encoder = device.create_command_encoder(&Default::default());
            job.encode_step(&mut encoder, [input.vertex_count().div_ceil(64), 1, 1])
                .unwrap();
            transfer.encode(&mut encoder);
            for (frame, geometry) in [&cpu, &gpu].into_iter().enumerate() {
                let draws = [SceneDraw {
                    geometry,
                    texture: &texture,
                    transform: &transform,
                    overlay: false,
                }];
                let views = [SceneView {
                    viewport: [0, 0, WIDTH, HEIGHT],
                    draws: &draws,
                }];
                renderer
                    .encode_view_frame_msaa4(
                        &mut encoder,
                        (
                            &msaa_color.create_view(&Default::default()),
                            &msaa_depth.create_view(&Default::default()),
                        ),
                        &color.create_view(&Default::default()),
                        &depth.create_view(&Default::default()),
                        wgpu::Color::BLACK,
                        &views,
                        &[],
                    )
                    .unwrap();
                encoder.copy_texture_to_buffer(
                    color.as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &readback,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: frame as u64 * image_bytes,
                            bytes_per_row: Some(WIDTH * 4),
                            rows_per_image: Some(HEIGHT),
                        },
                    },
                    color.size(),
                );
            }
            queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            rx.recv().unwrap().unwrap();
            let data = readback.slice(..).get_mapped_range().unwrap();
            let (reference, actual) = data.split_at(image_bytes as usize);
            let occupied = reference
                .chunks_exact(4)
                .filter(|p| p[..3].iter().any(|v| *v > 0))
                .count();
            let maximum = reference
                .iter()
                .zip(actual)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            let differing = reference.iter().zip(actual).filter(|(a, b)| a != b).count();
            eprintln!(
                "GROOM PIXELS tick={tick} vertices={} occupied={occupied} maximum_u8_error={maximum} differing_channels={differing} bounds={min:?}..{max:?}",
                input.vertex_count()
            );
            assert!(
                occupied > 1000,
                "empty rendering cannot establish groom parity"
            );
            maximum_across_poses = maximum_across_poses.max(maximum);
            for (pixel, (a, b)) in reference
                .chunks_exact(4)
                .zip(actual.chunks_exact(4))
                .enumerate()
                .filter(|(_, (a, b))| a.iter().zip(*b).any(|(x, y)| x.abs_diff(*y) > 1))
                .take(16)
            {
                eprintln!(
                    "GROOM PIXEL DIFFERENCE tick={tick} x={} y={} CPU={a:?} GPU={b:?}",
                    pixel % WIDTH as usize,
                    pixel / WIDTH as usize
                );
            }
            if let Some(prior) = prior_cpu_pixels {
                assert_ne!(prior, reference, "poses must change the rendered groom");
            }
            prior_cpu_pixels = Some(reference.to_vec());
            drop(data);
            readback.unmap();
        }
        assert!(scope.pop().await.is_none(), "GPU validation failure");
        assert!(
            maximum_across_poses <= 1,
            "actual groom compute/scatter/material mismatch: {maximum_across_poses}"
        );
    });
}

/// GPU pass timestamps for actual full-density extraction; excludes scatter,
/// rendering, CPU simulation, upload, and readback. This is not an FPS benchmark.
#[test]
#[ignore = "actual groom GPU extraction timestamp profile"]
fn resident_groom_extraction_gpu_timestamp_cost() {
    pollster::block_on(async {
        let mut demo = FemaleDemo::new().unwrap();
        demo.enable_physical_stance().unwrap();
        let instance = voxy_render::GraphicsOptions::default().create_instance();
        let adapter = instance.request_adapter(&Default::default()).await.unwrap();
        assert!(adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY));
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::TIMESTAMP_QUERY,
                ..Default::default()
            })
            .await
            .unwrap();
        eprintln!("GROOM EXTRACTION PROFILE adapter={:?}", adapter.get_info());
        let program = voxy_render::ComputeProgram::new(&device, voxy_render::FIBER_SURFACE_SHADER)
            .await
            .unwrap();
        let mut input = demo.gpu_hair_surface_input().unwrap();
        assert_eq!(input.vertex_count(), 2_363_760);
        let job = program.create_job(&device, input.bytes()).unwrap();
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("full groom extraction timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let resolved = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("groom timestamp resolve"),
            size: 256,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let period = f64::from(queue.get_timestamp_period());
        assert!(period.is_finite() && period > 0.);
        for tick in [1, 31] {
            while demo.steps < tick {
                demo.advance(crate::female_physical_stance::STEP).unwrap();
            }
            queue.write_buffer(
                job.buffer(),
                32,
                input
                    .replace_frames(&demo.gpu_hair_surface_frames())
                    .unwrap(),
            );
            let offset = input.transform_byte_offset();
            queue.write_buffer(
                job.buffer(),
                offset,
                input
                    .replace_transform(demo.gpu_hair_surface_transform())
                    .unwrap(),
            );
            let mut samples = Vec::new();
            const REPETITIONS: u32 = 16;
            for sample in 0..12 {
                let mut encoder = device.create_command_encoder(&Default::default());
                job.encode_repeated_steps_with_timestamps(
                    &mut encoder,
                    [input.vertex_count().div_ceil(64), 1, 1],
                    REPETITIONS,
                    wgpu::ComputePassTimestampWrites {
                        query_set: &queries,
                        beginning_of_pass_write_index: Some(0),
                        end_of_pass_write_index: Some(1),
                    },
                )
                .unwrap();
                // Complete the sampled submission before resolving counters, as in
                // the existing Metal QR profiler; do not treat unwritten values as time.
                let sampled = queue.submit([encoder.finish()]);
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(sampled),
                        timeout: None,
                    })
                    .unwrap();
                let mut encoder = device.create_command_encoder(&Default::default());
                encoder.resolve_query_set(&queries, 0..2, &resolved, 0);
                let dispatch = voxy_render::ComputeDispatch::gather_buffers(
                    &device,
                    &mut encoder,
                    &[(&resolved, 0, 0, 16)],
                    16,
                )
                .unwrap();
                queue.submit([encoder.finish()]);
                let mut pending = dispatch.begin_read();
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                let bytes = pending.try_read().unwrap().unwrap();
                let begin = u64::from_le_bytes(bytes[..8].try_into().unwrap());
                let end = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
                assert!(
                    end > begin,
                    "invalid GPU interval tick={tick} sample={sample}: begin={begin} end={end}"
                );
                let ms = (end - begin) as f64 * period / 1e6 / f64::from(REPETITIONS);
                if sample >= 3 {
                    samples.push(ms);
                }
            }
            samples.sort_by(f64::total_cmp);
            eprintln!(
                "GROOM EXTRACTION GPU tick={tick} vertices={} repetitions={REPETITIONS} samples={} median_ms={} min_ms={} max_ms={}",
                input.vertex_count(),
                samples.len(),
                samples[samples.len() / 2],
                samples[0],
                samples[samples.len() - 1]
            );
        }
    });
}

#[test]
#[ignore = "actual prepared body sparse physical shell GPU transfer qualification"]
fn full_body_gpu_shell_transfer_matches_cpu_and_clears_previous_deformation() {
    pollster::block_on(async {
        let mut demo = FemaleDemo::new().unwrap();
        demo.enable_physical_stance().unwrap();
        let mut base = demo.posed_surface_vertices();
        let count = base.len();
        demo.finish_surface_vertices(&mut base, &vec![[0.; 3]; demo.body_vertices], count);
        let mesh = voxy_render::SceneMesh::new(base.clone(), demo.indices.clone()).unwrap();
        let (offsets, weights, controls) = demo.gpu_shell_binding(count).unwrap();
        assert_eq!(demo.body_vertices, 60_866);
        assert_eq!(controls, 636);
        assert_eq!(weights.len(), demo.body_vertices * 3);
        assert!(
            offsets[demo.body_vertices..]
                .iter()
                .all(|n| *n == weights.len() as u32),
            "eyes must receive no shell displacement"
        );
        let instance = voxy_render::GraphicsOptions::default().create_instance();
        let adapter = instance.request_adapter(&Default::default()).await.unwrap();
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        let renderer = voxy_render::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let mut surface = renderer
            .prepare_surface_deformation(&device, &mesh, &offsets, &weights, controls)
            .unwrap();
        let mut maximum = 0f32;
        let accepted_shell = demo.skin.positions().to_vec();
        for (sample, amplitude) in [0.002_f64, -0.003, 0.].into_iter().enumerate() {
            // Smooth spatial deformation of an accepted shell, not random
            // per-node offsets that may invert tiny physical elements.
            let shell: Vec<_> = accepted_shell
                .iter()
                .map(|p| {
                    [
                        p[0] + amplitude * (0.5 + 0.2 * (p[1] * 3.).sin()),
                        p[1] + amplitude * 0.2 * (p[0] * 4.).sin(),
                        p[2] + amplitude * 0.3 * (p[1] * 2. + p[0]).cos(),
                    ]
                })
                .collect();
            if sample == 2 {
                demo.targets = accepted_shell.clone();
            }
            demo.skin
                .set_state(shell, vec![[0.; 3]; controls as usize])
                .unwrap();
            let delta = demo.surface_displacement().unwrap();
            if sample < 2 {
                assert!(delta.iter().flatten().any(|v| v.abs() > 0.0001));
            }
            let mut reference = base.clone();
            for (vertex, d) in reference.iter_mut().zip(&delta) {
                for k in 0..3 {
                    vertex.position[k] += d[k] as f32;
                }
            }
            surface
                .update_controls(&queue, &demo.gpu_shell_controls().unwrap())
                .unwrap();
            // Repeat encoding without restoring input: no accumulated/doubled motion.
            for repeat in 0..2 {
                let mut encoder = device.create_command_encoder(&Default::default());
                surface.encode(&mut encoder).unwrap();
                let size = (count * 36) as u64;
                let dispatch = voxy_render::ComputeDispatch::gather_buffers(
                    &device,
                    &mut encoder,
                    &[(surface.vertex_buffer(), 0, 0, size)],
                    size,
                )
                .unwrap();
                queue.submit([encoder.finish()]);
                let mut pending = dispatch.begin_read();
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                let bytes = pending.try_read().unwrap().unwrap();
                for (i, (actual, expected)) in bytes.chunks_exact(36).zip(&reference).enumerate() {
                    let values: Vec<f32> = actual
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                        .collect();
                    let error = Vec3::from_slice(&values).distance(Vec3::from(expected.position));
                    maximum = maximum.max(error);
                    assert!(
                        error < 3e-7,
                        "sample={sample} repeat={repeat} vertex={i} error={error}"
                    );
                    assert_eq!([values[3], values[4]], expected.uv);
                    assert_eq!(&values[5..9], &expected.color);
                    if i >= demo.body_vertices || amplitude == 0. {
                        assert_eq!(&values[..3], &base[i].position);
                    }
                }
            }
        }
        demo.rig_pose_only = true;
        assert!(
            demo.gpu_shell_controls()
                .unwrap()
                .iter()
                .flatten()
                .all(|v| *v == 0.)
        );
        eprintln!(
            "ACTUAL GPU SHELL body_vertices={} all_vertices={count} shell_controls={controls} samples=3 repeated_encodes=2 max_position_error_m={maximum} zero_reset_and_eye_attributes_preserved=true",
            demo.body_vertices
        );
    });
}

#[test]
#[ignore = "actual full CPU rig plus physical shell versus composed GPU stages"]
fn full_rig_and_shell_gpu_composition_matches_cpu_pose() {
    pollster::block_on(async {
        let mut demo = FemaleDemo::new().unwrap();
        demo.enable_physical_stance().unwrap();
        let instance = voxy_render::GraphicsOptions::default().create_instance();
        let adapter = instance.request_adapter(&Default::default()).await.unwrap();
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        let (foreign_device, _foreign_queue) =
            adapter.request_device(&Default::default()).await.unwrap();
        let mut renderer =
            voxy_render::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        renderer.enable_msaa4(&device).await.unwrap();
        renderer
            .reload_shader(&device, include_str!("female_material.wgsl"))
            .await
            .unwrap();
        let mut maximum = 0f32;
        let mut maximum_pixel_error = 0u8;
        let mut resident = None;
        for tick in [1, 31] {
            while demo.steps < tick {
                demo.advance(crate::female_physical_stance::STEP).unwrap();
            }
            let pre_skin = demo.pre_skin_surface_vertices();
            let mesh = voxy_render::SceneMesh::new(pre_skin, demo.indices.clone())
                .unwrap()
                .with_material_coordinates(demo.vertices.iter().map(|v| v.position).collect())
                .unwrap()
                .with_normals(demo.normals.iter().map(|n| n.to_array()).collect())
                .unwrap();
            let (offsets, weights, controls) =
                demo.gpu_shell_binding(mesh.vertices().len()).unwrap();
            if resident.is_none() {
                let mut target_vertices = mesh.vertices().to_vec();
                target_vertices.extend((0..3).map(|i| voxy_render::SceneVertex {
                    position: [100. + i as f32, 100., 100.],
                    uv: [0.25, 0.75],
                    color: [0.1, 0.2, 0.3, 1.],
                }));
                let mut target_normals = mesh.authored_normals().unwrap().to_vec();
                target_normals.extend([[0., 0., 1.]; 3]);
                let coordinates = demo
                    .vertices
                    .iter()
                    .map(|v| v.position)
                    .chain((0..3).map(|i| [100. + i as f32, 100., 100.]))
                    .collect();
                let target_mesh =
                    voxy_render::SceneMesh::new(target_vertices, demo.indices.clone())
                        .unwrap()
                        .with_normals(target_normals)
                        .unwrap()
                        .with_material_coordinates(coordinates)
                        .unwrap();
                let target = renderer.upload_compute_mesh(&device, &target_mesh).unwrap();
                resident = Some(
                    renderer
                        .prepare_surface_deformation_in_geometry(
                            &device, &mesh, &target, &offsets, &weights, controls,
                        )
                        .unwrap(),
                );
            }
            let surface = resident.as_mut().unwrap();
            surface.upload_source_mesh(&queue, &mesh).unwrap();
            let mut stale_encoder = device.create_command_encoder(&Default::default());
            assert!(matches!(
                surface.encode(&mut stale_encoder),
                Err(voxy_render::SceneError::InvalidGeometry)
            ));
            let bind_reference =
                voxy_render::SceneMesh::new(demo.vertices.clone(), demo.indices.clone())
                    .unwrap()
                    .with_normals(demo.normals.iter().map(|n| n.to_array()).collect())
                    .unwrap();
            if tick == 1 {
                surface
                    .set_normal_reference(&device, &bind_reference)
                    .unwrap();
            }
            // Rejected replacements must preserve the accepted bind-space normal
            // operator; the GPU/CPU comparison below verifies its continued use.
            let mut reversed_indices = demo.indices.clone();
            reversed_indices.swap(0, 1);
            let wrong_topology =
                voxy_render::SceneMesh::new(demo.vertices.clone(), reversed_indices).unwrap();
            assert!(matches!(
                surface.set_normal_reference(&device, &wrong_topology),
                Err(voxy_render::SceneError::InvalidGeometry)
            ));
            let wrong_count = voxy_render::SceneMesh::quad([1.; 4]);
            assert!(matches!(
                surface.set_normal_reference(&device, &wrong_count),
                Err(voxy_render::SceneError::InvalidGeometry)
            ));
            assert!(matches!(
                surface.set_normal_reference(&foreign_device, &bind_reference),
                Err(voxy_render::SceneError::InvalidGeometry)
            ));

            let stance = demo.physical_stance.as_ref().unwrap();
            let palette = stance.palette.clone();
            let actor_offset = stance.offset.as_vec3();
            if tick == 1 {
                surface
                    .enable_rigid_skinning(
                        &device,
                        mesh.authored_normals().unwrap(),
                        &demo.gpu_jump_weights(),
                        palette.len() as u32,
                        None,
                    )
                    .unwrap();
            }
            surface
                .update_controls(&queue, &demo.gpu_shell_controls().unwrap())
                .unwrap();
            let mut missing_pose = device.create_command_encoder(&Default::default());
            assert!(matches!(
                surface.encode(&mut missing_pose),
                Err(voxy_render::SceneError::InvalidTransform)
            ));
            surface
                .upload_rigid_pose_with_offset(&queue, &palette, actor_offset)
                .unwrap();
            assert!(matches!(
                surface.upload_source_mesh(&queue, &wrong_topology),
                Err(voxy_render::SceneError::InvalidGeometry)
            ));
            assert!(matches!(
                surface.upload_source_mesh(&queue, &wrong_count),
                Err(voxy_render::SceneError::InvalidGeometry)
            ));
            assert!(matches!(
                surface.upload_rigid_pose_with_offset(&queue, &palette, Vec3::splat(f32::NAN)),
                Err(voxy_render::SceneError::InvalidTransform)
            ));
            // Failed updates must leave the already accepted pose usable.

            let mut reference = demo.posed_surface_vertices();
            let delta = demo.surface_displacement().unwrap();
            let count = reference.len();
            demo.finish_surface_vertices(&mut reference, &delta, count);
            let mut encoder = device.create_command_encoder(&Default::default());
            surface.encode(&mut encoder).unwrap();
            let expected_normals = demo.prepared_normals.transport(&reference, &demo.normals);
            let bytes_len = (count * 36) as u64;
            let normal_bytes = (count * 12) as u64;
            let dispatch = voxy_render::ComputeDispatch::gather_buffers(
                &device,
                &mut encoder,
                &[
                    (surface.vertex_buffer(), 0, 0, bytes_len),
                    (surface.normal_buffer(), 0, bytes_len, normal_bytes),
                    (
                        surface.vertex_buffer(),
                        bytes_len,
                        bytes_len + normal_bytes,
                        108,
                    ),
                    (
                        surface.normal_buffer(),
                        normal_bytes,
                        bytes_len + normal_bytes + 108,
                        36,
                    ),
                ],
                bytes_len + normal_bytes + 144,
            )
            .unwrap();
            queue.submit([encoder.finish()]);
            let mut pending = dispatch.begin_read();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let bytes = pending.try_read().unwrap().unwrap();
            let tail = &bytes[(bytes_len + normal_bytes) as usize..];
            let expected_tail: Vec<_> = (0..3)
                .map(|i| voxy_render::SceneVertex {
                    position: [100. + i as f32, 100., 100.],
                    uv: [0.25, 0.75],
                    color: [0.1, 0.2, 0.3, 1.],
                })
                .collect();
            let expected_bytes: Vec<u8> = expected_tail
                .iter()
                .flat_map(|v| v.position.into_iter().chain(v.uv).chain(v.color))
                .flat_map(f32::to_le_bytes)
                .collect();
            let expected_tail_normal_bytes: Vec<u8> = [0f32, 0., 1.]
                .repeat(3)
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect();
            assert_eq!(&tail[..108], expected_bytes.as_slice());
            assert_eq!(&tail[108..], expected_tail_normal_bytes.as_slice());
            let mut worst = (0usize, 0f32);
            for (i, (actual, expected)) in bytes[..bytes_len as usize]
                .chunks_exact(36)
                .zip(&reference)
                .enumerate()
            {
                let values: Vec<f32> = actual
                    .chunks_exact(4)
                    .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                    .collect();
                let error = Vec3::from_slice(&values).distance(Vec3::from(expected.position));
                if error > worst.1 {
                    worst = (i, error);
                }
                assert_eq!([values[3], values[4]], expected.uv);
                assert_eq!(&values[5..9], &expected.color);
            }
            let mut normal_error = 0f32;
            for (actual, expected) in bytes[bytes_len as usize..(bytes_len + normal_bytes) as usize]
                .chunks_exact(12)
                .zip(&expected_normals)
            {
                let n = Vec3::new(
                    f32::from_le_bytes(actual[0..4].try_into().unwrap()),
                    f32::from_le_bytes(actual[4..8].try_into().unwrap()),
                    f32::from_le_bytes(actual[8..12].try_into().unwrap()),
                );
                assert!(n.is_finite());
                normal_error = normal_error.max(n.distance(*expected));
            }
            eprintln!("FULL RIG SHELL NORMAL tick={tick} max_vector_error={normal_error}");
            assert!(
                normal_error < 1e-3,
                "authored normal transport differs from CPU: {normal_error}"
            );
            let cpu_mesh = voxy_render::SceneMesh::new(reference.clone(), demo.indices.clone())
                .unwrap()
                .with_material_coordinates(demo.vertices.iter().map(|v| v.position).collect())
                .unwrap()
                .with_normals(expected_normals.iter().map(|n| n.to_array()).collect())
                .unwrap();
            let cpu_geometry = renderer.upload_mesh(&device, &cpu_mesh).unwrap();
            let gpu_normals: Vec<[f32; 3]> = bytes
                [bytes_len as usize..(bytes_len + normal_bytes) as usize]
                .chunks_exact(12)
                .map(|v| {
                    std::array::from_fn(|k| {
                        f32::from_le_bytes(v[k * 4..k * 4 + 4].try_into().unwrap())
                    })
                })
                .collect();
            let hybrid_mesh = voxy_render::SceneMesh::new(reference.clone(), demo.indices.clone())
                .unwrap()
                .with_material_coordinates(demo.vertices.iter().map(|v| v.position).collect())
                .unwrap()
                .with_normals(gpu_normals)
                .unwrap();
            let hybrid = renderer.upload_mesh(&device, &hybrid_mesh).unwrap();
            let all_error = compare_body_pixels(
                &device,
                &queue,
                &renderer,
                &cpu_geometry,
                surface.geometry(),
                tick,
                "full",
            );
            let normal_only = compare_body_pixels(
                &device,
                &queue,
                &renderer,
                &cpu_geometry,
                &hybrid,
                tick,
                "normal-only",
            );
            let position_only = compare_body_pixels(
                &device,
                &queue,
                &renderer,
                &hybrid,
                surface.geometry(),
                tick,
                "position-only",
            );
            eprintln!(
                "BODY PIXEL ISOLATION full={all_error} normal_only={normal_only} position_only={position_only}"
            );
            maximum_pixel_error = maximum_pixel_error.max(all_error);
            maximum = maximum.max(worst.1);
            eprintln!(
                "FULL RIG SHELL GPU tick={tick} vertices={count} worst_vertex={} max_position_error_m={}",
                worst.0, worst.1
            );
        }
        assert!(
            maximum_pixel_error <= 1,
            "full body rendered mismatch: {maximum_pixel_error}"
        );
        assert!(
            maximum < 3e-7,
            "full rig/shell composition differs from CPU: {maximum}"
        );
    });
}

#[test]
fn gpu_rigid_blend_equal_weights_matches_cpu_hemisphere_anchor() {
    pollster::block_on(async {
        let instance = voxy_render::GraphicsOptions::default().create_instance();
        let adapter = instance.request_adapter(&Default::default()).await.unwrap();
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        let points = [[0.1, 0., 0.], [0., 0.1, 0.], [0., 0., 0.1]];
        let mesh = voxy_render::SceneMesh::new(
            points
                .map(|position| voxy_render::SceneVertex {
                    position,
                    uv: [0.; 2],
                    color: [1.; 4],
                })
                .to_vec(),
            vec![0, 1, 2],
        )
        .unwrap();
        let renderer = voxy_render::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        let mut surface = renderer
            .prepare_surface_deformation(&device, &mesh, &[0, 0, 0, 0], &[], 1)
            .unwrap();
        let palette = [
            Mat4::IDENTITY,
            Mat4::from_rotation_z(200f32.to_radians()),
            Mat4::from_rotation_z(100f32.to_radians()),
        ];
        let row = voxy_render::SurfaceRigidSkinWeight {
            joints: [0, 1, 2, 0],
            weights: [0.4, 0.4, 0.2, 0.],
        };
        surface
            .enable_rigid_skinning(&device, &[[0., 0., 1.]; 3], &[row; 3], 3, None)
            .unwrap();
        surface.upload_rigid_pose(&queue, &palette).unwrap();
        surface.update_controls(&queue, &[[0.; 4]]).unwrap();
        let transforms: Vec<_> = palette
            .into_iter()
            .map(crate::rig_skinning::RigidSkinTransform::from_matrix)
            .collect();
        let mut encoder = device.create_command_encoder(&Default::default());
        surface.encode(&mut encoder).unwrap();
        let dispatch = voxy_render::ComputeDispatch::gather_buffers(
            &device,
            &mut encoder,
            &[(surface.vertex_buffer(), 0, 0, 108)],
            108,
        )
        .unwrap();
        queue.submit([encoder.finish()]);
        let mut pending = dispatch.begin_read();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let bytes = pending.try_read().unwrap().unwrap();
        let mut maximum = 0f32;
        for (actual, point) in bytes.chunks_exact(36).zip(points) {
            let position: [f32; 3] = std::array::from_fn(|k| {
                f32::from_le_bytes(actual[k * 4..k * 4 + 4].try_into().unwrap())
            });
            let expected = crate::rig_skinning::deform_point(
                Vec3::from(point),
                &[(0, 0.4), (1, 0.4), (2, 0.2), (0, 0.)],
                &transforms,
            );
            maximum = maximum.max(Vec3::from(position).distance(expected));
        }
        assert!(
            maximum < 3e-7,
            "equal-weight hemisphere mismatch: {maximum}"
        );
    });
}

/// Render the full base surface through the selected scene material. Geometry
/// and normal transport must both reach the draw buffers before comparison.
fn compare_body_pixels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &SceneRenderer,
    cpu: &voxy_render::SceneGeometry,
    gpu: &voxy_render::SceneGeometry,
    tick: usize,
    label: &str,
) -> u8 {
    const SIZE: u32 = 512;
    let samples = if std::env::var_os("VOXY_BODY_SINGLE_SAMPLE_DIAGNOSTIC").is_some() {
        1
    } else {
        4
    };
    let make = |format, usage, sample_count| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("full rig body image comparison"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = make(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        1,
    );
    let depth = make(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        1,
    );
    let msaa_color = make(
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        samples,
    );
    let msaa_depth = make(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        samples,
    );
    let size = u64::from(SIZE * SIZE * 4);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("body pixel comparison"),
        size: size * 2,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let texture = renderer
        .upload_texture(device, queue, 1, 1, &[255; 4])
        .unwrap();
    let center = Vec3::new(0., 0.2, 0.);
    let eye = center + Vec3::new(0.6, 0.1, 2.);
    let transform = renderer
        .create_transform(
            device,
            glam::camera::rh::proj::directx::orthographic(-0.65, 0.65, -0.8, 0.8, 0.01, 5.)
                * glam::camera::rh::view::look_at_mat4(eye, center, Vec3::Y),
        )
        .unwrap();
    transform.update_view_position(queue, eye).unwrap();
    let mut encoder = device.create_command_encoder(&Default::default());
    for (frame, geometry) in [cpu, gpu].into_iter().enumerate() {
        let draws = [SceneDraw {
            geometry,
            texture: &texture,
            transform: &transform,
            overlay: false,
        }];
        let views = [SceneView {
            viewport: [0, 0, SIZE, SIZE],
            draws: &draws,
        }];
        if samples == 4 {
            renderer
                .encode_view_frame_msaa4(
                    &mut encoder,
                    (
                        &msaa_color.create_view(&Default::default()),
                        &msaa_depth.create_view(&Default::default()),
                    ),
                    &color.create_view(&Default::default()),
                    &depth.create_view(&Default::default()),
                    wgpu::Color::BLACK,
                    &views,
                    &[],
                )
                .unwrap();
        } else {
            renderer
                .encode_view_frame(
                    &mut encoder,
                    &color.create_view(&Default::default()),
                    &depth.create_view(&Default::default()),
                    wgpu::Color::BLACK,
                    &views,
                    &[],
                )
                .unwrap();
        }
        encoder.copy_texture_to_buffer(
            color.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: frame as u64 * size,
                    bytes_per_row: Some(SIZE * 4),
                    rows_per_image: Some(SIZE),
                },
            },
            color.size(),
        );
    }
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    rx.recv().unwrap().unwrap();
    let data = readback.slice(..).get_mapped_range().unwrap();
    let (reference, actual) = data.split_at(size as usize);
    let occupied = reference
        .chunks_exact(4)
        .filter(|p| p[..3].iter().any(|v| *v > 0))
        .count();
    let maximum = reference
        .iter()
        .zip(actual)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    let differing = reference.iter().zip(actual).filter(|(a, b)| a != b).count();
    eprintln!(
        "FULL RIG BODY PIXELS samples={samples} label={label} tick={tick} occupied={occupied} maximum_u8_error={maximum} differing_channels={differing}"
    );
    assert!(
        occupied > 1000,
        "empty rendering cannot qualify body pixels"
    );
    for (i, (a, b)) in reference
        .chunks_exact(4)
        .zip(actual.chunks_exact(4))
        .enumerate()
        .filter(|(_, (a, b))| a.iter().zip(*b).any(|(x, y)| x.abs_diff(*y) > 1))
        .take(12)
    {
        eprintln!(
            "BODY PIXEL label={label} x={} y={} CPU={a:?} GPU={b:?}",
            i % SIZE as usize,
            i / SIZE as usize
        );
    }
    drop(data);
    readback.unmap();
    maximum
}

#[test]
#[ignore = "full facial detail and resident body GPU composition"]
fn resident_body_preserves_actual_facial_details_on_two_poses() {
    pollster::block_on(async {
        let mut demo = FemaleDemo::new().unwrap();
        demo.enable_physical_stance().unwrap();
        let instance = voxy_render::GraphicsOptions::default().create_instance();
        let adapter = instance.request_adapter(&Default::default()).await.unwrap();
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        let mut renderer = SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
        renderer.enable_msaa4(&device).await.unwrap();
        let diagnostic_material = std::env::var("VOXY_BODY_MATERIAL_OVERRIDE")
            .ok()
            .map(|p| std::fs::read_to_string(p).expect("explicit body diagnostic material"));
        renderer
            .reload_shader(
                &device,
                diagnostic_material
                    .as_deref()
                    .unwrap_or(include_str!("female_material.wgsl")),
            )
            .await
            .unwrap();
        let mut resident: Option<super::resident_body::ResidentBody> = None;
        let mut target: Option<voxy_render::SceneGeometry> = None;
        let mut maximum = 0u8;
        let ticks = if let Ok(list) = std::env::var("VOXY_BODY_POSE_LIST") {
            list.split(',')
                .map(|n| n.parse::<usize>().expect("explicit pose number"))
                .collect()
        } else if std::env::var_os("VOXY_BODY_ALL_POSES").is_some() {
            (1..=720).collect()
        } else if std::env::var_os("VOXY_BODY_EXTENDED_POSES").is_some() {
            vec![1, 31, 61, 121, 181, 241, 361, 481, 601, 720]
        } else {
            vec![1, 31]
        };
        for tick in ticks {
            while demo.steps < tick {
                demo.advance(crate::female_physical_stance::STEP).unwrap();
            }
            // Diagnostic publication only: keep the accepted physical evolution,
            // but isolate skeletal motion from shell/support secondary offsets.
            let accepted_rig_pose_only = demo.rig_pose_only;
            let skeletal_diagnostic = std::env::var_os("VOXY_BODY_SKELETAL_DIAGNOSTIC").is_some();
            if skeletal_diagnostic {
                demo.rig_pose_only = true;
            }
            let accepted_actor_offset = demo.physical_stance.as_ref().unwrap().offset;
            let zero_actor_diagnostic =
                std::env::var_os("VOXY_BODY_ZERO_ACTOR_DIAGNOSTIC").is_some();
            if zero_actor_diagnostic {
                demo.physical_stance.as_mut().unwrap().offset = glam::DVec3::ZERO;
            }
            let full = demo
                .mesh_without_hair()
                .unwrap()
                .with_prepared_upload_streams();
            let count = demo.vertices.len();
            assert!(full.vertices().len() > count);
            if let Some(target_geom) = target.as_mut() {
                assert!(
                    target_geom
                        .update_after_deforming_prefix(&queue, &full, full.vertices().len() + 1)
                        .is_err()
                );
                target_geom
                    .update_after_deforming_prefix(&queue, &full, count)
                    .unwrap();
                resident
                    .as_mut()
                    .unwrap()
                    .refresh(&queue, &demo, &full)
                    .unwrap();
            } else {
                let geometry = renderer.upload_compute_mesh(&device, &full).unwrap();
                resident = Some(
                    super::resident_body::ResidentBody::new(
                        &device, &queue, &renderer, &demo, &full, &geometry,
                    )
                    .unwrap(),
                );
                target = Some(geometry);
            }
            let tail = full.vertices().len() - count;
            let vertex_bytes = (tail * 36) as u64;
            let normal_bytes = (tail * 12) as u64;
            let mut encoder = device.create_command_encoder(&Default::default());
            resident.as_mut().unwrap().encode(&mut encoder).unwrap();
            let surface = resident.as_ref().unwrap().surface();
            let dispatch = voxy_render::ComputeDispatch::gather_buffers(
                &device,
                &mut encoder,
                &[
                    (
                        surface.vertex_buffer(),
                        (count * 36) as u64,
                        0,
                        vertex_bytes,
                    ),
                    (
                        surface.normal_buffer(),
                        (count * 12) as u64,
                        vertex_bytes,
                        normal_bytes,
                    ),
                ],
                vertex_bytes + normal_bytes,
            )
            .unwrap();
            queue.submit([encoder.finish()]);
            let mut pending = dispatch.begin_read();
            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
            let bytes = pending.try_read().unwrap().unwrap();
            let expected_vertices: Vec<u8> = full.vertices()[count..]
                .iter()
                .flat_map(|v| v.position.into_iter().chain(v.uv).chain(v.color))
                .flat_map(f32::to_le_bytes)
                .collect();
            let expected_normals: Vec<u8> = full.authored_normals().unwrap()[count..]
                .iter()
                .flatten()
                .copied()
                .flat_map(f32::to_le_bytes)
                .collect();
            assert_eq!(
                &bytes[..vertex_bytes as usize],
                expected_vertices.as_slice()
            );
            assert_eq!(&bytes[vertex_bytes as usize..], expected_normals.as_slice());
            if let Ok(directory) = std::env::var("VOXY_BODY_GEOMETRY_CAPTURE") {
                let directory = std::path::Path::new(&directory);
                std::fs::create_dir_all(directory).unwrap();
                let mut capture_encoder = device.create_command_encoder(&Default::default());
                let size = (full.vertices().len() * 36) as u64;
                let capture = voxy_render::ComputeDispatch::gather_buffers(
                    &device,
                    &mut capture_encoder,
                    &[(surface.vertex_buffer(), 0, 0, size)],
                    size,
                )
                .unwrap();
                queue.submit([capture_encoder.finish()]);
                let mut read = capture.begin_read();
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                let gpu = read.try_read().unwrap().unwrap();
                if let Some(buffer) = surface.rigid_vertex_buffer() {
                    let mut encoder = device.create_command_encoder(&Default::default());
                    let size = (count * 36) as u64;
                    let capture = voxy_render::ComputeDispatch::gather_buffers(
                        &device,
                        &mut encoder,
                        &[(buffer, 0, 0, size)],
                        size,
                    )
                    .unwrap();
                    queue.submit([encoder.finish()]);
                    let mut read = capture.begin_read();
                    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                    let bytes = read.try_read().unwrap().unwrap();
                    std::fs::write(directory.join(format!("gpu-rigid-{tick}.vertices")), bytes)
                        .unwrap();
                    let mut cpu_rigid = demo.pre_skin_surface_vertices();
                    let stance = demo.physical_stance.as_ref().unwrap();
                    demo.rig
                        .deform_pose(&mut cpu_rigid, demo.time as f32, &stance.pose);
                    for v in &mut cpu_rigid {
                        v.position = (Vec3::from_array(v.position).as_dvec3() + stance.offset)
                            .as_vec3()
                            .to_array();
                    }
                    let bytes: Vec<u8> = cpu_rigid
                        .iter()
                        .flat_map(|v| v.position.into_iter().chain(v.uv).chain(v.color))
                        .flat_map(f32::to_le_bytes)
                        .collect();
                    std::fs::write(directory.join(format!("cpu-rigid-{tick}.vertices")), bytes)
                        .unwrap();
                    let source = demo.pre_skin_surface_vertices();
                    let rows = demo.gpu_jump_weights();
                    let palette: Vec<_> = stance
                        .palette
                        .iter()
                        .map(|m| {
                            let (_, real, t) = m.to_scale_rotation_translation();
                            let real = real.normalize();
                            let dual = glam::Quat::from_xyzw(t.x, t.y, t.z, 0.) * real * 0.5;
                            (real, dual)
                        })
                        .collect();
                    let probes:Vec<_>=[23328usize,23326,23259,23262,405,29983].into_iter().map(|id| {
                        let row=rows[id];let anchor=(0..4).max_by(|a,b|row.weights[*a].total_cmp(&row.weights[*b])).unwrap();
                        let reference=palette[row.joints[anchor] as usize].0;
                        let mut real=glam::Quat::from_xyzw(0.,0.,0.,0.);let mut dual=real;
                        for k in 0..4 {let w=row.weights[k];if w==0. {continue;}
                            let(r,d)=palette[row.joints[k] as usize];let w=if reference.dot(r)<0. {-w}else{w};real += r*w;dual += d*w;
                        }
                        let accumulated_real=real.to_array();let accumulated_dual=dual.to_array();let length=real.length();
                        real /= length;dual /= length;dual=dual-real*real.dot(dual);
                        let translation=dual*real.conjugate()*2.;let rotated=real*Vec3::from_array(source[id].position);
                        serde_json::json!({"vertex":id,"source":source[id].position,"joints":row.joints,"weights":row.weights,"accumulated_real":accumulated_real,"accumulated_dual":accumulated_dual,"length":length,"normalized_real":real.to_array(),"orthogonalized_dual":dual.to_array(),"translation":translation.to_array(),"rotated":rotated.to_array(),"cpu_rigid":cpu_rigid[id].position})
                    }).collect();
                    let palette: Vec<_> = palette
                        .iter()
                        .map(|(r, d)| serde_json::json!({"real":r.to_array(),"dual":d.to_array()}))
                        .collect();
                    let stages = serde_json::json!({"tick":tick,"actor_offset":stance.offset.to_array(),"palette":palette,"probes":probes});
                    std::fs::write(
                        directory.join(format!("rigid-inputs-{tick}.json")),
                        serde_json::to_vec_pretty(&stages).unwrap(),
                    )
                    .unwrap();
                }
                let cpu: Vec<u8> = full
                    .vertices()
                    .iter()
                    .flat_map(|v| v.position.into_iter().chain(v.uv).chain(v.color))
                    .flat_map(f32::to_le_bytes)
                    .collect();
                let indices: Vec<u8> = full
                    .indices()
                    .iter()
                    .copied()
                    .flat_map(u32::to_le_bytes)
                    .collect();
                std::fs::write(directory.join(format!("cpu-{tick}.vertices")), cpu).unwrap();
                std::fs::write(directory.join(format!("gpu-{tick}.vertices")), gpu).unwrap();
                std::fs::write(directory.join(format!("mesh-{tick}.indices")), indices).unwrap();
                let center = Vec3::new(0., 0.2, 0.);
                let eye = center + Vec3::new(0.6, 0.1, 2.);
                let mvp =
                    glam::camera::rh::proj::directx::orthographic(-0.65, 0.65, -0.8, 0.8, 0.01, 5.)
                        * glam::camera::rh::view::look_at_mat4(eye, center, Vec3::Y);
                let metadata = serde_json::json!({"tick":tick,"mvp_columns":mvp.to_cols_array(),"viewport":[512,512],"vertex_stride":36,"base_vertices":count});
                std::fs::write(
                    directory.join(format!("capture-{tick}.json")),
                    serde_json::to_vec_pretty(&metadata).unwrap(),
                )
                .unwrap();
            }
            let cpu = renderer.upload_mesh(&device, &full).unwrap();
            if std::env::var_os("VOXY_BODY_NORMAL_ISOLATION").is_some() {
                let mut encoder = device.create_command_encoder(&Default::default());
                let size = (full.vertices().len() * 12) as u64;
                let capture = voxy_render::ComputeDispatch::gather_buffers(
                    &device,
                    &mut encoder,
                    &[(surface.normal_buffer(), 0, 0, size)],
                    size,
                )
                .unwrap();
                queue.submit([encoder.finish()]);
                let mut read = capture.begin_read();
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                let bytes = read.try_read().unwrap().unwrap();
                let normals: Vec<[f32; 3]> = bytes
                    .chunks_exact(12)
                    .map(|n| {
                        std::array::from_fn(|k| {
                            f32::from_le_bytes(n[k * 4..k * 4 + 4].try_into().unwrap())
                        })
                    })
                    .collect();
                let hybrid_mesh = full.clone().with_normals(normals).unwrap();
                let hybrid = renderer.upload_mesh(&device, &hybrid_mesh).unwrap();
                let normal_error = compare_body_pixels(
                    &device,
                    &queue,
                    &renderer,
                    &cpu,
                    &hybrid,
                    tick,
                    "normal-only",
                );
                let position_error = compare_body_pixels(
                    &device,
                    &queue,
                    &renderer,
                    &hybrid,
                    target.as_ref().unwrap(),
                    tick,
                    "position-only",
                );
                eprintln!(
                    "BODY ISOLATION tick={tick} normal_only={normal_error} position_only={position_error}"
                );
            }
            let error = compare_body_pixels(
                &device,
                &queue,
                &renderer,
                &cpu,
                target.as_ref().unwrap(),
                tick,
                "actual-facial-details",
            );
            eprintln!(
                "ACTUAL FACE GPU tick={tick} base_vertices={count} appended_vertices={tail} max_u8_error={error} skeletal_diagnostic={skeletal_diagnostic} zero_actor_diagnostic={zero_actor_diagnostic}"
            );
            demo.rig_pose_only = accepted_rig_pose_only;
            demo.physical_stance.as_mut().unwrap().offset = accepted_actor_offset;
            maximum = maximum.max(error);
            if tick == 31 {
                target.as_mut().unwrap().update(&queue, &full).unwrap();
                let mut restore_encoder = device.create_command_encoder(&Default::default());
                let size = (full.vertices().len() * 12) as u64;
                let restore = voxy_render::ComputeDispatch::gather_buffers(
                    &device,
                    &mut restore_encoder,
                    &[(surface.normal_buffer(), 0, 0, size)],
                    size,
                )
                .unwrap();
                queue.submit([restore_encoder.finish()]);
                let mut read = restore.begin_read();
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                let actual = read.try_read().unwrap().unwrap();
                let expected: Vec<u8> = full
                    .authored_normals()
                    .unwrap()
                    .iter()
                    .flatten()
                    .copied()
                    .flat_map(f32::to_le_bytes)
                    .collect();
                assert_eq!(
                    actual, expected,
                    "full update must restore all CPU normals after partial updates"
                );
                eprintln!(
                    "CPU NORMAL RESTORE vertices={} bitwise=true",
                    full.vertices().len()
                );
            }
        }
        assert!(
            maximum <= 1,
            "actual facial details/body mismatch: {maximum}"
        );
    });
}
