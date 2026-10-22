//! Candidate qualification against exhaustive f64 closed-pixel geometry.
use wgpu::util::DeviceExt;
mod depth_image_reference;
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Ray {
    begin: [f32; 4],
    end: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Hit {
    pixel: [i32; 2],
    parameter: f32,
    visited: u32,
}
fn reference(depth: &[f32], width: usize, height: usize, ray: Ray) -> Option<([i32; 2], f64)> {
    depth_image_reference::intersect(
        depth,
        width,
        height,
        [
            ray.begin[0] as f64,
            ray.begin[1] as f64,
            ray.begin[2] as f64,
        ],
        [ray.end[0] as f64, ray.end[1] as f64, ray.end[2] as f64],
    )
}
fn rays(width: u32, height: u32) -> Vec<Ray> {
    let mut out = Vec::new();
    for (a, b) in [
        ([0., 0., 0.], [16., 12., 1.]),
        ([16., 12., 1.], [0., 0., 0.]),
        ([0., 12., 0.], [12., 0., 1.]),
        ([12., 0., 0.], [0., 12., 1.]),
        ([8., 6., 0.5], [8., 6., 0.5]),
        ([8., 6., 0.], [8., 6., 1.]),
        ([0., 6., 0.5], [16., 6., 0.5]),
        ([8., 0., 0.5], [8., 12., 0.5]),
    ] {
        out.push(Ray {
            begin: [a[0], a[1], a[2], 0.],
            end: [b[0], b[1], b[2], 0.],
        });
    }
    let mut state = 0x564f5859u64;
    let mut random = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 32) as u32
    };
    for _ in 0..8192 {
        let a = [
            (random() % ((width + 4) * 8)) as f32 / 8. - 2.,
            (random() % ((height + 4) * 8)) as f32 / 8. - 2.,
            (random() % 21) as f32 / 16. - 0.125,
            0.,
        ];
        let b = [
            (random() % ((width + 4) * 8)) as f32 / 8. - 2.,
            (random() % ((height + 4) * 8)) as f32 / 8. - 2.,
            (random() % 21) as f32 / 16. - 0.125,
            0.,
        ];
        out.push(Ray { begin: a, end: b });
    }
    for (begin, end) in [
        ([f32::NAN, 0., 0., 0.], [1., 1., 1., 0.]),
        ([0., f32::INFINITY, 0., 0.], [1., 1., 1., 0.]),
        ([f32::MAX, 0., 0., 0.], [-f32::MAX, 1., 1., 0.]),
    ] {
        out.push(Ray { begin, end });
    }
    out
}
fn main() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    let source = concat!(
        include_str!("../../crates/voxy_render/src/archive_depth_dda.wgsl"),
        r#"
struct Ray {begin:vec4<f32>,end:vec4<f32>}
@group(0) @binding(0) var image:texture_2d<f32>;
@group(0) @binding(1) var<storage,read> rays:array<Ray>;
@group(0) @binding(2) var<storage,read_write> hits:array<DepthDdaHit>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){if id.x>=arrayLength(&rays){return;}hits[id.x]=trace_depth_dda(image,rays[id.x].begin.xyz,rays[id.x].end.xyz);}
@compute @workgroup_size(64) fn exact_predicate(@builtin(global_invocation_id) id:vec3<u32>){if id.x>=arrayLength(&rays){return;}let v=rays[id.x].begin;hits[id.x]=DepthDdaHit(vec2<i32>(select(0,1,dda_exact_edge_zero(v.x,v.y,v.z,v.w)),0),0.,0u);}

