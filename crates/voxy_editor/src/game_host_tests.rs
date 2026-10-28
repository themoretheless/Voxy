use crate::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

type Log = Arc<Mutex<Vec<(&'static str, f32)>>>;
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
struct MoveConfig {
    speed: f32,
}
#[derive(Debug)]
struct MoveBehavior {
    speed: f32,
    log: Log,
}
impl voxy_scene::Behavior for MoveBehavior {
    fn awake(&mut self, scene: &mut SceneGraph, owner: NodeId) {
        let mut local = scene.local(owner).unwrap();
        local.translation.x += 0.125;
        scene.set_local(owner, local).unwrap();
        self.log
            .lock()
            .unwrap()
            .push(("awake", local.translation.x));
    }
    fn start(&mut self, _: &mut SceneGraph, _: NodeId) {
        self.log.lock().unwrap().push(("start", 0.));
    }
    fn fixed_update(&mut self, scene: &mut SceneGraph, owner: NodeId, dt: f64) {
        let mut local = scene.local(owner).unwrap();
        local.translation.x += self.speed * dt as f32;
        scene.set_local(owner, local).unwrap();
        self.log
            .lock()
            .unwrap()
            .push(("fixed", local.translation.x));
    }
    fn on_destroy(&mut self, scene: &mut SceneGraph, owner: NodeId) {
        self.log
            .lock()
            .unwrap()
            .push(("destroy", scene.local(owner).unwrap().translation.x));
    }
}
fn host(log: Log, fail: Arc<AtomicBool>) -> Result<GameHost, Box<dyn std::error::Error>> {
    let mut components = editor_component_registry()?;
    components.register::<MoveConfig>("test.move.v1")?;
    let mut host = GameHost::new(components);
    let ui_log = log.clone();
    host.ui_actions = Some(voxy_gameplay::UiActionSetup::new(move |_| {
        ui_log.lock().unwrap().push(("ui-setup", 0.));
        Ok(())
    }));
    host.behaviors = Some(BehaviorSetup::new(move |scene, simulation| {
        let owners: Vec<_> = scene
            .components::<MoveConfig>()
            .map(|(id, value)| (id, *value))
            .collect();
        for (owner, config) in owners {
            simulation.attach(
                scene,
                owner,
                MoveBehavior {
                    speed: config.speed,
                    log: log.clone(),
                },
            )?;
        }
        if fail.load(Ordering::SeqCst) {
            return Err("host setup rejected".into());
        }
        Ok(())
    }));
    Ok(host)
}
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let index = NEXT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("voxy-host-{}-{nonce}-{index}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn app(path: &std::path::Path, host: GameHost) -> Result<App, Box<dyn std::error::Error>> {
    let mut app = App::from_project_with_registry(path, host.components)?;
    app.behavior_setup = host.behaviors;
    app.behavior_checkpoints = host.behavior_checkpoints;
    app.ui_action_checkpoints = host.ui_action_checkpoints;
    app.ui_action_setup = host.ui_actions;
    Ok(app)
}
#[test]
fn host_behaviors_run_identically_in_repeated_play_standalone_build_and_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Host game")?;
    let log = Log::default();
    let fail = Arc::new(AtomicBool::new(false));
    let mut editor = app(&path, host(log.clone(), fail.clone())?)?;
    editor.edit_key(KeyCode::KeyD)?;
    editor
        .scene
        .insert_component(editor.instances[0], MoveConfig { speed: 1. })?;
    editor.commit_authoring()?;
    editor.save_authoring()?;
    let authored = editor.authoring_document()?;
    for _ in 0..2 {
        editor.toggle_play()?;
        for _ in 0..120 {
            editor.advance_game(1.0 / 60.0)?;
        }
        assert!((editor.scene.local(editor.instances[0])?.translation.x - 2.125).abs() < 1e-4);
        editor.toggle_play()?;
        assert_eq!(editor.authoring_document()?, authored);
    }
    editor.stop_workers()?;
    run_project_with_host(
        &path,
        ViewportMode::GameCheck,
        host(log.clone(), fail.clone())?,
    )?;
    let package = temp.0.join("host.vpak");
    export_project_package_with_host(&path, &package, host(log.clone(), fail.clone())?)?;
    std::fs::remove_dir_all(path.parent().unwrap())?;
    run_packaged_game_with_host(&package, ViewportMode::GameCheck, host(log.clone(), fail)?)?;
    let events = log.lock().unwrap();
    for name in ["awake", "destroy", "ui-setup"] {
        assert_eq!(
            events.iter().filter(|(event, _)| *event == name).count(),
            5,
            "{name}"
        );
    }
    assert_eq!(
        events.iter().filter(|(event, _)| *event == "start").count(),
        4
    );
    assert_eq!(
        events.iter().filter(|(event, _)| *event == "fixed").count(),
        480
    );
    let final_positions: Vec<_> = events
        .iter()
        .filter(|(event, _)| *event == "destroy")
        .map(|(_, x)| *x)
        .collect();
    for index in [0, 1, 2, 4] {
        assert!((final_positions[index] - 2.125).abs() < 1e-4);
    }
    assert_eq!(final_positions[3], 0.125); // Build validates setup but never advances game time.
    Ok(())
}
#[test]
fn rejected_host_setup_destroys_partial_behaviors_restores_authoring_and_allows_retry()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Retry")?;
    let log = Log::default();
    let fail = Arc::new(AtomicBool::new(true));
    let mut editor = app(&path, host(log.clone(), fail.clone())?)?;
    editor.edit_key(KeyCode::KeyD)?;
    editor
        .scene
        .insert_component(editor.instances[0], MoveConfig { speed: 1. })?;
    editor.commit_authoring()?;
    let authored = editor.authoring_document()?;
    assert!(
        editor
            .toggle_play()
            .unwrap_err()
            .to_string()
            .contains("host setup rejected")
    );
    assert_eq!(editor.authoring_document()?, authored);
    assert!(editor.play.playing.is_none() && editor.play.simulation.is_none());
    assert!(editor.play.ui_actions.is_none() && editor.play.angular_motion.is_none());
    assert_eq!(
        log.lock()
            .unwrap()
            .iter()
            .filter(|(name, _)| *name == "destroy")
            .count(),
        1
    );
    fail.store(false, Ordering::SeqCst);
    editor.toggle_play()?;
    editor.advance_game(1.0 / 60.0)?;
    editor.toggle_play()?;
    assert_eq!(editor.authoring_document()?, authored);
    editor.stop_workers()?;
    assert_eq!(
        log.lock()
            .unwrap()
            .iter()
            .filter(|(name, _)| *name == "destroy")
            .count(),
        2
    );
    Ok(())
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SavedCounter {
    awake: u32,
    start: u32,
    ticks: u64,
}
impl voxy_scene::Behavior for SavedCounter {
    fn checkpoint(&self) -> Result<voxy_scene::BehaviorState, voxy_scene::BehaviorCheckpointError> {
        voxy_scene::BehaviorState::json("host.saved-counter.v1", self)
    }
    fn awake(&mut self, _: &mut SceneGraph, _: NodeId) {
        self.awake += 1;
    }
    fn start(&mut self, _: &mut SceneGraph, _: NodeId) {
        self.start += 1;
    }
    fn fixed_update(&mut self, scene: &mut SceneGraph, owner: NodeId, dt: f64) {
        self.ticks += 1;
        let mut local = scene.local(owner).unwrap();
        local.translation.x += dt as f32;
        scene.set_local(owner, local).unwrap();
    }
}
fn checkpoint_host(attach: bool) -> Result<GameHost, Box<dyn std::error::Error>> {
    let mut registry = voxy_scene::BehaviorCheckpointRegistry::default();
    registry.register::<SavedCounter>("host.saved-counter.v1")?;
    let mut host = GameHost::new(editor_component_registry()?);
    host.behavior_checkpoints = Arc::new(registry);
    if attach {
        host.behaviors = Some(BehaviorSetup::new(|scene, simulation| {
            let owners: Vec<_> = scene.nodes().map(|(owner, _, _)| owner).collect();
            for owner in owners {
                simulation.attach(
                    scene,
                    owner,
                    SavedCounter {
                        awake: 0,
                        start: 0,
                        ticks: 0,
                    },
                )?;
            }
            Ok(())
        }));
    }
    Ok(host)
}
#[test]
fn host_checkpoint_codecs_survive_project_build_and_source_free_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Checkpoint host")?;
    let mut editor = App::from_project(&path)?;
    editor.edit_key(KeyCode::KeyD)?;
    editor.commit_authoring()?;
    editor.save_authoring()?;
    editor.stop_workers()?;
    let source_host = checkpoint_host(true)?;
    let shared = source_host.behavior_checkpoints.clone();
    let mut configured = app(&path, source_host)?;
    assert!(Arc::ptr_eq(&configured.behavior_checkpoints, &shared));
    configured.stop_workers()?;
    let mut source = HeadlessGame::open_project(&path, checkpoint_host(true)?)?;
    let initial = source.snapshot()?;
    let saved = source.behavior_checkpoint(65536)?;
    let text = saved.to_json(128, 65536)?;
    let saved = voxy_scene::BehaviorCheckpoint::from_json(&text, 128, 65536)?;
    assert!(source.restore_behavior_checkpoint(&saved, 65536).is_err());
    assert_eq!(source.behavior_checkpoint(65536)?, saved);
    let mut trace = Vec::new();
    for _ in 0..120 {
        source.advance(1. / 60.)?;
        trace.push((source.snapshot()?, source.behavior_checkpoint(65536)?));
    }
    source.stop()?;
    let package = temp.0.join("checkpoint-host.vpak");
    export_project_package_with_host(&path, &package, checkpoint_host(true)?)?;
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut missing =
        HeadlessGame::open_package(&package, GameHost::new(editor_component_registry()?))?;
    let before = missing.behavior_checkpoint(65536)?;
    assert!(missing.restore_behavior_checkpoint(&saved, 65536).is_err());
    assert_eq!(missing.behavior_checkpoint(65536)?, before);
    missing.stop()?;
    let mut restored = HeadlessGame::open_package(&package, checkpoint_host(false)?)?;
    assert_eq!(restored.snapshot()?, initial);
    restored.restore_behavior_checkpoint(&saved, 65536)?;
    assert_eq!(restored.behavior_checkpoint(65536)?, saved);
    for (scene, behavior) in trace {
        restored.advance(1. / 60.)?;
        assert_eq!(restored.snapshot()?, scene);
        assert_eq!(restored.behavior_checkpoint(65536)?, behavior);
    }
    restored.stop()
}

