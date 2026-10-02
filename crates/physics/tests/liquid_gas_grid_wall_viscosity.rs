use physics::liquid::{
    FiniteDropletGasGrid, GasGridViscosityControl, GasGridWallViscosityControl,
    GasGridWallViscosityReport, VaporCell,
};
fn one(velocity: [f64; 3]) -> FiniteDropletGasGrid {
    FiniteDropletGasGrid::new(
        [0.0; 3],
        [1.0; 3],
        [1, 1, 1],
        vec![VaporCell {
            mass: 2.0,
            volume: 1.0,
            temperature: 3.0,
            specific_heat_cv: 4.0,
            velocity,
        }],
    )
    .unwrap()
}
fn control() -> GasGridWallViscosityControl {
    GasGridWallViscosityControl {
        shear_viscosity: 0.1,
        bulk_viscosity: 0.03,
        walls: [None; 6],
        ..Default::default()
    }
}
fn balance(
    before: &FiniteDropletGasGrid,
    after: &FiniteDropletGasGrid,
    r: GasGridWallViscosityReport,
) {
    let a = before.totals().unwrap();
    let b = after.totals().unwrap();
    for k in 0..3 {
        assert!((b.momentum[k] - a.momentum[k] + r.wall_impulse[k]).abs() < 1e-11);
        assert!(
            (r.face_impulses.iter().map(|p| p[k]).sum::<f64>() - r.wall_impulse[k]).abs() < 1e-11
        );
    }
    assert!(
        (b.kinetic_energy - a.kinetic_energy + r.dissipated_heat - r.work_on_gas).abs() < 1e-11
    );
    assert!((b.thermal_energy - a.thermal_energy - r.dissipated_heat).abs() < 1e-11);
    assert!(
        (b.kinetic_energy + b.thermal_energy - a.kinetic_energy - a.thermal_energy - r.work_on_gas)
            .abs()
            < 1e-11
    );
    for (a, b) in before.cells().iter().zip(after.cells()) {
        assert_eq!(a.mass, b.mass);
        assert_eq!(a.volume, b.volume);
        assert_eq!(a.specific_heat_cv, b.specific_heat_cv);
    }
}
#[test]
fn stationary_wall_exact_anisotropic_decay_heat_and_reaction() {
    let mut g = one([1.0, 2.0, -3.0]);
    let before = g.clone();
    let mut c = control();
    c.walls[0] = Some([0.0; 3]);
    let r = g.relax_wall_viscosity(0.2, c).unwrap();
    for k in 0..3 {
        let viscosity = if k == 0 { 4.0 / 3.0 * 0.1 + 0.03 } else { 0.1 };
        let decay = (-viscosity * 0.2_f64).exp(); // A=1,d=1/2,m=2.
        assert!((g.cells()[0].velocity[k] - before.cells()[0].velocity[k] * decay).abs() < 1e-13);
    }
    assert_eq!(r.face_steps, 2);
    assert_eq!(r.substeps, 1);
    assert_eq!(r.work_on_gas, 0.0);
    assert!(r.dissipated_heat > 0.0);
    balance(&before, &g, r);
}
#[test]
fn moving_tangential_wall_supplies_work_without_double_counting_heat() {
    let mut g = one([0.0; 3]);
    let before = g.clone();
    let mut c = control();
    c.walls[2] = Some([2.0, 0.0, -1.0]);
    let r = g.relax_wall_viscosity(0.3, c).unwrap();
    let exchange = -(-0.1_f64 * 0.3).exp_m1();
    assert!((g.cells()[0].velocity[0] - 2.0 * exchange).abs() < 1e-13);
    assert!((g.cells()[0].velocity[2] + exchange).abs() < 1e-13);
    assert!(r.work_on_gas > r.dissipated_heat && r.dissipated_heat > 0.0);
    balance(&before, &g, r);
}
#[test]
fn single_face_semigroup_and_all_stationary_faces_damp_a_singleton() {
    let mut a = one([1.0, 2.0, 3.0]);
    let mut b = a.clone();
    let mut c = control();
    c.walls[5] = Some([0.0; 3]);
    a.relax_wall_viscosity(1.0, c).unwrap();
    for _ in 0..100 {
        b.relax_wall_viscosity(0.01, c).unwrap();
    }
    for k in 0..3 {
        assert!((a.cells()[0].velocity[k] - b.cells()[0].velocity[k]).abs() < 1e-13);
    }
    assert!((a.cells()[0].temperature - b.cells()[0].temperature).abs() < 1e-12);
    let mut g = one([1.0, 2.0, 3.0]);
    let before = g.clone();
    c.walls = [Some([0.0; 3]); 6];
    let r = g.relax_wall_viscosity(0.2, c).unwrap();
    assert!(r.face_steps >= 12);
    balance(&before, &g, r);
    assert!(
        g.cells()[0]
            .velocity
            .iter()
            .zip(before.cells()[0].velocity)
            .all(|(a, b)| a.abs() < b.abs())
    );
}
#[test]
fn unequal_moving_walls_converge_to_simultaneous_impulse_work_and_heating() {
    let mut previous = None;
    for steps in [8, 16, 32] {
        let mut g = one([0.3, 0.0, 0.0]);
        let before = g.clone();
        let mut c = control();
        c.bulk_viscosity = 0.0;
        c.max_relaxation_number = 10.0;
        c.walls[2] = Some([0.0; 3]);
        c.walls[3] = Some([1.0, 0.0, 0.0]);
        let mut total = GasGridWallViscosityReport::default();
        for _ in 0..steps {
            let r = g.relax_wall_viscosity(0.5 / steps as f64, c).unwrap();
            total.work_on_gas += r.work_on_gas;
            total.dissipated_heat += r.dissipated_heat;
            for k in 0..3 {
                total.wall_impulse[k] += r.wall_impulse[k];
                for face in 0..6 {
                    total.face_impulses[face][k] += r.face_impulses[face][k];
                }
            }
        }
        balance(&before, &g, total);
        let time = 0.5;
        let rate = 0.1_f64;
        let lambda = 2.0 * rate;
        let equilibrium = 0.5;
        let relative = 0.3 - equilibrium;
        let i1 = -(-lambda * time).exp_m1() / lambda;
        let i2 = -(-2.0 * lambda * time).exp_m1() / (2.0 * lambda);
        let expected_v = equilibrium + relative * (-lambda * time).exp();
        let expected_work = 2.0 * rate * ((1.0 - equilibrium) * time - relative * i1);
        let expected_heat = [0.0, 1.0]
            .iter()
            .map(|u| {
                2.0 * rate
                    * ((equilibrium - u).powi(2) * time
                        + 2.0 * (equilibrium - u) * relative * i1
                        + relative * relative * i2)
            })
            .sum::<f64>();
        let error = (g.cells()[0].velocity[0] - expected_v).abs()
            + (total.work_on_gas - expected_work).abs()
            + (total.dissipated_heat - expected_heat).abs();
        eprintln!("wall time steps={steps}, error={error}");
        if let Some(old) = previous {
            assert!(old / error > 3.8, "ratio={}", old / error);
        }
        previous = Some(error);
    }
}
#[test]
fn disabled_faces_and_nonboundary_cells_remain_unchanged() {
    let base = one([1.0; 3]).cells()[0];
    let mut g = FiniteDropletGasGrid::new([0.0; 3], [1.0; 3], [3, 1, 1], vec![base; 3]).unwrap();
    let before = g.clone();
    let mut c = control();
    assert_eq!(g.relax_wall_viscosity(0.1, c).unwrap(), Default::default());
    assert_eq!(g, before);
    c.walls[0] = Some([0.0; 3]);
    g.relax_wall_viscosity(0.1, c).unwrap();
    assert_eq!(&g.cells()[1..], &before.cells()[1..]);
}
#[test]
fn late_budgets_invalid_coefficients_and_normal_wall_motion_roll_back() {
    let mut g = one([1.0; 3]);
    let before = g.clone();
    let mut c = control();
    c.walls[0] = Some([0.0; 3]);
    c.max_substeps = 1;
    assert!(g.relax_wall_viscosity(100.0, c).is_err());
    assert_eq!(g, before);
    c.max_substeps = 4096;
    c.max_face_steps = 3;
    assert!(g.relax_wall_viscosity(10.0, c).is_err());
    assert_eq!(g, before);
    c.max_face_steps = 1_000_000;
    c.walls[0] = Some([0.1, 0.0, 0.0]);
    assert!(g.relax_wall_viscosity(0.1, c).is_err());
    assert_eq!(g, before);
    c.walls[0] = Some([0.0; 3]);
    c.bulk_viscosity = f64::NAN;
    assert!(g.relax_wall_viscosity(0.1, c).is_err());
    assert_eq!(g, before);
    c.bulk_viscosity = 0.0;
    c.shear_viscosity = 0.0;
    assert_eq!(g.relax_wall_viscosity(0.1, c).unwrap(), Default::default());
    assert_eq!(g, before);
}
fn couette(n: usize) -> FiniteDropletGasGrid {
    let dy = 1.0 / n as f64;
    FiniteDropletGasGrid::new(
        [0.0; 3],
        [1.0, dy, 1.0],
        [1, n, 1],
        vec![
            VaporCell {
                mass: dy,
                volume: dy,
                temperature: 2.0,
                specific_heat_cv: 3.0,
                velocity: [0.0; 3]
            };
            n
        ],
    )
    .unwrap()
}
#[test]
fn transient_couette_flow_refines_toward_the_continuum_solution() {
    let mut previous = None;
    for n in [32, 64, 128] {
        let dy = 1.0 / n as f64;
        let viscosity = 0.05;
        let time = 0.25;
        let mut g = couette(n);
        let mut wall = control();
        wall.shear_viscosity = viscosity;
        wall.bulk_viscosity = 0.0;
        wall.walls[2] = Some([0.0; 3]);
        wall.walls[3] = Some([1.0, 0.0, 0.0]);
        let interior = GasGridViscosityControl {
            shear_viscosity: viscosity,
            bulk_viscosity: 0.0,
            ..Default::default()
        };
        let steps = (time / (0.02 * dy * dy / viscosity)).ceil() as usize;
        let dt = time / steps as f64;
        for _ in 0..steps {
            g.relax_viscosity_with_walls(dt, interior, wall).unwrap();
        }
        let pi = std::f64::consts::PI;
        let error = g
            .cells()
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let y = (i as f64 + 0.5) * dy;
                let mut exact = y;
                for j in 1..=100 {
                    let k = j as f64 * pi;
                    let average = (0.5 * k * dy).sin() / (0.5 * k * dy);
                    exact += 2.0 * if j % 2 == 0 { 1.0 } else { -1.0 } / k
                        * (k * y).sin()
                        * average
                        * (-viscosity * k * k * time).exp();
                }
                (c.velocity[0] - exact).abs()
            })
            .sum::<f64>()
            / n as f64;
        eprintln!("Couette n={n}, steps={steps}, error={error}");
        if let Some(old) = previous {
            assert!(old / error > 1.5, "ratio={}", old / error);
        }
        previous = Some(error);
    }
}

