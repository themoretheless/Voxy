//! Headless source ledger; optional `flow` mode advances a circular-aperture jet.
use physics::liquid::{
    Config, EmissionPulse, Liquid, LiquidField, Particle, ParticleInput, PulsedEmitter,
    TransportMaterial, ViscosityProfile, ViscousIntegrator,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let flow = match std::env::args().nth(2).as_deref() {
        None => false,
        Some("flow") => true,
        Some(_) => return Err("second argument must be flow".into()),
    };
    let profile = match std::env::args().nth(1).as_deref() {
        None | Some("viscous") => ViscosityProfile::VISCOUS_DEMO,
        Some("fluid") => ViscosityProfile::FLUID_DEMO,
        Some("thick") => ViscosityProfile::THICK_DEMO,
        Some(_) => return Err("expected fluid, viscous or thick".into()),
    };
    let config = if flow {
        Config {
            smoothing_radius: 0.02,
            particle_radius: 0.001,
            max_substeps: 2048,
            ..Config::default()
        }
    } else {
        Config::default()
    };
    let mut liquid = Liquid::new(vec![], vec![profile.material], config)?;
    liquid.configure_shear_thinning(vec![Some(profile.shear_thinning)])?;
    if flow {
        liquid.configure_transport(
            vec![],
            vec![TransportMaterial {
                conductivity: 0.0,
                ..TransportMaterial::default()
            }],
        )?;
        liquid.set_viscous_heating(true)?;
        liquid.set_viscous_integrator(ViscousIntegrator::Midpoint)?;
    }
    let template = ParticleInput {
        particle: Particle {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 1.0,
            material: 0,
        },
        field: flow.then_some(LiquidField {
            temperature: 300.0,
            concentration: 0.0,
        }),
        phase_fraction: None,
    };
    let mut source = PulsedEmitter::new(
        vec![
            EmissionPulse {
                start: 0.0,
                duration: 0.1,
                volume: 1e-6,
                speed: 2.0,
            },
            EmissionPulse {
                start: 0.3,
                duration: 0.15,
                volume: 0.5e-6,
                speed: 1.0,
            },
        ],
        template,
    )?;
    source.density = profile.material.rest_density;
    source.particle_volume = 0.1e-6;
    source.nozzle_radius = 0.003;
    let mut mass = 0.0;
    let mut momentum = [0.0; 3];
    let dt = if flow { 0.005 } else { 0.05 };
    let steps = if flow { 100 } else { 10 };
    let mut expected_momentum = [0.0; 3];
    for _ in 0..steps {
        let report = source.advance(&mut liquid, dt)?;
        mass += report.added.mass;
        for (sum, increment) in momentum.iter_mut().zip(report.added.momentum) {
            *sum += increment;
        }
        for (sum, increment) in expected_momentum.iter_mut().zip(report.added.momentum) {
            *sum += increment;
        }
        if flow {
            for (sum, gravity) in expected_momentum.iter_mut().zip(config.gravity) {
                *sum += mass * gravity * dt;
            }
            liquid.step(dt, None)?;
        }
    }
    println!(
        "reference_viscosity={},volume_ml={},mass_kg={},momentum={:?},particles={}",
        profile.material.viscosity,
        mass / source.density * 1e6,
        mass,
        momentum,
        liquid.particles().len()
    );
    if flow {
        verify_flow(&liquid, mass, expected_momentum)?;
    }
    Ok(())
}
fn verify_flow(
    liquid: &Liquid,
    mass: f64,
    expected_momentum: [f64; 3],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut actual = [0.0; 3];
    let mut actual_mass = 0.0;
    for particle in liquid.particles() {
        actual_mass += particle.mass;
        for (sum, velocity) in actual.iter_mut().zip(particle.velocity) {
            *sum += particle.mass * velocity;
        }
    }
    let error = actual
        .iter()
        .zip(expected_momentum)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    println!(
        "flow_mass_error={},flow_momentum_error={error}",
        (actual_mass - mass).abs()
    );
    if (actual_mass - mass).abs() > 1e-12 || error > 1e-10 {
        return Err("source/flow conservation check failed".into());
    }
    Ok(())
}
