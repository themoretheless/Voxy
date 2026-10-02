use physics::liquid::{FiniteDropletGasGrid, GasGridBoundary, GasGridHeatControl, VaporCell};
fn pair() -> FiniteDropletGasGrid {
    FiniteDropletGasGrid::new(
        [0.0; 3],
        [0.5, 1.0, 1.0],
        [2, 1, 1],
        vec![
            VaporCell {
                mass: 1.0,
                volume: 0.5,
                temperature: 4.0,
                velocity: [0.2, 0.0, 0.0],
                specific_heat_cv: 2.0,
            },
            VaporCell {
                mass: 2.0,
                volume: 0.5,
                temperature: 1.0,
                velocity: [-0.3, 0.0, 0.0],
                specific_heat_cv: 3.0,
            },
        ],
    )
    .unwrap()
}
fn control() -> GasGridHeatControl {
    GasGridHeatControl {
        conductivity: 0.5,
        ..Default::default()
    }
}
#[test]
fn unequal_capacities_match_exact_pair_and_preserve_mass_velocity_energy() {
    let mut g = pair();
    let before = g.clone();
    let totals = g.totals().unwrap();
    let r = g.conduct_heat(0.2, control()).unwrap();
    let decay = (-0.2_f64 * (1.0 / 2.0 + 1.0 / 6.0)).exp();
    let equilibrium = 1.75;
    assert!((g.cells()[0].temperature - equilibrium - (4.0 - equilibrium) * decay).abs() < 1e-13);
    assert!((g.cells()[1].temperature - equilibrium - (1.0 - equilibrium) * decay).abs() < 1e-13);
    assert!((r.absolute_transferred_heat - 4.5 * (1.0 - decay)).abs() < 1e-13);
    assert_eq!(r.pair_steps, 2);
    assert_eq!(r.substeps, 1);
    for (a, b) in g.cells().iter().zip(before.cells()) {
        assert_eq!(a.mass, b.mass);
        assert_eq!(a.volume, b.volume);
        assert_eq!(a.velocity, b.velocity);
        assert_eq!(a.specific_heat_cv, b.specific_heat_cv);
    }
    assert!((g.totals().unwrap().thermal_energy - totals.thermal_energy).abs() < 1e-13);
}
#[test]
fn periodic_two_cell_ring_has_two_distinct_conducting_faces() {
    let mut g = pair();
    let mut c = control();
    c.boundaries = [GasGridBoundary::Periodic; 3];
    let r = g.conduct_heat(0.2, c).unwrap();
    let decay = (-0.4_f64 * (1.0 / 2.0 + 1.0 / 6.0)).exp();
    assert!((g.cells()[0].temperature - 1.75 - 2.25 * decay).abs() < 1e-13);
    assert_eq!(r.pair_steps, 4);
}
#[test]
fn pair_semigroup_and_maximum_principle_hold_at_long_times() {
    let mut single = pair();
    let mut partitioned = pair();
    single.conduct_heat(1.0, control()).unwrap();
    for _ in 0..100 {
        partitioned.conduct_heat(0.01, control()).unwrap();
    }
    for (a, b) in single.cells().iter().zip(partitioned.cells()) {
        assert!((a.temperature - b.temperature).abs() < 1e-12);
    }
    single.conduct_heat(10.0, control()).unwrap();
    assert!(
        single
            .cells()
            .iter()
            .all(|c| c.temperature >= 1.0 && c.temperature <= 4.0)
    );
}
fn wave(n: usize) -> FiniteDropletGasGrid {
    let dx = 1.0 / n as f64;
    let sinc = (std::f64::consts::PI * dx).sin() / (std::f64::consts::PI * dx);
    FiniteDropletGasGrid::new(
        [0.0; 3],
        [dx, 1.0, 1.0],
        [n, 1, 1],
        (0..n)
            .map(|i| VaporCell {
                mass: dx,
                volume: dx,
                temperature: 2.0
                    + 0.1 * sinc * (2.0 * std::f64::consts::PI * (i as f64 + 0.5) * dx).cos(),
                velocity: [0.0; 3],
                specific_heat_cv: 2.0,
            })
            .collect(),
    )
    .unwrap()
}
#[test]
fn symmetric_pair_time_refinement_matches_discrete_fourier_mode() {
    let n = 8;
    let initial = wave(n);
    let eigenvalue = -4.0 * 0.1 * (n * n) as f64 * (std::f64::consts::PI / n as f64).sin().powi(2);
    let mut previous = None;
    for steps in [8, 16, 32] {
        let mut g = initial.clone();
        let c = GasGridHeatControl {
            conductivity: 0.2,
            max_exchange_number: 1.0,
            boundaries: [GasGridBoundary::Periodic; 3],
            ..Default::default()
        };
        for _ in 0..steps {
            g.conduct_heat(0.1 / steps as f64, c).unwrap();
        }
        let error = g
            .cells()
            .iter()
            .zip(initial.cells())
            .map(|(a, b)| {
                (a.temperature - 2.0 - (b.temperature - 2.0) * (eigenvalue * 0.1).exp()).abs()
            })
            .fold(0.0, f64::max);
        eprintln!("heat time: steps={steps}, max_error={error}");
        if let Some(old) = previous {
            assert!(old / error > 3.8, "ratio={}", old / error);
        }
        previous = Some(error);
    }
}
#[test]
fn spatial_refinement_matches_continuum_heat_equation() {
    let mut previous = None;
    for n in [8, 16, 32] {
        let mut g = wave(n);
        let initial = g.clone();
        let c = GasGridHeatControl {
            conductivity: 0.2,
            max_exchange_number: 0.005,
            boundaries: [GasGridBoundary::Periodic; 3],
            ..Default::default()
        };
        g.conduct_heat(0.02, c).unwrap();
        let decay = (-0.1 * (2.0 * std::f64::consts::PI).powi(2) * 0.02).exp();
        let error = g
            .cells()
            .iter()
            .zip(initial.cells())
            .map(|(a, b)| (a.temperature - 2.0 - (b.temperature - 2.0) * decay).abs())
            .sum::<f64>()
            / n as f64;
        eprintln!("heat mesh: n={n}, l1_error={error}");
        if let Some(old) = previous {
            assert!(old / error > 3.5, "ratio={}", old / error);
        }
        previous = Some(error);
    }
}
#[test]
fn late_substep_budget_and_invalid_conductivity_leave_every_cell_unchanged() {
    let mut g = pair();
    let before = g.clone();
    let mut c = control();
    c.max_substeps = 1;
    assert!(g.conduct_heat(10.0, c).is_err());
    assert_eq!(g, before);
    c.max_substeps = 100;
    c.max_pair_steps = 1;
    assert!(g.conduct_heat(0.2, c).is_err());
    assert_eq!(g, before);
    c.conductivity = f64::NAN;
    assert!(g.conduct_heat(0.2, c).is_err());
    assert_eq!(g, before);
    c.conductivity = 0.0;
    let r = g.conduct_heat(0.2, c).unwrap();
    assert_eq!(r, Default::default());
    assert_eq!(g, before);
}
