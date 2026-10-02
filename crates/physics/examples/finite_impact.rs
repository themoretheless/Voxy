//! Spinning deformable tetrahedron striking a stationary frictionless plane.
use physics::biomechanics::{Body, InertialBody, Material, PlaneContact};
fn main() -> Result<(), &'static str> {
    let points = vec![
        [0., 0.01, 0.],
        [0.1, 0.01, 0.],
        [0., 0.11, 0.],
        [0., 0.01, 0.1],
    ];
    let velocities = points
        .iter()
        .map(|p| [-10. * (p[1] - 0.035), -1. + 10. * (p[0] - 0.025), 0.])
        .collect();
    let body = Body::new(
        points,
        vec![false; 4],
        vec![([0, 1, 2, 3], Material::from_young_poisson(1e5, 0.3)?)],
    )?;
    let mut body = InertialBody::new(body, &[1000.], velocities)?;
    let stiffness = 80_000.;
    body.set_plane_contact(Some(PlaneContact::new([0., 1., 0.], 0., stiffness)?))?;
    let initial = body.diagnostics()?;
    let reference = initial.kinetic_j + initial.potential_j;
    let dt = 1e-5;
    let reaction = |body: &InertialBody| -> f64 {
        body.body()
            .positions()
            .iter()
            .map(|p| -stiffness * p[1].min(0.))
            .sum()
    };
    let mut impulse = 0.;
    let mut penetration = 0_f64;
    let mut worst_energy = 0_f64;
    println!(
        "time_s,center_y_m,vy_m_s,min_gap_m,plane_force_n,contact_j,mises_pa,relative_energy_error"
    );
    for step in 0..=6000 {
        if step > 0 {
            let old_force = reaction(&body);
            body.step(dt, 1e-5)?;
            impulse += 0.5 * dt * (old_force + reaction(&body));
        }
        let d = body.diagnostics()?;
        let gap = body
            .body()
            .positions()
            .iter()
            .map(|p| p[1])
            .fold(f64::INFINITY, f64::min);
        let relative = (d.kinetic_j + d.potential_j - reference) / reference;
        worst_energy = worst_energy.max(relative.abs());
        penetration = penetration.max(-gap);
        if step % 300 == 0 {
            let center: f64 = body
                .body()
                .positions()
                .iter()
                .zip(body.masses())
                .map(|(p, m)| p[1] * (m / d.mass_kg))
                .sum();
            let mises = body
                .body()
                .stresses_at(body.body().positions())?
                .iter()
                .map(|s| s.stress.von_mises_pa)
                .fold(0_f64, f64::max);
            println!(
                "{:.6},{center:.9},{:.9},{gap:.9},{:.6},{:.9},{mises:.6},{relative:.3e}",
                f64::from(step) * dt,
                d.momentum_kg_m_s[1] / d.mass_kg,
                reaction(&body),
                d.contact_j
            );
        }
    }
    let final_state = body.diagnostics()?;
    let impulse_error = final_state.momentum_kg_m_s[1] - initial.momentum_kg_m_s[1] - impulse;
    if worst_energy > 1e-3 || impulse_error.abs() > 1e-8 {
        return Err("impact verification failed");
    }
    eprintln!(
        "peak penetration={penetration:.6e} m; max relative energy deviation={worst_energy:.6e}; reaction impulse error={impulse_error:.3e} kg m/s"
    );
    Ok(())
}
