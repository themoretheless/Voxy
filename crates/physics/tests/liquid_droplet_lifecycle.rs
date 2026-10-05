use physics::liquid::{
    Config, DepositingImpact, DropletCoalescenceControl, DropletLifecycle, FilmImpactControl,
    FilmImpactEventReport, FilmRebound, ImpactSpray, Liquid, LiquidField, Material, Particle,
    TransportMaterial,
};
use physics::surface_film::{FilmMixture, Material as FilmMaterial, SurfaceFilm};
fn film() -> FilmMixture {
    FilmMixture::new(
        SurfaceFilm::new(
            &[
                [-2.0, 0.0, -2.0],
                [2.0, 0.0, -2.0],
                [2.0, 0.0, 2.0],
                [-2.0, 0.0, 2.0],
            ],
            vec![[0, 1, 2], [0, 2, 3]],
            FilmMaterial::default(),
        )
        .unwrap(),
        vec!["a".into(), "b".into()],
        vec![vec![0.5, 0.5]; 2],
    )
    .unwrap()
}
fn fluid(previous: &[[f64; 3]], velocity: &[[f64; 3]]) -> Liquid {
    let mut l = Liquid::new(
        previous
            .iter()
            .zip(velocity)
            .map(|(x, v)| Particle {
                position: std::array::from_fn(|k| x[k] + 0.1 * v[k]),
                velocity: *v,
                mass: 1e-6,
                material: 0,
            })
            .collect(),
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    l.configure_transport(
        (0..previous.len())
            .map(|i| LiquidField {
                temperature: 300.0 + 100.0 * i as f64,
                concentration: 0.0,
            })
            .collect(),
        vec![TransportMaterial::default()],
    )
    .unwrap();
    l.configure_species(
        vec!["a".into(), "b".into()],
        (0..previous.len())
            .map(|i| {
                if i % 2 == 0 {
                    vec![0.9, 0.1]
                } else {
                    vec![0.2, 0.8]
                }
            })
            .collect(),
    )
    .unwrap();
    l
}
fn control() -> FilmImpactControl {
    FilmImpactControl {
        dt: 0.1,
        max_contacts_per_lineage: 16,
        max_events: 64,
    }
}
fn lifecycle() -> DropletLifecycle {
    DropletLifecycle {
        coalescence: Some(DropletCoalescenceControl {
            dt: 0.1,
            surface_tension: 0.072,
            maximum_normal_speed: 10.0,
            max_events: 32,
        }),
        ..DropletLifecycle::default()
    }
}
fn model(capture: f64, fraction: f64) -> DepositingImpact {
    DepositingImpact {
        capture_speed: capture,
        spray: ImpactSpray {
            rebound: FilmRebound {
                restitution: 0.5,
                friction: 0.0,
            },
            children: 2,
            position_radius: 1e-5,
            surface_tension: 0.072,
            fragmentation_fraction: fraction,
        },
    }
}
fn kinetic(l: &Liquid) -> f64 {
    l.particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum()
}
fn ledger(before: &Liquid, after: &Liquid, film: &FilmMixture, r: &FilmImpactEventReport) {
    assert!((after.mass() + film.film().total_mass() - before.mass()).abs() < 1e-17);
    for (k, b) in before.species_totals().unwrap().unwrap().iter().enumerate() {
        assert!(
            (after.species_totals().unwrap().unwrap()[k] + film.component_masses().unwrap()[k] - b)
                .abs()
                < 1e-17
        );
    }
    let spray = &r.impact.spray;
    let capture = &r.impact.deposition.capture.absorbed;
    assert!(
        (kinetic(after)
            + spray.substrate_heat
            + spray.created_surface_energy
            + capture.kinetic_energy
            + r.coalescence.unresolved_kinetic_energy
            - kinetic(before))
        .abs()
            < 1e-15
    );
    assert!(
        (after.transport_totals().unwrap().unwrap().0 + capture.thermal_energy.unwrap()
            - before.transport_totals().unwrap().unwrap().0)
            .abs()
            < 1e-10
    );
    for k in 0..3 {
        let p = |l: &Liquid| {
            l.particles()
                .iter()
                .map(|p| p.mass * p.velocity[k])
                .sum::<f64>()
        };
        assert!(
            (p(after) + spray.substrate_impulse[k] + capture.momentum[k] - p(before)).abs() < 1e-16
        );
    }
}
#[test]
fn droplets_merge_then_deposit_in_the_same_interval() {
    let previous = [[-0.05, 0.1, 0.0], [0.05, 0.1, 0.0]];
    let mut l = fluid(&previous, &[[1.0, -2.0, 0.0], [-1.0, -2.0, 0.0]]);
    let original = l.clone();
    let radii = l.equivalent_sphere_radii().unwrap();
    let mut film = film();
    let r = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &previous,
            &mut film,
            model(3.0, 0.0),
            &radii,
            control(),
            &lifecycle(),
        )
        .unwrap();
    assert_eq!(r.events, 2);
    assert_eq!(r.coalescence.events.len(), 1);
    assert_eq!(r.impact.deposition.capture.particles, 1);
    assert!(l.particles().is_empty());
    assert!((r.coalescence.unresolved_kinetic_energy - 1e-6).abs() < 1e-17);
    ledger(&original, &l, &film, &r);
}
#[test]
fn wall_rebound_changes_the_next_droplet_collision() {
    let previous = [[0.0, 0.002, 0.0], [0.0, 0.008, 0.0]];
    let mut l = fluid(&previous, &[[0.0, -0.1, 0.0], [0.0, -0.02, 0.0]]);
    let original = l.clone();
    let radii = l.equivalent_sphere_radii().unwrap();
    let mut film = film();
    let mut response = model(0.0, 0.0);
    response.spray.rebound.restitution = 1.0;
    let r = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &previous,
            &mut film,
            response,
            &radii,
            control(),
            &lifecycle(),
        )
        .unwrap();
    assert_eq!(r.events, 2);
    assert_eq!(r.impact.spray.impacts, 1);
    assert_eq!(r.coalescence.events.len(), 1);
    assert_eq!(l.particles().len(), 1);
    assert!((l.particles()[0].velocity[1] - 0.04).abs() < 1e-14);
    assert!((l.particles()[0].position[1] - (0.007 + radii[0])).abs() < 1e-12);
    ledger(&original, &l, &film, &r);
}
#[test]
fn merged_drop_can_fragment_into_unequal_children_after_wall_contact() {
    let previous = [[-0.05, 0.1, 0.0], [0.05, 0.1, 0.0]];
    let mut l = fluid(&previous, &[[1.0, -2.0, 0.0], [-1.0, -2.0, 0.0]]);
    let original = l.clone();
    let radii = l.equivalent_sphere_radii().unwrap();
    let mut film = film();
    let mut life = lifecycle();
    life.mass_fractions = Some(vec![0.2, 0.8]);
    let r = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &previous,
            &mut film,
            model(0.0, 0.8),
            &radii,
            control(),
            &life,
        )
        .unwrap();
    assert_eq!(r.events, 2);
    assert_eq!(r.coalescence.events.len(), 1);
    assert_eq!(r.impact.spray.fragmented_particles, 1);
    assert_eq!(l.particles().len(), 2);
    assert!((l.particles()[0].mass / 2e-6 - 0.2).abs() < 1e-14);
    assert!(l.particles().iter().all(|p| p.position[1] > 0.01));
    ledger(&original, &l, &film, &r);
}
#[test]
fn late_failure_rolls_back_prepared_deposit_and_coalescence() {
    let previous = [[1.0, 0.002, 0.0], [-0.05, 0.1, 0.0], [0.05, 0.1, 0.0]];
    let mut l = fluid(
        &previous,
        &[[0.0, -0.1, 0.0], [1.0, -2.0, 0.0], [-1.0, -2.0, 0.0]],
    );
    let original = l.clone();
    let radii = l.equivalent_sphere_radii().unwrap();
    let mut film = film();
    let before = film.component_masses().unwrap();
    let mut c = control();
    c.max_events = 2;
    let error = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &previous,
            &mut film,
            model(3.0, 0.0),
            &radii,
            c,
            &lifecycle(),
        )
        .unwrap_err();
    assert_eq!(error, "film impact event budget");
    assert_eq!(l, original);
    assert_eq!(film.component_masses().unwrap(), before);
    assert_eq!(film.film().total_volume(), 0.0);
}
#[test]
fn growing_merged_sphere_cannot_penetrate_the_substrate() {
    let radius = (3.0 * 1e-6 / (4.0 * std::f64::consts::PI * 1000.0)).cbrt();
    let previous = [[-radius, 0.00065, 0.0], [radius, 0.00065, 0.0]];
    let mut l = fluid(&previous, &[[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]]);
    let original = l.clone();
    let radii = l.equivalent_sphere_radii().unwrap();
    let mut film = film();
    let error = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &previous,
            &mut film,
            model(0.0, 0.0),
            &radii,
            control(),
            &lifecycle(),
        )
        .unwrap_err();
    assert_eq!(error, "merged sphere penetrates film mesh");
    assert_eq!(l, original);
    assert_eq!(film.film().total_volume(), 0.0);
}

