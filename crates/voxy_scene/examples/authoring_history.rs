//! Authoring undo/redo does not rewind an independently loaded play world.
use voxy_scene::{ComponentRegistry, ObjectId, SceneDocument, SceneHistory};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = ComponentRegistry::default();
    registry.register::<u32>("game.health.v1")?;
    registry.register_with_references::<ObjectId>("game.target.v1", |id| vec![id.clone()])?;
    let document = SceneDocument::from_json(include_str!("data/game.scene.json"))?;
    let mut play = document.load(&registry, 1024)?;
    let player_id = ObjectId("player".into());
    let player = play.resolve(&player_id).unwrap();
    *play.graph.component_mut::<u32>(player)?.unwrap() = 40;
    let mut history = SceneHistory::new(document, &registry, 1024, 64, 1_000_000)?;
    // A gizmo owns its preview while the authoring history remains unchanged.
    let initial = history.current().clone();
    let mut drag = history.begin_edit();
    for position in [1.0, 2.0, 3.0, 4.0, 5.0] {
        let object = drag
            .document_mut()
            .objects
            .iter_mut()
            .find(|object| object.id == player_id)
            .unwrap();
        object.translation[0] = position;
        object.name = "Edited player".into();
        // Preview extraction uses the candidate, not the authoritative play world.
        let preview = drag.document().load(&registry, 1024)?;
        assert_eq!(
            preview.graph.name(preview.resolve(&player_id).unwrap())?,
            "Edited player"
        );
        assert_eq!(history.current(), &initial);
        assert_eq!(play.graph.component::<u32>(player)?, Some(&40));
    }
    assert!(history.commit_edit(drag, &registry)?);
    let edited = history.current().load(&registry, 1024)?;
    assert_eq!(
        edited.graph.name(edited.resolve(&player_id).unwrap())?,
        "Edited player"
    );
    assert!(history.undo());
    let restored = history.current().load(&registry, 1024)?;
    assert_eq!(
        restored.graph.name(restored.resolve(&player_id).unwrap())?,
        "Player"
    );
    assert_eq!(play.graph.component::<u32>(player)?, Some(&40));
    assert!(!history.undo()); // All five preview steps form one history entry.
    assert!(history.redo());
    std::fs::create_dir_all("target")?;
    std::fs::write(
        "target/game-edited.scene.json",
        history.current().to_json()?,
    )?;
    println!(
        "AUTHORING HISTORY PASS: five previews, one undo/redo step; play-world health remains 40"
    );
    Ok(())
}
