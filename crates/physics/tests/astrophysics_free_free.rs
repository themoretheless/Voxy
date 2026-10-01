use physics::{
    astrophysics_eos::Species,
    astrophysics_gas::{Boundary, Cell},
    astrophysics_nuclear::{Network, Nucleus},
    astrophysics_opacity::FreeFree,
    astrophysics_spherical::Sphere,
    astrophysics_spherical_radiation::FreeFreeSpectrum,
    astrophysics_thermal::STEFAN_BOLTZMANN,
};
fn setup() -> (Sphere, Vec<Vec<f64>>, Network, FreeFreeSpectrum) {
    let network = Network {
        nuclei: vec![Nucleus {
            mass_number: 1,
            charge: 1,
            binding_energy: 0.0,
        }],
        reactions: vec![],
    };
    let rows = vec![vec![1.0]; 2];
    let state = network.mixture(&rows[0]).unwrap().at(1.0, 1e6).unwrap();
    let sphere = Sphere {
        cells: vec![
            Cell {
                density: 1.0,
                momentum: 0.0,
                energy: state.internal_energy_density,
            };
            2
        ],
        spacing: 1e-18,
        gamma: 5.0 / 3.0,
        g: 0.0,
        outer: Boundary::Reflecting,
    };
    let spectrum = FreeFreeSpectrum {
        surface: physics::astrophysics_spherical_radiation::SpectralSurface::CellSource,
        absorption: FreeFree {
            gaunt_factor: 1.0,
            min_temperature: 1e5,
            max_temperature: 1e8,
        },
        min_frequency: 1e10,
        max_frequency: 1e18,
        bins: 256,
        ambient_temperature: 0.0,
    };
    (sphere, rows, network, spectrum)
}
#[test]
fn spectral_transfer_refines_to_optically_thin_planck_power() {
    let (sphere, rows, network, spectrum) = setup();
    let species = [Species {
        mass_fraction: 1.0,
        mass_number: 1,
        nuclear_charge: 1,
    }];
    let kappa = spectrum.absorption.planck(1.0, 1e6, &species).unwrap();
    let volume = 4.0 * std::f64::consts::PI / 3.0 * (2.0 * sphere.spacing).powi(3);
    let expected = 4.0 * kappa * STEFAN_BOLTZMANN * 1e6_f64.powi(4) * volume;
    let mut previous = f64::INFINITY;
    for bins in [64, 128, 256] {
        let settings = FreeFreeSpectrum { bins, ..spectrum };
        let result = sphere
            .free_free_radiation_rates(&rows, &network, settings, 1024, 4_000_000)
            .unwrap();
        assert_eq!(result.segments, 9558 * bins);
        let error = (result.luminosity / expected - 1.0).abs();
        assert!(error < previous, "{bins}: {error} >= {previous}");
        previous = error;
        assert!((result.heating.iter().sum::<f64>() + result.luminosity).abs() / expected < 1e-12);
    }
    assert!(previous < 3e-4, "{previous}");
    let saved = sphere.clone();
    assert!(sphere
        .free_free_radiation_rates(&rows, &network, spectrum, 1024, 100)
        .is_err());
    assert_eq!(sphere, saved);
    let powers = sphere
        .thermal_rates_free_free(&rows, &network, spectrum, 8, 20000, 0)
        .unwrap();
    assert_eq!(powers.nuclear, vec![0.0; 2]);
    assert_eq!(powers.net, powers.radiation);
}
#[test]
fn spectral_blackbody_irradiation_keeps_lte_balance_and_checks_domain() {
    let (mut sphere, rows, network, spectrum) = setup();
    sphere.spacing = 1.0;
    let settings = FreeFreeSpectrum {
        ambient_temperature: 1e6,
        bins: 32,
        ..spectrum
    };
    let result = sphere
        .free_free_radiation_rates(&rows, &network, settings, 4, 1216)
        .unwrap();
    let scale = 4.0 * std::f64::consts::PI * STEFAN_BOLTZMANN * 1e6_f64.powi(4);
    assert!(result.luminosity.abs() / scale < 1e-12);
    assert!(result.heating[0].abs() / scale < 1e-12);
    let invalid = FreeFreeSpectrum {
        absorption: FreeFree {
            min_temperature: 2e6,
            ..spectrum.absorption
        },
        ..settings
    };
    assert!(sphere
        .free_free_radiation_rates(&rows, &network, invalid, 4, 1216)
        .is_err());
}

