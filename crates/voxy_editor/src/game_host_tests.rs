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
    events: Arc<AtomicU64>,
    target: Option<NodeId>,
    ticks: usize,
}
impl voxy_scene::Behavior for RayGameBehavior {
    fn fixed_update(&mut self, scene: &mut SceneGraph, owner: NodeId, _: f64) {
        use voxy_gameplay::BoxTrigger;

        if self.ticks == 60 {
            scene
                .insert_component(
                    owner,
                    BoxTrigger {
                        half_extents: [0.5; 3],
                    },
                )
                .unwrap();
        }
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
        for participant in [owner, hit.owner] {
            assert_eq!(
                scene
                    .component::<voxy_gameplay::CollisionLayers>(participant)
                    .unwrap()
                    .copied(),
                Some(voxy_gameplay::CollisionLayers {
                    membership: 4,
                    mask: 4
                }),
            );
        }
        assert_ne!(hit.owner, owner);
        assert_eq!(hit.distance, 1.5);
        assert_eq!(hit.normal, Some(-glam::DVec3::X));
        let overlapping = queries
            .overlap_box(
                owner,
                [1.75, 0.5, 0.5],
                &mut voxy_gameplay::SupportQueryBudget::new(16).unwrap(),
                16,
                |candidate| candidate != owner,
            )
            .unwrap();
        assert_eq!(overlapping, [hit.owner]);
        let swept = queries
            .sweep_box_relative(
                owner,
                [0.25; 3],
                glam::DVec3::X * 4.,
                &mut voxy_gameplay::SupportQueryBudget::new(16).unwrap(),
                16,
                |candidate| (candidate != owner).then_some(glam::DVec3::NEG_X * 4.),
            )
            .unwrap();
        assert_eq!(
            swept,
            [voxy_gameplay::BoxSweepHit {
                owner: hit.owner,
                enter: 0.15625,
                exit: 0.34375,
                normal: Some(glam::DVec3::NEG_X),
            }]
        );
        assert_eq!(
            queries
                .cast_box_relative(
                    owner,
                    [0.25; 3],
                    glam::DVec3::X * 4.,
                    &mut voxy_gameplay::SupportQueryBudget::new(16).unwrap(),
                    |candidate| (candidate != owner).then_some(glam::DVec3::NEG_X * 4.),
                )
                .unwrap(),
            swept.first().copied(),
        );
        self.target = Some(hit.owner);
        self.ticks += 1;
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    fn trigger_event(
        &mut self,
        _: &mut SceneGraph,
        owner: NodeId,
        event: voxy_scene::TriggerEvent,
    ) {
        use voxy_scene::TriggerPhase;
        assert_eq!(event.trigger, owner);
        assert_eq!(Some(event.collider), self.target);
        match (self.ticks, event.phase) {
            (1, TriggerPhase::Enter) | (61, TriggerPhase::Exit) => {}
            other => panic!("unexpected trigger event: {other:?}"),
        }
        self.events.fetch_add(1, Ordering::Relaxed);
    }
}
fn ray_game_host(
    hits: Arc<AtomicU64>,
    events: Arc<AtomicU64>,
) -> Result<GameHost, Box<dyn std::error::Error>> {
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
                    events: events.clone(),
                    target: None,
                    ticks: 0,
                },
            )?;
        }
        Ok(())
    }));
    Ok(host)
}
#[test]
fn scene_queries_and_triggers_run_in_editor_standalone_and_source_free_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Scene ray game")?;
    let hits = Arc::new(AtomicU64::new(0));
    let events = Arc::new(AtomicU64::new(0));
    let mut editor = app(&path, ray_game_host(hits.clone(), events.clone())?)?;
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
        shooter,
        voxy_gameplay::BoxTrigger {
            half_extents: [1.75, 0.5, 0.5],
        },
    )?;
    editor.scene.insert_component(
        target,
        BoxCollider {
            half_extents: [0.5; 3],
        },
    )?;
    for participant in [shooter, target] {
        editor.scene.insert_component(
            participant,
            voxy_gameplay::CollisionLayers {
                membership: 4,
                mask: 4,
            },
        )?;
    }
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
    run_project_with_host(
        &path,
        ViewportMode::GameCheck,
        ray_game_host(hits.clone(), events.clone())?,
    )?;
    assert_eq!(hits.load(Ordering::Relaxed), 360);
    let package = temp.0.join("rays.vpak");
    export_project_package_with_host(
        &path,
        &package,
        ray_game_host(hits.clone(), events.clone())?,
    )?;
    assert_eq!(hits.load(Ordering::Relaxed), 360); // Build validates but never simulates time.
    std::fs::remove_dir_all(path.parent().unwrap())?;
    run_packaged_game_with_host(
        &package,
        ViewportMode::GameCheck,
        ray_game_host(hits.clone(), events.clone())?,
    )?;
    assert_eq!(hits.load(Ordering::Relaxed), 480);
    assert_eq!(events.load(Ordering::Relaxed), 8);
    Ok(())
}

