use physics::{
    astrophysics_radiation::blackbody,
    astrophysics_spherical_radiation::{Shell, transfer},
};
fn shells(n: u32, absorption: f64) -> Vec<Shell> {
    (1..=n)
        .map(|i| Shell {
            outer_radius: f64::from(i) / f64::from(n),
            absorption,
            temperature: 500.0,
        })
        .collect()
}
#[test]
fn uniform_sphere_matches_analytic_luminosity_and_thick_limit() {
    for opacity in [0.1_f64, 1.0, 100.0] {
        let s = shells(20, opacity);
        let result = transfer(&s, 0.0, 64, 100000).unwrap();
        let tau = 2.0 * opacity;
        let fraction = 1.0 - 2.0 * (1.0 - (1.0 + tau) * (-tau).exp()) / (tau * tau);
        let exact = 4.0 * std::f64::consts::PI.powi(2) * blackbody(500.0).unwrap() * fraction;
        assert!(
            (result.luminosity - exact).abs() / exact < 2e-4,
            "{} {exact}",
            result.luminosity
        );
        assert!((result.heating.iter().sum::<f64>() + result.luminosity).abs() / exact < 1e-12);
    }
}
#[test]
fn equilibrium_transparency_and_core_sampling() {
    let mut s = shells(20, 0.1);
    let b = blackbody(500.0).unwrap();
    let r = transfer(&s, b, 8, 10000).unwrap();
    assert!(r.heating.iter().all(|p| p.abs() < 1e-10));
    let r = transfer(&s, 0.0, 8, 10000).unwrap();
    assert!(r.heating.iter().all(|p| *p < 0.0));
    for shell in &mut s {
        shell.absorption = 0.0;
    }
    let r = transfer(&s, b, 8, 10000).unwrap();
    assert_eq!(r.luminosity, 0.0);
    assert!(r.heating.iter().all(|p| *p == 0.0));
}
#[test]
fn ray_refinement_and_budgets() {
    let s = shells(10, 1.0);
    let tau = 2.0_f64;
    let exact = 4.0
        * std::f64::consts::PI.powi(2)
        * blackbody(500.0).unwrap()
        * (1.0 - 2.0 * (1.0 - (1.0 + tau) * (-tau).exp()) / tau.powi(2));
    let coarse = transfer(&s, 0.0, 2, 10000).unwrap();
    let fine = transfer(&s, 0.0, 32, 10000).unwrap();
    assert!((fine.luminosity - exact).abs() < (coarse.luminosity - exact).abs());
    assert!(transfer(&s, 0.0, 32, 1).is_err());
    assert!(transfer(&s, 0.0, 0, 10000).is_err());
}
#[test]
fn spherical_heating_preserves_energy_and_budget_is_atomic() {
    use physics::{
        astrophysics_gas::{Boundary, Cell},
        astrophysics_spherical::Sphere,
        astrophysics_spherical_radiation::Heating,
    };
    let initial = Sphere {
        cells: vec![Cell::from_primitive(1.0, 0.0, 200000.0, 1.4).unwrap(); 20],
        spacing: 0.05,
        gamma: 1.4,
        g: 0.0,
        outer: Boundary::Reflecting,
    };
    let settings = Heating {
        specific_heat: 1000.0,
        opacity: 0.1,
        ambient: 0.0,
        rays_per_annulus: 4,
        max_segments: 200000,
        max_step: 0.1,
        max_steps: 1000,
    };
    for ambient in [0.0, blackbody(600.0).unwrap()] {
        let mut sphere = initial.clone();
        let e = sphere
            .radiate(
                1.0,
                Heating {
                    ambient,
                    ..settings
                },
            )
            .unwrap();
        assert!(
            (sphere.energy().unwrap() + e.escaped_energy - initial.energy().unwrap()).abs() < 1e-7
        );
        if ambient == 0.0 {
            assert!(e.escaped_energy > 0.0);
        } else {
            assert!(e.escaped_energy < 0.0);
        }
    }
    let mut sphere = initial.clone();
    assert!(
        sphere
            .radiate(
                1.0,
                Heating {
                    max_segments: 2000,
                    ..settings
                }
            )
            .is_err()
    );
    assert_eq!(sphere, initial);
}

