//! Shear decay via frozen stages or ordinary stepping; terminal x walls are excluded.
use physics::liquid::{
    Config, Formulation, Liquid, LiquidField, Material, Particle, ReflectingBox, TransportMaterial,
};
use std::f64::consts::PI;
fn kinetic(fluid: &Liquid) -> f64 {
    fluid
        .particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum()
}
fn initial(n: u32) -> Result<Liquid, Box<dyn std::error::Error>> {
    let dx = 1.0 / f64::from(n);
    let mut ps = Vec::new();
    for x in 0..3 * n {
        for y in 0..n {
            for z in 0..n {
                let position = [x, y, z].map(|a| (f64::from(a) + 0.5) * dx);
                ps.push(Particle {
                    position,
                    velocity: [
                        (PI * position[1]).sin() * (PI * position[2]).sin(),
                        0.0,
                        0.0,
                    ],
                    mass: 1000.0 * dx.powi(3),
                    material: 0,
                });
            }
        }
    }
    let mut fluid = Liquid::new(
        ps,
        vec![Material {
            rest_density: 1000.0,
            viscosity: 1000.0,
            sound_speed: 1.0,
        }],
        Config {
            smoothing_radius: 2.5 * dx,
            gravity: [0.0; 3],
            max_particles: 20_000,
            max_pairs: 5_000_000,
            max_neighbor_checks: 50_000_000,
            ..Config::default()
        },
    )?;
    fluid.set_formulation(Formulation::RestVolumeWendland);
    fluid.set_reflecting_box(Some(ReflectingBox {
        min: [0.0; 3],
        max: [3.0, 1.0, 1.0],
    }))?;
    fluid.set_reflecting_no_slip(true)?;
    fluid.configure_transport(
        vec![
            LiquidField {
                temperature: 300.0,
                concentration: 0.0
            };
            fluid.particles().len()
        ],
        vec![TransportMaterial {
            specific_heat: 2.0,
            conductivity: 0.0,
            ..TransportMaterial::default()
        }],
    )?;
    Ok(fluid)
}
fn run(
    n: u32,
    steps: u32,
    symmetric: bool,
    full: bool,
) -> Result<[f64; 3], Box<dyn std::error::Error>> {
    let mut fluid = initial(n)?;
    if full {
        fluid.set_viscous_heating(true)?;
        fluid.set_pressure_work(true)?;
        fluid.set_viscous_integrator(physics::liquid::ViscousIntegrator::Symmetric)?;
    }
    let initial_ke = kinetic(&fluid);
    let initial_heat = fluid.transport_totals()?.unwrap().0;
    let duration = 0.01;
    for _ in 0..steps {
        if full {
            fluid.step(duration / f64::from(steps), None)?;
        } else if symmetric {
            fluid.relax_viscosity_symmetric(duration / f64::from(steps))?;
        } else {
            fluid.relax_viscosity(duration / f64::from(steps))?;
        }
    }
    let decay = (-2.0 * PI.powi(2) * duration).exp();
    let mut error = 0.0;
    let mut transverse = 0.0;
    let mut reference = 0.0;
    for p in fluid.particles() {
        if p.position[0] < 1.0 || p.position[0] > 2.0 {
            continue;
        }
        let target = decay * (PI * p.position[1]).sin() * (PI * p.position[2]).sin();
        error += (p.velocity[0] - target).powi(2);
        transverse += p.velocity[1].powi(2) + p.velocity[2].powi(2);
        reference += target.powi(2);
    }
    let energy_error =
        (kinetic(&fluid) - initial_ke + fluid.transport_totals()?.unwrap().0 - initial_heat).abs()
            / initial_ke;
    println!(
        "{n},{steps},{duration},{},{},{},{}",
        fluid.particles().len(),
        (error / reference).sqrt(),
        (transverse / reference).sqrt(),
        energy_error
    );
    Ok([
        (error / reference).sqrt(),
        (transverse / reference).sqrt(),
        energy_error,
    ])
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "resolution,steps,duration,particles,axial_relative_l2,transverse_relative_l2,relative_energy_error"
    );
    let mode = std::env::args().nth(1);
    let (symmetric, full) = match mode.as_deref() {
        None => (false, false),
        Some("symmetric") => (true, false),
        Some("full") => (true, true),
        _ => return Err("expected symmetric or full".into()),
    };
    for n in [6, 10, 14] {
        for steps in [8, 16, 32] {
            run(n, steps, symmetric, full)?;
        }
    }
    Ok(())
}

#[test]
fn shear_decay_refinement_reduces_splitting_error_and_conserves_energy() {
    let coarse = run(6, 8, false, false).unwrap();
    let fine = run(6, 32, false, false).unwrap();
    assert!(fine[0] < coarse[0]);
    assert!(fine[1] < 0.4 * coarse[1]);
    assert!(coarse[2] < 1e-9 && fine[2] < 1e-9);
}

#[test]
fn symmetric_shear_decay_reduces_transverse_error_and_conserves_energy() {
    let coarse = run(10, 8, true, false).unwrap();
    let fine = run(10, 16, true, false).unwrap();
    assert!(fine[0] < coarse[0]);
    assert!(fine[1] < 0.3 * coarse[1]);
    assert!(coarse[2] < 1e-9 && fine[2] < 1e-9);
}
