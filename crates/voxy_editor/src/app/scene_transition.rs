//! Runtime scene transitions coordinated safely with active background workers.

use crate::*;
use voxy_assets::AssetStatus;

struct StagedPlayScene {
    document: SceneDocument,
    loaded: voxy_scene::LoadedScene,
    instances: Vec<NodeId>,
    object_ids: Vec<ObjectId>,
    next_id: u64,
    liquid: Option<(voxy_gameplay::SceneLiquidRuntime, Vec<Option<[f32; 4]>>)>,
}

impl App {
    pub(crate) fn transition_play_scene(
        &mut self,
        target: &SourcePath,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_none() || self.restore_play_frozen() {
            return Err("scene transition requires an active unfrozen Play session".into());
        }
        let staged = self.stage_play_scene_transition(target)?;
        self.retire_active_scene_workers(
            staged
                .loaded
                .graph
                .components::<voxy_gameplay::AudioSource>()
                .next()
                .is_none(),
        )?;
        self.install_staged_play_scene(staged);
        self.wait_for_transition_cpu_assets()?;
        self.start_installed_play_scene(target)
    }

    fn stage_play_scene_transition(
        &self,
        target: &SourcePath,
    ) -> Result<StagedPlayScene, Box<dyn std::error::Error>> {
        let target_path = self
            .authoring
            .authoring_project
            .root()
            .join(target.as_str());
        let imported = self.authoring.authoring_project.load(&target_path)?;
        let document = imported.value().expanded.clone();
        let next_id = self.validate_authoring_document_policy(&document, true)?;
        for object in &document.objects {
            if let Some(asset) = object
                .components
                .get("editor.model.v1")
                .and_then(serde_json::Value::as_str)
            {
                let id = AssetId(asset.into());
                if !self.available.contains(&id) && self.catalog.snapshot(&id).is_none() {
                    return Err(
                        format!("game model {} is unresolved in the asset manifest", id.0).into(),
                    );
                }
            }
        }
        let mut loaded = document.load(
            &self.authoring.authoring_project.registry,
            scene_limits::OBJECTS,
        )?;
        let mut instances = Vec::with_capacity(document.objects.len());
        let mut object_ids = Vec::with_capacity(document.objects.len());
        for object in &document.objects {
            let node = loaded.resolve(&object.id).ok_or("missing loaded node")?;
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
            object_ids.push(object.id.clone());
        }
        CharacterPhysics::new(&loaded.graph, 128, 128)
            .with_depenetration(true)
            .validate_start(&loaded.graph)?;
        let liquid = self
            .authoring
            .authoring_project
            .liquid_runtime(&loaded.graph)?;
        if let Some((runtime, _)) = &liquid {
            voxy_gameplay::validate_game_descriptors_with_liquid_runtime(
                &loaded.graph,
                128,
                runtime,
            )?;
        } else {
            voxy_gameplay::validate_game_descriptors(&loaded.graph, 128)?;
        }
        voxy_gameplay::validate_physics_joints(&loaded.graph, |id| loaded.resolve(id))?;
        shadow_authoring::validate_scene(&loaded.graph)?;
        if let Some((_, cam)) = loaded.graph.active_components::<EditorCamera>().next()
            && !cam.valid()
        {
            return Err("invalid scene camera".into());
        }
        Ok(StagedPlayScene {
            document,
            loaded,
            instances,
            object_ids,
            next_id,
            liquid,
        })
    }

