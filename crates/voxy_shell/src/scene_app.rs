//! Animated general 3D geometry plus a textured 2D overlay in a native window.
use glam::{Mat4, Quat, Vec3};
use std::{sync::Arc, time::Instant};
use voxy_render::{
    RenderOutcome, SceneCamera, SceneDraw, SceneGeometry, SceneMesh, SceneProjection,
    SceneRenderer, SceneSurface as Renderer, SceneTexture, SceneTransform, SceneVertex,
};
use voxy_scene::{Behavior, BehaviorRunner, NodeId, SceneGraph, Transform};
use voxy_scene_host::{
    DemoEnv, DemoFrame, DemoGpu, DemoTick, DemoView, FrameParts, HostRequest, Presented, SceneDemo,
    SharedDraw, SmokeReport, perspective_camera,
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

#[derive(Debug)]
struct Resources {
    host: Renderer,
    scene: SceneRenderer,
    cube: SceneGeometry,
    overlay: SceneGeometry,
    texture: SceneTexture,
    world_transform: SceneTransform,
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
    /// The presented demo; `None` shows the built-in rotating cube.
    demo: Option<Box<dyn SceneDemo>>,
    host_request: HostRequest,
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
    paused: bool,
    occluded: bool,
    /// Last title pushed to the windowing system.
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
            demo: None,
            host_request: HostRequest::default(),
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
            paused: false,
            occluded: false,
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
    /// Presents `demo` instead of the built-in rotating cube. Demos are
    /// mutually exclusive: a later call replaces the earlier demo.
    #[must_use]
    pub fn with_demo(mut self, demo: Box<dyn SceneDemo>) -> Self {
        self.demo = Some(demo);
        self
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

    #[expect(
        clippy::too_many_lines,
        reason = "predates the function-size ratchet (docs/refactor/PLAN.md 6.2)"
    )]
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
        let trace = self.host_request.frame_trace
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
        if let Some(demo) = &mut self.demo {
            demo.begin_frame(dt);
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
        if let Some(title) = self.demo.as_ref().and_then(|demo| demo.title(self.paused)) {
            set_title_if_changed(window, &mut self.last_title, title);
        }
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let object_world = self.object_world()?;
        let camera = self.build_scene_camera(size, object_world);
        let vp = camera.view_projection()?;
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
        if let Some(demo) = &mut self.demo {
            demo.prepare(&mut DemoFrame {
                device: r.host.device(),
                queue: r.host.queue(),
                adapter: r.host.adapter(),
                color_format: r.host.color_format(),
                scene: &r.scene,
                texture: &mut r.texture,
                world_transform: &r.world_transform,
                camera,
                object_world,
                size: [size.width, size.height],
                frame: self.frames,
                paused: self.paused,
                motion_vectors: self.motion_vectors == MotionMode::Enabled,
                trace_started: trace.then_some(frame_started),
            })?;
        }
        let mvp = vp * object_world;
        if self.motion_vectors == MotionMode::Enabled && r.host.temporal_frame().is_none() {
            self.motion.reset();
        }
        let motion = self.motion.prepare(mvp)?;
        if r.last_world_motion != Some(motion) {
            r.world_transform.update_motion(r.host.queue(), motion)?;
            r.last_world_motion = Some(motion);
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
        let FrameParts {
            draws: demo_draws,
            refractive: demo_refractive,
            prepare: demo_prepare,
            custom_pass,
        } = match &mut self.demo {
            Some(demo) => demo.frame(SharedDraw {
                texture: &r.texture,
                world_transform: &r.world_transform,
                overlay_transform: &r.overlay_transform,
            })?,
            // The built-in scene: the rotating cube (or the script's box) and its overlay quad.
            None => FrameParts {
                draws: vec![
                    SceneDraw {
                        geometry: &r.cube,
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
                ],
                ..FrameParts::default()
            },
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
        let mut active_draws: Vec<_> = demo_draws
            .into_iter()
            .filter(|draw| draw.overlay || object_active)
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
        let refractive_layers: Vec<_> = demo_refractive
            .into_iter()
            .filter(|_| object_active)
            .collect();
        let outcome = match (custom_pass, demo_prepare) {
            (Some(custom_pass), _) => r.host.render_custom(
                |encoder, target| -> Result<(), voxy_render::RendererError> {
                    custom_pass(&r.scene, encoder, target, &active_draws);
                    Ok(())
                },
            )?,
            (None, demo_prepare) => r.host.render_scene_with_preparation_and_temporal_hooks(
                &r.scene,
                &active_draws,
                &refractive_layers,
                |queue, encoder| {
                    if let Some(prepare) = demo_prepare {
                        prepare(queue, encoder)?;
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
            )?,
        };
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
            if let Some(demo) = &mut self.demo {
                demo.presented(Presented {
                    frame: self.frames,
                    size: [size.width, size.height],
                    object_active,
                    smoke: self.smoke,
                })?;
            }
            self.recovering_gpu = false;
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

mod demo_input;
mod events;
mod init;
