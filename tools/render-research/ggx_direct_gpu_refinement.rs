//! Direct renderer GPU APIs and native sampled policy; no child processes.
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
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
use dfg::{
    GgxEnergyBatchKey as Key, GgxEnergyBatchStage as Stage, GgxEnergyGpuPrecompute as Gpu,
    GgxEnergyRefinementDecision as Decision, GgxEnergyRefinementObservation as Observation,
    GgxEnergyRefinementPlan as Plan, GgxEnergyRefinementPolicy as Policy, PendingGgxEnergyBatch,
};
fn id(uv: [f32; 2]) -> [u32; 2] {
    uv.map(f32::to_bits)
}
fn rows(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}
fn read(
    device: &wgpu::Device,
    pending: &mut PendingGgxEnergyBatch,
    key: Key,
) -> Result<dfg::GgxEnergyBatchOutput, Box<dyn std::error::Error>> {
    loop {
        device.poll(wgpu::PollType::Poll)?;
        if let Some(output) = pending.try_read(device, key)? {
            return Ok(output);
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        4,
        "output independent-reference uncertainty-comparison"
    );
    let out = PathBuf::from(&args[1]);
    if out.exists() {
        return Err("preserve previous qualification".into());
    }
    fs::create_dir_all(&out)?;
    let refs = rows(Path::new(&args[2]))
        .into_iter()
        .filter(|r| r.get("nv").is_some())
        .map(|r| {
            (
                id([
                    r["nv"].as_f64().unwrap() as f32,
                    r["roughness"].as_f64().unwrap() as f32,
                ]),
                r,
            )
        })
        .collect::<HashMap<_, _>>();
    let prior: Value = serde_json::from_str(&fs::read_to_string(&args[3])?)?;
    let uncertainty = prior["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                id([
                    r["nv"].as_f64().unwrap() as f32,
                    r["roughness"].as_f64().unwrap() as f32,
                ]),
                [
                    r["reference_estimated_uncertainty"].as_f64().unwrap()
                        + r["B_reference_estimated_uncertainty"].as_f64().unwrap(),
                    r["B_reference_estimated_uncertainty"].as_f64().unwrap(),
                    r["reference_estimated_uncertainty"].as_f64().unwrap(),
                ],
            )
        })
        .collect::<HashMap<_, _>>();
    let instance = GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let gpu = pollster::block_on(Gpu::new(&device))?;
    let seeds = [
        [1., 0.005],
        [1., 0.0001],
        [0.999, 0.01],
        [0.001, 0.005],
        [0.005, 0.05],
        [0.01, 0.1],
        [0.05, 0.005],
        [0.05, 0.25],
    ];
    let regions = Plan::regions_from_seeds(&seeds)?;
    let policy = Policy::try_new(1e-5, 8)?;
    let mut level = 1;
    let mut history = Vec::new();
    loop {
        let plan = Plan::try_new(&regions, level)?;
        let mut coefficients = vec![[0.; 4]; plan.nodes().len()];
        let mut node_batches = 0;
        fs::write(
            out.join("progress.json"),
            serde_json::to_vec_pretty(
                &json!({"status":"running","pid":std::process::id(),"level":level,"stage":"nodes","history":history}),
            )?,
        )?;
        for (batch, coordinates) in plan
            .nodes()
            .chunks(dfg::GGX_ENERGY_GPU_BATCH_LIMIT)
            .enumerate()
        {
            let start = batch * dfg::GGX_ENERGY_GPU_BATCH_LIMIT;
            let key = Key {
                generation: 1,
                refinement: level,
                offset: start as u32,
            };
            let mut encoder = device.create_command_encoder(&Default::default());
            let encoded = gpu.encode_nodes(&mut encoder, key, coordinates)?;
            queue.submit([encoder.finish()]);
            let mut pending = encoded.submitted();
            let output = read(&device, &mut pending, key)?;
            assert_eq!(output.stage, Stage::IntegrateNodes);
            assert_eq!(output.coordinates, coordinates);
            assert_eq!(output.key, key);
            assert_eq!(
                pending.try_read(&device, key).unwrap_err(),
                voxy_gpu_core::ComputeError::Consumed
            );
            coefficients[start..start + coordinates.len()].copy_from_slice(&output.moments);
            node_batches += 1;
        }
        fs::write(out.join(format!("nodes-{level}.jsonl")),plan.nodes().iter().zip(&coefficients).map(|(p,m)|format!("{}\n",json!({"nv":p[0],"roughness":p[1],"samples":524288,"valid":true,"A":m[0],"B":m[1],"unit_loss":m[2]}))).collect::<String>())?;
        let manifest = json!({"refinement":level,"node_count":plan.nodes().len(),"cases":plan.probes().iter().map(|p|json!({"nv":p.uv[0],"roughness":p.uv[1],"xs":p.view_nodes,"ys":p.roughness_nodes,"bounds":p.bounds})).collect::<Vec<_>>()});
        fs::write(
            out.join(format!("level-{level}.json")),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        let mut observations = Vec::new();
        let mut probe_batches = 0;
        for (batch, probes) in plan
            .probes()
            .chunks(dfg::GGX_ENERGY_GPU_BATCH_LIMIT)
            .enumerate()
        {
            let start = batch * dfg::GGX_ENERGY_GPU_BATCH_LIMIT;
            let key = Key {
                generation: 1,
                refinement: level,
                offset: start as u32,
            };
            let mut encoder = device.create_command_encoder(&Default::default());
            let encoded = gpu.encode_probes(&mut encoder, key, probes, &coefficients)?;
            queue.submit([encoder.finish()]);
            let output = read(&device, &mut encoded.submitted(), key)?;
            assert_eq!(output.stage, Stage::InterpolateProbes);
            for (uv, m) in output.coordinates.into_iter().zip(output.moments) {
                let reference = &refs[&id(uv)]["reference"];
                observations.push(Observation {
                    uv,
                    gpu: m,
                    reference: [
                        reference["A"].as_f64().unwrap(),
                        reference["B"].as_f64().unwrap(),
                        reference["loss"].as_f64().unwrap(),
                    ],
                    uncertainty: uncertainty[&id(uv)],
                });
            }
            probe_batches += 1;
        }
        fs::write(out.join(format!("lookup-{level}.jsonl")),observations.iter().map(|o|format!("{}\n",json!({"nv":o.uv[0],"roughness":o.uv[1],"samples":524288,"interpolated":true,"valid":o.gpu[3]==1.,"A":o.gpu[0],"B":o.gpu[1],"unit_loss":o.gpu[2]}))).collect::<String>())?;
        let report = policy.assess(&plan, &observations)?;
        history.push(json!({"refinement":level,"node_batches":node_batches,"probe_batches":probe_batches,"node_count":plan.nodes().len(),"cases":observations.len(),"failed_samples":report.failed_samples(),"max_relative_errors_A_B_loss":report.max_relative_errors(),"decision":format!("{:?}",report.decision())}));
        match report.decision() {
            Decision::Refine { next } => level = next,
            Decision::BudgetExhausted => return Err("accuracy budget exhausted".into()),
            Decision::SampledPass => {
                let patches = plan.experimental_patch_data(&coefficients)?;
                let atlas = dfg::GgxDfgLut::with_experimental_energy_patches(&device, &patches)?;
                let mut encoder = device.create_command_encoder(&Default::default());
                atlas.encode(&mut encoder);
                queue.submit([encoder.finish()]);
                // Reject a completed result from an old generation; drain it before return.
                let key = Key {
                    generation: 2,
                    refinement: level,
                    offset: 0,
                };
                let mut encoder = device.create_command_encoder(&Default::default());
                let encoded = gpu.encode_nodes(&mut encoder, key, &[[1., 0.]])?;
                queue.submit([encoder.finish()]);
                let mut pending = encoded.submitted();
                let changed = Key {
                    generation: 3,
                    ..key
                };
                loop {
                    device.poll(wgpu::PollType::Poll)?;
                    match pending.try_read(&device, changed) {
                        Ok(None) => std::thread::sleep(std::time::Duration::from_millis(1)),
                        Err(voxy_gpu_core::ComputeError::Validation(message)) => {
                            assert!(message.contains("stale"));
                            break;
                        }
                        other => panic!("unexpected stale result {other:?}"),
                    }
                }
                let state = json!({"status":"direct_gpu_sampled_refinement_passed","adapter":adapter.get_info().name,"history":history,"experimental_patches":patches.len(),"subprocesses":0,"runtime_blocking_waits":0,"consumed_and_stale_results_rejected":true,"whole_cell_error_bound_proven":false,"default_production_selected":false,"scope":"direct renderer GPU APIs with native planning and policy; host harness polls and yields, no frame/FPS claim"});
                fs::write(
                    out.join("progress.json"),
                    serde_json::to_vec_pretty(&state)?,
                )?;
                println!("{state}");
                return Ok(());
            }
        }
    }
}
