use std::path::PathBuf;
use voxy_editor::{GameHost, HeadlessGame, editor_component_registry};
use winit::keyboard::KeyCode::{ArrowLeft, ArrowRight, Space};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(std::env::args_os().nth(1).ok_or("missing package")?);
    let mut game = HeadlessGame::open_package(&path, GameHost::new(editor_component_registry()?))?;
    let period = 1000.0 / 120.0 + 0.00001;
    let mut timestamp = period;
    game.advance(0.0)?;
    println!("CAM {:?}", game.presentation_camera().target.to_array());
    let mut run = |game: &mut HeadlessGame, frames| -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..frames {
            let previous = timestamp;
            timestamp += period;
            game.advance((timestamp - previous) / 1000.0)?;
            println!("CAM {:?}", game.presentation_camera().target.to_array());
        }
        Ok(())
    };
    run(&mut game, 60)?;
    game.key(ArrowRight, true)?;
    run(&mut game, 90)?;
    game.key(ArrowRight, false)?;
    game.key(Space, true)?;
    game.key(Space, false)?;
    run(&mut game, 22)?;
    run(&mut game, 80)?;
    game.key(ArrowLeft, true)?;
    run(&mut game, 16)?;
    game.key(ArrowLeft, false)?;
    run(&mut game, 20)?;
    println!("TICKS {}", game.fixed_ticks());
    Ok(())
}
