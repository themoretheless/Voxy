//! Load a genuine reference organ tetrahedralization into FEM and spatial Darcy.
//! Coefficients are explicit numerical specimens, not measured organ calibration.
use physics::biomechanics::*;
use std::path::PathBuf;
fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/anatomy/hra-female/tetrahedra");
    let path = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .map_or(root.join("left-ovary.vxtet"), PathBuf::from);
    let bytes = std::fs::read(&path).unwrap();
    let mesh = TetraMesh::from_bytes(&bytes).unwrap();
    let export_directory =
        std::env::args().find_map(|a| a.strip_prefix("--export-dir=").map(PathBuf::from));
    let boundary = mesh.boundary.clone();
    let points = mesh.points.len();
    let cells = mesh.cells.len();
    let faces = mesh.boundary.len();
    let mut body = mesh
        .into_body(
            vec![true; points],
            &Material {
                shear_pa: 5000.,
                bulk_pa: 50_000.,
                fibers: vec![],
            },
        )
        .unwrap();
    export_surface(&export_directory, "reference", &body, &boundary);
    if std::env::args().any(|a| a == "--myocardium") {
        let reference_positions = body.positions().to_vec();
        let low = std::array::from_fn::<_, 3, _>(|i| {
            body.positions()
                .iter()
                .map(|p| p[i])
                .fold(f64::INFINITY, f64::min)
        });
        let high = std::array::from_fn::<_, 3, _>(|i| {
            body.positions()
                .iter()
                .map(|p| p[i])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        let axis = (0..3)
            .max_by(|a, b| (high[*a] - low[*a]).total_cmp(&(high[*b] - low[*b])))
            .unwrap();
        let pins = body
            .positions()
            .iter()
            .map(|p| p[axis] <= low[axis] + 0.05 * (high[axis] - low[axis]))
            .collect();
        body = Body::new(
            body.positions().to_vec(),
            pins,
            body.elements()
                .iter()
                .map(|e| (e.nodes, e.material.clone()))
                .collect(),
        )
        .unwrap();
        let mut fiber = [0.; 3];
        fiber[axis] = 1.;
        let mut sheet = [0.; 3];
        sheet[(axis + 1) % 3] = 1.;
        let term = |scale_pa, exponent| ExponentialTerm { scale_pa, exponent };
        let law = Myocardium {
            matrix: term(2000., 7.),
            fiber: term(4000., 12.),
            sheet: term(1000., 5.),
            fiber_sheet: term(150., 10.),
            bulk_pa: 50_000.,
            fiber_direction: fiber,
            sheet_direction: sheet,
            active_tension_pa: 1000.,
        };
        body.set_myocardium_batch(&(0..cells).map(|i| (i, law)).collect::<Vec<_>>())
            .unwrap();
        for i in 0..cells {
            body.set_activation(i, 0.01).unwrap();
        }
        // Preserve the force tolerance on this refined anatomical mesh.
        let equilibrium = body.equilibrate(32_000, 1e-7).unwrap();
        let after_low = body
            .positions()
            .iter()
            .map(|p| p[axis])
            .fold(f64::INFINITY, f64::min);
        let after_high = body
            .positions()
            .iter()
            .map(|p| p[axis])
            .fold(f64::NEG_INFINITY, f64::max);
        println!(
            "cardiac_converged={} iterations={} residual_n={:.12e} fiber_axis={axis} initial_extent_m={:.12e} deformed_extent_m={:.12e} volume_m3={:.12e}",
            equilibrium.converged,
            equilibrium.iterations,
            equilibrium.residual_n,
            high[axis] - low[axis],
            after_high - after_low,
            body.volume_at(body.positions()).unwrap()
        );
        assert!(equilibrium.converged);
        assert!(after_high - after_low < high[axis] - low[axis]);
        export_surface(&export_directory, "contracted", &body, &boundary);
        if std::env::args().any(|a| a == "--cycle") {
            for i in 0..cells {
                body.set_activation(i, 0.).unwrap();
            }
            let relaxed = body.equilibrate(32_000, 1e-7).unwrap();
            let displacement = body
                .positions()
                .iter()
                .zip(&reference_positions)
                .map(|(x, r)| (0..3).map(|i| (x[i] - r[i]).powi(2)).sum::<f64>().sqrt())
                .fold(0., f64::max);
            println!(
                "cardiac_relaxed={} iterations={} residual_n={:.12e} max_return_displacement_m={displacement:.12e}",
                relaxed.converged, relaxed.iterations, relaxed.residual_n
            );
            assert!(relaxed.converged);
            export_surface(&export_directory, "relaxed", &body, &boundary);
        }
        return;
    }
    if std::env::args().any(|a| a == "--swell") {
        let low = body
            .positions()
            .iter()
            .map(|p| p[1])
            .fold(f64::INFINITY, f64::min);
        let high = body
            .positions()
            .iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max);
        // Explicit numerical clamp at the lower 5%; not anatomical ligament anchoring.
        let pins = body
            .positions()
            .iter()
            .map(|p| p[1] <= low + 0.05 * (high - low))
            .collect();
        body = Body::new(
            body.positions().to_vec(),
            pins,
            body.elements()
                .iter()
                .map(|e| (e.nodes, e.material.clone()))
                .collect(),
        )
        .unwrap();
        let reference = body.reference_volume();
        body.set_pore_fluid(PoreFluid {
            reference_fluid_volume_m3: 0.5 * reference,
            fluid_volume_m3: 0.501 * reference,
            biot_coefficient: 0.8,
            storage_m3_per_pa: reference / 100_000.,
        })
        .unwrap();
        let equilibrium = body.equilibrate(4000, 1e-7).unwrap();
        println!(
            "swelling_converged={} iterations={} residual_n={:.12e} initial_volume_m3={reference:.12e} deformed_volume_m3={:.12e} pore_pressure_pa={:.9}",
            equilibrium.converged,
            equilibrium.iterations,
            equilibrium.residual_n,
            body.volume_at(body.positions()).unwrap(),
            body.pore_response_at(body.positions()).unwrap().0
        );
        assert!(equilibrium.converged);
        export_surface(&export_directory, "swollen", &body, &boundary);
        return;
    }
    let report = body.equilibrate(4000, 1e-7).unwrap();
    assert!(report.converged);
    let volumes = body
        .stresses_at(body.positions())
        .unwrap()
        .into_iter()
        .map(|s| s.reference_volume_m3)
        .collect::<Vec<_>>();
    let stores = volumes
        .iter()
        .map(|v| PoreFluid {
            reference_fluid_volume_m3: 0.5 * v,
            fluid_volume_m3: 0.5 * v,
            biot_coefficient: 0.8,
            storage_m3_per_pa: v / 100_000.,
        })
        .collect();
    body.set_cell_pore_fluids(stores).unwrap();
    let k = [[1e-12, 0., 0.], [0., 1e-12, 0.], [0., 0., 1e-12]];
    let flow = body.deformed_darcy(&vec![k; cells], 0.001).unwrap();
    let pressure = body
        .elements()
        .iter()
        .map(|e| e.nodes.iter().map(|i| body.positions()[*i][1]).sum::<f64>() * 2500.)
        .collect::<Vec<_>>();
    let r = flow.response(&pressure).unwrap();
    let outflow: f64 = r.cell_outflows_m3_per_s.iter().sum();
    println!(
        "file={} vertices={points} tetrahedra={cells} boundary_faces={faces} volume_m3={:.12e} darcy_faces={} operator_bytes={} iterations={} residual_pa={:.12e} net_closed_outflow_m3_s={outflow:.12e} dissipation_w={:.12e}",
        path.display(),
        body.reference_volume(),
        flow.faces().len(),
        flow.operator_storage_bytes(),
        r.solver_iterations,
        r.residual_pa,
        r.dissipation_w
    );
    assert!(outflow.abs() < 1e-18);
    assert!((r.pressure_work_w - r.dissipation_w).abs() <= 1e-8 * r.dissipation_w.abs().max(1e-30));
    if let Some(directory) = &export_directory {
        use std::io::Write;
        let mut output = std::io::BufWriter::new(
            std::fs::File::create(directory.join("darcy-cells.csv")).unwrap(),
        );
        writeln!(output, "cell,pressure_pa,outflow_m3_s,vx_m_s,vy_m_s,vz_m_s").unwrap();
        for (i, ((p, q), v)) in pressure
            .iter()
            .zip(&r.cell_outflows_m3_per_s)
            .zip(&r.cell_centroid_velocities_m_per_s)
            .enumerate()
        {
            writeln!(
                output,
                "{i},{p:.17e},{q:.17e},{:.17e},{:.17e},{:.17e}",
                v[0], v[1], v[2]
            )
            .unwrap();
        }
        output.flush().unwrap();
        let mut output = std::io::BufWriter::new(
            std::fs::File::create(directory.join("darcy-faces.csv")).unwrap(),
        );
        writeln!(
            output,
            "face,node0,node1,node2,owner,neighbor,owner_to_neighbor_flow_m3_s"
        )
        .unwrap();
        for (i, (f, q)) in flow.faces().iter().zip(&r.face_flows_m3_per_s).enumerate() {
            let neighbor = f.neighbor.map_or(String::new(), |n| n.to_string());
            writeln!(
                output,
                "{i},{},{},{},{},{neighbor},{q:.17e}",
                f.nodes[0], f.nodes[1], f.nodes[2], f.owner
            )
            .unwrap();
        }
        output.flush().unwrap();
    }
}

