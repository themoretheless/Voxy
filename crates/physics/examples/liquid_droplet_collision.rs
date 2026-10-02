//! Swept head-on coalescence with an explicit unresolved-energy ledger.
use physics::liquid::{Config, DropletCoalescenceControl, Liquid, Material, Particle};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let previous = [[-0.025, 0.0, 0.0], [0.025, 0.0, 0.0]];
    let mut liquid = Liquid::new(
        vec![
            Particle {
                position: [0.025, 0.0, 0.0],
                velocity: [1.0, 0.0, 0.0],
                mass: 1e-6,
                material: 0,
            },
            Particle {
                position: [0.0, 0.0, 0.0],
                velocity: [-0.5, 0.0, 0.0],
                mass: 2e-6,
                material: 0,
            },
        ],
        vec![Material::WATER],
        Config::default(),
    )?;
    let before = liquid.clone();
    let kinetic = |l: &Liquid| {
        l.particles()
            .iter()
            .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>()
    };
    let report = liquid.coalesce_swept_droplets(
        &previous,
        DropletCoalescenceControl {
            dt: 0.05,
            surface_tension: 0.072,
            maximum_normal_speed: 2.0,
            max_events: 16,
        },
    )?;
    let error = (kinetic(&before) - kinetic(&liquid) - report.unresolved_kinetic_energy).abs();
    if report.events.len() != 1
        || liquid.particles().len() != 1
        || (liquid.mass() - before.mass()).abs() > 1e-18
        || error > 1e-18
    {
        return Err("collision conservation failed".into());
    }
    println!(
        "coalescences={} mass={} position={:?} velocity={:?} unresolved_kinetic_energy={} surface_release={} kinetic_ledger_error={}",
        report.events.len(),
        liquid.mass(),
        liquid.particles()[0].position,
        liquid.particles()[0].velocity,
        report.unresolved_kinetic_energy,
        report.released_surface_energy,
        error
    );
    Ok(())
}
