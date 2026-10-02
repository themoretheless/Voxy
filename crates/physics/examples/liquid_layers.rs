//! Coarse two-phase gravity experiment, not calibrated water/oil properties.
use physics::liquid::{BoundarySample, Config, Container, Liquid, Material, Particle};
fn metrics(liquid: &Liquid) -> ([f64; 2], [f64; 2], f64) {
    let mut mass = [0.0; 2];
    let mut height = [0.0; 2];
    for p in liquid.particles() {
        mass[p.material] += p.mass;
        height[p.material] += p.mass * p.position[1];
    }
    for a in 0..2 {
        height[a] /= mass[a];
    }
    let midpoint = 0.5 * (height[0] + height[1]);
    let misplaced = liquid
        .particles()
        .iter()
        .filter(|p| {
            (p.material == 0 && p.position[1] > midpoint)
                || (p.material == 1 && p.position[1] < midpoint)
        })
        .map(|p| p.mass)
        .sum::<f64>()
        / (mass[0] + mass[1]);
    (mass, height, misplaced)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Settings {
        seconds,
        layout,
        frequency,
        resolution,
        wave,
        support,
        formulation,
        viscosity,
        penalty_scale,
        wall_model,
    } = settings()?;
    let stable = layout != Layout::Inverted;
    let hydrostatic = layout == Layout::Hydrostatic;
    let spacing = 0.32 / f64::from(resolution);
    let scale = spacing / 0.08;
    let penalty = penalty_scale * 0.00001 * 0.18 / support;
    let materials = vec![
        Material {
            viscosity: viscosity[0],
            ..Material::WATER
        },
        Material {
            viscosity: viscosity[1],
            ..Material::OIL
        },
    ];
    let container = Container {
        min: [-0.16, 0.0, -0.16],
        max: [0.16, 1.04, 0.16],
        restitution: 0.0,
        friction: 0.0,
    };
    let particles = starting_particles(resolution, spacing, stable, wave, &materials);
    let mut liquid = Liquid::new(
        particles,
        materials.clone(),
        Config {
            smoothing_radius: support,
            particle_radius: 0.025 * scale,
            ..Config::default()
        },
    )?;
    liquid.set_formulation(formulation);
    configure_walls(&mut liquid, spacing, wall_model, container)?;
    liquid.set_interface_penalty(0, 1, penalty)?;
    let (initial_mass, initial_height, initial_misplaced) = metrics(&liquid);
    println!(
        "layers: stable={stable}, particles={}, viscosity={viscosity:?}, interface_penalty={penalty}, frequency={frequency}, resolution={resolution}, spacing={spacing}, wave={wave}, support={support}, formulation={formulation:?}, wall_model={wall_model:?}",
        liquid.particles().len()
    );
    println!("time,heavy_height,light_height,misplaced_fraction");
    println!(
        "0,{:.6},{:.6},{initial_misplaced:.6}",
        initial_height[0], initial_height[1]
    );
    log_state(&liquid, 0.0, spacing, &materials)?;
    let mut time = 0.0;
    let mut next_log = 1.0;
    while time < seconds {
        let step = (seconds - time).min(1.0 / frequency);
        liquid.step(step, Some(container))?;
        time += step;
        if time + 1e-10 >= next_log {
            let (_, height, misplaced) = metrics(&liquid);
            log_state(&liquid, time, spacing, &materials)?;
            println!("{time:.6},{:.6},{:.6},{misplaced:.6}", height[0], height[1]);
            next_log += 1.0;
        }
    }
    let hydro = report_pressure(&liquid, &materials, resolution, spacing)?;
    if hydrostatic {
        hydro.verify()?;
    }
    let (mass, height, misplaced) = metrics(&liquid);
    println!(
        "final: heavy_mass={:.6}, light_mass={:.6}, separation={:.6}, misplaced={misplaced:.6}",
        mass[0],
        mass[1],
        height[1] - height[0]
    );
    if mass
        .iter()
        .zip(initial_mass)
        .any(|(a, b)| (a - b).abs() > 1e-10)
    {
        return Err("phase mass changed".into());
    }
    if height[0] >= height[1] {
        return Err("density-order gate failed: heavy phase remains above light phase".into());
    }
    if seconds >= 20.0 && misplaced > 0.1 {
        return Err("phase-separation gate failed: misplaced mass exceeds 10%".into());
    }
    Ok(())
}

