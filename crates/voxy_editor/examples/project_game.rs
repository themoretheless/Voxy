//! Application-owned game behavior using public editor/project/package APIs.
//! `project_game --check examples/host-game/project.voxy` runs without a window.
use std::path::Path;
use voxy_editor::{BehaviorSetup, GameHost, ViewportMode};
use voxy_scene::{Behavior, NodeId, SceneGraph};

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MoveConfig {
    speed: f32,
}
#[derive(Debug)]
struct Move {
    speed: f32,
    fixed_ticks: u64,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct MoveState {
    speed_bits: u32,
    fixed_ticks: u64,
}
impl Behavior for Move {
    fn checkpoint(&self) -> Result<voxy_scene::BehaviorState, voxy_scene::BehaviorCheckpointError> {
        voxy_scene::BehaviorState::json(
            "sample.move-state.v1",
            &MoveState {
                speed_bits: self.speed.to_bits(),
                fixed_ticks: self.fixed_ticks,
            },
        )
    }
    fn fixed_update(&mut self, scene: &mut SceneGraph, owner: NodeId, dt: f64) {
        let mut pose = scene.local(owner).expect("live behavior owner");
        pose.translation.x += self.speed * dt as f32;
        scene.set_local(owner, pose).expect("finite sample motion");
        self.fixed_ticks += 1;
    }
    fn on_destroy(&mut self, scene: &mut SceneGraph, owner: NodeId) {
        if let Ok(pose) = scene.local(owner) {
            println!(
                "HOST MOVE END fixed={} x={:.6}",
                self.fixed_ticks, pose.translation.x
            );
        }
    }
}
fn game_host() -> Result<GameHost, Box<dyn std::error::Error>> {
    let mut registry = voxy_editor::editor_component_registry()?;
    registry.register::<MoveConfig>("sample.move.v1")?;
    let mut host = GameHost::new(registry);
    let mut checkpoints = voxy_scene::BehaviorCheckpointRegistry::default();
    checkpoints.register_with("sample.move-state.v1", |payload, _| {
        let state: MoveState = serde_json::from_slice(payload)
            .map_err(|error| voxy_scene::BehaviorCheckpointError::Codec(error.to_string()))?;
        let speed = f32::from_bits(state.speed_bits);
        if !speed.is_finite() || speed.abs() > 10. {
            return Err(voxy_scene::BehaviorCheckpointError::Invalid(
                "invalid saved sample speed".into(),
            ));
        }
        Ok(Box::new(Move {
            speed,
            fixed_ticks: state.fixed_ticks,
        }))
    })?;
    host.behavior_checkpoints = std::sync::Arc::new(checkpoints);

    host.behaviors = Some(BehaviorSetup::new(|scene, simulation| {
        let owners: Vec<_> = scene
            .components::<MoveConfig>()
            .map(|(id, value)| (id, *value))
            .collect();
        for (owner, config) in owners {
            if !config.speed.is_finite() || config.speed.abs() > 10. {
                return Err("sample speed must be finite and within [-10, 10]".into());
            }
            simulation.attach(
                scene,
                owner,
                Move {
                    speed: config.speed,
                    fixed_ticks: 0,
                },
            )?;
        }
        Ok(())
    }));
    Ok(host)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [mode, path] if mode == "--behavior-check" => {
            let mut game = voxy_editor::HeadlessGame::open_project(Path::new(path), game_host()?)?;
            for _ in 0..120 { game.advance(1. / 60.)?; }
            let saved = game.behavior_checkpoint(1024 * 1024)?;
            let text = saved.to_json(128, 1024 * 1024)?;
            let decoded = voxy_scene::BehaviorCheckpoint::from_json(&text, 128, 1024 * 1024)?;
            if decoded != saved { return Err("behavior checkpoint codec changed sample state".into()); }
            game.stop()?;
            println!("HOST BEHAVIOR CHECKPOINT PASS bytes={} ticks=120 full_game_save=false", text.len());
            Ok(())
        }

        [mode, path] if mode == "--check" || mode == "--editor" || mode == "--game" => {
            let mode = match mode.as_str() {
                "--check" => ViewportMode::GameCheck,
                "--editor" => ViewportMode::Interactive,
                _ => ViewportMode::Game,
            };
            voxy_editor::run_project_with_host(Path::new(path), mode, game_host()?)
        }
        [mode, path, output] if mode == "--build" => {
            voxy_editor::export_project_package_with_host(Path::new(path), Path::new(output), game_host()?)
        }
        [mode, path] if mode == "--package-check" || mode == "--package" => {
            let mode = if mode == "--package-check" { ViewportMode::GameCheck } else { ViewportMode::Game };
            voxy_editor::run_packaged_game_with_host(Path::new(path), mode, game_host()?)
        }
        _ => Err("usage: project_game --behavior-check|--check|--editor|--game project.voxy | --build project.voxy game.vpak | --package-check|--package game.vpak".into()),
    }
}