"#
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("full-pixel candidate DDA"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let (w, h) = (17u32, 13u32);
    let rays = rays(w, h);
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&rays),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = (rays.len() * 16) as u64;
    let result = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: result.as_entire_binding(),
            },
        ],
    });
    let mut failures = 0;
    let mut hits = 0;
    let mut max_error = 0f64;
    let mut unsupported = 0;
    for pattern in 0..4 {
        let depths: Vec<f32> = (0..w * h)
            .map(|i| match pattern {
                0 => 0.5,
                1 => {
                    if i % 17 == 8 {
                        0.25
                    } else {
                        1.
                    }
                }
                2 => (1 + (i * 37) % 15) as f32 / 16.,
                _ => {
                    if i % 5 == 0 {
                        1.
                    } else {
                        (1 + i % 15) as f32 / 16.
                    }
                }
            })
            .collect();
        queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(&depths),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            texture.size(),
        );
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups((rays.len() as u32).div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&result, 0, &read, 0, size);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let data = read.slice(..).get_mapped_range().unwrap();
        for (i, bytes) in data.chunks_exact(16).enumerate() {
            let actual: Hit = bytemuck::pod_read_unaligned(bytes);
            let ray = rays[i];
            if (0..3).any(|axis| {
                !ray.begin[axis].is_finite()
                    || !ray.end[axis].is_finite()
                    || !(ray.end[axis] - ray.begin[axis]).is_finite()
            }) {
                assert_eq!(
                    actual.visited,
                    u32::MAX,
                    "unsupported numerical input must request fallback rather than report a miss"
                );
                assert_eq!(actual.pixel, [-1, -1]);
                unsupported += 1;
                continue;
            }
            let expected = reference(&depths, w as usize, h as usize, rays[i]);
            let valid = match expected {
                None => actual.pixel == [-1, -1],
                Some((pixel, t)) => {
                    hits += 1;
                    let error = (f64::from(actual.parameter) - t).abs();
                    max_error = max_error.max(error);
                    actual.pixel == pixel && error <= 2e-6
                }
            };
            if !valid {
                if failures < 12 {
                    eprintln!(
                        "DDA_MISMATCH pattern={pattern} ray={i} input={:?} expected={expected:?} actual={actual:?}",
                        rays[i]
                    );
                }
                failures += 1;
            }
        }
        drop(data);
        read.unmap();
    }
    println!(
        "GPU_DEPTH_DDA rays={} patterns=4 oracle_hits={hits} mismatches={failures} unsupported_fallbacks={unsupported} max_parameter_error={max_error} production_enabled=false",
        rays.len() * 4
    );
    assert_eq!(
        failures, 0,
        "candidate is not admitted until every depth-image geometry case agrees"
    );
    qualify_predicate(&device, &queue, &shader, &layout, &pipeline_layout, &view);
}

fn qualify_predicate(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::BindGroupLayout,
    pipeline_layout: &wgpu::PipelineLayout,
    view: &wgpu::TextureView,
) {
    let mut state = 0x504f525441424c45u64;
    let mut random = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 32) as u32
    };
    let mut cases = Vec::new();
    for i in 0..20000 {
        let a = f32::from_bits(random() % 0x7f800000);
        let b = f32::from_bits(random() % 0x7f800000);
        let v = match i % 4 {
            0 => [a, b, -a, b],
            1 => [a, b, -b, a],
            2 => [
                a,
                b,
                -a,
                f32::from_bits(b.to_bits().saturating_add(1).min(0x7f7fffff)),
            ],
            _ => [
                a,
                b,
                f32::from_bits(random() % 0x7f800000),
                -f32::from_bits(random() % 0x7f800000),
            ],
        };
        cases.push(Ray {
            begin: v,
            end: [0.; 4],
        });
    }
    for v in [
        [f32::MAX, f32::MAX, -f32::MAX, f32::MAX],
        [
            f32::from_bits(1),
            f32::from_bits(1),
            -f32::from_bits(1),
            f32::from_bits(1),
        ],
        [0., -0., 0., 1.],
        [f32::NAN, 1., 0., 0.],
        [f32::INFINITY, 0., 0., 0.],
    ] {
        cases.push(Ray {
            begin: v,
            end: [0.; 4],
        });
    }
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&cases),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = (cases.len() * 16) as u64;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(pipeline_layout),
        module: shader,
        entry_point: Some("exact_predicate"),
        compilation_options: Default::default(),
        cache: None,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups((cases.len() as u32).div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &read, 0, size);
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    read.slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let data = read.slice(..).get_mapped_range().unwrap();
    for (i, bytes) in data.chunks_exact(16).enumerate() {
        let hit: Hit = bytemuck::pod_read_unaligned(bytes);
        let v = cases[i].begin;
        let expected = v.iter().all(|v| v.is_finite())
            && (f64::from(v[0]) * f64::from(v[1]) + f64::from(v[2]) * f64::from(v[3]) == 0.);
        assert_eq!(
            hit.pixel[0],
            i32::from(expected),
            "exact product cancellation failed at {i}: {v:?}"
        );
    }
    println!(
        "GPU_DDA_EDGE_PREDICATE cases={} exact_f64_product_reference=true subnormals=true overflowing_float32_products=true fused_fma_dependency=false",
        cases.len()
    );
}
