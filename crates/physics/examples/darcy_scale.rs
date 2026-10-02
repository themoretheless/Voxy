//! Matrix-free RT0 anisotropic linear-pressure patch; numerical scaling specimen.
#[path = "support/darcy_block.rs"]
mod fixture;
use physics::biomechanics::*;
use std::time::Instant;
fn main() {
    let n = std::env::args()
        .nth(1)
        .map_or(8, |s| s.parse().expect("block subdivisions 1..32"));
    let data = fixture::block(n);
    let cells = data.cells.len();
    let tensors = vec![fixture::K; cells];
    let started = Instant::now();
    let model = MixedDarcy::new(
        data.points,
        data.cells,
        &tensors,
        fixture::MU,
        &data.boundaries,
    )
    .unwrap();
    let construction = started.elapsed();
    let started = Instant::now();
    let response = model.response(&data.pressures).unwrap();
    let time = started.elapsed();
    let expected = fixture::expected_velocity();
    let error = response
        .cell_centroid_velocities_m_per_s
        .iter()
        .flat_map(|v| (0..3).map(move |i| (v[i] - expected[i]).abs()))
        .fold(0_f64, f64::max);
    println!(
        "cells={cells} faces={} operator_bytes={} iterations={} construct_s={:.6} solve_s={:.6} max_velocity_error_m_s={error:.12e} residual_pa={:.12e}",
        model.faces().len(),
        model.operator_storage_bytes(),
        response.solver_iterations,
        construction.as_secs_f64(),
        time.as_secs_f64(),
        response.residual_pa
    );
    assert!(error < 1e-16);
}
