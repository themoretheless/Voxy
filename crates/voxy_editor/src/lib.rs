//! Native authoring viewport; IO/decoding is asynchronous, publication/upload is owner-only.
pub mod project;
#[cfg(test)]
mod project_prefab_tests;
mod app {
    mod audio_control;
    mod authoring;
    mod authoring_checkpoint;
    mod game_ui;
    mod inspector;
    mod model_notice;
    pub(crate) mod model_relink;
    pub(crate) mod model_reload;
    mod picking_drag;
    mod play_mode;
    mod residency;
    pub(crate) mod resource_checkpoint;
    pub(crate) mod restore_barrier;
    pub(crate) mod restore_guard;
    pub(crate) mod snapshot_commands;
    mod viewport;
}
mod error;
pub use error::EditorError;
mod animation_preview;
mod animation_sources;
mod asset_checkpoint_wire;
mod audio_play;
mod audio_reload;
mod audio_session;
mod audio_settings;
mod authoring_session;
mod camera;
mod camera_authoring;
#[cfg(test)]
mod camera_authoring_tests;
mod component_collections;
mod component_fields;
mod fog_draw;
mod follow_camera;
#[cfg(test)]
mod follow_camera_tests;
mod frame_pacing;
mod frame_styles;
mod game_host;
mod headless_game;
mod ui_action_codecs;
pub use headless_game::{AnimationSnapshot, HeadlessGame};
#[cfg(test)]
mod game_host_tests;
mod liquid_draw;
mod locomotion;
#[cfg(test)]
mod locomotion_tests;
pub use locomotion::ModelLocomotion;
mod lod_smoke;
mod package_models;
mod packaged_game;
mod play_session;
pub use game_host::{BehaviorSetup, GameHost, run_project_with_host};
mod project_runtime;
pub use project_runtime::{run_project, run_project_with_components};
mod prefab_authoring;
mod prefab_fields;
mod scene_revision;
mod view_layout;
pub use packaged_game::{
    run_packaged_game, run_packaged_game_with_components, run_packaged_game_with_host,
    run_packaged_game_with_ui_actions,
};
mod prefab_smoke;
pub use camera::ViewportCamera as EditorCamera;
mod animation_runtime;
mod foot_placement;
mod gameplay_smoke;
mod gizmo;
mod gpu_model;
mod gpu_source_bindings;
mod model_playback;
mod retarget_authoring;
mod retarget_profile;
pub use foot_placement::{
    FootBinding, FootContactKey, FootOrientation, FootPivotKey, FootRuntime, FootSupportGeometry,
    ModelFootPlacement,
};
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
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelInstance {
    #[serde(with = "model_instance_asset_id")]
    pub asset: voxy_assets::AssetId,
}
mod model_instance_asset_id {
    pub fn serialize<S: serde::Serializer>(
        id: &voxy_assets::AssetId,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&id.0)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<voxy_assets::AssetId, D::Error> {
        <String as serde::Deserialize>::deserialize(deserializer).map(voxy_assets::AssetId)
    }
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
#[cfg(test)]
use voxy_assets::AssetLocations;
use voxy_assets::{
    AssetCatalog, AssetError, AssetId, AssetImportWorker, FileInputs, ImportedAsset,
    PublicationError, SourceDependencies, SourcePath, SourcePollWorker,
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
    SceneGraph, SceneHistory, SceneObject, SceneSimulation, Transform, save_scene_file,
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
    registry.register::<ModelLocomotion>("editor.model-locomotion.v1")?;
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
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    host: Option<SceneSurface>,
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
    fn new(
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
        mut renderer: SceneRenderer,
        host: Option<SceneSurface>,
        panel_texture: Option<SceneTexture>,
        budgets: (u64, u64),
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (image_budget, geometry_budget) = budgets;
        pollster::block_on(renderer.reload_shader(&device, include_str!("material.wgsl")))?;
        let texture = renderer.upload_texture(&device, &queue, 1, 1, &[255; 4])?;
        let gizmo_geometry = renderer.upload_mesh(&device, &translation_gizmo()?)?;
        let gizmo_transform = renderer.create_transform(&device, Mat4::IDENTITY)?;
        let panel_transform = renderer.create_transform(&device, Mat4::IDENTITY)?;
        let animated_models = animated_models::AnimatedModels::new(&renderer)?;
        let mut graphics = Self {
            animated_models,
            adapter,
            device,
            queue,
            host,
            fog: None,
            renderer,
            models: BTreeMap::new(),
            residency_cache: gpu_model::ResidencyCache::with_budget(image_budget),
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
        };
        graphics.residency_cache.geometry_budget = geometry_budget;
        Ok(graphics)
    }
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
    publication: Option<Arc<ImportedAsset<import::EditorAsset>>>,
    geometry: gpu_model::ModelGeometry,
    outline: Option<SceneGeometry>,
    parts: BTreeMap<u32, PartGraphics>,
    animated_textures: Vec<Option<Arc<SceneTexture>>>,
    animated_preview: Vec<SceneGeometry>,
    animated_model: Option<Arc<voxy_model::ModelAsset>>,
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
#[derive(Clone, Debug, PartialEq, Eq)]
enum InputRecipe {
    Direct,
    Manifest(AssetId),
}

impl InputRecipe {
    fn spawn_worker(
        &self,
        root: &std::path::Path,
    ) -> std::io::Result<AssetImportWorker<import::EditorAsset>> {
        let recipe = self.clone();
        AssetImportWorker::new(
            FileInputs::new(root)?,
            import::MODEL_INPUTS,
            import::MODEL_INPUT_BYTES,
            move |asset, provider, inputs| recipe.load(asset, provider, inputs),
        )
    }
    fn load(
        &self,
        asset: &AssetId,
        provider: &FileInputs,
        inputs: &mut voxy_assets::ImportInputs,
    ) -> Result<import::EditorAsset, String> {
        let source_path = match self {
            InputRecipe::Direct => SourcePath::new(asset.0.clone()).map_err(|e| e.to_string())?,
            InputRecipe::Manifest(manifest) => {
                let snapshot = inputs
                    .read(manifest.clone(), |id, limit| {
                        provider.read(id, limit.min(import::MANIFEST_BYTES))
                    })
                    .map_err(|e| format!("{e:?}"))?;
                let locations =
                    import::manifest_locations(&snapshot.bytes).map_err(|e| e.to_string())?;
                locations
                    .source(asset)
                    .cloned()
                    .ok_or("logical resource not found in manifest")?
            }
        };
        import::load(&source_path, provider, inputs).map_err(|e| e.to_string())
    }
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
    resource_notice: Option<panels::ResourceNotice>,
    prefab_label: String,
    import_config: Option<voxy_gameplay::AudioImportConfig>,
    field: Option<(usize, String)>,
    scroll: usize,
    parenting: bool,
    follow_pick: Option<[u8; 32]>,
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
    behavior_setup: Option<BehaviorSetup>,
    behavior_checkpoints: Arc<voxy_scene::BehaviorCheckpointRegistry>,
    ui_action_checkpoints: Arc<voxy_gameplay::UiActionCheckpointRegistry>,
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
    follow_pick: Option<camera_authoring::FollowPick>,
    follow_pick_epoch: u64,
    id: Option<AssetId>,
    catalog: AssetCatalog<ImportedAsset<import::EditorAsset>>,
    import_recipe: InputRecipe,
    pending_model: Option<app::model_reload::Pending>,
    model_relink: Option<app::model_relink::Pending>,
    model_relink_draft: Option<app::model_relink::Draft>,
    retired_model_relink: Option<std::thread::JoinHandle<()>>,
    model_relink_error: Option<(AssetId, String)>,
    resource_checkpoint: Option<app::resource_checkpoint::Barrier>,
    restore_barrier: Option<app::restore_barrier::RestoreBarrier>,
    editor_save: Option<app::restore_barrier::save::PendingEditorSave>,
    editor_load: Option<app::restore_barrier::load::PendingEditorLoad>,
    snapshot_draft: Option<app::snapshot_commands::SnapshotDraft>,
    snapshot_error: Option<String>,
    sources: SourceDependencies,
    imports: Option<AssetImportWorker<import::EditorAsset>>,
    watcher: Option<SourcePollWorker>,
    window: Option<Arc<Window>>,
    window_occluded: bool,
    presentation: frame_pacing::Presentation,
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
        Self::create_session(root, Some(id), recipe, smoke, registry)
    }

    fn create_session(
        root: &std::path::Path,
        id: Option<AssetId>,
        recipe: InputRecipe,
        smoke: bool,
        registry: Arc<ComponentRegistry>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let available: std::collections::BTreeSet<AssetId> = match &recipe {
            InputRecipe::Direct => id.iter().cloned().collect(),
            InputRecipe::Manifest(manifest) => {
                let bytes = FileInputs::new(root)?.read(manifest, import::MANIFEST_BYTES)?;
                import::model_inventory(&bytes)?
            }
        };
        if id.as_ref().is_some_and(|id| !available.contains(id)) {
            return Err("unknown initial model resource".into());
        }
        let authoring_project =
            prefab_authoring::AuthoringProject::with_registry(root, &recipe, registry)?;
        let reload = id.iter().cloned().collect();
        let import_recipe = recipe.clone();
        let imports = recipe.spawn_worker(root)?;
        let mut scene = SceneGraph::new(scene_limits::OBJECTS);
        let mut instances = Vec::new();
        let mut object_ids = Vec::new();
        if let Some(id) = &id {
            let model_node = scene.spawn(
                None,
                Transform {
                    translation: Vec3::new(0.0, 0.0, 0.5),
                    ..Transform::default()
                },
            )?;
            scene.set_name(model_node, "Model 1")?;
            scene.insert_component(model_node, ModelInstance { asset: id.clone() })?;
            instances.push(model_node);
            object_ids.push(ObjectId("model-0".into()));
        }
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
                follow_camera: None,
                physics: None,
                liquid: None,
                liquid_optics: Vec::new(),
                player_input: voxy_gameplay::player_input()?,
                ui_actions: None,
            },
            scene,
            extraction: SceneExtraction::new(scene_limits::OBJECTS),
            frame_styles: None,
            object_ids,
            instances,
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
            behavior_setup: None,
            behavior_checkpoints: Default::default(),
            ui_action_checkpoints: Arc::new(ui_action_codecs::registry()),
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
            follow_pick: None,
            follow_pick_epoch: 0,
            id,
            catalog: AssetCatalog::new(128, 1)?,
            sources: SourceDependencies::new(128, 512),
            imports: Some(imports),
            watcher: Some(SourcePollWorker::new(
                FileInputs::new(root)?,
                257,
                import::SOURCE_BYTES,
            )?),
            import_recipe,
            pending_model: None,
            model_relink: None,
            model_relink_draft: None,
            retired_model_relink: None,
            model_relink_error: None,
            resource_checkpoint: None,
            restore_barrier: None,
            editor_save: None,
            editor_load: None,
            snapshot_draft: None,
            snapshot_error: None,
            window: None,
            window_occluded: false,
            presentation: frame_pacing::Presentation::from_environment()?,
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

    #[cfg(test)]
    fn from_project(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        Self::from_project_with_registry(path, model_registry()?)
    }

    fn from_project_with_registry(
        path: &std::path::Path,
        registry: ComponentRegistry,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let registry = Arc::new(registry);
        let opened = project::open_for_editor(path, &registry)?;
        Self::from_open_project(&opened, registry)
    }

    fn from_open_project(
        opened: &project::OpenProject,
        registry: Arc<ComponentRegistry>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let manifest = opened
            .descriptor
            .asset_manifest
            .to_str()
            .ok_or("manifest path must be UTF-8")?;
        let recipe = InputRecipe::Manifest(SourcePath::new(manifest)?.observation_id());
        let mut app = Self::create_session(&opened.root, None, recipe, false, registry)?;
        if let Err(error) =
            app.configure_scene_for_editor(&opened.root.join(&opened.descriptor.startup_scene))
        {
            app.stop_workers()?;
            return Err(error);
        }
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
        self.presentation.apply(&mut host)?;
        if self.msaa4 {
            host.enable_msaa4()?;
        }
        let renderer = host.create_scene_renderer();
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
        let budgets = (
            std::env::var("VOXY_GPU_IMAGE_BUDGET_BYTES")
                .map_or(Ok(256 * 1024 * 1024), |value| value.parse::<u64>())?,
            std::env::var("VOXY_GPU_GEOMETRY_BUDGET_BYTES")
                .map_or(Ok(256 * 1024 * 1024), |value| value.parse::<u64>())?,
        );
        self.graphics = Some(Graphics::new(
            host.adapter().clone(),
            host.device().clone(),
            host.queue().clone(),
            renderer,
            Some(host),
            panel_texture,
            budgets,
        )?);
        window.set_visible(true);
        window.focus_window();
        window.request_redraw();
        self.window = Some(window);
        if self.smoke_deadline.is_some() {
            self.smoke_deadline = Some(Instant::now() + Duration::from_secs(15));
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn tick(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.poll_model_relink();
        if !self.restore_audio_frozen() {
            self.poll_audio_device()?;
            if let Some(audio) = self.audio.play_mut() {
                audio.poll_reload(&self.scene)?;
            }
        }
        self.queue_retarget_draft_source();
        for asset in self.required_cpu_model_assets() {
            if self.catalog.status(&asset).is_none() {
                self.reload.insert(asset);
            }
        }
        // Apply known invalidations before accepting any ready import completion.
        self.poll_sources()?;
        self.poll_pending_model()?;
        if self.pending_model.is_some() {
            return Ok(());
        }
        let Some(imports) = &mut self.imports else {
            return Ok(());
        };
        if let Some(mut completion) = imports.try_result()? {
            if let Ok(candidate) = &completion.result
                && let Err(error) =
                    self.validate_reload_candidate(completion.ticket.asset(), candidate.value())
            {
                completion.result = completion
                    .result
                    .and_then(|candidate| Err(candidate.into_failed(error.to_string())));
            }
            self.stage_or_complete_reload(completion.ticket, completion.result)?;
        }

        let imports = self
            .imports
            .as_mut()
            .ok_or("import worker disappeared during completion")?;
        if self.resource_checkpoint.is_none()
            && self.pending_model.is_none()
            && !imports.is_busy()
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
        if self.pending_model.is_none() && !self.restore_play_frozen() {
            self.synchronize_gpu_residency();
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
            if let Some(notice) = self.snapshot_path_notice() {
                window.set_title(&format!(
                    "Voxy — {} | {} | Enter confirms, Esc cancels",
                    notice.lines[0], notice.lines[1]
                ));
                return;
            }
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
                "Voxy model viewport — instance {}/{} | view {}/{} | F4 split, F10 smoothing, D duplicate, R model, P parent, A active, F6 play/stop, Z undo, Y redo; Ctrl/Cmd+Shift+S/O snapshot save/open",
                if self.instances.is_empty() { 0 } else { self.selected + 1 },
                self.instances.len(),
                if self.split_views() { self.active_view + 1 } else { 1 },
                if self.split_views() { 2 } else { 1 },
            ));
        }
    }

    fn stop_workers(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut save_errors = self.finish_editor_file_save();
        save_errors.extend(self.finish_editor_file_load());
        self.restore_barrier = None;
        self.model_relink_draft = None;
        self.close_play_audio();
        self.retired_ui_workers.extend(self.ui_live.close());
        self.close_audio_device();
        let mut errors = save_errors;
        let relink = self
            .model_relink
            .take()
            .map(app::model_relink::Pending::close);
        if let Err(error) = self.cancel_pending_model() {
            errors.push(format!("model publication stop: {error}"));
        }
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
        for worker in relink.into_iter().chain(self.retired_model_relink.take()) {
            if let Err(payload) = worker.join() {
                errors.push(format!(
                    "model relink worker panicked: {}",
                    panic_message(payload.as_ref())
                ));
            }
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
                    graphics.host.as_mut().map_or(Ok(()), |host| {
                        host.resize(size.width, size.height)
                            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })
                    })
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
                    && self.panels.as_ref().is_none_or(panels::Panels::input_ready)
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
                    && self.panels.as_ref().is_none_or(panels::Panels::input_ready)
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
            WindowEvent::Occluded(occluded) => {
                self.window_occluded = occluded;
                if occluded {
                    self.authoring.preview_seek = None;
                    if let Some(panels) = &mut self.panels {
                        panels.invalidate_presentation();
                    }
                    self.game_ui_input.cancel();
                    self.camera_drag = None;
                    self.finish_drag(false)
                } else {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    Ok(())
                }
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
                    if event.state == ElementState::Pressed {
                        let command = if self.snapshot_draft.is_some() {
                            self.snapshot_path_key(key, event.text.as_deref())
                                .map(|()| true)
                        } else if !event.repeat {
                            self.snapshot_shortcut(key)
                        } else {
                            Ok(false)
                        };
                        match command {
                            Ok(true) => return,
                            Ok(false) => {}
                            Err(error) => {
                                self.report_snapshot_error(error);
                                return;
                            }
                        }
                    }
                    if event.state == ElementState::Pressed && self.snapshot_escape(key) {
                        return;
                    }
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
        if let Some(window) = &self.window {
            let size = window.inner_size();
            let drawable = !self.window_occluded && size.width > 0 && size.height > 0;
            events.set_control_flow(frame_pacing::control_flow(drawable, Instant::now()));
            window.request_redraw();
        } else {
            events.set_control_flow(ControlFlow::Wait);
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
    let app = configured_app(source, Some(scene), false)?;
    let launch = match source {
        ModelSource::File(path) => {
            serde_json::json!({"version":1,"scene":app.authoring.authoring_project.source_path(scene)?.as_str(),"model":app.authoring.authoring_project.source_path(path)?.as_str(),"manifest":null})
        }
        ModelSource::Manifest { path, asset } => {
            serde_json::json!({"version":1,"scene":app.authoring.authoring_project.source_path(scene)?.as_str(),"model":asset.0,"manifest":app.authoring.authoring_project.source_path(path)?.as_str()})
        }
    };
    export_app_package(app, launch, Vec::new(), output)
}

/// Export a project descriptor and its scene/assets into the existing package format.
/// # Errors
/// Rejects invalid projects, failed imports, changed observed sources, quotas and existing output.
pub fn export_project_package(
    path: &std::path::Path,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    export_project_package_with_components(path, output, editor_component_registry()?)
}

/// Export with the application's component codecs, shared with project runtime.
/// # Errors
/// Reports project, component, import, source consistency and output errors.
pub fn export_project_package_with_components(
    path: &std::path::Path,
    output: &std::path::Path,
    registry: ComponentRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    export_project_package_with_host(path, output, GameHost::new(registry))
}

/// Build using the same executable setup used for project Play and package launch.
/// Code remains in the host binary; the resource package stores data only.
/// # Errors
/// Reports invalid sources, game setup/import failures and output errors.
pub fn export_project_package_with_host(
    path: &std::path::Path,
    output: &std::path::Path,
    host: GameHost,
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = Arc::new(host.components);
    let (opened, descriptor_text) = project::open_observed(path, &registry)?;
    let descriptor = SourcePath::new(
        path.canonicalize()?
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("descriptor must have a UTF-8 name")?,
    )?;
    let manifest = SourcePath::new(
        opened
            .descriptor
            .asset_manifest
            .to_str()
            .ok_or("manifest must be UTF-8")?,
    )?;
    let manifest_bytes =
        FileInputs::new(&opened.root)?.read(&manifest.observation_id(), import::MANIFEST_BYTES)?;
    let locations = import::manifest_locations(&manifest_bytes)?;
    // Keep every declared source: reopening a project validates the complete manifest.
    // Referenced models/audio/fonts additionally contribute their observed import closure.
    let mut extra: Vec<_> = locations
        .bindings()
        .map(|(_, source)| (source.clone(), None))
        .collect();
    extra.push((descriptor.clone(), Some(descriptor_text.into_bytes())));
    extra.push((manifest, Some(manifest_bytes)));
    let mut app = App::from_open_project(&opened, registry)?;
    app.behavior_setup = host.behaviors;
    app.behavior_checkpoints = host.behavior_checkpoints;
    app.ui_action_checkpoints = host.ui_action_checkpoints;
    app.ui_action_setup = host.ui_actions;
    let launch = serde_json::json!({"version":2,"project":descriptor.as_str()});
    export_app_package(app, launch, extra, output)
}

fn export_app_package(
    mut app: App,
    launch: serde_json::Value,
    mut extra: Vec<(SourcePath, Option<Vec<u8>>)>,
    output: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = (|| {
        extra.extend(package_models::declared_sources(&app)?);
        if let Some(prepared) = app
            .authoring
            .authoring_project
            .liquid_runtime_observed(&app.scene)?
        {
            for (id, observation) in prepared.inputs.observations() {
                let snapshot = observation
                    .as_ref()
                    .map_err(|error| format!("liquid package input: {error:?}"))?;
                extra.push((
                    SourcePath::new(id.0.clone())?,
                    Some(snapshot.bytes.to_vec()),
                ));
            }
        }
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
        let mut paths: Vec<_> = observations
            .keys()
            .map(|id| voxy_assets::SourcePath::new(id.0.clone()))
            .collect::<Result<_, _>>()?;
        paths.extend(extra.iter().map(|(source, _)| source.clone()));
        paths.sort();
        paths.dedup();
        let package = app
            .authoring
            .authoring_project
            .capture_package(&paths, &serde_json::to_vec(&launch)?)?;
        for (id, original) in observations {
            if package.inputs().observations().get(&id) != Some(&original) {
                return Err(format!("package source {} changed after validation", id.0).into());
            }
        }
        for (source, expected) in extra {
            if let Some(expected) = expected
                && package
                    .value()
                    .read(&source.observation_id(), 64 * 1024 * 1024)?
                    != expected
            {
                return Err(format!(
                    "project input {} changed during package export",
                    source.as_str()
                )
                .into());
            }
        }
        let bytes = package.value().to_bytes(voxy_assets::PackageLimits {
            max_entries: 4096,
            max_payload_bytes: 64 * 1024 * 1024,
            max_document_bytes: 256 * 1024 * 1024,
        })?;
        voxy_assets::save_atomic_new_file(output, &bytes, 256 * 1024 * 1024)?;
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

/// Opt into logical runtime model bindings in a game-checkpoint registry.
/// Resource revisions and animation state remain owned by their save participants.
/// Authoring scenes continue to use their existing model descriptor projection.
/// # Errors
/// Rejects a duplicate component type or schema; does not replace an existing codec.
pub fn register_game_checkpoint_components(
    registry: &mut ComponentRegistry,
) -> Result<(), voxy_scene::DocumentError> {
    registry.register::<ModelInstance>("runtime.model-instance.v1")
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

/// Open a project in the existing native authoring owner without a bootstrap model.
/// # Errors
/// Reports invalid project/scene data, resource setup and native window/GPU failures.
pub fn run_project_editor(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    run_project(path, ViewportMode::Interactive)
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
    ) && scene_path.is_none_or(|path| !path.is_file())
    {
        app.stop_workers()?;
        return Err("standalone game requires an existing --scene file".into());
    }
    match project_runtime::prepare_game(&mut app, mode) {
        Ok(true) => {
            app.stop_workers()?;
            return Ok(());
        }
        Ok(false) => {}
        Err(error) => {
            app.stop_workers()?;
            return Err(error);
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
            .snapshot(app.id.as_ref().expect("model-only fixture"))
            .ok_or("missing recovered resource")?;
        app.edit_key(KeyCode::KeyZ)?;
        app.edit_key(KeyCode::KeyY)?;
        if !app.pick_model(Vec2::new(352.0, 240.0), Vec2::new(640.0, 480.0))? || app.selected != 0 {
            return Err("smoke could not pick recovered model after undo".into());
        }
        let after_undo = app
            .catalog
            .snapshot(app.id.as_ref().expect("model-only fixture"))
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
mod tests;

#[cfg(test)]
use voxy_scene::SimulationLimits;
