use crate::*;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "voxy-combined-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn project(&self) -> PathBuf {
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/interactive-project");
        let root = self.0.join("source");
        std::fs::create_dir(&root).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                std::fs::copy(entry.path(), root.join(entry.file_name())).unwrap();
            }
        }
        root.join("project.voxy")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn host() -> Result<GameHost, Box<dyn std::error::Error>> {
    let mut host = GameHost::new(editor_component_registry()?);
    host.ui_actions = Some(voxy_gameplay::UiActionSetup::new(|handlers| {
        handlers.register("sample.dismiss".into(), |_, event, commands| {
            commands
                .push(voxy_scene::SceneCommand::SetActive(event.owner, false))
                .map_err(|error| error.to_string().into())
        })
    }));
    Ok(host)
}
fn object<'a>(scene: &'a voxy_scene::SceneDocument, name: &str) -> &'a voxy_scene::SceneObject {
    scene.objects.iter().find(|o| o.id.0 == name).unwrap()
}
fn replay(
    game: &mut HeadlessGame,
) -> Result<(voxy_scene::SceneDocument, (u64, f32), f32), Box<dyn std::error::Error>> {
    game.pointer([20., 20.], [1280., 720.], true)?;
    game.pointer([20., 20.], [1280., 720.], false)?;
    assert!(object(&game.snapshot()?, "dismiss").active); // Commands publish at advance.
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
        let snapshot = game.snapshot()?;
        let player = object(&snapshot, "player");
        assert!(player.translation[0] <= 0.801, "wall penetration");
        assert!(player.translation[1] >= 0.099, "floor penetration");
        max_height = max_height.max(player.translation[1]);
    }
    game.key(KeyCode::ArrowRight, false)?;
    let snapshot = game.snapshot()?;
    let player = object(&snapshot, "player");
    assert!(player.translation[0] > 0.7 && player.translation[0] < 0.801);
    assert!(player.translation[1] >= 0.099 && player.translation[1] < 0.11);
    assert!(max_height > 0.3, "jump was not delivered");
    assert!(!object(&snapshot, "dismiss").active);
    assert_eq!(game.fixed_ticks(), 180);
    let audio = game.audio_stats().ok_or("audio missing")?;
    assert_eq!(audio.0, 144_000);
    assert!(audio.1 > 0.);
    Ok((snapshot, audio, max_height))
}
#[test]
fn project_play_standalone_and_source_free_package_share_interactive_replay()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = temp.project();
    let config = host()?;
    let mut editor = App::from_project_with_registry(&path, config.components)?;
    editor.ui_action_setup = config.ui_actions;
    let authored = editor.authoring_document()?;
    editor.toggle_play()?;
    let mut play = HeadlessGame {
        app: editor,
        _package: None,
    };
    let expected = replay(&mut play)?;
    play.app.toggle_play()?;
    assert_eq!(play.snapshot()?, authored);
    assert!(play.audio_stats().is_none());
    play.stop()?;
    let mut standalone = HeadlessGame::open_project(&path, host()?)?;
    assert_eq!(replay(&mut standalone)?, expected);
    standalone.stop()?;
    let output = temp.0.join("interactive.vpak");
    export_project_package_with_host(&path, &output, host()?)?;
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut packaged = HeadlessGame::open_package(&output, host()?)?;
    let extracted = packaged
        ._package
        .as_ref()
        .unwrap()
        .project_path()?
        .parent()
        .unwrap()
        .to_path_buf();
    assert!(extracted.exists());
    assert_eq!(replay(&mut packaged)?, expected);
    packaged.stop()?;
    assert!(
        !extracted.exists(),
        "package extraction outlived game shutdown"
    );
    Ok(())
}
#[test]
fn headless_focus_loss_cancels_input_and_ui_capture_and_invalid_events_do_not_advance()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = temp.project();
    let mut game = HeadlessGame::open_project(&path, host()?)?;
    let before = game.snapshot()?;
    for dt in [f64::NAN, f64::INFINITY, -1.] {
        assert!(game.advance(dt).is_err());
    }
    assert!(game.pointer([f32::NAN, 20.], [1280., 720.], true).is_err());
    assert!(game.pointer([20., 20.], [0., 720.], true).is_err());
    assert_eq!(game.fixed_ticks(), 0);
    assert_eq!(game.snapshot()?, before);
    game.key(KeyCode::ArrowRight, true)?;
    for _ in 0..10 {
        game.advance(1.0 / 60.0)?;
    }
    let x = object(&game.snapshot()?, "player").translation[0];
    assert!(x > 0.);
    game.pointer([20., 20.], [1280., 720.], true)?;
    game.set_focused(false);
    for _ in 0..10 {
        game.advance(1.0 / 60.0)?;
    }
    assert!((object(&game.snapshot()?, "player").translation[0] - x).abs() < 1e-6);
    game.set_focused(true);
    game.pointer([20., 20.], [1280., 720.], false)?;
    game.advance(1.0 / 60.0)?;
    assert!(object(&game.snapshot()?, "dismiss").active);
    game.key(KeyCode::Tab, true)?;
    game.key(KeyCode::Space, true)?;
    game.key(KeyCode::Space, false)?;
    game.advance(1.0 / 60.0)?;
    let after = game.snapshot()?;
    assert!(!object(&after, "dismiss").active);
    assert!(
        object(&after, "player").translation[1] < 0.11,
        "focused UI leaked Space into jump"
    );
    game.stop()
}

#[test]
fn gameplay_input_checkpoint_keeps_quick_jump_and_following_simulation()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = temp.project();
    let mut original = HeadlessGame::open_project(&path, host()?)?;
    let mut resumed = HeadlessGame::open_project(&path, host()?)?;
    for game in [&mut original, &mut resumed] {
        game.pointer([20., 20.], [1280., 720.], true)?;
        game.pointer([20., 20.], [1280., 720.], false)?;
        game.advance(1. / 60.)?;
    }
    original.key(KeyCode::ArrowRight, true)?;
    original.key(KeyCode::Space, true)?;
    original.key(KeyCode::Space, false)?;
    let saved = original.input_checkpoint();
    let encoded = voxy_gameplay::encode_input_checkpoint(&saved, 65536, 131, 5)?;
    let input_file = temp.0.join("input-checkpoint.json");
    std::fs::write(&input_file, encoded)?;
    let decoded = voxy_gameplay::decode_input_checkpoint(
        &std::fs::read_to_string(input_file)?,
        65536,
        131,
        5,
    )?;
    resumed.set_focused(false);
    resumed.restore_input_checkpoint(&decoded)?;
    assert_eq!(resumed.input_checkpoint(), saved);
    let mut bad = saved.clone();
    bad.version = 2;
    assert!(resumed.restore_input_checkpoint(&bad).is_err());
    assert_eq!(resumed.input_checkpoint(), saved);
    let start = object(&original.snapshot()?, "player").translation[1];
    let mut peak = start;
    for tick in 0..120 {
        if tick == 30 {
            original.key(KeyCode::ArrowRight, false)?;
            resumed.key(KeyCode::ArrowRight, false)?;
        }
        original.advance(1. / 60.)?;
        resumed.advance(1. / 60.)?;
        let current = original.snapshot()?;
        peak = peak.max(object(&current, "player").translation[1]);
        assert_eq!(current, resumed.snapshot()?);
        assert_eq!(original.input_checkpoint(), resumed.input_checkpoint());
    }
    assert!(peak > start + 0.03, "quick jump was lost");
    original.stop()?;
    resumed.stop()
}
