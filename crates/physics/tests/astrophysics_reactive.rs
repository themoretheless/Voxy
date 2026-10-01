use physics::{
    astrophysics_gas::{Boundary, Cell},
    astrophysics_nuclear::{Budget, Network, Nucleus, Reaclib, Reaction},
    astrophysics_spherical::Sphere,
};
fn network() -> Network {
    Network {
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
                max_temperature: 1e9,
            },
            neutrino_fraction: 0.1,
        }],
    }
}
#[test]
fn burning_sphere_closed_and_open_energy_and_atomicity() {
    let network = network();
    for outer in [Boundary::Reflecting, Boundary::Outflow] {
        let mut rows = vec![vec![1.0, 0.0]; 8];
        let rho = 1000.0;
        let energy = network
            .mixture(&rows[0])
            .unwrap()
            .at(rho, 2e8)
            .unwrap()
            .internal_energy_density;
        let velocity = if outer == Boundary::Outflow { 1e4 } else { 0.0 };
        let mut sphere = Sphere {
            cells: vec![
                Cell {
                    density: rho,
                    momentum: rho * velocity,
                    energy: energy + 0.5 * rho * velocity * velocity
                };
                8
            ],
            spacing: 1e6,
            gamma: 5.0 / 3.0,
            g: 1e-10,
            outer,
        };
        let initial = sphere.reactive_energy(&rows, &network).unwrap();
        let budget = Budget {
            max_step: 1e-4,
            steps: 1000,
            fit_evaluations: 1000,
        };
        let report = sphere
            .step_reactive(&mut rows, &network, 0.001, 1e-4, 100, budget)
            .unwrap();
        let final_energy = sphere.reactive_energy(&rows, &network).unwrap()
            + report.escaped_energy
            + report.escaped_binding
            + report.escaped_neutrinos;
        assert!((final_energy - initial).abs() / initial.abs() < 1e-12);
        assert!(rows.iter().all(|r| r[1] > 0.0));
        assert!(report.deposited_energy > 0.0 && report.escaped_neutrinos > 0.0);
        if outer == Boundary::Outflow {
            assert!(report.escaped_species[0] > 0.0);
        }
        let saved = sphere.clone();
        let saved_rows = rows.clone();
        assert!(sphere
            .step_reactive(
                &mut rows,
                &network,
                0.001,
                1e-4,
                100,
                Budget {
                    steps: 12,
                    fit_evaluations: 12,
                    ..budget
                }
            )
            .is_err());
        assert_eq!(sphere, saved);
        assert_eq!(rows, saved_rows);
    }
}

#[test]
fn radiating_burning_sphere_energy_and_late_ray_failure() {
    use physics::astrophysics_spherical_radiation::{Heating, ReactiveSettings};
    let network = network();
    let mut rows = vec![vec![1.0, 0.0]; 8];
    let rho = 1000.0;
    let energy = network
        .mixture(&rows[0])
        .unwrap()
        .at(rho, 2e8)
        .unwrap()
        .internal_energy_density;
    let mut sphere = Sphere {
        cells: vec![
            Cell {
                density: rho,
                momentum: 0.0,
                energy
            };
            8
        ],
        spacing: 1e6,
        gamma: 5.0 / 3.0,
        g: 1e-10,
        outer: Boundary::Reflecting,
    };
    let initial = sphere.reactive_energy(&rows, &network).unwrap();
    let settings = ReactiveSettings {
        hydro_max_step: 1e-5,
        hydro_steps: 100,
        burn: Budget {
            max_step: 1e-6,
            steps: 1000,
            fit_evaluations: 1000,
        },
        radiation: Heating {
            specific_heat: 1.0,
            opacity: 1e-15,
            ambient: 0.0,
            rays_per_annulus: 2,
            max_segments: 10000,
            max_step: 1e-5,
            max_steps: 100,
        },
    };
    let result = sphere
        .step_reactive_radiating(&mut rows, &network, 1e-5, settings)
        .unwrap();
    assert!(result.radiation.escaped_energy > 0.0);
    assert!(result.dynamics.deposited_energy > 0.0);
    let final_energy = sphere.reactive_energy(&rows, &network).unwrap()
        + result.radiation.escaped_energy
        + result.dynamics.escaped_neutrinos
        + result.dynamics.escaped_binding
        + result.dynamics.escaped_energy;
    assert!((final_energy - initial).abs() / initial.abs() < 1e-12);
    let saved = sphere.clone();
    let saved_rows = rows.clone();
    // One transfer consumes 8*9*2 segments: the second radiation half must fail.
    assert!(sphere
        .step_reactive_radiating(
            &mut rows,
            &network,
            1e-5,
            ReactiveSettings {
                radiation: Heating {
                    max_segments: 144,
                    ..settings.radiation
                },
                ..settings
            }
        )
        .is_err());
    assert_eq!(sphere, saved);
    assert_eq!(rows, saved_rows);
}

