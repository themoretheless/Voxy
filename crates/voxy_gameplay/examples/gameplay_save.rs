//! Run a window-free falling character, save its owners, then resume in a new scene.
//! Animation, audio and UI are not participants in this example.
use voxy_gameplay::{
    CharacterPhysics, GameplayCheckpoint, GameplayCheckpointLimits, PhysicsError,
    PreparedGameplayCheckpoint, player_input, register_components,
};
use voxy_scene::{
    BehaviorCheckpointRegistry, ComponentRegistry, SceneDocument, SceneSimulation, SimulationLimits,
};

fn advance(game: &mut PreparedGameplayCheckpoint) -> Result<(), Box<dyn std::error::Error>> {
    let PreparedGameplayCheckpoint {
        scene,
        simulation,
        physics,
        input,
        fixed_ticks,
    } = game;
    simulation.advance_with(&mut scene.graph, 0.019, |graph, step| {
        physics.fixed_step(graph, input, step)?;
        *fixed_ticks += 1;
        Ok::<_, PhysicsError>(())
    })?;
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = ComponentRegistry::default();
    register_components(&mut registry)?;
    let limits = GameplayCheckpointLimits {
        simulation: SimulationLimits {
            fixed_step: 1. / 60.,
            max_steps: 4,
            max_behaviors: 8,
            max_commands: 16,
        },
        scene_capacity: 8,
        max_bodies: 4,
        max_colliders: 4,
        max_actions: 131,
        max_bindings: 5,
        max_bytes: 1024 * 1024,
    };
    let document = SceneDocument::from_json(
        r#"{"version":1,"objects":[
      {"id":"body","parent":null,"name":"body","active":true,"translation":[0,2,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{"game.character.v1":{"half_extents":[0.1,0.1,0.1],"speed":1,"gravity":-9.8,"jump_speed":3}}},
      {"id":"floor","parent":null,"name":"floor","active":true,"translation":[0,-0.1,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{"game.box.v1":{"half_extents":[10,0.1,10]}}}
    ]}"#,
    )?;
    let scene = document.load(&registry, limits.scene_capacity)?;
    let simulation = SceneSimulation::new(&scene.graph, limits.simulation)?;
    let physics = CharacterPhysics::new(&scene.graph, limits.max_bodies, limits.max_colliders);
    let mut source = PreparedGameplayCheckpoint {
        scene,
        simulation,
        physics,
        input: player_input()?,
        fixed_ticks: 0,
    };
    for _ in 0..17 {
        advance(&mut source)?;
    }
    // This constant names the fixed data and adapter contract of this example.
    // A project launcher must derive its own key from its actual inputs.
    let compatibility = "gameplay-save-example-v1";
    let saved = source.checkpoint(&registry, compatibility, limits)?;
    let json = saved.to_json(limits.max_bytes)?;
    let parsed = GameplayCheckpoint::from_json(&json, limits.max_bytes)?;
    let mut resumed = parsed.prepare(
        &registry,
        &BehaviorCheckpointRegistry::default(),
        compatibility,
        limits,
    )?;
    for _ in 0..240 {
        advance(&mut source)?;
        advance(&mut resumed)?;
        let a = source
            .checkpoint(&registry, compatibility, limits)?
            .to_json(limits.max_bytes)?;
        let b = resumed
            .checkpoint(&registry, compatibility, limits)?
            .to_json(limits.max_bytes)?;
        if a != b {
            return Err("resumed state diverged".into());
        }
    }
    println!(
        "GAMEPLAY CHECKPOINT PASS bytes={} following_frames=240 ticks={} full_app_save=false",
        json.len(),
        resumed.fixed_ticks
    );
    Ok(())
}
