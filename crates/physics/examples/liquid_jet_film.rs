//! Single-material jet capture, rebound or energy-gated spray on a thin film.
use physics::liquid::{
    Config, EmissionPulse, FilmRebound, ImpactSpray, Liquid, Material, Particle, ParticleInput,
    PulsedEmitter, TranslatingBody,
};
use physics::surface_film::{Material as FilmMaterial, SurfaceFilm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let viscosity = std::env::args()
        .nth(1)
        .map(|s| s.parse::<f64>())
        .transpose()?
        .unwrap_or(0.001);
    let response = std::env::args().nth(2).unwrap_or_else(|| "capture".into());
    if !matches!(response.as_str(), "capture" | "bounce" | "spray") {
        return Err("expected capture, bounce or spray as second argument".into());
    }
    let source_mode = std::env::args().nth(3).unwrap_or_else(|| "fixed".into());
    if !matches!(source_mode.as_str(), "fixed" | "finite") {
        return Err("expected fixed or finite as third argument".into());
    }
    let finite_source = source_mode == "finite";
    let material = Material {
        viscosity,
        ..Material::WATER
    };
    let mut liquid = Liquid::new(
        vec![],
        vec![material],
        Config {
            smoothing_radius: 0.01,
            particle_radius: 0.0002,
            max_substeps: 4096,
            ..Config::default()
        },
    )?;
    let mut emitter = PulsedEmitter::new(
        vec![EmissionPulse {
            start: 0.0,
            duration: 0.01,
            volume: 1e-6,
            speed: 1.0,
        }],
        ParticleInput {
            particle: Particle {
                position: [0.003, 0.02, -0.003],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            },
            field: None,
            phase_fraction: None,
        },
    )?;
    emitter.direction = [0.0, -1.0, 0.0];
    emitter.nozzle_radius = 0.001;
    emitter.particle_volume = 5e-8;
    let mut source_body = TranslatingBody {
        position: emitter.template.particle.position,
        velocity: [0.; 3],
        mass: 0.01,
    };
    let initial_source_mass = source_body.mass;
    let mut source_energy = 1.;
    if finite_source {
        emitter.nozzle_radius = 0.;
    }
    let mut film = SurfaceFilm::new(
        &[
            [-0.02, 0.0, -0.02],
            [0.02, 0.0, -0.02],
            [0.02, 0.0, 0.02],
            [-0.02, 0.0, 0.02],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        FilmMaterial {
            density: material.rest_density,
            viscosity,
            wetting: 1e-5,
            surface_tension: 0.0,
        },
    )?;
    let mut emitted = 0.0;
    let mut absorbed = 0.0;
    let mut impacts = 0;
    let mut fragments = 0;
    let mut surface_energy = 0.0;
    let mut substrate_heat = 0.0;
    let mut impulse = [0.0; 3];
    let mut expected_momentum = [0.0; 3];
    let mut emitted_momentum = [0.0; 3];
    for _ in 0..80 {
        let source = if finite_source {
            let old_velocity = source_body.velocity;
            let receipt = emitter.advance_from_translating_source(
                &mut liquid,
                0.001,
                &mut source_body,
                0.008,
                &mut source_energy,
                None,
            )?;
            // Explicit drift of the source; fluid gravity is accounted below.
            for axis in 0..3 {
                source_body.position[axis] +=
                    (0.5 * old_velocity[axis] + 0.5 * source_body.velocity[axis]) * 0.001;
            }
            receipt.particles
        } else {
            emitter.advance(&mut liquid, 0.001)?
        };
        emitted += source.added.mass;
        let airborne: f64 = liquid.particles().iter().map(|p| p.mass).sum();
        for axis in 0..3 {
            emitted_momentum[axis] += source.added.momentum[axis];
            expected_momentum[axis] +=
                source.added.momentum[axis] + airborne * [0.0, -9.81, 0.0][axis] * 0.001;
        }
        let previous: Vec<_> = liquid.particles().iter().map(|p| p.position).collect();
        liquid.step(0.001, None)?;
        if response == "spray" {
            let report = liquid.impact_spray_surface_film(
                &previous,
                &film,
                ImpactSpray {
                    rebound: FilmRebound {
                        restitution: 0.5,
                        friction: 0.1,
                    },
                    children: 4,
                    position_radius: 0.001,
                    surface_tension: 0.072,
                    fragmentation_fraction: 0.8,
                },
            )?;
            impacts += report.impacts;
            fragments += report.fragments_created;
            surface_energy += report.created_surface_energy;
            substrate_heat += report.substrate_heat;
            for (sum, p) in impulse.iter_mut().zip(report.substrate_impulse) {
                *sum += p;
            }
        } else if response == "bounce" {
            let report = liquid.rebound_surface_film(
                &previous,
                &film,
                FilmRebound {
                    restitution: 0.5,
                    friction: 0.1,
                },
            )?;
            impacts += report.particles;
            for (sum, p) in impulse.iter_mut().zip(report.substrate_impulse) {
                *sum += p;
            }
        } else {
            let capture = liquid.capture_surface_film(&previous, &mut film)?;
            impacts += capture.particles;
            absorbed += capture.absorbed.mass;
            for (sum, p) in impulse.iter_mut().zip(capture.absorbed.momentum) {
                *sum += p;
            }
        }
        film.step(0.001, [0.0; 3])?;
    }
    let remaining: f64 = liquid.particles().iter().map(|p| p.mass).sum();
    let error = (emitted - remaining - film.total_mass()).abs();
    let mut actual_momentum = impulse;
    for particle in liquid.particles() {
        for axis in 0..3 {
            actual_momentum[axis] += particle.mass * particle.velocity[axis];
        }
    }
    let momentum_error = actual_momentum
        .iter()
        .zip(expected_momentum)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    println!(
        "response={response},impacts={impacts},fragments_created={fragments},momentum_ledger_error={momentum_error}"
    );
    println!("created_surface_energy={surface_energy},impact_substrate_heat={substrate_heat}");
    println!(
        "viscosity={viscosity},emitted_mass={emitted},airborne_mass={remaining},film_mass={},mass_error={error}",
        film.total_mass()
    );
    println!(
        "absorbed_mass={absorbed},substrate_impulse={impulse:?},film_thickness={:?}",
        film.thickness()
    );
    if finite_source {
        let world_momentum_error = (0..3)
            .map(|axis| {
                (source_body.mass * source_body.velocity[axis] + actual_momentum[axis]
                    - (expected_momentum[axis] - emitted_momentum[axis]))
                    .abs()
            })
            .fold(0., f64::max);
        println!("source_fluid_substrate_momentum_error={world_momentum_error}");
        let source_mass_error = (source_body.mass + emitted - initial_source_mass).abs();
        println!(
            "source_mass={},source_velocity={:?},source_energy_remaining={},source_mass_error={}",
            source_body.mass, source_body.velocity, source_energy, source_mass_error
        );
        if world_momentum_error > 1e-10
            || source_mass_error > 1e-12
            || source_body.velocity[1] <= 0.
            || source_energy >= 1.
        {
            return Err("finite source recoil or inventory failed".into());
        }
    }
    if error > 1e-12 || momentum_error > 1e-10 || impacts == 0 {
        return Err("jet/film conservation or capture failed".into());
    }
    Ok(())
}
