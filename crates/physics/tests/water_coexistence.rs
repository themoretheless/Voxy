use physics::liquid::water_coexistence;

#[test]
fn reproduces_iapws95_table_eight_coexistence_states() {
    // Official IAPWS R6-95(2018), Table 8: T, p(Pa), rho_l, rho_v, h_l, h_v, s_l, s_v (SI).
    for (t, p, rl, rv, hl, hv, sl, sv) in [
        (
            275.,
            698.451167,
            999.887406,
            0.00550664919,
            7759.72202,
            2504289.95,
            28.3094670,
            9106.60121,
        ),
        (
            450., 932203.564, 890.341250, 4.81200360, 749161.585, 2774410.78, 2108.65845,
            6609.21221,
        ),
        (
            625., 16908269.3, 567.090385, 118.290280, 1686269.76, 2550716.25, 3801.94683,
            5185.06121,
        ),
    ] {
        let s = water_coexistence(t).unwrap_or_else(|error| panic!("T={t}: {error:?}"));
        for (name, actual, expected) in [
            ("pressure", s.pressure_pa, p),
            ("liquid density", s.liquid_density_kg_per_m3, rl),
            ("vapor density", s.vapor_density_kg_per_m3, rv),
            ("liquid h", s.liquid.enthalpy_j_per_kg, hl),
            ("vapor h", s.vapor.enthalpy_j_per_kg, hv),
            ("liquid s", s.liquid.entropy_j_per_kg_k, sl),
            ("vapor s", s.vapor.entropy_j_per_kg_k, sv),
        ] {
            assert!(
                (actual / expected - 1.).abs() < 2e-7,
                "T={t}, {name}, actual={actual}, expected={expected}"
            );
        }
        assert!((s.liquid.pressure_pa - s.vapor.pressure_pa).abs() <= 1e-6 + 1e-12 * s.pressure_pa);
        let gl = s.liquid.enthalpy_j_per_kg - t * s.liquid.entropy_j_per_kg_k;
        let gv = s.vapor.enthalpy_j_per_kg - t * s.vapor.entropy_j_per_kg_k;
        assert!((gl - gv).abs() <= 1e-12 * 461.51805 * t);
        let latent = s.vapor.enthalpy_j_per_kg - s.liquid.enthalpy_j_per_kg;
        assert!(
            (latent - t * (s.vapor.entropy_j_per_kg_k - s.liquid.entropy_j_per_kg_k)).abs() < 1e-4
        );
        assert!(latent > 0.);
        assert!(s.liquid_density_kg_per_m3 > s.vapor_density_kg_per_m3);
    }
}

#[test]
fn rejects_ice_domain_and_unhandled_critical_endpoint() {
    for t in [f64::NAN, f64::INFINITY, 273., 647.096, 700.] {
        assert!(water_coexistence(t).is_err());
    }
}
