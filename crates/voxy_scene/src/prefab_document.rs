//! Persistent prefab composition resolves into the existing scene document loader.
use crate::{ComponentRegistry, DocumentError, LoadedScene, ObjectId, SceneDocument, SceneObject};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// A scene or prefab asset. Instance sources are stable asset IDs, not paths.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PrefabSceneDocument {
    pub version: u32,
    pub objects: Vec<SceneObject>,
    #[serde(default)]
    pub instances: Vec<PrefabInstance>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PrefabInstance {
    /// Durable instance identity, independent of source revisions and runtime slots.
    pub id: ObjectId,
    pub asset: String,
    pub parent: Option<ObjectId>,
    /// Source-local IDs; nested targets use `instance_object_id`.
    #[serde(default)]
    pub overrides: BTreeMap<ObjectId, ObjectOverride>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObjectOverride {
    #[serde(default)]
    pub deleted: bool,
    pub parent: Option<ParentOverride>,
    pub name: Option<String>,
    pub active: Option<bool>,
    pub translation: Option<[f32; 3]>,
    pub rotation: Option<[f32; 4]>,
    pub scale: Option<[f32; 3]>,
    /// Some replaces/adds a registered component; None removes it.
    #[serde(default)]
    pub components: BTreeMap<String, Option<Value>>,
    /// Existing object-member JSON pointers. Null is a value; arrays are atomic.
    /// A schema cannot also have a whole-component override.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub component_members: BTreeMap<String, BTreeMap<String, Value>>,
    /// Declared collection paths; items are addressed by durable IDs.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub component_collections: BTreeMap<String, BTreeMap<String, crate::CollectionOverride>>,
}
impl ObjectOverride {
    fn apply(
        &self,
        object: &mut SceneObject,
        registry: &ComponentRegistry,
    ) -> Result<(), DocumentError> {
        if let Some(name) = &self.name {
            object.name.clone_from(name);
        }
        if let Some(value) = self.active {
            object.active = value;
        }
        if let Some(value) = self.translation {
            object.translation = value;
        }
        if let Some(value) = self.rotation {
            object.rotation = value;
        }
        if let Some(value) = self.scale {
            object.scale = value;
        }
        for (schema, value) in &self.components {
            if let Some(value) = value {
                object.components.insert(schema.clone(), value.clone());
            } else {
                object.components.remove(schema);
            }
        }
        for (schema, members) in &self.component_members {
            if self.components.contains_key(schema) || members.len() > 256 {
                return Err(invalid(
                    "conflicting or oversized component member overrides",
                ));
            }
            let component = object
                .components
                .get_mut(schema)
                .ok_or_else(|| invalid(format!("missing member override component {schema}")))?;
            for (path, value) in members {
                if members
                    .keys()
                    .any(|other| other != path && path.starts_with(&format!("{other}/")))
                {
                    return Err(invalid("overlapping component member overrides"));
                }
                *member_mut(component, path)? = value.clone();
            }
        }
        for (schema, collections) in &self.component_collections {
            if self.components.contains_key(schema) || collections.len() > 16 {
                return Err(invalid("conflicting collection component override"));
            }
            let declarations = registry.identified_collections(schema)?;
            let component = object
                .components
                .get_mut(schema)
                .ok_or_else(|| invalid("missing collection component"))?;
            for (path, changes) in collections {
                let key = declarations
                    .get(path)
                    .ok_or_else(|| invalid("undeclared identified collection"))?;
                if self.component_members.get(schema).is_some_and(|members| {
                    members.keys().any(|member| {
                        member == path
                            || member.starts_with(&format!("{path}/"))
                            || path.starts_with(&format!("{member}/"))
                    })
                }) {
                    return Err(invalid("overlapping member and collection overrides"));
                }
                let array = if path.is_empty() {
                    &mut *component
                } else {
                    member_mut(component, path)?
                };
                changes.apply(array, key)?;
            }
        }
        Ok(())
    }
}

pub(crate) fn member_mut<'a>(
    mut value: &'a mut Value,
    path: &str,
) -> Result<&'a mut Value, DocumentError> {
    if !path.starts_with('/') || path.len() > 1024 || path.split('/').count() > 33 {
        return Err(invalid("invalid component member path"));
    }
    for token in path[1..].split('/') {
        let mut key = String::new();
        let mut chars = token.chars();
        while let Some(ch) = chars.next() {
            if ch == '~' {
                key.push(match chars.next() {
                    Some('0') => '~',
                    Some('1') => '/',
                    _ => return Err(invalid("invalid component member escape")),
                });
            } else {
                key.push(ch);
            }
        }
        value = value
            .as_object_mut()
            .and_then(|object| object.get_mut(&key))
            .ok_or_else(|| invalid(format!("missing or non-object component member {path}")))?;
    }
    Ok(value)
}

