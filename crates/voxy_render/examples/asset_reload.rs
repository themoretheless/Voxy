//! Background OBJ import and owner-thread publication with stale-result rejection.
use voxy_assets::{AssetCatalog, AssetDependencies, AssetError, AssetId, AssetStatus};
use voxy_render::{ObjAsset, ObjLimits};
fn decode(source: &'static str) -> std::thread::JoinHandle<Result<ObjAsset, String>> {
    std::thread::spawn(move || {
        ObjAsset::parse(source, ObjLimits::default()).map_err(|e| e.to_string())
    })
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    const OBJ: &str = include_str!("assets/quad.obj");
    let mut assets = AssetCatalog::new(8, 2)?;
    let id = AssetId("models/quad".into());
    let first = assets.request(id.clone())?;
    assets.complete(
        &first,
        decode(OBJ).join().map_err(|_| "import worker panic")?,
    )?;
    let held = assets.snapshot(&id).unwrap();
    let obsolete = assets.request(id.clone())?;
    let worker = decode(OBJ);
    let current = assets.request(id.clone())?;
    assets.complete(
        &current,
        decode("not valid OBJ")
            .join()
            .map_err(|_| "import worker panic")?,
    )?;
    assert!(matches!(assets.status(&id), Some(AssetStatus::Failed(_))));
    assert!(std::sync::Arc::ptr_eq(
        &held,
        &assets.snapshot(&id).unwrap()
    ));
    assert_eq!(
        assets.complete(&obsolete, worker.join().map_err(|_| "import worker panic")?),
        Err(AssetError::StaleTicket)
    );
    let source = AssetId("sources/quad.obj".into());
    let mut dependencies = AssetDependencies::new(8, 16);
    dependencies.declare(source.clone())?;
    dependencies.declare(id.clone())?;
    dependencies.set(&id, [source.clone()])?;
    let affected = dependencies.affected([source])?;
    assert_eq!(affected.last(), Some(&id));
    for asset in affected {
        if asset == id {
            let retry = assets.request(asset)?;
            assets.complete(
                &retry,
                decode(OBJ).join().map_err(|_| "import worker panic")?,
            )?;
        }
    }
    assert_eq!(assets.status(&id), Some(&AssetStatus::Ready));
    assert!(!std::sync::Arc::ptr_eq(
        &held,
        &assets.snapshot(&id).unwrap()
    ));
    assert_eq!(
        held.mesh.indices(),
        assets.snapshot(&id).unwrap().mesh.indices()
    );
    println!(
        "ASSET RELOAD PASS: background OBJ import, failed reload keeps geometry, obsolete result rejected"
    );
    Ok(())
}
