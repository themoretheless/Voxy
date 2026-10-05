use physics::liquid::{
    Config, DepositingImpact, FilmImpactControl, FilmRebound, ImpactSpray, Liquid, LiquidField,
    Material, Particle, TransportMaterial,
};
use physics::surface_film::{FilmMixture, Material as FilmMaterial, SurfaceFilm};
fn film(corridor: bool) -> FilmMixture {
    let points: Vec<_> = if corridor {
        [0.0, 1.0]
            .into_iter()
            .flat_map(|x| {
                [
                    [x, -2.0, -2.0],
                    [x, 2.0, -2.0],
                    [x, 2.0, 2.0],
                    [x, -2.0, 2.0],
                ]
            })
            .collect()
    } else {
        vec![
            [-5.0, 0.0, -5.0],
            [5.0, 0.0, -5.0],
            [5.0, 0.0, 5.0],
            [-5.0, 0.0, 5.0],
        ]
    };
    let triangles = if corridor {
        vec![[0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7]]
    } else {
        vec![[0, 1, 2], [0, 2, 3]]
    };
    let count = triangles.len();
    FilmMixture::new(
        SurfaceFilm::new(&points, triangles, FilmMaterial::default()).unwrap(),
        vec!["a".into(), "b".into()],
        vec![vec![0.5, 0.5]; count],
    )
    .unwrap()
}
fn liquid(end: [f64; 3], velocity: [f64; 3], max_particles: usize) -> Liquid {
    liquid_with_viscosity(end, velocity, max_particles, Material::WATER.viscosity)
}
fn liquid_with_viscosity(
    end: [f64; 3],
    velocity: [f64; 3],
    max_particles: usize,
    viscosity: f64,
) -> Liquid {
    let mut liquid = Liquid::new(
        vec![Particle {
            position: end,
            velocity,
            mass: 0.001,
            material: 0,
        }],
        vec![Material {
            viscosity,
            ..Material::WATER
        }],
        Config {
            max_particles,
            ..Config::default()
        },
    )
    .unwrap();
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
    liquid
}
fn model(fraction: f64) -> DepositingImpact {
    DepositingImpact {
        capture_speed: 21.0,
        spray: ImpactSpray {
            rebound: FilmRebound {
                restitution: 0.5,
                friction: 0.0,
            },
            children: 2,
            position_radius: 0.0001,
            surface_tension: 0.072,
            fragmentation_fraction: fraction,
        },
    }
}
fn control() -> FilmImpactControl {
    FilmImpactControl {
        dt: 0.1,
        max_contacts_per_lineage: 8,
        max_events: 32,
    }
}
fn ke(l: &Liquid) -> f64 {
    l.particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum()
}
fn ledgers(
    before: &Liquid,
    after: &Liquid,
    film: &FilmMixture,
    report: &physics::liquid::FilmImpactEventReport,
) {
    let r = &report.impact;
    assert_eq!(
        report.deposited_thermal_energy.len(),
        r.deposition.capture.particles
    );
    let carried = report
        .deposited_thermal_energy
        .iter()
        .map(|(cell, energy)| {
            assert!(*cell < film.fractions().len());
            energy.unwrap()
        })
        .sum::<f64>();
    assert!((carried - r.deposition.capture.absorbed.thermal_energy.unwrap()).abs() < 1e-9);

    assert!((after.mass() + film.film().total_mass() - before.mass()).abs() < 1e-15);
    for (k, m) in after.species_totals().unwrap().unwrap().iter().enumerate() {
        assert!(
            (m + film.component_masses().unwrap()[k]
                - before.species_totals().unwrap().unwrap()[k])
                .abs()
                < 1e-15
        );
    }
    assert!(
        (ke(after)
            + r.spray.substrate_heat
            + r.spray.created_surface_energy
            + r.deposition.capture.absorbed.kinetic_energy
            - ke(before))
        .abs()
            < 1e-12
    );
    assert!(
        (after.transport_totals().unwrap().unwrap().0
            + r.deposition.capture.absorbed.thermal_energy.unwrap()
            - before.transport_totals().unwrap().unwrap().0)
            .abs()
            < 1e-9
    );
    for k in 0..3 {
        let p = |l: &Liquid| {
            l.particles()
                .iter()
                .map(|p| p.mass * p.velocity[k])
                .sum::<f64>()
        };
        assert!(
            (p(after) + r.spray.substrate_impulse[k] + r.deposition.capture.absorbed.momentum[k]
                - p(before))
            .abs()
                < 1e-12
        );
    }
}
#[test]
fn parent_rebounds_twice_then_deposits_within_the_same_interval() {
    let mut liquid = liquid([8.5, 0.0, 0.0], [80.0, 0.0, 0.0], 100);
    let original = liquid.clone();
    let mut film = film(true);
    let report = liquid
        .depositing_impact_spheres_surface_mixture_events(
            &[[0.5, 0.0, 0.0]],
            &mut film,
            model(0.0),
            &[0.01],
            control(),
        )
        .unwrap();
    assert_eq!(report.events, 3);
    assert_eq!(report.impact.spray.impacts, 2);
    assert_eq!(report.impact.deposition.capture.particles, 1);
    assert!(liquid.particles().is_empty());
    assert!((report.impact.spray.substrate_heat - 3.0).abs() < 1e-12);
    assert!((report.impact.deposition.capture.absorbed.kinetic_energy - 0.2).abs() < 1e-12);
    ledgers(&original, &liquid, &film, &report);
}
#[test]
fn daughter_and_granddaughter_impacts_keep_inventory_and_energy_balances() {
    let mut liquid = liquid([8.5, 0.0, 0.0], [80.0, 0.0, 0.0], 100);
    let original = liquid.clone();
    let mut film = film(true);
    let report = liquid
        .depositing_impact_spheres_surface_mixture_events(
            &[[0.5, 0.0, 0.0]],
            &mut film,
            model(0.0001),
            &[0.01],
            control(),
        )
        .unwrap();
    assert_eq!(report.events, 7);
    assert_eq!(report.impact.spray.impacts, 3);
    assert_eq!(report.impact.spray.fragmented_particles, 3);
    assert_eq!(report.impact.spray.fragments_created, 6);
    assert_eq!(report.impact.deposition.capture.particles, 4);
    assert!(liquid.particles().is_empty());
    ledgers(&original, &liquid, &film, &report);
}
#[test]
fn fragment_velocity_advances_only_after_birth_for_remaining_time() {
    let mut liquid = liquid([0.0, -0.1, 0.0], [0.0, -2.0, 0.0], 100);
    let original = liquid.clone();
    let mut film = film(false);
    let radius = liquid.equivalent_sphere_radii().unwrap()[0];
    let mut model = model(0.8);
    model.capture_speed = 0.0;
    model.spray.children = 4;
    let report = liquid
        .depositing_impact_spheres_surface_mixture_events(
            &[[0.0, 0.1, 0.0]],
            &mut film,
            model,
            &[radius],
            control(),
        )
        .unwrap();
    assert_eq!(report.events, 1);
    assert_eq!(liquid.particles().len(), 4);
    let remaining = 0.1 * (1.0 - (0.1 - radius) / 0.2);
    let child_radius = radius / 4.0_f64.cbrt();
    let ring = child_radius / (std::f64::consts::PI / 4.0).sin() * (1.0 + 1e-12);
    let speed = (2.0 * report.impact.spray.added_fragment_kinetic_energy / 0.001).sqrt();
    let expected = ring + speed * remaining;
    for p in liquid.particles() {
        assert!(
            ((p.position[0] * p.position[0] + p.position[2] * p.position[2]).sqrt() - expected)
                .abs()
                < 1e-11
        );
        assert!((p.position[1] - (radius + remaining)).abs() < 1e-11);
    }
    ledgers(&original, &liquid, &film, &report);
}
#[test]
fn late_event_lineage_and_particle_budgets_restore_both_complete_states() {
    for failure in 0..3 {
        let mut liquid = liquid(
            [8.5, 0.0, 0.0],
            [80.0, 0.0, 0.0],
            if failure == 2 { 3 } else { 100 },
        );
        let original = liquid.clone();
        let mut film = film(true);
        let masses = film.component_masses().unwrap();
        let mut controls = control();
        if failure == 0 {
            controls.max_events = 6;
        } else if failure == 1 {
            controls.max_contacts_per_lineage = 2;
        }
        let expected = [
            "film impact event budget",
            "film impact lineage contact budget",
            "film event fragmentation failed",
        ][failure];
        assert_eq!(
            liquid
                .depositing_impact_spheres_surface_mixture_events(
                    &[[0.5, 0.0, 0.0]],
                    &mut film,
                    model(0.0001),
                    &[0.01],
                    controls
                )
                .unwrap_err(),
            expected
        );
        assert_eq!(liquid, original);
        assert_eq!(film.component_masses().unwrap(), masses);
        assert_eq!(film.film().total_mass(), 0.0);
    }
}

