use super::*;
use crate::import::{FileInputs, ImportInputs, SourcePath, load};
use voxy_assets::{AssetError, AssetId, AssetStatus, FailedImport};
fn limits() -> Limits {
    Limits {
        assets: EditorAssetCheckpointLimits::default(),
        inputs: ImportedArchiveLimits {
            max_inputs: 4,
            max_input_bytes: 4 * 1024 * 1024,
            max_archive_bytes: 16 * 1024 * 1024,
        },
        max_bytes: 64 * 1024 * 1024,
        max_catalog_assets: 4,
        max_pending: 2,
        max_source_outputs: 4,
        max_source_edges: 16,
    }
}
fn imported() -> ImportedAsset<EditorAsset> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "voxy-catalog-{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("mesh.glb"),
        include_bytes!("../../../../voxy_render/examples/assets/animated-triangle.glb"),
    )
    .unwrap();
    let provider = FileInputs::new(&root).unwrap();
    let mut inputs = ImportInputs::new(4, 4 * 1024 * 1024);
    let asset = load(
        &SourcePath::new("mesh.glb").unwrap(),
        &provider,
        &mut inputs,
    )
    .unwrap();
    let value = inputs
        .finish(asset, |id, limit| provider.read(id, limit))
        .unwrap();
    std::fs::remove_dir_all(root).unwrap();
    value
}
#[test]
fn failed_reload_keeps_original_model_sources_and_revision_after_restore() {
    let id = AssetId("character".into());
    let missing = AssetId("missing.cfg".into());
    let mut catalog = AssetCatalog::new(4, 2).unwrap();
    let mut sources = SourceDependencies::new(4, 16);
    let ticket = catalog.request(id.clone()).unwrap();
    catalog
        .complete_observed(&mut sources, &ticket, Ok(imported()))
        .unwrap();
    let last_good = catalog.snapshot_with_revision(&id).unwrap();
    let reload = catalog.request(id.clone()).unwrap();
    assert!(capture(&catalog, &sources, limits()).is_err());
    let mut attempted = ImportInputs::new(4, 4 * 1024 * 1024);
    let _ = attempted.read(missing.clone(), |_, _| Err("missing input".into()));
    catalog
        .complete_observed(
            &mut sources,
            &reload,
            Err(FailedImport {
                error: "reload failed".into(),
                inputs: attempted,
            }),
        )
        .unwrap();
    let absent = catalog.request(AssetId("absent".into())).unwrap();
    catalog.complete(&absent, Err("not found".into())).unwrap();
    let bytes = capture(&catalog, &sources, limits()).unwrap();
    super::super::asset_checkpoint::retain_evidence("failed-model-catalog.voxyeic", &bytes);
    let file = std::env::temp_dir().join(format!(
        "voxy-catalog-file-{}-{}.voxyeic",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&file, &bytes).unwrap();
    let bytes = std::fs::read(&file).unwrap();
    std::fs::remove_file(file).unwrap();
    let mut restored = prepare(&bytes, limits()).unwrap();
    assert_eq!(
        capture(&restored.catalog, &restored.sources, limits()).unwrap(),
        bytes
    );
    assert_eq!(
        restored.catalog.status(&id),
        Some(&AssetStatus::Failed("reload failed".into()))
    );
    let loaded = restored.catalog.snapshot_with_revision(&id).unwrap();
    assert_eq!(loaded.0, last_good.0);
    assert_eq!(
        loaded.1.inputs().observations(),
        last_good.1.inputs().observations()
    );
    assert_eq!(restored.sources.checkpoint(), sources.checkpoint());
    assert_eq!(restored.sources.affected([missing]), vec![id.clone()]);
    assert_eq!(
        restored
            .catalog
            .complete(&reload, Err("late worker".into())),
        Err(AssetError::StaleTicket)
    );
    assert!(Arc::ptr_eq(
        &loaded.1,
        &restored.catalog.snapshot(&id).unwrap()
    ));
    let next = restored.catalog.request(id.clone()).unwrap();
    restored
        .catalog
        .complete_observed(&mut restored.sources, &next, Ok(imported()))
        .unwrap();
    assert_eq!(restored.catalog.status(&id), Some(&AssetStatus::Ready));
    assert_eq!(restored.catalog.snapshot_with_revision(&id).unwrap().0, 4);
    assert!(
        restored
            .sources
            .affected([AssetId("missing.cfg".into())])
            .is_empty()
    );
    restored
        .catalog
        .invalidate(std::slice::from_ref(&id))
        .unwrap();
    let dirty_bytes = capture(&restored.catalog, &restored.sources, limits()).unwrap();
    let dirty = prepare(&dirty_bytes, limits()).unwrap();
    assert_eq!(dirty.catalog.status(&id), Some(&AssetStatus::Dirty));
    assert_eq!(dirty.catalog.snapshot_with_revision(&id).unwrap().0, 4);
    // Detached restore and replacement never mutate the live original owner.
    assert_eq!(
        catalog.status(&id),
        Some(&AssetStatus::Failed("reload failed".into()))
    );
    assert!(Arc::ptr_eq(&last_good.1, &catalog.snapshot(&id).unwrap()));
}
#[test]
fn mismatched_dependencies_and_limits_reject_without_touching_live_catalog() {
    let id = AssetId("character".into());
    let mut catalog = AssetCatalog::new(4, 2).unwrap();
    let mut sources = SourceDependencies::new(4, 16);
    let ticket = catalog.request(id.clone()).unwrap();
    catalog
        .complete_observed(&mut sources, &ticket, Ok(imported()))
        .unwrap();
    let original = catalog.snapshot(&id).unwrap();
    let bytes = capture(&catalog, &sources, limits()).unwrap();
    assert!(
        prepare(
            &bytes,
            Limits {
                max_pending: 1,
                ..limits()
            }
        )
        .is_err()
    );
    assert!(
        prepare(
            &bytes,
            Limits {
                max_source_edges: 15,
                ..limits()
            }
        )
        .is_err()
    );
    let mut record: Record = serde_json::from_slice(&bytes[40..]).unwrap();
    record.sources.outputs[0].published = vec![AssetId("wrong.glb".into())];
    let body = serde_json::to_vec(&record).unwrap();
    let mut malformed = b"VOXYEIC1".to_vec();
    malformed.extend_from_slice(blake3::hash(&body).as_bytes());
    malformed.extend_from_slice(&body);
    assert!(prepare(&malformed, limits()).is_err());
    assert!(Arc::ptr_eq(&original, &catalog.snapshot(&id).unwrap()));
    sources.remove(&id);
    assert!(capture(&catalog, &sources, limits()).is_err());
}

#[test]
fn retained_only_graph_preserves_publication_identity_and_rejects_wrong_split() {
    let catalog = AssetCatalog::new(4, 2).unwrap();
    let sources = SourceDependencies::new(4, 16);
    let publication = Arc::new(imported());
    let retained = vec![publication.clone(), publication];
    let bytes =
        capture_with_retained(&catalog, &sources, &[], &[], &[], &retained, limits()).unwrap();
    let loaded = prepare(&bytes, limits()).unwrap();
    assert!(Arc::ptr_eq(&loaded.retained[0], &loaded.retained[1]));
    assert_eq!(
        loaded.retained[0].inputs().observations(),
        retained[0].inputs().observations()
    );
    assert_eq!(
        capture_with_retained(
            &loaded.catalog,
            &loaded.sources,
            &[],
            &[],
            &[],
            &loaded.retained,
            limits()
        )
        .unwrap(),
        bytes
    );
    let mut record: Record = serde_json::from_slice(&bytes[40..]).unwrap();
    record.retained = 1;
    let body = serde_json::to_vec(&record).unwrap();
    let mut malformed = b"VOXYEIC1".to_vec();
    malformed.extend_from_slice(blake3::hash(&body).as_bytes());
    malformed.extend_from_slice(&body);
    assert!(prepare(&malformed, limits()).is_err());
}
