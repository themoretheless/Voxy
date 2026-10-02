use physics::liquid::{Config, FiniteDropletGasGrid, Liquid, Material, Particle, VaporCell};
fn cell(velocity: f64) -> VaporCell {
    VaporCell {
        mass: 4.0,
        volume: 1.0,
        temperature: 300.0,
        velocity: [velocity, 0.0, 0.0],
        specific_heat_cv: 2.0,
    }
}
fn grid() -> FiniteDropletGasGrid {
    FiniteDropletGasGrid::new([0.0; 3], [1.0; 3], [2, 1, 1], vec![cell(0.4), cell(-0.3)]).unwrap()
}
fn fluid() -> Liquid {
    let mut l = Liquid::new(
        [
            ([0.2, 0.5, 0.5], [3.0, 0.0, 0.0], 2.0),
            ([1.2, 0.5, 0.5], [-2.0, 0.0, 0.0], 3.0),
            ([3.0, 0.5, 0.5], [4.0, 0.0, 0.0], 1.0),
            ([0.4, 0.5, 0.5], [7.0, 0.0, 0.0], 5.0),
        ]
        .into_iter()
        .map(|(position, velocity, mass)| Particle {
            position,
            velocity,
            mass,
            material: 0,
        })
        .collect(),
        vec![Material::WATER],
        Config::default(),
    )
    .unwrap();
    l.configure_droplet_population(Some(vec![true, true, true, false]))
        .unwrap();
    l
}
fn energy(l: &Liquid, g: &FiniteDropletGasGrid) -> f64 {
    let t = g.totals().unwrap();
    t.thermal_energy
        + t.kinetic_energy
        + l.particles()
            .iter()
            .map(|p| 0.5 * p.mass * p.velocity.iter().map(|v| v * v).sum::<f64>())
            .sum::<f64>()
}
#[test]
fn cartesian_membership_uses_x_fastest_order_and_half_open_faces() {
    let g = FiniteDropletGasGrid::new([-1.0; 3], [1.0; 3], [2; 3], vec![cell(0.0); 8]).unwrap();
    for (p, i) in [
        ([-1.0, -1.0, -1.0], 0),
        ([0.0, -0.5, -0.5], 1),
        ([-0.5, 0.0, -0.5], 2),
        ([0.0, 0.0, 0.0], 7),
    ] {
        assert_eq!(g.cell_index(p).unwrap(), Some(i));
    }
    for p in [
        [1.0, 0.0, 0.0],
        [-1.00001, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ] {
        assert_eq!(g.cell_index(p).unwrap(), None);
    }
    assert!(g.cell_index([f64::NAN, 0.0, 0.0]).is_err());
}
#[test]
fn distinct_cells_match_independent_pair_solutions_and_close_global_ledgers() {
    let mut l = fluid();
    let original = l.clone();
    let mut g = grid();
    let before = g.clone();
    let initial = energy(&l, &g);
    let r = l
        .exchange_marked_droplet_drag_grid(0.2, &mut g, &[1.0, 0.7, 0.2, 0.2], 0.5)
        .unwrap();
    assert_eq!(r.drag.pair_steps, 4);
    assert_eq!(r.affected_cells, 2);
    assert_eq!(r.outside_droplets, 1);
    assert_eq!(l.particles()[2], original.particles()[2]);
    assert_eq!(l.particles()[3], original.particles()[3]);
    let mut independent = original.clone();
    let mut cells = before.cells().to_vec();
    for (i, radius) in [1.0, 0.7].into_iter().enumerate() {
        independent
            .exchange_droplet_drag(i, 0.2, &mut cells[i], radius, 0.5)
            .unwrap();
    }
    for i in 0..2 {
        for k in 0..3 {
            assert!(
                (l.particles()[i].velocity[k] - independent.particles()[i].velocity[k]).abs()
                    < 1e-14
            );
            assert!((g.cells()[i].velocity[k] - cells[i].velocity[k]).abs() < 1e-14);
        }
        assert!((g.cells()[i].temperature - cells[i].temperature).abs() < 1e-12);
    }
    assert!((energy(&l, &g) - initial).abs() < 1e-12);
    let after = g.totals().unwrap();
    let old = before.totals().unwrap();
    assert!((after.mass - old.mass).abs() < 1e-14);
    assert_eq!(after.volume, old.volume);
    assert!((after.thermal_energy - old.thermal_energy - r.drag.dissipated_heat).abs() < 1e-12);
    for k in 0..3 {
        let delta: f64 = l
            .particles()
            .iter()
            .zip(original.particles())
            .map(|(a, b)| a.mass * (a.velocity[k] - b.velocity[k]))
            .sum();
        assert!((delta + r.drag.gas_impulse[k]).abs() < 1e-14);
        assert!((after.momentum[k] - old.momentum[k] - r.drag.gas_impulse[k]).abs() < 1e-14);
    }
}
#[test]
fn one_cell_grid_matches_shared_cell_symmetric_exchange() {
    let mut l = fluid();
    l.configure_droplet_population(Some(vec![true, false, false, true]))
        .unwrap();
    let mut reference = l.clone();
    let mut c = cell(0.4);
    let mut g = FiniteDropletGasGrid::new([0.0; 3], [1.0; 3], [1; 3], vec![c]).unwrap();
    let expected = reference
        .exchange_marked_droplet_drag(0.2, &mut c, &[1.0, 0.7, 0.2, 0.2], 0.5)
        .unwrap();
    let r = l
        .exchange_marked_droplet_drag_grid(0.2, &mut g, &[1.0, 0.7, 0.2, 0.2], 0.5)
        .unwrap();
    assert_eq!(l, reference);
    assert_eq!(g.cells()[0], c);
    assert_eq!(r.drag, expected);
}
#[test]
fn late_cell_heating_failure_restores_every_cell_and_the_complete_liquid() {
    let mut l = fluid();
    let before = l.clone();
    let mut second = cell(-0.3);
    second.temperature = 1e250;
    let mut g =
        FiniteDropletGasGrid::new([0.0; 3], [1.0; 3], [2, 1, 1], vec![cell(0.4), second]).unwrap();
    let old = g.clone();
    assert!(
        l.exchange_marked_droplet_drag_grid(0.2, &mut g, &[1.0, 0.7, 0.2, 0.2], 0.5)
            .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(g, old);
}
#[test]
fn invalid_geometry_or_radii_and_pair_budget_are_atomic() {
    assert!(FiniteDropletGasGrid::new([0.0; 3], [1.0; 3], [2, 1, 1], vec![cell(0.0)]).is_err());
    assert!(FiniteDropletGasGrid::new([0.0; 3], [2.0, 1.0, 1.0], [1; 3], vec![cell(0.0)]).is_err());
    assert!(FiniteDropletGasGrid::new([1e300; 3], [1.0; 3], [1; 3], vec![cell(0.0)]).is_err());
    let mut l = fluid();
    let before = l.clone();
    let mut g = grid();
    let old = g.clone();
    assert!(
        l.exchange_marked_droplet_drag_grid(0.2, &mut g, &[1.0, f64::MAX, 0.2, 0.2], 0.5)
            .is_err()
    );
    assert_eq!(l, before);
    assert_eq!(g, old);
    let mut limited = Liquid::new(
        l.particles().to_vec(),
        vec![Material::WATER],
        Config {
            max_neighbor_checks: 3,
            ..Config::default()
        },
    )
    .unwrap();
    limited
        .configure_droplet_population(Some(vec![true, true, true, false]))
        .unwrap();
    let original = limited.clone();
    assert!(
        limited
            .exchange_marked_droplet_drag_grid(0.2, &mut g, &[1.0, 0.7, 0.2, 0.2], 0.5)
            .is_err()
    );
    assert_eq!(limited, original);
    assert_eq!(g, old);
}
#[test]
fn unused_gas_cells_remain_exactly_unchanged() {
    let mut l = fluid();
    l.configure_droplet_population(Some(vec![true, false, false, false]))
        .unwrap();
    let mut g = grid();
    let untouched = g.cells()[1];
    let r = l
        .exchange_marked_droplet_drag_grid(0.2, &mut g, &[1.0, 0.7, 0.2, 0.2], 0.5)
        .unwrap();
    assert_eq!(r.affected_cells, 1);
    assert_eq!(g.cells()[1], untouched);
}
