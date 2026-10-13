use super::*;
use crate::{Joint, Transform};
use glam::{Mat4, Vec3};
fn make_rig(scale: Vec3, second: f32) -> Skeleton {
    let local = |translation| Transform {
        translation,
        ..Transform::IDENTITY
    };
    Skeleton::new(vec![
        Joint {
            name: Arc::from("basis"),
            parent: None,
            bind_local: Transform {
                translation: Vec3::new(0.3, -0.2, 0.5),
                rotation: Quat::from_rotation_z(0.3),
                scale,
                ..Transform::IDENTITY
            },
            inverse_bind: Mat4::IDENTITY,
        },
        Joint {
            name: Arc::from("hip"),
            parent: Some(0),
            bind_local: local(Vec3::ZERO),
            inverse_bind: Mat4::IDENTITY,
        },
        Joint {
            name: Arc::from("knee"),
            parent: Some(1),
            bind_local: local(Vec3::X),
            inverse_bind: Mat4::IDENTITY,
        },
        Joint {
            name: Arc::from("foot"),
            parent: Some(2),
            bind_local: local(Vec3::X * second),
            inverse_bind: Mat4::IDENTITY,
        },
        Joint {
            name: Arc::from("sibling"),
            parent: Some(1),
            bind_local: local(Vec3::Y),
            inverse_bind: Mat4::IDENTITY,
        },
    ])
    .unwrap()
}
fn point(pose: &Pose, rig: &Skeleton, joint: usize) -> Vec3 {
    pose.skin_matrices(rig).unwrap()[joint].w_axis.truncate()
}
fn preserve(before: &Pose, after: &Pose) {
    for (i, (a, b)) in before.local().iter().zip(after.local()).enumerate() {
        assert_eq!(a.translation, b.translation);
        assert_eq!(a.scale, b.scale);
        if i == 0 || i == 4 {
            assert_eq!(a.rotation, b.rotation);
        }
    }
}
#[test]
fn reachable_targets_keep_lengths_pole_side_orientation_and_unrelated_locals() {
    for scale in [Vec3::ONE, Vec3::new(-2., 2., 2.), Vec3::splat(-0.5)] {
        let rig = make_rig(scale, 0.8);
        let chain = TwoBoneChain::new(&rig, [1, 2, 3]).unwrap();
        let before = rig.bind_pose();
        let start = point(&before, &rig, 1);
        let a = point(&before, &rig, 2).distance(start);
        let b = point(&before, &rig, 3).distance(point(&before, &rig, 2));
        for i in 0..40 {
            let phase = i as f32 * 0.16;
            let direction = Vec3::new(phase.cos(), 0.4, phase.sin()).normalize();
            let target = start + direction * (a + b) * 0.7;
            let pole = start + Vec3::Y * (a + b);
            let rotation = Quat::from_rotation_x(0.4) * Quat::from_rotation_y(-0.3);
            let (solved, report) = before
                .solve_two_bone(
                    &chain,
                    TwoBoneTarget {
                        position: target,
                        pole,
                        rotation: Some(rotation),
                        weight: 1.,
                    },
                )
                .unwrap();
            assert!(!report.reach_clamped);
            let knee = point(&solved, &rig, 2);
            let end = point(&solved, &rig, 3);
            assert!(
                end.abs_diff_eq(target, 3e-6),
                "{scale:?} {end:?} {target:?}"
            );
            assert!(report.tip_position.abs_diff_eq(end, 1e-6));
            assert!((knee.distance(start) - a).abs() < 2e-6);
            assert!((end.distance(knee) - b).abs() < 2e-6);
            let transverse = knee - start - direction * (knee - start).dot(direction);
            assert!(transverse.dot(pole - start) > 0.);
            let m = solved.skin_matrices(&rig).unwrap()[3];
            let sign = m.determinant().signum();
            let axes = [
                m.x_axis.truncate(),
                m.y_axis.truncate(),
                m.z_axis.truncate(),
            ];
            for (axis, expected) in axes.into_iter().zip([Vec3::X, Vec3::Y, Vec3::Z]) {
                assert!((axis.normalize() * sign).abs_diff_eq(rotation * expected, 2e-6));
            }
            preserve(&before, &solved);
        }
    }
}
#[test]
fn unreachable_targets_clamp_at_both_reach_limits_and_equal_links_fold_at_zero() {
    for second in [0.5, 1., 2.] {
        let rig = make_rig(Vec3::ONE, second);
        let chain = TwoBoneChain::new(&rig, [1, 2, 3]).unwrap();
        let pose = rig.bind_pose();
        let root = point(&pose, &rig, 1);
        for distance in [0., 0.01, 10.] {
            let target = root + Vec3::Z * distance;
            let (solved, result) = pose
                .solve_two_bone(
                    &chain,
                    TwoBoneTarget {
                        position: target,
                        pole: root + Vec3::Y,
                        rotation: None,
                        weight: 1.,
                    },
                )
                .unwrap();
            let reach = distance.clamp((1_f32 - second).abs(), 1. + second);
            assert!(
                (result.tip_position.distance(root) - reach).abs() < 2e-6,
                "{second} {distance} {result:?}"
            );
            if distance > 0. {
                assert!(
                    result
                        .tip_position
                        .abs_diff_eq(root + Vec3::Z * reach, 3e-6)
                );
            }
            assert_eq!(result.reach_clamped, distance != reach);
            preserve(&pose, &solved);
        }
    }
}
#[test]
fn degenerate_poles_and_antiparallel_targets_are_finite_and_repeatable() {
    let rig = make_rig(Vec3::ONE, 1.);
    let chain = TwoBoneChain::new(&rig, [1, 2, 3]).unwrap();
    let pose = rig.bind_pose();
    let root = point(&pose, &rig, 1);
    let original = point(&pose, &rig, 3) - root;
    for direction in [original.normalize(), -original.normalize(), Vec3::X] {
        for pole in [root, root + direction] {
            let target = TwoBoneTarget {
                position: root + direction * 1.2,
                pole,
                rotation: None,
                weight: 1.,
            };
            let (a, ra) = pose.solve_two_bone(&chain, target).unwrap();
            let (b, rb) = pose.solve_two_bone(&chain, target).unwrap();
            assert_eq!(a, b);
            assert_eq!(ra, rb);
            assert!(ra.tip_position.abs_diff_eq(target.position, 3e-6));
        }
    }
}
#[test]
fn weights_and_frame_palette_preserve_root_motion_and_publication_metadata() {
    let rig = make_rig(Vec3::ONE, 0.8);
    let chain = TwoBoneChain::new(&rig, [1, 2, 3]).unwrap();
    let pose = rig.bind_pose();
    let root = point(&pose, &rig, 1);
    let target = TwoBoneTarget {
        position: root + Vec3::new(0.5, 0.8, 0.2),
        pole: root + Vec3::Y,
        rotation: None,
        weight: 1.,
    };
    let (full, _) = pose.solve_two_bone(&chain, target).unwrap();
    let (zero, _) = pose
        .solve_two_bone(
            &chain,
            TwoBoneTarget {
                weight: 0.,
                ..target
            },
        )
        .unwrap();
    assert_eq!(zero, pose);
    let (half, _) = pose
        .solve_two_bone(
            &chain,
            TwoBoneTarget {
                weight: 0.5,
                ..target
            },
        )
        .unwrap();
    for i in [1, 2, 3] {
        assert!(
            half.local()[i].rotation.abs_diff_eq(
                pose.local()[i]
                    .rotation
                    .slerp(full.local()[i].rotation, 0.5)
                    .normalize(),
                1e-6
            )
        );
    }
    let frame = AnimatorFrame {
        skin_matrices: pose.skin_matrices(&rig).unwrap(),
        pose,
        root_motion: Vec3::new(0.2, 0., -0.1),
        root_motion_joint: 1,
        transition_weight: 0.4,
    };
    let previous = frame.clone();
    let (next, _) = frame.with_two_bone_ik(&rig, &chain, target).unwrap();
    assert_eq!(next.root_motion, previous.root_motion);
    assert_eq!(next.root_motion_joint, previous.root_motion_joint);
    assert_eq!(next.transition_weight, previous.transition_weight);
    assert_eq!(next.skin_matrices, next.pose.skin_matrices(&rig).unwrap());
    assert_ne!(next.skin_matrices, previous.skin_matrices);
}
#[test]
fn invalid_targets_foreign_chains_and_unsupported_scales_do_not_change_source() {
    let rig = make_rig(Vec3::ONE, 1.);
    let chain = TwoBoneChain::new(&rig, [1, 2, 3]).unwrap();
    let pose = rig.bind_pose();
    let original = pose.clone();
    let target = TwoBoneTarget {
        position: Vec3::X,
        pole: Vec3::Y,
        rotation: None,
        weight: 1.,
    };
    for invalid in [
        TwoBoneTarget {
            position: Vec3::splat(f32::NAN),
            ..target
        },
        TwoBoneTarget {
            pole: Vec3::splat(f32::INFINITY),
            ..target
        },
        TwoBoneTarget {
            weight: -0.1,
            ..target
        },
        TwoBoneTarget {
            rotation: Some(Quat::from_xyzw(0., 0., 0., 2.)),
            ..target
        },
    ] {
        assert_eq!(
            pose.solve_two_bone(&chain, invalid).unwrap_err(),
            AnimationError::InvalidIkTarget
        );
        assert_eq!(pose, original);
    }
    let foreign = make_rig(Vec3::ONE, 0.8);
    assert_eq!(
        pose.solve_two_bone(&TwoBoneChain::new(&foreign, [1, 2, 3]).unwrap(), target)
            .unwrap_err(),
        AnimationError::SkeletonMismatch
    );
    assert!(TwoBoneChain::new(&rig, [0, 2, 3]).is_err());
    let scaled = make_rig(Vec3::new(1., 2., 1.), 1.);
    let scaled_chain = TwoBoneChain::new(&scaled, [1, 2, 3]).unwrap();
    assert_eq!(
        scaled
            .bind_pose()
            .solve_two_bone(&scaled_chain, target)
            .unwrap_err(),
        AnimationError::UnsupportedIkScale
    );
    let collapsed = make_rig(Vec3::ONE, 0.);
    assert_eq!(
        collapsed
            .bind_pose()
            .solve_two_bone(&TwoBoneChain::new(&collapsed, [1, 2, 3]).unwrap(), target)
            .unwrap_err(),
        AnimationError::DegenerateIkChain
    );
}

