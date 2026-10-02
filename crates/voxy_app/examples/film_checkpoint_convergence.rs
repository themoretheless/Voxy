//! Replay a captured numerical film on its fixed substrate to measure time refinement.
use physics::surface_film::{Material, SurfaceFilm, SurfaceFilmState, Wetting};
use serde_json::{Value, json};

fn number(value: &Value, key: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let number = value[key]
        .as_f64()
        .ok_or_else(|| format!("missing numeric {key}"))?;
    if !number.is_finite() {
        return Err(format!("nonfinite {key}").into());
    }
    Ok(number)
}
pub(crate) fn load(value: &Value) -> Result<SurfaceFilmState, Box<dyn std::error::Error>> {
    if value["capture"] != "numericalFilmState"
        || value["filmEnabled"] != true
        || value["state"]["format"] != "voxy.surface-film-state.v1"
        || value["state"]["units"] != "SI"
    {
        return Err("enabled SI film-state capture required".into());
    }
    let state = &value["state"]["physics"];
    let material = &state["material"];
    let wetting = if state["precursorWetting"].is_null() {
        None
    } else {
        Some(Wetting {
            contact_angle: number(&state["precursorWetting"], "contactAngleRad")?,
            precursor_thickness: number(&state["precursorWetting"], "precursorThicknessM")?,
        })
    };
    Ok(SurfaceFilmState {
        points: serde_json::from_value(state["pointsM"].clone())?,
        triangles: serde_json::from_value(state["triangles"].clone())?,
        cell_volumes_m3: serde_json::from_value(state["cellVolumesM3"].clone())?,
        material: Material {
            density: number(material, "density")?,
            viscosity: number(material, "viscosity")?,
            surface_tension: number(material, "surfaceTension")?,
            wetting: number(material, "wetting")?,
        },
        wetting,
    })
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if !(2..=3).contains(&arguments.len()) {
        return Err(
            "usage: film_checkpoint_convergence CAPTURE.json REPORT.json [DURATION_SECONDS]".into(),
        );
    }
    let duration: f64 = arguments.get(2).map_or(Ok(0.1), |v| v.parse())?;
    if !duration.is_finite() || duration <= 0. || duration > 1. {
        return Err("duration must be finite, positive and <=1 second".into());
    }
    let capture: Value = serde_json::from_slice(&std::fs::read(&arguments[0])?)?;
    let initial = load(&capture)?;
    let restored = SurfaceFilm::from_state(&initial)?;
    let initial_mass = restored.total_mass();
    let measured = number(&capture["state"]["measurements"], "massKg")?;
    if initial_mass <= 0. || (initial_mass - measured).abs() > initial_mass * 1e-12 {
        return Err("checkpoint mass mismatch or empty film".into());
    }
    let volume = restored.total_volume();
    let distance =
        |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum::<f64>() / volume;
    let mut states = Vec::new();
    let mut runs = Vec::new();
    for requested_dt in [0.001, 0.0005, 0.00025, 0.000125] {
        let steps = (duration / requested_dt).ceil() as usize;
        let dt = duration / steps as f64;
        let mut film = SurfaceFilm::from_state(&initial)?;
        let start = std::time::Instant::now();
        for _ in 0..steps {
            film.step_with_max_substep(dt, [0., -9.81, 0.], dt)?;
        }
        let final_state = film.state();
        let error = (film.total_mass() - initial_mass).abs() / initial_mass;
        if error > 1e-12
            || final_state
                .cell_volumes_m3
                .iter()
                .any(|v| !v.is_finite() || *v < 0.)
        {
            return Err("mass/positivity replay gate failed".into());
        }
        runs.push(json!({"dtSeconds":dt,"steps":steps,"wallSeconds":start.elapsed().as_secs_f64(),
            "relativeMassError":error,"normalizedVolumeL1Change":distance(&initial.cell_volumes_m3,&final_state.cell_volumes_m3),
            "maximumThicknessM":film.thickness().into_iter().fold(0.,f64::max)}));
        states.push(final_state.cell_volumes_m3);
    }
    let errors: Vec<_> = states[..3]
        .iter()
        .map(|state| distance(state, &states[3]))
        .collect();
    let trend = errors[0] > 1e-14 && errors.windows(2).all(|pair| pair[1] < pair[0]);
    let report = json!({"captureSource":arguments[0],"capturedFrame":capture["frame"],"capturedTime":capture["simulationTime"],
        "cells":initial.triangles.len(),"simulationSeconds":duration,"initialMassKg":initial_mass,"runs":runs,
        "normalizedVolumeL1VsFinest":errors,"timeRefinementTrend":trend,
        "scope":"Actual captured initial film; fixed substrate; original material/wetting; gravity/capillarity/wetting only. Sources, self-contact exchange and body motion are not replayed. Finest step is a numerical reference, not exact physiology."});
    std::fs::write(&arguments[1], serde_json::to_vec_pretty(&report)?)?;
    println!("{report}");
    Ok(())
}