#[test]
fn different_compositions_at_equal_temperature_remain_in_radiative_equilibrium() {
    use physics::{
        astrophysics_gas::{Boundary, Cell},
        astrophysics_nuclear::{Network, Nucleus},
        astrophysics_spherical::Sphere,
        astrophysics_spherical_radiation::Heating,
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
    let fractions = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![0.3, 0.7]];
    let temperature = 1e6;
    let density = 1.0;
    let cells = fractions
        .iter()
        .map(|row| Cell {
            density,
            momentum: 0.0,
            energy: network
                .mixture(row)
                .unwrap()
                .at(density, temperature)
                .unwrap()
                .internal_energy_density,
        })
        .collect();
    let mut sphere = Sphere {
        cells,
        spacing: 1.0,
        gamma: 5.0 / 3.0,
        g: 0.0,
        outer: Boundary::Reflecting,
    };
    let original = sphere.clone();
    let radius = sphere
        .composition_optical_depth_radius(&fractions, &network, 2.0, None, 2.0 / 3.0)
        .unwrap()
        .unwrap();
    assert!((radius - (3.0 - 1.0 / 3.0)).abs() < 1e-13);
    let opacity = physics::astrophysics_opacity::Table::new(
        physics::astrophysics_opacity::Kind::GreyAbsorption,
        vec![0.5, 2.0],
        vec![5e5, 2e6],
        vec![2.0; 4],
    )
    .unwrap();
    let table_radius = sphere
        .composition_optical_depth_radius(&fractions, &network, 0.0, Some(&opacity), 2.0 / 3.0)
        .unwrap()
        .unwrap();
    assert!((radius - table_radius).abs() < 1e-13);
    assert_eq!(
        sphere
            .composition_optical_depth_radius(&fractions, &network, 0.0, None, 2.0 / 3.0)
            .unwrap(),
        None
    );
    assert_eq!(sphere, original);
    assert_ne!(sphere.cells[0].energy, sphere.cells[1].energy);
    let settings = Heating {
        specific_heat: 1.0,
        opacity: 0.5,
        ambient: blackbody(temperature).unwrap(),
        rays_per_annulus: 8,
        max_segments: 100000,
        max_step: 1e-10,
        max_steps: 100,
    };
    let exchange = sphere
        .radiate_composition(1e-9, settings, &fractions, &network)
        .unwrap();
    let initial = original.energy().unwrap();
    assert!(exchange.escaped_energy.abs() / initial < 1e-12);
    for (after, before) in sphere.cells.iter().zip(&original.cells) {
        assert!((after.energy - before.energy).abs() / before.energy < 1e-12);
        assert_eq!(after.density, before.density);
        assert_eq!(after.momentum, before.momentum);
    }
    let mut invalid = fractions.clone();
    invalid[1] = vec![0.0, 0.9];
    let saved = sphere.clone();
    assert!(
        sphere
            .radiate_composition(1e-9, settings, &invalid, &network)
            .is_err()
    );
    assert_eq!(sphere, saved);
}

