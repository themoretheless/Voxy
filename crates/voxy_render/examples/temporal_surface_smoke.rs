//! Windowed submission-hook/error recovery proof independent of physics.
use std::sync::Arc;
use voxy_render::{
    AutoExposure, AutoExposureFrame, ExposureSettings, GraphicsBackend, GraphicsOptions,
    RenderOutcome, RendererError, SceneDraw, SceneGeometry, SceneMesh, SceneRenderer, SceneSurface,
    SceneTexture, SceneTransform, SurfaceOutput, TextureBlit,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

const FRAME_COMPUTE: &str = "@group(0) @binding(0) var<storage, read_write> value: array<u32>; @compute @workgroup_size(1) fn frame_step() { value[0] = value[0] * 3u + 7u; }";

struct ExposureState {
    engine: AutoExposure,
    previous: Option<AutoExposureFrame>,
    presentation_id: Option<u64>,
}

#[derive(Default)]
struct Smoke {
    options: GraphicsOptions,
    window: Option<Arc<Window>>,
    graphics: Option<(SceneSurface, SceneRenderer, TextureBlit)>,
    attempt: usize,
    skipped: usize,
    next_redraw: Option<std::time::Instant>,
    hdr: bool,
    exposure: Option<ExposureState>,
    retained: Option<(voxy_render::TemporalResolve, voxy_render::TemporalHistory)>,
    layer: Option<(SceneGeometry, SceneTransform)>,
    geometry: Option<(SceneGeometry, SceneTexture, SceneTransform)>,
    failure: Option<String>,
    extinction: Option<voxy_render::DropletExtinctionPass>,
    integrated: Option<(
        voxy_render::ComputeProgram,
        voxy_render::HdrHalfResolvePipeline,
    )>,
}
impl Smoke {
    #[allow(clippy::too_many_lines)]
    fn draw(&mut self) -> Result<bool, String> {
        if let Some((program, _)) = &mut self.integrated
            && self.attempt == 2
            && program.shader_revision() == 0
        {
            if pollster::block_on(program.reload_shader("invalid WGSL")).is_ok()
                || program.shader_revision() != 0
            {
                return Err("invalid integrated shader changed active pipeline".into());
            }
            pollster::block_on(program.reload_shader(&FRAME_COMPUTE.replace("* 3u", "* 5u")))
                .map_err(|error| error.to_string())?;
            if program.shader_revision() != 1 {
                return Err("integrated shader revision did not advance".into());
            }
        }
        let (host, scene, blit) = self.graphics.as_mut().ok_or("missing graphics")?;
        if self.integrated.is_some() && matches!(self.attempt, 1 | 3) {
            let geometry = &mut self.geometry.as_mut().ok_or("missing scene geometry")?.0;
            if self.attempt == 1 {
                let previous = host.temporal_frame().map(|frame| frame.presentation_id);
                if host
                    .replace_scene_mesh(scene, geometry, &SceneMesh::quad([f32::NAN; 4]))
                    .is_ok()
                    || host.temporal_frame().map(|frame| frame.presentation_id) != previous
                {
                    return Err("rejected mesh replacement changed temporal history".into());
                }
            } else {
                host.replace_scene_mesh(scene, geometry, &SceneMesh::quad([2.0, 1.0, 0.0, 1.0]))
                    .map_err(|error| error.to_string())?;
            }
        }
        if self.integrated.is_some() && matches!(self.attempt, 1 | 5) {
            let texture = &mut self.geometry.as_mut().ok_or("missing scene geometry")?.1;
            if self.attempt == 1 {
                let previous = host.temporal_frame().map(|frame| frame.presentation_id);
                if host
                    .replace_scene_texture(
                        scene,
                        texture,
                        1,
                        1,
                        &[255; 3],
                        voxy_render::TextureSampling::default(),
                    )
                    .is_ok()
                    || host.temporal_frame().map(|frame| frame.presentation_id) != previous
                {
                    return Err("rejected material replacement changed temporal history".into());
                }
                if host
                    .replace_scene_image_mips(
                        scene,
                        texture,
                        &[],
                        voxy_render::TextureSampling::default(),
                    )
                    .is_ok()
                    || host.temporal_frame().map(|frame| frame.presentation_id) != previous
                {
                    return Err("rejected mip material changed temporal history".into());
                }
            } else {
                use image::ImageEncoder;
                let mut png = Vec::new();
                image::codecs::png::PngEncoder::new(&mut png)
                    .write_image(&[255; 16], 2, 2, image::ExtendedColorType::Rgba8)
                    .map_err(|error| error.to_string())?;
                let image =
                    voxy_render::ImageAsset::decode(&png, voxy_render::ImageLimits::default())
                        .map_err(|error| error.to_string())?;
                host.replace_scene_image_mips(
                    scene,
                    texture,
                    &image.mip_chain(),
                    voxy_render::TextureSampling {
                        anisotropy: 4,
                        min_filter: voxy_render::TextureFilter::Linear,
                        mag_filter: voxy_render::TextureFilter::Linear,
                        ..voxy_render::TextureSampling::default()
                    },
                )
                .map_err(|error| error.to_string())?;
            }
        }
        let mut draws = probe_draws(
            self.geometry.as_ref().ok_or("missing scene geometry")?,
            self.layer.as_ref(),
        );
        if self.extinction.is_some() {
            draws.retain(|draw| !draw.overlay);
        }
        let extinction = self.extinction.as_ref();
        let attempt = self.attempt;
        let hdr = self.hdr;
        let integrated = self.integrated.as_ref();
        let multiplier = integrated.map_or(
            3,
            |(program, _)| {
                if program.shader_revision() == 0 { 3 } else { 5 }
            },
        );
        let exposure = self.exposure.as_mut();
        let retained_enabled = std::env::var("VOXY_TEMPORAL_RETAINED").as_deref() == Ok("1");
        let previous_history = self
            .retained
            .as_ref()
            .map(|(_, history)| history.color().clone());
        let retained = &mut self.retained;
        let previous_exposure_id = exposure.as_ref().and_then(|state| state.presentation_id);
        let mut candidate = None;
        let expected_motion = if std::env::var("VOXY_TEMPORAL_RG_MOTION").as_deref() == Ok("1") {
            wgpu::TextureFormat::Rg16Float
        } else {
            wgpu::TextureFormat::Rgba16Float
        };
        let expected_format = if self.hdr {
            wgpu::TextureFormat::Rgba16Float
        } else {
            host.color_format()
        };
        let mut prepared = None;
        let mut submitted = None;
        let outcome = host.render_scene_with_temporal_hooks(
            scene,
            &draws,
            |input, _| prepared = Some((input.presentation_id, input.reset_history)),
            |input, device, queue, target| {
                submitted = Some((input.presentation_id, input.reset_history));
                if input.color.format() != expected_format
                    || input.motion.format() != expected_motion
                    || input.motion.size() != input.depth.size()
                    || input.color.size() != input.depth.size()
                {
                    return Err(RendererError::TemporalConsumer(
                        "temporal formats/sizes differ".into(),
                    ));
                }
                if hdr && attempt == 0 {
                    validate_hdr_center(input.color, device, queue)
                        .map_err(RendererError::TemporalConsumer)?;
                    validate_opaque_inputs(input, device, queue, if extinction.is_some() { 128 } else { 0 })
                        .map_err(RendererError::TemporalConsumer)?;
                }
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                if retained_enabled {
                    if !hdr || input.motion.format() != wgpu::TextureFormat::Rg16Float {
                        return Err(RendererError::TemporalConsumer(
                            "retained history proof requires HDR and RG motion".into(),
                        ));
                    }
                    if retained.is_none() {
                        *retained = Some((
                            voxy_render::TemporalResolve::new(device)
                                .map_err(|e| RendererError::TemporalConsumer(e.to_string()))?,
                            voxy_render::TemporalHistory::new(
                                device,
                                input.color.width(),
                                input.color.height(),
                            )
                            .map_err(|e| RendererError::TemporalConsumer(e.to_string()))?,
                        ));
                    }
                    let (resolve, history) = retained.as_mut().expect("initialized history");
                    history
                        .resize(input.color.width(), input.color.height())
                        .map_err(|e| RendererError::TemporalConsumer(e.to_string()))?;
                    history
                        .encode_depth_attachment(&mut encoder, input.depth)
                        .map_err(|e| RendererError::TemporalConsumer(e.to_string()))?;
                    let frame = resolve
                        .prepare_into(
                            voxy_render::TemporalResolveInputs {
                                current: input.color,
                                motion: input.motion,
                                history: history.color(),
                                expected_previous_depth: history.depth(),
                                history_depth: history.depth(),
                            },
                            voxy_render::TemporalResolveOptions {
                                history_weight: 0.0,
                                depth_tolerance: 0.0,
                                reset_history: true,
                            },
                            history.output(),
                            false,
                        )
                        .map_err(|e| RendererError::TemporalConsumer(e.to_string()))?;
                    frame.encode(&mut encoder);
                }
                let proof_value = u32::try_from(input.presentation_id)
                    .map_err(|error| RendererError::TemporalConsumer(error.to_string()))?;
                let (compute, resolved) = if let Some((program, pipeline)) = integrated {
                    let job = program
                        .create_job(device, bytemuck::bytes_of(&proof_value))
                        .map_err(|error| RendererError::TemporalConsumer(error.to_string()))?;
                    for _ in 0..2 {
                        job.encode_step(&mut encoder, [1, 1, 1])
                            .map_err(|error| RendererError::TemporalConsumer(error.to_string()))?;
                    }
                    let compute = job
                        .encode_readback(&mut encoder)
                        .map_err(|error| RendererError::TemporalConsumer(error.to_string()))?;
                    let resolved = pipeline
                        .prepare(input.color)
                        .map_err(|error| RendererError::TemporalConsumer(error.to_string()))?;
                    resolved.encode(&mut encoder);
                    (Some(compute), Some(resolved))
                } else {
                    (None, None)
                };
                let extinction_frame = if let Some(pass) = extinction {
                    let mut input_storage = voxy_render::DropletExtinctionSceneInput::new(
                        voxy_render::ExtinctionGridView {
                            origin: [-1., -1., 0.], spacing: [2., 2., 1.], shape: [1, 1, 1], extinction_m_inverse: &[0.5],
                        }, extinction_camera(), input.color.width(), input.color.height(), 16_000_000,
                    ).map_err(|e| RendererError::TemporalConsumer(e.to_string()))?;
                    if std::env::var("VOXY_TEMPORAL_SCATTERING").as_deref() == Ok("1") {
                        input_storage = input_storage.with_directional_scattering(
                            voxy_render::DirectionalScatteringOptions {
                                direction_to_light:[0.,0.,1.],irradiance_rgb:[4.,2.,1.],albedo:0.8,asymmetry:0.,samples:32,
                            },16_000_000,64_000_000,
                        ).map_err(|e| RendererError::TemporalConsumer(e.to_string()))?;
                    }
                    if pollster::block_on(pass.prepare(&input_storage, input.color, input.depth, 0)).is_ok() {
                        return Err(RendererError::TemporalConsumer("zero extinction output budget admitted".into()));
                    }
                    let frame = pollster::block_on(pass.prepare(&input_storage, input.color, input.depth, 2_000_000))
                        .map_err(|e| RendererError::TemporalConsumer(e.to_string()))?;
                    pollster::block_on(frame.encode(&mut encoder))
                        .map_err(|e| RendererError::TemporalConsumer(e.to_string()))?;
                    Some(frame)
                } else { None };
                let display = prepare_display_exposure(
                    exposure.as_deref(),
                    input,
                    blit,
                    &mut encoder,
                    &mut candidate,
                )?;
                display.as_ref().unwrap_or(blit).encode(
                    device,
                    &mut encoder,
                    &extinction_frame.as_ref().map_or_else(
                        || resolved.as_ref().map_or(input.color, voxy_render::HdrHalfResolveJob::output),
                        voxy_render::DropletExtinctionFrame::output)
                        .create_view(&wgpu::TextureViewDescriptor::default()),
                    target,
                );
                if attempt == 1 {
                    Err(RendererError::TemporalConsumer("injected".into()))
                } else {
                    queue.submit([encoder.finish()]);
                    if let Some(frame) = &extinction_frame {
                        validate_extinction_center(frame.output(), device, queue)
                            .map_err(RendererError::TemporalConsumer)?;
                        println!("EXTINCTION WINDOW FRAME id={} size={}x{} bytes={} cpu_scene_upload=false", input.presentation_id, input.color.width(), input.color.height(), frame.allocation_bytes());
                    }
                    if let Some(compute) = compute {
                        let mut read = compute.begin_read();
                        device
                            .poll(wgpu::PollType::wait_indefinitely())
                            .map_err(|error| RendererError::TemporalConsumer(error.to_string()))?;
                        let bytes = read
                            .try_read()
                            .map_err(|error| RendererError::TemporalConsumer(error.to_string()))?
                            .ok_or_else(|| {
                                RendererError::TemporalConsumer("compute readback pending".into())
                            })?;
                        if bytes.as_slice()
                            != proof_value
                                .wrapping_mul(multiplier)
                                .wrapping_add(7)
                                .wrapping_mul(multiplier)
                                .wrapping_add(7)
                                .to_le_bytes()
                        {
                            return Err(RendererError::TemporalConsumer(
                                "integrated compute result differs".into(),
                            ));
                        }
                    }
                    Ok(())
                }
            },
        );
        if prepared.is_none()
            && submitted.is_none()
            && matches!(
                outcome,
                Ok(RenderOutcome::SkippedOccluded
                    | RenderOutcome::SkippedTimeout
                    | RenderOutcome::Reconfigured)
            )
        {
            self.skipped += 1;
            if self.skipped > 3000 {
                return Err(format!("surface did not become ready: {outcome:?}"));
            }
            return Ok(false);
        }
        let expected = verify_callbacks(
            host,
            attempt,
            integrated.is_some(),
            prepared,
            submitted,
            &outcome,
        )?;
        if let Some(state) = &mut self.exposure {
            if attempt == 1 {
                if state.presentation_id != previous_exposure_id || candidate.is_none() {
                    return Err("failed exposure candidate changed presented history".into());
                }
            } else {
                state.previous = Some(candidate.ok_or("presented frame missing GPU exposure")?);
                state.presentation_id = Some(expected.0);
            }
        }
        if let Some((_, history)) = &mut self.retained {
            if attempt == 1 {
                if previous_history.as_ref() != Some(history.color()) {
                    return Err("failed consumer committed retained history".into());
                }
            } else {
                history.presented();
                if !history.valid() {
                    return Err("presented history invalid".into());
                }
            }
        }
        if attempt == 3 {
            resize_probe(host, self.window.as_deref())?;
        }
        self.attempt += 1;
        Ok(self.attempt == if self.integrated.is_some() { 6 } else { 5 })
    }
}
impl ApplicationHandler for Smoke {
    #[allow(clippy::too_many_lines)]
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<(), String> {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("Voxy temporal hooks smoke")
                            .with_inner_size(winit::dpi::PhysicalSize::new(320, 240)),
                    )
                    .map_err(|error| error.to_string())?,
            );
            let instance = self
                .options
                .create_instance_with_display(event_loop.owned_display_handle());
            let output = match std::env::var("VOXY_TEMPORAL_OUTPUT").as_deref() {
                Err(std::env::VarError::NotPresent) | Ok("sdr") => SurfaceOutput::Sdr,
                Ok("linear") => SurfaceOutput::HdrLinear,
                Ok("pq") => SurfaceOutput::Hdr10,
                _ => return Err("VOXY_TEMPORAL_OUTPUT expects sdr|linear|pq".into()),
            };
            let mut host = pollster::block_on(SceneSurface::new_with_instance_and_output(
                window.clone(),
                320,
                240,
                self.options,
                instance,
                output,
            ))
            .map_err(|error| error.to_string())?;
            let auto_exposure = std::env::var("VOXY_TEMPORAL_AUTO_EXPOSURE").as_deref() == Ok("1");
            let integrated = std::env::var("VOXY_TEMPORAL_INTEGRATED").as_deref() == Ok("1");
            let extinction = std::env::var("VOXY_TEMPORAL_EXTINCTION").as_deref() == Ok("1");
            if !extinction && std::env::var("VOXY_TEMPORAL_SCATTERING").as_deref() == Ok("1") {
                return Err("scattering window qualification requires extinction mode".into());
            }
            if extinction && (integrated || auto_exposure || output != SurfaceOutput::Sdr) {
                return Err("extinction window qualification requires standalone SDR tone-mapped HDR inputs".into());
            }
            let hdr = integrated
                || extinction
                || auto_exposure
                || std::env::var("VOXY_TEMPORAL_HDR").as_deref() == Ok("1");
            self.hdr = hdr || output != SurfaceOutput::Sdr;
            if hdr {
                pollster::block_on(host.enable_hdr_temporal_color())
                    .map_err(|error| error.to_string())?;
            } else {
                pollster::block_on(host.enable_motion_vectors())
                    .map_err(|error| error.to_string())?;
            }
            if std::env::var("VOXY_TEMPORAL_RG_MOTION").as_deref() == Ok("1") {
                pollster::block_on(host.enable_two_channel_motion_vectors())
                    .map_err(|error| error.to_string())?;
            }
            println!("Temporal hooks on {:?}", host.adapter_info());
            let scene = host.create_scene_renderer();
            if extinction {
                self.extinction = Some(
                    pollster::block_on(voxy_render::DropletExtinctionPass::new(host.device()))
                        .map_err(|e| e.to_string())?,
                );
            }
            if integrated {
                let program =
                    pollster::block_on(host.create_compute_program(FRAME_COMPUTE, "frame_step"))
                        .map_err(|error| error.to_string())?;
                self.integrated = Some((
                    program,
                    host.create_hdr_resolve_pipeline()
                        .map_err(|error| error.to_string())?,
                ));
            }
            let geometry = scene
                .upload_mesh(host.device(), &SceneMesh::quad([4.0, 1.0, 0.0, 1.0]))
                .map_err(|error| error.to_string())?;
            let texture = scene
                .upload_texture(host.device(), host.queue(), 1, 1, &[255; 4])
                .map_err(|error| error.to_string())?;
            let transform = scene
                .create_transform(
                    host.device(),
                    if extinction {
                        extinction_camera()
                            .view_projection()
                            .map_err(|e| e.to_string())?
                    } else {
                        glam::Mat4::IDENTITY
                    },
                )
                .map_err(|error| error.to_string())?;
            self.layer = layer_geometry(&scene, host.device(), host.queue())?;
            self.geometry = Some((geometry, texture, transform));
            let blit = if output == SurfaceOutput::Hdr10 {
                TextureBlit::hdr10(host.device(), host.color_format(), 203.0)
                    .ok_or("invalid HDR10 encoder")?
            } else if output == SurfaceOutput::HdrLinear && auto_exposure {
                TextureBlit::linear_exposed(host.device(), host.color_format(), 1.0)
                    .ok_or("linear exposure rejected")?
            } else if output == SurfaceOutput::HdrLinear {
                TextureBlit::new(host.device(), host.color_format())
            } else if hdr {
                TextureBlit::tone_mapped(host.device(), host.color_format(), 1.0)
                    .ok_or("invalid exposure")?
            } else {
                TextureBlit::new(host.device(), host.color_format())
            };
            if auto_exposure {
                self.exposure = Some(ExposureState {
                    engine: AutoExposure::new(host.device()).map_err(|error| error.to_string())?,
                    previous: None,
                    presentation_id: None,
                });
            }
            self.graphics = Some((host, scene, blit));
            window.focus_window();
            window.request_redraw();
            self.window = Some(window);
            Ok(())
        })();
        if let Err(error) = result {
            self.failure = Some(error);
            event_loop.exit();
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self.next_redraw {
            if std::time::Instant::now() >= deadline {
                self.next_redraw = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
            } else {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(deadline));
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if self.failure.is_some()
            || self.graphics.is_none()
            || self.attempt >= if self.integrated.is_some() { 6 } else { 5 }
        {
            return;
        }
        if event != WindowEvent::RedrawRequested {
            return;
        }
        match self.draw() {
            Ok(true) => {
                println!(
                    "Temporal prepare/submit/publish and consumer-failure/resize reset passed"
                );
                event_loop.exit();
            }
            Ok(false) => {
                self.next_redraw =
                    Some(std::time::Instant::now() + std::time::Duration::from_millis(16));
            }
            Err(error) => {
                self.failure = Some(error);
                event_loop.exit();
            }
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = match std::env::var("VOXY_TEMPORAL_BACKEND").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("auto") => GraphicsBackend::Auto,
        Ok("metal") => GraphicsBackend::Metal,
        Ok("dx12") => GraphicsBackend::DirectX12,
        Ok("vulkan") => GraphicsBackend::Vulkan,
        Ok("gl") => GraphicsBackend::OpenGl,
        _ => return Err("VOXY_TEMPORAL_BACKEND expects auto|metal|dx12|vulkan|gl".into()),
    };
    let mut smoke = Smoke {
        options: GraphicsOptions {
            backend,
            ..Default::default()
        },
        ..Default::default()
    };
    EventLoop::new()?.run_app(&mut smoke)?;
    if let Some(error) = smoke.failure {
        return Err(error.into());
    }
    if smoke.attempt != if smoke.integrated.is_some() { 6 } else { 5 } {
        return Err("smoke exited before completing all attempts".into());
    }
    Ok(())
}

fn validate_hdr_center(
    texture: &wgpu::Texture,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), String> {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("HDR scene center readback"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: texture.width() / 2,
                y: texture.height() / 2,
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let index = queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(index),
            timeout: Some(std::time::Duration::from_secs(5)),
        })
        .map_err(|error| error.to_string())?;
    receiver
        .recv_timeout(std::time::Duration::from_secs(1))
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    let mapped = buffer
        .slice(..)
        .get_mapped_range()
        .map_err(|error| error.to_string())?;
    // IEEE binary16 scene output (4,1,0,1), proving values above one survive.
    let expected = match std::env::var("VOXY_TEMPORAL_LAYER").as_deref() {
        Ok("transparent") => [0x4000u16, 0x3c00, 0, 0x3c00],
        Ok("xray") => [0u16, 0, 0x4400, 0x3c00],
        _ => [0x4400u16, 0x3c00, 0, 0x3c00],
    };
    for (bytes, expected) in mapped[..8].chunks_exact(2).zip(expected) {
        if u16::from_le_bytes([bytes[0], bytes[1]]) != expected {
            return Err(format!("HDR geometry pixel mismatch: {:?}", &mapped[..8]));
        }
    }
    drop(mapped);
    buffer.unmap();
    println!("HDR scene/layer geometry readback matches {expected:?}");
    Ok(())
}

