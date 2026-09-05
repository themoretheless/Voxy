mod controls;
mod pet;

use std::sync::Arc;
use std::time::Instant;

use controls::{CameraRig, InputState};
use glam::{Mat4, Quat, Vec3};
use pet::{PetAction, PetState};
use voxy_animation::{
    AnimationClip, Animator, Joint, JointTrack, Playback, QuatKey, Skeleton, Transform,
};
use voxy_physics::{
    AnchoredAabb, DestructionPlan, Explosion, RayOrigin, RaycastConfig, RaycastResult, raycast,
};
use voxy_physics::{
    CharacterConfig, CharacterInput, CharacterState, ProjectileConfig, ProjectileOutcome,
    ProjectileState, RaceCheckpoint, RaceProgress, RaceTrack, VehicleConfig, VehicleInput,
    VehicleState, WaterBudget, WaterPlan, WaterStates, plan_impact_explosion, spawn_projectile,
    step_character, step_projectile, step_vehicle, step_water, update_race,
};
use voxy_render::{CameraView, RenderOutcome, Renderer, RendererError, SkinnedMesh, SkinnedVertex};
use voxy_runtime::{BootstrapScene, build_bootstrap_scene, rebuild_bootstrap_chunks};
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
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    presented_once: bool,
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
    water_states: Option<WaterStates>,
    water_active: Vec<VoxelPos>,
    water_tick_divider: u8,
    driving: bool,
    vehicle: Option<VehicleState>,
    race_track: Option<RaceTrack>,
    race_progress: RaceProgress,
    autopilot_tick: Option<u64>,
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
        match pollster::block_on(Renderer::new(Arc::clone(&window), size.width, size.height)) {
            Ok(mut renderer) => {
                window.set_title(&loading_title(35, "WORLD"));
                let scene = match build_bootstrap_scene(0x56_4f_58_59, 1) {
                    Ok(scene) => scene,
                    Err(error) => {
                        eprintln!("Voxy world bootstrap failed: {error}");
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
            WindowEvent::CloseRequested => event_loop.exit(),
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
                self.advance_animation(event_loop);
                self.poll_rebuild(event_loop);
                if let Some(renderer) = &mut self.renderer {
                    match renderer.render() {
                        Ok(outcome @ (RenderOutcome::Presented | RenderOutcome::Reconfigured)) => {
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
                            if matches!(error, RendererError::SurfaceLost) {
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
        self.actor_position = Vec3::new(16.0, 20.0, 16.0);
        self.character = Some(CharacterState {
            body: AnchoredAabb {
                anchor: VoxelPos {
                    x: 16,
                    y: 20,
                    z: 16,
                },
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
        self.water_states = Some(bootstrap_water_states(&scene.world)?);
        self.water_active = (12..=20)
            .flat_map(|x| (12..=20).map(move |z| VoxelPos { x, y: 18, z }))
            .collect();
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
            if !self.advance_autopilot(event_loop) {
                return;
            }
            let intent = self.input.take_move();
            let (forward, right) = self.camera.planar_basis();
            let movement = if self.pet.can_move() {
                (right * intent.local.x + forward * intent.local.y).normalize_or_zero()
            } else {
                Vec3::ZERO
            };
            let speed = if intent.sprint {
                SPRINT_SPEED
            } else {
                WALK_SPEED
            };
            if movement.length_squared() > f32::EPSILON {
                self.actor_yaw = movement.x.atan2(movement.z);
            }
            if self.driving {
                self.advance_vehicle(intent, FIXED_DT, event_loop);
            } else {
                let (Some(world), Some(character)) = (&self.world, &mut self.character) else {
                    return;
                };
                let registry = world.registry();
                if let Err(error) = step_character(
                    world,
                    registry,
                    character,
                    CharacterInput {
                        planar_velocity: [
                            f64::from(movement.x * speed),
                            f64::from(movement.z * speed),
                        ],
                        jump_pressed: intent.jump_pressed && self.pet.can_move(),
                    },
                    f64::from(FIXED_DT),
                    CharacterConfig::default(),
                ) {
                    eprintln!("Voxy character physics failed: {error}");
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

    fn spawn_shot(&mut self) -> Result<(), voxy_physics::ProjectileError> {
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
        let Some(world) = &mut self.world else {
            return;
        };
        let mut impacts = Vec::new();
        self.projectiles.retain_mut(|projectile| {
            match step_projectile(
                world,
                projectile,
                f64::from(dt),
                ProjectileConfig::default(),
            ) {
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
                    Ok(_) => {
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
    ) {
        let (Some(world), Some(vehicle)) = (&self.world, &mut self.vehicle) else {
            return;
        };
        let step = step_vehicle(
            world,
            world.registry(),
            vehicle,
            VehicleInput {
                throttle: f64::from(intent.local.y),
                steering: f64::from(intent.local.x),
                brake: intent.jump_pressed,
            },
            f64::from(dt),
            VehicleConfig::default(),
        );
        let step = match step {
            Ok(step) => step,
            Err(error) => {
                eprintln!("Voxy vehicle physics failed: {error}");
                event_loop.exit();
                return;
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
        self.water_tick_divider = (self.water_tick_divider + 1) % 60;
        if self.water_tick_divider != 0 || self.water_active.is_empty() {
            return;
        }
        let (Some(world), Some(states)) = (&mut self.world, self.water_states) else {
            return;
        };
        let plan = step_water(
            world,
            world.registry(),
            states,
            &self.water_active,
            EditSource::Simulation,
            WaterBudget::default(),
        );
        let changed = match plan {
            Ok(WaterPlan::Settled) => {
                self.water_active.clear();
                false
            }
            Ok(WaterPlan::Transaction { edit, next_active }) => match world.commit(edit) {
                Ok(_) => {
                    self.water_active = next_active.into_vec();
                    self.water_commits += 1;
                    true
                }
                Err(error) => {
                    eprintln!("Voxy water commit failed: {error}");
                    event_loop.exit();
                    return;
                }
            },
            Err(error) => {
                eprintln!("Voxy water simulation failed: {error}");
                event_loop.exit();
                return;
            }
        };
        if changed {
            self.refresh_world_geometry(event_loop);
        }
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
        match tick {
            1 => {
                self.input.mouse_motion((120.0, -35.0));
                self.input.key(KeyCode::KeyW, ElementState::Pressed, false);
            }
            70 => self.input.key(KeyCode::Space, ElementState::Pressed, false),
            71 => self
                .input
                .key(KeyCode::Space, ElementState::Released, false),
            100 | 360 => self.input.queue_fire(),
            180 => self.toggle_driving(),
            181 => self.input.key(KeyCode::KeyA, ElementState::Pressed, false),
            240 => self.input.key(KeyCode::KeyA, ElementState::Released, false),
            600 => {
                self.input.key(KeyCode::KeyW, ElementState::Released, false);
                let camera_moved = (self.camera.yaw - self.autopilot_initial_yaw).abs() > 0.01;
                let character_moved = self.actor_position.distance(self.autopilot_start) > 0.5;
                let passed = camera_moved
                    && character_moved
                    && self.water_commits > 0
                    && self.explosion_commits > 0
                    && self.peak_vehicle_speed > 0.1
                    && self.animation_frames > 500;
                self.autopilot_result = Some(passed);
                if passed {
                    println!(
                        "Voxy autopilot passed: water={}, explosions={}, peak_vehicle={:.2}, animation_frames={}",
                        self.water_commits,
                        self.explosion_commits,
                        self.peak_vehicle_speed,
                        self.animation_frames
                    );
                } else {
                    eprintln!(
                        "Voxy autopilot failed: camera={camera_moved}, movement={character_moved}, water={}, explosions={}, peak_vehicle={:.2}, animation_frames={}",
                        self.water_commits,
                        self.explosion_commits,
                        self.peak_vehicle_speed,
                        self.animation_frames
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
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = VoxyApp::default();
    event_loop.run_app(&mut app)?;
    if app.autopilot_result == Some(false) {
        return Err(std::io::Error::other("Voxy graphical autopilot failed").into());
    }
    Ok(())
}
