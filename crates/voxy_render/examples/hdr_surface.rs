//! Native linear HDR swapchain configure/present proof; no photometric display proof.
use std::sync::Arc;
use voxy_render::{
    GraphicsOptions, ProcessedColorTarget, RenderOutcome, RendererError, SceneSurface,
    SurfaceOutput, TextureBlit,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[derive(Default)]
struct Probe {
    window: Option<Arc<Window>>,
    host: Option<SceneSurface>,
    output: SurfaceOutput,
    pq: Option<(ProcessedColorTarget, TextureBlit)>,
    presented: usize,
    attempts: usize,
    tested_suspension: bool,
    failure: Option<String>,
}
impl Probe {
    fn draw(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let host = self.host.as_mut().ok_or("missing HDR surface")?;
        let (format, space) = match self.output {
            SurfaceOutput::HdrLinear => (
                wgpu::TextureFormat::Rgba16Float,
                wgpu::SurfaceColorSpace::ExtendedSrgbLinear,
            ),
            SurfaceOutput::Hdr10 => (
                wgpu::TextureFormat::Rgb10a2Unorm,
                wgpu::SurfaceColorSpace::Bt2100Pq,
            ),
            SurfaceOutput::Sdr => return Err("HDR proof requires an HDR mode".into()),
        };
        assert_eq!(host.color_format(), format);
        assert_eq!(host.color_space(), space);
        if self.presented == 1 && !self.tested_suspension {
            host.resize(0, 0)?;
            assert_eq!(
                host.render_custom(|_, _| Ok::<(), RendererError>(()))?,
                RenderOutcome::Suspended
            );
            host.resize(320, 240)?;
            self.tested_suspension = true;
        }
        self.attempts += 1;
        if self.attempts > 128 {
            return Err("HDR acquisition retries exhausted".into());
        }
        let device = host.device().clone();
        let pq = self.pq.as_ref();
        let outcome = host.render_custom(|encoder, view| {
            let linear_view = pq.map_or(view, |(source, _)| source.view());
            {
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("linear HDR surface sample"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: linear_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 4.0,
                                g: 2.0,
                                b: 0.5,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
            }
            if let Some((source, encoder_pass)) = pq {
                encoder_pass.encode(&device, encoder, source.view(), view);
            }
            Ok::<(), RendererError>(())
        })?;
        if outcome == RenderOutcome::Presented {
            self.presented += 1;
        }
        Ok(self.presented == 3)
    }
}
impl ApplicationHandler for Probe {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Voxy linear HDR proof")
                        .with_inner_size(winit::dpi::PhysicalSize::new(320, 240)),
                )?,
            );
            let options = GraphicsOptions::default();
            let instance = options.create_instance_with_display(event_loop.owned_display_handle());
            let host = pollster::block_on(SceneSurface::new_with_instance_and_output(
                Arc::clone(&window),
                320,
                240,
                options,
                instance,
                self.output,
            ))?;
            println!(
                "Configured HDR {:?}/{:?}; display {:?}",
                host.color_format(),
                host.color_space(),
                host.display_hdr_info()
            );
            if self.output == SurfaceOutput::Hdr10 {
                self.pq = Some((
                    ProcessedColorTarget::new(host.device(), 320, 240, true)?,
                    TextureBlit::hdr10(host.device(), host.color_format(), 203.0)
                        .ok_or("HDR10 display encoder rejected")?,
                ));
            }
            self.host = Some(host);
            window.focus_window();
            window.request_redraw();
            self.window = Some(window);
            Ok(())
        })();
        if let Err(error) = result {
            self.failure = Some(error.to_string());
            event_loop.exit();
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if event == WindowEvent::RedrawRequested {
            match self.draw() {
                Ok(true) => event_loop.exit(),
                Ok(false) => {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
                Err(error) => {
                    self.failure = Some(error.to_string());
                    event_loop.exit();
                }
            }
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = match std::env::var("VOXY_HDR_OUTPUT").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("linear") => SurfaceOutput::HdrLinear,
        Ok("pq") => SurfaceOutput::Hdr10,
        _ => return Err("VOXY_HDR_OUTPUT expects linear|pq".into()),
    };
    let mut probe = Probe {
        output,
        ..Default::default()
    };
    EventLoop::new()?.run_app(&mut probe)?;
    if let Some(error) = probe.failure {
        return Err(error.into());
    }
    if probe.presented != 3 || !probe.tested_suspension {
        return Err("HDR surface proof incomplete".into());
    }
    println!("HDR SURFACE PASS: {output:?}, three presentations, suspension/resume");
    Ok(())
}
