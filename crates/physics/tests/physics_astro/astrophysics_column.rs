use physics::{
    astrophysics_column::{Column, Slab},
    astrophysics_radiation::blackbody,
};
fn column() -> Column {
    Column {
        slabs: vec![
            Slab {
                thickness: 1.0,
                absorption: 0.5,
                temperature: 300.0,
                heat_capacity: 1000.0
            };
            4
        ],
        escaped_energy: 0.0,
    }
}
#[test]
fn equilibrium_and_flux_balance() {
    let mut c = column();
    let b = blackbody(300.0).unwrap();
    let r = c.rates(b, b).unwrap();
    assert!(r.heating.iter().all(|v| v.abs() < 1e-12));
    assert!(r.escaping.abs() < 1e-12);
    c.step(b, b, 100.0, 10000).unwrap();
    assert!(
        c.slabs
            .iter()
            .all(|s| (s.temperature - 300.0).abs() < 1e-10)
    );
}
#[test]
fn heating_and_cooling_conserve_combined_energy() {
    for boundary in [0.0, blackbody(600.0).unwrap()] {
        let mut c = column();
        let initial = c.internal_energy().unwrap();
        c.step(boundary, boundary, 100.0, 100000).unwrap();
        assert!((c.internal_energy().unwrap() + c.escaped_energy - initial).abs() < 1e-7);
        if boundary == 0.0 {
            assert!(c.slabs.iter().all(|s| s.temperature < 300.0));
            assert!(c.escaped_energy > 0.0);
        } else {
            assert!(c.slabs.iter().all(|s| s.temperature > 300.0));
            assert!(c.escaped_energy < 0.0);
        }
    }
}
#[test]
fn empty_radiation_and_atomic_budget() {
    let mut c = column();
    let before = c.clone();
    assert!(c.step(0.0, 0.0, 100.0, 1).is_err());
    assert_eq!(c, before);
    for s in &mut c.slabs {
        s.absorption = 0.0;
    }
    let e = c.internal_energy().unwrap();
    c.step(100.0, 100.0, 100.0, 1).unwrap();
    assert_eq!(c.internal_energy().unwrap(), e);
    assert_eq!(c.escaped_energy, 0.0);
}
