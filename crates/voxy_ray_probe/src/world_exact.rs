//! Source-query diagnostics, including certificate/exact split passes. Gold never enters GPU inputs.
use std::path::Path;
use voxy_render::archive_scene::ArchiveMeshSource;
pub(super) fn probe(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &voxy_render::RayScene,
    meshes: &[ArchiveMeshSource],
    cases_path: &Path,
    truth_path: &Path,
    source_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let cases: Vec<serde_json::Value> = std::fs::read_to_string(cases_path)?
        .lines()
        .filter(|l| !l.is_empty())
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let truth: serde_json::Value = serde_json::from_slice(&std::fs::read(truth_path)?)?;
    let gold: std::collections::BTreeMap<u64, bool> = truth["rows"]
        .as_array()
        .ok_or("missing oracle rows")?
        .iter()
        .map(|row| {
            Ok((
                row["global_ray"].as_u64().ok_or("missing oracle ID")?,
                row["exact_hit"].as_bool().ok_or("missing exact outcome")?,
            ))
        })
        .collect::<Result<_, Box<dyn std::error::Error>>>()?;
    let nearest = std::env::var("VOXY_RAY_WORLD_EXACT_NEAREST").as_deref() == Ok("1");
    let source_order_probe =
        std::env::var("VOXY_RAY_WORLD_INTERVAL_PROBE").as_deref() == Ok("source_order");
    let order_probe = source_order_probe
        || std::env::var("VOXY_RAY_WORLD_INTERVAL_PROBE").as_deref() == Ok("order");
    let interval_probe =
        order_probe || std::env::var("VOXY_RAY_WORLD_INTERVAL_PROBE").as_deref() == Ok("1");
    if interval_probe && nearest {
        return Err("interval target audit and world nearest are distinct modes".into());
    }
    let visibility = nearest && std::env::var("VOXY_RAY_WORLD_VISIBILITY").as_deref() == Ok("1");
    let nearest_gate = nearest
        && !visibility
        && std::env::var("VOXY_RAY_WORLD_NEAREST_GATE").as_deref() == Ok("1");
    let split_visibility =
        visibility && std::env::var("VOXY_RAY_WORLD_VISIBILITY_SPLIT").as_deref() == Ok("1");
    let bounded_visibility = nearest_gate
        || split_visibility
        || visibility && std::env::var("VOXY_RAY_WORLD_VISIBILITY_BOUNDED").as_deref() == Ok("1");
    let visibility_certificate =
        visibility && std::env::var("VOXY_RAY_WORLD_VISIBILITY_CERTIFICATE").as_deref() != Ok("0");
    let interval_reject = nearest
        && match std::env::var("VOXY_RAY_WORLD_INTERVAL_REJECT")
            .ok()
            .as_deref()
        {
            None | Some("1") => true,
            Some("0") => false,
            _ => return Err("interval rejection flag requires 0 or 1".into()),
        };
    let interval_nodes = nearest
        && match std::env::var("VOXY_RAY_WORLD_INTERVAL_NODES")
            .ok()
            .as_deref()
        {
            None | Some("1") => true,
            Some("0") => false,
            _ => return Err("interval node flag requires 0 or 1".into()),
        };
    let interval_order = nearest
        && interval_reject
        && std::env::var("VOXY_RAY_WORLD_INTERVAL_ORDER").as_deref() == Ok("1");
    let interval_mask = u32::from(interval_reject)
        | (u32::from(interval_nodes) << 1)
        | (u32::from(interval_order) << 2)
        | (u32::from(visibility_certificate) << 4);
    let interval_stats =
        nearest && std::env::var("VOXY_RAY_WORLD_INTERVAL_STATS").as_deref() == Ok("1");
    if (nearest_gate || split_visibility) && interval_stats {
        return Err("split visibility diagnostic uses the production 20-word output".into());
    }
    let material_admission =
        nearest && std::env::var("VOXY_RAY_WORLD_MATERIALS").as_deref() == Ok("1");
    let parameters = !visibility
        && ((nearest || interval_probe)
            || std::env::var("VOXY_RAY_WORLD_EXACT_PARAMETERS").as_deref() == Ok("1"));
    let hit_uv = std::env::var("VOXY_RAY_WORLD_EXACT_UV").as_deref() == Ok("1");
    if hit_uv && (!nearest || visibility) {
        return Err("exact UV output requires original-source nearest mode".into());
    }
    let words_per_record = if hit_uv {
        if interval_stats { 32 } else { 28 }
    } else if interval_stats {
        24usize
    } else if nearest || interval_probe {
        20usize
    } else if parameters {
        16usize
    } else {
        8
    };
    let mut parameter_gold = std::collections::BTreeMap::<u64, Option<(Vec<u32>, u32)>>::new();
    if parameters {
        for row in truth["rows"].as_array().ok_or("missing parameter rows")? {
            let id = row["global_ray"].as_u64().ok_or("parameter ID")?;
            if row.get("parameter_words").is_none() {
                return Err("parameter mode requires rounded rational gold".into());
            }
            let expected = if let Some(values) = row["parameter_words"].as_array() {
                let words = values
                    .iter()
                    .map(|v| Ok(u32::try_from(v.as_u64().ok_or("parameter word")?)?))
                    .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
                if words.len() != 6 {
                    return Err("parameter gold ABI".into());
                }
                Some((
                    words,
                    u32::try_from(row["residual_mask"].as_u64().ok_or("residual mask")?)?,
                ))
            } else {
                None
            };
            parameter_gold.insert(id, expected);
        }
    }
    let mut rays = Vec::<[u32; 20]>::new();
    let mut ids = Vec::new();
    for case in &cases {
        let values = case["ray_words"].as_array().ok_or("missing ray words")?;
        if values.len() != 20 {
            return Err("invalid ray ABI".into());
        }
        let mut ray = [0u32; 20];
        for (out, v) in ray.iter_mut().zip(values) {
            *out = u32::try_from(v.as_u64().ok_or("invalid ray word")?)?;
        }
        if !nearest {
            let mut slot = ray[16] as usize;
            let mut source = None;
            for mesh in meshes {
                if slot < mesh.instances.len() {
                    source = Some((mesh, slot));
                    break;
                }
                slot -= mesh.instances.len();
            }
            let (mesh, slot) = source.ok_or("invalid source instance")?;
            let primitive = ray[17] as usize;
            if primitive >= mesh.indices.len() / 3 {
                return Err("invalid source primitive".into());
            }
            let world: [[f32; 4]; 4] = serde_json::from_value(case["world"].clone())?;
            let local: [[f32; 3]; 3] = serde_json::from_value(case["local_triangle"].clone())?;
            if bytemuck::bytes_of(&world) != bytemuck::bytes_of(&mesh.instances[slot].world) {
                return Err("oracle source transform differs from native scene".into());
            }
            for k in 0..3 {
                let actual = mesh.vertices[mesh.indices[primitive * 3 + k] as usize].position;
                if bytemuck::bytes_of(&actual) != bytemuck::bytes_of(&local[k]) {
                    return Err("oracle source vertices differ from native scene".into());
                }
            }
        } else {
            ray[16] = 0;
            ray[17] = 0;
        }
        let id = case["global_ray"].as_u64().ok_or("missing case ID")?;
        if !gold.contains_key(&id) {
            return Err("case has no exact oracle".into());
        }
        ray[18] = id as u32;
        ray[19] = (id >> 32) as u32;
        ids.push(id);
        rays.push(ray);
    }
    if ids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != ids.len()
    {
        return Err("duplicate case IDs".into());
    }
    if rays.is_empty() || rays.len() != gold.len() {
        return Err("incomplete exact case/oracle coverage".into());
    }
    let capacity = rays.len().min(if bounded_visibility { 65536 } else { 16 });
    let input = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bounded exact source rays"),
        size: capacity as u64 * 80,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let size = capacity as u64 * words_per_record as u64 * 4;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("exact decisions"),
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let timing = std::env::var("VOXY_RAY_WORLD_EXACT_TIMING").as_deref() == Ok("1");
    if timing && !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
        return Err("missing timestamp feature".into());
    }
    let stage_timing = timing
        && nearest_gate
        && std::env::var("VOXY_RAY_WORLD_STAGE_TIMING").as_deref() == Ok("1");
    let query_count = if stage_timing { 8 } else { 2 };
    let timing_bytes = u64::from(query_count) * 8;
    let timestamps = timing.then(|| {
        device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("exact dispatch GPU duration"),
            ty: wgpu::QueryType::Timestamp,
            count: query_count,
        })
    });
    let resolve = timing.then(|| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("exact dispatch timestamps"),
            size: timing_bytes,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    });
    let time_readback = timing.then(|| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("completed exact timestamp readback"),
            size: timing_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        })
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let exact_group: u32 = std::env::var("VOXY_RAY_WORLD_GROUP")
        .ok()
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(1);
    let worker_limit: u32 = std::env::var("VOXY_RAY_WORLD_WORKERS")
        .ok()
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(16);
    if exact_group == 0
        || worker_limit == 0
        || exact_group > device.limits().max_compute_invocations_per_workgroup
        || exact_group > device.limits().max_compute_workgroup_size_x
    {
        return Err("invalid exact worker/group admission".into());
    }
    if !nearest_gate && (exact_group != 1 || worker_limit != 16) {
        return Err("exact worker tuning requires deferred nearest mode".into());
    }
    let exact_workers = capacity.min(if nearest_gate {
        worker_limit as usize
    } else {
        16
    });
    let scratch_bytes = exact_workers as u64 * 512 * 128 * 4;
    if scratch_bytes > device.limits().max_storage_buffer_binding_size {
        return Err("exact scratch exceeds binding limit".into());
    }
    let poison = std::env::var("VOXY_RAY_WORLD_EXACT_POISON").as_deref() == Ok("1");
    let scratch = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bounded per-ray integer workspace"),
        size: scratch_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut source = format!(
        "{}\n{}\n{}\n{}\n{}",
        if nearest_gate
            || split_visibility
            || std::env::var("VOXY_RAY_WORLD_DEFERRED_INPUT").as_deref() == Ok("1")
        {
            include_str!("../../voxy_render/src/world_ray_deferred_input.wgsl")
        } else {
            include_str!("../../voxy_render/src/world_ray_input.wgsl")
        },
        include_str!("../../voxy_render/src/world_ray_source_bindings.wgsl"),
        include_str!("../../voxy_render/src/world_exact_integer.wgsl"),
        include_str!("../../voxy_render/src/world_exact_ratio.wgsl"),
        include_str!("../../voxy_render/src/world_exact.wgsl")
    );
    if nearest || interval_probe {
        source = format!(
            "enable wgpu_ray_query;\n{source}\n{}\n{}\n{}",
            include_str!("../../voxy_render/src/archive_projection_pair.wgsl"),
            include_str!("../../voxy_render/src/world_triangle_pair.wgsl"),
            [
                include_str!("../../voxy_render/src/world_ray_bvh_interval.wgsl"),
                include_str!("../../voxy_render/src/world_source_certificate.wgsl"),
                include_str!("../../voxy_render/src/world_visibility_common.wgsl"),
                include_str!("../../voxy_render/src/world_exact_history_position.wgsl"),
                include_str!("../../voxy_render/src/world_nearest.wgsl"),
                include_str!("../../voxy_render/src/world_visibility.wgsl")
            ]
            .concat()
        );
    }
    if nearest_gate {
        source.push_str(include_str!(
            "../../voxy_render/src/world_nearest_deferred_exact.wgsl"
        ));
    }
    if split_visibility {
        source.push_str(include_str!(
            "../../voxy_render/src/world_visibility_deferred_exact.wgsl"
        ));
    }
    let material_words = if nearest {
        super::world_nearest_material::load(source_root, meshes, material_admission)?
    } else {
        vec![0, 0, 24, 0]
    };
    let can_bound = voxy_render::WorldRayQueryJob::packed_material_table_known(&material_words)
        && std::env::var("VOXY_RAY_WORLD_BOUND_WALK").as_deref() != Ok("0");
    let previous_seed =
        nearest_gate && std::env::var("VOXY_RAY_WORLD_PREVIOUS_SEED").as_deref() == Ok("1");
    let native_seed =
        nearest_gate && std::env::var("VOXY_RAY_WORLD_NATIVE_SEED").as_deref() == Ok("1");
    let fast_pipeline = if nearest_gate || split_visibility {
        let fast = [if native_seed {"enable wgpu_ray_query;"}else{""},include_str!("../../voxy_render/src/world_ray_deferred_input.wgsl"),
            include_str!("../../voxy_render/src/world_ray_source_bindings.wgsl"),
            include_str!("../../voxy_render/src/archive_projection_pair.wgsl"),
            include_str!("../../voxy_render/src/world_triangle_pair.wgsl"),
            include_str!("../../voxy_render/src/world_ray_bvh_interval.wgsl"),
            "fn nearest_prepare_reciprocal() {} fn nearest_node_miss_cached(base:u32,ray:u32,far:f32)->bool {return nearest_node_miss_before(base,ray,far);}",
            include_str!("../../voxy_render/src/world_source_certificate.wgsl"),include_str!("../../voxy_render/src/world_visibility_common.wgsl"),
            include_str!("../../voxy_render/src/world_visibility_fast.wgsl"),
            if native_seed {include_str!("../../voxy_render/src/world_visibility_native_seed.wgsl")}else{"fn visibility_seed(ray:u32) {}"}].join("\n");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(fast.into()),
        });
        Some(
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: None,
                module: &shader,
                entry_point: Some("main_visibility_fast"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[
                        (
                            "WORLD_FAST_DEFER_NONCLEAR",
                            if nearest_gate { 1. } else { 0. },
                        ),
                        ("WORLD_FAST_STRIDE", if hit_uv { 7. } else { 5. }),
                        ("WORLD_FAST_CAN_BOUND", if can_bound { 1. } else { 0. }),
                        (
                            "WORLD_FAST_PREVIOUS_SEED",
                            if previous_seed { 1. } else { 0. },
                        ),
                    ],
                    ..Default::default()
                },
                cache: None,
            }),
        )
    } else {
        None
    };
    // Paired qualification changes arithmetic scheduling only, never ray words.
    if std::env::var("VOXY_RAY_WORLD_CANCEL_RATIO").as_deref() == Ok("0") {
        source = source.replace(
            "const EX_CANCEL_RATIO:bool=true;",
            "const EX_CANCEL_RATIO:bool=false;",
        );
    }
    if std::env::var("VOXY_RAY_WORLD_NORMALIZE_CERTIFIED").as_deref() == Ok("0") {
        source = source.replace(
            "const EX_NORMALIZE_CERTIFIED:bool=true;",
            "const EX_NORMALIZE_CERTIFIED:bool=false;",
        );
    }
    if std::env::var("VOXY_RAY_WORLD_CERTIFIED_TARGET").as_deref() == Ok("1") {
        if nearest || interval_probe {
            return Err("certified target audit requires primitive parameter mode".into());
        }
        source = source.replace(
            " let decision=ex_triangle(a,b,c,d);",
            " let decision=ex_triangle_certified(a,b,c,d);",
        );
    }
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("integer-only closed triangle decision"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let exact_constants = [("WORLD_EXACT_GROUP", f64::from(exact_group))];
    let winner_pipeline = nearest_gate.then(|| {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("isolated exact certified winner"),
            layout: None,
            module: &module,
            entry_point: Some(if hit_uv {
                "main_nearest_uv_winner"
            } else {
                "main_nearest_winner"
            }),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &exact_constants,
                ..Default::default()
            },
            cache: None,
        })
    });
    let candidates_pipeline = nearest_gate.then(|| {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("isolated bounded nearest candidates"),
            layout: None,
            module: &module,
            entry_point: Some(if hit_uv {
                "main_nearest_uv_candidates"
            } else {
                "main_nearest_candidates"
            }),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &exact_constants,
                ..Default::default()
            },
            cache: None,
        })
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &module,
        entry_point: Some(if nearest_gate {
            if hit_uv {
                "main_nearest_uv_deferred"
            } else {
                "main_nearest_deferred"
            }
        } else if split_visibility {
            "main_visibility_deferred_exact"
        } else if source_order_probe {
            "main_source_order_interval_probe"
        } else if order_probe {
            "main_order_interval_probe"
        } else if interval_probe {
            "main_interval_probe"
        } else if visibility {
            if bounded_visibility {
                if interval_stats {
                    "main_visibility_bounded_stats"
                } else {
                    "main_visibility_bounded"
                }
            } else if interval_stats {
                "main_visibility_stats"
            } else {
                "main_visibility"
            }
        } else if hit_uv {
            if interval_stats {
                "main_nearest_uv_stats"
            } else {
                "main_nearest_uv"
            }
        } else if interval_stats {
            "main_nearest_stats"
        } else if nearest {
            "main_nearest"
        } else if parameters {
            "main_parameters"
        } else {
            "main"
        }),
        compilation_options: wgpu::PipelineCompilationOptions {
            constants: if nearest_gate { &exact_constants } else { &[] },
            ..Default::default()
        },
        cache: None,
    });
    let [vertices, indices, instances] = scene
        .archive_candidate_source_bindings()
        .ok_or("missing exact sources")?;
    use wgpu::util::DeviceExt;
    let nearest_buffers = if nearest {
        let layout = scene
            .archive_candidate_layout()
            .ok_or("missing mesh layout")?;
        if layout.len() > 2048 || layout.len() != meshes.len() {
            return Err("nearest mesh count exceeds uniform capacity".into());
        }
        let (mut counts, mut active) = scene
            .archive_exact_source_metadata()
            .ok_or("missing same-owner exact metadata")?;
        counts[4095][3] = interval_mask;
        let material_offset = active.len() as u64 * 4;
        active.extend_from_slice(&material_words);
        if active.len() as u64 * 4 > device.limits().max_storage_buffer_binding_size {
            return Err("source hierarchy exceeds binding capacity".into());
        }
        let buffers = (
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("source primitive counts"),
                contents: bytemuck::cast_slice(&counts),
                usage: wgpu::BufferUsages::UNIFORM,
            }),
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("same-owner active masks and source hierarchy"),
                contents: bytemuck::cast_slice(&active),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            }),
        );
        if material_admission {
            super::world_nearest_material::verify(
                device,
                queue,
                &buffers.1,
                material_offset,
                &material_words,
            )?;
        }
        Some(buffers)
    } else {
        None
    };
    let mut records = Vec::<u32>::with_capacity(rays.len() * words_per_record);
    let mut gpu_batch_ns = Vec::<f64>::new();
    let mut gpu_stage_batch_ns = Vec::<[f64; 4]>::new();
    let mut gpu_stage_batch_ticks = Vec::<Vec<u64>>::new();
    let mut gpu_batch_ticks = Vec::<[u64; 2]>::new();
    let mut result_batch_ms = Vec::<f64>::new();
    let dispatch_start = std::time::Instant::now();
    for batch in rays.chunks(capacity) {
        let mut result_start = std::time::Instant::now();
        if poison {
            queue.write_buffer(
                &scratch,
                0,
                bytemuck::cast_slice(&vec![0xdeadbeefu32; scratch_bytes as usize / 4]),
            );
        }
        queue.write_buffer(&input, 0, bytemuck::cast_slice(batch));
        let mut entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &input,
                    offset: 0,
                    size: std::num::NonZeroU64::new(batch.len() as u64 * 80),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: vertices.clone(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: indices.clone(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: instances.clone(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: scene
                    .archive_candidate_instance_layout()
                    .ok_or("missing layout")?,
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: output.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: scratch.as_entire_binding(),
            },
        ];
        if let Some((counts, active)) = &nearest_buffers {
            entries.extend([
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: scene.binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: counts.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: active.as_entire_binding(),
                },
            ]);
        }
        if visibility {
            entries.retain(|entry| entry.binding != 8);
        }
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let fast_group = fast_pipeline.as_ref().map(|pipeline| {
            let entries: Vec<_> = entries
                .iter()
                .filter(|entry| entry.binding != 6 && (entry.binding != 8 || native_seed))
                .cloned()
                .collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &entries,
            })
        });
        let winner_group = winner_pipeline.as_ref().map(|pipeline| {
            let entries: Vec<_> = entries
                .iter()
                .filter(|entry| entry.binding != 8)
                .cloned()
                .collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &entries,
            })
        });
        let candidates_group = candidates_pipeline.as_ref().map(|pipeline| {
            let entries: Vec<_> = entries
                .iter()
                .filter(|entry| entry.binding != 8)
                .cloned()
                .collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &entries,
            })
        });
        let exact_dispatch = (batch.len().min(exact_workers as usize) as u32).div_ceil(exact_group);
        if nearest_gate && std::env::var("VOXY_RAY_WORLD_REPLAY_WARMUP").as_deref() == Ok("1") {
            // Populate real prior hit IDs on GPU, in both A/B modes. No gold,
            // CPU-selected primitive or cached distance enters the timed query.
            let mut warm = device.create_command_encoder(&Default::default());
            warm.clear_buffer(&input, 28, Some(4));
            warm.clear_buffer(&input, 44, Some(4));
            for (p, g, n, reset) in [
                (
                    fast_pipeline.as_ref(),
                    fast_group.as_ref(),
                    (batch.len() as u32).div_ceil(64),
                    false,
                ),
                (
                    winner_pipeline.as_ref(),
                    winner_group.as_ref(),
                    exact_dispatch,
                    true,
                ),
                (
                    candidates_pipeline.as_ref(),
                    candidates_group.as_ref(),
                    exact_dispatch,
                    true,
                ),
                (Some(&pipeline), Some(&group), exact_dispatch, false),
            ] {
                if let (Some(p), Some(g)) = (p, g) {
                    {
                        let mut pass = warm.begin_compute_pass(&Default::default());
                        pass.set_pipeline(p);
                        pass.set_bind_group(0, g, &[]);
                        pass.dispatch_workgroups(n, 1, 1);
                    }
                    if reset {
                        warm.clear_buffer(&input, 44, Some(4));
                    }
                }
            }
            let submission = queue.submit([warm.finish()]);
            device.poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })?;
            result_start = std::time::Instant::now();
        }
        let bytes = batch.len() as u64 * words_per_record as u64 * 4;
        let mut encoder = device.create_command_encoder(&Default::default());
        if let (Some(pipeline), Some(group)) = (&fast_pipeline, &fast_group) {
            encoder.clear_buffer(&input, 28, Some(4));
            encoder.clear_buffer(&input, 44, Some(4));
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                timestamp_writes: timestamps
                    .as_ref()
                    .map(|set| wgpu::ComputePassTimestampWrites {
                        query_set: set,
                        beginning_of_pass_write_index: Some(0),
                        end_of_pass_write_index: if stage_timing { Some(1) } else { None },
                    }),
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups((batch.len() as u32).div_ceil(64), 1, 1);
        }
        if let (Some(pipeline), Some(group)) = (&winner_pipeline, &winner_group) {
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    timestamp_writes: if stage_timing {
                        timestamps
                            .as_ref()
                            .map(|set| wgpu::ComputePassTimestampWrites {
                                query_set: set,
                                beginning_of_pass_write_index: Some(2),
                                end_of_pass_write_index: Some(3),
                            })
                    } else {
                        None
                    },
                    ..Default::default()
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, group, &[]);
                pass.dispatch_workgroups(exact_dispatch, 1, 1);
            }
            encoder.clear_buffer(&input, 44, Some(4));
        }
        if let (Some(pipeline), Some(group)) = (&candidates_pipeline, &candidates_group) {
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    timestamp_writes: if stage_timing {
                        timestamps
                            .as_ref()
                            .map(|set| wgpu::ComputePassTimestampWrites {
                                query_set: set,
                                beginning_of_pass_write_index: Some(4),
                                end_of_pass_write_index: Some(5),
                            })
                    } else {
                        None
                    },
                    ..Default::default()
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, group, &[]);
                pass.dispatch_workgroups(exact_dispatch, 1, 1);
            }
            encoder.clear_buffer(&input, 44, Some(4));
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                timestamp_writes: timestamps
                    .as_ref()
                    .map(|set| wgpu::ComputePassTimestampWrites {
                        query_set: set,
                        beginning_of_pass_write_index: if stage_timing {
                            Some(6)
                        } else if nearest_gate || split_visibility {
                            None
                        } else {
                            Some(0)
                        },
                        end_of_pass_write_index: Some(if stage_timing { 7 } else { 1 }),
                    }),
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(
                if nearest_gate {
                    exact_dispatch
                } else if bounded_visibility {
                    batch.len().min(16) as u32
                } else {
                    batch.len() as u32
                },
                1,
                1,
            );
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, bytes);
        queue.submit([encoder.finish()]);
        let mapped_bytes = bytes;
        let (tx, rx) = std::sync::mpsc::channel();
        readback
            .slice(..mapped_bytes)
            .map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
        device.poll(wgpu::PollType::wait_indefinitely())?;
        rx.recv()??;
        {
            let data = readback.slice(..mapped_bytes).get_mapped_range()?;
            records.extend_from_slice(bytemuck::cast_slice(&data[..bytes as usize]));
        }
        readback.unmap();
        result_batch_ms.push(result_start.elapsed().as_secs_f64() * 1000.);
        // Metal can snapshot query data while encoding a resolve. Fence the
        // producing submission first, then encode and fence a separate resolve.
        if let (Some(set), Some(resolve), Some(read)) = (&timestamps, &resolve, &time_readback) {
            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.resolve_query_set(set, 0..query_count, resolve, 0);
            encoder.copy_buffer_to_buffer(resolve, 0, read, 0, timing_bytes);
            queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            device.poll(wgpu::PollType::wait_indefinitely())?;
            rx.recv()??;
            let data = read.slice(..).get_mapped_range()?;
            let ticks: Vec<u64> = data
                .chunks_exact(8)
                .map(|w| u64::from_le_bytes(w.try_into().unwrap()))
                .collect();
            let begin = ticks[0];
            let end = ticks[query_count as usize - 1];
            if stage_timing {
                if ticks.iter().any(|&v| v == 0 || v == u64::MAX)
                    || ticks.windows(2).any(|v| v[1] < v[0])
                {
                    return Err("invalid per-stage GPU timestamps".into());
                }
                let period = f64::from(queue.get_timestamp_period());
                gpu_stage_batch_ns.push([
                    (ticks[1] - ticks[0]) as f64 * period,
                    (ticks[3] - ticks[2]) as f64 * period,
                    (ticks[5] - ticks[4]) as f64 * period,
                    (ticks[7] - ticks[6]) as f64 * period,
                ]);
                gpu_stage_batch_ticks.push(ticks);
            }
            if begin == 0 || end == 0 || begin == u64::MAX || end == u64::MAX {
                return Err("unset GPU timestamps".into());
            }
            if gpu_batch_ticks.last().is_some_and(|last| begin < last[1]) {
                return Err("stale or overlapping GPU timestamp batches".into());
            }
            gpu_batch_ticks.push([begin, end]);
            let elapsed = end.checked_sub(begin).ok_or("nonmonotone GPU timestamps")?;
            if elapsed == 0 {
                return Err("empty GPU timestamp interval".into());
            }
            let period = queue.get_timestamp_period();
            if !period.is_finite() || period <= 0. {
                return Err("invalid GPU timestamp period".into());
            }
            gpu_batch_ns.push(elapsed as f64 * f64::from(period));
            drop(data);
            read.unmap();
        }
    }
    let dispatch_wall_ms = dispatch_start.elapsed().as_secs_f64() * 1000.;
    if timing {
        let gpu_ms = gpu_batch_ns.iter().sum::<f64>() / 1e6;
        let result_wall_ms = result_batch_ms.iter().sum::<f64>();
        println!(
            "WORLD GPU PROFILE batches={} gpu_dispatch_ms={gpu_ms} result_wall_ms={result_wall_ms} profiling_wall_ms={dispatch_wall_ms} timestamps_resolved_after_dispatch=true frame_fps_measured=false",
            gpu_batch_ns.len()
        );
        if let Some(path) = std::env::var_os("VOXY_RAY_WORLD_EXACT_TIMING_OUTPUT") {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)?;
            serde_json::to_writer_pretty(
                &mut file,
                &serde_json::json!({"exact_group_size":exact_group,"exact_workers":exact_workers,"scratch_bytes":scratch_bytes,"gpu_query_warmed":std::env::var("VOXY_RAY_WORLD_REPLAY_WARMUP").as_deref()==Ok("1"),"stage_timing_enabled":stage_timing,"gpu_stage_names":["certificate","winner","candidates","fallback"],"gpu_stage_batch_ns":gpu_stage_batch_ns,"gpu_stage_batch_ticks":gpu_stage_batch_ticks,"gpu_batch_ns":gpu_batch_ns,"gpu_batch_ticks":gpu_batch_ticks,"result_batch_ms":result_batch_ms,"result_wall_ms":result_wall_ms,"gpu_dispatch_ms":gpu_ms,"dispatch_readback_wall_ms":dispatch_wall_ms,"source_setup_included":false,"frame_fps_measured":false}),
            )?;
            file.write_all(b"\n")?;
            file.sync_all()?;
        }
    }
    let data = bytemuck::cast_slice::<u32, u8>(&records);
    let mut states = [0u64; 3];
    let mut max_slots = 0u32;
    let mut parameter_cases = 0usize;
    if let Some(path) = std::env::var_os("VOXY_RAY_WORLD_EXACT_OUTPUT") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;
        file.write_all(&data)?;
        file.sync_all()?;
    }
    for (index, (id, record)) in ids
        .iter()
        .zip(records.chunks_exact(words_per_record))
        .enumerate()
    {
        if record[4] != *id as u32
            || record[5] != (*id >> 32) as u32
            || (!nearest && (record[6] != rays[index][16] || record[7] != rays[index][17]))
        {
            return Err(format!("exact GPU identity mismatch id={id} record={record:?}").into());
        }
        if record[2] != 2048 || record[3] == 0 || record[3] > 512 {
            return Err(format!("incomplete exact GPU result id={id} record={record:?}").into());
        }
        if nearest && !visibility {
            let expected = truth["rows"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["global_ray"].as_u64() == Some(*id))
                .ok_or("missing winner oracle")?;
            for (offset, key) in [
                (6, "winner_slot"),
                (7, "winner_primitive"),
                (17, "winner_material"),
            ] {
                if record[offset] as u64 != expected[key].as_u64().ok_or("missing winner field")? {
                    return Err(format!("nearest winner mismatch id={id} field={key} record={record:?} expected={expected}").into());
                }
            }
        }
        if material_admission && !visibility {
            let expected = truth["rows"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["global_ray"].as_u64() == Some(*id))
                .unwrap();
            if u64::from((record[18] >> 2) & 3)
                != expected["material_kind"]
                    .as_u64()
                    .ok_or("missing material kind gold")?
                || record[18] & 2 == 0
            {
                return Err(format!("material event mismatch id={id} record={record:?}").into());
            }
        }
        if interval_probe && record[16] != 0 && record[0] != 0 {
            return Err(format!("false interval rejection id={id} record={record:?}").into());
        }
        if interval_stats
            && !visibility
            && (record[20] + record[21] != record[16] || record[22] != interval_mask)
        {
            return Err("interval work accounting mismatch".into());
        }
        if visibility && interval_stats && record[20] + record[21] + record[22] != record[16] {
            return Err("visibility work accounting mismatch".into());
        }
        max_slots = max_slots.max(record[3]);
        if record[0] > 2 {
            return Err("invalid exact decision state".into());
        }
        states[record[0] as usize] += 1;
        if parameters {
            match &parameter_gold[id] {
                Some((expected, mask)) => {
                    if record[14] != 1 || record[8..14] != expected[..] || record[15] != *mask {
                        return Err(format!("GPU rounded parameters mismatch id={id} record={record:?} expected={expected:?} mask={mask}").into());
                    }
                    parameter_cases += 1;
                }
                None => {
                    if record[14] != 0 {
                        return Err("unexpected parameter availability".into());
                    }
                }
            }
        }

        let expected_state = truth["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["global_ray"].as_u64() == Some(*id))
            .and_then(|r| {
                if visibility {
                    r["visibility_state"].as_u64()
                } else {
                    r["expected_state"].as_u64()
                }
            })
            .unwrap_or(u64::from(gold[id]));
        if u64::from(record[0]) != expected_state {
            return Err(format!(
                "exact GPU decision mismatch id={id} record={record:?} oracle={}",
                gold[id]
            )
            .into());
        }
    }
    if parameters && parameter_cases as u64 != states[1] {
        return Err("hit lacks parameters".into());
    }
    println!(
        "WORLD EXACT GPU DECISION PASS cases={} states={states:?} integer_bits=2048 max_slots={max_slots} scratch_bytes={scratch_bytes} source_geometry_unmodified=true gold_uploaded=false rounded_parameter_cases={parameter_cases} primitive_parameter_mode={parameters} world_nearest_probe={nearest} opaque_visibility={visibility} visibility_certificate={visibility_certificate} exact_uv={hit_uv} source_guard_bvh={nearest} scratch_poisoned={poison} material_admission={material_admission} interval_reject={interval_reject} interval_stats={interval_stats} interval_nodes={interval_nodes} interval_probe={interval_probe} dispatch_readback_wall_ms={dispatch_wall_ms} universal_parameter_qualification=false nearest_material_qualified=false fps_qualified=false",
        rays.len()
    );
    Ok(())
}
