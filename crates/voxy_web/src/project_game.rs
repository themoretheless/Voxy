//! In-memory project package playback for the browser.
use crate::browser::error;
use glam::{Mat4, Vec2, Vec3};
use std::{collections::BTreeMap, sync::Arc};
use voxy_assets::{
    AssetId, AssetLocations, PROJECT_LAUNCH_SOURCE, PROJECT_PACKAGE_LIMITS, ProjectDescriptor,
    ProjectPackageLaunch, ResourcePackage, SourcePath,
};
use voxy_gameplay::{
    AngularMotionBatch, BACK, BoxCollider, CharacterBody, CharacterPhysics, CollisionLayers,
    FORWARD, FollowCamera, FollowSmoother, GAMEPAD_JUMP, GAMEPAD_X, GAMEPAD_Z, JUMP, LEFT, RIGHT,
};
use voxy_input::InputMap;
use voxy_model::{ModelAnimation, ModelAsset, ModelRetarget};
use voxy_render::{
    OrbitView, SceneCamera, SceneDraw, SceneGeometry, SceneProjection, SceneRenderer,
    SceneSkinInstance, SceneSkinSource, SceneSkinner, SceneTexture, SceneTransform,
};
use voxy_scene::{
    ComponentRegistry, DocumentError, LoadedScene, NodeId, PrefabLimits, PrefabSceneDocument,
    RenderWorldCache, SceneSimulation, SimulationLimits,
};
use wasm_bindgen::JsValue;
mod clip_transition;
mod model_upload;
mod retarget;
mod root_motion;
use model_upload::load_model;
use retarget::RetargetPlayback;

const DOCUMENT_BYTES: usize = 8 * 1024 * 1024;
const GEOMETRY_BYTES: u64 = 128 * 1024 * 1024;
const DRAW_LIMIT: usize = 16_384;
const SOURCE_BYTES: usize = 16 * 1024 * 1024;
const OBJECTS: usize = 4096;
const ANIMATION_EVENT_LIMIT: usize = 4096;
const STEP: f64 = 1. / 60.;
const PAD_DEAD_ZONE: f32 = 0.15;

#[derive(Debug)]
pub(crate) struct ProjectGame {
    loaded: LoadedScene,
    simulation: SceneSimulation,
    animation_ticks: u64,
    animation_events: Vec<ProjectAnimationEvent>,
    animation_event_bytes: usize,
    clip_overrides: BTreeMap<NodeId, ClipOverride>,
    follow: Option<FollowView>,
    authored_view: Option<OrbitView>,
    render_worlds: RenderWorldCache,
    registry: ComponentRegistry,
    angular: AngularMotionBatch,
    physics: Option<CharacterPhysics>,
    input: InputMap,
    package: ResourcePackage,
    assets: AssetLocations,
    graphics: ProjectGraphics,
}

#[derive(Debug)]
struct FollowView {
    target: NodeId,
    descriptor: FollowCamera,
    smoother: FollowSmoother,
    view: OrbitView,
}

impl FollowView {
    fn load(loaded: &LoadedScene) -> Result<Option<Self>, JsValue> {
        let mut cameras = loaded.graph.active_components::<FollowCamera>();
        let Some((owner, descriptor)) = cameras.next() else {
            return Ok(None);
        };
        if cameras.next().is_some() || !descriptor.valid() {
            return Err(error("expected a single valid follow camera"));
        }
        let target = loaded
            .resolve(&descriptor.target)
            .ok_or_else(|| error("follow target is not in the loaded scene"))?;
        let mut view = loaded
            .graph
            .component::<OrbitView>(owner)
            .map_err(error)?
            .ok_or_else(|| error("follow camera requires an editor.camera.v1 view"))?
            .clone();
        if !view.valid() {
            return Err(error("invalid follow camera view"));
        }
        let desired = loaded
            .graph
            .world_matrix(target)
            .map_err(error)?
            .w_axis
            .truncate()
            .as_dvec3()
            + Vec3::from_array(descriptor.offset).as_dvec3();
        view.target = desired.as_vec3();
        view.legacy = false;
        if !view.valid() {
            return Err(error("follow camera target exceeds view bounds"));
        }
        Ok(Some(Self {
            target,
            descriptor: descriptor.clone(),
            smoother: FollowSmoother::new(desired).map_err(error)?,
            view,
        }))
    }
}

