//! Authoring-only immutable pose preview; no game clock or event queue.
#[allow(dead_code)]
pub(crate) mod checkpoint;
use std::sync::Arc;
use voxy_model::ModelAsset;
use voxy_scene::NodeId;
#[derive(Debug)]
pub(super) struct Preview {
    pub owner: NodeId,
    pub phase: f64,
    model: Arc<ModelAsset>,
    source: Arc<ModelAsset>,
    settings: crate::ModelAnimation,
    profile: Option<crate::ModelRetarget>,
    frame: Arc<voxy_animation::AnimatorFrame>,
}
/// Native preview owner observation; immutable resources stay alive for validation.
#[derive(Clone)]
pub(crate) struct PreviewObservation {
    owner: NodeId,
    phase: u64,
    model: Arc<ModelAsset>,
    source: Arc<ModelAsset>,
    frame: Arc<voxy_animation::AnimatorFrame>,
}
impl PartialEq for PreviewObservation {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
            && self.phase == other.phase
            && Arc::ptr_eq(&self.model, &other.model)
            && Arc::ptr_eq(&self.source, &other.source)
            && Arc::ptr_eq(&self.frame, &other.frame)
    }
}
impl Eq for PreviewObservation {}
impl Preview {
    pub(crate) fn observation(&self) -> PreviewObservation {
        // Settings/profile are private and are rebuilt together with this frame.
        let Self {
            owner,
            phase,
            model,
            source,
            frame,
            settings: _,
            profile: _,
        } = self;
        PreviewObservation {
            owner: *owner,
            phase: phase.to_bits(),
            model: model.clone(),
            source: source.clone(),
            frame: frame.clone(),
        }
    }
}
impl crate::App {
    pub(super) fn edit_morph_defaults(
        &mut self,
        token: [u8; 32],
        resume: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_some() || self.retarget_draft.is_some() {
            return Err("finish play or retarget editing before editing morph weights".into());
        }
        let document = self.panel_document()?;
        let object = document
            .objects
            .get(self.selected)
            .ok_or("missing morph owner")?;
        if crate::component_fields::morph_edit_token(object) != token {
            return Err("morph owner changed during editing".into());
        }
        let owner = *self
            .instances
            .get(self.selected)
            .ok_or("missing morph owner")?;
        let mut settings = self
            .scene
            .component::<crate::ModelAnimation>(owner)?
            .cloned()
            .ok_or("missing animation component")?;
        if resume {
            settings.morph_weights.clear();
        } else {
            let instance = self
                .scene
                .component::<crate::ModelInstance>(owner)?
                .ok_or("missing model instance")?;
            let asset = self
                .catalog
                .snapshot(&instance.asset)
                .ok_or("model is not loaded")?;
            let model = asset
                .value()
                .animated
                .as_ref()
                .ok_or("missing animated model")?;
            let nodes = model.morph_nodes();
            if nodes.is_empty() {
                return Err("model has no morph targets".into());
            }
            let mut weights = std::collections::BTreeMap::new();
            for node in nodes {
                let name = node.name.ok_or("morph node needs a unique authored name")?;
                if model.resolve_morph_node_name(&name)? != node.source_node {
                    return Err("morph node identity changed".into());
                }
                weights.insert(name.to_string(), node.weights);
            }
            settings.morph_weights = weights;
            let mut frame = model.bind_frame()?;
            settings.apply_morph_weights(model, &mut frame)?;
        }
        self.scene.insert_component(owner, settings)?;
        self.commit_authoring()?;
        self.field = None;
        Ok(())
    }

