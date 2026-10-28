use super::*;
fn fixture() -> (PropertyClip, PropertySet) {
    let name: Arc<str> = Arc::from("scalar");
    let track = |name: Arc<str>, a, b, mode, tangents| {
        PropertyTrack::new_with_interpolation(
            name,
            vec![
                PropertyKey {
                    time: -0.,
                    value: a,
                },
                PropertyKey { time: 2., value: b },
            ],
            2.,
            mode,
            tangents,
        )
        .unwrap()
    };
    let scalar = track(
        name.clone(),
        PropertyValue::Float(-0.),
        PropertyValue::Float(3.),
        Interpolation::CubicSpline,
        vec![
            (
                PropertyValue::Float(f32::from_bits(1)),
                PropertyValue::Float(2.),
            ),
            (PropertyValue::Float(-1.), PropertyValue::Float(-0.)),
        ],
    );
    let vector = track(
        Arc::from("vector"),
        PropertyValue::Vec3(Vec3::new(-0., f32::from_bits(1), 2.)),
        PropertyValue::Vec3(Vec3::new(3., -1., 0.)),
        Interpolation::Linear,
        vec![],
    );
    let color = track(
        Arc::from("color"),
        PropertyValue::Color([0., -0., 1., 0.5]),
        PropertyValue::Color([1., 2., 3., 4.]),
        Interpolation::Step,
        vec![],
    );
    let boolean = track(
        Arc::from("step"),
        PropertyValue::Bool(false),
        PropertyValue::Bool(true),
        Interpolation::Step,
        vec![],
    );
    let legacy = PropertyTrack::new(
        "legacy",
        vec![
            PropertyKey {
                time: 0.,
                value: PropertyValue::Bool(true),
            },
            PropertyKey {
                time: 2.,
                value: PropertyValue::Bool(false),
            },
        ],
        2.,
    )
    .unwrap();
    let defaults = PropertySet::new(vec![
        (name.clone(), PropertyValue::Float(-0.)),
        (
            Arc::from("other"),
            PropertyValue::Color([f32::from_bits(1), 0., 1., -0.]),
        ),
    ])
    .unwrap();
    (
        PropertyClip::new(
            name,
            2.,
            Playback::Loop,
            vec![scalar, vector, color, boolean, legacy],
        )
        .unwrap(),
        defaults,
    )
}
fn values(set: PropertySet) -> Vec<(String, Value)> {
    set.entries()
        .iter()
        .map(|(n, v)| (n.to_string(), (*v).into()))
        .collect()
}
#[test]
fn exact_words_native_sharing_legacy_boolean_and_120_sample_cycles_survive() {
    let limits = PropertyAssetLimits::default();
    let (clip, defaults) = fixture();
    let independent = fixture().0;
    let shared_keys = PropertyClip::new(
        "partial-sharing",
        3.,
        Playback::Clamp,
        clip.tracks().to_vec(),
    )
    .unwrap();
    let record = PropertyAssetCheckpoint::capture(
        &[clip.clone(), clip.clone(), independent, shared_keys],
        &[defaults.clone(), defaults],
        limits,
    )
    .unwrap();
    let json = record.to_json(limits.max_bytes).unwrap();
    let restored = PropertyAssetCheckpoint::from_json(&json, limits.max_bytes)
        .unwrap()
        .prepare(limits)
        .unwrap();
    assert_eq!(
        PropertyAssetCheckpoint::capture(&restored.clips, &restored.sets, limits).unwrap(),
        record
    );
    assert!(Arc::ptr_eq(
        &restored.clips[0].tracks,
        &restored.clips[1].tracks
    ));
    assert!(!Arc::ptr_eq(
        &restored.clips[0].tracks,
        &restored.clips[2].tracks
    ));
    assert!(!Arc::ptr_eq(
        &restored.clips[0].tracks,
        &restored.clips[3].tracks
    ));
    for (a, b) in restored.clips[0]
        .tracks
        .iter()
        .zip(restored.clips[3].tracks.iter())
    {
        assert!(Arc::ptr_eq(&a.keys, &b.keys));
        assert!(Arc::ptr_eq(&a.tangents, &b.tangents));
        assert!(Arc::ptr_eq(&a.name, &b.name));
    }
    let scalar = &restored.clips[0]
        .tracks
        .iter()
        .find(|t| t.name() == "scalar")
        .unwrap()
        .name;
    assert!(Arc::ptr_eq(&restored.clips[0].name, scalar));
    assert!(Arc::ptr_eq(
        scalar,
        &restored.sets[0]
            .entries
            .iter()
            .find(|v| v.0.as_ref() == "scalar")
            .unwrap()
            .0
    ));
    let sources = [
        clip.clone(),
        clip.clone(),
        fixture().0,
        PropertyClip::new(
            "partial-sharing",
            3.,
            Playback::Clamp,
            clip.tracks().to_vec(),
        )
        .unwrap(),
    ];
    for frame in 0..120 {
        let time = f64::from(frame) / 17. - 2.;
        for (a, b) in sources.iter().zip(&restored.clips) {
            assert_eq!(
                values(a.sample_clock(time).unwrap()),
                values(b.sample_clock(time).unwrap())
            );
        }
    }
}
#[test]
fn bad_refs_source_words_order_modes_unused_records_and_caps_reject_atomically() {
    let limits = PropertyAssetLimits::default();
    let (clip, set) = fixture();
    let original = PropertyAssetCheckpoint::capture(
        std::slice::from_ref(&clip),
        std::slice::from_ref(&set),
        limits,
    )
    .unwrap();
    let mut cases = Vec::new();
    let mut r = original.clone();
    r.version = 2;
    cases.push(r);
    let mut r = original.clone();
    r.clips[0].tracks = usize::MAX;
    cases.push(r);
    let mut r = original.clone();
    r.clips[0].duration = f32::NAN.to_bits();
    cases.push(r);
    let mut r = original.clone();
    r.tracks[0][0].mode = 255;
    cases.push(r);
    let mut r = original.clone();
    r.tracks[0].reverse();
    cases.push(r);
    let mut r = original.clone();
    r.keys[0][0].time = f32::NAN.to_bits();
    cases.push(r);
    let mut r = original.clone();
    r.sets[0][0].1 = Value::Float(f32::INFINITY.to_bits());
    cases.push(r);
    let mut r = original.clone();
    r.names.push("unused".into());
    cases.push(r);
    let mut r = original.clone();
    r.tangents.push(Vec::new());
    cases.push(r);
    let mut r = original.clone();
    r.keys[0][0].value = Value::Bool(true);
    cases.push(r);
    for r in cases {
        assert!(r.prepare(limits).is_err());
    }
    for limits in [
        PropertyAssetLimits {
            max_keys: 0,
            ..limits
        },
        PropertyAssetLimits {
            max_tracks: 0,
            ..limits
        },
        PropertyAssetLimits {
            max_tangent_pairs: 0,
            ..limits
        },
        PropertyAssetLimits {
            max_string_bytes: 0,
            ..limits
        },
        PropertyAssetLimits {
            max_bytes: 1,
            ..limits
        },
    ] {
        assert!(original.prepare(limits).is_err());
    }
    let json = original.to_json(limits.max_bytes).unwrap();
    assert!(PropertyAssetCheckpoint::from_json(&json, json.len() - 1).is_err());
    assert!(
        PropertyAssetCheckpoint::from_json(
            &json.replacen("\"version\":1", "\"unknown\":1,\"version\":1", 1),
            limits.max_bytes
        )
        .is_err()
    );
    assert_eq!(
        PropertyAssetCheckpoint::capture(&[clip], &[set], limits).unwrap(),
        original
    );
}
#[test]
fn empty_sets_and_empty_clip_track_allocations_keep_identity() {
    let limits = PropertyAssetLimits::default();
    let a = PropertyClip::new("empty", 1., Playback::Clamp, vec![]).unwrap();
    let b = PropertyClip::new("empty", 1., Playback::Clamp, vec![]).unwrap();
    let record =
        PropertyAssetCheckpoint::capture(&[a.clone(), a, b], &[PropertySet::default()], limits)
            .unwrap();
    let loaded = record.prepare(limits).unwrap();
    assert!(Arc::ptr_eq(
        &loaded.clips[0].tracks,
        &loaded.clips[1].tracks
    ));
    assert!(!Arc::ptr_eq(
        &loaded.clips[0].tracks,
        &loaded.clips[2].tracks
    ));
    assert!(loaded.sets[0].is_empty());
    let empty = PropertyAssetCheckpoint::capture(&[], &[], limits).unwrap();
    assert!(empty.prepare(limits).unwrap().clips.is_empty());
}

