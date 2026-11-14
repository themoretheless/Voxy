//! Apparent contact-angle estimator against synthetic spherical caps.
use physics::liquid::{Config, Container, Error, Liquid, Material, Particle, WallFace};
use std::f64::consts::PI;

fn config() -> Config {
    Config {
        smoothing_radius: 0.1,
        particle_radius: 0.02,
        gravity: [0.0; 3],
        ..Config::default()
    }
}
fn walls() -> Container {
    Container {
        min: [0.0; 3],
        max: [2.0; 3],
        restitution: 0.0,
        friction: 0.0,
    }
}
fn materials() -> Vec<Material> {
    vec![
        Material {
            rest_density: 1000.0,
            sound_speed: 20.0,
            viscosity: 0.001,
        };
        2
    ]
}
/// Particle centres on a spherical cap of radius `r` whose plane of centres is the
/// collision plane of `face` shifted by the particle radius, meeting it at `angle`.
fn cap(face: WallFace, angle: f64, r: f64, material: usize) -> Vec<Particle> {
    let radius = config().particle_radius;
    let plane = if face.lower {
        walls().min[face.axis] + radius
    } else {
        walls().max[face.axis] - radius
    };
    let inward = if face.lower { 1.0 } else { -1.0 };
    let height = -r * angle.cos();
    let mut center = [1.0, 1.0, 1.0];
    center[face.axis] = plane + inward * height;
    let count = 600;
    let golden = PI * (3.0 - 5f64.sqrt());
    (0..count)
        .filter_map(|i| {
            let z = 1.0 - 2.0 * (i as f64 + 0.5) / count as f64;
            let rho = (1.0 - z * z).sqrt();
            let phi = golden * i as f64;
            let local = [rho * phi.cos(), rho * phi.sin(), z];
            // Rotate so that `local[2]` points along the inward wall normal.
            let mut dir = [0.0; 3];
            let others: Vec<usize> = (0..3).filter(|a| *a != face.axis).collect();
            dir[face.axis] = inward * local[2];
            dir[others[0]] = local[0];
            dir[others[1]] = local[1];
            let position = [
                center[0] + r * dir[0],
                center[1] + r * dir[1],
                center[2] + r * dir[2],
            ];
            let above = inward * (position[face.axis] - plane) >= -1e-12;
            above.then_some(Particle {
                position,
                velocity: [0.0; 3],
                mass: 1e-3,
                material,
            })
        })
        .collect()
}

#[test]
fn spherical_caps_recover_the_prescribed_angle_on_every_face() {
    for face in [
        WallFace {
            axis: 1,
            lower: true,
        },
        WallFace {
            axis: 0,
            lower: false,
        },
        WallFace {
            axis: 2,
            lower: true,
        },
    ] {
        for degrees in [30.0, 60.0, 90.0, 120.0, 150.0] {
            let angle = degrees * PI / 180.0;
            let r = 0.3;
            let particles = cap(face, angle, r, 0);
            assert!(particles.len() >= 20, "{degrees}: {}", particles.len());
            let liquid = Liquid::new(particles, materials(), config()).unwrap();
            let estimate = liquid.apparent_contact_angle(0, walls(), face).unwrap();
            assert!(
                (estimate.angle_rad - angle).abs() < 1e-9,
                "{degrees}° on {face:?}: {}",
                estimate.angle_rad * 180.0 / PI
            );
            assert!((estimate.sphere_radius - r).abs() < 1e-9);
            assert!((estimate.contact_radius - r * angle.sin()).abs() < 1e-9);
            assert!(estimate.rms_misfit < 1e-9);
        }
    }
}

#[test]
fn only_the_requested_material_is_fitted() {
    let face = WallFace {
        axis: 1,
        lower: true,
    };
    let mut particles = cap(face, 0.5 * PI, 0.3, 0);
    let mut other = cap(face, 0.25 * PI, 0.15, 1);
    for p in &mut other {
        p.position[0] += 0.8;
    }
    particles.extend(other);
    let liquid = Liquid::new(particles, materials(), config()).unwrap();
    let a = liquid.apparent_contact_angle(0, walls(), face).unwrap();
    let b = liquid.apparent_contact_angle(1, walls(), face).unwrap();
    assert!((a.angle_rad - 0.5 * PI).abs() < 1e-9);
    assert!((b.angle_rad - 0.25 * PI).abs() < 1e-9);
    assert!((b.sphere_center[0] - 1.8).abs() < 1e-9);
    assert_eq!(a.particles + b.particles, liquid.particles().len());
}

#[test]
fn degenerate_samples_and_bad_selectors_are_rejected() {
    let face = WallFace {
        axis: 1,
        lower: true,
    };
    let few = cap(face, 0.5 * PI, 0.3, 0).into_iter().take(3).collect();
    let liquid = Liquid::new(few, materials(), config()).unwrap();
    assert_eq!(
        liquid.apparent_contact_angle(0, walls(), face),
        Err(Error::InvalidParticle)
    );
    let coplanar = (0..12)
        .map(|i| Particle {
            position: [
                0.3 + 0.05 * f64::from(i % 4),
                0.02,
                0.3 + 0.05 * f64::from(i / 4),
            ],
            velocity: [0.0; 3],
            mass: 1e-3,
            material: 0,
        })
        .collect();
    let liquid = Liquid::new(coplanar, materials(), config()).unwrap();
    assert_eq!(
        liquid.apparent_contact_angle(0, walls(), face),
        Err(Error::NumericalFailure)
    );
    let full = cap(face, 0.5 * PI, 0.3, 0);
    let liquid = Liquid::new(full, materials(), config()).unwrap();
    assert_eq!(
        liquid.apparent_contact_angle(2, walls(), face),
        Err(Error::InvalidMaterial)
    );
    assert_eq!(
        liquid.apparent_contact_angle(
            0,
            walls(),
            WallFace {
                axis: 3,
                lower: true
            }
        ),
        Err(Error::InvalidMaterial)
    );
    // A hemisphere resting on the floor does not reach the ceiling.
    assert_eq!(
        liquid.apparent_contact_angle(
            0,
            walls(),
            WallFace {
                axis: 1,
                lower: false
            }
        ),
        Err(Error::NumericalFailure)
    );
}
