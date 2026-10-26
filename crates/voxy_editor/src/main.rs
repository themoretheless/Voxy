//! Standalone native authoring entry point.
use voxy_editor::{ModelSource, ViewportMode, run_model_viewport_with_scene};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments: Vec<_> = std::env::args().skip(1).collect();
    match arguments.as_slice() {
        [command, directory, name] if command == "--create-project" => {
            let path = voxy_editor::project::create(std::path::Path::new(directory), name)?;
            println!("Created {}", path.display());
            return Ok(());
        }
        [command, path, output] if command == "--build-project" => {
            return voxy_editor::export_project_package(
                std::path::Path::new(path),
                std::path::Path::new(output),
            );
        }
        [command, path, mode] if command == "--package" => {
            let mode = match mode.as_str() {
                "--game" => ViewportMode::Game,
                "--game-check" => ViewportMode::GameCheck,
                "--game-native-smoke" => ViewportMode::GameSmoke,
                _ => return Err("package requires a game mode".into()),
            };
            return voxy_editor::run_packaged_game(std::path::Path::new(path), mode);
        }
        [command, path] if command == "--project" => {
            return voxy_editor::run_project_editor(std::path::Path::new(path));
        }
        [command, path, mode] if command == "--project" => {
            let mode = match mode.as_str() {
                "--game" => ViewportMode::Game,
                "--game-check" => ViewportMode::GameCheck,
                "--game-native-smoke" => ViewportMode::GameSmoke,
                _ => return Err("unknown project mode".into()),
            };
            return voxy_editor::run_project(std::path::Path::new(path), mode);
        }
        [command, path] if command == "--validate-project" => {
            let registry = voxy_editor::editor_component_registry()?;
            let project = voxy_editor::project::open(std::path::Path::new(path), &registry)?;
            println!(
                "Validated project {:?}: {} scene objects",
                project.descriptor.name,
                project.scene.objects.len()
            );
            return Ok(());
        }
        _ => {}
    }
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
        _ => return Err("usage: voxy_editor --create-project NEW_DIRECTORY NAME | --build-project project.voxy game.vpak | --package game.vpak --game|--game-check | --validate-project project.voxy | --project project.voxy [--game | --game-check | --game-native-smoke] | model.obj|model.gltf|model.glb | --manifest assets.json logical-id [--scene scene.json] [--game | --game-check | --smoke | --game-smoke | --scene3d-smoke | --prefab-smoke | --lod-smoke | --animation-smoke]".into()),
    };
    run_model_viewport_with_scene(&source, mode, scene.as_deref())
}
