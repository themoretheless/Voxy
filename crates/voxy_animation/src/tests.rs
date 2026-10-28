use super::*;

#[test]
fn local_f64_time_narrowing_preserves_same_time_translation_error_and_key_boundaries() {
    let rig = skeleton();
    let mut tracks = vec![JointTrack::default(); 2];
    tracks[0].translations = vec![
        Vec3Key {
            time: 0.,
            value: Vec3::new(1e6, -1e6, 0.1),
        },
        Vec3Key {
            time: 0.5,
            value: Vec3::new(-1e6, 1e6, -0.2),
        },
        Vec3Key {
            time: 1.,
            value: Vec3::ZERO,
        },
    ];
    let clip = AnimationClip::new("local", 1., Playback::Clamp, tracks, &rig).unwrap();
    let caps = clip
        .joint_translation_local_time_error_bounds(0, [0., 0.5])
        .unwrap()
        .unwrap();
    for step in 0..=17 {
        let time = step as f64 / 34.;
        let actual = clip.sample(&rig, time as f32).local()[0].translation;
        println!(
            "source_local_time_reference={{\"time\":{:?},\"actual\":{:?},\"caps\":{:?}}}",
            time,
            actual.to_array(),
            caps
        );
    }
    assert_eq!(
        clip.joint_translation_local_time_error_bounds(0, [0.25, 0.75])
            .unwrap(),
        None
    );
    assert!(
        clip.joint_translation_local_time_error_bounds(0, [0.5_f64.next_down(); 2])
            .unwrap()
            .is_some()
    );
    assert!(
        clip.joint_translation_local_time_error_bounds(0, [0.5_f64.next_up(); 2])
            .unwrap()
            .is_some()
    );
    assert!(
        clip.joint_translation_local_time_error_bounds(0, [0., f64::NAN])
            .is_err()
    );
    assert!(
        clip.joint_translation_local_time_error_bounds(0, [-1., 0.])
            .is_err()
    );
    assert_eq!(
        clip.joint_translation_local_time_error_bounds(1, [0., 1.])
            .unwrap(),
        Some([0.; 3])
    );
}
#[test]
fn local_translation_sample_error_partition_and_held_channel_contract() {
    let rig = skeleton();
    let mut tracks = vec![JointTrack::default(); 2];
    tracks[0].translations = vec![
        Vec3Key {
            time: 0.,
            value: Vec3::ZERO,
        },
        Vec3Key {
            time: 0.5,
            value: Vec3::X,
        },
        Vec3Key {
            time: 1.,
            value: Vec3::Y,
        },
    ];
    let linear = AnimationClip::new("linear", 1., Playback::Clamp, tracks.clone(), &rig).unwrap();
    assert_eq!(
        linear
            .joint_translation_sample_error_bounds(0, [0.25, 0.75])
            .unwrap(),
        None
    );
    assert!(
        linear
            .joint_translation_sample_error_bounds(0, [0., 0.5])
            .unwrap()
            .is_some()
    );
    assert!(
        linear
            .joint_translation_sample_error_bounds(0, [0.5, 1.])
            .unwrap()
            .is_some()
    );
    assert_eq!(
        linear
            .joint_translation_sample_error_bounds(1, [0., 1.])
            .unwrap(),
        Some([0.; 3])
    );
    let step = AnimationClip::new_with_interpolation(
        "step",
        1.,
        Playback::Clamp,
        tracks,
        vec![
            TrackInterpolation {
                translation: Interpolation::Step,
                ..TrackInterpolation::default()
            },
            TrackInterpolation::default(),
        ],
        &rig,
    )
    .unwrap();
    assert_eq!(
        step.joint_translation_local_time_error_bounds(0, [0.5_f64.next_down(); 2])
            .unwrap(),
        None
    );
    assert_eq!(
        step.joint_translation_sample_error_bounds(0, [0., 0.25])
            .unwrap(),
        Some([0.; 3])
    );
    assert_eq!(
        step.joint_translation_sample_error_bounds(0, [0., 0.5])
            .unwrap(),
        None
    );
    assert_eq!(
        step.joint_translation_sample_error_bounds(0, [0.5, 0.75])
            .unwrap(),
        Some([0.; 3])
    );
    assert!(
        step.joint_translation_sample_error_bounds(0, [0., f32::INFINITY])
            .is_err()
    );
    assert!(
        step.joint_translation_sample_error_bounds(0, [0., 1.1])
            .is_err()
    );
    let constant = AnimationClip::new(
        "constant",
        1.,
        Playback::Clamp,
        vec![
            JointTrack {
                translations: vec![
                    Vec3Key {
                        time: 0.,
                        value: Vec3::splat(1e6),
                    },
                    Vec3Key {
                        time: 1.,
                        value: Vec3::splat(1e6),
                    },
                ],
                ..JointTrack::default()
            },
            JointTrack::default(),
        ],
        &rig,
    )
    .unwrap();
    assert_eq!(
        constant
            .joint_translation_sample_error_bounds(0, [0., 1.])
            .unwrap(),
        Some([0.; 3])
    );
}

#[test]
fn rig_binding_accepts_exact_reconstruction_and_rejects_same_count_foreign_layouts() {
    let source = skeleton();
    let clip = AnimationClip::new(
        "bound",
        2.,
        Playback::Clamp,
        vec![JointTrack::default(); 2],
        &source,
    )
    .unwrap();
    let reconstructed = Skeleton::new(source.joints().to_vec()).unwrap();
    assert!(!Arc::ptr_eq(&source.joints, &reconstructed.joints));
    let pose = clip.try_sample(&reconstructed, 0.5).unwrap();
    assert_eq!(
        pose.skin_matrices(&reconstructed).unwrap(),
        source.bind_pose().skin_matrices(&source).unwrap()
    );
    assert!(Pose::blend(&pose, &reconstructed.bind_pose(), 0.5).is_ok());
    for change in 0..5 {
        let mut joints = source.joints().to_vec();
        match change {
            0 => joints[1].name = Arc::from("other bone"),
            1 => joints[1].parent = None,
            2 => joints[1].bind_local.translation = Vec3::Z,
            3 => joints[1].inverse_bind = Mat4::from_translation(-Vec3::Z),
            _ => {
                joints[0].name = Arc::from("hand");
                joints[1].name = Arc::from("root");
            }
        }
        let foreign = Skeleton::new(joints).unwrap();
        assert!(matches!(
            clip.try_sample(&foreign, 0.5),
            Err(AnimationError::SkeletonMismatch)
        ));
        assert!(matches!(
            pose.skin_matrices(&foreign),
            Err(AnimationError::SkeletonMismatch)
        ));
        for weight in [0., 0.5, 1.] {
            assert!(matches!(
                Pose::blend(&pose, &foreign.bind_pose(), weight),
                Err(AnimationError::SkeletonMismatch)
            ));
        }
        // The legacy sampler cannot substitute foreign bind defaults or relabel a clip.
        let unchecked = clip.sample(&foreign, 0.5);
        assert_eq!(unchecked, source.bind_pose());
        assert!(matches!(
            unchecked.skin_matrices(&foreign),
            Err(AnimationError::SkeletonMismatch)
        ));
    }
}

#[test]
fn foreign_transition_and_advance_preserve_active_crossfade() {
    let source = skeleton();
    let make = |rig: &Skeleton, name: &str| {
        Arc::new(
            AnimationClip::new(
                name,
                2.,
                Playback::Loop,
                vec![JointTrack::default(); 2],
                rig,
            )
            .unwrap(),
        )
    };
    let first = make(&source, "first");
    let reconstructed = Skeleton::new(source.joints().to_vec()).unwrap();
    let second = make(&reconstructed, "second");
    let mut animator = Animator::new(first);
    animator.advance(&source, 0.2).unwrap();
    animator.transition_to(second, 0.5).unwrap();
    animator.advance(&source, 0.1).unwrap();
    let mut control = animator.clone();
    let mut joints = source.joints().to_vec();
    joints[1].parent = None;
    let foreign = Skeleton::new(joints).unwrap();
    for duration in [0., 0.5] {
        assert!(matches!(
            animator.transition_to(make(&foreign, "foreign"), duration),
            Err(AnimationError::SkeletonMismatch)
        ));
        assert!(Arc::ptr_eq(&animator.current, &control.current));
        assert_eq!(animator.time, control.time);
        assert_eq!(
            animator.transition.as_ref().unwrap().elapsed,
            control.transition.as_ref().unwrap().elapsed
        );
        assert_eq!(
            animator.transition.as_ref().unwrap().source_time,
            control.transition.as_ref().unwrap().source_time
        );
    }
    assert!(matches!(
        animator.advance(&foreign, 0.1),
        Err(AnimationError::SkeletonMismatch)
    ));
    let actual = animator.advance(&source, 0.1).unwrap();
    let expected = control.advance(&source, 0.1).unwrap();
    assert_eq!(actual.pose, expected.pose);
    assert_eq!(actual.skin_matrices, expected.skin_matrices);
    assert_eq!(actual.transition_weight, expected.transition_weight);
    assert_eq!(actual.root_motion, expected.root_motion);
}

