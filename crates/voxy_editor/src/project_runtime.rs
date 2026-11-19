//! Project and legacy model entry points share the same game initialization/check.
use crate::{App, AudioOutputMode, ComponentRegistry, Duration, EventLoop, Instant, ViewportMode};
use std::path::Path;

/// Open a project in the editor, native game, or headless game-check mode.
/// # Errors
/// Reports invalid project/components, import, simulation and native runtime errors.
pub fn run_project(path: &Path, mode: ViewportMode) -> Result<(), Box<dyn std::error::Error>> {
    run_project_with_components(path, mode, crate::editor_component_registry()?)
}

/// Use the same application registry for project editing, Play and standalone.
/// Start with `editor_component_registry()` and install application components.
/// # Errors
/// Reports unsupported modes, invalid data, simulation and native runtime errors.
pub fn run_project_with_components(
    path: &Path,
    mode: ViewportMode,
    registry: ComponentRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    run_project_configured(path, mode, registry, None)
}

pub(super) fn run_project_configured(
    path: &Path,
    mode: ViewportMode,
    registry: ComponentRegistry,
    setup: Option<voxy_gameplay::UiActionSetup>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut host = crate::GameHost::new(registry);
    host.ui_actions = setup;
    run_project_host(path, mode, host)
}

/// Opens a project from a standalone editor binary that can also run packages.
///
/// # Errors
/// Reports an invalid mode, project, or native editor failure.
pub fn run_project_with_package_player(
    path: &Path,
    mode: ViewportMode,
    player: std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    if !matches!(mode, ViewportMode::Interactive | ViewportMode::PrefabSmoke) {
        return Err("package player requires editor mode".into());
    }
    run_project_host_with_player(
        path,
        mode,
        crate::GameHost::new(crate::editor_component_registry()?),
        Some(player),
        None,
    )
}

pub(super) fn run_project_host(
    path: &Path,
    mode: ViewportMode,
    host: crate::GameHost,
) -> Result<(), Box<dyn std::error::Error>> {
    run_project_host_with_player(path, mode, host, None, None)
}

pub(super) fn run_packaged_project_host(
    path: &Path,
    mode: ViewportMode,
    host: crate::GameHost,
    archive: std::sync::Arc<voxy_assets::ResourcePackage>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_project_host_with_player(path, mode, host, None, Some(archive))
}

fn run_project_host_with_player(
    path: &Path,
    mode: ViewportMode,
    host: crate::GameHost,
    player: Option<std::path::PathBuf>,
    model_package: Option<std::sync::Arc<voxy_assets::ResourcePackage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !matches!(
        mode,
        ViewportMode::Interactive
            | ViewportMode::PrefabSmoke
            | ViewportMode::Game
            | ViewportMode::GameCheck
            | ViewportMode::GameSmoke
    ) {
        return Err("project entry requires editor or game mode".into());
    }
    let mut app = match model_package {
        Some(archive) => App::from_packaged_project(path, host.components, archive)?,
        None => App::from_project_with_model_package(path, host.components, None)?,
    };
    app.build.player = player;
    app.ui_action_setup = host.ui_actions;
    app.behavior_setup = host.behaviors;
    app.behavior_checkpoints = host.behavior_checkpoints;
    app.ui_action_checkpoints = host.ui_action_checkpoints;
    if mode == ViewportMode::PrefabSmoke {
        app.start_prefab_smoke();
    }
    let result: Result<(), Box<dyn std::error::Error>> = (|| {
        if !prepare_game(&mut app, mode)? {
            EventLoop::new()?.run_app(&mut app)?;
        }
        if let Some(error) = app.error.take() {
            return Err(error.into());
        }
        Ok(())
    })();
    let cleanup = app.stop_workers();
    result?;
    cleanup
}

/// Returns true when the requested headless check has finished.
pub(super) fn prepare_game(
    app: &mut App,
    mode: ViewportMode,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !matches!(
        mode,
        ViewportMode::Game | ViewportMode::GameCheck | ViewportMode::GameSmoke
    ) {
        return Ok(false);
    }
    app.audio
        .set_output_mode(if mode == ViewportMode::GameCheck {
            AudioOutputMode::Offline
        } else {
            AudioOutputMode::Native
        });
    app.start_standalone()?;
    if mode == ViewportMode::GameSmoke {
        app.smoke_deadline = Some(Instant::now() + Duration::from_secs(30));
    }
    if mode == ViewportMode::GameCheck {
        app.check_game_ui()?;
        for _ in 0..120 {
            app.advance_game(1.0 / 60.0)?;
        }
        println!(
            "GAME STATE {}",
            serde_json::to_string(&app.authoring_document()?)?
        );
        if let Some(audio) = app.audio.play() {
            println!("GAME AUDIO frames={} peak={}", audio.frames, audio.peak);
        }
        println!(
            "GAME CHECK PASS ticks={} objects={} prefab_instances={}",
            app.play.simulation_ticks,
            app.object_ids.len(),
            app.authoring
                .authoring_source
                .as_ref()
                .map_or(0, |source| source.value().source.instances.len())
        );
        return Ok(true);
    }
    Ok(false)
}
