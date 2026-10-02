//! Prescribed cell fluid inventory in a coupled clitoral Biot-FEM specimen.
//! No vascular flow or measured physiological constants are assumed.
use physics::biomechanics::*;
use std::io::Write;
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
    let mut assembly = complex.coupled(1., 6)?;
    let baseline = assembly.body.cell_pore_fluids().to_vec();
    let volumes: Vec<_> = assembly
        .body
        .stresses_at(assembly.body.positions())?
        .iter()
        .map(|e| e.reference_volume_m3)
        .collect();
    let end = assembly.cell_ranges[1].end;
    let reference_volume: f64 = volumes[..end].iter().sum();
    println!(
        "stage,inventory_fraction,fluid_inventory_m3,corpora_volume_ratio,min_j,mean_corpus_pressure_pa,residual_n"
    );
    for (stage, fraction) in [0., 0.005, 0.01, 0.005, 0.].into_iter().enumerate() {
        let mut fluids = baseline.clone();
        for i in 0..end {
            fluids[i].fluid_volume_m3 += fraction * volumes[i];
        }
        assembly.body.set_cell_pore_fluids(fluids)?;
        let report = assembly.body.equilibrate(100000, 1e-7)?;
        if !report.converged {
            return Err("pore-filled specimen equilibrium did not converge".into());
        }
        let stresses = assembly.body.stresses_at(assembly.body.positions())?;
        let volume: f64 = stresses[..end]
            .iter()
            .map(|e| e.reference_volume_m3 * e.volume_ratio)
            .sum();
        let pressure = assembly
            .body
            .cell_pore_response_at(assembly.body.positions())?
            .0;
        let mean_pressure: f64 = pressure[..end]
            .iter()
            .zip(&volumes[..end])
            .map(|(p, v)| p * v)
            .sum::<f64>()
            / reference_volume;
        let inventory: f64 = assembly
            .body
            .cell_pore_fluids()
            .iter()
            .map(|f| f.fluid_volume_m3)
            .sum();
        println!(
            "{stage},{fraction:.6},{inventory:.12e},{:.12e},{:.12e},{mean_pressure:.12e},{:.12e}",
            volume / reference_volume,
            report.min_j,
            report.residual_n
        );
    }
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/voxy-urogenital-pore.obj".into());
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    for p in assembly.body.positions() {
        writeln!(file, "v {} {} {}", p[0], p[1], p[2])?;
    }
    for f in assembly.body.surface() {
        writeln!(file, "f {} {} {}", f[0] + 1, f[1] + 1, f[2] + 1)?;
    }
    Ok(())
}