#[test]
fn interrupted_crossfade_starts_from_displayed_pose_and_releases_snapshot() {
    let rig = skeleton();
    let constant = |name: &str, x: f32| {
        let mut tracks = vec![JointTrack::default(); 2];
        tracks[0].translations = vec![Vec3Key {
            time: 0.,
            value: Vec3::X * x,
        }];
        Arc::new(AnimationClip::new(name, 2., Playback::Clamp, tracks, &rig).unwrap())
    };
    let mut animator = Animator::new(constant("a", 0.));
    animator.transition_to(constant("b", 10.), 1.).unwrap();
    let displayed = animator.advance(&rig, 0.25).unwrap();
    assert_eq!(displayed.pose.local()[0].translation.x, 2.5);
    animator.transition_to(constant("c", 20.), 1.).unwrap();
    let snapshot = Arc::downgrade(
        animator
            .transition
            .as_ref()
            .unwrap()
            .source_pose
            .as_ref()
            .unwrap(),
    );
    assert_eq!(animator.advance(&rig, 0.).unwrap().pose, displayed.pose);
    assert_eq!(
        animator.advance(&rig, 0.2).unwrap().pose.local()[0]
            .translation
            .x,
        6.
    );
    let next_displayed = animator.advance(&rig, 0.).unwrap().pose;
    animator.transition_to(constant("d", -10.), 0.5).unwrap();
    assert!(snapshot.upgrade().is_none());
    assert_eq!(animator.advance(&rig, 0.).unwrap().pose, next_displayed);
    let snapshot = Arc::downgrade(
        animator
            .transition
            .as_ref()
            .unwrap()
            .source_pose
            .as_ref()
            .unwrap(),
    );
    assert_eq!(
        animator.advance(&rig, 0.5).unwrap().pose.local()[0]
            .translation
            .x,
        -10.
    );
    assert!(animator.transition.is_none());
    assert!(snapshot.upgrade().is_none());
}

#[test]
fn interrupted_snapshot_is_shared_and_failed_advance_keeps_it() {
    let rig = skeleton();
    let mut animator = Animator::new(root_clip(&rig, 0.));
    animator.transition_to(root_clip(&rig, 2.), 1.).unwrap();
    animator.advance(&rig, 0.25).unwrap();
    let displayed = animator.advance(&rig, 0.).unwrap().pose;
    animator.transition_to(root_clip(&rig, 4.), 1.).unwrap();
    assert_eq!(animator.advance(&rig, 0.).unwrap().pose, displayed);
    let mut control = animator.clone();
    let left = animator
        .transition
        .as_ref()
        .unwrap()
        .source_pose
        .as_ref()
        .unwrap();
    let right = control
        .transition
        .as_ref()
        .unwrap()
        .source_pose
        .as_ref()
        .unwrap();
    assert!(Arc::ptr_eq(left, right));
    assert!(animator.advance(&rig, f32::NAN).is_err());
    assert_eq!(animator.time, control.time);
    let actual = animator.advance(&rig, 0.1).unwrap();
    let expected = control.advance(&rig, 0.1).unwrap();
    assert_eq!(actual.pose, expected.pose);
    assert_eq!(actual.skin_matrices, expected.skin_matrices);
}

#[test]
fn interrupted_fade_preserves_rotated_scaled_hierarchical_palette() {
    let rig = skeleton();
    let clip = |name: &str, angle: f32, scale: f32| {
        let mut tracks = vec![JointTrack::default(); 2];
        for track in &mut tracks {
            track.rotations = vec![QuatKey {
                time: 0.,
                value: Quat::from_rotation_y(angle),
            }];
            track.scales = vec![Vec3Key {
                time: 0.,
                value: Vec3::splat(scale),
            }];
        }
        Arc::new(AnimationClip::new(name, 1., Playback::Clamp, tracks, &rig).unwrap())
    };
    let mut animator = Animator::new(clip("a", -0.6, 0.8));
    animator.transition_to(clip("b", 1.2, 1.7), 1.).unwrap();
    let before = animator.advance(&rig, 0.3).unwrap();
    animator.transition_to(clip("c", -1.1, 1.1), 0.4).unwrap();
    let after = animator.advance(&rig, 0.).unwrap();
    for (a, b) in before.pose.local().iter().zip(after.pose.local()) {
        assert!(a.translation.abs_diff_eq(b.translation, 1e-6));
        assert!(a.scale.abs_diff_eq(b.scale, 1e-6));
        assert!(a.rotation.angle_between(b.rotation) < 1e-5);
    }
    for (a, b) in before.skin_matrices.iter().zip(after.skin_matrices.iter()) {
        assert!(a.abs_diff_eq(*b, 2e-6));
    }
    let frame = animator.advance(&rig, 0.4).unwrap();
    assert!(frame.skin_matrices.iter().all(|m| m.is_finite()));
    assert!(animator.transition.is_none());
}

#[test]
fn root_motion_crossfade_blends_velocities_and_splits_completion_tail() {
    let rig = skeleton();
    let mut animator = Animator::new(root_clip(&rig, 2.));
    animator.advance(&rig, 0.8).unwrap();
    animator.transition_to(root_clip(&rig, 6.), 1.).unwrap();
    let frame = animator.advance(&rig, 0.25).unwrap();
    assert!((frame.root_motion.x - 0.625).abs() < 1e-5);
    let mut single = animator.clone();
    let mut split = animator.clone();
    let whole = single.advance(&rig, 1.).unwrap().root_motion;
    let pieces = split.advance(&rig, 0.5).unwrap().root_motion
        + split.advance(&rig, 0.5).unwrap().root_motion;
    assert!(whole.abs_diff_eq(pieces, 1e-5));
    assert!((whole.x - 4.875).abs() < 1e-5);
    assert!(single.transition.is_none());
    single.set_speed(0.).unwrap();
    assert_eq!(single.advance(&rig, 0.1).unwrap().root_motion, Vec3::ZERO);
}

#[test]
fn interrupted_frozen_source_has_no_root_displacement_and_pause_is_zero() {
    let rig = skeleton();
    let mut animator = Animator::new(root_clip(&rig, 2.));
    animator.transition_to(root_clip(&rig, 6.), 1.).unwrap();
    animator.advance(&rig, 0.2).unwrap();
    animator.transition_to(root_clip(&rig, 10.), 1.).unwrap();
    assert!((animator.advance(&rig, 0.2).unwrap().root_motion.x - 0.2).abs() < 1e-5);
    animator.set_speed(0.).unwrap();
    assert_eq!(animator.advance(&rig, 0.1).unwrap().root_motion, Vec3::ZERO);
}

#[test]
fn completed_fade_does_not_admit_invalid_zero_weight_source() {
    let rig = skeleton();
    let mut tracks = vec![JointTrack::default(); 2];
    tracks[0].rotations = vec![
        QuatKey {
            time: 0.,
            value: Quat::IDENTITY,
        },
        QuatKey {
            time: 1.,
            value: -Quat::IDENTITY,
        },
    ];
    let mut modes = vec![TrackInterpolation::default(); 2];
    modes[0].rotation = Interpolation::CubicSpline;
    let mut tangents = vec![JointTangents::default(); 2];
    tangents[0].rotation = vec![[Vec4::ZERO; 2]; 2];
    let source = Arc::new(
        AnimationClip::new_with_tangents(
            "singular midpoint",
            1.,
            Playback::Clamp,
            tracks,
            modes,
            tangents,
            &rig,
        )
        .unwrap(),
    );
    assert!(source.try_sample(&rig, 0.5).is_err());
    let mut animator = Animator::new(source);
    let target = root_clip(&rig, 2.);
    animator.transition_to(target.clone(), 0.5).unwrap();
    animator.advance(&rig, 0.25).unwrap();
    let frame = animator.advance(&rig, 0.25).unwrap();
    assert_eq!(frame.pose, target.try_sample(&rig, 0.5).unwrap());
    assert_eq!(frame.transition_weight, 1.);
    assert!(animator.transition.is_none());
}

#[test]
fn animator_root_integral_uses_step_event_weights_and_substeps() {
    let rig = skeleton();
    let mut tracks = vec![JointTrack::default(); 2];
    tracks[0].translations = vec![
        Vec3Key {
            time: 0.,
            value: Vec3::ZERO,
        },
        Vec3Key {
            time: 0.25,
            value: Vec3::X * 2.,
        },
        Vec3Key {
            time: 0.75,
            value: Vec3::X * 5.,
        },
    ];
    let mut modes = vec![TrackInterpolation::default(); 2];
    modes[0].translation = Interpolation::Step;
    let target = Arc::new(
        AnimationClip::new_with_interpolation(
            "step motion",
            1.,
            Playback::Loop,
            tracks,
            modes,
            &rig,
        )
        .unwrap(),
    );
    let mut whole = Animator::new(root_clip(&rig, 0.));
    whole.transition_to(target, 1.).unwrap();
    let mut split = whole.clone();
    let full = whole.advance(&rig, 1.).unwrap().root_motion;
    let mut sum = Vec3::ZERO;
    for _ in 0..4 {
        sum += split.advance(&rig, 0.25).unwrap().root_motion;
    }
    assert!(full.abs_diff_eq(Vec3::X * 2.75, 1e-6));
    assert!(sum.abs_diff_eq(full, 1e-6));
}

