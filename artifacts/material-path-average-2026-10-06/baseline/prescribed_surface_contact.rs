use physics::biomechanics::PrescribedTriangleSurface;
fn obstacle() -> PrescribedTriangleSurface {
    PrescribedTriangleSurface::new(
        vec![[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
        vec![[0, 1, 2]],
        0.001,
        0.03,
        100.,
    )
    .unwrap()
}
fn body() -> Vec<[f64; 3]> {
    vec![[0.1, 0.1, 0.015], [0.15, 0.1, 0.04], [0.1, 0.15, 0.04]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
#[test]
fn both_feature_gradients_match_independent_energy_differences() {
    let obstacle = obstacle();
    let body = body();
    let faces = [[0, 1, 2]];
    let response = obstacle.response(&body, &faces).unwrap();
    assert!(response.potential_j > 0.);
    let h = 1e-7;
    for node in 0..3 {
        for axis in 0..3 {
            let mut plus = body.clone();
            let mut minus = body.clone();
            plus[node][axis] += h;
            minus[node][axis] -= h;
            let fd = (obstacle.response(&plus, &faces).unwrap().potential_j
                - obstacle.response(&minus, &faces).unwrap().potential_j)
                / (2. * h);
            assert!(
                (fd - response.body_gradient_n[node][axis]).abs() < 1e-7,
                "body node={node} axis={axis}"
            );
            let mut plus = obstacle.positions().to_vec();
            let mut minus = plus.clone();
            plus[node][axis] += h;
            minus[node][axis] -= h;
            let fd = (obstacle
                .with_positions(plus)
                .unwrap()
                .response(&body, &faces)
                .unwrap()
                .potential_j
                - obstacle
                    .with_positions(minus)
                    .unwrap()
                    .response(&body, &faces)
                    .unwrap()
                    .potential_j)
                / (2. * h);
            assert!(
                (fd - response.obstacle_gradient_n[node][axis]).abs() < 1e-7,
                "obstacle node={node} axis={axis}"
            );
        }
    }
}
#[test]
fn feature_forces_and_torques_are_equal_and_opposite() {
    let obstacle = obstacle();
    let body = body();
    let response = obstacle.response(&body, &[[0, 1, 2]]).unwrap();
    let mut force = [0.; 3];
    let mut torque = [0.; 3];
    for (position, gradient) in body.iter().zip(&response.body_gradient_n).chain(
        obstacle
            .positions()
            .iter()
            .zip(&response.obstacle_gradient_n),
    ) {
        let moment = cross(*position, *gradient);
        for axis in 0..3 {
            force[axis] += gradient[axis];
            torque[axis] += moment[axis];
        }
    }
    assert!(force.iter().all(|v| v.abs() < 1e-12));
    assert!(torque.iter().all(|v| v.abs() < 1e-12));
}
#[test]
fn finite_triangle_has_no_infinite_plane_contact_outside_its_extent() {
    let obstacle = obstacle();
    let body: Vec<_> = body()
        .into_iter()
        .map(|p| [p[0] + 10., p[1], p[2]])
        .collect();
    let response = obstacle.response(&body, &[[0, 1, 2]]).unwrap();
    assert_eq!(response.potential_j, 0.);
    assert!(response.body_gradient_n.iter().flatten().all(|v| *v == 0.));
    assert!(
        response
            .obstacle_gradient_n
            .iter()
            .flatten()
            .all(|v| *v == 0.)
    );
}
#[test]
fn obstacle_work_is_independent_and_refines_towards_potential_change() {
    let obstacle = obstacle();
    let body = body();
    let faces = [[0, 1, 2]];
    let initial = obstacle.response(&body, &faces).unwrap();
    let run = |delta: f64| {
        let next = obstacle
            .with_positions(
                obstacle
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + delta])
                    .collect(),
            )
            .unwrap();
        let final_response = next.response(&body, &faces).unwrap();
        let work = obstacle
            .motion_work(
                &next,
                &initial.obstacle_gradient_n,
                &final_response.obstacle_gradient_n,
            )
            .unwrap();
        assert!(work > 0.);
        (final_response.potential_j - initial.potential_j - work).abs()
    };
    let coarse = run(0.0001);
    let fine = run(0.00005);
    println!("TRIANGLE SURFACE work defects coarse={coarse:e}, fine={fine:e}");
    assert!(fine < coarse * 0.3);
}
#[test]
fn endpoint_disjoint_surface_motion_cannot_tunnel_through_body() {
    let obstacle = obstacle();
    let body = body();
    let faces = [[0, 1, 2]];
    let next = obstacle
        .with_positions(
            obstacle
                .positions()
                .iter()
                .map(|p| [p[0], p[1], p[2] + 0.06])
                .collect(),
        )
        .unwrap();
    assert!(obstacle.response(&body, &faces).is_ok());
    assert!(next.response(&body, &faces).is_ok());
    assert!(!obstacle.path_is_open(&next, &body, &body, &faces).unwrap());
    let moved_obstacle = obstacle
        .with_positions(
            obstacle
                .positions()
                .iter()
                .map(|p| [p[0] + 0.001, p[1], p[2]])
                .collect(),
        )
        .unwrap();
    let moved_body: Vec<_> = body.iter().map(|p| [p[0] + 0.001, p[1], p[2]]).collect();
    assert!(
        obstacle
            .path_is_open(&moved_obstacle, &body, &moved_body, &faces)
            .unwrap()
    );
}
#[test]
fn malformed_geometry_and_changed_owner_reject_without_mutating_surface() {
    let obstacle = obstacle();
    let before = format!("{obstacle:?}");
    let mut invalid = obstacle.positions().to_vec();
    invalid[0][0] = f64::NAN;
    assert!(obstacle.with_positions(invalid).is_err());
    assert!(obstacle.with_positions(vec![[0.; 3]; 3]).is_err());
    assert!(obstacle.with_positions(vec![[0.; 3]; 2]).is_err());
    assert!(
        PrescribedTriangleSurface::new(
            obstacle.positions().to_vec(),
            vec![[0, 1, 2], [2, 1, 0]],
            0.001,
            0.03,
            100.
        )
        .is_err()
    );
    let same_geometry_new_owner = PrescribedTriangleSurface::new(
        obstacle.positions().to_vec(),
        vec![[0, 1, 2]],
        0.001,
        0.03,
        100.,
    )
    .unwrap();
    assert!(
        obstacle
            .path_is_open(&same_geometry_new_owner, &body(), &body(), &[[0, 1, 2]])
            .is_err()
    );
    assert!(
        obstacle
            .motion_work(&same_geometry_new_owner, &[[0.; 3]; 3], &[[0.; 3]; 3])
            .is_err()
    );
    assert!(obstacle.response(&body(), &[[0, 1, 9]]).is_err());
    assert_eq!(format!("{obstacle:?}"), before);
}

fn dynamics(pinned: bool, velocity_z: f64, maxwell: bool) -> physics::biomechanics::InertialBody {
    use physics::biomechanics::{
        Body, InertialBody, Material, MaxwellBranch, OgdenTerm, ViscoelasticOgden,
    };
    let mut body = Body::new(
        vec![
            [0.1, 0.1, 0.015],
            [0.2, 0.1, 0.02],
            [0.1, 0.2, 0.025],
            [0.1, 0.1, 0.115],
        ],
        vec![pinned; 4],
        vec![(
            [0, 1, 2, 3],
            Material::from_young_poisson(1e5, 0.3).unwrap(),
        )],
    )
    .unwrap();
    if maxwell {
        body.set_viscoelastic_ogden_batch(&[(
            0,
            ViscoelasticOgden::new(
                vec![OgdenTerm {
                    shear_pa: 5000.,
                    exponent: 2.,
                }],
                1e6,
                vec![MaxwellBranch {
                    shear_pa: 10000.,
                    relaxation_seconds: 0.2,
                }],
            )
            .unwrap(),
        )])
        .unwrap();
        InertialBody::new_viscoelastic_with_supports(body, &[1000.], vec![[0.; 3]; 4]).unwrap()
    } else if pinned {
        InertialBody::new_with_fixed_supports(body, &[1000.], vec![[0.; 3]; 4]).unwrap()
    } else {
        InertialBody::new(body, &[1000.], vec![[0., 0., velocity_z]; 4]).unwrap()
    }
}
#[test]
fn inertial_obstacle_motion_books_work_and_rejects_tunnelling_atomically() {
    use std::sync::Arc;
    let surface = Arc::new(obstacle());
    let mut body = dynamics(true, 0., false);
    assert!(body.set_prescribed_surface(Some(surface.clone())).unwrap() > 0.);
    let initial = body.diagnostics().unwrap();
    let next = Arc::new(
        surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 1e-6])
                    .collect(),
            )
            .unwrap(),
    );
    let receipt = body
        .step_with_surface_motion(None, next.clone(), 0.001, 1e-9)
        .unwrap();
    let after = body.diagnostics().unwrap();
    assert_eq!(receipt.support_work_j, 0.);
    assert_eq!(receipt.plane_work_j, 0.);
    assert!(receipt.surface_work_j > 0.);
    assert!(
        (after.potential_j
            - initial.potential_j
            - receipt.surface_work_j
            - receipt.energy_defect_j)
            .abs()
            < 1e-12
    );
    let before = format!("{body:?}");
    assert_eq!(
        body.step_with_surface_motion(None, Arc::new(obstacle()), 0.001, 1e-6)
            .unwrap_err(),
        "prescribed surface identity or contact law changed"
    );
    assert_eq!(format!("{body:?}"), before);
    let crossing = Arc::new(
        next.with_positions(
            next.positions()
                .iter()
                .map(|p| [p[0], p[1], p[2] + 0.2])
                .collect(),
        )
        .unwrap(),
    );
    assert_eq!(
        body.step_with_surface_motion(None, crossing, 0.001, 1e-6)
            .unwrap_err(),
        "inertial prescribed surface path crossing"
    );
    assert_eq!(format!("{body:?}"), before);
    let closed = Arc::new(
        surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], 0.015])
                    .collect(),
            )
            .unwrap(),
    );
    assert!(body.set_prescribed_surface(Some(closed)).is_err());
    assert_eq!(format!("{body:?}"), before);
}
#[test]
fn translating_triangle_obstacle_is_galilean_equivalent_in_actual_dynamics() {
    use std::sync::Arc;
    let static_surface = Arc::new(obstacle());
    let mut fixed = dynamics(false, -0.05, false);
    let speed = 0.2;
    let mut moving = dynamics(false, -0.05 + speed, false);
    fixed
        .set_prescribed_surface(Some(static_surface.clone()))
        .unwrap();
    moving
        .set_prescribed_surface(Some(static_surface.clone()))
        .unwrap();
    let initial = moving.diagnostics().unwrap();
    let mut work = 0.;
    let mut defect = 0.;
    let dt = 1e-5;
    for step in 1..=1000 {
        let time = f64::from(step) * dt;
        fixed.step(dt, 1e-6).unwrap();
        let before_p = moving.diagnostics().unwrap().momentum_kg_m_s[2];
        let next = Arc::new(
            static_surface
                .with_positions(
                    static_surface
                        .positions()
                        .iter()
                        .map(|p| [p[0], p[1], p[2] + speed * time])
                        .collect(),
                )
                .unwrap(),
        );
        let receipt = moving
            .step_with_surface_motion(None, next, dt, 1e-6)
            .unwrap();
        let after = moving.diagnostics().unwrap();
        assert!(
            (receipt.surface_work_j - speed * (after.momentum_kg_m_s[2] - before_p)).abs() < 1e-12
        );
        work += receipt.surface_work_j;
        defect += receipt.energy_defect_j;
        for (a, b) in fixed
            .body()
            .positions()
            .iter()
            .zip(moving.body().positions())
        {
            assert!((a[0] - b[0]).abs() < 1e-10 && (a[1] - b[1]).abs() < 1e-10);
            assert!((a[2] + speed * time - b[2]).abs() < 1e-10);
        }
        for (a, b) in fixed.velocities().iter().zip(moving.velocities()) {
            assert!((a[2] + speed - b[2]).abs() < 1e-8);
        }
    }
    let after = moving.diagnostics().unwrap();
    assert!(work > 0.);
    assert!(
        (after.kinetic_j + after.potential_j
            - initial.kinetic_j
            - initial.potential_j
            - work
            - defect)
            .abs()
            < 1e-10
    );
    println!("DYNAMIC TRIANGLE SURFACE actuator work={work:e}, defect={defect:e}");
}
#[test]
fn viscoelastic_surface_rejection_preserves_history_heat_and_obstacle_pose() {
    use physics::biomechanics::SupportTarget;
    use std::sync::Arc;
    let mut body = dynamics(true, 0., true);
    body.enable_maxwell_thermal(&[3500.], &[310.15]).unwrap();
    let sheared: Vec<_> = body
        .body()
        .positions()
        .iter()
        .enumerate()
        .map(|(node, p)| SupportTarget {
            node,
            position_m: [p[0] + 0.001 * p[1], p[1], p[2]],
        })
        .collect();
    body.step_viscoelastic(Some(&sheared), 0.001, 1e-6).unwrap();
    let surface = Arc::new(obstacle());
    body.set_prescribed_surface(Some(surface.clone())).unwrap();
    let held: Vec<_> = body
        .body()
        .positions()
        .iter()
        .enumerate()
        .map(|(node, p)| SupportTarget {
            node,
            position_m: *p,
        })
        .collect();
    let before = format!("{body:?}");
    let crossing = Arc::new(
        surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 0.2])
                    .collect(),
            )
            .unwrap(),
    );
    assert_eq!(
        body.step_viscoelastic_with_surface_motion(Some(&held), crossing, 0.001, 1e-6)
            .unwrap_err(),
        "inertial prescribed surface path crossing"
    );
    assert_eq!(format!("{body:?}"), before);
    let receipt = body
        .step_viscoelastic_with_surface_motion(Some(&held), surface, 0.001, 1e-6)
        .unwrap();
    assert_eq!(receipt.support.surface_work_j, 0.);
    assert!(receipt.viscous_heat_j > 0.);
}