#[test]
fn fixed_step_delivers_both_events_for_a_character_crossing_a_thin_trigger()
-> Result<(), Box<dyn std::error::Error>> {
    use voxy_gameplay::{BoxTrigger, CharacterBody, TriggerEvent, TriggerPhase};

    #[derive(Debug)]
    struct Recorder(Arc<Mutex<Vec<TriggerEvent>>>);
    impl voxy_scene::Behavior for Recorder {
        fn trigger_event(&mut self, _: &mut SceneGraph, _: NodeId, event: TriggerEvent) {
            self.0.lock().unwrap().push(event);
        }
    }
    let events = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&events);
    let mut host = GameHost::new(editor_component_registry()?);
    host.behaviors = Some(BehaviorSetup::new(move |scene, simulation| {
        let trigger = scene.components::<BoxTrigger>().next().unwrap().0;
        simulation.attach(scene, trigger, Recorder(log.clone()))?;
        Ok(())
    }));
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Continuous trigger")?;
    let mut editor = app(&path, host)?;
    editor.edit_key(KeyCode::KeyD)?;
    editor.edit_key(KeyCode::KeyD)?;
    let trigger = editor.instances[0];
    let character = editor.instances[1];
    editor.scene.set_local(
        trigger,
        Transform {
            translation: glam::Vec3::X,
            ..Default::default()
        },
    )?;
    editor.scene.set_local(character, Transform::default())?;
    editor.scene.insert_component(
        character,
        CharacterBody {
            half_extents: [0.1; 3],
            speed: 120.,
            gravity: 0.,
            jump_speed: 0.,
        },
    )?;
    editor.commit_authoring()?;
    editor.panel_action(panels::Action::Select(0))?;
    editor.panel_action(panels::Action::Physics)?;
    editor.panel_action(panels::Action::Trigger)?;
    editor.panel_action(panels::Action::Field(0))?;
    editor.field_key(KeyCode::Digit0, Some("0.05"))?;
    editor.field_key(KeyCode::Enter, None)?;
    let authored = editor.authoring_document()?;
    assert_eq!(
        editor
            .scene
            .component::<BoxTrigger>(editor.instances[0])?
            .unwrap()
            .half_extents,
        [0.05, 0.5, 0.5],
    );
    editor.panel_action(panels::Action::Field(0))?;
    editor.field_key(KeyCode::Minus, Some("-1"))?;
    assert!(editor.field_key(KeyCode::Enter, None).is_err());
    editor.field_key(KeyCode::Escape, None)?;
    assert_eq!(editor.authoring_document()?, authored);
    editor.toggle_play()?;
    let trigger = editor.instances[0];
    let character = editor.instances[1];
    editor.play.player_input.event(voxy_gameplay::RIGHT, 1.)?;
    editor.advance_game(1. / 60.)?;
    assert_eq!(editor.scene.local(character)?.translation.x, 2.);
    assert_eq!(
        *events.lock().unwrap(),
        [TriggerPhase::Enter, TriggerPhase::Exit].map(|phase| TriggerEvent {
            trigger,
            collider: character,
            phase,
        }),
    );
    assert!(
        !editor
            .play
            .triggers
            .as_ref()
            .unwrap()
            .contains(trigger, character)
    );
    editor.advance_game(1. / 60.)?;
    assert_eq!(events.lock().unwrap().len(), 2);
    editor.toggle_play()?;
    editor.panel_action(panels::Action::Select(0))?;
    editor.panel_action(panels::Action::Trigger)?;
    assert!(
        editor
            .scene
            .component::<BoxTrigger>(editor.instances[0])?
            .is_none()
    );
    editor.edit_key(KeyCode::KeyZ)?;
    assert_eq!(editor.authoring_document()?, authored);
    editor.stop_workers()?;
    Ok(())
}

#[test]
fn physics_inspector_edits_exact_collision_bits_and_reopens_project()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("layers"), "Collision layers")?;
    let mut editor = app(&path, GameHost::new(editor_component_registry()?))?;
    editor.edit_key(KeyCode::KeyD)?;
    editor.panel_action(panels::Action::Collider)?;
    editor.panel_action(panels::Action::Physics)?;
    for (index, text) in [(3, "2147483648"), (4, "4294967294")] {
        editor.panel_action(panels::Action::Field(index))?;
        editor.field_key(KeyCode::Digit2, Some(text))?;
        editor.field_key(KeyCode::Enter, None)?;
    }
    let saved = editor.authoring_document()?;
    let layer = |doc: &voxy_scene::SceneDocument| {
        doc.objects[0].components["game.collision-layers.v1"]["mask"].as_u64()
    };
    assert_eq!(layer(&saved), Some(4_294_967_294));
    assert_eq!(
        saved.objects[0].components["game.collision-layers.v1"]["membership"].as_u64(),
        Some(2_147_483_648)
    );
    editor.panel_action(panels::Action::Field(4))?;
    editor.field_key(KeyCode::Digit4, Some("4294967296"))?;
    assert!(editor.field_key(KeyCode::Enter, None).is_err());
    editor.field_key(KeyCode::Escape, None)?;
    assert_eq!(editor.authoring_document()?, saved);
    editor.edit_key(KeyCode::KeyZ)?;
    assert_eq!(
        layer(&editor.authoring_document()?),
        Some(u64::from(u32::MAX))
    );
    editor.edit_key(KeyCode::KeyY)?;
    assert_eq!(editor.authoring_document()?, saved);
    editor.save_authoring()?;
    editor.stop_workers()?;
    let mut reopened = app(&path, GameHost::new(editor_component_registry()?))?;
    assert_eq!(reopened.authoring_document()?, saved);
    reopened.stop_workers()?;
    Ok(())
}