struct ClickCounter(u64);
impl voxy_gameplay::UiActionHandler for ClickCounter {
    fn dispatch(
        &mut self,
        _: &SceneGraph,
        _: &voxy_gameplay::UiActionEvent,
        _: &mut voxy_scene::SceneCommands,
    ) -> Result<(), voxy_gameplay::GameplayError> {
        self.0 += 1;
        Ok(())
    }
    fn checkpoint(
        &self,
        _: &dyn Fn(NodeId) -> Option<voxy_scene::ObjectId>,
    ) -> Result<voxy_gameplay::UiActionState, voxy_gameplay::GameplayError> {
        voxy_gameplay::UiActionState::new(
            "test.click-counter.v1".into(),
            self.0.to_le_bytes().to_vec(),
        )
    }
}
fn ui_checkpoint_host(with_codec: bool) -> Result<GameHost, Box<dyn std::error::Error>> {
    let mut host = GameHost::new(editor_component_registry()?);
    host.ui_actions = Some(voxy_gameplay::UiActionSetup::new(|actions| {
        actions.register_checkpointable("count".into(), ClickCounter(0))
    }));
    if with_codec {
        let registry = Arc::get_mut(&mut host.ui_action_checkpoints).unwrap();
        registry.register("test.click-counter.v1", |bytes, _| {
            let words: [u8; 8] = bytes
                .try_into()
                .map_err(|_| voxy_gameplay::GameplayError::from("bad counter length"))?;
            Ok(Box::new(ClickCounter(u64::from_le_bytes(words))))
        })?;
    }
    Ok(host)
}
#[test]
fn host_ui_callback_state_and_pending_click_survive_source_free_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "UI checkpoint host")?;
    let mut editor = App::from_project(&path)?;
    editor.edit_key(KeyCode::KeyD)?;
    editor.scene.insert_component(
        editor.instances[0],
        voxy_gameplay::UiElement {
            origin: [0.; 2],
            size: [1.; 2],
            color: [1.; 4],
            layer: 0,
            enabled: true,
            action: Some("count".into()),
            text: None,
        },
    )?;
    editor.commit_authoring()?;
    editor.save_authoring()?;
    editor.stop_workers()?;
    let shared_host = ui_checkpoint_host(true)?;
    let shared = shared_host.ui_action_checkpoints.clone();
    let mut configured = app(&path, shared_host)?;
    assert!(Arc::ptr_eq(&configured.ui_action_checkpoints, &shared));
    configured.stop_workers()?;
    let limits = voxy_gameplay::UiActionCheckpointLimits {
        max_events: 128,
        max_handlers: 128,
        max_commands_per_event: 128,
        max_bytes: 100000,
    };
    let click = |game: &mut HeadlessGame| -> Result<(), Box<dyn std::error::Error>> {
        game.pointer([0.5, 0.5], [100., 100.], true)?;
        game.pointer([0.5, 0.5], [100., 100.], false)
    };
    let mut source = HeadlessGame::open_project(&path, ui_checkpoint_host(true)?)?;
    for _ in 0..23 {
        click(&mut source)?;
        source.advance(1. / 60.)?;
    }
    click(&mut source)?;
    let saved = source
        .ui_action_checkpoint(limits)?
        .ok_or("no UI actions")?;
    let json = saved.to_json(limits)?;
    let saved = voxy_gameplay::UiActionCheckpoint::from_json(&json, limits)?;
    // The file includes private callback state plus an accepted pending activation.
    let document: serde_json::Value = serde_json::from_str(&json)?;
    assert_eq!(document["events"].as_array().unwrap().len(), 1);
    assert_eq!(document["handlers"][0][1]["payload"][0], 23);
    let package = temp.0.join("ui-checkpoint.vpak");
    export_project_package_with_host(&path, &package, ui_checkpoint_host(true)?)?;
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut missing = HeadlessGame::open_package(&package, ui_checkpoint_host(false)?)?;
    let before = missing.ui_action_checkpoint(limits)?;
    assert!(
        missing
            .restore_ui_action_checkpoint(&saved, limits)
            .is_err()
    );
    assert_eq!(missing.ui_action_checkpoint(limits)?, before);
    missing.stop()?;
    let mut resumed = HeadlessGame::open_package(&package, ui_checkpoint_host(true)?)?;
    resumed.restore_ui_action_checkpoint(&saved, limits)?;
    assert_eq!(resumed.ui_action_checkpoint(limits)?, Some(saved));
    for _ in 0..120 {
        click(&mut source)?;
        click(&mut resumed)?;
        source.advance(1. / 60.)?;
        resumed.advance(1. / 60.)?;
        assert_eq!(
            source.ui_action_checkpoint(limits)?,
            resumed.ui_action_checkpoint(limits)?
        );
        assert_eq!(source.snapshot()?, resumed.snapshot()?);
    }
    source.stop()?;
    resumed.stop()
}

