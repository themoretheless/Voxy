mod controls;
mod graphics_options;
mod pet;
mod terrain_options;

use std::sync::Arc;
use std::time::{Duration, Instant};

use controls::{CameraRig, InputState};
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
use voxy_runtime::{BootstrapScene, build_procedural_scene, rebuild_bootstrap_chunks};
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
    graphics: voxy_render::GraphicsOptions,
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
    actor: Option<AnimatedActor>,
    last_update: Option<Instant>,
    accumulator: f32,
    input: InputState,
    camera: CameraRig,
    actor_position: Vec3,
    actor_yaw: f32,
    character: Option<CharacterState>,
    world: Option<World>,
    cursor_captured: bool,
    projectiles: Vec<ProjectileState>,
    resident_positions: Vec<ChunkPos>,
    camera_anchor: ChunkPos,
    derivation_epoch: u64,
    rebuild: Option<std::thread::JoinHandle<RebuildResult>>,
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
    pet: PetState,
    pet_ui_elapsed: f32,
    load_time_seconds: f32,
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

type RebuildResult = (
    u64,
    std::time::Duration,
    Result<Vec<voxy_runtime::BootstrapChunk>, voxy_runtime::BootstrapError>,
);

impl ApplicationHandler for VoxyApp {
    #[allow(clippy::too_many_lines)]
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let loading_started = Instant::now();
        let attributes = Window::default_attributes()
            .with_title(loading_title(0, "STARTING"))
            .with_visible(false)
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
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
                let scene_result = build_startup_scene(self.graphics);
                let scene = match scene_result {
                    Ok(scene) => scene,
                    Err(error) => {
                        eprintln!("Voxy world bootstrap failed: {error}");
                        self.startup_failure =
                            Some(format!("Voxy world bootstrap failed: {error}"));
                        event_loop.exit();
                        return;
                    }
                };
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
                let bind_matrices = match actor.skeleton.bind_pose().skin_matrices(&actor.skeleton)
                {
                    Ok(matrices) => matrices,
                    Err(error) => {
                        eprintln!("Voxy actor bind pose failed: {error}");
                        event_loop.exit();
                        return;
                    }
                };
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
        match event {
            WindowEvent::CloseRequested => {
                if self.autopilot_tick.is_some() && self.autopilot_result.is_none() {
                    eprintln!(
                        "Voxy autopilot interrupted by window close: tick={:?}, GPU character={}, GPU vehicle={}",
                        self.autopilot_tick, self.gpu_character_ticks, self.gpu_vehicle_ticks
                    );
                }
                event_loop.exit();
            }
            WindowEvent::Focused(false) => {
                self.input.clear_keyboard();
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
                #[allow(clippy::cast_precision_loss)]
                let width = self
                    .window
                    .as_ref()
                    .map_or(1.0, |window| window.inner_size().width.max(1) as f32);
                self.input.touch(touch, width);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                #[allow(clippy::cast_possible_truncation)]
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(position) => position.y as f32 / 40.0,
                };
                self.camera.zoom(lines);
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(message) = self.renderer.as_ref().and_then(Renderer::device_failure) {
                    self.startup_failure = Some(format!("Voxy GPU device lost: {message}"));
                    event_loop.exit();
                    return;
                }
                self.advance_animation(event_loop);
                self.poll_rebuild(event_loop);
                if let Some(renderer) = &mut self.renderer {
                    match renderer.render() {
                        Ok(outcome @ RenderOutcome::Presented) => {
                            self.presented_frames = self.presented_frames.saturating_add(1);
                            if !self.presented_once {
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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl VoxyApp {
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
        if std::env::args().any(|argument| argument == "--autopilot")
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

    fn advance_animation(&mut self, event_loop: &ActiveEventLoop) {
        const FIXED_DT: f32 = 1.0 / 60.0;
        const WALK_SPEED: f32 = 5.0;
        const SPRINT_SPEED: f32 = 8.5;
        let now = Instant::now();
        let elapsed = self
            .last_update
            .replace(now)
            .map_or(0.0, |last| now.duration_since(last).as_secs_f32().min(0.25));
        self.accumulator = (self.accumulator + elapsed).min(0.25);
        self.camera.apply_look(self.input.take_look());
        if self.input.take_fire()
            && let Err(error) = self.spawn_shot()
        {
            eprintln!("Voxy projectile launch failed: {error}");
        }
        while self.accumulator >= FIXED_DT {
            let saved = self.pending_character_intent.take();
            if saved.is_none() && !self.advance_autopilot(event_loop) {
                return;
            }
            let intent = saved.map_or_else(|| self.input.take_move(), |(intent, _)| intent);
            let (forward, right) = self.camera.planar_basis();
            let movement = if self.pet.can_move() {
                (right * intent.local.x + forward * intent.local.y).normalize_or_zero()
            } else {
                Vec3::ZERO
            };
            let movement = saved.map_or(movement, |(_, movement)| movement);
            let speed = if intent.sprint {
                SPRINT_SPEED
            } else {
                WALK_SPEED
            };
            if movement.length_squared() > f32::EPSILON {
                self.actor_yaw = movement.x.atan2(movement.z);
            }
            if self.driving {
                if !self.advance_vehicle(intent, FIXED_DT, event_loop) {
                    self.pending_character_intent = Some((intent, movement));
                    return;
                }
            } else {
                let (Some(world), Some(character)) = (&self.world, &mut self.character) else {
                    return;
                };
                let registry = world.registry();
                let config = CharacterConfig::default();
                let input = CharacterInput {
                    planar_velocity: [f64::from(movement.x * speed), f64::from(movement.z * speed)],
                    jump_pressed: intent.jump_pressed && self.pet.can_move(),
                };
                let dt = f64::from(FIXED_DT);
                let result = if let Some(program) = &self.gpu_collisions {
                    let Some(renderer) = &self.renderer else {
                        return;
                    };
                    if let Err(error) = program.poll_native() {
                        self.startup_failure = Some(error.to_string());
                        event_loop.exit();
                        return;
                    }
                    let started = self
                        .pending_character_started
                        .get_or_insert_with(Instant::now);
                    if started.elapsed() >= Duration::from_secs(30) {
                        self.startup_failure =
                            Some("GPU character step timed out after 30 seconds".into());
                        self.pending_character = None;
                        self.pending_character_intent = None;
                        self.pending_character_started = None;
                        event_loop.exit();
                        return;
                    }
                    let motion = if self.pending_character.is_none() && self.cuda_character_motion {
                        let Some(compute) = &self.motion_compute else {
                            return;
                        };
                        match voxy_app::cuda_motion::character_motion(
                            compute, character, input, dt, config,
                        ) {
                            Ok(motion) => Some(motion),
                            Err(error) => {
                                self.startup_failure = Some(error.to_string());
                                event_loop.exit();
                                return;
                            }
                        }
                    } else {
                        None
                    };
                    let task = self.pending_character.get_or_insert_with(|| {
                        let task = voxy_gpu::PendingGpuCharacter::new(
                            physics::CharacterState {
                                body: physics::AnchoredAabb {
                                    anchor: physics::Origin {
                                        x: character.body.anchor.x,
                                        y: character.body.anchor.y,
                                        z: character.body.anchor.z,
                                    },
                                    min: character.body.min,
                                    max: character.body.max,
                                },
                                velocity: character.velocity,
                                grounded: character.grounded,
                            },
                            input,
                            dt,
                            config,
                            registry,
                        );
                        if let Some(motion) = motion {
                            task.with_motion(motion.velocity, motion.displacement)
                        } else {
                            task
                        }
                    });
                    match task.try_step(program, renderer.queue(), world) {
                        Ok(None) => {
                            self.pending_character_intent = Some((intent, movement));
                            return;
                        }
                        Ok(Some((next, report))) => {
                            *character = CharacterState {
                                body: AnchoredAabb {
                                    anchor: VoxelPos {
                                        x: next.body.anchor.x,
                                        y: next.body.anchor.y,
                                        z: next.body.anchor.z,
                                    },
                                    min: next.body.min,
                                    max: next.body.max,
                                },
                                velocity: next.velocity,
                                grounded: next.grounded,
                            };
                            self.pending_character = None;
                            self.pending_character_started = None;
                            self.gpu_character_ticks += 1;
                            Ok(report)
                        }
                        Err(physics::CharacterError::Sweep(
                            voxy_gpu::CharacterGpuQueryError::Gpu(
                                voxy_gpu::GpuSweepError::StaleWorld,
                            ),
                        )) => {
                            self.pending_character = None;
                            self.pending_character_intent = Some((intent, movement));
                            return;
                        }
                        Err(error) => Err(error.to_string()),
                    }
                } else if self.cuda_collisions {
                    let Some(compute) = &self.motion_compute else {
                        return;
                    };
                    let result = voxy_app::cuda_motion::step_character_collision(
                        compute,
                        world,
                        registry,
                        character,
                        input,
                        dt,
                        config,
                        self.cuda_character_motion,
                    );
                    if result.is_ok() {
                        self.cuda_collision_ticks += 1;
                    }
                    result
                } else if self.cuda_character_motion {
                    let Some(compute) = &self.motion_compute else {
                        return;
                    };
                    voxy_app::cuda_motion::step_character(
                        compute, world, registry, character, input, dt, config,
                    )
                    .map_err(|error| error.to_string())
                } else {
                    step_character_in_field(
                        world,
                        registry,
                        character,
                        input,
                        dt,
                        config,
                        &[0.0, config.gravity, 0.0],
                    )
                    .map_err(|error| error.to_string())
                };
                if let Err(error) = result {
                    let message = format!("Voxy character physics failed: {error}");
                    eprintln!("{message}");
                    self.startup_failure = Some(message);
                    event_loop.exit();
                    return;
                }
                self.actor_position = character_position(character);
            }
            self.pet
                .tick(FIXED_DT, movement.length_squared() > f32::EPSILON);
            self.pet_ui_elapsed += FIXED_DT;
            if self.pet_ui_elapsed >= 0.5 {
                self.pet_ui_elapsed = 0.0;
                self.update_pet_title();
            }
            self.advance_projectiles(FIXED_DT, event_loop);
            self.advance_water(event_loop);
            if !self.advance_actor_animation(
                movement.length_squared() > f32::EPSILON,
                intent.sprint,
                FIXED_DT,
                event_loop,
            ) {
                return;
            }
            self.accumulator -= FIXED_DT;
        }
        let (desired_eye, target) = self.camera.eye_and_target(self.actor_position);
        let eye = self.world.as_ref().map_or(desired_eye, |world| {
            camera_collision(world, target, desired_eye)
        });
        if let Some(renderer) = &mut self.renderer
            && let Err(error) = renderer.update_camera(CameraView {
                eye,
                target,
                ..CameraView::default()
            })
        {
            eprintln!("Voxy camera update failed: {error}");
            event_loop.exit();
        }
    }

    fn spawn_shot(&mut self) -> Result<(), physics_voxel::ProjectileError> {
        let (eye, target) = self.camera.eye_and_target(self.actor_position);
        let direction = (target - eye).normalize_or_zero();
        let muzzle = target + direction;
        self.projectiles.push(spawn_projectile(
            ray_origin(muzzle),
            direction.to_array().map(f64::from),
            48.0,
        )?);
        Ok(())
    }

    fn advance_actor_animation(
        &mut self,
        moving: bool,
        sprinting: bool,
        dt: f32,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        let Some(actor) = &mut self.actor else {
            return false;
        };
        let animation_speed = if moving {
            if sprinting { 2.4 } else { 1.55 }
        } else {
            0.65
        };
        if let Err(error) = actor.animator.set_speed(animation_speed) {
            eprintln!("Voxy animation speed failed: {error}");
            event_loop.exit();
            return false;
        }
        let frame = match actor.animator.advance(&actor.skeleton, dt) {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!("Voxy animation failed: {error}");
                event_loop.exit();
                return false;
            }
        };
        if let Some(renderer) = &mut self.renderer {
            let model = Mat4::from_rotation_translation(
                Quat::from_rotation_y(self.actor_yaw),
                self.actor_position,
            );
            if let Err(error) = renderer
                .update_skin_matrices(&frame.skin_matrices)
                .and_then(|()| renderer.update_skinned_model(model))
            {
                eprintln!("Voxy animated actor update failed: {error}");
                event_loop.exit();
                return false;
            }
        }
        self.animation_frames = self.animation_frames.saturating_add(1);
        true
    }

    fn advance_projectiles(&mut self, dt: f32, event_loop: &ActiveEventLoop) {
        let motions = if let Some(compute) = self
            .motion_compute
            .as_ref()
            .filter(|_| self.cuda_projectiles)
        {
            if self.projectiles.is_empty() {
                None
            } else {
                let inputs: Vec<_> = self
                    .projectiles
                    .iter()
                    .map(|projectile| voxy_cuda::CudaProjectileInput {
                        velocity: projectile.velocity,
                        acceleration: ProjectileConfig::default().gravity,
                    })
                    .collect();
                match compute.projectile_motion(&inputs, f64::from(dt)) {
                    Ok(motions) => Some(motions),
                    Err(error) => {
                        let message = format!("Voxy CUDA projectile integration failed: {error}");
                        eprintln!("{message}");
                        self.startup_failure = Some(message);
                        event_loop.exit();
                        return;
                    }
                }
            }
        } else {
            None
        };
        let Some(world) = &mut self.world else {
            return;
        };
        let mut impacts = Vec::new();
        let mut index = 0;
        self.projectiles.retain_mut(|projectile| {
            let motion = motions.as_ref().map(|batch| batch[index]);
            index += 1;
            let result = if let Some(motion) = motion {
                physics_voxel::step_projectile_with_motion(
                    world,
                    projectile,
                    f64::from(dt),
                    ProjectileConfig::default(),
                    motion.velocity,
                    motion.displacement,
                )
            } else {
                step_projectile_in_field(
                    world,
                    projectile,
                    f64::from(dt),
                    ProjectileConfig::default(),
                    &ProjectileConfig::default().gravity,
                )
            };
            match result {
                Ok(ProjectileOutcome::Flying) => true,
                Ok(ProjectileOutcome::Impact(hit)) => {
                    impacts.push(hit);
                    false
                }
                Ok(
                    ProjectileOutcome::Expired
                    | ProjectileOutcome::Unloaded { .. }
                    | ProjectileOutcome::Unavailable { .. }
                    | ProjectileOutcome::StepBudgetExhausted,
                ) => false,
                Err(error) => {
                    eprintln!("Voxy projectile physics failed: {error}");
                    false
                }
            }
        });
        let mut world_changed = false;
        for hit in impacts {
            let plan = plan_impact_explosion(
                world,
                world.registry(),
                EditSource::Player(1),
                hit,
                Explosion {
                    radius: 2,
                    power: 100,
                    attenuation_per_squared_voxel: 12,
                    ..Explosion::default()
                },
            );
            match plan {
                Ok(DestructionPlan::Transaction(transaction)) => match world.commit(transaction) {
                    Ok(receipt) => {
                        if let Some(states) = self.water_states {
                            wake_water_after_edits(
                                world,
                                states,
                                &receipt.inverse.writes,
                                &mut self.water_active,
                            );
                        }
                        world_changed = true;
                        self.explosion_commits += 1;
                    }
                    Err(error) => {
                        eprintln!("Voxy explosion commit failed: {error}");
                        event_loop.exit();
                        return;
                    }
                },
                Ok(DestructionPlan::NoOp) => {}
                Err(error) => eprintln!("Voxy explosion planning failed: {error}"),
            }
        }
        if world_changed {
            self.refresh_world_geometry(event_loop);
        }
    }

    fn advance_vehicle(
        &mut self,
        intent: controls::MoveIntent,
        dt: f32,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        let (Some(world), Some(vehicle)) = (&self.world, &mut self.vehicle) else {
            return false;
        };
        let input = VehicleInput {
            throttle: f64::from(intent.local.y),
            steering: f64::from(intent.local.x),
            brake: intent.jump_pressed,
        };
        let config = VehicleConfig::default();
        let step = if self.gpu_collisions.is_some() {
            let program = self.gpu_collisions.as_ref().unwrap();
            let Some(renderer) = &self.renderer else {
                return false;
            };
            let started = self
                .pending_vehicle_started
                .get_or_insert_with(Instant::now);
            if started.elapsed() >= Duration::from_secs(30) {
                self.startup_failure = Some("GPU vehicle step timed out after 30 seconds".into());
                self.pending_vehicle = None;
                self.pending_vehicle_started = None;
                event_loop.exit();
                return false;
            }
            if let Err(error) = program.poll_native() {
                self.startup_failure = Some(error.to_string());
                event_loop.exit();
                return false;
            }
            let task = self.pending_vehicle.get_or_insert_with(|| {
                voxy_gpu::PendingGpuVehicle::new(
                    *vehicle,
                    input,
                    f64::from(dt),
                    config,
                    world.registry(),
                )
            });
            match task.try_step_with_integrator(
                program,
                renderer.queue(),
                world,
                |state, input, dt, config| {
                    if !self.cuda_vehicle_motion {
                        return Ok(None);
                    }
                    let compute = self
                        .motion_compute
                        .as_ref()
                        .ok_or_else(|| "CUDA compute unavailable".to_string())?;
                    voxy_app::cuda_motion::character_motion(compute, state, input, dt, config)
                        .map(|motion| Some((motion.velocity, motion.displacement)))
                        .map_err(|error| error.to_string())
                },
            ) {
                Ok(None) => return false,
                Ok(Some((next, report))) => {
                    *vehicle = next;
                    self.pending_vehicle = None;
                    self.pending_vehicle_started = None;
                    self.gpu_vehicle_ticks += 1;
                    Ok(report)
                }
                Err(voxy_gpu::VehicleGpuError::Character(physics::CharacterError::Sweep(
                    voxy_gpu::CharacterGpuQueryError::Gpu(voxy_gpu::GpuSweepError::StaleWorld),
                ))) => {
                    self.pending_vehicle = None;
                    return false;
                }
                Err(error) => Err(voxy_app::cuda_motion::MotionError::Collision(
                    error.to_string(),
                )),
            }
        } else if self.cuda_vehicle_motion || self.cuda_collisions {
            let Some(compute) = &self.motion_compute else {
                return false;
            };
            physics_voxel::step_vehicle_with_chassis_step(
                vehicle,
                input,
                f64::from(dt),
                config,
                |chassis, input, dt, config| {
                    if self.cuda_collisions {
                        voxy_app::cuda_motion::step_character_collision(
                            compute,
                            world,
                            world.registry(),
                            chassis,
                            input,
                            dt,
                            config,
                            self.cuda_vehicle_motion,
                        )
                        .map_err(voxy_app::cuda_motion::MotionError::Collision)
                    } else {
                        voxy_app::cuda_motion::step_character(
                            compute,
                            world,
                            world.registry(),
                            chassis,
                            input,
                            dt,
                            config,
                        )
                    }
                },
            )
        } else {
            step_vehicle(
                world,
                world.registry(),
                vehicle,
                input,
                f64::from(dt),
                config,
            )
            .map_err(voxy_app::cuda_motion::MotionError::from)
        };
        let step = match step {
            Ok(step) => step,
            Err(error) => {
                let message = format!("Voxy vehicle physics failed: {error}");
                eprintln!("{message}");
                self.startup_failure = Some(message);
                event_loop.exit();
                return false;
            }
        };
        self.actor_position = character_position(&vehicle.chassis);
        self.peak_vehicle_speed = self
            .peak_vehicle_speed
            .max(vehicle.longitudinal_speed.abs());
        #[allow(clippy::cast_possible_truncation)]
        {
            self.actor_yaw = vehicle.heading as f32;
        }
        if let Some(track) = &self.race_track {
            match update_race(
                track,
                &mut self.race_progress,
                step.previous_center,
                step.current_center,
                f64::from(dt),
            ) {
                Ok(true) if self.race_progress.finished => {
                    println!("Voxy race finished: {:.2}s", self.race_progress.elapsed);
                }
                Ok(true) => println!(
                    "Voxy checkpoint: {}/{} lap {}",
                    self.race_progress.next_checkpoint,
                    track.checkpoints().len(),
                    self.race_progress.completed_laps + 1
                ),
                Ok(false) => {}
                Err(error) => {
                    eprintln!("Voxy race progress failed: {error}");
                    event_loop.exit();
                }
            }
        }
        true
    }

    fn refresh_world_geometry(&mut self, event_loop: &ActiveEventLoop) {
        let Some(next_epoch) = self.derivation_epoch.checked_add(1) else {
            eprintln!("Voxy derived-data epoch exhausted");
            event_loop.exit();
            return;
        };
        self.derivation_epoch = next_epoch;
        self.rebuild_pending = Some(next_epoch);
        self.start_rebuild();
    }

    fn start_rebuild(&mut self) {
        if self.rebuild.is_some() || self.rebuild_pending.is_none() {
            return;
        }
        let Some(world) = self.world.clone() else {
            return;
        };
        let positions = self.resident_positions.clone();
        let epoch = self.derivation_epoch;
        self.rebuild_pending = None;
        self.rebuild = Some(std::thread::spawn(move || {
            let started = Instant::now();
            let result = rebuild_bootstrap_chunks(&world, &positions, epoch);
            (epoch, started.elapsed(), result)
        }));
    }

    fn poll_rebuild(&mut self, event_loop: &ActiveEventLoop) {
        if !self
            .rebuild
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
        {
            return;
        }
        let Some(worker) = self.rebuild.take() else {
            return;
        };
        let Ok((epoch, duration, result)) = worker.join() else {
            eprintln!("Voxy geometry worker panicked");
            event_loop.exit();
            return;
        };
        if epoch != self.derivation_epoch {
            self.start_rebuild();
            return;
        }
        let chunks = match result {
            Ok(chunks) => chunks,
            Err(error) => {
                eprintln!("Voxy post-destruction rebuild failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let uploads = chunks
            .iter()
            .map(|chunk| (chunk.pos, &chunk.mesh, &chunk.light))
            .collect::<Vec<_>>();
        if let Some(renderer) = &mut self.renderer
            && let Err(error) = renderer.upload_lit_chunks(&uploads, self.camera_anchor)
        {
            eprintln!("Voxy post-destruction upload failed: {error}");
            event_loop.exit();
            return;
        }
        println!(
            "Voxy background rebuild: {:.2}ms",
            duration.as_secs_f64() * 1000.0
        );
    }

    fn advance_water(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu_water.is_some() {
            if let Err(error) = self.advance_gpu_water(event_loop) {
                self.startup_failure = Some(format!("Voxy GPU water failed: {error}"));
                event_loop.exit();
            }
            return;
        }
        self.water_tick_divider = (self.water_tick_divider + 1) % 60;
        if self.water_tick_divider != 0 || self.water_active.is_empty() {
            return;
        }
        let (Some(world), Some(states)) = (&mut self.world, self.water_states) else {
            return;
        };
        let budget = WaterBudget::default();
        let deferred = self
            .water_active
            .split_off(self.water_active.len().min(budget.max_active));
        let plan = if let Some(program) = &self.cuda_water {
            let outcome = program
                .plan_world(
                    world,
                    world.registry(),
                    states,
                    &self.water_active,
                    EditSource::Simulation,
                    budget,
                )
                .map_err(|error| error.to_string());
            if outcome.is_ok() {
                self.cuda_water_ticks += 1;
            }
            outcome
        } else {
            step_water(
                world,
                world.registry(),
                states,
                &self.water_active,
                EditSource::Simulation,
                budget,
            )
            .map_err(|error| error.to_string())
        };
        let changed = match plan {
            Ok(WaterPlan::Settled) => {
                self.water_active = deferred;
                false
            }
            Ok(WaterPlan::Transaction { edit, next_active }) => match world.commit(edit) {
                Ok(_) => {
                    self.water_active = deferred;
                    self.water_active.extend(next_active.into_vec());
                    let mut seen = std::collections::BTreeSet::new();
                    self.water_active.retain(|position| seen.insert(*position));
                    self.water_commits += 1;
                    true
                }
                Err(error) => {
                    self.startup_failure = Some(format!("Voxy water commit failed: {error}"));
                    eprintln!("Voxy water commit failed: {error}");
                    event_loop.exit();
                    return;
                }
            },
            Err(error) => {
                self.startup_failure = Some(format!("Voxy water simulation failed: {error}"));
                eprintln!("Voxy water simulation failed: {error}");
                event_loop.exit();
                return;
            }
        };
        if changed {
            self.refresh_world_geometry(event_loop);
        }
    }

    fn advance_gpu_water(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let program = self
            .gpu_water
            .as_ref()
            .ok_or("GPU water program unavailable")?;
        program.poll_native().map_err(|error| error.to_string())?;
        if let Some(mut work) = self.pending_water.take() {
            let outcome = work.plan.try_plan().map_err(|error| error.to_string())?;
            let Some(plan) = outcome else {
                if work.started.elapsed() > Duration::from_secs(30) {
                    return Err("GPU water readback timed out".into());
                }
                self.pending_water = Some(work);
                return Ok(());
            };
            self.gpu_water_ticks += 1;
            if self.gpu_water_ticks == 1 {
                println!(
                    "Voxy first GPU water tick completed: {} active cells",
                    work.active.len()
                );
            }
            let world = self.world.as_mut().ok_or("GPU water world unavailable")?;
            let outcome = publish_water_plan(world, &mut self.water_active, work.active, plan)?;
            if outcome == WaterPublication::Retry {
                self.water_tick_divider = 59;
            }
            if outcome == WaterPublication::Committed {
                self.water_commits += 1;
                self.refresh_world_geometry(event_loop);
            }
            return Ok(());
        }
        self.water_tick_divider = (self.water_tick_divider + 1) % 60;
        if self.water_tick_divider != 0 || self.water_active.is_empty() {
            return Ok(());
        }
        let world = self.world.as_ref().ok_or("GPU water world unavailable")?;
        let states = self.water_states.ok_or("GPU water states unavailable")?;
        let renderer = self
            .renderer
            .as_ref()
            .ok_or("GPU water renderer unavailable")?;
        let budget = WaterBudget::default();
        self.water_active.sort_unstable();
        self.water_active.dedup();
        let active: Vec<_> = self
            .water_active
            .drain(..self.water_active.len().min(budget.max_active))
            .collect();
        let plan = program
            .begin_plan_world(
                renderer.queue(),
                world,
                world.registry(),
                states,
                &active,
                EditSource::Simulation,
                budget,
            )
            .map_err(|error| error.to_string())?;
        self.pending_water = Some(PendingGameWater {
            plan,
            active,
            started: Instant::now(),
        });
        Ok(())
    }

    fn release_autopilot_water(&mut self) -> Result<(), String> {
        let bed = self
            .autopilot_water_bed
            .ok_or("autopilot has no natural water bed")?;
        let world = self.world.as_mut().ok_or("autopilot world unavailable")?;
        let states = self
            .water_states
            .ok_or("autopilot water states unavailable")?;
        let plan = physics_voxel::plan_explosion(
            world,
            world.registry(),
            EditSource::Player(1),
            Explosion {
                center: bed,
                radius: 0,
                ..Explosion::default()
            },
        )
        .map_err(|error| error.to_string())?;
        let DestructionPlan::Transaction(edit) = plan else {
            return Err("autopilot water bed no longer destructible".into());
        };
        let receipt = world.commit(edit).map_err(|error| error.to_string())?;
        wake_water_after_edits(
            world,
            states,
            &receipt.inverse.writes,
            &mut self.water_active,
        );
        self.explosion_commits += 1;
        println!("Voxy autopilot destroyed natural water bed at {bed:?}");
        Ok(())
    }

    fn advance_autopilot(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let Some(current_tick) = self.autopilot_tick else {
            return true;
        };
        let Some(tick) = current_tick.checked_add(1) else {
            eprintln!("Voxy autopilot tick overflow");
            event_loop.exit();
            return false;
        };
        self.autopilot_tick = Some(tick);
        apply_autopilot_controls(&mut self.input, tick);
        match tick {
            1 => {
                self.input.mouse_motion((120.0, -35.0));
            }
            70 => self.input.key(KeyCode::Space, ElementState::Pressed, false),
            71 => self
                .input
                .key(KeyCode::Space, ElementState::Released, false),
            100 | 360 => self.input.queue_fire(),
            180 => self.toggle_driving(),
            240 => {
                if let Err(error) = self.release_autopilot_water() {
                    self.startup_failure = Some(error);
                    event_loop.exit();
                    return false;
                }
                self.refresh_world_geometry(event_loop);
            }
            600 => {
                let camera_moved = (self.camera.yaw - self.autopilot_initial_yaw).abs() > 0.01;
                let character_moved = self.actor_position.distance(self.autopilot_start) > 0.5;
                let passed = self.presented_frames >= 60
                    && camera_moved
                    && character_moved
                    && self.water_commits > 0
                    && self.explosion_commits > 0
                    && self.peak_vehicle_speed > 0.1
                    && self.animation_frames > 500;
                self.autopilot_result = Some(passed);
                println!(
                    "Voxy CUDA collision ticks completed: {}",
                    self.cuda_collision_ticks
                );
                println!("Voxy GPU water ticks completed: {}", self.gpu_water_ticks);
                println!("Voxy CUDA water ticks completed: {}", self.cuda_water_ticks);
                println!(
                    "Voxy GPU vehicle ticks completed: {}",
                    self.gpu_vehicle_ticks
                );
                println!(
                    "Voxy GPU character ticks completed: {}",
                    self.gpu_character_ticks
                );
                if passed {
                    println!(
                        "Voxy autopilot passed: water={}, explosions={}, peak_vehicle={:.2}, animation_frames={}, presented_frames={}",
                        self.water_commits,
                        self.explosion_commits,
                        self.peak_vehicle_speed,
                        self.animation_frames,
                        self.presented_frames
                    );
                } else {
                    eprintln!(
                        "Voxy autopilot failed: camera={camera_moved}, movement={character_moved}, water={}, explosions={}, peak_vehicle={:.2}, animation_frames={}, presented_frames={}",
                        self.water_commits,
                        self.explosion_commits,
                        self.peak_vehicle_speed,
                        self.animation_frames,
                        self.presented_frames
                    );
                }
                event_loop.exit();
                return false;
            }
            _ => {}
        }
        true
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn character_position(character: &CharacterState) -> Vec3 {
    Vec3::new(
        character.body.anchor.x as f32
            + ((character.body.min[0] + character.body.max[0]) * 0.5) as f32,
        character.body.anchor.y as f32 + character.body.min[1] as f32,
        character.body.anchor.z as f32
            + ((character.body.min[2] + character.body.max[2]) * 0.5) as f32,
    )
}

#[allow(clippy::cast_possible_truncation)]
fn camera_collision(world: &World, target: Vec3, desired_eye: Vec3) -> Vec3 {
    let offset = desired_eye - target;
    let distance = offset.length();
    if distance <= f32::EPSILON {
        return desired_eye;
    }
    let floor = target.floor();
    let origin = RayOrigin {
        voxel: VoxelPos {
            x: floor.x as i64,
            y: floor.y as i64,
            z: floor.z as i64,
        },
        offset: [
            f64::from(target.x - floor.x),
            f64::from(target.y - floor.y),
            f64::from(target.z - floor.z),
        ],
    };
    let direction = offset / distance;
    let result = raycast(
        world,
        origin,
        direction.to_array().map(f64::from),
        RaycastConfig {
            max_distance: f64::from(distance),
            max_steps: 128,
        },
    );
    match result {
        Ok(RaycastResult::Hit(hit)) => target + direction * (hit.distance as f32 - 0.3).max(0.75),
        _ => desired_eye,
    }
}

#[allow(clippy::cast_possible_truncation)]
fn ray_origin(position: Vec3) -> RayOrigin {
    let floor = position.floor();
    RayOrigin {
        voxel: VoxelPos {
            x: floor.x as i64,
            y: floor.y as i64,
            z: floor.z as i64,
        },
        offset: [
            f64::from(position.x - floor.x),
            f64::from(position.y - floor.y),
            f64::from(position.z - floor.z),
        ],
    }
}

fn apply_autopilot_controls(input: &mut InputState, tick: u64) {
    for (key, pressed) in [
        (KeyCode::KeyW, (1..600).contains(&tick)),
        (KeyCode::KeyA, (181..240).contains(&tick)),
    ] {
        input.key(
            key,
            if pressed {
                ElementState::Pressed
            } else {
                ElementState::Released
            },
            false,
        );
    }
}

/// Spawn above every collidable cell under the actor's straddling AABB footprint.
fn terrain_spawn(world: &World) -> Result<VoxelPos, String> {
    use voxy_world::{CollisionShape, Sample, VoxelView};
    let mut surface = None;
    for x in [15, 16] {
        for z in [15, 16] {
            let mut height = None;
            for y in (-32..=63).rev() {
                let block = match world.sample(VoxelPos { x, y, z }) {
                    Sample::Loaded(block) => block,
                    _ => return Err("spawn column is not fully loaded".into()),
                };
                let definition = world.registry().get(block).ok_or("unknown spawn block")?;
                if definition.collision != CollisionShape::Empty {
                    height = Some(y);
                    break;
                }
            }
            let height = height.ok_or("spawn has no supporting terrain")?;
            surface = Some(surface.map_or(height, |prior: i64| prior.max(height)));
        }
    }
    let y = surface.ok_or("spawn footprint is empty")? + 1;
    if y > 61 {
        return Err("spawn lacks loaded headroom".into());
    }
    Ok(VoxelPos { x: 16, y, z: 16 })
}

fn terrain_water_cells(
    scene: &BootstrapScene,
    states: WaterStates,
) -> Result<Vec<VoxelPos>, String> {
    use voxy_world::VoxelView;
    let mut cells = Vec::new();
    for chunk in &scene.chunks {
        let snapshot = scene
            .world
            .chunk(chunk.pos)
            .ok_or("water chunk is not loaded")?;
        let base_x = chunk
            .pos
            .x
            .checked_mul(32)
            .ok_or("water coordinate overflow")?;
        let base_y = chunk
            .pos
            .y
            .checked_mul(32)
            .ok_or("water coordinate overflow")?;
        let base_z = chunk
            .pos
            .z
            .checked_mul(32)
            .ok_or("water coordinate overflow")?;
        for y in 0..32_u8 {
            for z in 0..32_u8 {
                for x in 0..32_u8 {
                    let local =
                        voxy_world::LocalPos::new(x, y, z).map_err(|error| error.to_string())?;
                    if states.0.contains(&snapshot.data.blocks.get(local.index())) {
                        cells.push(VoxelPos {
                            x: base_x + i64::from(x),
                            y: base_y + i64::from(y),
                            z: base_z + i64::from(z),
                        });
                    }
                }
            }
        }
    }
    Ok(cells)
}

#[derive(Debug, Eq, PartialEq)]
enum WaterPublication {
    Settled,
    Committed,
    Retry,
}

fn publish_water_plan(
    world: &mut World,
    active: &mut Vec<VoxelPos>,
    submitted: Vec<VoxelPos>,
    plan: WaterPlan,
) -> Result<WaterPublication, String> {
    let outcome = match plan {
        WaterPlan::Settled => WaterPublication::Settled,
        WaterPlan::Transaction { edit, next_active } => match world.commit(edit) {
            Ok(_) => {
                active.extend(next_active.into_vec());
                WaterPublication::Committed
            }
            Err(voxy_world::CommitError::RevisionConflict { .. }) => {
                active.extend(submitted);
                WaterPublication::Retry
            }
            Err(error) => return Err(error.to_string()),
        },
    };
    active.sort_unstable();
    active.dedup();
    Ok(outcome)
}

/// Select existing submerged terrain away from resident boundaries. The
/// autopilot destroys only this real support block to exercise edit-triggered flow.
fn natural_water_bed(world: &World, states: WaterStates, water: &[VoxelPos]) -> Option<VoxelPos> {
    use voxy_world::{BlockStateId, Sample, VoxelView};
    water.iter().find_map(|&source| {
        let bed = VoxelPos {
            y: source.y.checked_sub(1)?,
            ..source
        };
        let Sample::Loaded(block) = world.sample(bed) else {
            return None;
        };
        if block == BlockStateId::AIR || states.0.contains(&block) {
            return None;
        }
        // Flow can wake several neighbors over later ticks; keep a loaded margin.
        for dx in -2..=2 {
            for dy in -2..=2 {
                for dz in -2..=2 {
                    let pos = VoxelPos {
                        x: bed.x.checked_add(dx)?,
                        y: bed.y.checked_add(dy)?,
                        z: bed.z.checked_add(dz)?,
                    };
                    if !matches!(world.sample(pos), Sample::Loaded(_)) {
                        return None;
                    }
                }
            }
        }
        match physics_voxel::plan_explosion(
            world,
            world.registry(),
            EditSource::Player(1),
            Explosion {
                center: bed,
                radius: 0,
                ..Explosion::default()
            },
        ) {
            Ok(DestructionPlan::Transaction(_)) => Some(bed),
            _ => None,
        }
    })
}

/// Reactivate existing water touching committed edits, preserving pending work.
fn wake_water_after_edits(
    world: &World,
    states: WaterStates,
    changed: &[voxy_world::VoxelWrite],
    active: &mut Vec<VoxelPos>,
) {
    use voxy_world::{Sample, VoxelView};
    let mut seen: std::collections::BTreeSet<_> = active.iter().copied().collect();
    for edit in changed {
        for (dx, dy, dz) in [
            (0, 0, 0),
            (-1, 0, 0),
            (1, 0, 0),
            (0, -1, 0),
            (0, 1, 0),
            (0, 0, -1),
            (0, 0, 1),
        ] {
            let (Some(x), Some(y), Some(z)) = (
                edit.pos.x.checked_add(dx),
                edit.pos.y.checked_add(dy),
                edit.pos.z.checked_add(dz),
            ) else {
                continue;
            };
            let position = VoxelPos { x, y, z };
            if let Sample::Loaded(block) = world.sample(position)
                && states.0.contains(&block)
                && seen.insert(position)
            {
                active.push(position);
            }
        }
    }
}

fn bootstrap_water_states(world: &World) -> Result<WaterStates, String> {
    let mut states = Vec::with_capacity(8);
    for level in 1..=8 {
        let key =
            ResourceKey::parse(format!("voxy:water_{level}")).map_err(|error| error.to_string())?;
        states.push(
            world
                .registry()
                .find(&key)
                .ok_or_else(|| format!("missing built-in state {key:?}"))?,
        );
    }
    let states: [_; 8] = states
        .try_into()
        .map_err(|_| "invalid built-in water state count".to_owned())?;
    WaterStates(states)
        .validate(world.registry())
        .map_err(|error| error.to_string())
}

fn bootstrap_actor() -> Result<(AnimatedActor, SkinnedMesh), Box<dyn std::error::Error>> {
    let joints = [
        ("root", Vec3::ZERO),
        ("left-leg", Vec3::new(-0.28, 1.05, 0.0)),
        ("right-leg", Vec3::new(0.28, 1.05, 0.0)),
        ("left-arm", Vec3::new(-0.68, 2.35, 0.0)),
        ("right-arm", Vec3::new(0.68, 2.35, 0.0)),
    ];
    let skeleton = Skeleton::new(
        joints
            .into_iter()
            .enumerate()
            .map(|(index, (name, pivot))| Joint {
                name: Arc::from(name),
                parent: (index != 0).then_some(0),
                bind_local: Transform {
                    translation: pivot,
                    ..Transform::IDENTITY
                },
                inverse_bind: Mat4::from_translation(-pivot),
            })
            .collect(),
    )?;
    let clip = Arc::new(AnimationClip::new(
        "walk-cycle",
        1.0,
        Playback::Loop,
        vec![
            JointTrack::default(),
            limb_swing(0.62),
            limb_swing(-0.62),
            limb_swing(-0.52),
            limb_swing(0.52),
        ],
        &skeleton,
    )?);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    push_actor_box(
        &mut vertices,
        &mut indices,
        [-0.5, 1.0, -0.28],
        [0.5, 2.5, 0.28],
        0,
    );
    push_actor_box(
        &mut vertices,
        &mut indices,
        [-0.46, 2.5, -0.36],
        [0.46, 3.38, 0.36],
        0,
    );
    push_actor_box(
        &mut vertices,
        &mut indices,
        [-0.5, 0.0, -0.24],
        [-0.06, 1.12, 0.24],
        1,
    );
    push_actor_box(
        &mut vertices,
        &mut indices,
        [0.06, 0.0, -0.24],
        [0.5, 1.12, 0.24],
        2,
    );
    push_actor_box(
        &mut vertices,
        &mut indices,
        [-0.82, 1.05, -0.2],
        [-0.5, 2.4, 0.2],
        3,
    );
    push_actor_box(
        &mut vertices,
        &mut indices,
        [0.5, 1.05, -0.2],
        [0.82, 2.4, 0.2],
        4,
    );
    let mesh = SkinnedMesh::new(vertices, indices, 5)?;
    Ok((
        AnimatedActor {
            skeleton,
            animator: Animator::new(clip),
        },
        mesh,
    ))
}

fn limb_swing(amount: f32) -> JointTrack {
    JointTrack {
        rotations: [amount, -amount, amount]
            .into_iter()
            .zip([0.0, 0.5, 1.0])
            .map(|(angle, time)| QuatKey {
                time,
                value: Quat::from_rotation_x(angle),
            })
            .collect(),
        ..JointTrack::default()
    }
}

fn push_actor_box(
    vertices: &mut Vec<SkinnedVertex>,
    indices: &mut Vec<u32>,
    min: [f32; 3],
    max: [f32; 3],
    joint: u16,
) {
    let corners = [
        [min[0], min[1], min[2]],
        [max[0], min[1], min[2]],
        [max[0], max[1], min[2]],
        [min[0], max[1], min[2]],
        [min[0], min[1], max[2]],
        [max[0], min[1], max[2]],
        [max[0], max[1], max[2]],
        [min[0], max[1], max[2]],
    ];
    let faces = [
        ([0, 3, 2, 1], [0.0, 0.0, -1.0]),
        ([4, 5, 6, 7], [0.0, 0.0, 1.0]),
        ([0, 4, 7, 3], [-1.0, 0.0, 0.0]),
        ([1, 2, 6, 5], [1.0, 0.0, 0.0]),
        ([0, 1, 5, 4], [0.0, -1.0, 0.0]),
        ([3, 7, 6, 2], [0.0, 1.0, 0.0]),
    ];
    for (face, normal) in faces {
        let start = u32::try_from(vertices.len()).expect("actor mesh stays small");
        for (corner, uv) in face
            .into_iter()
            .zip([[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]])
        {
            vertices.push(actor_vertex(corners[corner], normal, uv, joint));
        }
        indices.extend([start, start + 1, start + 2, start, start + 2, start + 3]);
    }
}

fn actor_vertex(position: [f32; 3], normal: [f32; 3], uv: [f32; 2], joint: u16) -> SkinnedVertex {
    SkinnedVertex {
        position,
        normal,
        uv,
        joints: [joint, 0, 0, 0],
        weights: [u16::MAX, 0, 0, 0],
    }
}

fn loading_title(percent: u8, stage: &str) -> String {
    let percent = percent.min(100);
    let filled = usize::from(percent / 5);
    let empty = 20_usize.saturating_sub(filled);
    format!(
        "Voxy Tamagotchi |{}{}| {percent:3}%  {stage}",
        "█".repeat(filled),
        "░".repeat(empty)
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.first().is_some_and(|arg| arg == "--game-package") {
        let path = arguments.get(1).ok_or("--game-package requires a path")?;
        let mode = match arguments.get(2).map(String::as_str) {
            None | Some("--game") => voxy_editor::ViewportMode::Game,
            Some("--game-check") => voxy_editor::ViewportMode::GameCheck,
            Some("--game-native-smoke") => voxy_editor::ViewportMode::GameSmoke,
            _ => return Err("unsupported packaged game mode".into()),
        };
        if arguments.len() > 3 {
            return Err("unexpected packaged game arguments".into());
        }
        return voxy_editor::run_packaged_game(std::path::Path::new(path), mode);
    }

    if let Some(command) = arguments
        .first()
        .filter(|arg| matches!(arg.as_str(), "--model" | "--model-manifest"))
    {
        let path = arguments.get(1).ok_or("missing model/manifest path")?;
        let source = if command == "--model-manifest" {
            voxy_editor::ModelSource::Manifest {
                path: path.into(),
                asset: voxy_assets::AssetId(
                    arguments.get(2).ok_or("missing logical asset ID")?.clone(),
                ),
            }
        } else {
            voxy_editor::ModelSource::File(path.into())
        };
        let scene = arguments
            .iter()
            .position(|arg| arg == "--scene")
            .map(|index| {
                arguments
                    .get(index + 1)
                    .ok_or("--scene requires a path")
                    .map(std::path::Path::new)
            })
            .transpose()?;
        if let Some(index) = arguments.iter().position(|arg| arg == "--export-game") {
            let output = arguments
                .get(index + 1)
                .ok_or("--export-game requires an output path")?;
            return voxy_editor::export_game_package(
                &source,
                scene.ok_or("--export-game requires --scene")?,
                std::path::Path::new(output),
            );
        }
        let mode = if arguments.iter().any(|arg| arg == "--animation-native-smoke") {
            voxy_editor::ViewportMode::AnimationSmoke
        } else if arguments.iter().any(|arg| arg == "--game-native-smoke") {
            voxy_editor::ViewportMode::GameSmoke
        } else if arguments.iter().any(|arg| arg == "--game-check") {
            voxy_editor::ViewportMode::GameCheck
        } else if arguments.iter().any(|arg| arg == "--game") {
            voxy_editor::ViewportMode::Game
        } else if arguments.iter().any(|arg| arg == "--smoke") {
            voxy_editor::ViewportMode::Smoke
        } else {
            voxy_editor::ViewportMode::Interactive
        };
        return voxy_editor::run_model_viewport_with_scene(&source, mode, scene);
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
    let mut app = VoxyApp {
        graphics,
        motion_compute,
        cuda_water,
        cuda_projectiles: std::env::args().any(|arg| arg == "--cuda-projectiles"),
        cuda_character_motion: std::env::args().any(|arg| arg == "--cuda-character-motion"),
        cuda_collisions: std::env::args().any(|arg| arg == "--cuda-collisions"),
        cuda_vehicle_motion: std::env::args().any(|arg| arg == "--cuda-vehicle-motion"),
        ..VoxyApp::default()
    };
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.startup_failure {
        return Err(std::io::Error::other(error).into());
    }
    if autopilot_requested && app.autopilot_result.is_none() {
        return Err(std::io::Error::other("Voxy graphical autopilot did not complete").into());
    }
    if app.autopilot_result == Some(false) {
        return Err(std::io::Error::other("Voxy graphical autopilot failed").into());
    }
    Ok(())
}

fn build_startup_scene(
    graphics: voxy_render::GraphicsOptions,
) -> Result<BootstrapScene, voxy_runtime::BootstrapError> {
    let backend =
        terrain_options::TerrainBackend::parse(std::env::args().skip(1)).map_err(|error| {
            eprintln!("Terrain selection failed: {error}");
            voxy_world::GenerationError::BackendFailure
        })?;
    if backend == terrain_options::TerrainBackend::Cpu {
        return build_procedural_scene(0x56_4f_58_59, 1);
    }
    voxy_runtime::build_generated_scene(0x56_4f_58_59, 1, |palette, water| {
        if let terrain_options::TerrainBackend::Cuda { ordinal } = backend {
            let generator =
                voxy_gpu::CudaTerrainGenerator::new(ordinal, palette, water).map_err(|error| {
                    eprintln!("CUDA terrain initialization failed: {error}");
                    voxy_world::GenerationError::BackendFailure
                })?;
            eprintln!("CUDA terrain: {:?}", generator.capabilities());
            Ok(Box::new(generator))
        } else {
            let generator =
                pollster::block_on(voxy_gpu::GpuTerrainGenerator::new(graphics, palette, water))
                    .map_err(|error| {
                        eprintln!("GPU terrain initialization failed: {error}");
                        voxy_world::GenerationError::BackendFailure
                    })?;
            eprintln!("GPU terrain: {:?}", generator.capabilities().adapter);
            Ok(Box::new(generator))
        }
    })
}

#[cfg(test)]
mod terrain_gameplay_tests {
    use super::*;
    use voxy_world::{CollisionShape, Sample, VoxelView};

    #[test]
    fn scripted_movement_recovers_after_focus_clear() {
        let mut input = InputState::default();
        apply_autopilot_controls(&mut input, 200);
        assert!(input.take_move().local.length() > 0.9);
        input.clear_keyboard();
        assert_eq!(input.take_move().local, glam::Vec2::ZERO);
        apply_autopilot_controls(&mut input, 200);
        let movement = input.take_move().local;
        assert!(movement.x < 0.0 && movement.y > 0.0);
        apply_autopilot_controls(&mut input, 241);
        assert_eq!(input.take_move().local, glam::Vec2::Y);
        apply_autopilot_controls(&mut input, 600);
        assert_eq!(input.take_move().local, glam::Vec2::ZERO);
    }

    #[test]
    fn pending_water_conflict_preserves_new_activations_and_retries_sources() {
        let mut scene = voxy_runtime::build_bootstrap_scene(42, 0).unwrap();
        let states = bootstrap_water_states(&scene.world).unwrap();
        let source = VoxelPos {
            x: 16,
            y: 18,
            z: 16,
        };
        let new = VoxelPos { x: 0, y: 20, z: 0 };
        let plan = step_water(
            &scene.world,
            scene.world.registry(),
            states,
            &[source],
            EditSource::Simulation,
            WaterBudget::default(),
        )
        .unwrap();
        scene
            .world
            .commit(voxy_world::EditTxn {
                source: EditSource::Simulation,
                expected: vec![],
                writes: vec![voxy_world::VoxelWrite {
                    pos: new,
                    block: states.0[0],
                }],
            })
            .unwrap();
        let mut active = vec![new, new];
        assert_eq!(
            publish_water_plan(&mut scene.world, &mut active, vec![source], plan).unwrap(),
            WaterPublication::Retry
        );
        assert_eq!(active, [new, source]);
        assert_eq!(scene.world.sample(source), Sample::Loaded(states.0[7]));
        let fresh = step_water(
            &scene.world,
            scene.world.registry(),
            states,
            &[source],
            EditSource::Simulation,
            WaterBudget::default(),
        )
        .unwrap();
        active.retain(|&pos| pos != source);
        assert_eq!(
            publish_water_plan(&mut scene.world, &mut active, vec![source], fresh).unwrap(),
            WaterPublication::Committed
        );
        assert!(active.contains(&new));
        assert_eq!(
            scene.world.sample(source),
            Sample::Loaded(voxy_world::BlockStateId::AIR)
        );
        assert_eq!(
            publish_water_plan(
                &mut scene.world,
                &mut active,
                vec![source],
                WaterPlan::Settled
            )
            .unwrap(),
            WaterPublication::Settled
        );
        assert!(active.contains(&new));
    }

    #[test]
    fn autopilot_releases_existing_procedural_water_through_destruction() {
        let scene = build_procedural_scene(0x56_4f_58_59, 1).unwrap();
        let states = bootstrap_water_states(&scene.world).unwrap();
        let cells = terrain_water_cells(&scene, states).unwrap();
        let bed = natural_water_bed(&scene.world, states, &cells).unwrap();
        let source = VoxelPos {
            y: bed.y + 1,
            ..bed
        };
        let initial = scene.world.sample(source);
        assert!(matches!(initial, Sample::Loaded(block) if states.0.contains(&block)));
        let mut app = VoxyApp {
            world: Some(scene.world),
            water_states: Some(states),
            autopilot_water_bed: Some(bed),
            ..VoxyApp::default()
        };
        app.release_autopilot_water().unwrap();
        let world = app.world.as_ref().unwrap();
        assert_eq!(world.sample(source), initial);
        assert_eq!(
            world.sample(bed),
            Sample::Loaded(voxy_world::BlockStateId::AIR)
        );
        assert!(app.water_active.contains(&source));
        let WaterPlan::Transaction { edit, .. } = step_water(
            world,
            world.registry(),
            states,
            &app.water_active,
            EditSource::Simulation,
            WaterBudget::default(),
        )
        .unwrap() else {
            panic!("existing water must flow into the destroyed bed");
        };
        assert!(
            edit.writes
                .iter()
                .any(|write| write.pos == bed && states.0.contains(&write.block))
        );
    }

    #[test]
    fn committed_edit_wakes_adjacent_water_without_duplicate_or_unloaded_work() {
        let mut scene = voxy_runtime::build_bootstrap_scene(42, 0).unwrap();
        let states = bootstrap_water_states(&scene.world).unwrap();
        let source = VoxelPos {
            x: 16,
            y: 18,
            z: 16,
        };
        let below = VoxelPos { y: 17, ..source };
        // Insert a support beneath a known full-water source and remove it using
        // the authoritative commit path, as an explosion would do.
        let support = scene
            .world
            .registry()
            .find(&ResourceKey::parse("voxy:stone").unwrap())
            .unwrap();
        scene
            .world
            .commit(voxy_world::EditTxn {
                source: EditSource::Editor,
                expected: vec![],
                writes: vec![voxy_world::VoxelWrite {
                    pos: below,
                    block: support,
                }],
            })
            .unwrap();
        let receipt = scene
            .world
            .commit(voxy_world::EditTxn {
                source: EditSource::Editor,
                expected: vec![],
                writes: vec![voxy_world::VoxelWrite {
                    pos: below,
                    block: voxy_world::BlockStateId::AIR,
                }],
            })
            .unwrap();
        let mut active = vec![];
        wake_water_after_edits(&scene.world, states, &receipt.inverse.writes, &mut active);
        assert!(active.contains(&source));
        let previous = active.clone();
        wake_water_after_edits(&scene.world, states, &receipt.inverse.writes, &mut active);
        assert_eq!(active, previous);
        wake_water_after_edits(
            &scene.world,
            states,
            &[voxy_world::VoxelWrite {
                pos: VoxelPos {
                    x: i64::MAX,
                    y: i64::MIN,
                    z: i64::MAX,
                },
                block: voxy_world::BlockStateId::AIR,
            }],
            &mut active,
        );
        assert_eq!(active, previous);
        let plan = step_water(
            &scene.world,
            scene.world.registry(),
            states,
            &active,
            EditSource::Simulation,
            WaterBudget::default(),
        )
        .unwrap();
        let WaterPlan::Transaction { edit, .. } = plan else {
            panic!("woken water did not flow into removed support");
        };
        scene.world.commit(edit).unwrap();
        let Sample::Loaded(block) = scene.world.sample(below) else {
            panic!("destination unloaded");
        };
        assert!(states.0.contains(&block));
    }

    #[test]
    fn procedural_spawn_has_clear_loaded_body_space() {
        let scene = build_procedural_scene(0x56_4f_58_59, 0).unwrap();
        let spawn = terrain_spawn(&scene.world).unwrap();
        for x in [15, 16] {
            for z in [15, 16] {
                for y in spawn.y..spawn.y + 2 {
                    let Sample::Loaded(block) = scene.world.sample(VoxelPos { x, y, z }) else {
                        panic!("spawn body is not loaded");
                    };
                    assert_eq!(
                        scene.world.registry().get(block).unwrap().collision,
                        CollisionShape::Empty
                    );
                }
            }
        }
    }

    #[test]
    fn active_water_is_discovered_from_actual_blocks() {
        let scene = voxy_runtime::build_bootstrap_scene(42, 0).unwrap();
        let states = bootstrap_water_states(&scene.world).unwrap();
        let active = terrain_water_cells(&scene, states).unwrap();
        assert!(active.len() >= 81);
        for position in &active {
            let Sample::Loaded(block) = scene.world.sample(*position) else {
                panic!("water is not loaded");
            };
            assert!(states.0.contains(&block));
        }
        for x in 12..=20 {
            for z in 12..=20 {
                assert!(active.contains(&VoxelPos { x, y: 18, z }));
            }
        }
    }
}
