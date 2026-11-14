use physics::liquid::{Config, Liquid, LiquidField, Material, Particle, TransportMaterial};
use physics::surface_film::{Material as FilmMaterial, SurfaceFilm};
fn film() -> SurfaceFilm {
    SurfaceFilm::new(
        &[
            [-1.0, 0.0, -1.0],
            [1.0, 0.0, -1.0],
            [1.0, 0.0, 1.0],
            [-1.0, 0.0, 1.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        FilmMaterial {
            density: 1000.0,
            wetting: 1e-3,
            ..FilmMaterial::default()
        },
    )
    .unwrap()
}
fn fluid() -> Liquid {
    let mut l = Liquid::new(
        vec![
            Particle {
                position: [0.5, -0.1, 0.0],
                velocity: [1.0, -2.0, 0.0],
                mass: 0.01,
                material: 0,
            },
            Particle {
                position: [-0.5, -0.1, 0.0],
                velocity: [0.0, -3.0, 1.0],
                mass: 0.02,
                material: 0,
            },
            Particle {
                position: [2.0, -0.1, 0.0],
                velocity: [0.0, -1.0, 0.0],
                mass: 0.03,
                material: 0,
            },
        ],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    l.configure_transport(
        vec![
            LiquidField {
                temperature: 300.0,
                concentration: 0.0
            };
            3
        ],
        vec![TransportMaterial::default()],
    )
    .unwrap();
    l
}
fn starts() -> Vec<[f64; 3]> {
    vec![[0.5, 0.1, 0.0], [-0.5, 0.1, 0.0], [2.0, 0.1, 0.0]]
}
#[test]
fn intersection_captures_once_misses_remain_and_ledgers_close() {
    let mut l = fluid();
    let mut f = film();
    let initial_heat = l.transport_totals().unwrap().unwrap().0;
    let capture = l.capture_surface_film(&starts(), &mut f).unwrap();
    assert_eq!(capture.particles, 2);
    assert!((capture.absorbed.mass - 0.03).abs() < 1e-15);
    assert!((f.total_mass() - 0.03).abs() < 1e-15);
    assert!((f.total_volume() - 3e-5).abs() < 1e-18);
    assert_eq!(l.particles().len(), 1);
    assert_eq!(l.particles()[0].mass, 0.03);
    assert!((capture.absorbed.momentum[0] - 0.01).abs() < 1e-15);
    assert!((capture.absorbed.momentum[1] + 0.08).abs() < 1e-15);
    assert!((capture.absorbed.momentum[2] - 0.02).abs() < 1e-15);
    assert!((capture.absorbed.kinetic_energy - 0.125).abs() < 1e-15);
    assert!(
        (l.transport_totals().unwrap().unwrap().0 + capture.absorbed.thermal_energy.unwrap()
            - initial_heat)
            .abs()
            < 1e-8
    );
    let before = f.total_mass();
    let capture = l.capture_surface_film(&[[2.0, 0.1, 0.0]], &mut f).unwrap();
    assert_eq!(capture.particles, 0);
    assert_eq!(before, f.total_mass());
}
#[test]
fn capture_failures_preserve_both_states_and_batch_deposits_are_atomic() {
    let mut l = fluid();
    let mut f = film();
    let before = l.clone();
    let heights = f.thickness();
    assert!(l.capture_surface_film(&[[0.0; 3]], &mut f).is_err());
    assert_eq!(l, before);
    assert_eq!(f.thickness(), heights);
    f.set_material(FilmMaterial {
        density: 900.0,
        ..f.material()
    })
    .unwrap();
    assert!(l.capture_surface_film(&starts(), &mut f).is_err());
    assert_eq!(l, before);
    assert_eq!(f.thickness(), heights);
    assert!(f.deposit_batch(&[(0, 1e-5), (999, 1e-5)]).is_err());
    assert_eq!(f.thickness(), heights);
    assert!(f.deposit_batch(&[(0, f64::MAX), (0, f64::MAX)]).is_err());
    assert_eq!(f.thickness(), heights);
    let mut bad = starts();
    bad[2][0] = f64::NAN;
    assert!(l.capture_surface_film(&bad, &mut f).is_err());
    assert_eq!(l, before);
    assert_eq!(f.thickness(), heights);
}
#[test]
fn first_surface_wins_and_parallel_starting_contact_does_not_capture() {
    let f = SurfaceFilm::new(
        &[
            [-1.0, 0.0, -1.0],
            [1.0, 0.0, -1.0],
            [0.0, 0.0, 1.0],
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [0.0, -1.0, 1.0],
        ],
        vec![[3, 4, 5], [0, 1, 2]],
        FilmMaterial::default(),
    )
    .unwrap();
    let hit = f
        .first_segment_hit([0.0, 1.0, 0.0], [0.0, -2.0, 0.0])
        .unwrap()
        .unwrap();
    assert_eq!(hit.0, 1);
    assert!((hit.1 - 1.0 / 3.0).abs() < 1e-15);
    assert!(
        f.first_segment_hit([0.0, 0.0, 0.0], [0.0, 1.0, 0.0])
            .unwrap()
            .is_none()
    );
    assert!(
        f.first_segment_hit([0.0, 0.1, 0.0], [0.1, 0.1, 0.0])
            .unwrap()
            .is_none()
    );
    assert!(f.first_segment_hit([0.0; 3], [0.0; 3]).unwrap().is_none());
}
#[test]
fn deposited_film_spreads_and_retains_captured_mass() {
    let mut l = fluid();
    let mut f = film();
    // Only one wet cell initially; skip the other particle by giving no crossing.
    let mut paths = starts();
    paths[1] = l.particles()[1].position;
    l.capture_surface_film(&paths, &mut f).unwrap();
    let before = f.thickness();
    let mass = f.total_mass();
    assert_eq!(before[1], 0.0);
    f.step(0.1, [0.0; 3]).unwrap();
    assert!(f.thickness()[1] > 0.0);
    assert!((f.total_mass() - mass).abs() < 1e-15);
}

fn mixed_fluid() -> Liquid {
    let mut l = fluid();
    l.configure_species(
        vec!["aqueous".into(), "gel".into()],
        vec![vec![0.25, 0.75], vec![0.8, 0.2], vec![0.6, 0.4]],
    )
    .unwrap();
    l
}
fn mixed_film() -> physics::surface_film::FilmMixture {
    let mut f = film();
    f.deposit(0, 0.00001).unwrap();
    physics::surface_film::FilmMixture::new(
        f,
        vec!["aqueous".into(), "gel".into()],
        vec![vec![1.0, 0.0]; 2],
    )
    .unwrap()
}
#[test]
fn unresolved_pure_capture_rolls_back_particle_batch_and_film() {
    for earlier_mass in [None, Some(0.01)] {
        let mut particles = Vec::new();
        for mass in earlier_mass.into_iter().chain([1e-30]) {
            particles.push(Particle {
                position: [0.5, -0.1, 0.],
                velocity: [0., -1., 0.],
                mass,
                material: 0,
            });
        }
        let mut liquid = Liquid::new(particles, vec![Material::WATER], Config::default()).unwrap();
        let mut film = film();
        film.deposit(0, 0.00001).unwrap();
        let before_liquid = format!("{liquid:?}");
        let before_film = format!("{film:?}");
        let paths = vec![[0.5, 0.1, 0.]; liquid.particles().len()];
        let result = liquid.capture_surface_film(&paths, &mut film);
        assert!(
            result.is_err(),
            "unresolved pure capture succeeded: {result:?}"
        );
        assert_eq!(format!("{liquid:?}"), before_liquid);
        assert_eq!(format!("{film:?}"), before_film);
    }
}
#[test]
fn unresolved_mixture_capture_keeps_particle_and_film_inventory() {
    let mut liquid = Liquid::new(
        vec![Particle {
            position: [0.5, -0.1, 0.],
            velocity: [0., -1., 0.],
            mass: 1e-30,
            material: 0,
        }],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    liquid
        .configure_transport(
            vec![LiquidField {
                temperature: 300.,
                concentration: 0.,
            }],
            vec![TransportMaterial::default()],
        )
        .unwrap();
    liquid
        .configure_species(vec!["aqueous".into(), "gel".into()], vec![vec![1., 0.]])
        .unwrap();
    let mut film = mixed_film();
    let before_liquid = format!("{liquid:?}");
    let before_film = format!("{film:?}");
    assert!(
        liquid
            .capture_surface_mixture(&[[0.5, 0.1, 0.]], &mut film)
            .is_err(),
        "unresolved deposit must not remove the incident particle"
    );
    assert_eq!(format!("{liquid:?}"), before_liquid);
    assert_eq!(format!("{film:?}"), before_film);
}
#[test]
fn heterogeneous_capture_preserves_component_mass_and_external_energy_ledgers() {
    let mut l = mixed_fluid();
    let mut f = mixed_film();
    let initial_fluid = l.species_totals().unwrap().unwrap();
    let initial_film = f.component_masses().unwrap();
    let heat = l.transport_totals().unwrap().unwrap().0;
    let report = l.capture_surface_mixture(&starts(), &mut f).unwrap();
    assert_eq!(report.capture.particles, 2);
    assert!((report.component_masses[0] - 0.0185).abs() < 1e-15);
    assert!((report.component_masses[1] - 0.0115).abs() < 1e-15);
    assert_eq!(l.species_fractions().unwrap(), &[vec![0.6, 0.4]]);
    assert!((report.capture.absorbed.mass - 0.03).abs() < 1e-15);
    assert!((report.capture.absorbed.kinetic_energy - 0.125).abs() < 1e-15);
    assert!((report.capture.absorbed.momentum[1] + 0.08).abs() < 1e-15);
    assert!(
        (l.transport_totals().unwrap().unwrap().0
            + report.capture.absorbed.thermal_energy.unwrap()
            - heat)
            .abs()
            < 1e-8
    );
    f.configure_viscosities(Some(vec![0.001, 1.0])).unwrap();
    f.step_with_surface_shear(0.1, [0.0; 3], &[[-0.2, 0.0, 0.0]; 2], 0.001)
        .unwrap();
    f.diffuse(0.1, 0.1, 0.001).unwrap();
    let fluid = l.species_totals().unwrap().unwrap();
    let surface = f.component_masses().unwrap();
    for k in 0..2 {
        assert!((initial_fluid[k] + initial_film[k] - fluid[k] - surface[k]).abs() < 1e-14);
    }
    assert!((f.film().total_mass() - 0.04).abs() < 1e-14);
}
#[test]
fn mixed_capture_late_path_schema_and_batch_errors_preserve_both_states() {
    let mut l = mixed_fluid();
    let mut f = mixed_film();
    let before = l.clone();
    let heights = f.film().thickness();
    let components = f.component_masses().unwrap();
    let mut bad = starts();
    bad[2][0] = f64::NAN;
    assert!(l.capture_surface_mixture(&bad, &mut f).is_err());
    assert_eq!(l, before);
    assert_eq!(f.film().thickness(), heights);
    assert_eq!(f.component_masses().unwrap(), components);
    assert!(
        f.deposit_batch(&[(0, 0.0001, vec![0.5, 0.5]), (1, 0.0001, vec![0.1, 0.1])])
            .is_err()
    );
    assert!(
        f.deposit_batch(&[(0, 0.0001, vec![0.5, 0.5]), (99, 0.0001, vec![1.0, 0.0])])
            .is_err()
    );
    assert_eq!(f.film().thickness(), heights);
    assert_eq!(f.component_masses().unwrap(), components);
    let mut incompatible = physics::surface_film::FilmMixture::new(
        film(),
        vec!["gel".into(), "aqueous".into()],
        vec![vec![1.0, 0.0]; 2],
    )
    .unwrap();
    assert!(
        l.capture_surface_mixture(&starts(), &mut incompatible)
            .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(incompatible.film().total_mass(), 0.0);
    let mut different_density = film();
    different_density
        .set_material(FilmMaterial {
            density: 900.0,
            ..different_density.material()
        })
        .unwrap();
    let mut different_density = physics::surface_film::FilmMixture::new(
        different_density,
        vec!["aqueous".into(), "gel".into()],
        vec![vec![1.0, 0.0]; 2],
    )
    .unwrap();
    assert!(
        l.capture_surface_mixture(&starts(), &mut different_density)
            .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(different_density.film().total_mass(), 0.0);
}

fn depositing_fluid(max_particles: usize) -> Liquid {
    let mut l = Liquid::new(
        vec![
            Particle {
                position: [-0.25, -0.1, 0.0],
                velocity: [0.01, -0.1, 0.0],
                mass: 0.001,
                material: 0,
            },
            Particle {
                position: [0.25, -0.1, 0.0],
                velocity: [0.0, -2.0, 0.0],
                mass: 0.001,
                material: 0,
            },
            Particle {
                position: [0.0, -0.1, 0.25],
                velocity: [0.0, -3.0, 0.0],
                mass: 0.001,
                material: 0,
            },
        ],
        vec![Material::WATER],
        Config {
            max_particles,
            ..Config::default()
        },
    )
    .unwrap();
    l.configure_transport(
        (0..3)
            .map(|i| LiquidField {
                temperature: 300.0 + i as f64 * 10.0,
                concentration: 0.0,
            })
            .collect(),
        vec![TransportMaterial::default()],
    )
    .unwrap();
    l.configure_species(
        vec!["aqueous".into(), "gel".into()],
        vec![vec![0.1, 0.9], vec![0.8, 0.2], vec![0.5, 0.5]],
    )
    .unwrap();
    l
}
fn deposition_model() -> physics::liquid::DepositingImpact {
    physics::liquid::DepositingImpact {
        capture_speed: 0.5,
        spray: physics::liquid::ImpactSpray {
            rebound: physics::liquid::FilmRebound {
                restitution: 0.5,
                friction: 0.0,
            },
            children: 8,
            position_radius: 0.005,
            surface_tension: 0.072,
            fragmentation_fraction: 0.8,
        },
    }
}
#[test]
fn one_step_deposits_slow_mixture_and_sprays_fast_impacts_with_complete_ledgers() {
    let mut l = depositing_fluid(100);
    let original = l.clone();
    let mut f = mixed_film();
    let film_mass = f.component_masses().unwrap();
    let previous: Vec<_> = l
        .particles()
        .iter()
        .map(|p| [p.position[0], 0.1, p.position[2]])
        .collect();
    let report = l
        .depositing_impact_surface_mixture(&previous, &mut f, deposition_model())
        .unwrap();
    assert_eq!(report.deposition.capture.particles, 1);
    assert_eq!(report.spray.impacts, 2);
    assert_eq!(report.spray.fragmented_particles, 2);
    assert_eq!(l.particles().len(), 16);
    for k in 0..2 {
        let initial = original.species_totals().unwrap().unwrap()[k] + film_mass[k];
        let final_mass = l.species_totals().unwrap().unwrap()[k] + f.component_masses().unwrap()[k];
        assert!((initial - final_mass).abs() < 1e-15);
    }
    let ke = |l: &Liquid| {
        l.particles()
            .iter()
            .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>()
    };
    assert!(
        (ke(&l)
            + report.spray.substrate_heat
            + report.spray.created_surface_energy
            + report.deposition.capture.absorbed.kinetic_energy
            - ke(&original))
        .abs()
            < 1e-15
    );
    assert!(
        (l.transport_totals().unwrap().unwrap().0
            + report.deposition.capture.absorbed.thermal_energy.unwrap()
            - original.transport_totals().unwrap().unwrap().0)
            .abs()
            < 1e-9
    );
    for k in 0..3 {
        let before: f64 = original
            .particles()
            .iter()
            .map(|p| p.mass * p.velocity[k])
            .sum();
        let after: f64 = l.particles().iter().map(|p| p.mass * p.velocity[k]).sum();
        assert!(
            (after
                + report.spray.substrate_impulse[k]
                + report.deposition.capture.absorbed.momentum[k]
                - before)
                .abs()
                < 1e-15
        );
    }
    // Descending original impact order: faster parent 2, then parent 1.
    assert_eq!(
        l.species_fractions().unwrap()[0],
        original.species_fractions().unwrap()[2]
    );
    assert_eq!(
        l.species_fractions().unwrap()[8],
        original.species_fractions().unwrap()[1]
    );
}
#[test]
fn late_spray_failure_restores_candidate_deposition_and_both_complete_states() {
    let mut l = depositing_fluid(10);
    let original = l.clone();
    let mut f = mixed_film();
    let heights = f.film().thickness();
    let masses = f.component_masses().unwrap();
    let previous: Vec<_> = l
        .particles()
        .iter()
        .map(|p| [p.position[0], 0.1, p.position[2]])
        .collect();
    assert!(
        l.depositing_impact_surface_mixture(&previous, &mut f, deposition_model())
            .is_err()
    );
    assert_eq!(l, original);
    assert_eq!(f.film().thickness(), heights);
    assert_eq!(f.component_masses().unwrap(), masses);
}

#[test]
fn invalid_deposition_controls_roll_back_even_when_all_particles_would_stick() {
    let mut l = depositing_fluid(100);
    let original = l.clone();
    let mut f = mixed_film();
    let heights = f.film().thickness();
    let masses = f.component_masses().unwrap();
    let previous: Vec<_> = l
        .particles()
        .iter()
        .map(|p| [p.position[0], 0.1, p.position[2]])
        .collect();
    for speed in [-1.0, f64::NAN, f64::INFINITY] {
        let mut model = deposition_model();
        model.capture_speed = speed;
        assert!(
            l.depositing_impact_surface_mixture(&previous, &mut f, model)
                .is_err()
        );
    }
    let mut model = deposition_model();
    model.capture_speed = 100.0;
    model.spray.children = 1;
    assert!(
        l.depositing_impact_surface_mixture(&previous, &mut f, model)
            .is_err()
    );
    assert_eq!(l, original);
    assert_eq!(f.film().thickness(), heights);
    assert_eq!(f.component_masses().unwrap(), masses);
}
