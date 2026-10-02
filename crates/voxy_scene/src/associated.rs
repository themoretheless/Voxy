//! Revision-validated associated data for typed requirements.
use crate::{
    ComponentRevision, ComponentTable, NodeId, SceneGraph, SceneGraphError, TableBuildError,
};
use std::{any::Any, marker::PhantomData, sync::Arc};

#[derive(Debug)]
struct Row<T> {
    revisions: (Option<ComponentRevision>, Option<ComponentRevision>),
    value: Arc<T>,
}

/// Owns derived rows for two component types; synchronization scans current
/// membership but only invokes the factory for new or changed requirements.
/// Shared data is read-only by API. Interior mutation and dependencies outside
/// A/B require explicit invalidation. Factory external side effects are not rolled back.
#[derive(Debug)]
pub struct AssociatedData<A, B, T> {
    rows: ComponentTable<Row<T>>,
    force_rebuild: bool,
    requirements: PhantomData<fn() -> (A, B)>,
}
impl<A: Any + Send + Sync, B: Any + Send + Sync, T> AssociatedData<A, B, T> {
    #[must_use]
    pub fn new(scene: &SceneGraph) -> Self {
        Self {
            rows: ComponentTable::new(scene),
            force_rebuild: true,
            requirements: PhantomData,
        }
    }

    /// Forces factory invocation at the next successful synchronization, for
    /// dependencies or interior mutations not represented by component revisions.
    pub fn invalidate(&mut self) {
        self.force_rebuild = true;
    }

    /// Releases published resources immediately, for processor retirement or
    /// unavailable external dependencies. Future synchronization builds fresh rows.
    /// This does not remove authoring components or invoke scene callbacks.
    pub fn clear(&mut self) {
        self.force_rebuild = true;
        self.rows.clear();
    }

    /// Stages a replacement membership table and reuses unchanged Arc-owned rows.
    /// Failure preserves all published rows and the invalidation flag. Success
    /// retires inactive/deleted/missing-requirement rows. Call at structural barriers.
    /// # Errors
    /// Rejects foreign scenes or propagates the first factory failure.
    pub fn synchronize<E>(
        &mut self,
        scene: &SceneGraph,
        mut build: impl FnMut(NodeId, &A, &B, Option<&T>) -> Result<T, E>,
    ) -> Result<(), TableBuildError<E>> {
        let force = self.force_rebuild;
        self.rows
            .rebuild_active_with::<A, B, E>(scene, |owner, first, second, old| {
                let revisions = (
                    scene.component_revision::<A>(owner).ok().flatten(),
                    scene.component_revision::<B>(owner).ok().flatten(),
                );
                let value = if let Some(previous) =
                    old.filter(|row| !force && row.revisions == revisions)
                {
                    Arc::clone(&previous.value)
                } else {
                    Arc::new(build(
                        owner,
                        first,
                        second,
                        old.map(|row| row.value.as_ref()),
                    )?)
                };
                Ok(Row { revisions, value })
            })?;
        self.force_rebuild = false;
        Ok(())
    }

    /// Visits published rows whose owners remain active and requirements remain
    /// revision-matched. Scans stored rows, not all scene slots; an invalidated
    /// table produces no values. Both borrows prevent changes during iteration.
    /// # Errors
    /// Rejects a foreign scene before producing an iterator.
    pub fn query<'a>(
        &'a self,
        scene: &'a SceneGraph,
    ) -> Result<impl Iterator<Item = (NodeId, &'a T)> + 'a, SceneGraphError> {
        Ok(self
            .rows
            .query(scene, true)?
            .filter_map(move |(owner, row)| {
                if self.force_rebuild {
                    return None;
                }
                // The dense query already validated the owner and inherited activity.
                let revisions = (
                    scene.component_revision::<A>(owner).ok().flatten(),
                    scene.component_revision::<B>(owner).ok().flatten(),
                );
                (revisions.0.is_some() && revisions.1.is_some() && row.revisions == revisions)
                    .then_some((owner, row.value.as_ref()))
            }))
    }

    /// Returns only currently eligible, revision-matched data. Changed components
    /// are unavailable until a successful synchronization; failed builds cannot
    /// accidentally dispatch stale derived values. Invalidation also hides rows.
    /// # Errors
    /// Rejects foreign scenes and stale owners.
    pub fn get<'a>(
        &'a self,
        scene: &SceneGraph,
        owner: NodeId,
    ) -> Result<Option<&'a T>, SceneGraphError> {
        let row = self.rows.get(scene, owner)?;
        if self.force_rebuild || !scene.active_in_hierarchy(owner)? {
            return Ok(None);
        }
        let revisions = (
            scene.component_revision::<A>(owner)?,
            scene.component_revision::<B>(owner)?,
        );
        Ok(row
            .filter(|row| {
                revisions.0.is_some() && revisions.1.is_some() && row.revisions == revisions
            })
            .map(|row| row.value.as_ref()))
    }
}
