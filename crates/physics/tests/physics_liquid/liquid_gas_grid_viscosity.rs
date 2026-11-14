use physics::liquid::{
    FiniteDropletGasGrid, GasGridViscosityBoundary, GasGridViscosityControl, VaporCell,
};
fn control() -> GasGridViscosityControl {
    GasGridViscosityControl {
        shear_viscosity: 0.1,
        max_relaxation_number: 0.1,
        ..Default::default()
    }
}
fn grid(
    shape: [usize; 3],
    spacing: [f64; 3],
    velocity: impl Fn([usize; 3]) -> [f64; 3],
) -> FiniteDropletGasGrid {
    let volume = spacing.iter().product::<f64>();
    FiniteDropletGasGrid::new(
        [0.0; 3],
        spacing,
        shape,
        (0..shape.iter().product())
            .map(|i| VaporCell {
                mass: volume,
                volume,
                temperature: 2.0,
                specific_heat_cv: 3.0,
                velocity: velocity([
                    i % shape[0],
                    (i / shape[0]) % shape[1],
                    i / (shape[0] * shape[1]),
                ]),
            })
            .collect(),
    )
    .unwrap()
}
fn balance(before: &FiniteDropletGasGrid, after: &FiniteDropletGasGrid, heat: f64) {
    let a = before.totals().unwrap();
    let b = after.totals().unwrap();
    for k in 0..3 {
        assert!((a.momentum[k] - b.momentum[k]).abs() < 1e-11);
    }
    assert!((a.kinetic_energy - b.kinetic_energy - heat).abs() < 1e-11);
    assert!((b.thermal_energy - a.thermal_energy - heat).abs() < 1e-11);
    for (a, b) in before.cells().iter().zip(after.cells()) {
        assert_eq!(a.mass, b.mass);
        assert_eq!(a.volume, b.volume);
        assert_eq!(a.specific_heat_cv, b.specific_heat_cv);
        assert!(b.temperature >= a.temperature);
    }
}
#[test]
fn unequal_mass_pair_stokes_longitudinal_and_transverse_decay_and_heat() {
    for axis in [0, 1] {
        let mut g = FiniteDropletGasGrid::new(
            [0.0; 3],
            [1.0; 3],
            [2, 1, 1],
            (0..2)
                .map(|i| {
                    let mut velocity = [0.0; 3];
                    velocity[axis] = if i == 0 { 2.0 } else { -1.0 };
                    VaporCell {
                        mass: if i == 0 { 1.0 } else { 2.0 },
                        volume: 1.0,
                        temperature: 2.0,
                        specific_heat_cv: if i == 0 { 2.0 } else { 5.0 },
                        velocity,
                    }
                })
                .collect(),
        )
        .unwrap();
        let before = g.clone();
        let c = GasGridViscosityControl {
            max_relaxation_number: 1.0,
            ..control()
        };
        let r = g.relax_viscosity(0.1, c).unwrap();
        let coefficient = 0.1_f64 * if axis == 0 { 4.0 / 3.0 } else { 1.0 };
        // Two gradient cells; each appears once in each symmetric half sweep.
        let z = 0.5 * 0.05 * coefficient * (1.0 + 0.5);
        let decay = ((1.0 - z) / (1.0 + z)).powi(4);
        assert!((g.cells()[0].velocity[axis] - 2.0 * decay).abs() < 1e-13);
        assert!((g.cells()[1].velocity[axis] + decay).abs() < 1e-13);
        assert_eq!(r.substeps, 1);
        assert_eq!(r.block_steps, 4);
        assert!(r.dissipated_heat > 0.0);
        balance(&before, &g, r.dissipated_heat);
    }
}
#[test]
fn rigid_translation_and_rotation_have_no_viscous_heat_in_stress_free_box() {
    let mut g = grid([4, 3, 2], [0.2, 0.3, 0.4], |p| {
        let x = p[0] as f64 * 0.2;
        let y = p[1] as f64 * 0.3;
        [2.0 - 0.4 * y, -1.0 + 0.4 * x, 3.0]
    });
    let before = g.clone();
    let r = g.relax_viscosity(0.1, control()).unwrap();
    assert!(r.dissipated_heat < 1e-27);
    for (a, b) in before.cells().iter().zip(g.cells()) {
        for k in 0..3 {
            assert!((a.velocity[k] - b.velocity[k]).abs() < 1e-14);
        }
        assert_eq!(a.temperature, b.temperature);
    }
}
#[test]
fn zero_shear_bulk_only_does_not_damp_a_transverse_wave() {
    let mut g = grid([8, 1, 1], [0.125, 1.0, 1.0], |p| {
        [0.0, (p[0] as f64).sin(), 0.0]
    });
    let before = g.clone();
    let c = GasGridViscosityControl {
        shear_viscosity: 0.0,
        bulk_viscosity: 0.2,
        boundaries: [GasGridViscosityBoundary::Periodic; 3],
        ..control()
    };
    let r = g.relax_viscosity(0.1, c).unwrap();
    assert_eq!(r.dissipated_heat, 0.0);
    assert_eq!(g, before);
}
fn wave(n: usize, two_dimensional: bool) -> FiniteDropletGasGrid {
    let dx = 1.0 / n as f64;
    let sinc = (std::f64::consts::PI * dx).sin() / (std::f64::consts::PI * dx);
    grid(
        [n, if two_dimensional { n } else { 1 }, 1],
        [dx, if two_dimensional { dx } else { 1.0 }, 1.0],
        |p| {
            let phase = 2.0
                * std::f64::consts::PI
                * (p[0] as f64
                    + 0.5
                    + if two_dimensional {
                        p[1] as f64 + 0.5
                    } else {
                        0.0
                    })
                * dx;
            if two_dimensional {
                [0.1 * sinc * sinc * phase.cos(), 0.0, 0.0]
            } else {
                [0.0, 0.1 * sinc * phase.cos(), 0.0]
            }
        },
    )
}
#[test]
fn transverse_wave_spatial_refinement_matches_viscous_diffusion() {
    let mut previous = None;
    for n in [8, 16, 32] {
        let mut g = wave(n, false);
        let before = g.clone();
        let c = GasGridViscosityControl {
            boundaries: [GasGridViscosityBoundary::Periodic; 3],
            max_relaxation_number: 0.05,
            ..control()
        };
        let r = g.relax_viscosity(0.02, c).unwrap();
        balance(&before, &g, r.dissipated_heat);
        let decay = (-0.1 * (2.0 * std::f64::consts::PI).powi(2) * 0.02).exp();
        let error = g
            .cells()
            .iter()
            .zip(before.cells())
            .map(|(a, b)| (a.velocity[1] - b.velocity[1] * decay).abs())
            .sum::<f64>()
            / n as f64;
        eprintln!("viscous shear mesh: n={n}, error={error}");
        if let Some(old) = previous {
            assert!(old / error > 3.5, "ratio={}", old / error);
        }
        previous = Some(error);
    }
}
#[test]
fn full_stress_cross_derivatives_generate_transverse_response_and_converge() {
    let mut previous = None;
    for n in [8, 16, 32] {
        let mut g = wave(n, true);
        let before = g.clone();
        let mu = 0.02;
        let bulk = 0.01;
        let time = 0.01;
        let c = GasGridViscosityControl {
            shear_viscosity: mu,
            bulk_viscosity: bulk,
            boundaries: [GasGridViscosityBoundary::Periodic; 3],
            ..control()
        };
        let r = g.relax_viscosity(time, c).unwrap();
        balance(&before, &g, r.dissipated_heat);
        let k2 = 2.0 * (2.0 * std::f64::consts::PI).powi(2);
        let transverse = (-mu * k2 * time).exp();
        let longitudinal = (-(4.0 / 3.0 * mu + bulk) * k2 * time).exp();
        let error = g
            .cells()
            .iter()
            .zip(before.cells())
            .map(|(a, b)| {
                (a.velocity[0] - 0.5 * b.velocity[0] * (longitudinal + transverse)).abs()
                    + (a.velocity[1] - 0.5 * b.velocity[0] * (longitudinal - transverse)).abs()
            })
            .sum::<f64>()
            / (n * n) as f64;
        assert!(g.cells().iter().any(|c| c.velocity[1].abs() > 1e-5));
        eprintln!("viscous full stress mesh: n={n}, error={error}");
        if let Some(old) = previous {
            assert!(old / error > 3.4, "ratio={}", old / error);
        }
        previous = Some(error);
    }
}
#[test]
fn midpoint_block_time_refinement_is_second_order() {
    let initial = wave(8, false);
    let eigenvalue = -4.0 * 0.1 * 64.0 * (std::f64::consts::PI / 8.0).sin().powi(2);
    let mut previous = None;
    for steps in [8, 16, 32] {
        let mut g = initial.clone();
        let c = GasGridViscosityControl {
            max_relaxation_number: 10.0,
            boundaries: [GasGridViscosityBoundary::Periodic; 3],
            ..control()
        };
        for _ in 0..steps {
            g.relax_viscosity(0.1 / steps as f64, c).unwrap();
        }
        let error = g
            .cells()
            .iter()
            .zip(initial.cells())
            .map(|(a, b)| (a.velocity[1] - b.velocity[1] * (eigenvalue * 0.1).exp()).abs())
            .fold(0.0, f64::max);
        eprintln!("viscous time: steps={steps}, error={error}");
        if let Some(old) = previous {
            assert!(old / error > 3.8, "ratio={}", old / error);
        }
        previous = Some(error);
    }
}
#[test]
fn late_budgets_and_invalid_controls_roll_back_the_whole_grid() {
    let mut g = wave(8, true);
    let before = g.clone();
    let mut c = control();
    c.max_substeps = 1;
    assert!(g.relax_viscosity(1.0, c).is_err());
    assert_eq!(g, before);
    c.max_substeps = 100;
    c.max_block_steps = 130;
    assert!(g.relax_viscosity(1.0, c).is_err());
    assert_eq!(g, before);
    c.max_block_steps = 1_000_000;
    c.shear_viscosity = f64::NAN;
    assert!(g.relax_viscosity(0.01, c).is_err());
    assert_eq!(g, before);
    c.shear_viscosity = 0.0;
    c.bulk_viscosity = 0.0;
    assert_eq!(g.relax_viscosity(0.1, c).unwrap(), Default::default());
    assert_eq!(g, before);
}
#[test]
fn singleton_periodic_grid_has_no_stress_or_budget_consumption() {
    let mut g = grid([1, 1, 1], [1.0; 3], |_| [1.0, 2.0, 3.0]);
    let before = g.clone();
    let c = GasGridViscosityControl {
        boundaries: [GasGridViscosityBoundary::Periodic; 3],
        max_substeps: 1,
        max_block_steps: 1,
        ..control()
    };
    assert_eq!(g.relax_viscosity(1000.0, c).unwrap(), Default::default());
    assert_eq!(g, before);
}

