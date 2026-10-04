//! Data-only inspector fields. Component codecs remain the validation authority.
use serde_json::Value;
use voxy_scene::SceneObject;

#[derive(Clone, Debug)]
pub(crate) struct ComponentField {
    pub schema: String,
    pub path: String,
    pub value: Value,
}

#[derive(Clone, Debug)]
pub(crate) struct BoundComponentField {
    owner: voxy_scene::ObjectId,
    field: ComponentField,
    item: Option<(String, String, String, String)>,
}
impl ComponentField {
    pub fn bind(
        &self,
        object: &SceneObject,
        registry: &voxy_scene::ComponentRegistry,
    ) -> Result<BoundComponentField, Box<dyn std::error::Error>> {
        let mut item = None;
        for (path, key) in registry.identified_collections(&self.schema)? {
            if let Some(suffix) = self.path.strip_prefix(&format!("{path}/")) {
                let (index, member) = suffix.split_once('/').ok_or("missing collection member")?;
                let value = object.components[&self.schema]
                    .pointer(path)
                    .and_then(Value::as_array)
                    .and_then(|values| values.get(index.parse::<usize>().ok()?))
                    .ok_or("missing collection item")?;
                let id = value
                    .get(key)
                    .and_then(Value::as_str)
                    .ok_or("missing collection identity")?;
                item = Some((path.clone(), id.into(), format!("/{member}"), key.clone()));
            }
        }
        Ok(BoundComponentField {
            owner: object.id.clone(),
            field: self.clone(),
            item,
        })
    }
}
impl BoundComponentField {
    pub fn identity(&self) -> String {
        serde_json::to_string(&(
            &self.owner.0,
            &self.field.schema,
            &self.item,
            self.item.is_none().then_some(&self.field.path),
        ))
        .expect("field identity strings serialize")
    }
    fn replace(
        &self,
        document: &mut voxy_scene::SceneDocument,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let object = document
            .objects
            .iter_mut()
            .find(|object| object.id == self.owner)
            .ok_or("component edit owner disappeared")?;
        let mut field = self.field.clone();
        if let Some((path, id, member, key)) = &self.item {
            if member == &format!("/{}", key.replace('~', "~0").replace('/', "~1")) {
                return Err("collection identity is immutable".into());
            }
            let values = object
                .components
                .get(&field.schema)
                .and_then(|value| value.pointer(path))
                .and_then(Value::as_array)
                .ok_or("collection disappeared")?;
            let indices: Vec<_> = values
                .iter()
                .enumerate()
                .filter(|(_, value)| value.get(key).and_then(Value::as_str) == Some(id))
                .map(|(index, _)| index)
                .collect();
            let [index] = indices.as_slice() else {
                return Err("collection item identity is missing or ambiguous".into());
            };
            field.path = format!("{path}/{index}{member}");
        }
        field.replace(object, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{App, InspectorMode, panels};
    use winit::keyboard::KeyCode;

    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    struct CustomList {
        items: Vec<CustomItem>,
    }
    #[derive(Clone, serde::Serialize, serde::Deserialize)]
    struct CustomItem {
        id: String,
        count: u32,
    }

    #[test]
    fn installed_custom_codecs_survive_editor_history_scene_io_play_and_workers() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut registry = crate::editor_component_registry().unwrap();
        registry.register::<CustomList>("custom.list").unwrap();
        registry
            .declare_identified_collection("custom.list", "/items", "id")
            .unwrap();
        registry
            .set_collection_default("custom.list", "/items", serde_json::json!({"count": 1}))
            .unwrap();
        let mut app = crate::configured_app_with_registry(
            &crate::ModelSource::File(fixture),
            None,
            false,
            registry,
        )
        .unwrap();
        let worker = app.authoring.authoring_project.worker_project().unwrap();
        assert!(std::sync::Arc::ptr_eq(
            &worker.registry,
            &app.authoring.authoring_project.registry
        ));
        assert_eq!(
            worker
                .registry
                .new_collection_item("custom.list", "/items", "fresh")
                .unwrap(),
            serde_json::json!({"id":"fresh", "count":1})
        );
        app.scene
            .insert_component(
                app.instances[0],
                CustomList {
                    items: vec![CustomItem {
                        id: "stable".into(),
                        count: 1,
                    }],
                },
            )
            .unwrap();
        app.commit_authoring().unwrap();
        let document = app.authoring_document().unwrap();
        let index = fields(&document.objects[0])
            .unwrap()
            .iter()
            .position(|field| field.schema == "custom.list" && field.path == "/items/0/count")
            .unwrap();
        app.inspector = InspectorMode::Components(index / 6);
        app.panel_action(panels::Action::Field(index)).unwrap();
        app.field_key(KeyCode::Digit7, Some("7")).unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
        assert_eq!(
            app.authoring_document().unwrap().objects[0].components["custom.list"]["items"][0]["count"],
            7
        );
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), document);
        app.edit_key(KeyCode::KeyY).unwrap();
        let edited = app.authoring_document().unwrap();
        let prepared = worker
            .prepare(
                voxy_scene::PrefabSceneDocument {
                    version: 1,
                    objects: edited.objects.clone(),
                    instances: vec![],
                },
                None,
            )
            .unwrap();
        assert_eq!(prepared.value().expanded, edited);
        let directory =
            std::env::temp_dir().join(format!("voxy-custom-registry-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        app.authoring.scene_path = Some(directory.join("scene.json"));
        app.save_authoring().unwrap();
        app.load_authoring().unwrap();
        assert_eq!(app.authoring_document().unwrap(), edited);
        app.panel_action(panels::Action::Play).unwrap();
        assert_eq!(app.authoring_document().unwrap(), edited);
        app.panel_action(panels::Action::Play).unwrap();
        assert_eq!(app.authoring_document().unwrap(), edited);
        app.stop_workers().unwrap();
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn bound_collection_field_follows_item_id_and_rejects_stale_edits() {
        let mut registry = voxy_scene::ComponentRegistry::default();
        registry.register::<Value>("custom").unwrap();
        registry
            .declare_identified_collection("custom", "/items", "id")
            .unwrap();
        let mut document = voxy_scene::SceneDocument::from_json(
            &serde_json::json!({"version":1,"objects":[{
                "id":"root","parent":null,"name":"root","active":true,
                "translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],
                "components":{"custom":{"items":[{"id":"a","gain":1},{"id":"b","gain":2}]}}
            }]})
            .to_string(),
        )
        .unwrap();
        let members = fields(&document.objects[0]).unwrap();
        let binding = members
            .iter()
            .find(|field| field.path == "/items/1/gain")
            .unwrap()
            .bind(&document.objects[0], &registry)
            .unwrap();
        document.objects[0].components.get_mut("custom").unwrap()["items"]
            .as_array_mut()
            .unwrap()
            .insert(0, serde_json::json!({"id":"new","gain":9}));
        let rebound = fields(&document.objects[0])
            .unwrap()
            .into_iter()
            .find(|field| field.path == "/items/2/gain")
            .unwrap()
            .bind(&document.objects[0], &registry)
            .unwrap();
        assert_eq!(binding.identity(), rebound.identity());
        binding.replace(&mut document, "3").unwrap();
        assert_eq!(
            document.objects[0].components["custom"]["items"][2]["gain"],
            3
        );
        assert_eq!(
            document.objects[0].components["custom"]["items"][1]["gain"],
            1
        );
        let retained = document.clone();
        assert!(binding.replace(&mut document, "4").is_err());
        assert_eq!(document, retained);
        let id = fields(&document.objects[0])
            .unwrap()
            .into_iter()
            .find(|field| field.path == "/items/2/id")
            .unwrap()
            .bind(&document.objects[0], &registry)
            .unwrap();
        assert!(id.replace(&mut document, "changed").is_err());
        assert_eq!(document, retained);
        document.objects[0].components.get_mut("custom").unwrap()["items"]
            .as_array_mut()
            .unwrap()
            .pop();
        let without_item = document.clone();
        assert!(binding.replace(&mut document, "4").is_err());
        assert_eq!(document, without_item);
    }

    #[test]
    fn editor_typing_survives_component_reindexing_but_rejects_changed_values() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        let owner = app.instances[0];
        app.scene
            .insert_component(
                owner,
                voxy_gameplay::AngularMotion {
                    axis: [0., 0., 1.],
                    radians_per_second: 1.,
                },
            )
            .unwrap();
        app.commit_authoring().unwrap();
        app.inspector = InspectorMode::Components(0);
        let before = app.authoring_document().unwrap();
        let index = fields(&before.objects[0])
            .unwrap()
            .iter()
            .position(|field| field.path == "/radians_per_second")
            .unwrap();
        app.panel_action(panels::Action::Field(index)).unwrap();
        app.field_key(KeyCode::Digit3, Some("3")).unwrap();
        app.scene
            .insert_component(owner, crate::SceneMaterial::default())
            .unwrap();
        app.commit_authoring().unwrap();
        let with_material = app.authoring_document().unwrap();
        app.reconcile_component_edit(&with_material).unwrap();
        assert_ne!(app.field.as_ref().unwrap().0, index);
        let rebound_index = app.field.as_ref().unwrap().0;
        assert_eq!(app.inspector, InspectorMode::Components(rebound_index / 6));
        app.field_key(KeyCode::Enter, None).unwrap();
        let edited = app.authoring_document().unwrap();
        assert_eq!(
            edited.objects[0].components["game.angular-motion.v1"]["radians_per_second"],
            3.0
        );
        assert_eq!(
            edited.objects[0].components["editor.material.v1"],
            with_material.objects[0].components["editor.material.v1"]
        );
        app.panel_action(panels::Action::Field(rebound_index))
            .unwrap();
        app.field_key(KeyCode::Digit4, Some("4")).unwrap();
        app.scene
            .component_mut::<voxy_gameplay::AngularMotion>(app.instances[0])
            .unwrap()
            .unwrap()
            .radians_per_second = 5.;
        app.commit_authoring().unwrap();
        let changed = app.authoring_document().unwrap();
        assert!(app.field_key(KeyCode::Enter, None).is_err());
        assert_eq!(app.authoring_document().unwrap(), changed);
        app.field_key(KeyCode::Escape, None).unwrap();
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), edited);
        app.stop_workers().unwrap();
    }

    #[test]
    fn generic_inspector_commits_validates_and_undoes_registered_fields() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.scene
            .insert_component(
                app.instances[0],
                voxy_gameplay::AngularMotion {
                    axis: [0., 0., 1.],
                    radians_per_second: 1.,
                },
            )
            .unwrap();
        app.commit_authoring().unwrap();
        app.inspector = InspectorMode::Components(0);
        let before = app.authoring_document().unwrap();
        let members = fields(&before.objects[0]).unwrap();
        let speed = members
            .iter()
            .position(|field| field.path == "/radians_per_second")
            .unwrap();
        app.panel_action(panels::Action::Field(speed)).unwrap();
        app.field_key(KeyCode::Digit2, Some("2.5")).unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
        let edited = app.authoring_document().unwrap();
        assert_eq!(
            edited.objects[0].components["game.angular-motion.v1"]["radians_per_second"],
            2.5
        );
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), before);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), edited);
        let axis = members
            .iter()
            .position(|field| field.path == "/axis/2")
            .unwrap();
        assert!(app.edit_component_field(axis, "0").is_err());
        assert!(app.edit_component_field(speed, "true").is_err());
        assert_eq!(app.authoring_document().unwrap(), edited);
        app.panel_action(panels::Action::ComponentPage(true))
            .unwrap();
        assert!(matches!(app.inspector, InspectorMode::Components(_)));
        app.panel_action(panels::Action::Play).unwrap();
        assert!(app.edit_component_field(speed, "3").is_err());
        app.panel_action(panels::Action::Play).unwrap();
        app.stop_workers().unwrap();
    }

    #[test]
    fn animation_inspector_rejects_invalid_values_and_round_trips_bind_pose_history() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/animated-triangle.glb");
        let mut app = App::new(&fixture, false).unwrap();
        // Authored settings must load before the model import completes.
        let mut pending = app.authoring_document().unwrap();
        pending.objects[0].components.insert(
            "editor.model-animation.v1".into(),
            serde_json::json!({"clip": 0, "speed": 1.0}),
        );
        app.validate_authoring_document(&pending).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.catalog.snapshot(&app.id).is_none() {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        app.panel_action(panels::Action::Animation).unwrap();
        let before = app.authoring_document().unwrap();
        let members = fields(&before.objects[0]).unwrap();
        let clip = members
            .iter()
            .position(|field| field.schema == "editor.model-animation.v1" && field.path == "/clip")
            .unwrap();
        let speed = members
            .iter()
            .position(|field| field.schema == "editor.model-animation.v1" && field.path == "/speed")
            .unwrap();
        assert!(app.edit_component_field(clip, "9999").is_err());
        assert!(app.edit_component_field(speed, "-1").is_err());
        assert!(app.edit_component_field(speed, "9").is_err());
        let transition = members
            .iter()
            .position(|field| {
                field.schema == "editor.model-animation.v1" && field.path == "/transition_seconds"
            })
            .unwrap();
        assert!(app.edit_component_field(transition, "-1").is_err());
        assert!(app.edit_component_field(transition, "61").is_err());
        assert_eq!(app.authoring_document().unwrap(), before);

        let motion = members
            .iter()
            .position(|field| {
                field.schema == "editor.model-animation.v1" && field.path == "/root_motion_joint"
            })
            .unwrap();
        assert!(app.edit_component_field(motion, "-1").is_err());
        assert!(app.edit_component_field(motion, "9999").is_err());
        assert!(app.edit_component_field(motion, "255").is_err());
        assert_eq!(app.authoring_document().unwrap(), before);
        app.edit_component_field(motion, "1").unwrap();
        let selected = app.authoring_document().unwrap();
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), before);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), selected);
        let named = members
            .iter()
            .position(|field| {
                field.schema == "editor.model-animation.v1" && field.path == "/root_motion_bone"
            })
            .unwrap();
        assert!(app.edit_component_field(named, "missing bone").is_err());
        assert_eq!(app.authoring_document().unwrap(), selected);
        let name = app
            .catalog
            .snapshot(&app.id)
            .unwrap()
            .value()
            .animated
            .as_ref()
            .unwrap()
            .joint_names()
            .iter()
            .flatten()
            .next()
            .unwrap()
            .to_string();
        app.edit_component_field(named, &name).unwrap();
        let named_document = app.authoring_document().unwrap();
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), selected);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), named_document);
        app.edit_component_field(clip, "null").unwrap();
        app.edit_component_field(speed, "0").unwrap();
        let paused = app.authoring_document().unwrap();
        app.edit_component_field(clip, "0").unwrap();
        let resumed = app.authoring_document().unwrap();
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), paused);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), resumed);
        let encoded = serde_json::to_string(&resumed).unwrap();
        let decoded = voxy_scene::SceneDocument::from_json(&encoded).unwrap();
        app.validate_authoring_document(&decoded).unwrap();
        let clip_name = members
            .iter()
            .position(|field| {
                field.schema == "editor.model-animation.v1" && field.path == "/clip_name"
            })
            .unwrap();
        assert!(app.edit_component_field(clip_name, "missing clip").is_err());
        assert_eq!(app.authoring_document().unwrap(), resumed);
        let selected_name = app
            .catalog
            .snapshot(&app.id)
            .unwrap()
            .value()
            .animated
            .as_ref()
            .unwrap()
            .animations[0]
            .name()
            .to_owned();
        app.edit_component_field(clip_name, &selected_name).unwrap();
        let named_clip = app.authoring_document().unwrap();
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), resumed);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), named_clip);
        let decoded_name =
            voxy_scene::SceneDocument::from_json(&serde_json::to_string(&named_clip).unwrap())
                .unwrap();
        app.validate_authoring_document(&decoded_name).unwrap();
        assert_eq!(decoded_name, named_clip);

        assert_eq!(decoded, resumed);
        app.stop_workers().unwrap();
    }

    #[test]
    fn identified_member_reset_uses_escaped_ids_and_keeps_inner_arrays_atomic() {
        let mut registry = voxy_scene::ComponentRegistry::default();
        registry.register::<Value>("custom").unwrap();
        registry
            .declare_identified_collection("custom", "/items", "user/id")
            .unwrap();
        let mut object = voxy_scene::SceneDocument::from_json(r#"{"version":1,"objects":[{"id":"root","parent":null,"name":"root","active":true,"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{"custom":{"items":[{"user/id":"a","vector":[9,10],"a/b":{"~gain":6}},{"user/id":"b","vector":[8,7],"a/b":{"~gain":5}}]}}}]}"#).unwrap().objects.remove(0);
        let mut base = object.clone();
        base.components.get_mut("custom").unwrap()["items"] = serde_json::json!([
            {"user/id":"b","vector":[1,2],"a/b":{"~gain":3}},
            {"user/id":"a","vector":[3,4],"a/b":{"~gain":2}}]);
        let members = fields(&object).unwrap();
        let vector = members
            .iter()
            .find(|field| field.path == "/items/0/vector/1")
            .unwrap();
        let (path, value) = vector
            .prefab_reset_value(&object, &base, &registry)
            .unwrap()
            .unwrap();
        assert_eq!(path, "/items/0/vector");
        assert_eq!(value, serde_json::json!([3, 4]));
        *object
            .components
            .get_mut("custom")
            .unwrap()
            .pointer_mut(&path)
            .unwrap() = value;
        assert_eq!(
            object.components["custom"]["items"][1]["vector"],
            serde_json::json!([8, 7])
        );
        let scalar = members
            .iter()
            .find(|field| field.path == "/items/0/a~1b/~0gain")
            .unwrap();
        assert_eq!(
            scalar
                .prefab_reset_value(&object, &base, &registry)
                .unwrap()
                .unwrap(),
            ("/items/0/a~1b/~0gain".into(), serde_json::json!(2))
        );
        let identity = members
            .iter()
            .find(|field| field.path == "/items/0/user~1id")
            .unwrap();
        assert!(
            identity
                .prefab_reset_value(&object, &base, &registry)
                .unwrap()
                .is_none()
        );
        base.components.get_mut("custom").unwrap()["items"][0]["user/id"] = serde_json::json!("a");
        assert!(
            scalar
                .prefab_reset_value(&object, &base, &registry)
                .is_err()
        );
    }

    #[test]
    fn escaped_paths_scalar_types_and_atomic_array_reset() {
        let mut object = voxy_scene::SceneDocument::from_json(r#"{"version":1,"objects":[{"id":"root","parent":null,"name":"root","active":true,"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{"custom":{"a/b":{"~flag":true},"text":"old","items":[1,2],"empty":null}}}]}"#).unwrap().objects.remove(0);
        let members = fields(&object).unwrap();
        let flag = members
            .iter()
            .find(|field| field.path == "/a~1b/~0flag")
            .unwrap();
        flag.replace(&mut object, "false").unwrap();
        assert_eq!(object.components["custom"]["a/b"]["~flag"], false);
        let text = members.iter().find(|field| field.path == "/text").unwrap();
        text.replace(&mut object, "Привет \"world\"").unwrap();
        assert_eq!(object.components["custom"]["text"], "Привет \"world\"");
        assert!(flag.replace(&mut object, "true").is_err());
        assert_eq!(
            members
                .iter()
                .find(|field| field.path == "/items/1")
                .unwrap()
                .reset_path(&object),
            "/items"
        );
        let null = members.iter().find(|field| field.path == "/empty").unwrap();
        assert!(null.replace(&mut object, "false").is_err());
        null.replace(&mut object, "null").unwrap();
    }
}
pub(crate) fn fields(object: &SceneObject) -> Result<Vec<ComponentField>, &'static str> {
    fn visit(
        schema: &str,
        path: String,
        value: &Value,
        depth: usize,
        out: &mut Vec<ComponentField>,
    ) -> Result<(), &'static str> {
        if depth > 32 || out.len() >= 4096 {
            return Err("component inspector capacity exceeded");
        }
        match value {
            Value::Object(members) => {
                for (key, value) in members {
                    visit(
                        schema,
                        format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")),
                        value,
                        depth + 1,
                        out,
                    )?;
                }
            }
            Value::Array(items) => {
                for (index, value) in items.iter().enumerate() {
                    visit(schema, format!("{path}/{index}"), value, depth + 1, out)?;
                }
            }
            _ => out.push(ComponentField {
                schema: schema.into(),
                path,
                value: value.clone(),
            }),
        }
        Ok(())
    }
    let mut out = Vec::new();
    for (schema, value) in &object.components {
        visit(schema, String::new(), value, 0, &mut out)?;
    }
    Ok(out)
}
impl ComponentField {
    pub fn reset_path(&self, object: &SceneObject) -> String {
        let mut prefix = String::new();
        for part in self.path.split('/').skip(1) {
            if object.components[&self.schema]
                .pointer(&prefix)
                .is_some_and(Value::is_array)
            {
                return prefix;
            }
            prefix.push('/');
            prefix.push_str(part);
        }
        prefix
    }
    pub(super) fn prefab_reset_value(
        &self,
        object: &SceneObject,
        base: &SceneObject,
        registry: &voxy_scene::ComponentRegistry,
    ) -> Result<Option<(String, Value)>, Box<dyn std::error::Error>> {
        let binding = self.bind(object, registry)?;
        if let Some((collection, id, member, identity)) = binding.item {
            let identity_path = format!("/{}", identity.replace('~', "~0").replace('/', "~1"));
            if member == identity_path {
                return Ok(None);
            }
            fn resolve<'a>(
                owner: &'a SceneObject,
                schema: &str,
                collection: &str,
                identity: &str,
                id: &str,
            ) -> Result<Option<(usize, &'a Value)>, Box<dyn std::error::Error>> {
                let Some(items) = owner
                    .components
                    .get(schema)
                    .and_then(|value| value.pointer(collection))
                    .and_then(Value::as_array)
                else {
                    return Ok(None);
                };
                let mut matching = items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| item.get(identity).and_then(Value::as_str) == Some(id));
                let found = matching.next();
                if matching.next().is_some() {
                    return Err("ambiguous collection identity during Reset".into());
                }
                Ok(found)
            }
            let Some((index, item)) = resolve(object, &self.schema, &collection, &identity, &id)?
            else {
                return Err("collection item disappeared during Reset".into());
            };
            let Some((_, inherited)) = resolve(base, &self.schema, &collection, &identity, &id)?
            else {
                return Ok(None);
            };
            // Arrays inside an identified item still follow ordinary atomic-vector semantics.
            let mut prefix = String::new();
            for part in member.split('/').skip(1) {
                if item.pointer(&prefix).is_some_and(Value::is_array) {
                    break;
                }
                prefix.push('/');
                prefix.push_str(part);
            }
            let Some(value) = inherited.pointer(&prefix) else {
                return Ok(None);
            };
            let path = format!("{collection}/{index}{prefix}");
            return Ok(Some((path, value.clone())));
        }
        let path = self.reset_path(object);
        Ok(base
            .components
            .get(&self.schema)
            .and_then(|value| value.pointer(&path))
            .map(|value| (path, value.clone())))
    }
    pub fn display(&self) -> String {
        self.value
            .as_str()
            .map_or_else(|| self.value.to_string(), str::to_owned)
    }
    pub fn replace(
        &self,
        object: &mut SceneObject,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let replacement = match self.value {
            Value::String(_) => Value::String(text.into()),
            _ => serde_json::from_str(text)?,
        };
        // This registered field is an optional clip index. Other component
        // fields retain their existing strict type boundary.
        let optional_clip = self.schema == "editor.model-animation.v1"
            && self.path == "/clip"
            && (replacement.is_null() || replacement.as_u64().is_some())
            && (self.value.is_null() || self.value.as_u64().is_some());
        if !optional_clip
            && std::mem::discriminant(&replacement) != std::mem::discriminant(&self.value)
        {
            return Err("component field type cannot change".into());
        }
        let current = object
            .components
            .get_mut(&self.schema)
            .and_then(|value| value.pointer_mut(&self.path))
            .ok_or("component field disappeared")?;
        if current != &self.value {
            return Err("component field changed during editing".into());
        }
        *current = replacement;
        Ok(())
    }
}

