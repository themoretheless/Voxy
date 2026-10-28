//! Explicit behavior codecs. Factories prepare new instances without lifecycle hooks.
use crate::{Behavior, BehaviorRunner, NodeId, ObjectId, SceneGraph, SceneGraphError};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum BehaviorCheckpointError {
    #[error("behavior does not support checkpoints: {0}")]
    Unsupported(String),
    #[error("invalid behavior checkpoint: {0}")]
    Invalid(String),
    #[error("behavior checkpoint exceeds its configured budget")]
    Capacity,
    #[error("behavior codec: {0}")]
    Codec(String),
    #[error(transparent)]
    Scene(#[from] SceneGraphError),
    #[error(transparent)]
    Simulation(#[from] crate::SimulationError),
}
/// A durable codec name and opaque, unchanged payload bytes. User codecs own
/// version migration and exact numeric representation of their private state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorState {
    schema: String,
    payload: Vec<u8>,
}
impl BehaviorState {
    /// # Errors
    /// Rejects empty, oversized or NUL-containing schema names.
    pub fn new(schema: String, payload: Vec<u8>) -> Result<Self, BehaviorCheckpointError> {
        let state = Self { schema, payload };
        state.validate()?;
        Ok(state)
    }
    /// JSON convenience codec; use integer bits in schemas needing exact floats.
    /// # Errors
    /// Rejects invalid schema names or serialization failures.
    pub fn json<T: Serialize>(schema: &str, value: &T) -> Result<Self, BehaviorCheckpointError> {
        Self::new(
            schema.into(),
            serde_json::to_vec(value).map_err(|e| BehaviorCheckpointError::Codec(e.to_string()))?,
        )
    }
    fn validate(&self) -> Result<(), BehaviorCheckpointError> {
        if self.schema.is_empty() || self.schema.len() > 1024 || self.schema.contains('\0') {
            return Err(BehaviorCheckpointError::Invalid(
                "invalid codec name".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedBehavior {
    owner: ObjectId,
    enabled: bool,
    started: bool,
    state: BehaviorState,
}
/// Attachment order and lifecycle flags are retained, including inactive entries.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorCheckpoint {
    version: u32,
    entries: Vec<SavedBehavior>,
}
impl BehaviorCheckpoint {
    fn validate(
        &self,
        max_behaviors: usize,
        max_bytes: usize,
    ) -> Result<(), BehaviorCheckpointError> {
        if self.version != 1 {
            return Err(BehaviorCheckpointError::Invalid(
                "unsupported version".into(),
            ));
        }
        if self.entries.len() > max_behaviors {
            return Err(BehaviorCheckpointError::Capacity);
        }
        for entry in &self.entries {
            if entry.owner.0.is_empty() {
                return Err(BehaviorCheckpointError::Invalid(
                    "missing owner identity".into(),
                ));
            }
            entry.state.validate()?;
        }
        if serde_json::to_vec(self)
            .map_err(|e| BehaviorCheckpointError::Codec(e.to_string()))?
            .len()
            > max_bytes
        {
            return Err(BehaviorCheckpointError::Capacity);
        }
        Ok(())
    }
    /// # Errors
    /// Checks input bytes before parsing, then entry/schema/output-size budgets.
    pub fn from_json(
        text: &str,
        max_behaviors: usize,
        max_bytes: usize,
    ) -> Result<Self, BehaviorCheckpointError> {
        if text.len() > max_bytes {
            return Err(BehaviorCheckpointError::Capacity);
        }
        let saved: Self = serde_json::from_str(text)
            .map_err(|e| BehaviorCheckpointError::Codec(e.to_string()))?;
        saved.validate(max_behaviors, max_bytes)?;
        Ok(saved)
    }
    /// # Errors
    /// Rejects invalid state or encoded byte/entry budget overflow.
    pub fn to_json(
        &self,
        max_behaviors: usize,
        max_bytes: usize,
    ) -> Result<String, BehaviorCheckpointError> {
        self.validate(max_behaviors, max_bytes)?;
        serde_json::to_string(self).map_err(|e| BehaviorCheckpointError::Codec(e.to_string()))
    }
}
/// Read-only binding context. Factories never receive mutable scene/simulation access.
pub struct BehaviorRestoreContext<'a> {
    scene: &'a SceneGraph,
    resolve: &'a dyn Fn(&ObjectId) -> Option<NodeId>,
}
impl std::fmt::Debug for BehaviorRestoreContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BehaviorRestoreContext")
            .finish_non_exhaustive()
    }
}
impl BehaviorRestoreContext<'_> {
    /// # Errors
    /// Rejects unresolved, foreign or stale object identities.
    pub fn resolve(&self, id: &ObjectId) -> Result<NodeId, BehaviorCheckpointError> {
        let node = (self.resolve)(id)
            .ok_or_else(|| BehaviorCheckpointError::Invalid("unresolved object".into()))?;
        self.scene.local(node)?;
        Ok(node)
    }
    #[must_use]
    pub fn scene(&self) -> &SceneGraph {
        self.scene
    }
}
type Factory = dyn Fn(&[u8], &BehaviorRestoreContext<'_>) -> Result<Box<dyn Behavior>, BehaviorCheckpointError>
    + Send
    + Sync;
#[derive(Default)]
pub struct BehaviorCheckpointRegistry {
    factories: HashMap<String, Box<Factory>>,
}
impl std::fmt::Debug for BehaviorCheckpointRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BehaviorCheckpointRegistry")
            .field("codecs", &self.factories.len())
            .finish()
    }
}
impl BehaviorCheckpointRegistry {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }

    /// Register a serde JSON behavior factory; awake/start are not dispatched.
    /// # Errors
    /// Rejects invalid/duplicate names without replacing prior codecs.
    pub fn register<B: Behavior + DeserializeOwned + 'static>(
        &mut self,
        schema: &str,
    ) -> Result<(), BehaviorCheckpointError> {
        self.register_with(schema, |payload, _| {
            serde_json::from_slice::<B>(payload)
                .map(|b| Box::new(b) as Box<dyn Behavior>)
                .map_err(|e| BehaviorCheckpointError::Codec(e.to_string()))
        })
    }
    /// # Errors
    /// Rejects invalid/duplicate codec names. Factory callbacks must not mutate
    /// external live state; their side effects/panics cannot be rolled back.
    pub fn register_with(
        &mut self,
        schema: &str,
        factory: impl Fn(
            &[u8],
            &BehaviorRestoreContext<'_>,
        ) -> Result<Box<dyn Behavior>, BehaviorCheckpointError>
        + Send
        + Sync
        + 'static,
    ) -> Result<(), BehaviorCheckpointError> {
        BehaviorState::new(schema.into(), Vec::new())?;
        if self.factories.contains_key(schema) {
            return Err(BehaviorCheckpointError::Invalid("duplicate codec".into()));
        }
        self.factories.insert(schema.into(), Box::new(factory));
        Ok(())
    }
    /// Prepare a separate runner, resolving owners and retaining exact attachment
    /// order/flags. No existing runner or scene is changed and no hooks are called.
    /// # Errors
    /// Rejects budgets, unknown codecs, invalid/aliased owners or factory errors.
    pub fn prepare(
        &self,
        scene: &SceneGraph,
        saved: &BehaviorCheckpoint,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_behaviors: usize,
        max_bytes: usize,
    ) -> Result<BehaviorRunner, BehaviorCheckpointError> {
        saved.validate(max_behaviors, max_bytes)?;
        let context = BehaviorRestoreContext {
            scene,
            resolve: &resolve,
        };
        let mut ids = HashMap::new();
        let mut owners = HashMap::new();
        let mut entries = Vec::with_capacity(saved.entries.len());
        for saved in &saved.entries {
            let owner = if let Some(&node) = ids.get(&saved.owner) {
                node
            } else {
                let owner = context.resolve(&saved.owner)?;
                if owners.get(&owner).is_some_and(|id| id != &saved.owner) {
                    return Err(BehaviorCheckpointError::Invalid(
                        "aliased object identities".into(),
                    ));
                }
                ids.insert(saved.owner.clone(), owner);
                owners.insert(owner, saved.owner.clone());
                owner
            };
            let factory = self
                .factories
                .get(&saved.state.schema)
                .ok_or_else(|| BehaviorCheckpointError::Unsupported(saved.state.schema.clone()))?;
            let behavior = factory(&saved.state.payload, &context)?;
            entries.push(crate::behavior::Entry {
                owner,
                behavior,
                enabled: saved.enabled,
                started: saved.started,
            });
        }
        Ok(BehaviorRunner { entries })
    }
}
impl BehaviorRunner {
    /// # Errors
    /// Rejects unsupported attached behaviors, missing/aliased IDs and budgets.
    /// Serialization runs without scene mutation or lifecycle dispatch.
    pub fn checkpoint(
        &self,
        scene: &SceneGraph,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max_behaviors: usize,
        max_bytes: usize,
    ) -> Result<BehaviorCheckpoint, BehaviorCheckpointError> {
        if self.entries.len() > max_behaviors {
            return Err(BehaviorCheckpointError::Capacity);
        }
        let mut ids: HashMap<NodeId, ObjectId> = HashMap::new();
        let mut owners = HashMap::new();
        let mut entries = Vec::new();
        for entry in &self.entries {
            scene.local(entry.owner)?;
            let owner = if let Some(id) = ids.get(&entry.owner) {
                id.clone()
            } else {
                let id = identify(entry.owner)
                    .ok_or_else(|| BehaviorCheckpointError::Invalid("missing owner".into()))?;
                if owners.get(&id).is_some_and(|node| *node != entry.owner) {
                    return Err(BehaviorCheckpointError::Invalid(
                        "aliased owner identity".into(),
                    ));
                }
                ids.insert(entry.owner, id.clone());
                owners.insert(id.clone(), entry.owner);
                id
            };
            entries.push(SavedBehavior {
                owner,
                enabled: entry.enabled,
                started: entry.started,
                state: entry.behavior.checkpoint()?,
            });
        }
        let saved = BehaviorCheckpoint {
            version: 1,
            entries,
        };
        saved.validate(max_behaviors, max_bytes)?;
        Ok(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Serialize, Deserialize)]
    struct Counter {
        label: u32,
        awake: u32,
        start: u32,
        enable: u32,
        disable: u32,
        fixed: u32,
        update: u32,
    }
    impl Counter {
        fn new(label: u32) -> Self {
            Self {
                label,
                awake: 0,
                start: 0,
                enable: 0,
                disable: 0,
                fixed: 0,
                update: 0,
            }
        }
    }
    impl Behavior for Counter {
        fn checkpoint(&self) -> Result<BehaviorState, BehaviorCheckpointError> {
            BehaviorState::json("sample.counter.v1", self)
        }
        fn awake(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.awake += 1;
        }
        fn start(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.start += 1;
        }
        fn on_enable(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.enable += 1;
        }
        fn on_disable(&mut self, _: &mut SceneGraph, _: NodeId) {
            self.disable += 1;
        }
        fn fixed_update(&mut self, scene: &mut SceneGraph, owner: NodeId, _: f64) {
            self.fixed += 1;
            let mut local = scene.local(owner).unwrap();
            local.translation.x += self.label as f32 * 0.001;
            scene.set_local(owner, local).unwrap();
        }
        fn update(&mut self, _: &mut SceneGraph, _: NodeId, _: f64) {
            self.update += 1;
        }
    }
    fn identity(_: NodeId) -> Option<ObjectId> {
        Some(ObjectId("owner".into()))
    }
    #[test]
    fn behavior_json_restore_keeps_order_lifecycle_and_120_following_updates() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Default::default()).unwrap();
        let mut runner = BehaviorRunner::default();
        runner.attach(&mut scene, owner, Counter::new(1)).unwrap();
        runner.attach(&mut scene, owner, Counter::new(2)).unwrap();
        runner.fixed_update(&mut scene, 0.01);
        runner.update(&mut scene, 0.01);
        let saved = runner.checkpoint(&scene, identity, 4, 65536).unwrap();
        let text = saved.to_json(4, 65536).unwrap();
        let saved = BehaviorCheckpoint::from_json(&text, 4, 65536).unwrap();
        let mut other_scene = SceneGraph::new(1);
        let other = other_scene
            .spawn(None, scene.local(owner).unwrap())
            .unwrap();
        let mut registry = BehaviorCheckpointRegistry::default();
        registry.register::<Counter>("sample.counter.v1").unwrap();
        assert!(registry.register::<Counter>("sample.counter.v1").is_err());
        let mut restored = registry
            .prepare(
                &other_scene,
                &saved,
                |id| (id.0 == "owner").then_some(other),
                4,
                65536,
            )
            .unwrap();
        assert_eq!(
            restored
                .checkpoint(&other_scene, identity, 4, 65536)
                .unwrap(),
            saved
        );
        for i in 0..120 {
            if i == 20 || i == 40 {
                let active = i == 40;
                scene.set_active(owner, active).unwrap();
                other_scene.set_active(other, active).unwrap();
            }
            runner.fixed_update(&mut scene, 0.01);
            restored.fixed_update(&mut other_scene, 0.01);
            runner.update(&mut scene, 0.01);
            restored.update(&mut other_scene, 0.01);
            assert_eq!(
                runner.checkpoint(&scene, identity, 4, 65536).unwrap(),
                restored
                    .checkpoint(&other_scene, identity, 4, 65536)
                    .unwrap()
            );
            assert_eq!(
                scene.local(owner).unwrap(),
                other_scene.local(other).unwrap()
            );
        }
    }
    #[test]
    fn opaque_payload_and_custom_object_reference_restore_without_hooks() {
        #[derive(Debug)]
        struct Linked {
            payload: Vec<u8>,
            target: NodeId,
        }
        impl Behavior for Linked {
            fn checkpoint(&self) -> Result<BehaviorState, BehaviorCheckpointError> {
                BehaviorState::new("sample.linked.v1".into(), self.payload.clone())
            }
            fn fixed_update(&mut self, scene: &mut SceneGraph, _: NodeId, _: f64) {
                scene.set_active(self.target, false).unwrap();
            }
        }
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Default::default()).unwrap();
        let mut original = BehaviorRunner::default();
        original
            .attach(
                &mut scene,
                owner,
                Linked {
                    payload: vec![0, 255, 128, 1],
                    target: owner,
                },
            )
            .unwrap();
        let saved = original.checkpoint(&scene, identity, 4, 65536).unwrap();
        let text = saved.to_json(4, 65536).unwrap();
        let saved = BehaviorCheckpoint::from_json(&text, 4, 65536).unwrap();
        let mut target_scene = SceneGraph::new(1);
        let target = target_scene.spawn(None, Default::default()).unwrap();
        let mut registry = BehaviorCheckpointRegistry::default();
        registry
            .register_with("sample.linked.v1", |payload, context| {
                assert_eq!(payload, [0, 255, 128, 1]);
                let target = context.resolve(&ObjectId("owner".into()))?;
                Ok(Box::new(Linked {
                    payload: payload.to_vec(),
                    target,
                }))
            })
            .unwrap();
        let mut restored = registry
            .prepare(
                &target_scene,
                &saved,
                |id| (id.0 == "owner").then_some(target),
                4,
                65536,
            )
            .unwrap();
        assert!(target_scene.active_in_hierarchy(target).unwrap());
        assert_eq!(
            restored
                .checkpoint(&target_scene, identity, 4, 65536)
                .unwrap(),
            saved
        );
        restored.fixed_update(&mut target_scene, 0.01);
        assert!(!target_scene.active_in_hierarchy(target).unwrap());
        assert!(scene.active_in_hierarchy(owner).unwrap());
    }

    #[derive(Debug)]
    struct Unsupported;
    impl Behavior for Unsupported {}
    #[test]
    fn unsupported_corrupt_and_overbudget_states_are_not_dropped() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Default::default()).unwrap();
        let mut runner = BehaviorRunner::default();
        runner.attach(&mut scene, owner, Unsupported).unwrap();
        assert!(matches!(
            runner.checkpoint(&scene, identity, 4, 65536),
            Err(BehaviorCheckpointError::Unsupported(_))
        ));
        let mut source = BehaviorRunner::default();
        source.attach(&mut scene, owner, Counter::new(1)).unwrap();
        let saved = source.checkpoint(&scene, identity, 4, 65536).unwrap();
        let registry = BehaviorCheckpointRegistry::default();
        assert!(
            registry
                .prepare(&scene, &saved, |_| Some(owner), 4, 65536)
                .is_err()
        );
        let mut registry = BehaviorCheckpointRegistry::default();
        registry.register::<Counter>("sample.counter.v1").unwrap();
        assert!(
            registry
                .prepare(&scene, &saved, |_| None, 4, 65536)
                .is_err()
        );
        assert!(saved.to_json(0, 65536).is_err());
        assert!(saved.to_json(4, 1).is_err());
        assert!(BehaviorCheckpoint::from_json("{}", 4, 1).is_err());
        let mut bad = saved.clone();
        bad.entries[0].state.payload = b"broken".to_vec();
        assert!(
            registry
                .prepare(&scene, &bad, |_| Some(owner), 4, 65536)
                .is_err()
        );
        assert_eq!(
            source.checkpoint(&scene, identity, 4, 65536).unwrap(),
            saved
        );
    }
    #[test]
    fn simulation_restores_only_into_empty_runner_without_replaying_hooks() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Default::default()).unwrap();
        let limits = crate::SimulationLimits {
            fixed_step: 1. / 60.,
            max_steps: 2,
            max_behaviors: 4,
            max_commands: 0,
        };
        let mut original = crate::SceneSimulation::new(&scene, limits).unwrap();
        original.attach(&mut scene, owner, Counter::new(1)).unwrap();
        original.advance(&mut scene, 0.02).unwrap();
        let saved = original
            .behavior_checkpoint(&scene, identity, 65536)
            .unwrap();
        let mut registry = BehaviorCheckpointRegistry::default();
        registry.register::<Counter>("sample.counter.v1").unwrap();
        assert!(
            original
                .restore_behavior_checkpoint(&scene, &saved, &registry, |_| Some(owner), 65536)
                .is_err()
        );
        assert_eq!(
            original
                .behavior_checkpoint(&scene, identity, 65536)
                .unwrap(),
            saved
        );
        let mut target = crate::SceneSimulation::new(&scene, limits).unwrap();
        target
            .restore_behavior_checkpoint(&scene, &saved, &registry, |_| Some(owner), 65536)
            .unwrap();
        assert_eq!(
            target.behavior_checkpoint(&scene, identity, 65536).unwrap(),
            saved
        );
    }
}
