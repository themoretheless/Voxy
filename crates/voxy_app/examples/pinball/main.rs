mod game;
mod visual;

use game::{Game, Input, STEP};
use glam::{Mat4, Vec3};
use std::{sync::Arc, time::Instant};
use voxy_render::{CameraView, RenderOutcome, Renderer};
use voxy_runtime::SimulationClock;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    game: Game,
    input: Input,
    last: Option<Instant>,
    clock: SimulationClock,
    previous: Game,
    speed: f64,
    paused: bool,
    display: Option<(u32, u32, bool, bool)>,
    smoke: bool,
    ticks: u64,
    frames: u64,
    failure: Option<String>,
}
impl App {
    fn draw(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let dt = self
            .last
            .replace(now)
            .map_or(0.0, |last| now.duration_since(last).as_secs_f64().min(0.1));
        let frame = self.clock.advance(dt, STEP, 128);
        for _ in 0..frame.steps {
            if self.smoke {
                self.input = Input {
                    launch: self.game.ready && self.ticks % 200 < 150,
                    left: self.ticks % 90 < 45,
                    right: self.ticks % 110 < 55,
                };
            }
            self.previous = self.game.clone();
            self.game.tick(self.input);
            if self.game.ready != self.previous.ready {
                self.previous = self.game.clone();
            }
            self.ticks += 1;
            if self.smoke {
                match self.ticks {
                    300 => {
                        self.clock.set_target(0.1);
                    }
                    400 => {
                        self.clock.set_target(1.0);
                    }
                    700 => {
                        self.clock.set_target(4.0);
                    }
                    1000 => {
                        self.clock.set_target(1.0);
                    }
                    _ => {}
                }
            }
        }
        let mut rendered = self.game.clone();
        rendered.ball.position = self.previous.ball.position * (1.0 - frame.alpha)
            + self.game.ball.position * frame.alpha;
        for i in 0..2 {
            rendered.angles[i] =
                self.previous.angles[i] * (1.0 - frame.alpha) + self.game.angles[i] * frame.alpha;
        }
        let renderer = self.renderer.as_mut().expect("initialized renderer");
        let display = (
            self.game.score,
            self.game.lives,
            self.game.ready,
            self.paused,
        );
        if self.display != Some(display) {
            renderer.upload_skinned_mesh(
                &visual::mesh(&self.game, self.paused),
                &visual::matrices(&self.game),
                Mat4::IDENTITY,
                0,
            )?;
            self.display = Some(display);
        }
        renderer.update_skin_matrices(&visual::matrices(&rendered))?;
        let outcome = renderer.render()?;
        if matches!(outcome, RenderOutcome::Presented) {
            self.frames += 1;
        }
        if let Some(window) = &self.window {
            window.set_title(&format!("Voxy Pinball | {:07} | Balls {} | Charge {:.0}% | Time {:.2}x → {:.2}x{} | S: slow | 1/2/4: speed | P: pause | N: step | R: reset | A/L: flippers | Space: launch",
                self.game.score, self.game.lives, self.game.charge * 100.0, self.clock.scale(), self.clock.target_scale(),
                if frame.overloaded { " (overloaded)" } else { "" }));
        }
        if self.smoke && self.ticks >= 2400 {
            assert!(self.frames > 0, "no presented frames");
            assert!(self.game.impacts > 0, "no bumper contacts in smoke run");
            println!(
                "PINBALL SMOKE OK: {} ticks, {} frames, {} bumper hits, {} flipper contacts, score {}",
                self.ticks, self.frames, self.game.impacts, self.game.flipper_hits, self.game.score
            );
            event_loop.exit();
        }
        Ok(())
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Voxy Pinball — loading")
                        .with_visible(false)
                        .with_inner_size(winit::dpi::LogicalSize::new(760, 1000)),
                )?,
            );
            let size = window.inner_size();
            let mut renderer =
                pollster::block_on(Renderer::new(Arc::clone(&window), size.width, size.height))?;
            renderer.set_material_pack(visual::materials());
            renderer.update_camera(CameraView {
                eye: Vec3::new(0.0, 32.0, 5.0),
                target: Vec3::new(0.0, 0.0, -10.0),
                up: Vec3::Y,
                vertical_fov_radians: 42_f32.to_radians(),
                near_plane: 0.1,
            })?;
            renderer.upload_skinned_mesh(
                &visual::mesh(&self.game, false),
                &visual::matrices(&self.game),
                Mat4::IDENTITY,
                0,
            )?;
            println!("Pinball GPU ready: {}", renderer.adapter_info().name);
            self.renderer = Some(renderer);
            self.last = Some(Instant::now());
            window.set_visible(true);
            window.focus_window();
            self.window = Some(window);
            Ok(())
        })();
        if let Err(error) = result {
            eprintln!("Pinball startup failed: {error}");
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
            WindowEvent::Focused(false) if !self.smoke => {
                self.input = Input::default();
                self.paused = true;
                self.clock.freeze();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if let PhysicalKey::Code(key) = event.physical_key {
                    match key {
                        KeyCode::KeyA | KeyCode::ArrowLeft => self.input.left = pressed,
                        KeyCode::KeyL | KeyCode::ArrowRight => self.input.right = pressed,
                        KeyCode::Space => self.input.launch = pressed,
                        KeyCode::KeyP if pressed && !event.repeat => {
                            self.paused = !self.paused;
                            self.clock
                                .set_target(if self.paused { 0.0 } else { self.speed });
                            self.last = Some(Instant::now());
                        }
                        KeyCode::KeyR if pressed && !event.repeat => {
                            self.game = Game::default();
                            self.input = Input::default();
                            self.clock = SimulationClock::default();
                            self.previous = self.game.clone();
                            self.speed = 1.0;
                            self.paused = false;
                        }
                        KeyCode::KeyS | KeyCode::Digit1 | KeyCode::Digit2 | KeyCode::Digit4
                            if pressed && !event.repeat =>
                        {
                            self.speed = match key {
                                KeyCode::KeyS => 0.1,
                                KeyCode::Digit2 => 2.0,
                                KeyCode::Digit4 => 4.0,
                                _ => 1.0,
                            };
                            self.paused = false;
                            self.clock.set_target(self.speed);
                        }
                        KeyCode::KeyN if pressed && !event.repeat && self.clock.scale() == 0.0 => {
                            self.game.tick(self.input);
                            self.previous = self.game.clone();
                            self.ticks += 1;
                        }
                        KeyCode::Escape if pressed => event_loop.exit(),
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.draw(event_loop) {
                    eprintln!("Pinball rendering failed: {error}");
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
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        speed: 1.0,
        smoke: std::env::args().any(|arg| arg == "--smoke"),
        ..App::default()
    };
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.failure {
        return Err(error.into());
    }
    if app.smoke && app.ticks < 2400 {
        return Err("smoke run closed before completion".into());
    }
    Ok(())
}
