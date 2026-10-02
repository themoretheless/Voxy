//! Fracture, close, then tangentially load two slabs through stick and slip.
use physics::cohesive;
#[path = "support/coupon.rs"]
mod support;
use support::{Coupon, coupon};
fn main() -> Result<(), &'static str> {
    let interface = cohesive::Material::new(1e9, 1e10, 1e5, 10.)?.with_friction(0.5, 1e9)?;
    let Coupon {
        mut body,
        mut prescribed,
        end,
        interface,
    } = coupon(interface)?;
    let rest = body.positions().to_vec();
    let loads = vec![[0.; 3]; rest.len()];
    for &i in &end {
        prescribed[i][0] = Some(interface.failure_m());
    }
    if !body.equilibrate(&loads, &prescribed, 50, 1e-6)?.converged {
        return Err("fracture did not converge");
    }
    let pressure = 1e10 * 1e-5 / (1. + 2. * 0.1 * 1e10 / 1e9);
    for (i, point) in rest.iter().enumerate() {
        let displacement = if i < rest.len() / 2 {
            -pressure / 1e9 * (point[0] + 0.1)
        } else {
            -1e-5 + pressure / 1e9 * (0.1 - point[0])
        };
        let at_outer = prescribed[i][0].is_some();
        prescribed[i] = [
            Some(displacement),
            if at_outer { Some(0.) } else { None },
            Some(0.),
        ];
    }
    if !body.equilibrate(&loads, &prescribed, 50, 1e-6)?.converged {
        return Err("closure did not converge");
    }
    println!(
        "Illustrative small-sliding slabs; prescribed uniform pressure={pressure:.6} Pa; mu=0.5; area=0.01 m²."
    );
    println!("end_slide_m,shear_reaction_n,friction_work_j,numerical_defect_j,residual_n");
    for step in 0..=10 {
        let displacement = f64::from(step) * 5e-6;
        for &i in &end {
            prescribed[i][1] = Some(displacement);
        }
        let equilibrium = body.equilibrate(&loads, &prescribed, 50, 1e-6)?;
        if !equilibrium.converged {
            return Err("shear load did not converge");
        }
        let force: f64 = end.iter().map(|&i| equilibrium.reactions_n[i][1]).sum();
        let reports = body.interface_reports()?;
        let dissipated: f64 = reports.iter().map(|r| r.friction_dissipated_j).sum();
        let numerical: f64 = reports.iter().map(|r| r.friction_numerical_j).sum();
        println!(
            "{displacement:.9},{force:.6},{dissipated:.9},{numerical:.9},{:.3e}",
            equilibrium.residual_n
        );
        let expected = (displacement / (1. / 1e9 + 2. * 0.1 / 5e8)).min(0.5 * pressure) * 0.01;
        if (force - expected).abs() > 1e-5 {
            return Err("Coulomb slab force mismatch");
        }
    }
    Ok(())
}
