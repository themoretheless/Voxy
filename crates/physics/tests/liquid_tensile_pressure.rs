//! Plan step F1: an admitted tension floor lets a stretched pair pull back,
//! while the default clamp leaves it inert. The default path is unchanged.
use physics::liquid::{Config, Liquid, Material, Particle};

fn stretched_pair(floor: Option<f64>) -> Liquid {
    let mut fluid = Liquid::new(
        vec![
            Particle {
                position: [-0.3, 0.0, 0.0],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            },
            Particle {
                position: [0.3, 0.0, 0.0],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            },
        ],
        vec![Material {
            // Rest density above the pair's SPH density: the pair is under tension.
            rest_density: 10.0,
            sound_speed: 1.0,
            viscosity: 0.0,
        }],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 1.0,
            ..Config::default()
        },
    )
    .unwrap();
    fluid.set_tensile_pressure_floor(floor).unwrap();
    fluid
}

fn separation(fluid: &Liquid) -> f64 {
    let p = fluid.particles();
    p[1].position[0] - p[0].position[0]
}

#[test]
fn tension_pulls_a_stretched_pair_together_and_the_default_leaves_it_inert() {
    let mut clamped = stretched_pair(None);
    let mut tensile = stretched_pair(Some(1e3));
    let before = separation(&clamped);
    for _ in 0..20 {
        clamped.step(0.01, None).unwrap();
        tensile.step(0.01, None).unwrap();
    }
    assert_eq!(
        separation(&clamped),
        before,
        "clamped pressure must not move the pair"
    );
    assert!(
        separation(&tensile) < before - 1e-6,
        "tension should pull the pair together: {} -> {}",
        before,
        separation(&tensile)
    );
    let p = tensile.particles();
    assert!(p[0].velocity[0] > 0. && p[1].velocity[0] < 0.);
    // Momentum is conserved by the symmetric pair force.
    assert!((p[0].velocity[0] + p[1].velocity[0]).abs() < 1e-12);
}

#[test]
fn the_floor_bounds_the_tension_and_input_is_validated() {
    // A tiny floor admits only that much tension, so the pull is weaker.
    let mut strong = stretched_pair(Some(1e3));
    let mut weak = stretched_pair(Some(1e-3));
    let before = separation(&strong);
    for _ in 0..20 {
        strong.step(0.01, None).unwrap();
        weak.step(0.01, None).unwrap();
    }
    assert!(before - separation(&weak) < before - separation(&strong));
    assert!(before - separation(&weak) > 0.);
    let mut fluid = stretched_pair(None);
    assert!(fluid.set_tensile_pressure_floor(Some(-1.)).is_err());
    assert!(fluid.set_tensile_pressure_floor(Some(f64::NAN)).is_err());
    assert_eq!(fluid.tensile_pressure_floor(), None);
    fluid.set_tensile_pressure_floor(Some(2.)).unwrap();
    assert_eq!(fluid.tensile_pressure_floor(), Some(2.));
}
