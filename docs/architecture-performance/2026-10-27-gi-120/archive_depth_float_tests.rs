#[test]
#[cfg(target_os = "macos")]
#[ignore = "native Metal depth-copy oracle; VOXY_FLOAT_DEPTH_PROOF must be a fresh directory"]
fn native_depth_copy_preserves_every_word() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(
        std::env::var_os("VOXY_FLOAT_DEPTH_PROOF").ok_or("proof directory required")?,
    );
    std::fs::create_dir(&root)?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let (width, height) = (259, 131);
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("native raster depth words"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let copy = crate::archive_depth_bounds::ArchiveDepthBounds::new_pixel_copy(
        &device, &depth, width, height,
    )?;
    assert_eq!(copy.texture.format(), wgpu::TextureFormat::R32Float);
    assert_eq!(copy.texture.mip_level_count(), 1);
    let source = r"
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{
 let p=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));return vec4<f32>(p[i],0.5,1.);
}
@fragment fn fragment(@builtin(position) p:vec4<f32>)->@builtin(frag_depth) f32{
 let index=u32(p.x)+u32(p.y)*259u;
 if index%6u==0u{return 0.;}if index%6u==1u{return 1.;}if index%6u==2u{return bitcast<f32>(0x80000000u);}
 return bitcast<f32>(0x3f000000u|((index*16777619u)&0x007fffffu));
}
@group(0) @binding(0) var original:texture_depth_2d;
@group(0) @binding(1) var copied:texture_2d<f32>;
@group(0) @binding(2) var<storage,read_write> result:array<vec2<u32>>;
@compute @workgroup_size(8,8) fn compare(@builtin(global_invocation_id) id:vec3<u32>){
 if any(id.xy>=textureDimensions(original)){return;}
 let p=vec2<i32>(id.xy);let index=id.x+id.y*259u;
 result[index]=vec2<u32>(bitcast<u32>(textureLoad(original,p,0)),bitcast<u32>(textureLoad(copied,p,0).x));
}
";
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let raster = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
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
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[],
        }),
        multiview_mask: None,
        cache: None,
    });
    let compare = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("compare"),
        compilation_options: Default::default(),
        cache: None,
    });
    let size = u64::from(width) * u64::from(height) * 8;
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
    let depth_view = depth.create_view(&Default::default());
    let copy_view = copy.texture.create_view(&Default::default());
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &compare.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&depth_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&copy_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: result.as_entire_binding(),
            },
        ],
    });
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
            ..Default::default()
        });
        pass.set_pipeline(&raster);
        pass.draw(0..3, 0..1);
    }
    copy.encode(&mut encoder);
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&compare);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
    }
    encoder.copy_buffer_to_buffer(&result, 0, &read, 0, size);
    let submission = queue.submit([encoder.finish()]);
    device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    })?;
    let (send, receive) = std::sync::mpsc::channel();
    read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        send.send(r).unwrap();
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    receive.recv()??;
    let bytes = read.slice(..).get_mapped_range()?;
    let pairs: &[[u32; 2]] = bytemuck::cast_slice(&bytes);
    let differences = pairs.iter().filter(|p| p[0] != p[1]).count();
    let unique = pairs
        .iter()
        .map(|p| p[0])
        .collect::<std::collections::HashSet<_>>()
        .len();
    std::fs::write(
        root.join("proof.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"pixels":width*height,"different_native_words":differences,"unique_depth_values":unique,"format":"R32Float","mip_levels":1,"native_pair_blake3":blake3::hash(&bytes).to_hex().to_string(),"adapter":format!("{:?}",adapter.get_info())}),
        )?,
    )?;
    assert!(
        unique > 1000,
        "the source must contain actual varying native depth"
    );
    assert_eq!(differences, 0);
    Ok(())
}
