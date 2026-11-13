//! The imported character with its skin, hair and surface-light GPU paths,
//! presented by the scene shell.
mod camera_motion;
mod frame;
mod gpu;

use crate::body_parameters::BodyParameters;
use crate::face_parameters::FaceParameters;
use crate::female_demo::FemaleDemo;
use crate::full_model_worker::FullModelWorker;
use crate::gpu_hair_solver::GpuHairLinearSolver;
use crate::humanoid_locomotion::LocomotionClip;
use camera_motion::CameraMotion;
use gpu::FemaleGpu;
use voxy_render::{SceneCamera, SceneTexture};
use voxy_scene_host::{
    DemoEnv, DemoFrame, DemoGpu, DemoInput, DemoKey, DemoReply, DemoResult, DemoScroll, DemoTick,
    DemoView, FrameParts, HostRequest, Presented, SceneDemo, SharedDraw, SmokeReport,
    perspective_camera,
};

#[derive(Debug)]
pub struct FemaleScene {
    female: FemaleDemo,
    camera_motion: CameraMotion,
    orbit_drag: bool,
    cursor: Option<(f64, f64)>,
    full_model_worker: Option<FullModelWorker>,
    hair_solver: Option<GpuHairLinearSolver>,
    smoke: bool,
    use_gpu_secondary: bool,
    gpu: Option<FemaleGpu>,
}

impl FemaleScene {
    /// Blender Studio's CC0 anatomical female mesh and its nonlinear skin patch.
    /// # Errors
    /// Rejects invalid imported geometry, skin topology or render bindings.
    pub fn new() -> DemoResult<Self> {
        Ok(Self {
            female: FemaleDemo::new()?,
            camera_motion: CameraMotion::default(),
            orbit_drag: false,
            cursor: None,
            full_model_worker: None,
            hair_solver: None,
            smoke: false,
            use_gpu_secondary: false,
            gpu: None,
        })
    }
    /// Skeletal animation without advancing the expensive physical shell solver.
    /// # Errors
    /// Rejects invalid character assets.
    pub fn animation() -> DemoResult<Self> {
        let mut scene = Self::new()?;
        scene.female.animation_only = true;
        scene.female.pressing = false;
        Ok(scene)
    }
    /// Fixed stance with accepted character physics and geometric foot IK.
    /// Hair and nonlinear tissue simulation are not advanced by this diagnostic.
    pub fn physical_stance() -> DemoResult<Self> {
        let mut scene = Self::animation()?;
        scene.female.enable_physical_stance()?;
        Ok(scene)
    }
    /// An authored grounded gait in place on the actual character rig.
    /// This preview does not advance the nonlinear physical shell solver.
    /// # Errors
    /// Rejects invalid assets and clips outside idle/walk/jog/sprint.
    pub fn locomotion(clip: LocomotionClip) -> DemoResult<Self> {
        let mut scene = Self::animation()?;
        scene.female.enable_authored_gait(clip)?;
        Ok(scene)
    }
    /// A close view of skeletal hand closure around a cylinder, sphere, or OBJ mesh.
    /// # Errors
    /// Rejects invalid character or object assets.
    pub fn grasp(object: &str) -> DemoResult<Self> {
        let mut scene = Self::animation()?;
        let female = &mut scene.female;
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
        Ok(scene)
    }
    /// The complete inertial-motion example with dynamic hair and skin.
    pub fn secondary() -> DemoResult<Self> {
        let mut scene = Self::new()?;
        let female = &mut scene.female;
        female.secondary_only = true;
        if std::env::var("VOXY_AUTHORED_JUMP").as_deref() == Ok("1") {
            female.enable_authored_jump()?;
        }
        female.yaw = 1.1;
        female.distance = 2.1;
        female.pressing = false;
        Ok(scene)
    }
    /// Sets continuous body proportions for the imported character.
    pub fn with_body_parameters(mut self, parameters: BodyParameters) -> DemoResult<Self> {
        parameters.validate()?;
        self.female.set_body_parameters(parameters)?;
        self.female.distance *= parameters.height_cm / 164.;
        Ok(self)
    }
    /// Enables a gradual visual cold response for the body. The body's normalized
    /// response parameter becomes the target; time constants are explicit.
    pub fn with_body_cold_response(
        mut self,
        initial: f64,
        onset_seconds: f64,
        recovery_seconds: f64,
    ) -> DemoResult<Self> {
        self.female
            .enable_cold_response(initial, onset_seconds, recovery_seconds)?;
        Ok(self)
    }
    /// Watches an atomically replaced JSON parameter file, also written by the MCP server.
    pub fn with_body_parameter_file(
        mut self,
        path: impl Into<std::path::PathBuf>,
    ) -> DemoResult<Self> {
        let path = path.into();
        let parameters = BodyParameters::from_json(&std::fs::read_to_string(&path)?)?;
        self = self.with_body_parameters(parameters)?;
        self.female.parameter_file = Some(path);
        Ok(self)
    }
    /// Watches a detailed face constructor preset, preserving the last valid model on errors.
    pub fn with_face_parameter_file(
        mut self,
        path: impl Into<std::path::PathBuf>,
    ) -> DemoResult<Self> {
        let path = path.into();
        let parameters = FaceParameters::from_json(&std::fs::read_to_string(&path)?)?;
        let female = &mut self.female;
        female.face_parameters = parameters;
        female.face_parameter_file = Some(path);
        female.yaw = 0.;
        female.distance = 0.5;
        Ok(self)
    }
    /// Deposits a neutral fluid patch on the body's reference surface (SI units).
    pub fn with_surface_film(
        mut self,
        center: [f64; 3],
        radius: f64,
        volume_m3: f64,
    ) -> DemoResult<Self> {
        self.female.enable_film(center, radius, volume_m3)?;
        Ok(self)
    }

