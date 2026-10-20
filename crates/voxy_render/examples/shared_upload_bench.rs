//! Same-process alternating pairs: CPU production + submission, and drained wall time.
//! Run with --release. No window; real Metal compute consumes every generated word.
use glam::{Quat, Vec3};
use std::time::Instant;
use voxy_render::SharedUploadRing;

fn produce(bytes: &mut [u8], joints: &[(Quat, Vec3)], frame: u32) {
    let rows: &mut [[f32; 4]] = bytemuck::cast_slice_mut(bytes);
    for (rows, &(rotation, translation)) in rows.chunks_exact_mut(2).zip(joints) {
        let t = translation + Vec3::splat(frame as f32 * 0.0001);
        rows[0] = rotation.to_array();
        rows[1] = (Quat::from_xyzw(t.x, t.y, t.z, 0.) * rotation * 0.5).to_array();
    }
}
fn bind(
    device: &wgpu::Device,
    pipeline: &wgpu::ComputePipeline,
    input: &wgpu::Buffer,
    output: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    })
}
fn dispatch(
    pipeline: &wgpu::ComputePipeline,
    group: &wgpu::BindGroup,
    rows: u32,
    encoder: &mut wgpu::CommandEncoder,
) {
    let mut pass = encoder.begin_compute_pass(&Default::default());
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, group, &[]);
    pass.dispatch_workgroups(rows.div_ceil(64), 1, 1);
}
fn main() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    eprintln!("adapter={:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
        ..Default::default()
    }))
    .unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            r"
            @group(0) @binding(0) var<storage,read> input:array<vec4u>;
            @group(0) @binding(1) var<storage,read_write> output:array<vec4u>;
            @compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3u) {
                if id.x < arrayLength(&input) { output[id.x] = input[id.x] ^ vec4u(0x13579bdu); }
            }
        "
            .into(),
        ),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    println!("bytes,pair,path,cpu_us_per_frame,drained_us_per_frame");
    let frames = 48;
    for size in [1024_u64, 32768, 262144, 1048576] {
        let joints: Vec<_> = (0..size / 32)
            .map(|i| {
                (
                    Quat::from_rotation_y(i as f32 * 0.001),
                    Vec3::new(i as f32 * 0.001, 0.2, -0.3),
                )
            })
            .collect();
        let input = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let ring = SharedUploadRing::new(&device, size, wgpu::BufferUsages::STORAGE).unwrap();
        let legacy_group = bind(&device, &pipeline, &input, &output);
        let mut groups: [Option<wgpu::BindGroup>; 3] = [None, None, None];
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        for pair in 0..10 {
            for shared in if pair % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let total = Instant::now();
                let mut cpu = 0.;
                for frame in 0..frames {
                    let start = Instant::now();
                    let mut encoder = device.create_command_encoder(&Default::default());
                    if shared {
                        assert!(
                            ring.encode(
                                &mut encoder,
                                |bytes| {
                                    for (row, &(rotation, translation)) in
                                        bytes.into_chunks::<32>().0.into_iter().zip(&joints)
                                    {
                                        let t = translation + Vec3::splat(frame as f32 * 0.0001);
                                        let dual =
                                            Quat::from_xyzw(t.x, t.y, t.z, 0.) * rotation * 0.5;
                                        row.write(bytemuck::cast([
                                            rotation.to_array(),
                                            dual.to_array(),
                                        ]));
                                    }
                                },
                                |slot, buffer, encoder| {
                                    let group = groups[slot].get_or_insert_with(|| {
                                        bind(&device, &pipeline, buffer, &output)
                                    });
                                    dispatch(&pipeline, group, (size / 16) as u32, encoder);
                                }
                            )
                            .unwrap(),
                            "drained three-frame ring must have a ready slot"
                        );
                    } else {
                        let mut values = Vec::with_capacity((size / 16) as usize);
                        for &(rotation, translation) in &joints {
                            let t = translation + Vec3::splat(frame as f32 * 0.0001);
                            values.push(rotation.to_array());
                            values.push(
                                (Quat::from_xyzw(t.x, t.y, t.z, 0.) * rotation * 0.5).to_array(),
                            );
                        }
                        queue.write_buffer(&input, 0, bytemuck::cast_slice(&values));
                        dispatch(&pipeline, &legacy_group, (size / 16) as u32, &mut encoder);
                    }
                    queue.submit([encoder.finish()]);
                    cpu += start.elapsed().as_secs_f64() * 1e6;
                    if frame % 3 == 2 {
                        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                    }
                }
                let wall = total.elapsed().as_secs_f64() * 1e6;
                // Pair zero warms both paths, allocator, groups and shader.
                if pair != 0 {
                    println!(
                        "{size},{pair},{},{:.3},{:.3}",
                        if shared { "shared" } else { "legacy" },
                        cpu / f64::from(frames),
                        wall / f64::from(frames)
                    );
                }
                // Read all actual GPU output after every half-pair, outside timings.
                let read = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let mut encoder = device.create_command_encoder(&Default::default());
                encoder.copy_buffer_to_buffer(&output, 0, &read, 0, size);
                queue.submit([encoder.finish()]);
                let (tx, rx) = std::sync::mpsc::channel();
                read.slice(..)
                    .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                rx.recv().unwrap().unwrap();
                let actual = read.slice(..).get_mapped_range().unwrap();
                let mut expected = vec![0_u32; (size / 4) as usize];
                produce(bytemuck::cast_slice_mut(&mut expected), &joints, frames - 1);
                for v in &mut expected {
                    *v ^= 0x13579bd;
                }
                assert_eq!(bytemuck::cast_slice::<u8, u32>(&actual), &expected);
                drop(actual);
                read.unmap();
            }
        }
    }
    assert!(pollster::block_on(scope.pop()).is_none());
    eprintln!("all GPU words matched; validation clean");
}
