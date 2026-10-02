//! Instantaneous Poiseuille operator consistency (not a time-evolved channel).
use physics::liquid::{Config, Formulation, Liquid, Material, Particle, ReflectingBox};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sine = std::env::args().nth(2).as_deref() == Some("sine");
    let cases = if std::env::args().nth(1).as_deref() == Some("dense") {
        vec![
            (8_u32, 2.0),
            (12, 2.5),
            (18, 3.0),
            (26, 3.5),
            (34, 4.0),
            (42, 4.5),
        ]
    } else {
        vec![(6, 2.5), (10, 2.5), (14, 2.5), (18, 2.5)]
    };
    println!(
        "resolution,support,support_over_spacing,interior_count,wall_count,interior_relative_rms,wall_relative_rms,total_relative_rms"
    );
    for (n, ratio) in cases {
        let spacing = 1.0 / f64::from(n);
        let support = ratio * spacing;
        let mut particles = Vec::new();
        for x in 0..n {
            for y in 0..n {
                for z in 0..n {
                    let position = [x, y, z].map(|i| (f64::from(i) + 0.5) * spacing);
                    particles.push(Particle {
                        position,
                        velocity: [
                            if sine {
                                (std::f64::consts::PI * position[1]).sin()
                            } else {
                                position[1] * (1.0 - position[1])
                            },
                            0.0,
                            0.0,
                        ],
                        mass: 1000.0 * spacing.powi(3),
                        material: 0,
                    });
                }
            }
        }
        let config = Config {
            smoothing_radius: support,
            gravity: [0.0; 3],
            max_particles: 100_000,
            max_pairs: 30_000_000,
            max_neighbor_checks: 500_000_000,
            ..Config::default()
        };
        let material = Material {
            rest_density: 1000.0,
            sound_speed: 1.0,
            viscosity: 1000.0,
        };
        let liquid = channel(particles.clone(), material, config)?;
        let moving = liquid.diagnostics()?.accelerations;
        for p in &mut particles {
            p.velocity = [0.0; 3];
        }
        let still = channel(particles, material, config)?;
        let resting = still.diagnostics()?.accelerations;
        let mut sums = [0.0; 2];
        let mut references = [0.0; 2];
        let mut counts = [0_u32; 2];
        for ((p, a), b) in liquid.particles().iter().zip(moving).zip(resting) {
            if [0, 2]
                .into_iter()
                .any(|axis| p.position[axis] <= support || p.position[axis] >= 1.0 - support)
            {
                continue;
            }
            let region = usize::from(p.position[1] < support || p.position[1] > 1.0 - support);
            let target = if sine {
                std::f64::consts::PI.powi(2) * (std::f64::consts::PI * p.position[1]).sin()
            } else {
                2.0
            };
            sums[region] += (a[0] - b[0] + target).powi(2);
            references[region] += target.powi(2);
            counts[region] += 1;
        }
        if counts.contains(&0) {
            return Err("empty channel measurement region".into());
        }
        println!(
            "{n},{support},{ratio},{},{},{},{},{}",
            counts[0],
            counts[1],
            (sums[0] / references[0]).sqrt(),
            (sums[1] / references[1]).sqrt(),
            ((sums[0] + sums[1]) / (references[0] + references[1])).sqrt()
        );
    }
    Ok(())
}

fn channel(
    particles: Vec<Particle>,
    material: Material,
    config: Config,
) -> Result<Liquid, physics::liquid::Error> {
    let mut liquid = Liquid::new(particles, vec![material], config)?;
    liquid.set_formulation(Formulation::RestVolumeWendland);
    liquid.set_reflecting_box(Some(ReflectingBox {
        min: [0.0; 3],
        max: [1.0; 3],
    }))?;
    liquid.set_reflecting_no_slip(true)?;
    Ok(liquid)
}