#[test]
fn co_translating_rig_supports_and_triangle_surface_balance_both_actuators() {
    use physics::biomechanics::SupportTarget;
    use std::sync::Arc;
    let surface = Arc::new(obstacle());
    let mut body = dynamics(true, 0., false);
    body.set_prescribed_surface(Some(surface.clone())).unwrap();
    let before = body.diagnostics().unwrap();
    let delta = 0.001;
    let targets: Vec<_> = body
        .body()
        .positions()
        .iter()
        .enumerate()
        .map(|(node, p)| SupportTarget {
            node,
            position_m: [p[0], p[1], p[2] + delta],
        })
        .collect();
    let next = Arc::new(
        surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + delta])
                    .collect(),
            )
            .unwrap(),
    );
    let receipt = body
        .step_with_surface_motion(Some(&targets), next.clone(), 0.001, 1e-9)
        .unwrap();
    let after = body.diagnostics().unwrap();
    assert!((after.contact_j - before.contact_j).abs() < 1e-12);
    assert!(receipt.surface_work_j > 0.);
    assert!((receipt.reaction_work_j + receipt.surface_work_j).abs() < 1e-12);
    assert!(
        (receipt.support_work_j + receipt.surface_work_j - receipt.pin_kinetic_work_j).abs()
            < 1e-12
    );
    assert_eq!(
        body.prescribed_surface().unwrap().positions(),
        next.positions()
    );
}

