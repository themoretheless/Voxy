//! Reference-atlas organ mesh with an explicitly synthetic passive HGO law.
//! Numerical clamps/loads are not anatomical attachments or human calibration.
use physics::biomechanics::*;
use std::{fs, io::Write, path::Path};
fn export(root: &Path, stage: &str, body: &Body, faces: &[[usize; 3]]) -> std::io::Result<()> {
    let mut file = fs::File::create(root.join(format!("{stage}.obj")))?;
    for p in body.positions() {
        writeln!(file, "v {:.17e} {:.17e} {:.17e}", p[0], p[1], p[2])?;
    }
    for f in faces {
        writeln!(file, "f {} {} {}", f[0] + 1, f[1] + 1, f[2] + 1)?;
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let source = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("assets/anatomy/hra-female/tetrahedra/right-ovary.vxtet");
    let output = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("/tmp/voxy-anatomy-hgo-right-ovary");
    let root = Path::new(output);
    let progress = args.iter().any(|a| a == "--progress");
    let substeps: usize = args
        .iter()
        .find_map(|a| a.strip_prefix("--time-substeps="))
        .unwrap_or("1")
        .parse()?;
    if !(1..=16).contains(&substeps) {
        return Err("time subdivisions must be 1..16".into());
    }
    fs::create_dir_all(root)?;
    let mut mesh = TetraMesh::from_bytes(&fs::read(source)?)?;
    let surface_patch = args.iter().any(|a| a == "--surface-patch");
    let refinements: usize = args
        .iter()
        .find_map(|a| a.strip_prefix("--refine="))
        .unwrap_or("0")
        .parse()?;
    if refinements > 2 || (refinements > 0 && !surface_patch) {
        return Err("refinement requires a fixed surface patch and at most two levels".into());
    }
    let low: Vec3 = std::array::from_fn(|a| {
        mesh.points
            .iter()
            .map(|p| p[a])
            .fold(f64::INFINITY, f64::min)
    });
    let high: Vec3 = std::array::from_fn(|a| {
        mesh.points
            .iter()
            .map(|p| p[a])
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let axis = (0..3)
        .max_by(|&a, &b| (high[a] - low[a]).total_cmp(&(high[b] - low[b])))
        .unwrap();
    let extent = high[axis] - low[axis];
    let inherited_clamp = args.iter().any(|a| a == "--inherited-clamp");
    let mut pins: Vec<_> = mesh
        .points
        .iter()
        .map(|p| p[axis] <= low[axis] + 0.05 * extent)
        .collect();
    let mut patch: Vec<_> = mesh
        .boundary
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.iter()
                .all(|&i| mesh.points[i][axis] >= high[axis] - 0.05 * extent)
        })
        .map(|(i, _)| i)
        .collect();
    for level in 0..refinements {
        let (refined, parents) = mesh.refined_once_with_parents()?;
        pins = if inherited_clamp {
            parents.iter().map(|&[a, b]| pins[a] && pins[b]).collect()
        } else {
            refined
                .points
                .iter()
                .map(|p| p[axis] <= low[axis] + 0.05 * extent)
                .collect()
        };
        let mut file = fs::File::create(root.join(format!("vertex-parents-{level}.csv")))?;
        writeln!(file, "node,parent_a,parent_b")?;
        for (i, [a, b]) in parents.iter().enumerate() {
            writeln!(file, "{i},{a},{b}")?;
        }
        mesh = refined;
        patch = patch
            .into_iter()
            .flat_map(|i| [4 * i, 4 * i + 1, 4 * i + 2, 4 * i + 3])
            .collect();
    }
    let faces = mesh.boundary.clone();
    let points = mesh.points.len();
    let cells = mesh.cells.len();
    let pinned_nodes: Vec<_> = pins
        .iter()
        .enumerate()
        .filter(|(_, p)| **p)
        .map(|(i, _)| i)
        .collect();
    fs::write(
        root.join("clamp-mode.txt"),
        format!("inherited_clamp={inherited_clamp}\npins={pinned_nodes:?}\n"),
    )?;
    let pinned_count = pins.iter().filter(|&&x| x).count();
    let targets: Vec<_> = mesh
        .points
        .iter()
        .enumerate()
        .filter(|(_, p)| p[axis] >= high[axis] - 0.05 * extent)
        .map(|(i, _)| i)
        .collect();
    let weights = if surface_patch {
        mesh.reference_patch_weights(&patch)?
    } else {
        let mut w = vec![0.; points];
        for &i in &targets {
            w[i] = 1.;
        }
        w
    };
    let total_weight = weights.iter().sum::<f64>();
    let targets: Vec<_> = weights
        .iter()
        .enumerate()
        .filter(|(_, w)| **w > 0.)
        .map(|(i, _)| i)
        .collect();
    fs::write(
        root.join("load-profile.txt"),
        format!(
            "surface_patch={surface_patch}\nrefinements={refinements}\npatch_faces={patch:?}\nweights={weights:?}\ntotal_weight={total_weight:.17e}\n"
        ),
    )?;
    let mut direction = [0.; 3];
    direction[axis] = 1.;
    let elastic = HgoMaterial {
        shear_pa: 3000.,
        bulk_pa: 50000.,
        fibers: vec![Fiber {
            direction,
            stiffness_pa: 9000.,
            exponent: 0.2,
            active_pa: 0.,
        }],
    };
    let mut body = mesh.into_body(
        pins,
        &Material {
            shear_pa: 3000.,
            bulk_pa: 50000.,
            fibers: vec![],
        },
    )?;
    let law = ViscoelasticHgo::new(elastic, &[(0.2, 0.3), (2., 0.5)])?;
    body.set_viscoelastic_hgo_batch(&(0..cells).map(|i| (i, law.clone())).collect::<Vec<_>>())?;
    fs::write(
        root.join("material-profile.txt"),
        format!(
            "shear_pa=3000\nbulk_pa=50000\nk1_pa=9000\nk2=0.2\nfiber_axis={axis}\ntau_s=0.2,2\nbeta=0.3,0.5\ncalibrated=false\n"
        ),
    )?;
    fs::write(
        root.join("force-targets.txt"),
        format!(
            "axis={axis}\ntargets={targets:?}\npins={:?}\n",
            pinned_nodes
        ),
    )?;
    fs::write(root.join("time-substeps.txt"), format!("{substeps}\n"))?;
    export(root, "rest", &body, &faces)?;
    fs::write(
        root.join("scope.txt"),
        format!(
            "source={source}\npoints={points}\ncells={cells}\naxis={axis}\npinned_nodes={pinned_count}\nloaded_nodes={}\ncalibrated=false\nregistered_to_body=false\nclamp_mode_from_clamp_mode_file=true\nload_mode_from_load_profile=true;total_force_N=0.001\nnominal_dt_s=0.1\n",
            targets.len()
        ),
    )?;
    println!("stage,physical_time_s,force_n,max_displacement_m,residual_n");
    for (step, (stage, force)) in [
        ("loaded", 0.001),
        ("hold-1", 0.001),
        ("hold-2", 0.001),
        ("released", 0.),
        ("recover-1", 0.),
        ("recover-2", 0.),
    ]
    .into_iter()
    .enumerate()
    {
        for &node in &targets {
            body.set_force(
                node,
                direction.map(|x| x * force * weights[node] / total_weight),
            )?;
        }
        for substep in 1..=substeps {
            let report = body.relax_step_lbfgs_states(
                0.1 / substeps as f64,
                10000,
                1e-7,
                |iteration, energy, residual, line_step, rejected, increment, _| {
                    if progress && (iteration % 100 == 0 || residual <= 1e-7) {
                        eprintln!(
                            "stage={stage} substep={substep}/{substeps} iteration={iteration} energy_J={energy:.12e} residual_N={residual:.12e} line_step={line_step:.12e} rejected_trials={rejected} iteration_displacement_m={increment:.12e}"
                        );
                    }
                },
            )?;
            let displacement = body
                .positions()
                .iter()
                .zip(body.rest_positions())
                .map(|(a, b)| (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt())
                .fold(0_f64, f64::max);
            let stage = if substep == substeps {
                stage.to_string()
            } else {
                format!("{stage}-sub-{substep}")
            };
            export(root, &stage, &body, &faces)?;
            println!(
                "{stage},{:.8},{force:.12e},{displacement:.12e},{:.12e}",
                (step as f64 + substep as f64 / substeps as f64) * 0.1,
                report.residual_n
            );
        }
    }
    Ok(())
}
