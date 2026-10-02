//! The same durable playable fixture, driven through public APIs without a window.
use voxy_gameplay::{CharacterBody, CharacterPhysics, JUMP, RIGHT, player_input};
use voxy_scene::{
    ComponentRegistry, ObjectId, SceneCommand, SceneDocument, SceneExtraction, SceneSimulation,
    SimulationLimits,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = ComponentRegistry::default();
    registry.register::<String>("editor.model.v1")?;
    voxy_gameplay::register_components(&mut registry)?;
    let document = SceneDocument::from_json(include_str!(
        "../../voxy_editor/examples/game/game.scene.json"
    ))?;
    let mut loaded = document.load(&registry, 8)?;
    let player = loaded
        .resolve(&ObjectId("player".into()))
        .ok_or("missing player")?;
    let mut simulation = SceneSimulation::new(
        &loaded.graph,
        SimulationLimits {
            fixed_step: 1.0 / 60.0,
            max_steps: 8,
            max_behaviors: 4,
            max_commands: 8,
        },
    )?;
    let mut physics = CharacterPhysics::new(&loaded.graph, 4, 4);
    let mut input = player_input()?;
    let plan = voxy_gameplay::character_schedule()?;
    for _ in 0..100 {
        simulation.advance_scoped(&mut loaded.graph, 0.01, plan, |system, scene, dt| {
            physics.run_scoped_system(system, scene, &mut input, dt)
        })?;
    }
    assert!(
        physics
            .state(&loaded.graph, player)?
            .ok_or("missing body")?
            .grounded
    );
    let original = loaded.graph.local(player)?;
    input.event(RIGHT, 1.0)?;
    input.event(JUMP, 1.0)?;
    input.event(JUMP, 0.0)?;
    for _ in 0..20 {
        simulation.advance_scoped(&mut loaded.graph, 1.0 / 60.0, plan, |system, scene, dt| {
            physics.run_scoped_system(system, scene, &mut input, dt)
        })?;
    }
    assert!(loaded.graph.local(player)?.translation.x > original.translation.x + 0.15);
    assert!(loaded.graph.local(player)?.translation.y > original.translation.y + 0.08);
    let mut rendering = SceneExtraction::<String>::new(8);
    voxy_scene::extraction_schedule()?.run_scene(&mut loaded.graph, |_, access| {
        rendering.refresh_scoped_with(access, |scene, owner| {
            simulation
                .render_world(scene, owner)
                .map_err(|_| voxy_scene::SceneGraphError::InvalidTransform)
        })
    })?;
    assert_eq!(rendering.instances().len(), 3);
    let saved = loaded.capture(&registry)?;
    let restored = saved.load(&registry, 8)?;
    let restored_player = restored
        .resolve(&ObjectId("player".into()))
        .ok_or("missing restored player")?;
    assert_ne!(restored_player, player);
    assert!(
        restored
            .graph
            .component::<CharacterBody>(restored_player)?
            .is_some()
    );
    simulation
        .commands()
        .remove_component::<CharacterBody>(player)?;
    for result in simulation.advance(&mut loaded.graph, 0.0)?.commands {
        result?;
    }
    physics.synchronize(&loaded.graph)?;
    assert_eq!(physics.body_count(), 0);
    simulation
        .commands()
        .push(SceneCommand::RemoveSubtree(player))?;
    for result in simulation.advance(&mut loaded.graph, 0.0)?.commands {
        result?;
    }
    simulation.stop(&mut loaded.graph)?;
    voxy_scene::extraction_schedule()?.run_scene(&mut loaded.graph, |_, access| {
        rendering.refresh_scoped_with(access, |scene, owner| scene.world_matrix(owner))
    })?;
    assert_eq!(rendering.instances().len(), 2);
    println!(
        "CHARACTER SCENE PASS: swept motion, quick-tap jump over step, interpolation, durable reload, typed barrier removal, lifecycle Stop"
    );
    Ok(())
}
