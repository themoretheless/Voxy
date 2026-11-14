use physics::liquid::{
    Config, Liquid, LiquidField, Material, Particle, Reaction, TransportMaterial,
};
fn fluid() -> Liquid {
    let mut l = Liquid::new(
        vec![Particle {
            position: [0.0; 3],
            velocity: [0.0; 3],
            mass: 2.0,
            material: 0,
        }],
        vec![Material::WATER],
        Config {
            gravity: [0.0; 3],
            ..Config::default()
        },
    )
    .unwrap();
    l.configure_transport(
        vec![LiquidField {
            temperature: 300.0,
            concentration: 0.0,
        }],
        vec![TransportMaterial {
            specific_heat: 2.0,
            conductivity: 0.0,
            diffusivity: 0.0,
            mixing_group: 0,
        }],
    )
    .unwrap();
    l.configure_species(
        vec!["fuel".into(), "oxidizer".into(), "product".into()],
        vec![vec![0.25, 0.5, 0.25]],
    )
    .unwrap();
    l.configure_reaction(Some(Reaction {
        fuel: 0,
        oxidizer: 1,
        product: 2,
        oxidizer_ratio: 2.0,
        rate: 4.0,
        chemical_energies: vec![100.0, 0.0, 0.0],
    }))
    .unwrap();
    l
}
fn total(l: &Liquid) -> f64 {
    l.chemical_energy().unwrap().unwrap() + l.transport_totals().unwrap().unwrap().0
}
#[test]
fn analytic_stoichiometry_and_energy() {
    let mut l = fluid();
    let initial = total(&l);
    l.react(0.5).unwrap();
    let row = &l.species_fractions().unwrap()[0];
    assert!((row[0] - 0.125).abs() < 1e-14);
    assert!((row[1] - 0.25).abs() < 1e-14);
    assert!((row[2] - 0.625).abs() < 1e-14);
    assert!((total(&l) - initial).abs() < 1e-12);
    let mut split = fluid();
    for _ in 0..10 {
        split.react(0.05).unwrap();
    }
    for (a, b) in row.iter().zip(&split.species_fractions().unwrap()[0]) {
        assert!((a - b).abs() < 1e-14);
    }
}
#[test]
fn automatic_steps_and_atomic_invalid_configuration() {
    for symmetric in [false, true] {
        let mut l = fluid();
        let initial = total(&l);
        if symmetric {
            l.set_viscous_heating(true).unwrap();
            l.step_symmetric_free(0.01).unwrap();
        } else {
            l.step(0.01, None).unwrap();
        }
        assert!(l.species_fractions().unwrap()[0][0] < 0.25);
        assert!((total(&l) - initial).abs() < 1e-10);
        let before = l.clone();
        assert!(l.react(f64::NAN).is_err());
        assert_eq!(l, before);
        assert!(
            l.configure_species(vec!["x".into()], vec![vec![1.0]])
                .is_err()
        );
        assert_eq!(l, before);
        assert!(
            l.configure_reaction(Some(Reaction {
                fuel: 0,
                oxidizer: 0,
                product: 2,
                oxidizer_ratio: 2.0,
                rate: 1.0,
                chemical_energies: vec![100.0, 0.0, 0.0]
            }))
            .is_err()
        );
        assert_eq!(l, before);
    }
}
#[test]
fn excess_reactant_and_large_time_stay_positive() {
    let mut l = fluid();
    l.configure_reaction(None).unwrap();
    l.configure_species(
        vec!["fuel".into(), "oxidizer".into(), "product".into()],
        vec![vec![0.1, 0.7, 0.2]],
    )
    .unwrap();
    l.configure_reaction(Some(Reaction {
        fuel: 0,
        oxidizer: 1,
        product: 2,
        oxidizer_ratio: 2.0,
        rate: 4.0,
        chemical_energies: vec![100.0, 0.0, 0.0],
    }))
    .unwrap();
    let initial = total(&l);
    l.react(1e100).unwrap();
    let row = &l.species_fractions().unwrap()[0];
    assert!(row[0] < 1e-14);
    assert!((row[1] - 0.5).abs() < 1e-14);
    assert!((row[2] - 0.5).abs() < 1e-14);
    assert!((total(&l) - initial).abs() < 1e-12);
}

