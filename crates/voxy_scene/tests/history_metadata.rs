use voxy_scene::{ComponentRegistry, SceneDocument, SceneHistory};
#[test]
fn authoring_metadata_is_atomic_bounded_and_undoable_even_for_equal_graphs() {
    let registry = ComponentRegistry::default();
    let scene = SceneDocument {
        version: 1,
        objects: vec![],
    };
    let first = serde_json::json!({"prefab":"first"});
    let second = serde_json::json!({"prefab":"second"});
    let mut history =
        SceneHistory::new_with_metadata(scene.clone(), first.clone(), &registry, 1, 8, 256)
            .unwrap();
    assert!(!history.undo());
    let stale = history.begin_edit();
    assert!(
        history
            .commit_with_metadata(scene.clone(), second.clone(), &registry)
            .unwrap()
    );
    assert!(history.commit_edit(stale, &registry).is_err());
    assert_eq!(history.metadata(), &second);
    assert!(history.undo());
    assert_eq!(history.metadata(), &first);
    assert!(!history.commit(scene.clone(), &registry).unwrap());
    assert!(history.redo());
    assert_eq!(history.metadata(), &second);
    let before = history.retained_bytes();
    assert!(
        history
            .check_metadata(&serde_json::json!({"huge":"x".repeat(512)}), &registry)
            .is_err()
    );
    assert_eq!(history.retained_bytes(), before);
    assert!(
        history
            .commit_with_metadata(
                scene,
                serde_json::json!({"huge":"x".repeat(512)}),
                &registry
            )
            .is_err()
    );
    assert_eq!(history.retained_bytes(), before);
    assert_eq!(history.metadata(), &second);
    assert!(history.undo());
    assert_eq!(history.metadata(), &first);
}

#[test]
fn saved_metadata_refresh_preserves_undo_and_redo_without_phantom_versions() {
    let registry = ComponentRegistry::default();
    let scene = SceneDocument {
        version: 1,
        objects: vec![],
    };
    let mut history = SceneHistory::new_with_metadata(
        scene.clone(),
        serde_json::json!("initial"),
        &registry,
        1,
        8,
        256,
    )
    .unwrap();
    history
        .commit_with_metadata(scene.clone(), serde_json::json!("edited"), &registry)
        .unwrap();
    history
        .refresh_metadata(serde_json::json!("saved"), &registry)
        .unwrap();
    assert!(history.undo());
    assert_eq!(history.metadata(), &serde_json::json!("initial"));
    history
        .refresh_metadata(serde_json::json!("initial-saved"), &registry)
        .unwrap();
    assert!(history.redo());
    assert_eq!(history.metadata(), &serde_json::json!("saved"));
    assert!(!history.redo());
    assert!(
        history
            .refresh_metadata(serde_json::json!("x".repeat(512)), &registry)
            .is_err()
    );
    assert_eq!(history.metadata(), &serde_json::json!("saved"));
}
