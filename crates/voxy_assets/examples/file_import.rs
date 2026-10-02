//! File-backed import acceptance; no watcher or concurrent filesystem lock implied.
use voxy_assets::{
    AssetCatalog, AssetId, FailedImport, FileInputs, ImportInputs, ImportedAsset,
    SourceDependencies, SourcePoller,
};
fn decode(
    provider: &FileInputs,
    source: &AssetId,
) -> Result<(String, ImportInputs), FailedImport<String>> {
    ImportInputs::new(4, 1024).decode_observed(|inputs| {
        let snapshot = inputs
            .read(source.clone(), |id, limit| provider.read(id, limit))
            .map_err(|e| format!("{e:?}"))?;
        String::from_utf8(snapshot.bytes.to_vec()).map_err(|e| e.to_string())
    })
}
fn poll_reloads(
    catalog: &mut AssetCatalog<ImportedAsset<String>>,
    sources: &mut SourceDependencies,
    provider: &FileInputs,
    source: &AssetId,
    root: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut poller = SourcePoller::new(4, 1024);
    poller.reconcile(sources)?;
    for bytes in [&[255, 254][..], b"restored", b"final"] {
        std::fs::write(root.join(&source.0), bytes)?;
        let changes = poller.poll(1, |id, limit| provider.read(id, limit));
        assert_eq!(changes, vec![source.clone()]);
        for affected in sources.affected(changes) {
            let ticket = catalog.request(affected)?;
            let result = match decode(provider, source) {
                Ok((value, inputs)) => inputs
                    .finish_observed(value, |id, limit| provider.read(id, limit))
                    .map_err(|rejected| FailedImport {
                        error: format!("{:?}", rejected.error),
                        inputs: rejected.inputs,
                    }),
                Err(failure) => Err(failure),
            };
            catalog.complete_observed(sources, &ticket, result)?;
        }
        poller.reconcile(sources)?;
        assert_eq!(catalog.pending(), 0);
    }
    assert!(
        poller
            .poll(1, |id, limit| provider.read(id, limit))
            .is_empty()
    );
    Ok(())
}
fn temporary_root() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "voxy-file-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir(&root)?;
    Ok(root)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = temporary_root()?;
    let source = AssetId("dialogue.txt".into());
    let output = AssetId("dialogue/compiled".into());
    std::fs::write(root.join(&source.0), "original")?;
    let provider = FileInputs::new(&root)?;
    let mut catalog = AssetCatalog::<ImportedAsset<String>>::new(1, 1)?;
    let mut sources = SourceDependencies::new(1, 4);
    let first = catalog.request(output.clone())?;
    let (value, inputs) = decode(&provider, &source).map_err(|failure| failure.error)?;
    let artifact = inputs
        .finish(value, |id, limit| provider.read(id, limit))
        .map_err(|e| format!("{e:?}"))?;
    catalog.complete_observed(&mut sources, &first, Ok(artifact))?;
    let original = catalog.snapshot(&output).ok_or("missing original")?;
    let old_key = original
        .inputs()
        .build_key("text", "1", "desktop", b"utf8")
        .map_err(|e| format!("{e:?}"))?;
    let old_digest = original.inputs().observations()[&source]
        .as_ref()
        .map_err(|e| format!("{e:?}"))?
        .digest;

    let reload = catalog.request(output.clone())?;
    let (value, inputs) = decode(&provider, &source).map_err(|failure| failure.error)?;
    std::fs::write(root.join(&source.0), "replaced")?; // Same length, different bytes.
    let rejected = inputs
        .finish_observed(value, |id, limit| provider.read(id, limit))
        .unwrap_err();
    assert_eq!(rejected.value, "original");
    assert_eq!(
        rejected.inputs.observations()[&source]
            .as_ref()
            .unwrap()
            .digest,
        old_digest
    );
    catalog.complete_observed(
        &mut sources,
        &reload,
        Err(FailedImport {
            error: format!("{:?}", rejected.error),
            inputs: rejected.inputs,
        }),
    )?;
    assert_eq!(catalog.snapshot(&output).unwrap().value(), "original");
    assert_eq!(catalog.pending(), 0);

    let retry = catalog.request(output.clone())?;
    let (value, inputs) = decode(&provider, &source).map_err(|failure| failure.error)?;
    let artifact = inputs
        .finish(value, |id, limit| provider.read(id, limit))
        .map_err(|e| format!("{e:?}"))?;
    catalog.complete_observed(&mut sources, &retry, Ok(artifact))?;
    let current = catalog.snapshot(&output).unwrap();
    assert_eq!(current.value(), "replaced");
    let current_key = current
        .inputs()
        .build_key("text", "1", "desktop", b"utf8")
        .map_err(|e| format!("{e:?}"))?;
    assert_ne!(old_key, current_key);
    assert_ne!(
        current_key,
        current
            .inputs()
            .build_key("text", "1", "mobile", b"utf8")
            .map_err(|e| format!("{e:?}"))?
    );

    assert_eq!(original.value(), "original");
    assert_ne!(
        current.inputs().observations()[&source]
            .as_ref()
            .unwrap()
            .digest,
        old_digest
    );
    let broken = catalog.request(output.clone())?;
    std::fs::write(root.join(&source.0), [255, 254])?;
    let failure = decode(&provider, &source).unwrap_err();
    assert_eq!(
        &*failure.inputs.observations()[&source]
            .as_ref()
            .unwrap()
            .bytes,
        &[255, 254]
    );
    catalog.complete_observed(&mut sources, &broken, Err(failure))?;
    assert_eq!(sources.affected([source.clone()]), vec![output.clone()]);
    assert_eq!(catalog.snapshot(&output).unwrap().value(), "replaced");
    assert_eq!(catalog.pending(), 0);
    poll_reloads(&mut catalog, &mut sources, &provider, &source, &root)?;
    assert_eq!(catalog.snapshot(&output).unwrap().value(), "final");
    assert_eq!(current.value(), "replaced");
    std::fs::remove_dir_all(root)?;
    println!(
        "FILE IMPORT PASS: changed source rejected, last good retained, retry publishes value and provenance, old snapshot survives, failed decode retains source bytes, polling reloads changed files and recovers decoding"
    );
    Ok(())
}
