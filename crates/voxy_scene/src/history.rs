//! Bounded authoring-document history, independent of live simulation/resources.
use crate::{ComponentRegistry, DocumentError, ObjectId, SceneDocument, SceneObject};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};
mod checkpoint;
pub use checkpoint::{HistoryCheckpointLimits, PreparedSceneHistory, SceneHistoryCheckpoint};
/// Opaque process-local observation of the native history transaction epoch.
#[derive(Clone, Debug)]
pub struct HistoryChangeToken(Arc<()>);
impl PartialEq for HistoryChangeToken {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for HistoryChangeToken {}
#[derive(Clone, Debug)]
pub(super) struct Snapshot {
    document: SceneDocument,
    metadata: serde_json::Value,
    auxiliary: serde_json::Value,
    bytes: usize,
}
#[derive(Clone, Debug)]
pub(super) struct CompactStep {
    doc_version: Option<u32>,
    order: Option<Vec<ObjectId>>,
    upserted: Vec<SceneObject>,
    metadata: Option<serde_json::Value>,
    auxiliary: Option<serde_json::Value>,
    bytes: usize,
}
pub(super) fn scene_object_exact_eq(a: &SceneObject, b: &SceneObject) -> bool {
    a.id == b.id
        && a.parent == b.parent
        && a.name == b.name
        && a.active == b.active
        && a.translation.map(f32::to_bits) == b.translation.map(f32::to_bits)
        && a.precise_translation_bits == b.precise_translation_bits
        && a.rotation.map(f32::to_bits) == b.rotation.map(f32::to_bits)
        && a.scale.map(f32::to_bits) == b.scale.map(f32::to_bits)
        && a.components == b.components
}
impl CompactStep {
    fn raw_bytes(
        doc_version: Option<u32>,
        order: Option<&[ObjectId]>,
        upserted: &[SceneObject],
        metadata: Option<&serde_json::Value>,
        auxiliary: Option<&serde_json::Value>,
    ) -> Result<usize, DocumentError> {
        let mut total = usize::from(doc_version.is_some()) * 4;
        if let Some(ids) = order {
            for id in ids {
                total = total
                    .checked_add(id.0.len().saturating_add(3))
                    .ok_or_else(|| DocumentError::Invalid("history delta byte overflow".into()))?;
            }
        }
        for object in upserted {
            let encoded = serde_json::to_vec(object)?;
            total = total
                .checked_add(encoded.len())
                .ok_or_else(|| DocumentError::Invalid("history delta byte overflow".into()))?;
        }
        if let Some(meta) = metadata {
            let len = if meta.is_null() {
                1
            } else {
                serde_json::to_vec(meta)?.len()
            };
            total = total
                .checked_add(len)
                .ok_or_else(|| DocumentError::Invalid("history delta byte overflow".into()))?;
        }
        if let Some(aux) = auxiliary {
            let len = if aux.is_null() {
                1
            } else {
                serde_json::to_vec(aux)?.len()
            };
            total = total
                .checked_add(len)
                .ok_or_else(|| DocumentError::Invalid("history delta byte overflow".into()))?;
        }
        Ok(total.max(1))
    }

    fn diff_parts<'a>(
        from: &'a Snapshot,
        to: &'a Snapshot,
    ) -> Result<
        (
            Option<u32>,
            Option<Vec<ObjectId>>,
            Vec<SceneObject>,
            Option<serde_json::Value>,
            Option<serde_json::Value>,
            usize,
        ),
        DocumentError,
    > {
        let doc_version =
            (from.document.version != to.document.version).then_some(to.document.version);
        let same_order = from.document.objects.len() == to.document.objects.len()
            && from
                .document
                .objects
                .iter()
                .zip(&to.document.objects)
                .all(|(a, b)| a.id == b.id);
        let order = (!same_order).then(|| {
            to.document
                .objects
                .iter()
                .map(|o| o.id.clone())
                .collect::<Vec<_>>()
        });
        let from_by_id: HashMap<&ObjectId, &SceneObject> =
            from.document.objects.iter().map(|o| (&o.id, o)).collect();
        let mut upserted = Vec::new();
        for obj in &to.document.objects {
            if !from_by_id
                .get(&obj.id)
                .is_some_and(|prev| scene_object_exact_eq(prev, obj))
            {
                upserted.push(obj.clone());
            }
        }
        let metadata = (from.metadata != to.metadata).then(|| to.metadata.clone());
        let auxiliary = (from.auxiliary != to.auxiliary).then(|| to.auxiliary.clone());
        let raw = Self::raw_bytes(
            doc_version,
            order.as_deref(),
            &upserted,
            metadata.as_ref(),
            auxiliary.as_ref(),
        )?;
        Ok((doc_version, order, upserted, metadata, auxiliary, raw))
    }