#[test]
fn late_start_failure_destroys_behaviors_and_restores_editor_for_retry()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Late failure")?;
    let log = Log::default();
    let reject = Arc::new(AtomicBool::new(true));
    let mut game_host = GameHost::new(editor_component_registry()?);
    let events = log.clone();
    let fail = reject.clone();
    game_host.behaviors = Some(BehaviorSetup::new(move |scene, simulation| {
        let owner = scene.nodes().next().unwrap().0;
        simulation.attach(
            scene,
            owner,
            MoveBehavior {
                speed: 1.,
                log: events.clone(),
            },
        )?;
        if fail.load(Ordering::SeqCst) {
            scene.insert_component(
                owner,
                voxy_gameplay::AngularMotion {
                    axis: [0.; 3],
                    radians_per_second: 1.,
                },
            )?;
        }
        Ok(())
    }));
    let mut editor = app(&path, game_host)?;
    editor.edit_key(KeyCode::KeyD)?;
    let authored = editor.authoring_document()?;
    assert!(editor.toggle_play().is_err());
    assert_eq!(editor.authoring_document()?, authored);
    assert!(editor.play.playing.is_none());
    assert!(editor.play.simulation.is_none());
    assert!(editor.play.ui_actions.is_none());
    assert_eq!(
        log.lock()
            .unwrap()
            .iter()
            .filter(|(name, _)| *name == "destroy")
            .count(),
        1
    );
    reject.store(false, Ordering::SeqCst);
    editor.toggle_play()?;
    editor.advance_game(1. / 60.)?;
    editor.toggle_play()?;
    assert_eq!(editor.authoring_document()?, authored);
    assert_eq!(editor.play.simulation_ticks, 0);
    editor.stop_workers()?;
    assert_eq!(
        log.lock()
            .unwrap()
            .iter()
            .filter(|(name, _)| *name == "destroy")
            .count(),
        2
    );
    Ok(())
}

