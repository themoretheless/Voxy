//! Frame export of a heterogeneous viscous liquid jet.
//! Positions, radii, deposited thickness and component balances come from physics.
use physics::liquid::*;
use physics::surface_film::{FilmMixture, Material as FilmMaterial, SurfaceFilm};
use std::io::{BufWriter, Write};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut out = BufWriter::new(std::io::stdout().lock());
    let mut fluid = Liquid::new(
        vec![],
        vec![Material::WATER],
        Config {
            smoothing_radius: 0.004,
            particle_radius: 0.0002,
            gravity: [0.0, -9.81, 0.0],
            max_substeps: 4096,
            ..Default::default()
        },
    )?;
    fluid.configure_transport(
        vec![],
        vec![TransportMaterial {
            specific_heat: 4000.0,
            conductivity: 0.0,
            diffusivity: 0.0,
            mixing_group: 0,
        }],
    )?;
    let mut profile = FluidMixtureProfile::DEMO;
    // Demonstration acoustic stiffness for this sparse particle jet.
    for component in &mut profile.components {
        component.sound_speed = 2.0;
        // The current FilmMixture deposition API requires common density.
        component.rest_density = 1000.0;
    }
    fluid.configure_fluid_mixture(profile, vec![], vec![])?;
    // Configurable polymer branch, independent of the solvent viscosity.
    // Solvent shear thinning and structural kinetics remain active independently.
    fluid.set_maxwell_fluid(
        0,
        Some(MaxwellFluid {
            modulus: 2.0,
            relaxation_time: 0.03,
        }),
    )?;
    fluid.configure_droplet_population(Some(vec![]))?;
    let mut source = PulsedEmitter::new(
        vec![EmissionPulse {
            start: 16.0 / 2048.0,
            duration: 80.0 / 2048.0,
            volume: 0.5e-6,
            speed: 1.4,
        }],
        ParticleInput {
            particle: Particle {
                position: [-0.009, 0.038, 0.0],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            },
            field: Some(LiquidField {
                temperature: 310.0,
                concentration: 0.0,
            }),
            phase_fraction: None,
        },
    )?;
    source.direction = [0.12, -0.993, 0.0];
    source.nozzle_radius = 0.0017;
    source.particle_volume = 5e-9;
    source.density = 1000.0;
    let mut points = Vec::new();
    for z in 0..=8 {
        for x in 0..=16 {
            points.push([-0.05 + x as f64 * 0.00625, 0.0, -0.025 + z as f64 * 0.00625]);
        }
    }
    let mut triangles = Vec::new();
    for z in 0..8 {
        for x in 0..16 {
            let a = z * 17 + x;
            triangles.extend([[a, a + 18, a + 1], [a, a + 17, a + 18]]);
        }
    }
    let surface = SurfaceFilm::new(
        &points,
        triangles.clone(),
        FilmMaterial {
            density: 1000.0,
            viscosity: 0.01,
            wetting: 1e-6,
            surface_tension: 0.025,
        },
    )?;
    let mut film = FilmMixture::new(
        surface,
        fluid.species_names().unwrap().to_vec(),
        vec![vec![1.0, 0.0, 0.0]; triangles.len()],
    )?;
    film.configure_viscosities(Some(
        profile.components.iter().map(|m| m.viscosity).collect(),
    ))?;
    let dt = 1.0 / 2048.0;
    let mut emitted = [0.0; 3];
    let mut captured = 0;
    let mut fragments = 0;
    for step in 0..=720 {
        if step % 8 == 0 {
            let frame = step / 8;
            let time = step as f64 * dt;
            let airborne = fluid.species_totals()?.unwrap();
            let deposited = film.component_masses()?;
            let error = (0..3)
                .map(|k| (emitted[k] - airborne[k] - deposited[k]).abs())
                .fold(0.0, f64::max);
            if error > 1e-12 {
                return Err("animation component mass imbalance".into());
            }
            writeln!(
                out,
                "S,{frame},{time},{},{captured},{fragments},{error}",
                film.film().total_mass()
            )?;
            writeln!(
                out,
                "R,{frame},{},{}",
                fluid.polymer_energy()?,
                fluid.polymer_relaxation_heat()
            )?;
            let radii = fluid.equivalent_sphere_radii()?;
            for ((p, r), composition) in fluid
                .particles()
                .iter()
                .zip(&radii)
                .zip(fluid.species_fractions().unwrap())
            {
                writeln!(
                    out,
                    "P,{frame},{},{},{},{r},{}",
                    p.position[0], p.position[1], p.position[2], composition[1]
                )?;
            }
            if !fluid.particles().is_empty() {
                let flags = fluid.droplet_population().unwrap();
                let supports: Vec<_> = radii
                    .iter()
                    .enumerate()
                    .map(|(i, r)| if flags[i] { 2.5 * r } else { 0.004 })
                    .collect();
                let mut min = [f64::INFINITY; 3];
                let mut max = [f64::NEG_INFINITY; 3];
                for (p, h) in fluid.particles().iter().zip(&supports) {
                    for k in 0..3 {
                        min[k] = min[k].min(p.position[k] - h - 0.001);
                        max[k] = max[k].max(p.position[k] + h + 0.001);
                    }
                }
                let matched = fluid.surface_volume_matched(
                    SurfaceConfig {
                        min,
                        max,
                        cell_size: 0.00055,
                        isovalue: 0.25,
                        material: None,
                        max_samples: 1_000_000,
                        max_checks: 2_000_000,
                        max_triangles: 60000,
                    },
                    &supports,
                    SurfaceVolumeControl {
                        relative_tolerance: 0.02,
                        max_iterations: 28,
                        max_checks: 30_000_000,
                    },
                )?;
                writeln!(
                    out,
                    "V,{frame},{},{}",
                    matched.target_volume, matched.measured_volume
                )?;
                for [a, b, c] in matched.surface.triangles {
                    writeln!(
                        out,
                        "T,{frame},{},{},{},{},{},{},{},{},{}",
                        a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]
                    )?;
                }
            }
            let fractions = film.fractions();
            let vertex_height = film.film().vertex_thickness(points.len())?;
            for (i, _height) in film.film().thickness().into_iter().enumerate() {
                let [a, b, c] = triangles[i];
                if vertex_height[a].max(vertex_height[b]).max(vertex_height[c]) < 1e-12 {
                    continue;
                }
                writeln!(
                    out,
                    "F,{frame},{},{},{},{},{},{},{},{},{},{}",
                    points[a][0],
                    points[a][2],
                    points[b][0],
                    points[b][2],
                    points[c][0],
                    points[c][2],
                    vertex_height[a],
                    vertex_height[b],
                    vertex_height[c],
                    fractions[i][1]
                )?;
            }
        }
        if step == 720 {
            break;
        }
        let composition = if step < 55 {
            [0.92, 0.08, 0.0]
        } else {
            [0.55, 0.45, 0.0]
        };
        let added = source.advance_with_species(&mut fluid, dt, &composition)?;
        for k in 0..3 {
            emitted[k] += added.added.mass * composition[k];
        }
        let previous: Vec<_> = fluid.particles().iter().map(|p| p.position).collect();
        let velocities: Vec<_> = fluid.particles().iter().map(|p| p.velocity).collect();
        fluid.step_carrier_and_droplets(dt, None)?;
        let radii = fluid.equivalent_sphere_radii()?;
        let report = fluid.depositing_impact_spheres_surface_mixture_lifecycle(
            &previous,
            &mut film,
            DepositingImpact {
                capture_speed: 0.9,
                spray: ImpactSpray {
                    rebound: FilmRebound {
                        restitution: 0.25,
                        friction: 0.15,
                    },
                    children: 3,
                    position_radius: 0.002,
                    surface_tension: 0.025,
                    fragmentation_fraction: 0.03,
                },
            },
            &radii,
            FilmImpactControl {
                dt,
                max_contacts_per_lineage: 16,
                max_events: 4096,
            },
            &DropletLifecycle {
                capture_growth_contact: true,
                // Resolve the raised liquid surface after every deposition event.
                // The floor triangles point upward; +1 selects their wet side.
                free_surface_side: Some(1.0),
                capture_film_immersion: true,
                flight: Some(DropletFlight {
                    initial_velocities: velocities,
                    acceleration: [0.0, -9.81, 0.0],
                    max_feature_checks: 1_000_000,
                }),
                ..Default::default()
            },
        )?;
        captured += report.impact.deposition.capture.particles;
        fragments += report.impact.spray.fragments_created;
        let traction = if step > 240 {
            [0.5, 0.0, 0.0]
        } else {
            [0.0; 3]
        };
        film.step_with_surface_shear(dt, [0.0, -9.81, 0.0], &vec![traction; triangles.len()], dt)?;
    }
    if captured == 0 || fragments == 0 {
        return Err("animation did not capture and fragment liquid".into());
    }
    eprintln!(
        "VISCOUS ANIMATION PASS: captured={captured}, fragments={fragments}, component masses conserved"
    );
    Ok(())
}
