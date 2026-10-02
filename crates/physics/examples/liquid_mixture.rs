//! Heterogeneous liquid mixtures with configurable viscosity.
//! This example combines mixture transport, shear thinning and structural kinetics.
use physics::liquid::{
    Config, FluidMixtureProfile, Liquid, LiquidField, Material, Particle, TransportMaterial,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "medium".into());
    let (fractions, viscosity, index) = match mode.as_str() {
        "medium" => ([0.01, 0.03, 0.08], 0.3, 0.7),
        "thick" | "mixed" => ([0.05, 0.2, 0.5], 2.0, 0.5),
        _ => return Err("expected medium, thick or mixed".into()),
    };
    let steps: usize = std::env::args()
        .nth(2)
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(100);
    if !(1..=10_000).contains(&steps) {
        return Err("expected 1..=10000 flow steps".into());
    }
    let particles = (0..3)
        .map(|i| Particle {
            position: [i as f64 * 0.2, 0.0, 0.0],
            velocity: [0.0; 3],
            mass: 1.0,
            material: 0,
        })
        .collect();
    let mut liquid = Liquid::new(
        particles,
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 1.0,
            ..Config::default()
        },
    )?;
    liquid.configure_transport(
        vec![
            LiquidField {
                temperature: 310.0,
                concentration: 0.0
            };
            3
        ],
        vec![TransportMaterial {
            specific_heat: 4000.0,
            conductivity: 0.0,
            diffusivity: 20.0,
            mixing_group: 0,
        }],
    )?;
    let rows = match mode.as_str() {
        "mixed" => vec![[0.97, 0.0, 0.03], [0.80, 0.15, 0.05], [0.50, 0.50, 0.0]],
        "medium" => fractions.iter().map(|y| [1.0 - y, 0.0, *y]).collect(),
        _ => fractions.iter().map(|y| [1.0 - y, *y, 0.0]).collect(),
    };
    let mut profile = FluidMixtureProfile::DEMO;
    profile.shear_thinning.flow_index = index;
    if mode == "medium" {
        profile.components[2].viscosity = viscosity;
    } else {
        profile.components[1].viscosity = viscosity;
    }
    liquid.configure_fluid_mixture(profile, rows, vec![1.0, 0.6, 0.2])?;
    let before = liquid.species_totals()?.unwrap();
    println!("synthetic_mode={mode}; coefficients are illustrative; no elastic stress");
    println!("stage,particle,component_mass_fractions,structure,density,apparent_viscosity");
    for stage in 0..=steps {
        let materials = liquid.effective_materials()?;
        for (i, m) in materials
            .iter()
            .enumerate()
            .filter(|_| stage == 0 || stage == steps)
        {
            println!(
                "{stage},{i},{:?},{},{},{}",
                liquid.species_fractions().unwrap()[i],
                liquid.structure_fractions()[i],
                m.rest_density,
                m.viscosity
            );
        }
        if stage < steps {
            liquid.step(0.01, None)?;
        }
    }
    let after = liquid.species_totals()?.unwrap();
    let error = before
        .iter()
        .zip(after)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    if error > 1e-12 {
        return Err("component mass conservation failed".into());
    }
    println!("component_mass_error={error}");
    Ok(())
}
