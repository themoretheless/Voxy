//! Winit event handler, active demo stepping, camera framing and smoke verification.
use super::*;

impl SceneApp {
    pub(super) fn advance_active_demo(
        &mut self,
        elapsed: f32,
        dt: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(demo) = &mut self.demo {
            demo.advance(DemoTick {
                elapsed,
                dt,
                smoke: self.smoke,
                frame: self.frames,
            })?;
        } else {
            self.angle += dt;
        }
        Ok(())
    }

    /// World matrix of the primary object; a script may have destroyed it.
    pub(super) fn object_world(&self) -> Result<Mat4, voxy_scene::SceneGraphError> {
        self.graph.world_matrix(self.object).or_else(|error| {
            if self.rush.is_some() {
                Ok(Mat4::IDENTITY)
            } else {
                Err(error)
            }
        })
    }

    pub(super) fn build_scene_camera(
        &self,
        size: winit::dpi::PhysicalSize<u32>,
        object_world: Mat4,
    ) -> SceneCamera {
        let aspect = size.width as f32 / size.height as f32;
        if let Some(demo) = &self.demo {
            demo.camera(DemoView {
                aspect,
                object_world,
            })
        } else {
            perspective_camera(Vec3::new(3.0, 2.0, 4.0), Vec3::ZERO, aspect, 100.0)
        }
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
        if self.frames >= 120 && self.demo.as_ref().is_none_or(|demo| demo.smoke_ready()) {
            if let Some(demo) = &self.demo {
                demo.verify_smoke(SmokeReport {
                    frames: self.frames,
                    resize_stages: self.resize_stages,
                    motion_vectors: self.motion_vectors == MotionMode::Enabled,
                })?;
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
        } else if self.started.elapsed().as_secs() > self.host_request.smoke_timeout_seconds {
            return Err(format!(
                "scene smoke timed out: frames={}, resize_stages={}, outcome={outcome:?}",
                self.frames, self.resize_stages
            )
            .into());
        }
        Ok(())
    }
    /// Drops the surface and every GPU resource created on its device.
    pub(super) fn release_gpu(&mut self) {
        self.resources = None;
        if let Some(demo) = &mut self.demo {
            demo.release();
        }
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
        self.release_gpu();
        self.motion.reset();
        self.touch.reset();
        self.last = Instant::now();
    }
    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
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
        if let Some(input) = demo_input::translate(&event)
            && let Some(demo) = &mut self.demo
        {
            match demo.input(input) {
                Ok(reply) => self.angle += reply.rotate_scene,
                Err(error) => self.fail(event_loop, &error),
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
            WindowEvent::Focused(false) => {
                self.touch.reset();
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
                if event.state == ElementState::Pressed
                    && !event.repeat
                    && event.physical_key == PhysicalKey::Code(KeyCode::Escape) =>
            {
                if self.smoke {
                    eprintln!("SCENE SMOKE EXIT: Escape at frame {}", self.frames);
                }
                event_loop.exit();
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