#[test]
fn animator_root_integral_uses_nonzero_cubic_derivatives_in_seconds() {
    let rig = skeleton();
    let mut tracks = vec![JointTrack::default(); 2];
    tracks[0].translations = vec![
        Vec3Key {
            time: 0.,
            value: Vec3::ZERO,
        },
        Vec3Key {
            time: 2.,
            value: Vec3::X * 2.,
        },
    ];
    let mut modes = vec![TrackInterpolation::default(); 2];
    modes[0].translation = Interpolation::CubicSpline;
    let mut tangents = vec![JointTangents::default(); 2];
    tangents[0].translation = vec![[Vec3::ZERO, Vec3::X * 4.], [Vec3::X * (-2.), Vec3::ZERO]];
    let target = Arc::new(
        AnimationClip::new_with_tangents(
            "cubic motion",
            2.,
            Playback::Clamp,
            tracks,
            modes,
            tangents,
            &rig,
        )
        .unwrap(),
    );
    let mut animator = Animator::new(root_clip(&rig, 0.));
    animator.transition_to(target, 2.).unwrap();
    let first = animator.advance(&rig, 1.).unwrap().root_motion;
    let second = animator.advance(&rig, 1.).unwrap().root_motion;
    assert!(first.abs_diff_eq(Vec3::X * 0.5, 1e-6));
    assert!((first + second).abs_diff_eq(Vec3::ZERO, 1e-6));
}

#[test]
fn selected_motion_joint_updates_both_fade_curves_and_preserves_invalid_selection() {
    let rig = skeleton();
    let clip = |name: &str, speed: f32| {
        let mut tracks = vec![JointTrack::default(); 2];
        tracks[0].translations = vec![
            Vec3Key {
                time: 0.,
                value: Vec3::ZERO,
            },
            Vec3Key {
                time: 1.,
                value: Vec3::X * 100.,
            },
        ];
        tracks[1].translations = vec![
            Vec3Key {
                time: 0.,
                value: Vec3::Y,
            },
            Vec3Key {
                time: 1.,
                value: Vec3::Y * (1. + speed),
            },
        ];
        Arc::new(AnimationClip::new(name, 1., Playback::Loop, tracks, &rig).unwrap())
    };
    let mut animator = Animator::new(clip("a", 2.));
    animator.transition_to(clip("b", 6.), 1.).unwrap();
    animator.advance(&rig, 0.25).unwrap();
    animator.set_root_motion_joint(1).unwrap();
    let mut control = animator.clone();
    assert!(matches!(
        animator.set_root_motion_joint(2),
        Err(AnimationError::InvalidRootMotionJoint(2))
    ));
    assert_eq!(animator.root_motion_joint(), 1);
    assert!(Arc::ptr_eq(&animator.motion_curve, &control.motion_curve));
    let actual = animator.advance(&rig, 0.25).unwrap();
    let expected = control.advance(&rig, 0.25).unwrap();
    assert_eq!(actual.root_motion_joint, 1);
    assert!(actual.root_motion.abs_diff_eq(Vec3::Y * 0.875, 1e-6));
    assert_eq!(actual.root_motion, expected.root_motion);
    assert_eq!(actual.pose, expected.pose);
    animator.transition_to(clip("c", 10.), 0.).unwrap();
    assert_eq!(
        animator.advance(&rig, 0.25).unwrap().root_motion,
        Vec3::Y * 2.5
    );
}

#[test]
fn joint_translation_edit_moves_descendants_and_rejects_invalid_updates() {
    let rig = skeleton();
    let mut pose = rig.bind_pose();
    let rotation = Quat::from_rotation_z(0.4);
    pose.set_joint_rotation(0, rotation).unwrap();
    let translation = Vec3::new(0.1, 0.2, 0.3);
    pose.set_joint_translation(0, translation).unwrap();
    let child = rig.joints()[1].bind_local.translation;
    let expected = translation + rotation * child;
    let before = pose.skin_matrices(&rig).unwrap();
    let child_rest = rig.joints()[1]
        .inverse_bind
        .inverse()
        .transform_point3(Vec3::ZERO);
    assert!(
        before[1]
            .transform_point3(child_rest)
            .abs_diff_eq(expected, 1e-6)
    );
    assert_eq!(pose.local()[0].rotation, rotation);
    assert!(
        pose.set_joint_translation(0, Vec3::splat(f32::NAN))
            .is_err()
    );
    assert!(pose.set_joint_translation(999, Vec3::ZERO).is_err());
    assert_eq!(pose.skin_matrices(&rig).unwrap(), before);
}

fn skeleton() -> Skeleton {
    Skeleton::new(vec![
        Joint {
            name: Arc::from("root"),
            parent: None,
            bind_local: Transform::IDENTITY,
            inverse_bind: Mat4::IDENTITY,
        },
        Joint {
            name: Arc::from("hand"),
            parent: Some(0),
            bind_local: Transform {
                translation: Vec3::Y,
                ..Transform::IDENTITY
            },
            inverse_bind: Mat4::from_translation(-Vec3::Y),
        },
    ])
    .unwrap()
}

#[test]
fn constant_joint_proofs_cover_authored_nonbind_values_and_unused_cubic_tangents() {
    let rig = skeleton();
    let expected = Transform {
        translation: Vec3::new(7., 8., 9.),
        rotation: Quat::from_rotation_y(0.4),
        scale: Vec3::new(-2., 3., 4.),
    };
    for mode in [
        Interpolation::Linear,
        Interpolation::Step,
        Interpolation::CubicSpline,
    ] {
        let track = JointTrack {
            translations: vec![
                Vec3Key {
                    time: 0.,
                    value: expected.translation,
                },
                Vec3Key {
                    time: 1.,
                    value: expected.translation,
                },
            ],
            rotations: vec![
                QuatKey {
                    time: 0.,
                    value: expected.rotation,
                },
                QuatKey {
                    time: 1.,
                    value: expected.rotation,
                },
            ],
            scales: vec![
                Vec3Key {
                    time: 0.,
                    value: expected.scale,
                },
                Vec3Key {
                    time: 1.,
                    value: expected.scale,
                },
            ],
        };
        let mut tangents = JointTangents::default();
        if mode == Interpolation::CubicSpline {
            tangents.translation = vec![
                [Vec3::splat(f32::MAX), Vec3::ZERO],
                [Vec3::ZERO, Vec3::splat(f32::MAX)],
            ];
            tangents.rotation = vec![
                [Vec4::splat(f32::MAX), Vec4::ZERO],
                [Vec4::ZERO, Vec4::splat(f32::MAX)],
            ];
            tangents.scale = tangents.translation.clone();
        }
        let clip = AnimationClip::new_with_tangents(
            "constant",
            1.,
            Playback::Loop,
            vec![track, JointTrack::default()],
            vec![
                TrackInterpolation {
                    translation: mode,
                    rotation: mode,
                    scale: mode,
                },
                TrackInterpolation::default(),
            ],
            vec![tangents, JointTangents::default()],
            &rig,
        )
        .unwrap();
        assert_eq!(clip.constant_joint_transform(0), Some(expected));
        assert_eq!(
            clip.constant_joint_transform(1),
            Some(rig.joints()[1].bind_local)
        );
        assert_eq!(clip.constant_joint_transform(2), None);
        assert!(!clip.joint_uses_bind_pose(0));
        for time in [-1., 0., 0.1, 0.20000002, 0.25, 0.5, 0.75, 1., 2.] {
            let sample = clip.try_sample(&rig, time).unwrap();
            assert_eq!(sample.local()[0].translation, expected.translation);
            assert_eq!(sample.local()[0].scale, expected.scale);
            assert!(
                clip.try_sample(&rig, time).unwrap().local()[0]
                    .matrix()
                    .abs_diff_eq(expected.matrix(), 2e-6)
            );
        }
    }
}

#[test]
fn constant_joint_proofs_reject_hidden_cubic_motion_and_accept_linear_antipodes() {
    let rig = skeleton();
    let q = Quat::from_rotation_y(0.4);
    for mode in [Interpolation::Linear, Interpolation::Step] {
        let track = JointTrack {
            rotations: vec![
                QuatKey { time: 0., value: q },
                QuatKey {
                    time: 1.,
                    value: -q,
                },
            ],
            ..Default::default()
        };
        let clip = AnimationClip::new_with_interpolation(
            "antipodes",
            1.,
            Playback::Clamp,
            vec![track, JointTrack::default()],
            vec![
                TrackInterpolation {
                    rotation: mode,
                    ..Default::default()
                },
                TrackInterpolation::default(),
            ],
            &rig,
        )
        .unwrap();
        assert_eq!(clip.constant_joint_transform(0).unwrap().rotation, q);
    }
    for channel in 0..4 {
        let mut track = JointTrack {
            translations: vec![
                Vec3Key {
                    time: 0.,
                    value: Vec3::ZERO,
                },
                Vec3Key {
                    time: 1.,
                    value: Vec3::ZERO,
                },
            ],
            rotations: vec![
                QuatKey { time: 0., value: q },
                QuatKey { time: 1., value: q },
            ],
            scales: vec![
                Vec3Key {
                    time: 0.,
                    value: Vec3::ONE,
                },
                Vec3Key {
                    time: 1.,
                    value: Vec3::ONE,
                },
            ],
        };
        let mut tangents = JointTangents {
            translation: vec![[Vec3::ZERO; 2]; 2],
            rotation: vec![[Vec4::ZERO; 2]; 2],
            scale: vec![[Vec3::ZERO; 2]; 2],
        };
        match channel {
            0 => tangents.translation[0][1] = Vec3::X,
            1 => tangents.rotation[0][1] = Vec4::X,
            2 => tangents.scale[0][1] = Vec3::X,
            _ => track.rotations[1].value = -q,
        }
        let clip = AnimationClip::new_with_tangents(
            "hidden motion",
            1.,
            Playback::Clamp,
            vec![track, JointTrack::default()],
            vec![
                TrackInterpolation {
                    translation: Interpolation::CubicSpline,
                    rotation: Interpolation::CubicSpline,
                    scale: Interpolation::CubicSpline,
                },
                TrackInterpolation::default(),
            ],
            vec![tangents, JointTangents::default()],
            &rig,
        )
        .unwrap();
        assert!(clip.constant_joint_transform(0).is_none());
        if channel == 0 {
            assert!(clip.try_sample(&rig, 0.5).unwrap().local()[0].translation.x > 0.1);
        }
        if channel == 3 {
            assert!(clip.try_sample(&rig, 0.5).is_err());
        }
    }
}

