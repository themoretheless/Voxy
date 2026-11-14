use physics::liquid::{
    Config, FilmRebound, ImpactSpray, Liquid, LiquidField, Material, Particle, TransportMaterial,
};
use physics::surface_film::{Material as FilmMaterial, SurfaceFilm};
fn sheet() -> SurfaceFilm {
    SurfaceFilm::new(
        &[[-1.0, 0.0, -1.0], [1.0, 0.0, -1.0], [0.0, 0.0, 1.0]],
        vec![[0, 1, 2]],
        FilmMaterial::default(),
    )
    .unwrap()
}
fn model() -> ImpactSpray {
    ImpactSpray {
        rebound: FilmRebound {
            restitution: 0.5,
            friction: 0.0,
        },
        children: 8,
        position_radius: 0.005,
        surface_tension: 0.072,
        fragmentation_fraction: 0.8,
    }
}
fn fluid(speeds: &[f64], max_particles: usize) -> Liquid {
    Liquid::new(
        speeds
            .iter()
            .enumerate()
            .map(|(i, speed)| Particle {
                position: [(i as f64 - 0.5) * 0.2, -0.001, 0.0],
                velocity: [0.0, -speed, 0.0],
                mass: 0.001,
                material: 0,
            })
            .collect(),
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            max_particles,
            ..Config::default()
        },
    )
    .unwrap()
}
fn paths(l: &Liquid) -> Vec<[f64; 3]> {
    l.particles()
        .iter()
        .map(|p| [p.position[0], 0.001, p.position[2]])
        .collect()
}
fn energy(l: &Liquid) -> f64 {
    l.particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum()
}
fn momentum(l: &Liquid) -> [f64; 3] {
    std::array::from_fn(|k| l.particles().iter().map(|p| p.mass * p.velocity[k]).sum())
}
#[test]
fn impact_energy_selects_fragmentation_and_preserves_all_inventories() {
    let mut l = fluid(&[0.01, 2.0], 100);
    l.configure_transport(
        vec![
            LiquidField {
                temperature: 300.0,
                concentration: 0.0,
            },
            LiquidField {
                temperature: 310.0,
                concentration: 0.0,
            },
        ],
        vec![TransportMaterial::default()],
    )
    .unwrap();
    l.configure_species(
        vec!["a".into(), "b".into()],
        vec![vec![0.1, 0.9], vec![0.8, 0.2]],
    )
    .unwrap();
    let before_energy = energy(&l);
    let before_momentum = momentum(&l);
    let heat = l.transport_totals().unwrap().unwrap().0;
    let components = l.species_totals().unwrap().unwrap();
    let report = l
        .impact_spray_surface_film(&paths(&l), &sheet(), model())
        .unwrap();
    assert_eq!(report.impacts, 2);
    assert_eq!(report.fragmented_particles, 1);
    assert_eq!(report.fragments_created, 8);
    assert_eq!(l.particles().len(), 9);
    assert_eq!(l.particles()[0].velocity, [0.0, 0.005, 0.0]);
    assert!(
        (energy(&l) + report.substrate_heat + report.created_surface_energy - before_energy).abs()
            < 1e-15
    );
    assert!(
        (report.added_fragment_kinetic_energy + report.created_surface_energy - 0.0012).abs()
            < 1e-15
    );
    let after = momentum(&l);
    for k in 0..3 {
        assert!((after[k] + report.substrate_impulse[k] - before_momentum[k]).abs() < 1e-15);
    }
    assert!((l.transport_totals().unwrap().unwrap().0 - heat).abs() < 1e-9);
    for (a, b) in components
        .into_iter()
        .zip(l.species_totals().unwrap().unwrap())
    {
        assert!((a - b).abs() < 1e-15);
    }
}
#[test]
fn late_fragment_budget_failure_rolls_back_prior_reflections_and_fragmentation() {
    let mut l = fluid(&[2.0, 2.0], 10);
    let before = l.clone();
    assert!(
        l.impact_spray_surface_film(&paths(&l), &sheet(), model())
            .is_err()
    );
    assert_eq!(l, before);
}
#[test]
fn zero_allocation_keeps_rebound_and_zero_surface_tension_allows_positive_budget() {
    let mut l = fluid(&[2.0], 100);
    let before = energy(&l);
    let mut m = model();
    m.fragmentation_fraction = 0.0;
    let report = l
        .impact_spray_surface_film(&paths(&l), &sheet(), m)
        .unwrap();
    assert_eq!(report.fragmented_particles, 0);
    assert!((energy(&l) + report.substrate_heat - before).abs() < 1e-15);
    let mut l = fluid(&[0.01], 100);
    m.fragmentation_fraction = 1.0;
    m.surface_tension = 0.0;
    let before = energy(&l);
    let report = l
        .impact_spray_surface_film(&paths(&l), &sheet(), m)
        .unwrap();
    assert_eq!(report.fragmented_particles, 1);
    assert_eq!(report.created_surface_energy, 0.0);
    assert_eq!(report.substrate_heat, 0.0);
    assert!((energy(&l) - before).abs() < 1e-18);
}
#[test]
fn misses_and_invalid_late_paths_leave_original_state_intact() {
    let mut l = fluid(&[2.0, 2.0], 100);
    let before = l.clone();
    let mut previous = paths(&l);
    previous[1][0] = f64::NAN;
    assert!(
        l.impact_spray_surface_film(&previous, &sheet(), model())
            .is_err()
    );
    assert_eq!(l, before);
    let previous: Vec<_> = l.particles().iter().map(|p| p.position).collect();
    let report = l
        .impact_spray_surface_film(&previous, &sheet(), model())
        .unwrap();
    assert_eq!(report.impacts, 0);
    assert_eq!(l, before);
}

