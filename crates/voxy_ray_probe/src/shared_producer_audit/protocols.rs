//! Standalone GPU world-ray protocol audits (reciprocal, modular words, scale, queue, guard, bounded visibility).
use super::*;
use wgpu::util::DeviceExt;

pub(super) fn reciprocal_node_audit(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("VOXY_RAY_SHARED_RECIPROCAL_AUDIT").as_deref() != Ok("1") {
        return Ok(());
    }
    let mut cases = Vec::<[u32; 32]>::new();
    let mut nodes = Vec::<u32>::new();
    let base = {
        let mut v = [0u32; 32];
        v[8] = 1f32.to_bits();
        v[16] = 1f32.to_bits();
        v
    };
    let mut push = |r: [u32; 32], b: [f32; 6]| {
        cases.push(r);
        nodes.extend(b.map(f32::to_bits));
        nodes.extend([0, 0]);
    };
    push(base, [0.; 6]);
    push(base, [-1., -1., -1., 1., 1., 1.]);
    push(base, [3., -1., -1., 4., 1., 1.]);
    push(base, [1., -1., -1., 2., 1., 1.]);
    let mut r = base;
    r[8] = 0;
    push(r, [-1., -1., -1., 1., 1., 1.]);
    r[0] = 2f32.to_bits();
    push(r, [-1., -1., -1., 1., 1., 1.]);
    r = base;
    r[8] = (-1f32).to_bits();
    push(r, [-3., -1., -1., -2., 1., 1.]);
    r = base;
    r[8] = 0;
    r[12] = 1f32.to_bits();
    push(r, [0.5, -1., -1., 1., 1., 1.]);
    r = base;
    r[12] = (-1f32).to_bits();
    push(r, [-1., -1., -1., 1., 1., 1.]);
    r = base;
    r[0] = 16777216f32.to_bits();
    r[4] = (-16777216f32).to_bits();
    push(r, [0.5, -1., -1., 1., 1., 1.]);
    r = base;
    r[8] = 1;
    push(r, [1., -1., -1., 2., 1., 1.]);
    r = base;
    r[8] = 252u32 << 23;
    r[16] = 2u32 << 23;
    push(r, [4., -1., -1., 8., 1., 1.]);
    for (a, b) in [
        (0x7effffffu32, 1f32.to_bits()),
        (0x7effffff, 0x71800000),
        (0x01000000, 1f32.to_bits()),
        (0x01000000, 0x81000000),
        (0x01800000, 0x01800000),
    ] {
        let mut r = base;
        r[0] = a;
        r[8] = b;
        push(r, [-1., -1., -1., 1., 1., 1.]);
    }
    let mut seed = 0x564f5859u32;
    let mut random = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed
    };
    for i in 0..3000 {
        let mut r = base;
        let mut b = [0f32; 6];
        for axis in 0..3 {
            if i < 2000 {
                let o = (random() % 2001) as f32 / 8. - 125.;
                let d = (random() % 2001) as f32 / 16. - 62.5;
                r[axis] = o.to_bits();
                r[4 + axis] = ((random() % 15) as f32 / 1048576. - 7. / 1048576.).to_bits();
                r[8 + axis] = d.to_bits();
                r[12 + axis] = ((random() % 15) as f32 / 1048576. - 7. / 1048576.).to_bits();
                let center = if i % 3 == 0 {
                    o + d * 0.5
                } else {
                    (random() % 4001) as f32 / 8. - 250.
                };
                let radius = 0.25 + (random() % 256) as f32 / 4.;
                b[axis] = center - radius;
                b[axis + 3] = center + radius;
            } else {
                let mut finite = || {
                    let v = random();
                    (v & 0x807fffff) | ((1 + (v >> 24) % 254) << 23)
                };
                r[axis] = finite();
                r[4 + axis] = finite();
                r[8 + axis] = finite();
                r[12 + axis] = finite();
                let a = f32::from_bits(finite());
                let c = f32::from_bits(finite());
                b[axis] = a.min(c);
                b[axis + 3] = a.max(c);
            }
        }
        push(r, b);
    }
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("reciprocal source ray controls"),
        contents: bytemuck::cast_slice(&cases),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let bounds = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("reciprocal source box controls"),
        contents: bytemuck::cast_slice(&nodes),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: cases.len() as u64 * 224,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let source=[include_str!("../../../voxy_render/src/archive_projection_pair.wgsl"),include_str!("../../../voxy_render/src/world_triangle_pair.wgsl"),include_str!("../../../voxy_render/src/world_ray_bvh_interval.wgsl"),include_str!("../../../voxy_render/src/world_node_reciprocal.wgsl"),r"
@group(0) @binding(0) var<storage,read> source:array<u32>;
@group(0) @binding(4) var<storage,read> exact_layout:array<vec4<u32>>;
@group(0) @binding(5) var<storage,read_write> exact_output:array<vec4<u32>>;
fn exact_ray_word(index:u32)->u32 {return 0u;}
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let index=id.x;let at=index*32u;if at+31u>=arrayLength(&source) {return;}
 for(var axis=0u;axis<3u;axis++) {nearest_node_origin[axis]=wi_add(wi_source_word(source[at+axis]),wi_source_word(source[at+4u+axis]));nearest_node_direction[axis]=wi_add(wi_source_word(source[at+8u+axis]),wi_source_word(source[at+12u+axis]));}
 let old=nearest_node_miss_before(index*8u,index,bitcast<f32>(source[at+16u]));nearest_prepare_reciprocal();let cached=nearest_node_miss_cached(index*8u,index,bitcast<f32>(source[at+16u]));let out=index*14u;
 exact_output[out]=vec4<u32>(select(0u,1u,old),select(0u,1u,cached),select(0u,1u,nearest_inverse_ready),0u);
 for(var axis=0u;axis<3u;axis++) {let inv=nearest_inverse[axis];let o=nearest_node_origin[axis];let d=nearest_node_direction[axis];exact_output[out+1u+axis]=vec4<u32>(bitcast<u32>(inv.lo),bitcast<u32>(inv.hi),inv.valid,0u);exact_output[out+4u+axis]=vec4<u32>(bitcast<u32>(o.lo),bitcast<u32>(o.hi),o.valid,0u);exact_output[out+7u+axis]=vec4<u32>(bitcast<u32>(d.lo),bitcast<u32>(d.hi),d.valid,0u);}
 let checked=wi_normal_bits(source[at])&&wi_normal_bits(source[at+8u]);
 for(var op=0u;op<4u;op++) {exact_output[out+10u+op]=vec4<u32>(0u);}
 if checked {
  let a=bitcast<f32>(source[at]);let b=bitcast<f32>(source[at+8u]);
  let al=nearest_admitted_add(a,b,false);let ah=nearest_admitted_add(a,b,true);let ml=nearest_admitted_mul(a,b,false);let mh=nearest_admitted_mul(a,b,true);
  exact_output[out+10u]=vec4<u32>(bitcast<u32>(al.value),al.valid,1u,0u);exact_output[out+11u]=vec4<u32>(bitcast<u32>(ah.value),ah.valid,1u,0u);exact_output[out+12u]=vec4<u32>(bitcast<u32>(ml.value),ml.valid,1u,0u);exact_output[out+13u]=vec4<u32>(bitcast<u32>(mh.value),mh.valid,1u,0u);
 }
}
"].join("\n");
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("original reciprocal and cached slab proof controls"),
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
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: output.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 10,
                resource: bounds.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups((cases.len() as u32).div_ceil(64), 1, 1);
    }
    queue.submit([encoder.finish()]);
    std::fs::write(
        root.join("reciprocal-cases.bin"),
        bytemuck::cast_slice(&cases),
    )?;
    std::fs::write(
        root.join("reciprocal-nodes.bin"),
        bytemuck::cast_slice(&nodes),
    )?;
    std::fs::write(
        root.join("reciprocal-output.bin"),
        read(device, queue, &output, cases.len() as u64 * 224)?,
    )?;
    Ok(())
}
pub(super) fn modular_word_audit(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut words = vec![
        0, 0x80000000, 1, 0x80000001, 0x7fffff, 0x800000, 0x800001, 0x3f800000, 0x3eaaaaab,
        0x7f7fffff, 0xff7fffff, 0x7f800000, 0xff800000, 0x7fc00001,
    ];
    let mut seed = 0x564f5859u32;
    for _ in 0..10000 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        words.push(seed);
    }
    let source = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("finite-field word controls"),
        contents: bytemuck::cast_slice(&words),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: words.len() as u64 * 16,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:Some("shared exact modular word audit"),source:wgpu::ShaderSource::Wgsl(format!("{}\n{}",include_str!("../../../voxy_render/src/world_modular_words.wgsl"),r"
@group(0) @binding(0) var<storage,read> source:array<u32>;
@group(0) @binding(1) var<storage,read_write> output:array<vec4<u32>>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {if id.x>=arrayLength(&source){return;}let raw=source[id.x];var result=vec4<u32>(raw,0u,0u,0u);if ((raw>>23u)&255u)!=255u {result.y=history_mod_word(raw,3u);result.z=history_mod_word(raw,5u);result.w=1u;}output[id.x]=result;}").into())});
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: source.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups((words.len() as u32).div_ceil(64), 1, 1);
    }
    queue.submit([encoder.finish()]);
    std::fs::write(root.join("modular-input.bin"), bytemuck::cast_slice(&words))?;
    std::fs::write(
        root.join("modular-output.bin"),
        read(device, queue, &output, words.len() as u64 * 16)?,
    )?;
    Ok(())
}
pub(super) fn scale(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut input = vec![
        0, 0x80000000, 1, 0x80000001, 0x007fffff, 0x00800000, 0x00800001, 0x80800001, 0x3f800000,
        0x3eaaaaab, 0x7f7fffff, 0xff7fffff, 0x7f800000, 0xff800000, 0x7fc00001,
    ];
    let mut seed = 0x564f5859u32;
    while input.len() < 4096 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        input.push(seed);
    }
    let source = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&input),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: input.len() as u64 * 16,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor{label:Some("integer coefficient transport audit"),source:wgpu::ShaderSource::Wgsl(format!("{}\n{}\n{}",archive_shader_source(),voxy_render::WORLD_RAY_PRODUCER_WGSL,r"
@group(0) @binding(10) var<storage,read> scale_source:array<u32>;
@compute @workgroup_size(64) fn scale_audit(@builtin(global_invocation_id) id:vec3<u32>){if id.x>=arrayLength(&scale_source){return;}let raw=scale_source[id.x];let v=world_scale_60(raw);let base=id.x*4u;world_exact_ray_words[base]=v.hi;world_exact_ray_words[base+1u]=v.lo;world_exact_ray_words[base+2u]=v.valid;world_exact_ray_words[base+3u]=raw;}
").into())});
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("scale_audit"),
        compilation_options: Default::default(),
        cache: None,
    });
    let a = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 10,
            resource: source.as_entire_binding(),
        }],
    });
    let b = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(1),
        entries: &[wgpu::BindGroupEntry {
            binding: 17,
            resource: output.as_entire_binding(),
        }],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &a, &[]);
        pass.set_bind_group(1, &b, &[]);
        pass.dispatch_workgroups((input.len() as u32).div_ceil(64), 1, 1);
    }
    queue.submit([encoder.finish()]);
    let bytes = read(device, queue, &output, input.len() as u64 * 16)?;
    std::fs::write(root.join("scale-input.bin"), bytemuck::cast_slice(&input))?;
    std::fs::write(root.join("scale-output.bin"), bytes)?;
    Ok(())
}
// Deliberately misleading hardware rays must never replace the paired source ray.
pub(super) fn separate_hint_safety(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("VOXY_RAY_SHARED_SEPARATE_NATIVE_SEED").as_deref() != Ok("1")
        && std::env::var("VOXY_RAY_SHARED_ANY_HIT_ADMISSION").as_deref() != Ok("1")
    {
        return Ok(());
    }
    let mesh = ArchiveMeshSource {
        vertices: [[-2., -2., 0.], [2., -2., 0.], [0., 2., 0.]]
            .into_iter()
            .map(|position| ArchiveVertex {
                position,
                normal: [0., 0., 1.],
                tangent: [1., 0., 0., 1.],
                uv: [0.5; 2],
            })
            .collect(),
        indices: vec![0, 1, 2],
        instances: vec![
            ArchiveInstance::new(Mat4::IDENTITY),
            ArchiveInstance::new(Mat4::from_scale(Vec3::new(1., 1., 0.))),
        ],
        material: 0,
        center: Vec3::ZERO,
    };
    let scene = Arc::new(archive_source_scene(device, &[mesh])?);
    let materials = [ArchiveMaterialSource {
        parameters: ArchiveMaterial::MIRROR,
        images: [None; 4],
        double_sided: false,
    }];
    let mut source = vec![0u32; 17 * 20];
    for i in 0..17 {
        let r = &mut source[i * 20..(i + 1) * 20];
        r[2] = (-2f32).to_bits();
        r[10] = 4f32.to_bits();
        r[3] = u32::from(i < 11);
        r[18] = i as u32;
        r[19] = 0x564f5859;
    }
    source[0] = 4f32.to_bits();
    source[4] = (-4f32).to_bits();
    source[20 + 4] = 4f32.to_bits();
    source[40] = 1;
    source[60 + 10] = 0;
    source[60 + 14] = 4f32.to_bits();
    source[80 + 14] = (-4f32).to_bits();
    source[100] = 16777216f32.to_bits();
    source[104] = (-16777216f32).to_bits();
    source[120 + 3] = 0;
    source[140] = f32::INFINITY.to_bits();
    source[160 + 2] = 0;
    source[180 + 2] = (-4f32).to_bits();
    let expected = [1u32, 0, 1, 1, 0, 1, 3, 2, 1, 1, 1];
    let mut captures = Vec::new();
    for mode in 0..4 {
        let native = mode % 2 == 1;
        let witness = mode >= 2;
        let job = ExactWorldNearest::with_options(
            device,
            scene.clone(),
            &materials,
            0,
            17,
            ExactWorldOptions {
                query_kind: if witness {
                    voxy_render::WorldRayQueryKind::HistoryAdmission
                } else {
                    voxy_render::WorldRayQueryKind::Nearest
                },
                history_any_hit: witness,
                node_reciprocal: std::env::var("VOXY_RAY_SHARED_NODE_RECIPROCAL").as_deref()
                    == Ok("1"),
                guarded_input: true,
                certify_visibility: true,
                defer_nearest: true,
                native_seed: native,
                separate_native_seed: native,
                native_source_witness: witness
                    && native
                    && std::env::var("VOXY_RAY_SHARED_NATIVE_SOURCE_WITNESS").as_deref() == Ok("1"),
                ..Default::default()
            },
        )?;
        queue.write_buffer(job.input(), 0, bytemuck::cast_slice(&source));
        let mut encoder = device.create_command_encoder(&Default::default());
        job.build_scene(&mut encoder);
        queue.submit([encoder.finish()]);
        let mut previous: Option<Vec<u8>> = None;
        let mut batches = Vec::new();
        for count in [17u32, 0, 1, 2, 5, 11, 16, 17] {
            let mut encoder = device.create_command_encoder(&Default::default());
            job.encode(&mut encoder, count)?;
            queue.submit([encoder.finish()]);
            let bytes = read(device, queue, job.output(), 17 * 80)?;
            if let Some(before) = &previous {
                assert_eq!(
                    &before[count as usize * 80..],
                    &bytes[count as usize * 80..],
                    "partial hint query changed unrequested outputs"
                );
            }
            #[allow(clippy::possible_missing_else)]
            for (i, record) in bytes.chunks_exact(80).enumerate().take(11) {
                let words: Vec<u32> = record
                    .chunks_exact(4)
                    .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
                    .collect();
                if witness && expected[i] == 1 {
                    assert!(words[0] == 1 || words[0] == 2, "witness safety row {i}");
                } else {
                    assert_eq!(words[0], expected[i], "hardware hint safety row {i}");
                }
                assert_eq!(&words[4..6], &source[i * 20 + 18..i * 20 + 20]);
                if expected[i] == 1 && words[0] == 1 {
                    assert_eq!(
                        &words[8..10],
                        &[
                            if i == 8 {
                                0
                            } else if i == 9 {
                                1f32.to_bits()
                            } else {
                                0.5f32.to_bits()
                            },
                            0
                        ]
                    );
                }
                if witness
                    && native
                    && std::env::var("VOXY_RAY_SHARED_NATIVE_SOURCE_WITNESS").as_deref() == Ok("1")
                {
                    assert_eq!(
                        words[18] & 65536 != 0,
                        i == 10,
                        "native proof must only accept the strict interior source hit"
                    );
                }
            }
            batches.push(bytes.clone());
            previous = Some(bytes);
        }
        let after = read(device, queue, job.input(), 17 * 80)?;
        for (i, word) in after.chunks_exact(4).enumerate() {
            if ![7, 11, 15].contains(&(i % 20)) {
                assert_eq!(u32::from_le_bytes(word.try_into().unwrap()), source[i]);
            }
        }
        std::fs::write(
            root.join(format!("hint-safety-{mode}.bin")),
            previous.unwrap(),
        )?;
        captures.push(batches);
    }
    for mode in 1..4 {
        for (before, after) in captures[0].iter().zip(&captures[mode]) {
            for (b, a) in before.chunks_exact(80).zip(after.chunks_exact(80)) {
                let b: Vec<u32> = b
                    .chunks_exact(4)
                    .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
                    .collect();
                let a: Vec<u32> = a
                    .chunks_exact(4)
                    .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
                    .collect();
                assert_eq!(&a[4..6], &b[4..6]);
                if mode >= 2 && b[0] == 1 && a[0] == 2 {
                    assert!([1024, 4096, 8192].contains(&a[1]));
                } else {
                    assert_eq!(a[0], b[0]);
                    assert_eq!(&a[6..16], &b[6..16]);
                    assert_eq!(a[17], b[17]);
                }
            }
        }
    }
    std::fs::write(
        root.join("hint-safety-input.bin"),
        bytemuck::cast_slice(&source),
    )?;
    std::fs::write(
        root.join("hint-safety.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"expected_first_states":expected,"batches":[17,0,1,2,5,11,16,17],"modes":["nearest","nearest-native","witness","witness-native"],"hardware_absence_not_a_miss":true,"misleading_high_parts_recertified":true,"subnormal_and_low_direction_preserved":true,"closed_endpoints":true,"singular_proxy_source_instance":true,"unrequested_output_tail_preserved":true,"coefficients_and_ids_preserved":true}),
        )?,
    )?;
    println!(
        "SEPARATE HINT SAFETY PASS paired-origin/direction subnormal zero-ray inactive nonfinite closed-endpoints partial-batches original-bits=true"
    );
    Ok(())
}
pub(super) fn deferred_queue_protocol(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Include high-entropy source words: the queue must not alter any of them
    // outside the three explicitly reserved padding fields.
    for count in [1u32, 17, 257, 1050] {
        let mut seed = 0x564f5859u32;
        let mut words = Vec::new();
        for _ in 0..count * 20 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            words.push(seed);
        }
        words[7] = 0;
        words[11] = 0;
        let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("deferred queue original words"),
            contents: bytemuck::cast_slice(&words),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        let proof = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(count) * 8,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}",
                    include_str!("../../../voxy_render/src/world_ray_deferred_input.wgsl"),
                    r"
