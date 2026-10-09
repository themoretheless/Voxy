//! Atomic structural batches, validated on a component-free hierarchy shadow.
use crate::{Node, NodeId, SceneGraph, SceneGraphError, Slot, Transform};
#[derive(Clone, Debug)]
pub enum SceneMutation {
    SetLocal(NodeId, Transform),
    SpawnNamed(String, Transform),
    RemoveSubtree(NodeId),
}
#[derive(Debug)]
pub struct MutationError {
    pub index: usize,
    pub cause: SceneGraphError,
}
impl std::fmt::Display for MutationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "command {}: {}", self.index + 1, self.cause)
    }
}
impl std::error::Error for MutationError {}
impl SceneGraph {
    fn hierarchy_shadow(&self) -> Self {
        Self {
            id: self.id,
            capacity: self.capacity,
            count: self.count,
            free: self.free.clone(),
            slots: self
                .slots
                .iter()
                .map(|slot| Slot {
                    generation: slot.generation,
                    node: slot.node.as_ref().map(|n| Node {
                        local: n.local,
                        name: n.name.clone(),
                        active: n.active,
                        world: n.world,
                        effective_active: n.effective_active,
                        parent: n.parent,
                        children: n.children.clone(),
                        components: crate::ComponentMap::default(),
                    }),
                })
                .collect(),
        }
    }
    fn mutate(&mut self, command: &SceneMutation) -> Result<(), SceneGraphError> {
        match command {
            SceneMutation::SetLocal(id, t) => self.set_local(*id, *t),
            SceneMutation::SpawnNamed(name, t) => {
                let id = self.spawn(None, *t)?;
                self.set_name(id, name.clone())
            }
            SceneMutation::RemoveSubtree(id) => self.remove_subtree(*id).map(|_| ()),
        }
    }
    /// Validates the entire ordered batch before touching live data/components.
    /// No callbacks run between validation and commit; handles, free-slot order and
    /// capacity are identical in both passes. Failed preflight consumes no IDs.
    /// # Errors
    /// Reports the first invalid command and leaves the scene unchanged.
    /// # Panics
    /// Panics if structural commit diverges from the successful preflight despite
    /// no intervening mutation or callbacks; this signals an internal invariant bug.
    pub fn apply_atomic(&mut self, commands: &[SceneMutation]) -> Result<(), MutationError> {
        if commands.is_empty() {
            return Ok(());
        }
        // Transform-only batches cannot change structure, so validating each command
        // in order gives the same first failure as the shadow replay without copying
        // the whole graph; `set_locals` then commits them with last-write-wins.
        if commands
            .iter()
            .all(|command| matches!(command, SceneMutation::SetLocal(..)))
        {
            let mut edits = Vec::with_capacity(commands.len());
            for (index, command) in commands.iter().enumerate() {
                let SceneMutation::SetLocal(id, local) = command else {
                    continue;
                };
                self.node(*id)
                    .map(|_| ())
                    .and_then(|()| local.matrix().map(|_| ()))
                    .map_err(|cause| MutationError { index, cause })?;
                edits.push((*id, *local));
            }
            self.set_locals(&edits)
                .expect("validated transform batch commits without structural change");
            return Ok(());
        }
        let mut shadow = self.hierarchy_shadow();
        for (index, command) in commands.iter().enumerate() {
            shadow
                .mutate(command)
                .map_err(|cause| MutationError { index, cause })?;
        }
        for command in commands {
            self.mutate(command)
                .expect("preflight and commit use identical structural state without callbacks");
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capacity_failure_preserves_transforms_components_and_handle_sequence() {
        let mut scene = SceneGraph::new(2);
        let root = scene.spawn(None, Transform::default()).unwrap();
        scene.insert_component(root, vec![1u32, 2]).unwrap();
        let transform = Transform {
            translation: glam::Vec3::X,
            ..Transform::default()
        };
        assert!(
            scene
                .apply_atomic(&[
                    SceneMutation::SetLocal(root, transform),
                    SceneMutation::SpawnNamed("first".into(), transform),
                    SceneMutation::SpawnNamed("overflow".into(), transform)
                ])
                .is_err()
        );
        assert_eq!(scene.local(root).unwrap(), Transform::default());
        assert_eq!(scene.len(), 1);
        assert_eq!(
            scene.component::<Vec<u32>>(root).unwrap(),
            Some(&vec![1, 2])
        );
        let next = scene.spawn(None, Transform::default()).unwrap();
        assert_eq!(next.slot, 1);
        assert_eq!(next.generation, 0);
    }
    #[test]
    fn delete_then_stale_write_rolls_back_subtree_and_payloads() {
        let mut scene = SceneGraph::new(3);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(root), Transform::default()).unwrap();
        scene
            .insert_component(child, String::from("unique"))
            .unwrap();
        let error = scene
            .apply_atomic(&[
                SceneMutation::RemoveSubtree(root),
                SceneMutation::SetLocal(child, Transform::default()),
            ])
            .unwrap_err();
        assert_eq!(error.index, 1);
        assert_eq!(scene.len(), 2);
        assert_eq!(scene.parent(child).unwrap(), Some(root));
        assert_eq!(scene.component::<String>(child).unwrap().unwrap(), "unique");
    }
    #[test]
    fn ordered_delete_can_free_capacity_for_creation() {
        let mut scene = SceneGraph::new(1);
        let old = scene.spawn(None, Transform::default()).unwrap();
        scene
            .apply_atomic(&[
                SceneMutation::RemoveSubtree(old),
                SceneMutation::SpawnNamed("new".into(), Transform::default()),
            ])
            .unwrap();
        let new = scene.find_named("new").next().unwrap();
        assert_ne!(old, new);
        assert_eq!(scene.len(), 1);
    }
}