#[test]
fn signed_nonuniform_tip_scale_keeps_proper_target_orientation_without_affecting_lengths() {
    for parent_scale in [Vec3::ONE, Vec3::new(-2., 2., 2.)] {
        let source = make_rig(parent_scale, 0.8);
        let mut joints = source.joints().to_vec();
        joints[3].bind_local.scale = Vec3::new(-1., 2., 1.);
        let rig = Skeleton::new(joints).unwrap();
        let chain = TwoBoneChain::new(&rig, [1, 2, 3]).unwrap();
        let pose = rig.bind_pose();
        let root = point(&pose, &rig, 1);
        let rotation = Quat::from_rotation_z(-0.7) * Quat::from_rotation_x(0.3);
        let target = TwoBoneTarget {
            position: root + Vec3::new(0.6, 0.8, 0.2),
            pole: root + Vec3::Y,
            rotation: Some(rotation),
            weight: 1.,
        };
        let (solved, result) = pose.solve_two_bone(&chain, target).unwrap();
        assert!(result.tip_position.abs_diff_eq(target.position, 3e-6));
        let matrix = solved.skin_matrices(&rig).unwrap()[3];
        let sign = matrix.determinant().signum();
        for (axis, expected) in [matrix.x_axis, matrix.y_axis, matrix.z_axis]
            .into_iter()
            .zip([Vec3::X, Vec3::Y, Vec3::Z])
        {
            assert!((axis.truncate().normalize() * sign).abs_diff_eq(rotation * expected, 2e-6));
        }
        preserve(&pose, &solved);
    }
}

