//! A 0.1 m radius sphere traverses an unfolded 3 m path in a 1 m corridor.
use physics::liquid::{Config, FilmRebound, Liquid, Material, Particle};
use physics::surface_film::{Material as FilmMaterial, SurfaceFilm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let points: Vec<_> = [0.0, 1.0]
        .into_iter()
        .flat_map(|x| {
            [
                [x, -2.0, -2.0],
                [x, 2.0, -2.0],
                [x, 2.0, 2.0],
                [x, -2.0, 2.0],
            ]
        })
        .collect();
    let film = SurfaceFilm::new(
        &points,
        vec![[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]],
        FilmMaterial::default(),
    )?;
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [3.5, 0.0, 0.0],
            velocity: [3.0, 0.0, 0.0],
            mass: 1.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config::default(),
    )?;
    let report = liquid.rebound_spheres_surface_film_multi(
        &[[0.5, 0.0, 0.0]],
        &film,
        FilmRebound {
            restitution: 1.0,
            friction: 0.0,
        },
        &[0.1],
        8,
    )?;
    let particle = liquid.particles()[0];
    let energy = 0.5 * particle.velocity.iter().map(|v| v * v).sum::<f64>();
    let momentum_error = particle
        .velocity
        .iter()
        .zip(report.rebound.substrate_impulse)
        .zip([3.0, 0.0, 0.0])
        .map(|((v, p), before)| (v + p - before).abs())
        .fold(0.0, f64::max);
    if report.contacts != 4
        || (particle.position[0] - 0.3).abs() > 1e-10
        || (energy + report.rebound.dissipated_energy - 4.5).abs() > 1e-12
        || momentum_error > 1e-12
    {
        return Err("multi-impact analytic/ledger check failed".into());
    }
    println!(
        "contacts={},position={:?},velocity={:?},heat={},momentum_error={momentum_error}",
        report.contacts, particle.position, particle.velocity, report.rebound.dissipated_energy
    );
    Ok(())
}
