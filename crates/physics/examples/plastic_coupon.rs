//! Load, yield and unload a tetrahedral solid. No visual window.
use physics::plasticity::{Material, mesh::Body};
fn main() -> Result<(), &'static str> {
    let material = Material::new(210e9, 0.3, 250e6, 1e9)?;
    let mut body = Body::new(
        vec![[0.; 3], [0.1, 0., 0.], [0., 0.1, 0.], [0., 0., 0.1]],
        vec![([0, 1, 2, 3], material)],
    )?;
    let mut supports = [
        [Some(0.); 3],
        [Some(0.), Some(0.), Some(0.)],
        [Some(0.), None, Some(0.)],
        [Some(0.), Some(0.), None],
    ];
    println!("Illustrative small-strain J2 coupon; E=210 GPa, nu=0.3, yield=250 MPa, H=1 GPa.");
    println!("strain,stress_pa,equivalent_plastic_strain,dissipated_j_m3,residual_n");
    for step in 0..=40 {
        let strain = f64::from(step) * 0.0001;
        supports[1][0] = Some(0.1 * strain);
        let equilibrium = body.equilibrate(&[[0.; 3]; 4], &supports, 40, 1e-5)?;
        if !equilibrium.converged {
            return Err("loaded coupon did not converge");
        }
        let state = body.states()[0];
        println!(
            "{strain:.6},{:.6},{:.9},{:.6},{:.3e}",
            body.responses()?[0].stress.cauchy_pa[0][0],
            state.equivalent_plastic_strain(),
            state.dissipated_j_m3(),
            equilibrium.residual_n
        );
    }
    supports[1][0] = None;
    let unloaded = body.equilibrate(&[[0.; 3]; 4], &supports, 40, 1e-5)?;
    if !unloaded.converged {
        return Err("unloaded coupon did not converge");
    }
    println!(
        "After removing force: residual extension={:.9} m; stress={:.6} Pa; residual={:.3e} N",
        body.positions()[1][0] - 0.1,
        body.responses()?[0].stress.cauchy_pa[0][0],
        unloaded.residual_n
    );
    Ok(())
}