#[test]
fn soft_extension_is_monotone_c1_and_bounded() {
    let length = 1.8;
    for ratio in [0., 0.5, 0.9, 0.999] {
        let boundary = length * ratio;
        let h = (length - boundary) * 1e-6;
        assert_eq!(soft_distance(boundary, length, ratio), boundary);
        let right = (soft_distance(boundary + h, length, ratio) - boundary) / h;
        assert!((right - 1.).abs() < 2e-6, "boundary slope {right}");
        let mut previous = boundary;
        for step in 1..=1000 {
            let distance = boundary + (length - boundary) * step as f64 / 100.;
            let actual = soft_distance(distance, length, ratio);
            assert!(actual >= previous && actual <= length && actual <= distance);
            previous = actual;
        }
    }
    for distance in [0., 0.2, 1.8, 100.] {
        assert_eq!(soft_distance(distance, length, 1.), distance);
    }
}

#[test]
fn softened_ik_preserves_exact_mode_lengths_and_geometric_reach_report() {
    let rig = make_rig(Vec3::ONE, 0.8);
    let chain = TwoBoneChain::new(&rig, [1, 2, 3]).unwrap();
    let pose = rig.bind_pose();
    let start = point(&pose, &rig, 1);
    let length = point(&pose, &rig, 2).distance(start)
        + point(&pose, &rig, 3).distance(point(&pose, &rig, 2));
    for distance in [0., 0.1, 1., 1.7, 2.5] {
        let target = TwoBoneTarget {
            position: start + Vec3::Y * distance,
            pole: start + Vec3::Z,
            rotation: None,
            weight: 1.,
        };
        let (exact, exact_report) = pose.solve_two_bone(&chain, target).unwrap();
        let (disabled, disabled_report) = pose.solve_two_bone_softened(&chain, target, 1.).unwrap();
        assert_eq!(exact_report, disabled_report);
        for (a, b) in exact.local().iter().zip(disabled.local()) {
            assert_eq!(a.rotation, b.rotation);
        }
        let (soft, report) = pose.solve_two_bone_softened(&chain, target, 0.8).unwrap();
        preserve(&pose, &soft);
        assert_eq!(report.reach_clamped, exact_report.reach_clamped);
        let expected = soft_distance(f64::from(distance), f64::from(length), f64::from(0.8f32))
            .clamp(0.2, f64::from(length)) as f32;
        assert!((report.tip_position.distance(start) - expected).abs() < 2e-5);
    }
    let target = TwoBoneTarget {
        position: start,
        pole: Vec3::Z,
        rotation: None,
        weight: 1.,
    };
    for ratio in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
        assert!(pose.solve_two_bone_softened(&chain, target, ratio).is_err());
    }
}

