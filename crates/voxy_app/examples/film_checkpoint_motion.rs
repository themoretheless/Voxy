//! Moving-substrate replay with a prescribed invertible motion and metered source.
#[allow(dead_code)]
#[path = "film_checkpoint_convergence.rs"]
mod checkpoint;
use physics::surface_film::SurfaceFilm;
use serde_json::{Value, json};
fn posed(points: &[[f64; 3]], time: f64, duration: f64) -> Vec<[f64; 3]> {
    let phase = (std::f64::consts::TAU * time / duration).sin();
    let (s, c) = (0.08 * phase).sin_cos();
    let stretch = 1. + 0.03 * phase;
    points
        .iter()
        .map(|p| [stretch * p[0], c * p[1] - s * p[2], s * p[1] + c * p[2]])
        .collect()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: film_checkpoint_motion CAPTURE.json REPORT.json".into());
    }
    let capture: Value = serde_json::from_slice(&std::fs::read(&args[0])?)?;
    let initial = checkpoint::load(&capture)?;
    let reference = SurfaceFilm::from_state(&initial)?;
    let mass = reference.total_mass();
    let volume = reference.total_volume();
    if mass <= 0. {
        return Err("nonempty film required".into());
    }
    let measured = capture["state"]["measurements"]["massKg"]
        .as_f64()
        .ok_or("missing mass")?;
    if !measured.is_finite() || (measured - mass).abs() > mass * 1e-12 {
        return Err("captured mass mismatch".into());
    }
    let source = initial
        .cell_volumes_m3
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .unwrap()
        .0;
    let duration = 0.02;
    let rate = 1e-9;
    let expected_mass = mass + rate * duration * initial.material.density;
    let mut states = Vec::new();
    let mut runs = Vec::new();
    for steps in [20, 40, 80, 160] {
        let dt = duration / steps as f64;
        let mut film = SurfaceFilm::from_state(&initial)?;
        let started = std::time::Instant::now();
        let mut supplied = 0.;
        let mut max_mass_error = 0_f64;
        for step in 1..=steps {
            let time = step as f64 * dt;
            let points = posed(&initial.points, time, duration);
            supplied +=
                film.advance_on_geometry(&points, dt, &[(source, rate)], [0., -9.81, 0.])?;
            let expected = mass + rate * time * initial.material.density;
            max_mass_error = max_mass_error.max((film.total_mass() - expected).abs() / expected);
        }
        let state = film.state();
        if max_mass_error > 1e-12
            || state
                .cell_volumes_m3
                .iter()
                .any(|v| !v.is_finite() || *v < 0.)
        {
            return Err("moving replay mass/positivity gate failed".into());
        }
        let returned_geometry_error = state
            .points
            .iter()
            .zip(&initial.points)
            .flat_map(|(a, b)| a.iter().zip(b).map(|(x, y)| (x - y).abs()))
            .fold(0_f64, f64::max);
        if returned_geometry_error > 1e-12 {
            return Err("motion failed to return to initial surface".into());
        }
        runs.push(json!({"steps":steps,"dtSeconds":dt,"wallSeconds":started.elapsed().as_secs_f64(),
            "suppliedVolumeM3":supplied,"finalMassKg":film.total_mass(),"maxRelativeMassError":max_mass_error,
            "returnedGeometryMaxErrorM":returned_geometry_error}));
        states.push(state.cell_volumes_m3);
    }
    let errors: Vec<_> = states[..3]
        .iter()
        .map(|v| {
            v.iter()
                .zip(&states[3])
                .map(|(a, b)| (a - b).abs())
                .sum::<f64>()
                / (volume + rate * duration)
        })
        .collect();
    let trend = errors[0] > 1e-14 && errors.windows(2).all(|p| p[1] < p[0]);
    let report = json!({"captureSource":args[0],"cells":initial.triangles.len(),"simulationSeconds":duration,
        "sourceCell":source,"sourceRateM3S":rate,"expectedFinalMassKg":expected_mass,"runs":runs,
        "normalizedVolumeL1VsFinest":errors,"timeRefinementTrend":trend,
        "scope":"Captured full-body substrate, prescribed smooth invertible rotation/stretch cycle, original material/wetting, gravity and metered source. No skeletal animation, self-contact exchange, inertial coupling or physiological calibration."});
    std::fs::write(&args[1], serde_json::to_vec_pretty(&report)?)?;
    println!("{report}");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prescribed_motion_returns_and_preserves_orientation() {
        let points = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        assert_eq!(posed(&points, 0., 1.), points);
        for time in [0.25, 0.5, 0.75, 1.] {
            let p = posed(&points, time, 1.);
            let determinant = p[0][0] * (p[1][1] * p[2][2] - p[1][2] * p[2][1]);
            assert!(determinant > 0.96 && determinant < 1.04);
        }
        for (a, b) in posed(&points, 1., 1.).iter().zip(points) {
            for (x, y) in a.iter().zip(b) {
                assert!((x - y).abs() < 1e-14);
            }
        }
    }
}
