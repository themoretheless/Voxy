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

fn limits() -> physics::plasticity::mesh::QuadraticSweepLimits {
    use physics::plasticity::mesh::{QuadraticClosestLimits, QuadraticSweepLimits};
    QuadraticSweepLimits {
        closest: QuadraticClosestLimits {
            distance_tolerance_m: 1e-7,
            max_patches: 4096,
            max_depth: 16,
        },
        minimum_time_fraction: 1e-5,
        max_intervals: 4096,
    }
}
#[test]
fn automatic_sweep_finds_crossing_and_bounds_work_without_mutation() {
    use physics::plasticity::mesh::QuadraticSweep;
    let reference = fixture();
    let mut corners = reference.positions()[..8].to_vec();
    for p in &mut corners[..4] {
        p[2] += 2.;
    }
    let m = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
    let body =
        QuadraticBody::from_linear(corners, vec![([0, 1, 2, 3], m), ([4, 5, 6, 7], m)]).unwrap();
    let groups = body.fragment_nodes();
    let start = body.positions().to_vec();
    let mut end = start.clone();
    for &n in &groups[0] {
        end[n][2] -= 4.;
    }
    let found = body.fragment_sweeps_at(&end, 0.01, limits(), 1000).unwrap();
    assert!(
        found
            .iter()
            .any(|r| matches!(r.sweep, QuadraticSweep::WithinClearance { .. }))
    );
    assert!(
        found
            .iter()
            .all(|r| r.source_component != r.target_component)
    );
    assert!(body.fragment_sweeps_at(&end, 0.01, limits(), 1).is_err());
    let first = body
        .first_fragment_clearance_at(&end, 0.01, limits(), 1000, 1e-4, 128)
        .unwrap()
        .unwrap();
    let entry = (2. - 0.01) / 4.;
    assert!(
        first.time_interval[0] <= entry + 1e-7 && first.time_interval[1] >= entry - 1e-7,
        "{first:?}"
    );
    assert!(first.earliest_witness_index.is_some());
    // Tiny search budgets must not turn an unresolved crossing into separation.
    let mut scarce = limits();
    scarce.max_intervals = 1;
    let unknown = body
        .first_fragment_clearance_at(&end, 0.01, scarce, 1000, 1e-4, 1)
        .unwrap()
        .unwrap();
    assert!(!unknown.converged);
    assert!(
        unknown
            .candidates
            .iter()
            .any(|c| c.clearance.witness.is_none())
    );
    assert_eq!(body.positions(), start);
    assert!(
        body.fragment_sweeps_at(&start, 0.01, limits(), 1000)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn invalid_limits_are_rejected_even_without_interfragment_queries() {
    let m = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
    let body = QuadraticBody::from_linear(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![([0, 1, 2, 3], m)],
    )
    .unwrap();
    let mut invalid = limits();
    invalid.closest.max_patches = 0;
    assert!(
        body.fragment_sweeps_at(body.positions(), 0.01, invalid, 1000)
            .is_err()
    );
    assert!(
        body.fragment_sweeps_at(body.positions(), 0.01, limits(), 1000)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn live_verlet_stops_before_crossing_and_failed_attempt_rolls_back() {
    use physics::plasticity::mesh::QuadraticDynamics;
    let reference = fixture();
    let mut corners = reference.positions()[..8].to_vec();
    for p in &mut corners[..4] {
        p[2] += 2.;
    }
    let m = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
    let body =
        QuadraticBody::from_linear(corners, vec![([0, 1, 2, 3], m), ([4, 5, 6, 7], m)]).unwrap();
    let mut v = vec![[0.; 3]; body.positions().len()];
    for &n in &body.fragment_nodes()[0] {
        v[n][2] = -4.;
    }
    let start = body.positions().to_vec();
    let loads = vec![[0.; 3]; v.len()];
    let mut d = QuadraticDynamics::new(body, &[1000.; 2], v).unwrap();
    let mut interval = d.clone();
    let report = interval
        .advance_loaded_until_fragment_clearance(
            1.,
            1e-4,
            &loads,
            [0.; 3],
            1e-6,
            0.01,
            limits(),
            1000,
            30,
            64,
        )
        .unwrap();
    assert!(report.stop_reason.is_some());
    assert!(report.advanced_s > 0. && report.advanced_s < 0.4975);
    assert!((report.advanced_s + report.remaining_s - 1.).abs() < 1e-12);
    let before = d.fragments().unwrap();
    let after = interval.fragments().unwrap();
    assert!(
        (after[0].center_m[2] - (before[0].center_m[2] - 4. * report.advanced_s)).abs() < 1e-10
    );
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(a.mass_kg, b.mass_kg);
        for axis in 0..3 {
            assert!((a.momentum_kg_m_s[axis] - b.momentum_kg_m_s[axis]).abs() < 1e-8);
        }
    }
    assert!(report.absolute_energy_defect_j <= 1e-6);
    let oldv = d.velocities().to_vec();
    assert!(
        d.step_loaded_before_fragment_clearance(
            1.,
            0.001,
            &loads,
            [0.; 3],
            1e-6,
            0.01,
            limits(),
            1000,
            1
        )
        .is_err()
    );
    assert_eq!(d.body().positions(), start);
    assert_eq!(d.velocities(), oldv);
    let r = d
        .step_loaded_before_fragment_clearance(
            1.,
            0.001,
            &loads,
            [0.; 3],
            1e-6,
            0.01,
            limits(),
            1000,
            20,
        )
        .unwrap();
    assert!(r.shortened);
    assert_eq!(r.accepted_dt_s, 0.25);
    assert!(r.energy_defect_j.abs() < 1e-6);
    assert!((d.body().positions()[0][2] - 1.).abs() < 1e-12);
}

#[test]
fn guarded_step_refines_energy_rejection_even_without_fragment_contact() {
    use physics::plasticity::mesh::QuadraticDynamics;
    let m = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
    let body = QuadraticBody::from_linear(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![([0, 1, 2, 3], m)],
    )
    .unwrap();
    let mut v = vec![[0.; 3]; body.positions().len()];
    v[1][0] = 1.;
    let loads = vec![[0.; 3]; v.len()];
    let mut d = QuadraticDynamics::new(body, &[1000.], v).unwrap();
    let mut trial = d.clone();
    assert!(trial.step_loaded(0.01, &loads, [0.; 3], 1e-8).is_err());
    let r = d
        .step_loaded_before_fragment_clearance(
            0.01,
            1e-10,
            &loads,
            [0.; 3],
            1e-8,
            0.01,
            limits(),
            1000,
            30,
        )
        .unwrap();
    assert!(r.shortened && r.accepted_dt_s < 0.01);
    assert!(r.energy_defect_j.abs() <= 1e-8 * r.accepted_dt_s / 0.01);
}

#[test]
fn separated_interval_completes_without_a_stop_reason() {
    use physics::plasticity::mesh::QuadraticDynamics;
    let m = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
    let body = QuadraticBody::from_linear(
        vec![[0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        vec![([0, 1, 2, 3], m)],
    )
    .unwrap();
    let n = body.positions().len();
    let mut d = QuadraticDynamics::new(body, &[1000.], vec![[0.; 3]; n]).unwrap();
    let r = d
        .advance_loaded_until_fragment_clearance(
            1.,
            1e-4,
            &vec![[0.; 3]; n],
            [0.; 3],
            1e-6,
            0.01,
            limits(),
            1000,
            30,
            64,
        )
        .unwrap();
    assert_eq!(r.advanced_s, 1.);
    assert_eq!(r.remaining_s, 0.);
    assert!(r.stop_reason.is_none());
    assert_eq!(r.steps.len(), 1);
}

#[test]
fn automatic_proximity_impact_discovers_pair_and_preserves_momentum_and_energy() {
    use physics::plasticity::mesh::QuadraticDynamics;
    let reference = fixture();
    let mut corners = reference.positions()[..8].to_vec();
    for p in &mut corners[..4] {
        p[2] += 0.01;
    }
    let m = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
    let body =
        QuadraticBody::from_linear(corners, vec![([0, 1, 2, 3], m), ([4, 5, 6, 7], m)]).unwrap();
    let mut v = vec![[0.; 3]; body.positions().len()];
    for &node in &body.fragment_nodes()[0] {
        v[node] = [0.2, -0.1, -1.];
    }
    let mut d = QuadraticDynamics::new(body, &[1000.; 2], v).unwrap();
    let before = d.energy().unwrap();
    let mut rebound = d.clone();
    assert!(
        rebound
            .impact_fragment_nodes_with_restitution(
                0.02,
                f64::NAN,
                limits().closest,
                1000,
                1e-10,
                10000
            )
            .is_err()
    );
    assert_eq!(rebound.velocities(), d.velocities());
    let bounce = rebound
        .impact_fragment_nodes_with_restitution(0.02, 0.5, limits().closest, 1000, 1e-10, 10000)
        .unwrap()
        .unwrap();
    assert!(bounce.contacts.len() > 1);
    for (before_g, after_g) in bounce
        .impact
        .relative_before_m_s
        .iter()
        .zip(&bounce.impact.relative_after_m_s)
    {
        let target = -0.5 * before_g.min(0.);
        assert!(*after_g >= target - 1e-10);
    }
    let bounce_energy = rebound.energy().unwrap();
    assert!(
        (bounce_energy.kinetic_j + bounce_energy.impact_dissipated_j - before.kinetic_j).abs()
            < 1e-8
    );
    for axis in 0..3 {
        assert!((bounce_energy.momentum_kg_m_s[axis] - before.momentum_kg_m_s[axis]).abs() < 1e-8);
        assert!(
            (bounce_energy.angular_momentum_kg_m2_s[axis] - before.angular_momentum_kg_m2_s[axis])
                .abs()
                < 1e-8
        );
    }
    let mut joint = d.clone();
    let result = joint
        .impact_fragment_nodes(0.02, limits().closest, 1000, 1e-8, 10000)
        .unwrap()
        .unwrap();
    assert!(result.contacts.len() > 1);
    let mut keys = std::collections::BTreeSet::new();
    assert!(
        result
            .contacts
            .iter()
            .all(|c| keys.insert((c.source_node, c.target_component)))
    );
    assert!(result.impact.relative_after_m_s.iter().all(|g| *g >= -1e-8));
    let joint_energy = joint.energy().unwrap();
    for a in 0..3 {
        assert!((joint_energy.momentum_kg_m_s[a] - before.momentum_kg_m_s[a]).abs() < 1e-8);
        assert!(
            (joint_energy.angular_momentum_kg_m2_s[a] - before.angular_momentum_kg_m2_s[a]).abs()
                < 1e-8
        );
    }
    assert!(
        (joint_energy.kinetic_j + joint_energy.impact_dissipated_j - before.kinetic_j).abs() < 1e-8
    );
    let original = d.velocities().to_vec();
    assert!(
        d.impact_nearest_fragment_node(0.02, 0.5, limits().closest, 1)
            .is_err()
    );
    assert_eq!(d.velocities(), original);
    assert!(
        d.impact_nearest_fragment_node(0.005, 0.5, limits().closest, 1000)
            .unwrap()
            .is_none()
    );
    assert_eq!(d.velocities(), original);
    let r = d
        .impact_nearest_fragment_node(0.02, 0.5, limits().closest, 1000)
        .unwrap()
        .unwrap();
    assert!((r.gap_m - 0.01).abs() < 1e-7);
    assert!(r.impact.impulse_n_s > 0. && r.impact.dissipated_j > 0.);
    let after = d.energy().unwrap();
    for a in 0..3 {
        assert!((before.momentum_kg_m_s[a] - after.momentum_kg_m_s[a]).abs() < 1e-9);
        assert!(
            (before.angular_momentum_kg_m2_s[a] - after.angular_momentum_kg_m2_s[a]).abs() < 1e-9
        );
    }
    assert!((after.kinetic_j + after.impact_dissipated_j - before.kinetic_j).abs() < 1e-9);
}

#[test]
fn finite_automatic_rebound_is_covariant_on_prestrained_rotated_translated_geometry() {
    use physics::{biomechanics::Material as Elastic, plasticity::mesh::FiniteQuadraticDynamics};
    let rotate = |p: [f64; 3]| {
        let (s, c) = 0.63_f64.sin_cos();
        let (u, v) = 0.41_f64.sin_cos();
        let x = c * p[0] - s * p[1];
        let y = s * p[0] + c * p[1];
        [v * x + u * p[2], y, -u * x + v * p[2]]
    };
    let make = |turned: bool| {
        let reference = fixture();
        let mut corners = reference.positions()[..8].to_vec();
        for p in &mut corners[..4] {
            p[2] += 0.01;
        }
        let m = Material::new(1e6, 0.3, 1e9, 0.).unwrap();
        let mut body =
            QuadraticBody::from_linear(corners, vec![([0, 1, 2, 3], m), ([4, 5, 6, 7], m)])
                .unwrap();
        let n = body.positions().len();
        let prescribed: Vec<_> = body
            .positions()
            .iter()
            .map(|&p| {
                let strained = [1.1 * p[0], 0.9 * p[1], 1.05 * p[2]];
                let x = if turned {
                    let r = rotate(strained);
                    std::array::from_fn(|a| r[a] + [8., -3., 4.][a])
                } else {
                    strained
                };
                std::array::from_fn(|a| Some(x[a] - p[a]))
            })
            .collect();
        assert!(
            body.equilibrate(&vec![[0.; 3]; n], &prescribed, 4, 1e-7)
                .unwrap()
                .converged
        );
        let mut velocities = vec![[0.; 3]; n];
        for &node in &body.fragment_nodes()[0] {
            velocities[node] = if turned {
                rotate([0.2, -0.1, -1.])
            } else {
                [0.2, -0.1, -1.]
            };
        }
        FiniteQuadraticDynamics::new(
            body,
            vec![Elastic::from_young_poisson(1e6, 0.3).unwrap(); 2],
            &[1000.; 2],
            velocities,
            &vec![false; n],
        )
        .unwrap()
    };
    let mut base = make(false);
    let mut turned = make(true);
    let mut search = limits().closest;
    search.distance_tolerance_m = 1e-10;
    search.max_patches = 65536;
    search.max_depth = 24;
    for d in [&mut base, &mut turned] {
        let before = d.energy().unwrap();
        assert!(before.elastic_j > 0.);
        let result = d
            .impact_fragment_nodes_with_restitution(0.02, 0.5, search, 1000, 1e-10, 10000)
            .unwrap()
            .unwrap();
        assert!(result.contacts.len() > 1);
        let after = d.energy().unwrap();
        assert_eq!(after.mass_kg, before.mass_kg);
        assert_eq!(after.elastic_j, before.elastic_j);
        assert!((after.kinetic_j + after.impact_dissipated_j - before.kinetic_j).abs() < 1e-8);
        for a in 0..3 {
            assert!((after.momentum_kg_m_s[a] - before.momentum_kg_m_s[a]).abs() < 1e-8);
            assert!(
                (after.angular_momentum_kg_m2_s[a] - before.angular_momentum_kg_m2_s[a]).abs()
                    < 1e-8
            );
        }
    }
    for (a, b) in base.velocities().iter().zip(turned.velocities()) {
        let r = rotate(*a);
        for k in 0..3 {
            assert!((r[k] - b[k]).abs() < 1e-7);
        }
    }
}
