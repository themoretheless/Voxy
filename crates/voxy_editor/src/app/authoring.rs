//! Authoring document persistence: save, load, restore, validate and commit.
//!
//! Methods of [`App`] split out of lib.rs (plan step C4); they keep the same
//! bodies and private access through `crate::*`.

use crate::*;

impl App {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn authoring_document(&self) -> Result<SceneDocument, voxy_scene::SceneGraphError> {
        CharacterPhysics::new(&self.scene, 128, 128)
            .validate(&self.scene)
            .map_err(|error| match error {
                voxy_gameplay::PhysicsError::Scene(error) => error,
                _ => voxy_scene::SceneGraphError::InvalidTransform,
            })?;
        if self
            .scene
            .components::<EditorCamera>()
            .any(|(_, camera)| !camera.valid())
        {
            return Err(voxy_scene::SceneGraphError::InvalidTransform);
        }
        if self
            .scene
            .components::<SceneMaterial>()
            .any(|(_, material)| !material.valid())
            || self
                .scene
                .components::<DirectionalLight>()
                .any(|(_, light)| !light.valid())
        {
            return Err(voxy_scene::SceneGraphError::InvalidTransform);
        }
        if self
            .scene
            .components::<voxy_scene::FogVolume>()
            .any(|(_, fog)| fog.validate().is_err())
        {
            return Err(voxy_scene::SceneGraphError::InvalidTransform);
        }
        shadow_authoring::validate_scene(&self.scene)
            .map_err(|_| voxy_scene::SceneGraphError::InvalidTransform)?;
        let registry = &self.authoring.authoring_project.registry;
        let objects = self
            .instances
            .iter()
            .enumerate()
            .map(|(index, node)| {
                self.scene.world_matrix(*node)?;
                let local = self.scene.local(*node)?;
                Ok(SceneObject {
                    id: self.object_ids[index].clone(),
                    parent: self.scene.parent(*node)?.and_then(|parent| {
                        self.instances
                            .iter()
                            .position(|node| *node == parent)
                            .map(|index| self.object_ids[index].clone())
                    }),
                    name: self.scene.name(*node)?.into(),
                    active: self.scene.active_self(*node)?,
                    translation: local.translation.to_array(),
                    rotation: local.rotation.to_array(),
                    scale: local.scale.to_array(),
                    components: {
                        let mut components = registry
                            .capture_registered_components(&self.scene, *node)
                            .map_err(|_| voxy_scene::SceneGraphError::InvalidTransform)?;
                        if let Some(model) = self.scene.component::<ModelInstance>(*node)? {
                            components.insert(
                                "editor.model.v1".into(),
                                serde_json::Value::String(model.asset.0.clone()),
                            );
                        }
                        if let Some(part) = self.scene.component::<ModelPart>(*node)?
                            && part.node != u32::MAX
                            && let Some(asset) = self
                                .scene
                                .component::<ModelInstance>(*node)?
                                .and_then(|model| self.catalog.snapshot(&model.asset))
                            && usize::try_from(part.node)
                                .map_or(true, |index| index >= asset.value().nodes.len())
                        {
                            return Err(voxy_scene::SceneGraphError::InvalidNode);
                        }
                        components
                    },
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(SceneDocument {
            version: 1,
            objects,
        })
    }

    pub(crate) fn restore_authoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let document = self
            .authoring
            .history
            .as_ref()
            .ok_or("missing history")?
            .current();
        let mut loaded = document.load(
            &self.authoring.authoring_project.registry,
            scene_limits::OBJECTS,
        )?;
        let mut instances = Vec::with_capacity(document.objects.len());
        for object in &document.objects {
            let node = loaded.resolve(&object.id).ok_or("missing restored node")?;
            if let Some(asset) = object
                .components
                .get("editor.model.v1")
                .and_then(serde_json::Value::as_str)
            {
                loaded.graph.insert_component(
                    node,
                    ModelInstance {
                        asset: AssetId(asset.into()),
                    },
                )?;
            }
            instances.push(node);
        }
        self.object_ids = document
            .objects
            .iter()
            .map(|object| object.id.clone())
            .collect();
        for node in &instances {
            if let Some(model) = loaded.graph.component::<ModelInstance>(*node)?
                && self.catalog.snapshot(&model.asset).is_none()
            {
                self.reload.insert(model.asset.clone());
            }
        }
        self.parenting = None;
        self.follow_pick = None;
        if let Some((_, camera)) = loaded.graph.active_components::<EditorCamera>().next() {
            if !camera.valid() {
                return Err("invalid authored camera".into());
            }
            self.camera = camera.clone();
        }
        self.scene = loaded.graph;
        self.instances = instances;
        self.selected = self.selected.min(self.instances.len().saturating_sub(1));
        if let Some(graphics) = &mut self.graphics {
            graphics.transforms.clear();
        }
        Ok(())
    }

    pub(crate) fn save_authoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.finish_drag(false)?;
        let path = self
            .authoring
            .scene_path
            .as_ref()
            .ok_or("start editor with --scene PATH to save")?;
        let edited = self.authoring_document()?;
        let metadata = self
            .authoring
            .history
            .as_ref()
            .ok_or("missing history")?
            .metadata();
        if metadata.is_null() {
            if let Some(imported) = &self.authoring.authoring_source {
                self.authoring.authoring_project.validate(imported)?;
            }
            if let Some(revision) = &self.authoring.scene_revision {
                revision.validate(path)?;
            }
            save_scene_file(
                path,
                &edited,
                &self.authoring.authoring_project.registry,
                scene_limits::OBJECTS,
                scene_limits::DOCUMENT_BYTES,
            )?;
            self.authoring.scene_revision =
                Some(scene_revision::SceneRevision::written(path, &edited)?);
            // Refresh the observation after our own publication so the next save
            // does not mistake it for an external edit. Legacy paths also retain
            // an exact file revision without requiring project membership.
            self.authoring.authoring_source = if self.authoring.authoring_source.is_some() {
                Some(self.authoring.authoring_project.load(path)?)
            } else {
                self.authoring.authoring_project.load(path).ok()
            };
        } else {
            let snapshot: prefab_authoring::AuthoredScene =
                serde_json::from_value(metadata.clone())?;
            let imported = self
                .authoring
                .authoring_source
                .as_ref()
                .ok_or("missing observed authoring publication")?;
            let baseline = snapshot.source.instance_baseline(
                &self.authoring.authoring_project.registry,
                voxy_scene::PrefabLimits {
                    max_objects: scene_limits::OBJECTS,
                    max_instances: 128,
                    max_depth: 16,
                },
                |asset| {
                    snapshot.dependencies.get(asset).cloned().ok_or_else(|| {
                        voxy_scene::DocumentError::Invalid(format!(
                            "missing historical prefab {asset}"
                        ))
                    })
                },
            )?;
            let source = snapshot.source.capture_edits(
                &baseline,
                &edited,
                &self.authoring.authoring_project.registry,
                scene_limits::OBJECTS,
            )?;
            let proposed_metadata = serde_json::to_value(prefab_authoring::AuthoredScene {
                source: source.clone(),
                expanded: edited.clone(),
                dependencies: snapshot.dependencies.clone(),
            })?;
            self.authoring
                .history
                .as_ref()
                .ok_or("missing history")?
                .check_metadata(
                    &proposed_metadata,
                    &self.authoring.authoring_project.registry,
                )?;
            self.authoring
                .authoring_project
                .save(path, imported, &snapshot, &source)?;
            let imported = self.authoring.authoring_project.load(path)?;
            self.authoring
                .history
                .as_mut()
                .ok_or("missing history")?
                .refresh_metadata(
                    serde_json::to_value(imported.value())?,
                    &self.authoring.authoring_project.registry,
                )?;
            self.authoring.authoring_source = Some(imported);
        }
        if let Some(window) = &self.window {
            window.set_title("Voxy — scene saved");
        }
        Ok(())
    }

    /// Validate a document candidate before removing live objects or history.
    /// Registered object-reference codecs reject dangling links transactionally.
    pub(crate) fn delete_authoring_subtree(
        &mut self,
        root: &ObjectId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut document = self.authoring_document()?;
        if !document.objects.iter().any(|object| &object.id == root) {
            return Err("missing selected object".into());
        }
        let mut children: BTreeMap<ObjectId, Vec<ObjectId>> = BTreeMap::new();
        for object in &document.objects {
            if let Some(parent) = &object.parent {
                children
                    .entry(parent.clone())
                    .or_default()
                    .push(object.id.clone());
            }
        }
        let mut removed = std::collections::BTreeSet::new();
        let mut pending = vec![root.clone()];
        while let Some(id) = pending.pop() {
            if removed.insert(id.clone()) {
                pending.extend(children.remove(&id).unwrap_or_default());
            }
        }
        document
            .objects
            .retain(|object| !removed.contains(&object.id));
        let next = self.validate_authoring_document(&document)?;
        self.authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit(document, &self.authoring.authoring_project.registry)?;
        self.restore_authoring()?;
        self.authoring.next_object_id = next;
        self.update_edit_title();
        Ok(())
    }

    pub(crate) fn validate_authoring_document(
        &self,
        document: &SceneDocument,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        self.validate_authoring_document_policy(document, false)
    }
    fn validate_authoring_document_policy(
        &self,
        document: &SceneDocument,
        allow_unresolved_models: bool,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let loaded = document.load(
            &self.authoring.authoring_project.registry,
            scene_limits::OBJECTS,
        )?;
        if let Some(runtime) = self
            .authoring
            .authoring_project
            .liquid_runtime(&loaded.graph)?
        {
            voxy_gameplay::validate_game_descriptors_with_liquid_runtime(
                &loaded.graph,
                128,
                &runtime.0,
            )?;
        } else {
            voxy_gameplay::validate_game_descriptors(&loaded.graph, 128)?;
        }
        for object in &document.objects {
            loaded
                .graph
                .world_matrix(loaded.resolve(&object.id).ok_or("missing loaded node")?)?;
        }
        let mut next = self.authoring.next_object_id;
        for object in &document.objects {
            if object
                .components
                .get("editor.model.v1")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| {
                    let id = AssetId(id.into());
                    id.0.is_empty()
                        || (!allow_unresolved_models
                            && !self.available.contains(&id)
                            && self.catalog.snapshot(&id).is_none()
                            && !self
                                .scene
                                .components::<ModelInstance>()
                                .any(|(_, model)| model.asset == id))
                })
            {
                return Err("viewport scene requires known model resources".into());
            }
            if let Some(value) = object
                .id
                .0
                .strip_prefix("model-")
                .or_else(|| object.id.0.strip_prefix("object-"))
                .and_then(|value| value.parse::<u64>().ok())
            {
                next = next.max(
                    value
                        .checked_add(1)
                        .ok_or("scene object identity exhausted")?,
                );
            }
        }
        CharacterPhysics::new(&loaded.graph, 128, 128).validate(&loaded.graph)?;
        shadow_authoring::validate_scene(&loaded.graph)?;
        if loaded
            .graph
            .components::<voxy_scene::FogVolume>()
            .any(|(_, fog)| fog.validate().is_err())
        {
            return Err("invalid authored fog calibration".into());
        }
        if loaded
            .graph
            .components::<SceneMaterial>()
            .any(|(_, value)| !value.valid())
            || loaded
                .graph
                .components::<DirectionalLight>()
                .any(|(_, value)| !value.valid())
            || loaded
                .graph
                .components::<EditorCamera>()
                .any(|(_, value)| !value.valid())
        {
            return Err("invalid editor camera/material/light".into());
        }
        for (node, profile) in loaded.graph.components::<ModelRetarget>() {
            profile.validate()?;
            let target_asset = loaded
                .graph
                .component::<String>(node)?
                .ok_or("retarget profile requires a model owner")?;
            let source = self.catalog.snapshot(&AssetId(profile.source.clone()));
            let target = self.catalog.snapshot(&AssetId(target_asset.clone()));
            let source = source
                .as_ref()
                .map(|asset| {
                    asset
                        .value()
                        .animated
                        .as_ref()
                        .ok_or("retarget source has no skeletal model")
                })
                .transpose()?;
            let target = target
                .as_ref()
                .map(|asset| {
                    asset
                        .value()
                        .animated
                        .as_ref()
                        .ok_or("retarget target has no skeletal model")
                })
                .transpose()?;
            if let (Some(source), Some(target)) = (source, target) {
                profile.compile_models(source, target)?;
            }
        }
        for (node, animation) in loaded.graph.components::<ModelAnimation>() {
            let model = loaded
                .graph
                .component::<String>(node)?
                .ok_or("animation requires a model owner")?;
            let model = loaded
                .graph
                .component::<ModelRetarget>(node)?
                .map_or(model.as_str(), |profile| profile.source.as_str());
            // Scene loading may precede asynchronous resource publication.
            // Runtime admission validates again against the published revision.
            let counts = self
                .catalog
                .snapshot(&AssetId(model.to_owned()))
                .map(|asset| {
                    asset.value().animated.as_ref().map_or((0, 0), |model| {
                        (model.animations.len(), model.skeleton.joints().len())
                    })
                });
            animation.validate(counts.map(|value| value.0), counts.map(|value| value.1))?;
            if (animation.root_motion_rotation
                || animation.root_motion_axes.into_iter().any(|axis| axis))
                && loaded.graph.component::<CharacterBody>(node)?.is_none()
            {
                return Err("root motion requires a CharacterBody on the model owner".into());
            }
            if let Some(asset) = self.catalog.snapshot(&AssetId(model.to_owned()))
                && let Some(model) = asset.value().animated.as_ref()
            {
                animation.resolve_clip(model)?;
                animation.resolve_motion_joint(model)?;
            }
        }
        for (node, controller) in loaded.graph.components::<ModelLocomotion>() {
            controller.validate()?;
            loaded
                .graph
                .component::<ModelAnimation>(node)?
                .ok_or("locomotion requires a model animation component")?;
            locomotion::body_owner(&loaded.graph, node)?;
            if loaded.graph.component::<ModelPart>(node)?.is_some() {
                return Err("locomotion requires a logical model owner".into());
            }
            let asset = loaded
                .graph
                .component::<String>(node)?
                .ok_or("locomotion requires a model owner")?;
            let source = loaded
                .graph
                .component::<ModelRetarget>(node)?
                .map_or(asset.as_str(), |profile| profile.source.as_str());
            if let Some(asset) = self.catalog.snapshot(&AssetId(source.into())) {
                let model = asset
                    .value()
                    .animated
                    .as_ref()
                    .ok_or("locomotion requires an animated model")?;
                controller.validate_model(model)?;
            }
        }
        for (node, settings) in loaded.graph.components::<ModelFootPlacement>() {
            settings.validate()?;
            if settings.feet.is_empty() {
                continue;
            }
            let model = loaded
                .graph
                .component::<String>(node)?
                .ok_or("foot placement requires a model owner")?;
            if loaded.graph.component::<CharacterBody>(node)?.is_none() {
                return Err("foot placement requires a CharacterBody on the model owner".into());
            }
            if let Some(asset) = self.catalog.snapshot(&AssetId(model.clone()))
                && let Some(model) = asset.value().animated.as_ref()
            {
                let source_asset = loaded
                    .graph
                    .component::<ModelRetarget>(node)?
                    .map(|profile| self.catalog.snapshot(&AssetId(profile.source.clone())));
                let animation_model = if let Some(source) = &source_asset {
                    source
                        .as_ref()
                        .and_then(|source| source.value().animated.as_ref())
                } else {
                    Some(model)
                };
                let Some(animation_model) = animation_model else {
                    continue;
                };
                foot_placement::FootRuntime::new_with_clips(
                    model,
                    settings.clone(),
                    &animation_model.animations,
                )?;
                let animation = loaded
                    .graph
                    .component::<ModelAnimation>(node)?
                    .cloned()
                    .unwrap_or_default();
                let clip_name = animation.resolve_clip(animation_model)?.and_then(|index| {
                    animation_model
                        .animations
                        .get(index)
                        .map(|clip| clip.name())
                });
                for foot in &settings.feet {
                    foot.contact_keys(clip_name)?;
                }
            }
        }
        for (node, part) in loaded.graph.components::<ModelPart>() {
            if part.node != u32::MAX
                && let Some(model) = loaded.graph.component::<String>(node)?
                && let Some(asset) = self.catalog.snapshot(&AssetId(model.clone()))
                && part.node as usize >= asset.value().nodes.len()
            {
                return Err("invalid imported model part".into());
            }
        }

        Ok(next)
    }