    pub(super) fn between(from: &Snapshot, to: &Snapshot) -> Result<Self, DocumentError> {
        let (doc_version, order, upserted, metadata, auxiliary, forward_raw) =
            Self::diff_parts(from, to)?;
        let (rev_version, rev_order, rev_upserted, rev_meta, rev_aux, _) =
            Self::diff_parts(to, from)?;
        let reverse_raw = Self::raw_bytes(
            rev_version,
            rev_order.as_deref(),
            &rev_upserted,
            rev_meta.as_ref(),
            rev_aux.as_ref(),
        )?;
        let paired = to
            .bytes
            .checked_add(reverse_raw)
            .ok_or_else(|| DocumentError::Invalid("history delta byte overflow".into()))?
            .saturating_sub(from.bytes);
        let bytes = forward_raw.max(paired).max(1);
        Ok(Self {
            doc_version,
            order,
            upserted,
            metadata,
            auxiliary,
            bytes,
        })
    }

    pub(super) fn apply(&self, base: &Snapshot) -> Result<Snapshot, DocumentError> {
        let version = self.doc_version.unwrap_or(base.document.version);
        let mut base_map: HashMap<&ObjectId, &SceneObject> =
            HashMap::with_capacity(base.document.objects.len());
        for obj in &base.document.objects {
            if base_map.insert(&obj.id, obj).is_some() {
                return Err(DocumentError::Invalid(format!(
                    "duplicate object in history base: {}",
                    obj.id.0
                )));
            }
        }
        let mut upserted_map: HashMap<&ObjectId, &SceneObject> =
            HashMap::with_capacity(self.upserted.len());
        for obj in &self.upserted {
            if upserted_map.insert(&obj.id, obj).is_some() {
                return Err(DocumentError::Invalid(format!(
                    "duplicate object in history delta: {}",
                    obj.id.0
                )));
            }
        }
        let objects = if let Some(order) = &self.order {
            let mut seen = HashSet::with_capacity(order.len());
            let mut rebuilt = Vec::with_capacity(order.len());
            for id in order {
                if !seen.insert(id) {
                    return Err(DocumentError::Invalid(format!(
                        "duplicate object in history delta order: {}",
                        id.0
                    )));
                }
                let obj = upserted_map
                    .get(id)
                    .copied()
                    .or_else(|| base_map.get(id).copied())
                    .ok_or_else(|| {
                        DocumentError::Invalid("missing object in history delta".into())
                    })?;
                rebuilt.push(obj.clone());
            }
            if upserted_map.keys().any(|id| !seen.contains(*id)) {
                return Err(DocumentError::Invalid(
                    "unused object patch in history delta".into(),
                ));
            }
            rebuilt
        } else {
            let mut used = 0_usize;
            let mut rebuilt = Vec::with_capacity(base.document.objects.len());
            for obj in &base.document.objects {
                if let Some(replacement) = upserted_map.get(&obj.id) {
                    used += 1;
                    rebuilt.push((*replacement).clone());
                } else {
                    rebuilt.push(obj.clone());
                }
            }
            if used != upserted_map.len() {
                return Err(DocumentError::Invalid(
                    "history delta references object missing from base document".into(),
                ));
            }
            rebuilt
        };
        let document = SceneDocument { version, objects };
        let metadata = self
            .metadata
            .clone()
            .unwrap_or_else(|| base.metadata.clone());
        let auxiliary = self
            .auxiliary
            .clone()
            .unwrap_or_else(|| base.auxiliary.clone());
        let bytes = SceneHistory::snapshot_bytes(&document, &metadata, &auxiliary)?;
        Ok(Snapshot {
            document,
            metadata,
            auxiliary,
            bytes,
        })
    }
}
/// Isolated multi-step authoring edit. Render `document()` for a preview; drop to
/// cancel. Commit once to create a single undo step. No live resource ownership.
#[derive(Debug)]
pub struct SceneEdit {
    document: SceneDocument,
    edit_epoch: Arc<()>,
    epoch: Arc<()>,
}
impl SceneEdit {
    /// Changes on mutable draft access; separate drafts never share a token.
    #[must_use]
    pub fn change_token(&self) -> HistoryChangeToken {
        HistoryChangeToken(self.edit_epoch.clone())
    }
    #[must_use]
    pub fn document(&self) -> &SceneDocument {
        &self.document
    }
    /// Candidate edits are validated on commit; intermediate previews may be invalid.
    pub fn document_mut(&mut self) -> &mut SceneDocument {
        self.edit_epoch = Arc::new(());
        &mut self.document
    }
}
#[derive(Debug)]
pub struct SceneHistory {
    current: Snapshot,
    epoch: Arc<()>,
    undo: VecDeque<CompactStep>,
    redo: Vec<CompactStep>,
    max_versions: usize,
    max_bytes: usize,
    scene_capacity: usize,
}
impl SceneHistory {
    /// Changes on commits, metadata refresh and undo/redo, including a return
    /// to equal document content. Detached edits have their own mutable state.
    #[must_use]
    pub fn change_token(&self) -> HistoryChangeToken {
        HistoryChangeToken(self.epoch.clone())
    }
    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }
    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }
    pub(super) fn snapshot_bytes(
        document: &SceneDocument,
        metadata: &serde_json::Value,
        auxiliary: &serde_json::Value,
    ) -> Result<usize, DocumentError> {
        let auxiliary_bytes = if auxiliary.is_null() {
            0
        } else {
            serde_json::to_vec(auxiliary)?.len()
        };
        document
            .to_json()?
            .len()
            .checked_add(serde_json::to_vec(metadata)?.len())
            .and_then(|bytes| bytes.checked_add(auxiliary_bytes))
            .ok_or_else(|| DocumentError::Invalid("history byte overflow".into()))
    }
    fn validate(
        document: SceneDocument,
        metadata: serde_json::Value,
        auxiliary: serde_json::Value,
        registry: &ComponentRegistry,
        capacity: usize,
        max_bytes: usize,
    ) -> Result<Snapshot, DocumentError> {
        let bytes = Self::snapshot_bytes(&document, &metadata, &auxiliary)?;
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
            edit_epoch: Arc::new(()),
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
        let prev_adj = self
            .undo
            .back()
            .map(|step| step.apply(&self.current))
            .transpose()?;
        let next_adj = self
            .redo
            .last()
            .map(|step| step.apply(&self.current))
            .transpose()?;
        let next_undo_step = prev_adj
            .as_ref()
            .map(|prev| CompactStep::between(&next, prev))
            .transpose()?;
        let next_redo_step = next_adj
            .as_ref()
            .map(|nxt| CompactStep::between(&next, nxt))
            .transpose()?;
        self.current = next;
        if let (Some(back), Some(updated)) = (self.undo.back_mut(), next_undo_step) {
            *back = updated;
        }
        if let (Some(last), Some(updated)) = (self.redo.last_mut(), next_redo_step) {
            *last = updated;
        }
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
        let reverse_step = CompactStep::between(&next, &self.current)?;
        self.current = next;
        self.epoch = Arc::new(());
        self.undo.push_back(reverse_step);
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
    /// Commits a document change, coalescing with the top undo state if both transitions
    /// only modify the exact same single `ObjectId` and neither metadata nor auxiliary drafts changed.
    /// # Errors
    /// Rejects invalid or oversized documents while preserving existing history.
    pub fn commit_coalesced(
        &mut self,
        document: SceneDocument,
        registry: &ComponentRegistry,
    ) -> Result<bool, DocumentError> {
        let prev = if self.redo.is_empty() {
            self.undo
                .back()
                .map(|step| step.apply(&self.current))
                .transpose()?
        } else {
            None
        };
        let can_coalesce = prev.as_ref().is_some_and(|prev| {
            prev.metadata == self.current.metadata
                && prev.auxiliary == self.current.auxiliary
                && single_modified_object_id(&prev.document, &self.current.document)
                    .zip(single_modified_object_id(&self.current.document, &document))
                    .is_some_and(|(a, b)| a == b)
        });
        if can_coalesce {
            let prev = prev.expect("checked in can_coalesce");
            let next = Self::validate(
                document,
                self.current.metadata.clone(),
                self.current.auxiliary.clone(),
                registry,
                self.scene_capacity,
                self.max_bytes,
            )?;
            let coalesced_step = CompactStep::between(&next, &prev)?;
            self.current = next;
            if let Some(back) = self.undo.back_mut() {
                *back = coalesced_step;
            }
            self.epoch = Arc::new(());
            while self.undo.len() + 1 > self.max_versions || self.retained_bytes() > self.max_bytes
            {
                self.undo.pop_front();
            }
            Ok(true)
        } else {
            self.commit(document, registry)
        }
    }
    pub fn undo(&mut self) -> bool {
        let Some(step) = self.undo.back() else {
            return false;
        };
        let Ok(previous) = step.apply(&self.current) else {
            return false;
        };
        let Ok(forward_step) = CompactStep::between(&previous, &self.current) else {
            return false;
        };
        self.undo.pop_back();
        self.epoch = Arc::new(());
        self.current = previous;
        self.redo.push(forward_step);
        true
    }
    pub fn redo(&mut self) -> bool {
        let Some(step) = self.redo.last() else {
            return false;
        };
        let Ok(next) = step.apply(&self.current) else {
            return false;
        };
        let Ok(reverse_step) = CompactStep::between(&next, &self.current) else {
            return false;
        };
        self.redo.pop();
        self.epoch = Arc::new(());
        self.current = next;
        self.undo.push_back(reverse_step);
        true
    }
}

