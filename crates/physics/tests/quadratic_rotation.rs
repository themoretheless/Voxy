use physics::plasticity::{
    Material,
    mesh::{QuadraticBody, QuadraticDynamics},
};
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dynamic(deform: bool) -> QuadraticDynamics {
    let body = QuadraticBody::from_linear(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![([0, 1, 2, 3], Material::new(1e7, 0.3, 1e9, 0.).unwrap())],
    )
    .unwrap();
    let velocities = body
        .positions()
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let r = p.map(|v| v - 0.25);
            let spin = cross([0.3, -0.7, 1.1], r);
            std::array::from_fn(|a| {
                [1., -2., 0.5][a] + spin[a] + if deform && i == 4 && a == 0 { 0.4 } else { 0. }
            })
        })
        .collect();
    QuadraticDynamics::new(body, &[600.], velocities).unwrap()
}
#[test]
fn distributed_tetra_inertia_and_rigid_spin_match_analytic_integral() {
    let d = dynamic(false);
    let before = d.velocities().to_vec();
    let r = d.fragment_rotations().unwrap().remove(0);
    for a in 0..3 {
        assert!((r.angular_velocity_rad_s[a] - [0.3, -0.7, 1.1][a]).abs() < 1e-12);
        for b in 0..3 {
            let expected = 100. * if a == b { 3. / 40. } else { 1. / 80. };
            assert!((r.inertia_kg_m2[a][b] - expected).abs() < 1e-12);
        }
    }
    assert!(r.deformation_kinetic_j < 1e-25);
    assert!(r.energy_defect_j.abs() < 1e-10);
    assert_eq!(d.velocities(), before);
}
#[test]
fn nonrigid_motion_retains_its_energy_instead_of_discarding_it() {
    let d = dynamic(true);
    let r = d.fragment_rotations().unwrap().remove(0);
    assert!(r.deformation_kinetic_j > 0.01);
    assert!(
        (r.translation_kinetic_j + r.rotation_kinetic_j + r.deformation_kinetic_j
            - d.energy().unwrap().kinetic_j)
            .abs()
            < 1e-10
    );
    let orbital = cross(r.fragment.center_m, r.fragment.momentum_kg_m_s);
    for a in 0..3 {
        assert!(
            (orbital[a] + r.spin_kg_m2_s[a] - r.fragment.angular_momentum_kg_m2_s[a]).abs() < 1e-10
        );
    }
}

#[test]
fn finite_fragment_spin_is_covariant_and_independent_of_world_origin() {
    use physics::{biomechanics::Material as Elastic, plasticity::mesh::FiniteQuadraticDynamics};
    let q = |p: [f64; 3]| [-p[1], p[0], p[2]];
    let shift = [8., -3., 4.];
    let center: [f64; 3] = std::array::from_fn(|a| q([0.25; 3])[a] + shift[a]);
    let rest: Vec<_> = [[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
        .into_iter()
        .map(|p| std::array::from_fn(|a| q(p)[a] + shift[a]))
        .collect();
    let body = QuadraticBody::from_linear(
        rest,
        vec![([0, 1, 2, 3], Material::new(1e7, 0.3, 1e9, 0.).unwrap())],
    )
    .unwrap();
    let omega = q([0.3, -0.7, 1.1]);
    let translation = q([1., -2., 0.5]);
    let velocities = body
        .positions()
        .iter()
        .map(|p| {
            let r = std::array::from_fn(|a| p[a] - center[a]);
            let spin = cross(omega, r);
            std::array::from_fn(|a| translation[a] + spin[a])
        })
        .collect();
    let count = body.positions().len();
    let d = FiniteQuadraticDynamics::new(
        body,
        vec![Elastic::from_young_poisson(1e7, 0.3).unwrap()],
        &[600.],
        velocities,
        &vec![false; count],
    )
    .unwrap();
    let result = d.fragment_rotations().unwrap().remove(0);
    let base = dynamic(false).fragment_rotations().unwrap().remove(0);
    for a in 0..3 {
        assert!((result.angular_velocity_rad_s[a] - omega[a]).abs() < 1e-11);
        assert!((result.spin_kg_m2_s[a] - q(base.spin_kg_m2_s)[a]).abs() < 1e-10);
    }
    assert!((result.rotation_kinetic_j - base.rotation_kinetic_j).abs() < 1e-10);
    assert!(result.deformation_kinetic_j < 1e-20);
}