#[test]
fn softened_frame_rebuilds_palette_without_modifying_extracted_motion() {
    for scale in [Vec3::ONE, Vec3::new(-2., 2., 2.), Vec3::splat(-0.5)] {
        let rig = make_rig(scale, 0.8);
        let chain = TwoBoneChain::new(&rig, [1, 2, 3]).unwrap();
        let pose = rig.bind_pose();
        let start = point(&pose, &rig, 1);
        let length = point(&pose, &rig, 2).distance(start)
            + point(&pose, &rig, 3).distance(point(&pose, &rig, 2));
        let frame = AnimatorFrame {
            skin_matrices: pose.skin_matrices(&rig).unwrap(),
            pose,
            root_motion: Vec3::new(0.1, -0.2, 0.3),
            root_motion_joint: 1,
            transition_weight: 0.45,
        };
        let target = TwoBoneTarget {
            position: start + Vec3::Y * length * 0.98,
            pole: start + Vec3::Z,
            rotation: Some(Quat::from_rotation_z(0.4)),
            weight: 1.,
        };
        let (next, report) = frame
            .clone()
            .with_softened_two_bone_ik(&rig, &chain, target, 0.8)
            .unwrap();
        assert_eq!(next.root_motion, frame.root_motion);
        assert_eq!(next.root_motion_joint, frame.root_motion_joint);
        assert_eq!(next.transition_weight, frame.transition_weight);
        assert_eq!(next.skin_matrices, next.pose.skin_matrices(&rig).unwrap());
        assert!(report.tip_position.distance(start) < target.position.distance(start));
        assert!(!report.reach_clamped);
        preserve(&frame.pose, &next.pose);
        let knee = point(&next.pose, &rig, 2);
        assert!((knee.distance(start) - length / 1.8).abs() < 2e-5);
        assert!((report.tip_position.distance(knee) - length * 0.8 / 1.8).abs() < 2e-5);
        let foreign = make_rig(scale, 0.9);
        assert!(
            frame
                .clone()
                .with_softened_two_bone_ik(&foreign, &chain, target, 0.8)
                .is_err()
        );
        assert!(
            frame
                .with_softened_two_bone_ik(&rig, &chain, target, f32::NAN)
                .is_err()
        );
    }
}