    pub(crate) fn load_authoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.load_authoring_policy(false)
    }
    fn load_authoring_policy(
        &mut self,
        allow_unresolved_models: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.finish_drag(false)?;
        let path = self
            .authoring
            .scene_path
            .as_ref()
            .ok_or("start editor with --scene PATH to load")?;
        let (document, imported, revision) = match self.authoring.authoring_project.load(path) {
            Ok(imported) => (imported.value().expanded.clone(), Some(imported), None),
            Err(error) => {
                // Existing standalone flat scene paths can be outside the model
                // project. Composed scenes require observed project dependencies.
                let (document, revision) =
                    scene_revision::SceneRevision::read(path).map_err(|_| error)?;
                (document, None, Some(revision))
            }
        };
        let next = self.validate_authoring_document_policy(&document, allow_unresolved_models)?;

        self.authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit_with_metadata(
                document,
                imported
                    .as_ref()
                    .filter(|value| !value.value().source.instances.is_empty())
                    .map(|value| serde_json::to_value(value.value()))
                    .transpose()?
                    .unwrap_or(serde_json::Value::Null),
                &self.authoring.authoring_project.registry,
            )?;
        self.restore_authoring()?;
        self.authoring.next_object_id = next;
        self.authoring.authoring_source = imported;
        self.authoring.scene_revision = revision;
        if let Some(window) = &self.window {
            window.set_title("Voxy — scene loaded");
        }
        Ok(())
    }

