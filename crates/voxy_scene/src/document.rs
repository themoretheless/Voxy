//! Versioned authoring documents. Runtime handles never form durable IDs.
use crate::{Component, NodeId, SceneGraph, SceneGraphError, Transform};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    any::TypeId,
    collections::{BTreeMap, HashMap, HashSet},
};

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[serde(transparent)]
pub struct ObjectId(pub String);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SceneObject {
    pub id: ObjectId,
    pub parent: Option<ObjectId>,
    pub name: String,
    pub active: bool,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    /// Keys are durable registered schema names, never Rust `TypeId` values.
    pub components: BTreeMap<String, Value>,
}
impl SceneObject {
    fn transform(&self) -> Transform {
        Transform {
            translation: Vec3::from_array(self.translation),
            rotation: Quat::from_array(self.rotation),
            scale: Vec3::from_array(self.scale),
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SceneDocument {
    pub version: u32,
    pub objects: Vec<SceneObject>,
}
#[derive(Debug)]
pub enum DocumentError {
    Json(serde_json::Error),
    Scene(SceneGraphError),
    Invalid(String),
}
impl std::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scene document error: {self:?}")
    }
}
impl std::error::Error for DocumentError {}
impl From<serde_json::Error> for DocumentError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl From<SceneGraphError> for DocumentError {
    fn from(e: SceneGraphError) -> Self {
        Self::Scene(e)
    }
}

type ReferenceVisitor = Box<dyn Fn(&Component) -> Vec<ObjectId> + Send + Sync>;
type ReferenceRemapper = Box<
    dyn Fn(&mut Component, &BTreeMap<ObjectId, ObjectId>) -> Result<(), DocumentError>
        + Send
        + Sync,
>;
type Decoder = fn(Value) -> Result<Component, serde_json::Error>;
type Encoder = fn(&Component) -> Result<Value, DocumentError>;
struct Codec {
    type_id: TypeId,
    decode: Decoder,
    encode: Encoder,
    references: ReferenceVisitor,
    remap: Option<ReferenceRemapper>,
    collections: BTreeMap<String, String>,
    collection_defaults: BTreeMap<String, Value>,
}
impl std::fmt::Debug for Codec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Codec")
            .field("type_id", &self.type_id)
            .finish_non_exhaustive()
    }
}
#[derive(Debug, Default)]
pub struct ComponentRegistry {
    codecs: BTreeMap<String, Codec>,
}
impl ComponentRegistry {
    /// Captures registered data components; unregistered runtime-only components
    /// remain owned by the live graph. The final document validates references.
    /// # Errors
    /// Rejects invalid node handles and component encoding failures.
    pub fn capture_registered_components(
        &self,
        scene: &SceneGraph,
        node: crate::NodeId,
    ) -> Result<BTreeMap<String, Value>, DocumentError> {
        let node = scene.node(node)?;
        self.codecs
            .iter()
            .filter_map(|(name, codec)| {
                node.components
                    .get(&codec.type_id)
                    .map(|component| (codec.encode)(component).map(|value| (name.clone(), value)))
            })
            .collect()
    }
    /// Copies registered durable components to another node using their codecs.
    /// Object references retain their original targets. Runtime-only components
    /// are not copied. Encoding/decoding completes before the destination changes.
    /// # Errors
    /// Rejects invalid node handles or component codec failures.
    pub fn copy_registered_components(
        &self,
        scene: &mut SceneGraph,
        source: crate::NodeId,
        destination: crate::NodeId,
    ) -> Result<(), DocumentError> {
        scene.node(destination)?;
        let values = self.capture_registered_components(scene, source)?;
        let decoded = values
            .into_iter()
            .map(|(name, value)| {
                let codec = &self.codecs[&name];
                (codec.decode)(value).map(|component| (codec.type_id, component))
            })
            .collect::<Result<Vec<_>, _>>()?;
        scene.node_mut(destination)?.components.extend(decoded);
        Ok(())
    }