#[test]
fn chronological_index_changes_preserve_distinct_parent_composition_and_temperature() {
    let mut liquid = Liquid::new(
        vec![
            Particle {
                position: [-0.2, -0.1, 0.0],
                velocity: [0.0, -4.0, 0.0],
                mass: 0.001,
                material: 0,
            },
            Particle {
                position: [0.2, -0.9, 0.0],
                velocity: [0.0, -10.0, 0.0],
                mass: 0.001,
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
                    concentration: 0.0,
                },
                LiquidField {
                    temperature: 350.0,
                    concentration: 0.0,
                },
            ],
            vec![TransportMaterial::default()],
        )
        .unwrap();
    liquid
        .configure_species(
            vec!["a".into(), "b".into()],
            vec![vec![0.9, 0.1], vec![0.2, 0.8]],
        )
        .unwrap();
    let original = liquid.clone();
    let mut film = film(false);
    let mut model = model(0.1);
    model.capture_speed = 5.0;
    let radii = liquid.equivalent_sphere_radii().unwrap();
    let report = liquid
        .depositing_impact_spheres_surface_mixture_events(
            &[[-0.2, 0.3, 0.0], [0.2, 0.1, 0.0]],
            &mut film,
            model,
            &radii,
            control(),
        )
        .unwrap();
    assert_eq!(report.events, 2);
    assert_eq!(report.impact.deposition.capture.particles, 1);
    assert_eq!(report.impact.spray.fragments_created, 2);
    assert_eq!(liquid.particles().len(), 2);
    for row in liquid.species_fractions().unwrap() {
        assert_eq!(row, &vec![0.2, 0.8]);
    }
    for field in liquid.fields().unwrap() {
        assert_eq!(field.temperature, 350.0);
    }
    assert!((film.component_masses().unwrap()[0] - 0.0009).abs() < 1e-15);
    assert!((film.component_masses().unwrap()[1] - 0.0001).abs() < 1e-15);
    ledgers(&original, &liquid, &film, &report);
}

