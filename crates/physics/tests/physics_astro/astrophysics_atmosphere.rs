use physics::{
    astrophysics_atmosphere::{Atmosphere, Error},
    astrophysics_eos::{Mixture, Species},
};
fn atmosphere() -> Atmosphere {
    Atmosphere {
        gravity: 100.0,
        opacity: 0.01,
        effective_temperature: 6000.0,
        top_gas_pressure: 0.0,
        mixture: Mixture::new(&[Species {
            mass_fraction: 1.0,
            mass_number: 1,
            nuclear_charge: 1,
        }])
        .unwrap(),
    }
}
#[test]
fn temperature_surface_and_hydrostatic_pressure_with_eos() {
    let a = atmosphere();
    let top = a.at(0.0).unwrap();
    assert_eq!(top.density, 0.0);
    let surface = a.at(2.0 / 3.0).unwrap();
    assert!((surface.temperature / a.effective_temperature - 1.0).abs() < 1e-14);
    for tau in [0.1, 1.0, 10.0] {
        let state = a.at(tau).unwrap();
        let eos = a.mixture.at(state.density, state.temperature).unwrap();
        assert!((eos.gas_pressure / state.gas_pressure - 1.0).abs() < 1e-13);
        assert!((eos.radiation_pressure / state.radiation_pressure - 1.0).abs() < 1e-13);
        assert!(
            ((state.gas_pressure + state.radiation_pressure - top.radiation_pressure)
                / (a.gravity / a.opacity * tau)
                - 1.0)
                .abs()
                < 1e-13
        );
    }
}
#[test]
fn nonstatic_and_invalid_atmospheres_are_rejected() {
    let a = atmosphere();
    assert_eq!(
        Atmosphere {
            effective_temperature: 1e8,
            ..a
        }
        .at(1.0),
        Err(Error::NoStaticAtmosphere)
    );
    assert!(a.at(-1.0).is_err());
    assert!(Atmosphere { opacity: 0.0, ..a }.at(1.0).is_err());
}

#[test]
fn atmosphere_gas_cell_preserves_thermal_and_kinetic_energy() {
    let a = atmosphere();
    let depth = 2.0 / 3.0;
    let state = a.at(depth).unwrap();
    let velocity = 123.0;
    let cell = a.gas_cell(depth, velocity).unwrap();
    assert_eq!(cell.density, state.density);
    assert!((cell.momentum / cell.density - velocity).abs() < 1e-13);
    let internal = cell.energy - 0.5 * cell.momentum.powi(2) / cell.density;
    let recovered = a.mixture.temperature(cell.density, internal).unwrap();
    assert!((recovered / state.temperature - 1.0).abs() < 1e-13);
    assert!(a.gas_cell(0.0, 0.0).is_err());
    assert!(a.gas_cell(depth, f64::INFINITY).is_err());
}