#[derive(Debug)]
pub(crate) struct ProjectGraphics {
    texture: SceneTexture,
    models: BTreeMap<AssetId, ProjectModel>,
    instances: Vec<ProjectInstance>,
    skinner: Option<SceneSkinner>,
    last_skin_tick: Option<u64>,
    event_sources: Vec<ProjectEventSource>,
}

#[derive(Debug)]
struct ModelPart {
    matrix: Mat4,
    geometry: PartGeometry,
    texture: Option<Arc<SceneTexture>>,
}

#[derive(Debug)]
enum PartGeometry {
    Static(Arc<SceneGeometry>),
    Skin {
        source: Arc<SceneSkinSource>,
        color: [f32; 4],
    },
}

#[derive(Debug)]
struct ProjectModel {
    parts: Vec<ModelPart>,
    animation: Option<Arc<ModelAsset>>,
}

#[derive(Debug)]
struct ProjectInstance {
    owner: NodeId,
    asset: AssetId,
    transforms: Vec<SceneTransform>,
    playback: Option<ProjectPlayback>,
}

#[derive(Debug)]
struct ProjectEventSource {
    owner: NodeId,
    identity: String,
    track: voxy_animation::ClipEvents,
    speed: f64,
}

#[derive(Debug, serde::Serialize)]
struct ProjectAnimationEvent {
    owner: String,
    clip: String,
    name: String,
    phase: f64,
    tick: u64,
    #[serde(skip)]
    fraction: f64,
}

fn animation_phase_at_tick(tick: u64, speed: f64, duration: f64) -> f64 {
    tick as f64 * STEP * speed / duration
}

struct EventBatch {
    events: Vec<ProjectAnimationEvent>,
    bytes: usize,
    remaining: usize,
    remaining_bytes: usize,
}

impl EventBatch {
    fn count_left(&self) -> usize {
        self.remaining - self.events.len()
    }

    fn append(
        &mut self,
        identity: &str,
        clip: &str,
        tick: u64,
        start: f64,
        end: f64,
        occurrences: Vec<voxy_animation::ClipEventOccurrence>,
    ) -> Result<(), String> {
        for event in occurrences {
            // Bound UTF-8 text, JSON escapes, and the fixed fields before cloning.
            self.bytes += 128 + 6 * (identity.len() + clip.len() + event.name.len());
            if self.bytes > self.remaining_bytes {
                return Err("animation event byte budget".into());
            }
            self.events.push(ProjectAnimationEvent {
                owner: identity.to_owned(),
                clip: clip.to_owned(),
                name: event.name.to_string(),
                phase: event.phase,
                tick,
                fraction: (event.phase - start) / (end - start),
            });
        }
        Ok(())
    }

    fn sort(&mut self) {
        self.events.sort_by(|a, b| {
            a.fraction
                .total_cmp(&b.fraction)
                .then_with(|| a.owner.cmp(&b.owner))
        });
    }
}

fn stage_authored_events(
    batch: &mut EventBatch,
    sources: &[ProjectEventSource],
    overrides: &BTreeMap<NodeId, ClipOverride>,
    scene: &voxy_scene::SceneGraph,
    tick: u64,
) -> Result<(), String> {
    for source in sources {
        if overrides.contains_key(&source.owner)
            || !scene
                .active_in_hierarchy(source.owner)
                .map_err(|e| e.to_string())?
        {
            continue;
        }
        let duration = f64::from(source.track.clip().duration());
        let phase = animation_phase_at_tick(tick, source.speed, duration);
        let cycle = phase.floor();
        let start = phase - cycle;
        let next_tick = tick.saturating_add(1);
        let end = animation_phase_at_tick(next_tick, source.speed, duration) - cycle;
        let occurrences = source
            .track
            .collect_loop_interval(start, end, batch.count_left())?;
        batch.append(
            &source.identity,
            source.track.clip().name(),
            next_tick,
            start,
            end,
            occurrences,
        )?;
    }
    Ok(())
}