#[test]
fn authored_contact_domain_preserves_source_indices_and_motion_identity() {
    let positions = vec![
        [-1., -1., 0.],
        [1., -1., 0.],
        [0., 1., 0.],
        [9., -1., 0.],
        [11., -1., 0.],
        [10., 1., 0.],
    ];
    let all = PrescribedTriangleSurface::new(
        positions.clone(),
        vec![[0, 1, 2], [3, 4, 5]],
        0.001,
        0.03,
        100.,
    )
    .unwrap();
    let excluded = all.with_contact_faces(vec![false, true]).unwrap();
    assert_eq!(excluded.positions(), all.positions());
    assert_eq!(excluded.faces(), all.faces());
    assert_eq!(all.contact_faces(), &[true, true]);
    assert_eq!(excluded.contact_faces(), &[false, true]);
    assert!(all.response(&body(), &[[0, 1, 2]]).unwrap().potential_j > 0.);
    let response = excluded.response(&body(), &[[0, 1, 2]]).unwrap();
    assert_eq!(response.potential_j, 0.);
    assert!(
        response
            .obstacle_gradient_n
            .iter()
            .flatten()
            .all(|&v| v == 0.)
    );
    let far_body: Vec<_> = body().iter().map(|p| [p[0] + 10., p[1], p[2]]).collect();
    assert!(
        excluded
            .response(&far_body, &[[0, 1, 2]])
            .unwrap()
            .potential_j
            > 0.
    );
    let moved = excluded
        .with_positions(positions.iter().map(|p| [p[0], p[1], 0.06]).collect())
        .unwrap();
    assert_eq!(moved.contact_faces(), excluded.contact_faces());
    assert!(
        excluded
            .path_is_open(&moved, &body(), &body(), &[[0, 1, 2]])
            .unwrap()
    );
    assert!(
        !excluded
            .path_is_open(&moved, &far_body, &far_body, &[[0, 1, 2]])
            .unwrap()
    );
    assert!(
        all.path_is_open(&excluded, &body(), &body(), &[[0, 1, 2]])
            .is_err()
    );
    assert!(
        excluded
            .motion_work(&all, &[[0.; 3]; 6], &[[0.; 3]; 6])
            .is_err()
    );
    assert!(all.with_contact_faces(vec![false, false]).is_err());
    assert!(all.with_contact_faces(vec![true]).is_err());
    // Exclusion cannot hide malformed geometry.
    let mut malformed = positions;
    malformed[0] = [f64::NAN; 3];
    assert!(excluded.with_positions(malformed).is_err());
}

