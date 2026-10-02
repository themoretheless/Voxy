//! Native surface acceptance of terminal device-loss handling.
use std::sync::Arc;
use voxy_render::{Renderer, RendererError};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[derive(Default)]
struct Probe {
    failure: Option<String>,
    completed: bool,
    options: voxy_render::GraphicsOptions,
    require_nvidia: bool,
    recovered: Option<Renderer>,
    window: Option<Arc<Window>>,
    recovery_deadline: Option<std::time::Instant>,
}
impl Probe {
    fn schedule_recovery(
        &mut self,
        window: Arc<Window>,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let instance = self
            .options
            .create_instance_with_display(event_loop.owned_display_handle());
        self.recovered = Some(check_recreation(window.clone(), self.options, instance)?);
        self.recovery_deadline =
            Some(std::time::Instant::now() + std::time::Duration::from_secs(5));
        window.request_redraw();
        self.window = Some(window);
        Ok(())
    }
}
impl ApplicationHandler for Probe {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let window = Arc::new(event_loop.create_window(
                Window::default_attributes().with_title("Voxy device loss acceptance"),
            )?);
            let options = self.options;
            let instance = options.create_instance_with_display(event_loop.owned_display_handle());
            let mut renderer = pollster::block_on(Renderer::new_with_instance(
                window.clone(),
                320,
                240,
                options,
                instance,
            ))?;
            println!("Device loss GPU: {:?}", renderer.adapter_info());
            if self.require_nvidia {
                let info = renderer.adapter_info();
                if info.vendor != 0x10de
                    || !matches!(
                        info.device_type,
                        wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
                    )
                {
                    return Err("selected renderer is not a physical NVIDIA adapter".into());
                }
            }
            let scene = renderer.create_scene_renderer();
            let program = pollster::block_on(voxy_render::ComputeProgram::new(
                renderer.device(),
                "@group(0) @binding(0) var<storage, read_write> data: array<u32>; @compute @workgroup_size(1) fn cs_main() { data[0] = 42u; }",
            ))?;
            let mut encoder = renderer
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let dispatch = program
                .create_job(renderer.device(), &[0; 4])?
                .encode(&mut encoder, [1, 1, 1])?;
            renderer.queue().submit([encoder.finish()]);
            renderer.device().destroy();
            // Deliver the real driver's loss callback before checking terminal guards.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while renderer.device_failure().is_none() && std::time::Instant::now() < deadline {
                let _ = renderer.device().poll(wgpu::PollType::Poll);
                std::thread::yield_now();
            }
            let diagnostic = renderer
                .device_failure()
                .ok_or("device-loss callback missing")?
                .to_owned();
            if diagnostic.is_empty() {
                return Err("empty device-loss diagnostic".into());
            }
            check_lost_readback(renderer.device(), dispatch)?;
            renderer.set_material_pack(voxy_render::MaterialPack::new(
                1,
                1,
                vec![voxy_render::MaterialLayer {
                    rgba8_srgb: Arc::from([255, 255, 255, 255]),
                }],
            )?);
            renderer.resize(640, 480);
            renderer.resize(0, 0);
            for result in [renderer.render(), renderer.render_scene(&scene, &[])] {
                match result {
                    Err(RendererError::DeviceLost(message)) if message == diagnostic => {}
                    other => {
                        return Err(format!("terminal device-loss guard failed: {other:?}").into());
                    }
                }
            }
            let origin = voxy_core::ChunkPos { x: 0, y: 0, z: 0 };
            for result in [
                renderer.upload_chunks(&[], origin),
                renderer.upload_lit_chunks(&[], origin),
                renderer.update_camera(voxy_render::CameraView::default()),
                renderer.update_skin_matrices(&[]),
                renderer.update_skinned_model(glam::Mat4::IDENTITY),
            ] {
                match result {
                    Err(RendererError::DeviceLost(message)) if message == diagnostic => {}
                    other => {
                        return Err(format!("device-loss upload guard failed: {other:?}").into());
                    }
                }
            }
            println!("PASS: native device loss rejects geometry, camera and skin writes");
            println!(
                "PASS: native device destruction retains diagnostic and guards render, scene and resize"
            );
            drop(program);
            drop(scene);
            drop(renderer);
            self.schedule_recovery(window, event_loop)?;
            Ok(())
        })();
        match result {
            Ok(()) => {}
            Err(error) => self.failure = Some(error.to_string()),
        }
        if self.failure.is_some() {
            event_loop.exit();
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if event == WindowEvent::CloseRequested {
            self.failure = Some("device-loss acceptance window closed before completion".into());
            event_loop.exit();
            return;
        }
        if event != WindowEvent::RedrawRequested {
            return;
        }
        let Some(renderer) = self.recovered.as_mut() else {
            return;
        };
        match renderer.render() {
            Ok(voxy_render::RenderOutcome::Presented) => {
                println!(
                    "PASS: recreated native surface presents a fresh frame on the same window"
                );
                self.completed = true;
                event_loop.exit();
            }
            Ok(outcome) => {
                if self
                    .recovery_deadline
                    .is_some_and(|deadline| std::time::Instant::now() >= deadline)
                {
                    self.failure = Some(format!("recreated surface did not present: {outcome:?}"));
                    event_loop.exit();
                } else if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            Err(error) => {
                self.failure = Some(error.to_string());
                event_loop.exit();
            }
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self.recovery_deadline {
            if !self.completed && std::time::Instant::now() >= deadline {
                self.failure = Some("recreated surface presentation deadline expired".into());
                event_loop.exit();
            } else {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(deadline));
            }
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let backend = match arguments.next().as_deref() {
        None | Some("auto") => voxy_render::GraphicsBackend::Auto,
        Some("metal") => voxy_render::GraphicsBackend::Metal,
        Some("vulkan") => voxy_render::GraphicsBackend::Vulkan,
        Some("dx12") => voxy_render::GraphicsBackend::DirectX12,
        Some("gl") => voxy_render::GraphicsBackend::OpenGl,
        _ => return Err("expected auto|metal|vulkan|dx12|gl and optional --require-nvidia".into()),
    };
    let require_nvidia = match arguments.next().as_deref() {
        None => false,
        Some("--require-nvidia") => true,
        _ => return Err("expected optional --require-nvidia".into()),
    };
    if arguments.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let mut probe = Probe {
        options: voxy_render::GraphicsOptions {
            backend,
            ..Default::default()
        },
        require_nvidia,
        ..Default::default()
    };
    EventLoop::new()?.run_app(&mut probe)?;
    if let Some(error) = probe.failure {
        return Err(error.into());
    }
    if !probe.completed {
        return Err("device-loss probe did not complete".into());
    }
    Ok(())
}

fn check_lost_readback(
    device: &wgpu::Device,
    dispatch: voxy_render::ComputeDispatch,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pending = dispatch.begin_read();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let _ = device.poll(wgpu::PollType::Poll);
        match pending.try_read() {
            Err(voxy_render::ComputeError::Mapping(message)) if !message.is_empty() => {
                break;
            }
            Ok(None) if std::time::Instant::now() < deadline => std::thread::yield_now(),
            other => {
                return Err(format!("lost-device readback failed to terminate: {other:?}").into());
            }
        }
    }
    if pending.try_read() != Err(voxy_render::ComputeError::Consumed) {
        return Err("failed device-loss readback was not consumed".into());
    }
    println!("PASS: destroyed-device compute mapping terminates with a consumed error");
    Ok(())
}

fn check_fresh_compute(renderer: &Renderer) -> Result<(), Box<dyn std::error::Error>> {
    if renderer.device_failure().is_some() {
        return Err("fresh device retained old loss state".into());
    }
    let program = pollster::block_on(voxy_render::ComputeProgram::new(
        renderer.device(),
        "@group(0) @binding(0) var<storage, read_write> data: array<u32>; @compute @workgroup_size(1) fn cs_main() { data[0] = 42u; }",
    ))?;
    let mut encoder = renderer
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let dispatch = program
        .create_job(renderer.device(), &[0; 4])?
        .encode(&mut encoder, [1, 1, 1])?;
    renderer.queue().submit([encoder.finish()]);
    let mut pending = dispatch.begin_read();
    renderer
        .device()
        .poll(wgpu::PollType::wait_indefinitely())?;
    let output = pending
        .try_read()?
        .ok_or("fresh compute readback pending after wait")?;
    if output.as_slice() != 42u32.to_ne_bytes() {
        return Err("fresh compute result mismatch".into());
    }
    println!("PASS: native renderer recreation on the same window restores exact compute readback");
    Ok(())
}

fn check_recreation(
    window: Arc<Window>,
    options: voxy_render::GraphicsOptions,
    instance: wgpu::Instance,
) -> Result<Renderer, Box<dyn std::error::Error>> {
    let renderer = pollster::block_on(Renderer::new_with_instance(
        window, 320, 240, options, instance,
    ))?;
    check_fresh_compute(&renderer)?;
    Ok(renderer)
}
