use crate::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "voxy-follow-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn project(&self) -> PathBuf {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/follow-camera-project");
        let root = self.0.join("source");
        std::fs::create_dir(&root).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                std::fs::copy(entry.path(), root.join(entry.file_name())).unwrap();
            }
        }
        root.join("project.voxy")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn host() -> Result<GameHost, Box<dyn std::error::Error>> {
    Ok(GameHost::new(editor_component_registry()?))
}
#[test]
fn nested_follow_camera_uses_interpolated_poses_and_matches_source_free_package()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = temp.project();
    let expanded = project::open(&path, &editor_component_registry()?)?.scene;
    let player = expanded
        .objects
        .iter()
        .find(|o| o.name == "player")
        .unwrap();
    let camera = expanded
        .objects
        .iter()
        .find(|o| o.name == "camera")
        .unwrap();
    assert_eq!(
        camera.components["game.follow-camera.v1"]["target"],
        serde_json::to_value(&player.id)?
    );
    assert_ne!(player.id.0, "player");
    let mut editor = App::from_project(&path)?;
    let authored = editor.authoring_document()?;
    let initial_camera = serde_json::to_value(&editor.camera)?;
    editor.toggle_play()?;
    editor.game_key(KeyCode::ArrowRight, ElementState::Pressed)?;
    let target_node = editor
        .scene
        .components::<voxy_gameplay::CharacterBody>()
        .next()
        .unwrap()
        .0;
    let config = editor
        .scene
        .components::<voxy_gameplay::FollowCamera>()
        .next()
        .unwrap()
        .1
        .clone();
    let initial_target = editor
        .play
        .simulation
        .as_ref()
        .unwrap()
        .render_world(&editor.scene, target_node)?
        .w_axis
        .truncate()
        .as_dvec3()
        + Vec3::from_array(config.offset).as_dvec3();
    let mut oracle = voxy_gameplay::FollowSmoother::new(initial_target)?;
    let mut trace = Vec::new();
    let mut render_only_motion = 0;
    let mut previous = editor.camera.target;
    let mut previous_ticks = editor.play.simulation_ticks;
    for _ in 0..240 {
        editor.advance_game(1. / 240.)?;
        let rendered_target = editor
            .play
            .simulation
            .as_ref()
            .unwrap()
            .render_world(&editor.scene, target_node)?
            .w_axis
            .truncate()
            .as_dvec3()
            + Vec3::from_array(config.offset).as_dvec3();
        assert_eq!(
            editor.camera.target,
            oracle
                .advance(rendered_target, config.half_life, 1. / 240.)?
                .as_vec3()
        );
        if editor.play.simulation_ticks == previous_ticks && editor.camera.target.x > previous.x {
            render_only_motion += 1;
        }
        assert!(editor.camera.target.x >= previous.x - 1e-7);
        assert!(editor.camera.target.x <= 0.801);
        assert!(editor.camera.matrix(Vec2::new(1280., 720.))?.is_finite());
        trace.push(editor.camera.target);
        previous = editor.camera.target;
        previous_ticks = editor.play.simulation_ticks;
    }
    assert!(
        render_only_motion > 100,
        "camera stayed fixed between physics ticks"
    );
    assert!(editor.camera.target.x > 0.5 && editor.camera.target.x < 0.8);
    editor.toggle_play()?;
    assert!(editor.play.follow_camera.is_none());
    assert_eq!(editor.authoring_document()?, authored);
    assert_eq!(serde_json::to_value(&editor.camera)?, initial_camera);
    editor.toggle_play()?;
    assert_eq!(editor.camera.target.x, 0.);
    editor.toggle_play()?;
    editor.stop_workers()?;
    let output = temp.0.join("camera.vpak");
    export_project_package_with_host(&path, &output, host()?)?;
    std::fs::remove_dir_all(path.parent().unwrap())?;
    let mut package = HeadlessGame::open_package(&output, host()?)?;
    package.key(KeyCode::ArrowRight, true)?;
    for expected in trace {
        package.advance(1. / 240.)?;
        assert_eq!(package.presentation_camera().target, expected);
    }
    package.stop()
}
#[test]
fn missing_target_and_ambiguous_camera_retain_view_and_start_failure_restores_authoring()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = temp.project();
    let mut app = App::from_project(&path)?;
    let authored = app.authoring_document()?;
    app.toggle_play()?;
    let target = app
        .scene
        .components::<voxy_gameplay::CharacterBody>()
        .next()
        .unwrap()
        .0;
    app.scene.remove_subtree(target)?;
    let retained = serde_json::to_value(&app.camera)?;
    assert!(
        app.advance_game(0.)
            .unwrap_err()
            .to_string()
            .contains("follow camera target")
    );
    assert_eq!(serde_json::to_value(&app.camera)?, retained);
    app.toggle_play()?;
    assert_eq!(app.authoring_document()?, authored);
    let (owner, descriptor) = app
        .scene
        .components::<voxy_gameplay::FollowCamera>()
        .map(|(n, c)| (n, c.clone()))
        .next()
        .unwrap();
    let view = app.scene.component::<EditorCamera>(owner)?.unwrap().clone();
    let second = app.scene.spawn(None, Transform::default())?;
    app.scene.insert_component(second, descriptor)?;
    app.scene.insert_component(second, view)?;
    app.instances.push(second);
    app.object_ids.push(ObjectId("second-camera".into()));
    app.commit_authoring()?;
    let before = app.authoring_document()?;
    assert!(
        app.toggle_play()
            .unwrap_err()
            .to_string()
            .contains("multiple active follow cameras")
    );
    assert!(
        app.play.playing.is_none()
            && app.play.simulation.is_none()
            && app.play.follow_camera.is_none()
    );
    assert_eq!(app.authoring_document()?, before);
    app.stop_workers()
}

