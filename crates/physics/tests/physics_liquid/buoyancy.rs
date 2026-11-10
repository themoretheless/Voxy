use physics::liquid::{
    BuoyancyConfig, Config, Error, FloatingBody, FluidLayer, Liquid, Material, Particle,
};
use std::f64::consts::PI;
fn system() -> Liquid {
    Liquid::new(
        vec![Particle {
            position: [0.0, -0.5, 0.0],
            velocity: [0.0; 3],
            mass: 1000.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap()
}
fn body(y: f64, mass: f64) -> FloatingBody {
    FloatingBody {
        position: [0.0, y, 0.0],
        velocity: [0.0; 3],
        mass,
        radius: 0.1,
    }
}
fn layer() -> FluidLayer {
    FluidLayer {
        bottom: -1.0,
        top: 0.0,
        material: 0,
    }
}
fn no_drag() -> BuoyancyConfig {
    BuoyancyConfig {
        drag_coefficient: 0.0,
        ..BuoyancyConfig::default()
    }
}
fn momentum(liquid: &Liquid, body: FloatingBody) -> [f64; 3] {
    std::array::from_fn(|axis| {
        body.mass * body.velocity[axis]
            + liquid
                .particles()
                .iter()
                .map(|p| p.mass * p.velocity[axis])
                .sum::<f64>()
    })
}
#[test]
fn displaced_mass_matches_analytic_sphere_caps_in_multiple_layers() {
    let mut liquid = Liquid::new(
        vec![
            Particle {
                position: [0.0, -0.5, 0.0],
                velocity: [0.0; 3],
                mass: 1000.0,
                material: 0,
            },
            Particle {
                position: [0.0, 0.5, 0.0],
                velocity: [0.0; 3],
                mass: 800.0,
                material: 1,
            },
        ],
        vec![Material::WATER, Material::OIL],
        Config::default(),
    )
    .unwrap();
    let mut sphere = body(0.0, 2.0);
    let layers = [
        layer(),
        FluidLayer {
            bottom: 0.0,
            top: 1.0,
            material: 1,
        },
    ];
    let report = liquid
        .couple_floating_body(&mut sphere, &layers, 0.001, no_drag())
        .unwrap();
    let volume = 4.0 * PI / 3.0 * 0.1_f64.powi(3);
    assert!((report.submerged_volume - volume).abs() < 1e-14);
    assert!((report.displaced_mass - volume * 900.0).abs() < 1e-12);
    let total = momentum(&liquid, sphere);
    assert!((total[1] + 2.0 * 9.81 * 0.001).abs() < 1e-12);
}
#[test]
fn light_body_rises_heavy_body_sinks_and_dry_body_free_falls() {
    for (mass, rises) in [(1.0, true), (10.0, false)] {
        let mut liquid = system();
        let mut sphere = body(-0.5, mass);
        liquid
            .couple_floating_body(&mut sphere, &[layer()], 0.001, no_drag())
            .unwrap();
        assert_eq!(sphere.velocity[1] > 0.0, rises);
    }
    let mut liquid = system();
    let mut sphere = body(0.5, 1.0);
    let report = liquid
        .couple_floating_body(&mut sphere, &[layer()], 0.001, no_drag())
        .unwrap();
    assert!(report.submerged_volume.abs() < 1e-14);
    assert!((sphere.velocity[1] + 9.81 * 0.001).abs() < 1e-12);
}
#[test]
fn neutral_half_submersion_and_recoil_match_archimedes() {
    let mut liquid = system();
    let half_mass = 1000.0 * 2.0 * PI / 3.0 * 0.1_f64.powi(3);
    let mut sphere = body(0.0, half_mass);
    let report = liquid
        .couple_floating_body(&mut sphere, &[layer()], 0.001, no_drag())
        .unwrap();
    // Water viscosity produces negligible extra drag from carrier recoil.
    assert!(sphere.velocity[1].abs() < 1e-8);
    assert!((report.displaced_mass - half_mass).abs() < 1e-12);
    assert!((report.layer_impulses[0][1] + half_mass * 9.81 * 0.001).abs() < 1e-8);
}
#[test]
fn drag_dissipates_energy_and_preserves_total_momentum() {
    let mut liquid = system();
    let mut sphere = body(-0.5, 2.0);
    sphere.velocity = [10.0, 0.0, 0.0];
    let before = momentum(&liquid, sphere);
    let energy = |liquid: &Liquid, body: FloatingBody| -> f64 {
        0.5 * body.mass * body.velocity.iter().map(|v| v * v).sum::<f64>()
            + liquid
                .particles()
                .iter()
                .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
                .sum::<f64>()
    };
    let initial_energy = energy(&liquid, sphere);
    liquid
        .couple_floating_body(
            &mut sphere,
            &[layer()],
            0.05,
            BuoyancyConfig {
                gravity: 0.0,
                ..BuoyancyConfig::default()
            },
        )
        .unwrap();
    assert!(energy(&liquid, sphere) < initial_energy);
    let after = momentum(&liquid, sphere);
    for axis in 0..3 {
        assert!((after[axis] - before[axis]).abs() < 1e-10);
    }
}
#[test]
fn missing_carrier_overlapping_layers_and_work_limits_are_atomic() {
    let mut liquid = system();
    let mut sphere = body(-0.5, 2.0);
    let before = liquid.clone();
    let body_before = sphere;
    let missing = FluidLayer {
        bottom: -0.4,
        top: 0.0,
        material: 0,
    };
    sphere.position[1] = -0.35;
    let missing_body = sphere;
    assert_eq!(
        liquid.couple_floating_body(&mut sphere, &[missing], 0.01, no_drag()),
        Err(Error::MissingFluidCarrier)
    );
    assert_eq!(liquid, before);
    assert_eq!(sphere, missing_body);
    sphere = body_before;
    assert_eq!(
        liquid.couple_floating_body(&mut sphere, &[layer(), layer()], 0.01, no_drag()),
        Err(Error::InvalidBuoyancy)
    );
    assert_eq!(liquid, before);
    assert_eq!(sphere, body_before);
    let mut liquid = Liquid::new(
        vec![before.particles()[0]; 2],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    let initial = liquid.clone();
    assert_eq!(
        liquid.couple_floating_body(
            &mut sphere,
            &[layer()],
            0.01,
            BuoyancyConfig {
                max_checks: 1,
                ..no_drag()
            }
        ),
        Err(Error::BuoyancyBudget)
    );
    assert_eq!(liquid, initial);
    assert_eq!(sphere, body_before);
}
