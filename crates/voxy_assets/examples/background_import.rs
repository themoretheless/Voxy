//! Offline owner loop: background file scan and decoding, owner-only publication.
use std::time::{Duration, Instant};
use voxy_assets::{
    AssetCatalog, AssetError, AssetId, AssetImportWorker, FileInputs, ImportCompletion,
    ImportWorkerError, PublicationError, SourceDependencies, SourcePollWorker,
};
fn completion(
    worker: &mut AssetImportWorker<String>,
) -> Result<ImportCompletion<String>, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = worker.try_result()? {
            return Ok(result);
        }
        if Instant::now() >= deadline {
            return Err("import timeout".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn scan(worker: &mut SourcePollWorker) -> Result<Vec<AssetId>, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = worker.try_result()? {
            return Ok(result);
        }
        if Instant::now() >= deadline {
            return Err("scan timeout".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "voxy-background-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir(&root)?;
    let id = AssetId("dialogue.txt".into());
    std::fs::write(root.join(&id.0), b"original")?;
    let owner = std::thread::current().id();
    let mut imports = AssetImportWorker::new(
        FileInputs::new(&root)?,
        1,
        1024,
        move |asset, provider, inputs| {
            assert_ne!(std::thread::current().id(), owner);
            let source = inputs
                .read(asset.clone(), |id, limit| provider.read(id, limit))
                .map_err(|e| format!("{e:?}"))?;
            String::from_utf8(source.bytes.to_vec()).map_err(|e| e.to_string())
        },
    )?;
    let mut catalog = AssetCatalog::new(1, 1)?;
    let mut sources = SourceDependencies::new(1, 2);
    let first = catalog.request(id.clone())?;
    imports.submit(&first)?;
    assert_eq!(imports.submit(&first), Err(ImportWorkerError::Busy));
    let result = completion(&mut imports)?;
    catalog.complete_observed(&mut sources, &result.ticket, result.result)?;
    let old = catalog.snapshot(&id).unwrap();
    let obsolete = catalog.request(id.clone())?;
    imports.submit(&obsolete)?;
    let current = catalog.request(id.clone())?;
    let result = completion(&mut imports)?;
    assert_eq!(
        catalog.complete_observed(&mut sources, &result.ticket, result.result),
        Err(PublicationError::Asset(AssetError::StaleTicket))
    );
    assert!(std::sync::Arc::ptr_eq(
        &old,
        &catalog.snapshot(&id).unwrap()
    ));
    imports.submit(&current)?;
    let result = completion(&mut imports)?;
    catalog.complete_observed(&mut sources, &result.ticket, result.result)?;
    let mut watcher = SourcePollWorker::new(FileInputs::new(&root)?, 1, 1024)?;
    // Initial scan invalidates once, then stable scans are quiet.
    watcher.request(&sources, 1)?;
    assert_eq!(scan(&mut watcher)?, vec![id.clone()]);
    for bytes in [&[255][..], b"replacement"] {
        std::fs::write(root.join(&id.0), bytes)?;
        watcher.request(&sources, 1)?;
        for affected in sources.affected(scan(&mut watcher)?) {
            let ticket = catalog.request(affected)?;
            imports.submit(&ticket)?;
            let result = completion(&mut imports)?;
            catalog.complete_observed(&mut sources, &result.ticket, result.result)?;
        }
        assert_eq!(catalog.pending(), 0);
    }
    assert_eq!(catalog.snapshot(&id).unwrap().value(), "replacement");
    assert_eq!(old.value(), "original");
    watcher.request(&sources, 1)?;
    assert!(scan(&mut watcher)?.is_empty());
    imports.close().join().map_err(|_| "import panic")?;
    watcher.close().join().map_err(|_| "watch panic")?;
    std::fs::remove_dir_all(root)?;
    println!(
        "BACKGROUND IMPORT PASS: decoder runs off owner thread, bounded work, file-triggered failed reload/recovery, immutable old snapshot, joined shutdown"
    );
    Ok(())
}
