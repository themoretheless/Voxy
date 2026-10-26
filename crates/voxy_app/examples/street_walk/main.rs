//! Native street walk: original procedural scene by default, imported Bistro via --bistro.
mod benchmark_cycle;
mod aa_validation;
#[path = "../../src/controls.rs"]
mod controls;
mod day_cycle;
#[path = "../../src/frame_profile.rs"]
mod frame_profile;
mod materials;
mod original;
mod panel;
mod ray_geometry_export;
mod scene;
mod world_ray_lighting;
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
    validation_headroom: Option<f32>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: ArchiveSceneRenderer,
    supersample: voxy_render::SupersampleResolve,
    offscreen: Option<wgpu::Texture>,
    /// Scene image when the settings panel takes a strip on the left; the
    /// frame copies it in at `panel_px`.
    scene_target: Option<wgpu::Texture>,
    panel_px: u32,
    bodycam: Option<voxy_render::BodycamPostProcess>,
}
/// Camera cycled with V: eye, orbit, chest bodycam, hand-held camcorder.
#[derive(Clone, Copy, PartialEq, Eq)]
enum StreetView {
    FirstPerson,
    ThirdPerson,
    Bodycam,
    Camcorder,
}
struct App {
    profile: Option<frame_profile::Profile>,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    probe_bake: Option<voxy_render::archive_scene::ArchiveProbeBake>,
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
    simulation_time: f64,
    simulation_steps: u64,
    measure_simulation_steps: u64,
    measure_water_start: Option<serde_json::Value>,
    frames: u64,
    measure_start: Option<Instant>,
    measure_start_unix_seconds: Option<f64>,
    replay_state: blake3::Hasher,
    durations: Vec<f64>,
    skipped_acquires: u64,
    physics_seconds: Vec<f64>,
    render_seconds: Vec<f64>,
    acquire_seconds: Vec<f64>,
    snapshot: Option<PathBuf>,
    benchmark: f64,
    benchmark_complete: bool,
    world_ray_audit_frames: u64,
    world_ray_audit_complete: bool,
    offscreen_benchmark: bool,
    render_every_physics_step: bool,
    failure: Option<String>,
    finished: bool,
    root: PathBuf,
    original: bool,
    original_seed: u64,
    ageing: original::Ageing,
    /// Captured panorama sky instead of the procedural atmosphere.
    hdri_sky: bool,
    render: panel::RenderSettings,
    rebuild_scene: bool,
    /// Set when a panel render setting should be applied to the live renderer.
    reconfigure: bool,
    weather: voxy_render::weather::Weather,
    weather_start: Instant,
    day_cycle: Option<day_cycle::DayCycle>,
    panel: Option<panel::Panel>,
    daylight_ev: Option<f32>,
    sun_elevation: Option<f32>,
    wetness: Option<f32>,
    pitch: Option<f32>,
    /// `--spawn pool` starts at the garden gate overlooking the villa pool.
    spawn_pool: bool,
    aa_validation: Option<aa_validation::AaValidation>,
    view: StreetView,
    rig: voxy_render::BodycamRig,
}
impl App {
    fn cycle_view(&mut self) {
        self.view = match self.view {
            StreetView::FirstPerson => StreetView::ThirdPerson,
            StreetView::ThirdPerson => StreetView::Bodycam,
            StreetView::Bodycam => StreetView::Camcorder,
            StreetView::Camcorder => StreetView::FirstPerson,
        };
        self.camera
            .set_distance(if self.view == StreetView::ThirdPerson {
                3.
            } else {
                0.
            });
        let profile = match self.view {
            StreetView::Camcorder => voxy_render::PhotorealLensProfile::Camcorder,
            _ => voxy_render::PhotorealLensProfile::Bodycam,
        };
        let look = self.camera.direction();
        let aim = Vec2::new(look.x.atan2(look.z), look.y.clamp(-1., 1.).asin());
        self.rig = voxy_render::BodycamRig::default();
        self.rig.flashlight_enabled = false;
        self.rig.aim_angles = aim;
        self.rig.chest_goal_angles = aim;
        self.rig.torso_spring.position = Vec3::new(aim.x, aim.y, 0.);
        self.rig.set_lens_profile(profile);
        if let Some(gpu) = &mut self.gpu {
            gpu.renderer.reset_temporal_history();
        }
        eprintln!(
            "STREET view {}",
            match self.view {
                StreetView::FirstPerson => "first person",
                StreetView::ThirdPerson => "third person",
                StreetView::Bodycam => "bodycam",
                StreetView::Camcorder => "camcorder",
            }
        );
    }
    fn new() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let value = |name: &str| {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        let mut app = Self {
            profile: None,
            weather: match value("--weather").as_deref() {
                Some("cloudy") => voxy_render::weather::Weather::CLOUDY,
                Some("rain") => voxy_render::weather::Weather::STORM,
                Some("snow") => voxy_render::weather::Weather::SNOW,
                _ => voxy_render::weather::Weather::CLEAR,
            },
            weather_start: Instant::now(),
            day_cycle: None,
            panel: None,
            daylight_ev: None,
            sun_elevation: value("--sun-elevation").and_then(|v| v.parse().ok()),
            wetness: value("--wetness").and_then(|v| v.parse().ok()),
            pitch: value("--pitch").and_then(|v| v.parse().ok()),
            spawn_pool: value("--spawn").as_deref() == Some("pool"),
            aa_validation: aa_validation::AaValidation::from_environment(),
            view: StreetView::FirstPerson,
            rig: voxy_render::BodycamRig::default(),
            window: None,
            gpu: None,
            probe_bake: None,
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
            simulation_time: 0.,
            simulation_steps: 0,
            measure_simulation_steps: 0,
            measure_water_start: None,
            frames: 0,
            measure_start: None,
            measure_start_unix_seconds: None,
            replay_state: blake3::Hasher::new(),
            durations: Vec::new(),
            skipped_acquires: 0,
            physics_seconds: Vec::new(),
            render_seconds: Vec::new(),
            acquire_seconds: Vec::new(),
            snapshot: value("--snapshot").map(PathBuf::from),
            offscreen_benchmark: args.iter().any(|a| a == "--offscreen-benchmark"),
            render_every_physics_step: args.iter().any(|a| a == "--offscreen-render-every-step"),
            benchmark_complete: false,
            world_ray_audit_frames: value("--world-ray-audit-frames")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            world_ray_audit_complete: false,
            benchmark: value("--benchmark")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.),
            failure: None,
            finished: false,
            original: !args.iter().any(|a| a == "--bistro"),
            ageing: original::Ageing::default(),
            hdri_sky: std::env::var("VOXY_STREET_SKY").as_deref() != Ok("analytic"),
            render: panel::RenderSettings::from_environment()
                .unwrap_or_else(|error| panic!("{error}")),
            rebuild_scene: false,
            reconfigure: false,
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
        #[allow(clippy::needless_borrows_for_generic_args)]
        {
            app.day_cycle = day_cycle::DayCycle::from_args(
                &value,
                app.original_seed,
                value("--weather").is_some(),
            );
        }
        app
    }
    fn init(&mut self, el: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        if self.benchmark > 0. || std::env::var_os("VOXY_STREET_BENCHMARK_PROGRESS").is_some() {
            eprintln!(
                "STREET RUN pid={} benchmark_seconds={} offscreen={} paired_dda={} tiles={} cell_bounds={} far_bound={}",
                std::process::id(),
                self.benchmark,
                self.offscreen_benchmark,
                std::env::var_os("VOXY_STREET_PAIRED_DDA").is_some(),
                std::env::var_os("VOXY_STREET_PAIRED_DDA_TILES").is_some(),
                std::env::var_os("VOXY_STREET_DDA_CELL_BOUNDS").is_some(),
                std::env::var_os("VOXY_STREET_DDA_FAR_BOUND").is_some()
            );
        }
        let interactive = (std::env::var("VOXY_STREET_BENCH_LIVE").as_deref() == Ok("1")
            || self.benchmark == 0. && self.profile.is_none())
            && self.snapshot.is_none()
            && self.aa_validation.is_none();
        let scale = el.primary_monitor().map_or(1., |m| m.scale_factor());
        let panel_px = if interactive {
            (panel::WIDTH as f64 * scale).round() as u32
        } else {
            0
        };
        let window = Arc::new(
            el.create_window(
                Window::default_attributes()
                    .with_visible(false)
                    .with_title(if self.original {
                        "Voxy — Original Street: WASD · Shift · Space · мышь · Esc"
                    } else {
                        "Voxy — Bistro: WASD · Shift · Space · мышь · Esc"
                    })
                    .with_window_level(if self.benchmark > 0. {
                        winit::window::WindowLevel::AlwaysOnTop
                    } else {
                        winit::window::WindowLevel::Normal
                    })
                    .with_inner_size(winit::dpi::PhysicalSize::new(1920 + panel_px, 1080))
                    .with_resizable(false),
            )?,
        );
        window.request_redraw();
        self.window = Some(window.clone());
        if !self.original {
            eprintln!("STREET loading full authored scene {}", self.root.display());
        }
        let mut scene = if self.original {
            original::generate(self.original_seed, self.ageing)?
        } else {
            scene::load(&self.root)?
        };
        let environment = if self.original {
            let (environment, sun, sun_colour) = materials::daylight()?;
            scene.sun = sun;
            scene.sun_colour = sun_colour;
            // The captured panorama is the default sky; the panel or VOXY_STREET_SKY=analytic
            // switches to the procedural atmosphere.
            self.hdri_sky.then_some(environment)
        } else {
            None
        };
        if self.spawn_pool {
            let (eye, direction) = scene
                .pool_view
                .ok_or("--spawn pool needs the original street")?;
            scene.eye = eye;
            scene.direction = direction;
        }
        let mut direction = scene.direction;
        if let Some(degrees) = self.pitch {
            let pitch = degrees.clamp(-80., 80.).to_radians();
            let planar = Vec3::new(direction.x, 0., direction.z).normalize();
            direction = planar * pitch.cos() + Vec3::Y * pitch.sin();
        }
        self.camera = WalkCamera::new(direction);
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
        if let Some(validation) = &mut self.aa_validation {
            std::fs::create_dir_all(&validation.directory)?;
            validation.eye = self.feet.as_vec3() + Vec3::Y * 1.65;
            validation.direction = scene.direction;
        }
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
            | if world_ray_lighting::enabled() {
                wgpu::Features::EXPERIMENTAL_RAY_QUERY
            } else {
                wgpu::Features::empty()
            }
            | if std::env::var_os("VOXY_STREET_RAY_PERSISTENT").is_some() {
                wgpu::Features::SUBGROUP
            } else {
                wgpu::Features::empty()
            }
            | if std::env::var_os("VOXY_STREET_SUBGROUP_CULLING").is_some() {
                adapter.features() & wgpu::Features::SUBGROUP
            } else {
                wgpu::Features::empty()
            }
            | if self.original {
                wgpu::Features::FLOAT32_FILTERABLE
            } else {
                wgpu::Features::empty()
            }
            | if self.profile.is_some() || std::env::var_os("VOXY_STREET_GPU_TIMING").is_some() {
                adapter.features() & wgpu::Features::TIMESTAMP_QUERY
            } else {
                wgpu::Features::empty()
            };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("Voxy full street"),
                required_features: features,
                experimental_features: world_ray_lighting::experimental_features()?,
                required_limits: if world_ray_lighting::enabled() {
                    adapter.limits()
                } else {
                    wgpu::Limits {
                        max_texture_dimension_2d: 16384,
                        max_storage_buffer_binding_size: adapter
                            .limits()
                            .max_storage_buffer_binding_size,
                        ..wgpu::Limits::default()
                    }
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
        if self.snapshot.is_some() || self.aa_validation.is_some() {
            config.usage |= wgpu::TextureUsages::COPY_SRC;
        }
        let panel_px = panel_px.min(size.width / 2);
        if panel_px > 0 {
            config.usage |= wgpu::TextureUsages::COPY_DST;
        }
        let scene_width = size.width - panel_px;
        eprintln!(
            "STREET GPU {:?}; {}x{} {:?}; full scene geometry; original mip chains when images are present",
            adapter.get_info(),
            size.width,
            size.height,
            config.present_mode
        );
        surface.configure(&device, &config);
        let render_scale: u32 = std::env::var("VOXY_STREET_RENDER_SCALE")
            .unwrap_or_else(|_| "1".into())
            .parse()?;
        let supersample = voxy_render::SupersampleResolve::new(
            &device,
            scene_width,
            size.height,
            render_scale,
            config.format,
        )?;
        let (render_width, render_height) = supersample.render_size();
        let world_job = world_ray_lighting::prepare(
            &device,
            &queue,
            &scene.geometry,
            &scene.materials,
            scene.images.len(),
        )?;
        let mut renderer = ArchiveSceneRenderer::new_with_environment(
            &device,
            &queue,
            config.format,
            render_width,
            render_height,
            scene.images,
            scene.materials,
            scene.geometry,
            &scene.lights,
            scene.sun,
            scene.sun_colour,
            scene.bounds,
            environment,
        )?;
        world_ray_lighting::connect(&mut renderer, world_job)?;
        let validation_headroom = self.aa_validation.as_ref().map(|_| 2.);
        let mode = self.configure_renderer(
            &mut renderer,
            validation_headroom.unwrap_or(hdr_headroom.unwrap_or(1.)),
            None,
        )?;
        self.probe_bake = if self.original {
            begin_street_probe_bake(&mut renderer, scene.bounds, self.render.probe_spacing)?
        } else {
            None
        };
        renderer.set_water(&voxy_render::voxy_water::WaterWorld::new(
            scene.water.clone(),
            voxy_render::WaveSpectrum::pool(),
        ));
        eprintln!(
            "STREET antialiasing={mode:?} render={}x{} output={}x{}",
            render_width, render_height, size.width, size.height
        );

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
        let scene_target = (panel_px > 0).then(|| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("street scene beside panel"),
                size: wgpu::Extent3d {
                    width: scene_width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        });
        if interactive {
            self.panel = Some(panel::Panel::new(&window, &device, config.format));
        }
        self.daylight_ev = Some(renderer.exposure_ev100());
        self.gpu = Some(Gpu {
            surface,
            adapter,
            hdr_last_check: Instant::now(),
            hdr_headroom,
            validation_headroom,
            device,
            queue,
            config,
            renderer,
            supersample,
            offscreen,
            scene_target,
            panel_px,
            bodycam: None,
        });
        self.last = Instant::now();
        if !self.offscreen_benchmark && self.probe_bake.is_none() {
            window.set_visible(true);
            window.focus_window();
        }
        if !self.offscreen_benchmark {
            self.capture(false);
        }
        Ok(())
    }
    /// Per-run renderer settings, shared by start-up and scene rebuilds.
    fn configure_renderer(
        &self,
        renderer: &mut ArchiveSceneRenderer,
        headroom: f32,
        keep_mode: Option<voxy_render::ArchiveAntialiasing>,
    ) -> Result<voxy_render::ArchiveAntialiasing, Box<dyn std::error::Error>> {
        if let Some(degrees) = self.sun_elevation {
            let angle = degrees.clamp(-10., 90.).to_radians();
            renderer.set_sun(
                Vec3::new(-angle.cos(), angle.sin(), 0.3 * angle.cos()),
                [1., 0.95, 0.85, 100000.],
            )?;
        }
        renderer.set_surface_wetness(self.wetness.map(|w| w.clamp(0., 1.)))?;
        renderer.set_display_headroom(headroom.max(1.))?;
        let mode = keep_mode.unwrap_or(self.render.antialiasing);
        renderer.set_antialiasing(mode)?;
        // Off by default: a fixed-focus lens fights a first-person walk.
        renderer.set_camera_lens(match self.render.lens {
            Some(lens) => voxy_render::ArchiveCameraLens {
                focal_length_m: 0.05,
                f_number: lens.f_number,
                focus_distance_m: lens.focus_distance_m,
                shutter_fraction: lens.shutter_fraction,
                sensor_width_m: 0.036,
            },
            None => voxy_render::ArchiveCameraLens::default(),
        })?;
        renderer.set_tone_mapper(self.render.tone_mapper);
        Ok(mode)
    }
    /// Regenerates the original street with the current ageing settings and swaps
    /// in a fresh renderer and collision world.
    fn rebuild_original(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let started = Instant::now();
        let mut scene = original::generate(self.original_seed, self.ageing)?;
        let (environment, sun, sun_colour) = materials::daylight()?;
        scene.sun = sun;
        scene.sun_colour = sun_colour;
        let environment = self.hdri_sky.then_some(environment);
        let gpu = self.gpu.as_ref().ok_or("missing gpu")?;
        let (width, height) = gpu.supersample.render_size();
        let world_job = world_ray_lighting::prepare(
            &gpu.device,
            &gpu.queue,
            &scene.geometry,
            &scene.materials,
            scene.images.len(),
        )?;
        let mut renderer = ArchiveSceneRenderer::new_with_environment(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            width,
            height,
            scene.images,
            scene.materials,
            scene.geometry,
            &scene.lights,
            scene.sun,
            scene.sun_colour,
            scene.bounds,
            environment,
        )?;
        world_ray_lighting::connect(&mut renderer, world_job)?;
        let headroom = gpu
            .validation_headroom
            .unwrap_or(gpu.hdr_headroom.unwrap_or(1.));
        self.configure_renderer(&mut renderer, headroom, None)?;
        self.probe_bake = if self.original {
            begin_street_probe_bake(&mut renderer, scene.bounds, self.render.probe_spacing)?
        } else {
            None
        };
        renderer.set_water(&voxy_render::voxy_water::WaterWorld::new(
            scene.water.clone(),
            voxy_render::WaveSpectrum::pool(),
        ));
        let gpu = self.gpu.as_mut().ok_or("missing gpu")?;
        gpu.renderer = renderer;
        gpu.bodycam = None;
        self.collision = Some(scene.collision);
        eprintln!(
            "STREET rebuilt with {:?} in {:.2}s",
            self.ageing,
            started.elapsed().as_secs_f64()
        );
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
        if let Some(mut bake) = self.probe_bake.take() {
            let ready = self
                .gpu
                .as_mut()
                .ok_or("missing probe renderer")?
                .renderer
                .advance_irradiance_probe_bake(&mut bake)?;
            if ready {
                eprintln!(
                    "STREET probes={} completed in {:.3} s before measurement",
                    bake.progress().1,
                    bake.elapsed_seconds()
                );
                if !self.offscreen_benchmark {
                    if let Some(window) = &self.window {
                        window.set_visible(true);
                        window.focus_window();
                    }
                }
            } else {
                self.probe_bake = Some(bake);
            }
            self.last = Instant::now();
            return Ok(());
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        if self.benchmark > 0. || self.profile.is_some() {
            self.input.cancel_all();
        }
        self.camera.apply_look(self.input.take_look());
        // Offscreen comparison must replay identical physics and weather states,
        // independent of shader compilation and previous-frame GPU latency.
        // Real wall time below remains the sole source for FPS measurement.
        let simulation_elapsed = if self.offscreen_benchmark
            && (self.benchmark > 0. || self.world_ray_audit_frames > 0)
        {
            if self.render_every_physics_step { benchmark_cycle::PHYSICS_DT } else { 0.25 }
        } else {
            elapsed
        };
        self.camera.step(simulation_elapsed as f32);
        self.accumulator += simulation_elapsed.min(0.25);
        const DT: f64 = 1. / 120.;
        let collision = self.collision.as_ref().ok_or("missing collision")?;
        let was_grounded = self.grounded;
        let start_feet = self.feet;
        let simulation_time_before = self.simulation_time;
        while self.accumulator >= DT {
            self.simulation_time += DT;
            self.simulation_steps += 1;
            let mut movement = self.input.take_move();
            if self.benchmark > 0. || self.profile.is_some() {
                movement.local = Vec2::new(
                    0.,
                    if self.simulation_time % 20. < 10. {
                        1.
                    } else {
                        -1.
                    },
                );
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
        let mut direction = self.camera.direction();
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
        // The rig runs in every view so the gait phase stays continuous when
        // switching cameras.
        let dt = (simulation_elapsed as f32).max(1e-4);
        let moved = (self.feet - start_feet).as_vec3();
        let planar_speed = Vec2::new(moved.x, moved.z).length() / dt;
        // Keep the rig aimed where the walk camera looks; the rig adds the
        // torso deadzone, footsteps and hand-held sway on top.
        let aim = Vec2::new(
            direction.x.atan2(direction.z),
            direction.y.clamp(-1., 1.).asin(),
        );
        let mut delta = self.rig.aim_angles - aim;
        delta.x = (delta.x + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        self.rig.apply_look(delta);
        self.rig.step(
            dt,
            planar_speed,
            self.grounded,
            was_grounded != self.grounded,
            0.18,
        );
        let lens = if matches!(self.view, StreetView::Bodycam | StreetView::Camcorder) {
            let (view, _, _) = self.rig.camera_and_flashlight(actor);
            eye = view.eye;
            direction = (view.target - view.eye).normalize_or(direction);
            Some((view.vertical_fov_radians, view.up))
        } else {
            if self.view == StreetView::FirstPerson {
                // Eyes ride the same gait, but the neck and vestibulo-ocular reflex
                // cancel most of it: keep a damped bob and sway, no rotation.
                let (gait, _) = self.rig.gait_offsets();
                let (forward, right) = self.camera.planar_basis();
                eye += right * gait.x * 0.3 + Vec3::Y * gait.y * 0.45 + forward * gait.z * 0.3;
            }
            None
        };
        if let Some(validation) = &self.aa_validation {
            (eye, direction) = validation.camera(self.frames);
        }
        let physics_time = now.elapsed().as_secs_f64();
        let acquire_start = Instant::now();
        // Panel render changes from the previous frame apply before this one.
        if std::mem::take(&mut self.reconfigure) {
            let mut gpu = self.gpu.take().ok_or("missing gpu")?;
            let headroom = gpu
                .validation_headroom
                .unwrap_or(gpu.hdr_headroom.unwrap_or(1.));
            let applied = self.configure_renderer(&mut gpu.renderer, headroom, None);
            self.gpu = Some(gpu);
            applied?;
        }
        let gpu = self.gpu.as_mut().ok_or("missing renderer")?;
        if gpu.config.color_space.is_hdr() && gpu.hdr_last_check.elapsed().as_secs_f32() >= 1. {
            let info = gpu.surface.display_hdr_info(&gpu.adapter);
            let headroom = info.tone_map_headroom();
            if headroom != gpu.hdr_headroom {
                eprintln!("STREET live HDR headroom={headroom:?} info={info:?}");
            }
            gpu.hdr_headroom = headroom;
            gpu.renderer.set_display_headroom(
                gpu.validation_headroom
                    .unwrap_or(headroom.unwrap_or(1.))
                    .max(1.),
            )?;
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
                    if let Some(profile) = &mut self.profile {
                        if profile.record(
                            Instant::now(),
                            std::time::Duration::from_secs_f64(physics_time),
                            std::time::Duration::ZERO,
                            std::time::Duration::ZERO,
                            Some(acquire_start.elapsed()),
                            false,
                        ) {
                            profile.finish(&format!("{:?}", gpu.adapter.get_info()))?;
                            self.finished = true;
                            el.exit();
                        }
                    }
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    self.skipped_acquires += 1;
                    if let Some(profile) = &mut self.profile {
                        if profile.record(
                            Instant::now(),
                            std::time::Duration::from_secs_f64(physics_time),
                            std::time::Duration::ZERO,
                            std::time::Duration::ZERO,
                            Some(acquire_start.elapsed()),
                            false,
                        ) {
                            profile.finish(&format!("{:?}", gpu.adapter.get_info()))?;
                            self.finished = true;
                            el.exit();
                        }
                    }
                    return Ok(());
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    return Err("surface validation failed".into());
                }
            })
        };
        let acquire_time = acquire_start.elapsed().as_secs_f64();
        let gpu_samples_before = gpu.renderer.gpu_times_ms.len();
        let render_start = Instant::now();
        let output = frame
            .as_ref()
            .map_or_else(|| gpu.offscreen.as_ref().unwrap(), |f| &f.texture);
        if self.aa_validation.is_some() && self.frames == 128 {
            gpu.renderer.reset_temporal_history();
        }
        if let Some(wet) = self.wetness {
            gpu.renderer.set_surface_wetness(Some(wet.clamp(0., 1.)))?;
        } else if self.day_cycle.is_none() {
            gpu.renderer.set_surface_wetness(None)?;
        }
        if self.day_cycle.is_none()
            && self
                .daylight_ev
                .is_some_and(|ev| (gpu.renderer.exposure_ev100() - ev).abs() > 0.01)
        {
            gpu.renderer.set_exposure_ev100(self.daylight_ev.unwrap())?;
        }
        if let Some(cycle) = &mut self.day_cycle {
            let frozen = self.profile.is_some()
                || self.aa_validation.is_some()
                || self.benchmark > 0.
                    && std::env::var("VOXY_STREET_BENCH_LIVE").as_deref() != Ok("1")
                || self.snapshot.is_some();
            cycle.update(
                &mut gpu.renderer,
                &mut self.weather,
                elapsed,
                frozen,
                self.wetness.is_some(),
            )?;
        }
        gpu.renderer.set_weather(self.weather)?;
        let weather_time = if self.aa_validation.is_some() {
            0.
        } else if self.profile.is_some()
            || self.offscreen_benchmark && (self.benchmark > 0. || self.world_ray_audit_frames > 0)
        {
            self.simulation_time as f32
        } else {
            self.weather_start.elapsed().as_secs_f32()
        };
        gpu.renderer.set_weather_time(weather_time);
        if self.offscreen_benchmark && (self.benchmark > 0. || self.world_ray_audit_frames > 0) {
            // Hash exact bits for every rendered state, including warmup. Final
            // feet alone cannot establish identical interpolated camera paths.
            self.replay_state.update(&self.frames.to_le_bytes());
            self.replay_state
                .update(&self.simulation_steps.to_le_bytes());
            for value in self
                .feet
                .to_array()
                .into_iter()
                .chain(self.previous.to_array())
                .chain([self.simulation_time, self.accumulator, self.velocity_y])
            {
                self.replay_state.update(&value.to_bits().to_le_bytes());
            }
            for value in eye
                .to_array()
                .into_iter()
                .chain(direction.to_array())
                .chain([weather_time, gpu.renderer.exposure_ev100()])
            {
                self.replay_state.update(&value.to_bits().to_le_bytes());
            }
            self.replay_state
                .update(&[u8::from(self.grounded), u8::from(lens.is_some())]);
            if let Some((fov, up)) = lens {
                for value in [fov, up.x, up.y, up.z] {
                    self.replay_state.update(&value.to_bits().to_le_bytes());
                }
            }
        }
        gpu.renderer.advance_water(if self.aa_validation.is_some() {
            0.
        } else if self.profile.is_some() || self.offscreen_benchmark {
            (self.simulation_time - simulation_time_before) as f32
        } else {
            elapsed as f32
        });
        // Wading pushes a wake through the pool; jumping in throws up a splash.
        let (from, to) = (start_feet.as_vec3(), self.feet.as_vec3());
        let wet = gpu
            .renderer
            .water()
            .and_then(|w| w.sample(to, gpu.renderer.water_time()))
            .is_some_and(|s| s.submersion(to) > 0.);
        if wet {
            let (a, b) = (Vec2::new(from.x, from.z), Vec2::new(to.x, to.z));
            if from.y - to.y > 0.05 {
                gpu.renderer.water_splash(b, 0.5, -0.06);
            }
            if a.distance(b) > 1e-3 {
                gpu.renderer.water_wake(a, b, 0.3, 0.01);
            }
        }
        if self.offscreen_benchmark && (self.benchmark > 0. || self.world_ray_audit_frames > 0) {
            self.replay_state.update(b"water-input-v1");
            if let Some(water) = gpu.renderer.water() {
                self.replay_state.update(&[1]);
                self.replay_state
                    .update(&(water.bodies.len() as u64).to_le_bytes());
                self.replay_state
                    .update(&gpu.renderer.water_time().to_bits().to_le_bytes());
            } else {
                self.replay_state.update(&[0]);
            }
        }
        let output_view = output.create_view(&Default::default());
        let scene_view = gpu
            .scene_target
            .as_ref()
            .map(|t| t.create_view(&Default::default()));
        let target_view = scene_view.as_ref().unwrap_or(&output_view);
        let submission = if let Some((fov, up)) = lens.filter(|_| self.aa_validation.is_none()) {
            let (width, height) = gpu.supersample.render_size();
            let post = gpu.bodycam.get_or_insert_with(|| {
                voxy_render::BodycamPostProcess::new(&gpu.device, gpu.config.format, width, height)
            });
            post.resize(&gpu.device, &gpu.queue, width, height);
            post.update_from_rig(&gpu.queue, &self.rig, self.frames);
            gpu.renderer.set_camera_projection(fov, up);
            gpu.renderer.set_linear_output(true);
            gpu.renderer.draw(post.hdr_view(), eye, direction);
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("street bodycam lens"),
                });
            post.encode(&mut encoder, target_view);
            gpu.queue.submit([encoder.finish()])
        } else {
            gpu.renderer
                .set_camera_projection(65_f32.to_radians(), Vec3::Y);
            gpu.renderer.set_linear_output(false);
            gpu.renderer
                .draw_supersampled(&gpu.supersample, target_view, eye, direction)?
        };
        let submission = if let Some(scene) = &gpu.scene_target {
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("street scene beside panel"),
                });
            // Clear the panel strip (it stays dark while the panel is hidden).
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("street panel strip"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            encoder.copy_texture_to_texture(
                scene.as_image_copy(),
                wgpu::TexelCopyTextureInfo {
                    texture: output,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: gpu.panel_px,
                        y: 0,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                scene.size(),
            );
            gpu.queue.submit([encoder.finish()])
        } else {
            submission
        };
        if self.offscreen_benchmark {
            gpu.device.poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })?;
        }
        let render_time = render_start.elapsed().as_secs_f64();
        self.frames += 1;
        if self.offscreen_benchmark && std::env::var_os("VOXY_STREET_BENCHMARK_PROGRESS").is_some()
        {
            eprintln!(
                "STREET completed_frame={} render_ms={:.3} physics_steps={} paired_dda={}",
                self.frames,
                render_time * 1000.,
                self.simulation_steps,
                std::env::var_os("VOXY_STREET_PAIRED_DDA").is_some()
            );
            if gpu.renderer.gpu_times_ms.len() > gpu_samples_before {
                let i = gpu.renderer.gpu_times_ms.len() - 1;
                eprintln!(
                    "STREET GPU collected_at_frame={} total_ms={:.3} opaque_ms={:.3} transparent_ms={:.3}",
                    self.frames,
                    gpu.renderer.gpu_times_ms[i],
                    gpu.renderer.gpu_shading_times_ms[i][0],
                    gpu.renderer.gpu_shading_times_ms[i][1]
                );
                if let Some([guides_ms, traversal_ms]) = gpu.renderer.gpu_gi_times_ms[i] {
                    eprintln!(
                        "STREET GPU GI collected_at_frame={} guides_ms={guides_ms:.3} traversal_ms={traversal_ms:.3}",
                        self.frames
                    );
                }
                if let Some(ms) = gpu.renderer.gpu_ssr_times_ms[i] {
                    eprintln!(
                        "STREET GPU SSR collected_at_frame={} traversal_ms={:.3}",
                        self.frames, ms
                    );
                }
            }
        }
        if let Some(root) = std::env::var_os("VOXY_STREET_HDR_STAGE_CAPTURE") {
            let root = PathBuf::from(root);
            std::fs::create_dir_all(&root)?;
            gpu.renderer.capture_hdr_stages(
                &gpu.supersample,
                output,
                &root.join(format!("frame-{:04}", self.frames)),
                self.frames == self.world_ray_audit_frames
                    || std::env::var("VOXY_STREET_HDR_STAGE_RAW_ALL").as_deref() == Ok("1")
                    || (self.frames == 1
                        && std::env::var("VOXY_STREET_HDR_STAGE_RAW_FIRST").as_deref() == Ok("1")),
            )?;
        }
        if self.world_ray_audit_frames > 0 && self.frames == self.world_ray_audit_frames {
            gpu.renderer.collect_completed_gpu_times()?;
            let source_profile = gpu.renderer.world_ray_stage_profile()?;
            if let Some(profile) = &source_profile {
                let path = std::env::var_os("VOXY_STREET_WORLD_RAY_STAGE_REPORT")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("/tmp/voxy-street-world-ray-stages.json"));
                std::fs::write(path, serde_json::to_vec_pretty(profile)?)?;
            }
            if let Some(path) = self.snapshot.take() {
                snapshot(gpu, output, &path)?;
            }
            let mut report = serde_json::json!({"qualification":"integration_audit_only","simulation_clock":"fixed_30_step_batch","simulation_steps":self.simulation_steps,"rendered_state_blake3":self.replay_state.finalize().to_hex().to_string(),"world_frame_enabled":gpu.renderer.world_ray_frame_enabled(),"world_query_encoded_frames":gpu.renderer.world_ray_query_encoded_frames(),"frames":self.frames,"last_completed_render_ms":render_time*1000.,"source_simplified":false,"internal_resolution":[gpu.supersample.render_size().0,gpu.supersample.render_size().1],"output_resolution":[gpu.config.width,gpu.config.height],"full_collision_triangles":collision.triangle_count(),"world_regions":gpu.renderer.world_ray_region_count(),"gpu_samples_drained":true,"gpu_gi_samples":gpu.renderer.gpu_gi_times_ms,"gpu_ssr_samples":gpu.renderer.gpu_ssr_times_ms,"complete_motion_cycles":0,"minimum_120_presented_fps_passed":false});
            report["water_render_stats"] = gpu
                .renderer
                .water_render_stats()
                .unwrap_or(serde_json::Value::Null);
            report["spawn_pool"] = self.spawn_pool.into();
            report["replay_state_schema"] = "camera-weather-water-v2".into();
            report["paired_dda_lazy_cell_bounds"] = false.into();
            report["paired_dda_touch_guard"] = false.into();
            report["paired_dda_tile_edge_cache"] =
                gpu.renderer.paired_dda_tile_edge_cache_enabled().into();
            report["world_ray_consumer_sorted"] = gpu.renderer.world_ray_consumer_sorted().into();
            report["metal_compute_max_threads_requested"] = 0u32.into();
            if std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE").as_deref() == Ok("1") {
                report["consumer_cache_audit"] = consumer_zero_audit(gpu)?;
            } else if std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_ZERO_AUDIT").as_deref()
                == Ok("1")
            {
                report["consumer_zero_audit"] = consumer_zero_audit(gpu)?;
            }
            report["software_traversal_work"] = software_traversal_work(gpu)?;
            let paired_requested = std::env::var("VOXY_STREET_WORLD_RAY_COMPARE_WIDE").as_deref() == Ok("1");
            if paired_requested {
                let proof = gpu.renderer.world_ray_same_producer_comparison()?.ok_or("same-producer comparison requires one shared source owner")?;
                let tiles = gpu.renderer.world_ray_frame_tiles().ok_or("missing paired frame tiles")?;
                let per_kind: u64 = tiles.iter().map(|t|u64::from(t[2])*u64::from(t[3])).sum();
                let expected = per_kind * (gpu.renderer.world_ray_region_count()/tiles.len()) as u64 * gpu.renderer.world_ray_query_encoded_frames();
                report["same_producer_expected_records"] = expected.into();
                report["same_producer_comparison"] = proof;
            }

            if std::env::var("VOXY_STREET_WORLD_RAY_FLOAT32_HASHES").as_deref() == Ok("1") {
                report["world_ray_float32_images"] = world_ray_float32_images(gpu)?;
            }
            let path = std::env::var_os("VOXY_STREET_WORLD_RAY_AUDIT_REPORT")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/tmp/voxy-street-world-ray-audit.json"));
            std::fs::write(&path, serde_json::to_vec_pretty(&report)?)?;
            if paired_requested && (report["same_producer_comparison"]["all_protected_words_equal"] != true
                || report["same_producer_comparison"]["compared_records"] != report["same_producer_expected_records"]
                || report["same_producer_comparison"]["explicitly_admitted_records"].as_u64().unwrap_or(0) == 0
                || report["same_producer_comparison"]["matched_hit_records"].as_u64().unwrap_or(0) == 0
                || report["same_producer_comparison"]["actual_consumer_HDR"]["all_native_words_equal_and_finite"] != true
                || report["same_producer_comparison"]["actual_consumer_HDR"]["compared_pixels"] != report["same_producer_expected_records"]) {
                return Err("same-producer traversal native-word comparison failed; complete report retained".into());
            }

            eprintln!("STREET WORLD AUDIT COMPLETE {report}");
            self.world_ray_audit_complete = true;
            self.finished = true;
            el.exit();
            return Ok(());
        }
        if let Some(validation) = &mut self.aa_validation {
            validation.frame_ms.push(render_time * 1000.);
            if aa_validation::AaValidation::capture(self.frames) {
                snapshot(
                    gpu,
                    output,
                    &validation
                        .directory
                        .join(format!("frame-{:03}.png", self.frames)),
                )?;
            }
            if self.frames == 160 {
                let report = serde_json::json!({
                    "scene":if self.original {"Original Street"} else {"Bistro"},
                    "antialiasing":format!("{:?}",gpu.renderer.antialiasing()),
                    "adapter":format!("{:?}",gpu.adapter.get_info()),
                    "resolution":[gpu.config.width,gpu.config.height],"internal_resolution":[gpu.supersample.render_size().0,gpu.supersample.render_size().1],
                    "format":format!("{:?}",gpu.config.format),
                    "render_headroom":gpu.validation_headroom.unwrap_or(gpu.hdr_headroom.unwrap_or(1.)),
                    "collision_triangles":collision.triangle_count(),
                    "source_simplified":false,
                    "frames":self.frames,
                    "route":"static 1-32; rotation 33-64; translation 65-96; cut 97; explicit reset 129",
                    "completed_frame_timings":self.offscreen_benchmark,
                    "frame_ms":validation.frame_ms,
                });
                std::fs::write(
                    validation.directory.join("report.json"),
                    serde_json::to_vec_pretty(&report)?,
                )?;
                eprintln!(
                    "STREET AA VALIDATION completed {}",
                    validation.directory.display()
                );
                if validation.cycle_metalfx
                    && gpu.renderer.antialiasing() != voxy_render::ArchiveAntialiasing::MetalFx
                {
                    let next =
                        if gpu.renderer.antialiasing() == voxy_render::ArchiveAntialiasing::Off {
                            voxy_render::ArchiveAntialiasing::Temporal
                        } else {
                            voxy_render::ArchiveAntialiasing::MetalFx
                        };
                    validation.cycle_metalfx = next != voxy_render::ArchiveAntialiasing::MetalFx;
                    validation.directory = validation
                        .directory
                        .parent()
                        .ok_or("capture directory has no parent")?
                        .join(if next == voxy_render::ArchiveAntialiasing::Temporal {
                            "temporal"
                        } else {
                            "metalfx"
                        });
                    std::fs::create_dir_all(&validation.directory)?;
                    validation.frame_ms.clear();
                    gpu.renderer.set_antialiasing(next)?;
                    self.frames = 0;
                    self.measure_start = None;
                    self.durations.clear();
                    eprintln!(
                        "STREET AA VALIDATION continuing with {next:?} on the same loaded scene"
                    );
                } else {
                    self.finished = true;
                    el.exit();
                }
            }
        }
        let capture_now = self.frames >= 120 && self.snapshot.is_some();
        if capture_now {
            if let Some(path) = self.snapshot.take() {
                snapshot(gpu, output, &path)?;
                eprintln!("STREET snapshot {}", path.display());
            }
        }
        if let (Some(panel), Some(window)) = (self.panel.as_mut(), self.window.as_ref()) {
            panel.draw(
                window,
                &gpu.device,
                &gpu.queue,
                &output_view,
                [gpu.config.width, gpu.config.height],
                panel::Settings {
                    cycle: &mut self.day_cycle,
                    weather: &mut self.weather,
                    wetness: &mut self.wetness,
                    ageing: self.original.then_some(&mut self.ageing),
                    hdri_sky: self.original.then_some(&mut self.hdri_sky),
                    seed_mut: self.original.then_some(&mut self.original_seed),
                    sun_elevation: &mut self.sun_elevation,
                    render: &mut self.render,
                    reconfigure: &mut self.reconfigure,
                    rebuild: &mut self.rebuild_scene,
                },
            );
        }
        if let Some(frame) = frame {
            gpu.queue.present(frame);
        }
        if std::mem::take(&mut self.rebuild_scene) {
            return self.rebuild_original();
        }
        if capture_now && gpu.config.usage.contains(wgpu::TextureUsages::COPY_SRC) {
            gpu.config.usage.remove(wgpu::TextureUsages::COPY_SRC);
            gpu.surface.configure(&gpu.device, &gpu.config);
        }
        let _ = gpu.device.poll(wgpu::PollType::Poll);
        if let Some(profile) = &mut self.profile {
            profile.cpu_stage(
                Instant::now(),
                "street_physics",
                std::time::Duration::from_secs_f64(physics_time),
            );
            if gpu.renderer.gpu_times_ms.len() > gpu_samples_before {
                let i = gpu.renderer.gpu_times_ms.len() - 1;
                let shading = gpu.renderer.gpu_shading_times_ms[i];
                profile.gpu_sample(
                    Instant::now(),
                    [
                        Some(gpu.renderer.gpu_times_ms[i]),
                        Some(shading[0]),
                        Some(shading[1]),
                    ],
                );
            }
            if profile.record(
                Instant::now(),
                std::time::Duration::from_secs_f64(physics_time),
                std::time::Duration::ZERO,
                std::time::Duration::from_secs_f64(render_time),
                Some(std::time::Duration::from_secs_f64(acquire_time)),
                true,
            ) {
                profile.finish(&format!("{:?}", gpu.adapter.get_info()))?;
                self.finished = true;
                el.exit();
            }
        }
        // draw collects the previous frame timestamp. At frame 181 the
        // latest collected sample still belongs to warmup, not measurement.
        if self.benchmark > 0. && self.frames == 181 {
            gpu.renderer.gpu_times_ms.clear();
            gpu.renderer.gpu_shading_times_ms.clear();
            gpu.renderer.gpu_gi_times_ms.clear();
            gpu.renderer.gpu_ssr_times_ms.clear();
        }
        if self.frames == 180 {
            self.measure_water_start = gpu.renderer.water_render_stats();
            self.skipped_acquires = 0;
            self.measure_start = Some(Instant::now());
            if self.benchmark > 0. {
                self.measure_start_unix_seconds = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_secs_f64());
            }
            self.measure_simulation_steps = self.simulation_steps;
            self.durations.clear();
        } else if self.frames > 180 {
            // Offscreen draw waited for THIS GPU submission. A start-to-start
            // delta would instead attribute frame 180 to sample 181 and omit
            // the final measured frame. Window sampling keeps its old clock.
            self.durations.push(if self.offscreen_benchmark {
                now.elapsed().as_secs_f64()
            } else {
                elapsed
            });
            self.physics_seconds.push(physics_time);
            self.render_seconds.push(render_time);
            self.acquire_seconds.push(acquire_time);
        }
        if self.frames.is_multiple_of(240) {
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
            && self.measure_start.is_some_and(|t| {
                self.offscreen_benchmark || t.elapsed().as_secs_f64() >= self.benchmark
            })
            && (self.simulation_steps - self.measure_simulation_steps) as f64 * DT >= self.benchmark
        {
            let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
            let mut times = self.durations.clone();
            times.sort_by(f64::total_cmp);
            let n = times.len();
            let measure_seconds = self.measure_start.unwrap().elapsed().as_secs_f64();
            let measure_end_unix_seconds = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs_f64());
            let mut report = serde_json::json!({"measurement_start_unix_seconds":self.measure_start_unix_seconds,"measurement_end_unix_seconds":measure_end_unix_seconds,"antialiasing":format!("{:?}",gpu.renderer.antialiasing()),"scene":if self.original{"Voxy Original Street"}else{"Bistro"},"measurement":if self.offscreen_benchmark{"completed_full_resolution_offscreen_frames"}else{"presented_window_frames"},"resolution":[gpu.config.width,gpu.config.height],"internal_resolution":[gpu.supersample.render_size().0,gpu.supersample.render_size().1],"present_mode":format!("{:?}",gpu.config.present_mode),"source_simplified":false,"camera_controlled":true,"motion_period_seconds":20,"measured_physics_steps":self.simulation_steps-self.measure_simulation_steps,"measured_simulation_seconds":(self.simulation_steps-self.measure_simulation_steps) as f64*DT,"complete_motion_cycles":(self.simulation_steps-self.measure_simulation_steps)/2400,"occluded_or_timed_out_acquires":self.skipped_acquires,"sampled_frame_seconds":self.durations.iter().sum::<f64>(),"sampled_mean_fps":n as f64/self.durations.iter().sum::<f64>(),"window_measurement_valid":!self.offscreen_benchmark&&self.skipped_acquires==0&&!gpu.renderer.opaque_ray_capture_enabled(),"completed_offscreen_measurement_valid":self.offscreen_benchmark&&!gpu.renderer.opaque_ray_capture_enabled(),"frames":n,"seconds":measure_seconds,"mean_fps":n as f64/measure_seconds,"p50_ms":times[n/2]*1000.,"p95_ms":times[n*95/100]*1000.,"p99_ms":times[n*99/100]*1000.,"max_frame_ms":times[n-1]*1000.,"minimum_sampled_fps":1./times[n-1],"minimum_120_presented_fps_passed":!self.offscreen_benchmark&&!gpu.renderer.opaque_ray_capture_enabled()&&self.skipped_acquires==0&&(self.simulation_steps-self.measure_simulation_steps)>=2400&&times[n-1]<=1./120.,"full_collision_triangles":collision.triangle_count(),"feet":self.feet.to_array(),"grounded":self.grounded,"cpu_physics_mean_ms":self.physics_seconds.iter().sum::<f64>()/n as f64*1000.,"cpu_render_mean_ms":self.render_seconds.iter().sum::<f64>()/n as f64*1000.,"surface_acquire_mean_ms":self.acquire_seconds.iter().sum::<f64>()/n as f64*1000.,"gpu_samples":gpu.renderer.gpu_times_ms.len(),"gpu_shading_mean_ms":if gpu.renderer.gpu_shading_times_ms.is_empty(){None}else{Some([0usize,1].map(|i|gpu.renderer.gpu_shading_times_ms.iter().map(|v|v[i]).sum::<f64>()/gpu.renderer.gpu_shading_times_ms.len() as f64))},"gpu_mean_ms":if gpu.renderer.gpu_times_ms.is_empty(){None}else{Some(gpu.renderer.gpu_times_ms.iter().sum::<f64>()/gpu.renderer.gpu_times_ms.len() as f64)}});
            let rendered_cycle = benchmark_cycle::audit(
                self.offscreen_benchmark,
                self.render_every_physics_step,
                gpu.renderer.opaque_ray_capture_enabled(),
                self.simulation_steps-self.measure_simulation_steps,
                &self.durations,
            );
            report["rendered_every_physics_step"] = rendered_cycle.every_physics_step_rendered.into();
            report["complete_120Hz_rendered_motion_cycles"] = rendered_cycle.complete_cycles.into();
            report["minimum_120_completed_frame_fps_passed"] = rendered_cycle.minimum_120_completed_fps.into();
            report["water_scope"] = serde_json::json!({"start":self.measure_water_start,"end":gpu.renderer.water_render_stats()});
            report["spawn_pool"] = self.spawn_pool.into();
            report["replay_state_schema"] = "camera-weather-water-v2".into();
            let gi_samples: Vec<_> = gpu
                .renderer
                .gpu_gi_times_ms
                .iter()
                .filter_map(|v| *v)
                .collect();
            let ssr_samples: Vec<_> = gpu
                .renderer
                .gpu_ssr_times_ms
                .iter()
                .filter_map(|v| *v)
                .collect();
            let settings = serde_json::json!({"replay_state_blake3":self.replay_state.finalize().to_hex().to_string(),"replay_render_frames":self.frames,"benchmark_stop_policy":if self.offscreen_benchmark {"fixed_simulation_duration"}else{"wall_and_simulation_duration"},"simulation_clock":if self.render_every_physics_step {"fixed_one_120Hz_step_per_render"}else if self.offscreen_benchmark {"fixed_30_step_batch"}else{"wall_time"},"measurement_start_simulation_steps":self.measure_simulation_steps,"transmission_depth":gpu.renderer.transmission_depth_enabled(),"ray_persistent":gpu.renderer.opaque_ray_persistent_enabled(),"world_ray_frame":gpu.renderer.world_ray_frame_enabled(),"world_ray_regions":gpu.renderer.world_ray_region_count(),"ray_capture_enabled":gpu.renderer.opaque_ray_capture_enabled(),"ray_hierarchy":gpu.renderer.opaque_ray_hierarchy_enabled(),"ray_hierarchy_reciprocal_cache":gpu.renderer.opaque_ray_hierarchy_reciprocal_enabled(),"ray_hierarchy_near_first":gpu.renderer.opaque_ray_hierarchy_near_first_enabled(),"ray_hierarchy_bounds_bytes":gpu.renderer.opaque_ray_hierarchy_bounds_bytes(),"ray_queue":gpu.renderer.opaque_ray_queue_enabled(),"ray_spatial_key":std::env::var_os("VOXY_STREET_RAY_SPATIAL_KEY").is_some(),"ray_queue_total_encoded_frames":gpu.renderer.opaque_ray_queue_encoded_frames(),"paired_dda_interval_cache":std::env::var_os("VOXY_STREET_DDA_INTERVAL_CACHE").is_some(),"ray_coherent":std::env::var_os("VOXY_STREET_RAY_COHERENT").is_some(),"ray_force_fallback":std::env::var_os("VOXY_STREET_RAY_FORCE_FALLBACK").is_some(),"ray_stages":std::env::var_os("VOXY_STREET_RAY_STAGES").is_some()&&std::env::var_os("VOXY_STREET_COMPUTE_GI").is_some(),"paired_dda_plane_cache":std::env::var_os("VOXY_STREET_DDA_PLANE_CACHE").is_some(),"paired_dda_point_cache":std::env::var_os("VOXY_STREET_DDA_POINT_CACHE").is_some(),"gpu_ssr_samples":ssr_samples.len(),"gpu_ssr_mean_ms":if ssr_samples.is_empty(){None}else{Some(ssr_samples.iter().sum::<f64>()/ssr_samples.len() as f64)},"compute_ssr":std::env::var_os("VOXY_STREET_COMPUTE_GI").is_some()&&std::env::var_os("VOXY_STREET_COMPUTE_SSR").is_some(),"gpu_gi_samples":gi_samples.len(),"gpu_gi_mean_ms":if gi_samples.is_empty(){None}else{Some([0usize,1].map(|i|gi_samples.iter().map(|v|v[i]).sum::<f64>()/gi_samples.len() as f64))},"compute_gi":std::env::var_os("VOXY_STREET_COMPUTE_GI").is_some(),"early_gi":std::env::var_os("VOXY_STREET_EARLY_GI").is_some(),"paired_dda_edge_cache":std::env::var_os("VOXY_STREET_DDA_EDGE_CACHE").is_some(),"paired_depth_dda":std::env::var_os("VOXY_STREET_PAIRED_DDA").is_some(),"paired_dda_tiles":std::env::var_os("VOXY_STREET_PAIRED_DDA_TILES").is_some(),"paired_dda_cell_bounds":std::env::var_os("VOXY_STREET_DDA_CELL_BOUNDS").is_some(),"paired_dda_far_bound":std::env::var_os("VOXY_STREET_DDA_FAR_BOUND").is_some(),"paired_dda_hierarchy":std::env::var_os("VOXY_STREET_DDA_HIERARCHY").is_some(),"refinement_cache":std::env::var_os("VOXY_STREET_REFINE_CACHE").is_some(),"fx_off":std::env::var("VOXY_FX_OFF").ok()});
            if let (serde_json::Value::Object(root), serde_json::Value::Object(settings)) =
                (&mut report, settings)
            {
                root.extend(settings);
            }
            let query_settings = serde_json::json!({
                "metal_compute_max_threads_requested":0u32,
                "metal_compute_audit_requested":std::env::var("WGPU_METAL_COMPUTE_AUDIT").as_deref()==Ok("1"),
                "measurement_duration_clock":"single_monotonic_elapsed",
                "frame_sample_clock":if self.offscreen_benchmark {"current_completed_frame"}else{"start_to_start"},
                "completed_frame_samples_seconds":self.durations,
                "current_render_samples_seconds":self.render_seconds,
                "world_query_encoded_frames":gpu.renderer.world_ray_query_encoded_frames(),
                "world_ray_consumer_cache":std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE").as_deref()==Ok("1"),
                "paired_dda_tile_edge_cache":gpu.renderer.paired_dda_tile_edge_cache_enabled(),
                "world_ray_consumer_sorted":gpu.renderer.world_ray_consumer_sorted(),
                "world_ray_consumer_profile":std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_PROFILE").as_deref()==Ok("1"),
                "world_ray_consumer_cache_check":std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE_CHECK").as_deref()==Ok("1"),
                "paired_dda_lazy_cell_bounds":false,
                "paired_dda_touch_guard":false
            });
            if let (serde_json::Value::Object(root), serde_json::Value::Object(settings)) =
                (&mut report, query_settings)
            {
                root.extend(settings);
            }
            std::fs::write(
                std::env::var_os("VOXY_STREET_BENCHMARK_REPORT")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("/tmp/voxy-street-benchmark.json")),
                serde_json::to_vec_pretty(&report)?,
            )?;
            eprintln!("STREET BENCHMARK {report}");
            if let Some(directory) = std::env::var_os("VOXY_STREET_RAY_CAPTURE_DIR") {
                let directory = PathBuf::from(directory);
                gpu.renderer.save_opaque_ray_capture(&directory)?;
                eprintln!("STREET exact ray diagnostic saved {}", directory.display());
            }
            self.benchmark_complete = true;
            self.finished = true;
            el.exit();
        }
        Ok(())
    }
}
fn software_traversal_work(gpu: &Gpu) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let Some(buffer) = gpu.renderer.world_ray_work_stats_buffer() else {
        if std::env::var("VOXY_STREET_WORLD_RAY_WORK_STATS").as_deref() == Ok("1") {
            return Err("work statistics require one shared prepared source owner".into());
        }
        return Ok(serde_json::Value::Null);
    };
    let read = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("software traversal count snapshot"),
        size: 48,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &read, 0, 48);
    gpu.queue.submit([encoder.finish()]);
    let (send, receive) = std::sync::mpsc::channel();
    read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = send.send(r);
    });
    gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
    receive.recv()??;
    let bytes = read.slice(..).get_mapped_range()?;
    if let Some(path) = std::env::var_os("VOXY_STREET_WORLD_RAY_WORK_RAW") {
        std::fs::write(path, &*bytes)?;
    }
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
        .collect();
    let pair = |i: usize| u64::from(words[i]) | (u64::from(words[i + 1]) << 32);
    if words[10] == u32::MAX || words[11] == u32::MAX {
        return Err("diagnostic per-ray work counter saturated".into());
    }
    let result = serde_json::json!({"scope":"combined GI/SSR, cumulative job lifetime, software certificate only","diagnostic_only":true,"excluded":"first ray in each encode; all hardware and exact-fallback work","sampled_records":pair(0),"visited_nodes":pair(2),"triangle_tests":pair(4),"records_with_nodes":pair(6),"records_with_triangles":pair(8),"maximum_nodes":words[10],"maximum_triangles":words[11],"raw_words":words,"timing_includes_diagnostic_overhead":true,"fps_qualified":false});
    drop(bytes);
    read.unmap();
    Ok(result)
}
fn consumer_zero_audit(gpu: &Gpu) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let cache = std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE").as_deref() == Ok("1");
    let count = gpu.renderer.world_ray_region_count();
    if count == 0 {
        return Ok(serde_json::Value::Null);
    }
    let tiles = gpu
        .renderer
        .world_ray_frame_tiles()
        .map(|v| v.len())
        .unwrap_or(count);
    let read = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("consumer zero audit counters"),
        size: count as u64 * 48,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    for i in 0..count {
        let buffer = gpu
            .renderer
            .world_ray_region_audit_buffer(i)
            .ok_or("missing region audit")?;
        encoder.copy_buffer_to_buffer(buffer, 0, &read, i as u64 * 48, 48);
    }
    gpu.queue.submit([encoder.finish()]);
    let (send, receive) = std::sync::mpsc::channel();
    read.slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = send.send(result);
        });
    gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
    receive.recv()??;
    let bytes = read.slice(..).get_mapped_range()?;
    if let Some(path) = std::env::var_os("VOXY_STREET_WORLD_RAY_CONSUMER_ZERO_RAW") {
        std::fs::write(path, &*bytes)?;
    }
    let mut sums = vec![[0u64; 4]; count.div_ceil(tiles)];
    let mut records = Vec::new();
    for (i, row) in bytes.chunks_exact(48).enumerate() {
        let words: Vec<u32> = row
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect();
        let extra = &words[8..12];
        if cache {
            if extra[2] != 0 {
                return Err("cached fallback changed Float32 bits".into());
            }
        } else if extra[0] != extra[1] + extra[2] + extra[3] {
            return Err("consumer zero classification lost a ray".into());
        }
        for k in 0..4 {
            sums[i / tiles][k] += u64::from(extra[k]);
        }
        records.push(words);
    }
    Ok(
        serde_json::json!({"diagnostic_only":!cache,"consumer_cache":cache,"source_queries_skipped":if !cache {serde_json::json!(0)}else if std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE_CHECK").as_deref()==Ok("1")||std::env::var_os("VOXY_STREET_WORLD_RAY_RESOLVE_AUDIT").is_some() {serde_json::json!(sums.iter().map(|v|v[0]).sum::<u64>())}else{serde_json::Value::Null},"mismatch_counter_scope":"owner lifetime across checked frames","cache_checks_enabled":cache&&std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE_CHECK").as_deref()==Ok("1"),"labels":if cache {vec!["pruned","checked","mismatched","nonfinite"]}else{vec!["eligible","positive_zero","other_finite","nonfinite"]},"by_kind":sums,"region_counters":records,"tiles_per_kind":tiles,"timing_includes_duplicate_fallback_work":!cache||std::env::var("VOXY_STREET_WORLD_RAY_CONSUMER_CACHE_CHECK").as_deref()==Ok("1"),"minimum_120_fps_qualified":false}),
    )
}
// Read the actual Float32 lighting outputs after the completed-frame fence.
// This optional audit preserves every texel bit, without exporting huge images.
fn world_ray_float32_images(gpu: &Gpu) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let (gi, ssr) = gpu
        .renderer
        .opaque_lighting_outputs()
        .ok_or("Float32 lighting hashes require compute GI and SSR outputs")?;
    let mut records = Vec::new();
    for (name, texture) in [("gi", gi), ("ssr", ssr)] {
        if texture.format() != wgpu::TextureFormat::Rgba32Float {
            return Err("lighting hash source is not Float32 RGBA".into());
        }
        let width = texture.width();
        let height = texture.height();
        let row = width * 16;
        let padded = row.div_ceil(256) * 256;
        let read = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("full precision lighting readback"),
            size: u64::from(padded) * u64::from(height),
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
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let submission = gpu.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..).map_async(wgpu::MapMode::Read, move |v| {
            let _ = tx.send(v);
        });
        gpu.device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })?;
        rx.recv()??;
        let bytes = read.slice(..).get_mapped_range()?;
        let mut hash = blake3::Hasher::new();
        let mut nonfinite = 0u64;
        let mut nonzero_rgb = 0u64;
        for raw in bytes.chunks_exact(padded as usize) {
            let pixels = &raw[..row as usize];
            hash.update(pixels);
            for (channel, word) in pixels.chunks_exact(4).enumerate() {
                let bits = u32::from_le_bytes(word.try_into().unwrap());
                nonfinite += u64::from(bits & 0x7f800000 == 0x7f800000);
                nonzero_rgb += u64::from(channel % 4 != 3 && bits & 0x7fffffff != 0);
            }
        }
        records.push(serde_json::json!({"kind":name,"format":"Rgba32Float","size":[width,height],"texels":u64::from(width)*u64::from(height),"unpadded_bytes":u64::from(row)*u64::from(height),"blake3":hash.finalize().to_hex().to_string(),"nonfinite_channels":nonfinite,"nonzero_rgb_channels":nonzero_rgb}));
        drop(bytes);
        read.unmap();
    }
    Ok(
        serde_json::json!({"scope":"all channels of the actual completed Float32 GI and SSR outputs","reference_textures_hashed":false,"row_padding_excluded":true,"readback_after_frame_timing":true,"images":records,"fps_qualified":false}),
    )
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
        let report = serde_json::json!({"surface_format":format!("{:?}",gpu.config.format),"colour_space":format!("{:?}",gpu.config.color_space),"live_headroom":gpu.hdr_headroom,"render_headroom":gpu.validation_headroom.unwrap_or(gpu.hdr_headroom.unwrap_or(1.)),"peak_relative_sdr_white":peak,"pixels_above_sdr_white":above,"pixels":w as usize*h as usize,"finite":true,"native_hdr_capture":raw_path,"png_is_sdr_preview":true});
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
                eprintln!("STREET FAILURE phase=initialization error={e}");
                self.failure = Some(e.to_string());
                el.exit();
            }
        }
    }
    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if self.offscreen_benchmark
            && matches!(
                &event,
                WindowEvent::Focused(_)
                    | WindowEvent::KeyboardInput { .. }
                    | WindowEvent::MouseInput { .. }
                    | WindowEvent::MouseWheel { .. }
                    | WindowEvent::CursorMoved { .. }
            )
        {
            return;
        }
        if self.benchmark > 0. {
            match &event {
                WindowEvent::KeyboardInput { event, .. }
                    if event.physical_key == PhysicalKey::Code(KeyCode::Escape)
                        && event.state == ElementState::Pressed =>
                {
                    eprintln!(
                        "STREET BENCHMARK CANCELLED reason=escape frames={}",
                        self.frames
                    );
                    el.exit();
                    return;
                }
                WindowEvent::KeyboardInput { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::MouseWheel { .. } => return,
                _ => {}
            }
        }
        if let WindowEvent::KeyboardInput { event: key, .. } = &event {
            if key.physical_key == PhysicalKey::Code(KeyCode::F1)
                && key.state == ElementState::Pressed
                && !key.repeat
            {
                if let Some(panel) = self.panel.as_mut() {
                    panel.visible = !panel.visible;
                }
                return;
            }
        }
        if !self.captured {
            if let (Some(panel), Some(window)) = (self.panel.as_mut(), self.window.as_ref()) {
                let consumed = panel.on_event(window, &event);
                let over = panel.wants_pointer()
                    && matches!(
                        event,
                        WindowEvent::MouseInput { .. } | WindowEvent::MouseWheel { .. }
                    );
                if consumed || over {
                    return;
                }
            }
        }
        if self.profile.is_some()
            && matches!(
                event,
                WindowEvent::KeyboardInput { .. }
                    | WindowEvent::MouseInput { .. }
                    | WindowEvent::MouseWheel { .. }
                    | WindowEvent::CursorMoved { .. }
            )
        {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                if self.benchmark > 0. {
                    eprintln!(
                        "STREET BENCHMARK CANCELLED reason=window_close frames={}",
                        self.frames
                    );
                }
                el.exit();
            }
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
                        && self.day_cycle.as_mut().is_some_and(|c| c.key(key))
                    {
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
                        self.cycle_view();
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
                if self.view == StreetView::Camcorder {
                    self.rig.apply_servo_zoom(lines);
                } else {
                    self.camera.zoom(lines);
                }
            }
            WindowEvent::RedrawRequested => {}
            _ => {}
        }
    }
    fn device_event(&mut self, _: &ActiveEventLoop, _: winit::event::DeviceId, event: DeviceEvent) {
        if self.captured && self.benchmark <= 0. {
            if let DeviceEvent::MouseMotion { delta } = event {
                self.input.mouse_motion(delta);
            }
        }
    }
    fn exiting(&mut self, _el: &ActiveEventLoop) {
        if self.benchmark <= 0. {
            return;
        }
        eprintln!(
            "STREET EXIT benchmark_complete={} frames={} simulation_steps={} failure={:?}",
            self.benchmark_complete, self.frames, self.simulation_steps, self.failure
        );
        if !self.benchmark_complete {
            let path = std::env::var_os("VOXY_STREET_BENCHMARK_REPORT")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/tmp/voxy-street-benchmark.json"))
                .with_extension("interrupted.json");
            let diagnostic = serde_json::json!({"qualified":false,"benchmark_complete":false,"frames":self.frames,"simulation_steps":self.simulation_steps,"phase":if self.probe_bake.is_some(){"probe_bake"}else if self.frames<=180{"warmup"}else{"measurement"},"failure":self.failure,"minimum_120_presented_fps_passed":false});
            match serde_json::to_vec_pretty(&diagnostic)
                .map_err(|e| e.to_string())
                .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()))
            {
                Ok(()) => eprintln!("STREET interrupted diagnostic {}", path.display()),
                Err(e) => eprintln!("STREET interruption diagnostic failed: {e}"),
            }
        }
    }
    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if self.gpu.is_some() && !self.finished {
            if let Err(e) = self.draw(el) {
                eprintln!(
                    "STREET FAILURE phase=draw frames={} simulation_steps={} error={e}",
                    self.frames, self.simulation_steps
                );
                self.failure = Some(e.to_string());
                self.finished = true;
                el.exit();
            }
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(directory) = std::env::var_os("VOXY_STREET_EXPORT_RAY_GEOMETRY") {
        let app = App::new();
        if !app.original {
            return Err("ray geometry export requires the original street".into());
        }
        let scene = original::generate(app.original_seed, app.ageing)?;
        ray_geometry_export::write(std::path::Path::new(&directory), &scene, app.original_seed)?;
        return Ok(());
    }
    let profile = frame_profile::Profile::parse(std::env::args().skip(1), true)
        .map_err(std::io::Error::other)?;
    let mut event_builder = EventLoop::builder();
    #[cfg(target_os = "macos")]
    if std::env::args().any(|a| a == "--offscreen-benchmark") {
        use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
        event_builder
            .with_activation_policy(ActivationPolicy::Accessory)
            .with_activate_ignoring_other_apps(false)
            .with_default_menu(false);
    }
    let el = event_builder.build()?;
    el.set_control_flow(ControlFlow::Poll);
    let mut app = App::new();
    if profile.is_some()
        && (app.benchmark > 0.
            || app.offscreen_benchmark
            || app.aa_validation.is_some()
            || app.snapshot.is_some())
    {
        return Err(
            "--profile requires an on-screen run without --benchmark, snapshot or AA validation"
                .into(),
        );
    }
    if std::env::args().any(|a| a == "--world-ray-audit-frames")
        && (app.world_ray_audit_frames < 4
            || !app.offscreen_benchmark
            || app.benchmark > 0.
            || !world_ray_lighting::enabled()
            || profile.is_some()
            || app.aa_validation.is_some())
    {
        return Err("world ray audit requires at least four offscreen frames, world-ray mode, and no timed benchmark/profile/AA comparison".into());
    }
    if std::env::var("VOXY_STREET_WORLD_RAY_COMPARE_WIDE").as_deref() == Ok("1")
        && (app.world_ray_audit_frames < 4 || !app.offscreen_benchmark || app.benchmark > 0.
            || std::env::var("VOXY_STREET_WORLD_RAY_STAGE_PROFILE").as_deref()==Ok("1")
            || std::env::var("VOXY_STREET_WORLD_RAY_FRAME").as_deref()!=Ok("1")
            || std::env::var("VOXY_STREET_WORLD_RAY_WIDE_NODES").as_deref()!=Ok("1")
            || std::env::var("VOXY_STREET_WORLD_RAY_WORK_STATS").as_deref()==Ok("1")) {
        return Err("same-producer comparison requires full-frame wide offscreen world ray audit without timing profile or work statistics".into());
    }
    if app.render_every_physics_step && (!app.offscreen_benchmark || app.benchmark <= 0. || app.world_ray_audit_frames > 0 || profile.is_some()) {
        return Err("--offscreen-render-every-step requires an offscreen timed benchmark without audit/profile mode".into());
    }
    app.profile = profile;
    let event_result = el.run_app(&mut app);
    if app.benchmark > 0. || std::env::var_os("VOXY_STREET_BENCHMARK_PROGRESS").is_some() {
        eprintln!(
            "STREET EVENT LOOP RETURN result={event_result:?} frames={} simulation_steps={} benchmark_complete={}",
            app.frames, app.simulation_steps, app.benchmark_complete
        );
    }
    event_result?;
    if let Some(e) = app.failure {
        return Err(e.into());
    }
    if app.benchmark > 0. && !app.benchmark_complete {
        return Err("street benchmark interrupted before complete report".into());
    }
    if app.world_ray_audit_frames > 0 && !app.world_ray_audit_complete {
        return Err("world ray audit interrupted before completion".into());
    }
    if app.profile.as_ref().is_some_and(|p| !p.is_complete()) {
        return Err("street profile interrupted before report completion".into());
    }
    Ok(())
}

/// Irradiance probes along the street corridor between the facades, so
/// shaded walls and pavement receive light bounced off the street rather than
/// only the open sky. `None` spacing skips the bake.
fn begin_street_probe_bake(
    renderer: &mut ArchiveSceneRenderer,
    bounds: (Vec3, Vec3),
    spacing: Option<f32>,
) -> Result<Option<voxy_render::archive_scene::ArchiveProbeBake>, Box<dyn std::error::Error>> {
    let Some(spacing) = spacing else {
        return Ok(None);
    };
    let min = Vec3::new(-7., 0.6, bounds.0.z + 4.);
    let max = Vec3::new(7., 12., bounds.1.z - 4.);
    Ok(Some(
        renderer.begin_irradiance_probe_bake(min, max, spacing)?,
    ))
}
