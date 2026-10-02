//! Bonded idealized IAS/EAS ring with dynamic activation and fixed supports.
//! Synthetic fast excitation scenario, not calibrated physiological parameters.
use physics::biomechanics::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut release_phases = 4_usize;
    let mut substeps = 200_usize;
    for arg in std::env::args().skip(1) {
        if let Some(value) = arg.strip_prefix("--release-phases=") {
            release_phases = value.parse()?;
        } else if let Some(value) = arg.strip_prefix("--substeps=") {
            substeps = value.parse()?;
        } else {
            return Err(format!("unknown argument: {arg}").into());
        }
    }
    if !(1..=1000).contains(&release_phases) || !(1..=100000).contains(&substeps) {
        return Err("invalid phase/substep count".into());
    }
    let mut tissue = sphincter_layers()?;
    for i in 0..tissue.elements().len() {
        tissue.set_active_fiber_length_law(
            i,
            ActiveFiberLengthLaw {
                optimal_stretch: 1.,
                half_width: 0.5,
            },
        )?;
    }
    let velocities = vec![[0.; 3]; tissue.positions().len()];
    let density = vec![1000.; tissue.elements().len()];
    let mut body = InertialBody::new_with_fixed_supports(tissue, &density, velocities)?;
    let law = ActiveFiberVelocityLaw {
        max_shortening_per_s: 2.,
        shortening_curvature: 0.25,
        eccentric_limit: 1.5,
        eccentric_rate_per_s: 1.,
    };
    let mut drives = [0, 1].map(|region| MuscleRegionDrive {
        region,
        activation: 0.,
        excitation: 0.,
        kinetics: ActivationKinetics {
            rise_seconds: 0.002,
            fall_seconds: 0.005,
            tonic_activation: 0.,
        },
    });
    let mut activation_work = 0.;
    let mut correction_work = 0.;
    let mut total_defect = 0.;
    println!(
        "seconds,ias_activation,eas_activation,lumen_m3,kinetic_j,activation_work_j,correction_work_j,total_work_defect_j,min_j,max_support_reaction_n,max_total_von_mises_pa"
    );
    for phase in 0..4 + release_phases {
        drives[0].excitation = if phase < 4 { 0.2 } else { 0. };
        drives[1].excitation = if phase < 4 { 0.1 } else { 0. };
        for _ in 0..substeps {
            let r = body.step_driven_muscle(&mut drives, 0.0004 / substeps as f64, 1e-7, law)?;
            activation_work += r.activation_work_j;
            correction_work += r.correction_work_j;
            total_defect += r.energy_defect_j;
        }
        let volume: f64 = body.body().cavities()[0]
            .faces
            .iter()
            .map(|face| {
                let [a, b, c] = face.map(|i| body.body().positions()[i]);
                (a[0] * (b[1] * c[2] - b[2] * c[1])
                    + a[1] * (b[2] * c[0] - b[0] * c[2])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.
            })
            .sum();
        let stresses = body.body().muscle_stresses(body.velocities(), law)?;
        let min_j = stresses
            .iter()
            .map(|e| e.volume_ratio)
            .fold(f64::INFINITY, f64::min);
        let max_vm = stresses
            .iter()
            .map(|e| e.stress.von_mises_pa)
            .fold(0_f64, f64::max);
        let reaction = body
            .muscle_support_reactions(law)?
            .iter()
            .map(|f| f.iter().map(|v| v * v).sum::<f64>().sqrt())
            .fold(0_f64, f64::max);
        println!(
            "{:.6},{:.12e},{:.12e},{volume:.12e},{:.12e},{activation_work:.12e},{correction_work:.12e},{total_defect:.12e},{min_j:.12e},{reaction:.12e},{max_vm:.12e}",
            (phase + 1) as f64 * 0.0004,
            drives[0].activation,
            drives[1].activation,
            body.diagnostics()?.kinetic_j
        );
    }
    Ok(())
}
