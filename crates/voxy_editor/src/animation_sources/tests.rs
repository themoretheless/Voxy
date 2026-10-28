use super::*;
#[test]
fn source_pairs_keep_distinct_property_allocations_and_native_model_aliases() {
    let model = Arc::new(
        voxy_model::ModelAsset::parse(
            include_bytes!("../../../voxy_render/examples/assets/animated-triangle.glb"),
            &[],
            voxy_model::ModelLimits::default(),
        )
        .unwrap(),
    );
    let clip = model.animations[0].clone();
    let clone = Arc::new(clip.as_ref().clone());
    let track = voxy_animation::PropertyTrack::new(
        "weight",
        vec![voxy_animation::PropertyKey {
            time: 0.,
            value: voxy_animation::PropertyValue::Float(0.25),
        }],
        1.,
    )
    .unwrap();
    let a = Arc::new(
        PropertyClip::new("property", 1., voxy_animation::Playback::Loop, vec![track]).unwrap(),
    );
    let b = Arc::new(a.as_ref().clone());
    let mut source = Sources::default();
    assert_eq!(source.intern(&clip, None, 4).unwrap(), "playback:0");
    assert_eq!(source.intern(&clip, Some(&a), 4).unwrap(), "playback:1");
    assert_eq!(source.intern(&clip, Some(&a), 4).unwrap(), "playback:1");
    assert_eq!(source.intern(&clip, Some(&b), 4).unwrap(), "playback:2");
    assert_eq!(source.intern(&clone, Some(&a), 4).unwrap(), "playback:3");
    assert!(source.intern(&clone, None, 4).is_none());
    let limits = voxy_model::ModelAssetCheckpointLimits::default();
    let graph = voxy_model::ModelAssetCheckpoint::capture_with_sources(
        &[model],
        &source.clips,
        &source
            .properties
            .iter()
            .map(|p| p.as_ref().clone())
            .collect::<Vec<_>>(),
        limits,
    )
    .unwrap();
    let graph = graph.prepare_sources(limits).unwrap();
    let properties = graph.properties.clone();
    let loaded = prepare(&source.bindings, &graph.clips, graph.properties, 4).unwrap();
    let first = loaded.resolve("playback:0").unwrap();
    let a = loaded.resolve("playback:1").unwrap();
    let b = loaded.resolve("playback:2").unwrap();
    let clone = loaded.resolve("playback:3").unwrap();
    assert!(first.1.is_none());
    assert!(Arc::ptr_eq(&first.0, &graph.models[0].animations[0]));
    assert!(Arc::ptr_eq(&first.0, &a.0));
    assert!(!Arc::ptr_eq(&first.0, &clone.0));
    assert!(Arc::ptr_eq(
        a.1.as_ref().unwrap(),
        clone.1.as_ref().unwrap()
    ));
    assert!(!Arc::ptr_eq(a.1.as_ref().unwrap(), b.1.as_ref().unwrap()));
    assert_eq!(
        a.1.as_ref().unwrap().tracks().as_ptr(),
        b.1.as_ref().unwrap().tracks().as_ptr()
    );
    assert!(loaded.resolve("playback:01").is_none());
    let mut reordered = Sources::default();
    for (clip, property) in loaded.sources.iter().rev() {
        reordered.intern(clip, property.as_ref(), 4).unwrap();
    }
    assert!(reordered.matches(&loaded));
    assert!(prepare(&source.bindings, &graph.clips, properties.clone(), 3).is_err());
    let mut bad = source.bindings.clone();
    bad.push(bad[0].clone());
    assert!(prepare(&bad, &graph.clips, properties.clone(), 5).is_err());
    let mut bad = source.bindings.clone();
    bad[0].clip = 1;
    assert!(prepare(&bad, &graph.clips, properties, 4).is_err());
}
