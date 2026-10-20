mod qualify_depth_mips;
use voxy_render::archive_depth_bounds as ray_bounds_owner;
#[path="../../crates/voxy_render/src/archive_frustum.rs"]
mod frustum_qualification;
#[path = "../../crates/voxy_app/examples/street_walk/materials.rs"]
mod materials;
#[path = "../../crates/voxy_app/examples/street_walk/scene.rs"]
mod scene;
use wgpu::util::DeviceExt;
// Actual GPU readback of low bits and HDR values in scanned-material transport.
fn main() {
    let instance = voxy_render::GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: (adapter.features() & wgpu::Features::TIMESTAMP_QUERY) | wgpu::Features::FLOAT32_FILTERABLE | (adapter.features() & wgpu::Features::SUBGROUP),
        ..Default::default()
    }))
    .unwrap();
    let expected = vec![
        1.0_f32 / 65535.,
        2. / 65535.,
        0.12345679,
        1.,
        1.000001,
        2.500001,
        0.8765432,
        1.,
    ];
    let source = voxy_render::archive_scene::ArchiveImageSource::Float {
        width: 2,
        height: 1,
        pixels: expected.clone(),
    };
    let texture = source.upload(&device, &queue).unwrap();
    assert_eq!(texture.format(), wgpu::TextureFormat::Rgba32Float);
    assert_eq!(texture.mip_level_count(), 2);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("float material readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 2,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = buffer.slice(..).get_mapped_range().unwrap();
    for (actual, expected) in mapped[..32].chunks_exact(4).zip(expected) {
        assert_eq!(
            u32::from_le_bytes(actual.try_into().unwrap()),
            expected.to_bits()
        );
    }
    println!(
        "GPU_FLOAT_MATERIAL adapter={} original_float32_bits_exact=true complete_mip_chain=true",
        adapter.get_info().name
    );
    drop(mapped);
    buffer.unmap();
    let panorama = voxy_render::HdrImageAsset::from_rgb(
        2,
        1,
        vec![131072., 0.125, 2., 131072., 0.125, 2.],
        voxy_render::ImageLimits::default(),
    )
    .unwrap();
    assert!(
        voxy_render::ImportedEnvironment::from_panorama(
            &device,
            &queue,
            &panorama,
            2,
            voxy_render::ImageLimits::default()
        )
        .is_err()
    );
    let environment = voxy_render::ImportedEnvironment::from_panorama_full_precision(
        &device,
        &queue,
        &panorama,
        2,
        voxy_render::ImageLimits::default(),
    )
    .unwrap();
    assert_eq!(
        environment.specular().output().format(),
        wgpu::TextureFormat::Rgba32Float
    );
    assert_eq!(
        environment.diffuse().output().format(),
        wgpu::TextureFormat::Rgba32Float
    );
    let hdr_readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("HDR beyond half range"),
        size: 512,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    environment.encode(&mut encoder);
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: environment.specular().output(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &hdr_readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(2),
            },
        },
        wgpu::Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        },
    );
    let submission = queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    hdr_readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let data = hdr_readback.slice(..).get_mapped_range().unwrap();
    for row in 0..2 {
        for pixel in 0..2 {
            let start = row * 256 + pixel * 16;
            let red = f32::from_le_bytes(data[start..start + 4].try_into().unwrap());
            assert_eq!(red.to_bits(), 131072_f32.to_bits());
        }
    }
    println!("GPU_HDR_ENVIRONMENT source_peak=131072 retained_exact=true source_clipping=false");
    drop(data);
    hdr_readback.unmap();
    qualify_packed_sampling(&device, &queue);
    qualify_stable_culling(&device, &queue);
    qualify_cpu_frustum();
    qualify_history_rejection(&device,&queue);
    qualify_depth_hierarchy(&device,&queue);
    qualify_exact_ray_bounds(&device,&queue);
}

