use physics::plasticity::{Material, mesh::QuadraticBody};
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
fn kinetic(mass: &[Vec<f64>], velocities: &[[f64; 3]]) -> f64 {
    mass.iter()
        .enumerate()
        .map(|(i, row)| {
            row.iter()
                .enumerate()
                .map(|(j, &m)| {
                    0.5 * m
                        * (0..3)
                            .map(|axis| velocities[i][axis] * velocities[j][axis])
                            .sum::<f64>()
                })
                .sum::<f64>()
        })
        .sum()
}
#[test]
fn corner_impact_matches_analytic_effective_mass_restitution_and_consistent_energy() {
    let body = fixture();
    let mass = body.consistent_mass(&[1000., 2000.]).unwrap();
    let inertia = body.consistent_inertia(&[1000., 2000.]).unwrap();
    let velocity: Vec<_> = (0..20)
        .map(|i| {
            if i < 4 || (8..14).contains(&i) {
                [0.3, 0.2, -1.]
            } else {
                [-0.1, 0.4, 0.5]
            }
        })
        .collect();
    let mut weights = vec![0.; 20];
    weights[0] = 1.;
    weights[4] = -1.;
    // For the exact T10 matrix, the inverse corner diagonal is 100/(rho*V).
    let effective = 1. / (100. / (1000. / 6.) + 100. / (2000. / 6.));
    for e in [0., 0.5, 1.] {
        let r = inertia
            .normal_impact(&velocity, &weights, [0., 0., 1.], e)
            .unwrap();
        assert!((r.effective_mass_kg - effective).abs() < 1e-12);
        assert!((r.impulse_n_s - (1. + e) * 1.5 * effective).abs() < 1e-12);
        assert!((r.relative_velocity_after_m_s - e * 1.5).abs() < 1e-12);
        let loss = 0.5 * (1. - e * e) * 1.5 * 1.5 * effective;
        assert!((r.dissipated_j - loss).abs() < 1e-12);
        assert!((kinetic(&mass, &velocity) - kinetic(&mass, &r.velocities) - loss).abs() < 1e-9);
        assert!(r.energy_defect_j.abs() < 1e-12);
        for (i, row) in mass.iter().enumerate() {
            for axis in 0..3 {
                let impulse: f64 = row
                    .iter()
                    .enumerate()
                    .map(|(j, m)| m * (r.velocities[j][axis] - velocity[j][axis]))
                    .sum();
                assert!((impulse - r.nodal_impulse_n_s[i][axis]).abs() < 1e-10);
            }
        }
        for axis in 0..3 {
            let momentum: f64 = mass
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    row.iter().sum::<f64>() * (r.velocities[i][axis] - velocity[i][axis])
                })
                .sum();
            assert!(momentum.abs() < 1e-9);
            let a = (axis + 1) % 3;
            let b = (axis + 2) % 3;
            let angular: f64 = mass
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    row.iter()
                        .enumerate()
                        .map(|(j, m)| {
                            m * (body.positions()[i][a] * (r.velocities[j][b] - velocity[j][b])
                                - body.positions()[i][b] * (r.velocities[j][a] - velocity[j][a]))
                        })
                        .sum::<f64>()
                })
                .sum();
            assert!(angular.abs() < 1e-9);
        }
        assert!((r.velocities[1][2] - velocity[1][2]).abs() > 1e-6);
    }
}
#[test]
fn separating_velocity_is_unchanged_and_invalid_constraints_are_rejected() {
    let body = fixture();
    let inertia = body.consistent_inertia(&[1000., 2000.]).unwrap();
    let mut velocity = vec![[0.; 3]; 20];
    velocity[0][2] = 1.;
    let mut weights = vec![0.; 20];
    weights[0] = 1.;
    weights[4] = -1.;
    let r = inertia
        .normal_impact(&velocity, &weights, [0., 0., 1.], 0.5)
        .unwrap();
    assert_eq!(r.velocities, velocity);
    assert_eq!(r.impulse_n_s, 0.);
    assert_eq!(r.dissipated_j, 0.);
    assert!(
        inertia
            .normal_impact(&velocity, &vec![0.; 20], [0., 0., 1.], 0.)
            .is_err()
    );
    assert!(
        inertia
            .normal_impact(&velocity, &weights, [0., 0., 2.], 0.)
            .is_err()
    );
    assert!(
        inertia
            .normal_impact(&velocity, &weights, [0., 0., 1.], 1.01)
            .is_err()
    );
    assert!(
        inertia
            .normal_impact(&velocity, &[], [0., 0., 1.], 0.)
            .is_err()
    );
}