#[test]
fn simultaneous_wall_and_drop_contact_uses_wall_priority() {
    let radius = (3.0 * 1e-6 / (4.0 * std::f64::consts::PI * 1000.0)).cbrt();
    let previous = [[-radius, radius, 0.0], [radius, radius, 0.0]];
    let mut l = fluid(&previous, &[[1.0, -0.1, 0.0], [-1.0, -0.1, 0.0]]);
    let original = l.clone();
    let radii = l.equivalent_sphere_radii().unwrap();
    let mut film = film();
    let r = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &previous,
            &mut film,
            model(3.0, 0.0),
            &radii,
            control(),
            &lifecycle(),
        )
        .unwrap();
    assert_eq!(r.events, 2);
    assert!(r.coalescence.events.is_empty());
    assert_eq!(r.impact.deposition.capture.particles, 2);
    ledger(&original, &l, &film, &r);
}

#[test]
fn population_mask_keeps_sph_samples_out_of_unified_coalescence() {
    for (marked, captures, merges) in [(false, 2, 0), (true, 1, 1)] {
        let previous = [[-0.05, 0.1, 0.0], [0.05, 0.1, 0.0]];
        let mut l = fluid(&previous, &[[1.0, -2.0, 0.0], [-1.0, -2.0, 0.0]]);
        l.configure_droplet_population(Some(vec![marked; 2]))
            .unwrap();
        let original = l.clone();
        let radii = l.equivalent_sphere_radii().unwrap();
        let mut film = film();
        let r = l
            .depositing_impact_spheres_surface_mixture_lifecycle(
                &previous,
                &mut film,
                model(3.0, 0.0),
                &radii,
                control(),
                &lifecycle(),
            )
            .unwrap();
        assert_eq!(r.coalescence.events.len(), merges);
        assert_eq!(r.impact.deposition.capture.particles, captures);
        assert_eq!(l.droplet_population().unwrap().len(), 0);
        ledger(&original, &l, &film, &r);
    }
}