#[test]
fn mirrored_bind_transforms_keep_signed_scale_through_hierarchy_and_palette() {
    for bits in 0_u32..8 {
        let scale = Vec3::new(
            if bits & 1 != 0 { -2.0 } else { 2.0 },
            if bits & 2 != 0 { -3.0 } else { 3.0 },
            if bits & 4 != 0 { -4.0 } else { 4.0 },
        );
        let root = Transform {
            scale,
            rotation: Quat::from_rotation_y(0.4),
            translation: Vec3::new(1.0, 2.0, 3.0),
        };
        let child = Transform {
            translation: Vec3::Y,
            ..Transform::IDENTITY
        };
        let skeleton = Skeleton::new(vec![
            Joint {
                name: "root".into(),
                parent: None,
                bind_local: root,
                inverse_bind: Mat4::IDENTITY,
            },
            Joint {
                name: "child".into(),
                parent: Some(0),
                bind_local: child,
                inverse_bind: Mat4::IDENTITY,
            },
        ])
        .unwrap();
        let palette = skeleton.bind_pose().skin_matrices(&skeleton).unwrap();
        assert!(palette[0].abs_diff_eq(root.matrix(), 1e-6));
        assert!(palette[1].abs_diff_eq(root.matrix() * child.matrix(), 1e-6));
        assert_eq!(
            palette[0].determinant().is_sign_negative(),
            bits.count_ones() % 2 == 1
        );
    }
}

#[test]
fn signed_scale_channels_and_blends_reject_only_singular_samples() {
    let skeleton = skeleton();
    let tracks = vec![
        JointTrack {
            scales: vec![
                Vec3Key {
                    time: 0.0,
                    value: Vec3::ONE,
                },
                Vec3Key {
                    time: 2.0,
                    value: Vec3::new(-1.0, 1.0, 1.0),
                },
            ],
            ..JointTrack::default()
        },
        JointTrack::default(),
    ];
    let linear =
        AnimationClip::new("mirror", 2.0, Playback::Clamp, tracks.clone(), &skeleton).unwrap();
    assert_eq!(
        linear.try_sample(&skeleton, 1.5).unwrap().local()[0]
            .scale
            .x,
        -0.5
    );
    assert!(linear.try_sample(&skeleton, 1.0).is_err());
    let a = linear.try_sample(&skeleton, 0.0).unwrap();
    let b = linear.try_sample(&skeleton, 2.0).unwrap();
    assert!(Pose::blend(&a, &b, 0.5).is_err());
    assert_eq!(Pose::blend(&a, &b, 1.0).unwrap(), b);
    let mut animator = Animator::new(Arc::new(linear));
    animator.advance(&skeleton, 0.5).unwrap();
    let before = animator.time;
    assert!(animator.advance(&skeleton, 0.5).is_err());
    assert_eq!(animator.time, before);
    let step = AnimationClip::new_with_interpolation(
        "step mirror",
        2.0,
        Playback::Clamp,
        tracks.clone(),
        vec![
            TrackInterpolation {
                scale: Interpolation::Step,
                ..TrackInterpolation::default()
            },
            TrackInterpolation::default(),
        ],
        &skeleton,
    )
    .unwrap();
    assert_eq!(
        step.try_sample(&skeleton, 1.0).unwrap().local()[0].scale,
        Vec3::ONE
    );
    assert_eq!(
        step.try_sample(&skeleton, 2.0).unwrap().local()[0].scale,
        Vec3::new(-1.0, 1.0, 1.0)
    );
    let cubic = AnimationClip::new_with_tangents(
        "cubic mirror",
        2.0,
        Playback::Clamp,
        tracks,
        vec![
            TrackInterpolation {
                scale: Interpolation::CubicSpline,
                ..TrackInterpolation::default()
            },
            TrackInterpolation::default(),
        ],
        vec![
            JointTangents {
                scale: vec![[Vec3::ZERO; 2]; 2],
                ..JointTangents::default()
            },
            JointTangents::default(),
        ],
        &skeleton,
    )
    .unwrap();
    assert!(cubic.try_sample(&skeleton, 1.0).is_err());
    assert!(cubic.try_sample(&skeleton, 1.5).unwrap().local()[0].scale.x < 0.0);
    let mirrored = AnimationClip::new(
        "negative endpoints",
        2.0,
        Playback::Clamp,
        vec![
            JointTrack {
                scales: vec![
                    Vec3Key {
                        time: 0.0,
                        value: -Vec3::ONE,
                    },
                    Vec3Key {
                        time: 2.0,
                        value: -Vec3::ONE * 2.0,
                    },
                ],
                ..JointTrack::default()
            },
            JointTrack::default(),
        ],
        &skeleton,
    )
    .unwrap();
    assert_eq!(
        mirrored.try_sample(&skeleton, 1.0).unwrap().local()[0].scale,
        Vec3::splat(-1.5)
    );
}

#[test]
fn hierarchy_and_inverse_bind_produce_skin_matrices() {
    let skeleton = skeleton();
    let matrices = skeleton.bind_pose().skin_matrices(&skeleton).unwrap();
    assert_eq!(matrices.len(), 2);
    assert!(
        matrices
            .iter()
            .all(|matrix| matrix.abs_diff_eq(Mat4::IDENTITY, 1.0e-6))
    );
}

#[test]
fn cubic_derivatives_use_segment_seconds_and_normalize_rotation() {
    let skeleton = skeleton();
    let end = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    let tracks = vec![
        JointTrack {
            translations: vec![
                Vec3Key {
                    time: 1.0,
                    value: Vec3::ZERO,
                },
                Vec3Key {
                    time: 3.0,
                    value: Vec3::X * 2.0,
                },
            ],
            rotations: vec![
                QuatKey {
                    time: 1.0,
                    value: Quat::IDENTITY,
                },
                QuatKey {
                    time: 3.0,
                    value: end,
                },
            ],
            scales: vec![
                Vec3Key {
                    time: 1.0,
                    value: Vec3::ONE,
                },
                Vec3Key {
                    time: 3.0,
                    value: Vec3::splat(3.0),
                },
            ],
        },
        JointTrack::default(),
    ];
    let mode = TrackInterpolation {
        translation: Interpolation::CubicSpline,
        rotation: Interpolation::CubicSpline,
        scale: Interpolation::Linear,
    };
    let tangents = vec![
        JointTangents {
            translation: vec![[Vec3::ZERO, Vec3::X * 4.0], [Vec3::X * -2.0, Vec3::ZERO]],
            rotation: vec![[Vec4::ZERO, Vec4::Z * 2.0], [Vec4::ZERO; 2]],
            scale: vec![],
        },
        JointTangents::default(),
    ];
    let clip = AnimationClip::new_with_tangents(
        "cubic",
        4.0,
        Playback::Clamp,
        tracks.clone(),
        vec![mode, TrackInterpolation::default()],
        tangents.clone(),
        &skeleton,
    )
    .unwrap();
    assert_eq!(
        clip.try_sample(&skeleton, 0.0).unwrap().local()[0].translation,
        Vec3::ZERO
    );
    assert!(
        clip.try_sample(&skeleton, 1.5).unwrap().local()[0]
            .translation
            .abs_diff_eq(Vec3::X * 1.625, 1e-6)
    );
    let midpoint = clip.try_sample(&skeleton, 2.0).unwrap();
    assert!(
        midpoint.local()[0]
            .translation
            .abs_diff_eq(Vec3::X * 2.5, 1e-6)
    );
    assert_eq!(midpoint.local()[0].scale, Vec3::splat(2.0));
    let expected = (Vec4::from_array(Quat::IDENTITY.to_array()) * 0.5
        + Vec4::from_array(end.to_array()) * 0.5
        + Vec4::Z * 0.5)
        .normalize();
    assert!(Vec4::from_array(midpoint.local()[0].rotation.to_array()).abs_diff_eq(expected, 1e-6));
    assert!(midpoint.local()[0].rotation.is_normalized());
    assert_eq!(
        clip.try_sample(&skeleton, 3.0).unwrap().local()[0].rotation,
        end
    );
    assert_eq!(
        clip.try_sample(&skeleton, 10.0).unwrap().local()[0].translation,
        Vec3::X * 2.0
    );
    assert!(clip.try_sample(&skeleton, f32::NAN).is_err());
    assert!(
        AnimationClip::new_with_interpolation(
            "missing",
            4.0,
            Playback::Clamp,
            tracks.clone(),
            vec![mode, TrackInterpolation::default()],
            &skeleton
        )
        .is_err()
    );
    let mut broken = tangents.clone();
    broken[0].rotation[0][1].x = f32::INFINITY;
    assert!(
        AnimationClip::new_with_tangents(
            "bad",
            4.0,
            Playback::Clamp,
            tracks.clone(),
            vec![mode, TrackInterpolation::default()],
            broken,
            &skeleton
        )
        .is_err()
    );
    let mut short = tracks;
    short[0].translations.truncate(1);
    assert!(
        AnimationClip::new_with_tangents(
            "short",
            4.0,
            Playback::Clamp,
            short,
            vec![mode, TrackInterpolation::default()],
            tangents,
            &skeleton
        )
        .is_err()
    );
}

