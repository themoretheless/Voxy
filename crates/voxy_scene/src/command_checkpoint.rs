//! Durable pending command order, including expected stale-handle failures.
use super::*;
use crate::{ComponentRegistry, DocumentError, ObjectId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum CommandCheckpointError {
    #[error("invalid command checkpoint: {0}")]
    Invalid(String),
    #[error("command checkpoint exceeds its configured budget")]
    Capacity,
    #[error(transparent)]
    Scene(#[from] SceneGraphError),
    #[error(transparent)]
    Document(#[from] DocumentError),
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum SavedNode {
    Live(ObjectId),
    Invalid,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransformBits {
    translation: [u32; 3],
    rotation: [u32; 4],
    scale: [u32; 3],
}
impl From<Transform> for TransformBits {
    fn from(t: Transform) -> Self {
        Self {
            translation: t.translation.to_array().map(f32::to_bits),
            rotation: t.rotation.to_array().map(f32::to_bits),
            scale: t.scale.to_array().map(f32::to_bits),
        }
    }
}
impl TransformBits {
    fn decode(&self) -> Transform {
        Transform {
            translation: glam::Vec3::from_array(self.translation.map(f32::from_bits)),
            rotation: glam::Quat::from_array(self.rotation.map(f32::from_bits)),
            scale: glam::Vec3::from_array(self.scale.map(f32::from_bits)),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum SavedComponent {
    Insert(serde_json::Value),
    Remove,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum SavedCommand {
    Spawn(Option<SavedNode>, TransformBits),
    SetLocal(SavedNode, TransformBits),
    SetActive(SavedNode, bool),
    Reparent(SavedNode, Option<SavedNode>),
    Remove(SavedNode),
    Name(SavedNode, String),
    Component(SavedNode, String, SavedComponent),
}
/// Pending order, capacity policy and previous barrier's spawn journal. Component
/// payloads use the existing registered codecs, never raw Rust type names.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneCommandsCheckpoint {
    version: u32,
    capacity: usize,
    scene_capacity: usize,
    commands: Vec<SavedCommand>,
    spawned: Vec<(usize, SavedNode)>,
}
impl SceneCommandsCheckpoint {
    fn validate(
        &self,
        max_commands: usize,
        max_bytes: usize,
    ) -> Result<(), CommandCheckpointError> {
        if self.version != 1 {
            return Err(CommandCheckpointError::Invalid(
                "unsupported version".into(),
            ));
        }
        if self.commands.len() > max_commands
            || self.spawned.len() > max_commands
            || self.commands.len() > self.capacity
            || self.spawned.len() > self.capacity
        {
            return Err(CommandCheckpointError::Capacity);
        }
        if self
            .spawned
            .iter()
            .any(|(index, _)| *index >= self.capacity)
        {
            return Err(CommandCheckpointError::Invalid(
                "invalid spawn journal index".into(),
            ));
        }
        if serde_json::to_vec(self)?.len() > max_bytes {
            return Err(CommandCheckpointError::Capacity);
        }
        Ok(())
    }
    /// # Errors
    /// Rejects byte/count/version/encoding limits; outer file IO must also be bounded.
    pub fn from_json(
        text: &str,
        max_commands: usize,
        max_bytes: usize,
    ) -> Result<Self, CommandCheckpointError> {
        if text.len() > max_bytes {
            return Err(CommandCheckpointError::Capacity);
        }
        let saved: Self = serde_json::from_str(text)?;
        saved.validate(max_commands, max_bytes)?;
        Ok(saved)
    }
    /// # Errors
    /// Rejects invalid state or count/encoded byte limits.
    pub fn to_json(
        &self,
        max_commands: usize,
        max_bytes: usize,
    ) -> Result<String, CommandCheckpointError> {
        self.validate(max_commands, max_bytes)?;
        Ok(serde_json::to_string(self)?)
    }
}
impl SceneCommands {
    /// # Errors
    /// Rejects foreign scenes, missing/aliased durable IDs, unsupported component
    /// codecs and byte limits without draining or applying the queue.
    pub fn checkpoint(
        &self,
        scene: &SceneGraph,
        registry: &ComponentRegistry,
        identify: impl Fn(NodeId) -> Option<ObjectId>,
        max_bytes: usize,
    ) -> Result<SceneCommandsCheckpoint, CommandCheckpointError> {
        if scene.id != self.scene {
            return Err(SceneGraphError::InvalidNode.into());
        }
        let mut ids: HashMap<NodeId, ObjectId> = HashMap::new();
        let mut owners = HashMap::new();
        let mut node = |owner| {
            if scene.node(owner).is_err() {
                return Ok(SavedNode::Invalid);
            }
            if let Some(id) = ids.get(&owner) {
                return Ok(SavedNode::Live(id.clone()));
            }
            let id = identify(owner)
                .ok_or_else(|| CommandCheckpointError::Invalid("missing owner identity".into()))?;
            if id.0.is_empty() || owners.get(&id).is_some_and(|old| *old != owner) {
                return Err(CommandCheckpointError::Invalid(
                    "invalid or aliased identity".into(),
                ));
            }
            ids.insert(owner, id.clone());
            owners.insert(id.clone(), owner);
            Ok(SavedNode::Live(id))
        };
        let mut commands = Vec::with_capacity(self.commands.len());
        for command in &self.commands {
            commands.push(match command {
                Queued::Scene(SceneCommand::Spawn(parent, local)) => {
                    SavedCommand::Spawn(parent.map(&mut node).transpose()?, (*local).into())
                }
                Queued::Scene(SceneCommand::SetLocal(owner, local)) => {
                    SavedCommand::SetLocal(node(*owner)?, (*local).into())
                }
                Queued::Scene(SceneCommand::SetActive(owner, active)) => {
                    SavedCommand::SetActive(node(*owner)?, *active)
                }
                Queued::Scene(SceneCommand::Reparent(owner, parent)) => {
                    SavedCommand::Reparent(node(*owner)?, parent.map(&mut node).transpose()?)
                }
                Queued::Scene(SceneCommand::RemoveSubtree(owner)) => {
                    SavedCommand::Remove(node(*owner)?)
                }
                Queued::Name(owner, name) => SavedCommand::Name(node(*owner)?, name.clone()),
                Queued::Component(change) => {
                    let intent = change.intent();
                    let schema = registry.queued_schema(intent.type_id)?.to_owned();
                    let value = match intent.value {
                        Some(value) => {
                            SavedComponent::Insert(registry.encode_queued(intent.type_id, value)?)
                        }
                        None => SavedComponent::Remove,
                    };
                    SavedCommand::Component(node(intent.owner)?, schema, value)
                }
            });
        }
        let spawned = self
            .spawned
            .iter()
            .map(|(index, owner)| Ok((*index, node(*owner)?)))
            .collect::<Result<_, CommandCheckpointError>>()?;
        let saved = SceneCommandsCheckpoint {
            version: 1,
            capacity: self.capacity,
            scene_capacity: scene.capacity,
            commands,
            spawned,
        };
        saved.validate(self.capacity, max_bytes)?;
        Ok(saved)
    }
    /// # Errors
    /// Admits every command and journal binding into a candidate before replacing
    /// this queue. Invalid source handles remain guaranteed-invalid sentinels so
    /// original per-command errors and following command order are preserved.
    pub fn restore_checkpoint(
        &mut self,
        scene: &SceneGraph,
        registry: &ComponentRegistry,
        saved: &SceneCommandsCheckpoint,
        resolve: impl Fn(&ObjectId) -> Option<NodeId>,
        max_bytes: usize,
    ) -> Result<(), CommandCheckpointError> {
        if scene.id != self.scene {
            return Err(SceneGraphError::InvalidNode.into());
        }
        saved.validate(self.capacity, max_bytes)?;
        if saved.capacity != self.capacity || saved.scene_capacity != scene.capacity {
            return Err(CommandCheckpointError::Invalid(
                "queue/scene capacity mismatch".into(),
            ));
        }
        let mut ids = HashMap::new();
        let mut owners = HashMap::new();
        let mut node = |saved: &SavedNode| {
            let SavedNode::Live(id) = saved else {
                return Ok(NodeId {
                    scene: 0,
                    slot: 0,
                    generation: 0,
                });
            };
            if id.0.is_empty() {
                return Err(CommandCheckpointError::Invalid("empty identity".into()));
            }
            if let Some(&owner) = ids.get(id) {
                return Ok(owner);
            }
            let owner = resolve(id)
                .ok_or_else(|| CommandCheckpointError::Invalid("unresolved identity".into()))?;
            scene.node(owner)?;
            if owners.get(&owner).is_some_and(|old| old != id) {
                return Err(CommandCheckpointError::Invalid("aliased identities".into()));
            }
            ids.insert(id.clone(), owner);
            owners.insert(owner, id.clone());
            Ok(owner)
        };
        let mut candidate = Self::new(scene, self.capacity);
        for command in &saved.commands {
            let command = match command {
                SavedCommand::Spawn(parent, local) => Queued::Scene(SceneCommand::Spawn(
                    parent.as_ref().map(&mut node).transpose()?,
                    local.decode(),
                )),
                SavedCommand::SetLocal(owner, local) => {
                    Queued::Scene(SceneCommand::SetLocal(node(owner)?, local.decode()))
                }
                SavedCommand::SetActive(owner, active) => {
                    Queued::Scene(SceneCommand::SetActive(node(owner)?, *active))
                }
                SavedCommand::Reparent(owner, parent) => Queued::Scene(SceneCommand::Reparent(
                    node(owner)?,
                    parent.as_ref().map(&mut node).transpose()?,
                )),
                SavedCommand::Remove(owner) => {
                    Queued::Scene(SceneCommand::RemoveSubtree(node(owner)?))
                }
                SavedCommand::Name(owner, name) => Queued::Name(node(owner)?, name.clone()),
                SavedCommand::Component(owner, schema, value) => {
                    Queued::Component(registry.restore_queued(
                        node(owner)?,
                        schema,
                        match value {
                            SavedComponent::Insert(value) => Some(value),
                            SavedComponent::Remove => None,
                        },
                    )?)
                }
            };
            candidate.admit(command)?;
        }
        candidate.spawned = saved
            .spawned
            .iter()
            .map(|(index, owner)| Ok((*index, node(owner)?)))
            .collect::<Result<_, CommandCheckpointError>>()?;
        *self = candidate;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Health {
        value: u32,
    }
    fn fixture() -> (ComponentRegistry, crate::LoadedScene) {
        let mut registry = ComponentRegistry::default();
        registry.register::<Health>("test.health.v1").unwrap();
        let scene=crate::SceneDocument::from_json(r#"{"version":1,"objects":[
         {"id":"root","parent":null,"name":"root","active":true,"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{}},
         {"id":"child","parent":"root","name":"child","active":true,"translation":[0,1,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{}}
        ]}"#).unwrap().load(&registry,8).unwrap();
        (registry, scene)
    }
    #[test]
    fn pending_queue_json_rebind_keeps_order_errors_components_and_spawn_journal() {
        let (registry, mut source) = fixture();
        let root = source.resolve(&ObjectId("root".into())).unwrap();
        let child = source.resolve(&ObjectId("child".into())).unwrap();
        let mut queue = SceneCommands::new(&source.graph, 16);
        queue
            .push(SceneCommand::Spawn(Some(root), Transform::default()))
            .unwrap();
        assert_eq!(queue.apply(&mut source.graph).unwrap(), [Ok(())]);
        source
            .identify(queue.spawned()[0].1, ObjectId("prior-spawn".into()))
            .unwrap();
        let mut foreign = SceneGraph::new(1);
        let invalid = foreign.spawn(None, Transform::default()).unwrap();
        let mut nan = Transform::default();
        nan.translation.x = f32::NAN;
        queue
            .push(SceneCommand::SetLocal(
                root,
                Transform {
                    translation: glam::Vec3::X,
                    ..Default::default()
                },
            ))
            .unwrap();
        queue.push(SceneCommand::SetLocal(child, nan)).unwrap();
        queue.insert_component(root, Health { value: 9 }).unwrap();
        queue.remove_component::<Health>(child).unwrap();
        queue.set_name(root, "renamed".into()).unwrap();
        queue.push(SceneCommand::RemoveSubtree(child)).unwrap();
        queue
            .push(SceneCommand::Spawn(None, Transform::default()))
            .unwrap();
        queue.set_name(child, "late".into()).unwrap();
        queue.push(SceneCommand::SetActive(invalid, false)).unwrap();
        queue.push(SceneCommand::SetLocal(invalid, nan)).unwrap();
        queue
            .push(SceneCommand::Reparent(root, Some(root)))
            .unwrap();
        queue
            .push(SceneCommand::Spawn(None, Transform::default()))
            .unwrap();
        let saved = queue
            .checkpoint(
                &source.graph,
                &registry,
                |node| source.identity(node).cloned(),
                65536,
            )
            .unwrap();
        let text = saved.to_json(16, 65536).unwrap();
        let saved = SceneCommandsCheckpoint::from_json(&text, 16, 65536).unwrap();
        let mut document = source.capture(&registry).unwrap();
        document.objects.reverse();
        let mut target = document.load(&registry, 8).unwrap();
        let mut restored = SceneCommands::new(&target.graph, 16);
        restored
            .restore_checkpoint(
                &target.graph,
                &registry,
                &saved,
                |id| target.resolve(id),
                65536,
            )
            .unwrap();
        assert_eq!(
            target.identity(restored.spawned()[0].1).unwrap().0,
            "prior-spawn"
        );
        let a = queue.apply(&mut source.graph).unwrap();
        let b = restored.apply(&mut target.graph).unwrap();
        assert_eq!(a, b);
        assert!(a.iter().any(Result::is_err));
        for &(index, node) in queue.spawned() {
            source
                .identify(node, ObjectId(format!("after-{index}")))
                .unwrap();
        }
        for &(index, node) in restored.spawned() {
            target
                .identify(node, ObjectId(format!("after-{index}")))
                .unwrap();
        }
        let canonical = |loaded: &crate::LoadedScene| {
            loaded
                .capture(&registry)
                .unwrap()
                .objects
                .into_iter()
                .map(|object| (object.id.clone(), object))
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        assert_eq!(canonical(&source), canonical(&target));
        assert_eq!(source.graph.components::<Health>().count(), 1);
        assert_eq!(target.graph.components::<Health>().count(), 1);
        assert!(queue.apply(&mut source.graph).unwrap().is_empty());
        assert!(restored.apply(&mut target.graph).unwrap().is_empty());
    }
    #[test]
    fn corrupt_or_unsupported_queue_never_replaces_pending_live_commands() {
        let (registry, mut scene) = fixture();
        let root = scene.resolve(&ObjectId("root".into())).unwrap();
        let mut queue = SceneCommands::new(&scene.graph, 8);
        queue.set_name(root, "keep".into()).unwrap();
        let saved = queue
            .checkpoint(
                &scene.graph,
                &registry,
                |node| scene.identity(node).cloned(),
                65536,
            )
            .unwrap();
        let mut wrong_version = saved.clone();
        wrong_version.version = 2;
        let mut unknown = saved.clone();
        unknown.commands.push(SavedCommand::Component(
            SavedNode::Live(ObjectId("root".into())),
            "unknown".into(),
            SavedComponent::Insert(serde_json::json!(1)),
        ));
        for bad in [wrong_version, unknown] {
            assert!(
                queue
                    .restore_checkpoint(
                        &scene.graph,
                        &registry,
                        &bad,
                        |id| scene.resolve(id),
                        65536
                    )
                    .is_err()
            );
            assert_eq!(
                queue
                    .checkpoint(
                        &scene.graph,
                        &registry,
                        |node| scene.identity(node).cloned(),
                        65536
                    )
                    .unwrap(),
                saved
            );
        }
        assert!(
            queue
                .restore_checkpoint(&scene.graph, &registry, &saved, |_| None, 65536)
                .is_err()
        );
        assert!(SceneCommandsCheckpoint::from_json("{}", 8, 1).is_err());
        assert!(saved.to_json(8, 1).is_err());
        assert_eq!(queue.apply(&mut scene.graph).unwrap(), [Ok(())]);
        #[derive(Debug)]
        struct Opaque;
        queue.insert_component(root, Opaque).unwrap();
        assert!(
            queue
                .checkpoint(
                    &scene.graph,
                    &registry,
                    |node| scene.identity(node).cloned(),
                    65536
                )
                .is_err()
        );
        assert_eq!(queue.apply(&mut scene.graph).unwrap(), [Ok(())]);
        assert!(scene.graph.component::<Opaque>(root).unwrap().is_some());
    }
    #[test]
    fn null_component_insertion_is_distinct_from_removal_after_json() {
        #[derive(Debug, Serialize, Deserialize)]
        struct Marker;
        let mut registry = ComponentRegistry::default();
        registry.register::<Marker>("test.null.v1").unwrap();
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Default::default()).unwrap();
        let mut queue = SceneCommands::new(&scene, 1);
        queue.insert_component(owner, Marker).unwrap();
        let saved = queue
            .checkpoint(&scene, &registry, |_| Some(ObjectId("owner".into())), 65536)
            .unwrap();
        let text = saved.to_json(1, 65536).unwrap();
        let saved = SceneCommandsCheckpoint::from_json(&text, 1, 65536).unwrap();
        let mut restored = SceneCommands::new(&scene, 1);
        restored
            .restore_checkpoint(&scene, &registry, &saved, |_| Some(owner), 65536)
            .unwrap();
        assert_eq!(restored.apply(&mut scene).unwrap(), [Ok(())]);
        assert!(scene.component::<Marker>(owner).unwrap().is_some());
        queue = SceneCommands::new(&scene, 1);
        queue.remove_component::<Marker>(owner).unwrap();
        let saved = queue
            .checkpoint(&scene, &registry, |_| Some(ObjectId("owner".into())), 65536)
            .unwrap();
        let saved = SceneCommandsCheckpoint::from_json(&saved.to_json(1, 65536).unwrap(), 1, 65536)
            .unwrap();
        restored
            .restore_checkpoint(&scene, &registry, &saved, |_| Some(owner), 65536)
            .unwrap();
        assert_eq!(restored.apply(&mut scene).unwrap(), [Ok(())]);
        assert!(scene.component::<Marker>(owner).unwrap().is_none());
    }

    #[test]
    fn registered_component_f64_survives_pending_json_roundtrip_exactly() {
        #[derive(Debug, Serialize, Deserialize)]
        struct Precise {
            value: f64,
        }
        let mut registry = ComponentRegistry::default();
        registry.register::<Precise>("test.precise.v1").unwrap();
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Default::default()).unwrap();
        let mut bits = 0x123456789abcdef0u64;
        for _ in 0..1024 {
            bits = bits.wrapping_mul(6364136223846793005).wrapping_add(1);
            let value = f64::from_bits(bits);
            if !value.is_finite() {
                continue;
            }
            let mut queue = SceneCommands::new(&scene, 1);
            queue.insert_component(owner, Precise { value }).unwrap();
            let saved = queue
                .checkpoint(&scene, &registry, |_| Some(ObjectId("owner".into())), 65536)
                .unwrap();
            let text = saved.to_json(1, 65536).unwrap();
            let saved = SceneCommandsCheckpoint::from_json(&text, 1, 65536).unwrap();
            let mut restored = SceneCommands::new(&scene, 1);
            restored
                .restore_checkpoint(&scene, &registry, &saved, |_| Some(owner), 65536)
                .unwrap();
            assert_eq!(restored.apply(&mut scene).unwrap(), [Ok(())]);
            assert_eq!(
                scene
                    .component::<Precise>(owner)
                    .unwrap()
                    .unwrap()
                    .value
                    .to_bits(),
                bits,
                "bits={bits:016x}"
            );
        }
    }
}
