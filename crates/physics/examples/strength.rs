//! A load-controlled tetrahedral coupon with constrained transverse strain.
use physics::biomechanics::{Body, Material};
fn main() -> Result<(), &'static str> {
    let material = Material::from_young_poisson(2e6, 0.3)?;
    let modulus = material.bulk_pa + 4. * material.shear_pa / 3.;
    let length = 0.1;
    let area = length * length / 6.;
    let load = 1.;
    let mut body = Body::new(
        vec![
            [0.; 3],
            [length, 0., 0.],
            [0., length, 0.],
            [0., 0., length],
        ],
        vec![true, false, true, true],
        vec![([0, 1, 2, 3], material)],
    )?;
    body.set_force(1, [load, 0., 0.])?;
    let equilibrium = body.equilibrate(2000, 5e-6)?;
    if !equilibrium.converged {
        return Err("coupon did not converge");
    }
    let stress = body.stresses_at(body.positions())?[0].stress;
    println!("Illustrative hyperelastic coupon; SI units; transverse strain constrained.");
    println!(
        "load={load:.6} N; residual={:.3e} N",
        equilibrium.residual_n
    );
    println!(
        "extension={:.9} m; linear limit={:.9} m",
        body.positions()[1][0] - length,
        load * length / (area * modulus)
    );
    println!(
        "Cauchy axial={:.6} Pa; force/area={:.6} Pa",
        stress.cauchy_pa[0][0],
        load / area
    );
    println!(
        "principal={:?} Pa; von Mises={:.6} Pa",
        stress.principal_pa, stress.von_mises_pa
    );
    println!(
        "Principal spatial axes (columns)={:?}",
        stress.principal_directions
    );
    println!(
        "Yield utilization at illustrative 1000 Pa threshold={:.6}",
        stress.yield_utilization(1000.)?
    );
    println!(
        "Normal strength utilization at illustrative tensile 500 / compressive 5000 Pa thresholds={:.6}",
        stress.normal_strength_utilization(500., 5000.)?
    );
    Ok(())
}
