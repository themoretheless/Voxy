use physics::{
    astrophysics_eos::{Mixture, Species},
    astrophysics_gas::{Boundary, Cell},
    astrophysics_spherical::Sphere,
};
fn mixture() -> Mixture {
    Mixture::new(&[Species {
        mass_fraction: 1.0,
        mass_number: 1,
        nuclear_charge: 1,
    }])
    .unwrap()
}
fn sphere(rho: f64, t: f64) -> Sphere {
    let e = mixture().at(rho, t).unwrap().internal_energy_density;
    Sphere {
        cells: vec![
            Cell {
                density: rho,
                momentum: 0.0,
                energy: e
            };
            10
        ],
        spacing: 0.1,
        gamma: 5.0 / 3.0,
        g: 0.0,
        outer: Boundary::Reflecting,
    }
}
#[test]
fn constant_radiation_pressure_does_not_drive_spurious_flow() {
    let mut s = sphere(1e-3, 1e7);
    let before = s.clone();
    s.step_ionized(1e-9, 1e-10, 100, mixture()).unwrap();
    for (a, b) in s.cells.iter().zip(before.cells) {
        assert!((a.density - b.density).abs() / b.density < 1e-12);
        assert!(a.momentum.abs() < 1e-6);
        assert!((a.energy - b.energy).abs() / b.energy < 1e-12);
    }
}
#[test]
fn ionized_gas_limit_matches_gamma_five_thirds_dynamics() {
    let mut s = sphere(1.0, 1000.0);
    s.cells[3].energy *= 0.9;
    let mut gamma = s.clone();
    s.step_ionized(1e-5, 1e-6, 100, mixture()).unwrap();
    gamma.step(1e-5, 1e-6, 100).unwrap();
    for (a, b) in s.cells.iter().zip(gamma.cells) {
        assert!((a.density - b.density).abs() < 1e-10);
        assert!((a.energy - b.energy).abs() / b.energy < 1e-10);
        assert!((a.momentum - b.momentum).abs() < 1e-7);
    }
}
#[test]
fn radiation_pressure_gradient_evolves_conservatively() {
    let mut s = sphere(1e-3, 1e7);
    s.g = 1e15;
    for (i, c) in s.cells.iter_mut().enumerate() {
        c.energy = mixture()
            .at(c.density, 1e7 * (1.0 - 0.02 * i as f64))
            .unwrap()
            .internal_energy_density;
    }
    let initial = s.energy().unwrap();
    let mass = s.totals().unwrap()[0];
    s.step_ionized(1e-9, 1e-10, 100, mixture()).unwrap();
    assert!((s.energy().unwrap() - initial).abs() / initial.abs() < 1e-12);
    assert!((s.totals().unwrap()[0] - mass).abs() < 1e-14);
    assert!(s.cells.iter().any(|c| c.momentum.abs() > 1.0));
    let before = s.clone();
    assert!(s.step_ionized(1e-9, 1e-10, 1, mixture()).is_err());
    assert_eq!(s, before);
}

