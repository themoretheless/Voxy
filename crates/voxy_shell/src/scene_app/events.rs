//! Winit event handler, active demo stepping, camera framing and smoke verification.
use super::*;

impl SceneApp {
    pub(super) fn advance_active_demo(
        &mut self,
        elapsed: f32,
        dt: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(female) = &mut self.female {
            if self.full_model_worker.is_none() {
                let step = if self.smoke {
                    1.0 / 60.0
                } else {
                    f64::from(elapsed)
                };
                match self.hair_solver.as_mut() {
                    Some(solver) => female.advance_with_hair_solver(step, solver)?,
                    None => female.advance(step)?,
                }
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
        Ok(())
    }

    pub(super) fn build_scene_camera(
        &self,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> Result<SceneCamera, Box<dyn std::error::Error>> {
        let aspect = size.width as f32 / size.height as f32;
        Ok(SceneCamera {
            eye: if let Some(female) = &self.female {
                female.eye()
            } else if let Some(xray) = &self.xray {
                xray.eye(self.graph.world_matrix(self.object)?)
            } else if self.liquid_optics && self.liquids.is_some() {
                Vec3::new(0.0, 0.1, 3.5)
            } else if self.liquids.is_some() || self.wear.is_some() || self.fem.is_some() {
                Vec3::new(0.0, 1.4, 6.0)
            } else if let Some(gravity) = &self.gravity {
                Vec3::new(0.0, 0.0, gravity.camera_distance(aspect))
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
                aspect,
                near: 0.1,
                far: self
                    .gravity
                    .as_ref()
                    .map_or(100.0, |gravity| gravity.camera_distance(aspect) * 3.0),
            },
        })
    }

    pub(super) fn verify_smoke_completion(
        &mut self,
        outcome: RenderOutcome,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(window) = &self.window else {
            return Ok(());
        };
        let Some(r) = &self.resources else {
            return Ok(());
        };
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
                        return Err("film layers were absent from a presented smoke frame".into());
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
                rush.verify_smoke(&self.graph)?;
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
        Ok(())
    }
    pub(super) fn fail(&mut self, event_loop: &ActiveEventLoop, error: &dyn std::fmt::Display) {
        self.failure = Some(error.to_string());
        eprintln!("SCENE ERROR: {error}");
        event_loop.exit();
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
            if actions.toggle_world
                && let Ok(active) = self.graph.active_self(self.root)
            {
                let _ = self.graph.set_active(self.root, !active);
                self.motion.reset();
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
                    PhysicalKey::Code(
                        KeyCode::Digit1 | KeyCode::Digit2 | KeyCode::Digit3 | KeyCode::Digit4,
                    ) if self.female.as_ref().is_some_and(
                        voxy_demos::female_demo::FemaleDemo::has_grounded_playback,
                    ) =>
                    {
                        use voxy_demos::humanoid_locomotion::LocomotionClip;
                        let clip = match event.physical_key {
                            PhysicalKey::Code(KeyCode::Digit1) => LocomotionClip::Idle,
                            PhysicalKey::Code(KeyCode::Digit2) => LocomotionClip::Walk,
                            PhysicalKey::Code(KeyCode::Digit3) => LocomotionClip::Jog,
                            _ => LocomotionClip::Sprint,
                        };
                        if let Some(female) = &mut self.female
                            && let Err(error) = female.select_grounded_gait(clip)
                        {
                            eprintln!("Gait selection rejected: {error}");
                        }
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
                        if let Some(female) = &mut self.female
                            && let Err(error) = female.adjust_grasp(delta)
                        {
                            self.fail(event_loop, &error);
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
                        self.angle -= 0.15;
                    }
                    PhysicalKey::Code(KeyCode::ArrowRight) if self.xray.is_some() => {
                        self.angle += 0.15;
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
                            match voxy_demos::fem_demo::FemDemo::new() {
                                Ok(demo) => self.fem = Some(demo),
                                Err(error) => {
                                    self.fail(event_loop, &error);
                                    return;
                                }
                            }
                        }
                        if self.wear.is_some() {
                            match voxy_demos::wear_demo::WearDemo::new() {
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
