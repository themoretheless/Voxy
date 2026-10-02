//! Anatomical cell-resolved pore transport, with optional fixed-skeleton isolation.
//! Synthetic coefficients and a numerical clamp; not calibrated lung physiology.
use physics::{biomechanics::*, lymph::*};
use std::path::PathBuf;
fn main() {
    let path = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../assets/anatomy/hra-female/tetrahedra/right-lung-middle-envelope.vxtet")
        });
    let numeric_option = |prefix: &str, default: f64| {
        std::env::args()
            .find_map(|a| {
                a.strip_prefix(prefix)
                    .map(|v| v.parse::<f64>().expect("numeric time option"))
            })
            .unwrap_or(default)
    };
    let seconds = numeric_option("--seconds=", 1e-4);
    let max_step = numeric_option("--max-step=", seconds);
    assert!(seconds.is_finite() && seconds > 0. && max_step.is_finite() && max_step > 0.);
    let output = std::env::args().find_map(|a| a.strip_prefix("--output=").map(PathBuf::from));
    let mesh = TetraMesh::from_bytes(&std::fs::read(path).unwrap()).unwrap();
    let fixed = std::env::args().any(|a| a == "--fixed");
    let low = mesh
        .points
        .iter()
        .map(|p| p[1])
        .fold(f64::INFINITY, f64::min);
    let high = mesh
        .points
        .iter()
        .map(|p| p[1])
        .fold(f64::NEG_INFINITY, f64::max);
    let pins = mesh
        .points
        .iter()
        .map(|p| fixed || p[1] <= low + 0.05 * (high - low))
        .collect();
    let mut body = mesh
        .into_body(
            pins,
            &Material {
                shear_pa: 5000.,
                bulk_pa: 50_000.,
                fibers: vec![],
            },
        )
        .unwrap();
    let stores = body
        .stresses_at(body.positions())
        .unwrap()
        .iter()
        .zip(body.elements())
        .map(|(s, e)| {
            let y = e.nodes.iter().map(|i| body.positions()[*i][1]).sum::<f64>() / 4.;
            let pressure = 100. * (y - low) / (high - low);
            let reference = 0.5 * s.reference_volume_m3;
            let storage = s.reference_volume_m3 / 100_000.;
            PoreFluid {
                reference_fluid_volume_m3: reference,
                fluid_volume_m3: reference + storage * pressure,
                storage_m3_per_pa: storage,
                biot_coefficient: 0.8,
            }
        })
        .collect::<Vec<_>>();
    body.set_cell_pore_fluids(stores).unwrap();
    let reference_positions = body.positions().to_vec();
    let initial_energy = body.evaluate(body.positions()).unwrap().0;
    let count = body.elements().len();
    let permeability = vec![[[1e-12, 0., 0.], [0., 1e-12, 0.], [0., 0., 1e-12]]; count];
    if std::env::args().any(|a| a == "--coupled-implicit") {
        let initial_volume: f64 = body
            .cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .sum();
        let mut protein: Vec<_> = body
            .cell_pore_fluids()
            .iter()
            .map(|f| 10. * f.fluid_volume_m3)
            .collect();
        let initial_protein: f64 = protein.iter().sum();
        let mut elapsed = 0.;
        while elapsed < seconds {
            let h = max_step.min(seconds - elapsed);
            let report = body
                .implicit_cell_pore_protein_step(
                    &mut protein,
                    &permeability,
                    0.001,
                    h,
                    ImplicitPoreConfig {
                        outer_iterations: 200,
                        pressure_tolerance_pa: 1e-6,
                        relaxation: 0.5,
                        solid_iterations: 32_000,
                        solid_tolerance_n: 1e-7,
                    },
                )
                .unwrap();
            elapsed += h;
            println!(
                "coupled_time_s={elapsed:.12e} outer_iterations={} pressure_residual_pa={:.12e} force_residual_n={:.12e}",
                report.iterations, report.pressure_residual_pa, report.solid.residual_n
            );
        }
        let final_volume: f64 = body
            .cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .sum();
        let protein_drift = protein.iter().sum::<f64>() - initial_protein;
        let final_energy = body.evaluate(body.positions()).unwrap().0;
        let displacement = body
            .positions()
            .iter()
            .zip(&reference_positions)
            .map(|(a, b)| (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt())
            .fold(0., f64::max);
        assert!((final_volume - initial_volume).abs() < 1e-10 * initial_volume);
        assert!(protein_drift.abs() < 1e-10 * initial_protein);
        assert!(final_energy <= initial_energy * (1. + 1e-10));
        println!(
            "coupled_fluid_drift_m3={:.12e} protein_drift_kg={protein_drift:.12e} final_energy_j={final_energy:.12e} max_displacement_m={displacement:.12e}",
            final_volume - initial_volume
        );
        return;
    }
    let flow = body.deformed_darcy(&permeability, 0.001).unwrap();
    if std::env::args().any(|a| a == "--implicit") {
        assert!(
            fixed,
            "implicit storage isolation currently requires --fixed"
        );
        let storage: Vec<_> = body
            .cell_pore_fluids()
            .iter()
            .map(|f| f.storage_m3_per_pa)
            .collect();
        let mut pressure = body.cell_pore_response_at(body.positions()).unwrap().0;
        let initial_content: f64 = pressure.iter().zip(&storage).map(|(p, s)| p * s).sum();
        let mut protein: Vec<_> = body
            .cell_pore_fluids()
            .iter()
            .map(|f| 10. * f.fluid_volume_m3)
            .collect();
        let initial_protein: f64 = protein.iter().sum();
        let mut elapsed = 0.;
        let mut iterations = 0;
        while elapsed < seconds {
            let h = max_step.min(seconds - elapsed);
            let (next, response) = flow.implicit_storage_step(&pressure, &storage, h).unwrap();
            let new_volumes: Vec<_> = next
                .iter()
                .zip(body.cell_pore_fluids())
                .map(|(p, f)| f.reference_fluid_volume_m3 + f.storage_m3_per_pa * p)
                .collect();
            protein = flow
                .implicit_protein_step(&protein, &new_volumes, &response.face_flows_m3_per_s, h)
                .unwrap();
            pressure = next;
            iterations += response.solver_iterations;
            elapsed += h;
        }
        let final_content: f64 = pressure.iter().zip(&storage).map(|(p, s)| p * s).sum();
        let final_energy: f64 = pressure
            .iter()
            .zip(&storage)
            .map(|(p, s)| 0.5 * s * p * p)
            .sum();
        assert!((final_content - initial_content).abs() < 1e-10 * initial_content.abs());
        assert!(final_energy <= initial_energy * (1. + 1e-10));
        println!(
            "implicit_cells={count} time_s={seconds:.12e} max_step_s={max_step:.12e} iterations={iterations} content_drift_m3={:.12e} final_energy_j={final_energy:.12e} pressure_min_pa={:.9} pressure_max_pa={:.9}",
            final_content - initial_content,
            pressure.iter().copied().fold(f64::INFINITY, f64::min),
            pressure.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        );
        let protein_drift = protein.iter().sum::<f64>() - initial_protein;
        let concentration_error = pressure
            .iter()
            .zip(body.cell_pore_fluids())
            .zip(&protein)
            .map(|((p, f), m)| {
                (m / (f.reference_fluid_volume_m3 + f.storage_m3_per_pa * p) - 10.).abs()
            })
            .fold(0., f64::max);
        assert!(protein_drift.abs() < 1e-10 * initial_protein);
        assert!(concentration_error < 1e-8);
        println!(
            "implicit_protein_drift_kg={protein_drift:.12e} max_uniform_concentration_error_kg_m3={concentration_error:.12e}"
        );
        if let Some(path) = output {
            use std::io::Write;
            let mut writer = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
            writeln!(
                writer,
                "cell,fluid_volume_m3,pore_pressure_pa,volume_ratio,protein_kg"
            )
            .unwrap();
            for (i, (p, f)) in pressure.iter().zip(body.cell_pore_fluids()).enumerate() {
                writeln!(
                    writer,
                    "{i},{:.17e},{p:.17e},1,{:.17e}",
                    f.reference_fluid_volume_m3 + f.storage_m3_per_pa * p,
                    protein[i]
                )
                .unwrap();
            }
            writer.flush().unwrap();
        }
        return;
    }
    let edges = flow
        .faces()
        .iter()
        .map(|f| Exchange {
            from: f.owner,
            to: f.neighbor.unwrap(),
            hydraulic_m3_per_pa_s: 0.,
            reflection: 0.,
            protein_permeability_m3_per_s: 0.,
            pump_head_pa: 0.,
            valve: false,
        })
        .collect();
    let spaces = body
        .cell_pore_fluids()
        .iter()
        .map(|p| FluidSpace {
            reference_volume_m3: p.reference_fluid_volume_m3,
            initial_volume_m3: p.fluid_volume_m3,
            initial_protein_kg: 10. * p.fluid_volume_m3,
            reference_pressure_pa: 0.,
            compliance_m3_per_pa: p.storage_m3_per_pa,
            oncotic_pa_per_kg_m3: 0.,
        })
        .collect();
    let mut network = LymphNetwork::new(spaces, edges).unwrap();
    let volume = network.total_volume();
    let protein = network.total_protein();
    let initial_volumes = network.volumes().to_vec();
    let mut tissue = CellPoreTissue::new(body, (0..count).collect(), 32_000, 1e-7).unwrap();
    tissue
        .step_mixed_darcy(&mut network, &permeability, 0.001, seconds, max_step)
        .unwrap();
    let moved = network
        .volumes()
        .iter()
        .zip(initial_volumes)
        .map(|(a, b)| (a - b).abs())
        .sum::<f64>()
        / 2.;
    let (_, energy) = tissue
        .body()
        .cell_pore_response_at(tissue.body().positions())
        .unwrap();
    println!(
        "cells={count} fixed_skeleton={fixed} time_s={seconds:.12e} max_step_s={max_step:.12e} internally_redistributed_m3={moved:.12e} fluid_drift_m3={:.12e} protein_drift_kg={:.12e} storage_energy_j={energy:.12e}",
        network.total_volume() - volume,
        network.total_protein() - protein
    );
    assert!((network.total_volume() - volume).abs() < 1e-12 * volume);
    assert!((network.total_protein() - protein).abs() < 1e-12 * protein);
    assert!(moved > 0.);
    let final_stresses = tissue
        .body()
        .stresses_at(tissue.body().positions())
        .unwrap();
    let deformed_volume: f64 = final_stresses
        .iter()
        .map(|s| s.reference_volume_m3 * s.volume_ratio)
        .sum();
    let max_displacement = tissue
        .body()
        .positions()
        .iter()
        .zip(reference_positions)
        .map(|(x, r)| (0..3).map(|k| (x[k] - r[k]).powi(2)).sum::<f64>().sqrt())
        .fold(0., f64::max);
    println!(
        "deformed_volume_m3={deformed_volume:.12e} max_displacement_m={max_displacement:.12e}"
    );
    if !fixed {
        assert!(max_displacement > 0.);
    }
    if let Some(path) = output {
        use std::io::Write;
        let pressures = tissue
            .body()
            .cell_pore_response_at(tissue.body().positions())
            .unwrap()
            .0;
        let mut writer = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
        writeln!(writer, "cell,fluid_volume_m3,pore_pressure_pa,volume_ratio").unwrap();
        for (i, ((v, p), s)) in network
            .volumes()
            .iter()
            .zip(pressures)
            .zip(&final_stresses)
            .enumerate()
        {
            writeln!(writer, "{i},{v:.17e},{p:.17e},{:.17e}", s.volume_ratio).unwrap();
        }
        writer.flush().unwrap();
    }
    let final_energy = tissue.body().evaluate(tissue.body().positions()).unwrap().0;
    assert!(final_energy <= initial_energy * (1. + 1e-10));
    println!(
        "initial_total_energy_j={initial_energy:.12e} final_total_energy_j={final_energy:.12e}"
    );
}
