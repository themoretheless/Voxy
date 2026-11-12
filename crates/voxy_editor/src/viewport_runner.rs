//! Native viewport launch entrypoints and post-window smoke verification.

use glam::Vec2;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use voxy_assets::{AssetId, SourcePath};
use voxy_scene::ComponentRegistry;
use winit::{event_loop::EventLoop, keyboard::KeyCode};

use crate::{
    App, InputRecipe, ModelSource, ViewportMode, animation_smoke, gameplay_smoke, lod_smoke,
    project_runtime, run_project, scene3d_smoke,
};

pub(crate) fn configured_app(
    source: &ModelSource,
    scene_path: Option<&std::path::Path>,
    smoke: bool,
) -> Result<App, Box<dyn std::error::Error>> {
    let mut app = match source {
        ModelSource::File(path) => App::new(path, smoke)?,
        ModelSource::Manifest { path, asset } => App::from_manifest(path, asset.clone(), smoke)?,
    };
    if let Some(path) = scene_path
        && let Err(error) = app.configure_scene(path)
    {
        app.stop_workers()?;
        return Err(error);
    }
    Ok(app)
}

pub(crate) fn configured_app_with_registry(
    source: &ModelSource,
    scene_path: Option<&std::path::Path>,
    smoke: bool,
    registry: ComponentRegistry,
) -> Result<App, Box<dyn std::error::Error>> {
    let (path, asset) = match source {
        ModelSource::File(path) => (path.canonicalize()?, None),
        ModelSource::Manifest { path, asset } => (path.canonicalize()?, Some(asset.clone())),
    };
    let root = path.parent().ok_or("missing source root")?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("source name must be UTF-8")?;
    let source_path = SourcePath::new(name)?;
    let (id, recipe) = match asset {
        Some(id) => (id, InputRecipe::Manifest(source_path.observation_id())),
        None => (AssetId(name.into()), InputRecipe::Direct),
    };
    let mut app = App::create_with_registry(root, id, recipe, smoke, Arc::new(registry))?;
    if let Some(path) = scene_path
        && let Err(error) = app.configure_scene(path)
    {
        app.stop_workers()?;
        return Err(error);
    }
    Ok(app)
}

/// Open a project in the existing native authoring owner without a bootstrap model.
/// # Errors
/// Reports invalid project/scene data, resource setup and native window/GPU failures.
pub fn run_project_editor(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    run_project(path, ViewportMode::Interactive)
}

/// Runs with application codecs installed before scene loading or worker setup.
/// Start from `editor_component_registry()` to retain the built-in components.
/// The same immutable registry is used for editing, Play, saving and prefab workers.
/// # Errors
/// Reports invalid scenes, component data, imports and native runtime failures.
pub fn run_model_viewport_with_components(
    source: &ModelSource,
    mode: ViewportMode,
    scene_path: Option<&std::path::Path>,
    registry: ComponentRegistry,
    ui_actions: Option<voxy_gameplay::UiActionSetup>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_model_viewport_configured_registry(source, mode, scene_path, ui_actions, Some(registry))
}

/// Runs the native model viewport with asynchronous imports and owner publication.
/// # Errors
/// Reports configuration, window/GPU/worker failures and smoke acceptance timeout.
pub fn run_model_viewport(
    source: &ModelSource,
    mode: ViewportMode,
) -> Result<(), Box<dyn std::error::Error>> {
    run_model_viewport_with_scene(source, mode, None)
}

/// Runs the viewport with an optional authoring file; existing files load before the window opens.
/// # Errors
/// Reports invalid scene/resource references, IO and native viewport failures.
pub fn run_model_viewport_with_scene(
    source: &ModelSource,
    mode: ViewportMode,
    scene_path: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_model_viewport_configured(source, mode, scene_path, None)
}

/// Runs the ordinary editor or standalone mode with application UI callbacks.
/// A fresh registry is configured on every Play; failed setup does not start Play.
/// # Errors
/// Reports callback registration, scene, import and native runtime failures.
pub fn run_model_viewport_with_ui_actions(
    source: &ModelSource,
    mode: ViewportMode,
    scene_path: Option<&std::path::Path>,
    setup: voxy_gameplay::UiActionSetup,
) -> Result<(), Box<dyn std::error::Error>> {
    run_model_viewport_configured(source, mode, scene_path, Some(setup))
}

fn run_model_viewport_configured(
    source: &ModelSource,
    mode: ViewportMode,
    scene_path: Option<&std::path::Path>,
    setup: Option<voxy_gameplay::UiActionSetup>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_model_viewport_configured_registry(source, mode, scene_path, setup, None)
}

#[expect(
    clippy::too_many_lines,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
