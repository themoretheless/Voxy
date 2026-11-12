//! Deterministic snapshot ring buffer, server reconciliation rollback, and object-level deltas
//! for authoritative multiplayer and lockstep replay (`docs/architecture.md` §6.3).
//!
//! These are in-memory data structures only: there is no transport, no byte-level compression,
//! and nothing here opens a socket. A delta carries whole changed objects plus removed IDs.

use crate::{ComponentRegistry, DocumentError, LoadedScene, ObjectId, SceneDocument, SceneObject};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use voxy_core::crc32c;

/// Canonical tick-stamped scene snapshot with a deterministic `CRC-32C` state digest.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TickSnapshot {
    pub tick: u64,
    pub state_crc32c: u32,
    pub document: SceneDocument,
}

impl TickSnapshot {
    /// Captures a canonical tick snapshot and computes its deterministic `CRC-32C` digest.
    /// # Errors
    /// Propagates [`DocumentError`] if `document` fails canonical JSON serialization.
    pub fn from_document(tick: u64, mut document: SceneDocument) -> Result<Self, DocumentError> {
        document.objects.sort_by(|a, b| a.id.cmp(&b.id));
        let canonical = document.to_json()?;
        let state_crc32c = crc32c(canonical.as_bytes());
        Ok(Self {
            tick,
            state_crc32c,
            document,
        })
    }
}

/// Compact object-level delta between a baseline `SceneDocument` and a target `SceneDocument`
/// for bandwidth-bounded network replication.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneSnapshotDelta {
    pub base_tick: u64,
    pub target_tick: u64,
    pub target_crc32c: u32,
    pub upserted: Vec<SceneObject>,
    pub removed: Vec<ObjectId>,
}

impl SceneSnapshotDelta {
    /// Computes the minimal object-level delta from `base` to `target`.
    #[must_use]
    pub fn diff(base: &TickSnapshot, target: &TickSnapshot) -> Self {
        let base_map: BTreeMap<&ObjectId, &SceneObject> =
            base.document.objects.iter().map(|o| (&o.id, o)).collect();
        let target_map: BTreeMap<&ObjectId, &SceneObject> =
            target.document.objects.iter().map(|o| (&o.id, o)).collect();

        let mut upserted = Vec::new();
        for (id, obj) in &target_map {
            if base_map.get(id) != Some(obj) {
                upserted.push((*obj).clone());
            }
        }
        let mut removed = Vec::new();
        for id in base_map.keys() {
            if !target_map.contains_key(id) {
                removed.push((*id).clone());
            }
        }

        Self {
            base_tick: base.tick,
            target_tick: target.tick,
            target_crc32c: target.state_crc32c,
            upserted,
            removed,
        }
    }

    /// Applies this delta to `base` and verifies the resulting `CRC-32C` state digest.
    /// # Errors
    /// Rejects base tick mismatches, invalid transforms, or post-apply checksum divergence.
    pub fn apply(&self, base: &TickSnapshot) -> Result<TickSnapshot, DocumentError> {
        if base.tick != self.base_tick {
            return Err(DocumentError::Invalid(format!(
                "delta base tick mismatch: expected {}, got {}",
                self.base_tick, base.tick
            )));
        }
        let mut objects: BTreeMap<ObjectId, SceneObject> = base
            .document
            .objects
            .iter()
            .cloned()
            .map(|o| (o.id.clone(), o))
            .collect();
        for id in &self.removed {
            objects.remove(id);
        }
        for obj in &self.upserted {
            objects.insert(obj.id.clone(), obj.clone());
        }
        let doc = SceneDocument {
            version: base.document.version,
            objects: objects.into_values().collect(),
        };
        let reconstructed = TickSnapshot::from_document(self.target_tick, doc)?;
        if reconstructed.state_crc32c != self.target_crc32c {
            return Err(DocumentError::Invalid(format!(
                "delta state CRC mismatch at tick {}: expected {:#010x}, got {:#010x}",
                self.target_tick, self.target_crc32c, reconstructed.state_crc32c
            )));
        }
        Ok(reconstructed)
    }
}

/// Bounded ring buffer of historical [`TickSnapshot`]s for client-side prediction rollback
/// and server state reconciliation.
#[derive(Debug)]
pub struct SnapshotRollbackBuffer {
    max_history: usize,
    history: VecDeque<TickSnapshot>,
}