    /// Register durable schema names such as `game.health.v1`. Register migrations
    /// explicitly when changing schemas. Components must store durable `ObjectId`
    /// references instead of runtime `NodeId` values.
    /// # Errors
    /// Rejects empty names, duplicate names and duplicate Rust types.
    pub fn register<T: Serialize + DeserializeOwned + Send + Sync + 'static>(
        &mut self,
        name: &str,
    ) -> Result<(), DocumentError> {
        if name.is_empty()
            || self.codecs.contains_key(name)
            || self.codecs.values().any(|c| c.type_id == TypeId::of::<T>())
        {
            return Err(DocumentError::Invalid(format!(
                "duplicate or empty component schema: {name}"
            )));
        }
        self.codecs.insert(
            name.to_owned(),
            Codec {
                type_id: TypeId::of::<T>(),
                decode: |value| {
                    Ok(Component(
                        Box::new(serde_json::from_value::<T>(value)?),
                        crate::next_component_revision(),
                    ))
                },
                references: Box::new(|_| Vec::new()),
                remap: None,
                collections: BTreeMap::new(),
                collection_defaults: BTreeMap::new(),
                encode: |component| {
                    let value = component
                        .0
                        .downcast_ref::<T>()
                        .ok_or_else(|| DocumentError::Invalid("component type mismatch".into()))?;
                    Ok(serde_json::to_value(value)?)
                },
            },
        );
        Ok(())
    }
    /// Declares an array of objects whose string member is a durable item ID.
    /// Declaration is explicit: ordinary vectors and arrays remain atomic.
    /// # Errors
    /// Rejects unknown codecs, overlapping paths, invalid paths and quotas.
    pub fn declare_identified_collection(
        &mut self,
        schema: &str,
        path: &str,
        identity_member: &str,
    ) -> Result<(), DocumentError> {
        let codec = self
            .codecs
            .get_mut(schema)
            .ok_or_else(|| DocumentError::Invalid("unknown collection schema".into()))?;
        if identity_member.is_empty()
            || identity_member.len() > 256
            || path.len() > 1024
            || (!path.is_empty() && !path.starts_with('/'))
            || path.split('/').count() > 33
            || codec.collections.len() >= 16
            || codec.collections.keys().any(|other| {
                other == path
                    || path.starts_with(&format!("{other}/"))
                    || other.starts_with(&format!("{path}/"))
            })
        {
            return Err(DocumentError::Invalid(
                "invalid or overlapping collection declaration".into(),
            ));
        }
        // Validate escape sequences through the existing member pointer decoder.
        if !path.is_empty() {
            for part in path[1..].split('/') {
                let mut chars = part.chars();
                while let Some(character) = chars.next() {
                    if character == '~' && !matches!(chars.next(), Some('0' | '1')) {
                        return Err(DocumentError::Invalid(
                            "invalid collection pointer escape".into(),
                        ));
                    }
                }
            }
        }
        codec
            .collections
            .insert(path.into(), identity_member.into());
        Ok(())
    }
    /// Supplies new-item data for an already declared collection. Identity is
    /// supplied separately when creating each item. Final scene validation still
    /// checks the concrete component and any object references.
    /// # Errors
    /// Rejects undeclared collections, non-object or oversized templates.
    pub fn set_collection_default(
        &mut self,
        schema: &str,
        path: &str,
        mut value: Value,
    ) -> Result<(), DocumentError> {
        let codec = self
            .codecs
            .get_mut(schema)
            .ok_or_else(|| DocumentError::Invalid("unknown collection schema".into()))?;
        let key = codec
            .collections
            .get(path)
            .ok_or_else(|| DocumentError::Invalid("undeclared collection default".into()))?;
        let members = value
            .as_object_mut()
            .ok_or_else(|| DocumentError::Invalid("collection default must be an object".into()))?;
        members.remove(key);
        if serde_json::to_vec(&value)?.len() > 65_536 {
            return Err(DocumentError::Invalid(
                "collection default byte limit exceeded".into(),
            ));
        }
        codec.collection_defaults.insert(path.into(), value);
        Ok(())
    }
    /// Creates item data from codec defaults and a caller-owned durable identity.
    /// # Errors
    /// Rejects missing defaults or empty/oversized identities. Component validity
    /// is checked when the item is inserted into its complete scene context.
    pub fn new_collection_item(
        &self,
        schema: &str,
        path: &str,
        id: &str,
    ) -> Result<Value, DocumentError> {
        if id.is_empty() || id.len() > 256 {
            return Err(DocumentError::Invalid(
                "invalid new collection identity".into(),
            ));
        }
        let codec = self
            .codecs
            .get(schema)
            .ok_or_else(|| DocumentError::Invalid("unknown collection schema".into()))?;
        let key = codec
            .collections
            .get(path)
            .ok_or_else(|| DocumentError::Invalid("undeclared collection".into()))?;
        let mut value = codec
            .collection_defaults
            .get(path)
            .ok_or_else(|| DocumentError::Invalid("collection has no new-item defaults".into()))?
            .clone();
        value
            .as_object_mut()
            .ok_or_else(|| DocumentError::Invalid("invalid collection default".into()))?
            .insert(key.clone(), Value::String(id.into()));
        Ok(value)
    }
    /// Declared collection paths and their string identity members for a codec.
    /// # Errors
    /// Rejects an unknown component schema.
    pub fn identified_collections(
        &self,
        schema: &str,
    ) -> Result<&BTreeMap<String, String>, DocumentError> {
        Ok(&self
            .codecs
            .get(schema)
            .ok_or_else(|| DocumentError::Invalid("unknown collection schema".into()))?
            .collections)
    }
    fn validate_collections(&self, schema: &str, value: &Value) -> Result<(), DocumentError> {
        for (path, key) in self.identified_collections(schema)? {
            let mut candidate = value.clone();
            let collection = if path.is_empty() {
                &mut candidate
            } else {
                crate::prefab_document::member_mut(&mut candidate, path)?
            };
            crate::collection_override::indexed(collection, key)?;
        }
        Ok(())
    }
    /// Registers a schema with an explicit visitor for internal object references.
    /// All returned IDs must exist in the loaded document.
    /// # Errors
    /// Uses the same registration checks as register.
    pub fn register_with_references<T: Serialize + DeserializeOwned + Send + Sync + 'static>(
        &mut self,
        name: &str,
        references: fn(&T) -> Vec<ObjectId>,
    ) -> Result<(), DocumentError> {
        self.register::<T>(name)?;
        self.codecs
            .get_mut(name)
            .ok_or_else(|| DocumentError::Invalid("registered codec missing".into()))?
            .references = Box::new(move |component| {
            component
                .0
                .downcast_ref::<T>()
                .map_or_else(Vec::new, references)
        });
        Ok(())
    }
    /// Registers typed reference discovery and rewriting for prefab instances.
    /// Only `ObjectId` fields declared by this codec are rewritten; ordinary strings
    /// and external asset identifiers are untouched.
    /// # Errors
    /// Rejects duplicate/empty schemas using the ordinary registry checks.
    pub fn register_with_reference_remap<
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
    >(
        &mut self,
        name: &str,
        references: fn(&T) -> Vec<ObjectId>,
        remap: fn(&mut T, &BTreeMap<ObjectId, ObjectId>),
    ) -> Result<(), DocumentError> {
        self.register_with_references::<T>(name, references)?;
        self.codecs
            .get_mut(name)
            .ok_or_else(|| DocumentError::Invalid("registered codec missing".into()))?
            .remap = Some(Box::new(move |component, ids| {
            let value = component
                .0
                .downcast_mut::<T>()
                .ok_or_else(|| DocumentError::Invalid("decoded component type mismatch".into()))?;
            remap(value, ids);
            Ok(())
        }));
        Ok(())
    }

    pub(crate) fn remap_value(
        &self,
        name: &str,
        value: &Value,
        ids: &BTreeMap<ObjectId, ObjectId>,
    ) -> Result<Value, DocumentError> {
        self.validate_collections(name, value)?;
        let codec = self
            .codecs
            .get(name)
            .ok_or_else(|| DocumentError::Invalid(format!("unknown schema {name}")))?;
        let mut component = (codec.decode)(value.clone())?;
        let before = (codec.references)(&component);
        let expected: Vec<_> = before
            .iter()
            .map(|id| ids.get(id).unwrap_or(id).clone())
            .collect();
        if before != expected {
            let remap = codec.remap.as_ref().ok_or_else(|| {
                DocumentError::Invalid(format!(
                    "schema {name} has instance references but no reference remapper"
                ))
            })?;
            remap(&mut component, ids)?;
            if (codec.references)(&component) != expected {
                return Err(DocumentError::Invalid(format!(
                    "schema {name} did not remap its declared references"
                )));
            }
        }
        let encoded = (codec.encode)(&component)?;
        self.validate_collections(name, &encoded)?;
        Ok(encoded)
    }
}

