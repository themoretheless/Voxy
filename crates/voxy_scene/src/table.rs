//! Contiguous typed storage for batch simulation, separate from authoring metadata.
use crate::{NodeId, SceneGraph, SceneGraphError};
use std::any::Any;

/// Failure before publication of a replacement associated-data table.
#[derive(Debug, PartialEq)]
pub enum TableBuildError<E> {
    ForeignScene,
    Build { owner: NodeId, error: E },
}

impl<E: std::fmt::Display> std::fmt::Display for TableBuildError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ForeignScene => write!(f, "associated data belongs to a different scene"),
            Self::Build { owner, error } => {
                write!(
                    f,
                    "associated data construction failed for {owner:?}: {error}"
                )
            }
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for TableBuildError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ForeignScene => None,
            Self::Build { error, .. } => Some(error),
        }
    }
}

/// Dense values and owners plus a sparse slot index. Removal may change row order.
/// This table belongs to one scene; it does not store values inside scene nodes.
/// Call synchronize at structural barriers to release values of deleted owners.
#[derive(Debug)]
pub struct ComponentTable<T> {
    scene: u64,
    owners: Vec<NodeId>,
    values: Vec<T>,
    sparse: Vec<Option<(u64, usize)>>,
}
impl<T> ComponentTable<T> {
    /// Rebuilds associated data for active owners with both required components.
    /// The factory receives the previous row for inspection, without mutable access.
    /// All rows are staged before publication; an error leaves the entire old table
    /// intact. Success drops removed/inactive rows and replaces surviving rows.
    /// Call at a structural barrier. Factory side effects are not rolled back.
    /// # Errors
    /// Rejects foreign scenes before invoking the factory, or returns its first error.
    pub fn rebuild_active_with<A, B, E>(
        &mut self,
        scene: &SceneGraph,
        mut build: impl FnMut(NodeId, &A, &B, Option<&T>) -> Result<T, E>,
    ) -> Result<(), TableBuildError<E>>
    where
        A: Any + Send + Sync,
        B: Any + Send + Sync,
    {
        if scene.id != self.scene {
            return Err(TableBuildError::ForeignScene);
        }
        let mut staged = Self::new(scene);
        for (owner, first, second) in scene.active_components_with::<A, B>() {
            let previous = self.row(owner).map(|row| &self.values[row]);
            let value = build(owner, first, second, previous)
                .map_err(|error| TableBuildError::Build { owner, error })?;
            // Owners come from this immutable scene borrow and cannot become stale.
            staged.push_row(owner, value);
        }
        *self = staged;
        Ok(())
    }

