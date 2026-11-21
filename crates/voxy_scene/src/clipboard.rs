//! Subtree clipboard capture and transactional paste with durable ID allocation and reference remapping.
use crate::{
    ComponentRegistry, DocumentError, ObjectId, SceneDocument, SceneGraphError, SceneObject,
};
use glam::DVec3;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
struct ClipboardEntry {
    object: SceneObject,
    is_root: bool,
    external_parent: Option<ObjectId>,
}

/// Captured selection subtrees ready to paste into the same or another `SceneDocument`.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneClipboard {
    entries: Vec<ClipboardEntry>,
}

/// Result of pasting a [`SceneClipboard`] into a [`SceneDocument`].
#[derive(Clone, Debug, PartialEq)]
pub struct ClipboardPaste {
    pub document: SceneDocument,
    pub root_ids: Vec<ObjectId>,
    pub new_ids: Vec<ObjectId>,
}

fn collect_descendants(
    document: &SceneDocument,
    children: &BTreeMap<Option<&ObjectId>, Vec<usize>>,
    root_index: usize,
    external_parent: Option<ObjectId>,
    seen: &mut BTreeSet<ObjectId>,
    entries: &mut Vec<ClipboardEntry>,
) {
    let mut stack = vec![(root_index, true, external_parent)];
    while let Some((index, is_root, ext_parent)) = stack.pop() {
        let object = &document.objects[index];
        if !seen.insert(object.id.clone()) {
            continue;
        }
        entries.push(ClipboardEntry {
            object: object.clone(),
            is_root,
            external_parent: ext_parent,
        });
        if let Some(child_indices) = children.get(&Some(&object.id)) {
            for &child in child_indices.iter().rev() {
                stack.push((child, false, None));
            }
        }
    }
}

fn advance_next_id(document: &SceneDocument, next_id: &mut u64) -> Result<(), DocumentError> {
    for object in &document.objects {
        if let Some(n) = object
            .id
            .0
            .strip_prefix("model-")
            .or_else(|| object.id.0.strip_prefix("object-"))
            .and_then(|s| s.parse::<u64>().ok())
        {
            let candidate = n
                .checked_add(1)
                .ok_or_else(|| DocumentError::Invalid("scene object identity exhausted".into()))?;
            *next_id = (*next_id).max(candidate);
        }
    }
    Ok(())
}

fn allocate_unique_id(
    is_model: bool,
    existing: &mut BTreeSet<ObjectId>,
    next_id: &mut u64,
) -> Result<ObjectId, DocumentError> {
    let prefix = if is_model { "model-" } else { "object-" };
    loop {
        let candidate = ObjectId(format!("{prefix}{}", *next_id));
        *next_id = (*next_id)
            .checked_add(1)
            .ok_or_else(|| DocumentError::Invalid("scene object identity exhausted".into()))?;
        if existing.insert(candidate.clone()) {
            return Ok(candidate);
        }
    }
}

fn apply_offset(object: &mut SceneObject, offset: [f32; 3]) -> Result<(), DocumentError> {
    if !offset.iter().all(|v| v.is_finite()) {
        return Err(DocumentError::Invalid(
            "clipboard paste offset must be finite".into(),
        ));
    }
    if offset == [0.0, 0.0, 0.0] {
        return Ok(());
    }
    if let Some(precise) = object.checked_precise_translation()? {
        let shifted = precise + DVec3::from_array(offset.map(f64::from));
        if !shifted.is_finite() {
            return Err(DocumentError::Invalid(
                "pasted translation exceeds finite range".into(),
            ));
        }
        object.translation = shifted.as_vec3().to_array();
        object.precise_translation_bits = Some(shifted.to_array().map(f64::to_bits));
    } else {
        let shifted = [
            object.translation[0] + offset[0],
            object.translation[1] + offset[1],
            object.translation[2] + offset[2],
        ];
        if !shifted.iter().all(|v| v.is_finite()) {
            return Err(DocumentError::Invalid(
                "pasted translation exceeds finite range".into(),
            ));
        }
        object.translation = shifted;
    }
    Ok(())
}