    fn wants_gpu_secondary(&self) -> bool {
        if std::env::var_os("VOXY_GPU_SECONDARY").is_some() {
            return true;
        }
        !self.smoke
            && std::env::var_os("VOXY_DISABLE_GPU_SECONDARY").is_none()
            && std::env::var_os("VOXY_CPU_SECONDARY").is_none()
            && std::env::var_os("VOXY_ASYNC_FULL").is_none()
            && std::env::var_os("VOXY_GPU_BODY").is_none()
            && std::env::var_os("VOXY_GPU_SURFACE_LIGHT").is_none()
            && std::env::var_os("VOXY_GPU_HAIR_PHYSICS").is_none()
            && std::env::var_os("VOXY_RESIDENT_HAIR").is_none()
            && self.female.can_use_gpu_secondary()
    }

    fn view(&self) -> [f32; 3] {
        [self.female.yaw, self.female.pitch, self.female.distance]
    }

    fn key_pressed(&mut self, key: DemoKey) -> DemoResult {
        let female = &mut self.female;
        match key {
            DemoKey::Digit1 | DemoKey::Digit2 | DemoKey::Digit3 | DemoKey::Digit4
                if female.has_grounded_playback() =>
            {
                let clip = match key {
                    DemoKey::Digit1 => LocomotionClip::Idle,
                    DemoKey::Digit2 => LocomotionClip::Walk,
                    DemoKey::Digit3 => LocomotionClip::Jog,
                    _ => LocomotionClip::Sprint,
                };
                if let Err(error) = female.select_grounded_gait(clip) {
                    eprintln!("Gait selection rejected: {error}");
                }
            }
            DemoKey::Digit1 | DemoKey::Digit2 | DemoKey::Digit3 if female.hand_focus => {
                let shape = match key {
                    DemoKey::Digit1 => "cylinder",
                    DemoKey::Digit2 => "sphere",
                    _ => "handle",
                };
                if let Err(error) = female.set_grasp_target(shape) {
                    eprintln!("Grasp target rejected: {error}");
                }
            }
            DemoKey::B => female.show_complexion = !female.show_complexion,
            DemoKey::P => female.pressing = !female.pressing,
            DemoKey::M => {
                female.show_skin = !female.show_skin;
                female.show_strain = false;
            }
            DemoKey::K => {
                female.show_strain = !female.show_strain;
                female.show_skin = false;
            }
            DemoKey::C => female.probe_enabled = !female.probe_enabled,
            DemoKey::BracketLeft | DemoKey::BracketRight if female.hand_focus => {
                let delta = if key == DemoKey::BracketRight {
                    0.1
                } else {
                    -0.1
                };
                female.adjust_grasp(delta)?;
            }
            DemoKey::G if female.hand_focus => female.set_grasp_cycle(true),
            _ => {}
        }
        Ok(())
    }

    fn key(&mut self, key: DemoKey, pressed: bool, repeat: bool) -> DemoResult {
        let held = match key {
            DemoKey::ArrowLeft => Some(0),
            DemoKey::ArrowRight => Some(1),
            DemoKey::ArrowUp => Some(2),
            DemoKey::ArrowDown => Some(3),
            _ => None,
        };
        if let Some(index) = held {
            self.camera_motion.held[index] = pressed;
        }
        if pressed && !repeat {
            self.key_pressed(key)?;
        }
        Ok(())
    }
}