    fn retire_active_scene_workers(
        &mut self,
        close_audio_device: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.abort_pending_gpu_for_scene_transition()?;
        self.game_ui_input = game_ui_input::GameUiInput::new();
        self.retired_ui_workers.extend(self.ui_live.close());
        let mut remaining = Vec::new();
        for worker in self.retired_ui_workers.drain(..) {
            if worker.is_finished() || remaining.len() >= 8 {
                if let Err(payload) = worker.join() {
                    return Err(format!(
                        "retired UI worker panicked: {}",
                        panic_message(payload.as_ref())
                    )
                    .into());
                }
            } else {
                remaining.push(worker);
            }
        }
        self.retired_ui_workers = remaining;
        if let Some(graphics) = &mut self.graphics {
            graphics.ui_draws.clear();
            graphics.ui_presented = false;
            graphics.ui_focus.clear();
            graphics.ui_focus_key = None;
            graphics.ui_focus_deferred = None;
            graphics.transforms.clear();
            graphics.animated_models.clear();
            graphics.liquid_revision = None;
            graphics.lod_history.clear();
        }
        self.close_play_audio();
        if close_audio_device {
            self.close_audio_device();
        } else {
            self.audio.cancel_pending_audio();
        }
        self.play.stop_simulation(&mut self.scene)?;
        Ok(())
    }

    fn install_staged_play_scene(&mut self, staged: StagedPlayScene) {
        if let Some((_, camera)) = staged
            .loaded
            .graph
            .active_components::<EditorCamera>()
            .next()
        {
            self.camera = camera.clone();
        }
        self.scene = staged.loaded.graph;
        self.instances = staged.instances;
        self.object_ids = staged.object_ids;
        self.selected = 0;
        self.extra_selected.clear();
        if self.standalone {
            self.play.playing = Some(staged.document);
            self.authoring.next_object_id = self.authoring.next_object_id.max(staged.next_id);
        }
        let (liquid, optics) = staged
            .liquid
            .map_or((None, Vec::new()), |(r, o)| (Some(r), o));
        self.play.liquid_optics = optics;
        self.play.liquid = liquid;
        for asset in self.required_cpu_model_assets() {
            if self.catalog.snapshot(&asset).is_none() {
                self.reload.insert(asset);
            }
        }
    }

    fn transition_blocking_cpu_assets(&self) -> BTreeSet<AssetId> {
        if self.window.is_none() {
            return self.required_cpu_model_assets();
        }
        let mut required = BTreeSet::new();
        for (_, profile) in self.scene.components::<ModelRetarget>() {
            required.insert(AssetId(profile.source.clone()));
        }
        for (node, model) in self.scene.components::<ModelInstance>() {
            let needs_cpu_metadata = self
                .scene
                .component::<ModelAnimation>(node)
                .ok()
                .flatten()
                .is_some()
                || self
                    .scene
                    .component::<ModelLocomotion>(node)
                    .ok()
                    .flatten()
                    .is_some()
                || self
                    .scene
                    .component::<ModelRetarget>(node)
                    .ok()
                    .flatten()
                    .is_some()
                || self
                    .scene
                    .component::<ModelFootPlacement>(node)
                    .ok()
                    .flatten()
                    .is_some()
                || self
                    .scene
                    .component::<ModelPart>(node)
                    .ok()
                    .flatten()
                    .is_some();
            if needs_cpu_metadata {
                required.insert(model.asset.clone());
            }
        }
        required
    }

