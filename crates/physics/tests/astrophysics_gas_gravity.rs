use physics::{
    astrophysics_gas::{Boundary, Cell, Gas},
    astrophysics_gas_gravity::{SelfGravitatingGas, field},
};
fn system(n: usize) -> SelfGravitatingGas {
    SelfGravitatingGas {
        gas: Gas {
            cells: vec![Cell::from_primitive(1.0, 0.0, 0.1, 1.4).unwrap(); n],
            spacing: 1.0 / n as f64,
            gamma: 1.4,
            boundary: Boundary::Reflecting,
        },
        g: 0.1,
    }
}
#[test]
fn uniform_slab_matches_analytic_field_and_potential() {
    let s = system(100);
    let f = field(&s.gas, s.g).unwrap();
    for (i, a) in f.acceleration.iter().enumerate() {
        let x = (i as f64 + 0.5) * s.gas.spacing;
        assert!((a - 2.0 * std::f64::consts::PI * s.g * (1.0 - 2.0 * x)).abs() < 1e-14);
    }
    assert!((f.potential_energy - std::f64::consts::PI * s.g / 3.0).abs() < 1e-14);
    let force: f64 = s
        .gas
        .cells
        .iter()
        .zip(f.acceleration)
        .map(|(c, a)| c.density * s.gas.spacing * a)
        .sum();
    assert!(force.abs() < 1e-14);
}
#[test]
fn collapse_preserves_mass_symmetry_and_energy() {
    let mut errors = Vec::new();
    for n in [100, 200, 400] {
        let mut s = system(n);
        let initial = s.energy().unwrap();
        s.step(0.1, 0.1 / n as f64, 10000, 100000).unwrap();
        let total = s.gas.totals().unwrap();
        assert!((total[0] - 1.0).abs() < 1e-12);
        assert!(total[1].abs() < 1e-12);
        assert!(s.gas.cells[n / 2].density > 1.0);
        for (a, b) in s.gas.cells.iter().zip(s.gas.cells.iter().rev()) {
            assert!((a.density - b.density).abs() < 1e-12);
            assert!((a.momentum + b.momentum).abs() < 1e-12);
        }
        errors.push((s.energy().unwrap() - initial).abs());
    }
    assert!(errors.iter().all(|error| *error < 1e-10), "{errors:?}");
}
#[test]
fn late_budget_failure_is_atomic_and_periodic_rejected() {
    let mut s = system(20);
    let before = s.clone();
    assert!(s.step(0.1, 0.01, 2, 10000).is_err());
    assert_eq!(s, before);
    assert!(s.step(0.1, 0.01, 10000, 1).is_err());
    assert_eq!(s, before);
    s.gas.boundary = Boundary::Periodic;
    assert!(field(&s.gas, s.g).is_err());
}
#[test]
fn potential_is_energy_derivative_for_nonuniform_density() {
    let mut s = system(12);
    for (i, c) in s.gas.cells.iter_mut().enumerate() {
        c.density *= 1.0 + 0.1 * i as f64;
        c.energy *= 1.0 + 0.1 * i as f64;
    }
    let f = field(&s.gas, s.g).unwrap();
    let energy: f64 = s
        .gas
        .cells
        .iter()
        .zip(&f.potential)
        .map(|(c, p)| 0.5 * c.density * s.gas.spacing * p)
        .sum();
    assert!((energy - f.potential_energy).abs() < 1e-14);
    for i in 0..s.gas.cells.len() {
        let mut plus = s.gas.clone();
        let mut minus = plus.clone();
        plus.cells[i].density += 1e-6;
        minus.cells[i].density -= 1e-6;
        let derivative = (field(&plus, s.g).unwrap().potential_energy
            - field(&minus, s.g).unwrap().potential_energy)
            / 2e-6;
        assert!((derivative - f.potential[i] * s.gas.spacing).abs() < 1e-9);
    }
}
#[test]
fn outflow_ledger_preserves_mass_and_energy_with_throughflow() {
    for throughflow in [false, true] {
        let mut s = system(80);
        s.gas.boundary = Boundary::Outflow;
        for (i, c) in s.gas.cells.iter_mut().enumerate() {
            let velocity = if throughflow {
                2.0
            } else {
                2.0 * ((i as f64 + 0.5) / 80.0 - 0.5)
            };
            *c = Cell::from_primitive(1.0, velocity, 0.1, 1.4).unwrap();
        }
        let initial_mass = s.gas.totals().unwrap()[0];
        let initial_energy = s.energy().unwrap();
        let mut mass = 0.0;
        let mut energy = 0.0;
        for _ in 0..100 {
            let e = s.advance(0.001, 0.001, 100, 100).unwrap();
            mass += e.escaped_mass;
            energy += e.escaped_energy;
        }
        assert!((s.gas.totals().unwrap()[0] + mass - initial_mass).abs() < 1e-12);
        assert!((s.energy().unwrap() + energy - initial_energy).abs() < 1e-10);
        if !throughflow {
            assert!(mass > 0.01);
        }
    }
}
