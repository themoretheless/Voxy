//! Instantaneous manufactured steady power-law channel profile; not an evolved flow.
use physics::liquid::{
    Config, Formulation, Liquid, Material, Particle, ReflectingBox, ShearThinning,
};
fn velocity(y: f64) -> f64 {
    let index = 0.75;
    let power = 1.0 + 1.0 / index;
    index / (index + 1.0)
        * (1300.0_f64 * 0.1 / 10.0).powf(1.0 / index)
        * (0.5_f64.powf(power) - (y - 0.5).abs().powf(power))
}
fn fluid(particles: Vec<Particle>, support: f64) -> Result<Liquid, physics::liquid::Error> {
    let mut f = Liquid::new(
        particles,
        vec![Material::CONDENSED_MILK_DEMO],
        Config {
            smoothing_radius: support,
            gravity: [0.0; 3],
            max_particles: 50_000,
            max_pairs: 20_000_000,
            max_neighbor_checks: 300_000_000,
            ..Config::default()
        },
    )?;
    f.set_formulation(Formulation::RestVolumeWendland);
    f.set_reflecting_box(Some(ReflectingBox {
        min: [0.0; 3],
        max: [1.0; 3],
    }))?;
    f.set_reflecting_no_slip(true)?;
    f.configure_shear_thinning(vec![Some(ShearThinning::CONDENSED_MILK_DEMO)])?;
    Ok(f)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "resolution,support,bulk_count,wall_count,centre_count,bulk_relative_rms,wall_relative_rms,centre_relative_rms,total_relative_rms"
    );
    for (n, ratio) in [(12_u32, 2.0), (18, 2.5), (26, 3.0), (34, 3.5)] {
        measure(n, ratio)?;
    }
    Ok(())
}
fn measure(n: u32, ratio: f64) -> Result<[f64; 4], Box<dyn std::error::Error>> {
    let dx = 1.0 / f64::from(n);
    let support = ratio * dx;
    let mut ps = Vec::new();
    for x in 0..n {
        for y in 0..n {
            for z in 0..n {
                let position = [x, y, z].map(|a| (f64::from(a) + 0.5) * dx);
                ps.push(Particle {
                    position,
                    velocity: [velocity(position[1]), 0.0, 0.0],
                    mass: 1300.0 * dx.powi(3),
                    material: 0,
                });
            }
        }
    }
    let moving = fluid(ps.clone(), support)?;
    let forces = moving.diagnostics()?.accelerations;
    for p in &mut ps {
        p.velocity = [0.0; 3];
    }
    let still = fluid(ps, support)?.diagnostics()?.accelerations;
    let mut errors = [0.0; 3];
    let mut counts = [0_u32; 3];
    for ((p, a), b) in moving.particles().iter().zip(forces).zip(still) {
        if [0, 2].into_iter().any(|axis| {
            p.position[axis] <= 2.0 * support || p.position[axis] >= 1.0 - 2.0 * support
        }) {
            continue;
        }
        let region = if p.position[1] < support || p.position[1] > 1.0 - support {
            1
        } else if (p.position[1] - 0.5).abs() < support {
            2
        } else {
            0
        };
        errors[region] += ((a[0] - b[0]) / 0.1 + 1.0).powi(2);
        counts[region] += 1;
    }
    if counts.contains(&0) {
        return Err("empty power-law measurement region".into());
    }
    println!(
        "{n},{support},{},{},{},{},{},{},{}",
        counts[0],
        counts[1],
        counts[2],
        (errors[0] / f64::from(counts[0])).sqrt(),
        (errors[1] / f64::from(counts[1])).sqrt(),
        (errors[2] / f64::from(counts[2])).sqrt(),
        (errors.iter().sum::<f64>() / f64::from(counts.iter().sum::<u32>())).sqrt()
    );
    Ok([
        (errors[0] / f64::from(counts[0])).sqrt(),
        (errors[1] / f64::from(counts[1])).sqrt(),
        (errors[2] / f64::from(counts[2])).sqrt(),
        (errors.iter().sum::<f64>() / f64::from(counts.iter().sum::<u32>())).sqrt(),
    ])
}
#[test]
fn nonlinear_viscous_operator_matches_power_law_channel_control() {
    let errors = measure(12, 2.0).unwrap();
    assert!(errors[0] < 0.1);
    assert!(errors[3] < 0.1);
}
