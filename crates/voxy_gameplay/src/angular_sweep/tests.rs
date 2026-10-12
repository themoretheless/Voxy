use super::*;
fn body() -> [DVec3; 3] {
    [DVec3::X * 0.4, DVec3::Y * 0.1, DVec3::Z * 0.02]
}
fn wall() -> AffineBox {
    AffineBox {
        center: DVec3::Z * 0.25,
        edges: [DVec3::X * 2., DVec3::Y * 2., DVec3::Z * 0.02],
    }
}
#[test]
fn middle_of_half_and_full_turn_hits_even_when_endpoints_are_clear() {
    let edges = body();
    let obstacle = wall();
    let angle = (0.23 / 0.4_f64.hypot(0.02)).asin() - 0.02_f64.atan2(0.4);
    for turn in [
        std::f64::consts::PI,
        std::f64::consts::TAU,
        4. * std::f64::consts::TAU,
    ] {
        let final_edges = edges.map(|edge| DQuat::from_rotation_y(turn) * edge);
        assert!(obstacle.penetration_affine(DVec3::ZERO, edges).is_none());
        assert!(
            obstacle
                .penetration_affine(DVec3::ZERO, final_edges)
                .is_none()
        );
        let hit = sweep(DVec3::ZERO, edges, DVec3::Y * turn, &[obstacle], 256).unwrap();
        assert!((hit.fraction * turn - angle).abs() < 1e-7, "{hit:?}");
        assert!(hit.fraction * turn <= angle);
        assert!(
            obstacle
                .penetration_affine(
                    DVec3::ZERO,
                    edges.map(|edge| DQuat::from_rotation_y(turn * hit.fraction) * edge)
                )
                .is_none()
        );
    }
}
#[test]
fn grounded_yaw_and_turning_away_are_certified_over_the_whole_arc() {
    let floor = AffineBox {
        center: DVec3::Y * -0.1,
        edges: [DVec3::X * 2., DVec3::Y * 0.1, DVec3::Z * 2.],
    };
    assert_eq!(
        sweep(
            DVec3::Y * 0.1,
            body(),
            DVec3::Y * 4. * std::f64::consts::TAU,
            &[floor],
            1
        )
        .unwrap()
        .fraction,
        1.
    );
    let obstacle = wall();
    let angle = (0.23 / 0.4_f64.hypot(0.02)).asin() - 0.02_f64.atan2(0.4);
    let edges = body().map(|edge| DQuat::from_rotation_y(angle) * edge);
    assert_eq!(
        sweep(DVec3::ZERO, edges, -DVec3::Y * angle, &[obstacle], 1)
            .unwrap()
            .fraction,
        1.
    );
}
#[test]
fn projection_extrema_bound_independent_quaternion_samples() {
    for index in 0..20 {
        let axis = DVec3::new(0.3 + f64::from(index) * 0.01, 0.7, -0.2).normalize();
        let normal = DVec3::new(-0.4, 0.1 + f64::from(index) * 0.03, 0.8).normalize();
        let angle = 0.1 + f64::from(index) * 0.7;
        let edges = [DVec3::new(0.4, 0.1, -0.03), DVec3::Y * 0.1, DVec3::Z * 0.02];
        let upper = support_max(edges, axis, normal, angle);
        let mut observed = 0_f64;
        for step in 0..=2000 {
            let rotation = DQuat::from_axis_angle(axis, angle * f64::from(step) / 2000.);
            let support = edges
                .iter()
                .map(|edge| (rotation * *edge).dot(normal).abs())
                .sum::<f64>();
            assert!(support <= upper + 1e-14);
            observed = observed.max(support);
        }
        assert!(upper - observed < 1e-5);
    }
}
#[test]
fn budget_failure_is_explicit_and_tall_yaw_uses_perpendicular_radius() {
    assert!(matches!(
        sweep(
            DVec3::ZERO,
            body(),
            DVec3::Y * std::f64::consts::PI,
            &[wall()],
            1
        ),
        Err(PhysicsError::SweepBudget)
    ));
    let mut edges = body();
    edges[1] = DVec3::Y * 1000.;
    let hit = sweep(
        DVec3::ZERO,
        edges,
        DVec3::Y * std::f64::consts::PI,
        &[wall()],
        256,
    )
    .unwrap();
    assert!((0.1..0.3).contains(&hit.fraction));
}
#[test]
fn clearance_stops_before_nominal_contact_and_checks_stationary_envelopes() {
    let edges = [DVec3::X * 0.1, DVec3::Y * 0.1, DVec3::Z * 0.1];
    let obstacle = AffineBox {
        center: DVec3::X * 2.,
        edges,
    };
    let candidates = [&obstacle];
    let run = |clearance, speed: f64, center: DVec3| {
        advance_with_clearance(
            &candidates,
            |t| Ok((center + DVec3::X * speed * t, edges)),
            speed,
            0.3,
            0.3,
            clearance,
            &mut 256,
            &mut 4096,
        )
    };
    let nominal = run(0., 3., DVec3::ZERO).unwrap();
    let inflated = run(0.2, 3., DVec3::ZERO).unwrap();
    assert!((nominal.fraction - 0.6).abs() < 1e-8);
    assert!((inflated.fraction - (1.6 / 3.)).abs() < 1e-8);
    assert!(inflated.fraction < nominal.fraction);
    // A truly displaced body can reach the wall here, while the nominal
    // body still has a 0.2 gap. The envelope must not advance beyond it.
    let front = 3. * inflated.fraction + 0.1 + 0.2;
    assert!(front <= 1.9 + 1e-8);
    assert!(front >= 1.9 - 1e-8);
    assert_eq!(run(0.2, 0., DVec3::X * 1.7).unwrap().fraction, 0.);
    assert_eq!(run(0.2, 0., DVec3::ZERO).unwrap().fraction, 1.);
    assert!(run(-0.1, 3., DVec3::ZERO).is_err());
    assert!(run(f64::NAN, 3., DVec3::ZERO).is_err());
}