pub(crate) fn member_diff(
    old: &Value,
    new: &Value,
    path: &str,
    result: &mut BTreeMap<String, Value>,
) -> bool {
    if old == new {
        return true;
    }
    if let (Some(a), Some(b)) = (old.as_object(), new.as_object()) {
        if a.keys().eq(b.keys()) && path.split('/').count() < 32 {
            for (key, value) in a {
                let encoded = key.replace('~', "~0").replace('/', "~1");
                if !member_diff(value, &b[key], &format!("{path}/{encoded}"), result) {
                    return false;
                }
            }
            return true;
        }
    }
    if path.is_empty() || path.len() > 1024 {
        return false;
    }
    result.insert(path.to_owned(), new.clone());
    result.len() <= 256
}
/// Explicit wrapper distinguishes no parent override from reparenting to a root.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ParentOverride {
    pub parent: Option<ObjectId>,
    /// External parents bypass source-local ID remapping.
    #[serde(default)]
    pub external: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct PrefabLimits {
    pub max_depth: usize,
    pub max_instances: usize,
    pub max_objects: usize,
}
impl Default for PrefabLimits {
    fn default() -> Self {
        Self {
            max_depth: 32,
            max_instances: 4096,
            max_objects: 65_536,
        }
    }
}
/// A validated flattened publication plus every transitively observed asset ID.
#[derive(Debug)]
pub struct ExpandedScene {
    pub document: SceneDocument,
    pub dependencies: BTreeSet<String>,
}
/// Length framing prevents ambiguous concatenation of arbitrary authoring IDs.
#[must_use]
pub fn instance_object_id(instance: &ObjectId, local: &ObjectId) -> ObjectId {
    ObjectId(format!(
        "{}:{}{}:{}",
        instance.0.len(),
        instance.0,
        local.0.len(),
        local.0
    ))
}
fn invalid(message: impl Into<String>) -> DocumentError {
    DocumentError::Invalid(message.into())
}
struct Expansion<'a, F> {
    registry: &'a ComponentRegistry,
    resolve: &'a mut F,
    limits: PrefabLimits,
    instances: usize,
    dependencies: BTreeSet<String>,
    stack: Vec<String>,
}
impl<F: FnMut(&str) -> Result<PrefabSceneDocument, DocumentError>> Expansion<'_, F> {
    fn instantiate(
        &self,
        instance: &PrefabInstance,
        mut child: Vec<SceneObject>,
    ) -> Result<Vec<SceneObject>, DocumentError> {
        let ids: BTreeMap<_, _> = child
            .iter()
            .map(|object| {
                (
                    object.id.clone(),
                    instance_object_id(&instance.id, &object.id),
                )
            })
            .collect();
        if ids.len() != child.len() {
            return Err(invalid("duplicate prefab source object identity"));
        }
        for target in instance.overrides.keys() {
            if !ids.contains_key(target) {
                return Err(invalid(format!(
                    "missing prefab override target {}",
                    target.0
                )));
            }
        }
        child.retain(|object| {
            !instance
                .overrides
                .get(&object.id)
                .is_some_and(|value| value.deleted)
        });
        for object in &mut child {
            let overridden_parent = instance
                .overrides
                .get(&object.id)
                .and_then(|value| value.parent.clone());
            if let Some(overrides) = instance.overrides.get(&object.id) {
                for (schema, value) in &overrides.components {
                    if value.is_none() && !object.components.contains_key(schema) {
                        return Err(invalid(format!("missing overridden component {schema}")));
                    }
                }
                overrides.apply(object, self.registry)?;
            }
            object.id = ids[&object.id].clone();
            object.parent = if let Some(overridden) = overridden_parent {
                overridden.parent.as_ref().map(|parent| {
                    if overridden.external {
                        parent.clone()
                    } else {
                        ids.get(parent).unwrap_or(parent).clone()
                    }
                })
            } else {
                match &object.parent {
                    Some(parent) => Some(
                        ids.get(parent)
                            .cloned()
                            .ok_or_else(|| invalid("prefab parent escapes its source"))?,
                    ),
                    None => instance.parent.clone(),
                }
            };
            for (schema, value) in &mut object.components {
                *value = self.registry.remap_value(schema, value, &ids)?;
            }
        }
        Ok(child)
    }

    fn expand(&mut self, source: &PrefabSceneDocument) -> Result<Vec<SceneObject>, DocumentError> {
        if source.version != 1 {
            return Err(invalid("unsupported prefab scene version"));
        }
        if source.objects.len() > self.limits.max_objects {
            return Err(invalid("prefab object budget exceeded"));
        }
        // Validate before instance framing: a derived nonempty ID must not hide
        // an empty source ID, and authoring baselines require unambiguous IDs too.
        let mut identities = BTreeSet::new();
        for object in &source.objects {
            if object.id.0.is_empty() || !identities.insert(&object.id) {
                return Err(invalid("empty or duplicate prefab source object identity"));
            }
        }
        let mut objects = source.objects.clone();
        let mut instances = BTreeSet::new();
        for instance in &source.instances {
            if instance.id.0.is_empty()
                || instance.asset.is_empty()
                || !instances.insert(&instance.id)
            {
                return Err(invalid(
                    "empty or duplicate prefab instance identity/source",
                ));
            }
            self.instances = self
                .instances
                .checked_add(1)
                .ok_or_else(|| invalid("prefab instance budget overflow"))?;
            if self.instances > self.limits.max_instances {
                return Err(invalid("prefab instance budget exceeded"));
            }
            if self.stack.len() >= self.limits.max_depth {
                return Err(invalid("prefab nesting budget exceeded"));
            }
            if self.stack.contains(&instance.asset) {
                return Err(invalid(format!(
                    "prefab dependency cycle: {}",
                    instance.asset
                )));
            }
            self.dependencies.insert(instance.asset.clone());
            self.stack.push(instance.asset.clone());
            let child = (self.resolve)(&instance.asset)?;
            let child = self.expand(&child)?;
            self.stack.pop();
            if child.len() > self.limits.max_objects.saturating_sub(objects.len()) {
                return Err(invalid("prefab expanded object budget exceeded"));
            }
            objects.extend(self.instantiate(instance, child)?);
        }
        Ok(objects)
    }
}
impl PrefabSceneDocument {
    /// Source revisions are resolved by the caller's existing asset pipeline.
    /// The caller must keep observations/revision validation around publication.
    /// # Errors
    /// Rejects cycles, quotas, stale overrides, bad schemas/references and invalid
    /// scene topology before returning a publication. No destination is mutated.
    pub fn expand(
        &self,
        registry: &ComponentRegistry,
        limits: PrefabLimits,
        mut resolve: impl FnMut(&str) -> Result<Self, DocumentError>,
    ) -> Result<ExpandedScene, DocumentError> {
        let mut expansion = Expansion {
            registry,
            resolve: &mut resolve,
            limits,
            instances: 0,
            dependencies: BTreeSet::new(),
            stack: vec![],
        };
        let document = SceneDocument {
            version: 1,
            objects: expansion.expand(self)?,
        };
        document.load(registry, limits.max_objects)?;
        Ok(ExpandedScene {
            document,
            dependencies: expansion.dependencies,
        })
    }
    /// Uses the ordinary transactional scene loader after bounded composition.
    /// # Errors
    /// Reports the same expansion and scene validation failures as expand.
    pub fn load(
        &self,
        registry: &ComponentRegistry,
        limits: PrefabLimits,
        resolve: impl FnMut(&str) -> Result<Self, DocumentError>,
    ) -> Result<LoadedScene, DocumentError> {
        self.expand(registry, limits, resolve)?
            .document
            .load(registry, limits.max_objects)
    }
}