    pub(crate) fn history_key(&mut self, key: KeyCode) -> Result<(), Box<dyn std::error::Error>> {
        let changed = self.authoring.step_history(key == KeyCode::KeyZ)?;
        if changed {
            self.restore_authoring()?;
        }
        Ok(())
    }

    pub(crate) fn verify_scene_round_trip(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.authoring.scene_path.is_some() {
            let expected = self.authoring_document()?;
            self.save_authoring()?;
            self.edit_key(KeyCode::Delete)?;
            self.load_authoring()?;
            if self.authoring_document()? != expected {
                return Err("native scene round trip changed authoring data".into());
            }
            println!(
                "MODEL SCENE PASS: saved file restored IDs, transforms and resource references"
            );
        }
        Ok(())
    }

    pub(crate) fn configure_scene(
        &mut self,
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.configure_scene_policy(path, false)
    }
    pub(crate) fn configure_scene_for_editor(
        &mut self,
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.configure_scene_policy(path, true)
    }
    fn configure_scene_policy(
        &mut self,
        path: &std::path::Path,
        allow_unresolved_models: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.authoring.scene_path = Some(path.into());
        if path.try_exists()? {
            self.load_authoring_policy(allow_unresolved_models)?;
            let metadata = self
                .authoring
                .history
                .as_ref()
                .ok_or("missing history")?
                .metadata()
                .clone();
            self.authoring.history = Some(SceneHistory::new_with_metadata(
                self.authoring_document()?,
                metadata,
                &self.authoring.authoring_project.registry,
                scene_limits::OBJECTS,
                64,
                scene_limits::HISTORY_BYTES,
            )?);
        }
        Ok(())
    }

    pub(crate) fn commit_authoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let result = (|| {
            CharacterPhysics::new(&self.scene, 128, 128).validate(&self.scene)?;
            let document = self.authoring_document()?;
            self.validate_authoring_document(&document)?;
            self.authoring
                .history
                .as_mut()
                .ok_or("missing history")?
                .commit(document, &self.authoring.authoring_project.registry)?;
            Ok(())
        })();
        if result.is_err() {
            self.restore_authoring()?;
        }
        result
    }
}
