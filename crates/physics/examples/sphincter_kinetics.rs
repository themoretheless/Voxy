//! Idealized IAS/EAS specimen with explicit synthetic activation times.
//! Quasistatic mechanics; no pelvic anatomy, reflexes or calibrated muscle data.
use physics::biomechanics::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut body = sphincter_layers()?;
    if std::env::args().any(|a| a == "--length-dependent") {
        for i in 0..body.elements().len() {
            body.set_active_fiber_length_law(
                i,
                ActiveFiberLengthLaw {
                    optimal_stretch: 1.,
                    half_width: 0.5,
                },
            )?;
        }
    }
    let mut drives = [0, 1].map(|region| MuscleRegionDrive {
        region,
        activation: 0.,
        excitation: 0.,
        kinetics: ActivationKinetics {
            rise_seconds: 0.2,
            fall_seconds: 0.5,
            tonic_activation: if region == 0 { 0.02 } else { 0. },
        },
    });
    println!("seconds,ias_activation,eas_activation,lumen_volume_m3,min_j,residual_force_n");
    let mut time = 0.;
    for phase in 0..8 {
        let active = phase < 4;
        drives[0].excitation = if active { 0.2 } else { 0. };
        drives[1].excitation = if active { 0.1 } else { 0. };
        let seconds = if active { 0.1 } else { 0.5 };
        let report = body.step_muscle_regions(&mut drives, seconds, 50000, 1e-5)?;
        time += seconds;
        let volume: f64 = body.cavities()[0]
            .faces
            .iter()
            .map(|f| {
                let [a, b, c] = f.map(|i| body.positions()[i]);
                (a[0] * (b[1] * c[2] - b[2] * c[1])
                    + a[1] * (b[2] * c[0] - b[0] * c[2])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.
            })
            .sum();
        println!(
            "{time:.6},{:.12e},{:.12e},{volume:.12e},{:.12e},{:.12e}",
            drives[0].activation, drives[1].activation, report.min_j, report.residual_n
        );
    }
    Ok(())
}
