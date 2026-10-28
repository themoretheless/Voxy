use super::*;
fn rig() -> Skeleton {
    Skeleton::new(
        (0..3)
            .map(|i| Joint {
                name: Arc::from(format!("joint-{i}")),
                parent: (i > 0).then_some((i - 1) as u16),
                bind_local: Transform {
                    translation: Vec3::new(i as f32 * 0.25, -0.0, 0.),
                    rotation: Quat::from_rotation_y(i as f32 * 0.125),
                    scale: Vec3::ONE,
                },
                inverse_bind: Mat4::IDENTITY * 2.0,
            })
            .collect(),
    )
    .unwrap()
}
fn clip(rig: &Skeleton) -> AnimationClip {
    let mut tracks = vec![JointTrack::default(); 3];
    tracks[0].translations = vec![
        Vec3Key {
            time: 0.,
            value: Vec3::new(0.25, -0.0, 0.),
        },
        Vec3Key {
            time: 1.,
            value: Vec3::new(1.25, 0.5, 0.),
        },
    ];
    tracks[1].rotations = vec![
        QuatKey {
            time: 0.,
            value: Quat::IDENTITY,
        },
        QuatKey {
            time: 1.,
            value: Quat::from_rotation_z(0.75),
        },
    ];
    tracks[2].rotations = vec![
        QuatKey {
            time: 0.,
            value: Quat::from_rotation_x(0.125),
        },
        QuatKey {
            time: 1.,
            value: Quat::from_rotation_x(0.75),
        },
    ];
    tracks[2].scales = vec![
        Vec3Key {
            time: 0.,
            value: Vec3::ONE,
        },
        Vec3Key {
            time: 1.,
            value: Vec3::new(1.125, 0.875, 1.),
        },
    ];
    let mut modes = vec![TrackInterpolation::default(); 3];
    modes[0].translation = Interpolation::CubicSpline;
    modes[1].rotation = Interpolation::Step;
    modes[2].rotation = Interpolation::CubicSpline;
    let mut tangents = vec![JointTangents::default(); 3];
    tangents[0].translation = vec![
        [Vec3::ZERO, Vec3::new(0.25, 0.5, 0.)],
        [Vec3::new(1.25, 0.25, 0.), Vec3::ZERO],
    ];
    tangents[2].rotation = vec![
        [Vec4::ZERO, Vec4::new(0.125, 0., 0., 0.)],
        [Vec4::new(0.25, 0., 0., 0.), Vec4::ZERO],
    ];
    AnimationClip::new_with_tangents("source", 1., Playback::Loop, tracks, modes, tangents, rig)
        .unwrap()
}
#[test]
fn shared_rigs_outer_clip_arcs_clone_caches_and_exact_compiled_receipts_survive_120_phases() {
    let rig = rig();
    let independent_rig = Skeleton::new(rig.joints().to_vec()).unwrap();
    let first = Arc::new(clip(&rig));
    for joint in 0..3 {
        first.root_rigid_curve(joint).unwrap();
    }
    let clone = Arc::new((*first).clone());
    let separate = Arc::new(clip(&rig));
    let skeletons = vec![rig.clone(), rig.clone(), independent_rig];
    let clips = vec![first.clone(), clone, first.clone(), separate];
    let limits = AnimationAssetLimits::default();
    let saved = AnimationAssetCheckpoint::capture(&skeletons, &clips, limits).unwrap();
    let json = saved.to_json(limits.max_bytes).unwrap();
    let decoded = AnimationAssetCheckpoint::from_json(&json, limits.max_bytes).unwrap();
    let restored = decoded.prepare(limits).unwrap();
    assert!(Arc::ptr_eq(
        &restored.skeletons[0].joints,
        &restored.skeletons[1].joints
    ));
    assert!(!Arc::ptr_eq(
        &restored.skeletons[0].joints,
        &restored.skeletons[2].joints
    ));
    assert!(Arc::ptr_eq(&restored.clips[0], &restored.clips[2]));
    assert!(!Arc::ptr_eq(&restored.clips[0], &restored.clips[1]));
    assert!(same_group(&restored.clips[0], &restored.clips[1]));
    assert!(!same_group(&restored.clips[0], &restored.clips[3]));
    assert!(Arc::ptr_eq(
        &restored.clips[0].rig,
        &restored.skeletons[0].joints
    ));
    assert_eq!(
        AnimationAssetCheckpoint::capture(&restored.skeletons, &restored.clips, limits)
            .unwrap()
            .to_json(limits.max_bytes)
            .unwrap(),
        json
    );
    for frame in 0..120 {
        let phase = f64::from(frame) / 119.;
        let a = first.try_sample_phase(&rig, phase).unwrap();
        let b = restored.clips[0]
            .try_sample_phase(&restored.skeletons[0], phase)
            .unwrap();
        assert_eq!(a.checkpoint(), b.checkpoint());
        let a = first
            .try_sample_phase64(&rig, phase)
            .unwrap()
            .skin_matrices(&rig)
            .unwrap();
        let b = restored.clips[0]
            .try_sample_phase64(&restored.skeletons[0], phase)
            .unwrap()
            .skin_matrices(&restored.skeletons[0])
            .unwrap();
        for (a, b) in a.iter().zip(&b) {
            assert_eq!(
                a.to_cols_array().map(f64::to_bits),
                b.to_cols_array().map(f64::to_bits)
            );
        }
        for joint in 0..3 {
            let a = first
                .motion_curve(joint)
                .unwrap()
                .integral(phase, phase + 0.02, 0.25, 0.75);
            let b = restored.clips[0].motion_curve(joint).unwrap().integral(
                phase,
                phase + 0.02,
                0.25,
                0.75,
            );
            assert_eq!(
                a.to_array().map(f64::to_bits),
                b.to_array().map(f64::to_bits)
            );
        }
    }
}
#[test]
fn consumed_cache_budgets_cached_failures_and_uninitialized_cells_keep_future_behavior() {
    let count = 17000usize;
    let rig = Skeleton::new(
        (0..5)
            .map(|i| Joint {
                name: Arc::from(format!("b{i}")),
                parent: None,
                bind_local: Transform::IDENTITY,
                inverse_bind: Mat4::IDENTITY,
            })
            .collect(),
    )
    .unwrap();
    let track = JointTrack {
        translations: (0..count)
            .map(|i| Vec3Key {
                time: i as f32 / (count - 1) as f32,
                value: Vec3::splat(0.125),
            })
            .collect(),
        rotations: (0..count)
            .map(|i| QuatKey {
                time: i as f32 / (count - 1) as f32,
                value: Quat::IDENTITY,
            })
            .collect(),
        scales: Vec::new(),
    };
    let clip =
        Arc::new(AnimationClip::new("budget", 1., Playback::Clamp, vec![track; 5], &rig).unwrap());
    for joint in [2, 1] {
        clip.motion_curve(joint).unwrap();
    }
    assert_eq!(
        clip.motion_curve(3).unwrap_err(),
        AnimationError::RootMotionBudget
    );
    for joint in [2, 0, 1] {
        clip.root_rotation_curve(joint).unwrap();
    }
    assert_eq!(
        clip.root_rotation_curve(3).unwrap_err(),
        AnimationError::RootRotationBudget
    );
    clip.root_rigid_curve(2).unwrap();
    assert_eq!(
        clip.root_rigid_curve(3).unwrap_err(),
        AnimationError::RootMotionBudget
    );
    assert!(clip.motion_curves[4].get().is_none() && clip.rotation_curves[4].get().is_none());
    let limits = AnimationAssetLimits::default();
    let saved = AnimationAssetCheckpoint::capture(
        std::slice::from_ref(&rig),
        std::slice::from_ref(&clip),
        limits,
    )
    .unwrap();
    let json = saved.to_json(limits.max_bytes).unwrap();
    let restored = AnimationAssetCheckpoint::from_json(&json, limits.max_bytes)
        .unwrap()
        .prepare(limits)
        .unwrap();
    let loaded = &restored.clips[0];
    assert_eq!(loaded.motion_cache_keys.load(Ordering::Relaxed), 51000);
    assert_eq!(loaded.rotation_cache_keys.load(Ordering::Relaxed), 51000);
    assert!(loaded.motion_curves[4].get().is_none() && loaded.rotation_curves[4].get().is_none());
    assert_eq!(
        loaded.motion_curve(3).unwrap_err(),
        AnimationError::RootMotionBudget
    );
    assert_eq!(
        loaded.root_rotation_curve(3).unwrap_err(),
        AnimationError::RootRotationBudget
    );
    assert_eq!(
        loaded.motion_curve(4).unwrap_err(),
        clip.motion_curve(4).unwrap_err()
    );
    assert_eq!(
        loaded.root_rotation_curve(4).unwrap_err(),
        clip.root_rotation_curve(4).unwrap_err()
    );
    assert_eq!(
        AnimationAssetCheckpoint::capture(&restored.skeletons, &restored.clips, limits).unwrap(),
        AnimationAssetCheckpoint::capture(
            std::slice::from_ref(&rig),
            std::slice::from_ref(&clip),
            limits
        )
        .unwrap()
    );
}
#[test]
fn malformed_source_ownership_receipts_and_policies_reject_without_changing_live_clip() {
    let rig = rig();
    let clip = Arc::new(clip(&rig));
    clip.root_rigid_curve(1).unwrap();
    let limits = AnimationAssetLimits::default();
    let saved = AnimationAssetCheckpoint::capture(
        std::slice::from_ref(&rig),
        std::slice::from_ref(&clip),
        limits,
    )
    .unwrap();
    let before = saved.to_json(limits.max_bytes).unwrap();
    for case in 0..8 {
        let mut bad = saved.clone();
        match case {
            0 => bad.version = 2,
            1 => bad.rigs[0][1].parent = Some(1),
            2 => bad.groups[0].tracks[0].translations[0][0] = f32::NAN.to_bits(),
            3 => bad.groups[0].motion_keys += 1,
            4 => bad.clips[0] = 999,
            5 => bad.allocations[0] = 999,
            6 => bad.strings.push("unused-name".into()),
            7 => bad.groups[0].constants[0] = Some([0; 10]),
            _ => unreachable!(),
        }
        assert!(bad.prepare(limits).is_err(), "case {case}");
    }
    let mut json: serde_json::Value = serde_json::from_str(&before).unwrap();
    json["groups"][0]["root"]["duration"] = serde_json::json!(1_u64);
    let bad = AnimationAssetCheckpoint::from_json(&json.to_string(), limits.max_bytes).unwrap();
    assert!(matches!(bad.prepare(limits), Err(Error::CompilerMismatch)));
    for case in 0..7 {
        let mut tight = limits;
        match case {
            0 => tight.max_rigs = 0,
            1 => tight.max_total_joints = 0,
            2 => tight.max_groups = 0,
            3 => tight.max_clip_allocations = 0,
            4 => tight.max_references = 0,
            5 => tight.max_keys = 0,
            6 => tight.max_string_bytes = 0,
            _ => unreachable!(),
        }
        assert!(saved.prepare(tight).is_err());
        assert!(
            AnimationAssetCheckpoint::capture(
                std::slice::from_ref(&rig),
                std::slice::from_ref(&clip),
                tight
            )
            .is_err()
        );
    }
    assert!(saved.to_json(before.len() - 1).is_err());
    assert!(AnimationAssetCheckpoint::from_json(&before, before.len() - 1).is_err());
    assert_eq!(
        AnimationAssetCheckpoint::capture(
            std::slice::from_ref(&rig),
            std::slice::from_ref(&clip),
            limits
        )
        .unwrap()
        .to_json(limits.max_bytes)
        .unwrap(),
        before
    );
    clip.motion_cache_keys.fetch_add(1, Ordering::Relaxed);
    assert!(matches!(
        AnimationAssetCheckpoint::capture(
            std::slice::from_ref(&rig),
            std::slice::from_ref(&clip),
            limits
        ),
        Err(Error::Changing)
    ));
    clip.motion_cache_keys.fetch_sub(1, Ordering::Relaxed);
}

