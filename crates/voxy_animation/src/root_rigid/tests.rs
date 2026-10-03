use super::*;
use crate::{
    AnimationClip, Animator, Interpolation, Joint, JointTangents, JointTrack, QuatKey, Skeleton,
    TrackInterpolation, Transform, Vec3Key,
};
use glam::{Mat4, Quat, Vec4};
fn rig(bind: Vec3) -> Skeleton {
    Skeleton::new(vec![Joint {
        name: Arc::from("root"),
        parent: None,
        bind_local: Transform {
            translation: bind,
            ..Transform::IDENTITY
        },
        inverse_bind: Mat4::IDENTITY,
    }])
    .unwrap()
}
fn clip(
    bind: Vec3,
    translations: Vec<Vec3Key>,
    rotations: Vec<QuatKey>,
    modes: TrackInterpolation,
    tangents: JointTangents,
    playback: Playback,
) -> (Skeleton, Arc<AnimationClip>) {
    let rig = rig(bind);
    let clip = AnimationClip::new_with_tangents(
        "motion",
        1.,
        playback,
        vec![JointTrack {
            translations,
            rotations,
            ..Default::default()
        }],
        vec![modes],
        vec![tangents],
        &rig,
    )
    .unwrap();
    (rig, Arc::new(clip))
}
fn linear_turn(bind: Vec3) -> (Skeleton, Arc<AnimationClip>) {
    clip(
        bind,
        vec![
            Vec3Key {
                time: 0.,
                value: Vec3::X * 0.6,
            },
            Vec3Key {
                time: 1.,
                value: Vec3::new(1.6, 0.2, 0.),
            },
        ],
        vec![
            QuatKey {
                time: 0.,
                value: Quat::IDENTITY,
            },
            QuatKey {
                time: 1.,
                value: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            },
        ],
        TrackInterpolation::default(),
        JointTangents::default(),
        Playback::Loop,
    )
}
fn equivalent(a: RootRigidTransform, b: RootRigidTransform, tolerance: f64) {
    assert!(
        a.translation.abs_diff_eq(b.translation, tolerance),
        "{a:?} {b:?}"
    );
    for axis in [DVec3::X, DVec3::Y, DVec3::Z] {
        assert!(
            (a.rotation * axis).abs_diff_eq(b.rotation * axis, tolerance),
            "{a:?} {b:?}"
        );
    }
}
#[test]
fn turning_cycles_transport_translation_and_adjacent_intervals_compose() {
    let (_, clip) = linear_turn(Vec3::X * 0.6);
    let curve = clip.root_rigid_curve(0).unwrap();
    let axes = [true; 3];
    let cycle = curve.sample(1., axes).unwrap();
    assert!(
        cycle
            .translation
            .abs_diff_eq(DVec3::new(1.6, 0.2, 0.6), 1e-7)
    );
    let fourth = curve.sample(4., axes).unwrap();
    assert!(
        fourth.translation.abs_diff_eq(DVec3::Y * 0.8, 2e-7),
        "{fourth:?}"
    );
    assert!((fourth.rotation * DVec3::X).abs_diff_eq(DVec3::X, 1e-12));
    let full = curve.path(0.37, 2.63, axes, 256).unwrap();
    let a = curve.path(0.37, 0.9, axes, 256).unwrap();
    let b = curve.path(0.9, 2.63, axes, 256).unwrap();
    equivalent(
        a.end_transform().compose(b.end_transform()).unwrap(),
        full.end_transform(),
        1e-12,
    );
    let expected = curve
        .sample(0.37, axes)
        .unwrap()
        .inverse()
        .unwrap()
        .compose(curve.sample(2.63, axes).unwrap())
        .unwrap();
    equivalent(full.end_transform(), expected, 1e-12);
    for span in full.spans() {
        for u in [0., 0.25, 0.5, 0.75, 1.] {
            let time = 0.37 + span.start() + u * (span.end() - span.start());
            let source = curve
                .sample(0.37, axes)
                .unwrap()
                .inverse()
                .unwrap()
                .compose(curve.sample(time, axes).unwrap())
                .unwrap();
            equivalent(span.sample(u).unwrap(), source, 1e-12);
        }
    }
    // A far clock cancels its common old cycle prefix before generating a small tick.
    let far = curve.path(1_048_576.25, 1_048_576.5, axes, 256).unwrap();
    let near = curve.path(0.25, 0.5, axes, 256).unwrap();
    equivalent(far.end_transform(), near.end_transform(), 1e-12);
    assert_eq!(far.spans().len(), near.spans().len());
    equivalent(
        full.spans().last().unwrap().sample(1.).unwrap(),
        full.end_transform(),
        1e-12,
    );
}
#[test]
fn masks_match_in_place_bone_position_without_rotating_initial_authored_offset() {
    let bind = Vec3::new(0.2, 1., 0.);
    let (_, clip) = linear_turn(bind);
    let curve = clip.root_rigid_curve(0).unwrap();
    for axes in [[false; 3], [true; 3], [true, false, false]] {
        let path = curve.path(0., 0.75, axes, 256).unwrap();
        for span in path.spans() {
            for step in 0..=20 {
                let u = f64::from(step) / 20.;
                let time = span.start() + u * (span.end() - span.start());
                let p = DVec3::new(0.6, 0., 0.).lerp(DVec3::new(1.6, 0.2, 0.), time);
                let in_place = DVec3::from_array(std::array::from_fn(|i| {
                    if axes[i] { f64::from(bind[i]) } else { p[i] }
                }));
                let target = DVec3::from_array(std::array::from_fn(|i| {
                    if axes[i] {
                        p[i] - [0.6, 0., 0.][i] + f64::from(bind[i])
                    } else {
                        p[i]
                    }
                }));
                let actual = span.sample(u).unwrap();
                assert!(
                    actual
                        .transform_point(in_place)
                        .unwrap()
                        .abs_diff_eq(target, 2e-7)
                );
                assert!((actual.rotation * DVec3::X).abs_diff_eq(
                    DQuat::from_rotation_y(std::f64::consts::FRAC_PI_2 * time) * DVec3::X,
                    1e-12
                ));
            }
        }
    }
    equivalent(
        curve.sample(0., [true; 3]).unwrap(),
        RootRigidTransform::IDENTITY,
        1e-12,
    );
}
#[test]
fn closed_cubic_moving_pivot_preserves_excursion_projection_hulls_and_speed() {
    let p0 = Vec3::X * 0.6;
    let tangent = Vec3::new(0., 3., 1.);
    let (_, clip) = clip(
        p0,
        vec![
            Vec3Key {
                time: 0.,
                value: p0,
            },
            Vec3Key {
                time: 1.,
                value: p0,
            },
        ],
        vec![
            QuatKey {
                time: 0.,
                value: Quat::IDENTITY,
            },
            QuatKey {
                time: 1.,
                value: Quat::IDENTITY,
            },
        ],
        TrackInterpolation {
            translation: Interpolation::CubicSpline,
            rotation: Interpolation::CubicSpline,
            ..Default::default()
        },
        JointTangents {
            translation: vec![[Vec3::ZERO, tangent], [-tangent, Vec3::ZERO]],
            rotation: vec![[Vec4::ZERO, Vec4::Y * 8.], [-Vec4::Y * 8., Vec4::ZERO]],
            ..Default::default()
        },
        Playback::Loop,
    );
    let curve = clip.root_rigid_curve(0).unwrap();
    for axes in [[false; 3], [true; 3]] {
        let path = curve.path(0., 1., axes, 256).unwrap();
        equivalent(path.end_transform(), RootRigidTransform::IDENTITY, 1e-12);
        assert!(path.spans().len() > 1);
        for span in path.spans() {
            for point in [DVec3::new(-0.4, 0.2, 0.03), DVec3::new(0.4, 1000., 0.02)] {
                let normals = [DVec3::Y, DVec3::new(0.3, -0.4, 0.7)];
                let bounds = normals.map(|normal| span.projection_bounds(point, normal).unwrap());
                let speed = span.point_speed_bound(point).unwrap();
                assert!(
                    speed < 100.,
                    "tall yaw point must not use its full sphere speed: {speed}"
                );
                for step in 0..=40 {
                    let u = f64::from(step) / 40.;
                    let time = span.start() + u * (span.end() - span.start());
                    let p = p0.as_dvec3() + tangent.as_dvec3() * (time * (1. - time));
                    let q = DQuat::from_xyzw(0., 8. * time * (1. - time), 0., 1.).normalize();
                    let pivot = if axes[0] { p0.as_dvec3() } else { p };
                    let expected = p + q * (point - pivot);
                    let actual = span.sample(u).unwrap().transform_point(point).unwrap();
                    assert!(
                        actual.abs_diff_eq(expected, 1e-9),
                        "{actual:?} {expected:?}"
                    );
                    for (normal, [low, high]) in normals.into_iter().zip(bounds) {
                        let value = normal.dot(expected);
                        assert!(value >= low - 1e-10 && value <= high + 1e-10);
                    }
                    if !axes[0] {
                        assert!(
                            (bounds[0][0] - point.y).abs() < 1e-7
                                && (bounds[0][1] - point.y).abs() < 1e-7
                        );
                    }
                    if step > 0 && step < 40 {
                        let h = 1e-5;
                        let a = span.sample(u - h).unwrap().transform_point(point).unwrap();
                        let b = span.sample(u + h).unwrap().transform_point(point).unwrap();
                        assert!((b - a).length() / (2. * h) <= speed * (1. + 1e-5) + 1e-6);
                    }
                }
            }
        }
    }
}
#[test]
fn step_events_merge_simultaneous_channels_and_boundaries_are_consumed_once() {
    for time in [0.25, 0.5] {
        let p0 = Vec3::X * 0.6;
        let p1 = Vec3::new(1.6, 0.2, 0.);
        let (_, clip) = clip(
            p0,
            vec![
                Vec3Key {
                    time: 0.,
                    value: p0,
                },
                Vec3Key { time, value: p1 },
            ],
            vec![
                QuatKey {
                    time: 0.,
                    value: Quat::IDENTITY,
                },
                QuatKey {
                    time: 0.5,
                    value: Quat::from_rotation_y(0.7),
                },
            ],
            TrackInterpolation {
                translation: Interpolation::Step,
                rotation: Interpolation::Step,
                ..Default::default()
            },
            JointTangents::default(),
            Playback::Clamp,
        );
        let curve = clip.root_rigid_curve(0).unwrap();
        let path = curve.path(0., 1., [true; 3], 256).unwrap();
        assert_eq!(
            path.spans().iter().filter(|span| span.is_step()).count(),
            if time == 0.5 { 1 } else { 2 }
        );
        for span in path.spans().iter().filter(|span| span.is_step()) {
            assert!(span.point_speed_bound(DVec3::X).unwrap() > 0.);
            if time == 0.5 {
                for i in 0..=20 {
                    let u = f64::from(i) / 20.;
                    let expected_q =
                        DQuat::from_array(Quat::from_rotation_y(0.7).to_array().map(f64::from))
                            .normalize();
                    let expected_q = DQuat::IDENTITY.slerp(expected_q, u);
                    let expected = RootRigidTransform {
                        rotation: expected_q,
                        translation: p0.as_dvec3().lerp(p1.as_dvec3(), u)
                            - expected_q * p0.as_dvec3(),
                    };
                    equivalent(span.sample(u).unwrap(), expected, 1e-12);
                }
            }
        }
        let a = curve.path(0., 0.5, [true; 3], 256).unwrap();
        let b = curve.path(0.5, 1., [true; 3], 256).unwrap();
        assert!(b.spans().iter().all(|span| !span.is_step()));
        equivalent(
            a.end_transform().compose(b.end_transform()).unwrap(),
            path.end_transform(),
            1e-12,
        );
        assert!(
            curve
                .path(10., 11., [true; 3], 256)
                .unwrap()
                .spans()
                .is_empty()
        );
    }
}
#[test]
fn rigid_clock_publication_rejects_budgets_before_commit_and_retries_same_interval() {
    let (rig, clip) = linear_turn(Vec3::X * 0.6);
    let mut animator = Animator::new(clip.clone());
    let mut control = animator.clone();
    assert!(
        animator
            .advance_with_root_rigid_motion(&rig, 1., [true; 3], 0)
            .is_err()
    );
    let (actual, path) = animator
        .advance_with_root_rigid_motion(&rig, 0.25, [true; 3], 256)
        .unwrap();
    let (expected, reference) = control
        .advance_with_root_rigid_motion(&rig, 0.25, [true; 3], 256)
        .unwrap();
    assert_eq!(actual.pose, expected.pose);
    equivalent(path.end_transform(), reference.end_transform(), 1e-12);
    assert!(
        clip.root_rigid_curve(0)
            .unwrap()
            .path(0., 1_000_000., [true; 3], 256)
            .is_err()
    );
    assert!(
        clip.root_rigid_curve(0)
            .unwrap()
            .sample(1e30, [true; 3])
            .is_err()
    );
    animator.set_speed(0.).unwrap();
    assert!(
        animator
            .advance_with_root_rigid_motion(&rig, 0.25, [true; 3], 256)
            .unwrap()
            .1
            .spans()
            .is_empty()
    );
}
#[test]
fn selected_translation_cache_is_shared_bounded_and_preserves_failed_selection() {
    let rig = Skeleton::new(
        (0..3)
            .map(|i| Joint {
                name: Arc::from(format!("joint{i}")),
                parent: None,
                bind_local: Transform::IDENTITY,
                inverse_bind: Mat4::IDENTITY,
            })
            .collect(),
    )
    .unwrap();
    let keys = |count: usize| {
        (0..count)
            .map(|i| Vec3Key {
                time: if count == 1 {
                    0.
                } else {
                    i as f32 / (count - 1) as f32
                },
                value: Vec3::X
                    * if count == 1 {
                        0.
                    } else {
                        i as f32 / (count - 1) as f32
                    },
            })
            .collect()
    };
    let clip = Arc::new(
        AnimationClip::new(
            "cache",
            1.,
            Playback::Loop,
            vec![
                JointTrack {
                    translations: keys(1),
                    ..Default::default()
                },
                JointTrack {
                    translations: keys(32768),
                    ..Default::default()
                },
                JointTrack {
                    translations: keys(32768),
                    ..Default::default()
                },
            ],
            &rig,
        )
        .unwrap(),
    );
    let mut animator = Animator::new(clip.clone());
    animator.set_root_motion_joint(1).unwrap();
    let a = clip.root_rigid_curve(1).unwrap();
    let b = (*clip).clone().root_rigid_curve(1).unwrap();
    assert!(Arc::ptr_eq(&a.0, &b.0));
    assert!(Arc::ptr_eq(&a.0.translation, &animator.motion_curve));
    assert_eq!(
        clip.motion_cache_keys
            .load(std::sync::atomic::Ordering::Relaxed),
        32769
    );
    animator.advance(&rig, 0.1).unwrap();
    let clock = animator.time;
    assert_eq!(
        animator.set_root_motion_joint(2).unwrap_err(),
        AnimationError::RootMotionBudget
    );
    assert_eq!(animator.root_motion_joint(), 1);
    assert_eq!(animator.time, clock);
    assert!(Arc::ptr_eq(&a.0.translation, &animator.motion_curve));
    assert_eq!(animator.advance(&rig, 0.1).unwrap().root_motion.x, 0.1);
}

