//! Independent exact narrow-phase replay. Reference hits never enter the GPU shader.
use super::captured_aabbs::{i32_at, load, u32_at};
use std::path::Path;
use wgpu::util::DeviceExt;
pub(super) fn probe(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    root: &Path,
    control: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    const RAY_BATCH: u32 = 65536;
    let data = load(device, root)?;
    let (w, h) = (data.width, data.height);
    let count = data.count;
    let scene = voxy_render::RayScene::from_aabbs(device, &data.boxes, 1)?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("lossless captured depth upload"),
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
    queue.write_texture(
        texture.as_image_copy(),
        &data.depth,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w * 4),
            rows_per_image: Some(h),
        },
        texture.size(),
    );
    let input_view = texture.create_view(&Default::default());
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bit-preserved native depth"),
        size: texture.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&Default::default());
    let copy_shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:None,source:wgpu::ShaderSource::Wgsl(r#"
@group(0) @binding(0) var input_depth:texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{let p=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));return vec4<f32>(p[i],0.,1.);}
@fragment fn fs(@builtin(position) p:vec4<f32>)->@builtin(frag_depth) f32{return textureLoad(input_depth,vec2<i32>(p.xy),0).x;}
"#.into())});
    let copy = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: wgpu::VertexState {
            module: &copy_shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &copy_shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[],
        }),
        multiview_mask: None,
        cache: None,
    });
    let copy_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &copy.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&input_view),
        }],
    });

    let bounds =
        voxy_render::archive_depth_bounds::ArchiveDepthBounds::new_hierarchy(device, &depth, w, h)?;
    let bounds_view = bounds.texture.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&copy);
        pass.set_bind_group(0, &copy_bind, &[]);
        pass.draw(0..3, 0..1);
    }
    bounds.encode(&mut encoder);
    scene.build(&mut encoder);
    let submission = queue.submit([encoder.finish()]);
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    })?;
    let source = concat!(
        "enable wgpu_ray_query;\n",
        include_str!("../../voxy_render/src/archive_projection_pair.wgsl"),
        include_str!("../../voxy_render/src/archive_ray_projection.wgsl"),
        include_str!("../../voxy_render/src/archive_depth_dda_paired.wgsl"),
        include_str!("../../voxy_render/src/archive_ray_interval.wgsl"),
        include_str!("../../voxy_render/src/archive_depth_dda_resumable.wgsl"),
        include_str!("captured_narrow.wgsl")
    );
    let source = source
        .replace("image:texture_2d<f32>", "image:texture_depth_2d")
        .replace(
            "textureLoad(image,vec2<i32>(x,y),0).x",
            "textureLoad(image,vec2<i32>(x,y),0)",
        );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("paired hardware narrow phase"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let constants: Vec<(String, f64)> = serde_json::from_value(data.meta["constants"].clone())?;
    let constants: Vec<_> = constants
        .iter()
        .filter(|(k, _)| k.starts_with("ARCHIVE_DDA_"))
        .map(|(k, v)| (k.as_str(), *v))
        .collect();
    let mut entries = Vec::new();
    for binding in 0..8 {
        let ty = match binding {
            0 => wgpu::BindingType::AccelerationStructure {
                vertex_return: false,
            },
            1 | 4 | 5 | 7 => wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage {
                    read_only: binding == 1 || binding == 7,
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            2 | 3 => wgpu::BindingType::Texture {
                sample_type: if binding == 2 {
                    wgpu::TextureSampleType::Depth
                } else {
                    wgpu::TextureSampleType::Float { filterable: false }
                },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            _ => wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        };
        entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty,
            count: None,
        });
    }
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("matched exact ray comparison layout"),
        entries: &entries,
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("matched exact candidate/control"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some(if control { "original_control" } else { "main" }),
        compilation_options: wgpu::PipelineCompilationOptions {
            constants: &constants,
            ..Default::default()
        },
        cache: None,
    });
    let mut reports = Vec::new();
    for capture in data.captures {
        let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("complete original segment payload"),
            contents: &capture.segments,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let order = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("same complete original ray queue"),
            contents: &capture.order,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let result_size = u64::from(count) * 32;
        let mode_size = u64::from(count) * 16;
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: result_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let modes = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: mode_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: result_size + mode_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let batch_count = count.div_ceil(RAY_BATCH);
        let timers = if device.features().contains(wgpu::Features::TIMESTAMP_QUERY)
            && batch_count * 2 <= 4096
        {
            let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("individual complete ray dispatch timings"),
                ty: wgpu::QueryType::Timestamp,
                count: batch_count * 2,
            });
            let resolve = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: u64::from(batch_count) * 16,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let read = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: u64::from(batch_count) * 16,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            Some((queries, resolve, read))
        } else {
            None
        };
        let start = std::time::Instant::now();
        let mut batches = 0;
        // Every record is processed. Bound each dispatch rather than impose a ray
        // iteration budget, omit samples, or risk one huge diagnostic GPU kernel.
        for first in (0..count).step_by(RAY_BATCH as usize) {
            let span = (count - first).min(RAY_BATCH);
            let grid: Vec<u8> = [data.tw, data.th, w, h, first, span, 0, 0]
                .into_iter()
                .flat_map(u32::to_le_bytes)
                .collect();
            let grid = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &grid,
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                        resource: wgpu::BindingResource::TextureView(&depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&bounds_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: output.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: modes.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: grid.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: order.as_entire_binding(),
                    },
                ],
            });
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    timestamp_writes: timers.as_ref().map(|(queries, _, _)| {
                        wgpu::ComputePassTimestampWrites {
                            query_set: queries,
                            beginning_of_pass_write_index: Some(batches * 2),
                            end_of_pass_write_index: Some(batches * 2 + 1),
                        }
                    }),
                    ..Default::default()
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind, &[]);
                pass.dispatch_workgroups(span.div_ceil(64), 1, 1);
            }
            queue.submit([encoder.finish()]);
            batches += 1;
        }
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&output, 0, &read, 0, result_size);
        encoder.copy_buffer_to_buffer(&modes, 0, &read, result_size, mode_size);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
        device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })?;
        rx.recv()??;
        let completed_wall_ms = start.elapsed().as_secs_f64() * 1000.;
        let bytes = read.slice(..).get_mapped_range()?;
        let mut counts = [0u64; 4];
        let (
            mut mismatches,
            mut native_hits,
            mut native_misses,
            mut candidates,
            mut tests,
            mut invalid,
            mut native_pair_divisions,
        ) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
        let mut reasons = std::collections::BTreeMap::new();
        for index in 0..count as usize {
            let actual = &bytes[index * 32..index * 32 + 32];
            let expected = &capture.hits[index * 32..index * 32 + 32];
            let mode =
                &bytes[result_size as usize + index * 16..result_size as usize + index * 16 + 16];
            let which = u32_at(mode, 0);
            if which > 3 {
                return Err("invalid narrow-phase execution mode".into());
            }
            counts[which as usize] += 1;
            candidates += u64::from(u32_at(mode, 4));
            tests += u64::from(u32_at(mode, 12));
            let reason = u32_at(mode, 8);
            if reason & 32 != 0 {
                invalid += 1;
            }
            if reason != 0 {
                *reasons.entry(reason).or_insert(0u64) += 1;
            }
            let equal = actual[..16] == expected[..16]
                && actual[20..] == expected[20..]
                && (which == 1 || actual[16..20] == expected[16..20]);
            if !equal {
                if mismatches < 8 {
                    eprintln!(
                        "NARROW_MISMATCH kind={} index={index} mode={which} reason={reason} actual={:?} expected={:?}",
                        capture.kind, actual, expected
                    );
                }
                mismatches += 1;
            }
            if which == 1 {
                native_pair_divisions += u64::from(u32_at(actual, 16));
                if i32_at(actual, 0) >= 0 {
                    native_hits += 1;
                } else {
                    native_misses += 1;
                }
            }
        }
        drop(bytes);
        read.unmap();
        let gpu_timing = if let Some((queries, resolve, timer_read)) = &timers {
            // Counter sample buffers are not output-buffer hazards. Resolve only after
            // the completed output fence, so the last encoder's samples are available.
            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.resolve_query_set(queries, 0..batch_count * 2, resolve, 0);
            encoder.copy_buffer_to_buffer(resolve, 0, timer_read, 0, u64::from(batch_count) * 16);
            queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            timer_read
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
            device.poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })?;
            rx.recv()??;
            let bytes = timer_read.slice(..).get_mapped_range()?;
            let ticks: Vec<[u64; 2]> = bytes
                .chunks_exact(16)
                .map(|b| {
                    [
                        u64::from_le_bytes(b[..8].try_into().unwrap()),
                        u64::from_le_bytes(b[8..].try_into().unwrap()),
                    ]
                })
                .collect();
            drop(bytes);
            timer_read.unmap();
            let samples: Vec<f64> = ticks
                .iter()
                .map(|t| {
                    t[1].checked_sub(t[0])
                        .map(|d| d as f64 * f64::from(queue.get_timestamp_period()) / 1e6)
                        .unwrap_or(f64::NAN)
                })
                .collect();
            if let Some(directory) = std::env::var_os("VOXY_RAY_NARROW_PROFILE_DIR") {
                let directory = Path::new(&directory);
                std::fs::create_dir_all(directory)?;
                let path = directory.join(format!(
                    "{}-{}.json",
                    capture.kind,
                    if control { "control" } else { "hardware" }
                ));
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)?;
                serde_json::to_writer_pretty(
                    &mut file,
                    &serde_json::json!({"ticks":ticks,"timestamp_period_ns":queue.get_timestamp_period(),"samples_ms":samples}),
                )?;
            }
            serde_json::json!({"samples":samples.len(),"all_intervals_positive":samples.iter().all(|v|v.is_finite()&&*v>0.),"sum_ms":samples.iter().sum::<f64>(),"workload_isolated":false})
        } else {
            serde_json::Value::Null
        };
        let report = serde_json::json!({"kind":capture.kind,"original_control":control,"gpu_timing":gpu_timing,"rays":count,"mode_counts":counts,"native_hits":native_hits,"native_misses":native_misses,"fallback_reasons":reasons,"candidate_count":candidates,"narrow_plane_tests":tests,"native_pair_divisions":native_pair_divisions,"geometric_word_mismatches":mismatches,"invalid_candidates":invalid,"batches":batches,"ray_batch_capacity":RAY_BATCH,"completed_wall_ms":completed_wall_ms,"reference_hits_uploaded":false,"original_queue_order_preserved":true,"work_count_equality_required_for_fallback":true});
        println!("HARDWARE_NARROW {report}");
        reports.push(report);
        if mismatches != 0 || invalid != 0 || (!control && (native_hits == 0 || native_misses == 0))
        {
            return Err("exact hardware narrow phase not admitted for complete snapshot".into());
        }
    }
    println!(
        "{}",
        serde_json::json!({"schema":"voxy-exact-hardware-narrow-v1","scope":"complete captured chunks, not full street or universal proof","original_control":control,"viewport":[w,h],"results":reports,"envelope_universal_proof":false,"street_hardware_connected":false,"minimum_120_presented_fps_passed":false})
    );
    Ok(())
}
