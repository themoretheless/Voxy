//! Execute the actual GgxDfgLut owner in a standalone GPU qualification harness.
use voxy_gpu_core::GraphicsOptions;
#[derive(Debug)]
pub enum RendererError {
    InvalidPrefilterSamples(u32),
    InvalidSurfaceSize {
        width: u32,
        height: u32,
        max_dimension: u32,
    },
}
#[path = "../../crates/voxy_render/src/dfg.rs"]
mod dfg;
fn main() {
    let size = std::env::args()
        .nth(1)
        .map(|s| s.parse::<u32>().expect("table size"))
        .unwrap_or(4);
    assert!(size >= 2 && size <= 64);
    let args = std::env::args().collect::<Vec<_>>();
    assert!(
        [
            "--cubic",
            "--loss-scaled",
            "--fresnel-six",
            "--smooth-residual"
        ]
        .iter()
        .filter(|mode| args.iter().any(|arg| arg == **mode))
        .count()
            <= 1,
        "choose exactly one interpolation candidate"
    );
    assert!(
        !(args.iter().any(|arg| arg == "--boundary-probes")
            && args.iter().any(|arg| arg == "--corner-probes")),
        "choose one probe set"
    );
    let interpolation_only = std::env::args().any(|s| s == "--interpolation-only");
    let row_stride = (size * 16).div_ceil(256) * 256;
    let texture_bytes = u64::from(row_stride) * u64::from(size);
    let instance = GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let info = adapter.get_info();
    assert_ne!(info.device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    assert!(dfg::GgxDfgLut::with_samples(&device, 4, 0).is_err());
    assert!(dfg::GgxDfgLut::with_energy_moments(&device, 0).is_err());
    let legacy = dfg::GgxDfgLut::new(&device, 2).unwrap();
    assert!(!legacy.has_energy_moments());
    assert_eq!(legacy.output().format(), wgpu::TextureFormat::Rgba16Float);
    let lut = dfg::GgxDfgLut::with_energy_moments(&device, size).unwrap();
    assert!(lut.has_energy_moments() && lut.belongs_to(&device));
    assert_eq!(lut.output().format(), wgpu::TextureFormat::Rgba32Float);
    let _ = lut.view();
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("actual DFG owner readback"),
        size: texture_bytes + 128,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let chunked = std::env::args().any(|s| s == "--chunked");
    let tile = std::env::args()
        .find_map(|s| s.strip_prefix("--tile=").map(str::to_owned))
        .map(|s| s.parse::<u32>().expect("tile size"))
        .unwrap_or(4);
    assert!((1..=8).contains(&tile));
    if chunked {
        for y in (0..size).step_by(tile as usize) {
            for x in (0..size).step_by(tile as usize) {
                let mut encoder = device.create_command_encoder(&Default::default());
                lut.encode_region(
                    &mut encoder,
                    [x, y, (size - x).min(tile), (size - y).min(tile)],
                )
                .unwrap();
                let submission = queue.submit([encoder.finish()]);
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: None,
                    })
                    .unwrap();
            }
        }
    }
    let mut encoder = device.create_command_encoder(&Default::default());
    if !chunked {
        lut.encode(&mut encoder);
    }
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: lut.output(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_stride),
                rows_per_image: Some(size),
            },
        },
        wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
    );
    let samples_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("actual DFG interpolated samples"),
        size: 128,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mut lookup_source = format!(
        "{}\n{}",
        include_str!("../../crates/voxy_render/src/material_energy_lookup.wgsl"),
        r#"
@group(0) @binding(0) var lut:texture_2d<f32>;
@group(0) @binding(1) var<storage,read_write> result:array<vec4<f32>>;
@compute @workgroup_size(8) fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let queries=array<vec2<f32>,8>(vec2<f32>(0.25,0.25),vec2<f32>(0.5,0.25),vec2<f32>(0.75,0.25),vec2<f32>(0.25,0.5),vec2<f32>(0.5,0.5),vec2<f32>(0.75,0.5),vec2<f32>(0.25,0.75),vec2<f32>(0.75,0.75));
 result[id.x]=lookup_ggx_energy(lut,queries[id.x]);
}
"#
    );
    if std::env::args().any(|s| s == "--cubic") {
        lookup_source = lookup_source.replace(
            "lookup_ggx_energy(lut,queries[id.x])",
            "lookup_ggx_energy_cubic(lut,queries[id.x])",
        );
    }
    if std::env::args().any(|s| s == "--loss-scaled") {
        lookup_source = lookup_source.replace(
            "lookup_ggx_energy(lut,queries[id.x])",
            "lookup_ggx_energy_loss_scaled(lut,queries[id.x])",
        );
    }
    if std::env::args().any(|s| s == "--fresnel-six") {
        lookup_source = lookup_source.replace(
            "lookup_ggx_energy(lut,queries[id.x])",
            "lookup_ggx_energy_fresnel_six(lut,queries[id.x])",
        );
    }
    if std::env::args().any(|s| s == "--boundary-probes") {
        lookup_source=lookup_source.replace("let queries=array<vec2<f32>,8>(vec2<f32>(0.25,0.25),vec2<f32>(0.5,0.25),vec2<f32>(0.75,0.25),vec2<f32>(0.25,0.5),vec2<f32>(0.5,0.5),vec2<f32>(0.75,0.5),vec2<f32>(0.25,0.75),vec2<f32>(0.75,0.75));","let queries=array<vec2<f32>,8>(vec2<f32>(0.137,0.213),vec2<f32>(0.333,0.417),vec2<f32>(0.618,0.731),vec2<f32>(0.901,0.237),vec2<f32>(0.003,0.4),vec2<f32>(0.999,0.4),vec2<f32>(0.5,0.001),vec2<f32>(0.5,0.999));");
    }
    if std::env::args().any(|s| s == "--smooth-residual") {
        lookup_source = lookup_source.replace(
            "lookup_ggx_energy(lut,queries[id.x])",
            "lookup_ggx_energy_fresnel_residual(lut,queries[id.x])",
        );
    }
    if std::env::args().any(|s| s == "--corner-probes") {
        lookup_source=lookup_source.replace("let queries=array<vec2<f32>,8>(vec2<f32>(0.25,0.25),vec2<f32>(0.5,0.25),vec2<f32>(0.75,0.25),vec2<f32>(0.25,0.5),vec2<f32>(0.5,0.5),vec2<f32>(0.75,0.5),vec2<f32>(0.25,0.75),vec2<f32>(0.75,0.75));","let queries=array<vec2<f32>,8>(vec2<f32>(1.0,0.005),vec2<f32>(1.0,0.0001),vec2<f32>(0.999,0.01),vec2<f32>(0.001,0.005),vec2<f32>(0.005,0.05),vec2<f32>(0.01,0.1),vec2<f32>(0.05,0.005),vec2<f32>(0.05,0.25));");
    }
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("actual DFG lookup audit"),
        source: wgpu::ShaderSource::Wgsl(lookup_source.into()),
    });
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
                    min_binding_size: wgpu::BufferSize::new(128),
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
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(lut.view()),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: samples_buffer.as_entire_binding(),
            },
        ],
    });
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&samples_buffer, 0, &staging, texture_bytes, 128);
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
    for y in 0..size {
        for x in 0..size {
            let b = (y * row_stride + x * 16) as usize;
            let m = std::array::from_fn::<_, 4, _>(|c| {
                f32::from_le_bytes(bytes[b + 4 * c..b + 4 * c + 4].try_into().unwrap())
            });
            assert!(
                m.iter().all(|v| v.is_finite() && *v >= 0. && *v <= 1.),
                "invalid owner pixel {x},{y}: {m:?}"
            );
            assert_eq!(m[3], 1.);
            assert!((m[0] + m[1] + m[2] - 1.).abs() <= 5e-6);
            let view_coordinate = x as f32 / (size - 1) as f32;
            let nv = view_coordinate * view_coordinate;
            let roughness = y as f32 / (size - 1) as f32;
            if !interpolation_only {
                println!(
                    "{{\"nv\":{nv},\"roughness\":{roughness},\"samples\":524288,\"A\":{},\"B\":{},\"unit_loss\":{}}}",
                    m[0], m[1], m[2]
                );
            }
        }
    }
    let queries = if std::env::args().any(|s| s == "--corner-probes") {
        [
            (1.0, 0.005),
            (1.0, 0.0001),
            (0.999, 0.01),
            (0.001, 0.005),
            (0.005, 0.05),
            (0.01, 0.1),
            (0.05, 0.005),
            (0.05, 0.25),
        ]
    } else if std::env::args().any(|s| s == "--boundary-probes") {
        [
            (0.137f32, 0.213f32),
            (0.333, 0.417),
            (0.618, 0.731),
            (0.901, 0.237),
            (0.003, 0.4),
            (0.999, 0.4),
            (0.5, 0.001),
            (0.5, 0.999),
        ]
    } else {
        [
            (0.25f32, 0.25f32),
            (0.5, 0.25),
            (0.75, 0.25),
            (0.25, 0.5),
            (0.5, 0.5),
            (0.75, 0.5),
            (0.25, 0.75),
            (0.75, 0.75),
        ]
    };
    for (i, (nv, roughness)) in queries.into_iter().enumerate() {
        let b = texture_bytes as usize + i * 16;
        let m = std::array::from_fn::<_, 4, _>(|c| {
            f32::from_le_bytes(bytes[b + 4 * c..b + 4 * c + 4].try_into().unwrap())
        });
        if !std::env::args().any(|s| s == "--allow-rejected-probes") {
            assert_eq!(m[3], 1.);
        }
        assert!(m.iter().all(|v| v.is_finite() && *v >= 0. && *v <= 1.));
        println!(
            "{{\"nv\":{nv},\"roughness\":{roughness},\"samples\":524288,\"interpolated\":true,\"valid\":{},\"A\":{},\"B\":{},\"unit_loss\":{}}}",
            m[3] == 1.,
            m[0],
            m[1],
            m[2]
        );
    }
    println!(
        "{{\"adapter\":{:?},\"backend\":\"{:?}\",\"owner\":\"GgxDfgLut\",\"format\":\"RGBA32Float\",\"grid\":\"square-view-linear-roughness-endpoints\",\"pixels\":{},\"interpolation_queries\":8,\"scope\":\"actual renderer owner and fragment integration; not scene lighting or table interpolation accuracy\"}}",
        info.name,
        info.backend,
        size * size
    );
}
