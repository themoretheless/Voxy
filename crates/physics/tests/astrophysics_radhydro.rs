use physics::{
    astrophysics_gas::{Boundary, Cell, Gas},
    astrophysics_radhydro::RadiatingGas,
    astrophysics_radiation::blackbody,
};
fn system() -> RadiatingGas {
    RadiatingGas {
        gas: Gas {
            cells: vec![Cell::from_primitive(1.0, 0.0, 120000.0, 1.4).unwrap(); 40],
            spacing: 0.1,
            gamma: 1.4,
            boundary: Boundary::Reflecting,
        },
        specific_heat: 1000.0,
        opacity: 0.5,
        escaped_energy: 0.0,
    }
}
#[test]
fn irradiation_drives_motion_and_conserves_energy() {
    let mut s = system();
    let initial = s.gas.totals().unwrap();
    let incoming = blackbody(600.0).unwrap();
    for _ in 0..100 {
        s.step(incoming, 0.0, 0.001, 1000, 1000).unwrap();
    }
    let totals = s.gas.totals().unwrap();
    assert!((totals[0] - initial[0]).abs() < 1e-12);
    assert!((totals[2] + s.escaped_energy - initial[2]).abs() < 1e-7);
    assert!(
        s.gas
            .cells
            .iter()
            .any(|c| (c.momentum / c.density).abs() > 1e-5)
    );
    assert!(s.column().unwrap().slabs[0].temperature > 300.0);
}
#[test]
fn lte_preserves_uniform_gas() {
    let mut s = system();
    let b = blackbody(300.0).unwrap();
    s.step(b, b, 0.1, 10000, 10000).unwrap();
    for cell in s.gas.cells {
        assert!((cell.momentum).abs() < 1e-10);
        assert!((cell.energy - 300000.0).abs() < 1e-8);
    }
    assert!(s.escaped_energy.abs() < 1e-8);
}
#[test]
fn late_component_failure_rolls_back_hydro() {
    let mut s = system();
    s.gas.cells[0].momentum = 1.0;
    let before = s.clone();
    assert!(s.step(0.0, 0.0, 0.1, 10000, 0).is_err());
    assert_eq!(s, before);
    assert!(s.step(0.0, 0.0, 0.1, 1, 10000).is_err());
    assert_eq!(s, before);
}