#[test]
fn dynamic_body_authored_in_physics_inspector_hits_wall_in_play_and_source_free_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Dynamic body")?;
    let host = || Ok::<_, Box<dyn std::error::Error>>(GameHost::new(editor_component_registry()?));
    let mut editor = app(&path, host()?)?;
    editor.edit_key(KeyCode::KeyD)?;
    editor.edit_key(KeyCode::KeyD)?;
    let owner = editor.instances[0];
    let wall = editor.instances[1];
    editor.scene.set_local(owner, Transform::default())?;
    editor.scene.set_local(
        wall,
        Transform {
            translation: glam::Vec3::new(0.6, -2., 0.),
            ..Default::default()
        },
    )?;
    editor.scene.insert_component(
        wall,
        BoxCollider {
            half_extents: [0.05, 4., 0.5],
        },
    )?;
    editor.commit_authoring()?;
    editor.panel_action(panels::Action::Select(0))?;
    editor.panel_action(panels::Action::Physics)?;
    editor.panel_action(panels::Action::DynamicBody)?;
    for (index, value) in [(5, "2.000000000000001"), (6, "1.000000000000001")] {
        editor.panel_action(panels::Action::Field(index))?;
        editor.field_key(KeyCode::Digit1, Some(value))?;
        editor.field_key(KeyCode::Enter, None)?;
    }
    let authored = editor.authoring_document()?;
    let descriptor = editor
        .scene
        .component::<voxy_gameplay::LiquidBody>(owner)?
        .unwrap();
    assert_eq!(
        descriptor.mass_kg.to_bits(),
        2.000000000000001_f64.to_bits()
    );
    assert_eq!(
        descriptor.initial_velocity_m_s[0].to_bits(),
        1.000000000000001_f64.to_bits()
    );
    editor.panel_action(panels::Action::DynamicBody)?;
    assert!(
        editor
            .scene
            .component::<voxy_gameplay::LiquidBody>(owner)?
            .is_none()
    );
    editor.edit_key(KeyCode::KeyZ)?;
    assert_eq!(editor.authoring_document()?, authored);
    editor.save_authoring()?;

    let mut trace = Vec::new();
    editor.toggle_play()?;
    for _ in 0..60 {
        editor.advance_game(1. / 60.)?;
        trace.push(editor.authoring_document()?);
    }
    let end = &trace.last().unwrap().objects[0].translation;
    assert!((end[0] - 0.5).abs() < 1e-6, "{end:?}");
    assert!(end[1] < -4., "{end:?}");
    editor.toggle_play()?;
    assert_eq!(editor.authoring_document()?, authored);
    editor.stop_workers()?;

    let package = temp.0.join("body.vpak");
    export_project_package_with_host(&path, &package, host()?)?;
    {
        let mut standalone = HeadlessGame::open_project(&path, host()?)?;
        for expected in &trace {
            standalone.advance(1. / 60.)?;
            assert_eq!(&standalone.snapshot()?, expected);
        }
    }
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut packaged = HeadlessGame::open_package(&package, host()?)?;
    for expected in &trace {
        packaged.advance(1. / 60.)?;
        assert_eq!(&packaged.snapshot()?, expected);
    }
    Ok(())
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
fn kinematic_body_sweeps_to_wall_and_matches_source_free_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Kinematic body")?;
    let host = || Ok::<_, Box<dyn std::error::Error>>(GameHost::new(editor_component_registry()?));
    let mut editor = app(&path, host()?)?;
    editor.edit_key(KeyCode::KeyD)?;
    editor.edit_key(KeyCode::KeyD)?;
    editor.edit_key(KeyCode::KeyD)?;
    editor.edit_key(KeyCode::KeyD)?;
    let mover = editor.instances[0];
    let wall = editor.instances[1];
    let second_mover = editor.instances[2];
    let character = editor.instances[3];
    editor.scene.set_local(mover, Transform::default())?;
    editor.scene.set_local(
        wall,
        Transform {
            translation: glam::Vec3::X * 0.6,
            ..Default::default()
        },
    )?;
    editor.scene.insert_component(
        mover,
        BoxCollider {
            half_extents: [0.05; 3],
        },
    )?;
    editor.scene.insert_component(
        wall,
        BoxCollider {
            half_extents: [0.01, 0.5, 0.5],
        },
    )?;
    editor.scene.set_local(
        second_mover,
        Transform {
            translation: glam::Vec3::Z,
            ..Default::default()
        },
    )?;
    editor.scene.set_local(
        character,
        Transform {
            translation: glam::Vec3::new(0.8, 0., 1.),
            ..Default::default()
        },
    )?;
    editor
        .scene
        .insert_component(second_mover, BoxCollider::default())?;
    editor.scene.insert_component(
        second_mover,
        voxy_gameplay::KinematicBody {
            velocity_m_s: [120., 0., 0.],
        },
    )?;
    editor.scene.insert_component(
        character,
        CharacterBody {
            gravity: 0.,
            ..Default::default()
        },
    )?;
    editor.commit_authoring()?;
    editor.panel_action(panels::Action::Select(0))?;
    editor.panel_action(panels::Action::Physics)?;
    editor.panel_action(panels::Action::KinematicBody)?;
    editor.panel_action(panels::Action::Field(5))?;
    editor.field_key(KeyCode::Digit1, Some("120"))?;
    editor.field_key(KeyCode::Enter, None)?;
    editor.save_authoring()?;
    let authored = editor.authoring_document()?;
    let mut trace = Vec::new();
    editor.toggle_play()?;
    for _ in 0..8 {
        editor.advance_game(1. / 60.)?;
        let snapshot = editor.authoring_document()?;
        let x = snapshot.objects[0].translation[0];
        assert!((x - 0.54).abs() < 2e-7, "{x}");
        assert_eq!(
            snapshot.objects[1].translation,
            authored.objects[1].translation
        );
        let second_x = snapshot.objects[2].translation[0];
        assert!((second_x - 0.7).abs() < 2e-7, "{second_x}");
        assert_eq!(
            snapshot.objects[3].translation,
            authored.objects[3].translation
        );
        trace.push(snapshot);
    }
    editor.toggle_play()?;
    assert_eq!(editor.authoring_document()?, authored);
    editor.stop_workers()?;

    let package = temp.0.join("kinematic.vpak");
    export_project_package_with_host(&path, &package, host()?)?;
    {
        let mut standalone = HeadlessGame::open_project(&path, host()?)?;
        for expected in &trace {
            standalone.advance(1. / 60.)?;
            assert_eq!(&standalone.snapshot()?, expected);
        }
    }
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut packaged = HeadlessGame::open_package(&package, host()?)?;
    for expected in &trace {
        packaged.advance(1. / 60.)?;
        assert_eq!(&packaged.snapshot()?, expected);
    }
    Ok(())
}

