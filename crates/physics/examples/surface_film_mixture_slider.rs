//! Moving finite body entrains an equal-density mixture with local viscosity.
//! No squeeze pressure, elastic gel, film inertia or calibrated biological values.
use physics::liquid::{ThermalTranslatingBody, TranslatingBody};
use physics::surface_film::{FilmMixture, Material, SlidingPatch, SurfaceFilm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut film = SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material {
            viscosity: 0.05,
            wetting: 0.0,
            surface_tension: 0.0,
            ..Material::default()
        },
    )?;
    film.deposit(0, 0.05)?;
    film.deposit(1, 0.05)?;
    let mut mixture = FilmMixture::new(
        film,
        vec!["low_viscosity".into(), "high_viscosity".into()],
        vec![vec![1.0, 0.0], vec![0.0, 1.0]],
    )?;
    mixture.configure_viscosities(Some(vec![0.01, 0.1]))?;
    let mut body = ThermalTranslatingBody {
        mechanics: TranslatingBody {
            position: [0.5, 0.05, 0.5],
            velocity: [0.3, 0.0, 0.0],
            mass: 2.0,
        },
        specific_heat: 4.0,
        temperature: 300.0,
    };
    let energy = |b: ThermalTranslatingBody| -> Result<f64, Box<dyn std::error::Error>> {
        Ok(b.thermal_energy()?
            + 0.5 * b.mechanics.mass * b.mechanics.velocity.iter().map(|v| v * v).sum::<f64>())
    };
    let initial_energy = energy(body)?;
    let masses = mixture.component_masses()?;
    let initial_momentum = body.mechanics.mass * body.mechanics.velocity[0];
    let mut impulse = 0.0;
    let patch = SlidingPatch {
        normal: [0.0, 1.0, 0.0],
        tangent: [1.0, 0.0, 0.0],
        half_extents: [0.25, 0.25],
    };
    println!(
        "time,x,speed,temperature,high_viscosity_fraction_0,viscosity_0,viscosity_1,mean_wetted_area"
    );
    for i in 1..=10 {
        let report = mixture.advance_sliding_patch(0.1, patch, &mut body, 0.001)?;
        impulse += report.substrate_impulse[0];
        let viscosity = mixture.effective_viscosities();
        println!(
            "{},{},{},{},{},{},{},{}",
            i as f64 * 0.1,
            body.mechanics.position[0],
            body.mechanics.velocity[0],
            body.temperature,
            mixture.fractions()[0][1],
            viscosity[0],
            viscosity[1],
            report.mean_wetted_area
        );
    }
    let component_error = masses
        .iter()
        .zip(mixture.component_masses()?)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    let energy_error = (energy(body)? - initial_energy).abs();
    let momentum_error =
        (body.mechanics.mass * body.mechanics.velocity[0] + impulse - initial_momentum).abs();
    println!(
        "component_mass_error={component_error},body_energy_error={energy_error},momentum_error={momentum_error}"
    );
    if component_error > 1e-10 || energy_error > 1e-10 || momentum_error > 1e-12 {
        return Err("mixture/body conservation failed".into());
    }
    Ok(())
}
