//! Batch gameplay storage with authoring import and explicit persistence extraction.
use voxy_input::{Binding, Control, InputMap};
use voxy_scene::{
    ComponentRegistry, ComponentTable, ObjectId, SceneCommand, SceneCommands, SceneDocument,
    SchedulePlan, SystemAccess, SystemSpec,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = ComponentRegistry::default();
    registry.register::<u32>("game.health.v1")?;
    registry.register_with_references::<ObjectId>("game.target.v1", |value| vec![value.clone()])?;
    let document = SceneDocument::from_json(include_str!("data/game.scene.json"))?;
    let mut world = document.load(&registry, 1024)?;
    let player = world.resolve(&ObjectId("player".into())).unwrap();
    let level = world.resolve(&ObjectId("level".into())).unwrap();
    let mut health = ComponentTable::new(&world.graph);
    // Transfer ownership of hot gameplay values out of node-level authoring storage.
    let initial = world.graph.remove_component::<u32>(player)?.unwrap();
    health.insert(&world.graph, player, initial - 25)?;
    let plan = health_plan()?;
    let regenerate = |health: &mut ComponentTable<u32>,
                      graph: &voxy_scene::SceneGraph|
     -> Result<(), voxy_scene::ScheduleError> {
        plan.run(|system| match system {
            "regenerate" => {
                for (_, value) in health.query_mut(graph, true).map_err(|e| e.to_string())? {
                    *value = value.saturating_add(1).min(100);
                }
                Ok(())
            }
            "validate" => {
                if health
                    .get(graph, player)
                    .map_err(|e| e.to_string())?
                    .is_some_and(|value| *value > 100)
                {
                    Err("invalid health".into())
                } else {
                    Ok(())
                }
            }
            _ => Err("unknown system".into()),
        })
    };
    regenerate(&mut health, &world.graph)?;
    let pause_control = Control { device: 0, code: 1 };
    let mut input = InputMap::new(8, 16);
    input.bind(
        "pause_world",
        vec![Binding {
            control: pause_control,
            scale: 1.0,
        }],
    )?;
    input.event(pause_control, 1.0)?;
    if input.state("pause_world").unwrap().pressed {
        world.graph.set_active(level, false)?;
    }
    input.finish_frame();
    input.event(pause_control, 0.0)?;
    regenerate(&mut health, &world.graph)?;
    assert_eq!(health.get(&world.graph, player)?, Some(&76));
    input.finish_frame();
    input.event(pause_control, 1.0)?;
    if input.state("pause_world").unwrap().pressed {
        world.graph.set_active(level, true)?;
    }
    regenerate(&mut health, &world.graph)?;
    assert_eq!(health.get(&world.graph, player)?, Some(&77));
    // Explicit extraction bridges simulation state back to the durable schema.
    world
        .graph
        .insert_component(player, *health.get(&world.graph, player)?.unwrap())?;
    let saved = world.capture(&registry)?;
    let restored = saved.load(&registry, 1024)?;
    assert_eq!(
        restored
            .graph
            .component::<u32>(restored.resolve(&ObjectId("player".into())).unwrap())?,
        Some(&77)
    );
    let mut commands = SceneCommands::new(&world.graph, 16);
    commands.push(SceneCommand::RemoveSubtree(level))?;
    assert!(health.get(&world.graph, player)?.is_some());
    for result in commands.apply(&mut world.graph)? {
        result?;
    }
    assert_eq!(health.synchronize(&world.graph)?, 1);
    assert_eq!(health.stored_len(), 0);
    println!(
        "BATCH GAMEPLAY PASS: typed health storage, activity, extraction, roundtrip and cleanup"
    );
    Ok(())
}

fn health_plan() -> Result<SchedulePlan, voxy_scene::ScheduleError> {
    SchedulePlan::build(
        &[
            SystemSpec {
                name: "regenerate".into(),
                phase: 0,
                after: vec![],
                access: vec![
                    SystemAccess {
                        resource: "health".into(),
                        write: true,
                    },
                    SystemAccess {
                        resource: "hierarchy".into(),
                        write: false,
                    },
                ],
            },
            SystemSpec {
                name: "validate".into(),
                phase: 1,
                after: vec!["regenerate".into()],
                access: vec![SystemAccess {
                    resource: "health".into(),
                    write: false,
                }],
            },
        ],
        16,
    )
}