#[test]
fn standalone_start_is_explicit_and_does_not_toggle_an_active_session()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Standalone")?;
    let mut game = App::from_project(&path)?;
    game.start_standalone()?;
    game.advance_game(1. / 60.)?;
    assert!(game.start_standalone().is_err());
    assert!(game.play.playing.is_some());
    assert_eq!(game.play.simulation_ticks, 1);
    game.stop_workers()?;
    Ok(())
}

#[test]
fn ui_setup_failure_can_retry_and_failed_standalone_restores_mode()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "UI retry")?;
    let fail = Arc::new(AtomicBool::new(true));
    let reject = fail.clone();
    let mut config = GameHost::new(editor_component_registry()?);
    config.ui_actions = Some(voxy_gameplay::UiActionSetup::new(move |_| {
        if reject.load(Ordering::SeqCst) {
            return Err("UI setup rejected".into());
        }
        Ok(())
    }));
    let mut editor = app(&path, config)?;
    editor.edit_key(KeyCode::KeyD)?;
    let authored = editor.authoring_document()?;
    assert!(editor.start_standalone().is_err());
    assert!(!editor.standalone);
    assert_eq!(editor.authoring_document()?, authored);
    assert!(editor.play.playing.is_none() && editor.play.ui_actions.is_none());
    assert!(editor.play.simulation.is_none() && editor.play.physics.is_none());
    assert!(!editor.audio.is_pending() && editor.audio.play().is_none());
    fail.store(false, Ordering::SeqCst);
    editor.toggle_play()?;
    editor.advance_game(1. / 60.)?;
    editor.toggle_play()?;
    assert_eq!(editor.authoring_document()?, authored);
    assert_eq!(
        editor.authoring.history.as_ref().unwrap().current(),
        &authored
    );
    editor.stop_workers()?;
    Ok(())
}

