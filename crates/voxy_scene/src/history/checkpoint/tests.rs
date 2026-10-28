use super::*;
fn registry() -> ComponentRegistry {
    let mut registry = ComponentRegistry::default();
    registry.register::<u32>("game.health.v1").unwrap();
    registry
        .register_with_references::<crate::ObjectId>("game.target.v1", |id| vec![id.clone()])
        .unwrap();
    registry
}
fn doc() -> SceneDocument {
    SceneDocument::from_json(include_str!("../../../examples/data/game.scene.json")).unwrap()
}
fn limits() -> HistoryCheckpointLimits {
    HistoryCheckpointLimits {
        max_versions: 16,
        max_history_bytes: 100000,
        max_scene_capacity: 8,
        max_archive_bytes: 200000,
        max_edits: 16,
        max_draft_objects: 16,
    }
}
fn history(registry: &ComponentRegistry) -> SceneHistory {
    let mut history = SceneHistory::new_with_metadata(
        doc(),
        serde_json::json!({"prefab": "one"}),
        registry,
        8,
        8,
        100000,
    )
    .unwrap();
    for step in 0..5 {
        history
            .edit(registry, |doc| {
                doc.objects[0].translation[0] = step as f32;
                Ok(())
            })
            .unwrap();
        history
            .commit_auxiliary(serde_json::json!({"material": step}), registry)
            .unwrap();
    }
    assert!(history.undo());
    assert!(history.undo());
    history
}
#[test]
fn history_roundtrip_preserves_both_stacks_metadata_auxiliary_bytes_and_120_continuations() {
    let registry = registry();
    let mut a = history(&registry);
    let saved = a.checkpoint(&[], limits()).unwrap();
    let json = saved.to_json(200000).unwrap();
    let saved = SceneHistoryCheckpoint::from_json(&json, 200000).unwrap();
    let PreparedSceneHistory {
        history: mut b,
        edits,
    } = saved.prepare(&registry, limits()).unwrap();
    assert!(edits.is_empty());
    assert_eq!(
        json,
        b.checkpoint(&[], limits())
            .unwrap()
            .to_json(200000)
            .unwrap()
    );
    for step in 0..120 {
        match step % 7 {
            0 | 1 => assert_eq!(a.undo(), b.undo()),
            2 | 3 => assert_eq!(a.redo(), b.redo()),
            4 => {
                for history in [&mut a, &mut b] {
                    history
                        .commit_auxiliary(serde_json::json!({"revision":step}), &registry)
                        .unwrap();
                }
            }
            5 => {
                for history in [&mut a, &mut b] {
                    history
                        .refresh_metadata(serde_json::json!({"prefab":step}), &registry)
                        .unwrap();
                }
            }
            _ => {
                for history in [&mut a, &mut b] {
                    history
                        .edit(&registry, |doc| {
                            doc.objects[0].translation[1] = step as f32 * 0.125;
                            Ok(())
                        })
                        .unwrap();
                }
            }
        }
        assert_eq!(a.retained_bytes(), b.retained_bytes());
        assert_eq!(
            a.checkpoint(&[], limits())
                .unwrap()
                .to_json(200000)
                .unwrap(),
            b.checkpoint(&[], limits())
                .unwrap()
                .to_json(200000)
                .unwrap()
        );
    }
}
#[test]
fn detached_epoch_allocation_sharing_and_staleness_survive_equal_documents_and_new_history() {
    let registry = registry();
    let mut original = history(&registry);
    let stale_a = original.begin_edit();
    let stale_b = original.begin_edit();
    let current = original.current().clone();
    original
        .edit(&registry, |doc| {
            doc.objects[0].translation[2] = 5.;
            Ok(())
        })
        .unwrap();
    assert!(original.undo());
    assert_eq!(*original.current(), current);
    let stale_c = original.begin_edit();
    original
        .refresh_metadata(serde_json::json!({"epoch":"new"}), &registry)
        .unwrap();
    let valid = original.begin_edit();
    let saved = original
        .checkpoint(&[&stale_a, &valid, &stale_b, &stale_c], limits())
        .unwrap();
    let json = saved.to_json(200000).unwrap();
    let mut restored = SceneHistoryCheckpoint::from_json(&json, 200000)
        .unwrap()
        .prepare(&registry, limits())
        .unwrap();
    assert!(Arc::ptr_eq(
        &restored.edits[0].epoch,
        &restored.edits[2].epoch
    ));
    assert!(!Arc::ptr_eq(
        &restored.edits[0].epoch,
        &restored.edits[3].epoch
    ));
    assert!(Arc::ptr_eq(
        &restored.history.epoch,
        &restored.edits[1].epoch
    ));
    assert!(!Arc::ptr_eq(&restored.history.epoch, &original.epoch));
    assert_eq!(
        json,
        restored
            .history
            .checkpoint(&restored.edits.iter().collect::<Vec<_>>(), limits())
            .unwrap()
            .to_json(200000)
            .unwrap()
    );
    let stale_c = restored.edits.pop().unwrap();
    let stale_b = restored.edits.pop().unwrap();
    let mut valid = restored.edits.pop().unwrap();
    let stale_a = restored.edits.pop().unwrap();
    for stale in [stale_a, stale_b, stale_c] {
        assert!(restored.history.commit_edit(stale, &registry).is_err());
    }
    valid.document_mut().objects[0].translation[2] = 0.25;
    valid.document_mut().objects[0].translation[1] = -0.0;
    assert!(restored.history.commit_edit(valid, &registry).unwrap());
    // An otherwise valid transaction from the original history cannot attach to a fresh owner.
    assert!(
        restored
            .history
            .commit_edit(original.begin_edit(), &registry)
            .is_err()
    );
}
#[test]
fn invalid_intermediate_draft_retains_float_payload_words_and_is_rejected_only_at_commit() {
    let registry = registry();
    let original = history(&registry);
    let mut draft = original.begin_edit();
    let words = [0x80000000, 0x7fc01234, 0xff800000];
    draft.document_mut().objects[0].translation = words.map(f32::from_bits);
    draft.document_mut().objects[0].parent =
        Some(crate::ObjectId("missing-draft-reference".into()));
    let json = original
        .checkpoint(&[&draft], limits())
        .unwrap()
        .to_json(200000)
        .unwrap();
    let mut restored = SceneHistoryCheckpoint::from_json(&json, 200000)
        .unwrap()
        .prepare(&registry, limits())
        .unwrap();
    let draft = restored.edits.pop().unwrap();
    assert_eq!(
        draft.document().objects[0].translation.map(f32::to_bits),
        words
    );
    let before = restored
        .history
        .checkpoint(&[], limits())
        .unwrap()
        .to_json(200000)
        .unwrap();
    assert!(restored.history.commit_edit(draft, &registry).is_err());
    assert_eq!(
        restored
            .history
            .checkpoint(&[], limits())
            .unwrap()
            .to_json(200000)
            .unwrap(),
        before
    );
}
#[test]
fn malformed_versions_counts_epochs_documents_and_tight_limits_reject_without_mutating_original() {
    let registry = registry();
    let original = history(&registry);
    let edit = original.begin_edit();
    let saved = original.checkpoint(&[&edit], limits()).unwrap();
    let json = saved.to_json(200000).unwrap();
    for case in 0..8 {
        let mut bad = saved.clone();
        match case {
            0 => bad.version = 2,
            1 => bad.current.bytes += 1,
            2 => bad.undo[0].document.objects[0].parent = Some(crate::ObjectId("missing".into())),
            3 => bad.edits[0].epoch = 2,
            4 => bad.epoch_count = 2,
            5 => bad.max_versions = 1,
            6 => bad.max_bytes = 1,
            7 => bad.redo[0].bytes = usize::MAX,
            _ => unreachable!(),
        }
        assert!(bad.prepare(&registry, limits()).is_err(), "case {case}");
    }
    for case in 0..6 {
        let mut tight = limits();
        match case {
            0 => tight.max_versions = 1,
            1 => tight.max_history_bytes = 1,
            2 => tight.max_scene_capacity = 1,
            3 => tight.max_edits = 0,
            4 => tight.max_archive_bytes = json.len() - 1,
            5 => tight.max_draft_objects = 1,
            _ => unreachable!(),
        }
        assert!(saved.prepare(&registry, tight).is_err());
        assert!(original.checkpoint(&[&edit], tight).is_err());
    }
    assert!(saved.to_json(json.len() - 1).is_err());
    assert!(SceneHistoryCheckpoint::from_json(&json, json.len() - 1).is_err());
    assert_eq!(
        original
            .checkpoint(&[&edit], limits())
            .unwrap()
            .to_json(200000)
            .unwrap(),
        json
    );
}
