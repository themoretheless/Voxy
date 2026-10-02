//! Bounded structural changes applied at an explicit simulation barrier.
use crate::{NodeId, SceneGraph, SceneGraphError, Transform};
#[derive(Clone, Copy, Debug)]
pub enum SceneCommand {
    Spawn(Option<NodeId>, Transform),
    SetLocal(NodeId, Transform),
    SetActive(NodeId, bool),
    Reparent(NodeId, Option<NodeId>),
    RemoveSubtree(NodeId),
}
trait ComponentChange: Send + Sync + std::fmt::Debug {
    fn apply(self: Box<Self>, scene: &mut SceneGraph) -> Result<(), SceneGraphError>;
}
struct Insert<T> {
    owner: NodeId,
    value: T,
}
impl<T> std::fmt::Debug for Insert<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Insert")
            .field(&self.owner)
            .field(&std::any::type_name::<T>())
            .finish()
    }
}
impl<T: std::any::Any + Send + Sync> ComponentChange for Insert<T> {
    fn apply(self: Box<Self>, scene: &mut SceneGraph) -> Result<(), SceneGraphError> {
        scene.insert_component(self.owner, self.value).map(|_| ())
    }
}
struct Remove<T> {
    owner: NodeId,
    marker: std::marker::PhantomData<fn() -> T>,
}
impl<T> std::fmt::Debug for Remove<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Remove")
            .field(&self.owner)
            .field(&std::any::type_name::<T>())
            .finish()
    }
}
impl<T: std::any::Any + Send + Sync> ComponentChange for Remove<T> {
    fn apply(self: Box<Self>, scene: &mut SceneGraph) -> Result<(), SceneGraphError> {
        scene.remove_component::<T>(self.owner).map(|_| ())
    }
}
#[derive(Debug)]
enum Queued {
    Scene(SceneCommand),
    Component(Box<dyn ComponentChange>),
    Name(NodeId, String),
}
#[derive(Debug)]
pub struct SceneCommands {
    scene: u64,
    capacity: usize,
    commands: Vec<Queued>,
    spawned: Vec<(usize, NodeId)>,
}
impl SceneCommands {
    #[must_use]
    pub fn new(scene: &SceneGraph, capacity: usize) -> Self {
        Self {
            scene: scene.id,
            capacity,
            commands: Vec::new(),
            spawned: Vec::new(),
        }
    }
    /// Identifies the scene that owns this queue, without exposing its contents.
    #[must_use]
    pub const fn scene_identity(&self) -> crate::SceneId {
        crate::SceneId(self.scene)
    }
    /// Atomically admits another bounded buffer for the same scene.
    /// On rejection both buffers remain unchanged. Command application itself
    /// retains its existing per-command error semantics at the frame barrier.
    pub fn append(&mut self, other: &mut Self) -> Result<(), SceneGraphError> {
        if self.scene != other.scene {
            return Err(SceneGraphError::InvalidNode);
        }
        if other.commands.len() > self.capacity.saturating_sub(self.commands.len()) {
            return Err(SceneGraphError::Capacity);
        }
        self.commands.append(&mut other.commands);
        Ok(())
    }
    /// Queues a change without mutating the world. Handles are validated at apply.
    /// # Errors
    /// Rejects a full queue; the command is not added.
    pub fn push(&mut self, command: SceneCommand) -> Result<(), SceneGraphError> {
        self.admit(Queued::Scene(command))
    }
    fn admit(&mut self, command: Queued) -> Result<(), SceneGraphError> {
        if self.commands.len() >= self.capacity {
            return Err(SceneGraphError::Capacity);
        }
        self.commands.push(command);
        Ok(())
    }
    /// Queues typed insertion/replacement. Payload byte/clone cost belongs to T.
    /// # Errors
    /// A full queue returns Capacity; owner validation happens at the barrier.
    pub fn insert_component<T: std::any::Any + Send + Sync>(
        &mut self,
        owner: NodeId,
        value: T,
    ) -> Result<(), SceneGraphError> {
        self.admit(Queued::Component(Box::new(Insert { owner, value })))
    }
    /// Queues removal; removing an absent component is successful.
    /// # Errors
    /// Rejects a full queue; stale owners fail at the barrier.
    pub fn remove_component<T: std::any::Any + Send + Sync>(
        &mut self,
        owner: NodeId,
    ) -> Result<(), SceneGraphError> {
        self.admit(Queued::Component(Box::new(Remove::<T> {
            owner,
            marker: std::marker::PhantomData,
        })))
    }
    /// # Errors
    /// Rejects a full queue; the name's byte allocation is caller-owned.
    pub fn set_name(&mut self, owner: NodeId, name: String) -> Result<(), SceneGraphError> {
        self.admit(Queued::Name(owner, name))
    }
    /// Successful Spawn command results from the last apply, with command index.
    /// Use these handles to submit component writes at the following barrier.
    #[must_use]
    pub fn spawned(&self) -> &[(usize, NodeId)] {
        &self.spawned
    }
    /// Applies in insertion order, returning a result for every command.
    /// Contiguous transform writes coalesce before structural commands.
    /// Each command is individually validated, not the whole batch atomically.
    /// Failed commands do not prevent later commands from being attempted.
    /// # Errors
    /// Rejects a foreign scene without draining the queue.
    pub fn apply(
        &mut self,
        scene: &mut SceneGraph,
    ) -> Result<Vec<Result<(), SceneGraphError>>, SceneGraphError> {
        if scene.id != self.scene {
            return Err(SceneGraphError::InvalidNode);
        }
        self.spawned.clear();
        let mut commands = self.commands.drain(..).peekable();
        let mut results = Vec::new();
        while let Some(command) = commands.next() {
            if let Queued::Scene(SceneCommand::SetLocal(id, local)) = command {
                let mut edits = Vec::new();
                let mut current = Some((id, local));
                while let Some((id, local)) = current {
                    let valid = local.matrix().and_then(|_| scene.node(id).map(|_| ()));
                    if valid.is_ok() {
                        edits.push((id, local));
                    }
                    results.push(valid);
                    current = if matches!(
                        commands.peek(),
                        Some(Queued::Scene(SceneCommand::SetLocal(_, _)))
                    ) {
                        match commands.next() {
                            Some(Queued::Scene(SceneCommand::SetLocal(id, local))) => {
                                Some((id, local))
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                }
                scene.set_locals(&edits)?;
            } else {
                let index = results.len();
                results.push(match command {
                    Queued::Scene(SceneCommand::Spawn(parent, local)) => {
                        scene.spawn(parent, local).map(|owner| {
                            self.spawned.push((index, owner));
                        })
                    }
                    Queued::Component(change) => change.apply(scene),
                    Queued::Name(owner, name) => scene.set_name(owner, name),
                    Queued::Scene(SceneCommand::SetActive(id, active)) => {
                        scene.set_active(id, active)
                    }
                    Queued::Scene(SceneCommand::Reparent(id, parent)) => scene.reparent(id, parent),
                    Queued::Scene(SceneCommand::RemoveSubtree(id)) => {
                        scene.remove_subtree(id).map(|_| ())
                    }
                    Queued::Scene(SceneCommand::SetLocal(id, local)) => scene.set_local(id, local),
                });
            }
        }
        Ok(results)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordered_creation_typed_crud_names_and_failed_commands_are_barrier_owned() {
        let mut scene = SceneGraph::new(2);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let mut commands = SceneCommands::new(&scene, 8);
        commands
            .push(SceneCommand::Spawn(Some(root), Transform::default()))
            .unwrap();
        commands
            .push(SceneCommand::Spawn(None, Transform::default()))
            .unwrap();
        assert_eq!(scene.len(), 1);
        assert_eq!(
            commands.apply(&mut scene).unwrap(),
            [Ok(()), Err(SceneGraphError::Capacity)]
        );
        let child = commands.spawned()[0].1;
        assert_eq!(commands.spawned()[0].0, 0);
        commands.insert_component(child, 1_u32).unwrap();
        commands.insert_component(child, 2_u32).unwrap();
        commands.set_name(child, "Child".into()).unwrap();
        commands.remove_component::<u32>(root).unwrap();
        assert_eq!(scene.component::<u32>(child).unwrap(), None);
        assert!(
            commands
                .apply(&mut scene)
                .unwrap()
                .iter()
                .all(Result::is_ok)
        );
        assert_eq!(scene.component::<u32>(child).unwrap(), Some(&2));
        assert_eq!(scene.name(child).unwrap(), "Child");
        commands.remove_component::<u32>(child).unwrap();
        commands.push(SceneCommand::RemoveSubtree(child)).unwrap();
        commands.insert_component(child, 3_u32).unwrap();
        commands.insert_component(root, 4_u32).unwrap();
        assert_eq!(
            commands.apply(&mut scene).unwrap(),
            [Ok(()), Ok(()), Err(SceneGraphError::InvalidNode), Ok(())]
        );
        let reused = scene.spawn(None, Transform::default()).unwrap();
        assert_ne!(reused, child);
        assert_eq!(scene.component::<u32>(reused).unwrap(), None);
        assert_eq!(scene.component::<u32>(root).unwrap(), Some(&4));
    }
    #[test]
    fn commands_wait_for_barrier_respect_bounds_and_report_stale_targets() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let mut commands = SceneCommands::new(&scene, 2);
        commands.push(SceneCommand::RemoveSubtree(owner)).unwrap();
        commands
            .push(SceneCommand::SetActive(owner, false))
            .unwrap();
        assert_eq!(
            commands.push(SceneCommand::SetActive(owner, true)),
            Err(SceneGraphError::Capacity)
        );
        assert_eq!(scene.len(), 1);
        assert!(commands.apply(&mut SceneGraph::new(1)).is_err());
        assert_eq!(
            commands.apply(&mut scene).unwrap(),
            vec![Ok(()), Err(SceneGraphError::InvalidNode)]
        );
        assert!(commands.apply(&mut scene).unwrap().is_empty());
    }
    #[test]
    fn failed_components_release_payload_once_and_later_commands_still_apply() {
        #[derive(Debug)]
        struct Counted(std::sync::Arc<std::sync::atomic::AtomicUsize>);
        impl Drop for Counted {
            fn drop(&mut self) {
                self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
        let drops = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut scene = SceneGraph::new(2);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let expired = scene.spawn(None, Transform::default()).unwrap();
        let mut commands = SceneCommands::new(&scene, 4);
        commands
            .insert_component(expired, Counted(std::sync::Arc::clone(&drops)))
            .unwrap();
        commands
            .push(SceneCommand::SetLocal(
                root,
                Transform {
                    translation: glam::Vec3::splat(f32::NAN),
                    ..Transform::default()
                },
            ))
            .unwrap();
        commands
            .push(SceneCommand::Spawn(None, Transform::default()))
            .unwrap();
        commands
            .insert_component(root, Counted(std::sync::Arc::clone(&drops)))
            .unwrap();
        scene.remove_subtree(expired).unwrap();
        assert_eq!(
            commands.apply(&mut scene).unwrap(),
            vec![
                Err(SceneGraphError::InvalidNode),
                Err(SceneGraphError::InvalidTransform),
                Ok(()),
                Ok(())
            ]
        );
        assert_eq!(commands.spawned().len(), 1);
        assert_eq!(commands.spawned()[0].0, 2);
        assert_ne!(commands.spawned()[0].1, expired);
        assert_eq!(drops.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(scene.component::<Counted>(root).unwrap().is_some());
        assert!(commands.apply(&mut scene).unwrap().is_empty());
        assert_eq!(drops.load(std::sync::atomic::Ordering::SeqCst), 1);
        scene.remove_component::<Counted>(root).unwrap();
        assert_eq!(drops.load(std::sync::atomic::Ordering::SeqCst), 2);
        commands.set_name(root, "after errors".into()).unwrap();
        assert_eq!(commands.apply(&mut scene).unwrap(), vec![Ok(())]);
        assert_eq!(drops.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn transform_groups_flush_before_reparent_and_keep_per_command_errors() {
        let mut scene = SceneGraph::new(2);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(root), Transform::default()).unwrap();
        let mut commands = SceneCommands::new(&scene, 8);
        let at = |x| Transform {
            translation: glam::Vec3::X * x,
            ..Transform::default()
        };
        commands
            .push(SceneCommand::SetLocal(root, at(2.0)))
            .unwrap();
        commands
            .push(SceneCommand::SetLocal(child, at(f32::NAN)))
            .unwrap();
        commands
            .push(SceneCommand::SetLocal(child, at(3.0)))
            .unwrap();
        commands.push(SceneCommand::Reparent(child, None)).unwrap();
        commands
            .push(SceneCommand::SetLocal(root, at(9.0)))
            .unwrap();
        assert_eq!(
            commands.apply(&mut scene).unwrap(),
            vec![
                Ok(()),
                Err(SceneGraphError::InvalidTransform),
                Ok(()),
                Ok(()),
                Ok(())
            ]
        );
        assert!(
            scene
                .world_matrix(child)
                .unwrap()
                .transform_point3(glam::Vec3::ZERO)
                .abs_diff_eq(glam::Vec3::X * 3.0, 1e-6)
        );
    }
}

#[cfg(test)]
mod append_tests {
    use super::*;
    #[test]
    fn batch_admission_preserves_both_buffers_on_capacity_and_scene_rejection() {
        let mut scene = SceneGraph::new(1);
        let owner = scene.spawn(None, Transform::default()).unwrap();
        let mut target = SceneCommands::new(&scene, 1);
        let mut batch = SceneCommands::new(&scene, 2);
        batch.push(SceneCommand::SetActive(owner, false)).unwrap();
        batch.push(SceneCommand::SetActive(owner, true)).unwrap();
        assert!(target.append(&mut batch).is_err());
        assert!(target.apply(&mut scene).unwrap().is_empty());
        let other = SceneGraph::new(1);
        let mut foreign = SceneCommands::new(&other, 2);
        assert!(foreign.append(&mut batch).is_err());
        assert_eq!(batch.apply(&mut scene).unwrap().len(), 2);
        assert!(scene.active_in_hierarchy(owner).unwrap());
    }
}