#[test]
fn joint_solver_uses_spectral_absorption_and_cumulative_budgets() {
    use physics::{
        astrophysics_equilibrium::Search,
        astrophysics_nuclear::{Reaclib, Reaction},
    };
    // Deliberately synthetic temperature-independent heating isolates the solver
    // from reaction-fit stiffness; absorption itself is the physical free-free law.
    let mut network = Network {
        nuclei: vec![
            Nucleus {
                mass_number: 4,
                charge: 2,
                binding_energy: 0.0,
            },
            Nucleus {
                mass_number: 12,
                charge: 6,
                binding_energy: 1e-15,
            },
        ],
        reactions: vec![Reaction {
            reactants: vec![3, 0],
            products: vec![0, 1],
            rate: Reaclib {
                sets: vec![[6_f64.ln(), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]],
                min_temperature: 1e8,
                max_temperature: 4e8,
            },
            neutrino_fraction: 0.0,
        }],
    };
    let rows = vec![vec![1.0, 0.0]; 2];
    let absorption = FreeFree {
        gaunt_factor: 1.0,
        min_temperature: 1e8,
        max_temperature: 4e8,
    };
    let species = [Species {
        mass_fraction: 1.0,
        mass_number: 4,
        nuclear_charge: 2,
    }];
    let cooling = 4.0
        * absorption.planck(1000.0, 2e8, &species).unwrap()
        * STEFAN_BOLTZMANN
        * 2e8_f64.powi(4);
    let nuclear = network
        .rates(&rows[0], 1000.0, 2e8, 1)
        .unwrap()
        .deposited_power;
    network.reactions[0].rate.sets[0][0] += (cooling / nuclear).ln();
    let reference = network.mixture(&rows[0]).unwrap().at(1000.0, 2e8).unwrap();
    let surface = reference.gas_pressure + reference.radiation_pressure;
    let mut sphere = Sphere {
        cells: [800.0, 1200.0]
            .into_iter()
            .map(|density| Cell {
                density,
                momentum: 0.0,
                energy: network
                    .mixture(&rows[0])
                    .unwrap()
                    .at(density, 2e8)
                    .unwrap()
                    .internal_energy_density,
            })
            .collect(),
        spacing: 1e-5,
        gamma: 5.0 / 3.0,
        g: 6.67430e-11,
        outer: Boundary::Reflecting,
    };
    let initial = sphere.clone();
    let spectrum = FreeFreeSpectrum {
        surface: physics::astrophysics_spherical_radiation::SpectralSurface::CellSource,
        absorption,
        min_frequency: 1e14,
        max_frequency: 1e21,
        bins: 64,
        ambient_temperature: 0.0,
    };
    let search = Search {
        absolute_power_tolerance: 0.0,
        min_density: 500.0,
        max_density: 2000.0,
        min_temperature: 1.01e8,
        max_temperature: 3.99e8,
        relative_tolerance: 1e-7,
        iterations: 40,
        evaluations: 500,
        fit_evaluations: 1000,
    };
    let report = sphere
        .equilibrate_stellar_free_free(&rows, &network, surface, spectrum, 8, 2_000_000, search)
        .unwrap();
    assert!(report.thermal.relative_imbalance().unwrap() < 1e-7);
    assert!(report.hydrostatic_residual < 1e-7);
    assert_eq!(report.segments, report.evaluations * 64 * 74);
    assert_eq!(report.fit_evaluations, report.evaluations * 2);
    let fresh = sphere
        .thermal_rates_free_free(&rows, &network, spectrum, 8, 64 * 74, 2)
        .unwrap();
    assert!(fresh.relative_imbalance().unwrap() < 2e-7);
    let mut failed = initial.clone();
    assert!(failed
        .equilibrate_stellar_free_free(&rows, &network, surface, spectrum, 8, 64 * 6 * 8, search)
        .is_err());
    assert_eq!(failed, initial);
}