#[test]
fn rigid_envelope_broadphase_retains_contacts_outside_the_nominal_sweep() {
    use voxy_animation::{RootRigidPath, RootRigidTwist};
    let edges = [DVec3::X * 0.1, DVec3::Y * 0.1, DVec3::Z * 0.1];
    let wall = AffineBox {
        center: DVec3::X,
        edges: [DVec3::X * 0.1, DVec3::Y * 2., DVec3::Z * 2.],
    };
    let path = RootRigidPath::from_twists(
        &[(
            RootRigidTwist {
                linear: DVec3::X * 0.7,
                angular: DVec3::ZERO,
            },
            1.,
        )],
        1,
    )
    .unwrap();
    let run = |clearance, queries: &mut usize| {
        sweep_rigid_path_with_clearance(
            DVec3::ZERO,
            edges,
            DQuat::IDENTITY,
            &path,
            DQuat::IDENTITY,
            DVec3::ZERO,
            1.,
            clearance,
            &[wall],
            256,
            queries,
        )
    };
    assert!(run(0., &mut 4096).unwrap().complete);
    let hit = run(0.2, &mut 4096).unwrap();
    assert!(!hit.complete);
    assert!((hit.path_fraction - 0.6 / 0.7).abs() < 1e-8);
    let far = AffineBox {
        center: DVec3::X * 10.,
        ..wall
    };
    let far_hit = sweep_rigid_path_with_clearance(
        DVec3::ZERO,
        edges,
        DQuat::IDENTITY,
        &path,
        DQuat::IDENTITY,
        DVec3::ZERO,
        1.,
        0.2,
        &[far],
        0,
        &mut 4096,
    )
    .unwrap();
    assert!(far_hit.complete);
    // The near wall must survive broadphase even when no advancement work
    // is available: budget failure, rather than a falsely complete path.
    assert!(matches!(
        sweep_rigid_path_with_clearance(
            DVec3::ZERO,
            edges,
            DQuat::IDENTITY,
            &path,
            DQuat::IDENTITY,
            DVec3::ZERO,
            1.,
            0.2,
            &[wall],
            0,
            &mut 4096
        ),
        Err(PhysicsError::SweepBudget)
    ));
    assert!((hit.displacement.x - 0.6).abs() < 1e-8);
    assert_eq!(hit.completed_spans, 0);
    assert!(matches!(run(0.2, &mut 1), Err(PhysicsError::SweepBudget)));
    assert!(run(f64::NAN, &mut 4096).is_err());
    let stationary = RootRigidPath::from_twists(
        &[(
            RootRigidTwist {
                linear: DVec3::ZERO,
                angular: DVec3::ZERO,
            },
            1.,
        )],
        1,
    )
    .unwrap();
    let hit = sweep_rigid_path_with_clearance(
        DVec3::X * 0.7,
        edges,
        DQuat::IDENTITY,
        &stationary,
        DQuat::IDENTITY,
        DVec3::ZERO,
        1.,
        0.2,
        &[wall],
        256,
        &mut 4096,
    )
    .unwrap();
    assert!(!hit.complete);
    assert_eq!(hit.path_fraction, 0.);
}

