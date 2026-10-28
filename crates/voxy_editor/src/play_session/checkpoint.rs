//! Complete PlaySession participant. App audio/resources/UI routing remain outer owners.
use super::*;
use crate::{
    animation_runtime::checkpoint::AnimationRuntimeCheckpoint, camera::ViewportCamera,
    follow_camera::FollowRuntimeCheckpoint,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use voxy_animation::{AnimationClip, PropertyClip};
use voxy_gameplay::{
    AngularMotionCheckpoint, CharacterPhysicsCheckpoint, GameplayCheckpointLimits,
    SceneLiquidCheckpoint, SceneLiquidCheckpointLimits, SceneLiquidWireLimits, UiActionCheckpoint,
    UiActionCheckpointLimits, UiActionCheckpointRegistry,
};
use voxy_model::ModelAsset;
use voxy_scene::{
    BehaviorCheckpointRegistry, ComponentRegistry, LoadedScene, SceneCheckpointView,
    SceneSimulationCheckpoint,
};
type Error = Box<dyn std::error::Error>;
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct PlayCheckpointLimits {
    pub core: GameplayCheckpointLimits,
    pub max_angular_motions: usize,
    pub max_optical_materials: usize,
    pub ui: UiActionCheckpointLimits,
    pub liquid: SceneLiquidCheckpointLimits,
    pub liquid_wire: SceneLiquidWireLimits,
}
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlaySessionCheckpoint {
    version: u32,
    compatibility: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    authoring: Option<SceneDocument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scene: Option<SceneSimulationCheckpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    idle_scene: Option<SceneDocument>,
    animations: AnimationRuntimeCheckpoint,
    angular: Option<AngularMotionCheckpoint>,
    ticks: u64,
    camera: ViewportCamera,
    follow: Option<FollowRuntimeCheckpoint>,
    optics: Vec<Option<[u32; 4]>>,
    liquid: Option<Vec<u8>>,
    physics: Option<CharacterPhysicsCheckpoint>,
    input: String,
    ui: Option<UiActionCheckpoint>,
}
/// One fresh scene and its matching PlaySession. Host must stage all outer owners
/// before moving this unit into App. Dropping a failed candidate does not touch live state.
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct PreparedPlaySession {
    pub scene: LoadedScene,
    pub play: PlaySession,
    pub camera: ViewportCamera,
}
#[allow(dead_code)]
impl PlaySession {
    pub(crate) fn checkpoint(
        &self,
        scene: &SceneCheckpointView<'_>,
        registry: &ComponentRegistry,
        compatibility: &str,
        camera: &ViewportCamera,
        identify_model: impl Fn(&Arc<ModelAsset>) -> Option<String>,
        identify_clip: impl Fn(&Arc<AnimationClip>, Option<&Arc<PropertyClip>>) -> Option<String>,
        limits: PlayCheckpointLimits,
    ) -> Result<PlaySessionCheckpoint, Error> {
        // Exhaustive: new runtime fields must receive a deliberate save decision.
        // Instant is a host frame-pacing epoch, rebased after loading; the actual
        // simulation accumulator/presentation clock is saved in SceneSimulation.
        let Self {
            playing,
            simulation_time: _,
            simulation,
            animations,
            angular_motion,
            simulation_ticks,
            follow_camera,
            liquid_optics,
            liquid,
            physics,
            player_input,
            ui_actions,
        } = self;
        let (version, authoring, saved_scene, idle_scene) = match (playing, simulation) {
            (Some(authoring), Some(simulation)) => (
                1,
                Some(authoring.clone()),
                Some(simulation.checkpoint_view(scene, registry, limits.core.max_bytes)?),
                None,
            ),
            (None, None) => (2, None, None, Some(scene.capture(registry)?)),
            _ => return Err("inconsistent Play lifecycle checkpoint".into()),
        };
        let identify = |node| scene.identity(node).cloned();
        let saved = PlaySessionCheckpoint {
            version,
            compatibility: compatibility.into(),
            authoring,
            scene: saved_scene,
            idle_scene,
            animations: animations.checkpoint(
                scene.graph,
                identify,
                identify_model,
                identify_clip,
                limits.core.max_bytes,
            )?,
            angular: angular_motion
                .as_ref()
                .map(voxy_gameplay::AngularMotionBatch::checkpoint),
            ticks: *simulation_ticks,
            camera: camera.clone(),
            follow: follow_camera
                .as_ref()
                .map(|follow| follow.checkpoint(camera, identify))
                .transpose()?,
            optics: liquid_optics
                .iter()
                .map(|o| o.map(|v| v.map(f32::to_bits)))
                .collect(),
            liquid: liquid
                .as_ref()
                .map(|liquid| {
                    liquid
                        .checkpoint(scene.graph, identify)?
                        .to_bytes(limits.liquid_wire.max_bytes)
                })
                .transpose()?,
            physics: physics
                .as_ref()
                .map(|physics| physics.checkpoint(scene.graph, identify))
                .transpose()?,
            input: voxy_gameplay::encode_input_checkpoint(
                &player_input.checkpoint(),
                limits.core.max_bytes,
                limits.core.max_actions,
                limits.core.max_bindings,
            )?,
            ui: ui_actions
                .as_ref()
                .map(|ui| ui.checkpoint(scene.graph, identify, limits.ui))
                .transpose()?,
        };
        saved.to_json(limits.core.max_bytes)?;
        Ok(saved)
    }
}
#[allow(dead_code)]
impl PlaySessionCheckpoint {
    fn validate_envelope(&self) -> Result<(), Error> {
        let valid_mode = matches!(
            (self.version, &self.authoring, &self.scene, &self.idle_scene),
            (1, Some(_), Some(_), None) | (2, None, None, Some(_))
        );
        if !valid_mode
            || self.compatibility.is_empty()
            || self.compatibility.len() > 1024
            || !self.camera.valid()
            || (self.liquid.is_none() && !self.optics.is_empty())
        {
            return Err("invalid Play checkpoint envelope".into());
        }
        Ok(())
    }
    pub(crate) fn to_json(&self, max_bytes: usize) -> Result<String, Error> {
        self.validate_envelope()?;
        let json = serde_json::to_string(self)?;
        if json.len() > max_bytes {
            return Err("Play checkpoint byte budget".into());
        }
        Ok(json)
    }
    /// The host also bounds its underlying file read. This prepares no live state.
    pub(crate) fn from_json(json: &str, max_bytes: usize) -> Result<Self, Error> {
        if json.len() > max_bytes {
            return Err("Play checkpoint byte budget".into());
        }
        let saved: Self = serde_json::from_str(json)?;
        saved.validate_envelope()?;
        Ok(saved)
    }
    pub(crate) fn prepare(
        &self,
        compatibility: &str,
        registry: &ComponentRegistry,
        behaviors: &BehaviorCheckpointRegistry,
        ui_codecs: &UiActionCheckpointRegistry,
        resolve_model: impl Fn(&str) -> Option<Arc<ModelAsset>>,
        resolve_clip: impl Fn(&str) -> Option<(Arc<AnimationClip>, Option<Arc<PropertyClip>>)>,
        limits: PlayCheckpointLimits,
    ) -> Result<PreparedPlaySession, Error> {
        if self.compatibility != compatibility {
            return Err("incompatible Play checkpoint".into());
        }
        self.to_json(limits.core.max_bytes)?;
        if self.optics.len() > limits.max_optical_materials {
            return Err("Play optical material budget".into());
        }
        // Stop must remain possible after loading, with the same component schemas.
        if let Some(authoring) = &self.authoring {
            authoring.load(registry, limits.core.scene_capacity)?;
        }
        let (scene, simulation) = match (&self.scene, &self.idle_scene) {
            (Some(saved), None) => {
                let (scene, simulation) = saved.prepare(
                    registry,
                    behaviors,
                    limits.core.simulation,
                    limits.core.scene_capacity,
                    limits.core.max_bytes,
                )?;
                (scene, Some(simulation))
            }
            (None, Some(saved)) => (saved.load(registry, limits.core.scene_capacity)?, None),
            _ => return Err("invalid Play checkpoint lifecycle".into()),
        };
        let resolve = |id: &voxy_scene::ObjectId| scene.resolve(id);
        let animations = self.animations.prepare(
            &scene.graph,
            resolve,
            resolve_model,
            resolve_clip,
            limits.core.max_bytes,
        )?;
        let angular_motion = self
            .angular
            .map(|angular| angular.prepare(&scene.graph, limits.max_angular_motions))
            .transpose()?;
        let follow = self
            .follow
            .as_ref()
            .map(|follow| follow.prepare(&scene.graph, resolve))
            .transpose()?;
        if let Some((_, view)) = &follow {
            if serde_json::to_vec(view)? != serde_json::to_vec(&self.camera)? {
                return Err("inconsistent saved follow camera".into());
            }
        }
        let liquid = self
            .liquid
            .as_ref()
            .map(|bytes| {
                SceneLiquidCheckpoint::from_bytes(bytes, limits.liquid_wire)?.prepare(
                    &scene.graph,
                    resolve,
                    limits.liquid,
                )
            })
            .transpose()?;
        let optics: Vec<_> = self
            .optics
            .iter()
            .map(|o| o.map(|v| v.map(f32::from_bits)))
            .collect();
        if optics.iter().flatten().any(|v| {
            v.iter().any(|x| !x.is_finite()) || v[..3].iter().any(|x| *x < 0.) || v[3] < 1.
        }) {
            return Err("invalid saved liquid optics".into());
        }
        let physics = self
            .physics
            .as_ref()
            .map(|saved| {
                let mut physics = CharacterPhysics::new(
                    &scene.graph,
                    limits.core.max_bodies,
                    limits.core.max_colliders,
                );
                physics.restore_checkpoint(&scene.graph, saved, resolve)?;
                Ok::<_, voxy_gameplay::PhysicsError>(physics)
            })
            .transpose()?;
        let mut player_input = InputMap::new(limits.core.max_actions, limits.core.max_bindings);
        player_input.restore_checkpoint(&voxy_gameplay::decode_input_checkpoint(
            &self.input,
            limits.core.max_bytes,
            limits.core.max_actions,
            limits.core.max_bindings,
        )?)?;
        // Last participant is intentionally fallible; a late codec/reference error
        // drops this entire private candidate instead of publishing a partial load.
        let ui_actions = self
            .ui
            .as_ref()
            .map(|ui| ui_codecs.prepare(&scene.graph, ui, resolve, limits.ui))
            .transpose()?;
        Ok(PreparedPlaySession {
            camera: self.camera.clone(),
            play: PlaySession {
                playing: self.authoring.clone(),
                simulation_time: Instant::now(),
                simulation,
                animations,
                angular_motion,
                simulation_ticks: self.ticks,
                follow_camera: follow.map(|(runtime, _)| runtime),
                liquid_optics: optics,
                liquid,
                physics,
                player_input,
                ui_actions,
            },
            scene,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use voxy_scene::{ObjectId, SceneCommand, Transform};
    use winit::{event::ElementState, keyboard::KeyCode};
    pub(crate) fn limits() -> PlayCheckpointLimits {
        PlayCheckpointLimits {
            core: GameplayCheckpointLimits {
                simulation: voxy_scene::SimulationLimits {
                    fixed_step: 1. / 120.,
                    max_steps: 8,
                    max_behaviors: 8,
                    max_commands: 32,
                },
                scene_capacity: 16,
                max_bodies: 8,
                max_colliders: 8,
                max_actions: 128,
                max_bindings: 128,
                max_bytes: 8 * 1024 * 1024,
            },
            max_angular_motions: 8,
            max_optical_materials: 8,
            ui: UiActionCheckpointLimits {
                max_events: 8,
                max_handlers: 8,
                max_commands_per_event: 8,
                max_bytes: 100000,
            },
            liquid: SceneLiquidCheckpointLimits {
                liquid: physics::liquid::LiquidStateCheckpointLimits {
                    core: physics::liquid::LiquidCheckpointLimits {
                        max_particles: 256,
                        max_materials: 4,
                        max_pairs: 200000,
                        max_neighbor_checks: 2000000,
                        max_substeps: 1024,
                    },
                    max_boundary_samples: 128,
                    max_interfaces: 8,
                    max_species: 8,
                    max_name_bytes: 1024,
                    max_reaction_attempts: 4096,
                },
                max_sources: 8,
                max_bodies: 8,
                max_pulses_per_source: 16,
                max_binding_entries: 128,
            },
            liquid_wire: SceneLiquidWireLimits {
                max_bytes: 1000000,
                max_allocation_bytes: 1000000,
                max_values: 1000000,
                liquid: physics::liquid::LiquidWireLimits {
                    max_bytes: 1000000,
                    max_allocation_bytes: 1000000,
                    max_values: 1000000,
                },
            },
        }
    }
    #[derive(Debug, Serialize, Deserialize)]
    struct Counter {
        ticks: u64,
    }
    impl voxy_scene::Behavior for Counter {
        fn checkpoint(
            &self,
        ) -> Result<voxy_scene::BehaviorState, voxy_scene::BehaviorCheckpointError> {
            voxy_scene::BehaviorState::json("play.counter.v1", self)
        }
        fn fixed_update(&mut self, _: &mut SceneGraph, _: voxy_scene::NodeId, _: f64) {
            self.ticks += 1;
        }
    }
    struct Clicks(u64);
    impl voxy_gameplay::UiActionHandler for Clicks {
        fn dispatch(
            &mut self,
            _: &SceneGraph,
            _: &voxy_gameplay::UiActionEvent,
            _: &mut voxy_scene::SceneCommands,
        ) -> Result<(), voxy_gameplay::GameplayError> {
            self.0 += 1;
            Ok(())
        }
        fn checkpoint(
            &self,
            _: &dyn Fn(voxy_scene::NodeId) -> Option<ObjectId>,
        ) -> Result<voxy_gameplay::UiActionState, voxy_gameplay::GameplayError> {
            voxy_gameplay::UiActionState::new(
                "play.clicks.v1".into(),
                self.0.to_le_bytes().to_vec(),
            )
        }
    }
    fn codecs() -> (
        ComponentRegistry,
        BehaviorCheckpointRegistry,
        UiActionCheckpointRegistry,
    ) {
        let mut components = crate::editor_component_registry().unwrap();
        crate::register_game_checkpoint_components(&mut components).unwrap();
        let mut behaviors = BehaviorCheckpointRegistry::default();
        behaviors.register::<Counter>("play.counter.v1").unwrap();
        let mut ui = UiActionCheckpointRegistry::default();
        ui.register("play.clicks.v1", |bytes, _| {
            Ok(Box::new(Clicks(u64::from_le_bytes(
                bytes.try_into().map_err(|_| "invalid clicks")?,
            ))))
        })
        .unwrap();
        (components, behaviors, ui)
    }
    fn fixture(registry: &ComponentRegistry) -> (PreparedPlaySession, Arc<ModelAsset>) {
        let mut scene = SceneDocument {
            version: 1,
            objects: Vec::new(),
        }
        .load(registry, 16)
        .unwrap();
        for (name, position) in [
            ("player", [0., 1.5, 0.]),
            ("floor", [0., -0.1, 0.]),
            ("angular", [3., 1., 0.]),
            ("source", [10., 3., 0.]),
            ("button", [0.; 3]),
            ("animated", [4., 0., 0.]),
        ] {
            let node = scene
                .graph
                .spawn(
                    None,
                    Transform {
                        translation: glam::Vec3::from_array(position),
                        ..Default::default()
                    },
                )
                .unwrap();
            scene.identify(node, ObjectId(name.into())).unwrap();
        }
        let player = scene.resolve(&ObjectId("player".into())).unwrap();
        scene
            .graph
            .insert_component(
                player,
                voxy_gameplay::CharacterBody {
                    half_extents: [0.1; 3],
                    speed: 1.,
                    gravity: -9.8,
                    jump_speed: 3.,
                },
            )
            .unwrap();
        let floor = scene.resolve(&ObjectId("floor".into())).unwrap();
        scene
            .graph
            .insert_component(
                floor,
                voxy_gameplay::BoxCollider {
                    half_extents: [5., 0.1, 5.],
                },
            )
            .unwrap();
        let angular = scene.resolve(&ObjectId("angular".into())).unwrap();
        scene
            .graph
            .insert_component(
                angular,
                voxy_gameplay::AngularMotion {
                    axis: [0., 1., 0.],
                    radians_per_second: 0.73,
                },
            )
            .unwrap();
        let source = scene.resolve(&ObjectId("source".into())).unwrap();
        scene
            .graph
            .insert_component(
                source,
                voxy_gameplay::LiquidSource {
                    pulses: vec![voxy_gameplay::LiquidPulse {
                        start_s: 0.,
                        duration_s: 1.,
                        volume_m3: 0.00001,
                        speed_m_s: 1.,
                    }],
                    density_kg_m3: 1000.,
                    particle_volume_m3: 0.00001,
                    nozzle_radius_m: 0.001,
                    direction: [1., 0., 0.],
                    material_asset: "water".into(),
                },
            )
            .unwrap();
        let button = scene.resolve(&ObjectId("button".into())).unwrap();
        scene
            .graph
            .insert_component(
                button,
                voxy_gameplay::UiElement {
                    origin: [0.; 2],
                    size: [1.; 2],
                    color: [1.; 4],
                    layer: 0,
                    enabled: true,
                    action: Some("click".into()),
                    text: None,
                },
            )
            .unwrap();
        let model = Arc::new(
            ModelAsset::parse(
                include_bytes!("../../../voxy_render/examples/assets/root-pivot-turn.glb"),
                &[],
                voxy_model::ModelLimits::default(),
            )
            .unwrap(),
        );
        let animated = scene.resolve(&ObjectId("animated".into())).unwrap();
        scene
            .graph
            .insert_component(
                animated,
                crate::ModelInstance {
                    asset: voxy_assets::AssetId("turn".into()),
                },
            )
            .unwrap();
        scene
            .graph
            .insert_component(
                animated,
                crate::ModelAnimation {
                    root_motion_axes: [false; 3],
                    root_motion_rotation: false,
                    ..Default::default()
                },
            )
            .unwrap();
        let mut simulation = SceneSimulation::new(&scene.graph, limits().core.simulation).unwrap();
        simulation
            .attach(&mut scene.graph, button, Counter { ticks: 0 })
            .unwrap();
        let mut ui = voxy_gameplay::UiActionHandlers::new(&scene.graph, 8, 8, 8);
        ui.register_checkpointable("click".into(), Clicks(0))
            .unwrap();
        let liquid = voxy_gameplay::SceneLiquidRuntime::new(
            &scene.graph,
            vec![("water".into(), physics::liquid::Material::WATER)],
            physics::liquid::Config {
                gravity: [0.; 3],
                max_particles: 256,
                max_pairs: 200000,
                max_neighbor_checks: 2000000,
                max_substeps: 1024,
                ..Default::default()
            },
            8,
        )
        .unwrap();
        let play = PlaySession {
            playing: Some(scene.capture(registry).unwrap()),
            simulation_time: Instant::now(),
            simulation: Some(simulation),
            animations: Default::default(),
            angular_motion: Some(voxy_gameplay::AngularMotionBatch::new(&scene.graph, 8).unwrap()),
            simulation_ticks: 0,
            follow_camera: None,
            liquid_optics: vec![Some([0.2, 0.1, 0.05, 1.333])],
            liquid: Some(liquid),
            physics: Some(CharacterPhysics::new(&scene.graph, 8, 8)),
            player_input: voxy_gameplay::player_input().unwrap(),
            ui_actions: Some(ui),
        };
        (
            PreparedPlaySession {
                scene,
                play,
                camera: ViewportCamera::default(),
            },
            model,
        )
    }
    fn advance(game: &mut PreparedPlaySession, model: &Arc<ModelAsset>, dt: f64) {
        let PreparedPlaySession { scene, play, .. } = game;
        let play_simulation = play.simulation.as_mut().unwrap();
        for result in play
            .ui_actions
            .as_mut()
            .unwrap()
            .dispatch(&scene.graph, play_simulation.commands())
            .unwrap()
        {
            result.unwrap();
        }
        play_simulation
            .advance_with(&mut scene.graph, dt, |graph, step| {
                play.angular_motion
                    .as_mut()
                    .unwrap()
                    .fixed_step(graph, step)?;
                play.physics
                    .as_mut()
                    .unwrap()
                    .fixed_step(graph, &mut play.player_input, step)?;
                play.liquid.as_mut().unwrap().tick(graph, step, None)?;
                play.simulation_ticks += 1;
                Ok::<_, Box<dyn std::error::Error>>(())
            })
            .unwrap();
        let models = std::collections::BTreeMap::from([(
            voxy_assets::AssetId("turn".into()),
            model.clone(),
        )]);
        play.animations = play
            .animations
            .prepare_wall(&scene.graph, &models, dt)
            .unwrap();
        play.player_input.finish_frame();
    }
    fn capture(
        game: &PreparedPlaySession,
        model: &Arc<ModelAsset>,
        registry: &ComponentRegistry,
    ) -> PlaySessionCheckpoint {
        let view = SceneCheckpointView::new(
            &game.scene.graph,
            game.scene
                .graph
                .nodes()
                .map(|(node, _, _)| (game.scene.identity(node).unwrap().clone(), node)),
        )
        .unwrap();
        game.play
            .checkpoint(
                &view,
                registry,
                "test/project/resources/adapter-v1",
                &game.camera,
                |asset| Arc::ptr_eq(asset, model).then(|| "model-v1".into()),
                |clip, _| {
                    model
                        .animations
                        .iter()
                        .position(|candidate| Arc::ptr_eq(candidate, clip))
                        .map(|i| format!("clip-{i}"))
                },
                limits(),
            )
            .unwrap()
    }
    #[test]
    fn idle_session_checkpoint_restores_without_starting_simulation() {
        let (registry, behaviors, ui) = codecs();
        let (mut original, model) = fixture(&registry);
        original.play.playing = None;
        original.play.simulation = None;
        let saved = capture(&original, &model, &registry);
        let json = saved.to_json(limits().core.max_bytes).unwrap();
        let decoded = PlaySessionCheckpoint::from_json(&json, limits().core.max_bytes).unwrap();
        let (running_game, running_model) = fixture(&registry);
        let running = capture(&running_game, &running_model, &registry);
        for (authoring, scene, idle_scene, version) in [
            (running.authoring.clone(), None, saved.idle_scene.clone(), 2),
            (None, running.scene.clone(), saved.idle_scene.clone(), 2),
            (None, None, None, 2),
            (None, None, saved.idle_scene.clone(), 1),
            (
                running.authoring.clone(),
                running.scene.clone(),
                saved.idle_scene.clone(),
                1,
            ),
        ] {
            let mut invalid = saved.clone();
            invalid.authoring = authoring;
            invalid.scene = scene;
            invalid.idle_scene = idle_scene;
            invalid.version = version;
            assert!(invalid.to_json(limits().core.max_bytes).is_err());
            let malformed = serde_json::to_string(&invalid).unwrap();
            assert!(PlaySessionCheckpoint::from_json(&malformed, limits().core.max_bytes).is_err());
        }
        let restored = decoded
            .prepare(
                "test/project/resources/adapter-v1",
                &registry,
                &behaviors,
                &ui,
                |key| (key == "model-v1").then(|| model.clone()),
                |key| {
                    key.strip_prefix("clip-")
                        .and_then(|n| n.parse::<usize>().ok())
                        .and_then(|n| model.animations.get(n))
                        .map(|clip| (clip.clone(), None))
                },
                limits(),
            )
            .unwrap();
        assert!(restored.play.playing.is_none());
        assert!(restored.play.simulation.is_none());
        assert_eq!(
            capture(&restored, &model, &registry)
                .to_json(limits().core.max_bytes)
                .unwrap(),
            json
        );
    }
    #[test]
    fn full_play_record_rebuilds_scene_and_continues_120_frames_with_late_failure_atomicity() {
        let (registry, behaviors, ui) = codecs();
        let (mut original, model) = fixture(&registry);
        for _ in 0..17 {
            advance(&mut original, &model, 0.019);
        }
        original
            .play
            .player_input
            .event(voxy_gameplay::RIGHT, 0.375)
            .unwrap();
        original
            .play
            .player_input
            .event(voxy_gameplay::JUMP, 1.)
            .unwrap();
        original
            .play
            .player_input
            .event(voxy_gameplay::JUMP, 0.)
            .unwrap();
        let button = original.scene.resolve(&ObjectId("button".into())).unwrap();
        original
            .play
            .ui_actions
            .as_mut()
            .unwrap()
            .enqueue(
                &original.scene.graph,
                voxy_gameplay::UiActionEvent {
                    owner: button,
                    action: "click".into(),
                },
            )
            .unwrap();
        original
            .play
            .simulation
            .as_mut()
            .unwrap()
            .commands()
            .push(SceneCommand::SetActive(button, false))
            .unwrap();
        let saved = capture(&original, &model, &registry);
        let json = saved.to_json(limits().core.max_bytes).unwrap();
        let saved = PlaySessionCheckpoint::from_json(&json, limits().core.max_bytes).unwrap();
        let load = |saved: &PlaySessionCheckpoint, ui: &UiActionCheckpointRegistry| {
            saved.prepare(
                "test/project/resources/adapter-v1",
                &registry,
                &behaviors,
                ui,
                |key| (key == "model-v1").then(|| model.clone()),
                |key| {
                    model
                        .animations
                        .iter()
                        .enumerate()
                        .find(|(i, _)| key == format!("clip-{i}"))
                        .map(|(_, clip)| (clip.clone(), None))
                },
                limits(),
            )
        };
        assert!(load(&saved, &UiActionCheckpointRegistry::default()).is_err());
        assert_eq!(
            capture(&original, &model, &registry)
                .to_json(limits().core.max_bytes)
                .unwrap(),
            json
        );
        let mut resumed = load(&saved, &ui).unwrap();
        assert_ne!(
            resumed.scene.graph.identity(),
            original.scene.graph.identity()
        );
        assert_eq!(
            capture(&resumed, &model, &registry)
                .to_json(limits().core.max_bytes)
                .unwrap(),
            json
        );
        for i in 0..120 {
            for game in [&mut original, &mut resumed] {
                let button = game.scene.resolve(&ObjectId("button".into())).unwrap();
                game.scene.graph.set_active(button, true).unwrap();
                game.play
                    .ui_actions
                    .as_mut()
                    .unwrap()
                    .enqueue(
                        &game.scene.graph,
                        voxy_gameplay::UiActionEvent {
                            owner: button,
                            action: "click".into(),
                        },
                    )
                    .unwrap();
                if i == 30 {
                    game.play
                        .player_input
                        .event(voxy_gameplay::JUMP, 1.)
                        .unwrap();
                }
                if i == 31 {
                    game.play
                        .player_input
                        .event(voxy_gameplay::JUMP, 0.)
                        .unwrap();
                }
            }
            let dt = [0.003, 0.019, 0.011][i % 3];
            advance(&mut original, &model, dt);
            advance(&mut resumed, &model, dt);
            assert_eq!(
                capture(&original, &model, &registry)
                    .to_json(limits().core.max_bytes)
                    .unwrap(),
                capture(&resumed, &model, &registry)
                    .to_json(limits().core.max_bytes)
                    .unwrap()
            );
        }
        assert!(original.play.simulation_ticks > 120);
        assert!(
            original
                .play
                .liquid
                .as_ref()
                .unwrap()
                .liquid()
                .particles()
                .len()
                > 32
        );
        let mut bad = saved.clone();
        bad.compatibility = "wrong".into();
        assert!(load(&bad, &ui).is_err());
        bad = saved.clone();
        bad.optics[0] = Some([f32::NAN.to_bits(); 4]);
        assert!(load(&bad, &ui).is_err());
        bad = saved.clone();
        bad.liquid.as_mut().unwrap()[0] ^= 1;
        assert!(load(&bad, &ui).is_err());
        assert!(PlaySessionCheckpoint::from_json(&json, json.len() - 1).is_err());
    }
    #[test]
    fn full_play_resources_file_owns_models_and_guards_input_physics_and_command_queues() {
        exercise_full_play_resources(false, AcceptedFrameMode::Previous);
    }
    #[test]
    #[ignore = "requires real GPU; full Play/resource file with accepted GPU frame"]
    fn full_play_resources_gpu_stages_one_runtime_and_original_accepted_frame() {
        exercise_full_play_resources(true, AcceptedFrameMode::Previous);
    }
    #[test]
    #[ignore = "requires real GPU; shared runtime/render frame restored from source-free session"]
    fn full_play_resources_gpu_restores_shared_runtime_frame_identity() {
        exercise_full_play_resources(true, AcceptedFrameMode::Shared);
    }
    #[test]
    #[ignore = "requires real GPU; equal independent runtime/render frames remain distinct"]
    fn full_play_resources_gpu_keeps_equal_independent_frames_distinct() {
        exercise_full_play_resources(true, AcceptedFrameMode::EqualIndependent);
    }
    pub(crate) fn resource_restore_limits() -> crate::app::resource_checkpoint::RestoreLimits {
        use crate::app::resource_checkpoint::RestoreLimits;
        RestoreLimits {
            catalog: crate::import::catalog_checkpoint::Limits {
                assets: Default::default(),
                inputs: voxy_assets::ImportedArchiveLimits {
                    max_inputs: 34,
                    max_input_bytes: 32 * 1024 * 1024,
                    max_archive_bytes: 64 * 1024 * 1024,
                },
                max_bytes: 128 * 1024 * 1024,
                max_catalog_assets: 256,
                max_pending: 4,
                max_source_outputs: 256,
                max_source_edges: 8192,
            },
            max_bytes: 256 * 1024 * 1024,
            max_outputs: 128,
            max_queued: 4096,
            max_sources: 257,
            max_file_bytes: crate::import::SOURCE_BYTES,
        }
    }
    #[test]
    fn idle_play_origin_tracks_input_preview_state_and_mode_without_starting_a_game() {
        let (registry, _, _) = codecs();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = crate::App::new(&path, false).unwrap();
        app.tick().unwrap();
        app.begin_model_checkpoint().unwrap();
        let until = Instant::now() + std::time::Duration::from_secs(10);
        while !app.poll_model_checkpoint().unwrap() {
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        let view = SceneCheckpointView::new(
            &app.scene,
            app.object_ids
                .iter()
                .cloned()
                .zip(app.instances.iter().copied()),
        )
        .unwrap();
        let origin = app
            .observe_play_restore_origin(
                &view,
                &registry,
                "idle-test",
                resource_restore_limits(),
                limits(),
            )
            .unwrap();
        origin
            .validate(&app, &view, &registry, "idle-test")
            .unwrap();
        app.play.simulation_time += std::time::Duration::from_nanos(1);
        origin
            .validate(&app, &view, &registry, "idle-test")
            .unwrap();
        let token = app.scene.change_token();
        app.play
            .player_input
            .event(voxy_gameplay::RIGHT, 0.5)
            .unwrap();
        assert_eq!(token, app.scene.change_token());
        assert!(
            origin
                .validate(&app, &view, &registry, "idle-test")
                .is_err()
        );
        let origin = app
            .observe_play_restore_origin(
                &view,
                &registry,
                "idle-test",
                resource_restore_limits(),
                limits(),
            )
            .unwrap();
        app.play.physics = Some(CharacterPhysics::new(&app.scene, 8, 8));
        assert!(
            origin
                .validate(&app, &view, &registry, "idle-test")
                .is_err()
        );
        let origin = app
            .observe_play_restore_origin(
                &view,
                &registry,
                "idle-test",
                resource_restore_limits(),
                limits(),
            )
            .unwrap();
        origin
            .validate(&app, &view, &registry, "idle-test")
            .unwrap();
        app.play.physics = Some(app.play.physics.take().unwrap().with_depenetration(true));
        assert!(
            origin
                .validate(&app, &view, &registry, "idle-test")
                .is_err()
        );
        let origin = app
            .observe_play_restore_origin(
                &view,
                &registry,
                "idle-test",
                resource_restore_limits(),
                limits(),
            )
            .unwrap();
        app.play.playing = Some(SceneDocument {
            version: 1,
            objects: vec![],
        });
        assert!(
            origin
                .validate(&app, &view, &registry, "idle-test")
                .is_err()
        );
        assert!(
            app.observe_play_restore_origin(
                &view,
                &registry,
                "idle-test",
                resource_restore_limits(),
                limits()
            )
            .is_err()
        );
        app.play.playing = None;
        assert!(app.play.simulation.is_none());
        assert_eq!(token, app.scene.change_token());
        app.stop_workers().unwrap();
    }
    enum AcceptedFrameMode {
        Previous,
        Shared,
        EqualIndependent,
    }
    fn exercise_full_play_resources(with_gpu: bool, frame_mode: AcceptedFrameMode) {
        let shared_frame = matches!(frame_mode, AcceptedFrameMode::Shared);
        let equal_independent = matches!(frame_mode, AcceptedFrameMode::EqualIndependent);
        let (registry, behaviors, ui) = codecs();
        let registry = Arc::new(registry);
        let (mut original, model) = fixture(&registry);
        for _ in 0..17 {
            advance(&mut original, &model, 0.019);
        }
        let source =
            std::env::temp_dir().join(format!("voxy-play-resource-source-{}", std::process::id()));
        let target_root =
            std::env::temp_dir().join(format!("voxy-play-resource-target-{}", std::process::id()));
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&target_root).unwrap();
        std::fs::write(
            source.join("quad.obj"),
            include_bytes!("../../../voxy_render/examples/assets/quad.obj"),
        )
        .unwrap();
        let mut app = crate::App::new(&source.join("quad.obj"), false).unwrap();
        app.tick().unwrap();
        app.begin_model_checkpoint().unwrap();
        let until = Instant::now() + std::time::Duration::from_secs(10);
        while !app.poll_model_checkpoint().unwrap() {
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        let id = voxy_assets::AssetId("turn".into());
        let value = crate::import::EditorAsset {
            mesh: model
                .scene_meshes_frame(&model.bind_frame().unwrap())
                .unwrap()[0]
                .clone(),
            animated: Some(model.clone()),
            lod: None,
            skinned_lod: None,
            nodes: vec![],
            images: vec![],
        };
        let value = voxy_assets::ImportInputs::new(0, 0)
            .finish(value, |_, _| unreachable!())
            .unwrap();
        let ticket = app.catalog.request(id.clone()).unwrap();
        app.catalog
            .complete_observed(&mut app.sources, &ticket, Ok(value))
            .unwrap();
        app.available.insert(id.clone());
        let resource_limits = resource_restore_limits();
        let bindings: Vec<_> = original
            .scene
            .graph
            .nodes()
            .map(|(node, _, _)| (original.scene.identity(node).unwrap().clone(), node))
            .collect();
        std::mem::swap(&mut app.scene, &mut original.scene.graph);
        std::mem::swap(&mut app.play, &mut original.play);
        std::mem::swap(&mut app.camera, &mut original.camera);
        std::fs::write(
            source.join("tone.wav"),
            crate::audio_session::checkpoint::tests::wav(),
        )
        .unwrap();
        let sound = bindings.iter().find(|(id, _)| id.0 == "button").unwrap().1;
        app.scene
            .insert_component(
                sound,
                voxy_gameplay::AudioSource {
                    asset: "tone.wav".into(),
                    import_settings: None,
                    bus: 0,
                    gain: 0.8,
                    looping: true,
                    spatial: false,
                    near: 1.,
                    far: 5.,
                },
            )
            .unwrap();
        app.audio.set_play(
            crate::audio_play::AudioPlay::prepare(
                &app.scene,
                &app.authoring.authoring_project,
                None,
            )
            .unwrap(),
        );
        app.audio
            .play_mut()
            .unwrap()
            .advance(&app.scene, 7)
            .unwrap();
        let input_viewport = [128.; 2];
        let input_snapshot =
            voxy_gameplay::extract_scene_ui(&app.scene, input_viewport, 128).unwrap();
        assert!(
            app.game_ui_input
                .admit_presented(&app.scene, input_viewport, Some(&input_snapshot))
                .unwrap()
        );
        app.game_ui_input
            .key(
                &app.scene,
                input_viewport,
                KeyCode::Tab,
                ElementState::Pressed,
                false,
                false,
            )
            .unwrap();
        assert!(
            app.game_ui_input
                .key(
                    &app.scene,
                    input_viewport,
                    KeyCode::Enter,
                    ElementState::Pressed,
                    false,
                    false
                )
                .unwrap()
                .1
                .is_none()
        );
        app.resume_model_checkpoint().unwrap();
        app.begin_restore_barrier(8 * 1024 * 1024).unwrap();
        let until = Instant::now() + std::time::Duration::from_secs(10);
        while !app
            .poll_restore_barrier(|node| {
                bindings
                    .iter()
                    .find(|(_, n)| *n == node)
                    .map(|(id, _)| id.clone())
            })
            .unwrap()
        {
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        assert!(app.restore_play_frozen());
        let view = SceneCheckpointView::new(&app.scene, bindings.clone()).unwrap();
        let mut rejected_ui = crate::ui_live::checkpoint::tests::limits();
        rejected_ui.max_bytes = 1;
        let captured_ticks = app.play.simulation_ticks;
        assert!(
            app.coordinated_play_checkpoint(
                &view,
                &registry,
                "play-resource-test-v1",
                resource_limits,
                limits(),
                rejected_ui,
            )
            .is_err()
        );
        assert!(app.restore_play_frozen());
        assert_eq!(app.play.simulation_ticks, captured_ticks);
        let mut coordinated = app
            .coordinated_play_checkpoint(
                &view,
                &registry,
                "play-resource-test-v1",
                resource_limits,
                limits(),
                crate::ui_live::checkpoint::tests::limits(),
            )
            .unwrap();
        let file_limits = crate::app::restore_barrier::file::CoordinatedFileLimits {
            max_bytes: 512 * 1024 * 1024,
            max_game_bytes: resource_limits.max_bytes,
            ui: crate::ui_live::checkpoint::tests::limits(),
            max_audio_bytes: 8 * 1024 * 1024,
            max_input_bytes: 10000,
        };
        let combined_path = target_root.join("game.voxycgr");
        coordinated.save_file(&combined_path, file_limits).unwrap();
        let original_combined = std::fs::read(&combined_path).unwrap();
        coordinated = crate::app::restore_barrier::CoordinatedPlayCheckpoint::read_file(
            &combined_path,
            file_limits,
        )
        .unwrap();
        assert_eq!(
            coordinated.to_bytes(file_limits).unwrap(),
            original_combined
        );
        let mut header_only = file_limits;
        header_only.max_bytes = 71;
        assert!(
            coordinated
                .to_bytes(header_only)
                .unwrap_err()
                .to_string()
                .contains("coordinated file byte budget")
        );
        let mut first_only = file_limits;
        first_only.max_bytes = 72
            + coordinated
                .game
                .to_bytes(resource_limits.max_bytes)
                .unwrap()
                .len();
        assert!(
            coordinated
                .to_bytes(first_only)
                .unwrap_err()
                .to_string()
                .contains("coordinated file byte budget")
        );
        let mut too_small = file_limits;
        too_small.max_bytes = original_combined.len() - 1;
        assert!(coordinated.save_file(&combined_path, too_small).is_err());
        assert_eq!(std::fs::read(&combined_path).unwrap(), original_combined);
        assert!(
            crate::app::restore_barrier::CoordinatedPlayCheckpoint::read_file(
                &combined_path,
                too_small
            )
            .is_err()
        );
        let mut corrupt = original_combined.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(
            crate::app::restore_barrier::CoordinatedPlayCheckpoint::from_bytes(
                &corrupt,
                file_limits
            )
            .is_err()
        );
        let mut trailing = original_combined.clone();
        trailing.push(0);
        assert!(
            crate::app::restore_barrier::CoordinatedPlayCheckpoint::from_bytes(
                &trailing,
                file_limits
            )
            .is_err()
        );
        let reseal = |bytes: &mut Vec<u8>| {
            let mut hash = blake3::Hasher::new();
            hash.update(&bytes[8..40]);
            hash.update(&bytes[72..]);
            bytes[40..72].copy_from_slice(hash.finalize().as_bytes());
        };
        let mut invalid_length = original_combined.clone();
        invalid_length[8..16].copy_from_slice(&u64::MAX.to_le_bytes());
        reseal(&mut invalid_length);
        assert!(
            crate::app::restore_barrier::CoordinatedPlayCheckpoint::from_bytes(
                &invalid_length,
                file_limits
            )
            .is_err()
        );
        let mut invalid_utf8 = original_combined.clone();
        *invalid_utf8.last_mut().unwrap() = 255;
        reseal(&mut invalid_utf8);
        assert!(
            crate::app::restore_barrier::CoordinatedPlayCheckpoint::from_bytes(
                &invalid_utf8,
                file_limits
            )
            .is_err()
        );
        let mut section_limit = file_limits;
        section_limit.max_game_bytes = coordinated
            .game
            .to_bytes(resource_limits.max_bytes)
            .unwrap()
            .len()
            - 1;
        assert!(
            crate::app::restore_barrier::CoordinatedPlayCheckpoint::from_bytes(
                &original_combined,
                section_limit
            )
            .is_err()
        );
        crate::import::asset_checkpoint::retain_evidence(
            "coordinated-game.voxycgr",
            &original_combined,
        );
        assert_eq!(
            coordinated
                .game
                .to_bytes(resource_limits.max_bytes)
                .unwrap(),
            app.capture_play_resources(
                &view,
                &registry,
                "play-resource-test-v1",
                resource_limits,
                limits()
            )
            .unwrap()
            .to_bytes(resource_limits.max_bytes)
            .unwrap(),
        );
        assert!(
            !coordinated
                .ui
                .to_bytes(crate::ui_live::checkpoint::tests::limits())
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            coordinated.audio.to_json(8 * 1024 * 1024).unwrap(),
            app.restore_barrier_snapshots()
                .unwrap()
                .1
                .to_json(8 * 1024 * 1024)
                .unwrap()
        );
        let options = || crate::app::restore_barrier::CoordinatedPrepare {
            compatibility: "play-resource-test-v1",
            registry: &registry,
            behaviors: &behaviors,
            ui_actions: &ui,
            root: &target_root,
            resources: resource_limits,
            play: limits(),
            ui: crate::ui_live::checkpoint::tests::limits(),
            max_audio_bytes: 8 * 1024 * 1024,
            restored_ui_cache: (0, 0, 0, 0),
        };
        let mut prepared_group = coordinated.prepare(options()).unwrap();
        let prepared_game = prepared_group.game.as_ref().unwrap();
        assert_ne!(
            prepared_game.game.scene.graph.identity(),
            app.scene.identity()
        );
        assert_eq!(
            prepared_game.game.play.simulation_ticks,
            app.play.simulation_ticks
        );
        assert!(prepared_group.ui.is_some() && prepared_group.audio.is_some());
        assert!(prepared_group.project.is_some());
        let target_input = prepared_group.ui_input.as_mut().unwrap();
        assert_eq!(
            target_input
                .checkpoint_historical(&prepared_game.game.scene.checkpoint_view(), 10000)
                .unwrap()
                .to_json(10000)
                .unwrap(),
            coordinated.ui_input.to_json(10000).unwrap()
        );
        let target_snapshot =
            voxy_gameplay::extract_scene_ui(&prepared_game.game.scene.graph, input_viewport, 128)
                .unwrap();
        assert!(
            target_input
                .admit_presented(
                    &prepared_game.game.scene.graph,
                    input_viewport,
                    Some(&target_snapshot)
                )
                .unwrap()
        );
        let release = target_input
            .key(
                &prepared_game.game.scene.graph,
                input_viewport,
                KeyCode::Enter,
                ElementState::Released,
                false,
                false,
            )
            .unwrap()
            .1
            .unwrap();
        assert_eq!(
            release.owner,
            prepared_game
                .game
                .scene
                .resolve(&ObjectId("button".into()))
                .unwrap()
        );
        assert_eq!(release.action, "click");
        assert!(
            target_input
                .key(
                    &prepared_game.game.scene.graph,
                    input_viewport,
                    KeyCode::Enter,
                    ElementState::Released,
                    false,
                    false
                )
                .unwrap()
                .1
                .is_none()
        );

        let source_model = prepared_game.resources.resources.models[0].clone();
        let prepared_capture = capture(&prepared_game.game, &source_model, &registry);
        assert!(
            !prepared_capture
                .to_json(limits().core.max_bytes)
                .unwrap()
                .is_empty()
        );
        for worker in prepared_group.discard() {
            worker.join().unwrap();
        }
        let bad_audio = crate::audio_session::checkpoint::AudioSessionCheckpoint::from_json(
            r#"{"version":2,"play":null}"#,
            1024,
        )
        .unwrap();
        let original_audio = std::mem::replace(&mut coordinated.audio, bad_audio);
        // Audio semantic admission is deliberately last, after model workers
        // and UI owners exist. Failure drops only the detached candidate.
        assert!(coordinated.prepare(options()).is_err());
        coordinated.audio = original_audio;
        let retry = coordinated.prepare(options()).unwrap();
        for worker in retry.discard() {
            worker.join().unwrap();
        }
        assert_eq!(app.play.simulation_ticks, captured_ticks);
        assert!(app.restore_play_frozen());
        let (_, guard) = app
            .observe_play_resources(
                &view,
                &registry,
                "play-resource-test-v1",
                resource_limits,
                limits(),
            )
            .unwrap();
        guard
            .validate(&app, &view, &registry, "play-resource-test-v1")
            .unwrap();
        let scene_change = app.scene.change_token();
        drop(view);
        let coordinated_origin = app
            .observe_coordinated_origin(
                &bindings,
                &registry,
                "play-resource-test-v1",
                resource_limits,
                limits(),
                crate::ui_live::checkpoint::tests::limits(),
            )
            .unwrap();
        let guarded = coordinated
            .prepare_guarded(options(), coordinated_origin)
            .unwrap();
        guarded
            .validate_origin(&mut app, &bindings, &registry, "play-resource-test-v1")
            .unwrap();
        app.game_ui_input.cancel();
        assert!(
            guarded
                .validate_origin(&mut app, &bindings, &registry, "play-resource-test-v1")
                .is_err()
        );
        app.game_ui_input = coordinated
            .ui_input
            .prepare_historical(
                &SceneCheckpointView::new(&app.scene, bindings.clone()).unwrap(),
                128,
                10000,
            )
            .unwrap();
        guarded
            .validate_origin(&mut app, &bindings, &registry, "play-resource-test-v1")
            .unwrap();
        let pending_input = app.play.player_input.checkpoint();
        app.play
            .player_input
            .event(voxy_gameplay::RIGHT, 0.375)
            .unwrap();
        assert!(
            guarded
                .validate_origin(&mut app, &bindings, &registry, "play-resource-test-v1")
                .is_err()
        );
        app.play
            .player_input
            .restore_checkpoint(&pending_input)
            .unwrap();
        guarded
            .validate_origin(&mut app, &bindings, &registry, "play-resource-test-v1")
            .unwrap();
        app.audio
            .set_output_mode(crate::audio_session::AudioOutputMode::Native);
        assert!(
            guarded
                .validate_origin(&mut app, &bindings, &registry, "play-resource-test-v1")
                .is_err()
        );
        app.audio
            .set_output_mode(crate::audio_session::AudioOutputMode::Offline);
        guarded
            .validate_origin(&mut app, &bindings, &registry, "play-resource-test-v1")
            .unwrap();
        for worker in guarded.discard() {
            worker.join().unwrap();
        }
        let frame = app
            .play
            .simulation
            .as_mut()
            .unwrap()
            .advance(&mut app.scene, 1e-9)
            .unwrap();
        assert_eq!(frame.time.steps, 0);
        assert_eq!(scene_change, app.scene.change_token());
        let view = SceneCheckpointView::new(&app.scene, bindings.clone()).unwrap();
        assert!(
            guard
                .validate(&app, &view, &registry, "play-resource-test-v1")
                .is_err()
        );
        let (_, guard) = app
            .observe_play_resources(
                &view,
                &registry,
                "play-resource-test-v1",
                resource_limits,
                limits(),
            )
            .unwrap();
        app.play.simulation_time += std::time::Duration::from_nanos(1);
        assert!(
            guard
                .validate(&app, &view, &registry, "play-resource-test-v1")
                .is_err()
        );
        let (_, guard) = app
            .observe_play_resources(
                &view,
                &registry,
                "play-resource-test-v1",
                resource_limits,
                limits(),
            )
            .unwrap();
        app.play
            .player_input
            .event(voxy_gameplay::RIGHT, 0.375)
            .unwrap();
        assert_eq!(scene_change, app.scene.change_token());
        assert!(
            guard
                .validate(&app, &view, &registry, "play-resource-test-v1")
                .is_err()
        );
        let (_, guard) = app
            .observe_play_resources(
                &view,
                &registry,
                "play-resource-test-v1",
                resource_limits,
                limits(),
            )
            .unwrap();
        app.play.physics = Some(app.play.physics.take().unwrap().with_depenetration(true));
        assert_eq!(scene_change, app.scene.change_token());
        assert!(
            guard
                .validate(&app, &view, &registry, "play-resource-test-v1")
                .is_err()
        );
        let (_, guard) = app
            .observe_play_resources(
                &view,
                &registry,
                "play-resource-test-v1",
                resource_limits,
                limits(),
            )
            .unwrap();
        let button = view.resolve(&ObjectId("button".into())).unwrap();
        app.play
            .ui_actions
            .as_mut()
            .unwrap()
            .enqueue(
                &app.scene,
                voxy_gameplay::UiActionEvent {
                    owner: button,
                    action: "click".into(),
                },
            )
            .unwrap();
        assert!(
            guard
                .validate(&app, &view, &registry, "play-resource-test-v1")
                .is_err()
        );
        let (_, guard) = app
            .observe_play_resources(
                &view,
                &registry,
                "play-resource-test-v1",
                resource_limits,
                limits(),
            )
            .unwrap();
        app.play
            .simulation
            .as_mut()
            .unwrap()
            .commands()
            .push(SceneCommand::SetActive(button, false))
            .unwrap();
        assert!(
            guard
                .validate(&app, &view, &registry, "play-resource-test-v1")
                .is_err()
        );
        let (mut saved, guard) = app
            .observe_play_resources(
                &view,
                &registry,
                "play-resource-test-v1",
                resource_limits,
                limits(),
            )
            .unwrap();
        guard
            .validate(&app, &view, &registry, "play-resource-test-v1")
            .unwrap();
        struct Offscreen {
            device: wgpu::Device,
            queue: wgpu::Queue,
            renderer: voxy_render::SceneRenderer,
            cache: crate::gpu_model::ResidencyCache,
            models: crate::ModelGraphics,
            animated: crate::animated_models::AnimatedModels,
            matrix: glam::Mat4,
            reference: Vec<u8>,
        }
        let gpu_context = if with_gpu {
            let instance = voxy_render::GraphicsOptions::default().create_instance();
            let adapter =
                pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
            assert_ne!(adapter.get_info().device_type, wgpu::DeviceType::Cpu);
            println!("FULL PLAY GPU {:?}", adapter.get_info());
            let (device, queue) =
                pollster::block_on(adapter.request_device(&Default::default())).unwrap();
            let renderer =
                voxy_render::SceneRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
            let mut cache = crate::gpu_model::ResidencyCache::default();
            let publication = app.catalog.snapshot(&id).unwrap();
            let mut models = crate::ModelGraphics::upload_on(
                &renderer,
                &device,
                &queue,
                publication.value(),
                &mut cache,
            )
            .unwrap();
            models.publication = Some(publication.clone());
            let owner = view.resolve(&ObjectId("animated".into())).unwrap();
            let frame = if shared_frame || equal_independent {
                let frame = app.play.animations.frame(owner, &model).unwrap();
                if equal_independent {
                    Arc::new((*frame).clone())
                } else {
                    frame
                }
            } else {
                Arc::new(model.bind_frame().unwrap())
            };
            let mut animated = crate::animated_models::AnimatedModels::new(&renderer).unwrap();
            animated
                .restore_accepted_owner(
                    &renderer,
                    &device,
                    &queue,
                    crate::animated_models::Request {
                        publication: Some(publication.clone()),
                        owner,
                        model: model.clone(),
                        settings: app
                            .scene
                            .component::<crate::ModelAnimation>(owner)
                            .unwrap()
                            .unwrap()
                            .clone(),
                        frame: Some(frame.clone()),
                        lod: None,
                        textures: models.animated_textures.clone(),
                        texture_storage: models._images.clone(),
                    },
                    731,
                    models.geometry_bytes(),
                    cache.geometry_budget,
                )
                .unwrap();
            let mesh = &model.scene_meshes_frame(&frame).unwrap()[0];
            let points: Vec<_> = mesh
                .vertices()
                .iter()
                .map(|v| glam::Vec3::from_array(v.position))
                .collect();
            let indices = mesh.indices();
            let edge = (points[indices[1] as usize] - points[indices[0] as usize]).normalize();
            let normal = edge
                .cross(points[indices[2] as usize] - points[indices[0] as usize])
                .normalize();
            let basis = glam::Mat4::from_mat3(
                glam::Mat3::from_cols(edge, normal.cross(edge), normal).transpose(),
            );
            let mut min = glam::Vec3::splat(f32::INFINITY);
            let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
            for point in points {
                let p = basis.transform_point3(point);
                min = min.min(p);
                max = max.max(p);
            }
            let matrix = glam::Mat4::from_translation(glam::Vec3::new(0., 0., 0.5))
                * glam::Mat4::from_scale(glam::Vec3::splat(1.4 / (max - min).max_element()))
                * glam::Mat4::from_translation(-(min + max) * 0.5)
                * basis;
            let reference = crate::animated_models::tests::pixels(
                &renderer,
                &device,
                &queue,
                animated.geometries(owner).unwrap().collect(),
                matrix,
            );
            assert!(
                reference
                    .chunks_exact(4)
                    .any(|p| p[..3].iter().any(|v| *v != 0))
            );
            let gpu_sources = crate::app::resource_checkpoint::play::PlayGpuSources {
                shared_frames: app.shared_render_frame_roles(&animated, &view).unwrap(),
                preview_shared_gpu_owner: app
                    .preview_render_frame_role(&animated, |node| view.identity(node).cloned())
                    .unwrap(),
                live: crate::gpu_source_bindings::LiveSources {
                    models: vec![(id.clone(), publication)],
                    owners: animated.source_publication_owners().unwrap(),
                },
                accepted: animated
                    .checkpoint_accepted_frames(
                        |node| view.identity(node).cloned(),
                        limits().core.max_bytes,
                    )
                    .unwrap(),
            };
            assert_eq!(gpu_sources.shared_frames.len(), usize::from(shared_frame));
            saved = app
                .capture_play_resources_with_gpu(
                    &view,
                    &registry,
                    "play-resource-test-v1",
                    resource_limits,
                    limits(),
                    Some(gpu_sources),
                )
                .unwrap();
            Some(Offscreen {
                device,
                queue,
                renderer,
                cache,
                models,
                animated,
                matrix,
                reference,
            })
        } else {
            None
        };
        let current_group = crate::app::restore_barrier::CoordinatedPlayCheckpoint {
            game: saved,
            ui: app.ui_live.checkpoint(&view, file_limits.ui).unwrap(),
            audio: app.restore_barrier_snapshots().unwrap().1.clone(),
            ui_input: app
                .game_ui_input
                .checkpoint_historical(&view, file_limits.max_input_bytes)
                .unwrap(),
        };
        let current_path = target_root.join("accepted-game.voxycgr");
        current_group.save_file(&current_path, file_limits).unwrap();
        crate::import::asset_checkpoint::retain_evidence(
            "coordinated-accepted-game.voxycgr",
            &std::fs::read(&current_path).unwrap(),
        );
        let saved = current_group.game;
        let bytes = saved.to_bytes(resource_limits.max_bytes).unwrap();
        crate::import::asset_checkpoint::retain_evidence("full-play-resources.voxypgr", &bytes);
        let file = target_root.join("state.voxypgr");
        std::fs::write(&file, &bytes).unwrap();
        std::fs::remove_dir_all(&source).unwrap();
        // No source model/manifest reread: the entire group prepares against an
        // empty target root after the source project directory is gone.
        let source_free_checkpoint =
            crate::app::restore_barrier::CoordinatedPlayCheckpoint::read_file(
                &current_path,
                file_limits,
            )
            .unwrap();
        if with_gpu {
            for roles in [
                serde_json::json!([["animated", "animated"], ["animated", "animated"]]),
                serde_json::json!([["missing-runtime", "animated"]]),
                serde_json::json!([["animated", "missing-gpu"]]),
            ] {
                let mut malformed =
                    crate::app::restore_barrier::CoordinatedPlayCheckpoint::read_file(
                        &current_path,
                        file_limits,
                    )
                    .unwrap();
                let mut value = serde_json::to_value(&malformed.game).unwrap();
                value["shared_frames"] = roles;
                malformed.game = serde_json::from_value(value).unwrap();
                assert!(
                    malformed.prepare(options()).is_err(),
                    "invalid shared-frame roles were admitted"
                );
            }
            if matches!(frame_mode, AcceptedFrameMode::Previous) {
                let mut malformed =
                    crate::app::restore_barrier::CoordinatedPlayCheckpoint::read_file(
                        &current_path,
                        file_limits,
                    )
                    .unwrap();
                let mut value = serde_json::to_value(&malformed.game).unwrap();
                value["shared_frames"] = serde_json::json!([["animated", "animated"]]);
                malformed.game = serde_json::from_value(value).unwrap();
                assert!(
                    malformed.prepare(options()).is_err(),
                    "different accepted states were joined"
                );
            }
        }
        let mut source_free_group = source_free_checkpoint.prepare(options()).unwrap();
        assert_eq!(
            source_free_group.game.as_ref().unwrap().gpu.is_some(),
            with_gpu
        );
        assert_eq!(
            source_free_group
                .game
                .as_ref()
                .unwrap()
                .game
                .play
                .simulation_ticks,
            captured_ticks
        );
        let source_audio_json = source_free_checkpoint
            .audio
            .to_json(file_limits.max_audio_bytes)
            .unwrap();
        assert!(
            serde_json::from_str::<serde_json::Value>(&source_audio_json).unwrap()["play"]
                .is_object()
        );
        let mut restored_audio = crate::audio_session::AudioSession::default();
        restored_audio.checkpoint_publication().unwrap().publish(
            match source_free_group.audio.take().unwrap() {
                crate::app::restore_barrier::PreparedCoordinatedAudio::Offline(audio) => *audio,
                crate::app::restore_barrier::PreparedCoordinatedAudio::Native(_) => {
                    panic!("offline fixture")
                }
            },
        );
        let restored_scene = &source_free_group.game.as_ref().unwrap().game.scene;
        assert_eq!(
            restored_audio.play().unwrap().frames,
            app.audio.play().unwrap().frames
        );
        // Advance independent native owners after the source project is removed.
        for step in 0..120 {
            let ticks = [0, 1, 2, 5][step % 4];
            app.audio
                .play_mut()
                .unwrap()
                .advance(&app.scene, ticks)
                .unwrap();
            restored_audio
                .play_mut()
                .unwrap()
                .advance(&restored_scene.graph, ticks)
                .unwrap();
            assert_eq!(
                restored_audio.play().unwrap().frames,
                app.audio.play().unwrap().frames
            );
            assert_eq!(
                restored_audio.play().unwrap().peak.to_bits(),
                app.audio.play().unwrap().peak.to_bits()
            );
        }
        restored_audio.request_checkpoint().unwrap();
        let deadline = Instant::now() + std::time::Duration::from_secs(10);
        let restored_audio_state = loop {
            if let Some(saved) = restored_audio
                .poll_checkpoint(
                    |node| restored_scene.identity(node).cloned(),
                    file_limits.max_audio_bytes,
                )
                .unwrap()
            {
                break saved;
            }
            restored_audio
                .play_mut()
                .unwrap()
                .poll_reload(&restored_scene.graph)
                .unwrap();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        let original_audio_state = app
            .audio
            .poll_checkpoint(
                |node| {
                    bindings
                        .iter()
                        .find(|(_, n)| *n == node)
                        .map(|(id, _)| id.clone())
                },
                file_limits.max_audio_bytes,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::to_value(restored_audio_state).unwrap()["play"]["playback"],
            serde_json::to_value(original_audio_state).unwrap()["play"]["playback"]
        );
        restored_audio.close_play_audio();
        assert!(restored_audio.join_workers().is_empty());
        for worker in source_free_group.discard() {
            worker.join().unwrap();
        }

        let saved = crate::app::restore_barrier::CoordinatedPlayCheckpoint::read_file(
            &current_path,
            file_limits,
        )
        .unwrap()
        .game;
        assert_eq!(
            saved.to_bytes(resource_limits.max_bytes).unwrap(),
            std::fs::read(&file).unwrap()
        );
        assert!(
            saved
                .prepare(
                    "play-resource-test-v1",
                    &registry,
                    &behaviors,
                    &UiActionCheckpointRegistry::default(),
                    &target_root,
                    resource_limits,
                    limits()
                )
                .is_err()
        );
        let mut resumed = saved
            .prepare(
                "play-resource-test-v1",
                &registry,
                &behaviors,
                &ui,
                &target_root,
                resource_limits,
                limits(),
            )
            .unwrap();
        let restored_model = resumed
            .resources
            .resources
            .catalog
            .snapshot(&id)
            .unwrap()
            .value()
            .animated
            .as_ref()
            .unwrap()
            .clone();
        assert!(!Arc::ptr_eq(&restored_model, &model));
        assert!(Arc::ptr_eq(
            &restored_model,
            &resumed.resources.resources.models[0]
        ));
        assert!(Arc::ptr_eq(
            &restored_model,
            &resumed.game.play.animations.checkpoint_model_roots()[0]
        ));
        assert_ne!(resumed.game.scene.graph.identity(), app.scene.identity());
        let mut staged_gpu = None;
        if let Some(context) = &gpu_context {
            let live = context.models.geometry_bytes() + context.animated.allocation_bytes();
            let target_gpu = || crate::gpu_model::batch::UploadTarget {
                renderer: &context.renderer,
                device: &context.device,
                queue: &context.queue,
                cache: &context.cache,
                live_geometry: live,
                replaced_geometry: live,
            };
            let outer = context
                .device
                .push_error_scope(wgpu::ErrorFilter::Validation);
            let source_before = app
                .capture_play_resources(
                    &view,
                    &registry,
                    "play-resource-test-v1",
                    resource_limits,
                    limits(),
                )
                .unwrap()
                .to_bytes(resource_limits.max_bytes)
                .unwrap();
            let cache_before = context.cache.state();
            let mut constrained = context.cache.fork_for_upload();
            constrained.geometry_budget = live;
            let rejected = saved
                .prepare(
                    "play-resource-test-v1",
                    &registry,
                    &behaviors,
                    &ui,
                    &target_root,
                    resource_limits,
                    limits(),
                )
                .unwrap();
            let (_, rejected_origin) = app
                .observe_play_resources(
                    &view,
                    &registry,
                    "play-resource-test-v1",
                    resource_limits,
                    limits(),
                )
                .unwrap();
            assert!(
                rejected
                    .stage_gpu(
                        crate::gpu_model::batch::UploadTarget {
                            cache: &constrained,
                            ..target_gpu()
                        },
                        rejected_origin
                    )
                    .is_err()
            );
            let cancelled = saved
                .prepare(
                    "play-resource-test-v1",
                    &registry,
                    &behaviors,
                    &ui,
                    &target_root,
                    resource_limits,
                    limits(),
                )
                .unwrap();
            let (_, cancelled_origin) = app
                .observe_play_resources(
                    &view,
                    &registry,
                    "play-resource-test-v1",
                    resource_limits,
                    limits(),
                )
                .unwrap();
            drop(cancelled.stage_gpu(target_gpu(), cancelled_origin).unwrap());
            assert_eq!(context.cache.state(), cache_before);
            assert_eq!(
                source_before,
                app.capture_play_resources(
                    &view,
                    &registry,
                    "play-resource-test-v1",
                    resource_limits,
                    limits()
                )
                .unwrap()
                .to_bytes(resource_limits.max_bytes)
                .unwrap()
            );
            drop(view);
            let coordinated = crate::app::restore_barrier::CoordinatedPlayCheckpoint::read_file(
                &current_path,
                file_limits,
            )
            .unwrap();
            for incomplete in [false, true] {
                let origin = app
                    .observe_coordinated_origin(
                        &bindings,
                        &registry,
                        "play-resource-test-v1",
                        resource_limits,
                        limits(),
                        file_limits.ui,
                    )
                    .unwrap();
                let mut candidate = coordinated.prepare_guarded(options(), origin).unwrap();
                if incomplete {
                    candidate.ui_input = None;
                    assert!(candidate.stage_gpu(target_gpu()).is_err());
                } else {
                    for worker in candidate.stage_gpu(target_gpu()).unwrap().discard() {
                        worker.join().unwrap();
                    }
                }
                assert_eq!(context.cache.state(), cache_before);
            }
            let origin = app
                .observe_coordinated_origin(
                    &bindings,
                    &registry,
                    "play-resource-test-v1",
                    resource_limits,
                    limits(),
                    file_limits.ui,
                )
                .unwrap();
            let unguarded = coordinated.prepare(options()).unwrap();
            assert!(unguarded.stage_gpu(target_gpu()).is_err());
            let candidate = coordinated.prepare_guarded(options(), origin).unwrap();
            let pending_group = candidate.stage_gpu(target_gpu()).unwrap();
            let ready_group = pollster::block_on(pending_group.validate()).unwrap();
            assert!(ready_group.outer.game.is_none());
            assert!(ready_group.outer.project.is_some());
            assert!(ready_group.outer.ui.is_some());
            assert!(ready_group.outer.audio.is_some());
            assert!(ready_group.outer.ui_input.is_some());
            let group_owner = ready_group
                .game
                .game
                .scene
                .resolve(&ObjectId("animated".into()))
                .unwrap();
            let group_pixels = crate::animated_models::tests::pixels(
                &context.renderer,
                &context.device,
                &context.queue,
                ready_group
                    .game
                    .gpu
                    .animated
                    .geometries(group_owner)
                    .unwrap()
                    .collect(),
                context.matrix,
            );
            assert_eq!(context.reference, group_pixels);
            let (group_model, group_accepted) = ready_group
                .game
                .gpu
                .animated
                .accepted_frame(group_owner)
                .unwrap();
            let group_runtime = ready_group
                .game
                .game
                .play
                .animations
                .frame(group_owner, group_model)
                .unwrap();
            assert_eq!(Arc::ptr_eq(group_accepted, &group_runtime), shared_frame);

            ready_group
                .validate_origin(
                    &mut app,
                    &bindings,
                    &registry,
                    "play-resource-test-v1",
                    &target_gpu(),
                )
                .unwrap();
            app.audio
                .set_output_mode(crate::audio_session::AudioOutputMode::Native);
            assert!(
                ready_group
                    .validate_origin(
                        &mut app,
                        &bindings,
                        &registry,
                        "play-resource-test-v1",
                        &target_gpu(),
                    )
                    .is_err()
            );
            app.audio
                .set_output_mode(crate::audio_session::AudioOutputMode::Offline);
            ready_group
                .validate_origin(
                    &mut app,
                    &bindings,
                    &registry,
                    "play-resource-test-v1",
                    &target_gpu(),
                )
                .unwrap();
            let original_input = app.play.player_input.checkpoint();
            app.play
                .player_input
                .event(voxy_gameplay::RIGHT, 0.625)
                .unwrap();
            assert!(
                ready_group
                    .validate_origin(
                        &mut app,
                        &bindings,
                        &registry,
                        "play-resource-test-v1",
                        &target_gpu(),
                    )
                    .is_err()
            );
            app.play
                .player_input
                .restore_checkpoint(&original_input)
                .unwrap();
            ready_group
                .validate_origin(
                    &mut app,
                    &bindings,
                    &registry,
                    "play-resource-test-v1",
                    &target_gpu(),
                )
                .unwrap();
            for worker in ready_group.discard() {
                worker.join().unwrap();
            }
            assert_eq!(context.cache.state(), cache_before);
            let view = SceneCheckpointView::new(&app.scene, bindings.clone()).unwrap();
            let pending = resumed.stage_gpu(target_gpu(), guard).unwrap();
            let ready = pollster::block_on(pending.validate()).unwrap();
            ready
                .validate_origin(
                    &app,
                    &view,
                    &registry,
                    "play-resource-test-v1",
                    &target_gpu(),
                )
                .unwrap();
            let owner = ready
                .game
                .scene
                .resolve(&ObjectId("animated".into()))
                .unwrap();
            assert_eq!(ready.gpu.scene, ready.game.scene.graph.identity());
            let accepted = ready.gpu.animated.accepted_frame(owner).unwrap().1;
            let simulated = ready
                .game
                .play
                .animations
                .frame(owner, &restored_model)
                .unwrap();
            if shared_frame {
                assert!(
                    Arc::ptr_eq(accepted, &simulated),
                    "shared runtime/render frame lost Arc identity"
                );
            } else {
                if equal_independent {
                    assert_eq!(accepted.skin_matrices, simulated.skin_matrices);
                } else {
                    assert_ne!(accepted.skin_matrices, simulated.skin_matrices);
                }
                assert!(!Arc::ptr_eq(accepted, &simulated));
            }
            let actual = crate::animated_models::tests::pixels(
                &context.renderer,
                &context.device,
                &context.queue,
                ready.gpu.animated.geometries(owner).unwrap().collect(),
                context.matrix,
            );
            assert_eq!(context.reference, actual);
            let accepted_checkpoint = ready
                .gpu
                .animated
                .checkpoint_accepted_frames(
                    |node| ready.game.scene.identity(node).cloned(),
                    limits().core.max_bytes,
                )
                .unwrap();
            assert_eq!(
                serde_json::to_value(accepted_checkpoint).unwrap()["owners"][0]["ticks"],
                731
            );

            crate::import::asset_checkpoint::retain_evidence(
                "full-play-reference.rgba",
                &context.reference,
            );
            crate::import::asset_checkpoint::retain_evidence("full-play-restored.rgba", &actual);
            let input = app.play.player_input.checkpoint();
            app.play
                .player_input
                .event(voxy_gameplay::RIGHT, 0.25)
                .unwrap();
            assert!(
                ready
                    .validate_origin(
                        &app,
                        &view,
                        &registry,
                        "play-resource-test-v1",
                        &target_gpu()
                    )
                    .is_err()
            );
            app.play.player_input.restore_checkpoint(&input).unwrap();
            ready
                .validate_origin(
                    &app,
                    &view,
                    &registry,
                    "play-resource-test-v1",
                    &target_gpu(),
                )
                .unwrap();
            // Restore the parked editor source, then load the same game from idle.
            drop(view);
            std::mem::swap(&mut app.scene, &mut original.scene.graph);
            std::mem::swap(&mut app.play, &mut original.play);
            std::mem::swap(&mut app.camera, &mut original.camera);
            assert!(app.play.playing.is_none() && app.play.simulation.is_none());
            let idle_view = SceneCheckpointView::new(
                &app.scene,
                app.object_ids
                    .iter()
                    .cloned()
                    .zip(app.instances.iter().copied()),
            )
            .unwrap();
            let idle_origin = app
                .observe_play_restore_origin(
                    &idle_view,
                    &registry,
                    "play-resource-test-v1",
                    resource_limits,
                    limits(),
                )
                .unwrap();
            idle_origin
                .validate(&app, &idle_view, &registry, "play-resource-test-v1")
                .unwrap();
            let idle_candidate = saved
                .prepare(
                    "play-resource-test-v1",
                    &registry,
                    &behaviors,
                    &ui,
                    &target_root,
                    resource_limits,
                    limits(),
                )
                .unwrap();
            let candidate_geometry = ready
                .gpu
                .models
                .values()
                .map(crate::ModelGraphics::geometry_bytes)
                .sum::<u64>()
                + ready.gpu.animated.allocation_bytes();
            let idle_target = || crate::gpu_model::batch::UploadTarget {
                live_geometry: live + candidate_geometry,
                ..target_gpu()
            };
            let idle_pending = idle_candidate
                .stage_gpu(idle_target(), idle_origin)
                .unwrap();
            let mut idle_ready = pollster::block_on(idle_pending.validate()).unwrap();
            idle_ready
                .validate_origin(
                    &app,
                    &idle_view,
                    &registry,
                    "play-resource-test-v1",
                    &idle_target(),
                )
                .unwrap();
            let idle_owner = idle_ready
                .game
                .scene
                .resolve(&ObjectId("animated".into()))
                .unwrap();
            let idle_pixels = crate::animated_models::tests::pixels(
                &context.renderer,
                &context.device,
                &context.queue,
                idle_ready
                    .gpu
                    .animated
                    .geometries(idle_owner)
                    .unwrap()
                    .collect(),
                context.matrix,
            );
            assert_eq!(context.reference, idle_pixels);
            crate::import::asset_checkpoint::retain_evidence(
                "full-play-idle-restored.rgba",
                &idle_pixels,
            );
            app.play.simulation_time += std::time::Duration::from_nanos(1);
            idle_ready
                .validate_origin(
                    &app,
                    &idle_view,
                    &registry,
                    "play-resource-test-v1",
                    &idle_target(),
                )
                .unwrap();
            let old_input = app.play.player_input.checkpoint();
            app.play
                .player_input
                .event(voxy_gameplay::RIGHT, 0.125)
                .unwrap();
            assert!(
                idle_ready
                    .validate_origin(
                        &app,
                        &idle_view,
                        &registry,
                        "play-resource-test-v1",
                        &idle_target()
                    )
                    .is_err()
            );
            app.play
                .player_input
                .restore_checkpoint(&old_input)
                .unwrap();
            idle_ready
                .validate_origin(
                    &app,
                    &idle_view,
                    &registry,
                    "play-resource-test-v1",
                    &idle_target(),
                )
                .unwrap();
            app.play.playing = Some(SceneDocument {
                version: 1,
                objects: vec![],
            });
            assert!(
                idle_ready
                    .validate_origin(
                        &app,
                        &idle_view,
                        &registry,
                        "play-resource-test-v1",
                        &idle_target()
                    )
                    .is_err()
            );
            app.play.playing = None;
            idle_ready
                .validate_origin(
                    &app,
                    &idle_view,
                    &registry,
                    "play-resource-test-v1",
                    &idle_target(),
                )
                .unwrap();
            assert!(app.play.simulation.is_none());
            idle_ready
                .resources
                .imports
                .take()
                .unwrap()
                .close()
                .join()
                .unwrap();
            idle_ready
                .resources
                .watcher
                .take()
                .unwrap()
                .close()
                .join()
                .unwrap();
            drop(idle_ready);
            drop(idle_view);
            std::mem::swap(&mut app.scene, &mut original.scene.graph);
            std::mem::swap(&mut app.play, &mut original.play);
            std::mem::swap(&mut app.camera, &mut original.camera);
            resumed = crate::app::resource_checkpoint::play::PreparedPlayResources {
                resources: ready.resources,
                game: ready.game,
                gpu: None,
                accepted_frames: None,
            };
            staged_gpu = Some(ready.gpu);
            assert!(pollster::block_on(outer.pop()).is_none());
        }
        let final_origin = app
            .observe_coordinated_origin(
                &bindings,
                &registry,
                "play-resource-test-v1",
                resource_limits,
                limits(),
                crate::ui_live::checkpoint::tests::limits(),
            )
            .unwrap();
        let until = Instant::now() + std::time::Duration::from_secs(10);
        while !app.resume_restore_barrier().unwrap() {
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }

        assert!(
            final_origin
                .validate(&mut app, &bindings, &registry, "play-resource-test-v1")
                .is_err()
        );
        app.begin_restore_barrier(8 * 1024 * 1024).unwrap();
        while !app
            .poll_restore_barrier(|node| {
                bindings
                    .iter()
                    .find(|(_, n)| *n == node)
                    .map(|(id, _)| id.clone())
            })
            .unwrap()
        {
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        assert!(
            final_origin
                .validate(&mut app, &bindings, &registry, "play-resource-test-v1")
                .is_err()
        );
        while !app.resume_restore_barrier().unwrap() {
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }

        std::mem::swap(&mut app.scene, &mut original.scene.graph);
        std::mem::swap(&mut app.play, &mut original.play);
        std::mem::swap(&mut app.camera, &mut original.camera);
        assert_eq!(
            capture(&original, &model, &registry)
                .to_json(limits().core.max_bytes)
                .unwrap(),
            capture(&resumed.game, &restored_model, &registry)
                .to_json(limits().core.max_bytes)
                .unwrap()
        );
        for i in 0..120 {
            let dt = [0.003, 0.019, 0.011][i % 3];
            advance(&mut original, &model, dt);
            advance(&mut resumed.game, &restored_model, dt);
            assert_eq!(
                capture(&original, &model, &registry)
                    .to_json(limits().core.max_bytes)
                    .unwrap(),
                capture(&resumed.game, &restored_model, &registry)
                    .to_json(limits().core.max_bytes)
                    .unwrap()
            );
        }
        if let Some(gpu) = &staged_gpu {
            let owner = resumed
                .game
                .scene
                .resolve(&ObjectId("animated".into()))
                .unwrap();
            assert!(gpu.animated.accepted_frame(owner).is_some());
        }
        resumed
            .resources
            .imports
            .take()
            .unwrap()
            .close()
            .join()
            .unwrap();
        resumed
            .resources
            .watcher
            .take()
            .unwrap()
            .close()
            .join()
            .unwrap();
        app.stop_workers().unwrap();
        std::fs::remove_dir_all(target_root).unwrap();
    }
}
