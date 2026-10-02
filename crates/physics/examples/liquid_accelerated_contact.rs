use physics::surface_film::{Material, SurfaceFilm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let film = SurfaceFilm::new(
        &[[0.0, 2.0, 0.0], [1.0, 2.0, 0.0], [0.0, 2.0, 1.0]],
        vec![[0, 1, 2]],
        Material::default(),
    )?;
    let start = [0.2, 1.0, 0.2];
    if film.first_sphere_hit(start, start, 0.1)?.is_some() {
        return Err("endpoint chord unexpectedly hit".into());
    }
    let hit = film
        .first_accelerated_sphere_hit(start, [0.0, 4.0, 0.0], [0.0, -8.0, 0.0], 1.0, 0.1, 100)?
        .ok_or("parabolic contact missed")?;
    let expected = (1.0 - 0.1_f64.sqrt()) / 2.0;
    if (hit.time - expected).abs() > 1e-12 {
        return Err("contact time differs from analytical plane root".into());
    }
    println!(
        "chord_hit=false parabolic_hit=true time_fraction={} point={:?} normal={:?}",
        hit.time, hit.point, hit.normal
    );
    Ok(())
}