pub(crate) fn run_model_viewport_configured_registry(
    source: &ModelSource,
    mode: ViewportMode,
    scene_path: Option<&std::path::Path>,
    setup: Option<voxy_gameplay::UiActionSetup>,
    registry: Option<ComponentRegistry>,
) -> Result<(), Box<dyn std::error::Error>> {
    let smoke = mode == ViewportMode::Smoke;
    let mut app = if let Some(registry) = registry {
        configured_app_with_registry(source, scene_path, smoke, registry)?
    } else {
        configured_app(source, scene_path, smoke)?
    };
    app.ui_action_setup = setup;
    if matches!(
        mode,
        ViewportMode::Game | ViewportMode::GameCheck | ViewportMode::GameSmoke
    ) && scene_path.is_none_or(|path| !path.is_file())
    {
        app.stop_workers()?;
        return Err("standalone game requires an existing --scene file".into());
    }
    match project_runtime::prepare_game(&mut app, mode) {
        Ok(true) => {
            app.stop_workers()?;
            return Ok(());
        }
        Ok(false) => {}
        Err(error) => {
            app.stop_workers()?;
            return Err(error);
        }
    }
    if mode == ViewportMode::AnimationSmoke {
        let mut smoke = animation_smoke::Smoke::default();
        smoke.profile = std::env::var_os("VOXY_ANIMATION_PROFILE").is_some();
        smoke.foot_contact = std::env::var_os("VOXY_FOOT_CONTACT_SMOKE").is_some();
        smoke.foot_reload = std::env::var_os("VOXY_FOOT_RELOAD_SMOKE").is_some();
        smoke.composed_root = std::env::var_os("VOXY_COMPOSED_ROOT_SMOKE").is_some();
        smoke.root_rotation =
            smoke.composed_root || std::env::var_os("VOXY_ROOT_ROTATION_SMOKE").is_some();
        smoke.root_motion =
            smoke.root_rotation || std::env::var_os("VOXY_ROOT_MOTION_SMOKE").is_some();
        smoke.oriented_body = smoke.root_motion
            && !smoke.root_rotation
            && std::env::var_os("VOXY_ORIENTED_CHARACTER_SMOKE").is_some();
        app.animation_smoke = Some(smoke);
        app.smoke_deadline = Some(Instant::now() + Duration::from_secs(30));
    }
    if mode == ViewportMode::PrefabSmoke {
        app.start_prefab_smoke();
    }
    if mode == ViewportMode::LodSmoke {
        app.lod_smoke = Some(lod_smoke::Smoke::default());
        app.smoke_deadline = Some(Instant::now() + Duration::from_secs(30));
        app.camera.legacy = false;
        app.camera.distance = 1000.;
    }
    if mode == ViewportMode::Scene3DSmoke {
        app.scene3d_smoke = Some(scene3d_smoke::Smoke::default());
        app.smoke_deadline = Some(Instant::now() + Duration::from_secs(15));
    }
    if mode == ViewportMode::GameplaySmoke {
        app.gameplay_smoke = Some(gameplay_smoke::Smoke::new());
        app.smoke_deadline = Some(Instant::now() + Duration::from_secs(15));
    }
    if smoke {
        app.edit_key(KeyCode::KeyD)?;
        app.edit_key(KeyCode::ArrowRight)?;
        app.edit_key(KeyCode::Tab)?;
        app.edit_key(KeyCode::KeyZ)?;
        app.edit_key(KeyCode::KeyY)?;
    }
    let instance_state: Vec<_> = app
        .instances
        .iter()
        .map(|node| app.scene.world_matrix(*node))
        .collect::<Result<_, _>>()?;
    let event_result = (|| {
        EventLoop::new()?.run_app(&mut app)?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })();
    let cleanup = app.stop_workers();
    event_result?;
    cleanup?;
    if let Some(error) = app.error.take() {
        return Err(error.into());
    }
    if smoke {
        if !app.recovered {
            return Err("smoke closed before corrupt-file recovery".into());
        }
        let before_drag = app.authoring_document()?;
        app.begin_drag(Vec2::splat(100.0), Vec2::new(640.0, 480.0))?;
        app.preview_drag(Vec2::new(140.0, 120.0))?;
        app.finish_drag(true)?;
        app.edit_key(KeyCode::KeyZ)?;
        app.begin_drag(Vec2::splat(100.0), Vec2::new(640.0, 480.0))?;
        app.preview_drag(Vec2::new(150.0, 150.0))?;
        app.finish_drag(false)?;
        app.edit_key(KeyCode::KeyY)?;
        app.edit_key(KeyCode::KeyZ)?;
        if app.authoring_document()? != before_drag {
            return Err("smoke drag undo/cancel changed scene".into());
        }
        println!("MODEL DRAG PASS: one transaction, cancellation preserved redo");
        let authoring_before_delete = app.authoring_document()?;
        let count = app.instances.len();
        for _ in 0..count {
            app.edit_key(KeyCode::Delete)?;
        }
        if !app.instances.is_empty() {
            return Err("smoke deletion left instances".into());
        }
        for _ in 0..count {
            app.edit_key(KeyCode::KeyZ)?;
        }
        if app.authoring_document()? != authoring_before_delete {
            return Err("smoke deletion undo changed authoring data".into());
        }
        for _ in 0..count {
            app.edit_key(KeyCode::KeyY)?;
        }
        if !app.instances.is_empty() {
            return Err("smoke deletion redo left instances".into());
        }
        for _ in 0..count {
            app.edit_key(KeyCode::KeyZ)?;
        }
        println!("MODEL DELETE PASS: empty scene and undo/redo restored object identities");
        let resource = app
            .catalog
            .snapshot(app.id.as_ref().expect("model-only fixture"))
            .ok_or("missing recovered resource")?;
        app.edit_key(KeyCode::KeyZ)?;
        app.edit_key(KeyCode::KeyY)?;
        if !app.pick_model(Vec2::new(352.0, 240.0), Vec2::new(640.0, 480.0))? || app.selected != 0 {
            return Err("smoke could not pick recovered model after undo".into());
        }
        let after_undo = app
            .catalog
            .snapshot(app.id.as_ref().expect("model-only fixture"))
            .ok_or("undo removed resource")?;
        if !Arc::ptr_eq(&resource, &after_undo) {
            return Err("undo replaced resource revision".into());
        }
        for (node, expected) in app.instances.iter().zip(instance_state) {
            if app.scene.world_matrix(*node)? != expected {
                return Err("smoke reload changed instance transform".into());
            }
        }
        println!(
            "MODEL INSTANCES PASS: {} independent transforms, shared geometry",
            app.instances.len()
        );
        app.verify_scene_round_trip()?;
        println!(
            "ASSET WINDOW PASS: {} native frames; failed reload retained model; corrected file recovered",
            app.frames
        );
    }
    Ok(())
}