#[test]
fn reactive_table_matches_constant_and_post_burn_domain_failure_is_atomic() {
    use physics::{
        astrophysics_opacity::{Kind, Table},
        astrophysics_spherical_radiation::{Error, Heating, ReactiveError, ReactiveSettings},
    };
    let network = network();
    let initial_rows = vec![vec![1.0, 0.0]; 8];
    let rho = 1000.0;
    let energy = network
        .mixture(&initial_rows[0])
        .unwrap()
        .at(rho, 2e8)
        .unwrap()
        .internal_energy_density;
    let original = Sphere {
        cells: vec![
            Cell {
                density: rho,
                momentum: 0.0,
                energy
            };
            8
        ],
        spacing: 1e6,
        gamma: 5.0 / 3.0,
        g: 1e-10,
        outer: Boundary::Reflecting,
    };
    let settings = ReactiveSettings {
        hydro_max_step: 1e-4,
        hydro_steps: 100,
        burn: Budget {
            max_step: 1e-4,
            steps: 1000,
            fit_evaluations: 1000,
        },
        radiation: Heating {
            specific_heat: 1.0,
            opacity: 1e-20,
            ambient: 0.0,
            rays_per_annulus: 2,
            max_segments: 10000,
            max_step: 1e-4,
            max_steps: 100,
        },
    };
    let table = Table::new(
        Kind::GreyAbsorption,
        vec![500.0, 2000.0],
        vec![1e8, 4e8],
        vec![1e-20; 4],
    )
    .unwrap();
    let mut tabulated = original.clone();
    let mut constant = original.clone();
    let mut a_rows = initial_rows.clone();
    let mut b_rows = initial_rows.clone();
    let a = tabulated
        .step_reactive_tabulated(&mut a_rows, &network, 0.001, settings, &table)
        .unwrap();
    let b = constant
        .step_reactive_radiating(&mut b_rows, &network, 0.001, settings)
        .unwrap();
    for (a, b) in tabulated.cells.iter().zip(&constant.cells) {
        assert!((a.energy / b.energy - 1.0).abs() < 1e-13);
        assert!((a.density / b.density - 1.0).abs() < 1e-13);
    }
    assert_eq!(a_rows, b_rows);
    assert!((a.radiation.escaped_energy / b.radiation.escaped_energy - 1.0).abs() < 1e-12);
    let initial = original.reactive_energy(&initial_rows, &network).unwrap();
    let final_energy = tabulated.reactive_energy(&a_rows, &network).unwrap()
        + a.radiation.escaped_energy
        + a.dynamics.escaped_energy
        + a.dynamics.escaped_binding
        + a.dynamics.escaped_neutrinos;
    assert!((final_energy - initial).abs() / initial.abs() < 1e-12);
    let narrow = Table::new(
        Kind::GreyAbsorption,
        vec![500.0, 2000.0],
        vec![1e8, 200000001.0],
        vec![1e-20; 4],
    )
    .unwrap();
    let mut failed = original.clone();
    let mut failed_rows = initial_rows.clone();
    assert_eq!(
        failed.step_reactive_tabulated(&mut failed_rows, &network, 0.001, settings, &narrow),
        Err(ReactiveError::Radiation(Error::Opacity(
            physics::astrophysics_opacity::Error::OutsideDomain
        )))
    );
    assert_eq!(failed, original);
    assert_eq!(failed_rows, initial_rows);
}

