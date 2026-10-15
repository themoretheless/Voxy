//! Photorealistic Rectilinear Cinema / Eye Camera (`cam`) Showcase & Snapshot Generator.
//!
//! Renders the same photorealistic Linear HDR PBR, CIE Overcast Sky GI, GTAO Multi-Bounce,
//! Dual-Lobe Wet Clearcoat, and AgX Tone Mapping pipeline as `bodycam`, but configured with
//! [`PhotorealLensProfile::CleanCam`]:
//! - Full-frame 16:9 image (no black corner housing bezel, no extreme fisheye barrel distortion)
//! - Natural 35mm cinema / human-eye field of view ($70^\circ$ vertical)
//! - Clean optical glass (minimal chromatic aberration, low ISO grain, crisp acutance)
//! - Direct eye-level camera mount without chest-vest deadzone lag
//!
//! Run interactively:
//!   `cargo run -p voxy_app --example cam`
//! Capture deterministic PNG snapshot:
//!   `cargo run -p voxy_app --example cam -- --snapshot /tmp/voxy-cam-preview.png`

#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

#[path = "bodycam.rs"]
mod bodycam_scene;

use std::{path::PathBuf, sync::Arc, time::Instant};

use glam::{Vec2, Vec3};
use voxy_core::ChunkPos;
use voxy_render::{PhotorealLensProfile, RenderOutcome, Renderer};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct CleanCamDemo {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    female_scene: Option<bodycam_scene::BunkerFemaleScene>,
    snapshot_path: Option<PathBuf>,
    smoke: bool,
    start: Instant,
    last_tick: Instant,
    frames: u64,
    flashlight_on: bool,
    wetness: f32,
    failure: Option<String>,
}

impl Default for CleanCamDemo {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            window: None,
            renderer: None,
            female_scene: None,
            snapshot_path: None,
            smoke: false,
            start: now,
            last_tick: now,
            frames: 0,
            flashlight_on: true,
            wetness: 0.72,
            failure: None,
        }
    }
}

impl CleanCamDemo {
    fn update_and_draw(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let dt = now
            .duration_since(self.last_tick)
            .as_secs_f32()
            .clamp(1.0 / 60.0, 0.05);
        self.last_tick = now;
        let t = now.duration_since(self.start).as_secs_f32();

        let renderer = self.renderer.as_mut().expect("initialized renderer");
        renderer.set_bodycam_enabled(true);

        let actor_feet_pos = Vec3::new(
            15.6 + (t * 0.45).sin() * 0.16,
            15.0,
            8.35 + (t * 0.30).cos() * 0.12,
        );
        let look_delta = Vec2::new((t * 0.9).cos() * 0.0010, (t * 0.7).sin() * 0.0007);
        let planar_speed = if self.snapshot_path.is_some() {
            0.25
        } else {
            1.15
        };

        {
            let rig = renderer.bodycam_rig_mut();
            rig.flashlight_enabled = self.flashlight_on;
            rig.apply_look(look_delta);
            rig.step(dt, planar_speed, true, false, 0.34);
        }
        let (camera, flash_pos, flash_dir) =
            renderer.bodycam_rig().camera_and_flashlight(actor_feet_pos);
        renderer.update_camera(camera)?;
        renderer.set_bodycam_flashlight_and_wetness(flash_pos, flash_dir, self.wetness);

        let size = self
            .window
            .as_ref()
            .map_or((1280, 720), |w| {
                let s = w.inner_size();
                (s.width.max(1280), s.height.max(720))
            });
        if let Some(female_scene) = &mut self.female_scene {
            female_scene.update_for_camera(
                renderer.queue(),
                camera,
                size.0,
                size.1,
                dt,
                self.snapshot_path.is_none() && self.frames % 2 == 0,
            )?;
        }

        if let Some(snapshot_path) = self.snapshot_path.take() {
            let (width, height, rgba) = if let Some(female_scene) = &self.female_scene {
                let draws = female_scene.draws();
                renderer.capture_bodycam_with_scene_rgba8(
                    self.frames + 1,
                    Some((&female_scene.scene_renderer, &draws)),
                )?
            } else {
                renderer.capture_bodycam_rgba8(self.frames + 1)?
            };
            image::save_buffer(&snapshot_path, &rgba, width, height, image::ColorType::Rgba8)?;
            println!(
                "CAM SNAPSHOT OK: saved {}x{} PNG to {}",
                width,
                height,
                snapshot_path.display()
            );
            event_loop.exit();
            return Ok(());
        }

        let outcome = if let Some(female_scene) = &self.female_scene {
            let draws = female_scene.draws();
            renderer.render_bodycam_with_scene(&female_scene.scene_renderer, &draws)?
        } else {
            renderer.render()?
        };
        if matches!(outcome, RenderOutcome::Presented) {
            self.frames += 1;
        }
        if self.smoke && self.frames >= 12 {
            println!("CAM SMOKE OK: {} frames presented", self.frames);
            event_loop.exit();
        }
        Ok(())
    }
}