impl SceneDemo for FemaleScene {
    fn configure(&mut self, env: DemoEnv) -> HostRequest {
        self.smoke = env.smoke;
        self.use_gpu_secondary = self.wants_gpu_secondary();
        if self.use_gpu_secondary {
            if std::env::var_os("VOXY_GPU_SECONDARY").is_none() {
                self.female.simulate_hair = false;
            }
            if !self.female.simulate_hair {
                self.female.gpu_secondary_fast = true;
            }
        }
        HostRequest {
            physical_size: self.use_gpu_secondary.then_some([1280, 900]),
            msaa4: !self.use_gpu_secondary || std::env::var_os("VOXY_GPU_SECONDARY_MSAA").is_some(),
            unthrottled_presentation: self.use_gpu_secondary
                || std::env::var_os("VOXY_ASYNC_FULL").is_some(),
            focus: true,
            smoke_timeout_seconds: 60,
            frame_trace: true,
        }
    }
    fn create(&mut self, gpu: DemoGpu<'_>) -> DemoResult<Option<SceneTexture>> {
        self.create_gpu(gpu).map(Some)
    }
    fn release(&mut self) {
        self.gpu = None;
    }
    fn begin_frame(&mut self, dt: f32) {
        let view = self
            .camera_motion
            .advance(self.view(), dt, self.female.hand_focus);
        [self.female.yaw, self.female.pitch, self.female.distance] = view;
    }
    fn advance(&mut self, tick: DemoTick) -> DemoResult {
        if self.full_model_worker.is_none() {
            let step = if tick.smoke {
                1.0 / 60.0
            } else {
                f64::from(tick.elapsed)
            };
            match self.hair_solver.as_mut() {
                Some(solver) => self.female.advance_with_hair_solver(step, solver)?,
                None => self.female.advance(step)?,
            }
        }
        Ok(())
    }
    fn camera(&self, view: DemoView) -> SceneCamera {
        perspective_camera(self.female.eye(), self.female.focus(), view.aspect, 100.0)
    }
    fn prepare(&mut self, frame: &mut DemoFrame<'_>) -> DemoResult {
        self.prepare_frame(frame)
    }
    fn frame<'a>(&'a mut self, shared: SharedDraw<'a>) -> DemoResult<FrameParts<'a>> {
        self.frame_parts(shared)
    }
    fn presented(&mut self, info: Presented) -> DemoResult {
        self.frame_presented(info)
    }
    fn input(&mut self, input: DemoInput) -> DemoResult<DemoReply> {
        match input {
            DemoInput::Key {
                key,
                pressed,
                repeat,
            } => self.key(key, pressed, repeat)?,
            DemoInput::PrimaryButton { pressed } => self.orbit_drag = pressed,
            DemoInput::PointerMoved { x, y } => {
                if self.orbit_drag
                    && let Some((last_x, last_y)) = self.cursor
                {
                    self.camera_motion.orbit(
                        self.view(),
                        (x - last_x) as f32 * 0.007,
                        (y - last_y) as f32 * 0.007,
                    );
                }
                self.cursor = Some((x, y));
            }
            DemoInput::Scroll(scroll) => {
                let amount = match scroll {
                    DemoScroll::Lines(y) => -y * 0.1,
                    DemoScroll::Pixels(y) => -y as f32 * 0.003,
                };
                self.camera_motion
                    .zoom(self.view(), amount, self.female.hand_focus);
            }
            DemoInput::FocusLost => {
                self.orbit_drag = false;
                self.camera_motion.held = [false; 4];
                self.cursor = None;
            }
        }
        Ok(DemoReply::default())
    }
    fn title(&self, paused: bool) -> Option<String> {
        let title = if let Some(worker) = &self.full_model_worker {
            format!("Full physics worker | {}", worker.title)
        } else if self
            .gpu
            .as_ref()
            .is_some_and(|gpu| gpu.gpu_secondary.is_some())
        {
            format!(
                "GPU preview | neutral face / initial irradiance | {}",
                self.female.title()
            )
        } else {
            self.female.title()
        };
        Some(if paused {
            format!("Paused | {title}")
        } else {
            format!("Playing | {title}")
        })
    }
    fn verify_smoke(&self, report: SmokeReport) -> DemoResult {
        self.female.verify()?;
        if self.female.film.is_some() {
            let gpu = self
                .gpu
                .as_ref()
                .ok_or("character resources are not created")?;
            if gpu.optical_frames != u64::from(report.frames) {
                return Err("film layers were absent from a presented smoke frame".into());
            }
            if gpu.film_mass_checks != u64::from(report.frames) {
                return Err("film mass checks missed a presented smoke frame".into());
            }
            println!(
                "FILM MASS SMOKE PASS: {} checks, max source-adjusted relative error={:.9e}",
                gpu.film_mass_checks, gpu.film_mass_max_relative_error
            );
            println!(
                "FILM OPTICAL SMOKE PASS: {} presented layers, temporal={}, resize_stages={}",
                gpu.optical_frames, report.motion_vectors, report.resize_stages
            );
        }
        println!(
            "FEMALE SKIN SMOKE PASS: {} nonlinear steps with rendered surface deformation",
            self.female.steps
        );
        Ok(())
    }
}
