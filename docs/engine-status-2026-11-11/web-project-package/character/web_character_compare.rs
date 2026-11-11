use std::path::PathBuf;
use voxy_editor::{GameHost, HeadlessGame};
use winit::keyboard::KeyCode::{ArrowLeft, ArrowRight, Space};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(std::env::args_os().nth(1).ok_or("missing package")?);
    let mut game = HeadlessGame::open_package(
        &path,
        GameHost::new(voxy_editor::editor_component_registry()?),
    )?;
    let snap =
        |label: &str, game: &mut HeadlessGame, count| -> Result<(), Box<dyn std::error::Error>> {
            for _ in 0..count {
                game.advance(1.0 / 60.0)?;
            }
            println!(
                "PROBE {label} {} {}",
                game.fixed_ticks(),
                game.snapshot()?.to_json()?
            );
            Ok(())
        };
    snap("settled", &mut game, 30)?;
    game.key(ArrowRight, true)?;
    snap("wall", &mut game, 45)?;
    game.key(ArrowRight, false)?;
    game.key(Space, true)?;
    game.key(Space, false)?;
    snap("jump", &mut game, 11)?;
    snap("landed", &mut game, 40)?;
    game.key(ArrowLeft, true)?;
    snap("left", &mut game, 8)?;
    game.key(ArrowLeft, false)?;
    snap("stopped", &mut game, 10)?;
    Ok(())
}
