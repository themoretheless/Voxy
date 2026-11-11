const FIXED_DT: f32 = 1.0 / 60.0;
const WALK_SPEED: f32 = 5.0;
mod animals;
mod app_tick;
mod bootstrap;
mod cpu_replay;
mod graphics_options;
mod native_plants;
mod pet;
mod terrain_options;

#[cfg(test)]
use app_tick::apply_autopilot_controls;
pub(crate) use app_tick::character_position;
pub(crate) use bootstrap::actor_vertex;
use bootstrap::{
    WaterPublication, bootstrap_actor, bootstrap_water_states, build_startup_scene,
    handle_early_cli_commands, loading_title, natural_water_bed, publish_water_plan, terrain_spawn,
    terrain_water_cells, wake_water_after_edits,
};

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use glam::{Mat4, Quat, Vec3};
use pet::{PetAction, PetState};
use physics_voxel::{
    AnchoredAabb, DestructionPlan, Explosion, RayOrigin, RaycastConfig, RaycastResult, raycast,
};
use physics_voxel::{
    CharacterConfig, CharacterInput, CharacterState, ProjectileConfig, ProjectileOutcome,
    ProjectileState, RaceCheckpoint, RaceProgress, RaceTrack, VehicleConfig, VehicleInput,
    VehicleState, WaterBudget, WaterPlan, WaterStates, plan_impact_explosion, spawn_projectile,
    step_character_in_field, step_projectile_in_field, step_vehicle, step_water, update_race,
};
use voxy_animation::{
    AnimationClip, Animator, Joint, JointTrack, Playback, QuatKey, Skeleton, Transform,
};
use voxy_render::{CameraView, RenderOutcome, Renderer, RendererError, SkinnedMesh, SkinnedVertex};
#[cfg(test)]
use voxy_runtime::build_procedural_scene;
use voxy_runtime::{
    BootstrapChunk, BootstrapScene, JobClass, RebuildWorkerPool, extract_chunk_neighborhood,
    invalidated_derived_chunks,
};
use voxy_shell::controls::{self, CameraRig, InputState};
use voxy_shell::frame_profile;
use voxy_world::{ChunkPos, EditSource, ResourceKey, VoxelPos, World};
use winit::application::ApplicationHandler;
use winit::event::{
    DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

#[derive(Debug, Default)]
struct VoxyApp {
    profile: Option<frame_profile::Profile>,
    profile_tick: u64,
    profile_completed_ticks: u64,
    shadow_audit: bool,
    shadow_audit_samples: u64,
    shadow_audit_last_tick: u64,
    profile_partial_publications: u64,
    profile_full_publications: u64,
    profile_water: Duration,
    profile_animation: Duration,
    profile_pose: Duration,
    graphics: voxy_render::GraphicsOptions,
    terrain_radius: i32,
    motion_compute: Option<Arc<voxy_cuda::CudaCompute>>,
    cuda_projectiles: bool,
    cuda_character_motion: bool,
    cuda_collisions: bool,
    cuda_collision_ticks: u64,
    cuda_vehicle_motion: bool,
    startup_failure: Option<String>,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    presented_once: bool,
    presented_frames: u64,
    last_startup_outcome: Option<RenderOutcome>,
    surface_retry_at: Option<Instant>,
    actor: Option<AnimatedActor>,
    pending_actor_pose: Option<(Vec<Mat4>, Mat4)>,
    actor_palette_scratch: Vec<Mat4>,
    last_update: Option<Instant>,
    accumulator: f32,
    input: InputState,
    camera: CameraRig,
    actor_position: Vec3,
    actor_yaw: f32,
    character: Option<CharacterState>,
    world: Option<World>,
    native_plants: Option<native_plants::NativePlants>,
    vegetation_check_frames: u8,
    cursor_captured: bool,
    projectiles: Vec<ProjectileState>,
    resident_positions: Vec<ChunkPos>,
    /// Last rebuilt light and mesh per resident chunk; uploaded as one set.
    derived_chunks: BTreeMap<ChunkPos, BootstrapChunk>,
    /// Resident chunks whose derived data must be rebuilt after recent commits.
    dirty_chunks: BTreeSet<ChunkPos>,
    camera_anchor: ChunkPos,
    derivation_epoch: u64,
    rebuild_pool: Option<RebuildWorkerPool>,
    rebuild_pending: Option<u64>,
    cuda_water: Option<voxy_gpu::CudaWaterTransferProgram>,
    cuda_water_ticks: u64,
    gpu_water: Option<voxy_gpu::WaterTransferProgram>,
    gpu_water_ticks: u64,
    gpu_collisions: Option<voxy_gpu::VoxelRegionProgram>,
    pending_character: Option<voxy_gpu::PendingGpuCharacter>,
    pending_character_intent: Option<(controls::MoveIntent, Vec3)>,
    pending_character_started: Option<Instant>,
    gpu_character_ticks: u64,
    pending_vehicle: Option<voxy_gpu::PendingGpuVehicle>,
    pending_vehicle_started: Option<Instant>,
    gpu_vehicle_ticks: u64,
    pending_water: Option<PendingGameWater>,
    water_states: Option<WaterStates>,
    water_active: Vec<VoxelPos>,
    water_tick_divider: u8,
    driving: bool,
    vehicle: Option<VehicleState>,
    race_track: Option<RaceTrack>,
    race_progress: RaceProgress,
    autopilot_tick: Option<u64>,
    autopilot_water_bed: Option<VoxelPos>,
    autopilot_start: Vec3,
    autopilot_initial_yaw: f32,
    explosion_commits: u64,
    water_commits: u64,
    peak_vehicle_speed: f64,
    animation_frames: u64,
    autopilot_result: Option<bool>,
    animal_check: bool,
    animal_check_tick: u64,
    animal_check_captures: u64,
    companions: Vec<animals::Companion>,
    pet: PetState,
    pet_ui_elapsed: f32,
    load_time_seconds: f32,
    bodycam_mode: bool,
    clean_cam_mode: bool,
    camcorder_mode: bool,
    was_grounded: bool,
}

#[derive(Debug)]
struct PendingGameWater {
    plan: voxy_gpu::PendingWaterPlan,
    active: Vec<VoxelPos>,
    started: Instant,
}

#[derive(Debug)]
struct AnimatedActor {
    skeleton: Skeleton,
    animator: Animator,
}

impl ApplicationHandler for VoxyApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let loading_started = Instant::now();
        let attributes = Window::default_attributes()
            .with_title(loading_title(0, "STARTING"))
            .with_visible(false)
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
        let attributes = if self.profile.is_some() {
            attributes
                .with_inner_size(winit::dpi::PhysicalSize::new(1920, 1080))
                .with_resizable(false)
        } else {
            attributes
        };
        let Ok(window) = event_loop.create_window(attributes).map(Arc::new) else {
            eprintln!("Voxy could not create a desktop window");
            event_loop.exit();
            return;
        };
        let size = window.inner_size();
        window.set_title(&loading_title(10, "GPU"));
        let instance = self
            .graphics
            .create_instance_with_display(event_loop.owned_display_handle());
        match pollster::block_on(Renderer::new_with_instance(
            Arc::clone(&window),
            size.width,
            size.height,
            self.graphics,
            instance,
        )) {
            Ok(mut renderer) => {
                window.set_title(&loading_title(35, "WORLD"));
                let scene_result = build_startup_scene(self.graphics, self.terrain_radius);
                let mut scene = match scene_result {
                    Ok(scene) => scene,
                    Err(error) => {
                        eprintln!("Voxy world bootstrap failed: {error}");
                        self.startup_failure =
                            Some(format!("Voxy world bootstrap failed: {error}"));
                        event_loop.exit();
                        return;
                    }
                };
                if self.animal_check
                    && let Err(error) = animals::prepare_check_scene(&mut scene)
                {
                    self.startup_failure =
                        Some(format!("Animal inspection terrain failed: {error}"));
                    event_loop.exit();
                    return;
                }
                let uploads = scene
                    .chunks
                    .iter()
                    .map(|chunk| (chunk.pos, &chunk.mesh, &chunk.light))
                    .collect::<Vec<_>>();
                window.set_title(&loading_title(72, "MESHES"));
                if let Err(error) = renderer.upload_lit_chunks(&uploads, scene.anchor) {
                    eprintln!("Voxy mesh upload failed: {error}");
                    event_loop.exit();
                    return;
                }
                window.set_title(&loading_title(84, "PET"));
                let (actor, actor_mesh) = match bootstrap_actor() {
                    Ok(actor) => actor,
                    Err(error) => {
                        eprintln!("Voxy animated actor bootstrap failed: {error}");
                        event_loop.exit();
                        return;
                    }
                };
                let mut bind_matrices =
                    match actor.skeleton.bind_pose().skin_matrices(&actor.skeleton) {
                        Ok(matrices) => matrices,
                        Err(error) => {
                            eprintln!("Voxy actor bind pose failed: {error}");
                            event_loop.exit();
                            return;
                        }
                    };
                for x in [2.0, -2.0] {
                    bind_matrices.extend(
                        [Mat4::from_translation(Vec3::new(x, 4.0, 0.0)); animals::JOINTS / 2],
                    );
                }
                if let Err(error) = renderer.upload_skinned_mesh(
                    &actor_mesh,
                    &bind_matrices,
                    Mat4::from_translation(Vec3::new(16.0, 10.0, 16.0)),
                    2,
                ) {
                    eprintln!("Voxy animated mesh upload failed: {error}");
                    event_loop.exit();
                    return;
                }
                window.set_title(&loading_title(94, "GAMEPLAY"));
                let info = renderer.adapter_info();
                println!(
                    "Voxy GPU: {} ({:?}), {} chunks, {} greedy quads",
                    info.name,
                    info.backend,
                    scene.chunks.len(),
                    scene
                        .chunks
                        .iter()
                        .map(|chunk| chunk.mesh.quad_count())
                        .sum::<usize>()
                );
                if std::env::args().any(|argument| argument == "--gpu-water") {
                    match pollster::block_on(voxy_gpu::WaterTransferProgram::new(renderer.device()))
                    {
                        Ok(program) => {
                            self.gpu_water = Some(program);
                            println!(
                                "Voxy water simulation: GPU ordered transfers ({:?})",
                                info.backend
                            );
                        }
                        Err(error) => {
                            self.startup_failure =
                                Some(format!("Voxy GPU water initialization failed: {error}"));
                            event_loop.exit();
                            return;
                        }
                    }
                }
                if std::env::args().any(|argument| argument == "--gpu-collisions") {
                    match pollster::block_on(voxy_gpu::VoxelRegionProgram::new(renderer.device())) {
                        Ok(program) => self.gpu_collisions = Some(program),
                        Err(error) => {
                            self.startup_failure =
                                Some(format!("GPU collision initialization failed: {error}"));
                            event_loop.exit();
                            return;
                        }
                    }
                    println!(
                        "Voxy character collision: nonblocking GPU broadphase ({:?})",
                        info.backend
                    );
                }
                renderer.set_bodycam_enabled(self.bodycam_mode);
                if self.camcorder_mode {
                    renderer
                        .bodycam_rig_mut()
                        .set_lens_profile(voxy_render::PhotorealLensProfile::Camcorder);
                } else if self.clean_cam_mode {
                    renderer
                        .bodycam_rig_mut()
                        .set_lens_profile(voxy_render::PhotorealLensProfile::CleanCam);
                }
                if self.bodycam_mode {
                    let profile_name = if self.camcorder_mode {
                        "Camcorder"
                    } else if self.clean_cam_mode {
                        "Clean Cam"
                    } else {
                        "Bodycam"
                    };
                    println!(
                        "Voxy Photorealistic Mode active ({profile_name}) (B: bodycam, V: clean cam, N: camcorder, F: light)"
                    );
                }
                if !self.animal_check && !std::env::args().any(|arg| arg == "--voxel-plants") {
                    let validation = renderer
                        .device()
                        .push_error_scope(wgpu::ErrorFilter::Validation);
                    let result = native_plants::NativePlants::build(
                        &renderer,
                        &scene.world,
                        &scene.chunks,
                        0x56_4f_58_59,
                    );
                    if let Some(error) = pollster::block_on(validation.pop()) {
                        self.startup_failure =
                            Some(format!("Native vegetation GPU validation: {error}"));
                        event_loop.exit();
                        return;
                    }
                    match result {
                        Ok((plants, gpu)) => {
                            renderer.set_native_vegetation(Some(gpu));
                            self.native_plants = Some(plants);
                        }
                        Err(error) => {
                            self.startup_failure =
                                Some(format!("Native vegetation startup failed: {error}"));
                            event_loop.exit();
                            return;
                        }
                    }
                }
                self.renderer = Some(renderer);
                self.actor = Some(actor);
                match self.initialize_gameplay(&scene) {
                    Ok(()) => {}
                    Err(error) => {
                        eprintln!("Voxy water bootstrap failed: {error}");
                        event_loop.exit();
                        return;
                    }
                }
                self.autopilot_tick = (std::env::args().any(|argument| argument == "--autopilot")
                    || std::env::var("VOXY_AUTOPILOT").is_ok_and(|value| value == "1"))
                .then_some(0);
                self.autopilot_start = self.actor_position;
                self.autopilot_initial_yaw = self.camera.yaw;
                self.derived_chunks = scene
                    .chunks
                    .into_iter()
                    .map(|chunk| (chunk.pos, chunk))
                    .collect();
                self.rebuild_pool = Some(RebuildWorkerPool::new(scene.world.epoch(), 2, 8));
                self.world = Some(scene.world);
                self.last_update = Some(Instant::now());
                self.load_time_seconds = loading_started.elapsed().as_secs_f32();
                self.window = Some(Arc::clone(&window));
                self.update_pet_title();
                println!("Voxy loaded in {:.2} seconds", self.load_time_seconds);
                window.set_visible(true);
                window.focus_window();
                window.request_redraw();
            }
            Err(error) => {
                eprintln!("Voxy renderer initialization failed: {error}");
                self.startup_failure =
                    Some(format!("Voxy renderer initialization failed: {error}"));
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .window
            .as_ref()
            .is_none_or(|window| window.id() != window_id)
        {
            return;
        }
        if self.profile.is_some()
            && matches!(
                event,
                WindowEvent::KeyboardInput { .. }
                    | WindowEvent::MouseInput { .. }
                    | WindowEvent::MouseWheel { .. }
                    | WindowEvent::CursorMoved { .. }
            )
        {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                if self.animal_check && self.animal_check_captures < 3 {
                    let reason = format!(
                        "Animal inspection interrupted by window close at tick {}: {}/3 captures",
                        self.animal_check_tick, self.animal_check_captures
                    );
                    eprintln!("{reason}");
                    if !animals::hold_check_window() {
                        self.startup_failure = Some(reason);
                    }
                }
                if self.profile.is_some() {
                    eprintln!("Voxy profile interrupted by window close");
                }
                if self.autopilot_tick.is_some() && self.autopilot_result.is_none() {
                    eprintln!(
                        "Voxy autopilot interrupted by window close: tick={:?}, GPU character={}, GPU vehicle={}",
                        self.autopilot_tick, self.gpu_character_ticks, self.gpu_vehicle_ticks
                    );
                }
                event_loop.exit();
            }
            WindowEvent::Focused(false) => {
                self.input.cancel_all();
                self.pending_character_intent = None;
                self.set_cursor_captured(false);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if code == KeyCode::Escape && event.state == ElementState::Pressed {
                        self.set_cursor_captured(false);
                    } else if code == KeyCode::KeyR
                        && event.state == ElementState::Pressed
                        && !event.repeat
                    {
                        self.toggle_driving();
                    } else if code == KeyCode::KeyB
                        && event.state == ElementState::Pressed
                        && !event.repeat
                    {
                        let enable =
                            !(self.bodycam_mode && !self.clean_cam_mode && !self.camcorder_mode);
                        self.apply_camera_profile(
                            enable,
                            false,
                            false,
                            voxy_render::PhotorealLensProfile::Bodycam,
                        );
                        println!(
                            "Voxy Bodycam Photorealistic Mode: {}",
                            if self.bodycam_mode { "ON" } else { "OFF" }
                        );
                    } else if code == KeyCode::KeyV
                        && event.state == ElementState::Pressed
                        && !event.repeat
                    {
                        let enable = !(self.bodycam_mode && self.clean_cam_mode);
                        self.apply_camera_profile(
                            enable,
                            enable,
                            false,
                            voxy_render::PhotorealLensProfile::CleanCam,
                        );
                        println!(
                            "Voxy Clean Photorealistic Camera (Cam): {}",
                            if self.bodycam_mode && self.clean_cam_mode {
                                "ON"
                            } else {
                                "OFF"
                            }
                        );
                    } else if code == KeyCode::KeyN
                        && event.state == ElementState::Pressed
                        && !event.repeat
                    {
                        let enable = !(self.bodycam_mode && self.camcorder_mode);
                        self.apply_camera_profile(
                            enable,
                            false,
                            enable,
                            voxy_render::PhotorealLensProfile::Camcorder,
                        );
                        println!(
                            "Voxy Hand-Held Video Camcorder Mode: {}",
                            if self.bodycam_mode && self.camcorder_mode {
                                "ON (Use Mouse Wheel for 1x-12x Servo Zoom)"
                            } else {
                                "OFF"
                            }
                        );
                    } else if code == KeyCode::KeyF
                        && event.state == ElementState::Pressed
                        && !event.repeat
                    {
                        if let Some(renderer) = &mut self.renderer {
                            let rig = renderer.bodycam_rig_mut();
                            rig.flashlight_enabled = !rig.flashlight_enabled;
                            println!(
                                "Voxy Tactical Flashlight: {}",
                                if rig.flashlight_enabled { "ON" } else { "OFF" }
                            );
                        }
                    } else if event.state == ElementState::Pressed
                        && !event.repeat
                        && self.handle_pet_key(code)
                    {
                    } else {
                        self.input.key(code, event.state, event.repeat);
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if self.cursor_captured {
                    self.input.queue_fire();
                } else {
                    self.set_cursor_captured(true);
                }
            }
            WindowEvent::Touch(touch) => {
                let width = self
                    .window
                    .as_ref()
                    .map_or(1.0, |window| window.inner_size().width.max(1) as f32);
                self.input.touch(touch, width);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(position) => position.y as f32 / 40.0,
                };
                if self.bodycam_mode && self.camcorder_mode {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.bodycam_rig_mut().apply_servo_zoom(lines);
                    }
                } else {
                    self.camera.zoom(lines);
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::Occluded(false) => {
                self.surface_retry_at = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if self
                    .profile
                    .as_ref()
                    .is_some_and(frame_profile::Profile::is_complete)
                {
                    return;
                }
                if let Some(message) = self.renderer.as_ref().and_then(Renderer::device_failure) {
                    self.startup_failure = Some(format!("Voxy GPU device lost: {message}"));
                    event_loop.exit();
                    return;
                }
                self.profile_water = Duration::ZERO;
                self.profile_animation = Duration::ZERO;
                self.profile_pose = Duration::ZERO;
                let profile_started = Instant::now();
                let presented_before = self
                    .renderer
                    .as_ref()
                    .map_or(0, Renderer::profile_presented_frames);
                self.advance_animation(event_loop);
                let simulation_time = profile_started.elapsed();
                let preparation_started = Instant::now();
                self.poll_rebuild(event_loop);
                let mut profile_preparation = Duration::ZERO;
                let mut profile_render = Duration::ZERO;
                let mut vegetation_edited = false;
                if let Some(renderer) = &mut self.renderer {
                    if let (Some(plants), Some(world)) = (&mut self.native_plants, &mut self.world)
                    {
                        match plants.sync(world, renderer) {
                            Ok(Some(receipt)) => {
                                self.dirty_chunks
                                    .extend(invalidated_derived_chunks(&receipt));
                                vegetation_edited = true;
                            }
                            Ok(None) => {}
                            Err(error) => {
                                self.startup_failure = Some(error.to_string());
                                event_loop.exit();
                                return;
                            }
                        }
                        if std::env::args().any(|a| a == "--vegetation-check")
                            && let Err(error) = renderer.update_camera(plants.inspection_camera())
                        {
                            self.startup_failure = Some(error.to_string());
                            event_loop.exit();
                            return;
                        }
                    }
                    if self.shadow_audit
                        && self.profile_completed_ticks > self.shadow_audit_last_tick
                    {
                        let tick = self.profile_completed_ticks;
                        let result = renderer
                            .capture_shadow_receiver_comparison(tick, tick as f32 * FIXED_DT);
                        match result {
                            Ok(([control, candidate], [before, after]))
                                if control == candidate && after < before =>
                            {
                                self.shadow_audit_samples += 1;
                                self.shadow_audit_last_tick = tick;
                                println!(
                                    "SHADOW PARITY tick={tick} width={} height={} casters_before={before} casters_after={after} blake3={}",
                                    control.0,
                                    control.1,
                                    blake3::hash(&control.2),
                                );
                                if tick >= 1200 {
                                    println!(
                                        "SHADOW PARITY PASS samples={} last_tick={tick} resident_chunks={} destruction_commits={} partial_publications={} full_publications={}",
                                        self.shadow_audit_samples,
                                        self.derived_chunks.len(),
                                        self.explosion_commits,
                                        self.profile_partial_publications,
                                        self.profile_full_publications,
                                    );
                                    event_loop.exit();
                                    return;
                                }
                            }
                            Ok(([control, candidate], [before, after])) => {
                                let changed = control
                                    .2
                                    .chunks_exact(4)
                                    .zip(candidate.2.chunks_exact(4))
                                    .filter(|(a, b)| a != b)
                                    .count();
                                self.startup_failure = Some(format!(
                                    "Shadow receiver parity mismatch at tick {tick}: changed_pixels={changed} casters_before={before} casters_after={after}",
                                ));
                                event_loop.exit();
                                return;
                            }
                            Err(error) => {
                                self.startup_failure = Some(error.to_string());
                                event_loop.exit();
                                return;
                            }
                        }
                    }
                    profile_preparation = preparation_started.elapsed();
                    let render_started = Instant::now();
                    let rendered = renderer.render();
                    self.surface_retry_at = matches!(
                        rendered,
                        Ok(RenderOutcome::SkippedOccluded | RenderOutcome::SkippedTimeout)
                    )
                    .then(|| Instant::now() + Duration::from_millis(50));
                    match rendered {
                        Ok(outcome)
                            if outcome == RenderOutcome::Presented
                                || (outcome == RenderOutcome::SkippedOccluded
                                    && std::env::args().any(|a| a == "--vegetation-check")) =>
                        {
                            if outcome == RenderOutcome::Presented {
                                self.presented_frames = self.presented_frames.saturating_add(1);
                            }
                            if std::env::args().any(|a| a == "--vegetation-check") {
                                self.vegetation_check_frames =
                                    self.vegetation_check_frames.saturating_add(1);
                                if self.vegetation_check_frames == 3 {
                                    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                                        let (w, h, with) =
                                            renderer.capture_world_rgba8(self.presented_frames)?;
                                        let (_, _, without) = renderer
                                            .capture_world_without_vegetation_rgba8(
                                                self.presented_frames,
                                            )?;
                                        let changed_pixels = with
                                            .chunks_exact(4)
                                            .zip(without.chunks_exact(4))
                                            .filter(|(a, b)| {
                                                a[..3]
                                                    .iter()
                                                    .zip(&b[..3])
                                                    .any(|(a, b)| a.abs_diff(*b) > 3)
                                            })
                                            .count();
                                        let plants = self
                                            .native_plants
                                            .as_ref()
                                            .map_or(0, native_plants::NativePlants::active_count);
                                        let (trees, shrubs) = self.native_plants.as_ref().map_or(
                                            (0, 0),
                                            native_plants::NativePlants::kind_counts,
                                        );
                                        let output = if self.bodycam_mode {
                                            "artifacts/vegetation-world/hdr"
                                        } else {
                                            "artifacts/vegetation-world"
                                        };
                                        std::fs::create_dir_all(output)?;
                                        image::save_buffer(
                                            format!("{output}/with-plants.png"),
                                            &with,
                                            w,
                                            h,
                                            image::ColorType::Rgba8,
                                        )?;
                                        image::save_buffer(
                                            format!("{output}/without-plants.png"),
                                            &without,
                                            w,
                                            h,
                                            image::ColorType::Rgba8,
                                        )?;
                                        let report = serde_json::json!({"passed": trees>0 && shrubs>0 && changed_pixels>1000,"plants":plants,"trees":trees,"shrubs":shrubs,"hdr":self.bodycam_mode,"capture":"offscreen_gpu","changed_pixels":changed_pixels,"presented_frames":self.presented_frames,"width":w,"height":h});
                                        std::fs::write(
                                            format!("{output}/report.json"),
                                            serde_json::to_vec_pretty(&report)?,
                                        )?;
                                        println!("Native vegetation world check: {report}");
                                        if trees == 0 || shrubs == 0 || changed_pixels <= 1000 {
                                            return Err("native vegetation was not visible in the world frame".into());
                                        }
                                        Ok(())
                                    })();
                                    if let Err(error) = result {
                                        self.startup_failure = Some(error.to_string());
                                    }
                                    event_loop.exit();
                                }
                            }

                            // Short-interval captures diagnose geometry before a long
                            // movement run. They retain the full movement acceptance
                            // condition and therefore cannot pass that check early.
                            let animal_capture_interval =
                                if std::env::args().any(|a| a == "--animal-fast-portraits") {
                                    20
                                } else {
                                    180
                                };
                            if self.animal_check
                                && self.animal_check_captures < 3
                                && self.animal_check_tick
                                    >= (self.animal_check_captures + 1) * animal_capture_interval
                            {
                                let result = (|| -> Result<(), String> {
                                    let (width, height, rgba) = renderer
                                        .capture_bodycam_rgba8(self.presented_frames)
                                        .map_err(|e| e.to_string())?;
                                    let animal_output = animals::output_directory();
                                    std::fs::create_dir_all(&animal_output)
                                        .map_err(|e| e.to_string())?;
                                    image::save_buffer(
                                        animal_output
                                            .join(format!("frame-{}.png", self.animal_check_tick)),
                                        &rgba,
                                        width,
                                        height,
                                        image::ColorType::Rgba8,
                                    )
                                    .map_err(|e| e.to_string())?;
                                    let report = serde_json::json!({"passed":self.companions.iter().all(|animal|animal.grounded() && animal.fur_particles()==144 && animal.travelled_distance>0.5),"tick":self.animal_check_tick,"presented_frames":self.presented_frames,"animals":self.companions.iter().map(|animal|serde_json::json!({"breed":animal.breed_id(),"position":animal.position().to_array(),"fur_tip_displacement":animal.fur_tip_displacement().unwrap_or(f32::NAN),"grounded":animal.grounded(),"fur_particles":animal.fur_particles(),"travelled_distance":animal.travelled_distance})).collect::<Vec<_>>()});
                                    std::fs::write(
                                        animal_output.join("report.json"),
                                        serde_json::to_vec_pretty(&report)
                                            .map_err(|e| e.to_string())?,
                                    )
                                    .map_err(|e| e.to_string())?;
                                    println!("Voxy animals capture: {report}");
                                    Ok(())
                                })();
                                if let Err(error) = result {
                                    self.startup_failure = Some(error);
                                    event_loop.exit();
                                }
                                self.animal_check_captures += 1;
                                if self.animal_check_captures == 3 {
                                    if !self.companions.iter().all(|animal| {
                                        animal.grounded()
                                            && animal.fur_particles() == 144
                                            && animal.travelled_distance > 0.5
                                    }) {
                                        self.startup_failure=Some("Animal window check failed: grounding, movement or fur attachment".into());
                                    } else {
                                        println!(
                                            "Voxy animals window check passed: 3 captures, 2 grounded moving animals, 288 simulated fur particles"
                                        );
                                    }
                                    if !animals::hold_check_window() {
                                        event_loop.exit();
                                    }
                                }
                            }

                            if !self.presented_once && outcome == RenderOutcome::Presented {
                                println!("Voxy first voxel frame presented ({outcome:?})");
                                self.presented_once = true;
                            }
                        }
                        Ok(outcome) if self.last_startup_outcome != Some(outcome) => {
                            eprintln!("Voxy startup frame deferred: {outcome:?}");
                            self.last_startup_outcome = Some(outcome);
                        }
                        Err(error) => {
                            eprintln!("Voxy render failed: {error}");
                            if matches!(
                                error,
                                RendererError::SurfaceLost | RendererError::DeviceLost(_)
                            ) {
                                self.startup_failure = Some(format!("Voxy render failed: {error}"));
                                event_loop.exit();
                            }
                        }
                        _ => {}
                    }
                    profile_render = render_started.elapsed();
                }
                if vegetation_edited {
                    self.refresh_world_geometry(event_loop);
                }
                if !self.shadow_audit
                    && let Some(profile) = &mut self.profile
                {
                    profile.cpu_stage(Instant::now(), "water", self.profile_water);
                    profile.cpu_stage(Instant::now(), "actor_animation", self.profile_animation);
                    profile.cpu_stage(Instant::now(), "pose_publication", self.profile_pose);
                    if let Some((pose, writes)) = self
                        .renderer
                        .as_mut()
                        .and_then(Renderer::take_profile_pose_timing)
                    {
                        let now = Instant::now();
                        let path = if pose.fallback {
                            "pose_exact_vertex_scan"
                        } else if pose.workers == 0 {
                            "pose_family_certificate"
                        } else {
                            "pose_group_certificate"
                        };
                        let mut stage = |name, micros| {
                            profile.cpu_stage(now, name, Duration::from_secs_f64(micros * 1e-6));
                        };
                        stage("pose_admission", pose.total_wall_us);
                        stage(path, pose.total_wall_us);
                        stage("pose_spawn_wall", pose.spawn_wall_us);
                        stage("pose_worker_launch_max", pose.launch_delay_max_us);
                        stage("pose_join_wall", pose.join_wall_us);
                        if pose.cpu_clock_available {
                            stage("pose_caller_cpu", pose.caller_cpu_us);
                            stage("pose_worker_cpu_sum", pose.worker_cpu_sum_us);
                            stage("pose_join_cpu", pose.join_cpu_us);
                        }
                        profile.cpu_stage(now, "pose_queue_writes", writes);
                    }
                    let renderer = self.renderer.as_ref();
                    if let Some(ms) = renderer.and_then(Renderer::profile_gpu_ms) {
                        profile.gpu_sample(Instant::now(), [Some(ms), None, None]);
                    }
                    if let Some(renderer) = renderer {
                        profile.vegetation_gpu_sample(
                            Instant::now(),
                            renderer.profile_vegetation_gpu_ms(),
                        );
                    }
                    if profile.record(
                        Instant::now(),
                        simulation_time,
                        profile_preparation,
                        profile_render,
                        renderer.map(Renderer::profile_acquire_time),
                        renderer.is_some_and(|r| r.profile_presented_frames() > presented_before),
                    ) {
                        let adapter = self
                            .renderer
                            .as_ref()
                            .map(|r| format!("{:?}", r.adapter_info()))
                            .unwrap_or_default();
                        match renderer.and_then(Renderer::profile_gpu_timestamp_status) {
                            Some([scheduled, valid, rejected, map_failures, pending]) => eprintln!(
                                "Voxy GPU timestamp readbacks scheduled={scheduled} valid={valid} rejected={rejected} map_failures={map_failures} pending={pending}"
                            ),
                            None => eprintln!("Voxy GPU timestamp queries unavailable"),
                        }
                        let workload = frame_profile::VoxelWorkload {
                            simulation_ticks: self.profile_completed_ticks,
                            destruction_commits: self.explosion_commits,
                            water_commits: self.water_commits,
                            resident_chunks: self.derived_chunks.len(),
                            partial_publications: self.profile_partial_publications,
                            full_publications: self.profile_full_publications,
                        };
                        if let Err(error) = profile.finish_voxel(&adapter, workload) {
                            self.startup_failure = Some(error.to_string());
                        }
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if self.cursor_captured
            && let DeviceEvent::MouseMotion { delta } = event
        {
            self.input.mouse_motion(delta);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self
            .surface_retry_at
            .filter(|deadline| *deadline > Instant::now())
        {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            return;
        }
        event_loop.set_control_flow(ControlFlow::Wait);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl VoxyApp {
    fn apply_camera_profile(
        &mut self,
        bodycam: bool,
        clean_cam: bool,
        camcorder: bool,
        profile: voxy_render::PhotorealLensProfile,
    ) {
        self.bodycam_mode = bodycam;
        self.clean_cam_mode = clean_cam;
        self.camcorder_mode = camcorder;
        if let Some(renderer) = &mut self.renderer {
            renderer.bodycam_rig_mut().set_lens_profile(profile);
            renderer.set_bodycam_enabled(self.bodycam_mode);
        }
    }

    fn handle_pet_key(&mut self, code: KeyCode) -> bool {
        let action = match code {
            KeyCode::KeyE => PetAction::Feed,
            KeyCode::KeyP => PetAction::Play,
            KeyCode::KeyZ => PetAction::Sleep,
            KeyCode::KeyH => PetAction::Heal,
            KeyCode::KeyC => PetAction::Clean,
            _ => return false,
        };
        self.pet.act(action);
        self.update_pet_title();
        true
    }

    fn update_pet_title(&self) {
        if let Some(window) = &self.window {
            window.set_title(&format!(
                "{}  |  LOADED {:.2}s",
                self.pet.title(),
                self.load_time_seconds
            ));
        }
    }

    fn initialize_gameplay(&mut self, scene: &BootstrapScene) -> Result<(), String> {
        let spawn = terrain_spawn(&scene.world)?;
        self.actor_position = Vec3::new(
            16.0,
            f32::from(i16::try_from(spawn.y).map_err(|error| error.to_string())?),
            16.0,
        );
        self.companions = [false, true]
            .into_iter()
            .map(|cat| {
                animals::Companion::spawn(
                    VoxelPos {
                        x: spawn.x + if cat { -2 } else { 2 },
                        y: spawn.y + 4,
                        ..spawn
                    },
                    cat,
                )
            })
            .collect();
        self.character = Some(CharacterState {
            body: AnchoredAabb {
                anchor: spawn,
                min: [-0.35, 0.0, -0.35],
                max: [0.35, 1.8, 0.35],
            },
            velocity: [0.0; 3],
            grounded: false,
        });
        self.vehicle = self.character.map(|chassis| VehicleState {
            chassis,
            heading: 0.0,
            longitudinal_speed: 0.0,
        });
        self.race_track = Some(
            RaceTrack::new(
                vec![
                    RaceCheckpoint {
                        center: [16.0, 10.0, 24.0],
                        radius: 5.0,
                    },
                    RaceCheckpoint {
                        center: [24.0, 10.0, 16.0],
                        radius: 5.0,
                    },
                    RaceCheckpoint {
                        center: [16.0, 10.0, 8.0],
                        radius: 5.0,
                    },
                    RaceCheckpoint {
                        center: [8.0, 10.0, 16.0],
                        radius: 5.0,
                    },
                ],
                3,
            )
            .map_err(|error| error.to_string())?,
        );
        self.resident_positions = scene.chunks.iter().map(|chunk| chunk.pos).collect();
        self.camera_anchor = scene.anchor;
        let states = bootstrap_water_states(&scene.world)?;
        self.water_active = terrain_water_cells(scene, states)?;
        if self
            .profile
            .as_ref()
            .is_some_and(|p| p.scenario == frame_profile::Scenario::Water)
            || std::env::args().any(|argument| argument == "--autopilot")
            || std::env::var("VOXY_AUTOPILOT").is_ok_and(|value| value == "1")
        {
            self.autopilot_water_bed = natural_water_bed(&scene.world, states, &self.water_active);
        }
        println!("Voxy water activation: {} cells", self.water_active.len());
        self.water_states = Some(states);
        Ok(())
    }

    fn set_cursor_captured(&mut self, captured: bool) {
        let Some(window) = &self.window else {
            return;
        };
        if captured {
            if window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
                .is_ok()
            {
                window.set_cursor_visible(false);
                self.cursor_captured = true;
            }
        } else {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
            window.set_cursor_visible(true);
            self.cursor_captured = false;
        }
    }

    fn toggle_driving(&mut self) {
        self.pending_vehicle = None;
        self.pending_vehicle_started = None;
        self.pending_character = None;
        self.pending_character_intent = None;
        self.pending_character_started = None;
        self.driving = !self.driving;
        if self.driving {
            if let (Some(character), Some(vehicle)) = (self.character, &mut self.vehicle) {
                vehicle.chassis = character;
                vehicle.longitudinal_speed = 0.0;
            }
            println!("Voxy racing mode: WASD drive, Space brake, R exit");
        } else {
            if let Some(vehicle) = self.vehicle {
                self.character = Some(vehicle.chassis);
            }
            println!("Voxy character mode");
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if animals::check_requested() {
        std::env::set_current_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))?;
    }
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if let Some(()) = handle_early_cli_commands(&arguments)? {
        return Ok(());
    }

    terrain_options::TerrainBackend::parse(std::env::args().skip(1))
        .map_err(std::io::Error::other)?;
    if std::env::args().any(|arg| arg == "--cuda-collisions")
        && std::env::args().any(|arg| arg == "--gpu-collisions")
    {
        return Err(std::io::Error::other("choose --gpu-collisions or --cuda-collisions").into());
    }
    if std::env::args().any(|arg| arg == "--cuda-water")
        && std::env::args().any(|arg| arg == "--gpu-water")
    {
        return Err(std::io::Error::other("choose --gpu-water or --cuda-water").into());
    }
    let graphics =
        graphics_options::parse(std::env::args().skip(1)).map_err(std::io::Error::other)?;
    let terrain_radius =
        terrain_options::startup_radius(std::env::args().skip(1)).map_err(std::io::Error::other)?;
    let autopilot_requested = std::env::args().any(|arg| arg == "--autopilot")
        || std::env::var("VOXY_AUTOPILOT").is_ok_and(|value| value == "1");
    let motion_compute = terrain_options::motion_device(std::env::args().skip(1))
        .map_err(std::io::Error::other)?
        .map(|ordinal| voxy_cuda::CudaCompute::new(ordinal, 16 * 1024 * 1024).map(Arc::new))
        .transpose()?;
    if let Some(compute) = &motion_compute {
        eprintln!("CUDA motion integration: {:?}", compute.capabilities()?);
        if std::env::args().any(|arg| arg == "--cuda-collisions") {
            eprintln!("Voxy character collision: CUDA broadphase and f64 contacts");
        }
    }
    let cuda_water = if std::env::args().any(|arg| arg == "--cuda-water") {
        let compute = motion_compute
            .as_ref()
            .ok_or("CUDA water device not initialized")?;
        eprintln!("Voxy water simulation: CUDA ordered transfers");
        Some(voxy_gpu::CudaWaterTransferProgram::new(compute.clone()))
    } else {
        None
    };
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let camcorder_mode = std::env::args()
        .any(|arg| matches!(arg.as_str(), "--camcorder" | "--videocam" | "--video"))
        || std::env::var("VOXY_CAMCORDER").is_ok_and(|value| value == "1");
    let clean_cam_mode = !camcorder_mode
        && (animals::check_requested()
            || std::env::args().any(|arg| matches!(arg.as_str(), "--cam" | "--clean-cam"))
            || std::env::var("VOXY_CAM").is_ok_and(|value| value == "1"));
    let bodycam_mode = camcorder_mode
        || clean_cam_mode
        || std::env::args().any(|arg| matches!(arg.as_str(), "--bodycam" | "--photoreal"))
        || std::env::var("VOXY_BODYCAM").is_ok_and(|value| value == "1");
    let profile = frame_profile::Profile::parse(std::env::args().skip(1), false)
        .map_err(std::io::Error::other)?;
    if let Some(p) = &profile {
        eprintln!("Voxy profile enabled: {p:?}");
    }
    if profile.is_some() && autopilot_requested {
        return Err("--profile and --autopilot are mutually exclusive".into());
    }
    let shadow_audit = std::env::var("VOXY_VEGETATION_SHADOW_ROUTE_AUDIT").as_deref() == Ok("1");
    if shadow_audit
        && (bodycam_mode
            || !profile
                .as_ref()
                .is_some_and(|profile| profile.scenario == frame_profile::Scenario::Water))
    {
        return Err("shadow route audit requires the orthographic water profile".into());
    }
    let mut app = VoxyApp {
        profile,
        shadow_audit,
        animal_check: animals::check_requested(),
        graphics,
        terrain_radius,
        motion_compute,
        cuda_water,
        cuda_projectiles: std::env::args().any(|arg| arg == "--cuda-projectiles"),
        cuda_character_motion: std::env::args().any(|arg| arg == "--cuda-character-motion"),
        cuda_collisions: std::env::args().any(|arg| arg == "--cuda-collisions"),
        cuda_vehicle_motion: std::env::args().any(|arg| arg == "--cuda-vehicle-motion"),
        bodycam_mode,
        clean_cam_mode,
        camcorder_mode,
        ..VoxyApp::default()
    };
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.startup_failure {
        return Err(std::io::Error::other(error).into());
    }
    if app.shadow_audit && app.shadow_audit_last_tick < 1200 {
        return Err("shadow route audit interrupted before tick 1200".into());
    }
    if !app.shadow_audit && app.profile.as_ref().is_some_and(|p| !p.is_complete()) {
        return Err("frame profile interrupted before report completion".into());
    }
    if autopilot_requested && app.autopilot_result.is_none() {
        return Err(std::io::Error::other("Voxy graphical autopilot did not complete").into());
    }
    if app.autopilot_result == Some(false) {
        return Err(std::io::Error::other("Voxy graphical autopilot failed").into());
    }
    Ok(())
}

#[cfg(test)]
mod terrain_gameplay_tests;