#[test]
fn spectral_radiative_evolution_and_real_reaction_close_energy_atomically() {
    use physics::{
        astrophysics_nuclear::Budget,
        astrophysics_reaclib::{parse, MEV_JOULES},
        astrophysics_spherical_radiation::{Heating, ReactiveSettings},
    };
    let rate = parse(
        include_str!("data/triple_alpha_fy05.reaclib"),
        1.9e8,
        2.1e8,
        3,
    )
    .unwrap();
    let mut network = Network {
        nuclei: vec![
            Nucleus {
                mass_number: 4,
                charge: 2,
                binding_energy: 0.0,
            },
            Nucleus {
                mass_number: 12,
                charge: 6,
                binding_energy: rate.q_mev * MEV_JOULES,
            },
        ],
        reactions: vec![],
    };
    network.reactions.push(
        rate.reaction(&network, &["he4", "c12"], 0.0, 1e-10)
            .unwrap(),
    );
    let mut rows = vec![vec![1.0, 0.0]; 2];
    let state = network.mixture(&rows[0]).unwrap().at(1e5, 2e8).unwrap();
    let mut sphere = Sphere {
        cells: vec![
            Cell {
                density: 1e5,
                momentum: 0.0,
                energy: state.internal_energy_density
            };
            2
        ],
        spacing: 1e6,
        gamma: 5.0 / 3.0,
        g: 6.67430e-11,
        outer: Boundary::Reflecting,
    };
    let reference = sphere
        .initialize_balanced(
            &rows,
            &network,
            state.gas_pressure + state.radiation_pressure,
        )
        .unwrap();
    let initial = sphere.clone();
    let initial_rows = rows.clone();
    let energy = sphere.reactive_energy(&rows, &network).unwrap();
    let spectrum = FreeFreeSpectrum {
        surface: physics::astrophysics_spherical_radiation::SpectralSurface::CellSource,
        absorption: FreeFree {
            gaunt_factor: 1.0,
            min_temperature: 1.9e8,
            max_temperature: 2.1e8,
        },
        min_frequency: 1e14,
        max_frequency: 1e21,
        bins: 64,
        ambient_temperature: 0.0,
    };
    let settings = ReactiveSettings {
        hydro_max_step: 1e-3,
        hydro_steps: 100,
        burn: Budget {
            max_step: 1e-3,
            steps: 100,
            fit_evaluations: 1000,
        },
        radiation: Heating {
            specific_heat: 1.0,
            opacity: 0.0,
            ambient: 0.0,
            rays_per_annulus: 8,
            max_segments: 100_000,
            max_step: 1e-3,
            max_steps: 100,
        },
    };
    let report = sphere
        .step_reactive_free_free(
            &mut rows,
            &network,
            1e-3,
            settings,
            spectrum,
            Some(&reference),
            None,
        )
        .unwrap();
    let final_energy = sphere.reactive_energy(&rows, &network).unwrap()
        + report.radiation.escaped_energy
        + report.dynamics.escaped_energy
        + report.dynamics.escaped_binding
        + report.dynamics.escaped_neutrinos;
    assert!((final_energy - energy).abs() / energy.abs() < 1e-12);
    assert!(report.radiation.escaped_energy > 0.0);
    assert!(report.dynamics.deposited_energy > 0.0);
    assert!(rows.iter().all(|r| r[1] > 0.0));
    assert!(report.radiation.steps >= 2);
    assert_eq!(report.radiation.segments, report.radiation.steps * 64 * 74);
    let mut first_half = initial.clone();
    let first = first_half
        .radiate_free_free(
            0.0005,
            settings.radiation,
            &initial_rows,
            &network,
            spectrum,
        )
        .unwrap();
    let mut failed = initial.clone();
    let mut failed_rows = initial_rows.clone();
    let limits = ReactiveSettings {
        radiation: Heating {
            max_segments: first.segments,
            ..settings.radiation
        },
        ..settings
    };
    assert!(failed
        .step_reactive_free_free(
            &mut failed_rows,
            &network,
            1e-3,
            limits,
            spectrum,
            Some(&reference),
            None
        )
        .is_err());
    assert_eq!(failed, initial);
    assert_eq!(failed_rows, initial_rows);
    // Refine the explicit thermal timestep against an independently finer run.
    let mut evolved = Vec::new();
    for max_step in [1e-5, 5e-6, 2.5e-6] {
        let mut model = initial.clone();
        let limits = Heating {
            max_step,
            max_steps: 1000,
            max_segments: 2_000_000,
            ..settings.radiation
        };
        let exchange = model
            .radiate_free_free(1e-3, limits, &initial_rows, &network, spectrum)
            .unwrap();
        assert!(
            (model.totals().unwrap()[2] + exchange.escaped_energy - initial.totals().unwrap()[2])
                .abs()
                / initial.totals().unwrap()[2]
                < 1e-12
        );
        evolved.push(model);
    }
    let difference = |a: &Sphere, b: &Sphere| {
        a.cells
            .iter()
            .zip(&b.cells)
            .map(|(x, y)| (x.energy - y.energy).abs())
            .sum::<f64>()
    };
    let coarse = difference(&evolved[0], &evolved[2]);
    let medium = difference(&evolved[1], &evolved[2]);
    assert!(medium > 0.0 && coarse > 2.0 * medium, "{coarse} {medium}");
    // Fixed-density radiation uses the same finite-band power ledger.
    let mut cooling = initial.clone();
    let before = cooling.totals().unwrap()[2];
    let exchange = cooling
        .radiate_free_free(1e-3, settings.radiation, &initial_rows, &network, spectrum)
        .unwrap();
    assert!(
        (cooling.totals().unwrap()[2] + exchange.escaped_energy - before).abs() / before < 1e-12
    );
}