#[test]
fn imported_reaction_with_tabulated_radiation_has_a_cumulative_energy_ledger() {
    use physics::{
        astrophysics_opacity::{Kind, Table},
        astrophysics_reaclib::{parse, MEV_JOULES},
        astrophysics_spherical_radiation::{Heating, ReactiveSettings},
    };
    // The narrow bounds below are numerical test bounds, not a fit calibration.
    let rate = parse(
        include_str!("data/triple_alpha_fy05.reaclib"),
        1.99e8,
        2.01e8,
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
    let rows = vec![vec![1.0, 0.0]; 8];
    let rho = 1e5;
    let energy = network
        .mixture(&rows[0])
        .unwrap()
        .at(rho, 2e8)
        .unwrap()
        .internal_energy_density;
    let initial_sphere = Sphere {
        cells: vec![
            Cell {
                density: rho,
                momentum: 0.0,
                energy
            };
            8
        ],
        spacing: 1.25e6,
        gamma: 5.0 / 3.0,
        g: 6.67430e-11,
        outer: Boundary::Outflow,
    };
    let table = Table::from_csv(Kind::GreyAbsorption,
        "density_kg_m3,temperature_K,opacity_m2_kg\n10000,100000000,1e-16\n1000000,100000000,2e-16\n10000,400000000,5e-17\n1000000,400000000,1e-16\n", 4).unwrap();
    let evolve = |dt: f64, count: usize| {
        let mut sphere = initial_sphere.clone();
        let mut fractions = rows.clone();
        let initial = sphere.reactive_energy(&fractions, &network).unwrap();
        let settings = ReactiveSettings {
            hydro_max_step: dt / 4.0,
            hydro_steps: 1000,
            burn: Budget {
                max_step: dt / 10.0,
                steps: 1000,
                fit_evaluations: 3000,
            },
            radiation: Heating {
                specific_heat: 1.0,
                opacity: 0.0,
                ambient: 0.0,
                rays_per_annulus: 4,
                max_segments: 100000,
                max_step: dt / 4.0,
                max_steps: 1000,
            },
        };
        let mut escaped = 0.0;
        for _ in 0..count {
            let report = sphere
                .step_reactive_tabulated(&mut fractions, &network, dt, settings, &table)
                .unwrap();
            escaped += report.radiation.escaped_energy
                + report.dynamics.escaped_energy
                + report.dynamics.escaped_binding
                + report.dynamics.escaped_neutrinos;
            assert_eq!(report.dynamics.escaped_neutrinos, 0.0);
            assert!(
                (sphere.reactive_energy(&fractions, &network).unwrap() + escaped - initial).abs()
                    / initial.abs()
                    < 1e-12
            );
        }
        assert!(fractions.iter().all(|row| row[1] > 0.0));
        let c = sphere.cells[0];
        let temperature = network
            .mixture(&fractions[0])
            .unwrap()
            .temperature(c.density, c.energy - 0.5 * c.momentum.powi(2) / c.density)
            .unwrap();
        (temperature, fractions[0][1])
    };
    let coarse = evolve(0.01, 20);
    let fine = evolve(0.005, 40);
    assert!((coarse.0 / fine.0 - 1.0).abs() < 1e-6);
    assert!((coarse.1 / fine.1 - 1.0).abs() < 1e-3);
}

#[test]
fn reactive_exterior_inflow_closes_nuclear_and_gas_energy_ledgers() {
    use physics::astrophysics_spherical::{CompositionExterior, ReactiveBudget};
    let network = network();
    let mut rows = vec![vec![1.0, 0.0]; 8];
    let energy = network
        .mixture(&rows[0])
        .unwrap()
        .at(1000.0, 2e8)
        .unwrap()
        .internal_energy_density;
    let mut s = Sphere {
        cells: vec![
            Cell {
                density: 1000.0,
                momentum: 0.0,
                energy
            };
            8
        ],
        spacing: 1e6,
        gamma: 5.0 / 3.0,
        g: 1e-10,
        outer: Boundary::Outflow,
    };
    let exterior = CompositionExterior {
        fractions: vec![0.5, 0.5],
        cell: Cell {
            density: 2000.0,
            momentum: 0.0,
            energy: network
                .mixture(&[0.5, 0.5])
                .unwrap()
                .at(2000.0, 2e8)
                .unwrap()
                .internal_energy_density,
        },
    };
    let initial = s.reactive_energy(&rows, &network).unwrap();
    let budget = ReactiveBudget {
        hydro_max_step: 1e-7,
        hydro_steps: 100,
        burn: Budget {
            max_step: 1e-7,
            steps: 1000,
            fit_evaluations: 1000,
        },
    };
    let report = s
        .step_reactive_exterior(&mut rows, &network, 1e-6, budget, &exterior)
        .unwrap();
    assert!(report.escaped_species[1] < 0.0 && report.escaped_binding > 0.0);
    let final_energy = s.reactive_energy(&rows, &network).unwrap()
        + report.escaped_energy
        + report.escaped_binding
        + report.escaped_neutrinos;
    assert!((final_energy - initial).abs() / initial.abs() < 1e-12);
    let saved = s.clone();
    let saved_rows = rows.clone();
    assert!(s
        .step_reactive_exterior(
            &mut rows,
            &network,
            1e-6,
            ReactiveBudget {
                burn: Budget {
                    steps: 12,
                    fit_evaluations: 12,
                    ..budget.burn
                },
                ..budget
            },
            &exterior
        )
        .is_err());
    assert_eq!(s, saved);
    assert_eq!(rows, saved_rows);
}

#[test]
fn radiating_exterior_inflow_conserves_energy_and_rolls_back_late_ray_failure() {
    use physics::astrophysics_spherical::CompositionExterior;
    let network = network();
    let mut rows = vec![vec![1.0, 0.0]; 8];
    let energy = network
        .mixture(&rows[0])
        .unwrap()
        .at(1000.0, 2e8)
        .unwrap()
        .internal_energy_density;
    let mut s = Sphere {
        cells: vec![
            Cell {
                density: 1000.0,
                momentum: 0.0,
                energy
            };
            8
        ],
        spacing: 1e6,
        gamma: 5.0 / 3.0,
        g: 1e-10,
        outer: Boundary::Outflow,
    };
    let exterior = CompositionExterior {
        fractions: vec![0.5, 0.5],
        cell: Cell {
            density: 2000.0,
            momentum: 0.0,
            energy: network
                .mixture(&[0.5, 0.5])
                .unwrap()
                .at(2000.0, 2e8)
                .unwrap()
                .internal_energy_density,
        },
    };
    let initial = s.reactive_energy(&rows, &network).unwrap();
    use physics::astrophysics_spherical_radiation::{Heating, ReactiveSettings};
    let settings = ReactiveSettings {
        hydro_max_step: 1e-7,
        hydro_steps: 100,
        burn: Budget {
            max_step: 1e-7,
            steps: 1000,
            fit_evaluations: 1000,
        },
        radiation: Heating {
            specific_heat: 1.0,
            opacity: 1e-20,
            ambient: 0.0,
            rays_per_annulus: 2,
            max_segments: 10000,
            max_step: 1e-6,
            max_steps: 100,
        },
    };
    use physics::astrophysics_opacity::{Kind, Table};
    let original = s.clone();
    let original_rows = rows.clone();
    let table = Table::new(
        Kind::GreyAbsorption,
        vec![500.0, 3000.0],
        vec![1e8, 4e8],
        vec![1e-20; 4],
    )
    .unwrap();
    let mut tabulated = original.clone();
    let mut tabulated_rows = original_rows.clone();
    let tabulated_report = tabulated
        .step_reactive_radiating_exterior(
            &mut tabulated_rows,
            &network,
            1e-6,
            settings,
            &exterior,
            Some(&table),
        )
        .unwrap();
    let report = s
        .step_reactive_radiating_exterior(&mut rows, &network, 1e-6, settings, &exterior, None)
        .unwrap();
    assert!(report.dynamics.escaped_species[1] < 0.0 && report.dynamics.escaped_binding > 0.0);
    let final_energy = s.reactive_energy(&rows, &network).unwrap()
        + report.dynamics.escaped_energy
        + report.dynamics.escaped_binding
        + report.dynamics.escaped_neutrinos
        + report.radiation.escaped_energy;
    assert!((final_energy - initial).abs() / initial.abs() < 1e-12);
    for (a, b) in s.cells.iter().zip(&tabulated.cells) {
        assert!((a.energy - b.energy).abs() / a.energy.abs() < 1e-12);
        assert!((a.density - b.density).abs() / a.density < 1e-12);
    }
    assert_eq!(rows, tabulated_rows);
    assert!(
        (report.radiation.escaped_energy - tabulated_report.radiation.escaped_energy).abs()
            / report.radiation.escaped_energy.abs()
            < 1e-12
    );
    // The first radiation half is in range; inflow pushes an outer density above
    // the table ceiling before the second half validates opacity.
    let narrow = Table::new(
        Kind::GreyAbsorption,
        vec![500.0, 1000.0],
        vec![1e8, 4e8],
        vec![1e-20; 4],
    )
    .unwrap();
    let mut rejected = original.clone();
    let mut rejected_rows = original_rows.clone();
    let failure = rejected.step_reactive_radiating_exterior(
        &mut rejected_rows,
        &network,
        1e-6,
        settings,
        &exterior,
        Some(&narrow),
    );
    assert!(matches!(
        failure,
        Err(
            physics::astrophysics_spherical_radiation::ReactiveError::Radiation(
                physics::astrophysics_spherical_radiation::Error::Opacity(
                    physics::astrophysics_opacity::Error::OutsideDomain
                )
            )
        )
    ));
    assert_eq!(rejected, original);
    assert_eq!(rejected_rows, original_rows);
    let saved = s.clone();
    let saved_rows = rows.clone();
    assert!(s
        .step_reactive_radiating_exterior(
            &mut rows,
            &network,
            1e-6,
            ReactiveSettings {
                radiation: Heating {
                    max_segments: 144,
                    ..settings.radiation
                },
                ..settings
            },
            &exterior,
            None
        )
        .is_err());
    assert_eq!(s, saved);
    assert_eq!(rows, saved_rows);
}

#[test]
fn polytrope_exterior_burning_and_tabulated_radiation_share_one_energy_ledger() {
    use physics::{
        astrophysics_opacity::{Kind, Table},
        astrophysics_spherical::CompositionExterior,
        astrophysics_spherical_radiation::{Heating, ReactiveSettings},
        astrophysics_star::{lane_emden, Scaling},
    };
    let network = network();
    let profile = lane_emden(1.5, 0.001, 5.0, 6000).unwrap();
    let fractions = vec![1.0, 0.0];
    let mixture = network.mixture(&fractions).unwrap();
    let state = mixture.at(1000.0, 2e8).unwrap();
    let scaling = Scaling::new(
        1.5,
        1000.0,
        state.gas_pressure + state.radiation_pressure,
        1e-10,
    )
    .unwrap();
    let mut sphere = Sphere {
        cells: vec![
            Cell {
                density: 1000.0,
                momentum: 0.0,
                energy: state.internal_energy_density
            };
            16
        ],
        spacing: scaling.length * profile.surface.unwrap().xi * 0.5 / 16.0,
        gamma: 5.0 / 3.0,
        g: 1e-10,
        outer: Boundary::Outflow,
    };
    let mut rows = vec![fractions.clone(); 16];
    sphere
        .initialize_polytrope(&rows, &network, &profile, scaling, 16)
        .unwrap();
    let a = sphere.spacing * 16.0 / scaling.length;
    let avg = profile
        .shell_average(a, a + sphere.spacing / scaling.length, 16)
        .unwrap();
    let density = scaling.central_density * avg.density_over_central;
    let temperature = mixture
        .temperature_from_pressure(
            density,
            scaling.central_pressure * avg.pressure_over_central,
        )
        .unwrap();
    let exterior = CompositionExterior {
        fractions,
        cell: Cell {
            density,
            momentum: 0.0,
            energy: mixture
                .at(density, temperature)
                .unwrap()
                .internal_energy_density,
        },
    };
    let table = Table::new(
        Kind::GreyAbsorption,
        vec![1.0, 1e5],
        vec![1e8, 4e8],
        vec![1e-20; 4],
    )
    .unwrap();
    let settings = ReactiveSettings {
        hydro_max_step: 1e-7,
        hydro_steps: 100,
        burn: Budget {
            max_step: 1e-7,
            steps: 10000,
            fit_evaluations: 10000,
        },
        radiation: Heating {
            specific_heat: 1.0,
            opacity: 1e-20,
            ambient: 0.0,
            rays_per_annulus: 2,
            max_segments: 100000,
            max_step: 1e-6,
            max_steps: 100,
        },
    };
    let initial = sphere.reactive_energy(&rows, &network).unwrap();
    let mut escaped = 0.0;
    for _ in 0..3 {
        let report = sphere
            .step_reactive_radiating_exterior(
                &mut rows,
                &network,
                1e-6,
                settings,
                &exterior,
                Some(&table),
            )
            .unwrap();
        assert!(report.dynamics.deposited_energy > 0.0);
        assert!(report.radiation.escaped_energy > 0.0);
        escaped += report.dynamics.escaped_energy
            + report.dynamics.escaped_binding
            + report.dynamics.escaped_neutrinos
            + report.radiation.escaped_energy;
        assert!(
            (sphere.reactive_energy(&rows, &network).unwrap() + escaped - initial).abs()
                / initial.abs()
                < 1e-12
        );
    }
    assert!(rows.iter().all(|row| row[1] > 0.0));
    let saved = sphere.clone();
    let saved_rows = rows.clone();
    assert!(sphere
        .step_reactive_radiating_exterior(
            &mut rows,
            &network,
            1e-6,
            ReactiveSettings {
                radiation: Heating {
                    max_segments: 544,
                    ..settings.radiation
                },
                ..settings
            },
            &exterior,
            Some(&table)
        )
        .is_err());
    assert_eq!(sphere, saved);
    assert_eq!(rows, saved_rows);
}

#[test]
fn balanced_burning_sphere_closed_and_open_preserves_complete_energy() {
    use physics::astrophysics_spherical::{CompositionExterior, ReactiveBudget};
    let network = network();
    for boundary in [Boundary::Reflecting, Boundary::Outflow] {
        let mut rows = vec![vec![1.0, 0.0]; 8];
        let mix = network.mixture(&rows[0]).unwrap();
        let state = mix.at(1000.0, 2e8).unwrap();
        let surface = state.gas_pressure + state.radiation_pressure;
        let mut s = Sphere {
            cells: vec![
                Cell {
                    density: 1000.0,
                    momentum: 0.0,
                    energy: state.internal_energy_density
                };
                8
            ],
            spacing: 1e6,
            gamma: 5.0 / 3.0,
            g: 1e-10,
            outer: boundary,
        };
        let external_mix = network.mixture(&[0.5, 0.5]).unwrap();
        let external_t = external_mix
            .temperature_from_pressure(1500.0, surface)
            .unwrap();
        let exterior = CompositionExterior {
            fractions: vec![0.5, 0.5],
            cell: Cell {
                density: 1500.0,
                momentum: 0.0,
                energy: external_mix
                    .at(1500.0, external_t)
                    .unwrap()
                    .internal_energy_density,
            },
        };
        let reference = if boundary == Boundary::Outflow {
            s.initialize_balanced_exterior(&rows, &network, surface, &exterior)
                .unwrap()
        } else {
            s.initialize_balanced(&rows, &network, surface).unwrap()
        };
        let exterior = if boundary == Boundary::Outflow {
            Some(&exterior)
        } else {
            None
        };
        let budget = ReactiveBudget {
            hydro_max_step: 1e-7,
            hydro_steps: 100,
            burn: Budget {
                max_step: 1e-7,
                steps: 1000,
                fit_evaluations: 1000,
            },
        };
        let initial = s.reactive_energy(&rows, &network).unwrap();
        let before = s.clone();
        let report = s
            .step_reactive_balanced(&mut rows, &network, 1e-6, budget, &reference, exterior)
            .unwrap();
        assert!(report.deposited_energy > 0.0);
        assert!(rows.iter().all(|r| r[1] > 0.0));
        assert_ne!(s, before);
        let total = s.reactive_energy(&rows, &network).unwrap()
            + report.escaped_energy
            + report.escaped_binding
            + report.escaped_neutrinos;
        assert!((total - initial).abs() / initial.abs() < 1e-12);
        let saved = s.clone();
        let saved_rows = rows.clone();
        let failure = s.step_reactive_balanced(
            &mut rows,
            &network,
            1e-6,
            ReactiveBudget {
                burn: Budget {
                    steps: 13,
                    fit_evaluations: 13,
                    ..budget.burn
                },
                ..budget
            },
            &reference,
            exterior,
        );
        assert!(failure.is_err());
        assert_eq!(s, saved);
        assert_eq!(rows, saved_rows);
    }
}

#[test]
fn balanced_radiating_burning_sphere_energy_and_late_failure() {
    use physics::astrophysics_opacity::{Kind, Table};
    use physics::astrophysics_spherical::{CompositionExterior, ReactiveBudget};
    use physics::astrophysics_spherical_radiation::{Heating, ReactiveSettings};
    let network = network();
    for boundary in [Boundary::Reflecting, Boundary::Outflow] {
        let mut rows = vec![vec![1.0, 0.0]; 8];
        let mix = network.mixture(&rows[0]).unwrap();
        let state = mix.at(1000.0, 2e8).unwrap();
        let surface = state.gas_pressure + state.radiation_pressure;
        let mut s = Sphere {
            cells: vec![
                Cell {
                    density: 1000.0,
                    momentum: 0.0,
                    energy: state.internal_energy_density
                };
                8
            ],
            spacing: 1e6,
            gamma: 5.0 / 3.0,
            g: 1e-10,
            outer: boundary,
        };
        let external_mix = network.mixture(&[0.5, 0.5]).unwrap();
        let external_t = external_mix
            .temperature_from_pressure(1500.0, surface)
            .unwrap();
        let exterior = CompositionExterior {
            fractions: vec![0.5, 0.5],
            cell: Cell {
                density: 1500.0,
                momentum: 0.0,
                energy: external_mix
                    .at(1500.0, external_t)
                    .unwrap()
                    .internal_energy_density,
            },
        };
        let reference = if boundary == Boundary::Outflow {
            s.initialize_balanced_exterior(&rows, &network, surface, &exterior)
                .unwrap()
        } else {
            s.initialize_balanced(&rows, &network, surface).unwrap()
        };
        let exterior = if boundary == Boundary::Outflow {
            Some(&exterior)
        } else {
            None
        };
        let budget = ReactiveBudget {
            hydro_max_step: 1e-7,
            hydro_steps: 100,
            burn: Budget {
                max_step: 1e-7,
                steps: 1000,
                fit_evaluations: 1000,
            },
        };
        let settings = ReactiveSettings {
            hydro_max_step: budget.hydro_max_step,
            hydro_steps: budget.hydro_steps,
            burn: budget.burn,
            radiation: Heating {
                specific_heat: 1.0,
                opacity: 1e-20,
                ambient: 0.0,
                rays_per_annulus: 2,
                max_segments: 10000,
                max_step: 1e-6,
                max_steps: 100,
            },
        };
        let table = Table::new(
            Kind::GreyAbsorption,
            vec![500.0, 2000.0],
            vec![1e8, 4e8],
            vec![1e-20; 4],
        )
        .unwrap();
        let table = if boundary == Boundary::Outflow {
            Some(&table)
        } else {
            None
        };
        let initial = s.reactive_energy(&rows, &network).unwrap();
        let before = s.clone();
        let report = s
            .step_reactive_radiating_balanced(
                &mut rows, &network, 1e-6, settings, &reference, exterior, table,
            )
            .unwrap();
        assert!(report.dynamics.deposited_energy > 0.0);
        assert!(rows.iter().all(|r| r[1] > 0.0));
        assert_ne!(s, before);
        let total = s.reactive_energy(&rows, &network).unwrap()
            + report.dynamics.escaped_energy
            + report.dynamics.escaped_binding
            + report.dynamics.escaped_neutrinos
            + report.radiation.escaped_energy;
        assert!((total - initial).abs() / initial.abs() < 1e-12);
        let saved = s.clone();
        let saved_rows = rows.clone();
        let failure = s.step_reactive_radiating_balanced(
            &mut rows,
            &network,
            1e-6,
            ReactiveSettings {
                radiation: Heating {
                    max_segments: 144,
                    ..settings.radiation
                },
                ..settings
            },
            &reference,
            exterior,
            table,
        );
        assert!(failure.is_err());
        assert_eq!(s, saved);
        assert_eq!(rows, saved_rows);
    }
}

#[test]
fn instantaneous_thermal_balance_closes_local_and_global_power_without_mutation() {
    use physics::astrophysics_spherical_radiation::Heating;
    let network = network();
    let rows = vec![vec![1.0, 0.0]; 8];
    let state = network.mixture(&rows[0]).unwrap().at(1000.0, 2e8).unwrap();
    let s = Sphere {
        cells: vec![
            Cell {
                density: 1000.0,
                momentum: 0.0,
                energy: state.internal_energy_density
            };
            8
        ],
        spacing: 1e6,
        gamma: 5.0 / 3.0,
        g: 1e-10,
        outer: Boundary::Outflow,
    };
    let saved = s.clone();
    let saved_rows = rows.clone();
    let settings = Heating {
        specific_heat: 1.0,
        opacity: 1e-20,
        ambient: 0.0,
        rays_per_annulus: 2,
        max_segments: 10000,
        max_step: 1e-6,
        max_steps: 100,
    };
    let rates = s.thermal_rates(settings, &rows, &network, None, 8).unwrap();
    assert_eq!(rates.fit_evaluations, 8);
    assert_eq!(rates.segments, 144);
    assert!(rates.nuclear.iter().all(|p| *p > 0.0));
    assert!(rates.radiation.iter().all(|p| *p < 0.0));
    let nuclear: f64 = rates.nuclear.iter().sum();
    let net: f64 = rates.net.iter().sum();
    assert!((net + rates.luminosity - nuclear).abs() / nuclear < 1e-12);
    for i in 0..8 {
        assert_eq!(rates.net[i], rates.nuclear[i] + rates.radiation[i]);
    }
    assert!(s.thermal_rates(settings, &rows, &network, None, 7).is_err());
    assert_eq!(s, saved);
    assert_eq!(rows, saved_rows);
}

#[test]
fn thermal_residual_detects_local_imbalance_despite_global_cancellation() {
    use physics::astrophysics_spherical_radiation::ThermalRates;
    let mut rates = ThermalRates {
        nuclear: vec![3.0, 1.0],
        radiation: vec![-1.0, -3.0],
        net: vec![2.0, -2.0],
        neutrinos: vec![0.0; 2],
        luminosity: 4.0,
        segments: 0,
        fit_evaluations: 0,
    };
    assert_eq!(rates.net.iter().sum::<f64>(), 0.0);
    assert_eq!(rates.relative_imbalance().unwrap(), 0.5);
    rates.radiation = vec![-3.0, -1.0];
    rates.net = vec![0.0; 2];
    assert_eq!(rates.relative_imbalance().unwrap(), 0.0);
    rates.net[0] = 1.0;
    assert!(rates.relative_imbalance().is_err());
    rates.nuclear = vec![f64::MAX; 2];
    rates.radiation = vec![-f64::MAX; 2];
    rates.net = vec![0.0; 2];
    assert_eq!(rates.relative_imbalance().unwrap(), 0.0);
}

#[test]
fn thermal_search_balances_burning_shells_and_rolls_back_failed_searches() {
    use physics::astrophysics_spherical_radiation::{Heating, ThermalSearch, ThermalSearchError};
    let network = network();
    let rows = vec![vec![1.0, 0.0]; 2];
    let mut sphere = Sphere {
        cells: [1000.0, 1200.0]
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
        spacing: 1e6,
        gamma: 5.0 / 3.0,
        g: 0.0,
        outer: Boundary::Outflow,
    };
    let initial = sphere.clone();
    let heating = Heating {
        specific_heat: 1.0,
        opacity: 1e-17,
        ambient: 0.0,
        rays_per_annulus: 8,
        max_segments: 100_000,
        max_step: 1.0,
        max_steps: 1,
    };
    let search = ThermalSearch {
        min_temperature: 1.01e8,
        max_temperature: 9.99e8,
        relative_tolerance: 1e-8,
        max_sweeps: 8,
        max_evaluations: 1000,
        fit_evaluations: 2000,
    };
    let report = sphere
        .equilibrate_thermal(heating, &rows, &network, None, search)
        .unwrap();
    assert!(report.sweeps > 0);
    assert_eq!(report.segments, report.evaluations * 48);
    assert_eq!(report.fit_evaluations, report.evaluations * 2);
    let fresh = sphere
        .thermal_rates(heating, &rows, &network, None, 2)
        .unwrap();
    assert!(fresh.relative_imbalance().unwrap() <= search.relative_tolerance);
    assert!(fresh.nuclear.iter().all(|p| *p > 0.0));
    assert!(fresh.radiation.iter().all(|p| *p < 0.0));
    let nuclear = fresh.nuclear.iter().sum::<f64>();
    assert!((fresh.luminosity - nuclear).abs() / nuclear < 2e-8);
    for (old, new) in initial.cells.iter().zip(&sphere.cells) {
        assert_eq!(old.density, new.density);
        assert_eq!(old.momentum, new.momentum);
        assert_ne!(old.energy, new.energy);
    }
    let table = physics::astrophysics_opacity::Table::new(
        physics::astrophysics_opacity::Kind::GreyAbsorption,
        vec![500.0, 2000.0],
        vec![1e8, 1e9],
        vec![heating.opacity; 4],
    )
    .unwrap();
    let mut tabulated = initial.clone();
    let table_report = tabulated
        .equilibrate_thermal(heating, &rows, &network, Some(&table), search)
        .unwrap();
    assert!(table_report.rates.relative_imbalance().unwrap() <= search.relative_tolerance);
    for (a, b) in sphere.cells.iter().zip(&tabulated.cells) {
        assert!((a.energy - b.energy).abs() / a.energy < 1e-8);
    }
    let balanced = sphere.clone();
    let unchanged = sphere
        .equilibrate_thermal(heating, &rows, &network, None, search)
        .unwrap();
    assert_eq!(unchanged.sweeps, 0);
    assert_eq!(sphere, balanced);
    for (settings, limits) in [
        (
            heating,
            ThermalSearch {
                max_evaluations: 2,
                ..search
            },
        ),
        (
            Heating {
                max_segments: 48,
                ..heating
            },
            search,
        ),
        (
            heating,
            ThermalSearch {
                fit_evaluations: 2,
                ..search
            },
        ),
    ] {
        let mut failed = initial.clone();
        assert!(failed
            .equilibrate_thermal(settings, &rows, &network, None, limits)
            .is_err());
        assert_eq!(failed, initial);
    }
    let mut failed = initial.clone();
    let error = failed
        .equilibrate_thermal(
            Heating {
                opacity: 1e-30,
                ..heating
            },
            &rows,
            &network,
            None,
            search,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ThermalSearchError::Unbracketed { shell: 0 }
    ));
    assert_eq!(failed, initial);
}