#[test]
fn kinematic_platform_carries_character_and_stops_at_rider_obstruction()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Moving platform")?;
    let host = || Ok::<_, Box<dyn std::error::Error>>(GameHost::new(editor_component_registry()?));
    let mut editor = app(&path, host()?)?;
    for _ in 0..3 {
        editor.edit_key(KeyCode::KeyD)?;
    }
    let [platform, rider, wall] = editor.instances[..] else {
        return Err("expected three scene nodes".into());
    };
    editor.scene.set_local(platform, Transform::default())?;
    editor.scene.set_local(
        rider,
        Transform {
            translation: glam::Vec3::Y * 0.25,
            ..Default::default()
        },
    )?;
    editor.scene.set_local(
        wall,
        Transform {
            translation: glam::Vec3::new(0.5, 0.375, 0.),
            ..Default::default()
        },
    )?;
    editor.scene.insert_component(
        platform,
        BoxCollider {
            half_extents: [0.25, 0.125, 0.25],
        },
    )?;
    editor.scene.insert_component(
        platform,
        voxy_gameplay::KinematicBody {
            velocity_m_s: [1., 0.25, 0.],
        },
    )?;
    editor.scene.insert_component(
        rider,
        CharacterBody {
            half_extents: [0.125; 3],
            ..Default::default()
        },
    )?;
    editor.scene.insert_component(
        wall,
        BoxCollider {
            half_extents: [0.125, 0.125, 0.25],
        },
    )?;
    editor.commit_authoring()?;
    editor.save_authoring()?;
    let authored = editor.authoring_document()?;
    let mut trace = Vec::new();
    editor.toggle_play()?;
    for _ in 0..24 {
        editor.advance_game(1. / 60.)?;
        let snapshot = editor.authoring_document()?;
        let [x, y, _] = snapshot.objects[0].translation;
        let [rider_x, rider_y, _] = snapshot.objects[1].translation;
        assert!((rider_x - x).abs() < 1e-7);
        assert!((rider_y - y - 0.25).abs() < 1e-7);
        trace.push(snapshot);
    }
    let first = trace.first().unwrap().objects[0].translation;
    let end = trace.last().unwrap().objects[0].translation;
    assert!(first[0] > 0. && first[1] > 0.);
    assert!((end[0] - 0.25).abs() < 1e-6, "{end:?}");
    assert!((end[1] - 0.0625).abs() < 1e-6, "{end:?}");
    editor.play.player_input.event(voxy_gameplay::JUMP, 1.)?;
    for _ in 0..3 {
        editor.advance_game(1. / 60.)?;
    }
    let released = editor.authoring_document()?;
    let [platform_x, platform_y, _] = released.objects[0].translation;
    let [rider_x, rider_y, _] = released.objects[1].translation;
    assert!(platform_x > end[0] && rider_x <= end[0] + 1e-6);
    assert!(rider_y - platform_y > 0.25);
    editor.toggle_play()?;
    assert_eq!(editor.authoring_document()?, authored);
    editor.stop_workers()?;

    let package = temp.0.join("platform.vpak");
    export_project_package_with_host(&path, &package, host()?)?;
    {
        let mut standalone = HeadlessGame::open_project(&path, host()?)?;
        for expected in &trace {
            standalone.advance(1. / 60.)?;
            assert_eq!(&standalone.snapshot()?, expected);
        }
    }
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut packaged = HeadlessGame::open_package(&package, host()?)?;
    for expected in &trace {
        packaged.advance(1. / 60.)?;
        assert_eq!(&packaged.snapshot()?, expected);
    }
    Ok(())
}

