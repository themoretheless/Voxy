//! Neutral mechanical scenarios with synthetic parameters; no clinical fitting.
use physics::biomechanics::*;
use std::io::Write;
fn material(direction: Option<Vec3>) -> Material {
    Material {
        shear_pa: 500.,
        bulk_pa: 5000.,
        fibers: direction
            .map(|direction| {
                vec![Fiber {
                    direction,
                    stiffness_pa: 50.,
                    exponent: 2.,
                    active_pa: 1000.,
                }]
            })
            .unwrap_or_default(),
    }
}
fn layers() -> [Material; 4] {
    [
        material(None),
        material(Some([0., 0., 1.])),
        material(Some([1., 0., 0.])),
        material(None),
    ]
}
fn export(body: &Body, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    for p in body.positions() {
        writeln!(file, "v {} {} {}", p[0], p[1], p[2])?;
    }
    for f in body.surface() {
        writeln!(file, "f {} {} {}", f[0] + 1, f[1] + 1, f[2] + 1)?;
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/voxy-urogenital".into());
    let output = std::path::Path::new(&output);
    std::fs::create_dir_all(output)?;
    let urethra = UrogenitalWallGeometry {
        radii_m: [0.0015, 0.0018, 0.0022, 0.0026, 0.003],
        length_m: 0.02,
        sectors: 8,
        segments: 2,
    }
    .urethra(layers())?;
    let vagina = UrogenitalWallGeometry {
        radii_m: [0.008, 0.009, 0.01, 0.011, 0.012],
        length_m: 0.03,
        sectors: 8,
        segments: 2,
    }
    .vagina(layers())?;
    println!("specimen,phase,min_j,residual_n");
    for (name, mut body) in [("urethra", urethra), ("vagina", vagina)] {
        export(&body, &output.join(format!("{name}-rest.obj")))?;
        let mut drives = [1, 2].map(|region| MuscleRegionDrive {
            region,
            activation: 0.,
            excitation: 0.03,
            kinetics: ActivationKinetics {
                rise_seconds: 0.2,
                fall_seconds: 0.5,
                tonic_activation: 0.,
            },
        });
        for phase in ["active", "released"] {
            if phase == "released" {
                for drive in &mut drives {
                    drive.excitation = 0.;
                }
            }
            let report = body.step_muscle_regions(&mut drives, 0.2, 50000, 1e-7)?;
            println!(
                "{name},{phase},{:.12e},{:.12e}",
                report.min_j, report.residual_n
            );
            export(&body, &output.join(format!("{name}-{phase}.obj")))?;
        }
    }
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
    .build(material(Some([0., 0., 1.])), material(None), material(None))?;
    for (name, body) in ["corpus-crus-left", "corpus-crus-right"]
        .into_iter()
        .zip(&mut complex.corpora_crura)
        .chain(std::iter::once(("glans", &mut complex.glans)))
        .chain(
            ["vestibular-bulb-left", "vestibular-bulb-right"]
                .into_iter()
                .zip(&mut complex.vestibular_bulbs),
        )
    {
        export(body, &output.join(format!("clitoris-{name}-rest.obj")))?;
        let tip = (0..body.positions().len())
            .max_by(|&a, &b| body.positions()[a][2].total_cmp(&body.positions()[b][2]))
            .unwrap();
        body.set_force(tip, [0., 0., -0.0001])?;
        let report = body.equilibrate(50000, 1e-7)?;
        if !report.converged {
            return Err("clitoral specimen nonconvergence".into());
        }
        println!(
            "clitoris-{name},loaded,{:.12e},{:.12e}",
            report.min_j, report.residual_n
        );
        export(body, &output.join(format!("clitoris-{name}-loaded.obj")))?;
    }
    let mut assembly = complex.coupled(1., 6)?;
    export(&assembly.body, &output.join("clitoris-coupled-before.obj"))?;
    let range = assembly.node_ranges[2].clone();
    let tip = range
        .max_by(|&a, &b| {
            assembly.body.positions()[a][2].total_cmp(&assembly.body.positions()[b][2])
        })
        .unwrap();
    // Independent parts already carry the demonstration loads. Clear all of them
    // before applying one load to the coupled glans.
    for i in 0..assembly.body.positions().len() {
        assembly.body.set_force(i, [0.; 3])?;
    }
    assembly.body.set_force(tip, [0., 0., -0.0001])?;
    let report = assembly.body.equilibrate(100000, 1e-7)?;
    if !report.converged {
        return Err("coupled clitoral specimen nonconvergence".into());
    }
    println!(
        "clitoris-coupled,loaded,{:.12e},{:.12e}",
        report.min_j, report.residual_n
    );
    export(&assembly.body, &output.join("clitoris-coupled-loaded.obj"))?;
    Ok(())
}
