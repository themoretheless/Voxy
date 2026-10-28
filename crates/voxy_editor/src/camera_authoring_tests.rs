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
        let root = std::env::temp_dir().join(format!(
            "voxy-camera-authoring-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn nested(&self) -> PathBuf {
        let root = self.0.join("nested");
        std::fs::create_dir(&root).unwrap();
        for entry in std::fs::read_dir(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/follow-camera-project"),
        )
        .unwrap()
        {
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
fn token(app: &App) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    Ok(camera_authoring::request_token(
        &app.authoring_document()?.objects[app.selected],
    )?)
}
fn choice(app: &App, target: &ObjectId) -> [u8; 32] {
    camera_authoring::choice_token(app.follow_pick.as_ref().unwrap().key, target)
}
#[test]
fn add_follow_camera_pick_edit_undo_remove_and_reopen_use_existing_authoring_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = project::create(&temp.0.join("new"), "Camera authoring")?;
    let mut app = App::from_project(&path)?;
    app.edit_key(KeyCode::KeyD)?;
    app.edit_key(KeyCode::KeyD)?;
    app.selected = 0;
    app.inspector = InspectorMode::Components(0);
    let before = app.authoring_document()?;
    let target = before.objects[1].id.clone();
    app.panel_action(panels::Action::FollowCamera(token(&app)?))?;
    assert!(app.follow_pick.is_some());
    assert_eq!(app.authoring_document()?, before);
    app.panel_action(panels::Action::FollowTargetChoice(choice(&app, &target)))?;
    let added = app.authoring_document()?;
    assert_eq!(
        added.objects[0].components["game.follow-camera.v1"]["target"],
        serde_json::to_value(&target)?
    );
    assert!(added.objects[0].components.contains_key("editor.camera.v1"));
    app.edit_key(KeyCode::KeyZ)?;
    assert_eq!(app.authoring_document()?, before);
    app.edit_key(KeyCode::KeyY)?;
    assert_eq!(app.authoring_document()?, added);
    let fields = component_fields::fields(&added.objects[0])?;
    let half = fields
        .iter()
        .position(|f| f.schema == "game.follow-camera.v1" && f.path == "/half_life")
        .unwrap();
    let target_field = fields
        .iter()
        .position(|f| f.schema == "game.follow-camera.v1" && f.path == "/target")
        .unwrap();
    app.inspector = InspectorMode::Components(half / 6);
    app.panel_action(panels::Action::Field(half))?;
    app.field_key(KeyCode::Digit3, Some("0.3"))?;
    app.field_key(KeyCode::Enter, None)?;
    let edited = app.authoring_document()?;
    assert_eq!(
        edited.objects[0].components["game.follow-camera.v1"]["half_life"],
        serde_json::json!(0.3)
    );
    assert!(app.edit_component_field(half, "-1").is_err());
    assert!(
        app.edit_component_field(target_field, "missing-target")
            .is_err()
    );
    assert_eq!(app.authoring_document()?, edited);
    app.save_authoring()?;
    app.selected = 1;
    assert!(app.edit_key(KeyCode::KeyG).is_err());
    assert_eq!(app.authoring_document()?, edited);
    let target_handle = app.instances[1];
    assert!(app.edit_key(KeyCode::Delete).is_err());
    assert!(app.scene.local(target_handle).is_ok());
    assert_eq!(app.authoring_document()?, edited);
    assert_eq!(app.authoring.history.as_ref().unwrap().current(), &edited);
    app.selected = 0;
    app.panel_action(panels::Action::FollowCamera(token(&app)?))?;
    let removed = app.authoring_document()?;
    assert!(
        !removed.objects[0]
            .components
            .contains_key("game.follow-camera.v1")
    );
    assert!(
        removed.objects[0]
            .components
            .contains_key("editor.camera.v1")
    );
    app.edit_key(KeyCode::KeyZ)?;
    assert_eq!(app.authoring_document()?, edited);
    app.stop_workers()?;
    let mut reopened = App::from_project(&path)?;
    assert_eq!(reopened.authoring_document()?, edited);
    reopened.edit_key(KeyCode::KeyD)?;
    let appended = reopened.authoring_document()?;
    assert_eq!(appended.objects.len(), 3);
    let unique: std::collections::BTreeSet<_> =
        appended.objects.iter().map(|o| o.id.clone()).collect();
    assert_eq!(unique.len(), 3);
    reopened.stop_workers()
}
#[test]
fn nested_target_picker_uses_stable_ids_and_saves_member_overrides()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = temp.nested();
    let mut app = App::from_project(&path)?;
    let original = app.authoring_document()?;
    let owner = original
        .objects
        .iter()
        .find(|o| o.name == "camera")
        .unwrap()
        .id
        .clone();
    let target = original
        .objects
        .iter()
        .find(|o| o.name == "ground")
        .unwrap()
        .id
        .clone();

    // The candidate token remains a durable ID even when the displayed rows move.
    let mut reordered = original.clone();
    reordered.objects.reverse();
    app.authoring
        .history
        .as_mut()
        .unwrap()
        .commit(reordered, &editor_component_registry()?)?;
    app.restore_authoring()?;
    app.selected = app.object_ids.iter().position(|id| id == &owner).unwrap();
    let document = app.authoring_document()?;
    let object = &document.objects[app.selected];
    let fields = component_fields::fields(object)?;
    let index = fields
        .iter()
        .position(|f| f.schema == "game.follow-camera.v1" && f.path == "/target")
        .unwrap();
    app.inspector = InspectorMode::Components(index / 6);
    let request = token(&app)?;
    let mut panel = panels::Panels::new()?;
    panel.build(
        &document,
        app.selected,
        Vec2::new(1280., 720.),
        false,
        None,
        0,
        false,
        app.inspector,
        "",
        "",
        None,
    )?;
    assert!(
        panel
            .regions
            .iter()
            .any(|(_, action)| *action == panels::Action::FollowTarget(request))
    );
    assert!(
        panel
            .regions
            .iter()
            .any(|(_, action)| *action == panels::Action::FollowCamera(request))
    );
    app.panel_action(panels::Action::FollowTarget(request))?;
    let candidate = choice(&app, &target);
    panel.follow_pick = app.follow_pick.as_ref().map(|pick| pick.key);
    panel.build(
        &document,
        app.selected,
        Vec2::new(1280., 720.),
        false,
        None,
        0,
        false,
        app.inspector,
        "",
        "",
        None,
    )?;
    assert!(
        panel
            .regions
            .iter()
            .any(|(_, action)| *action == panels::Action::FollowTargetChoice(candidate))
    );
    app.panel_action(panels::Action::FollowTargetChoice(candidate))?;
    let edited = app.authoring_document()?;
    assert_eq!(
        edited
            .objects
            .iter()
            .find(|o| o.id == owner)
            .unwrap()
            .components["game.follow-camera.v1"]["target"],
        serde_json::to_value(&target)?
    );
    app.save_authoring()?;
    let saved: voxy_scene::PrefabSceneDocument = serde_json::from_slice(&std::fs::read(
        path.parent().unwrap().join("main.scene.json"),
    )?)?;
    assert!(saved.objects.is_empty());
    assert_eq!(saved.instances.len(), 1);
    assert!(saved.instances[0].overrides.values().any(|change| {
        change
            .component_members
            .get("game.follow-camera.v1")
            .is_some_and(|members| {
                members.get("/target")
                    == Some(
                        &serde_json::to_value(voxy_scene::instance_object_id(
                            &ObjectId("world".into()),
                            &ObjectId("ground".into()),
                        ))
                        .unwrap(),
                    )
            })
    }));
    app.stop_workers()?;
    let mut reopened = App::from_project(&path)?;
    let document = reopened.authoring_document()?;
    assert_eq!(
        document
            .objects
            .iter()
            .find(|o| o.id == owner)
            .unwrap()
            .components["game.follow-camera.v1"]["target"],
        serde_json::to_value(&target)?
    );
    reopened.selected = document.objects.iter().position(|o| o.id == owner).unwrap();
    let fields = component_fields::fields(&document.objects[reopened.selected])?;
    let target_index = fields
        .iter()
        .position(|field| field.schema == "game.follow-camera.v1" && field.path == "/target")
        .unwrap();
    reopened.inspector = InspectorMode::Components(target_index / 6);
    assert!(
        reopened
            .prefab_overridden_fields(&document)?
            .contains(&target_index)
    );
    reopened.panel_action(panels::Action::ResetField(target_index))?;
    let reset = reopened.authoring_document()?;
    let source_target = original
        .objects
        .iter()
        .find(|o| o.name == "player")
        .unwrap()
        .id
        .clone();
    assert_eq!(
        reset
            .objects
            .iter()
            .find(|o| o.id == owner)
            .unwrap()
            .components["game.follow-camera.v1"]["target"],
        serde_json::to_value(source_target)?
    );
    reopened.edit_key(KeyCode::KeyZ)?;
    assert_eq!(reopened.authoring_document()?, document);
    reopened.stop_workers()
}
#[test]
fn stale_and_canceled_camera_pick_actions_never_publish_a_change()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = Temp::new();
    let path = temp.nested();
    let mut app = App::from_project(&path)?;
    let doc = app.authoring_document()?;
    app.selected = doc.objects.iter().position(|o| o.name == "camera").unwrap();
    let target_id = doc
        .objects
        .iter()
        .find(|o| o.name == "ground")
        .unwrap()
        .id
        .clone();
    let request = token(&app)?;
    app.panel_action(panels::Action::FollowTarget(request))?;
    let old_candidate = choice(&app, &target_id);
    app.edit_key(KeyCode::Escape)?;
    assert!(app.follow_pick.is_none());
    assert!(
        app.panel_action(panels::Action::FollowTargetChoice(old_candidate))
            .is_err()
    );
    assert_eq!(app.authoring_document()?, doc);
    app.panel_action(panels::Action::FollowTarget(request))?;
    let candidate = choice(&app, &target_id);
    assert_ne!(candidate, old_candidate);
    assert!(
        app.panel_action(panels::Action::FollowTargetChoice(old_candidate))
            .is_err()
    );
    assert!(app.follow_pick.is_some());
    assert_eq!(app.authoring_document()?, doc);
    app.scene
        .component_mut::<voxy_gameplay::FollowCamera>(app.instances[app.selected])?
        .unwrap()
        .half_life = 0.4;
    let external = app.authoring_document()?;
    assert!(
        app.panel_action(panels::Action::FollowTargetChoice(candidate))
            .is_err()
    );
    assert_eq!(app.authoring_document()?, external);
    assert!(
        app.panel_action(panels::Action::FollowTarget(request))
            .is_err()
    );
    assert_eq!(app.authoring_document()?, external);
    app.restore_authoring()?;
    app.panel_action(panels::Action::FollowTarget(token(&app)?))?;
    app.toggle_play()?;
    assert!(app.follow_pick.is_none());
    app.panel_action(panels::Action::FollowTargetChoice(candidate))?;
    app.toggle_play()?;
    assert_eq!(app.authoring_document()?, doc);
    app.stop_workers()
}
