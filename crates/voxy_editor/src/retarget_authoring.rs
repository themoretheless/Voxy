//! A profile edit is one authoring transaction, including source and named mappings.
use crate::{App, InspectorMode, ModelRetarget, RetargetJointProfile};
use voxy_assets::AssetId;
use voxy_scene::SceneDocument;
const SCHEMA: &str = "editor.model-retarget.v1";
#[derive(Debug)]
pub(super) struct Draft {
    pub base: SceneDocument,
    pub document: SceneDocument,
    owner: usize,
}
#[derive(Clone, Debug)]
pub(super) struct BonePicker {
    binding: crate::component_fields::BoundComponentField,
    profile: serde_json::Value,
    target: bool,
    model: std::sync::Arc<voxy_render::ModelAsset>,
    pub choices: Vec<String>,
    pub page: usize,
}
impl App {
    fn retarget_bone_model(
        &self,
        target: bool,
    ) -> Result<std::sync::Arc<voxy_render::ModelAsset>, Box<dyn std::error::Error>> {
        let draft = self
            .retarget_draft
            .as_ref()
            .ok_or("open a profile edit first")?;
        let asset = if target {
            self.scene
                .component::<crate::ModelInstance>(self.instances[draft.owner])?
                .ok_or("missing target owner")?
                .asset
                .clone()
        } else {
            let source = draft.document.objects[draft.owner].components[SCHEMA]
                .get("source")
                .and_then(serde_json::Value::as_str)
                .ok_or("missing source asset")?;
            AssetId(source.into())
        };
        self.catalog
            .snapshot(&asset)
            .ok_or("wait for the rig to load")?
            .value()
            .animated
            .clone()
            .ok_or_else(|| "asset has no skeletal rig".into())
    }
    pub(super) fn open_retarget_bones(
        &mut self,
        index: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let draft = self
            .retarget_draft
            .as_ref()
            .ok_or("open a profile edit first")?;
        let object = &draft.document.objects[draft.owner];
        let field = crate::component_fields::fields(object)?
            .into_iter()
            .nth(index)
            .ok_or("missing bone field")?;
        let parts = field.path.split('/').collect::<Vec<_>>();
        if field.schema != SCHEMA
            || parts.len() != 4
            || parts[1] != "joints"
            || !matches!(parts[3], "source" | "target")
        {
            return Err("choose a source or target bone field".into());
        }
        let target = parts[3] == "target";
        let model = self.retarget_bone_model(target)?;
        let choices = model
            .joint_names()
            .iter()
            .enumerate()
            .filter_map(|(i, name)| {
                let name = name.as_ref()?;
                (model.resolve_joint_name(name).ok().map(usize::from) == Some(i))
                    .then(|| name.to_string())
            })
            .collect::<Vec<_>>();
        if choices.is_empty() {
            return Err("rig has no unique named bones".into());
        }
        self.retarget_picker = Some(BonePicker {
            binding: field.bind(object, &self.authoring.authoring_project.registry)?,
            profile: object.components[SCHEMA].clone(),
            target,
            model,
            choices,
            page: 0,
        });
        self.field = None;
        self.component_edit = None;
        self.panel_cache = None;
        Ok(())
    }
    pub(super) fn choose_retarget_bone(
        &mut self,
        index: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let picker = self.retarget_picker.as_ref().ok_or("no bone selection")?;
        let name = picker
            .choices
            .get(index)
            .ok_or("missing bone choice")?
            .clone();
        let draft = self.retarget_draft.as_ref().ok_or("missing profile edit")?;
        if draft.document.objects[draft.owner].components.get(SCHEMA) != Some(&picker.profile) {
            return Err("profile changed; reopen the bone list".into());
        }
        let model = self.retarget_bone_model(picker.target)?;
        if !std::sync::Arc::ptr_eq(&model, &picker.model) {
            return Err("rig changed; reopen the bone list".into());
        }
        model.resolve_joint_name(&name)?;
        let binding = picker.binding.clone();
        self.commit_component_binding(&binding, &name)?;
        self.retarget_picker = None;
        self.panel_cache = None;
        Ok(())
    }
    pub(super) fn retarget_bone_page(
        &mut self,
        forward: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let picker = self.retarget_picker.as_mut().ok_or("no bone selection")?;
        let pages = picker.choices.len().div_ceil(6).max(1);
        picker.page = if forward {
            (picker.page + 1) % pages
        } else {
            (picker.page + pages - 1) % pages
        };
        self.panel_cache = None;
        Ok(())
    }