#[derive(Debug)]
struct ClipOverride {
    model: Arc<ModelAsset>,
    retarget: Option<RetargetPlayback>,
    animator: voxy_animation::Animator,
    settings: ModelAnimation,
    identity: String,
    track: Option<voxy_animation::ClipEvents>,
    pose: SampledProjectPose,
    root_basis: Option<root_motion::RootBasis>,
    root_reference: Option<root_motion::RootReference>,
}

struct StagedClip {
    animator: voxy_animation::Animator,
    pose: SampledProjectPose,
    displacement: Option<(Mat4, Vec3)>,
    trajectory: Option<(root_motion::RootBasis, voxy_animation::RootRigidPath)>,
    fade: Option<root_motion::PreparedFade>,
    reference: Option<root_motion::RootReference>,
}

fn clip_event_track(
    settings: &ModelAnimation,
    clip: &Arc<voxy_animation::AnimationClip>,
) -> Result<voxy_animation::ClipEvents, &'static str> {
    voxy_animation::ClipEvents::new(
        clip.clone(),
        settings
            .events
            .iter()
            .map(|event| voxy_animation::ClipEvent {
                name: Arc::from(event.name.as_str()),
                phase: event.phase,
            })
            .collect(),
    )
}

#[derive(Debug)]
struct ProjectPlayback {
    settings: ModelAnimation,
    retarget: Option<RetargetPlayback>,
    skins: Vec<Option<SceneSkinInstance>>,
    root_basis: Option<root_motion::RootBasis>,
    sampled: Option<SampledProjectPose>,
}

#[derive(Clone, Debug)]
struct SampledProjectPose {
    tick: u64,
    mesh_matrix: Mat4,
    joints: Vec<Mat4>,
    weights: Vec<Vec<f32>>,
}

impl SampledProjectPose {
    fn from_frame(
        model: &ModelAsset,
        frame: voxy_animation::AnimatorFrame,
        tick: u64,
    ) -> Result<Self, voxy_model::ModelError> {
        let mesh_matrix = model.mesh_transform(&frame.pose)?;
        if !mesh_matrix.is_finite() {
            return Err(voxy_model::ModelError(
                "nonfinite project animation transform".into(),
            ));
        }
        Ok(Self {
            tick,
            mesh_matrix,
            weights: model.frame_weights(&frame)?,
            joints: frame.skin_matrices,
        })
    }
}

impl ProjectPlayback {
    fn sample(
        model: &ModelAsset,
        settings: &ModelAnimation,
        ticks: u64,
        retarget: Option<&RetargetPlayback>,
    ) -> Result<voxy_animation::AnimatorFrame, JsValue> {
        let source = retarget.map_or(model, |retarget| retarget.source.as_ref());
        let phase = settings.clip.map_or(0.0, |index| {
            let duration = f64::from(source.animations[index].duration());
            animation_phase_at_tick(ticks, f64::from(settings.speed), duration).fract()
        });
        let mut frame = source
            .sample_frame_phase(settings.clip, phase)
            .map_err(error)?;
        if settings.root_motion_rotation || settings.root_motion_axes.into_iter().any(|axis| axis) {
            frame.root_motion_joint = settings.resolve_motion_joint(source).map_err(error)?;
        }
        (frame, _) =
            root_motion::display_frame(frame, source, settings, retarget).map_err(error)?;
        settings
            .apply_morph_weights(model, &mut frame)
            .map_err(error)?;
        Ok(frame)
    }

    fn update_pose(&mut self, model: &ModelAsset, tick: u64) -> Result<(), JsValue> {
        if self.sampled.as_ref().is_none_or(|pose| pose.tick != tick) {
            let frame = Self::sample(model, &self.settings, tick, self.retarget.as_ref())?;
            self.sampled = Some(SampledProjectPose::from_frame(model, frame, tick).map_err(error)?);
        }
        Ok(())
    }
}

