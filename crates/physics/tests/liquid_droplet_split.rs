use physics::liquid::{
    Config, DropletSplit, Liquid, LiquidField, Material, Particle, Thixotropy, TransportMaterial,
};
fn fluid() -> Liquid {
    Liquid::new(
        vec![Particle {
            position: [1.0, 2.0, 3.0],
            velocity: [2.0, -3.0, 1.0],
            mass: 0.001,
            material: 0,
        }],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap()
}
fn model() -> DropletSplit {
    DropletSplit {
        children: 8,
        axis: [0.0, 1.0, 0.0],
        position_radius: 0.001,
        surface_tension: 0.072,
        available_energy: 0.0002,
    }
}
#[test]
fn volume_and_density_set_disjoint_fragment_geometry() {
    for density in [800.0, 1000.0, 2500.0] {
        for count in [2, 3, 8, 64] {
            let mut l = Liquid::new(
                vec![Particle {
                    position: [0.0; 3],
                    velocity: [0.0; 3],
                    mass: 0.001,
                    material: 0,
                }],
                vec![Material {
                    rest_density: density,
                    ..Material::WATER
                }],
                Config::default(),
            )
            .unwrap();
            let snapshot = l.clone();
            let radius =
                (3.0 * 0.001 / (density * count as f64 * 4.0 * std::f64::consts::PI)).cbrt();
            let expected = radius / (std::f64::consts::PI / count as f64).sin();
            let minimum = l.droplet_fragment_ring_radius(0, count).unwrap();
            assert!((minimum / expected - 1.0).abs() < 2e-12);
            assert_eq!(l, snapshot);
            let report = l
                .split_droplet(
                    0,
                    DropletSplit {
                        children: count,
                        position_radius: 1e-9,
                        surface_tension: 0.0,
                        ..model()
                    },
                )
                .unwrap();
            assert_eq!(report.position_radius, minimum);
            for (i, a) in l.particles().iter().enumerate() {
                for b in &l.particles()[..i] {
                    let distance = a
                        .position
                        .iter()
                        .zip(b.position)
                        .map(|(x, y)| (x - y).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    let radii = [a.mass, b.mass]
                        .iter()
                        .map(|m| (3.0 * m / (density * 4.0 * std::f64::consts::PI)).cbrt())
                        .sum::<f64>();
                    assert!(distance >= radii * (1.0 - 1e-13));
                }
            }
        }
    }
    let mut l = fluid();
    let report = l
        .split_droplet(
            0,
            DropletSplit {
                position_radius: 0.5,
                ..model()
            },
        )
        .unwrap();
    assert_eq!(report.position_radius, 0.5);
}

#[test]
fn unrepresentable_fragment_positions_reject_without_mutation() {
    let mut l = Liquid::new(
        vec![Particle {
            position: [1e20; 3],
            velocity: [0.0; 3],
            mass: 0.001,
            material: 0,
        }],
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    let initial = l.clone();
    assert!(l.split_droplet(0, model()).is_err());
    assert_eq!(l, initial);
    assert!(l.droplet_fragment_ring_radius(0, 1).is_err());
    assert!(l.droplet_fragment_ring_radius(1, 8).is_err());
}
#[test]
fn fragmentation_conserves_mass_center_momentum_and_budgeted_energy() {
    for count in [2, 3, 8, 64] {
        let mut l = fluid();
        let p = l.particles()[0];
        let mut controls = model();
        controls.children = count;
        let r = l.split_droplet(0, controls).unwrap();
        assert_eq!(l.particles().len(), count);
        let mut mass = 0.0;
        let mut momentum = [0.0; 3];
        let mut center = [0.0; 3];
        let mut energy = 0.0;
        for child in l.particles() {
            mass += child.mass;
            energy += 0.5 * child.mass * child.velocity.iter().map(|v| v * v).sum::<f64>();
            for k in 0..3 {
                momentum[k] += child.mass * child.velocity[k];
                center[k] += child.mass * child.position[k];
            }
        }
        assert!((mass - p.mass).abs() < 1e-16);
        for k in 0..3 {
            assert!((momentum[k] - p.mass * p.velocity[k]).abs() < 1e-16);
            assert!((center[k] / mass - p.position[k]).abs() < 1e-13);
        }
        let initial = 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>();
        assert!((energy - initial - r.added_kinetic_energy).abs() < 1e-16);
        assert!(
            (r.added_kinetic_energy + r.created_surface_energy - controls.available_energy).abs()
                < 1e-16
        );
        let radius = (3.0 * p.mass / (1000.0 * 4.0 * std::f64::consts::PI)).cbrt();
        let independently_sum_area: f64 = l
            .particles()
            .iter()
            .map(|c| {
                let radius = (3.0 * c.mass / (1000.0 * 4.0 * std::f64::consts::PI)).cbrt();
                4.0 * std::f64::consts::PI * radius * radius
            })
            .sum();
        let cost = 0.072 * (independently_sum_area - 4.0 * std::f64::consts::PI * radius * radius);
        assert!((r.created_surface_energy - cost).abs() < 1e-16);
    }
}
#[test]
fn children_keep_thermal_species_and_structural_inventories() {
    let mut l = fluid();
    l.configure_transport(
        vec![LiquidField {
            temperature: 310.0,
            concentration: 0.2,
        }],
        vec![TransportMaterial::default()],
    )
    .unwrap();
    l.configure_species(vec!["solvent".into(), "gel".into()], vec![vec![0.7, 0.3]])
        .unwrap();
    l.configure_species_heat_capacities(Some(vec![4000.0, 2000.0]))
        .unwrap();
    l.configure_thixotropy(
        vec![Some(Thixotropy {
            recovery_rate: 0.1,
            breakdown: 0.3,
            broken_viscosity_ratio: 0.2,
            broken_yield_ratio: 0.2,
        })],
        vec![0.4],
    )
    .unwrap();
    l.deposit_dissipation_heat(&[1e-20]).unwrap();
    let buffered = l.suspension_heat_buffer().iter().sum::<f64>();
    let heat = l.transport_totals().unwrap().unwrap().0;
    let species = l.species_totals().unwrap().unwrap();
    l.split_droplet(0, model()).unwrap();
    assert!((l.transport_totals().unwrap().unwrap().0 - heat).abs() < 1e-9);
    for (a, b) in species.iter().zip(l.species_totals().unwrap().unwrap()) {
        assert!((a - b).abs() < 1e-16);
    }
    assert!((l.suspension_heat_buffer().iter().sum::<f64>() - buffered).abs() < 1e-30);
    assert_eq!(l.suspension_heat_buffer().len(), 8);
    assert!(l.structure_fractions().iter().all(|v| *v == 0.4));
}
#[test]
fn insufficient_energy_and_invalid_geometry_or_budget_are_atomic() {
    let mut l = fluid();
    let before = l.clone();
    let mut bad = model();
    bad.available_energy = 0.0;
    assert!(l.split_droplet(0, bad).is_err());
    assert_eq!(l, before);
    bad = model();
    bad.axis = [0.0; 3];
    assert!(l.split_droplet(0, bad).is_err());
    assert_eq!(l, before);
    bad = model();
    bad.position_radius = f64::INFINITY;
    assert!(l.split_droplet(0, bad).is_err());
    assert_eq!(l, before);
    let p = before.particles()[0];
    let mut limited = Liquid::new(
        vec![p],
        vec![Material::WATER],
        Config {
            max_particles: 2,
            ..Config::default()
        },
    )
    .unwrap();
    let before = limited.clone();
    assert!(limited.split_droplet(0, model()).is_err());
    assert_eq!(limited, before);
}
#[test]
fn fragments_preserve_local_saturation_pressure_and_latent_inventory() {
    use physics::liquid::{PhaseChange, SaturationCurve};
    let mut l = fluid();
    l.configure_transport(
        vec![LiquidField {
            temperature: 300.0,
            concentration: 0.0,
        }],
        vec![TransportMaterial::default()],
    )
    .unwrap();
    l.configure_phase_change(
        vec![Some(PhaseChange {
            temperature: 300.0,
            latent_heat: 100.0,
            high_phase: Material::OIL,
        })],
        vec![0.3],
    )
    .unwrap();
    l.configure_saturation(
        vec![Some(SaturationCurve {
            reference_temperature: 300.0,
            reference_pressure: 1e5,
            latent_heat: 100.0,
            vapor_gas_constant: 1.0,
            min_temperature: 290.0,
            max_temperature: 310.0,
        })],
        vec![1e5],
    )
    .unwrap();
    let heat = l.transport_totals().unwrap().unwrap().0;
    let fraction = l.phase_fractions().unwrap()[0];
    l.split_droplet(0, model()).unwrap();
    assert!(l.saturation_pressures().unwrap().iter().all(|p| *p == 1e5));
    for f in l.phase_fractions().unwrap() {
        assert!((f - fraction).abs() < 1e-12);
    }
    assert!((l.transport_totals().unwrap().unwrap().0 - heat).abs() < 1e-9);
}
#[test]
fn unrepresentable_fragment_velocity_is_rejected() {
    let mut p = fluid().particles()[0];
    p.velocity = [1e100; 3];
    let mut l = Liquid::new(vec![p], vec![Material::WATER], Config::default()).unwrap();
    let before = l.clone();
    assert!(l.split_droplet(0, model()).is_err());
    assert_eq!(l, before);
}

#[test]
fn unequal_fragments_preserve_inventory_momentum_center_and_energy() {
    let mut l = fluid();
    l.configure_transport(
        vec![LiquidField {
            temperature: 310.0,
            concentration: 0.2,
        }],
        vec![TransportMaterial::default()],
    )
    .unwrap();
    l.configure_species(vec!["water".into(), "gel".into()], vec![vec![0.7, 0.3]])
        .unwrap();
    l.deposit_dissipation_heat(&[1e-20]).unwrap();
    let initial = l.clone();
    let parent = l.particles()[0];
    let weights = [0.01, 0.04, 0.15, 0.8];
    let controls = DropletSplit {
        children: 4,
        ..model()
    };
    let surface = l
        .droplet_fragment_surface_energy_with_mass_fractions(0, &weights, 0.072)
        .unwrap();
    let minimum = l
        .droplet_fragment_ring_radius_with_mass_fractions(0, &weights)
        .unwrap();
    let report = l
        .split_droplet_with_mass_fractions(0, controls, &weights)
        .unwrap();
    assert_eq!(report.created_surface_energy, surface);
    assert_eq!(report.position_radius, minimum);
    assert!((l.mass() - parent.mass).abs() < 1e-16);
    let mut ke = 0.0;
    let mut area = 0.0;
    for (i, p) in l.particles().iter().enumerate() {
        assert!((p.mass / parent.mass - weights[i]).abs() < 1e-14);
        ke += 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>();
        let radius = (3.0 * p.mass / (4.0 * std::f64::consts::PI * 1000.0)).cbrt();
        area += 4.0 * std::f64::consts::PI * radius * radius;
        for q in &l.particles()[..i] {
            let r = (3.0 * q.mass / (4.0 * std::f64::consts::PI * 1000.0)).cbrt();
            let distance = p
                .position
                .iter()
                .zip(q.position)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(distance >= (radius + r) * (1.0 - 1e-12));
        }
        assert!((l.suspension_heat_buffer()[i] - 1e-20 * weights[i]).abs() < 1e-30);
    }
    for k in 0..3 {
        let momentum: f64 = l.particles().iter().map(|p| p.mass * p.velocity[k]).sum();
        let center: f64 = l
            .particles()
            .iter()
            .map(|p| p.mass * p.position[k])
            .sum::<f64>()
            / parent.mass;
        assert!((momentum - parent.mass * parent.velocity[k]).abs() < 1e-16);
        assert!((center - parent.position[k]).abs() < 1e-13);
    }
    let initial_ke = 0.5 * parent.mass * parent.velocity.iter().map(|v| v * v).sum::<f64>();
    assert!((ke - initial_ke + surface - controls.available_energy).abs() < 1e-16);
    let r = (3.0 * parent.mass / (4.0 * std::f64::consts::PI * 1000.0)).cbrt();
    assert!((surface - 0.072 * (area - 4.0 * std::f64::consts::PI * r * r)).abs() < 1e-16);
    assert!(
        (l.transport_totals().unwrap().unwrap().0 - initial.transport_totals().unwrap().unwrap().0)
            .abs()
            < 1e-9
    );
    for (a, b) in l
        .species_totals()
        .unwrap()
        .unwrap()
        .iter()
        .zip(initial.species_totals().unwrap().unwrap())
    {
        assert!((a - b).abs() < 1e-16);
    }
}
#[test]
fn invalid_unequal_distributions_and_insufficient_energy_are_atomic() {
    let mut l = fluid();
    let original = l.clone();
    for fractions in [
        vec![0.0, 1.0],
        vec![0.2, 0.2],
        vec![f64::NAN, 0.5],
        vec![-0.1, 1.1],
        vec![1.0],
    ] {
        assert!(
            l.split_droplet_with_mass_fractions(
                0,
                DropletSplit {
                    children: fractions.len(),
                    ..model()
                },
                &fractions
            )
            .is_err()
        );
        assert_eq!(l, original);
    }
    assert!(
        l.split_droplet_with_mass_fractions(
            0,
            DropletSplit {
                children: 2,
                available_energy: 0.0,
                ..model()
            },
            &[0.2, 0.8]
        )
        .is_err()
    );
    assert_eq!(l, original);
}
