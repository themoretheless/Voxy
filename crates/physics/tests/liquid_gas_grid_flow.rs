use physics::liquid::{FiniteDropletGasGrid, GasGridBoundary, GasGridFlowControl, VaporCell};
fn control() -> GasGridFlowControl {
    GasGridFlowControl {
        gas_constant: 1.0,
        boundaries: [GasGridBoundary::Periodic; 3],
        ..Default::default()
    }
}
fn grid(n: usize, temps: Vec<f64>, velocity: [f64; 3]) -> FiniteDropletGasGrid {
    let volume = 1.0 / n as f64;
    FiniteDropletGasGrid::new(
        [0.0; 3],
        [volume, 1.0, 1.0],
        [n, 1, 1],
        temps
            .into_iter()
            .map(|temperature| VaporCell {
                mass: volume,
                volume,
                temperature,
                velocity,
                specific_heat_cv: 2.0,
            })
            .collect(),
    )
    .unwrap()
}
#[test]
fn uniform_periodic_state_stays_uniform_in_three_dimensions() {
    let spacing = [0.2, 0.3, 0.4];
    let volume = spacing.iter().product::<f64>();
    let c = VaporCell {
        mass: volume,
        volume,
        temperature: 1.0,
        velocity: [0.2, -0.1, 0.05],
        specific_heat_cv: 2.0,
    };
    let mut g = FiniteDropletGasGrid::new([0.0; 3], spacing, [4, 3, 2], vec![c; 24]).unwrap();
    let r = g.advance_euler(0.1, control()).unwrap();
    assert!(r.substeps > 1);
    assert_eq!(r.wall_impulse, [0.0; 3]);
    for after in g.cells() {
        assert!((after.mass - c.mass).abs() < 1e-14);
        assert!((after.temperature - c.temperature).abs() < 1e-14);
        for k in 0..3 {
            assert!((after.velocity[k] - c.velocity[k]).abs() < 1e-14);
        }
    }
}
#[test]
fn pressure_pulse_launches_opposed_flow_and_preserves_mass_momentum_energy() {
    let mut temps = vec![1.0; 16];
    temps[8] = 2.0;
    let mut g = grid(16, temps, [0.0; 3]);
    let before = g.totals().unwrap();
    g.advance_euler(0.03, control()).unwrap();
    let after = g.totals().unwrap();
    assert!(g.cells()[7].velocity[0] < 0.0);
    assert!(g.cells()[9].velocity[0] > 0.0);
    assert!(after.kinetic_energy > 0.0);
    assert!(g.cells()[8].temperature < 2.0);
    assert!((after.mass - before.mass).abs() < 1e-13);
    assert!((after.thermal_energy + after.kinetic_energy - before.thermal_energy).abs() < 1e-13);
    assert!(after.momentum.iter().all(|p| p.abs() < 1e-13));
}
#[test]
fn reflecting_walls_receive_impulse_without_energy_or_mass_flux() {
    let mut g = grid(1, vec![1.0], [2.0, 0.0, 0.0]);
    let before = g.totals().unwrap();
    let mut c = control();
    c.boundaries = [GasGridBoundary::Reflecting; 3];
    let r = g.advance_euler(0.01, c).unwrap();
    let after = g.totals().unwrap();
    assert!(r.wall_impulse[0] > 0.0);
    assert!(g.cells()[0].velocity[0] < 2.0);
    assert!(g.cells()[0].temperature > 1.0);
    assert!((after.mass - before.mass).abs() < 1e-14);
    assert!(
        (after.thermal_energy + after.kinetic_energy
            - before.thermal_energy
            - before.kinetic_energy)
            .abs()
            < 1e-13
    );
    for k in 0..3 {
        assert!((after.momentum[k] + r.wall_impulse[k] - before.momentum[k]).abs() < 1e-13);
    }
}
#[test]
fn advected_entropy_wave_converges_to_exact_cell_averages() {
    let mut previous = None;
    for n in [16, 32, 64] {
        let dx = 1.0 / n as f64;
        let sinc = (std::f64::consts::PI * dx).sin() / (std::f64::consts::PI * dx);
        let cells = (0..n)
            .map(|i| {
                let x = (i as f64 + 0.5) * dx;
                let density = 1.0 + 0.05 * sinc * (2.0 * std::f64::consts::PI * x).sin();
                VaporCell {
                    mass: density * dx,
                    volume: dx,
                    temperature: 1.0 / density,
                    velocity: [0.3, 0.0, 0.0],
                    specific_heat_cv: 2.0,
                }
            })
            .collect();
        let mut g = FiniteDropletGasGrid::new([0.0; 3], [dx, 1.0, 1.0], [n, 1, 1], cells).unwrap();
        g.advance_euler(0.1, control()).unwrap();
        let error = g
            .cells()
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let x = (i as f64 + 0.5) * dx - 0.03;
                let exact = 1.0 + 0.05 * sinc * (2.0 * std::f64::consts::PI * x).sin();
                (c.mass / dx - exact).abs() * dx
            })
            .sum::<f64>();
        eprintln!("entropy wave: n={n}, density_l1_error={error}");
        if let Some(old) = previous {
            assert!(old / error > 1.7, "ratio={}", old / error);
        }
        previous = Some(error);
    }
}
#[test]
fn late_substep_and_face_budget_errors_roll_back_every_cell() {
    let mut g = grid(4, vec![1.0, 2.0, 1.0, 1.0], [0.0; 3]);
    let before = g.clone();
    let mut c = control();
    c.max_substeps = 1;
    assert!(g.advance_euler(1.0, c).is_err());
    assert_eq!(g, before);
    c.max_substeps = 100;
    c.max_face_updates = 1;
    assert!(g.advance_euler(0.1, c).is_err());
    assert_eq!(g, before);
    c.courant = 0.5;
    assert!(g.advance_euler(0.1, c).is_err());
    assert_eq!(g, before);
}
#[test]
fn varying_heat_capacity_is_rejected_without_implicit_mixing() {
    let mut cells = grid(2, vec![1.0; 2], [0.0; 3]).cells().to_vec();
    cells[1].specific_heat_cv = 3.0;
    let mut g = FiniteDropletGasGrid::new([0.0; 3], [0.5, 1.0, 1.0], [2, 1, 1], cells).unwrap();
    let before = g.clone();
    assert!(g.advance_euler(0.1, control()).is_err());
    assert_eq!(g, before);
}

