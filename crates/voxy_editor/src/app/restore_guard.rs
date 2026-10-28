//! Native scene/history/draft observations for the enclosing restore transaction.
//! Model clocks/publications have their own observation; UI/audio/host remain separate.
#![allow(dead_code)]
use crate::*;
#[derive(PartialEq, Eq)]
struct DragGuard {
    node: NodeId,
    coordinates: [[u32; 2]; 3],
    original: ([u32; 3], [u32; 4], [u32; 3]),
    world: [u32; 3],
    axis: u8,
    edit: voxy_scene::HistoryChangeToken,
}
struct ProjectGuard {
    identity: Arc<()>,
    registry: Arc<ComponentRegistry>,
}
impl PartialEq for ProjectGuard {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.identity, &other.identity) && Arc::ptr_eq(&self.registry, &other.registry)
    }
}
impl Eq for ProjectGuard {}
#[derive(PartialEq, Eq)]
pub(crate) struct SceneAuthoringGuard {
    scene: voxy_scene::SceneChangeToken,
    project: ProjectGuard,
    history: Option<voxy_scene::HistoryChangeToken>,
    drag: Option<DragGuard>,
    objects: Vec<ObjectId>,
    instances: Vec<NodeId>,
    selected: usize,
    next_object_id: u64,
    settings_written: BTreeMap<String, [u8; 32]>,
    scene_path: Option<std::path::PathBuf>,
    scene_revision: Option<crate::scene_revision::SceneRevision>,
    prefab_assets: Vec<AssetId>,
    prefab_choice: usize,
    preview_seek: Option<(NodeId, [u32; 4])>,
    preview: Option<crate::animation_preview::PreviewObservation>,
}
impl SceneAuthoringGuard {
    pub(crate) fn capture(app: &App) -> Self {
        Self {
            scene: app.scene.change_token(),
            preview: app
                .authoring
                .animation_preview
                .as_ref()
                .map(crate::animation_preview::Preview::observation),
            project: ProjectGuard {
                identity: app.authoring.authoring_project.observation_token(),
                registry: app.authoring.authoring_project.registry.clone(),
            },
            history: app
                .authoring
                .history
                .as_ref()
                .map(SceneHistory::change_token),
            drag: app.drag.as_ref().map(|drag| DragGuard {
                node: drag.node,
                coordinates: [
                    drag.origin.to_array().map(f32::to_bits),
                    drag.viewport.to_array().map(f32::to_bits),
                    drag.viewport_origin.to_array().map(f32::to_bits),
                ],
                original: (
                    drag.original.translation.to_array().map(f32::to_bits),
                    drag.original.rotation.to_array().map(f32::to_bits),
                    drag.original.scale.to_array().map(f32::to_bits),
                ),
                world: drag.world_origin.to_array().map(f32::to_bits),
                axis: match drag.axis {
                    DragAxis::Plane => 0,
                    DragAxis::X => 1,
                    DragAxis::Y => 2,
                    DragAxis::Z => 3,
                },
                edit: drag.edit.change_token(),
            }),
            objects: app.object_ids.clone(),
            instances: app.instances.clone(),
            selected: app.selected,
            next_object_id: app.authoring.next_object_id,
            settings_written: app.authoring.settings_written.clone(),
            scene_path: app.authoring.scene_path.clone(),
            scene_revision: app.authoring.scene_revision.clone(),
            prefab_assets: app.authoring.prefab_assets.clone(),
            prefab_choice: app.authoring.prefab_choice,
            preview_seek: app
                .authoring
                .preview_seek
                .map(|(node, phase)| (node, phase.map(f32::to_bits))),
        }
    }
    pub(crate) fn validate(&self, app: &App) -> Result<(), EditorError> {
        if self != &Self::capture(app) {
            return Err("scene or authoring state changed during restore preparation".into());
        }
        Ok(())
    }
}
#[derive(PartialEq, Eq)]
struct ModelObservation {
    animation: crate::animation_runtime::AnimationChangeToken,
    catalog: voxy_assets::AssetChangeToken,
    sources: voxy_assets::AssetChangeToken,
    recipe: InputRecipe,
    current: Option<AssetId>,
    available: BTreeSet<AssetId>,
    queued: BTreeSet<AssetId>,
    imports: bool,
    watcher: bool,
}
pub(crate) struct ModelRuntimeGuard {
    fence: Arc<()>,
    state: ModelObservation,
}
impl ModelRuntimeGuard {
    pub(crate) fn capture(app: &App) -> Result<Self, EditorError> {
        let Some(super::resource_checkpoint::Barrier::Ready { epoch, .. }) =
            &app.resource_checkpoint
        else {
            return Err("model origin requires an ordered ready fence".into());
        };
        Ok(Self {
            fence: epoch.clone(),
            state: ModelObservation {
                animation: app.play.animations.change_token(),
                catalog: app.catalog.change_token(),
                sources: app.sources.change_token(),
                recipe: app.import_recipe.clone(),
                current: app.id.clone(),
                available: app.available.clone(),
                queued: app.reload.clone(),
                imports: app.imports.is_some(),
                watcher: app.watcher.is_some(),
            },
        })
    }
    pub(crate) fn validate(&self, app: &App) -> Result<(), EditorError> {
        let current = Self::capture(app)?;
        if !Arc::ptr_eq(&self.fence, &current.fence) || self.state != current.state {
            return Err(
                "model runtime or resource origin changed during restore preparation".into(),
            );
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_observation_detects_clock_sources_and_ready_fence_replacement() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/animated-triangle.glb");
        let mut app = App::new(&fixture, false).unwrap();
        app.tick().unwrap();
        app.begin_model_checkpoint().unwrap();
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !app.poll_model_checkpoint().unwrap() {
            assert!(std::time::Instant::now() < until);
            std::thread::yield_now();
        }
        let id = app.id.clone().unwrap();
        let models = BTreeMap::from([(
            id.clone(),
            app.catalog
                .snapshot(&id)
                .unwrap()
                .value()
                .animated
                .as_ref()
                .unwrap()
                .clone(),
        )]);
        let guard = ModelRuntimeGuard::capture(&app).unwrap();
        let scene = app.scene.change_token();
        app.play.animations = app
            .play
            .animations
            .prepare(&app.scene, &models, 0.05)
            .unwrap();
        assert_eq!(scene, app.scene.change_token());
        assert!(guard.validate(&app).is_err());
        let guard = ModelRuntimeGuard::capture(&app).unwrap();
        app.catalog.invalidate(std::slice::from_ref(&id)).unwrap();
        assert!(guard.validate(&app).is_err());
        let guard = ModelRuntimeGuard::capture(&app).unwrap();
        let saved = app.sources.checkpoint();
        app.sources = saved.prepare(256, 8192).unwrap();
        assert_eq!(saved, app.sources.checkpoint());
        assert!(guard.validate(&app).is_err());
        let guard = ModelRuntimeGuard::capture(&app).unwrap();
        app.resume_model_checkpoint().unwrap();
        assert!(guard.validate(&app).is_err());
        app.begin_model_checkpoint().unwrap();
        let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !app.poll_model_checkpoint().unwrap() {
            assert!(std::time::Instant::now() < until);
            std::thread::yield_now();
        }
        let current = ModelRuntimeGuard::capture(&app).unwrap();
        assert!(guard.state == current.state);
        assert!(guard.validate(&app).is_err());
        app.stop_workers().unwrap();
    }
    #[test]
    fn native_app_observation_rejects_scene_selection_and_history_changes() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        let guard = SceneAuthoringGuard::capture(&app);
        guard.validate(&app).unwrap();
        app.authoring.next_object_id += 1;
        assert!(guard.validate(&app).is_err());
        let guard = SceneAuthoringGuard::capture(&app);
        app.selected += 1;
        assert!(guard.validate(&app).is_err());
        let node = app.scene.spawn(None, Default::default()).unwrap();
        let guard = SceneAuthoringGuard::capture(&app);
        let structure = app.scene.structure_revision();
        app.scene
            .set_local(
                node,
                Transform {
                    translation: Vec3::X,
                    ..Default::default()
                },
            )
            .unwrap();
        app.scene.set_local(node, Default::default()).unwrap();
        assert_eq!(structure, app.scene.structure_revision());
        assert!(guard.validate(&app).is_err());
        let registry = ComponentRegistry::default();
        let mut history = SceneHistory::new(
            voxy_scene::SceneDocument {
                version: 1,
                objects: vec![],
            },
            &registry,
            8,
            8,
            65536,
        )
        .unwrap();
        history
            .commit_auxiliary(serde_json::json!({"value": 1}), &registry)
            .unwrap();
        app.authoring.history = Some(history);
        let guard = SceneAuthoringGuard::capture(&app);
        app.authoring.history.as_mut().unwrap().undo();
        app.authoring.history.as_mut().unwrap().redo();
        assert!(guard.validate(&app).is_err());
        app.drag = Some(ModelDrag {
            node,
            origin: Vec2::ZERO,
            viewport: Vec2::splat(100.),
            viewport_origin: Vec2::ZERO,
            original: Transform::default(),
            world_origin: Vec3::ZERO,
            axis: DragAxis::Plane,
            edit: app.authoring.history.as_ref().unwrap().begin_edit(),
        });
        let guard = SceneAuthoringGuard::capture(&app);
        let _ = app.drag.as_mut().unwrap().edit.document_mut();
        assert!(guard.validate(&app).is_err());
        let guard = SceneAuthoringGuard::capture(&app);
        app.drag.as_mut().unwrap().origin.x = -0.;
        assert!(guard.validate(&app).is_err());
        let guard = SceneAuthoringGuard::capture(&app);
        app.object_ids.push(ObjectId("another".into()));
        assert!(guard.validate(&app).is_err());
        app.stop_workers().unwrap();
    }
    #[test]
    fn native_authoring_metadata_changes_invalidate_without_scene_edits() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        let scene = app.scene.change_token();
        let guard = SceneAuthoringGuard::capture(&app);
        app.authoring
            .settings_written
            .insert("material.settings".into(), [7; 32]);
        assert!(guard.validate(&app).is_err());
        let guard = SceneAuthoringGuard::capture(&app);
        app.authoring.scene_path = Some("another.scene".into());
        assert!(guard.validate(&app).is_err());
        let guard = SceneAuthoringGuard::capture(&app);
        app.authoring
            .prefab_assets
            .push(AssetId("prefab.scene".into()));
        assert!(guard.validate(&app).is_err());
        let guard = SceneAuthoringGuard::capture(&app);
        app.authoring.prefab_choice += 1;
        assert!(guard.validate(&app).is_err());
        let node = app.instances[0];
        app.authoring.preview_seek = Some((node, [0.; 4]));
        let guard = SceneAuthoringGuard::capture(&app);
        app.authoring.preview_seek.as_mut().unwrap().1[0] = -0.;
        assert!(guard.validate(&app).is_err());
        let document = voxy_scene::SceneDocument {
            version: 1,
            objects: vec![],
        };
        app.authoring.scene_revision = Some(
            crate::scene_revision::SceneRevision::written(
                std::path::Path::new("before.scene"),
                &document,
            )
            .unwrap(),
        );
        let guard = SceneAuthoringGuard::capture(&app);
        app.authoring.scene_revision = Some(
            crate::scene_revision::SceneRevision::written(
                std::path::Path::new("after.scene"),
                &document,
            )
            .unwrap(),
        );
        assert!(guard.validate(&app).is_err());
        let guard = SceneAuthoringGuard::capture(&app);
        let worker_project = app.authoring.authoring_project.worker_project().unwrap();
        app.authoring.authoring_project = worker_project;
        guard.validate(&app).unwrap();
        let guard = SceneAuthoringGuard::capture(&app);
        app.authoring.authoring_project.registry =
            Arc::new(crate::editor_component_registry().unwrap());
        assert!(guard.validate(&app).is_err());
        let guard = SceneAuthoringGuard::capture(&app);
        app.authoring.authoring_project = crate::prefab_authoring::AuthoringProject::with_registry(
            fixture.parent().unwrap(),
            &InputRecipe::Direct,
            app.authoring.authoring_project.registry.clone(),
        )
        .unwrap();
        assert!(guard.validate(&app).is_err());
        assert_eq!(scene, app.scene.change_token());
        app.stop_workers().unwrap();
    }
}