#[test]
fn onset_gate_changes_fragmentation_without_losing_mass_or_energy() {
    for (threshold, expected_events, expected_captures) in [(1e12, 3, 1), (1.0, 7, 4)] {
        let mut liquid = liquid([8.5, 0.0, 0.0], [80.0, 0.0, 0.0], 100);
        let original = liquid.clone();
        let mut film = film(true);
        let report = liquid
            .depositing_impact_spheres_surface_mixture_events_with_onset(
                &[[0.5, 0.0, 0.0]],
                &mut film,
                model(0.0001),
                &[0.01],
                control(),
                physics::liquid::DryWallSplashOnset {
                    critical_parameter: threshold,
                },
            )
            .unwrap();
        assert_eq!(report.events, expected_events);
        assert_eq!(
            report.impact.deposition.capture.particles,
            expected_captures
        );
        ledgers(&original, &liquid, &film, &report);
    }
}
#[test]
fn invalid_onset_rolls_back_liquid_and_film() {
    let mut liquid = liquid([8.5, 0.0, 0.0], [80.0, 0.0, 0.0], 100);
    let original = liquid.clone();
    let mut film = film(true);
    let film_before = film.component_masses().unwrap();
    assert!(
        liquid
            .depositing_impact_spheres_surface_mixture_events_with_onset(
                &[[0.5, 0.0, 0.0]],
                &mut film,
                model(0.0001),
                &[0.01],
                control(),
                physics::liquid::DryWallSplashOnset {
                    critical_parameter: f64::NAN
                },
            )
            .is_err()
    );
    assert_eq!(liquid, original);
    assert_eq!(film.component_masses().unwrap(), film_before);
}

