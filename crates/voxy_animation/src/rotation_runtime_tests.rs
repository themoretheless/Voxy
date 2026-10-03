use super::*;
fn turn() -> (Skeleton, Arc<AnimationClip>) {
    let rig = Skeleton::new(vec![Joint {
        name: Arc::from("root"),
        parent: None,
        bind_local: Transform::IDENTITY,
        inverse_bind: Mat4::IDENTITY,
    }])
    .unwrap();
    let keys = (0..=4)
        .map(|i| QuatKey {
            time: i as f32 / 16.,
            value: Quat::from_rotation_y(i as f32 * std::f32::consts::FRAC_PI_2),
        })
        .collect();
    let clip = AnimationClip::new(
        "turn",
        0.25,
        Playback::Loop,
        vec![JointTrack {
            rotations: keys,
            ..Default::default()
        }],
        &rig,
    )
    .unwrap();
    (rig, Arc::new(clip))
}
#[test]
fn frame_and_rotation_share_phase_winding_pause_and_bounded_clock() {
    let (rig, clip) = turn();
    let mut animator = Animator::new(clip.clone());
    let (frame, path) = animator.advance_with_root_rotation(&rig, 0.5, 256).unwrap();
    assert!(path.angular_travel_bound() >= 2. * std::f64::consts::TAU - 1e-6);
    assert!(path.end_rotation().abs_diff_eq(glam::DQuat::IDENTITY, 1e-6));
    assert!(
        frame.pose.local()[0]
            .rotation
            .abs_diff_eq(Quat::IDENTITY, 1e-6)
    );
    assert_eq!(animator.time, 0.);
    let mut split = Animator::new(clip);
    let (_, a) = split.advance_with_root_rotation(&rig, 0.125, 256).unwrap();
    let (_, b) = split.advance_with_root_rotation(&rig, 0.375, 256).unwrap();
    assert!((a.end_rotation() * b.end_rotation()).abs_diff_eq(path.end_rotation(), 1e-12));
    animator.set_speed(0.).unwrap();
    let (paused, hold) = animator.advance_with_root_rotation(&rig, 1., 256).unwrap();
    assert_eq!(paused.pose, frame.pose);
    assert!(hold.spans().is_empty());
    assert_eq!(hold.angular_travel_bound(), 0.);
}
#[test]
fn path_budget_foreign_rig_and_active_transition_preserve_clock() {
    let (rig, clip) = turn();
    let mut animator = Animator::new(clip.clone());
    let mut control = animator.clone();
    assert_eq!(
        animator
            .advance_with_root_rotation(&rig, 0.5, 1)
            .unwrap_err(),
        AnimationError::RootRotationBudget
    );
    assert_eq!(animator.time, control.time);
    let mut joints = rig.joints().to_vec();
    joints[0].name = Arc::from("foreign");
    assert_eq!(
        animator
            .advance_with_root_rotation(&Skeleton::new(joints).unwrap(), 0.1, 256)
            .unwrap_err(),
        AnimationError::SkeletonMismatch
    );
    let (_, actual) = animator
        .advance_with_root_rotation(&rig, 0.125, 256)
        .unwrap();
    let (_, expected) = control
        .advance_with_root_rotation(&rig, 0.125, 256)
        .unwrap();
    assert_eq!(actual.spans().len(), expected.spans().len());
    assert!(
        actual
            .end_rotation()
            .abs_diff_eq(expected.end_rotation(), 1e-12)
    );
    animator.transition_to(clip, 1.).unwrap();
    let clock = animator.time;
    let elapsed = animator.transition.as_ref().unwrap().elapsed;
    assert_eq!(
        animator
            .advance_with_root_rotation(&rig, 0.1, 256)
            .unwrap_err(),
        AnimationError::RootRotationTransitionUnsupported
    );
    assert_eq!(animator.time, clock);
    assert_eq!(animator.transition.as_ref().unwrap().elapsed, elapsed);
}
#[test]
fn constant_translation_proof_does_not_require_constant_rotation() {
    let (rig, _) = turn();
    let build = |tangent: Vec3| {
        AnimationClip::new_with_tangents(
            "pivot",
            1.,
            Playback::Loop,
            vec![JointTrack {
                translations: vec![
                    Vec3Key {
                        time: 0.,
                        value: Vec3::X,
                    },
                    Vec3Key {
                        time: 1.,
                        value: Vec3::X,
                    },
                ],
                rotations: vec![
                    QuatKey {
                        time: 0.,
                        value: Quat::IDENTITY,
                    },
                    QuatKey {
                        time: 1.,
                        value: Quat::from_rotation_y(1.),
                    },
                ],
                ..Default::default()
            }],
            vec![TrackInterpolation {
                translation: Interpolation::CubicSpline,
                ..Default::default()
            }],
            vec![JointTangents {
                translation: vec![[Vec3::ZERO, tangent], [-tangent, Vec3::ZERO]],
                ..Default::default()
            }],
            &rig,
        )
        .unwrap()
    };
    let fixed = build(Vec3::ZERO);
    assert_eq!(fixed.constant_joint_translation(0), Some(Vec3::X));
    assert!(fixed.constant_joint_transform(0).is_none());
    assert!(build(Vec3::Y).constant_joint_translation(0).is_none());
    assert!(fixed.constant_joint_translation(1).is_none());
}