#[test]
fn oblique_wave_with_unequal_axis_wavenumbers_converges_to_full_stress_solution() {
    let mut previous = None;
    for n in [8, 16, 32] {
        let dx = 1.0 / n as f64;
        let pi = std::f64::consts::PI;
        let average = (pi * dx).sin() / (pi * dx) * (2.0 * pi * dx).sin() / (2.0 * pi * dx);
        let mut g = grid([n, n, 1], [dx, dx, 1.0], |p| {
            let phase = 2.0 * pi * (p[0] as f64 + 0.5 + 2.0 * (p[1] as f64 + 0.5)) * dx;
            [0.1 * average * phase.cos(), 0.0, 0.0]
        });
        let initial = g.clone();
        let c = GasGridViscosityControl {
            shear_viscosity: 0.02,
            bulk_viscosity: 0.01,
            boundaries: [GasGridViscosityBoundary::Periodic; 3],
            ..control()
        };
        g.relax_viscosity(0.01, c).unwrap();
        let k2 = 5.0 * (2.0 * pi).powi(2);
        let transverse = (-0.02 * k2 * 0.01).exp();
        let longitudinal = (-(4.0 / 3.0 * 0.02 + 0.01) * k2 * 0.01).exp();
        let error = g
            .cells()
            .iter()
            .zip(initial.cells())
            .map(|(a, b)| {
                (a.velocity[0] - b.velocity[0] * (0.8 * transverse + 0.2 * longitudinal)).abs()
                    + (a.velocity[1] - b.velocity[0] * 0.4 * (longitudinal - transverse)).abs()
            })
            .sum::<f64>()
            / (n * n) as f64;
        eprintln!("viscous oblique mesh: n={n}, error={error}");
        if let Some(old) = previous {
            assert!(old / error > 1.7, "ratio={}", old / error);
        }
        previous = Some(error);
    }
}

