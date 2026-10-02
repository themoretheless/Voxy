//! Equal-density composition transport, diffusion and Newtonian viscosity feedback.
use physics::surface_film::{FilmMixture, Material, SurfaceFilm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "advection".into());
    if !matches!(mode.as_str(), "advection" | "diffusion" | "viscosity") {
        return Err("expected advection, diffusion or viscosity".into());
    }
    let mut film = SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material {
            surface_tension: 0.0,
            wetting: 0.0,
            ..Material::default()
        },
    )?;
    film.deposit(0, 0.0005)?;
    film.deposit(1, 0.0002)?;
    let mut mixture = FilmMixture::new(
        film,
        vec!["aqueous".into(), "gel_tracer".into()],
        vec![vec![0.25, 0.75], vec![1.0, 0.0]],
    )?;
    if mode == "viscosity" {
        mixture.configure_viscosities(Some(vec![0.001, 1.0]))?;
    }
    let initial = mixture.component_masses()?;
    println!("time,height_0,height_1,gel_fraction_0,gel_fraction_1,viscosity_0,viscosity_1");
    for i in 0..=10 {
        let h = mixture.film().thickness();
        let fractions = mixture.fractions();
        let viscosity = mixture.effective_viscosities();
        println!(
            "{},{},{},{},{},{},{}",
            i as f64 * 0.1,
            h[0],
            h[1],
            fractions[0][1],
            fractions[1][1],
            viscosity[0],
            viscosity[1]
        );
        if i < 10 {
            if mode == "diffusion" {
                // Deliberately large illustrative D; this is not a material calibration.
                mixture.diffuse(0.1, 2.0, 0.001)?;
            } else if mode == "viscosity" {
                mixture.step_with_surface_shear(
                    0.1,
                    [-1.0, 0.0, 0.0],
                    &[[1.0, 0.0, 0.0]; 2],
                    0.001,
                )?;
            } else {
                mixture.step_with_advection(0.1, [0.0; 3], &[[-1.0, 0.0, 0.0]; 2], 0.001)?;
            }
        }
    }
    let error = initial
        .iter()
        .zip(mixture.component_masses()?)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    println!("component_mass_error={error}");
    if error > 1e-12 {
        return Err("component mass conservation failed".into());
    }
    Ok(())
}
