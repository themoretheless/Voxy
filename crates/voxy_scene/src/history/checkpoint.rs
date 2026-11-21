//! Exact history and detached-edit ownership, including stale edit epochs.
use super::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug)]
pub struct HistoryCheckpointLimits {
    pub max_versions: usize,
    pub max_history_bytes: usize,
    pub max_scene_capacity: usize,
    pub max_archive_bytes: usize,
    pub max_edits: usize,
    pub max_draft_objects: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Object {
    id: crate::ObjectId,
    parent: Option<crate::ObjectId>,
    name: String,
    active: bool,
    translation: [u32; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    precise_translation_bits: Option<[u64; 3]>,
    rotation: [u32; 4],
    scale: [u32; 3],
    components: std::collections::BTreeMap<String, serde_json::Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u32,
    objects: Vec<Object>,
}
impl From<&SceneDocument> for Document {
    fn from(value: &SceneDocument) -> Self {
        let SceneDocument { version, objects } = value;
        Self {
            version: *version,
            objects: objects
                .iter()
                .map(|value| {
                    let crate::SceneObject {
                        id,
                        parent,
                        name,
                        active,
                        translation,
                        precise_translation_bits,
                        rotation,
                        scale,
                        components,
                    } = value;
                    Object {
                        id: id.clone(),
                        parent: parent.clone(),
                        name: name.clone(),
                        active: *active,
                        translation: translation.map(f32::to_bits),
                        precise_translation_bits: *precise_translation_bits,
                        rotation: rotation.map(f32::to_bits),
                        scale: scale.map(f32::to_bits),
                        components: components.clone(),
                    }
                })
                .collect(),
        }
    }
}
impl From<Document> for SceneDocument {
    fn from(value: Document) -> Self {
        let Document { version, objects } = value;
        Self {
            version,
            objects: objects
                .into_iter()
                .map(|value| {
                    let Object {
                        id,
                        parent,
                        name,
                        active,
                        translation,
                        precise_translation_bits,
                        rotation,
                        scale,
                        components,
                    } = value;
                    crate::SceneObject {
                        id,
                        parent,
                        name,
                        active,
                        translation: translation.map(f32::from_bits),
                        precise_translation_bits,
                        rotation: rotation.map(f32::from_bits),
                        scale: scale.map(f32::from_bits),
                        components,
                    }
                })
                .collect(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedSnapshot {
    document: Document,
    metadata: serde_json::Value,
    auxiliary: serde_json::Value,
    bytes: usize,
}
impl From<&Snapshot> for SavedSnapshot {
    fn from(value: &Snapshot) -> Self {
        let Snapshot {
            document,
            metadata,
            auxiliary,
            bytes,
        } = value;
        Self {
            document: document.into(),
            metadata: metadata.clone(),
            auxiliary: auxiliary.clone(),
            bytes: *bytes,
        }
    }
}
fn deserialize_present_value<'de, D>(deserializer: D) -> Result<Option<serde_json::Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde_json::Value::deserialize(deserializer).map(Some)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedStep {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    doc_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    order: Option<Vec<crate::ObjectId>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    upserted: Vec<Object>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present_value"
    )]
    metadata: Option<serde_json::Value>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present_value"
    )]
    auxiliary: Option<serde_json::Value>,
    bytes: usize,
}
fn capture_object(value: &crate::SceneObject) -> Object {
    let crate::SceneObject {
        id,
        parent,
        name,
        active,
        translation,
        precise_translation_bits,
        rotation,
        scale,
        components,
    } = value;
    Object {
        id: id.clone(),
        parent: parent.clone(),
        name: name.clone(),
        active: *active,
        translation: translation.map(f32::to_bits),
        precise_translation_bits: *precise_translation_bits,
        rotation: rotation.map(f32::to_bits),
        scale: scale.map(f32::to_bits),
        components: components.clone(),
    }
}
fn restore_object(value: &Object) -> crate::SceneObject {
    let Object {
        id,
        parent,
        name,
        active,
        translation,
        precise_translation_bits,
        rotation,
        scale,
        components,
    } = value;
    crate::SceneObject {
        id: id.clone(),
        parent: parent.clone(),
        name: name.clone(),
        active: *active,
        translation: translation.map(f32::from_bits),
        precise_translation_bits: *precise_translation_bits,
        rotation: rotation.map(f32::from_bits),
        scale: scale.map(f32::from_bits),
        components: components.clone(),
    }
}
impl From<&CompactStep> for SavedStep {
    fn from(value: &CompactStep) -> Self {
        Self {
            doc_version: value.doc_version,
            order: value.order.clone(),
            upserted: value.upserted.iter().map(capture_object).collect(),
            metadata: value.metadata.clone(),
            auxiliary: value.auxiliary.clone(),
            bytes: value.bytes,
        }
    }
}
impl From<&SavedStep> for CompactStep {
    fn from(value: &SavedStep) -> Self {
        Self {
            doc_version: value.doc_version,
            order: value.order.clone(),
            upserted: value.upserted.iter().map(restore_object).collect(),
            metadata: value.metadata.clone(),
            auxiliary: value.auxiliary.clone(),
            bytes: value.bytes,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Edit {
    document: Document,
    epoch: usize,
}
const CHECKPOINT_VERSION: u32 = 2;
/// Epoch zero is current. Other IDs intern actual detached Arc allocations, not
/// equal documents; a stale edit stays stale even after undo returns equal data.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneHistoryCheckpoint {
    version: u32,
    max_versions: usize,
    max_bytes: usize,
    scene_capacity: usize,
    current: SavedSnapshot,
    undo: Vec<SavedStep>,
    redo: Vec<SavedStep>,
    epoch_count: usize,
    edits: Vec<Edit>,
}
#[derive(Debug)]
pub struct PreparedSceneHistory {
    pub history: SceneHistory,
    pub edits: Vec<SceneEdit>,
}
fn invalid(message: &str) -> DocumentError {
    DocumentError::Invalid(message.into())
}
impl SceneHistory {
    /// Capture committed versions and caller-owned detached gestures as one unit.
    /// Draft documents may be invalid; their exact float words are retained and
    /// normal validation still runs when the restored edit is committed.
    pub fn checkpoint(
        &self,
        edits: &[&SceneEdit],
        limits: HistoryCheckpointLimits,
    ) -> Result<SceneHistoryCheckpoint, DocumentError> {
        let Self {
            current,
            epoch,
            undo,
            redo,
            max_versions,
            max_bytes,
            scene_capacity,
        } = self;
        if edits.len() > limits.max_edits {
            return Err(invalid("history detached edit budget"));
        }
        let mut epochs = vec![epoch.clone()];
        let edits = edits
            .iter()
            .map(|edit| {
                let SceneEdit {
                    document,
                    epoch,
                    edit_epoch: _,
                } = *edit;
                let id = epochs
                    .iter()
                    .position(|old| Arc::ptr_eq(old, epoch))
                    .unwrap_or_else(|| {
                        epochs.push(epoch.clone());
                        epochs.len() - 1
                    });
                Edit {
                    document: document.into(),
                    epoch: id,
                }
            })
            .collect();
        let saved = SceneHistoryCheckpoint {
            version: CHECKPOINT_VERSION,
            max_versions: *max_versions,
            max_bytes: *max_bytes,
            scene_capacity: *scene_capacity,
            current: current.into(),
            undo: undo.iter().map(Into::into).collect(),
            redo: redo.iter().map(Into::into).collect(),
            epoch_count: epochs.len(),
            edits,
        };
        saved.admit(limits)?;
        saved.to_json(limits.max_archive_bytes)?;
        Ok(saved)
    }
}
impl SceneHistoryCheckpoint {
    fn admit(&self, limits: HistoryCheckpointLimits) -> Result<(), DocumentError> {
        let versions = self
            .undo
            .len()
            .checked_add(self.redo.len())
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| invalid("history version overflow"))?;
        let bytes = self
            .undo
            .iter()
            .chain(&self.redo)
            .try_fold(self.current.bytes, |total, saved| {
                total.checked_add(saved.bytes)
            })
            .ok_or_else(|| invalid("history byte overflow"))?;
        if self.version != CHECKPOINT_VERSION
            || self.max_versions == 0
            || self.max_versions > limits.max_versions
            || versions > self.max_versions
            || self.max_bytes > limits.max_history_bytes
            || bytes > self.max_bytes
            || self.scene_capacity > limits.max_scene_capacity
            || self.edits.len() > limits.max_edits
            || self.epoch_count == 0
            || self.epoch_count > self.edits.len().saturating_add(1)
        {
            return Err(invalid("history checkpoint policy or header"));
        }
        let mut next = 1;
        for edit in &self.edits {
            if edit.epoch == next {
                next += 1;
            } else if edit.epoch >= next {
                return Err(invalid("history edit epoch reference"));
            }
            if edit.document.objects.len() > limits.max_draft_objects {
                return Err(invalid("history draft object budget"));
            }
        }
        if next != self.epoch_count {
            return Err(invalid("unreferenced history edit epoch"));
        }
        Ok(())
    }
    pub fn to_json(&self, max_bytes: usize) -> Result<String, DocumentError> {
        // Bound output while encoding, including a caller-owned oversized draft.
        struct Writer {
            bytes: Vec<u8>,
            limit: usize,
        }
        impl std::io::Write for Writer {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
                    return Err(std::io::Error::other("history archive byte budget"));
                }
                self.bytes.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut writer = Writer {
            bytes: Vec::new(),
            limit: max_bytes,
        };
        serde_json::to_writer(&mut writer, self)?;
        String::from_utf8(writer.bytes).map_err(|_| invalid("history archive UTF-8"))
    }
    pub fn from_json(json: &str, max_bytes: usize) -> Result<Self, DocumentError> {
        if json.len() > max_bytes {
            return Err(invalid("history archive byte budget"));
        }
        Ok(serde_json::from_str(json)?)
    }
    pub fn prepare(
        &self,
        registry: &ComponentRegistry,
        limits: HistoryCheckpointLimits,
    ) -> Result<PreparedSceneHistory, DocumentError> {
        self.admit(limits)?;
        self.to_json(limits.max_archive_bytes)?;
        let current = SceneHistory::validate(
            self.current.document.clone().into(),
            self.current.metadata.clone(),
            self.current.auxiliary.clone(),
            registry,
            self.scene_capacity,
            self.max_bytes,
        )?;
        if current.bytes != self.current.bytes {
            return Err(invalid("history retained byte count differs"));
        }
        let undo: VecDeque<_> = self.decode_chain(&self.undo, &current, registry)?.into();
        let redo = self.decode_chain(&self.redo, &current, registry)?;
        let epochs: Vec<_> = (0..self.epoch_count).map(|_| Arc::new(())).collect();
        let edits = self
            .edits
            .iter()
            .map(|saved| SceneEdit {
                edit_epoch: Arc::new(()),
                document: saved.document.clone().into(),
                epoch: epochs[saved.epoch].clone(),
            })
            .collect();
        Ok(PreparedSceneHistory {
            history: SceneHistory {
                current,
                epoch: epochs[0].clone(),
                undo,
                redo,
                max_versions: self.max_versions,
                max_bytes: self.max_bytes,
                scene_capacity: self.scene_capacity,
            },
            edits,
        })
    }

    fn decode_chain(
        &self,
        chain: &[SavedStep],
        start: &Snapshot,
        registry: &ComponentRegistry,
    ) -> Result<Vec<CompactStep>, DocumentError> {
        let mut cursor = start.clone();
        let mut steps_rev = Vec::with_capacity(chain.len());
        for saved in chain.iter().rev() {
            let step = CompactStep::from(saved);
            let next = step.apply(&cursor)?;
            let validated = SceneHistory::validate(
                next.document,
                next.metadata,
                next.auxiliary,
                registry,
                self.scene_capacity,
                self.max_bytes,
            )?;
            let recomputed = CompactStep::between(&cursor, &validated)?;
            if recomputed.bytes != saved.bytes {
                return Err(invalid("history step byte count differs"));
            }
            cursor = validated;
            steps_rev.push(step);
        }
        steps_rev.reverse();
        Ok(steps_rev)
    }
}

#[cfg(test)]
mod tests;