impl SnapshotRollbackBuffer {
    #[must_use]
    pub fn new(max_history: usize) -> Self {
        let cap = max_history.max(1);
        Self {
            max_history: cap,
            history: VecDeque::with_capacity(cap),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.history.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Records a new snapshot in strictly increasing tick order, evicting the oldest entry if full.
    /// # Errors
    /// Rejects non-monotonic ticks.
    pub fn push(&mut self, snapshot: TickSnapshot) -> Result<(), DocumentError> {
        if let Some(last) = self.history.back()
            && snapshot.tick <= last.tick
        {
            return Err(DocumentError::Invalid(
                "snapshot ticks must increase strictly".into(),
            ));
        }
        if self.history.len() >= self.max_history {
            self.history.pop_front();
        }
        self.history.push_back(snapshot);
        Ok(())
    }

    /// Looks up the snapshot recorded at `tick`.
    #[must_use]
    pub fn get(&self, tick: u64) -> Option<&TickSnapshot> {
        self.history.iter().find(|s| s.tick == tick)
    }

    /// Verifies whether the locally predicted state at `server_tick` matches `authoritative_crc32c`.
    /// If it diverges, truncates all speculative history after `server_tick` and returns `true`
    /// (indicating that the caller must rollback and replay from the authoritative snapshot).
    #[must_use]
    pub fn reconcile_or_truncate(&mut self, server_tick: u64, authoritative_crc32c: u32) -> bool {
        let Some(pos) = self.history.iter().position(|s| s.tick == server_tick) else {
            self.history.clear();
            return true;
        };
        if self.history[pos].state_crc32c == authoritative_crc32c {
            // Prune confirmed older history prior to `pos` to free memory:
            drop(self.history.drain(..pos));
            false
        } else {
            self.history.clear();
            true
        }
    }

    /// Rolls back to the snapshot recorded at `delta.base_tick`, applies the authoritative
    /// `delta`, and loads the result into a freshly staged scene.
    ///
    /// On success the history keeps only the authoritative snapshot (everything older is
    /// confirmed, everything newer was speculative), and the caller replaces its world with the
    /// returned [`LoadedScene`] before replaying local input from `delta.target_tick`.
    /// On failure neither the history nor any existing world changes.
    /// # Errors
    /// Rejects a base tick that is no longer (or not yet) in the history, a delta whose result
    /// does not match its `CRC-32C`, and a resulting document that fails [`SceneDocument::load`].
    pub fn rollback_and_apply_authoritative(
        &mut self,
        registry: &ComponentRegistry,
        capacity: usize,
        delta: &SceneSnapshotDelta,
    ) -> Result<LoadedScene, DocumentError> {
        let base = self.get(delta.base_tick).ok_or_else(|| {
            DocumentError::Invalid(format!(
                "rollback base tick {} is not in the snapshot history",
                delta.base_tick
            ))
        })?;
        let authoritative = delta.apply(base)?;
        let loaded = authoritative.document.load(registry, capacity)?;
        self.history.clear();
        self.history.push_back(authoritative);
        Ok(loaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_doc(x: f32) -> SceneDocument {
        SceneDocument {
            version: 1,
            objects: vec![
                SceneObject {
                    id: ObjectId("static_floor".into()),
                    parent: None,
                    name: "Floor".into(),
                    active: true,
                    translation: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                    components: BTreeMap::new(),
                },
                SceneObject {
                    id: ObjectId("player".into()),
                    parent: None,
                    name: "Player".into(),
                    active: true,
                    translation: [x, 1.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                    components: BTreeMap::new(),
                },
            ],
        }
    }

    #[test]
    fn snapshot_delta_and_rollback_buffer_reconcile_authoritative_ticks() {
        let snap1 = TickSnapshot::from_document(10, sample_doc(1.0)).unwrap();
        let snap2 = TickSnapshot::from_document(11, sample_doc(2.5)).unwrap();

        let delta = SceneSnapshotDelta::diff(&snap1, &snap2);
        assert_eq!(delta.upserted.len(), 1);
        assert_eq!(delta.upserted[0].id, ObjectId("player".into()));
        assert!(delta.removed.is_empty());

        let applied = delta.apply(&snap1).unwrap();
        assert_eq!(applied, snap2);

        let mut buffer = SnapshotRollbackBuffer::new(4);
        buffer.push(snap1.clone()).unwrap();
        buffer.push(snap2.clone()).unwrap();
        assert!(!buffer.reconcile_or_truncate(10, snap1.state_crc32c));
        assert_eq!(buffer.len(), 2);

        // Divergent server checksum triggers rollback truncation:
        assert!(buffer.reconcile_or_truncate(11, snap2.state_crc32c ^ 0xFF));
        assert!(buffer.is_empty());
    }

    #[test]
    fn rollback_applies_authoritative_delta_and_drops_speculative_history() {
        let registry = ComponentRegistry::default();
        let confirmed = TickSnapshot::from_document(10, sample_doc(1.0)).unwrap();
        let predicted = TickSnapshot::from_document(11, sample_doc(2.5)).unwrap();
        let server = TickSnapshot::from_document(11, sample_doc(4.0)).unwrap();
        let delta = SceneSnapshotDelta::diff(&confirmed, &server);

        let mut buffer = SnapshotRollbackBuffer::new(4);
        buffer.push(confirmed).unwrap();
        buffer.push(predicted).unwrap();

        let loaded = buffer
            .rollback_and_apply_authoritative(&registry, 8, &delta)
            .unwrap();
        let player = loaded.resolve(&ObjectId("player".into())).unwrap();
        assert_eq!(loaded.graph.local(player).unwrap().translation.x, 4.0);
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.get(11), Some(&server));
        assert!(buffer.get(10).is_none());
    }

    #[test]
    fn rollback_rejects_unknown_base_and_corrupt_delta_without_touching_history() {
        let registry = ComponentRegistry::default();
        let confirmed = TickSnapshot::from_document(10, sample_doc(1.0)).unwrap();
        let server = TickSnapshot::from_document(11, sample_doc(4.0)).unwrap();
        let mut buffer = SnapshotRollbackBuffer::new(4);
        buffer.push(confirmed.clone()).unwrap();

        let mut stale = SceneSnapshotDelta::diff(&confirmed, &server);
        stale.base_tick = 7;
        assert!(
            buffer
                .rollback_and_apply_authoritative(&registry, 8, &stale)
                .is_err()
        );

        let mut corrupt = SceneSnapshotDelta::diff(&confirmed, &server);
        corrupt.target_crc32c ^= 1;
        assert!(
            buffer
                .rollback_and_apply_authoritative(&registry, 8, &corrupt)
                .is_err()
        );

        // Capacity too small for the authoritative document: the staged load fails.
        let valid = SceneSnapshotDelta::diff(&confirmed, &server);
        assert!(
            buffer
                .rollback_and_apply_authoritative(&registry, 1, &valid)
                .is_err()
        );
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.get(10), Some(&confirmed));
    }
}