#[test]
fn growth_contact_can_capture_slow_merged_droplets_and_rolls_back_on_budget() {
    let radius = (3.0 * 1e-6 / (4.0 * std::f64::consts::PI * 1000.0)).cbrt();
    let previous = [[-radius, 0.00065, 0.0], [radius, 0.00065, 0.0]];
    for budget in [1, 64] {
        let mut l = fluid(&previous, &[[1.0, 0.1, 0.0], [-1.0, 0.1, 0.0]]);
        l.configure_droplet_population(Some(vec![true, true]))
            .unwrap();
        let original = l.clone();
        let radii = l.equivalent_sphere_radii().unwrap();
        let mut film = film();
        let mut life = lifecycle();
        life.capture_growth_contact = true;
        let mut c = control();
        c.max_events = budget;
        let result = l.depositing_impact_spheres_surface_mixture_lifecycle(
            &previous,
            &mut film,
            model(3.0, 0.0),
            &radii,
            c,
            &life,
        );
        if budget == 1 {
            assert_eq!(result.unwrap_err(), "film impact event budget");
            assert_eq!(l, original);
            assert_eq!(film.film().total_mass(), 0.0);
        } else {
            let r = result.unwrap();
            assert_eq!(r.events, 2);
            assert_eq!(r.coalescence.events.len(), 1);
            assert_eq!(r.impact.deposition.capture.particles, 1);
            assert!(l.particles().is_empty());
            ledger(&original, &l, &film, &r);
        }
    }
}
#[test]
fn growth_capture_does_not_accept_a_fast_merged_drop() {
    let radius = (3.0 * 1e-6 / (4.0 * std::f64::consts::PI * 1000.0)).cbrt();
    let previous = [[-radius, 0.00065, 0.0], [radius, 0.00065, 0.0]];
    let mut l = fluid(&previous, &[[1.0, 5.0, 0.0], [-1.0, 5.0, 0.0]]);
    let original = l.clone();
    let radii = l.equivalent_sphere_radii().unwrap();
    let mut film = film();
    let mut life = lifecycle();
    life.capture_growth_contact = true;
    assert_eq!(
        l.depositing_impact_spheres_surface_mixture_lifecycle(
            &previous,
            &mut film,
            model(3.0, 0.0),
            &radii,
            control(),
            &life
        )
        .unwrap_err(),
        "merged sphere penetrates film mesh"
    );
    assert_eq!(l, original);
    assert_eq!(film.film().total_mass(), 0.0);
}