#[test]
fn variable_frame_lifecycle_matches_editor_standalone_and_source_free_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Frame lifecycle")?;
    let editor_log = Log::default();
    let mut editor = app(
        &path,
        host(editor_log.clone(), Arc::new(AtomicBool::new(false)))?,
    )?;
    editor.edit_key(KeyCode::KeyD)?;
    editor
        .scene
        .insert_component(editor.instances[0], MoveConfig { speed: 1. })?;
    editor.commit_authoring()?;
    editor.save_authoring()?;
    let authored = editor.authoring_document()?;
    let stale_authoring = editor.instances[0];
    editor.toggle_play()?;
    assert!(editor.scene.local(stale_authoring).is_err());
    let standalone_log = Log::default();
    let mut standalone = HeadlessGame::open_project(
        &path,
        host(standalone_log.clone(), Arc::new(AtomicBool::new(false)))?,
    )?;
    let output = temp.0.join("game.vpak");
    export_project_package_with_host(
        &path,
        &output,
        host(Log::default(), Arc::new(AtomicBool::new(false)))?,
    )?;
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let packaged_log = Log::default();
    let mut packaged = HeadlessGame::open_package(
        &output,
        host(packaged_log.clone(), Arc::new(AtomicBool::new(false)))?,
    )?;
    for elapsed in [
        0.,
        1. / 240.,
        1. / 240.,
        1. / 120.,
        0.25,
        0.,
        1. / 60.,
        1. / 90.,
    ] {
        editor.advance_game(elapsed)?;
        standalone.advance(elapsed)?;
        packaged.advance(elapsed)?;
        assert_eq!(standalone.snapshot()?, editor.authoring_document()?);
        assert_eq!(packaged.snapshot()?, standalone.snapshot()?);
        assert_eq!(editor.play.simulation_ticks, standalone.fixed_ticks());
        assert_eq!(packaged.fixed_ticks(), standalone.fixed_ticks());
    }
    let before = editor.authoring_document()?;
    let ticks = editor.play.simulation_ticks;
    for elapsed in [-1., f64::NAN, f64::INFINITY] {
        assert!(editor.advance_game(elapsed).is_err());
        assert!(standalone.advance(elapsed).is_err());
        assert!(packaged.advance(elapsed).is_err());
    }
    assert_eq!(editor.authoring_document()?, before);
    assert_eq!(editor.play.simulation_ticks, ticks);
    let runtime = editor.instances[0];
    editor.toggle_play()?;
    assert!(editor.scene.local(runtime).is_err());
    assert_eq!(editor.authoring_document()?, authored);
    assert_eq!(editor.play.simulation_ticks, 0);
    standalone.stop()?;
    packaged.stop()?;
    editor.stop_workers()?;
    editor.stop_workers()?; // Explicit cleanup followed by Drop must not repeat destroy.
    assert_eq!(*editor_log.lock().unwrap(), *standalone_log.lock().unwrap());
    assert_eq!(
        *packaged_log.lock().unwrap(),
        *standalone_log.lock().unwrap()
    );
    assert_eq!(
        editor_log
            .lock()
            .unwrap()
            .iter()
            .filter(|(name, _)| *name == "destroy")
            .count(),
        1
    );
    Ok(())
}