fn log_state(
    liquid: &Liquid,
    time: f64,
    spacing: f64,
    materials: &[Material],
) -> Result<(), physics::liquid::Error> {
    let densities = liquid.diagnostics()?.densities;
    let mut compression = [0.0_f64; 2];
    let mut speed = [0.0_f64; 2];
    let mut kinetic = 0.0;
    let mut gravitational = 0.0;
    let mut compression_energy = 0.0;
    for (p, density) in liquid.particles().iter().zip(densities) {
        let material = materials[p.material];
        let reference = material.rest_density;
        let x = (density / reference - 1.0).max(0.0);
        compression_energy += p.mass * material.sound_speed.powi(2) * (x.ln_1p() - x / (1.0 + x));
        kinetic += 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>();
        gravitational += p.mass * 9.81 * p.position[1];
        compression[p.material] = compression[p.material].max(density / reference - 1.0);
        speed[p.material] =
            speed[p.material].max(p.velocity.iter().map(|v| v * v).sum::<f64>().sqrt());
    }
    let mut min_spacing = f64::INFINITY;
    for (i, first) in liquid.particles().iter().enumerate() {
        for second in &liquid.particles()[i + 1..] {
            let distance = first
                .position
                .iter()
                .zip(second.position)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            min_spacing = min_spacing.min(distance / spacing);
        }
    }
    eprintln!(
        "state: time={time:.6}, max_compression={compression:?}, max_speed={speed:?}, min_spacing_ratio={min_spacing}"
    );
    let interface = liquid.interface_energy()?;
    let fluid_mechanical_reference = kinetic + gravitational + compression_energy + interface;
    // Static extrapolated ghost pressure has no potential in this reference.
    // MassDensity also uses a different pressure gradient (spiky).
    // This reference is not a proven conserved Hamiltonian for either case.
    eprintln!(
        "energy: time={time:.6}, kinetic={kinetic}, gravitational={gravitational}, compression={compression_energy}, interface={interface}, fluid_mechanical_reference={fluid_mechanical_reference}"
    );
    Ok(())
}