#[test]
fn zero_duration_spectral_evolution_validates_inputs_without_ray_work() {
    use physics::astrophysics_spherical_radiation::Heating;
    let (mut sphere, rows, network, spectrum) = setup();
    let saved = sphere.clone();
    let settings = Heating {
        specific_heat: 1.0,
        opacity: 0.0,
        ambient: 0.0,
        rays_per_annulus: 1,
        max_segments: 0,
        max_step: 1.0,
        max_steps: 0,
    };
    let result = sphere
        .radiate_free_free(0.0, settings, &rows, &network, spectrum)
        .unwrap();
    assert_eq!(result.steps, 0);
    assert_eq!(result.segments, 0);
    assert_eq!(sphere, saved);
    assert!(sphere
        .radiate_free_free(
            0.0,
            settings,
            &rows,
            &network,
            FreeFreeSpectrum {
                bins: 0,
                ..spectrum
            }
        )
        .is_err());
    assert_eq!(sphere, saved);
}

#[test]
fn opaque_spectral_core_transport_scales_with_inverse_absorption() {
    let (mut sphere, _, network, spectrum) = setup();
    let rows = vec![vec![1.0]; 3];
    let mixture = network.mixture(&rows[0]).unwrap();
    sphere.cells = [1.1e6, 1e6, 0.9e6]
        .into_iter()
        .map(|t| Cell {
            density: 1.0,
            momentum: 0.0,
            energy: mixture.at(1.0, t).unwrap().internal_energy_density,
        })
        .collect();
    sphere.spacing = 1e3;
    let calculate = |gaunt_factor| {
        sphere
            .free_free_radiation_rates(
                &rows,
                &network,
                FreeFreeSpectrum {
                    absorption: FreeFree {
                        gaunt_factor,
                        ..spectrum.absorption
                    },
                    bins: 64,
                    ..spectrum
                },
                16,
                64 * 24 * 16,
            )
            .unwrap()
    };
    let a = calculate(100.0);
    let b = calculate(1000.0);
    assert!(a.heating[0] < 0.0 && b.heating[0] < 0.0);
    let ratio = b.heating[0] / a.heating[0];
    assert!((ratio - 0.1).abs() < 0.01, "{ratio}");
    for rates in [a, b] {
        assert!(
            (rates.heating.iter().sum::<f64>() + rates.luminosity).abs() / rates.luminosity < 1e-12
        );
    }
}

