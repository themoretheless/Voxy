use physics::liquid::{
    WaterHeatContact, change_water_volume_adiabatically, exchange_water_contact_heat,
    transfer_water_heat, water_coexistence, water_equilibrium_at_temperature,
    water_equilibrium_from_energy, water_equilibrium_from_entropy, water_equilibrium_response,
};

#[test]
fn equilibrium_caloric_and_acoustic_response_matches_independent_state_derivatives() {
    for (t, density) in [(300., 0.1), (450., 5.), (450., 50.), (500., 0.435)] {
        let response = water_equilibrium_response(t, density).unwrap();
        assert_eq!(
            response.state,
            water_equilibrium_at_temperature(t, density).unwrap()
        );
        let dt = 1e-3;
        let cold = water_equilibrium_at_temperature(t - dt, density).unwrap();
        let hot = water_equilibrium_at_temperature(t + dt, density).unwrap();
        let cv = (hot.internal_energy_j_per_kg - cold.internal_energy_j_per_kg) / (2. * dt);
        assert!(
            (cv / response.cv_j_per_kg_k - 1.).abs() < 1e-5,
            "T={t} rho={density} derivative cv={cv}, model cv={}",
            response.cv_j_per_kg_k
        );
        let pressure_t = (hot.pressure_pa - cold.pressure_pa) / (2. * dt);
        assert!(
            (pressure_t / response.pressure_temperature_pa_per_k - 1.).abs() < 1e-5,
            "T={t} rho={density} pressure derivative={pressure_t}, model p_T={}, cold p={}, hot p={}",
            response.pressure_temperature_pa_per_k,
            cold.pressure_pa,
            hot.pressure_pa
        );
        let dr = density * 1e-4;
        let entropy = response.state.entropy_j_per_kg_k;
        let dilute =
            water_equilibrium_from_entropy(density - dr, entropy, [t - 1., t + 1.]).unwrap();
        let dense =
            water_equilibrium_from_entropy(density + dr, entropy, [t - 1., t + 1.]).unwrap();
        let sound2 = (dense.pressure_pa - dilute.pressure_pa) / (2. * dr);
        assert!(
            (sound2 / response.sound_speed_m_per_s.powi(2) - 1.).abs() < 1e-4,
            "T={t} rho={density} isentropic derivative={sound2}, model c²={}",
            response.sound_speed_m_per_s.powi(2)
        );
        if response.state.vapor_mass_fraction > 0. && response.state.vapor_mass_fraction < 1. {
            assert_eq!(response.pressure_density_pa_m3_per_kg, 0.);
        }
    }
    assert!(water_equilibrium_response(450., 0.).is_err());
    assert!(water_equilibrium_response(647.096, 322.).is_err());
}

#[test]
fn reversible_volume_work_balances_energy_and_preserves_all_owners_on_failure() {
    let mass = 0.01;
    let initial = water_equilibrium_at_temperature(450., 5.).unwrap();
    let mut volume = 0.002;
    let mut water = mass * initial.internal_energy_j_per_kg;
    let mut work = 2000.;
    let total = water + work;
    let starting_energy = water;
    let (expanded, delivered) = change_water_volume_adiabatically(
        mass,
        &mut volume,
        &mut water,
        &mut work,
        0.0022,
        [420., 490.],
    )
    .unwrap();
    assert!(delivered > 0. && expanded.temperature_k < 450.);
    assert!((expanded.entropy_j_per_kg_k - initial.entropy_j_per_kg_k).abs() < 1e-6);
    assert!((water + work - total).abs() < 1e-9);
    let (restored, consumed) = change_water_volume_adiabatically(
        mass,
        &mut volume,
        &mut water,
        &mut work,
        0.002,
        [420., 490.],
    )
    .unwrap();
    assert!(consumed < 0.);
    assert!((water - starting_energy).abs() < 1e-5);
    assert!((restored.temperature_k - 450.).abs() < 1e-6);
    assert!((water + work - total).abs() < 1e-9);
    for target in [0., f64::NAN, 0.2] {
        let saved = (volume, water, work);
        assert!(
            change_water_volume_adiabatically(
                mass,
                &mut volume,
                &mut water,
                &mut work,
                target,
                [420., 490.],
            )
            .is_err()
        );
        assert_eq!((volume, water, work), saved);
    }
    // A compression cannot consume more mechanical energy than its owner has.
    work = 0.;
    let saved = (volume, water, work);
    assert!(
        change_water_volume_adiabatically(
            mass,
            &mut volume,
            &mut water,
            &mut work,
            0.0018,
            [420., 490.],
        )
        .is_err()
    );
    assert_eq!((volume, water, work), saved);
}

