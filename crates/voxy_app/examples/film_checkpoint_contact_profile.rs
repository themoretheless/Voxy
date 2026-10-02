//! Paired full-capture geometry/transport/contact CPU profile.
#[allow(dead_code)]
#[path = "film_checkpoint_convergence.rs"]
mod checkpoint;
use physics::surface_film::{BridgeConfig, SurfaceFilm};
use serde_json::{Value, json};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: film_checkpoint_contact_profile CAPTURE.json REPORT.json".into());
    }
    let capture: Value = serde_json::from_slice(&std::fs::read(&args[0])?)?;
    let initial = checkpoint::load(&capture)?;
    let mut films = [
        SurfaceFilm::from_state(&initial)?,
        SurfaceFilm::from_state(&initial)?,
    ];
    let mass = films[0].total_mass();
    if mass <= 0. {
        return Err("nonempty film required".into());
    }
    let measured = capture["state"]["measurements"]["massKg"]
        .as_f64()
        .ok_or("missing captured mass")?;
    if !measured.is_finite() || (measured - mass).abs() > mass * 1e-12 {
        return Err("capture mass mismatch".into());
    }
    let mut samples = [Vec::new(), Vec::new()];
    let mut first = [0.; 2];
    let mut transferred = 0.;
    let mut max_mass_error = [0_f64; 2];
    let config = BridgeConfig::default();
    for frame in 0..22 {
        let phase = std::f64::consts::TAU * (frame + 1) as f64 / 22.;
        let points: Vec<_> = initial
            .points
            .iter()
            .map(|p| [p[0] * (1. + 0.03 * phase.sin()), p[1], p[2]])
            .collect();
        for mode in if frame % 2 == 0 { [0, 1] } else { [1, 0] } {
            let start = std::time::Instant::now();
            let (_, gross) = films[mode].advance_on_geometry_with_contact(
                &points,
                0.001,
                &[],
                [0., -9.81, 0.],
                (mode == 1).then_some(config),
            )?;
            let milliseconds = start.elapsed().as_secs_f64() * 1000.;
            if frame == 0 {
                first[mode] = milliseconds;
            }
            if frame >= 2 {
                samples[mode].push(milliseconds);
            }
            max_mass_error[mode] =
                max_mass_error[mode].max((films[mode].total_mass() - mass).abs() / mass);
            if mode == 1 {
                transferred += gross;
            }
        }
    }
    if max_mass_error.iter().any(|e| *e > 1e-12) {
        return Err("contact profile mass gate failed".into());
    }
    let mut runs = Vec::new();
    for mode in 0..2 {
        let state = films[mode].state();
        if state
            .cell_volumes_m3
            .iter()
            .any(|v| !v.is_finite() || *v < 0.)
        {
            return Err("invalid contact profile volume".into());
        }
        samples[mode].sort_by(f64::total_cmp);
        runs.push(json!({"selfContactEnabled":mode==1,"firstFrameMs":first[mode],"steadySamplesMs":samples[mode],
            "steadyUpperMedianMs":samples[mode][samples[mode].len()/2],"maxRelativeMassError":max_mass_error[mode]}));
    }
    let report = json!({"captureSource":args[0],"cells":initial.triangles.len(),"vertices":initial.points.len(),
        "frames":22,"dtSeconds":0.001,"grossContactTransferM3":transferred,"runs":runs,
        "scope":"Paired CPU geometry/transport/contact, alternating execution order, two warmup frames excluded from steady samples. Prescribed invertible stretch on captured surface. No rendering, tissue solve, source or CCD. Zero transfer cannot validate actual contact exchange."});
    std::fs::write(&args[1], serde_json::to_vec_pretty(&report)?)?;
    println!("{report}");
    Ok(())
}
