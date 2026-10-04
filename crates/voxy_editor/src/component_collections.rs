//! Collection controls address durable owners, schemas and item IDs, never row indices.
use voxy_scene::{CollectionEdit, ComponentRegistry, SceneObject};
pub(super) type DeletedItems = std::collections::BTreeMap<[u8; 32], Vec<([u8; 32], String)>>;

pub(super) struct Collection<'a> {
    pub schema: &'a str,
    pub path: &'a str,
    pub identity: &'a str,
    pub items: &'a [serde_json::Value],
    pub key: [u8; 32],
}
pub(super) fn item_key(id: &str) -> [u8; 32] {
    *blake3::hash(id.as_bytes()).as_bytes()
}
pub(super) fn collections<'a>(
    object: &'a SceneObject,
    registry: &'a ComponentRegistry,
) -> Result<Vec<Collection<'a>>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();
    for (schema, value) in &object.components {
        for (path, identity) in registry.identified_collections(schema)? {
            let items = value
                .pointer(path)
                .and_then(serde_json::Value::as_array)
                .ok_or("declared collection is missing")?;
            let key = *blake3::hash(&serde_json::to_vec(&(&object.id, schema, path))?).as_bytes();
            result.push(Collection {
                schema,
                path,
                identity,
                items,
                key,
            });
        }
    }
    Ok(result)
}
impl crate::App {
    pub(super) fn prefab_collection_order_resets(
        &self,
        document: &voxy_scene::SceneDocument,
    ) -> Result<std::collections::BTreeSet<[u8; 32]>, Box<dyn std::error::Error>> {
        let mut result = std::collections::BTreeSet::new();
        if self.play.playing.is_some() {
            return Ok(result);
        }
        let Some(base) = self.prefab_field_base()? else {
            return Ok(result);
        };
        let Some(object) = document.objects.get(self.selected) else {
            return Ok(result);
        };
        for list in collections(object, &self.authoring.authoring_project.registry)? {
            let Some(source) = base
                .components
                .get(list.schema)
                .and_then(|value| value.pointer(list.path))
                .and_then(serde_json::Value::as_array)
            else {
                continue;
            };
            if inherited_collection_order(&list, source) != list.items {
                result.insert(list.key);
            }
        }
        Ok(result)
    }

    pub(super) fn reset_prefab_collection_order(
        &mut self,
        key: [u8; 32],
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.reset_prefab_collection_topology(key, false, None)
    }

    pub(super) fn restore_prefab_collection_deleted(
        &mut self,
        key: [u8; 32],
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.reset_prefab_collection_topology(key, true, None)
    }

    pub(super) fn restore_prefab_collection_item(
        &mut self,
        key: [u8; 32],
        item: [u8; 32],
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.reset_prefab_collection_topology(key, true, Some(item))
    }