#[test]
fn primary_translation_compilation_is_bounded_and_concurrent_rigid_selection_shares_coefficients() {
    let rig = rig(Vec3::ZERO);
    let many = (0..=crate::MAX_ROOT_TRANSLATION_CACHE_KEYS)
        .map(|i| Vec3Key {
            time: i as f32 / crate::MAX_ROOT_TRANSLATION_CACHE_KEYS as f32,
            value: Vec3::X,
        })
        .collect();
    assert_eq!(
        AnimationClip::new(
            "too many",
            1.,
            Playback::Loop,
            vec![JointTrack {
                translations: many,
                ..Default::default()
            }],
            &rig
        )
        .unwrap_err(),
        AnimationError::RootMotionBudget
    );
    let (_, clip) = linear_turn(Vec3::X * 0.6);
    let mut workers = Vec::new();
    for _ in 0..16 {
        let clip = clip.clone();
        workers.push(std::thread::spawn(move || {
            clip.root_rigid_curve(0).unwrap()
        }));
    }
    let first = workers.remove(0).join().unwrap();
    for worker in workers {
        assert!(Arc::ptr_eq(&first.0, &worker.join().unwrap().0));
    }
    assert_eq!(
        clip.motion_cache_keys
            .load(std::sync::atomic::Ordering::Relaxed),
        2
    );
    assert_eq!(
        clip.rotation_cache_keys
            .load(std::sync::atomic::Ordering::Relaxed),
        2
    );
}