#[test]
fn coulomb_impact_slides_or_sticks_without_reversing_and_closes_energy() {
    let body = fixture();
    let inertia = body.consistent_inertia(&[1000., 2000.]).unwrap();
    let mass = body.consistent_mass(&[1000., 2000.]).unwrap();
    let mut velocity = vec![[0.; 3]; 20];
    velocity[0] = [2., 1., -1.];
    let mut weights = vec![0.; 20];
    weights[0] = 1.;
    weights[4] = -1.;
    for mu in [0., 0.2, 10.] {
        let r = inertia
            .frictional_impact(&velocity, &weights, [0., 0., 1.], 0.5, mu)
            .unwrap();
        let tx = r.velocities[0][0] - r.velocities[4][0];
        let ty = r.velocities[0][1] - r.velocities[4][1];
        let slip = tx.hypot(ty);
        assert!(slip <= 5_f64.sqrt() + 1e-12);
        assert!(tx >= -1e-12 && ty >= -1e-12);
        let jt = r.nodal_impulse_n_s[0][0].hypot(r.nodal_impulse_n_s[0][1]);
        let expected = (r.effective_mass_kg * 5_f64.sqrt()).min(mu * r.impulse_n_s);
        assert!((jt - expected).abs() < 1e-12);
        if mu == 10. {
            assert!(slip < 1e-12);
        }
        assert!(
            (kinetic(&mass, &velocity) - kinetic(&mass, &r.velocities) - r.dissipated_j).abs()
                < 1e-10
        );
        assert!(r.energy_defect_j.abs() < 1e-12);
    }
    assert!(
        inertia
            .frictional_impact(&velocity, &weights, [0., 0., 1.], 0.5, -1.)
            .is_err()
    );
}

#[test]
fn coupled_inelastic_projection_activates_separating_contact_and_closes_energy() {
    use physics::plasticity::mesh::QuadraticImpactConstraint;
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
    let inertia = body.consistent_inertia(&[1000.; 3]).unwrap();
    let mass = body.consistent_mass(&[1000.; 3]).unwrap();
    let n = body.positions().len();
    let mut v = vec![[0.; 3]; n];
    for (i, nodes) in body.fragment_nodes().iter().enumerate() {
        for &node in nodes {
            v[node][2] = [-4., 0., -1.][i];
        }
    }
    let constraint = |a, b| {
        let mut w = vec![0.; n];
        w[a] = 1.;
        w[b] = -1.;
        QuadraticImpactConstraint {
            weights: w,
            normal: [0., 0., 1.],
        }
    };
    let cs = [constraint(0, 4), constraint(4, 8)];
    assert!(inertia.inelastic_impacts(&v, &cs, 1e-10, 1).is_err());
    let r = inertia.inelastic_impacts(&v, &cs, 1e-10, 1000).unwrap();
    assert!(r.relative_before_m_s[1] > 0. && r.impulses_n_s[1] > 0.);
    let effective = (1000. / 6.) / 100.;
    assert!((r.impulses_n_s[0] - (7. / 3.) * effective).abs() < 1e-9);
    assert!((r.impulses_n_s[1] - (2. / 3.) * effective).abs() < 1e-9);
    assert!(r.relative_after_m_s.iter().all(|g| g.abs() < 1e-10));
    assert!((kinetic(&mass, &v) - kinetic(&mass, &r.velocities) - r.dissipated_j).abs() < 1e-8);
    let bounce = inertia
        .restitution_impacts(&v, &cs, 0.5, 1e-10, 1000)
        .unwrap();
    assert!((bounce.relative_after_m_s[0] - 2.).abs() < 1e-10);
    assert!(bounce.relative_after_m_s[1].abs() < 1e-10);
    assert!(
        (kinetic(&mass, &v) - kinetic(&mass, &bounce.velocities) - bounce.dissipated_j).abs()
            < 1e-8
    );
    // These coupled Newton e=1 targets inject energy via the initially separating contact.
    assert!(
        inertia
            .restitution_impacts(&v, &cs, 1., 1e-10, 1000)
            .is_err()
    );
    let reversed = [cs[1].clone(), cs[0].clone()];
    let other = inertia
        .inelastic_impacts(&v, &reversed, 1e-10, 1000)
        .unwrap();
    for (a, b) in r.velocities.iter().zip(&other.velocities) {
        for k in 0..3 {
            assert!((a[k] - b[k]).abs() < 1e-9);
        }
    }
    for axis in 0..3 {
        let change: f64 = mass
            .iter()
            .enumerate()
            .map(|(i, row)| row.iter().sum::<f64>() * (r.velocities[i][axis] - v[i][axis]))
            .sum();
        assert!(change.abs() < 1e-9);
    }
}

