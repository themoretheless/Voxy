//! Full original geometry AS build and per-mesh stable candidate identity gate.
use std::path::Path;
use voxy_render::archive_scene::{ArchiveInstance, ArchiveMeshSource, ArchiveVertex};
use wgpu::util::DeviceExt;
fn read<T: bytemuck::Pod>(
    root: &Path,
    descriptor: &serde_json::Value,
) -> Result<Vec<T>, Box<dyn std::error::Error>> {
    let name = descriptor["file"].as_str().ok_or("missing payload")?;
    if Path::new(name).components().count() != 1 {
        return Err("invalid payload name".into());
    }
    let bytes = std::fs::read(root.join(name))?;
    if bytes.len() as u64 != descriptor["bytes"].as_u64().ok_or("missing size")?
        || bytes.len() % std::mem::size_of::<T>() != 0
    {
        return Err("invalid payload length".into());
    }
    Ok(bytes
        .chunks_exact(std::mem::size_of::<T>())
        .map(bytemuck::pod_read_unaligned)
        .collect())
}
const SHADER: &str = r#"
enable wgpu_ray_query;
struct Input { origin:vec4<f32>, direction:vec4<f32>, expected:vec4<u32> }
@group(0) @binding(0) var scene:acceleration_structure;
@group(0) @binding(1) var<storage,read> inputs:array<Input>;
@group(0) @binding(2) var<storage,read_write> output:array<vec4<u32>>;
@group(0) @binding(3) var<storage,read> source_layout:array<vec4<u32>>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&inputs) {return;}let input=inputs[id.x];var q:ray_query;
 rayQueryInitialize(&q,scene,RayDesc(0u,255u,0.,1.,input.origin.xyz,input.direction.xyz));
 var found=0u;var count=0u;
 while rayQueryProceed(&q) {let c=rayQueryGetCandidateIntersection(&q);if c.kind==3u {count+=1u;if c.instance_custom_data==input.expected.x && c.primitive_index==input.expected.y && source_layout[c.instance_custom_data].x==input.expected.z {found=1u;}}}
 output[id.x]=vec4<u32>(found,count,input.expected.x,input.expected.y);
}
"#;
pub(super) fn probe(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json"))?)?;
    if manifest["schema"] != "voxy-full-world-ray-geometry-v1" || !cfg!(target_endian = "little") {
        return Err("unsupported geometry capture".into());
    }
    let mut meshes = Vec::new();
    let mut inputs = Vec::<[u32; 12]>::new();
    let mut slot = 0u32;
    for item in manifest["meshes"].as_array().ok_or("missing meshes")? {
        let vertices = read::<ArchiveVertex>(root, &item["vertices"])?;
        let indices = read::<u32>(root, &item["indices"])?;
        let instances = read::<ArchiveInstance>(root, &item["instances"])?;
        if vertices.len() as u64 != item["vertex_count"].as_u64().ok_or("vertex count")?
            || indices.len() as u64 != item["index_count"].as_u64().ok_or("index count")?
            || instances.len() as u64 != item["instance_count"].as_u64().ok_or("instance count")?
        {
            return Err("count mismatch".into());
        }
        if indices.len() < 3 || indices.iter().any(|&i| i as usize >= vertices.len()) {
            return Err("invalid source mesh".into());
        }
        for (instance_id, instance) in instances.iter().enumerate() {
            let singular = instance.normal.iter().flatten().any(|v| !v.is_finite());
            if instance_id != 0 && instance_id + 1 != instances.len() && !singular {
                continue;
            }
            let w = glam::Mat4::from_cols_array_2d(&instance.world);
            let (primitive, center, normal) = indices
                .chunks_exact(3)
                .enumerate()
                .find_map(|(primitive, triangle)| {
                    let p: Vec<_> = triangle
                        .iter()
                        .map(|&i| {
                            w.transform_point3(glam::Vec3::from(vertices[i as usize].position))
                        })
                        .collect();
                    (p[1] - p[0])
                        .cross(p[2] - p[0])
                        .try_normalize()
                        .map(|normal| (primitive, (p[0] + p[1] + p[2]) / 3., normal))
                })
                .unwrap_or_else(|| {
                    let p: Vec<_>=indices[..3].iter().map(|&i|w.transform_point3(glam::Vec3::from(vertices[i as usize].position))).collect();
                    (0,(p[0]+p[1]+p[2])/3.,glam::Vec3::Z)
                });
            let origin = center + normal * 0.25;
            let direction = -normal;
            let mut input = [0u32; 12];
            for a in 0..3 {
                input[a] = origin[a].to_bits();
                input[a + 4] = direction[a].to_bits();
            }
            input[8] = slot + instance_id as u32;
            input[9] = primitive as u32;
            input[10] = meshes.len() as u32;
            inputs.push(input);
        }
        slot = slot
            .checked_add(instances.len() as u32)
            .ok_or("instance overflow")?;
        meshes.push(ArchiveMeshSource {
            vertices,
            indices,
            instances,
            material: usize::try_from(item["material"].as_u64().ok_or("material")?)?,
            center: glam::Vec3::ZERO,
        });
    }
    let all_triangles = std::env::var("VOXY_RAY_WORLD_FULL_COVERAGE").as_deref() == Ok("1");
    if all_triangles {
        inputs.clear();
        let mut slot = 0u32;
        for (mesh_id, mesh) in meshes.iter().enumerate() {
            for instance in &mesh.instances {
                let world = glam::Mat4::from_cols_array_2d(&instance.world);
                for (primitive, triangle) in mesh.indices.chunks_exact(3).enumerate() {
                    let points: [glam::Vec3; 3] = std::array::from_fn(|k| {
                        world.transform_point3(glam::Vec3::from(
                            mesh.vertices[triangle[k] as usize].position,
                        ))
                    });
                    let center = (points[0] + points[1] + points[2]) / 3.;
                    let normal = (points[1] - points[0])
                        .cross(points[2] - points[0])
                        .try_normalize()
                        .unwrap_or(glam::Vec3::Z);
                    let origin = center + normal * 0.25;
                    let mut ray = [0u32; 12];
                    for axis in 0..3 {
                        ray[axis] = origin[axis].to_bits();
                        ray[axis + 4] = (-normal[axis]).to_bits();
                    }
                    ray[8] = slot;
                    ray[9] = primitive as u32;
                    ray[10] = mesh_id as u32;
                    inputs.push(ray);
                }
                slot += 1;
            }
        }
    }
    let mut scene = voxy_render::RayScene::from_archive_candidates(device, &meshes, 0.0001)?;
    assert_eq!(
        scene.archive_candidate_layout().unwrap().len(),
        meshes.len()
    );
    // Immutable source identity and transforms must stay synchronized with TLAS.
    let first = meshes
        .iter()
        .flat_map(|m| m.instances.iter())
        .next()
        .ok_or("no instances")?;
    let m = first.world;
    let native = [
        m[0][0], m[1][0], m[2][0], m[3][0], m[0][1], m[1][1], m[2][1], m[3][1], m[0][2], m[1][2],
        m[2][2], m[3][2],
    ];
    let native = if glam::Mat4::from_cols_array_2d(&m).determinant() == 0. {
        [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.]
    } else {
        native
    };
    assert_eq!(
        scene.set_instance(0, native, 7, 255),
        Err(voxy_render::RaySceneError::InvalidTransform)
    );
    let mut changed = native;
    changed[3] = if native[3].next_up().is_finite() { native[3].next_up() } else { native[3].next_down() };
    assert_eq!(
        scene.set_instances(&[
            voxy_render::RayInstanceUpdate {
                index: 0,
                transform: native,
                custom_index: 0,
                mask: 0
            },
            voxy_render::RayInstanceUpdate {
                index: 0,
                transform: changed,
                custom_index: 0,
                mask: 255
            },
        ]),
        Err(voxy_render::RaySceneError::InvalidTransform)
    );
    let ray_bytes = inputs.len() as u64 * 48;
    let output_bytes = inputs.len() as u64 * 16;
    let limits = device.limits();
    if ray_bytes > u64::from(limits.max_storage_buffer_binding_size)
        || output_bytes > u64::from(limits.max_storage_buffer_binding_size)
        || (inputs.len() as u32).div_ceil(64) > limits.max_compute_workgroups_per_dimension
    {
        return Err(
            "full coverage exceeds one-dispatch probe capacity; requires lossless batching".into(),
        );
    }
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("world mesh identity rays"),
        contents: bytemuck::cast_slice(&inputs),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = inputs.len() as u64 * 16;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("world mesh identity gate"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
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
                resource: scene.binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: scene
                    .archive_candidate_instance_layout()
                    .ok_or("missing instance layout")?,
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    scene.build(&mut encoder);
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups((inputs.len() as u32).div_ceil(64), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, size);
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let data = readback.slice(..).get_mapped_range()?;
    let values = bytemuck::cast_slice::<u8, [u32; 4]>(&data);
    let missing: Vec<_> = values
        .iter()
        .enumerate()
        .filter(|(_, v)| v[0] != 1)
        .map(|(i, _)| i)
        .collect();
    if !missing.is_empty() {
        return Err(format!("missing world mesh candidates: {missing:?}").into());
    }
    drop(data);
    readback.unmap();
    let expected_vertices: Vec<_> = meshes
        .iter()
        .flat_map(|m| m.vertices.iter().copied())
        .collect();
    let expected_indices: Vec<_> = meshes
        .iter()
        .flat_map(|m| m.indices.iter().copied())
        .collect();
    let expected_instances: Vec<_> = meshes
        .iter()
        .flat_map(|m| m.instances.iter().copied())
        .collect();
    let expected: [&[u8]; 3] = [
        bytemuck::cast_slice(&expected_vertices),
        bytemuck::cast_slice(&expected_indices),
        bytemuck::cast_slice(&expected_instances),
    ];
    let mut retained_bytes = 0;
    for (binding, bytes) in scene
        .archive_candidate_source_bindings()
        .ok_or("missing source buffers")?
        .into_iter()
        .zip(expected)
    {
        let wgpu::BindingResource::Buffer(binding) = binding else {
            return Err("invalid source buffer binding".into());
        };
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("source byte preservation readback"),
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(binding.buffer, binding.offset, &read, 0, bytes.len() as u64);
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let mapped = read.slice(..).get_mapped_range()?;
        if &mapped[..] != bytes {
            return Err("GPU original source bytes changed".into());
        }
        retained_bytes += bytes.len();
    }
    println!(
        "WORLD SOURCE BYTES PASS bytes={retained_bytes} all_triangle_centroid_rays={all_triangles} singular_normals_preserved=true"
    );
    println!(
        "WORLD GEOMETRY PASS meshes={} instances={slot} candidate_primitives={} required_candidates={} proxy_instances={} padding=0.0001 narrow_phase_qualified=false fps_qualified=false",
        meshes.len(),
        scene.primitive_count(),
        inputs.len(),
        scene.archive_candidate_proxy_count().unwrap()
    );
    if std::env::var("VOXY_RAY_WORLD_NARROW").as_deref()==Ok("1") {
        super::world_narrow::probe(device,queue,&scene,&meshes,all_triangles)?;
    }
    if let Some(slot) = std::env::var_os("VOXY_RAY_WORLD_NEAREST_REMOVE_SLOT") {
        if std::env::var("VOXY_RAY_WORLD_EXACT_NEAREST").as_deref() != Ok("1") {
            return Err("removal diagnostic requires nearest mode".into());
        }
        scene.remove_instance(slot.to_str().ok_or("invalid removal slot")?.parse()?)?;
        let mut encoder = device.create_command_encoder(&Default::default());
        scene.build_instances(&mut encoder);
        queue.submit([encoder.finish()]);
    }
    if let Some(cases)=std::env::var_os("VOXY_RAY_WORLD_EXACT_CASES") {
        let truth=std::env::var_os("VOXY_RAY_WORLD_EXACT_TRUTH").ok_or("exact cases require an oracle for diagnostic qualification")?;
        super::world_exact::probe(device,queue,&scene,&meshes,std::path::Path::new(&cases),std::path::Path::new(&truth),root)?;
    }
    Ok(())
}