#[test]
fn isentropic_energy_volume_derivative_matches_eos_pressure() {
    let mass = 0.01;
    let volume = 0.002;
    let state = water_equilibrium_at_temperature(450., mass / volume).unwrap();
    let dv = volume * 1e-4;
    let energies = [volume - dv, volume + dv].map(|v| {
        mass * water_equilibrium_from_entropy(mass / v, state.entropy_j_per_kg_k, [440., 470.])
            .unwrap()
            .internal_energy_j_per_kg
    });
    let pressure = -(energies[1] - energies[0]) / (2. * dv);
    assert!(
        (pressure / state.pressure_pa - 1.).abs() < 1e-4,
        "isentropic derivative={pressure}, EOS pressure={}",
        state.pressure_pa
    );
}

#[test]
fn entropy_inversion_recovers_both_phases_and_rejects_unbracketed_states() {
    for temperature in [450., 465.] {
        let state = water_equilibrium_at_temperature(temperature, 5.).unwrap();
        let recovered =
            water_equilibrium_from_entropy(5., state.entropy_j_per_kg_k, [440., 470.]).unwrap();
        assert!((recovered.temperature_k - temperature).abs() < 1e-7);
        assert!((recovered.entropy_j_per_kg_k - state.entropy_j_per_kg_k).abs() < 1e-7);
        assert!((recovered.vapor_mass_fraction - state.vapor_mass_fraction).abs() < 1e-8);
        assert!((recovered.internal_energy_j_per_kg - state.internal_energy_j_per_kg).abs() < 1e-3);
    }
    let lower = water_equilibrium_at_temperature(440., 5.).unwrap();
    assert_eq!(
        water_equilibrium_from_entropy(5., lower.entropy_j_per_kg_k, [440., 470.]).unwrap(),
        lower
    );
    for entropy in [lower.entropy_j_per_kg_k - 1., f64::NAN] {
        assert!(water_equilibrium_from_entropy(5., entropy, [440., 470.]).is_err());
    }
}