#[test]
fn tabulated_absorption_matches_constant_and_domain_exit_rolls_back() {
    use physics::{
        astrophysics_gas::{Boundary, Cell},
        astrophysics_nuclear::{Network, Nucleus},
        astrophysics_opacity::{Kind, Table},
        astrophysics_spherical::Sphere,
        astrophysics_spherical_radiation::Heating,
    };
    let network = Network {
        nuclei: vec![Nucleus {
            mass_number: 1,
            charge: 1,
            binding_energy: 0.0,
        }],
        reactions: vec![],
    };
    let fractions = vec![vec![1.0]; 3];
    let energy = network
        .mixture(&fractions[0])
        .unwrap()
        .at(1.0, 1e6)
        .unwrap()
        .internal_energy_density;
    let original = Sphere {
        cells: vec![
            Cell {
                density: 1.0,
                momentum: 0.0,
                energy
            };
            3
        ],
        spacing: 1.0,
        gamma: 5.0 / 3.0,
        g: 0.0,
        outer: Boundary::Reflecting,
    };
    let settings = Heating {
        specific_heat: 1.0,
        opacity: 0.5,
        ambient: 0.0,
        rays_per_annulus: 8,
        max_segments: 100000,
        max_step: 1e-11,
        max_steps: 1000,
    };
    let table = Table::new(
        Kind::GreyAbsorption,
        vec![0.5, 2.0],
        vec![5e5, 2e6],
        vec![0.5; 4],
    )
    .unwrap();
    let scalar_rates = original
        .composition_radiation_rates(settings, &fractions, &network, None)
        .unwrap();
    let table_rates = original
        .composition_radiation_rates(settings, &fractions, &network, Some(&table))
        .unwrap();
    assert!((scalar_rates.luminosity / table_rates.luminosity - 1.0).abs() < 1e-13);
    assert!(
        (scalar_rates.heating.iter().sum::<f64>() + scalar_rates.luminosity).abs()
            / scalar_rates.luminosity
            < 1e-12
    );
    let mut tabulated = original.clone();
    let mut constant = original.clone();
    let a = tabulated
        .radiate_tabulated(1e-10, settings, &fractions, &network, &table)
        .unwrap();
    let b = constant
        .radiate_composition(1e-10, settings, &fractions, &network)
        .unwrap();
    for (a, b) in tabulated.cells.iter().zip(&constant.cells) {
        assert!((a.energy / b.energy - 1.0).abs() < 1e-13);
    }
    assert!((a.escaped_energy / b.escaped_energy - 1.0).abs() < 1e-13);
    assert!(
        (tabulated.energy().unwrap() + a.escaped_energy - original.energy().unwrap()).abs()
            / original.energy().unwrap()
            < 1e-12
    );
    let steep = Table::new(
        Kind::GreyAbsorption,
        vec![0.5, 2.0],
        vec![5e5, 1e6, 2e6],
        vec![1e-100, 0.5, 0.5, 1e-100, 0.5, 0.5],
    )
    .unwrap();
    let loose = Heating {
        max_step: 1e-8,
        ..settings
    };
    let mut steep_sphere = original.clone();
    let mut flat_sphere = original.clone();
    let steep_work = steep_sphere
        .radiate_tabulated(1e-9, loose, &fractions, &network, &steep)
        .unwrap();
    let flat_work = flat_sphere
        .radiate_tabulated(1e-9, loose, &fractions, &network, &table)
        .unwrap();
    assert!(steep_work.steps > flat_work.steps);
    assert!(
        (steep_sphere.energy().unwrap() + steep_work.escaped_energy - original.energy().unwrap())
            .abs()
            / original.energy().unwrap()
            < 1e-12
    );
    let narrow = Table::new(
        Kind::GreyAbsorption,
        vec![0.5, 2.0],
        vec![999999.99, 2e6],
        vec![0.5; 4],
    )
    .unwrap();
    let mut failed = original.clone();
    assert!(
        failed
            .radiate_tabulated(1e-10, settings, &fractions, &network, &narrow)
            .is_err()
    );
    assert_eq!(failed, original);
    let wrong_kind = Table::new(
        Kind::RosselandTotal,
        vec![0.5, 2.0],
        vec![5e5, 2e6],
        vec![0.5; 4],
    )
    .unwrap();
    assert!(
        failed
            .radiate_tabulated(1e-10, settings, &fractions, &network, &wrong_kind)
            .is_err()
    );
    assert_eq!(failed, original);
}

#[test]
fn radial_optical_depth_radius_uniform_layered_and_thin() {
    use physics::astrophysics_spherical_radiation::optical_depth_radius;
    let profile = shells(20, 2.0);
    let radius = optical_depth_radius(&profile, 2.0 / 3.0).unwrap().unwrap();
    assert!((radius - 2.0 / 3.0).abs() < 1e-14);
    assert_eq!(optical_depth_radius(&profile, 3.0).unwrap(), None);
    let profile = vec![
        Shell {
            outer_radius: 1.0,
            absorption: 2.0,
            temperature: 500.0,
        },
        Shell {
            outer_radius: 2.0,
            absorption: 0.0,
            temperature: 500.0,
        },
    ];
    assert!((optical_depth_radius(&profile, 1.0).unwrap().unwrap() - 0.5).abs() < 1e-14);
    assert_eq!(optical_depth_radius(&profile, 2.0).unwrap(), Some(0.0));
    assert!(optical_depth_radius(&profile, 0.0).is_err());
    let mut invalid = profile;
    invalid[0].absorption = -1.0;
    assert!(optical_depth_radius(&invalid, 1.0).is_err());
}

#[test]
fn effective_temperature_recovers_blackbody_and_handles_extreme_radius() {
    use physics::{
        astrophysics_spherical_radiation::effective_temperature,
        astrophysics_thermal::STEFAN_BOLTZMANN,
    };
    let radius = 7e8_f64;
    let temperature = 6000_f64;
    let luminosity =
        4.0 * std::f64::consts::PI * radius.powi(2) * STEFAN_BOLTZMANN * temperature.powi(4);
    assert!((effective_temperature(luminosity, radius).unwrap() / temperature - 1.0).abs() < 1e-13);
    assert_eq!(effective_temperature(0.0, radius).unwrap(), 0.0);
    assert!(effective_temperature(-1.0, radius).is_err());
    assert!(effective_temperature(1.0, 0.0).is_err());
    assert!(effective_temperature(1e300, 1e200).unwrap().is_finite());
}