#[test]
fn local_composition_eos_matches_uniform_and_drives_stratified_gas() {
    use physics::astrophysics_nuclear::{Network, Nucleus};
    let network = Network {
        nuclei: vec![
            Nucleus {
                mass_number: 1,
                charge: 1,
                binding_energy: 0.0,
            },
            Nucleus {
                mass_number: 4,
                charge: 2,
                binding_energy: 0.0,
            },
        ],
        reactions: vec![],
    };
    let mut uniform = sphere(1.0, 1e7);
    uniform.cells[3].energy *= 0.9;
    let mut reference = uniform.clone();
    let mut fractions = vec![vec![1.0, 0.0]; 10];
    uniform
        .step_composition_ionized(&mut fractions, &network, 1e-9, 1e-10, 100)
        .unwrap();
    reference.step_ionized(1e-9, 1e-10, 100, mixture()).unwrap();
    assert_eq!(uniform, reference);
    assert!(
        fractions
            .iter()
            .all(|row| row[1] == 0.0 && (row[0] - 1.0).abs() < 1e-12)
    );

    let mut stratified = sphere(1.0, 1e7);
    let original = stratified.clone();
    let mut rows: Vec<_> = (0..10)
        .map(|i| {
            if i < 5 {
                vec![1.0, 0.0]
            } else {
                vec![0.0, 1.0]
            }
        })
        .collect();
    let energy = stratified.energy().unwrap();
    stratified
        .step_composition_ionized(&mut rows, &network, 1e-9, 1e-10, 100)
        .unwrap();
    assert!(stratified.cells.iter().any(|c| c.momentum.abs() > 1e-3));
    assert!((stratified.energy().unwrap() - energy).abs() / energy < 1e-12);
    let saved_rows = rows.clone();
    let saved = stratified.clone();
    assert!(
        stratified
            .step_composition_ionized(&mut rows, &network, 1e-9, 1e-10, 1)
            .is_err()
    );
    assert_eq!(stratified, saved);
    assert_eq!(rows, saved_rows);
    assert_ne!(stratified, original);
}

#[test]
fn hydrostatic_residual_matches_uniform_gravity_and_refines_equilibrium() {
    use physics::astrophysics_nuclear::{Network, Nucleus};
    let network = Network {
        nuclei: vec![Nucleus {
            mass_number: 1,
            charge: 1,
            binding_energy: 0.0,
        }],
        reactions: vec![],
    };
    let uniform = sphere(1.0, 1000.0);
    let rows = vec![vec![1.0]; uniform.cells.len()];
    assert!(
        uniform
            .hydrostatic_residual(&rows, &network)
            .unwrap()
            .iter()
            .all(|a| a.abs() < 1e-12)
    );
    let make = |n: usize| {
        let mut s = uniform.clone();
        s.g = 0.1;
        s.spacing = 1.0 / n as f64;
        s.cells = (0..n)
            .map(|i| {
                let a = i as f64 * s.spacing;
                let b = a + s.spacing;
                let mean_r2 = 0.6 * (b.powi(5) - a.powi(5)) / (b.powi(3) - a.powi(3));
                let pressure = 10.0 - 2.0 * std::f64::consts::PI * s.g / 3.0 * mean_r2;
                // Low-temperature fully ionized gas limit, radiation negligible.
                Cell {
                    density: 1.0,
                    momentum: 0.0,
                    energy: 1.5 * pressure,
                }
            })
            .collect();
        s
    };
    let error = |n| {
        let s = make(n);
        let residual = s
            .hydrostatic_residual(&vec![vec![1.0]; n], &network)
            .unwrap();
        // Existing reflecting face closure is not a hydrostatic atmosphere.
        residual[2..n - 2]
            .iter()
            .map(|a| a.abs())
            .fold(0.0, f64::max)
    };
    assert!(error(80) < error(40) * 0.6);
}

#[test]
fn static_residual_matches_initial_momentum_derivative() {
    use physics::astrophysics_nuclear::{Network, Nucleus};
    let network = Network {
        nuclei: vec![Nucleus {
            mass_number: 1,
            charge: 1,
            binding_energy: 0.0,
        }],
        reactions: vec![],
    };
    let mut initial = sphere(1.0, 1000.0);
    initial.g = 0.1;
    for (i, cell) in initial.cells.iter_mut().enumerate() {
        cell.energy *= 1.0 + 0.01 * i as f64;
    }
    let rows = vec![vec![1.0]; initial.cells.len()];
    let residual = initial.hydrostatic_residual(&rows, &network).unwrap();
    let derivative_error = |dt| {
        let mut evolved = initial.clone();
        let mut fractions = rows.clone();
        evolved
            .step_composition_ionized(&mut fractions, &network, dt, dt, 10)
            .unwrap();
        evolved
            .cells
            .iter()
            .zip(&initial.cells)
            .zip(&residual)
            .map(|((after, before), expected)| {
                (after.momentum / dt / before.density - expected).abs()
            })
            .fold(0.0, f64::max)
    };
    let coarse = derivative_error(1e-7);
    let fine = derivative_error(5e-8);
    assert!(fine < coarse * 0.6);
    assert!(fine / residual.iter().map(|a| a.abs()).fold(0.0, f64::max) < 1e-2);
}

