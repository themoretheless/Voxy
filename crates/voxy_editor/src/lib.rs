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
    pub(crate) mod window_events;
}
pub(crate) use app::window_events::panic_message;
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
pub use lod_smoke::lod_transition_with_hysteresis;
mod package_export;
mod package_models;
mod packaged_game;
mod play_session;
mod viewport_runner;
pub use game_host::{BehaviorSetup, GameHost, run_project_with_host};
pub use package_export::{
    export_game_package, export_project_package, export_project_package_with_components,
    export_project_package_with_host,
};
mod editor_build;
mod project_runtime;
pub use project_runtime::{
    run_project, run_project_with_components, run_project_with_package_player,
};
#[cfg(test)]
pub(crate) use viewport_runner::configured_app_with_registry;
pub(crate) use viewport_runner::{configured_app, run_model_viewport_configured_registry};
pub use viewport_runner::{
    run_model_viewport, run_model_viewport_with_components, run_model_viewport_with_scene,
    run_model_viewport_with_ui_actions, run_project_editor,
};
mod prefab_authoring;
mod prefab_fields;
mod scene_revision;
mod view_layout;
pub use packaged_game::{
    run_packaged_game, run_packaged_game_with_components, run_packaged_game_with_host,
    run_packaged_game_with_ui_actions,
};
pub use view_layout::compute_split_pane_rects;
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
pub use follow_camera::follow_camera_spring_step;
use gizmo::DragAxis;
pub use gizmo::{snap_angle_radians, snap_translation_to_grid, translation_gizmo};
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
    trigger_outline: Option<SceneGeometry>,
    trigger_transforms: BTreeMap<u8, SceneTransform>,
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
            trigger_outline: None,
            trigger_transforms: BTreeMap::new(),
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
                .trigger_outline
                .as_ref()
                .map_or(0, SceneGeometry::allocation_bytes)
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
struct PanelRevisionKey {
    scene_id: voxy_scene::SceneId,
    scene_token: voxy_scene::SceneChangeToken,
    history_doc_ptr: usize,
    retarget_draft: bool,
    bone_picker_page: Option<(usize, usize)>,
    preview_phase_bits: Option<u64>,
    deleted_page: usize,
    focused: Option<voxy_ui::WidgetId>,
    selected: usize,
    size_bits: [u32; 2],
    playing: bool,
    inspector: InspectorMode,
    texture_has_image: bool,
    resource_notice: Option<panels::ResourceNotice>,
    prefab_choice: Option<AssetId>,
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
    panel_cache: Option<PanelRevisionKey>,
    build: editor_build::Build,
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
    gpu_budget_retries: BTreeMap<AssetId, app::model_reload::BudgetRetry>,
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
                triggers: None,
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
            build: Default::default(),
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
            gpu_budget_retries: BTreeMap::new(),
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
        app.build.project = Some((
            opened.descriptor_path.clone(),
            opened.root.join(&opened.descriptor.startup_scene),
        ));
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

        self.queue_gpu_budget_retries();
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

#[cfg(test)]
mod tests;

#[cfg(test)]
use voxy_scene::SimulationLimits;
