//! Imposed homogeneous stretch on an experimental constitutive point.
//! Coefficients are synthetic; no FEM geometry or experimental calibration.
use physics::biomechanics::{Fiber, HgoMaterial, ViscoelasticHgo};
fn main() -> Result<(), &'static str> {
    let mut law = ViscoelasticHgo::new(
        HgoMaterial {
            shear_pa: 3000.,
            bulk_pa: 50000.,
            fibers: vec![Fiber {
                direction: [1., 0., 0.],
                stiffness_pa: 9000.,
                exponent: 0.2,
                active_pa: 0.,
            }],
        },
        &[(31.75, 0.3), (334.03, 0.5)],
    )?;
    println!("time_s,stretch,first_piola_x_pa,cauchy_x_pa,incremental_energy_j_m3");
    // Ten-second ramp, then 900-second fixed-stretch hold, dt=0.1 s.
    // All deformations have J=1. No grip force or specimen shape is inferred.
    for k in 1..=9100 {
        let time = f64::from(k) * 0.1;
        let stretch = 1. + 0.2 * (time / 10.).min(1.);
        let transverse = 1. / stretch.sqrt();
        let f = [
            [stretch, 0., 0.],
            [0., transverse, 0.],
            [0., 0., transverse],
        ];
        let response = law.advance(f, 0.1)?;
        if k <= 100 || k % 10 == 0 {
            println!(
                "{time:.8},{stretch:.8},{:.12e},{:.12e},{:.12e}",
                response.first_piola[0][0],
                stretch * response.first_piola[0][0],
                response.energy_density
            );
        }
    }
    Ok(())
}