    fn reset_prefab_collection_topology(
        &mut self,
        key: [u8; 32],
        restore_deleted: bool,
        target: Option<[u8; 32]>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_some() {
            return Err("stop Play before resetting collection order".into());
        }
        let mut snapshot = self.prefab_reset_snapshot()?;
        let base = self
            .prefab_field_base()?
            .ok_or("selected object is not a prefab instance")?;
        let mut document = self.authoring_document()?;
        let object = document
            .objects
            .get(self.selected)
            .ok_or("missing selected object")?;
        let lists = collections(object, &self.authoring.authoring_project.registry)?;
        let list = lists
            .iter()
            .find(|list| list.key == key)
            .ok_or("collection target changed")?;
        let source = base
            .components
            .get(list.schema)
            .and_then(|value| value.pointer(list.path))
            .and_then(serde_json::Value::as_array)
            .ok_or("prefab has no inherited collection")?;
        if let Some(target) = target {
            let mut candidates = std::collections::BTreeSet::new();
            for item in source {
                let id = item[list.identity]
                    .as_str()
                    .ok_or("invalid inherited item identity")?;
                if item_key(id) == target
                    && !list
                        .items
                        .iter()
                        .any(|current| current[list.identity].as_str() == Some(id))
                {
                    candidates.insert(id.to_owned());
                }
            }
            for instance in &snapshot.source.instances {
                for (local, overrides) in &instance.overrides {
                    if voxy_scene::instance_object_id(&instance.id, local) == object.id {
                        if let Some(changes) = overrides
                            .component_collections
                            .get(list.schema)
                            .and_then(|changes| changes.get(list.path))
                        {
                            candidates.extend(
                                changes
                                    .removed
                                    .iter()
                                    .filter(|id| item_key(id) == target)
                                    .cloned(),
                            );
                        }
                    }
                }
            }
            if candidates.len() != 1 {
                return Err("deleted item is missing or ambiguous".into());
            }
        }
        let mut reordered = if restore_deleted {
            list.items.to_vec()
        } else {
            inherited_collection_order(list, source)
        };
        if restore_deleted {
            for (index, item) in source.iter().enumerate() {
                let id = &item[list.identity];
                if target.is_some_and(|target| id.as_str().is_none_or(|id| item_key(id) != target))
                {
                    continue;
                }
                if reordered
                    .iter()
                    .any(|current| &current[list.identity] == id)
                {
                    continue;
                }
                let position = source[..index]
                    .iter()
                    .rev()
                    .find_map(|previous| {
                        reordered
                            .iter()
                            .position(|current| current[list.identity] == previous[list.identity])
                            .map(|at| at + 1)
                    })
                    .or_else(|| {
                        source[index + 1..].iter().find_map(|next| {
                            reordered
                                .iter()
                                .position(|current| current[list.identity] == next[list.identity])
                        })
                    })
                    .unwrap_or(reordered.len());
                reordered.insert(position, item.clone());
            }
        }
        let schema = list.schema.to_owned();
        let path = list.path.to_owned();
        let owner = object.id.clone();
        *document.objects[self.selected]
            .components
            .get_mut(&schema)
            .and_then(|value| value.pointer_mut(&path))
            .ok_or("collection disappeared")? = serde_json::Value::Array(reordered);
        for instance in &mut snapshot.source.instances {
            for (local, overrides) in &mut instance.overrides {
                if voxy_scene::instance_object_id(&instance.id, local) == owner {
                    overrides.components.remove(&schema);
                    if let Some(changes) = overrides
                        .component_collections
                        .get_mut(&schema)
                        .and_then(|collections| collections.get_mut(&path))
                    {
                        if restore_deleted {
                            if let Some(target) = target {
                                changes.removed.retain(|id| item_key(id) != target);
                            } else {
                                changes.removed.clear();
                            }
                        } else {
                            changes.order = None;
                        }
                    }
                }
            }
        }
        self.commit_prefab_reset(snapshot, document)
    }

    #[cfg(test)]
    pub(super) fn prefab_collection_deleted_resets(
        &self,
        document: &voxy_scene::SceneDocument,
    ) -> Result<std::collections::BTreeSet<[u8; 32]>, Box<dyn std::error::Error>> {
        Ok(self
            .prefab_collection_deleted_items(document)?
            .into_keys()
            .collect())
    }

    pub(super) fn prefab_collection_deleted_items(
        &self,
        document: &voxy_scene::SceneDocument,
    ) -> Result<DeletedItems, Box<dyn std::error::Error>> {
        let mut result = DeletedItems::new();
        if self.play.playing.is_some() {
            return Ok(result);
        }
        let Some(base) = self.prefab_field_base()? else {
            return Ok(result);
        };
        let Some(object) = document.objects.get(self.selected) else {
            return Ok(result);
        };
        let snapshot: crate::prefab_authoring::AuthoredScene = serde_json::from_value(
            self.authoring
                .history
                .as_ref()
                .ok_or("missing history")?
                .metadata()
                .clone(),
        )?;
        for list in collections(object, &self.authoring.authoring_project.registry)? {
            let mut deleted = std::collections::BTreeMap::<String, String>::new();
            if let Some(source) = base
                .components
                .get(list.schema)
                .and_then(|value| value.pointer(list.path))
                .and_then(serde_json::Value::as_array)
            {
                for item in source {
                    if !list
                        .items
                        .iter()
                        .any(|current| current[list.identity] == item[list.identity])
                    {
                        let id = item[list.identity]
                            .as_str()
                            .ok_or("invalid inherited identity")?;
                        deleted.insert(
                            id.to_owned(),
                            item.get("name")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or(id)
                                .chars()
                                .take(40)
                                .collect(),
                        );
                    }
                }
            }
            for instance in &snapshot.source.instances {
                for (local, overrides) in &instance.overrides {
                    if voxy_scene::instance_object_id(&instance.id, local) == object.id {
                        if let Some(changes) = overrides
                            .component_collections
                            .get(list.schema)
                            .and_then(|changes| changes.get(list.path))
                        {
                            for id in &changes.removed {
                                deleted
                                    .entry(id.clone())
                                    .or_insert_with(|| id.chars().take(40).collect());
                            }
                        }
                    }
                }
            }
            if !deleted.is_empty() {
                result.insert(
                    list.key,
                    deleted
                        .into_iter()
                        .map(|(id, label)| (item_key(&id), label))
                        .collect(),
                );
            }
        }
        Ok(result)
    }

