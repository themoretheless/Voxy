//! `--control-stdio`: keep the street running and drive it with JSON commands.
//!
//! One command per stdin line, one reply per stdout line (`{"id":…,"ok":true,…}`); stdout
//! carries nothing else. `{"event":"ready"}` is printed once the scene is warm. Commands:
//! `camera {eye,look | place} | release`, `weather {…}`, `time {hour}`,
//! `tour {on,time}` (the cinematic fly-through, for measuring in motion),
//! `capture {path,frames,interval_ms,settle_ms}`, `stats {seconds}`, `quit`.
//! The camera is pinned where it was put (no gravity) until `release`.
use super::{App, apply_teleport_state, day_cycle, panel};
use glam::{DVec3, Vec3};
use serde_json::{Value, json};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use winit::event_loop::ActiveEventLoop;

pub struct Control {
    commands: mpsc::Receiver<String>,
    pub pinned: Option<DVec3>,
    capture: Option<(Value, PathBuf)>,
    stats: Option<Stats>,
    ready: bool,
}
struct Stats {
    id: Value,
    until: Instant,
    frames_ms: Vec<f64>,
    eyes: Vec<[f32; 3]>,
    gpu_from: usize,
    shading_from: usize,
}
impl Control {
    pub fn spawn() -> Self {
        let (sender, commands) = mpsc::channel();
        std::thread::spawn(move || {
            for line in std::io::stdin().lines() {
                let Ok(line) = line else { break };
                if sender.send(line).is_err() {
                    return;
                }
            }
            // The driver closed the pipe: shut the scene down.
            let _ = sender.send(r#"{"cmd":"quit"}"#.into());
        });
        Self {
            commands,
            pinned: None,
            capture: None,
            stats: None,
            ready: false,
        }
    }
}
fn reply(value: Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{value}");
    let _ = out.flush();
}
fn vec3(v: &Value) -> Option<Vec3> {
    let a = v.as_array()?;
    let p = Vec3::new(
        a.first()?.as_f64()? as f32,
        a.get(1)?.as_f64()? as f32,
        a.get(2)?.as_f64()? as f32,
    );
    (a.len() == 3 && p.is_finite()).then_some(p)
}
fn percentile(sorted: &[f64], q: f64) -> f64 {
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}
impl App {
    /// Applies queued commands and finishes timed ones. `frame_seconds` is the wall time
    /// since the previous draw.
    pub(super) fn control_poll(&mut self, el: &ActiveEventLoop, frame_seconds: f64) {
        let Some(mut control) = self.control.take() else {
            return;
        };
        if !control.ready && self.frames >= 120 {
            control.ready = true;
            reply(json!({"event": "ready", "frames": self.frames}));
        }
        if let Some(stats) = &mut control.stats {
            stats.frames_ms.push(frame_seconds * 1000.);
            stats
                .eyes
                .push((self.feet + DVec3::Y * 1.65).as_vec3().to_array());
            if Instant::now() >= stats.until {
                let stats = control.stats.take().unwrap();
                reply(self.stats_report(stats));
            }
        }
        while let Ok(line) = control.commands.try_recv() {
            let Ok(command) = serde_json::from_str::<Value>(&line) else {
                reply(json!({"ok": false, "error": "invalid JSON"}));
                continue;
            };
            let id = command["id"].clone();
            match self.control_apply(&mut control, &command, el) {
                Ok(Some(mut result)) => {
                    result["id"] = id;
                    result["ok"] = true.into();
                    reply(result);
                }
                // Deferred: capture and stats reply when they complete.
                Ok(None) => {}
                Err(error) => reply(json!({"id": id, "ok": false, "error": error})),
            }
        }
        self.control = Some(control);
    }
    fn control_apply(
        &mut self,
        control: &mut Control,
        command: &Value,
        el: &ActiveEventLoop,
    ) -> Result<Option<Value>, String> {
        match command["cmd"].as_str().unwrap_or("") {
            "camera" => self.apply_camera_command(control, command).map(Some),
            "tour" => {
                self.tour_active = command["on"].as_bool().unwrap_or(true);
                if let Some(time) = command["time"].as_f64() {
                    self.tour_time = time.max(0.);
                }
                if self.tour_active {
                    control.pinned = None;
                }
                Ok(Some(
                    json!({"tour": self.tour_active, "time": self.tour_time}),
                ))
            }
            "release" => {
                control.pinned = None;
                Ok(Some(json!({})))
            }
            "weather" => Ok(Some(self.apply_weather_command(command))),
            "time" => {
                let hour = command["hour"].as_f64().ok_or("time needs hour")?;
                let cycle = self.day_cycle.get_or_insert_with(|| {
                    let mut cycle = day_cycle::DayCycle::new(self.original_seed);
                    cycle.auto_weather = false;
                    cycle
                });
                cycle.set_hour_of_day(hour.rem_euclid(24.));
                cycle.paused = true;
                Ok(Some(json!({"hour": hour.rem_euclid(24.)})))
            }
            "capture" => {
                self.apply_capture_command(control, command)?;
                Ok(None)
            }
            "stats" => {
                self.apply_stats_command(control, command);
                Ok(None)
            }
            "quit" => {
                self.finished = true;
                el.exit();
                Ok(Some(json!({})))
            }
            other => Err(format!("unknown cmd {other:?}")),
        }
    }
    fn apply_camera_command(
        &mut self,
        control: &mut Control,
        command: &Value,
    ) -> Result<Value, String> {
        let (feet, direction) = if let Some(place) = command["place"].as_str() {
            let (_, feet, direction) = panel::TOUR_WAYPOINTS
                .iter()
                .find(|(name, ..)| name.to_lowercase().contains(&place.to_lowercase()))
                .ok_or_else(|| {
                    format!(
                        "unknown place; one of: {}",
                        panel::TOUR_WAYPOINTS.map(|w| w.0).join(", ")
                    )
                })?;
            (DVec3::from_array(*feet), Vec3::from_array(*direction))
        } else {
            let eye = vec3(&command["eye"]).ok_or("camera needs eye [x,y,z] or place")?;
            let direction = match vec3(&command["look"]) {
                Some(look) => (look - eye).normalize_or(self.camera.direction()),
                None => self.camera.direction(),
            };
            (eye.as_dvec3() - DVec3::Y * 1.65, direction)
        };
        apply_teleport_state(
            &mut self.feet,
            &mut self.previous,
            &mut self.velocity_y,
            &mut self.grounded,
            &mut self.camera,
            feet.to_array(),
            direction.normalize().to_array(),
        );
        self.tour_active = false;
        control.pinned = Some(feet);
        if let Some(gpu) = &mut self.gpu {
            gpu.renderer.reset_temporal_history();
        }
        Ok(json!({
            "eye": (feet + DVec3::Y * 1.65).to_array(),
            "direction": self.camera.direction().to_array()
        }))
    }
    fn apply_weather_command(&mut self, command: &Value) -> Value {
        let number = |key: &str| command[key].as_f64().map(|v| v as f32);
        let weather = &mut self.weather;
        let [x, z] = weather.wind;
        let speed = number("wind_speed").unwrap_or(x.hypot(z)).clamp(0., 100.);
        let angle = number("wind_direction").map_or(z.atan2(x), f32::to_radians);
        weather.wind = [angle.cos() * speed, angle.sin() * speed];
        if let Some(v) = number("wind_gust") {
            weather.wind_gust = v.clamp(0., 1.);
        }
        for (key, field) in [
            ("coverage", &mut weather.coverage),
            ("density", &mut weather.density),
            ("rain", &mut weather.rain),
            ("snow", &mut weather.snow),
        ] {
            if let Some(v) = command[key].as_f64() {
                *field = (v as f32).clamp(0., 1.);
            }
        }
        if let Some(v) = number("wetness") {
            self.wetness = Some(v.clamp(0., 1.));
        }
        // An explicit weather must not be replaced by the seasonal schedule.
        if let Some(cycle) = &mut self.day_cycle {
            cycle.auto_weather = false;
        }
        let w = self.weather;
        json!({
            "wind": w.wind, "wind_gust": w.wind_gust, "coverage": w.coverage,
            "density": w.density, "rain": w.rain, "snow": w.snow
        })
    }
    fn apply_capture_command(
        &mut self,
        control: &mut Control,
        command: &Value,
    ) -> Result<(), String> {
        if control.capture.is_some() {
            return Err("a capture is already running".into());
        }
        let path = PathBuf::from(command["path"].as_str().ok_or("capture needs path")?);
        self.capture_frames = command["frames"].as_u64().unwrap_or(1).clamp(1, 64) as usize;
        self.capture_taken = 0;
        self.capture_interval =
            Duration::from_millis(command["interval_ms"].as_u64().unwrap_or(100));
        // Let temporal history settle after a camera or weather change.
        self.capture_due = Some(
            Instant::now() + Duration::from_millis(command["settle_ms"].as_u64().unwrap_or(300)),
        );
        self.snapshot = Some(path.clone());
        control.capture = Some((command["id"].clone(), path));
        Ok(())
    }
    fn apply_stats_command(&mut self, control: &mut Control, command: &Value) {
        let seconds = command["seconds"].as_f64().unwrap_or(3.).clamp(0.2, 60.);
        let (gpu_from, shading_from) = self.gpu.as_ref().map_or((0, 0), |gpu| {
            (
                gpu.renderer.gpu_times_ms.len(),
                gpu.renderer.gpu_shading_times_ms.len(),
            )
        });
        control.stats = Some(Stats {
            id: command["id"].clone(),
            until: Instant::now() + Duration::from_secs_f64(seconds),
            frames_ms: Vec::new(),
            eyes: Vec::new(),
            gpu_from,
            shading_from,
        });
    }
    /// Called when the last frame of a capture has been written.
    pub(super) fn control_capture_done(&mut self) {
        let Some(control) = &mut self.control else {
            return;
        };
        let Some((id, path)) = control.capture.take() else {
            return;
        };
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("frame");
        let files: Vec<String> = (0..self.capture_taken)
            .map(|i| {
                path.with_file_name(format!("{stem}-{i:02}.png"))
                    .display()
                    .to_string()
            })
            .collect();
        self.capture_due = None;
        reply(json!({
            "id": id, "ok": true, "files": files,
            "eye": (self.feet + DVec3::Y * 1.65).to_array(),
            "direction": self.camera.direction().to_array()
        }));
    }
    fn stats_report(&self, stats: Stats) -> Value {
        let mut ms = stats.frames_ms;
        if ms.is_empty() {
            return json!({"id": stats.id, "ok": false, "error": "no frames"});
        }
        let total: f64 = ms.iter().sum();
        let frames = ms.len();
        let over_120 = ms.iter().filter(|t| **t > 1000. / 120.).count();
        let over_60 = ms.iter().filter(|t| **t > 1000. / 60.).count();
        // The slowest frames in order of occurrence: when and where each hitch happened.
        let mut order: Vec<usize> = (0..frames).collect();
        order.sort_by(|a, b| ms[*b].total_cmp(&ms[*a]));
        order.truncate(8);
        order.sort_unstable();
        let mut at = 0.;
        let mut starts = Vec::with_capacity(frames);
        for t in &ms {
            starts.push(at);
            at += t / 1000.;
        }
        let worst: Vec<Value> = order
            .iter()
            .map(|&i| json!({"frame": i, "at_s": starts[i], "ms": ms[i], "eye": stats.eyes.get(i)}))
            .collect();
        ms.sort_by(f64::total_cmp);
        let mean = |values: &[f64]| {
            (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
        };
        let (gpu_ms, shading) = self.gpu.as_ref().map_or((None, None), |gpu| {
            let r = &gpu.renderer;
            let gpu_ms = mean(&r.gpu_times_ms[stats.gpu_from.min(r.gpu_times_ms.len())..]);
            let s = &r.gpu_shading_times_ms[stats.shading_from.min(r.gpu_shading_times_ms.len())..];
            let shading = (!s.is_empty())
                .then(|| [0, 1].map(|i| s.iter().map(|v| v[i]).sum::<f64>() / s.len() as f64));
            (gpu_ms, shading)
        });
        json!({
            "id": stats.id, "ok": true, "frames": frames, "seconds": total / 1000.,
            "mean_fps": frames as f64 / (total / 1000.),
            "p50_ms": percentile(&ms, 0.5), "p95_ms": percentile(&ms, 0.95),
            "p99_ms": percentile(&ms, 0.99), "max_ms": ms[frames - 1],
            "over_120hz_frames": over_120, "over_60hz_frames": over_60, "worst_frames": worst,
            "gpu_scene_mean_ms": gpu_ms, "gpu_opaque_transparent_mean_ms": shading,
            "monitor_name": self.pacer.monitor().name,
            "monitor_refresh_hz": self.pacer.monitor().hz,
            "present_mode": self.gpu.as_ref().map(|g| format!("{:?}", g.config.present_mode)),
            "resolution": self.gpu.as_ref().map(|g| [g.config.width, g.config.height])
        })
    }
}