fn ballistic_fixture(
    start: [f64; 3],
    velocity: [f64; 3],
    acceleration: [f64; 3],
) -> (Liquid, DropletLifecycle) {
    let l = fluid(&[start], &[velocity]);
    let mut particles = l.particles().to_vec();
    particles[0].position =
        std::array::from_fn(|k| start[k] + 0.1 * velocity[k] + 0.005 * acceleration[k]);
    particles[0].velocity = std::array::from_fn(|k| velocity[k] + 0.1 * acceleration[k]);
    let fixture_start =
        std::array::from_fn(|k| particles[0].position[k] - 0.1 * particles[0].velocity[k]);
    let mut endpoint = fluid(&[fixture_start], &[particles[0].velocity]);
    endpoint
        .configure_droplet_population(Some(vec![true]))
        .unwrap();
    let controls = DropletLifecycle {
        flight: Some(physics::liquid::DropletFlight {
            initial_velocities: vec![velocity],
            acceleration,
            max_feature_checks: 1000,
        }),
        ..DropletLifecycle::default()
    };
    (endpoint, controls)
}
#[test]
fn ballistic_capture_uses_contact_velocity_and_only_precontact_external_work() {
    let start = [0.0, 0.02, 0.0];
    let (mut l, lifecycle) = ballistic_fixture(start, [0.0; 3], [0.0, -10.0, 0.0]);
    let mut f = film();
    let r = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &[start],
            &mut f,
            model(10.0, 0.0),
            &[0.001],
            control(),
            &lifecycle,
        )
        .unwrap();
    let t = (2.0_f64 * 0.019 / 10.0).sqrt();
    assert_eq!(r.impact.deposition.capture.particles, 1);
    assert!((r.impact.deposition.capture.absorbed.momentum[1] + 1e-6 * 10.0 * t).abs() < 1e-17);
    assert!((r.flight.impulse[1] + 1e-6 * 10.0 * t).abs() < 1e-17);
    assert!((r.flight.work - 1e-6 * 10.0 * 0.019).abs() < 1e-17);
    assert!((r.impact.deposition.capture.absorbed.kinetic_energy - r.flight.work).abs() < 1e-17);
}
#[test]
fn ballistic_elastic_bounce_keeps_gravity_after_contact() {
    let start = [0.0, 0.02, 0.0];
    let (mut l, lifecycle) = ballistic_fixture(start, [0.0, -1.0, 0.0], [0.0, -10.0, 0.0]);
    let mut f = film();
    let mut m = model(0.0, 0.0);
    m.spray.rebound.restitution = 1.0;
    let r = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &[start],
            &mut f,
            m,
            &[0.001],
            control(),
            &lifecycle,
        )
        .unwrap();
    let t = ((1.38_f64).sqrt() - 1.0) / 10.0;
    let remaining = 0.1 - t;
    let bounce = 1.0 + 10.0 * t;
    assert_eq!(r.events, 1);
    assert!((l.particles()[0].velocity[1] - (bounce - 10.0 * remaining)).abs() < 1e-12);
    assert!(
        (l.particles()[0].position[1] - (0.001 + bounce * remaining - 5.0 * remaining * remaining))
            .abs()
            < 1e-12
    );
    assert!((kinetic(&l) - 0.5e-6 - r.flight.work).abs() < 1e-17);
    assert!(
        (l.particles()[0].mass * l.particles()[0].velocity[1]
            + r.impact.spray.substrate_impulse[1]
            + 1e-6
            - r.flight.impulse[1])
            .abs()
            < 1e-17
    );
}
#[test]
fn ballistic_curved_path_captures_even_when_endpoint_chord_misses() {
    let start = [0.0, 0.009, 0.0];
    let (mut l, lifecycle) = ballistic_fixture(start, [0.0, -0.4, 0.0], [0.0, 8.0, 0.0]);
    let mut f = film();
    assert!(
        f.film()
            .first_sphere_hit(start, l.particles()[0].position, 0.001)
            .unwrap()
            .is_none()
    );
    let r = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &[start],
            &mut f,
            model(10.0, 0.0),
            &[0.001],
            control(),
            &lifecycle,
        )
        .unwrap();
    assert_eq!(r.events, 1);
    assert_eq!(r.impact.deposition.capture.particles, 1);
}
#[test]
fn ballistic_endpoint_mismatch_and_geometry_budget_roll_back() {
    let start = [0.0, 0.02, 0.0];
    let (mut l, mut lifecycle) = ballistic_fixture(start, [0.0; 3], [0.0, -10.0, 0.0]);
    let mut f = film();
    let before = l.clone();
    let before_f = f.film().total_mass();
    lifecycle.flight.as_mut().unwrap().max_feature_checks = 1;
    assert!(
        l.depositing_impact_spheres_surface_mixture_lifecycle(
            &[start],
            &mut f,
            model(10.0, 0.0),
            &[0.001],
            control(),
            &lifecycle
        )
        .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(f.film().total_mass(), before_f);
    lifecycle.flight.as_mut().unwrap().max_feature_checks = 1000;
    lifecycle.flight.as_mut().unwrap().initial_velocities[0][1] = 1.0;
    assert!(
        l.depositing_impact_spheres_surface_mixture_lifecycle(
            &[start],
            &mut f,
            model(10.0, 0.0),
            &[0.001],
            control(),
            &lifecycle
        )
        .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(f.film().total_mass(), before_f);
}

#[test]
fn newborn_fragment_flight_balances_external_work_and_impulse() {
    let start = [0.0, 0.04, 0.0];
    let (mut l, life) = ballistic_fixture(start, [0.0, -1.0, 0.0], [0.0, -10.0, 0.0]);
    let mut f = film();
    let mut m = model(0.0, 0.1);
    m.spray.surface_tension = 0.0;
    m.spray.position_radius = 0.015;
    let r = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &[start],
            &mut f,
            m,
            &[0.001],
            control(),
            &life,
        )
        .unwrap();
    assert_eq!(r.impact.spray.fragments_created, 2);
    assert_eq!(r.events, 1);
    assert!((r.flight.impulse[1] + 1e-6).abs() < 1e-17);
    assert!(
        (kinetic(&l) + r.impact.spray.substrate_heat + r.impact.spray.created_surface_energy
            - 0.5e-6
            - r.flight.work)
            .abs()
            < 1e-17
    );
    let momentum: f64 = l.particles().iter().map(|p| p.mass * p.velocity[1]).sum();
    assert!(
        (momentum + r.impact.spray.substrate_impulse[1] + 1e-6 - r.flight.impulse[1]).abs() < 1e-17
    );
    let t = (1.78_f64.sqrt() - 1.0) / 10.0;
    let expected_velocity = 0.5 * (1.0 + 10.0 * t) - 10.0 * (0.1 - t);
    assert!(
        l.particles()
            .iter()
            .all(|p| (p.velocity[1] - expected_velocity).abs() < 1e-12)
    );
}

#[test]
fn same_acceleration_merge_follows_center_of_mass_parabola() {
    let previous = [[-0.02, 1.0, 0.0], [0.02, 1.0, 0.0]];
    let velocities = [[0.3, 0.0, 0.0], [-0.3, 0.0, 0.0]];
    let mut l = fluid(
        &[[-0.02, 1.05, 0.0], [0.02, 1.05, 0.0]],
        &[[0.3, -1.0, 0.0], [-0.3, -1.0, 0.0]],
    );
    l.configure_droplet_population(Some(vec![true, true]))
        .unwrap();
    let mut life = lifecycle();
    life.flight = Some(physics::liquid::DropletFlight {
        initial_velocities: velocities.to_vec(),
        acceleration: [0.0, -10.0, 0.0],
        max_feature_checks: 1000,
    });
    let mut f = film();
    let r = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &previous,
            &mut f,
            model(0.0, 0.0),
            &[0.001; 2],
            control(),
            &life,
        )
        .unwrap();
    assert_eq!(r.coalescence.events.len(), 1);
    assert_eq!(l.particles().len(), 1);
    assert!(l.particles()[0].position[0].abs() < 1e-12);
    assert!((l.particles()[0].position[1] - 0.95).abs() < 1e-12);
    assert!((l.particles()[0].velocity[1] + 1.0).abs() < 1e-12);
    assert!(
        (kinetic(&l) + r.coalescence.unresolved_kinetic_energy - 0.09e-6 - r.flight.work).abs()
            < 1e-17
    );
    assert!((r.flight.impulse[1] + 2e-6).abs() < 1e-17);
}

