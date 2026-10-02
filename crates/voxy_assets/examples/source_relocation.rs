//! Physical source rename preserves the logical output, with owner invalidation.
use std::time::{Duration, Instant};
use voxy_assets::{
    AssetCatalog, AssetError, AssetId, AssetImportWorker, AssetLocations, FileInputs,
    ImportCompletion, ImportWorkerError, PublicationError, SourceDependencies, SourcePath,
};
fn wait(
    worker: &mut AssetImportWorker<String>,
) -> Result<ImportCompletion<String>, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = worker.try_result()? {
            return Ok(result);
        }
        if Instant::now() >= deadline {
            return Err("worker timeout".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "voxy-relocation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir(&root)?;
    std::fs::write(root.join("original.txt"), "original")?;
    let stable = AssetId("logical-resource-1".into());
    let old_path = SourcePath::new("original.txt")?;
    let new_path = SourcePath::new("renamed.txt")?;
    let mut locations = AssetLocations::new(1, 128);
    locations.bind(stable.clone(), old_path.clone())?;
    let mut catalog = AssetCatalog::new(1, 1)?;
    let mut sources = SourceDependencies::new(1, 1);
    let mut worker = AssetImportWorker::new_with_locations(
        FileInputs::new(&root)?,
        1,
        128,
        0,
        |asset, source, provider, inputs, _| {
            assert_eq!(asset.0, "logical-resource-1");
            let snapshot = inputs
                .read(source.observation_id(), |id, limit| {
                    provider.read(id, limit)
                })
                .map_err(|e| format!("{e:?}"))?;
            String::from_utf8(snapshot.bytes.to_vec()).map_err(|e| e.to_string())
        },
    )?;
    let first = catalog.prepare_import(stable.clone(), &[])?;
    assert_eq!(
        worker.submit_import(&first),
        Err(ImportWorkerError::LocationRequired)
    );
    worker.submit_at(&first, locations.source(&stable).unwrap())?;
    let result = wait(&mut worker)?;
    catalog.complete_observed(&mut sources, &result.ticket, result.result)?;
    let held = catalog.snapshot(&stable).unwrap();
    let obsolete = catalog.prepare_import(stable.clone(), &[])?;
    worker.submit_at(&obsolete, locations.source(&stable).unwrap())?;
    catalog.invalidate(std::slice::from_ref(&stable))?;
    std::fs::rename(root.join(old_path.as_str()), root.join(new_path.as_str()))?;
    locations.relocate(&stable, new_path.clone())?;
    let manifest = root.join("assets.json");
    locations.save_manifest(&manifest, 1024)?;
    locations = AssetLocations::load_manifest(&manifest, 1, 128, 1024)?;
    assert_eq!(locations.source(&stable), Some(&new_path));
    std::fs::write(root.join(new_path.as_str()), "replacement")?;
    let obsolete_result = wait(&mut worker)?;
    assert_eq!(obsolete_result.source, Some(old_path.clone()));
    assert_eq!(
        catalog.complete_observed(
            &mut sources,
            &obsolete_result.ticket,
            obsolete_result.result
        ),
        Err(PublicationError::Asset(AssetError::StaleTicket))
    );
    let fresh = catalog.prepare_import(stable.clone(), &[])?;
    worker.submit_at(&fresh, locations.source(&stable).unwrap())?;
    let result = wait(&mut worker)?;
    assert_eq!(result.source, Some(new_path.clone()));
    catalog.complete_observed(&mut sources, &result.ticket, result.result)?;
    assert_eq!(catalog.snapshot(&stable).unwrap().value(), "replacement");
    assert_eq!(held.value(), "original");
    assert!(sources.affected([old_path.observation_id()]).is_empty());
    assert_eq!(
        sources.affected([new_path.observation_id()]),
        vec![stable.clone()]
    );
    assert_eq!(locations.asset_at(&new_path), Some(&stable));
    assert_eq!(catalog.pending(), 0);
    worker.close().join().map_err(|_| "worker panic")?;
    std::fs::remove_dir_all(root)?;
    println!(
        "SOURCE RELOCATION PASS: stable output identity, captured old/new locations, stale completion rejected, source invalidation mapping replaced, old snapshot survives"
    );
    Ok(())
}
