use physics::{
    astrophysics_gas::{Boundary, Cell},
    astrophysics_spherical::Sphere,
};
fn sphere(n: usize, g: f64) -> Sphere {
    Sphere {
        cells: vec![Cell::from_primitive(1.0, 0.0, 0.1, 1.4).unwrap(); n],
        spacing: 1.0 / n as f64,
        gamma: 1.4,
        g,
        outer: Boundary::Reflecting,
    }
}
#[test]
fn uniform_sphere_binding_and_field() {
    let s = sphere(100, 0.1);
    let f = s.field().unwrap();
    let mass = 4.0 * std::f64::consts::PI / 3.0;
    assert!((f.energy + 0.6 * s.g * mass * mass).abs() < 1e-13);
    for (i, a) in f.acceleration.iter().enumerate() {
        let lo = i as f64 * s.spacing;
        let hi = lo + s.spacing;
        let mean = 0.75 * (hi.powi(4) - lo.powi(4)) / (hi.powi(3) - lo.powi(3));
        assert!((a + 4.0 * std::f64::consts::PI * s.g * mean / 3.0).abs() < 1e-12);
    }
    let energy: f64 = s
        .cells
        .iter()
        .zip(&f.potential)
        .enumerate()
        .map(|(i, (c, p))| {
            let a = i as f64 * s.spacing;
            let b = a + s.spacing;
            0.5 * c.density * p * 4.0 * std::f64::consts::PI / 3.0 * (b.powi(3) - a.powi(3))
        })
        .sum();
    assert!((energy - f.energy).abs() < 1e-13);
}
#[test]
fn geometric_pressure_preserves_uniform_static_gas() {
    let mut s = sphere(40, 0.0);
    s.step(0.1, 0.001, 10000).unwrap();
    assert!(s.cells.iter().all(|c| (c.density - 1.0).abs() < 1e-12
        && c.momentum.abs() < 1e-12
        && (c.energy - 0.25).abs() < 1e-12));
}
#[test]
fn collapse_and_open_exchange_conserve_energy_mass() {
    for outer in [Boundary::Reflecting, Boundary::Outflow] {
        let mut s = sphere(60, 0.1);
        s.outer = outer;
        if outer == Boundary::Outflow {
            for (i, c) in s.cells.iter_mut().enumerate() {
                *c = Cell::from_primitive(1.0, i as f64 / 60.0, 0.1, 1.4).unwrap();
            }
        }
        let mass = s.totals().unwrap()[0];
        let energy = s.energy().unwrap();
        let e = s.step(0.1, 0.0005, 10000).unwrap();
        assert!((s.totals().unwrap()[0] + e.escaped_mass - mass).abs() < 1e-11);
        assert!((s.energy().unwrap() + e.escaped_energy - energy).abs() < 1e-10);
        if outer == Boundary::Reflecting {
            assert!(s.cells[0].density > 1.0);
        } else {
            assert!(e.escaped_mass > 0.0);
        }
    }
}
#[test]
fn late_budget_failure_is_atomic() {
    let mut s = sphere(40, 0.1);
    let before = s.clone();
    assert!(s.step(0.1, 0.001, 2).is_err());
    assert_eq!(s, before);
    s.outer = Boundary::Periodic;
    assert!(s.field().is_err());
}

#[test]
fn composition_transport_conserves_species_with_open_boundary() {
    for boundary in [Boundary::Reflecting, Boundary::Outflow] {
        let mut s = sphere(20, 0.0);
        s.outer = boundary;
        for (i, cell) in s.cells.iter_mut().enumerate() {
            *cell = Cell::from_primitive(1.0, 0.2 * i as f64 / 20.0, 0.1, 1.4).unwrap();
        }
        let mut fractions: Vec<_> = (0..20)
            .map(|i| {
                if i < 10 {
                    vec![1.0, 0.0]
                } else {
                    vec![0.0, 1.0]
                }
            })
            .collect();
        let masses = |s: &Sphere, rows: &[Vec<f64>]| -> [f64; 2] {
            let mut total = [0.0; 2];
            for (i, (cell, row)) in s.cells.iter().zip(rows).enumerate() {
                let a = i as f64 * s.spacing;
                let b = a + s.spacing;
                let mass =
                    cell.density * 4.0 * std::f64::consts::PI / 3.0 * (b.powi(3) - a.powi(3));
                for j in 0..2 {
                    total[j] += mass * row[j];
                }
            }
            total
        };
        let initial = masses(&s, &fractions);
        let (_, escaped) = s
            .step_composition(&mut fractions, 0.02, 0.0001, 1000)
            .unwrap();
        let final_mass = masses(&s, &fractions);
        for j in 0..2 {
            assert!((final_mass[j] + escaped[j] - initial[j]).abs() < 1e-12);
        }
        assert!(
            fractions.iter().all(|row| row.iter().all(|x| *x >= 0.0)
                && (row.iter().sum::<f64>() - 1.0).abs() < 1e-12)
        );
        let original_sphere = s.clone();
        let original_fractions = fractions.clone();
        assert!(s.step_composition(&mut fractions, 0.02, 0.0001, 2).is_err());
        assert_eq!(s, original_sphere);
        assert_eq!(fractions, original_fractions);
    }
}