#[test]
fn follow_checkpoint_rebinds_fresh_scene_and_preserves_camera_continuation()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = temp.project();
    let mut original = App::from_project(&path)?;
    let mut loaded = App::from_project(&path)?;
    original.toggle_play()?;
    loaded.toggle_play()?;
    original.game_key(KeyCode::ArrowRight, ElementState::Pressed)?;
    loaded.game_key(KeyCode::ArrowRight, ElementState::Pressed)?;
    for _ in 0..37 {
        original.advance_game(0.019)?;
        loaded.advance_game(0.019)?;
    }
    let saved =
        original
            .play
            .follow_camera
            .as_ref()
            .unwrap()
            .checkpoint(&original.camera, |node| {
                original
                    .instances
                    .iter()
                    .position(|n| *n == node)
                    .map(|i| original.object_ids[i].clone())
            })?;
    let json = serde_json::to_string(&saved)?;
    let decoded: crate::follow_camera::FollowRuntimeCheckpoint = serde_json::from_str(&json)?;
    let resolve = |id: &ObjectId| {
        loaded
            .object_ids
            .iter()
            .position(|i| i == id)
            .map(|i| loaded.instances[i])
    };
    assert!(decoded.prepare(&loaded.scene, |_| None).is_err());
    let (runtime, view) = decoded.prepare(&loaded.scene, resolve)?;
    loaded.play.follow_camera = Some(runtime);
    loaded.camera = view;
    for frame in 0..120 {
        let dt = [0., 0.003, 0.021, 0.037][frame % 4];
        original.advance_game(dt)?;
        loaded.advance_game(dt)?;
        assert_eq!(
            serde_json::to_string(&original.camera)?,
            serde_json::to_string(&loaded.camera)?
        );
        let checkpoint =
            loaded
                .play
                .follow_camera
                .as_ref()
                .unwrap()
                .checkpoint(&loaded.camera, |node| {
                    loaded
                        .instances
                        .iter()
                        .position(|n| *n == node)
                        .map(|i| loaded.object_ids[i].clone())
                })?;
        let expected =
            original
                .play
                .follow_camera
                .as_ref()
                .unwrap()
                .checkpoint(&original.camera, |node| {
                    original
                        .instances
                        .iter()
                        .position(|n| *n == node)
                        .map(|i| original.object_ids[i].clone())
                })?;
        assert_eq!(
            serde_json::to_string(&checkpoint)?,
            serde_json::to_string(&expected)?
        );
    }
    original.stop_workers()?;
    loaded.stop_workers()
}