@group(0) @binding(1) var<storage,read_write> proof:array<vec2<u32>>;
@compute @workgroup_size(64) fn append(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&deferred_rays) {return;}
 if !deferred_append(id.x) {proof[id.x]=vec2<u32>(0xffffffffu);}
}
@compute @workgroup_size(1) fn consume(@builtin(global_invocation_id) id:vec3<u32>) {
 loop {let ray=deferred_take();if ray==0xffffffffu {break;}
  // Exactly one consumer visits each queued source index.
  proof[ray]=vec2<u32>(ray,exact_ray_word(ray*20u+18u));
 }
}
"
                )
                .into(),
            ),
        });
        // Explicit layout shared across pipelines; no exclusive auto-layout alias.
        let binding = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[binding(0), binding(1)],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let append = pipeline("append");
        let consume = pipeline("consume");
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: proof.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&append);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&consume);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(count.min(16), 1, 1);
        }
        queue.submit([encoder.finish()]);
        let after = read(device, queue, &input, u64::from(count) * 80)?;
        let proof = read(device, queue, &proof, u64::from(count) * 8)?;
        let after: Vec<u32> = after
            .chunks_exact(4)
            .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
            .collect();
        for (i, (&old, &new)) in words.iter().zip(&after).enumerate() {
            if ![7, 11, 15].contains(&(i % 20)) {
                assert_eq!(old, new, "queue changed source word {i}");
            }
        }
        assert_eq!(after[7], count);
        assert!(after[11] >= count);
        let mut ids: Vec<u32> = after.chunks_exact(20).map(|r| r[15]).collect();
        ids.sort_unstable();
        assert_eq!(ids, (0..count).collect::<Vec<_>>());
        for (i, row) in proof.chunks_exact(8).enumerate() {
            assert_eq!(u32::from_le_bytes(row[..4].try_into().unwrap()), i as u32);
            assert_eq!(
                u32::from_le_bytes(row[4..].try_into().unwrap()),
                words[i * 20 + 18]
            );
        }
        std::fs::write(
            root.join(format!("deferred-{count}-before.bin")),
            bytemuck::cast_slice(&words),
        )?;
        std::fs::write(
            root.join(format!("deferred-{count}-after.bin")),
            bytemuck::cast_slice(&after),
        )?;
        std::fs::write(root.join(format!("deferred-{count}-proof.bin")), proof)?;
    }
    println!(
        "DEFERRED SOURCE QUEUE PASS batches=1,17,257,1050 coefficients_and_ids_bit_exact=true no_duplicate_or_dropped_indices=true"
    );
    Ok(())
}
// Analytic source workload: one leaf/triangle, one interior hit and one
// root-box miss. Ray zero is deliberately excluded from diagnostic sampling.
pub(super) fn traversal_work_protocol(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("VOXY_RAY_SHARED_WORK_STATS").as_deref() != Ok("1") {
        return Ok(());
    }
    let mesh = ArchiveMeshSource {
        vertices: [[-2., -2., 0.], [2., -2., 0.], [0., 2., 0.]]
            .into_iter()
            .map(|position| ArchiveVertex {
                position,
                normal: [0., 0., 1.],
                tangent: [1., 0., 0., 1.],
                uv: [0.5; 2],
            })
            .collect(),
        indices: vec![0, 1, 2],
        instances: vec![ArchiveInstance::new(Mat4::IDENTITY)],
        material: 0,
        center: Vec3::ZERO,
    };
    let scene = Arc::new(archive_source_scene(device, &[mesh])?);
    let materials = [ArchiveMaterialSource {
        parameters: ArchiveMaterial::MIRROR,
        images: [None; 4],
        double_sided: true,
    }];
    let job = ExactWorldNearest::with_options(
        device,
        scene,
        &materials,
        0,
        3,
        ExactWorldOptions {
            previous_hit_seed: true,
            traversal_work_stats: true,
            guarded_input: true,
            certify_visibility: true,
            defer_nearest: true,
            ..Default::default()
        },
    )?;
    let mut source = vec![0u32; 60];
    for i in 0..3 {
        source[i * 20 + 2] = 1f32.to_bits();
        source[i * 20 + 3] = 1;
        source[i * 20 + 10] = (-2f32).to_bits();
        source[i * 20 + 18] = i as u32;
    }
    source[40] = 10f32.to_bits();
    queue.write_buffer(job.input(), 0, bytemuck::cast_slice(&source));
    let mut encoder = device.create_command_encoder(&Default::default());
    job.build_scene(&mut encoder);
    job.encode(&mut encoder, 3)?;
    queue.submit([encoder.finish()]);
    let decode = |b: Vec<u8>| {
        b.chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect::<Vec<_>>()
    };
    let initial = decode(read(
        device,
        queue,
        job.traversal_work_stats_buffer().unwrap(),
        48,
    )?);
    assert_eq!(initial, vec![2, 0, 2, 0, 1, 0, 2, 0, 1, 0, 1, 1]);
    // Explicit artificial seeds solely test carrying; never presented as work.
    let mut seed = vec![0u32; 12];
    for i in (0..10).step_by(2) {
        seed[i] = u32::MAX - 1;
        seed[i + 1] = 7;
    }
    queue.write_buffer(
        job.traversal_work_stats_buffer().unwrap(),
        0,
        bytemuck::cast_slice(&seed),
    );
    let mut carry = Vec::new();
    for step in 1..=2u64 {
        let mut encoder = device.create_command_encoder(&Default::default());
        job.encode(&mut encoder, 3)?;
        queue.submit([encoder.finish()]);
        let words = decode(read(
            device,
            queue,
            job.traversal_work_stats_buffer().unwrap(),
            48,
        )?);
        for (pair, delta) in [2u64, 2, 1, 2, 1].into_iter().enumerate() {
            let at = pair * 2;
            let actual = u64::from(words[at]) | (u64::from(words[at + 1]) << 32);
            let expected = (7u64 << 32) + u64::from(u32::MAX - 1) + step * delta;
            assert_eq!(
                actual, expected,
                "GPU counter carry pair {pair} step {step}"
            );
        }
        carry.push(words);
    }
    let before = carry.last().unwrap().clone();
    for count in [0, 1] {
        let mut encoder = device.create_command_encoder(&Default::default());
        job.encode(&mut encoder, count)?;
        queue.submit([encoder.finish()]);
        let after = decode(read(
            device,
            queue,
            job.traversal_work_stats_buffer().unwrap(),
            48,
        )?);
        assert_eq!(after, before);
    }
    // Previous source hit is deliberately moved to a current miss with the same
    // stable ID. Its failed source re-certification is still one triangle test.
    source[20] = 10f32.to_bits();
    queue.write_buffer(job.input(), 0, bytemuck::cast_slice(&source));
    queue.write_buffer(
        job.traversal_work_stats_buffer().unwrap(),
        0,
        bytemuck::cast_slice(&[0u32; 12]),
    );
    let mut encoder = device.create_command_encoder(&Default::default());
    job.encode(&mut encoder, 3)?;
    queue.submit([encoder.finish()]);
    let failed_seed = decode(read(
        device,
        queue,
        job.traversal_work_stats_buffer().unwrap(),
        48,
    )?);
    assert_eq!(failed_seed, vec![2, 0, 2, 0, 1, 0, 2, 0, 1, 0, 1, 1]);
    let protected = decode(read(device, queue, job.input(), 240)?);
    for i in 0..60 {
        if ![7, 11, 15].contains(&(i % 20)) {
            assert_eq!(protected[i], source[i]);
        }
    }
    std::fs::write(
        root.join("traversal-work-protocol.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"analytic_initial_words":initial,"expected_nodes":2,"expected_triangles":1,"sampled_records":2,"first_ray_excluded":true,"carry_test_seeds_artificial":seed,"carry_results":carry,"failed_seed_counted":true,"failed_seed_words":failed_seed,"zero_and_single_batch_no_samples":true,"protected_words_bit_exact":true}),
        )?,
    )?;
    println!(
        "TRAVERSAL WORK PROTOCOL PASS analytic_nodes=2 analytic_triangles=1 carry64=true protected_source=true"
    );
    Ok(())
}
pub(super) fn guard_protocol(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    job: &ExactWorldNearest,
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor{label:Some("explicit admission GPU producer"),source:wgpu::ShaderSource::Wgsl(r"
@group(0) @binding(0) var<storage,read_write> rays:array<u32>;
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id:vec3<u32>){let base=id.x*20u;for(var k=0u;k<20u;k++){rays[base+k]=0u;}rays[base+3u]=id.x;rays[base+18u]=123u+id.x;if id.x==1u{rays[base+2u]=0x3f800000u;rays[base+10u]=0xc0000000u;}if id.x==2u{rays[base]=0x7fc00001u;}}
".into())});
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: job.input().as_entire_binding(),
        }],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    job.build_scene(&mut encoder);
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(3, 1, 1);
    }
    job.encode(&mut encoder, 3)?;
    queue.submit([encoder.finish()]);
    let record_bytes = u64::from(job.output_record_words()) * 4;
    let data = read(device, queue, job.output(), record_bytes * 3)?;
    for (i, row) in data.chunks_exact(record_bytes as usize).enumerate() {
        let w: Vec<u32> = row
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect();
        assert_eq!(w[4], 123 + i as u32);
        assert_eq!(w[0], [3, 1, 2][i]);
        if i == 2 {
            assert_eq!(w[1], 512);
        }
        if i != 1 {
            assert_eq!(w[6], u32::MAX);
            assert!(w[8..16].iter().all(|&v| v == 0));
        }
    }
    std::fs::write(root.join("guard-protocol.bin"), data)?;
    Ok(())
}
// Exercise workspace reuse, partial binding views, and successive batch lengths.
pub(super) fn bounded_visibility_protocol(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    job: &ExactWorldNearest,
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bounded visibility batch producer"),
        source: wgpu::ShaderSource::Wgsl(
            r"
@group(0) @binding(0) var<storage,read_write> rays:array<u32>;
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let base=id.x*20u;for(var k=0u;k<20u;k++){rays[base+k]=0u;}
 let mode=id.x%4u;rays[base+3u]=select(1u,0u,mode==0u);
 rays[base+18u]=1000u+id.x;rays[base+19u]=0xabcdef01u;
 rays[base+2u]=0x3f800000u;rays[base+10u]=0xc0000000u;
 if mode==2u {rays[base]=0x7fc00001u;}
 if mode==3u {rays[base+4u]=1u;}
}
@compute @workgroup_size(1) fn rewrite(@builtin(global_invocation_id) id:vec3<u32>) {
 rays[id.x*20u+18u]+=10000u;
}
"
            .into(),
        ),
    });
    let pipeline = |entry| {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            cache: None,
        })
    };
    let init = pipeline("main");
    let rewrite = pipeline("rewrite");
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &init.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: job.input().as_entire_binding(),
        }],
    });
    let rewrite_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &rewrite.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: job.input().as_entire_binding(),
        }],
    });
    let capacity = job.capacity();
    let expected_workers = capacity.min(
        std::env::var("VOXY_RAY_SHARED_WORKERS")
            .ok()
            .map(|s| s.parse::<u32>())
            .transpose()?
            .unwrap_or(16),
    );
    assert_eq!(job.exact_worker_count(), expected_workers);
    assert_eq!(
        job.workspace_bytes(),
        u64::from(expected_workers) * 512 * 128 * 4
    );
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&init);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(capacity, 1, 1);
    }
    job.encode(&mut encoder, capacity)?;
    queue.submit([encoder.finish()]);
    let full = read(device, queue, job.output(), u64::from(capacity) * 80)?;
    for (i, row) in full.chunks_exact(80).enumerate() {
        let word = |k: usize| u32::from_le_bytes(row[k * 4..k * 4 + 4].try_into().unwrap());
        assert_eq!(word(0), [3, 1, 2, 1][i % 4], "bounded ray {i}");
        assert_eq!(word(4), 1000 + i as u32);
        assert_eq!(word(5), 0xabcdef01);
        if i % 4 == 2 {
            assert_eq!(word(1), 64);
        }
    }
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&rewrite);
        pass.set_bind_group(0, &rewrite_group, &[]);
        pass.dispatch_workgroups(capacity, 1, 1);
    }
    for count in [17, 1, 0] {
        job.encode(&mut encoder, count)?;
    }
    assert!(job.encode(&mut encoder, capacity + 1).is_err());
    queue.submit([encoder.finish()]);
    let partial = read(device, queue, job.output(), u64::from(capacity) * 80)?;
    for (i, row) in partial.chunks_exact(80).enumerate() {
        let id = u32::from_le_bytes(row[16..20].try_into().unwrap());
        assert_eq!(id, 1000 + i as u32 + if i < 17 { 10000 } else { 0 });
        if i >= 17 {
            assert_eq!(row, &full[i * 80..i * 80 + 80]);
        }
    }
    std::fs::write(root.join("bounded-full.bin"), full)?;
    std::fs::write(root.join("bounded-partial.bin"), partial)?;
    println!(
        "BOUNDED VISIBILITY PASS rays={capacity} workers={expected_workers} partial=17,1,0 tail_unchanged=true exact_subnormal=true"
    );
    Ok(())
}
