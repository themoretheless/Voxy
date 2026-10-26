//! Exercise the production nearest-update control flow, with small exact mock
//! arithmetic. This is not a substitute for the source-geometry GPU oracle.
#[test]
#[ignore = "exclusive Metal GPU; fresh VOXY_NEAREST_TIE_PROOF required"]
#[cfg(target_os = "macos")]
fn superseded_ties_do_not_depend_on_visit_order() -> Result<(), Box<dyn std::error::Error>> {
    use wgpu::util::DeviceExt;
    let root = std::path::PathBuf::from(
        std::env::var_os("VOXY_NEAREST_TIE_PROOF").ok_or("fresh proof directory required")?,
    );
    std::fs::create_dir(&root)?;
    let source = include_str!("world_nearest.wgsl");
    let extract = |name: &str| {
        let start = source.find(&format!("fn {name}(")).unwrap();
        let tail = &source[start..];
        let end = tail[1..].find("\nfn ").map_or(tail.len(), |v| v + 1);
        tail[..end].to_owned()
    };
    let save = extract("nearest_save_parameters");
    let update = extract("nearest_evaluate_and_update");
    let old = update.replace("  if cmp.order<0 {nearest_ties=0u;}\n", "");
    assert_ne!(old, update);
    let support = r#"
struct ExInt{value:u32,valid:u32}
struct ExParameters{t:ExInt,u:ExInt,v:ExInt,denominator:ExInt,valid:u32}
struct ExVec{x:u32}
struct Triangle{a:ExVec,b:ExVec,c:ExVec}
struct Decision{state:u32,reason:u32}
struct Compare{order:i32,valid:u32}
@group(0) @binding(0) var<storage,read> events:array<vec4<u32>>;
@group(0) @binding(1) var<storage,read_write> result:array<vec4<u32>>;
var<private> exact_layout:array<vec4<u32>,3>;
var<private> ex_parameters:ExParameters;
var<private> nearest_best:ExParameters;
var<private> nearest_slot:u32;
var<private> nearest_primitive:u32;
var<private> nearest_kind:u32;
var<private> nearest_admitted_kind:u32;
var<private> nearest_best_t:u32;
var<private> nearest_candidate_t:u32;
var<private> nearest_exact_calls:u32;
var<private> nearest_ties:u32;
var<private> event:u32;
var<private> unknown:bool;
fn exact_copy_parameter(value:ExInt,slot:u32)->ExInt{return value;}
fn nearest_source_vertices(slot:u32,primitive:u32,ray:u32)->Triangle{return Triangle(ExVec(slot),ExVec(0u),ExVec(0u));}
fn ex_triangle(a:ExVec,b:ExVec,c:ExVec,d:ExVec)->Decision{ex_parameters=ExParameters(ExInt(events[event][a.x],1u),ExInt(0u,1u),ExInt(0u,1u),ExInt(1u,1u),1u);return Decision(1u,0u);}
fn nearest_admit_decision(material:u32,decision:Decision)->bool{return true;}
fn ex_cmp_products(a:ExInt,b:ExInt,c:ExInt,d:ExInt)->Compare{return Compare(select(select(0,1,a.value*b.value>c.value*d.value),-1,a.value*b.value<c.value*d.value),1u);}
fn nearest_numeric_unknown(){unknown=true;}
@compute @workgroup_size(1) fn main(@builtin(global_invocation_id) id:vec3<u32>){
 event=id.x;if event>=arrayLength(&events){return;}
 nearest_ties=0u;nearest_best.valid=0u;
 for(var slot=0u;slot<3u;slot++){nearest_evaluate_and_update(slot,0u,0u,ExVec(0u));}
 result[event]=vec4<u32>(nearest_best.t.value,nearest_ties,nearest_slot,select(0u,1u,unknown));
}
"#;
    let events = [
        [3u32, 3, 1, 0],
        [3, 1, 3, 0],
        [1, 3, 3, 0],
        [1, 1, 3, 0],
        [3, 1, 1, 0],
        [1, 3, 1, 0],
        [1, 1, 1, 0],
    ];
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("synthetic exact event ordering"),
        contents: bytemuck::cast_slice(&events),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let mut reports = Vec::new();
    for (name, update) in [("before", old), ("after", update)] {
        let wgsl = format!("{support}\n{save}\n{update}");
        std::fs::write(root.join(format!("{name}.wgsl")), &wgsl)?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("production nearest-update control flow"),
            source: wgpu::ShaderSource::Wgsl(wgsl.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 112,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 112,
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
            pass.dispatch_workgroups(events.len() as u32, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &read, 0, 112);
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
        let data = read.slice(..).get_mapped_range()?;
        std::fs::write(root.join(format!("{name}.bin")), &data)?;
        let rows: Vec<[u32; 4]> = data
            .chunks_exact(16)
            .map(|r| {
                std::array::from_fn(|i| u32::from_le_bytes(r[i * 4..i * 4 + 4].try_into().unwrap()))
            })
            .collect();
        let expected: Vec<[u32; 4]> = events
            .iter()
            .map(|v| {
                let minimum = *v[..3].iter().min().unwrap();
                [
                    minimum,
                    v[..3].iter().filter(|&&x| x == minimum).count() as u32 - 1,
                    v[..3].iter().position(|&x| x == minimum).unwrap() as u32,
                    0,
                ]
            })
            .collect();
        if name == "before" {
            assert_eq!(rows[0][1], 1);
            assert_ne!(rows, expected);
        } else {
            assert_eq!(rows, expected);
        }
        reports.push(serde_json::json!({"variant":name,"rows":rows,"expected":expected,"all_expected":rows==expected}));
        drop(data);
        read.unmap();
    }
    std::fs::write(
        root.join("proof.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"scope":"production nearest update control flow; mocked small exact arithmetic","reports":reports}),
        )?,
    )?;
    Ok(())
}