#[test]
fn invalid_cubic_pose_keeps_animator_clock_and_transition() {
    let skeleton = skeleton();
    let cubic = AnimationClip::new_with_tangents(
        "zero quaternion",
        2.0,
        Playback::Clamp,
        vec![
            JointTrack {
                rotations: vec![
                    QuatKey {
                        time: 0.0,
                        value: Quat::IDENTITY,
                    },
                    QuatKey {
                        time: 2.0,
                        value: -Quat::IDENTITY,
                    },
                ],
                ..JointTrack::default()
            },
            JointTrack::default(),
        ],
        vec![
            TrackInterpolation {
                rotation: Interpolation::CubicSpline,
                ..TrackInterpolation::default()
            },
            TrackInterpolation::default(),
        ],
        vec![
            JointTangents {
                rotation: vec![[Vec4::ZERO; 2]; 2],
                ..JointTangents::default()
            },
            JointTangents::default(),
        ],
        &skeleton,
    )
    .unwrap();
    assert!(matches!(
        cubic.try_sample(&skeleton, 1.0),
        Err(AnimationError::InvalidPose(0))
    ));
    let valid = Arc::new(
        AnimationClip::new(
            "bind",
            2.0,
            Playback::Clamp,
            vec![JointTrack::default(); 2],
            &skeleton,
        )
        .unwrap(),
    );
    let mut looped = cubic.clone();
    looped.playback = Playback::Loop;
    let looped = Arc::new(looped);
    let mut loop_player = Animator::new(looped.clone());
    loop_player.time = 2_f64.next_down();
    assert!(loop_player.advance_wall(&skeleton, 0.).is_ok());
    let loop_clock = loop_player.time;
    assert!(matches!(
        loop_player.advance_wall(&skeleton, 1.),
        Err(AnimationError::InvalidPose(0))
    ));
    assert_eq!(loop_player.time, loop_clock);
    assert!(Arc::ptr_eq(&loop_player.current, &looped));
    assert!(loop_player.transition.is_none());
    assert!(loop_player.advance_wall(&skeleton, 0.25).is_ok());

    for duration in [0., 2.] {
        let mut player = Animator::new(valid.clone());
        player.transition_to(valid.clone(), 2.).unwrap();
        player.advance_wall(&skeleton, 0.25).unwrap();
        let before = player.clone();
        assert!(matches!(
            player.transition_to_at_phase(Arc::new(cubic.clone()), duration, 0.5),
            Err(AnimationError::InvalidPose(0))
        ));
        assert!(Arc::ptr_eq(&player.current, &before.current));
        assert_eq!(player.time, before.time);
        let after_transition = player.transition.as_ref().unwrap();
        let before_transition = before.transition.as_ref().unwrap();
        assert!(Arc::ptr_eq(
            &after_transition.source,
            &before_transition.source
        ));
        assert_eq!(after_transition.source_time, before_transition.source_time);
        assert_eq!(after_transition.elapsed, before_transition.elapsed);
        assert_eq!(after_transition.duration, before_transition.duration);
        let recovered = player.advance_wall(&skeleton, 0.25).unwrap();
        let expected = before.clone().advance_wall(&skeleton, 0.25).unwrap();
        assert_eq!(recovered.skin_matrices, expected.skin_matrices);
        assert_eq!(recovered.root_motion, expected.root_motion);
    }
    let mut animator = Animator::new(valid);
    animator.transition_to(Arc::new(cubic), 2.0).unwrap();
    let before = animator.clone();
    assert!(animator.advance(&skeleton, 1.0).is_err());
    assert_eq!(animator.time, before.time);
    assert_eq!(
        animator.transition.as_ref().unwrap().elapsed,
        before.transition.as_ref().unwrap().elapsed
    );
    assert_eq!(
        animator.transition.as_ref().unwrap().source_time,
        before.transition.as_ref().unwrap().source_time
    );
    assert!(animator.advance(&skeleton, 0.25).is_ok());
}

#[test]
fn cubic_scale_overshoot_is_sampled_and_invalid_zero_scale_is_rejected() {
    let skeleton = skeleton();
    let clip = AnimationClip::new_with_tangents(
        "scale",
        2.0,
        Playback::Clamp,
        vec![
            JointTrack {
                scales: vec![
                    Vec3Key {
                        time: 0.0,
                        value: Vec3::ONE,
                    },
                    Vec3Key {
                        time: 1.0,
                        value: Vec3::ONE,
                    },
                ],
                ..JointTrack::default()
            },
            JointTrack::default(),
        ],
        vec![
            TrackInterpolation {
                scale: Interpolation::CubicSpline,
                ..TrackInterpolation::default()
            },
            TrackInterpolation::default(),
        ],
        vec![
            JointTangents {
                scale: vec![
                    [Vec3::ZERO, Vec3::splat(-4.0)],
                    [Vec3::splat(4.0), Vec3::ZERO],
                ],
                ..JointTangents::default()
            },
            JointTangents::default(),
        ],
        &skeleton,
    )
    .unwrap();
    assert!(
        clip.try_sample(&skeleton, 0.25).unwrap().local()[0]
            .scale
            .abs_diff_eq(Vec3::splat(0.25), 1e-6)
    );
    assert!(matches!(
        clip.try_sample(&skeleton, 0.5),
        Err(AnimationError::InvalidPose(0))
    ));
    assert_eq!(
        clip.try_sample(&skeleton, 1.0).unwrap().local()[0].scale,
        Vec3::ONE
    );
}

#[test]
fn step_channels_hold_values_at_boundaries_without_changing_linear_channels() {
    let skeleton = skeleton();
    let rotation = Quat::from_rotation_z(1.2);
    let track = JointTrack {
        translations: vec![
            Vec3Key {
                time: 0.2,
                value: Vec3::ZERO,
            },
            Vec3Key {
                time: 0.5,
                value: Vec3::X,
            },
            Vec3Key {
                time: 1.0,
                value: Vec3::Y,
            },
        ],
        rotations: vec![
            QuatKey {
                time: 0.2,
                value: Quat::IDENTITY,
            },
            QuatKey {
                time: 0.5,
                value: rotation,
            },
        ],
        scales: vec![
            Vec3Key {
                time: 0.0,
                value: Vec3::ONE,
            },
            Vec3Key {
                time: 1.0,
                value: Vec3::splat(3.0),
            },
        ],
    };
    let tracks = vec![track, JointTrack::default()];
    let modes = vec![
        TrackInterpolation {
            translation: Interpolation::Step,
            rotation: Interpolation::Step,
            scale: Interpolation::Linear,
        },
        TrackInterpolation::default(),
    ];
    let clip = AnimationClip::new_with_interpolation(
        "mixed",
        2.0,
        Playback::Clamp,
        tracks.clone(),
        modes.clone(),
        &skeleton,
    )
    .unwrap();
    for (time, expected) in [
        (-1.0, Vec3::ZERO),
        (0.2, Vec3::ZERO),
        (0.5_f32.next_down(), Vec3::ZERO),
        (0.5, Vec3::X),
        (0.5_f32.next_up(), Vec3::X),
        (1.0_f32.next_down(), Vec3::X),
        (1.0, Vec3::Y),
        (3.0, Vec3::Y),
    ] {
        let pose = clip.sample(&skeleton, time);
        assert_eq!(pose.local()[0].translation, expected, "time={time}");
        assert_eq!(pose.local()[1], skeleton.joints()[1].bind_local);
    }
    assert_eq!(
        clip.sample(&skeleton, 0.5_f32.next_down()).local()[0].rotation,
        Quat::IDENTITY
    );
    assert_eq!(clip.sample(&skeleton, 0.5).local()[0].rotation, rotation);
    assert_eq!(
        clip.sample(&skeleton, 0.5).local()[0].scale,
        Vec3::splat(2.0)
    );
    let looped = AnimationClip::new_with_interpolation(
        "mixed",
        2.0,
        Playback::Loop,
        tracks.clone(),
        modes,
        &skeleton,
    )
    .unwrap();
    assert_eq!(
        looped.sample(&skeleton, 2.0).local()[0].translation,
        Vec3::ZERO
    );
    assert_eq!(
        looped.sample(&skeleton, -1.5).local()[0].translation,
        Vec3::X
    );
    assert!(matches!(
        AnimationClip::new_with_interpolation(
            "bad",
            2.0,
            Playback::Loop,
            tracks,
            vec![],
            &skeleton
        ),
        Err(AnimationError::InterpolationCountMismatch {
            expected: 2,
            actual: 0
        })
    ));
}