#[test]
fn uniform_hydrostatic_initializer_refines_and_is_atomic() {
    use physics::astrophysics_nuclear::{Network, Nucleus};
    let network = Network {
        nuclei: vec![Nucleus {
            mass_number: 1,
            charge: 1,
            binding_energy: 0.0,
        }],
        reactions: vec![],
    };
    let residual = |n: usize| {
        let mut s = sphere(1.0, 1000.0);
        s.cells = vec![s.cells[0]; n];
        s.spacing = 1.0 / n as f64;
        s.g = 0.1;
        let rows = vec![vec![1.0]; n];
        s.initialize_uniform_hydrostatic(&rows, &network, 10.0)
            .unwrap();
        let virial = s.virial(&rows, &network, 10.0).unwrap();
        assert_eq!(virial.twice_kinetic, 0.0);
        assert!(virial.balance().abs() / virial.pressure < 1e-13);

        assert!(s.cells[0].energy > s.cells[n - 1].energy);
        let saved = s.clone();
        let mut invalid = rows.clone();
        invalid[n - 1][0] = 0.5;
        assert!(
            s.initialize_uniform_hydrostatic(&invalid, &network, 10.0)
                .is_err()
        );
        assert_eq!(s, saved);
        let acceleration = s.hydrostatic_residual(&rows, &network).unwrap();
        acceleration[2..n - 2]
            .iter()
            .map(|a| a.abs())
            .fold(0.0, f64::max)
    };
    assert!(residual(80) < residual(40) * 0.6);
}

#[test]
fn atmospheric_exterior_flux_conserves_mass_energy_and_rolls_back() {
    use physics::{astrophysics_atmosphere::Atmosphere, astrophysics_spherical::Exterior};
    let atmosphere = Atmosphere {
        gravity: 1e5,
        opacity: 0.01,
        effective_temperature: 1e5,
        top_gas_pressure: 1.0,
        mixture: mixture(),
    };
    let interior = atmosphere.gas_cell(1.0, 0.0).unwrap();
    let exterior = Exterior {
        cell: atmosphere.gas_cell(0.1, 0.0).unwrap(),
        mixture: mixture(),
    };
    let mut s = Sphere {
        cells: vec![interior; 10],
        spacing: 0.1,
        gamma: 5.0 / 3.0,
        g: 0.0,
        outer: Boundary::Outflow,
    };
    let initial_mass = s.totals().unwrap()[0];
    let initial_energy = s.energy().unwrap();
    let report = s
        .step_ionized_exterior(1e-9, 1e-10, 100, mixture(), exterior)
        .unwrap();
    assert!(report.escaped_mass > 0.0);
    assert!(
        (s.totals().unwrap()[0] + report.escaped_mass - initial_mass).abs() / initial_mass < 1e-12
    );
    assert!(
        (s.energy().unwrap() + report.escaped_energy - initial_energy).abs() / initial_energy
            < 1e-12
    );
    let saved = s.clone();
    assert!(
        s.step_ionized_exterior(1e-9, 1e-10, 2, mixture(), exterior)
            .is_err()
    );
    assert_eq!(s, saved);
    let mut reflected = saved;
    reflected.outer = Boundary::Reflecting;
    assert!(
        reflected
            .step_ionized_exterior(1e-9, 1e-10, 100, mixture(), exterior)
            .is_err()
    );
}

