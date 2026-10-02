//! Conforming uniform subdivision replay of a captured piecewise-planar substrate.
#[allow(dead_code)]
#[path = "film_checkpoint_convergence.rs"]
mod checkpoint;
use physics::surface_film::{SurfaceFilm, SurfaceFilmState};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn refine(
    state: &SurfaceFilmState,
    parents: &[usize],
) -> Result<(SurfaceFilmState, Vec<usize>), &'static str> {
    if parents.len() != state.triangles.len() {
        return Err("parent-cell count mismatch");
    }
    let mut points = state.points.clone();
    let mut midpoints = BTreeMap::new();
    let mut triangles = Vec::new();
    let mut volumes = Vec::new();
    let mut donors = Vec::new();
    for (index, &[a, b, c]) in state.triangles.iter().enumerate() {
        let mut midpoint = |a: usize, b: usize| {
            let edge = (a.min(b), a.max(b));
            *midpoints.entry(edge).or_insert_with(|| {
                let point = std::array::from_fn(|axis| {
                    state.points[a][axis] * 0.5 + state.points[b][axis] * 0.5
                });
                let id = points.len();
                points.push(point);
                id
            })
        };
        let ab = midpoint(a, b);
        let bc = midpoint(b, c);
        let ca = midpoint(c, a);
        triangles.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
        volumes.extend([state.cell_volumes_m3[index] * 0.25; 4]);
        donors.extend([parents[index]; 4]);
    }
    Ok((
        SurfaceFilmState {
            points,
            triangles,
            cell_volumes_m3: volumes,
            material: state.material,
            wetting: state.wetting,
        },
        donors,
    ))
}
fn restrict(volumes: &[f64], parents: &[usize], count: usize) -> Vec<f64> {
    let mut result = vec![0.; count];
    for (&volume, &parent) in volumes.iter().zip(parents) {
        result[parent] += volume;
    }
    result
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.len() != 2 {
        return Err("usage: film_checkpoint_spatial CAPTURE.json REPORT.json".into());
    }
    let capture: Value = serde_json::from_slice(&std::fs::read(&arguments[0])?)?;
    let original = checkpoint::load(&capture)?;
    let initial = SurfaceFilm::from_state(&original)?;
    let count = original.triangles.len();
    let volume = initial.total_volume();
    let mass = initial.total_mass();
    if mass <= 0. {
        return Err("nonempty film required".into());
    }
    let measured = capture["state"]["measurements"]["massKg"]
        .as_f64()
        .ok_or("missing captured mass")?;
    if !measured.is_finite() || (measured - mass).abs() > mass * 1e-12 {
        return Err("captured mass mismatch".into());
    }
    let mut state = original;
    let mut parents: Vec<_> = (0..count).collect();
    let mut results = Vec::new();
    let mut runs = Vec::new();
    for level in 0..=2 {
        let mut film = SurfaceFilm::from_state(&state)?;
        let starting = restrict(&state.cell_volumes_m3, &parents, count);
        let initial_error = (film.total_mass() - mass).abs() / mass;
        if initial_error > 1e-12 {
            return Err("subdivision changed film mass".into());
        }
        let started = std::time::Instant::now();
        for _ in 0..80 {
            film.step_with_max_substep(0.00025, [0., -9.81, 0.], 0.00025)?;
        }
        let final_state = film.state();
        let restricted = restrict(&final_state.cell_volumes_m3, &parents, count);
        let error = (film.total_mass() - mass).abs() / mass;
        if error > 1e-12 {
            return Err("replay mass gate failed".into());
        }
        let base_wall = started.elapsed().as_secs_f64();
        let mut half_step = SurfaceFilm::from_state(&state)?;
        let half_started = std::time::Instant::now();
        for _ in 0..160 {
            half_step.step_with_max_substep(0.000125, [0., -9.81, 0.], 0.000125)?;
        }
        let half_state = half_step.state();
        if half_state
            .cell_volumes_m3
            .iter()
            .any(|v| !v.is_finite() || *v < 0.)
        {
            return Err("invalid half-step volume".into());
        }
        let half_mass_error = (half_step.total_mass() - mass).abs() / mass;
        if half_mass_error > 1e-12 {
            return Err("half-step replay mass gate failed".into());
        }
        let half_restricted = restrict(&half_state.cell_volumes_m3, &parents, count);
        let temporal_error = restricted
            .iter()
            .zip(&half_restricted)
            .map(|(a, b)| (a - b).abs())
            .sum::<f64>()
            / volume;
        runs.push(json!({"level":level,"cells":state.triangles.len(),"vertices":state.points.len(),
            "wallSeconds":base_wall,"initialRelativeMassError":initial_error,
            "halfStepWallSeconds":half_started.elapsed().as_secs_f64(),"halfStepRelativeMassError":half_mass_error,
            "normalizedVolumeL1DtVsHalfDt":temporal_error,
            "relativeMassError":error,"normalizedVolumeL1Change":restricted.iter().zip(&starting).map(|(a,b)|(a-b).abs()).sum::<f64>()/volume}));
        results.push(restricted);
        if level < 2 {
            (state, parents) = refine(&state, &parents)?;
        }
    }
    let distance =
        |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(a, b)| (a - b).abs()).sum::<f64>() / volume;
    let coarse = distance(&results[0], &results[2]);
    let fine = distance(&results[1], &results[2]);
    let report = json!({"captureSource":arguments[0],"capturedFrame":capture["frame"],"initialMassKg":mass,
        "simulationSeconds":0.02,"dtSeconds":0.00025,"runs":runs,
        "normalizedVolumeL1CoarseVsFinest":coarse,"normalizedVolumeL1FineVsFinest":fine,
        "halfDtSeconds":0.000125,
        "maxTemporalToSpatialErrorRatio":runs.iter().filter_map(|r|r["normalizedVolumeL1DtVsHalfDt"].as_f64()).fold(0_f64,f64::max)/fine,
        "spatialRefinementTrend":coarse>1e-14 && fine<coarse,
        "scope":"Fixed captured piecewise-planar surface with conforming 1-to-4 subdivisions and original material/wetting. Child volumes inherit parent thickness. Restricted comparisons; no geometry smoothing, source/contact/body motion replay or physiology calibration."});
    std::fs::write(&arguments[1], serde_json::to_vec_pretty(&report)?)?;
    println!("{report}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refinement_shares_midpoints_preserves_mass_and_restriction() {
        let state = SurfaceFilmState {
            points: vec![[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
            cell_volumes_m3: vec![4e-6, 2e-6],
            material: physics::surface_film::Material::default(),
            wetting: None,
        };
        let (fine, parents) = refine(&state, &[0, 1]).unwrap();
        assert_eq!(fine.points.len(), 9);
        assert_eq!(fine.triangles.len(), 8);
        assert_eq!(
            restrict(&fine.cell_volumes_m3, &parents, 2),
            state.cell_volumes_m3
        );
        let film = SurfaceFilm::from_state(&fine).unwrap();
        assert_eq!(&film.thickness()[..4], &[8e-6; 4]);
        assert_eq!(&film.thickness()[4..], &[4e-6; 4]);
        assert!(refine(&state, &[]).is_err());
    }
}