#[test]
fn clip_interpolates_loops_and_uses_bind_fallbacks() {
    let skeleton = skeleton();
    let clip = AnimationClip::new(
        "wave",
        2.0,
        Playback::Loop,
        vec![
            JointTrack {
                translations: vec![
                    Vec3Key {
                        time: 0.0,
                        value: Vec3::ZERO,
                    },
                    Vec3Key {
                        time: 2.0,
                        value: Vec3::new(2.0, 0.0, 0.0),
                    },
                ],
                ..JointTrack::default()
            },
            JointTrack::default(),
        ],
        &skeleton,
    )
    .unwrap();
    let pose = clip.sample(&skeleton, 1.0);
    assert!(pose.local()[0].translation.abs_diff_eq(Vec3::X, 1.0e-6));
    assert!(pose.local()[1].translation.abs_diff_eq(Vec3::Y, 1.0e-6));
    let looped = clip.sample(&skeleton, 3.0);
    assert!(looped.local()[0].translation.abs_diff_eq(Vec3::X, 1.0e-6));
}

#[test]
fn joint_override_rejects_invalid_rotation_without_mutation() {
    let mut pose = Pose {
        rig: Arc::from([skeleton().joints()[0].clone()]),
        local: vec![Transform::IDENTITY],
    };
    for invalid in [
        Quat::from_xyzw(0., 0., 0., 0.),
        Quat::from_xyzw(f32::NAN, 0., 0., 1.),
    ] {
        assert!(pose.set_joint_rotation(0, invalid).is_err());
        assert_eq!(pose.local[0], Transform::IDENTITY);
    }
    assert!(pose.set_joint_rotation(1, Quat::IDENTITY).is_err());
    let rotation = Quat::from_rotation_x(0.7);
    pose.set_joint_rotation(0, rotation).unwrap();
    assert_eq!(pose.local[0].rotation, rotation);
    assert_eq!(pose.local[0].translation, Vec3::ZERO);
    assert_eq!(pose.local[0].scale, Vec3::ONE);
}
#[test]
fn cubic_rotation_underflow_recovers_nonzero_curve_without_admitting_true_zero() {
    let keys = [
        QuatKey {
            time: 0.,
            value: Quat::IDENTITY,
        },
        QuatKey {
            time: 1.,
            value: -Quat::IDENTITY,
        },
    ];
    // At the midpoint, endpoint terms cancel. The surviving tangent term
    // is one eighth of the smallest positive f32, nonzero in f64.
    let tangents = [
        [Vec4::ZERO, Vec4::new(f32::from_bits(1), 0., 0., 0.)],
        [Vec4::ZERO, Vec4::ZERO],
    ];
    let q = sample_quat(
        &keys,
        0.5,
        Quat::IDENTITY,
        Interpolation::CubicSpline,
        &tangents,
    );
    assert!(q.is_finite() && q.is_normalized());
    assert!((q * Vec3::Y).abs_diff_eq(-Vec3::Y, 1e-6));
    assert!(
        !sample_quat(
            &keys,
            0.5,
            Quat::IDENTITY,
            Interpolation::CubicSpline,
            &[[Vec4::ZERO; 2]; 2]
        )
        .is_finite()
    );
}
#[test]
fn cubic_rotation_recovers_large_amplitude_and_cancellation_but_rejects_zero() {
    let keys = [
        QuatKey {
            time: 0.,
            value: Quat::IDENTITY,
        },
        QuatKey {
            time: 16.,
            value: Quat::IDENTITY,
        },
    ];
    let large = Vec4::new(3e38, 0., 0., 0.);
    let cancelling = [[Vec4::ZERO, large], [large, Vec4::ZERO]];
    assert_eq!(
        sample_quat(
            &keys,
            8.,
            Quat::IDENTITY,
            Interpolation::CubicSpline,
            &cancelling
        ),
        Quat::IDENTITY
    );
    let amplitude = [[Vec4::ZERO, large], [Vec4::ZERO, Vec4::ZERO]];
    let q = sample_quat(
        &keys,
        8.,
        Quat::IDENTITY,
        Interpolation::CubicSpline,
        &amplitude,
    );
    assert!(q.is_finite() && q.is_normalized());
    assert!((q * Vec3::Y).abs_diff_eq(-Vec3::Y, 1e-6));
    let opposite = [
        keys[0],
        QuatKey {
            time: 16.,
            value: -Quat::IDENTITY,
        },
    ];
    assert!(
        !sample_quat(
            &opposite,
            8.,
            Quat::IDENTITY,
            Interpolation::CubicSpline,
            &[[Vec4::ZERO; 2]; 2]
        )
        .is_finite()
    );
}
#[test]
fn cubic_vector_cancellation_recovers_finite_value_but_keeps_real_overflow() {
    let keys = [
        Vec3Key {
            time: 0.,
            value: Vec3::splat(2e38),
        },
        Vec3Key {
            time: 4.,
            value: Vec3::splat(2e38),
        },
    ];
    let tangents = [
        [Vec3::ZERO, Vec3::splat(3e38)],
        [Vec3::splat(3e38), Vec3::ZERO],
    ];
    let sampled = sample_vec3(&keys, 2., Vec3::ZERO, Interpolation::CubicSpline, &tangents);
    assert_eq!(sampled, Vec3::splat(2e38));
    assert_eq!(
        sample_vec3(&keys, 0., Vec3::ZERO, Interpolation::CubicSpline, &tangents),
        keys[0].value
    );
    assert_eq!(
        sample_vec3(&keys, 4., Vec3::ZERO, Interpolation::CubicSpline, &tangents),
        keys[1].value
    );
    let overflow = [
        [Vec3::ZERO, Vec3::splat(3e38)],
        [Vec3::splat(-3e38), Vec3::ZERO],
    ];
    assert!(!sample_vec3(&keys, 2., Vec3::ZERO, Interpolation::CubicSpline, &overflow).is_finite());
}
#[test]
fn large_finite_translation_blends_and_samples_without_spurious_overflow() {
    let rig = Skeleton::new(vec![skeleton().joints()[0].clone()]).unwrap();
    let mut a = rig.bind_pose();
    let mut b = a.clone();
    a.local[0].translation = Vec3::splat(2e38);
    b.local[0].translation = Vec3::splat(-2e38);
    let keys = [
        Vec3Key {
            time: 0.,
            value: a.local[0].translation,
        },
        Vec3Key {
            time: 1.,
            value: b.local[0].translation,
        },
    ];
    for weight in [0., 0.25, 0.5, 0.75, 1.] {
        let blended = Pose::blend(&a, &b, weight).unwrap();
        let sampled = sample_vec3(&keys, weight, Vec3::ZERO, Interpolation::Linear, &[]);
        assert_eq!(sampled, blended.local[0].translation);
        assert!(blended.skin_matrices(&rig).unwrap()[0].is_finite());
    }
    assert_eq!(
        Pose::blend(&a, &b, 0.).unwrap().local[0].translation,
        a.local[0].translation
    );
    assert_eq!(
        Pose::blend(&a, &b, 1.).unwrap().local[0].translation,
        b.local[0].translation
    );
    assert_eq!(
        Pose::blend(&a, &b, 0.5).unwrap().local[0].translation,
        Vec3::ZERO
    );
}
#[test]
fn pose_blend_uses_shortest_rotation_path() {
    let a = Pose {
        rig: Arc::from([skeleton().joints()[0].clone()]),
        local: vec![Transform::IDENTITY],
    };
    let b = Pose {
        rig: a.rig.clone(),
        local: vec![Transform {
            translation: Vec3::new(2.0, 0.0, 0.0),
            rotation: Quat::from_rotation_y(std::f32::consts::PI),
            scale: Vec3::splat(2.0),
        }],
    };
    let pose = Pose::blend(&a, &b, 0.5).unwrap();
    assert!(pose.local[0].translation.abs_diff_eq(Vec3::X, 1.0e-6));
    assert!(pose.local[0].scale.abs_diff_eq(Vec3::splat(1.5), 1.0e-6));
    assert!(pose.local[0].rotation.is_normalized());
}

#[test]
fn malformed_hierarchy_and_tracks_are_rejected() {
    let invalid = Skeleton::new(vec![Joint {
        name: Arc::from("cycle"),
        parent: Some(0),
        bind_local: Transform::IDENTITY,
        inverse_bind: Mat4::IDENTITY,
    }]);
    assert!(matches!(invalid, Err(AnimationError::InvalidParent { .. })));

    let skeleton = skeleton();
    let clip = AnimationClip::new(
        "bad",
        1.0,
        Playback::Clamp,
        vec![
            JointTrack {
                translations: vec![
                    Vec3Key {
                        time: 0.8,
                        value: Vec3::ZERO,
                    },
                    Vec3Key {
                        time: 0.2,
                        value: Vec3::ONE,
                    },
                ],
                ..JointTrack::default()
            },
            JointTrack::default(),
        ],
        &skeleton,
    );
    assert!(matches!(clip, Err(AnimationError::InvalidTrack { .. })));
}

fn root_clip(skeleton: &Skeleton, end: f32) -> Arc<AnimationClip> {
    Arc::new(
        AnimationClip::new(
            "walk",
            1.0,
            Playback::Loop,
            vec![
                JointTrack {
                    translations: vec![
                        Vec3Key {
                            time: 0.0,
                            value: Vec3::ZERO,
                        },
                        Vec3Key {
                            time: 1.0,
                            value: Vec3::new(end, 0.0, 0.0),
                        },
                    ],
                    ..JointTrack::default()
                },
                JointTrack::default(),
            ],
            skeleton,
        )
        .unwrap(),
    )
}

