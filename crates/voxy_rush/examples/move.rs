use voxy_rush::{PositionScript, rush::CancellationToken};
use voxy_scene::{SceneGraph, Transform};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default())?;
    let source = match std::env::args().nth(1) {
        Some(path) => std::fs::read_to_string(path)?,
        None => include_str!("move.r").to_owned(),
    };
    let script = PositionScript::compile(&source)?;
    let cancellation = CancellationToken::default();
    for frame in 0..60 {
        script.update(
            &mut scene,
            owner,
            1.0 / 60.0,
            f64::from(frame) / 60.0,
            &cancellation,
        )?;
    }
    println!(
        "Position after 60 ticks: {:?}",
        scene.local(owner)?.translation
    );
    Ok(())
}
