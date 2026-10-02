//! Illustrative fixed-normal dynamic wear; all material constants are demo inputs.
use physics::friction::Material as Contact;
use physics::liquid::{Config, Liquid, LiquidField, Material, Particle, TransportMaterial};
use physics::wear::{Layer, Material as Wear, WearSliderInput, WearSuspension};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let steps: u32 = std::env::args()
        .nth(1)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(20);
    if steps == 0 || steps > 200 {
        return Err("step count must be between 1 and 200".into());
    }
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [0.; 3],
            velocity: [0.; 3],
            mass: 1e-5,
            material: 0,
        }],
        vec![Material {
            rest_density: 1000.,
            sound_speed: 2.,
            viscosity: 0.001,
        }],
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
    liquid.set_viscous_heating(true)?;
    liquid.set_pressure_work(true)?;
    let mut system = WearSuspension::new_translating(
        Layer::new(0.01, 0.002, 2500.)?,
        Contact::new(1e9, 1e9, 1e-6)?,
        Wear::new(1e8, 1e-3)?,
        liquid,
        0.01,
        [0., 0.001, 0.],
    )?;
    system.initialize_slider([-1e-4, 0., 0.], [1., 0., 0.])?;
    let initial = system.energy();
    let initial_mass = system.layer().remaining_mass_kg();
    let initial_momentum = system.parent_momentum_kg_m_s().unwrap();
    let mut plane_impulse = [0.; 3];
    println!(
        "step,speed_m_s,grains,dust_kg,enthalpy_gain_j,surface_j,numerical_loss_j,energy_defect_j"
    );
    for step in 1..=steps {
        let positions = [system.liquid().particles()[0].position; 16];
        let result = system.step_sliding(WearSliderInput {
            emission_positions_m: &positions,
            heat_weights: &[1.],
            dt_s: 0.001,
            coupling_steps: 1,
        })?;
        for axis in 0..3 {
            plane_impulse[axis] -= result.slider.impulse_n_s[axis];
        }
        let energy = system.energy();
        let numerical = energy.drag_numerical_loss_j
            + energy.contact_numerical_loss_j
            + energy.parent_integration_numerical_loss_j;
        let defect = energy.parent_kinetic_j - initial.parent_kinetic_j + energy.kinetic_j
            - initial.kinetic_j
            + energy.enthalpy_j
            - initial.enthalpy_j
            + energy.suspension_heat_buffer_j
            - initial.suspension_heat_buffer_j
            + energy.surface_energy_j
            + energy.contact_spring_j
            + numerical;
        assert!(defect.abs() < 1e-17);
        assert_eq!(energy.friction_input_j, 0.);
        assert_eq!(energy.parent_impulse_work_j, 0.);
        let dust_mass: f64 = system.grains().iter().map(|p| p.mass_kg()).sum();
        assert!((system.layer().remaining_mass_kg() + dust_mass - initial_mass).abs() < 1e-14);
        for axis in 0..3 {
            let momentum = system.parent_momentum_kg_m_s().unwrap()[axis]
                + plane_impulse[axis]
                + system.liquid().particles()[0].mass
                    * system.liquid().particles()[0].velocity[axis]
                + system
                    .grains()
                    .iter()
                    .map(|p| p.mass_kg() * p.velocity_m_s()[axis])
                    .sum::<f64>();
            assert!((momentum - initial_momentum[axis]).abs() < 1e-18);
        }
        println!(
            "{step},{:.12e},{},{dust_mass:.12e},{:.12e},{:.12e},{numerical:.12e},{defect:.12e}",
            system.parent_velocity_m_s().unwrap()[1],
            system.grains().len(),
            energy.enthalpy_j - initial.enthalpy_j,
            energy.surface_energy_j
        );
    }
    Ok(())
}