fn qualify_packed_sampling(device: &wgpu::Device, queue: &wgpu::Queue) {
    use voxy_render::archive_scene::ArchiveImageSource;
    let (w, h) = (32, 16);
    let pixels = |offset: u32| {
        (0..w * h)
            .flat_map(|i| {
                [
                    ((i * 977 + offset) % 65536) as f32 / 65535.,
                    ((i * 919 + offset) % 65536) as f32 / 65535.,
                    ((i * 853 + offset) % 65536) as f32 / 65535.,
                    1.,
                ]
            })
            .collect::<Vec<_>>()
    };
    let originals = [pixels(1), pixels(2), pixels(3)];
    let [c, a, n] = originals.clone().map(|pixels| ArchiveImageSource::Float {
        width: w,
        height: h,
        pixels,
    });
    let packed = materials::pack_set(c, a, n).unwrap();
    let mut textures = Vec::new();
    for pixels in originals {
        textures.push(
            ArchiveImageSource::Float {
                width: w,
                height: h,
                pixels,
            }
            .upload(device, queue)
            .unwrap(),
        );
    }
    for image in packed {
        textures.push(image.upload(device, queue).unwrap());
    }
    let views: Vec<_> = textures
        .iter()
        .map(|t| t.create_view(&Default::default()))
        .collect();
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        min_filter: wgpu::FilterMode::Linear,
        mag_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        anisotropy_clamp: 16,
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        ..Default::default()
    });
    let result = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("packing mismatch count"),
        contents: &[0; 4],
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:Some("full precision packed sampler comparison"),source:wgpu::ShaderSource::Wgsl(r#"
@group(0) @binding(0) var colour:texture_2d<f32>;
@group(0) @binding(1) var arm:texture_2d<f32>;
@group(0) @binding(2) var normal:texture_2d<f32>;
@group(0) @binding(3) var packed_colour:texture_2d<f32>;
@group(0) @binding(4) var packed_normal:texture_2d<f32>;
@group(0) @binding(5) var filtering:sampler;
@group(0) @binding(6) var<storage,read_write> mismatch:atomic<u32>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let uv=vec2<f32>(f32(id.x%41u)/17.-0.3,f32(id.x%37u)/19.-0.2);
 let lod=f32(id.x%61u)/12.;
 let c=textureSampleLevel(colour,filtering,uv,lod);let a=textureSampleLevel(arm,filtering,uv,lod);let n=textureSampleLevel(normal,filtering,uv,lod);
 let pc=textureSampleLevel(packed_colour,filtering,uv,lod);let pn=textureSampleLevel(packed_normal,filtering,uv,lod);
 if any(bitcast<vec3<u32>>(c.rgb)!=bitcast<vec3<u32>>(pc.rgb))||any(bitcast<vec3<u32>>(n.rgb)!=bitcast<vec3<u32>>(pn.rgb))
   ||bitcast<u32>(a.r)!=bitcast<u32>(pc.a)||bitcast<u32>(a.g)!=bitcast<u32>(pn.a) {atomicAdd(&mismatch,1u);}
}
"#.into())});
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("packed float sampling"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let mut entries: Vec<_> = views
        .iter()
        .enumerate()
        .map(|(i, v)| wgpu::BindGroupEntry {
            binding: i as u32,
            resource: wgpu::BindingResource::TextureView(v),
        })
        .collect();
    entries.push(wgpu::BindGroupEntry {
        binding: 5,
        resource: wgpu::BindingResource::Sampler(&sampler),
    });
    entries.push(wgpu::BindGroupEntry {
        binding: 6,
        resource: result.as_entire_binding(),
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &entries,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(64, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&result, 0, &readback, 0, 4);
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
    let failures = u32::from_le_bytes(data[..4].try_into().unwrap());
    assert_eq!(failures, 0, "packed material sampling changed float32 bits");
    println!(
        "GPU_PACKED_MATERIAL filtered_samples=4096 retained_channels_bit_exact=true mismatches={failures}"
    );
}

fn qualify_stable_culling(device: &wgpu::Device, queue: &wgpu::Queue) {
    use glam::{Mat4, Vec3, Vec4};
    use voxy_render::archive_scene::ArchiveInstance;
    let instances: Vec<_> = (0..769)
        .map(|i| {
            let mut instance = ArchiveInstance::new(Mat4::from_translation(Vec3::new(
                (i % 7) as f32 - 3.,
                0.,
                0.,
            )));
            instance.world[0][1] = -0.0;
            instance.normal[2][3] = f32::from_bits(0x3e123457 + i);
            instance
        })
        .collect();
    let spheres: Vec<_> = instances
        .iter()
        .map(|i| (Vec3::new(i.world[3][0], 0., 0.), 0.05))
        .collect();
    let bytes = bytemuck::cast_slice(&instances);
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("exact instance input"),
        contents: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("exact compacted instances"),
        size: bytes.len() as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::VERTEX
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let culling =
        voxy_render::ArchiveInstanceCulling::new(device, &input, &output, &spheres, 36).unwrap();
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 20 + bytes.len() as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    for phase in 0..3 {
        let extent = match phase {
            0 => 1.,
            1 => -10.,
            _ => 100.,
        };
        let planes = [
            Vec4::new(1., 0., 0., extent),
            Vec4::new(-1., 0., 0., extent),
            Vec4::new(0., 1., 0., extent),
            Vec4::new(0., -1., 0., extent),
            Vec4::new(0., 0., 1., extent),
            Vec4::new(0., 0., -1., extent),
        ];
        let expected: Vec<_> = instances
            .iter()
            .enumerate()
            .filter_map(|(i, instance)| match phase {
                0 => (instance.world[3][0].abs() <= 1.05).then_some(i),
                1 => None,
                _ => Some(i),
            })
            .collect();
        culling.prepare(queue, planes);
        let mut encoder = device.create_command_encoder(&Default::default());
        culling.encode(&mut encoder);
        encoder.copy_buffer_to_buffer(culling.indirect(), 0, &readback, 0, 20);
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 20, bytes.len() as u64);
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
        assert_eq!(u32::from_le_bytes(data[0..4].try_into().unwrap()), 36);
        assert_eq!(
            u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize,
            expected.len()
        );
        for (destination, &original) in expected.iter().enumerate() {
            assert_eq!(
                &data[20 + destination * 112..20 + (destination + 1) * 112],
                bytemuck::bytes_of(&instances[original]),
                "instance ordering or original float32 bits changed"
            );
        }
        drop(data);
        readback.unmap();
    }
    println!(
        "GPU_INSTANCE_CULLING instances=769 phases=partial_empty_full stable_order=true payload_bits_exact=true stale_instances=false"
    );
}

fn qualify_cpu_frustum() {
    use glam::{Vec3,Vec4};
    let sets=[
        [Vec4::new(1.,0.,0.,1.),Vec4::new(-1.,0.,0.,1.),Vec4::new(0.,1.,0.,1.),Vec4::new(0.,-1.,0.,1.),Vec4::new(0.,0.,1.,1.),Vec4::new(0.,0.,-1.,1.)],
        [Vec4::new(0.31,-0.21,0.77,1.3),Vec4::new(-0.7,0.2,0.31,1.1),Vec4::new(0.2,0.99,0.13,0.7),Vec4::new(-0.1,-0.83,0.15,1.6),Vec4::new(0.03,0.01,0.99,0.1),Vec4::new(-0.01,0.02,-0.9,4.)]
    ];
    let mut spheres:Vec<_>=(0..10003).map(|i|(Vec3::new(((i*977)%65536) as f32/16000.-2.,((i*919)%65536) as f32/16000.-2.,((i*853)%65536) as f32/16000.-2.),(i%37) as f32/127.)).collect();
    for x in [1.,-1.,f32::from_bits(1_f32.to_bits()+1),f32::from_bits(1_f32.to_bits()-1),f32::NAN] {spheres.push((Vec3::new(x,0.,0.),0.));}
    let groups=frustum_qualification::ArchiveFrustumGroups::new(&spheres);let mut actual=Vec::new();
    for values in sets {
        let planes=values.map(|p|(p,p.truncate().length()));
        let expected:Vec<_>=spheres.iter().enumerate().filter_map(|(i,(c,r))|planes.iter().all(|(p,len)|p.truncate().dot(*c)+p.w>=-r*len).then_some(i)).collect();
        groups.collect(&planes,&mut actual);assert_eq!(actual,expected,"SIMD changed scalar visibility or ordering");
    }
    println!("CPU_SIMD_FRUSTUM samples={} plane_sets=2 selection_and_order_exact=true",spheres.len());
}

fn qualify_history_rejection(device:&wgpu::Device,queue:&wgpu::Queue) {
    let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:Some("actual history-surface gate"),source:wgpu::ShaderSource::Wgsl(concat!(include_str!("../../crates/voxy_render/src/archive_history.wgsl"),r#"
@group(0) @binding(0) var<storage,read_write> result:array<u32,8>;
@compute @workgroup_size(8) fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 var recovered=vec3<f32>(0.);var w=1.;var angular=0.001;
 switch id.x {case 1u:{recovered.z=1.;}case 2u:{recovered.z=0.01;}case 3u:{w=-1.;}case 4u:{w=100.;recovered.z=0.20;}case 5u:{w=100.;recovered.z=0.30;}case 6u:{recovered.x=bitcast<f32>(0x7fc00000u);}case 7u:{w=0.;}default:{}}
 result[id.x]=select(0u,1u,archive_history_surface_matches(vec3<f32>(0.),recovered,w,angular));
}
"#).into())});
    let pipeline=device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {label:None,layout:None,module:&shader,entry_point:Some("main"),compilation_options:Default::default(),cache:None});
    let data=device.create_buffer(&wgpu::BufferDescriptor {label:None,size:32,usage:wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_SRC,mapped_at_creation:false});
    let readback=device.create_buffer(&wgpu::BufferDescriptor {label:None,size:32,usage:wgpu::BufferUsages::MAP_READ|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
    let bind=device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&pipeline.get_bind_group_layout(0),entries:&[wgpu::BindGroupEntry {binding:0,resource:data.as_entire_binding()}]});
    let mut encoder=device.create_command_encoder(&Default::default());
    {let mut pass=encoder.begin_compute_pass(&Default::default());pass.set_pipeline(&pipeline);pass.set_bind_group(0,&bind,&[]);pass.dispatch_workgroups(1,1,1);}
    encoder.copy_buffer_to_buffer(&data,0,&readback,0,32);let submission=queue.submit([encoder.finish()]);let(tx,rx)=std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read,move|r|tx.send(r).unwrap());device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();rx.recv().unwrap().unwrap();
    let bytes=readback.slice(..).get_mapped_range().unwrap();let values:Vec<_>=bytes.chunks_exact(4).map(|c|u32::from_le_bytes(c.try_into().unwrap())).collect();
    assert_eq!(values,vec![1,0,1,0,1,0,0,0]);
    println!("GPU_HISTORY_SURFACE cases=8 coherent_surfaces_kept=true disoccluded_and_invalid_rejected=true");
}


