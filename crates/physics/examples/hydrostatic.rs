use physics::liquid::{BoundarySample, Config, Container, Liquid, Material, Particle};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spacing = 0.08_f64;
    let density = 1000.0;
    let sound_speed = 20.0;
    let mut particles = Vec::new();
    for x in 0..4 {
        for y in 0..6 {
            for z in 0..4 {
                particles.push(Particle {
                    position: [
                        -0.12 + f64::from(x) * spacing,
                        0.04 + f64::from(y) * spacing,
                        -0.12 + f64::from(z) * spacing,
                    ],
                    velocity: [0.0; 3],
                    mass: density * spacing.powi(3),
                    material: 0,
                });
            }
        }
    }
    let mut samples =
        BoundarySample::box_grid([-0.4, -0.24, -0.4], [0.4, 0.0, 0.4], spacing, 4000)?;
    let floor_count = samples.len();
    for (min, max) in [
        ([-0.4, 0.0, -0.4], [-0.16, 1.04, 0.4]),
        ([0.16, 0.0, -0.4], [0.4, 1.04, 0.4]),
        ([-0.16, 0.0, -0.4], [0.16, 1.04, -0.16]),
        ([-0.16, 0.0, 0.16], [0.16, 1.04, 0.4]),
    ] {
        samples.extend(BoundarySample::box_grid(min, max, spacing, 4000)?);
    }
    let mut liquid = Liquid::new(
        particles,
        vec![Material {
            rest_density: density,
            sound_speed,
            viscosity: 100.0,
        }],
        Config {
            smoothing_radius: 0.18,
            particle_radius: 0.025,
            ..Config::default()
        },
    )?;
    liquid.configure_boundaries(samples)?;
    let container = Container {
        min: [-0.16, 0.0, -0.16],
        max: [0.16, 1.04, 0.16],
        restitution: 0.0,
        friction: 0.0,
    };
    let mass = liquid.mass();
    let weight = mass * 9.81;
    let mut force_sum = 0.0;
    let mut total_sum = 0.0;
    let mut viscous_sum = 0.0;
    let mut count = 0;
    for frame in 0..1200 {
        liquid.step(1.0 / 240.0, Some(container))?;
        if frame >= 960 && frame % 12 == 0 {
            let diagnostic = liquid.boundary_diagnostics()?;
            force_sum -= diagnostic.reaction_forces[..floor_count]
                .iter()
                .map(|force| force[1])
                .sum::<f64>();
            let viscous_reaction: f64 = diagnostic
                .viscous_reaction_forces
                .iter()
                .map(|force| force[1])
                .sum();
            viscous_sum -= viscous_reaction;
            total_sum -= viscous_reaction;
            total_sum -= diagnostic
                .reaction_forces
                .iter()
                .map(|force| force[1])
                .sum::<f64>();
            count += 1;
        }
    }
    let diagnostic = liquid.boundary_diagnostics()?;
    let floor_force = force_sum / f64::from(count);
    let total_support = total_sum / f64::from(count);
    let viscous_support = viscous_sum / f64::from(count);
    let relative_error = (total_support - weight).abs() / weight;
    let rms_speed = (liquid
        .particles()
        .iter()
        .map(|particle| {
            particle
                .velocity
                .iter()
                .map(|value| value * value)
                .sum::<f64>()
        })
        .sum::<f64>()
        / 96.0)
        .sqrt();
    println!(
        "hydrostatic: mass={mass:.6}, weight={weight:.6}, mean_floor_support={floor_force:.6}, mean_total_support={total_support:.6}, mean_viscous_support={viscous_support:.6}, relative_error={relative_error:.6}, rms_speed={rms_speed:.6}"
    );
    for (layer, (particle, rho)) in liquid
        .particles()
        .iter()
        .zip(diagnostic.particle_densities)
        .enumerate()
        .filter(|(index, _)| index % 16 == 0)
    {
        let pressure = sound_speed * sound_speed * (rho - density).max(0.0);
        println!(
            "probe={layer}, height={:.6}, density={rho:.6}, pressure={pressure:.6}",
            particle.position[1]
        );
    }
    if (liquid.mass() - mass).abs() > 1e-10 || !relative_error.is_finite() {
        return Err("invalid hydrostatic state".into());
    }
    // This is a measured gate, not a claim of complete hydrostatic convergence.
    if relative_error > 0.1 || rms_speed > 0.05 {
        return Err(
            "hydrostatic acceptance gate failed: support error >10% or RMS speed >0.05".into(),
        );
    }
    Ok(())
}
