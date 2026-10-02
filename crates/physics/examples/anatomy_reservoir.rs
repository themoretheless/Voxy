//! Synthetic pressure reservoir on one actual lung boundary triangle.
//! The chosen triangle is a numerical port, not an identified vessel opening.
use physics::biomechanics::*;
fn main() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/anatomy/hra-female/tetrahedra/right-lung-middle-envelope.vxtet");
    let mesh = TetraMesh::from_bytes(&std::fs::read(path).unwrap()).unwrap();
    let port = mesh.boundary[0];
    let n = mesh.points.len();
    let body = mesh
        .into_body(
            vec![true; n],
            &Material::from_young_poisson(8000., 0.3).unwrap(),
        )
        .unwrap();
    let storage: Vec<_> = body
        .stresses_at(body.positions())
        .unwrap()
        .iter()
        .map(|s| s.reference_volume_m3 / 100_000.)
        .collect();
    let count = storage.len();
    let k = [[1e-12, 0., 0.], [0., 1e-12, 0.], [0., 0., 1e-12]];
    let external_pressure = 100.;
    let flow = MixedDarcy::new(
        body.positions().to_vec(),
        body.elements().iter().map(|e| e.nodes).collect(),
        &vec![k; count],
        0.001,
        &[(port, external_pressure)],
    )
    .unwrap();
    let old = vec![20.; count];
    let dt = 0.001;
    let (pressure, response) = flow.implicit_storage_step(&old, &storage, dt).unwrap();
    let boundary_outflow: f64 = flow
        .faces()
        .iter()
        .zip(&response.face_flows_m3_per_s)
        .filter(|(f, _)| f.neighbor.is_none())
        .map(|(_, q)| q)
        .sum();
    let content_change: f64 = pressure
        .iter()
        .zip(&storage)
        .map(|(p, s)| s * (p - 20.))
        .sum();
    let energy_change: f64 = pressure
        .iter()
        .zip(&storage)
        .map(|(p, s)| 0.5 * s * (p - 20.) * (p + 20.))
        .sum();
    let time_loss: f64 = pressure
        .iter()
        .zip(&storage)
        .map(|(p, s)| 0.5 * s * (p - 20.).powi(2))
        .sum();
    let boundary_work = -dt * external_pressure * boundary_outflow;
    let ledger = boundary_work - energy_change - dt * response.dissipation_w - time_loss;
    assert!(content_change > 0. && boundary_outflow < 0.);
    assert!((content_change + dt * boundary_outflow).abs() < 1e-8 * content_change);
    assert!(ledger.abs() < 1e-8 * boundary_work);
    let reference = body.stresses_at(body.positions()).unwrap();
    let old_mass: Vec<_> = reference
        .iter()
        .zip(&storage)
        .map(|(s, storage)| 10. * (0.5 * s.reference_volume_m3 + 20. * storage))
        .collect();
    let new_volumes: Vec<_> = reference
        .iter()
        .zip(&storage)
        .zip(&pressure)
        .map(|((s, storage), p)| 0.5 * s.reference_volume_m3 + storage * p)
        .collect();
    let concentrations: Vec<_> = flow
        .faces()
        .iter()
        .map(|f| {
            if f.neighbor.is_none() {
                Some(20.)
            } else {
                None
            }
        })
        .collect();
    let protein = flow
        .implicit_protein_step_with_boundary(
            &old_mass,
            &new_volumes,
            &response.face_flows_m3_per_s,
            dt,
            &concentrations,
        )
        .unwrap();
    let protein_gain: f64 = protein.iter().zip(&old_mass).map(|(a, b)| a - b).sum();
    let expected_gain = -dt * 20. * boundary_outflow;
    let protein_ledger = protein_gain - expected_gain;
    assert!(protein_gain > 0. && protein.iter().all(|m| *m >= 0.));
    assert!(protein_ledger.abs() < 1e-12 * old_mass.iter().sum::<f64>());
    println!(
        "protein_gain_kg={protein_gain:.12e} protein_boundary_ledger_error_kg={protein_ledger:.12e}"
    );
    println!(
        "cells={count} numerical_port={port:?} iterations={} fluid_gain_m3={content_change:.12e} boundary_work_j={boundary_work:.12e} energy_ledger_error_j={ledger:.12e}",
        response.solver_iterations
    );
    // Drive the same numerical port in the opposite direction from accepted state.
    let draining = MixedDarcy::new(
        body.positions().to_vec(),
        body.elements().iter().map(|e| e.nodes).collect(),
        &vec![k; count],
        0.001,
        &[(port, 0.)],
    )
    .unwrap();
    let (next_pressure, next_flux) = draining
        .implicit_storage_step(&pressure, &storage, dt)
        .unwrap();
    let next_volumes: Vec<_> = reference
        .iter()
        .zip(&storage)
        .zip(&next_pressure)
        .map(|((s, storage), p)| 0.5 * s.reference_volume_m3 + storage * p)
        .collect();
    // No reservoir protein concentration is needed for a genuine external outflow.
    let next_protein = draining
        .implicit_protein_step_with_boundary(
            &protein,
            &next_volumes,
            &next_flux.face_flows_m3_per_s,
            dt,
            &vec![None; draining.faces().len()],
        )
        .unwrap();
    let external_water: f64 = draining
        .faces()
        .iter()
        .zip(&next_flux.face_flows_m3_per_s)
        .filter(|(f, _)| f.neighbor.is_none())
        .map(|(_, q)| q)
        .sum();
    let external_protein: f64 = draining
        .faces()
        .iter()
        .zip(&next_flux.face_flows_m3_per_s)
        .filter(|(f, _)| f.neighbor.is_none())
        .map(|(f, q)| q * next_protein[f.owner] / next_volumes[f.owner])
        .sum();
    let volume_change: f64 = next_pressure
        .iter()
        .zip(&pressure)
        .zip(&storage)
        .map(|((a, b), s)| s * (a - b))
        .sum();
    let mass_change: f64 = next_protein.iter().zip(&protein).map(|(a, b)| a - b).sum();
    assert!(external_water > 0. && external_protein > 0. && volume_change < 0. && mass_change < 0.);
    assert!((volume_change + dt * external_water).abs() < 1e-8 * volume_change.abs());
    assert!((mass_change + dt * external_protein).abs() < 1e-12 * protein.iter().sum::<f64>());
    let energy_change: f64 = next_pressure
        .iter()
        .zip(&pressure)
        .zip(&storage)
        .map(|((a, b), s)| 0.5 * s * (a - b) * (a + b))
        .sum();
    let temporal_loss: f64 = next_pressure
        .iter()
        .zip(&pressure)
        .zip(&storage)
        .map(|((a, b), s)| 0.5 * s * (a - b).powi(2))
        .sum();
    let energy_error = energy_change + dt * next_flux.dissipation_w + temporal_loss;
    assert!(energy_error.abs() < 1e-8 * energy_change.abs());
    println!(
        "reversed_fluid_change_m3={volume_change:.12e} reversed_protein_change_kg={mass_change:.12e} reversed_energy_error_j={energy_error:.12e}"
    );
    let reservoir_compliance = 1e-14;
    let (finite_pressure, reservoir_pressure, finite_flux) = flow
        .implicit_reservoir_step(&old, &storage, dt, external_pressure, reservoir_compliance)
        .unwrap();
    let cell_gain: f64 = finite_pressure
        .iter()
        .zip(&storage)
        .map(|(p, s)| s * (p - 20.))
        .sum();
    let reservoir_gain = reservoir_compliance * (reservoir_pressure - external_pressure);
    let energy_change: f64 = finite_pressure
        .iter()
        .zip(&storage)
        .map(|(p, s)| 0.5 * s * (p - 20.) * (p + 20.))
        .sum::<f64>()
        + 0.5
            * reservoir_compliance
            * (reservoir_pressure - external_pressure)
            * (reservoir_pressure + external_pressure);
    let temporal_loss: f64 = finite_pressure
        .iter()
        .zip(&storage)
        .map(|(p, s)| 0.5 * s * (p - 20.).powi(2))
        .sum::<f64>()
        + 0.5 * reservoir_compliance * (reservoir_pressure - external_pressure).powi(2);
    let energy_error = energy_change + dt * finite_flux.dissipation_w + temporal_loss;
    assert!(cell_gain > 0. && reservoir_gain < 0. && reservoir_pressure < external_pressure);
    assert!((cell_gain + reservoir_gain).abs() < 1e-6 * cell_gain);
    assert!(energy_error.abs() < 1e-8 * energy_change.abs());
    println!(
        "finite_reservoir_pressure_pa={reservoir_pressure:.12e} tissue_fluid_gain_m3={cell_gain:.12e} combined_fluid_drift_m3={:.12e} combined_energy_error_j={energy_error:.12e}",
        cell_gain + reservoir_gain
    );
    let old_reservoir_volume = 1e-9;
    let new_reservoir_volume = old_reservoir_volume + reservoir_gain;
    let finite_volumes: Vec<_> = reference
        .iter()
        .zip(&storage)
        .zip(&finite_pressure)
        .map(|((s, storage), p)| 0.5 * s.reference_volume_m3 + storage * p)
        .collect();
    let initial_reservoir_protein = 20. * old_reservoir_volume;
    let (finite_protein, reservoir_protein) = flow
        .implicit_protein_reservoir_step(
            &old_mass,
            &finite_volumes,
            &finite_flux.face_flows_m3_per_s,
            dt,
            initial_reservoir_protein,
            new_reservoir_volume,
        )
        .unwrap();
    let tissue_gain: f64 = finite_protein
        .iter()
        .zip(&old_mass)
        .map(|(a, b)| a - b)
        .sum();
    let reservoir_change = reservoir_protein - initial_reservoir_protein;
    let protein_drift = tissue_gain + reservoir_change;
    assert!(tissue_gain > 0. && reservoir_change < 0.);
    assert!(
        protein_drift.abs() < 1e-12 * (old_mass.iter().sum::<f64>() + initial_reservoir_protein)
    );
    assert!(finite_protein.iter().all(|m| *m >= 0.) && reservoir_protein >= 0.);
    println!(
        "finite_tissue_protein_gain_kg={tissue_gain:.12e} finite_reservoir_protein_change_kg={reservoir_change:.12e} combined_protein_drift_kg={protein_drift:.12e}"
    );
    let mut persistent = PoreReservoir {
        reference_volume_m3: old_reservoir_volume - reservoir_compliance * external_pressure,
        reference_pressure_pa: 0.,
        compliance_m3_per_pa: reservoir_compliance,
        fluid_volume_m3: old_reservoir_volume,
        protein_kg: initial_reservoir_protein,
    };
    let mut tissue_pressure = old.clone();
    let mut tissue_fluid: Vec<_> = old_mass.iter().map(|m| m / 10.).collect();
    let mut tissue_protein = old_mass.clone();
    let initial_water = tissue_fluid.iter().sum::<f64>() + persistent.fluid_volume_m3;
    let initial_mass = tissue_protein.iter().sum::<f64>() + persistent.protein_kg;
    for step in 1..=3 {
        flow.implicit_reservoir_transport_step(
            &mut tissue_pressure,
            &mut tissue_fluid,
            &mut tissue_protein,
            &storage,
            &mut persistent,
            dt,
        )
        .unwrap();
        println!(
            "persistent_step={step} reservoir_pressure_pa={:.12e} reservoir_volume_m3={:.12e}",
            persistent.pressure_pa().unwrap(),
            persistent.fluid_volume_m3
        );
    }
    let water_drift = tissue_fluid.iter().sum::<f64>() + persistent.fluid_volume_m3 - initial_water;
    let mass_drift = tissue_protein.iter().sum::<f64>() + persistent.protein_kg - initial_mass;
    assert!(water_drift.abs() < 1e-10 * initial_water && mass_drift.abs() < 1e-12 * initial_mass);
    println!(
        "persistent_combined_fluid_drift_m3={water_drift:.12e} persistent_combined_protein_drift_kg={mass_drift:.12e}"
    );
}
