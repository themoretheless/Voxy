use physics::liquid::{
    water_homogeneous_from_energy, water_homogeneous_response, water_homogeneous_state,
};

#[test]
fn pressure_response_matches_finite_differences_and_caloric_sound_identity() {
    for (t, rho) in [
        (275., 1000.),
        (300., 996.556),
        (500., 0.435),
        (900., 52.615),
    ] {
        let response = water_homogeneous_response(t, rho).unwrap();
        assert_eq!(response.state, water_homogeneous_state(t, rho).unwrap());
        let dt = t * 1e-6;
        let dr = rho * 1e-6;
        let pt = (water_homogeneous_state(t + dt, rho).unwrap().pressure_pa
            - water_homogeneous_state(t - dt, rho).unwrap().pressure_pa)
            / (2. * dt);
        let pr = (water_homogeneous_state(t, rho + dr).unwrap().pressure_pa
            - water_homogeneous_state(t, rho - dr).unwrap().pressure_pa)
            / (2. * dr);
        assert!((pt / response.pressure_temperature_pa_per_k - 1.).abs() < 1e-5);
        assert!((pr / response.pressure_density_pa_m3_per_kg - 1.).abs() < 1e-5);
        let sound2 = response.pressure_density_pa_m3_per_kg
            + t * response.pressure_temperature_pa_per_k.powi(2)
                / (rho * rho * response.state.cv_j_per_kg_k);
        assert!((sound2 / response.state.sound_speed_m_per_s.powi(2) - 1.).abs() < 1e-12);
        if t == 275. {
            assert!(response.pressure_temperature_pa_per_k < 0.);
        }
    }
}
#[test]
fn internal_energy_inversion_recovers_liquid_and_vapor_temperature() {
    for (t, rho, bracket) in [
        (300., 996.556, [299.99, 300.01]),
        (500., 0.435, [450., 650.]),
        (900., 52.615, [700., 1100.]),
    ] {
        let original = water_homogeneous_state(t, rho).unwrap();
        let (recovered, state) =
            water_homogeneous_from_energy(rho, original.internal_energy_j_per_kg, bracket).unwrap();
        assert!((recovered - t).abs() < 1e-7);
        assert!((state.internal_energy_j_per_kg - original.internal_energy_j_per_kg).abs() < 1e-5);
        assert!((state.pressure_pa / original.pressure_pa - 1.).abs() < 1e-8);
        for endpoint in bracket {
            let s = water_homogeneous_state(endpoint, rho).unwrap();
            assert_eq!(
                water_homogeneous_from_energy(rho, s.internal_energy_j_per_kg, bracket)
                    .unwrap()
                    .0,
                endpoint
            );
        }
    }
}

#[test]
fn energy_inversion_rejects_invalid_brackets_and_unbracketed_energy() {
    let rho = 0.435;
    let lower = water_homogeneous_state(450., rho)
        .unwrap()
        .internal_energy_j_per_kg;
    let upper = water_homogeneous_state(650., rho)
        .unwrap()
        .internal_energy_j_per_kg;
    for (energy, bracket) in [
        (lower - 1., [450., 650.]),
        (upper + 1., [450., 650.]),
        (lower, [650., 450.]),
        (lower, [450., 450.]),
        (lower, [f64::NAN, 650.]),
        (f64::NAN, [450., 650.]),
    ] {
        assert!(water_homogeneous_from_energy(rho, energy, bracket).is_err());
    }
}
#[test]
fn reproduces_all_iapws95_table_seven_homogeneous_states() {
    // Official IAPWS R6-95(2018), Table 7: T, rho, p(MPa), cv(kJ/kg/K), w, s(kJ/kg/K).
    for (t, rho, p, cv, w, s) in [
        (
            300.,
            996.556,
            0.0992418352,
            4.13018112,
            1501.51914,
            0.393062643,
        ),
        (
            300.,
            1005.308,
            20.0022515,
            4.06798347,
            1534.92501,
            0.387405401,
        ),
        (
            300.,
            1188.202,
            700.004704,
            3.46135580,
            2443.57992,
            0.132609616,
        ),
        (
            500.,
            0.435,
            0.0999679423,
            1.50817541,
            548.314253,
            7.94488271,
        ),
        (500., 4.532, 0.999938125, 1.66991025, 535.739001, 6.82502725),
        (
            500., 838.025, 10.0003858, 3.22106219, 1271.28441, 2.56690919,
        ),
        (
            500., 1084.564, 700.000405, 3.07437693, 2412.00877, 2.03237509,
        ),
        (647., 358., 22.0384756, 6.18315728, 252.145078, 4.32092307),
        (900., 0.241, 0.100062559, 1.75890657, 724.027147, 9.16653194),
        (900., 52.615, 20.0000690, 1.93510526, 698.445674, 6.59070225),
        (
            900., 870.769, 700.000006, 2.66422350, 2019.33608, 4.17223802,
        ),
    ] {
        let state = water_homogeneous_state(t, rho).unwrap();
        for (name, actual, expected) in [
            ("p", state.pressure_pa, p * 1e6),
            ("cv", state.cv_j_per_kg_k, cv * 1000.),
            ("w", state.sound_speed_m_per_s, w),
            ("s", state.entropy_j_per_kg_k, s * 1000.),
        ] {
            let tolerance = if name == "p" && t == 300. && rho == 996.556 {
                1e-5
            } else {
                2e-7
            };
            assert!(
                (actual / expected - 1.).abs() < tolerance,
                "T={t}, rho={rho}, {name}: actual={actual}, expected={expected}"
            );
        }
        assert!(
            (state.enthalpy_j_per_kg - state.internal_energy_j_per_kg - state.pressure_pa / rho)
                .abs()
                < 1e-8
        );
    }
}
#[test]
fn caloric_derivative_matches_cv_without_independent_energy_fit() {
    for (t, rho) in [(300., 996.556), (500., 0.435), (900., 52.615)] {
        let s = water_homogeneous_state(t, rho).unwrap();
        let derivative = (water_homogeneous_state(t + 0.001, rho)
            .unwrap()
            .internal_energy_j_per_kg
            - water_homogeneous_state(t - 0.001, rho)
                .unwrap()
                .internal_energy_j_per_kg)
            / 0.002;
        assert!((derivative / s.cv_j_per_kg_k - 1.).abs() < 1e-7);
    }
}
#[test]
fn rejects_invalid_inputs_and_unhandled_critical_singularity() {
    for (t, rho) in [
        (f64::NAN, 1.),
        (300., 0.),
        (300., f64::INFINITY),
        (273., 1000.),
        (1274., 1.),
        (647.096, 322.),
    ] {
        assert!(water_homogeneous_state(t, rho).is_err());
    }
}
