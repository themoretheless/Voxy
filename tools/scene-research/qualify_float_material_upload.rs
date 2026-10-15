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
        required_features: wgpu::Features::FLOAT32_FILTERABLE,
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
