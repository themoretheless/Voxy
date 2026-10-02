//! Finite-radius contact can precede, or occur without, a center crossing.
use physics::liquid::{Config, FilmRebound, Liquid, Material, Particle};
use physics::surface_film::{Material as FilmMaterial, SurfaceFilm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let film = SurfaceFilm::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        FilmMaterial::default(),
    )?;
    let start = [0.5, 0.5, -0.06];
    let end = [0.5, -0.5, -0.06];
    let hit = film
        .first_sphere_hit(start, end, 0.1)?
        .ok_or("missing edge contact")?;
    if film.first_segment_hit(start, end)?.is_some() {
        return Err("center path unexpectedly crosses the triangle".into());
    }
    let mut liquid = Liquid::new(
        vec![Particle {
            position: end,
            velocity: [0.0, -2.0, 0.0],
            mass: 0.001,
            material: 0,
        }],
        vec![Material::WATER],
        Config::default(),
    )?;
    let report = liquid.rebound_spheres_surface_film(
        &[start],
        &film,
        FilmRebound {
            restitution: 0.5,
            friction: 0.1,
        },
        &[0.1],
    )?;
    let outgoing = liquid.particles()[0];
    let kinetic = 0.5 * outgoing.mass * outgoing.velocity.iter().map(|v| v * v).sum::<f64>();
    let energy_error = (kinetic + report.dissipated_energy - 0.002).abs();
    let momentum_error = outgoing
        .velocity
        .iter()
        .zip(report.substrate_impulse)
        .zip([0.0, -0.002, 0.0])
        .map(|((v, p), initial)| (0.001 * v + p - initial).abs())
        .fold(0.0, f64::max);
    if energy_error > 1e-14 || momentum_error > 1e-14 {
        return Err("finite-radius impact ledger failed".into());
    }
    println!(
        "time={},normal={:?},position={:?},velocity={:?},energy_error={energy_error},momentum_error={momentum_error}",
        hit.time, hit.normal, outgoing.position, outgoing.velocity
    );
    Ok(())
}
