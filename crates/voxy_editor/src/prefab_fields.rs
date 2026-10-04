//! Prefab field reset resolves declared collection items by ID; ordinary vectors remain atomic.
use crate::InspectorMode;
use serde_json::Value;
use voxy_scene::SceneObject;

#[derive(Clone, Copy)]
enum Target {
    Translation,
    Rotation,
    Scale,
    Member(&'static str, &'static str),
}

fn target(mode: InspectorMode, index: usize, object: &SceneObject) -> Option<Target> {
    use Target::{Member, Rotation, Scale, Translation};
    let member = match mode {
        InspectorMode::Transform => {
            return match index {
                0..=2 => Some(Translation),
                3..=5 => Some(Rotation),
                6..=8 => Some(Scale),
                _ => None,
            };
        }
        InspectorMode::Behavior => (
            "game.angular-motion.v1",
            match index {
                0..=2 => "axis",
                3 => "radians_per_second",
                _ => return None,
            },
        ),
        InspectorMode::Material => (
            "editor.material.v1",
            match index {
                0..=3 => "tint",
                4 => "lit",
                _ => return None,
            },
        ),
        InspectorMode::Physics => {
            if object.components.contains_key("game.character.v1") {
                (
                    "game.character.v1",
                    match index {
                        0..=2 => "half_extents",
                        3 => "speed",
                        4 => "gravity",
                        5 => "jump_speed",
                        _ => return None,
                    },
                )
            } else {
                (
                    "game.box.v1",
                    if index <= 2 {
                        "half_extents"
                    } else {
                        return None;
                    },
                )
            }
        }
        InspectorMode::Audio => (
            "game.audio-source.v1",
            *["gain", "bus", "looping", "spatial", "near", "far"].get(index)?,
        ),
        InspectorMode::Mixer => ("game.audio-bus.v1", *["bus", "gain"].get(index)?),
        InspectorMode::ImportSettings
        | InspectorMode::Components(_)
        | InspectorMode::Collections(_, _) => return None,
    };
    Some(Member(member.0, member.1))
}

fn value(target: Target, object: &SceneObject) -> Option<Value> {
    match target {
        Target::Translation => Some(serde_json::json!(object.translation)),
        Target::Rotation => Some(serde_json::json!(object.rotation)),
        Target::Scale => Some(serde_json::json!(object.scale)),
        Target::Member(schema, member) => object.components.get(schema)?.get(member).cloned(),
    }
}

fn built_in_overridden(
    mode: InspectorMode,
    index: usize,
    object: &SceneObject,
    base: &SceneObject,
) -> bool {
    let Some(target) = target(mode, index, object) else {
        return false;
    };
    value(target, base)
        .is_some_and(|base| value(target, object).is_some_and(|current| current != base))
}

fn built_in_reset(
    mode: InspectorMode,
    index: usize,
    object: &mut SceneObject,
    base: &SceneObject,
) -> Result<(), &'static str> {
    let target = target(mode, index, object).ok_or("field has no prefab reset target")?;
    match target {
        Target::Translation => object.translation = base.translation,
        Target::Rotation => object.rotation = base.rotation,
        Target::Scale => object.scale = base.scale,
        Target::Member(schema, member) => {
            let replacement = value(target, base).ok_or("prefab has no inherited field")?;
            let field = object
                .components
                .get_mut(schema)
                .and_then(|value| value.get_mut(member))
                .ok_or("component field is unavailable")?;
            *field = replacement;
        }
    }
    Ok(())
}

impl crate::App {
    pub(super) fn prefab_field_base(
        &self,
    ) -> Result<Option<SceneObject>, Box<dyn std::error::Error>> {
        let metadata = self
            .authoring
            .history
            .as_ref()
            .ok_or("missing history")?
            .metadata();
        if metadata.is_null() {
            return Ok(None);
        }
        let snapshot: crate::prefab_authoring::AuthoredScene =
            serde_json::from_value(metadata.clone())?;
        let selected = self
            .object_ids
            .get(self.selected)
            .ok_or("missing selected object")?;
        if snapshot
            .source
            .objects
            .iter()
            .any(|object| &object.id == selected)
        {
            return Ok(None);
        }
        let baseline = snapshot.source.instance_baseline(
            &self.authoring.authoring_project.registry,
            voxy_scene::PrefabLimits {
                max_objects: 128,
                max_instances: 128,
                max_depth: 16,
            },
            |asset| {
                snapshot.dependencies.get(asset).cloned().ok_or_else(|| {
                    voxy_scene::DocumentError::Invalid(format!("missing historical prefab {asset}"))
                })
            },
        )?;
        Ok(baseline
            .objects
            .into_iter()
            .find(|object| &object.id == selected))
    }