#[test]
fn joint_stellar_search_satisfies_hydrostatic_and_thermal_equations() {
    use physics::astrophysics_equilibrium::Search;
    use physics::astrophysics_spherical_radiation::Heating;
    let network = network();
    let rows = vec![vec![1.0, 0.0]; 2];
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
        spacing: 1e6,
        gamma: 5.0 / 3.0,
        g: 0.01,
        outer: Boundary::Reflecting,
    };
    let initial = sphere.clone();
    let heating = Heating {
        specific_heat: 1.0,
        opacity: 1e-17,
        ambient: 0.0,
        rays_per_annulus: 8,
        max_segments: 100_000,
        max_step: 1.0,
        max_steps: 1,
    };
    let search = Search {
        absolute_power_tolerance: 0.0,
        min_density: 100.0,
        max_density: 10000.0,
        min_temperature: 1.01e8,
        max_temperature: 9.99e8,
        relative_tolerance: 1e-8,
        iterations: 60,
        evaluations: 1000,
        fit_evaluations: 2000,
    };
    let surface = 2e17;
    let report = sphere
        .equilibrate_stellar(&rows, &network, surface, heating, None, search)
        .unwrap();
    assert!(report.iterations > 0);
    assert!(report.hydrostatic_residual <= search.relative_tolerance);
    assert!(report.thermal.relative_imbalance().unwrap() <= search.relative_tolerance);
    assert_eq!(report.segments, report.evaluations * 48);
    assert_eq!(report.fit_evaluations, report.evaluations * 2);
    let required = sphere.hydrostatic_pressures(surface).unwrap();
    for (i, cell) in sphere.cells.iter().enumerate() {
        let mixture = network.mixture(&rows[i]).unwrap();
        let t = mixture.temperature(cell.density, cell.energy).unwrap();
        let state = mixture.at(cell.density, t).unwrap();
        let pressure = state.gas_pressure + state.radiation_pressure;
        assert!((pressure / required.cells[i] - 1.0).abs() < 2e-8);
    }
    let fresh = sphere
        .thermal_rates(heating, &rows, &network, None, 2)
        .unwrap();
    assert!(fresh.relative_imbalance().unwrap() < 2e-8);
    assert!(fresh.nuclear.iter().all(|p| *p > 0.0));
    assert!(sphere.cells[0].density > sphere.cells[1].density);
    // Convert the converged profile into the exact discrete mechanical reference.
    let reference = sphere
        .initialize_balanced(&rows, &network, surface)
        .unwrap();
    let projected = sphere
        .thermal_rates(heating, &rows, &network, None, 2)
        .unwrap();
    assert!(projected.relative_imbalance().unwrap() < 1e-7);
    let stationary = sphere.clone();
    let mut transported = rows.clone();
    sphere
        .step_composition_balanced(&mut transported, &network, 0.001, 1e-5, 1000, &reference)
        .unwrap();
    assert_eq!(sphere, stationary);
    assert_eq!(transported, rows);

    let mut failed = initial.clone();
    assert!(failed
        .equilibrate_stellar(
            &rows,
            &network,
            surface,
            heating,
            None,
            Search {
                evaluations: 2,
                ..search
            }
        )
        .is_err());
    assert_eq!(failed, initial);
    let mut failed = initial.clone();
    assert!(failed
        .equilibrate_stellar(
            &rows,
            &network,
            surface,
            heating,
            None,
            Search {
                iterations: 1,
                ..search
            }
        )
        .is_err());
    assert_eq!(failed, initial);
}