// The reference represents every particle as a horizontal slab of fixed initial thickness.
// It is an empirical hydrostatic comparison, not an exact deformed particle-volume model.
fn column_pressure(particles: &[Particle], spacing: f64, height: f64) -> f64 {
    9.81 / 0.32_f64.powi(2)
        * particles
            .iter()
            .map(|p| p.mass * (0.5 + (p.position[1] - height) / spacing).clamp(0.0, 1.0))
            .sum::<f64>()
}
#[derive(Debug)]
struct HydrostaticReport {
    profile_error: f64,
    support_error: f64,
    rms_speed: f64,
}
impl HydrostaticReport {
    fn verify(&self) -> Result<(), &'static str> {
        if !self.profile_error.is_finite() || self.profile_error > 0.1 {
            return Err("hydrostatic pressure-profile gate failed: relative error exceeds 10%");
        }
        if !self.support_error.is_finite() || self.support_error > 0.1 {
            return Err("hydrostatic wall-support gate failed: relative error exceeds 10%");
        }
        if !self.rms_speed.is_finite() || self.rms_speed > 0.05 {
            return Err("hydrostatic stationarity gate failed: RMS speed exceeds 0.05 m/s");
        }
        Ok(())
    }
}
fn report_pressure(
    liquid: &Liquid,
    materials: &[Material],
    resolution: u32,
    spacing: f64,
) -> Result<HydrostaticReport, physics::liquid::Error> {
    let densities = liquid.diagnostics()?.densities;
    let mut bins = vec![[0.0_f64; 4]; usize::try_from(2 * resolution).unwrap()];
    for (p, density) in liquid.particles().iter().zip(densities) {
        let material = materials[p.material];
        let pressure = material.sound_speed.powi(2) * (density - material.rest_density).max(0.0);
        let volume = p.mass / density;
        // Saturate the top bin: a deformed column can rise above the initial height.
        let index = (0..bins.len() - 1)
            .filter(|&i| p.position[1] >= (f64::from(u32::try_from(i).unwrap()) + 1.0) * spacing)
            .count();
        bins[index][0] += volume;
        bins[index][1] += volume * p.position[1];
        bins[index][2] += volume * pressure;
        bins[index][3] += volume * column_pressure(liquid.particles(), spacing, p.position[1]);
    }
    let mut squared_error = 0.0;
    let mut squared_reference = 0.0;
    for (index, bin) in bins.iter().enumerate().filter(|(_, b)| b[0] > 0.0) {
        let [volume, height, pressure, reference] = *bin;
        let (height, pressure, reference) =
            (height / volume, pressure / volume, reference / volume);
        squared_error += volume * (pressure - reference).powi(2);
        squared_reference += volume * reference.powi(2);
        eprintln!(
            "profile: bin={index}, height={height:.9}, pressure={pressure:.9}, column_reference={reference:.9}, volume={volume:.9}"
        );
    }
    let support: f64 = if liquid.reflecting_box().is_some() {
        liquid
            .reflecting_diagnostics()?
            .reaction_forces
            .iter()
            .map(|force| -force[1])
            .sum()
    } else {
        let boundary = liquid.boundary_diagnostics()?;
        boundary
            .reaction_forces
            .iter()
            .zip(boundary.viscous_reaction_forces)
            .map(|(pressure, viscosity)| -pressure[1] - viscosity[1])
            .sum()
    };
    let weight = 9.81 * liquid.mass();
    let report = HydrostaticReport {
        profile_error: (squared_error / squared_reference).sqrt(),
        support_error: (support - weight).abs() / weight,
        rms_speed: (liquid
            .particles()
            .iter()
            .map(|p| p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>()
            / liquid.mass())
        .sqrt(),
    };
    eprintln!(
        "hydrostatic: profile_relative_error={}, sph_support={support}, weight={weight}, support_relative_error={}, rms_speed={}",
        report.profile_error, report.support_error, report.rms_speed
    );
    Ok(report)
}

fn configure_walls(
    liquid: &mut Liquid,
    spacing: f64,
    model: WallModel,
    container: Container,
) -> Result<(), physics::liquid::Error> {
    if model == WallModel::Reflect {
        liquid.set_reflecting_box(Some(physics::liquid::ReflectingBox {
            min: container.min,
            max: container.max,
        }))
    } else {
        liquid.configure_boundaries(tank_samples(spacing)?)?;
        liquid.set_static_pressure_extrapolation(model == WallModel::Extrapolate)
    }
}

fn tank_samples(spacing: f64) -> Result<Vec<BoundarySample>, physics::liquid::Error> {
    let mut samples =
        BoundarySample::box_grid([-0.4, -0.24, -0.4], [0.4, 0.0, 0.4], spacing, 4000)?;
    for (min, max) in [
        ([-0.4, 0.0, -0.4], [-0.16, 1.04, 0.4]),
        ([0.16, 0.0, -0.4], [0.4, 1.04, 0.4]),
        ([-0.16, 0.0, -0.4], [0.16, 1.04, -0.16]),
        ([-0.16, 0.0, 0.16], [0.16, 1.04, 0.4]),
    ] {
        samples.extend(BoundarySample::box_grid(min, max, spacing, 4000)?);
    }
    Ok(samples)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Layout {
    Inverted,
    Stable,
    Hydrostatic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WallModel {
    Legacy,
    Extrapolate,
    Reflect,
}

struct Settings {
    seconds: f64,
    layout: Layout,
    frequency: f64,
    resolution: u32,
    wave: bool,
    support: f64,
    formulation: physics::liquid::Formulation,
    viscosity: [f64; 2],
    penalty_scale: f64,
    wall_model: WallModel,
}
fn settings() -> Result<Settings, Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let seconds: f64 = args.first().map_or(Ok(20.0), |s| s.parse())?;
    let layout = match args.get(1).map(String::as_str) {
        None | Some("inverted") => Layout::Inverted,
        Some("stable") => Layout::Stable,
        Some("hydrostatic") => Layout::Hydrostatic,
        Some(_) => return Err("layout must be stable, inverted or hydrostatic".into()),
    };
    let frequency: f64 = args.get(2).map_or(Ok(240.0), |s| s.parse())?;
    if !frequency.is_finite() || !(60.0..=2000.0).contains(&frequency) {
        return Err("frequency must be in [60,2000]".into());
    }
    if !seconds.is_finite() || seconds <= 0.0 || seconds > 120.0 {
        return Err("seconds must be in (0,120]".into());
    }
    let resolution: u32 = args.get(3).map_or(Ok(4), |s| s.parse())?;
    if !(4..=8).contains(&resolution) {
        return Err("resolution must be in [4,8]".into());
    }
    let wave = match args.get(4).map(String::as_str) {
        None | Some("legacy") => false,
        Some("wave") => true,
        Some(_) => return Err("seed must be legacy or wave".into()),
    };
    let support: f64 = args
        .get(5)
        .map_or(Ok(0.18 * 4.0 / f64::from(resolution)), |s| s.parse())?;
    if !support.is_finite() || !(0.06..=0.24).contains(&support) {
        return Err("support must be in [0.06,0.24]".into());
    }
    let formulation = match args.get(6).map(String::as_str) {
        None | Some("mass") => physics::liquid::Formulation::MassDensity,
        Some("volume") => physics::liquid::Formulation::RestVolume,
        Some("wendland") => physics::liquid::Formulation::RestVolumeWendland,
        Some(_) => return Err("formulation must be mass, volume or wendland".into()),
    };
    let heavy_viscosity: f64 = args.get(7).map_or(Ok(100.0), |s| s.parse())?;
    let light_viscosity: f64 = args.get(10).map_or(Ok(heavy_viscosity), |s| s.parse())?;
    let viscosity = [heavy_viscosity, light_viscosity];
    let penalty_scale: f64 = args.get(8).map_or(Ok(1.0), |s| s.parse())?;
    if viscosity.iter().any(|v| !v.is_finite() || *v < 0.0)
        || !penalty_scale.is_finite()
        || penalty_scale < 0.0
    {
        return Err("viscosity and penalty scale must be finite and nonnegative".into());
    }
    let wall_model = match args.get(9).map(String::as_str) {
        None | Some("legacy") => WallModel::Legacy,
        Some("extrapolate") => WallModel::Extrapolate,
        Some("reflect") => WallModel::Reflect,
        Some(_) => return Err("wall pressure must be legacy, extrapolate or reflect".into()),
    };
    Ok(Settings {
        seconds,
        layout,
        frequency,
        resolution,
        wave,
        support,
        formulation,
        viscosity,
        penalty_scale,
        wall_model,
    })
}

fn starting_particles(
    resolution: u32,
    spacing: f64,
    stable: bool,
    wave: bool,
    materials: &[Material],
) -> Vec<Particle> {
    let mut particles = Vec::new();
    for x in 0..resolution {
        for y in 0..2 * resolution {
            for z in 0..resolution {
                let material = usize::from(if stable {
                    y >= resolution
                } else {
                    y < resolution
                });
                let mut position = [
                    -0.16 + (f64::from(x) + 0.5) * spacing,
                    (f64::from(y) + 0.5) * spacing,
                    -0.16 + (f64::from(z) + 0.5) * spacing,
                ];
                let phase = (3.0 * (position[0] + 0.12)
                    + 7.0 * (position[1] - 0.04)
                    + (position[2] + 0.12))
                    / 0.08;
                if wave {
                    let frequency = 2.0 * std::f64::consts::PI / 0.32;
                    position[1] +=
                        0.004 * (frequency * position[0]).cos() * (frequency * position[2]).cos();
                } else {
                    position[0] += 0.004 * phase.sin();
                }
                particles.push(Particle {
                    position,
                    velocity: [0.0; 3],
                    mass: materials[material].rest_density * spacing.powi(3),
                    material,
                });
            }
        }
    }
    particles
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refinement_preserves_phase_volumes_and_initial_order() {
        let materials = [Material::WATER, Material::OIL];
        for resolution in [4, 6, 8] {
            let spacing = 0.32 / f64::from(resolution);
            for stable in [false, true] {
                let particles = starting_particles(resolution, spacing, stable, true, &materials);
                let mut volumes = [0.0; 2];
                let mut height = [0.0; 2];
                for p in particles {
                    let volume = p.mass / materials[p.material].rest_density;
                    volumes[p.material] += volume;
                    height[p.material] += volume * p.position[1];
                    let radius = 0.025 * spacing / 0.08;
                    assert!(p.position[0].abs() + radius < 0.16);
                    assert!(p.position[2].abs() + radius < 0.16);
                    assert!(p.position[1] >= radius);
                }
                for a in 0..2 {
                    assert!((volumes[a] - 0.032768).abs() < 1e-12);
                    height[a] /= volumes[a];
                }
                let expected = if stable { [0.16, 0.48] } else { [0.48, 0.16] };
                for (actual, expected) in height.into_iter().zip(expected) {
                    assert!((actual - expected).abs() < 1e-12);
                }
            }
        }
    }
    #[test]
    fn column_pressure_reference_matches_piecewise_hydrostatics() {
        let materials = [Material::WATER, Material::OIL];
        for resolution in [4, 6, 8] {
            let spacing = 0.32 / f64::from(resolution);
            let particles = starting_particles(resolution, spacing, true, false, &materials);
            // The legacy lateral seed does not affect vertical column integration.
            for height in [0.0, 0.13, 0.32, 0.41, 0.64, 0.8] {
                let expected = 9.81
                    * (1000.0 * (0.32_f64 - height).max(0.0)
                        + 800.0 * (0.64 - height.max(0.32)).max(0.0));
                let actual = column_pressure(&particles, spacing, height);
                assert!(
                    (actual - expected).abs() < 1e-9,
                    "resolution {resolution}, height {height}: {actual} != {expected}"
                );
            }
        }
    }
    #[test]
    fn refinement_preserves_tank_quadrature_volume() {
        for resolution in [4, 6, 8] {
            let samples = tank_samples(0.32 / f64::from(resolution)).unwrap();
            let volume: f64 = samples.iter().map(|s| s.volume).sum();
            assert!((volume - 0.712704).abs() < 1e-10);
        }
    }
}