/// Reads the authoring composition with the same bounded IO as ordinary scenes.
/// # Errors
/// Reports quotas, IO and malformed/unknown JSON fields.
pub fn read_prefab_scene_file(
    path: &std::path::Path,
    max_bytes: usize,
) -> Result<PrefabSceneDocument, crate::SceneFileError> {
    let text = crate::file::read_text(path, max_bytes)?;
    serde_json::from_str(&text).map_err(|error| DocumentError::from(error).into())
}
/// Validates the full dependency expansion before atomic authoring-file replacement.
/// Instance links and overrides are saved intact, rather than baking the flat graph.
/// # Errors
/// Reports quotas/expansion failures before publication and IO errors with the
/// same explicit committed flag as ordinary scene saves.
pub fn save_prefab_scene_file(
    path: &std::path::Path,
    document: &PrefabSceneDocument,
    registry: &ComponentRegistry,
    limits: PrefabLimits,
    max_bytes: usize,
    resolve: impl FnMut(&str) -> Result<PrefabSceneDocument, DocumentError>,
) -> Result<(), crate::SceneFileError> {
    let text = serde_json::to_string_pretty(document).map_err(DocumentError::from)?;
    if text.len() > max_bytes {
        return Err(crate::SceneFileError::TooLarge);
    }
    document.expand(registry, limits, resolve)?;
    crate::file::write_text(path, &text)
}

