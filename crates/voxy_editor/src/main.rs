//! Standalone native authoring entry point.
use voxy_editor::{ModelSource, ViewportMode, run_model_viewport_with_scene};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments: Vec<_> = std::env::args().skip(1).collect();
    let scene = if let Some(index) = arguments.iter().position(|arg| arg == "--scene") {
        arguments.remove(index);
        if index >= arguments.len() {
            return Err("--scene requires a path".into());
        }
        Some(std::path::PathBuf::from(arguments.remove(index)))
    } else {
        None
    };
    let mode = if let Some(index) = arguments
        .iter()
        .position(|arg| arg == "--game-native-smoke")
    {
        arguments.remove(index);
        ViewportMode::GameSmoke
    } else if let Some(index) = arguments.iter().position(|arg| arg == "--game-check") {
        arguments.remove(index);
        ViewportMode::GameCheck
    } else if let Some(index) = arguments.iter().position(|arg| arg == "--game") {
        arguments.remove(index);
        ViewportMode::Game
    } else if let Some(index) = arguments.iter().position(|arg| arg == "--prefab-smoke") {
        arguments.remove(index);
        ViewportMode::PrefabSmoke
    } else if let Some(index) = arguments.iter().position(|arg| arg == "--scene3d-smoke") {
        arguments.remove(index);
        ViewportMode::Scene3DSmoke
    } else if let Some(index) = arguments.iter().position(|arg| arg == "--game-smoke") {
        arguments.remove(index);
        ViewportMode::GameplaySmoke
    } else if let Some(index) = arguments.iter().position(|arg| arg == "--lod-smoke") {
        arguments.remove(index);
        ViewportMode::LodSmoke
    } else if let Some(index) = arguments.iter().position(|arg| arg == "--animation-smoke") {
        arguments.remove(index);
        ViewportMode::AnimationSmoke
    } else if let Some(index) = arguments.iter().position(|arg| arg == "--smoke") {
        arguments.remove(index);
        ViewportMode::Smoke
    } else {
        ViewportMode::Interactive
    };
    let source = match arguments.as_slice() {
        [flag, path, asset] if flag == "--manifest" => ModelSource::Manifest {
            path: path.into(), asset: voxy_assets::AssetId(asset.clone()),
        },
        [path] if !path.starts_with("--") => ModelSource::File(path.into()),
        _ => return Err("usage: voxy_editor model.obj|model.gltf|model.glb | --manifest assets.json logical-id [--scene scene.json] [--game | --game-check | --smoke | --game-smoke | --scene3d-smoke | --prefab-smoke | --lod-smoke | --animation-smoke]".into()),
    };
    run_model_viewport_with_scene(&source, mode, scene.as_deref())
}
