use physics::liquid::{water_saturation, water_saturation_temperature};

#[test]
fn inverse_pressure_recovers_temperatures_and_rejects_extrapolation() {
    for t in [273.16, 277., 300., 373.1243, 500., 630., 647.096] {
        let p = water_saturation(t).unwrap().pressure_pa;
        assert!((water_saturation_temperature(p).unwrap() - t).abs() < 2e-12);
    }
    assert!((water_saturation_temperature(101325.).unwrap() - 373.1243).abs() < 0.0001);
    let minimum = water_saturation(273.16).unwrap().pressure_pa;
    assert_eq!(water_saturation_temperature(minimum).unwrap(), 273.16);
    assert_eq!(water_saturation_temperature(22.064e6).unwrap(), 647.096);
    for p in [
        f64::NAN,
        f64::INFINITY,
        -1.,
        0.,
        minimum - 0.001,
        22.064e6 + 1.,
    ] {
        assert!(water_saturation_temperature(p).is_err());
    }
}

#[test]
fn matches_iapws_sr1_1992_published_table_one() {
    // IAPWS PDF page 7: rounded independent verification values, SI units.
    // T, p, dp/dT, rho_l, rho_v, h_l, h_v; tolerances reflect printed precision.
    for (t, expected, tolerance) in [
        (
            273.16,
            [611.657, 44.436693, 999.789, 0.00485426, 0.611786, 2500.5e3],
            [0.001, 0.000001, 0.001, 0.00000001, 0.00001, 50.],
        ),
        (
            373.1243,
            [101325., 3616., 958.365, 0.597586, 419.05e3, 2675.7e3],
            [0.5, 0.5, 0.001, 0.000001, 5., 50.],
        ),
        (
            647.096,
            [22.064e6, 268.0e3, 322., 322., 2086.6e3, 2086.6e3],
            [0.01, 500., 0.000001, 0.000001, 50., 50.],
        ),
    ] {
        let s = water_saturation(t).unwrap();
        let actual = [
            s.pressure_pa,
            s.pressure_derivative_pa_per_k,
            s.liquid_density_kg_per_m3,
            s.vapor_density_kg_per_m3,
            s.liquid_enthalpy_j_per_kg,
            s.vapor_enthalpy_j_per_kg,
        ];
        for i in 0..actual.len() {
            assert!(
                (actual[i] - expected[i]).abs() <= tolerance[i],
                "T={t}, column={i}, actual={}, expected={}",
                actual[i],
                expected[i]
            );
        }
    }
}

#[test]
fn pressure_derivative_and_clapeyron_relation_hold_across_domain() {
    for t in [280., 300., 373., 450., 550., 630.] {
        let s = water_saturation(t).unwrap();
        let derivative = (water_saturation(t + 0.001).unwrap().pressure_pa
            - water_saturation(t - 0.001).unwrap().pressure_pa)
            / 0.002;
        assert!((derivative / s.pressure_derivative_pa_per_k - 1.).abs() < 1e-8);
        let latent = t
            * s.pressure_derivative_pa_per_k
            * (1. / s.vapor_density_kg_per_m3 - 1. / s.liquid_density_kg_per_m3);
        assert!((s.vaporization_enthalpy_j_per_kg() - latent).abs() < 1e-8);
        assert!(s.liquid_density_kg_per_m3 > s.vapor_density_kg_per_m3);
        assert!(s.vaporization_enthalpy_j_per_kg() > 0.);
        assert!(s.liquid_internal_energy_j_per_kg().is_finite());
        assert!(s.vapor_internal_energy_j_per_kg().is_finite());
    }
    assert_eq!(
        water_saturation(647.096)
            .unwrap()
            .vaporization_enthalpy_j_per_kg(),
        0.
    );
}

#[test]
fn rejects_outside_liquid_vapor_coexistence_domain() {
    for t in [f64::NAN, f64::INFINITY, -1., 273.159, 647.097] {
        assert!(water_saturation(t).is_err());
    }
}
