//! Interactive resident GPU physics: Space pauses, R resets, Esc exits.
use std::{sync::Arc, time::Instant};
use voxy_gpu::{
    GravityBody, GravityBudget, GravityJob, GravityParameters, GravityProgram, GravityView,
};
use voxy_render::{GraphicsBackend, GraphicsOptions, RenderOutcome, SceneSurface};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};
type Error = Box<dyn std::error::Error>;
struct Resources {
    host: SceneSurface,
    program: GravityProgram,
    job: GravityJob,
    drawing: GravityView,
}
impl Resources {
    fn orbit(
        program: &GravityProgram,
        host: &SceneSurface,
    ) -> Result<(GravityJob, GravityView), Error> {
        let job = program.create_job(
            host.device(),
            &[
                GravityBody {
                    mass: 1.0,
                    position: [-0.4, 0.0, 0.0],
                    velocity: [0.0, -0.56, 0.0],
                },
                GravityBody {
                    mass: 1.0,
                    position: [0.4, 0.0, 0.0],
                    velocity: [0.0, 0.56, 0.0],
                },
            ],
            GravityParameters {
                constant: 0.5,
                softening: 0.02,
                uniform_acceleration: [0.0; 3],
                dt: 1.0 / 240.0,
            },
        )?;
        let drawing =
            pollster::block_on(GravityView::new(host.device(), &job, host.color_format()))?;
        Ok((job, drawing))
    }
    fn reset(&mut self) -> Result<(), Error> {
        let (job, drawing) = Self::orbit(&self.program, &self.host)?;
        self.job = job;
        self.drawing = drawing;
        Ok(())
    }
    fn check_acquisition(&mut self, width: u32, height: u32) -> Result<(), Error> {
        self.host.resize(0, 0)?;
        let outcome = self
            .host
            .render_custom::<_, Error>(|_, _| panic!("suspended surface encoded work"))?;
        assert_eq!(outcome, RenderOutcome::Suspended);
        self.host.resize(width, height)?;
        println!("PASS: suspended surface encodes no work");
        Ok(())
    }
    fn new(
        window: Arc<Window>,
        graphics: GraphicsOptions,
        instance: wgpu::Instance,
    ) -> Result<Self, Error> {
        let size = window.inner_size();
        let host = pollster::block_on(SceneSurface::new_with_instance(
            window,
            size.width,
            size.height,
            graphics,
            instance,
        ))?;
        println!("GPU gravity window: {:?}", host.adapter_info());
        let program =
            pollster::block_on(GravityProgram::new(host.device(), GravityBudget::default()))?;
        let (job, drawing) = Self::orbit(&program, &host)?;
        Ok(Self {
            host,
            program,
            job,
            drawing,
        })
    }
}
struct App {
    graphics: GraphicsOptions,
    window: Option<Arc<Window>>,
    resources: Option<Resources>,
    paused: bool,
    last: Instant,
    started: Instant,
    last_outcome: Option<RenderOutcome>,
    accumulator: f64,
    smoke: bool,
    frames: u32,
    steps: u64,
    failure: Option<String>,
    resized: bool,
}
impl App {
    fn draw(&mut self) -> Result<(), Error> {
        let elapsed = self.last.elapsed().as_secs_f64().min(0.1);
        self.last = Instant::now();
        if !self.paused {
            self.accumulator += elapsed;
        }
        let Some(resources) = &mut self.resources else {
            return Ok(());
        };
        if self.smoke && self.frames == 5 {
            let mut called = false;
            let result = resources.host.render_custom::<_, Error>(|_, _| {
                called = true;
                Err("injected custom encoding failure".into())
            });
            if !called {
                result?;
                return Ok(());
            }
            if result.is_ok() {
                return Err("custom encoding failure was discarded".into());
            }
            println!("PASS: acquired custom frame callback failure handled without presentation");
        }
        let steps = if self.paused {
            0
        } else if self.smoke {
            4
        } else {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            {
                (self.accumulator * 240.0).floor().min(24.0) as u32
            }
        };
        let outcome = resources
            .host
            .render_custom::<_, Error>(|encoder, target| {
                if steps > 0 {
                    resources.job.encode_steps(encoder, steps)?;
                }
                resources.drawing.encode(encoder, target);
                Ok(())
            })?;
        self.last_outcome = Some(outcome);
        if outcome == RenderOutcome::Presented {
            self.accumulator = (self.accumulator - f64::from(steps) / 240.0).max(0.0);
            self.frames += 1;
            self.steps += u64::from(steps);
        }
        Ok(())
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: Error) {
        self.failure = Some(error.to_string());
        event_loop.exit();
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<(), Error> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Voxy GPU gravity | Space: pause | R: reset | Esc: exit")
                        .with_inner_size(winit::dpi::PhysicalSize::new(720, 720)),
                )?,
            );
            let instance = self
                .graphics
                .create_instance_with_display(event_loop.owned_display_handle());
            let mut resources = Resources::new(window.clone(), self.graphics, instance)?;
            if self.smoke {
                let size = window.inner_size();
                resources.check_acquisition(size.width, size.height)?;
            }
            self.resources = Some(resources);
            self.window = Some(window);
            self.last = Instant::now();
            Ok(())
        })();
        if let Err(error) = result {
            self.fail(event_loop, error);
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.smoke && self.frames >= 60 {
            return;
        }
        let result = (|| -> Result<(), Error> {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(size) => {
                    if size.width == 800 && size.height == 600 {
                        self.resized = true;
                    }
                    if let Some(resources) = &mut self.resources {
                        resources.host.resize(size.width, size.height)?;
                    }
                    self.accumulator = 0.0;
                    self.last = Instant::now();
                }
                WindowEvent::KeyboardInput { event, .. }
                    if event.state == ElementState::Pressed && !event.repeat =>
                {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),
                        PhysicalKey::Code(KeyCode::Space) => {
                            self.paused = !self.paused;
                            self.accumulator = 0.0;
                        }
                        PhysicalKey::Code(KeyCode::KeyR) => {
                            if let Some(resources) = &mut self.resources {
                                resources.reset()?;
                            }
                            self.accumulator = 0.0;
                        }
                        _ => {}
                    }
                }
                WindowEvent::RedrawRequested => {
                    self.draw()?;
                    if self.smoke && self.frames == 10 {
                        if let Some(window) = &self.window {
                            let _ =
                                window.request_inner_size(winit::dpi::PhysicalSize::new(800, 600));
                        }
                    }
                    if self.smoke && self.frames == 20 {
                        self.paused = true;
                    }
                    if self.smoke && self.frames == 30 {
                        self.paused = false;
                    }
                    if self.smoke && self.frames == 40 {
                        if let Some(resources) = &mut self.resources {
                            resources.reset()?;
                        }
                    }
                    if self.smoke && self.frames >= 60 {
                        if self.steps != 200 || !self.resized {
                            return Err(format!(
                                "smoke failed: {} steps, resize observed {}",
                                self.steps, self.resized
                            )
                            .into());
                        }
                        println!(
                            "PASS: 60 presented GPU gravity frames, resize, 10 paused frames, reset; 200 resident steps, no body readback"
                        );
                        event_loop.exit();
                    }
                }
                _ => {}
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.fail(event_loop, error);
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.smoke && self.started.elapsed().as_secs() >= 30 {
            self.failure = Some(format!(
                "GPU gravity smoke timed out: {} frames, {} steps, last outcome {:?}",
                self.frames, self.steps, self.last_outcome
            ));
            event_loop.exit();
            return;
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
fn main() -> Result<(), Error> {
    let mut smoke = false;
    let mut graphics = GraphicsOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--smoke" => smoke = true,
            "--fallback" => graphics.force_fallback_adapter = true,
            "--backend" => {
                graphics.backend = match args.next().as_deref() {
                    Some("auto") => GraphicsBackend::Auto,
                    Some("metal") => GraphicsBackend::Metal,
                    Some("vulkan") => GraphicsBackend::Vulkan,
                    Some("dx12") => GraphicsBackend::DirectX12,
                    Some("gl") => GraphicsBackend::OpenGl,
                    _ => return Err("--backend expects auto|metal|vulkan|dx12|gl".into()),
                }
            }
            "--help" => {
                println!(
                    "gpu_gravity [--smoke] [--backend auto|metal|vulkan|dx12|gl] [--fallback]"
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
    let mut app = App {
        graphics,
        window: None,
        resources: None,
        paused: false,
        last: Instant::now(),
        started: Instant::now(),
        last_outcome: None,
        accumulator: 0.0,
        smoke,
        frames: 0,
        steps: 0,
        failure: None,
        resized: false,
    };
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.failure {
        return Err(error.into());
    }
    Ok(())
}
