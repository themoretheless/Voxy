//! Actual archive frame -> shared GPU world-ray producer -> exact job -> consumer.
use glam::{Mat4, Vec3};
use std::{path::Path, sync::Arc};
use voxy_render::{ExactWorldNearest, ExactWorldOptions, WorldRayKind, archive_scene::*};

mod image_fixtures;
mod protocols;
use image_fixtures::exact_image_hit_fixture;
use protocols::*;

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
#[expect(
    clippy::too_many_lines,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
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
