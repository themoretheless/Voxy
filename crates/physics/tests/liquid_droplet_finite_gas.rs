use physics::liquid::{Config, Liquid, Material, Particle, VaporCell};
fn fluid() -> Liquid {
    Liquid::new(
        vec![Particle {
            position: [0.0; 3],
            velocity: [3.0, 0.0, 0.0],
            mass: 2.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap()
}
fn gas() -> VaporCell {
    VaporCell {
        mass: 4.0,
        volume: 1.0,
        temperature: 300.0,
        velocity: [-1.0, 2.0, 0.0],
        specific_heat_cv: 2.0,
    }
}
fn energy(l: &Liquid, g: VaporCell) -> f64 {
    let p = l.particles()[0];
    0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>() + g.energy(1.0).unwrap()
}
#[test]
fn finite_cell_changes_both_velocities_and_closes_momentum_energy() {
    let mut l = fluid();
    let mut g = gas();
    let p = l.particles()[0];
    let old = g;
    let initial = energy(&l, g);
    let heat = l.exchange_droplet_drag(0, 0.2, &mut g, 1.0, 0.5).unwrap();
    let inverse = 1.0 / p.mass + 1.0 / old.mass;
    let speed = 20.0_f64.sqrt();
    let factor = 1.0 + 0.5 * 4.0 * 0.5 * std::f64::consts::PI * inverse * speed * 0.2;
    for k in 0..3 {
        assert!(
            (l.particles()[0].velocity[k]
                - g.velocity[k]
                - (p.velocity[k] - old.velocity[k]) / factor)
                .abs()
                < 1e-14
        );
        assert!(
            (p.mass * l.particles()[0].velocity[k] + g.mass * g.velocity[k]
                - p.mass * p.velocity[k]
                - old.mass * old.velocity[k])
                .abs()
                < 1e-14
        );
    }
    assert!(g.temperature > old.temperature);
    assert!((heat - 8.0 * (g.temperature - old.temperature)).abs() < 1e-12);
    assert!((energy(&l, g) - initial).abs() < 1e-12);
}
#[test]
fn exact_pair_semigroup_and_invalid_state_rolls_back_both() {
    let mut single = fluid();
    let mut split = single.clone();
    let mut a = gas();
    let mut b = a;
    single
        .exchange_droplet_drag(0, 1.0, &mut a, 1.0, 0.5)
        .unwrap();
    for _ in 0..10 {
        split
            .exchange_droplet_drag(0, 0.1, &mut b, 1.0, 0.5)
            .unwrap();
    }
    for k in 0..3 {
        assert!(
            (single.particles()[0].velocity[k] - split.particles()[0].velocity[k]).abs() < 1e-14
        );
    }
    assert!((a.temperature - b.temperature).abs() < 1e-12);
    let before = single.clone();
    let previous = a;
    assert!(
        single
            .exchange_droplet_drag(0, -1.0, &mut a, 1.0, 0.5)
            .is_err()
    );
    assert_eq!(single, before);
    assert_eq!(a, previous);
    assert!(
        single
            .exchange_droplet_drag(0, 1.0, &mut a, f64::MAX, 0.5)
            .is_err()
    );
    assert_eq!(single, before);
    assert_eq!(a, previous);
}

fn cloud() -> Liquid {
    let mut l = Liquid::new(
        [
            ([3.0, 0.0, 0.0], 2.0),
            ([-2.0, 0.0, 0.0], 3.0),
            ([7.0, 2.0, 0.0], 5.0),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (velocity, mass))| Particle {
            position: [i as f64 * 0.1, 0.0, 0.0],
            velocity,
            mass,
            material: 0,
        })
        .collect(),
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    l.configure_droplet_population(Some(vec![true, true, false]))
        .unwrap();
    l.configure_transport(
        vec![
            physics::liquid::LiquidField {
                temperature: 310.0,
                concentration: 0.0
            };
            3
        ],
        vec![physics::liquid::TransportMaterial::default()],
    )
    .unwrap();
    l.configure_species(
        vec!["carrier".into(), "gel".into()],
        vec![vec![1.0, 0.0], vec![0.5, 0.5], vec![0.1, 0.9]],
    )
    .unwrap();
    l
}
fn cloud_gas() -> VaporCell {
    VaporCell {
        velocity: [0.4, 0.0, 0.0],
        ..gas()
    }
}
fn cloud_energy(l: &Liquid, g: VaporCell) -> f64 {
    l.particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum::<f64>()
        + g.energy(1.0).unwrap()
}
#[test]
fn shared_cell_exchange_changes_gas_and_preserves_all_cloud_ledgers() {
    let mut l = cloud();
    let mut g = cloud_gas();
    let before = l.clone();
    let old = g;
    let initial = cloud_energy(&l, g);
    let report = l
        .exchange_marked_droplet_drag(0.2, &mut g, &[1.0, 0.7, 0.2], 0.5)
        .unwrap();
    assert_eq!(report.pair_steps, 4);
    assert_eq!(l.fields(), before.fields());
    assert_eq!(l.species_fractions(), before.species_fractions());
    assert_eq!(l.structure_fractions(), before.structure_fractions());
    assert_eq!(l.particles()[2], before.particles()[2]);
    assert!(report.dissipated_heat > 0.0);
    assert!(g.temperature > old.temperature);
    assert!((cloud_energy(&l, g) - initial).abs() < 1e-12);
    assert!(
        (report.dissipated_heat
            - old.mass * old.specific_heat_cv * (g.temperature - old.temperature))
            .abs()
            < 1e-12
    );
    for k in 0..3 {
        let delta: f64 = l
            .particles()
            .iter()
            .zip(before.particles())
            .map(|(a, b)| a.mass * (a.velocity[k] - b.velocity[k]))
            .sum();
        assert!((delta + report.gas_impulse[k]).abs() < 1e-14);
        assert!((report.gas_impulse[k] - g.mass * (g.velocity[k] - old.velocity[k])).abs() < 1e-14);
    }
}
#[test]
fn cloud_drag_refines_quadratically_to_independent_simultaneous_rk4_solution() {
    let derivative = |v: [f64; 3]| {
        let mut f = [0.0; 3];
        for i in 0..2 {
            let w = v[i] - v[2];
            let radius = if i == 0 { 1.0 } else { 0.7 };
            let force = std::f64::consts::PI * radius * radius * w * w.abs();
            f[i] = -force / if i == 0 { 2.0 } else { 3.0 };
            f[2] += force / 4.0;
        }
        f
    };
    let mut reference = [3.0, -2.0, 0.4];
    let dt = 0.2 / 2048.0;
    for _ in 0..2048 {
        let a = derivative(reference);
        let b = derivative(std::array::from_fn(|k| reference[k] + 0.5 * dt * a[k]));
        let c = derivative(std::array::from_fn(|k| reference[k] + 0.5 * dt * b[k]));
        let d = derivative(std::array::from_fn(|k| reference[k] + dt * c[k]));
        for k in 0..3 {
            reference[k] += dt * (a[k] + 2.0 * b[k] + 2.0 * c[k] + d[k]) / 6.0;
        }
    }
    let mut previous_error = None;
    for steps in [8, 16, 32] {
        let mut l = cloud();
        let mut g = cloud_gas();
        for _ in 0..steps {
            l.exchange_marked_droplet_drag(0.2 / steps as f64, &mut g, &[1.0, 0.7, 0.2], 0.5)
                .unwrap();
        }
        let values = [
            l.particles()[0].velocity[0],
            l.particles()[1].velocity[0],
            g.velocity[0],
        ];
        let error = values
            .iter()
            .zip(reference)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        eprintln!("shared cloud: steps={steps}, velocity_error={error}");
        if let Some(old) = previous_error {
            assert!(old / error > 3.8, "ratio={}", old / error);
        }
        previous_error = Some(error);
    }
}
#[test]
fn shared_cell_control_budget_and_late_overflow_roll_back_both_states() {
    let mut l = cloud();
    let mut g = cloud_gas();
    let before = l.clone();
    let old = g;
    assert!(
        l.exchange_marked_droplet_drag(0.2, &mut g, &[1.0, f64::MAX, 0.2], 0.5)
            .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(g, old);
    assert!(
        l.exchange_marked_droplet_drag(0.2, &mut g, &[1.0, 0.7], 0.5)
            .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(g, old);
    let mut limited = Liquid::new(
        l.particles().to_vec(),
        vec![Material::WATER],
        Config {
            max_neighbor_checks: 3,
            ..Config::default()
        },
    )
    .unwrap();
    limited
        .configure_droplet_population(Some(vec![true, true, false]))
        .unwrap();
    let original = limited.clone();
    assert!(
        limited
            .exchange_marked_droplet_drag(0.2, &mut g, &[1.0, 0.7, 0.2], 0.5)
            .is_err()
    );
    assert_eq!(limited, original);
    assert_eq!(g, old);
}
#[test]
fn no_marked_drops_leave_finite_cell_untouched() {
    let mut l = cloud();
    l.configure_droplet_population(Some(vec![false; 3]))
        .unwrap();
    let before = l.clone();
    let mut g = cloud_gas();
    let old = g;
    assert_eq!(
        l.exchange_marked_droplet_drag(0.2, &mut g, &[1.0, 0.7, 0.2], 0.5)
            .unwrap(),
        physics::liquid::FiniteDropletDragReport::default()
    );
    assert_eq!(l, before);
    assert_eq!(g, old);
}
