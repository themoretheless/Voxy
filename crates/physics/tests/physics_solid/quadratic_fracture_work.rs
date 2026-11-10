use physics::{
    cohesive::Material as Bond,
    plasticity::{
        Material,
        mesh::{QuadraticBody, QuadraticDynamics},
    },
};
fn coupon() -> (QuadraticBody, Vec<bool>, [usize; 6], [usize; 6]) {
    let material = Material::new(1e7, 0.3, 1e9, 0.).unwrap();
    let mut body = QuadraticBody::from_linear(
        vec![
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [0.; 3],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., -1.],
        ],
        vec![([0, 1, 2, 3], material), ([4, 5, 6, 7], material)],
    )
    .unwrap();
    let edges = body.edge_midpoints();
    let midpoint = |a: usize, b: usize| {
        edges
            .iter()
            .find(|(edge, _)| *edge == [a.min(b), a.max(b)])
            .unwrap()
            .1
    };
    let minus = [4, 5, 6, midpoint(4, 5), midpoint(5, 6), midpoint(4, 6)];
    let plus = [0, 1, 2, midpoint(0, 1), midpoint(1, 2), midpoint(0, 2)];
    let mut upper = vec![false; body.positions().len()];
    for value in &mut upper[..4] {
        *value = true;
    }
    for (edge, node) in edges {
        upper[node] = edge[0] < 4 && edge[1] < 4;
    }
    body.add_cohesive_interface(minus, plus, Bond::new(1e6, 2e6, 1000., 10.).unwrap())
        .unwrap();
    (body, upper, minus, plus)
}

fn dynamic() -> QuadraticDynamics {
    let (body, upper, _, _) = coupon();
    let velocities = upper
        .iter()
        .map(|u| [0.03, -0.02, if *u { 0.5 } else { -0.5 }])
        .collect();
    QuadraticDynamics::new(body, &[1000.; 2], velocities).unwrap()
}
#[test]
fn dynamic_crack_splits_without_losing_mass_momentum_or_kinetic_inventory() {
    let mut d = dynamic();
    let n = d.velocities().len();
    let initial = d.energy().unwrap();
    assert!(initial.momentum_kg_m_s[0].abs() > 1.);
    assert!(initial.angular_momentum_kg_m2_s[2].abs() > 1.);
    let report = d
        .advance_loaded_with_fracture_work(
            0.04,
            &vec![[0.; 3]; n],
            [0.; 3],
            physics::plasticity::mesh::QuadraticAdvanceLimits {
                minimum_dt_s: 1e-8,
                maximum_dt_s: 1e-4,
                max_attempts: 20000,
                energy_tolerance_j: 0.01,
            },
            0.05,
        )
        .unwrap();
    assert!(
        report
            .substeps
            .iter()
            .any(|s| s.fragment_count_before == 1 && s.fragment_count_after == 2)
    );
    let broken: Vec<_> = report
        .substeps
        .iter()
        .flat_map(|s| s.newly_broken_interfaces.iter().copied())
        .collect();
    assert_eq!(broken, vec![0]);
    assert_eq!(report.fragments.len(), 2);
    assert!(report.absolute_energy_defect_j <= 0.01 && report.interface_absolute_error_j <= 0.05);
    let defects: f64 = report.substeps.iter().map(|s| s.energy_defect_j).sum();
    let fracture: f64 = report.substeps.iter().map(|s| s.fracture_work_j).sum();
    let e = d.energy().unwrap();
    assert!((fracture - 5.).abs() < 1e-10);
    assert!((e.mass_kg - initial.mass_kg).abs() < 1e-10);
    for axis in 0..3 {
        assert!((e.momentum_kg_m_s[axis] - initial.momentum_kg_m_s[axis]).abs() < 1e-7);
        assert!(
            (e.angular_momentum_kg_m2_s[axis] - initial.angular_momentum_kg_m2_s[axis]).abs()
                < 1e-7
        );
    }
    let actual = e.kinetic_j
        + e.elastic_j
        + e.hardening_j
        + e.dissipated_j
        + e.cohesive_stored_j
        + e.fracture_dissipated_j
        - initial.kinetic_j;
    assert!((actual - defects).abs() < 1e-8);
    assert!((e.fracture_dissipated_j - 5.).abs() < 1e-10);
}
#[test]
fn late_work_budget_failure_restores_dynamic_pose_velocity_and_histories() {
    let mut d = dynamic();
    let before = d.clone();
    let n = d.velocities().len();
    assert!(
        d.step_loaded_with_fracture_work(1e-4, &vec![[0.; 3]; n], [0.; 3], 1e-4, 0.)
            .is_err()
    );
    assert_eq!(d.body().positions(), before.body().positions());
    assert_eq!(d.velocities(), before.velocities());
    assert_eq!(d.body().states(), before.body().states());
    assert_eq!(
        d.body().cohesive_interfaces()[0].states(),
        before.body().cohesive_interfaces()[0].states()
    );
}

#[test]
fn adaptive_interval_failure_after_valid_prefix_preserves_original_state() {
    let mut d = dynamic();
    let before = d.clone();
    let n = d.velocities().len();
    let loads = vec![[0.; 3]; n];
    let mut prefix = d.clone();
    prefix
        .step_loaded_with_fracture_work(6.25e-6, &loads, [0.; 3], 0.000125, 0.0125)
        .unwrap();
    assert_ne!(prefix.body().positions(), before.body().positions());
    let failure = d
        .advance_loaded_with_fracture_work(
            5e-5,
            &loads,
            [0.; 3],
            physics::plasticity::mesh::QuadraticAdvanceLimits {
                minimum_dt_s: 1e-9,
                maximum_dt_s: 1e-5,
                max_attempts: 1,
                energy_tolerance_j: 0.001,
            },
            0.1,
        )
        .unwrap_err();
    assert_eq!(failure, "adaptive fracture attempt limit reached");
    assert_eq!(d.body().positions(), before.body().positions());
    assert_eq!(d.velocities(), before.velocities());
    assert_eq!(d.body().states(), before.body().states());
    assert_eq!(
        d.body().cohesive_interfaces()[0].states(),
        before.body().cohesive_interfaces()[0].states()
    );
}
