use physics::plasticity::{
    Material,
    mesh::{QuadraticBody, QuadraticDynamics},
};
fn fixture() -> QuadraticBody {
    let material = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
    QuadraticBody::from_linear(
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
    .unwrap()
}

fn dynamic() -> QuadraticDynamics {
    let body = fixture();
    let count = body.positions().len();
    let groups = body.fragment_nodes();
    let mut velocities = vec![[0.; 3]; count];
    for &i in &groups[0] {
        velocities[i] = [0.02, -0.03, -0.5];
    }
    for &i in &groups[1] {
        velocities[i] = [0.02, -0.03, 0.5];
    }
    QuadraticDynamics::new(body, &[1000.; 2], velocities).unwrap()
}
#[test]
fn owned_impact_transfers_linear_and_angular_momentum_and_retains_restitution_loss() {
    for restitution in [0., 0.5, 1.] {
        let mut d = dynamic();
        let before = d.energy().unwrap();
        let n = d.velocities().len();
        let mut first = vec![0.; n];
        let mut second = vec![0.; n];
        first[1] = 1.;
        second[5] = 1.;
        let result = d
            .impact_fragments(&first, &second, [0., 0., 1.], restitution)
            .unwrap();
        assert!(result.impulse_n_s > 0.);
        assert!((result.relative_velocity_after_m_s - restitution).abs() < 1e-12);
        let after = d.energy().unwrap();
        for axis in 0..3 {
            assert!((after.momentum_kg_m_s[axis] - before.momentum_kg_m_s[axis]).abs() < 1e-10);
            assert!(
                (after.angular_momentum_kg_m2_s[axis] - before.angular_momentum_kg_m2_s[axis])
                    .abs()
                    < 1e-10
            );
        }
        assert!((after.kinetic_j + after.impact_dissipated_j - before.kinetic_j).abs() < 1e-10);
        assert_eq!(after.impact_dissipated_j, result.dissipated_j);
        let rotations = d.fragment_rotations().unwrap();
        assert!(rotations.iter().any(|r| r.rotation_kinetic_j > 0.));
    }
}
#[test]
fn invalid_point_pair_and_same_fragment_restore_velocity_and_loss_inventory() {
    let mut d = dynamic();
    let before = d.velocities().to_vec();
    let n = before.len();
    let mut first = vec![0.; n];
    let mut second = vec![0.; n];
    first[1] = 1.;
    second[4] = 1.;
    assert!(
        d.impact_fragments(&first, &second, [0., 0., 1.], 0.5)
            .is_err()
    );
    second[4] = 0.;
    second[0] = 1.;
    assert!(
        d.impact_fragments(&first, &second, [0., 0., 1.], 0.5)
            .is_err()
    );
    assert_eq!(d.velocities(), before);
    assert_eq!(d.energy().unwrap().impact_dissipated_j, 0.);
}

#[test]
fn finite_fragment_impact_preserves_momenta_in_rotated_translated_geometry() {
    use physics::{biomechanics::Material as Elastic, plasticity::mesh::FiniteQuadraticDynamics};
    let q = |p: [f64; 3]| [p[2], p[1], -p[0]];
    let offset = [8., -3., 4.];
    let reference = fixture();
    let corners = reference.positions()[..8]
        .iter()
        .map(|p| std::array::from_fn(|a| q(*p)[a] + offset[a]))
        .collect();
    let m = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
    let body =
        QuadraticBody::from_linear(corners, vec![([0, 1, 2, 3], m), ([4, 5, 6, 7], m)]).unwrap();
    let n = body.positions().len();
    let groups = body.fragment_nodes();
    let mut velocities = vec![[0.; 3]; n];
    for &i in &groups[0] {
        velocities[i] = q([0.02, -0.03, -0.5]);
    }
    for &i in &groups[1] {
        velocities[i] = q([0.02, -0.03, 0.5]);
    }
    let mut d = FiniteQuadraticDynamics::new(
        body,
        vec![Elastic::from_young_poisson(1e6, 0.3).unwrap(); 2],
        &[1000.; 2],
        velocities,
        &vec![false; n],
    )
    .unwrap();
    let before = d.energy().unwrap();
    let mut first = vec![0.; n];
    let mut second = vec![0.; n];
    first[1] = 1.;
    second[5] = 1.;
    let impact = d
        .impact_fragments(&first, &second, q([0., 0., 1.]), 0.5)
        .unwrap();
    let after = d.energy().unwrap();
    assert!((impact.relative_velocity_after_m_s - 0.5).abs() < 1e-12);
    assert!((after.kinetic_j + after.impact_dissipated_j - before.kinetic_j).abs() < 1e-10);
    for a in 0..3 {
        assert!((after.momentum_kg_m_s[a] - before.momentum_kg_m_s[a]).abs() < 1e-10);
        assert!(
            (after.angular_momentum_kg_m2_s[a] - before.angular_momentum_kg_m2_s[a]).abs() < 1e-10
        );
    }
}

#[test]
fn owned_frictional_impact_retains_loss_and_momenta_and_rejects_invalid_mu() {
    let body = fixture();
    let mut velocities = vec![[0.; 3]; body.positions().len()];
    for &i in &body.fragment_nodes()[0] {
        velocities[i] = [2., 1., -1.];
    }
    let mut d = QuadraticDynamics::new(body, &[1000.; 2], velocities).unwrap();
    let before = d.energy().unwrap();
    let mut first = vec![0.; d.velocities().len()];
    let mut second = first.clone();
    first[1] = 1.;
    second[5] = 1.;
    let original = d.velocities().to_vec();
    assert!(
        d.impact_fragments_with_friction(&first, &second, [0., 0., 1.], 0.5, f64::NAN)
            .is_err()
    );
    assert_eq!(d.velocities(), original);
    let r = d
        .impact_fragments_with_friction(&first, &second, [0., 0., 1.], 0.5, 0.4)
        .unwrap();
    let after = d.energy().unwrap();
    assert_eq!(after.impact_dissipated_j, r.dissipated_j);
    assert!((after.kinetic_j + after.impact_dissipated_j - before.kinetic_j).abs() < 1e-9);
    for a in 0..3 {
        assert!((after.momentum_kg_m_s[a] - before.momentum_kg_m_s[a]).abs() < 1e-9);
        assert!(
            (after.angular_momentum_kg_m2_s[a] - before.angular_momentum_kg_m2_s[a]).abs() < 1e-9
        );
    }
}

#[test]
fn owned_coupled_contacts_commit_once_and_rollback_failed_solve() {
    use physics::plasticity::mesh::QuadraticFragmentImpactConstraint;
    let m = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
    let mut points = Vec::new();
    for _ in 0..3 {
        points.extend([[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
    }
    let body = QuadraticBody::from_linear(
        points,
        vec![([0, 1, 2, 3], m), ([4, 5, 6, 7], m), ([8, 9, 10, 11], m)],
    )
    .unwrap();
    let n = body.positions().len();
    let mut v = vec![[0.; 3]; n];
    for (group, nodes) in body.fragment_nodes().iter().enumerate() {
        for &node in nodes {
            v[node] = [0.2, -0.3, [-4., 0., -1.][group]];
        }
    }
    let mut d = QuadraticDynamics::new(body, &[1000.; 3], v).unwrap();
    let contact = |a, b| {
        let mut first = vec![0.; n];
        let mut second = first.clone();
        first[a] = 1.;
        second[b] = 1.;
        QuadraticFragmentImpactConstraint {
            first,
            second,
            normal: [0., 0., 1.],
        }
    };
    let cs = [contact(1, 5), contact(5, 9)];
    let original = d.velocities().to_vec();
    let before = d.energy().unwrap();
    assert!(d.impact_fragment_contacts(&cs, 0.01, 1e-10, 1).is_err());
    assert_eq!(d.velocities(), original);
    assert_eq!(d.energy().unwrap().impact_dissipated_j, 0.);
    let invalid = [cs[0].clone(), contact(5, 4)];
    assert!(
        d.impact_fragment_contacts(&invalid, 0.01, 1e-10, 1000)
            .is_err()
    );
    assert_eq!(d.velocities(), original);
    let mut bounce = d.clone();
    assert!(
        bounce
            .impact_fragment_contacts_with_restitution(&cs, 0.01, 1., 1e-10, 1000)
            .is_err()
    );
    assert_eq!(bounce.velocities(), original);
    assert_eq!(bounce.energy().unwrap().impact_dissipated_j, 0.);
    let rebound = bounce
        .impact_fragment_contacts_with_restitution(&cs, 0.01, 0.5, 1e-10, 1000)
        .unwrap();
    let rebound_energy = bounce.energy().unwrap();
    assert!((rebound.relative_after_m_s[0] - 2.).abs() < 1e-10);
    assert!(
        (rebound_energy.kinetic_j + rebound_energy.impact_dissipated_j - before.kinetic_j).abs()
            < 1e-8
    );
    let r = d.impact_fragment_contacts(&cs, 0.01, 1e-10, 1000).unwrap();
    let after = d.energy().unwrap();
    assert_eq!(after.impact_dissipated_j, r.dissipated_j);
    assert!((after.kinetic_j + after.impact_dissipated_j - before.kinetic_j).abs() < 1e-8);
    for a in 0..3 {
        assert!((after.momentum_kg_m_s[a] - before.momentum_kg_m_s[a]).abs() < 1e-8);
        assert!(
            (after.angular_momentum_kg_m2_s[a] - before.angular_momentum_kg_m2_s[a]).abs() < 1e-8
        );
    }
    assert!(r.impulses_n_s.iter().all(|j| *j > 0.));
}
