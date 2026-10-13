use super::*;
#[test]
fn preview_preserves_loop_endpoint_and_does_not_change_playback() {
    let rig = Skeleton::new(vec![Joint {
        name: Arc::from("root"),
        parent: None,
        bind_local: Transform::IDENTITY,
        inverse_bind: Mat4::IDENTITY,
    }])
    .unwrap();
    let clip = Arc::new(
        AnimationClip::new(
            "loop",
            2.,
            Playback::Loop,
            vec![JointTrack {
                translations: vec![
                    Vec3Key {
                        time: 0.,
                        value: Vec3::ZERO,
                    },
                    Vec3Key {
                        time: 2.,
                        value: Vec3::X,
                    },
                ],
                ..Default::default()
            }],
            &rig,
        )
        .unwrap(),
    );
    let mut animator = Animator::try_new(clip.clone()).unwrap();
    animator.advance_wall(&rig, 0.25).unwrap();
    let phase = animator.phase_interval_wall(0.).unwrap();
    for (p, x) in [(0., 0.), (0.25, 0.25), (0.5, 0.5), (1., 1.)] {
        let pose = clip.try_sample_phase(&rig, p).unwrap();
        assert_eq!(pose.local()[0].translation.x, x);
        pose.skin_matrices(&rig).unwrap();
    }
    assert_eq!(
        clip.try_sample(&rig, 2.).unwrap().local()[0].translation.x,
        0.
    );
    for p in [f64::NAN, f64::INFINITY, -0.01, 1.01] {
        assert_eq!(
            clip.try_sample_phase(&rig, p),
            Err(AnimationError::InvalidSampleTime)
        );
    }
    assert_eq!(animator.phase_interval_wall(0.).unwrap(), phase);
}
