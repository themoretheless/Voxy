//! Explicitly declared collection identity; array position is never an identity.
use crate::DocumentError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Authoring operations target a declared collection and stable item IDs.
#[derive(Clone, Debug, PartialEq)]
pub enum CollectionEdit {
    InsertDefault {
        id: String,
        before: Option<String>,
    },
    Insert {
        value: Value,
        before: Option<String>,
    },
    Remove {
        id: String,
    },
    Move {
        id: String,
        before: Option<String>,
    },
}

impl crate::SceneDocument {
    /// Applies one collection edit after validating the complete candidate scene.
    /// `before: None` inserts or moves to the end. Failed edits preserve self.
    /// # Errors
    /// Rejects unknown declarations/items, identity collisions, quotas, invalid
    /// components, references or transforms.
    pub fn edit_collection(
        &mut self,
        registry: &crate::ComponentRegistry,
        object: &crate::ObjectId,
        schema: &str,
        path: &str,
        edit: CollectionEdit,
        capacity: usize,
    ) -> Result<(), DocumentError> {
        let edit = match edit {
            CollectionEdit::InsertDefault { id, before } => CollectionEdit::Insert {
                value: registry.new_collection_item(schema, path, &id)?,
                before,
            },
            edit => edit,
        };
        let key = registry
            .identified_collections(schema)?
            .get(path)
            .ok_or_else(|| invalid("undeclared identified collection"))?;
        let mut candidate = self.clone();
        let component = candidate
            .objects
            .iter_mut()
            .find(|value| &value.id == object)
            .and_then(|object| object.components.get_mut(schema))
            .ok_or_else(|| invalid("missing collection owner or component"))?;
        let collection = if path.is_empty() {
            component
        } else {
            crate::prefab_document::member_mut(component, path)?
        };
        indexed(collection, key)?;
        let items = collection
            .as_array_mut()
            .ok_or_else(|| invalid("identified collection must be an array"))?;
        let position = |items: &[Value], id: &str| {
            items
                .iter()
                .position(|value| value.get(key).and_then(Value::as_str) == Some(id))
        };
        match edit {
            CollectionEdit::InsertDefault { .. } => {
                return Err(invalid("unresolved collection default"));
            }
            CollectionEdit::Insert { value, before } => {
                let id = value
                    .get(key)
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("new collection item requires identity"))?;
                if position(items, id).is_some() {
                    return Err(invalid("collection addition identity collision"));
                }
                let at = before
                    .as_deref()
                    .map(|id| {
                        position(items, id)
                            .ok_or_else(|| invalid("collection insertion anchor disappeared"))
                    })
                    .transpose()?
                    .unwrap_or(items.len());
                items.insert(at, value);
            }
            CollectionEdit::Remove { id } => {
                let at = position(items, &id)
                    .ok_or_else(|| invalid("collection removal target disappeared"))?;
                items.remove(at);
            }
            CollectionEdit::Move { id, before } => {
                let at = position(items, &id)
                    .ok_or_else(|| invalid("collection move target disappeared"))?;
                if before.as_deref() != Some(id.as_str()) {
                    let value = items.remove(at);
                    let at = before
                        .as_deref()
                        .map(|id| {
                            position(items, id)
                                .ok_or_else(|| invalid("collection move anchor disappeared"))
                        })
                        .transpose()?
                        .unwrap_or(items.len());
                    items.insert(at, value);
                }
            }
        }
        indexed(collection, key)?;
        candidate.load(registry, capacity)?;
        *self = candidate;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CollectionOverride {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub added: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub removed: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub members: BTreeMap<String, BTreeMap<String, Value>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub replaced: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
}
fn invalid(message: &str) -> DocumentError {
    DocumentError::Invalid(message.into())
}
pub(crate) fn indexed(
    value: &Value,
    key: &str,
) -> Result<(Vec<String>, BTreeMap<String, Value>), DocumentError> {
    let values = value
        .as_array()
        .ok_or_else(|| invalid("identified collection must be an array"))?;
    if values.len() > 256 {
        return Err(invalid("identified collection capacity exceeded"));
    }
    let mut order = Vec::new();
    let mut items = BTreeMap::new();
    for value in values {
        let id = value
            .get(key)
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty() && id.len() <= 256)
            .ok_or_else(|| invalid("collection item requires a bounded string identity"))?;
        if items.insert(id.into(), value.clone()).is_some() {
            return Err(invalid("duplicate collection item identity"));
        }
        order.push(id.into());
    }
    Ok((order, items))
}
impl CollectionOverride {
    pub(crate) fn diff(old: &Value, new: &Value, key: &str) -> Result<Self, DocumentError> {
        let (old_order, old_items) = indexed(old, key)?;
        let (new_order, new_items) = indexed(new, key)?;
        let mut result = Self::default();
        for id in old_items.keys() {
            if !new_items.contains_key(id) {
                result.removed.insert(id.clone());
            }
        }
        for (id, value) in &new_items {
            match old_items.get(id) {
                None => {
                    result.added.insert(id.clone(), value.clone());
                }
                Some(old) if old != value => {
                    let mut members = BTreeMap::new();
                    if crate::prefab_document::member_diff(old, value, "", &mut members) {
                        result.members.insert(id.clone(), members);
                    } else {
                        result.replaced.insert(id.clone(), value.clone());
                    }
                }
                _ => {}
            }
        }
        let mut inherited_order: Vec<_> = old_order
            .into_iter()
            .filter(|id| new_items.contains_key(id))
            .collect();
        // Match apply's implicit order: surviving source items, then additions
        // in deterministic identity order. Adding at the default position must
        // not freeze the source's ordering in an explicit override.
        inherited_order.extend(result.added.keys().cloned());
        if inherited_order != new_order {
            result.order = Some(new_order);
        }
        Ok(result)
    }
    pub(crate) fn apply(&self, value: &mut Value, key: &str) -> Result<(), DocumentError> {
        if self
            .added
            .keys()
            .chain(self.removed.iter())
            .chain(self.members.keys())
            .chain(self.replaced.keys())
            .chain(self.order.iter().flatten())
            .any(|id| id.is_empty() || id.len() > 256)
        {
            return Err(invalid("invalid collection override identity"));
        }
        if self.added.len() + self.removed.len() + self.members.len() + self.replaced.len() > 256 {
            return Err(invalid("collection override capacity exceeded"));
        }
        let (source_order, mut items) = indexed(value, key)?;
        for id in &self.removed {
            if self.added.contains_key(id)
                || self.members.contains_key(id)
                || self.replaced.contains_key(id)
            {
                return Err(invalid("conflicting collection overrides"));
            }
            items.remove(id);
        }
        for (id, item) in &self.added {
            if items.contains_key(id)
                || self.members.contains_key(id)
                || self.replaced.contains_key(id)
            {
                return Err(invalid("collection addition identity collision"));
            }
            if item.get(key).and_then(Value::as_str) != Some(id) {
                return Err(invalid("collection addition identity mismatch"));
            }
            items.insert(id.clone(), item.clone());
        }
        for (id, replacement) in &self.replaced {
            if self.members.contains_key(id)
                || !items.contains_key(id)
                || replacement.get(key).and_then(Value::as_str) != Some(id)
            {
                return Err(invalid("invalid collection replacement"));
            }
            items.insert(id.clone(), replacement.clone());
        }
        for (id, members) in &self.members {
            let item = items
                .get_mut(id)
                .ok_or_else(|| invalid("overridden collection item disappeared"))?;
            if members.len() > 256 {
                return Err(invalid("collection member capacity exceeded"));
            }
            for (path, replacement) in members {
                if members
                    .keys()
                    .any(|other| other != path && path.starts_with(&format!("{other}/")))
                {
                    return Err(invalid("overlapping collection member overrides"));
                }
                *crate::prefab_document::member_mut(item, path)? = replacement.clone();
            }
            if item.get(key).and_then(Value::as_str) != Some(id) {
                return Err(invalid("collection override changed identity"));
            }
        }
        let mut order = if let Some(authored) = &self.order {
            if authored.len() > 256
                || authored.iter().collect::<BTreeSet<_>>().len() != authored.len()
            {
                return Err(invalid("invalid collection order"));
            }
            authored
                .iter()
                .filter(|id| items.contains_key(*id))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            source_order
                .iter()
                .filter(|id| items.contains_key(*id))
                .cloned()
                .collect()
        };
        // New source items follow the nearest surviving source sibling. They are
        // inherited even when the instance has an explicit local order.
        for (index, id) in source_order.iter().enumerate() {
            if !items.contains_key(id) || order.contains(id) {
                continue;
            }
            let position = source_order[..index]
                .iter()
                .rev()
                .find_map(|previous| {
                    order
                        .iter()
                        .position(|item| item == previous)
                        .map(|at| at + 1)
                })
                .or_else(|| {
                    source_order[index + 1..]
                        .iter()
                        .find_map(|next| order.iter().position(|item| item == next))
                })
                .unwrap_or(order.len());
            order.insert(position, id.clone());
        }
        for id in items.keys() {
            if !order.contains(id) {
                order.push(id.clone());
            }
        }
        let candidate = Value::Array(
            order
                .into_iter()
                .map(|id| items.remove(&id).expect("ordered item exists"))
                .collect(),
        );
        indexed(&candidate, key)?;
        *value = candidate;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_additions_do_not_freeze_inherited_order() {
        let source = serde_json::json!([
            {"id":"a","value":1}, {"id":"b","value":2}, {"id":"deleted","value":3}
        ]);
        let edited = serde_json::json!([
            {"id":"a","value":9}, {"id":"b","value":2},
            {"id":"local-a","value":4}, {"id":"local-z","value":5}
        ]);
        let changes = super::CollectionOverride::diff(&source, &edited, "id").unwrap();
        assert!(changes.order.is_none());
        let mut round_trip = source.clone();
        changes.apply(&mut round_trip, "id").unwrap();
        assert_eq!(round_trip, edited);
        let mut updated = serde_json::json!([
            {"id":"b","value":20}, {"id":"new","value":6},
            {"id":"a","value":10}, {"id":"deleted","value":30}
        ]);
        changes.apply(&mut updated, "id").unwrap();
        assert_eq!(
            updated,
            serde_json::json!([
                {"id":"b","value":20}, {"id":"new","value":6},
                {"id":"a","value":9}, {"id":"local-a","value":4}, {"id":"local-z","value":5}
            ])
        );
        let interleaved = serde_json::json!([
            {"id":"local-z","value":5}, {"id":"a","value":9},
            {"id":"b","value":2}, {"id":"local-a","value":4}
        ]);
        let explicit = super::CollectionOverride::diff(&source, &interleaved, "id").unwrap();
        assert!(explicit.order.is_some());
        let mut round_trip = source;
        explicit.apply(&mut round_trip, "id").unwrap();
        assert_eq!(round_trip, interleaved);
    }

    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_create_empty_collection_items_with_unique_caller_ids_and_validation() {
        #[derive(Serialize, Deserialize)]
        struct Item {
            id: String,
            count: u32,
        }
        #[derive(Serialize, Deserialize)]
        struct List {
            items: Vec<Item>,
        }
        let mut registry = crate::ComponentRegistry::default();
        registry.register::<List>("list").unwrap();
        assert!(
            registry
                .set_collection_default("list", "/items", json!({"count":4}))
                .is_err()
        );
        registry
            .declare_identified_collection("list", "/items", "id")
            .unwrap();
        assert!(registry.new_collection_item("list", "/items", "a").is_err());
        registry
            .set_collection_default(
                "list",
                "/items",
                json!({"id":"fixed-template-id","count":4}),
            )
            .unwrap();
        assert_eq!(
            registry.new_collection_item("list", "/items", "a").unwrap(),
            json!({"id":"a","count":4})
        );
        assert_eq!(
            registry.new_collection_item("list", "/items", "b").unwrap(),
            json!({"id":"b","count":4})
        );
        assert!(
            registry
                .set_collection_default("list", "/items", json!([]))
                .is_err()
        );
        assert!(
            registry
                .set_collection_default("list", "/items", json!({"large":"x".repeat(65_536)}))
                .is_err()
        );
        assert_eq!(
            registry.new_collection_item("list", "/items", "a").unwrap()["count"],
            4
        );
        assert!(registry.new_collection_item("list", "/items", "").is_err());
        let document = crate::SceneDocument::from_json(
            &json!({"version":1,"objects":[{
                "id":"root","parent":null,"name":"root","active":true,
                "translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],
                "components":{"list":{"items":[]}}
            }]})
            .to_string(),
        )
        .unwrap();
        let mut history =
            crate::SceneHistory::new(document.clone(), &registry, 128, 16, 1048576).unwrap();
        let root = crate::ObjectId("root".into());
        history
            .edit_collection(
                &registry,
                &root,
                "list",
                "/items",
                CollectionEdit::InsertDefault {
                    id: "a".into(),
                    before: None,
                },
            )
            .unwrap();
        let added = history.current().clone();
        assert!(
            history
                .edit_collection(
                    &registry,
                    &root,
                    "list",
                    "/items",
                    CollectionEdit::InsertDefault {
                        id: "a".into(),
                        before: None
                    }
                )
                .is_err()
        );
        assert_eq!(history.current(), &added);
        assert!(history.undo());
        assert_eq!(history.current(), &document);
        registry
            .set_collection_default("list", "/items", json!({"count":"invalid"}))
            .unwrap();
        assert!(
            history
                .edit_collection(
                    &registry,
                    &root,
                    "list",
                    "/items",
                    CollectionEdit::InsertDefault {
                        id: "b".into(),
                        before: None
                    }
                )
                .is_err()
        );
        assert_eq!(history.current(), &document);
        assert!(history.redo());
        assert_eq!(history.current(), &added);
    }

    #[test]
    fn collection_authoring_operations_use_ids_validate_and_preserve_history() {
        #[derive(Serialize, Deserialize)]
        struct Item {
            id: String,
            target: crate::ObjectId,
            value: f32,
        }
        #[derive(Serialize, Deserialize)]
        struct List {
            items: Vec<Item>,
        }
        let mut registry = crate::ComponentRegistry::default();
        registry
            .register_with_references::<List>("list", |list| {
                list.items.iter().map(|item| item.target.clone()).collect()
            })
            .unwrap();
        registry
            .declare_identified_collection("list", "/items", "id")
            .unwrap();
        let document = crate::SceneDocument::from_json(&json!({"version":1,"objects":[{
            "id":"root","parent":null,"name":"root","active":true,
            "translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],
            "components":{"list":{"items":[{"id":"a","target":"root","value":1},{"id":"b","target":"root","value":2}]}}
        }]}).to_string()).unwrap();
        let mut history = crate::SceneHistory::new_with_metadata(
            document.clone(),
            json!({"source":"retained"}),
            &registry,
            128,
            16,
            1048576,
        )
        .unwrap();
        let root = crate::ObjectId("root".into());
        history
            .edit_collection(
                &registry,
                &root,
                "list",
                "/items",
                CollectionEdit::Insert {
                    value: json!({"id":"new","target":"root","value":3}),
                    before: Some("a".into()),
                },
            )
            .unwrap();
        history
            .edit_collection(
                &registry,
                &root,
                "list",
                "/items",
                CollectionEdit::Move {
                    id: "b".into(),
                    before: Some("a".into()),
                },
            )
            .unwrap();
        let moved = history.current().clone();
        let items = moved.objects[0].components["list"]["items"]
            .as_array()
            .unwrap();
        assert_eq!(
            items
                .iter()
                .map(|item| item["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["new", "b", "a"]
        );
        history
            .edit_collection(
                &registry,
                &root,
                "list",
                "/items",
                CollectionEdit::Remove { id: "a".into() },
            )
            .unwrap();
        assert!(history.undo());
        assert_eq!(history.current(), &moved);
        for edit in [
            CollectionEdit::Insert {
                value: json!({"id":"b","target":"root","value":3}),
                before: None,
            },
            CollectionEdit::Insert {
                value: json!({"id":"bad","target":"missing","value":3}),
                before: None,
            },
            CollectionEdit::Insert {
                value: json!({"id":"bad","target":"root","value":"invalid"}),
                before: None,
            },
            CollectionEdit::Move {
                id: "b".into(),
                before: Some("missing".into()),
            },
            CollectionEdit::Remove {
                id: "missing".into(),
            },
        ] {
            assert!(
                history
                    .edit_collection(&registry, &root, "list", "/items", edit)
                    .is_err()
            );
            assert_eq!(history.current(), &moved);
            assert_eq!(history.metadata(), &json!({"source":"retained"}));
        }
        assert!(
            !history
                .edit_collection(
                    &registry,
                    &root,
                    "list",
                    "/items",
                    CollectionEdit::Move {
                        id: "b".into(),
                        before: Some("b".into())
                    }
                )
                .unwrap()
        );
        assert!(history.redo());
        assert_eq!(
            history.current().objects[0].components["list"]["items"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(history.undo());
        assert!(history.undo());
        assert!(history.undo());
        assert_eq!(history.current(), &document);
    }

    #[test]
    fn failed_collection_changes_leave_original_value_intact() {
        let original = json!([{"id":"a","gain":1}]);
        let invalid_cases = [
            CollectionOverride {
                added: BTreeMap::from([("a".into(), json!({"id":"a"}))]),
                ..Default::default()
            },
            CollectionOverride {
                added: BTreeMap::from([("b".into(), json!({"id":"wrong"}))]),
                ..Default::default()
            },
            CollectionOverride {
                members: BTreeMap::from([(
                    "missing".into(),
                    BTreeMap::from([("/gain".into(), json!(2))]),
                )]),
                ..Default::default()
            },
            CollectionOverride {
                members: BTreeMap::from([(
                    "a".into(),
                    BTreeMap::from([("/id".into(), json!("b"))]),
                )]),
                ..Default::default()
            },
            CollectionOverride {
                order: Some(vec!["a".into(), "a".into()]),
                ..Default::default()
            },
            CollectionOverride {
                removed: BTreeSet::from([String::new()]),
                ..Default::default()
            },
        ];
        for changes in invalid_cases {
            let mut value = original.clone();
            assert!(changes.apply(&mut value, "id").is_err());
            assert_eq!(value, original);
        }
        assert!(indexed(&json!([{"id":"a"}, {"id":"a"}]), "id").is_err());
        assert!(indexed(&json!([{"id":3}]), "id").is_err());
    }

    #[test]
    fn new_source_items_keep_relative_order_around_surviving_anchors() {
        let mut value = json!([{"id":"first"},{"id":"a"},{"id":"middle"},{"id":"b"},{"id":"last"}]);
        let changes = CollectionOverride {
            order: Some(vec!["b".into(), "a".into(), "vanished".into()]),
            ..Default::default()
        };
        changes.apply(&mut value, "id").unwrap();
        assert_eq!(
            value
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["b", "last", "first", "a", "middle"]
        );
    }
}