#[test]
fn enclosing_names_link_skeleton_clips_properties_and_metadata_without_merging_equal_names() {
    use crate::{
        AnimationAssetCheckpoint, AnimationAssetLimits, AnimationClip, Joint, JointTrack, Skeleton,
        Transform,
    };
    use glam::Mat4;
    let shared: Arc<str> = Arc::from("linked");
    let independent: Arc<str> = Arc::from("linked");
    let metadata_only: Arc<str> = Arc::from("");
    let rig = Skeleton::new(vec![Joint {
        name: shared.clone(),
        parent: None,
        bind_local: Transform::IDENTITY,
        inverse_bind: Mat4::IDENTITY,
    }])
    .unwrap();
    let clip = Arc::new(
        AnimationClip::new(
            shared.clone(),
            1.,
            Playback::Loop,
            vec![JointTrack::default()],
            &rig,
        )
        .unwrap(),
    );
    let track = PropertyTrack::new(
        shared.clone(),
        vec![PropertyKey {
            time: -0.,
            value: PropertyValue::Float(f32::from_bits(1)),
        }],
        1.,
    )
    .unwrap();
    let properties = PropertyClip::new(shared.clone(), 1., Playback::Loop, vec![track]).unwrap();
    let set = PropertySet::new(vec![(independent.clone(), PropertyValue::Float(-0.))]).unwrap();
    let names = vec![shared.clone(), independent.clone(), shared, metadata_only];
    let limits = PropertyAssetLimits::default();
    let animation_limits = AnimationAssetLimits::default();
    let skeletal = AnimationAssetCheckpoint::capture_with_names(
        std::slice::from_ref(&rig),
        std::slice::from_ref(&clip),
        &names,
        animation_limits,
    )
    .unwrap();
    let record = PropertyAssetCheckpoint::capture_with_names(
        std::slice::from_ref(&properties),
        std::slice::from_ref(&set),
        &names,
        limits,
    )
    .unwrap();
    let restored_skeleton = AnimationAssetCheckpoint::from_json(
        &skeletal.to_json(animation_limits.max_bytes).unwrap(),
        animation_limits.max_bytes,
    )
    .unwrap()
    .prepare(animation_limits)
    .unwrap();
    let loaded = PropertyAssetCheckpoint::from_json(
        &record.to_json(limits.max_bytes).unwrap(),
        limits.max_bytes,
    )
    .unwrap()
    .prepare_with_names(limits, &restored_skeleton.names)
    .unwrap();
    assert!(Arc::ptr_eq(
        &restored_skeleton.names[0],
        &restored_skeleton.names[2]
    ));
    assert!(!Arc::ptr_eq(
        &restored_skeleton.names[0],
        &restored_skeleton.names[1]
    ));
    assert!(Arc::ptr_eq(
        &restored_skeleton.skeletons[0].joints()[0].name,
        &loaded.clips[0].name
    ));
    assert!(Arc::ptr_eq(
        &loaded.clips[0].name,
        &loaded.clips[0].tracks[0].name
    ));
    assert!(Arc::ptr_eq(
        &restored_skeleton.names[1],
        &loaded.sets[0].entries[0].0
    ));
    assert!(Arc::ptr_eq(&loaded.names[0], &restored_skeleton.names[0]));
    for phase in 0..120 {
        assert_eq!(
            values(properties.sample_clock(f64::from(phase) / 71.).unwrap()),
            values(
                loaded.clips[0]
                    .sample_clock(f64::from(phase) / 71.)
                    .unwrap()
            )
        );
    }
    let mut merged = restored_skeleton.names.clone();
    merged[1] = merged[0].clone();
    assert!(record.prepare_with_names(limits, &merged).is_err());
    let mut split = restored_skeleton.names.clone();
    split[2] = Arc::from(split[2].as_ref());
    assert!(record.prepare_with_names(limits, &split).is_err());
    let mut wrong = restored_skeleton.names.clone();
    wrong[0] = Arc::from("wrong");
    assert!(record.prepare_with_names(limits, &wrong).is_err());
    assert!(record.prepare_with_names(limits, &[]).is_err());
    assert_eq!(
        PropertyAssetCheckpoint::capture_with_names(&[properties], &[set], &names, limits).unwrap(),
        record
    );
}