#[test]
fn nearest_contact_reports_reconstructible_features_and_closed_gaps_read_only() {
    let surface = obstacle();
    let before = format!("{surface:?}");
    let points = body();
    let feature = surface
        .nearest_active_contact(&points, &[[0, 1, 2]])
        .unwrap()
        .unwrap();
    assert_eq!(feature.body_face, [0, 1, 2]);
    assert_eq!(feature.obstacle_face_index, 0);
    assert!((feature.distance_m - 0.015).abs() < 1e-12);
    assert!((feature.gap_m - 0.014).abs() < 1e-12);
    for weights in [feature.body_weights, feature.obstacle_weights] {
        assert!((weights.iter().sum::<f64>() - 1.).abs() < 1e-12);
        assert!(weights.iter().all(|&w| (0. ..=1.).contains(&w)));
    }
    let a: [f64; 3] = std::array::from_fn(|axis| {
        feature
            .body_face
            .iter()
            .zip(feature.body_weights)
            .map(|(&node, w)| points[node][axis] * w)
            .sum()
    });
    let b: [f64; 3] = std::array::from_fn(|axis| {
        surface.faces()[feature.obstacle_face_index]
            .iter()
            .zip(feature.obstacle_weights)
            .map(|(&node, w)| surface.positions()[node][axis] * w)
            .sum()
    });
    let distance = a
        .iter()
        .zip(b)
        .map(|(&a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!((distance - feature.distance_m).abs() < 1e-12);
    let closed: Vec<_> = points.iter().map(|p| [p[0], p[1], p[2] - 0.015]).collect();
    assert!(
        surface
            .nearest_active_contact(&closed, &[[0, 1, 2]])
            .unwrap()
            .unwrap()
            .gap_m
            < 0.
    );
    assert!(surface.response(&closed, &[[0, 1, 2]]).is_err());
    let far: Vec<_> = points.iter().map(|p| [p[0] + 10., p[1], p[2]]).collect();
    assert!(
        surface
            .nearest_active_contact(&far, &[[0, 1, 2]])
            .unwrap()
            .is_none()
    );
    assert!(
        surface
            .nearest_active_contact(&points, &[[0, 1, 9]])
            .is_err()
    );
    assert_eq!(format!("{surface:?}"), before);
}

#[test]
fn normal_contact_blocks_match_force_differences_and_preserve_rigid_nullspace() {
    let surface = obstacle();
    for gap in [0.014, 0.001, 11e-6] {
        let points: Vec<_> = body()
            .iter()
            .map(|p| [p[0], p[1], p[2] + 0.001 + gap - 0.015])
            .collect();
        let blocks = surface.normal_stencils(&points, &[[0, 1, 2]]).unwrap();
        assert_eq!(blocks.len(), 1);
        let block = &blocks[0];
        assert_eq!(block.obstacle_face_index, 0);
        assert!(block.normal_curvature_n_m > 0.);
        let h = gap * 1e-4;
        let plus: Vec<_> = points.iter().map(|p| [p[0], p[1], p[2] + h]).collect();
        let minus: Vec<_> = points.iter().map(|p| [p[0], p[1], p[2] - h]).collect();
        let force_derivative = (surface
            .response(&plus, &[[0, 1, 2]])
            .unwrap()
            .body_gradient_n
            .iter()
            .map(|g| g[2])
            .sum::<f64>()
            - surface
                .response(&minus, &[[0, 1, 2]])
                .unwrap()
                .body_gradient_n
                .iter()
                .map(|g| g[2])
                .sum::<f64>())
            / (2. * h);
        assert!((force_derivative / block.normal_curvature_n_m - 1.).abs() < 1e-6);
        let translation = [[0.1, -0.2, 0.3]; 3];
        let (a, b) = block.apply(translation, translation);
        assert!(a.iter().chain(&b).flatten().all(|v| v.abs() < 1e-6));
        let displacement = [[0.2, 0.3, 0.4], [-0.1, 0.5, 0.2], [0.1, -0.3, -0.4]];
        let obstacle_displacement = [[-0.2, 0.1, 0.3], [0.3, -0.1, -0.2], [-0.4, 0.2, 0.1]];
        let (a, b) = block.apply(displacement, obstacle_displacement);
        let energy = a
            .iter()
            .zip(displacement)
            .chain(b.iter().zip(obstacle_displacement))
            .map(|(g, d)| g.iter().zip(d).map(|(v, d)| v * d).sum::<f64>())
            .sum::<f64>();
        assert!(energy >= 0.);
        for axis in 0..3 {
            assert!(
                a.iter().chain(&b).map(|g| g[axis]).sum::<f64>().abs()
                    < block.normal_curvature_n_m * 1e-12
            );
        }
    }
    let closed: Vec<_> = body().iter().map(|p| [p[0], p[1], p[2] - 0.015]).collect();
    assert!(surface.normal_stencils(&closed, &[[0, 1, 2]]).is_err());
}

#[test]
fn implicit_midpoint_books_obstacle_work_and_keeps_rejection_atomic() {
    use std::sync::Arc;
    let surface = Arc::new(obstacle());
    let mut pinned = dynamics(true, 0., false);
    pinned
        .set_prescribed_surface(Some(surface.clone()))
        .unwrap();
    let targets: Vec<_> = pinned
        .body()
        .positions()
        .iter()
        .enumerate()
        .map(|(node, &position_m)| physics::biomechanics::SupportTarget { node, position_m })
        .collect();
    let next = Arc::new(
        surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 1e-6])
                    .collect(),
            )
            .unwrap(),
    );
    let before = pinned.diagnostics().unwrap();
    let receipt = pinned
        .step_implicit_with_surface_motion(Some(&targets), next.clone(), 1e-4, 1e-8)
        .unwrap();
    let after = pinned.diagnostics().unwrap();
    assert!(receipt.surface_work_j > 0.);
    assert!(
        (after.potential_j - before.potential_j - receipt.surface_work_j - receipt.energy_defect_j)
            .abs()
            < 1e-12
    );
    let snapshot = format!("{pinned:?}");
    let crossing = Arc::new(
        next.with_positions(next.positions().iter().map(|p| [p[0], p[1], 0.2]).collect())
            .unwrap(),
    );
    assert!(
        pinned
            .step_implicit_with_surface_motion(Some(&targets), crossing, 1e-4, 1e-8)
            .is_err()
    );
    assert_eq!(format!("{pinned:?}"), snapshot);
    assert!(
        pinned
            .step_implicit_with_surface_motion(Some(&targets[..3]), next, 1e-4, 1e-8)
            .is_err()
    );
    assert_eq!(format!("{pinned:?}"), snapshot);
}

