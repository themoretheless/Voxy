use super::*;
use crate::import::{load, FileInputs, ImportInputs, SourcePath};

#[test]
fn original_observations_and_shared_models_survive_source_removal() {
    let root = std::env::temp_dir().join(format!(
        "voxy-publications-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let source = include_bytes!("../../../../voxy_render/examples/assets/animated-triangle.glb");
    std::fs::write(root.join("mesh.glb"), source).unwrap();
    let provider = FileInputs::new(&root).unwrap();
    let mut observations = ImportInputs::new(4, 4 * 1024 * 1024);
    let asset = load(
        &SourcePath::new("mesh.glb").unwrap(),
        &provider,
        &mut observations,
    )
    .unwrap();
    let mut second_observations = ImportInputs::new(4, 4 * 1024 * 1024);
    for id in observations.observations().keys() {
        second_observations
            .read(id.clone(), |id, limit| provider.read(id, limit))
            .unwrap();
    }
    let second = Arc::new(
        second_observations
            .finish(asset.clone(), |id, limit| provider.read(id, limit))
            .unwrap(),
    );
    let first = Arc::new(
        observations
            .finish(asset, |id, limit| provider.read(id, limit))
            .unwrap(),
    );
    let assets = EditorAssetCheckpointLimits::default();
    let inputs = ImportedArchiveLimits {
        max_inputs: 4,
        max_input_bytes: 4 * 1024 * 1024,
        max_archive_bytes: 32 * 1024 * 1024,
    };
    let max_bytes = 64 * 1024 * 1024;
    let bytes = capture(
        &[first.clone(), second, first.clone()],
        assets,
        inputs,
        max_bytes,
    )
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    let restored = prepare(&bytes, assets, inputs, max_bytes).unwrap();
    assert!(Arc::ptr_eq(&restored[0], &restored[2]));
    assert!(!Arc::ptr_eq(&restored[0], &restored[1]));
    assert!(Arc::ptr_eq(
        restored[0].value().animated.as_ref().unwrap(),
        restored[1].value().animated.as_ref().unwrap()
    ));
    assert_eq!(
        restored[0].inputs().observations(),
        first.inputs().observations()
    );
    assert_eq!(
        capture(&restored, assets, inputs, max_bytes).unwrap(),
        bytes
    );
    let tighter = ImportedArchiveLimits {
        max_inputs: 3,
        ..inputs
    };
    assert!(prepare(&bytes, assets, tighter, max_bytes).is_err());
    assert!(prepare(&bytes, assets, inputs, bytes.len() - 1).is_err());
    let mut corrupt = bytes.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(prepare(&corrupt, assets, inputs, max_bytes).is_err());
    let mut wire: Publications = serde_json::from_slice(&bytes[40..]).unwrap();
    wire.references = vec![1, 0, 1];
    let body = serde_json::to_vec(&wire).unwrap();
    let mut reordered = b"VOXYEIP1".to_vec();
    reordered.extend_from_slice(blake3::hash(&body).as_bytes());
    reordered.extend_from_slice(&body);
    assert!(prepare(&reordered, assets, inputs, max_bytes).is_err());
    assert_eq!(
        first.inputs().observations(),
        restored[0].inputs().observations()
    );
}

#[test]
fn empty_publication_graph_round_trips() {
    let assets = EditorAssetCheckpointLimits::default();
    let inputs = ImportedArchiveLimits {
        max_inputs: 4,
        max_input_bytes: 1024,
        max_archive_bytes: 8192,
    };
    let bytes = capture(&[], assets, inputs, 65536).unwrap();
    assert!(prepare(&bytes, assets, inputs, 65536).unwrap().is_empty());
}