    pub(super) fn prefab_overridden_fields(
        &self,
        document: &voxy_scene::SceneDocument,
    ) -> Result<std::collections::BTreeSet<usize>, Box<dyn std::error::Error>> {
        if self.play.playing.is_some() || document.objects.is_empty() {
            return Ok(Default::default());
        }
        let Some(base) = self.prefab_field_base()? else {
            return Ok(Default::default());
        };
        let Some(object) = document.objects.get(self.selected) else {
            return Ok(Default::default());
        };
        if matches!(self.inspector, InspectorMode::Components(_)) {
            return Ok(crate::component_fields::fields(object)?
                .into_iter()
                .enumerate()
                .filter_map(|(index, field)| {
                    let differs = field
                        .prefab_reset_value(
                            object,
                            &base,
                            &self.authoring.authoring_project.registry,
                        )
                        .ok()
                        .flatten()
                        .is_some_and(|(path, inherited)| {
                            object.components[&field.schema]
                                .pointer(&path)
                                .is_some_and(|value| value != &inherited)
                        });
                    differs.then_some(index)
                })
                .collect());
        }
        Ok((0..9)
            .filter(|&index| built_in_overridden(self.inspector, index, object, &base))
            .collect())
    }

    pub(super) fn reset_prefab_field(
        &mut self,
        index: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_some() {
            return Err("stop playing before resetting a prefab field".into());
        }
        let mut snapshot = self.prefab_reset_snapshot()?;
        let base = self
            .prefab_field_base()?
            .ok_or("selected object is not a prefab instance")?;
        let mut document = self.authoring_document()?;
        let object = document
            .objects
            .get_mut(self.selected)
            .ok_or("missing selected object")?;
        let generic_schema = if matches!(self.inspector, InspectorMode::Components(_)) {
            crate::component_fields::fields(object)?
                .into_iter()
                .nth(index)
                .map(|field| field.schema)
        } else {
            None
        };
        if matches!(self.inspector, InspectorMode::Components(_)) {
            let field = crate::component_fields::fields(object)?
                .into_iter()
                .nth(index)
                .ok_or("missing component field")?;
            let (path, replacement) = field
                .prefab_reset_value(object, &base, &self.authoring.authoring_project.registry)?
                .ok_or("prefab has no inherited field for this item")?;
            *object
                .components
                .get_mut(&field.schema)
                .and_then(|value| value.pointer_mut(&path))
                .ok_or("component field disappeared")? = replacement;
        } else {
            built_in_reset(self.inspector, index, object, &base)?;
        }
        // An explicit field reset may split a legacy atomic component override.
        // Merely loading/saving old files continues to preserve atomic semantics.
        if let Some(Target::Member(schema, _)) = target(self.inspector, index, object) {
            for instance in &mut snapshot.source.instances {
                for (local, overrides) in &mut instance.overrides {
                    if voxy_scene::instance_object_id(&instance.id, local) == object.id {
                        overrides.components.remove(schema);
                    }
                }
            }
        }
        if let Some(schema) = generic_schema {
            for instance in &mut snapshot.source.instances {
                for (local, overrides) in &mut instance.overrides {
                    if voxy_scene::instance_object_id(&instance.id, local) == object.id {
                        overrides.components.remove(&schema);
                    }
                }
            }
        }
        self.commit_prefab_reset(snapshot, document)
    }
    pub(super) fn prefab_reset_snapshot(
        &self,
    ) -> Result<crate::prefab_authoring::AuthoredScene, Box<dyn std::error::Error>> {
        let observed = self
            .authoring
            .authoring_source
            .as_ref()
            .ok_or("missing observed prefab scene")?;
        self.authoring.authoring_project.validate(observed)?;
        let snapshot: crate::prefab_authoring::AuthoredScene = serde_json::from_value(
            self.authoring
                .history
                .as_ref()
                .ok_or("missing history")?
                .metadata()
                .clone(),
        )?;
        for (asset, expected) in &snapshot.dependencies {
            if observed.value().dependencies.get(asset) != Some(expected) {
                return Err("historical prefab source changed; reload before resetting".into());
            }
        }
        Ok(snapshot)
    }
    pub(super) fn commit_prefab_reset(
        &mut self,
        mut snapshot: crate::prefab_authoring::AuthoredScene,
        document: voxy_scene::SceneDocument,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.validate_authoring_document(&document)?;
        let registry = &self.authoring.authoring_project.registry;
        let baseline = snapshot.source.instance_baseline(
            &registry,
            voxy_scene::PrefabLimits {
                max_objects: 128,
                max_instances: 128,
                max_depth: 16,
            },
            |asset| {
                snapshot.dependencies.get(asset).cloned().ok_or_else(|| {
                    voxy_scene::DocumentError::Invalid(format!("missing historical prefab {asset}"))
                })
            },
        )?;
        snapshot.source = snapshot
            .source
            .capture_edits(&baseline, &document, &registry, 128)?;
        snapshot.expanded = document.clone();
        self.authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit_with_metadata(document, serde_json::to_value(snapshot)?, &registry)?;
        self.restore_authoring()?;
        self.field = None;
        self.panel_cache = None;
        Ok(())
    }
}