    pub(super) fn preview_animation_marker(
        &mut self,
        index: usize,
        token: [u8; 32],
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_some() {
            return Err("stop play before previewing animation".into());
        }
        let document = self.panel_document()?;
        let object = document
            .objects
            .get(self.selected)
            .ok_or("missing preview owner")?;
        if crate::component_fields::marker_list_token(object)? != token {
            return Err("animation markers changed".into());
        }
        let owner = *self
            .instances
            .get(self.selected)
            .ok_or("missing preview owner")?;
        let settings = self
            .scene
            .component::<crate::ModelAnimation>(owner)?
            .cloned()
            .ok_or("missing animation")?;
        let phase = settings
            .events
            .get(index)
            .ok_or("missing preview marker")?
            .phase;
        self.preview_animation_phase(phase, true)
    }
    pub(super) fn preview_animation_phase(
        &mut self,
        phase: f64,
        toggle: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_some() || self.retarget_draft.is_some() {
            return Err("finish play or retarget editing before previewing animation".into());
        }
        if !phase.is_finite() || !(0.0..=1.0).contains(&phase) {
            return Err("invalid preview phase".into());
        }
        let owner = *self
            .instances
            .get(self.selected)
            .ok_or("missing preview owner")?;
        let settings = self
            .scene
            .component::<crate::ModelAnimation>(owner)?
            .cloned()
            .ok_or("missing animation")?;
        let asset = &self
            .scene
            .component::<crate::ModelInstance>(owner)?
            .ok_or("missing model")?
            .asset;
        let model = self
            .catalog
            .snapshot(asset)
            .and_then(|asset| asset.value().animated.clone())
            .ok_or("preview model is not loaded")?;
        if toggle
            && self
                .authoring
                .animation_preview
                .as_ref()
                .is_some_and(|preview| preview.owner == owner && preview.phase == phase)
            && self
                .preview_animation_frame(owner, &model, &settings)
                .is_some()
        {
            self.authoring.animation_preview = None;
            self.panel_cache = None;
            return Ok(());
        }
        let profile = self
            .scene
            .component::<crate::ModelRetarget>(owner)?
            .cloned();
        let source = if let Some(profile) = &profile {
            self.catalog
                .snapshot(&voxy_assets::AssetId(profile.source.clone()))
                .and_then(|asset| asset.value().animated.clone())
                .ok_or("preview source is not loaded")?
        } else {
            model.clone()
        };
        let binding = profile
            .as_ref()
            .map(|profile| profile.compile_models(&source, &model))
            .transpose()?;
        settings.validate(
            Some(source.animations.len()),
            Some(source.skeleton.joints().len()),
        )?;
        let mut frame = source.sample_frame_phase(settings.resolve_clip(&source)?, phase)?;
        frame.root_motion_joint = settings.resolve_motion_joint(&source)?;
        let (frame, _) = crate::animation_runtime::prepare_displayed_frame(
            frame,
            &model,
            &settings,
            binding.as_ref(),
        )?;
        self.panel_cache = None;
        self.authoring.animation_preview = Some(Preview {
            owner,
            phase,
            model,
            source,
            settings,
            profile,
            frame: Arc::new(frame),
        });
        Ok(())
    }
    pub(super) fn continue_preview_seek(
        &mut self,
        cursor: glam::Vec2,
        scale: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some((owner, rect)) = self.authoring.preview_seek else {
            return Ok(());
        };
        if !cursor.is_finite() || !scale.is_finite() || scale <= 0. || rect[2] <= 0. {
            self.authoring.preview_seek = None;
            return Err("invalid preview pointer coordinates".into());
        }
        let valid = self.instances.get(self.selected) == Some(&owner)
            && self.play.playing.is_none()
            && self.retarget_draft.is_none()
            && matches!(self.inspector, crate::InspectorMode::Components(_))
            && self
                .authoring
                .animation_preview
                .as_ref()
                .is_some_and(|preview| {
                    let current = self
                        .scene
                        .component::<crate::ModelInstance>(owner)
                        .ok()
                        .flatten()
                        .and_then(|instance| self.catalog.snapshot(&instance.asset))
                        .and_then(|asset| asset.value().animated.clone());
                    if current
                        .as_ref()
                        .is_none_or(|model| !Arc::ptr_eq(model, &preview.model))
                    {
                        return false;
                    }
                    self.scene
                        .component::<crate::ModelAnimation>(owner)
                        .ok()
                        .flatten()
                        .is_some_and(|settings| {
                            self.preview_animation_frame(owner, &preview.model, settings)
                                .is_some()
                        })
                });
        if !valid {
            self.authoring.preview_seek = None;
            return Ok(());
        }
        let phase = f64::from(((cursor.x / scale - rect[0]) / rect[2]).clamp(0., 1.));
        if let Err(error) = self.preview_animation_phase(phase, false) {
            self.authoring.preview_seek = None;
            return Err(error);
        }
        Ok(())
    }
    pub(super) fn preview_animation_frame(
        &self,
        owner: NodeId,
        model: &Arc<ModelAsset>,
        settings: &crate::ModelAnimation,
    ) -> Option<Arc<voxy_animation::AnimatorFrame>> {
        let preview = self.authoring.animation_preview.as_ref()?;
        if preview.owner != owner
            || self.instances.get(self.selected) != Some(&owner)
            || !Arc::ptr_eq(&preview.model, model)
            || &preview.settings != settings
        {
            return None;
        }
        let profile = self
            .scene
            .component::<crate::ModelRetarget>(owner)
            .ok()?
            .cloned();
        if profile != preview.profile {
            return None;
        }
        if let Some(profile) = profile {
            let source = self
                .catalog
                .snapshot(&voxy_assets::AssetId(profile.source))?;
            if !Arc::ptr_eq(source.value().animated.as_ref()?, &preview.source) {
                return None;
            }
        }
        Some(preview.frame.clone())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn prepared_source_guard_observes_preview_frame_identity_and_exact_phase() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/animated-triangle.glb");
        let mut app = crate::App::new(&path, false).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app.catalog.snapshot(app.id.as_ref().unwrap()).is_none() {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        app.panel_action(crate::panels::Action::Animation).unwrap();
        let scene = app.scene.change_token();
        let clock = app.play.animations.serial();
        let guard = crate::app::restore_guard::SceneAuthoringGuard::capture(&app);
        app.preview_animation_phase(0.25, false).unwrap();
        assert!(guard.validate(&app).is_err());
        let frame_json = |app: &crate::App| {
            let preview = app.authoring.animation_preview.as_ref().unwrap();
            preview
                .frame
                .checkpoint(&preview.model.skeleton, 8 * 1024 * 1024)
                .unwrap()
                .to_json(8 * 1024 * 1024)
                .unwrap()
        };
        let before = frame_json(&app);
        let preview = app.authoring.animation_preview.as_ref().unwrap();
        let view = voxy_scene::SceneCheckpointView::new(
            &app.scene,
            app.object_ids
                .iter()
                .cloned()
                .zip(app.instances.iter().copied()),
        )
        .unwrap();
        let model = preview.model.clone();
        let saved = preview
            .checkpoint(
                &view,
                |root| std::sync::Arc::ptr_eq(root, &model).then(|| "model-0".into()),
                8 * 1024 * 1024,
            )
            .unwrap();
        let json = saved.to_json(8 * 1024 * 1024).unwrap();
        let decoded =
            super::checkpoint::PreviewCheckpoint::from_json(&json, 8 * 1024 * 1024).unwrap();
        let mut target = voxy_scene::SceneGraph::new(4);
        let owner = target.spawn(None, Default::default()).unwrap();
        let target_view =
            voxy_scene::SceneCheckpointView::new(&target, [(app.object_ids[0].clone(), owner)])
                .unwrap();
        assert!(
            decoded
                .prepare(&target_view, |_| None, 8 * 1024 * 1024)
                .is_err()
        );
        let limits = crate::import::asset_checkpoint::EditorAssetCheckpointLimits::default();
        let roots = preview.model_roots();
        assert_eq!(roots.len(), 1);
        let graph = crate::import::asset_checkpoint::EditorAssetCheckpoint::capture_with_models(
            &[],
            &roots,
            limits,
        )
        .unwrap();
        let bytes = graph.to_bytes(limits).unwrap();
        let graph =
            crate::import::asset_checkpoint::EditorAssetCheckpoint::from_bytes(&bytes, limits)
                .unwrap();
        let restored_models = graph.prepare_with_models(limits).unwrap();
        assert_eq!(restored_models.models.len(), 1);
        let fresh_model = restored_models.models[0].clone();
        assert!(!std::sync::Arc::ptr_eq(&fresh_model, &model));
        if let Some(directory) = std::env::var_os("VOXY_PREVIEW_RESOURCE_EVIDENCE_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(directory.join("model-graph.voxyasset"), &bytes).unwrap();
            std::fs::write(directory.join("preview.json"), &json).unwrap();
        }
        let restored = decoded
            .prepare(
                &target_view,
                |key| (key == "model-0").then(|| fresh_model.clone()),
                8 * 1024 * 1024,
            )
            .unwrap();
        let mut bad_alias: serde_json::Value = serde_json::from_str(&json).unwrap();
        bad_alias["source"] = "model-1".into();
        let bad_alias = super::checkpoint::PreviewCheckpoint::from_json(
            &bad_alias.to_string(),
            8 * 1024 * 1024,
        )
        .unwrap();
        assert!(
            bad_alias
                .prepare(&target_view, |_| Some(model.clone()), 8 * 1024 * 1024)
                .is_err()
        );
        assert_ne!(restored.owner, preview.owner);
        assert_eq!(restored.phase.to_bits(), preview.phase.to_bits());
        assert!(std::sync::Arc::ptr_eq(&restored.model, &restored.source));
        assert_eq!(
            before,
            restored
                .frame
                .checkpoint(&restored.model.skeleton, 8 * 1024 * 1024)
                .unwrap()
                .to_json(8 * 1024 * 1024)
                .unwrap()
        );
        let guard = crate::app::restore_guard::SceneAuthoringGuard::capture(&app);
        app.preview_animation_phase(0.25, false).unwrap();
        assert_eq!(before, frame_json(&app));
        assert!(guard.validate(&app).is_err());
        app.preview_animation_phase(0., false).unwrap();
        let guard = crate::app::restore_guard::SceneAuthoringGuard::capture(&app);
        app.authoring.animation_preview.as_mut().unwrap().phase = -0.;
        assert!(guard.validate(&app).is_err());
        let guard = crate::app::restore_guard::SceneAuthoringGuard::capture(&app);
        app.authoring.animation_preview = None;
        assert!(guard.validate(&app).is_err());
        assert_eq!(scene, app.scene.change_token());
        assert_eq!(clock, app.play.animations.serial());
        assert!(app.play.animations.take_events().is_empty());
        app.stop_workers().unwrap();
    }

    #[test]
    fn retarget_preview_uses_source_phase_target_bind_and_in_place_axes() {
        let directory = std::env::temp_dir().join(format!(
            "voxy-retarget-preview-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let source_bytes =
            include_bytes!("../../voxy_render/examples/assets/animated-triangle.glb");
        std::fs::write(directory.join("source.glb"), source_bytes).unwrap();
        let glb = gltf::binary::Glb::from_slice(source_bytes).unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&glb.json).unwrap();
        json["nodes"][0]["name"] = serde_json::json!("pelvis");
        json["nodes"][0]["translation"] = serde_json::json!([3., 4., 0.]);
        json["animations"] = serde_json::json!([]);
        let target_bytes = gltf::binary::Glb {
            header: glb.header,
            json: serde_json::to_vec(&json).unwrap().into(),
            bin: glb.bin,
        }
        .to_vec()
        .unwrap();
        std::fs::write(directory.join("target.glb"), target_bytes).unwrap();
        let mut app = crate::App::new(&directory.join("target.glb"), false).unwrap();
        let owner = app.instances[0];
        app.scene
            .insert_component(
                owner,
                crate::ModelRetarget {
                    source: "source.glb".into(),
                    joints: vec![crate::RetargetJointProfile {
                        source: "root".into(),
                        target: "pelvis".into(),
                        rotation_basis: glam::Quat::IDENTITY.to_array(),
                        translation_basis: glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
                            .to_array(),
                        translation_scale: 2.,
                    }],
                },
            )
            .unwrap();
        app.scene
            .insert_component(
                owner,
                crate::ModelAnimation {
                    clip_name: "move".into(),
                    events: vec![crate::ModelAnimationEvent {
                        name: "middle".into(),
                        phase: 0.5,
                    }],
                    ..Default::default()
                },
            )
            .unwrap();
        let source_id = voxy_assets::AssetId("source.glb".into());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app
            .catalog
            .snapshot(app.id.as_ref().expect("model-only fixture"))
            .is_none()
            || app.catalog.snapshot(&source_id).is_none()
        {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let document = app.authoring_document().unwrap();
        let token = crate::component_fields::marker_list_token(&document.objects[0]).unwrap();
        app.preview_animation_marker(0, token).unwrap();
        let model = app
            .catalog
            .snapshot(app.id.as_ref().expect("model-only fixture"))
            .unwrap()
            .value()
            .animated
            .as_ref()
            .unwrap()
            .clone();
        let settings = app
            .scene
            .component::<crate::ModelAnimation>(owner)
            .unwrap()
            .unwrap()
            .clone();
        let frame = app
            .preview_animation_frame(owner, &model, &settings)
            .unwrap();
        assert!((frame.pose.local()[0].translation - glam::Vec3::new(3., 6., 0.)).length() < 1e-5);
        let expected = glam::Mat4::from_translation(glam::Vec3::new(3., 6., 0.))
            * model.skeleton.joints()[0].inverse_bind;
        for (actual, expected) in frame.skin_matrices[0]
            .to_cols_array()
            .into_iter()
            .zip(expected.to_cols_array())
        {
            assert!((actual - expected).abs() < 1e-5);
        }
        assert_eq!(app.authoring_document().unwrap(), document);
        app.scene
            .component_mut::<crate::ModelAnimation>(owner)
            .unwrap()
            .unwrap()
            .root_motion_axes = [false, true, false];
        let settings = app
            .scene
            .component::<crate::ModelAnimation>(owner)
            .unwrap()
            .unwrap()
            .clone();
        assert!(
            app.preview_animation_frame(owner, &model, &settings)
                .is_none()
        );
        // Same marker phase with changed settings must rebuild, not toggle the stale pose off.
        app.preview_animation_marker(0, token).unwrap();
        let settings = app
            .scene
            .component::<crate::ModelAnimation>(owner)
            .unwrap()
            .unwrap()
            .clone();
        let frame = app
            .preview_animation_frame(owner, &model, &settings)
            .unwrap();
        assert!((frame.pose.local()[0].translation - glam::Vec3::new(3., 4., 0.)).length() < 1e-5);
        let preview = app.authoring.animation_preview.as_ref().unwrap();
        let roots = preview.model_roots();
        assert_eq!(roots.len(), 2);
        let view = voxy_scene::SceneCheckpointView::new(
            &app.scene,
            app.object_ids
                .iter()
                .cloned()
                .zip(app.instances.iter().copied()),
        )
        .unwrap();
        let saved = preview
            .checkpoint(
                &view,
                |root| {
                    roots
                        .iter()
                        .position(|candidate| std::sync::Arc::ptr_eq(candidate, root))
                        .map(|i| format!("model:{i}"))
                },
                8 * 1024 * 1024,
            )
            .unwrap();
        let limits = crate::import::asset_checkpoint::EditorAssetCheckpointLimits::default();
        let graph = crate::import::asset_checkpoint::EditorAssetCheckpoint::capture_with_models(
            &[],
            &roots,
            limits,
        )
        .unwrap();
        let bytes = graph.to_bytes(limits).unwrap();
        let graph =
            crate::import::asset_checkpoint::EditorAssetCheckpoint::from_bytes(&bytes, limits)
                .unwrap();
        let models = graph.prepare_with_models(limits).unwrap().models;
        assert!(!std::sync::Arc::ptr_eq(&models[0], &roots[0]));
        assert!(!std::sync::Arc::ptr_eq(&models[1], &roots[1]));
        let restored = saved
            .prepare(
                &view,
                |key| {
                    models
                        .get(key.strip_prefix("model:")?.parse::<usize>().ok()?)
                        .cloned()
                },
                8 * 1024 * 1024,
            )
            .unwrap();
        assert!(!std::sync::Arc::ptr_eq(&restored.model, &restored.source));
        assert_eq!(
            preview
                .frame
                .checkpoint(&preview.model.skeleton, 8 * 1024 * 1024)
                .unwrap()
                .to_json(8 * 1024 * 1024)
                .unwrap(),
            restored
                .frame
                .checkpoint(&restored.model.skeleton, 8 * 1024 * 1024)
                .unwrap()
                .to_json(8 * 1024 * 1024)
                .unwrap()
        );
        app.begin_model_checkpoint().unwrap();
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !app.poll_model_checkpoint().unwrap() {
            assert!(std::time::Instant::now() < until);
            std::thread::yield_now();
        }
        let policy = crate::play_session::checkpoint::tests::resource_restore_limits();
        let resource_file = app
            .capture_model_resources(policy.catalog)
            .unwrap()
            .to_bytes(policy.max_bytes)
            .unwrap();
        let empty = directory.join("empty-target");
        std::fs::create_dir(&empty).unwrap();
        let resources = crate::app::resource_checkpoint::ModelResourcesCheckpoint::from_bytes(
            &resource_file,
            policy.max_bytes,
        )
        .unwrap();
        let mut resources = resources.prepare(&empty, policy).unwrap();
        assert_eq!(resources.preview_models.len(), 2);
        assert!(!std::sync::Arc::ptr_eq(
            &resources.preview_models[0],
            &resources.preview_models[1]
        ));
        for (index, id) in [app.id.as_ref().unwrap(), &source_id]
            .into_iter()
            .enumerate()
        {
            let publication = resources.resources.catalog.snapshot(id).unwrap();
            assert!(std::sync::Arc::ptr_eq(
                &resources.preview_models[index],
                publication.value().animated.as_ref().unwrap()
            ));
            assert!(!std::sync::Arc::ptr_eq(
                &resources.preview_models[index],
                &roots[index]
            ));
        }
        if let Some(worker) = resources.imports.take() {
            worker.close().join().unwrap();
        }
        if let Some(worker) = resources.watcher.take() {
            worker.close().join().unwrap();
        }
        assert!(app.resume_model_checkpoint().unwrap());
        let accepted = app
            .authoring
            .animation_preview
            .as_ref()
            .unwrap()
            .frame
            .clone();
        let profile = app
            .scene
            .component::<crate::ModelRetarget>(owner)
            .unwrap()
            .unwrap()
            .clone();
        app.scene
            .component_mut::<crate::ModelRetarget>(owner)
            .unwrap()
            .unwrap()
            .joints[0]
            .target = "missing".into();
        assert!(
            app.preview_animation_frame(owner, &model, &settings)
                .is_none()
        );
        assert!(app.preview_animation_marker(0, token).is_err());
        assert!(std::sync::Arc::ptr_eq(
            &accepted,
            &app.authoring.animation_preview.as_ref().unwrap().frame
        ));
        app.scene.insert_component(owner, profile).unwrap();
        assert!(std::sync::Arc::ptr_eq(
            &accepted,
            &app.preview_animation_frame(owner, &model, &settings)
                .unwrap()
        ));
        assert_eq!(app.play.animations.serial(), 0);
        assert!(app.play.animations.take_events().is_empty());
        app.stop_workers().unwrap();
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn marker_preview_is_authoring_only_toggleable_and_rejects_stale_configuration() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/animated-triangle.glb");
        let mut app = crate::App::new(&path, false).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while app
            .catalog
            .snapshot(app.id.as_ref().expect("model-only fixture"))
            .is_none()
        {
            app.tick().unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        app.panel_action(crate::panels::Action::Animation).unwrap();
        let document = app.authoring_document().unwrap();
        let token = crate::component_fields::marker_list_token(&document.objects[0]).unwrap();
        app.panel_action(crate::panels::Action::MarkerAdd(token))
            .unwrap();
        let document = app.authoring_document().unwrap();
        let token = crate::component_fields::marker_list_token(&document.objects[0]).unwrap();
        app.panel_action(crate::panels::Action::MarkerPreview(0, token))
            .unwrap();
        assert_eq!(app.authoring_document().unwrap(), document);
        let owner = app.instances[0];
        let model = app
            .catalog
            .snapshot(app.id.as_ref().expect("model-only fixture"))
            .unwrap()
            .value()
            .animated
            .as_ref()
            .unwrap()
            .clone();
        let settings = app
            .scene
            .component::<crate::ModelAnimation>(owner)
            .unwrap()
            .unwrap()
            .clone();
        let frame = app
            .preview_animation_frame(owner, &model, &settings)
            .unwrap();
        assert_eq!(frame.pose, model.sample_pose_phase(Some(0), 0.5).unwrap());
        assert_eq!(app.play.animations.serial(), 0);
        assert!(app.play.animations.take_events().is_empty());
        let mut changed = settings.clone();
        changed.clip = None;
        assert!(
            app.preview_animation_frame(owner, &model, &changed)
                .is_none()
        );
        assert!(app.preview_animation_marker(0, [0; 32]).is_err());
        assert!(
            app.preview_animation_frame(owner, &model, &settings)
                .is_some()
        );
        app.panel_action(crate::panels::Action::MarkerPreview(0, token))
            .unwrap();
        assert!(app.authoring.animation_preview.is_none());
        app.panel_action(crate::panels::Action::MarkerPreview(0, token))
            .unwrap();
        app.toggle_play().unwrap();
        assert!(app.authoring.animation_preview.is_none());
        assert!(app.preview_animation_marker(0, token).is_err());
        app.toggle_play().unwrap();
        assert_eq!(app.authoring_document().unwrap(), document);
        let owner = app.instances[0];
        app.scene
            .component_mut::<crate::ModelAnimation>(owner)
            .unwrap()
            .unwrap()
            .events
            .clear();
        let document = app.authoring_document().unwrap();
        let settings = app
            .scene
            .component::<crate::ModelAnimation>(owner)
            .unwrap()
            .unwrap()
            .clone();
        app.inspector = crate::InspectorMode::Components(0);
        let mut panels = crate::panels::Panels::new().unwrap();
        panels
            .build(
                &document,
                0,
                glam::Vec2::new(1024., 640.),
                false,
                None,
                0,
                false,
                app.inspector,
                "Texture: none",
                "Preview",
                None,
            )
            .unwrap();
        panels.frame_outcome(voxy_render::RenderOutcome::Presented);
        let rect = panels
            .regions
            .iter()
            .find(|(_, action)| *action == crate::panels::Action::PreviewSeek)
            .unwrap()
            .0;
        app.panels = Some(panels);
        for fraction in [0.0_f32, 0.123456, 0.5, 0.99999] {
            let point = glam::Vec2::new(rect[0] + rect[2] * fraction, rect[1] + rect[3] * 0.5);
            let action = app.panel_target(point).unwrap().unwrap();
            let crate::panels::Action::PreviewPhase(phase) = action else {
                panic!("seek action expected");
            };
            assert!((phase - f64::from(fraction)).abs() < 1e-6);
            app.panel_action(action).unwrap();
        }
        for phase in [0., 0.123456, 0.5, 1., 1.] {
            app.panel_action(crate::panels::Action::PreviewPhase(phase))
                .unwrap();
            let frame = app
                .preview_animation_frame(owner, &model, &settings)
                .unwrap();
            assert_eq!(frame.pose, model.sample_pose_phase(Some(0), phase).unwrap());
            assert_eq!(app.authoring_document().unwrap(), document);
        }
        for (scale, fraction) in [(1.0_f32, -0.2_f32), (2., 0.123456), (1.5, 1.2)] {
            app.authoring.preview_seek = Some((owner, rect));
            let cursor = glam::Vec2::new((rect[0] + rect[2] * fraction) * scale, rect[1] * scale);
            app.continue_preview_seek(cursor, scale).unwrap();
            assert!(
                (app.authoring.animation_preview.as_ref().unwrap().phase
                    - f64::from(fraction.clamp(0., 1.)))
                .abs()
                    < 1e-6
            );
            assert_eq!(app.authoring_document().unwrap(), document);
        }
        let phase = app.authoring.animation_preview.as_ref().unwrap().phase;
        app.authoring.preview_seek = Some((owner, rect));
        app.scene
            .component_mut::<crate::ModelAnimation>(owner)
            .unwrap()
            .unwrap()
            .clip = None;
        app.continue_preview_seek(glam::Vec2::ZERO, 2.).unwrap();
        assert!(app.authoring.preview_seek.is_none());
        assert_eq!(
            app.authoring.animation_preview.as_ref().unwrap().phase,
            phase
        );
        app.scene.insert_component(owner, settings).unwrap();
        app.authoring.preview_seek = Some((owner, rect));
        app.continue_preview_seek(glam::Vec2::splat(f32::NAN), 2.)
            .unwrap_err();
        assert!(app.authoring.preview_seek.is_none());
        let accepted = app
            .authoring
            .animation_preview
            .as_ref()
            .unwrap()
            .frame
            .clone();
        assert!(app.preview_animation_phase(f64::NAN, false).is_err());
        assert!(std::sync::Arc::ptr_eq(
            &accepted,
            &app.authoring.animation_preview.as_ref().unwrap().frame
        ));
        app.stop_workers().unwrap();
    }
}