#[test]
fn reconstructed_wet_surface_captures_earlier_and_retains_component_inventory() {
    let start = [0.1, 0.02, 0.1];
    let (mut l, mut life) = ballistic_fixture(start, [0.0; 3], [0.0, -10.0, 0.0]);
    l.set_maxwell_fluid(
        0,
        Some(physics::liquid::MaxwellFluid {
            modulus: 2.0,
            relaxation_time: 0.03,
        }),
    )
    .unwrap();
    l.set_conformations(vec![[[4., 0., 0.], [0., 1., 0.], [0., 0., 1.]]])
        .unwrap();
    let polymer_energy = l.polymer_energy().unwrap();
    let mut f = film();
    f.deposit_batch(&[(0, 0.08, vec![0.5, 0.5]), (1, 0.08, vec![0.5, 0.5])])
        .unwrap();
    let before = f.component_masses().unwrap();
    life.free_surface_side = Some(-1.0); // floor fixture winding faces down
    let r = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &[start],
            &mut f,
            model(10.0, 0.0),
            &[0.001],
            control(),
            &life,
        )
        .unwrap();
    let time = (2.0_f64 * 0.009 / 10.0).sqrt();
    assert_eq!(r.events, 1);
    assert!((r.impact.deposition.capture.absorbed.polymer_energy - polymer_energy).abs() < 1e-20);
    assert!((r.impact.deposition.capture.absorbed.momentum[1] + 1e-6 * 10.0 * time).abs() < 1e-17);
    assert!(
        (r.impact.deposition.capture.absorbed.kinetic_energy - 1e-6 * 10.0 * 0.009).abs() < 1e-17
    );
    let after = f.component_masses().unwrap();
    for (i, added) in [0.9e-6, 0.1e-6].into_iter().enumerate() {
        assert!((after[i] - before[i] - added).abs() < 1e-13);
    }
}
#[test]
fn invalid_free_surface_side_rolls_back_complete_liquid_and_film_inventory() {
    let start = [0.1, 0.02, 0.1];
    let (mut l, mut life) = ballistic_fixture(start, [0.0; 3], [0.0, -10.0, 0.0]);
    let mut f = film();
    let original = l.clone();
    life.free_surface_side = Some(0.0);
    assert!(
        l.depositing_impact_spheres_surface_mixture_lifecycle(
            &[start],
            &mut f,
            model(10.0, 0.0),
            &[0.001],
            control(),
            &life
        )
        .is_err()
    );
    assert_eq!(l, original);
    assert_eq!(f.film().total_volume(), 0.0);
    assert_eq!(f.component_masses().unwrap(), vec![0.0, 0.0]);
}