fn resize_probe(host: &mut SceneSurface, window: Option<&Window>) -> Result<(), String> {
    let requested = winit::dpi::PhysicalSize::new(321, 241);
    let size = window
        .and_then(|window| window.request_inner_size(requested))
        .unwrap_or(requested);
    host.resize(size.width, size.height)
        .map_err(|error| error.to_string())?;
    if host.temporal_frame().is_some() {
        return Err("resize retained stale published inputs".into());
    }
    Ok(())
}

fn probe_draws<'a>(
    resources: &'a (SceneGeometry, SceneTexture, SceneTransform),
    layer: Option<&'a (SceneGeometry, SceneTransform)>,
) -> Vec<SceneDraw<'a>> {
    let (geometry, texture, transform) = resources;
    let mut draws = vec![SceneDraw {
        geometry,
        texture,
        transform,
        overlay: false,
    }];
    if let Some((geometry, transform)) = layer {
        draws.push(SceneDraw {
            geometry,
            texture,
            transform,
            overlay: false,
        });
    }
    draws.push(SceneDraw {
        geometry,
        texture,
        transform,
        overlay: true,
    });
    draws
}
fn layer_geometry(
    scene: &SceneRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Option<(SceneGeometry, SceneTransform)>, String> {
    use voxy_render::SceneDepthMode;
    let (color, mode) = match std::env::var("VOXY_TEMPORAL_LAYER").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("opaque") => return Ok(None),
        Ok("transparent") => ([0.0, 1.0, 0.0, 0.5], SceneDepthMode::Transparent),
        Ok("xray") => ([0.0, 0.0, 4.0, 1.0], SceneDepthMode::Xray),
        _ => return Err("VOXY_TEMPORAL_LAYER expects opaque|transparent|xray".into()),
    };
    let mut geometry = scene
        .upload_mesh(device, &SceneMesh::quad(color))
        .map_err(|error| error.to_string())?;
    geometry.set_depth_mode(mode);
    // Put X-ray internals behind opaque depth=0; shared-depth rendering would hide them.
    let z = if mode == SceneDepthMode::Xray {
        0.75
    } else {
        0.0
    };
    let transform = scene
        .create_transform(
            device,
            glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, z)),
        )
        .map_err(|error| error.to_string())?;
    transform
        .update_motion(
            queue,
            voxy_render::MotionMatrices {
                current: glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, z)),
                previous: glam::Mat4::from_translation(glam::Vec3::new(0.5, 0.0, z)),
                history_valid: true,
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(Some((geometry, transform)))
}

