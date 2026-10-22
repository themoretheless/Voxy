use super::*;
use crate::frame_upload_test_support as support;
fn fixture(device: &wgpu::Device, queue: &wgpu::Queue, count: usize) -> NativeVegetation {
    use crate::archive_scene::{ArchiveMaterial, ArchiveVertex};
    let vertices = [[-0.2, 0., 0.], [0.2, 0., 0.], [0., 0.4, 0.]]
        .map(|position| ArchiveVertex {
            position,
            normal: [0., 0., 1.],
            tangent: [1., 0., 0., 1.],
            uv: [0.; 2],
        })
        .to_vec();
    NativeVegetation::new(
        device,
        queue,
        wgpu::TextureFormat::Rgba8Unorm,
        vec![
            ArchiveImageSource::Rgba(
                crate::ImageAsset::from_rgba(
                    1,
                    1,
                    vec![120, 210, 80, 255],
                    crate::ImageLimits::default(),
                )
                .unwrap(),
            ),
            ArchiveImageSource::Rgba(
                crate::ImageAsset::from_rgba(
                    1,
                    1,
                    vec![128, 128, 255, 255],
                    crate::ImageLimits::default(),
                )
                .unwrap(),
            ),
        ],
        vec![ArchiveMaterialSource {
            parameters: ArchiveMaterial::metallic_roughness([0.2, 0.7, 0.3, 1.], 0., 0.5, 0., -1.),
            images: [Some(0), None, Some(1), None],
            double_sided: true,
        }],
        vec![ArchiveMeshSource {
            vertices,
            indices: vec![0, 1, 2],
            instances: vec![ArchiveInstance::new(Mat4::IDENTITY); count],
            material: 0,
            center: Vec3::ZERO,
        }],
        (Vec3::splat(-1.), Vec3::ONE),
    )
    .unwrap()
}
fn camera(frame: u32) -> Camera {
    Camera {
        vp: Mat4::from_rotation_y(frame as f32 * 0.03).to_cols_array_2d(),
        inverse_vp: Mat4::IDENTITY.to_cols_array_2d(),
        shadow: Mat4::IDENTITY.to_cols_array_2d(),
        eye: [0., 0., 3., 1.],
        sun: [0.4, 0.8, 0.2, 0.],
        flags: [0., frame as f32 * 0.01, 0., 0.],
    }
}
fn probe(device: &wgpu::Device, native: &NativeVegetation) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {label:None,source:wgpu::ShaderSource::Wgsl(r"
        struct Camera { v:array<vec4f,15> }
        @group(0) @binding(0) var<uniform> camera:Camera;
        struct Out {@builtin(position) p:vec4f,@location(0) c:vec4f}
        @vertex fn vs(@builtin(instance_index) id:u32,@location(0) a:vec4f,@location(1) b:vec4f,
          @location(2) c:vec4f,@location(3) d:vec4f,@location(4) e:vec4f,@location(5) f:vec4f,@location(6) g:vec4f)->Out {
            var cv=vec4f(0.); for(var i=0u;i<15u;i++){cv+=camera.v[i]*f32(i+1u)*0.001;}
            var out:Out;out.p=vec4f((f32(id%64u)+0.5)/32.-1.,(f32(id/64u%16u)+0.5)/8.-1.,0.5,1.);
            out.c=vec4f(fract(abs((a+b*2.+c*3.+d*4.+e*5.+f*6.+g*7.)*0.023+cv)).xyz,1.);return out;
        }
        @fragment fn fs(in:Out)->@location(0) vec4f{return in.c;}
    ".into())});
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&native.pipeline.get_bind_group_layout(0))],
        immediate_size: 0,
    });
    let attributes = wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x4,2=>Float32x4,3=>Float32x4,4=>Float32x4,5=>Float32x4,6=>Float32x4];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<ArchiveInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &attributes,
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::PointList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}
fn draw(
    native: &NativeVegetation,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    output: &wgpu::TextureView,
    frame: u32,
) {
    let (group, instances) = native.input_buffers(encoder, queue, &camera(frame));
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: output,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &group, &[]);
    pass.set_vertex_buffer(0, instances[0].slice(..));
    pass.draw(0..1, 0..native.meshes[0].instance_count);
}
#[test]
#[ignore = "physical Apple Metal; cameras and instance fetch parity"]
fn gpu_frame_vegetation_instances_and_camera_match_legacy() {
    let (_, device, queue) = support::device();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut shared = fixture(&device, &queue, 8);
    let mut legacy = fixture(&device, &queue, 8);
    assert!(shared.camera_ring.is_some() && shared.meshes[0].instance_ring.is_some());
    legacy.camera_ring = None;
    legacy.meshes[0].instance_ring = None;
    let pipeline = probe(&device, &shared);
    let output = support::output(&device);
    let view = output.create_view(&Default::default());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4096 * 12,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut pending = Vec::new();
    for frame in 0..6 {
        let count = [8, 2, 0, 6, 3, 7][frame as usize];
        let data: Vec<Vec<ArchiveInstance>> = vec![
            (0..count)
                .map(|i| {
                    ArchiveInstance::new(Mat4::from_rotation_translation(
                        glam::Quat::from_rotation_y(frame as f32 * 0.11),
                        Vec3::new(i as f32 * 0.03, frame as f32 * 0.2, 0.),
                    ))
                })
                .collect(),
        ];
        for (kind, native) in [(0, &mut legacy), (1, &mut shared)] {
            native.update_instances(&queue, &data).unwrap();
            let invalid = vec![vec![ArchiveInstance {
                world: [[f32::NAN; 4]; 4],
                normal: [[0.; 4]; 3],
            }]];
            assert!(native.update_instances(&queue, &invalid).is_err());
            let mut encoder = device.create_command_encoder(&Default::default());
            draw(native, &queue, &mut encoder, &pipeline, &view, frame);
            support::capture(
                &mut encoder,
                &output,
                &read,
                (frame as u64 * 2 + kind) * 4096,
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
    for pair in actual.chunks_exact(8192) {
        assert_eq!(&pair[..4096], &pair[4096..]);
    }
    assert!(
        actual
            .chunks_exact(4096)
            .any(|image| image != &actual[..4096])
    );
    assert!(pollster::block_on(scope.pop()).is_none());
}
#[test]
#[ignore = "manual alternating camera/instance production input benchmarks"]
fn paired_frame_vegetation_benchmark() {
    let (_, device, queue) = support::device();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let output = support::output(&device);
    let view = output.create_view(&Default::default());
    println!("path,count,pair,mode,cpu_us,drained_us");
    for (path, count) in [
        ("vegetation_camera", 1),
        ("vegetation_instances", 64),
        ("vegetation_instances", 4096),
    ] {
        let mut shared = fixture(&device, &queue, count);
        let mut legacy = fixture(&device, &queue, count);
        assert!(shared.camera_ring.is_some() && shared.meshes[0].instance_ring.is_some());
        legacy.camera_ring = None;
        legacy.meshes[0].instance_ring = None;
        if path == "vegetation_camera" {
            shared.meshes[0].instance_ring = None;
        } else {
            shared.camera_ring = None;
        }
        let pipeline = probe(&device, &shared);
        let mut data = vec![vec![ArchiveInstance::new(Mat4::IDENTITY); count]];
        support::pairs(
            &device,
            &queue,
            path,
            count,
            |use_shared, frame, encoder| {
                let native = if use_shared { &mut shared } else { &mut legacy };
                if path != "vegetation_camera" {
                    data[0][0] =
                        ArchiveInstance::new(Mat4::from_translation(Vec3::X * frame as f32 * 0.01));
                    native.update_instances(&queue, &data).unwrap();
                }
                draw(native, &queue, encoder, &pipeline, &view, frame);
            },
        );
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}
#[test]
#[ignore = "physical Apple Metal; cameras and instance fetch parity"]
fn gpu_generated_vegetation_matches_snapshot_and_overflow() {
    let (_, device, queue) = support::device();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let mut shared = fixture(&device, &queue, 8);
    let mut legacy = fixture(&device, &queue, 8);
    assert!(shared.camera_ring.is_some() && shared.meshes[0].instance_ring.is_some());
    legacy.camera_ring = None;
    legacy.meshes[0].instance_ring = None;
    let pipeline = probe(&device, &shared);
    let output = support::output(&device);
    let view = output.create_view(&Default::default());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4096 * 12,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut pending = Vec::new();
    for frame in 0..6 {
        let count = [8, 2, 0, 6, 3, 7][frame as usize];
        let data: Vec<Vec<ArchiveInstance>> = vec![
            (0..count)
                .map(|i| {
                    ArchiveInstance::new(Mat4::from_rotation_translation(
                        glam::Quat::from_rotation_y(frame as f32 * 0.11),
                        Vec3::new(i as f32 * 0.03, frame as f32 * 0.2, 0.),
                    ))
                })
                .collect(),
        ];
        for (kind, native) in [(0, &mut legacy), (1, &mut shared)] {
            if kind == 1 {
                native
                    .update_instances_generated(
                        &queue,
                        std::iter::once((count, data[0].iter().copied())),
                    )
                    .unwrap();
                assert!(native.meshes[0].snapshot.is_empty());
                for component in 0..28 {
                    let mut invalid = ArchiveInstance::new(Mat4::IDENTITY);
                    if component < 16 {
                        invalid.world[component / 4][component % 4] = f32::INFINITY;
                    } else {
                        invalid.normal[(component - 16) / 4][component % 4] = f32::NEG_INFINITY;
                    }
                    assert!(
                        native
                            .update_instances_generated(
                                &queue,
                                std::iter::once((1, std::iter::once(invalid)))
                            )
                            .is_err()
                    );
                }
                let mut bad = data[0].clone();
                bad.push(ArchiveInstance {
                    world: [[f32::NAN; 4]; 4],
                    normal: [[0.; 4]; 3],
                });
                assert!(
                    native
                        .update_instances_generated(
                            &queue,
                            std::iter::once((bad.len(), bad.into_iter()))
                        )
                        .is_err()
                );
                assert!(
                    native
                        .update_instances_generated(
                            &queue,
                            std::iter::once((1, std::iter::empty()))
                        )
                        .is_err()
                );
                // Discarded encoders release only their own reference; this frame
                // remains usable, including repeated encodes before submission.
                let mut abandoned = device.create_command_encoder(&Default::default());
                draw(native, &queue, &mut abandoned, &pipeline, &view, frame);
                drop(abandoned.finish());
            } else {
                native
                    .update_instances_generated(
                        &queue,
                        std::iter::once((count, data[0].iter().copied())),
                    )
                    .unwrap();
            }
            let invalid = vec![vec![ArchiveInstance {
                world: [[f32::NAN; 4]; 4],
                normal: [[0.; 4]; 3],
            }]];
            assert!(native.update_instances(&queue, &invalid).is_err());
            let mut encoder = device.create_command_encoder(&Default::default());
            draw(native, &queue, &mut encoder, &pipeline, &view, frame);
            support::capture(
                &mut encoder,
                &output,
                &read,
                (frame as u64 * 2 + kind) * 4096,
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
    for pair in actual.chunks_exact(8192) {
        assert_eq!(&pair[..4096], &pair[4096..]);
    }
    assert!(
        actual
            .chunks_exact(4096)
            .any(|image| image != &actual[..4096])
    );
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "physical Metal paired snapshot versus generated instances"]
fn paired_generated_vegetation() {
    run_generated_vegetation(false);
}
#[test]
#[ignore = "CPU input generation into physical Apple shared buffers; no draw or submission"]
fn paired_generated_input_instances() {
    run_generated_vegetation(true);
}
fn run_generated_vegetation(input_only: bool) {
    let (_, device, queue) = support::device();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let output = support::output(&device);
    let view = output.create_view(&Default::default());
    for (case, count, control, overflow) in [
        ("instances", 64, false, false),
        ("instances", 4096, false, false),
        ("instances_aa", 4096, true, false),
        ("instances_overflow", 4096, false, true),
    ] {
        let mut before = fixture(&device, &queue, count);
        let mut after = fixture(&device, &queue, count);
        before.camera_ring = None;
        after.camera_ring = None;
        let pipeline = probe(&device, &before);
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let mut held = Vec::new();
        if overflow {
            for native in [&before, &after] {
                for _ in 0..3 {
                    held.push(
                        native.meshes[0]
                            .instance_ring
                            .as_ref()
                            .unwrap()
                            .prepare(112, |mut bytes| {
                                bytes.copy_from_slice(&[0; 112]);
                                Ok::<_, ComputeError>(())
                            })
                            .unwrap(),
                    );
                }
            }
        }
        let mut source = vec![ArchiveInstance::new(Mat4::IDENTITY); count];
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
                source[0] =
                    ArchiveInstance::new(Mat4::from_translation(Vec3::X * frame as f32 * 0.01));
                let native = if new { &mut after } else { &mut before };
                if new && !control {
                    native
                        .update_instances_generated(
                            &queue,
                            std::iter::once((count, source.iter().copied())),
                        )
                        .unwrap();
                    assert_eq!(
                        native.meshes[0].generated.as_ref().unwrap().is_direct(),
                        !overflow
                    );
                } else {
                    // Old app producer + renderer-owned CPU snapshot + encode copy.
                    native.update_instances(&queue, &[source.clone()]).unwrap();
                }
                if input_only {
                    let _ = native.instance_buffers(encoder);
                } else {
                    draw(native, &queue, encoder, &pipeline, &view, frame);
                }
            },
        );
        drop(held);
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[test]
#[ignore = "idle physical Metal GPU; complete 1080p production vegetation passes"]
fn paired_full_frame_instances() {
    use crate::full_frame_upload_bench as full;
    let (_, device, queue) = full::device();
    let target = full::Target::new(&device);
    let scene = crate::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
    let background = full::Background::new(&scene, &device, &queue);
    let mut before = fixture(&device, &queue, full::COUNT);
    let mut after = fixture(&device, &queue, full::COUNT);
    assert!(before.camera_ring.is_some() && after.meshes[0].instance_ring.is_some());
    let control = std::env::var("VOXY_FULL_PHASE").unwrap_or_else(|_| "aa".into()) == "aa";
    let mut source: Vec<_> = (0..full::COUNT)
        .map(|i| {
            ArchiveInstance::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.065),
                glam::Quat::IDENTITY,
                Vec3::new(
                    (i % 64) as f32 * 0.035 - 1.1,
                    (i / 64) as f32 * 0.03 - 0.95,
                    0.,
                ),
            ))
        })
        .collect();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    full::pairs(
        &device,
        &queue,
        &target,
        "instances",
        |new, frame, encoder| {
            source[0] = ArchiveInstance::new(Mat4::from_scale_rotation_translation(
                Vec3::splat(0.065),
                glam::Quat::IDENTITY,
                Vec3::new(-1.1 + frame as f32 * 0.0001, -0.95, 0.),
            ));
            let native = if new { &mut after } else { &mut before };
            native.frozen_benchmark_time = Some(frame as f32 / 120.);
            if new && !control {
                native
                    .update_instances_generated(
                        &queue,
                        std::iter::once((full::COUNT, source.iter().copied())),
                    )
                    .unwrap();
                assert!(native.meshes[0].generated.as_ref().unwrap().is_direct());
            } else {
                native.update_instances(&queue, &[source.clone()]).unwrap();
            }
            scene.encode(
                encoder,
                &target.view,
                &target.depth,
                wgpu::Color::BLACK,
                &[background.draw()],
            );
            native.encode(
                &device,
                &queue,
                encoder,
                &target.view,
                &target.depth,
                full::camera().view_projection().unwrap(),
                full::camera().eye,
                false,
            );
        },
    );
}
