use physics::liquid::{
    Config, FilmRebound, Liquid, LiquidField, Material, Particle, TransportMaterial,
};
use physics::surface_film::{Material as FilmMaterial, SurfaceFilm};
fn film() -> SurfaceFilm {
    SurfaceFilm::new(
        &[[-2.0, 0.0, -2.0], [2.0, 0.0, -2.0], [0.0, 0.0, 2.0]],
        vec![[0, 1, 2]],
        FilmMaterial::default(),
    )
    .unwrap()
}
fn fluid() -> Liquid {
    Liquid::new(
        vec![Particle {
            position: [0.2, -0.5, 0.0],
            velocity: [2.0, -3.0, 1.0],
            mass: 2.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap()
}
#[test]
fn oblique_rebound_closes_momentum_and_kinetic_energy_ledgers() {
    let mut l = fluid();
    let f = film();
    let report = l
        .rebound_surface_film(
            &[[0.0, 0.5, 0.0]],
            &f,
            FilmRebound {
                restitution: 0.5,
                friction: 0.25,
            },
        )
        .unwrap();
    assert_eq!(report.particles, 1);
    assert_eq!(l.particles()[0].velocity, [1.5, 1.5, 0.75]);
    assert!((l.particles()[0].position[0] - 0.175).abs() < 1e-15);
    assert!((l.particles()[0].position[1] - 0.25).abs() < 1e-13);
    let before = [2.0, -3.0, 1.0];
    let after = l.particles()[0].velocity;
    for axis in 0..3 {
        assert!(
            (2.0 * after[axis] + report.substrate_impulse[axis] - 2.0 * before[axis]).abs() < 1e-14
        );
    }
    let energy = |v: [f64; 3]| v.iter().map(|v| v * v).sum::<f64>();
    assert!((energy(after) + report.dissipated_energy - energy(before)).abs() < 1e-14);
    assert_eq!(f.total_mass(), 0.0);
}
#[test]
fn elastic_and_stopping_limits_preserve_mass_and_composition() {
    for (restitution, friction) in [(1.0, 0.0), (0.0, 1.0)] {
        let mut l = fluid();
        l.configure_transport(
            vec![LiquidField {
                temperature: 310.0,
                concentration: 0.3,
            }],
            vec![TransportMaterial::default()],
        )
        .unwrap();
        l.configure_species(vec!["water".into(), "gel".into()], vec![vec![0.8, 0.2]])
            .unwrap();
        let totals = l.species_totals().unwrap();
        let heat = l.transport_totals().unwrap();
        let report = l
            .rebound_surface_film(
                &[[0.0, 0.5, 0.0]],
                &film(),
                FilmRebound {
                    restitution,
                    friction,
                },
            )
            .unwrap();
        assert_eq!(l.species_totals().unwrap(), totals);
        assert_eq!(l.transport_totals().unwrap(), heat);
        if restitution == 1.0 {
            assert_eq!(report.dissipated_energy, 0.0);
        } else {
            assert_eq!(l.particles()[0].velocity, [0.0; 3]);
            assert_eq!(report.dissipated_energy, 14.0);
        }
    }
}
#[test]
fn underside_and_miss_work_and_invalid_controls_roll_back() {
    let mut l = fluid();
    let original = l.clone();
    let f = film();
    assert!(
        l.rebound_surface_film(
            &[[0.0, 0.5, 0.0]],
            &f,
            FilmRebound {
                restitution: 1.1,
                friction: 0.0
            }
        )
        .is_err()
    );
    assert_eq!(l, original);
    assert!(
        l.rebound_surface_film(
            &[[f64::NAN, 0.5, 0.0]],
            &f,
            FilmRebound {
                restitution: 1.0,
                friction: 0.0
            }
        )
        .is_err()
    );
    assert_eq!(l, original);
    let r = l
        .rebound_surface_film(
            &[[0.0, -1.0, 0.0]],
            &f,
            FilmRebound {
                restitution: 1.0,
                friction: 0.0,
            },
        )
        .unwrap();
    assert_eq!(r.particles, 0);
    assert_eq!(l, original);
    let mut p = original.particles()[0];
    p.position[1] = 0.5;
    p.velocity[1] = 3.0;
    let mut under = Liquid::new(vec![p], vec![Material::WATER], Config::default()).unwrap();
    under
        .rebound_surface_film(
            &[[0.0, -0.5, 0.0]],
            &f,
            FilmRebound {
                restitution: 1.0,
                friction: 0.0,
            },
        )
        .unwrap();
    assert_eq!(under.particles()[0].velocity[1], -3.0);
    assert!((under.particles()[0].position[1] + 0.5).abs() < 1e-13);
}

#[test]
fn zero_restitution_contact_remains_detectable_under_gravity() {
    let mut l = fluid();
    let f = film();
    let model = FilmRebound {
        restitution: 0.0,
        friction: 1.0,
    };
    l.rebound_surface_film(&[[0.0, 0.5, 0.0]], &f, model)
        .unwrap();
    assert!(l.particles()[0].position[1] > 0.0);
    for _ in 0..4 {
        let previous = vec![l.particles()[0].position];
        l.step(0.001, None).unwrap();
        let r = l.rebound_surface_film(&previous, &f, model).unwrap();
        assert_eq!(r.particles, 1);
        assert!(l.particles()[0].position[1] >= 0.0);
        assert_eq!(l.particles()[0].velocity, [0.0; 3]);
    }
}
