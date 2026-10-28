//! Shared input, physics, UI and audio project; native modes require explicit flags.
use std::path::Path;
use voxy_editor::{GameHost, HeadlessGame, ViewportMode};
use winit::keyboard::KeyCode;
fn host() -> Result<GameHost, Box<dyn std::error::Error>> {
    let mut host = GameHost::new(voxy_editor::editor_component_registry()?);
    host.ui_actions = Some(voxy_gameplay::UiActionSetup::new(|handlers| {
        handlers.register("sample.dismiss".into(), |_, event, commands| {
            commands
                .push(voxy_scene::SceneCommand::SetActive(event.owner, false))
                .map_err(|error| error.to_string().into())
        })
    }));
    Ok(host)
}
fn replay(mut game: HeadlessGame) -> Result<(), Box<dyn std::error::Error>> {
    game.pointer([20., 20.], [1280., 720.], true)?;
    game.pointer([20., 20.], [1280., 720.], false)?;
    game.key(KeyCode::ArrowRight, true)?;
    let mut max_height = 0_f32;
    for tick in 0..180 {
        if tick == 25 {
            game.key(KeyCode::Space, true)?;
        }
        if tick == 26 {
            game.key(KeyCode::Space, false)?;
        }
        game.advance(1.0 / 60.0)?;
        let scene = game.snapshot()?;
        let player = scene
            .objects
            .iter()
            .find(|o| o.name == "player")
            .ok_or("player missing")?;
        max_height = max_height.max(player.translation[1]);
    }
    game.key(KeyCode::ArrowRight, false)?;
    let scene = game.snapshot()?;
    let player = scene
        .objects
        .iter()
        .find(|o| o.name == "player")
        .ok_or("player missing")?;
    let button = scene
        .objects
        .iter()
        .find(|o| o.name == "dismiss")
        .ok_or("button missing")?;
    let (frames, peak) = game.audio_stats().ok_or("audio not started")?;
    if !(player.translation[0] > 0.7
        && player.translation[0] < 0.801
        && player.translation[1] >= 0.099
        && player.translation[1] < 0.11
        && max_height > 0.3
        && !button.active
        && frames > 0
        && peak > 0.)
    {
        return Err(format!("combined scenario failed: player={:?}, max_height={max_height}, button={}, frames={frames}, peak={peak}",player.translation,button.active).into());
    }
    println!(
        "INTERACTIVE RESULT {}",
        serde_json::json!({"scene":scene,"ticks":game.fixed_ticks(),"max_height":max_height,"audio_frames":frames,"audio_peak":peak,"camera":game.presentation_camera()})
    );
    game.stop()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [mode, path] if mode == "--check" => replay(HeadlessGame::open_project(Path::new(path),host()?)?),
        [mode, path] if mode == "--package-check" => replay(HeadlessGame::open_package(Path::new(path),host()?)?),
        [mode, path, output] if mode == "--build" => voxy_editor::export_project_package_with_host(Path::new(path),Path::new(output),host()?),
        [mode, path] if mode == "--editor" || mode == "--game" => voxy_editor::run_project_with_host(Path::new(path),if mode == "--editor" { ViewportMode::Interactive } else { ViewportMode::Game },host()?),
        [mode, path] if mode == "--package" => voxy_editor::run_packaged_game_with_host(Path::new(path),ViewportMode::Game,host()?),
        _ => Err("usage: interactive_project --check|--editor|--game project.voxy | --build project.voxy game.vpak | --package-check|--package game.vpak".into()),
    }
}