#[test]
fn coupled_single_constraint_restitution_matches_analytic_normal_kernel() {
    use physics::plasticity::mesh::QuadraticImpactConstraint;
    let body = fixture();
    let inertia = body.consistent_inertia(&[1000., 2000.]).unwrap();
    let mut v = vec![[0.; 3]; body.positions().len()];
    v[0][2] = -1.;
    let mut weights = vec![0.; v.len()];
    weights[0] = 1.;
    weights[4] = -1.;
    let c = QuadraticImpactConstraint {
        weights: weights.clone(),
        normal: [0., 0., 1.],
    };
    for e in [0., 0.5, 1.] {
        let ordinary = inertia.normal_impact(&v, &weights, c.normal, e).unwrap();
        let joint = inertia
            .restitution_impacts(&v, std::slice::from_ref(&c), e, 1e-10, 100)
            .unwrap();
        assert!((joint.impulses_n_s[0] - ordinary.impulse_n_s).abs() < 1e-12);
        assert!((joint.relative_after_m_s[0] - e).abs() < 1e-10);
        assert!((joint.dissipated_j - ordinary.dissipated_j).abs() < 1e-12);
    }
    assert!(
        inertia
            .restitution_impacts(&v, &[c], 1.1, 1e-10, 100)
            .is_err()
    );
}

#[test]
fn elastic_coupled_impact_under_oblique_common_motion_reports_roundoff_separately() {
    use physics::plasticity::mesh::QuadraticImpactConstraint;
    let body = fixture();
    let inertia = body.consistent_inertia(&[1000., 2000.]).unwrap();
    let mass = body.consistent_mass(&[1000., 2000.]).unwrap();
    let n = body.positions().len();
    let groups = body.fragment_nodes();
    for angle in [0.13_f64, 0.47, 0.83, 1.17, 2.31] {
        let normal = [angle.cos(), angle.sin(), 0.];
        let mut v = vec![[123.4, -57.8, 31.2]; n];
        for &node in &groups[0] {
            for a in 0..3 {
                v[node][a] -= 0.7 * normal[a];
            }
        }
        let mut weights = vec![0.; n];
        weights[0] = 1.;
        weights[4] = -1.;
        let c = QuadraticImpactConstraint { weights, normal };
        let r = inertia
            .restitution_impacts(&v, &[c], 1., 1e-9, 100)
            .unwrap();
        assert!((r.relative_after_m_s[0] + r.relative_before_m_s[0]).abs() < 1e-9);
        assert!(r.energy_defect_j >= 0. && r.energy_defect_j <= r.energy_roundoff_bound_j);
        assert!(
            (kinetic(&mass, &r.velocities) - kinetic(&mass, &v) + r.dissipated_j
                - r.energy_defect_j)
                .abs()
                < 1e-6
        );
        assert!(r.dissipated_j < 1e-9);
    }
}