#[cfg(test)]
mod collection_reset_tests {
    use crate::{App, InspectorMode, ModelSource, component_fields::fields, panels::Action};
    #[derive(serde::Serialize, serde::Deserialize)]
    struct List {
        items: Vec<Item>,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Item {
        id: String,
        count: u32,
        gain: u32,
    }
    fn field(app: &App, item: &str, member: &str) -> usize {
        let document = app.authoring_document().unwrap();
        let index = document.objects[0].components["custom.list"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .position(|value| value["id"] == item)
            .unwrap();
        fields(&document.objects[0])
            .unwrap()
            .iter()
            .position(|field| {
                field.schema == "custom.list" && field.path == format!("/items/{index}/{member}")
            })
            .unwrap()
    }
    fn reset_item(app: &mut App, id: &str) {
        app.inspector = InspectorMode::Collections(0, 0);
        let document = app.authoring_document().unwrap();
        let lists = crate::component_collections::collections(
            &document.objects[0],
            &app.authoring.authoring_project.registry,
        )
        .unwrap();
        let key = lists
            .iter()
            .find(|list| list.schema == "custom.list")
            .unwrap()
            .key;
        let action = Action::CollectionReset(key, crate::component_collections::item_key(id));
        let mut panels =
            crate::panels::Panels::with_registry(app.authoring.authoring_project.registry.clone())
                .unwrap();
        panels.collection_resets = app.prefab_collection_resets(&document).unwrap();
        panels
            .build(
                &document,
                app.selected,
                glam::Vec2::new(1000.0, 700.0),
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
        panels.frame_outcome(voxy_render::RenderOutcome::Presented);
        let rect = panels
            .regions
            .iter()
            .find(|(_, target)| *target == action)
            .unwrap()
            .0;
        let point = glam::Vec2::new(rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0);
        assert_eq!(panels.hit(point), Some(action));
        app.panels = Some(panels);
        assert_eq!(app.panel_target(point).unwrap(), Some(action));
        app.panel_action(action).unwrap();
    }
    #[test]
    fn collection_member_reset_preserves_topology_other_members_and_source_inheritance() {
        let root = std::env::temp_dir().join(format!("voxy-item-reset-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let model = root.join("mesh.obj");
        std::fs::write(&model, "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        let mut registry = crate::editor_component_registry().unwrap();
        registry.register::<List>("custom.list").unwrap();
        registry
            .declare_identified_collection("custom.list", "/items", "id")
            .unwrap();
        let mut app =
            crate::configured_app_with_registry(&ModelSource::File(model), None, false, registry)
                .unwrap();
        let mut leaf = app.authoring_document().unwrap();
        leaf.objects[0].components.insert("custom.list".into(), serde_json::json!({"items":[
            {"id":"a","count":1,"gain":10},{"id":"b","count":2,"gain":20},{"id":"c","count":3,"gain":30}]}));
        let prefab = root.join("leaf.prefab");
        std::fs::write(&prefab, leaf.to_json().unwrap()).unwrap();
        let scene = root.join("scene.json");
        std::fs::write(
            &scene,
            serde_json::json!({"version":1,"objects":[],"instances":[
            {"id":"outer","asset":"leaf.prefab","parent":null,"overrides":{}}]})
            .to_string(),
        )
        .unwrap();
        app.configure_scene(&scene).unwrap();
        let mut edited = app.authoring_document().unwrap();
        edited.objects[0].components.get_mut("custom.list").unwrap()["items"] = serde_json::json!([
            {"id":"b","count":9,"gain":20},{"id":"local","count":7,"gain":7},{"id":"a","count":8,"gain":11}]);
        app.authoring
            .history
            .as_mut()
            .unwrap()
            .commit(edited.clone(), &app.authoring.authoring_project.registry)
            .unwrap();
        app.restore_authoring().unwrap();
        app.save_authoring().unwrap();
        app.inspector = InspectorMode::Components(0);
        let b_count = field(&app, "b", "count");
        let marks = app.prefab_overridden_fields(&edited).unwrap();
        assert!(marks.contains(&b_count));
        assert!(!marks.contains(&field(&app, "b", "gain")));
        assert!(!marks.contains(&field(&app, "b", "id")));
        assert!(!marks.contains(&field(&app, "local", "count")));
        let old_metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        app.panel_action(Action::ResetField(b_count)).unwrap();
        let reset = app.authoring_document().unwrap();
        let mut expected = edited.clone();
        expected.objects[0]
            .components
            .get_mut("custom.list")
            .unwrap()["items"][0]["count"] = serde_json::json!(2);
        assert_eq!(reset, expected);
        assert!(
            !app.prefab_overridden_fields(&reset)
                .unwrap()
                .contains(&b_count)
        );
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), edited);
        assert_eq!(
            app.authoring.history.as_ref().unwrap().metadata(),
            &old_metadata
        );
        app.edit_key(winit::keyboard::KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), reset);
        let metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        assert!(
            app.panel_action(Action::ResetField(field(&app, "local", "count")))
                .is_err()
        );
        assert!(
            app.panel_action(Action::ResetField(field(&app, "b", "id")))
                .is_err()
        );
        assert_eq!(app.authoring_document().unwrap(), reset);
        assert_eq!(
            app.authoring.history.as_ref().unwrap().metadata(),
            &metadata
        );
        app.save_authoring().unwrap();
        let saved: voxy_scene::PrefabSceneDocument =
            serde_json::from_slice(&std::fs::read(&scene).unwrap()).unwrap();
        let changes = &saved.instances[0].overrides[&leaf.objects[0].id].component_collections["custom.list"]
            ["/items"];
        assert!(!changes.members.contains_key("b"));
        assert!(changes.removed.contains("c"));
        assert!(changes.added.contains_key("local"));
        assert_eq!(changes.members["a"]["/gain"], 11);
        leaf.objects[0].components.get_mut("custom.list").unwrap()["items"] = serde_json::json!([
            {"id":"c","count":3,"gain":30},{"id":"a","count":100,"gain":10},{"id":"b","count":4,"gain":40}]);
        std::fs::write(&prefab, leaf.to_json().unwrap()).unwrap();
        app.load_authoring().unwrap();
        let inherited = app.authoring_document().unwrap();
        assert_eq!(
            inherited.objects[0].components["custom.list"]["items"],
            serde_json::json!([
            {"id":"b","count":4,"gain":40},{"id":"local","count":7,"gain":7},{"id":"a","count":8,"gain":11}])
        );
        app.panel_action(Action::ResetField(field(&app, "a", "count")))
            .unwrap();
        assert_eq!(
            app.authoring_document().unwrap().objects[0].components["custom.list"]["items"][2]["count"],
            100
        );
        app.save_authoring().unwrap();
        app.load_authoring().unwrap();
        assert_eq!(
            app.authoring_document().unwrap().objects[0].components["custom.list"]["items"][2]["gain"],
            11
        );
        let before_whole = app.authoring_document().unwrap();
        let before_whole_metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        reset_item(&mut app, "a");
        let whole_reset = app.authoring_document().unwrap();
        assert_eq!(
            whole_reset.objects[0].components["custom.list"]["items"],
            serde_json::json!([
            {"id":"b","count":4,"gain":40},{"id":"local","count":7,"gain":7},{"id":"a","count":100,"gain":10}])
        );
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), before_whole);
        assert_eq!(
            app.authoring.history.as_ref().unwrap().metadata(),
            &before_whole_metadata
        );
        app.edit_key(winit::keyboard::KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), whole_reset);
        reset_item(&mut app, "local");
        let local_reset = app.authoring_document().unwrap();
        assert_eq!(
            local_reset.objects[0].components["custom.list"]["items"],
            serde_json::json!([
            {"id":"b","count":4,"gain":40},{"id":"a","count":100,"gain":10}])
        );
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), whole_reset);
        app.edit_key(winit::keyboard::KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), local_reset);
        app.save_authoring().unwrap();
        app.load_authoring().unwrap();
        assert_eq!(app.authoring_document().unwrap(), local_reset);
        let saved: voxy_scene::PrefabSceneDocument =
            serde_json::from_slice(&std::fs::read(&scene).unwrap()).unwrap();
        let changes = &saved.instances[0].overrides[&leaf.objects[0].id].component_collections["custom.list"]
            ["/items"];
        assert!(changes.added.is_empty());
        assert!(changes.members.is_empty());
        assert!(changes.removed.contains("c"));
        assert!(changes.order.is_some());
        let key = crate::component_collections::collections(
            &local_reset.objects[0],
            &app.authoring.authoring_project.registry,
        )
        .unwrap()[0]
            .key;
        assert!(
            app.prefab_collection_order_resets(&local_reset)
                .unwrap()
                .contains(&key)
        );
        let before_order_metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        app.inspector = InspectorMode::Collections(0, 0);
        let mut panels =
            crate::panels::Panels::with_registry(app.authoring.authoring_project.registry.clone())
                .unwrap();
        panels.collection_order_resets = app.prefab_collection_order_resets(&local_reset).unwrap();
        panels.collection_deleted_resets =
            app.prefab_collection_deleted_resets(&local_reset).unwrap();
        panels
            .build(
                &local_reset,
                app.selected,
                glam::Vec2::new(1000.0, 700.0),
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
        panels.frame_outcome(voxy_render::RenderOutcome::Presented);
        let action = Action::CollectionResetOrder(key);
        let restore_rect = panels
            .regions
            .iter()
            .find(|(_, action)| *action == Action::CollectionRestoreDeleted(key))
            .unwrap()
            .0;
        assert_eq!(
            panels.hit(glam::Vec2::new(
                restore_rect[0] + restore_rect[2] / 2.0,
                restore_rect[1] + restore_rect[3] / 2.0
            )),
            Some(Action::CollectionRestoreDeleted(key))
        );
        let rect = panels
            .regions
            .iter()
            .find(|(_, target)| *target == action)
            .unwrap()
            .0;
        let point = glam::Vec2::new(rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0);
        assert_eq!(panels.hit(point), Some(action));
        app.panels = Some(panels);
        assert_eq!(app.panel_target(point).unwrap(), Some(action));
        app.panel_action(action).unwrap();
        let ordered = app.authoring_document().unwrap();
        assert_eq!(
            ordered.objects[0].components["custom.list"]["items"],
            serde_json::json!([
                {"id":"a","count":100,"gain":10}, {"id":"b","count":4,"gain":40}
            ])
        );
        assert!(
            app.prefab_collection_order_resets(&ordered)
                .unwrap()
                .is_empty()
        );
        let snapshot: crate::prefab_authoring::AuthoredScene =
            serde_json::from_value(app.authoring.history.as_ref().unwrap().metadata().clone())
                .unwrap();
        let changes = &snapshot.source.instances[0].overrides[&leaf.objects[0].id]
            .component_collections["custom.list"]["/items"];
        assert!(changes.order.is_none());
        assert!(changes.removed.contains("c"));
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), local_reset);
        assert_eq!(
            app.authoring.history.as_ref().unwrap().metadata(),
            &before_order_metadata
        );
        app.edit_key(winit::keyboard::KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), ordered);
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert!(
            app.prefab_collection_deleted_resets(&local_reset)
                .unwrap()
                .contains(&key)
        );
        let before_restore_metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        app.panel_action(Action::CollectionRestoreDeleted(key))
            .unwrap();
        let restored = app.authoring_document().unwrap();
        assert_eq!(
            restored.objects[0].components["custom.list"]["items"],
            serde_json::json!([
                {"id":"b","count":4,"gain":40}, {"id":"c","count":3,"gain":30}, {"id":"a","count":100,"gain":10}
            ])
        );
        assert!(
            app.prefab_collection_deleted_resets(&restored)
                .unwrap()
                .is_empty()
        );
        let snapshot: crate::prefab_authoring::AuthoredScene =
            serde_json::from_value(app.authoring.history.as_ref().unwrap().metadata().clone())
                .unwrap();
        let changes = &snapshot.source.instances[0].overrides[&leaf.objects[0].id]
            .component_collections["custom.list"]["/items"];
        assert!(changes.removed.is_empty());
        assert!(changes.order.is_some());
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), local_reset);
        assert_eq!(
            app.authoring.history.as_ref().unwrap().metadata(),
            &before_restore_metadata
        );
        app.edit_key(winit::keyboard::KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), restored);
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        // Changed external sources reject Reset before publishing any history.
        app.panel_action(Action::CollectionDelete(
            key,
            crate::component_collections::item_key("b"),
        ))
        .unwrap();
        let two_deleted = app.authoring_document().unwrap();
        let two_deleted_metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        let mut panels =
            crate::panels::Panels::with_registry(app.authoring.authoring_project.registry.clone())
                .unwrap();
        panels.collection_deleted_items =
            app.prefab_collection_deleted_items(&two_deleted).unwrap();
        assert_eq!(panels.collection_deleted_items[&key].len(), 2);
        // Exercise the chooser's second page independently of prefab mutation.
        let third = crate::component_collections::item_key("third");
        let fourth = crate::component_collections::item_key("fourth");
        panels
            .collection_deleted_items
            .get_mut(&key)
            .unwrap()
            .extend([(third, "Third".into()), (fourth, "Fourth".into())]);
        panels.deleted_page = 1;
        panels
            .build(
                &two_deleted,
                app.selected,
                glam::Vec2::new(1000.0, 700.0),
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
        panels.frame_outcome(voxy_render::RenderOutcome::Presented);
        let fourth_action = Action::CollectionRestoreItem(key, fourth);
        let fourth_rect = panels
            .regions
            .iter()
            .find(|(_, target)| *target == fourth_action)
            .unwrap()
            .0;
        assert_eq!(
            panels.hit(glam::Vec2::new(
                fourth_rect[0] + fourth_rect[2] / 2.0,
                fourth_rect[1] + fourth_rect[3] / 2.0
            )),
            Some(fourth_action)
        );
        assert!(
            !panels
                .regions
                .iter()
                .any(|(_, target)| *target == Action::CollectionRestoreItem(key, third))
        );
        panels
            .collection_deleted_items
            .get_mut(&key)
            .unwrap()
            .truncate(2);
        panels.deleted_page = 0;
        panels
            .build(
                &two_deleted,
                app.selected,
                glam::Vec2::new(1000.0, 700.0),
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
        panels.frame_outcome(voxy_render::RenderOutcome::Presented);
        let action =
            Action::CollectionRestoreItem(key, crate::component_collections::item_key("b"));
        let rect = panels
            .regions
            .iter()
            .find(|(_, target)| *target == action)
            .unwrap()
            .0;
        let point = glam::Vec2::new(rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0);
        assert_eq!(panels.hit(point), Some(action));
        app.panels = Some(panels);
        assert_eq!(app.panel_target(point).unwrap(), Some(action));
        app.panel_action(action).unwrap();
        let one_restored = app.authoring_document().unwrap();
        assert_eq!(
            one_restored.objects[0].components["custom.list"]["items"],
            serde_json::json!([
                {"id":"a","count":100,"gain":10}, {"id":"b","count":4,"gain":40}
            ])
        );
        let snapshot: crate::prefab_authoring::AuthoredScene =
            serde_json::from_value(app.authoring.history.as_ref().unwrap().metadata().clone())
                .unwrap();
        let changes = &snapshot.source.instances[0].overrides[&leaf.objects[0].id]
            .component_collections["custom.list"]["/items"];
        assert_eq!(
            changes.removed,
            std::collections::BTreeSet::from(["c".to_owned()])
        );
        let stable_metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        assert!(
            app.panel_action(Action::CollectionRestoreItem(
                key,
                crate::component_collections::item_key("b")
            ))
            .is_err()
        );
        assert!(
            app.panel_action(Action::CollectionRestoreItem(
                key,
                crate::component_collections::item_key("missing")
            ))
            .is_err()
        );
        assert_eq!(app.authoring_document().unwrap(), one_restored);
        assert_eq!(
            app.authoring.history.as_ref().unwrap().metadata(),
            &stable_metadata
        );
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), two_deleted);
        assert_eq!(
            app.authoring.history.as_ref().unwrap().metadata(),
            &two_deleted_metadata
        );
        app.edit_key(winit::keyboard::KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), one_restored);
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), local_reset);
        leaf.objects[0].components.get_mut("custom.list").unwrap()["items"][1]["gain"] =
            serde_json::json!(200);
        std::fs::write(&prefab, leaf.to_json().unwrap()).unwrap();
        let metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        let key = crate::component_collections::collections(
            &local_reset.objects[0],
            &app.authoring.authoring_project.registry,
        )
        .unwrap()[0]
            .key;
        assert!(
            app.panel_action(Action::CollectionReset(
                key,
                crate::component_collections::item_key("a")
            ))
            .is_err()
        );
        assert_eq!(app.authoring_document().unwrap(), local_reset);
        assert_eq!(
            app.authoring.history.as_ref().unwrap().metadata(),
            &metadata
        );
        let removed_source = leaf.objects[0].components.get_mut("custom.list").unwrap()["items"]
            .as_array_mut()
            .unwrap()
            .remove(0);
        assert_eq!(removed_source["id"], "c");
        std::fs::write(&prefab, leaf.to_json().unwrap()).unwrap();
        app.load_authoring().unwrap();
        let absent_document = app.authoring_document().unwrap();
        let absent_metadata = app.authoring.history.as_ref().unwrap().metadata().clone();
        assert!(
            app.prefab_collection_deleted_resets(&absent_document)
                .unwrap()
                .contains(&key)
        );
        app.panel_action(Action::CollectionRestoreItem(
            key,
            crate::component_collections::item_key("c"),
        ))
        .unwrap();
        assert_eq!(app.authoring_document().unwrap(), absent_document);
        assert!(
            app.prefab_collection_deleted_resets(&absent_document)
                .unwrap()
                .is_empty()
        );
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(
            app.authoring.history.as_ref().unwrap().metadata(),
            &absent_metadata
        );
        app.edit_key(winit::keyboard::KeyCode::KeyY).unwrap();
        app.save_authoring().unwrap();
        app.load_authoring().unwrap();
        assert!(
            app.prefab_collection_deleted_resets(&app.authoring_document().unwrap())
                .unwrap()
                .is_empty()
        );
        leaf.objects[0].components.get_mut("custom.list").unwrap()["items"]
            .as_array_mut()
            .unwrap()
            .insert(0, removed_source);
        std::fs::write(&prefab, leaf.to_json().unwrap()).unwrap();
        app.load_authoring().unwrap();
        assert!(
            app.authoring_document().unwrap().objects[0].components["custom.list"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["id"] == "c")
        );
        app.stop_workers().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
