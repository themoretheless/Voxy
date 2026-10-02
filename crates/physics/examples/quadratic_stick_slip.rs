//! Pressure-balanced contact fixture: hold below mu*N, slide above it, unload.
//! cargo run -p physics --example quadratic_stick_slip > stick-slip.csv
use physics::{
    biomechanics::Material as Elastic,
    plasticity::{
        Material,
        mesh::{
            FiniteQuadraticDynamics, QuadraticAdvanceLimits, QuadraticBody, QuadraticPlaneContact,
            QuadraticPlaneCoulomb,
        },
    },
};
fn main() -> Result<(), &'static str> {
    let mesh = QuadraticBody::from_linear(
        vec![
            [0., -0.001, 0.],
            [0.1, -0.001, 0.],
            [0., 0.099, 0.],
            [0., -0.001, 0.1],
        ],
        vec![([0, 1, 2, 3], Material::new(1e5, 0.3, 1e9, 0.)?)],
    )?;
    let plane = QuadraticPlaneContact::new([0., 1., 0.], 0., 1e6)?;
    let normal = mesh.plane_contact_at(mesh.positions(), plane)?;
    let normal_force = normal.force_n[1];
    let mut body = FiniteQuadraticDynamics::new(
        mesh,
        vec![Elastic::from_young_poisson(1e5, 0.3)?],
        &[1000.],
        vec![[0.; 3]; 10],
        &[false; 10],
    )?;
    body.set_plane_contact(Some(plane))?;
    body.set_plane_coulomb(Some(QuadraticPlaneCoulomb::new(0.5, 1e-10, 5000, 4096)?))?;
    let initial = body.energy()?;
    let initial_energy = initial.kinetic_j + initial.elastic_j + initial.contact_j;
    let mut work = 0.;
    let mut elapsed = 0.;
    let mut worst = 0_f64;
    println!(
        "time_s,load_over_normal,mean_vx_m_s,friction_loss_j,projection_loss_diagnostic_j,energy_defect_j"
    );
    for ratio in [0.49, 0.51, 0.] {
        let loads: Vec<_> = normal
            .gradient_n
            .iter()
            .map(|g| [-ratio * g[1], g[1], 0.])
            .collect();
        for _ in 0..40 {
            let before = body.positions().to_vec();
            body.advance_loaded(
                1e-5,
                &loads,
                [0.; 3],
                QuadraticAdvanceLimits {
                    minimum_dt_s: 1e-9,
                    maximum_dt_s: 1e-5,
                    max_attempts: 1000,
                    energy_tolerance_j: 1e-8,
                },
            )?;
            elapsed += 1e-5;
            work += loads
                .iter()
                .zip(body.positions().iter().zip(before))
                .map(|(f, (p, q))| {
                    f.iter()
                        .zip(p.iter().zip(q))
                        .map(|(f, (p, q))| f * (p - q))
                        .sum::<f64>()
                })
                .sum::<f64>();
            let energy = body.energy()?;
            let defect = energy.kinetic_j
                + energy.elastic_j
                + energy.contact_j
                + energy.friction_dissipated_j
                - initial_energy
                - work;
            worst = worst.max(defect.abs());
            println!(
                "{elapsed:.8},{ratio:.3},{:.12e},{:.12e},{:.12e},{defect:.12e}",
                energy.momentum_kg_m_s[0] / energy.mass_kg,
                energy.friction_dissipated_j,
                energy.friction_numerical_j
            );
        }
    }
    eprintln!(
        "reference normal force={normal_force:e} N; mu=0.5; absolute energy envelope={worst:e} J"
    );
    Ok(())
}