fn validate_opaque_inputs(
    input: &voxy_render::TemporalFrame<'_>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    expected_depth: u8,
) -> Result<(), String> {
    let converted = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("portable depth readback"),
        size: input.depth.size(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    TextureBlit::depth(device, wgpu::TextureFormat::Rgba8Unorm).encode(
        device,
        &mut encoder,
        &input
            .depth
            .create_view(&wgpu::TextureViewDescriptor::default()),
        &converted.create_view(&wgpu::TextureViewDescriptor::default()),
    );
    queue.submit([encoder.finish()]);
    let depth = read_temporal_center(&converted, device, queue, 4, wgpu::TextureAspect::All)?;
    if depth[..3].iter().any(|v| v.abs_diff(expected_depth) > 1) || depth[3] != 255 {
        return Err(format!("opaque depth contaminated: {depth:?}"));
    }
    let rg = input.motion.format() == wgpu::TextureFormat::Rg16Float;
    let motion = read_temporal_center(
        input.motion,
        device,
        queue,
        if rg { 4 } else { 8 },
        wgpu::TextureAspect::All,
    )?;
    let channels = if rg { &motion[..] } else { &motion[..6] };
    if channels
        .chunks_exact(2)
        .any(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) & 0x7fff != 0)
        || (!rg && motion[6..] != [0, 0x3c])
    {
        return Err(format!("opaque motion contaminated: {motion:?}"));
    }
    println!(
        "Opaque depth byte={expected_depth} and stationary motion preserved despite moving world layer"
    );
    Ok(())
}
fn read_temporal_center(
    texture: &wgpu::Texture,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bytes_per_pixel: u32,
    aspect: wgpu::TextureAspect,
) -> Result<Vec<u8>, String> {
    let row = (texture.width() * bytes_per_pixel).next_multiple_of(256);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("temporal depth/motion readback"),
        size: u64::from(row) * u64::from(texture.height()),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    // Full subresource copy also respects depth-copy requirements on DX12.
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(texture.height()),
            },
        },
        texture.size(),
    );
    let index = queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(index),
            timeout: Some(std::time::Duration::from_secs(5)),
        })
        .map_err(|error| error.to_string())?;
    receiver
        .recv_timeout(std::time::Duration::from_secs(1))
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    let mapped = buffer
        .slice(..)
        .get_mapped_range()
        .map_err(|error| error.to_string())?;
    let offset = (texture.height() / 2 * row + texture.width() / 2 * bytes_per_pixel) as usize;
    let pixel = mapped[offset..offset + bytes_per_pixel as usize].to_vec();
    drop(mapped);
    buffer.unmap();
    Ok(pixel)
}