    pub(super) fn collection_page(
        &mut self,
        choice: bool,
        forward: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let crate::InspectorMode::Collections(index, page) = self.inspector else {
            return Ok(());
        };
        let document = self.authoring_document()?;
        let object = document
            .objects
            .get(self.selected)
            .ok_or("missing selected object")?;
        let lists = collections(object, &self.authoring.authoring_project.registry)?;
        if lists.is_empty() {
            return Ok(());
        }
        let index = index % lists.len();
        self.inspector = if choice {
            crate::InspectorMode::Collections((index + 1) % lists.len(), 0)
        } else {
            let pages = lists[index].items.len().div_ceil(3).max(1);
            let page = page % pages;
            crate::InspectorMode::Collections(
                index,
                if forward {
                    (page + 1) % pages
                } else {
                    (page + pages - 1) % pages
                },
            )
        };
        Ok(())
    }
    pub(super) fn collection_action(
        &mut self,
        key: [u8; 32],
        target: Option<[u8; 32]>,
        forward: Option<bool>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_some() {
            return Err("stop Play before editing collections".into());
        }
        let mut document = self.authoring_document()?;
        let object = document
            .objects
            .get(self.selected)
            .ok_or("missing selected object")?;
        let lists = collections(object, &self.authoring.authoring_project.registry)?;
        let list = lists
            .iter()
            .find(|list| list.key == key)
            .ok_or("collection target changed")?;
        let edit = if let Some(target) = target {
            let matching: Vec<_> = list
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    item.get(list.identity)
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|id| item_key(id) == target)
                })
                .collect();
            let [(index, item)] = matching.as_slice() else {
                return Err("collection item is missing or ambiguous".into());
            };
            let id = item[list.identity]
                .as_str()
                .ok_or("invalid item ID")?
                .to_owned();
            if let Some(forward) = forward {
                let anchor = if forward {
                    if *index + 1 >= list.items.len() {
                        return Ok(());
                    }
                    list.items.get(*index + 2)
                } else {
                    if *index == 0 {
                        return Ok(());
                    }
                    list.items.get(*index - 1)
                };
                CollectionEdit::Move {
                    id,
                    before: anchor
                        .map(|item| {
                            item[list.identity]
                                .as_str()
                                .ok_or("invalid anchor ID")
                                .map(str::to_owned)
                        })
                        .transpose()?,
                }
            } else {
                CollectionEdit::Remove { id }
            }
        } else {
            let mut entropy = [0_u8; 32];
            getrandom::fill(&mut entropy)
                .map_err(|error| format!("cannot allocate collection ID: {error}"))?;
            let id = format!("item-{}", blake3::Hash::from(entropy).to_hex());
            CollectionEdit::InsertDefault { id, before: None }
        };
        let owner = object.id.clone();
        let schema = list.schema.to_owned();
        let path = list.path.to_owned();
        document.edit_collection(
            &self.authoring.authoring_project.registry,
            &owner,
            &schema,
            &path,
            edit,
            128,
        )?;
        let next = self.validate_authoring_document(&document)?;
        self.authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit(document, &self.authoring.authoring_project.registry)?;
        self.restore_authoring()?;
        self.authoring.next_object_id = next;
        self.panel_cache = None;
        Ok(())
    }
}
fn inherited_collection_order(
    list: &Collection<'_>,
    source: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let source_ids: Vec<_> = source
        .iter()
        .map(|item| item[list.identity].as_str())
        .collect();
    let mut reordered = list.items.to_vec();
    reordered.sort_by_key(|item| {
        let id = item[list.identity].as_str().unwrap_or_default();
        (
            source_ids
                .iter()
                .position(|source| *source == Some(id))
                .unwrap_or(usize::MAX),
            id.to_owned(),
        )
    });
    reordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        App, InspectorMode, ModelSource,
        panels::{Action, Panels},
    };
    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    struct List {
        items: Vec<Item>,
    }
    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    struct Item {
        id: String,
        count: u32,
    }

    fn build(app: &mut App) {
        let document = app.authoring_document().unwrap();
        app.panels
            .as_mut()
            .unwrap()
            .build(
                &document,
                app.selected,
                glam::Vec2::new(1000., 700.),
                false,
                None,
                0,
                false,
                app.inspector,
                "",
                "",
                None,
            )
            .unwrap();
        app.panels
            .as_mut()
            .unwrap()
            .frame_outcome(voxy_render::RenderOutcome::Presented);
    }
    fn click(app: &mut App, action: Action) {
        build(app);
        let rect = app
            .panels
            .as_ref()
            .unwrap()
            .regions
            .iter()
            .find(|(_, target)| *target == action)
            .unwrap()
            .0;
        let target = app
            .panel_target(glam::Vec2::new(
                rect[0] + rect[2] / 2.,
                rect[1] + rect[3] / 2.,
            ))
            .unwrap()
            .unwrap();
        assert_eq!(target, action);
        app.panel_action(target).unwrap();
    }
    fn ids(app: &App) -> Vec<String> {
        app.authoring_document().unwrap().objects[0].components["custom.list"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["id"].as_str().unwrap().into())
            .collect()
    }
    #[test]
    fn collection_buttons_edit_empty_lists_follow_ids_and_round_trip_history() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut registry = crate::editor_component_registry().unwrap();
        registry.register::<List>("custom.list").unwrap();
        registry
            .declare_identified_collection("custom.list", "/items", "id")
            .unwrap();
        registry
            .set_collection_default("custom.list", "/items", serde_json::json!({"count": 1}))
            .unwrap();
        let mut app =
            crate::configured_app_with_registry(&ModelSource::File(fixture), None, false, registry)
                .unwrap();
        app.scene
            .insert_component(app.instances[0], List { items: vec![] })
            .unwrap();
        app.commit_authoring().unwrap();
        app.inspector = InspectorMode::Collections(0, 0);
        app.panels =
            Some(Panels::with_registry(app.authoring.authoring_project.registry.clone()).unwrap());
        let original = app.authoring_document().unwrap();
        let key = collections(
            &original.objects[0],
            &app.authoring.authoring_project.registry,
        )
        .unwrap()[0]
            .key;
        click(&mut app, Action::CollectionAdd(key));
        let first = ids(&app)[0].clone();
        let delete_first = Action::CollectionDelete(key, item_key(&first));
        click(&mut app, Action::CollectionAdd(key));
        let second = ids(&app)[1].clone();
        assert_ne!(first, second);
        click(
            &mut app,
            Action::CollectionMove(key, item_key(&first), true),
        );
        assert_eq!(ids(&app), [second.clone(), first.clone()]);
        // The captured delete action still addresses the original item after moving it.
        click(&mut app, delete_first);
        assert_eq!(ids(&app), [second.clone()]);
        let deleted = app.authoring_document().unwrap();
        assert!(app.panel_action(delete_first).is_err());
        assert_eq!(app.authoring_document().unwrap(), deleted);
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(ids(&app), [second.clone(), first.clone()]);
        click(
            &mut app,
            Action::CollectionMove(key, item_key(&first), false),
        );
        assert_eq!(ids(&app), [first.clone(), second.clone()]);
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(ids(&app), [first.clone(), second.clone()]);
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), original);
        app.edit_key(winit::keyboard::KeyCode::KeyY).unwrap();
        assert_eq!(ids(&app), [first.clone()]);
        let directory =
            std::env::temp_dir().join(format!("voxy-collection-controls-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        app.authoring.scene_path = Some(directory.join("scene.json"));
        app.save_authoring().unwrap();
        app.load_authoring().unwrap();
        assert_eq!(ids(&app), [first.clone()]);
        click(&mut app, delete_first);
        click(&mut app, Action::CollectionAdd(key));
        assert_ne!(ids(&app)[0], first);
        app.panel_action(Action::Play).unwrap();
        let playing = app.authoring_document().unwrap();
        app.panel_action(Action::CollectionAdd(key)).unwrap();
        assert_eq!(app.authoring_document().unwrap(), playing);
        app.panel_action(Action::Play).unwrap();
        for _ in 0..3 {
            click(&mut app, Action::CollectionAdd(key));
        }
        let four = ids(&app);
        click(&mut app, Action::CollectionPage(true));
        assert_eq!(app.inspector, InspectorMode::Collections(0, 1));
        build(&mut app);
        assert!(
            !app.panels
                .as_ref()
                .unwrap()
                .regions
                .iter()
                .any(|(_, action)| *action == Action::CollectionDelete(key, item_key(&four[0])))
        );
        assert!(
            app.panels
                .as_ref()
                .unwrap()
                .regions
                .iter()
                .any(|(_, action)| *action == Action::CollectionDelete(key, item_key(&four[3])))
        );
        click(
            &mut app,
            Action::CollectionMove(key, item_key(&four[3]), false),
        );
        assert_eq!(
            ids(&app),
            [
                four[0].clone(),
                four[1].clone(),
                four[3].clone(),
                four[2].clone()
            ]
        );
        click(&mut app, Action::CollectionPage(false));
        assert_eq!(app.inspector, InspectorMode::Collections(0, 0));
        app.panel_action(Action::Duplicate).unwrap();
        assert!(app.panel_action(Action::CollectionAdd(key)).is_err());
        app.stop_workers().unwrap();
        std::fs::remove_dir_all(directory).unwrap();
    }
}

