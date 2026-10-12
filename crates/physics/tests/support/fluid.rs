//! Shared single-particle liquid fixture (plan step B3).
use physics::liquid::{Config, Liquid, Material, Particle};

/// One 2 kg water particle at the origin moving +X at 3 m/s, default config.
pub fn fluid() -> Liquid {
    Liquid::new(
        vec![Particle {
            position: [0.0; 3],
            velocity: [3.0, 0.0, 0.0],
            mass: 2.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap()
}
