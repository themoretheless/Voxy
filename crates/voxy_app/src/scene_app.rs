//! Animated general 3D geometry plus a textured 2D overlay in a native window.
use glam::{Mat4, Quat, Vec3};
use std::{sync::Arc, time::Instant};
use voxy_render::{
    RenderOutcome, SceneCamera, SceneDraw, SceneGeometry, SceneMesh, SceneProjection,
    SceneRenderer, SceneSurface as Renderer, SceneTexture, SceneTransform, SceneVertex,
};
use voxy_scene::{Behavior, BehaviorRunner, NodeId, SceneGraph, Transform};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

#[derive(Debug)]
struct ResidentHair {
    input: voxy_render::FiberSurfaceInput,
    job: voxy_render::ComputeJob,
    transfer: voxy_render::FiberSurfaceTransfer,
    indices: Vec<u32>,
    dirty: bool,
}
#[derive(Debug)]
struct Resources {
    host: Renderer,
    liquid_renderer: Option<voxy_render::ScreenSpaceFluidRenderer>,
    scene: SceneRenderer,
    cube: SceneGeometry,
    hair_geometry: Option<SceneGeometry>,
    gpu_hair: Option<ResidentHair>,
    gpu_secondary: Option<voxy_render::SurfaceDeformation>,
    gpu_secondary_reference: Vec<[f32; 4]>,
    gpu_secondary_last_controls: Vec<[f32; 4]>,
    film_layer: Option<SceneGeometry>,
    optical_frames: u64,
    film_mass_checks: u64,
    film_mass_max_relative_error: f64,
    overlay: SceneGeometry,
    legend_labels: Vec<[&'static str; 3]>,
    internal: Option<SceneGeometry>,
    texture: SceneTexture,
    material_creases: Vec<f32>,
    atlas_update: crate::female_complexion::AtlasUpdate,
    world_transform: SceneTransform,
    last_view_position: Option<Vec3>,
    last_world_motion: Option<voxy_render::MotionMatrices>,
    last_overlay_mvp: Option<Mat4>,
    overlay_transform: SceneTransform,
    motion_frames: u64,
    rush_floor: Option<SceneGeometry>,
    rush_crate: Option<SceneGeometry>,
    rush_panel: Option<crate::rush_diagnostics::DiagnosticPanel>,
    script_transforms: std::collections::HashMap<NodeId, SceneTransform>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MotionMode {
    Disabled,
    Enabled,
}
/// Shared native scene shell for desktop and mobile entrypoints.
#[derive(Debug)]
pub struct SceneApp {
    female: Option<crate::female_demo::FemaleDemo>,
    orbit_drag: bool,
    camera_motion: crate::camera_motion::CameraMotion,
    full_model_worker: Option<crate::full_model_worker::FullModelWorker>,
    cursor: Option<(f64, f64)>,
    liquids: Option<crate::liquid_demo::LiquidDemo>,
    liquid_optics: bool,
    wear: Option<crate::wear_demo::WearDemo>,
    fem: Option<crate::fem_demo::FemDemo>,
    gravity: Option<crate::gravity_demo::GravityDemo>,
    strands: Option<crate::strands_demo::StrandsDemo>,
    tissues: Option<crate::tissue_demo::TissueDemo>,
    xray: Option<crate::xray_demo::XrayDemo>,
    graphics: voxy_render::GraphicsOptions,
    window: Option<Arc<Window>>,
    resources: Option<Resources>,
    input: crate::SceneInput,
    graph: SceneGraph,
    behaviors: BehaviorRunner,
    rush: Option<crate::scene_rush::SceneRush>,
    rush_smoke: bool,
    frame_loop: voxy_runtime::FrameLoop,
    lifecycle_smoke: bool,
    motion: voxy_render::MotionHistory,
    motion_vectors: MotionMode,
    root: NodeId,
    object: NodeId,
    started: Instant,
    frames: u32,
    profile_last_present: Option<Instant>,
    defer_redraw_until: Option<Instant>,
    resize_stages: u8,
    smoke: bool,
    gravity_smoke_stage: u8,
    paused: bool,
    occluded: bool,
    female_window_title: String,
    /// Last title pushed to the windowing system by the gravity / X-ray / tissue paths.
    last_title: String,
    done: bool,
    recovering_gpu: bool,
    injected_device_loss: bool,
    angle: f32,
    touch: crate::touch::TouchControls,
    last: Instant,
    failure: Option<String>,
}
impl SceneApp {
    /// # Errors
    /// Returns an error if the initial scene hierarchy cannot be created.
    pub fn new(smoke: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let mut graph = SceneGraph::new(1024);
        let root = graph.spawn(None, Transform::default())?;
        let object = graph.spawn(Some(root), Transform::default())?;
        Ok(Self {
            female: None,
            orbit_drag: false,
            camera_motion: Default::default(),
            full_model_worker: None,
            cursor: None,
            liquids: None,
            liquid_optics: false,
            wear: None,
            fem: None,
            gravity: None,
            strands: None,
            tissues: None,
            xray: None,
            graphics: voxy_render::GraphicsOptions::default(),
            input: crate::SceneInput::new(KeyCode::Space, KeyCode::KeyV)?,
            graph,
            behaviors: BehaviorRunner::default(),
            rush: None,
            rush_smoke: false,
            frame_loop: voxy_runtime::FrameLoop::default(),
            lifecycle_smoke: false,
            motion: voxy_render::MotionHistory::default(),
            motion_vectors: MotionMode::Disabled,
            root,
            object,
            window: None,
            resources: None,
            started: Instant::now(),
            frames: 0,
            profile_last_present: None,
            defer_redraw_until: None,
            resize_stages: 0,
            smoke,
            gravity_smoke_stage: 0,
            paused: false,
            occluded: false,
            female_window_title: String::new(),
            last_title: String::new(),
            done: false,
            recovering_gpu: false,
            injected_device_loss: false,
            angle: 0.0,
            touch: crate::touch::TouchControls::default(),
            last: Instant::now(),
            failure: None,
        })
    }
    /// Shows water and oil particle reservoirs with fixed-step SPH simulation.
    /// # Errors
    /// Returns invalid simulation setup errors.
    pub fn with_liquids(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self.female = None;
        self.wear = None;
        self.fem = None;
        self.liquids = Some(crate::liquid_demo::LiquidDemo::new()?);
        self.gravity = None;
        self.xray = None;
        self.tissues = None;
        self.strands = None;
        Ok(self)
    }
    /// Shows finite point sources with mass depletion and energy-accounted recoil.
    pub fn with_finite_liquid_sources(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self = self.with_liquids()?;
        self.liquids = Some(crate::liquid_demo::LiquidDemo::new_finite_sources()?);
        self.liquid_optics = false;
        Ok(self)
    }
    /// Shows jets, impact spray and surface-film deposition for water and oil.
    pub fn with_liquid_impacts(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self = self.with_liquids()?;
        self.liquids = Some(crate::liquid_demo::LiquidDemo::new_impacts()?);
        self.liquid_optics = true;
        Ok(self)
    }
    /// Finite energy/mass sources with recoil, impact spray and deposited film.
    pub fn with_finite_liquid_impacts(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self = self.with_liquids()?;
        self.liquids = Some(crate::liquid_demo::LiquidDemo::new_finite_impacts()?);
        self.liquid_optics = true;
        Ok(self)
    }
    /// Keeps the older particle mesh available for comparison.
    #[must_use]
    pub fn with_liquid_optics(mut self, enabled: bool) -> Self {
        self.liquid_optics = enabled;
        self
    }
    /// Shows prescribed voxel abrasion and the emitted physical suspension.
    pub fn with_voxel_wear(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self = self.with_liquids()?;
        self.liquids = None;
        self.wear = None;
        self.fem = None;
        self.wear = Some(crate::wear_demo::WearDemo::new()?);
        Ok(self)
    }
    /// Dynamic cohesive fracture with current FEM geometry uploaded each frame.
    pub fn with_fem_fracture(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self = self.with_liquids()?;
        self.liquids = None;
        self.wear = None;
        self.fem = Some(crate::fem_demo::FemDemo::new()?);
        Ok(self)
    }
    /// Shows simulated grass and hair instead of the rotating cube.
    #[must_use]
    pub fn with_strands(mut self) -> Self {
        self.female = None;
        self.liquids = None;
        self.wear = None;
        self.fem = None;
        self.xray = None;
        self.tissues = None;
        self.gravity = None;
        self.strands = Some(crate::strands_demo::StrandsDemo::new());
        self
    }
    /// Shows six abstract soft-tissue specimens, left to right:
    /// skin, buttock, breast, lip, sphincter, penis. Space pauses the simulation.
    #[must_use]
    pub fn with_tissues(mut self) -> Self {
        self.xray = None;
        self.female = None;
        self.liquids = None;
        self.wear = None;
        self.fem = None;
        self.gravity = None;
        self.strands = None;
        self.tissues = Some(crate::tissue_demo::TissueDemo::new());
        self
    }
    /// Shows load-controlled finite-element biomechanics specimens.
    /// # Errors
    /// Returns invalid material or mesh construction errors.
    pub fn with_biomechanics(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self = self.with_tissues();
        self.tissues = Some(crate::tissue_demo::TissueDemo::biomechanics()?);
        Ok(self)
    }
    #[must_use]
    pub fn with_body_motion(mut self) -> Self {
        self.female = None;
        self.liquids = None;
        self.wear = None;
        self.fem = None;
        self.gravity = None;
        self.strands = None;
        self.tissues = Some(crate::tissue_demo::TissueDemo::body());
        self
    }
    /// Shows finite spheres under mutual Newtonian gravity.
    #[must_use]
    pub fn with_gravity(mut self, collision: bool) -> Self {
        self.female = None;
        self.liquids = None;
        self.wear = None;
        self.fem = None;
        self.strands = None;
        self.xray = None;
        self.tissues = None;
        self.gravity = Some(crate::gravity_demo::GravityDemo::new(collision));
        self
    }
    /// Starts the gravity-relative planet walking demonstration.
    #[must_use]
    pub fn with_gravity_planet(mut self) -> Self {
        self = self.with_gravity(false);
        if let Some(gravity) = &mut self.gravity {
            gravity.planet();
        }
        self
    }
    /// Shows a simulated muscle ring inside a ghost shell.
    /// X toggles reveal; S toggles 15% speed; arrows orbit; Space pauses.
    #[must_use]
    pub fn with_xray(mut self) -> Self {
        self.female = None;
        self.liquids = None;
        self.wear = None;
        self.fem = None;
        self.gravity = None;
        self.strands = None;
        self.tissues = None;
        self.xray = Some(crate::xray_demo::XrayDemo::new());
        self
    }
    /// Uses the imported Blender body as the X-ray shell, with illustrative internals.
    /// # Errors
    /// Rejects invalid OBJ geometry or asset budgets.
    pub fn with_xray_model(self) -> Result<Self, Box<dyn std::error::Error>> {
        let mut app = self.with_xray();
        app.xray = Some(crate::xray_demo::XrayDemo::with_model()?);
        Ok(app)
    }
    /// Shows Blender Studio's CC0 anatomical female mesh and its nonlinear skin patch.
    /// # Errors
    /// Rejects invalid imported geometry, skin topology or render bindings.
    pub fn with_female(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self.liquids = None;
        self.wear = None;
        self.fem = None;
        self.gravity = None;
        self.strands = None;
        self.tissues = None;
        self.xray = None;
        self.female = Some(crate::female_demo::FemaleDemo::new()?);
        Ok(self)
    }
    /// Shows skeletal animation without advancing the expensive physical shell solver.
    /// # Errors
    /// Rejects invalid character assets.
    pub fn with_female_animation(self) -> Result<Self, Box<dyn std::error::Error>> {
        let mut app = self.with_female()?;
        if let Some(female) = &mut app.female {
            female.animation_only = true;
            female.pressing = false;
        }
        Ok(app)
    }
    /// Opens a close view of skeletal hand closure around a cylinder, sphere, or OBJ mesh.
    /// # Errors
    /// Rejects invalid character or object assets.
    pub fn with_female_grasp(self, object: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut app = self.with_female_animation()?;
        if let Some(female) = &mut app.female {
            female.surface_diffusion_enabled = false;
            female.prepare_grasp_presets()?;
            female.set_grasp_target(object)?;
            female.set_grasp_cycle(true);
            // Show the grasp immediately, then continue the opening/closing loop.
            female.preview_pose(3.0);
            female.set_hand_focus(true);
            female.yaw = -0.85;
            female.pitch = 0.08;
            female.distance = 0.32;
        }
        Ok(app)
    }
    /// Selects the complete inertial-motion example with dynamic hair and skin.
    pub fn with_female_secondary(self) -> Result<Self, Box<dyn std::error::Error>> {
        let mut app = self.with_female()?;
        if let Some(female) = &mut app.female {
            female.secondary_only = true;
            female.yaw = 1.1;
            female.distance = 2.1;
            female.simulate_hair = true;
            female.pressing = false;
        }
        Ok(app)
    }
    /// Sets continuous body proportions for the imported character.
    pub fn with_body_parameters(
        mut self,
        parameters: crate::body_parameters::BodyParameters,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        parameters.validate()?;
        let female = self
            .female
            .as_mut()
            .ok_or("select a female model before body parameters")?;
        female.set_body_parameters(parameters)?;
        female.distance *= parameters.height_cm / 164.;
        Ok(self)
    }
    /// Enables a gradual visual cold response for the selected body. The body's
    /// normalized response parameter becomes the target; time constants are explicit.
    pub fn with_body_cold_response(
        mut self,
        initial: f64,
        onset_seconds: f64,
        recovery_seconds: f64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        self.female
            .as_mut()
            .ok_or("select a body before enabling cold response")?
            .enable_cold_response(initial, onset_seconds, recovery_seconds)?;
        Ok(self)
    }
    /// Watches an atomically replaced JSON parameter file, also written by the MCP server.
    pub fn with_body_parameter_file(
        mut self,
        path: impl Into<std::path::PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.into();
        let parameters =
            crate::body_parameters::BodyParameters::from_json(&std::fs::read_to_string(&path)?)?;
        self = self.with_body_parameters(parameters)?;
        self.female.as_mut().unwrap().parameter_file = Some(path);
        Ok(self)
    }
    /// Watches a detailed face constructor preset, preserving the last valid model on errors.
    pub fn with_face_parameter_file(
        mut self,
        path: impl Into<std::path::PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.into();
        let parameters =
            crate::face_parameters::FaceParameters::from_json(&std::fs::read_to_string(&path)?)?;
        let female = self
            .female
            .as_mut()
            .ok_or("select a female model before face parameters")?;
        female.face_parameters = parameters;
        female.face_parameter_file = Some(path);
        female.yaw = 0.;
        female.distance = 0.5;
        Ok(self)
    }
    /// Deposits a neutral fluid patch on the imported body's reference surface (SI units).
    pub fn with_surface_film(
        mut self,
        center: [f64; 3],
        radius: f64,
        volume_m3: f64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        self.female
            .as_mut()
            .ok_or("select female model first")?
            .enable_film(center, radius, volume_m3)?;
        Ok(self)
    }
    /// Selects graphics options before the platform creates or resumes its surface.
    #[must_use]
    pub fn with_graphics(mut self, options: voxy_render::GraphicsOptions) -> Self {
        self.graphics = options;
        self
    }
    /// Enables the optional float velocity pass, with explicit support failure.
    #[must_use]
    pub fn with_motion_vectors(mut self, enabled: bool) -> Self {
        self.motion_vectors = if enabled {
            MotionMode::Enabled
        } else {
            MotionMode::Disabled
        };
        self
    }
    /// Configures platform key bindings for the named scene actions.
    /// # Errors
    /// Returns invalid binding errors.
    pub fn with_action_keys(
        mut self,
        pause: KeyCode,
        toggle_world: KeyCode,
    ) -> Result<Self, voxy_input::InputError> {
        self.input = crate::SceneInput::new(pause, toggle_world)?;
        Ok(self)
    }

    /// Attaches a Rust behavior to the primary rendered object. Its transform
    /// changes run after the demo's base animation. The primary object must remain
    /// alive because the demo's camera/render resources still reference it.
    /// # Errors
    /// Returns invalid owner errors from the scene runner.
    pub fn with_object_behavior<B: Behavior + 'static>(
        mut self,
        behavior: B,
    ) -> Result<Self, voxy_scene::SceneGraphError> {
        self.behaviors
            .attach(&mut self.graph, self.object, behavior)?;
        Ok(self)
    }

    /// Exercises inherited activity during a smoke run: hides the world object
    /// for frames 40..60 while leaving its overlay visible.
    #[must_use]
    pub fn with_lifecycle_smoke(mut self) -> Self {
        self.lifecycle_smoke = true;
        self
    }

    /// Attaches a persistent .r behavior to the visible scene object.
    pub fn with_script(
        mut self,
        path: impl AsRef<std::path::Path>,
        selected: Vec<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        self.rush = Some(
            crate::scene_rush::SceneRush::new(&mut self.graph, self.object, path, selected)
                .map_err(std::io::Error::other)?,
        );
        Ok(self)
    }

    /// Enables the native demo character collider and a static floor.
    pub fn with_script_character(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self.rush
            .as_mut()
            .ok_or("Attach a script before enabling its character")?
            .enable_character(&mut self.graph)
            .map_err(std::io::Error::other)?;
        Ok(self)
    }

    /// Enables deterministic gameplay input for the bundled player.r smoke demo.
    #[must_use]
    pub fn with_script_smoke(mut self) -> Self {
        self.rush_smoke = true;
        self
    }

    /// Runs the scene on the platform event-loop thread.
    /// # Errors
    /// Returns event-loop, initialization or drawing errors.
    pub fn run(mut self, event_loop: EventLoop<()>) -> Result<(), Box<dyn std::error::Error>> {
        event_loop.set_control_flow(ControlFlow::Poll);
        let result = event_loop.run_app(&mut self);
        self.behaviors.clear(&mut self.graph);
        if let Some(rush) = &mut self.rush {
            rush.scripts
                .clear(&mut self.graph)
                .map_err(std::io::Error::other)?;
            rush.finish_frame();
        }
        result?;
        if let Some(failure) = self.failure {
            return Err(failure.into());
        }
        if self.smoke && !self.done {
            return Err(format!(
                "scene smoke ended before verification: frames={}, resize_stages={}",
                self.frames, self.resize_stages
            )
            .into());
        }
        Ok(())
    }

    fn initialize(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.resources.is_some() {
            return Ok(());
        }
        let window = if let Some(window) = &self.window {
            Arc::clone(window)
        } else {
            Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Voxy 2D/3D | Space: pause | Esc: exit")
                        .with_visible(false)
                        .with_inner_size(if std::env::var_os("VOXY_GPU_SECONDARY").is_some() {
                            winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(1280, 900))
                        } else {
                            winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(900., 650.))
                        }),
                )?,
            )
        };
        let size = window.inner_size();
        let options = self.graphics;
        let instance = options.create_instance_with_display(event_loop.owned_display_handle());
        let mut host = pollster::block_on(Renderer::new_with_instance(
            Arc::clone(&window),
            size.width,
            size.height,
            options,
            instance,
        ))?;
        if std::env::var_os("VOXY_GPU_SECONDARY").is_some()
            || std::env::var_os("VOXY_ASYNC_FULL").is_some()
        {
            match host.enable_unthrottled_presentation() {
                Ok(mode) => eprintln!("GPU SECONDARY presentation={mode:?}"),
                Err(error) => eprintln!(
                    "GPU SECONDARY non-vsync unavailable: {error}; retaining existing presentation"
                ),
            }
        }
        if self.motion_vectors == MotionMode::Enabled {
            pollster::block_on(host.enable_motion_vectors())?;
        }
        if self.female.is_some()
            && (std::env::var_os("VOXY_GPU_SECONDARY").is_none()
                || std::env::var_os("VOXY_GPU_SECONDARY_MSAA").is_some())
        {
            host.enable_msaa4()?;
        }
        if std::env::var_os("VOXY_GPU_SECONDARY").is_some() {
            eprintln!(
                "GPU SECONDARY resolution={}x{} MSAA4={}",
                size.width,
                size.height,
                std::env::var_os("VOXY_GPU_SECONDARY_MSAA").is_some()
            );
        }
        let mut scene = host.create_scene_renderer();
        if self.female.is_some() {
            pollster::block_on(
                scene.reload_shader(host.device(), crate::female_eyes::MATERIAL_SHADER),
            )?;
        }
        let use_gpu_hair = std::env::var_os("VOXY_ASYNC_FULL").is_some()
            && std::env::var_os("VOXY_DISABLE_GPU_HAIR").is_none()
            && self.female.as_ref().is_some_and(|f| {
                f.face_parameters == crate::face_parameters::FaceParameters::default()
                    && f.film.is_none()
                    && f.simulate_hair
            });
        let mesh = if let Some(female) = &self.female {
            if use_gpu_hair {
                female.mesh_with_full_hair()?
            } else {
                female.mesh()?
            }
        } else if let Some(xray) = &self.xray {
            xray.shell(Vec3::new(0.0, 0.2, 3.0))?
        } else if let Some(fem) = &self.fem {
            fem.mesh()?
        } else if let Some(wear) = &self.wear {
            wear.mesh()?
        } else if let Some(liquids) = &self.liquids {
            liquids.mesh()?
        } else if let Some(tissues) = &self.tissues {
            tissues.mesh()?
        } else if let Some(gravity) = &self.gravity {
            gravity.mesh()?
        } else if let Some(strands) = &self.strands {
            strands.mesh()?
        } else if self.rush.is_some() {
            rush_box_mesh([0.2, 0.65, 0.95, 1.0])?
        } else {
            cube_mesh()?
        };
        let mut hair_geometry = None;
        let mut gpu_hair = None;
        let cube = if use_gpu_hair {
            let female = self
                .female
                .as_ref()
                .ok_or("resident hair requires full model")?;
            let (_, indices) =
                crate::full_model_worker::partition_hair(&mesh).map_err(std::io::Error::other)?;
            let mut hair = scene.upload_compute_mesh(host.device(), &mesh)?;
            hair.update_index_partition(
                host.queue(),
                if female.show_hair { &indices } else { &[] },
            )?;
            pollster::block_on(scene.set_geometry_opaque_shader(
                host.device(),
                &mut hair,
                include_str!("female_hair_material.wgsl"),
            ))?;
            let input = female.gpu_hair_surface_input()?;
            let base = mesh
                .vertices()
                .iter()
                .position(|v| v.uv[0] == -7.)
                .ok_or("missing full hair surface")?;
            let program = pollster::block_on(voxy_render::ComputeProgram::new(
                host.device(),
                voxy_render::FIBER_SURFACE_SHADER,
            ))?;
            let job = program.create_job(host.device(), input.bytes())?;
            let transfer = pollster::block_on(voxy_render::FiberSurfaceTransfer::new(
                host.device(),
                &hair,
                job.buffer(),
                &input,
                base as u32,
            ))?;
            eprintln!(
                "FULL MODEL GPU HAIR initialized vertices={} resident=true",
                input.vertex_count()
            );
            gpu_hair = Some(ResidentHair {
                input,
                job,
                transfer,
                indices,
                dirty: true,
            });
            hair_geometry = Some(hair);
            scene.upload_mesh(host.device(), &female.mesh_without_hair()?)?
        } else if std::env::var_os("VOXY_ASYNC_FULL").is_some() {
            let (body, hair) =
                crate::full_model_worker::partition_hair(&mesh).map_err(std::io::Error::other)?;
            let mut partitions = scene.upload_shared_mesh_partitions(host.device(), &mesh)?;
            let mut hair_owner = partitions.pop().unwrap();
            let mut body_owner = partitions.pop().unwrap();
            body_owner.update_index_partition(host.queue(), &body)?;
            hair_owner.update_index_partition(host.queue(), &hair)?;
            pollster::block_on(scene.set_geometry_opaque_shader(
                host.device(),
                &mut hair_owner,
                include_str!("female_hair_material.wgsl"),
            ))?;
            hair_geometry = Some(hair_owner);
            body_owner
        } else if std::env::var_os("VOXY_GPU_SECONDARY").is_some() {
            scene.upload_mesh(host.device(), &SceneMesh::quad([1.; 4]))?
        } else if self.liquids.is_some() || self.wear.is_some() || self.fem.is_some() {
            let mut geometry = scene.reserve_geometry(host.device(), 60_024, 60_024)?;
            geometry.update(host.queue(), &mesh)?;
            geometry
        } else {
            scene.upload_mesh(host.device(), &mesh)?
        };
        let (gpu_secondary, gpu_secondary_reference) = if std::env::var_os("VOXY_GPU_SECONDARY")
            .is_some()
        {
            let female = self
                .female
                .as_ref()
                .ok_or("GPU secondary mode requires the body preview")?;
            let (offsets, weights, count) = female.gpu_secondary_binding(mesh.vertices().len())?;
            eprintln!(
                "GPU SECONDARY preview: {} vertices; neutral initial facial pose and baked initial irradiance; same nonlinear tissue simulation",
                mesh.vertices().len()
            );
            let mut gpu = scene.prepare_surface_deformation(
                host.device(),
                &mesh,
                &offsets,
                &weights,
                count,
            )?;
            gpu.set_deforming_normal_prefix(female.gpu_secondary_body_vertices())?;
            let reference = female.gpu_secondary_controls();
            gpu.update_controls(host.queue(), &vec![[0.; 4]; reference.len()])?;
            (Some(gpu), reference)
        } else {
            (None, Vec::new())
        };
        let rush_floor = self
            .rush
            .as_ref()
            .map(|_| scene.upload_mesh(host.device(), &rush_box_mesh([0.15, 0.19, 0.25, 1.0])?))
            .transpose()?;
        let rush_crate = self
            .rush
            .as_ref()
            .map(|_| scene.upload_mesh(host.device(), &rush_box_mesh([0.95, 0.55, 0.15, 1.0])?))
            .transpose()?;
        let internal = self
            .xray
            .as_ref()
            .map(|xray| scene.upload_mesh(host.device(), &xray.internal()?))
            .transpose()?;
        let overlay_mesh = if self.female.is_some() {
            crate::diagnostic_legend::mesh(
                &self.female.as_ref().unwrap().diagnostic_legend_labels(),
            )?
        } else {
            SceneMesh::quad([1.0, 1.0, 1.0, 0.8])
        };
        let overlay = scene.upload_mesh(host.device(), &overlay_mesh)?;
        let texture = scene.upload_texture(
            host.device(),
            host.queue(),
            2,
            2,
            &[
                255, 180, 50, 255, 50, 180, 255, 255, 50, 180, 255, 255, 255, 180, 50, 255,
            ],
        )?;
        let texture = if let Some(female) = &self.female {
            scene.upload_texture_with_sampling(
                host.device(),
                host.queue(),
                crate::female_complexion::WIDTH,
                crate::female_complexion::SIZE,
                &crate::female_complexion::atlas_for_parameters(&female.face_parameters),
                voxy_render::TextureSampling {
                    min_filter: voxy_render::TextureFilter::Linear,
                    mag_filter: voxy_render::TextureFilter::Linear,
                    ..Default::default()
                },
            )?
        } else if self.liquids.is_some()
            || self.wear.is_some()
            || self.fem.is_some()
            || self.rush.is_some()
            || self.xray.is_some()
            || self.strands.is_some()
            || self.gravity.is_some()
            || self.tissues.is_some()
        {
            scene.upload_texture(host.device(), host.queue(), 1, 1, &[255; 4])?
        } else {
            texture
        };
        let world_transform = scene.create_transform(host.device(), Mat4::IDENTITY)?;
        let overlay_transform = scene.create_transform(host.device(), Mat4::IDENTITY)?;
        println!("SCENE GPU: {:?}", host.adapter_info());
        // Fresh resources (first start or GPU recovery): forget the cached title so it is re-sent.
        self.last_title.clear();
        if self.liquids.is_some() || self.wear.is_some() || self.fem.is_some() {
            window.set_title(
                if self.fem.is_some() { "Voxy FEM | dynamic fracture | Space: pause | R: reset | Esc: exit" } else if self.wear.is_some() { "Voxy wear | receding solid and emitted grains | Space: pause | R: reset | Esc: exit" } else if self.liquid_optics { "Voxy liquid optics | water (left), oil (right) | Space: pause | R: reset | Esc: exit" } else { "Voxy liquids | water (blue), oil (gold) | Space: pause | R: reset | Esc: exit" },
            );
        }
        if self.strands.is_some() {
            window.set_title("Voxy grass & hair | Space: pause | Esc: exit");
        }
        if let Some(gravity) = &self.gravity {
            set_title_if_changed(&window, &mut self.last_title, gravity.title());
        }
        if self.tissues.as_ref().is_some_and(|t| t.is_body()) {
            window.set_title(
                "Voxy skeleton + soft tissues | front / rear | walk, jump, rest | Space: pause",
            );
        } else if self.tissues.is_some() {
            window.set_title(
                "Voxy tissues: skin | buttock | breast | lip | sphincter | penis — Space: pause",
            );
        }
        if let Some(female) = &self.female {
            let title = if gpu_secondary.is_some() {
                format!(
                    "GPU preview | neutral face / initial irradiance | {}",
                    female.title()
                )
            } else {
                female.title()
            };
            self.female_window_title = if self.paused {
                format!("Paused | {title}")
            } else {
                format!("Playing | {title}")
            };
            window.set_title(&self.female_window_title);
        }
        self.motion.reset();
        self.resources = Some(Resources {
            host,
            liquid_renderer: None,
            scene,
            cube,
            hair_geometry,
            gpu_hair,
            gpu_secondary,
            gpu_secondary_last_controls: vec![[0.; 4]; gpu_secondary_reference.len()],
            gpu_secondary_reference,
            film_layer: None,
            optical_frames: 0,
            film_mass_checks: 0,
            film_mass_max_relative_error: 0.,
            overlay,
            legend_labels: self
                .female
                .as_ref()
                .map(|f| f.diagnostic_legend_labels())
                .unwrap_or_default(),
            internal,
            texture,
            material_creases: self
                .female
                .as_ref()
                .map(|f| f.face_parameters.material_signature())
                .unwrap_or_default(),
            atlas_update: Default::default(),
            world_transform,
            last_view_position: None,
            last_world_motion: None,
            last_overlay_mvp: None,
            overlay_transform,
            motion_frames: 0,
            rush_floor,
            rush_crate,
            rush_panel: None,
            script_transforms: std::collections::HashMap::new(),
        });
        window.set_visible(true);
        if self.rush.is_some()
            || self.smoke
            || self.xray.is_some()
            || self.female.is_some()
            || self.strands.is_some()
            || self.gravity.is_some()
        {
            window.focus_window();
        }
        self.window = Some(window);
        if self.full_model_worker.is_none() && std::env::var_os("VOXY_ASYNC_FULL").is_some() {
            let simulation = self
                .female
                .take()
                .ok_or("background full model requires female example")?;
            let view = simulation.background_view_replica()?;
            let controls = crate::full_model_worker::Controls::capture(&view, self.paused);
            self.full_model_worker = Some(crate::full_model_worker::FullModelWorker::new(
                simulation,
                controls,
                self.resources
                    .as_ref()
                    .is_some_and(|r| r.gpu_hair.is_some()),
            )?);
            self.female = Some(view);
        }
        self.last = Instant::now();
        Ok(())
    }
    #[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
    fn draw(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        if self.done {
            return Ok(());
        }
        if let Some(message) = self
            .resources
            .as_ref()
            .and_then(|r| r.host.device_failure())
        {
            return Err(voxy_render::RendererError::DeviceLost(message.to_owned()).into());
        }
        let trace = self.female.is_some()
            && (std::env::var_os("VOXY_FRAME_PROFILE").is_some()
                || (std::env::var_os("VOXY_FACE_FRAME_TRACE").is_some() && self.frames < 8));
        let frame_started = Instant::now();
        if trace {
            eprintln!("FACE FRAME {} begin", self.frames);
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f32();
        if trace {
            eprintln!(
                "FRAME CADENCE frame={} elapsed_ms={:.3} paused={}",
                self.frames,
                f64::from(elapsed) * 1000.,
                self.paused
            );
        }
        let dt = elapsed.min(0.1);
        self.last = now;
        if let Some(female) = &mut self.female {
            let view = self.camera_motion.advance(
                [female.yaw, female.pitch, female.distance],
                dt,
                female.hand_focus,
            );
            [female.yaw, female.pitch, female.distance] = view;
        }
        self.frame_loop.set_paused(self.paused);
        let frame_work = self.frame_loop.advance(f64::from(elapsed));
        if !self.paused {
            if let Some(female) = &mut self.female {
                if self.full_model_worker.is_none() {
                    female.advance(if self.smoke {
                        1.0 / 60.0
                    } else {
                        f64::from(elapsed)
                    })?;
                }
            } else if let Some(xray) = &mut self.xray {
                if self.smoke {
                    xray.enabled = !(40..80).contains(&self.frames);
                    xray.slow = self.frames >= 80;
                }
                xray.advance(f64::from(dt))?;
            } else if let Some(fem) = &mut self.fem {
                fem.advance(if self.smoke {
                    1.0 / 60.0
                } else {
                    f64::from(dt)
                })?;
            } else if let Some(wear) = &mut self.wear {
                wear.advance(if self.smoke {
                    1.0 / 60.0
                } else {
                    f64::from(dt)
                })?;
            } else if let Some(liquids) = &mut self.liquids {
                liquids.advance(if self.smoke {
                    1.0 / 60.0
                } else {
                    f64::from(dt)
                })?;
            } else if let Some(tissues) = &mut self.tissues {
                tissues.advance(f64::from(dt))?;
                if let Some(title) = tissues
                    .body_motion_title()
                    .or_else(|| tissues.biomechanics_title())
                    && let Some(window) = &self.window
                {
                    set_title_if_changed(window, &mut self.last_title, title);
                }
            } else if let Some(gravity) = &mut self.gravity {
                if self.smoke && self.frames >= 30 && self.gravity_smoke_stage == 0 {
                    gravity.reset(true);
                    self.gravity_smoke_stage = 1;
                }
                if self.smoke
                    && self.frames >= 90
                    && self.gravity_smoke_stage == 1
                    && gravity.contacts > 0
                {
                    println!("GRAVITY COLLISION PASS: {} contacts", gravity.contacts);
                    gravity.planet();
                    self.gravity_smoke_stage = 2;
                }
                gravity.advance(f64::from(dt))?;
            } else if let Some(strands) = &mut self.strands {
                strands.advance(f64::from(dt))?;
            } else {
                self.angle += dt;
            }
        }
        if self.rush.is_none() {
            self.graph.set_local(
                self.root,
                Transform {
                    rotation: Quat::from_rotation_y(self.angle),
                    ..Default::default()
                },
            )?;
            self.graph.set_local(
                self.object,
                Transform {
                    rotation: Quat::from_rotation_x(self.angle * 0.4),
                    ..Default::default()
                },
            )?;
        }
        if self.lifecycle_smoke && self.smoke && (self.frames == 40 || self.frames == 60) {
            self.graph.set_active(self.root, self.frames == 60)?;
            self.motion.reset();
        }
        if let Some(rush) = &mut self.rush {
            rush.poll_reload(&self.graph);
        }
        if self.paused {
            self.behaviors.sync(&mut self.graph);
        } else {
            let delta = frame_work.delta_seconds;
            if let Some(rush) = &mut self.rush {
                if self.rush_smoke {
                    rush.smoke_input(self.frames);
                }
                rush.begin_frame(&mut self.graph)
                    .map_err(std::io::Error::other)?;
            }
            let fixed_seconds = self.frame_loop.fixed_seconds();
            for _ in 0..frame_work.fixed.steps {
                self.behaviors.fixed_update(&mut self.graph, fixed_seconds);
                if let Some(rush) = &mut self.rush {
                    rush.fixed_update(&mut self.graph, fixed_seconds)
                        .map_err(std::io::Error::other)?;
                }
            }
            self.behaviors.update(&mut self.graph, delta);
            if let Some(rush) = &mut self.rush {
                rush.scripts
                    .update(&mut self.graph, delta)
                    .map_err(std::io::Error::other)?;
                rush.finish_frame();
            }
        }
        if trace {
            eprintln!(
                "FACE FRAME {} animation_ready_ms={:.3}",
                self.frames,
                frame_started.elapsed().as_secs_f64() * 1000.
            );
        }
        let Some(window) = &self.window else {
            return Ok(());
        };
        if let Some(gravity) = &self.gravity {
            set_title_if_changed(window, &mut self.last_title, gravity.title());
        }
        if let Some(xray) = &self.xray {
            set_title_if_changed(window, &mut self.last_title, xray.title());
        }
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let Some(r) = &mut self.resources else {
            return Ok(());
        };
        // Native window size may change before its Resized event is dispatched.
        // Synchronize all render targets before deriving camera matrices.
        if r.host.configured_size() != (size.width, size.height)
            || r.host.surface_state() == voxy_render::SurfaceState::Suspended
        {
            r.host.resize(size.width, size.height)?;
            self.motion.reset();
        }
        if self.smoke && self.frames == 15 {
            self.motion.reset();
            r.host.resize(0, 0)?;
            if r.host.motion_frame().is_some() {
                return Err("suspended surface exposed stale motion frame".into());
            }
            if r.host.render_scene(&r.scene, &[])? != RenderOutcome::Suspended {
                return Err("zero-size surface did not suspend".into());
            }
            r.host.resize(size.width, size.height)?;
            if r.host.motion_frame().is_some() {
                return Err("resized surface exposed uninitialized motion frame".into());
            }
        }
        let camera = SceneCamera {
            eye: if let Some(female) = &self.female {
                female.eye()
            } else if let Some(xray) = &self.xray {
                xray.eye(self.graph.world_matrix(self.object)?)
            } else if self.liquid_optics && self.liquids.is_some() {
                Vec3::new(0.0, 0.1, 3.5)
            } else if self.liquids.is_some() || self.wear.is_some() || self.fem.is_some() {
                Vec3::new(0.0, 1.4, 6.0)
            } else if let Some(gravity) = &self.gravity {
                Vec3::new(
                    0.0,
                    0.0,
                    gravity.camera_distance(size.width as f32 / size.height as f32),
                )
            } else if self.strands.is_some() || self.tissues.is_some() {
                Vec3::new(0.0, 0.2, 6.0)
            } else {
                Vec3::new(3.0, 2.0, 4.0)
            },
            target: if let Some(female) = &self.female {
                female.focus()
            } else if let Some(xray) = &self.xray {
                self.graph
                    .world_matrix(self.object)?
                    .transform_point3(xray.focus())
            } else if self.liquid_optics && self.liquids.is_some() {
                Vec3::new(0.0, -0.35, 0.0)
            } else {
                Vec3::ZERO
            },
            up: Vec3::Y,
            projection: SceneProjection::Perspective {
                vertical_fov: 55_f32.to_radians(),
                aspect: size.width as f32 / size.height as f32,
                near: 0.1,
                far: self.gravity.as_ref().map_or(100.0, |gravity| {
                    gravity.camera_distance(size.width as f32 / size.height as f32) * 3.0
                }),
            },
        };
        let vp = camera.view_projection()?;
        if let Some(female) = &self.female {
            if let Some((creases, pixels)) = r
                .atlas_update
                .poll(&r.material_creases, &female.face_parameters)?
            {
                r.texture = r.scene.upload_texture_with_sampling(
                    r.host.device(),
                    r.host.queue(),
                    crate::female_complexion::WIDTH,
                    crate::female_complexion::SIZE,
                    &pixels,
                    voxy_render::TextureSampling {
                        min_filter: voxy_render::TextureFilter::Linear,
                        mag_filter: voxy_render::TextureFilter::Linear,
                        ..Default::default()
                    },
                )?;
                r.material_creases = creases;
            }
            let title = if let Some(worker) = &self.full_model_worker {
                format!("Full physics worker | {}", worker.title)
            } else if r.gpu_secondary.is_some() {
                format!(
                    "GPU preview | neutral face / initial irradiance | {}",
                    female.title()
                )
            } else {
                female.title()
            };
            let title = if self.paused {
                format!("Paused | {title}")
            } else {
                format!("Playing | {title}")
            };
            if title != self.female_window_title {
                window.set_title(&title);
                self.female_window_title = title;
            }
            let view_position = female.eye();
            if r.last_view_position != Some(view_position) {
                r.world_transform
                    .update_view_position(r.host.queue(), view_position)?;
                r.last_view_position = Some(view_position);
            }
            let prepared_mesh = if let Some(worker) = &mut self.full_model_worker {
                let result = worker.poll(crate::full_model_worker::Controls::capture(
                    female,
                    self.paused,
                ));
                let result = match result {
                    Ok(frame) => frame,
                    Err(error) => {
                        eprintln!(
                            "Full-model preparation failed; retaining last uploaded pose: {error}"
                        );
                        None
                    }
                };
                if let Some(frame) = result {
                    eprintln!(
                        "FULL MODEL WORKER work_ms={:.3} physics_ms={:.3} mesh_ms={:.3} partition_ms={:.3} streams_ms={:.3} skin_solver_ms={:.3} hair_solver_ms={:.3} simulated_s={:.6}",
                        frame.work_ms,
                        frame.phase_ms[0],
                        frame.phase_ms[1],
                        frame.phase_ms[2],
                        frame.phase_ms[3],
                        frame.solver_ms[0],
                        frame.solver_ms[1],
                        frame.simulation_time
                    );
                    let upload_started = Instant::now();
                    if let Some(gpu) = &mut r.gpu_hair {
                        let frames = frame
                            .hair_frames
                            .as_ref()
                            .ok_or("missing solved GPU hair frames")?;
                        let bytes = gpu.input.replace_frames(frames)?;
                        let uploaded = bytes.len();
                        r.cube.update(r.host.queue(), &frame.mesh)?;
                        r.host.queue().write_buffer(gpu.job.buffer(), 32, bytes);
                        gpu.dirty = true;
                        if let Some(hair) = &mut r.hair_geometry {
                            hair.update_index_partition(
                                r.host.queue(),
                                if frame.hair_visible {
                                    &gpu.indices
                                } else {
                                    &[]
                                },
                            )?;
                        }
                        eprintln!(
                            "FULL MODEL GPU HAIR frame body_vertices={} hair_vertices={} upload_bytes={uploaded} cpu_hair_vertices=0",
                            frame.mesh.vertices().len(),
                            gpu.input.vertex_count()
                        );
                    } else {
                        r.cube
                            .update_shared_vertex_streams(r.host.queue(), &frame.mesh)?;
                        r.cube
                            .update_index_partition(r.host.queue(), &frame.body_indices)?;
                        if let Some(hair) = &mut r.hair_geometry {
                            hair.update_index_partition(r.host.queue(), &frame.hair_indices)?;
                        }
                    }
                    if std::env::var_os("VOXY_PRESENT_PROFILE").is_some()
                        || std::env::var_os("VOXY_FRAME_PROFILE").is_some()
                    {
                        eprintln!(
                            "FULL MODEL CPU UPLOAD stage_ms={:.3}",
                            upload_started.elapsed().as_secs_f64() * 1000.
                        );
                    }
                }
                None
            } else if r.gpu_secondary.is_none() {
                Some(female.mesh()?)
            } else {
                None
            };
            if let Some(mesh) = prepared_mesh {
                if let Some((body, film)) = if female.film.is_some() {
                    mesh.split_material_layer(-2., -6.)?
                } else {
                    None
                } {
                    r.cube.update(r.host.queue(), &body)?;
                    if let Some(layer) = &mut r.film_layer {
                        layer.update(r.host.queue(), &film)?;
                    } else {
                        r.film_layer = Some(r.scene.upload_mesh(r.host.device(), &film)?);
                    }
                } else {
                    r.film_layer = None;
                    r.cube.update(r.host.queue(), &mesh)?;
                }
            }
            if trace {
                eprintln!(
                    "FACE FRAME {} mesh_ready_ms={:.3}",
                    self.frames,
                    frame_started.elapsed().as_secs_f64() * 1000.
                );
            }
        }
        if let Some(fem) = &self.fem {
            r.cube.update(r.host.queue(), &fem.mesh()?)?;
        }
        if let Some(wear) = &self.wear {
            r.cube.update(r.host.queue(), &wear.mesh()?)?;
        }
        if let Some(liquids) = &self.liquids {
            if self.liquid_optics {
                if self.motion_vectors == MotionMode::Enabled {
                    return Err(
                        "screen-space droplets do not yet provide temporal motion vectors".into(),
                    );
                }
                if r.liquid_renderer
                    .as_ref()
                    .is_none_or(|f| f.size() != [size.width, size.height])
                {
                    r.liquid_renderer =
                        Some(voxy_render::ScreenSpaceFluidRenderer::new_with_adapter(
                            r.host.device(),
                            r.host.adapter(),
                            r.host.color_format(),
                            size.width,
                            size.height,
                            8192,
                        )?);
                }
                let (mesh, particles) = liquids.optical_scene()?;
                r.cube.update(r.host.queue(), &mesh)?;
                let films = liquids.optical_film_triangles()?;
                r.liquid_renderer.as_mut().unwrap().update_with_film(
                    r.host.queue(),
                    camera,
                    &particles,
                    &films,
                    1.1,
                    voxy_render::FluidDepthFilter::Bilateral,
                )?;
            } else {
                r.cube.update(r.host.queue(), &liquids.mesh()?)?;
            }
        }
        if let Some(tissues) = &self.tissues {
            r.cube.update(r.host.queue(), &tissues.mesh()?)?;
        }
        if let Some(strands) = &self.strands {
            r.cube.update(r.host.queue(), &strands.mesh()?)?;
        }
        if let Some(gravity) = &self.gravity {
            r.cube.update(r.host.queue(), &gravity.mesh()?)?;
        }
        if let Some(xray) = &self.xray {
            let eye = self
                .graph
                .world_matrix(self.object)?
                .inverse()
                .transform_point3(xray.eye(self.graph.world_matrix(self.object)?));
            r.cube.update(r.host.queue(), &xray.shell(eye)?)?;
            r.cube.set_depth_mode(xray.shell_depth_mode());
            if let Some(internal) = &mut r.internal {
                internal.update(r.host.queue(), &xray.internal_for_eye(eye)?)?;
                internal.set_depth_mode(xray.internal_depth_mode());
            }
        }
        let mvp = vp
            * self.graph.world_matrix(self.object).or_else(|error| {
                if self.rush.is_some() {
                    Ok(Mat4::IDENTITY)
                } else {
                    Err(error)
                }
            })?;
        if self.motion_vectors == MotionMode::Enabled && r.host.temporal_frame().is_none() {
            self.motion.reset();
        }
        let motion = self.motion.prepare(mvp)?;
        if r.last_world_motion != Some(motion) {
            r.world_transform.update_motion(r.host.queue(), motion)?;
            r.last_world_motion = Some(motion);
        }
        if let Some(female) = &self.female {
            let labels = female.diagnostic_legend_labels();
            if labels != r.legend_labels {
                r.overlay
                    .update(r.host.queue(), &crate::diagnostic_legend::mesh(&labels)?)?;
                r.legend_labels = labels;
            }
        }
        let overlay_vp = SceneCamera {
            eye: Vec3::Z,
            target: Vec3::ZERO,
            up: Vec3::Y,
            projection: SceneProjection::Orthographic {
                left: 0.0,
                right: size.width as f32,
                bottom: 0.0,
                top: size.height as f32,
                near: 0.0,
                far: 2.0,
            },
        }
        .view_projection()?;
        let model = Mat4::from_scale_rotation_translation(
            Vec3::new(160.0, 64.0, 1.0),
            Quat::IDENTITY,
            Vec3::new(110.0, size.height as f32 - 60.0, 0.0),
        );
        let overlay_mvp = overlay_vp * model;
        if r.last_overlay_mvp != Some(overlay_mvp) {
            r.overlay_transform.update(r.host.queue(), overlay_mvp)?;
            r.last_overlay_mvp = Some(overlay_mvp);
        }
        let expect_temporal_reset = r.host.temporal_frame().is_none();
        let mut temporal_candidate = None;
        let mut temporal_submitted = None;
        let mut draw_list = Vec::new();
        let draws = [
            SceneDraw {
                geometry: r
                    .gpu_secondary
                    .as_ref()
                    .map_or(&r.cube, |gpu| gpu.geometry()),
                texture: &r.texture,
                transform: &r.world_transform,
                overlay: false,
            },
            SceneDraw {
                geometry: &r.overlay,
                texture: &r.texture,
                transform: &r.overlay_transform,
                overlay: true,
            },
        ];
        let [body_draw, overlay_draw] = draws;
        draw_list.push(body_draw);
        if let Some(hair) = &r.hair_geometry {
            draw_list.push(SceneDraw {
                geometry: hair,
                texture: &r.texture,
                transform: &r.world_transform,
                overlay: false,
            });
        }
        draw_list.push(overlay_draw);
        let draws = draw_list.as_slice();
        let draws = if self
            .female
            .as_ref()
            .is_some_and(|female| !female.diagnostic_legend_visible())
            || self.liquids.is_some()
            || self.wear.is_some()
            || self.fem.is_some()
            || self.xray.is_some()
            || self.strands.is_some()
            || self.gravity.is_some()
            || self.tissues.is_some()
        {
            &draws[..draws.len() - 1]
        } else {
            &draws[..]
        };
        let mut xray_draws = Vec::new();
        let draws = if let Some(internal) = &r.internal {
            xray_draws.push(SceneDraw {
                geometry: &r.cube,
                texture: &r.texture,
                transform: &r.world_transform,
                overlay: false,
            });
            xray_draws.push(SceneDraw {
                geometry: internal,
                texture: &r.texture,
                transform: &r.world_transform,
                overlay: false,
            });
            &xray_draws[..]
        } else {
            draws
        };
        let object_active = self.graph.active_in_hierarchy(self.object).unwrap_or(false);
        if self.rush.is_some() {
            r.script_transforms
                .retain(|id, _| self.graph.local(*id).is_ok());
            for (id, _, _) in self.graph.nodes() {
                if id == self.root || id == self.object {
                    continue;
                }
                let mvp = vp * self.graph.world_matrix(id)?;
                if let Some(transform) = r.script_transforms.get(&id) {
                    transform.update(r.host.queue(), mvp)?;
                } else {
                    r.script_transforms
                        .insert(id, r.scene.create_transform(r.host.device(), mvp)?);
                }
            }
        }
        let mut active_draws: Vec<_> = draws
            .iter()
            .filter(|draw| draw.overlay || object_active)
            .map(|draw| SceneDraw {
                geometry: draw.geometry,
                texture: draw.texture,
                transform: draw.transform,
                overlay: draw.overlay,
            })
            .collect();
        for (id, transform) in &r.script_transforms {
            if self.graph.active_in_hierarchy(*id).unwrap_or(false) {
                let geometry = if self.graph.name(*id)? == "floor" {
                    r.rush_floor.as_ref().unwrap_or(&r.cube)
                } else {
                    r.rush_crate.as_ref().unwrap_or(&r.cube)
                };
                active_draws.push(SceneDraw {
                    geometry,
                    texture: &r.texture,
                    transform,
                    overlay: false,
                });
            }
        }
        if let Some(rush) = &mut self.rush {
            let visible = rush.panel_visible;
            let text = rush.panel_text();
            if visible {
                if r.rush_panel.as_ref().is_none_or(|panel| panel.text != text) {
                    r.rush_panel = Some(crate::rush_diagnostics::DiagnosticPanel::build(
                        &r.scene,
                        &r.host,
                        text.to_owned(),
                    )?);
                }
                if let Some(panel) = &r.rush_panel {
                    active_draws.push(SceneDraw {
                        geometry: &panel.geometry,
                        texture: &panel.texture,
                        transform: &panel.transform,
                        overlay: true,
                    });
                }
            }
        }
        let refractive_layers: Vec<_> = r
            .film_layer
            .iter()
            .filter(|_| object_active)
            .map(|geometry| SceneDraw {
                geometry,
                texture: &r.texture,
                transform: &r.world_transform,
                overlay: false,
            })
            .collect();
        let gpu_controls = self
            .female
            .as_ref()
            .filter(|_| r.gpu_secondary.is_some())
            .map(|female| {
                female
                    .gpu_secondary_controls()
                    .iter()
                    .zip(&r.gpu_secondary_reference)
                    .map(|(p, r)| std::array::from_fn(|k| p[k] - r[k]))
                    .collect::<Vec<[f32; 4]>>()
            });
        let update_gpu = gpu_controls
            .as_ref()
            .is_some_and(|controls| controls != &r.gpu_secondary_last_controls);
        let outcome = if self.liquid_optics && self.liquids.is_some() {
            let fluid = r
                .liquid_renderer
                .as_ref()
                .ok_or("missing liquid renderer")?;
            r.host.render_custom(
                |encoder, target| -> Result<(), voxy_render::RendererError> {
                    fluid.encode(&r.scene, encoder, target, Default::default(), &active_draws);
                    Ok(())
                },
            )?
        } else {
            r.host.render_scene_with_preparation_and_temporal_hooks(
                &r.scene,
                &active_draws,
                &refractive_layers,
                |queue, encoder| {
                    if let Some(hair) = &mut r.gpu_hair {
                        if hair.dirty {
                            hair.job
                                .encode_step(
                                    encoder,
                                    [hair.input.vertex_count().div_ceil(64), 1, 1],
                                )
                                .map_err(|_| {
                                    voxy_render::RendererError::Scene(
                                        voxy_render::SceneError::InvalidGeometry,
                                    )
                                })?;
                            hair.transfer.encode(encoder);
                            hair.dirty = false;
                        }
                    }
                    if update_gpu
                        && let (Some(gpu), Some(controls)) = (&r.gpu_secondary, &gpu_controls)
                    {
                        gpu.upload_controls(queue, controls)
                            .map_err(voxy_render::RendererError::Scene)?;
                        gpu.encode(encoder)
                            .map_err(voxy_render::RendererError::Scene)?;
                    }
                    Ok(())
                },
                |inputs, _encoder| {
                    temporal_candidate = Some((inputs.presentation_id, inputs.reset_history));
                },
                |inputs, _device, _queue, _target| {
                    temporal_submitted = Some((inputs.presentation_id, inputs.reset_history));
                    Ok(())
                },
            )?
        };
        if outcome == RenderOutcome::Presented
            && let Some(controls) = gpu_controls
        {
            r.gpu_secondary_last_controls = controls;
        }
        if matches!(
            outcome,
            RenderOutcome::SkippedOccluded | RenderOutcome::SkippedTimeout
        ) {
            self.defer_redraw_until = Some(Instant::now() + std::time::Duration::from_millis(16));
        } else if self.female.is_some()
            && r.gpu_secondary.is_none()
            && self.full_model_worker.is_none()
        {
            // Let native input/accessibility run between expensive full-model frames.
            self.defer_redraw_until = Some(Instant::now() + std::time::Duration::from_millis(16));
        } else {
            self.defer_redraw_until = None;
        }
        if trace {
            eprintln!(
                "FACE FRAME {} outcome={outcome:?} elapsed_ms={:.3}",
                self.frames,
                frame_started.elapsed().as_secs_f64() * 1000.
            );
        }
        if outcome == RenderOutcome::Presented {
            if !refractive_layers.is_empty() || (self.liquid_optics && self.liquids.is_some()) {
                r.optical_frames += 1;
            }
            if self.smoke
                && let Some(film) = self.female.as_ref().and_then(|f| f.film.as_ref())
            {
                r.film_mass_max_relative_error = r
                    .film_mass_max_relative_error
                    .max(film.verify_mass_balance(1e-9)?);
                r.film_mass_checks += 1;
            }
            self.recovering_gpu = false;
            if object_active {
                if let Some(female) = &mut self.female {
                    female.record_presentation(u64::from(self.frames), [size.width, size.height]);
                }
            }
            if self.motion_vectors == MotionMode::Enabled {
                let (target, frame_id) = r
                    .host
                    .motion_frame()
                    .ok_or("presented motion frame is missing")?;
                if target.width() != size.width || target.height() != size.height {
                    return Err("motion dimensions do not match the presented surface".into());
                }
                if frame_id != r.motion_frames + 1 {
                    return Err("motion frame ID does not match the presented frame".into());
                }
                let temporal = r
                    .host
                    .temporal_frame()
                    .ok_or("presented temporal inputs are missing")?;
                if temporal.presentation_id != frame_id || temporal.depth.size() != target.size() {
                    return Err("depth and motion inputs are not frame aligned".into());
                }
                if temporal_candidate != Some((frame_id, temporal.reset_history)) {
                    return Err("pre-present inputs differ from published temporal inputs".into());
                }
                if temporal_submitted != temporal_candidate {
                    return Err("submitted inputs differ from prepared temporal inputs".into());
                }
                if temporal.reset_history != expect_temporal_reset {
                    return Err("temporal reset does not match history invalidation".into());
                }
                r.motion_frames = frame_id;
            }
            self.motion.presented(mvp)?;
            if std::env::var_os("VOXY_FRAME_PROFILE").is_some()
                || std::env::var_os("VOXY_PRESENT_PROFILE").is_some()
            {
                let presented = Instant::now();
                if let Some(previous) = self.profile_last_present {
                    eprintln!(
                        "PRESENT CADENCE frame={} interval_ms={:.6} paused={}",
                        self.frames,
                        presented.duration_since(previous).as_secs_f64() * 1000.,
                        self.paused
                    );
                }
                self.profile_last_present = Some(presented);
            }
            self.frames += 1;
        }
        if self.smoke {
            if self.frames == 30 {
                let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(720, 540));
            }
            if self.frames == 60 {
                let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(900, 650));
            }
            if self.frames >= 120
                && self
                    .gravity
                    .as_ref()
                    .is_none_or(|g| self.gravity_smoke_stage == 2 && g.planet_verified())
            {
                if let Some(gravity) = &self.gravity {
                    if gravity.contacts == 0 {
                        return Err("gravity smoke did not observe a collision".into());
                    }
                    println!(
                        "GRAVITY SMOKE PASS: {} steps, {} contacts; inverted walking, jump and landing verified",
                        gravity.steps, gravity.contacts
                    );
                }
                if let Some(fem) = &self.fem {
                    fem.verify()?;
                    println!(
                        "FEM SMOKE PASS: {} accepted steps; mass, momenta, fracture energy and work budgets verified",
                        fem.steps
                    );
                }
                if let Some(wear) = &self.wear {
                    wear.verify()?;
                    println!(
                        "WEAR SMOKE PASS: {} steps, voxel geometry/collision and grain mass verified",
                        wear.steps
                    );
                }
                if let Some(liquids) = &self.liquids {
                    liquids.verify()?;
                    if self.liquid_optics {
                        if r.optical_frames != u64::from(self.frames) {
                            return Err("liquid optical passes missing from presented frame".into());
                        }
                        println!(
                            "LIQUID OPTICAL SMOKE PASS: {} presented frames",
                            r.optical_frames
                        );
                    }
                    println!(
                        "LIQUID SMOKE PASS: {} fixed steps, both reservoirs conserve mass and spread",
                        liquids.steps
                    );
                }
                if let Some(female) = &self.female {
                    female.verify()?;
                    if female.film.is_some() {
                        if r.optical_frames != u64::from(self.frames) {
                            return Err(
                                "film layers were absent from a presented smoke frame".into()
                            );
                        }
                        if r.film_mass_checks != u64::from(self.frames) {
                            return Err("film mass checks missed a presented smoke frame".into());
                        }
                        println!(
                            "FILM MASS SMOKE PASS: {} checks, max source-adjusted relative error={:.9e}",
                            r.film_mass_checks, r.film_mass_max_relative_error
                        );
                        println!(
                            "FILM OPTICAL SMOKE PASS: {} presented layers, temporal={}, resize_stages={}",
                            r.optical_frames,
                            self.motion_vectors == MotionMode::Enabled,
                            self.resize_stages
                        );
                    }
                    println!(
                        "FEMALE SKIN SMOKE PASS: {} nonlinear steps with rendered surface deformation",
                        female.steps
                    );
                }
                if self.resize_stages != 3 {
                    return Err("requested resize events were not observed".into());
                }
                if self.rush_smoke {
                    let rush = self.rush.as_mut().ok_or("missing Rush smoke runtime")?;
                    rush.verify_smoke(&self.graph)
                        .map_err(std::io::Error::other)?;
                    if r.script_transforms.len() != 2 {
                        return Err("spawned Rush cube was not submitted for rendering".into());
                    }
                    println!(
                        "RUSH SMOKE PASS: player moved, one cube rendered, reward event changed scale, real physics contacts delivered"
                    );
                }
                println!("SCENE SMOKE PASS: {} presented frames", self.frames);
                self.done = true;
                event_loop.exit();
            } else if self.started.elapsed().as_secs()
                > if self.xray.is_some()
                    || self.female.is_some()
                    || self.strands.is_some()
                    || self.gravity.is_some()
                {
                    60
                } else {
                    20
                }
            {
                return Err(format!(
                    "scene smoke timed out: frames={}, resize_stages={}, outcome={outcome:?}",
                    self.frames, self.resize_stages
                )
                .into());
            }
        }
        Ok(())
    }
    fn recreate_gpu(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.recovering_gpu = true;
        self.resources = None;
        self.motion.reset();
        self.frame_loop.reset();
        self.last = Instant::now();
        self.initialize(event_loop)?;
        eprintln!("SCENE GPU RECOVERED: resources recreated, history reset");
        Ok(())
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: &dyn std::fmt::Display) {
        self.failure = Some(error.to_string());
        eprintln!("SCENE ERROR: {error}");
        event_loop.exit();
    }
}
/// Push `title` to the windowing system only when it differs from the last one sent.
fn set_title_if_changed(window: &Window, last_title: &mut String, title: String) {
    if *last_title != title {
        window.set_title(&title);
        *last_title = title;
    }
}
impl ApplicationHandler for SceneApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.frame_loop.reset();
        self.last = Instant::now();
        self.input.focused(true);
        if let Err(error) = self.initialize(event_loop) {
            self.fail(event_loop, &error);
        }
    }
    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.frame_loop.reset();
        self.input.focused(false);
        self.resources = None;
        self.motion.reset();
        self.touch.reset();
        self.last = Instant::now();
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let WindowEvent::Focused(focused) = &event {
            self.input.focused(*focused);
            if let Some(rush) = &mut self.rush {
                rush.focused(*focused);
            }
        }
        if let WindowEvent::KeyboardInput { event, .. } = &event
            && let PhysicalKey::Code(key) = event.physical_key
        {
            if let Some(rush) = &mut self.rush {
                rush.keyboard(key, event.state == ElementState::Pressed);
            }
            let actions = self
                .input
                .keyboard(key, event.state == ElementState::Pressed);
            if actions.pause {
                self.paused = !self.paused;
                self.frame_loop.set_paused(self.paused);
                self.last = Instant::now();
            }
            if actions.toggle_world {
                if let Ok(active) = self.graph.active_self(self.root) {
                    let _ = self.graph.set_active(self.root, !active);
                    self.motion.reset();
                }
            }
        }
        if self.female.is_some()
            && let WindowEvent::KeyboardInput { event, .. } = &event
        {
            let index = match event.physical_key {
                PhysicalKey::Code(KeyCode::ArrowLeft) => Some(0),
                PhysicalKey::Code(KeyCode::ArrowRight) => Some(1),
                PhysicalKey::Code(KeyCode::ArrowUp) => Some(2),
                PhysicalKey::Code(KeyCode::ArrowDown) => Some(3),
                _ => None,
            };
            if let Some(index) = index {
                self.camera_motion.held[index] = event.state == ElementState::Pressed;
            }
        }
        match event {
            WindowEvent::CloseRequested => {
                if self.smoke {
                    eprintln!("SCENE SMOKE EXIT: close requested at frame {}", self.frames);
                }
                event_loop.exit();
            }
            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                self.frame_loop.reset();
                self.last = Instant::now();
                if occluded {
                    event_loop.set_control_flow(ControlFlow::Wait);
                } else {
                    self.motion.reset();
                    if let Some(resources) = &mut self.resources {
                        resources.host.invalidate_temporal_history();
                    }
                    event_loop.set_control_flow(ControlFlow::Poll);
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::Touch(touch) => {
                if let Some(window) = &self.window {
                    let size = window.inner_size();
                    if size.width != 0 && size.height != 0 {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                        let position = glam::Vec2::new(
                            touch.location.x as f32 / size.width as f32,
                            touch.location.y as f32 / size.height as f32,
                        );
                        match self.touch.update(touch.id, touch.phase, position) {
                            crate::touch::TouchAction::Rotate(delta) => self.angle += delta,
                            crate::touch::TouchAction::TogglePause => self.paused = !self.paused,
                            crate::touch::TouchAction::None => {}
                        }
                    }
                }
            }
            WindowEvent::MouseInput {
                state,
                button: winit::event::MouseButton::Left,
                ..
            } if self.female.is_some() => {
                self.orbit_drag = state == ElementState::Pressed;
            }
            WindowEvent::CursorMoved { position, .. } if self.female.is_some() => {
                if self.orbit_drag
                    && let Some((x, y)) = self.cursor
                    && let Some(female) = &mut self.female
                {
                    self.camera_motion.orbit(
                        [female.yaw, female.pitch, female.distance],
                        (position.x - x) as f32 * 0.007,
                        (position.y - y) as f32 * 0.007,
                    );
                }
                self.cursor = Some((position.x, position.y));
            }
            WindowEvent::MouseWheel { delta, .. } if self.female.is_some() => {
                if let Some(female) = &mut self.female {
                    let amount = match delta {
                        winit::event::MouseScrollDelta::LineDelta(_, y) => -y * 0.1,
                        winit::event::MouseScrollDelta::PixelDelta(p) => -p.y as f32 * 0.003,
                    };
                    self.camera_motion.zoom(
                        [female.yaw, female.pitch, female.distance],
                        amount,
                        female.hand_focus,
                    );
                }
            }
            WindowEvent::Focused(false) => {
                self.touch.reset();
                self.orbit_drag = false;
                self.camera_motion.held = [false; 4];
                self.cursor = None;
            }
            WindowEvent::Resized(size) => {
                self.motion.reset();
                if self.smoke && self.frames >= 30 && size.width == 720 && size.height == 540 {
                    self.resize_stages |= 1;
                }
                if self.smoke && self.frames >= 60 && size.width == 900 && size.height == 650 {
                    self.resize_stages |= 2;
                }
                if let Some(r) = &mut self.resources
                    && let Err(error) = r.host.resize(size.width, size.height)
                {
                    let recoverable = matches!(
                        &error,
                        voxy_render::RendererError::DeviceLost(_)
                            | voxy_render::RendererError::SurfaceLost
                    );
                    if recoverable && !self.recovering_gpu {
                        if let Err(error) = self.recreate_gpu(event_loop) {
                            self.fail(event_loop, &error);
                        }
                    } else {
                        self.fail(event_loop, &error);
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Escape) => {
                        if self.smoke {
                            eprintln!("SCENE SMOKE EXIT: Escape at frame {}", self.frames);
                        }
                        event_loop.exit();
                    }
                    PhysicalKey::Code(KeyCode::Digit1 | KeyCode::Digit2 | KeyCode::Digit3)
                        if self.female.as_ref().is_some_and(|f| f.hand_focus) =>
                    {
                        if let Some(female) = &mut self.female {
                            let shape = match event.physical_key {
                                PhysicalKey::Code(KeyCode::Digit1) => "cylinder",
                                PhysicalKey::Code(KeyCode::Digit2) => "sphere",
                                _ => "handle",
                            };
                            if let Err(error) = female.set_grasp_target(shape) {
                                eprintln!("Grasp target rejected: {error}");
                            }
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyB) if self.female.is_some() => {
                        if let Some(female) = &mut self.female {
                            female.show_complexion = !female.show_complexion;
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyP) if self.female.is_some() => {
                        if let Some(female) = &mut self.female {
                            female.pressing = !female.pressing;
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyM) if self.female.is_some() => {
                        if let Some(female) = &mut self.female {
                            female.show_skin = !female.show_skin;
                            female.show_strain = false;
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyK) if self.female.is_some() => {
                        if let Some(female) = &mut self.female {
                            female.show_strain = !female.show_strain;
                            female.show_skin = false;
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyC) if self.female.is_some() => {
                        if let Some(female) = &mut self.female {
                            female.probe_enabled = !female.probe_enabled;
                        }
                    }
                    PhysicalKey::Code(KeyCode::BracketLeft | KeyCode::BracketRight)
                        if self.female.as_ref().is_some_and(|female| female.hand_focus) =>
                    {
                        let delta =
                            if event.physical_key == PhysicalKey::Code(KeyCode::BracketRight) {
                                0.1
                            } else {
                                -0.1
                            };
                        if let Some(female) = &mut self.female {
                            if let Err(error) = female.adjust_grasp(delta) {
                                self.fail(event_loop, &error);
                                return;
                            }
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyG)
                        if self.female.as_ref().is_some_and(|female| female.hand_focus) =>
                    {
                        if let Some(female) = &mut self.female {
                            female.set_grasp_cycle(true);
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyF) if self.xray.is_some() => {
                        if let Some(xray) = &mut self.xray {
                            xray.closeup = !xray.closeup;
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyL) => {
                        if let Some(xray) = &mut self.xray {
                            xray.local = !xray.local;
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyX) => {
                        if let Some(xray) = &mut self.xray {
                            xray.enabled = !xray.enabled;
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyS) => {
                        if let Some(xray) = &mut self.xray {
                            xray.slow = !xray.slow;
                        }
                    }
                    PhysicalKey::Code(KeyCode::ArrowLeft) if self.xray.is_some() => {
                        self.angle -= 0.15
                    }
                    PhysicalKey::Code(KeyCode::ArrowRight) if self.xray.is_some() => {
                        self.angle += 0.15
                    }
                    PhysicalKey::Code(KeyCode::Digit1) => {
                        if let Some(gravity) = &mut self.gravity {
                            gravity.reset(false);
                        }
                    }
                    PhysicalKey::Code(KeyCode::Digit2) => {
                        if let Some(gravity) = &mut self.gravity {
                            gravity.reset(true);
                        }
                    }
                    PhysicalKey::Code(KeyCode::Digit3) => {
                        if let Some(gravity) = &mut self.gravity {
                            gravity.planet();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyJ) => {
                        if let Some(gravity) = &mut self.gravity {
                            gravity.jump();
                        }
                    }
                    PhysicalKey::Code(KeyCode::KeyR) => {
                        if self.fem.is_some() {
                            match crate::fem_demo::FemDemo::new() {
                                Ok(demo) => self.fem = Some(demo),
                                Err(error) => {
                                    self.fail(event_loop, &error);
                                    return;
                                }
                            }
                        }
                        if self.wear.is_some() {
                            match crate::wear_demo::WearDemo::new() {
                                Ok(demo) => self.wear = Some(demo),
                                Err(error) => {
                                    self.fail(event_loop, &error);
                                    return;
                                }
                            }
                        }
                        if let Some(liquids) = &self.liquids {
                            match liquids.restart() {
                                Ok(demo) => self.liquids = Some(demo),
                                Err(error) => {
                                    self.fail(event_loop, &error);
                                    return;
                                }
                            }
                        }
                        if let Some(gravity) = &mut self.gravity {
                            gravity.restart();
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested if !self.occluded => {
                if self.smoke
                    && self.frames == 30
                    && !self.injected_device_loss
                    && std::env::var_os("VOXY_SCENE_DEVICE_LOSS_SMOKE").is_some()
                    && let Some(resources) = &self.resources
                {
                    self.injected_device_loss = true;
                    resources.host.device().destroy();
                    let deadline = Instant::now() + std::time::Duration::from_secs(2);
                    while resources.host.device_failure().is_none() && Instant::now() < deadline {
                        let _ = resources.host.poll_device();
                        std::thread::yield_now();
                    }
                    if resources.host.device_failure().is_none() {
                        self.fail(event_loop, &"device-loss callback did not arrive");
                        return;
                    }
                }
                if let Err(error) = self.draw(event_loop) {
                    let recoverable = matches!(
                        error.downcast_ref::<voxy_render::RendererError>(),
                        Some(
                            voxy_render::RendererError::DeviceLost(_)
                                | voxy_render::RendererError::SurfaceLost
                        )
                    );
                    if recoverable && !self.recovering_gpu {
                        if let Err(error) = self.recreate_gpu(event_loop) {
                            self.fail(event_loop, &error);
                        }
                    } else {
                        self.fail(event_loop, &error);
                    }
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self.defer_redraw_until {
            if Instant::now() < deadline {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                return;
            }
            self.defer_redraw_until = None;
            event_loop.set_control_flow(ControlFlow::Poll);
        }
        if !self.occluded
            && self.resources.is_some()
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
    }
}
fn rush_box_mesh(color: [f32; 4]) -> Result<SceneMesh, voxy_render::SceneError> {
    let cube = cube_mesh()?;
    let vertices = cube
        .vertices()
        .iter()
        .map(|vertex| {
            let mut v = *vertex;
            let shade = 0.65
                + if v.position[1] > 0.0 { 0.25 } else { 0.0 }
                + if v.position[0] < 0.0 { 0.1 } else { 0.0 };
            v.position = v.position.map(|coordinate| coordinate * 0.5);
            v.color = [
                color[0] * shade,
                color[1] * shade,
                color[2] * shade,
                color[3],
            ];
            v
        })
        .collect();
    SceneMesh::new(vertices, cube.indices().to_vec())
}

fn cube_mesh() -> Result<SceneMesh, voxy_render::SceneError> {
    let positions = [
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    let vertices = positions
        .into_iter()
        .enumerate()
        .map(|(i, position)| SceneVertex {
            position,
            uv: [
                if i % 2 == 0 { 0.0 } else { 1.0 },
                if i % 4 < 2 { 1.0 } else { 0.0 },
            ],
            color: [1.0; 4],
        })
        .collect();
    SceneMesh::new(
        vertices,
        vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 3, 7, 6, 3, 6, 2, 0, 4, 7, 0, 7,
            3, 1, 2, 6, 1, 6, 5,
        ],
    )
}