impl crate::App {
    pub(super) fn prefab_collection_resets(
        &self,
        document: &voxy_scene::SceneDocument,
    ) -> Result<std::collections::BTreeSet<([u8; 32], [u8; 32])>, Box<dyn std::error::Error>> {
        let mut resets = std::collections::BTreeSet::new();
        if self.play.playing.is_some() {
            return Ok(resets);
        }
        let Some(base) = self.prefab_field_base()? else {
            return Ok(resets);
        };
        let Some(object) = document.objects.get(self.selected) else {
            return Ok(resets);
        };
        for list in collections(object, &self.authoring.authoring_project.registry)? {
            let Some(inherited) = base
                .components
                .get(list.schema)
                .and_then(|value| value.pointer(list.path))
                .and_then(serde_json::Value::as_array)
            else {
                continue;
            };
            for item in list.items {
                let id = item[list.identity]
                    .as_str()
                    .ok_or("invalid collection identity")?;
                let source = inherited
                    .iter()
                    .find(|source| source[list.identity].as_str() == Some(id));
                if source != Some(item) {
                    resets.insert((list.key, item_key(id)));
                }
            }
        }
        Ok(resets)
    }
    pub(super) fn reset_prefab_collection_item(
        &mut self,
        key: [u8; 32],
        target: [u8; 32],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_some() {
            return Err("stop Play before resetting a collection item".into());
        }
        let mut snapshot = self.prefab_reset_snapshot()?;
        let base = self
            .prefab_field_base()?
            .ok_or("selected object is not a prefab instance")?;
        let mut document = self.authoring_document()?;
        let object = document
            .objects
            .get(self.selected)
            .ok_or("missing selected object")?;
        let lists = collections(object, &self.authoring.authoring_project.registry)?;
        let list = lists
            .iter()
            .find(|list| list.key == key)
            .ok_or("collection target changed")?;
        let matching: Vec<_> = list
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.get(list.identity)
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|id| item_key(id) == target)
            })
            .collect();
        let [(index, item)] = matching.as_slice() else {
            return Err("collection item is missing or ambiguous".into());
        };
        let id = item[list.identity]
            .as_str()
            .ok_or("invalid item identity")?;
        let source = base
            .components
            .get(list.schema)
            .and_then(|value| value.pointer(list.path))
            .and_then(serde_json::Value::as_array)
            .ok_or("prefab has no inherited collection")?;
        let mut inherited = source
            .iter()
            .filter(|item| item[list.identity].as_str() == Some(id));
        let replacement = inherited.next().cloned();
        if inherited.next().is_some() {
            return Err("ambiguous inherited collection identity".into());
        }
        let schema = list.schema.to_owned();
        let path = list.path.to_owned();
        let index = *index;
        let owner = object.id.clone();
        let items = document.objects[self.selected]
            .components
            .get_mut(&schema)
            .and_then(|value| value.pointer_mut(&path))
            .and_then(serde_json::Value::as_array_mut)
            .ok_or("collection disappeared during Reset")?;
        if let Some(replacement) = replacement {
            items[index] = replacement;
        } else {
            items.remove(index);
        }
        // An explicit reset may split a legacy whole-component override.
        for instance in &mut snapshot.source.instances {
            for (local, overrides) in &mut instance.overrides {
                if voxy_scene::instance_object_id(&instance.id, local) == owner {
                    overrides.components.remove(&schema);
                }
            }
        }
        self.commit_prefab_reset(snapshot, document)
    }
}
