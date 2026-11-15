//! Stdio MCP server that lets an agent look at, drive and measure the street walk scene.
//!
//! It keeps one warm `street_walk --control-stdio` child (built next to this binary), so
//! after the first call a camera move, weather change or capture takes about a second.
//! Tools: `street_set`, `street_capture` (frames + motion heat map), `street_motion`
//! (block-matched sway amplitude and frequency), `street_stats` (frame times), `street_open`
//! and `street_close`. Local stdio only; no listening socket.
#[path = "../../../tools/mcp_stdio.rs"]
mod mcp_stdio;
use image::{GenericImageView, ImageEncoder, RgbImage, imageops::FilterType};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Instant;

type Failure = Box<dyn std::error::Error>;

const INSTRUCTIONS: &str = "Voxy street walk viewer. The first call starts the scene (~20 s); later calls reuse it. \
Coordinates are metres: the street runs along +Z from z=-29, carriageway centre x=0, pavements |x| 3.5-7, \
street trees at |x| 5.7-6.1 every ~12 m, eye height ~1.7. Use street_set to place the camera (eye+look, or \
place: Улица, Площадь, Бассейн...) and set wind/weather/hour; street_capture to see it (set return_frames \
for a strip of consecutive frames); street_motion to measure sway in pixels and Hz instead of judging it by \
eye; street_stats for frame-time percentiles and GPU time. After changing hour, exposure adapts for a few \
seconds: pass settle_ms 3000+.";

struct Session {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    key: String,
    log: PathBuf,
    out_dir: PathBuf,
}
impl Session {
    fn open(seed: Option<u64>, backend: &str) -> Result<Self, Failure> {
        let exe = std::env::current_exe()?;
        let dir = exe.parent().ok_or("no exe directory")?;
        let street = dir.join(if cfg!(windows) {
            "street_walk.exe"
        } else {
            "street_walk"
        });
        if !street.exists() {
            return Err(format!(
                "{} not built (cargo build --release --example street_walk -p voxy_app)",
                street.display()
            )
            .into());
        }
        let out_dir = std::env::temp_dir().join(format!("voxy-street-mcp-{}", std::process::id()));
        std::fs::create_dir_all(&out_dir)?;
        let log = out_dir.join("street_walk.log");
        let mut command = Command::new(&street);
        command.args(["--control-stdio", "--sdr", "--backend", backend]);
        if let Some(seed) = seed {
            command.args(["--seed", &seed.to_string()]);
        }
        let mut child = command
            .env("VOXY_STREET_GPU_TIMING", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(std::fs::File::create(&log)?)
            .spawn()?;
        let mut session = Self {
            stdin: child.stdin.take().ok_or("no child stdin")?,
            stdout: BufReader::new(child.stdout.take().ok_or("no child stdout")?),
            child,
            next_id: 0,
            key: format!("{seed:?}/{backend}"),
            log,
            out_dir,
        };
        let ready = session.read()?;
        if ready["event"] != "ready" {
            return Err(format!("unexpected startup message {ready}").into());
        }
        Ok(session)
    }
    fn read(&mut self) -> Result<Value, Failure> {
        let mut line = String::new();
        if self.stdout.read_line(&mut line)? == 0 {
            let log = std::fs::read_to_string(&self.log).unwrap_or_default();
            let tail: Vec<&str> = log.lines().rev().take(20).collect();
            return Err(format!(
                "street_walk exited:\n{}",
                tail.into_iter().rev().collect::<Vec<_>>().join("\n")
            )
            .into());
        }
        Ok(serde_json::from_str(&line)?)
    }
    fn send(&mut self, mut command: Value) -> Result<Value, Failure> {
        self.next_id += 1;
        command["id"] = self.next_id.into();
        writeln!(self.stdin, "{command}")?;
        self.stdin.flush()?;
        loop {
            let reply = self.read()?;
            if reply["id"] == self.next_id {
                return if reply["ok"] == true {
                    Ok(reply)
                } else {
                    Err(reply["error"].as_str().unwrap_or("command failed").into())
                };
            }
        }
    }
    /// Applies any camera, weather and time arguments present in `a`.
    fn apply(&mut self, a: &Value) -> Result<Value, Failure> {
        let mut state = json!({});
        if a["release"] == true {
            self.send(json!({"cmd": "release"}))?;
        }
        if a.get("eye").is_some() || a.get("place").is_some() {
            let mut camera = json!({"cmd": "camera"});
            for key in ["eye", "look", "place"] {
                if let Some(v) = a.get(key) {
                    camera[key] = v.clone();
                }
            }
            state["camera"] = self.send(camera)?;
        }
        const WEATHER: [&str; 8] = [
            "wind_speed",
            "wind_direction",
            "wind_gust",
            "coverage",
            "density",
            "rain",
            "snow",
            "wetness",
        ];
        if WEATHER.iter().any(|key| a.get(*key).is_some()) {
            let mut weather = json!({"cmd": "weather"});
            for key in WEATHER {
                if let Some(v) = a.get(key) {
                    weather[key] = v.clone();
                }
            }
            state["weather"] = self.send(weather)?;
        }
        if let Some(hour) = a.get("hour") {
            state["time"] = self.send(json!({"cmd": "time", "hour": hour}))?;
        }
        Ok(state)
    }
    /// Captures `frames` frames and returns them with the mean seconds between frames.
    fn capture(
        &mut self,
        a: &Value,
        frames: u64,
        interval_ms: u64,
    ) -> Result<(Vec<RgbImage>, f64, Value), Failure> {
        for entry in std::fs::read_dir(&self.out_dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|x| x == "png") {
                let _ = std::fs::remove_file(path);
            }
        }
        let settle_ms = a["settle_ms"].as_u64().unwrap_or(300);
        let started = Instant::now();
        let reply = self.send(json!({
            "cmd": "capture", "path": self.out_dir.join("cap.png"),
            "frames": frames, "interval_ms": interval_ms, "settle_ms": settle_ms
        }))?;
        let elapsed = started.elapsed().as_secs_f64() - settle_ms as f64 / 1000.;
        let crop = a["crop"].as_array().map(|c| {
            c.iter()
                .map(|v| v.as_u64().unwrap_or(0) as u32)
                .collect::<Vec<_>>()
        });
        let images = reply["files"]
            .as_array()
            .ok_or("capture returned no files")?
            .iter()
            .map(|p| load(Path::new(p.as_str().unwrap_or("")), crop.as_deref()))
            .collect::<Result<Vec<_>, _>>()?;
        if images.is_empty() {
            return Err("capture returned no frames".into());
        }
        // Wall time per frame includes readback, so it is longer than interval_ms.
        let step = (elapsed / images.len() as f64).max(interval_ms as f64 / 1000.);
        Ok((images, step, reply))
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, r#"{{"cmd":"quit"}}"#);
        let _ = self.stdin.flush();
        let _ = self.child.wait();
    }
}

