//! Native planar mirror with moving emissive geometry and presented-only history.
use glam::{Mat4, Vec2, Vec3};
#[path = "support/demo_controls.rs"]
mod demo_controls;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use voxy_render::*;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

fn emitter(offset: f32) -> Result<SceneMesh, Box<dyn std::error::Error>> {
    Ok(SceneMesh::new(
        [
            [-0.8 + offset, -0.8, 3.0],
            [0.8 + offset, -0.8, 3.0],
            [offset, 0.8, 3.0],
        ]
        .map(|position| SceneVertex {
            position,
            uv: [0.0; 2],
            color: [1.0; 4],
        })
        .to_vec(),
        vec![0, 1, 2],
    )?)
}
fn mirror(curved: bool) -> Result<SceneMesh, Box<dyn std::error::Error>> {
    if curved {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for y in 0..=16 {
            for x in 0..=16 {
                let px = x as f32 / 8.0 - 1.0;
                let py = y as f32 / 8.0 - 1.0;
                vertices.push(SceneVertex {
                    position: [px, py, 0.5 + 0.15 * (px * px + py * py)],
                    uv: [x as f32 / 16.0, y as f32 / 16.0],
                    color: [0.7, 0.7, 0.7, 1.0],
                });
                if x < 16 && y < 16 {
                    let i = y * 17 + x;
                    indices.extend([i, i + 1, i + 18, i, i + 18, i + 17]);
                }
            }
        }
        return Ok(SceneMesh::new(vertices, indices)?);
    }
    Ok(SceneMesh::new(
        [
            [-1.0, -1.0, 0.5],
            [1.0, -1.0, 0.5],
            [1.0, 1.0, 0.5],
            [-1.0, 1.0, 0.5],
        ]
        .map(|position| SceneVertex {
            position,
            uv: [(position[0] + 1.0) * 0.5, (position[1] + 1.0) * 0.5],
            color: [0.7, 0.7, 0.7, 1.0],
        })
        .to_vec(),
        vec![0, 1, 2, 0, 2, 3],
    )?)
}
#[allow(unsafe_code)]
fn experimental() -> wgpu::ExperimentalFeatures {
    unsafe { wgpu::ExperimentalFeatures::enabled() }
}
fn check_offscreen(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    size: [u32; 2],
    hdr: &wgpu::Texture,
    ui_pixel: [u32; 2],
    capture: Option<&std::path::Path>,
    encode: impl FnOnce(
        &mut wgpu::CommandEncoder,
        &wgpu::TextureView,
    ) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("demo offscreen display validation"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let row_bytes = (size[0] * 4).div_ceil(256) * 256;
    let capture_size = if capture.is_some() {
        u64::from(row_bytes) * u64::from(size[1])
    } else {
        0
    };
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("demo HDR and controls pixels"),
        size: 512 + capture_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encode(&mut encoder, &target.create_view(&Default::default()))?;
    for (texture, origin, offset) in [
        (
            hdr,
            wgpu::Origin3d {
                x: size[0] / 2,
                y: size[1] / 2,
                z: 0,
            },
            0,
        ),
        (
            &target,
            wgpu::Origin3d {
                x: ui_pixel[0],
                y: ui_pixel[1],
                z: 0,
            },
            256,
        ),
    ] {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset,
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
    }
    if capture.is_some() {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 512,
                    bytes_per_row: Some(row_bytes),
                    rows_per_image: Some(size[1]),
                },
            },
            target.size(),
        );
    }
    queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    receiver.recv()??;
    if let Some(error) = pollster::block_on(scope.pop()) {
        return Err(error.into());
    }
    let pixels = readback.slice(..).get_mapped_range()?;
    let color: [f32; 4] =
        std::array::from_fn(|i| f32::from_le_bytes(pixels[i * 4..i * 4 + 4].try_into().unwrap()));
    if color.iter().any(|v| !v.is_finite()) || color[0] <= 1.0 || color[3] != 1.0 {
        return Err(format!("expected finite emissive HDR at mirror centre, got {color:?}").into());
    }
    if pixels[256..260] != [255; 4] {
        return Err("UI control did not reach the display target".into());
    }
    if let Some(path) = capture {
        let mut rgba = Vec::with_capacity((size[0] * size[1] * 4) as usize);
        for row in pixels[512..].chunks_exact(row_bytes as usize) {
            rgba.extend_from_slice(&row[..(size[0] * 4) as usize]);
        }
        match format {
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                for pixel in rgba.chunks_exact_mut(4) {
                    pixel.swap(0, 2);
                }
            }
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {}
            _ => return Err("capture requires an RGBA/BGRA8 display format".into()),
        }
        image::RgbaImage::from_raw(size[0], size[1], rgba)
            .ok_or("capture extent mismatch")?
            .save(path)?;
        println!("PLANAR GPU CAPTURE: {}", path.display());
    }
    drop(pixels);
    readback.unmap();
    Ok(())
}
#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    host: Option<SceneSurface>,
    scene: Option<RayScene>,
    texture: Option<SceneTexture>,
    lighting: Option<GgxRayLightingPipeline>,
    temporal: Option<PlanarTemporalPipeline>,
    history: Option<TemporalHistory>,
    blit: Option<TextureBlit>,
    controls: Option<demo_controls::DemoControls>,
    ui: Option<voxy_ui_winit::WindowUi>,
    disable_temporal: bool,
    roughness: f32,
    curved: bool,
    mirror_geometry: Option<ReconstructionGuideMesh>,
    spatial: Option<ReflectionSpatialPipeline>,
    offscreen_check: bool,
    always_on_top: bool,
    checked_frames: u32,
    recoveries: u32,
    recovery_check: bool,
    resize_check: bool,
    resize_origin: Option<[u32; 2]>,
    capture: Option<std::path::PathBuf>,
    previous_emitter: Option<f32>,
    previous_camera: Option<Mat4>,
    clock: voxy_runtime::FrameLoop,
    tick: Option<Instant>,
    phase: f32,
    paused: bool,
    suspended: bool,
    occluded: bool,
    size: [u32; 2],
    backend: GraphicsBackend,
    smoke: bool,
    frames: u32,
    deadline: Option<Instant>,
    retry: Option<Instant>,
    failure: Option<String>,
}
impl App {
    fn reset(&mut self) {
        if let Some(ui) = &mut self.ui {
            let _ = ui.invalidate_presentation();
        }
        if let Some(history) = &mut self.history {
            history.reset();
        }
        self.previous_emitter = None;
        self.previous_camera = None;
        self.clock.reset();
        self.tick = Some(Instant::now());
    }
    fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        let window = if let Some(window) = &self.window {
            window.clone()
        } else {
            Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("Voxy planar mirror | Space pause | R reset | Esc exit")
                        .with_inner_size(winit::dpi::LogicalSize::new(640, 480)),
                )?,
            )
        };
        let instance = GraphicsOptions {
            backend: self.backend,
            ..Default::default()
        }
        .create_instance_with_display(event_loop.owned_display_handle());
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))?;
        if !adapter
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
        {
            return Err("selected surface adapter lacks experimental ray query".into());
        }
        drop(surface);
        let size = window.inner_size();
        let host = pollster::block_on(SceneSurface::new_with_adapter_and_output_experimental(
            window.clone(),
            size.width,
            size.height,
            &instance,
            adapter,
            wgpu::Features::EXPERIMENTAL_RAY_QUERY,
            SurfaceOutput::Sdr,
            experimental(),
        ))?;
        println!("PLANAR SCENE GPU: {:?}", host.adapter().get_info());
        let device = host.device();
        self.scene = Some(RayScene::from_scene_mesh(device, &emitter(0.0)?, 1)?);
        self.texture = Some(host.create_scene_renderer().upload_texture(
            device,
            host.queue(),
            2,
            2,
            &[
                255, 255, 255, 255, 180, 200, 255, 255, 180, 200, 255, 255, 255, 255, 255, 255,
            ],
        )?);
        self.lighting = Some(GgxRayLightingPipeline::new(device)?);
        self.temporal = Some(PlanarTemporalPipeline::new(device)?);
        self.spatial = Some(ReflectionSpatialPipeline::new(device)?);
        self.mirror_geometry = Some(ReconstructionGuideMesh::from_scene_mesh(
            device,
            &mirror(self.curved)?,
            Mat4::IDENTITY,
            0.8,
            self.roughness,
        )?);
        self.history = Some(TemporalHistory::new(device, size.width, size.height)?);
        self.blit = Some(
            TextureBlit::tone_mapped(device, host.color_format(), 1.0)
                .ok_or("tone mapping unavailable")?,
        );
        self.controls = Some(demo_controls::DemoControls::new(&host)?);
        let mut ui = voxy_ui_winit::WindowUi::new(3);
        ui.set_scale(window.scale_factor())?;
        self.ui = Some(ui);
        self.size = [size.width, size.height];
        self.host = Some(host);
        self.retry = None;
        self.deadline
            .get_or_insert(Instant::now() + Duration::from_secs(60));
        self.reset();
        if self.always_on_top {
            window.set_window_level(winit::window::WindowLevel::AlwaysOnTop);
        }
        window.focus_window();
        window.request_redraw();
        self.window = Some(window);
        Ok(())
    }
    fn draw(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        if (self.smoke || self.offscreen_check)
            && self.deadline.is_some_and(|t| Instant::now() >= t)
        {
            return Err(
                format!("planar check timed out after {} presentations", self.frames).into(),
            );
        }
        if self.suspended {
            return Ok(());
        }
        if self.occluded && !self.offscreen_check {
            self.retry = Some(Instant::now() + Duration::from_millis(100));
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.retry.unwrap()));
            return Ok(());
        }
        let window = self.window.as_ref().ok_or("missing window")?.clone();
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            self.reset();
            return Ok(());
        }
        if self.resize_check
            && self.checked_frames == 1
            && self.resize_origin == Some([size.width, size.height])
        {
            self.retry = Some(Instant::now() + Duration::from_millis(100));
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.retry.unwrap()));
            return Ok(());
        }
        if self.size != [size.width, size.height] {
            self.host
                .as_mut()
                .ok_or("missing host")?
                .resize(size.width, size.height)?;
            self.history
                .as_mut()
                .ok_or("missing history")?
                .resize(size.width, size.height)?;
            self.size = [size.width, size.height];
            self.reset();
        }
        let now = Instant::now();
        let delta = self
            .tick
            .replace(now)
            .map_or(0.0, |old| now.duration_since(old).as_secs_f64());
        self.clock.set_paused(self.paused);
        self.phase += self
            .clock
            .advance(if self.smoke || self.offscreen_check {
                1.0 / 60.0
            } else {
                delta
            })
            .delta_seconds as f32;
        let offset = self.phase.sin() * 0.7;
        let perfect_planar = !self.curved && self.roughness == 0.0;
        let host = self.host.as_mut().ok_or("missing host")?;
        if let Some(reason) = host.device_failure() {
            return Err(RendererError::DeviceLost(reason.to_owned()).into());
        }
        let device = host.device().clone();
        let logical = size.to_logical::<f32>(window.scale_factor());
        let controls = self.controls.as_mut().ok_or("missing controls")?;
        controls.update(
            host.queue(),
            Vec2::new(logical.width, logical.height),
            self.paused,
            !self.disable_temporal,
            perfect_planar,
        )?;
        let scene = self.scene.as_mut().ok_or("missing scene")?;
        scene.replace_scene_mesh(&emitter(offset)?)?;
        let camera =
            glam::camera::rh::proj::directx::perspective(
                1.0,
                size.width as f32 / size.height as f32,
                0.1,
                10.0,
            ) * glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO, Vec3::Y);
        let frame = RasterRayFrame::with_ggx_pipeline(
            &device,
            host.adapter(),
            scene,
            RasterRayOptions {
                dimensions: self.size,
                view_projection: camera,
                clear_depth: 1.0,
                light: SurfacePointLight {
                    position: [0.0, 0.0, 2.0],
                    intensity: [8.0; 3],
                    bias: 0.001,
                },
                reflection: SurfaceReflectionOptions {
                    material: ReconstructionMaterial::new([0.0; 3], 1.0, 0.0)?,
                    camera: [0.0, 0.0, 3.0],
                    bias: 0.001,
                    maximum_distance: 10.0,
                },
            },
            &[[20.0, 5.0, 1.0, 1.0]],
            self.frames,
            self.lighting.as_ref().ok_or("missing lighting")?,
        )?;
        if self.mirror_geometry.is_none() {
            self.mirror_geometry = Some(ReconstructionGuideMesh::from_scene_mesh(
                &device,
                &mirror(self.curved)?,
                Mat4::IDENTITY,
                0.8,
                self.roughness,
            )?);
        }
        let geometry = self
            .mirror_geometry
            .as_ref()
            .ok_or("missing mirror geometry")?;
        let material = frame.material_inputs(self.texture.as_ref().ok_or("missing texture")?)?;
        let previous = if let Some(offset) = self.previous_emitter {
            let mesh = emitter(offset)?;
            vec![PreviousReflectionTriangle {
                identity: [0; 4],
                vertices: std::array::from_fn(|i| {
                    let p = mesh.vertices()[i].position;
                    [p[0], p[1], p[2], 1.0]
                }),
            }]
        } else {
            Vec::new()
        };
        let mut filtered = self
            .temporal
            .as_ref()
            .ok_or("missing temporal pipeline")?
            .prepare(
                frame.reflection_job(),
                &previous,
                PlanarReflectionCameras {
                    cameras: [camera, self.previous_camera.unwrap_or(camera)],
                    planes: [[0.0, 0.0, 1.0, -0.5]; 2],
                },
                self.history.as_mut().ok_or("missing history")?,
                TemporalResolveOptions {
                    history_weight: if self.disable_temporal { 0.0 } else { 0.9 },
                    depth_tolerance: 0.001,
                    reset_history: self.previous_emitter.is_none(),
                },
                true,
            )?;
        let spatial = self
            .spatial
            .as_ref()
            .ok_or("missing spatial filter")?
            .prepare(
                frame.primary(),
                frame.reflection_job(),
                ReflectionSpatialOptions::default(),
            )?;
        let composed = RadianceComposition::new_hdr(
            &device,
            frame.direct_radiance(),
            if perfect_planar {
                filtered.output()
            } else {
                spatial.output()
            },
        )?;
        let color = composed.output().create_view(&Default::default());
        let blit = self.blit.as_ref().ok_or("missing tone mapper")?;
        let depth = frame.depth().create_view(&Default::default());
        let encode = |encoder: &mut wgpu::CommandEncoder,
                      target: &wgpu::TextureView|
         -> Result<(), Box<dyn std::error::Error>> {
            scene.build(encoder);
            frame.encode(encoder, &[(geometry, &material)])?;
            if perfect_planar {
                filtered.encode(encoder)?;
            } else {
                spatial.encode(encoder);
            }
            composed.encode(encoder);
            blit.encode_checked(&device, encoder, &color, target)?;
            controls.encode(encoder, target, &depth);
            Ok(())
        };
        let outcome = if self.offscreen_check {
            check_offscreen(
                &device,
                host.queue(),
                host.color_format(),
                self.size,
                composed.output(),
                [(30.0 * window.scale_factor()) as u32; 2],
                if self.checked_frames == 2 {
                    self.capture.as_deref()
                } else {
                    None
                },
                encode,
            )?;
            self.checked_frames += 1;
            RenderOutcome::SkippedOccluded
        } else {
            host.render_custom::<_, Box<dyn std::error::Error>>(encode)?
        };
        // Perfect-planar history must never accept sampled rough reflection output.
        filtered.finish(if perfect_planar {
            outcome
        } else {
            RenderOutcome::SkippedOccluded
        })?;
        let committed = outcome == RenderOutcome::Presented;
        self.ui
            .as_mut()
            .ok_or("missing UI")?
            .set_presented_regions(&controls.regions, committed)?;
        if self.offscreen_check {
            if self.history.as_ref().ok_or("missing history")?.valid()
                || self.previous_emitter.is_some()
            {
                return Err("offscreen work committed presentation history".into());
            }
            if self.resize_check && self.checked_frames == 1 {
                host.resize(0, 0)?;
                let suspended = host.render_custom::<_, Box<dyn std::error::Error>>(|_, _| {
                    Err("suspended host called GPU encoder".into())
                })?;
                if suspended != RenderOutcome::Suspended {
                    return Err("zero-size suspension did not skip acquisition".into());
                }
                host.resize(size.width, size.height)?;
                self.resize_origin = Some([size.width, size.height]);
                let reported = window.request_inner_size(winit::dpi::PhysicalSize::new(643, 437));
                eprintln!(
                    "PLANAR SIZE REQUEST: original {:?}, immediate {:?}",
                    self.resize_origin, reported
                );
            }
            if self.recovery_check && self.checked_frames == 1 && self.recoveries == 0 {
                host.device().destroy();
                let _ = host.device().poll(wgpu::PollType::Poll);
            }
            if self.checked_frames == 3 {
                if self.resize_check {
                    if Some(self.size) == self.resize_origin {
                        return Err("resize did not recreate target dimensions".into());
                    }
                    println!(
                        "PLANAR RESIZE PASS: actual {}x{} GPU graph/UI readback, zero-size host suspension skipped callback; native minimization/display not measured",
                        self.size[0], self.size[1]
                    );
                }
                if self.recovery_check && self.recoveries != 1 {
                    return Err("device recreation did not execute exactly once".into());
                }
                if self.recovery_check {
                    println!(
                        "PLANAR GPU RECOVERY PASS: destroyed device, rebuilt resources on same window, invalidated history"
                    );
                }
                println!(
                    "PLANAR SCENE OFFSCREEN PASS: raster/ray/temporal/HDR/tone-map/UI; no presentation or history commit"
                );
                event_loop.exit();
            } else {
                window.request_redraw();
            }
            return Ok(());
        }
        if committed {
            self.previous_emitter = Some(offset);
            self.previous_camera = Some(camera);
            self.frames += 1;
            window.set_title(&format!(
                "Voxy mirror | {} frames | controls: pause / reset / temporal | Space / R / T / G roughness / C curved",
                self.frames
            ));
            if self.smoke && self.frames >= 120 {
                println!(
                    "PLANAR SCENE PRESENT PASS: 120 native frames; scene pose committed only on host presentation; planar history used only in compatible mode; image quality not measured"
                );
                event_loop.exit();
                return Ok(());
            }
            self.retry = None;
            event_loop.set_control_flow(ControlFlow::Wait);
            window.request_redraw();
        } else {
            self.retry = Some(Instant::now() + Duration::from_millis(100));
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.retry.unwrap()));
        }
        Ok(())
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.suspended = false;
        self.occluded = false;
        event_loop.set_control_flow(ControlFlow::Wait);
        let result = if self.host.is_none() {
            self.init(event_loop)
        } else {
            self.reset();
            if let Some(w) = &self.window {
                w.request_redraw();
            }
            Ok(())
        };
        if let Err(error) = result {
            self.failure = Some(error.to_string());
            event_loop.exit();
        }
    }
    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.suspended = true;
        self.retry = None;
        self.reset();
        self.size = [0; 2];
        if let Some(host) = &mut self.host {
            if let Err(error) = host.resize(0, 0) {
                self.failure = Some(error.to_string());
                event_loop.exit();
            }
        }
        event_loop.set_control_flow(ControlFlow::Wait);
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.window.as_ref().is_none_or(|w| w.id() != id) {
            return;
        }
        let dispatch = self
            .ui
            .as_mut()
            .map(|ui| ui.event(&event))
            .unwrap_or_default();
        let activated = match dispatch.pointer {
            voxy_ui::PointerAction::Release { id, clicked: true } => Some(id),
            _ => match dispatch.keyboard {
                voxy_ui::KeyAction::Release { id, clicked: true } => Some(id),
                _ => None,
            },
        };
        if let Some(id) = activated {
            match id.0 {
                1 => {
                    self.paused = !self.paused;
                    self.clock.set_paused(self.paused);
                    self.tick = Some(Instant::now());
                }
                2 => self.reset(),
                3 => {
                    self.disable_temporal = !self.disable_temporal;
                    self.reset();
                }
                _ => {}
            }
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.draw(event_loop) {
                    let recoverable = matches!(
                        error.downcast_ref::<RendererError>(),
                        Some(RendererError::SurfaceLost | RendererError::DeviceLost(_))
                    );
                    if recoverable && self.recoveries < 3 {
                        self.recoveries += 1;
                        self.reset();
                        match self.init(event_loop) {
                            Ok(()) => eprintln!("PLANAR GPU RECREATED: {}", self.recoveries),
                            Err(error) => {
                                self.failure = Some(error.to_string());
                                event_loop.exit();
                            }
                        }
                    } else {
                        self.failure = Some(error.to_string());
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                self.reset();
                if occluded {
                    self.retry = Some(Instant::now() + Duration::from_millis(100));
                    event_loop.set_control_flow(ControlFlow::WaitUntil(self.retry.unwrap()));
                } else if let Some(w) = &self.window {
                    self.retry = None;
                    event_loop.set_control_flow(ControlFlow::Wait);
                    w.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } | WindowEvent::Resized(_) => {
                self.reset();
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),
                    PhysicalKey::Code(KeyCode::Space) if !dispatch.consumed => {
                        self.paused = !self.paused;
                        self.clock.set_paused(self.paused);
                        self.tick = Some(Instant::now());
                    }
                    PhysicalKey::Code(KeyCode::KeyR) => self.reset(),
                    PhysicalKey::Code(KeyCode::KeyG) => {
                        self.roughness = if self.roughness < 0.1 {
                            0.2
                        } else if self.roughness < 0.4 {
                            0.5
                        } else if self.roughness < 0.9 {
                            1.0
                        } else {
                            0.0
                        };
                        self.mirror_geometry = None;
                        self.reset();
                    }
                    PhysicalKey::Code(KeyCode::KeyC) => {
                        self.curved = !self.curved;
                        self.mirror_geometry = None;
                        self.reset();
                    }
                    PhysicalKey::Code(KeyCode::KeyT) => {
                        self.disable_temporal = !self.disable_temporal;
                        self.reset();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if (self.smoke || self.offscreen_check) && self.deadline.is_some_and(|t| now >= t) {
            self.failure = Some(format!(
                "planar check timed out after {} presentations",
                self.frames
            ));
            event_loop.exit();
            return;
        }
        if self.retry.is_some_and(|t| now >= t) {
            self.retry = None;
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        let wake = if self.smoke || self.offscreen_check {
            match (self.retry, self.deadline) {
                (Some(retry), Some(deadline)) => Some(retry.min(deadline)),
                (retry, deadline) => retry.or(deadline),
            }
        } else {
            self.retry
        };
        event_loop.set_control_flow(wake.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::default();
    let mut opt_in = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--experimental" => opt_in = true,
            "--smoke" => app.smoke = true,
            "--always-on-top" => app.always_on_top = true,
            "--curved" => app.curved = true,
            "--recovery-check" => app.recovery_check = true,
            "--resize-check" => app.resize_check = true,
            "--capture" => app.capture = Some(args.next().ok_or("missing capture path")?.into()),
            "--offscreen-check" => app.offscreen_check = true,
            "--roughness" => {
                app.roughness = args.next().ok_or("missing roughness")?.parse()?;
                if !app.roughness.is_finite() || !(0.0..=1.0).contains(&app.roughness) {
                    return Err("roughness must be finite in 0..1".into());
                }
            }
            "--backend" => {
                app.backend = match args.next().as_deref() {
                    Some("metal") => GraphicsBackend::Metal,
                    Some("dx12") => GraphicsBackend::DirectX12,
                    Some("vulkan") => GraphicsBackend::Vulkan,
                    _ => return Err("expected --backend metal|dx12|vulkan".into()),
                }
            }
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    if (app.recovery_check || app.resize_check || app.capture.is_some()) && !app.offscreen_check {
        return Err("--recovery-check/--resize-check require --offscreen-check".into());
    }
    if app.smoke && app.offscreen_check {
        return Err("choose --smoke or --offscreen-check".into());
    }
    if !opt_in {
        return Err("planar_scene requires --experimental".into());
    }
    EventLoop::new()?.run_app(&mut app)?;
    if let Some(error) = app.failure {
        return Err(error.into());
    }
    if app.smoke && app.frames < 120 {
        return Err("smoke exited before 120 presentations".into());
    }
    Ok(())
}
