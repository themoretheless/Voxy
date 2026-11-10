use physics::astrophysics_eos::{Mixture, Species};
fn hydrogen() -> Mixture {
    Mixture::new(&[Species {
        mass_fraction: 1.0,
        mass_number: 1,
        nuclear_charge: 1,
    }])
    .unwrap()
}
#[test]
fn composition_sets_molecular_weights() {
    let h = hydrogen();
    assert_eq!(h.mean_molecular_weight(), 0.5);
    assert_eq!(h.electron_molecular_weight(), 1.0);
    let he = Mixture::new(&[Species {
        mass_fraction: 1.0,
        mass_number: 4,
        nuclear_charge: 2,
    }])
    .unwrap();
    assert_eq!(he.mean_molecular_weight(), 4.0 / 3.0);
    assert_eq!(he.electron_molecular_weight(), 2.0);
    let mix = Mixture::new(&[
        Species {
            mass_fraction: 0.7,
            mass_number: 1,
            nuclear_charge: 1,
        },
        Species {
            mass_fraction: 0.28,
            mass_number: 4,
            nuclear_charge: 2,
        },
        Species {
            mass_fraction: 0.02,
            mass_number: 16,
            nuclear_charge: 8,
        },
    ])
    .unwrap();
    assert!((mix.mean_molecular_weight() - 1.0 / 1.62125).abs() < 1e-14);
}
#[test]
fn energy_temperature_roundtrip_and_heat_capacity_derivative() {
    let h = hydrogen();
    for rho in [1e-3, 1.0, 1e5] {
        for t in [1000.0, 1e6, 1e7] {
            let s = h.at(rho, t).unwrap();
            let restored = h.temperature(rho, s.internal_energy_density).unwrap();
            assert!((restored - t).abs() / t < 1e-14);
            let delta = t * 1e-5;
            let derivative = (h.at(rho, t + delta).unwrap().internal_energy_density
                - h.at(rho, t - delta).unwrap().internal_energy_density)
                / (2.0 * delta * rho);
            assert!((derivative - s.specific_heat_cv).abs() / s.specific_heat_cv < 1e-8);
        }
    }
}
#[test]
fn adiabatic_sound_speed_matches_gas_and_radiation_limits() {
    let h = hydrogen();
    let gas = h.at(10.0, 1000.0).unwrap();
    assert!((gas.sound_speed_squared / (5.0 / 3.0 * gas.gas_pressure / 10.0) - 1.0).abs() < 1e-10);
    let rad = h.at(1e-3, 1e7).unwrap();
    assert!(
        (rad.sound_speed_squared / (4.0 / 3.0 * rad.radiation_pressure / 1e-3) - 1.0).abs() < 2e-4
    );
}
#[test]
fn invalid_composition_and_inputs_rejected() {
    assert!(Mixture::new(&[]).is_err());
    assert!(
        Mixture::new(&[Species {
            mass_fraction: 0.5,
            mass_number: 1,
            nuclear_charge: 1
        }])
        .is_err()
    );
    assert!(hydrogen().at(0.0, 1000.0).is_err());
    assert!(hydrogen().temperature(1.0, -1.0).is_err());
    assert!(hydrogen().at(1.0, f64::MAX).is_err());
}

#[test]
fn pressure_inversion_recovers_gas_and_radiation_dominated_states() {
    let mixture = hydrogen();
    for density in [1e-6, 1.0, 1e6] {
        for temperature in [100.0, 1e6, 1e8] {
            let state = mixture.at(density, temperature).unwrap();
            let recovered = mixture
                .temperature_from_pressure(density, state.gas_pressure + state.radiation_pressure)
                .unwrap();
            assert!((recovered / temperature - 1.0).abs() < 1e-13);
        }
    }
    assert_eq!(mixture.temperature_from_pressure(1.0, 0.0).unwrap(), 0.0);
    assert!(mixture.temperature_from_pressure(0.0, 1.0).is_err());
    assert!(mixture.temperature_from_pressure(1.0, -1.0).is_err());
    assert!(mixture.temperature_from_pressure(1.0, f64::NAN).is_err());
}