fn single_modified_object_id<'a>(
    before: &'a SceneDocument,
    after: &'a SceneDocument,
) -> Option<&'a crate::ObjectId> {
    if before.version != after.version || before.objects.len() != after.objects.len() {
        return None;
    }
    let mut modified = None;
    for (a, b) in before.objects.iter().zip(&after.objects) {
        if a.id != b.id {
            return None;
        }
        if a != b {
            if modified.is_some() {
                return None;
            }
            modified = Some(&a.id);
        }
    }
    modified
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
        let original_epoch = history.change_token();
        let mut isolated = history.begin_edit();
        let another = history.begin_edit();
        assert_ne!(isolated.change_token(), another.change_token());
        let before_draft = isolated.change_token();
        let _ = isolated.document_mut();
        assert_ne!(before_draft, isolated.change_token());
        assert_eq!(original_epoch, history.change_token());

        let stale = history.begin_edit();
        let draft = serde_json::json!({"settings": {"frames": 10}});
        history.commit_auxiliary(draft.clone(), &registry).unwrap();
        assert_ne!(original_epoch, history.change_token());
        assert_eq!(history.current(), &original);
        assert_eq!(history.metadata(), &serde_json::json!({"composition": 1}));
        assert!(history.commit_edit(stale, &registry).is_err());
        let committed_epoch = history.change_token();
        assert!(history.undo());
        assert_ne!(original_epoch, history.change_token());
        assert_ne!(committed_epoch, history.change_token());
        let undone_epoch = history.change_token();
        assert!(history.auxiliary().is_null());
        assert!(history.redo());
        assert_ne!(committed_epoch, history.change_token());
        assert_ne!(undone_epoch, history.change_token());
        assert_eq!(history.auxiliary(), &draft);
        let accepted_epoch = history.change_token();
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
        assert_eq!(accepted_epoch, history.change_token());
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
    #[test]
    fn coalesced_commits_merge_same_object_edits_into_single_undo_step() {
        let registry = registry();
        let original = document();
        let mut history = SceneHistory::new(original.clone(), &registry, 8, 8, 100_000).unwrap();
        for step in ["step1", "step2", "step3"] {
            let mut next = history.current().clone();
            next.objects[0].name = step.into();
            assert!(history.commit_coalesced(next, &registry).unwrap());
        }
        assert_eq!(history.current().objects[0].name, "step3");
        assert!(history.undo());
        assert_eq!(history.current(), &original);
        assert!(!history.undo());
    }
}
