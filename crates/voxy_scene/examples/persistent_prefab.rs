//! Two nested authored instances keep links/overrides across an atomic save/load.
use std::{collections::BTreeMap, path::Path};
use voxy_scene::{
    ComponentRegistry, DocumentError, ObjectId, ObjectOverride, PrefabInstance, PrefabLimits,
    PrefabSceneDocument, instance_object_id, read_prefab_scene_file, save_prefab_scene_file,
};
fn instance(id: &str, asset: &str) -> PrefabInstance {
    PrefabInstance {
        id: ObjectId(id.into()),
        asset: asset.into(),
        parent: None,
        overrides: BTreeMap::new(),
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = ComponentRegistry::default();
    registry.register::<u32>("game.health.v1")?;
    registry.register_with_reference_remap::<ObjectId>(
        "game.target.v1",
        |id| vec![id.clone()],
        |id, ids| {
            if let Some(replacement) = ids.get(id) {
                *id = replacement.clone();
            }
        },
    )?;
    let source: PrefabSceneDocument = serde_json::from_str(include_str!("data/game.scene.json"))?;
    let nested = PrefabSceneDocument {
        version: 1,
        objects: vec![],
        instances: vec![instance("level-instance", "game-level")],
    };
    let mut edited = instance("first", "nested-level");
    let local_player = instance_object_id(
        &ObjectId("level-instance".into()),
        &ObjectId("player".into()),
    );
    edited.overrides.insert(
        local_player.clone(),
        ObjectOverride {
            components: BTreeMap::from([("game.health.v1".into(), Some(serde_json::json!(75)))]),
            ..ObjectOverride::default()
        },
    );
    let authored = PrefabSceneDocument {
        version: 1,
        objects: vec![],
        instances: vec![edited, instance("second", "nested-level")],
    };
    let resolve = |asset: &str| match asset {
        "game-level" => Ok(source.clone()),
        "nested-level" => Ok(nested.clone()),
        _ => Err(DocumentError::Invalid(format!(
            "unknown prefab asset {asset}"
        ))),
    };
    std::fs::create_dir_all("target")?;
    let path = Path::new("target/prefab-roundtrip.scene.json");
    save_prefab_scene_file(
        path,
        &authored,
        &registry,
        PrefabLimits::default(),
        64 * 1024,
        resolve,
    )?;
    let restored = read_prefab_scene_file(path, 64 * 1024)?;
    assert_eq!(restored, authored);
    let loaded = restored.load(&registry, PrefabLimits::default(), resolve)?;
    let first = loaded
        .resolve(&instance_object_id(
            &ObjectId("first".into()),
            &local_player,
        ))
        .unwrap();
    let second = loaded
        .resolve(&instance_object_id(
            &ObjectId("second".into()),
            &local_player,
        ))
        .unwrap();
    assert_eq!(loaded.graph.component::<u32>(first)?, Some(&75));
    assert_eq!(loaded.graph.component::<u32>(second)?, Some(&100));
    let link = loaded.graph.component::<ObjectId>(first)?.unwrap();
    assert!(loaded.resolve(link).is_some());
    println!(
        "PERSISTENT PREFAB PASS: nested instances, overrides, remapped target and authoring save/load"
    );
    Ok(())
}
