use physics::liquid::{
    Config, Liquid, LiquidField, Material, Particle, SaturationCurve, TransportMaterial, VaporCell,
    VaporExchangeAccuracy, VaporInterface,
};
fn interface() -> VaporInterface {
    VaporInterface {
        curve: SaturationCurve {
            reference_temperature: 10.0,
            reference_pressure: 10.0,
            latent_heat: 100.0,
            vapor_gas_constant: 1.0,
            min_temperature: 5.0,
            max_temperature: 20.0,
        },
        area: 0.1,
        accommodation: 0.01,
    }
}
fn fluid() -> Liquid {
    let mut l = Liquid::new(
        vec![Particle {
            position: [0.0; 3],
            velocity: [2.0, 0.0, 0.0],
            mass: 1.0,
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
            temperature: 10.0,
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
    l
}
fn vapor(mass: f64) -> VaporCell {
    VaporCell {
        mass,
        volume: 1.0,
        temperature: 10.0,
        velocity: [-1.0, 1.0, 0.0],
        specific_heat_cv: 1.0,
    }
}
fn totals(l: &Liquid, v: VaporCell) -> (f64, [f64; 3], f64) {
    let p = l.particles()[0];
    let mut momentum = [0.0; 3];
    for (i, m) in momentum.iter_mut().enumerate() {
        *m = p.mass * p.velocity[i] + v.mass * v.velocity[i];
    }
    let energy = l.transport_totals().unwrap().unwrap().0
        + 0.5 * p.mass * p.velocity.iter().map(|x| x * x).sum::<f64>()
        + v.energy(interface().curve.latent_heat).unwrap();
    (p.mass + v.mass, momentum, energy)
}
#[test]
fn evaporation_and_condensation_conserve_mass_momentum_energy() {
    for mass in [0.0, 0.05, 2.0] {
        let mut l = fluid();
        let mut v = vapor(mass);
        let before = totals(&l, v);
        let transferred = l
            .exchange_vapor(
                0,
                &mut v,
                interface(),
                1.0,
                VaporExchangeAccuracy::default(),
            )
            .unwrap();
        if mass < 1.0 {
            assert!(transferred > 0.0);
            assert!(l.fields().unwrap()[0].temperature < 10.0);
        } else {
            assert!(transferred < 0.0);
            assert!(l.fields().unwrap()[0].temperature > 10.0);
        }
        let after = totals(&l, v);
        assert!((after.0 - before.0).abs() < 1e-12);
        for (a, b) in after.1.iter().zip(before.1) {
            assert!((a - b).abs() < 1e-12);
        }
        assert!((after.2 - before.2).abs() < 1e-10);
        assert!((v.mass - mass - transferred).abs() < 1e-14);
        assert!(l.particles()[0].mass > 0.0 && v.mass >= 0.0);
    }
}
#[test]
fn kinetic_flux_matches_formula_and_equilibrium_has_no_transfer() {
    let model = interface();
    let mut v = vapor(0.05);
    let expected = model.accommodation * (10.0 - 0.5) / (2.0 * std::f64::consts::PI * 10.0).sqrt();
    assert!((model.mass_flux(10.0, v).unwrap() - expected).abs() < 1e-14);
    v.mass = 1.0;
    let mut l = fluid();
    let before = l.clone();
    let old = v;
    let transferred = l
        .exchange_vapor(0, &mut v, model, 1.0, VaporExchangeAccuracy::default())
        .unwrap();
    assert_eq!(transferred, 0.0);
    assert_eq!(l, before);
    assert_eq!(v, old);
    v.temperature = 12.0;
    v.mass = 10.0 * (12.0_f64 / 10.0).sqrt() / 12.0;
    assert!(model.mass_flux(10.0, v).unwrap().abs() < 1e-14);
}
fn reference(n: u32) -> (f64, f64, f64) {
    let dt = 2.0 / f64::from(n);
    let mut mass = 1.0;
    let mut energy = 20.0;
    let rhs = |m: f64, e: f64| {
        let gas_mass = 1.05 - m;
        let t = e / (2.0 * m);
        let gas_t = (25.5 - e - 100.0 * gas_mass) / gas_mass;
        let saturation = 10.0 * (100.0 * (0.1 - 1.0 / t)).exp();
        let rate = 0.001 * (saturation / t.sqrt() - gas_mass * gas_t / gas_t.sqrt())
            / (2.0 * std::f64::consts::PI).sqrt();
        let donor = if rate >= 0.0 { t } else { gas_t };
        (-rate, -rate * (donor + 100.0))
    };
    for _ in 0..n {
        let a = rhs(mass, energy);
        let b = rhs(mass + 0.5 * dt * a.0, energy + 0.5 * dt * a.1);
        let c = rhs(mass + 0.5 * dt * b.0, energy + 0.5 * dt * b.1);
        let d = rhs(mass + dt * c.0, energy + dt * c.1);
        mass += dt / 6.0 * (a.0 + 2.0 * b.0 + 2.0 * c.0 + d.0);
        energy += dt / 6.0 * (a.1 + 2.0 * b.1 + 2.0 * c.1 + d.1);
    }
    let gas_mass = 1.05 - mass;
    (
        mass,
        energy / (2.0 * mass),
        (25.5 - energy - 100.0 * gas_mass) / gas_mass,
    )
}
#[test]
fn finite_cell_feedback_matches_independent_thermal_mass_ode() {
    let exact = reference(8192);
    let refined = reference(16384);
    assert!((exact.1 - refined.1).abs() < 1e-10);
    let mut l = fluid();
    let mut v = vapor(0.05);
    v.velocity = l.particles()[0].velocity;
    let before = totals(&l, v);
    let initial_pressure = v.pressure(1.0).unwrap();
    l.exchange_vapor(
        0,
        &mut v,
        interface(),
        2.0,
        VaporExchangeAccuracy {
            relative_tolerance: 1e-8,
            mass_tolerance: 1e-14,
            temperature_tolerance: 1e-8,
            max_attempts: 4096,
        },
    )
    .unwrap();
    println!(
        "mass_error={:.15e},liquid_temperature_error={:.15e},vapor_temperature_error={:.15e}",
        (l.particles()[0].mass - exact.0).abs(),
        (l.fields().unwrap()[0].temperature - exact.1).abs(),
        (v.temperature - exact.2).abs()
    );
    assert!((l.particles()[0].mass - exact.0).abs() < 1e-8);
    assert!((l.fields().unwrap()[0].temperature - exact.1).abs() < 1e-6);
    assert!((v.temperature - exact.2).abs() < 1e-6);
    assert!(v.pressure(1.0).unwrap() > initial_pressure);
    assert!((totals(&l, v).2 - before.2).abs() < 1e-10);
}
#[test]
fn failed_interface_exchange_rolls_back_both_states() {
    let mut l = fluid();
    let mut v = vapor(0.05);
    let before = l.clone();
    let old = v;
    assert!(
        l.exchange_vapor(
            0,
            &mut v,
            interface(),
            1.0,
            VaporExchangeAccuracy {
                relative_tolerance: 0.0,
                mass_tolerance: 1e-20,
                temperature_tolerance: 1e-20,
                max_attempts: 1
            }
        )
        .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(v, old);
    assert!(
        l.exchange_vapor(
            0,
            &mut v,
            VaporInterface {
                area: -1.0,
                ..interface()
            },
            1.0,
            VaporExchangeAccuracy::default()
        )
        .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(v, old);
    let mut bad = v;
    bad.specific_heat_cv = 2.0;
    let old_bad = bad;
    assert!(
        l.exchange_vapor(
            0,
            &mut bad,
            interface(),
            1.0,
            VaporExchangeAccuracy::default()
        )
        .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(bad, old_bad);
}

#[test]
fn interface_tolerance_refinement_reduces_thermal_mass_error() {
    let exact = reference(16384);
    let mut errors = Vec::new();
    for tolerance in [1e-4, 1e-6, 1e-8] {
        let mut l = fluid();
        let mut v = vapor(0.05);
        v.velocity = l.particles()[0].velocity;
        l.exchange_vapor(
            0,
            &mut v,
            interface(),
            2.0,
            VaporExchangeAccuracy {
                relative_tolerance: tolerance,
                mass_tolerance: tolerance * 1e-6,
                temperature_tolerance: tolerance,
                max_attempts: 4096,
            },
        )
        .unwrap();
        let error = (l.fields().unwrap()[0].temperature - exact.1).abs();
        println!("relative_tolerance={tolerance:e},temperature_error={error:.15e}");
        errors.push(error);
    }
    assert!(errors[1] < errors[0] / 5.0);
    assert!(errors[2] < errors[1] / 5.0);
}

fn solution_model() -> physics::liquid::SolutionVaporInterface {
    physics::liquid::SolutionVaporInterface {
        interface: interface(),
        solvent: 0,
        molar_masses: vec![1.0, 2.0, 4.0],
    }
}
fn solution(fractions: Vec<f64>) -> Liquid {
    let mut l = fluid();
    l.configure_species(
        vec!["solvent".into(), "solute-a".into(), "solute-b".into()],
        vec![fractions],
    )
    .unwrap();
    l
}
#[test]
fn ideal_solution_pressure_uses_moles_and_suppresses_evaporation() {
    let model = solution_model();
    let row = vec![0.5, 0.3, 0.2];
    let activity = 0.5 / (0.5 + 0.3 / 2.0 + 0.2 / 4.0);
    assert!((model.activity(&row).unwrap() - activity).abs() < 1e-14);
    assert!((model.equilibrium_pressure(10.0, &row).unwrap() - 10.0 * activity).abs() < 1e-13);
    let mut pure = fluid();
    let mut mixed = solution(row);
    let mut v = vapor(0.05);
    let mut vs = v;
    let pure_transfer = pure
        .exchange_vapor(
            0,
            &mut v,
            interface(),
            0.1,
            VaporExchangeAccuracy::default(),
        )
        .unwrap();
    let transfer = mixed
        .exchange_solution_vapor(0, &mut vs, &model, 0.1, VaporExchangeAccuracy::default())
        .unwrap();
    assert!(transfer > 0.0 && transfer < pure_transfer);
}
#[test]
fn selective_evaporation_and_condensation_conserve_every_species_mass() {
    for gas_mass in [0.0, 0.05, 2.0] {
        let mut l = solution(vec![0.5, 0.3, 0.2]);
        let mut v = vapor(gas_mass);
        let before = totals(&l, v);
        let species = l.species_totals().unwrap().unwrap();
        let transferred = l
            .exchange_solution_vapor(
                0,
                &mut v,
                &solution_model(),
                1.0,
                VaporExchangeAccuracy::default(),
            )
            .unwrap();
        let after = l.species_totals().unwrap().unwrap();
        assert!((after[0] + v.mass - species[0] - gas_mass).abs() < 1e-12);
        assert!((after[1] - species[1]).abs() < 1e-12);
        assert!((after[2] - species[2]).abs() < 1e-12);
        let row = &l.species_fractions().unwrap()[0];
        assert!((row.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        if gas_mass < 1.0 {
            assert!(transferred > 0.0);
            assert!(row[1] > 0.3 && row[2] > 0.2);
        } else {
            assert!(transferred < 0.0);
            assert!(row[1] < 0.3 && row[2] < 0.2);
        }
        let final_totals = totals(&l, v);
        assert!((final_totals.0 - before.0).abs() < 1e-12);
        for (a, b) in final_totals.1.iter().zip(before.1) {
            assert!((a - b).abs() < 1e-12);
        }
        assert!((final_totals.2 - before.2).abs() < 1e-10);
    }
}
#[test]
fn solvent_can_condense_into_an_initially_solvent_free_solution() {
    let mut l = solution(vec![0.0, 0.6, 0.4]);
    let mut v = vapor(0.1);
    l.exchange_solution_vapor(
        0,
        &mut v,
        &solution_model(),
        0.1,
        VaporExchangeAccuracy::default(),
    )
    .unwrap();
    assert!(l.species_fractions().unwrap()[0][0] > 0.0);
    let mass = l.species_totals().unwrap().unwrap();
    assert!((mass[0] + v.mass - 0.1).abs() < 1e-12);
    assert!((mass[1] - 0.6).abs() < 1e-12);
    assert!((mass[2] - 0.4).abs() < 1e-12);
}
fn solution_reference(n: u32) -> (f64, f64) {
    let mut mass = 1.0;
    let mut energy = 20.0;
    let dt = 2.0 / f64::from(n);
    let rhs = |m: f64, e: f64| {
        let gas_mass = 1.05 - m;
        let t = e / (2.0 * m);
        let gas_t = (25.5 - e - 100.0 * gas_mass) / gas_mass;
        let solvent = m - 0.5;
        let activity = solvent / (solvent + 0.2);
        let saturation = 10.0 * (100.0 * (0.1 - 1.0 / t)).exp();
        let rate = 0.001 * (activity * saturation / t.sqrt() - gas_mass * gas_t / gas_t.sqrt())
            / (2.0 * std::f64::consts::PI).sqrt();
        let donor = if rate >= 0.0 { t } else { gas_t };
        (-rate, -rate * (donor + 100.0))
    };
    for _ in 0..n {
        let a = rhs(mass, energy);
        let b = rhs(mass + 0.5 * dt * a.0, energy + 0.5 * dt * a.1);
        let c = rhs(mass + 0.5 * dt * b.0, energy + 0.5 * dt * b.1);
        let d = rhs(mass + dt * c.0, energy + dt * c.1);
        mass += dt / 6.0 * (a.0 + 2.0 * b.0 + 2.0 * c.0 + d.0);
        energy += dt / 6.0 * (a.1 + 2.0 * b.1 + 2.0 * c.1 + d.1);
    }
    (mass, energy / (2.0 * mass))
}
#[test]
fn composition_feedback_matches_independent_mass_thermal_ode() {
    let exact = solution_reference(8192);
    let fine = solution_reference(16384);
    assert!((exact.1 - fine.1).abs() < 1e-10);
    let mut l = solution(vec![0.5, 0.3, 0.2]);
    let mut v = vapor(0.05);
    v.velocity = l.particles()[0].velocity;
    l.exchange_solution_vapor(
        0,
        &mut v,
        &solution_model(),
        2.0,
        VaporExchangeAccuracy {
            relative_tolerance: 1e-8,
            mass_tolerance: 1e-14,
            temperature_tolerance: 1e-8,
            max_attempts: 4096,
        },
    )
    .unwrap();
    println!(
        "solution_mass_error={:.15e},solution_temperature_error={:.15e}",
        (l.particles()[0].mass - exact.0).abs(),
        (l.fields().unwrap()[0].temperature - exact.1).abs()
    );
    assert!((l.particles()[0].mass - exact.0).abs() < 1e-8);
    assert!((l.fields().unwrap()[0].temperature - exact.1).abs() < 1e-6);
    assert!((l.species_totals().unwrap().unwrap()[0] + v.mass - 0.55).abs() < 1e-12);
}
#[test]
fn invalid_solution_model_or_accuracy_rolls_back_composition_and_vapor() {
    let mut l = solution(vec![0.5, 0.3, 0.2]);
    let mut v = vapor(0.05);
    let before = l.clone();
    let old = v;
    let mut bad = solution_model();
    bad.molar_masses[1] = 0.0;
    assert!(
        l.exchange_solution_vapor(0, &mut v, &bad, 1.0, VaporExchangeAccuracy::default())
            .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(v, old);
    bad = solution_model();
    bad.solvent = 3;
    assert!(
        l.exchange_solution_vapor(0, &mut v, &bad, 1.0, VaporExchangeAccuracy::default())
            .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(v, old);
    assert!(
        l.exchange_solution_vapor(
            0,
            &mut v,
            &solution_model(),
            1.0,
            VaporExchangeAccuracy {
                relative_tolerance: 0.0,
                mass_tolerance: 1e-20,
                temperature_tolerance: 1e-20,
                max_attempts: 1
            }
        )
        .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(v, old);
    l.configure_reaction(Some(physics::liquid::Reaction {
        fuel: 0,
        oxidizer: 1,
        product: 2,
        oxidizer_ratio: 1.0,
        rate: 1.0,
        chemical_energies: vec![100.0, 0.0, 0.0],
    }))
    .unwrap();
    let reactive = l.clone();
    assert!(
        l.exchange_solution_vapor(
            0,
            &mut v,
            &solution_model(),
            1.0,
            VaporExchangeAccuracy::default()
        )
        .is_err()
    );
    assert_eq!(l, reactive);
    assert_eq!(v, old);
}
#[test]
fn selective_evaporation_with_component_capacities_conserves_total_energy() {
    for mass in [0.0, 2.0] {
        let mut l = solution(vec![0.5, 0.3, 0.2]);
        l.configure_species_heat_capacities(Some(vec![2.0, 3.0, 4.0]))
            .unwrap();
        // Restore the requested initial temperature without clearing the species model.
        l.configure_transport(
            vec![LiquidField {
                temperature: 10.0,
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
        let mut v = vapor(mass);
        let before = totals(&l, v);
        let species = l.species_totals().unwrap().unwrap();
        let transferred = l
            .exchange_solution_vapor(
                0,
                &mut v,
                &solution_model(),
                0.5,
                VaporExchangeAccuracy::default(),
            )
            .unwrap();
        assert_eq!(transferred > 0.0, mass == 0.0);
        let after = totals(&l, v);
        assert!((before.0 - after.0).abs() < 1e-12);
        assert!((before.2 - after.2).abs() < 1e-10);
        for (a, b) in before.1.iter().zip(after.1) {
            assert!((a - b).abs() < 1e-12);
        }
        let new = l.species_totals().unwrap().unwrap();
        assert!((new[0] + transferred - species[0]).abs() < 1e-12);
        for i in 1..3 {
            assert!((new[i] - species[i]).abs() < 1e-12);
        }
        let row = &l.species_fractions().unwrap()[0];
        let cp = row[0] * 2.0 + row[1] * 3.0 + row[2] * 4.0;
        assert!((l.particle_specific_heats().unwrap().unwrap()[0] - cp).abs() < 1e-12);
    }
}
