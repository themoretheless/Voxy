use physics::plasticity::{
    Material,
    mesh::{QuadraticBody, QuadraticPlaneContact, QuadraticPlaneCoulomb},
};
fn mesh() -> QuadraticBody {
    QuadraticBody::from_linear(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![([0, 1, 2, 3], Material::new(1e5, 0.3, 1e9, 0.).unwrap())],
    )
    .unwrap()
}
#[test]
fn ample_coulomb_capacity_sticks_without_clamping_nodes_independently() {
    let mesh = mesh();
    let plane = QuadraticPlaneContact::new([0., 1., 0.], 2., 1000.).unwrap();
    let report = mesh
        .coulomb_plane_impulse_at(
            mesh.positions(),
            &[[0.1, 0., 0.]; 10],
            &[1000.],
            &[false; 10],
            plane,
            QuadraticPlaneCoulomb::new(0.5, 1e-9, 5000, 100).unwrap(),
            1.,
        )
        .unwrap();
    println!(
        "stick: iterations={}, residual={:e}, defect={:e}",
        report.iterations, report.residual_m_s, report.energy_defect_j
    );
    assert!(report.velocities.iter().flatten().all(|v| v.abs() < 1e-7));
    assert!(report.residual_m_s <= 1e-9);
    assert!(report.endpoint_dissipated_j < 1e-7 && report.numerical_dissipated_j > 0.);
    assert!(report.energy_defect_j.abs() < 1e-7);
    assert!(
        report
            .support_impulse_n_s
            .iter()
            .flatten()
            .all(|v| v.abs() < 1e-8)
    );
}
#[test]
fn sliding_saturates_coulomb_impulse_and_obeys_mass_and_energy_balances() {
    let mesh = mesh();
    let plane = QuadraticPlaneContact::new([0., 1., 0.], 2., 1000.).unwrap();
    let report = mesh
        .coulomb_plane_impulse_at(
            mesh.positions(),
            &[[1., 0., 0.]; 10],
            &[1000.],
            &[false; 10],
            plane,
            QuadraticPlaneCoulomb::new(0.1, 1e-10, 100, 100).unwrap(),
            0.001,
        )
        .unwrap();
    let expected = -0.1
        * mesh
            .plane_contact_at(mesh.positions(), plane)
            .unwrap()
            .force_n[1]
        * 0.001;
    let impulse: f64 = report.nodal_impulse_n_s.iter().map(|v| v[0]).sum();
    assert!((impulse - expected).abs() < 1e-10);
    let mass = mesh.consistent_mass(&[1000.]).unwrap();
    let momentum_change: f64 = mass
        .iter()
        .enumerate()
        .map(|(i, row)| row.iter().sum::<f64>() * (report.velocities[i][0] - 1.))
        .sum();
    assert!((momentum_change - impulse).abs() < 1e-10);
    assert!(report.endpoint_dissipated_j > 0. && report.numerical_dissipated_j > 0.);
    assert!(report.energy_defect_j.abs() < 1e-10);
    assert!(report.velocities.iter().all(|v| v[1] == 0. && v[2] == 0.));
}
#[test]
fn supported_impulses_include_mass_coupling_and_limits_are_explicit() {
    let mesh = mesh();
    let plane = QuadraticPlaneContact::new([0., 1., 0.], 2., 1000.).unwrap();
    let law = QuadraticPlaneCoulomb::new(0.5, 1e-8, 5000, 100).unwrap();
    let mut velocities = [[0.1, 0., 0.]; 10];
    velocities[0] = [0.; 3];
    let mut pins = [false; 10];
    pins[0] = true;
    let report = mesh
        .coulomb_plane_impulse_at(
            mesh.positions(),
            &velocities,
            &[1000.],
            &pins,
            plane,
            law,
            1.,
        )
        .unwrap();
    assert_eq!(report.velocities[0], [0.; 3]);
    assert!(
        report.support_impulse_n_s[1..]
            .iter()
            .flatten()
            .all(|v| v.abs() < 1e-8)
    );
    assert!(report.support_impulse_n_s[0].iter().any(|v| v.abs() > 1e-6));
    assert_eq!(
        mesh.coulomb_plane_impulse_at(
            mesh.positions(),
            &velocities,
            &[1000.],
            &pins,
            plane,
            QuadraticPlaneCoulomb::new(0.5, 1e-8, 5000, 1).unwrap(),
            1.
        )
        .unwrap_err(),
        "quadratic Coulomb sample limit reached"
    );
    assert_eq!(
        mesh.coulomb_plane_impulse_at(
            mesh.positions(),
            &velocities,
            &[1000.],
            &pins,
            plane,
            QuadraticPlaneCoulomb::new(0.5, 1e-14, 1, 100).unwrap(),
            1.
        )
        .unwrap_err(),
        "quadratic Coulomb iteration limit reached"
    );
    assert!(
        mesh.coulomb_plane_impulse_at(
            mesh.positions(),
            &[[0.1; 3]; 10],
            &[1000.],
            &pins,
            plane,
            law,
            1.
        )
        .is_err()
    );
}

