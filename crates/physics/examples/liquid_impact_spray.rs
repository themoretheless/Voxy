//! Explicit idealized splash: a prescribed fraction of impact loss feeds fragmentation.
//! No measured splash threshold, spatial gas flow or stored surface energy.
use physics::liquid::{
    Config, DropletGas, FilmRebound, ImpactSpray, Liquid, Material, Particle, VaporCell,
};
use physics::surface_film::{Material as FilmMaterial, SurfaceFilm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let finite_gas = match std::env::args().nth(1).as_deref() {
        None | Some("prescribed") => false,
        Some("finite") => true,
        Some(_) => return Err("expected prescribed or finite".into()),
    };
    let mut l = Liquid::new(
        vec![Particle {
            position: [0.0, -0.001, 0.0],
            velocity: [0.0, -2.0, 0.0],
            mass: 0.001,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 0.001,
            particle_radius: 0.00001,
            max_substeps: 4096,
            ..Config::default()
        },
    )?;
    let film = SurfaceFilm::new(
        &[[-1.0, 0.0, -1.0], [1.0, 0.0, -1.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        FilmMaterial::default(),
    )?;
    let initial = 0.5 * 0.001 * 4.0;
    let impact = l.impact_spray_surface_film(
        &[[0.0, 0.001, 0.0]],
        &film,
        ImpactSpray {
            rebound: FilmRebound {
                restitution: 0.5,
                friction: 0.0,
            },
            children: 8,
            position_radius: 0.005,
            surface_tension: 0.072,
            fragmentation_fraction: 0.8,
        },
    )?;
    let radii: Vec<_> = l
        .particles()
        .iter()
        .map(|p| (3.0 * p.mass / (4000.0 * std::f64::consts::PI)).cbrt())
        .collect();
    let mut cell = VaporCell {
        mass: 0.01,
        volume: 0.01 / 1.2,
        temperature: 300.0,
        velocity: [0.0; 3],
        specific_heat_cv: 718.0,
    };
    let initial_cell = cell.energy(1.0)?;
    let mut air_heat = 0.0;
    for _ in 0..10 {
        if finite_gas {
            for (index, radius) in radii.iter().enumerate() {
                air_heat += l.exchange_droplet_drag(index, 0.001, &mut cell, *radius, 0.47)?;
            }
        } else {
            let report = l.apply_droplet_drag(
                0.001,
                DropletGas {
                    velocity: [0.0; 3],
                    density: 1.2,
                    drag_coefficient: 0.47,
                },
                &radii,
            )?;
            air_heat += report.dissipated_heat;
        }
        l.step(0.001, None)?;
    }
    let heat = impact.substrate_heat
        + if finite_gas {
            cell.energy(1.0)? - initial_cell
        } else {
            air_heat
        };
    println!(
        "finite_gas={finite_gas},gas_velocity={:?},gas_temperature={},gas_drag_heat={air_heat}",
        cell.velocity, cell.temperature
    );
    let kinetic: f64 = l
        .particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum();
    let error = (kinetic + heat + impact.created_surface_energy - initial).abs();
    println!(
        "fragments={},surface_energy={},reservoir_energy={heat},kinetic={kinetic},energy_error={error}",
        l.particles().len(),
        impact.created_surface_energy
    );
    println!("particle,mass,x,y,z,vx,vy,vz");
    for (i, p) in l.particles().iter().enumerate() {
        println!(
            "{i},{},{},{},{},{},{},{}",
            p.mass,
            p.position[0],
            p.position[1],
            p.position[2],
            p.velocity[0],
            p.velocity[1],
            p.velocity[2]
        );
    }
    let tolerance = if finite_gas {
        128.0 * f64::EPSILON * (initial_cell + initial)
    } else {
        1e-12
    };
    if error > tolerance {
        return Err("impact/fragmentation energy ledger failed".into());
    }
    Ok(())
}
