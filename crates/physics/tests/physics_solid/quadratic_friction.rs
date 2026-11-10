use physics::plasticity::{
    Material,
    mesh::{QuadraticBody, QuadraticPlaneContact, QuadraticPlaneFriction},
};
fn mesh() -> QuadraticBody {
    QuadraticBody::from_linear(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![([0, 1, 2, 3], Material::new(1e5, 0.3, 1e9, 0.).unwrap())],
    )
    .unwrap()
}
#[test]
fn uniform_sliding_recovers_coulomb_resultant_and_power() {
    let mesh = mesh();
    let plane = QuadraticPlaneContact::new([0., 1., 0.], 2., 1000.).unwrap();
    let law = QuadraticPlaneFriction::new(0.6, 0.01).unwrap();
    let velocities = vec![[2., -7., -3.]; 10];
    let normal = mesh
        .plane_contact_at(mesh.positions(), plane)
        .unwrap()
        .force_n[1];
    let response = mesh
        .plane_friction_at(mesh.positions(), &velocities, plane, law)
        .unwrap();
    let denominator = (13_f64 + 0.0001).sqrt();
    for (axis, speed) in [(0, 2.), (2, -3.)] {
        assert!((response.resultant_n[axis] + 0.6 * normal * speed / denominator).abs() < 1e-9);
    }
    assert_eq!(response.resultant_n[1], 0.);
    assert!((response.power_w + 0.6 * normal * 13. / denominator).abs() < 1e-8);
    assert!(response.resultant_n[0].hypot(response.resultant_n[2]) <= 0.6 * normal);
}
#[test]
fn interpolated_velocity_power_matches_nodal_work_and_frame_covariance() {
    let mesh = mesh();
    let plane = QuadraticPlaneContact::new([0., 1., 0.], 0.3, 1000.).unwrap();
    let law = QuadraticPlaneFriction::new(0.5, 0.02).unwrap();
    let velocities: Vec<_> = mesh
        .positions()
        .iter()
        .map(|p| [p[0] - 0.2, p[1] + 0.1, p[2] - 0.7])
        .collect();
    let response = mesh
        .plane_friction_at(mesh.positions(), &velocities, plane, law)
        .unwrap();
    let nodal_power: f64 = response
        .forces_n
        .iter()
        .zip(&velocities)
        .map(|(f, v)| f.iter().zip(v).map(|(a, b)| a * b).sum::<f64>())
        .sum();
    assert!(response.power_w < 0. && (nodal_power - response.power_w).abs() < 1e-10);
    let positions: Vec<_> = mesh
        .positions()
        .iter()
        .map(|p| [2. - p[1], 3. + p[0], p[2] - 1.])
        .collect();
    let velocities: Vec<_> = velocities.iter().map(|v| [-v[1], v[0], v[2]]).collect();
    let moved = mesh
        .plane_friction_at(
            &positions,
            &velocities,
            QuadraticPlaneContact::new([-1., 0., 0.], -1.7, 1000.).unwrap(),
            law,
        )
        .unwrap();
    assert!((moved.power_w - response.power_w).abs() < 1e-9);
    for (a, b) in response.forces_n.iter().zip(moved.forces_n) {
        assert!(
            (b[0] + a[1]).abs() < 1e-9 && (b[1] - a[0]).abs() < 1e-9 && (b[2] - a[2]).abs() < 1e-9
        );
    }
}
#[test]
fn no_contact_no_slip_and_zero_coefficient_give_zero_force() {
    let mesh = mesh();
    let plane = QuadraticPlaneContact::new([0., 1., 0.], 2., 1000.).unwrap();
    let law = QuadraticPlaneFriction::new(0.5, 0.01).unwrap();
    for (plane, law, velocities) in [
        (plane, law, vec![[0., 7., 0.]; 10]),
        (
            plane,
            QuadraticPlaneFriction::new(0., 0.01).unwrap(),
            vec![[1.; 3]; 10],
        ),
        (
            QuadraticPlaneContact::new([0., 1., 0.], -1., 1000.).unwrap(),
            law,
            vec![[1.; 3]; 10],
        ),
    ] {
        let response = mesh
            .plane_friction_at(mesh.positions(), &velocities, plane, law)
            .unwrap();
        assert_eq!(response.forces_n, vec![[0.; 3]; 10]);
        assert_eq!(response.power_w, 0.);
    }
    assert!(QuadraticPlaneFriction::new(-0.1, 0.01).is_err());
    assert!(QuadraticPlaneFriction::new(0.5, 0.).is_err());
    assert!(
        mesh.plane_friction_at(mesh.positions(), &[[0.; 3]; 9], plane, law)
            .is_err()
    );
    assert!(
        mesh.plane_friction_at(mesh.positions(), &[[f64::NAN; 3]; 10], plane, law)
            .is_err()
    );
}
