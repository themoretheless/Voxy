//! Native window event loop dispatch, title updates, worker shutdown, and smoke acceptance.

use crate::*;

impl App {
    pub(crate) fn smoke_acceptance(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if self.smoke_deadline.is_none() {
            return Ok(false);
        }
        if let Some(play) = &self.smoke_play {
            if let Some(frame) = play.stop_frame {
                if self.frames > frame {
                    println!(
                        "MODEL PLAY PASS: fixed_ticks={} native_frames={} stop_presented=true",
                        self.play.simulation_ticks,
                        self.frames - play.first_frame
                    );
                    return Ok(true);
                }
            } else if play.started.elapsed() >= Duration::from_millis(250)
                && self.frames > play.first_frame + 3
                && self.play.simulation_ticks >= 12
            {
                self.toggle_play()?;
                self.smoke_play
                    .as_mut()
                    .ok_or("missing smoke Play")?
                    .stop_frame = Some(self.frames);
            }
        } else if self.recovered
            && self
                .last_publication
                .is_some_and(|time| time.elapsed() >= Duration::from_secs(1))
            && self.failed_at.is_some_and(|frame| self.frames > frame + 20)
        {
            self.toggle_play()?;
            self.smoke_play = Some(SmokePlay {
                started: Instant::now(),
                first_frame: self.frames,
                stop_frame: None,
            });
        }
        Ok(false)
    }

    pub(crate) fn update_edit_title(&self) {
        if let Some(window) = &self.window {
            if let Some(notice) = self.snapshot_path_notice() {
                window.set_title(&format!(
                    "Voxy — {} | {} | Enter confirms, Esc cancels",
                    notice.lines[0], notice.lines[1]
                ));
                return;
            }
            if self.audio.is_pending() {
                window.set_title("Voxy — preparing audio; F6 cancels play");
                return;
            }
            if self.standalone {
                window.set_title("Voxy Game");
                return;
            }
            if self.play.playing.is_some() && self.play.physics.is_some() {
                window.set_title("Voxy Play — Left/Right move, W/S depth, Space jump; F6 Stop");
                return;
            }
            window.set_title(&format!(
                "Voxy model viewport — instance {}/{} | view {}/{} | F4 split, F10 smoothing, D duplicate, R model, P parent, A active, F6 play/stop, Z undo, Y redo; Ctrl/Cmd+Shift+S/O snapshot save/open",
                if self.instances.is_empty() { 0 } else { self.selected + 1 },
                self.instances.len(),
                if self.split_views() { self.active_view + 1 } else { 1 },
                if self.split_views() { 2 } else { 1 },
            ));
        }
    }

