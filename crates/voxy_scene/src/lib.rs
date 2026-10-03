//! Renderer-independent scene hierarchy with generational object handles.
mod simulation;
pub use simulation::{
    SceneSimulation, SimulationError, SimulationFrame, SimulationLimits, SimulationStepError,
};
mod extraction;
pub use extraction::{
    ExtractedInstance, SceneExtraction, ScopedExtractionError, extraction_schedule,
};
mod picking;
pub use picking::{PickBounds, PickError, PickHit, PickRay, PickResult, PickViewport};

mod file;
pub use file::{SceneFileError, read_scene_file, save_scene_file};

mod history;
pub use history::{SceneEdit, SceneHistory};

mod events;
pub use events::{EventChannel, EventCursor, EventError, EventRead};

mod schedule;
pub use schedule::{
    ResourceAccessDenied, SceneAccessDenied, SceneSystemAccess, ScheduleError, SchedulePlan, SystemAccess, SystemFailure,
    SystemSpec,
};

mod transaction;
pub use transaction::{MutationError, SceneMutation};

mod commands;
mod collection_override;
pub use collection_override::{CollectionEdit, CollectionOverride};
pub use commands::{SceneCommand, SceneCommands};

mod table;
pub use table::{ComponentTable, TableBuildError};

mod associated;
pub use associated::AssociatedData;

mod document;
pub use document::{
    ComponentRegistry, DocumentError, LoadedScene, ObjectId, SceneDocument, SceneObject,
};

mod behavior;
pub use behavior::{Behavior, BehaviorRunner};

mod prefab_document;
pub use prefab_document::{
    ExpandedScene, ObjectOverride, ParentOverride, PrefabInstance, PrefabLimits,
    PrefabSceneDocument, instance_object_id, read_prefab_scene_file, save_prefab_scene_file,
};

mod prefab;
pub use prefab::{Prefab, PrefabError, PrefabNode};

use glam::{Mat4, Quat, Vec3};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Opaque runtime identity, deliberately distinct from durable document IDs.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SceneId(u64);

static NEXT_SCENE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId {
    scene: u64,
    slot: usize,
    generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}
impl Transform {
    /// # Errors
    /// Rejects non-finite values and non-unit rotations.
    pub fn matrix(self) -> Result<Mat4, SceneGraphError> {
        if !self.translation.is_finite()
            || !self.scale.is_finite()
            || !self.rotation.is_finite()
            || (self.rotation.length_squared() - 1.0).abs() > 1e-4
        {
            return Err(SceneGraphError::InvalidTransform);
        }
        let matrix =
            Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation);
        if matrix.is_finite() {
            Ok(matrix)
        } else {
            Err(SceneGraphError::InvalidTransform)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SceneGraphError {
    InvalidNode,
    InvalidTransform,
    Cycle,
    Capacity,
    WorldOverflow,
}
impl std::fmt::Display for SceneGraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scene graph error: {self:?}")
    }
}
impl std::error::Error for SceneGraphError {}

/// Opaque change token. Compare for equality; it is not a durable document identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentRevision(u64);
static NEXT_COMPONENT_REVISION: AtomicU64 = AtomicU64::new(1);
fn next_component_revision() -> ComponentRevision {
    ComponentRevision(
        NEXT_COMPONENT_REVISION
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .expect("component revision space exhausted"),
    )
}
struct Component(Box<dyn Any + Send + Sync>, ComponentRevision);
impl std::fmt::Debug for Component {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Component").finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct Node {
    local: Transform,
    name: String,
    active: bool,
    world: Result<Mat4, SceneGraphError>,
    effective_active: bool,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    components: HashMap<TypeId, Component>,
}
#[derive(Debug)]
struct Slot {
    generation: u64,
    node: Option<Node>,
}

