//! Native authoring viewport; IO/decoding is asynchronous, publication/upload is owner-only.
mod animation_preview;
mod audio_play;
mod audio_reload;
mod audio_session;
mod audio_settings;
mod authoring_session;
mod camera;
mod component_collections;
mod component_fields;
mod fog_draw;
mod frame_styles;
mod liquid_draw;
mod lod_smoke;
mod packaged_game;
mod play_session;
mod prefab_authoring;
mod prefab_fields;
mod scene_revision;
mod view_layout;
pub use packaged_game::{run_packaged_game, run_packaged_game_with_ui_actions};
mod prefab_smoke;
pub use camera::ViewportCamera as EditorCamera;
mod animation_runtime;
mod foot_placement;
mod gameplay_smoke;
mod gizmo;
mod gpu_model;
mod model_playback;
mod retarget_authoring;
mod retarget_profile;
pub use foot_placement::{FootBinding, FootContactKey, ModelFootPlacement};
pub use retarget_profile::{ModelRetarget, RetargetJointProfile};
mod animated_models;
mod animation_smoke;
mod scene_limits;
pub use model_playback::{ModelAnimation, ModelAnimationEvent};
mod import;
mod material;
mod scene3d_smoke;
#[cfg(test)]
mod scene3d_tests;
mod shadow_authoring;
pub use material::{DirectionalLight, SceneMaterial};
pub use shadow_authoring::{DirectionalShadow, DirectionalShadowFilter, OpaqueShadowCaster};
mod game_ui_input;
mod panel_focus;
mod panels;
mod ui_draw;
mod ui_live;
mod ui_text;
use gizmo::DragAxis;
pub use gizmo::translation_gizmo;
mod picking;
mod selection;
pub use selection::selection_outline;

/// A scene instance references a logical resource independently of its geometry revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelInstance {
    pub asset: voxy_assets::AssetId,
}
/// Index of an imported hierarchy node in a shared model resource.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPart {
    pub node: u32,
}

