//! Neutral mechanical fold specimens with synthetic material data, SI units.
use physics::skin::*;
use std::io::Write;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/voxy-genital-folds".into());
    std::fs::create_dir_all(&output)?;
    let material = SkinMaterial {
        layers: vec![Layer {
            thickness: 0.0003,
            density: 1000.,
            shear_modulus: 1000.,
            collagen_modulus: 2000.,
            collagen_exponent: 4.,
            dispersion: 0.1,
            fiber_angle: 0.5,
            relaxation: vec![Relaxation {
                modulus: 500.,
                time: 0.02,
            }],
        }],
    };
    let prepuce = PrepuceGeometry {
        inner_radius_m: 0.009,
        outer_radius_m: 0.012,
        length_m: 0.02,
        sectors: 12,
        rows_per_section: 4,
    }
    .build(material.clone())?;
    let [left, right] = LabiaMinoraGeometry {
        length_m: 0.04,
        fold_width_m: 0.008,
        fold_height_m: 0.004,
        separation_m: 0.014,
        longitudinal_segments: 6,
        transverse_segments: 4,
    }
    .build(material)?;
    println!("specimen,phase,seconds,energy_j,min_area_ratio,crest_displacement_m");
    for (name, mut fold) in [
        ("prepuce", prepuce),
        ("labia-left", left),
        ("labia-right", right),
    ] {
        let initial = fold.skin.positions().to_vec();
        let mut forces = vec![[0.; 3]; initial.len()];
        for step in 0..8 {
            forces.fill([0.; 3]);
            if step < 4 {
                for &node in &fold.crest_nodes {
                    forces[node][2] = 0.0001 / fold.crest_nodes.len() as f64;
                }
            }
            let report = fold
                .skin
                .step(0.002, [0.; 3], &forces, &[], SolverConfig::default())?;
            let displacement = fold
                .crest_nodes
                .iter()
                .map(|&i| fold.skin.positions()[i][2] - initial[i][2])
                .sum::<f64>()
                / fold.crest_nodes.len() as f64;
            println!(
                "{name},{},{:.6},{:.12e},{:.12e},{displacement:.12e}",
                if step < 4 { "load" } else { "release" },
                (step + 1) as f64 * 0.002,
                report.energy,
                report.min_area_ratio
            );
            if step == 3 || step == 7 {
                let path = std::path::Path::new(&output).join(format!(
                    "{name}-{}.obj",
                    if step == 3 { "loaded" } else { "released" }
                ));
                let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
                for p in fold.skin.positions() {
                    writeln!(file, "v {} {} {}", p[0], p[1], p[2])?;
                }
                for t in fold.skin.triangles() {
                    writeln!(file, "f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1)?;
                }
            }
        }
    }
    use physics::biomechanics::{LabiaMajoraGeometry, Material};
    let pads = LabiaMajoraGeometry {
        length_m: 0.05,
        width_m: 0.018,
        height_m: 0.008,
        separation_m: 0.03,
        sectors: 12,
        latitude_rings: 3,
    }
    .build(Material::from_young_poisson(3000., 0.45)?)?;
    let mut ledger = std::io::BufWriter::new(std::fs::File::create(
        std::path::Path::new(&output).join("majora.csv"),
    )?);
    writeln!(ledger, "specimen,phase,apex_z_m,min_j,residual_n")?;
    for (name, mut pad) in [
        ("majora-left", pads[0].clone()),
        ("majora-right", pads[1].clone()),
    ] {
        for (phase, force) in [("loaded", -0.001), ("released", 0.)] {
            pad.body.set_force(pad.apex_node, [0., 0., force])?;
            let report = pad.body.equilibrate(20000, 1e-7)?;
            if !report.converged {
                return Err("majora equilibrium did not converge".into());
            }
            writeln!(
                ledger,
                "{name},{phase},{:.12e},{:.12e},{:.12e}",
                pad.body.positions()[pad.apex_node][2],
                report.min_j,
                report.residual_n
            )?;
            let mut file = std::io::BufWriter::new(std::fs::File::create(
                std::path::Path::new(&output).join(format!("{name}-{phase}.obj")),
            )?);
            for p in pad.body.positions() {
                writeln!(file, "v {} {} {}", p[0], p[1], p[2])?;
            }
            for t in pad.body.surface() {
                writeln!(file, "f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1)?;
            }
        }
    }
    Ok(())
}
