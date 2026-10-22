//! Same-process relocation comparison against the frozen pre-move implementation.
use crate::SharedUploadRing;
use crate::frame_upload_test_support as support;
use crate::shared_upload_before as before;

fn group(
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
fn consume(
    pipeline: &wgpu::ComputePipeline,
    group: &wgpu::BindGroup,
    encoder: &mut wgpu::CommandEncoder,
    words: u32,
) {
    let mut pass = encoder.begin_compute_pass(&Default::default());
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, group, &[]);
    pass.dispatch_workgroups(words.div_ceil(64), 1, 1);
}
#[test]
#[ignore = "physical Metal; alternating pre-move versus core implementation, not legacy staging"]
fn paired_shared_upload_relocation() {
    let (_, device, queue) = support::device();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            r"
        @group(0) @binding(0) var<storage,read> input:array<u32>;
        @group(0) @binding(1) var<storage,read_write> output:array<u32>;
        @compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3u) {
            if id.x<arrayLength(&input) {output[id.x]=input[id.x]^0x13579bdu;}
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
    println!("case,bytes,pair,mode,upload_us,cpu_us,drained_us");
    for (name, size, overflow, control) in [
        ("camera", 320, false, false),
        ("instances", 112 * 4096, false, false),
        ("particles", 32 * 4096, false, false),
        ("overflow", 320, true, false),
        ("control_aa", 320, false, true),
    ] {
        let old =
            before::SharedUploadRing::new(&device, size, wgpu::BufferUsages::STORAGE).unwrap();
        let old_control =
            before::SharedUploadRing::new(&device, size, wgpu::BufferUsages::STORAGE).unwrap();
        let new = SharedUploadRing::new(&device, size, wgpu::BufferUsages::STORAGE).unwrap();
        assert_eq!(old.allocation_bytes(), new.allocation_bytes());
        let old_target = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let new_target = device.create_buffer(&wgpu::BufferDescriptor {
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
        let old_groups: Vec<_> = old
            .buffers()
            .chain(std::iter::once(&old_target))
            .map(|b| group(&device, &pipeline, b, &output))
            .collect();
        let control_groups: Vec<_> = old_control
            .buffers()
            .chain(std::iter::once(&new_target))
            .map(|b| group(&device, &pipeline, b, &output))
            .collect();
        let new_groups: Vec<_> = new
            .buffers()
            .chain(std::iter::once(&new_target))
            .map(|b| group(&device, &pipeline, b, &output))
            .collect();
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut payload: Vec<u32> = (0..size / 4)
            .map(|i| (i as u32).wrapping_mul(0x71327))
            .collect();
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        // Interleave three-frame blocks, so scheduling/thermal drift is shared
        // by both implementations. Pair totals cover 96 frames per implementation.
        for pair in 0..10 {
            let mut cpu = [0.; 2];
            let mut upload = [0.; 2];
            let mut wall = [0.; 2];
            for block in 0..32 {
                for after in if (pair + block) % 2 == 0 {
                    [false, true]
                } else {
                    [true, false]
                } {
                    let index = usize::from(after);
                    let mut held = Vec::new();
                    if overflow {
                        // Keep all three slots leased but unsubmitted.
                        for _ in 0..3 {
                            let mut encoder = device.create_command_encoder(&Default::default());
                            let (slot, _) = if after {
                                new.write_or_stage(&mut encoder, &new_target, |mut b| b.fill(0))
                                    .unwrap()
                            } else {
                                old.write_or_stage(&mut encoder, &old_target, |mut b| b.fill(0))
                                    .unwrap()
                            };
                            assert!(slot < 3);
                            held.push(encoder.finish());
                        }
                    }
                    let start = std::time::Instant::now();
                    for frame in 0..3 {
                        payload[0] = block * 3 + frame + 1;
                        let step = std::time::Instant::now();
                        let mut encoder = device.create_command_encoder(&Default::default());
                        let bytes = bytemuck::cast_slice(&payload);
                        let upload_start = std::time::Instant::now();
                        let (slot, groups) = if after && control {
                            (
                                old_control
                                    .write_or_stage(&mut encoder, &new_target, |mut b| {
                                        b.copy_from_slice(bytes)
                                    })
                                    .unwrap()
                                    .0,
                                &control_groups,
                            )
                        } else if after {
                            (
                                new.write_or_stage(&mut encoder, &new_target, |mut b| {
                                    b.copy_from_slice(bytes)
                                })
                                .unwrap()
                                .0,
                                &new_groups,
                            )
                        } else {
                            (
                                old.write_or_stage(&mut encoder, &old_target, |mut b| {
                                    b.copy_from_slice(bytes)
                                })
                                .unwrap()
                                .0,
                                &old_groups,
                            )
                        };
                        upload[index] += upload_start.elapsed().as_secs_f64() * 1e6;
                        assert_eq!(
                            slot == 3,
                            overflow,
                            "comparison must exercise the requested path"
                        );
                        consume(&pipeline, &groups[slot], &mut encoder, (size / 4) as u32);
                        queue.submit([encoder.finish()]);
                        cpu[index] += step.elapsed().as_secs_f64() * 1e6;
                        if frame % 3 == 2 {
                            device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                        }
                    }
                    wall[index] += start.elapsed().as_secs_f64() * 1e6;
                    // Verify every GPU word outside timings after every half-pair.
                    let mut encoder = device.create_command_encoder(&Default::default());
                    encoder.copy_buffer_to_buffer(&output, 0, &read, 0, size);
                    queue.submit([encoder.finish()]);
                    let actual = support::read(&device, &read);
                    assert!(
                        bytemuck::cast_slice::<u8, u32>(&actual)
                            .iter()
                            .zip(&payload)
                            .all(|(&a, &b)| a == b ^ 0x13579bd)
                    );
                    drop(held);
                    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                }
            }
            if pair != 0 {
                for after in [false, true] {
                    let index = usize::from(after);
                    println!(
                        "{name},{size},{pair},{},{:.3},{:.3},{:.3}",
                        if after { "after" } else { "before" },
                        upload[index] / 96.,
                        cpu[index] / 96.,
                        wall[index] / 96.
                    );
                }
            }
        }
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}
