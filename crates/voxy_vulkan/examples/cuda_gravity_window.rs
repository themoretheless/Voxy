#![allow(unsafe_code)]
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod native {
    //! Interactive resident CUDA physics with direct Vulkan or DX12 storage rendering.
    use std::{sync::Arc, time::Instant};
    use voxy_cuda::{CudaCompute, CudaGravityBody, CudaGravityBudget, CudaGravityParameters};
    use voxy_gpu::GravityView;
    use voxy_render::{GraphicsBackend, GraphicsOptions, RenderOutcome, SceneSurface};
    use voxy_vulkan::CudaGravityGraphics;
    use winit::{
        application::ApplicationHandler,
        event::{ElementState, WindowEvent},
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
        keyboard::{KeyCode, PhysicalKey},
        window::{Window, WindowId},
    };
    type Error = Box<dyn std::error::Error + Send + Sync>;
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Backend {
        Cuda,
        Export,
    }
    struct ExportPhysics {
        job: voxy_gpu::GravityJob,
        storage: voxy_vulkan::VulkanExportBuffer,
    }
    enum Physics {
        Cuda(Box<CudaGravityGraphics>),
        #[cfg(target_os = "windows")]
        Dx12(Box<voxy_vulkan::CudaGravityD3d12Graphics>),
        Export(Box<ExportPhysics>),
    }
    impl Physics {
        fn buffer(&self) -> Result<&wgpu::Buffer, Error> {
            match self {
                Self::Cuda(gravity) => gravity.buffer(),
                #[cfg(target_os = "windows")]
                Self::Dx12(gravity) => gravity.buffer(),
                Self::Export(physics) => physics.storage.buffer(),
            }
        }
        unsafe fn publish(
            &mut self,
            device: &wgpu::Device,
            queue: &wgpu::Queue,
            steps: u32,
        ) -> Result<(), Error> {
            match self {
                Self::Cuda(gravity) => unsafe { gravity.publish(steps) },
                #[cfg(target_os = "windows")]
                Self::Dx12(gravity) => unsafe { gravity.publish(steps) },
                Self::Export(physics) => {
                    let ExportPhysics { job, storage } = physics.as_mut();
                    let mut encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                    if steps > 0 {
                        job.encode_steps(&mut encoder, steps)?;
                    }
                    encoder.copy_buffer_to_buffer(job.buffer(), 0, storage.buffer()?, 0, 96);
                    queue.submit([encoder.finish()]);
                    unsafe {
                        let handle = storage.release()?;
                        drop(handle);
                        storage.acquire()
                    }
                }
            }
        }
    }
    struct Resources {
        host: SceneSurface,
        gravity: Physics,
        export_only: bool,
        dx12: bool,
        ordinal: usize,
        drawing: GravityView,
    }
    impl Resources {
        fn orbit(
            host: &SceneSurface,
            ordinal: usize,
            export_only: bool,
            dx12: bool,
        ) -> Result<(Physics, GravityView), Error> {
            let gravity = if export_only {
                let program = pollster::block_on(voxy_gpu::GravityProgram::new(
                    host.device(),
                    voxy_gpu::GravityBudget::default(),
                ))?;
                let job = program.create_job(
                    host.device(),
                    &[
                        voxy_gpu::GravityBody {
                            mass: 1.0,
                            position: [-0.4, 0.0, 0.0],
                            velocity: [0.0, -0.56, 0.0],
                        },
                        voxy_gpu::GravityBody {
                            mass: 1.0,
                            position: [0.4, 0.0, 0.0],
                            velocity: [0.0, 0.56, 0.0],
                        },
                    ],
                    voxy_gpu::GravityParameters {
                        constant: 0.5,
                        softening: 0.02,
                        uniform_acceleration: [0.0; 3],
                        dt: 1.0 / 240.0,
                    },
                )?;
                // SAFETY: Single-threaded event loop; no concurrent queue access.
                let storage = unsafe { voxy_vulkan::VulkanExportBuffer::new(host.device(), 96)? };
                Physics::Export(Box::new(ExportPhysics { job, storage }))
            } else {
                Self::cuda_orbit(host, ordinal, dx12)?
            };
            let drawing = pollster::block_on(GravityView::from_buffer(
                host.device(),
                gravity.buffer()?,
                2,
                host.color_format(),
            ))?;
            Ok((gravity, drawing))
        }
        fn cuda_orbit(host: &SceneSurface, ordinal: usize, dx12: bool) -> Result<Physics, Error> {
            let compute = CudaCompute::new(ordinal, 1024 * 1024)?;
            let bodies = [
                CudaGravityBody {
                    mass: 1.0,
                    position: [-0.4, 0.0, 0.0],
                    velocity: [0.0, -0.56, 0.0],
                },
                CudaGravityBody {
                    mass: 1.0,
                    position: [0.4, 0.0, 0.0],
                    velocity: [0.0, 0.56, 0.0],
                },
            ];
            let parameters = CudaGravityParameters {
                constant: 0.5,
                softening: 0.02,
                uniform_acceleration: [0.0; 3],
                dt: 1.0 / 240.0,
            };
            // SAFETY: Single-threaded event loop; no unsubmitted graphics work.
            if dx12 {
                #[cfg(target_os = "windows")]
                {
                    return Ok(Physics::Dx12(Box::new(unsafe {
                        voxy_vulkan::CudaGravityD3d12Graphics::new(
                            host.device(),
                            host.queue(),
                            compute,
                            &bodies,
                            parameters,
                            CudaGravityBudget::default(),
                        )?
                    })));
                }
                #[cfg(not(target_os = "windows"))]
                {
                    return Err("DX12 requires Windows".into());
                }
            }
            Ok(Physics::Cuda(Box::new(unsafe {
                CudaGravityGraphics::new(
                    host.device(),
                    compute,
                    &bodies,
                    parameters,
                    CudaGravityBudget::default(),
                )?
            })))
        }
        fn reset(&mut self) -> Result<(), Error> {
            let (gravity, drawing) =
                Self::orbit(&self.host, self.ordinal, self.export_only, self.dx12)?;
            self.gravity = gravity;
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
        fn matching_adapter(
            instance: &wgpu::Instance,
            compute: &CudaCompute,
            dx12: bool,
        ) -> Result<wgpu::Adapter, Error> {
            if dx12 {
                #[cfg(target_os = "windows")]
                {
                    let (luid, mask) = compute.windows_adapter_identity()?;
                    if luid == [0; 8] || mask != 1 {
                        return Err("CUDA LUID/node mask unavailable or linked".into());
                    }
                    return pollster::block_on(instance.enumerate_adapters(wgpu::Backends::DX12))
                        .into_iter()
                        .find(|adapter| {
                            voxy_vulkan::adapter_luid(adapter).is_ok_and(|value| value == luid)
                        })
                        .ok_or_else(|| "no DX12 adapter matching selected CUDA GPU".into());
                }
                #[cfg(not(target_os = "windows"))]
                {
                    return Err("DX12 requires Windows".into());
                }
            }
            let cuda = compute.capabilities()?;
            pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN))
                .into_iter()
                .find(|adapter| {
                    voxy_vulkan::adapter_uuid(adapter).is_ok_and(|uuid| uuid == cuda.uuid)
                })
                .ok_or_else(|| "no Vulkan adapter matching selected CUDA GPU".into())
        }
        fn new(
            window: Arc<Window>,
            instance: &wgpu::Instance,
            ordinal: usize,
            export_only: bool,
            dx12: bool,
        ) -> Result<Self, Error> {
            let adapter = if export_only {
                pollster::block_on(
                    instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
                )?
            } else {
                let compute = CudaCompute::new(ordinal, 1024 * 1024)?;
                Self::matching_adapter(instance, &compute, dx12)?
            };
            let size = window.inner_size();
            let host = pollster::block_on(SceneSurface::new_with_adapter(
                window,
                size.width,
                size.height,
                instance,
                adapter,
                if dx12 {
                    wgpu::Features::empty()
                } else {
                    voxy_vulkan::EXTERNAL_MEMORY_FEATURE
                },
            ))?;
            println!(
                "Gravity window export_only={export_only}: {:?}, dx12={dx12}",
                host.adapter_info()
            );
            let (gravity, drawing) = Self::orbit(&host, ordinal, export_only, dx12)?;
            Ok(Self {
                host,
                gravity,
                export_only,
                dx12,
                ordinal,
                drawing,
            })
        }
    }
    struct App {
        mode: Backend,
        ordinal: usize,
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
                println!(
                    "PASS: acquired custom frame callback failure handled without presentation"
                );
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
            let device = resources.host.device().clone();
            let queue = resources.host.queue().clone();
            let outcome = resources
                .host
                .render_custom::<_, Error>(|encoder, target| {
                    // SAFETY: Surface acquisition succeeded before this callback;
                    // single-threaded queue, retained drawing binding is not used until publication ends.
                    unsafe {
                        resources.gravity.publish(&device, &queue, steps)?;
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
        fn fail(&mut self, event_loop: &ActiveEventLoop, error: &dyn std::error::Error) {
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
                            .with_title("Voxy gravity | Space: pause | R: reset | Esc: exit")
                            .with_inner_size(winit::dpi::PhysicalSize::new(720, 720)),
                    )?,
                );
                let instance = self
                    .graphics
                    .create_instance_with_display(event_loop.owned_display_handle());
                let mut resources = Resources::new(
                    window.clone(),
                    &instance,
                    self.ordinal,
                    self.mode == Backend::Export,
                    self.graphics.backend == GraphicsBackend::DirectX12,
                )?;
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
                self.fail(event_loop, error.as_ref());
            }
        }
        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _id: WindowId,
            event: WindowEvent,
        ) {
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
                        if self.smoke
                            && self.frames == 10
                            && let Some(window) = &self.window
                        {
                            let _ =
                                window.request_inner_size(winit::dpi::PhysicalSize::new(800, 600));
                        }
                        if self.smoke && self.frames == 20 {
                            self.paused = true;
                        }
                        if self.smoke && self.frames == 30 {
                            self.paused = false;
                        }
                        if self.smoke
                            && self.frames == 40
                            && let Some(resources) = &mut self.resources
                        {
                            resources.reset()?;
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
                                "PASS: 60 presented gravity frames, resize, 10 paused frames, reset; 200 resident steps, no body readback"
                            );
                            event_loop.exit();
                        }
                    }
                    _ => {}
                }
                Ok(())
            })();
            if let Err(error) = result {
                self.fail(event_loop, error.as_ref());
            }
        }
        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            if self.smoke && self.started.elapsed().as_secs() >= 30 {
                self.failure = Some(format!(
                    "Gravity window smoke timed out: {} frames, {} steps, last outcome {:?}",
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
    pub fn run() -> Result<(), Error> {
        let mut smoke = false;
        let mut export_only = false;
        let mut graphics = GraphicsOptions {
            backend: GraphicsBackend::Vulkan,
            ..GraphicsOptions::default()
        };
        let mut ordinal = 0;
        let mut seen = false;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--smoke" => smoke = true,
                "--export-only" => export_only = true,
                "--dx12" => graphics.backend = GraphicsBackend::DirectX12,
                "--cuda-device" => {
                    if seen {
                        return Err("--cuda-device must be specified once".into());
                    }
                    seen = true;
                    ordinal = args
                        .next()
                        .ok_or("--cuda-device requires an ordinal")?
                        .parse::<usize>()?;
                    i32::try_from(ordinal)?;
                }
                "--help" => {
                    println!(
                        "cuda_gravity_window [--smoke] [--cuda-device N] [--export-only] [--dx12]"
                    );
                    return Ok(());
                }
                _ => return Err(format!("unknown argument: {arg}").into()),
            }
        }
        if graphics.backend == GraphicsBackend::DirectX12
            && (export_only || !cfg!(target_os = "windows"))
        {
            return Err(
                "--dx12 requires Windows CUDA mode; cannot combine with --export-only".into(),
            );
        }
        if export_only && seen {
            return Err("--export-only cannot select a CUDA device".into());
        }
        let mut app = App {
            mode: if export_only {
                Backend::Export
            } else {
                Backend::Cuda
            },
            ordinal,
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
}
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    native::run()
}
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Err("requires Linux/Windows Vulkan and NVIDIA CUDA".into())
}