#[test]
fn composed_stress_and_wall_stage_is_atomic_and_rejects_periodic_wall_conflicts() {
    let mut g = couette(4);
    let before = g.clone();
    let mut wall = control();
    wall.bulk_viscosity = 0.0;
    wall.walls[3] = Some([1.0, 0.0, 0.0]);
    wall.max_substeps = 1;
    let interior = GasGridViscosityControl {
        shear_viscosity: 0.1,
        ..Default::default()
    };
    // First wall and interior stages can succeed; the second half exceeds the
    // total wall-stage budget and must roll back both earlier changes.
    assert!(g.relax_viscosity_with_walls(0.001, interior, wall).is_err());
    assert_eq!(g, before);
    wall.max_substeps = 4096;
    let mut periodic = interior;
    periodic.boundaries[1] = physics::liquid::GasGridViscosityBoundary::Periodic;
    assert!(g.relax_viscosity_with_walls(0.001, periodic, wall).is_err());
    assert_eq!(g, before);
    wall.shear_viscosity = 0.2;
    assert!(g.relax_viscosity_with_walls(0.001, interior, wall).is_err());
    assert_eq!(g, before);
    wall.shear_viscosity = 0.1;
    let r = g.relax_viscosity_with_walls(0.001, interior, wall).unwrap();
    let mut energy_report = r.walls;
    energy_report.dissipated_heat += r.interior.dissipated_heat;
    balance(&before, &g, energy_report);
}