#[test]
fn slow_immersed_drop_is_captured_at_time_zero_with_all_ledgers() {
    let start = [0.1, 0.005, 0.1];
    let (mut l, mut life) = ballistic_fixture(start, [0.0, 0.5, 0.0], [0.0, -10.0, 0.0]);
    let mut f = film();
    f.deposit_batch(&[(0, 0.08, vec![0.5, 0.5]), (1, 0.08, vec![0.5, 0.5])])
        .unwrap();
    life.free_surface_side = Some(-1.0);
    life.capture_film_immersion = true;
    let r = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &[start],
            &mut f,
            model(1.0, 0.0),
            &[0.0001],
            control(),
            &life,
        )
        .unwrap();
    assert!(l.particles().is_empty());
    assert_eq!(r.events, 1);
    assert_eq!(r.flight.work, 0.0);
    assert_eq!(r.flight.impulse, [0.0; 3]);
    assert!((r.impact.deposition.capture.absorbed.kinetic_energy - 0.125e-6).abs() < 1e-17);
    assert!((r.impact.deposition.capture.absorbed.momentum[1] - 0.5e-6).abs() < 1e-17);
    assert!(
        (r.impact.deposition.capture.absorbed.thermal_energy.unwrap() - 1e-6 * 4184.0 * 300.0)
            .abs()
            < 1e-12
    );
    assert_eq!(r.impact.deposition.component_masses, vec![0.9e-6, 0.1e-6]);
}
fn rising_film_fixture() -> (
    Liquid,
    physics::surface_film::FilmMixture,
    Vec<[f64; 3]>,
    DropletLifecycle,
) {
    let previous = vec![[0.0, 0.005, 0.0], [0.1, 0.01008, 0.1]];
    let mut l = Liquid::new(
        previous
            .iter()
            .zip([1.6, 0.8])
            .map(|(p, mass)| Particle {
                position: *p,
                velocity: [0.0; 3],
                mass,
                material: 0,
            })
            .collect(),
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
            2
        ],
        vec![TransportMaterial::default()],
    )
    .unwrap();
    l.configure_species(
        vec!["a".into(), "b".into()],
        vec![vec![0.9, 0.1], vec![0.2, 0.8]],
    )
    .unwrap();
    let mut f = film();
    f.deposit_batch(&[(0, 0.08, vec![0.5, 0.5]), (1, 0.08, vec![0.5, 0.5])])
        .unwrap();
    let life = DropletLifecycle {
        free_surface_side: Some(-1.0),
        capture_film_immersion: true,
        ..DropletLifecycle::default()
    };
    (l, f, previous, life)
}
#[test]
fn prepared_deposition_raises_surface_and_engulfs_another_drop_in_same_call() {
    let (mut l, mut f, previous, life) = rising_film_fixture();
    let before = f.component_masses().unwrap();
    let incoming = l.species_totals().unwrap().unwrap();
    let r = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &previous,
            &mut f,
            model(1.0, 0.0),
            &[1e-6; 2],
            control(),
            &life,
        )
        .unwrap();
    assert_eq!(r.events, 2);
    assert_eq!(r.impact.deposition.capture.particles, 2);
    assert!(l.particles().is_empty());
    assert!((r.impact.deposition.capture.deposited_volume - 0.0024).abs() < 1e-16);
    for (i, after) in f.component_masses().unwrap().iter().enumerate() {
        assert!((after - before[i] - incoming[i]).abs() < 1e-12);
    }
}
#[test]
fn engulfment_budget_failure_restores_all_prepared_deposits_and_components() {
    let (mut l, mut f, previous, life) = rising_film_fixture();
    let before = l.clone();
    let inventory = f.component_masses().unwrap();
    let volumes = f.film().state().cell_volumes_m3;
    let mut c = control();
    c.max_events = 1;
    assert_eq!(
        l.depositing_impact_spheres_surface_mixture_lifecycle(
            &previous,
            &mut f,
            model(1.0, 0.0),
            &[1e-6; 2],
            c,
            &life
        )
        .unwrap_err(),
        "film impact event budget"
    );
    assert_eq!(l, before);
    assert_eq!(f.component_masses().unwrap(), inventory);
    assert_eq!(f.film().state().cell_volumes_m3, volumes);
}

