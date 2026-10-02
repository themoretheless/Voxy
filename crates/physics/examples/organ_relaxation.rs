//! Synthetic material experiment: held-strain relaxation and load-controlled FEM creep.
//! This is not an anatomical liver or brain calibration.
use physics::biomechanics::{Body, Material, MaxwellBranch, OgdenTerm, ViscoelasticOgden};
fn material() -> Result<ViscoelasticOgden, &'static str> {
    ViscoelasticOgden::new(
        vec![
            OgdenTerm {
                shear_pa: 300.,
                exponent: -4.,
            },
            OgdenTerm {
                shear_pa: 100.,
                exponent: 2.,
            },
        ],
        5000.,
        vec![
            MaxwellBranch {
                shear_pa: 700.,
                relaxation_seconds: 0.2,
            },
            MaxwellBranch {
                shear_pa: 400.,
                relaxation_seconds: 2.,
            },
        ],
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut held = material()?;
    let mut body = Body::new(
        vec![[0., 0., 0.], [0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]],
        vec![true, false, true, true],
        vec![(
            [0, 1, 2, 3],
            Material {
                shear_pa: 400.,
                bulk_pa: 5000.,
                fibers: vec![],
            },
        )],
    )?;
    body.set_viscoelastic_ogden(0, material()?)?;
    body.set_force(1, [0.01, 0., 0.])?;
    let initial = body.equilibrate(4000, 1e-8)?;
    if !initial.converged {
        return Err("initial equilibrium did not converge".into());
    }
    let f = [[1., 0.2, 0.], [0., 1., 0.], [0., 0., 1.]];
    println!("time_s,held_shear_piola_pa,loaded_tip_mm,viscous_loss_j_per_m3,residual_n");
    println!(
        "0,{},{},0,{}",
        held.response(f, 0.)?.first_piola[0][1],
        body.positions()[1][0] * 1000.,
        initial.residual_n
    );
    for step in 1..=100 {
        let loss = held.advance(f, 0.02)?;
        let report = body.relax_step(0.02, 4000, 1e-8)?;
        println!(
            "{},{},{},{},{}",
            f64::from(step) * 0.02,
            held.response(f, 0.)?.first_piola[0][1],
            body.positions()[1][0] * 1000.,
            loss,
            report.residual_n
        );
    }
    Ok(())
}