#[test]
fn implicit_midpoint_free_motion_solves_constant_acceleration_and_contact_force() {
    use std::sync::Arc;
    let mut free = dynamics(false, 0., false);
    let far = Arc::new(
        obstacle()
            .with_positions(
                obstacle()
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], -10.])
                    .collect(),
            )
            .unwrap(),
    );
    free.set_prescribed_surface(Some(far.clone())).unwrap();
    free.set_uniform_acceleration([0., -9.81, 0.]).unwrap();
    let initial = free.body().positions().to_vec();
    let dt = 1e-4;
    free.step_implicit_with_surface_motion(None, far, dt, 1e-8)
        .unwrap();
    for (point, old) in free.body().positions().iter().zip(initial) {
        assert!((point[1] - old[1] + 0.5 * 9.81 * dt * dt).abs() < 1e-10);
    }
    let mut contact = dynamics(false, 0., false);
    let surface = Arc::new(obstacle());
    contact
        .set_prescribed_surface(Some(surface.clone()))
        .unwrap();
    let before = contact.diagnostics().unwrap();
    let receipt = contact
        .step_implicit_with_surface_motion(None, surface, 1e-5, 1e-7)
        .unwrap();
    assert!(contact.diagnostics().unwrap().momentum_kg_m_s[2] > before.momentum_kg_m_s[2]);
    assert_eq!(receipt.surface_work_j, 0.);
    assert!(receipt.energy_defect_j.abs() < 1e-7);
}

#[test]
fn implicit_moving_contact_is_galilean_equivalent_and_books_actuator_power() {
    use std::sync::Arc;
    let speed = 0.2;
    let dt = 1e-5;
    let surface = Arc::new(obstacle());
    let mut reference = dynamics(false, 0., false);
    let mut moving = dynamics(false, speed, false);
    reference
        .set_prescribed_surface(Some(surface.clone()))
        .unwrap();
    moving
        .set_prescribed_surface(Some(surface.clone()))
        .unwrap();
    for step in 1..=100 {
        reference
            .step_implicit_with_surface_motion(None, surface.clone(), dt, 1e-7)
            .unwrap();
        let next = Arc::new(
            surface
                .with_positions(
                    surface
                        .positions()
                        .iter()
                        .map(|p| [p[0], p[1], p[2] + speed * step as f64 * dt])
                        .collect(),
                )
                .unwrap(),
        );
        let momentum = moving.diagnostics().unwrap().momentum_kg_m_s[2];
        let receipt = moving
            .step_implicit_with_surface_motion(None, next, dt, 1e-7)
            .unwrap();
        let impulse = moving.diagnostics().unwrap().momentum_kg_m_s[2] - momentum;
        assert!((receipt.surface_work_j - speed * impulse).abs() < 1e-10);
        for (a, b) in reference
            .body()
            .positions()
            .iter()
            .zip(moving.body().positions())
        {
            assert!((b[0] - a[0]).abs() < 1e-9);
            assert!((b[1] - a[1]).abs() < 1e-9);
            assert!((b[2] - a[2] - speed * step as f64 * dt).abs() < 1e-9);
        }
    }
    let mut maxwell = dynamics(true, 0., true);
    maxwell
        .set_prescribed_surface(Some(surface.clone()))
        .unwrap();
    let snapshot = format!("{maxwell:?}");
    assert!(
        maxwell
            .step_implicit_with_surface_motion(None, surface, dt, 1e-7)
            .is_err()
    );
    assert_eq!(format!("{maxwell:?}"), snapshot);
}

#[test]
fn implicit_maxwell_split_preserves_heat_work_and_rolls_back_every_owner() {
    use physics::biomechanics::SupportTarget;
    use std::sync::Arc;
    let mut body = dynamics(true, 0., true);
    body.enable_maxwell_thermal(&[3500.], &[310.15]).unwrap();
    let shear: Vec<_> = body
        .body()
        .positions()
        .iter()
        .enumerate()
        .map(|(node, p)| SupportTarget {
            node,
            position_m: [p[0] + 0.001 * p[1], p[1], p[2]],
        })
        .collect();
    body.step_viscoelastic(Some(&shear), 0.001, 1e-6).unwrap();
    let surface = Arc::new(obstacle());
    body.set_prescribed_surface(Some(surface.clone())).unwrap();
    let held: Vec<_> = body
        .body()
        .positions()
        .iter()
        .enumerate()
        .map(|(node, &position_m)| SupportTarget { node, position_m })
        .collect();
    let snapshot = format!("{body:?}");
    let crossing = Arc::new(
        surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], 0.2])
                    .collect(),
            )
            .unwrap(),
    );
    assert!(
        body.step_viscoelastic_implicit_with_surface_motion(Some(&held), crossing, 0.001, 1e-6)
            .is_err()
    );
    assert_eq!(format!("{body:?}"), snapshot);
    let mut explicit = body.clone();
    let old = explicit
        .step_viscoelastic_with_surface_motion(Some(&held), surface.clone(), 0.001, 1e-6)
        .unwrap();
    let new = body
        .step_viscoelastic_implicit_with_surface_motion(Some(&held), surface, 0.001, 1e-6)
        .unwrap();
    assert!(new.viscous_heat_j > 0.);
    assert_eq!(new.support.surface_work_j, 0.);
    assert_eq!(new.viscous_heat_j, old.viscous_heat_j);
    assert_eq!(format!("{body:?}"), format!("{explicit:?}"));
    assert!(new.total_energy_defect_j.abs() < 1e-6);
}

#[test]
fn loose_energy_budget_cannot_bypass_implicit_force_equilibrium() {
    use std::sync::Arc;
    let surface = Arc::new(obstacle());
    let mut strict = dynamics(false, 0., false);
    strict
        .set_prescribed_surface(Some(surface.clone()))
        .unwrap();
    let mut diagnostic = strict.clone();
    strict
        .step_implicit_with_surface_motion(None, surface.clone(), 1e-5, 1e-8)
        .unwrap();
    diagnostic
        .step_implicit_with_surface_motion(None, surface, 1e-5, 1e100)
        .unwrap();
    assert!(diagnostic.diagnostics().unwrap().momentum_kg_m_s[2] > 0.);
    assert_eq!(strict.body().positions(), diagnostic.body().positions());
    assert_eq!(strict.velocities(), diagnostic.velocities());
}

#[test]
fn implicit_tiny_step_preserves_impulse_below_world_position_resolution() {
    use std::sync::Arc;
    let mut free = dynamics(false, 0., false);
    let surface = Arc::new(
        obstacle()
            .with_positions(
                obstacle()
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], -10.])
                    .collect(),
            )
            .unwrap(),
    );
    free.set_prescribed_surface(Some(surface.clone())).unwrap();
    free.set_uniform_acceleration([0., -9.81, 0.]).unwrap();
    let positions = free.body().positions().to_vec();
    let dt = 1e-10;
    // Require equilibrium refinement even though the drift cannot change
    // world coordinates; the local displacement still represents it.
    free.step_implicit_with_surface_motion(None, surface, dt, 1e-18)
        .unwrap();
    // The drift is below the representable world-coordinate increment, while
    // the impulse is representable and must not be reconstructed as zero.
    assert_eq!(free.body().positions(), positions);
    for velocity in free.velocities() {
        assert!((velocity[1] + 9.81 * dt).abs() < 1e-18);
    }
}