#[derive(Debug)]
pub struct SceneGraph {
    id: u64,
    slots: Vec<Slot>,
    free: Vec<usize>,
    capacity: usize,
    count: usize,
}
impl SceneGraph {
    /// Explicit bound prevents unrestricted object growth.
    ///
    /// # Panics
    /// Panics if the process exhausts the u64 scene identifier space.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let id = NEXT_SCENE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .expect("scene identifier space exhausted");
        Self {
            id,
            slots: Vec::new(),
            free: Vec::new(),
            capacity,
            count: 0,
        }
    }

    fn node(&self, id: NodeId) -> Result<&Node, SceneGraphError> {
        if id.scene != self.id {
            return Err(SceneGraphError::InvalidNode);
        }
        self.slots
            .get(id.slot)
            .filter(|slot| slot.generation == id.generation)
            .and_then(|slot| slot.node.as_ref())
            .ok_or(SceneGraphError::InvalidNode)
    }
    fn node_mut(&mut self, id: NodeId) -> Result<&mut Node, SceneGraphError> {
        if id.scene != self.id {
            return Err(SceneGraphError::InvalidNode);
        }
        self.slots
            .get_mut(id.slot)
            .filter(|slot| slot.generation == id.generation)
            .and_then(|slot| slot.node.as_mut())
            .ok_or(SceneGraphError::InvalidNode)
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.count
    }
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    #[must_use]
    pub const fn identity(&self) -> SceneId {
        SceneId(self.id)
    }

    /// # Errors
    /// Rejects invalid parents/transforms and exceeding the configured capacity.
    pub fn spawn(
        &mut self,
        parent: Option<NodeId>,
        local: Transform,
    ) -> Result<NodeId, SceneGraphError> {
        local.matrix()?;
        if let Some(parent) = parent {
            self.node(parent)?;
        }
        if self.count >= self.capacity {
            return Err(SceneGraphError::Capacity);
        }
        let slot = if let Some(slot) = self.free.pop() {
            slot
        } else {
            self.slots.push(Slot {
                generation: 0,
                node: None,
            });
            self.slots.len() - 1
        };
        let id = NodeId {
            scene: self.id,
            slot,
            generation: self.slots[slot].generation,
        };
        let local_matrix = local.matrix()?;
        let (world, effective_active) = if let Some(parent) = parent {
            let parent = self.node(parent)?;
            (
                parent
                    .world
                    .and_then(|matrix| checked_world(matrix * local_matrix)),
                parent.effective_active,
            )
        } else {
            (Ok(local_matrix), true)
        };
        self.slots[slot].node = Some(Node {
            local,
            name: String::new(),
            active: true,
            world,
            effective_active,
            parent,
            children: Vec::new(),
            components: HashMap::new(),
        });
        if let Some(parent) = parent {
            self.node_mut(parent)?.children.push(id);
        }
        self.count += 1;
        Ok(id)
    }

    /// # Errors
    /// Rejects stale or foreign handles.
    pub fn name(&self, id: NodeId) -> Result<&str, SceneGraphError> {
        Ok(&self.node(id)?.name)
    }

    /// Names are labels and need not be unique.
    /// # Errors
    /// Rejects stale or foreign handles.
    pub fn set_name(&mut self, id: NodeId, name: impl Into<String>) -> Result<(), SceneGraphError> {
        self.node_mut(id)?.name = name.into();
        Ok(())
    }

    /// Returns matching live nodes in slot order, including inactive nodes.
    pub fn find_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = NodeId> + 'a {
        self.nodes()
            .filter_map(move |(id, _, _)| (self.name(id).ok() == Some(name)).then_some(id))
    }

    /// # Errors
    /// Rejects stale or foreign handles.
    pub fn active_self(&self, id: NodeId) -> Result<bool, SceneGraphError> {
        Ok(self.node(id)?.active)
    }

    /// Changes local activity without overwriting descendants' local flags.
    /// # Errors
    /// Rejects stale or foreign handles.
    pub fn set_active(&mut self, id: NodeId, active: bool) -> Result<(), SceneGraphError> {
        if self.node(id)?.active != active {
            self.node_mut(id)?.active = active;
            self.refresh_subtree(id)?;
        }
        Ok(())
    }

    /// Effective activity requires the node and every ancestor to be active.
    /// # Errors
    /// Rejects stale or foreign handles.
    pub fn active_in_hierarchy(&self, id: NodeId) -> Result<bool, SceneGraphError> {
        Ok(self.node(id)?.effective_active)
    }

    /// Visits effectively active component owners in slot order.
    /// Inactive nodes retain their components and transforms.
    pub fn active_components<T: Any + Send + Sync>(
        &self,
    ) -> impl Iterator<Item = (NodeId, &T)> + '_ {
        self.components::<T>()
            .filter(|(id, _)| self.active_in_hierarchy(*id) == Ok(true))
    }

    /// Visits effectively active owners containing both required component types.
    /// References are borrowed from the current graph, so replacement/removal cannot
    /// invalidate an outstanding iterator. Slot order is stable; no membership cache
    /// or associated-data allocation is introduced.
    pub fn active_components_with<A: Any + Send + Sync, B: Any + Send + Sync>(
        &self,
    ) -> impl Iterator<Item = (NodeId, &A, &B)> + '_ {
        self.active_components::<A>().filter_map(|(owner, first)| {
            self.component::<B>(owner)
                .ok()
                .flatten()
                .map(|second| (owner, first, second))
        })
    }

    /// Attaches one value per Rust type; replacing a component returns its old value.
    /// Components are owned by the node and dropped when its subtree is removed.
    /// # Panics
    /// Panics if the process exhausts the u64 component revision space.
    /// # Errors
    /// Rejects stale or foreign handles before attaching the value.
    pub fn insert_component<T: Any + Send + Sync>(
        &mut self,
        id: NodeId,
        value: T,
    ) -> Result<Option<T>, SceneGraphError> {
        let old = self.node_mut(id)?.components.insert(
            TypeId::of::<T>(),
            Component(Box::new(value), next_component_revision()),
        );
        Ok(old
            .and_then(|value| value.0.downcast::<T>().ok())
            .map(|value| *value))
    }

    /// # Errors
    /// Rejects stale or foreign handles; a missing component returns `None`.
    pub fn component<T: Any + Send + Sync>(
        &self,
        id: NodeId,
    ) -> Result<Option<&T>, SceneGraphError> {
        Ok(self
            .node(id)?
            .components
            .get(&TypeId::of::<T>())
            .and_then(|value| value.0.downcast_ref()))
    }

    /// Reads a component's current change token without changing it.
    /// Interior mutation through shared references is not tracked; callers must
    /// request mutable access when publishing changes to such components.
    /// # Errors
    /// Rejects stale or foreign owners; absent components return None.
    pub fn component_revision<T: Any + Send + Sync>(
        &self,
        id: NodeId,
    ) -> Result<Option<ComponentRevision>, SceneGraphError> {
        Ok(self
            .node(id)?
            .components
            .get(&TypeId::of::<T>())
            .map(|value| value.1))
    }

    /// Obtaining mutable access conservatively changes the revision even if the
    /// caller performs no write. Missing components do not consume a revision.
    /// # Panics
    /// Panics if the process exhausts the u64 component revision space.
    /// # Errors
    /// Rejects stale or foreign handles; a missing component returns `None`.
    pub fn component_mut<T: Any + Send + Sync>(
        &mut self,
        id: NodeId,
    ) -> Result<Option<&mut T>, SceneGraphError> {
        let Some(value) = self.node_mut(id)?.components.get_mut(&TypeId::of::<T>()) else {
            return Ok(None);
        };
        value.1 = next_component_revision();
        Ok(value.0.downcast_mut())
    }

    /// Detaches and returns a component without deleting the node.
    /// # Errors
    /// Rejects stale or foreign handles; a missing component returns `None`.
    pub fn remove_component<T: Any + Send + Sync>(
        &mut self,
        id: NodeId,
    ) -> Result<Option<T>, SceneGraphError> {
        Ok(self
            .node_mut(id)?
            .components
            .remove(&TypeId::of::<T>())
            .and_then(|value| value.0.downcast::<T>().ok())
            .map(|value| *value))
    }

    /// Visits nodes with a component in stable slot order.
    pub fn components<T: Any + Send + Sync>(&self) -> impl Iterator<Item = (NodeId, &T)> + '_ {
        self.nodes().filter_map(|(id, _, _)| {
            self.component::<T>(id)
                .ok()
                .flatten()
                .map(|value| (id, value))
        })
    }

    /// Enumerates live nodes in slot order, with local transforms and parents.
    pub fn nodes(&self) -> impl Iterator<Item = (NodeId, Transform, Option<NodeId>)> + '_ {
        self.slots.iter().enumerate().filter_map(|(slot, entry)| {
            entry.node.as_ref().map(|node| {
                (
                    NodeId {
                        scene: self.id,
                        slot,
                        generation: entry.generation,
                    },
                    node.local,
                    node.parent,
                )
            })
        })
    }

    /// # Errors
    /// Rejects stale or foreign handles.
    pub fn local(&self, id: NodeId) -> Result<Transform, SceneGraphError> {
        Ok(self.node(id)?.local)
    }

    /// # Errors
    /// Rejects invalid handles and invalid transforms before mutation.
    pub fn set_local(&mut self, id: NodeId, local: Transform) -> Result<(), SceneGraphError> {
        local.matrix()?;
        if self.node(id)?.local != local {
            self.node_mut(id)?.local = local;
            self.refresh_subtree(id)?;
        }
        Ok(())
    }

    /// Applies a transform batch with last-write-wins duplicates. Refreshes each
    /// affected subtree once, collapsing overlapping ancestor/descendant edits.
    /// # Errors
    /// Validates every handle/value before mutation; invalid input leaves the graph unchanged.
    pub fn set_locals(&mut self, edits: &[(NodeId, Transform)]) -> Result<(), SceneGraphError> {
        if edits.is_empty() {
            return Ok(());
        }
        // A single edit cannot overlap another dirty subtree. Avoid scene-sized
        // scratch allocation and ancestor scanning for this common editor path.
        if let [(id, local)] = edits {
            return self.set_local(*id, *local);
        }
        for &(id, local) in edits {
            self.node(id)?;
            local.matrix()?;
        }
        let mut values = vec![None; self.slots.len()];
        let mut unique = Vec::with_capacity(edits.len());
        for &(id, local) in edits {
            if values[id.slot].is_none() {
                unique.push(id);
            }
            values[id.slot] = Some(local);
        }
        let mut dirty = vec![false; self.slots.len()];
        for &id in &unique {
            if let Some(local) = values[id.slot]
                && self.node(id)?.local != local
            {
                self.node_mut(id)?.local = local;
                dirty[id.slot] = true;
            }
        }
        let mut roots = Vec::new();
        for id in unique {
            if !dirty[id.slot] {
                continue;
            }
            let mut ancestor = self.node(id)?.parent;
            let mut covered = false;
            while let Some(parent) = ancestor {
                if dirty[parent.slot] {
                    covered = true;
                    break;
                }
                ancestor = self.node(parent)?.parent;
            }
            if !covered {
                roots.push(id);
            }
        }
        for root in roots {
            self.refresh_subtree(root)?;
        }
        Ok(())
    }

    /// Returns the immediate parent of a live scene node.
    /// # Errors
    /// Rejects stale or foreign handles.
    pub fn parent(&self, id: NodeId) -> Result<Option<NodeId>, SceneGraphError> {
        Ok(self.node(id)?.parent)
    }

    /// Changes parent while preserving the local transform.
    /// # Errors
    /// Rejects stale/foreign handles and cycles without changing the hierarchy.
    pub fn reparent(&mut self, id: NodeId, parent: Option<NodeId>) -> Result<(), SceneGraphError> {
        let old_parent = self.node(id)?.parent;
        let mut ancestor = parent;
        while let Some(current) = ancestor {
            if current == id {
                return Err(SceneGraphError::Cycle);
            }
            ancestor = self.node(current)?.parent;
        }
        if old_parent == parent {
            return Ok(());
        }
        if let Some(old) = old_parent {
            self.node_mut(old)?.children.retain(|&child| child != id);
        }
        if let Some(new) = parent {
            self.node_mut(new)?.children.push(id);
        }
        self.node_mut(id)?.parent = parent;
        self.refresh_subtree(id)?;
        Ok(())
    }

    /// # Errors
    /// Rejects stale/foreign handles and floating-point overflow in composed transforms.
    pub fn world_matrix(&self, id: NodeId) -> Result<Mat4, SceneGraphError> {
        self.node(id)?.world
    }

    // Eager propagation preserves constant-time queries for every consumer.
    // Iterative traversal supports deep hierarchies without stack recursion.
    fn refresh_subtree(&mut self, root: NodeId) -> Result<(), SceneGraphError> {
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            let node = self.node(id)?;
            let local = node.local.matrix()?;
            let active = node.active;
            let (world, effective_active) = if let Some(parent) = node.parent {
                let parent = self.node(parent)?;
                (
                    parent
                        .world
                        .and_then(|matrix| checked_world(matrix * local)),
                    active && parent.effective_active,
                )
            } else {
                (Ok(local), active)
            };
            let node = self.node_mut(id)?;
            node.world = world;
            node.effective_active = effective_active;
            pending.extend(node.children.iter().copied());
        }
        Ok(())
    }

    /// Deletes the complete subtree and invalidates all its handles.
    ///
    /// # Panics
    /// Panics if internal hierarchy invariants are broken; the public mutation
    /// methods preserve these invariants.
    /// # Errors
    /// Rejects invalid handles before mutation.
    pub fn remove_subtree(&mut self, root: NodeId) -> Result<usize, SceneGraphError> {
        let parent = self.node(root)?.parent;
        if let Some(parent) = parent {
            self.node_mut(parent)?.children.retain(|&id| id != root);
        }
        let mut pending = vec![root];
        let mut removed = 0;
        while let Some(id) = pending.pop() {
            let slot = &mut self.slots[id.slot];
            let node = slot
                .node
                .take()
                .expect("validated hierarchy contains live children");
            pending.extend(node.children);
            // Exhausted generations retire the slot permanently.
            if let Some(generation) = slot.generation.checked_add(1) {
                slot.generation = generation;
                self.free.push(id.slot);
            }
            removed += 1;
        }
        self.count -= removed;
        Ok(removed)
    }
}

