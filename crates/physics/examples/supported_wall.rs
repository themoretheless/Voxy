//! Synthetic wall and deformable surrounding tissue; not a calibrated pelvic model.
use physics::biomechanics::*;
use std::io::Write;

fn export(body: &Body, path: &str) -> std::io::Result<()> {
    export_coordinates(body.positions(), &body.surface(), path)
}
fn export_coordinates(points: &[Vec3], faces: &[[usize; 3]], path: &str) -> std::io::Result<()> {
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    for p in points {
        writeln!(file, "v {} {} {}", p[0], p[1], p[2])?;
    }
    for f in faces {
        writeln!(file, "f {} {} {}", f[0] + 1, f[1] + 1, f[2] + 1)?;
    }
    Ok(())
}

// Average all cap fans: avoids privileging an arbitrary ring vertex when
// pressure acts on a deformed/nonplanar virtual end. These are benchmark hulls.
fn install_pressure_hull(
    body: &mut Body,
    side: Vec<[usize; 3]>,
    ends: [Vec<usize>; 2],
) -> Result<Vec<usize>, &'static str> {
    let count = ends[0].len();
    if count < 3 || ends[1].len() != count {
        return Err("invalid hull end rings");
    }
    let mut indices = Vec::new();
    for anchor in 0..count {
        let mut faces = side.clone();
        for k in 1..count - 1 {
            faces.push([
                ends[0][anchor],
                ends[0][(anchor + k + 1) % count],
                ends[0][(anchor + k) % count],
            ]);
            faces.push([
                ends[1][anchor],
                ends[1][(anchor + k) % count],
                ends[1][(anchor + k + 1) % count],
            ]);
        }
        indices.push(body.cavities().len());
        body.add_gauge_pressure_cavity(Cavity {
            faces,
            pressure_pa: 0.,
        })?;
    }
    Ok(indices)
}
fn set_hull_pressure(
    body: &mut Body,
    indices: &[usize],
    pressure: f64,
) -> Result<(), &'static str> {
    for &i in indices {
        body.set_gauge_pressure(i, -pressure / indices.len() as f64)?;
    }
    Ok(())
}
fn assign_viscous_layers(
    body: &mut Body,
    rings: usize,
    sectors: usize,
    base: &Material,
    scales: &[f64],
    times: [f64; 2],
    tissue: &str,
    profile: &mut impl Write,
) -> Result<(), Box<dyn std::error::Error>> {
    if scales.len() != rings - 1 {
        return Err("invalid viscous layer count".into());
    }
    let layers: Vec<_> = body
        .elements()
        .iter()
        .map(|e| {
            e.nodes
                .iter()
                .map(|i| (i % (rings * sectors)) / sectors)
                .min()
                .unwrap()
        })
        .collect();
    let mut assignments = Vec::new();
    for layer in 0..scales.len() {
        let mu = base.shear_pa * scales[layer];
        let multiplier = 1. + 0.5 * layer as f64;
        let law = ViscoelasticOgden::new(
            vec![OgdenTerm {
                shear_pa: 0.5 * mu,
                exponent: 2.,
            }],
            base.bulk_pa,
            vec![
                MaxwellBranch {
                    shear_pa: 0.3 * mu,
                    relaxation_seconds: times[0] * multiplier,
                },
                MaxwellBranch {
                    shear_pa: 0.2 * mu,
                    relaxation_seconds: times[1] * multiplier,
                },
            ],
        )?;
        writeln!(
            profile,
            "{tissue},{layer},{mu},{},{},{},{},{},{}",
            0.5 * mu,
            base.bulk_pa,
            0.3 * mu,
            times[0] * multiplier,
            0.2 * mu,
            times[1] * multiplier
        )?;
        for (i, l) in layers.iter().enumerate() {
            if *l == layer {
                assignments.push((i, law.clone()));
            }
        }
    }
    if assignments.len() != layers.len() {
        return Err("unassigned viscous element".into());
    }
    body.set_viscoelastic_ogden_batch(&assignments)?;
    Ok(())
}
fn refine_schedule(stages: Vec<(String, f64)>, substeps: usize) -> Vec<(String, f64)> {
    stages
        .into_iter()
        .flat_map(|(stage, factor)| {
            (1..=substeps).map(move |i| {
                (
                    if i == substeps {
                        stage.clone()
                    } else {
                        format!("{stage}-sub-{i}")
                    },
                    factor,
                )
            })
        })
        .collect()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or("/tmp/voxy-supported-wall".into());
    std::fs::create_dir_all(&output)?;
    let probe = std::env::args().find_map(|a| a.strip_prefix("--probe-from=").map(str::to_owned));
    let resume = std::env::args().find_map(|a| a.strip_prefix("--resume-from=").map(str::to_owned));
    let resumed = resume.is_some();
    if probe.is_some() && resume.is_some() {
        return Err("choose probe or resume".into());
    }
    let support_pressure: f64 = std::env::args()
        .find_map(|a| a.strip_prefix("--support-pressure=").map(str::to_owned))
        .unwrap_or("0".into())
        .parse()?;
    if !support_pressure.is_finite() || support_pressure < 0. {
        return Err("support pressure must be finite and nonnegative".into());
    }
    let increments: usize = std::env::args()
        .find_map(|a| a.strip_prefix("--load-increments=").map(str::to_owned))
        .unwrap_or("1".into())
        .parse()?;
    if !(1..=32).contains(&increments) {
        return Err("load increments must be 1..=32".into());
    }
    let max_iterations: usize = std::env::args()
        .find_map(|a| a.strip_prefix("--max-iterations=").map(str::to_owned))
        .unwrap_or("100000".into())
        .parse()?;
    if !(1..=100000).contains(&max_iterations) {
        return Err("max iterations must be 1..=100000".into());
    }
    let contact_law = match std::env::args()
        .find_map(|a| a.strip_prefix("--contact-law=").map(str::to_owned))
        .as_deref()
    {
        None | Some("triangle") => SurfaceContactLaw::TriangleMinimum,
        Some("primitive") => SurfaceContactLaw::ExperimentalPrimitiveSum,
        _ => return Err("contact law must be triangle or primitive".into()),
    };
    let snapshot_every: usize = std::env::args()
        .find_map(|a| a.strip_prefix("--snapshot-every=").map(str::to_owned))
        .unwrap_or("1000".into())
        .parse()?;
    let contact_preconditioner = std::env::args().any(|a| a == "--contact-preconditioner");
    if contact_preconditioner && contact_law != SurfaceContactLaw::ExperimentalPrimitiveSum {
        return Err("contact preconditioner requires primitive law".into());
    }
    std::fs::write(
        format!("{output}/preconditioner.txt"),
        format!("contact_curvature={contact_preconditioner}\n"),
    )?;
    let load_law = std::env::args()
        .find_map(|a| a.strip_prefix("--load-law=").map(str::to_owned))
        .unwrap_or("reference-lateral".into());
    if !["reference-lateral", "closed-reference", "closed-follower"].contains(&load_law.as_str()) {
        return Err(
            "load law must be reference-lateral, closed-reference or closed-follower".into(),
        );
    }
    std::fs::write(format!("{output}/load-law.txt"), format!("{load_law}\n"))?;
    let viscoelastic = std::env::args().any(|a| a == "--viscoelastic");
    let dt: f64 = std::env::args()
        .find_map(|a| a.strip_prefix("--dt=").map(str::to_owned))
        .unwrap_or("0.1".into())
        .parse()?;
    let time_substeps: usize = std::env::args()
        .find_map(|a| a.strip_prefix("--time-substeps=").map(str::to_owned))
        .unwrap_or("1".into())
        .parse()?;
    if !(1..=16).contains(&time_substeps) || (!viscoelastic && time_substeps != 1) {
        return Err("time substeps require viscous mode and must be 1..=16".into());
    }
    let physical_dt = dt / time_substeps as f64;
    if !physical_dt.is_finite() || physical_dt <= 0. {
        return Err("invalid subdivided physical timestep".into());
    }
    let hold_steps: usize = std::env::args()
        .find_map(|a| a.strip_prefix("--hold-steps=").map(str::to_owned))
        .unwrap_or(if viscoelastic { "5" } else { "0" }.into())
        .parse()?;
    let recovery_steps: usize = std::env::args()
        .find_map(|a| a.strip_prefix("--recovery-steps=").map(str::to_owned))
        .unwrap_or(if viscoelastic { "10" } else { "0" }.into())
        .parse()?;
    if !dt.is_finite() || dt <= 0. || hold_steps > 100 || recovery_steps > 100 {
        return Err("invalid physical time schedule".into());
    }
    if !viscoelastic && (hold_steps > 0 || recovery_steps > 0) {
        return Err("time schedule requires viscoelastic mode".into());
    }
    if viscoelastic && (resumed || probe.is_some() || contact_preconditioner) {
        return Err(
            "viscoelastic mode requires a fresh reference and standard L-BFGS preconditioner"
                .into(),
        );
    }
    std::fs::write(
        format!("{output}/material-mode.txt"),
        format!(
            "viscoelastic={viscoelastic}\ncalibrated=false\ndt_s={dt}\ntime_substeps={time_substeps}\nactual_dt_s={physical_dt}\nhold_steps={hold_steps}\nrecovery_steps={recovery_steps}\n"
        ),
    )?;
    let sectors = 16;
    let segments = 6;
    let material = Material::from_young_poisson(8000., 0.3)?;
    let mut wall = elliptical_tube(
        &[0.008, 0.009, 0.010, 0.011, 0.012],
        0.03,
        sectors,
        segments,
        &vec![material.clone(); 4],
        true,
        [1., 0.3],
    )?;
    let mut support = elliptical_tube(
        &[0.0125, 0.014, 0.016],
        0.03,
        sectors,
        segments,
        &vec![material.clone(); 2],
        true,
        [1., 0.3],
    )?;
    if viscoelastic {
        let mut profile = std::fs::File::create(format!("{output}/material-profile.csv"))?;
        writeln!(
            profile,
            "tissue,layer,instant_shear_pa,equilibrium_shear_pa,bulk_pa,branch_0_shear_pa,branch_0_tau_s,branch_1_shear_pa,branch_1_tau_s"
        )?;
        assign_viscous_layers(
            &mut wall,
            5,
            sectors,
            &material,
            &[0.75, 1., 1.25, 0.8],
            [0.2, 2.],
            "wall",
            &mut profile,
        )?;
        assign_viscous_layers(
            &mut support,
            3,
            sectors,
            &material,
            &[0.6, 0.9],
            [0.5, 5.],
            "support",
            &mut profile,
        )?;
    }
    let mut assembly = Body::assemble_tissues(&[wall.clone(), support.clone()])?;
    let offset = assembly.node_ranges[1].start;
    let faces: Vec<_> = wall
        .surface()
        .into_iter()
        .filter(|f| f.iter().all(|i| i % (5 * sectors) >= 4 * sectors))
        .collect();
    let mut areas = vec![0.; wall.positions().len()];
    let mut forces = vec![[0.; 3]; wall.positions().len()];
    for face in faces {
        let [a, b, c] = face.map(|i| wall.rest_positions()[i]);
        let u: [f64; 3] = std::array::from_fn(|k| b[k] - a[k]);
        let v: [f64; 3] = std::array::from_fn(|k| c[k] - a[k]);
        let normal = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let area = normal.iter().map(|n| n * n).sum::<f64>().sqrt() / 2.;
        for node in face {
            areas[node] += area / 3.;
            for k in 0..3 {
                forces[node][k] -= 20. * normal[k] / 6.;
            }
        }
    }
    let mut support_forces = vec![[0.; 3]; support.positions().len()];
    for face in support
        .surface()
        .into_iter()
        .filter(|f| f.iter().all(|i| i % (3 * sectors) >= 2 * sectors))
    {
        let [a, b, c] = face.map(|i| support.rest_positions()[i]);
        let u: [f64; 3] = std::array::from_fn(|k| b[k] - a[k]);
        let v: [f64; 3] = std::array::from_fn(|k| c[k] - a[k]);
        let normal = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for node in face {
            for k in 0..3 {
                support_forces[node][k] -= support_pressure * normal[k] / 6.;
            }
        }
    }
    // Surface-density stiffness (N/m^3) times lumped reference area gives N/m.
    let mut bonds = Vec::new();
    for z in 0..=segments {
        for a in 0..sectors {
            let node = z * 5 * sectors + 4 * sectors + a;
            bonds.push(([node, offset + z * 3 * sectors + a], 1e7 * areas[node]));
        }
    }
    assembly.body.add_tissue_bonds(&bonds)?;
    // One combined group includes cross-tissue pairs as well as self-contact.
    // Separate groups would omit all wall/support collision pairs.
    assembly
        .body
        .set_surface_contacts(vec![TissueSurfaceContact {
            faces: assembly.body.surface(),
            minimum_distance_m: 1e-6,
            activation_gap_m: 5e-5,
            pair_stiffness_n_m: 1.,
        }])?;
    assembly.body.set_surface_contact_law(contact_law)?;
    std::fs::write(
        format!("{output}/contact-law.txt"),
        format!("{contact_law:?}\n"),
    )?;
    let mut hulls = [Vec::new(), Vec::new()];
    if load_law != "reference-lateral" {
        for (tissue, (part, rings, node_offset)) in [(&wall, 5, 0), (&support, 3, offset)]
            .into_iter()
            .enumerate()
        {
            let side = part
                .surface()
                .into_iter()
                .filter(|f| {
                    f.iter()
                        .all(|i| i % (rings * sectors) >= (rings - 1) * sectors)
                })
                .map(|f| f.map(|i| i + node_offset))
                .collect();
            let ends = [0, segments].map(|z| {
                (0..sectors)
                    .map(|a| node_offset + z * rings * sectors + (rings - 1) * sectors + a)
                    .collect()
            });
            hulls[tissue] = install_pressure_hull(&mut assembly.body, side, ends)?;
        }
        if load_law == "closed-reference" {
            let reference = assembly.body.rest_positions().to_vec();
            let base = assembly.body.evaluate(&reference)?.1;
            set_hull_pressure(&mut assembly.body, &hulls[0], 20.)?;
            set_hull_pressure(&mut assembly.body, &hulls[1], support_pressure)?;
            let pressure = assembly.body.evaluate(&reference)?.1;
            for i in 0..forces.len() {
                forces[i] = std::array::from_fn(|k| base[i][k] - pressure[i][k]);
            }
            for i in 0..support_forces.len() {
                support_forces[i] =
                    std::array::from_fn(|k| base[offset + i][k] - pressure[offset + i][k]);
            }
            for indices in &hulls {
                set_hull_pressure(&mut assembly.body, indices, 0.)?;
            }
        } else {
            forces.fill([0.; 3]);
            support_forces.fill([0.; 3]);
        }
    }
    if let Some(path) = resume {
        let text = std::fs::read_to_string(path)?;
        let mut points = Vec::new();
        let mut faces = Vec::new();
        for line in text.lines() {
            let a: Vec<_> = line.split_whitespace().collect();
            if a.first() == Some(&"v") && a.len() == 4 {
                points.push([a[1].parse()?, a[2].parse()?, a[3].parse()?]);
            } else if a.first() == Some(&"f") && a.len() == 4 {
                faces.push([
                    a[1].parse::<usize>()?.checked_sub(1).ok_or("bad face")?,
                    a[2].parse::<usize>()?.checked_sub(1).ok_or("bad face")?,
                    a[3].parse::<usize>()?.checked_sub(1).ok_or("bad face")?,
                ]);
            }
        }
        if faces != assembly.body.surface() {
            return Err("resume topology mismatch".into());
        }
        assembly.body.restore_diagnostic_positions(&points)?;
    }
    export(
        &assembly.body,
        &format!("{output}/{}.obj", if resumed { "imported" } else { "rest" }),
    )?;
    println!(
        "stage,residual_n,min_j,wall_max_displacement_m,support_max_displacement_m,minimum_surface_distance_m,contact_energy_j,cross_tissue_contact_energy_j,support_pressure_pa,load_factor,applied_support_pressure_pa,contact_law,load_law,physical_time_s"
    );
    let mut stages = Vec::new();
    for step in 1..=increments {
        stages.push((
            if step == increments {
                "loaded".into()
            } else {
                format!("load-{step}")
            },
            step as f64 / increments as f64,
        ));
    }
    for step in 1..=hold_steps {
        stages.push((format!("hold-{step}"), 1.));
    }
    for step in 1..=increments {
        stages.push((
            if step == increments {
                "released".into()
            } else {
                format!("unload-{step}")
            },
            1. - step as f64 / increments as f64,
        ));
    }
    for step in 1..=recovery_steps {
        stages.push((format!("recover-{step}"), 0.));
    }
    let stages = refine_schedule(stages, time_substeps);
    let mut time_s = 0.;
    for (stage, factor) in stages {
        let previous_committed = viscoelastic.then(|| assembly.body.clone());
        let step_end_s = time_s + if viscoelastic { physical_dt } else { 0. };

        if load_law == "closed-follower" {
            set_hull_pressure(&mut assembly.body, &hulls[0], 20. * factor)?;
            set_hull_pressure(&mut assembly.body, &hulls[1], support_pressure * factor)?;
        }

        for (i, f) in forces.iter().enumerate() {
            assembly.body.set_force(i, f.map(|x| x * factor))?;
        }
        for (i, f) in support_forces.iter().enumerate() {
            assembly.body.set_force(offset + i, f.map(|x| x * factor))?;
        }
        if let Some(path) = &probe {
            let mut points = Vec::new();
            let mut faces = Vec::new();
            for line in std::fs::read_to_string(path)?.lines() {
                let a: Vec<_> = line.split_whitespace().collect();
                if a.first() == Some(&"v") && a.len() == 4 {
                    points.push([a[1].parse()?, a[2].parse()?, a[3].parse()?]);
                } else if a.first() == Some(&"f") && a.len() == 4 {
                    faces.push([
                        a[1].parse::<usize>()?.checked_sub(1).ok_or("bad face")?,
                        a[2].parse::<usize>()?.checked_sub(1).ok_or("bad face")?,
                        a[3].parse::<usize>()?.checked_sub(1).ok_or("bad face")?,
                    ]);
                }
            }
            if faces != assembly.body.surface() {
                return Err("probe topology mismatch".into());
            }
            let curvature = assembly.body.surface_primitive_curvature_at(&points)?;
            let mut curvature_file = std::io::BufWriter::new(std::fs::File::create(format!(
                "{output}/contact-curvature.csv"
            ))?);
            writeln!(curvature_file, "node,normal_contact_curvature_n_m,pinned")?;
            for (i, c) in curvature.iter().enumerate() {
                writeln!(
                    curvature_file,
                    "{i},{c:.12e},{}",
                    assembly.body.rest_positions()[i][2] == 0.
                )?;
            }
            let stencils = assembly.body.surface_primitive_stencils_at(&points)?;
            let mut topology = std::io::BufWriter::new(std::fs::File::create(format!(
                "{output}/primitive-stencils.csv"
            ))?);
            writeln!(topology, "kind,group,n0,n1,n2,n3")?;
            for stencil in stencils {
                match stencil {
                    SurfacePrimitive::VertexFace {
                        group,
                        vertex,
                        face,
                    } => writeln!(
                        topology,
                        "vertex_face,{group},{vertex},{},{},{}",
                        face[0], face[1], face[2]
                    )?,
                    SurfacePrimitive::EdgeEdge { group, edges } => writeln!(
                        topology,
                        "edge_edge,{group},{},{},{},{}",
                        edges[0][0], edges[0][1], edges[1][0], edges[1][1]
                    )?,
                }
            }
            let (_, gradient) = assembly.body.evaluate(&points)?;
            let mut components = Vec::new();
            for (node, g) in gradient.iter().enumerate() {
                if assembly.body.rest_positions()[node][2] == 0. {
                    continue;
                }
                for axis in 0..3 {
                    components.push((node, axis, g[axis]));
                }
            }
            components.sort_by(|a, b| b.2.abs().total_cmp(&a.2.abs()));
            let mut derivatives = std::io::BufWriter::new(std::fs::File::create(format!(
                "{output}/gradient-probe.csv"
            ))?);
            writeln!(
                derivatives,
                "node,axis,step_m,analytic_n,central_n,left_n,right_n"
            )?;
            let energy = assembly.body.evaluate(&points)?.0;
            let (mesh_primitive_energy, mesh_primitive_gradient) =
                assembly.body.surface_primitive_energy_at(&points)?;
            let mut mesh_probe = std::io::BufWriter::new(std::fs::File::create(format!(
                "{output}/mesh-primitive-probe.csv"
            ))?);
            writeln!(
                mesh_probe,
                "node,axis,step_m,analytic_n,central_n,left_n,right_n"
            )?;
            let pair_map = |x: &[[f64; 3]]| -> Result<
                std::collections::BTreeMap<(usize, [usize; 3], [usize; 3]), f64>,
                &'static str,
            > {
                Ok(assembly
                    .body
                    .active_surface_pairs_at(x)?
                    .into_iter()
                    .map(|(g, a, b, _, e)| ((g, a, b), e))
                    .collect())
            };
            let base_pairs = pair_map(&points)?;
            let mut strongest: Option<(usize, usize, [usize; 3], [usize; 3], f64)> = None;
            let mut pair_derivatives = std::io::BufWriter::new(std::fs::File::create(format!(
                "{output}/pair-derivatives.csv"
            ))?);
            writeln!(
                pair_derivatives,
                "node,axis,step_m,group,a0,a1,a2,b0,b1,b2,left_n,right_n,split_n"
            )?;
            let mut plain = assembly.body.clone();
            plain.set_surface_contacts(vec![])?;
            let (plain_energy, plain_gradient) = plain.evaluate(&points)?;
            let mut terms = std::io::BufWriter::new(std::fs::File::create(format!(
                "{output}/gradient-terms.csv"
            ))?);
            writeln!(
                terms,
                "node,axis,step_m,term,analytic_n,central_n,left_n,right_n"
            )?;
            for &(node, axis, analytic) in components.iter().take(6) {
                for step in [1e-8, 1e-9, 1e-10] {
                    let mut plus = points.clone();
                    let mut minus = points.clone();
                    plus[node][axis] += step;
                    minus[node][axis] -= step;
                    let ep = assembly.body.evaluate(&plus)?.0;
                    let em = assembly.body.evaluate(&minus)?.0;
                    let mesh_ep = assembly.body.surface_primitive_energy_at(&plus)?.0;
                    let mesh_em = assembly.body.surface_primitive_energy_at(&minus)?.0;
                    writeln!(
                        mesh_probe,
                        "{node},{axis},{step:.12e},{:.12e},{:.12e},{:.12e},{:.12e}",
                        mesh_primitive_gradient[node][axis],
                        (mesh_ep - mesh_em) / (2. * step),
                        (mesh_primitive_energy - mesh_em) / step,
                        (mesh_ep - mesh_primitive_energy) / step
                    )?;
                    let plus_pairs = pair_map(&plus)?;
                    let minus_pairs = pair_map(&minus)?;
                    let keys: std::collections::BTreeSet<_> = base_pairs
                        .keys()
                        .chain(plus_pairs.keys())
                        .chain(minus_pairs.keys())
                        .copied()
                        .collect();
                    for key in keys {
                        let (group, a, b) = key;
                        if !a.contains(&node) && !b.contains(&node) {
                            continue;
                        }
                        let e = *base_pairs.get(&key).unwrap_or(&0.);
                        let p = *plus_pairs.get(&key).unwrap_or(&0.);
                        let m = *minus_pairs.get(&key).unwrap_or(&0.);
                        let left = (e - m) / step;
                        let right = (p - e) / step;
                        if step == 1e-10
                            && strongest
                                .as_ref()
                                .is_none_or(|s| (right - left).abs() > s.4.abs())
                        {
                            strongest = Some((node, axis, a, b, right - left));
                        }
                        writeln!(
                            pair_derivatives,
                            "{node},{axis},{step:.12e},{group},{},{},{},{},{},{},{left:.12e},{right:.12e},{:.12e}",
                            a[0],
                            a[1],
                            a[2],
                            b[0],
                            b[1],
                            b[2],
                            right - left
                        )?;
                    }
                    let pp = plain.evaluate(&plus)?.0;
                    let pm = plain.evaluate(&minus)?.0;
                    for (term, e, p, m, g) in [
                        (
                            "without_surface_contact",
                            plain_energy,
                            pp,
                            pm,
                            plain_gradient[node][axis],
                        ),
                        (
                            "surface_contact",
                            energy - plain_energy,
                            ep - pp,
                            em - pm,
                            analytic - plain_gradient[node][axis],
                        ),
                    ] {
                        writeln!(
                            terms,
                            "{node},{axis},{step:.12e},{term},{g:.12e},{:.12e},{:.12e},{:.12e}",
                            (p - m) / (2. * step),
                            (e - m) / step,
                            (p - e) / step
                        )?;
                    }
                    writeln!(
                        derivatives,
                        "{node},{axis},{step:.12e},{analytic:.12e},{:.12e},{:.12e},{:.12e}",
                        (ep - em) / (2. * step),
                        (energy - em) / step,
                        (ep - energy) / step
                    )?;
                }
            }
            if let Some((node, axis, a, b, _)) = strongest {
                let mut primitive = std::io::BufWriter::new(std::fs::File::create(format!(
                    "{output}/primitive-probe.csv"
                ))?);
                writeln!(primitive, "step_m,analytic_n,central_n,left_n,right_n")?;
                let (energy, g) = assembly
                    .body
                    .surface_pair_primitive_barrier_at(&points, a, b, 1e-6, 5e-5, 1.)?;
                let local = a
                    .iter()
                    .chain(b.iter())
                    .position(|i| *i == node)
                    .ok_or("missing probe node")?;
                for h in [1e-8, 1e-9, 1e-10] {
                    let mut plus = points.clone();
                    let mut minus = points.clone();
                    plus[node][axis] += h;
                    minus[node][axis] -= h;
                    let ep = assembly
                        .body
                        .surface_pair_primitive_barrier_at(&plus, a, b, 1e-6, 5e-5, 1.)?
                        .0;
                    let em = assembly
                        .body
                        .surface_pair_primitive_barrier_at(&minus, a, b, 1e-6, 5e-5, 1.)?
                        .0;
                    writeln!(
                        primitive,
                        "{h:.12e},{:.12e},{:.12e},{:.12e},{:.12e}",
                        g[local][axis],
                        (ep - em) / (2. * h),
                        (energy - em) / h,
                        (ep - energy) / h
                    )?;
                }
                let mut file = std::io::BufWriter::new(std::fs::File::create(format!(
                    "{output}/closest-feature.csv"
                ))?);
                writeln!(
                    file,
                    "node,axis,perturbation_m,a0,a1,a2,b0,b1,b2,wa0,wa1,wa2,wb0,wb1,wb2,distance_m"
                )?;
                for shift in [-1e-10, 0., 1e-10] {
                    let mut trial = points.clone();
                    trial[node][axis] += shift;
                    let (wa, wb, d) = assembly.body.surface_pair_closest_at(&trial, a, b)?;
                    writeln!(
                        file,
                        "{node},{axis},{shift:.12e},{},{},{},{},{},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{d:.12e}",
                        a[0],
                        a[1],
                        a[2],
                        b[0],
                        b[1],
                        b[2],
                        wa[0],
                        wa[1],
                        wa[2],
                        wb[0],
                        wb[1],
                        wb[2]
                    )?;
                }
            }
            let rows = assembly.body.active_surface_pairs_at(&points)?;
            let mut file = std::io::BufWriter::new(std::fs::File::create(format!(
                "{output}/active-pairs.csv"
            ))?);
            writeln!(
                file,
                "group,a0,a1,a2,b0,b1,b2,distance_m,energy_j,cross_tissue"
            )?;
            for (group, a, b, d, e) in rows {
                writeln!(
                    file,
                    "{group},{},{},{},{},{},{},{d:.12e},{e:.12e},{}",
                    a[0],
                    a[1],
                    a[2],
                    b[0],
                    b[1],
                    b[2],
                    (a[0] >= offset) != (b[0] >= offset)
                )?;
            }
            return Ok(());
        }
        let surface = assembly.body.surface();
        let mut snapshot_error = None;
        let observe = |i, e, r, step, rejected, displacement, x: &[Vec3]| {
            if i % 100 == 0 {
                eprintln!(
                    "supported_wall_trace,{stage},{i},{e:.12e},{r:.12e},{step:.12e},{rejected},{displacement:.12e}"
                );
            }
            if snapshot_every > 0 && i > 0 && i % snapshot_every == 0 && snapshot_error.is_none() {
                let save = || -> std::io::Result<()> {
                    let stem = format!("{output}/{stage}-iterate-{i}");
                    export_coordinates(x, &surface, &format!("{stem}.obj"))?;
                    std::fs::write(
                        format!("{stem}.txt"),
                        format!(
                            "equilibrium_certified=false\niteration={i}\nenergy_j={e:.12e}\nresidual_n={r:.12e}\nload_factor={factor:.12e}\napplied_support_pressure_pa={:.12e}\ncontact_law={contact_law:?}\ncontact_curvature_preconditioner={contact_preconditioner}\nload_law={load_law}\nphysical_step_end_s={step_end_s}\nviscoelastic={viscoelastic}\n",
                            support_pressure * factor
                        ),
                    )
                };
                if let Err(error) = save() {
                    snapshot_error = Some(error);
                }
            }
        };
        let solve = if viscoelastic {
            assembly
                .body
                .relax_step_lbfgs_states(physical_dt, max_iterations, 1e-7, observe)
        } else {
            assembly.body.equilibrate_lbfgs_preconditioned_states(
                max_iterations,
                1e-7,
                contact_preconditioner,
                observe,
            )
        };
        let result = match solve {
            Ok(result) => result,
            Err(error) => {
                if let Some(previous) = previous_committed {
                    assembly.body = previous;
                }
                export(
                    &assembly.body,
                    &format!("{output}/{stage}-last-committed.obj"),
                )?;
                std::fs::write(
                    format!("{output}/{stage}-failed.txt"),
                    format!(
                        "physical_step_committed=false\nlast_committed_time_s={time_s}\nattempted_step_end_s={step_end_s}\nattempted_support_pressure_pa={}\nreason={error}\n",
                        support_pressure * factor
                    ),
                )?;
                return Err(error.into());
            }
        };
        time_s = step_end_s;
        if let Some(error) = snapshot_error {
            return Err(error.into());
        }
        if !result.converged {
            export(&assembly.body, &format!("{output}/{stage}-unconverged.obj"))?;
            std::fs::write(
                format!("{output}/{stage}-unconverged.txt"),
                format!(
                    "converged=false\niterations={}\nresidual_n={:.12e}\nmin_j={:.12e}\nminimum_surface_distance_m={:.12e}\n",
                    result.iterations,
                    result.residual_n,
                    result.min_j,
                    assembly
                        .body
                        .minimum_surface_contact_distance()?
                        .ok_or("missing contact group")?
                ),
            )?;
            return Err(format!(
                "{stage} did not converge: {} N; diagnostic geometry saved",
                result.residual_n
            )
            .into());
        }
        let displacement = |range: std::ops::Range<usize>| {
            range
                .map(|i| {
                    (0..3)
                        .map(|k| {
                            (assembly.body.positions()[i][k] - assembly.body.rest_positions()[i][k])
                                .powi(2)
                        })
                        .sum::<f64>()
                        .sqrt()
                })
                .fold(0., f64::max)
        };
        let energy = assembly.body.evaluate(assembly.body.positions())?.0;
        let mut without_contact = assembly.body.clone();
        without_contact.set_surface_contacts(vec![])?;
        let contact_energy = energy - without_contact.evaluate(assembly.body.positions())?.0;
        let mut self_contact = assembly.body.clone();
        let surface = self_contact.surface();
        let groups = [false, true].map(|support| TissueSurfaceContact {
            faces: surface
                .iter()
                .copied()
                .filter(|f| (f[0] >= offset) == support)
                .collect(),
            minimum_distance_m: 1e-6,
            activation_gap_m: 5e-5,
            pair_stiffness_n_m: 1.,
        });
        self_contact.set_surface_contacts(groups.to_vec())?;
        let cross_energy = energy - self_contact.evaluate(assembly.body.positions())?.0;
        println!(
            "{stage},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{contact_energy:.12e},{cross_energy:.12e},{support_pressure:.12e},{factor:.12e},{:.12e},{contact_law:?},{load_law},{time_s:.12e}",
            result.residual_n,
            result.min_j,
            displacement(assembly.node_ranges[0].clone()),
            displacement(assembly.node_ranges[1].clone()),
            assembly
                .body
                .minimum_surface_contact_distance()?
                .ok_or("missing contact group")?,
            support_pressure * factor
        );
        export(&assembly.body, &format!("{output}/{stage}.obj"))?;
        std::io::stdout().flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hull() -> (Body, Vec<usize>) {
        let mut b = elliptical_tube(
            &[0.008, 0.010, 0.012],
            0.03,
            8,
            3,
            &vec![Material::from_young_poisson(8000., 0.3).unwrap(); 2],
            true,
            [1., 0.3],
        )
        .unwrap();
        let side = b
            .surface()
            .into_iter()
            .filter(|f| f.iter().all(|i| i % 24 >= 16))
            .collect();
        let ends = [0, 3].map(|z| (0..8).map(|a| z * 24 + 16 + a).collect());
        let indices = install_pressure_hull(&mut b, side, ends).unwrap();
        (b, indices)
    }
    #[test]
    fn closed_reference_matches_follower_forces_at_reference_only() {
        let (mut b, indices) = hull();
        let x = b.positions().to_vec();
        let zero = b.evaluate(&x).unwrap();
        set_hull_pressure(&mut b, &indices, 100.).unwrap();
        let pressure = b.evaluate(&x).unwrap();
        let forces: Vec<Vec3> = zero
            .1
            .iter()
            .zip(&pressure.1)
            .map(|(a, b)| std::array::from_fn(|k| a[k] - b[k]))
            .collect();
        let mut dead = b.clone();
        set_hull_pressure(&mut dead, &indices, 0.).unwrap();
        for (i, f) in forces.into_iter().enumerate() {
            dead.set_force(i, f).unwrap();
        }
        let fixed_reference = dead.evaluate(&x).unwrap().1;
        for (a, b) in fixed_reference.iter().zip(&pressure.1) {
            for k in 0..3 {
                assert!((a[k] - b[k]).abs() < 1e-12);
            }
        }
        let mut changed = x.clone();
        for i in 24..changed.len() {
            changed[i][0] *= 1.03;
            changed[i][1] *= 0.95;
        }
        let follower = b.evaluate(&changed).unwrap().1;
        let fixed = dead.evaluate(&changed).unwrap().1;
        assert!(
            follower
                .iter()
                .zip(fixed)
                .any(|(a, b)| (0..3).any(|k| (a[k] - b[k]).abs() > 1e-6))
        );
    }
    #[test]
    fn averaged_closed_hull_pressure_has_consistent_gradient_on_nonplanar_end() {
        let (mut b, indices) = hull();
        set_hull_pressure(&mut b, &indices, 100.).unwrap();
        let mut x = b.positions().to_vec();
        for i in 72..96 {
            x[i][2] += 0.0002 * ((i % 8) as f64 * std::f64::consts::TAU / 8.).cos();
        }
        let (_, gradient) = b.evaluate(&x).unwrap();
        for node in [24, 40, 72, 80, 88, 95] {
            for axis in 0..3 {
                let mut p = x.clone();
                let mut m = x.clone();
                p[node][axis] += 1e-8;
                m[node][axis] -= 1e-8;
                let fd = (b.evaluate(&p).unwrap().0 - b.evaluate(&m).unwrap().0) / 2e-8;
                assert!(
                    (fd - gradient[node][axis]).abs() < 1e-6,
                    "node={node} axis={axis} fd={fd} g={}",
                    gradient[node][axis]
                );
            }
        }
        let reference = b.rest_positions().to_vec();
        set_hull_pressure(&mut b, &indices, 0.).unwrap();
        assert_eq!(b.rest_positions(), reference);
    }
    #[test]
    fn heterogeneous_layers_have_distinct_response_and_preserve_reference() {
        let (mut b, _) = hull();
        let reference = b.rest_positions().to_vec();
        let mut profile = Vec::new();
        let base = Material::from_young_poisson(8000., 0.3).unwrap();
        assign_viscous_layers(
            &mut b,
            3,
            8,
            &base,
            &[1., 2.],
            [0.2, 2.],
            "wall",
            &mut profile,
        )
        .unwrap();
        let f = [[1., 0.1, 0.], [0., 1., 0.], [0., 0., 1.]];
        let energies: Vec<_> = b
            .elements()
            .iter()
            .map(|e| e.response(f).unwrap().energy_density)
            .collect();
        let lo = energies.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = energies.iter().copied().fold(0., f64::max);
        assert!(lo > 0.);
        assert!((hi / lo - 2.).abs() < 1e-10);
        assert_eq!(b.rest_positions(), reference);
        assert_eq!(String::from_utf8(profile).unwrap().lines().count(), 2);
    }

    #[test]
    fn time_refinement_preserves_pressure_intervals_and_coarse_end_labels() {
        let original = vec![("loaded".into(), 1.), ("released".into(), 0.)];
        assert_eq!(refine_schedule(original.clone(), 1), original);
        let refined = refine_schedule(original, 2);
        assert_eq!(
            refined,
            vec![
                ("loaded-sub-1".into(), 1.),
                ("loaded".into(), 1.),
                ("released-sub-1".into(), 0.),
                ("released".into(), 0.)
            ]
        );
        let dt = 0.1;
        assert_eq!(2. * dt, refined.len() as f64 * (dt / 2.));
    }
}
