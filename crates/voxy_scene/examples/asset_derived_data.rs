//! Explicitly joins immutable asset publication with scene-derived data.
use std::sync::Arc;
use voxy_assets::{AssetCatalog, AssetId, AssetStatus};
use voxy_scene::{AssociatedData, SceneGraph, Transform};

fn main() {
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    scene.insert_component(owner, 2_u32).unwrap();
    scene.insert_component(owner, true).unwrap();
    let mut catalog = AssetCatalog::<u32>::new(1, 1).unwrap();
    let asset = AssetId("gameplay/multiplier".into());
    let ticket = catalog.request(asset.clone()).unwrap();
    catalog.complete(&ticket, Ok(10)).unwrap();
    let previous = catalog.snapshot(&asset).unwrap();
    let mut derived = AssociatedData::<u32, bool, u32>::new(&scene);
    derived
        .synchronize::<()>(&scene, |_, n, _, _| Ok(*n * *previous))
        .unwrap();
    assert_eq!(derived.get(&scene, owner).unwrap(), Some(&20));

    let ticket = catalog.request(asset.clone()).unwrap();
    catalog
        .complete(&ticket, Err("reload rejected".into()))
        .unwrap();
    assert!(matches!(
        catalog.status(&asset),
        Some(AssetStatus::Failed(_))
    ));
    let last_good = catalog.snapshot(&asset).unwrap();
    assert!(Arc::ptr_eq(&previous, &last_good));
    derived
        .synchronize::<()>(&scene, |_, _, _, _| panic!("last-good version changed"))
        .unwrap();

    let ticket = catalog.request(asset.clone()).unwrap();
    catalog.complete(&ticket, Ok(30)).unwrap();
    let current = catalog.snapshot(&asset).unwrap();
    // Keep the old strong snapshot alive while comparing immutable versions.
    // No numeric request revision is mistaken for a published content version.
    if !Arc::ptr_eq(&previous, &current) {
        derived.invalidate();
    }
    assert!(derived.get(&scene, owner).unwrap().is_none());
    derived
        .synchronize::<()>(&scene, |_, n, _, old| {
            assert_eq!(old, Some(&20));
            Ok(*n * *current)
        })
        .unwrap();
    assert_eq!(derived.get(&scene, owner).unwrap(), Some(&60));
    assert!(catalog.remove(&asset));
    if catalog.snapshot(&asset).is_none() {
        derived.invalidate();
    }
    assert_eq!(derived.query(&scene).unwrap().count(), 0);
    println!("asset-derived data: failed reload, immutable replacement and removal checks passed");
}