#[test]
fn viscous_material_suppresses_splash_for_identical_impact_and_energy_budget() {
    let diameter = 2.0 * (3.0 * 0.001 / (4.0 * std::f64::consts::PI * 1000.0)).cbrt();
    let water = physics::liquid::ImpactNumbers::new(
        1000.0,
        Material::WATER.viscosity,
        0.072,
        diameter,
        80.0,
    )
    .unwrap();
    let onset = physics::liquid::DryWallSplashOnset {
        critical_parameter: water.splash_parameter * 0.3,
    };
    for (viscosity, captures) in [
        (Material::WATER.viscosity, 4),
        (Material::WATER.viscosity * 256.0, 1),
    ] {
        let mut liquid = liquid_with_viscosity([8.5, 0.0, 0.0], [80.0, 0.0, 0.0], 100, viscosity);
        let original = liquid.clone();
        let mut film = film(true);
        let report = liquid
            .depositing_impact_spheres_surface_mixture_events_with_onset(
                &[[0.5, 0.0, 0.0]],
                &mut film,
                model(0.0001),
                &[0.01],
                control(),
                onset,
            )
            .unwrap();
        assert_eq!(report.impact.deposition.capture.particles, captures);
        ledgers(&original, &liquid, &film, &report);
    }
}

#[test]
fn tangential_speed_does_not_trigger_normal_splash_onset() {
    let mut liquid = liquid([4.0, -0.1, 0.0], [80.0, -2.0, 0.0], 100);
    let radius = liquid.equivalent_sphere_radii().unwrap()[0];
    let n = physics::liquid::ImpactNumbers::new(
        1000.0,
        Material::WATER.viscosity,
        0.072,
        2.0 * radius,
        2.0,
    )
    .unwrap();
    let original = liquid.clone();
    let mut film = film(false);
    let mut response = model(0.8);
    response.capture_speed = 0.0;
    let report = liquid
        .depositing_impact_spheres_surface_mixture_events_with_onset(
            &[[-4.0, 0.1, 0.0]],
            &mut film,
            response,
            &[radius],
            control(),
            physics::liquid::DryWallSplashOnset {
                critical_parameter: n.splash_parameter * 2.0,
            },
        )
        .unwrap();
    assert_eq!(report.events, 1);
    assert_eq!(report.impact.spray.fragmented_particles, 0);
    assert_eq!(liquid.particles().len(), 1);
    assert!((liquid.particles()[0].velocity[0] - 80.0).abs() < 1e-12);
    ledgers(&original, &liquid, &film, &report);
}

#[test]
fn unequal_fragment_cascade_deposits_all_mass_and_species() {
    let mut liquid = liquid([8.5, 0.0, 0.0], [80.0, 0.0, 0.0], 100);
    let original = liquid.clone();
    let mut film = film(true);
    let report = liquid
        .depositing_impact_spheres_surface_mixture_events_with_mass_fractions(
            &[[0.5, 0.0, 0.0]],
            &mut film,
            model(0.0001),
            &[0.01],
            control(),
            &[0.2, 0.8],
            None,
        )
        .unwrap();
    assert_eq!(report.events, 7);
    assert_eq!(report.impact.deposition.capture.particles, 4);
    assert_eq!(report.impact.spray.fragmented_particles, 3);
    assert!(liquid.particles().is_empty());
    ledgers(&original, &liquid, &film, &report);
}
