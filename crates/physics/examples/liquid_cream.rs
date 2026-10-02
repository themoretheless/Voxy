//! Manufactured shear-field check of homogenized, regularized whipped-cream rheology.
use physics::liquid::{Config, Liquid, Particle, WhippedCreamProfile};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() > 4 {
        return Err("usage: liquid_cream [overrun [yield_stress_pa [reference_viscosity_pa_s [flow_index]]]]".into());
    }
    let parameter = |index: usize, default: f64| -> Result<f64, Box<dyn std::error::Error>> {
        args.get(index)
            .map_or(Ok(default), |value| Ok(value.parse::<f64>()?))
    };
    let overrun = parameter(0, 1.0)?;
    let mut profile = WhippedCreamProfile::DEMO.with_overrun(1000.0, overrun)?;
    profile.rheology.yield_stress = parameter(1, profile.rheology.yield_stress)?;
    profile.material.viscosity = parameter(2, profile.material.viscosity)?;
    profile.rheology.shear_thinning.flow_index =
        parameter(3, profile.rheology.shear_thinning.flow_index)?;
    // Diagnostics stay off stdout so the flow curve remains usable as CSV.
    eprintln!(
        "density_kg_m3={}, overrun={overrun}, yield_stress_pa={}, reference_viscosity_pa_s={}, flow_index={}",
        profile.material.rest_density,
        profile.rheology.yield_stress,
        profile.material.viscosity,
        profile.rheology.shear_thinning.flow_index
    );
    println!("rate_per_s,viscosity_pa_s,shear_stress_pa");
    for rate in [0.0_f64, 0.1, 1.0, 10.0, 100.0] {
        let mut particles = Vec::new();
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    particles.push(Particle {
                        position: [f64::from(x) * 0.1, f64::from(y) * 0.1, f64::from(z) * 0.1],
                        velocity: [rate * f64::from(y) * 0.1, 0.0, 0.0],
                        mass: 0.5,
                        material: 0,
                    });
                }
            }
        }
        let mut liquid = Liquid::new(
            particles,
            vec![profile.material],
            Config {
                smoothing_radius: 1.0,
                ..Config::default()
            },
        )?;
        liquid.configure_herschel_bulkley(&[Some(profile.rheology)])?;
        let viscosity = liquid.effective_materials()?[13].viscosity;
        println!("{rate},{viscosity},{}", viscosity * rate);
    }
    Ok(())
}