impl ProjectGame {
    pub(crate) fn load(
        bytes: &[u8],
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Self, JsValue> {
        let package = ResourcePackage::from_bytes(bytes, PROJECT_PACKAGE_LIMITS).map_err(error)?;
        let launch: ProjectPackageLaunch = serde_json::from_slice(
            &package
                .read(&AssetId(PROJECT_LAUNCH_SOURCE.into()), 65_536)
                .map_err(error)?,
        )
        .map_err(error)?;
        let descriptor: ProjectDescriptor = serde_json::from_slice(
            &package
                .read(
                    &launch.descriptor_path().map_err(error)?.observation_id(),
                    DOCUMENT_BYTES,
                )
                .map_err(error)?,
        )
        .map_err(error)?;
        if descriptor.version != 1 || descriptor.name.trim().is_empty() {
            return Err(error("invalid project descriptor"));
        }
        let path = |path: &std::path::Path| {
            SourcePath::new(
                path.to_str()
                    .ok_or_else(|| error("invalid project path"))?
                    .to_owned(),
            )
            .map_err(error)
        };
        let manifest = package
            .read(
                &path(&descriptor.asset_manifest)?.observation_id(),
                DOCUMENT_BYTES,
            )
            .map_err(error)?;
        let assets = AssetLocations::from_json(
            std::str::from_utf8(&manifest).map_err(error)?,
            PROJECT_PACKAGE_LIMITS.max_entries,
            PROJECT_PACKAGE_LIMITS.max_payload_bytes,
            DOCUMENT_BYTES,
        )
        .map_err(error)?;
        let startup = package
            .read(
                &path(&descriptor.startup_scene)?.observation_id(),
                DOCUMENT_BYTES,
            )
            .map_err(error)?;
        let scene: PrefabSceneDocument = serde_json::from_slice(&startup).map_err(error)?;
        // Only register codecs with a corresponding browser runtime owner.
        let mut registry = ComponentRegistry::default();
        registry
            .register::<String>("editor.model.v1")
            .map_err(error)?;
        registry
            .register::<voxy_gameplay::AngularMotion>("game.angular-motion.v1")
            .map_err(document_error)?;
        registry
            .register::<CharacterBody>("game.character.v1")
            .map_err(document_error)?;
        registry
            .register::<BoxCollider>("game.box.v1")
            .map_err(document_error)?;
        registry
            .register::<CollisionLayers>("game.collision-layers.v1")
            .map_err(document_error)?;
        registry
            .register::<OrbitView>("editor.camera.v1")
            .map_err(document_error)?;
        voxy_gameplay::register_follow_camera(&mut registry).map_err(document_error)?;
        registry
            .register::<ModelAnimation>("editor.model-animation.v1")
            .map_err(document_error)?;
        registry
            .register::<ModelRetarget>("editor.model-retarget.v1")
            .map_err(document_error)?;
        let loaded = scene
            .load(
                &registry,
                PrefabLimits {
                    max_objects: OBJECTS,
                    max_instances: 128,
                    max_depth: 16,
                },
                |id| {
                    let source = assets
                        .source(&AssetId(id.into()))
                        .ok_or_else(|| DocumentError::Invalid(format!("missing prefab {id}")))?;
                    let bytes = package
                        .read(&source.observation_id(), DOCUMENT_BYTES)
                        .map_err(DocumentError::Invalid)?;
                    Ok(serde_json::from_slice(&bytes)?)
                },
            )
            .map_err(document_error)?;
        let angular = AngularMotionBatch::new(&loaded.graph, OBJECTS).map_err(error)?;
        let graph = &loaded.graph;
        let physics = if graph.components::<CharacterBody>().next().is_some()
            || graph.components::<BoxCollider>().next().is_some()
        {
            let physics = CharacterPhysics::new(graph, 128, 128).with_depenetration(true);
            physics.validate_start(graph).map_err(error)?;
            Some(physics)
        } else {
            None
        };
        let authored_view = graph
            .active_components::<OrbitView>()
            .next()
            .map(|(_, view)| view.clone());
        if authored_view.as_ref().is_some_and(|view| !view.valid()) {
            return Err(error("invalid authored camera"));
        }
        let follow = FollowView::load(&loaded)?;
        let simulation = SceneSimulation::new(
            graph,
            SimulationLimits {
                fixed_step: STEP,
                max_steps: 8,
                max_behaviors: 0,
                max_commands: 0,
            },
        )
        .map_err(error)?;
        let graphics = ProjectGraphics::build(&package, &assets, &loaded, renderer, device, queue)?;
        let clip_overrides = Self::authored_root_playbacks(&loaded, &graphics)?;
        Ok(Self {
            loaded,
            simulation,
            animation_ticks: 0,
            animation_events: Vec::new(),
            animation_event_bytes: 0,
            clip_overrides,
            follow,
            authored_view,
            render_worlds: RenderWorldCache::default(),
            registry,
            angular,
            physics,
            input: voxy_gameplay::player_input_with_gamepad().map_err(error)?,
            package,
            assets,
            graphics,
        })
    }

    pub(crate) fn rebuild_graphics(
        &self,
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<ProjectGraphics, JsValue> {
        ProjectGraphics::build(
            &self.package,
            &self.assets,
            &self.loaded,
            renderer,
            device,
            queue,
        )
    }

    pub(crate) fn install_graphics(&mut self, graphics: ProjectGraphics) {
        self.graphics = graphics;
    }

    pub(crate) fn key(&mut self, code: &str, pressed: bool) -> Result<bool, JsValue> {
        let control = match code {
            "KeyA" => LEFT,
            "KeyD" => RIGHT,
            "KeyW" => FORWARD,
            "KeyS" => BACK,
            "Space" => JUMP,
            _ => return Ok(false),
        };
        self.input
            .event(control, f32::from(pressed))
            .map_err(error)?;
        Ok(true)
    }

    pub(crate) fn gamepad(&mut self, x: f32, z: f32, jump: bool) -> Result<(), JsValue> {
        if !x.is_finite() || !z.is_finite() || x.abs() > 1.0 || z.abs() > 1.0 {
            return Err(error("invalid gamepad axes"));
        }
        let axis = Vec2::new(x, z);
        let length = axis.length();
        let movement = if length <= PAD_DEAD_ZONE {
            Vec2::ZERO
        } else {
            axis / length * ((length.min(1.0) - PAD_DEAD_ZONE) / (1.0 - PAD_DEAD_ZONE))
        };
        self.input.event(GAMEPAD_X, movement.x).map_err(error)?;
        self.input.event(GAMEPAD_Z, movement.y).map_err(error)?;
        self.input
            .event(GAMEPAD_JUMP, f32::from(jump))
            .map_err(error)
    }

    pub(crate) fn focus(&mut self, focused: bool) {
        self.input.set_focused(focused);
    }

    pub(crate) fn step(&mut self, ticks: u32) -> Result<(), JsValue> {
        if ticks > 120 {
            return Err(error("project step exceeds 120 ticks"));
        }
        for _ in 0..ticks {
            self.advance(STEP)?;
        }
        Ok(())
    }

    pub(crate) fn advance(&mut self, elapsed: f64) -> Result<u32, JsValue> {
        let frame = self
            .simulation
            .advance_with(&mut self.loaded.graph, elapsed, |scene, dt| {
                let mut batch = EventBatch {
                    events: Vec::new(),
                    bytes: 0,
                    remaining: ANIMATION_EVENT_LIMIT - self.animation_events.len(),
                    remaining_bytes: DOCUMENT_BYTES - self.animation_event_bytes,
                };
                stage_authored_events(
                    &mut batch,
                    &self.graphics.event_sources,
                    &self.clip_overrides,
                    scene,
                    self.animation_ticks,
                )?;
                let next_tick = self.animation_ticks.saturating_add(1);
                let mut overrides = Vec::with_capacity(self.clip_overrides.len());
                for (&owner, state) in &self.clip_overrides {
                    if scene
                        .active_in_hierarchy(owner)
                        .map_err(|error| error.to_string())?
                    {
                        overrides.push((owner, state.stage_tick(&mut batch, next_tick, dt)?));
                    }
                }
                self.angular
                    .fixed_step(scene, dt)
                    .map_err(|failure| failure.to_string())?;
                if let Some(physics) = &mut self.physics {
                    root_motion::apply_character_motion(
                        physics,
                        scene,
                        &mut self.input,
                        dt,
                        root_motion::TickAnimations {
                            staged: &mut overrides,
                            owners: &self.clip_overrides,
                            batch: &mut batch,
                            tick: next_tick,
                        },
                    )?;
                } else {
                    self.input.finish_frame();
                }
                batch.sort();
                for (owner, staged) in overrides {
                    let state = self
                        .clip_overrides
                        .get_mut(&owner)
                        .expect("staged clip override");
                    state.animator = staged.animator;
                    state.pose = staged.pose;
                    state.root_reference = staged.reference;
                }
                self.animation_events.extend(batch.events);
                self.animation_event_bytes += batch.bytes;
                self.animation_ticks = next_tick;
                Ok::<(), String>(())
            })
            .map_err(error)?;
        self.render_worlds.clear();
        if let Some(follow) = &mut self.follow {
            let desired = self
                .simulation
                .render_world_cached(&self.loaded.graph, &mut self.render_worlds, follow.target)
                .map_err(error)?
                .w_axis
                .truncate()
                .as_dvec3()
                + Vec3::from_array(follow.descriptor.offset).as_dvec3();
            let mut smoother = follow.smoother.clone();
            let target = smoother
                .advance(desired, follow.descriptor.half_life, elapsed)
                .map_err(error)?
                .as_vec3();
            let mut view = follow.view.clone();
            view.target = target;
            if !view.valid() {
                return Err(error("follow camera target exceeds view bounds"));
            }
            follow.view = view;
            follow.smoother = smoother;
        }
        Ok(frame.time.steps as u32)
    }

    fn current_view(&self) -> Option<&OrbitView> {
        self.follow
            .as_ref()
            .map(|follow| &follow.view)
            .or(self.authored_view.as_ref())
    }

    pub(crate) fn take_animation_events(&mut self) -> Result<Option<String>, JsValue> {
        if self.animation_events.is_empty() {
            return Ok(None);
        }
        let events = serde_json::to_string(&self.animation_events).map_err(error)?;
        self.animation_events.clear();
        self.animation_event_bytes = 0;
        Ok(Some(events))
    }

    pub(crate) fn camera_target(&self) -> Vec<f32> {
        self.current_view()
            .map_or(Vec3::ZERO, |view| view.target)
            .to_array()
            .to_vec()
    }

    pub(crate) fn state(&self) -> Result<String, JsValue> {
        self.loaded
            .capture(&self.registry)
            .map_err(document_error)?
            .to_json()
            .map_err(document_error)
    }

    pub(crate) fn commit_draws(&mut self) {
        self.graphics.last_skin_tick = Some(self.animation_ticks);
    }

    pub(crate) fn draws<'a>(
        &'a mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        size: Vec2,
    ) -> Result<Vec<SceneDraw<'a>>, JsValue> {
        let vp = if let Some(view) = self.current_view() {
            view.scene_camera(size)
                .map_err(error)?
                .map_or(Ok(Mat4::IDENTITY), SceneCamera::view_projection)
                .map_err(error)?
        } else {
            SceneCamera {
                eye: Vec3::new(1.8, 1.4, 3.),
                target: Vec3::ZERO,
                up: Vec3::Y,
                projection: SceneProjection::Perspective {
                    vertical_fov: 55_f32.to_radians(),
                    aspect: size.x / size.y,
                    near: 0.1,
                    far: 1000.,
                },
            }
            .view_projection()
            .map_err(error)?
        };
        let mut draws = Vec::with_capacity(self.graphics.instances.len());
        let update_skins = self.graphics.last_skin_tick != Some(self.animation_ticks);
        for instance in &mut self.graphics.instances {
            if !self
                .loaded
                .graph
                .active_in_hierarchy(instance.owner)
                .map_err(error)?
            {
                continue;
            }
            let world = vp
                * self
                    .simulation
                    .render_world_cached(
                        &self.loaded.graph,
                        &mut self.render_worlds,
                        instance.owner,
                    )
                    .map_err(error)?;
            let model = &self.graphics.models[&instance.asset];
            let sampled = match (model.animation.as_ref(), instance.playback.as_mut()) {
                (Some(model), Some(playback)) => {
                    if let Some(state) = self.clip_overrides.get(&instance.owner) {
                        Some((&state.pose, &playback.skins))
                    } else {
                        playback.update_pose(model, self.animation_ticks)?;
                        playback
                            .sampled
                            .as_ref()
                            .map(|pose| (pose, &playback.skins))
                    }
                }
                _ => None,
            };
            let mesh_matrix = sampled.map_or(Mat4::IDENTITY, |(pose, _)| pose.mesh_matrix);
            for (index, (part, transform)) in
                model.parts.iter().zip(&instance.transforms).enumerate()
            {
                let (geometry, matrix) = match &part.geometry {
                    PartGeometry::Static(geometry) => {
                        (geometry.as_ref(), mesh_matrix * part.matrix)
                    }
                    PartGeometry::Skin { .. } => {
                        let (pose, skins) =
                            sampled.ok_or_else(|| error("missing project skin pose"))?;
                        let skin = skins[index]
                            .as_ref()
                            .ok_or_else(|| error("missing project skin instance"))?;
                        if update_skins {
                            let skinner = self
                                .graphics
                                .skinner
                                .as_ref()
                                .ok_or_else(|| error("missing project skinner"))?;
                            let prepared = if pose.weights[index].is_empty() {
                                skinner.prepare_pose(skin, &pose.joints)
                            } else {
                                skinner.prepare_morph_pose(skin, &pose.joints, &pose.weights[index])
                            }
                            .map_err(error)?;
                            skinner
                                .encode_prepared_pose(queue, encoder, &prepared)
                                .map_err(error)?;
                        }
                        (skin.geometry(), part.matrix)
                    }
                };
                transform.update(queue, world * matrix).map_err(error)?;
                draws.push(SceneDraw {
                    geometry,
                    texture: part.texture.as_deref().unwrap_or(&self.graphics.texture),
                    transform,
                    overlay: false,
                });
            }
        }
        Ok(draws)
    }
}

