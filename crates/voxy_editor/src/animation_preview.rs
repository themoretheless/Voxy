//! Authoring-only immutable pose preview; no game clock or event queue.
use std::sync::Arc;
use voxy_render::ModelAsset;
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
impl crate::App {
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
        let pose = source.sample_pose_phase(settings.resolve_clip(&source)?, phase)?;
        let frame = voxy_animation::AnimatorFrame {
            skin_matrices: pose.skin_matrices(&source.skeleton)?,
            pose,
            root_motion: glam::Vec3::ZERO,
            root_motion_joint: settings.resolve_motion_joint(&source)?,
            transition_weight: 1.,
        };
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
        while app.catalog.snapshot(&app.id).is_none() || app.catalog.snapshot(&source_id).is_none()
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
            .snapshot(&app.id)
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
        while app.catalog.snapshot(&app.id).is_none() {
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
            .snapshot(&app.id)
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