#[test]
fn whole_body_clearance_accounts_for_pivot_signed_scale_and_evaluation_error() {
    use voxy_animation::{RootRigidApproximation, RootRigidPath, RootRigidTwist};
    let edges = [DVec3::X * 0.4, DVec3::Y * 0.1, DVec3::Z * 0.02];
    let origin = DVec3::new(0.6, -0.1, 0.2);
    let basis = DQuat::from_rotation_z(0.4);
    let approximation = RootRigidApproximation {
        path: RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::ZERO,
                    angular: DVec3::ZERO,
                },
                1.,
            )],
            1,
        )
        .unwrap(),
        origin_error_bound: 0.01,
        angular_error_bound: 0.2,
    };
    let physical_radius = corners(edges)
        .into_iter()
        .map(|v| (v - origin).length())
        .fold(0_f64, f64::max);
    for scale in [-2., 0.5, 2.] {
        let clearance =
            rigid_body_clearance(&approximation, edges, basis, origin, scale, 0.003).unwrap();
        let expected = scale.abs() * 0.01 + 2. * (0.1_f64).sin() * physical_radius + 0.003;
        assert!((clearance - expected).abs() < 1e-12);
        for corner in corners(edges) {
            let source = basis.conjugate() * (corner - origin) / scale;
            let actual =
                scale * (basis * (DQuat::from_rotation_y(0.2) * source + DVec3::Z * 0.01)) + origin;
            assert!((actual - corner).length() <= clearance);
        }
    }
    assert!(rigid_body_clearance(&approximation, edges, basis, origin, 0., 0.).is_err());
    assert!(rigid_body_clearance(&approximation, edges, basis, origin, 1., f64::NAN).is_err());
    let moving = RootRigidApproximation {
        path: RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::X * 0.7,
                    angular: DVec3::ZERO,
                },
                1.,
            )],
            1,
        )
        .unwrap(),
        origin_error_bound: 0.15,
        angular_error_bound: 0.,
    };
    let body = [DVec3::X * 0.1, DVec3::Y * 0.1, DVec3::Z * 0.1];
    let wall = AffineBox {
        center: DVec3::X,
        edges: [DVec3::X * 0.1, DVec3::Y * 2., DVec3::Z * 2.],
    };
    let hit = sweep_rigid_approximation(
        DVec3::ZERO,
        body,
        DQuat::IDENTITY,
        &moving,
        DQuat::IDENTITY,
        DVec3::ZERO,
        1.,
        0.05,
        &[wall],
        256,
        &mut 4096,
    )
    .unwrap();
    assert!(!hit.complete);
    assert!((hit.path_fraction - 0.6 / 0.7).abs() < 1e-8);
    let empty = RootRigidApproximation {
        path: RootRigidPath::from_twists(&[], 0).unwrap(),
        origin_error_bound: 0.2,
        angular_error_bound: 0.,
    };
    let hit = sweep_rigid_approximation(
        DVec3::X * 0.7,
        body,
        DQuat::IDENTITY,
        &empty,
        DQuat::IDENTITY,
        DVec3::ZERO,
        1.,
        0.,
        &[wall],
        256,
        &mut 4096,
    )
    .unwrap();
    assert!(!hit.complete);
    assert_eq!(hit.path_fraction, 0.);
}

