//! Bounded scheduling control: uses the production key and insertion block.
fn shader() -> String {
    let fast = include_str!("world_visibility_fast.wgsl");
    let begin = fast.find("fn visibility_bound_key(").unwrap();
    let end = begin + fast[begin..].find("\nfn visibility_node_key(").unwrap();
    let wide = include_str!("world_visibility_wide.wgsl");
    let start = wide.find("   if WORLD_WIDE_NEAR_FIRST&&").unwrap();
    let stop = start + wide[start..].find("   size+=children;continue;").unwrap();
    format!(
        r#"
struct Pair {{lo:f32,hi:f32}}
@group(0) @binding(0) var<storage,read> nearest_active:array<u32>;
@group(0) @binding(1) var<storage,read_write> output:array<u32>;
var<private> nearest_node_origin:array<Pair,3>;
var<private> nearest_node_direction:array<Pair,3>;
override WORLD_WIDE_NEAR_FIRST:bool=false;
const WORLD_FAST_DEFER_NONCLEAR:bool=true;
const WORLD_FAST_CAN_BOUND:bool=true;
fn wi_finite(value:f32)->bool {{return (bitcast<u32>(value)&0x7f800000u)!=0x7f800000u;}}
{}
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id:vec3<u32>) {{
 let batch=id.x;let begin=batch*72u;if begin>=arrayLength(&nearest_active){{return;}}
 let children=nearest_active[begin];
 nearest_node_direction[0]=Pair(bitcast<f32>(nearest_active[begin+1u]),bitcast<f32>(nearest_active[begin+1u]));
 var stack:array<u32,64>;let size=2u;stack[0]=0xdeadbeefu;stack[1]=0x13579bdfu;
 for(var lane=0u;lane<children;lane++){{stack[size+lane]=begin+8u+lane*8u;}}
{}
 let out=batch*12u;output[out]=stack[0];output[out+1u]=stack[1];output[out+2u]=children;
 for(var lane=0u;lane<children;lane++){{output[out+3u+lane]=(stack[size+children-1u-lane]-begin-8u)/8u;}}
}}
"#,
        &fast[begin..end],
        &wide[start..stop]
    )
}
#[test]
fn actual_scheduling_block_validates() {
    let module = naga::front::wgsl::parse_str(&shader()).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
}
#[test]
#[ignore = "exclusive Metal GPU; fresh VOXY_WIDE_ORDER_PROOF required"]
#[cfg(target_os = "macos")]
fn actual_sort_preserves_frontier_and_prior_stack() -> Result<(), Box<dyn std::error::Error>> {
    use wgpu::util::DeviceExt;
    let root = std::path::PathBuf::from(
        std::env::var_os("VOXY_WIDE_ORDER_PROOF").ok_or("fresh proof directory required")?,
    );
    std::fs::create_dir(&root)?;
    let source = shader();
    std::fs::write(root.join("production-block.wgsl"), &source)?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("production wide scheduling block"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let mut data = Vec::new();
    let mut expected = Vec::new();
    for children in 2..=8 {
        for direction in [-1f32, 1.] {
            for equal in [false, true] {
                let mut words = [0u32; 72];
                words[0] = children;
                words[1] = direction.to_bits();
                let mut centers = Vec::new();
                for lane in 0..children {
                    let center = if equal {
                        0.
                    } else {
                        ((lane * 5 + 3) % children) as f32
                    };
                    centers.push(center);
                    words[8 + lane as usize * 8] = center.to_bits();
                    words[11 + lane as usize * 8] = center.to_bits();
                }
                let mut popped: Vec<u32> = (0..children).rev().collect();
                popped.sort_by(|&a, &b| {
                    (centers[a as usize] * direction).total_cmp(&(centers[b as usize] * direction))
                });
                data.extend(words);
                expected.push(popped);
            }
        }
    }
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("synthetic source bounds for scheduling only"),
        contents: bytemuck::cast_slice(&data),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let mut reports = Vec::new();
    for enabled in [false, true] {
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[("WORLD_WIDE_NEAR_FIRST", if enabled { 1. } else { 0. })],
                ..Default::default()
            },
            cache: None,
        });
        let bytes = expected.len() as u64 * 48;
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
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
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(expected.len() as u32, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &read, 0, bytes);
        let submission = queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })?;
        rx.recv()??;
        let mapped = read.slice(..).get_mapped_range()?;
        std::fs::write(root.join(format!("enabled-{enabled}.bin")), &mapped)?;
        for (i, row) in mapped.chunks_exact(48).enumerate() {
            let w: Vec<u32> = row
                .chunks_exact(4)
                .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                .collect();
            assert_eq!(&w[..2], &[0xdeadbeef, 0x13579bdf]);
            let n = w[2] as usize;
            let actual = &w[3..3 + n];
            let want = if enabled {
                expected[i].clone()
            } else {
                (0..n as u32).rev().collect()
            };
            assert_eq!(actual, want, "case {i}, enabled {enabled}");
        }
        reports.push(serde_json::json!({"enabled":enabled,"cases":expected.len(),"all_frontiers_and_stack_prefixes_preserved":true}));
        drop(mapped);
        read.unmap();
    }
    std::fs::write(
        root.join("proof.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"scope":"production scheduling block on synthetic bound keys only","reports":reports,"performance_qualified":false}),
        )?,
    )?;
    Ok(())
}