#[test]
fn stop_discards_pending_commands_input_and_fractional_clock_before_replay()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Clean restart")?;
    let mut editor = App::from_project(&path)?;
    editor.edit_key(KeyCode::KeyD)?;
    let authored = editor.authoring_document()?;
    editor.toggle_play()?;
    let runtime = editor.instances[0];
    editor.advance_game(1. / 240.)?;
    editor.game_key(KeyCode::Space, ElementState::Pressed)?;
    let mut changed = editor.scene.local(runtime)?;
    changed.translation.x = 99.;
    editor
        .play
        .simulation
        .as_mut()
        .unwrap()
        .commands()
        .push(voxy_scene::SceneCommand::SetLocal(runtime, changed))?;
    editor.toggle_play()?;
    assert_eq!(editor.authoring_document()?, authored);
    assert!(editor.scene.local(runtime).is_err());
    assert_eq!(
        editor.play.player_input.checkpoint(),
        voxy_gameplay::player_input()?.checkpoint()
    );
    editor.toggle_play()?;
    editor.advance_game(1. / 240.)?;
    assert_eq!(editor.play.simulation_ticks, 0);
    assert_eq!(editor.authoring_document()?, authored);
    editor.advance_game(1. / 60.)?;
    assert_eq!(editor.play.simulation_ticks, 1);
    assert_eq!(editor.authoring_document()?, authored);
    editor.toggle_play()?;
    editor.stop_workers()?;
    Ok(())
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
struct RayGameConfig {
    reach: f64,
}
#[derive(Debug)]
struct RayGameBehavior {
    reach: f64,
    hits: Arc<AtomicU64>,
}
impl voxy_scene::Behavior for RayGameBehavior {
    fn fixed_update(&mut self, scene: &mut SceneGraph, owner: NodeId, _: f64) {
        let origin = scene
            .world_matrix(owner)
            .unwrap()
            .w_axis
            .truncate()
            .as_dvec3();
        let queries = voxy_gameplay::SceneQueries::from_scene(scene, 16).unwrap();
        let hit = queries
            .cast_ray(
                voxy_gameplay::RayQuery {
                    origin,
                    direction: glam::DVec3::X,
                    max_distance: self.reach,
                    solid: true,
                },
                &mut voxy_gameplay::SupportQueryBudget::new(16).unwrap(),
                |candidate| candidate != owner,
            )
            .unwrap()
            .unwrap();
        assert_ne!(hit.owner, owner);
        assert_eq!(hit.distance, 1.5);
        assert_eq!(hit.normal, Some(-glam::DVec3::X));
        self.hits.fetch_add(1, Ordering::Relaxed);
    }
}
fn ray_game_host(hits: Arc<AtomicU64>) -> Result<GameHost, Box<dyn std::error::Error>> {
    let mut components = editor_component_registry()?;
    components.register::<RayGameConfig>("test.ray-game.v1")?;
    let mut host = GameHost::new(components);
    host.behaviors = Some(BehaviorSetup::new(move |scene, simulation| {
        let owners: Vec<_> = scene
            .components::<RayGameConfig>()
            .map(|(owner, config)| (owner, *config))
            .collect();
        for (owner, config) in owners {
            simulation.attach(
                scene,
                owner,
                RayGameBehavior {
                    reach: config.reach,
                    hits: hits.clone(),
                },
            )?;
        }
        Ok(())
    }));
    Ok(host)
}
#[test]
fn scene_raycasts_run_in_editor_standalone_and_source_free_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Scene ray game")?;
    let hits = Arc::new(AtomicU64::new(0));
    let mut editor = app(&path, ray_game_host(hits.clone())?)?;
    editor.edit_key(KeyCode::KeyD)?;
    editor.edit_key(KeyCode::KeyD)?;
    let shooter = editor.instances[0];
    let target = editor.instances[1];
    editor
        .scene
        .insert_component(shooter, RayGameConfig { reach: 10. })?;
    editor.scene.insert_component(
        shooter,
        BoxCollider {
            half_extents: [0.1; 3],
        },
    )?;
    editor.scene.insert_component(
        target,
        BoxCollider {
            half_extents: [0.5; 3],
        },
    )?;
    let mut transform = editor.scene.local(target)?;
    transform.translation = glam::Vec3::X * 2.;
    editor.scene.set_local(target, transform)?;
    editor.commit_authoring()?;
    editor.save_authoring()?;
    let authored = editor.authoring_document()?;
    for _ in 0..2 {
        editor.toggle_play()?;
        for _ in 0..120 {
            editor.advance_game(1. / 60.)?;
        }
        editor.toggle_play()?;
        assert_eq!(editor.authoring_document()?, authored);
    }
    assert_eq!(hits.load(Ordering::Relaxed), 240);
    editor.stop_workers()?;
    run_project_with_host(&path, ViewportMode::GameCheck, ray_game_host(hits.clone())?)?;
    assert_eq!(hits.load(Ordering::Relaxed), 360);
    let package = temp.0.join("rays.vpak");
    export_project_package_with_host(&path, &package, ray_game_host(hits.clone())?)?;
    assert_eq!(hits.load(Ordering::Relaxed), 360); // Build validates but never simulates time.
    std::fs::remove_dir_all(path.parent().unwrap())?;
    run_packaged_game_with_host(
        &package,
        ViewportMode::GameCheck,
        ray_game_host(hits.clone())?,
    )?;
    assert_eq!(hits.load(Ordering::Relaxed), 480);
    Ok(())
}