#[derive(Default)]
struct Server {
    session: Option<Session>,
}
impl Server {
    fn session(&mut self, a: &Value) -> Result<&mut Session, Failure> {
        let backend = a["backend"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| std::env::var("VOXY_STREET_BACKEND").ok())
            .unwrap_or_else(|| "auto".into());
        let seed = a["seed"].as_u64();
        let key = format!("{seed:?}/{backend}");
        let restart = a["restart"] == true
            || self.session.as_ref().is_some_and(|s| {
                (a.get("seed").is_some() || a.get("backend").is_some()) && s.key != key
            });
        if restart {
            self.session = None;
        }
        if self.session.is_none() {
            self.session = Some(Session::open(seed, &backend)?);
        }
        Ok(self.session.as_mut().unwrap())
    }
    fn dispatch(&mut self, request: &Value) -> Value {
        let id = request["id"].clone();
        let result = match request["method"].as_str().unwrap_or("") {
            "initialize" => json!({
                "protocolVersion": request["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "voxy-street", "version": "0.2.0"},
                "instructions": INSTRUCTIONS
            }),
            "ping" => json!({}),
            "tools/list" => json!({"tools": tools()}),
            "tools/call" => match self.call(&request["params"]) {
                Ok(content) => json!({"content": content, "isError": false}),
                Err(e) => {
                    json!({"content": [{"type": "text", "text": e.to_string()}], "isError": true})
                }
            },
            other => {
                return json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": format!("unknown method {other}")}});
            }
        };
        json!({"jsonrpc": "2.0", "id": id, "result": result})
    }
    fn call(&mut self, params: &Value) -> Result<Vec<Value>, Failure> {
        let a = &params["arguments"];
        let text = |v: &Value| -> Result<Vec<Value>, Failure> {
            Ok(vec![
                json!({"type": "text", "text": serde_json::to_string_pretty(v)?}),
            ])
        };
        match params["name"].as_str().unwrap_or("") {
            "street_open" => {
                let started = Instant::now();
                let session = self.session(a)?;
                text(
                    &json!({"open": true, "session": session.key, "seconds": started.elapsed().as_secs_f64()}),
                )
            }
            "street_close" => {
                self.session = None;
                text(&json!({"open": false}))
            }
            "street_set" => {
                let state = self.session(a)?.apply(a)?;
                text(&state)
            }
            "street_stats" => {
                let session = self.session(a)?;
                let state = session.apply(a)?;
                let mut stats = session.send(
                    json!({"cmd": "stats", "seconds": a["seconds"].as_f64().unwrap_or(3.)}),
                )?;
                stats["applied"] = state;
                text(&stats)
            }
            "street_capture" => {
                let session = self.session(a)?;
                let state = session.apply(a)?;
                let frames = a["frames"].as_u64().unwrap_or(4).clamp(1, 16);
                let (images, step, reply) =
                    session.capture(a, frames, a["interval_ms"].as_u64().unwrap_or(100))?;
                let max_width = a["max_width"].as_u64().unwrap_or(960) as u32;
                let (w, h) = images[0].dimensions();
                let (heat, peak, moving) = heat_map(&images);
                let mut content = text(&json!({
                    "frames": images.len(), "size": [w, h], "seconds_between_frames": step,
                    "peak_change": peak, "moving_fraction": moving,
                    "eye": reply["eye"], "direction": reply["direction"], "applied": state
                }))?;
                if a["return_frames"] == true && images.len() > 1 {
                    // One strip, frames left to right, so motion reads at a glance.
                    let mut strip = RgbImage::new(w * images.len() as u32, h);
                    for (i, image) in images.iter().enumerate() {
                        image::imageops::replace(&mut strip, image, i as i64 * w as i64, 0);
                    }
                    content.push(json!({"type": "text", "text": "frames, left to right:"}));
                    content.push(png(
                        &strip,
                        max_width.max(w * images.len() as u32).min(2400),
                    )?);
                } else {
                    content.push(png(&images[0], max_width)?);
                }
                if images.len() > 1 {
                    content.push(json!({"type": "text", "text": "motion heat map (orange = changed between frames):"}));
                    content.push(png(&heat, max_width)?);
                }
                Ok(content)
            }
            "street_motion" => {
                let session = self.session(a)?;
                let state = session.apply(a)?;
                let frames = a["frames"].as_u64().unwrap_or(48).clamp(4, 64);
                let (images, step, _) =
                    session.capture(a, frames, a["interval_ms"].as_u64().unwrap_or(60))?;
                let block = a["block"].as_u64().unwrap_or(32).clamp(8, 128) as u32;
                let (mut report, map) = motion(&images, step, block);
                // Vertical field of view is 65 degrees unless the panel changed it.
                if let Some(distance) = a["distance_m"].as_f64() {
                    let metres_per_px = 2. * distance * (65_f64.to_radians() / 2.).tan() / 1080.;
                    report["metres_per_pixel"] = metres_per_px.into();
                    for key in [
                        "amplitude_px_median",
                        "amplitude_px_p90",
                        "amplitude_px_max",
                    ] {
                        if let Some(px) = report[key].as_f64() {
                            report[key.replace("_px_", "_m_")] = (px * metres_per_px).into();
                        }
                    }
                }
                report["applied"] = state;
                let mut content = text(&report)?;
                content.push(json!({"type": "text", "text": "first frame with per-block sway amplitude (red = large, blue = small, untinted = static or textureless):"}));
                content.push(png(&map, a["max_width"].as_u64().unwrap_or(960) as u32)?);
                Ok(content)
            }
            other => Err(format!("unknown tool {other}").into()),
        }
    }
}

fn tools() -> Value {
    let v3 = json!({"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3});
    let set = json!({
        "eye": v3, "look": v3,
        "place": {"type": "string", "description": "named viewpoint (substring): Улица, Площадь и метро, Бассейн и вилла, Сауна, Дзен-сад, Терраса..."},
        "release": {"type": "boolean", "description": "unpin the camera so it walks/falls again"},
        "wind_speed": {"type": "number", "description": "m/s"},
        "wind_direction": {"type": "number", "description": "degrees"},
        "wind_gust": {"type": "number", "minimum": 0, "maximum": 1},
        "coverage": {"type": "number"}, "density": {"type": "number"},
        "rain": {"type": "number"}, "snow": {"type": "number"}, "wetness": {"type": "number"},
        "hour": {"type": "number", "description": "local solar hour; pauses the day cycle there"}
    });
    let with = |extra: Value| {
        let mut properties = set.clone();
        for (k, v) in extra.as_object().unwrap() {
            properties[k] = v.clone();
        }
        json!({"type": "object", "properties": properties})
    };
    let shot = json!({
        "settle_ms": {"type": "integer", "description": "wait before the first frame (default 300)"},
        "crop": {"type": "array", "items": {"type": "integer"}, "minItems": 4, "maxItems": 4, "description": "x,y,w,h in the 1920x1080 frame"},
        "max_width": {"type": "integer", "minimum": 160, "maximum": 2400}
    });
    let merged = |a: &Value, b: Value| {
        let mut out = a.clone();
        for (k, v) in b.as_object().unwrap() {
            out[k] = v.clone();
        }
        out
    };
    json!([
        {"name": "street_open", "description": "Start (or restart) the street scene. Optional: seed, backend (auto|dx12|vulkan|metal), restart. Other tools open it implicitly.",
         "inputSchema": {"type": "object", "properties": {"seed": {"type": "integer"}, "backend": {"type": "string"}, "restart": {"type": "boolean"}}}},
        {"name": "street_close", "description": "Stop the street scene.", "inputSchema": {"type": "object", "properties": {}}},
        {"name": "street_set", "description": "Place the camera and set wind, weather and hour. The camera stays pinned where it is put. Every other tool accepts the same arguments and applies them first.",
         "inputSchema": with(json!({}))},
        {"name": "street_capture", "description": "Capture frames from the running scene: first frame (or a left-to-right strip with return_frames) plus a motion heat map.",
         "inputSchema": with(merged(&shot, json!({
            "frames": {"type": "integer", "minimum": 1, "maximum": 16},
            "interval_ms": {"type": "integer", "minimum": 0, "maximum": 2000},
            "return_frames": {"type": "boolean"}}))),
         "annotations": {"readOnlyHint": true}},
        {"name": "street_motion", "description": "Measure motion over a frame sequence by block matching: per-block sway amplitude (px, and metres with distance_m), dominant frequency in Hz, how coherent neighbouring blocks are (1 = everything moves together like jelly, 0 = independent), and an amplitude map. Keep the camera still; crop to the tree for speed.",
         "inputSchema": with(merged(&shot, json!({
            "frames": {"type": "integer", "minimum": 4, "maximum": 64},
            "interval_ms": {"type": "integer", "minimum": 0, "maximum": 1000},
            "block": {"type": "integer", "minimum": 8, "maximum": 128},
            "distance_m": {"type": "number", "description": "camera-to-subject distance for px to metres"}}))),
         "annotations": {"readOnlyHint": true}},
        {"name": "street_stats", "description": "Frame-time statistics over `seconds` (default 3) of the live window: mean FPS, p50/p95/p99/max ms, frames over the 120 Hz and 60 Hz budgets, GPU scene time.",
         "inputSchema": with(json!({"seconds": {"type": "number", "minimum": 0.2, "maximum": 60}})),
         "annotations": {"readOnlyHint": true}}
    ])
}

fn load(path: &Path, crop: Option<&[u32]>) -> Result<RgbImage, Failure> {
    let image = image::open(path)?;
    let image = match crop {
        Some([x, y, w, h]) => {
            let (iw, ih) = image.dimensions();
            let x = (*x).min(iw - 1);
            let y = (*y).min(ih - 1);
            image.crop_imm(x, y, (*w).min(iw - x).max(1), (*h).min(ih - y).max(1))
        }
        _ => image,
    };
    Ok(image.to_rgb8())
}

/// Max per-pixel change across the sequence over a dimmed first frame; returns the map,
/// the peak change and the fraction of pixels that moved.
fn heat_map(images: &[RgbImage]) -> (RgbImage, u8, f64) {
    let (w, h) = images[0].dimensions();
    let mut heat = RgbImage::new(w, h);
    let mut peak = 0u8;
    let mut moving = 0usize;
    for y in 0..h {
        for x in 0..w {
            let mut change = 0u8;
            for pair in images.windows(2) {
                let (p, q) = (pair[0].get_pixel(x, y).0, pair[1].get_pixel(x, y).0);
                change = change.max((0..3).map(|i| p[i].abs_diff(q[i])).max().unwrap_or(0));
            }
            peak = peak.max(change);
            moving += usize::from(change > 24);
            // Below ~16 is TAA jitter and specular sparkle, not motion.
            let v = (change.saturating_sub(16) as f32 / 48.).min(1.);
            let base = images[0].get_pixel(x, y).0;
            heat.put_pixel(
                x,
                y,
                image::Rgb([
                    (base[0] as f32 * 0.25 + 255. * v) as u8,
                    (base[1] as f32 * 0.25 + 160. * v * v) as u8,
                    (base[2] as f32 * 0.25) as u8,
                ]),
            );
        }
    }
    (heat, peak, moving as f64 / (w * h) as f64)
}

fn luma(image: &RgbImage) -> Vec<f32> {
    image
        .pixels()
        .map(|p| 0.299 * p.0[0] as f32 + 0.587 * p.0[1] as f32 + 0.114 * p.0[2] as f32)
        .collect()
}

/// Tracks each textured block through the sequence by SAD block matching (±`RANGE` px per
/// step) and summarises the resulting displacement tracks.
fn motion(images: &[RgbImage], step_seconds: f64, block: u32) -> (Value, RgbImage) {
    const RANGE: i32 = 4;
    let (w, h) = images[0].dimensions();
    let lumas: Vec<Vec<f32>> = images.iter().map(luma).collect();
    let at = |l: &[f32], x: i32, y: i32| l[(y as u32 * w + x as u32) as usize];
    let (bx, by) = ((w / block).max(1), (h / block).max(1));
    let mut tracks: Vec<Option<Vec<(f32, f32)>>> = Vec::new();
    for j in 0..by {
        for i in 0..bx {
            let (x0, y0) = ((i * block) as i32, (j * block) as i32);
            let (x1, y1) = (
                (x0 + block as i32).min(w as i32),
                (y0 + block as i32).min(h as i32),
            );
            // Textureless blocks (sky, plain walls) cannot be matched reliably.
            let mut sum = 0.;
            let mut sq = 0.;
            let n = ((x1 - x0) * (y1 - y0)) as f32;
            for y in y0..y1 {
                for x in x0..x1 {
                    let v = at(&lumas[0], x, y);
                    sum += v;
                    sq += v * v;
                }
            }
            if sq / n - (sum / n) * (sum / n) < 60. {
                tracks.push(None);
                continue;
            }
            // Match every frame against the first one, searching around the previous
            // estimate: errors cannot accumulate into drift, and large sway is followed.
            let mut track = vec![(0., 0.)];
            let mut position = (0i32, 0i32);
            for frame in &lumas[1..] {
                let mut best = (f32::INFINITY, position.0, position.1);
                for dy in position.1 - RANGE..=position.1 + RANGE {
                    for dx in position.0 - RANGE..=position.0 + RANGE {
                        if x0 + dx < 0 || y0 + dy < 0 || x1 + dx > w as i32 || y1 + dy > h as i32 {
                            continue;
                        }
                        let mut sad = 0.;
                        for y in (y0..y1).step_by(2) {
                            for x in (x0..x1).step_by(2) {
                                sad += (at(&lumas[0], x, y) - at(frame, x + dx, y + dy)).abs();
                            }
                        }
                        // Prefer the smaller shift on ties so static blocks stay at zero.
                        let sad = sad + 0.02 * n * ((dx * dx + dy * dy) as f32).sqrt();
                        if sad < best.0 {
                            best = (sad, dx, dy);
                        }
                    }
                }
                position = (best.1, best.2);
                track.push((position.0 as f32, position.1 as f32));
            }
            tracks.push(Some(track));
        }
    }
    let duration = step_seconds * (images.len() - 1) as f64;
    let mut amplitudes = Vec::new();
    let mut frequencies = Vec::new();
    let mut block_amplitude = vec![None; tracks.len()];
    for (index, track) in tracks.iter().enumerate() {
        let Some(track) = track else { continue };
        let n = track.len() as f32;
        let mean = track
            .iter()
            .fold((0., 0.), |m, p| (m.0 + p.0 / n, m.1 + p.1 / n));
        // Principal axis of the track: project onto the direction of largest spread.
        let (mut xx, mut xy, mut yy) = (0., 0., 0.);
        for p in track {
            let (dx, dy) = (p.0 - mean.0, p.1 - mean.1);
            xx += dx * dx;
            xy += dx * dy;
            yy += dy * dy;
        }
        let angle = 0.5 * (2. * xy).atan2(xx - yy);
        let series: Vec<f32> = track
            .iter()
            .map(|p| (p.0 - mean.0) * angle.cos() + (p.1 - mean.1) * angle.sin())
            .collect();
        let (lo, hi) = series
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |m, v| {
                (m.0.min(*v), m.1.max(*v))
            });
        let amplitude = (hi - lo) as f64;
        block_amplitude[index] = Some(amplitude);
        if amplitude < 1.5 {
            continue;
        }
        amplitudes.push(amplitude);
        let crossings = series
            .windows(2)
            .filter(|p| (p[0] < 0.) != (p[1] < 0.))
            .count();
        // Fewer than three crossings is under one period: the recording is too short.
        if duration > 0. && crossings >= 3 {
            frequencies.push(crossings as f64 / 2. / duration);
        }
    }
    // Coherence: mean correlation of horizontal motion between horizontally adjacent blocks.
    let mut coherence = Vec::new();
    for j in 0..by {
        for i in 0..bx.saturating_sub(1) {
            let (a, b) = (
                &tracks[(j * bx + i) as usize],
                &tracks[(j * bx + i + 1) as usize],
            );
            if let (Some(a), Some(b)) = (a, b) {
                let series = |t: &[(f32, f32)]| -> Vec<f32> {
                    let m = t.iter().map(|p| p.0).sum::<f32>() / t.len() as f32;
                    t.iter().map(|p| p.0 - m).collect()
                };
                let (a, b) = (series(a), series(b));
                let dot: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
                let norm = (a.iter().map(|x| x * x).sum::<f32>()
                    * b.iter().map(|x| x * x).sum::<f32>())
                .sqrt();
                if norm > 4. {
                    coherence.push((dot / norm) as f64);
                }
            }
        }
    }
    let sorted = |mut v: Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v
    };
    let quantile =
        |v: &[f64], q: f64| (!v.is_empty()).then(|| v[((v.len() - 1) as f64 * q).round() as usize]);
    let amplitudes = sorted(amplitudes);
    let frequencies = sorted(frequencies);
    let textured = tracks.iter().flatten().count();
    let peak = amplitudes.last().copied().unwrap_or(0.).max(1.);
    let mut map = images[0].clone();
    for j in 0..by {
        for i in 0..bx {
            let Some(amplitude) = block_amplitude[(j * bx + i) as usize] else {
                continue;
            };
            if amplitude < 1.5 {
                continue;
            }
            let t = (amplitude / peak).clamp(0., 1.) as f32;
            for y in j * block..((j + 1) * block).min(h) {
                for x in i * block..((i + 1) * block).min(w) {
                    let p = map.get_pixel_mut(x, y);
                    p.0 = [
                        (p.0[0] as f32 * 0.45 + 140. * t) as u8,
                        (p.0[1] as f32 * 0.45) as u8,
                        (p.0[2] as f32 * 0.45 + 140. * (1. - t)) as u8,
                    ];
                }
            }
        }
    }
    let report = json!({
        "frames": images.len(), "seconds": duration, "seconds_between_frames": step_seconds,
        "block_px": block, "blocks": tracks.len(), "textured_blocks": textured,
        "moving_blocks": amplitudes.len(),
        "moving_fraction_of_textured": if textured > 0 { amplitudes.len() as f64 / textured as f64 } else { 0. },
        "amplitude_px_median": quantile(&amplitudes, 0.5),
        "amplitude_px_p90": quantile(&amplitudes, 0.9),
        "amplitude_px_max": amplitudes.last(),
        "frequency_hz_median": quantile(&frequencies, 0.5),
        "frequency_hz_p10": quantile(&frequencies, 0.1),
        "frequency_hz_p90": quantile(&frequencies, 0.9),
        "neighbour_coherence_mean": (!coherence.is_empty()).then(|| coherence.iter().sum::<f64>() / coherence.len() as f64),
        "note": "frequency needs at least 1.5 periods in the recording (null otherwise): use more frames for slow sway"
    });
    (report, map)
}

fn png(image: &RgbImage, max_width: u32) -> Result<Value, Failure> {
    let scaled;
    let image = if image.width() > max_width {
        let h = (image.height() as u64 * max_width as u64 / image.width() as u64).max(1) as u32;
        scaled = image::imageops::resize(image, max_width, h, FilterType::Triangle);
        &scaled
    } else {
        image
    };
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes).write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(json!({"type": "image", "mimeType": "image/png", "data": base64(&bytes)}))
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= c.len() {
                s.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

fn main() -> Result<(), Failure> {
    let mut server = Server::default();
    mcp_stdio::serve(|request| server.dispatch(request))?;
    Ok(())
}