impl ApplicationHandler for CleanCamDemo {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let headless = self.snapshot_path.is_some() || self.smoke;
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Voxy Photorealistic Camera Showcase — Female Character in Tactical Bunker")
                        .with_visible(!headless)
                        .with_inner_size(winit::dpi::LogicalSize::new(1280, 720)),
                )?,
            );
            let size = window.inner_size();
            let mut renderer = pollster::block_on(Renderer::new(
                Arc::clone(&window),
                size.width.max(1280),
                size.height.max(720),
            ))?;
            renderer.set_bodycam_enabled(true);
            {
                let rig = renderer.bodycam_rig_mut();
                rig.set_lens_profile(PhotorealLensProfile::CleanCam);
                rig.aim_angles = Vec2::new(0.04, -0.05);
                rig.chest_goal_angles = Vec2::new(0.04, -0.05);
                for _ in 0..24 {
                    rig.step(1.0 / 60.0, 0.25, true, false, 0.34);
                }
            }
            let chunks = bodycam_scene::build_tactical_compound_chunks()?;
            let uploads: Vec<_> = chunks
                .iter()
                .map(|c| (c.pos, &c.mesh, &c.light))
                .collect();
            renderer.upload_lit_chunks(&uploads, ChunkPos::default())?;
            let female_scene = bodycam_scene::BunkerFemaleScene::new(&renderer)?;
            println!(
                "Voxy Clean Cam GPU ready: {} ({} chunks + CC0 female character uploaded)",
                renderer.adapter_info().name,
                uploads.len()
            );
            self.female_scene = Some(female_scene);
            self.renderer = Some(renderer);
            self.start = Instant::now();
            self.last_tick = self.start;
            self.window = Some(window);
            if headless {
                self.update_and_draw(event_loop)?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("Clean Cam showcase startup failed: {error}");
            self.failure = Some(error.to_string());
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if let PhysicalKey::Code(key) = event.physical_key {
                    match key {
                        KeyCode::Escape if pressed => event_loop.exit(),
                        KeyCode::KeyF if pressed && !event.repeat => {
                            self.flashlight_on = !self.flashlight_on;
                        }
                        KeyCode::KeyB if pressed && !event.repeat => {
                            if let Some(renderer) = &mut self.renderer {
                                renderer
                                    .bodycam_rig_mut()
                                    .set_lens_profile(PhotorealLensProfile::Bodycam);
                            }
                        }
                        KeyCode::KeyV if pressed && !event.repeat => {
                            if let Some(renderer) = &mut self.renderer {
                                renderer
                                    .bodycam_rig_mut()
                                    .set_lens_profile(PhotorealLensProfile::CleanCam);
                            }
                        }
                        KeyCode::KeyN if pressed && !event.repeat => {
                            if let Some(renderer) = &mut self.renderer {
                                renderer
                                    .bodycam_rig_mut()
                                    .set_lens_profile(PhotorealLensProfile::Camcorder);
                            }
                        }
                        KeyCode::Space if pressed && !event.repeat => {
                            if let Some(renderer) = &mut self.renderer {
                                renderer
                                    .bodycam_rig_mut()
                                    .add_recoil_impulse(0.035, -0.010, 0.015);
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 40.0,
                };
                if let Some(renderer) = &mut self.renderer {
                    renderer.bodycam_rig_mut().apply_servo_zoom(lines);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.update_and_draw(event_loop) {
                    eprintln!("Clean Cam showcase draw failed: {error}");
                    self.failure = Some(error.to_string());
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut snapshot_path = None;
    let mut smoke = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--snapshot" => {
                let path = args
                    .next()
                    .ok_or("--snapshot requires an output PNG path")?;
                snapshot_path = Some(PathBuf::from(path));
            }
            "--smoke" => smoke = true,
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = CleanCamDemo {
        snapshot_path,
        smoke,
        ..CleanCamDemo::default()
    };
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.failure {
        return Err(error.into());
    }
    Ok(())
}
