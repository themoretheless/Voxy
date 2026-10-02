//! Threaded diamond rebuilds; only owner publication unlocks dependent imports.
use std::time::{Duration, Instant};
use voxy_assets::{
    AssetCatalog, AssetDependencies, AssetId, AssetImportWorker, AssetStatus, FileInputs,
    ImportedAsset, RebuildPlan, RebuildStatus, SourceDependencies,
};
fn id(s: &str) -> AssetId {
    AssetId(s.into())
}
fn rebuild(
    graph: &AssetDependencies,
    catalog: &mut AssetCatalog<ImportedAsset<u32>>,
    sources: &mut SourceDependencies,
    worker: &mut AssetImportWorker<u32>,
) -> Result<RebuildPlan, Box<dyn std::error::Error>> {
    let affected = graph.affected([id("base")])?;
    let resident: Vec<_> = affected
        .iter()
        .filter(|asset| catalog.status(asset).is_some())
        .cloned()
        .collect();
    catalog.invalidate(&resident)?;
    let mut plan = RebuildPlan::new(graph, [id("base")], 1)?;
    while let Some(claim) = plan.claim_ready()? {
        let asset = claim.asset().clone();
        let dependencies: Vec<_> = graph
            .dependencies(&asset)
            .unwrap()
            .iter()
            .cloned()
            .collect();
        let job = catalog.prepare_import(asset.clone(), &dependencies)?;
        worker.submit_rebuild(&claim, &job)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let result = loop {
            if let Some(result) = worker.try_result()? {
                break result;
            }
            if Instant::now() >= deadline {
                return Err("import timeout".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        let published = result.result.is_ok();
        catalog.complete_observed(sources, &result.ticket, result.result)?;
        plan.finish_claim(&result.rebuild.ok_or("missing rebuild attempt")?, published)?;
    }
    assert!(plan.is_finished());
    Ok(plan)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!(
        "voxy-dependency-rebuild-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir(&root)?;
    let mut graph = AssetDependencies::new(4, 4);
    for asset in ["base", "material", "mesh", "scene"] {
        graph.declare(id(asset))?;
    }
    graph.set(&id("material"), [id("base")])?;
    graph.set(&id("mesh"), [id("base")])?;
    graph.set(&id("scene"), [id("material"), id("mesh")])?;
    let mut worker = AssetImportWorker::new_with_dependencies(
        FileInputs::new(&root)?,
        1,
        16,
        2,
        |asset, provider, inputs, dependencies| {
            if asset.0 == "base" {
                let bytes = inputs
                    .read(id("base.txt"), |id, limit| provider.read(id, limit))
                    .map_err(|e| format!("{e:?}"))?;
                std::str::from_utf8(&bytes.bytes)
                    .map_err(|e| e.to_string())?
                    .parse::<u32>()
                    .map_err(|e| e.to_string())
            } else {
                Ok(dependencies
                    .values()
                    .map(|value| *value.value())
                    .sum::<u32>()
                    + 1)
            }
        },
    )?;
    let mut catalog = AssetCatalog::new(4, 1)?;
    let mut sources = SourceDependencies::new(4, 1);
    std::fs::write(root.join("base.txt"), "1")?;
    rebuild(&graph, &mut catalog, &mut sources, &mut worker)?;
    let old = catalog.snapshot(&id("scene")).unwrap();
    assert_eq!(*old.value(), 5);
    std::fs::write(root.join("base.txt"), "2")?;
    assert_eq!(sources.affected([id("base.txt")]), vec![id("base")]);
    rebuild(&graph, &mut catalog, &mut sources, &mut worker)?;
    assert_eq!(*catalog.snapshot(&id("scene")).unwrap().value(), 7);
    assert_eq!(*old.value(), 5);
    std::fs::write(root.join("base.txt"), "broken")?;
    let failed = rebuild(&graph, &mut catalog, &mut sources, &mut worker)?;
    assert_eq!(failed.status(&id("base")), Some(RebuildStatus::Failed));
    assert_eq!(failed.status(&id("scene")), Some(RebuildStatus::Blocked));
    assert_eq!(*catalog.snapshot(&id("scene")).unwrap().value(), 7);
    assert_eq!(catalog.pending(), 0);
    assert_eq!(catalog.status(&id("scene")), Some(&AssetStatus::Dirty));
    worker.close().join().map_err(|_| "worker panic")?;
    std::fs::remove_dir_all(root)?;
    println!(
        "DEPENDENCY REBUILD PASS: threaded diamond publication order, replacement snapshots, failed source blocks dependent jobs and retains last good scene"
    );
    Ok(())
}
