//! Open-source animated model and controller qualification through public project APIs.
use std::path::Path;
use voxy_editor::{GameHost, HeadlessGame, ViewportMode};
use winit::keyboard::KeyCode;
fn host() -> Result<GameHost, Box<dyn std::error::Error>> {
    Ok(GameHost::new(voxy_editor::editor_component_registry()?))
}
fn asset_info(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    let model =
        voxy_model::ModelAsset::parse_animated(&bytes, &[], voxy_model::ModelLimits::default())?;
    let idle = model
        .animations
        .iter()
        .position(|clip| clip.name() == "Idle")
        .ok_or("Idle clip missing")?;
    let pose = model.sample_pose_phase64(Some(idle), 0.)?;
    let frame = model.sample_frame_phase(Some(idle), 0.)?;
    let surfaces = model.scene_surfaces64_with_weights(&pose, &model.frame_weights(&frame)?)?;
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    for surface in surfaces {
        for vertex in surface.positions {
            for axis in 0..3 {
                low[axis] = low[axis].min(vertex[axis]);
                high[axis] = high[axis].max(vertex[axis]);
            }
        }
    }
    println!(
        "ASSET INFO {}",
        serde_json::json!({"joints":model.skeleton.joints().len(),"mesh_instances":model.mesh_instance_count(),"primitives":model.primitives.len(),"morph_primitives":model.morph_primitive_count(),"clips":model.animations.iter().map(|clip|clip.name()).collect::<Vec<_>>(),"idle_min":low,"idle_max":high})
    );
    Ok(())
}
fn replay(mut game: HeadlessGame) -> Result<(), Box<dyn std::error::Error>> {
    let scene = game.snapshot()?;
    let owner = scene
        .objects
        .iter()
        .find(|object| object.components.contains_key("editor.model-locomotion.v1"))
        .ok_or("locomotion owner missing")?
        .id
        .clone();
    let mut transitions = Vec::new();
    let mut prior = String::new();
    let mut pose_hashes = std::collections::BTreeSet::new();
    for tick in 0..240 {
        if tick == 30 {
            game.key(KeyCode::ArrowRight, true)?;
        }
        if tick == 60 {
            game.key(KeyCode::Space, true)?;
        }
        if tick == 61 {
            game.key(KeyCode::Space, false)?;
        }
        if tick == 150 {
            game.key(KeyCode::ArrowRight, false)?;
        }
        game.advance(1. / 60.)?;
        let frame = game
            .animation_snapshot(&owner)?
            .ok_or("accepted animation frame missing")?;
        if frame
            .skin_matrices
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err("nonfinite accepted skin palette".into());
        }
        pose_hashes.insert(
            blake3::hash(&serde_json::to_vec(&frame.skin_matrices)?)
                .to_hex()
                .to_string(),
        );
        if frame.clip_name != prior {
            transitions.push((tick, frame.clip_name.clone()));
            prior = frame.clip_name;
        }
    }
    for expected in ["Idle", "Walking", "Jump"] {
        if !transitions.iter().any(|(_, name)| name == expected) {
            return Err(format!("missing locomotion clip {expected}: {transitions:?}").into());
        }
    }
    if pose_hashes.len() < 30 {
        return Err("skeletal pose did not animate".into());
    }
    println!(
        "LOCOMOTION RESULT {}",
        serde_json::json!({"scene":game.snapshot()?,"camera":game.presentation_camera(),"ticks":game.fixed_ticks(),"transitions":transitions,"pose_hashes":pose_hashes,"animation":game.animation_snapshot(&owner)?})
    );
    game.stop()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [mode,path] if mode=="--asset-info"=>asset_info(Path::new(path)),
        [mode,path] if mode=="--check"=>replay(HeadlessGame::open_project(Path::new(path),host()?)?),
        [mode,path] if mode=="--package-check"=>replay(HeadlessGame::open_package(Path::new(path),host()?)?),
        [mode,path,out] if mode=="--build"=>voxy_editor::export_project_package_with_host(Path::new(path),Path::new(out),host()?),
        [mode,path] if mode=="--editor"||mode=="--game"=>voxy_editor::run_project_with_host(Path::new(path),if mode=="--editor"{ViewportMode::Interactive}else{ViewportMode::Game},host()?),
        [mode,path] if mode=="--package"=>voxy_editor::run_packaged_game_with_host(Path::new(path),ViewportMode::Game,host()?),
        _=>Err("usage: locomotion_project --asset-info model.glb | --check|--editor|--game project.voxy | --build project.voxy game.vpak | --package-check|--package game.vpak".into()),
    }
}
