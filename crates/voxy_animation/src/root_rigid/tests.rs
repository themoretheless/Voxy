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
                    let velocity = span.velocity(u).unwrap().unwrap();
                    let dp = tangent.as_dvec3() * (1. - 2. * time);
                    let r = 8. * time * (1. - time);
                    let omega = DVec3::Y * (16. * (1. - 2. * time) / (1. + r*r));
                    let denominator = 1.+r*r;
                    let independent_acceleration = (-32./denominator
                        -256.*r*(1.-2.*time).powi(2)/(denominator*denominator)).abs();
                    assert!(independent_acceleration <= span.rotation().angular_acceleration_bound().unwrap().unwrap());
                    let enclosed=span.rotation().enclosed_angular_derivative_bounds().unwrap().unwrap();
                    assert!(independent_acceleration<=enclosed.acceleration_bound());
                    assert!((16. * (1.-2.*time)/(1.+r*r)).abs()<=enclosed.speed_bound());
                    let rates = span.twist_rate_bounds().unwrap().unwrap();
                    let bounds = span.twist_bounds().unwrap().unwrap();
                    let twist = velocity.spatial_twist(span.sample(u).unwrap()).unwrap();
                    assert!(twist.linear.length() <= bounds.linear_speed_bound);
                    assert!(twist.angular.length() <= bounds.angular_speed_bound*(1.+1e-12));
                    assert!(rates.angular>=independent_acceleration);
                    let acceleration = DVec3::Y*(-32./denominator-256.*r*(1.-2.*time).powi(2)/(denominator*denominator));
                    let ddp = -2.*tangent.as_dvec3();
                    let dpivot_rate = if axes[0] { DVec3::ZERO } else { dp };
                    let ddpivot = if axes[0] { DVec3::ZERO } else { ddp };
                    let spatial_linear_rate = ddp - acceleration.cross(p) - omega.cross(dp)
                        - omega.cross(q*dpivot_rate) - q*ddpivot;
                    assert!(spatial_linear_rate.length()<=rates.linear);
                    let enclosed=span.enclosed_twist_bounds().unwrap().unwrap();
                    assert!(spatial_linear_rate.length()<=enclosed.rates.linear);
                    assert!(twist.linear.length()<=enclosed.linear_speed_bound);
                    let dpivot = if axes[0] { DVec3::ZERO } else { dp };
                    let expected_velocity = dp + omega.cross(q * (point - pivot)) - q * dpivot;
                    let actual_velocity = velocity.linear + velocity.angular.cross(span.sample(u).unwrap().rotation * point);
                    assert!((velocity.angular - omega).length() < 1e-8);
                    assert!((actual_velocity - expected_velocity).length() < 1e-8);
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
        // A spatial prefix must retain each instantaneous event and its swept geometry.
        let prefix = RootRigidPath::from_twists(&[(RootRigidTwist {
            linear: DVec3::new(0.2, -0.1, 0.3),
            angular: DVec3::X * 0.4,
        }, 0.3)], 1).unwrap();
        let joined = prefix.append_spatial(&path, 256).unwrap();
        for (original, appended) in path.spans().iter().zip(&joined.spans()[1..]) {
            assert_eq!(original.is_step(), appended.is_step());
            assert!((appended.start() - (original.start() + 0.3)).abs() < 1e-12);
            assert!((appended.end() - (original.end() + 0.3)).abs() < 1e-12);
            for u in [0., 0.25, 0.5, 0.75, 1.] {
                equivalent(appended.sample(u).unwrap(),
                    original.sample(u).unwrap().compose(prefix.end_transform()).unwrap(), 1e-12);
            }
            if original.is_step() {
                assert!(appended.velocity(0.5).unwrap().is_none());
            }
        }
        equivalent(joined.end_transform(),
            path.end_transform().compose(prefix.end_transform()).unwrap(), 1e-12);
        assert!(matches!(path.blend_spatial(&path, [0., 1.], 1., 0.01, 0.01, 256),
            Err(AnimationError::RootRotationTransitionUnsupported)));
        let partition = path.partition_for_blend(&path,256).unwrap();
        let recorded_source:Vec<_> = partition.steps.iter().flat_map(|step|step.source_spans.iter().copied()).collect();
        let recorded_target:Vec<_> = partition.steps.iter().flat_map(|step|step.target_spans.iter().copied()).collect();
        let expected_events:Vec<_> = path.spans().iter().enumerate().filter(|(_,span)|span.is_step()).map(|(index,_)|index).collect();
        assert_eq!(recorded_source,expected_events);
        assert_eq!(recorded_target,expected_events);
        for interval in &partition.intervals {
            assert_eq!(interval.source_span,interval.target_span);
            assert!(interval.start<interval.end);
        }
        for span in path.spans().iter().filter(|span| span.is_step()) {
            assert!(span.twist_rate_bounds().unwrap().is_none());
            assert_eq!(span.velocity(0.5).unwrap(), None);
            assert_eq!(span.rotation().angular_velocity(0.5).unwrap(), None);
            assert!(span.rotation().angular_velocity_bounds(0.5).unwrap().is_none());
            assert!(span.spatial_twist_enclosure(0.5).unwrap().is_none());
            assert!(span.enclosed_twist_bounds().unwrap().is_none());
            assert_eq!(span.rotation().angular_acceleration_bound().unwrap(), None);
            assert!(span.rotation().enclosed_angular_derivative_bounds().unwrap().is_none());
            assert!(span.velocity(f64::NAN).is_err());
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
                    match (a.velocity(u).unwrap(), b.velocity(u).unwrap()) {
                        (Some(source), Some(target)) => {
                            let converted = source.transformed(a.sample(u).unwrap().rotation, basis, scale, offset).unwrap();
                            assert!((converted.linear - target.linear).length() < 1e-9);
                            assert!((converted.angular - target.angular).length() < 1e-9);
                            let original_twist=source.spatial_twist(a.sample(u).unwrap()).unwrap();
                            let expected_twist=target.spatial_twist(b.sample(u).unwrap()).unwrap();
                            let mapped_twist=original_twist.transformed(basis,scale,offset).unwrap();
                            assert!((mapped_twist.linear-expected_twist.linear).length()<1e-9);
                            assert!((mapped_twist.angular-expected_twist.angular).length()<1e-9);
                            let bounds=a.twist_bounds().unwrap().unwrap().transformed(basis,scale,offset).unwrap();
                            assert!(mapped_twist.linear.length()<=bounds.linear_speed_bound*(1.+1e-12)+1e-12);
                            assert!(mapped_twist.angular.length()<=bounds.angular_speed_bound*(1.+1e-12)+1e-12);
                        }
                        (None, None) => {}
                        _ => panic!("coordinate transport changed velocity admission"),
                    }
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

#[test]
fn paused_crossfades_keep_pose_blending_and_publish_identity_rigid_motion() {
    let (rig, clip) = linear_turn(Vec3::X * 0.6);
    let mut animator = Animator::new(clip.clone());
    animator.advance(&rig,0.2).unwrap();
    animator.transition_to_at_phase(clip,1.,0.5).unwrap();
    let mut regular = animator.clone();
    let (zero, path) = animator.advance_with_root_rigid_motion(&rig,0.,[true;3],256).unwrap();
    assert_eq!(zero.pose,regular.advance(&rig,0.).unwrap().pose);
    assert!(path.spans().is_empty());
    equivalent(path.end_transform(),RootRigidTransform::IDENTITY,0.);
    animator.set_speed(0.).unwrap(); regular.set_speed(0.).unwrap();
    let mut rotation_only = animator.clone();
    let (actual,path) = animator.advance_with_root_rigid_motion(&rig,0.25,[true;3],256).unwrap();
    let expected = regular.advance(&rig,0.25).unwrap();
    assert_eq!(actual.pose,expected.pose);
    assert_eq!(actual.transition_weight,0.25);
    assert_eq!(actual.root_motion,Vec3::ZERO);
    assert!(path.spans().is_empty());
    equivalent(path.end_transform(),RootRigidTransform::IDENTITY,0.);
    let (rotation_frame,rotation_path) = rotation_only.advance_with_root_rotation(&rig,0.25,256).unwrap();
    assert_eq!(rotation_frame.pose,expected.pose);
    assert!(rotation_path.spans().is_empty());
    animator.set_speed(1.).unwrap();
    assert!(matches!(animator.advance_with_root_rigid_motion(&rig,0.1,[true;3],256),Err(AnimationError::RootRotationTransitionUnsupported)));
    animator.set_speed(0.).unwrap();
    assert!(animator.advance_with_root_rigid_motion(&rig,f32::NAN,[true;3],256).is_err());
    let (completed,path) = animator.advance_with_root_rigid_motion(&rig,0.75,[true;3],256).unwrap();
    assert_eq!(completed.pose,regular.advance(&rig,0.75).unwrap().pose);
    assert_eq!(completed.transition_weight,1.);
    assert!(path.spans().is_empty());
    animator.set_speed(1.).unwrap(); regular.set_speed(1.).unwrap();
    assert_eq!(animator.advance_with_root_rigid_motion(&rig,0.1,[true;3],256).unwrap().0.pose,
        regular.advance(&rig,0.1).unwrap().pose);
}

#[test]
fn composed_velocity_includes_rotating_pivot_and_matches_curve_derivative() {
    let (_, clip) = linear_turn(Vec3::X*0.6);
    let curve = clip.root_rigid_curve(0).unwrap();
    let path = curve.path(0.,1.,[true;3],256).unwrap();
    for span in path.spans() {
        for fraction in [0.1,0.4,0.9] {
            assert_eq!(span.rotation().angular_acceleration_bound().unwrap(), Some(0.));
            let velocity = span.velocity(fraction).unwrap().unwrap();
            let transform = span.sample(fraction).unwrap();
            let expected_angular = DVec3::Y * std::f64::consts::FRAC_PI_2;
            let expected_linear = DVec3::new(1.,0.2,0.) - expected_angular.cross(transform.rotation * (DVec3::X*0.6));
            assert!((velocity.angular-expected_angular).length()<1e-6);
            assert!((velocity.linear-expected_linear).length()<1e-6);
            let h = 1e-5;
            let before = span.sample(fraction-h).unwrap();
            let after = span.sample(fraction+h).unwrap();
            let dt = 2.*h*(span.end()-span.start());
            assert!((velocity.linear-(after.translation-before.translation)/dt).length()<1e-8);
            for axis in [DVec3::X,DVec3::Y,DVec3::Z] {
                let numeric = (after.rotation*axis-before.rotation*axis)/dt;
                assert!((numeric-velocity.angular.cross(transform.rotation*axis)).length()<1e-8);
            }
        }
        assert!(span.velocity(f64::NAN).is_err());
    }
}

#[test]
fn velocity_coordinate_transport_accounts_for_shifted_origin_and_rejects_invalid_inputs() {
    let velocity = RootRigidVelocity { linear:DVec3::new(2.,3.,4.), angular:DVec3::Y*2. };
    let rotation = DQuat::from_rotation_y(std::f64::consts::FRAC_PI_2);
    let result = velocity.transformed(rotation,DQuat::IDENTITY,0.,DVec3::X).unwrap();
    assert!((result.linear-DVec3::X*2.).length()<1e-12);
    assert_eq!(result.angular,velocity.angular);
    for (q,s,c) in [(DQuat::from_xyzw(0.,0.,0.,0.),1.,DVec3::ZERO),
        (DQuat::IDENTITY,-1.,DVec3::ZERO),(DQuat::IDENTITY,f64::INFINITY,DVec3::ZERO),
        (DQuat::IDENTITY,1.,DVec3::NAN)] {
        assert!(velocity.transformed(rotation,q,s,c).is_err());
    }
    assert!(velocity.transformed(DQuat::from_xyzw(0.,0.,0.,0.),DQuat::IDENTITY,1.,DVec3::ZERO).is_err());
    assert!(velocity.transformed(rotation,DQuat::IDENTITY,f64::MAX,DVec3::ZERO).is_err());
    assert!(RootRigidVelocity { linear:DVec3::NAN,..velocity }.transformed(rotation,DQuat::IDENTITY,0.,DVec3::ZERO).is_err());
    assert!(velocity.transformed(rotation,DQuat::IDENTITY,1.,DVec3::ZERO).is_ok());
}

#[test]
fn constant_spatial_twist_integrates_offset_rotation_and_translation_without_pivot_drift() {
    let pivot = DVec3::new(0.6,0.2,-0.4);
    for rate in [0.,1e-9,1e-4,0.3,7.] {
        let angular = DVec3::Y*rate;
        let twist = RootRigidTwist { linear:-angular.cross(pivot)+DVec3::Y*0.7, angular };
        for duration in [0.,0.001,0.2,1.] {
            let actual = twist.increment(duration).unwrap();
            let rotation = DQuat::from_rotation_y(rate*duration);
            let expected = RootRigidTransform { rotation, translation:pivot-rotation*pivot+DVec3::Y*(0.7*duration) };
            equivalent(actual,expected,1e-12);
            let half = twist.increment(duration*0.5).unwrap();
            equivalent(half.compose(half).unwrap(),actual,1e-12);
        }
    }
    let start = RootRigidTransform { translation:DVec3::new(2.,3.,4.), rotation:DQuat::from_rotation_x(0.4) };
    let angular = DVec3::Y*2.;
    let velocity = RootRigidVelocity { linear:angular.cross(start.translation-pivot), angular };
    let twist = velocity.spatial_twist(start).unwrap();
    let actual = twist.increment(0.2).unwrap().compose(start).unwrap();
    let rotation = DQuat::from_rotation_y(0.4);
    equivalent(actual,RootRigidTransform { rotation:rotation*start.rotation,
        translation:pivot+rotation*(start.translation-pivot) },1e-12);
    assert!(twist.increment(f64::NAN).is_err());
    assert!(twist.increment(-1.).is_err());
    assert!(RootRigidTwist { linear:DVec3::NAN,..twist }.increment(0.).is_err());
    assert!(twist.increment(f64::MAX).is_err());
    assert!(twist.increment(0.2).is_ok());
}

#[test]
fn ordered_twist_path_retains_noncommuting_rotations_and_encloses_collision_points() {
    let segments = [
        (RootRigidTwist { linear:DVec3::new(0.2,0.4,0.1), angular:DVec3::Y*2. },0.4),
        (RootRigidTwist { linear:DVec3::new(-0.3,0.1,0.4), angular:DVec3::X*1.5 },0.3),
    ];
    let path = RootRigidPath::from_twists(&segments,2).unwrap();
    assert_eq!(path.spans().len(),2);
    let expected = segments[1].0.increment(segments[1].1).unwrap()
        .compose(segments[0].0.increment(segments[0].1).unwrap()).unwrap();
    equivalent(path.end_transform(),expected,1e-12);
    equivalent(path.spans()[0].sample(1.).unwrap(),path.spans()[1].sample(0.).unwrap(),1e-12);
    let reversed = segments[0].0.increment(segments[0].1).unwrap()
        .compose(segments[1].0.increment(segments[1].1).unwrap()).unwrap();
    assert!((expected.rotation*DVec3::Z-reversed.rotation*DVec3::Z).length()>0.1);
    let basis = DQuat::from_rotation_z(0.6);
    let offset = DVec3::new(0.6,0.3,-0.2);
    for scale in [0.,2.5] {
        let mapped = path.transformed(basis,scale,offset).unwrap();
        for (source,span) in path.spans().iter().zip(mapped.spans()) {
            let rates = span.twist_rate_bounds().unwrap().unwrap();
            assert_eq!((rates.linear,rates.angular),(0.,0.));
            let point = DVec3::new(1.,-0.7,0.4);
            let normal = DVec3::new(0.3,-0.4,0.2);
            let bounds = span.projection_bounds(point,normal).unwrap();
            let speed = span.point_speed_bound(point).unwrap();
            for i in 0..=100 {
                let u = i as f64/100.;
                let actual = span.sample(u).unwrap();
                let original = source.sample(u).unwrap();
                let rotation = (basis*original.rotation*basis.conjugate()).normalize();
                equivalent(actual,RootRigidTransform { rotation,
                    translation:scale*(basis*original.translation)+offset-rotation*offset },1e-12);
                let value = normal.dot(actual.transform_point(point).unwrap());
                assert!(value>=bounds[0] && value<=bounds[1]);
                let velocity = span.velocity(u).unwrap().unwrap();
                let transferred = source.velocity(u).unwrap().unwrap().transformed(original.rotation,basis,scale,offset).unwrap();
                assert!((velocity.linear-transferred.linear).length()<1e-12);
                assert!((velocity.angular-transferred.angular).length()<1e-12);
                if i<100 {
                    let delta=(span.sample(u+0.01).unwrap().transform_point(point).unwrap()-actual.transform_point(point).unwrap()).length();
                    assert!(delta<=speed*0.01+1e-12);
                }
            }
        }
    }
    assert!(RootRigidPath::from_twists(&segments,1).is_err());
    assert!(RootRigidPath::from_twists(&[(segments[0].0,0.)],2).is_err());
    assert!(RootRigidPath::from_twists(&[(segments[0].0,f64::NAN)],2).is_err());
    assert!(RootRigidPath::from_twists(&[],0).unwrap().spans().is_empty());
    assert!(RootRigidPath::from_twists(&segments,2).is_ok());
}

#[test]
fn varying_spatial_velocity_refines_with_bounds_against_independent_ramp_integrals() {
    let sample = |time| Ok(RootRigidTwist { linear:DVec3::X*time, angular:DVec3::Y*time });
    let tolerance = 0.002;
    let approximation = RootRigidPath::integrate_spatial(1.,RootTwistRateBounds { linear:1.,angular:1. },
        tolerance,tolerance,4096,sample).unwrap();
    assert!(approximation.path.spans().len()>1);
    assert!(approximation.origin_error_bound<=tolerance);
    assert!(approximation.angular_error_bound<=tolerance);
    // xi(t)=t*xi0 commutes with itself: exact endpoint is exp(xi0/2).
    let reference = RootRigidTwist { linear:DVec3::X,angular:DVec3::Y }.increment(0.5).unwrap();
    let actual = approximation.path.end_transform();
    assert!((actual.translation-reference.translation).length()<=approximation.origin_error_bound+1e-12);
    let error = (actual.rotation*reference.rotation.conjugate()).to_scaled_axis().length();
    assert!(error<=approximation.angular_error_bound+1e-12);
    assert!(RootRigidPath::integrate_spatial(1.,RootTwistRateBounds { linear:1.,angular:1. },
        tolerance,tolerance,1,sample).is_err());
    let constant = RootRigidPath::integrate_spatial(1.,RootTwistRateBounds { linear:0.,angular:0. },
        0.,0.,1,|_| Ok(RootRigidTwist { linear:DVec3::X,angular:DVec3::Y })).unwrap();
    assert_eq!(constant.path.spans().len(),1);
    equivalent(constant.path.end_transform(),RootRigidTwist { linear:DVec3::X,angular:DVec3::Y }.increment(1.).unwrap(),1e-12);
    assert!(RootRigidPath::integrate_spatial(1.,RootTwistRateBounds { linear:-1.,angular:0. },
        tolerance,tolerance,4096,sample).is_err());
    assert!(RootRigidPath::integrate_spatial(1.,RootTwistRateBounds { linear:1.,angular:1. },
        tolerance,tolerance,4096,|_| Err(AnimationError::NumericalOverflow)).is_err());
    assert!(RootRigidPath::integrate_spatial(0.,RootTwistRateBounds { linear:1.,angular:1. },
        tolerance,tolerance,0,|_| panic!("zero interval must not sample")).unwrap().path.spans().is_empty());
}

#[test]
fn noncommuting_varying_twists_cover_an_independently_authored_rigid_curve() {
    let reference = |time:f64| RootRigidTransform {
        rotation:DQuat::from_rotation_y(0.5*time*time)*DQuat::from_rotation_x(0.5*time),
        translation:DVec3::new(time.sin(),0.3*time*time,0.)
    };
    let sample = |time:f64| {
        let transform = reference(time);
        let angular = DVec3::Y*time+DQuat::from_rotation_y(0.5*time*time)*(DVec3::X*0.5);
        let linear = DVec3::new(time.cos(),0.6*time,0.)-angular.cross(transform.translation);
        Ok(RootRigidTwist { linear,angular })
    };
    // On [0,1]: |omega'| <= 1.5, |t|<1, |t'|<1.2, |t''|<1.2;
    // therefore |v'| <= 1.2 + 1.5*1 + 1.5*1.2 < 6.
    let approximation = RootRigidPath::integrate_spatial(1.,RootTwistRateBounds { linear:6.,angular:2. },
        0.002,0.002,4096,sample).unwrap();
    for span in approximation.path.spans() {
        for fraction in [0.,0.5,1.] {
            let time = span.start()+fraction*(span.end()-span.start());
            let expected = reference(time);
            let actual = span.sample(fraction).unwrap();
            assert!((actual.translation-expected.translation).length()<=approximation.origin_error_bound+1e-12);
            let angular_error=(actual.rotation*expected.rotation.conjugate()).to_scaled_axis().length();
            assert!(angular_error<=approximation.angular_error_bound+1e-12);
        }
    }
}

#[test]
fn approximation_error_transport_covers_shifted_target_origin_and_collision_points() {
    let reference = |time:f64| {
        RootRigidTwist { linear:DVec3::X,angular:DVec3::Y }.increment(0.5*time*time).unwrap()
    };
    let source = RootRigidPath::integrate_spatial(1.,RootTwistRateBounds { linear:1.,angular:1. },
        0.02,0.02,4096,|t| Ok(RootRigidTwist { linear:DVec3::X*t,angular:DVec3::Y*t })).unwrap();
    let basis = DQuat::from_rotation_z(0.4);
    let offset = DVec3::new(10.,3.,-2.);
    for scale in [0.,2.5] {
        let target = source.transformed(basis,scale,offset).unwrap();
        assert!(target.origin_error_bound>scale*source.origin_error_bound);
        assert_eq!(target.angular_error_bound,source.angular_error_bound);
        for span in target.path.spans() {
            for u in [0.,0.5,1.] {
                let time=span.start()+u*(span.end()-span.start());
                let h=reference(time);
                let rotation=(basis*h.rotation*basis.conjugate()).normalize();
                let expected=RootRigidTransform { rotation,
                    translation:scale*(basis*h.translation)+offset-rotation*offset };
                let actual=span.sample(u).unwrap();
                for point in [DVec3::ZERO,DVec3::X,DVec3::new(2.,-3.,4.)] {
                    let error=(actual.transform_point(point).unwrap()-expected.transform_point(point).unwrap()).length();
                    assert!(error<=target.point_error_bound(point).unwrap()+1e-12);
                }
            }
        }
    }
    assert!(source.point_error_bound(DVec3::NAN).is_err());
    let invalid=RootRigidApproximation { origin_error_bound:-1.,..source.clone() };
    assert!(invalid.transformed(basis,1.,offset).is_err());
    assert!(invalid.point_error_bound(DVec3::ZERO).is_err());
    let large=RootRigidApproximation { angular_error_bound:100.,origin_error_bound:0.,..source.clone() };
    assert_eq!(large.point_error_bound(DVec3::X).unwrap(),2.);
    assert!(source.transformed(basis,-1.,offset).is_err());
    assert!(source.transformed(basis,1.,offset).is_ok());
}

#[test]
fn compiled_clip_twist_rates_drive_bounded_integration_of_actual_root_motion() {
    let (_,clip)=linear_turn(Vec3::X*0.6);
    let original=clip.root_rigid_curve(0).unwrap().path(0.,1.,[true;3],256).unwrap();
    assert_eq!(original.spans().len(),1);
    let span=&original.spans()[0];
    let rates=span.twist_rate_bounds().unwrap().unwrap();
    let approximation=RootRigidPath::integrate_spatial(1.,rates,0.005,0.005,4096,|time| {
        let transform=span.sample(time)?;
        span.velocity(time)?.ok_or(AnimationError::RootRotationBudget)?.spatial_twist(transform)
    }).unwrap();
    let actual=approximation.path.end_transform();
    let expected=original.end_transform();
    assert!((actual.translation-expected.translation).length()<=approximation.origin_error_bound+1e-12);
    assert!((actual.rotation*expected.rotation.conjugate()).to_scaled_axis().length()<=approximation.angular_error_bound+1e-12);
    // Independently authored translation keys plus quarter-turn of bind pivot .6X.
    assert!((actual.translation-DVec3::new(1.6,0.2,0.6)).length()<=approximation.origin_error_bound+1e-7);
}

#[test]
fn blend_partition_unions_different_normalized_key_boundaries_and_stationary_paths() {
    let twist=RootRigidTwist { linear:DVec3::X,angular:DVec3::Y };
    let source=RootRigidPath::from_twists(&[(twist,0.25),(twist,0.75)],2).unwrap();
    let target=RootRigidPath::from_twists(&[(twist,1.),(twist,1.)],2).unwrap();
    let partition=source.partition_for_blend(&target,3).unwrap();
    let intervals:Vec<_>=partition.intervals.iter().map(|part|(part.start,part.end,part.source_span,part.target_span)).collect();
    assert_eq!(intervals,vec![(0.,0.25,Some(0),Some(0)),(0.25,0.5,Some(1),Some(0)),(0.5,1.,Some(1),Some(1))]);
    assert!(partition.steps.is_empty());
    assert!(source.partition_for_blend(&target,2).is_err());
    let stationary=RootRigidPath::from_twists(&[],0).unwrap();
    let partition=stationary.partition_for_blend(&target,2).unwrap();
    assert!(partition.intervals.iter().all(|part|part.source_span.is_none()));
    assert!(stationary.partition_for_blend(&stationary,1).unwrap().intervals[0].target_span.is_none());
    let zero=RootRigidTwist { linear:DVec3::ZERO,angular:DVec3::ZERO };
    let collapsed=RootRigidPath::from_twists(&[(zero,1e-300),(zero,1e300)],2).unwrap();
    assert!(collapsed.partition_for_blend(&stationary,4).is_err());
}

#[test]
fn common_frame_blend_bounds_and_retiming_drive_independent_fade_integral() {
    let base=RootRigidTwist {linear:DVec3::X,angular:DVec3::Y};
    let source=RootRigidPath::from_twists(&[(base,1.)],1).unwrap();
    let target=RootRigidPath::from_twists(&[(base.retimed(3.).unwrap(),1.)],1).unwrap();
    let source_bounds=source.spans()[0].twist_bounds().unwrap().unwrap().retimed(0.5).unwrap();
    let target_bounds=target.spans()[0].twist_bounds().unwrap().unwrap().retimed(2.).unwrap();
    let bounds=source_bounds.blend(target_bounds,0.,1.,1.).unwrap();
    let from=base.retimed(0.5).unwrap();
    let to=base.retimed(6.).unwrap();
    assert!(bounds.rates.linear>=(to.linear-from.linear).length());
    assert!(bounds.rates.angular>=(to.angular-from.angular).length());
    let approximation=RootRigidPath::integrate_spatial(1.,bounds.rates,0.02,0.02,4096,|t|from.blend(to,t)).unwrap();
    // Commuting xi(t)=(.5+5.5*t)*xi0 integrates exactly to exp(3.25*xi0).
    let expected=base.increment(3.25).unwrap();
    let actual=approximation.path.end_transform();
    assert!((actual.translation-expected.translation).length()<=approximation.origin_error_bound+1e-12);
    assert!((actual.rotation*expected.rotation.conjugate()).to_scaled_axis().length()<=approximation.angular_error_bound+1e-12);
    assert_eq!(from.blend(to,0.).unwrap().linear,from.linear);
    assert_eq!(from.blend(to,1.).unwrap().angular,to.angular);
    assert_eq!(source_bounds.retimed(0.).unwrap().rates.linear,0.);
    assert!(source_bounds.retimed(-1.).is_err());
    assert!(source_bounds.blend(target_bounds,0.,f64::NAN,1.).is_err());
    assert!(source_bounds.blend(target_bounds,0.,1.,0.).is_err());
    assert!(from.blend(to,2.).is_err());
}

#[test]
fn spatial_twist_similarity_keeps_shifted_origin_coupling_at_zero_scale() {
    let twist=RootRigidTwist {linear:DVec3::new(2.,3.,4.),angular:DVec3::Y*2.};
    let bounds=RootSpatialTwistBounds {linear_speed_bound:twist.linear.length(),angular_speed_bound:2.,
        rates:RootTwistRateBounds {linear:1.,angular:3.}};
    let offset=DVec3::X*4.;
    let transformed=twist.transformed(DQuat::IDENTITY,0.,offset).unwrap();
    assert_eq!(transformed.linear,DVec3::Z*8.);
    let mapped=bounds.transformed(DQuat::IDENTITY,0.,offset).unwrap();
    assert_eq!(mapped.linear_speed_bound,8.);
    assert_eq!(mapped.rates.linear,12.);
    for (basis,scale,offset) in [(DQuat::from_xyzw(0.,0.,0.,0.),1.,offset),
        (DQuat::IDENTITY,-1.,offset),(DQuat::IDENTITY,1.,DVec3::NAN)] {
        assert!(twist.transformed(basis,scale,offset).is_err());
        assert!(bounds.transformed(basis,scale,offset).is_err());
    }
    assert!(bounds.transformed(DQuat::IDENTITY,f64::MAX,offset).is_err());
    assert!(twist.transformed(DQuat::IDENTITY,1.,offset).is_ok());
}

#[test]
fn animator_rigid_fade_plan_stages_completion_tail_and_frozen_source_without_clock_publication() {
    let (rig,clip)=linear_turn(Vec3::X*0.6);
    let mut animator=Animator::new(clip.clone());
    animator.advance(&rig,0.2).unwrap();
    animator.transition_to_at_phase(clip.clone(),0.1,0.4).unwrap();
    let phase=animator.normalized_phase();
    let mut reference=animator.clone();
    let plan=animator.prepare_root_rigid_fade(&rig,0.25,[true;3],256).unwrap().unwrap();
    assert_eq!(animator.normalized_phase(),phase);
    assert_eq!(plan.frame.pose,reference.advance(&rig,0.25).unwrap().pose);
    assert_eq!(plan.candidate.normalized_phase(),reference.normalized_phase());
    assert!((plan.fade_wall_seconds-0.1).abs()<1e-7);
    assert!((plan.tail_wall_seconds-0.15).abs()<1e-7);
    assert_eq!(plan.weights,[0.,1.]);
    assert!(plan.source_fade.is_some() && plan.source_factor.is_some());
    assert!(plan.target_tail.duration()>0.14);
    assert!(animator.prepare_root_rigid_fade(&rig,0.25,[true;3],1).is_err());
    assert_eq!(animator.normalized_phase(),phase);
    animator.advance(&rig,0.04).unwrap();
    animator.transition_to_at_phase(clip,0.2,0.7).unwrap();
    let interrupted_phase=animator.normalized_phase();
    let plan=animator.prepare_root_rigid_fade(&rig,0.05,[true;3],256).unwrap().unwrap();
    assert!(plan.source_fade.is_none() && plan.source_factor.is_none());
    assert!(plan.target_tail.spans().is_empty());
    assert_eq!(animator.normalized_phase(),interrupted_phase);
    assert!(animator.prepare_root_rigid_fade(&rig,f32::NAN,[true;3],256).is_err());
    animator.set_speed(0.).unwrap();
    let plan=animator.prepare_root_rigid_fade(&rig,0.25,[true;3],256).unwrap().unwrap();
    assert!(plan.target_fade.spans().is_empty());
    assert!(plan.target_tail.spans().is_empty());
    assert_eq!(plan.frame.transition_weight,1.);
}

#[test]
fn spatial_path_append_matches_ordered_twists_and_transports_polynomial_pivot() {
    let a=RootRigidTwist {linear:DVec3::new(0.3,0.1,0.),angular:DVec3::X*0.5};
    let b=RootRigidTwist {linear:DVec3::Z*0.2,angular:DVec3::Y*0.8};
    let first=RootRigidPath::from_twists(&[(a,0.3)],1).unwrap();
    let second=RootRigidPath::from_twists(&[(b,0.4)],1).unwrap();
    let assembled=first.append_spatial(&second,2).unwrap();
    let reference=RootRigidPath::from_twists(&[(a,0.3),(b,0.4)],2).unwrap();
    equivalent(assembled.end_transform(),reference.end_transform(),1e-12);
    for (actual,expected) in assembled.spans().iter().zip(reference.spans()) {
        for u in [0.,0.3,1.] {equivalent(actual.sample(u).unwrap(),expected.sample(u).unwrap(),1e-12);}
    }
    let (_,clip)=linear_turn(Vec3::X*0.6);
    let polynomial=clip.root_rigid_curve(0).unwrap().path(0.,1.,[true;3],256).unwrap();
    let assembled=first.append_spatial(&polynomial,256).unwrap();
    for (source,target) in polynomial.spans().iter().zip(&assembled.spans()[1..]) {
        for u in [0.,0.3,1.] {
            equivalent(target.sample(u).unwrap(),source.sample(u).unwrap().compose(first.end_transform()).unwrap(),1e-12);
        }
    }
    assert!(first.append_spatial(&second,1).is_err());
    let first=RootRigidApproximation {path:first,origin_error_bound:0.01,angular_error_bound:0.02};
    let second=RootRigidApproximation {path:second,origin_error_bound:0.03,angular_error_bound:0.04};
    let assembled=first.append_spatial(&second,2).unwrap();
    assert!(assembled.origin_error_bound>0.04);
    assert_eq!(assembled.angular_error_bound,0.06);
    let approximate_first=first.path.end_transform();
    let approximate_next=second.path.end_transform();
    let true_first=RootRigidTransform {translation:approximate_first.translation+DVec3::X*0.01,
        rotation:DQuat::from_rotation_z(0.02)*approximate_first.rotation};
    let true_next=RootRigidTransform {translation:approximate_next.translation+DVec3::Y*0.03,
        rotation:DQuat::from_rotation_x(0.04)*approximate_next.rotation};
    let actual=true_next.compose(true_first).unwrap();
    let nominal=assembled.path.end_transform();
    assert!((actual.translation-nominal.translation).length()<=assembled.origin_error_bound);
    assert!((actual.rotation*nominal.rotation.conjugate()).to_scaled_axis().length()<=assembled.angular_error_bound);
    assert!(first.append_spatial(&second,1).is_err());
}

#[test]
fn compiled_spatial_blend_retimes_keys_and_bounds_a_commuting_fade() {
    let base = RootRigidTwist { linear: DVec3::new(0.2, 0.1, -0.3), angular: DVec3::Y * 0.7 };
    let source = RootRigidPath::from_twists(&[(base, 0.2), (base.retimed(2.).unwrap(), 0.3)], 2).unwrap();
    let target = RootRigidPath::from_twists(&[(base.retimed(3.).unwrap(), 2.)], 1).unwrap();
    let blended = source.blend_spatial(&target, [0., 1.], 1., 0.01, 0.01, 4096).unwrap();
    assert!(blended.origin_error_bound <= 0.01);
    assert!(blended.angular_error_bound <= 0.01);
    let integral = |t: f64| if t <= 0.4 { 0.5*t + 2.75*t*t } else { 0.64 + t + 2.5*t*t - 0.8 };
    for span in blended.path.spans() {
        for u in [0., 0.5, 1.] {
            let t = span.start() + (span.end()-span.start())*u;
            let expected = base.increment(integral(t)).unwrap();
            let actual = span.sample(u).unwrap();
            assert!((actual.translation-expected.translation).length() <= blended.origin_error_bound + 1e-12);
            assert!((actual.rotation*expected.rotation.conjugate()).to_scaled_axis().length() <= blended.angular_error_bound + 1e-12);
        }
    }
    assert!(source.blend_spatial(&target, [0., 1.], 1., 0.01, 0.01, 2).is_err());
    let stationary = RootRigidPath::from_twists(&[], 0).unwrap();
    let frozen = stationary.blend_spatial(&target, [0., 1.], 1., 0.01, 0.01, 4096).unwrap();
    let expected = base.increment(3.).unwrap();
    let actual = frozen.path.end_transform();
    assert!((actual.translation-expected.translation).length() <= frozen.origin_error_bound);
    assert!((actual.rotation*expected.rotation.conjugate()).to_scaled_axis().length() <= frozen.angular_error_bound);
    assert!(source.blend_spatial(&target, [0., 2.], 1., 0.01, 0.01, 4096).is_err());
}

#[test]
fn compiled_spatial_blend_preserves_noncommuting_key_order() {
    let source = RootRigidPath::from_twists(&[(RootRigidTwist { linear: DVec3::ZERO, angular: DVec3::X }, 0.2),
        (RootRigidTwist { linear: DVec3::ZERO, angular: DVec3::X * 2. }, 0.3)], 2).unwrap();
    let target = RootRigidPath::from_twists(&[(RootRigidTwist { linear: DVec3::ZERO, angular: DVec3::Y * 3. }, 2.)], 1).unwrap();
    let blended = source.blend_spatial(&target, [0.3, 0.3], 1., 0., 0., 2).unwrap();
    assert_eq!(blended.path.spans().len(), 2);
    let first = DQuat::from_scaled_axis(DVec3::new(0.35, 1.8, 0.) * 0.4);
    let second = DQuat::from_scaled_axis(DVec3::new(0.7, 1.8, 0.) * 0.6);
    equivalent(blended.path.end_transform(), RootRigidTransform { translation: DVec3::ZERO,
        rotation: (second*first).normalize() }, 1e-12);
    assert!((first*second*blended.path.end_transform().rotation.conjugate()).to_scaled_axis().length() > 0.01);
}

#[test]
fn rigid_fade_integration_keeps_explicit_frames_and_target_completion_tail() {
    let (rig, clip) = linear_turn(Vec3::X*0.6);
    let mut animator = Animator::new(clip.clone());
    animator.advance(&rig, 0.2).unwrap();
    animator.set_speed(2.).unwrap();
    animator.transition_to_at_phase(clip, 0.1, 0.4).unwrap();
    let phase = animator.normalized_phase();
    let plan = animator.prepare_root_rigid_fade(&rig, 0.25, [true;3], 256).unwrap().unwrap();
    let source_frame = RootRigidTransform {translation:DVec3::new(0.4,0.1,-0.2), rotation:DQuat::from_rotation_x(0.3)};
    let target_frame = RootRigidTransform {translation:DVec3::Z*0.5, rotation:DQuat::from_rotation_y(0.7)};
    let assembled = plan.integrate_spatial(Some(source_frame), target_frame, 0.01, 0.01, 4096).unwrap();
    assert!((assembled.path.duration()-0.25).abs()<1e-12);
    assert_eq!(animator.normalized_phase(), phase);
    let count = plan.target_tail.spans().len();
    let fade_spans = &assembled.path.spans()[..assembled.path.spans().len()-count];
    let fade_end = fade_spans.last().unwrap().sample(1.).unwrap();
    let tail_frame = target_frame.compose(plan.target_fade.end_transform()).unwrap();
    let mut different_from_restarting_axes = false;
    for (original, actual) in plan.target_tail.spans().iter().zip(&assembled.path.spans()[fade_spans.len()..]) {
        for u in [0.,0.5,1.] {
            let local = original.sample(u).unwrap();
            let rotation = tail_frame.rotation*local.rotation*tail_frame.rotation.conjugate();
            let spatial = RootRigidTransform {rotation:rotation.normalize(),
                translation:tail_frame.rotation*local.translation+tail_frame.translation-rotation*tail_frame.translation};
            equivalent(actual.sample(u).unwrap(),spatial.compose(fade_end).unwrap(),1e-12);
            let restarted = fade_end.compose(local).unwrap();
            different_from_restarting_axes |= (actual.sample(u).unwrap().translation-restarted.translation).length()>1e-4;
        }
        let speed = original.velocity(0.5).unwrap().unwrap().angular.length();
        let retimed_speed = actual.velocity(0.5).unwrap().unwrap().angular.length();
        assert!((retimed_speed-2.*speed).abs()<1e-6);
    }
    assert!(different_from_restarting_axes);
    assert!(plan.integrate_spatial(None,target_frame,0.01,0.01,4096).is_err());
    assert!(plan.integrate_spatial(Some(source_frame),target_frame,0.01,0.01,1).is_err());
    assert_eq!(animator.normalized_phase(),phase);
}

#[test]
fn rigid_path_retiming_preserves_geometry_and_changes_velocity() {
    let twist = RootRigidTwist {linear:DVec3::new(0.4,0.1,0.2),angular:DVec3::Y*0.7};
    let path = RootRigidPath::from_twists(&[(twist,0.5),(twist,1.)],2).unwrap();
    let slower = path.retimed(3.).unwrap();
    equivalent(slower.end_transform(),path.end_transform(),1e-12);
    for (a,b) in path.spans().iter().zip(slower.spans()) {
        for u in [0.,0.3,1.] {equivalent(a.sample(u).unwrap(),b.sample(u).unwrap(),1e-12);}
        assert_eq!(b.start(),a.start()*2.);
        assert_eq!(b.end(),a.end()*2.);
        let a = a.velocity(0.5).unwrap().unwrap();
        let b = b.velocity(0.5).unwrap().unwrap();
        assert!((b.linear-a.linear*0.5).length()<1e-12);
        assert!((b.angular-a.angular*0.5).length()<1e-12);
    }
    assert!(path.retimed(0.).is_err());
    assert!(path.retimed(f64::NAN).is_err());
    let stationary = RootRigidPath::from_twists(&[],0).unwrap().retimed(0.4).unwrap();
    assert_eq!(stationary.duration(),0.4);
    assert!(stationary.spans().is_empty());
}

#[test]
fn approximation_projection_expands_for_origin_and_angular_error_at_body_vertices() {
    let path = RootRigidPath::from_twists(&[(RootRigidTwist {
        linear:DVec3::ZERO, angular:DVec3::ZERO },1.)],1).unwrap();
    let approximation = RootRigidApproximation {path,origin_error_bound:0.2,angular_error_bound:0.3};
    let point = DVec3::X*2.;
    let normal = DVec3::Z*3.;
    let nominal = approximation.path.spans()[0].projection_bounds(point,normal).unwrap();
    let bounds = approximation.span_projection_bounds(0,point,normal).unwrap();
    let exact = (DQuat::from_rotation_y(0.3)*point-DVec3::Z*0.2).dot(normal);
    assert!(exact<nominal[0]);
    assert!(bounds[0]<=exact && exact<=bounds[1]);
    let unit = approximation.span_projection_bounds(0,point,DVec3::Z).unwrap();
    assert!((bounds[0]-unit[0]*3.).abs()<1e-10);
    assert!((bounds[1]-unit[1]*3.).abs()<1e-10);
    assert!(approximation.span_projection_bounds(1,point,normal).is_err());
    assert!(approximation.span_projection_bounds(0,point,DVec3::NAN).is_err());
    assert!(approximation.span_projection_bounds(0,DVec3::NAN,normal).is_err());
    let malformed = RootRigidApproximation {angular_error_bound:f64::NAN,..approximation};
    assert!(malformed.span_projection_bounds(0,point,normal).is_err());
}

#[test]
fn screw_field_enclosure_rejects_polynomial_motion_without_a_certificate() {
    let (_,clip)=linear_turn(Vec3::X*0.6);
    let path=clip.root_rigid_curve(0).unwrap().path(0.,1.,[true;3],256).unwrap();
    assert!(matches!(path.screw_field_enclosure(0,0.5,256),
        Err(AnimationError::RootRotationTransitionUnsupported)));
}

#[test]
fn imported_cubic_angular_derivative_has_outward_rational_bounds() {
    let p0=Vec3::X*0.6;
    let (_,clip)=clip(p0,vec![Vec3Key {time:0.,value:p0},Vec3Key {time:1.,value:p0}],
        vec![QuatKey {time:0.,value:Quat::IDENTITY},QuatKey {time:1.,value:Quat::IDENTITY}],
        TrackInterpolation {rotation:Interpolation::CubicSpline,..Default::default()},
        JointTangents {rotation:vec![[Vec4::ZERO,Vec4::new(3.,7.,-2.,1.)],
            [Vec4::new(-1.,4.,2.,0.3),Vec4::ZERO]],..Default::default()},Playback::Clamp);
    let path=clip.root_rigid_curve(0).unwrap().path(0.,1.,[true;3],256).unwrap()
        .transformed(DQuat::from_rotation_z(0.4),1.,DVec3::ZERO).unwrap();
    for span in path.spans() {
        let (control,left,_right)=span.rotation().cubic_velocity_inputs().unwrap();
        let bounds=span.rotation().enclosed_angular_derivative_bounds().unwrap().unwrap();
        println!("ANGULAR_RATE_ENCLOSURE {:?}",(control,span.start(),span.end(),
            [bounds.speed_bound(),bounds.acceleration_bound()]));
        for u in [0.,0.2,0.5,0.8,1.] {
            let bounds=span.rotation().cubic_angular_velocity_bounds(u).unwrap().unwrap();
            assert!(bounds.iter().all(|v|v[0].is_finite()&&v[1]-v[0]<1e-8));
            println!("CUBIC_ANGULAR_ENCLOSURE {:?}",(control,left.to_array(),span.start(),span.end(),u,bounds));
        }
        assert!(span.rotation().cubic_angular_velocity_bounds(f64::NAN).is_err());
    }
    let (_,linear)=linear_turn(p0);
    let path=linear.root_rigid_curve(0).unwrap().path(0.,1.,[true;3],256).unwrap();
    assert!(path.spans()[0].rotation().cubic_angular_velocity_bounds(0.5).unwrap().is_none());
}

#[test]
fn cubic_spatial_velocity_encloses_moving_pivot_for_all_extraction_axes() {
    let p0=Vec3::X*0.6;
    let (_,clip)=clip(p0,vec![Vec3Key {time:0.,value:p0},Vec3Key {time:1.,value:p0+Vec3::new(0.3,0.2,-0.1)}],
        vec![QuatKey {time:0.,value:Quat::IDENTITY},QuatKey {time:1.,value:Quat::IDENTITY}],
        TrackInterpolation {translation:Interpolation::CubicSpline,rotation:Interpolation::CubicSpline,..Default::default()},
        JointTangents {translation:vec![[Vec3::ZERO,Vec3::new(0.,3.,1.)],[Vec3::new(-0.5,-2.,2.),Vec3::ZERO]],
            rotation:vec![[Vec4::ZERO,Vec4::new(3.,7.,-2.,1.)],[Vec4::new(-1.,4.,2.,0.3),Vec4::ZERO]],
            ..Default::default()},Playback::Clamp);
    for mask in 0..8 {
        let axes=[mask&1!=0,mask&2!=0,mask&4!=0];
        let path=clip.root_rigid_curve(0).unwrap().path(0.,1.,axes,256).unwrap()
            .transformed(DQuat::from_rotation_z(0.4),1.5,DVec3::new(0.4,-0.2,0.3)).unwrap();
        for span in path.spans() {
            let (control,left,right)=span.rotation().cubic_velocity_inputs().unwrap();
            let angular=span.rotation().enclosed_angular_derivative_bounds().unwrap().unwrap();
            let bounds=span.enclosed_twist_bounds().unwrap().unwrap();
            println!("SPATIAL_RATE_ENCLOSURE {:?}",(span.additive.map(|v|v.to_array()),span.pivot.map(|v|v.to_array()),
                span.start(),span.end(),[angular.speed_bound(),angular.acceleration_bound()],
                [bounds.linear_speed_bound,bounds.angular_speed_bound,bounds.rates.linear,bounds.rates.angular]));
            for u in [0.,0.2,0.5,0.8,1.] {
                let e=span.spatial_twist_enclosure(u).unwrap().unwrap();
                assert!(e.linear_bounds().into_iter().chain(e.angular_bounds()).all(|v|v[0].is_finite()&&v[1]-v[0]<1e-7));
                let fractions=[(u-0.01_f64).max(0.),(u+0.01_f64).min(1.)];
                let range=span.spatial_twist_enclosure_range(fractions).unwrap().unwrap();
                for point in [fractions[0],u,fractions[1]] {
                    let sample=span.spatial_twist_enclosure(point).unwrap().unwrap();
                    for (whole,part) in range.linear_bounds().into_iter().chain(range.angular_bounds())
                        .zip(sample.linear_bounds().into_iter().chain(sample.angular_bounds())) {
                        assert!(whole[0]<=part[0] && part[1]<=whole[1]);
                    }
                    println!("CUBIC_RANGE_ENCLOSURE {:?}",(span.additive.map(|v|v.to_array()),span.pivot.map(|v|v.to_array()),
                        control,left.to_array(),right.to_array(),span.start(),span.end(),point,range.linear_bounds(),range.angular_bounds()));
                }
                println!("CUBIC_SPATIAL_ENCLOSURE {:?}",(span.additive.map(|v|v.to_array()),span.pivot.map(|v|v.to_array()),
                    control,left.to_array(),right.to_array(),span.start(),span.end(),u,e.linear_bounds(),e.angular_bounds()));
            }
        }
    }
}

#[test]
fn linear_and_held_spatial_fields_have_velocity_enclosures() {
    let (_,clip)=linear_turn(Vec3::X*0.6);
    for mask in 0..8 {
        let axes=[mask&1!=0,mask&2!=0,mask&4!=0];
        let path=clip.root_rigid_curve(0).unwrap().path(0.,1.,axes,256).unwrap()
            .transformed(DQuat::from_rotation_z(0.4),1.5,DVec3::new(0.4,-0.2,0.3)).unwrap();
        for span in path.spans() {
            let (from,axis,left,right)=span.rotation().arc_velocity_inputs().unwrap();
            for u in [0.,0.2,0.5,0.8,1.] {
                let e=span.spatial_twist_enclosure(u).unwrap().unwrap();
                let fractions=[(u-0.01_f64).max(0.),(u+0.01_f64).min(1.)];
                let range=span.spatial_twist_enclosure_range(fractions).unwrap().unwrap();
                for point in [fractions[0],u,fractions[1]] {
                    println!("ARC_RANGE_ENCLOSURE {:?}",(span.additive.map(|v|v.to_array()),span.pivot.map(|v|v.to_array()),
                        (from.to_array(),axis.to_array(),left.to_array(),right.to_array()),span.start(),span.end(),point,
                        range.linear_bounds(),range.angular_bounds()));
                }
                assert!(span.spatial_twist_enclosure_range([0.8,0.2]).is_err());
                assert!(span.spatial_twist_enclosure_range([0.,f64::NAN]).is_err());
                println!("ARC_SPATIAL_ENCLOSURE {:?}",(span.additive.map(|v|v.to_array()),span.pivot.map(|v|v.to_array()),
                    (from.to_array(),axis.to_array(),left.to_array(),right.to_array()),span.start(),span.end(),u,
                    e.linear_bounds(),e.angular_bounds()));
            }
        }
    }
    let p0=Vec3::ZERO;
    let (_,clip)=self::clip(p0,vec![Vec3Key {time:0.,value:p0},Vec3Key {time:1.,value:Vec3::X}],
        vec![],TrackInterpolation::default(),JointTangents::default(),Playback::Clamp);
    let path=clip.root_rigid_curve(0).unwrap().path(0.,1.,[true;3],256).unwrap();
    for span in path.spans() {
        let e=span.spatial_twist_enclosure(0.5).unwrap().unwrap();
        for (i,value) in [1.,0.,0.].into_iter().enumerate() {
            assert!(e.linear_bounds()[i][0]<=value && value<=e.linear_bounds()[i][1]);
            assert!(e.angular_bounds()[i][0]<=0. && 0.<=e.angular_bounds()[i][1]);
        }
    }
}

#[test]
fn sampled_velocity_uncertainty_is_accumulated_and_cannot_be_refined_away() {
    let actual=RootRigidTwist {linear:DVec3::X,angular:DVec3::Y};
    let nominal=actual.retimed(0.9).unwrap();
    let enclosure=actual.enclosure().unwrap();
    let error=enclosure.error_bounds(nominal).unwrap();
    assert!(error.linear_bound()>=0.1_f64.next_down());
    assert!(error.angular_bound()>=0.1_f64.next_down());
    assert_eq!(actual.enclosure().unwrap().error_bounds(actual).unwrap().linear_bound(),0.);
    let approximation=RootRigidPath::integrate_spatial_enclosed(1.,RootTwistRateBounds {linear:0.,angular:0.},
        0.3,0.11,64,|_|Ok((nominal,enclosure))).unwrap();
    for span in approximation.path.spans() {
        for u in [0.,0.5,1.] {
            let time=span.start()+u*(span.end()-span.start());
            let reference=actual.increment(time).unwrap();
            let sampled=span.sample(u).unwrap();
            assert!((reference.translation-sampled.translation).length()<=approximation.origin_error_bound);
            assert!((reference.rotation*sampled.rotation.conjugate()).to_scaled_axis().length()<=approximation.angular_error_bound+1e-14);
        }
    }
    assert!(RootRigidPath::integrate_spatial_enclosed(1.,RootTwistRateBounds {linear:0.,angular:0.},
        0.01,0.01,64,|_|Ok((nominal,enclosure))).is_err());
}

#[test]
fn outward_bound_transport_covers_reflected_frames_and_weight_derivatives() {
    let source = RootSpatialTwistBounds {linear_speed_bound: 2., angular_speed_bound: 3., rates: RootTwistRateBounds {linear: 5., angular: 7.}};
    let frame = RootRigidEnclosure::from_transform(RootRigidTransform {translation: DVec3::new(2.,-3.,4.), rotation: DQuat::from_rotation_y(0.7)}).unwrap();
    let mapped = source.enclosed_transformed(frame,-2.).unwrap();
    assert!(mapped.linear_speed_bound >= 31.);
    assert!(mapped.rates.linear >= 73.);
    assert_eq!(mapped.angular_speed_bound,3.);
    let retimed = source.enclosed_retimed_between(0.7,0.3).unwrap();
    assert!(retimed.linear_speed_bound >= 2.*(0.7/0.3));
    assert!(retimed.rates.angular >= 7.*(0.7/0.3)*(0.7/0.3));
    let target = RootSpatialTwistBounds {linear_speed_bound: 4., angular_speed_bound: 6., rates: RootTwistRateBounds {linear: 10., angular: 14.}};
    for weights in [[0.,1.],[1.,0.],[0.3,0.3]] {
        let blend = source.enclosed_blend(target,weights,0.5).unwrap();
        let rate=(weights[1]-weights[0]).abs()/0.5;
        for w in weights {
            assert!(blend.rates.linear >= (1.-w)*5.+w*10.+rate*6.);
            assert!(blend.rates.angular >= (1.-w)*7.+w*14.+rate*9.);
        }
    }
    assert_eq!(source.enclosed_retimed_between(0.,1.).unwrap().rates.linear,0.);
    assert!(source.enclosed_retimed_between(1.,0.).is_err());
    assert!(source.enclosed_blend(target,[0.,f64::NAN],1.).is_err());
    assert!(source.enclosed_transformed(frame,f64::MAX).is_err());
}

#[test]
fn outward_integrator_preserves_clocks_and_refines_large_screw_angles() {
    let twist=RootRigidTwist {linear:DVec3::new(0.4,-0.2,0.3),angular:DVec3::Y*7.};
    let result=RootRigidPath::integrate_spatial_outward(0.7,RootTwistRateBounds {linear:0.,angular:0.},0.,0.,32,
        |_|Ok((twist,twist.enclosure()?))).unwrap();
    assert!(result.path.spans().len()>=8);
    assert_eq!(result.path.duration(),0.7);
    assert_eq!(result.origin_error_bound,0.);
    assert_eq!(result.angular_error_bound,0.);
    for (i,span) in result.path.spans().iter().enumerate() {
        assert_eq!(span.end(),0.7*((i+1) as f64/result.path.spans().len() as f64));
        let bounds=result.path.screw_field_enclosure(i,1.,32).unwrap().translation_bounds();
        let exact=twist.increment(span.end()).unwrap().translation;
        for j in 0..3 {assert!(exact[j]>=bounds[j][0]-1e-14 && exact[j]<=bounds[j][1]+1e-14);}
    }
    assert!(RootRigidPath::integrate_spatial_outward(0.7,RootTwistRateBounds {linear:0.,angular:0.},0.,0.,1,
        |_|Ok((twist,twist.enclosure()?))).is_err());
    let actual=RootRigidTwist {linear:DVec3::X,angular:DVec3::ZERO};
    let nominal=actual.retimed(0.9).unwrap();
    let biased=RootRigidPath::integrate_spatial_outward(0.7,RootTwistRateBounds {linear:0.,angular:0.},0.08,0.,32,
        |_|Ok((nominal,actual.enclosure()?))).unwrap();
    assert!(biased.origin_error_bound>=0.7*(1.-0.9));
    assert!(biased.origin_error_bound<=0.08);
    assert_eq!(biased.angular_error_bound,0.);
    assert!(RootRigidPath::integrate_spatial_outward(0.7,RootTwistRateBounds {linear:0.,angular:0.},0.01,0.,32,
        |_|Ok((nominal,actual.enclosure()?))).is_err());
}

#[test]
fn outward_integrator_encloses_independent_accelerating_translation() {
    let result=RootRigidPath::integrate_spatial_outward(0.7,RootTwistRateBounds {linear:1.,angular:0.},0.01,1e-12,128,
        |time| {
            let nominal=RootRigidTwist {linear:DVec3::X*(1.+time),angular:DVec3::ZERO};
            // 1+time is rounded in the nominal sample: enclose the exact sum
            // through independently enclosed constant fields and interpolation.
            let base=RootRigidTwist {linear:DVec3::X,angular:DVec3::ZERO}.enclosure()?;
            let target=RootRigidTwist {linear:DVec3::X*2.,angular:DVec3::ZERO}.enclosure()?;
            Ok((nominal,base.blended(&target,[0.,1.],time)?))
        }).unwrap();
    assert!(result.path.spans().len()>1);
    assert!(result.origin_error_bound<=0.01);
    assert!(result.angular_error_bound<=1e-12);
    for span in result.path.spans() {
        for u in [0.,0.25,0.5,0.75,1.] {
            let time=span.start()+u*(span.end()-span.start());
            let reference=time+0.5*time*time;
            let observed=span.sample(u).unwrap().translation.x;
            assert!((reference-observed).abs()<=result.origin_error_bound);
        }
    }
}

#[test]
fn spatial_time_ranges_validate_span_boundaries_without_clamping_invalid_times() {
    let (_,clip)=linear_turn(Vec3::X*0.6);
    let path=clip.root_rigid_curve(0).unwrap().path(0.2,0.8,[true;3],256).unwrap();
    for span in path.spans() {
        let enclosure=span.spatial_twist_enclosure_at_times([span.start(),span.end()]).unwrap().unwrap();
        for u in [0.,0.3,0.5,1.] {
            let point=span.spatial_twist_enclosure(u).unwrap().unwrap();
            for (whole,part) in enclosure.linear_bounds().into_iter().chain(enclosure.angular_bounds())
                .zip(point.linear_bounds().into_iter().chain(point.angular_bounds())) {
                assert!(whole[0]<=part[0] && part[1]<=whole[1]);
            }
        }
        assert!(span.spatial_twist_enclosure_at_times([span.start().next_down(),span.end()]).is_err());
        assert!(span.spatial_twist_enclosure_at_times([span.start(),span.end().next_up()]).is_err());
        assert!(span.spatial_twist_enclosure_at_times([span.end(),span.start()]).is_err());
    }
}

#[test]
fn world_point_envelope_encloses_rotation_chord_scaling_and_error_sum() {
    let path=RootRigidPath::from_twists(&[],0).unwrap();
    for angle in [0.,1e-12,0.2,1.,2.,std::f64::consts::PI.next_down(),std::f64::consts::PI,7.] {
        let approximation=RootRigidApproximation {path:path.clone(),origin_error_bound:0.01,angular_error_bound:angle};
        for point in [DVec3::ZERO,DVec3::new(0.4,-0.1,0.2),DVec3::new(2.,3.,4.)] {
            for scale in [-2.,0.,0.5,2.] {
                let bound=approximation.enclosed_world_point_error_bound(point,scale,0.003).unwrap();
                println!("WORLD_POINT_ENCLOSURE {:?}",(angle,point.to_array(),scale,0.01,0.003,bound));
                let reference=scale.abs()*(0.01+2.*(0.5*angle.min(std::f64::consts::PI)).sin()*point.length())+0.003;
                assert!(bound>=reference || reference-bound<1e-14);
            }
        }
        assert!(approximation.enclosed_world_point_error_bound(DVec3::splat(f64::MAX),1.,0.).is_err() || angle==0.);
        assert!(approximation.enclosed_world_point_error_bound(DVec3::ZERO,1.,-1.).is_err());
    }
    let exact=RootRigidApproximation {path,origin_error_bound:0.,angular_error_bound:0.};
    assert_eq!(exact.enclosed_world_point_error_bound(DVec3::ONE,1.,0.).unwrap(),0.);
}

#[test]
fn inverse_similarity_encloses_affine_corner_sums_and_reflections() {
    let transform=RootRigidTransform {translation:DVec3::new(0.6,-0.1,0.2),rotation:DQuat::from_rotation_z(0.4)};
    let frame=RootRigidEnclosure::from_transform(transform).unwrap();
    let edges=[DVec3::new(0.4,0.03,-0.02),DVec3::new(0.01,0.1,0.04),DVec3::new(-0.02,0.01,0.2)];
    let approximation=RootRigidApproximation {path:RootRigidPath::from_twists(&[],0).unwrap(),origin_error_bound:0.01,angular_error_bound:0.2};
    for scale in [-2.,0.5,2.] {
        for signs in 0..8 {
            let points=std::array::from_fn::<_,3,_>(|i|if signs&(1<<i)==0 {-edges[i]}else{edges[i]});
            let bounds=frame.inverse_similarity_point_sum_bounds(&points,scale).unwrap();
            let reference=transform.rotation.conjugate()*(points.into_iter().sum::<DVec3>()-transform.translation)/scale;
            for i in 0..3 {assert!(bounds[i][0]<=reference[i]&&reference[i]<=bounds[i][1]);}
            let error=approximation.enclosed_world_point_box_error_bound(bounds,scale,0.003).unwrap();
            println!("INVERSE_POINT_ENCLOSURE {:?}",(points.map(|v|v.to_array()),transform.translation.to_array(),transform.rotation.to_array(),scale,bounds,error));
        }
    }
    assert!(frame.inverse_similarity_point_sum_bounds(&edges,0.).is_err());
    assert!(frame.inverse_similarity_point_sum_bounds(&[DVec3::splat(f64::NAN)],1.).is_err());
    assert!(approximation.enclosed_world_point_box_error_bound([[1.,-1.];3],1.,0.).is_err());
}

#[test]
fn ordered_screw_speed_caps_enclose_initial_and_interior_point_velocities() {
    let segments=[
        (RootRigidTwist {linear:DVec3::new(0.3,-0.1,0.2),angular:DVec3::X*0.7},0.2),
        (RootRigidTwist {linear:DVec3::new(-0.2,0.4,0.1),angular:DVec3::Y*0.8},0.3),
        (RootRigidTwist {linear:DVec3::new(0.1,0.2,-0.3),angular:DVec3::Z*0.5},0.4)];
    let path=RootRigidPath::from_twists(&segments,3).unwrap();
    let points=[DVec3::new(0.4,0.1,-0.2),DVec3::new(-0.3,0.2,0.5),DVec3::Y*1000.];
    for scale in [-2.,0.5,2.] {
        for point in points {
            let box_point=point.to_array().map(|v|[v,v]);
            let caps=path.enclosed_screw_point_speed_bounds(&[box_point],scale,3).unwrap();
            for (index,span) in path.spans().iter().enumerate() {
                let (twist,_)=span.screw.unwrap();
                for u in [0.,0.3,0.8,1.] {
                    let position=span.sample(u).unwrap().transform_point(point).unwrap();
                    let speed=(twist.angular.cross(position)+twist.linear).length()*(span.end()-span.start())*scale.abs();
                    assert!(speed<=caps[index]+1e-12);
                }
                println!("SCREW_SPEED_ENCLOSURE {:?}",(segments.map(|(t,d)|(t.linear.to_array(),t.angular.to_array(),d)),
                    path.spans().iter().map(|s|(s.start(),s.end())).collect::<Vec<_>>(),index,point.to_array(),scale,caps[index]));
            }
        }
    }
    assert!(path.enclosed_screw_point_speed_bounds(&[],1.,3).is_err());
    assert!(path.enclosed_screw_point_speed_bounds(&[[[0.,0.];3]],1.,2).is_err());
    let stationary=RootRigidPath::from_twists(&[(RootRigidTwist {linear:DVec3::ZERO,angular:DVec3::ZERO},1.)],1).unwrap();
    assert_eq!(stationary.enclosed_screw_point_speed_bounds(&[[[0.,0.];3]],1.,1).unwrap(),vec![0.]);
}

#[test]
fn enclosing_point_boxes_survive_signed_world_similarity_maps() {
    let transform=RootRigidTransform {translation:DVec3::new(2.,-0.3,0.7),rotation:DQuat::from_rotation_y(0.6)};
    let frame=RootRigidEnclosure::from_transform(transform).unwrap();
    let point=[[-0.3,0.4],[0.2,0.5],[-0.1,0.6]];
    for scale in [-2.,0.,0.5,2.] {
        let image=frame.similarity_point_box_bounds(point,scale).unwrap();
        for corner in 0..8 {
            let p=DVec3::from_array(std::array::from_fn(|i|point[i][usize::from(corner&(1<<i)!=0)]));
            let reference=transform.translation+scale*(transform.rotation*p);
            for i in 0..3 {assert!(image[i][0]<=reference[i]&&reference[i]<=image[i][1]);}
        }
        println!("POINT_BOX_IMAGE {:?}",(point,transform.translation.to_array(),transform.rotation.to_array(),scale,image));
    }
    assert!(frame.transform_point_box_bounds([[1.,-1.];3]).is_err());
}

#[test]
fn prepared_prefix_query_benchmark_includes_preparation_work() {
    let twist=RootRigidTwist {linear:DVec3::new(0.1,-0.2,0.3),angular:DVec3::Y*0.4};
    let segments=vec![(twist,0.01);64];
    let path=RootRigidPath::from_twists(&segments,64).unwrap();
    for round in 0..3 {
        let start=std::time::Instant::now();
        for index in 0..64 {std::hint::black_box(path.screw_field_enclosure(index,0.37,64).unwrap());}
        let replay=start.elapsed();
        let start=std::time::Instant::now();
        let cache=path.prepare_screw_enclosures(64).unwrap();
        for index in 0..64 {std::hint::black_box(cache.sample(index,0.37).unwrap());}
        let prepared=start.elapsed();
        println!("PREFIX_QUERY_TIMING {:?}",(round,64,replay.as_nanos(),prepared.as_nanos()));
    }
}

#[test]
fn canonical_coordinate_projection_range_retains_all_segment_directions() {
    let first=RootRigidTwist {linear:DVec3::new(0.3,0.,0.1),angular:DVec3::Y*0.4};
    let second=RootRigidTwist {linear:DVec3::new(-0.2,0.1,0.2),angular:DVec3::Y*0.7};
    let path=RootRigidPath::from_twists(&[(first,0.2),(second,0.3)],2).unwrap();
    let cache=path.prepare_screw_enclosures(2).unwrap();
    assert_eq!(cache.coordinate_velocity_range(1),Some([0.,0.1]));
    assert_eq!(cache.coordinate_velocity_range(0),None);
    assert_eq!(cache.coordinate_velocity_range(3),None);
    let changed=RootRigidPath::from_twists(&[(first,0.2),(RootRigidTwist {angular:DVec3::X*1e-300,..second},0.3)],2).unwrap();
    assert_eq!(changed.prepare_screw_enclosures(2).unwrap().coordinate_velocity_range(1),None);
}

#[test]
fn prepared_coordinate_ranges_cover_late_reversal_and_empty_paths() {
    let first=RootRigidTwist {linear:DVec3::new(0.2,0.1,-0.3),angular:DVec3::Y*0.4};
    let last=RootRigidTwist {linear:DVec3::new(-0.1,-0.2,0.3),..first};
    let mut segments=vec![(first,0.001);64];segments.push((last,0.001));
    let path=RootRigidPath::from_twists(&segments,65).unwrap();
    let cache=path.prepare_screw_enclosures(65).unwrap();
    assert_eq!(cache.coordinate_velocity_range(1),Some([-0.2,0.1]));
    let zero=RootRigidPath::from_twists(&[],0).unwrap();
    let cache=zero.prepare_screw_enclosures(0).unwrap();
    for axis in 0..3 {assert_eq!(cache.coordinate_velocity_range(axis),Some([0.,0.]));}
}
