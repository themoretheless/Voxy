use physics::astrophysics_nuclear::{Budget, Network, Nucleus, Reaclib, Reaction};
fn network() -> Network {
    Network {
        nuclei: vec![
            Nucleus {
                mass_number: 4,
                charge: 2,
                binding_energy: 1e-12,
            },
            Nucleus {
                mass_number: 12,
                charge: 6,
                binding_energy: 4e-12,
            },
        ],
        reactions: vec![Reaction {
            reactants: vec![3, 0],
            products: vec![0, 1],
            rate: Reaclib {
                sets: vec![[6_f64.ln(), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]],
                min_temperature: 1e8,
                max_temperature: 1e9,
            },
            neutrino_fraction: 0.1,
        }],
    }
}
#[test]
fn triple_fuel_analytic_decay_and_energy_reservoir() {
    let n = network();
    let mut fractions = [1.0, 0.0];
    let initial = n.reservoir(&fractions).unwrap();
    let result = n
        .burn(
            &mut fractions,
            1000.0,
            1e8,
            1.0,
            Budget {
                max_step: 0.001,
                steps: 2000,
                fit_evaluations: 2000,
            },
        )
        .unwrap();
    let exact = 1.0 / (1.0 + 3.0_f64 / 8.0).sqrt();
    assert!((fractions[0] - exact).abs() < 1e-4);
    assert!((fractions.iter().sum::<f64>() - 1.0).abs() < 1e-13);
    let error =
        n.reservoir(&fractions).unwrap() + result.deposited_energy + result.escaped_neutrinos
            - initial;
    assert!(error.abs() / initial.abs() < 1e-13);
    assert!(result.deposited_energy > 0.0 && result.escaped_neutrinos > 0.0);
}
#[test]
fn coefficient_sets_add_and_density_order_is_correct() {
    let mut n = network();
    let set = n.reactions[0].rate.sets[0];
    n.reactions[0].rate.sets.push(set);
    assert!((n.reactions[0].rate.evaluate(1e8).unwrap() - 12.0).abs() < 1e-12);
    let mut first = [1.0, 0.0];
    let mut second = first;
    let b = Budget {
        max_step: 1e-6,
        steps: 2,
        fit_evaluations: 4,
    };
    n.burn(&mut first, 1000.0, 1e8, 1e-6, b).unwrap();
    n.burn(&mut second, 2000.0, 1e8, 1e-6, b).unwrap();
    assert!(((1.0 - second[0]) / (1.0 - first[0]) - 4.0).abs() < 1e-8);
}
#[test]
fn invalid_charge_range_and_late_budget_are_atomic() {
    let mut n = network();
    let mut x = [1.0, 0.0];
    let b = Budget {
        max_step: 0.001,
        steps: 2,
        fit_evaluations: 2,
    };
    assert!(n.burn(&mut x, 1000.0, 1e8, 1.0, b).is_err());
    assert_eq!(x, [1.0, 0.0]);
    assert!(n.burn(&mut x, 1000.0, 1e7, 1.0, b).is_err());
    assert_eq!(x, [1.0, 0.0]);
    n.nuclei[1].charge = 5;
    assert!(n.reservoir(&x).is_err());
}

#[test]
fn isochoric_feedback_conserves_energy_and_rolls_back() {
    let n = network();
    let density = 1000.0;
    let initial_fractions = [1.0, 0.0];
    let initial_energy = n
        .mixture(&initial_fractions)
        .unwrap()
        .at(density, 2e8)
        .unwrap()
        .internal_energy_density
        / density;
    let mut fractions = initial_fractions;
    let mut energy = initial_energy;
    let budget = Budget {
        max_step: 1e-4,
        steps: 200,
        fit_evaluations: 200,
    };
    let result = n
        .burn_isochoric(&mut fractions, density, &mut energy, 0.01, budget)
        .unwrap();
    let temperature = n
        .mixture(&fractions)
        .unwrap()
        .temperature(density, density * energy)
        .unwrap();
    assert!(temperature > 2e8);
    assert!(fractions[1] > 0.0);
    let initial_total = initial_energy + n.reservoir(&initial_fractions).unwrap();
    let final_total = energy + n.reservoir(&fractions).unwrap() + result.escaped_neutrinos;
    assert!((final_total - initial_total).abs() < initial_total.abs() * 1e-12);
    let mut unchanged = initial_fractions;
    let mut unchanged_energy = initial_energy;
    let failure = n.burn_isochoric(
        &mut unchanged,
        density,
        &mut unchanged_energy,
        0.01,
        Budget {
            steps: 2,
            fit_evaluations: 2,
            ..budget
        },
    );
    assert_eq!(
        failure,
        Err(physics::astrophysics_nuclear::Error::BudgetExceeded)
    );
    assert_eq!(unchanged, initial_fractions);
    assert_eq!(unchanged_energy, initial_energy);
}