fn prepare_display_exposure(
    state: Option<&ExposureState>,
    input: &voxy_render::TemporalFrame<'_>,
    blit: &TextureBlit,
    encoder: &mut wgpu::CommandEncoder,
    candidate: &mut Option<AutoExposureFrame>,
) -> Result<Option<TextureBlit>, RendererError> {
    let Some(state) = state else {
        return Ok(None);
    };
    let history = if input.reset_history {
        None
    } else {
        state.previous.as_ref()
    };
    let frame = state
        .engine
        .prepare(
            input.color,
            history,
            ExposureSettings::default(),
            1.0 / 60.0,
        )
        .map_err(|error| RendererError::TemporalConsumer(error.to_string()))?;
    frame.encode(encoder);
    let display = blit.with_auto_exposure(&frame).ok_or_else(|| {
        RendererError::TemporalConsumer("display pass does not accept GPU exposure".into())
    })?;
    *candidate = Some(frame);
    Ok(Some(display))
}

fn verify_callbacks(
    host: &SceneSurface,
    attempt: usize,
    integrated: bool,
    prepared: Option<(u64, bool)>,
    submitted: Option<(u64, bool)>,
    outcome: &Result<RenderOutcome, RendererError>,
) -> Result<(u64, bool), String> {
    let expected = [
        (1, true),
        (2, false),
        (2, true),
        (3, integrated),
        (4, true),
        (5, true),
    ][attempt];
    if prepared != Some(expected) || submitted != prepared {
        return Err(format!(
            "callback mismatch: {prepared:?}/{submitted:?}, expected {expected:?}"
        ));
    }
    if attempt == 1 {
        if !matches!(outcome, Err(RendererError::TemporalConsumer(error)) if error == "injected")
            || host.temporal_frame().is_some()
        {
            return Err("consumer failure did not invalidate published history".into());
        }
    } else {
        if !matches!(outcome, Ok(RenderOutcome::Presented)) {
            return Err(format!("unexpected render outcome: {outcome:?}"));
        }
        let published = host.temporal_frame().ok_or("missing published inputs")?;
        if (published.presentation_id, published.reset_history) != expected {
            return Err("published inputs differ from callbacks".into());
        }
    }
    Ok(expected)
}

