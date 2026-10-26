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
