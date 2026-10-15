//! End-to-end runtime resource construction and handoff; no subprocesses.
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
impl std::fmt::Display for RendererError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "renderer error: {self:?}")
    }
}
impl std::error::Error for RendererError {}
#[path = "../../crates/voxy_render/src/dfg.rs"]
mod dfg;
use dfg::{
    GgxEnergyAtlasBuild as Build, GgxEnergyAtlasSlot as Slot, GgxEnergyBuildBudget as Budget,
    GgxEnergyBuildError as Error, GgxEnergyBuildPhase as Phase, GgxEnergyGpuPrecompute as Gpu,
    GgxEnergyOracleSample as Oracle, GgxEnergyRefinementPlan as Plan,
    GgxEnergyRefinementPolicy as Policy,
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
fn budget() -> Budget {
    Budget {
        max_nodes: 4096,
        max_probes: 512,
        max_candidate_atlas_bytes: 2 * 1024 * 1024,
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 4, "output reference uncertainty-report");
    let out = PathBuf::from(&args[1]);
    if out.exists() {
        return Err("preserve prior qualification".into());
    }
    fs::create_dir_all(&out)?;
    let references = rows(Path::new(&args[2]))
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
    let previous: Value = serde_json::from_str(&fs::read_to_string(&args[3])?)?;
    let uncertainty = previous["results"]
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
    let plan = Plan::try_new(&regions, 1)?;
    let oracle = plan
        .probes()
        .iter()
        .map(|p| {
            let r = &references[&id(p.uv)]["reference"];
            Oracle {
                uv: p.uv,
                reference: [
                    r["A"].as_f64().unwrap(),
                    r["B"].as_f64().unwrap(),
                    r["loss"].as_f64().unwrap(),
                ],
                uncertainty: uncertainty[&id(p.uv)],
            }
        })
        .collect::<Vec<_>>();
    let instance = GraphicsOptions::default().create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))?;
    let gpu = pollster::block_on(Gpu::new(&device))?;
    let initial = dfg::GgxDfgLut::new(&device, 4)?;
    let mut encoder = device.create_command_encoder(&Default::default());
    initial.encode(&mut encoder);
    queue.submit([encoder.finish()]);
    let initial_texture = initial.output().clone();
    let mut slot = Slot::new(initial, 0);
    slot.request_generation(1)?;
    let mut build = Build::new(
        &device,
        &queue,
        1,
        regions.clone(),
        oracle.clone(),
        Policy::try_new(1e-5, 8)?,
        budget(),
    )?;
    let mut upload_seen = false;
    let mut last = None;
    let mut polls = 0;
    let ready = loop {
        polls += 1;
        let ready = build.poll(&gpu, slot.requested_generation())?;
        let p = build.progress();
        let marker = (p.phase, p.refinement, p.completed_nodes, p.completed_probes);
        if last != Some(marker) {
            fs::write(
                out.join("progress.json"),
                serde_json::to_vec_pretty(
                    &json!({"status":"running","pid":std::process::id(),"phase":format!("{:?}",p.phase),"refinement":p.refinement,"completed_nodes":p.completed_nodes,"completed_probes":p.completed_probes}),
                )?,
            )?;
            last = Some(marker);
        }
        if let Some(ready) = ready {
            break ready;
        }
        upload_seen |= p.phase == Phase::Upload;
        assert_eq!(slot.current().output(), &initial_texture);
        assert_eq!(slot.installed_generation(), 0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    assert!(upload_seen);
    assert_eq!(
        build.progress().completed_nodes,
        build.progress().total_nodes
    );
    assert_eq!(build.progress().completed_probes, 72);
    let history=build.assessments().iter().map(|(level,report)|json!({"refinement":level,"decision":format!("{:?}",report.decision()),"max_relative_errors_A_B_loss":report.max_relative_errors(),"failed_samples":report.failed_samples()})).collect::<Vec<_>>();
    assert!(matches!(build.poll(&gpu, 1), Err(Error::Consumed)));
    let old = slot.try_install(ready)?;
    assert_eq!(old.output(), &initial_texture);
    assert_eq!(slot.installed_generation(), 1);
    let installed = slot.current().output().clone();
    // A complete but obsolete candidate may never replace the current resource.
    slot.request_generation(2)?;
    let small = Plan::regions_from_seeds(&seeds[..1])?;
    let mut obsolete = Build::new(
        &device,
        &queue,
        2,
        small.clone(),
        oracle[..9].to_vec(),
        Policy::try_new(1e-5, 8)?,
        budget(),
    )?;
    let stale_ready = loop {
        if let Some(ready) = obsolete.poll(&gpu, 2)? {
            break ready;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    slot.request_generation(3)?;
    assert!(matches!(
        slot.try_install(stale_ready),
        Err(Error::StaleGeneration)
    ));
    assert_eq!(slot.current().output(), &installed);
    assert_eq!(slot.installed_generation(), 1);
    // Cancellation with an actual queued GPU packet also leaves the owner intact.
    let mut cancelled = Build::new(
        &device,
        &queue,
        3,
        small,
        oracle[..9].to_vec(),
        Policy::try_new(1e-5, 8)?,
        budget(),
    )?;
    assert!(cancelled.poll(&gpu, 3)?.is_none());
    slot.request_generation(4)?;
    assert!(matches!(
        cancelled.poll(&gpu, 4),
        Err(Error::StaleGeneration)
    ));
    assert_eq!(cancelled.progress().completed_nodes, 0);
    assert_eq!(slot.current().output(), &installed);
    let (tx, rx) = std::sync::mpsc::channel();
    queue.on_submitted_work_done(move || {
        let _ = tx.send(());
    });
    loop {
        device.poll(wgpu::PollType::Poll)?;
        if rx.try_recv().is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let state = json!({"status":"runtime_atlas_build_and_owner_handoff_passed","adapter":adapter.get_info().name,"history":history,"fixed_probes":72,"poll_calls":polls,"upload_phase_observed_before_ready":upload_seen,"old_owner_preserved_until_ready":true,"stale_complete_candidate_rejected":true,"in_flight_cancel_preserves_owner":true,"installed_generation":slot.installed_generation(),"requested_generation":slot.requested_generation(),"runtime_blocking_waits":0,"subprocesses":0,"scene_bindings_mutated":false,"whole_cell_error_bound_proven":false,"default_production_selected":false});
    fs::write(
        out.join("progress.json"),
        serde_json::to_vec_pretty(&state)?,
    )?;
    println!("{state}");
    Ok(())
}
