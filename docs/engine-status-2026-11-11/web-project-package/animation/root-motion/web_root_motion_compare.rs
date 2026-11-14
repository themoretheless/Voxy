use std::path::PathBuf;
use voxy_editor::{GameHost, HeadlessGame, editor_component_registry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let package = PathBuf::from(std::env::args_os().nth(1).ok_or("missing package")?);
    let mut game = HeadlessGame::open_package(&package, GameHost::new(editor_component_registry()?))?;
    for tick in 1..=120 {
        game.advance(1.0 / 60.0)?;
        if [12, 30, 120].contains(&tick) {
            println!("PROBE {} {}", game.fixed_ticks(), game.snapshot()?.to_json()?);
        }
    }
    Ok(())
}
