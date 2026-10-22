//! Actual archive frame -> shared GPU world-ray producer -> exact job -> consumer.
use glam::{Mat4, Vec3};
use std::{path::Path, sync::Arc};
use voxy_render::{ExactWorldNearest, ExactWorldOptions, WorldRayKind, archive_scene::*};
use wgpu::util::DeviceExt;
fn archive_source_scene(
    device: &wgpu::Device,
    meshes: &[ArchiveMeshSource],
) -> Result<voxy_render::RayScene, voxy_render::RaySceneError> {
    if std::env::var("VOXY_STREET_WORLD_RAY_PRIMITIVE_GROUPS").as_deref() == Ok("1") {
        voxy_render::RayScene::from_archive_candidates_with_primitive_groups(device, meshes, 0.)
    } else {
        voxy_render::RayScene::from_archive_candidates(device, meshes, 0.)
    }
}
fn read(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    bytes: u64,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &read, 0, bytes);
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    })?;
    rx.recv()??;
    let data = read.slice(..).get_mapped_range()?.to_vec();
    read.unmap();
    Ok(data)
}
fn image_dump(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    bytes_per_pixel: u32,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let row = texture.width() * bytes_per_pixel;
    assert_eq!(row % 256, 0);
    let size = u64::from(row) * u64::from(texture.height());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("source admission image comparison"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(texture.height()),
            },
        },
        texture.size(),
    );
    queue.submit([encoder.finish()]);
    let bytes = read(device, queue, &buffer, size)?;
    if bytes_per_pixel == 16 {
        assert!(
            bytes
                .chunks_exact(4)
                .all(|w| f32::from_le_bytes(w.try_into().unwrap()).is_finite())
        );
    } else {
        assert!(
            bytes
                .chunks_exact(2)
                .all(|w| u16::from_le_bytes(w.try_into().unwrap()) & 0x7c00 != 0x7c00)
        );
    }
    std::fs::write(path, bytes)?;
    Ok(())
}
fn image_region_compare(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    actual: &wgpu::Texture,
    reference: &wgpu::Texture,
    root: &Path,
    kind: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let row = actual.width() * 16;
    assert_eq!(row % 256, 0);
    let size = u64::from(row) * u64::from(actual.height());
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("source image region independent reference"),
        size: size * 2,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    for (index, texture) in [actual, reference].into_iter().enumerate() {
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: index as u64 * size,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(actual.height()),
                },
            },
            texture.size(),
        );
    }
    queue.submit([encoder.finish()]);
    let bytes = read(device, queue, &buffer, size * 2)?;
    let split = size as usize;
    let mut outside = 0;
    let mut changed_inside = 0;
    for (index, (a, b)) in bytes[..split]
        .chunks_exact(16)
        .zip(bytes[split..].chunks_exact(16))
        .enumerate()
    {
        let x = index as u32 % actual.width();
        let y = index as u32 / actual.width();
        if (12..16).contains(&x) && (12..16).contains(&y) {
            changed_inside += usize::from(a != b);
        } else {
            assert_eq!(
                a, b,
                "source region changed an unrelated {kind} pixel ({x},{y})"
            );
            outside += 1;
        }
        for word in a.chunks_exact(4) {
            assert!(f32::from_le_bytes(word.try_into().unwrap()).is_finite());
        }
    }
    std::fs::write(
        root.join(format!("image-{kind}-actual.bin")),
        &bytes[..split],
    )?;
    std::fs::write(
        root.join(format!("image-{kind}-reference.bin")),
        &bytes[split..],
    )?;
    println!(
        "WORLD IMAGE REGION {kind} outside_pixels_bit_exact={outside} changed_inside={changed_inside} finite=true"
    );
    Ok(())
}
fn reciprocal_node_audit(
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
    let source=[include_str!("../../voxy_render/src/archive_projection_pair.wgsl"),include_str!("../../voxy_render/src/world_triangle_pair.wgsl"),include_str!("../../voxy_render/src/world_ray_bvh_interval.wgsl"),include_str!("../../voxy_render/src/world_node_reciprocal.wgsl"),r"
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
fn modular_word_audit(
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
    let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:Some("shared exact modular word audit"),source:wgpu::ShaderSource::Wgsl(format!("{}\n{}",include_str!("../../voxy_render/src/world_modular_words.wgsl"),r"
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
fn scale(
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
fn separate_hint_safety(
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
            root.join(format!("hint-safety-{}.bin", mode)),
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
fn deferred_queue_protocol(
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
                    include_str!("../../voxy_render/src/world_ray_deferred_input.wgsl"),
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
fn traversal_work_protocol(
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
fn guard_protocol(
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
fn bounded_visibility_protocol(
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
fn consume(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    renderer: &ArchiveSceneRenderer,
    job: &ExactWorldNearest,
    producer: &voxy_render::ArchiveWorldRayProducer,
    root: &Path,
    name: &str,
) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    let count = producer.count();
    let hit_words = job.output_record_words();
    let row_bytes = u64::from(20 + hit_words) * 4;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("live-ray GPU consumer"),
        size: u64::from(count) * row_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let source = format!(
        "const HIT_WORDS:u32={hit_words}u;\n{}",
        r"
@group(0) @binding(0) var<storage,read> rays:array<u32>;
@group(0) @binding(1) var<storage,read> hits:array<u32>;
@group(0) @binding(2) var<storage,read_write> result:array<u32>;
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id:vec3<u32>){let base=id.x*(20u+HIT_WORDS);for(var k=0u;k<20u;k++){result[base+k]=rays[id.x*20u+k];}for(var k=0u;k<HIT_WORDS;k++){result[base+20u+k]=hits[id.x*HIT_WORDS+k];}}
"
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
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
                resource: job.input().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: job.output().as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    producer.trace(job, &mut encoder)?;
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(count, 1, 1);
    }
    queue.submit([encoder.finish()]);
    let data = read(device, queue, &output, u64::from(count) * row_bytes)?;
    let mut input = Vec::new();
    let mut hits = Vec::new();
    let mut cases = Vec::new();
    let mut states = [0u32; 4];
    for row in data.chunks_exact(row_bytes as usize) {
        let r: Vec<u32> = row
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect();
        input.extend_from_slice(&row[..80]);
        hits.extend_from_slice(&row[80..]);
        assert_eq!([r[18], r[19]], [r[24], r[25]]);
        states[r[20] as usize] += 1;
        if r[3] == 0 {
            assert_eq!(r[20], 3);
        } else if r[3] == 2 {
            assert_eq!([r[20], r[21]], [2, 512]);
        } else {
            assert!(r[20] <= 2);
            cases.push(serde_json::json!({"global_ray":u64::from(r[18])|(u64::from(r[19])<<32),"ray_words":&r[..20]}));
        }
    }
    std::fs::write(root.join(format!("{name}-input.bin")), &input)?;
    std::fs::write(root.join(format!("{name}-hits.bin")), hits)?;
    let mut cases_jsonl = String::new();
    for v in &cases {
        use std::fmt::Write;
        let _ = writeln!(cases_jsonl, "{v}");
    }
    std::fs::write(root.join(format!("{name}-cases.jsonl")), cases_jsonl)?;
    println!(
        "LIVE PRODUCER {name} queries={count} states={states:?} renderer_aa={:?}",
        renderer.antialiasing()
    );
    Ok(input
        .chunks_exact(4)
        .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
        .collect())
}
// A real small receiver sits on a reflected live ray at exactly t=1/64.
// Its affine source transform represents origin + direction*(60/64) exactly;
// it does not cover that ray's primary raster pixel. No hit record is injected.
fn exact_image_hit_fixture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    live_ray: &[u8],
    output: &wgpu::Texture,
    root: &Path,
    optical: bool,
    one_sided: bool,
    tail: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    exact_image_hit_fixture_impl(
        device, queue, live_ray, output, root, optical, one_sided, tail, false,
    )
}
#[allow(clippy::fn_params_excessive_bools)]
fn exact_image_hit_fixture_impl(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    live_ray: &[u8],
    output: &wgpu::Texture,
    root: &Path,
    optical: bool,
    one_sided: bool,
    tail: bool,
    canonicalized: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let position_consumer =
        std::env::var("VOXY_RAY_SHARED_POSITION_CONSUMER").as_deref() == Ok("1");
    let residual_position = position_consumer
        && tail
        && std::env::var("VOXY_RAY_SHARED_POSITION_RESIDUAL").as_deref() == Ok("1");
    let words: Vec<u32> = live_ray
        .chunks_exact(4)
        .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
        .collect();
    let origin = Vec3::from_array(std::array::from_fn(|axis| f32::from_bits(words[axis])));
    let direction = Vec3::from_array(std::array::from_fn(|axis| {
        ((f64::from(f32::from_bits(words[8 + axis])) + f64::from(f32::from_bits(words[12 + axis])))
            / 60.) as f32
    }));
    let mesh = |points: [[f32; 3]; 4], normal: [f32; 3], world: Mat4| ArchiveMeshSource {
        vertices: points
            .into_iter()
            .map(|position| ArchiveVertex {
                position,
                normal,
                tangent: [1., 0., 0., 1.],
                uv: [0.5; 2],
            })
            .collect(),
        indices: vec![0, 1, 2, 0, 2, 3],
        instances: vec![ArchiveInstance::new(world)],
        material: 0,
        center: Vec3::ZERO,
    };
    let front = mesh(
        [[-2., -2., 0.], [2., -2., 0.], [2., 2., 0.], [-2., 2., 0.]],
        [0., 0., 1.],
        Mat4::IDENTITY,
    );
    let corners = if tail {
        [
            [-0.00390625, -0.001953125],
            [0.00390625, -0.001953125],
            [0.00390625, 0.00390625],
            [-0.00390625, 0.00390625],
        ]
    } else {
        [[-0.05, -0.04], [0.06, -0.04], [0.06, 0.07], [-0.05, 0.07]]
    };
    let mut receiver = mesh(
        corners.map(|xy| [direction.x + xy[0], direction.y + xy[1], direction.z]),
        [0., 0., -1.],
        Mat4::from_scale_rotation_translation(
            Vec3::splat(if tail { 1.5 } else { 0.9375 }),
            glam::Quat::IDENTITY,
            origin,
        ),
    );
    if residual_position {
        for v in &mut receiver.vertices {
            v.position[2] += 0.0009765625;
        }
    }
    let position_tilt = std::env::var("VOXY_RAY_SHARED_POSITION_TILT_NUMERATOR")
        .ok()
        .map(|v| v.parse::<u16>())
        .transpose()?
        .unwrap_or(0);
    if residual_position && position_tilt != 0 {
        let slope = f32::from(position_tilt) / 16.;
        for v in &mut receiver.vertices {
            v.position[2] += (v.position[0] - direction.x) * slope;
        }
    }
    receiver.material = usize::from(optical || one_sided);
    if position_consumer && one_sided {
        receiver.indices = vec![0, 2, 1, 0, 3, 2];
    }
    let meshes = vec![front, receiver];
    let mut materials = vec![ArchiveMaterialSource {
        parameters: ArchiveMaterial::MIRROR,
        images: [None; 4],
        double_sided: true,
    }];
    if optical || one_sided {
        let mut glass = ArchiveMaterial::MIRROR;
        if optical {
            glass.surface[2] = 1.;
        }
        materials.push(ArchiveMaterialSource {
            parameters: glass,
            images: [None; 4],
            double_sided: !one_sided,
        });
    }
    if one_sided {
        materials[0].double_sided = false;
    }
    let prefix_base = if optical {
        "optical-image-hit"
    } else if one_sided {
        "one-sided-image-hit"
    } else if tail {
        "tail-image-hit"
    } else {
        "exact-image-hit"
    };
    let prefix = if residual_position {
        "position-residual-image-hit".to_owned()
    } else if position_consumer {
        format!("position-{prefix_base}")
    } else {
        prefix_base.to_owned()
    };
    let history_admission = !position_consumer
        && std::env::var("VOXY_RAY_SHARED_HISTORY_ADMISSION").as_deref() == Ok("1");
    let scene = Arc::new(archive_source_scene(device, &meshes)?);
    if std::env::var("VOXY_RAY_SHARED_HISTORY_POSITION_ADMISSION").as_deref() == Ok("1") {
        for (kind, point) in [
            (voxy_render::WorldRayQueryKind::Nearest, true),
            (voxy_render::WorldRayQueryKind::HistoryAdmission, true),
            (voxy_render::WorldRayQueryKind::OpaqueVisibility, true),
            (voxy_render::WorldRayQueryKind::MaterialSurface, false),
        ] {
            let bad = ExactWorldNearest::with_options(
                device,
                scene.clone(),
                &materials,
                0,
                16,
                ExactWorldOptions {
                    query_kind: kind,
                    hit_position: point,
                    history_position_admission: true,
                    ..Default::default()
                },
            );
            assert!(
                matches!(bad, Err(voxy_render::RaySceneError::Unsupported)),
                "history position admission escaped its explicit render-only scope"
            );
        }
    }
    let position_exact_vertex_cache =
        std::env::var("VOXY_RAY_SHARED_POSITION_EXACT_VERTEX_CACHE").as_deref() == Ok("1");
    let position_cache_not_ready =
        std::env::var("VOXY_RAY_SHARED_POSITION_VERTEX_CACHE_NOT_READY").as_deref() == Ok("1");
    let position_native_seed =
        std::env::var("VOXY_RAY_SHARED_POSITION_NATIVE_SEED").as_deref() == Ok("1");
    let job = Arc::new(ExactWorldNearest::with_options(
        device,
        scene,
        &materials,
        0,
        16,
        ExactWorldOptions {
            wide_source_nodes: std::env::var("VOXY_RAY_SHARED_POSITION_WIDE_NODES").as_deref() == Ok("1"),
            skip_zero_prefix_multiply: std::env::var(
                "VOXY_RAY_SHARED_POSITION_SKIP_ZERO_PREFIX_MULTIPLY",
            )
            .as_deref()
                == Ok("1"),
            cached_relative_interval: std::env::var(
                "VOXY_RAY_SHARED_POSITION_CACHED_RELATIVE_INTERVAL",
            )
            .as_deref()
                == Ok("1"),
            geometry_bounds_cache: std::env::var("VOXY_RAY_SHARED_POSITION_GEOMETRY_BOUNDS_CACHE")
                .as_deref()
                == Ok("1"),
            fused_source_affine: std::env::var("VOXY_RAY_SHARED_POSITION_FUSED_SOURCE_AFFINE")
                .as_deref()
                == Ok("1"),
            exact_vertex_cache: position_exact_vertex_cache,
            native_seed: position_native_seed,
            separate_native_seed: position_native_seed,
            native_triangle_hint: std::env::var("VOXY_RAY_SHARED_POSITION_NATIVE_TRIANGLE_HINT")
                .as_deref()
                == Ok("1"),
            certified_ratio_seed: std::env::var("VOXY_RAY_SHARED_CERTIFIED_RATIO_SEED").as_deref()
                == Ok("1"),
            history_position_admission: std::env::var("VOXY_RAY_SHARED_HISTORY_POSITION_ADMISSION")
                .as_deref()
                == Ok("1"),
            hit_position: position_consumer,
            query_kind: if position_consumer {
                voxy_render::WorldRayQueryKind::MaterialSurface
            } else if history_admission {
                voxy_render::WorldRayQueryKind::HistoryAdmission
            } else {
                voxy_render::WorldRayQueryKind::Nearest
            },
            history_any_hit: !position_consumer
                && std::env::var("VOXY_RAY_SHARED_ANY_HIT_ADMISSION").as_deref() == Ok("1"),
            guarded_input: true,
            certify_visibility: true,
            defer_nearest: true,
            ..Default::default()
        },
    )?);
    if std::env::var("VOXY_RAY_SHARED_POSITION_WIDE_NODES").as_deref() == Ok("1") {
        assert!(job.wide_source_nodes_enabled(), "small consumer fixture must actually admit BVH8");
    }
    let mut encoder = device.create_command_encoder(&Default::default());
    job.build_scene(&mut encoder);
    if position_cache_not_ready {
        let buffer = job
            .exact_vertex_cache_buffer()
            .ok_or("cache not-ready fixture requires an enabled vertex cache")?;
        let header = job
            .exact_vertex_cache_header()
            .ok_or("vertex cache header missing")?;
        encoder.clear_buffer(buffer, u64::from(header) * 4, Some(4));
    }
    queue.submit([encoder.finish()]);
    let vertex_cache_diagnostics = if position_exact_vertex_cache {
        let buffer = job
            .exact_vertex_cache_buffer()
            .ok_or("enabled cache missing")?;
        let header = job
            .exact_vertex_cache_header()
            .ok_or("cache header missing")? as usize;
        let bytes = read(device, queue, buffer, buffer.size())?;
        let words: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect();
        let h = &words[header..header + 16];
        assert_eq!(
            (h[0], h[4], h[5], h[6]),
            (u32::from(!position_cache_not_ready), 12, 0, h[2])
        );
        let admitted = (0..h[2] as usize)
            .filter(|&i| (0..3).all(|axis| words[h[3] as usize + i * 12 + axis * 4] & 32768 != 0))
            .count();
        assert!(
            admitted > 0,
            "directed fixture must exercise admitted lossless records"
        );
        Some(
            serde_json::json!({"ready":h[0],"world_vertices":h[2],"checked_all":h[6],"mismatches":h[5],"admitted_vertices":admitted}),
        )
    } else {
        None
    };
    let source_geometry:Vec<_>=meshes.iter().map(|m|serde_json::json!({"vertices":m.vertices.iter().map(|v|v.position.map(f32::to_bits)).collect::<Vec<_>>(),"world":m.instances[0].world.map(|r|r.map(f32::to_bits)),"indices":m.indices,"material":m.material})).collect();
    let mut renderer = ArchiveSceneRenderer::new(
        device,
        queue,
        wgpu::TextureFormat::Rgba16Float,
        32,
        32,
        vec![],
        materials,
        meshes,
        &[],
        Vec3::new(0.2, 1., 0.3).normalize(),
        [1., 1., 1., 10.],
        (Vec3::splat(-2.), Vec3::splat(4.)),
    )?;
    for _ in 0..2 {
        renderer.draw(
            &output.create_view(&Default::default()),
            Vec3::new(0.1, 0., 3.),
            -Vec3::Z,
        );
    }
    renderer.add_world_ray_region(job.clone(), WorldRayKind::Reflection, [12, 12], [4, 4])?;
    renderer.draw(
        &output.create_view(&Default::default()),
        Vec3::new(0.1, 0., 3.),
        -Vec3::Z,
    );
    let input = read(device, queue, job.input(), 16 * 80)?;
    let actual_words: Vec<u32> = input[..80]
        .chunks_exact(4)
        .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
        .collect();
    let same = (0..4)
        .all(|part| (0..3).all(|axis| actual_words[part * 4 + axis] == words[part * 4 + axis]));
    if !same {
        assert!(
            !canonicalized,
            "canonical front receiver ray did not stabilize"
        );
        // A subdivided parent scene can rasterize a slightly different origin.
        // Seed this directed receiver from ITS actual GPU front-quad ray instead
        // of weakening the exact t assertion or discarding paired low words.
        drop(renderer);
        drop(job);
        return exact_image_hit_fixture_impl(
            device,
            queue,
            &input[..80],
            output,
            root,
            optical,
            one_sided,
            tail,
            true,
        );
    }
    let hits = read(device, queue, job.output(), 16 * 80)?;
    let first: Vec<u32> = hits[..80]
        .chunks_exact(4)
        .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
        .collect();
    let position_admitted = first[0] == 2 && first[1] == 16384;
    if position_admitted {
        assert!(position_consumer && residual_position);
        assert_ne!(first[18] & 131072, 0);
    } else if history_admission && (optical || one_sided || tail) {
        assert_eq!(first[0], 2);
        assert_eq!(
            first[1],
            if one_sided && job.history_any_hit_enabled() {
                8192
            } else if optical {
                2048
            } else if one_sided {
                1024
            } else {
                4096
            }
        );
    } else {
        assert_eq!(first[0], 1, "live reflected ray must hit the receiver");
        assert_eq!(first[6], 1, "source identity must be the receiver");
        if residual_position {
            assert_ne!(first[15] & 1, 0);
        } else if tail {
            assert_eq!(first[8], 0x3ccccccd);
            assert_eq!(first[15] & 1, 1);
        } else {
            assert_eq!(
                [first[8], first[9], first[15] & 1],
                [0x3c800000, 0, 0],
                "receiver must have exact t=1/64 with no remaining rational tail"
            );
        }
    }
    let counts = read(
        device,
        queue,
        renderer.world_ray_region_audit_buffer(0).unwrap(),
        32,
    )?;
    let counts: Vec<u32> = counts
        .chunks_exact(4)
        .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
        .collect();
    assert_eq!(counts.iter().sum::<u32>(), 16);
    if position_consumer {
        let position = read(
            device,
            queue,
            job.output(),
            job.world_position_offset_bytes().unwrap() + 16 * 32,
        )?;
        let position = &position[job.world_position_offset_bytes().unwrap() as usize..];
        let first_position: Vec<u32> = position[..32]
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect();
        if position_admitted {
            assert_eq!(first_position, vec![0; 8]);
        } else {
            assert_eq!(first_position[6], 7);
            if residual_position {
                assert_ne!(
                    first_position[7], 0,
                    "negative fixture must retain an actual rational world-position tail"
                );
            } else {
                assert_eq!(
                    first_position[7], 0,
                    "direct fixture must have a fully represented exact world point"
                );
            }
        }
        std::fs::write(root.join(format!("{prefix}-position.bin")), position)?;
        if residual_position {
            assert_eq!(counts[2], 0);
            assert!(counts[3] + counts[5] > 0);
        } else if optical {
            assert_eq!(counts[2], 0);
            assert!(counts[4] > 0);
        } else {
            assert!(
                counts[2] > 0,
                "exact point and material-sided source must feed radiance"
            );
        }
    } else if optical || one_sided || tail {
        assert!(
            counts[if history_admission {
                5
            } else if optical {
                4
            } else if tail {
                3
            } else {
                7
            }] > 0,
            "unsupported source hit must retain full original handling"
        );
        assert_eq!(counts[2], 0);
    } else {
        assert!(
            counts[2] > 0,
            "source hit must reach actual radiance reconstruction"
        );
    }
    image_dump(
        device,
        queue,
        output,
        8,
        &root.join(format!("{prefix}-native.bin")),
    )?;
    image_dump(
        device,
        queue,
        &renderer
            .opaque_gi_precision_textures()
            .ok_or("GI precision texture required")?
            .0,
        16,
        &root.join(format!("{prefix}-gi.bin")),
    )?;
    image_dump(
        device,
        queue,
        &renderer
            .opaque_ssr_precision_textures()
            .ok_or("SSR precision texture required")?
            .0,
        16,
        &root.join(format!("{prefix}-ssr.bin")),
    )?;
    std::fs::write(root.join(format!("{prefix}-input.bin")), input)?;
    std::fs::write(root.join(format!("{prefix}-output.bin")), hits)?;
    std::fs::write(
        root.join(format!("{prefix}-fixture.json")),
        serde_json::to_vec_pretty(
            &serde_json::json!({"wide_source_nodes":job.wide_source_nodes_enabled(),"wide_source_block_blake3":job.wide_source_digest(),"skip_zero_prefix_multiply":job.skip_zero_prefix_multiply_enabled(),"exact_integer_source_blake3":job.exact_integer_source_digest(),"fused_source_affine":std::env::var("VOXY_RAY_SHARED_POSITION_FUSED_SOURCE_AFFINE").as_deref()==Ok("1"),"exact_vertex_cache_diagnostics":vertex_cache_diagnostics,"exact_vertex_cache":position_exact_vertex_cache,"exact_vertex_cache_ready_cleared":position_cache_not_ready,"native_seed":position_native_seed,"native_triangle_hint":std::env::var("VOXY_RAY_SHARED_POSITION_NATIVE_TRIANGLE_HINT").as_deref()==Ok("1"),"history_position_admission":std::env::var("VOXY_RAY_SHARED_HISTORY_POSITION_ADMISSION").as_deref()==Ok("1"),"first_position_admitted":position_admitted,"residual_position":residual_position,"position_consumer":position_consumer,"source_geometry":source_geometry,"expected_first_t_words":if tail {None}else{Some([0x3c800000u32,0])},"expected_first_t_rational":if residual_position {"derive-from-original-source"}else if tail {"1/40"}else{"1/64"},"branch_counts":counts,"image_size":[32,32],"region":[12,12,4,4],"optical":optical,"one_sided":one_sided,"tail":tail,"history_admission":history_admission,"any_hit_admission_enabled":job.history_any_hit_enabled(),"full_street_qualified":false}),
        )?,
    )?;
    if one_sided
        && !position_consumer
        && std::env::var("VOXY_RAY_SHARED_POSITIVE_CACHE").as_deref() == Ok("1")
    {
        assert!(
            job.history_any_hit_enabled(),
            "positive cache fixture requires material fallback admission"
        );
        assert_eq!(
            std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE_CHECK").as_deref(),
            Ok("1")
        );
        renderer.clear_world_ray_regions();
        renderer.set_world_ray_frame(job.clone(), [4, 4])?;
        assert_eq!(
            renderer.paired_dda_tile_edge_cache_enabled(),
            std::env::var("VOXY_STREET_DDA_TILE_EDGE_CACHE").as_deref() == Ok("1")
        );
        assert_eq!(
            renderer.world_ray_consumer_sorted(),
            std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_SORTED").as_deref() == Ok("1"),
            "prepared frame did not capture requested sorted consumer mode"
        );
        for _ in 0..3 {
            renderer.draw(
                &output.create_view(&Default::default()),
                Vec3::new(0.1, 0., 3.),
                -Vec3::Z,
            );
        }
        device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })?;
        let mut positive = Vec::new();
        for (kind, pair) in [
            (
                "gi",
                renderer
                    .opaque_gi_precision_textures()
                    .ok_or("positive GI reference missing")?,
            ),
            (
                "ssr",
                renderer
                    .opaque_ssr_precision_textures()
                    .ok_or("positive SSR reference missing")?,
            ),
        ] {
            let actual = root.join(format!("positive-cache-{kind}-actual.bin"));
            let reference = root.join(format!("positive-cache-{kind}-reference.bin"));
            image_dump(device, queue, &pair.0, 16, &actual)?;
            image_dump(device, queue, &pair.1, 16, &reference)?;
            let bytes = std::fs::read(actual)?;
            assert_eq!(
                bytes,
                std::fs::read(reference)?,
                "full positive cached {kind} differs from independent inline reference"
            );
            let count = bytes
                .chunks_exact(16)
                .filter(|pixel| {
                    let rgba: Vec<f32> = pixel
                        .chunks_exact(4)
                        .map(|v| f32::from_le_bytes(v.try_into().unwrap()))
                        .collect();
                    rgba[3] > 0. && rgba[..3].iter().any(|&v| v != 0.)
                })
                .count();
            assert!(
                count > 0,
                "positive cache fixture has no nonzero {kind} radiance"
            );
            positive.push(count);
        }
        let mut totals = [0u64; 4];
        let mut counters = Vec::new();
        for region in 0..renderer.world_ray_region_count() {
            let bytes = read(
                device,
                queue,
                renderer.world_ray_region_audit_buffer(region).unwrap(),
                48,
            )?;
            let words: Vec<u32> = bytes
                .chunks_exact(4)
                .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
                .collect();
            assert_eq!(words[10], 0, "positive cache Float32 mismatch");
            assert_eq!(words[11], 0);
            for i in 0..4 {
                totals[i] += u64::from(words[8 + i]);
            }
            counters.push(words);
        }
        assert!(
            totals[1] > totals[0],
            "positive cached contributions were not checked"
        );
        std::fs::write(
            root.join("positive-cache-proof.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"image_size":[32,32],"tile":[4,4],"regions":renderer.world_ray_region_count(),"positive_rgb_alpha_pixels_gi_ssr":positive,"images_match_independent_inline_bits":true,"checked_nonpruned_contributions":totals[1]-totals[0],"totals_pruned_checked_mismatch_nonfinite":totals,"raw_region_counters":counters,"full_street_qualified":false,"fps_qualified":false}),
            )?,
        )?;
        println!(
            "POSITIVE CONSUMER CACHE PASS positive_gi={} positive_ssr={} checked_nonpruned={} full_images_bit_exact=true",
            positive[0],
            positive[1],
            totals[1] - totals[0]
        );
    }
    println!(
        "WORLD IMAGE HIT PASS optical={optical} original_live_ray=true source_affine_geometry=true exact_t={} radiance_branch_count={} optical_fallback_count={} no_uploaded_hit_record=true",
        if residual_position {
            "derive-from-original-source"
        } else if tail {
            "1/40"
        } else {
            "1/64"
        },
        counts[2],
        counts[4]
    );
    Ok(())
}
pub(super) fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::var_os("VOXY_RAY_SHARED_PRODUCER_OUTPUT")
        .ok_or("producer audit output directory required")?;
    let root = Path::new(&root);
    std::fs::create_dir(root)?;
    scale(device, queue, root)?;
    reciprocal_node_audit(device, queue, root)?;
    deferred_queue_protocol(device, queue, root)?;
    separate_hint_safety(device, queue, root)?;
    if std::env::var("VOXY_RAY_SHARED_HISTORY_ADMISSION").as_deref() == Ok("1") {
        modular_word_audit(device, queue, root)?;
    }
    let quads = [
        (
            [[-2., -2., 0.], [2., -2., 0.], [2., 2., 0.], [-2., 2., 0.]],
            [0., 0., 1.],
        ),
        (
            [
                [-2., -2., 0.],
                [-2., 2., 0.],
                [-2., 2., 64.],
                [-2., -2., 64.],
            ],
            [1., 0., 0.],
        ),
        (
            [[2., -2., 0.], [2., -2., 64.], [2., 2., 64.], [2., 2., 0.]],
            [-1., 0., 0.],
        ),
        (
            [
                [-2., -2., 0.],
                [-2., -2., 64.],
                [2., -2., 64.],
                [2., -2., 0.],
            ],
            [0., 1., 0.],
        ),
        (
            [[-2., 2., 0.], [2., 2., 0.], [2., 2., 64.], [-2., 2., 64.]],
            [0., -1., 0.],
        ),
    ];
    let visibility = std::env::var("VOXY_RAY_SHARED_PRODUCER_VISIBILITY").as_deref() == Ok("1");
    let defer_visibility =
        visibility && std::env::var("VOXY_RAY_SHARED_PRODUCER_DEFERRED").as_deref() == Ok("1");
    let defer_nearest =
        !visibility && std::env::var("VOXY_RAY_SHARED_PRODUCER_NEAREST_GATE").as_deref() == Ok("1");
    let history_admission =
        std::env::var("VOXY_RAY_SHARED_HISTORY_ADMISSION").as_deref() == Ok("1");
    let query_kind = if visibility {
        voxy_render::WorldRayQueryKind::OpaqueVisibility
    } else if history_admission {
        voxy_render::WorldRayQueryKind::HistoryAdmission
    } else {
        voxy_render::WorldRayQueryKind::Nearest
    };
    let hit_uv = std::env::var("VOXY_RAY_SHARED_PRODUCER_UV").as_deref() == Ok("1");
    let uv_words = [
        [0x3eaaaaab, 1],
        [0x3f666666, 0x80000001],
        [0x3dcccccd, 2],
        [0x3f800001, 0],
    ];
    let mut meshes: Vec<_> = quads
        .into_iter()
        .map(|(points, normal)| ArchiveMeshSource {
            vertices: points
                .into_iter()
                .enumerate()
                .map(|(i, position)| ArchiveVertex {
                    position,
                    normal,
                    tangent: [1., 0., 0., 1.],
                    uv: if hit_uv {
                        uv_words[i].map(f32::from_bits)
                    } else {
                        [0.5; 2]
                    },
                })
                .collect(),
            indices: vec![0, 1, 2, 0, 2, 3],
            instances: vec![ArchiveInstance::new(Mat4::IDENTITY)],
            material: 0,
            center: Vec3::ZERO,
        })
        .collect();
    if let Some(n) = std::env::var("VOXY_RAY_SHARED_SUBDIVISIONS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
    {
        if !(2..=32).contains(&n) {
            return Err("fixture subdivisions must be 2..32".into());
        }
        for mesh in &mut meshes {
            let corners = [mesh.vertices[0], mesh.vertices[1], mesh.vertices[3]];
            mesh.vertices.clear();
            mesh.indices.clear();
            for y in 0..n {
                for x in 0..n {
                    let first = mesh.vertices.len() as u32;
                    for [dx, dy] in [[0, 0], [1, 0], [1, 1], [0, 1]] {
                        let u = (x + dx) as f32 / n as f32;
                        let v = (y + dy) as f32 / n as f32;
                        let mut vertex = corners[0];
                        for axis in 0..3 {
                            vertex.position[axis] = corners[0].position[axis]
                                + (corners[1].position[axis] - corners[0].position[axis]) * u
                                + (corners[2].position[axis] - corners[0].position[axis]) * v;
                        }
                        for axis in 0..2 {
                            vertex.uv[axis] = corners[0].uv[axis]
                                + (corners[1].uv[axis] - corners[0].uv[axis]) * u
                                + (corners[2].uv[axis] - corners[0].uv[axis]) * v;
                        }
                        mesh.vertices.push(vertex);
                    }
                    mesh.indices
                        .extend([first, first + 1, first + 2, first, first + 2, first + 3]);
                }
            }
        }
    }
    let scene = Arc::new(archive_source_scene(device, &meshes)?);
    if let Some((_, hierarchy)) = scene.archive_exact_source_metadata() {
        std::fs::write(
            root.join("source-hierarchy.bin"),
            bytemuck::cast_slice(&hierarchy),
        )?;
    }
    let materials = vec![ArchiveMaterialSource {
        parameters: ArchiveMaterial::MIRROR,
        images: [None; 4],
        double_sided: std::env::var("VOXY_RAY_SHARED_ALL_ONE_SIDED").as_deref() != Ok("1"),
    }];
    let mut exported = Vec::new();
    for (i, m) in meshes.iter().enumerate() {
        let mut item = serde_json::json!({"id":i,"material":0,"vertex_count":m.vertices.len(),"index_count":m.indices.len(),"instance_count":m.instances.len()});
        for (field, data) in [
            ("vertices", bytemuck::cast_slice(&m.vertices)),
            ("indices", bytemuck::cast_slice(&m.indices)),
            ("instances", bytemuck::cast_slice(&m.instances)),
        ] {
            let name = format!("{i}-{field}.bin");
            std::fs::write(root.join(&name), data)?;
            item[field] = serde_json::json!({"file":name,"bytes":data.len()});
        }
        exported.push(item);
    }
    std::fs::write(
        root.join("material.bin"),
        bytemuck::bytes_of(&ArchiveMaterial::MIRROR),
    )?;
    std::fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"schema":"voxy-full-world-ray-geometry-v1","meshes":exported,"materials":[{"id":0,"parameters":{"file":"material.bin","bytes":80},"images":[null,null,null,null],"double_sided":materials[0].double_sided}],"image_count":0}),
        )?,
    )?;
    let job = Arc::new(ExactWorldNearest::with_options(
        device,
        scene.clone(),
        &materials,
        0,
        if visibility || defer_nearest { 257 } else { 16 },
        ExactWorldOptions {
            guarded_input: true,
            hit_uv,
            query_kind,
            certify_visibility: visibility || defer_nearest,
            defer_nearest,
            exact_worker_limit: std::env::var("VOXY_RAY_SHARED_WORKERS")
                .ok()
                .map(|s| s.parse::<std::num::NonZeroU32>())
                .transpose()?,
            traversal_work_stats: std::env::var("VOXY_RAY_SHARED_WORK_STATS").as_deref() == Ok("1"),
            exact_group_size: std::env::var("VOXY_RAY_SHARED_GROUP")
                .ok()
                .map(|s| s.parse::<std::num::NonZeroU32>())
                .transpose()?,
            previous_hit_seed: defer_nearest
                && std::env::var("VOXY_RAY_SHARED_PREVIOUS_SEED").as_deref() == Ok("1"),
            history_any_hit: std::env::var("VOXY_RAY_SHARED_ANY_HIT_ADMISSION").as_deref()
                == Ok("1"),
            node_reciprocal: std::env::var("VOXY_RAY_SHARED_NODE_RECIPROCAL").as_deref() == Ok("1"),
            native_source_witness: std::env::var("VOXY_RAY_SHARED_NATIVE_SOURCE_WITNESS")
                .as_deref()
                == Ok("1"),
            separate_native_seed: std::env::var("VOXY_RAY_SHARED_SEPARATE_NATIVE_SEED").as_deref()
                == Ok("1"),
            native_seed: defer_nearest
                && std::env::var("VOXY_RAY_SHARED_NATIVE_SEED").as_deref() == Ok("1"),
            defer_visibility,
            ..Default::default()
        },
    )?);
    // Build this retained owner before testing native candidates.
    let mut encoder = device.create_command_encoder(&Default::default());
    job.build_scene(&mut encoder);
    queue.submit([encoder.finish()]);
    traversal_work_protocol(device, queue, root)?;
    guard_protocol(device, queue, &job, root)?;
    if (visibility || defer_nearest) && !hit_uv {
        bounded_visibility_protocol(device, queue, &job, root)?;
    }
    let wrong_job = ExactWorldNearest::with_options(
        device,
        scene,
        &materials,
        0,
        16,
        ExactWorldOptions {
            guarded_input: true,
            hit_uv,
            query_kind,
            certify_visibility: visibility || defer_nearest,
            defer_nearest,
            exact_worker_limit: std::env::var("VOXY_RAY_SHARED_WORKERS")
                .ok()
                .map(|s| s.parse::<std::num::NonZeroU32>())
                .transpose()?,
            traversal_work_stats: std::env::var("VOXY_RAY_SHARED_WORK_STATS").as_deref() == Ok("1"),
            exact_group_size: std::env::var("VOXY_RAY_SHARED_GROUP")
                .ok()
                .map(|s| s.parse::<std::num::NonZeroU32>())
                .transpose()?,
            previous_hit_seed: defer_nearest
                && std::env::var("VOXY_RAY_SHARED_PREVIOUS_SEED").as_deref() == Ok("1"),
            history_any_hit: std::env::var("VOXY_RAY_SHARED_ANY_HIT_ADMISSION").as_deref()
                == Ok("1"),
            node_reciprocal: std::env::var("VOXY_RAY_SHARED_NODE_RECIPROCAL").as_deref() == Ok("1"),
            native_source_witness: std::env::var("VOXY_RAY_SHARED_NATIVE_SOURCE_WITNESS")
                .as_deref()
                == Ok("1"),
            separate_native_seed: std::env::var("VOXY_RAY_SHARED_SEPARATE_NATIVE_SEED").as_deref()
                == Ok("1"),
            native_seed: defer_nearest
                && std::env::var("VOXY_RAY_SHARED_NATIVE_SEED").as_deref() == Ok("1"),
            defer_visibility,
            ..Default::default()
        },
    )?;
    let mut renderer = ArchiveSceneRenderer::new(
        device,
        queue,
        wgpu::TextureFormat::Rgba16Float,
        32,
        32,
        vec![],
        materials,
        meshes,
        &[],
        Vec3::new(0.2, 1., 0.3).normalize(),
        [1., 1., 1., 10.],
        (Vec3::new(-2., -2., 0.), Vec3::new(2., 2., 64.)),
    )?;
    let output = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 32,
            height: 32,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    renderer.draw(
        &output.create_view(&Default::default()),
        Vec3::new(0., 0., 3.),
        -Vec3::Z,
    );
    let diffuse = renderer.world_ray_producer(&job, WorldRayKind::Diffuse, [12, 12], [4, 4])?;
    let reflected =
        renderer.world_ray_producer(&job, WorldRayKind::Reflection, [12, 12], [4, 4])?;
    assert!(
        renderer
            .world_ray_producer(&job, WorldRayKind::Diffuse, [31, 31], [2, 2])
            .is_err()
    );
    assert!(
        renderer
            .world_ray_producer(
                &job,
                WorldRayKind::Diffuse,
                [0, 0],
                if visibility || defer_nearest {
                    [17, 17]
                } else {
                    [5, 5]
                }
            )
            .is_err()
    );
    let mut rejected = device.create_command_encoder(&Default::default());
    assert!(diffuse.trace(&wrong_job, &mut rejected).is_err());
    let inactive = consume(
        device,
        queue,
        &renderer,
        &job,
        &diffuse,
        root,
        "first-frame-diffuse",
    )?;
    assert!(inactive.chunks_exact(20).all(|r| r[3] == 0));
    renderer.draw(
        &output.create_view(&Default::default()),
        Vec3::new(0., 0., 3.),
        -Vec3::Z,
    );
    let active = consume(device, queue, &renderer, &job, &diffuse, root, "diffuse")?;
    assert!(active.chunks_exact(20).all(|r| r[3] == 1));
    consume(
        device,
        queue,
        &renderer,
        &job,
        &reflected,
        root,
        "reflection",
    )?;
    renderer.draw(
        &output.create_view(&Default::default()),
        Vec3::new(0.1, 0., 3.),
        -Vec3::Z,
    );
    let moved = consume(
        device,
        queue,
        &renderer,
        &job,
        &diffuse,
        root,
        "diffuse-moved",
    )?;
    if std::env::var("VOXY_RAY_SHARED_PREVIOUS_SEED").as_deref() == Ok("1") {
        consume(
            device,
            queue,
            &renderer,
            &job,
            &diffuse,
            root,
            "diffuse-repeat",
        )?;
        renderer.draw(
            &output.create_view(&Default::default()),
            Vec3::new(0.125, 0., 3.),
            -Vec3::Z,
        );
        consume(
            device,
            queue,
            &renderer,
            &job,
            &diffuse,
            root,
            "diffuse-small-move",
        )?;
        renderer.draw(
            &output.create_view(&Default::default()),
            Vec3::new(1.4, 0., 3.),
            -Vec3::Z,
        );
        consume(
            device,
            queue,
            &renderer,
            &job,
            &diffuse,
            root,
            "diffuse-large-move",
        )?;
    }
    let coefficients = |words: &[u32]| {
        words
            .chunks_exact(20)
            .flat_map(|r| [0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14].map(|i| r[i]))
            .collect::<Vec<_>>()
    };
    assert_ne!(
        coefficients(&active),
        coefficients(&moved),
        "producer retained stale camera/guides"
    );
    if std::env::var("VOXY_RAY_SHARED_IMAGE_REGION").as_deref() == Ok("1") {
        if visibility {
            return Err("image region requires nearest queries".into());
        }
        assert_eq!(renderer.world_ray_region_count(), 0);
        renderer.add_world_ray_region(job.clone(), WorldRayKind::Diffuse, [12, 12], [4, 4])?;
        renderer.add_world_ray_region(job.clone(), WorldRayKind::Reflection, [12, 12], [4, 4])?;
        assert_eq!(renderer.world_ray_region_count(), 2);
        renderer.draw(
            &output.create_view(&Default::default()),
            Vec3::new(0.1, 0., 3.),
            -Vec3::Z,
        );
        device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })?;
        let (actual, reference) = renderer
            .opaque_gi_precision_textures()
            .ok_or("image audit needs GI_PRECISION_PROOF")?;
        image_region_compare(device, queue, actual, reference, root, "gi")?;
        let (actual, reference) = renderer
            .opaque_ssr_precision_textures()
            .ok_or("image audit needs SSR precision textures")?;
        image_region_compare(device, queue, actual, reference, root, "ssr")?;
        if std::env::var_os("VOXY_STREET_WORLD_RAY_RESOLVE_AUDIT").is_none() {
            for index in 0..2 {
                assert!(
                    read(
                        device,
                        queue,
                        renderer.world_ray_region_audit_buffer(index).unwrap(),
                        32
                    )?
                    .iter()
                    .all(|&b| b == 0)
                );
            }
            renderer.clear_world_ray_regions();
            assert_eq!(renderer.world_ray_region_count(), 0);
            println!(
                "LIVE WORLD IMAGE REGION WITHOUT COUNTERS PASS actual_GI_SSR=true outside_pixels_bit_exact=true counter_buffer_zero=true source_owner_release=true full_street=false fps_qualified=false"
            );
            return Ok(());
        }
        let mut totals = [0u32; 8];
        for region in 0..2 {
            let bytes = read(
                device,
                queue,
                renderer.world_ray_region_audit_buffer(region).unwrap(),
                32,
            )?;
            let counts: Vec<u32> = bytes
                .chunks_exact(4)
                .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
                .collect();
            assert_eq!(
                counts.iter().sum::<u32>(),
                16,
                "all output pixels must resolve once"
            );
            assert!(
                counts[3] + counts[5] > 0,
                "live rational tails must retain full original query"
            );
            for (total, count) in totals.iter_mut().zip(counts) {
                *total += count;
            }
        }
        assert!(
            totals[1] > 0,
            "live reflection miss must reach the image consumer"
        );
        std::fs::write(
            root.join("image-region-branch-counts.json"),
            serde_json::to_vec_pretty(&totals)?,
        )?;
        let live_ray = read(device, queue, job.input(), 80)?;
        exact_image_hit_fixture(device, queue, &live_ray, &output, root, false, false, false)?;
        exact_image_hit_fixture(device, queue, &live_ray, &output, root, true, false, false)?;
        exact_image_hit_fixture(device, queue, &live_ray, &output, root, false, true, false)?;
        exact_image_hit_fixture(device, queue, &live_ray, &output, root, false, false, true)?;
        println!(
            "LIVE WORLD IMAGE REGION PASS before_opaque_lighting=true GI=true SSR=true rational_tail_fallback=true region_pixels=16 full_street=false fps_qualified=false"
        );
    }
    if std::env::var("VOXY_RAY_SHARED_FULL_FRAME").as_deref() == Ok("1") {
        renderer.clear_world_ray_regions();
        assert!(renderer.set_world_ray_frame(job.clone(), [17, 17]).is_err());
        assert!(!renderer.world_ray_frame_enabled());
        renderer.set_world_ray_frame(job.clone(), [13, 7])?;
        assert_eq!(
            renderer.paired_dda_tile_edge_cache_enabled(),
            std::env::var("VOXY_STREET_DDA_TILE_EDGE_CACHE").as_deref() == Ok("1")
        );
        assert_eq!(
            renderer.world_ray_consumer_sorted(),
            std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_SORTED").as_deref() == Ok("1"),
            "prepared frame did not capture requested sorted consumer mode"
        );
        assert!(renderer.world_ray_frame_enabled());
        assert_eq!(renderer.world_ray_region_count(), 30);
        assert!(!renderer.opaque_ray_queue_enabled());
        assert!(renderer.opaque_ray_stages_configuration().is_none());
        assert!(
            renderer
                .add_world_ray_region(job.clone(), WorldRayKind::Diffuse, [0, 0], [1, 1])
                .is_err()
        );
        let tiles = renderer.world_ray_frame_tiles().unwrap().to_vec();
        assert_eq!(tiles.last(), Some(&[26, 28, 6, 4]));
        let auditing = std::env::var_os("VOXY_STREET_WORLD_RAY_RESOLVE_AUDIT").is_some();
        renderer.reset_temporal_history();
        renderer.draw(
            &output.create_view(&Default::default()),
            Vec3::new(0.1, 0., 3.),
            -Vec3::Z,
        );
        device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })?;
        assert_eq!(
            renderer.world_ray_query_encoded_frames(),
            u64::from(auditing),
            "no-history default frame must clear without source queries"
        );
        let cache_sticky_test = std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE").as_deref()
            == Ok("1")
            && std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE_CHECK").as_deref() == Ok("1")
            && job.history_any_hit_enabled();
        for pass in 0..5 {
            if pass == 0 && cache_sticky_test {
                let prior = read(
                    device,
                    queue,
                    renderer.world_ray_region_audit_buffer(0).unwrap(),
                    48,
                )?;
                assert_eq!(
                    u32::from_le_bytes(prior[40..44].try_into().unwrap()),
                    0,
                    "first frame already recorded an error"
                );
                queue.write_buffer(
                    renderer.world_ray_region_audit_buffer(0).unwrap(),
                    40,
                    bytemuck::cast_slice(&[1u32]),
                );
            }
            renderer.draw(
                &output.create_view(&Default::default()),
                Vec3::new(0.1, 0., 3.),
                -Vec3::Z,
            );
            device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })?;
            if pass == 0 && cache_sticky_test {
                let bytes = read(
                    device,
                    queue,
                    renderer.world_ray_region_audit_buffer(0).unwrap(),
                    48,
                )?;
                assert_eq!(
                    u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
                    1,
                    "error counter must survive the next frame's clear"
                );
                // Remove only the explicitly injected test marker, after verifying
                // no naturally occurring error was added to it in this draw.
                queue.write_buffer(
                    renderer.world_ray_region_audit_buffer(0).unwrap(),
                    40,
                    bytemuck::cast_slice(&[0u32]),
                );
                std::fs::write(
                    root.join("consumer-cache-sticky-counter.json"),
                    serde_json::to_vec_pretty(
                        &serde_json::json!({"injected_errors":1,"preserved_after_next_frame":1,"natural_errors_added":0,"explicit_test_marker_removed":true}),
                    )?,
                )?;
            }
        }
        assert_eq!(
            renderer.world_ray_query_encoded_frames(),
            5 + u64::from(auditing)
        );
        let mut branch_counts = Vec::new();
        let mut consumer_zero_counts = Vec::new();
        let mut total = 0;
        for (kind_index, kind) in [WorldRayKind::Diffuse, WorldRayKind::Reflection]
            .into_iter()
            .enumerate()
        {
            let mut seen = vec![false; 1024];
            let base = renderer.world_ray_producer(&job, kind, [0, 0], [13, 7])?;
            for (index, &[x, y, w, h]) in tiles.iter().enumerate() {
                let bytes = read(
                    device,
                    queue,
                    renderer
                        .world_ray_region_audit_buffer(kind_index * tiles.len() + index)
                        .unwrap(),
                    32,
                )?;
                let counts: Vec<u32> = bytes
                    .chunks_exact(4)
                    .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
                    .collect();
                if auditing {
                    assert_eq!(counts.iter().sum::<u32>(), w * h);
                } else {
                    assert!(counts.iter().all(|&v| v == 0));
                }
                total += w * h;
                branch_counts.push(counts);
                if std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_ZERO_AUDIT").as_deref() == Ok("1")
                    || std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE").as_deref() == Ok("1")
                {
                    let bytes = read(
                        device,
                        queue,
                        renderer
                            .world_ray_region_audit_buffer(kind_index * tiles.len() + index)
                            .unwrap(),
                        48,
                    )?;
                    let words: Vec<u32> = bytes
                        .chunks_exact(4)
                        .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
                        .collect();
                    if std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE").as_deref() == Ok("1") {
                        assert_eq!(words[10], 0, "cached fallback changed Float32 bits");
                        assert_eq!(words[11], 0);
                    } else {
                        assert_eq!(words[8], words[9] + words[10] + words[11]);
                    }
                    assert!(words[8] <= w * h);
                    consumer_zero_counts.push(words);
                }
                let producer = base.for_region(device, &job, [x, y], [w, h])?;
                let words = consume(
                    device,
                    queue,
                    &renderer,
                    &job,
                    &producer,
                    root,
                    &format!("frame-{kind_index}-tile-{index}"),
                )?;
                for ray in words.chunks_exact(20) {
                    let id = ray[18] as usize;
                    assert!(!seen[id]);
                    seen[id] = true;
                    assert_eq!(ray[19], kind_index as u32);
                }
            }
            assert!(seen.into_iter().all(|v| v));
        }
        if !consumer_zero_counts.is_empty() {
            std::fs::write(
                root.join("consumer-zero-audit.json"),
                serde_json::to_vec_pretty(
                    &serde_json::json!({"diagnostic_only":std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE").as_deref()!=Ok("1"),"consumer_cache":std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE").as_deref()==Ok("1"),"source_queries_skipped":if std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE").as_deref()==Ok("1") {serde_json::Value::Null}else{serde_json::json!(0)},"region_counters":consumer_zero_counts,"material_gate_enabled":job.history_any_hit_enabled(),"tiles":tiles,"fps_qualified":false}),
                )?,
            )?;
        }
        assert_eq!(total, 2048);
        image_dump(
            device,
            queue,
            &output,
            8,
            &root.join("full-frame-native.bin"),
        )?;
        image_dump(
            device,
            queue,
            renderer
                .opaque_gi_precision_textures()
                .ok_or("GI precision texture required")?
                .0,
            16,
            &root.join("full-frame-gi.bin"),
        )?;
        image_dump(
            device,
            queue,
            renderer
                .opaque_ssr_precision_textures()
                .ok_or("SSR precision texture required")?
                .0,
            16,
            &root.join("full-frame-ssr.bin"),
        )?;
        renderer.collect_completed_gpu_times()?;
        if let Some(profile) = renderer.world_ray_stage_profile()? {
            std::fs::write(
                root.join("world-stage-profile.json"),
                serde_json::to_vec_pretty(&profile)?,
            )?;
        }
        let gi = renderer
            .gpu_gi_times_ms
            .last()
            .copied()
            .flatten()
            .ok_or("full frame test requires GPU timing")?;
        let ssr = renderer
            .gpu_ssr_times_ms
            .last()
            .copied()
            .flatten()
            .ok_or("full frame SSR timing missing")?;
        assert!(gi.into_iter().all(|v| v > 0. && v.is_finite()) && ssr > 0. && ssr.is_finite());
        std::fs::write(
            root.join("world-frame.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"size":[32,32],"tiles":tiles,"regions":30,"branch_counts":branch_counts,"queries":2048,"gpu_gi_ms":gi,"gpu_ssr_ms":ssr,"old_whole_frame_traversal_skipped":true,"all_source_ids_once":true,"source_capture_scope":"standalone all-ray producer/query replay after render","render_queries_may_be_consumer_pruned":std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE").as_deref()==Ok("1")&&job.history_any_hit_enabled(),"audit_counters_enabled":auditing,"history_admission":history_admission,"any_hit_admission_enabled":job.history_any_hit_enabled(),"native_source_witness_enabled":job.native_source_witness_enabled(),"node_reciprocal_enabled":job.node_reciprocal_enabled(),"query_encoded_frames":renderer.world_ray_query_encoded_frames(),"consumer_sorted_prepared":renderer.world_ray_consumer_sorted(),"tile_edge_cache_prepared":renderer.paired_dda_tile_edge_cache_enabled(),"full_street":false,"fps_qualified":false}),
            )?,
        )?;
        assert!(renderer.set_world_ray_frame(job.clone(), [17, 17]).is_err());
        assert!(renderer.world_ray_frame_enabled());
        renderer.clear_world_ray_regions();
        assert!(!renderer.world_ray_frame_enabled());
        assert_eq!(renderer.world_ray_region_count(), 0);
        println!(
            "LIVE WORLD FULL FRAME PASS dimensions=32x32 region_count=30 queries=2048 all_original_ids_once=true partial_edges=true shared_kernels=true original_frame_pass_skipped=true GPU_timing_valid=true fps_qualified=false"
        );
    }
    println!(
        "LIVE ARCHIVE WORLD PRODUCER PASS original_ray_function=true exact_scale_words=true GPU_consumer=true guarded_admission=true live_camera_bindings=true image_shading_integrated={} fps_qualified=false",
        std::env::var("VOXY_RAY_SHARED_IMAGE_REGION").as_deref() == Ok("1")
            || std::env::var("VOXY_RAY_SHARED_FULL_FRAME").as_deref() == Ok("1")
    );
    Ok(())
}
