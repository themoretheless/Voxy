//! Prescribed sliding contact -> energetic dry dust -> heated SPH suspension.
//! A local solver demonstration, not a game-loop or airborne-smoke simulation.
use physics::friction::Material as ContactMaterial;
use physics::liquid::{
    Config, Liquid, LiquidField, Material, Particle, PhaseChange, TransportMaterial,
};
use physics::suspension::Particle as Grain;
use physics::wear::{Layer, Material as WearMaterial, WearSuspension, WearSuspensionInput};

fn kinetic(liquid: &Liquid, grains: &[Grain]) -> f64 {
    liquid
        .particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum::<f64>()
        + grains
            .iter()
            .map(|p| 0.5 * p.mass_kg() * p.velocity_m_s().iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let material = Material {
        rest_density: 1000.,
        sound_speed: 2.,
        viscosity: 0.001,
    };
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [0.; 3],
            velocity: [0.; 3],
            mass: 1e-5,
            material: 0,
        }],
        vec![material],
        Config {
            gravity: [0.; 3],
            ..Config::default()
        },
    )?;
    liquid.configure_transport(
        vec![LiquidField {
            temperature: 10.,
            concentration: 0.,
        }],
        vec![TransportMaterial {
            specific_heat: 1.,
            conductivity: 0.,
            ..TransportMaterial::default()
        }],
    )?;
    liquid.configure_phase_change(
        vec![Some(PhaseChange {
            temperature: 10.,
            latent_heat: 10_000.,
            high_phase: material,
        })],
        vec![0.],
    )?;
    liquid.set_viscous_heating(true)?;
    liquid.set_pressure_work(true)?;
    let initial_enthalpy = liquid.transport_totals()?.unwrap().0;
    let layer = Layer::new(0.01, 0.002, 2500.)?;
    let initial_mass = layer.remaining_mass_kg();
    let contact = ContactMaterial::new(1e9, 1e9, 0.5)?;
    let mut system =
        WearSuspension::new(layer, contact, WearMaterial::new(1e8, 1e-3)?, liquid, 100.)?;
    let mut surface_energy = 0.;
    let mut emitted_kinetic = 0.;
    let mut emitted_momentum = 0.;
    let mut numerical_drag = 0.;
    let mut driver_work = 0.;
    let mut last_gap = 0.;
    println!(
        "step,grains,remaining_solid_kg,dust_kg,surface_j,enthalpy_gain_j,drag_numerical_j,latent_fraction,mixture_density_kg_m3"
    );
    for step in 1..=20 {
        let gap = f64::from(step) * 1e-5;
        let positions = [system.liquid().particles()[0].position; 16];
        let report = system.step_contact(WearSuspensionInput {
            gap_m: [-1e-4, gap, 0.],
            normal: [1., 0., 0.],
            emission_positions_m: &positions,
            inherited_velocity_m_s: [0.001, 0., 0.],
            heat_weights: &[1.],
            dt_s: 1e-4,
            coupling_steps: 1,
        })?;
        let liquid = system.liquid();
        let grains = system.grains();
        let layer = system.layer();
        driver_work += report.contact.tangential_traction_pa[1] * 0.01 * (gap - last_gap);
        last_gap = gap;
        surface_energy += report.surface_energy_j;
        emitted_kinetic += report.emitted_kinetic_j;
        emitted_momentum += report.wear.mass_kg * 0.001;
        numerical_drag += report.drag.numerical_loss_j;
        let owned = system.energy();
        assert!((owned.surface_energy_j - surface_energy).abs() < 1e-15);
        assert!((owned.drag_numerical_loss_j - numerical_drag).abs() < 1e-25);
        assert!((owned.emission_kinetic_input_j - emitted_kinetic).abs() < 1e-25);
        let dust_mass: f64 = grains.iter().map(Grain::mass_kg).sum();
        let enthalpy = liquid.transport_totals()?.unwrap().0 - initial_enthalpy;
        let endpoint_contact_energy = (report.contact.tangential_stored_j_m2
            + report.contact.numerical_dissipated_j_m2)
            * 0.01;
        let accepted = endpoint_contact_energy
            + surface_energy
            + enthalpy
            + kinetic(liquid, grains)
            + numerical_drag;
        assert!((accepted - driver_work - emitted_kinetic).abs() < 1e-11);
        assert!((layer.remaining_mass_kg() + dust_mass - initial_mass).abs() < 1e-14);
        let momentum = liquid.particles()[0].mass * liquid.particles()[0].velocity[0]
            + grains
                .iter()
                .map(|p| p.mass_kg() * p.velocity_m_s()[0])
                .sum::<f64>();
        assert!((momentum - emitted_momentum).abs() < 1e-21);
        let cell = liquid.suspension_inventory(grains)?[0];
        println!(
            "{step},{},{:.12e},{dust_mass:.12e},{surface_energy:.12e},{enthalpy:.12e},{numerical_drag:.12e},{:.12e},{:.12e}",
            grains.len(),
            layer.remaining_mass_kg(),
            liquid.phase_fractions().unwrap()[0],
            cell.mixture_density_kg_m3
        );
    }
    Ok(())
}
