// Exact equality intentionally checks unchanged masses and exactly zero states.
#![allow(clippy::float_cmp)]
//! A stretched isolated drop with and without surface tension.
use physics::liquid::{Config, Liquid, Material, Particle};
fn spread(liquid: &Liquid) -> [f64; 3] {
    let mass = liquid.mass();
    let center: [f64; 3] = std::array::from_fn(|axis| {
        liquid
            .particles()
            .iter()
            .map(|p| p.mass * p.position[axis])
            .sum::<f64>()
            / mass
    });
    std::array::from_fn(|axis| {
        (liquid
            .particles()
            .iter()
            .map(|p| p.mass * (p.position[axis] - center[axis]).powi(2))
            .sum::<f64>()
            / mass)
            .sqrt()
    })
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut particles = Vec::new();
    let spacing: f64 = 0.08;
    for x in -2..=2 {
        for y in -1..=1 {
            for z in -1..=1 {
                particles.push(Particle {
                    position: [
                        f64::from(x) * spacing,
                        f64::from(y) * spacing,
                        f64::from(z) * spacing,
                    ],
                    velocity: [0.0; 3],
                    mass: 1000.0 * spacing.powi(3),
                    material: 0,
                });
            }
        }
    }
    let mut drop = Liquid::new(
        particles,
        vec![Material {
            viscosity: 5.0,
            ..Material::WATER
        }],
        Config {
            smoothing_radius: 0.16,
            particle_radius: 0.03,
            gravity: [0.0; 3],
            ..Config::default()
        },
    )?;
    let mut reference = drop.clone();
    drop.set_surface_strength(0, 0.05)?;
    let initial = spread(&drop);
    let mass = drop.mass();
    for frame in 0..480 {
        drop.step(1.0 / 240.0, None)?;
        reference.step(1.0 / 240.0, None)?;
        assert_eq!(drop.mass(), mass);
        if frame % 120 == 0 {
            println!(
                "frame={frame} drop_spread={:?} reference_spread={:?}",
                spread(&drop),
                spread(&reference)
            );
        }
    }
    let final_spread = spread(&drop);
    let initial_aspect = initial[0] / initial[1];
    let final_aspect = final_spread[0] / final_spread[1];
    println!("aspect: initial={initial_aspect:.3} final={final_aspect:.3}");
    assert!(final_aspect < initial_aspect, "drop did not become rounder");
    assert!(
        final_spread.iter().sum::<f64>() < spread(&reference).iter().sum::<f64>(),
        "surface force did not contain drop"
    );
    Ok(())
}