impl ProjectGraphics {
    fn build(
        package: &ResourcePackage,
        assets: &AssetLocations,
        loaded: &LoadedScene,
        renderer: &SceneRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Self, JsValue> {
        let graph = &loaded.graph;
        let mut event_sources = Vec::new();
        let mut animation_sources = BTreeMap::<AssetId, Arc<ModelAsset>>::new();
        let mut remaining_geometry = GEOMETRY_BYTES;
        let mut skinner = None;
        let mut remaining_draws = DRAW_LIMIT;
        let mut models = BTreeMap::new();
        let mut instances = Vec::new();
        for (owner, _) in graph.components::<ModelAnimation>() {
            if graph.component::<String>(owner).map_err(error)?.is_none() {
                return Err(error("model animation requires a model binding"));
            }
        }
        for (owner, _) in graph.components::<ModelRetarget>() {
            if graph.component::<String>(owner).map_err(error)?.is_none() {
                return Err(error("retarget profile requires a model binding"));
            }
        }
        for (_, model) in graph.components::<String>() {
            let asset = AssetId(model.clone());
            if !models.contains_key(&asset) {
                let source = assets
                    .source(&asset)
                    .ok_or_else(|| error(format!("missing model {model}")))?;
                let parts = load_model(
                    package,
                    source,
                    renderer,
                    device,
                    queue,
                    &mut remaining_geometry,
                    &mut skinner,
                )?;
                models.insert(asset.clone(), parts);
            }
        }
        for (owner, model) in graph.components::<String>() {
            let asset = AssetId(model.clone());
            let model = &models[&asset];
            let settings = graph.component::<ModelAnimation>(owner).map_err(error)?;
            let profile = graph.component::<ModelRetarget>(owner).map_err(error)?;
            let playback = if let Some(animation) = &model.animation {
                let retarget = profile
                    .map(|profile| {
                        RetargetPlayback::load(
                            package,
                            assets,
                            &models,
                            &mut animation_sources,
                            profile,
                            animation,
                        )
                    })
                    .transpose()?;
                let source = retarget
                    .as_ref()
                    .map_or(animation.as_ref(), |retarget| retarget.source.as_ref());
                let mut settings = settings.cloned().unwrap_or_else(|| ModelAnimation {
                    clip: (!source.animations.is_empty()).then_some(0),
                    ..Default::default()
                });
                settings
                    .validate(
                        Some(source.animations.len()),
                        Some(source.skeleton.joints().len()),
                    )
                    .map_err(error)?;
                settings.clip = settings.resolve_clip(source).map_err(error)?;
                let root_basis =
                    root_motion::root_basis(graph, owner, source, &settings, retarget.as_ref())?;
                if !settings.events.is_empty()
                    && settings.speed > 0.0
                    && let Some(index) = settings.clip
                {
                    let track =
                        clip_event_track(&settings, &source.animations[index]).map_err(error)?;
                    let identity = loaded
                        .identity(owner)
                        .ok_or_else(|| error("unidentified animation event owner"))?;
                    event_sources.push(ProjectEventSource {
                        owner,
                        identity: identity.0.clone(),
                        track,
                        speed: f64::from(settings.speed),
                    });
                }
                let initial = ProjectPlayback::sample(animation, &settings, 0, retarget.as_ref())?;
                let weights = animation.frame_weights(&initial).map_err(error)?;
                let mut skins = Vec::with_capacity(model.parts.len());
                for (part, weights) in model.parts.iter().zip(weights) {
                    let skin = if let PartGeometry::Skin { source, color } = &part.geometry {
                        let skinner = skinner
                            .as_ref()
                            .ok_or_else(|| error("missing project skinner"))?;
                        let live = GEOMETRY_BYTES - remaining_geometry;
                        let instance = if weights.is_empty() {
                            skinner.create_instance(
                                renderer,
                                source.clone(),
                                &initial.skin_matrices,
                                *color,
                                live,
                                GEOMETRY_BYTES,
                            )
                        } else {
                            skinner.create_morph_instance(
                                renderer,
                                source.clone(),
                                &initial.skin_matrices,
                                &weights,
                                *color,
                                live,
                                GEOMETRY_BYTES,
                            )
                        }
                        .map_err(error)?;
                        remaining_geometry -= instance.allocation_bytes();
                        Some(instance)
                    } else {
                        None
                    };
                    skins.push(skin);
                }
                Some(ProjectPlayback {
                    settings,
                    retarget,
                    skins,
                    root_basis,
                    sampled: None,
                })
            } else {
                if profile.is_some() {
                    return Err(error("retarget target must contain an animation rig"));
                }
                if settings.is_some() {
                    return Err(error("model animation requires an animated model"));
                }
                None
            };
            let count = model.parts.len();
            remaining_draws = remaining_draws
                .checked_sub(count)
                .ok_or_else(|| error("browser project draw budget exceeded"))?;
            let transforms = (0..count)
                .map(|_| {
                    renderer
                        .create_transform(device, Mat4::IDENTITY)
                        .map_err(error)
                })
                .collect::<Result<Vec<_>, _>>()?;
            instances.push(ProjectInstance {
                owner,
                asset,
                transforms,
                playback,
            });
        }
        Ok(Self {
            texture: renderer
                .upload_texture(device, queue, 1, 1, &[255; 4])
                .map_err(error)?,
            models,
            instances,
            skinner,
            last_skin_tick: None,
            event_sources,
        })
    }
}

fn document_error(value: DocumentError) -> JsValue {
    error(format!("{value:?}"))
}
