//! A spinning elastic tetrahedron falling under gravity; SI units.
use physics::biomechanics::{Body, InertialBody, Material};
fn main() -> Result<(), &'static str> {
    let points = vec![[0.; 3], [0.1, 0., 0.], [0., 0.1, 0.], [0., 0., 0.1]];
    let velocities = points
        .iter()
        .map(|p| [-10. * (p[1] - 0.025), 10. * (p[0] - 0.025), 0.])
        .collect();
    let body = Body::new(
        points,
        vec![false; 4],
        vec![([0, 1, 2, 3], Material::from_young_poisson(1e5, 0.3)?)],
    )?;
    let mut body = InertialBody::new(body, &[1000.], velocities)?;
    body.set_uniform_acceleration([0., -9.81, 0.])?;
    let initial = body.diagnostics()?;
    println!("time_s,center_y_m,vy_m_s,total_energy_j,relative_energy_defect");
    for step in 0..=1000 {
        if step > 0 {
            body.step(0.0001, 1e-6)?;
        }
        if step % 100 == 0 {
            let d = body.diagnostics()?;
            let center_y: f64 = body
                .body()
                .positions()
                .iter()
                .zip(body.masses())
                .map(|(p, m)| p[1] * (m / d.mass_kg))
                .sum();
            let energy = d.kinetic_j + d.potential_j;
            let reference = initial.kinetic_j + initial.potential_j;
            println!(
                "{:.6},{:.9},{:.9},{:.9},{:.3e}",
                f64::from(step) * 0.0001,
                center_y,
                d.momentum_kg_m_s[1] / d.mass_kg,
                energy,
                (energy - reference) / reference
            );
        }
    }
    Ok(())
}
