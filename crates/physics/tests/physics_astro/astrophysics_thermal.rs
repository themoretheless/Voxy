use physics::astrophysics_thermal::{STEFAN_BOLTZMANN, ThermalBody};
fn body() -> ThermalBody {
    ThermalBody {
        heat_capacity: 100.0,
        temperature: 300.0,
        area: 2.0,
        emissivity: 0.8,
        radiated_energy: 0.0,
    }
}
#[test]
fn luminosity_and_energy_balance() {
    let mut b = body();
    assert!((b.luminosity(0.0).unwrap() - 1.6 * STEFAN_BOLTZMANN * 300_f64.powi(4)).abs() < 1e-10);
    b.step(1000.0, 0.0, 10.0).unwrap();
    assert!(b.temperature > 0.0 && b.temperature < 310.0);
    assert!((b.heat_capacity * b.temperature + b.radiated_energy - 31000.0).abs() < 1e-9);
    assert!((b.radiated_energy - 10.0 * b.luminosity(0.0).unwrap()).abs() < 1e-8);
}
#[test]
fn stiff_cooling_and_bath_heating_stay_bounded() {
    let mut b = body();
    b.step(0.0, 200.0, 1e20).unwrap();
    assert!((b.temperature - 200.0).abs() < 1e-10);
    let mut b = body();
    b.step(0.0, 400.0, 1e20).unwrap();
    assert!((b.temperature - 400.0).abs() < 1e-10);
    assert!(b.radiated_energy < 0.0);
}
#[test]
fn insulation_and_invalid_input_are_atomic() {
    let mut b = body();
    b.emissivity = 0.0;
    b.step(1000.0, 0.0, 100.0).unwrap();
    assert_eq!(b.temperature, 310.0);
    assert_eq!(b.radiated_energy, 0.0);
    let before = b;
    assert!(b.step(-1.0, 0.0, 1.0).is_err());
    assert_eq!(b, before);
    assert!(b.step(f64::MAX, 0.0, f64::MAX).is_err());
    assert_eq!(b, before);
}