#[test]
fn planted_ankle_stays_on_support_through_actor_motion_and_exact_ik() {
    use crate::{FootPlant, RootRigidTransform};
    let rig = make_rig(Vec3::ONE, 0.8);
    let chain = TwoBoneChain::new(&rig, [1, 2, 3]).unwrap();
    let pose = rig.bind_pose();
    let start = point(&pose, &rig, 1);
    let anchor = RootRigidTransform {
        translation: (start + Vec3::Y * 1.2).as_dvec3(),
        rotation: DQuat::from_rotation_z(0.3),
    };
    let support = RootRigidTransform::IDENTITY;
    let plant = FootPlant::capture(anchor, support).unwrap();
    for step in 0..60 {
        let actor = RootRigidTransform {
            translation: DVec3::new(0.05 * (step as f64 * 0.1).sin(), 0., 0.),
            rotation: DQuat::from_rotation_y(0.05 * (step as f64 * 0.1).cos()),
        };
        let target = plant.target(support, actor, start + Vec3::Z, 1.).unwrap();
        let (solved, report) = pose.solve_two_bone(&chain, target).unwrap();
        assert!(!report.reach_clamped);
        assert!(
            actor
                .transform_point(report.tip_position.as_dvec3())
                .unwrap()
                .distance(anchor.translation)
                < 3e-6
        );
        preserve(&pose, &solved);
        let global_rotation = globals(&solved).unwrap()[3]
            .to_scale_rotation_translation()
            .1;
        assert!(
            (actor.rotation * global_rotation)
                .dot(anchor.rotation)
                .abs()
                > 1. - 1e-6
        );
    }
}