#[test]
fn skeleton_only_and_empty_archives_preserve_accepted_near_unit_quaternion_bits() {
    let rotation = Quat::from_array([0., -0.0, 0.0001, 1.0000001]);
    assert!(rotation.is_normalized());
    let rig = Skeleton::new(vec![Joint {
        name: Arc::from("near-unit"),
        parent: None,
        bind_local: Transform {
            translation: Vec3::new(-0.0, f32::from_bits(1), 0.125),
            rotation,
            scale: Vec3::new(1.25, 0.875, 1.),
        },
        inverse_bind: Mat4::from_scale(Vec3::new(2., 3., 4.)) * (-2.),
    }])
    .unwrap();
    let limits = AnimationAssetLimits::default();
    let saved = AnimationAssetCheckpoint::capture(std::slice::from_ref(&rig), &[], limits).unwrap();
    let restored = AnimationAssetCheckpoint::from_json(
        &saved.to_json(limits.max_bytes).unwrap(),
        limits.max_bytes,
    )
    .unwrap()
    .prepare(limits)
    .unwrap();
    assert_eq!(
        restored.skeletons[0].joints()[0]
            .bind_local
            .rotation
            .to_array()
            .map(f32::to_bits),
        rotation.to_array().map(f32::to_bits)
    );
    assert_eq!(
        restored.skeletons[0].joints()[0]
            .inverse_bind
            .to_cols_array()
            .map(f32::to_bits),
        rig.joints()[0]
            .inverse_bind
            .to_cols_array()
            .map(f32::to_bits)
    );
    let empty = AnimationAssetCheckpoint::capture(&[], &[], limits)
        .unwrap()
        .prepare(limits)
        .unwrap();
    assert!(empty.skeletons.is_empty() && empty.clips.is_empty());
}