#[test]
fn eddington_surface_has_opaque_flux_scaling_and_preserves_irradiated_lte() {
    use physics::astrophysics_spherical_radiation::SpectralSurface;
    let (mut sphere, rows, network, spectrum) = setup();
    sphere.spacing = 1e3;
    let calculate = |gaunt_factor, ambient_temperature| {
        sphere
            .free_free_radiation_rates(
                &rows,
                &network,
                FreeFreeSpectrum {
                    surface: SpectralSurface::EddingtonApproximation,
                    absorption: FreeFree {
                        gaunt_factor,
                        ..spectrum.absorption
                    },
                    bins: 64,
                    ambient_temperature,
                    ..spectrum
                },
                16,
                64 * 12 * 16,
            )
            .unwrap()
    };
    let a = calculate(100.0, 0.0);
    let b = calculate(1000.0, 0.0);
    assert!((b.luminosity / a.luminosity - 0.1).abs() < 0.01);
    for rates in [a, b] {
        assert!(
            (rates.heating.iter().sum::<f64>() + rates.luminosity).abs() / rates.luminosity < 1e-12
        );
    }
    let lte = calculate(100.0, 1e6);
    let scale = 4.0
        * std::f64::consts::PI
        * (2.0 * sphere.spacing).powi(2)
        * STEFAN_BOLTZMANN
        * 1e6_f64.powi(4);
    assert!(lte.luminosity.abs() / scale < 1e-12);
}

