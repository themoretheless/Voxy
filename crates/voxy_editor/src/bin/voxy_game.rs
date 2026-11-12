//! Native package player entry point.
use std::{ffi::OsStr, path::PathBuf};
use voxy_editor::{ViewportMode, bundled_game_package, run_packaged_game};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let bundled =
        || -> std::io::Result<PathBuf> { bundled_game_package(&std::env::current_exe()?) };
    let (package, mode) = match arguments.as_slice() {
        [] => (bundled()?, ViewportMode::Game),
        [flag] if flag == "--embedded-game-check" => (bundled()?, ViewportMode::GameCheck),
        [flag] if flag == "--embedded-game-native-smoke" => (bundled()?, ViewportMode::GameSmoke),
        [flag] if flag == "--help" || flag == "-h" => {
            println!(
                "voxy_game [--package GAME.vpak [--game | --game-check | --game-native-smoke]]"
            );
            return Ok(());
        }
        [flag, package, mode @ ..] if flag == "--package" || flag == "--game-package" => {
            let mode = match mode {
                [] => ViewportMode::Game,
                [mode] if mode == OsStr::new("--game") => ViewportMode::Game,
                [mode] if mode == OsStr::new("--game-check") => ViewportMode::GameCheck,
                [mode] if mode == OsStr::new("--game-native-smoke") => ViewportMode::GameSmoke,
                _ => return Err("unknown game mode; see voxy_game --help".into()),
            };
            (PathBuf::from(package), mode)
        }
        _ => return Err("unknown argument; see voxy_game --help".into()),
    };
    run_packaged_game(&package, mode)
}
