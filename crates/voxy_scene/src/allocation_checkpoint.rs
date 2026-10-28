//! Staged-only restoration of allocation order and durable identity reservations.
use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedSlot {
    generation: u64,
    owner: Option<ObjectId>,
    children: Vec<ObjectId>,
}
/// Arena slots/free-list order and sibling order, not reusable scene identities.
/// Applied only to a newly loaded private graph before any handles are published.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneAllocationCheckpoint {
    version: u32,
    capacity: usize,
    structure_revision: u64,
    slots: Vec<SavedSlot>,
    free: Vec<usize>,
    retired_ids: Vec<ObjectId>,
}
impl crate::SceneCheckpointView<'_> {
    /// Capture allocation/identity metadata needed for deterministic future spawns.
    /// # Errors
    /// Rejects unidentified live objects/children and encoded byte overflow.
    pub fn allocation_checkpoint(
        &self,
        max_bytes: usize,
    ) -> Result<SceneAllocationCheckpoint, DocumentError> {
        let mut slots = Vec::with_capacity(self.graph.slots.len());
        for slot in &self.graph.slots {
            let (owner, children) = if let Some(node) = &slot.node {
                let handle = NodeId {
                    scene: self.graph.id,
                    slot: slots.len(),
                    generation: slot.generation,
                };
                let owner = self
                    .identity(handle)
                    .cloned()
                    .ok_or_else(|| DocumentError::Invalid("unmapped live allocation".into()))?;
                let children = node
                    .children
                    .iter()
                    .map(|node| {
                        self.identity(*node).cloned().ok_or_else(|| {
                            DocumentError::Invalid("unmapped child allocation".into())
                        })
                    })
                    .collect::<Result<_, _>>()?;
                (Some(owner), children)
            } else {
                (None, Vec::new())
            };
            slots.push(SavedSlot {
                generation: slot.generation,
                owner,
                children,
            });
        }
        let retired_ids = self
            .ids
            .iter()
            .filter(|(_, node)| self.graph.node(**node).is_err())
            .map(|(id, _)| id.clone())
            .collect();
        let saved = SceneAllocationCheckpoint {
            version: 1,
            capacity: self.graph.capacity,
            structure_revision: self.graph.structure_revision,
            slots,
            free: self.graph.free.clone(),
            retired_ids,
        };
        if serde_json::to_vec(&saved)?.len() > max_bytes {
            return Err(DocumentError::Invalid(
                "allocation checkpoint byte budget".into(),
            ));
        }
        Ok(saved)
    }
}
impl LoadedScene {
    pub fn allocation_checkpoint(
        &self,
        max_bytes: usize,
    ) -> Result<SceneAllocationCheckpoint, DocumentError> {
        self.checkpoint_view().allocation_checkpoint(max_bytes)
    }
    /// Engine-internal: target must be a private fresh document load, with no
    /// published handles/caches. All fallible validation precedes moving nodes.
    pub(crate) fn restore_staged_allocation(
        &mut self,
        saved: &SceneAllocationCheckpoint,
        max_bytes: usize,
    ) -> Result<(), DocumentError> {
        let invalid = || DocumentError::Invalid("invalid allocation checkpoint".into());
        if saved.version != 1
            || saved.capacity != self.graph.capacity
            || serde_json::to_vec(saved)?.len() > max_bytes
        {
            return Err(invalid());
        }
        let mut free = HashSet::new();
        for &index in &saved.free {
            if index >= saved.slots.len()
                || !free.insert(index)
                || saved.slots[index].owner.is_some()
            {
                return Err(invalid());
            }
        }
        let mut new_ids = BTreeMap::new();
        let mut remap = HashMap::new();
        for (index, slot) in saved.slots.iter().enumerate() {
            if let Some(id) = &slot.owner {
                let old = self.resolve(id).ok_or_else(invalid)?;
                let next = NodeId {
                    scene: self.graph.id,
                    slot: index,
                    generation: slot.generation,
                };
                if id.0.is_empty() || new_ids.insert(id.clone(), next).is_some() {
                    return Err(invalid());
                }
                remap.insert(old, next);
                let old_node = self.graph.node(old)?;
                if old_node.children.len() != slot.children.len() {
                    return Err(invalid());
                }
                let mut unique = HashSet::new();
                for child in &slot.children {
                    let child = self.resolve(child).ok_or_else(invalid)?;
                    if !unique.insert(child) || self.graph.parent(child)? != Some(old) {
                        return Err(invalid());
                    }
                }
            } else if !slot.children.is_empty()
                || (!free.contains(&index) && slot.generation != u64::MAX)
            {
                return Err(invalid());
            }
        }
        if new_ids.len() != self.graph.len() {
            return Err(invalid());
        }
        for (history_index, id) in saved.retired_ids.iter().enumerate() {
            // Separate non-live identity namespace: it stays disjoint even if
            // the caller configures the maximum usize arena capacity.
            let history_node = self.graph.history_handle(history_index);
            if id.0.is_empty() || new_ids.insert(id.clone(), history_node).is_some() {
                return Err(invalid());
            }
        }
        // Resolve the entire new hierarchy before taking any component-owned nodes.
        let mut plan = Vec::with_capacity(saved.slots.len());
        for slot in &saved.slots {
            if let Some(id) = &slot.owner {
                let old = self.resolve(id).ok_or_else(invalid)?;
                let parent = self
                    .graph
                    .parent(old)?
                    .map(|p| remap.get(&p).copied().ok_or_else(invalid))
                    .transpose()?;
                let children = slot
                    .children
                    .iter()
                    .map(|id| new_ids.get(id).copied().ok_or_else(invalid))
                    .collect::<Result<Vec<_>, _>>()?;
                plan.push(Some((old, parent, children)));
            } else {
                plan.push(None);
            }
        }
        self.graph.bump_change();
        let mut slots = Vec::with_capacity(saved.slots.len());
        for (slot, plan) in saved.slots.iter().zip(plan) {
            let node = plan.map(|(old, parent, children)| {
                let mut node = self.graph.slots[old.slot]
                    .node
                    .take()
                    .expect("validated private allocation");
                node.parent = parent;
                node.children = children;
                node
            });
            slots.push(crate::Slot {
                generation: slot.generation,
                node,
            });
        }
        self.graph.slots = slots;
        self.graph.free.clone_from(&saved.free);
        self.graph.structure_revision = saved.structure_revision;
        self.ids = new_ids;
        Ok(())
    }
}

impl SceneAllocationCheckpoint {
    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }
}