#[test]
fn path_averaged_force_is_gradient_of_search_objective_and_books_narrow_gap_work() {
    let surface = obstacle();
    let start = body();
    let end: Vec<_> = start.iter().map(|p| [p[0], p[1], p[2] + 0.001]).collect();
    let next = surface
        .with_positions(
            surface
                .positions()
                .iter()
                .map(|p| [p[0], p[1], p[2] + 0.002])
                .collect(),
        )
        .unwrap();
    let path = surface
        .path_response(&next, &start, &end, &[[0, 1, 2]])
        .unwrap();
    let h = 1e-7;
    for node in 0..3 {
        for axis in 0..3 {
            let mut plus = end.clone();
            let mut minus = end.clone();
            plus[node][axis] += h;
            minus[node][axis] -= h;
            let derivative = (surface
                .path_response(&next, &start, &plus, &[[0, 1, 2]])
                .unwrap()
                .midpoint_objective_j
                - surface
                    .path_response(&next, &start, &minus, &[[0, 1, 2]])
                    .unwrap()
                    .midpoint_objective_j)
                / (2. * h);
            // End positions change twice as fast as the midpoint unknown.
            assert!((2. * derivative - path.body_gradient_n[node][axis]).abs() < 1e-6);
        }
    }
    let near: Vec<_> = start
        .iter()
        .map(|p| [p[0], p[1], p[2] + 0.001 + 11e-6 - 0.015])
        .collect();
    let moved = surface
        .with_positions(
            surface
                .positions()
                .iter()
                .map(|p| [p[0], p[1], p[2] + 1e-7])
                .collect(),
        )
        .unwrap();
    let path = surface
        .path_response(&moved, &near, &near, &[[0, 1, 2]])
        .unwrap();
    let work = path
        .obstacle_gradient_n
        .iter()
        .map(|g| g[2] * 1e-7)
        .sum::<f64>();
    let delta = moved.response(&near, &[[0, 1, 2]]).unwrap().potential_j
        - surface.response(&near, &[[0, 1, 2]]).unwrap().potential_j;
    assert!((delta - work).abs() < 1e-10);
    let wrong = obstacle();
    assert!(
        surface
            .path_response(&wrong, &start, &end, &[[0, 1, 2]])
            .is_err()
    );
}

#[test]
fn path_quadrature_resolves_submicrometre_contact_work_without_widening_budget() {
    let surface = PrescribedTriangleSurface::new(
        vec![[-1., -1., 0.], [1., -1., 0.], [0., 1., 0.]],
        vec![[0, 1, 2]],
        0.0001,
        0.003,
        100.,
    )
    .unwrap();
    let points = [
        [0.1, 0.1, 0.000100435],
        [0.15, 0.1, 0.025],
        [0.1, 0.15, 0.025],
    ];
    let displacement = 0.000000138;
    let next = surface
        .with_positions(
            surface
                .positions()
                .iter()
                .map(|p| [p[0], p[1], p[2] + displacement])
                .collect(),
        )
        .unwrap();
    let path = surface
        .path_response(&next, &points, &points, &[[0, 1, 2]])
        .unwrap();
    let work = path
        .obstacle_gradient_n
        .iter()
        .map(|g| g[2] * displacement)
        .sum::<f64>();
    let change = next.response(&points, &[[0, 1, 2]]).unwrap().potential_j
        - surface.response(&points, &[[0, 1, 2]]).unwrap().potential_j;
    assert!(
        (change - work).abs() < 1e-11,
        "path work error {}",
        change - work
    );
    assert!(
        surface
            .path_is_open(&next, &points, &points, &[[0, 1, 2]])
            .unwrap()
    );
}

#[test]
fn subnanometre_contact_energy_detects_endpoint_reconstruction_roundoff() {
    let surface = PrescribedTriangleSurface::new(
        vec![[-1., -1., 1.], [1., -1., 1.], [0., 1., 1.]],
        vec![[0, 1, 2]],
        0.0001,
        0.003,
        100.,
    )
    .unwrap();
    let solved: [[f64; 3]; 3] = [
        [0.1, 0.1, 1.0001000000001],
        [0.15, 0.1, 1.025],
        [0.1, 0.15, 1.025],
    ];
    let mut reconstructed = solved;
    reconstructed[0][2] -= 2e-16;
    let faces = [[0, 1, 2]];
    let displacement = (reconstructed[0][2] - solved[0][2]).abs();
    assert!(displacement > 0. && displacement < 3e-16);
    let first = surface.response(&solved, &faces).unwrap();
    let second = surface.response(&reconstructed, &faces).unwrap();
    assert!(
        surface
            .path_is_open(&surface, &solved, &reconstructed, &faces)
            .unwrap()
    );
    // Both endpoints remain open; a world-coordinate ULP nevertheless changes
    // independently evaluated barrier energy far above the step work budget.
    assert!((second.potential_j - first.potential_j).abs() > 1e-7);
}

#[test]
fn implicit_narrow_gap_preserves_integrated_endpoint_and_energy_balance() {
    use std::sync::Arc;
    let mut free = dynamics(false, 0., false);
    let reference = obstacle();
    let surface = Arc::new(
        reference
            .with_positions(
                reference
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], 0.013999])
                    .collect(),
            )
            .unwrap(),
    );
    free.set_prescribed_surface(Some(surface.clone())).unwrap();
    free.set_uniform_acceleration([0., -9.81, 0.]).unwrap();
    let next = Arc::new(
        surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 0.2e-6])
                    .collect(),
            )
            .unwrap(),
    );
    let start = free.body().positions().to_vec();
    let old_velocity = free.velocities().to_vec();
    let before = free.diagnostics().unwrap();
    let dt = 1e-7;
    let tolerance = 1e-9;
    let receipt = free
        .step_implicit_with_surface_motion(None, next.clone(), dt, tolerance)
        .unwrap();
    let after = free.diagnostics().unwrap();
    assert!(
        (after.kinetic_j + after.potential_j
            - before.kinetic_j
            - before.potential_j
            - receipt.surface_work_j)
            .abs()
            <= tolerance
    );
    assert!(receipt.surface_work_j.abs() > 1e-6);
    assert!(
        surface
            .path_is_open(
                &next,
                &start,
                free.body().positions(),
                &free.body().surface()
            )
            .unwrap()
    );
    for node in 0..start.len() {
        for axis in 0..3 {
            let defect = free.body().positions()[node][axis]
                - start[node][axis]
                - 0.5 * dt * (old_velocity[node][axis] + free.velocities()[node][axis]);
            assert!(defect.abs() < 1e-11, "drift residual {defect}");
        }
    }
}

