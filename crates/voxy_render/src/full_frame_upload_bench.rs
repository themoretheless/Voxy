//! Full production-pass offscreen frames. GPU service timestamps and completion
//! callbacks are deliberately separate from CPU encode/submit and display pacing.
use crate::frame_upload_test_support as support;
use std::time::{Duration, Instant};
pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;
pub const COUNT: usize = 4096;
const FRAMES: u32 = 48;

// A late frame must not trigger a burst of already overdue starts. Preserve the
// original grid separately in telemetry; moving the next start cannot erase a
// missed 120 Hz deadline or external GPU contention.
fn next_due(due: Instant, started: Instant, period: Duration) -> Instant {
    let next = due + period;
    if next.saturating_duration_since(started) < period / 2 {
        started + period
    } else {
        next
    }
}

#[test]
fn paced_schedule_does_not_burst_after_preemption() {
    let epoch = Instant::now();
    let period = Duration::from_nanos(8_333_333);
    let late = epoch + period * 12;
    let next = next_due(epoch, late, period);
    assert_eq!(next, late + period);
    assert!(next > epoch + period * 12);
    assert_eq!(next_due(next, next, period), next + period);
    assert_eq!(next_due(epoch, epoch, period), epoch + period);
    // Ordinary scheduler lateness must not accumulate into a lower frame rate.
    assert_eq!(next_due(epoch, epoch + period / 8, period), epoch + period);
    let nearly_next = epoch + period * 7 / 8;
    assert_eq!(next_due(epoch, nearly_next, period), nearly_next + period);
}

