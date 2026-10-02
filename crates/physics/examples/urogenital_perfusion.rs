//! Finite-reservoir transport in a coupled clitoral Biot-FEM specimen.
//! No vascular flow or measured physiological constants are assumed.
use physics::biomechanics::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let material = Material::from_young_poisson(3000., 0.4)?;
    let mut complex = ClitoralGeometry {
        corpus_radius_m: 0.002,
        crus_length_m: 0.018,
        body_length_m: 0.01,
        root_half_separation_m: 0.012,
        body_half_separation_m: 0.003,
        glans_radii_m: [0.004, 0.003, 0.004],
        bulb_radii_m: [0.005, 0.007, 0.01],
        bulb_half_separation_m: 0.015,
        sectors: 8,
        segments: 3,
    }
    .build(material.clone(), material.clone(), material)?;
    for part in complex
        .corpora_crura
        .iter_mut()
        .chain(std::iter::once(&mut complex.glans))
        .chain(complex.vestibular_bulbs.iter_mut())
    {
        let fluids = part
            .stresses_at(part.positions())?
            .iter()
            .map(|e| PoreFluid {
                reference_fluid_volume_m3: 0.3 * e.reference_volume_m3,
                fluid_volume_m3: 0.3 * e.reference_volume_m3,
                biot_coefficient: 0.8,
                storage_m3_per_pa: 1e-4 * e.reference_volume_m3,
            })
            .collect();
        part.set_cell_pore_fluids(fluids)?;
    }
    let assembly = complex.coupled(1., 6)?;
    let n = assembly.body.elements().len();
    let reservoir = |pressure| physics::lymph::FluidSpace {
        reference_volume_m3: 1e-6,
        initial_volume_m3: 1e-6,
        reference_pressure_pa: pressure,
        compliance_m3_per_pa: 1e-8,
        initial_protein_kg: 1e-5,
        oncotic_pa_per_kg_m3: 0.,
    };
    let solute = assembly
        .body
        .cell_pore_fluids()
        .iter()
        .map(|f| PerfusionCellSolute {
            protein_kg: 10. * f.fluid_volume_m3,
            oncotic_pa_per_kg_m3: 0.,
        })
        .collect();
    let mut ports = Vec::new();
    for range in &assembly.cell_ranges[..2] {
        for (cell, reservoir, incoming) in [(range.start, 0, true), (range.end - 1, 1, false)] {
            ports.push(PerfusionPort {
                tissue_cell: cell,
                reservoir,
                reservoir_to_tissue: incoming,
                hydraulic_m3_per_pa_s: 1e-11,
                reflection: 0.,
                protein_permeability_m3_per_s: 0.,
            });
        }
    }
    let permeability = vec![[[1e-12, 0., 0.], [0., 1e-12, 0.], [0., 0., 1e-12]]; n];
    let mut perfusion = PorePerfusion::new(
        assembly.body,
        permeability,
        0.001,
        solute,
        vec![reservoir(100.), reservoir(0.)],
        ports,
        100000,
        1e-7,
    )?;
    let water = perfusion.network().total_volume();
    let protein = perfusion.network().total_protein();
    println!(
        "time_s,inlet_volume_m3,outlet_volume_m3,tissue_volume_m3,min_j,water_drift_m3,protein_drift_kg"
    );
    for stage in 1..=5 {
        if std::env::args().any(|arg| arg == "--adaptive") {
            let report = perfusion.step_adaptive(
                0.001,
                AdaptiveTissueExchangeConfig {
                    exchange: physics::lymph::AdaptiveExchangeConfig {
                        relative_tolerance: 0.,
                        absolute_volume_tolerance_m3: 1e-15,
                        absolute_protein_tolerance_kg: 1e-14,
                        min_step_seconds: 1e-8,
                        max_step_seconds: 0.001,
                        max_trials: 10000,
                    },
                    absolute_position_tolerance_m: 1e-7,
                },
            )?;
            eprintln!(
                "stage={stage} accepted={} rejected={} max_error_ratio={:.6e}",
                report.accepted_steps, report.rejected_steps, report.max_accepted_error_ratio
            );
        } else {
            perfusion.step_second_order(0.001, 0.0001)?;
        }
        let nodes = perfusion.reservoir_nodes();
        let volumes = perfusion.network().volumes();
        let tissue: f64 = volumes[..n].iter().sum();
        let min_j = perfusion
            .body()
            .stresses_at(perfusion.body().positions())?
            .iter()
            .map(|s| s.volume_ratio)
            .fold(f64::INFINITY, f64::min);
        println!(
            "{:.6},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}",
            stage as f64 * 0.001,
            volumes[nodes[0]],
            volumes[nodes[1]],
            tissue,
            min_j,
            perfusion.network().total_volume() - water,
            perfusion.network().total_protein() - protein
        );
        assert!(min_j > 0.);
        assert!((perfusion.network().total_volume() - water).abs() < water * 1e-10);
        assert!((perfusion.network().total_protein() - protein).abs() < protein * 1e-10);
    }
    Ok(())
}