#[test]
fn thermal_coefficient_and_activation_cutoff() {
    use physics::liquid::ReactionKinetics;
    let law = ReactionKinetics {
        activation_temperature: 1200.0,
        ignition_temperature: 350.0,
    };
    assert_eq!(law.coefficient(40.0, 300.0).unwrap(), 0.0);
    assert!((law.coefficient(40.0, 600.0).unwrap() - 40.0 * (-2.0_f64).exp()).abs() < 1e-14);
    assert!(law.coefficient(40.0, 900.0).unwrap() > law.coefficient(40.0, 600.0).unwrap());
    assert_eq!(
        ReactionKinetics {
            ignition_temperature: 0.0,
            ..law
        }
        .coefficient(40.0, 0.0)
        .unwrap(),
        0.0
    );
    let mut l = fluid();
    l.configure_reaction_kinetics(Some(law)).unwrap();
    let before = l.clone();
    l.react(10.0).unwrap();
    assert_eq!(l, before);
    l.add_heat(&[240.0]).unwrap(); // 4 J/K capacity: heat from 300 K to 360 K.
    let initial = total(&l);
    l.react(0.1).unwrap();
    assert!(l.species_fractions().unwrap()[0][0] < 0.25);
    assert!((total(&l) - initial).abs() < 1e-12);
    let before = l.clone();
    assert!(
        l.configure_reaction_kinetics(Some(ReactionKinetics {
            activation_temperature: f64::NAN,
            ..law
        }))
        .is_err()
    );
    assert_eq!(l, before);
    l.add_heat(&[-400.0]).unwrap();
    let row = l.species_fractions().unwrap().to_vec();
    l.react(1.0).unwrap();
    assert_eq!(l.species_fractions().unwrap(), row);
}
fn hot_reactor() -> Liquid {
    let mut l = fluid();
    l.configure_reaction(Some(Reaction {
        fuel: 0,
        oxidizer: 1,
        product: 2,
        oxidizer_ratio: 2.0,
        rate: 40.0,
        chemical_energies: vec![2000.0, 0.0, 0.0],
    }))
    .unwrap();
    l.configure_reaction_kinetics(Some(physics::liquid::ReactionKinetics {
        activation_temperature: 1200.0,
        ignition_temperature: 0.0,
    }))
    .unwrap();
    l.configure_reaction_accuracy(None).unwrap();
    l
}
fn reference_fuel(n: u32) -> f64 {
    let dt = 1.0 / f64::from(n);
    let mut y = 0.25;
    let rhs = |fuel: f64| -80.0 * (-1200.0 / (300.0 + 1000.0 * (0.25 - fuel))).exp() * fuel * fuel;
    for _ in 0..n {
        let a = rhs(y);
        let b = rhs(y + 0.5 * dt * a);
        let c = rhs(y + 0.5 * dt * b);
        let d = rhs(y + dt * c);
        y += dt / 6.0 * (a + 2.0 * b + 2.0 * c + d);
    }
    y
}
#[test]
fn thermal_acceleration_converges_against_independent_ode() {
    let reference = reference_fuel(8192);
    let fine = reference_fuel(16384);
    let mut errors = Vec::new();
    for n in [8_u32, 16, 32] {
        let mut l = hot_reactor();
        let initial = total(&l);
        for _ in 0..n {
            l.react(1.0 / f64::from(n)).unwrap();
        }
        let fuel = l.species_fractions().unwrap()[0][0];
        let error = (fuel - reference).abs();
        errors.push(error);
        println!("steps={n},fuel={fuel:.15},error={error:.15e}");
        assert!((total(&l) - initial).abs() < 1e-9);
    }
    assert!((reference - fine).abs() < errors[2] * 0.001);
    assert!(errors[0] / errors[1] > 3.5);
    assert!(errors[1] / errors[2] > 3.5);
}
#[test]
fn thermal_kinetics_couples_to_both_flow_integrators() {
    let reference = reference_fuel(8192);
    for symmetric in [false, true] {
        for adaptive in [false, true] {
            let mut l = hot_reactor();
            if adaptive {
                l.configure_reaction_accuracy(Some(physics::liquid::ReactionAccuracy::default()))
                    .unwrap();
            }
            let initial = total(&l);
            for _ in 0..64 {
                if symmetric {
                    l.set_viscous_heating(true).unwrap();
                    l.step_symmetric_free(1.0 / 64.0).unwrap();
                } else {
                    l.step(1.0 / 64.0, None).unwrap();
                }
            }
            assert!((l.species_fractions().unwrap()[0][0] - reference).abs() < 1e-5);
            assert!((total(&l) - initial).abs() < 1e-8);
        }
    }
}