#[test]
fn temperature_dependent_rate_responds_to_heating() {
    let mut n = network();
    n.reactions[0].rate.sets[0][6] = 2.0;
    let rho = 1000.0;
    let mut coupled = [1.0, 0.0];
    let mut held = coupled;
    let mut energy = n
        .mixture(&coupled)
        .unwrap()
        .at(rho, 2e8)
        .unwrap()
        .internal_energy_density
        / rho;
    let budget = Budget {
        max_step: 1e-4,
        steps: 2000,
        fit_evaluations: 2000,
    };
    n.burn_isochoric(&mut coupled, rho, &mut energy, 0.1, budget)
        .unwrap();
    n.burn(&mut held, rho, 2e8, 0.1, budget).unwrap();
    assert!(coupled[1] > held[1]);
    let mut refined = [1.0, 0.0];
    let mut refined_energy = n
        .mixture(&refined)
        .unwrap()
        .at(rho, 2e8)
        .unwrap()
        .internal_energy_density
        / rho;
    n.burn_isochoric(
        &mut refined,
        rho,
        &mut refined_energy,
        0.1,
        Budget {
            max_step: 5e-5,
            steps: 3000,
            fit_evaluations: 3000,
        },
    )
    .unwrap();
    assert!((coupled[1] - refined[1]).abs() < 1e-7);
    assert!((energy - refined_energy).abs() / energy < 1e-7);
}

#[test]
fn instantaneous_rates_have_analytic_units_budget_and_endothermic_sign() {
    use physics::astrophysics_nuclear::{AVOGADRO, Error};
    let mut n = network();
    let fractions = [1.0, 0.0];
    let rates = n.rates(&fractions, 1000.0, 1e8, 1).unwrap();
    assert!((rates.mass_fraction_rates[0] + 3.0 / 16.0).abs() < 1e-14);
    assert!((rates.mass_fraction_rates[1] - 3.0 / 16.0).abs() < 1e-14);
    let total = 1000.0 * AVOGADRO * 1e-12 / 64.0;
    assert!((rates.deposited_power / (total * 0.9) - 1.0).abs() < 1e-14);
    assert!((rates.escaped_neutrino_power / (total * 0.1) - 1.0).abs() < 1e-14);
    assert_eq!(rates.fit_evaluations, 1);
    assert_eq!(
        n.rates(&fractions, 1000.0, 1e8, 0),
        Err(Error::BudgetExceeded)
    );
    let h = 1e-6;
    let mut evolved = fractions;
    let burn = n
        .burn(
            &mut evolved,
            1000.0,
            1e8,
            h,
            Budget {
                max_step: h,
                steps: 1,
                fit_evaluations: 1,
            },
        )
        .unwrap();
    assert!((burn.deposited_energy / h / rates.deposited_power - 1.0).abs() < 1e-14);
    for (i, x) in evolved.iter().enumerate() {
        assert!((x - fractions[i] - h * rates.mass_fraction_rates[i]).abs() < 1e-16);
    }
    n.nuclei[1].binding_energy = 2e-12;
    let cooling = n.rates(&fractions, 1000.0, 1e8, 1).unwrap();
    assert!(cooling.deposited_power < 0.0);
    assert_eq!(cooling.escaped_neutrino_power, 0.0);
    assert_eq!(fractions, [1.0, 0.0]);
}
