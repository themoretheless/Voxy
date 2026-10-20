//! Diagnostic compensated narrow phase. Ambiguous records remain explicit work;
//! neither native nearest hits nor unresolved intervals become precision claims.
use glam::DVec3;
use voxy_render::archive_scene::ArchiveMeshSource;
const BATCH: usize = 65_536;
fn split(value: f64) -> [f32; 2] {
    let hi = value as f32;
    [hi, (value - f64::from(hi)) as f32]
}
fn decoded(ray: &[u32; 20], start: usize) -> DVec3 {
    DVec3::from_array(std::array::from_fn(|k| {
        f64::from(f32::from_bits(ray[start + k])) + f64::from(f32::from_bits(ray[start + 4 + k]))
    }))
}
fn reference(p: [DVec3; 3], origin: DVec3, direction: DVec3) -> Option<[f64; 3]> {
    let e1 = p[1] - p[0];
    let e2 = p[2] - p[0];
    let cross = direction.cross(e2);
    let determinant = e1.dot(cross);
    if determinant == 0. || !determinant.is_finite() {
        return None;
    }
    let delta = origin - p[0];
    let q = delta.cross(e1);
    Some([
        e2.dot(q) / determinant,
        delta.dot(cross) / determinant,
        direction.dot(q) / determinant,
    ])
}
fn failure_case(
    meshes: &[ArchiveMeshSource],
    ray: &[u32; 20],
    record: &[u32; 8],
    reference: [f64; 3],
) -> serde_json::Value {
    let mut slot = ray[16] as usize;
    for mesh in meshes {
        if slot < mesh.instances.len() {
            let primitive = ray[17] as usize;
            let local: Vec<_> = mesh.indices[primitive * 3..primitive * 3 + 3]
                .iter()
                .map(|&i| mesh.vertices[i as usize].position)
                .collect();
            return serde_json::json!({"ray_words":ray,"gpu_words":record,"world":mesh.instances[slot].world,"local_triangle":local,"f64_reference":reference});
        }
        slot -= mesh.instances.len();
    }
    serde_json::json!({"error":"invalid stable slot"})
}
pub(super) fn probe(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &voxy_render::RayScene,
    meshes: &[ArchiveMeshSource],
    all: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rays = Vec::<[u32; 20]>::new();
    let mut references = Vec::new();
    let mut slot = 0u32;
    for mesh in meshes {
        for (instance_id, instance) in mesh.instances.iter().enumerate() {
            if !all && instance_id != 0 && !instance.normal.iter().flatten().any(|v| !v.is_finite())
            {
                slot += 1;
                continue;
            }
            let world = glam::Mat4::from_cols_array_2d(&instance.world).as_dmat4();
            for (primitive, triangle) in mesh.indices.chunks_exact(3).enumerate() {
                let p: [DVec3; 3] = std::array::from_fn(|k| {
                    world.transform_point3(
                        glam::Vec3::from(mesh.vertices[triangle[k] as usize].position).as_dvec3(),
                    )
                });
                let normal = (p[1] - p[0])
                    .cross(p[2] - p[0])
                    .try_normalize()
                    .unwrap_or(DVec3::Z);
                let origin = (p[0] + p[1] + p[2]) / 3. + normal * 0.25;
                let direction = -normal;
                let mut ray = [0u32; 20];
                for k in 0..3 {
                    let o = split(origin[k]);
                    let d = split(direction[k]);
                    ray[k] = o[0].to_bits();
                    ray[k + 4] = o[1].to_bits();
                    ray[k + 8] = d[0].to_bits();
                    ray[k + 12] = d[1].to_bits();
                }
                ray[16] = slot;
                ray[17] = primitive as u32;
                references.push(reference(p, decoded(&ray, 0), decoded(&ray, 8)));
                rays.push(ray);
            }
            slot += 1;
        }
    }
    let mut expected_states = None;
    if let Some(path) = std::env::var_os("VOXY_RAY_WORLD_NARROW_CASES") {
        let cases: Vec<serde_json::Value> = serde_json::from_slice(&std::fs::read(path)?)?;
        rays.clear();
        references.clear();
        let mut states = Vec::new();
        for case in cases {
            let words = case["ray_words"].as_array().ok_or("case ray words")?;
            if words.len() != 20 {
                return Err("case ray ABI".into());
            }
            let mut ray = [0u32; 20];
            for (out, word) in ray.iter_mut().zip(words) {
                *out = u32::try_from(word.as_u64().ok_or("ray word")?)?;
            }
            let source = failure_case(meshes, &ray, &[0u32; 8], [0.; 3]);
            let world: [[f32; 4]; 4] = serde_json::from_value(source["world"].clone())?;
            let local: [[f32; 3]; 3] = serde_json::from_value(source["local_triangle"].clone())?;
            let matrix = glam::Mat4::from_cols_array_2d(&world).as_dmat4();
            let points = local.map(|p| matrix.transform_point3(glam::Vec3::from(p).as_dvec3()));
            references.push(reference(points, decoded(&ray, 0), decoded(&ray, 8)));
            rays.push(ray);
            states.push(u32::try_from(
                case["expected_state"]
                    .as_u64()
                    .ok_or("case expected state")?,
            )?);
        }
        expected_states = Some(states);
    }
    for (id, ray) in rays.iter_mut().enumerate() {
        ray[18] = id as u32;
        ray[19] = ((id as u64) >> 32) as u32;
    }
    let queued_enabled = std::env::var("VOXY_RAY_WORLD_GPU_EXACT_QUEUE").as_deref() == Ok("1");
    let mut queued_gold = std::collections::BTreeMap::<u64, bool>::new();
    let parameter_mode = std::env::var("VOXY_RAY_WORLD_EXACT_PARAMETERS").as_deref() == Ok("1");
    let mut parameter_gold = std::collections::BTreeMap::<u64, Option<(Vec<u32>, u32)>>::new();
    if queued_enabled {
        let path = std::env::var_os("VOXY_RAY_WORLD_EXACT_TRUTH")
            .ok_or("queued qualification requires exact truth")?;
        let truth: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
        let rows = truth["rows"].as_array().ok_or("missing exact truth rows")?;
        for row in rows {
            let id = row["global_ray"].as_u64().ok_or("missing exact truth ID")?;
            let hit = row["exact_hit"]
                .as_bool()
                .ok_or("missing exact truth outcome")?;
            if parameter_mode {
                if row.get("parameter_words").is_none() {
                    return Err("queued parameters require rational parameter gold".into());
                }
                let expected = if let Some(values) = row["parameter_words"].as_array() {
                    let words = values
                        .iter()
                        .map(|v| Ok(u32::try_from(v.as_u64().ok_or("parameter word")?)?))
                        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
                    Some((
                        words,
                        u32::try_from(row["residual_mask"].as_u64().ok_or("residual mask")?)?,
                    ))
                } else {
                    None
                };
                parameter_gold.insert(id, expected);
            }
            if queued_gold.insert(id, hit).is_some() {
                return Err("duplicate exact oracle IDs".into());
            }
        }
    }
    let capacity = rays.len().min(BATCH);
    if capacity == 0 {
        return Err("no world narrow rays".into());
    }
    let input = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bounded high-low world rays"),
        size: capacity as u64 * 80,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("world narrow results"),
        size: capacity as u64 * 32,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: capacity as u64 * 32,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let exact_job = super::world_exact_queue::QueuedExact::new(device, capacity);
    let source = format!(
        "enable wgpu_ray_query;\n{}\n{}\n{}\n{}",
        include_str!("../../voxy_render/src/archive_projection_pair.wgsl"),
        include_str!("../../voxy_render/src/world_triangle_pair.wgsl"),
        include_str!("../../voxy_render/src/world_origin_expansion.wgsl"),
        include_str!("world_narrow.wgsl").replacen("enable wgpu_ray_query;", "", 1)
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("compensated world triangle interval gate"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let [vertices, indices, instances] = scene
        .archive_candidate_source_bindings()
        .ok_or("missing original sources")?;
    let mut raw_file = if let Some(root) = std::env::var_os("VOXY_RAY_WORLD_NARROW_CAPTURE") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir(&root)?;
        Some(
            std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(root.join("results.bin"))?,
        )
    } else {
        None
    };
    let mut fallback_file = if let Some(root) = std::env::var_os("VOXY_RAY_WORLD_NARROW_CAPTURE") {
        Some(
            std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(std::path::PathBuf::from(root).join("fallback-cases.jsonl"))?,
        )
    } else {
        None
    };
    let mut queued_file = if queued_enabled {
        if let Some(root) = std::env::var_os("VOXY_RAY_WORLD_NARROW_CAPTURE") {
            Some(
                std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(std::path::PathBuf::from(root).join("queued-exact.bin"))?,
            )
        } else {
            None
        }
    } else {
        None
    };
    let mut parameter_file = if queued_enabled && parameter_mode {
        if let Some(root) = std::env::var_os("VOXY_RAY_WORLD_NARROW_CAPTURE") {
            Some(
                std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(std::path::PathBuf::from(root).join("queued-parameters.bin"))?,
            )
        } else {
            None
        }
    } else {
        None
    };
    let mut parameter_cases = 0usize;
    let mut states = [0u64; 4];
    let mut combined = [0u64; 3];
    let mut exact_seen = std::collections::BTreeSet::new();
    let mut reasons = [0u64; 64];
    let mut max_error = [0f64; 3];
    let mut low_results = 0u64;
    for (batch_id, batch) in rays.chunks(BATCH).enumerate() {
        queue.write_buffer(&input, 0, bytemuck::cast_slice(batch));
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
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &input,
                        offset: 0,
                        size: std::num::NonZeroU64::new(batch.len() as u64 * 80),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: vertices.clone(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: indices.clone(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: instances.clone(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: scene
                        .archive_candidate_instance_layout()
                        .ok_or("missing layout")?,
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: exact_job.queue.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.clear_buffer(&exact_job.queue, 0, Some(4));
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups((batch.len() as u32).div_ceil(64), 1, 1);
        }
        if queued_enabled {
            exact_job.encode(device, scene, &input, batch.len(), &mut encoder)?;
        }
        let bytes = batch.len() as u64 * 32;
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, bytes);
        queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback
            .slice(..bytes)
            .map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        let mut batch_pending = std::collections::BTreeSet::new();
        {
            let data = readback.slice(..bytes).get_mapped_range()?;
            if let Some(file) = &mut raw_file {
                use std::io::Write;
                file.write_all(&data)?;
            }
            for (local, record) in bytemuck::cast_slice::<u8, [u32; 8]>(&data)
                .iter()
                .enumerate()
            {
                let state = record[0] as usize;
                if state >= 4 {
                    return Err("invalid world narrow state".into());
                }
                states[state] += 1;
                if state == 2 {
                    batch_pending.insert(local);
                } else if state < 2 {
                    combined[state] += 1;
                }

                if state == 2 {
                    if record[7] >= 64 {
                        return Err("unknown fallback reason".into());
                    }
                    reasons[record[7] as usize] += 1;
                }
                if let Some(expected) = &expected_states {
                    if record[0] != expected[batch_id * BATCH + local] {
                        return Err(format!(
                            "boundary state mismatch ray={} actual={} expected={}",
                            batch_id * BATCH + local,
                            record[0],
                            expected[batch_id * BATCH + local]
                        )
                        .into());
                    }
                }
                let reference = references[batch_id * BATCH + local];
                if state == 2 {
                    if let Some(file) = &mut fallback_file {
                        use std::io::Write;
                        let mut case = failure_case(
                            meshes,
                            &rays[batch_id * BATCH + local],
                            record,
                            reference.unwrap_or([0.; 3]),
                        );
                        case["global_ray"] = serde_json::json!(batch_id * BATCH + local);
                        case["f64_reference_defined"] = serde_json::json!(reference.is_some());
                        serde_json::to_writer(&mut *file, &case)?;
                        file.write_all(b"\n")?;
                    }
                }

                if state == 3 {
                    return Err(format!(
                        "missing native candidate ray={}",
                        batch_id * BATCH + local
                    )
                    .into());
                }
                if state == 0
                    && reference.is_some_and(|[t, u, v]| {
                        t >= 0. && t <= 1. && u >= 0. && v >= 0. && u + v <= 1.
                    })
                {
                    return Err(format!(
                        "certified miss of centroid ray={}",
                        batch_id * BATCH + local
                    )
                    .into());
                }
                if state == 1 {
                    let expected = reference.ok_or("certified hit without finite reference")?;
                    for k in 0..3 {
                        let hi = f32::from_bits(record[1 + k * 2]);
                        let lo = f32::from_bits(record[2 + k * 2]);
                        if lo != 0. {
                            low_results += 1;
                        }
                        let actual = f64::from(hi) + f64::from(lo);
                        let error = (actual - expected[k]).abs();
                        max_error[k] = max_error[k].max(error);
                        if !actual.is_finite() || error > 2e-10 * (1. + expected[k].abs()) {
                            let case = failure_case(
                                meshes,
                                &rays[batch_id * BATCH + local],
                                record,
                                expected,
                            );
                            if let Some(root) = std::env::var_os("VOXY_RAY_WORLD_NARROW_CAPTURE") {
                                std::fs::write(
                                    std::path::PathBuf::from(root).join("failure.json"),
                                    serde_json::to_vec_pretty(&case)?,
                                )?;
                            }
                            return Err(format!("compensated result differs from f64 reference ray={} component={k} actual={actual} expected={}",batch_id*BATCH+local,expected[k]).into());
                        }
                    }
                }
            }
        }
        readback.unmap();
        if queued_enabled {
            let exact = exact_job.read(device, batch.len())?;
            if exact.len() != batch_pending.len() {
                return Err("GPU work queue omitted unresolved records".into());
            }
            for (local, record) in exact {
                if let Some(file) = &mut queued_file {
                    use std::io::Write;
                    let raw = [
                        batch_id as u32,
                        local,
                        record[0],
                        record[1],
                        record[2],
                        record[3],
                        record[4],
                        record[5],
                        record[6],
                        record[7],
                    ];
                    file.write_all(bytemuck::bytes_of(&raw))?;
                }

                if !batch_pending.remove(&(local as usize)) {
                    return Err("unexpected exact queued ID".into());
                }
                let id = (batch_id * BATCH + local as usize) as u64;
                let ray = &batch[local as usize];
                if record[4] != id as u32
                    || record[5] != (id >> 32) as u32
                    || record[6] != ray[16]
                    || record[7] != ray[17]
                {
                    return Err("queued exact identity differs from fast input".into());
                }
                let expected = queued_gold
                    .get(&id)
                    .ok_or("queued case has no exact oracle")?;
                if (record[0] == 1) != *expected || !exact_seen.insert(id) {
                    return Err(format!(
                        "queued exact decision mismatch id={id} record={record:?} oracle={expected}"
                    )
                    .into());
                }
                if exact_job.parameters {
                    if let Some(file) = &mut parameter_file {
                        use std::io::Write;
                        let mut raw = [0u32; 18];
                        raw[0] = batch_id as u32;
                        raw[1] = local;
                        raw[2..].copy_from_slice(&record);
                        file.write_all(bytemuck::bytes_of(&raw))?;
                    }
                    match &parameter_gold[&id] {
                        Some((words, mask)) => {
                            if record[14] != 1 || record[8..14] != words[..] || record[15] != *mask
                            {
                                return Err(format!(
                                    "queued rounded parameter mismatch id={id} record={record:?}"
                                )
                                .into());
                            }
                            parameter_cases += 1;
                        }
                        None => {
                            if record[14] != 0 {
                                return Err("unexpected queued parameter availability".into());
                            }
                        }
                    }
                }
                combined[record[0] as usize] += 1;
            }
        }
    }
    if queued_enabled {
        if exact_seen.len() != queued_gold.len() {
            return Err("not every exact oracle case was reached".into());
        }
        if combined.iter().sum::<u64>() != rays.len() as u64 {
            return Err("incomplete coupled GPU coverage".into());
        }
        if parameter_mode && parameter_cases as u64 != combined[1] - states[1] {
            return Err("queued hit lacks parameters".into());
        }
        println!(
            "WORLD COUPLED GPU DECISIONS PASS rays={} combined_miss_hit_unresolved={combined:?} exact_cases={} rounded_parameter_cases={parameter_cases} gpu_compaction=true cpu_selection=false gold_uploaded=false primitive_parameter_mode={parameter_mode} nearest_material_qualified=false fps_qualified=false",
            rays.len(),
            exact_seen.len()
        );
    }
    if let Some(file) = raw_file {
        file.sync_all()?;
    }
    if let Some(file) = fallback_file {
        file.sync_all()?;
    }
    if let Some(file) = queued_file {
        file.sync_all()?;
    }
    if let Some(file) = parameter_file {
        file.sync_all()?;
    }
    println!(
        "WORLD NARROW DIAGNOSTIC rays={} states={states:?} fallback_reasons={reasons:?} explicit_boundary_cases={} max_f64_reference_error={max_error:?} nonzero_low_components={low_results} boundary_fallback_complete=false universal_precision_qualified=false nearest_material_qualified=false fps_qualified=false",
        rays.len(),
        expected_states.is_some()
    );
    Ok(())
}
