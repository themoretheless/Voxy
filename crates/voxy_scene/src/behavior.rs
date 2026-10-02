//! Deterministic Rust behavior lifecycle for scene objects.
use crate::{NodeId, SceneGraph, SceneGraphError};

/// Hooks execute in attachment order. Hooks may edit the scene; activity and handle
/// validity are checked again before start and update. Panics are not recovered.
pub trait Behavior: Send + std::fmt::Debug {
    fn awake(&mut self, _scene: &mut SceneGraph, _owner: NodeId) {}
    fn on_enable(&mut self, _scene: &mut SceneGraph, _owner: NodeId) {}
    fn start(&mut self, _scene: &mut SceneGraph, _owner: NodeId) {}
    fn update(&mut self, _scene: &mut SceneGraph, _owner: NodeId, _delta: f64) {}
    fn fixed_update(&mut self, _scene: &mut SceneGraph, _owner: NodeId, _delta: f64) {}
    fn on_disable(&mut self, _scene: &mut SceneGraph, _owner: NodeId) {}
    /// Owner may already be invalid when a subtree has been removed.
    fn on_destroy(&mut self, _scene: &mut SceneGraph, _owner: NodeId) {}
}

#[derive(Debug)]
struct Entry {
    owner: NodeId,
    behavior: Box<dyn Behavior>,
    enabled: bool,
    started: bool,
}

