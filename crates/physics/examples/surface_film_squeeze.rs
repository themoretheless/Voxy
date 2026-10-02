//! Vented squeeze pressure on an equilateral triangular fully filled gap.
//! Prescribed closing speed only; body motion and volume extrusion are not applied.
use physics::surface_film::{Material, SqueezePressureControl, SurfaceFilm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let n = std::env::args()
        .nth(1)
        .map(|s| s.parse::<usize>())
        .transpose()?
        .unwrap_or(32);
    if n == 0 || n > 128 {
        return Err("expected subdivisions in 1..128".into());
    }
    let transport = match std::env::args().nth(2).as_deref() {
        None | Some("pressure") => false,
        Some("transport") => true,
        _ => return Err("expected pressure or transport as second argument".into()),
    };
    let gap = 0.001_f64;
    let viscosity = 0.05;
    let speed = 1e-5;
    let mut points = Vec::new();
    let mut rows = Vec::new();
    for j in 0..=n {
        let row: Vec<_> = (0..=n - j)
            .map(|i| {
                let index = points.len();
                points.push([
                    (i as f64 + 0.5 * j as f64) / n as f64,
                    0.0,
                    j as f64 * 3.0_f64.sqrt() / (2.0 * n as f64),
                ]);
                index
            })
            .collect();
        rows.push(row);
    }
    let mut triangles = Vec::new();
    for j in 0..n {
        for i in 0..n - j {
            triangles.push([rows[j][i], rows[j][i + 1], rows[j + 1][i]]);
            if i + 1 < n - j {
                triangles.push([rows[j][i + 1], rows[j + 1][i + 1], rows[j + 1][i]]);
            }
        }
    }
    let cells = triangles.len();
    let mut film = SurfaceFilm::new(
        &points,
        triangles,
        Material {
            viscosity,
            ..Material::default()
        },
    )?;
    let cell_area = 3.0_f64.sqrt() / (4.0 * (n * n) as f64);
    for i in 0..cells {
        film.deposit(i, cell_area * gap)?;
    }
    let report = film.solve_squeeze_pressure(
        &vec![gap; cells],
        &vec![speed; cells],
        None,
        SqueezePressureControl::default(),
    )?;
    let mobility = gap.powi(3) / (12.0 * viscosity);
    let exact_load = speed * 3.0_f64.sqrt() / (320.0 * mobility);
    let exact_rate = speed * 3.0_f64.sqrt() / 4.0;
    let volume_error = (report.vented_volume_rate - exact_rate).abs() / exact_rate;
    let power_error = (report.dissipated_power - report.normal_load * speed).abs();
    println!(
        "cells={cells},iterations={},relative_residual={}",
        report.iterations, report.relative_residual
    );
    println!(
        "normal_load={},analytic_load={exact_load},relative_load_error={}",
        report.normal_load,
        (report.normal_load - exact_load).abs() / exact_load
    );
    println!(
        "vented_volume_rate={},relative_volume_balance_error={volume_error},dissipated_power={},pressure_work_error={power_error}",
        report.vented_volume_rate, report.dissipated_power
    );
    if volume_error > 1e-8 || power_error > 1e-8 * report.dissipated_power {
        return Err("squeeze pressure conservation failed".into());
    }
    if transport {
        let before = film.total_volume();
        let squeezed = film.step_squeeze(
            0.1,
            &vec![speed; cells],
            SqueezePressureControl::default(),
            0.001,
        )?;
        let volume_error = (film.total_volume() + squeezed.vented_volume - before).abs();
        let max_gap_error = film
            .thickness()
            .iter()
            .map(|h| (h - (gap - speed * 0.1)).abs())
            .fold(0.0, f64::max);
        let work_error = (squeezed.dissipated_energy - speed * squeezed.normal_impulse).abs();
        println!(
            "transport_substeps={},vented_volume={},normal_impulse={},dissipated_energy={},volume_error={volume_error},max_gap_error={max_gap_error},pressure_work_error={work_error}",
            squeezed.substeps,
            squeezed.vented_volume,
            squeezed.normal_impulse,
            squeezed.dissipated_energy
        );
        if volume_error > 1e-12 * before
            || max_gap_error > 1e-10 * gap
            || work_error > 1e-8 * squeezed.dissipated_energy
        {
            return Err("squeeze transport conservation failed".into());
        }
    }
    Ok(())
}