#[test]
fn extracted_root_rotation_returns_to_bind_and_rebuilds_the_hierarchical_palette() {
    let mut joints = skeleton().joints().to_vec();
    joints[1].bind_local.rotation = Quat::from_rotation_x(0.23);
    joints[1].bind_local.scale = Vec3::new(-1., 2., 1.);
    joints[1].inverse_bind = joints[1].bind_local.matrix().inverse();
    let rig = Skeleton::new(joints).unwrap();
    let mut pose = rig.bind_pose();
    pose.local[0].translation = Vec3::X * 7.;
    pose.local[0].rotation = Quat::from_rotation_y(0.4);
    pose.local[1] = Transform {
        translation: Vec3::new(3., 4., 5.),
        rotation: Quat::from_rotation_z(0.7),
        scale: Vec3::new(-2., 3., 4.),
    };
    let original = AnimatorFrame {
        properties: Default::default(),
        skin_matrices: pose.skin_matrices(&rig).unwrap(),
        pose,
        root_motion: Vec3::new(0.1, 0.2, 0.3),
        root_motion_joint: 1,
        transition_weight: 0.5,
    };
    let output = original.clone().without_root_rotation(&rig).unwrap();
    assert_eq!(output.pose.local()[0], original.pose.local()[0]);
    assert_eq!(
        output.pose.local()[1].translation,
        original.pose.local()[1].translation
    );
    assert_eq!(output.pose.local()[1].scale, original.pose.local()[1].scale);
    assert_eq!(
        output.pose.local()[1].rotation,
        rig.joints()[1].bind_local.rotation
    );
    assert_eq!(output.root_motion, original.root_motion);
    assert_eq!(output.transition_weight, original.transition_weight);
    let expected_global = original.pose.local()[0].matrix()
        * Mat4::from_scale_rotation_translation(
            Vec3::new(-2., 3., 4.),
            Quat::from_rotation_x(0.23),
            Vec3::new(3., 4., 5.),
        );
    assert!(
        output.skin_matrices[1].abs_diff_eq(expected_global * rig.joints()[1].inverse_bind, 1e-6)
    );
    assert!(
        (output.skin_matrices[1].transform_point3(Vec3::X)
            - original.skin_matrices[1].transform_point3(Vec3::X))
        .length()
            > 0.1
    );
    let (first, displacement) = output
        .clone()
        .into_in_place_translation(&rig, [true, false, true])
        .unwrap();
    let (second, other_displacement) = original
        .clone()
        .into_in_place_translation(&rig, [true, false, true])
        .unwrap();
    let second = second.without_root_rotation(&rig).unwrap();
    assert_eq!(first.pose, second.pose);
    assert_eq!(first.skin_matrices, second.skin_matrices);
    assert_eq!(displacement, other_displacement);
    assert_eq!(displacement, Vec3::new(0.1, 0., 0.3));
    let mut foreign = rig.joints().to_vec();
    foreign[1].name = Arc::from("different");
    assert_eq!(
        original
            .clone()
            .without_root_rotation(&Skeleton::new(foreign).unwrap())
            .unwrap_err(),
        AnimationError::SkeletonMismatch
    );
    let mut invalid = original.clone();
    invalid.root_motion_joint = u16::MAX;
    assert_eq!(
        invalid.without_root_rotation(&rig).unwrap_err(),
        AnimationError::InvalidRootMotionJoint(u16::MAX)
    );
    let mut invalid = original.clone();
    invalid.pose.local[1].rotation = Quat::from_array([f32::NAN; 4]);
    assert_eq!(
        invalid.without_root_rotation(&rig).unwrap_err(),
        AnimationError::InvalidPose(1)
    );
    let mut invalid = original.clone();
    invalid.pose.local[0].translation = Vec3::splat(f32::INFINITY);
    assert_eq!(
        invalid.without_root_rotation(&rig).unwrap_err(),
        AnimationError::InvalidPose(0)
    );
    assert!(
        original.pose.local[1]
            .rotation
            .abs_diff_eq(Quat::from_rotation_z(0.7), 1e-6)
    );
}
#[test]
fn extraction_masks_selected_child_axes_and_preserves_other_channels() {
    let rig = skeleton();
    let mut pose = rig.bind_pose();
    pose.local[0].translation = Vec3::X * 7.0;
    pose.local[1] = Transform {
        translation: Vec3::new(3.0, 4.0, 5.0),
        rotation: Quat::from_rotation_z(0.4),
        scale: Vec3::new(-2.0, 3.0, 4.0),
    };
    let original = pose.clone();
    let frame = AnimatorFrame {
        properties: Default::default(),
        skin_matrices: pose.skin_matrices(&rig).unwrap(),
        pose,
        root_motion: Vec3::new(0.1, 0.2, 0.3),
        root_motion_joint: 1,
        transition_weight: 0.5,
    };
    let (frame, motion) = frame
        .into_in_place_translation(&rig, [false, true, false])
        .unwrap();
    assert_eq!(motion, Vec3::Y * 0.2);
    assert_eq!(frame.root_motion, Vec3::new(0.1, 0.0, 0.3));
    assert_eq!(frame.pose.local()[0], original.local()[0]);
    assert_eq!(frame.pose.local()[1].translation, Vec3::new(3.0, 1.0, 5.0));
    assert_eq!(frame.pose.local()[1].rotation, original.local()[1].rotation);
    assert_eq!(frame.pose.local()[1].scale, original.local()[1].scale);
    assert_eq!(frame.transition_weight, 0.5);
    assert_eq!(frame.skin_matrices, frame.pose.skin_matrices(&rig).unwrap());
    let mut invalid = frame;
    invalid.root_motion = Vec3::splat(f32::INFINITY);
    assert!(invalid.into_in_place_translation(&rig, [true; 3]).is_err());
}

#[test]
fn extracted_translation_is_removed_from_pose_palette_and_consumed_once() {
    let rig = skeleton();
    let mut animator = Animator::new(root_clip(&rig, 2.0));
    let sampled = animator.advance(&rig, 0.75).unwrap();
    let original = sampled.clone();
    let (in_place, motion) = sampled
        .into_in_place_translation(&rig, [true, false, true])
        .unwrap();
    assert_eq!(motion, Vec3::X * 1.5);
    assert_eq!(in_place.root_motion, Vec3::ZERO);
    assert_eq!(
        in_place.pose.local()[0].translation.x,
        rig.joints()[0].bind_local.translation.x
    );
    assert_eq!(in_place.pose.local()[1], original.pose.local()[1]);
    assert_eq!(
        in_place.pose.local()[0].rotation,
        original.pose.local()[0].rotation
    );
    assert_eq!(
        in_place.skin_matrices,
        in_place.pose.skin_matrices(&rig).unwrap()
    );
    assert_ne!(in_place.skin_matrices, original.skin_matrices);
    let (again, twice) = in_place
        .clone()
        .into_in_place_translation(&rig, [true, false, true])
        .unwrap();
    assert_eq!(twice, Vec3::ZERO);
    assert_eq!(again.pose, in_place.pose);
    let (wrapped, motion) = animator
        .advance(&rig, 0.5)
        .unwrap()
        .into_in_place_translation(&rig, [true, false, true])
        .unwrap();
    assert_eq!(motion, Vec3::X);
    assert_eq!(
        wrapped.pose.local()[0].translation.x,
        rig.joints()[0].bind_local.translation.x
    );
    let (unchanged, zero) = original
        .clone()
        .into_in_place_translation(&rig, [false; 3])
        .unwrap();
    assert_eq!(zero, Vec3::ZERO);
    assert_eq!(unchanged.pose, original.pose);
    assert_eq!(unchanged.root_motion, original.root_motion);
    let mut joints = rig.joints().to_vec();
    joints[0].name = "foreign".into();
    let foreign = Skeleton::new(joints).unwrap();
    assert!(matches!(
        original.into_in_place_translation(&foreign, [true; 3]),
        Err(AnimationError::SkeletonMismatch)
    ));
}

#[test]
fn clock_sampling_does_not_wrap_rounded_phase_before_loop_seam() {
    let rig = skeleton();
    let clip = root_clip(&rig, 2.);
    let before = 1_f64.next_down();
    assert_eq!(before as f32, 1.);
    assert_eq!(
        clip.try_sample_clock(&rig, before).unwrap().local()[0].translation,
        Vec3::X * 2.
    );
    assert_eq!(
        clip.try_sample_clock(&rig, 1.).unwrap().local()[0].translation,
        Vec3::ZERO
    );
    assert_eq!(
        clip.try_sample_clock(&rig, -f64::EPSILON).unwrap().local()[0].translation,
        Vec3::X * 2.
    );
    let mut animator = Animator::new(clip.clone());
    animator.time = before;
    let last = animator.advance_wall(&rig, 0.).unwrap();
    assert_eq!(last.pose.local()[0].translation, Vec3::X * 2.);
    assert_eq!(last.root_motion, Vec3::ZERO);
    let wrapped = animator.advance_wall(&rig, 1. - before).unwrap();
    assert_eq!(wrapped.pose.local()[0].translation, Vec3::ZERO);
    assert_eq!(wrapped.root_motion.x, (2. * (1. - before)) as f32);
    assert_eq!(animator.time, 0.);
    assert!(matches!(
        clip.try_sample_clock(&rig, f64::NAN),
        Err(AnimationError::InvalidSampleTime)
    ));
}