// Exercise the actual reduction shader through every level, including odd
// extents and a one-ULP foreground depth in the final border texel.
fn qualify_depth_hierarchy(device:&wgpu::Device,queue:&wgpu::Queue) {
 let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:Some("exact depth reduction"),source:wgpu::ShaderSource::Wgsl(include_str!("../../crates/voxy_render/src/archive_depth_reduce.wgsl").into())});
 let layout=device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {label:None,entries:&[
  wgpu::BindGroupLayoutEntry {binding:0,visibility:wgpu::ShaderStages::COMPUTE,ty:wgpu::BindingType::Texture {sample_type:wgpu::TextureSampleType::Float {filterable:false},view_dimension:wgpu::TextureViewDimension::D2,multisampled:false},count:None},
  wgpu::BindGroupLayoutEntry {binding:1,visibility:wgpu::ShaderStages::COMPUTE,ty:wgpu::BindingType::StorageTexture {access:wgpu::StorageTextureAccess::WriteOnly,format:wgpu::TextureFormat::R32Float,view_dimension:wgpu::TextureViewDimension::D2},count:None}
 ]});
 let pipeline_layout=device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {label:None,bind_group_layouts:&[Some(&layout)],immediate_size:0});
 let pipeline=device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {label:None,layout:Some(&pipeline_layout),module:&shader,entry_point:Some("reduce_depth"),compilation_options:Default::default(),cache:None});
 let mut levels=0;
 for (mut width,mut height) in [(1u32,1u32),(3,5),(17,9),(64,32)] {
  let texture=|w,h|device.create_texture(&wgpu::TextureDescriptor {label:Some("depth hierarchy fixture"),size:wgpu::Extent3d {width:w,height:h,depth_or_array_layers:1},mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format:wgpu::TextureFormat::R32Float,usage:wgpu::TextureUsages::TEXTURE_BINDING|wgpu::TextureUsages::STORAGE_BINDING|wgpu::TextureUsages::COPY_SRC|wgpu::TextureUsages::COPY_DST,view_formats:&[]});
  let mut expected:Vec<f32>=(0..width*height).map(|i|0.2+((i*97)%701) as f32/1000.).collect();
  *expected.last_mut().unwrap()=f32::from_bits(0.2f32.to_bits()-1);
  let mut source=texture(width,height);
  queue.write_texture(source.as_image_copy(),bytemuck::cast_slice(&expected),wgpu::TexelCopyBufferLayout {offset:0,bytes_per_row:Some(width*4),rows_per_image:Some(height)},source.size());
  loop {
   let w=width.div_ceil(2);let h=height.div_ceil(2);
   let mut next=Vec::new();
   for y in 0..h {for x in 0..w {let mut value=1f32;for dy in 0..2 {for dx in 0..2 {value=value.min(expected[((y*2+dy).min(height-1)*width+(x*2+dx).min(width-1)) as usize]);}}next.push(value);}}
   let target=texture(w,h);let source_view=source.create_view(&Default::default());let target_view=target.create_view(&Default::default());
   let bind=device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&layout,entries:&[wgpu::BindGroupEntry {binding:0,resource:wgpu::BindingResource::TextureView(&source_view)},wgpu::BindGroupEntry {binding:1,resource:wgpu::BindingResource::TextureView(&target_view)}]});
   let stride=(w*4).div_ceil(256)*256;
   let readback=device.create_buffer(&wgpu::BufferDescriptor {label:None,size:u64::from(stride*h),usage:wgpu::BufferUsages::COPY_DST|wgpu::BufferUsages::MAP_READ,mapped_at_creation:false});
   let mut encoder=device.create_command_encoder(&Default::default());
   {let mut pass=encoder.begin_compute_pass(&Default::default());pass.set_pipeline(&pipeline);pass.set_bind_group(0,&bind,&[]);pass.dispatch_workgroups(w.div_ceil(8),h.div_ceil(8),1);}
   encoder.copy_texture_to_buffer(target.as_image_copy(),wgpu::TexelCopyBufferInfo {buffer:&readback,layout:wgpu::TexelCopyBufferLayout {offset:0,bytes_per_row:Some(stride),rows_per_image:Some(h)}},target.size());
   let submission=queue.submit([encoder.finish()]);let(tx,rx)=std::sync::mpsc::channel();readback.slice(..).map_async(wgpu::MapMode::Read,move|r|tx.send(r).unwrap());device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();rx.recv().unwrap().unwrap();
   let bytes=readback.slice(..).get_mapped_range().unwrap();
   for y in 0..h {for x in 0..w {let offset=(y*stride+x*4) as usize;let actual=u32::from_le_bytes(bytes[offset..offset+4].try_into().unwrap());assert_eq!(actual,next[(y*w+x) as usize].to_bits(),"depth hierarchy changed bits at {width}x{height} => {w}x{h}");}}
   drop(bytes);readback.unmap();levels+=1;
   if w==1&&h==1 {break;}
   source=target;expected=next;width=w;height=h;
  }
 }
 println!("GPU_DEPTH_HIERARCHY extents=4 levels={levels} every_texel_bit_exact=true odd_borders_preserved=true");
}