use glam::{Mat4, Vec2, Vec3};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
    time::{Duration, Instant},
};
use voxy_assets::{
    AssetCatalog, AssetError, AssetId, AssetImportWorker, AssetLocations, FileInputs,
    ImportedAsset, PublicationError, SourceDependencies, SourcePath, SourcePollWorker,
};
use voxy_gameplay::{BoxCollider, CharacterBody, CharacterPhysics};
#[cfg(test)]
use voxy_render::{ObjAsset, ObjLimits};
use voxy_render::{
    RenderOutcome, SceneDraw, SceneGeometry, SceneRenderer, SceneSurface, SceneTexture,
    SceneTransform,
};
use voxy_scene::{
    Behavior, ComponentRegistry, NodeId, ObjectId, SceneDocument, SceneEdit, SceneExtraction,
    SceneGraph, SceneHistory, SceneObject, SceneSimulation, SimulationLimits, Transform,
    save_scene_file,
};
#[cfg(test)]
use voxy_scene::{PickBounds, PickViewport};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};
fn game_control(key: KeyCode) -> Option<voxy_input::Control> {
    match key {
        KeyCode::ArrowLeft => Some(voxy_gameplay::LEFT),
        KeyCode::ArrowRight => Some(voxy_gameplay::RIGHT),
        KeyCode::KeyW => Some(voxy_gameplay::FORWARD),
        KeyCode::KeyS => Some(voxy_gameplay::BACK),
        KeyCode::Space => Some(voxy_gameplay::JUMP),
        _ => None,
    }
}
fn model_registry() -> Result<ComponentRegistry, voxy_scene::DocumentError> {
    let mut registry = ComponentRegistry::default();
    registry.register::<String>("editor.model.v1")?;
    registry.register::<ModelPart>("editor.model-part.v1")?;
    registry.register::<ModelAnimation>("editor.model-animation.v1")?;
    registry.register::<ModelFootPlacement>("editor.foot-placement.v1")?;
    registry.register::<ModelRetarget>("editor.model-retarget.v1")?;
    registry.register::<SceneMaterial>("editor.material.v1")?;
    registry.register::<DirectionalLight>("editor.light.v1")?;
    registry.register::<DirectionalShadow>("editor.directional-shadow.v1")?;
    registry.register::<OpaqueShadowCaster>("editor.opaque-shadow-caster.v1")?;
    registry.register::<voxy_scene::FogVolume>("scene.fog.v1")?;
    registry.register::<EditorCamera>("editor.camera.v1")?;
    voxy_gameplay::register_components(&mut registry)?;
    Ok(registry)
}
#[derive(Debug)]
struct Graphics {
    host: SceneSurface,
    fog: Option<fog_draw::FogDraw>,
    renderer: SceneRenderer,
    models: BTreeMap<AssetId, ModelGraphics>,
    animated_models: animated_models::AnimatedModels,
    residency_cache: gpu_model::ResidencyCache,
    residency_errors: BTreeMap<AssetId, gpu_model::DeferredUpload>,
    optical_liquids: BTreeMap<u8, voxy_render::ScreenSpaceFluidRenderer>,
    liquid_revision: Option<(voxy_scene::SceneId, u64)>,
    liquid_geometry: Option<SceneGeometry>,
    liquid_transforms: BTreeMap<u8, SceneTransform>,
    gizmo_geometry: SceneGeometry,
    gizmo_transforms: BTreeMap<u8, SceneTransform>,
    texture: SceneTexture,
    transforms: HashMap<(u8, NodeId), SceneTransform>,
    lod_history: HashMap<(u8, NodeId), voxy_render::SceneLodHistory>,
    lod_pending: BTreeSet<(AssetId, usize)>,
    panel_geometry: Option<SceneGeometry>,
    panel_texture: Option<SceneTexture>,
    panel_transform: SceneTransform,
    ui_draws: Vec<ui_live::GpuDraw>,
    ui_focus: Vec<ui_live::GpuDraw>,
    ui_focus_key: Option<ui_draw::FocusRing>,
    ui_focus_deferred: Option<(ui_draw::FocusRing, (u64, u64, u64, u64))>,
    ui_presented: bool,
}
impl Graphics {
    fn geometry_bytes(&self) -> u64 {
        self.models
            .values()
            .map(ModelGraphics::geometry_bytes)
            .sum::<u64>()
            + self.animated_models.allocation_bytes()
            + self
                .fog
                .as_ref()
                .map_or(0, fog_draw::FogDraw::allocation_bytes)
            + self.gizmo_geometry.allocation_bytes()
            + self
                .optical_liquids
                .values()
                .map(voxy_render::ScreenSpaceFluidRenderer::allocation_bytes)
                .sum::<u64>()
            + self
                .liquid_geometry
                .as_ref()
                .map_or(0, SceneGeometry::allocation_bytes)
            + self
                .panel_geometry
                .as_ref()
                .map_or(0, SceneGeometry::allocation_bytes)
            + self
                .ui_draws
                .iter()
                .chain(self.ui_focus.iter())
                .map(|draw| draw.geometry.allocation_bytes())
                .sum::<u64>()
    }
}
#[derive(Debug)]
struct ModelGraphics {
    geometry: gpu_model::ModelGeometry,
    outline: Option<SceneGeometry>,
    parts: BTreeMap<u32, PartGraphics>,
    animated_textures: Vec<Option<Arc<SceneTexture>>>,
    animated_preview: Vec<SceneGeometry>,
    animated_model: Option<Arc<voxy_render::ModelAsset>>,
    animated_lod: Option<Arc<voxy_render::SkinnedLodMesh>>,
    _images: Vec<Arc<SceneTexture>>,
}
#[derive(Debug)]
struct PartGraphics {
    geometry: Arc<SceneGeometry>,
    outline: Option<Arc<SceneGeometry>>,
    texture: Option<Arc<SceneTexture>>,
}
impl ModelGraphics {
    fn part(
        &self,
        index: Option<u32>,
    ) -> Option<(
        &SceneGeometry,
        Option<&SceneGeometry>,
        Option<&SceneTexture>,
    )> {
        match index {
            None => Some((
                self.geometry.base(),
                self.outline.as_ref(),
                (self.animated_textures.len() == 1)
                    .then(|| self.animated_textures[0].as_deref())
                    .flatten(),
            )),
            Some(index) => self.parts.get(&index).map(|part| {
                (
                    part.geometry.as_ref(),
                    part.outline.as_deref(),
                    part.texture.as_deref(),
                )
            }),
        }
    }
}
#[derive(Debug)]
enum InputRecipe {
    Direct,
    Manifest(AssetId),
}
#[derive(Debug)]
struct ModelDrag {
    node: NodeId,
    origin: Vec2,
    viewport: Vec2,
    viewport_origin: Vec2,
    original: Transform,
    world_origin: Vec3,
    axis: DragAxis,
    edit: SceneEdit,
}
#[derive(Debug)]
struct InputTrace {
    enabled: bool,
    pending: bool,
    last_outcome: Option<RenderOutcome>,
}
use audio_session::AudioOutputMode;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InspectorMode {
    Transform,
    Physics,
    Material,
    Behavior,
    Audio,
    Mixer,
    ImportSettings,
    Components(usize),
    Collections(usize, usize),
}
#[derive(Debug, PartialEq)]
struct PanelInput {
    focused: Option<voxy_ui::WidgetId>,
    document: SceneDocument,
    prefab_metadata: serde_json::Value,
    selected: usize,
    size: [f32; 2],
    playing: bool,
    inspector: InspectorMode,
    texture_label: String,
    prefab_label: String,
    import_config: Option<voxy_gameplay::AudioImportConfig>,
    field: Option<(usize, String)>,
    scroll: usize,
    parenting: bool,
}
#[derive(Debug)]
struct SmokePlay {
    started: Instant,
    first_frame: u64,
    stop_frame: Option<u64>,
}
#[derive(Debug)]
struct App {
    authoring: authoring_session::AuthoringSession,
    play: play_session::PlaySession,
    audio: audio_session::AudioSession,
    scene: SceneGraph,
    extraction: SceneExtraction<ModelInstance>,
    frame_styles: Option<frame_styles::FrameStyles>,
    object_ids: Vec<ObjectId>,
    instances: Vec<NodeId>,
    selected: usize,
    cursor: Option<Vec2>,
    camera: camera::ViewportCamera,
    secondary_camera: Option<camera::ViewportCamera>,
    active_view: u8,
    msaa4: bool,
    camera_drag: Option<(MouseButton, Vec2)>,
    drag: Option<ModelDrag>,
    ui_live: ui_live::UiLive,
    game_ui_input: game_ui_input::GameUiInput,
    ui_action_setup: Option<voxy_gameplay::UiActionSetup>,
    retired_ui_workers: Vec<std::thread::JoinHandle<()>>,
    standalone: bool,
    keyboard_device: Option<winit::event::DeviceId>,
    inspector: InspectorMode,
    smoke_play: Option<SmokePlay>,
    gameplay_smoke: Option<gameplay_smoke::Smoke>,
    scene3d_smoke: Option<scene3d_smoke::Smoke>,
    lod_smoke: Option<lod_smoke::Smoke>,
    animation_smoke: Option<animation_smoke::Smoke>,
    prefab_smoke: Option<prefab_smoke::Smoke>,
    panels: Option<panels::Panels>,
    panel_cache: Option<PanelInput>,
    modifiers: winit::keyboard::ModifiersState,
    field: Option<(usize, String)>,
    component_edit: Option<component_fields::BoundComponentField>,
    retarget_draft: Option<retarget_authoring::Draft>,
    retarget_picker: Option<retarget_authoring::BonePicker>,
    tree_scroll: usize,
    parenting: Option<NodeId>,
    id: AssetId,
    catalog: AssetCatalog<ImportedAsset<import::EditorAsset>>,
    sources: SourceDependencies,
    imports: Option<AssetImportWorker<import::EditorAsset>>,
    watcher: Option<SourcePollWorker>,
    window: Option<Arc<Window>>,
    graphics: Option<Graphics>,
    reload: BTreeSet<AssetId>,
    available: BTreeSet<AssetId>,
    scan_active: bool,
    last_scan: Instant,
    last_publication: Option<Instant>,
    frames: u64,
    trace: InputTrace,
    failed_at: Option<u64>,
    recovered: bool,
    smoke_deadline: Option<Instant>,
    error: Option<String>,
}
impl App {
    fn new(path: &std::path::Path, smoke: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.canonicalize()?;
        let root = path.parent().ok_or("missing source root")?;
        let id = AssetId(
            path.file_name()
                .and_then(|s| s.to_str())
                .ok_or("source name must be UTF-8")?
                .into(),
        );
        SourcePath::new(id.0.clone())?;
        let recipe = InputRecipe::Direct;
        Self::create(root, id, recipe, smoke)
    }
    fn from_manifest(
        path: &std::path::Path,
        asset: AssetId,
        smoke: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.canonicalize()?;
        let root = path.parent().ok_or("missing manifest root")?;
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("manifest name must be UTF-8")?;
        Self::create(
            root,
            asset,
            InputRecipe::Manifest(SourcePath::new(name)?.observation_id()),
            smoke,
        )
    }
    #[allow(clippy::too_many_lines)]
    fn create(
        root: &std::path::Path,
        id: AssetId,
        recipe: InputRecipe,
        smoke: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::create_with_registry(root, id, recipe, smoke, Arc::new(model_registry()?))
    }
    fn create_with_registry(
        root: &std::path::Path,
        id: AssetId,
        recipe: InputRecipe,
        smoke: bool,
        registry: Arc<ComponentRegistry>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let available = match &recipe {
            InputRecipe::Direct => BTreeSet::from([id.clone()]),
            InputRecipe::Manifest(manifest) => {
                let bytes = FileInputs::new(root)?.read(manifest, 65536)?;
                AssetLocations::from_json(std::str::from_utf8(&bytes)?, 128, 65536, 65536)?
                    .bindings()
                    .filter(|(_, source)| {
                        std::path::Path::new(source.as_str())
                            .extension()
                            .and_then(|extension| extension.to_str())
                            .is_some_and(|extension| {
                                matches!(
                                    extension.to_ascii_lowercase().as_str(),
                                    "obj" | "gltf" | "glb" | "vmodel"
                                )
                            })
                    })
                    .map(|(id, _)| id.clone())
                    .collect()
            }
        };
        if !available.contains(&id) {
            return Err("unknown initial model resource".into());
        }
        let authoring_project =
            prefab_authoring::AuthoringProject::with_registry(root, &recipe, registry)?;
        let reload = BTreeSet::from([id.clone()]);
        let imports = AssetImportWorker::new(
            FileInputs::new(root)?,
            34,
            32 * 1024 * 1024,
            move |asset, provider, inputs| {
                let source_path = match &recipe {
                    InputRecipe::Direct => {
                        SourcePath::new(asset.0.clone()).map_err(|error| error.to_string())?
                    }
                    InputRecipe::Manifest(manifest) => {
                        let snapshot = inputs
                            .read(manifest.clone(), |id, limit| {
                                provider.read(id, limit.min(65536))
                            })
                            .map_err(|e| format!("{e:?}"))?;
                        let json =
                            std::str::from_utf8(&snapshot.bytes).map_err(|e| e.to_string())?;
                        let locations = AssetLocations::from_json(json, 128, 65536, 65536)
                            .map_err(|e| e.to_string())?;
                        locations
                            .source(asset)
                            .cloned()
                            .ok_or("logical resource not found in manifest")?
                    }
                };
                import::load(&source_path, provider, inputs)
            },
        )?;
        let mut scene = SceneGraph::new(scene_limits::OBJECTS);
        let model_node = scene.spawn(
            None,
            Transform {
                translation: Vec3::new(0.0, 0.0, 0.5),
                ..Transform::default()
            },
        )?;
        scene.set_name(model_node, "Model 1")?;
        scene.insert_component(model_node, ModelInstance { asset: id.clone() })?;
        let mut app = Self {
            authoring: authoring_session::AuthoringSession {
                preview_seek: None,
                animation_preview: None,
                history: None,
                settings_written: BTreeMap::new(),
                scene_path: None,
                scene_revision: None,
                prefab_assets: authoring_project.prefabs()?,
                prefab_choice: 0,
                authoring_source: None,
                next_object_id: 1,
                authoring_project,
            },
            play: play_session::PlaySession {
                playing: None,
                simulation_time: Instant::now(),
                simulation: None,
                angular_motion: None,
                animations: Default::default(),
                simulation_ticks: 0,
                physics: None,
                liquid: None,
                liquid_optics: Vec::new(),
                player_input: voxy_gameplay::player_input()?,
                ui_actions: None,
            },
            scene,
            extraction: SceneExtraction::new(scene_limits::OBJECTS),
            frame_styles: None,
            object_ids: vec![ObjectId("model-0".into())],
            instances: vec![model_node],
            selected: 0,
            cursor: None,
            camera: camera::ViewportCamera::default(),
            secondary_camera: None,
            active_view: 0,
            msaa4: false,
            camera_drag: None,
            drag: None,
            audio: audio_session::AudioSession::default(),
            ui_live: ui_live::UiLive::default(),
            game_ui_input: game_ui_input::GameUiInput::new(),
            ui_action_setup: None,
            retired_ui_workers: Vec::new(),
            standalone: false,
            keyboard_device: None,
            inspector: InspectorMode::Transform,
            smoke_play: None,
            gameplay_smoke: None,
            scene3d_smoke: None,
            lod_smoke: None,
            animation_smoke: None,
            prefab_smoke: None,
            panels: None,
            panel_cache: None,
            modifiers: winit::keyboard::ModifiersState::empty(),
            field: None,
            component_edit: None,
            retarget_draft: None,
            retarget_picker: None,
            tree_scroll: 0,
            parenting: None,
            id,
            catalog: AssetCatalog::new(128, 1)?,
            sources: SourceDependencies::new(128, 512),
            imports: Some(imports),
            watcher: Some(SourcePollWorker::new(FileInputs::new(root)?, 257, 65536)?),
            window: None,
            graphics: None,
            reload,
            available,
            scan_active: false,
            last_scan: Instant::now(),
            last_publication: None,
            frames: 0,
            trace: InputTrace {
                enabled: std::env::var_os("VOXY_EDITOR_TRACE_INPUT").is_some(),
                pending: true,
                last_outcome: None,
            },
            failed_at: None,
            recovered: false,
            smoke_deadline: smoke.then(|| Instant::now() + Duration::from_secs(15)),
            error: None,
        };
        app.authoring.history = Some(SceneHistory::new(
            app.authoring_document()?,
            &app.authoring.authoring_project.registry,
            scene_limits::OBJECTS,
            64,
            scene_limits::HISTORY_BYTES,
        )?);
        Ok(app)
    }
    fn initialize(&mut self, events: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        if self.window.is_some() {
            return Ok(());
        }
        let window = Arc::new(
            events.create_window(
                Window::default_attributes()
                    .with_visible(false)
                    .with_title(if self.standalone {
                        "Voxy Game"
                    } else {
                        "Voxy model viewport"
                    })
                    .with_inner_size(winit::dpi::LogicalSize::new(
                        640.0,
                        if self.standalone { 480.0 } else { 680.0 },
                    ))
                    .with_min_inner_size(winit::dpi::LogicalSize::new(
                        480.0,
                        if self.standalone { 480.0 } else { 680.0 },
                    )),
            )?,
        );
        let size = window.inner_size();
        let mut host = pollster::block_on(SceneSurface::new(
            Arc::clone(&window),
            size.width,
            size.height,
        ))?;
        if self.msaa4 {
            host.enable_msaa4()?;
        }
        let mut renderer = host.create_scene_renderer();
        pollster::block_on(renderer.reload_shader(host.device(), include_str!("material.wgsl")))?;
        let texture = renderer.upload_texture(host.device(), host.queue(), 1, 1, &[255; 4])?;

        let gizmo_geometry = renderer.upload_mesh(host.device(), &translation_gizmo()?)?;
        let gizmo_transform = renderer.create_transform(host.device(), Mat4::IDENTITY)?;
        let panel_transform = renderer.create_transform(host.device(), Mat4::IDENTITY)?;
        let panel_texture = if !self.standalone
            && (self.smoke_deadline.is_none()
                || self.prefab_smoke.is_some()
                || std::env::var_os("VOXY_RETARGET_ROTATION_SMOKE").is_some())
        {
            let panels =
                panels::Panels::with_registry(self.authoring.authoring_project.registry.clone())?;
            let texture =
                renderer.upload_texture(host.device(), host.queue(), 512, 128, &panels.rgba)?;
            self.panels = Some(panels);
            Some(texture)
        } else {
            None
        };
        let animated_models = animated_models::AnimatedModels::new(&renderer)?;
        self.graphics = Some(Graphics {
            animated_models,
            host,
            fog: None,
            renderer,
            models: BTreeMap::new(),
            residency_cache: gpu_model::ResidencyCache::with_budget(
                std::env::var("VOXY_GPU_IMAGE_BUDGET_BYTES")
                    .map_or(Ok(256 * 1024 * 1024), |value| value.parse::<u64>())?,
            ),
            residency_errors: BTreeMap::new(),
            optical_liquids: BTreeMap::new(),
            liquid_revision: None,
            liquid_geometry: None,
            liquid_transforms: BTreeMap::new(),
            gizmo_geometry,
            gizmo_transforms: BTreeMap::from([(0, gizmo_transform)]),
            texture,
            transforms: HashMap::new(),
            lod_history: HashMap::new(),
            lod_pending: BTreeSet::new(),
            panel_geometry: None,
            panel_texture,
            panel_transform,
            ui_draws: Vec::new(),
            ui_focus: Vec::new(),
            ui_focus_key: None,
            ui_focus_deferred: None,
            ui_presented: false,
        });
        if let Some(graphics) = &mut self.graphics {
            graphics.residency_cache.geometry_budget =
                std::env::var("VOXY_GPU_GEOMETRY_BUDGET_BYTES")
                    .map_or(Ok(256 * 1024 * 1024), |value| value.parse::<u64>())?;
        }
        window.set_visible(true);
        window.focus_window();
        window.request_redraw();
        self.window = Some(window);
        if self.smoke_deadline.is_some() {
            self.smoke_deadline = Some(Instant::now() + Duration::from_secs(15));
        }
        Ok(())
    }
    fn poll_sources(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(watcher) = &mut self.watcher else {
            return Ok(());
        };
        if let Some(changes) = watcher.try_result()? {
            self.scan_active = false;
            let affected = self.sources.affected(changes);
            if !affected.is_empty() {
                self.catalog.invalidate(&affected)?;
                self.reload.extend(affected);
            }
        }
        if !self.scan_active && self.last_scan.elapsed() >= Duration::from_millis(200) {
            watcher.request(&self.sources, 129)?;
            self.scan_active = true;
            self.last_scan = Instant::now();
        }
        Ok(())
    }
    #[allow(clippy::too_many_lines)]
    fn tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.poll_audio_device()?;
        if let Some(audio) = self.audio.play_mut() {
            audio.poll_reload(&self.scene)?;
        }
        self.queue_retarget_draft_source();
        for (_, profile) in self.scene.components::<ModelRetarget>() {
            let source = AssetId(profile.source.clone());
            if self.catalog.status(&source).is_none() {
                self.reload.insert(source);
            }
        }
        // Apply known invalidations before accepting any ready import completion.
        self.poll_sources()?;
        let Some(imports) = &mut self.imports else {
            return Ok(());
        };
        if let Some(mut completion) = imports.try_result()? {
            if let Ok(candidate) = &completion.result {
                let output = completion.ticket.asset();
                let referenced_parts: Vec<_> = self
                    .scene
                    .components::<ModelPart>()
                    .filter(|(node, _)| {
                        self.scene
                            .component::<ModelInstance>(*node)
                            .ok()
                            .flatten()
                            .is_some_and(|model| &model.asset == output)
                    })
                    .map(|(_, part)| part.node)
                    .collect();
                let invalid_part = referenced_parts.iter().any(|part| {
                    *part != u32::MAX
                        && usize::try_from(*part)
                            .map_or(true, |index| index >= candidate.value().nodes.len())
                });
                let changed_hierarchy = !referenced_parts.is_empty()
                    && self.catalog.snapshot(output).is_some_and(|previous| {
                        previous.value().nodes.len() != candidate.value().nodes.len()
                            || previous
                                .value()
                                .nodes
                                .iter()
                                .zip(&candidate.value().nodes)
                                .any(|(a, b)| a.parent != b.parent || a.name != b.name)
                    });
                if invalid_part || changed_hierarchy {
                    completion.result = completion.result.and_then(|candidate| Err(candidate.into_failed("imported hierarchy changed; preserve last good model and reimport explicitly".to_owned())));
                }
            }
            let output = completion.ticket.asset().clone();
            let successful = completion.result.is_ok();
            match self.catalog.complete_observed(
                &mut self.sources,
                &completion.ticket,
                completion.result,
            ) {
                Ok(()) if successful => {
                    let asset = self
                        .catalog
                        .snapshot(&output)
                        .ok_or("missing published model")?;
                    // Inspect the newly published mesh using disjoint fields;
                    // the import queue remains mutably borrowed until tick ends.
                    let required =
                        self.scene
                            .active_components::<ModelInstance>()
                            .any(|(node, model)| {
                                model.asset == output
                                    && self.catalog.snapshot(&model.asset).is_none_or(|asset| {
                                        asset
                                            .value()
                                            .mesh_for(
                                                self.scene
                                                    .component::<ModelPart>(node)
                                                    .ok()
                                                    .flatten()
                                                    .map(|part| part.node),
                                            )
                                            .is_some()
                                    })
                            });
                    if required && let Some(graphics) = &mut self.graphics {
                        graphics.residency_cache.geometry_live = graphics.geometry_bytes();
                        let model = ModelGraphics::upload(
                            &graphics.renderer,
                            &graphics.host,
                            asset.value(),
                            &mut graphics.residency_cache,
                        );
                        match model {
                            Ok(model) => {
                                graphics.models.insert(output.clone(), model);
                                graphics.residency_errors.remove(&output);
                            }
                            Err(error) => {
                                eprintln!("GPU MODEL RETAINED asset={} error={error}", output.0);
                                graphics.residency_errors.insert(
                                    output.clone(),
                                    gpu_model::DeferredUpload::new(
                                        &asset,
                                        &graphics.residency_cache,
                                        error.to_string(),
                                    ),
                                );
                            }
                        }
                    }
                    self.recovered |= self.failed_at.is_some();
                    self.last_publication = Some(Instant::now());
                    if let Some(window) = &self.window {
                        window.set_title("Voxy — model loaded");
                    }
                    println!(
                        "MODEL PUBLISHED frames={} first_x={} asset={}",
                        self.frames,
                        asset.value().mesh.vertices()[0].position[0],
                        output.0
                    );
                }
                Ok(()) => {
                    self.failed_at.get_or_insert(self.frames);
                    if let Some(window) = &self.window {
                        window.set_title("Voxy — import failed; showing previous model");
                    }
                    println!(
                        "MODEL FAILED frames={} last_good={}",
                        self.frames,
                        self.catalog.snapshot(&output).is_some()
                    );
                }
                Err(PublicationError::Asset(AssetError::StaleTicket)) => {}
                Err(error) => return Err(error.into()),
            }
        }
        let imports = self
            .imports
            .as_mut()
            .ok_or("import worker disappeared during completion")?;
        if !imports.is_busy()
            && let Some(id) = self.reload.first().cloned()
        {
            let ticket = self.catalog.request(id.clone())?;
            match imports.submit(&ticket) {
                Ok(()) => {
                    self.reload.remove(&id);
                }
                Err(voxy_assets::ImportWorkerError::Busy) => {}
                Err(error) => return Err(error.into()),
            }
        }
        self.synchronize_gpu_residency();
        Ok(())
    }
    fn required_cpu_model_assets(&self) -> BTreeSet<AssetId> {
        self.scene
            .components::<ModelInstance>()
            .map(|(_, model)| model.asset.clone())
            .chain(
                self.scene
                    .components::<ModelRetarget>()
                    .map(|(_, profile)| AssetId(profile.source.clone())),
            )
            .chain(std::iter::once(self.id.clone()))
            .collect()
    }
    fn required_gpu_assets(&self) -> std::collections::BTreeSet<AssetId> {
        self.scene
            .active_components::<ModelInstance>()
            .filter(|(node, model)| {
                self.catalog.snapshot(&model.asset).is_none_or(|asset| {
                    asset
                        .value()
                        .mesh_for(
                            self.scene
                                .component::<ModelPart>(*node)
                                .ok()
                                .flatten()
                                .map(|part| part.node),
                        )
                        .is_some()
                })
            })
            .map(|(_, model)| model.asset.clone())
            .collect()
    }
    /// Visible scene references own residency; authoring and CPU catalog stay intact.
    fn synchronize_gpu_residency(&mut self) {
        let required = self.required_gpu_assets();
        let Some(graphics) = &mut self.graphics else {
            return;
        };
        graphics.models.retain(|asset, _| required.contains(asset));
        graphics.residency_cache.geometry_live = graphics.geometry_bytes();
        graphics.residency_cache.prune();
        graphics
            .residency_errors
            .retain(|asset, _| required.contains(asset));
        for id in required {
            if graphics.models.contains_key(&id) && !graphics.residency_errors.contains_key(&id) {
                continue;
            }
            if let Some(asset) = self.catalog.snapshot(&id) {
                if graphics
                    .residency_errors
                    .get(&id)
                    .is_some_and(|deferred| deferred.blocks(&asset, &graphics.residency_cache))
                {
                    continue;
                }
                let model = ModelGraphics::upload(
                    &graphics.renderer,
                    &graphics.host,
                    asset.value(),
                    &mut graphics.residency_cache,
                );
                match model {
                    Ok(model) => {
                        graphics.models.insert(id.clone(), model);
                        graphics.residency_cache.geometry_live = graphics.geometry_bytes();
                        graphics.residency_errors.remove(&id);
                    }
                    Err(error) => {
                        let message = error.to_string();
                        if graphics
                            .residency_errors
                            .get(&id)
                            .map(|error| &error.message)
                            != Some(&message)
                        {
                            eprintln!("GPU MODEL DEFERRED asset={} error={error}", id.0);
                        }
                        graphics.residency_errors.insert(
                            id,
                            gpu_model::DeferredUpload::new(
                                &asset,
                                &graphics.residency_cache,
                                message,
                            ),
                        );
                    }
                }
            }
        }
    }
    fn check_game_ui(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = voxy_gameplay::SceneUiRuntime::new(128);
        ui.refresh(&self.scene, [1280.0, 720.0])?;
        let snapshot = ui.snapshot().ok_or("missing game UI snapshot")?;
        let count = snapshot.elements.len();
        let expected: Vec<_> = snapshot
            .elements
            .iter()
            .filter(|element| element.enabled && element.clip.is_some())
            .filter_map(|element| {
                element
                    .descriptor
                    .action
                    .as_ref()
                    .map(|action| voxy_gameplay::UiActionEvent {
                        owner: element.owner,
                        action: action.clone(),
                    })
            })
            .collect();
        for event in &expected {
            ui.traverse(false);
            ui.key_press();
            if ui.key_release().as_ref() != Some(event) {
                return Err("game UI keyboard target mismatch".into());
            }
        }
        let text = ui_text::UiTextPreparation::prepare_import(
            &self.scene,
            &self.authoring.authoring_project,
            [1280.0, 720.0],
        )?;
        let text = text.value();
        let meshes = ui_draw::build(ui.snapshot().ok_or("missing UI layout")?, &text.runs)?;
        let glyph_quads: usize = meshes
            .iter()
            .filter(|draw| draw.atlas.is_some())
            .map(|draw| draw.mesh.indices().len() / 6)
            .sum();
        if meshes.iter().any(|draw| {
            !ui.snapshot().is_some_and(|snapshot| {
                snapshot
                    .elements
                    .iter()
                    .any(|element| element.owner == draw.owner)
            })
        }) {
            return Err("UI mesh owner missing".into());
        }
        println!(
            "GAME UI MESH draws={} glyph_quads={glyph_quads}",
            meshes.len()
        );
        println!(
            "GAME UI TEXT fonts={} runs={} glyphs={}",
            text.font_count(),
            text.runs.len(),
            text.runs
                .iter()
                .map(|run| run.run.glyphs().len())
                .sum::<usize>()
        );
        println!("GAME UI LAYOUT elements={count} buttons={}", expected.len());
        Ok(())
    }
    fn advance_game(&mut self, elapsed: f64) -> Result<(), Box<dyn std::error::Error>> {
        let mut models: BTreeMap<_, _> = if let Some(graphics) = &self.graphics {
            graphics
                .models
                .iter()
                .filter_map(|(id, resource)| {
                    resource
                        .animated_model
                        .as_ref()
                        .map(|model| (id.clone(), model.clone()))
                })
                .collect()
        } else {
            self.scene
                .components::<ModelInstance>()
                .filter_map(|(_, instance)| {
                    let resource = self.catalog.snapshot(&instance.asset)?;
                    Some((
                        instance.asset.clone(),
                        resource.value().animated.as_ref()?.clone(),
                    ))
                })
                .collect()
        };
        for (_, profile) in self.scene.components::<ModelRetarget>() {
            let source = AssetId(profile.source.clone());
            if let Some(asset) = self.catalog.snapshot(&source)
                && let Some(model) = asset.value().animated.as_ref()
            {
                models.insert(source, model.clone());
            }
        }
        self.play.animations.synchronize(&self.scene)?;
        if let Some(simulation) = &mut self.play.simulation {
            if let Some(actions) = &mut self.play.ui_actions {
                for result in actions.dispatch(&self.scene, simulation.commands())? {
                    if let Err(error) = result {
                        eprintln!("GAME UI ACTION FAILED: {error}");
                    }
                }
            }
            if let Some(physics) = &mut self.play.physics {
                physics.synchronize(&self.scene)?;
            }
            let frame_result = simulation.advance_scoped_with_animation_events(
                &mut self.scene,
                elapsed,
                animation_runtime::schedule()?,
                |system, mut access, dt| {
                    if system == "angular.step" {
                        if let Some(motion) = &mut self.play.angular_motion {
                            motion
                                .fixed_scoped(access, dt)
                                .map_err(voxy_gameplay::GameplayFixedError::Motion)?;
                        }
                    } else if system == "character.step" {
                        access.require_write("liquid.world").map_err(|e| {
                            voxy_gameplay::GameplayFixedError::Motion(e.to_string())
                        })?;
                        let mut liquid = self.play.liquid.clone();
                        if let Some(runtime) = &mut liquid {
                            runtime
                                .tick(
                                    access.read().map_err(|e| {
                                        voxy_gameplay::GameplayFixedError::Motion(e.to_string())
                                    })?,
                                    dt,
                                    None,
                                )
                                .map_err(voxy_gameplay::GameplayFixedError::Motion)?;
                        }
                        access
                            .require_write("animation.playback")
                            .map_err(|error| {
                                voxy_gameplay::GameplayFixedError::Motion(error.to_string())
                            })?;
                        let candidate = if let Some(physics) = &mut self.play.physics {
                            access.require_write("character.physics").map_err(|error| {
                                voxy_gameplay::GameplayFixedError::Motion(error.to_string())
                            })?;
                            access.require_write("player.input").map_err(|error| {
                                voxy_gameplay::GameplayFixedError::Motion(error.to_string())
                            })?;
                            self.play
                                .animations
                                .fixed_step(
                                    access.write().map_err(|error| {
                                        voxy_gameplay::GameplayFixedError::Motion(error.to_string())
                                    })?,
                                    &models,
                                    physics,
                                    &mut self.play.player_input,
                                    dt,
                                )
                                .map_err(|error| match error {
                                    voxy_gameplay::CharacterTickError::Physics(error) => {
                                        voxy_gameplay::GameplayFixedError::Physics(error)
                                    }
                                    voxy_gameplay::CharacterTickError::Preparation(error) => {
                                        voxy_gameplay::GameplayFixedError::Motion(error)
                                    }
                                })?
                        } else {
                            let candidate = self
                                .play
                                .animations
                                .prepare_wall(
                                    access.read().map_err(|error| {
                                        voxy_gameplay::GameplayFixedError::Motion(error.to_string())
                                    })?,
                                    &models,
                                    dt,
                                )
                                .map_err(voxy_gameplay::GameplayFixedError::Motion)?;
                            if candidate.requires_pose_preparation()
                                || !candidate.motions().is_empty()
                                || !candidate.trajectories().is_empty()
                            {
                                return Err(voxy_gameplay::GameplayFixedError::Motion(
                                    "root motion requires a physics runtime".into(),
                                ));
                            }
                            access.require_write("player.input").map_err(|error| {
                                voxy_gameplay::GameplayFixedError::Motion(error.to_string())
                            })?;
                            self.play.player_input.finish_frame();
                            candidate
                        };
                        if let Some(runtime) = &mut liquid {
                            runtime
                                .publish_body_pose(access.write().map_err(|e| {
                                    voxy_gameplay::GameplayFixedError::Motion(e.to_string())
                                })?)
                                .map_err(voxy_gameplay::GameplayFixedError::Motion)?;
                        }
                        self.play.animations = candidate;
                        self.play.liquid = liquid;
                    } else if let Some(physics) = &mut self.play.physics {
                        physics
                            .run_scoped_system(system, access, &mut self.play.player_input, dt)
                            .map_err(voxy_gameplay::GameplayFixedError::Physics)?;
                    }
                    Ok::<_, voxy_gameplay::GameplayFixedError>(
                        self.play
                            .animations
                            .take_events()
                            .into_iter()
                            .map(|(owner, event)| (owner, event.name, event.phase))
                            .collect(),
                    )
                },
            );
            if let Err(voxy_scene::SimulationStepError::System {
                completed_steps, ..
            }) = &frame_result
            {
                self.play.simulation_ticks = self
                    .play
                    .simulation_ticks
                    .saturating_add(u64::try_from(*completed_steps)?);
            }
            let frame = frame_result?;
            if let Some(audio) = self.audio.play_mut() {
                audio_play::schedule()?
                    .run_scene(&mut self.scene, |_, access| {
                        audio.advance_scoped(access, frame.time.steps)
                    })
                    .map_err(|error| error.to_string())?;
            }
            self.play.simulation_ticks = self
                .play
                .simulation_ticks
                .saturating_add(u64::try_from(frame.time.steps)?);
            for result in frame.commands {
                result?;
            }
            if let Some(physics) = &mut self.play.physics {
                physics.synchronize(&self.scene)?;
            }
        }
        Ok(())
    }
    fn start_standalone(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let required = self.required_cpu_model_assets();
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            self.tick()?;
            for id in &required {
                if let Some(voxy_assets::AssetStatus::Failed(error)) = self.catalog.status(id) {
                    return Err(format!("game asset {} failed: {error}", id.0).into());
                }
            }
            if required
                .iter()
                .all(|id| self.catalog.snapshot(id).is_some())
            {
                break;
            }
            if Instant::now() >= deadline {
                return Err("game resource imports timed out".into());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        self.validate_authoring_document(&self.authoring_document()?)?;
        self.standalone = true;
        self.toggle_play()
    }
    #[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
    fn draw(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let profile_start = self
            .animation_smoke
            .as_ref()
            .is_some_and(|s| s.profile)
            .then(Instant::now);
        self.tick()?;
        let elapsed = self.play.simulation_time.elapsed().as_secs_f64();
        self.play.simulation_time = Instant::now();
        self.advance_game(elapsed)?;
        voxy_scene::extraction_schedule()?
            .run_scene(&mut self.scene, |_, access| {
                let render_world = |scene: &voxy_scene::SceneGraph, owner| {
                    if let Some(simulation) = &self.play.simulation {
                        simulation
                            .render_world(scene, owner)
                            .map_err(|error| match error {
                                voxy_scene::SimulationError::Scene(error) => error,
                                _ => voxy_scene::SceneGraphError::InvalidNode,
                            })
                    } else {
                        scene.world_matrix(owner)
                    }
                };
                let styles = frame_styles::FrameStyles::prepare_with_world(
                    &access,
                    &self.camera,
                    scene_limits::OBJECTS,
                    &render_world,
                )?;
                self.extraction
                    .refresh_scoped_with(access, render_world)
                    .map_err(|error| error.to_string())?;
                self.frame_styles = Some(styles);
                Ok::<(), String>(())
            })
            .map_err(|error| error.to_string())?;
        if let Some(window) = &self.window
            && self.panels.is_some()
        {
            let size = window.inner_size().to_logical::<f32>(window.scale_factor());
            if size.width > 0.0 && size.height > 0.0 {
                let document = self.panel_document()?;
                self.reconcile_component_edit(&document)?;
                self.panels.as_mut().unwrap().preview_phase = self
                    .authoring
                    .animation_preview
                    .as_ref()
                    .filter(|preview| self.instances.get(self.selected) == Some(&preview.owner))
                    .map(|preview| preview.phase);
                self.panels.as_mut().unwrap().retarget_draft = self.retarget_draft.is_some();
                self.panels.as_mut().unwrap().bone_picker = self
                    .retarget_picker
                    .as_ref()
                    .map(|p| (p.choices.clone(), p.page));
                let input = PanelInput {
                    prefab_metadata: self
                        .authoring
                        .history
                        .as_ref()
                        .map_or(serde_json::Value::Null, |history| {
                            history.metadata().clone()
                        }),
                    focused: self
                        .panels
                        .as_ref()
                        .and_then(|panels| panels.focus.focused()),
                    document,
                    selected: self.selected,
                    size: [size.width, size.height],
                    playing: self.play.playing.is_some(),
                    inspector: self.inspector,
                    texture_label: self
                        .instances
                        .get(self.selected)
                        .and_then(|node| {
                            let model = self
                                .scene
                                .component::<ModelInstance>(*node)
                                .ok()
                                .flatten()?;
                            let asset = self.catalog.snapshot(&model.asset)?;
                            let part = self.scene.component::<ModelPart>(*node).ok().flatten()?;
                            asset.value().nodes.get(part.node as usize)?.image
                        })
                        .map_or_else(|| "Texture: none".into(), |_| "Texture: PNG/JPEG".into()),
                    prefab_label: self
                        .authoring
                        .prefab_assets
                        .get(self.authoring.prefab_choice)
                        .map_or_else(|| "Prefab: none".into(), |id| format!("Prefab: {}", id.0)),
                    import_config: self.settings_config(),
                    field: self.field.clone(),
                    scroll: self.tree_scroll,
                    parenting: self.parenting.is_some(),
                };
                if self.panel_cache.as_ref() != Some(&input) {
                    let overrides = self.prefab_overridden_fields(&input.document)?;
                    let collection_resets = self.prefab_collection_resets(&input.document)?;
                    let order_resets = self.prefab_collection_order_resets(&input.document)?;
                    let deleted_items = self.prefab_collection_deleted_items(&input.document)?;
                    let deleted_resets = deleted_items.keys().copied().collect();
                    self.panels
                        .as_mut()
                        .ok_or("missing panels")?
                        .collection_deleted_resets = deleted_resets;
                    self.panels
                        .as_mut()
                        .ok_or("missing panels")?
                        .collection_deleted_items = deleted_items;
                    self.panels
                        .as_mut()
                        .ok_or("missing panels")?
                        .collection_order_resets = order_resets;
                    self.panels
                        .as_mut()
                        .ok_or("missing panels")?
                        .collection_resets = collection_resets;
                    self.panels
                        .as_mut()
                        .ok_or("missing panels")?
                        .overridden_fields = overrides;
                    let mesh = self.panels.as_mut().ok_or("missing panels")?.build(
                        &input.document,
                        input.selected,
                        Vec2::from_array(input.size),
                        input.playing,
                        input.field.as_ref().map(|(id, text)| (*id, text.as_str())),
                        input.scroll,
                        input.parenting,
                        input.inspector,
                        &input.texture_label,
                        &input.prefab_label,
                        input.import_config,
                    )?;
                    if let Some(graphics) = &mut self.graphics {
                        graphics.panel_geometry = Some(
                            graphics
                                .renderer
                                .upload_mesh(graphics.host.device(), &mesh)?,
                        );
                    }
                    self.panel_cache = Some(input);
                }
            }
        }
        let viewport = self.window.as_ref().map_or([1, 1], |window| {
            let size = window.inner_size();
            [size.width.max(1), size.height.max(1)]
        });
        if let Some(graphics) = &mut self.graphics {
            if self.play.playing.is_some() || self.standalone {
                if let Err(error) = self.ui_live.poll(
                    &self.scene,
                    &self.authoring.authoring_project,
                    [viewport[0] as f32, viewport[1] as f32],
                    graphics,
                ) {
                    eprintln!("GAME UI publication retained previous resources: {error}");
                }
            } else {
                self.retired_ui_workers.extend(self.ui_live.close());
                graphics.ui_draws.clear();
                graphics.ui_presented = false;
            }
        }
        let ring = self
            .ui_live
            .focus_snapshot()
            .and_then(|snapshot| self.game_ui_input.focus_ring(snapshot));
        if let Some(graphics) = &mut self.graphics {
            graphics.residency_cache.geometry_live = graphics.geometry_bytes();
            let deferred = ring.as_ref().is_some_and(|ring| {
                graphics
                    .ui_focus_deferred
                    .as_ref()
                    .is_some_and(|(prior, state)| {
                        prior == ring && *state == graphics.residency_cache.state()
                    })
            });
            if graphics.ui_focus_key != ring && !deferred {
                if let Some(ring) = &ring {
                    let mesh = ring.mesh()?;
                    graphics.residency_cache.geometry_live = graphics.geometry_bytes();
                    match ui_live::upload_on(
                        &graphics.renderer,
                        graphics.host.device(),
                        graphics.host.queue(),
                        &mut graphics.residency_cache,
                        &[mesh],
                    ) {
                        Ok(draws) => {
                            graphics.ui_focus = draws;
                            graphics.ui_focus_key = Some(ring.clone());
                            graphics.ui_focus_deferred = None;
                        }
                        Err(error) => {
                            graphics.ui_focus.clear();
                            graphics.ui_focus_key = None;
                            graphics.residency_cache.geometry_live = graphics.geometry_bytes();
                            graphics.ui_focus_deferred =
                                Some((ring.clone(), graphics.residency_cache.state()));
                            eprintln!("GAME UI FOCUS DEFERRED: {error}");
                        }
                    }
                } else {
                    graphics.ui_focus.clear();
                    graphics.ui_focus_key = None;
                    graphics.ui_focus_deferred = None;
                }
            }
        }
        let styles = self
            .frame_styles
            .as_ref()
            .ok_or("missing presentation styles")?;
        let split = self.split_views() && viewport[0] >= 2;
        let mut views = Vec::new();
        for (index, region) in view_layout::regions(viewport, split)
            .into_iter()
            .enumerate()
        {
            let id = index as u8;
            let camera = if !split || id == self.active_view {
                &styles.camera
            } else {
                self.secondary_camera
                    .as_ref()
                    .ok_or("missing second camera")?
            };
            let size = Vec2::new(region[2] as f32, region[3] as f32);
            views.push((id, region, camera.matrix(size)?, camera.scene_camera(size)?));
        }
        if views.is_empty() {
            return Ok(());
        }
        let animation_requests = if self.play.playing.is_some()
            || self.standalone
            || self.authoring.animation_preview.is_some()
        {
            self.extraction
                .instances()
                .iter()
                .filter_map(|instance| {
                    if styles.parts.contains_key(&instance.owner) {
                        return None;
                    }
                    let published = self
                        .graphics
                        .as_ref()?
                        .models
                        .get(&instance.component.asset)?;
                    let model = published.animated_model.as_ref()?.clone();
                    let settings = self
                        .scene
                        .component::<ModelAnimation>(instance.owner)
                        .ok()
                        .flatten()
                        .cloned()
                        .unwrap_or(ModelAnimation {
                            clip: (!model.animations.is_empty()).then_some(0),
                            speed: 1.0,
                            ..ModelAnimation::default()
                        });
                    let frame = if self.play.playing.is_some() || self.standalone {
                        self.play.animations.frame(instance.owner, &model)
                    } else {
                        Some(self.preview_animation_frame(instance.owner, &model, &settings)?)
                    };
                    Some(animated_models::Request {
                        owner: instance.owner,
                        frame,
                        model,
                        settings,
                        lod: published.animated_lod.clone(),
                        textures: published.animated_textures.clone(),
                        texture_storage: published._images.clone(),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let optical_particles = if !self.msaa4 && views.iter().any(|v| v.3.is_some()) {
            self.play
                .liquid
                .as_ref()
                .map(|runtime| {
                    liquid_draw::optical_particles(runtime.liquid(), &self.play.liquid_optics)
                })
                .transpose()?
                .flatten()
        } else {
            None
        };
        if let Some(graphics) = &mut self.graphics {
            let liquid_revision = self
                .play
                .liquid
                .as_ref()
                .map(|_| (self.scene.identity(), self.play.simulation_ticks));
            if graphics.liquid_revision != liquid_revision {
                let mesh = self
                    .play
                    .liquid
                    .as_ref()
                    .map(|r| liquid_draw::mesh(r.liquid()))
                    .transpose()?
                    .flatten();
                if let Some(mesh) = &mesh {
                    let bytes = (mesh.vertices().len()
                        * std::mem::size_of::<voxy_render::SceneVertex>()
                        + mesh.indices().len() * 4) as u64;
                    if graphics.geometry_bytes().saturating_add(bytes)
                        > graphics.residency_cache.geometry_budget
                    {
                        return Err("liquid GPU geometry budget exceeded".into());
                    }
                }
                graphics.liquid_geometry = mesh
                    .as_ref()
                    .map(|mesh| graphics.renderer.upload_mesh(graphics.host.device(), mesh))
                    .transpose()?;
                graphics.liquid_revision = liquid_revision;
            }
            if let Some(particles) = &optical_particles {
                graphics
                    .optical_liquids
                    .retain(|id, _| views.iter().any(|v| v.0 == *id && v.3.is_some()));
                for &(id, region, _, camera) in &views {
                    let Some(camera) = camera else {
                        continue;
                    };
                    if graphics
                        .optical_liquids
                        .get(&id)
                        .is_none_or(|r| r.size() != region[2..])
                    {
                        let candidate =
                            voxy_render::ScreenSpaceFluidRenderer::new_with_adapter_budget(
                                graphics.host.device(),
                                graphics.host.adapter(),
                                graphics.host.color_format(),
                                region[2],
                                region[3],
                                16384,
                                graphics.geometry_bytes(),
                                graphics.residency_cache.geometry_budget,
                            )?;
                        graphics.optical_liquids.insert(id, candidate);
                    }
                    graphics.optical_liquids.get_mut(&id).unwrap().update(
                        graphics.host.queue(),
                        camera,
                        particles,
                        1.,
                        voxy_render::FluidDepthFilter::default(),
                    )?;
                }
            } else {
                graphics.optical_liquids.clear();
            }
            let animation_start = profile_start.map(|_| Instant::now());
            let other_live = graphics
                .geometry_bytes()
                .saturating_sub(graphics.animated_models.allocation_bytes());
            if self.play.playing.is_none()
                && !self.standalone
                && self.authoring.animation_preview.is_none()
            {
                graphics.animated_models.clear();
            } else {
                for (owner, error) in graphics.animated_models.synchronize(
                    &graphics.renderer,
                    graphics.host.device(),
                    graphics.host.queue(),
                    animation_requests,
                    self.play.animations.serial(),
                    other_live,
                    graphics.residency_cache.geometry_budget,
                ) {
                    eprintln!("MODEL ANIMATION retained previous frame for {owner:?}: {error}");
                }
            }
            let animation_us =
                animation_start.map_or(0., |start| start.elapsed().as_secs_f64() * 1e6);
            graphics.transforms.retain(|(view, owner), _| {
                views.iter().any(|v| v.0 == *view) && styles.materials.contains_key(owner)
            });
            graphics.lod_history.retain(|(view, owner), _| {
                views.iter().any(|v| v.0 == *view) && styles.materials.contains_key(owner)
            });
            graphics
                .gizmo_transforms
                .retain(|view, _| views.iter().any(|v| v.0 == *view));
            let selected = self.instances.get(self.selected).and_then(|node| {
                self.extraction
                    .instances()
                    .iter()
                    .find(|instance| instance.owner == *node)
            });
            graphics.animated_models.retain_lod_views(
                &views
                    .iter()
                    .filter(|view| view.3.is_some())
                    .map(|view| view.0)
                    .collect::<Vec<_>>(),
            );
            let animation_other_live = graphics
                .geometry_bytes()
                .saturating_sub(graphics.animated_models.allocation_bytes());
            let lod_start = profile_start.map(|_| Instant::now());
            for &(view, region, _, camera) in &views {
                for instance in self.extraction.instances() {
                    if let Err(error) = graphics.animated_models.select_lod(
                        &graphics.renderer,
                        graphics.host.device(),
                        instance.owner,
                        view,
                        camera,
                        instance.world,
                        [region[2], region[3]],
                        animation_other_live,
                        graphics.residency_cache.geometry_budget,
                    ) {
                        eprintln!("ANIMATED LOD retained previous selection: {error}");
                    }
                }
            }
            graphics.animated_models.evict_unused_lod();
            let lod_us = lod_start.map_or(0., |start| start.elapsed().as_secs_f64() * 1e6);
            let mut requirements = Vec::new();
            for &(view, region, view_projection, lod_camera) in &views {
                if graphics.liquid_geometry.is_some() {
                    let transform = match graphics.liquid_transforms.entry(view) {
                        std::collections::btree_map::Entry::Occupied(e) => e.into_mut(),
                        std::collections::btree_map::Entry::Vacant(e) => e.insert(
                            graphics
                                .renderer
                                .create_transform(graphics.host.device(), Mat4::IDENTITY)?,
                        ),
                    };
                    transform.update(graphics.host.queue(), view_projection)?;
                }
                for instance in self.extraction.instances() {
                    let key = (view, instance.owner);
                    let transform = match graphics.transforms.entry(key) {
                        std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
                        std::collections::hash_map::Entry::Vacant(entry) => entry.insert(
                            graphics
                                .renderer
                                .create_transform(graphics.host.device(), instance.world)?,
                        ),
                    };
                    transform.update(graphics.host.queue(), view_projection * instance.world)?;
                    let material = styles
                        .materials
                        .get(&instance.owner)
                        .copied()
                        .unwrap_or_default();
                    let light =
                        styles
                            .light
                            .filter(|_| material.lit)
                            .map_or([0., 0., 1., 0.], |light| {
                                [
                                    light.direction[0],
                                    light.direction[1],
                                    light.direction[2],
                                    light.intensity,
                                ]
                            });
                    transform.update_scene_material(
                        graphics.host.queue(),
                        instance.world,
                        material.tint,
                        light,
                    )?;
                    if !styles.parts.contains_key(&instance.owner) {
                        requirements.push(gpu_model::LodViewRequest {
                            asset: &instance.component.asset,
                            camera: lod_camera,
                            world: instance.world,
                            viewport: [region[2], region[3]],
                            previous: graphics
                                .lod_history
                                .get(&key)
                                .and_then(voxy_render::SceneLodHistory::previous),
                        });
                    }
                }
                if let Some(instance) = selected {
                    let transform = match graphics.gizmo_transforms.entry(view) {
                        std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
                        std::collections::btree_map::Entry::Vacant(entry) => entry.insert(
                            graphics
                                .renderer
                                .create_transform(graphics.host.device(), Mat4::IDENTITY)?,
                        ),
                    };
                    transform.update(
                        graphics.host.queue(),
                        view_projection * Mat4::from_translation(instance.world.w_axis.truncate()),
                    )?;
                }
            }
            let requested = gpu_model::collect_lod_requests(&graphics.models, requirements);
            let other_bytes = graphics.geometry_bytes()
                - graphics
                    .models
                    .values()
                    .map(ModelGraphics::geometry_bytes)
                    .sum::<u64>();
            let (live, pending) = gpu_model::reconcile_lod_residency(
                &graphics.renderer,
                graphics.host.device(),
                &mut graphics.models,
                &requested,
                other_bytes,
                graphics.residency_cache.geometry_budget,
            );
            if pending != graphics.lod_pending && std::env::var_os("VOXY_LOD_TRACE").is_some() {
                eprintln!(
                    "LOD RESIDENCY live={live} budget={} pending={pending:?}",
                    graphics.residency_cache.geometry_budget
                );
            }
            graphics.lod_pending = pending;
            graphics.residency_cache.geometry_live = live;
            let mut view_draws = Vec::new();
            let mut view_casters = Vec::new();
            let mut gpu_caster_draws = 0;
            let mut gpu_caster_owners = Vec::new();
            for &(view, region, _, lod_camera) in &views {
                let mut draws = Vec::with_capacity(self.extraction.instances().len() + 2);
                let mut casters = Vec::new();
                for instance in self.extraction.instances() {
                    if let Some(geometries) = graphics
                        .animated_models
                        .geometry_inputs_for_view(instance.owner, view)
                    {
                        for (primitive, posed) in geometries.enumerate() {
                            let geometry = posed.geometry;
                            let texture = graphics
                                .animated_models
                                .texture(instance.owner, primitive)
                                .unwrap_or(&graphics.texture);
                            if styles.casters.contains(&instance.owner) {
                                gpu_caster_draws += usize::from(posed.gpu_deformed);
                                if self.trace.enabled
                                    && posed.gpu_deformed
                                    && !gpu_caster_owners.contains(&instance.owner)
                                {
                                    gpu_caster_owners.push(instance.owner);
                                }
                                casters.push(fog_draw::OpaqueCaster {
                                    geometry,
                                    world: instance.world,
                                });
                            }
                            draws.push(SceneDraw {
                                geometry,
                                texture,
                                transform: &graphics.transforms[&(view, instance.owner)],
                                overlay: false,
                            });
                        }
                        continue;
                    }
                    if let Some(model) = graphics.models.get(&instance.component.asset)
                        && !styles.parts.contains_key(&instance.owner)
                        && !model.animated_preview.is_empty()
                    {
                        for (primitive, geometry) in model.animated_preview.iter().enumerate() {
                            if styles.casters.contains(&instance.owner) {
                                casters.push(fog_draw::OpaqueCaster {
                                    geometry,
                                    world: instance.world,
                                });
                            }
                            draws.push(SceneDraw {
                                geometry,
                                texture: model.animated_textures[primitive]
                                    .as_deref()
                                    .unwrap_or(&graphics.texture),
                                transform: &graphics.transforms[&(view, instance.owner)],
                                overlay: false,
                            });
                        }
                        continue;
                    }
                    if let Some(model) = graphics.models.get(&instance.component.asset)
                        && let Some((geometry, _, texture)) =
                            model.part(styles.parts.get(&instance.owner).copied())
                    {
                        let geometry = if !styles.parts.contains_key(&instance.owner) {
                            let history = graphics
                                .lod_history
                                .entry((view, instance.owner))
                                .or_default();
                            let previous = history.previous();
                            let geometry = model.geometry.for_view(
                                lod_camera,
                                instance.world,
                                [region[2], region[3]],
                                history,
                            );
                            if previous != history.previous()
                                && std::env::var_os("VOXY_LOD_TRACE").is_some()
                            {
                                eprintln!(
                                    "LOD DRAW view={view} asset={} level={:?} triangles={} viewport={:?}",
                                    instance.component.asset.0,
                                    history.previous(),
                                    geometry.index_count() / 3,
                                    [region[2], region[3]]
                                );
                            }
                            geometry
                        } else {
                            geometry
                        };
                        if styles.casters.contains(&instance.owner) {
                            casters.push(fog_draw::OpaqueCaster {
                                geometry,
                                world: instance.world,
                            });
                        }
                        draws.push(SceneDraw {
                            geometry,
                            texture: texture.unwrap_or(&graphics.texture),
                            transform: &graphics.transforms[&(view, instance.owner)],
                            overlay: false,
                        });
                    }
                }
                if !self.standalone
                    && let Some(instance) = selected
                    && graphics
                        .animated_models
                        .geometries(instance.owner)
                        .is_none()
                    && let Some(model) = graphics.models.get(&instance.component.asset)
                    && let Some((_, Some(outline), _)) =
                        model.part(styles.parts.get(&instance.owner).copied())
                    && let Some(transform) = graphics.transforms.get(&(view, instance.owner))
                {
                    draws.push(SceneDraw {
                        geometry: outline,
                        texture: &graphics.texture,
                        transform,
                        overlay: true,
                    });
                }
                if self.play.playing.is_none()
                    && let Some(instance) = selected
                    && graphics.models.contains_key(&instance.component.asset)
                    && let Some(transform) = graphics.gizmo_transforms.get(&view)
                {
                    draws.push(SceneDraw {
                        geometry: &graphics.gizmo_geometry,
                        texture: &graphics.texture,
                        transform,
                        overlay: true,
                    });
                }
                if !graphics.optical_liquids.contains_key(&view)
                    && let Some(geometry) = &graphics.liquid_geometry
                {
                    draws.push(SceneDraw {
                        geometry,
                        texture: &graphics.texture,
                        transform: &graphics.liquid_transforms[&view],
                        overlay: false,
                    });
                }
                view_draws.push(draws);
                view_casters.push(casters);
            }
            let mut overlays = Vec::new();
            for ui in graphics.ui_draws.iter().chain(graphics.ui_focus.iter()) {
                if styles.ui_owners.contains(&ui.owner) {
                    overlays.push(SceneDraw {
                        geometry: &ui.geometry,
                        texture: ui.texture.as_deref().unwrap_or(&graphics.texture),
                        transform: &graphics.panel_transform,
                        overlay: true,
                    });
                }
            }
            if let (Some(geometry), Some(texture)) =
                (&graphics.panel_geometry, &graphics.panel_texture)
            {
                overlays.push(SceneDraw {
                    geometry,
                    texture,
                    transform: &graphics.panel_transform,
                    overlay: true,
                });
            }
            let draw_count = view_draws.iter().map(Vec::len).sum::<usize>() + overlays.len();
            let present_start = profile_start.map(|_| Instant::now());
            if styles.fog.is_empty() {
                if let Some(fog) = &mut graphics.fog
                    && fog.deactivate(graphics.host.device())?
                {
                    graphics.fog = None;
                }
            }
            let outcome = if let Some((_, volume, bounds)) = styles.fog.first() {
                if styles.fog.len() != 1 {
                    return Err("multiple overlapping fog volumes are not yet composed".into());
                }
                if self.msaa4 {
                    return Err("editor fog requires single-sample scene inputs".into());
                }
                if !graphics.optical_liquids.is_empty() {
                    return Err("fog/liquid merged optical depth is not yet available".into());
                }
                if graphics.fog.is_none() {
                    graphics.fog = Some(pollster::block_on(fog_draw::FogDraw::new(
                        graphics.host.device(),
                        graphics.host.color_format(),
                    ))?);
                }
                let window = self
                    .window
                    .as_ref()
                    .ok_or("missing fog window")?
                    .inner_size();
                let device = graphics.host.device().clone();
                let other = graphics
                    .geometry_bytes()
                    .saturating_sub(graphics.fog.as_ref().unwrap().allocation_bytes());
                let budget = graphics
                    .residency_cache
                    .geometry_budget
                    .checked_sub(other)
                    .ok_or("fog scene memory budget exceeded")?;
                let fog = graphics.fog.as_mut().unwrap();
                fog.prepare_targets_with_shadow(
                    &device,
                    [window.width, window.height],
                    &views,
                    styles.shadow,
                    view_casters.iter().map(Vec::len).sum(),
                    budget,
                )?;
                let result = graphics.host.render_custom(|encoder, output| {
                    fog.encode(
                        &device,
                        encoder,
                        output,
                        &graphics.renderer,
                        &views,
                        &view_draws,
                        &overlays,
                        *volume,
                        *bounds,
                        styles.light,
                        &view_casters,
                        styles.shadow.map(|(_, settings)| settings),
                    )
                });
                if matches!(&result, Ok(voxy_render::RenderOutcome::Presented)) {
                    fog.submitted(graphics.host.queue());
                } else {
                    // render_custom has already discarded any failed encoder.
                    fog.discard_unsubmitted();
                }
                result?
            } else if !split || views.len() == 1 {
                let mut draws = view_draws.pop().ok_or("missing view draws")?;
                draws.extend(overlays);
                if let Some(fluid) = graphics.optical_liquids.get(&views[0].0) {
                    graphics.host.render_custom(|encoder, output| {
                        fluid.encode(
                            &graphics.renderer,
                            encoder,
                            output,
                            wgpu::Color::BLACK,
                            &draws,
                        );
                        Ok::<_, voxy_render::RendererError>(())
                    })?
                } else {
                    graphics.host.render_scene(&graphics.renderer, &draws)?
                }
            } else {
                let scene_views: Vec<_> = views
                    .iter()
                    .zip(&view_draws)
                    .map(|(view, draws)| voxy_render::SceneView {
                        viewport: view.1,
                        draws,
                    })
                    .collect();
                if graphics.optical_liquids.is_empty() {
                    graphics
                        .host
                        .render_scene_views(&graphics.renderer, &scene_views, &overlays)?
                } else {
                    let fluids: Vec<_> = views
                        .iter()
                        .enumerate()
                        .filter_map(|(index, view)| {
                            graphics.optical_liquids.get(&view.0).map(|f| (index, f))
                        })
                        .collect();
                    graphics.host.render_scene_views_with_fluids(
                        &graphics.renderer,
                        &scene_views,
                        &fluids,
                        &overlays,
                    )?
                }
            };
            if let Some(start) = profile_start {
                println!(
                    "ANIMATION FRAME PROFILE frame={} playing={} views={} draws={} animation_us={animation_us:.3} lod_us={lod_us:.3} submit_present_us={:.3} total_cpu_us={:.3} outcome={outcome:?}",
                    self.frames,
                    self.play.playing.is_some(),
                    views.len(),
                    draw_count,
                    present_start.unwrap().elapsed().as_secs_f64() * 1e6,
                    start.elapsed().as_secs_f64() * 1e6
                );
            }
            if self.trace.enabled && self.trace.pending && self.trace.last_outcome != Some(outcome)
            {
                println!(
                    "EDITOR PRESENT frames={} draws={} outcome={outcome:?} positions={:?}",
                    self.frames,
                    draw_count,
                    self.extraction
                        .instances()
                        .iter()
                        .map(|instance| (instance.owner, instance.world.w_axis))
                        .collect::<Vec<_>>()
                );
            }
            self.trace.last_outcome = Some(outcome);
            if let Some(panels) = &mut self.panels {
                panels.frame_outcome(outcome);
            }
            if outcome == RenderOutcome::Presented {
                if !styles.fog.is_empty() && self.trace.enabled {
                    println!(
                        "EDITOR FOG PRESENT frame={} views={} owner={:?} shadow={} casters={} gpu_casters={}",
                        self.frames + 1,
                        views.len(),
                        styles.fog[0].0,
                        styles.shadow.is_some(),
                        view_casters.iter().map(Vec::len).sum::<usize>(),
                        gpu_caster_draws
                    );
                    for owner in &gpu_caster_owners {
                        if let Some((ticks, palette)) =
                            graphics.animated_models.pose_evidence(*owner)
                        {
                            println!(
                                "EDITOR FOG GPU POSE frame={} owner={owner:?} ticks={ticks} palette={palette}",
                                self.frames + 1
                            );
                        }
                    }
                }
                self.ui_live.frame_presented();
                graphics.ui_presented |= self.ui_live.ready();
                self.trace.pending = false;
                self.frames += 1;
            }
        }
        Ok(())
    }

    fn split_views(&self) -> bool {
        self.secondary_camera.is_some() && self.play.playing.is_none() && !self.standalone
    }
    fn activate_view(&mut self, view: u8) -> Result<(), Box<dyn std::error::Error>> {
        if view != self.active_view && self.split_views() {
            self.finish_drag(false)?;
            self.camera_drag = None;
            if let Some(other) = &mut self.secondary_camera {
                std::mem::swap(&mut self.camera, other);
            }
            self.active_view = view;
            self.update_edit_title();
        }
        Ok(())
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn view_pointer(
        &mut self,
        cursor: Vec2,
        size: Vec2,
    ) -> Result<Option<(Vec2, Vec2, Vec2)>, Box<dyn std::error::Error>> {
        if !self.split_views() || size.x < 2. {
            return Ok(Some((cursor, size, Vec2::ZERO)));
        }
        let regions = view_layout::regions([size.x as u32, size.y as u32], true);
        let Some((view, local, view_size)) = view_layout::hit(&regions, cursor) else {
            return Ok(None);
        };
        self.activate_view(view)?;
        Ok(Some((local, view_size, cursor - local)))
    }
    fn active_camera_size(&self, size: Vec2) -> Vec2 {
        if self.split_views() && size.x >= 2. {
            let left = (size.x * 0.5).floor();
            Vec2::new(
                if self.active_view == 0 {
                    left
                } else {
                    size.x - left
                },
                size.y,
            )
        } else {
            size
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn cursor_in_viewport(&self, cursor: Vec2) -> bool {
        let Some(window) = &self.window else {
            return false;
        };
        let scale = window.scale_factor() as f32;
        let size = window.inner_size();
        let size = Vec2::new(size.width as f32, size.height as f32) / scale;
        let cursor = cursor / scale;
        cursor.x >= 150_f32.min(size.x * 0.25)
            && cursor.x < size.x - 180_f32.min(size.x * 0.3)
            && cursor.y >= 0.
            && cursor.y < size.y
    }
    #[cfg(test)]
    fn pick_bounds(
        &mut self,
        cursor: Vec2,
        size: Vec2,
        bounds: PickBounds,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if size.x <= 0.0 || size.y <= 0.0 {
            return Ok(false);
        }
        let Some(ray) = (PickViewport {
            origin: Vec2::ZERO,
            size,
        })
        .ray(Mat4::IDENTITY, cursor, 1.0)?
        else {
            return Ok(false);
        };
        for node in &self.instances {
            self.scene.insert_component(*node, bounds)?;
        }
        let hit = self.scene.pick(ray, 1)?.hit;
        if let Some(hit) = hit {
            self.selected = self
                .instances
                .iter()
                .position(|node| *node == hit.node)
                .ok_or("picked foreign instance")?;
            if let Some(window) = &self.window {
                window.set_title(&format!(
                    "Voxy — selected instance {}/{}",
                    self.selected + 1,
                    self.instances.len()
                ));
            }
            return Ok(true);
        }
        Ok(false)
    }
    fn pick_model(&mut self, cursor: Vec2, size: Vec2) -> Result<bool, Box<dyn std::error::Error>> {
        let mut nearest = None;
        for (index, node) in self.instances.iter().enumerate() {
            let Some(model) = self.scene.component::<ModelInstance>(*node)? else {
                continue;
            };
            let Some(asset) = self.catalog.snapshot(&model.asset) else {
                continue;
            };
            let Some(mesh) = asset.value().mesh_for(
                self.scene
                    .component::<ModelPart>(*node)?
                    .map(|part| part.node),
            ) else {
                continue;
            };
            if let Some((_, depth)) = picking::pick_mesh_depth(
                &self.scene,
                &[*node],
                mesh,
                cursor,
                size,
                self.camera.matrix(size)?,
            )? && nearest.is_none_or(|(_, previous)| depth < previous)
            {
                nearest = Some((index, depth));
            }
        }
        if let Some((index, _)) = nearest {
            self.selected = index;
            self.update_edit_title();
            return Ok(true);
        }
        Ok(false)
    }
    fn begin_drag(
        &mut self,
        cursor: Vec2,
        viewport: Vec2,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.finish_drag(false)?;
        if viewport.x <= 0.0 || viewport.y <= 0.0 || !viewport.is_finite() || !cursor.is_finite() {
            return Ok(());
        }
        let Some(node) = self.instances.get(self.selected).copied() else {
            return Ok(());
        };
        self.drag = Some(ModelDrag {
            node,
            origin: cursor,
            viewport,
            viewport_origin: Vec2::ZERO,
            original: self.scene.local(node)?,
            world_origin: self.scene.world_matrix(node)?.w_axis.truncate(),
            axis: DragAxis::Plane,
            edit: self
                .authoring
                .history
                .as_ref()
                .ok_or("missing history")?
                .begin_edit(),
        });
        Ok(())
    }
    fn queue_ui_action(&mut self, event: voxy_gameplay::UiActionEvent) {
        if let Some(actions) = &mut self.play.ui_actions
            && let Err(error) = actions.enqueue(&self.scene, event)
        {
            eprintln!("GAME UI ACTION REJECTED: {error}");
        }
    }
    fn game_ui_display_ready(&mut self, viewport: [f32; 2]) -> bool {
        // Headless callers have no asynchronous display; native input must use
        // the snapshot acknowledged by an actual presented frame.
        if self.window.is_none() {
            return true;
        }
        match self
            .game_ui_input
            .admit_presented(&self.scene, viewport, self.ui_live.presented())
        {
            Ok(ready) => ready,
            Err(error) => {
                eprintln!("GAME UI INPUT CANCELLED: {error}");
                false
            }
        }
    }
    fn activate_game_ui(
        &mut self,
        event: voxy_gameplay::UiActionEvent,
    ) -> Result<(), voxy_input::InputError> {
        if self
            .play
            .ui_actions
            .as_ref()
            .is_some_and(|actions| actions.handles(&event.action))
        {
            self.queue_ui_action(event);
            return Ok(());
        }
        if self.play.player_input.state(&event.action).is_none() {
            // UI-only actions share the same bounded named map as physical input.
            self.play.player_input.bind(&event.action, Vec::new())?;
        }
        self.play.player_input.activate(&event.action)?;
        eprintln!(
            "GAME UI ACTION owner={:?} action={}",
            event.owner, event.action
        );
        Ok(())
    }
    // Native viewport coordinates use the same bounded f32 layout as rendering.
    #[allow(clippy::cast_precision_loss)]
    fn game_ui_key(
        &mut self,
        key: KeyCode,
        state: ElementState,
        repeat: bool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if self.play.playing.is_none() && !self.standalone {
            return Ok(false);
        }
        let viewport = self.window.as_ref().map_or([1280.0, 720.0], |window| {
            let size = window.inner_size();
            [size.width as f32, size.height as f32]
        });
        if !self.game_ui_display_ready(viewport) {
            return Ok(false);
        }
        match self.game_ui_input.key(
            &self.scene,
            viewport,
            key,
            state,
            repeat,
            self.modifiers.shift_key(),
        ) {
            Ok((consumed, event)) => {
                if let Some(event) = event {
                    self.activate_game_ui(event)?;
                }
                Ok(consumed)
            }
            Err(error) => {
                eprintln!("GAME UI INPUT CANCELLED: {error}");
                Ok(false)
            }
        }
    }
    fn game_ui_pointer(
        &mut self,
        cursor: Vec2,
        viewport: Vec2,
        pressed: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_none() && !self.standalone {
            return Ok(());
        }
        if !self.game_ui_display_ready(viewport.to_array()) {
            return Ok(());
        }
        match self.game_ui_input.pointer(
            &self.scene,
            viewport.to_array(),
            cursor.to_array(),
            pressed,
        ) {
            Ok(Some(event)) => {
                self.activate_game_ui(event)?;
            }
            Ok(None) => {}
            Err(error) => eprintln!("GAME UI INPUT CANCELLED: {error}"),
        }
        Ok(())
    }
    // Winit coordinates and finite monitor DPI are intentionally represented as f32.
    #[allow(clippy::cast_possible_truncation)]
    fn pointer_press(
        &mut self,
        cursor: Vec2,
        viewport: Vec2,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.authoring.preview_seek = None;
        if self.play.playing.is_none()
            && self
                .panels
                .as_ref()
                .is_some_and(|panels| !panels.input_ready())
        {
            return Ok(());
        }
        if let Some(window) = &self.window
            && self.panels.is_some()
        {
            let point = cursor / window.scale_factor() as f32;
            let size = viewport / window.scale_factor() as f32;
            if let Some(action) = self.panel_target(point)? {
                if matches!(action, panels::Action::PreviewPhase(_)) {
                    let rect = self
                        .panels
                        .as_ref()
                        .unwrap()
                        .regions
                        .iter()
                        .find(|(_, action)| *action == panels::Action::PreviewSeek)
                        .ok_or("missing seek region")?
                        .0;
                    self.authoring.preview_seek = self
                        .instances
                        .get(self.selected)
                        .copied()
                        .map(|owner| (owner, rect));
                }
                let result = self.panel_action(action);
                if result.is_err() {
                    self.authoring.preview_seek = None;
                }
                return result;
            }
            if point.x < 150.0_f32.min(size.x * 0.25)
                || point.x >= size.x - 180.0_f32.min(size.x * 0.3)
            {
                return Ok(());
            }
        }
        self.field = None;
        self.parenting = None;
        if self.play.playing.is_some() || self.standalone {
            self.game_ui_pointer(cursor, viewport, true)?;
            return Ok(());
        }
        let Some((cursor, viewport, viewport_origin)) = self.view_pointer(cursor, viewport)? else {
            return Ok(());
        };
        let axis = if let Some(node) = self.instances.get(self.selected)
            && self
                .scene
                .component::<ModelInstance>(*node)?
                .is_some_and(|model| self.catalog.snapshot(&model.asset).is_some())
            && self.scene.active_in_hierarchy(*node)?
        {
            gizmo::hit_axis_camera(
                self.camera.matrix(viewport)?,
                self.scene.world_matrix(*node)?.w_axis.truncate(),
                cursor,
                viewport,
            )
        } else {
            None
        };
        if let Some(axis) = axis {
            self.begin_drag(cursor, viewport)?;
            if let Some(drag) = &mut self.drag {
                drag.axis = axis;
            }
        } else if self.pick_model(cursor, viewport)? {
            self.begin_drag(cursor, viewport)?;
        }
        if let Some(drag) = &mut self.drag {
            drag.viewport_origin = viewport_origin;
        }
        Ok(())
    }
    fn preview_drag(&mut self, cursor: Vec2) -> Result<(), Box<dyn std::error::Error>> {
        self.trace.pending |= self.drag.is_some();
        if let Some(drag) = &self.drag {
            let cursor = cursor - drag.viewport_origin;
            let mut local = drag.original;
            let vp = self.camera.matrix(drag.viewport)?;
            let world_origin = drag.world_origin;
            let depth = vp.project_point3(world_origin).z;
            let plane_delta = camera::unproject(vp, cursor, drag.viewport, depth)?
                - camera::unproject(vp, drag.origin, drag.viewport, depth)?;
            let world_delta = match drag.axis {
                DragAxis::Plane => plane_delta,
                DragAxis::X => gizmo::axis_delta(
                    vp,
                    world_origin,
                    Vec3::X,
                    cursor - drag.origin,
                    drag.viewport,
                ),
                DragAxis::Y => gizmo::axis_delta(
                    vp,
                    world_origin,
                    Vec3::Y,
                    cursor - drag.origin,
                    drag.viewport,
                ),
                DragAxis::Z => gizmo::axis_delta(
                    vp,
                    world_origin,
                    Vec3::Z,
                    cursor - drag.origin,
                    drag.viewport,
                ),
            };
            let delta = if let Some(parent) = self.scene.parent(drag.node)? {
                let inverse = self.scene.world_matrix(parent)?.inverse();
                if !inverse.is_finite() {
                    return Ok(());
                }
                inverse.transform_vector3(world_delta)
            } else {
                world_delta
            };
            local.translation += delta;
            self.scene.set_local(drag.node, local)?;
        }
        Ok(())
    }
    fn finish_drag(&mut self, commit: bool) -> Result<(), Box<dyn std::error::Error>> {
        let Some(mut drag) = self.drag.take() else {
            return Ok(());
        };
        if self.trace.enabled {
            println!("EDITOR DRAG FINISH frames={} commit={commit}", self.frames);
        }
        self.trace.pending = true;
        self.trace.last_outcome = None;
        if !commit {
            self.scene.set_local(drag.node, drag.original)?;
            return Ok(());
        }
        *drag.edit.document_mut() = match self.authoring_document() {
            Ok(document) => document,
            Err(error) => {
                self.scene.set_local(drag.node, drag.original)?;
                return Err(error.into());
            }
        };
        if let Err(error) = self
            .authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit_edit(drag.edit, &self.authoring.authoring_project.registry)
        {
            self.scene.set_local(drag.node, drag.original)?;
            return Err(error.into());
        }
        Ok(())
    }
    #[allow(clippy::too_many_lines)]
    fn authoring_document(&self) -> Result<SceneDocument, voxy_scene::SceneGraphError> {
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
    fn restore_authoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    fn save_authoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    fn validate_authoring_document(
        &self,
        document: &SceneDocument,
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
                .is_some_and(|id| !self.available.contains(&AssetId(id.into())))
            {
                return Err("viewport scene requires known model resources".into());
            }
            if let Some(value) = object
                .id
                .0
                .strip_prefix("model-")
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
    fn load_authoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
        let next = self.validate_authoring_document(&document)?;

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
    fn history_key(&mut self, key: KeyCode) -> Result<(), Box<dyn std::error::Error>> {
        let changed = self.authoring.step_history(key == KeyCode::KeyZ)?;
        if changed {
            self.restore_authoring()?;
        }
        Ok(())
    }
    fn verify_scene_round_trip(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    fn configure_scene(
        &mut self,
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.authoring.scene_path = Some(path.into());
        if path.try_exists()? {
            self.load_authoring()?;
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
    // Do not carry captured activation between authoring and runtime sessions.
    fn clear_panel_interactions(&mut self) {
        self.authoring.preview_seek = None;
        if let Some(panels) = &mut self.panels {
            panels.focus.cancel();
        }
        self.field = None;
        self.parenting = None;
    }
    fn panel_document(&self) -> Result<SceneDocument, Box<dyn std::error::Error>> {
        if let Some(draft) = &self.retarget_draft {
            return Ok(draft.document.clone());
        }
        if self.play.playing.is_some() {
            Ok(self
                .authoring
                .history
                .as_ref()
                .ok_or("missing history")?
                .current()
                .clone())
        } else {
            Ok(self.authoring_document()?)
        }
    }
    fn panel_target(
        &mut self,
        point: Vec2,
    ) -> Result<Option<panels::Action>, Box<dyn std::error::Error>> {
        let document = self.panel_document()?;
        let Some(panels) = &mut self.panels else {
            return Ok(None);
        };
        if !panels
            .focus
            .matches_context(&document, self.selected, self.inspector)
        {
            panels.focus.cancel();
            self.panel_cache = None;
            return Ok(None);
        }
        let action = panels.hit(point);
        panels.focus.pointer(action)?;
        if action == Some(panels::Action::PreviewSeek) {
            let rect = panels
                .regions
                .iter()
                .find(|(_, action)| *action == panels::Action::PreviewSeek)
                .ok_or("missing preview seek region")?
                .0;
            return Ok(Some(panels::Action::PreviewPhase(f64::from(
                ((point.x - rect[0]) / rect[2]).clamp(0., 1.),
            ))));
        }
        Ok(action)
    }
    fn panel_key(
        &mut self,
        key: KeyCode,
        state: ElementState,
        repeat: bool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if self.standalone
            || self.play.playing.is_some()
            || self.field.is_some()
            || !matches!(key, KeyCode::Tab | KeyCode::Enter | KeyCode::Space)
            || self.panels.is_none()
        {
            return Ok(false);
        }
        let document = self.panel_document()?;
        let panels = self.panels.as_mut().ok_or("missing panels")?;
        if !panels
            .focus
            .matches_context(&document, self.selected, self.inspector)
        {
            panels.focus.cancel();
            // The next frame rebuilds geometry and targets; stale indices cannot
            // dispatch before that frame after a shortcut changes the hierarchy.
            self.panel_cache = None;
            return Ok(true);
        }
        if !panels.input_ready() {
            panels.focus.cancel();
            return Ok(true);
        }
        let action = match key {
            KeyCode::Tab => {
                if state == ElementState::Pressed && !repeat {
                    panels.focus.traverse(self.modifiers.shift_key());
                }
                None
            }
            KeyCode::Enter | KeyCode::Space => panels.focus.activate(
                if key == KeyCode::Enter { 1 } else { 2 },
                state == ElementState::Pressed,
                repeat,
            ),
            _ => None,
        };
        if let Some(action) = action {
            self.panel_action(action)?;
        }
        Ok(true)
    }
    fn authoring_input_key(
        &mut self,
        key: KeyCode,
        text: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.play.playing.is_none()
            && key != KeyCode::Escape
            && self
                .panels
                .as_ref()
                .is_some_and(|panels| !panels.input_ready())
        {
            if self.trace.enabled {
                println!(
                    "EDITOR KEY REJECTED frames={} key={key:?} reason=panels-not-presented",
                    self.frames
                );
            }
            return Ok(());
        }
        if self.field.is_some() {
            self.field_key(key, text)
        } else {
            self.edit_key(key)
        }
    }
    #[allow(clippy::too_many_lines)]
    fn edit_key(&mut self, key: KeyCode) -> Result<(), Box<dyn std::error::Error>> {
        if self.retarget_draft.is_some() {
            if key == KeyCode::Escape {
                self.cancel_retarget();
                return Ok(());
            }
            return Err("apply or cancel the retarget profile edit first".into());
        }
        if self.standalone {
            return Ok(());
        }
        if self.trace.enabled {
            println!("EDITOR KEY frames={} key={key:?}", self.frames);
        }
        self.trace.pending = true;
        self.trace.last_outcome = None;
        self.finish_drag(false)?;
        if key == KeyCode::KeyH && self.play.playing.is_none() {
            return self.expand_model();
        }
        if key == KeyCode::F10 {
            let enabled = !self.msaa4;
            if let Some(graphics) = &mut self.graphics {
                if enabled {
                    pollster::block_on(graphics.renderer.enable_msaa4(graphics.host.device()))?;
                    graphics.host.enable_msaa4()?;
                } else {
                    graphics.host.disable_msaa4();
                }
            }
            self.msaa4 = enabled;
            if let Some(panels) = &mut self.panels {
                panels.invalidate_presentation();
            }
            self.update_edit_title();
            return Ok(());
        }
        if key == KeyCode::F4 && self.play.playing.is_none() {
            self.camera_drag = None;
            if self.secondary_camera.take().is_none() {
                let mut other = self.camera.clone();
                other.legacy = false;
                other.perspective = !self.camera.perspective;
                self.secondary_camera = Some(other);
            }
            self.active_view = 0;
            if let Some(graphics) = &mut self.graphics {
                graphics.lod_history.clear();
            }
            if let Some(panels) = &mut self.panels {
                panels.invalidate_presentation();
            }
            self.update_edit_title();
            return Ok(());
        }
        if key == KeyCode::F7 {
            self.camera.legacy = false;
            self.camera.perspective = !self.camera.perspective;
            return Ok(());
        }
        if key == KeyCode::F8 {
            self.camera = camera::ViewportCamera::default();
            return Ok(());
        }
        if key == KeyCode::KeyF {
            if let Some(node) = self.instances.get(self.selected) {
                self.camera.target = self.scene.world_matrix(*node)?.w_axis.truncate();
                self.camera.legacy = false;
            }
            return Ok(());
        }
        if key == KeyCode::F6 {
            return self.toggle_play();
        }
        if self.play.playing.is_some() {
            return Ok(());
        }
        match key {
            KeyCode::F5 if self.inspector == InspectorMode::ImportSettings => {
                return self.save_audio_settings();
            }
            KeyCode::F9 if self.inspector == InspectorMode::ImportSettings => {
                return self.open_audio_settings();
            }
            KeyCode::F5 => return self.save_authoring(),
            KeyCode::F9 => return self.load_authoring(),
            _ => {}
        }
        if key == KeyCode::Escape {
            self.parenting = None;
            return Ok(());
        }
        if matches!(key, KeyCode::KeyZ | KeyCode::KeyY) {
            self.parenting = None;
            return self.history_key(key);
        }
        if key != KeyCode::KeyP {
            self.parenting = None;
        }
        let node = self.instances.get(self.selected).copied();
        if node.is_none() && key != KeyCode::KeyD {
            return Ok(());
        }
        let mut local = node
            .map(|node| self.scene.local(node))
            .transpose()?
            .unwrap_or(Transform {
                translation: Vec3::new(0.0, 0.0, 0.5),
                ..Transform::default()
            });
        match key {
            KeyCode::KeyG => {
                let node = node.ok_or("missing selected node")?;
                let owners: Vec<_> = self
                    .scene
                    .components::<EditorCamera>()
                    .map(|(owner, _)| owner)
                    .collect();
                for owner in owners {
                    self.scene.remove_component::<EditorCamera>(owner)?;
                }
                self.scene.insert_component(node, self.camera.clone())?;
            }
            KeyCode::KeyM => {
                self.inspector = if self.inspector == InspectorMode::Material {
                    InspectorMode::Transform
                } else {
                    InspectorMode::Material
                };
                self.field = None;
                if self.inspector == InspectorMode::Material
                    && let Some(node) = node
                    && self.scene.component::<SceneMaterial>(node)?.is_none()
                {
                    self.scene
                        .insert_component(node, SceneMaterial::default())?;
                } else {
                    return Ok(());
                }
            }
            KeyCode::KeyL => {
                let node = node.ok_or("missing selected node")?;
                if self.scene.component::<DirectionalLight>(node)?.is_some() {
                    self.scene.remove_component::<DirectionalLight>(node)?;
                } else {
                    self.scene
                        .insert_component(node, DirectionalLight::default())?;
                }
            }
            KeyCode::KeyI => {
                self.inspector = if self.inspector == InspectorMode::Physics {
                    InspectorMode::Transform
                } else {
                    InspectorMode::Physics
                };
                self.field = None;
                return Ok(());
            }
            KeyCode::KeyC => {
                let node = node.ok_or("missing selected node")?;
                if self.scene.component::<CharacterBody>(node)?.is_some() {
                    self.scene.remove_component::<CharacterBody>(node)?;
                } else {
                    self.scene.remove_component::<BoxCollider>(node)?;
                    self.scene
                        .insert_component(node, CharacterBody::default())?;
                }
            }
            KeyCode::KeyB => {
                let node = node.ok_or("missing selected node")?;
                if self.scene.component::<BoxCollider>(node)?.is_some() {
                    self.scene.remove_component::<BoxCollider>(node)?;
                } else {
                    self.scene.remove_component::<CharacterBody>(node)?;
                    self.scene.insert_component(node, BoxCollider::default())?;
                }
            }
            KeyCode::KeyP => return self.panel_action(panels::Action::Parent),
            KeyCode::KeyA => {
                let node = node.ok_or("missing selected node")?;
                self.scene
                    .set_active(node, !self.scene.active_self(node)?)?;
            }
            KeyCode::KeyR => {
                if let Some(node) = node {
                    self.scene.remove_component::<ModelPart>(node)?;
                }
                let node = node.ok_or("missing selected node")?;
                let current = &self
                    .scene
                    .component::<ModelInstance>(node)?
                    .ok_or("missing model")?
                    .asset;
                let next = self
                    .available
                    .range((
                        std::ops::Bound::Excluded(current),
                        std::ops::Bound::Unbounded,
                    ))
                    .next()
                    .or_else(|| self.available.first())
                    .ok_or("empty resources")?
                    .clone();
                self.scene.insert_component(
                    node,
                    ModelInstance {
                        asset: next.clone(),
                    },
                )?;
                if self.catalog.snapshot(&next).is_none() {
                    self.reload.insert(next);
                }
            }
            KeyCode::KeyD => {
                if self.instances.len() >= scene_limits::OBJECTS {
                    return Ok(());
                }
                if node.is_some() {
                    local.translation.x += 0.15;
                }
                let next = self
                    .authoring
                    .next_object_id
                    .checked_add(1)
                    .ok_or("object identity exhausted")?;
                let parent = node
                    .map(|node| self.scene.parent(node))
                    .transpose()?
                    .flatten();
                let duplicate = self.scene.spawn(parent, local)?;
                self.scene.insert_component(
                    duplicate,
                    ModelInstance {
                        asset: node
                            .and_then(|node| {
                                self.scene.component::<ModelInstance>(node).ok().flatten()
                            })
                            .map_or_else(|| self.id.clone(), |model| model.asset.clone()),
                    },
                )?;
                if let Some(source) = node {
                    self.authoring
                        .authoring_project
                        .registry
                        .copy_registered_components(&mut self.scene, source, duplicate)?;
                    self.scene
                        .set_active(duplicate, self.scene.active_self(source)?)?;
                }
                self.scene.set_name(
                    duplicate,
                    format!("Model {}", self.authoring.next_object_id + 1),
                )?;
                self.instances.push(duplicate);
                self.object_ids
                    .push(ObjectId(format!("model-{}", self.authoring.next_object_id)));
                self.authoring.next_object_id = next;
                self.selected = self.instances.len() - 1;
            }
            KeyCode::Delete | KeyCode::Backspace => {
                self.scene
                    .remove_subtree(node.ok_or("missing selected node")?)?;
                let surviving: Vec<_> = self
                    .instances
                    .iter()
                    .copied()
                    .zip(self.object_ids.iter().cloned())
                    .filter(|(node, _)| self.scene.local(*node).is_ok())
                    .collect();
                (self.instances, self.object_ids) = surviving.into_iter().unzip();
                self.selected = self.selected.min(self.instances.len().saturating_sub(1));
                if let Some(graphics) = &mut self.graphics {
                    graphics.transforms.clear();
                }
            }
            KeyCode::Tab => self.selected = (self.selected + 1) % self.instances.len(),
            KeyCode::ArrowLeft => {
                local.translation.x -= 0.05;
                self.scene
                    .set_local(node.ok_or("missing selected node")?, local)?;
            }
            KeyCode::ArrowRight => {
                local.translation.x += 0.05;
                self.scene
                    .set_local(node.ok_or("missing selected node")?, local)?;
            }
            KeyCode::ArrowUp => {
                local.translation.y += 0.05;
                self.scene
                    .set_local(node.ok_or("missing selected node")?, local)?;
            }
            KeyCode::ArrowDown => {
                local.translation.y -= 0.05;
                self.scene
                    .set_local(node.ok_or("missing selected node")?, local)?;
            }
            _ => return Ok(()),
        }
        if key != KeyCode::Tab {
            self.commit_authoring()?;
        }
        self.update_edit_title();
        Ok(())
    }
    fn commit_authoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    fn toggle_audio_source(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let node = *self
            .instances
            .get(self.selected)
            .ok_or("missing selected node")?;
        if self
            .scene
            .component::<voxy_gameplay::AudioSource>(node)?
            .is_some()
        {
            self.scene
                .remove_component::<voxy_gameplay::AudioSource>(node)?;
        } else {
            let asset = self
                .authoring
                .authoring_project
                .audio_assets()?
                .into_iter()
                .next()
                .ok_or("project has no WAV audio asset")?;
            self.scene.insert_component(
                node,
                voxy_gameplay::AudioSource {
                    import_settings: None,
                    asset: asset.0,
                    bus: 0,
                    gain: 1.,
                    looping: false,
                    spatial: true,
                    near: 1.,
                    far: 5.,
                },
            )?;
        }
        self.commit_authoring()
    }
    fn toggle_audio_bus(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let node = *self
            .instances
            .get(self.selected)
            .ok_or("missing selected node")?;
        if self
            .scene
            .component::<voxy_gameplay::AudioBus>(node)?
            .is_some()
        {
            self.scene
                .remove_component::<voxy_gameplay::AudioBus>(node)?;
        } else {
            let used: BTreeSet<_> = self
                .scene
                .active_components::<voxy_gameplay::AudioBus>()
                .map(|(_, bus)| bus.bus)
                .collect();
            let bus = (0..16)
                .find(|bus| !used.contains(bus))
                .ok_or("all audio buses have active owners")?;
            self.scene
                .insert_component(node, voxy_gameplay::AudioBus { bus, gain: 1. })?;
        }
        self.commit_authoring()
    }
    fn toggle_audio_listener(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let node = *self
            .instances
            .get(self.selected)
            .ok_or("missing selected node")?;
        if self
            .scene
            .component::<voxy_gameplay::AudioListener>(node)?
            .is_some()
        {
            self.scene
                .remove_component::<voxy_gameplay::AudioListener>(node)?;
        } else {
            self.scene
                .insert_component(node, voxy_gameplay::AudioListener::default())?;
        }
        self.commit_authoring()
    }
    fn panel_action(&mut self, action: panels::Action) -> Result<(), Box<dyn std::error::Error>> {
        if self.retarget_draft.is_some()
            && !matches!(
                action,
                panels::Action::Field(_)
                    | panels::Action::ComponentPage(_)
                    | panels::Action::RetargetApply
                    | panels::Action::RetargetCancel
                    | panels::Action::RetargetRemove
                    | panels::Action::RetargetPair(_)
                    | panels::Action::RetargetDeletePair(_)
                    | panels::Action::RetargetBones(_)
                    | panels::Action::RetargetBone(_)
                    | panels::Action::RetargetBonePage(_)
                    | panels::Action::RetargetBoneClose
            )
        {
            return Err("apply or cancel the retarget profile edit first".into());
        }
        self.finish_drag(false)?;
        self.trace.pending = true;
        self.trace.last_outcome = None;
        self.field = None;
        self.component_edit = None;
        if !matches!(action, panels::Action::Select(_) | panels::Action::Parent) {
            self.parenting = None;
        }
        if self.play.playing.is_some() && action != panels::Action::Play {
            return Ok(());
        }
        match action {
            panels::Action::Select(index) => {
                if index < self.instances.len() {
                    if let Some(node) = self.parenting.take() {
                        self.scene.reparent(node, Some(self.instances[index]))?;
                        self.commit_authoring()?;
                    } else {
                        self.selected = index;
                    }
                    self.update_edit_title();
                }
                Ok(())
            }
            panels::Action::Field(index) => {
                if matches!(self.inspector, InspectorMode::Components(_)) {
                    let document = self.panel_document()?;
                    let object = document
                        .objects
                        .get(self.selected)
                        .ok_or("missing selected object")?;
                    let field = component_fields::fields(object)?
                        .into_iter()
                        .nth(index)
                        .ok_or("missing component field")?;
                    self.component_edit =
                        Some(field.bind(object, &self.authoring.authoring_project.registry)?);
                }
                self.field = Some((index, String::new()));
                Ok(())
            }
            panels::Action::ResetField(index) => self.reset_prefab_field(index),
            panels::Action::PreviewSeek => self.preview_animation_phase(0.5, false),
            panels::Action::PreviewPhase(phase) => self.preview_animation_phase(phase, false),
            panels::Action::MarkerAdd(token) => self.edit_animation_marker(None, token),
            panels::Action::MarkerPreview(index, token) => {
                self.preview_animation_marker(index, token)
            }
            panels::Action::MarkerDelete(index, token) => {
                self.edit_animation_marker(Some(index), token)
            }
            panels::Action::Duplicate => self.edit_key(KeyCode::KeyD),
            panels::Action::Delete => self.edit_key(KeyCode::Delete),
            panels::Action::Resource => self.edit_key(KeyCode::KeyR),
            panels::Action::Parent => {
                if let Some(node) = self.instances.get(self.selected).copied() {
                    if self.scene.parent(node)?.is_some() {
                        self.scene.reparent(node, None)?;
                        self.commit_authoring()?;
                    } else {
                        self.parenting = Some(node);
                    }
                }
                Ok(())
            }
            panels::Action::Character => self.edit_key(KeyCode::KeyC),
            panels::Action::Collider => self.edit_key(KeyCode::KeyB),
            panels::Action::AudioSource => self.toggle_audio_source(),
            panels::Action::AudioListener => self.toggle_audio_listener(),
            panels::Action::AudioBus => self.toggle_audio_bus(),
            panels::Action::AudioSettingsLoad => self.open_audio_settings(),
            panels::Action::AudioSettingsSave => self.save_audio_settings(),
            panels::Action::RetargetBones(index) => self.open_retarget_bones(index),
            panels::Action::RetargetBone(index) => self.choose_retarget_bone(index),
            panels::Action::RetargetBonePage(forward) => self.retarget_bone_page(forward),
            panels::Action::RetargetBoneClose => {
                self.retarget_picker = None;
                self.panel_cache = None;
                Ok(())
            }
            panels::Action::Retarget => self.begin_retarget(),
            panels::Action::RetargetApply => self.apply_retarget(),
            panels::Action::RetargetCancel => {
                self.cancel_retarget();
                Ok(())
            }
            panels::Action::RetargetPair(add) => self.retarget_pair(add),
            panels::Action::RetargetDeletePair(index) => self.delete_retarget_pair(index),
            panels::Action::RetargetRemove => {
                self.remove_retarget_draft()?;
                Ok(())
            }
            panels::Action::Animation => {
                if self.play.playing.is_some() {
                    return Err("stop play before editing animation".into());
                }
                let node = self
                    .instances
                    .get(self.selected)
                    .copied()
                    .ok_or("missing selected model")?;
                if self.scene.component::<ModelAnimation>(node)?.is_some() {
                    self.scene.remove_component::<ModelAnimation>(node)?;
                } else {
                    self.scene
                        .insert_component(node, ModelAnimation::default())?;
                }
                self.commit_authoring()?;
                self.inspector = InspectorMode::Components(0);
                self.field = None;
                Ok(())
            }
            panels::Action::Behavior => {
                self.inspector = match self.inspector {
                    InspectorMode::Behavior | InspectorMode::ImportSettings => InspectorMode::Audio,
                    InspectorMode::Audio => InspectorMode::Mixer,
                    InspectorMode::Mixer => InspectorMode::Components(0),
                    InspectorMode::Components(_) => InspectorMode::Collections(0, 0),
                    InspectorMode::Collections(_, _) => InspectorMode::Transform,
                    _ => InspectorMode::Behavior,
                };
                self.field = None;
                Ok(())
            }
            panels::Action::ComponentPage(forward) => {
                if let InspectorMode::Components(page) = self.inspector {
                    let document = self.panel_document()?;
                    let object = document
                        .objects
                        .get(self.selected)
                        .ok_or("missing selected object")?;
                    let count = component_fields::fields(object)?.len();
                    let pages = count.div_ceil(6).max(1);
                    self.inspector = InspectorMode::Components(if forward {
                        (page + 1) % pages
                    } else {
                        (page + pages - 1) % pages
                    });
                }
                Ok(())
            }
            panels::Action::CollectionChoice => self.collection_page(true, false),
            panels::Action::CollectionPage(forward) => self.collection_page(false, forward),
            panels::Action::CollectionAdd(key) => self.collection_action(key, None, None),
            panels::Action::CollectionDelete(key, item) => {
                self.collection_action(key, Some(item), None)
            }
            panels::Action::CollectionMove(key, item, forward) => {
                self.collection_action(key, Some(item), Some(forward))
            }
            panels::Action::CollectionReset(key, item) => {
                self.reset_prefab_collection_item(key, item)
            }
            panels::Action::CollectionResetOrder(key) => self.reset_prefab_collection_order(key),
            panels::Action::CollectionRestoreDeleted(key) => {
                self.restore_prefab_collection_deleted(key)
            }
            panels::Action::CollectionRestoreItem(key, item) => {
                self.restore_prefab_collection_item(key, item)
            }
            panels::Action::CollectionDeletedPage => {
                let panels = self.panels.as_mut().ok_or("missing panels")?;
                panels.deleted_page = panels.deleted_page.wrapping_add(1);
                panels.invalidate_presentation();
                self.panel_cache = None;
                Ok(())
            }
            panels::Action::Motion => {
                let node = *self
                    .instances
                    .get(self.selected)
                    .ok_or("missing selected node")?;
                if self
                    .scene
                    .component::<voxy_gameplay::AngularMotion>(node)?
                    .is_some()
                {
                    self.scene
                        .remove_component::<voxy_gameplay::AngularMotion>(node)?;
                } else {
                    self.scene.insert_component(
                        node,
                        voxy_gameplay::AngularMotion {
                            axis: [0., 1., 0.],
                            radians_per_second: 1.,
                        },
                    )?;
                }
                self.commit_authoring()
            }
            panels::Action::Physics => self.edit_key(KeyCode::KeyI),
            panels::Action::Active => self.edit_key(KeyCode::KeyA),
            panels::Action::Play => self.edit_key(KeyCode::F6),
            panels::Action::Save => self.edit_key(KeyCode::F5),
            panels::Action::Load => self.edit_key(KeyCode::F9),
            panels::Action::RevertPrefab => self.revert_prefab(),
            panels::Action::PlacePrefab => self.place_prefab(),
            panels::Action::CreatePrefab => self.create_prefab_from_selection(),
            panels::Action::PrefabChoice => {
                self.authoring.prefab_assets = self.authoring.authoring_project.prefabs()?;
                if !self.authoring.prefab_assets.is_empty() {
                    self.authoring.prefab_choice =
                        (self.authoring.prefab_choice + 1) % self.authoring.prefab_assets.len();
                }
                Ok(())
            }
        }
    }
    #[allow(clippy::too_many_lines)]
    fn field_key(
        &mut self,
        key: KeyCode,
        text: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if key == KeyCode::Escape {
            self.field = None;
            return Ok(());
        }
        if matches!(key, KeyCode::Enter | KeyCode::NumpadEnter)
            && matches!(self.inspector, InspectorMode::Components(_))
        {
            let (_, text) = self.field.clone().ok_or("missing component field")?;
            let binding = self
                .component_edit
                .clone()
                .ok_or("missing component field binding")?;
            return self.commit_component_binding(&binding, &text);
        }
        let Some((index, value)) = &mut self.field else {
            return Ok(());
        };
        if key == KeyCode::Backspace {
            value.pop();
            return Ok(());
        }
        if matches!(key, KeyCode::Enter | KeyCode::NumpadEnter) {
            self.trace.pending = true;
            self.trace.last_outcome = None;
            if self.inspector == InspectorMode::Audio && matches!(*index, 6 | 7) {
                let node = *self
                    .instances
                    .get(self.selected)
                    .ok_or("missing selected node")?;
                let mut source = self
                    .scene
                    .component::<voxy_gameplay::AudioSource>(node)?
                    .cloned()
                    .ok_or("selected object has no audio source")?;
                if *index == 6 {
                    source.asset.clone_from(value);
                } else {
                    source.import_settings = if value.is_empty() {
                        None
                    } else {
                        Some(value.clone())
                    };
                }
                if !source.valid() {
                    return Err("invalid audio asset ID".into());
                }
                self.scene.insert_component(node, source)?;
                self.commit_authoring()?;
                self.field = None;
                return Ok(());
            }
            if self.inspector == InspectorMode::ImportSettings {
                let index = *index;
                let text = value.clone();
                return self.edit_audio_settings(index, &text);
            }
            let value: f32 = value.parse()?;
            if !value.is_finite() {
                return Err("property must be finite".into());
            }
            let node = *self
                .instances
                .get(self.selected)
                .ok_or("missing selected node")?;
            if self.inspector == InspectorMode::Mixer {
                let mut bus = self
                    .scene
                    .component::<voxy_gameplay::AudioBus>(node)?
                    .copied()
                    .ok_or("selected object has no audio bus")?;
                match *index {
                    0 if value.fract().abs() < f32::EPSILON && (0.0..=15.0).contains(&value) => {
                        bus.bus = value.to_string().parse()?;
                    }
                    1 => bus.gain = value,
                    _ => return Err("audio mixer requires bus 0..15".into()),
                }
                if !bus.valid() {
                    return Err("audio bus gain must be 0..1".into());
                }
                self.scene.insert_component(node, bus)?;
                self.commit_authoring()?;
                self.field = None;
                return Ok(());
            }
            if self.inspector == InspectorMode::Audio {
                let mut source = self
                    .scene
                    .component::<voxy_gameplay::AudioSource>(node)?
                    .cloned()
                    .ok_or("selected object has no audio source")?;
                match *index {
                    0 => source.gain = value,
                    1 if value.fract().abs() < f32::EPSILON && (0.0..=15.0).contains(&value) => {
                        source.bus = value.to_string().parse()?;
                    }
                    2 | 3 if value == 0. || value.to_bits() == 1_f32.to_bits() => {
                        let enabled = value.to_bits() == 1_f32.to_bits();
                        if *index == 2 {
                            source.looping = enabled;
                        } else {
                            source.spatial = enabled;
                        }
                    }
                    4 => source.near = value,
                    5 => source.far = value,
                    _ => return Err("audio requires bus 0..15 and toggle 0/1".into()),
                }
                if !source.valid() {
                    return Err("invalid audio gain or attenuation range".into());
                }
                self.scene.insert_component(node, source)?;
                self.commit_authoring()?;
                self.field = None;
                return Ok(());
            }
            if self.inspector == InspectorMode::Material {
                let index = *index;
                let mut material = self
                    .scene
                    .component::<SceneMaterial>(node)?
                    .copied()
                    .unwrap_or_default();
                match index {
                    0..=3 => material.tint[index] = value,
                    4 if value == 0. || value.to_bits() == 1_f32.to_bits() => {
                        material.lit = value.to_bits() == 1_f32.to_bits();
                    }
                    _ => return Err("material field requires RGBA [0,1], Lit 0/1".into()),
                }
                if !material.valid() {
                    return Err("opaque material requires RGB in [0,1] and alpha 1".into());
                }
                self.scene.insert_component(node, material)?;
                self.commit_authoring()?;
                self.field = None;
                return Ok(());
            }
            if self.inspector == InspectorMode::Behavior {
                let index = *index;
                let mut motion = self
                    .scene
                    .component::<voxy_gameplay::AngularMotion>(node)?
                    .copied()
                    .ok_or("selected object has no angular motion")?;
                match index {
                    0..=2 => motion.axis[index] = value,
                    3 => motion.radians_per_second = f64::from(value),
                    _ => return Err("unknown angular motion property".into()),
                }
                if !motion.valid() {
                    return Err(
                        "motion requires a nonzero finite axis and bounded finite rate".into(),
                    );
                }
                self.scene.insert_component(node, motion)?;
                self.commit_authoring()?;
                self.field = None;
                return Ok(());
            }
            if self.inspector == InspectorMode::Physics {
                let index = *index;
                if let Some(mut body) = self.scene.component::<CharacterBody>(node)?.copied() {
                    match index {
                        0..=2 => body.half_extents[index] = value,
                        3 => body.speed = f64::from(value),
                        4 => body.gravity = f64::from(value),
                        5 => body.jump_speed = f64::from(value),
                        _ => return Err("unknown character property".into()),
                    }
                    self.scene.insert_component(node, body)?;
                } else if let Some(mut collider) =
                    self.scene.component::<BoxCollider>(node)?.copied()
                {
                    if index > 2 {
                        return Err("unknown collider property".into());
                    }
                    collider.half_extents[index] = value;
                    self.scene.insert_component(node, collider)?;
                } else {
                    return Err("selected object has no physics component".into());
                }
                self.commit_authoring()?;
                self.field = None;
                return Ok(());
            }
            let original = self.scene.local(node)?;
            let mut local = original;
            match *index {
                0..=2 => local.translation[*index] = value,
                3..=5 => {
                    let (x, y, z) = local.rotation.to_euler(glam::EulerRot::XYZ);
                    let mut rotation = [x, y, z];
                    rotation[*index - 3] = value.to_radians();
                    local.rotation = glam::Quat::from_euler(
                        glam::EulerRot::XYZ,
                        rotation[0],
                        rotation[1],
                        rotation[2],
                    );
                }
                6..=8 => local.scale[*index - 6] = value,
                _ => return Err("unknown transform field".into()),
            }
            self.scene.set_local(node, local)?;
            self.commit_authoring()?;
            self.field = None;
            return Ok(());
        }
        if matches!(self.inspector, InspectorMode::Components(_))
            || self.inspector == InspectorMode::Audio && matches!(*index, 6 | 7)
        {
            if let Some(text) = text
                && value.len() + text.len() <= 1024
                && !text.chars().any(char::is_control)
            {
                value.push_str(text);
            }
            return Ok(());
        }
        if let Some(text) = text
            && value.len() + text.len() <= 32
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
        {
            value.push_str(text);
        }
        Ok(())
    }
    fn expand_model(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let owner = *self
            .instances
            .get(self.selected)
            .ok_or("no selected model")?;
        if self.scene.component::<ModelPart>(owner)?.is_some() {
            return Err("model hierarchy already expanded".into());
        }
        let id = self
            .scene
            .component::<ModelInstance>(owner)?
            .ok_or("missing model")?
            .asset
            .clone();
        let asset = self.catalog.snapshot(&id).ok_or("model still loading")?;
        if asset.value().nodes.is_empty() {
            return Err("selected asset has no glTF hierarchy".into());
        }
        let mut document = self.authoring_document()?;
        if document.objects.len() + asset.value().nodes.len() > scene_limits::OBJECTS {
            return Err("scene capacity exceeded".into());
        }
        let old_id = document.objects[self.selected].id.clone();
        // Keep the selected object as a container; its part index is one past the nodes.
        document.objects[self.selected].components.insert(
            "editor.model-part.v1".into(),
            serde_json::to_value(ModelPart { node: u32::MAX })?,
        );
        let next = self
            .authoring
            .next_object_id
            .checked_add(u64::try_from(asset.value().nodes.len())?)
            .ok_or("object identity exhausted")?;
        let ids: Vec<_> = (self.authoring.next_object_id..next)
            .map(|index| ObjectId(format!("model-{index}")))
            .collect();
        for (index, node) in asset.value().nodes.iter().enumerate() {
            document.objects.push(SceneObject {
                id: ids[index].clone(),
                parent: Some(
                    node.parent
                        .map_or_else(|| old_id.clone(), |parent| ids[parent].clone()),
                ),
                name: node.name.clone(),
                active: true,
                translation: node.local.translation.to_array(),
                rotation: node.local.rotation.to_array(),
                scale: node.local.scale.to_array(),
                components: BTreeMap::from([
                    (
                        "editor.model.v1".into(),
                        serde_json::Value::String(id.0.clone()),
                    ),
                    (
                        "editor.model-part.v1".into(),
                        serde_json::to_value(ModelPart {
                            node: u32::try_from(index)?,
                        })?,
                    ),
                ]),
            });
        }
        self.authoring
            .history
            .as_mut()
            .ok_or("missing history")?
            .commit(document, &self.authoring.authoring_project.registry)?;
        self.authoring.next_object_id = next;
        self.restore_authoring()?;
        Ok(())
    }
    fn close_play_audio(&mut self) {
        self.audio.close_play_audio();
    }
    fn close_audio_device(&mut self) {
        self.audio.close_audio_device();
    }
    fn poll_audio_device(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self
            .audio
            .poll(&self.scene, &self.authoring.authoring_project)?
        {
            audio_session::AudioPoll::Ready | audio_session::AudioPoll::RetryPlay => {
                self.toggle_play()?
            }
            audio_session::AudioPoll::Pending => {}
        }
        Ok(())
    }
    fn toggle_play(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.retarget_draft.is_some() {
            return Err("apply or cancel the retarget profile edit before Play".into());
        }
        self.finish_drag(false)?;
        self.clear_panel_interactions();
        if self.audio.is_pending() {
            self.close_audio_device();
            self.update_edit_title();
            return Ok(());
        }
        self.game_ui_input.cancel();
        if let Some(expected) = self.play.playing.take() {
            self.close_play_audio();
            self.close_audio_device();
            self.play.stop(&mut self.scene)?;
            if let Some(graphics) = &mut self.graphics {
                graphics.animated_models.clear();
            }
            self.restore_authoring()?;
            if self.authoring_document()? != expected {
                return Err("Stop failed to restore authoring scene".into());
            }
        } else {
            CharacterPhysics::new(&self.scene, 128, 128)
                .with_depenetration(true)
                .validate_start(&self.scene)?;
            if let Some(runtime) = self
                .authoring
                .authoring_project
                .liquid_runtime(&self.scene)?
            {
                voxy_gameplay::validate_game_descriptors_with_liquid_runtime(
                    &self.scene,
                    128,
                    &runtime.0,
                )?;
            } else {
                voxy_gameplay::validate_game_descriptors(&self.scene, 128)?;
            }
            let audio = match self.audio.prepare_start(
                &self.scene,
                &self.authoring.authoring_project,
                self.window.is_some(),
            )? {
                audio_session::AudioStart::Pending => {
                    self.update_edit_title();
                    return Ok(());
                }
                audio_session::AudioStart::Ready(audio) => audio,
            };
            self.play.player_input = voxy_gameplay::player_input()?;
            let authoring_before_play = self.authoring_document()?;
            self.restore_authoring()?; // Detached runtime graph, fresh generational handles.
            let liquid = self
                .authoring
                .authoring_project
                .liquid_runtime(&self.scene)?;
            let mut actions = voxy_gameplay::UiActionHandlers::new(&self.scene, 128, 128, 128);
            actions.register("voxy.ui.hide".into(), |_, event, commands| {
                commands
                    .push(voxy_scene::SceneCommand::SetActive(event.owner, false))
                    .map_err(|error| error.to_string())
            })?;
            if let Some(setup) = &mut self.ui_action_setup {
                setup.configure(&mut actions)?;
            }
            let (liquid, optics) = liquid.map_or((None, Vec::new()), |(runtime, optics)| {
                (Some(runtime), optics)
            });
            self.play.liquid_optics = optics;
            self.play.liquid = liquid;
            self.play.playing = Some(authoring_before_play);
            self.authoring.animation_preview = None;
            self.play.ui_actions = Some(actions);
            let simulation = SceneSimulation::new(
                &self.scene,
                SimulationLimits {
                    fixed_step: 1.0 / 60.0,
                    max_steps: 8,
                    max_behaviors: 128,
                    max_commands: 128,
                },
            )?;
            let has_characters = self.scene.components::<CharacterBody>().next().is_some()
                || self.scene.components::<BoxCollider>().next().is_some();
            if has_characters {
                self.play.physics =
                    Some(CharacterPhysics::new(&self.scene, 128, 128).with_depenetration(true));
            }
            self.play.angular_motion =
                Some(voxy_gameplay::AngularMotionBatch::new(&self.scene, 128)?);
            self.audio.set_play(audio);
            self.play.simulation = Some(simulation);
            self.play.simulation_ticks = 0;
            self.play.animations.clear();
            self.play.simulation_time = Instant::now();
        }
        self.update_edit_title();
        Ok(())
    }
    fn game_key(
        &mut self,
        key: KeyCode,
        state: ElementState,
    ) -> Result<(), voxy_input::InputError> {
        if self.play.playing.is_some()
            && let Some(control) = game_control(key)
        {
            self.play.player_input.event(
                control,
                if state == ElementState::Pressed {
                    1.0
                } else {
                    0.0
                },
            )?;
        }
        Ok(())
    }
    fn smoke_acceptance(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if self.smoke_deadline.is_none() {
            return Ok(false);
        }
        if let Some(play) = &self.smoke_play {
            if let Some(frame) = play.stop_frame {
                if self.frames > frame {
                    println!(
                        "MODEL PLAY PASS: fixed_ticks={} native_frames={} stop_presented=true",
                        self.play.simulation_ticks,
                        self.frames - play.first_frame
                    );
                    return Ok(true);
                }
            } else if play.started.elapsed() >= Duration::from_millis(250)
                && self.frames > play.first_frame + 3
                && self.play.simulation_ticks >= 12
            {
                self.toggle_play()?;
                self.smoke_play
                    .as_mut()
                    .ok_or("missing smoke Play")?
                    .stop_frame = Some(self.frames);
            }
        } else if self.recovered
            && self
                .last_publication
                .is_some_and(|time| time.elapsed() >= Duration::from_secs(1))
            && self.failed_at.is_some_and(|frame| self.frames > frame + 20)
        {
            self.toggle_play()?;
            self.smoke_play = Some(SmokePlay {
                started: Instant::now(),
                first_frame: self.frames,
                stop_frame: None,
            });
        }
        Ok(false)
    }
    fn update_edit_title(&self) {
        if let Some(window) = &self.window {
            if self.audio.is_pending() {
                window.set_title("Voxy — preparing audio; F6 cancels play");
                return;
            }
            if self.standalone {
                window.set_title("Voxy Game");
                return;
            }
            if self.play.playing.is_some() && self.play.physics.is_some() {
                window.set_title("Voxy Play — Left/Right move, W/S depth, Space jump; F6 Stop");
                return;
            }
            window.set_title(&format!(
                "Voxy model viewport — instance {}/{} | view {}/{} | F4 split, F10 smoothing, D duplicate, R model, P parent, A active, F6 play/stop, Z undo, Y redo",
                if self.instances.is_empty() { 0 } else { self.selected + 1 },
                self.instances.len(),
                if self.split_views() { self.active_view + 1 } else { 1 },
                if self.split_views() { 2 } else { 1 },
            ));
        }
    }
    fn stop_workers(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.close_play_audio();
        self.retired_ui_workers.extend(self.ui_live.close());
        self.close_audio_device();
        let mut errors = Vec::new();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.play.stop_simulation(&mut self.scene)
        })) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => errors.push(format!("simulation stop: {error}")),
            Err(payload) => errors.push(format!(
                "simulation stop panicked: {}",
                panic_message(payload.as_ref())
            )),
        }
        // Close both producers before waiting for either thread to finish.
        let imports = self
            .imports
            .take()
            .map(voxy_assets::AssetImportWorker::close);
        let watcher = self
            .watcher
            .take()
            .map(voxy_assets::SourcePollWorker::close);
        if let Some(worker) = imports
            && let Err(payload) = worker.join()
        {
            errors.push(format!(
                "import worker panicked: {}",
                panic_message(payload.as_ref())
            ));
        }
        if let Some(worker) = watcher
            && let Err(payload) = worker.join()
        {
            errors.push(format!(
                "poll worker panicked: {}",
                panic_message(payload.as_ref())
            ));
        }
        errors.extend(self.audio.join_workers());
        for worker in self.retired_ui_workers.drain(..) {
            if let Err(payload) = worker.join() {
                errors.push(format!(
                    "audio worker panicked: {}",
                    panic_message(payload.as_ref())
                ));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!("shutdown failed: {}", errors.join("; ")).into())
        }
    }
}
fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic")
}
impl Drop for App {
    fn drop(&mut self) {
        if let Err(error) = self.stop_workers() {
            eprintln!("VOXY SHUTDOWN FAILED: {error}");
        }
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        if let Err(error) = self.initialize(events) {
            self.error = Some(error.to_string());
            events.exit();
        }
    }
    #[allow(
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss
    )]
    fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if self.trace.enabled
            && matches!(
                event,
                WindowEvent::Focused(_) | WindowEvent::Occluded(_) | WindowEvent::Resized(_)
            )
        {
            println!(
                "EDITOR WINDOW event={event:?} visible={:?} focused={:?}",
                self.window.as_ref().and_then(|window| window.is_visible()),
                self.window.as_ref().map(|window| window.has_focus())
            );
        }
        let result = match event {
            WindowEvent::CloseRequested => {
                events.exit();
                Ok(())
            }
            WindowEvent::Resized(size) => self.finish_drag(false).and_then(|()| {
                self.authoring.preview_seek = None;
                self.camera_drag = None;
                if let Some(panels) = &mut self.panels {
                    panels.invalidate_presentation();
                }
                self.graphics.as_mut().map_or(Ok(()), |graphics| {
                    graphics
                        .host
                        .resize(size.width, size.height)
                        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })
                })
            }),
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::CursorMoved { position, .. } => {
                let position = position.to_logical::<f32>(1.0);
                let cursor = Vec2::new(position.x, position.y);
                self.cursor = Some(cursor);
                if self.authoring.preview_seek.is_some() {
                    let scale = self
                        .window
                        .as_ref()
                        .map_or(1., |window| window.scale_factor() as f32);
                    self.continue_preview_seek(cursor, scale)
                } else if let Some((button, previous)) = self.camera_drag {
                    let delta = cursor - previous;
                    self.camera_drag = Some((button, cursor));
                    if button == MouseButton::Right {
                        self.camera.orbit(delta);
                        Ok(())
                    } else {
                        let size = self.window.as_ref().map_or(Vec2::ONE, |w| {
                            let s = w.inner_size();
                            Vec2::new(s.width as f32, s.height as f32)
                        });
                        self.camera
                            .pan(delta, self.active_camera_size(size))
                            .map_err(Into::into)
                    }
                } else {
                    self.preview_drag(cursor)
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y = camera::wheel_steps(
                    delta,
                    self.window
                        .as_ref()
                        .map_or(1., |window| window.scale_factor()),
                );
                if let (Some(cursor), Some(window)) = (self.cursor, &self.window)
                    && cursor.x / (window.scale_factor() as f32) < 150.0
                {
                    self.tree_scroll = if y < 0.0 {
                        self.tree_scroll
                            .saturating_add(1)
                            .min(self.instances.len().saturating_sub(1))
                    } else {
                        self.tree_scroll.saturating_sub(1)
                    };
                    Ok(())
                } else if self
                    .cursor
                    .is_some_and(|cursor| !self.standalone && self.cursor_in_viewport(cursor))
                    && self
                        .panels
                        .as_ref()
                        .is_none_or(|panels| panels.input_ready())
                {
                    let cursor = self.cursor.unwrap();
                    let size = self.window.as_ref().map(|w| w.inner_size()).unwrap();
                    self.view_pointer(cursor, Vec2::new(size.width as f32, size.height as f32))
                        .map(|hit| {
                            if hit.is_some() {
                                self.camera.zoom(y);
                            }
                        })
                } else {
                    Ok(())
                }
            }
            WindowEvent::MouseInput {
                state,
                button: button @ (MouseButton::Right | MouseButton::Middle),
                ..
            } => self.finish_drag(false).and_then(|()| {
                self.camera_drag = None;
                if state == ElementState::Pressed
                    && self
                        .panels
                        .as_ref()
                        .is_none_or(|panels| panels.input_ready())
                    && let Some(cursor) = self
                        .cursor
                        .filter(|cursor| !self.standalone && self.cursor_in_viewport(*cursor))
                    && let Some(size) = self.window.as_ref().map(|w| w.inner_size())
                    && self
                        .view_pointer(cursor, Vec2::new(size.width as f32, size.height as f32))?
                        .is_some()
                {
                    self.camera_drag = Some((button, cursor));
                }
                Ok(())
            }),
            WindowEvent::CursorLeft { .. } => {
                self.authoring.preview_seek = None;
                self.camera_drag = None;
                self.cursor = None;
                self.game_ui_input.cancel();
                self.finish_drag(false)
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let (Some(cursor), Some(window)) = (self.cursor, &self.window) {
                    let size = window.inner_size().to_logical::<f32>(1.0);
                    let viewport = Vec2::new(size.width, size.height);
                    self.pointer_press(cursor, viewport).or_else(|error| {
                        if let Some(window) = &self.window {
                            window.set_title(&format!("Voxy — operation failed: {error}"));
                        }
                        eprintln!("EDITOR OPERATION FAILED: {error}");
                        Ok(())
                    })
                } else {
                    Ok(())
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.authoring.preview_seek = None;
                if let (Some(cursor), Some(window)) = (self.cursor, &self.window) {
                    let size = window.inner_size();
                    self.game_ui_pointer(
                        cursor,
                        Vec2::new(size.width as f32, size.height as f32),
                        false,
                    )
                    .and_then(|()| self.finish_drag(true))
                    .or_else(|error| {
                        eprintln!("GAME UI ACTION FAILED: {error}");
                        Ok(())
                    })
                } else {
                    self.game_ui_input.cancel();
                    self.finish_drag(true)
                }
            }
            WindowEvent::Occluded(true) => {
                self.authoring.preview_seek = None;
                if let Some(panels) = &mut self.panels {
                    panels.invalidate_presentation();
                }
                self.game_ui_input.cancel();
                self.camera_drag = None;
                self.finish_drag(false)
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                Ok(())
            }
            WindowEvent::Focused(focused) => {
                self.play.player_input.set_focused(focused);
                if !focused {
                    self.authoring.preview_seek = None;
                    self.game_ui_input.cancel();
                    self.modifiers = winit::keyboard::ModifiersState::empty();
                    if let Some(panels) = &mut self.panels {
                        panels.invalidate_presentation();
                    }
                    self.camera_drag = None;
                }
                if focused {
                    Ok(())
                } else {
                    self.finish_drag(false)
                }
            }
            WindowEvent::KeyboardInput {
                device_id, event, ..
            } => {
                self.keyboard_device = Some(device_id);
                if let PhysicalKey::Code(key) = event.physical_key {
                    if self
                        .game_ui_key(key, event.state, event.repeat)
                        .unwrap_or_else(|error| {
                            eprintln!("GAME UI ACTION FAILED: {error}");
                            true
                        })
                    {
                        // Release any earlier gameplay binding even when UI acquired focus
                        // while that key was held; otherwise jump could stay latched.
                        if event.state == ElementState::Released {
                            self.game_key(key, event.state)
                                .expect("digital input event");
                        }
                        return;
                    }
                    match self.panel_key(key, event.state, event.repeat) {
                        Ok(true) => return,
                        Err(error) => {
                            eprintln!("PANEL OPERATION FAILED: {error}");
                            if let Some(window) = &self.window {
                                window
                                    .set_title(&format!("Voxy — panel operation failed: {error}"));
                            }
                            return;
                        }
                        Ok(false) => {}
                    }
                    self.game_key(key, event.state)
                        .expect("digital input event");
                    if event.state == ElementState::Released {
                        return;
                    }
                    let result = self.authoring_input_key(key, event.text.as_deref());
                    match result {
                        Err(error)
                            if self.field.is_some()
                                || matches!(
                                    key,
                                    KeyCode::F5
                                        | KeyCode::F10
                                        | KeyCode::F9
                                        | KeyCode::KeyP
                                        | KeyCode::F6
                                        | KeyCode::KeyC
                                        | KeyCode::KeyB
                                ) =>
                        {
                            if let Some(window) = &self.window {
                                window
                                    .set_title(&format!("Voxy — scene operation failed: {error}"));
                            }
                            eprintln!("SCENE OPERATION FAILED: {error}");
                            Ok(())
                        }
                        result => result,
                    }
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.error = Some(error.to_string());
            events.exit();
        }
        if self.error.is_none() {
            match if self.standalone && self.smoke_deadline.is_some() {
                if self.frames >= 3
                    && self.play.simulation_ticks > 0
                    && self.graphics.as_ref().is_some_and(|graphics| {
                        self.required_gpu_assets()
                            .iter()
                            .all(|id| graphics.models.contains_key(id))
                            && self.ui_live.ready()
                            && graphics.ui_presented
                    })
                {
                    println!(
                        "GAME NATIVE PASS frames={} ticks={}",
                        self.frames, self.play.simulation_ticks
                    );
                    Ok(true)
                } else {
                    Ok(false)
                }
            } else if self.animation_smoke.is_some() {
                self.animation_acceptance()
            } else if self.prefab_smoke.is_some() {
                self.prefab_acceptance()
            } else if self.lod_smoke.is_some() {
                self.lod_acceptance()
            } else if self.scene3d_smoke.is_some() {
                self.scene3d_acceptance()
            } else if self.gameplay_smoke.is_some() {
                self.gameplay_acceptance()
            } else {
                self.smoke_acceptance()
            } {
                Ok(true) => events.exit(),
                Ok(false) => {}
                Err(error) => {
                    self.error = Some(error.to_string());
                    events.exit();
                }
            }
        }
        if self
            .smoke_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.error = Some(format!(
                "native acceptance timed out: presented={}, last_surface={:?}",
                self.frames, self.trace.last_outcome
            ));
            events.exit();
        }
    }
    fn device_event(
        &mut self,
        _: &ActiveEventLoop,
        device: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if matches!(event, winit::event::DeviceEvent::Removed)
            && self.keyboard_device == Some(device)
        {
            self.play.player_input.disconnect(0);
            self.keyboard_device = None;
        }
    }
    fn about_to_wait(&mut self, events: &ActiveEventLoop) {
        events.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
/// Selects the stable resource opened by the native model viewport.
#[derive(Clone, Debug)]
pub enum ModelSource {
    File(std::path::PathBuf),
    Manifest {
        path: std::path::PathBuf,
        asset: AssetId,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewportMode {
    Interactive,
    Game,
    GameCheck,
    GameSmoke,
    Smoke,
    GameplaySmoke,
    Scene3DSmoke,
    LodSmoke,
    AnimationSmoke,
    PrefabSmoke,
}
/// Exports the validated scene/import dependency closure as an immutable package.
/// # Errors
/// Rejects changed inputs, invalid resources, quotas and existing output files.
pub fn export_game_package(
    source: &ModelSource,
    scene: &std::path::Path,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = configured_app(source, Some(scene), false)?;
    let result = (|| {
        app.start_standalone()?;
        let authored = app
            .authoring
            .authoring_source
            .as_ref()
            .ok_or("packaging requires a project-scoped scene")?;
        let mut observations = authored.inputs().observations().clone();
        for asset in app.required_cpu_model_assets() {
            let imported = app
                .catalog
                .snapshot(&asset)
                .ok_or("missing packaged model")?;
            for (id, input) in imported.inputs().observations() {
                if observations
                    .get(id)
                    .is_some_and(|previous| previous != input)
                {
                    return Err("inconsistent dependency revisions during package export".into());
                }
                observations.insert(id.clone(), input.clone());
            }
        }
        if let Some(audio) = app.audio.play() {
            for (id, input) in audio.input_observations(&app.scene)? {
                if observations
                    .get(&id)
                    .is_some_and(|previous| previous != &input)
                {
                    return Err(
                        "inconsistent audio dependency revisions during package export".into(),
                    );
                }
                observations.insert(id, input);
            }
        }
        let text = ui_text::UiTextPreparation::prepare_import(
            &app.scene,
            &app.authoring.authoring_project,
            [1280.0, 720.0],
        )?;
        for (id, input) in &text.value().observations {
            if observations
                .get(id)
                .is_some_and(|previous| previous != input)
            {
                return Err(
                    "inconsistent UI font dependency revisions during package export".into(),
                );
            }
            observations.insert(id.clone(), input.clone());
        }
        let paths: Vec<_> = observations
            .keys()
            .map(|id| voxy_assets::SourcePath::new(id.0.clone()))
            .collect::<Result<_, _>>()?;
        let launch = match source {
            ModelSource::File(path) => {
                serde_json::json!({"version":1,"scene":app.authoring.authoring_project.source_path(scene)?.as_str(),"model":app.authoring.authoring_project.source_path(path)?.as_str(),"manifest":null})
            }
            ModelSource::Manifest { path, asset } => {
                serde_json::json!({"version":1,"scene":app.authoring.authoring_project.source_path(scene)?.as_str(),"model":asset.0,"manifest":app.authoring.authoring_project.source_path(path)?.as_str()})
            }
        };
        let package = app
            .authoring
            .authoring_project
            .capture_package(&paths, &serde_json::to_vec(&launch)?)?;
        for (id, original) in observations {
            if package.inputs().observations().get(&id) != Some(&original) {
                return Err(format!("package source {} changed after validation", id.0).into());
            }
        }
        let bytes = package.value().to_bytes(voxy_assets::PackageLimits {
            max_entries: 4096,
            max_payload_bytes: 64 * 1024 * 1024,
            max_document_bytes: 256 * 1024 * 1024,
        })?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)?;
        std::io::Write::write_all(&mut file, &bytes)?;
        file.sync_all()?;
        println!(
            "GAME PACKAGE PASS sources={} bytes={}",
            paths.len(),
            bytes.len()
        );
        Ok(())
    })();
    app.stop_workers()?;
    result
}
fn configured_app(
    source: &ModelSource,
    scene_path: Option<&std::path::Path>,
    smoke: bool,
) -> Result<App, Box<dyn std::error::Error>> {
    let mut app = match source {
        ModelSource::File(path) => App::new(path, smoke)?,
        ModelSource::Manifest { path, asset } => App::from_manifest(path, asset.clone(), smoke)?,
    };
    if let Some(path) = scene_path
        && let Err(error) = app.configure_scene(path)
    {
        app.stop_workers()?;
        return Err(error);
    }
    Ok(app)
}
/// Returns the editor's built-in codecs. Register application components and
/// collection declarations on this value before passing it to the viewport.
/// # Errors
/// Reports invalid built-in component registration.
pub fn editor_component_registry() -> Result<ComponentRegistry, voxy_scene::DocumentError> {
    model_registry()
}

fn configured_app_with_registry(
    source: &ModelSource,
    scene_path: Option<&std::path::Path>,
    smoke: bool,
    registry: ComponentRegistry,
) -> Result<App, Box<dyn std::error::Error>> {
    let (path, asset) = match source {
        ModelSource::File(path) => (path.canonicalize()?, None),
        ModelSource::Manifest { path, asset } => (path.canonicalize()?, Some(asset.clone())),
    };
    let root = path.parent().ok_or("missing source root")?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("source name must be UTF-8")?;
    let source_path = SourcePath::new(name)?;
    let (id, recipe) = match asset {
        Some(id) => (id, InputRecipe::Manifest(source_path.observation_id())),
        None => (AssetId(name.into()), InputRecipe::Direct),
    };
    let mut app = App::create_with_registry(root, id, recipe, smoke, Arc::new(registry))?;
    if let Some(path) = scene_path
        && let Err(error) = app.configure_scene(path)
    {
        app.stop_workers()?;
        return Err(error);
    }
    Ok(app)
}

/// Runs with application codecs installed before scene loading or worker setup.
/// Start from `editor_component_registry()` to retain the built-in components.
/// The same immutable registry is used for editing, Play, saving and prefab workers.
/// # Errors
/// Reports invalid scenes, component data, imports and native runtime failures.
pub fn run_model_viewport_with_components(
    source: &ModelSource,
    mode: ViewportMode,
    scene_path: Option<&std::path::Path>,
    registry: ComponentRegistry,
    ui_actions: Option<voxy_gameplay::UiActionSetup>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_model_viewport_configured_registry(source, mode, scene_path, ui_actions, Some(registry))
}

/// Runs the native model viewport with asynchronous imports and owner publication.
/// # Errors
/// Reports configuration, window/GPU/worker failures and smoke acceptance timeout.
pub fn run_model_viewport(
    source: &ModelSource,
    mode: ViewportMode,
) -> Result<(), Box<dyn std::error::Error>> {
    run_model_viewport_with_scene(source, mode, None)
}
/// Runs the viewport with an optional authoring file; existing files load before the window opens.
/// # Errors
/// Reports invalid scene/resource references, IO and native viewport failures.
#[allow(clippy::too_many_lines)]
pub fn run_model_viewport_with_scene(
    source: &ModelSource,
    mode: ViewportMode,
    scene_path: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_model_viewport_configured(source, mode, scene_path, None)
}
/// Runs the ordinary editor or standalone mode with application UI callbacks.
/// A fresh registry is configured on every Play; failed setup does not start Play.
/// # Errors
/// Reports callback registration, scene, import and native runtime failures.
pub fn run_model_viewport_with_ui_actions(
    source: &ModelSource,
    mode: ViewportMode,
    scene_path: Option<&std::path::Path>,
    setup: voxy_gameplay::UiActionSetup,
) -> Result<(), Box<dyn std::error::Error>> {
    run_model_viewport_configured(source, mode, scene_path, Some(setup))
}
#[allow(clippy::too_many_lines)]
fn run_model_viewport_configured(
    source: &ModelSource,
    mode: ViewportMode,
    scene_path: Option<&std::path::Path>,
    setup: Option<voxy_gameplay::UiActionSetup>,
) -> Result<(), Box<dyn std::error::Error>> {
    run_model_viewport_configured_registry(source, mode, scene_path, setup, None)
}
fn run_model_viewport_configured_registry(
    source: &ModelSource,
    mode: ViewportMode,
    scene_path: Option<&std::path::Path>,
    setup: Option<voxy_gameplay::UiActionSetup>,
    registry: Option<ComponentRegistry>,
) -> Result<(), Box<dyn std::error::Error>> {
    let smoke = mode == ViewportMode::Smoke;
    let mut app = if let Some(registry) = registry {
        configured_app_with_registry(source, scene_path, smoke, registry)?
    } else {
        configured_app(source, scene_path, smoke)?
    };
    app.ui_action_setup = setup;
    if matches!(
        mode,
        ViewportMode::Game | ViewportMode::GameCheck | ViewportMode::GameSmoke
    ) {
        if scene_path.is_none_or(|path| !path.is_file()) {
            return Err("standalone game requires an existing --scene file".into());
        }
        app.audio
            .set_output_mode(if mode == ViewportMode::GameCheck {
                AudioOutputMode::Offline
            } else {
                AudioOutputMode::Native
            });
        if let Err(error) = app.start_standalone() {
            app.stop_workers()?;
            return Err(error);
        }
        if mode == ViewportMode::GameSmoke {
            app.smoke_deadline = Some(Instant::now() + Duration::from_secs(30));
        }
        if mode == ViewportMode::GameCheck {
            app.check_game_ui()?;
            for _ in 0..120 {
                app.advance_game(1.0 / 60.0)?;
            }
            println!(
                "GAME STATE {}",
                serde_json::to_string(&app.authoring_document()?)?
            );
            if let Some(audio) = app.audio.play() {
                println!("GAME AUDIO frames={} peak={}", audio.frames, audio.peak);
            }
            println!(
                "GAME CHECK PASS ticks={} objects={} prefab_instances={}",
                app.play.simulation_ticks,
                app.object_ids.len(),
                app.authoring
                    .authoring_source
                    .as_ref()
                    .map_or(0, |source| source.value().source.instances.len())
            );
            app.stop_workers()?;
            return Ok(());
        }
    }
    if mode == ViewportMode::AnimationSmoke {
        let mut smoke = animation_smoke::Smoke::default();
        smoke.profile = std::env::var_os("VOXY_ANIMATION_PROFILE").is_some();
        smoke.foot_contact = std::env::var_os("VOXY_FOOT_CONTACT_SMOKE").is_some();
        smoke.foot_reload = std::env::var_os("VOXY_FOOT_RELOAD_SMOKE").is_some();
        smoke.composed_root = std::env::var_os("VOXY_COMPOSED_ROOT_SMOKE").is_some();
        smoke.root_rotation =
            smoke.composed_root || std::env::var_os("VOXY_ROOT_ROTATION_SMOKE").is_some();
        smoke.root_motion =
            smoke.root_rotation || std::env::var_os("VOXY_ROOT_MOTION_SMOKE").is_some();
        smoke.oriented_body = smoke.root_motion
            && !smoke.root_rotation
            && std::env::var_os("VOXY_ORIENTED_CHARACTER_SMOKE").is_some();
        app.animation_smoke = Some(smoke);
        app.smoke_deadline = Some(Instant::now() + Duration::from_secs(30));
    }
    if mode == ViewportMode::PrefabSmoke {
        app.prefab_smoke = Some(prefab_smoke::Smoke::default());
        app.smoke_deadline = Some(Instant::now() + Duration::from_secs(30));
    }
    if mode == ViewportMode::LodSmoke {
        app.lod_smoke = Some(lod_smoke::Smoke::default());
        app.smoke_deadline = Some(Instant::now() + Duration::from_secs(30));
        app.camera.legacy = false;
        app.camera.distance = 1000.;
    }
    if mode == ViewportMode::Scene3DSmoke {
        app.scene3d_smoke = Some(scene3d_smoke::Smoke::default());
        app.smoke_deadline = Some(Instant::now() + Duration::from_secs(15));
    }
    if mode == ViewportMode::GameplaySmoke {
        app.gameplay_smoke = Some(gameplay_smoke::Smoke::new());
        app.smoke_deadline = Some(Instant::now() + Duration::from_secs(15));
    }
    if smoke {
        app.edit_key(KeyCode::KeyD)?;
        app.edit_key(KeyCode::ArrowRight)?;
        app.edit_key(KeyCode::Tab)?;
        app.edit_key(KeyCode::KeyZ)?;
        app.edit_key(KeyCode::KeyY)?;
    }
    let instance_state: Vec<_> = app
        .instances
        .iter()
        .map(|node| app.scene.world_matrix(*node))
        .collect::<Result<_, _>>()?;
    let event_result = (|| {
        EventLoop::new()?.run_app(&mut app)?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })();
    let cleanup = app.stop_workers();
    event_result?;
    cleanup?;
    if let Some(error) = app.error.take() {
        return Err(error.into());
    }
    if smoke {
        if !app.recovered {
            return Err("smoke closed before corrupt-file recovery".into());
        }
        let before_drag = app.authoring_document()?;
        app.begin_drag(Vec2::splat(100.0), Vec2::new(640.0, 480.0))?;
        app.preview_drag(Vec2::new(140.0, 120.0))?;
        app.finish_drag(true)?;
        app.edit_key(KeyCode::KeyZ)?;
        app.begin_drag(Vec2::splat(100.0), Vec2::new(640.0, 480.0))?;
        app.preview_drag(Vec2::new(150.0, 150.0))?;
        app.finish_drag(false)?;
        app.edit_key(KeyCode::KeyY)?;
        app.edit_key(KeyCode::KeyZ)?;
        if app.authoring_document()? != before_drag {
            return Err("smoke drag undo/cancel changed scene".into());
        }
        println!("MODEL DRAG PASS: one transaction, cancellation preserved redo");
        let authoring_before_delete = app.authoring_document()?;
        let count = app.instances.len();
        for _ in 0..count {
            app.edit_key(KeyCode::Delete)?;
        }
        if !app.instances.is_empty() {
            return Err("smoke deletion left instances".into());
        }
        for _ in 0..count {
            app.edit_key(KeyCode::KeyZ)?;
        }
        if app.authoring_document()? != authoring_before_delete {
            return Err("smoke deletion undo changed authoring data".into());
        }
        for _ in 0..count {
            app.edit_key(KeyCode::KeyY)?;
        }
        if !app.instances.is_empty() {
            return Err("smoke deletion redo left instances".into());
        }
        for _ in 0..count {
            app.edit_key(KeyCode::KeyZ)?;
        }
        println!("MODEL DELETE PASS: empty scene and undo/redo restored object identities");
        let resource = app
            .catalog
            .snapshot(&app.id)
            .ok_or("missing recovered resource")?;
        app.edit_key(KeyCode::KeyZ)?;
        app.edit_key(KeyCode::KeyY)?;
        if !app.pick_model(Vec2::new(352.0, 240.0), Vec2::new(640.0, 480.0))? || app.selected != 0 {
            return Err("smoke could not pick recovered model after undo".into());
        }
        let after_undo = app
            .catalog
            .snapshot(&app.id)
            .ok_or("undo removed resource")?;
        if !Arc::ptr_eq(&resource, &after_undo) {
            return Err("undo replaced resource revision".into());
        }
        for (node, expected) in app.instances.iter().zip(instance_state) {
            if app.scene.world_matrix(*node)? != expected {
                return Err("smoke reload changed instance transform".into());
            }
        }
        println!(
            "MODEL INSTANCES PASS: {} independent transforms, shared geometry",
            app.instances.len()
        );
        app.verify_scene_round_trip()?;
        println!(
            "ASSET WINDOW PASS: {} native frames; failed reload retained model; corrected file recovered",
            app.frames
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_view_input_keeps_cameras_independent_and_drag_coordinates_local() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.camera.legacy = false;
        app.camera.perspective = false;
        app.camera.distance = 4.;
        let before = app.authoring_document().unwrap();
        app.edit_key(KeyCode::F10).unwrap();
        assert!(app.msaa4);
        app.edit_key(KeyCode::F4).unwrap();
        assert!(app.split_views());
        let (local, size, origin) = app
            .view_pointer(Vec2::new(750., 350.), Vec2::new(1001., 700.))
            .unwrap()
            .unwrap();
        assert_eq!(
            (local, size, origin),
            (
                Vec2::new(250., 350.),
                Vec2::new(501., 700.),
                Vec2::new(500., 0.)
            )
        );
        assert_eq!(app.active_view, 1);
        assert!(
            app.view_pointer(Vec2::ZERO, Vec2::new(1., 700.))
                .unwrap()
                .is_some()
        );
        assert_eq!(app.active_view, 1);
        assert!(app.camera.perspective);
        assert!(!app.secondary_camera.as_ref().unwrap().perspective);
        app.camera.zoom(2.);
        let right_distance = app.camera.distance;
        assert_ne!(right_distance, 4.);
        assert_eq!(app.secondary_camera.as_ref().unwrap().distance, 4.);
        let owner = app.instances[app.selected];
        let original = app.scene.local(owner).unwrap();
        app.begin_drag(local, size).unwrap();
        app.drag.as_mut().unwrap().viewport_origin = origin;
        app.preview_drag(origin + local).unwrap();
        assert_eq!(app.scene.local(owner).unwrap(), original);
        app.preview_drag(origin + local + Vec2::new(10., 0.))
            .unwrap();
        assert_ne!(app.scene.local(owner).unwrap(), original);
        app.view_pointer(Vec2::new(250., 350.), Vec2::new(1001., 700.))
            .unwrap();
        assert_eq!(app.active_view, 0);
        assert!(app.drag.is_none());
        assert_eq!(app.scene.local(owner).unwrap(), original);
        assert!(!app.camera.perspective);
        assert_eq!(app.camera.distance, 4.);
        app.activate_view(1).unwrap();
        assert_eq!(app.camera.distance, right_distance);
        app.edit_key(KeyCode::F4).unwrap();
        assert!(!app.split_views());
        assert_eq!(app.active_view, 0);
        assert_eq!(app.camera.distance, right_distance);
        app.edit_key(KeyCode::F10).unwrap();
        assert!(!app.msaa4);
        assert_eq!(app.authoring_document().unwrap(), before);
        app.stop_workers().unwrap();
    }

    fn verify_native_play_audio(app: &mut App, expected: &SceneDocument) {
        if std::env::var_os("VOXY_TEST_NATIVE_AUDIO").is_some() {
            app.scene
                .insert_component(
                    app.instances[0],
                    voxy_gameplay::AudioBus { bus: 0, gain: 0.5 },
                )
                .unwrap();
            app.commit_authoring().unwrap();
            let with_bus = app.authoring_document().unwrap();
            app.audio.set_output_mode(AudioOutputMode::Native);
            app.toggle_play().unwrap();
            assert!(app.audio.is_pending() && app.play.playing.is_none());
            assert_eq!(app.authoring_document().unwrap(), with_bus);
            let deadline = Instant::now() + Duration::from_mins(3);
            while app.play.playing.is_none() {
                app.poll_audio_device().unwrap();
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(10));
            }
            app.advance_game(1.0 / 60.0).unwrap();
            std::thread::sleep(Duration::from_millis(150));
            assert!((app.audio.play().unwrap().peak - 0.125).abs() < 1e-6);
            let stats = app.audio.play().unwrap().native_stats().unwrap();
            assert!(stats.supplied > 0);
            assert_eq!(stats.errors, 0);
            println!(
                "EDITOR PLAY AUDIO DEVICE PASS supplied={} errors={}",
                stats.supplied, stats.errors
            );
            app.toggle_play().unwrap();
            assert!(app.audio.play().is_none());
            assert_eq!(app.authoring_document().unwrap(), with_bus);
            app.panel_action(panels::Action::AudioBus).unwrap();
            assert_eq!(&app.authoring_document().unwrap(), expected);
        }
    }
    fn wait_audio_publication(app: &mut App, failed: bool, after: u64) -> u64 {
        let deadline = Instant::now() + Duration::from_secs(5);
        let id = AssetId("tone.wav".into());
        loop {
            app.tick().unwrap();
            let catalog = &app.audio.play().unwrap().catalog;
            let revision = catalog.snapshot_with_revision(&id).unwrap().0;
            let ready = if failed {
                matches!(
                    catalog.status(&id),
                    Some(voxy_assets::AssetStatus::Failed(_))
                )
            } else {
                matches!(catalog.status(&id), Some(voxy_assets::AssetStatus::Ready))
                    && revision > after
            };
            if ready {
                return revision;
            }
            assert!(Instant::now() < deadline, "audio watch did not settle");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn verify_play_audio_watch(app: &mut App, root: &std::path::Path) {
        let baseline = wait_audio_publication(app, false, 0);
        app.scene
            .component_mut::<voxy_gameplay::AudioSource>(app.instances[0])
            .unwrap()
            .unwrap()
            .looping = false;
        app.advance_game(1.0 / 60.0).unwrap();
        app.audio.play_mut().unwrap().peak = 0.;
        let deadline = Instant::now() + Duration::from_millis(400);
        while Instant::now() < deadline {
            app.tick().unwrap();
            app.advance_game(1.0 / 60.0).unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        let audio = app.audio.play().unwrap();
        assert!(
            audio.peak.abs() < 1e-7,
            "unchanged watch restarted a consumed one-shot"
        );
        assert_eq!(
            audio
                .catalog
                .snapshot_with_revision(&AssetId("tone.wav".into()))
                .unwrap()
                .0,
            baseline
        );
        app.scene
            .component_mut::<voxy_gameplay::AudioSource>(app.instances[0])
            .unwrap()
            .unwrap()
            .looping = true;
        let wav_path = root.join("tone.wav");
        let original = std::fs::read(&wav_path).unwrap();
        std::fs::write(&wav_path, b"broken WAV").unwrap();
        assert_eq!(wait_audio_publication(app, true, baseline), baseline);
        app.audio.play_mut().unwrap().peak = 0.;
        app.advance_game(1.0 / 60.0).unwrap();
        assert!((app.audio.play().unwrap().peak - 0.25).abs() < 1e-6);
        let mut repaired = original.clone();
        let end = repaired.len();
        repaired[end - 2..].copy_from_slice(&16384_i16.to_le_bytes());
        std::fs::write(&wav_path, &repaired).unwrap();
        let replacement = wait_audio_publication(app, false, baseline);
        app.audio.play_mut().unwrap().peak = 0.;
        app.advance_game(1.0 / 60.0).unwrap();
        assert!((app.audio.play().unwrap().peak - 0.5).abs() < 1e-6);
        let settings = root.join("tone.import.json");
        let original_settings = std::fs::read(&settings).unwrap();
        std::fs::write(&settings, b"invalid settings").unwrap();
        assert_eq!(wait_audio_publication(app, true, replacement), replacement);
        voxy_gameplay::AudioImportConfig::from_json(&original_settings)
            .unwrap()
            .save_file(&settings)
            .unwrap();
        let recovered = wait_audio_publication(app, false, replacement);
        std::fs::remove_file(&wav_path).unwrap();
        assert_eq!(wait_audio_publication(app, true, recovered), recovered);
        std::fs::write(&wav_path, original).unwrap();
        wait_audio_publication(app, false, recovered);
        println!(
            "EDITOR AUDIO WATCH PASS: invalid WAV/settings and deletion retained publication/PCM; repairs replaced revision"
        );
    }
    fn verify_audio_manifest_watch(root: &std::path::Path) {
        let manifest = root.join("assets.json");
        let write_manifest = |source: &str| {
            std::fs::write(
                &manifest,
                serde_json::to_vec(&serde_json::json!({
                    "version": 1, "assets": [
                        {"asset": "mesh", "source": "quad.obj"},
                        {"asset": "tone.wav", "source": source},
                        {"asset": "tone.import.json", "source": "tone.import.json"}
                    ]
                }))
                .unwrap(),
            )
            .unwrap();
        };
        write_manifest("tone.wav");
        let mut app = App::from_manifest(&manifest, AssetId("mesh".into()), false).unwrap();
        app.scene
            .insert_component(
                app.instances[0],
                voxy_gameplay::AudioSource {
                    asset: "tone.wav".into(),
                    import_settings: Some("tone.import.json".into()),
                    bus: 0,
                    gain: 1.,
                    looping: true,
                    spatial: false,
                    near: 1.,
                    far: 5.,
                },
            )
            .unwrap();
        app.commit_authoring().unwrap();
        let expected = app.authoring_document().unwrap();
        app.toggle_play().unwrap();
        app.advance_game(1.0 / 60.0).unwrap();
        let baseline = wait_audio_publication(&mut app, false, 0);
        write_manifest("missing.wav");
        assert_eq!(wait_audio_publication(&mut app, true, baseline), baseline);
        app.audio.play_mut().unwrap().peak = 0.;
        app.advance_game(1.0 / 60.0).unwrap();
        assert!((app.audio.play().unwrap().peak - 0.25).abs() < 1e-6);
        let mut wav = std::fs::read(root.join("tone.wav")).unwrap();
        let end = wav.len();
        wav[end - 2..].copy_from_slice(&16384_i16.to_le_bytes());
        std::fs::write(root.join("missing.wav"), wav).unwrap();
        wait_audio_publication(&mut app, false, baseline);
        app.audio.play_mut().unwrap().peak = 0.;
        app.advance_game(1.0 / 60.0).unwrap();
        assert!((app.audio.play().unwrap().peak - 0.5).abs() < 1e-6);
        app.toggle_play().unwrap();
        assert_eq!(app.authoring_document().unwrap(), expected);
        app.stop_workers().unwrap();
        println!(
            "EDITOR AUDIO MANIFEST WATCH PASS: remap failure retained PCM; creation of newly observed missing path recovered"
        );
    }
    fn verify_background_audio_preparation(app: &mut App, expected: &SceneDocument) {
        let mut preparation =
            audio_play::AudioPreparation::new(&app.scene, &app.authoring.authoring_project, 48000)
                .unwrap();
        assert!(preparation.matches(&app.scene).unwrap());
        let deadline = Instant::now() + Duration::from_secs(5);
        while !preparation.poll().unwrap() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(&app.authoring_document().unwrap(), expected);
        app.scene
            .component_mut::<voxy_gameplay::AudioSource>(app.instances[0])
            .unwrap()
            .unwrap()
            .import_settings = None;
        assert!(!preparation.matches(&app.scene).unwrap());
        app.scene
            .component_mut::<voxy_gameplay::AudioSource>(app.instances[0])
            .unwrap()
            .unwrap()
            .import_settings = Some("tone.import.json".into());
        preparation.close().join().unwrap();
        let mut preparation =
            audio_play::AudioPreparation::new(&app.scene, &app.authoring.authoring_project, 48000)
                .unwrap();
        assert!(!preparation.poll().unwrap());
        app.audio.inject_preparation(preparation);
        app.toggle_play().unwrap();
        assert!(!app.audio.has_preparation() && app.audio.has_retired_workers());
        assert!(app.play.playing.is_none() && app.play.simulation.is_none());
        assert_eq!(&app.authoring_document().unwrap(), expected);
    }
    #[test]
    fn play_imports_scene_audio_advances_pcm_and_stop_releases_session() {
        let root = std::env::temp_dir().join(format!(
            "voxy-play-audio-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let model = root.join("quad.obj");
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../voxy_render/examples/assets/quad.obj"),
            &model,
        )
        .unwrap();
        let mut wav = b"RIFF\x26\0\0\0WAVEfmt \x10\0\0\0".to_vec();
        for value in [1_u16, 1] {
            wav.extend_from_slice(&value.to_le_bytes());
        }
        for value in [48000_u32, 96000] {
            wav.extend_from_slice(&value.to_le_bytes());
        }
        for value in [2_u16, 16] {
            wav.extend_from_slice(&value.to_le_bytes());
        }
        wav.extend_from_slice(b"data\x02\0\0\0");
        wav.extend_from_slice(&8192_i16.to_le_bytes());
        std::fs::write(root.join("tone.wav"), wav).unwrap();
        let mut app = App::new(&model, false).unwrap();
        let settings_path = root.join("tone.import.json");
        let valid_settings =
            r#"{"version":1,"max_input_bytes":1024,"max_frames":10,"max_filter_evaluations":650}"#;
        std::fs::write(&settings_path, valid_settings).unwrap();
        let descriptor = voxy_gameplay::AudioSource {
            import_settings: Some("tone.import.json".into()),
            asset: "missing.wav".into(),
            bus: 0,
            gain: 1.,
            looping: true,
            spatial: false,
            near: 1.,
            far: 5.,
        };
        app.scene
            .insert_component(app.instances[0], descriptor.clone())
            .unwrap();
        app.commit_authoring().unwrap();
        let before = app.authoring_document().unwrap();
        assert!(app.toggle_play().is_err());
        assert!(app.play.playing.is_none());
        assert_eq!(app.authoring_document().unwrap(), before);
        app.scene
            .insert_component(
                app.instances[0],
                voxy_gameplay::AudioSource {
                    asset: "tone.wav".into(),
                    ..descriptor
                },
            )
            .unwrap();
        app.commit_authoring().unwrap();
        let expected = app.authoring_document().unwrap();
        audio_settings::tests::verify_settings_editor(&mut app, &settings_path);
        verify_background_audio_preparation(&mut app, &expected);
        app.toggle_play().unwrap();
        app.advance_game(1.0 / 60.0).unwrap();
        let audio = app.audio.play().unwrap();
        assert_eq!(audio.frames, 800);
        assert!((audio.peak - 0.25).abs() < 1e-6);
        verify_play_audio_watch(&mut app, &root);
        app.toggle_play().unwrap();
        assert!(app.audio.play().is_none());
        assert_eq!(app.authoring_document().unwrap(), expected);
        std::fs::write(
            &settings_path,
            r#"{"version":1,"max_input_bytes":1,"max_frames":10,"max_filter_evaluations":650}"#,
        )
        .unwrap();
        assert!(app.toggle_play().is_err());
        assert!(app.play.playing.is_none());
        assert_eq!(app.authoring_document().unwrap(), expected);
        std::fs::write(&settings_path, valid_settings).unwrap();
        verify_native_play_audio(&mut app, &expected);
        app.stop_workers().unwrap();
        drop(app);
        verify_audio_manifest_watch(&root);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pending_play_cancel_keeps_authoring_history_and_simulation_stopped() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        let before = app.authoring_document().unwrap();
        app.audio.request_play();
        app.advance_game(1.0).unwrap();
        assert_eq!(app.play.simulation_ticks, 0);
        assert!(app.play.playing.is_none() && app.play.simulation.is_none());
        app.toggle_play().unwrap();
        assert!(!app.audio.is_pending());
        assert_eq!(app.authoring_document().unwrap(), before);
        assert_eq!(app.authoring.history.as_ref().unwrap().current(), &before);
        app.stop_workers().unwrap();
    }

    #[test]
    fn audio_settings_reference_inspector_supports_default_and_history() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.scene
            .insert_component(
                app.instances[0],
                voxy_gameplay::AudioSource {
                    asset: "tone.wav".into(),
                    import_settings: None,
                    bus: 0,
                    gain: 1.,
                    looping: false,
                    spatial: true,
                    near: 1.,
                    far: 5.,
                },
            )
            .unwrap();
        app.commit_authoring().unwrap();
        app.inspector = InspectorMode::Audio;
        app.panel_action(panels::Action::Field(7)).unwrap();
        app.field_key(KeyCode::KeyA, Some("audio/tone.import.json"))
            .unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
        let configured = app.authoring_document().unwrap();
        assert_eq!(
            configured.objects[0].components["game.audio-source.v1"]["import_settings"],
            "audio/tone.import.json"
        );
        app.panel_action(panels::Action::Field(7)).unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
        let defaults = app.authoring_document().unwrap();
        assert!(
            defaults.objects[0].components["game.audio-source.v1"]
                .get("import_settings")
                .is_none()
        );
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), configured);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), defaults);
        app.stop_workers().unwrap();
    }

    #[test]
    fn audio_component_crud_discovers_assets_and_rejects_competing_listener() {
        let root = std::env::temp_dir().join(format!(
            "voxy-audio-crud-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let model = root.join("quad.obj");
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../voxy_render/examples/assets/quad.obj"),
            &model,
        )
        .unwrap();
        let mut app = App::new(&model, false).unwrap();
        let original = app.authoring_document().unwrap();
        assert!(app.panel_action(panels::Action::AudioSource).is_err());
        assert_eq!(app.authoring_document().unwrap(), original);
        std::fs::write(root.join("tone.wav"), []).unwrap();
        app.panel_action(panels::Action::AudioSource).unwrap();
        let source = app.authoring_document().unwrap();
        assert_eq!(
            source.objects[0].components["game.audio-source.v1"]["asset"],
            "tone.wav"
        );
        app.panel_action(panels::Action::AudioSource).unwrap();
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), source);
        app.edit_key(KeyCode::KeyD).unwrap();
        app.selected = 0;
        app.panel_action(panels::Action::AudioListener).unwrap();
        let one_listener = app.authoring_document().unwrap();
        app.selected = 1;
        assert!(app.panel_action(panels::Action::AudioListener).is_err());
        assert_eq!(app.authoring_document().unwrap(), one_listener);
        app.selected = 0;
        app.panel_action(panels::Action::AudioListener).unwrap();
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), one_listener);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(
            app.scene
                .components::<voxy_gameplay::AudioListener>()
                .count(),
            0
        );
        app.stop_workers().unwrap();
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mixer_inspector_round_trip_history_and_duplicate_bus_rollback() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.inspector = InspectorMode::Mixer;
        app.panel_action(panels::Action::AudioBus).unwrap();
        app.panel_action(panels::Action::Field(1)).unwrap();
        app.field_key(KeyCode::Digit2, Some("0.25")).unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
        let expected = app.authoring_document().unwrap();
        assert_eq!(
            expected.objects[0].components["game.audio-bus.v1"]["gain"],
            0.25
        );
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(
            app.authoring_document().unwrap().objects[0].components["game.audio-bus.v1"]["gain"],
            1.
        );
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), expected);
        let serialized = serde_json::to_string(&expected).unwrap();
        let saved: SceneDocument = serde_json::from_str(&serialized).unwrap();
        let loaded = saved.load(&model_registry().unwrap(), 128).unwrap();
        assert!(
            (voxy_gameplay::extract_scene_audio(&loaded.graph, 128)
                .unwrap()
                .buses[0]
                .gain
                - 0.25)
                .abs()
                < 1e-6
        );
        app.panel_action(panels::Action::Field(1)).unwrap();
        app.field_key(KeyCode::Digit2, Some("2")).unwrap();
        assert!(app.field_key(KeyCode::Enter, None).is_err());
        assert_eq!(app.authoring_document().unwrap(), expected);
        assert!(app.edit_key(KeyCode::KeyD).is_err());
        assert_eq!(app.authoring_document().unwrap(), expected);
        app.panel_action(panels::Action::AudioBus).unwrap();
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), expected);
        app.toggle_play().unwrap();
        app.toggle_play().unwrap();
        assert_eq!(app.authoring_document().unwrap(), expected);
        app.stop_workers().unwrap();
    }
    #[test]
    fn audio_inspector_edits_asset_and_parameters_with_validated_history() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.scene
            .insert_component(
                app.instances[0],
                voxy_gameplay::AudioSource {
                    import_settings: None,
                    asset: "tone.wav".into(),
                    bus: 0,
                    gain: 1.,
                    looping: true,
                    spatial: false,
                    near: 1.,
                    far: 5.,
                },
            )
            .unwrap();
        app.commit_authoring().unwrap();
        app.panel_action(panels::Action::Behavior).unwrap();
        app.panel_action(panels::Action::Behavior).unwrap();
        assert_eq!(app.inspector, InspectorMode::Audio);
        for (index, text) in [
            (0, "0.25"),
            (1, "3"),
            (2, "0"),
            (3, "1"),
            (4, "2"),
            (5, "8"),
            (6, "audio/new.wav"),
        ] {
            app.panel_action(panels::Action::Field(index)).unwrap();
            app.field_key(KeyCode::Digit1, Some(text)).unwrap();
            app.field_key(KeyCode::Enter, None).unwrap();
        }
        let saved = app.authoring_document().unwrap();
        let data = &saved.objects[0].components["game.audio-source.v1"];
        assert_eq!(data["asset"], "audio/new.wav");
        assert_eq!(data["bus"], 3);
        assert_eq!(data["looping"], false);
        assert_eq!(data["spatial"], true);
        for (index, text) in [(0, "2"), (1, "1.5"), (2, "2"), (4, "9"), (6, "")] {
            app.panel_action(panels::Action::Field(index)).unwrap();
            app.field_key(KeyCode::Digit1, Some(text)).unwrap();
            assert!(app.field_key(KeyCode::Enter, None).is_err());
            assert_eq!(app.authoring_document().unwrap(), saved);
        }
        app.field_key(KeyCode::Escape, None).unwrap();
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(
            app.authoring_document().unwrap().objects[0].components["game.audio-source.v1"]["asset"],
            "tone.wav"
        );
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), saved);
        app.stop_workers().unwrap();
    }

    #[test]
    fn transform_inspector_accepts_character_rotation_and_rejects_scale_atomically() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.panel_action(panels::Action::Character).unwrap();
        let before = app.authoring_document().unwrap();
        for (index, text) in [(6, "2"), (7, "0.5"), (8, "-1")] {
            app.panel_action(panels::Action::Field(index)).unwrap();
            app.field_key(KeyCode::Digit2, Some(text)).unwrap();
            assert!(app.field_key(KeyCode::Enter, None).is_err());
            assert_eq!(app.authoring_document().unwrap(), before);
            assert_eq!(app.authoring.history.as_ref().unwrap().current(), &before);
        }
        app.panel_action(panels::Action::Field(4)).unwrap();
        app.field_key(KeyCode::Digit9, Some("90")).unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
        let rotated = app.authoring_document().unwrap();
        let rotation = glam::Quat::from_array(rotated.objects[0].rotation);
        assert!(rotation.abs_diff_eq(
            glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            1e-6
        ));
        assert_eq!(app.authoring.history.as_ref().unwrap().current(), &rotated);
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), before);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), rotated);
        app.panel_action(panels::Action::Field(0)).unwrap();
        app.field_key(KeyCode::Digit2, Some("2")).unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
        let moved = app.authoring_document().unwrap();
        assert!((moved.objects[0].translation[0] - 2.).abs() < 1e-6);
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), rotated);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), moved);
        app.stop_workers().unwrap();
    }

    #[test]
    fn duplication_preserves_registered_behavior_and_activity_with_new_identity() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.panel_action(panels::Action::Motion).unwrap();
        app.edit_key(KeyCode::KeyA).unwrap();
        let original = app.authoring_document().unwrap();
        app.edit_key(KeyCode::KeyD).unwrap();
        let duplicated = app.authoring_document().unwrap();
        assert_eq!(duplicated.objects.len(), 2);
        assert_ne!(duplicated.objects[0].id, duplicated.objects[1].id);
        assert_eq!(
            duplicated.objects[0].components,
            duplicated.objects[1].components
        );
        assert!(!duplicated.objects[1].active);
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), original);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), duplicated);
        app.stop_workers().unwrap();
    }

    #[test]
    fn behavior_inspector_edits_undoes_and_rejects_conflicting_writers() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.panel_action(panels::Action::Motion).unwrap();
        app.panel_action(panels::Action::Behavior).unwrap();
        app.panel_action(panels::Action::Field(3)).unwrap();
        app.field_key(KeyCode::Minus, Some("-2")).unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
        let saved = app.authoring_document().unwrap();
        assert_eq!(
            saved.objects[0].components["game.angular-motion.v1"]["radians_per_second"],
            -2.
        );
        app.panel_action(panels::Action::Field(1)).unwrap();
        app.field_key(KeyCode::Digit0, Some("0")).unwrap();
        assert!(app.field_key(KeyCode::Enter, None).is_err());
        assert_eq!(app.authoring_document().unwrap(), saved);
        app.field_key(KeyCode::Escape, None).unwrap();
        assert!(app.panel_action(panels::Action::Collider).is_err());
        assert_eq!(app.authoring_document().unwrap(), saved);
        app.panel_action(panels::Action::Motion).unwrap();
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), saved);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert!(
            !app.authoring_document().unwrap().objects[0]
                .components
                .contains_key("game.angular-motion.v1")
        );
        app.stop_workers().unwrap();
    }

    #[test]
    fn multiple_authored_bodies_collide_in_play_and_restart_from_document() {
        let root =
            std::env::temp_dir().join(format!("voxy-multiple-bodies-play-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("quad.obj"),
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )
        .unwrap();
        let mut app = App::new(&root.join("quad.obj"), false).unwrap();
        let mut document = app.authoring_document().unwrap();
        document.objects.truncate(1);
        let body = &mut document.objects[0];
        body.translation = [0.; 3];
        body.scale = [1.; 3];
        body.rotation = [0., 0., 0., 1.];
        body.components.clear();
        body.components.insert(
            "game.box.v1".into(),
            serde_json::to_value(BoxCollider {
                half_extents: [0.01; 3],
            })
            .unwrap(),
        );
        body.components.insert(
            "game.liquid-body.v1".into(),
            serde_json::to_value(voxy_gameplay::LiquidBody {
                mass_kg: 1.,
                initial_velocity_m_s: [3., 0., 0.],
            })
            .unwrap(),
        );
        body.components.insert(
            "game.liquid-mass.v1".into(),
            serde_json::to_value(voxy_gameplay::LiquidMassDistribution {
                parts: vec![voxy_gameplay::LiquidMassPart {
                    mass_kg: 1.,
                    center_m: [0.02, 0., 0.],
                    half_edges_m: [[0.01, 0., 0.], [0., 0.01, 0.], [0., 0., 0.01]],
                }],
            })
            .unwrap(),
        );
        let mut second = body.clone();
        second.id = ObjectId("second-body".into());
        second.translation = [0.08, 0., 0.];
        second.components.insert(
            "game.liquid-body.v1".into(),
            serde_json::to_value(voxy_gameplay::LiquidBody {
                mass_kg: 1.,
                initial_velocity_m_s: [0.; 3],
            })
            .unwrap(),
        );
        // First owner's geometry lives solely on its child, exercising persisted compounds.
        let mut child = document.objects[0].clone();
        child.id = ObjectId("body-shape".into());
        child.parent = Some(document.objects[0].id.clone());
        child.translation = [0.; 3];
        child.components.remove("game.liquid-body.v1");
        child.components.remove("game.liquid-mass.v1");
        document.objects[0].components.remove("game.box.v1");
        document.objects.push(second);
        document.objects.push(child);
        app.authoring
            .history
            .as_mut()
            .unwrap()
            .commit(document.clone(), &app.authoring.authoring_project.registry)
            .unwrap();
        app.restore_authoring().unwrap();
        app.toggle_play().unwrap();
        app.advance_game(1. / 60.).unwrap();
        app.advance_game(1. / 60.).unwrap();
        let runtime = app.play.liquid.as_ref().unwrap();
        for (_, properties) in runtime.body_mass_properties() {
            let properties = properties.unwrap();
            assert_eq!(properties.mass, 1.);
            assert_eq!(properties.center, [0.02, 0., 0.]);
            assert!(properties.principal_moments.iter().all(|m| *m > 0.));
        }
        for ((node, frame), (owner, state)) in
            runtime.body_rigid_frames().zip(runtime.body_rigid_states())
        {
            assert_eq!(node, owner);
            let frame = frame.unwrap();
            let prepared = frame.prepare_pose(state, 1., 1e-5).unwrap();
            assert_eq!(
                prepared.pose.translation,
                app.scene.local(node).unwrap().translation
            );
            assert_eq!(prepared.pose.scale, app.scene.local(node).unwrap().scale);
        }
        let states: Vec<_> = runtime.body_states().collect();
        assert_eq!(states.len(), 2);
        for (node, state) in states {
            assert!((state.velocity[0] - 1.5).abs() < 1e-10);
            assert!(
                (state.position[0]
                    - f64::from(app.scene.local(node).unwrap().translation.x)
                    - 0.02)
                    .abs()
                    < 1e-5
            );
        }
        runtime.validate_bindings(&app.scene).unwrap();
        app.toggle_play().unwrap();
        assert_eq!(app.authoring_document().unwrap(), document);
        app.toggle_play().unwrap();
        let velocities: Vec<_> = app
            .play
            .liquid
            .as_ref()
            .unwrap()
            .body_states()
            .map(|(_, state)| state.velocity[0])
            .collect();
        assert_eq!(velocities, vec![3., 0.]);
        app.toggle_play().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn authored_liquid_body_moves_in_play_and_stop_restores_both_descriptors() {
        let root =
            std::env::temp_dir().join(format!("voxy-liquid-body-play-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("quad.obj"),
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )
        .unwrap();
        std::fs::write(
            root.join("water.json"),
            r#"{"rest_density":1000,"sound_speed":20,"viscosity":0.001}"#,
        )
        .unwrap();
        let mut app = App::new(&root.join("quad.obj"), false).unwrap();
        let mut document = app.authoring_document().unwrap();
        let body = &mut document.objects[0];
        body.translation = [0.; 3];
        body.scale = [1.; 3];
        body.rotation = glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_4).to_array();
        body.components.insert(
            "game.box.v1".into(),
            serde_json::to_value(BoxCollider {
                half_extents: [0.01, 2., 2.],
            })
            .unwrap(),
        );
        body.components.insert(
            "game.liquid-body.v1".into(),
            serde_json::to_value(voxy_gameplay::LiquidBody {
                mass_kg: 3.,
                initial_velocity_m_s: [0.; 3],
            })
            .unwrap(),
        );
        let mut jet = body.clone();
        jet.id = ObjectId("jet-1".into());
        jet.name = "jet".into();
        jet.translation = [-0.1, 0., 0.];
        jet.rotation = [0., 0., 0., 1.];
        jet.components.clear();
        jet.components.insert(
            "game.liquid-source.v1".into(),
            serde_json::to_value(voxy_gameplay::LiquidSource {
                pulses: vec![voxy_gameplay::LiquidPulse {
                    start_s: 0.,
                    duration_s: 1. / 60.,
                    volume_m3: 0.001,
                    speed_m_s: 2.,
                }],
                density_kg_m3: 1000.,
                particle_volume_m3: 0.001,
                nozzle_radius_m: 0.,
                direction: [1., 0., 0.],
                material_asset: "water.json".into(),
            })
            .unwrap(),
        );
        document.objects.push(jet);
        app.authoring
            .history
            .as_mut()
            .unwrap()
            .commit(document.clone(), &app.authoring.authoring_project.registry)
            .unwrap();
        app.restore_authoring().unwrap();
        app.toggle_play().unwrap();
        app.advance_game(1. / 60.).unwrap();
        app.advance_game(1. / 60.).unwrap();
        let runtime = app.play.liquid.as_ref().unwrap();
        let (owner, state) = runtime.body_state().unwrap();
        assert!(state.velocity[0] > 0.);
        assert!(app.scene.local(owner).unwrap().translation.x > 0.);
        assert_eq!(
            app.scene.local(owner).unwrap().rotation.to_array(),
            document.objects[0].rotation
        );
        assert!((runtime.liquid().mass() - 1.).abs() < 1e-12);
        runtime.validate_bindings(&app.scene).unwrap();
        app.toggle_play().unwrap();
        assert!(app.play.liquid.is_none());
        assert_eq!(app.authoring_document().unwrap(), document);
        app.toggle_play().unwrap();
        assert_eq!(
            app.play
                .liquid
                .as_ref()
                .unwrap()
                .body_state()
                .unwrap()
                .1
                .velocity,
            [0.; 3]
        );
        app.toggle_play().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn authored_liquid_play_steps_and_stop_restores_document() {
        let root = std::env::temp_dir().join(format!("voxy-liquid-play-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("quad.obj"),
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )
        .unwrap();
        std::fs::write(
            root.join("water.json"),
            r#"{"rest_density":1000,"sound_speed":20,"viscosity":0.001,"optics":[0.2,0.1,0.05,1.333]}"#,
        )
        .unwrap();
        let mut app = App::new(&root.join("quad.obj"), false).unwrap();
        let mut document = app.authoring_document().unwrap();
        document.objects[0].components.insert(
            "game.liquid-source.v1".into(),
            serde_json::to_value(voxy_gameplay::LiquidSource {
                pulses: vec![voxy_gameplay::LiquidPulse {
                    start_s: 0.,
                    duration_s: 1.,
                    volume_m3: 0.001,
                    speed_m_s: 2.,
                }],
                density_kg_m3: 1000.,
                particle_volume_m3: 0.001,
                nozzle_radius_m: 0.,
                direction: [1., 0., 0.],
                material_asset: "water.json".into(),
            })
            .unwrap(),
        );
        app.authoring
            .history
            .as_mut()
            .unwrap()
            .commit(document.clone(), &app.authoring.authoring_project.registry)
            .unwrap();
        app.restore_authoring().unwrap();
        app.toggle_play().unwrap();
        assert!(app.play.liquid.is_some());
        assert_eq!(app.play.liquid_optics, vec![Some([0.2, 0.1, 0.05, 1.333])]);
        app.advance_game(1. / 60.).unwrap();
        let runtime = app.play.liquid.as_ref().unwrap();
        assert!((runtime.liquid().mass() - 1. / 60.).abs() < 1e-12);
        assert_eq!(runtime.liquid().particles().len(), 1);
        app.toggle_play().unwrap();
        assert!(app.play.liquid.is_none());
        assert!(app.play.liquid_optics.is_empty());
        assert_eq!(app.authoring_document().unwrap(), document);
        std::fs::remove_file(root.join("water.json")).unwrap();
        assert!(app.toggle_play().is_err());
        assert!(app.play.playing.is_none());
        assert!(app.play.liquid.is_none());
        assert_eq!(app.authoring_document().unwrap(), document);
        std::fs::write(
            root.join("water.json"),
            r#"{"rest_density":1000,"sound_speed":20,"viscosity":0.001,"optics":[0.2,0.1,0.05,1.333]}"#,
        )
        .unwrap();
        app.toggle_play().unwrap();
        assert_eq!(app.play.liquid.as_ref().unwrap().liquid().mass(), 0.);
        app.toggle_play().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn physics_component_inspector_crud_history_and_play_use_authored_descriptors() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.edit_key(KeyCode::KeyC).unwrap();
        app.panel_action(panels::Action::Physics).unwrap();
        app.panel_action(panels::Action::Field(0)).unwrap();
        app.field_key(KeyCode::Digit8, Some("0.08")).unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
        let owner = app.instances[0];
        assert!(
            (app.scene
                .component::<CharacterBody>(owner)
                .unwrap()
                .unwrap()
                .half_extents[0]
                - 0.08)
                .abs()
                < 1e-6
        );
        let saved = app.authoring_document().unwrap();
        app.edit_key(KeyCode::KeyC).unwrap();
        assert!(
            app.scene
                .component::<CharacterBody>(app.instances[0])
                .unwrap()
                .is_none()
        );
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), saved);
        app.panel_action(panels::Action::Field(0)).unwrap();
        app.field_key(KeyCode::Minus, Some("-1")).unwrap();
        assert!(app.field_key(KeyCode::Enter, None).is_err());
        assert_eq!(app.authoring_document().unwrap(), saved);
        app.field_key(KeyCode::Escape, None).unwrap();
        app.toggle_play().unwrap();
        assert!(app.play.physics.is_some());
        let playing_owner = app.instances[0];
        assert_ne!(playing_owner, owner);
        app.game_key(KeyCode::ArrowRight, ElementState::Pressed)
            .unwrap();
        for _ in 0..3 {
            let physics = app.play.physics.as_mut().unwrap();
            app.play
                .simulation
                .as_mut()
                .unwrap()
                .advance_with(&mut app.scene, 1.0 / 60.0, |scene, dt| {
                    physics.fixed_step(scene, &mut app.play.player_input, dt)
                })
                .unwrap();
        }
        assert!(app.scene.local(playing_owner).unwrap().translation.x > 0.0);
        app.toggle_play().unwrap();
        assert!(app.play.physics.is_none());
        assert_eq!(app.authoring_document().unwrap(), saved);
        assert!(!app.play.player_input.state("move_x").unwrap().held);
        app.edit_key(KeyCode::KeyD).unwrap();
        assert_eq!(app.scene.components::<CharacterBody>().count(), 2);
        app.edit_key(KeyCode::KeyB).unwrap();
        assert!(
            app.scene
                .component::<CharacterBody>(app.instances[1])
                .unwrap()
                .is_none()
        );
        assert!(
            app.scene
                .component::<BoxCollider>(app.instances[1])
                .unwrap()
                .is_some()
        );
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.scene.components::<CharacterBody>().count(), 2);
    }
    #[test]
    fn playable_fixture_persists_physics_and_rejects_invalid_parenting_or_scale() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/game");
        let mut app =
            App::from_manifest(&root.join("assets.json"), AssetId("player".into()), false).unwrap();
        app.configure_scene(&root.join("game.scene.json")).unwrap();
        let expected = app.authoring_document().unwrap();
        assert_eq!(expected.objects.len(), 3);
        assert_eq!(app.scene.components::<BoxCollider>().count(), 2);
        app.restore_authoring().unwrap();
        assert_eq!(app.authoring_document().unwrap(), expected);
        app.panel_action(panels::Action::Field(6)).unwrap();
        app.field_key(KeyCode::Digit2, Some("2")).unwrap();
        assert!(app.field_key(KeyCode::Enter, None).is_err());
        app.field_key(KeyCode::Escape, None).unwrap();
        assert_eq!(app.authoring_document().unwrap(), expected);
        app.toggle_play().unwrap();
        app.toggle_play().unwrap();
        assert_eq!(app.authoring_document().unwrap(), expected);
    }
    #[test]
    fn authored_motion_runs_in_fixed_loop_and_stop_restores_descriptor() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.scene
            .insert_component(
                app.instances[0],
                voxy_gameplay::AngularMotion {
                    axis: [0., 0., 1.],
                    radians_per_second: 2.,
                },
            )
            .unwrap();
        app.commit_authoring().unwrap();
        let expected = app.authoring_document().unwrap();
        app.toggle_play().unwrap();
        for _ in 0..60 {
            app.advance_game(1.0 / 60.0).unwrap();
        }
        assert!(
            app.scene
                .local(app.instances[0])
                .unwrap()
                .rotation
                .angle_between(glam::Quat::from_rotation_z(2.))
                < 1e-3
        );
        assert_eq!(app.authoring.history.as_ref().unwrap().current(), &expected);
        app.toggle_play().unwrap();
        assert_eq!(app.authoring_document().unwrap(), expected);
        app.scene
            .insert_component(
                app.instances[0],
                voxy_gameplay::AngularMotion {
                    axis: [0.; 3],
                    radians_per_second: 2.,
                },
            )
            .unwrap();
        assert!(app.toggle_play().is_err());
        assert!(app.play.playing.is_none());
    }
    #[test]
    fn authored_character_turn_uses_physics_and_stop_restores_pose() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        let owner = app.instances[0];
        app.scene
            .insert_component(
                owner,
                CharacterBody {
                    speed: 0.,
                    gravity: 0.,
                    ..Default::default()
                },
            )
            .unwrap();
        app.scene
            .insert_component(
                owner,
                voxy_gameplay::AngularMotion {
                    axis: [0., 1., 0.],
                    radians_per_second: 2.,
                },
            )
            .unwrap();
        app.commit_authoring().unwrap();
        let expected = app.authoring_document().unwrap();
        app.toggle_play().unwrap();
        for _ in 0..60 {
            app.advance_game(1. / 60.).unwrap();
        }
        assert!(
            app.scene
                .local(app.instances[0])
                .unwrap()
                .rotation
                .abs_diff_eq(glam::Quat::from_rotation_y(2.), 1e-5)
        );
        assert_eq!(app.authoring.history.as_ref().unwrap().current(), &expected);
        app.toggle_play().unwrap();
        assert_eq!(app.authoring_document().unwrap(), expected);
    }
    #[test]
    fn shutdown_joins_workers_after_behavior_destroy_panics() {
        #[derive(Debug)]
        struct PanickingDestroy;
        impl Behavior for PanickingDestroy {
            fn on_destroy(&mut self, _: &mut SceneGraph, _: NodeId) {
                panic!("test destroy failure");
            }
        }
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        let mut simulation = SceneSimulation::new(
            &app.scene,
            SimulationLimits {
                fixed_step: 1.0 / 60.0,
                max_steps: 8,
                max_behaviors: 1,
                max_commands: 1,
            },
        )
        .unwrap();
        simulation
            .attach(&mut app.scene, app.instances[0], PanickingDestroy)
            .unwrap();
        app.play.simulation = Some(simulation);
        assert!(
            app.stop_workers()
                .unwrap_err()
                .to_string()
                .contains("panicked")
        );
        assert!(app.imports.is_none());
        assert!(app.watcher.is_none());
        app.stop_workers().unwrap();
    }
    #[test]
    fn shutdown_joins_workers_even_if_simulation_rejects_scene() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        let foreign = SceneGraph::new(1);
        app.play.simulation = Some(
            SceneSimulation::new(
                &foreign,
                SimulationLimits {
                    fixed_step: 1.0 / 60.0,
                    max_steps: 8,
                    max_behaviors: 1,
                    max_commands: 1,
                },
            )
            .unwrap(),
        );
        assert!(
            app.stop_workers()
                .unwrap_err()
                .to_string()
                .contains("simulation stop")
        );
        assert!(app.play.simulation.is_none());
        assert!(app.imports.is_none());
        assert!(app.watcher.is_none());
        app.stop_workers().unwrap();
    }
    #[test]
    fn standalone_playable_fixture_quick_jump_lands_without_losing_actor() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/game");
        let mut app = configured_app(
            &ModelSource::Manifest {
                path: root.join("assets.json"),
                asset: AssetId("player".into()),
            },
            Some(&root.join("game.scene.json")),
            false,
        )
        .unwrap();
        app.start_standalone().unwrap();
        let actor = app.scene.components::<CharacterBody>().next().unwrap().0;
        for _ in 0..60 {
            app.advance_game(1. / 60.).unwrap();
        }
        let before = app.scene.local(actor).unwrap().translation;
        assert!(
            app.play
                .physics
                .as_ref()
                .unwrap()
                .state(&app.scene, actor)
                .unwrap()
                .unwrap()
                .grounded
        );
        app.game_key(KeyCode::Space, ElementState::Pressed).unwrap();
        app.game_key(KeyCode::Space, ElementState::Released)
            .unwrap();
        let mut peak = before.y;
        for _ in 0..180 {
            app.advance_game(1. / 60.).unwrap();
            let position = app.scene.local(actor).unwrap().translation;
            assert!(position.is_finite());
            peak = peak.max(position.y);
            assert!(app.scene.active_in_hierarchy(actor).unwrap());
            assert!(
                app.scene
                    .component::<ModelInstance>(actor)
                    .unwrap()
                    .is_some()
            );
        }
        let after = app.scene.local(actor).unwrap().translation;
        assert!(peak > before.y + 0.1, "quick tap must jump");
        assert!(
            (after.y - before.y).abs() < 1e-5,
            "must land at starting height"
        );
        assert!(
            app.play
                .physics
                .as_ref()
                .unwrap()
                .state(&app.scene, actor)
                .unwrap()
                .unwrap()
                .grounded
        );
        eprintln!(
            "STANDALONE_JUMP before={before:?} peak_y={peak} after={after:?} ticks={}",
            app.play.simulation_ticks
        );
        app.stop_workers().unwrap();
    }
    #[test]
    fn standalone_uses_fixed_loop_without_preview_spin_or_authoring_controls() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        let expected = app.authoring_document().unwrap();
        app.start_standalone().unwrap();
        for _ in 0..120 {
            app.advance_game(1.0 / 60.0).unwrap();
        }
        assert_eq!(app.play.simulation_ticks, 120);
        assert_eq!(app.authoring_document().unwrap(), expected);
        for key in [
            KeyCode::F6,
            KeyCode::Delete,
            KeyCode::KeyD,
            KeyCode::F5,
            KeyCode::F9,
        ] {
            app.edit_key(key).unwrap();
        }
        assert!(app.play.playing.is_some());
        assert_eq!(app.authoring_document().unwrap(), expected);
        assert_eq!(app.authoring.history.as_ref().unwrap().current(), &expected);
        app.stop_workers().unwrap();
    }
    #[test]
    fn registered_ui_callback_uses_scene_barrier_without_creating_input_action() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.toggle_play().unwrap();
        let owner = app.scene.spawn(None, Transform::default()).unwrap();
        app.scene
            .insert_component(
                owner,
                voxy_gameplay::UiElement {
                    origin: [0.0; 2],
                    size: [1.0; 2],
                    color: [1.0; 4],
                    layer: 0,
                    enabled: true,
                    action: Some("voxy.ui.hide".into()),
                    text: None,
                },
            )
            .unwrap();
        app.pointer_press(Vec2::splat(50.0), Vec2::splat(100.0))
            .unwrap();
        app.game_ui_pointer(Vec2::splat(50.0), Vec2::splat(100.0), false)
            .unwrap();
        assert!(app.scene.active_in_hierarchy(owner).unwrap());
        assert!(app.play.player_input.state("voxy.ui.hide").is_none());
        app.advance_game(1.0 / 60.0).unwrap();
        assert!(!app.scene.active_in_hierarchy(owner).unwrap());
        app.toggle_play().unwrap();
        app.stop_workers().unwrap();
    }
    #[test]
    fn ui_activation_reaches_fixed_input_once_and_stop_resets_custom_actions() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.toggle_play().unwrap();
        let owner = app.scene.spawn(None, Transform::default()).unwrap();
        app.scene
            .insert_component(
                owner,
                voxy_gameplay::UiElement {
                    origin: [0.0; 2],
                    size: [1.0; 2],
                    color: [1.0; 4],
                    layer: 0,
                    enabled: true,
                    action: Some("jump".into()),
                    text: None,
                },
            )
            .unwrap();
        app.game_ui_key(KeyCode::Tab, ElementState::Pressed, false)
            .unwrap();
        for key in [KeyCode::Enter, KeyCode::Space] {
            app.game_ui_key(key, ElementState::Pressed, false).unwrap();
        }
        app.game_ui_key(KeyCode::Enter, ElementState::Released, false)
            .unwrap();
        assert!(!app.play.player_input.state("jump").unwrap().pressed);
        app.game_ui_key(KeyCode::Space, ElementState::Released, false)
            .unwrap();
        assert!(app.play.player_input.state("jump").unwrap().pressed);
        app.advance_game(0.0).unwrap();
        assert!(app.play.player_input.state("jump").unwrap().pressed);
        app.advance_game(1.0 / 60.0).unwrap();
        assert!(!app.play.player_input.state("jump").unwrap().pressed);
        app.scene
            .component_mut::<voxy_gameplay::UiElement>(owner)
            .unwrap()
            .unwrap()
            .action = Some("menu".into());
        app.pointer_press(Vec2::splat(50.0), Vec2::splat(100.0))
            .unwrap();
        app.game_ui_pointer(Vec2::splat(50.0), Vec2::splat(100.0), false)
            .unwrap();
        assert!(app.play.player_input.state("menu").unwrap().pressed);
        assert!(!app.play.player_input.state("menu").unwrap().held);
        app.advance_game(1.0 / 60.0).unwrap();
        assert!(!app.play.player_input.state("menu").unwrap().pressed);
        app.toggle_play().unwrap();
        assert!(app.play.player_input.state("menu").is_none());
        app.stop_workers().unwrap();
    }
    #[test]
    fn custom_ui_setup_recreates_callback_state_per_play_and_rejects_failed_start() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        let authoring = app.authoring_document().unwrap();
        let setups = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = Arc::clone(&setups);
        app.ui_action_setup = Some(voxy_gameplay::UiActionSetup::new(move |handlers| {
            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut count = 0_u32;
            handlers.register("app.counter".into(), move |_, event, commands| {
                count += 1;
                let mut local = Transform::default();
                local.translation.x = count as f32;
                commands
                    .push(voxy_scene::SceneCommand::SetLocal(event.owner, local))
                    .map_err(|error| error.to_string())
            })
        }));
        for _ in 0..2 {
            app.toggle_play().unwrap();
            let owner = app.scene.spawn(None, Transform::default()).unwrap();
            app.scene
                .insert_component(
                    owner,
                    voxy_gameplay::UiElement {
                        origin: [0.0; 2],
                        size: [1.0; 2],
                        color: [1.0; 4],
                        layer: 0,
                        enabled: true,
                        action: Some("app.counter".into()),
                        text: None,
                    },
                )
                .unwrap();
            app.activate_game_ui(voxy_gameplay::UiActionEvent {
                owner,
                action: "app.counter".into(),
            })
            .unwrap();
            app.advance_game(0.0).unwrap();
            assert_eq!(app.scene.local(owner).unwrap().translation.x, 1.0);
            app.toggle_play().unwrap();
            assert_eq!(app.authoring_document().unwrap(), authoring);
        }
        assert_eq!(setups.load(std::sync::atomic::Ordering::SeqCst), 2);
        app.ui_action_setup = Some(voxy_gameplay::UiActionSetup::new(|handlers| {
            handlers.register("voxy.ui.hide".into(), |_, _, _| Ok(()))
        }));
        assert!(app.toggle_play().is_err());
        assert!(app.play.playing.is_none());
        assert!(app.play.simulation.is_none());
        assert!(app.play.ui_actions.is_none());
        assert_eq!(app.authoring_document().unwrap(), authoring);
        app.stop_workers().unwrap();
    }

    #[test]
    fn play_pointer_path_activates_runtime_ui_and_stop_cancels_capture() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.toggle_play().unwrap();
        let owner = app.scene.spawn(None, Transform::default()).unwrap();
        app.scene
            .insert_component(
                owner,
                voxy_gameplay::UiElement {
                    origin: [0.0; 2],
                    size: [1.0; 2],
                    color: [1.0; 4],
                    layer: 0,
                    enabled: true,
                    action: Some("start".into()),
                    text: None,
                },
            )
            .unwrap();
        assert!(
            app.game_ui_key(KeyCode::Tab, ElementState::Pressed, false)
                .unwrap()
        );
        assert!(
            app.game_ui_key(KeyCode::Enter, ElementState::Pressed, false)
                .unwrap()
        );
        let (_, keyboard) = app
            .game_ui_input
            .key(
                &app.scene,
                [1280.0, 720.0],
                KeyCode::Enter,
                ElementState::Released,
                false,
                false,
            )
            .unwrap();
        assert_eq!(keyboard.unwrap().owner, owner);
        app.pointer_press(Vec2::splat(50.0), Vec2::splat(100.0))
            .unwrap();
        let event = app
            .game_ui_input
            .pointer(&app.scene, [100.0; 2], [50.0; 2], false)
            .unwrap()
            .unwrap();
        assert_eq!(event.owner, owner);
        assert_eq!(event.action, "start");
        app.scene
            .component_mut::<voxy_gameplay::UiElement>(owner)
            .unwrap()
            .unwrap()
            .action = Some("voxy.ui.hide".into());
        app.game_ui_pointer(Vec2::splat(50.0), Vec2::splat(100.0), true)
            .unwrap();
        app.game_ui_pointer(Vec2::splat(50.0), Vec2::splat(100.0), false)
            .unwrap();
        assert!(app.scene.active_in_hierarchy(owner).unwrap());
        app.advance_game(0.0).unwrap();
        assert!(!app.scene.active_in_hierarchy(owner).unwrap());
        app.scene.set_active(owner, true).unwrap();
        app.pointer_press(Vec2::splat(50.0), Vec2::splat(100.0))
            .unwrap();
        app.toggle_play().unwrap();
        assert!(
            app.game_ui_input
                .pointer(&app.scene, [100.0; 2], [50.0; 2], false)
                .unwrap()
                .is_none()
        );
        app.stop_workers().unwrap();
    }
    #[test]
    fn native_play_simulation_ticks_and_stop_preserve_authoring() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        let expected = app.authoring_document().unwrap();
        app.toggle_play().unwrap();
        let mut ticks = 0;
        for _ in 0..100 {
            ticks += app
                .play
                .simulation
                .as_mut()
                .unwrap()
                .advance(&mut app.scene, 0.01)
                .unwrap()
                .time
                .steps;
        }
        assert_eq!(ticks, 60);
        let rotation = app.scene.local(app.instances[0]).unwrap().rotation;
        assert!(rotation.angle_between(glam::Quat::IDENTITY) < 1e-6);
        assert_eq!(app.authoring.history.as_ref().unwrap().current(), &expected);
        app.toggle_play().unwrap();
        assert_eq!(app.authoring_document().unwrap(), expected);
        assert!(app.play.simulation.is_none());
        app.stop_workers().unwrap();
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn hierarchy_inspector_and_play_preserve_authoring_history() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.edit_key(KeyCode::KeyD).unwrap();
        app.edit_key(KeyCode::KeyP).unwrap();
        app.panel_action(panels::Action::Select(0)).unwrap();
        let parent = app.instances[0];
        let child = app.instances[1];
        assert_eq!(app.scene.parent(child).unwrap(), Some(parent));
        let document = app.authoring_document().unwrap();
        assert_eq!(
            document.objects[1].parent,
            Some(document.objects[0].id.clone())
        );
        app.restore_authoring().unwrap();
        assert_eq!(app.authoring_document().unwrap(), document);
        app.panel_action(panels::Action::Field(0)).unwrap();
        app.field_key(KeyCode::Digit2, Some("2")).unwrap();
        app.field_key(KeyCode::Enter, None).unwrap();
        assert_eq!(
            app.scene.local(app.instances[1]).unwrap().translation.x,
            2.0
        );
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), document);
        app.panel_action(panels::Action::Field(8)).unwrap();
        app.field = Some((8, "NaN".into()));
        assert!(app.field_key(KeyCode::Enter, None).is_err());
        assert_eq!(app.authoring_document().unwrap(), document);
        app.field_key(KeyCode::Escape, None).unwrap();
        // World-X drag must convert through a rotated/scaled parent.
        let parent = app.instances[0];
        let child = app.instances[1];
        let mut local = app.scene.local(parent).unwrap();
        local.rotation = glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        local.scale = Vec3::splat(2.0);
        app.scene.set_local(parent, local).unwrap();
        let document = app.authoring_document().unwrap();
        app.authoring
            .history
            .as_mut()
            .unwrap()
            .commit(document, &model_registry().unwrap())
            .unwrap();
        let before_world = app.scene.world_matrix(child).unwrap().w_axis.truncate();
        app.begin_drag(Vec2::ZERO, Vec2::splat(100.0)).unwrap();
        app.drag.as_mut().unwrap().axis = DragAxis::X;
        app.preview_drag(Vec2::new(10.0, 0.0)).unwrap();
        let after_world = app.scene.world_matrix(child).unwrap().w_axis.truncate();
        assert!((after_world - before_world - Vec3::new(0.2, 0.0, 0.0)).length() < 1e-5);
        app.finish_drag(false).unwrap();
        let authored = app.authoring_document().unwrap();
        let history = app.authoring.history.as_ref().unwrap().current().clone();
        app.toggle_play().unwrap();
        let runtime = app.instances[0];
        let mut local = app.scene.local(runtime).unwrap();
        local.translation = Vec3::splat(9.0);
        app.scene.set_local(runtime, local).unwrap();
        app.edit_key(KeyCode::Delete).unwrap();
        assert_eq!(app.instances.len(), 2);
        assert_eq!(app.authoring.history.as_ref().unwrap().current(), &history);
        app.toggle_play().unwrap();
        assert_eq!(app.authoring_document().unwrap(), authored);
        app.selected = 0;
        app.edit_key(KeyCode::Delete).unwrap();
        assert!(app.instances.is_empty());
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), authored);
        app.stop_workers().unwrap();
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn multiple_resources_reload_independently_and_round_trip() {
        let directory = std::env::temp_dir().join(format!(
            "voxy-editor-multiple-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let triangle = "v -0.5 -0.5 0\nv 0.5 -0.5 0\nv -0.5 0.5 0\nf 1 2 3\n";
        let small = "v 0 0 0\nv 0.2 0 0\nv 0 0.2 0\nf 1 2 3\n";
        std::fs::write(directory.join("a.obj"), triangle).unwrap();
        std::fs::write(directory.join("b.obj"), small).unwrap();
        std::fs::write(directory.join("assets.json"),r#"{"version":1,"assets":[{"asset":"a","source":"a.obj"},{"asset":"b","source":"b.obj"}]}"#).unwrap();
        let mut app =
            App::from_manifest(&directory.join("assets.json"), AssetId("a".into()), false).unwrap();
        app.edit_key(KeyCode::KeyD).unwrap();
        app.edit_key(KeyCode::KeyR).unwrap();
        let a = AssetId("a".into());
        let b = AssetId("b".into());
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.catalog.snapshot(&a).is_none() || app.catalog.snapshot(&b).is_none() {
            app.tick().unwrap();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        // Initial observations deliberately invalidate once to close the baseline race.
        let settle = Instant::now() + Duration::from_millis(650);
        while Instant::now() < settle {
            app.tick().unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }
        let original_a = app.catalog.snapshot(&a).unwrap();
        let original_b = app.catalog.snapshot(&b).unwrap();
        assert_ne!(original_a.value().mesh.vertices().len(), 0);
        assert_ne!(
            original_a.value().mesh.vertices()[0].position,
            original_b.value().mesh.vertices()[0].position
        );
        app.selected = 1;
        app.panel_action(panels::Action::Parent).unwrap();
        app.panel_action(panels::Action::Select(0)).unwrap();
        let expected = app.authoring_document().unwrap();
        assert_eq!(
            expected.objects[1].parent,
            Some(expected.objects[0].id.clone())
        );
        assert_eq!(expected.objects[0].components["editor.model.v1"], "a");
        assert_eq!(expected.objects[1].components["editor.model.v1"], "b");
        assert!(
            app.pick_model(Vec2::new(200.0, 300.0), Vec2::new(640.0, 480.0))
                .unwrap()
        );
        assert_eq!(app.selected, 0);
        app.authoring.scene_path = Some(directory.join("scene.json"));
        app.save_authoring().unwrap();
        let mut fresh =
            App::from_manifest(&directory.join("assets.json"), a.clone(), false).unwrap();
        fresh
            .configure_scene(&directory.join("scene.json"))
            .unwrap();
        assert_eq!(fresh.authoring_document().unwrap(), expected);
        app.edit_key(KeyCode::KeyZ).unwrap(); // detach undo
        app.edit_key(KeyCode::KeyZ).unwrap(); // resource undo
        assert_eq!(
            app.authoring_document().unwrap().objects[1].components["editor.model.v1"],
            "a"
        );
        app.edit_key(KeyCode::KeyY).unwrap();
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), expected);
        std::fs::write(directory.join("b.obj"), "broken").unwrap();
        while app.failed_at.is_none() {
            app.tick().unwrap();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(Arc::ptr_eq(&original_a, &app.catalog.snapshot(&a).unwrap()));
        assert!(Arc::ptr_eq(&original_b, &app.catalog.snapshot(&b).unwrap()));
        std::fs::write(directory.join("b.obj"), triangle).unwrap();
        while Arc::ptr_eq(&original_b, &app.catalog.snapshot(&b).unwrap()) {
            app.tick().unwrap();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(Arc::ptr_eq(&original_a, &app.catalog.snapshot(&a).unwrap()));
        assert_eq!(app.authoring_document().unwrap(), expected);
        app.stop_workers().unwrap();
        fresh.stop_workers().unwrap();
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn gizmo_pointer_gestures_constrain_axes_and_undo_without_changing_resource() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        let asset = ObjAsset::parse(
            &std::fs::read_to_string(&fixture).unwrap(),
            ObjLimits {
                source_bytes: 4096,
                attributes: 64,
                vertices: 64,
                triangles: 64,
            },
        )
        .unwrap();
        let imported = voxy_assets::ImportInputs::new(0, 0)
            .finish(import::EditorAsset::from(asset), |_, _| {
                panic!("unexpected input")
            })
            .unwrap();
        let ticket = app.catalog.request(app.id.clone()).unwrap();
        app.catalog.complete(&ticket, Ok(imported)).unwrap();
        let resource = app.catalog.snapshot(&app.id).unwrap();
        let before = app.authoring_document().unwrap();
        for (origin, expected_axis) in [
            (Vec2::new(360.0, 240.0), DragAxis::X),
            (Vec2::new(320.0, 200.0), DragAxis::Y),
            (Vec2::new(320.0, 240.0), DragAxis::Z),
        ] {
            app.pointer_press(origin, Vec2::new(640.0, 480.0)).unwrap();
            assert_eq!(app.drag.as_ref().unwrap().axis, expected_axis);
            app.preview_drag(origin + Vec2::new(20.0, -20.0)).unwrap();
            app.finish_drag(true).unwrap();
            let moved = app.scene.local(app.instances[0]).unwrap().translation;
            match expected_axis {
                DragAxis::X => {
                    assert!(moved.x > 0.0);
                    assert_eq!(moved.y.to_bits(), 0.0_f32.to_bits());
                    assert_eq!(moved.z.to_bits(), 0.5_f32.to_bits());
                }
                DragAxis::Y => {
                    assert_eq!(moved.x.to_bits(), 0.0_f32.to_bits());
                    assert!(moved.y > 0.0);
                    assert_eq!(moved.z.to_bits(), 0.5_f32.to_bits());
                }
                DragAxis::Z => {
                    assert_eq!(moved.x.to_bits(), 0.0_f32.to_bits());
                    assert_eq!(moved.y.to_bits(), 0.0_f32.to_bits());
                    assert!(moved.z > 0.5);
                }
                DragAxis::Plane => panic!("expected axis"),
            }
            app.edit_key(KeyCode::KeyZ).unwrap();
            assert_eq!(app.authoring_document().unwrap(), before);
            assert!(Arc::ptr_eq(
                &resource,
                &app.catalog.snapshot(&app.id).unwrap()
            ));
        }
        app.stop_workers().unwrap();
    }

    #[test]
    fn authoring_file_round_trip_preserves_ids_transforms_and_resource_references() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let directory = std::env::temp_dir().join(format!(
            "voxy-editor-scene-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("scene.json");
        let mut first = App::new(&fixture, false).unwrap();
        first.authoring.scene_path = Some(path.clone());
        first.edit_key(KeyCode::KeyD).unwrap();
        first.edit_key(KeyCode::ArrowUp).unwrap();
        first
            .scene
            .set_name(first.instances[1], "Saved duplicate")
            .unwrap();
        first.scene.set_active(first.instances[1], false).unwrap();
        let mut local = first.scene.local(first.instances[1]).unwrap();
        local.scale = Vec3::new(1.5, 0.75, 1.0);
        local.rotation = glam::Quat::from_rotation_z(0.3);
        first.scene.set_local(first.instances[1], local).unwrap();
        first.save_authoring().unwrap();
        let saved = first.authoring_document().unwrap();
        let mut reopened = App::new(&fixture, false).unwrap();
        reopened.authoring.scene_path = Some(path.clone());
        reopened.load_authoring().unwrap();
        assert_eq!(reopened.authoring_document().unwrap(), saved);
        assert!(reopened.authoring.authoring_source.is_none());
        assert!(reopened.authoring.scene_revision.is_some());
        reopened.save_authoring().unwrap();
        reopened.save_authoring().unwrap();
        let observed_bytes = std::fs::read(&path).unwrap();
        // Even a formatting-only external write changes the observed revision.
        let mut external_bytes = observed_bytes.clone();
        external_bytes.push(b'\n');
        std::fs::write(&path, &external_bytes).unwrap();
        assert!(reopened.save_authoring().is_err());
        assert_eq!(std::fs::read(&path).unwrap(), external_bytes);
        assert_eq!(reopened.authoring_document().unwrap(), saved);
        std::fs::write(&path, &observed_bytes).unwrap();
        reopened.save_authoring().unwrap();
        reopened.edit_key(KeyCode::KeyD).unwrap();
        assert_eq!(reopened.object_ids[2], ObjectId("model-2".into()));
        let before_invalid = reopened.authoring_document().unwrap();
        std::fs::write(&path, "invalid scene").unwrap();
        assert!(reopened.load_authoring().is_err());
        assert!(reopened.save_authoring().is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "invalid scene");
        assert_eq!(reopened.authoring_document().unwrap(), before_invalid);
        let mut foreign = saved.clone();
        foreign.objects[0].components.insert(
            "editor.model.v1".into(),
            serde_json::Value::String("other-model".into()),
        );
        std::fs::write(&path, foreign.to_json().unwrap()).unwrap();
        assert!(reopened.load_authoring().is_err());
        assert_eq!(reopened.authoring_document().unwrap(), before_invalid);
        reopened.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(reopened.authoring_document().unwrap(), saved);
        first.stop_workers().unwrap();
        reopened.stop_workers().unwrap();
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn drag_is_one_undo_step_and_cancel_preserves_redo() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        let before = app.authoring_document().unwrap();
        app.begin_drag(Vec2::splat(50.0), Vec2::splat(100.0))
            .unwrap();
        for x in 51_u16..=70 {
            app.preview_drag(Vec2::new(f32::from(x), 40.0)).unwrap();
        }
        assert_eq!(app.authoring.history.as_ref().unwrap().current(), &before);
        app.finish_drag(true).unwrap();
        let moved = app.authoring_document().unwrap();
        assert_ne!(before, moved);
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.authoring_document().unwrap(), before);
        app.begin_drag(Vec2::splat(50.0), Vec2::splat(100.0))
            .unwrap();
        app.preview_drag(Vec2::splat(80.0)).unwrap();
        app.edit_key(KeyCode::Escape).unwrap();
        assert_eq!(app.authoring_document().unwrap(), before);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), moved);
        app.begin_drag(Vec2::splat(50.0), Vec2::splat(100.0))
            .unwrap();
        app.preview_drag(Vec2::splat(60.0)).unwrap();
        app.finish_drag(false).unwrap();
        assert_eq!(app.authoring_document().unwrap(), moved);
        app.stop_workers().unwrap();
    }

    #[test]
    fn deletion_restores_empty_scene_and_preserves_object_identity() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.edit_key(KeyCode::KeyD).unwrap();
        let duplicate_id = app.object_ids[1].clone();
        app.edit_key(KeyCode::Delete).unwrap();
        assert_eq!(app.object_ids, vec![ObjectId("model-0".into())]);
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.object_ids[1], duplicate_id);
        app.edit_key(KeyCode::KeyY).unwrap();
        app.edit_key(KeyCode::Backspace).unwrap();
        assert!(app.instances.is_empty());
        app.edit_key(KeyCode::Tab).unwrap();
        app.edit_key(KeyCode::ArrowRight).unwrap();
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.object_ids, vec![ObjectId("model-0".into())]);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert!(app.instances.is_empty());
        app.edit_key(KeyCode::KeyD).unwrap();
        assert_eq!(app.instances.len(), 1);
        assert_ne!(app.object_ids[0], duplicate_id);
        assert_ne!(app.object_ids[0], ObjectId("model-0".into()));
        app.stop_workers().unwrap();
    }

    #[test]
    fn mouse_selection_uses_transforms_depth_activity_and_restored_handles() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        app.edit_key(KeyCode::KeyD).unwrap();
        let bounds = PickBounds {
            min: Vec3::new(-0.1, -0.1, 0.0),
            max: Vec3::new(0.1, 0.1, 0.0),
            layers: 1,
        };
        let size = Vec2::splat(100.0);
        assert!(
            app.pick_bounds(Vec2::new(57.5, 50.0), size, bounds)
                .unwrap()
        );
        assert_eq!(app.selected, 1);
        assert!(app.pick_bounds(Vec2::splat(50.0), size, bounds).unwrap());
        assert_eq!(app.selected, 0);
        assert!(!app.pick_bounds(Vec2::ZERO, size, bounds).unwrap());
        let duplicate = app.instances[1];
        app.scene
            .set_local(
                duplicate,
                Transform {
                    translation: Vec3::new(0.0, 0.0, 0.25),
                    ..Transform::default()
                },
            )
            .unwrap();
        assert!(app.pick_bounds(Vec2::splat(50.0), size, bounds).unwrap());
        assert_eq!(app.selected, 1);
        app.scene.set_active(duplicate, false).unwrap();
        app.pick_bounds(Vec2::splat(50.0), size, bounds).unwrap();
        assert_eq!(app.selected, 0);
        app.edit_key(KeyCode::KeyZ).unwrap();
        app.edit_key(KeyCode::KeyY).unwrap();
        assert!(
            app.pick_bounds(Vec2::new(57.5, 50.0), size, bounds)
                .unwrap()
        );
        assert_eq!(app.selected, 1);
        app.stop_workers().unwrap();
    }

    #[test]
    fn reload_preserves_scene_instance_and_edited_transform() {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../voxy_render/examples/assets/quad.obj");
        let mut app = App::new(&fixture, false).unwrap();
        let node = app.instances[0];
        let edited = Transform {
            translation: Vec3::new(0.2, -0.1, 0.5),
            scale: Vec3::splat(0.75),
            ..Transform::default()
        };
        app.scene.set_local(node, edited).unwrap();
        app.edit_key(KeyCode::KeyD).unwrap();
        let duplicate = app.instances[1];
        app.edit_key(KeyCode::ArrowRight).unwrap();
        let duplicate_matrix = app.scene.world_matrix(duplicate).unwrap();
        assert_ne!(duplicate_matrix, edited.matrix().unwrap());
        app.edit_key(KeyCode::Tab).unwrap();
        assert_eq!(app.instances[app.selected], node);
        let old_ticket = app.catalog.request(app.id.clone()).unwrap();
        app.catalog
            .invalidate(std::slice::from_ref(&app.id))
            .unwrap();
        assert!(matches!(
            app.catalog.complete(&old_ticket, Err("obsolete".into())),
            Err(AssetError::StaleTicket)
        ));
        let new_ticket = app.catalog.request(app.id.clone()).unwrap();
        app.catalog
            .complete(&new_ticket, Err("invalid OBJ".into()))
            .unwrap();
        assert_eq!(app.instances[0], node);
        assert_eq!(app.instances.len(), 2);
        assert_eq!(app.scene.world_matrix(duplicate).unwrap(), duplicate_matrix);
        assert_eq!(
            app.scene
                .component::<ModelInstance>(duplicate)
                .unwrap()
                .unwrap()
                .asset,
            app.id
        );
        assert_eq!(
            app.scene.world_matrix(node).unwrap(),
            edited.matrix().unwrap()
        );
        assert_eq!(
            app.scene
                .component::<ModelInstance>(node)
                .unwrap()
                .unwrap()
                .asset,
            app.id
        );
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_ne!(
            app.scene.world_matrix(app.instances[1]).unwrap(),
            duplicate_matrix
        );
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(
            app.scene.world_matrix(app.instances[1]).unwrap(),
            duplicate_matrix
        );
        app.edit_key(KeyCode::KeyZ).unwrap();
        app.edit_key(KeyCode::KeyZ).unwrap();
        assert_eq!(app.instances.len(), 1);
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.instances.len(), 2);
        app.edit_key(KeyCode::ArrowUp).unwrap();
        let after_new_edit = app.authoring_document().unwrap();
        app.edit_key(KeyCode::KeyY).unwrap();
        assert_eq!(app.authoring_document().unwrap(), after_new_edit);
        app.stop_workers().unwrap();
    }
}