/// A scheduler owns behavior instances; `SceneGraph` owns data components.
/// Call sync after structural edits, update once per frame and `fixed_update` per
/// simulation tick. Multiple behaviors per node are supported. A scheduler must
/// be used with the scene it was attached to. Use clear before discarding it when
/// destroy callbacks are needed; Rust Drop alone does not dispatch callbacks.
#[derive(Debug, Default)]
pub struct BehaviorRunner {
    entries: Vec<Entry>,
}
impl BehaviorRunner {
    /// Awake runs immediately, including for inactive nodes. Enable/start are
    /// deferred until sync/update. Behaviors added by user code are not serialized.
    /// # Errors
    /// Rejects stale or foreign handles before calling awake.
    pub fn attach<B: Behavior + 'static>(
        &mut self,
        scene: &mut SceneGraph,
        owner: NodeId,
        mut behavior: B,
    ) -> Result<(), SceneGraphError> {
        scene.node(owner)?;
        behavior.awake(scene, owner);
        self.entries.push(Entry {
            owner,
            behavior: Box::new(behavior),
            enabled: false,
            started: false,
        });
        Ok(())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Observes activation and destruction in attachment order. Changes made by
    /// hooks are observed at the next dispatch boundary. Dead entries are retired.
    pub fn sync(&mut self, scene: &mut SceneGraph) {
        self.entries.retain_mut(|entry| {
            if let Ok(active) = scene.active_in_hierarchy(entry.owner) {
                if active != entry.enabled {
                    entry.enabled = active;
                    if active {
                        entry.behavior.on_enable(scene, entry.owner);
                    } else {
                        entry.behavior.on_disable(scene, entry.owner);
                    }
                }
                true
            } else {
                if entry.enabled {
                    entry.behavior.on_disable(scene, entry.owner);
                }
                entry.behavior.on_destroy(scene, entry.owner);
                false
            }
        });
    }

    fn dispatch(&mut self, scene: &mut SceneGraph, delta: f64, fixed: bool) {
        if !delta.is_finite() || delta < 0.0 {
            return;
        }
        self.sync(scene);
        for entry in &mut self.entries {
            if !entry.enabled || scene.active_in_hierarchy(entry.owner) != Ok(true) {
                continue;
            }
            if !entry.started {
                entry.started = true;
                entry.behavior.start(scene, entry.owner);
            }
            if scene.active_in_hierarchy(entry.owner) != Ok(true) {
                continue;
            }
            if fixed {
                entry.behavior.fixed_update(scene, entry.owner, delta);
            } else {
                entry.behavior.update(scene, entry.owner, delta);
            }
        }
        self.sync(scene);
    }

    /// Invalid deltas are ignored without invoking hooks.
    pub fn update(&mut self, scene: &mut SceneGraph, delta: f64) {
        self.dispatch(scene, delta, false);
    }
    /// Start precedes the first update of either kind and runs exactly once.
    pub fn fixed_update(&mut self, scene: &mut SceneGraph, delta: f64) {
        self.dispatch(scene, delta, true);
    }

    /// Dispatches disable/destroy for every owned behavior, then releases it.
    pub fn clear(&mut self, scene: &mut SceneGraph) {
        for mut entry in self.entries.drain(..) {
            if entry.enabled {
                entry.behavior.on_disable(scene, entry.owner);
            }
            entry.behavior.on_destroy(scene, entry.owner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Transform;
    use std::sync::{Arc, Mutex};
    #[derive(Debug)]
    struct Log(Arc<Mutex<Vec<&'static str>>>);
    impl Log {
        fn push(&self, event: &'static str) {
            self.0.lock().unwrap().push(event);
        }
    }
    impl Behavior for Log {
        fn awake(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.push("awake");
        }
        fn on_enable(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.push("enable");
        }
        fn start(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.push("start");
        }
        fn update(&mut self, _: &mut SceneGraph, _: NodeId, _: f64) {
            self.push("update");
        }
        fn fixed_update(&mut self, _: &mut SceneGraph, _: NodeId, _: f64) {
            self.push("fixed");
        }
        fn on_disable(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.push("disable");
        }
        fn on_destroy(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.push("destroy");
        }
    }
    #[test]
    fn parent_activation_start_once_and_destroy_order() {
        let mut scene = SceneGraph::new(2);
        let parent = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(parent), Transform::default()).unwrap();
        scene.set_active(parent, false).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut runner = BehaviorRunner::default();
        runner
            .attach(&mut scene, child, Log(events.clone()))
            .unwrap();
        runner.update(&mut scene, 0.1);
        assert_eq!(*events.lock().unwrap(), vec!["awake"]);
        scene.set_active(parent, true).unwrap();
        runner.fixed_update(&mut scene, 0.02);
        runner.update(&mut scene, 0.1);
        scene.set_active(parent, false).unwrap();
        runner.sync(&mut scene);
        scene.set_active(parent, true).unwrap();
        runner.update(&mut scene, 0.1);
        scene.remove_subtree(parent).unwrap();
        runner.sync(&mut scene);
        runner.sync(&mut scene);
        assert!(runner.is_empty());
        assert_eq!(
            *events.lock().unwrap(),
            vec![
                "awake", "enable", "start", "fixed", "update", "disable", "enable", "update",
                "disable", "destroy"
            ]
        );
    }
    #[derive(Debug)]
    struct DeleteOnStart;
    impl Behavior for DeleteOnStart {
        fn start(&mut self, scene: &mut SceneGraph, owner: NodeId) {
            scene.remove_subtree(owner).unwrap();
        }
        fn update(&mut self, _: &mut SceneGraph, _: NodeId, _: f64) {
            panic!("dead owner updated");
        }
    }
    #[test]
    fn deletion_in_callback_and_invalid_delta_are_safe() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let mut runner = BehaviorRunner::default();
        runner.attach(&mut scene, owner, DeleteOnStart).unwrap();
        runner.update(&mut scene, f64::NAN);
        assert_eq!(scene.len(), 1);
        runner.update(&mut scene, 0.1);
        assert!(scene.is_empty());
        assert!(runner.is_empty());
        assert_eq!(
            runner.attach(&mut scene, owner, DeleteOnStart),
            Err(SceneGraphError::InvalidNode)
        );
    }
    #[test]
    fn clear_dispatches_cleanup_once_for_active_and_inactive_behaviors() {
        let mut scene = SceneGraph::new(2);
        let active = scene.spawn(None, Transform::default()).unwrap();
        let inactive = scene.spawn(None, Transform::default()).unwrap();
        scene.set_active(inactive, false).unwrap();
        let a = Arc::new(Mutex::new(Vec::new()));
        let b = Arc::new(Mutex::new(Vec::new()));
        let mut runner = BehaviorRunner::default();
        runner.attach(&mut scene, active, Log(a.clone())).unwrap();
        runner.attach(&mut scene, inactive, Log(b.clone())).unwrap();
        runner.sync(&mut scene);
        runner.clear(&mut scene);
        runner.clear(&mut scene);
        assert_eq!(
            *a.lock().unwrap(),
            vec!["awake", "enable", "disable", "destroy"]
        );
        assert_eq!(*b.lock().unwrap(), vec!["awake", "destroy"]);
        assert!(runner.is_empty());
        assert_eq!(scene.len(), 2);
    }
}
