use physics::liquid::{Config, FilmRebound, Liquid, Material, Particle};
use physics::surface_film::{Material as FilmMaterial, SurfaceFilm};
fn corridor() -> SurfaceFilm {
    let points: Vec<_> = [0.0, 1.0]
        .into_iter()
        .flat_map(|x| {
            [
                [x, -2.0, -2.0],
                [x, 2.0, -2.0],
                [x, 2.0, 2.0],
                [x, -2.0, 2.0],
            ]
        })
        .collect();
    SurfaceFilm::new(
        &points,
        vec![[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]],
        FilmMaterial::default(),
    )
    .unwrap()
}
fn fluid(end: f64, speed: f64) -> Liquid {
    Liquid::new(
        vec![Particle {
            position: [end, 0.0, 0.0],
            velocity: [speed, 0.0, 0.0],
            mass: 1.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap()
}
fn elastic() -> FilmRebound {
    FilmRebound {
        restitution: 1.0,
        friction: 0.0,
    }
}
fn near_vector(actual: [f64; 3], expected: [f64; 3]) {
    for (a, b) in actual.into_iter().zip(expected) {
        assert!((a - b).abs() < 1e-12, "{a} != {b}");
    }
}
#[test]
fn four_elastic_contacts_match_independent_folded_corridor_path() {
    let mut liquid = fluid(3.5, 3.0);
    let report = liquid
        .rebound_spheres_surface_film_multi(&[[0.5, 0.0, 0.0]], &corridor(), elastic(), &[0.1], 8)
        .unwrap();
    // Center travels in [0.1,0.9]. Reflect the unfolded 3 m path with period 1.6 m.
    assert_eq!(report.contacts, 4);
    assert_eq!(report.rebound.particles, 1);
    assert!((liquid.particles()[0].position[0] - 0.3).abs() < 1e-10);
    near_vector(liquid.particles()[0].velocity, [3.0, 0.0, 0.0]);
    near_vector(report.rebound.substrate_impulse, [0.0; 3]);
    assert_eq!(report.rebound.dissipated_energy, 0.0);
}
#[test]
fn inelastic_multiple_contacts_preserve_momentum_and_energy_ledgers() {
    let mut liquid = fluid(4.5, 4.0);
    let report = liquid
        .rebound_spheres_surface_film_multi(
            &[[0.5, 0.0, 0.0]],
            &corridor(),
            FilmRebound {
                restitution: 0.5,
                friction: 0.0,
            },
            &[0.1],
            8,
        )
        .unwrap();
    assert_eq!(report.contacts, 2);
    assert!((liquid.particles()[0].position[0] - 0.6).abs() < 1e-10);
    near_vector(liquid.particles()[0].velocity, [1.0, 0.0, 0.0]);
    near_vector(report.rebound.substrate_impulse, [3.0, 0.0, 0.0]);
    assert_eq!(report.rebound.dissipated_energy, 7.5);
    assert_eq!(0.5 + report.rebound.dissipated_energy, 8.0);
}
#[test]
fn separating_initial_touch_does_not_mask_a_later_wall() {
    let mut liquid = fluid(2.1, 2.0);
    let report = liquid
        .rebound_spheres_surface_film_multi(&[[0.1, 0.0, 0.0]], &corridor(), elastic(), &[0.1], 8)
        .unwrap();
    assert_eq!(report.contacts, 2);
    assert!((liquid.particles()[0].position[0] - 0.5).abs() < 1e-10);
    near_vector(liquid.particles()[0].velocity, [2.0, 0.0, 0.0]);
    let mut single = fluid(1.5, 2.0);
    let report = single
        .rebound_spheres_surface_film(&[[0.1, 0.0, 0.0]], &corridor(), elastic(), &[0.1])
        .unwrap();
    assert_eq!(report.particles, 1);
    near_vector(single.particles()[0].velocity, [-2.0, 0.0, 0.0]);
}
#[test]
fn contact_budget_failure_rolls_back_all_particle_fields() {
    use physics::liquid::{LiquidField, TransportMaterial};
    let mut liquid = fluid(3.5, 3.0);
    liquid
        .configure_transport(
            vec![LiquidField {
                temperature: 300.0,
                concentration: 0.0,
            }],
            vec![TransportMaterial::default()],
        )
        .unwrap();
    liquid
        .configure_species(vec!["a".into(), "b".into()], vec![vec![0.2, 0.8]])
        .unwrap();
    let original = liquid.clone();
    assert_eq!(
        liquid
            .rebound_spheres_surface_film_multi(
                &[[0.5, 0.0, 0.0]],
                &corridor(),
                elastic(),
                &[0.1],
                2
            )
            .unwrap_err(),
        "film multi-impact contact budget"
    );
    assert_eq!(liquid, original);
    assert!(
        liquid
            .rebound_spheres_surface_film_multi(
                &[[0.5, 0.0, 0.0]],
                &corridor(),
                elastic(),
                &[0.1],
                0
            )
            .is_err()
    );
    assert_eq!(liquid, original);
}
#[test]
fn subdivided_constant_force_free_path_matches_complete_multi_impact_step() {
    let film = corridor();
    let mut full = fluid(3.5, 3.0);
    let all = full
        .rebound_spheres_surface_film_multi(&[[0.5, 0.0, 0.0]], &film, elastic(), &[0.1], 8)
        .unwrap();
    let mut start = 0.5;
    let mut speed = 3.0;
    let mut contacts = 0;
    for _ in 0..10 {
        let mut part = fluid(start + 0.1 * speed, speed);
        let report = part
            .rebound_spheres_surface_film_multi(&[[start, 0.0, 0.0]], &film, elastic(), &[0.1], 1)
            .unwrap();
        contacts += report.contacts;
        start = part.particles()[0].position[0];
        speed = part.particles()[0].velocity[0];
    }
    assert_eq!(contacts, all.contacts);
    assert!((start - full.particles()[0].position[0]).abs() < 1e-10);
    assert_eq!(speed, full.particles()[0].velocity[0]);
}

#[test]
fn orthogonal_corner_resolves_second_zero_time_contact() {
    let points = [
        [1.0, -2.0, -2.0],
        [1.0, 2.0, -2.0],
        [1.0, 2.0, 2.0],
        [1.0, -2.0, 2.0],
        [-2.0, 1.0, -2.0],
        [2.0, 1.0, -2.0],
        [2.0, 1.0, 2.0],
        [-2.0, 1.0, 2.0],
    ];
    let film = SurfaceFilm::new(
        &points,
        vec![[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]],
        FilmMaterial::default(),
    )
    .unwrap();
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [2.5, 2.5, 0.0],
            velocity: [2.0, 2.0, 0.0],
            mass: 1.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    let report = liquid
        .rebound_spheres_surface_film_multi(&[[0.5, 0.5, 0.0]], &film, elastic(), &[0.1], 8)
        .unwrap();
    assert_eq!(report.contacts, 2);
    near_vector(liquid.particles()[0].position, [-0.7, -0.7, 0.0]);
    near_vector(liquid.particles()[0].velocity, [-2.0, -2.0, 0.0]);
    near_vector(report.rebound.substrate_impulse, [4.0, 4.0, 0.0]);
    assert!(report.rebound.dissipated_energy.abs() < 1e-12);
}

#[test]
fn later_particle_budget_error_restores_earlier_particle_and_composition() {
    use physics::liquid::{LiquidField, TransportMaterial};
    let mut liquid = Liquid::new(
        vec![
            Particle {
                position: [1.1, 0.0, 0.0],
                velocity: [0.6, 0.0, 0.0],
                mass: 1.0,
                material: 0,
            },
            Particle {
                position: [3.5, 0.0, 0.5],
                velocity: [3.0, 0.0, 0.0],
                mass: 1.0,
                material: 0,
            },
        ],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    liquid
        .configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.2,
                },
                LiquidField {
                    temperature: 320.0,
                    concentration: 0.8,
                },
            ],
            vec![TransportMaterial::default()],
        )
        .unwrap();
    liquid
        .configure_species(
            vec!["a".into(), "b".into()],
            vec![vec![0.2, 0.8], vec![0.7, 0.3]],
        )
        .unwrap();
    let initial = liquid.clone();
    assert_eq!(
        liquid
            .rebound_spheres_surface_film_multi(
                &[[0.5, 0.0, 0.0], [0.5, 0.0, 0.5]],
                &corridor(),
                elastic(),
                &[0.1, 0.1],
                2
            )
            .unwrap_err(),
        "film multi-impact contact budget"
    );
    assert_eq!(liquid, initial);
}
