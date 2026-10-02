//! Bounded authoring-document history, independent of live simulation/resources.
use crate::{ComponentRegistry, DocumentError, SceneDocument};
use std::{collections::VecDeque, sync::Arc};
#[derive(Debug)]
struct Snapshot {
    document: SceneDocument,
    metadata: serde_json::Value,
    auxiliary: serde_json::Value,
    bytes: usize,
}
/// Isolated multi-step authoring edit. Render `document()` for a preview; drop to
/// cancel. Commit once to create a single undo step. No live resource ownership.
#[derive(Debug)]
pub struct SceneEdit {
    document: SceneDocument,
    epoch: Arc<()>,
}
impl SceneEdit {
    #[must_use]
    pub fn document(&self) -> &SceneDocument {
        &self.document
    }
    /// Candidate edits are validated on commit; intermediate previews may be invalid.
    pub fn document_mut(&mut self) -> &mut SceneDocument {
        &mut self.document
    }
}
#[derive(Debug)]
pub struct SceneHistory {
    current: Snapshot,
    epoch: Arc<()>,
    undo: VecDeque<Snapshot>,
    redo: Vec<Snapshot>,
    max_versions: usize,
    max_bytes: usize,
    scene_capacity: usize,
}
impl SceneHistory {
    fn validate(
        document: SceneDocument,
        metadata: serde_json::Value,
        auxiliary: serde_json::Value,
        registry: &ComponentRegistry,
        capacity: usize,
        max_bytes: usize,
    ) -> Result<Snapshot, DocumentError> {
        let auxiliary_bytes = if auxiliary.is_null() {
            0
        } else {
            serde_json::to_vec(&auxiliary)?.len()
        };
        let bytes = document
            .to_json()?
            .len()
            .checked_add(serde_json::to_vec(&metadata)?.len())
            .and_then(|bytes| bytes.checked_add(auxiliary_bytes))
            .ok_or_else(|| DocumentError::Invalid("history byte overflow".into()))?;
        if bytes > max_bytes {
            return Err(DocumentError::Invalid(
                "history document byte limit exceeded".into(),
            ));
        }
        document.load(registry, capacity)?;
        Ok(Snapshot {
            document,
            metadata,
            auxiliary,
            bytes,
        })
    }
    /// # Errors
    /// Rejects zero version capacity, invalid documents and serialized-byte limits.
    pub fn new(
        document: SceneDocument,
        registry: &ComponentRegistry,
        scene_capacity: usize,
        max_versions: usize,
        max_bytes: usize,
    ) -> Result<Self, DocumentError> {
        Self::new_with_metadata(
            document,
            serde_json::Value::Null,
            registry,
            scene_capacity,
            max_versions,
            max_bytes,
        )
    }
    /// Creates one initial authoring version containing document and metadata.
    /// # Errors
    /// Rejects zero version capacity, invalid documents and the combined byte limit.
    pub fn new_with_metadata(
        document: SceneDocument,
        metadata: serde_json::Value,
        registry: &ComponentRegistry,
        scene_capacity: usize,
        max_versions: usize,
        max_bytes: usize,
    ) -> Result<Self, DocumentError> {
        if max_versions == 0 {
            return Err(DocumentError::Invalid("zero history capacity".into()));
        }
        let current = Self::validate(
            document,
            metadata,
            serde_json::Value::Null,
            registry,
            scene_capacity,
            max_bytes,
        )?;
        Ok(Self {
            current,
            epoch: Arc::new(()),
            undo: VecDeque::new(),
            redo: Vec::new(),
            max_versions,
            max_bytes,
            scene_capacity,
        })
    }
    #[must_use]
    pub fn current(&self) -> &SceneDocument {
        &self.current.document
    }
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        self.current.bytes
            + self
                .undo
                .iter()
                .chain(&self.redo)
                .map(|s| s.bytes)
                .sum::<usize>()
    }
    /// Starts a detached gesture/transaction from the current authoring version.
    /// Preview mutations do not discard redo or alter current/undo state.
    #[must_use]
    pub fn begin_edit(&self) -> SceneEdit {
        SceneEdit {
            document: self.current.document.clone(),
            epoch: Arc::clone(&self.epoch),
        }
    }
    /// Commits all preview steps as one undo entry. Rejects edits begun before
    /// another successful commit, undo or redo, even if the document is equal again.
    /// # Errors
    /// Rejects stale transactions and invalid candidates without changing history.
    pub fn commit_edit(
        &mut self,
        edit: SceneEdit,
        registry: &ComponentRegistry,
    ) -> Result<bool, DocumentError> {
        if !Arc::ptr_eq(&edit.epoch, &self.epoch) {
            return Err(DocumentError::Invalid("stale authoring edit".into()));
        }
        self.commit(edit.document, registry)
    }
    /// Edits a declared collection by item ID through the existing undo history.
    /// Returns false for a no-op without discarding redo.
    /// # Errors
    /// Rejects invalid collection changes or scene/history limits atomically.
    pub fn edit_collection(
        &mut self,
        registry: &ComponentRegistry,
        object: &crate::ObjectId,
        schema: &str,
        path: &str,
        edit: crate::CollectionEdit,
    ) -> Result<bool, DocumentError> {
        let mut candidate = self.current.document.clone();
        candidate.edit_collection(registry, object, schema, path, edit, self.scene_capacity)?;
        self.commit(candidate, registry)
    }
    /// Validates a candidate before changing history. New edits discard redo;
    /// oldest undo versions are evicted to respect version/serialized-byte limits.
    /// Returns false for an unchanged document without discarding redo.
    /// # Errors
    /// Rejects invalid/oversized documents, preserving all history.
    pub fn commit(
        &mut self,
        document: SceneDocument,
        registry: &ComponentRegistry,
    ) -> Result<bool, DocumentError> {
        self.commit_with_metadata(document, self.current.metadata.clone(), registry)
    }
    /// Returns application-owned authoring data from the same undo/redo version.
    /// GPU resources and live callbacks must stay outside this serialized snapshot.
    #[must_use]
    pub fn metadata(&self) -> &serde_json::Value {
        &self.current.metadata
    }

    /// Checks a saved authoring payload against the current document's byte quota
    /// before a caller publishes its file. Does not alter versions or epochs.
    /// # Errors
    /// Rejects a candidate exceeding the combined document/metadata byte quota.
    pub fn check_metadata(
        &self,
        metadata: &serde_json::Value,
        registry: &ComponentRegistry,
    ) -> Result<(), DocumentError> {
        Self::validate(
            self.current.document.clone(),
            metadata.clone(),
            self.current.auxiliary.clone(),
            registry,
            self.scene_capacity,
            self.max_bytes,
        )?;
        Ok(())
    }
    /// Refreshes serialization/source metadata after saving the current version.
    /// This does not create an authoring edit or discard redo. Both payloads keep
    /// the same quota; oldest undo/farthest redo versions may be evicted if needed.
    /// # Errors
    /// Oversized metadata preserves every history version and transaction epoch.
    pub fn refresh_metadata(
        &mut self,
        metadata: serde_json::Value,
        registry: &ComponentRegistry,
    ) -> Result<(), DocumentError> {
        if metadata == self.current.metadata {
            return Ok(());
        }
        let next = Self::validate(
            self.current.document.clone(),
            metadata,
            self.current.auxiliary.clone(),
            registry,
            self.scene_capacity,
            self.max_bytes,
        )?;
        self.current = next;
        self.epoch = Arc::new(());
        while self.retained_bytes() > self.max_bytes {
            if self.undo.pop_front().is_none() {
                self.redo.remove(0);
            }
        }
        Ok(())
    }
    /// Commits document and bounded authoring metadata atomically. Metadata-only
    /// changes create undo entries; both payloads count toward the same byte limit.
    /// # Errors
    /// Invalid/oversized candidates preserve current, undo and redo state.
    pub fn commit_with_metadata(
        &mut self,
        document: SceneDocument,
        metadata: serde_json::Value,
        registry: &ComponentRegistry,
    ) -> Result<bool, DocumentError> {
        self.commit_state(document, metadata, self.current.auxiliary.clone(), registry)
    }
    /// Additional application-owned resource drafts from the same undo version.
    #[must_use]
    pub fn auxiliary(&self) -> &serde_json::Value {
        &self.current.auxiliary
    }
    /// Commits resource drafts without replacing composition metadata or document.
    /// # Errors
    /// Rejects combined byte limits or invalid documents without changing history.
    pub fn commit_auxiliary(
        &mut self,
        auxiliary: serde_json::Value,
        registry: &ComponentRegistry,
    ) -> Result<bool, DocumentError> {
        self.commit_state(
            self.current.document.clone(),
            self.current.metadata.clone(),
            auxiliary,
            registry,
        )
    }
    fn commit_state(
        &mut self,
        document: SceneDocument,
        metadata: serde_json::Value,
        auxiliary: serde_json::Value,
        registry: &ComponentRegistry,
    ) -> Result<bool, DocumentError> {
        if document == self.current.document
            && metadata == self.current.metadata
            && auxiliary == self.current.auxiliary
        {
            return Ok(false);
        }
        let next = Self::validate(
            document,
            metadata,
            auxiliary,
            registry,
            self.scene_capacity,
            self.max_bytes,
        )?;
        let old = std::mem::replace(&mut self.current, next);
        self.epoch = Arc::new(());
        self.undo.push_back(old);
        self.redo.clear();
        while self.undo.len() + 1 > self.max_versions || self.retained_bytes() > self.max_bytes {
            self.undo.pop_front();
        }
        Ok(true)
    }
    /// Applies an edit to an isolated document clone before validating/committing.
    /// # Errors
    /// Edit errors or validation failures preserve current/undo/redo state.
    pub fn edit(
        &mut self,
        registry: &ComponentRegistry,
        edit: impl FnOnce(&mut SceneDocument) -> Result<(), DocumentError>,
    ) -> Result<bool, DocumentError> {
        let mut candidate = self.current.document.clone();
        edit(&mut candidate)?;
        self.commit(candidate, registry)
    }
    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo.pop_back() {
            self.epoch = Arc::new(());
            self.redo
                .push(std::mem::replace(&mut self.current, previous));
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo.pop() {
            self.epoch = Arc::new(());
            self.undo
                .push_back(std::mem::replace(&mut self.current, next));
            true
        } else {
            false
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> SceneDocument {
        SceneDocument::from_json(include_str!("../examples/data/game.scene.json")).unwrap()
    }
    fn registry() -> ComponentRegistry {
        let mut registry = ComponentRegistry::default();
        registry.register::<u32>("game.health.v1").unwrap();
        registry
            .register_with_references::<crate::ObjectId>("game.target.v1", |id| vec![id.clone()])
            .unwrap();
        registry
    }
    #[test]
    fn resource_drafts_share_history_epochs_metadata_and_quota() {
        let registry = registry();
        let original = document();
        let mut history = SceneHistory::new_with_metadata(
            original.clone(),
            serde_json::json!({"composition": 1}),
            &registry,
            8,
            8,
            4096,
        )
        .unwrap();
        let stale = history.begin_edit();
        let draft = serde_json::json!({"settings": {"frames": 10}});
        history.commit_auxiliary(draft.clone(), &registry).unwrap();
        assert_eq!(history.current(), &original);
        assert_eq!(history.metadata(), &serde_json::json!({"composition": 1}));
        assert!(history.commit_edit(stale, &registry).is_err());
        assert!(history.undo());
        assert!(history.auxiliary().is_null());
        assert!(history.redo());
        assert_eq!(history.auxiliary(), &draft);
        let bytes = history.retained_bytes();
        assert!(
            history
                .commit_auxiliary(
                    serde_json::json!({"oversized": "x".repeat(4096)}),
                    &registry
                )
                .is_err()
        );
        assert_eq!(history.auxiliary(), &draft);
        assert_eq!(history.retained_bytes(), bytes);
        history
            .refresh_metadata(serde_json::json!({"composition": 2}), &registry)
            .unwrap();
        assert_eq!(history.auxiliary(), &draft);
    }
    #[test]
    fn undo_redo_branching_and_invalid_edits_preserve_state() {
        let registry = registry();
        let original = document();
        let mut history = SceneHistory::new(original.clone(), &registry, 8, 8, 100_000).unwrap();
        history
            .edit(&registry, |d| {
                d.objects[0].name = "changed".into();
                Ok(())
            })
            .unwrap();
        assert!(history.undo());
        assert_eq!(history.current(), &original);
        assert!(history.redo());
        assert_eq!(history.current().objects[0].name, "changed");
        assert!(history.undo());
        assert!(
            history
                .edit(&registry, |d| {
                    d.objects[0].parent = Some(crate::ObjectId("missing".into()));
                    Ok(())
                })
                .is_err()
        );
        assert_eq!(history.current(), &original);
        assert!(history.redo());
        assert!(history.undo());
        history
            .edit(&registry, |d| {
                d.objects[0].name = "branch".into();
                Ok(())
            })
            .unwrap();
        assert!(!history.redo());
    }
    #[test]
    fn version_and_serialized_byte_limits_evict_oldest_history() {
        let registry = registry();
        let mut history = SceneHistory::new(document(), &registry, 8, 2, 100_000).unwrap();
        for name in ["one", "two", "three"] {
            history
                .edit(&registry, |d| {
                    d.objects[0].name = name.into();
                    Ok(())
                })
                .unwrap();
        }
        assert!(history.undo());
        assert_eq!(history.current().objects[0].name, "two");
        assert!(!history.undo());
        let bytes = document().to_json().unwrap().len();
        let mut history = SceneHistory::new(document(), &registry, 8, 8, bytes + 100).unwrap();
        history
            .edit(&registry, |d| {
                d.objects[0].name = "short".into();
                Ok(())
            })
            .unwrap();
        assert!(history.retained_bytes() <= bytes + 100);
        assert!(!history.undo());
    }
    #[test]
    fn detached_gesture_is_one_step_and_undo_redo_invalidates_old_previews() {
        let registry = registry();
        let original = document();
        let mut history = SceneHistory::new(original.clone(), &registry, 8, 8, 100_000).unwrap();
        let mut gesture = history.begin_edit();
        for name in ["drag1", "drag2", "drag3"] {
            gesture.document_mut().objects[0].name = name.into();
        }
        assert_eq!(history.current(), &original);
        assert!(history.commit_edit(gesture, &registry).unwrap());
        assert_eq!(history.current().objects[0].name, "drag3");
        let stale = history.begin_edit();
        assert!(history.undo());
        assert_eq!(history.current(), &original);
        assert!(!history.undo());
        assert!(history.redo());
        assert!(history.commit_edit(stale, &registry).is_err());
        assert_eq!(history.current().objects[0].name, "drag3");
    }
    #[test]
    fn cancelled_invalid_and_foreign_previews_preserve_history() {
        let registry = registry();
        let mut history = SceneHistory::new(document(), &registry, 8, 8, 100_000).unwrap();
        history
            .edit(&registry, |d| {
                d.objects[0].name = "saved".into();
                Ok(())
            })
            .unwrap();
        assert!(history.undo());
        let mut cancelled = history.begin_edit();
        cancelled.document_mut().objects[0].name = "cancelled".into();
        drop(cancelled);
        let mut invalid = history.begin_edit();
        invalid.document_mut().objects[0].parent = Some(crate::ObjectId("missing".into()));
        assert!(history.commit_edit(invalid, &registry).is_err());
        let other = SceneHistory::new(document(), &registry, 8, 8, 100_000).unwrap();
        assert!(history.commit_edit(other.begin_edit(), &registry).is_err());
        assert!(history.redo());
        assert_eq!(history.current().objects[0].name, "saved");
        let unchanged = history.begin_edit();
        assert!(!history.commit_edit(unchanged, &registry).unwrap());
    }
}
