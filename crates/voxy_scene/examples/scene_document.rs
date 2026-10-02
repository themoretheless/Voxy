//! Load an authored scene, simulate an edit, save and reload with fresh handles.
use voxy_scene::{ComponentRegistry, ObjectId, SceneDocument};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = ComponentRegistry::default();
    registry.register::<u32>("game.health.v1")?;
    registry.register_with_references::<ObjectId>("game.target.v1", |value| vec![value.clone()])?;
    let document = SceneDocument::from_json(include_str!("data/game.scene.json"))?;
    let mut loaded = document.load(&registry, 1024)?;
    let player = loaded.resolve(&ObjectId("player".into())).unwrap();
    *loaded.graph.component_mut::<u32>(player)?.unwrap() -= 25;
    let target = loaded.graph.component::<ObjectId>(player)?.unwrap();
    assert!(loaded.resolve(target).is_some());
    let captured = loaded.capture(&registry)?;
    std::fs::create_dir_all("target")?;
    let path = "target/game-roundtrip.scene.json";
    std::fs::write(path, captured.to_json()?)?;
    let restored =
        SceneDocument::from_json(&std::fs::read_to_string(path)?)?.load(&registry, 1024)?;
    let restored_player = restored.resolve(&ObjectId("player".into())).unwrap();
    assert_ne!(restored_player, player);
    assert_eq!(restored.graph.component::<u32>(restored_player)?, Some(&75));
    assert_eq!(restored.capture(&registry)?, captured);
    println!("SCENE DOCUMENT PASS: saved {path}, restored health=75 with fresh runtime handles");
    Ok(())
}
