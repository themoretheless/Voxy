//! Hydrostatic body/recoil demonstration in a prescribed water layer.
//! The free surface stays at y=0; this example does not solve its deformation.
use physics::liquid::{
    BuoyancyConfig, Config, FloatingBody, FluidLayer, Liquid, Material, Particle,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for mass in [1.0, 10.0] {
        let mut liquid = Liquid::new(
            vec![Particle {
                position: [0.0, -0.5, 0.0],
                velocity: [0.0; 3],
                mass: 1000.0,
                material: 0,
            }],
            vec![Material::WATER],
            Config::default(),
        )?;
        let mut body = FloatingBody {
            position: [0.0, -0.3, 0.0],
            velocity: [0.0; 3],
            mass,
            radius: 0.1,
        };
        let layer = FluidLayer {
            bottom: -10.0,
            top: 0.0,
            material: 0,
        };
        for frame in 0..240 {
            let report = liquid.couple_floating_body(
                &mut body,
                &[layer],
                1.0 / 120.0,
                BuoyancyConfig::default(),
            )?;
            let total = body.mass * body.velocity[1]
                + liquid.particles()[0].mass * liquid.particles()[0].velocity[1];
            let weight_impulse = -mass * 9.81 * f64::from(frame + 1) / 120.0;
            assert!(
                (total - weight_impulse).abs() < 1e-9,
                "momentum exchange failed"
            );
            if frame % 60 == 0 {
                println!(
                    "mass={mass} frame={frame} y={:.3} vy={:.3} displaced_mass={:.3}",
                    body.position[1], body.velocity[1], report.displaced_mass
                );
            }
        }
        if mass < 4.0 {
            assert!(body.position[1] > -0.3, "light sphere did not rise");
        } else {
            assert!(body.position[1] < -0.3, "heavy sphere did not sink");
        }
    }
    Ok(())
}
