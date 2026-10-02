use voxy_input::{Binding, Control, InputMap};
use voxy_rush::{ScriptEvent, ScriptManager, rush::StateValue};
use voxy_scene::{EventChannel, SceneGraph, Transform};
fn main() -> Result<(), String> {
    let mut scene = SceneGraph::new(16);
    let player = scene
        .spawn(None, Transform::default())
        .map_err(|e| e.to_string())?;
    let mut scripts = ScriptManager::new(16, 64);
    let file = std::env::args()
        .nth(1)
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/examples/player.r").into());
    scripts.attach(
        &mut scene,
        player,
        file,
        vec!["distance".into(), "events".into()],
    )?;
    let mut input = InputMap::new(2, 2);
    for (name, code) in [("move", 0), ("spawn", 1)] {
        input
            .bind(
                name,
                vec![Binding {
                    control: Control { device: 0, code },
                    scale: 1.0,
                }],
            )
            .map_err(|e| e.to_string())?;
        input
            .event(Control { device: 0, code }, 1.0)
            .map_err(|e| e.to_string())?;
    }
    scripts.set_input(&input, &["move", "spawn"])?;
    scripts.fixed_update(&mut scene, 0.5)?;
    scripts.update(&mut scene, 0.5)?;
    input.finish_frame();
    scripts.set_input(&input, &["move", "spawn"])?;
    scripts.fixed_update(&mut scene, 0.5)?;
    scripts.update(&mut scene, 0.5)?;
    let mut events = EventChannel::new(16).map_err(|e| e.to_string())?;
    let mut cursor = events.subscribe(false);
    events
        .emit(ScriptEvent {
            target: Some(player),
            name: "scene.reward".into(),
            payload: StateValue::Null,
        })
        .map_err(|e| e.to_string())?;
    scripts.read_events(&mut scene, &events, &mut cursor)?;
    assert_eq!(scene.local(player).unwrap().translation.x, 4.0);
    assert_eq!(scene.local(player).unwrap().scale.x, 2.0);
    assert_eq!(scene.len(), 2);
    assert!(scripts.diagnostics.is_empty(), "{:?}", scripts.diagnostics);
    println!(
        "player x=4, scale=2; one spawned crate; saved state: {:?}",
        scripts.save(player)?
    );
    scripts.clear(&mut scene)?;
    Ok(())
}