#[test]
fn three_dimensional_stress_free_relaxation_conserves_angular_momentum_and_is_galilean() {
    let mut g = grid([3, 2, 2], [0.2, 0.3, 0.4], |p| {
        [
            (p[0] as f64 + 2.0 * p[1] as f64).sin(),
            (p[2] as f64 - p[0] as f64).cos(),
            0.3 * p[1] as f64,
        ]
    });
    let angular = |g: &FiniteDropletGasGrid| {
        let mut l = [0.0; 3];
        for (i, c) in g.cells().iter().enumerate() {
            let x = [
                (i % 3) as f64 * 0.2,
                ((i / 3) % 2) as f64 * 0.3,
                (i / 6) as f64 * 0.4,
            ];
            for k in 0..3 {
                l[k] += c.mass
                    * (x[(k + 1) % 3] * c.velocity[(k + 2) % 3]
                        - x[(k + 2) % 3] * c.velocity[(k + 1) % 3]);
            }
        }
        l
    };
    let initial = g.clone();
    let before = angular(&g);
    let boost = [3.0, -2.0, 4.0];
    let mut shifted = grid([3, 2, 2], [0.2, 0.3, 0.4], |p| {
        let i = p[0] + 3 * (p[1] + 2 * p[2]);
        std::array::from_fn(|k| initial.cells()[i].velocity[k] + boost[k])
    });
    let report = g.relax_viscosity(0.03, control()).unwrap();
    let shifted_report = shifted.relax_viscosity(0.03, control()).unwrap();
    balance(&initial, &g, report.dissipated_heat);
    for k in 0..3 {
        assert!((before[k] - angular(&g)[k]).abs() < 1e-13);
    }
    assert!((report.dissipated_heat - shifted_report.dissipated_heat).abs() < 1e-13);
    for (a, b) in g.cells().iter().zip(shifted.cells()) {
        for k in 0..3 {
            assert!((b.velocity[k] - a.velocity[k] - boost[k]).abs() < 1e-13);
        }
        assert!((a.temperature - b.temperature).abs() < 1e-13);
    }
}
