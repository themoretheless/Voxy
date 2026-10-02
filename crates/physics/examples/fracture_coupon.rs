//! Two solid cubes joined by an energy-regularized cohesive crack plane.
use physics::cohesive;
#[path = "support/coupon.rs"]
mod support;
use support::{Coupon, coupon};
fn main() -> Result<(), &'static str> {
    let Coupon {
        mut body,
        mut prescribed,
        end,
        interface,
    } = coupon(cohesive::Material::new(1e9, 1e10, 1e5, 10.)?)?;
    let loads = vec![[0.; 3]; 16];
    let mut work = 0.;
    let mut previous_force = 0.;
    let mut previous_displacement = 0.;
    println!("Illustrative 0.01 m² crack; Gc=10 J/m²; peak=100 kPa; small strain/sliding.");
    println!("end_displacement_m,opening_m,reaction_n,damage,dissipated_j,work_j,residual_n");
    for step in 0..=40 {
        let opening = f64::from(step) * interface.failure_m() / 40.;
        let traction = if opening <= interface.onset_m() {
            1e9 * opening
        } else {
            1e5 * (interface.failure_m() - opening) / (interface.failure_m() - interface.onset_m())
        };
        let displacement = opening + 2. * 0.1 * traction / 1e9;
        for &i in &end {
            prescribed[i][0] = Some(displacement);
        }
        let result = body.equilibrate(&loads, &prescribed, 50, 1e-6)?;
        if !result.converged {
            return Err("fracture coupon did not converge");
        }
        let force: f64 = end.iter().map(|&i| result.reactions_n[i][0]).sum();
        work += 0.5 * (force + previous_force) * (displacement - previous_displacement);
        previous_force = force;
        previous_displacement = displacement;
        let reports = body.interface_reports()?;
        let damage = reports[0].quadrature[0].damage;
        let dissipated: f64 = reports.iter().map(|r| r.dissipated_j).sum();
        println!(
            "{displacement:.9},{opening:.9},{force:.6},{damage:.6},{dissipated:.9},{work:.9},{:.3e}",
            result.residual_n
        );
    }
    for &i in &end {
        prescribed[i][0] = Some(-1e-5);
    }
    let closed = body.equilibrate(&loads, &prescribed, 50, 1e-6)?;
    if !closed.converged {
        return Err("closed coupon did not converge");
    }
    let force: f64 = end.iter().map(|&i| closed.reactions_n[i][0]).sum();
    println!(
        "After break and reclosure: reaction={force:.6} N; damage={:.6}; dissipated={:.9} J",
        body.interface_reports()?[0].quadrature[0].damage,
        body.interface_reports()?
            .iter()
            .map(|r| r.dissipated_j)
            .sum::<f64>()
    );
    if (work - 0.1).abs() > 1e-9 {
        return Err("fracture energy balance failed");
    }
    Ok(())
}
