//! Simulation and HUD independently consume the same damage events.
use voxy_scene::{ComponentTable, EventChannel, NodeId, SceneGraph, Transform};
#[derive(Debug)]
struct Damage {
    target: NodeId,
    amount: u32,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = SceneGraph::new(2);
    let player = scene.spawn(None, Transform::default())?;
    let mut health = ComponentTable::new(&scene);
    health.insert(&scene, player, 100_u32)?;
    let mut events = EventChannel::new(32)?;
    let mut simulation = events.subscribe(false);
    let mut hud = events.subscribe(false);
    for amount in [10, 15] {
        if events
            .emit(Damage {
                target: player,
                amount,
            })?
            .is_some()
        {
            return Err("damage queue overflow".into());
        }
    }
    let batch = events.read(&mut simulation)?;
    if batch.missed != 0 {
        return Err("simulation lost damage events".into());
    }
    for damage in batch.events {
        if let Some(value) = health.get_mut(&scene, damage.target)? {
            *value = value.saturating_sub(damage.amount);
        }
    }
    let batch = events.read(&mut hud)?;
    assert_eq!(batch.missed, 0);
    let displayed_damage: u32 = batch.events.iter().map(|event| event.amount).sum();
    assert_eq!(displayed_damage, 25);
    assert_eq!(health.get(&scene, player)?, Some(&75));
    assert!(events.read(&mut simulation)?.events.is_empty());
    println!("EVENT GAMEPLAY PASS: health=75, independent HUD damage=25, no repeated delivery");
    Ok(())
}