impl PrefabSceneDocument {
    /// Expands source objects before this scene's instance overrides for authoring
    /// diff capture. Nested asset overrides remain authored in those assets.
    /// This baseline is not validated as a runtime scene: overrides can repair
    /// references or transforms that would otherwise fail final scene validation.
    /// # Errors
    /// Rejects source resolution, identity, nesting and resource quota failures.
    pub fn instance_baseline(
        &self,
        registry: &ComponentRegistry,
        limits: PrefabLimits,
        mut resolve: impl FnMut(&str) -> Result<Self, DocumentError>,
    ) -> Result<SceneDocument, DocumentError> {
        let mut source = self.clone();
        for instance in &mut source.instances {
            instance.overrides.clear();
        }
        let mut expansion = Expansion {
            registry,
            resolve: &mut resolve,
            limits,
            instances: 0,
            dependencies: BTreeSet::new(),
            stack: vec![],
        };
        Ok(SceneDocument {
            version: 1,
            objects: expansion.expand(&source)?,
        })
    }
    /// Captures property/component edits against the expanded publication while
    /// retaining source links. The baseline must come from `instance_baseline` so
    /// reverting edits or resurrecting deleted objects clears obsolete overrides.
    /// # Errors
    /// Rejects invalid edited documents, malformed derived IDs and reference
    /// codecs unable to reverse instance links. Final expansion validates topology.
    // Authoring changes are exact; a tolerance would silently discard small edits.
    #[allow(clippy::float_cmp)]
    pub fn capture_edits(
        &self,
        baseline: &SceneDocument,
        edited: &SceneDocument,
        registry: &ComponentRegistry,
        capacity: usize,
    ) -> Result<Self, DocumentError> {
        edited.load(registry, capacity)?;
        let before: BTreeMap<_, _> = baseline
            .objects
            .iter()
            .map(|object| (&object.id, object))
            .collect();
        let after: BTreeMap<_, _> = edited
            .objects
            .iter()
            .map(|object| (&object.id, object))
            .collect();
        let authored: BTreeSet<_> = self.objects.iter().map(|object| &object.id).collect();
        let mut result = self.clone();
        let mut consumed = BTreeSet::new();
        for instance in &mut result.instances {
            instance.overrides.clear();
            let prefix = format!("{}:{}", instance.id.0.len(), instance.id.0);
            let mut reverse = BTreeMap::new();
            for object in &baseline.objects {
                if authored.contains(&object.id) {
                    continue;
                }
                if let Some(suffix) = object.id.0.strip_prefix(&prefix) {
                    let (length, local) = suffix
                        .split_once(':')
                        .ok_or_else(|| invalid("invalid derived prefab identity"))?;
                    if length.parse::<usize>().ok() != Some(local.len()) {
                        return Err(invalid("invalid derived prefab identity length"));
                    }
                    reverse.insert(object.id.clone(), ObjectId(local.to_owned()));
                }
            }
            for (global, local) in &reverse {
                let old = before[global];
                consumed.insert(global.clone());
                let Some(new) = after.get(global) else {
                    instance.overrides.insert(
                        local.clone(),
                        ObjectOverride {
                            deleted: true,
                            ..ObjectOverride::default()
                        },
                    );
                    continue;
                };
                let overrides = instance.overrides.entry(local.clone()).or_default();
                if old.parent != new.parent {
                    overrides.parent = Some(ParentOverride {
                        external: new
                            .parent
                            .as_ref()
                            .is_some_and(|parent| !reverse.contains_key(parent)),
                        parent: new
                            .parent
                            .as_ref()
                            .map(|parent| reverse.get(parent).unwrap_or(parent).clone()),
                    });
                }
                if old.name != new.name {
                    overrides.name = Some(new.name.clone());
                }
                if old.active != new.active {
                    overrides.active = Some(new.active);
                }
                if old.translation != new.translation {
                    overrides.translation = Some(new.translation);
                }
                if old.rotation != new.rotation {
                    overrides.rotation = Some(new.rotation);
                }
                if old.scale != new.scale {
                    overrides.scale = Some(new.scale);
                }
                // A source item can disappear and later return. Recapturing an
                // unrelated edit must retain the instance's explicit deletion.
                if let Some(previous) = self
                    .instances
                    .iter()
                    .find(|source| source.id == instance.id)
                    .and_then(|source| source.overrides.get(local))
                {
                    for (schema, collections) in &previous.component_collections {
                        let (Some(old_component), Some(new_component)) =
                            (old.components.get(schema), new.components.get(schema))
                        else {
                            continue;
                        };
                        for (path, changes) in collections {
                            let key = registry
                                .identified_collections(schema)?
                                .get(path)
                                .ok_or_else(|| invalid("undeclared collection tombstone"))?;
                            let (_, old_items) = crate::collection_override::indexed(
                                old_component
                                    .pointer(path)
                                    .ok_or_else(|| invalid("missing baseline collection"))?,
                                key,
                            )?;
                            let (_, new_items) = crate::collection_override::indexed(
                                new_component
                                    .pointer(path)
                                    .ok_or_else(|| invalid("missing edited collection"))?,
                                key,
                            )?;
                            let removed = changes
                                .removed
                                .iter()
                                .filter(|id| {
                                    !old_items.contains_key(*id) && !new_items.contains_key(*id)
                                })
                                .cloned()
                                .collect::<std::collections::BTreeSet<_>>();
                            if !removed.is_empty() {
                                overrides
                                    .component_collections
                                    .entry(schema.clone())
                                    .or_default()
                                    .insert(
                                        path.clone(),
                                        crate::CollectionOverride {
                                            removed,
                                            ..Default::default()
                                        },
                                    );
                            }
                        }
                    }
                }
                for schema in old.components.keys().chain(new.components.keys()) {
                    if old.components.get(schema) != new.components.get(schema) {
                        let value = new
                            .components
                            .get(schema)
                            .map(|value| registry.remap_value(schema, value, &reverse))
                            .transpose()?;
                        let legacy = self
                            .instances
                            .iter()
                            .find(|source| source.id == instance.id)
                            .and_then(|source| source.overrides.get(local))
                            .is_some_and(|source| source.components.contains_key(schema));
                        let mut members = BTreeMap::new();
                        let mut collections = BTreeMap::new();
                        let granular = if let (Some(old), Some(new)) =
                            (old.components.get(schema), value.as_ref())
                        {
                            let old = registry.remap_value(schema, old, &reverse)?;
                            let mut ordinary = new.clone();
                            if !legacy {
                                for (path, key) in registry.identified_collections(schema)? {
                                    let old_array = old
                                        .pointer(path)
                                        .ok_or_else(|| invalid("missing source collection"))?;
                                    let new_array = new
                                        .pointer(path)
                                        .ok_or_else(|| invalid("missing edited collection"))?;
                                    if old_array != new_array {
                                        collections.insert(
                                            path.clone(),
                                            crate::CollectionOverride::diff(
                                                old_array, new_array, key,
                                            )?,
                                        );
                                        *ordinary
                                            .pointer_mut(path)
                                            .ok_or_else(|| invalid("missing collection mask"))? =
                                            old_array.clone();
                                    }
                                }
                            }
                            !legacy && member_diff(&old, &ordinary, "", &mut members)
                        } else {
                            false
                        };
                        if granular {
                            if !members.is_empty() {
                                overrides.component_members.insert(schema.clone(), members);
                            }
                            if !collections.is_empty() {
                                let retained = overrides
                                    .component_collections
                                    .entry(schema.clone())
                                    .or_default();
                                for (path, mut changes) in collections {
                                    if let Some(previous) = retained.get(&path) {
                                        changes.removed.extend(previous.removed.iter().cloned());
                                    }
                                    retained.insert(path, changes);
                                }
                            }
                        } else {
                            overrides.component_collections.remove(schema);
                            overrides.components.insert(schema.clone(), value);
                        }
                    }
                }
            }
        }
        for instance in &mut result.instances {
            instance
                .overrides
                .retain(|_, changes| changes != &ObjectOverride::default());
        }
        result.objects = edited
            .objects
            .iter()
            .filter(|object| !consumed.contains(&object.id))
            .cloned()
            .collect();
        Ok(result)
    }
}