#[test]
fn kinematic_scene_collider_pushes_dynamic_body_in_play_and_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Kinematic impact")?;
    let host = || Ok::<_, Box<dyn std::error::Error>>(GameHost::new(editor_component_registry()?));
    let mut editor = app(&path, host()?)?;
    editor.edit_key(KeyCode::KeyD)?;
    editor.edit_key(KeyCode::KeyD)?;
    let [platform, body] = editor.instances[..] else {
        return Err("expected a platform and a dynamic body".into());
    };
    editor.scene.set_local(platform, Transform::default())?;
    editor.scene.set_local(
        body,
        Transform {
            translation: glam::Vec3::X * 0.25,
            ..Default::default()
        },
    )?;
    editor.scene.insert_component(
        platform,
        BoxCollider {
            half_extents: [0.0625, 0.5, 0.5],
        },
    )?;
    editor.scene.insert_component(
        body,
        BoxCollider {
            half_extents: [0.0625; 3],
        },
    )?;
    editor.scene.insert_component(
        platform,
        voxy_gameplay::KinematicBody {
            velocity_m_s: [15., 0., 0.],
        },
    )?;
    editor.scene.insert_component(
        body,
        voxy_gameplay::LiquidBody {
            mass_kg: 2.,
            initial_velocity_m_s: [0.; 3],
        },
    )?;
    editor.commit_authoring()?;
    editor.save_authoring()?;
    let authored = editor.authoring_document()?;
    let mut trace = Vec::new();
    editor.toggle_play()?;
    for tick in 1..=4 {
        editor.advance_game(1. / 60.)?;
        let frame = editor.authoring_document()?;
        let x = tick as f32 * 0.25;
        assert_eq!(frame.objects[0].translation[0], x);
        assert_eq!(frame.objects[1].translation[0], x + 0.125);
        let (_, state) = editor.play.liquid.as_ref().unwrap().body_state().unwrap();
        assert!((state.velocity[0] - 15.).abs() < 1e-11);
        assert!(state.velocity[1] < 0.);
        trace.push(frame);
    }
    editor.toggle_play()?;
    assert_eq!(editor.authoring_document()?, authored);
    editor.stop_workers()?;

    let package = temp.0.join("kinematic-impact.vpak");
    export_project_package_with_host(&path, &package, host()?)?;
    {
        let mut standalone = HeadlessGame::open_project(&path, host()?)?;
        for expected in &trace {
            standalone.advance(1. / 60.)?;
            assert_eq!(&standalone.snapshot()?, expected);
        }
    }
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut packaged = HeadlessGame::open_package(&package, host()?)?;
    for expected in &trace {
        packaged.advance(1. / 60.)?;
        assert_eq!(&packaged.snapshot()?, expected);
    }
    Ok(())
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
fn kinematic_scene_collider_stops_when_dynamic_body_is_pinned()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Kinematic constraint")?;
    let host = || Ok::<_, Box<dyn std::error::Error>>(GameHost::new(editor_component_registry()?));
    let mut editor = app(&path, host()?)?;
    for _ in 0..4 {
        editor.edit_key(KeyCode::KeyD)?;
    }
    let [platform, body, wall, free_platform] = editor.instances[..] else {
        return Err("expected two platforms, a body and a wall".into());
    };
    for (node, x, half_extents) in [
        (platform, 0., [0.0625, 0.5, 0.5]),
        (body, 0.25, [0.0625; 3]),
        (wall, 0.5, [0.0625, 0.5, 0.5]),
    ] {
        editor.scene.set_local(
            node,
            Transform {
                translation: glam::Vec3::X * x,
                ..Default::default()
            },
        )?;
        editor
            .scene
            .insert_component(node, BoxCollider { half_extents })?;
    }
    editor.scene.insert_component(
        platform,
        voxy_gameplay::KinematicBody {
            velocity_m_s: [30., 0., 0.],
        },
    )?;
    editor.scene.insert_component(
        body,
        voxy_gameplay::LiquidBody {
            mass_kg: 2.,
            initial_velocity_m_s: [0.; 3],
        },
    )?;
    editor.scene.set_local(
        free_platform,
        Transform {
            translation: glam::Vec3::Z * 2.,
            ..Default::default()
        },
    )?;
    editor
        .scene
        .insert_component(free_platform, BoxCollider::default())?;
    editor.scene.insert_component(
        free_platform,
        voxy_gameplay::KinematicBody {
            velocity_m_s: [7.5, 0., 0.],
        },
    )?;
    editor.commit_authoring()?;
    editor.save_authoring()?;
    let authored = editor.authoring_document()?;
    let mut trace = Vec::new();
    editor.toggle_play()?;
    for tick in 0..4 {
        editor.advance_game(1. / 60.)?;
        let frame = editor.authoring_document()?;
        let platform_x = frame.objects[0].translation[0];
        let body_x = frame.objects[1].translation[0];
        assert!(
            (0.249999..=0.25).contains(&platform_x),
            "tick {tick}: {platform_x}"
        );
        assert!(
            (0.374999..=0.375).contains(&body_x),
            "tick {tick}: {body_x}"
        );
        assert_eq!(
            frame.objects[2].translation,
            authored.objects[2].translation
        );
        assert_eq!(
            frame.objects[3].translation,
            [(tick + 1) as f32 * 0.125, 0., 2.],
        );
        let (_, state) = editor.play.liquid.as_ref().unwrap().body_state().unwrap();
        assert!(state.position[0] <= 0.375 + 1e-12, "{:?}", state.position);
        assert!(state.position[0] >= f64::from(platform_x) + 0.125 - 1e-12);
        if tick > 0 {
            assert!(
                state.velocity[0].abs() < 1e-10,
                "tick {tick}: {:?}",
                state.velocity
            );
        }
        assert!(state.velocity[1] < 0.);
        trace.push(frame);
    }
    editor.toggle_play()?;
    assert_eq!(editor.authoring_document()?, authored);
    editor.stop_workers()?;

    let package = temp.0.join("pinned.vpak");
    export_project_package_with_host(&path, &package, host()?)?;
    {
        let mut standalone = HeadlessGame::open_project(&path, host()?)?;
        for expected in &trace {
            standalone.advance(1. / 60.)?;
            assert_eq!(&standalone.snapshot()?, expected);
        }
    }
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut packaged = HeadlessGame::open_package(&package, host()?)?;
    for expected in &trace {
        packaged.advance(1. / 60.)?;
        assert_eq!(&packaged.snapshot()?, expected);
    }
    Ok(())
}