#[test]
fn enclosing_name_bindings_keep_duplicates_independent_equal_strings_and_old_wire_shape() {
    let limits = AnimationAssetLimits::default();
    let skeleton = rig();
    let shared = skeleton.joints()[0].name.clone();
    let independent: Arc<str> = Arc::from(shared.as_ref());
    let names = vec![shared.clone(), independent, shared, Arc::from("")];
    let record = AnimationAssetCheckpoint::capture_with_names(
        std::slice::from_ref(&skeleton),
        &[],
        &names,
        limits,
    )
    .unwrap();
    let loaded = record.prepare(limits).unwrap();
    assert!(Arc::ptr_eq(&loaded.names[0], &loaded.names[2]));
    assert!(Arc::ptr_eq(
        &loaded.names[0],
        &loaded.skeletons[0].joints()[0].name
    ));
    assert!(!Arc::ptr_eq(&loaded.names[0], &loaded.names[1]));
    let mut corrupt = record.clone();
    corrupt.name_bindings[0] = usize::MAX;
    assert!(corrupt.prepare(limits).is_err());
    assert!(
        record
            .prepare(AnimationAssetLimits {
                max_references: 3,
                ..limits
            })
            .is_err()
    );
    let plain = AnimationAssetCheckpoint::capture(&[skeleton], &[], limits).unwrap();
    let json = plain.to_json(limits.max_bytes).unwrap();
    assert!(!json.contains("name_bindings"));
    assert!(
        AnimationAssetCheckpoint::from_json(&json, limits.max_bytes)
            .unwrap()
            .prepare(limits)
            .unwrap()
            .names
            .is_empty()
    );
}
