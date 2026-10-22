//! Replay every captured ray against the exact production DDA; diagnostic only.
use std::path::Path;
use wgpu::util::DeviceExt;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let width = args
        .windows(2)
        .find(|a| a[0] == "--workgroup")
        .map(|a| a[1].parse::<u32>().unwrap())
        .unwrap_or(64);
    assert!([32, 64, 128, 256].contains(&width));
    const REPEATS: u32 = 9;
    let persistent = args.iter().any(|a| a == "--persistent");
    let resumed = persistent || args.iter().any(|a| a == "--resumable");
    let quantum = args
        .windows(2)
        .find(|a| a[0] == "--quantum")
        .map(|a| a[1].parse::<u32>().unwrap())
        .unwrap_or(8);
    assert!((1..=64).contains(&quantum));
    let groups = args
        .windows(2)
        .find(|a| a[0] == "--groups")
        .map(|a| a[1].parse::<u32>().unwrap())
        .unwrap_or(256);
    assert!((1..=8192).contains(&groups));
    let root = Path::new(args.get(1).expect("capture directory"));
    let meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("metadata.json")).unwrap()).unwrap();
    assert_eq!(meta["schema"], "voxy-ray-chunk-v1");
    assert_eq!(meta["byte_order"], "little");
    let w = meta["viewport"][0].as_u64().unwrap() as u32;
    let h = meta["viewport"][1].as_u64().unwrap() as u32;
    let n = meta["ray_count_per_kind"].as_u64().unwrap();
    assert!(n > 0 && n <= 524288);
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: wgpu::Features::TIMESTAMP_QUERY
            | if persistent {
                wgpu::Features::SUBGROUP
            } else {
                wgpu::Features::empty()
            },
        ..Default::default()
    }))
    .unwrap();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("captured depth bits"),
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
    let bytes = std::fs::read(root.join("depth.bin")).unwrap();
    assert_eq!(bytes.len(), w as usize * h as usize * 4);
    queue.write_texture(
        texture.as_image_copy(),
        &bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w * 4),
            rows_per_image: Some(h),
        },
        texture.size(),
    );
    let input_view = texture.create_view(&Default::default());
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("native replay depth"),
        size: texture.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let depth_view = depth.create_view(&Default::default());
    let copy_shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:None,source:wgpu::ShaderSource::Wgsl(r"
@group(0) @binding(0) var input_depth:texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{let p=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));return vec4<f32>(p[i],0.,1.);}
@fragment fn fs(@builtin(position) p:vec4<f32>)->@builtin(frag_depth) f32{return textureLoad(input_depth,vec2<i32>(p.xy),0).x;}
".into())});
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
        voxy_render::archive_depth_bounds::ArchiveDepthBounds::new_hierarchy(&device, &depth, w, h)
            .unwrap();
    let source = concat!(
        include_str!("../../crates/voxy_render/src/archive_projection_pair.wgsl"),
        include_str!("../../crates/voxy_render/src/archive_ray_projection.wgsl"),
        include_str!("../../crates/voxy_render/src/archive_depth_dda_paired.wgsl"),
        include_str!("../../crates/voxy_render/src/archive_ray_interval.wgsl"),
        include_str!("../../crates/voxy_render/src/archive_depth_dda_resumable.wgsl"),
        r#"
@group(0) @binding(0) var depth:texture_2d<f32>;
@group(0) @binding(1) var<storage,read> inputs:array<DdaProjectedSegment>;
@group(0) @binding(2) var<storage,read_write> outputs:array<DepthDdaPairHit>;
@group(0) @binding(3) var tile_depth:texture_2d<f32>;
@group(0) @binding(4) var<storage,read> order:array<u32>;
fn dda_tile_range(tile:vec2<i32>,level:i32)->vec2<f32>{return textureLoad(tile_depth,tile,level).xy;}
fn dda_tile_levels()->u32{return textureNumLevels(tile_depth);}
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=arrayLength(&inputs){return;}let index=order[id.x];let s=inputs[index];var hit=precise_miss(0u);
 if s.state==1u {hit=trace_depth_dda_paired(depth,DdaVectorPair(s.begin,s.begin_low),DdaVectorPair(s.end,s.end_low));}
 outputs[index]=hit;
}
"#
    );
    let source = if persistent {
        let start = source.find("@compute @workgroup_size(64) fn main").unwrap();
        source[..start].to_owned()
            + r"
@group(0) @binding(5) var<storage,read_write> next_ray:atomic<u32>;
@compute @workgroup_size(64) fn main() {
 var cursor:DdaCursor;var index=0u;var ray_pending=false;
 loop {
  if !ray_pending {
   let position=atomicAdd(&next_ray,1u);
   if position<arrayLength(&inputs) {
    index=order[position];let s=inputs[index];
    if s.state==1u {
     cursor=dda_cursor_init(depth,DdaVectorPair(s.begin,s.begin_low),DdaVectorPair(s.end,s.end_low));ray_pending=!cursor.done;
     if !ray_pending {outputs[index]=cursor.hit;}
    }else {outputs[index]=precise_miss(0u);}
   }
  }
  if ray_pending {
   for(var step=0u;step<QUANTUMu;step++) {if cursor.done {break;}dda_cursor_step(depth,&cursor);}
   if cursor.done {outputs[index]=cursor.hit;ray_pending=false;}
  }
  if !subgroupAny(ray_pending||atomicLoad(&next_ray)<arrayLength(&inputs)) {break;}
 }
}
"
    } else if resumed {
        source.replace("hit=trace_depth_dda_paired(depth,DdaVectorPair(s.begin,s.begin_low),DdaVectorPair(s.end,s.end_low));",r"var cursor=dda_cursor_init(depth,DdaVectorPair(s.begin,s.begin_low),DdaVectorPair(s.end,s.end_low));loop {if cursor.done {break;}for(var step=0u;step<QUANTUMu;step++) {if cursor.done {break;}dda_cursor_step(depth,&cursor);}}hit=cursor.hit;")
    } else {
        source.to_owned()
    };
    let source = source.replace("QUANTUM", &quantum.to_string());
    let source = source
        .replace("image:texture_2d<f32>", "image:texture_depth_2d")
        .replace("var depth:texture_2d<f32>", "var depth:texture_depth_2d")
        .replace(
            "textureLoad(image,vec2<i32>(x,y),0).x",
            "textureLoad(image,vec2<i32>(x,y),0)",
        );
    let source = source.replace(
        "@workgroup_size(64) fn main",
        &format!("@workgroup_size({width}) fn main"),
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("exact captured ray replay"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let constants: Vec<(String, f64)> = serde_json::from_value(meta["constants"].clone()).unwrap();
    let constants: Vec<_> = constants
        .iter()
        .filter(|(k, _)| k.starts_with("ARCHIVE_DDA_"))
        .map(|(k, v)| (k.as_str(), *v))
        .collect();
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("captured original DDA"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions {
            constants: &constants,
            ..Default::default()
        },
        cache: None,
    });
    let mut reports = Vec::new();
    let make_queries = || {
        device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("diagnostic replay traversal"),
            ty: wgpu::QueryType::Timestamp,
            count: REPEATS * 2,
        })
    };
    let timing = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(REPEATS) * 16,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let timing_read = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(REPEATS) * 16,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    for kind in meta["ray_kinds"].as_array().unwrap() {
        let kind = kind.as_str().unwrap();
        let segments = std::fs::read(root.join(format!("{kind}.segments.bin"))).unwrap();
        let expected = std::fs::read(root.join(format!("{kind}.hits.bin"))).unwrap();
        let order = std::fs::read(root.join(format!("{kind}.order.bin"))).unwrap();
        assert_eq!(segments.len() as u64, n * 176);
        assert_eq!(expected.len() as u64, n * 32);
        assert_eq!(order.len() as u64, n * 4);
        let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &segments,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let ids = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &order,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: n * 32,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: n * 32,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let next = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("persistent exact ray cursor"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let pixel_bounds_view = bounds.texture.create_view(&Default::default());
        let mut entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&depth_view),
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
                resource: wgpu::BindingResource::TextureView(&pixel_bounds_view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: ids.as_entire_binding(),
            },
        ];
        if persistent {
            entries.push(wgpu::BindGroupEntry {
                binding: 5,
                resource: next.as_entire_binding(),
            });
        }
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let queries = make_queries();
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
        for repeat in 0..REPEATS {
            if persistent {
                encoder.clear_buffer(&next, 0, None);
            }
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &queries,
                    beginning_of_pass_write_index: Some(repeat * 2),
                    end_of_pass_write_index: Some(repeat * 2 + 1),
                }),
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(
                if persistent {
                    groups
                } else {
                    (n as u32).div_ceil(width)
                },
                1,
                1,
            );
        }
        encoder.resolve_query_set(&queries, 0..REPEATS * 2, &timing, 0);
        encoder.copy_buffer_to_buffer(&timing, 0, &timing_read, 0, u64::from(REPEATS) * 16);
        encoder.copy_buffer_to_buffer(&output, 0, &read, 0, n * 32);
        let batch_start = std::time::Instant::now();
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
        let completed_batch_wall_ms = batch_start.elapsed().as_secs_f64() * 1000.;
        let actual = read.slice(..).get_mapped_range().unwrap();
        let mismatches = actual
            .chunks_exact(32)
            .zip(expected.chunks_exact(32))
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(
            mismatches, 0,
            "captured original intersection words changed for {kind}"
        );
        drop(actual);
        read.unmap();
        let (tx, rx) = std::sync::mpsc::channel();
        timing_read
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let data = timing_read.slice(..).get_mapped_range().unwrap();
        let samples: Vec<f64> = data
            .chunks_exact(16)
            .map(|v| {
                let a = u64::from_le_bytes(v[..8].try_into().unwrap());
                let b = u64::from_le_bytes(v[8..].try_into().unwrap());
                (b - a) as f64 * f64::from(queue.get_timestamp_period()) / 1e6
            })
            .collect();
        drop(data);
        timing_read.unmap();
        reports.push(serde_json::json!({"kind":kind,"rays":n,"all_hit_words_bit_exact":true,"completed_batch_wall_ms":completed_batch_wall_ms,"batch_dispatches":REPEATS,"gpu_timestamps_all_positive":samples.iter().all(|v|v.is_finite()&&*v>0.),"performance_comparison_qualified":false,"gpu_traversal_ms":samples,"first_sample_cold":true}));
    }
    println!(
        "{}",
        serde_json::json!({"scope":"one complete captured chunk replayed using original GPU DDA; not full scene performance", "resumable":resumed,"persistent":persistent,"quantum":quantum,"groups":groups,"workgroup":width,"adapter":adapter.get_info().name,"full_scene_fps_proven":false,"results":reports})
    );
}