#[test]
fn exterior_inflow_uses_supplied_composition_and_conserves_species() {
    use physics::{
        astrophysics_nuclear::{Network, Nucleus},
        astrophysics_spherical::CompositionExterior,
    };
    let network = Network {
        nuclei: vec![
            Nucleus {
                mass_number: 1,
                charge: 1,
                binding_energy: 0.0,
            },
            Nucleus {
                mass_number: 4,
                charge: 2,
                binding_energy: 0.0,
            },
        ],
        reactions: vec![],
    };
    let mut s = sphere(1.0, 1000.0);
    s.outer = Boundary::Outflow;
    let mut rows = vec![vec![1.0, 0.0]; 10];
    let initial_mass = s.totals().unwrap()[0];
    let external = CompositionExterior {
        fractions: vec![0.0, 1.0],
        cell: Cell {
            density: 2.0,
            momentum: 0.0,
            energy: network
                .mixture(&[0.0, 1.0])
                .unwrap()
                .at(2.0, 1000.0)
                .unwrap()
                .internal_energy_density,
        },
    };
    let (exchange, escaped) = s
        .step_composition_exterior(&mut rows, &network, 1e-9, 1e-10, 100, &external)
        .unwrap();
    assert!(exchange.escaped_mass < 0.0 && escaped[1] < 0.0);
    let mut masses = [0.0; 2];
    for (i, (cell, row)) in s.cells.iter().zip(&rows).enumerate() {
        let a = i as f64 * s.spacing;
        let b = a + s.spacing;
        let mass = cell.density * 4.0 * std::f64::consts::PI / 3.0 * (b.powi(3) - a.powi(3));
        for j in 0..2 {
            masses[j] += mass * row[j];
        }
    }
    assert!((masses[0] + escaped[0] - initial_mass).abs() / initial_mass < 1e-12);
    assert!((masses[1] + escaped[1]).abs() / initial_mass < 1e-12);
    assert!(rows.last().unwrap()[1] > 0.0);
    let saved = s.clone();
    let saved_rows = rows.clone();
    assert!(
        s.step_composition_exterior(&mut rows, &network, 1e-9, 1e-10, 2, &external)
            .is_err()
    );
    assert_eq!(s, saved);
    assert_eq!(rows, saved_rows);
}

#[test]
fn stratified_hydrostatic_initializer_matches_uniform_and_virial() {
    use physics::astrophysics_nuclear::{Network, Nucleus};
    let network = Network {
        nuclei: vec![Nucleus {
            mass_number: 1,
            charge: 1,
            binding_energy: 0.0,
        }],
        reactions: vec![],
    };
    let rows = vec![vec![1.0]; 10];
    let mut uniform = sphere(1.0, 1000.0);
    uniform.g = 0.1;
    let pressures = uniform.hydrostatic_pressures(10.0).unwrap();
    assert_eq!(pressures.faces.len(), 11);
    for (i, pressure) in pressures.faces.iter().enumerate() {
        let r = i as f64 * uniform.spacing;
        let exact = 10.0 + 2.0 * std::f64::consts::PI / 3.0 * uniform.g * (1.0 - r * r);
        assert!((pressure / exact - 1.0).abs() < 1e-13);
    }
    for (i, mean) in pressures.cells.iter().enumerate() {
        assert!(*mean <= pressures.faces[i] && *mean >= pressures.faces[i + 1]);
    }
    let mut general = uniform.clone();
    uniform
        .initialize_uniform_hydrostatic(&rows, &network, 10.0)
        .unwrap();
    general
        .initialize_hydrostatic(&rows, &network, 10.0)
        .unwrap();
    for (a, b) in uniform.cells.iter().zip(&general.cells) {
        assert!((a.energy / b.energy - 1.0).abs() < 1e-13);
    }
    for (i, cell) in general.cells.iter_mut().enumerate() {
        cell.density = if i < 3 {
            10.0
        } else if i < 7 {
            3.0
        } else {
            1.0
        };
        cell.energy = mixture()
            .at(cell.density, 1000.0)
            .unwrap()
            .internal_energy_density;
        cell.momentum = cell.density * 0.1;
    }
    let densities: Vec<_> = general.cells.iter().map(|c| c.density).collect();
    general
        .initialize_hydrostatic(&rows, &network, 10.0)
        .unwrap();
    assert_eq!(
        densities,
        general.cells.iter().map(|c| c.density).collect::<Vec<_>>()
    );
    let v = general.virial(&rows, &network, 10.0).unwrap();
    assert_eq!(v.twice_kinetic, 0.0);
    assert!(v.balance().abs() / v.gravity.abs() < 1e-12);
    let saved = general.clone();
    let mut invalid = rows;
    invalid[0][0] = 0.5; // Reverse integration reaches this after all outer shells.
    assert!(
        general
            .initialize_hydrostatic(&invalid, &network, 10.0)
            .is_err()
    );
    assert_eq!(general, saved);
}

