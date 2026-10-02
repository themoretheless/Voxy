//! Inspect force residuals of a synthetic compliant-wall specimen before circuit coupling.
use physics::biomechanics::{Fiber, Material, tube};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mat = Material {
        shear_pa: 2000.,
        bulk_pa: 20000.,
        fibers: vec![Fiber {
            direction: [1., 0., 0.],
            stiffness_pa: 1000.,
            exponent: 3.,
            active_pa: 10000.,
        }],
    };
    let base = tube(&[0.01, 0.014], 0.025, 8, 1, &[mat], true)?;
    for activation in [0., 0.15] {
        for pressure in [0., 1., 100., 500.] {
            let mut body = base.clone();
            body.set_pressure(0, pressure)?;
            for i in 0..body.elements().len() {
                body.set_activation(i, activation)?;
            }
            let report = body.equilibrate(4000, 1e-8)?;
            println!(
                "activation {activation} pressure {pressure}: {report:?}, cavity {}",
                body.cavity_volume(0)?
            );
        }
    }
    Ok(())
}
