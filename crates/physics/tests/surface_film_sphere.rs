use physics::liquid::{Config, FilmRebound, Liquid, Material as LiquidMaterial, Particle};
use physics::surface_film::{Material, SurfaceFilm};
fn film() -> SurfaceFilm {
    SurfaceFilm::new(
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        Material::default(),
    )
    .unwrap()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} != {b}");
}
#[test]
fn contact_time_is_invariant_under_rotation_translation_and_scale() {
    for scale in [0.001, 1.0, 1000.0] {
        let transform = |p: [f64; 3]| [2.0 + scale * p[2], -3.0 + scale * p[0], 4.0 + scale * p[1]];
        let points = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]].map(transform);
        let f = SurfaceFilm::new(&points, vec![[0, 1, 2]], Material::default()).unwrap();
        let hit = f
            .first_sphere_hit(
                transform([0.5, 0.5, -0.06]),
                transform([0.5, -0.5, -0.06]),
                0.1 * scale,
            )
            .unwrap()
            .unwrap();
        assert!((hit.time - 0.42).abs() < 1e-11);
        for (actual, expected) in hit.normal.into_iter().zip([-0.6, 0.0, 0.8]) {
            assert!((actual - expected).abs() < 1e-11);
        }
    }
}

#[test]
fn late_sphere_penetration_rolls_back_an_earlier_valid_impact() {
    let particles = vec![
        Particle {
            position: [0.2, -0.5, 0.2],
            velocity: [0.0, -2.0, 0.0],
            mass: 0.001,
            material: 0,
        },
        Particle {
            position: [0.4, -0.5, 0.2],
            velocity: [0.0, -2.0, 0.0],
            mass: 0.001,
            material: 0,
        },
    ];
    let mut liquid =
        Liquid::new(particles, vec![LiquidMaterial::WATER], Config::default()).unwrap();
    let initial = liquid.clone();
    assert!(
        liquid
            .rebound_spheres_surface_film(
                &[[0.2, 0.5, 0.2], [0.4, 0.05, 0.2]],
                &film(),
                FilmRebound {
                    restitution: 0.5,
                    friction: 0.0
                },
                &[0.1, 0.1]
            )
            .is_err()
    );
    assert_eq!(liquid, initial);
    assert!(
        liquid
            .rebound_spheres_surface_film(
                &[[0.2, 0.5, 0.2], [0.4, 0.5, 0.2]],
                &film(),
                FilmRebound {
                    restitution: 0.5,
                    friction: 0.0
                },
                &[0.1]
            )
            .is_err()
    );
    assert_eq!(liquid, initial);
}
#[test]
fn sphere_contacts_face_edge_and_vertex_before_center_crossing() {
    let f = film();
    for side in [-1.0, 1.0] {
        let hit = f
            .first_sphere_hit([0.2, side, 0.2], [0.2, -side, 0.2], 0.1)
            .unwrap()
            .unwrap();
        near(hit.time, 0.45);
        near(hit.normal[1], side);
        for (actual, expected) in hit.point.into_iter().zip([0.2, 0.0, 0.2]) {
            near(actual, expected);
        }
    }
    let edge = f
        .first_sphere_hit([0.5, 0.5, -0.06], [0.5, -0.5, -0.06], 0.1)
        .unwrap()
        .unwrap();
    near(edge.time, 0.42);
    near(edge.normal[1], 0.8);
    near(edge.normal[2], -0.6);
    assert_eq!(edge.point, [0.5, 0.0, 0.0]);
    let vertex = f
        .first_sphere_hit([-0.06, 0.5, -0.06], [-0.06, -0.5, -0.06], 0.1)
        .unwrap()
        .unwrap();
    near(
        vertex.time,
        0.5 - (0.01_f64 - 2.0 * 0.06_f64.powi(2)).sqrt(),
    );
    assert_eq!(vertex.point, [0.0; 3]);
    assert!(
        f.first_segment_hit([-0.06, 0.5, -0.06], [-0.06, -0.5, -0.06])
            .unwrap()
            .is_none()
    );
}
#[test]
fn tangency_overlap_stationary_and_misses_have_defined_results() {
    let f = film();
    let tangent = f
        .first_sphere_hit([0.5, 0.5, -0.1], [0.5, -0.5, -0.1], 0.1)
        .unwrap()
        .unwrap();
    assert!((tangent.time - 0.5).abs() < 1e-8);
    let overlap = f
        .first_sphere_hit([0.2, 0.05, 0.2], [0.2, 0.05, 0.2], 0.1)
        .unwrap()
        .unwrap();
    assert_eq!(overlap.time, 0.0);
    near(overlap.penetration, 0.05);
    assert!(
        f.first_sphere_hit([0.5, 0.2, -1.0], [0.5, 0.2, 1.0], 0.1)
            .unwrap()
            .is_none()
    );
    assert!(
        f.first_sphere_hit([0.2, 1.0, 0.2], [0.2, 1.0, 0.2], 0.1)
            .unwrap()
            .is_none()
    );
    for r in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(f.first_sphere_hit([0.0; 3], [1.0; 3], r).is_err());
    }
}

#[test]
fn purely_grazing_sphere_does_not_receive_a_friction_impulse() {
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [0.5, -0.5, -0.1],
            velocity: [0.0, -2.0, 0.0],
            mass: 0.001,
            material: 0,
        }],
        vec![LiquidMaterial::WATER],
        Config::default(),
    )
    .unwrap();
    let initial = liquid.clone();
    let report = liquid
        .rebound_spheres_surface_film(
            &[[0.5, 0.5, -0.1]],
            &film(),
            FilmRebound {
                restitution: 0.5,
                friction: 0.9,
            },
            &[0.1],
        )
        .unwrap();
    assert_eq!(report.particles, 0);
    assert_eq!(report.substrate_impulse, [0.0; 3]);
    assert_eq!(report.dissipated_energy, 0.0);
    assert_eq!(liquid, initial);
}
#[test]
fn finite_radius_rebound_conserves_momentum_and_energy_and_rejects_penetration() {
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [0.2, -0.5, 0.2],
            velocity: [0.0, -2.0, 0.0],
            mass: 0.001,
            material: 0,
        }],
        vec![LiquidMaterial::WATER],
        Config::default(),
    )
    .unwrap();
    let report = liquid
        .rebound_spheres_surface_film(
            &[[0.2, 0.5, 0.2]],
            &film(),
            FilmRebound {
                restitution: 0.5,
                friction: 0.0,
            },
            &[0.1],
        )
        .unwrap();
    near(liquid.particles()[0].position[1], 0.4);
    near(liquid.particles()[0].velocity[1], 1.0);
    near(report.substrate_impulse[1], -0.003);
    near(report.dissipated_energy, 0.0015);
    let snapshot = liquid.clone();
    assert_eq!(
        liquid
            .rebound_spheres_surface_film(
                &[[0.2, 0.05, 0.2]],
                &film(),
                FilmRebound {
                    restitution: 0.5,
                    friction: 0.0
                },
                &[0.1]
            )
            .unwrap_err(),
        "initial film sphere penetration"
    );
    assert_eq!(liquid, snapshot);
    let report = liquid
        .rebound_spheres_surface_film(
            &[[0.2, 0.1, 0.2]],
            &film(),
            FilmRebound {
                restitution: 0.5,
                friction: 0.0,
            },
            &[0.1],
        )
        .unwrap();
    assert_eq!(report.particles, 0);
    assert_eq!(liquid, snapshot);
}
