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
/// Optional asynchronous tail audit; absent from the normal rendering path.
#[derive(Debug)]
struct SurfaceLightAudit {
    buffer: wgpu::Buffer,
    pending: Option<std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>>,
}
impl SurfaceLightAudit {
    fn poll(&mut self, device: &wgpu::Device) -> Result<(), Box<dyn std::error::Error>> {
        device.poll(wgpu::PollType::Poll)?;
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(result) => {
                    result?;
                    let data = self.buffer.slice(..).get_mapped_range()?;
                    let h: [u32; 8] = std::array::from_fn(|i| {
                        u32::from_le_bytes(
                            data[i * 4..i * 4 + 4]
                                .try_into()
                                .expect("four header bytes"),
                        )
                    });
                    eprintln!(
                        "GPU SURFACE LIGHT AUDIT geometry={} admitted={} has_accepted={} generation={} error_estimate={:.8} relative_squared={:.8e}",
                        h[0],
                        h[1],
                        h[2],
                        h[3],
                        f32::from_bits(h[4]),
                        f32::from_bits(h[5])
                    );
                    drop(data);
                    self.buffer.unmap();
                    self.pending = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
    fn begin(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        self.pending = Some(rx);
    }
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
    gpu_body: Option<voxy_demos::female_demo::resident_body::ResidentBody>,
    gpu_surface_light: Option<voxy_render::SurfaceLightFrame>,
    gpu_surface_colors: Option<voxy_render::SurfaceColorTransfer>,
    gpu_surface_light_audit: Option<SurfaceLightAudit>,
    gpu_secondary_reference: Vec<[f32; 4]>,
    gpu_secondary_last_controls: Vec<[f32; 4]>,
    gpu_jump_last_palette: Vec<Mat4>,
    film_layer: Option<SceneGeometry>,
    optical_frames: u64,
    film_mass_checks: u64,
    film_mass_max_relative_error: f64,
    overlay: SceneGeometry,
    legend_labels: Vec<[&'static str; 3]>,
    internal: Option<SceneGeometry>,
    texture: SceneTexture,
    material_creases: Vec<f32>,
    atlas_update: voxy_demos::female_complexion::AtlasUpdate,
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
    female: Option<voxy_demos::female_demo::FemaleDemo>,
    orbit_drag: bool,
    camera_motion: crate::camera_motion::CameraMotion,
    full_model_worker: Option<voxy_demos::full_model_worker::FullModelWorker>,
    hair_solver: Option<voxy_demos::gpu_hair_solver::GpuHairLinearSolver>,
    cursor: Option<(f64, f64)>,
    liquids: Option<voxy_demos::liquid_demo::LiquidDemo>,
    liquid_optics: bool,
    wear: Option<voxy_demos::wear_demo::WearDemo>,
    fem: Option<voxy_demos::fem_demo::FemDemo>,
    gravity: Option<voxy_demos::gravity_demo::GravityDemo>,
    strands: Option<voxy_demos::strands_demo::StrandsDemo>,
    tissues: Option<voxy_demos::tissue_demo::TissueDemo>,
    xray: Option<voxy_demos::xray_demo::XrayDemo>,
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
            hair_solver: None,
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
        self.liquids = Some(voxy_demos::liquid_demo::LiquidDemo::new()?);
        self.gravity = None;
        self.xray = None;
        self.tissues = None;
        self.strands = None;
        Ok(self)
    }
    /// Shows finite point sources with mass depletion and energy-accounted recoil.
    pub fn with_finite_liquid_sources(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self = self.with_liquids()?;
        self.liquids = Some(voxy_demos::liquid_demo::LiquidDemo::new_finite_sources()?);
        self.liquid_optics = false;
        Ok(self)
    }
    /// Shows jets, impact spray and surface-film deposition for water and oil.
    pub fn with_liquid_impacts(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self = self.with_liquids()?;
        self.liquids = Some(voxy_demos::liquid_demo::LiquidDemo::new_impacts()?);
        self.liquid_optics = true;
        Ok(self)
    }
    /// Finite energy/mass sources with recoil, impact spray and deposited film.
    pub fn with_finite_liquid_impacts(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self = self.with_liquids()?;
        self.liquids = Some(voxy_demos::liquid_demo::LiquidDemo::new_finite_impacts()?);
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
        self.wear = Some(voxy_demos::wear_demo::WearDemo::new()?);
        Ok(self)
    }
    /// Dynamic cohesive fracture with current FEM geometry uploaded each frame.
    pub fn with_fem_fracture(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self = self.with_liquids()?;
        self.liquids = None;
        self.wear = None;
        self.fem = Some(voxy_demos::fem_demo::FemDemo::new()?);
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
        self.strands = Some(voxy_demos::strands_demo::StrandsDemo::new());
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
        self.tissues = Some(voxy_demos::tissue_demo::TissueDemo::new());
        self
    }
    /// Shows load-controlled finite-element biomechanics specimens.
    /// # Errors
    /// Returns invalid material or mesh construction errors.
    pub fn with_biomechanics(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        self = self.with_tissues();
        self.tissues = Some(voxy_demos::tissue_demo::TissueDemo::biomechanics()?);
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
        self.tissues = Some(voxy_demos::tissue_demo::TissueDemo::body());
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
        self.gravity = Some(voxy_demos::gravity_demo::GravityDemo::new(collision));
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
        self.xray = Some(voxy_demos::xray_demo::XrayDemo::new());
        self
    }
    /// Uses the imported Blender body as the X-ray shell, with illustrative internals.
    /// # Errors
    /// Rejects invalid OBJ geometry or asset budgets.
    pub fn with_xray_model(self) -> Result<Self, Box<dyn std::error::Error>> {
        let mut app = self.with_xray();
        app.xray = Some(voxy_demos::xray_demo::XrayDemo::with_model()?);
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
        self.female = Some(voxy_demos::female_demo::FemaleDemo::new()?);
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
    /// Shows fixed stance with accepted character physics and geometric foot IK.
    /// Hair and nonlinear tissue simulation are not advanced by this diagnostic.
    pub fn with_female_physical_stance(self) -> Result<Self, Box<dyn std::error::Error>> {
        let mut app = self.with_female_animation()?;
        if let Some(female) = &mut app.female {
            female.enable_physical_stance()?;
        }
        Ok(app)
    }
    /// Shows an authored grounded gait in place on the actual character rig.
    /// This preview does not advance the nonlinear physical shell solver.
    /// # Errors
    /// Rejects invalid assets and clips outside idle/walk/jog/sprint.
    pub fn with_female_locomotion(
        self,
        clip: voxy_demos::humanoid_locomotion::LocomotionClip,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut app = self.with_female_animation()?;
        if let Some(female) = &mut app.female {
            female.enable_authored_gait(clip)?;
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
            if std::env::var("VOXY_AUTHORED_JUMP").as_deref() == Ok("1") {
                female.enable_authored_jump()?;
            }
            female.yaw = 1.1;
            female.distance = 2.1;
            female.pressing = false;
        }
        Ok(app)
    }
    /// Sets continuous body proportions for the imported character.
    pub fn with_body_parameters(
        mut self,
        parameters: voxy_demos::body_parameters::BodyParameters,
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
        let parameters = voxy_demos::body_parameters::BodyParameters::from_json(
            &std::fs::read_to_string(&path)?,
        )?;
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
        let parameters = voxy_demos::face_parameters::FaceParameters::from_json(
            &std::fs::read_to_string(&path)?,
        )?;
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
        self.rush = Some(crate::scene_rush::SceneRush::new(
            &mut self.graph,
            self.object,
            path,
            selected,
        )?);
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
            self.advance_active_demo(elapsed, dt)?;
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
                rush.begin_frame(&mut self.graph)?;
            }
            let fixed_seconds = self.frame_loop.fixed_seconds();
            for _ in 0..frame_work.fixed.steps {
                self.behaviors.fixed_update(&mut self.graph, fixed_seconds);
                if let Some(rush) = &mut self.rush {
                    rush.fixed_update(&mut self.graph, fixed_seconds)?;
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
        let camera = self.build_scene_camera(size)?;
        let vp = camera.view_projection()?;
        let Some(r) = &mut self.resources else {
            return Ok(());
        };
        if let Some(female) = &mut self.female {
            female.resident_surface_light =
                r.gpu_surface_light.as_ref().is_some_and(|light| {
                    female.can_use_resident_surface_light(light.surface_fraction())
                }) && std::env::var_os("VOXY_GPU_SURFACE_LIGHT_CPU_REFERENCE").is_none();
        }
        if let Some(audit) = &mut r.gpu_surface_light_audit {
            audit.poll(r.host.device())?;
        }
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
        if let Some(female) = &self.female {
            if let Some((creases, pixels)) = r
                .atlas_update
                .poll(&r.material_creases, &female.face_parameters)?
            {
                r.texture = r.scene.upload_texture_with_sampling(
                    r.host.device(),
                    r.host.queue(),
                    voxy_demos::female_complexion::WIDTH,
                    voxy_demos::female_complexion::SIZE,
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
                let result = worker.poll(voxy_demos::full_model_worker::Controls::capture(
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
                        let offset = gpu.input.transform_byte_offset();
                        let pose = gpu.input.replace_transform(frame.hair_transform)?;
                        r.host.queue().write_buffer(gpu.job.buffer(), offset, pose);
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
            } else if let Some(gpu) = &mut r.gpu_hair {
                let frames = female.gpu_hair_surface_frames();
                let bytes = gpu.input.replace_frames(&frames)?;
                r.host.queue().write_buffer(gpu.job.buffer(), 32, bytes);
                let offset = gpu.input.transform_byte_offset();
                let pose = gpu
                    .input
                    .replace_transform(female.gpu_hair_surface_transform())?;
                r.host.queue().write_buffer(gpu.job.buffer(), offset, pose);
                gpu.dirty = true;
                if let Some(hair) = &mut r.hair_geometry {
                    hair.update_index_partition(
                        r.host.queue(),
                        if female.show_hair { &gpu.indices } else { &[] },
                    )?;
                }
                Some(female.mesh_without_hair()?)
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
                    if let Some(body) = &mut r.gpu_body {
                        r.cube.update_after_deforming_prefix(
                            r.host.queue(),
                            &mesh,
                            body.base_vertices(),
                        )?;
                        body.refresh(r.host.queue(), female, &mesh)?;
                    } else {
                        r.cube.update(r.host.queue(), &mesh)?;
                    }
                }
            }
            if let Some(light) = &r.gpu_surface_light {
                light.upload_reference_normals(r.host.queue(), &female.surface_light_normals())?;
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
                let mesh = liquids.optical_background()?;
                let (particle_count, particles) = liquids.optical_particles()?;
                r.cube.update(r.host.queue(), &mesh)?;
                let films = liquids.optical_film_triangles()?;
                r.liquid_renderer
                    .as_mut()
                    .unwrap()
                    .update_generated_with_film(
                        r.host.queue(),
                        camera,
                        particle_count,
                        particles,
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
                r.overlay.update(
                    r.host.queue(),
                    &voxy_demos::diagnostic_legend::mesh(&labels)?,
                )?;
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
        let mut surface_light_audit_encoded = false;
        let mut temporal_candidate = None;
        let mut temporal_submitted = None;
        let mut draw_list = Vec::with_capacity(3);
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
            draws
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
        let gpu_jump_palette = self
            .female
            .as_ref()
            .filter(|_| r.gpu_secondary.is_some())
            .map(voxy_demos::female_demo::FemaleDemo::gpu_jump_palette);
        let update_gpu = gpu_controls
            .as_ref()
            .is_some_and(|controls| controls != &r.gpu_secondary_last_controls)
            || gpu_jump_palette
                .as_ref()
                .is_some_and(|palette| palette != &r.gpu_jump_last_palette);
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
                    if let Some(body) = &mut r.gpu_body {
                        body.encode(encoder)
                            .map_err(voxy_render::RendererError::Scene)?;
                    }
                    if let Some(hair) = &mut r.gpu_hair
                        && hair.dirty
                    {
                        hair.job
                            .encode_step(encoder, [hair.input.vertex_count().div_ceil(64), 1, 1])
                            .map_err(|_| {
                                voxy_render::RendererError::Scene(
                                    voxy_render::SceneError::InvalidGeometry,
                                )
                            })?;
                        hair.transfer.encode(encoder);
                        hair.dirty = false;
                    }
                    if update_gpu
                        && let (Some(gpu), Some(controls)) = (&r.gpu_secondary, &gpu_controls)
                    {
                        if let Some(palette) = &gpu_jump_palette {
                            gpu.upload_rigid_pose(queue, palette)
                                .map_err(voxy_render::RendererError::Scene)?;
                        }
                        gpu.upload_controls(queue, controls)
                            .map_err(voxy_render::RendererError::Scene)?;
                        gpu.encode(encoder)
                            .map_err(voxy_render::RendererError::Scene)?;
                    }
                    if let Some(light) = &mut r.gpu_surface_light
                        && self.female.as_ref().is_some_and(|f| {
                            f.can_use_resident_surface_light(light.surface_fraction())
                        })
                    {
                        light
                            .encode(encoder, 64)
                            .map_err(voxy_render::RendererError::Scene)?;
                        if let Some(colors) = &r.gpu_surface_colors {
                            colors.encode(encoder);
                        }
                        if let Some(audit) = &r.gpu_surface_light_audit
                            && audit.pending.is_none()
                        {
                            encoder.copy_buffer_to_buffer(
                                light.admission().header_buffer(),
                                0,
                                &audit.buffer,
                                0,
                                32,
                            );
                            surface_light_audit_encoded = true;
                        }
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
            if let Some(palette) = gpu_jump_palette {
                r.gpu_jump_last_palette = palette;
            }
        }
        if matches!(
            outcome,
            RenderOutcome::SkippedOccluded | RenderOutcome::SkippedTimeout
        ) {
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
            if surface_light_audit_encoded && let Some(audit) = &mut r.gpu_surface_light_audit {
                audit.begin();
            }
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
            if object_active && let Some(female) = &mut self.female {
                female.record_presentation(u64::from(self.frames), [size.width, size.height]);
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
            self.verify_smoke_completion(outcome, event_loop)?;
        }
        Ok(())
    }
}
/// Push `title` to the windowing system only when it differs from the last one sent.
fn set_title_if_changed(window: &Window, last_title: &mut String, title: String) {
    if *last_title != title {
        window.set_title(&title);
        *last_title = title;
    }
}

mod events;
mod init;