#[test]
fn translated_oblique_contact_preserves_energy_and_feature_forces() {
    let surface = PrescribedTriangleSurface::new(
        vec![[0., 0., 0.], [0.5, 0., 0.5], [0., 0.5, 0.5]],
        vec![[0, 1, 2]],
        1e-7,
        0.003,
        100.,
    )
    .unwrap();
    let gap = 2.0_f64.powi(-20);
    let body = [
        [0.125, 0.125, 0.25 + gap],
        [0.125, 0.125, 0.3],
        [0.14, 0.125, 0.4],
    ];
    let faces = [[0, 1, 2]];
    let reference = surface.response(&body, &faces).unwrap();
    for shift in [1048576., -1048576.] {
        let moved = surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| p.map(|v| v + shift))
                    .collect(),
            )
            .unwrap();
        let response = moved
            .response(&body.map(|p| p.map(|v| v + shift)), &faces)
            .unwrap();
        assert!((response.potential_j - reference.potential_j).abs() < 1e-11);
        for (actual, expected) in response
            .body_gradient_n
            .iter()
            .zip(&reference.body_gradient_n)
            .chain(
                response
                    .obstacle_gradient_n
                    .iter()
                    .zip(&reference.obstacle_gradient_n),
            )
        {
            for axis in 0..3 {
                assert!((actual[axis] - expected[axis]).abs() < 1e-5);
            }
        }
    }
}

#[test]
fn co_moving_body_has_feasible_predictor_when_stationary_guess_closes_gap() {
    use std::sync::Arc;
    let mut body = dynamics(false, 0.1, false);
    let surface = Arc::new(
        PrescribedTriangleSurface::new(
            vec![
                [-1., -1., 0.0139999],
                [1., -1., 0.0139999],
                [0., 1., 0.0139999],
            ],
            vec![[0, 1, 2]],
            0.001,
            0.003,
            1e-8,
        )
        .unwrap(),
    );
    body.set_prescribed_surface(Some(surface.clone())).unwrap();
    let next = Arc::new(
        surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 1e-6])
                    .collect(),
            )
            .unwrap(),
    );
    let start = body.body().positions().to_vec();
    let faces = body.body().surface();
    assert!(next.response(&start, &faces).is_err());
    let predicted: Vec<_> = start.iter().map(|p| [p[0], p[1], p[2] + 1e-6]).collect();
    assert!(next.response(&predicted, &faces).is_ok());
    let before = body.diagnostics().unwrap();
    let receipt = body
        .step_implicit_with_surface_motion(None, next.clone(), 1e-5, 1e-12)
        .unwrap();
    let after = body.diagnostics().unwrap();
    assert!(
        (after.kinetic_j + after.potential_j
            - before.kinetic_j
            - before.potential_j
            - receipt.surface_work_j)
            .abs()
            <= 1e-12
    );
    assert!(
        surface
            .path_is_open(&next, &start, body.body().positions(), &faces)
            .unwrap()
    );
    assert!(body.body().positions()[0][2] > start[0][2] + 0.9e-6);
}

#[test]
fn infeasible_velocity_predictor_keeps_original_error_and_entire_state() {
    use std::sync::Arc;
    let mut body = dynamics(false, 0.1, false);
    let surface = Arc::new(
        PrescribedTriangleSurface::new(
            vec![
                [-1., -1., 0.0139999],
                [1., -1., 0.0139999],
                [0., 1., 0.0139999],
            ],
            vec![[0, 1, 2]],
            0.001,
            0.003,
            1e-8,
        )
        .unwrap(),
    );
    body.set_prescribed_surface(Some(surface.clone())).unwrap();
    let next = Arc::new(
        surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 0.01])
                    .collect(),
            )
            .unwrap(),
    );
    let snapshot = format!("{body:?}");
    assert_eq!(
        body.step_implicit_with_surface_motion(None, next, 1e-5, 1e-12)
            .unwrap_err(),
        "closed surface contact gap"
    );
    assert_eq!(format!("{body:?}"), snapshot);
}

#[test]
fn moving_contact_restores_a_guess_when_stationary_and_velocity_poses_are_closed() {
    use std::sync::Arc;
    let mut body = dynamics(false, 0.0001, false);
    let surface = Arc::new(
        PrescribedTriangleSurface::new(
            vec![
                [-1., -1., 0.0139999],
                [1., -1., 0.0139999],
                [0., 1., 0.0139999],
            ],
            vec![[0, 1, 2]],
            0.001,
            0.003,
            1.,
        )
        .unwrap(),
    );
    body.set_prescribed_surface(Some(surface.clone())).unwrap();
    let next = Arc::new(
        surface
            .with_positions(
                surface
                    .positions()
                    .iter()
                    .map(|p| [p[0], p[1], p[2] + 0.2e-6])
                    .collect(),
            )
            .unwrap(),
    );
    let start = body.body().positions().to_vec();
    let faces = body.body().surface();
    let predicted: Vec<_> = start.iter().map(|p| [p[0], p[1], p[2] + 1e-9]).collect();
    assert!(
        surface
            .path_response(&next, &start, &start, &faces)
            .is_err()
    );
    assert!(
        surface
            .path_response(&next, &start, &predicted, &faces)
            .is_err()
    );
    let before = body.diagnostics().unwrap();
    let receipt = body
        .step_implicit_with_surface_motion(None, next.clone(), 1e-5, 1e-10)
        .unwrap();
    let after = body.diagnostics().unwrap();
    assert!(
        (after.kinetic_j + after.potential_j
            - before.kinetic_j
            - before.potential_j
            - receipt.surface_work_j)
            .abs()
            <= 1e-10
    );
    assert!(
        surface
            .path_is_open(&next, &start, body.body().positions(), &faces)
            .unwrap()
    );
    assert!(body.body().positions()[0][2] > start[0][2] + 0.1e-6);
}

