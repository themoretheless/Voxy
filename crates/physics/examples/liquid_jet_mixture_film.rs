//! Heterogeneous jets with composed deposition, optional spray and film transport.
//! Illustrative impact coefficients; no calibrated wetting or biological model.
use physics::liquid::{
    Config, DepositingImpact, EmissionPulse, FilmRebound, ImpactSpray, Liquid, LiquidField,
    Material, Particle, ParticleInput, PulsedEmitter, SpeciesProperties, TransportMaterial,
    ViscosityBlend,
};
use physics::surface_film::{FilmMixture, Material as FilmMaterial, SurfaceFilm};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hybrid = match std::env::args().nth(1).as_deref() {
        None | Some("capture") => false,
        Some("hybrid") => true,
        _ => return Err("expected capture or hybrid".into()),
    };
    let names = vec!["aqueous".into(), "viscous_analog".into()];
    let mut liquid = Liquid::new(
        vec![],
        vec![Material::WATER],
        Config {
            smoothing_radius: 0.01,
            particle_radius: 0.0002,
            max_substeps: 4096,
            ..Config::default()
        },
    )?;
    liquid.configure_transport(
        vec![],
        vec![TransportMaterial {
            conductivity: 0.0,
            diffusivity: 0.0,
            ..TransportMaterial::default()
        }],
    )?;
    liquid.configure_species(names.clone(), vec![])?;
    liquid.configure_species_properties(Some(SpeciesProperties {
        components: vec![
            Material::WATER,
            Material {
                viscosity: 0.01,
                ..Material::WATER
            },
        ],
        viscosity: ViscosityBlend::Logarithmic,
    }))?;
    let mut emitter = PulsedEmitter::new(
        vec![EmissionPulse {
            start: 0.0,
            duration: if hybrid { 0.005 } else { 0.01 },
            volume: if hybrid { 0.5e-6 } else { 1e-6 },
            speed: if hybrid { 0.1 } else { 1.0 },
        }],
        ParticleInput {
            particle: Particle {
                position: [if hybrid { -0.01 } else { 0.003 }, 0.02, -0.003],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            },
            field: Some(LiquidField {
                temperature: 300.0,
                concentration: 0.0,
            }),
            phase_fraction: None,
        },
    )?;
    emitter.direction = [0.0, -1.0, 0.0];
    emitter.nozzle_radius = 0.001;
    emitter.particle_volume = 5e-8;
    let mut fast_emitter = if hybrid {
        let mut template = emitter.template;
        template.particle.position[0] = 0.01;
        let mut fast = PulsedEmitter::new(
            vec![EmissionPulse {
                start: 0.0,
                duration: 0.005,
                volume: 0.5e-6,
                speed: 2.0,
            }],
            template,
        )?;
        fast.direction = emitter.direction;
        fast.nozzle_radius = emitter.nozzle_radius;
        fast.particle_volume = emitter.particle_volume;
        Some(fast)
    } else {
        None
    };
    let surface = SurfaceFilm::new(
        &[
            [-0.02, 0.0, -0.02],
            [0.02, 0.0, -0.02],
            [0.02, 0.0, 0.02],
            [-0.02, 0.0, 0.02],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        FilmMaterial {
            density: 1000.0,
            viscosity: 0.001,
            wetting: 1e-5,
            surface_tension: 0.0,
        },
    )?;
    let mut film = FilmMixture::new(surface, names, vec![vec![1.0, 0.0]; 2])?;
    film.configure_viscosities(Some(vec![0.001, 0.01]))?;
    let mut emitted = [0.0; 2];
    let mut captured = [0.0; 2];
    let mut impacts = 0;
    let mut deposited_particles = 0;
    let mut fragments = 0;
    for i in 0..80 {
        let composition = if hybrid || i < 5 {
            [0.9, 0.1]
        } else {
            [0.2, 0.8]
        };
        let source = emitter.advance_with_species(&mut liquid, 0.001, &composition)?;
        for k in 0..2 {
            emitted[k] += source.added.mass * composition[k];
        }
        if let Some(fast) = &mut fast_emitter {
            let composition = [0.2, 0.8];
            let source = fast.advance_with_species(&mut liquid, 0.001, &composition)?;
            for k in 0..2 {
                emitted[k] += source.added.mass * composition[k];
            }
        }
        let previous: Vec<_> = liquid.particles().iter().map(|p| p.position).collect();
        liquid.step(0.001, None)?;
        let report = if hybrid {
            let report = liquid.depositing_impact_surface_mixture(
                &previous,
                &mut film,
                DepositingImpact {
                    capture_speed: 0.8,
                    spray: ImpactSpray {
                        rebound: FilmRebound {
                            restitution: 0.5,
                            friction: 0.1,
                        },
                        children: 4,
                        position_radius: 0.001,
                        surface_tension: 0.072,
                        fragmentation_fraction: 0.8,
                    },
                },
            )?;
            impacts += report.spray.impacts;
            fragments += report.spray.fragments_created;
            report.deposition
        } else {
            liquid.capture_surface_mixture(&previous, &mut film)?
        };
        impacts += report.capture.particles;
        deposited_particles += report.capture.particles;
        for k in 0..2 {
            captured[k] += report.component_masses[k];
        }
        film.step(0.001, [0.0; 3], 0.001)?;
    }
    let airborne = liquid.species_totals()?.ok_or("missing airborne species")?;
    let surface = film.component_masses()?;
    let error = (0..2)
        .map(|k| (emitted[k] - airborne[k] - surface[k]).abs())
        .fold(0.0, f64::max);
    let capture_error = (0..2)
        .map(|k| (captured[k] - surface[k]).abs())
        .fold(0.0, f64::max);
    println!(
        "hybrid={hybrid},impacts={impacts},deposited_particles={deposited_particles},fragments_created={fragments},emitted_components={emitted:?},airborne_components={airborne:?},film_components={surface:?}"
    );
    println!("component_mass_error={error},capture_ledger_error={capture_error}");
    println!(
        "film_height={:?},film_composition={:?},film_viscosity={:?}",
        film.film().thickness(),
        film.fractions(),
        film.effective_viscosities()
    );
    if impacts == 0
        || error > 1e-12
        || capture_error > 1e-12
        || (hybrid && (deposited_particles == 0 || fragments == 0))
    {
        return Err("jet mixture conservation failed".into());
    }
    Ok(())
}