#[test]
fn inactive_periodic_axes_do_not_consume_a_false_acoustic_cfl_budget() {
    let mut g = grid(1, vec![1.0], [2.0, 0.0, 0.0]);
    let before = g.clone();
    let mut c = control();
    c.max_substeps = 1;
    let r = g.advance_euler(1000.0, c).unwrap();
    assert_eq!(g, before);
    assert_eq!(r, Default::default());
}
#[test]
fn supersonic_counterflow_keeps_positive_states_and_conserves_energy() {
    let mut g = grid(2, vec![0.01; 2], [0.0; 3]);
    let mut cells = g.cells().to_vec();
    cells[0].velocity[0] = -5.0;
    cells[1].velocity[0] = 5.0;
    g = FiniteDropletGasGrid::new([0.0; 3], [0.5, 1.0, 1.0], [2, 1, 1], cells).unwrap();
    let before = g.totals().unwrap();
    g.advance_euler(0.1, control()).unwrap();
    let after = g.totals().unwrap();
    assert!(
        g.cells()
            .iter()
            .all(|c| c.mass > 0.0 && c.temperature > 0.0)
    );
    assert!((after.mass - before.mass).abs() < 1e-13);
    assert!(
        (after.thermal_energy + after.kinetic_energy
            - before.thermal_energy
            - before.kinetic_energy)
            .abs()
            < 1e-12
    );
}
