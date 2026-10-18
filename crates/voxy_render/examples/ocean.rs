//! FFT ocean water variant with a free camera.
//!
//! Window: click to capture the mouse (Esc releases, Esc again quits), WASD move,
//! Space/E up, Q/C down, Shift x5, wheel changes speed. Settings: Up/Down sun elevation,
//! Left/Right sun azimuth, `[`/`]` wind speed, `-`/`=` swell height, `,`/`.` choppiness,
//! `9`/`0` low clouds, `F`/`G` foam, `1`/`2` exposure.
//!
//! Headless: `--shot ocean.png [--frames 240] [--size 1280x720] [--cam x,y,z,yawDeg,pitchDeg]`
//! renders fixed 1/60 s steps offscreen and writes a PNG.
use std::{collections::HashSet, sync::Arc, time::Instant};
use voxy_render::{
    GraphicsOptions, OceanCamera, OceanRenderer, OceanSettings, RendererError, SceneSurface,
};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

struct Args {
    shot: Option<String>,
    frames: u32,
    size: [u32; 2],
    camera: OceanCamera,
}

fn parse_args() -> Result<Args, Box<dyn std::error::Error>> {
    let mut args = Args {
        shot: None,
        frames: 240,
        size: [1280, 720],
        camera: OceanCamera::default(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{flag} expects a value"));
        match flag.as_str() {
            "--shot" => args.shot = Some(value()?),
            "--frames" => args.frames = value()?.parse()?,
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--size expects WxH")?;
                args.size = [w.parse()?, h.parse()?];
            }
            "--cam" => {
                let v: Vec<f32> = value()?
                    .split(',')
                    .map(str::parse)
                    .collect::<Result<_, _>>()?;
                if v.len() < 3 {
                    return Err("--cam expects x,y,z[,yawDeg,pitchDeg]".into());
                }
                let c = &mut args.camera;
                c.position = glam::Vec3::new(v[0], v[1], v[2]);
                c.yaw = v.get(3).copied().unwrap_or(0.0).to_radians();
                c.pitch = v.get(4).copied().unwrap_or(0.0).to_radians();
            }
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    Ok(args)
}

fn shot(args: &Args, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let options = GraphicsOptions::default();
    let instance = options.create_instance();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: options.power_preference,
        force_fallback_adapter: options.force_fallback_adapter,
        compatible_surface: None,
        apply_limit_buckets: false,
    }))?;
    println!("Adapter: {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("ocean shot device"),
        required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
        ..Default::default()
    }))?;
    let [w, h] = args.size;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ocean shot"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target.create_view(&Default::default());
    let mut ocean = OceanRenderer::new(&device, &queue, format, w, h);
    let mut camera = args.camera;
    let start = Instant::now();
    for _ in 0..args.frames {
        let mut encoder = device.create_command_encoder(&Default::default());
        ocean.encode(&queue, &mut encoder, &target_view, &camera, 1.0 / 60.0);
        queue.submit([encoder.finish()]);
        device.poll(wgpu::PollType::wait_indefinitely())?;
        camera.position.y = camera.position.y.max(ocean.water_height() + 0.9);
    }
    let bytes_per_row = (w * 4).div_ceil(256) * 256;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ocean shot readback"),
        size: u64::from(bytes_per_row * h),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(h),
            },
        },
        target.size(),
    );
    queue.submit([encoder.finish()]);
    readback.map_async(wgpu::MapMode::Read, .., |result| {
        result.expect("map ocean shot")
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let mapped = readback.get_mapped_range(..)?;
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for row in mapped.chunks_exact(bytes_per_row as usize) {
        pixels.extend_from_slice(&row[..(w * 4) as usize]);
    }
    image::save_buffer(path, &pixels, w, h, image::ColorType::Rgba8)?;
    println!(
        "OCEAN SHOT {path}: {} frames in {:.1}s, {} CDLOD nodes, water height {:.2} m",
        args.frames,
        start.elapsed().as_secs_f32(),
        ocean.node_count(),
        ocean.water_height()
    );
    Ok(())
}

struct App {
    window: Option<Arc<Window>>,
    host: Option<SceneSurface>,
    ocean: Option<OceanRenderer>,
    camera: OceanCamera,
    settings: OceanSettings,
    speed: f32,
    keys: HashSet<KeyCode>,
    captured: bool,
    looking: bool,
    last: Instant,
    fps_time: f32,
    fps_frames: u32,
    failure: Option<String>,
}

impl App {
    fn new(camera: OceanCamera) -> Self {
        Self {
            window: None,
            host: None,
            ocean: None,
            camera,
            settings: OceanSettings::default(),
            speed: 6.0,
            keys: HashSet::new(),
            captured: false,
            looking: false,
            last: Instant::now(),
            fps_time: 0.0,
            fps_frames: 0,
            failure: None,
        }
    }

    fn set_capture(&mut self, captured: bool) {
        let Some(window) = &self.window else { return };
        self.captured = captured;
        if captured {
            let _ = window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
        } else {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
        }
        window.set_cursor_visible(!captured);
    }

    fn adjust(&mut self, key: KeyCode) {
        let s = &mut self.settings;
        let (field, step, min, max) = match key {
            KeyCode::ArrowUp => (&mut s.sun_elevation, 0.5, -2.0, 60.0),
            KeyCode::ArrowDown => (&mut s.sun_elevation, -0.5, -2.0, 60.0),
            KeyCode::ArrowRight => (&mut s.sun_azimuth, 2.0, -180.0, 180.0),
            KeyCode::ArrowLeft => (&mut s.sun_azimuth, -2.0, -180.0, 180.0),
            KeyCode::BracketRight => (&mut s.wind_speed, 0.5, 2.0, 20.0),
            KeyCode::BracketLeft => (&mut s.wind_speed, -0.5, 2.0, 20.0),
            KeyCode::Equal => (&mut s.swell_height, 0.1, 0.0, 4.0),
            KeyCode::Minus => (&mut s.swell_height, -0.1, 0.0, 4.0),
            KeyCode::Period => (&mut s.choppiness, 0.05, 0.0, 1.6),
            KeyCode::Comma => (&mut s.choppiness, -0.05, 0.0, 1.6),
            KeyCode::Digit0 => (&mut s.clouds, 0.05, 0.0, 1.0),
            KeyCode::Digit9 => (&mut s.clouds, -0.05, 0.0, 1.0),
            KeyCode::KeyG => (&mut s.foam, 0.1, 0.0, 3.0),
            KeyCode::KeyF => (&mut s.foam, -0.1, 0.0, 3.0),
            KeyCode::Digit2 => (&mut s.exposure, 0.05, 0.1, 4.0),
            KeyCode::Digit1 => (&mut s.exposure, -0.05, 0.1, 4.0),
            _ => return,
        };
        *field = (*field + step).clamp(min, max);
    }

    fn update_camera(&mut self, dt: f32) {
        let held = |k| self.keys.contains(&k);
        let axis = |pos, neg| f32::from(u8::from(held(pos))) - f32::from(u8::from(held(neg)));
        let forward = axis(KeyCode::KeyW, KeyCode::KeyS);
        let strafe = axis(KeyCode::KeyD, KeyCode::KeyA);
        let up = f32::from(u8::from(held(KeyCode::KeyE) || held(KeyCode::Space)))
            - f32::from(u8::from(held(KeyCode::KeyQ) || held(KeyCode::KeyC)));
        let boost = if held(KeyCode::ShiftLeft) || held(KeyCode::ShiftRight) {
            5.0
        } else {
            1.0
        };
        let [f, r, _] = self.camera.basis();
        let motion = f * forward + r * strafe + glam::Vec3::Y * up;
        let c = &mut self.camera;
        c.position += motion * self.speed * boost * dt;
        c.position.y = c.position.y.clamp(1.0, 3000.0);
        // keep clear of the displaced water surface under the camera
        if let Some(ocean) = &self.ocean {
            let floor = ocean.water_height() + 0.9;
            if c.position.y < floor {
                c.position.y += (floor - c.position.y) * (dt * 12.0).min(1.0);
            }
        }
    }

    fn draw(&mut self) -> Result<(), RendererError> {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;
        self.update_camera(dt);
        let (Some(host), Some(ocean), Some(window)) =
            (&mut self.host, &mut self.ocean, &self.window)
        else {
            return Ok(());
        };
        ocean.set_settings(self.settings);
        let size = window.inner_size();
        ocean.resize(host.device(), host.queue(), size.width, size.height);
        let queue = host.queue().clone();
        let camera = self.camera;
        host.render_custom(|encoder, view| {
            ocean.encode(&queue, encoder, view, &camera, dt);
            Ok::<(), RendererError>(())
        })?;
        let _ = host.device().poll(wgpu::PollType::Poll);
        self.fps_time += dt;
        self.fps_frames += 1;
        if self.fps_time > 0.5 {
            let s = &self.settings;
            window.set_title(&format!(
                "Voxy ocean — {:.0} fps, {} nodes | sun {:.1}°/{:.0}° wind {:.1} m/s swell {:.1} m chop {:.2}",
                self.fps_frames as f32 / self.fps_time,
                ocean.node_count(),
                s.sun_elevation,
                s.sun_azimuth,
                s.wind_speed,
                s.swell_height,
                s.choppiness
            ));
            self.fps_time = 0.0;
            self.fps_frames = 0;
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
                        .with_title("Voxy ocean")
                        .with_inner_size(winit::dpi::PhysicalSize::new(1600, 900)),
                )?,
            );
            let size = window.inner_size();
            let options = GraphicsOptions::default();
            let instance = options.create_instance_with_display(event_loop.owned_display_handle());
            let host = pollster::block_on(SceneSurface::new_with_instance(
                Arc::clone(&window),
                size.width,
                size.height,
                options,
                instance,
            ))?;
            self.ocean = Some(OceanRenderer::new(
                host.device(),
                host.queue(),
                host.color_format(),
                size.width,
                size.height,
            ));
            self.host = Some(host);
            window.request_redraw();
            self.window = Some(window);
            self.last = Instant::now();
            Ok(())
        })();
        if let Err(error) = result {
            self.failure = Some(error.to_string());
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(host) = &mut self.host
                    && let Err(error) = host.resize(size.width, size.height)
                {
                    self.failure = Some(error.to_string());
                    event_loop.exit();
                }
            }
            WindowEvent::Focused(false) => {
                self.keys.clear();
                self.set_capture(false);
            }
            WindowEvent::MouseInput { state, button, .. } => match button {
                MouseButton::Left if state == ElementState::Pressed => self.set_capture(true),
                MouseButton::Right => self.looking = state == ElementState::Pressed,
                _ => {}
            },
            WindowEvent::MouseWheel { delta, .. } => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                self.speed = (self.speed * 1.15f32.powf(y)).clamp(0.5, 400.0);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(key) = event.physical_key else {
                    return;
                };
                if event.state == ElementState::Pressed {
                    if key == KeyCode::Escape {
                        if self.captured {
                            self.set_capture(false);
                        } else {
                            event_loop.exit();
                        }
                        return;
                    }
                    self.keys.insert(key);
                    self.adjust(key);
                } else {
                    self.keys.remove(&key);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.draw() {
                    self.failure = Some(error.to_string());
                    event_loop.exit();
                    return;
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: winit::event::DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event
            && (self.captured || self.looking)
        {
            let c = &mut self.camera;
            c.yaw += delta.0 as f32 * 0.0022;
            c.pitch = (c.pitch - delta.1 as f32 * 0.0022).clamp(-1.5, 1.5);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    if let Some(path) = &args.shot {
        return shot(&args, path);
    }
    let mut app = App::new(args.camera);
    EventLoop::new()?.run_app(&mut app)?;
    match app.failure {
        Some(error) => Err(error.into()),
        None => Ok(()),
    }
}