fn planted_rig() -> (Skeleton, [TwoBoneChain; 2]) {
    let mut joints = vec![Joint {
        name: Arc::from("pelvis"),
        parent: None,
        bind_local: Transform {
            translation: Vec3::Y * 1.5,
            ..Transform::IDENTITY
        },
        inverse_bind: Mat4::IDENTITY,
    }];
    for (side, x) in [-0.2, 0.2].into_iter().enumerate() {
        let base = joints.len() as u16;
        for (part, parent, translation) in [
            ("hip", 0, Vec3::X * x),
            ("knee", base, -Vec3::Y * 0.8),
            ("ankle", base + 1, -Vec3::Y * 0.6),
        ] {
            joints.push(Joint {
                name: Arc::from(format!("{side}_{part}")),
                parent: Some(parent),
                bind_local: Transform {
                    translation,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::IDENTITY,
            });
        }
    }
    let rig = Skeleton::new(joints).unwrap();
    let chains = [
        TwoBoneChain::new(&rig, [1, 2, 3]).unwrap(),
        TwoBoneChain::new(&rig, [4, 5, 6]).unwrap(),
    ];
    (rig, chains)
}

#[test]
fn shared_pelvis_reaches_both_uneven_plants_with_bounded_offset() {
    let (rig, chains) = planted_rig();
    let pose = rig.bind_pose();
    let targets = [-0.2, 0.2].map(|x| TwoBoneTarget {
        position: Vec3::new(x, if x < 0. { 0. } else { 0.2 }, 0.),
        pole: Vec3::new(x, 0.8, 1.),
        rotation: Some(Quat::IDENTITY),
        weight: 1.,
    });
    let (solved, report) = pose
        .solve_planted_legs(&chains, targets, Vec3::Y, 0.3)
        .unwrap();
    assert!(report.pelvis_offset.abs_diff_eq(-Vec3::Y * 0.1, 2e-6));
    for side in 0..2 {
        assert!(
            report.legs[side]
                .tip_position
                .distance(targets[side].position)
                < 3e-6
        );
        let [hip, knee, ankle] = chains[side].joints.map(|i| point(&solved, &rig, i));
        assert!((hip.distance(knee) - 0.8).abs() < 2e-6);
        assert!((knee.distance(ankle) - 0.6).abs() < 2e-6);
    }
    for joint in 1..pose.local().len() {
        assert_eq!(
            pose.local()[joint].translation,
            solved.local()[joint].translation
        );
        assert_eq!(pose.local()[joint].scale, solved.local()[joint].scale);
    }
    assert_eq!(
        pose.solve_planted_legs(&chains, targets, Vec3::Y, 0.05)
            .unwrap_err(),
        AnimationError::UnreachableFootPlant
    );
    assert_eq!(pose, rig.bind_pose());
}

#[test]
fn pelvis_reach_accounts_for_fold_limit_and_invalid_contact_jobs() {
    let (rig, chains) = planted_rig();
    let pose = rig.bind_pose();
    let targets = [-0.2, 0.2].map(|x| TwoBoneTarget {
        position: Vec3::new(x, 1.5, 0.),
        pole: Vec3::new(x, 0.8, 1.),
        rotation: None,
        weight: 1.,
    });
    let (_, report) = pose
        .solve_planted_legs(&chains, targets, Vec3::Y, 0.5)
        .unwrap();
    assert!((report.pelvis_offset.length() - 0.2).abs() < 2e-6);
    for (leg, target) in report.legs.iter().zip(targets) {
        assert!(leg.tip_position.distance(target.position) < 3e-6);
    }
    assert!(
        pose.solve_planted_legs(
            &[chains[0].clone(), chains[0].clone()],
            targets,
            Vec3::Y,
            0.5
        )
        .is_err()
    );
    assert!(
        pose.solve_planted_legs(&chains, targets, Vec3::ZERO, 0.5)
            .is_err()
    );
    assert!(
        pose.solve_planted_legs(&chains, targets, Vec3::Y, f32::NAN)
            .is_err()
    );
    let mut blended = targets;
    blended[0].weight = 0.5;
    assert!(
        pose.solve_planted_legs(&chains, blended, Vec3::Y, 0.5)
            .is_err()
    );
    // An already feasible stance must not add pelvis motion.
    let targets = [3, 6].map(|joint| TwoBoneTarget {
        position: point(&pose, &rig, joint),
        pole: Vec3::Z,
        rotation: None,
        weight: 1.,
    });
    let (_, report) = pose
        .solve_planted_legs(&chains, targets, Vec3::Y, 0.5)
        .unwrap();
    assert_eq!(report.pelvis_offset, Vec3::ZERO);
}

#[test]
fn planted_frame_correction_does_not_become_actor_root_motion() {
    let (rig, chains) = planted_rig();
    let pose = rig.bind_pose();
    let frame = AnimatorFrame {
        skin_matrices: pose.skin_matrices(&rig).unwrap(),
        pose,
        root_motion: Vec3::new(0.3, 0., 0.2),
        root_motion_joint: 0,
        transition_weight: 0.6,
    };
    let targets = [-0.2, 0.2].map(|x| TwoBoneTarget {
        position: Vec3::new(x, 0., 0.),
        pole: Vec3::Z,
        rotation: None,
        weight: 1.,
    });
    let previous = frame.clone();
    let (next, report) = frame
        .with_planted_legs(&rig, &chains, targets, Vec3::Y, 0.3)
        .unwrap();
    assert!(report.pelvis_offset.y < 0.);
    assert_eq!(next.root_motion, previous.root_motion);
    assert_eq!(next.root_motion_joint, previous.root_motion_joint);
    assert_eq!(next.transition_weight, previous.transition_weight);
    assert_eq!(next.skin_matrices, next.pose.skin_matrices(&rig).unwrap());
}

#[test]
fn exact_ik_preserves_toe_contact_during_animated_foot_roll() {
    use crate::{FootPlant, RootRigidTransform};
    let rig = make_rig(Vec3::ONE, 0.8);
    let chain = TwoBoneChain::new(&rig, [1, 2, 3]).unwrap();
    let pose = rig.bind_pose();
    let start = point(&pose, &rig, 1);
    let initial = RootRigidTransform {
        translation: (start + Vec3::Y * 1.2).as_dvec3(),
        rotation: DQuat::IDENTITY,
    };
    let pivot = DVec3::new(0., -0.05, 0.12);
    let support = RootRigidTransform::IDENTITY;
    let plant = FootPlant::capture_pivot(initial, pivot, support).unwrap();
    let contact = initial.transform_point(pivot).unwrap();
    for step in 0..60 {
        let actor = RootRigidTransform {
            translation: DVec3::X * (step as f64 * 0.1).sin() * 0.05,
            rotation: DQuat::from_rotation_y(0.03),
        };
        let rotation = DQuat::from_rotation_x(step as f64 * 0.01);
        let target = plant
            .target_with_rotation(support, actor, rotation, Vec3::Z, 1.)
            .unwrap();
        let (solved, report) = pose.solve_two_bone(&chain, target).unwrap();
        assert!(!report.reach_clamped);
        let foot = globals(&solved).unwrap()[3];
        let actual = actor.transform_point(foot.transform_point3(pivot)).unwrap();
        assert!(
            actual.distance(contact) < 3e-6,
            "toe drift at {step}: {actual:?} != {contact:?}"
        );
    }
}

#[test]
fn extension_margin_conditions_knees_without_sliding_planted_feet() {
    let (rig, chains) = planted_rig();
    let pose = rig.bind_pose();
    let targets = [3, 6].map(|foot| TwoBoneTarget {
        position: point(&pose, &rig, foot),
        pole: Vec3::Z,
        rotation: Some(Quat::IDENTITY),
        weight: 1.,
    });
    let (solved, report) = pose
        .solve_planted_legs_with_extension_margin(&chains, targets, Vec3::Y, 0.1, 0.015)
        .unwrap();
    assert!((report.pelvis_offset.y + 0.015).abs() < 2e-6);
    for (chain, target) in chains.iter().zip(targets) {
        let [hip, knee, ankle] = chain.joints.map(|j| point(&solved, &rig, j));
        assert!(ankle.distance(target.position) < 3e-6);
        assert!(hip.distance(ankle) <= 1.385 + 2e-6);
        assert!((hip.distance(knee) - 0.8).abs() < 2e-6);
        assert!((knee.distance(ankle) - 0.6).abs() < 2e-6);
    }
    let zero = pose
        .solve_planted_legs_with_extension_margin(&chains, targets, Vec3::Y, 0.1, 0.)
        .unwrap();
    assert_eq!(
        zero.0,
        pose.solve_planted_legs(&chains, targets, Vec3::Y, 0.1)
            .unwrap()
            .0
    );
    for margin in [f32::NAN, -0.01, 1.21] {
        assert_eq!(
            pose.solve_planted_legs_with_extension_margin(&chains, targets, Vec3::Y, 0.1, margin)
                .unwrap_err(),
            AnimationError::InvalidIkTarget
        );
    }
    assert_eq!(
        pose.solve_planted_legs_with_extension_margin(&chains, targets, Vec3::Y, 0.005, 0.015)
            .unwrap_err(),
        AnimationError::UnreachableFootPlant
    );
    assert_eq!(pose, rig.bind_pose());
}