/// A fully loaded world owns the mapping between durable IDs and runtime handles.
#[derive(Debug)]
pub struct LoadedScene {
    pub graph: SceneGraph,
    ids: BTreeMap<ObjectId, NodeId>,
}
impl LoadedScene {
    #[must_use]
    pub fn resolve(&self, id: &ObjectId) -> Option<NodeId> {
        self.ids
            .get(id)
            .copied()
            .filter(|id| self.graph.node(*id).is_ok())
    }
    /// Maps a live runtime handle back to its durable authoring identity.
    #[must_use]
    pub fn identity(&self, node: NodeId) -> Option<&ObjectId> {
        self.graph.node(node).ok()?;
        self.ids
            .iter()
            .find_map(|(id, candidate)| (*candidate == node).then_some(id))
    }

    /// Assigns a durable identity to a newly created live object.
    /// # Errors
    /// Rejects stale handles, empty/duplicate IDs and already identified objects.
    pub fn identify(&mut self, node: NodeId, id: ObjectId) -> Result<(), DocumentError> {
        self.graph.node(node)?;
        if id.0.is_empty()
            || self.ids.contains_key(&id)
            || self.ids.values().any(|existing| *existing == node)
        {
            return Err(DocumentError::Invalid(
                "duplicate or empty object identity".into(),
            ));
        }
        self.ids.insert(id, node);
        Ok(())
    }

