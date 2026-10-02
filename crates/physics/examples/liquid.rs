// Exact equality intentionally checks unchanged masses and exactly zero states.
#![allow(clippy::float_cmp)]
//! CPU dam-break smoke test: free-moving SPH particles in a closed tank.
use physics::liquid::{Config, Container, Liquid, Material, Particle};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut particles = Vec::new();
    let spacing: f64 = 0.08;
    for x in 0..5 {
        for y in 0..6 {
            for z in 0..4 {
                particles.push(Particle {
                    position: [
                        -0.7 + f64::from(x) * spacing,
                        0.1 + f64::from(y) * spacing,
                        -0.12 + f64::from(z) * spacing,
                    ],
                    velocity: [0.0; 3],
                    mass: 1000.0 * spacing.powi(3),
                    material: 0,
                });
            }
        }
    }
    let mut liquid = Liquid::new(
        particles,
        vec![Material::WATER],
        Config {
            smoothing_radius: 0.16,
            particle_radius: 0.03,
            ..Config::default()
        },
    )?;
    let container = Container {
        min: [-1.0, 0.0, -0.3],
        max: [1.0, 1.0, 0.3],
        restitution: 0.0,
        friction: 0.02,
    };
    let mass = liquid.mass();
    for frame in 0..240 {
        let stats = liquid.step(1.0 / 120.0, Some(container))?;
        assert_eq!(liquid.mass(), mass);
        if frame % 60 == 0 {
            println!(
                "frame={frame} particles={} mass={mass:.3} substeps={} density_ratio={:.3}",
                liquid.particles().len(),
                stats.substeps,
                stats.max_density_ratio
            );
        }
    }
    let min = liquid
        .particles()
        .iter()
        .map(|p| p.position[0])
        .fold(f64::INFINITY, f64::min);
    let max = liquid
        .particles()
        .iter()
        .map(|p| p.position[0])
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(max - min > 0.5, "water did not spread");
    println!("passed: horizontal spread={:.3}", max - min);
    Ok(())
}