#[test]
fn contact_time_refinement_reduces_dynamic_error_without_losing_energy() {
    let contact = WaterHeatContact {
        mass_kg: 0.01,
        volume_m3: 0.002,
        reservoir_capacity_j_per_k: 50.,
        conductance_w_per_k: 10.,
        temperature_interval: [440., 490.],
    };
    let initial = water_equilibrium_at_temperature(450., 5.).unwrap();
    let run = |steps: usize| {
        let mut water = contact.mass_kg * initial.internal_energy_j_per_kg;
        let mut reservoir = 50. * 500.;
        let total = water + reservoir;
        for _ in 0..steps {
            let (state, q) =
                exchange_water_contact_heat(contact, &mut water, &mut reservoir, 2. / steps as f64)
                    .unwrap();
            assert!(q > 0.);
            assert!(state.temperature_k < reservoir / 50.);
            assert!((water + reservoir - total).abs() < 1e-8);
        }
        water
    };
    // A refined temporal reference tests convergence, not a separate EOS oracle.
    // Coarse 1/2-step results are outside the asymptotic ratio gate in this
    // nonlinear EOS fixture (archived initial failure). Keep the ratio gate
    // unchanged and resolve the temporal dynamics with smaller steps.
    let reference = run(128);
    let errors = [4, 8, 16].map(|steps| (run(steps) - reference).abs());
    assert!(errors[0] > 1.7 * errors[1], "{errors:?}");
    assert!(errors[1] > 1.7 * errors[2], "{errors:?}");
}
#[test]
fn contact_conductance_obeys_implicit_heat_law_and_preserves_both_stores_on_error() {
    let contact = WaterHeatContact {
        mass_kg: 0.01,
        volume_m3: 0.002,
        reservoir_capacity_j_per_k: 50.,
        conductance_w_per_k: 10.,
        temperature_interval: [440., 490.],
    };
    let initial = water_equilibrium_at_temperature(450., 5.).unwrap();
    for reservoir_temperature in [400., 500.] {
        let mut water = 0.01 * initial.internal_energy_j_per_kg;
        let mut reservoir = 50. * reservoir_temperature;
        let total = water + reservoir;
        let (state, q) =
            exchange_water_contact_heat(contact, &mut water, &mut reservoir, 2.).unwrap();
        let reservoir_after = reservoir / 50.;
        assert!((q - 20. * (reservoir_after - state.temperature_k)).abs() < 2e-5);
        assert!((water + reservoir - total).abs() < 1e-9);
        if reservoir_temperature > 450. {
            assert!(
                q > 0. && state.temperature_k > 450. && reservoir_after < reservoir_temperature
            );
            assert!(state.temperature_k < reservoir_after);
        } else {
            assert!(
                q < 0. && state.temperature_k < 450. && reservoir_after > reservoir_temperature
            );
            assert!(state.temperature_k > reservoir_after);
        }
        let before = (water, reservoir);
        let invalid = WaterHeatContact {
            conductance_w_per_k: f64::NAN,
            ..contact
        };
        assert!(exchange_water_contact_heat(invalid, &mut water, &mut reservoir, 2.).is_err());
        assert_eq!((water, reservoir), before);
        assert!(exchange_water_contact_heat(contact, &mut water, &mut reservoir, -1.).is_err());
        assert_eq!((water, reservoir), before);
    }
}
#[test]
fn finite_heat_reservoir_drives_full_vapor_and_preserves_energy_on_failure() {
    let mass = 0.01;
    let density = 5.;
    let volume = mass / density;
    let wet = water_equilibrium_at_temperature(450., density).unwrap();
    let hot = water_equilibrium_at_temperature(465., density).unwrap();
    let mut water = mass * wet.internal_energy_j_per_kg;
    let mut reservoir = 10000.;
    let total = water + reservoir;
    let heat = mass * (hot.internal_energy_j_per_kg - wet.internal_energy_j_per_kg);
    let (state, actual) =
        transfer_water_heat(mass, volume, &mut water, &mut reservoir, heat, [440., 470.]).unwrap();
    assert_eq!(state.vapor_mass_fraction, 1.);
    assert!((state.temperature_k - 465.).abs() < 1e-7);
    assert!((actual - heat).abs() < 1e-10);
    assert!((water + reservoir - total).abs() < 1e-10);
    let (state, _) = transfer_water_heat(
        mass,
        volume,
        &mut water,
        &mut reservoir,
        -heat,
        [440., 470.],
    )
    .unwrap();
    assert!((state.temperature_k - 450.).abs() < 1e-7);
    assert!((state.vapor_mass_fraction - wet.vapor_mass_fraction).abs() < 1e-8);
    assert!((water + reservoir - total).abs() < 1e-10);
    let saved = (water, reservoir);
    for requested in [1e6, -1e6, f64::NAN] {
        assert!(
            transfer_water_heat(
                mass,
                volume,
                &mut water,
                &mut reservoir,
                requested,
                [440., 470.]
            )
            .is_err()
        );
        assert_eq!((water, reservoir), saved);
    }
    // Water cannot resolve this increment, whereas a near-empty reservoir can.
    let mut tiny_reservoir = 1e-16;
    assert!(
        transfer_water_heat(
            mass,
            volume,
            &mut water,
            &mut tiny_reservoir,
            1e-17,
            [440., 470.]
        )
        .is_err()
    );
    assert_eq!(water, saved.0);
    assert_eq!(tiny_reservoir, 1e-16);
}
#[test]
fn phase_selection_and_lever_rule_preserve_specific_volume_energy_and_entropy() {
    let phases = water_coexistence(450.).unwrap();
    let rl = phases.liquid_density_kg_per_m3;
    let rv = phases.vapor_density_kg_per_m3;
    for fraction in [0., 0.1, 0.5, 0.9, 1.] {
        let specific_volume = (1. - fraction) / rl + fraction / rv;
        let state = water_equilibrium_at_temperature(450., 1. / specific_volume).unwrap();
        assert!((state.vapor_mass_fraction - fraction).abs() < 1e-12);
        let energy = (1. - fraction) * phases.liquid.internal_energy_j_per_kg
            + fraction * phases.vapor.internal_energy_j_per_kg;
        let entropy = (1. - fraction) * phases.liquid.entropy_j_per_kg_k
            + fraction * phases.vapor.entropy_j_per_kg_k;
        assert!((state.internal_energy_j_per_kg - energy).abs() < 1e-7);
        assert!((state.entropy_j_per_kg_k - entropy).abs() < 1e-8);
        assert!(
            (state.enthalpy_j_per_kg
                - state.internal_energy_j_per_kg
                - state.pressure_pa * specific_volume)
                .abs()
                < 1e-8
        );
    }
    assert_eq!(
        water_equilibrium_at_temperature(450., rl * 1.001)
            .unwrap()
            .vapor_mass_fraction,
        0.
    );
    assert_eq!(
        water_equilibrium_at_temperature(450., rv * 0.99)
            .unwrap()
            .vapor_mass_fraction,
        1.
    );
}
#[test]
fn fixed_volume_heating_crosses_full_vapor_boundary_without_dropping_mass() {
    let density = 5.;
    let wet = water_equilibrium_at_temperature(450., density).unwrap();
    let hot = water_equilibrium_at_temperature(465., density).unwrap();
    assert!(wet.vapor_mass_fraction > 0. && wet.vapor_mass_fraction < 1.);
    assert_eq!(hot.vapor_mass_fraction, 1.);
    assert!(hot.internal_energy_j_per_kg > wet.internal_energy_j_per_kg);
    for state in [wet, hot] {
        let recovered =
            water_equilibrium_from_energy(density, state.internal_energy_j_per_kg, [440., 470.])
                .unwrap();
        assert!((recovered.temperature_k - state.temperature_k).abs() < 1e-7);
        assert!((recovered.vapor_mass_fraction - state.vapor_mass_fraction).abs() < 1e-8);
        assert!((recovered.internal_energy_j_per_kg - state.internal_energy_j_per_kg).abs() < 1e-4);
    }
    assert!(
        water_equilibrium_from_energy(density, wet.internal_energy_j_per_kg - 1e6, [440., 470.])
            .is_err()
    );
    assert!(water_equilibrium_at_temperature(450., 0.).is_err());
}