#[test]
fn reactive_heat_advances_latent_fraction_without_false_temperature_rise() {
    use physics::liquid::{PhaseChange, ReactionKinetics};
    let mut l = fluid();
    l.configure_phase_change(
        vec![Some(PhaseChange {
            temperature: 300.0,
            latent_heat: 100.0,
            high_phase: Material::WATER,
        })],
        vec![0.0],
    )
    .unwrap();
    l.configure_reaction_kinetics(Some(ReactionKinetics {
        activation_temperature: 1200.0,
        ignition_temperature: 0.0,
    }))
    .unwrap();
    let initial = total(&l);
    l.react(0.5).unwrap();
    let rate = 4.0 * (-4.0_f64).exp();
    let fuel = 0.25 / (1.0 + 2.0 * rate * 0.25 * 0.5);
    assert!((l.species_fractions().unwrap()[0][0] - fuel).abs() < 1e-14);
    assert!((l.fields().unwrap()[0].temperature - 300.0).abs() < 1e-12);
    assert!((l.phase_fractions().unwrap()[0] - (0.25 - fuel)).abs() < 1e-14);
    assert!((total(&l) - initial).abs() < 1e-12);
}

#[test]
fn adaptive_chemistry_resolves_single_large_step_and_tolerance_refinement() {
    use physics::liquid::ReactionAccuracy;
    let reference = reference_fuel(16384);
    let mut errors = Vec::new();
    for tolerance in [1e-3, 1e-6, 1e-9] {
        let mut l = hot_reactor();
        l.configure_reaction_accuracy(Some(ReactionAccuracy {
            relative_tolerance: tolerance,
            fraction_tolerance: tolerance * 1e-3,
            temperature_tolerance: tolerance * 1e-2,
            max_attempts: 4096,
        }))
        .unwrap();
        let initial = total(&l);
        l.react(1.0).unwrap();
        let error = (l.species_fractions().unwrap()[0][0] - reference).abs();
        println!("relative_tolerance={tolerance:e},fuel_error={error:.15e}");
        errors.push(error);
        assert!((total(&l) - initial).abs() < 1e-8);
    }
    assert!(errors[1] < errors[0] / 10.0);
    assert!(errors[2] < errors[1] / 10.0);
    assert!(errors[2] < 1e-7);
}
#[test]
fn exhausted_chemical_accuracy_rolls_back_full_step() {
    use physics::liquid::ReactionAccuracy;
    for integration in 0..3 {
        let mut l = hot_reactor();
        l.configure_reaction_accuracy(Some(ReactionAccuracy {
            relative_tolerance: 0.0,
            fraction_tolerance: 1e-15,
            temperature_tolerance: 1e-15,
            max_attempts: 1,
        }))
        .unwrap();
        if integration == 2 {
            l.set_viscous_heating(true).unwrap();
        }
        let before = l.clone();
        let result = match integration {
            0 => l.react(1.0),
            1 => l.step(0.01, None).map(|_| ()),
            _ => l.step_symmetric_free(0.01).map(|_| ()),
        };
        assert!(result.is_err());
        assert_eq!(l, before);
        assert!(
            l.configure_reaction_accuracy(Some(ReactionAccuracy {
                max_attempts: 0,
                ..ReactionAccuracy::default()
            }))
            .is_err()
        );
        assert_eq!(l, before);
    }
}
#[test]
fn default_adaptive_kinetics_matches_rapid_self_heating_reference() {
    use physics::liquid::ReactionKinetics;
    let rhs = |fuel: f64| -2e5 * (-3000.0 / (300.0 + 5000.0 * (0.25 - fuel))).exp() * fuel * fuel;
    let reference = |n: u32| {
        let dt = 0.1 / f64::from(n);
        let mut y = 0.25;
        for _ in 0..n {
            let a = rhs(y);
            let b = rhs(y + 0.5 * dt * a);
            let c = rhs(y + 0.5 * dt * b);
            let d = rhs(y + dt * c);
            y += dt / 6.0 * (a + 2.0 * b + 2.0 * c + d);
        }
        y
    };
    let mut l = fluid();
    l.configure_reaction(Some(Reaction {
        fuel: 0,
        oxidizer: 1,
        product: 2,
        oxidizer_ratio: 2.0,
        rate: 1e5,
        chemical_energies: vec![10000.0, 0.0, 0.0],
    }))
    .unwrap();
    l.configure_reaction_kinetics(Some(ReactionKinetics {
        activation_temperature: 3000.0,
        ignition_temperature: 0.0,
    }))
    .unwrap();
    let initial = total(&l);
    l.react(0.1).unwrap();
    let exact = reference(65536);
    assert!((exact - reference(32768)).abs() < 1e-10);
    let fuel = l.species_fractions().unwrap()[0][0];
    println!(
        "rapid_reactor_fuel={fuel:.15},reference={exact:.15},error={:.15e}",
        (fuel - exact).abs()
    );
    assert!(fuel > 0.0 && fuel < 0.01);
    assert!((fuel - exact).abs() < 1e-7);
    assert!((total(&l) - initial).abs() < 1e-8);
}

