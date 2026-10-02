//! Bounds selection maps a runtime hit to a durable ID for an undoable authoring edit.
use glam::Vec3;
use voxy_scene::{ComponentRegistry, ObjectId, PickBounds, PickRay, SceneDocument, SceneHistory};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = ComponentRegistry::default();
    registry.register::<u32>("game.health.v1")?;
    registry.register_with_references::<ObjectId>("game.target.v1", |id| vec![id.clone()])?;
    let document = SceneDocument::from_json(include_str!("data/game.scene.json"))?;
    let mut history = SceneHistory::new(document, &registry, 1024, 64, 1_000_000)?;
    let mut view = history.current().load(&registry, 1024)?;
    let player = view.resolve(&ObjectId("player".into())).unwrap();
    view.graph.insert_component(
        player,
        PickBounds {
            min: -Vec3::ONE,
            max: Vec3::ONE,
            layers: 1,
        },
    )?;
    let hit = view
        .graph
        .pick(
            PickRay::new(Vec3::new(12.0, 0.0, -10.0), Vec3::Z, 100.0)?,
            1,
        )?
        .hit
        .unwrap();
    let selected = view.identity(hit.node).unwrap().clone();
    assert_eq!(selected, ObjectId("player".into()));
    history.edit(&registry, |document| {
        document
            .objects
            .iter_mut()
            .find(|object| object.id == selected)
            .unwrap()
            .name = "Selected player".into();
        Ok(())
    })?;
    assert!(history.undo());
    assert_eq!(history.current().objects[0].name, "Player");
    println!(
        "PICKING AUTHORING PASS: selected player at distance {}, edit undone by durable identity",
        hit.distance
    );
    Ok(())
}
