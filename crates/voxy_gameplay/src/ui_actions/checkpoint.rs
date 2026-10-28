//! Explicit opt-in UI callback state; arbitrary captured closures remain unsupported.
use super::*;
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use voxy_scene::{NodeId, ObjectId};

/// A versioned application schema and exact opaque state bytes.
/// Applications own migrations and exact numeric encoding inside the payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiActionState {
    schema: String,
    payload: Vec<u8>,
}
impl UiActionState {
    pub fn new(schema: String, payload: Vec<u8>) -> Result<Self, GameplayError> {
        if schema.is_empty() || schema.len() > 1024 || schema.contains('\0') {
            return Err("invalid UI action codec name".into());
        }
        Ok(Self { schema, payload })
    }
}
/// Stateful application callback. Checkpoint capture must be read-only and encode
/// every private field, replacing stored scene handles through `identify`.
pub trait UiActionHandler: Send {
    fn dispatch(
        &mut self,
        scene: &SceneGraph,
        event: &UiActionEvent,
        commands: &mut SceneCommands,
    ) -> Result<(), GameplayError>;
    fn checkpoint(
        &self,
        identify: &dyn Fn(NodeId) -> Option<ObjectId>,
    ) -> Result<UiActionState, GameplayError>;
}
#[derive(Clone, Copy, Debug)]
pub struct UiActionCheckpointLimits {
    pub max_events: usize,
    pub max_handlers: usize,
    pub max_commands_per_event: usize,
    pub max_bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiActionCheckpoint {
    version: u32,
    event_capacity: usize,
    handler_capacity: usize,
    command_capacity: usize,
    handlers: Vec<(String, UiActionState)>,
    events: Vec<(ObjectId, String)>,
}
/// Read-only identity resolver for new callback instances. Factories must resolve
/// saved references here; raw handles from the source process must not be reused.
pub struct UiActionRestoreContext<'a> {
    scene: &'a SceneGraph,
    resolve: &'a dyn Fn(&ObjectId) -> Option<NodeId>,
    identities: RefCell<BTreeMap<NodeId, ObjectId>>,
    nodes: RefCell<BTreeMap<ObjectId, NodeId>>,
    failed: Cell<bool>,
}
impl std::fmt::Debug for UiActionRestoreContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiActionRestoreContext")
            .finish_non_exhaustive()
    }
}
impl UiActionRestoreContext<'_> {
    pub fn scene(&self) -> &SceneGraph {
        self.scene
    }
    pub fn resolve(&self, id: &ObjectId) -> Result<NodeId, GameplayError> {
        let result = self.resolve_checked(id);
        if result.is_err() {
            self.failed.set(true);
        }
        result
    }
    fn resolve_checked(&self, id: &ObjectId) -> Result<NodeId, GameplayError> {
        if id.0.is_empty() {
            return Err("empty UI object identity".into());
        }
        let node = (self.resolve)(id).ok_or("missing UI checkpoint object")?;
        self.scene.local(node)?;
        if self
            .identities
            .borrow_mut()
            .insert(node, id.clone())
            .is_some_and(|old| old != *id)
            || self
                .nodes
                .borrow_mut()
                .insert(id.clone(), node)
                .is_some_and(|old| old != node)
        {
            return Err("ambiguous UI checkpoint object".into());
        }
        Ok(node)
    }
}
type Factory = dyn Fn(&[u8], &UiActionRestoreContext<'_>) -> Result<Box<dyn UiActionHandler>, GameplayError>
    + Send
    + Sync;
#[derive(Default)]
pub struct UiActionCheckpointRegistry {
    factories: BTreeMap<String, Box<Factory>>,
}
impl std::fmt::Debug for UiActionCheckpointRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiActionCheckpointRegistry")
            .field("codecs", &self.factories.len())
            .finish()
    }
}
impl UiActionCheckpointRegistry {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }
    pub fn schemas(&self) -> impl Iterator<Item = &str> {
        self.factories.keys().map(String::as_str)
    }
    /// Factories prepare independent callbacks without dispatching events or
    /// mutating external live state; arbitrary side effects cannot be rolled back.
    pub fn register(
        &mut self,
        schema: &str,
        factory: impl Fn(
            &[u8],
            &UiActionRestoreContext<'_>,
        ) -> Result<Box<dyn UiActionHandler>, GameplayError>
        + Send
        + Sync
        + 'static,
    ) -> Result<(), GameplayError> {
        UiActionState::new(schema.into(), Vec::new())?;
        if self.factories.contains_key(schema) {
            return Err("duplicate UI action codec".into());
        }
        self.factories.insert(schema.into(), Box::new(factory));
        Ok(())
    }
    pub fn prepare(
        &self,
        scene: &SceneGraph,
        saved: &UiActionCheckpoint,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        limits: UiActionCheckpointLimits,
    ) -> Result<UiActionHandlers, GameplayError> {
        saved.validate(limits)?;
        // Missing codecs reject before invoking any factory.
        for (_, state) in &saved.handlers {
            if !self.factories.contains_key(&state.schema) {
                return Err(format!("unsupported UI action codec: {}", state.schema).into());
            }
        }
        let context = UiActionRestoreContext {
            scene,
            resolve: &resolve,
            identities: RefCell::new(BTreeMap::new()),
            nodes: RefCell::new(BTreeMap::new()),
            failed: Cell::new(false),
        };
        let events = saved
            .events
            .iter()
            .map(|(id, action)| {
                Ok(UiActionEvent {
                    owner: context.resolve(id)?,
                    action: action.clone(),
                })
            })
            .collect::<Result<VecDeque<_>, GameplayError>>()?;
        let mut handlers = BTreeMap::new();
        for (action, state) in &saved.handlers {
            let handler = self.factories[&state.schema](&state.payload, &context)?;
            if context.failed.get() {
                return Err("UI factory ignored an invalid object reference".into());
            }
            let recovered =
                handler.checkpoint(&|node| context.identities.borrow().get(&node).cloned())?;
            if recovered != *state {
                return Err("UI callback factory changed saved state".into());
            }
            handlers.insert(action.clone(), Handler::Checkpointed(handler));
        }
        // Preserve accepted pending events, including targets that expired after
        // enqueue. Normal dispatch performs their validity/error checks once.
        Ok(UiActionHandlers {
            scene: scene.identity(),
            events,
            handlers,
            event_capacity: saved.event_capacity,
            handler_capacity: saved.handler_capacity,
            command_capacity: saved.command_capacity,
        })
    }
}
impl UiActionHandlers {
    pub fn register_checkpointable(
        &mut self,
        action: String,
        handler: impl UiActionHandler + 'static,
    ) -> Result<(), GameplayError> {
        if !valid_action_name(&action) {
            return Err("invalid UI action name".into());
        }
        if self.handlers.contains_key(&action) {
            return Err("duplicate UI action handler".into());
        }
        if self.handlers.len() >= self.handler_capacity {
            return Err("UI handler capacity exceeded".into());
        }
        self.handlers
            .insert(action, Handler::Checkpointed(Box::new(handler)));
        Ok(())
    }
    pub fn checkpoint(
        &self,
        scene: &SceneGraph,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        limits: UiActionCheckpointLimits,
    ) -> Result<UiActionCheckpoint, GameplayError> {
        let Self {
            scene: owner,
            events,
            handlers,
            event_capacity,
            handler_capacity,
            command_capacity,
        } = self;
        if *owner != scene.identity() {
            return Err("foreign UI checkpoint scene".into());
        }
        let ids = RefCell::new(BTreeMap::new());
        let nodes = RefCell::new(BTreeMap::new());
        let failed = Cell::new(false);
        let identify = |node| {
            if scene.local(node).is_err() {
                failed.set(true);
                return None;
            }
            let Some(id) = identify(node).filter(|id| !id.0.is_empty()) else {
                failed.set(true);
                return None;
            };
            if ids
                .borrow_mut()
                .insert(node, id.clone())
                .is_some_and(|old| old != id)
                || nodes
                    .borrow_mut()
                    .insert(id.clone(), node)
                    .is_some_and(|old| old != node)
            {
                failed.set(true);
                return None;
            }
            Some(id)
        };
        let handlers = handlers
            .iter()
            .map(|(name, handler)| {
                let Handler::Checkpointed(handler) = handler else {
                    return Err(format!("UI action does not support checkpoints: {name}").into());
                };
                Ok((name.clone(), handler.checkpoint(&identify)?))
            })
            .collect::<Result<Vec<_>, GameplayError>>()?;
        let events = events
            .iter()
            .map(|event| {
                Ok((
                    identify(event.owner).ok_or("unidentified UI event owner")?,
                    event.action.clone(),
                ))
            })
            .collect::<Result<Vec<_>, GameplayError>>()?;
        if failed.get() {
            return Err("UI handler ignored an invalid capture reference".into());
        }
        let saved = UiActionCheckpoint {
            version: 1,
            event_capacity: *event_capacity,
            handler_capacity: *handler_capacity,
            command_capacity: *command_capacity,
            handlers,
            events,
        };
        saved.validate(limits)?;
        Ok(saved)
    }
}
impl UiActionCheckpoint {
    fn validate(&self, limits: UiActionCheckpointLimits) -> Result<(), GameplayError> {
        if self.version != 1
            || self.event_capacity > limits.max_events
            || self.handler_capacity > limits.max_handlers
            || self.command_capacity > limits.max_commands_per_event
            || self.events.len() > self.event_capacity
            || self.handlers.len() > self.handler_capacity
        {
            return Err("UI checkpoint version or capacity".into());
        }
        let mut previous = None;
        for (name, state) in &self.handlers {
            if !valid_action_name(name) || previous.is_some_and(|old| old >= name) {
                return Err("invalid or duplicate UI action identity".into());
            }
            UiActionState::new(state.schema.clone(), Vec::new())?;
            previous = Some(name);
        }
        if self
            .events
            .iter()
            .any(|(id, action)| id.0.is_empty() || !valid_action_name(action))
        {
            return Err("invalid UI queued event".into());
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > limits.max_bytes {
            return Err("UI checkpoint byte budget".into());
        }
        Ok(())
    }
    pub fn to_json(&self, limits: UiActionCheckpointLimits) -> Result<String, GameplayError> {
        self.validate(limits)?;
        serde_json::to_string(self).map_err(|e| e.to_string().into())
    }
    /// Bound file reads before constructing the input string.
    pub fn from_json(text: &str, limits: UiActionCheckpointLimits) -> Result<Self, GameplayError> {
        if text.len() > limits.max_bytes {
            return Err("UI checkpoint byte budget".into());
        }
        let saved: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        saved.validate(limits)?;
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use voxy_scene::{SceneCommand, Transform};
    fn limits() -> UiActionCheckpointLimits {
        UiActionCheckpointLimits {
            max_events: 8,
            max_handlers: 4,
            max_commands_per_event: 4,
            max_bytes: 100000,
        }
    }
    #[derive(Serialize, Deserialize)]
    struct SavedCounter {
        calls: u64,
        accumulated: u64,
        target: ObjectId,
    }
    struct Counter {
        calls: u64,
        accumulated: f64,
        target: NodeId,
    }
    impl UiActionHandler for Counter {
        fn dispatch(
            &mut self,
            scene: &SceneGraph,
            _: &UiActionEvent,
            commands: &mut SceneCommands,
        ) -> Result<(), GameplayError> {
            self.calls += 1;
            self.accumulated += 1e-19;
            let mut pose = scene.local(self.target)?;
            pose.translation.x = self.calls as f32;
            commands.push(SceneCommand::SetLocal(self.target, pose))?;
            if self.calls.is_multiple_of(17) {
                return Err("intentional callback failure".into());
            }
            Ok(())
        }
        fn checkpoint(
            &self,
            identify: &dyn Fn(NodeId) -> Option<ObjectId>,
        ) -> Result<UiActionState, GameplayError> {
            UiActionState::new(
                "counter.v1".into(),
                serde_json::to_vec(&SavedCounter {
                    calls: self.calls,
                    accumulated: self.accumulated.to_bits(),
                    target: identify(self.target).ok_or("missing counter target")?,
                })
                .map_err(|e| e.to_string())?,
            )
        }
    }
    fn registry() -> UiActionCheckpointRegistry {
        let mut registry = UiActionCheckpointRegistry::default();
        registry
            .register("counter.v1", |payload, context| {
                let saved: SavedCounter =
                    serde_json::from_slice(payload).map_err(|e| e.to_string())?;
                Ok(Box::new(Counter {
                    calls: saved.calls,
                    accumulated: f64::from_bits(saved.accumulated),
                    target: context.resolve(&saved.target)?,
                }))
            })
            .unwrap();
        registry
    }
    fn fixture(reverse: bool) -> (SceneGraph, BTreeMap<ObjectId, NodeId>) {
        let mut scene = SceneGraph::new(4);
        let mut ids = BTreeMap::new();
        for name in if reverse {
            ["target", "button"]
        } else {
            ["button", "target"]
        } {
            let node = scene.spawn(None, Transform::default()).unwrap();
            ids.insert(ObjectId(name.into()), node);
            if name == "button" {
                scene
                    .insert_component(
                        node,
                        crate::UiElement {
                            origin: [0.; 2],
                            size: [1.; 2],
                            color: [1.; 4],
                            layer: 0,
                            enabled: true,
                            action: Some("counter".into()),
                            text: None,
                        },
                    )
                    .unwrap();
            }
        }
        (scene, ids)
    }
    fn identify(ids: &BTreeMap<ObjectId, NodeId>, node: NodeId) -> Option<ObjectId> {
        ids.iter()
            .find(|(_, n)| **n == node)
            .map(|(id, _)| id.clone())
    }
    fn event(ids: &BTreeMap<ObjectId, NodeId>) -> UiActionEvent {
        UiActionEvent {
            owner: ids[&ObjectId("button".into())],
            action: "counter".into(),
        }
    }
    fn dispatch(actions: &mut UiActionHandlers, scene: &mut SceneGraph) -> Vec<bool> {
        let mut commands = SceneCommands::new(scene, 8);
        let result = actions
            .dispatch(scene, &mut commands)
            .unwrap()
            .into_iter()
            .map(|r| r.is_ok())
            .collect();
        for r in commands.apply(scene).unwrap() {
            r.unwrap();
        }
        result
    }
    #[test]
    fn callback_state_and_pending_events_rebind_and_continue_120_barriers() {
        let (mut scene, ids) = fixture(false);
        let mut actions = UiActionHandlers::new(&scene, 8, 4, 4);
        actions
            .register_checkpointable(
                "counter".into(),
                Counter {
                    calls: 0,
                    accumulated: -0.,
                    target: ids[&ObjectId("target".into())],
                },
            )
            .unwrap();
        for _ in 0..23 {
            actions.enqueue(&scene, event(&ids)).unwrap();
            dispatch(&mut actions, &mut scene);
        }
        for _ in 0..3 {
            actions.enqueue(&scene, event(&ids)).unwrap();
        }
        let saved = actions
            .checkpoint(&scene, |node| identify(&ids, node), limits())
            .unwrap();
        let json = saved.to_json(limits()).unwrap();
        let saved = UiActionCheckpoint::from_json(&json, limits()).unwrap();
        let (mut other, other_ids) = fixture(true);
        let target = ids[&ObjectId("target".into())];
        let new_target = other_ids[&ObjectId("target".into())];
        other
            .set_local(new_target, scene.local(target).unwrap())
            .unwrap();
        // Target expiry occurs after enqueue but before capture/restore dispatch.
        scene.set_active(event(&ids).owner, false).unwrap();
        other.set_active(event(&other_ids).owner, false).unwrap();
        let mut resumed = registry()
            .prepare(&other, &saved, |id| other_ids.get(id).copied(), limits())
            .unwrap();
        assert_eq!(
            actions
                .checkpoint(&scene, |n| identify(&ids, n), limits())
                .unwrap(),
            resumed
                .checkpoint(&other, |n| identify(&other_ids, n), limits())
                .unwrap()
        );
        assert_eq!(dispatch(&mut actions, &mut scene), vec![false; 3]);
        assert_eq!(dispatch(&mut resumed, &mut other), vec![false; 3]);
        scene.set_active(event(&ids).owner, true).unwrap();
        other.set_active(event(&other_ids).owner, true).unwrap();
        for _ in 0..120 {
            actions.enqueue(&scene, event(&ids)).unwrap();
            resumed.enqueue(&other, event(&other_ids)).unwrap();
            assert_eq!(
                dispatch(&mut actions, &mut scene),
                dispatch(&mut resumed, &mut other)
            );
            assert_eq!(
                scene.local(target).unwrap(),
                other.local(new_target).unwrap()
            );
            assert_eq!(
                actions
                    .checkpoint(&scene, |n| identify(&ids, n), limits())
                    .unwrap(),
                resumed
                    .checkpoint(&other, |n| identify(&other_ids, n), limits())
                    .unwrap()
            );
        }
        assert!(
            actions
                .dispatch(&other, &mut SceneCommands::new(&other, 1))
                .is_err()
        );
    }
    #[test]
    fn legacy_callbacks_reject_capture_without_consuming_pending_work() {
        let (mut scene, ids) = fixture(false);
        let mut actions = UiActionHandlers::new(&scene, 8, 4, 4);
        let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let shared = count.clone();
        actions
            .register("counter".into(), move |_, _, _| {
                shared.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(())
            })
            .unwrap();
        actions.enqueue(&scene, event(&ids)).unwrap();
        assert!(
            actions
                .checkpoint(&scene, |n| identify(&ids, n), limits())
                .is_err()
        );
        assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert_eq!(dispatch(&mut actions, &mut scene), vec![true]);
        assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
    #[test]
    fn malformed_codecs_limits_and_failed_preparation_preserve_live_owner() {
        let (scene, ids) = fixture(false);
        let mut actions = UiActionHandlers::new(&scene, 8, 4, 4);
        actions
            .register_checkpointable(
                "counter".into(),
                Counter {
                    calls: 23,
                    accumulated: 1e-18,
                    target: ids[&ObjectId("target".into())],
                },
            )
            .unwrap();
        actions.enqueue(&scene, event(&ids)).unwrap();
        let saved = actions
            .checkpoint(&scene, |n| identify(&ids, n), limits())
            .unwrap();
        let (other, other_ids) = fixture(true);
        assert!(
            UiActionCheckpointRegistry::default()
                .prepare(&other, &saved, |id| other_ids.get(id).copied(), limits())
                .is_err()
        );
        assert!(
            registry()
                .prepare(&other, &saved, |_| None, limits())
                .is_err()
        );
        assert!(
            registry()
                .prepare(
                    &other,
                    &saved,
                    |_| Some(other_ids[&ObjectId("button".into())]),
                    limits()
                )
                .is_err()
        );
        for policy in [
            UiActionCheckpointLimits {
                max_events: 0,
                ..limits()
            },
            UiActionCheckpointLimits {
                max_handlers: 0,
                ..limits()
            },
            UiActionCheckpointLimits {
                max_commands_per_event: 0,
                ..limits()
            },
            UiActionCheckpointLimits {
                max_bytes: 1,
                ..limits()
            },
        ] {
            assert!(
                registry()
                    .prepare(&other, &saved, |id| other_ids.get(id).copied(), policy)
                    .is_err()
            );
        }
        let json = saved.to_json(limits()).unwrap();
        assert!(
            UiActionCheckpoint::from_json(
                &json,
                UiActionCheckpointLimits {
                    max_bytes: json.len() - 1,
                    ..limits()
                }
            )
            .is_err()
        );
        let mut bad = saved.clone();
        bad.version = 2;
        assert!(bad.to_json(limits()).is_err());
        bad = saved.clone();
        bad.handlers.push(bad.handlers[0].clone());
        assert!(bad.to_json(limits()).is_err());
        let mut wrong = UiActionCheckpointRegistry::default();
        wrong
            .register("counter.v1", |payload, context| {
                let saved: SavedCounter =
                    serde_json::from_slice(payload).map_err(|e| e.to_string())?;
                Ok(Box::new(Counter {
                    calls: saved.calls + 1,
                    accumulated: f64::from_bits(saved.accumulated),
                    target: context.resolve(&saved.target)?,
                }))
            })
            .unwrap();
        assert!(
            wrong
                .prepare(&other, &saved, |id| other_ids.get(id).copied(), limits())
                .is_err()
        );
        assert_eq!(
            saved,
            actions
                .checkpoint(&scene, |n| identify(&ids, n), limits())
                .unwrap()
        );
        let mut codecs = registry();
        assert!(
            codecs
                .register("counter.v1", |_, _| Err("duplicate".into()))
                .is_err()
        );
        assert!(
            actions
                .register_checkpointable(
                    "counter".into(),
                    Counter {
                        calls: 0,
                        accumulated: 0.,
                        target: ids[&ObjectId("target".into())]
                    }
                )
                .is_err()
        );
    }
}