#[test]
fn reactive_heat_uses_pressure_shifted_latent_plateau() {
    use physics::liquid::{PhaseChange, ReactionKinetics, SaturationCurve};
    let mut l = fluid();
    l.configure_phase_change(
        vec![Some(PhaseChange {
            temperature: 300.0,
            latent_heat: 100.0,
            high_phase: Material::WATER,
        })],
        vec![0.0],
    )
    .unwrap();
    let curve = SaturationCurve {
        reference_temperature: 300.0,
        reference_pressure: 1e5,
        latent_heat: 100.0,
        vapor_gas_constant: 1.0,
        min_temperature: 280.0,
        max_temperature: 400.0,
    };
    l.configure_saturation(vec![Some(curve)], vec![curve.pressure(290.0).unwrap()])
        .unwrap();
    l.configure_reaction_kinetics(Some(ReactionKinetics {
        activation_temperature: 1200.0,
        ignition_temperature: 0.0,
    }))
    .unwrap();
    let initial = total(&l);
    l.react(0.5).unwrap();
    let rate = 4.0 * (-1200.0_f64 / 290.0).exp();
    let fuel = 0.25 / (1.0 + rate * 0.25);
    assert!((l.species_fractions().unwrap()[0][0] - fuel).abs() < 1e-12);
    assert!((l.fields().unwrap()[0].temperature - 290.0).abs() < 1e-10);
    assert!((l.phase_fractions().unwrap()[0] - (0.2 + 0.25 - fuel)).abs() < 1e-12);
    assert!((total(&l) - initial).abs() < 1e-10);
}
#[test]
fn reaction_with_different_component_capacities_decodes_actual_composition() {
    let mut l = fluid();
    let initial = total(&l);
    l.configure_species_heat_capacities(Some(vec![2.0, 4.0, 8.0]))
        .unwrap();
    assert!((total(&l) - initial).abs() < 1e-12);
    l.react(0.5).unwrap();
    let row = &l.species_fractions().unwrap()[0];
    assert!((row[0] - 0.125).abs() < 1e-14);
    let cp = 2.0 * row[0] + 4.0 * row[1] + 8.0 * row[2];
    let thermal = l.transport_totals().unwrap().unwrap().0;
    assert!((l.fields().unwrap()[0].temperature - thermal / (2.0 * cp)).abs() < 1e-12);
    assert!((total(&l) - initial).abs() < 1e-12);
}