#[test]
fn local_composition_hydrostatic_pressure_matches_independent_quadrature() {
    use physics::astrophysics_nuclear::{Network, Nucleus};
    let network = Network {
        nuclei: vec![
            Nucleus {
                mass_number: 1,
                charge: 1,
                binding_energy: 0.0,
            },
            Nucleus {
                mass_number: 4,
                charge: 2,
                binding_energy: 0.0,
            },
        ],
        reactions: vec![],
    };
    let mut s = sphere(1.0, 1000.0);
    s.g = 0.1;
    let rows: Vec<_> = (0..10)
        .map(|i| {
            if i < 5 {
                vec![0.0, 1.0]
            } else {
                vec![1.0, 0.0]
            }
        })
        .collect();
    for (i, cell) in s.cells.iter_mut().enumerate() {
        cell.density = 10.0 - i as f64;
        cell.energy = network
            .mixture(&rows[i])
            .unwrap()
            .at(cell.density, 1000.0)
            .unwrap()
            .internal_energy_density;
    }
    s.initialize_hydrostatic(&rows, &network, 10.0).unwrap();
    // Midpoint quadrature of rho*G*M(r)/r², with the volume weighting
    // obtained by exchanging the pressure and shell-volume integrals.
    // No analytic pressure primitives from the initializer are used here.
    let mut edge_pressure = 10.0;
    for i in (0..10).rev() {
        let a = i as f64 * s.spacing;
        let b = a + s.spacing;
        let enclosed: f64 = s.cells[..i]
            .iter()
            .enumerate()
            .map(|(j, c)| {
                let inner = j as f64 * s.spacing;
                let outer = inner + s.spacing;
                c.density * 4.0 * std::f64::consts::PI / 3.0 * (outer.powi(3) - inner.powi(3))
            })
            .sum();
        let cell = s.cells[i];
        let mut full_drop = 0.0;
        let mut average_drop = 0.0;
        let dr = s.spacing / 10000.0;
        for j in 0..10000 {
            let r = a + (j as f64 + 0.5) * dr;
            let mass = enclosed
                + 4.0 * std::f64::consts::PI / 3.0 * cell.density * (r.powi(3) - a.powi(3));
            let increment = s.g * cell.density * mass / (r * r) * dr;
            full_drop += increment;
            average_drop += increment * (r.powi(3) - a.powi(3)) / (b.powi(3) - a.powi(3));
        }
        let mix = network.mixture(&rows[i]).unwrap();
        let temperature = mix.temperature(cell.density, cell.energy).unwrap();
        let state = mix.at(cell.density, temperature).unwrap();
        let pressure = state.gas_pressure + state.radiation_pressure;
        assert!((pressure / (edge_pressure + average_drop) - 1.0).abs() < 1e-8);
        edge_pressure += full_drop;
    }
    let virial = s.virial(&rows, &network, 10.0).unwrap();
    assert!(virial.balance().abs() / virial.gravity.abs() < 1e-12);
}