fn qualify_exact_ray_bounds(device:&wgpu::Device,queue:&wgpu::Queue) {
 let (w,h)=(33u32,19u32);
 let size=wgpu::Extent3d {width:w,height:h,depth_or_array_layers:1};
 let make=|format,usage|device.create_texture(&wgpu::TextureDescriptor {label:Some("ray equivalence fixture"),size,mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format,usage,view_formats:&[]});
 let source=make(wgpu::TextureFormat::R32Float,wgpu::TextureUsages::COPY_DST|wgpu::TextureUsages::TEXTURE_BINDING);
 let values:Vec<f32>=(0..w*h).map(|i|if i%11==0 {1.}else if i%7==0 {f32::from_bits(0.2f32.to_bits()-1)}else{0.3+(i%17) as f32*0.03}).collect();
 queue.write_texture(source.as_image_copy(),bytemuck::cast_slice(&values),wgpu::TexelCopyBufferLayout {offset:0,bytes_per_row:Some(w*4),rows_per_image:Some(h)},size);
 let depth=make(wgpu::TextureFormat::Depth32Float,wgpu::TextureUsages::RENDER_ATTACHMENT|wgpu::TextureUsages::TEXTURE_BINDING);
 let copy_shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:None,source:wgpu::ShaderSource::Wgsl(r#"
@group(0) @binding(0) var input_depth:texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index) i:u32)->@builtin(position) vec4<f32>{let p=array<vec2<f32>,3>(vec2<f32>(-1.,-1.),vec2<f32>(3.,-1.),vec2<f32>(-1.,3.));return vec4<f32>(p[i],0.,1.);}
@fragment fn fs(@builtin(position) p:vec4<f32>)->@builtin(frag_depth) f32{return textureLoad(input_depth,vec2<i32>(p.xy),0).x;}
"#.into())});
 let copy=device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {label:None,layout:None,vertex:wgpu::VertexState {module:&copy_shader,entry_point:Some("vs"),compilation_options:Default::default(),buffers:&[]},primitive:Default::default(),depth_stencil:Some(wgpu::DepthStencilState {format:wgpu::TextureFormat::Depth32Float,depth_write_enabled:Some(true),depth_compare:Some(wgpu::CompareFunction::Always),stencil:Default::default(),bias:Default::default()}),multisample:Default::default(),fragment:Some(wgpu::FragmentState {module:&copy_shader,entry_point:Some("fs"),compilation_options:Default::default(),targets:&[]}),multiview_mask:None,cache:None});
 let bind=device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&copy.get_bind_group_layout(0),entries:&[wgpu::BindGroupEntry {binding:0,resource:wgpu::BindingResource::TextureView(&source.create_view(&Default::default()))}]});
 let bounds=ray_bounds_owner::ArchiveDepthBounds::new_hierarchy(device,&depth,w,h).unwrap();
 let background=make(wgpu::TextureFormat::Rgba32Float,wgpu::TextureUsages::COPY_DST|wgpu::TextureUsages::TEXTURE_BINDING);
 let colours:Vec<f32>=(0..w*h).flat_map(|i|[i as f32/137.,(i%19) as f32/17.,(i%7) as f32/5.,1.]).collect();
 queue.write_texture(background.as_image_copy(),bytemuck::cast_slice(&colours),wgpu::TexelCopyBufferLayout {offset:0,bytes_per_row:Some(w*16),rows_per_image:Some(h)},size);
 let mut camera=vec![0f32;112];let identity=glam::Mat4::IDENTITY.to_cols_array();
 for offset in [0,16,32,76,92] {camera[offset..offset+16].copy_from_slice(&identity);}
 camera[60..64].copy_from_slice(&[w as f32,h as f32,1.,1.]);camera[108..112].copy_from_slice(&[0.,0.,1.,0.01]);
 let camera=device.create_buffer_init(&wgpu::util::BufferInitDescriptor {label:None,contents:bytemuck::cast_slice(&camera),usage:wgpu::BufferUsages::UNIFORM});
 let full_shader=voxy_render::archive_scene::archive_shader_source()+ &concat!(include_str!("ray_before_refine_cache.wgsl"),r#"
@group(1) @binding(6) var<storage,read_write> ray_results:array<vec4<u32>>;
@compute @workgroup_size(64) fn ray_proof(@builtin(global_invocation_id) invocation:vec3<u32>){
 let i=invocation.x;if i>=8192u{return;}
 let x=f32(i%64u)/32.-0.99;let y=f32((i/64u)%64u)/32.-0.99;
 let phase=f32(i)*0.371;let direction=normalize(vec3<f32>(sin(phase),cos(phase*1.17),select(0.7,-0.4,i%5u==0u)));
 let origin=vec3<f32>(x,y,select(0.05,0.98,i%3u==0u));let history=i>=4096u;
 ray_results[2u*i]=bitcast<vec4<u32>>(screen_reflection(origin,direction,history,60.,1.,8u,false));
 ray_results[2u*i+1u]=bitcast<vec4<u32>>(screen_reflection(origin,direction,history,60.,1.,8u,true));
}
override REFINE_BENCH_NEW:bool=false;
@compute @workgroup_size(64) fn refine_bench(@builtin(global_invocation_id) invocation:vec3<u32>){
 let i=invocation.x;if i>=8192u{return;}
 let x=f32(i%64u)/32.-0.99;let y=f32((i/64u)%64u)/32.-0.99;
 for(var j=0u;j<32u;j++) {
  let phase=f32(i+j*8192u)*0.371;let direction=normalize(vec3<f32>(sin(phase),cos(phase*1.17),select(0.7,-0.4,i%5u==0u)));
  let origin=vec3<f32>(x,y,select(0.05,0.98,i%3u==0u));
  if REFINE_BENCH_NEW {ray_results[i*32u+j]=bitcast<vec4<u32>>(screen_reflection(origin,direction,i>=4096u,60.,1.,8u,true));}
  else {ray_results[i*32u+j]=bitcast<vec4<u32>>(ray_before_refine_cache(origin,direction,i>=4096u,60.,1.,8u,true));}
 }
}
"#);
 let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor {label:Some("actual complete screen-ray equivalence"),source:wgpu::ShaderSource::Wgsl(full_shader.into())});
 let pipeline=device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {label:None,layout:None,module:&shader,entry_point:Some("ray_proof"),compilation_options:wgpu::PipelineCompilationOptions {constants:&[("ARCHIVE_REFINE_CACHE",1.)],..Default::default()},cache:None});
 let bytes=8192*2*16;
 let bounds_read=device.create_buffer(&wgpu::BufferDescriptor {label:None,size:u64::from(256*bounds.texture.height()),usage:wgpu::BufferUsages::MAP_READ|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
 let result=device.create_buffer(&wgpu::BufferDescriptor {label:None,size:bytes,usage:wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_SRC,mapped_at_creation:false});
 let read=device.create_buffer(&wgpu::BufferDescriptor {label:None,size:bytes,usage:wgpu::BufferUsages::MAP_READ|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
 let group0=device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&pipeline.get_bind_group_layout(0),entries:&[wgpu::BindGroupEntry {binding:0,resource:camera.as_entire_binding()}]});
 let group1=device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&pipeline.get_bind_group_layout(1),entries:&[wgpu::BindGroupEntry {binding:6,resource:result.as_entire_binding()}]});
 let dv=depth.create_view(&Default::default());let bv=background.create_view(&Default::default());let cv=bounds.texture.create_view(&Default::default());
 let group2=device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&pipeline.get_bind_group_layout(2),entries:&[wgpu::BindGroupEntry {binding:0,resource:wgpu::BindingResource::TextureView(&bv)},wgpu::BindGroupEntry {binding:3,resource:wgpu::BindingResource::TextureView(&dv)},wgpu::BindGroupEntry {binding:4,resource:wgpu::BindingResource::TextureView(&dv)},wgpu::BindGroupEntry {binding:7,resource:wgpu::BindingResource::TextureView(&cv)}]});
 let mut encoder=device.create_command_encoder(&Default::default());
 {let mut pass=encoder.begin_render_pass(&wgpu::RenderPassDescriptor {label:None,color_attachments:&[],depth_stencil_attachment:Some(wgpu::RenderPassDepthStencilAttachment {view:&dv,depth_ops:Some(wgpu::Operations {load:wgpu::LoadOp::Clear(1.),store:wgpu::StoreOp::Store}),stencil_ops:None}),..Default::default()});pass.set_pipeline(&copy);pass.set_bind_group(0,&bind,&[]);pass.draw(0..3,0..1);}
 bounds.encode(&mut encoder);
 {let mut pass=encoder.begin_compute_pass(&Default::default());pass.set_pipeline(&pipeline);pass.set_bind_group(0,&group0,&[]);pass.set_bind_group(1,&group1,&[]);pass.set_bind_group(2,&group2,&[]);pass.dispatch_workgroups(128,1,1);}
 encoder.copy_texture_to_buffer(bounds.texture.as_image_copy(),wgpu::TexelCopyBufferInfo {buffer:&bounds_read,layout:wgpu::TexelCopyBufferLayout {offset:0,bytes_per_row:Some(256),rows_per_image:Some(bounds.texture.height())}},bounds.texture.size());
 encoder.copy_buffer_to_buffer(&result,0,&read,0,bytes);let submission=queue.submit([encoder.finish()]);let(tx,rx)=std::sync::mpsc::channel();read.slice(..).map_async(wgpu::MapMode::Read,move|r|tx.send(r).unwrap());device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();rx.recv().unwrap().unwrap();
 let(tx,rx)=std::sync::mpsc::channel();bounds_read.slice(..).map_async(wgpu::MapMode::Read,move|r|tx.send(r).unwrap());device.poll(wgpu::PollType::Wait {submission_index:None,timeout:None}).unwrap();rx.recv().unwrap().unwrap();
 let minima=bounds_read.slice(..).get_mapped_range().unwrap();
 for y in 0..h.div_ceil(8) {for x in 0..w.div_ceil(8) {let mut expected=1f32;let mut maximum=0f32;for dy in 0..8 {for dx in 0..8 {let d=values[((y*8+dy).min(h-1)*w+(x*8+dx).min(w-1)) as usize];expected=expected.min(d);if d<1. {maximum=maximum.max(d);}}}let offset=(y*256+x*8) as usize;assert_eq!(u32::from_le_bytes(minima[offset..offset+4].try_into().unwrap()),expected.to_bits(),"fused bounds lost depth at tile {x},{y}");assert_eq!(u32::from_le_bytes(minima[offset+4..offset+8].try_into().unwrap()),maximum.to_bits(),"fused bounds lost maximum at tile {x},{y}");}}
 drop(minima);bounds_read.unmap();
 let data=read.slice(..).get_mapped_range().unwrap();let mut hits=0;
 for (i,pair) in data.chunks_exact(32).enumerate(){assert_eq!(&pair[..16],&pair[16..],"actual ray output changed at ray {i}");if u32::from_le_bytes(pair[12..16].try_into().unwrap())!=0 {hits+=1;}}
 assert!(hits>0&&hits<8192,"fixture must exercise both hits and misses");
 println!("GPU_EXACT_RAY_BOUNDS rays=8192 history_phases=2 full_refinements=8 hits={hits} complete_radiance_and_coverage_bit_exact=true");
 drop(data);read.unmap();
 qualify_depth_mips::run(device,queue,&bounds.texture,&values,w,h);
 // Exercise the actual material entry, including history admission and radiance.
 let mut tile_samples=[Vec::new(),Vec::new(),Vec::new(),Vec::new(),Vec::new()];let mut tile_reference:Option<Vec<u8>>=None;
 let paired_pipelines=[(false,false,false,false,false),(true,true,true,false,false),(true,true,true,true,false),(true,false,true,true,false),(true,true,true,true,true)].map(|(tiles,cells,far,hierarchy,edge_cache)|device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {label:Some("native paired DDA material gate"),layout:None,module:&shader,entry_point:Some("ray_proof"),compilation_options:wgpu::PipelineCompilationOptions {constants:&[("ARCHIVE_DDA_EDGE_CACHE",if edge_cache {1.}else{0.}),("ARCHIVE_DDA_HIERARCHY",if hierarchy {1.}else{0.}),("ARCHIVE_PAIRED_DDA",1.),("ARCHIVE_DDA_TILES",if tiles {1.}else{0.}),("ARCHIVE_DDA_CELL_BOUNDS",if cells {1.}else{0.}),("ARCHIVE_DDA_FAR_BOUND",if far {1.}else{0.})],..Default::default()},cache:None}));
 for (run,variant) in [0usize,1,2,3,4,4,3,2,1,0,0,1,2,3,4,4,3,2,1,0].into_iter().enumerate() {
 let paired=&paired_pipelines[variant];
 let p0=device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&paired.get_bind_group_layout(0),entries:&[wgpu::BindGroupEntry {binding:0,resource:camera.as_entire_binding()}]});
 let p1=device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&paired.get_bind_group_layout(1),entries:&[wgpu::BindGroupEntry {binding:6,resource:result.as_entire_binding()}]});
 let p2=device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&paired.get_bind_group_layout(2),entries:&[wgpu::BindGroupEntry {binding:0,resource:wgpu::BindingResource::TextureView(&bv)},wgpu::BindGroupEntry {binding:3,resource:wgpu::BindingResource::TextureView(&dv)},wgpu::BindGroupEntry {binding:4,resource:wgpu::BindingResource::TextureView(&dv)},wgpu::BindGroupEntry {binding:7,resource:wgpu::BindingResource::TextureView(&cv)}]});
 let began=std::time::Instant::now();let mut encoder=device.create_command_encoder(&Default::default());
 {let mut pass=encoder.begin_compute_pass(&Default::default());pass.set_pipeline(&paired);pass.set_bind_group(0,&p0,&[]);pass.set_bind_group(1,&p1,&[]);pass.set_bind_group(2,&p2,&[]);pass.dispatch_workgroups(128,1,1);}
 encoder.copy_buffer_to_buffer(&result,0,&read,0,bytes);let submission=queue.submit([encoder.finish()]);let(tx,rx)=std::sync::mpsc::channel();read.slice(..).map_async(wgpu::MapMode::Read,move|r|tx.send(r).unwrap());device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();rx.recv().unwrap().unwrap();let completion_ms=began.elapsed().as_secs_f64()*1000.;
 let data=read.slice(..).get_mapped_range().unwrap();let mut paired_hits=[0usize;2];
 if let Some(reference)=&tile_reference {assert_eq!(&data[..],&reference[..],"tile skipping changed material radiance or coverage");}else{tile_reference=Some(data.to_vec());}
 for (i,pair) in data.chunks_exact(32).enumerate(){assert_eq!(&pair[..16],&pair[16..],"paired material changed with irrelevant old march bounds at {i}");for value in pair[..16].chunks_exact(4){assert!(f32::from_le_bytes(value.try_into().unwrap()).is_finite(),"nonfinite paired material output {i}");}if f32::from_le_bytes(pair[12..16].try_into().unwrap())>0. {paired_hits[i/4096]+=1;}}
 if run>=4 {tile_samples[variant].push(completion_ms);}assert!(paired_hits.iter().all(|&h|h>0&&h<4096));drop(data);read.unmap();
 println!("GPU_PAIRED_DDA_MATERIAL rays=8192 native_depth=true history_phases=2 variant={variant} run={run} hits={paired_hits:?} complete_dispatch_and_readback_ms={completion_ms} finite_outputs=true presented_fps_qualification=false");
 }
 println!("GPU_PAIRED_DDA_TILES_BENCH synthetic=true rays_per_dispatch=8192 alternating_samples=16 full_ms={:?} flat_optimized_ms={:?} hierarchy_cells_ms={:?} hierarchy_ms={:?} edge_cache_ms={:?} every_radiance_and_coverage_bit_exact=true presented_fps_qualification=false",tile_samples[0],tile_samples[1],tile_samples[2],tile_samples[3],tile_samples[4]);
 benchmark_refine_cache(device,queue,&shader,&camera,&bv,&dv,&cv);
}