#[test]
fn exact_conditional_screw_motion_preserves_floor_contact_and_jump() {
    use voxy_animation::{RootRigidApproximation, RootRigidPath, RootRigidTwist};
    let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
    let floor = AffineBox {
        center: DVec3::Y * (-0.5),
        edges: [DVec3::X * 4., DVec3::Y * 0.5, DVec3::Z * 4.],
    };
    for vertical in [0., 0.1, -0.1] {
        let path = RootRigidPath::from_twists(
            &[(
                RootRigidTwist {
                    linear: DVec3::new(0.3, vertical, 0.1),
                    angular: DVec3::Y * 0.4,
                },
                0.5,
            )],
            1,
        )
        .unwrap();
        let approximation = RootRigidApproximation {
            path,
            origin_error_bound: 0.,
            angular_error_bound: 0.,
        };
        for scale in [-2., 1.] {
            let hit = sweep_rigid_approximation(
                DVec3::Y * 0.125,
                edges,
                DQuat::from_rotation_y(0.2),
                &approximation,
                DQuat::from_rotation_y(0.3),
                DVec3::new(0.6, 0.2, -0.3),
                scale,
                0.,
                &[floor],
                256,
                &mut 4096,
            )
            .unwrap();
            assert_eq!(hit.complete, vertical * scale >= 0.);
            if vertical * scale < 0. {
                assert_eq!(hit.path_fraction, 0.);
            }
        }
    }
    // A wall is not discarded by the floor certificate.
    let wall = AffineBox {
        center: DVec3::X,
        edges: [DVec3::X * 0.125, DVec3::Y * 2., DVec3::Z * 2.],
    };
    let path = RootRigidPath::from_twists(
        &[(
            RootRigidTwist {
                linear: DVec3::X * 1.2,
                angular: DVec3::ZERO,
            },
            1.,
        )],
        1,
    )
    .unwrap();
    let approximation = RootRigidApproximation {
        path,
        origin_error_bound: 0.,
        angular_error_bound: 0.,
    };
    let hit = sweep_rigid_approximation(
        DVec3::Y * 0.125,
        edges,
        DQuat::IDENTITY,
        &approximation,
        DQuat::IDENTITY,
        DVec3::ZERO,
        1.,
        0.,
        &[floor, wall],
        256,
        &mut 4096,
    )
    .unwrap();
    assert!(!hit.complete);
    assert!((hit.path_fraction - 0.75 / 1.2).abs() < 1e-8);
}

#[test]
fn supported_screw_contact_survives_source_and_actor_axis_permutations() {
    use voxy_animation::{RootRigidApproximation, RootRigidPath, RootRigidTwist};
    let edges = [DVec3::X * 0.125, DVec3::Y * 0.125, DVec3::Z * 0.125];
    let floor = AffineBox {
        center: DVec3::Y * (-0.5),
        edges: [DVec3::X * 4., DVec3::Y * 0.5, DVec3::Z * 4.],
    };
    let quarter = DQuat::from_xyzw(0., 0., 0.5, 0.5).normalize();
    for (actor, basis) in [
        (DQuat::from_rotation_y(0.2), quarter),
        (quarter, DQuat::from_rotation_x(0.3)),
    ] {
        for scale in [-2., 1.] {
            let path = RootRigidPath::from_twists(
                &[(
                    RootRigidTwist {
                        linear: DVec3::new(0., 0.3, 0.1),
                        angular: DVec3::X * 0.4,
                    },
                    0.5,
                )],
                1,
            )
            .unwrap();
            let approximation = RootRigidApproximation {
                path,
                origin_error_bound: 0.,
                angular_error_bound: 0.,
            };
            let hit = sweep_rigid_approximation(
                DVec3::Y * 0.125,
                edges,
                actor,
                &approximation,
                basis,
                DVec3::new(0.6, 0.2, -0.3),
                scale,
                0.,
                &[floor],
                256,
                &mut 4096,
            )
            .unwrap();
            assert!(hit.complete);
        }
    }
}