#[test]
fn fast_immersed_drop_is_rejected_without_tunneling_or_partial_deposition() {
    let start = [0.1, 0.005, 0.1];
    let (mut l, mut life) = ballistic_fixture(start, [0.0, -2.0, 0.0], [0.0, -10.0, 0.0]);
    let mut f = film();
    f.deposit_batch(&[(0, 0.08, vec![0.5, 0.5]), (1, 0.08, vec![0.5, 0.5])])
        .unwrap();
    let before = l.clone();
    let inventory = f.component_masses().unwrap();
    life.free_surface_side = Some(-1.0);
    life.capture_film_immersion = true;
    assert_eq!(
        l.depositing_impact_spheres_surface_mixture_lifecycle(
            &[start],
            &mut f,
            model(1.0, 0.0),
            &[0.0001],
            control(),
            &life
        )
        .unwrap_err(),
        "fast immersed droplet requires wet impact response"
    );
    assert_eq!(l, before);
    assert_eq!(f.component_masses().unwrap(), inventory);
}

#[test]
fn captures_report_thermal_energy_in_the_receiving_cell() {
    let previous = [[-0.5, 0.1, 0.5], [0.5, 0.1, -0.5]];
    let mut l = fluid(&previous, &[[0., -2., 0.]; 2]);
    let initial = l.transport_totals().unwrap().unwrap().0;
    let mut f = film();
    let report = l
        .depositing_impact_spheres_surface_mixture_lifecycle(
            &previous,
            &mut f,
            model(3., 0.),
            &[0.01; 2],
            control(),
            &DropletLifecycle::default(),
        )
        .unwrap();
    assert_eq!(report.deposited_thermal_energy.len(), 2);
    let mut heat = [0.; 2];
    for (cell, energy) in report.deposited_thermal_energy {
        heat[cell] += energy.unwrap();
    }
    assert!((heat[1] - 1e-6 * 4184. * 300.).abs() < 1e-12);
    assert!((heat[0] - 1e-6 * 4184. * 400.).abs() < 1e-12);
    assert!((heat.iter().sum::<f64>() - initial).abs() < 1e-12);
    assert_eq!(l.mass(), 0.);
}

