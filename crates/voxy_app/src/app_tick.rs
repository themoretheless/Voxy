//! Fixed-tick gameplay simulation, background chunk rebuilds, water, projectiles and autopilot.
use super::*;

impl VoxyApp {
    pub(super) fn advance_animation(&mut self, event_loop: &ActiveEventLoop) {
        const SPRINT_SPEED: f32 = 8.5;
        let now = Instant::now();
        let elapsed = self
            .last_update
            .replace(now)
            .map_or(0.0, |last| now.duration_since(last).as_secs_f32().min(0.25));
        self.accumulator = (self.accumulator + elapsed).min(0.25);
        let look_delta = self.input.take_look();
        self.camera.apply_look(look_delta);
        if let Some(renderer) = &mut self.renderer {
            renderer.bodycam_rig_mut().apply_look(look_delta);
        }
        if self.input.take_fire()
            && let Err(error) = self.spawn_shot()
        {
            eprintln!("Voxy projectile launch failed: {error}");
        }
        while self.accumulator >= FIXED_DT {
            if let Some(scenario) = self.profile.as_ref().map(|p| p.scenario) {
                self.profile_tick += 1;
                self.input.cancel_all();
                let tick = self.profile_tick;
                if scenario != frame_profile::Scenario::Idle {
                    self.input.key(
                        KeyCode::KeyW,
                        if tick % 1200 < 600 {
                            ElementState::Pressed
                        } else {
                            ElementState::Released
                        },
                        false,
                    );
                    self.input.key(
                        KeyCode::KeyS,
                        if tick % 1200 >= 600 {
                            ElementState::Pressed
                        } else {
                            ElementState::Released
                        },
                        false,
                    );
                }
                if scenario == frame_profile::Scenario::Water && tick == 720 {
                    if let Err(error) = self.release_autopilot_water() {
                        self.startup_failure = Some(error);
                        event_loop.exit();
                        return;
                    }
                    self.refresh_world_geometry(event_loop);
                }
                if scenario == frame_profile::Scenario::Water && tick % 600 == 300 {
                    self.input.queue_fire();
                }
            }
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
                        match voxy_demos::cuda_motion::character_motion(
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
                    let result = voxy_demos::cuda_motion::step_character_collision(
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
                    result.map_err(|error| error.to_string())
                } else if self.cuda_character_motion {
                    let Some(compute) = &self.motion_compute else {
                        return;
                    };
                    voxy_demos::cuda_motion::step_character(
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
                let grounded = character.grounded;
                let just_jumped_or_landed = grounded != self.was_grounded;
                self.was_grounded = grounded;
                let planar_speed = (character.velocity[0].hypot(character.velocity[2])) as f32;
                if let Some(renderer) = &mut self.renderer {
                    renderer.bodycam_rig_mut().step(
                        FIXED_DT,
                        planar_speed,
                        grounded,
                        just_jumped_or_landed,
                        0.28,
                    );
                }
            }
            self.pet
                .tick(FIXED_DT, movement.length_squared() > f32::EPSILON);
            self.pet_ui_elapsed += FIXED_DT;
            if self.pet_ui_elapsed >= 0.5 {
                self.pet_ui_elapsed = 0.0;
                self.update_pet_title();
            }
            self.advance_projectiles(FIXED_DT, event_loop);
            let water_started = self.profile.as_ref().map(|_| Instant::now());
            self.advance_water(event_loop);
            if let Some(started) = water_started {
                self.profile_water += started.elapsed();
            }
            let animation_started = self.profile.as_ref().map(|_| Instant::now());
            if !self.advance_actor_animation(
                movement.length_squared() > f32::EPSILON,
                intent.sprint,
                FIXED_DT,
                event_loop,
            ) {
                return;
            }
            if let Some(started) = animation_started {
                self.profile_animation += started.elapsed();
            }
            self.accumulator -= FIXED_DT;
        }
        let pose_started = self.profile.as_ref().map(|_| Instant::now());
        if let (Some(renderer), Some((palette, model))) =
            (&mut self.renderer, self.pending_actor_pose.take())
        {
            if let Err(error) = renderer.update_skinned_pose(&palette, model) {
                self.startup_failure =
                    Some(format!("Animal/actor pose publication failed: {error}"));
                event_loop.exit();
                return;
            }
            self.actor_palette_scratch = palette;
        }
        if let Some(started) = pose_started {
            self.profile_pose += started.elapsed();
        }
        if let Some(renderer) = &mut self.renderer {
            let spheres: Vec<_> = self
                .companions
                .iter()
                .flat_map(animals::Companion::shadow_spheres)
                .collect();
            if let Err(error) = renderer.set_subject_shadow_spheres(&spheres) {
                self.startup_failure = Some(error.to_string());
                event_loop.exit();
                return;
            }
        }
        if self.animal_check && self.companions.len() == 2 {
            let group = self.animal_check_captures == 0;
            let selected = usize::from(self.animal_check_captures >= 2);
            let paw = !group && std::env::args().any(|arg| arg == "--animal-paw");
            let face = !group && (paw || std::env::args().any(|arg| arg == "--animal-face"));
            let front = std::env::args().any(|arg| arg == "--animal-front");
            let face_view = if paw {
                self.companions[selected].inspection_paw()
            } else {
                self.companions[selected].inspection_face()
            };
            let center = if face {
                face_view.0
            } else if group {
                (self.companions[0].position() + self.companions[1].position()) * 0.5
                    + Vec3::Y * 0.35
            } else {
                self.companions[selected].inspection_center()
            };
            let distance = if face {
                face_view.1
            } else if group {
                1.8
            } else if selected == 0 {
                0.85
            } else {
                0.55
            };
            let forward = if group {
                Vec3::Z
            } else {
                self.companions[selected].forward()
            };
            let right = Vec3::new(forward.z, 0.0, -forward.x);
            if let Some(renderer) = &mut self.renderer {
                if let Err(error) = renderer.update_camera(CameraView {
                    eye: center
                        + (if paw {
                            right * 0.7 + Vec3::Y * 0.9 + forward * 1.5
                        } else if front {
                            forward * 1.5 + Vec3::Y * 0.10
                        } else if std::env::args().any(|arg| arg == "--animal-side") {
                            right * 1.5 + Vec3::Y * 0.12
                        } else if face {
                            right * 0.6 + Vec3::Y * 0.12 + forward * 1.5
                        } else {
                            right * 1.6 + Vec3::Y * 0.8 + forward * 2.2
                        }) * distance,
                    target: center,
                    vertical_fov_radians: if face {
                        20_f32.to_radians()
                    } else {
                        42_f32.to_radians()
                    },
                    ..CameraView::default()
                }) {
                    self.startup_failure = Some(error.to_string());
                    event_loop.exit();
                }
                renderer.set_bodycam_flashlight_and_wetness(
                    center + Vec3::new(0.0, 2.0, 2.0),
                    Vec3::NEG_Z,
                    0.0,
                );
            }
        } else if self.bodycam_mode {
            if let Some(renderer) = &mut self.renderer {
                let (cam_view, flash_pos, flash_dir) = renderer
                    .bodycam_rig()
                    .camera_and_flashlight(self.actor_position);
                renderer.set_bodycam_flashlight_and_wetness(
                    flash_pos,
                    flash_dir,
                    if self.clean_cam_mode && self.native_plants.is_some() {
                        0.06
                    } else {
                        0.68
                    },
                );
                if let Err(error) = renderer.update_camera(cam_view) {
                    eprintln!("Voxy bodycam camera update failed: {error}");
                    event_loop.exit();
                }
            }
        } else {
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
    }

    fn spawn_shot(&mut self) -> Result<(), physics_voxel::ProjectileError> {
        let (eye, target) = if self.bodycam_mode
            && let Some(renderer) = &mut self.renderer
        {
            renderer.bodycam_rig_mut().add_recoil_impulse(
                -3.2_f32.to_radians(),
                0.8_f32.to_radians(),
                1.9_f32.to_radians(),
            );
            let (cam, flash_pos, flash_dir) = renderer
                .bodycam_rig()
                .camera_and_flashlight(self.actor_position);
            (
                flash_pos,
                flash_pos + flash_dir * 4.0 + (cam.target - cam.eye) * 0.05,
            )
        } else {
            self.camera.eye_and_target(self.actor_position)
        };
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
        if let Some(world) = &self.world {
            self.animal_check_tick += 1;
            let empty_aabb = AnchoredAabb {
                anchor: VoxelPos { x: 0, y: 0, z: 0 },
                min: [0.0; 3],
                max: [0.0; 3],
            };
            let mut companion_bodies = [empty_aabb; 4];
            let companion_count = self.companions.len().min(companion_bodies.len());
            for (slot, animal) in companion_bodies[..companion_count]
                .iter_mut()
                .zip(&self.companions)
            {
                *slot = animal.body();
            }
            for (index, animal) in self.companions.iter_mut().enumerate() {
                let target = if self.animal_check
                    && (self.animal_check_tick < 180
                        || self.animal_check_tick >= 520
                        || (340..=365).contains(&self.animal_check_tick))
                {
                    animal.position()
                } else if self.animal_check {
                    let t = self.animal_check_tick as f32 * dt;
                    self.actor_position
                        + Vec3::new(
                            if index == 0 { 3.0 } else { -3.0 },
                            0.0,
                            4.0 + (t * 0.8).sin() * 3.0,
                        )
                } else {
                    self.actor_position + Vec3::X * if index == 0 { 1.2 } else { -1.2 }
                };
                let mut obstacles = [empty_aabb; 4];
                let mut obstacle_count = 0;
                for (other, &body) in companion_bodies[..companion_count].iter().enumerate() {
                    if other != index && obstacle_count < obstacles.len() {
                        obstacles[obstacle_count] = body;
                        obstacle_count += 1;
                    }
                }
                if let Some(character) = &self.character
                    && obstacle_count < obstacles.len()
                {
                    obstacles[obstacle_count] = character.body;
                    obstacle_count += 1;
                }
                if let Err(error) =
                    animal.tick_with_bodies(world, target, dt, &obstacles[..obstacle_count])
                {
                    self.startup_failure = Some(format!("Animal physics failed: {error}"));
                    event_loop.exit();
                    return false;
                }
                if index < companion_count {
                    companion_bodies[index] = animal.body();
                }
            }
        }
        if self.renderer.is_some() {
            let model = Mat4::from_rotation_translation(
                Quat::from_rotation_y(self.actor_yaw),
                self.actor_position,
            );
            // Catch-up ticks reuse the pending pose; ordinary frames reuse the
            // buffer returned after the last successful renderer publication.
            let mut palette = self.pending_actor_pose.take().map_or_else(
                || std::mem::take(&mut self.actor_palette_scratch),
                |(palette, _)| palette,
            );
            palette.clear();
            palette.extend_from_slice(&frame.skin_matrices);
            let actor_inverse = model.inverse();
            for animal in &self.companions {
                if let Err(error) = animal.append_palette(actor_inverse, &mut palette) {
                    self.startup_failure = Some(error);
                    event_loop.exit();
                    return false;
                }
            }
            self.pending_actor_pose = Some((palette, model));
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
                        self.dirty_chunks
                            .extend(invalidated_derived_chunks(&receipt));
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
        let step = if let Some(program) = &self.gpu_collisions {
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
                    voxy_demos::cuda_motion::character_motion(compute, state, input, dt, config)
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
                Err(error) => Err(voxy_demos::cuda_motion::MotionError::Collision(
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
                        voxy_demos::cuda_motion::step_character_collision(
                            compute,
                            world,
                            world.registry(),
                            chassis,
                            input,
                            dt,
                            config,
                            self.cuda_vehicle_motion,
                        )
                        .map_err(|error| {
                            voxy_demos::cuda_motion::MotionError::Collision(error.to_string())
                        })
                    } else {
                        voxy_demos::cuda_motion::step_character(
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
            .map_err(voxy_demos::cuda_motion::MotionError::from)
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

    pub(super) fn refresh_world_geometry(&mut self, event_loop: &ActiveEventLoop) {
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
        // Only chunks invalidated by commits since the last rebuild are derived again;
        // their merged output equals a full resident rebuild.
        let positions: Vec<ChunkPos> = self
            .resident_positions
            .iter()
            .copied()
            .filter(|pos| self.dirty_chunks.contains(pos))
            .collect();
        self.dirty_chunks.clear();
        let epoch = self.derivation_epoch;
        self.rebuild_pending = None;
        if positions.is_empty() {
            return;
        }
        self.rebuild = Some(std::thread::spawn(move || {
            let started = Instant::now();
            let result = rebuild_bootstrap_chunks(&world, &positions, epoch);
            (epoch, started.elapsed(), result)
        }));
    }

    pub(super) fn poll_rebuild(&mut self, event_loop: &ActiveEventLoop) {
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
        let chunks = match result {
            Ok(chunks) => chunks,
            Err(error) => {
                eprintln!("Voxy post-destruction rebuild failed: {error}");
                event_loop.exit();
                return;
            }
        };
        // Results stay valid for chunks no later commit invalidated; those chunks are
        // already in `dirty_chunks` and rebuilt below when the epoch moved on.
        let rebuilt = chunks.len();
        for chunk in chunks {
            self.derived_chunks.insert(chunk.pos, chunk);
        }
        let uploads = self
            .derived_chunks
            .values()
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
            "Voxy background rebuild: {:.2}ms ({rebuilt} chunks)",
            duration.as_secs_f64() * 1000.0
        );
        if epoch != self.derivation_epoch {
            self.start_rebuild();
        }
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
                Ok(receipt) => {
                    self.dirty_chunks
                        .extend(invalidated_derived_chunks(&receipt));
                    self.water_active = deferred;
                    self.water_active.extend(next_active.into_vec());
                    // Order is not observable by the planner: `step_water` copies the active
                    // slice into a `BTreeSet` and emits `next_active` sorted. It only decides
                    // which cells `split_off(max_active)` defers once the list exceeds the
                    // budget, where the GPU path already keys that partition on sorted order.
                    self.water_active.sort_unstable();
                    self.water_active.dedup();
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
            let outcome = publish_water_plan(
                world,
                &mut self.water_active,
                &mut self.dirty_chunks,
                work.active,
                plan,
            )?;
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

    pub(super) fn release_autopilot_water(&mut self) -> Result<(), String> {
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
        self.dirty_chunks
            .extend(invalidated_derived_chunks(&receipt));
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

pub(crate) fn character_position(character: &CharacterState) -> Vec3 {
    Vec3::new(
        character.body.anchor.x as f32
            + ((character.body.min[0] + character.body.max[0]) * 0.5) as f32,
        character.body.anchor.y as f32 + character.body.min[1] as f32,
        character.body.anchor.z as f32
            + ((character.body.min[2] + character.body.max[2]) * 0.5) as f32,
    )
}

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

pub(super) fn apply_autopilot_controls(input: &mut InputState, tick: u64) {
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