#[test]
fn moving_scene_platform_transfers_impulse_to_liquid_in_play_and_package()
-> Result<(), Box<dyn std::error::Error>> {
    check_scene_platform_liquid(LiquidPlatformCase::Particle)
}

#[test]
fn moving_scene_platform_stops_at_pinned_liquid_in_play_and_package()
-> Result<(), Box<dyn std::error::Error>> {
    check_scene_platform_liquid(LiquidPlatformCase::Pinned)
}

#[test]
fn moving_scene_platform_transfers_impulse_to_liquid_and_dynamic_body_together()
-> Result<(), Box<dyn std::error::Error>> {
    check_scene_platform_liquid(LiquidPlatformCase::Parallel)
}

#[test]
fn moving_scene_platform_passes_impulse_through_liquid_to_a_dynamic_body()
-> Result<(), Box<dyn std::error::Error>> {
    check_scene_platform_liquid(LiquidPlatformCase::Relay)
}

#[test]
fn moving_scene_platform_compresses_particle_against_a_finite_body_in_one_step()
-> Result<(), Box<dyn std::error::Error>> {
    check_scene_platform_liquid(LiquidPlatformCase::PackedRelay)
}

#[test]
#[ignore = "known continuous contact budget failure after off-center particle impact"]
fn moving_scene_platform_transmits_off_center_impulse_to_spinning_body()
-> Result<(), Box<dyn std::error::Error>> {
    check_scene_platform_liquid(LiquidPlatformCase::RotatingRelay)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LiquidPlatformCase {
    Particle,
    Pinned,
    Parallel,
    Relay,
    PackedRelay,
    RotatingRelay,
}

#[expect(
    clippy::too_many_lines,
    reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
)]
fn check_scene_platform_liquid(case: LiquidPlatformCase) -> Result<(), Box<dyn std::error::Error>> {
    let pinned = case == LiquidPlatformCase::Pinned;
    let rotating = case == LiquidPlatformCase::RotatingRelay;
    let packed = case == LiquidPlatformCase::PackedRelay;
    let relay = case == LiquidPlatformCase::Relay || packed || rotating;
    let mixed = case == LiquidPlatformCase::Parallel || relay;
    let temp = Temp::new();
    let path = project::create(&temp.0.join("source"), "Liquid platform")?;
    let root = path.parent().unwrap();
    let manifest_path = root.join("assets.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    manifest["assets"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"asset":"water","source":"water.json"}));
    std::fs::write(manifest_path, serde_json::to_vec(&manifest)?)?;
    std::fs::write(
        root.join("water.json"),
        br#"{"rest_density":1000,"sound_speed":20,"viscosity":0.001}"#,
    )?;

    let mut editor = App::from_project(&path)?;
    let nozzles = if pinned {
        vec![[-0.04, -0.04], [-0.04, 0.04], [0.04, -0.04], [0.04, 0.04]]
    } else {
        vec![[0., 0.]]
    };
    for _ in 0..1 + nozzles.len() + usize::from(pinned || relay) + usize::from(mixed) {
        editor.edit_key(KeyCode::KeyD)?;
    }
    let platform = editor.instances[0];
    editor.scene.set_local(platform, Transform::default())?;
    editor.scene.insert_component(
        platform,
        BoxCollider {
            half_extents: [0.0625, 0.5, 0.5],
        },
    )?;
    editor.scene.insert_component(
        platform,
        voxy_gameplay::KinematicBody {
            velocity_m_s: [if pinned { 30. } else { 15. }, 0., 0.],
        },
    )?;
    if relay {
        editor.scene.insert_component(
            platform,
            voxy_gameplay::CollisionLayers {
                membership: 2,
                mask: 1 | 4,
            },
        )?;
    }
    let dt = 1. / 60.;
    let volume = if pinned { 0.004 } else { 0.001 };
    for (index, &[y, z]) in nozzles.iter().enumerate() {
        let source = editor.instances[1 + index];
        editor.scene.set_local(
            source,
            Transform {
                translation: glam::Vec3::new(0.25, y, z),
                ..Default::default()
            },
        )?;
        editor.scene.insert_component(
            source,
            voxy_gameplay::LiquidSource {
                pulses: vec![voxy_gameplay::LiquidPulse {
                    start_s: 0.,
                    duration_s: dt,
                    volume_m3: volume,
                    speed_m_s: 0.,
                }],
                density_kg_m3: 1000.,
                particle_volume_m3: volume,
                nozzle_radius_m: 0.,
                direction: [1., 0., 0.],
                material_asset: "water".into(),
            },
        )?;
    }
    if pinned || relay {
        let wall = editor.instances[1 + nozzles.len()];
        editor.scene.set_local(
            wall,
            Transform {
                translation: glam::Vec3::X * if relay { 0.3125 } else { 0.5 },
                ..Default::default()
            },
        )?;
        editor.scene.insert_component(
            wall,
            BoxCollider {
                half_extents: [0.0625, 0.5, 0.5],
            },
        )?;
        if relay {
            editor.scene.insert_component(
                wall,
                voxy_gameplay::CollisionLayers {
                    membership: 4,
                    mask: 2,
                },
            )?;
        }
    }
    if mixed {
        let body = *editor.instances.last().unwrap();
        editor.scene.set_local(
            body,
            Transform {
                translation: if relay {
                    glam::Vec3::new(
                        if packed { 0.34375 } else { 0.40625 },
                        0.,
                        if rotating { 0.025 } else { 0. },
                    )
                } else {
                    glam::Vec3::new(0.25, 0., 0.25)
                },
                ..Default::default()
            },
        )?;
        editor.scene.insert_component(
            body,
            BoxCollider {
                half_extents: [0.0625; 3],
            },
        )?;
        editor.scene.insert_component(
            body,
            voxy_gameplay::LiquidBody {
                mass_kg: 2.,
                initial_velocity_m_s: [0.; 3],
            },
        )?;
    }
    if rotating {
        editor.scene.insert_component(
            *editor.instances.last().unwrap(),
            voxy_gameplay::LiquidMassDistribution {
                parts: vec![voxy_gameplay::LiquidMassPart {
                    mass_kg: 2.,
                    center_m: [0.; 3],
                    half_edges_m: [[0.0625, 0., 0.], [0., 0.0625, 0.], [0., 0., 0.0625]],
                }],
            },
        )?;
    }
    if relay {
        editor.scene.insert_component(
            *editor.instances.last().unwrap(),
            voxy_gameplay::CollisionLayers {
                membership: 8,
                mask: 1,
            },
        )?;
    }
    editor.commit_authoring()?;
    editor.save_authoring()?;
    let authored = editor.authoring_document()?;
    let read = |app: &App| {
        app.play
            .liquid
            .as_ref()
            .unwrap()
            .liquid()
            .particles()
            .to_vec()
    };
    let mut trace = Vec::new();
    editor.toggle_play()?;
    for tick in 1..=4 {
        editor.advance_game(dt)?;
        let particles = read(&editor);
        assert_eq!(particles.len(), nozzles.len());
        if pinned {
            let x = f64::from(editor.scene.local(editor.instances[0])?.translation.x);
            if tick <= 2 {
                assert!(
                    (0.324999..=0.32500001).contains(&x),
                    "tick {tick}: platform {x}"
                );
            }
            assert!(x <= 0.375);
            assert!(
                particles
                    .iter()
                    .any(|particle| particle.velocity[2].abs() > 1e-6)
            );
            for particle in &particles {
                if particle.position[1].abs() < 0.525 && particle.position[2].abs() < 0.525 {
                    assert!(
                        particle.position[0] <= 0.4125 + 1e-10,
                        "tick {tick}: {particle:?}"
                    );
                    assert!(
                        particle.position[0] >= x + 0.0875 - 1e-10,
                        "tick {tick}: {particle:?}"
                    );
                }
            }
        } else if rotating {
            assert!(particles[0].velocity[0] > 0.);
        } else if relay {
            let expected = if packed || tick == 1 { 11.25 } else { 3.75 };
            assert!(
                (particles[0].velocity[0] - expected).abs() < 1e-10,
                "tick {tick}: {:?}",
                particles[0]
            );
            assert_eq!(
                editor.scene.local(editor.instances[0])?.translation.x,
                0.1875
            );
        } else {
            assert!((particles[0].velocity[0] - 15.).abs() < 1e-10);
            assert!((particles[0].position[0] - (f64::from(tick) * 0.25 + 0.0875)).abs() < 1e-10);
            assert!(particles[0].velocity[1] < 0.);
        }
        let body = editor
            .play
            .liquid
            .as_ref()
            .unwrap()
            .body_state()
            .map(|(_, state)| state);
        let rigid = editor
            .play
            .liquid
            .as_ref()
            .unwrap()
            .body_rigid_states()
            .next()
            .map(|(_, state)| state);
        if rotating {
            let state = rigid.unwrap();
            if tick == 1 {
                assert_eq!(state.motion.velocity[0], 0.);
                assert_eq!(state.spin.unwrap().angular_momentum, [0.; 3]);
            } else {
                assert!(state.motion.velocity[0] > 0., "tick {tick}: {state:?}");
                assert!(state.spin.unwrap().angular_momentum[1] < -1e-5, "{state:?}");
            }
        } else if relay {
            let state = body.unwrap();
            let expected = if packed {
                11.25
            } else if tick == 1 {
                0.
            } else {
                3.75
            };
            assert!(
                (state.velocity[0] - expected).abs() < 1e-10,
                "tick {tick}: {state:?}"
            );
            if !packed && tick >= 2 {
                let momentum =
                    particles[0].mass * particles[0].velocity[0] + state.mass * state.velocity[0];
                assert!((momentum - 11.25).abs() < 1e-10);
            }
        } else if mixed {
            let state = body.unwrap();
            assert!((state.velocity[0] - 15.).abs() < 1e-10, "{state:?}");
            assert!((state.position[0] - (f64::from(tick) * 0.25 + 0.125)).abs() < 1e-10);
            assert!(state.velocity[1] < 0.);
        }
        trace.push((editor.authoring_document()?, particles, rigid));
    }
    editor.toggle_play()?;
    assert_eq!(editor.authoring_document()?, authored);
    editor.stop_workers()?;

    let package_path = temp.0.join("liquid-platform.vpak");
    export_project_package(&path, &package_path)?;
    let replay = |path: &std::path::Path| -> Result<(), Box<dyn std::error::Error>> {
        let mut standalone = App::from_project(path)?;
        standalone.start_standalone()?;
        for (scene, particles, body) in &trace {
            standalone.advance_game(dt)?;
            assert_eq!(&standalone.authoring_document()?, scene);
            assert_eq!(&read(&standalone), particles);
            assert_eq!(
                &standalone
                    .play
                    .liquid
                    .as_ref()
                    .unwrap()
                    .body_rigid_states()
                    .next()
                    .map(|(_, state)| state),
                body
            );
        }
        standalone.stop_workers()?;
        Ok(())
    };
    replay(&path)?;
    std::fs::remove_dir_all(root)?;
    let package = packaged_game::MaterializedPackage::open(&package_path)?;
    replay(&package.project_path()?)?;
    Ok(())
}
