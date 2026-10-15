//! Native street walk: original procedural scene by default, imported Bistro via --bistro.
#[path = "../../src/controls.rs"]
mod controls;
mod materials;
mod original;
mod scene;
use controls::{InputState, WalkCamera};
use glam::{DVec3, Vec2, Vec3};
use std::{path::PathBuf, sync::Arc, time::Instant};
use voxy_render::archive_scene::ArchiveSceneRenderer;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};
struct Gpu {
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    hdr_last_check: Instant,
    hdr_headroom: Option<f32>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: ArchiveSceneRenderer,
    offscreen: Option<wgpu::Texture>,
}
struct App {
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    collision: Option<physics::static_triangle_world::StaticTriangleWorld>,
    input: InputState,
    camera: WalkCamera,
    feet: DVec3,
    previous: DVec3,
    velocity_y: f64,
    grounded: bool,
    captured: bool,
    last: Instant,
    accumulator: f64,
    frames: u64,
    measure_start: Option<Instant>,
    durations: Vec<f64>,
    physics_seconds: Vec<f64>,
    render_seconds: Vec<f64>,
    acquire_seconds: Vec<f64>,
    snapshot: Option<PathBuf>,
    benchmark: f64,
    offscreen_benchmark: bool,
    failure: Option<String>,
    finished: bool,
    root: PathBuf,
    original: bool,
    original_seed: u64,
    weather: voxy_render::weather::Weather,
    weather_start: Instant,
    sun_elevation: Option<f32>,
}
impl App {
    fn new() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let value = |name: &str| {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        let mut app = Self {
            weather: match value("--weather").as_deref() {
                Some("cloudy") => voxy_render::weather::Weather::CLOUDY,
                Some("rain") => voxy_render::weather::Weather::STORM,
                Some("snow") => voxy_render::weather::Weather::SNOW,
                _ => voxy_render::weather::Weather::CLEAR,
            },
            weather_start: Instant::now(),
            sun_elevation: value("--sun-elevation").and_then(|v| v.parse().ok()),
            window: None,
            gpu: None,
            collision: None,
            input: InputState::default(),
            camera: WalkCamera::new(Vec3::NEG_Z),
            feet: DVec3::ZERO,
            previous: DVec3::ZERO,
            velocity_y: 0.,
            grounded: false,
            captured: false,
            last: Instant::now(),
            accumulator: 0.,
            frames: 0,
            measure_start: None,
            durations: Vec::new(),
            physics_seconds: Vec::new(),
            render_seconds: Vec::new(),
            acquire_seconds: Vec::new(),
            snapshot: value("--snapshot").map(PathBuf::from),
            offscreen_benchmark: args.iter().any(|a| a == "--offscreen-benchmark"),
            benchmark: value("--benchmark")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.),
            failure: None,
            finished: false,
            original: !args.iter().any(|a| a == "--bistro"),
            original_seed: value("--seed")
                .and_then(|v| {
                    if let Some(v) = v.strip_prefix("0x") {
                        u64::from_str_radix(v, 16).ok()
                    } else {
                        v.parse().ok()
                    }
                })
                .unwrap_or(0x564f5859),
            root: value("--assets").map(PathBuf::from).unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../assets/environments/bistro/cache")
            }),
        };
        if value("--wind-speed").is_some() || value("--wind-direction").is_some() {
            let [x, z] = app.weather.wind;
            let speed = value("--wind-speed")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(x.hypot(z));
            let angle = value("--wind-direction")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(z.atan2(x).to_degrees())
                .to_radians();
            if speed.is_finite() && angle.is_finite() {
                app.weather.wind = [
                    angle.cos() * speed.clamp(0., 100.),
                    angle.sin() * speed.clamp(0., 100.),
                ];
            }
        }
        if let Some(gust) = value("--wind-gust").and_then(|v| v.parse::<f32>().ok()) {
            if gust.is_finite() {
                app.weather.wind_gust = gust.clamp(0., 1.);
            }
        }
        app
    }
    fn init(&mut self, el: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        let window = Arc::new(
            el.create_window(
                Window::default_attributes()
                    .with_title(if self.original {
                        "Voxy — Original Street: WASD · Shift · Space · мышь · Esc"
                    } else {
                        "Voxy — Bistro: WASD · Shift · Space · мышь · Esc"
                    })
                    .with_inner_size(winit::dpi::PhysicalSize::new(1920, 1080))
                    .with_resizable(false),
            )?,
        );
        window.request_redraw();
        self.window = Some(window.clone());
        if !self.original {
            eprintln!("STREET loading full authored scene {}", self.root.display());
        }
        let mut scene = if self.original {
            original::generate(self.original_seed)?
        } else {
            scene::load(&self.root)?
        };
        let environment = if self.original {
            Some(materials::sky()?)
        } else {
            None
        };
        if let Some(panorama) = &environment {
            scene.sun = materials::sun_direction(panorama);
        }
        self.camera = WalkCamera::new(scene.direction);
        self.feet = scene.eye.as_dvec3() - DVec3::Y * 1.65;
        let initial = self.feet;
        let planar = Vec3::new(scene.direction.x, 0., scene.direction.z)
            .normalize()
            .as_dvec3();
        let mut spawn = None;
        for distance in [0., 1., 2., 3., 4., 5., -1., -2.] {
            let candidate = initial + planar * distance;
            if let Ok(recovered) =
                scene
                    .collision
                    .recover_capsule(candidate.to_array(), 1.8, 0.3, planar.to_array())
            {
                let (ground, grounded) =
                    scene
                        .collision
                        .move_capsule(recovered, 1.8, 0.3, [0., -20., 0.])?;
                if grounded {
                    spawn = Some(DVec3::from_array(ground));
                    break;
                }
            }
        }
        self.feet = spawn.ok_or("no nonintersecting grounded street spawn")?;
        self.grounded = true;
        eprintln!(
            "STREET grounded spawn: authored={initial:?} resolved={:?}",
            self.feet
        );
        self.previous = self.feet;
        self.collision = Some(scene.collision);
        let instance = voxy_render::GraphicsOptions {
            backend: voxy_render::GraphicsBackend::Metal,
            ..Default::default()
        }
        .create_instance();
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))?;
        let features = wgpu::Features::TEXTURE_COMPRESSION_BC
            | if self.original {
                wgpu::Features::FLOAT32_FILTERABLE
            } else {
                wgpu::Features::empty()
            }
            | if std::env::var_os("VOXY_STREET_GPU_TIMING").is_some() {
                adapter.features() & wgpu::Features::TIMESTAMP_QUERY
            } else {
                wgpu::Features::empty()
            };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("Voxy full street"),
                required_features: features,
                required_limits: wgpu::Limits {
                    max_texture_dimension_2d: 16384,
                    max_storage_buffer_binding_size:
                        adapter.limits().max_storage_buffer_binding_size,
                    ..wgpu::Limits::default()
                },
                ..Default::default()
            }))?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or("surface unavailable")?;
        let caps = surface.get_capabilities(&adapter);
        let hdr_supported = caps
            .color_spaces(wgpu::TextureFormat::Rgba16Float)
            .contains(wgpu::SurfaceColorSpaces::EXTENDED_SRGB_LINEAR);
        let wants_hdr = !std::env::args().any(|a| a == "--sdr");
        if wants_hdr && hdr_supported {
            config.format = wgpu::TextureFormat::Rgba16Float;
            config.color_space = wgpu::SurfaceColorSpace::ExtendedSrgbLinear;
        } else {
            config.format = caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .ok_or("sRGB surface unavailable")?;
            config.color_space = wgpu::SurfaceColorSpace::Srgb;
            if wants_hdr {
                eprintln!(
                    "STREET HDR not advertised for float16/linear surface; explicit SDR fallback"
                );
            }
        }
        let hdr_info = surface.display_hdr_info(&adapter);
        let hdr_headroom = hdr_info.tone_map_headroom();
        eprintln!(
            "STREET DISPLAY format={:?} colour_space={:?} hdr_info={hdr_info:?}",
            config.format, config.color_space
        );
        config.present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
            wgpu::PresentMode::Immediate
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        config.desired_maximum_frame_latency = 2;
        if self.snapshot.is_some() {
            config.usage |= wgpu::TextureUsages::COPY_SRC;
        }
        eprintln!(
            "STREET GPU {:?}; {}x{} {:?}; full scene geometry; original mip chains when images are present",
            adapter.get_info(),
            size.width,
            size.height,
            config.present_mode
        );
        surface.configure(&device, &config);
        let mut renderer = ArchiveSceneRenderer::new_with_environment(
            &device,
            &queue,
            config.format,
            size.width,
            size.height,
            scene.images,
            scene.materials,
            scene.geometry,
            &scene.lights,
            scene.sun,
            scene.sun_colour,
            scene.bounds,
            environment,
        )?;
        if let Some(degrees) = self.sun_elevation {
            let angle = degrees.clamp(-10., 90.).to_radians();
            renderer.set_sun(
                Vec3::new(-angle.cos(), angle.sin(), 0.3 * angle.cos()),
                [1., 0.95, 0.85, 100000.],
            )?;
        }
        renderer.set_display_headroom(hdr_headroom.unwrap_or(1.).max(1.))?;
        let offscreen = if self.offscreen_benchmark {
            Some(device.create_texture(&wgpu::TextureDescriptor {
                label: Some("full resolution completed-frame benchmark"),
                size: wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            }))
        } else {
            None
        };
        self.gpu = Some(Gpu {
            surface,
            adapter,
            hdr_last_check: Instant::now(),
            hdr_headroom,
            device,
            queue,
            config,
            renderer,
            offscreen,
        });
        self.last = Instant::now();
        window.focus_window();
        self.capture(false);
        Ok(())
    }
    fn capture(&mut self, captured: bool) {
        if let Some(w) = &self.window {
            if captured {
                let grabbed = w
                    .set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| w.set_cursor_grab(CursorGrabMode::Confined))
                    .is_ok();
                self.captured = grabbed;
                w.set_cursor_visible(!grabbed);
            } else {
                let _ = w.set_cursor_grab(CursorGrabMode::None);
                w.set_cursor_visible(true);
                self.captured = false;
                self.input.cancel_all();
            }
        }
    }
    fn draw(&mut self, el: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        if self.finished {
            return Ok(());
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        self.camera.apply_look(self.input.take_look());
        self.camera.step(elapsed as f32);
        self.accumulator += elapsed.min(0.25);
        const DT: f64 = 1. / 120.;
        let collision = self.collision.as_ref().ok_or("missing collision")?;
        while self.accumulator >= DT {
            let mut movement = self.input.take_move();
            if self.benchmark > 0. {
                movement.local = Vec2::new(0., if self.frames % 2400 < 1200 { 1. } else { -1. });
            }
            let (forward, right) = self.camera.planar_basis();
            let planar = right * movement.local.x + forward * movement.local.y;
            let speed = if movement.sprint { 5.5 } else { 2.2 };
            if movement.jump_pressed && self.grounded {
                self.velocity_y = 5.5;
                self.grounded = false;
            }
            self.velocity_y = (self.velocity_y - 9.81 * DT).max(-48.);
            self.previous = self.feet;
            let delta = [
                f64::from(planar.x) * speed * DT,
                self.velocity_y * DT,
                f64::from(planar.z) * speed * DT,
            ];
            let (p, grounded) = collision.move_capsule(self.feet.to_array(), 1.8, 0.3, delta)?;
            self.feet = DVec3::from_array(p);
            self.grounded = grounded;
            if grounded && self.velocity_y < 0. {
                self.velocity_y = 0.;
            }
            if self.feet.y - self.previous.y < delta[1] * 0.5 && self.velocity_y > 0. {
                self.velocity_y = 0.;
            }
            self.accumulator -= DT;
        }
        let actor = self
            .previous
            .lerp(self.feet, self.accumulator / DT)
            .as_vec3();
        let target = actor + Vec3::Y * 1.65;
        let direction = self.camera.direction();
        let mut eye = target - direction * self.camera.distance;
        if self.camera.distance > 0. {
            if let Some(hit) = collision.sweep_capsule(
                target.as_dvec3().to_array(),
                target.as_dvec3().to_array(),
                0.12,
                (eye - target).as_dvec3().to_array(),
            )? {
                eye = target + (eye - target) * hit.fraction as f32;
            }
        }
        let physics_time = now.elapsed().as_secs_f64();
        let acquire_start = Instant::now();
        let gpu = self.gpu.as_mut().ok_or("missing renderer")?;
        if gpu.config.color_space.is_hdr() && gpu.hdr_last_check.elapsed().as_secs_f32() >= 1. {
            let info = gpu.surface.display_hdr_info(&gpu.adapter);
            let headroom = info.tone_map_headroom();
            if headroom != gpu.hdr_headroom {
                eprintln!("STREET live HDR headroom={headroom:?} info={info:?}");
            }
            gpu.hdr_headroom = headroom;
            gpu.renderer
                .set_display_headroom(headroom.unwrap_or(1.).max(1.))?;
            gpu.hdr_last_check = Instant::now();
        }
        let frame = if gpu.offscreen.is_some() {
            None
        } else {
            Some(match gpu.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(f)
                | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
                wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                    gpu.surface.configure(&gpu.device, &gpu.config);
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    return Err("surface validation failed".into());
                }
            })
        };
        let acquire_time = acquire_start.elapsed().as_secs_f64();
        let render_start = Instant::now();
        let output = frame
            .as_ref()
            .map_or_else(|| gpu.offscreen.as_ref().unwrap(), |f| &f.texture);
        gpu.renderer.set_weather(self.weather)?;
        gpu.renderer
            .set_weather_time(self.weather_start.elapsed().as_secs_f32());
        let submission =
            gpu.renderer
                .draw(&output.create_view(&Default::default()), eye, direction);
        if self.offscreen_benchmark {
            gpu.device.poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })?;
        }
        let render_time = render_start.elapsed().as_secs_f64();
        self.frames += 1;
        let capture_now = self.frames >= 120 && self.snapshot.is_some();
        if capture_now {
            if let Some(path) = self.snapshot.take() {
                snapshot(gpu, output, &path)?;
                eprintln!("STREET snapshot {}", path.display());
            }
        }
        if let Some(frame) = frame {
            gpu.queue.present(frame);
        }
        if capture_now && gpu.config.usage.contains(wgpu::TextureUsages::COPY_SRC) {
            gpu.config.usage.remove(wgpu::TextureUsages::COPY_SRC);
            gpu.surface.configure(&gpu.device, &gpu.config);
        }
        let _ = gpu.device.poll(wgpu::PollType::Poll);
        if self.frames == 180 {
            self.measure_start = Some(Instant::now());
            self.durations.clear();
        } else if self.frames > 180 {
            self.durations.push(elapsed);
            self.physics_seconds.push(physics_time);
            self.render_seconds.push(render_time);
            self.acquire_seconds.push(acquire_time);
        }
        if self.frames % 240 == 0 {
            let seconds = self.durations.iter().rev().take(240).sum::<f64>();
            let count = self.durations.len().min(240);
            if seconds > 0. {
                self.window.as_ref().unwrap().set_title(&format!(
                    "Voxy {} — {} · {:.1} FPS · 1920×1080 · WASD · Shift · Space · V · Esc",
                    if self.original {
                        "Original Street"
                    } else {
                        "Bistro"
                    },
                    if gpu.config.color_space.is_hdr() {
                        format!("HDR/EDR {:.2}×", gpu.hdr_headroom.unwrap_or(1.))
                    } else {
                        "SDR".into()
                    },
                    count as f64 / seconds
                ));
                eprintln!(
                    "STREET frames={} fps={:.2} feet={:?} grounded={}",
                    self.frames,
                    count as f64 / seconds,
                    self.feet,
                    self.grounded
                );
            }
        }
        if self.benchmark > 0.
            && self
                .measure_start
                .is_some_and(|t| t.elapsed().as_secs_f64() >= self.benchmark)
        {
            let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
            let mut times = self.durations.clone();
            times.sort_by(f64::total_cmp);
            let n = times.len();
            let report = serde_json::json!({"scene":if self.original{"Voxy Original Street"}else{"Bistro"},"measurement":if self.offscreen_benchmark{"completed_full_resolution_offscreen_frames"}else{"presented_window_frames"},"resolution":[gpu.config.width,gpu.config.height],"present_mode":format!("{:?}",gpu.config.present_mode),"source_simplified":false,"frames":n,"seconds":self.measure_start.unwrap().elapsed().as_secs_f64(),"mean_fps":n as f64/self.measure_start.unwrap().elapsed().as_secs_f64(),"p50_ms":times[n/2]*1000.,"p95_ms":times[n*95/100]*1000.,"p99_ms":times[n*99/100]*1000.,"full_collision_triangles":collision.triangle_count(),"feet":self.feet.to_array(),"grounded":self.grounded,"cpu_physics_mean_ms":self.physics_seconds.iter().sum::<f64>()/n as f64*1000.,"cpu_render_mean_ms":self.render_seconds.iter().sum::<f64>()/n as f64*1000.,"surface_acquire_mean_ms":self.acquire_seconds.iter().sum::<f64>()/n as f64*1000.,"gpu_samples":gpu.renderer.gpu_times_ms.len(),"gpu_mean_ms":if gpu.renderer.gpu_times_ms.is_empty(){None}else{Some(gpu.renderer.gpu_times_ms.iter().sum::<f64>()/gpu.renderer.gpu_times_ms.len() as f64)}});
            std::fs::write(
                "/tmp/voxy-street-benchmark.json",
                serde_json::to_vec_pretty(&report)?,
            )?;
            eprintln!("STREET BENCHMARK {report}");
            self.finished = true;
            el.exit();
        }
        Ok(())
    }
}
fn snapshot(
    gpu: &Gpu,
    texture: &wgpu::Texture,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let w = gpu.config.width;
    let h = gpu.config.height;
    let hdr = gpu.config.format == wgpu::TextureFormat::Rgba16Float;
    let bytes_per_pixel = if hdr { 8 } else { 4 };
    let stride = (w * bytes_per_pixel).div_ceil(256) * 256;
    let read = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("street snapshot"),
        size: u64::from(stride) * u64::from(h),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &read,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);
    read.slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.unwrap());
    gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
    let bytes = read.slice(..).get_mapped_range()?;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    let mut linear_rgb = Vec::<[f32; 3]>::new();
    for row in bytes.chunks_exact(stride as usize) {
        for p in row[..(w * bytes_per_pixel) as usize].chunks_exact(bytes_per_pixel as usize) {
            if hdr {
                let rgb = std::array::from_fn(|c| {
                    decode_half(u16::from_le_bytes([p[c * 2], p[c * 2 + 1]]))
                });
                if rgb.iter().any(|v| !v.is_finite()) {
                    return Err("nonfinite HDR output pixel".into());
                }
                linear_rgb.push(rgb);
                for x in rgb {
                    // This PNG is explicitly an SDR preview; the PFM below preserves every HDR value.
                    let x = x.max(0.);
                    let x =
                        ((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0., 1.);
                    let encoded = if x <= 0.0031308 {
                        12.92 * x
                    } else {
                        1.055 * x.powf(1. / 2.4) - 0.055
                    };
                    rgba.push((encoded * 255.).round().clamp(0., 255.) as u8);
                }
                rgba.push(255);
            } else if matches!(
                gpu.config.format,
                wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Bgra8Unorm
            ) {
                rgba.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
            } else {
                rgba.extend_from_slice(p);
            }
        }
    }
    if hdr {
        use std::io::Write;
        let raw_path = path.with_extension("pfm");
        let mut raw = std::io::BufWriter::new(std::fs::File::create(&raw_path)?);
        write!(raw, "PF\n{w} {h}\n-1.0\n")?;
        for row in linear_rgb.chunks_exact(w as usize).rev() {
            for rgb in row {
                for value in rgb {
                    raw.write_all(&value.to_le_bytes())?;
                }
            }
        }
        raw.flush()?;
        let peak = linear_rgb.iter().flatten().copied().fold(0., f32::max);
        let above = linear_rgb
            .iter()
            .filter(|p| p.iter().any(|v| *v > 1.))
            .count();
        let report = serde_json::json!({"surface_format":format!("{:?}",gpu.config.format),"colour_space":format!("{:?}",gpu.config.color_space),"live_headroom":gpu.hdr_headroom,"peak_relative_sdr_white":peak,"pixels_above_sdr_white":above,"pixels":w as usize*h as usize,"finite":true,"native_hdr_capture":raw_path,"png_is_sdr_preview":true});
        std::fs::write(
            path.with_extension("hdr.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        eprintln!("STREET HDR PIXELS {report}");
    }
    image::save_buffer(path, &rgba, w, h, image::ColorType::Rgba8)?;
    Ok(())
}
fn decode_half(bits: u16) -> f32 {
    let sign = u32::from(bits & 0x8000) << 16;
    let exponent = u32::from((bits >> 10) & 31);
    let mantissa = u32::from(bits & 1023);
    match exponent {
        0 => {
            let value = mantissa as f32 * (1. / 16777216.);
            if sign == 0 { value } else { -value }
        }
        31 => f32::from_bits(sign | 0x7f800000 | (mantissa << 13)),
        _ => f32::from_bits(sign | ((exponent + 112) << 23) | (mantissa << 13)),
    }
}
#[cfg(test)]
mod hdr_snapshot_tests {
    use super::*;
    #[test]
    fn half_readback_preserves_reference_white_and_hdr_highlights() {
        assert_eq!(decode_half(0x3c00), 1.);
        assert_eq!(decode_half(0x4000), 2.);
        assert_eq!(decode_half(0x7bff), 65504.);
        assert_eq!(decode_half(1), 2_f32.powi(-24));
        assert_eq!(decode_half(0xbc00), -1.);
        assert!(decode_half(0x7c00).is_infinite());
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_none() {
            if let Err(e) = self.init(el) {
                self.failure = Some(e.to_string());
                el.exit();
            }
        }
    }
    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Focused(false) => self.capture(false),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.capture(true),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    if key == KeyCode::Escape && event.state == ElementState::Pressed {
                        self.capture(false);
                    } else if event.state == ElementState::Pressed
                        && !event.repeat
                        && matches!(
                            key,
                            KeyCode::Digit1 | KeyCode::Digit2 | KeyCode::Digit3 | KeyCode::Digit4
                        )
                    {
                        let wind = self.weather.wind;
                        let gust = self.weather.wind_gust;
                        self.weather = match key {
                            KeyCode::Digit2 => voxy_render::weather::Weather::CLOUDY,
                            KeyCode::Digit3 => voxy_render::weather::Weather::STORM,
                            KeyCode::Digit4 => voxy_render::weather::Weather::SNOW,
                            _ => voxy_render::weather::Weather::CLEAR,
                        };
                        self.weather.wind = wind;
                        self.weather.wind_gust = gust;
                    } else if event.state == ElementState::Pressed
                        && !event.repeat
                        && matches!(key, KeyCode::F5 | KeyCode::F6 | KeyCode::F7 | KeyCode::F8)
                    {
                        let [x, z] = self.weather.wind;
                        let speed = x.hypot(z);
                        let angle = z.atan2(x)
                            + if key == KeyCode::F6 {
                                std::f32::consts::FRAC_PI_4
                            } else {
                                0.
                            };
                        let speed = match key {
                            KeyCode::F5 => {
                                if speed > 0. {
                                    0.
                                } else {
                                    8.
                                }
                            }
                            KeyCode::F7 => (speed - 2.).max(0.),
                            KeyCode::F8 => (speed + 2.).min(100.),
                            _ => speed,
                        };
                        self.weather.wind = [angle.cos() * speed, angle.sin() * speed];
                    } else if key == KeyCode::KeyV
                        && event.state == ElementState::Pressed
                        && !event.repeat
                    {
                        self.camera
                            .set_distance(if self.camera.desired_distance() == 0. {
                                3.
                            } else {
                                0.
                            });
                    } else {
                        self.input.key(key, event.state, event.repeat);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.,
                };
                self.camera.zoom(lines);
            }
            WindowEvent::RedrawRequested => {}
            _ => {}
        }
    }
    fn device_event(&mut self, _: &ActiveEventLoop, _: winit::event::DeviceId, event: DeviceEvent) {
        if self.captured {
            if let DeviceEvent::MouseMotion { delta } = event {
                self.input.mouse_motion(delta);
            }
        }
    }
    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if self.gpu.is_some() && !self.finished {
            if let Err(e) = self.draw(el) {
                self.failure = Some(e.to_string());
                self.finished = true;
                el.exit();
            }
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let el = EventLoop::new()?;
    el.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    el.run_app(&mut app)?;
    if let Some(e) = app.failure {
        return Err(e.into());
    }
    Ok(())
}
