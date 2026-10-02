use physics::liquid::{
    Config, DropletCoalescenceControl, Liquid, LiquidField, Material, Particle, SpeciesProperties,
    Thixotropy, TransportMaterial, ViscosityBlend,
};
fn particle(x: f64, v: f64, mass: f64) -> Particle {
    Particle {
        position: [x, 0.0, 0.0],
        velocity: [v, 0.0, 0.0],
        mass,
        material: 0,
    }
}
fn kinetic(l: &Liquid) -> f64 {
    l.particles()
        .iter()
        .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
        .sum()
}
fn angular(l: &Liquid) -> [f64; 3] {
    std::array::from_fn(|k| {
        l.particles()
            .iter()
            .map(|p| {
                p.mass
                    * (p.position[(k + 1) % 3] * p.velocity[(k + 2) % 3]
                        - p.position[(k + 2) % 3] * p.velocity[(k + 1) % 3])
            })
            .sum()
    })
}
#[test]
fn heterogeneous_merge_preserves_composition_enthalpy_momentum_and_angular_ledger() {
    let mut particles = vec![
        particle(-0.01, 2.0, 0.001),
        particle(0.01, -1.0, 0.002),
        particle(0.1, 0.0, 0.0005),
    ];
    particles[0].velocity[1] = 1.0;
    particles[1].velocity[1] = -0.5;
    let mut l = Liquid::new(particles, vec![Material::WATER], Config::default()).unwrap();
    l.configure_transport(
        vec![
            LiquidField {
                temperature: 300.0,
                concentration: 0.1,
            },
            LiquidField {
                temperature: 400.0,
                concentration: 0.3,
            },
            LiquidField {
                temperature: 330.0,
                concentration: 0.2,
            },
        ],
        vec![TransportMaterial::default()],
    )
    .unwrap();
    l.configure_species(
        vec!["aqueous".into(), "gel".into()],
        vec![vec![0.9, 0.1], vec![0.2, 0.8], vec![0.5, 0.5]],
    )
    .unwrap();
    l.configure_species_heat_capacities(Some(vec![4200.0, 2000.0]))
        .unwrap();
    l.configure_species_properties(Some(SpeciesProperties {
        components: vec![
            Material {
                rest_density: 800.0,
                ..Material::WATER
            },
            Material {
                rest_density: 1200.0,
                ..Material::WATER
            },
        ],
        viscosity: ViscosityBlend::Logarithmic,
    }))
    .unwrap();
    l.configure_thixotropy(
        vec![Some(Thixotropy {
            recovery_rate: 0.1,
            breakdown: 0.2,
            broken_viscosity_ratio: 0.5,
            broken_yield_ratio: 0.5,
        })],
        vec![0.2, 0.8, 0.5],
    )
    .unwrap();
    l.deposit_dissipation_heat(&[1e-20, 2e-20, 3e-20]).unwrap();
    let before = l.clone();
    let old_heat = l.transport_totals().unwrap().unwrap();
    let old_volume: f64 = before.particles()[..2]
        .iter()
        .zip(before.effective_materials().unwrap())
        .map(|(p, m)| p.mass / m.rest_density)
        .sum();
    let report = l.merge_droplets(&[1, 0], 0.072).unwrap();
    assert_eq!(report.particle_index, 1);
    assert_eq!(l.particles()[0], before.particles()[2]);
    assert!((l.mass() - before.mass()).abs() < 1e-16);
    let p = l.particles()[1];
    assert!((p.mass - 0.003).abs() < 1e-16);
    let rho = l.effective_materials().unwrap()[1].rest_density;
    assert!((p.mass / rho - old_volume).abs() < 1e-18);
    for k in 0..3 {
        let momentum = |l: &Liquid| {
            l.particles()
                .iter()
                .map(|p| p.mass * p.velocity[k])
                .sum::<f64>()
        };
        assert!((momentum(&l) - momentum(&before)).abs() < 1e-16);
        let center = |l: &Liquid| {
            l.particles()
                .iter()
                .map(|p| p.mass * p.position[k])
                .sum::<f64>()
        };
        assert!((center(&l) - center(&before)).abs() < 1e-16);
        assert!(
            (angular(&l)[k] + report.unresolved_angular_momentum[k] - angular(&before)[k]).abs()
                < 1e-16
        );
    }
    assert!((kinetic(&l) + report.unresolved_kinetic_energy - kinetic(&before)).abs() < 1e-16);
    let heat = l.transport_totals().unwrap().unwrap();
    assert!((heat.0 - old_heat.0).abs() < 1e-9);
    assert!((heat.1 - old_heat.1).abs() < 1e-16);
    for (a, b) in before
        .species_totals()
        .unwrap()
        .unwrap()
        .iter()
        .zip(l.species_totals().unwrap().unwrap())
    {
        assert!((a - b).abs() < 1e-16);
    }
    assert!((l.structure_fractions()[1] - 0.6).abs() < 1e-14);
    assert!((l.suspension_heat_buffer()[1] - 3e-20).abs() < 1e-30);
    let area = |v: f64| {
        4.0 * std::f64::consts::PI * (3.0 * v / (4.0 * std::f64::consts::PI)).cbrt().powi(2)
    };
    let old_area: f64 = before.particles()[..2]
        .iter()
        .zip(before.effective_materials().unwrap())
        .map(|(p, m)| area(p.mass / m.rest_density))
        .sum();
    assert!((report.released_surface_energy - 0.072 * (old_area - area(old_volume))).abs() < 1e-16);
}
fn control() -> DropletCoalescenceControl {
    DropletCoalescenceControl {
        dt: 0.1,
        surface_tension: 0.072,
        maximum_normal_speed: 10.0,
        max_events: 8,
    }
}
fn cascade() -> Liquid {
    Liquid::new(
        vec![
            particle(0.1, 2.0, 1e-6),
            particle(0.0, 0.0, 1e-6),
            particle(-0.1, -2.0, 1e-6),
        ],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap()
}
#[test]
fn swept_contact_merges_crossing_paths_and_continues_descendant_motion() {
    let mut l = cascade();
    let before = l.clone();
    let report = l
        .coalesce_swept_droplets(&[[-0.1, 0.0, 0.0], [0.0; 3], [0.1, 0.0, 0.0]], control())
        .unwrap();
    assert_eq!(report.events.len(), 2);
    assert_eq!(l.particles().len(), 1);
    assert!(l.particles()[0].position[0].abs() < 1e-14);
    assert!(l.particles()[0].velocity[0].abs() < 1e-14);
    assert!((l.mass() - before.mass()).abs() < 1e-18);
    assert!((kinetic(&l) + report.unresolved_kinetic_energy - kinetic(&before)).abs() < 1e-18);
    assert!(report.released_surface_energy > 0.0);
}
#[test]
fn late_event_and_search_budgets_roll_back_entire_cascade() {
    for budget in [0, 1] {
        let mut l = cascade();
        let before = l.clone();
        let mut c = control();
        if budget == 0 {
            c.max_events = 1;
        } else {
            l = Liquid::new(
                before.particles().to_vec(),
                vec![Material::WATER],
                Config {
                    max_neighbor_checks: 3,
                    ..Config::default()
                },
            )
            .unwrap();
        }
        let before = l.clone();
        assert!(
            l.coalesce_swept_droplets(&[[-0.1, 0.0, 0.0], [0.0; 3], [0.1, 0.0, 0.0]], c)
                .is_err()
        );
        assert_eq!(l, before);
    }
}
#[test]
fn speed_gate_and_separating_touch_do_not_coalesce() {
    let mut l = cascade();
    let before = l.clone();
    let mut c = control();
    c.maximum_normal_speed = 1.0;
    assert!(
        l.coalesce_swept_droplets(&[[-0.1, 0.0, 0.0], [0.0; 3], [0.1, 0.0, 0.0]], c)
            .unwrap()
            .events
            .is_empty()
    );
    assert_eq!(l, before);
    let r = (3.0 * 1e-6 / (4.0 * std::f64::consts::PI * 1000.0)).cbrt();
    let mut l = Liquid::new(
        vec![particle(-0.1, -1.0, 1e-6), particle(0.1, 1.0, 1e-6)],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    assert!(
        l.coalesce_swept_droplets(&[[-r, 0.0, 0.0], [r, 0.0, 0.0]], control())
            .unwrap()
            .events
            .is_empty()
    );
}
#[test]
fn invalid_merge_indices_and_surface_controls_are_atomic() {
    let mut l = cascade();
    let before = l.clone();
    for indices in [vec![0, 0], vec![0, 3], vec![0]] {
        assert!(l.merge_droplets(&indices, 0.072).is_err());
        assert_eq!(l, before);
    }
    assert!(l.merge_droplets(&[0, 1], f64::NAN).is_err());
    assert_eq!(l, before);
}

#[test]
fn collision_result_matches_subdivided_linear_paths() {
    let previous = [[-0.1, 0.0, 0.0], [0.0; 3], [0.1, 0.0, 0.0]];
    let mut full = cascade();
    let full_report = full.coalesce_swept_droplets(&previous, control()).unwrap();
    let mut split = Liquid::new(
        vec![
            particle(0.0, 2.0, 1e-6),
            particle(0.0, 0.0, 1e-6),
            particle(0.0, -2.0, 1e-6),
        ],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    let mut half = control();
    half.dt = 0.05;
    let first = split.coalesce_swept_droplets(&previous, half).unwrap();
    let old: Vec<_> = split.particles().iter().map(|p| p.position).collect();
    let second = split.coalesce_swept_droplets(&old, half).unwrap();
    assert_eq!(split.particles().len(), full.particles().len());
    for (a, b) in split.particles().iter().zip(full.particles()) {
        assert!((a.mass - b.mass).abs() < 1e-18);
        for k in 0..3 {
            assert!((a.position[k] - b.position[k]).abs() < 1e-14);
            assert!((a.velocity[k] - b.velocity[k]).abs() < 1e-14);
        }
    }
    assert!(
        (first.released_surface_energy + second.released_surface_energy
            - full_report.released_surface_energy)
            .abs()
            < 1e-18
    );
    assert!(
        (first.unresolved_kinetic_energy + second.unresolved_kinetic_energy
            - full_report.unresolved_kinetic_energy)
            .abs()
            < 1e-18
    );
}
