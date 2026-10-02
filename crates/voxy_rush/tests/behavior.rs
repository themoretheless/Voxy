use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
use voxy_rush::{ScriptManager, rush::StateValue};
use voxy_scene::{SceneGraph, Transform};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct File(PathBuf);
impl File {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "voxy-rush-{}-{}.r",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, source).unwrap();
        Self(path)
    }
    fn write(&self, source: &str) {
        std::fs::write(&self.0, source).unwrap();
    }
}
impl Drop for File {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
#[test]
fn instances_keep_independent_state_start_once_and_fixed_ticks() {
    let file = File::new(
        "mut n = 0\nfn start() { n += 10 }\nfn update(delta) { n += delta; set_position(self, [n,0,0]) }\nfn fixed_update(delta) { n += 1 }",
    );
    let mut scene = SceneGraph::new(4);
    let a = scene.spawn(None, Transform::default()).unwrap();
    let b = scene.spawn(None, Transform::default()).unwrap();
    let mut scripts = ScriptManager::new(4, 16);
    for id in [a, b] {
        scripts
            .attach(&mut scene, id, &file.0, vec!["n".into()])
            .unwrap();
    }
    scene.set_active(b, false).unwrap();
    scripts.fixed_update(&mut scene, 0.1).unwrap();
    scripts.update(&mut scene, 0.5).unwrap();
    scripts.update(&mut scene, 0.5).unwrap();
    assert_eq!(scripts.save(a).unwrap()["n"], StateValue::Number(12.0));
    assert_eq!(scripts.save(b).unwrap()["n"], StateValue::Number(0.0));
    scene.set_active(b, true).unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    assert_eq!(scripts.save(b).unwrap()["n"], StateValue::Number(11.0));
    assert!(scripts.diagnostics.is_empty());
}
#[test]
fn failed_handler_discards_all_commands_and_pauses_with_stack() {
    let file = File::new(
        "fn fail() { return 1 / 0 }\nfn update(delta) { set_position(self,[9,0,0]); spawn(\"lost\",[0,0,0]); fail() }",
    );
    let mut scene = SceneGraph::new(4);
    let id = scene.spawn(None, Transform::default()).unwrap();
    let mut scripts = ScriptManager::new(4, 16);
    scripts.attach(&mut scene, id, &file.0, vec![]).unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    assert_eq!(scene.local(id).unwrap(), Transform::default());
    assert_eq!(scene.len(), 1);
    assert!(scripts.paused(id));
    let diagnostic = &scripts.diagnostics[0];
    assert_eq!(diagnostic.file, file.0);
    assert_eq!(diagnostic.error.location.as_ref().unwrap().line, 1);
    assert!(!diagnostic.error.stack.is_empty());
    scripts.update(&mut scene, 1.0).unwrap();
    assert_eq!(scripts.diagnostics.len(), 1);
}
#[test]
fn reload_is_atomic_and_transfers_only_selected_state() {
    let file = File::new(
        "mut n = 0\nmut temporary = 0\nfn update(delta) { n += 1; temporary += 1; set_position(self,[n,temporary,0]) }",
    );
    let mut scene = SceneGraph::new(4);
    let id = scene.spawn(None, Transform::default()).unwrap();
    let mut scripts = ScriptManager::new(4, 16);
    scripts
        .attach(&mut scene, id, &file.0, vec!["n".into()])
        .unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    file.write("fn update( {");
    assert_eq!(scripts.reload_changed(&scene).len(), 1);
    scripts.update(&mut scene, 1.0).unwrap();
    assert_eq!(scene.local(id).unwrap().translation.x, 2.0);
    file.write("mut n = 100\nmut temporary = 0\nfn update(delta) { n += 10; temporary += 1; set_position(self,[n,temporary,0]) }");
    assert!(scripts.reload_changed(&scene).is_empty());
    scripts.update(&mut scene, 1.0).unwrap();
    assert_eq!(
        scene.local(id).unwrap().translation,
        glam::Vec3::new(12.0, 1.0, 0.0)
    );
    let saved = scripts.save(id).unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    scripts.restore(id, &saved).unwrap();
    assert_eq!(scripts.save(id).unwrap(), saved);
    file.write("const n = 1\nfn update(delta) { }");
    assert_eq!(scripts.reload_changed(&scene).len(), 1);
    scripts.update(&mut scene, 1.0).unwrap();
    assert_eq!(scene.local(id).unwrap().translation.x, 22.0);
}
#[test]
fn stale_handles_do_not_address_reused_slots_and_destroy_runs_once() {
    let file = File::new(
        "const target = find(\"target\")[0]\nfn update(delta) { set_position(target,[9,0,0]) }\nfn on_destroy() { spawn(\"farewell\",[0,0,0]) }",
    );
    let mut scene = SceneGraph::new(4);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    let target = scene.spawn(None, Transform::default()).unwrap();
    scene.set_name(target, "target").unwrap();
    let mut scripts = ScriptManager::new(4, 16);
    scripts.attach(&mut scene, owner, &file.0, vec![]).unwrap();
    scene.remove_subtree(target).unwrap();
    let replacement = scene.spawn(None, Transform::default()).unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    assert!(scripts.paused(owner));
    assert_eq!(scene.local(replacement).unwrap(), Transform::default());
    let file2 = File::new("fn on_destroy() { spawn(\"farewell\",[0,0,0]) }");
    let second = scene.spawn(None, Transform::default()).unwrap();
    scripts
        .attach(&mut scene, second, &file2.0, vec![])
        .unwrap();
    scene.remove_subtree(second).unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    assert_eq!(scene.find_named("farewell").count(), 1);
}
#[test]
fn foreign_scene_is_rejected_without_retiring_instances() {
    let file = File::new("mut n = 0\nfn update(delta) { n += 1 }");
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    let mut scripts = ScriptManager::new(1, 1);
    scripts
        .attach(&mut scene, owner, &file.0, vec!["n".into()])
        .unwrap();
    assert!(scripts.update(&mut SceneGraph::new(1), 1.0).is_err());
    scripts.update(&mut scene, 1.0).unwrap();
    assert_eq!(scripts.save(owner).unwrap()["n"], StateValue::Number(1.0));
}
#[test]
fn command_limit_is_transactional_and_initializers_cannot_write_scene() {
    let file =
        File::new("fn update(delta) { set_position(self,[1,0,0]); spawn(\"overflow\",[0,0,0]) }");
    let mut scene = SceneGraph::new(4);
    let id = scene.spawn(None, Transform::default()).unwrap();
    let mut scripts = ScriptManager::new(4, 1);
    scripts.attach(&mut scene, id, &file.0, vec![]).unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    assert_eq!(scene.local(id).unwrap(), Transform::default());
    assert_eq!(scene.len(), 1);
    assert!(scripts.paused(id));
    let initializer = File::new("set_position(self,[9,0,0])");
    let other = scene.spawn(None, Transform::default()).unwrap();
    assert!(
        scripts
            .attach(&mut scene, other, &initializer.0, vec![])
            .is_err()
    );
    assert_eq!(scene.local(other).unwrap(), Transform::default());
}
#[test]
fn selected_save_roundtrips_through_disk() {
    let file = File::new("mut n = 0\nfn update(delta) { n += 1 }");
    let save = File::new("");
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    let mut scripts = ScriptManager::new(1, 1);
    scripts
        .attach(&mut scene, owner, &file.0, vec!["n".into()])
        .unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    scripts.save_file(owner, &save.0).unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    scripts.restore_file(owner, &save.0).unwrap();
    assert_eq!(scripts.save(owner).unwrap()["n"], StateValue::Number(1.0));
}
#[cfg(feature = "physics-events")]
#[test]
fn physics_contacts_and_scene_channel_reach_handlers() {
    let file = File::new(
        "mut n = 0\nfn on_event(event) { if event.name == \"physics.contact\" { n += event.payload.normal[1] } else { n += 10 } }",
    );
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    let mut scripts = ScriptManager::new(1, 8);
    scripts
        .attach(&mut scene, owner, &file.0, vec!["n".into()])
        .unwrap();
    let step = physics::CharacterStep {
        requested_displacement: [0.0; 3],
        applied_displacement: [0.0; 3],
        contacts: vec![physics::CharacterContact {
            normal: [0, 1, 0],
            obstacle: 7u32,
        }],
        grounded: true,
        stepped_up: false,
    };
    scripts
        .character_events(&mut scene, owner, &step, |id| {
            StateValue::Number(*id as f64)
        })
        .unwrap();
    assert_eq!(scripts.save(owner).unwrap()["n"], StateValue::Number(11.0));
    let mut channel = voxy_scene::EventChannel::new(1).unwrap();
    let mut cursor = channel.subscribe(false);
    for name in ["lost", "retained"] {
        channel
            .emit(voxy_rush::ScriptEvent {
                target: Some(owner),
                name: name.into(),
                payload: StateValue::Null,
            })
            .unwrap();
    }
    assert_eq!(
        scripts
            .read_events(&mut scene, &channel, &mut cursor)
            .unwrap(),
        1
    );
    assert_eq!(scripts.save(owner).unwrap()["n"], StateValue::Number(21.0));
}
#[test]
fn script_removes_subtree_and_runs_destroy_hooks_once() {
    let parent_file = File::new(
        "fn update(delta) { destroy(self) }\nfn on_destroy() { spawn(\"parent-farewell\",[0,0,0]) }",
    );
    let child_file = File::new("fn on_destroy() { spawn(\"child-farewell\",[0,0,0]) }");
    let mut scene = SceneGraph::new(8);
    let parent = scene.spawn(None, Transform::default()).unwrap();
    let child = scene.spawn(Some(parent), Transform::default()).unwrap();
    let mut scripts = ScriptManager::new(8, 16);
    scripts
        .attach(&mut scene, parent, &parent_file.0, vec![])
        .unwrap();
    scripts
        .attach(&mut scene, child, &child_file.0, vec![])
        .unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    assert!(scene.local(parent).is_err());
    assert!(scene.local(child).is_err());
    assert_eq!(scene.find_named("parent-farewell").count(), 1);
    assert_eq!(scene.find_named("child-farewell").count(), 1);
    assert!(scripts.diagnostics.is_empty());
    assert!(scripts.command_errors.is_empty());
}
#[test]
fn apply_failure_rolls_back_successful_commands_and_pauses_handler() {
    let file = File::new(
        "fn update(delta) { set_position(self,[9,0,0]); spawn(\"first\",[0,0,0]); spawn(\"overflow\",[0,0,0]) }",
    );
    let mut scene = SceneGraph::new(2);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    scene
        .insert_component(owner, String::from("unique payload"))
        .unwrap();
    let mut scripts = ScriptManager::new(4, 16);
    scripts.attach(&mut scene, owner, &file.0, vec![]).unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    assert_eq!(scene.len(), 1);
    assert_eq!(scene.local(owner).unwrap(), Transform::default());
    assert_eq!(
        scene.component::<String>(owner).unwrap().unwrap(),
        "unique payload"
    );
    assert!(scripts.paused(owner));
    assert!(
        scripts.diagnostics[0]
            .to_string()
            .contains("Scene transaction rejected")
    );
    scripts.update(&mut scene, 1.0).unwrap();
    assert_eq!(scripts.diagnostics.len(), 1);
}
#[test]
fn failed_destruction_batch_does_not_dispatch_destroy_hooks() {
    let file = File::new(
        "fn update(delta) { destroy(self); set_position(self,[9,0,0]) }\nfn on_destroy() { spawn(\"farewell\",[0,0,0]) }",
    );
    let mut scene = SceneGraph::new(4);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    let mut scripts = ScriptManager::new(4, 16);
    scripts.attach(&mut scene, owner, &file.0, vec![]).unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    assert_eq!(scene.len(), 1);
    assert_eq!(scene.local(owner).unwrap(), Transform::default());
    assert_eq!(scene.find_named("farewell").count(), 0);
    assert!(scripts.paused(owner));
}
#[test]
fn successful_reload_resumes_paused_instance_and_keeps_selected_state() {
    let file = File::new("mut n = 0\nfn update(delta) { n += 1; return 1 / 0 }");
    let mut scene = SceneGraph::new(1);
    let owner = scene.spawn(None, Transform::default()).unwrap();
    let mut scripts = ScriptManager::new(1, 4);
    scripts
        .attach(&mut scene, owner, &file.0, vec!["n".into()])
        .unwrap();
    scripts.update(&mut scene, 1.0).unwrap();
    assert!(scripts.paused(owner));
    file.write("mut n = 99\nfn update(delta) { n += 1 }");
    assert!(scripts.reload_changed(&scene).is_empty());
    assert!(!scripts.paused(owner));
    scripts.update(&mut scene, 1.0).unwrap();
    assert_eq!(scripts.save(owner).unwrap()["n"], StateValue::Number(2.0));
}