#[test]
fn multiple_fragmenting_impacts_keep_each_parents_composition_heat_and_center() {
    let mut l = fluid(&[2.0, 3.0], 100);
    l.configure_transport(
        vec![
            LiquidField {
                temperature: 300.0,
                concentration: 0.0,
            },
            LiquidField {
                temperature: 320.0,
                concentration: 0.0,
            },
        ],
        vec![TransportMaterial::default()],
    )
    .unwrap();
    l.configure_species(
        vec!["a".into(), "b".into()],
        vec![vec![0.1, 0.9], vec![0.8, 0.2]],
    )
    .unwrap();
    let before = l.clone();
    let report = l
        .impact_spray_surface_film(&paths(&l), &sheet(), model())
        .unwrap();
    assert_eq!(report.fragmented_particles, 2);
    assert_eq!(report.fragments_created, 16);
    assert_eq!(l.particles().len(), 16);
    for (group, parent) in [(0, 1), (1, 0)] {
        let particles = &l.particles()[group * 8..group * 8 + 8];
        let mass: f64 = particles.iter().map(|p| p.mass).sum();
        assert!((mass - before.particles()[parent].mass).abs() < 1e-17);
        for k in [0, 2] {
            let center: f64 = particles
                .iter()
                .map(|p| p.mass * p.position[k])
                .sum::<f64>()
                / mass;
            assert!((center - before.particles()[parent].position[k]).abs() < 1e-14);
        }
        for i in group * 8..group * 8 + 8 {
            assert_eq!(
                l.species_fractions().unwrap()[i],
                before.species_fractions().unwrap()[parent]
            );
            assert_eq!(l.fields().unwrap()[i], before.fields().unwrap()[parent]);
        }
    }
    assert!(
        (energy(&l) + report.substrate_heat + report.created_surface_energy - energy(&before))
            .abs()
            < 1e-15
    );
    let p = momentum(&l);
    let initial = momentum(&before);
    for k in 0..3 {
        assert!((p[k] + report.substrate_impulse[k] - initial[k]).abs() < 1e-15);
    }
}
