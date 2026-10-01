use physics::{
    astrophysics_gas::{Boundary, Cell, Gas},
    astrophysics_radhydro::{CoupledBudget, RadiatingGas},
    astrophysics_radiation::blackbody,
};
fn system(n: usize) -> RadiatingGas {
    RadiatingGas {
        gas: Gas {
            cells: vec![Cell::from_primitive(1.0, 0.0, 120000.0, 1.4).unwrap(); n],
            spacing: 1.0 / n as f64,
            gamma: 1.4,
            boundary: Boundary::Reflecting,
        },
        specific_heat: 1000.0,
        opacity: 0.5,
        escaped_energy: 0.0,
    }
}
fn budget() -> CoupledBudget {
    CoupledBudget {
        max_step: 1e-4,
        gravity_steps: 1000,
        hydro_steps: 100000,
        thermal_steps: 10000,
    }
}
#[test]
fn irradiation_and_gravity_evolve_in_one_atomic_step() {
    let mut s = system(40);
    let initial = s.total_energy_with_gravity(1000.0).unwrap();
    let b = blackbody(600.0).unwrap();
    let w = s.step_with_gravity(1000.0, b, 0.0, 0.01, budget()).unwrap();
    assert!(w.gravity_steps >= 100 && w.hydro_steps > 0 && w.thermal_steps > 0);
    assert!((s.gas.totals().unwrap()[0] - 1.0).abs() < 1e-12);
    assert!(s.gas.cells.iter().any(|c| c.momentum.abs() > 1e-4));
    assert!(s.escaped_energy < 0.0);
    assert!((s.total_energy_with_gravity(1000.0).unwrap() - initial).abs() / initial < 1e-3);
}
#[test]
fn joint_energy_balance_on_multiple_meshes() {
    let mut errors = Vec::new();
    let b = blackbody(300.0).unwrap();
    for n in [40, 80, 160] {
        let mut s = system(n);
        let initial = s.total_energy_with_gravity(1000.0).unwrap();
        s.step_with_gravity(
            1000.0,
            b,
            b,
            0.01,
            CoupledBudget {
                max_step: 0.001 / n as f64,
                gravity_steps: 10000,
                ..budget()
            },
        )
        .unwrap();
        errors.push((s.total_energy_with_gravity(1000.0).unwrap() - initial).abs());
    }
    assert!(errors.iter().all(|error| *error < 1e-6), "{errors:?}");
}
#[test]
fn shared_budget_failure_restores_radiation_and_hydro() {
    let mut s = system(40);
    let before = s.clone();
    let b = blackbody(600.0).unwrap();
    assert!(
        s.step_with_gravity(
            1000.0,
            b,
            0.0,
            0.01,
            CoupledBudget {
                gravity_steps: 2,
                ..budget()
            }
        )
        .is_err()
    );
    assert_eq!(s, before);
    assert!(
        s.step_with_gravity(
            1000.0,
            b,
            0.0,
            0.01,
            CoupledBudget {
                thermal_steps: 2,
                ..budget()
            }
        )
        .is_err()
    );
    assert_eq!(s, before);
}
#[test]
fn long_irradiated_collapse_keeps_energy_without_global_rescaling() {
    let mut s = system(40);
    s.gas.spacing = 0.1;
    let initial = s.total_energy_with_gravity(1000.0).unwrap();
    let b = blackbody(600.0).unwrap();
    for _ in 0..1000 {
        s.step_with_gravity(
            1000.0,
            b,
            0.0,
            0.001,
            CoupledBudget {
                gravity_steps: 100,
                ..budget()
            },
        )
        .unwrap();
    }
    let error = (s.total_energy_with_gravity(1000.0).unwrap() - initial).abs();
    assert!(error / initial < 1e-10, "{error}");
    assert!((s.gas.totals().unwrap()[0] - 4.0).abs() < 1e-11);
}
#[test]
fn open_irradiated_column_reports_full_boundary_ledger() {
    let mut s = system(40);
    s.gas.spacing = 0.1;
    s.gas.boundary = Boundary::Outflow;
    for (i, c) in s.gas.cells.iter_mut().enumerate() {
        let u = 600.0 * (i as f64 / 39.0 - 0.5);
        *c = Cell::from_primitive(1.0, u, 120000.0, 1.4).unwrap();
    }
    let initial = s.total_energy_with_gravity(1000.0).unwrap();
    let mass = s.gas.totals().unwrap()[0];
    let b = blackbody(600.0).unwrap();
    let mut escaped_energy = 0.0;
    let mut escaped_mass = 0.0;
    for _ in 0..100 {
        let w = s
            .step_with_gravity(
                1000.0,
                b,
                0.0,
                0.0001,
                CoupledBudget {
                    max_step: 0.0001,
                    ..budget()
                },
            )
            .unwrap();
        escaped_energy += w.escaped_gas_energy;
        escaped_mass += w.escaped_mass;
    }
    assert!((s.total_energy_with_gravity(1000.0).unwrap() + escaped_energy - initial).abs() < 1e-7);
    assert!((s.gas.totals().unwrap()[0] + escaped_mass - mass).abs() < 1e-12);
    assert!(escaped_mass > 0.0);
}