    pub(crate) fn stop_workers(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut save_errors = self.finish_editor_file_save();
        save_errors.extend(self.finish_editor_file_load());
        self.restore_barrier = None;
        self.model_relink_draft = None;
        self.close_play_audio();
        self.retired_ui_workers.extend(self.ui_live.close());
        self.close_audio_device();
        let mut errors = save_errors;
        let relink = self
            .model_relink
            .take()
            .map(app::model_relink::Pending::close);
        if let Err(error) = self.cancel_pending_model() {
            errors.push(format!("model publication stop: {error}"));
        }
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.play.stop_simulation(&mut self.scene)
        })) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => errors.push(format!("simulation stop: {error}")),
            Err(payload) => errors.push(format!(
                "simulation stop panicked: {}",
                panic_message(payload.as_ref())
            )),
        }
        // Close both producers before waiting for either thread to finish.
        let imports = self
            .imports
            .take()
            .map(voxy_assets::AssetImportWorker::close);
        let watcher = self
            .watcher
            .take()
            .map(voxy_assets::SourcePollWorker::close);
        if let Some(worker) = imports
            && let Err(payload) = worker.join()
        {
            errors.push(format!(
                "import worker panicked: {}",
                panic_message(payload.as_ref())
            ));
        }
        if let Some(worker) = watcher
            && let Err(payload) = worker.join()
        {
            errors.push(format!(
                "poll worker panicked: {}",
                panic_message(payload.as_ref())
            ));
        }
        for worker in relink.into_iter().chain(self.retired_model_relink.take()) {
            if let Err(payload) = worker.join() {
                errors.push(format!(
                    "model relink worker panicked: {}",
                    panic_message(payload.as_ref())
                ));
            }
        }
        errors.extend(self.audio.join_workers());
        for worker in self.retired_ui_workers.drain(..) {
            if let Err(payload) = worker.join() {
                errors.push(format!(
                    "audio worker panicked: {}",
                    panic_message(payload.as_ref())
                ));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!("shutdown failed: {}", errors.join("; ")).into())
        }
    }

    fn evaluate_window_acceptance(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if self.standalone && self.smoke_deadline.is_some() {
            if self.frames >= 3
                && self.play.simulation_ticks > 0
                && self.graphics.as_ref().is_some_and(|graphics| {
                    self.required_gpu_assets()
                        .iter()
                        .all(|id| graphics.models.contains_key(id))
                        && self.ui_live.ready()
                        && graphics.ui_presented
                })
            {
                println!(
                    "GAME NATIVE PASS frames={} ticks={}",
                    self.frames, self.play.simulation_ticks
                );
                Ok(true)
            } else {
                Ok(false)
            }
        } else if self.animation_smoke.is_some() {
            self.animation_acceptance()
        } else if self.prefab_smoke.is_some() {
            self.prefab_acceptance()
        } else if self.lod_smoke.is_some() {
            self.lod_acceptance()
        } else if self.scene3d_smoke.is_some() {
            self.scene3d_acceptance()
        } else if self.gameplay_smoke.is_some() {
            self.gameplay_acceptance()
        } else {
            self.smoke_acceptance()
        }
    }
}

pub(crate) fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic")
}

