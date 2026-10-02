//! Inertial fracture of two cubes; momentum and energy diagnostics, no window.
use physics::{cohesive, plasticity::mesh::DynamicBody};
#[path = "support/coupon.rs"]
mod support;
fn main() -> Result<(), &'static str> {
    let support::Coupon {
        body,
        prescribed,
        end,
        interface,
    } = support::coupon(cohesive::Material::new(1e9, 1e10, 1e5, 10.)?)?;
    // The static fixture's boundary conditions are deliberately released here.
    let _ = (prescribed, end, interface);
    let mut velocities = vec![[0.; 3]; 16];
    for (node, velocity) in velocities.iter_mut().enumerate() {
        velocity[0] = if node < 8 { -0.5 } else { 0.5 };
    }
    let mut dynamic = DynamicBody::new(body, &[1000.; 12], velocities)?;
    let initial = dynamic.diagnostics()?.kinetic_j;
    let coarse = dynamic.step(
        0.0002,
        &[[0.; 3]; 16],
        [0.; 3],
        &[[None; 3]; 16],
        30,
        1e-5,
        1e-4,
    )?;
    if coarse.converged {
        return Err("coarse peak-skipping step should be rejected");
    }
    println!(
        "Rejected coarse step: force residual={:.3e} N; energy defect={:.9} J",
        coarse.residual_n,
        coarse
            .energy_defect_j
            .ok_or("missing energy rejection diagnostic")?
    );
    println!(
        "dt=5e-6 s; density=1000 kg/m³; initial kinetic={initial:.9} J; fracture target=0.1 J."
    );
    println!("time_s,fragments,kinetic_j,elastic_j,fracture_j,energy_defect_j,momentum_x");
    for step in 1..=320 {
        let report = dynamic.step(
            5e-6,
            &[[0.; 3]; 16],
            [0.; 3],
            &[[None; 3]; 16],
            30,
            1e-5,
            1e-4,
        )?;
        if !report.converged {
            return Err("dynamic fracture step rejected");
        }
        if step % 40 == 0 {
            let diagnostic = dynamic.diagnostics()?;
            let defect = diagnostic.kinetic_j
                + diagnostic.elastic_j
                + diagnostic.interface_stored_j
                + diagnostic.fracture_dissipated_j
                - initial;
            println!(
                "{:.9},{},{:.9},{:.9},{:.9},{defect:.6e},{:.6e}",
                f64::from(step) * 5e-6,
                dynamic.fragments()?.len(),
                diagnostic.kinetic_j,
                diagnostic.elastic_j,
                diagnostic.fracture_dissipated_j,
                diagnostic.momentum_kg_m_s[0]
            );
        }
    }
    let fragments = dynamic.fragments()?;
    if fragments.len() != 2 {
        return Err("cubes did not detach");
    }
    for (index, fragment) in fragments.iter().enumerate() {
        println!(
            "fragment {index}: mass={:.9} kg; center={:?} m; velocity={:?} m/s",
            fragment.mass_kg, fragment.center_m, fragment.velocity_m_s
        );
    }
    let diagnostic = dynamic.diagnostics()?;
    if (diagnostic.fracture_dissipated_j - 0.1).abs() > 1e-8
        || diagnostic.momentum_kg_m_s.iter().any(|v| v.abs() > 1e-8)
    {
        return Err("fracture/momentum validation failed");
    }
    Ok(())
}
