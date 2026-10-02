use physics::astrophysics_opacity::{Error, Kind, Table};
#[test]
fn power_law_values_and_derivatives_are_exact_in_log_space() {
    let rho = vec![0.1_f64, 1.0, 100.0];
    let temperatures = vec![1e4_f64, 1e6, 1e8];
    let law = |r: f64, t: f64| 0.2 * r.powf(0.7) * (t / 1e6).powf(-3.5);
    let values = rho
        .iter()
        .flat_map(|r| temperatures.iter().map(move |t| law(*r, *t)))
        .collect();
    let table = Table::new(Kind::GreyAbsorption, rho, temperatures, values).unwrap();
    assert!((table.maximum_temperature_slope() - 3.5).abs() < 1e-13);
    for r in [0.1, 0.4, 1.0, 17.0, 100.0] {
        for t in [1e4, 3e5, 1e6, 7e7, 1e8] {
            let state = table.at(r, t).unwrap();
            assert!((state.opacity / law(r, t) - 1.0).abs() < 1e-13);
            assert!((state.dln_opacity_dln_density - 0.7).abs() < 1e-13);
            assert!((state.dln_opacity_dln_temperature + 3.5).abs() < 1e-13);
        }
    }
}
#[test]
fn domain_and_invalid_tables_are_explicit() {
    let table = Table::new(
        Kind::RosselandTotal,
        vec![1.0, 2.0],
        vec![100.0, 200.0],
        vec![0.1; 4],
    )
    .unwrap();
    assert_eq!(table.at(0.5, 150.0), Err(Error::OutsideDomain));
    assert_eq!(table.at(1.5, 201.0), Err(Error::OutsideDomain));
    assert_eq!(table.at(f64::NAN, 150.0), Err(Error::InvalidInput));
    assert!(
        Table::new(
            Kind::GreyAbsorption,
            vec![2.0, 1.0],
            vec![100.0, 200.0],
            vec![0.1; 4]
        )
        .is_err()
    );
    assert!(
        Table::new(
            Kind::GreyAbsorption,
            vec![1.0, 2.0],
            vec![100.0, 200.0],
            vec![0.0; 4]
        )
        .is_err()
    );
    assert!(
        Table::new(
            Kind::GreyAbsorption,
            vec![1.0, 2.0],
            vec![100.0, 200.0],
            vec![0.1; 3]
        )
        .is_err()
    );
}

#[test]
fn slope_bound_includes_each_density_row_and_temperature_interval() {
    let table = Table::new(
        Kind::GreyAbsorption,
        vec![1.0, 2.0],
        vec![1.0, std::f64::consts::E, std::f64::consts::E.powi(2)],
        vec![1.0, 1.0, 1.0, 1.0, (-20_f64).exp(), (-18_f64).exp()],
    )
    .unwrap();
    assert!((table.maximum_temperature_slope() - 20.0).abs() < 1e-13);
    assert!(
        table
            .at(1.5, 2.0)
            .unwrap()
            .dln_opacity_dln_temperature
            .abs()
            <= table.maximum_temperature_slope()
    );
}

#[test]
fn csv_unordered_grid_and_invalid_nodes() {
    let csv = "density_kg_m3,temperature_K,opacity_m2_kg\n2,200,8\n1,100,1\n2,100,2\n1,200,4\n";
    let table = Table::from_csv(Kind::GreyAbsorption, csv, 4).unwrap();
    assert!((table.at(2.0, 200.0).unwrap().opacity - 8.0).abs() < 1e-14);
    assert_eq!(
        Table::from_csv(Kind::GreyAbsorption, csv, 3),
        Err(Error::BudgetExceeded)
    );
    assert!(Table::from_csv(Kind::GreyAbsorption, &csv.replace("1,200,4", "2,200,4"), 4).is_err());
    assert!(Table::from_csv(Kind::GreyAbsorption, &csv.replace("1,200,4\n", ""), 4).is_err());
    assert!(
        Table::from_csv(
            Kind::GreyAbsorption,
            &csv.replace("opacity_m2_kg", "opacity_cm2_g"),
            4
        )
        .is_err()
    );
}

#[test]
fn free_free_planck_mean_matches_independent_frequency_integral() {
    use physics::{
        astrophysics_eos::{BOLTZMANN, Species},
        astrophysics_opacity::FreeFree,
    };
    let model = FreeFree {
        gaunt_factor: 1.2,
        min_temperature: 1e5,
        max_temperature: 1e8,
    };
    let hydrogen = [Species {
        mass_fraction: 1.0,
        mass_number: 1,
        nuclear_charge: 1,
    }];
    let rho = 1.0;
    let temperature = 1e6;
    // Independent cgs reference: rho=0.001 g/cm³, ne=ni=rho/m_u(g).
    let reference = model.spectral(rho, temperature, 1e16, &hydrogen).unwrap();
    assert!((reference / 6.137_665_600_371_251 - 1.0).abs() < 1e-13);

    // Integrate kappa_nu * x³/(exp(x)-1) over dimensionless photon energy.
    let count = 20000;
    let dx = 40.0 / f64::from(count);
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for i in 0..count {
        let x = (f64::from(i) + 0.5) * dx;
        let frequency = x * BOLTZMANN * temperature / 6.626_070_15e-34;
        let weight = x.powi(3) / x.exp_m1();
        numerator += model
            .spectral(rho, temperature, frequency, &hydrogen)
            .unwrap()
            * weight
            * dx;
        denominator += weight * dx;
    }
    let analytic = model.planck(rho, temperature, &hydrogen).unwrap();
    assert!((numerator / denominator / analytic - 1.0).abs() < 2e-7);
    assert!((model.planck(2.0, temperature, &hydrogen).unwrap() / analytic - 2.0).abs() < 1e-12);
    assert!(
        (model.planck(rho, 2.0 * temperature, &hydrogen).unwrap() / analytic - 2_f64.powf(-3.5))
            .abs()
            < 1e-12
    );
    let helium = [Species {
        mass_fraction: 1.0,
        mass_number: 4,
        nuclear_charge: 2,
    }];
    assert!((model.planck(rho, temperature, &helium).unwrap() / analytic - 0.5).abs() < 1e-12);
    let table = model
        .planck_table(vec![0.1, 10.0], vec![1e5, 1e8], &hydrogen)
        .unwrap();
    assert_eq!(table.kind, Kind::PlanckAbsorption);
    assert!((table.at(rho, temperature).unwrap().opacity / analytic - 1.0).abs() < 1e-12);
    assert_eq!(model.planck(rho, 1e9, &hydrogen), Err(Error::OutsideDomain));
    assert!(model.spectral(rho, temperature, 0.0, &hydrogen).is_err());
    assert!(model.planck(rho, temperature, &[]).is_err());
}
