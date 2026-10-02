use physics::liquid::{Config, Liquid, LiquidField, Material, Particle, TransportMaterial};
fn fluid() -> Liquid {
    let mut l = Liquid::new(
        (0..2)
            .map(|i| Particle {
                position: [i as f64 * 0.2, 0.0, 0.0],
                velocity: [0.0; 3],
                mass: 1.0,
                material: 0,
            })
            .collect(),
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            smoothing_radius: 1.0,
            ..Config::default()
        },
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
        vec![TransportMaterial {
            specific_heat: 2.0,
            conductivity: 0.0,
            diffusivity: 20.0,
            mixing_group: 0,
        }],
    )
    .unwrap();
    l.configure_species(
        vec!["a".into(), "b".into()],
        vec![vec![1.0, 0.0], vec![0.0, 1.0]],
    )
    .unwrap();
    l
}
#[test]
fn configuration_preserves_energy_and_invalid_configuration_is_atomic() {
    let mut l = fluid();
    let energy = l.transport_totals().unwrap().unwrap().0;
    l.configure_species_heat_capacities(Some(vec![2.0, 4.0]))
        .unwrap();
    assert_eq!(
        l.particle_specific_heats().unwrap().unwrap(),
        vec![2.0, 4.0]
    );
    assert_eq!(l.fields().unwrap()[1].temperature, 150.0);
    assert_eq!(l.transport_totals().unwrap().unwrap().0, energy);
    let before = l.clone();
    assert!(
        l.configure_species_heat_capacities(Some(vec![2.0, 0.0]))
            .is_err()
    );
    assert_eq!(l, before);
    assert!(
        l.configure_species(vec!["a".into(), "b".into()], vec![vec![1.0, 0.0]; 2])
            .is_err()
    );
    assert_eq!(l, before);
    l.configure_species_heat_capacities(None).unwrap();
    assert_eq!(l.fields().unwrap()[1].temperature, 300.0);
}
#[test]
fn diffusing_components_carry_heat_and_preserve_isothermal_state() {
    for symmetric in [false, true] {
        let mut l = fluid();
        // Set equal temperature AFTER energy-preserving capacity configuration.
        l.configure_species_heat_capacities(Some(vec![2.0, 4.0]))
            .unwrap();
        l.configure_transport(
            vec![
                LiquidField {
                    temperature: 300.0,
                    concentration: 0.0
                };
                2
            ],
            vec![TransportMaterial {
                specific_heat: 2.0,
                conductivity: 0.0,
                diffusivity: 20.0,
                mixing_group: 0,
            }],
        )
        .unwrap();
        let energy = l.transport_totals().unwrap().unwrap().0;
        let masses = l.species_totals().unwrap().unwrap();
        if symmetric {
            l.set_viscous_heating(true).unwrap();
            l.step_symmetric_free(0.01).unwrap();
        } else {
            l.step(0.01, None).unwrap();
        }
        assert!(l.species_fractions().unwrap()[0][1] > 0.0);
        for f in l.fields().unwrap() {
            assert!((f.temperature - 300.0).abs() < 1e-10);
        }
        assert!((l.transport_totals().unwrap().unwrap().0 - energy).abs() < 1e-9);
        for (a, b) in masses.iter().zip(l.species_totals().unwrap().unwrap()) {
            assert!((a - b).abs() < 1e-12);
        }
    }
}
#[test]
fn reservoir_uses_actual_component_capacity_and_energy_ledger() {
    let mut l = fluid();
    l.configure_species_heat_capacities(Some(vec![2.0, 4.0]))
        .unwrap();
    let initial = l.transport_totals().unwrap().unwrap().0;
    let heat = l.exchange_reservoir_heat(0.5, 600.0, &[3.0, 3.0]).unwrap();
    for (i, cp) in [2.0_f64, 4.0].iter().enumerate() {
        let old = if i == 0 { 300.0 } else { 150.0 };
        let expected = 600.0 + (old - 600.0) * (-3.0 * 0.5 / cp).exp();
        assert!((l.fields().unwrap()[i].temperature - expected).abs() < 1e-10);
    }
    assert!((l.transport_totals().unwrap().unwrap().0 - initial - heat).abs() < 1e-9);
}