pub fn device() -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let required = wgpu::Features::MAPPABLE_PRIMARY_BUFFERS | wgpu::Features::TIMESTAMP_QUERY;
    assert!(
        adapter.features().contains(required),
        "full frame GPU timing capability required"
    );
    eprintln!("FULL_ADAPTER {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: required,
        ..Default::default()
    }))
    .unwrap();
    (adapter, device, queue)
}
pub struct Target {
    pub colour: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub depth: wgpu::TextureView,
}
impl Target {
    pub fn new(device: &wgpu::Device) -> Self {
        let texture = |format, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("full frame 1080p fixture"),
                size: wgpu::Extent3d {
                    width: WIDTH,
                    height: HEIGHT,
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
        let colour = texture(
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let depth = texture(
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        Self {
            view: colour.create_view(&Default::default()),
            colour,
            depth: depth.create_view(&Default::default()),
        }
    }
    pub fn read(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(WIDTH * HEIGHT * 4),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            self.colour.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(WIDTH * 4),
                    rows_per_image: Some(HEIGHT),
                },
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        support::read(device, &read)
    }
}
pub fn camera() -> crate::SceneCamera {
    crate::SceneCamera {
        eye: glam::Vec3::new(0., 0., 3.),
        target: glam::Vec3::ZERO,
        up: glam::Vec3::Y,
        projection: crate::SceneProjection::Perspective {
            vertical_fov: 1.,
            aspect: WIDTH as f32 / HEIGHT as f32,
            near: 0.1,
            far: 10.,
        },
    }
}
pub struct Background {
    mesh: crate::SceneGeometry,
    texture: crate::SceneTexture,
    transform: crate::SceneTransform,
}
impl Background {
    pub fn new(scene: &crate::SceneRenderer, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for y in 0..12 {
            for x in 0..20 {
                let x0 = (x as f32 - 10.) * 0.28;
                let y0 = (y as f32 - 6.) * 0.28;
                let color = if (x + y) % 2 == 0 {
                    [0.13, 0.24, 0.35, 1.]
                } else {
                    [0.65, 0.54, 0.36, 1.]
                };
                let base = vertices.len() as u32;
                for position in [
                    [x0, y0, -0.4],
                    [x0 + 0.28, y0, -0.4],
                    [x0 + 0.28, y0 + 0.28, -0.4],
                    [x0, y0 + 0.28, -0.4],
                ] {
                    vertices.push(crate::SceneVertex {
                        position,
                        uv: [0.; 2],
                        color,
                    });
                }
                indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
        let mesh = crate::SceneMesh::new(vertices, indices).unwrap();
        Self {
            mesh: scene.upload_mesh(device, &mesh).unwrap(),
            texture: scene
                .upload_texture(device, queue, 1, 1, &[255; 4])
                .unwrap(),
            transform: scene
                .create_transform(device, camera().view_projection().unwrap())
                .unwrap(),
        }
    }
    pub fn draw(&self) -> crate::SceneDraw<'_> {
        crate::SceneDraw {
            geometry: &self.mesh,
            texture: &self.texture,
            transform: &self.transform,
            overlay: false,
        }
    }
}

pub fn pairs(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    target: &Target,
    path: &str,
    mut frame: impl FnMut(bool, u32, &mut wgpu::CommandEncoder),
) {
    let phase = std::env::var("VOXY_FULL_PHASE").unwrap_or_else(|_| "aa".into());
    assert!(phase == "aa" || phase == "ab");
    let paced = std::env::var("VOXY_FULL_PROTOCOL").unwrap_or_else(|_| "paced".into()) == "paced";
    let protocol = if paced { "paced120" } else { "throughput" };
    let period = Duration::from_secs_f64(1. / 120.);
    let pacer = std::env::var("VOXY_FULL_PACER").unwrap_or_else(|_| "bounded".into());
    assert!(matches!(pacer.as_str(), "legacy" | "bounded" | "compare"));
    println!("FULL_PACER,{pacer},global_deadlines_retained");
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    // Byte-exact full-resolution oracle before timings. No readback in frames.
    for seed in [0, 19] {
        let mut images = Vec::new();
        for mode in [false, true] {
            let mut encoder = device.create_command_encoder(&Default::default());
            frame(mode, seed, &mut encoder);
            queue.submit([encoder.finish()]);
            images.push(target.read(device, queue));
        }
        assert_eq!(images[0], images[1], "1080p full-pass parity seed={seed}");
        assert!(
            images[0].chunks_exact(4).any(|p| p != &images[0][..4]),
            "blank frame oracle"
        );
    }
    println!("FULL_PARITY,{path},{phase},1920,1080,2,exact");
    let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
        label: None,
        ty: wgpu::QueryType::Timestamp,
        count: FRAMES * 2,
    });
    let resolve = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (u64::from(FRAMES) * 16).div_ceil(256) * 256,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(FRAMES) * 16,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // Apple M4 exposes stage-boundary counters, not encoder-level timestamps.
    // Matched one-workgroup marker passes read the actual colour target, forcing
    // read/write dependencies across ALL intervening render passes. Independent
    // empty markers could otherwise overlap graphics on a different GPU stage. Their
    // small transition/dispatch cost is included equally in A/A and A/B.
    let marker_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let marker_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("full frame timestamp marker"),
        source: wgpu::ShaderSource::Wgsl(
            r"@group(0) @binding(0) var colour:texture_2d<f32>;
@group(0) @binding(1) var<storage,read_write> checksum:vec4f;
@compute @workgroup_size(1) fn main() {checksum=textureLoad(colour,vec2i(0),0);}"
                .into(),
        ),
    });
    let marker_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &marker_shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let checksum = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("GPU marker dependency"),
        size: 16,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    let marker_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &marker_pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&target.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: checksum.as_entire_binding(),
            },
        ],
    });
    assert!(
        pollster::block_on(marker_scope.pop()).is_none(),
        "GPU marker setup validation"
    );
    let marker = |encoder: &mut wgpu::CommandEncoder, index: u32, end: bool| {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("full frame GPU counter boundary"),
            timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                query_set: &queries,
                beginning_of_pass_write_index: (!end).then_some(index),
                end_of_pass_write_index: end.then_some(index),
            }),
        });
        pass.set_pipeline(&marker_pipeline);
        pass.set_bind_group(0, &marker_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    };
    let epoch = Instant::now();
    let timestamp_us = f64::from(queue.get_timestamp_period()) / 1000.;
    let blocks = std::env::var("VOXY_FULL_BLOCKS")
        .ok()
        .map(|v| v.parse::<u32>().unwrap())
        .unwrap_or(4);
    let pair_count = std::env::var("VOXY_FULL_PAIRS")
        .ok()
        .map(|v| v.parse::<u32>().unwrap())
        .unwrap_or(9);
    println!(
        "FULL_HEADER,path,phase,protocol,pair,block,mode,frame,cpu_us,gpu_us,completion_us,start_us,submit_us,callback_us,gpu_start_tick,gpu_end_tick,start_gap_us,gpu_gap_us,backpressure_us,deadline_miss"
    );
    // Full pair zero is warm-up; every measured pair reverses short AB/BA blocks.
    for pair in 0..=pair_count {
        for block in 0..blocks {
            for mode in if (pair + block) % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                let block_start = Instant::now();
                let legacy_pacer = pacer == "legacy" || (pacer == "compare" && !mode);
                let queue_depth = if paced && !legacy_pacer { 1usize } else { 3 };
                let mut due = block_start;
                let (tx, rx) = std::sync::mpsc::channel();
                let mut cpu = Vec::new();
                let mut submissions: Vec<wgpu::SubmissionIndex> = Vec::new();
                for index in 0..FRAMES {
                    if paced {
                        if legacy_pacer {
                            due = block_start + period * index;
                        }
                        while Instant::now() < due {
                            device.poll(wgpu::PollType::Poll).unwrap();
                            let remaining = due.saturating_duration_since(Instant::now());
                            if remaining > Duration::from_micros(250) {
                                std::thread::sleep(remaining.min(Duration::from_micros(500)));
                            } else {
                                std::hint::spin_loop();
                            }
                        }
                    }
                    let waiting = Instant::now();
                    if index as usize >= queue_depth {
                        device
                            .poll(wgpu::PollType::Wait {
                                submission_index: Some(
                                    submissions[index as usize - queue_depth].clone(),
                                ),
                                timeout: None,
                            })
                            .unwrap();
                    }
                    let backpressure_us = waiting.elapsed().as_secs_f64() * 1e6;
                    let started = Instant::now();
                    let assigned_due = due;
                    if paced && !legacy_pacer {
                        due = next_due(due, started, period);
                    }
                    let mut encoder = device.create_command_encoder(&Default::default());
                    marker(&mut encoder, index * 2, false);
                    frame(mode, block * FRAMES + index, &mut encoder);
                    marker(&mut encoder, index * 2 + 1, true);
                    submissions.push(queue.submit([encoder.finish()]));
                    let submitted = Instant::now();
                    let done = tx.clone();
                    queue.on_submitted_work_done(move || {
                        let _ = done.send((index, Instant::now()));
                    });
                    cpu.push((
                        started,
                        submitted,
                        started.elapsed().as_secs_f64() * 1e6,
                        backpressure_us,
                        assigned_due,
                    ));
                }
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                let mut completions = vec![epoch; FRAMES as usize];
                for _ in 0..FRAMES {
                    let (i, done) = rx.recv().unwrap();
                    completions[i as usize] = done;
                }
                // Query resolve and readback are separate submissions after completion,
                // outside both full-frame CPU and per-frame GPU timestamp intervals.
                let mut encoder = device.create_command_encoder(&Default::default());
                encoder.resolve_query_set(&queries, 0..FRAMES * 2, &resolve, 0);
                encoder.copy_buffer_to_buffer(&resolve, 0, &read, 0, u64::from(FRAMES) * 16);
                queue.submit([encoder.finish()]);
                let bytes = support::read(device, &read);
                let ticks: Vec<_> = bytes
                    .chunks_exact(8)
                    .map(|v| u64::from_le_bytes(v.try_into().unwrap()))
                    .collect();
                if pair == 0 {
                    continue;
                }
                for i in 0..FRAMES as usize {
                    assert!(
                        ticks[2 * i + 1] > ticks[2 * i],
                        "missing hardware timestamp"
                    );
                    let (start, submit, cpu_us, backpressure_us, assigned_due) = cpu[i];
                    let gpu_us = (ticks[2 * i + 1] - ticks[2 * i]) as f64 * timestamp_us;
                    let completion_us = completions[i].duration_since(start).as_secs_f64() * 1e6;
                    let start_gap = if i == 0 {
                        0.
                    } else {
                        start.duration_since(cpu[i - 1].0).as_secs_f64() * 1e6
                    };
                    let gpu_gap = if i == 0 {
                        0.
                    } else {
                        (ticks[2 * i + 1] - ticks[2 * i - 1]) as f64 * timestamp_us
                    };
                    if paced {
                        println!(
                            "PACE,{path},{phase},{pair},{block},{},{i},{},{:.3},{:.3},{}",
                            if mode { "after" } else { "before" },
                            queue_depth,
                            assigned_due.duration_since(epoch).as_secs_f64() * 1e6,
                            start.saturating_duration_since(assigned_due).as_secs_f64() * 1e6,
                            u8::from(completions[i] > assigned_due + period),
                        );
                    }
                    println!(
                        "FULL,{path},{phase},{protocol},{pair},{block},{},{i},{cpu_us:.3},{gpu_us:.3},{completion_us:.3},{:.3},{:.3},{:.3},{},{},{start_gap:.3},{gpu_gap:.3},{backpressure_us:.3},{}",
                        if mode { "after" } else { "before" },
                        start.duration_since(epoch).as_secs_f64() * 1e6,
                        submit.duration_since(epoch).as_secs_f64() * 1e6,
                        completions[i].duration_since(epoch).as_secs_f64() * 1e6,
                        ticks[2 * i],
                        ticks[2 * i + 1],
                        u8::from(paced && completions[i] > block_start + period * (i as u32 + 1))
                    );
                }
            }
        }
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}