#[test]
fn polytrope_initializes_local_eos_mass_and_temperature_atomically() {
    use physics::{
        astrophysics_nuclear::{Network, Nucleus},
        astrophysics_star::{Scaling, lane_emden},
    };
    let network = Network {
        nuclei: vec![Nucleus {
            mass_number: 1,
            charge: 1,
            binding_energy: 0.0,
        }],
        reactions: vec![],
    };
    let profile = lane_emden(1.0, 0.001, 4.0, 5000).unwrap();
    let mut s = sphere(1.0, 1000.0);
    s.g = 6.67430e-11;
    let scaling = Scaling::new(1.0, 1e5, 1e17, s.g).unwrap();
    // Leave a small margin inside the surface to avoid roundoff domain overshoot.
    s.spacing = scaling.length * profile.surface.unwrap().xi * 0.999 / 10.0;
    let rows = vec![vec![1.0]; 10];
    s.initialize_polytrope(&rows, &network, &profile, scaling, 32)
        .unwrap();
    let mut previous = f64::INFINITY;
    for cell in &s.cells {
        let t = mixture().temperature(cell.density, cell.energy).unwrap();
        assert!(t < previous);
        assert_eq!(cell.momentum, 0.0);
        previous = t;
    }
    let endpoint = profile.sample(s.spacing * 10.0 / scaling.length).unwrap();
    let expected = scaling
        .point(profile.index, endpoint)
        .unwrap()
        .enclosed_mass;
    assert!((s.totals().unwrap()[0] / expected - 1.0).abs() < 1e-6);
    let saved = s.clone();
    let mut invalid = rows.clone();
    invalid[9][0] = 0.5;
    assert!(
        s.initialize_polytrope(&invalid, &network, &profile, scaling, 32)
            .is_err()
    );
    assert_eq!(s, saved);
    assert!(
        s.initialize_polytrope(
            &rows,
            &network,
            &profile,
            Scaling {
                length: scaling.length * 2.0,
                ..scaling
            },
            32
        )
        .is_err()
    );
    assert_eq!(s, saved);
}

#[test]
fn exterior_force_residual_matches_pressure_flux_and_initial_derivative() {
    use physics::{
        astrophysics_nuclear::{Network, Nucleus},
        astrophysics_spherical::CompositionExterior,
    };
    let network = Network {
        nuclei: vec![Nucleus {
            mass_number: 1,
            charge: 1,
            binding_energy: 0.0,
        }],
        reactions: vec![],
    };
    let mut s = sphere(1.0, 1000.0);
    s.outer = Boundary::Outflow;
    let rows = vec![vec![1.0]; 10];
    let exterior = CompositionExterior {
        fractions: vec![1.0],
        cell: Cell {
            density: 1.0,
            momentum: 0.0,
            energy: mixture().at(1.0, 2000.0).unwrap().internal_energy_density,
        },
    };
    let residual = s
        .hydrostatic_residual_exterior(&rows, &network, &exterior)
        .unwrap();
    assert!(residual[..9].iter().all(|a| a.abs() < 1e-8));
    let inside = mixture().at(1.0, 1000.0).unwrap();
    let outside = mixture().at(1.0, 2000.0).unwrap();
    let dp = outside.gas_pressure + outside.radiation_pressure
        - inside.gas_pressure
        - inside.radiation_pressure;
    let expected = -1.5 * dp / (1.0 - 0.9_f64.powi(3));
    assert!((residual[9] / expected - 1.0).abs() < 1e-12);
    let h = 1e-13;
    s.step_composition_exterior(&mut rows.clone(), &network, h, h, 10, &exterior)
        .unwrap();
    assert!((s.cells[9].momentum / h / residual[9] - 1.0).abs() < 1e-7);
    s.outer = Boundary::Reflecting;
    assert!(
        s.hydrostatic_residual_exterior(&rows, &network, &exterior)
            .is_err()
    );
}

