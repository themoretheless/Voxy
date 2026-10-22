//! Differential GPU qualification of archived triangle arithmetic candidates.
use wgpu::util::DeviceExt;
fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("output directory"));
    let candidate =
        std::fs::read_to_string(std::env::args().nth(2).expect("candidate WGSL")).unwrap();
    std::fs::create_dir_all(&root).unwrap();
    let interval_products = std::env::args().any(|arg| arg == "--interval-products");
    let mut cases = Vec::<[[f32; 4]; 9]>::new();
    let mut seed = 0x747269616e676c65u64;
    let mut random = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as u32
    };
    for i in 0..32768 {
        let mut c = [[0.; 4]; 9];
        for v in &mut c[..8] {
            for x in &mut v[..3] {
                let bits = random();
                *x = if i % 4 == 0 {
                    f32::from_bits(bits)
                } else {
                    (bits as i32 as f32 / 2147483648.) * 2f32.powi((i % 80) - 40)
                };
            }
        }
        if i % 3 == 0 {
            for v in &mut c[4..8] {
                *v = [0.; 4];
            }
        }
        c[8] = [0., 0., 1., 0.];
        cases.push(c);
    }
    // Closed edges, near/far endpoints, adjacent floats, singular and FTZ inputs.
    let edge = [
        0.,
        -0.,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        0.25,
        0.5,
        1.,
        f32::from_bits(1f32.to_bits() - 1),
        f32::from_bits(1f32.to_bits() + 1),
        -1.,
        f32::MAX,
        f32::INFINITY,
        f32::NAN,
    ];
    for x in edge {
        for y in edge {
            for z in edge {
                let mut c = [[0.; 4]; 9];
                c[0] = [0., 0., 1., 0.];
                c[1] = [1., 0., 0., 0.];
                c[2] = [0., 1., 0., 0.];
                c[3] = [x, y, z, 0.];
                c[8] = [0., 0., 1., 0.];
                cases.push(c);
            }
        }
    }
    if interval_products {
        cases.clear();
        for i in 0..100000 {
            let mut c = [[0.; 4]; 9];
            let a = random();
            let mut ah = random();
            let b = random();
            let mut bh = random();
            if i % 4 < 2 {
                ah = a;
            }
            if i % 4 == 0 || i % 4 == 2 {
                bh = b;
            }
            c[0] = [
                f32::from_bits(a),
                f32::from_bits(ah),
                f32::from_bits(u32::from(i % 11 != 0)),
                0.,
            ];
            c[1] = [
                f32::from_bits(b),
                f32::from_bits(bh),
                f32::from_bits(u32::from(i % 17 != 0)),
                0.,
            ];
            cases.push(c);
        }
        let endpoints = [
            0u32, 0x80000000, 1, 0x007fffff, 0x00800000, 0x00800001, 0x3effffff, 0x3f000000,
            0x3f7fffff, 0x3f800000, 0x3f800001, 0xbf800000, 0x7f7fffff, 0xff7fffff, 0x7f800000,
            0xff800000, 0x7fc00001, 0xffc00001,
        ];
        for a in endpoints {
            for ah in endpoints {
                for b in endpoints {
                    for bh in endpoints {
                        let mut c = [[0.; 4]; 9];
                        c[0] = [f32::from_bits(a), f32::from_bits(ah), f32::from_bits(1), 0.];
                        c[1] = [f32::from_bits(b), f32::from_bits(bh), f32::from_bits(1), 0.];
                        cases.push(c);
                    }
                }
            }
        }
    }
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    println!("TRIANGLE CERTIFICATE GPU {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&cases),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = cases.len() as u64 * 32;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: size * 2,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let body = if interval_products {
        r"
struct Input {v:array<vec4<f32>,9>}
@group(0) @binding(0) var<storage,read> inputs:array<Input>;
@group(0) @binding(1) var<storage,read_write> outputs:array<vec4<u32>>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&inputs) {return;}let c=inputs[id.x];
 let a=WorldInterval(c.v[0].x,c.v[0].y,bitcast<u32>(c.v[0].z));let b=WorldInterval(c.v[1].x,c.v[1].y,bitcast<u32>(c.v[1].z));
 let r=wi_mul(a,b);outputs[id.x*2u]=vec4(r.valid,bitcast<u32>(r.lo),bitcast<u32>(r.hi),id.x);outputs[id.x*2u+1u]=vec4(0u);
}
"
    } else {
        r"
struct Input {v:array<vec4<f32>,9>}
@group(0) @binding(0) var<storage,read> inputs:array<Input>;
@group(0) @binding(1) var<storage,read_write> outputs:array<vec4<u32>>;
fn unpack(a:vec4<f32>,b:vec4<f32>)->WorldPair3 {return WorldPair3(vec2(a.x,b.x),vec2(a.y,b.y),vec2(a.z,b.z));}
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&inputs) {return;}let c=inputs[id.x];
 let d=unpack(c.v[0],c.v[4]);let e1=unpack(c.v[1],c.v[5]);let e2=unpack(c.v[2],c.v[6]);let delta=unpack(c.v[3],c.v[7]);
 let result=world_triangle_pair(delta,d,wp_scalar(vec3(0.)),e1,e2,wi_vec(e1),wi_vec(e2),wi_vec(delta),c.v[8].xy,c.v[8].zw);
 outputs[id.x*2u]=vec4(result.state,result.reason,bitcast<u32>(result.t.x),bitcast<u32>(result.t.y));
 outputs[id.x*2u+1u]=bitcast<vec4<u32>>(vec4(result.u,result.v));
}
"
    };
    let mut encoder = device.create_command_encoder(&Default::default());
    for (index, triangle) in [
        include_str!("../src/world_triangle_pair.wgsl"),
        candidate.as_str(),
    ]
    .into_iter()
    .enumerate()
    {
        let source = [
            include_str!("../src/archive_projection_pair.wgsl"),
            triangle,
            body,
        ]
        .join("\n");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("triangle certificate differential"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        });
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups((cases.len() as u32).div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, index as u64 * size, size);
    }
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let data = readback.slice(..).get_mapped_range().unwrap();
    let (a, b) = data.split_at(size as usize);
    let mut states = [0usize; 3];
    let mut mismatches = Vec::new();
    for (i, (x, y)) in a.chunks_exact(32).zip(b.chunks_exact(32)).enumerate() {
        states[u32::from_ne_bytes(x[..4].try_into().unwrap()) as usize] += 1;
        if x != y {
            mismatches.push(i);
        }
    }
    std::fs::write(root.join("inputs.bin"), bytemuck::cast_slice(&cases)).unwrap();
    std::fs::write(root.join("paired-results.bin"), &data).unwrap();
    let summary = serde_json::json!({"mode":if interval_products {"interval_product"}else{"triangle"},"cases":cases.len(),"states":states,"mismatches":mismatches,"complete_result_bytes_per_case":32,"scope":"GPU differential against unchanged reference; not independent exact oracle or full scene performance","fps_qualified":false});
    std::fs::write(
        root.join("summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
    println!("{summary}");
    assert!(mismatches.is_empty());
}