    #[must_use]
    pub fn new(scene: &SceneGraph) -> Self {
        Self {
            scene: scene.id,
            owners: Vec::new(),
            values: Vec::new(),
            sparse: Vec::new(),
        }
    }
    fn validate(&self, scene: &SceneGraph, owner: NodeId) -> Result<(), SceneGraphError> {
        if scene.id != self.scene {
            return Err(SceneGraphError::InvalidNode);
        }
        scene.node(owner)?;
        Ok(())
    }
    fn row(&self, owner: NodeId) -> Option<usize> {
        if owner.scene != self.scene {
            return None;
        }
        self.sparse
            .get(owner.slot)
            .copied()
            .flatten()
            .filter(|(generation, _)| *generation == owner.generation)
            .map(|(_, row)| row)
    }
    fn remove_row(&mut self, row: usize) -> T {
        let old = self.owners.swap_remove(row);
        self.sparse[old.slot] = None;
        let value = self.values.swap_remove(row);
        if let Some(moved) = self.owners.get(row) {
            self.sparse[moved.slot] = Some((moved.generation, row));
        }
        value
    }
    fn push_row(&mut self, owner: NodeId, value: T) {
        if self.sparse.len() <= owner.slot {
            self.sparse.resize(owner.slot + 1, None);
        }
        let row = self.values.len();
        self.owners.push(owner);
        self.values.push(value);
        self.sparse[owner.slot] = Some((owner.generation, row));
    }
    /// # Errors
    /// Rejects foreign scenes and invalid owners before changing the table.
    pub fn insert(
        &mut self,
        scene: &SceneGraph,
        owner: NodeId,
        value: T,
    ) -> Result<Option<T>, SceneGraphError> {
        self.validate(scene, owner)?;
        if let Some(row) = self.row(owner) {
            return Ok(Some(std::mem::replace(&mut self.values[row], value)));
        }
        if self.sparse.len() <= owner.slot {
            self.sparse.resize(owner.slot + 1, None);
        }
        // A reused slot cannot inherit the previous generation's component.
        if let Some((_, old_row)) = self.sparse[owner.slot] {
            self.remove_row(old_row);
        }
        self.push_row(owner, value);
        Ok(None)
    }
    /// # Errors
    /// Rejects foreign scenes and stale owners.
    pub fn get(&self, scene: &SceneGraph, owner: NodeId) -> Result<Option<&T>, SceneGraphError> {
        self.validate(scene, owner)?;
        Ok(self.row(owner).map(|row| &self.values[row]))
    }
    /// # Errors
    /// Rejects foreign scenes and stale owners.
    pub fn get_mut(
        &mut self,
        scene: &SceneGraph,
        owner: NodeId,
    ) -> Result<Option<&mut T>, SceneGraphError> {
        self.validate(scene, owner)?;
        Ok(self.row(owner).map(|row| &mut self.values[row]))
    }
    /// # Errors
    /// Rejects foreign scenes and stale owners.
    pub fn remove(
        &mut self,
        scene: &SceneGraph,
        owner: NodeId,
    ) -> Result<Option<T>, SceneGraphError> {
        self.validate(scene, owner)?;
        Ok(self.row(owner).map(|row| self.remove_row(row)))
    }
    /// Drops deleted-owner values and repairs dense indices at a structural barrier.
    /// # Errors
    /// Rejects a foreign scene without deleting any values.
    pub fn synchronize(&mut self, scene: &SceneGraph) -> Result<usize, SceneGraphError> {
        if scene.id != self.scene {
            return Err(SceneGraphError::InvalidNode);
        }
        let mut row = 0;
        let mut removed = 0;
        while row < self.owners.len() {
            if scene.node(self.owners[row]).is_err() {
                self.remove_row(row);
                removed += 1;
            } else {
                row += 1;
            }
        }
        Ok(removed)
    }
    /// Visits live rows, optionally filtering by inherited activity.
    /// The graph borrow prevents structural mutation during the batch.
    /// # Errors
    /// Rejects a foreign scene.
    pub fn query_mut<'a>(
        &'a mut self,
        scene: &'a SceneGraph,
        active_only: bool,
    ) -> Result<impl Iterator<Item = (NodeId, &'a mut T)> + 'a, SceneGraphError> {
        if scene.id != self.scene {
            return Err(SceneGraphError::InvalidNode);
        }
        Ok(self
            .owners
            .iter()
            .copied()
            .zip(self.values.iter_mut())
            .filter(move |(id, _)| {
                if active_only {
                    scene.active_in_hierarchy(*id) == Ok(true)
                } else {
                    scene.node(*id).is_ok()
                }
            }))
    }
    /// Visits live rows through shared borrows, optionally filtering activity.
    /// Dense row order may change after removal. The scene borrow prevents
    /// structural changes while the iterator is alive.
    /// # Errors
    /// Rejects foreign scenes before producing an iterator.
    pub fn query<'a>(
        &'a self,
        scene: &'a SceneGraph,
        active_only: bool,
    ) -> Result<impl Iterator<Item = (NodeId, &'a T)> + 'a, SceneGraphError> {
        if scene.id != self.scene {
            return Err(SceneGraphError::InvalidNode);
        }
        Ok(self
            .owners
            .iter()
            .copied()
            .zip(self.values.iter())
            .filter(move |(id, _)| {
                if active_only {
                    scene.active_in_hierarchy(*id) == Ok(true)
                } else {
                    scene.node(*id).is_ok()
                }
            }))
    }

    /// Releases all owned rows without modifying the scene or its components.
    /// Repeated calls on an empty table have no effect.
    pub fn clear(&mut self) {
        self.owners.clear();
        self.values.clear();
        self.sparse.clear();
    }

    /// Stored rows, including dead owners until the next structural barrier.
    #[must_use]
    pub fn stored_len(&self) -> usize {
        self.values.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Transform;
    #[test]
    fn dense_removal_updates_moved_indices_and_reused_slots() {
        let mut scene = SceneGraph::new(3);
        let ids: Vec<_> = (0..3)
            .map(|_| scene.spawn(None, Transform::default()).unwrap())
            .collect();
        let mut table = ComponentTable::new(&scene);
        for (value, id) in ids.iter().enumerate() {
            table.insert(&scene, *id, value).unwrap();
        }
        assert_eq!(table.remove(&scene, ids[0]), Ok(Some(0)));
        assert_eq!(table.get(&scene, ids[2]), Ok(Some(&2)));
        scene.remove_subtree(ids[1]).unwrap();
        let replacement = scene.spawn(None, Transform::default()).unwrap();
        assert_eq!(table.get(&scene, ids[1]), Err(SceneGraphError::InvalidNode));
        assert_eq!(table.get(&scene, replacement), Ok(None));
        table.insert(&scene, replacement, 9).unwrap();
        assert_eq!(table.get(&scene, replacement), Ok(Some(&9)));
        assert_eq!(table.stored_len(), 2);
        scene.remove_subtree(ids[2]).unwrap();
        assert_eq!(table.synchronize(&scene), Ok(1));
        assert_eq!(table.get(&scene, replacement), Ok(Some(&9)));
    }
    #[test]
    fn active_batch_respects_parent_activity_and_rejects_foreign_scene() {
        let mut scene = SceneGraph::new(2);
        let root = scene.spawn(None, Transform::default()).unwrap();
        let child = scene.spawn(Some(root), Transform::default()).unwrap();
        let mut table = ComponentTable::new(&scene);
        table.insert(&scene, child, 10_u32).unwrap();
        scene.set_active(root, false).unwrap();
        for (_, value) in table.query_mut(&scene, true).unwrap() {
            *value += 1;
        }
        assert_eq!(table.get(&scene, child), Ok(Some(&10)));
        scene.set_active(root, true).unwrap();
        for (_, value) in table.query_mut(&scene, true).unwrap() {
            *value += 1;
        }
        assert_eq!(table.get(&scene, child), Ok(Some(&11)));
        let foreign = SceneGraph::new(2);
        assert_eq!(
            table.synchronize(&foreign),
            Err(SceneGraphError::InvalidNode)
        );
        assert!(table.query_mut(&foreign, true).is_err());
        assert_eq!(table.stored_len(), 1);
    }
}