fn extinction_camera() -> voxy_render::SceneCamera {
    voxy_render::SceneCamera {
        eye: glam::Vec3::Z,
        target: glam::Vec3::ZERO,
        up: glam::Vec3::Y,
        projection: voxy_render::SceneProjection::Orthographic {
            left: -1.,
            right: 1.,
            bottom: -1.,
            top: 1.,
            near: 0.,
            far: 2.,
        },
    }
}
fn validate_extinction_center(
    texture: &wgpu::Texture,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), String> {
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("window extinction center qualification"),
        size: 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: texture.width() / 2,
                y: texture.height() / 2,
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| e.to_string())?;
    rx.recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let mapped = staging
        .slice(..)
        .get_mapped_range()
        .map_err(|e| e.to_string())?;
    let scattered = if std::env::var("VOXY_TEMPORAL_SCATTERING").as_deref() == Ok("1") {
        0.8 * (1. - (-1_f32).exp()) / (8. * std::f32::consts::PI)
    } else {
        0.
    };
    for (k, expected) in [
        4. * ((-0.5_f32).exp() + scattered),
        (-0.5_f32).exp() + 2. * scattered,
        scattered,
        1.,
    ]
    .into_iter()
    .enumerate()
    {
        let actual = half::f16::from_bits(u16::from_le_bytes(
            mapped[k * 2..k * 2 + 2].try_into().unwrap(),
        ))
        .to_f32();
        if (actual - expected).abs() > 0.002 {
            return Err(format!(
                "window extinction center channel={k} actual={actual} expected={expected}"
            ));
        }
    }
    drop(mapped);
    staging.unmap();
    Ok(())
}
