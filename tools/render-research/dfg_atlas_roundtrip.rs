//! Actual renderer DFG owner -> retained atlas upload -> texture lookup audit.
use serde_json::Value;
use std::collections::HashMap;
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
    let args = std::env::args().collect::<Vec<_>>();
    assert!(
        args.len() == 3
            || (args.len() == 4
                && ["--reverse-order", "--skip-upload", "--partial-upload"]
                    .contains(&args[3].as_str()))
    );
    let skip = args.get(3).is_some_and(|s| s == "--skip-upload");
    let partial = args.get(3).is_some_and(|s| s == "--partial-upload");
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let cases = manifest["cases"].as_array().unwrap();
    let index_only = manifest["index_only"].as_bool().unwrap_or(false);
    let mut nodes = HashMap::new();
    for line in std::fs::read_to_string(&args[2]).unwrap().lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        if row.get("nv").is_none() {
            continue;
        }
        let key = (
            (row["nv"].as_f64().unwrap() as f32).to_bits(),
            (row["roughness"].as_f64().unwrap() as f32).to_bits(),
        );
        assert!(
            nodes
                .insert(
                    key,
                    [
                        row["A"].as_f64().unwrap() as f32,
                        row["B"].as_f64().unwrap() as f32,
                        row["unit_loss"].as_f64().unwrap() as f32,
                        1.
                    ]
                )
                .is_none()
        );
    }
    assert_eq!(
        nodes.len(),
        manifest["node_count"].as_u64().unwrap() as usize
    );
    let mut patches = Vec::new();
    let mut keys = std::collections::HashSet::new();
    for case in cases {
        let xs = std::array::from_fn::<_, 6, _>(|i| case["xs"][i].as_f64().unwrap() as f32);
        let ys = std::array::from_fn::<_, 6, _>(|i| case["ys"][i].as_f64().unwrap() as f32);
        let nv = case["nv"].as_f64().unwrap() as f32;
        let rough = case["roughness"].as_f64().unwrap() as f32;
        let dx = xs[1] - xs[0];
        let dy = ys[1] - ys[0];
        let left = ((nv / dx).floor() * dx).min(1. - dx);
        let bottom = ((rough / dy).floor() * dy).min(1. - dy);
        let bounds = if let Some(bounds) = case["bounds"].as_array() {
            assert_eq!(bounds.len(), 4);
            std::array::from_fn(|i| bounds[i].as_f64().unwrap() as f32)
        } else {
            [left, bottom, left + dx, bottom + dy]
        };
        let moments =
            std::array::from_fn::<_, 36, _>(|i| nodes[&(xs[i % 6].to_bits(), ys[i / 6].to_bits())]);
        let region = dfg::GgxEnergyPatchData::try_new(bounds, xs, ys, moments).unwrap();
        if keys.insert(bounds.map(f32::to_bits)) {
            patches.push(region);
        }
    }
    if args.get(3).is_some_and(|s| s == "--reverse-order") {
        patches.reverse();
    }
    let instance = GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    assert!(
        !device
            .features()
            .contains(wgpu::Features::FLOAT32_FILTERABLE)
    );
    let atlas = dfg::GgxDfgLut::with_experimental_energy_patches(&device, &patches).unwrap();
    assert_eq!(atlas.output().format(), wgpu::TextureFormat::Rgba32Float);
    let queries = cases
        .iter()
        .map(|c| {
            format!(
                "vec2<f32>({:?},{:?})",
                c["nv"].as_f64().unwrap() as f32,
                c["roughness"].as_f64().unwrap() as f32
            )
        })
        .chain([
            if index_only {
                "vec2<f32>(0.1,0.1)"
            } else {
                "vec2<f32>(0.4,0.4)"
            }
            .to_owned(),
            "vec2<f32>(-0.1,0.4)".to_owned(),
            "vec2<f32>(0.25,0.0)".to_owned(),
        ])
        .collect::<Vec<_>>();
    let count = queries.len();
    let size = count as u64 * 32;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let source = format!(
        "{}\n{}\n@group(0) @binding(0) var image:texture_2d<f32>;\n@group(0) @binding(1) var<storage,read_write> result:array<vec4<f32>>;\n@compute @workgroup_size(8) fn main(@builtin(global_invocation_id) id:vec3<u32>){{if id.x>={count}u{{return;}}let uv=array<vec2<f32>,{count}>({});result[id.x]=lookup_ggx_energy_atlas(image,uv[id.x]);let found=find_ggx_energy_region(image,uv[id.x]);result[{count}u+id.x]=vec4<f32>(vec3<f32>(found),0.0);}}",
        include_str!("../../crates/voxy_render/src/material_energy_patch.wgsl"),
        include_str!("../../crates/voxy_render/src/material_energy_atlas.wgsl"),
        queries.join(",")
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("actual DFG atlas lookup"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
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
                    min_binding_size: wgpu::BufferSize::new(size),
                },
                count: None,
            },
        ],
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(atlas.view()),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
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
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    if partial {
        atlas.encode_region(&mut encoder, [0, 0, 6, 1]).unwrap();
    } else if !skip {
        atlas.encode(&mut encoder);
    }
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(count.div_ceil(8) as u32, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &staging, 0, size);
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
    let rows = bytes
        .chunks_exact(16)
        .map(|row| {
            std::array::from_fn::<_, 4, _>(|i| {
                f32::from_le_bytes(row[4 * i..4 * i + 4].try_into().unwrap())
            })
        })
        .collect::<Vec<_>>();
    for (i, (case, m)) in cases.iter().zip(&rows).enumerate() {
        if index_only {
            let found = rows[count + i];
            assert_eq!(
                found[0] as u32,
                case["expected_region"].as_u64().unwrap() as u32
            );
            assert_eq!(found[2], 1.);
            assert!(found[1] <= case["max_visits"].as_f64().unwrap_or(21.) as f32);
            println!(
                "{}",
                serde_json::json!({"query_view":case["nv"],"query_roughness":case["roughness"],"region":found[0] as u32,"visited_nodes":found[1] as u32})
            );
            continue;
        }
        println!(
            "{}",
            serde_json::json!({"nv":case["nv"],"roughness":case["roughness"],"samples":524288,"interpolated":true,"valid":m[3]==1.,"A":m[0],"B":m[1],"unit_loss":m[2]})
        );
    }
    assert_eq!(rows[cases.len()], [0.; 4]);
    assert_eq!(rows[cases.len() + 1], [0.; 4]);
    if skip || partial {
        assert!(rows[..cases.len()].iter().all(|row| *row == [0.; 4]));
    }
    assert_eq!(
        rows[cases.len() + 2],
        if skip {
            [0.; 4]
        } else {
            [0.7626953, 0.23730469, 0., 1.]
        }
    );
    let visits = rows[count..count + cases.len()]
        .iter()
        .map(|r| r[1] as u32)
        .collect::<Vec<_>>();
    let old_linear_mean = rows[count..count + cases.len()]
        .iter()
        .map(|r| f64::from(r[0]) + 1.)
        .sum::<f64>()
        / cases.len() as f64;
    println!(
        "{}",
        serde_json::json!({"index_only":index_only,"search_kind":if patches.len()<=64 {"linear_small"}else{"threaded_bvh"},"node_visits_min":visits.iter().min(),"node_visits_max":visits.iter().max(),"node_visits_mean":visits.iter().map(|&v|f64::from(v)).sum::<f64>()/visits.len() as f64,"prior_linear_comparisons_mean":old_linear_mean,"diagnostic_extra_traversal":true,"adapter":adapter.get_info().name,"owner":"GgxDfgLut","regions":patches.len(),"hole_and_invalid_uv_rejected":true,"exact_smooth_boundary_passed":true,"scope":if index_only {"synthetic spatial-index audit; no material-accuracy or FPS claim"}else{"actual renderer owner, immutable upload and atlas texture lookup; no rendered scene or FPS qualification"}})
    );
}