// Preserve the audited outward boundary and atlas metre coordinates for inspection.
fn export_surface(directory: &Option<PathBuf>, name: &str, body: &Body, faces: &[[usize; 3]]) {
    use std::io::Write;
    let Some(directory) = directory else {
        return;
    };
    std::fs::create_dir_all(directory).unwrap();
    let points = body.positions();
    let path = directory.join(format!("{name}.obj"));
    let mut output = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
    writeln!(
        output,
        "# Voxy FEM boundary; atlas coordinates in metres; synthetic material inputs"
    )
    .unwrap();
    for p in points {
        writeln!(output, "v {:.17e} {:.17e} {:.17e}", p[0], p[1], p[2]).unwrap();
    }
    for f in faces {
        writeln!(output, "f {} {} {}", f[0] + 1, f[1] + 1, f[2] + 1).unwrap();
    }
    output.flush().unwrap();
    println!("surface_snapshot={}", path.display());
    let path = directory.join(format!("{name}.vtk"));
    let mut output = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
    writeln!(output, "# vtk DataFile Version 3.0\nVoxy FEM; metres, pascals; synthetic material\nASCII\nDATASET UNSTRUCTURED_GRID").unwrap();
    writeln!(output, "POINTS {} double", points.len()).unwrap();
    for p in points {
        writeln!(output, "{:.17e} {:.17e} {:.17e}", p[0], p[1], p[2]).unwrap();
    }
    let elements = body.elements();
    writeln!(output, "CELLS {} {}", elements.len(), elements.len() * 5).unwrap();
    for e in elements {
        writeln!(
            output,
            "4 {} {} {} {}",
            e.nodes[0], e.nodes[1], e.nodes[2], e.nodes[3]
        )
        .unwrap();
    }
    writeln!(output, "CELL_TYPES {}", elements.len()).unwrap();
    for _ in elements {
        writeln!(output, "10").unwrap();
    }
    let stresses = body.stresses_at(points).unwrap();
    writeln!(output, "CELL_DATA {}", stresses.len()).unwrap();
    for (label, values) in [
        (
            "volume_ratio",
            stresses.iter().map(|s| s.volume_ratio).collect::<Vec<_>>(),
        ),
        (
            "reference_volume_m3",
            stresses.iter().map(|s| s.reference_volume_m3).collect(),
        ),
        (
            "pressure_pa",
            stresses.iter().map(|s| s.stress.pressure_pa).collect(),
        ),
        (
            "von_mises_pa",
            stresses.iter().map(|s| s.stress.von_mises_pa).collect(),
        ),
    ] {
        writeln!(output, "SCALARS {label} double 1\nLOOKUP_TABLE default").unwrap();
        for v in values {
            writeln!(output, "{v:.17e}").unwrap();
        }
    }
    writeln!(output, "TENSORS cauchy_pa double").unwrap();
    for s in &stresses {
        for row in s.stress.cauchy_pa {
            writeln!(output, "{:.17e} {:.17e} {:.17e}", row[0], row[1], row[2]).unwrap();
        }
    }
    output.flush().unwrap();
    println!("volume_snapshot={}", path.display());
}
