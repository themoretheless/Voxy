//! GPU interpolation audit of locally refined, GPU-produced moment patches.
use serde_json::Value;
use std::collections::HashMap;
use voxy_gpu_core::{ComputeProgram, GraphicsOptions};
fn main() {
    const STRIDE: usize = 162;
    let args = std::env::args().collect::<Vec<_>>();
    let invalidate = args
        .get(3)
        .is_some_and(|arg| arg == "--invalidate-first-node");
    assert!(args.len() == 3 || (args.len() == 4 && invalidate));
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let mut nodes = HashMap::new();
    for line in std::fs::read_to_string(&args[2]).unwrap().lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        if row.get("nv").is_none() {
            continue;
        }
        let nv = row["nv"].as_f64().unwrap() as f32;
        let r = row["roughness"].as_f64().unwrap() as f32;
        let m = [
            row["A"].as_f64().unwrap() as f32,
            row["B"].as_f64().unwrap() as f32,
            row["unit_loss"].as_f64().unwrap() as f32,
            1.,
        ];
        assert!(m.iter().all(|v| v.is_finite() && (0. ..=1.).contains(v)));
        assert!((m[0] + m[1] + m[2] - 1.).abs() <= 5e-6);
        assert!(nodes.insert((nv.to_bits(), r.to_bits()), m).is_none());
    }
    assert_eq!(
        nodes.len(),
        manifest["node_count"].as_u64().unwrap() as usize
    );
    let cases = manifest["cases"].as_array().unwrap();
    let mut words = vec![0u32; 1 + cases.len() * STRIDE];
    words[0] = cases.len() as u32;
    for (i, case) in cases.iter().enumerate() {
        let b = 1 + i * STRIDE;
        words[b] = (case["nv"].as_f64().unwrap() as f32).to_bits();
        words[b + 1] = (case["roughness"].as_f64().unwrap() as f32).to_bits();
        let xs = case["xs"].as_array().unwrap();
        let ys = case["ys"].as_array().unwrap();
        for j in 0..6 {
            words[b + 2 + j] = (xs[j].as_f64().unwrap() as f32).to_bits();
            words[b + 8 + j] = (ys[j].as_f64().unwrap() as f32).to_bits();
        }
        for y in 0..6 {
            for x in 0..6 {
                let m = nodes[&(words[b + 2 + x], words[b + 8 + y])];
                for c in 0..4 {
                    words[b + 14 + 4 * (y * 6 + x) + c] = m[c].to_bits();
                }
            }
        }
    }
    if invalidate {
        for i in 0..cases.len() {
            words[1 + i * STRIDE + 17] = 0;
        }
    }
    let source = format!(
        "{}\n{}",
        include_str!("../../crates/voxy_render/src/material_energy_patch.wgsl"),
        r#"
@group(0) @binding(0) var<storage,read_write> data:array<u32>;
@compute @workgroup_size(8) fn cs_main(@builtin(global_invocation_id) id:vec3<u32>) {
 if id.x>=data[0]{return;}let b=1u+162u*id.x;
 var region:GgxEnergyPatch;
 for(var i=0u;i<6u;i++){region.view_nodes[i]=bitcast<f32>(data[b+2u+i]);region.roughness_nodes[i]=bitcast<f32>(data[b+8u+i]);}
 for(var i=0u;i<36u;i++){for(var c=0u;c<4u;c++){region.moments[i][c]=bitcast<f32>(data[b+14u+4u*i+c]);}}
 let m=interpolate_ggx_energy_patch(region,vec2<f32>(bitcast<f32>(data[b]),bitcast<f32>(data[b+1u])));
 for(var c=0u;c<3u;c++){data[b+158u+c]=bitcast<u32>(m[c]);}data[b+161u]=select(0u,1u,m.w==1.0);
}
"#
    );
    let instance = GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let program = pollster::block_on(ComputeProgram::new(&device, &source)).unwrap();
    let bytes = words
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect::<Vec<_>>();
    let job = program.create_job(&device, &bytes).unwrap();
    let mut encoder = device.create_command_encoder(&Default::default());
    job.encode_step(&mut encoder, [cases.len().div_ceil(8) as u32, 1, 1])
        .unwrap();
    let snapshot = job.encode_snapshot(&mut encoder).unwrap();
    let submission = queue.submit([encoder.finish()]);
    let mut read = snapshot.begin_read();
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    let bytes = read.try_read().unwrap().unwrap();
    let output = bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect::<Vec<_>>();
    for (i, case) in cases.iter().enumerate() {
        let b = 1 + i * STRIDE;
        assert_eq!(&words[b..b + 158], &output[b..b + 158]);
        let m = std::array::from_fn::<_, 3, _>(|c| f32::from_bits(output[b + 158 + c]));
        println!(
            "{}",
            serde_json::json!({"nv":case["nv"],"roughness":case["roughness"],"samples":524288,"interpolated":true,"valid":output[b+161]==1,"A":m[0],"B":m[1],"unit_loss":m[2]})
        );
    }
    println!(
        "{}",
        serde_json::json!({"adapter":adapter.get_info().name,"scope":"GPU-produced coefficients and GPU local region interpolation; no production region atlas or whole-domain admission"})
    );
}
