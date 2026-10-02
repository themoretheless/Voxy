use physics::liquid::{
    Config, Liquid, LiquidField, Material, Particle, PhaseChange, TransportMaterial,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Synthetic coefficients make the latent plateau easy to inspect, not a water preset.
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 1.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            ..Config::default()
        },
    )?;
    liquid.configure_transport(
        vec![LiquidField {
            temperature: 5.0,
            concentration: 0.0,
        }],
        vec![TransportMaterial {
            specific_heat: 10.0,
            conductivity: 0.0,
            ..TransportMaterial::default()
        }],
    )?;
    liquid.configure_phase_change(
        vec![Some(PhaseChange {
            temperature: 10.0,
            latent_heat: 100.0,
            high_phase: Material::OIL,
        })],
        vec![0.0],
    )?;
    for delta in [0.0, 50.0, 50.0, 50.0, 50.0, -200.0] {
        liquid.add_heat(&[delta])?;
        println!(
            "energy={:.1}, temperature={:.1}, high_fraction={:.2}, density={:.1}",
            liquid.transport_totals()?.unwrap().0,
            liquid.fields().unwrap()[0].temperature,
            liquid.phase_fractions().unwrap()[0],
            liquid.effective_materials()?[0].rest_density
        );
    }
    assert!((liquid.fields().unwrap()[0].temperature - 5.0).abs() < 1e-12);
    assert!((liquid.transport_totals()?.unwrap().0 - 50.0).abs() < 1e-12);
    Ok(())
}