    fn wait_for_transition_cpu_assets(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let required = self.transition_blocking_cpu_assets();
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            self.tick()?;
            for id in &required {
                if let Some(AssetStatus::Failed(error)) = self.catalog.status(id) {
                    return Err(format!("transition scene asset {} failed: {error}", id.0).into());
                }
            }
            if required
                .iter()
                .all(|id| self.catalog.snapshot(id).is_some())
            {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err("transition scene resource imports timed out".into());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn start_installed_play_scene(
        &mut self,
        target: &SourcePath,
    ) -> Result<(), Box<dyn std::error::Error>> {
        locomotion::validate_scene(&self.scene, &self.animation_models())?;
        if let audio_session::AudioStart::Ready(audio) = self.audio.prepare_start(
            &self.scene,
            &self.authoring.authoring_project,
            self.window.is_some(),
        )? {
            self.audio.set_play(audio);
        }
        self.play.player_input = voxy_gameplay::player_input()?;
        let mut actions = voxy_gameplay::UiActionHandlers::new(&self.scene, 128, 128, 128);
        crate::ui_action_codecs::register_handlers(&mut actions)?;
        if let Some(setup) = &mut self.ui_action_setup {
            setup.configure(&mut actions)?;
        }
        self.play.ui_actions = Some(actions);
        self.play.simulation = Some(SceneSimulation::new(&self.scene, scene_limits::SIMULATION)?);
        if let Some(setup) = &mut self.behavior_setup {
            let simulation = self.play.simulation.as_mut().expect("new simulation");
            setup.configure(&mut self.scene, simulation)?;
            simulation.reset_interpolation(&self.scene)?;
        }
        let routes = voxy_gameplay::prepare_npc_routes(
            &self.scene,
            128,
            &mut voxy_gameplay::SupportQueryBudget::new(65_536)?,
        )?;
        for (owner, route) in routes {
            self.scene.insert_component(owner, route)?;
        }
        let has_characters = self.scene.components::<CharacterBody>().next().is_some()
            || self.scene.components::<BoxCollider>().next().is_some();
        self.play.physics = has_characters
            .then(|| CharacterPhysics::new(&self.scene, 128, 128).with_depenetration(true));
        self.play.angular_motion = Some(voxy_gameplay::AngularMotionBatch::new(&self.scene, 128)?);
        self.play.triggers = None;
        self.play.animations.clear();
        self.play.follow_camera = None;
        self.update_follow_camera(0.)?;
        self.play.simulation_time = Instant::now();
        self.active_play_scene = Some(target.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_two_scene_project(root: &std::path::Path) -> std::path::PathBuf {
        let proj = crate::project::create(root, "TransitionDemo").unwrap();
        std::fs::write(
            root.join("quad_a.obj"),
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )
        .unwrap();
        std::fs::write(
            root.join("quad_b.obj"),
            "v 0 0 0\nv 2 0 0\nv 0 2 0\nf 1 2 3\n",
        )
        .unwrap();
        std::fs::write(
            root.join("slow_bg.obj"),
            "v 0 0 0\nv 3 0 0\nv 0 3 0\nf 1 2 3\n",
        )
        .unwrap();
        std::fs::write(
            root.join("assets.json"),
            r#"{"version":1,"assets":[{"asset":"quad-a","source":"quad_a.obj"},{"asset":"quad-b","source":"quad_b.obj"},{"asset":"slow-bg","source":"slow_bg.obj"},{"asset":"arena-scene","source":"arena.scene.json"}]}"#,
        )
        .unwrap();
        std::fs::write(
            root.join("main.scene.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "version": 1,
                "objects": [
                    {
                        "id": "scene-a-mesh",
                        "parent": null,
                        "name": "Scene A Mesh",
                        "active": true,
                        "translation": [0.0, 0.0, 0.0],
                        "rotation": [0.0, 0.0, 0.0, 1.0],
                        "scale": [1.0, 1.0, 1.0],
                        "components": { "editor.model.v1": "quad-a" }
                    },
                    {
                        "id": "go-arena",
                        "parent": null,
                        "name": "Go Arena Button",
                        "active": true,
                        "translation": [0.0, 0.0, 0.0],
                        "rotation": [0.0, 0.0, 0.0, 1.0],
                        "scale": [1.0, 1.0, 1.0],
                        "components": {
                            "game.ui-element.v1": {
                                "origin": [0.0, 0.0],
                                "size": [0.25, 0.25],
                                "color": [0.2, 0.4, 0.8, 1.0],
                                "layer": 0,
                                "enabled": true,
                                "action": "voxy.scene.load:arena.scene.json",
                                "text": null
                            }
                        }
                    }
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.join("arena.scene.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "version": 1,
                "objects": [
                    {
                        "id": "arena-hero",
                        "parent": null,
                        "name": "Arena Hero",
                        "active": true,
                        "translation": [4.0, 2.0, 0.0],
                        "rotation": [0.0, 0.0, 0.0, 1.0],
                        "scale": [1.0, 1.0, 1.0],
                        "components": {
                            "editor.model.v1": "quad-b",
                            "game.character.v1": {
                                "half_extents": [0.25, 0.5, 0.25],
                                "speed": 3.0,
                                "gravity": -9.8,
                                "jump_speed": 4.0
                            }
                        }
                    }
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        proj
    }

    #[test]
    fn transition_with_active_background_import_and_editor_stop_restores_scene() {
        let temp = crate::project_prefab_tests::Temp::new();
        let proj = write_two_scene_project(&temp.0.join("proj"));
        let mut app = App::from_project(&proj).unwrap();
        let original_doc = app.authoring_document().unwrap();

        // Enter Editor Play (F6), then submit an uncached background import (`slow-bg`)
        // right before requesting a transition so `imports.is_busy()` is true during handoff.
        app.toggle_play().unwrap();
        assert_eq!(
            app.active_play_scene.as_ref().map(SourcePath::as_str),
            Some("main.scene.json")
        );
        let bg_id = AssetId("slow-bg".into());
        let bg_ticket = app.catalog.request(bg_id.clone()).unwrap();
        app.imports.as_mut().unwrap().submit(&bg_ticket).unwrap();
        assert!(app.imports.as_ref().unwrap().is_busy());

        app.transition_play_scene(&SourcePath::new("arena.scene.json").unwrap())
            .unwrap();
        assert_eq!(
            app.active_play_scene.as_ref().map(SourcePath::as_str),
            Some("arena.scene.json")
        );
        assert!(app.resolve_object(&ObjectId("arena-hero".into())).is_some());
        assert!(app.catalog.snapshot(&AssetId("quad-b".into())).is_some());
        assert!(app.catalog.snapshot(&bg_id).is_some());

        // Advance simulation ticks in Scene B, verify invalid transition fails without breaking B.
        app.advance_game(1.0 / 60.0).unwrap();
        assert!(
            app.transition_play_scene(&SourcePath::new("does-not-exist.scene.json").unwrap())
                .is_err()
        );
        assert_eq!(
            app.active_play_scene.as_ref().map(SourcePath::as_str),
            Some("arena.scene.json")
        );

        // Stopping Play must restore exact Scene A authoring document untouched.
        app.toggle_play().unwrap();
        assert!(app.active_play_scene.is_none());
        assert_eq!(app.authoring_document().unwrap(), original_doc);
        app.stop_workers().unwrap();
    }

    #[test]
    fn ui_action_and_zero_extract_package_transition_cleanly() {
        let temp = crate::project_prefab_tests::Temp::new();
        let proj_dir = temp.0.join("proj");
        let proj = write_two_scene_project(&proj_dir);

        // 1. UI click triggers `voxy.scene.load:arena.scene.json` in HeadlessGame.
        let mut game = HeadlessGame::open_project(
            &proj,
            GameHost::new(crate::editor_component_registry().unwrap()),
        )
        .unwrap();
        assert_eq!(game.active_scene_path(), Some("main.scene.json"));
        game.pointer([40.0, 40.0], [1280.0, 720.0], true).unwrap();
        game.pointer([40.0, 40.0], [1280.0, 720.0], false).unwrap();
        game.advance(1.0 / 60.0).unwrap();
        assert_eq!(game.active_scene_path(), Some("arena.scene.json"));
        let snap = game.snapshot().unwrap();
        assert!(snap.objects.iter().any(|o| o.id.0 == "arena-hero"));
        game.stop().unwrap();

        // 2. Export to .voxypkg, delete source dir completely, and transition in-memory!
        let pkg = temp.0.join("demo.voxypkg");
        crate::export_project_package(&proj, &pkg).unwrap();
        std::fs::remove_dir_all(&proj_dir).unwrap();
        let mut pkg_game = HeadlessGame::open_package(
            &pkg,
            GameHost::new(crate::editor_component_registry().unwrap()),
        )
        .unwrap();
        assert_eq!(pkg_game.active_scene_path(), Some("main.scene.json"));
        pkg_game.transition_scene("arena.scene.json").unwrap();
        assert_eq!(pkg_game.active_scene_path(), Some("arena.scene.json"));
        pkg_game.advance(1.0 / 60.0).unwrap();
        assert!(
            pkg_game
                .snapshot()
                .unwrap()
                .objects
                .iter()
                .any(|o| o.id.0 == "arena-hero")
        );
        pkg_game.stop().unwrap();
    }
}