impl crate::App {
    pub(super) fn reconcile_component_edit(
        &mut self,
        document: &voxy_scene::SceneDocument,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(binding) = &self.component_edit else {
            return Ok(());
        };
        if self.field.is_none() || !matches!(self.inspector, crate::InspectorMode::Components(_)) {
            self.component_edit = None;
            return Ok(());
        }
        let registry = &self.authoring.authoring_project.registry;
        let index = if let Some(object) = document
            .objects
            .get(self.selected)
            .filter(|object| object.id == binding.owner)
        {
            fields(object)?
                .into_iter()
                .enumerate()
                .find_map(|(index, field)| {
                    field
                        .bind(object, &registry)
                        .ok()
                        .filter(|current| current.identity() == binding.identity())
                        .map(|_| index)
                })
        } else {
            None
        };
        if let Some(index) = index {
            self.field.as_mut().ok_or("missing field")?.0 = index;
            self.inspector = crate::InspectorMode::Components(index / 6);
        } else {
            self.field = None;
            self.component_edit = None;
        }
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn edit_component_field(
        &mut self,
        index: usize,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_some() {
            return Err("stop play before editing components".into());
        }
        let document = self.panel_document()?;
        let object = document
            .objects
            .get(self.selected)
            .ok_or("missing selected object")?;
        let field = fields(object)?
            .into_iter()
            .nth(index)
            .ok_or("missing component field")?;
        let binding = field.bind(object, &self.authoring.authoring_project.registry)?;
        self.commit_component_binding(&binding, text)
    }
    pub(super) fn commit_component_binding(
        &mut self,
        binding: &BoundComponentField,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_some() {
            return Err("stop play before editing components".into());
        }
        if let Some(draft) = &mut self.retarget_draft {
            binding.replace(&mut draft.document, text)?;
            self.field = None;
            self.component_edit = None;
            self.panel_cache = None;
            return Ok(());
        }
        let mut document = self.authoring_document()?;
        binding.replace(&mut document, text)?;
        let next = self.validate_authoring_document(&document)?;
        self.authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit(document, &self.authoring.authoring_project.registry)?;
        self.restore_authoring()?;
        self.authoring.next_object_id = next;
        self.field = None;
        self.component_edit = None;
        self.panel_cache = None;
        Ok(())
    }
}