fn benchmark_refine_cache(device:&wgpu::Device,queue:&wgpu::Queue,shader:&wgpu::ShaderModule,camera:&wgpu::Buffer,background:&wgpu::TextureView,depth:&wgpu::TextureView,bounds:&wgpu::TextureView) {
 if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {println!("GPU_REFINE_CACHE_BENCH unavailable_timestamp_queries=true");return;}
 let result=device.create_buffer(&wgpu::BufferDescriptor {label:Some("every benchmark ray output"),size:262144*16,usage:wgpu::BufferUsages::STORAGE|wgpu::BufferUsages::COPY_SRC,mapped_at_creation:false});
 let read=device.create_buffer(&wgpu::BufferDescriptor {label:None,size:262144*16,usage:wgpu::BufferUsages::MAP_READ|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
 let pipelines=[false,true].map(|candidate|device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {label:Some("refinement cache paired kernel"),layout:None,module:shader,entry_point:Some("refine_bench"),compilation_options:wgpu::PipelineCompilationOptions {constants:&[("REFINE_BENCH_NEW",if candidate{1.}else{0.}),("ARCHIVE_REFINE_CACHE",1.)],..Default::default()},cache:None}));
 let groups=pipelines.iter().map(|pipeline|[
  device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&pipeline.get_bind_group_layout(0),entries:&[wgpu::BindGroupEntry {binding:0,resource:camera.as_entire_binding()}]}),
  device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&pipeline.get_bind_group_layout(1),entries:&[wgpu::BindGroupEntry {binding:6,resource:result.as_entire_binding()}]}),
  device.create_bind_group(&wgpu::BindGroupDescriptor {label:None,layout:&pipeline.get_bind_group_layout(2),entries:&[wgpu::BindGroupEntry {binding:0,resource:wgpu::BindingResource::TextureView(background)},wgpu::BindGroupEntry {binding:3,resource:wgpu::BindingResource::TextureView(depth)},wgpu::BindGroupEntry {binding:4,resource:wgpu::BindingResource::TextureView(depth)},wgpu::BindGroupEntry {binding:7,resource:wgpu::BindingResource::TextureView(bounds)}]})
 ]).collect::<Vec<_>>();
 let query=device.create_query_set(&wgpu::QuerySetDescriptor {label:None,ty:wgpu::QueryType::Timestamp,count:2});
 let resolved=device.create_buffer(&wgpu::BufferDescriptor {label:None,size:256,usage:wgpu::BufferUsages::QUERY_RESOLVE|wgpu::BufferUsages::COPY_SRC,mapped_at_creation:false});
 let times=device.create_buffer(&wgpu::BufferDescriptor {label:None,size:16,usage:wgpu::BufferUsages::MAP_READ|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false});
 let mut samples=[Vec::new(),Vec::new()];let mut expected:Option<Vec<u8>>=None;
 for (run,variant) in [0usize,1,0,1,1,0,0,1,1,0].into_iter().enumerate() {
  let mut encoder=device.create_command_encoder(&Default::default());
  {let mut pass=encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {label:Some("262144 real screen rays"),timestamp_writes:Some(wgpu::ComputePassTimestampWrites {query_set:&query,beginning_of_pass_write_index:Some(0),end_of_pass_write_index:Some(1)})});pass.set_pipeline(&pipelines[variant]);for (i,group) in groups[variant].iter().enumerate(){pass.set_bind_group(i as u32,group,&[]);}pass.dispatch_workgroups(128,1,1);}
  let submission=queue.submit([encoder.finish()]);device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();
  // Resolve after producer completion, including Metal stage-counter writes.
  let mut encoder=device.create_command_encoder(&Default::default());encoder.resolve_query_set(&query,0..2,&resolved,0);encoder.copy_buffer_to_buffer(&resolved,0,&times,0,16);encoder.copy_buffer_to_buffer(&result,0,&read,0,read.size());let submission=queue.submit([encoder.finish()]);
  let(tx,rx)=std::sync::mpsc::channel();times.slice(..).map_async(wgpu::MapMode::Read,move|r|tx.send(r).unwrap());let(tx2,rx2)=std::sync::mpsc::channel();read.slice(..).map_async(wgpu::MapMode::Read,move|r|tx2.send(r).unwrap());device.poll(wgpu::PollType::Wait {submission_index:Some(submission),timeout:None}).unwrap();rx.recv().unwrap().unwrap();rx2.recv().unwrap().unwrap();
  let bytes=times.slice(..).get_mapped_range().unwrap();let start=u64::from_le_bytes(bytes[..8].try_into().unwrap());let end=u64::from_le_bytes(bytes[8..16].try_into().unwrap());assert!(end>start&&end!=u64::MAX&&start>0,"invalid kernel timestamps");let ms=(end-start) as f64*f64::from(queue.get_timestamp_period())/1e6;drop(bytes);times.unmap();
  let data=read.slice(..).get_mapped_range().unwrap();if let Some(expected)=&expected {assert_eq!(&data[..],&expected[..],"paired ray-kernel output changed");}else{expected=Some(data.to_vec());}drop(data);read.unmap();if run>=2 {samples[variant].push(ms);}
 }
 println!("GPU_REFINE_CACHE_BENCH synthetic=true rays_per_dispatch=262144 warmups=2 alternating_samples=8 old_ms={:?} candidate_ms={:?} every_ray_output_bit_exact=true presented_fps_qualification=false",samples[0],samples[1]);
}