#[test]
fn spherical_contact_transport_keeps_static_composition_and_gravity_ledgers() {
    use physics::{
        astrophysics_nuclear::{Network, Nucleus},
        astrophysics_spherical::CompositionExterior,
    };
    let network = Network {
        nuclei: vec![
            Nucleus {
                mass_number: 1,
                charge: 1,
                binding_energy: 0.0,
            },
            Nucleus {
                mass_number: 4,
                charge: 2,
                binding_energy: 0.0,
            },
        ],
        reactions: vec![],
    };
    let mut s = sphere(1.0, 1000.0);
    let mut rows: Vec<_> = (0..10)
        .map(|i| {
            if i < 5 {
                vec![1.0, 0.0]
            } else {
                vec![0.0, 1.0]
            }
        })
        .collect();
    for (i, cell) in s.cells.iter_mut().enumerate() {
        cell.density = if i < 5 { 1.0 } else { 2.0 };
        let mix = network.mixture(&rows[i]).unwrap();
        let t = mix.temperature_from_pressure(cell.density, 1e7).unwrap();
        cell.energy = mix.at(cell.density, t).unwrap().internal_energy_density;
    }
    let saved = s.clone();
    let saved_rows = rows.clone();
    let (report, species) = s
        .step_composition_contact(&mut rows, &network, 1e-7, 1e-8, 100, None)
        .unwrap();
    // EOS pressure inversion differs at floating-point roundoff between mixtures.
    for (a, b) in rows.iter().flatten().zip(saved_rows.iter().flatten()) {
        assert!((a - b).abs() < 1e-16);
    }
    for (a, b) in s.cells.iter().zip(&saved.cells) {
        assert!((a.density / b.density - 1.0).abs() < 1e-12);
        assert!((a.energy / b.energy - 1.0).abs() < 1e-12);
        assert!(a.momentum.abs() < 1e-12);
    }
    assert!(report.escaped_mass.abs() < 1e-16);
    assert!(species.iter().all(|x| x.abs() < 1e-16));
    s.g = 0.1;
    s.outer = Boundary::Outflow;
    let exterior = CompositionExterior {
        fractions: vec![0.0, 1.0],
        cell: Cell {
            density: 3.0,
            momentum: 0.0,
            energy: network
                .mixture(&[0.0, 1.0])
                .unwrap()
                .at(3.0, 2000.0)
                .unwrap()
                .internal_energy_density,
        },
    };
    let initial = s.energy().unwrap();
    let mass = s.totals().unwrap()[0];
    let (report, _) = s
        .step_composition_contact(&mut rows, &network, 1e-7, 1e-8, 100, Some(&exterior))
        .unwrap();
    assert!((s.energy().unwrap() + report.escaped_energy - initial).abs() / initial.abs() < 1e-12);
    assert!((s.totals().unwrap()[0] + report.escaped_mass - mass).abs() / mass < 1e-12);
    let saved = s.clone();
    let saved_rows = rows.clone();
    assert!(
        s.step_composition_contact(&mut rows, &network, 1e-7, 1e-8, 1, Some(&exterior))
            .is_err()
    );
    assert_eq!(s, saved);
    assert_eq!(rows, saved_rows);
}

#[test]
fn balanced_sphere_keeps_equilibrium_and_evolves_perturbations_conservatively() {
    use physics::astrophysics_nuclear::{Network, Nucleus};
    let network = Network {
        nuclei: vec![
            Nucleus {
                mass_number: 1,
                charge: 1,
                binding_energy: 0.0,
            },
            Nucleus {
                mass_number: 4,
                charge: 2,
                binding_energy: 0.0,
            },
        ],
        reactions: vec![],
    };
    let mut sphere = sphere(1.0, 1000.0);
    sphere.g = 0.1;
    let mut rows: Vec<_> = (0..10)
        .map(|i| {
            if i < 5 {
                vec![0.0, 1.0]
            } else {
                vec![1.0, 0.0]
            }
        })
        .collect();
    for (i, cell) in sphere.cells.iter_mut().enumerate() {
        cell.density = 10.0 - i as f64;
        cell.energy = network
            .mixture(&rows[i])
            .unwrap()
            .at(cell.density, 1000.0)
            .unwrap()
            .internal_energy_density;
    }
    let reference = sphere.initialize_balanced(&rows, &network, 10.0).unwrap();
    let equilibrium = sphere.clone();
    let equilibrium_rows = rows.clone();
    let report = sphere
        .step_composition_balanced(&mut rows, &network, 1.0, 0.001, 2000, &reference)
        .unwrap();
    assert!(report.steps >= 1000);
    assert_eq!(sphere, equilibrium);
    assert_eq!(rows, equilibrium_rows);
    sphere.cells[4].energy *= 1.001;
    let initial_energy = sphere.energy().unwrap();
    let initial_mass = sphere.totals().unwrap()[0];
    let perturbed = sphere.clone();
    sphere
        .step_composition_balanced(&mut rows, &network, 0.001, 0.0001, 100, &reference)
        .unwrap();
    assert_ne!(sphere, perturbed);
    assert!(sphere.cells.iter().any(|c| c.momentum != 0.0));
    assert!((sphere.energy().unwrap() - initial_energy).abs() / initial_energy.abs() < 1e-12);
    assert!((sphere.totals().unwrap()[0] - initial_mass).abs() / initial_mass < 1e-12);
    let saved = sphere.clone();
    let saved_rows = rows.clone();
    assert!(
        sphere
            .step_composition_balanced(&mut rows, &network, 0.01, 0.0001, 1, &reference)
            .is_err()
    );
    assert_eq!(sphere, saved);
    assert_eq!(rows, saved_rows);
}

