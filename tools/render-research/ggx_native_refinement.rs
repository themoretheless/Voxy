//! Native renderer planning and policy driving real GPU qualification tools.
//! Uses an existing independent f64 oracle, not production whole-cell admission.
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
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
    GgxEnergyRefinementDecision as Decision, GgxEnergyRefinementObservation as Observation,
    GgxEnergyRefinementPlan as Plan, GgxEnergyRefinementPolicy as Policy,
};
fn key(n: f32, r: f32) -> [u32; 2] {
    [n.to_bits(), r.to_bits()]
}
fn read_rows(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn run(
    executable: &Path,
    args: &[String],
    out: &Path,
    err: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new(executable)
        .args(args)
        .stdout(fs::File::create(out)?)
        .stderr(fs::File::create(err)?)
        .status()?;
    if !status.success() {
        return Err(format!("qualification tool failed: {status}; see {}", err.display()).into());
    }
    Ok(())
}
fn execute(created: &mut Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(
        args.len(),
        6,
        "output moments lookup independent-reference prior-comparison"
    );
    let out = PathBuf::from(&args[1]);
    if out.exists() {
        return Err("preserve existing qualification; choose a fresh output directory".into());
    }
    fs::create_dir_all(&out)?;
    *created = Some(out.clone());
    let references = read_rows(Path::new(&args[4]))
        .into_iter()
        .filter(|r| r.get("nv").is_some())
        .map(|r| {
            (
                key(
                    r["nv"].as_f64().unwrap() as f32,
                    r["roughness"].as_f64().unwrap() as f32,
                ),
                r,
            )
        })
        .collect::<HashMap<_, _>>();
    let prior: Value = serde_json::from_str(&fs::read_to_string(&args[5])?)?;
    let uncertainties = prior["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                key(
                    r["nv"].as_f64().unwrap() as f32,
                    r["roughness"].as_f64().unwrap() as f32,
                ),
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
    let policy = Policy::try_new(1e-5, 8)?;
    let mut level = 1;
    let mut history = Vec::new();
    loop {
        let plan = Plan::try_new(&regions, level)?;
        assert_eq!(plan.probes().len(), references.len());
        let manifest = json!({"refinement":plan.refinement(),"node_count":plan.nodes().len(),"cases":plan.probes().iter().map(|p|json!({"nv":p.uv[0],"roughness":p.uv[1],"xs":p.view_nodes,"ys":p.roughness_nodes,"bounds":p.bounds})).collect::<Vec<_>>()});
        let manifest_path = out.join(format!("level-{level}.json"));
        fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
        let query_path = out.join(format!("level-{level}.txt"));
        fs::write(
            &query_path,
            plan.nodes()
                .iter()
                .map(|p| format!("{} {}\n", p[0], p[1]))
                .collect::<String>(),
        )?;
        fs::write(
            out.join("progress.json"),
            serde_json::to_vec_pretty(
                &json!({"status":"running","pid":std::process::id(),"level":level,"stage":"gpu_nodes","history":history}),
            )?,
        )?;
        let node_path = out.join(format!("nodes-{level}.jsonl"));
        run(
            Path::new(&args[2]),
            &[
                "524288".to_owned(),
                format!("--query-file={}", query_path.display()),
                "--batch-size=32".to_owned(),
            ],
            &node_path,
            &out.join(format!("nodes-{level}.stderr")),
        )?;
        let lookup_path = out.join(format!("lookup-{level}.jsonl"));
        run(
            Path::new(&args[3]),
            &[
                manifest_path.to_string_lossy().into(),
                node_path.to_string_lossy().into(),
            ],
            &lookup_path,
            &out.join(format!("lookup-{level}.stderr")),
        )?;
        let lookup = read_rows(&lookup_path)
            .into_iter()
            .filter(|r| r.get("nv").is_some())
            .map(|r| {
                (
                    key(
                        r["nv"].as_f64().unwrap() as f32,
                        r["roughness"].as_f64().unwrap() as f32,
                    ),
                    r,
                )
            })
            .collect::<HashMap<_, _>>();
        assert_eq!(lookup.len(), plan.probes().len());
        let observations = plan
            .probes()
            .iter()
            .map(|p| {
                let id = key(p.uv[0], p.uv[1]);
                let gpu = &lookup[&id];
                let reference = &references[&id]["reference"];
                Observation {
                    uv: p.uv,
                    gpu: [
                        gpu["A"].as_f64().unwrap() as f32,
                        gpu["B"].as_f64().unwrap() as f32,
                        gpu["unit_loss"].as_f64().unwrap() as f32,
                        if gpu["valid"].as_bool().unwrap() {
                            1.
                        } else {
                            0.
                        },
                    ],
                    reference: [
                        reference["A"].as_f64().unwrap(),
                        reference["B"].as_f64().unwrap(),
                        reference["loss"].as_f64().unwrap(),
                    ],
                    uncertainty: uncertainties[&id],
                }
            })
            .collect::<Vec<_>>();
        let report = policy.assess(&plan, &observations)?;
        history.push(json!({"refinement":level,"node_count":plan.nodes().len(),"cases":plan.probes().len(),"failed_samples":report.failed_samples(),"max_relative_errors_A_B_loss":report.max_relative_errors(),"decision":format!("{:?}",report.decision())}));
        match report.decision() {
            Decision::Refine { next } => {
                level = next;
            }
            Decision::SampledPass => {
                let moments = read_rows(&node_path)
                    .into_iter()
                    .filter(|r| r.get("nv").is_some())
                    .map(|r| {
                        (
                            key(
                                r["nv"].as_f64().unwrap() as f32,
                                r["roughness"].as_f64().unwrap() as f32,
                            ),
                            [
                                r["A"].as_f64().unwrap() as f32,
                                r["B"].as_f64().unwrap() as f32,
                                r["unit_loss"].as_f64().unwrap() as f32,
                                1.,
                            ],
                        )
                    })
                    .collect::<HashMap<_, _>>();
                assert_eq!(moments.len(), plan.nodes().len());
                let coefficients = plan
                    .nodes()
                    .iter()
                    .map(|p| moments[&key(p[0], p[1])])
                    .collect::<Vec<_>>();
                let patches = plan.experimental_patch_data(&coefficients)?;
                let state = json!({"status":"native_sampled_refinement_passed","history":history,"experimental_patches":patches.len(),"whole_cell_error_bound_proven":false,"default_production_selected":false,"scope":"native renderer planner and policy, real GPU nodes/interpolation, existing independent f64 oracle"});
                fs::write(
                    out.join("progress.json"),
                    serde_json::to_vec_pretty(&state)?,
                )?;
                println!("{state}");
                return Ok(());
            }
            Decision::BudgetExhausted => {
                fs::write(
                    out.join("progress.json"),
                    serde_json::to_vec_pretty(
                        &json!({"status":"budget_exhausted","history":history}),
                    )?,
                )?;
                return Err("sampled accuracy not admitted".into());
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut created = None;
    let result = execute(&mut created);
    if let (Some(out), Err(error)) = (&created, &result) {
        let path = out.join("progress.json");
        let mut state = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .unwrap_or_else(|| json!({}));
        if state["status"] != "budget_exhausted" {
            state["status"] = json!("failed");
        }
        state["error"] = json!(error.to_string());
        fs::write(path, serde_json::to_vec_pretty(&state)?)?;
    }
    result
}