#[test]
fn physical_stellar_profile_satisfies_explicit_absolute_and_relative_balance() {
    use physics::{
        astrophysics_equilibrium::Search,
        astrophysics_reaclib::{parse, MEV_JOULES},
        astrophysics_spherical_radiation::SpectralSurface,
    };
    let rate = parse(include_str!("data/triple_alpha_fy05.reaclib"), 1e6, 1e9, 3).unwrap();
    let mut network = Network {
        nuclei: vec![
            Nucleus {
                mass_number: 4,
                charge: 2,
                binding_energy: 0.0,
            },
            Nucleus {
                mass_number: 12,
                charge: 6,
                binding_energy: rate.q_mev * MEV_JOULES,
            },
        ],
        reactions: vec![],
    };
    network.reactions.push(
        rate.reaction(&network, &["he4", "c12"], 0.0, 1e-10)
            .unwrap(),
    );
    let rows = vec![vec![1.0, 0.0]; 8];
    let mixture = network.mixture(&rows[0]).unwrap();

    let outer = mixture.at(1.0, 1e6).unwrap();
    let surface = outer.gas_pressure + outer.radiation_pressure;
    // A non-equilibrium coarse seed selects the hot branch; split it onto the
    // finer grid without changing mass or internal energy. It is not an oracle.
    let cells: Vec<Cell> = include_str!("data/helium_structure_seed.csv")
        .lines()
        .skip(1)
        .flat_map(|line| {
            let values = line
                .split(',')
                .map(|v| v.parse::<f64>().unwrap())
                .collect::<Vec<_>>();
            [Cell {
                density: values[2],
                momentum: 0.0,
                energy: values[3],
            }; 2]
        })
        .collect();
    let mut sphere = Sphere {
        cells,
        spacing: 1.25e7,
        gamma: 5.0 / 3.0,
        g: 6.67430e-11,
        outer: Boundary::Reflecting,
    };
    let spectrum = FreeFreeSpectrum {
        surface: SpectralSurface::EddingtonApproximation,
        absorption: FreeFree {
            gaunt_factor: 1.0,
            min_temperature: 1e6,
            max_temperature: 1e9,
        },
        min_frequency: 1e12,
        max_frequency: 1e22,
        bins: 32,
        ambient_temperature: 0.0,
    };
    let initial_rates = sphere
        .thermal_rates_free_free(&rows, &network, spectrum, 4, 100000, 24)
        .unwrap();
    assert!(initial_rates.relative_imbalance().unwrap() > 1e-3);
    let search = Search {
        min_density: 1e-6,
        max_density: 1e10,
        min_temperature: 1.01e6,
        max_temperature: 9.99e8,
        relative_tolerance: 1e-6,
        absolute_power_tolerance: 8e24,
        iterations: 150,
        evaluations: 10000,
        fit_evaluations: 240000,
    };
    let report = sphere
        .equilibrate_stellar_free_free(&rows, &network, surface, spectrum, 4, 100000000, search)
        .unwrap();
    assert!(report.hydrostatic_residual <= search.relative_tolerance);
    assert!(report.thermal_residual <= search.relative_tolerance);
    let fresh = sphere
        .thermal_rates_free_free(&rows, &network, spectrum, 4, 100000, 24)
        .unwrap();
    for i in 0..8 {
        let allowed = search
            .absolute_power_tolerance
            .max(search.relative_tolerance * (fresh.nuclear[i].abs() + fresh.radiation[i].abs()));
        assert!(
            fresh.net[i].abs() <= allowed * 1.01,
            "shell {i}: {} > {allowed}",
            fresh.net[i]
        );
    }
    let heat = fresh.nuclear.iter().sum::<f64>();
    assert!((fresh.luminosity / heat - 1.0).abs() < 1e-6);
    assert!(search.absolute_power_tolerance / heat < 1e-6);
    let required = sphere.hydrostatic_pressures(surface).unwrap();
    for (i, cell) in sphere.cells.iter().enumerate() {
        let t = mixture.temperature(cell.density, cell.energy).unwrap();
        let state = mixture.at(cell.density, t).unwrap();
        assert!(
            ((state.gas_pressure + state.radiation_pressure) / required.cells[i] - 1.0).abs()
                < 1e-6
        );
    }
}

#[test]
fn opaque_transfer_damps_alternating_cell_temperatures() {
    let (mut sphere, _, network, spectrum) = setup();
    let rows = vec![vec![1.0]; 8];
    let mixture = network.mixture(&rows[0]).unwrap();
    sphere.cells = (0..8)
        .map(|i| Cell {
            density: 1.0,
            momentum: 0.0,
            energy: mixture
                .at(1.0, if i % 2 == 0 { 1.01e6 } else { 0.99e6 })
                .unwrap()
                .internal_energy_density,
        })
        .collect();
    sphere.spacing = 1e3;
    let calculate = |gaunt_factor| {
        sphere
            .free_free_radiation_rates(
                &rows,
                &network,
                FreeFreeSpectrum {
                    absorption: FreeFree {
                        gaunt_factor,
                        ..spectrum.absorption
                    },
                    bins: 64,
                    ..spectrum
                },
                8,
                64 * 144 * 8,
            )
            .unwrap()
    };
    let a = calculate(100.0);
    let b = calculate(1000.0);
    for i in 2..6 {
        if i % 2 == 0 {
            assert!(a.heating[i] < 0.0);
        } else {
            assert!(a.heating[i] > 0.0);
        }
        assert!((b.heating[i] / a.heating[i] - 0.1).abs() < 0.01);
    }
}
