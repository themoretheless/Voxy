//! Coupled passive clitoral tissues with synthetic Ogden-Maxwell relaxation.
//! Quasistatic timed mechanics, not calibrated vascular or neural response.
use physics::biomechanics::*;
use std::io::Write;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = Material::from_young_poisson(3000., 0.4)?;
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
    .build(base.clone(), base.clone(), base)?;
    let law = ViscoelasticOgden::new(
        vec![OgdenTerm {
            shear_pa: 500.,
            exponent: 2.,
        }],
        5000.,
        vec![MaxwellBranch {
            shear_pa: 1000.,
            relaxation_seconds: 0.1,
        }],
    )?;
    for tissue in std::iter::once(&mut complex.glans).chain(complex.vestibular_bulbs.iter_mut()) {
        for i in 0..tissue.elements().len() {
            tissue.set_viscoelastic_ogden(i, law.clone())?;
        }
    }
    let mut assembly = complex.coupled(1., 6)?;
    let range = assembly.node_ranges[2].clone();
    let tip = range
        .max_by(|&a, &b| {
            assembly.body.positions()[a][2].total_cmp(&assembly.body.positions()[b][2])
        })
        .unwrap();
    let rest_z = assembly.body.positions()[tip][2];
    println!("seconds,phase,glans_displacement_m,min_j,residual_n");
    for step in 0..12 {
        let loaded = step < 8;
        assembly
            .body
            .set_force(tip, [0., 0., if loaded { -0.0001 } else { 0. }])?;
        let report = assembly.body.relax_step(0.02, 100000, 1e-7)?;
        println!(
            "{:.4},{},{:.12e},{:.12e},{:.12e}",
            (step + 1) as f64 * 0.02,
            if loaded { "load" } else { "release" },
            assembly.body.positions()[tip][2] - rest_z,
            report.min_j,
            report.residual_n
        );
    }
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/voxy-urogenital-relaxation.obj".into());
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    for p in assembly.body.positions() {
        writeln!(file, "v {} {} {}", p[0], p[1], p[2])?;
    }
    for f in assembly.body.surface() {
        writeln!(file, "f {} {} {}", f[0] + 1, f[1] + 1, f[2] + 1)?;
    }
    Ok(())
}
