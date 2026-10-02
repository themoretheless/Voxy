//! Kinematic pure-Couette mean transport; no normal pressure/contact/body solve.
use physics::surface_film::{Material, SurfaceFilm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wall_speed = std::env::args()
        .nth(1)
        .map(|s| s.parse::<f64>())
        .transpose()?
        .unwrap_or(0.02);
    let mut film = SurfaceFilm::new(
        &[
            [0.0, 0.0, 0.0],
            [0.01, 0.0, 0.0],
            [0.01, 0.0, 0.01],
            [0.0, 0.0, 0.01],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Material {
            surface_tension: 0.0,
            wetting: 0.0,
            ..Material::default()
        },
    )?;
    film.deposit(0, 5e-8)?;
    let mass = film.total_mass();
    println!("time,first_height,second_height,mass");
    for i in 0..=20 {
        let h = film.thickness();
        println!("{},{},{},{}", i as f64 * 0.1, h[0], h[1], film.total_mass());
        if i < 20 {
            film.step_with_advection(0.1, [0.0; 3], &[[-wall_speed / 2.0, 0.0, 0.0]; 2], 0.001)?;
        }
    }
    let error = (film.total_mass() - mass).abs();
    println!("mass_error={error}");
    if error > 1e-15 {
        return Err("film entrainment mass balance failed".into());
    }
    Ok(())
}
