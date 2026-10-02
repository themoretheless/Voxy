//! Synthetic oval four-layer walls, with pressure and distinct muscle actions.
use physics::biomechanics::*;
use std::io::Write;
fn cavity_volume(body: &Body) -> f64 {
    body.cavities()[0]
        .faces
        .iter()
        .map(|face| {
            let [a, b, c] = face.map(|i| body.positions()[i]);
            (a[0] * (b[1] * c[2] - b[2] * c[1])
                + a[1] * (b[2] * c[0] - b[0] * c[2])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.
        })
        .sum()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/voxy-urogenital-oval".into());
    std::fs::create_dir_all(&output)?;
    let passive = Material {
        shear_pa: 500.,
        bulk_pa: 5000.,
        fibers: vec![],
    };
    let muscle = |direction| Material {
        fibers: vec![Fiber {
            direction,
            stiffness_pa: 50.,
            exponent: 2.,
            active_pa: 1000.,
        }],
        ..passive.clone()
    };
    println!("specimen,stage,lumen_volume_m3,tip_z_m,min_j,residual_n");
    for (name, radii, length, scales) in [
        (
            "urethra",
            [0.0015, 0.0018, 0.0022, 0.0026, 0.003],
            0.02,
            [1., 0.6],
        ),
        (
            "vagina",
            [0.008, 0.009, 0.01, 0.011, 0.012],
            0.03,
            [1., 0.3],
        ),
    ] {
        let original = UrogenitalWallGeometry {
            radii_m: radii,
            length_m: length,
            sectors: 8,
            segments: 2,
        }
        .oval_wall(
            scales,
            [
                passive.clone(),
                muscle([0., 0., 1.]),
                muscle([1., 0., 0.]),
                passive.clone(),
            ],
        )?;
        for (stage, region, pressure) in [
            ("rest", None, 0.),
            ("pressure", None, 20.),
            ("longitudinal", Some(1), 0.),
            ("circular", Some(2), 0.),
        ] {
            let mut body = original.clone();
            body.set_pressure(0, pressure)?;
            if let Some(region) = region {
                for i in 0..body.elements().len() {
                    if body.elements()[i].region == region {
                        body.set_activation(i, 0.03)?;
                    }
                }
            }
            let report = body.equilibrate(100000, 1e-7)?;
            if !report.converged {
                return Err("oval specimen did not converge".into());
            }
            let tip = body
                .positions()
                .iter()
                .map(|p| p[2])
                .fold(f64::NEG_INFINITY, f64::max);
            println!(
                "{name},{stage},{:.12e},{tip:.12e},{:.12e},{:.12e}",
                cavity_volume(&body),
                report.min_j,
                report.residual_n
            );
            let mut file = std::io::BufWriter::new(std::fs::File::create(format!(
                "{output}/{name}-{stage}.obj"
            ))?);
            for p in body.positions() {
                writeln!(file, "v {} {} {}", p[0], p[1], p[2])?;
            }
            for face in body.surface() {
                writeln!(file, "f {} {} {}", face[0] + 1, face[1] + 1, face[2] + 1)?;
            }
        }
    }
    Ok(())
}