#[test]
fn coulomb_solution_preserves_normal_velocity_and_frame_covariance() {
    let mesh = mesh();
    let plane = QuadraticPlaneContact::new([0., 1., 0.], 2., 1000.).unwrap();
    let law = QuadraticPlaneCoulomb::new(0.5, 1e-9, 5000, 100).unwrap();
    let velocities = [[0.1, 2., 0.07]; 10];
    let original = mesh
        .coulomb_plane_impulse_at(
            mesh.positions(),
            &velocities,
            &[1000.],
            &[false; 10],
            plane,
            law,
            0.01,
        )
        .unwrap();
    assert!(original.velocities.iter().all(|v| v[1] == 2.));
    let positions: Vec<_> = mesh
        .positions()
        .iter()
        .map(|p| [2. - p[1], 3. + p[0], p[2] - 1.])
        .collect();
    let velocities: Vec<_> = velocities.iter().map(|v| [-v[1], v[0], v[2]]).collect();
    let moved = mesh
        .coulomb_plane_impulse_at(
            &positions,
            &velocities,
            &[1000.],
            &[false; 10],
            QuadraticPlaneContact::new([-1., 0., 0.], 0., 1000.).unwrap(),
            law,
            0.01,
        )
        .unwrap();
    for (a, b) in original.velocities.iter().zip(&moved.velocities) {
        assert!(
            (b[0] + a[1]).abs() < 1e-8 && (b[1] - a[0]).abs() < 1e-8 && (b[2] - a[2]).abs() < 1e-8
        );
    }
    for (a, b) in original
        .nodal_impulse_n_s
        .iter()
        .zip(&moved.nodal_impulse_n_s)
    {
        assert!(
            (b[0] + a[1]).abs() < 1e-7 && (b[1] - a[0]).abs() < 1e-7 && (b[2] - a[2]).abs() < 1e-7
        );
    }
    assert!((moved.endpoint_dissipated_j - original.endpoint_dissipated_j).abs() < 1e-8);
    assert!((moved.numerical_dissipated_j - original.numerical_dissipated_j).abs() < 1e-8);
}
#[test]
fn absent_contact_or_zero_coefficient_preserve_velocity() {
    let mesh = mesh();
    for (offset, coefficient) in [(-1., 0.5), (2., 0.)] {
        let report = mesh
            .coulomb_plane_impulse_at(
                mesh.positions(),
                &[[0.1, 0., 0.]; 10],
                &[1000.],
                &[false; 10],
                QuadraticPlaneContact::new([0., 1., 0.], offset, 1000.).unwrap(),
                QuadraticPlaneCoulomb::new(coefficient, 1e-9, 100, 100).unwrap(),
                1.,
            )
            .unwrap();
        assert_eq!(report.velocities, vec![[0.1, 0., 0.]; 10]);
        assert_eq!(report.nodal_impulse_n_s, vec![[0.; 3]; 10]);
        assert_eq!(
            report.endpoint_dissipated_j + report.numerical_dissipated_j,
            0.
        );
        assert_eq!(report.iterations, 0);
    }
}
