use super::*;
fn publications() -> (Publication, Publication) {
    let provider = voxy_assets::FileInputs::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../voxy_render/examples/assets"),
    )
    .unwrap();
    let mut inputs = voxy_assets::ImportInputs::new(4, 4 * 1024 * 1024);
    let mut asset = crate::import::load(
        &voxy_assets::SourcePath::new("animated-triangle.glb").unwrap(),
        &provider,
        &mut inputs,
    )
    .unwrap();
    asset.images = vec![
        voxy_render::ImageAsset::from_rgba(1, 1, vec![255, 0, 0, 255], Default::default()).unwrap(),
    ];
    let mut replacement = asset.clone();
    replacement.images[0] =
        voxy_render::ImageAsset::from_rgba(1, 1, vec![0, 255, 0, 255], Default::default()).unwrap();
    (
        Arc::new(
            inputs
                .finish(asset, |id, max| provider.read(id, max))
                .unwrap(),
        ),
        Arc::new(
            voxy_assets::ImportInputs::new(0, 0)
                .finish(replacement, |_, _| unreachable!())
                .unwrap(),
        ),
    )
}
#[test]
fn roles_keep_historical_materials_and_aliases_across_reversed_scene_ids() {
    let (old, current) = publications();
    let mut scene = SceneGraph::new(4);
    let left = scene.spawn(None, Default::default()).unwrap();
    let right = scene.spawn(None, Default::default()).unwrap();
    let live = LiveSources {
        models: vec![(AssetId("model".into()), current.clone())],
        owners: vec![(left, old), (right, current)],
    };
    let identify = |node| {
        if node == left {
            Some(ObjectId("left".into()))
        } else if node == right {
            Some(ObjectId("right".into()))
        } else {
            None
        }
    };
    let bindings = GpuSourceBindings::capture(&live, identify, 3).unwrap();
    let limits = crate::import::asset_checkpoint::EditorAssetCheckpointLimits::default();
    let inputs = voxy_assets::ImportedArchiveLimits {
        max_inputs: 4,
        max_input_bytes: 4 * 1024 * 1024,
        max_archive_bytes: 32 * 1024 * 1024,
    };
    let bytes = crate::import::publication_checkpoint::capture(
        &live.publications(),
        limits,
        inputs,
        64 * 1024 * 1024,
    )
    .unwrap();
    let roots =
        crate::import::publication_checkpoint::prepare(&bytes, limits, inputs, 64 * 1024 * 1024)
            .unwrap();
    let bindings: GpuSourceBindings =
        serde_json::from_slice(&serde_json::to_vec(&bindings).unwrap()).unwrap();
    let mut target = SceneGraph::new(4);
    let new_right = target.spawn(None, Default::default()).unwrap();
    let new_left = target.spawn(None, Default::default()).unwrap();
    let resolve = |id: &ObjectId| match id.0.as_str() {
        "left" => Some(new_left),
        "right" => Some(new_right),
        _ => None,
    };
    let loaded = bindings.prepare(&target, resolve, &roots, 3).unwrap();
    assert_eq!(loaded.scene, target.identity());
    assert!(Arc::ptr_eq(
        &loaded.models[&AssetId("model".into())],
        &loaded.owners[&new_right]
    ));
    assert!(!Arc::ptr_eq(
        &loaded.owners[&new_left],
        &loaded.owners[&new_right]
    ));
    assert_eq!(
        loaded.owners[&new_left].value().images[0].rgba(),
        [255, 0, 0, 255]
    );
    assert_eq!(
        loaded.owners[&new_right].value().images[0].rgba(),
        [0, 255, 0, 255]
    );
    assert!(Arc::ptr_eq(
        loaded.owners[&new_left].value().animated.as_ref().unwrap(),
        loaded.owners[&new_right].value().animated.as_ref().unwrap()
    ));
    assert!(
        bindings
            .prepare(&target, |_| Some(new_left), &roots, 3)
            .is_err()
    );
    assert!(bindings.prepare(&target, |_| None, &roots, 3).is_err());
    assert!(bindings.prepare(&target, resolve, &roots, 2).is_err());
    let mut bad = bindings.clone();
    bad.owners[0].1 = 0;
    assert!(bad.prepare(&target, resolve, &roots, 3).is_err());
    let mut bad = bindings.clone();
    bad.owners[1].0 = bad.owners[0].0.clone();
    assert!(bad.prepare(&target, resolve, &roots, 3).is_err());
    assert!(GpuSourceBindings::capture(&live, |_| None, 3).is_err());
    assert!(GpuSourceBindings::capture(&live, |_| Some(ObjectId("same".into())), 3).is_err());
}