#[test]
fn thermal_lifecycle_credits_receiving_cells_and_rolls_back_late_failure() {
    let previous = [[-0.5, 0.1, 0.5], [0.5, 0.1, -0.5]];
    let initial = fluid(&previous, &[[0., -2., 0.]; 2]);
    let target =
        physics::surface_film::ThermalFilmMixture::new(film(), vec![4184.; 2], &[300.; 2]).unwrap();
    let mut rejected = initial.clone();
    let mut rejected_film = target.clone();
    let before_liquid = format!("{rejected:?}");
    let before_film = format!("{rejected_film:?}");
    let mut limited = control();
    limited.max_events = 1;
    assert!(
        rejected
            .depositing_impact_spheres_thermal_surface_mixture_lifecycle(
                &previous,
                &mut rejected_film,
                model(3., 0.),
                &[0.01; 2],
                limited,
                &DropletLifecycle::default()
            )
            .is_err()
    );
    assert_eq!(format!("{rejected:?}"), before_liquid);
    assert_eq!(format!("{rejected_film:?}"), before_film);
    let mut accepted = initial.clone();
    let mut accepted_film = target;
    let report = accepted
        .depositing_impact_spheres_thermal_surface_mixture_lifecycle(
            &previous,
            &mut accepted_film,
            model(3., 0.),
            &[0.01; 2],
            control(),
            &DropletLifecycle::default(),
        )
        .unwrap();
    assert_eq!(report.impact.deposition.capture.particles, 2);
    let temperatures = accepted_film.temperatures().unwrap();
    assert!((temperatures[0].unwrap() - 400.).abs() < 1e-10);
    assert!((temperatures[1].unwrap() - 300.).abs() < 1e-10);
    assert!(
        (accepted_film.energies_j().iter().sum::<f64>()
            - initial.transport_totals().unwrap().unwrap().0)
            .abs()
            < 1e-12
    );
    assert!((accepted_film.mixture().film().total_mass() - initial.mass()).abs() < 1e-15);
}

#[test]
fn sub_inventory_resolution_capture_preserves_droplet_and_thermal_film() {
    let previous = [[0.5, 0.1, -0.5]];
    let mut l = Liquid::new(
        vec![Particle {
            position: [0.5, -0.1, -0.5],
            velocity: [0., -2., 0.],
            mass: 1e-30,
            material: 0,
        }],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    l.configure_transport(
        vec![LiquidField {
            temperature: 300.,
            concentration: 0.,
        }],
        vec![TransportMaterial::default()],
    )
    .unwrap();
    l.configure_species(vec!["a".into(), "b".into()], vec![vec![0.2, 0.8]])
        .unwrap();
    let mut base = film();
    base.deposit(0, 0.001, &[0.2, 0.8]).unwrap();
    base.deposit(1, 0.001, &[0.2, 0.8]).unwrap();
    let mut target =
        physics::surface_film::ThermalFilmMixture::new(base, vec![4184.; 2], &[300.; 2]).unwrap();
    let before_liquid = format!("{l:?}");
    let before_film = format!("{target:?}");
    assert!(
        l.depositing_impact_spheres_thermal_surface_mixture_lifecycle(
            &previous,
            &mut target,
            model(3., 0.),
            &[0.01],
            control(),
            &DropletLifecycle::default()
        )
        .is_err()
    );
    assert_eq!(format!("{l:?}"), before_liquid);
    assert_eq!(format!("{target:?}"), before_film);
}