#[test]
fn bounded_clocks_preserve_small_ticks_after_long_elapsed_time_and_many_loops() {
    let rig = skeleton();
    let mut animator = Animator::new(root_clip(&rig, 2.));
    // At this elapsed time f32 addition loses a 60 Hz tick completely.
    let long = 1_048_576_f32;
    assert_eq!(long + 1. / 60., long);
    animator.time = f64::from(long) + 0.875;
    let dt = 1. / 60.;
    let frame = animator.advance(&rig, dt).unwrap();
    assert!(frame.root_motion.abs_diff_eq(Vec3::X * (2. * dt), 1e-7));
    assert!((animator.time - (0.875 + f64::from(dt))).abs() < 1e-12);
    animator.set_speed(8.).unwrap();
    let frame = animator.advance(&rig, 1.).unwrap();
    assert_eq!(frame.root_motion, Vec3::X * 16.);
    assert!((0. ..1.).contains(&animator.time));
    animator.set_speed(1.).unwrap();
    let initial = animator.time;
    let mut displacement = 0_f64;
    for _ in 0..100_000 {
        let frame = animator.advance(&rig, dt).unwrap();
        displacement += f64::from(frame.root_motion.x);
        assert!((0. ..1.).contains(&animator.time));
    }
    assert!((displacement - 200_000. * f64::from(dt)).abs() < 1e-6);
    let expected = (initial + 100_000. * f64::from(dt)).rem_euclid(1.);
    assert!((animator.time - expected).abs() < 1e-12);
}

#[test]
fn bounded_crossfade_clocks_match_local_phase_and_preserve_failed_tick() {
    let rig = skeleton();
    let mut control = Animator::new(root_clip(&rig, 2.));
    control.advance(&rig, 0.875).unwrap();
    control.transition_to(root_clip(&rig, 4.), 0.5).unwrap();
    control.advance(&rig, 0.125).unwrap();
    let mut long = control.clone();
    long.time += 1_048_576.;
    long.transition.as_mut().unwrap().source_time += 1_048_576.;
    let before = long.clone();
    assert!(long.advance(&rig, f32::NAN).is_err());
    assert_eq!(long.time, before.time);
    assert_eq!(
        long.transition.as_ref().unwrap().source_time,
        before.transition.as_ref().unwrap().source_time
    );
    for dt in [1. / 60., 0.25, 0.5] {
        let actual = long.advance(&rig, dt).unwrap();
        let expected = control.advance(&rig, dt).unwrap();
        assert_eq!(actual.pose, expected.pose);
        assert!(actual.root_motion.abs_diff_eq(expected.root_motion, 1e-7));
        assert_eq!(actual.transition_weight, expected.transition_weight);
    }
    assert!(long.transition.is_none());
    // Clamp remains at its endpoint rather than wrapping and replaying motion.
    let mut clip = (*root_clip(&rig, 2.)).clone();
    clip.playback = Playback::Clamp;
    clip.root_curve = Arc::new(root_curve::RootCurve::new(
        &clip.tracks[0].translations,
        clip.interpolation[0].translation,
        &clip.tangents[0].translation,
        clip.duration,
        Playback::Clamp,
    ));
    let mut clamped = Animator::new(Arc::new(clip));
    clamped.time = 1_048_576.;
    assert_eq!(
        clamped.advance(&rig, 1. / 60.).unwrap().root_motion,
        Vec3::ZERO
    );
    assert_eq!(clamped.time, 1.);
}

#[test]
fn animator_preserves_root_motion_across_loop_boundary() {
    let skeleton = skeleton();
    let mut animator = Animator::new(root_clip(&skeleton, 2.0));
    let first = animator.advance(&skeleton, 0.75).unwrap();
    let wrapped = animator.advance(&skeleton, 0.5).unwrap();
    assert!((first.root_motion.x - 1.5).abs() < 1.0e-6);
    assert!((wrapped.root_motion.x - 1.0).abs() < 1.0e-6);
}

#[test]
fn crossfade_is_bounded_and_reaches_target() {
    let skeleton = skeleton();
    let mut animator = Animator::new(root_clip(&skeleton, 0.0));
    animator
        .transition_to(root_clip(&skeleton, 2.0), 0.5)
        .unwrap();
    let half = animator.advance(&skeleton, 0.25).unwrap();
    assert!((half.transition_weight - 0.5).abs() < f32::EPSILON);
    let complete = animator.advance(&skeleton, 0.25).unwrap();
    assert!((complete.transition_weight - 1.0).abs() < f32::EPSILON);
    assert!(animator.transition.is_none());
}

#[test]
fn incompatible_clip_rejects_without_advancing() {
    let skeleton = skeleton();
    let mut animator = Animator::new(root_clip(&skeleton, 2.0));
    let other = Skeleton::new(vec![skeleton.joints()[0].clone()]).unwrap();
    assert!(matches!(
        animator.advance(&other, 0.5),
        Err(AnimationError::TrackCountMismatch { .. })
    ));
    assert_eq!(animator.time.to_bits(), 0.0f64.to_bits());
    assert!((animator.advance(&skeleton, 0.5).unwrap().root_motion.x - 1.0).abs() < 1e-6);
}

#[test]
fn palette_overflow_preserves_clock_and_unfinished_transition() {
    let good = skeleton();
    let mut joints = good.joints().to_vec();
    joints[0].inverse_bind = Mat4::from_scale(Vec3::splat(f32::MAX));
    let bad = Skeleton::new(joints).unwrap();
    let mut tracks = vec![JointTrack::default(); bad.joints().len()];
    tracks[0].scales = vec![
        Vec3Key {
            time: 0.,
            value: Vec3::splat(0.5),
        },
        Vec3Key {
            time: 1.,
            value: Vec3::splat(3.),
        },
    ];
    let target =
        Arc::new(AnimationClip::new("overflow scale", 1., Playback::Clamp, tracks, &bad).unwrap());
    let mut animator = Animator::new(root_clip(&bad, 0.0));
    animator.advance(&bad, 0.25).unwrap();
    animator.transition_to(target, 0.5).unwrap();
    animator.advance(&bad, 0.1).unwrap();
    let before = animator.clone();
    assert!(matches!(
        animator.advance(&bad, 0.4),
        Err(AnimationError::InvalidPose(_))
    ));
    assert_eq!(animator.time.to_bits(), before.time.to_bits());
    let transition = animator.transition.as_ref().unwrap();
    let prior = before.transition.as_ref().unwrap();
    assert_eq!(
        transition.source_time.to_bits(),
        prior.source_time.to_bits()
    );
    assert_eq!(transition.elapsed.to_bits(), prior.elapsed.to_bits());
    let mut control = before;
    let actual = animator.advance(&bad, 0.05).unwrap();
    let expected = control.advance(&bad, 0.05).unwrap();
    assert_eq!(actual.pose, expected.pose);
    assert_eq!(actual.root_motion, expected.root_motion);
    assert!(animator.transition.is_some());
}

#[test]
fn root_motion_overflow_rejects_without_advancing() {
    let skeleton = skeleton();
    let mut animator = Animator::new(root_clip(&skeleton, f32::MAX));
    animator.set_speed(8.0).unwrap();
    assert!(matches!(
        animator.advance(&skeleton, 1.0),
        Err(AnimationError::NumericalOverflow)
    ));
    assert_eq!(animator.time.to_bits(), 0.0f64.to_bits());
    animator.set_speed(0.0).unwrap();
    assert!(
        animator
            .advance(&skeleton, 1.0)
            .unwrap()
            .root_motion
            .is_finite()
    );
}

#[test]
fn finite_local_scales_cannot_publish_overflowing_global_matrices() {
    let mut joints = skeleton().joints().to_vec();
    for joint in &mut joints {
        joint.bind_local.scale = Vec3::splat(1e20);
    }
    let skeleton = Skeleton::new(joints).unwrap();
    assert!(matches!(
        skeleton.bind_pose().skin_matrices(&skeleton),
        Err(AnimationError::InvalidPose(1))
    ));
    let clip = AnimationClip::new(
        "overflowing bind hierarchy",
        1.,
        Playback::Loop,
        vec![JointTrack::default(); skeleton.joints().len()],
        &skeleton,
    )
    .unwrap();
    assert!(matches!(
        Animator::try_new(Arc::new(clip)),
        Err(AnimationError::InvalidPose(1))
    ));
    let valid_rig = self::skeleton();
    let clip = root_clip(&valid_rig, 2.);
    let mut checked = Animator::try_new(clip.clone()).unwrap();
    let mut legacy = Animator::new(clip);
    let checked_frame = checked.advance_wall(&valid_rig, 0.25).unwrap();
    let legacy_frame = legacy.advance_wall(&valid_rig, 0.25).unwrap();
    assert_eq!(checked_frame.pose, legacy_frame.pose);
    assert_eq!(checked_frame.skin_matrices, legacy_frame.skin_matrices);
    assert_eq!(checked_frame.root_motion, legacy_frame.root_motion);
}