impl SceneClipboard {
    /// Captures the minimal covering subtrees for `selected_ids` in document order.
    /// When both an ancestor and a descendant are selected, only the ancestor subtree
    /// is captured once so hierarchy paste never duplicates child nodes.
    ///
    /// # Errors
    /// Rejects empty selections or unknown object identifiers.
    pub fn capture(
        document: &SceneDocument,
        selected_ids: &[ObjectId],
    ) -> Result<Self, DocumentError> {
        if selected_ids.is_empty() {
            return Err(DocumentError::Invalid("cannot copy empty selection".into()));
        }
        let mut by_id = BTreeMap::new();
        for (index, object) in document.objects.iter().enumerate() {
            by_id.insert(object.id.clone(), index);
        }
        let mut selected_set = BTreeSet::new();
        for id in selected_ids {
            if !by_id.contains_key(id) {
                return Err(DocumentError::Invalid(format!(
                    "cannot copy unknown object {}",
                    id.0
                )));
            }
            selected_set.insert(id.clone());
        }
        let mut children: BTreeMap<Option<&ObjectId>, Vec<usize>> = BTreeMap::new();
        for (index, object) in document.objects.iter().enumerate() {
            children
                .entry(object.parent.as_ref())
                .or_default()
                .push(index);
        }
        let mut roots = Vec::new();
        for (index, object) in document.objects.iter().enumerate() {
            if !selected_set.contains(&object.id) {
                continue;
            }
            let mut ancestor = object.parent.as_ref();
            let mut covered = false;
            let mut steps = 0usize;
            while let Some(parent_id) = ancestor {
                if selected_set.contains(parent_id) {
                    covered = true;
                    break;
                }
                steps += 1;
                if steps > document.objects.len() {
                    return Err(DocumentError::Scene(SceneGraphError::Cycle));
                }
                ancestor = by_id
                    .get(parent_id)
                    .and_then(|&idx| document.objects[idx].parent.as_ref());
            }
            if !covered {
                roots.push((index, object.parent.clone()));
            }
        }
        let mut seen = BTreeSet::new();
        let mut entries = Vec::new();
        for (root_index, external_parent) in roots {
            collect_descendants(
                document,
                &children,
                root_index,
                external_parent,
                &mut seen,
                &mut entries,
            );
        }
        Ok(Self { entries })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Pastes captured subtrees into a clone of `document`, remapping internal `ObjectId`s
    /// across hierarchy links and registered components and shifting root translations by `root_offset`.
    ///
    /// # Errors
    /// Rejects capacity overflows, invalid offsets, or invalid resulting scene state without mutating inputs.
    pub fn paste(
        &self,
        document: &SceneDocument,
        registry: &ComponentRegistry,
        capacity: usize,
        root_offset: [f32; 3],
        next_id: &mut u64,
    ) -> Result<ClipboardPaste, DocumentError> {
        self.paste_filtered(document, registry, capacity, root_offset, next_id, |_| {})
    }

    /// Same as [`Self::paste`], allowing callers to adjust cloned component maps prior to remapping/validation.
    ///
    /// # Errors
    /// Rejects capacity overflows, invalid offsets, or invalid resulting scene state.
    pub fn paste_filtered(
        &self,
        document: &SceneDocument,
        registry: &ComponentRegistry,
        capacity: usize,
        root_offset: [f32; 3],
        next_id: &mut u64,
        mut filter: impl FnMut(&mut BTreeMap<String, serde_json::Value>),
    ) -> Result<ClipboardPaste, DocumentError> {
        let total = document
            .objects
            .len()
            .checked_add(self.entries.len())
            .ok_or(DocumentError::Scene(SceneGraphError::Capacity))?;
        if total > capacity {
            return Err(DocumentError::Scene(SceneGraphError::Capacity));
        }
        let mut id_counter = *next_id;
        advance_next_id(document, &mut id_counter)?;
        let mut existing: BTreeSet<ObjectId> =
            document.objects.iter().map(|obj| obj.id.clone()).collect();
        let mut id_map = BTreeMap::new();
        for entry in &self.entries {
            let is_model = entry.object.components.contains_key("editor.model.v1");
            let new_id = allocate_unique_id(is_model, &mut existing, &mut id_counter)?;
            id_map.insert(entry.object.id.clone(), new_id);
        }
        let existing_roots: BTreeSet<ObjectId> =
            document.objects.iter().map(|obj| obj.id.clone()).collect();
        let mut root_ids = Vec::new();
        let mut new_ids = Vec::with_capacity(self.entries.len());
        let mut pasted = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            let mut obj = entry.object.clone();
            let new_id = id_map[&entry.object.id].clone();
            obj.id = new_id.clone();
            if entry.is_root {
                obj.parent = entry
                    .external_parent
                    .as_ref()
                    .filter(|parent| existing_roots.contains(*parent))
                    .cloned();
                apply_offset(&mut obj, root_offset)?;
                root_ids.push(new_id.clone());
            } else {
                let old_parent =
                    entry.object.parent.as_ref().ok_or_else(|| {
                        DocumentError::Invalid("missing copied child parent".into())
                    })?;
                obj.parent = Some(id_map[old_parent].clone());
            }
            filter(&mut obj.components);
            let mut remapped = BTreeMap::new();
            for (schema, value) in &obj.components {
                let updated = if registry.contains_schema(schema) {
                    registry.remap_value(schema, value, &id_map)?
                } else {
                    value.clone()
                };
                remapped.insert(schema.clone(), updated);
            }
            obj.components = remapped;
            new_ids.push(new_id);
            pasted.push(obj);
        }
        let mut candidate = document.clone();
        candidate.objects.extend(pasted);
        candidate.load(registry, capacity)?;
        *next_id = id_counter;
        Ok(ClipboardPaste {
            document: candidate,
            root_ids,
            new_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TargetLink {
        target: ObjectId,
    }

    fn base_object(id: &str, parent: Option<&str>, x: f32) -> SceneObject {
        SceneObject {
            id: ObjectId(id.into()),
            parent: parent.map(|p| ObjectId(p.into())),
            name: id.into(),
            active: true,
            translation: [x, 0.0, 0.0],
            precise_translation_bits: None,
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
            components: BTreeMap::new(),
        }
    }

    #[test]
    fn capture_deduplicates_nested_selection_and_remaps_internal_references() {
        let mut registry = ComponentRegistry::default();
        registry
            .register_with_reference_remap::<TargetLink>(
                "test.link.v1",
                |link| vec![link.target.clone()],
                |link, ids| {
                    if let Some(new_id) = ids.get(&link.target) {
                        link.target = new_id.clone();
                    }
                },
            )
            .unwrap();
        let mut root = base_object("object-0", None, 1.0);
        let precise = DVec3::new(1.0 + 1e-9, 2.0, 3.0);
        root.translation = precise.as_vec3().to_array();
        root.precise_translation_bits = Some(precise.to_array().map(f64::to_bits));
        let mut child = base_object("object-1", Some("object-0"), 2.0);
        child
            .components
            .insert("test.link.v1".into(), json!({ "target": "object-0" }));
        let sibling = base_object("object-2", None, -1.0);
        let doc = SceneDocument {
            version: 1,
            objects: vec![root, child, sibling],
        };

        // Selecting both parent and child only captures the parent subtree once.
        let clip = SceneClipboard::capture(
            &doc,
            &[ObjectId("object-0".into()), ObjectId("object-1".into())],
        )
        .unwrap();
        assert_eq!(clip.len(), 2);

        let mut next_id = 1;
        let pasted = clip
            .paste(&doc, &registry, 16, [0.5, 0.0, 0.0], &mut next_id)
            .unwrap();
        assert_eq!(pasted.root_ids, vec![ObjectId("object-3".into())]);
        assert_eq!(
            pasted.new_ids,
            vec![ObjectId("object-3".into()), ObjectId("object-4".into())]
        );
        assert_eq!(next_id, 5);
        assert_eq!(pasted.document.objects.len(), 5);

        let pasted_root = &pasted.document.objects[3];
        let pasted_child = &pasted.document.objects[4];
        assert_eq!(pasted_root.parent, None);
        assert_eq!(
            pasted_child.parent.as_ref(),
            Some(&ObjectId("object-3".into()))
        );
        assert_eq!(
            pasted_child.components["test.link.v1"],
            json!({ "target": "object-3" })
        );
        let expected_precise = precise + DVec3::new(0.5, 0.0, 0.0);
        assert_eq!(
            pasted_root.precise_translation_bits,
            Some(expected_precise.to_array().map(f64::to_bits))
        );
    }

    #[test]
    fn paste_rejects_capacity_overflow_without_mutating_next_id() {
        let registry = ComponentRegistry::default();
        let doc = SceneDocument {
            version: 1,
            objects: vec![base_object("object-0", None, 0.0)],
        };
        let clip = SceneClipboard::capture(&doc, &[ObjectId("object-0".into())]).unwrap();
        let mut next_id = 10;
        let err = clip
            .paste(&doc, &registry, 1, [0.0, 0.0, 0.0], &mut next_id)
            .unwrap_err();
        assert!(matches!(
            err,
            DocumentError::Scene(SceneGraphError::Capacity)
        ));
        assert_eq!(next_id, 10);
    }
}