    pub(super) fn begin_retarget(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_some() || self.retarget_draft.is_some() {
            return Err("stop Play and finish the current profile edit first".into());
        }
        let base = self.authoring_document()?;
        let mut document = base.clone();
        let object = document
            .objects
            .get_mut(self.selected)
            .ok_or("select a model owner")?;
        if !object.components.contains_key(SCHEMA) {
            let node = *self
                .instances
                .get(self.selected)
                .ok_or("missing model owner")?;
            let asset = &self
                .scene
                .component::<crate::ModelInstance>(node)?
                .ok_or("retarget requires a model owner")?
                .asset;
            let imported = self
                .catalog
                .snapshot(asset)
                .ok_or("wait for the target rig to load")?;
            let model = imported
                .value()
                .animated
                .as_ref()
                .ok_or("target has no skeletal rig")?;
            let joints = model
                .joint_names()
                .iter()
                .enumerate()
                .filter_map(|(index, name)| {
                    let name = name.as_ref()?;
                    (model.resolve_joint_name(name).ok().map(usize::from) == Some(index)).then(
                        || RetargetJointProfile {
                            source: name.to_string(),
                            target: name.to_string(),
                            rotation_basis: glam::Quat::IDENTITY.to_array(),
                            translation_basis: glam::Quat::IDENTITY.to_array(),
                            translation_scale: 1.,
                        },
                    )
                })
                .collect::<Vec<_>>();
            if joints.is_empty() {
                return Err("target rig has no unique named bones".into());
            }
            object.components.insert(
                SCHEMA.into(),
                serde_json::to_value(ModelRetarget {
                    source: asset.0.clone(),
                    joints,
                })?,
            );
        }
        self.retarget_draft = Some(Draft {
            base,
            document,
            owner: self.selected,
        });
        self.inspector = InspectorMode::Components(0);
        self.field = None;
        self.component_edit = None;
        self.panel_cache = None;
        Ok(())
    }
    pub(super) fn cancel_retarget(&mut self) {
        self.retarget_draft = None;
        self.retarget_picker = None;
        self.field = None;
        self.component_edit = None;
        self.panel_cache = None;
    }
    pub(super) fn apply_retarget(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let draft = self.retarget_draft.as_ref().ok_or("no profile edit")?;
        if self.authoring_document()? != draft.base {
            return Err("scene changed during profile edit; cancel and reopen".into());
        }
        if let Some(value) = draft.document.objects[draft.owner].components.get(SCHEMA) {
            let profile: ModelRetarget = serde_json::from_value(value.clone())?;
            profile.validate()?;
            let node = self.instances[draft.owner];
            let target_id = &self
                .scene
                .component::<crate::ModelInstance>(node)?
                .ok_or("missing target owner")?
                .asset;
            let source = self
                .catalog
                .snapshot(&AssetId(profile.source.clone()))
                .ok_or("wait for the source rig to load")?;
            let target = self
                .catalog
                .snapshot(target_id)
                .ok_or("wait for the target rig to load")?;
            profile.compile_models(
                source
                    .value()
                    .animated
                    .as_ref()
                    .ok_or("source has no skeletal rig")?,
                target
                    .value()
                    .animated
                    .as_ref()
                    .ok_or("target has no skeletal rig")?,
            )?;
        }
        let document = draft.document.clone();
        let next = self.validate_authoring_document(&document)?;
        self.authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit(document, &self.authoring.authoring_project.registry)?;
        self.restore_authoring()?;
        self.authoring.next_object_id = next;
        self.cancel_retarget();
        Ok(())
    }
    pub(super) fn remove_retarget_draft(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let draft = self.retarget_draft.as_mut().ok_or("no profile edit")?;
        draft.document.objects[draft.owner]
            .components
            .remove(SCHEMA);
        self.field = None;
        self.component_edit = None;
        self.panel_cache = None;
        Ok(())
    }
    pub(super) fn delete_retarget_pair(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        let draft = self.retarget_draft.as_mut().ok_or("open a profile edit first")?;
        let pairs = draft.document.objects[draft.owner].components.get_mut(SCHEMA)
            .and_then(|value| value.get_mut("joints"))
            .and_then(serde_json::Value::as_array_mut).ok_or("missing bone pairs")?;
        if index >= pairs.len() { return Err("missing bone pair".into()); }
        pairs.remove(index);
        self.retarget_picker = None;
        self.field = None;
        self.component_edit = None;
        self.panel_cache = None;
        Ok(())
    }
    pub(super) fn retarget_pair(&mut self, add: bool) -> Result<(), Box<dyn std::error::Error>> {
        let draft = self
            .retarget_draft
            .as_mut()
            .ok_or("open a profile edit first")?;
        let pairs = draft.document.objects[draft.owner]
            .components
            .get_mut(SCHEMA)
            .and_then(|value| value.get_mut("joints"))
            .and_then(serde_json::Value::as_array_mut)
            .ok_or("missing bone pairs")?;
        if add {
            if pairs.len() >= voxy_animation::MAX_JOINTS {
                return Err("bone pair capacity exceeded".into());
            }
            pairs.push(serde_json::to_value(RetargetJointProfile {
                source: String::new(),
                target: String::new(),
                rotation_basis: glam::Quat::IDENTITY.to_array(),
                translation_basis: glam::Quat::IDENTITY.to_array(),
                translation_scale: 1.,
            })?);
        } else {
            pairs.pop();
        }
        self.field = None;
        self.component_edit = None;
        self.panel_cache = None;
        Ok(())
    }
    pub(super) fn queue_retarget_draft_source(&mut self) {
        let Some(draft) = &self.retarget_draft else {
            return;
        };
        let Some(source) = draft.document.objects[draft.owner]
            .components
            .get(SCHEMA)
            .and_then(|v| v.get("source"))
            .and_then(serde_json::Value::as_str)
        else {
            return;
        };
        if !source.is_empty() && source.len() <= 1024 && !source.contains('\0') {
            let id = AssetId(source.into());
            if self.catalog.status(&id).is_none() {
                self.reload.insert(id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panels::Action;
    fn field(app: &App, path: &str) -> usize {
        crate::component_fields::fields(&app.panel_document().unwrap().objects[app.selected])
            .unwrap()
            .iter()
            .position(|f| f.schema == SCHEMA && f.path == path)
            .unwrap()
    }
    #[test]
    fn source_and_pairs_apply_once_invalid_drafts_cancel_and_history_are_atomic() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "voxy-retarget-authoring-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let original = include_bytes!("../../voxy_render/examples/assets/foot-contact.glb");
        std::fs::write(root.join("target.glb"), original).unwrap();
        let glb = gltf::binary::Glb::from_slice(original).unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&glb.json).unwrap();
        json["nodes"][0]["name"] = serde_json::json!("sourceHip");
        std::fs::write(
            root.join("source.glb"),
            gltf::binary::Glb {
                header: glb.header,
                json: serde_json::to_vec(&json).unwrap().into(),
                bin: glb.bin,
            }
            .to_vec()
            .unwrap(),
        )
        .unwrap();
        let mut app = App::new(&root.join("target.glb"), false).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.catalog.snapshot(&app.id).is_none() {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let before = app.authoring_document().unwrap();
        app.panel_action(Action::Retarget).unwrap();
        assert_eq!(app.authoring_document().unwrap(), before);
        assert!(app.toggle_play().is_err());
        assert!(app.panel_action(Action::Select(0)).is_err());
        let index = field(&app, "/source");
        app.edit_component_field(index, "source.glb").unwrap();
        while app
            .catalog
            .snapshot(&AssetId("source.glb".into()))
            .is_none()
        {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(app.panel_action(Action::RetargetApply).is_err());
        assert_eq!(app.authoring_document().unwrap(), before);
        assert!(app.retarget_draft.is_some());
        let index = field(&app, "/joints/0/source");
        app.panel_action(Action::RetargetBones(index)).unwrap();
        assert!(
            app.retarget_picker
                .as_ref()
                .unwrap()
                .choices
                .contains(&"sourceHip".into())
        );
        let scale = field(&app, "/joints/0/translation_scale");
        app.edit_component_field(scale, "2").unwrap();
        assert!(app.panel_action(Action::RetargetBone(0)).is_err());
        assert_eq!(app.authoring_document().unwrap(), before);
        app.panel_action(Action::RetargetBoneClose).unwrap();
        app.panel_action(Action::RetargetBones(index)).unwrap();
        let choice = app
            .retarget_picker
            .as_ref()
            .unwrap()
            .choices
            .iter()
            .position(|name| name == "sourceHip")
            .unwrap();
        app.panel_action(Action::RetargetBonePage(true)).unwrap();
        app.panel_action(Action::RetargetBone(choice)).unwrap();
        assert!(app.retarget_picker.is_none());
        let target_field = field(&app, "/joints/0/target");
        app.panel_action(Action::RetargetBones(target_field)).unwrap();
        let target_model = app.retarget_picker.as_ref().unwrap().model.clone();
        let target_choice = app.retarget_picker.as_ref().unwrap().choices.iter()
            .position(|name| name == "hip").unwrap();
        assert!(!app.retarget_picker.as_ref().unwrap().choices.contains(&"sourceHip".into()));
        let draft_before_reload = app.retarget_draft.as_ref().unwrap().document.clone();
        // Reimport identical names: name lookup alone cannot detect this stale list.
        app.reload.insert(app.id.clone());
        let reload_deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::sync::Arc::ptr_eq(&target_model, &app.retarget_bone_model(true).unwrap()) {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < reload_deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(app.panel_action(Action::RetargetBone(target_choice)).is_err());
        assert_eq!(app.retarget_draft.as_ref().unwrap().document, draft_before_reload);
        assert_eq!(app.authoring_document().unwrap(), before);
        app.panel_action(Action::RetargetBoneClose).unwrap();
        app.panel_action(Action::RetargetBones(target_field)).unwrap();
        let choice = app.retarget_picker.as_ref().unwrap().choices.iter()
            .position(|name| name == "hip").unwrap();
        app.panel_action(Action::RetargetBone(choice)).unwrap();
        assert!(app.retarget_picker.is_none());
        let index = field(&app, "/joints/0/rotation_basis/2");
        app.edit_component_field(index, "0.70710677").unwrap();
        assert!(app.panel_action(Action::RetargetApply).is_err());
        assert_eq!(app.authoring_document().unwrap(), before);
        let index = field(&app, "/joints/0/rotation_basis/3");
        app.edit_component_field(index, "0.70710677").unwrap();
        app.panel_action(Action::RetargetApply).unwrap();
        let accepted = app.authoring_document().unwrap();
        assert_ne!(accepted, before);
        assert!(app.retarget_draft.is_none());
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), before);
        app.edit_key(winit::keyboard::KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), accepted);
        app.panel_action(Action::Retarget).unwrap();
        let pairs_before = app.retarget_draft.as_ref().unwrap().document.objects[0].components[SCHEMA]["joints"].as_array().unwrap().clone();
        assert!(pairs_before.len() >= 3);
        let bone_field = field(&app, "/joints/1/source");
        app.panel_action(Action::RetargetBones(bone_field)).unwrap();
        app.panel_action(Action::RetargetDeletePair(1)).unwrap();
        assert!(app.retarget_picker.is_none());
        let mut expected = pairs_before.clone(); expected.remove(1);
        assert_eq!(app.retarget_draft.as_ref().unwrap().document.objects[0].components[SCHEMA]["joints"], serde_json::json!(expected));
        let draft_after = app.retarget_draft.as_ref().unwrap().document.clone();
        assert!(app.panel_action(Action::RetargetDeletePair(usize::MAX)).is_err());
        assert_eq!(app.retarget_draft.as_ref().unwrap().document, draft_after);
        assert_eq!(app.authoring_document().unwrap(), accepted);
        app.panel_action(Action::RetargetApply).unwrap();
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), accepted);
        app.panel_action(Action::Retarget).unwrap();
        app.panel_action(Action::RetargetPair(true)).unwrap();
        assert!(app.panel_action(Action::RetargetApply).is_err());
        app.panel_action(Action::RetargetCancel).unwrap();
        assert_eq!(app.authoring_document().unwrap(), accepted);
        app.panel_action(Action::Retarget).unwrap();
        app.panel_action(Action::RetargetRemove).unwrap();
        app.panel_action(Action::RetargetApply).unwrap();
        assert!(
            !app.authoring_document().unwrap().objects[0]
                .components
                .contains_key(SCHEMA)
        );
        app.edit_key(winit::keyboard::KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), accepted);
        app.stop_workers().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn full_capacity_profiles_fit_the_paged_component_inspector() {
        let profile = ModelRetarget {
            source: "source.glb".into(),
            joints: (0..voxy_animation::MAX_JOINTS)
                .map(|i| RetargetJointProfile {
                    source: format!("source{i}"),
                    target: format!("target{i}"),
                    rotation_basis: glam::Quat::IDENTITY.to_array(),
                    translation_basis: glam::Quat::IDENTITY.to_array(),
                    translation_scale: 1.,
                })
                .collect(),
        };
        profile.validate().unwrap();
        // Reuse the repository's ordinary scene serializer for the object shape.
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/foot-contact.glb");
        let mut app = App::new(&fixture, false).unwrap();
        let mut object = app.authoring_document().unwrap().objects.remove(0);
        object
            .components
            .insert(SCHEMA.into(), serde_json::to_value(profile).unwrap());
        assert!(crate::component_fields::fields(&object).unwrap().len() > 256);
        app.stop_workers().unwrap();
    }
}
