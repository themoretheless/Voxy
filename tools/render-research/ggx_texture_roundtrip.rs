//! Actual GPU storage-texture write -> textureLoad -> staging readback audit.
use voxy_gpu_core::GraphicsOptions;
fn main() {
    let instance = GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let info = adapter.get_info();
    assert_ne!(info.device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let values = [1e-18f32, 6.2504313e-10, 6.271335e-6, 0.6931472];
    for (format, name) in [
        (wgpu::TextureFormat::Rgba16Float, "rgba16float"),
        (wgpu::TextureFormat::Rgba32Float, "rgba32float"),
    ] {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("GGX moment precision audit"),
            size: wgpu::Extent3d {
                width: 4,
                height: 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sampled moments"),
            size: 256,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("moment precision readback"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let source = format!(
            "@group(0) @binding(0) var image:texture_storage_2d<{name},write>;\n@compute @workgroup_size(4,2) fn main(@builtin(global_invocation_id) id:vec3<u32>){{let bits=array<u32,4>({}u,{}u,{}u,{}u);textureStore(image,vec2<i32>(id.xy),vec4<f32>(0.5,0.25,bitcast<f32>(bits[id.x])*(1.0-0.5*f32(id.y)),1.0));}}",
            values[0].to_bits(),
            values[1].to_bits(),
            values[2].to_bits(),
            values[3].to_bits()
        );
        let write_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("write moment texture"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let write_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("write moments"),
            layout: None,
            module: &write_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let write_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &write_pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });
        let read_source = format!(
            "{}\n{}",
            include_str!("../../crates/voxy_render/src/material_energy_lookup.wgsl"),
            r#"
@group(0) @binding(0) var image:texture_2d<f32>;
@group(0) @binding(1) var<storage,read_write> result:array<vec4<f32>>;
@compute @workgroup_size(4,2) fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let index=id.y*4u+id.x;
 result[index]=textureLoad(image,vec2<i32>(id.xy),0);
 let queries=array<vec2<f32>,8>(vec2<f32>(0.125,0.25),vec2<f32>(0.25,0.25),vec2<f32>(0.25,0.5),vec2<f32>(0.75,0.5),vec2<f32>(0.0,0.0),vec2<f32>(1.0,1.0),vec2<f32>(-0.1,0.5),vec2<f32>(0.5,1.1));
 result[8u+index]=lookup_ggx_energy(image,queries[index]);
}
"#
        );
        let read_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("load and interpolate moment texture"),
            source: wgpu::ShaderSource::Wgsl(read_source.into()),
        });
        // Explicit nonfiltering float layout works without FLOAT32_FILTERABLE.
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(256),
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let read_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("load moments"),
            layout: Some(&pipeline_layout),
            module: &read_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let read_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
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
            pass.set_pipeline(&write_pipeline);
            pass.set_bind_group(0, &write_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&read_pipeline);
            pass.set_bind_group(0, &read_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, 256);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let bytes = staging.slice(..).get_mapped_range().unwrap();
        let results = std::array::from_fn::<_, 16, _>(|i| {
            std::array::from_fn::<_, 4, _>(|c| {
                f32::from_le_bytes(
                    bytes[(i * 4 + c) * 4..(i * 4 + c + 1) * 4]
                        .try_into()
                        .unwrap(),
                )
            })
        });
        for i in 0..8 {
            let row = results[i];
            let expected = values[i % 4] * if i < 4 { 1. } else { 0.5 };
            assert_eq!([row[0], row[1], row[3]], [0.5, 0.25, 1.]);
            if name == "rgba32float" {
                assert_eq!(row[2].to_bits(), expected.to_bits());
            } else if i % 4 < 2 {
                assert_eq!(row[2], 0.);
            }
            println!(
                "{{\"format\":\"{name}\",\"case\":\"texel\",\"input_bits\":{},\"loaded_bits\":{}}}",
                expected.to_bits(),
                row[2].to_bits()
            );
        }
        // Physical view coordinates map through sqrt to the stored grid.
        let r: Vec<f64> = results[..8].iter().map(|v| f64::from(v[2])).collect();
        let expected = [
            (0.125_f64, 0.25_f64),
            (0.25, 0.25),
            (0.25, 0.5),
            (0.75, 0.5),
            (0.0, 0.0),
            (1.0, 1.0),
        ]
        .map(|(u, v)| {
            let x = f64::from((u as f32).sqrt() * 3.0);
            let first = (x.floor() as usize).min(3);
            let second = (first + 1).min(3);
            let t = x - first as f64;
            ((1.0 - t) * r[first] + t * r[second]) * (1.0 - v)
                + ((1.0 - t) * r[first + 4] + t * r[second + 4]) * v
        });
        for i in 0..6 {
            let actual = results[8 + i];
            assert_eq!([actual[0], actual[1], actual[3]], [0.5, 0.25, 1.]);
            assert!(
                (f64::from(actual[2]) - expected[i]).abs() <= expected[i].abs() * 2e-7,
                "interpolation mismatch: {actual:?}, expected {}",
                expected[i]
            );
            if name == "rgba32float" {
                assert!(actual[2] > 0.);
            }
            println!(
                "{{\"format\":\"{name}\",\"case\":\"interpolation\",\"query\":{i},\"expected\":{},\"actual\":{}}}",
                expected[i], actual[2]
            );
        }
        for row in &results[14..16] {
            assert_eq!(*row, [0.; 4]);
        }
    }
    println!(
        "{{\"adapter\":{:?},\"backend\":\"{:?}\",\"float32_filterable_requested\":false,\"scope\":\"GPU storage write, textureLoad and shared manual bilinear lookup; no production material integration\"}}",
        info.name, info.backend
    );
}