impl Drop for App {
    fn drop(&mut self) {
        if let Err(error) = self.stop_workers() {
            eprintln!("VOXY SHUTDOWN FAILED: {error}");
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        if let Err(error) = self.initialize(events) {
            self.error = Some(error.to_string());
            events.exit();
        }
    }

    fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if self.trace.enabled
            && matches!(
                event,
                WindowEvent::Focused(_) | WindowEvent::Occluded(_) | WindowEvent::Resized(_)
            )
        {
            println!(
                "EDITOR WINDOW event={event:?} visible={:?} focused={:?}",
                self.window.as_ref().and_then(|window| window.is_visible()),
                self.window.as_ref().map(|window| window.has_focus())
            );
        }
        let result = match event {
            WindowEvent::CloseRequested => {
                events.exit();
                Ok(())
            }
            WindowEvent::Resized(size) => self.finish_drag(false).and_then(|()| {
                self.authoring.preview_seek = None;
                self.camera_drag = None;
                if let Some(panels) = &mut self.panels {
                    panels.invalidate_presentation();
                }
                self.graphics.as_mut().map_or(Ok(()), |graphics| {
                    graphics.host.as_mut().map_or(Ok(()), |host| {
                        host.resize(size.width, size.height)
                            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })
                    })
                })
            }),
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::CursorMoved { position, .. } => {
                let position = position.to_logical::<f32>(1.0);
                let cursor = Vec2::new(position.x, position.y);
                self.cursor = Some(cursor);
                if self.authoring.preview_seek.is_some() {
                    let scale = self
                        .window
                        .as_ref()
                        .map_or(1., |window| window.scale_factor() as f32);
                    self.continue_preview_seek(cursor, scale)
                } else if let Some((button, previous)) = self.camera_drag {
                    let delta = cursor - previous;
                    self.camera_drag = Some((button, cursor));
                    if button == MouseButton::Right {
                        self.camera.orbit(delta);
                        Ok(())
                    } else {
                        let size = self.window.as_ref().map_or(Vec2::ONE, |w| {
                            let s = w.inner_size();
                            Vec2::new(s.width as f32, s.height as f32)
                        });
                        self.camera
                            .pan(delta, self.active_camera_size(size))
                            .map_err(Into::into)
                    }
                } else {
                    self.preview_drag(cursor)
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y = camera::wheel_steps(
                    delta,
                    self.window
                        .as_ref()
                        .map_or(1., |window| window.scale_factor()),
                );
                if let (Some(cursor), Some(window)) = (self.cursor, &self.window)
                    && cursor.x / (window.scale_factor() as f32) < 150.0
                {
                    self.tree_scroll = if y < 0.0 {
                        self.tree_scroll
                            .saturating_add(1)
                            .min(self.instances.len().saturating_sub(1))
                    } else {
                        self.tree_scroll.saturating_sub(1)
                    };
                    Ok(())
                } else if self
                    .cursor
                    .is_some_and(|cursor| !self.standalone && self.cursor_in_viewport(cursor))
                    && self.panels.as_ref().is_none_or(panels::Panels::input_ready)
                {
                    let cursor = self.cursor.unwrap();
                    let size = self.window.as_ref().map(|w| w.inner_size()).unwrap();
                    self.view_pointer(cursor, Vec2::new(size.width as f32, size.height as f32))
                        .map(|hit| {
                            if hit.is_some() {
                                self.camera.zoom(y);
                            }
                        })
                } else {
                    Ok(())
                }
            }
            WindowEvent::MouseInput {
                state,
                button: button @ (MouseButton::Right | MouseButton::Middle),
                ..
            } => self.finish_drag(false).and_then(|()| {
                self.camera_drag = None;
                if state == ElementState::Pressed
                    && self.panels.as_ref().is_none_or(panels::Panels::input_ready)
                    && let Some(cursor) = self
                        .cursor
                        .filter(|cursor| !self.standalone && self.cursor_in_viewport(*cursor))
                    && let Some(size) = self.window.as_ref().map(|w| w.inner_size())
                    && self
                        .view_pointer(cursor, Vec2::new(size.width as f32, size.height as f32))?
                        .is_some()
                {
                    self.camera_drag = Some((button, cursor));
                }
                Ok(())
            }),
            WindowEvent::CursorLeft { .. } => {
                self.authoring.preview_seek = None;
                self.camera_drag = None;
                self.cursor = None;
                self.game_ui_input.cancel();
                self.finish_drag(false)
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let (Some(cursor), Some(window)) = (self.cursor, &self.window) {
                    let size = window.inner_size().to_logical::<f32>(1.0);
                    let viewport = Vec2::new(size.width, size.height);
                    self.pointer_press(cursor, viewport).or_else(|error| {
                        if let Some(window) = &self.window {
                            window.set_title(&format!("Voxy — operation failed: {error}"));
                        }
                        eprintln!("EDITOR OPERATION FAILED: {error}");
                        Ok(())
                    })
                } else {
                    Ok(())
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.authoring.preview_seek = None;
                if let (Some(cursor), Some(window)) = (self.cursor, &self.window) {
                    let size = window.inner_size();
                    self.game_ui_pointer(
                        cursor,
                        Vec2::new(size.width as f32, size.height as f32),
                        false,
                    )
                    .and_then(|()| self.finish_drag(true))
                    .or_else(|error| {
                        eprintln!("GAME UI ACTION FAILED: {error}");
                        Ok(())
                    })
                } else {
                    self.game_ui_input.cancel();
                    self.finish_drag(true)
                }
            }
            WindowEvent::Occluded(occluded) => {
                self.window_occluded = occluded;
                if occluded {
                    self.authoring.preview_seek = None;
                    if let Some(panels) = &mut self.panels {
                        panels.invalidate_presentation();
                    }
                    self.game_ui_input.cancel();
                    self.camera_drag = None;
                    self.finish_drag(false)
                } else {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    Ok(())
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                Ok(())
            }
            WindowEvent::Focused(focused) => {
                self.play.player_input.set_focused(focused);
                if !focused {
                    self.authoring.preview_seek = None;
                    self.game_ui_input.cancel();
                    self.modifiers = winit::keyboard::ModifiersState::empty();
                    if let Some(panels) = &mut self.panels {
                        panels.invalidate_presentation();
                    }
                    self.camera_drag = None;
                }
                if focused {
                    Ok(())
                } else {
                    self.finish_drag(false)
                }
            }
            WindowEvent::KeyboardInput {
                device_id, event, ..
            } => {
                self.keyboard_device = Some(device_id);
                if let PhysicalKey::Code(key) = event.physical_key {
                    if event.state == ElementState::Pressed {
                        let command = if self.snapshot_draft.is_some() {
                            self.snapshot_path_key(key, event.text.as_deref())
                                .map(|()| true)
                        } else if !event.repeat {
                            self.snapshot_shortcut(key)
                        } else {
                            Ok(false)
                        };
                        match command {
                            Ok(true) => return,
                            Ok(false) => {}
                            Err(error) => {
                                self.report_snapshot_error(error);
                                return;
                            }
                        }
                    }
                    if event.state == ElementState::Pressed && self.snapshot_escape(key) {
                        return;
                    }
                    if self
                        .game_ui_key(key, event.state, event.repeat)
                        .unwrap_or_else(|error| {
                            eprintln!("GAME UI ACTION FAILED: {error}");
                            true
                        })
                    {
                        // Release any earlier gameplay binding even when UI acquired focus
                        // while that key was held; otherwise jump could stay latched.
                        if event.state == ElementState::Released {
                            self.game_key(key, event.state)
                                .expect("digital input event");
                        }
                        return;
                    }
                    match self.panel_key(key, event.state, event.repeat) {
                        Ok(true) => return,
                        Err(error) => {
                            eprintln!("PANEL OPERATION FAILED: {error}");
                            if let Some(window) = &self.window {
                                window
                                    .set_title(&format!("Voxy — panel operation failed: {error}"));
                            }
                            return;
                        }
                        Ok(false) => {}
                    }
                    self.game_key(key, event.state)
                        .expect("digital input event");
                    if event.state == ElementState::Released {
                        return;
                    }
                    let result = self.authoring_input_key(key, event.text.as_deref());
                    match result {
                        Err(error)
                            if self.field.is_some()
                                || matches!(
                                    key,
                                    KeyCode::F5
                                        | KeyCode::F10
                                        | KeyCode::F9
                                        | KeyCode::KeyP
                                        | KeyCode::F6
                                        | KeyCode::KeyC
                                        | KeyCode::KeyB
                                ) =>
                        {
                            if let Some(window) = &self.window {
                                window
                                    .set_title(&format!("Voxy — scene operation failed: {error}"));
                            }
                            eprintln!("SCENE OPERATION FAILED: {error}");
                            Ok(())
                        }
                        result => result,
                    }
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.error = Some(error.to_string());
            events.exit();
        }
        if self.error.is_none() {
            match self.evaluate_window_acceptance() {
                Ok(true) => events.exit(),
                Ok(false) => {}
                Err(error) => {
                    self.error = Some(error.to_string());
                    events.exit();
                }
            }
        }
        if self
            .smoke_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.error = Some(format!(
                "native acceptance timed out: presented={}, last_surface={:?}",
                self.frames, self.trace.last_outcome
            ));
            events.exit();
        }
    }

    fn device_event(
        &mut self,
        _: &ActiveEventLoop,
        device: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if matches!(event, winit::event::DeviceEvent::Removed)
            && self.keyboard_device == Some(device)
        {
            self.play.player_input.disconnect(0);
            self.keyboard_device = None;
        }
    }

    fn about_to_wait(&mut self, events: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            let size = window.inner_size();
            let drawable = !self.window_occluded && size.width > 0 && size.height > 0;
            events.set_control_flow(frame_pacing::control_flow(drawable, Instant::now()));
            window.request_redraw();
        } else {
            events.set_control_flow(ControlFlow::Wait);
        }
    }
}