#[test]
fn stiff_supported_tetrahedron_converges_with_strict_midpoint_work() {
    use physics::biomechanics::{Body, InertialBody, Material};
    use std::sync::Arc;
    let reference = vec![[0., 0., 0.1], [0.1, 0., 0.1], [0., 0.1, 0.1], [0., 0., 0.2]];
    let body = Body::new(
        reference.clone(),
        vec![true, true, true, false],
        vec![(
            [0, 1, 2, 3],
            Material::from_young_poisson(1e8, 0.45).unwrap(),
        )],
    )
    .unwrap();
    let mut dynamics = InertialBody::new_with_fixed_supports(
        body,
        &[1000.],
        vec![[0.; 3], [0.; 3], [0.; 3], [0.005, -0.003, -0.01]],
    )
    .unwrap();
    let surface = Arc::new(obstacle());
    dynamics
        .set_prescribed_surface(Some(surface.clone()))
        .unwrap();
    let before = dynamics.diagnostics().unwrap();
    let initial = dynamics.body().positions().to_vec();
    let receipt = dynamics
        .step_implicit_with_surface_motion(None, surface.clone(), 0.002, 1e-10)
        .unwrap();
    let after = dynamics.diagnostics().unwrap();
    assert_eq!(&dynamics.body().positions()[..3], &reference[..3]);
    assert_ne!(dynamics.body().positions()[3], reference[3]);
    let independent_defect =
        after.kinetic_j + after.potential_j - before.kinetic_j - before.potential_j;
    assert!(independent_defect.abs() <= 1e-10);
    assert!((independent_defect - receipt.energy_defect_j).abs() <= 1e-14);
    assert_eq!(receipt.surface_work_j, 0.);
    assert_eq!(receipt.reaction_work_j, 0.);
    assert!(
        surface
            .path_is_open(
                &surface,
                &initial,
                dynamics.body().positions(),
                &[[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]]
            )
            .unwrap()
    );
}

#[test]
fn initial_trajectory_rejects_crossing_missed_by_finite_gauss_samples_atomically() {
    use std::sync::Arc;
    let initial_surface = Arc::new(
        obstacle()
            .with_positions(vec![[-1., -1., -2.], [1., -1., -2.], [0., 1., -2.]])
            .unwrap(),
    );
    let next = Arc::new(
        initial_surface
            .with_positions(vec![[-1., -1., 2.], [1., -1., 2.], [0., 1., 2.]])
            .unwrap(),
    );
    let mut dynamic = dynamics(false, 0., false);
    dynamic
        .set_prescribed_surface(Some(initial_surface.clone()))
        .unwrap();
    let positions = dynamic.body().positions().to_vec();
    let faces = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
    let samples = initial_surface
        .path_response(&next, &positions, &positions, &faces)
        .unwrap();
    assert!(samples.midpoint_objective_j.is_finite());
    assert!(
        samples
            .body_gradient_n
            .iter()
            .flatten()
            .all(|v| v.is_finite())
    );
    assert!(
        !initial_surface
            .path_is_open(&next, &positions, &positions, &faces)
            .unwrap()
    );
    let before = format!("{dynamic:?}");
    assert_eq!(
        dynamic
            .step_implicit_with_surface_motion(None, next, 1e-4, 1e-8)
            .unwrap_err(),
        "implicit initial contact trajectory inadmissible"
    );
    assert_eq!(format!("{dynamic:?}"), before);
}

#[test]
fn nonlinear_prescribed_dilation_rejects_midpoint_work_defect_without_contact() {
    use physics::biomechanics::{Body, InertialBody, Material, SupportTarget};
    use std::sync::Arc;
    let reference = vec![[0., 0., 0.1], [0.1, 0., 0.1], [0., 0.1, 0.1], [0., 0., 0.2]];
    let material = Material::from_young_poisson(1e6, 0.45).unwrap();
    let body = Body::new(
        reference.clone(),
        vec![true; 4],
        vec![([0, 1, 2, 3], material)],
    )
    .unwrap();
    let mut dynamics =
        InertialBody::new_with_fixed_supports(body, &[1000.], vec![[0.; 3]; 4]).unwrap();
    let surface = Arc::new(obstacle());
    dynamics
        .set_prescribed_surface(Some(surface.clone()))
        .unwrap();
    let targets: Vec<_> = reference
        .iter()
        .enumerate()
        .map(|(node, p)| SupportTarget {
            node,
            position_m: p.map(|v| v * 1.05),
        })
        .collect();
    // Independent scalar oracle for uniform dilation: the isochoric term
    // vanishes and U(lambda) = K*V0*(lambda^3-1)^2/2.
    let lambda: f64 = 1.05;
    let midpoint = 0.5 * (1. + lambda);
    let bulk = 1e6 / (3. * (1. - 2. * 0.45));
    let volume = 0.1_f64.powi(3) / 6.;
    let analytic_defect = 0.5 * bulk * volume * (lambda.powi(3) - 1.).powi(2)
        - 3. * bulk * volume * midpoint.powi(2) * (midpoint.powi(3) - 1.) * (lambda - 1.);
    let endpoint: Vec<_> = targets.iter().map(|p| p.position_m).collect();
    let mid: Vec<_> = reference
        .iter()
        .zip(&endpoint)
        .map(|(old, end)| std::array::from_fn(|axis| old[axis] + 0.5 * (end[axis] - old[axis])))
        .collect();
    let initial_energy = dynamics.body().evaluate(&reference).unwrap().0;
    let final_energy = dynamics.body().evaluate(&endpoint).unwrap().0;
    let gradient = dynamics.body().evaluate(&mid).unwrap().1;
    let work: f64 = gradient
        .iter()
        .enumerate()
        .map(|(node, g)| {
            (0..3)
                .map(|axis| g[axis] * (endpoint[node][axis] - reference[node][axis]))
                .sum::<f64>()
        })
        .sum();
    assert!((final_energy - initial_energy - work - analytic_defect).abs() < 1e-10);
    assert!(analytic_defect > 0.1);
    let before = format!("{dynamics:?}");
    assert_eq!(
        dynamics
            .step_implicit_with_surface_motion(Some(&targets), surface, 0.02, 1e-10)
            .unwrap_err(),
        "implicit midpoint work defect"
    );
    assert_eq!(format!("{dynamics:?}"), before);
}
