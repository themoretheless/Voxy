//! A finite upper plane squeezes a filled, vented Newtonian film.
use physics::liquid::{ThermalTranslatingBody, TranslatingBody};
use physics::surface_film::{Material, SqueezePressureControl, SurfaceFilm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut film = SurfaceFilm::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        Material {
            viscosity: 0.05,
            ..Material::default()
        },
    )?;
    film.deposit(0, 0.025)?;
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.25, 0.05, 0.25],
            velocity: [0.0, -0.03, 0.0],
            mass: 2.0,
        },
        specific_heat: 4.0,
        temperature: 300.0,
    };
    let initial = body;
    let report = film.advance_squeeze_body(
        0.1,
        [0.0, 1.0, 0.0],
        &mut body,
        SqueezePressureControl::default(),
        0.001,
    )?;
    let volume_error = (0.025 - film.total_volume() - report.transport.vented_volume).abs();
    let momentum_error = (2.0 * (body.mechanics.velocity[1] - initial.mechanics.velocity[1])
        + report.substrate_impulse[1])
        .abs();
    let energy_error = (body.thermal_energy()? - initial.thermal_energy()?
        + body.mechanics.velocity[1].powi(2)
        - initial.mechanics.velocity[1].powi(2))
    .abs();
    println!(
        "gap={}, closing_speed={}, temperature={}",
        body.mechanics.position[1], -body.mechanics.velocity[1], body.temperature
    );
    println!(
        "vented_volume={}, volume_error={volume_error}, momentum_error={momentum_error}, energy_error={energy_error}",
        report.transport.vented_volume
    );
    if volume_error > 1e-14 || momentum_error > 1e-14 || energy_error > 1e-10 {
        return Err("squeeze body conservation failed".into());
    }
    Ok(())
}
