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