#[test]
fn balanced_open_sphere_keeps_equilibrium_and_accounts_for_reservoir_inflow() {
    use physics::{
        astrophysics_nuclear::{Network, Nucleus},
        astrophysics_spherical::CompositionExterior,
    };
    let network = Network {
        nuclei: vec![
            Nucleus {
                mass_number: 1,
                charge: 1,
                binding_energy: 0.0,
            },
            Nucleus {
                mass_number: 4,
                charge: 2,
                binding_energy: 0.0,
            },
        ],
        reactions: vec![],
    };
    let mut s = sphere(1.0, 1000.0);
    s.g = 0.1;
    s.outer = Boundary::Outflow;
    let mut rows = vec![vec![1.0, 0.0]; 10];
    for (i, c) in s.cells.iter_mut().enumerate() {
        c.density = 10.0 - i as f64;
        c.energy = network
            .mixture(&rows[i])
            .unwrap()
            .at(c.density, 1000.0)
            .unwrap()
            .internal_energy_density;
    }
    let mix = network.mixture(&[0.5, 0.5]).unwrap();
    let t = mix.temperature_from_pressure(1.0, 10.0).unwrap();
    let mut exterior = CompositionExterior {
        fractions: vec![0.5, 0.5],
        cell: Cell {
            density: 1.0,
            momentum: 0.0,
            energy: mix.at(1.0, t).unwrap().internal_energy_density,
        },
    };
    let reference = s
        .initialize_balanced_exterior(&rows, &network, 10.0, &exterior)
        .unwrap();
    let saved = s.clone();
    let saved_rows = rows.clone();
    let (report, species) = s
        .step_composition_balanced_exterior(
            &mut rows, &network, 1.0, 0.001, 2000, &reference, &exterior,
        )
        .unwrap();
    assert_eq!(s, saved);
    assert_eq!(rows, saved_rows);
    assert_eq!(report.escaped_mass, 0.0);
    assert_eq!(report.escaped_energy, 0.0);
    assert!(species.iter().all(|m| *m == 0.0));
    exterior.cell.energy *= 1.01;
    let energy = s.energy().unwrap();
    let mass = s.totals().unwrap()[0];
    let (report, species) = s
        .step_composition_balanced_exterior(
            &mut rows, &network, 0.001, 0.0001, 100, &reference, &exterior,
        )
        .unwrap();
    assert!(report.escaped_mass < 0.0 && species[1] < 0.0);
    assert!((s.energy().unwrap() + report.escaped_energy - energy).abs() / energy.abs() < 1e-12);
    assert!((s.totals().unwrap()[0] + report.escaped_mass - mass).abs() / mass < 1e-12);
    let saved = s.clone();
    let saved_rows = rows.clone();
    assert!(
        s.step_composition_balanced_exterior(
            &mut rows, &network, 0.01, 0.0001, 1, &reference, &exterior
        )
        .is_err()
    );
    assert_eq!(s, saved);
    assert_eq!(rows, saved_rows);
}
