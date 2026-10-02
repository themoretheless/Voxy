//! Geometry-selected planar Newtonian contact; no squeeze-pressure solve.
use physics::liquid::{ThermalTranslatingBody, TranslatingBody};
use physics::surface_film::{Material, SlidingPatch, SurfaceFilm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut film = SurfaceFilm::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        Material {
            viscosity: 2.0,
            ..Material::default()
        },
    )?;
    film.deposit(0, 0.05)?;
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.25, 0.1, 0.25],
            velocity: [0.3, 0.0, 0.0],
            mass: 2.0,
        },
        specific_heat: 4.0,
        temperature: 300.0,
    };
    let energy = |b: ThermalTranslatingBody| {
        b.thermal_energy().unwrap()
            + 0.5 * b.mechanics.mass * b.mechanics.velocity.iter().map(|v| v * v).sum::<f64>()
    };
    let initial = energy(body);
    let patch = SlidingPatch {
        normal: [0.0, 1.0, 0.0],
        tangent: [1.0, 0.0, 0.0],
        half_extents: [0.25, 0.25],
    };
    println!("time,x,speed,temperature,wetted_area");
    for i in 0..10 {
        let report = film.advance_sliding_patch(0.1, patch, &mut body, 0.001)?;
        println!(
            "{},{},{},{},{}",
            (i + 1) as f64 * 0.1,
            body.mechanics.position[0],
            body.mechanics.velocity[0],
            body.temperature,
            report.mean_wetted_area
        );
    }
    let error = (energy(body) - initial).abs();
    println!("body_energy_error={error}");
    if error > 1e-10 {
        return Err("body kinetic/thermal conservation failed".into());
    }
    Ok(())
}