#[test]
fn coordinate_transport_preserves_ordered_curves_bounds_and_step_events() {
    let basis = DQuat::from_rotation_z(0.7) * DQuat::from_rotation_x(-0.4);
    let offset = DVec3::new(3., -2., 0.5);
    for mode in [
        Interpolation::Linear,
        Interpolation::Step,
        Interpolation::CubicSpline,
    ] {
        let tangents = if mode == Interpolation::CubicSpline {
            JointTangents {
                translation: vec![
                    [Vec3::ZERO, Vec3::new(0., 3., 1.)],
                    [-Vec3::new(0., 3., 1.), Vec3::ZERO],
                ],
                rotation: vec![[Vec4::ZERO, Vec4::Y * 2.], [-Vec4::Y * 2., Vec4::ZERO]],
                ..Default::default()
            }
        } else {
            Default::default()
        };
        let (_, clip) = clip(
            Vec3::X * 0.6,
            vec![
                Vec3Key {
                    time: 0.,
                    value: Vec3::X * 0.6,
                },
                Vec3Key {
                    time: 1.,
                    value: Vec3::new(1.6, 0.2, 0.),
                },
            ],
            vec![
                QuatKey {
                    time: 0.,
                    value: Quat::IDENTITY,
                },
                QuatKey {
                    time: 1.,
                    value: Quat::from_rotation_y(1.2),
                },
            ],
            TrackInterpolation {
                translation: mode,
                rotation: mode,
                ..Default::default()
            },
            tangents,
            Playback::Loop,
        );
        let path = clip
            .root_rigid_curve(0)
            .unwrap()
            .path(0.13, 2.4, [true; 3], 256)
            .unwrap();
        for scale in [0., 2.5] {
            let mapped = path.transformed(basis, scale, offset).unwrap();
            assert_eq!(mapped.spans().len(), path.spans().len());
            assert_eq!(mapped.duration(), path.duration());
            assert!((mapped.angular_travel_bound() - path.angular_travel_bound()).abs() < 1e-10);
            let convert = |h: RootRigidTransform| RootRigidTransform {
                rotation: (basis * h.rotation * basis.conjugate()).normalize(),
                translation: scale * (basis * h.translation) + offset
                    - (basis * h.rotation * basis.conjugate()) * offset,
            };
            equivalent(mapped.end_transform(), convert(path.end_transform()), 1e-10);
            for (a, b) in path.spans().iter().zip(mapped.spans()) {
                assert_eq!(
                    (a.start(), a.end(), a.is_step()),
                    (b.start(), b.end(), b.is_step())
                );
                let point = DVec3::new(1.4, -0.7, 2.);
                let normal = DVec3::new(-0.3, 0.8, 0.4);
                let bounds = b.projection_bounds(point, normal).unwrap();
                let speed = b.point_speed_bound(point).unwrap();
                for i in 0..=100 {
                    let u = i as f64 / 100.;
                    equivalent(b.sample(u).unwrap(), convert(a.sample(u).unwrap()), 1e-10);
                    let value = normal.dot(b.sample(u).unwrap().transform_point(point).unwrap());
                    assert!(
                        value >= bounds[0] && value <= bounds[1],
                        "{mode:?} {value} {bounds:?}"
                    );
                    if i < 100 {
                        let delta = (b.sample(u + 0.01).unwrap().transform_point(point).unwrap()
                            - b.sample(u).unwrap().transform_point(point).unwrap())
                        .length()
                            / 0.01;
                        assert!(delta <= speed + 1e-8, "{delta} {speed}");
                    }
                }
            }
        }
        for (q, s, c) in [
            (DQuat::from_xyzw(0., 0., 0., 0.), 1., offset),
            (basis, -1., offset),
            (basis, f64::INFINITY, offset),
            (basis, 1., DVec3::NAN),
        ] {
            assert!(path.transformed(q, s, c).is_err());
        }
        assert!(path.transformed(basis, f64::MAX, offset).is_err());
        assert!(path.transformed(basis, 2.5, offset).is_ok());
    }
}