fn checked_world(matrix: Mat4) -> Result<Mat4, SceneGraphError> {
    if matrix.is_finite() {
        Ok(matrix)
    } else {
        Err(SceneGraphError::WorldOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn position(x: f32) -> Transform {
        Transform {
            translation: Vec3::new(x, 0.0, 0.0),
            ..Default::default()
        }
    }

    #[test]
    fn parent_motion_propagates_and_reparent_is_local() {
        let mut graph = SceneGraph::new(4);
        let a = graph
            .spawn(
                None,
                Transform {
                    scale: Vec3::splat(2.0),
                    ..position(3.0)
                },
            )
            .unwrap();
        let b = graph.spawn(Some(a), position(4.0)).unwrap();
        assert!(
            (graph
                .world_matrix(b)
                .unwrap()
                .transform_point3(Vec3::ZERO)
                .x
                - 11.0)
                .abs()
                < 1e-5
        );
        graph.set_local(a, position(9.0)).unwrap();
        assert!(
            (graph
                .world_matrix(b)
                .unwrap()
                .transform_point3(Vec3::ZERO)
                .x
                - 13.0)
                .abs()
                < 1e-5
        );
        graph.reparent(b, None).unwrap();
        assert!(
            (graph
                .world_matrix(b)
                .unwrap()
                .transform_point3(Vec3::ZERO)
                .x
                - 4.0)
                .abs()
                < 1e-5
        );
    }

    #[test]
    fn cycles_foreign_and_stale_handles_cannot_mutate() {
        let mut graph = SceneGraph::new(3);
        let root = graph.spawn(None, position(1.0)).unwrap();
        let child = graph.spawn(Some(root), position(2.0)).unwrap();
        let before = graph.world_matrix(child).unwrap();
        assert_eq!(
            graph.reparent(root, Some(child)),
            Err(SceneGraphError::Cycle)
        );
        assert_eq!(graph.world_matrix(child).unwrap(), before);
        let foreign = SceneGraph::new(1).spawn(None, position(0.0)).unwrap();
        assert_eq!(
            graph.reparent(child, Some(foreign)),
            Err(SceneGraphError::InvalidNode)
        );
        assert_eq!(graph.remove_subtree(root).unwrap(), 2);
        let replacement = graph.spawn(None, position(0.0)).unwrap();
        assert_ne!(replacement, child);
        assert_eq!(
            graph.set_local(child, position(99.0)),
            Err(SceneGraphError::InvalidNode)
        );
        assert_eq!(graph.world_matrix(root), Err(SceneGraphError::InvalidNode));
    }

    #[test]
    fn invalid_transform_and_capacity_leave_scene_intact() {
        let mut graph = SceneGraph::new(1);
        let id = graph.spawn(None, position(1.0)).unwrap();
        assert_eq!(
            graph.spawn(None, position(2.0)),
            Err(SceneGraphError::Capacity)
        );
        assert_eq!(
            graph.set_local(id, position(f32::NAN)),
            Err(SceneGraphError::InvalidTransform)
        );
        assert_eq!(graph.len(), 1);
        assert!(graph.world_matrix(id).unwrap().is_finite());
    }
    #[test]
    fn deep_subtree_removal_is_iterative_and_reusable() {
        let mut graph = SceneGraph::new(8192);
        let root = graph.spawn(None, Transform::default()).unwrap();
        let mut tip = root;
        for _ in 1..8192 {
            tip = graph.spawn(Some(tip), Transform::default()).unwrap();
        }
        assert_eq!(graph.world_matrix(tip).unwrap(), Mat4::IDENTITY);
        assert_eq!(graph.nodes().count(), 8192);
        assert_eq!(graph.remove_subtree(root).unwrap(), 8192);
        assert!(graph.is_empty());
        assert_eq!(graph.nodes().count(), 0);
        let reused = graph.spawn(None, Transform::default()).unwrap();
        assert_ne!(tip, reused);
        assert_eq!(graph.world_matrix(tip), Err(SceneGraphError::InvalidNode));
    }

    #[test]
    fn composed_overflow_is_reported_without_changing_nodes() {
        let mut graph = SceneGraph::new(2);
        let root = graph
            .spawn(
                None,
                Transform {
                    scale: Vec3::splat(f32::MAX),
                    ..Default::default()
                },
            )
            .unwrap();
        let child = graph
            .spawn(
                Some(root),
                Transform {
                    scale: Vec3::splat(2.0),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            graph.world_matrix(child),
            Err(SceneGraphError::WorldOverflow)
        );
        assert_eq!(graph.len(), 2);
        graph.set_local(root, Transform::default()).unwrap();
        assert!(graph.world_matrix(child).unwrap().is_finite());
    }
}

#[cfg(test)]
mod component_tests {
    use super::*;
    #[test]
    fn gameplay_components_replace_query_mutate_and_detach() {
        let mut scene = SceneGraph::new(3);
        let player = scene.spawn(None, Transform::default()).unwrap();
        let other = scene.spawn(None, Transform::default()).unwrap();
        assert_eq!(scene.insert_component(player, 100_u32), Ok(None));
        assert_eq!(scene.insert_component(player, 80_u32), Ok(Some(100)));
        scene
            .insert_component(player, String::from("player"))
            .unwrap();
        scene.insert_component(other, 20_u32).unwrap();
        *scene.component_mut::<u32>(player).unwrap().unwrap() -= 10;
        assert_eq!(
            scene
                .components::<u32>()
                .map(|(_, hp)| *hp)
                .collect::<Vec<_>>(),
            vec![70, 20]
        );
        assert_eq!(scene.remove_component::<u32>(player), Ok(Some(70)));
        assert_eq!(scene.component::<u32>(player), Ok(None));
        assert_eq!(
            scene.component::<String>(player).unwrap().unwrap(),
            "player"
        );
    }
    #[test]
    fn subtree_drops_components_and_reused_handles_cannot_access_them() {
        struct Lifetime(std::sync::Arc<std::sync::atomic::AtomicUsize>);
        impl Drop for Lifetime {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        }
        let dropped = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut scene = SceneGraph::new(2);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(root), Transform::default()).unwrap();
        for id in [root, child] {
            scene
                .insert_component(id, Lifetime(dropped.clone()))
                .unwrap();
        }
        let foreign = SceneGraph::new(1)
            .spawn(None, Transform::default())
            .unwrap();
        assert_eq!(
            scene.insert_component(foreign, 5_u32),
            Err(SceneGraphError::InvalidNode)
        );
        scene.remove_subtree(root).unwrap();
        assert_eq!(dropped.load(Ordering::Relaxed), 2);
        let replacement = scene.spawn(None, Transform::default()).unwrap();
        assert_eq!(
            scene.component::<u32>(child),
            Err(SceneGraphError::InvalidNode)
        );
        assert_eq!(scene.component::<u32>(replacement), Ok(None));
    }
}

#[cfg(test)]
mod activity_tests {
    use super::*;
    #[test]
    fn hierarchy_activity_preserves_child_state_and_changes_with_parenting() {
        let mut scene = SceneGraph::new(3);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(root), Transform::default()).unwrap();
        let leaf = scene.spawn(Some(child), Transform::default()).unwrap();
        for id in [root, child, leaf] {
            scene.insert_component(id, 1_u32).unwrap();
        }
        scene.set_active(root, false).unwrap();
        assert_eq!(scene.active_components::<u32>().count(), 0);
        assert_eq!(scene.active_self(child), Ok(true));
        scene.set_active(child, false).unwrap();
        scene.set_active(root, true).unwrap();
        assert_eq!(scene.active_components::<u32>().count(), 1);
        scene.reparent(leaf, None).unwrap();
        assert_eq!(scene.active_in_hierarchy(leaf), Ok(true));
        assert_eq!(scene.components::<u32>().count(), 3);
        scene.remove_subtree(child).unwrap();
        assert_eq!(
            scene.set_active(child, true),
            Err(SceneGraphError::InvalidNode)
        );
    }
    #[test]
    fn names_are_nonunique_labels_and_reused_slots_have_clean_metadata() {
        let mut scene = SceneGraph::new(2);
        let a = scene.spawn(None, Transform::default()).unwrap();
        let b = scene.spawn(None, Transform::default()).unwrap();
        scene.set_name(a, "enemy").unwrap();
        scene.set_name(b, "enemy").unwrap();
        assert_eq!(scene.find_named("enemy").collect::<Vec<_>>(), vec![a, b]);
        scene.set_active(a, false).unwrap();
        scene.remove_subtree(a).unwrap();
        let replacement = scene.spawn(None, Transform::default()).unwrap();
        assert_eq!(scene.name(replacement), Ok(""));
        assert_eq!(scene.active_self(replacement), Ok(true));
        assert_eq!(scene.name(a), Err(SceneGraphError::InvalidNode));
        assert_eq!(scene.find_named("enemy").collect::<Vec<_>>(), vec![b]);
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    fn oracle(scene: &SceneGraph, id: NodeId) -> Mat4 {
        let mut ancestors = Vec::new();
        let mut current = Some(id);
        while let Some(id) = current {
            let node = scene.node(id).unwrap();
            ancestors.push(node.local.matrix().unwrap());
            current = node.parent;
        }
        ancestors
            .into_iter()
            .rev()
            .fold(Mat4::IDENTITY, |world, local| world * local)
    }
    #[test]
    fn cache_matches_full_recompute_after_edits_reparenting_and_reuse() {
        let mut scene = SceneGraph::new(128);
        let mut ids = Vec::new();
        for i in 0..128 {
            let parent = if i == 0 { None } else { Some(ids[(i - 1) / 2]) };
            ids.push(scene.spawn(parent, Transform::default()).unwrap());
        }
        for round in 0..80 {
            let id = ids[(round * 17) % ids.len()];
            scene
                .set_local(
                    id,
                    Transform {
                        translation: Vec3::new(0.125, 1.0, -0.5),
                        rotation: Quat::from_rotation_y(0.3),
                        scale: Vec3::splat(1.01),
                    },
                )
                .unwrap();
            scene.set_active(id, round % 3 != 0).unwrap();
            for (id, _, _) in scene.nodes() {
                assert!(
                    scene
                        .world_matrix(id)
                        .unwrap()
                        .abs_diff_eq(oracle(&scene, id), 1e-4)
                );
                let mut expected = true;
                let mut current = Some(id);
                while let Some(id) = current {
                    let node = scene.node(id).unwrap();
                    expected &= node.active;
                    current = node.parent;
                }
                assert_eq!(scene.active_in_hierarchy(id), Ok(expected));
            }
        }
        scene.reparent(ids[1], None).unwrap();
        assert!(
            scene
                .world_matrix(ids[127])
                .unwrap()
                .abs_diff_eq(oracle(&scene, ids[127]), 1e-4)
        );
        let leaf = ids[127];
        scene.remove_subtree(leaf).unwrap();
        let new = scene.spawn(None, Transform::default()).unwrap();
        assert_eq!(scene.world_matrix(leaf), Err(SceneGraphError::InvalidNode));
        assert_eq!(scene.world_matrix(new), Ok(Mat4::IDENTITY));
        assert_eq!(scene.active_in_hierarchy(new), Ok(true));
    }
    #[test]
    fn single_batch_edit_preserves_descendants_and_rejects_stale_handles() {
        let mut scene = SceneGraph::new(3);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(root), Transform::default()).unwrap();
        let leaf = scene.spawn(Some(child), Transform::default()).unwrap();
        let moved = Transform {
            translation: Vec3::Y,
            ..Transform::default()
        };
        scene.set_locals(&[(child, moved)]).unwrap();
        assert_eq!(
            scene
                .world_matrix(leaf)
                .unwrap()
                .transform_point3(Vec3::ZERO),
            Vec3::Y
        );
        let before = scene.world_matrix(leaf).unwrap();
        let invalid = Transform {
            translation: Vec3::splat(f32::NAN),
            ..moved
        };
        assert!(scene.set_locals(&[(child, invalid)]).is_err());
        assert_eq!(scene.world_matrix(leaf).unwrap(), before);
        scene.remove_subtree(child).unwrap();
        assert_eq!(
            scene.set_locals(&[(child, moved)]),
            Err(SceneGraphError::InvalidNode)
        );
    }

    #[test]
    fn batch_edits_match_oracle_and_invalid_batch_is_atomic() {
        let mut scene = SceneGraph::new(3);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(root), Transform::default()).unwrap();
        let leaf = scene.spawn(Some(child), Transform::default()).unwrap();
        let x = Transform {
            translation: Vec3::X,
            ..Transform::default()
        };
        let y = Transform {
            translation: Vec3::Y,
            ..Transform::default()
        };
        scene
            .set_locals(&[(child, x), (root, x), (child, y), (leaf, x)])
            .unwrap();
        assert!(
            scene
                .world_matrix(leaf)
                .unwrap()
                .abs_diff_eq(oracle(&scene, leaf), 1e-6)
        );
        assert!(
            scene
                .world_matrix(leaf)
                .unwrap()
                .transform_point3(Vec3::ZERO)
                .abs_diff_eq(Vec3::new(2.0, 1.0, 0.0), 1e-6)
        );
        let before = scene.world_matrix(leaf).unwrap();
        let bad = Transform {
            translation: Vec3::splat(f32::NAN),
            ..Transform::default()
        };
        assert!(scene.set_locals(&[(root, y), (child, bad)]).is_err());
        assert_eq!(scene.world_matrix(leaf).unwrap(), before);
    }
}
