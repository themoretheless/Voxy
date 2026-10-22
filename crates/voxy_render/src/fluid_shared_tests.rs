use super::*;
use crate::frame_upload_test_support as support;
fn camera(frame: u32) -> crate::SceneCamera {
    crate::SceneCamera {
        eye: glam::Vec3::new(frame as f32 * 0.01, 0., 3.),
        target: glam::Vec3::ZERO,
        up: glam::Vec3::Y,
        projection: crate::SceneProjection::Perspective {
            vertical_fov: 1.,
            aspect: 4.,
            near: 0.1,
            far: 10.,
        },
    }
}
fn particles(count: usize, frame: u32) -> Vec<FluidRenderParticle> {
    (0..count)
        .map(|i| FluidRenderParticle {
            position_radius: [
                (i % 17) as f32 * 0.01 + frame as f32 * 0.02,
                (i / 17 % 13) as f32 * 0.01,
                0.,
                0.2,
            ],
            absorption_ior: [0.6, 0.2, 0.1, 1.33],
        })
        .collect()
}
#[test]
#[ignore = "physical Apple Silicon Metal; fluid pixel and snapshot parity"]
fn gpu_frame_fluid_snapshots_match_legacy_including_overflow() {
    let (adapter, device, queue) = support::device();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let scene = crate::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut shared = ScreenSpaceFluidRenderer::new_with_adapter(
        &device,
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        64,
        16,
        16,
    )
    .unwrap();
    let mut legacy = ScreenSpaceFluidRenderer::new_with_shared_budget(
        &device,
        wgpu::TextureFormat::Rgba8Unorm,
        64,
        16,
        16,
        0,
    )
    .unwrap();
    assert!(
        shared.camera_ring.is_some()
            && shared.particle_ring.is_some()
            && shared.film_ring.is_some()
    );
    let required = ScreenSpaceFluidRenderer::required_allocation_bytes(
        wgpu::TextureFormat::Rgba8Unorm,
        64,
        16,
        16,
    )
    .unwrap();
    assert!(shared.allocation_bytes() > required);
    let budgeted = ScreenSpaceFluidRenderer::new_with_adapter_budget(
        &device,
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        64,
        16,
        16,
        0,
        required,
    )
    .unwrap();
    assert!(budgeted.camera_ring.is_none() && budgeted.particle_ring.is_none());
    assert_eq!(budgeted.allocation_bytes(), required);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let target = support::output(&device);
    let view = target.create_view(&Default::default());
    let bytes = 64 * 16 * 4;
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes * 12,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut pending = Vec::new();
    for frame in 0..6 {
        let count = [5, 1, 0, 3, 2, 4][frame as usize];
        let data = particles(count, frame);
        let film = FluidRenderFilmTriangle::new(
            [[-0.3, -0.3, 0.1], [0.3, -0.3, 0.1], [0., 0.3, 0.1]],
            0.02,
            [0.2, 0.4, 0.1, 1.33],
        )
        .unwrap();
        let films = if frame % 2 == 0 {
            vec![film]
        } else {
            Vec::new()
        };
        for (kind, renderer) in [(0, &mut legacy), (1, &mut shared)] {
            renderer
                .update_with_film(
                    &queue,
                    camera(frame),
                    &data,
                    &films,
                    1.,
                    FluidDepthFilter::Bilateral,
                )
                .unwrap();
            assert!(
                renderer
                    .update(
                        &queue,
                        camera(frame),
                        &particles(17, frame),
                        1.,
                        FluidDepthFilter::None
                    )
                    .is_err()
            );
            let mut encoder = device.create_command_encoder(&Default::default());
            renderer.encode(&scene, &mut encoder, &view, wgpu::Color::BLACK, &[]);
            support::capture(
                &mut encoder,
                &target,
                &read,
                (frame as u64 * 2 + kind) * bytes,
            );
            if kind == 0 {
                queue.submit([encoder.finish()]);
            } else {
                pending.push(encoder.finish());
            }
        }
    }
    queue.submit(pending);
    let actual = support::read(&device, &read);
    for pair in actual.chunks_exact((bytes * 2) as usize) {
        assert_eq!(&pair[..bytes as usize], &pair[bytes as usize..]);
    }
    assert!(
        actual
            .chunks_exact(bytes as usize)
            .any(|image| image != &actual[..bytes as usize])
    );
    // Diagnostics get a fresh camera lease, including repeated post-submit use.
    let mut encoder = device.create_command_encoder(&Default::default());
    shared.encode_diagnostic(&mut encoder, &view, FluidDiagnostic::Thickness);
    let (pipeline, group, shared_output) = shared.create_compute_filter(&device).unwrap();
    let (legacy_pipeline, legacy_group, legacy_output) =
        legacy.create_compute_filter(&device).unwrap();
    shared.encode_compute_filter(&mut encoder, &pipeline, &group);
    legacy.encode_compute_filter(&mut encoder, &legacy_pipeline, &legacy_group);
    let compute_read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: shared_output.size() * 2,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(&shared_output, 0, &compute_read, 0, shared_output.size());
    encoder.copy_buffer_to_buffer(
        &legacy_output,
        0,
        &compute_read,
        shared_output.size(),
        shared_output.size(),
    );
    queue.submit([encoder.finish()]);
    let compute = support::read(&device, &compute_read);
    assert_eq!(
        &compute[..shared_output.size() as usize],
        &compute[shared_output.size() as usize..]
    );
    assert!(pollster::block_on(scope.pop()).is_none());
}
#[test]
#[ignore = "manual alternating production fluid particle/camera benchmarks"]
fn paired_frame_fluid_benchmark() {
    let (adapter, device, queue) = support::device();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let scene = crate::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let output = support::output(&device);
    let view = output.create_view(&Default::default());
    println!("path,count,pair,mode,cpu_us,drained_us");
    for (path, count) in [
        ("fluid_camera", 0),
        ("fluid_particles", 64),
        ("fluid_particles", 4096),
        ("fluid_films", 64),
        ("fluid_films", 1024),
    ] {
        let mut shared = ScreenSpaceFluidRenderer::new_with_adapter(
            &device,
            &adapter,
            wgpu::TextureFormat::Rgba8Unorm,
            64,
            16,
            count.max(1),
        )
        .unwrap();
        let mut legacy = ScreenSpaceFluidRenderer::new_with_shared_budget(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            64,
            16,
            count.max(1),
            0,
        )
        .unwrap();
        assert!(shared.camera_ring.is_some() && shared.particle_ring.is_some());
        if path != "fluid_camera" {
            shared.camera_ring = None;
        }
        // Isolate each changed upload; other inputs use the same legacy path.
        let film = FluidRenderFilmTriangle::new(
            [[-0.3, -0.3, 0.1], [0.3, -0.3, 0.1], [0., 0.3, 0.1]],
            0.02,
            [0.2, 0.4, 0.1, 1.33],
        )
        .unwrap();
        let films = if path == "fluid_films" {
            vec![film; count]
        } else {
            Vec::new()
        };
        // Include the compatibility API's owned CPU snapshot cost in each pair.
        let mut data = particles(if path == "fluid_films" { 0 } else { count }, 0);
        support::pairs(
            &device,
            &queue,
            path,
            count,
            |use_shared, frame, encoder| {
                if let Some(p) = data.first_mut() {
                    p.position_radius[0] = frame as f32 * 0.001;
                }
                let renderer = if use_shared { &mut shared } else { &mut legacy };
                renderer
                    .update_with_film(
                        &queue,
                        camera(frame),
                        &data,
                        &films,
                        1.,
                        FluidDepthFilter::None,
                    )
                    .unwrap();
                renderer.encode(&scene, encoder, &view, wgpu::Color::BLACK, &[]);
            },
        );
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}
#[test]
#[ignore = "physical Apple Silicon Metal; fluid pixel and snapshot parity"]
fn gpu_generated_fluid_matches_snapshot_and_overflow() {
    let (adapter, device, queue) = support::device();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let scene = crate::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let mut shared = ScreenSpaceFluidRenderer::new_with_adapter(
        &device,
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        64,
        16,
        16,
    )
    .unwrap();
    let mut legacy = ScreenSpaceFluidRenderer::new_with_shared_budget(
        &device,
        wgpu::TextureFormat::Rgba8Unorm,
        64,
        16,
        16,
        0,
    )
    .unwrap();
    assert!(
        shared.camera_ring.is_some()
            && shared.particle_ring.is_some()
            && shared.film_ring.is_some()
    );
    let required = ScreenSpaceFluidRenderer::required_allocation_bytes(
        wgpu::TextureFormat::Rgba8Unorm,
        64,
        16,
        16,
    )
    .unwrap();
    assert!(shared.allocation_bytes() > required);
    let budgeted = ScreenSpaceFluidRenderer::new_with_adapter_budget(
        &device,
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        64,
        16,
        16,
        0,
        required,
    )
    .unwrap();
    assert!(budgeted.camera_ring.is_none() && budgeted.particle_ring.is_none());
    assert_eq!(budgeted.allocation_bytes(), required);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let target = support::output(&device);
    let view = target.create_view(&Default::default());
    let bytes = 64 * 16 * 4;
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes * 12,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut pending = Vec::new();
    for frame in 0..6 {
        let count = [5, 1, 0, 3, 2, 4][frame as usize];
        let data = particles(count, frame);
        let film = FluidRenderFilmTriangle::new(
            [[-0.3, -0.3, 0.1], [0.3, -0.3, 0.1], [0., 0.3, 0.1]],
            0.02,
            [0.2, 0.4, 0.1, 1.33],
        )
        .unwrap();
        let films = if frame % 2 == 0 {
            vec![film]
        } else {
            Vec::new()
        };
        for (kind, renderer) in [(0, &mut legacy), (1, &mut shared)] {
            if kind == 1 {
                renderer
                    .update_generated_with_film(
                        &queue,
                        camera(frame),
                        count,
                        data.iter().copied(),
                        &films,
                        1.,
                        FluidDepthFilter::Bilateral,
                    )
                    .unwrap();
                assert!(renderer.particle_snapshot.is_empty());
                let mut invalid_camera = camera(frame);
                invalid_camera.eye.x = f32::NAN;
                assert!(
                    renderer
                        .update_generated_with_film(
                            &queue,
                            invalid_camera,
                            count,
                            data.iter().copied(),
                            &films,
                            1.,
                            FluidDepthFilter::Bilateral
                        )
                        .is_err()
                );
                let mut bad = data.clone();
                bad.push(FluidRenderParticle {
                    position_radius: [0., 0., 0., f32::NAN],
                    absorption_ior: [0., 0., 0., 1.33],
                });
                assert!(
                    renderer
                        .update_generated_with_film(
                            &queue,
                            camera(frame),
                            bad.len(),
                            bad.into_iter(),
                            &films,
                            1.,
                            FluidDepthFilter::Bilateral
                        )
                        .is_err()
                );
                assert!(
                    renderer
                        .update_generated_with_film(
                            &queue,
                            camera(frame),
                            1,
                            std::iter::empty(),
                            &films,
                            1.,
                            FluidDepthFilter::Bilateral
                        )
                        .is_err()
                );
                let mut abandoned = device.create_command_encoder(&Default::default());
                renderer.encode(&scene, &mut abandoned, &view, wgpu::Color::BLACK, &[]);
                drop(abandoned.finish());
            } else {
                renderer
                    .update_generated_with_film(
                        &queue,
                        camera(frame),
                        count,
                        data.iter().copied(),
                        &films,
                        1.,
                        FluidDepthFilter::Bilateral,
                    )
                    .unwrap();
            }
            assert!(
                renderer
                    .update(
                        &queue,
                        camera(frame),
                        &particles(17, frame),
                        1.,
                        FluidDepthFilter::None
                    )
                    .is_err()
            );
            let mut encoder = device.create_command_encoder(&Default::default());
            renderer.encode(&scene, &mut encoder, &view, wgpu::Color::BLACK, &[]);
            support::capture(
                &mut encoder,
                &target,
                &read,
                (frame as u64 * 2 + kind) * bytes,
            );
            if kind == 0 {
                queue.submit([encoder.finish()]);
            } else {
                pending.push(encoder.finish());
            }
        }
    }
    queue.submit(pending);
    let actual = support::read(&device, &read);
    for pair in actual.chunks_exact((bytes * 2) as usize) {
        assert_eq!(&pair[..bytes as usize], &pair[bytes as usize..]);
    }
    assert!(
        actual
            .chunks_exact(bytes as usize)
            .any(|image| image != &actual[..bytes as usize])
    );
    // Diagnostics get a fresh camera lease, including repeated post-submit use.
    let mut encoder = device.create_command_encoder(&Default::default());
    shared.encode_diagnostic(&mut encoder, &view, FluidDiagnostic::Thickness);
    let (pipeline, group, shared_output) = shared.create_compute_filter(&device).unwrap();
    let (legacy_pipeline, legacy_group, legacy_output) =
        legacy.create_compute_filter(&device).unwrap();
    shared.encode_compute_filter(&mut encoder, &pipeline, &group);
    legacy.encode_compute_filter(&mut encoder, &legacy_pipeline, &legacy_group);
    let compute_read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: shared_output.size() * 2,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(&shared_output, 0, &compute_read, 0, shared_output.size());
    encoder.copy_buffer_to_buffer(
        &legacy_output,
        0,
        &compute_read,
        shared_output.size(),
        shared_output.size(),
    );
    queue.submit([encoder.finish()]);
    let compute = support::read(&device, &compute_read);
    assert_eq!(
        &compute[..shared_output.size() as usize],
        &compute[shared_output.size() as usize..]
    );
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "physical Metal paired snapshot versus generated particles"]
fn paired_generated_particles() {
    run_generated_particles(false);
}
#[test]
#[ignore = "CPU input generation into physical Apple shared buffers; no draw or submission"]
fn paired_generated_input_particles() {
    run_generated_particles(true);
}
fn run_generated_particles(input_only: bool) {
    let (adapter, device, queue) = support::device();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let scene = crate::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let output = support::output(&device);
    let view = output.create_view(&Default::default());
    for (case, count, control, overflow) in [
        ("particles", 64, false, false),
        ("particles", 4096, false, false),
        ("particles_aa", 4096, true, false),
        ("particles_overflow", 4096, false, true),
    ] {
        let mut before = ScreenSpaceFluidRenderer::new_with_adapter(
            &device,
            &adapter,
            wgpu::TextureFormat::Rgba8Unorm,
            64,
            16,
            count,
        )
        .unwrap();
        let mut after = ScreenSpaceFluidRenderer::new_with_adapter(
            &device,
            &adapter,
            wgpu::TextureFormat::Rgba8Unorm,
            64,
            16,
            count,
        )
        .unwrap();
        if !input_only {
            before.camera_ring = None;
            after.camera_ring = None;
        }
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let mut held = Vec::new();
        if overflow {
            for renderer in [&before, &after] {
                for _ in 0..3 {
                    held.push(
                        renderer
                            .particle_ring
                            .as_ref()
                            .unwrap()
                            .prepare(32, |mut bytes| {
                                bytes.copy_from_slice(&[0; 32]);
                                Ok::<_, crate::ComputeError>(())
                            })
                            .unwrap(),
                    );
                }
            }
        }
        let case = if input_only {
            format!("input_{case}")
        } else {
            case.to_owned()
        };
        support::generation_pairs_mode(
            &device,
            &queue,
            &case,
            count,
            !input_only,
            |new, frame, encoder| {
                let renderer = if new { &mut after } else { &mut before };
                let values = (0..count).map(|i| FluidRenderParticle {
                    position_radius: [
                        (i % 17) as f32 * 0.01 + frame as f32 * 0.02,
                        (i / 17 % 13) as f32 * 0.01,
                        0.,
                        0.2,
                    ],
                    absorption_ior: [0.6, 0.2, 0.1, 1.33],
                });
                if new && !control {
                    renderer
                        .update_generated_with_film(
                            &queue,
                            camera(frame),
                            count,
                            values,
                            &[],
                            1.,
                            FluidDepthFilter::None,
                        )
                        .unwrap();
                    assert_eq!(
                        renderer.generated_particles.as_ref().unwrap().is_direct(),
                        !overflow
                    );
                } else {
                    renderer
                        .update_with_film(
                            &queue,
                            camera(frame),
                            &values.collect::<Vec<_>>(),
                            &[],
                            1.,
                            FluidDepthFilter::None,
                        )
                        .unwrap();
                }
                if input_only {
                    let _ = renderer.particle_buffers(encoder);
                } else {
                    renderer.encode(&scene, encoder, &view, wgpu::Color::BLACK, &[]);
                }
            },
        );
        drop(held);
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "idle physical Metal GPU; complete 1080p production fluid passes"]
fn paired_full_frame_particles() {
    use crate::full_frame_upload_bench as full;
    let (adapter, device, queue) = full::device();
    let target = full::Target::new(&device);
    let scene = crate::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let background = full::Background::new(&scene, &device, &queue);
    let mut before = ScreenSpaceFluidRenderer::new_with_adapter(
        &device,
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        full::WIDTH,
        full::HEIGHT,
        full::COUNT,
    )
    .unwrap();
    let mut after = ScreenSpaceFluidRenderer::new_with_adapter(
        &device,
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        full::WIDTH,
        full::HEIGHT,
        full::COUNT,
    )
    .unwrap();
    assert!(before.camera_ring.is_some() && after.particle_ring.is_some());
    let control = std::env::var("VOXY_FULL_PHASE").unwrap_or_else(|_| "aa".into()) == "aa";
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    full::pairs(
        &device,
        &queue,
        &target,
        "particles",
        |new, frame, encoder| {
            let renderer = if new { &mut after } else { &mut before };
            let values = (0..full::COUNT).map(|i| {
                let mass = 0.00003 + (i % 7) as f64 * 0.000001;
                let radius = (3. * mass / (4. * std::f64::consts::PI)).cbrt() as f32;
                FluidRenderParticle {
                    position_radius: [
                        (i % 64) as f32 * 0.035 - 1.1 + frame as f32 * 0.00001,
                        (i / 64) as f32 * 0.03 - 0.95,
                        0.,
                        radius,
                    ],
                    absorption_ior: [0.6, 0.2, 0.1, 1.33],
                }
            });
            if new && !control {
                renderer
                    .update_generated_with_film(
                        &queue,
                        full::camera(),
                        full::COUNT,
                        values,
                        &[],
                        1.,
                        FluidDepthFilter::Bilateral,
                    )
                    .unwrap();
                assert!(renderer.generated_particles.as_ref().unwrap().is_direct());
            } else {
                renderer
                    .update_with_film(
                        &queue,
                        full::camera(),
                        &values.collect::<Vec<_>>(),
                        &[],
                        1.,
                        FluidDepthFilter::Bilateral,
                    )
                    .unwrap();
            }
            renderer.encode(
                &scene,
                encoder,
                &target.view,
                wgpu::Color::BLACK,
                &[background.draw()],
            );
        },
    );
}