    /// Captures all live objects. Newly spawned objects need an explicit durable ID.
    /// # Errors
    /// Rejects unmapped objects and unregistered components rather than losing data.
    pub fn capture(&self, registry: &ComponentRegistry) -> Result<SceneDocument, DocumentError> {
        let reverse: HashMap<_, _> = self
            .ids
            .iter()
            .map(|(id, node)| (*node, id.clone()))
            .collect();
        let mut objects = Vec::new();
        for (node_id, local, parent) in self.graph.nodes() {
            let id = reverse
                .get(&node_id)
                .ok_or_else(|| DocumentError::Invalid("unmapped live object".into()))?
                .clone();
            let node = self.graph.node(node_id)?;
            let mut components = BTreeMap::new();
            for (type_id, component) in &node.components {
                let (name, codec) = registry
                    .codecs
                    .iter()
                    .find(|(_, codec)| codec.type_id == *type_id)
                    .ok_or_else(|| {
                        DocumentError::Invalid(format!("unregistered component on {}", id.0))
                    })?;
                for target in (codec.references)(component) {
                    if self.resolve(&target).is_none() {
                        return Err(DocumentError::Invalid(format!(
                            "missing component reference {}",
                            target.0
                        )));
                    }
                }
                components.insert(name.clone(), (codec.encode)(component)?);
            }
            objects.push(SceneObject {
                id,
                parent: parent
                    .map(|p| {
                        reverse
                            .get(&p)
                            .cloned()
                            .ok_or_else(|| DocumentError::Invalid("unmapped parent".into()))
                    })
                    .transpose()?,
                name: node.name.clone(),
                active: node.active,
                translation: local.translation.to_array(),
                rotation: local.rotation.to_array(),
                scale: local.scale.to_array(),
                components,
            });
        }
        objects.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(SceneDocument {
            version: 1,
            objects,
        })
    }
}
impl SceneDocument {
    /// # Errors
    /// Rejects malformed JSON and unknown top-level/object fields.
    pub fn from_json(text: &str) -> Result<Self, DocumentError> {
        Ok(serde_json::from_str(text)?)
    }
    /// # Errors
    /// Returns JSON serialization errors.
    pub fn to_json(&self) -> Result<String, DocumentError> {
        for object in &self.objects {
            object.transform().matrix()?;
        }
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Loads into a privately staged graph. Failure never changes an existing world.
    /// Order of objects in the file does not constrain parenting.
    /// # Errors
    /// Rejects unsupported versions, duplicate/empty IDs, missing parents, cycles,
    /// invalid transforms, capacity overflow and unknown/malformed components.
    pub fn load(
        &self,
        registry: &ComponentRegistry,
        capacity: usize,
    ) -> Result<LoadedScene, DocumentError> {
        if self.version != 1 {
            return Err(DocumentError::Invalid(format!(
                "unsupported scene version {}",
                self.version
            )));
        }
        if self.objects.len() > capacity {
            return Err(SceneGraphError::Capacity.into());
        }
        let mut unique = HashSet::new();
        for object in &self.objects {
            if object.id.0.is_empty() || !unique.insert(&object.id) {
                return Err(DocumentError::Invalid(
                    "empty or duplicate object ID".into(),
                ));
            }
        }
        let mut graph = SceneGraph::new(capacity);
        let mut ids = BTreeMap::new();
        for object in &self.objects {
            let id = graph.spawn(None, object.transform())?;
            graph.set_name(id, object.name.clone())?;
            graph.set_active(id, object.active)?;
            for (name, value) in &object.components {
                registry.validate_collections(name, value)?;
                let codec = registry
                    .codecs
                    .get(name)
                    .ok_or_else(|| DocumentError::Invalid(format!("unknown schema {name}")))?;
                let component = (codec.decode)(value.clone())?;
                if !codec.collections.is_empty() {
                    registry.validate_collections(name, &(codec.encode)(&component)?)?;
                }
                graph
                    .node_mut(id)?
                    .components
                    .insert(codec.type_id, component);
            }
            ids.insert(object.id.clone(), id);
        }
        for node in graph.slots.iter().filter_map(|slot| slot.node.as_ref()) {
            for codec in registry.codecs.values() {
                if let Some(component) = node.components.get(&codec.type_id) {
                    for target in (codec.references)(component) {
                        if !ids.contains_key(&target) {
                            return Err(DocumentError::Invalid(format!(
                                "missing component reference {}",
                                target.0
                            )));
                        }
                    }
                }
            }
        }
        for object in &self.objects {
            if let Some(parent) = &object.parent {
                let parent = ids.get(parent).copied().ok_or_else(|| {
                    DocumentError::Invalid(format!("missing parent {}", parent.0))
                })?;
                graph.reparent(ids[&object.id], Some(parent))?;
            }
        }
        Ok(LoadedScene { graph, ids })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Target {
        health: u32,
        target: ObjectId,
    }
    fn object(id: &str, parent: Option<&str>) -> SceneObject {
        SceneObject {
            id: ObjectId(id.into()),
            parent: parent.map(|p| ObjectId(p.into())),
            name: id.into(),
            active: true,
            translation: [1.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            components: BTreeMap::new(),
        }
    }
    #[test]
    fn component_copy_retains_durable_reference_and_excludes_runtime_state() {
        let mut registry = ComponentRegistry::default();
        registry
            .register_with_references::<Target>("game.target.v1", |value| {
                vec![value.target.clone()]
            })
            .unwrap();
        let mut scene = SceneGraph::new(3);
        let source = scene.spawn(None, Transform::default()).unwrap();
        let destination = scene.spawn(None, Transform::default()).unwrap();
        scene
            .insert_component(
                source,
                Target {
                    health: 75,
                    target: ObjectId("root".into()),
                },
            )
            .unwrap();
        scene.insert_component(source, 123_u64).unwrap();
        registry
            .copy_registered_components(&mut scene, source, destination)
            .unwrap();
        let copied = scene.component::<Target>(destination).unwrap().unwrap();
        assert_eq!(copied.health, 75);
        assert_eq!(copied.target, ObjectId("root".into()));
        assert!(scene.component::<u64>(destination).unwrap().is_none());
        scene.remove_subtree(source).unwrap();
        let before = registry
            .capture_registered_components(&scene, destination)
            .unwrap();
        assert!(
            registry
                .copy_registered_components(&mut scene, source, destination)
                .is_err()
        );
        assert_eq!(
            registry
                .capture_registered_components(&scene, destination)
                .unwrap(),
            before
        );
    }

    #[test]
    fn roundtrip_remaps_durable_ids_and_preserves_components_activity_and_parenting() {
        let mut registry = ComponentRegistry::default();
        registry
            .register_with_references::<Target>("game.target.v1", |value| {
                vec![value.target.clone()]
            })
            .unwrap();
        let mut child = object("child", Some("root"));
        child.active = false;
        child.components.insert(
            "game.target.v1".into(),
            serde_json::json!({"health":75,"target":"root"}),
        );
        let document = SceneDocument {
            version: 1,
            objects: vec![child, object("root", None)],
        };
        let first = SceneDocument::from_json(&document.to_json().unwrap())
            .unwrap()
            .load(&registry, 2)
            .unwrap();
        let child = first.resolve(&ObjectId("child".into())).unwrap();
        assert_eq!(first.graph.active_self(child), Ok(false));
        assert_eq!(
            first
                .graph
                .world_matrix(child)
                .unwrap()
                .transform_point3(Vec3::ZERO),
            Vec3::new(2.0, 0.0, 0.0)
        );
        let target = first.graph.component::<Target>(child).unwrap().unwrap();
        assert_eq!(target.health, 75);
        let root = first.resolve(&target.target).unwrap();
        let captured = first.capture(&registry).unwrap();
        let second = captured.load(&registry, 2).unwrap();
        assert_ne!(second.resolve(&target.target).unwrap(), root);
        assert_eq!(second.capture(&registry).unwrap(), captured);
    }
    #[test]
    fn invalid_documents_fail_without_affecting_existing_world() {
        let registry = ComponentRegistry::default();
        let original = SceneDocument {
            version: 1,
            objects: vec![object("a", None)],
        };
        let existing = original.load(&registry, 1).unwrap();
        let before = existing.capture(&registry).unwrap();
        let cases = [
            SceneDocument {
                version: 2,
                objects: vec![],
            },
            SceneDocument {
                version: 1,
                objects: vec![object("a", None), object("a", None)],
            },
            SceneDocument {
                version: 1,
                objects: vec![object("a", Some("missing"))],
            },
            SceneDocument {
                version: 1,
                objects: vec![object("a", Some("b")), object("b", Some("a"))],
            },
        ];
        for document in cases {
            assert!(document.load(&registry, 2).is_err());
        }
        assert!(original.load(&registry, 0).is_err());
        assert_eq!(existing.capture(&registry).unwrap(), before);
    }
    #[test]
    fn unknown_or_malformed_components_are_never_silently_discarded() {
        let mut registry = ComponentRegistry::default();
        registry.register::<u32>("health.v1").unwrap();
        assert!(registry.register::<u32>("other.v1").is_err());
        assert!(registry.register::<String>("health.v1").is_err());
        let mut node = object("a", None);
        node.components.insert("unknown.v1".into(), Value::Null);
        let mut document = SceneDocument {
            version: 1,
            objects: vec![node],
        };
        assert!(document.load(&registry, 1).is_err());
        document.objects[0].components.clear();
        document.objects[0]
            .components
            .insert("health.v1".into(), Value::String("bad".into()));
        assert!(document.load(&registry, 1).is_err());
        document.objects[0].components.clear();
        let mut loaded = document.load(&registry, 1).unwrap();
        let owner = loaded.resolve(&ObjectId("a".into())).unwrap();
        loaded
            .graph
            .insert_component(owner, String::from("unregistered"))
            .unwrap();
        assert!(loaded.capture(&registry).is_err());
        loaded.graph.remove_subtree(owner).unwrap();
        assert!(loaded.resolve(&ObjectId("a".into())).is_none());
    }
    #[test]
    fn registered_references_must_resolve_and_new_objects_need_unique_identity() {
        let mut registry = ComponentRegistry::default();
        registry
            .register_with_references::<ObjectId>("target.v1", |value| vec![value.clone()])
            .unwrap();
        let mut node = object("a", None);
        node.components
            .insert("target.v1".into(), Value::String("missing".into()));
        let mut document = SceneDocument {
            version: 1,
            objects: vec![node],
        };
        assert!(document.load(&registry, 2).is_err());
        document.objects[0].components.clear();
        let mut loaded = document.load(&registry, 2).unwrap();
        let new = loaded.graph.spawn(None, Transform::default()).unwrap();
        assert!(loaded.capture(&registry).is_err());
        assert!(loaded.identify(new, ObjectId("a".into())).is_err());
        loaded.identify(new, ObjectId("b".into())).unwrap();
        assert_eq!(loaded.capture(&registry).unwrap().objects.len(), 2);
    }
}
