//! MCP server that lets an agent look at, drive and measure the street walk scene.
//!
//! Keeps one warm `street_walk --control-stdio` child (built next to this binary),
//! and supports both stdio (default) and `--http 127.0.0.1:<port>` Streamable HTTP.
use image::{GenericImageView, ImageEncoder, RgbImage, imageops::FilterType};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Arc;
use std::time::Instant;
use voxy_mcp::{
    CallToolResult, CompleteParams, CompletionValues, ContentBlock, GetPromptResult,
    MutableMcpHandler, MutexMcpHandler, PromptArgument, PromptDef, PromptMessage, RequestContext,
    ResourceContents, ResourceDef, ResourceTemplateDef, RpcError, ServerConfig, ToolAnnotations,
    ToolDef, base64_encode, parse_transport_args, serve,
};

const WAYPOINTS: [&str; 9] = [
    "Улица",
    "Площадь и метро",
    "Бассейн и вилла",
    "Сауна и оранжерея",
    "Дзен-сад и бочче",
    "Терраса на крыше",
    "Обсерватория",
    "Винный грот",
    "Ателье художника",
];

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
        if std::env::var_os("VOXY_STREET_PROBE_CACHE").is_none() {
            let probe_dir = std::env::temp_dir().join("voxy-street-probe-cache");
            std::fs::create_dir_all(&probe_dir)?;
            command.env("VOXY_STREET_PROBE_CACHE", probe_dir.join("street"));
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
    last_state: Value,
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

    fn call_stats(
        &mut self,
        ctx: &RequestContext,
        a: &Value,
    ) -> Result<Vec<ContentBlock>, Failure> {
        ctx.report_progress(0.2, Some(1.0), Some("applying street state"));
        let (state, mut stats) = {
            let session = self.session(a)?;
            let state = session.apply(a)?;
            let tour = a["tour"].as_bool().unwrap_or(false);
            if tour {
                session.send(json!({"cmd": "tour", "time": 0}))?;
            }
            ctx.report_progress(0.6, Some(1.0), Some("measuring live frame times"));
            let stats = session
                .send(json!({"cmd": "stats", "seconds": a["seconds"].as_f64().unwrap_or(3.)}))?;
            if tour {
                session.send(json!({"cmd": "tour", "on": false}))?;
            }
            (state, stats)
        };
        self.last_state = state.clone();
        ctx.report_progress(1.0, Some(1.0), Some("stats ready"));
        stats["applied"] = state;
        text_content(&stats)
    }

    fn call_capture(
        &mut self,
        ctx: &RequestContext,
        a: &Value,
    ) -> Result<Vec<ContentBlock>, Failure> {
        ctx.report_progress(0.2, Some(1.0), Some("capturing frames"));
        let frames = a["frames"].as_u64().unwrap_or(4).clamp(1, 16);
        let interval = a["interval_ms"].as_u64().unwrap_or(100);
        let (state, images, step, reply) = {
            let session = self.session(a)?;
            let state = session.apply(a)?;
            let (images, step, reply) = session.capture(a, frames, interval)?;
            (state, images, step, reply)
        };
        self.last_state = state.clone();
        ctx.report_progress(0.8, Some(1.0), Some("encoding PNG capture"));
        let max_width = a["max_width"].as_u64().unwrap_or(960) as u32;
        let (w, h) = images[0].dimensions();
        let (heat, peak, moving) = heat_map(&images);
        let mut content = text_content(&json!({
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
            content.push(ContentBlock::Text {
                text: "frames, left to right:".to_owned(),
            });
            content.push(png(
                &strip,
                max_width.max(w * images.len() as u32).min(2400),
            )?);
        } else {
            content.push(png(&images[0], max_width)?);
        }
        if images.len() > 1 {
            content.push(ContentBlock::Text {
                text: "motion heat map (orange = changed between frames):".to_owned(),
            });
            content.push(png(&heat, max_width)?);
        }
        ctx.report_progress(1.0, Some(1.0), Some("capture ready"));
        Ok(content)
    }

    fn call_motion(
        &mut self,
        ctx: &RequestContext,
        a: &Value,
    ) -> Result<Vec<ContentBlock>, Failure> {
        ctx.report_progress(0.2, Some(1.0), Some("capturing motion sequence"));
        let frames = a["frames"].as_u64().unwrap_or(48).clamp(4, 64);
        let interval = a["interval_ms"].as_u64().unwrap_or(60);
        let (state, images, step) = {
            let session = self.session(a)?;
            let state = session.apply(a)?;
            let (images, step, _) = session.capture(a, frames, interval)?;
            (state, images, step)
        };
        self.last_state = state.clone();
        ctx.report_progress(0.7, Some(1.0), Some("analyzing block motion"));
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
        let mut content = text_content(&report)?;
        content.push(ContentBlock::Text {
            text: "first frame with per-block sway amplitude (red = large, blue = small, untinted = static or textureless):".to_owned(),
        });
        content.push(png(&map, a["max_width"].as_u64().unwrap_or(960) as u32)?);
        ctx.report_progress(1.0, Some(1.0), Some("motion analysis complete"));
        Ok(content)
    }
}

impl MutableMcpHandler for Server {
    fn config(&self) -> ServerConfig {
        ServerConfig::new("voxy-street", "0.2.0").with_instructions(INSTRUCTIONS)
    }

    fn list_tools(&self) -> Vec<ToolDef> {
        tools()
    }

    fn call_tool(
        &mut self,
        ctx: &RequestContext,
        name: &str,
        arguments: Value,
    ) -> Result<CallToolResult, String> {
        let a = &arguments;
        let blocks = match name {
            "street_open" => {
                let started = Instant::now();
                let session = self.session(a).map_err(|e| e.to_string())?;
                text_content(&json!({
                    "open": true,
                    "session": session.key,
                    "seconds": started.elapsed().as_secs_f64()
                }))
            }
            "street_close" => {
                self.session = None;
                text_content(&json!({"open": false}))
            }
            "street_set" => {
                let applied = self
                    .session(a)
                    .and_then(|s| s.apply(a))
                    .map_err(|e| e.to_string())?;
                self.last_state = applied.clone();
                text_content(&applied)
            }
            "street_stats" => self.call_stats(ctx, a),
            "street_capture" => self.call_capture(ctx, a),
            "street_motion" => self.call_motion(ctx, a),
            other => return Err(format!("unknown tool {other}")),
        }
        .map_err(|e| e.to_string())?;
        ctx.notify_resource_updated("voxy://street/state");
        Ok(CallToolResult {
            content: blocks,
            structured_content: None,
            is_error: false,
        })
    }

    fn list_resources(&mut self) -> Vec<ResourceDef> {
        vec![
            ResourceDef {
                uri: "voxy://street/waypoints".into(),
                name: "street-waypoints".into(),
                title: None,
                description: Some("Named viewpoints around the generated street and villa.".into()),
                mime_type: Some("application/json".into()),
                size: None,
            },
            ResourceDef {
                uri: "voxy://street/state".into(),
                name: "street-state".into(),
                title: None,
                description: Some(
                    "Live street_walk session open status and last applied state.".into(),
                ),
                mime_type: Some("application/json".into()),
                size: None,
            },
        ]
    }

    fn list_resource_templates(&self) -> Vec<ResourceTemplateDef> {
        vec![ResourceTemplateDef {
            uri_template: "voxy://street/waypoint/{place}".into(),
            name: "street-waypoint".into(),
            title: None,
            description: Some("Inspect matching street viewpoint name.".into()),
            mime_type: Some("application/json".into()),
        }]
    }

    fn read_resource(
        &mut self,
        _ctx: &RequestContext,
        uri: &str,
    ) -> Result<Vec<ResourceContents>, RpcError> {
        match uri {
            "voxy://street/waypoints" => Ok(vec![ResourceContents::json(
                uri,
                &json!({ "waypoints": WAYPOINTS }),
            )]),
            "voxy://street/state" => Ok(vec![ResourceContents::json(
                uri,
                &json!({
                    "open": self.session.is_some(),
                    "session": self.session.as_ref().map(|s| &s.key),
                    "lastApplied": self.last_state,
                }),
            )]),
            _ => {
                if let Some(place) = uri.strip_prefix("voxy://street/waypoint/") {
                    let matched: Vec<&&str> = WAYPOINTS
                        .iter()
                        .filter(|w| w.to_lowercase().contains(&place.to_lowercase()))
                        .collect();
                    return Ok(vec![ResourceContents::json(
                        uri,
                        &json!({ "query": place, "matches": matched }),
                    )]);
                }
                Err(RpcError::resource_not_found(uri))
            }
        }
    }

    fn list_prompts(&self) -> Vec<PromptDef> {
        vec![
            PromptDef {
                name: "inspect_street_fps".into(),
                title: Some("Inspect Street FPS & Frame Pacing".into()),
                description: Some("Measure FPS percentiles across key street viewpoints.".into()),
                arguments: vec![PromptArgument {
                    name: "place".into(),
                    description: Some("Viewpoint name".into()),
                    required: false,
                }],
            },
            PromptDef {
                name: "inspect_foliage_motion".into(),
                title: Some("Measure Foliage Wind Sway".into()),
                description: Some(
                    "Capture motion heat map and Hz/amplitude across wind speeds.".into(),
                ),
                arguments: vec![],
            },
        ]
    }

    fn get_prompt(
        &mut self,
        _ctx: &RequestContext,
        name: &str,
        arguments: Value,
    ) -> Result<GetPromptResult, RpcError> {
        let place = arguments
            .get("place")
            .and_then(Value::as_str)
            .unwrap_or("Улица");
        let text = match name {
            "inspect_street_fps" => format!(
                "Call `street_stats` with place `{place}` and `seconds: 3` (and optionally `tour: true`), then report mean FPS, p50/p99 ms, `monitor_refresh_hz`, `over_120hz_frames`, and `gpu_scene_mean_ms`."
            ),
            "inspect_foliage_motion" => {
                "Call `street_motion` at eye `[-3.0, 1.7, -9.0]`, look `[-5.9, 3.6, -5.0]` for wind speeds 0, 8, 15 m/s and compare amplitude_px_p90 and frequency_hz_median.".to_owned()
            }
            _ => return Err(RpcError::invalid_params(format!("unknown prompt: {name}"))),
        };
        Ok(GetPromptResult {
            description: Some("Street inspection prompt".into()),
            messages: vec![PromptMessage {
                role: "user".into(),
                content: ContentBlock::Text { text },
            }],
        })
    }

    fn complete(
        &mut self,
        _ctx: &RequestContext,
        params: &CompleteParams,
    ) -> Result<CompletionValues, RpcError> {
        if params.argument.name == "place" {
            return Ok(CompletionValues::from_candidates(
                WAYPOINTS.into_iter().map(str::to_owned),
                &params.argument.value,
            ));
        }
        Ok(CompletionValues::default())
    }
}

fn text_content(v: &Value) -> Result<Vec<ContentBlock>, Failure> {
    Ok(vec![ContentBlock::Text {
        text: serde_json::to_string_pretty(v)?,
    }])
}

fn tools() -> Vec<ToolDef> {
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
    let ro = ToolAnnotations {
        read_only_hint: Some(true),
        destructive_hint: None,
        idempotent_hint: None,
        open_world_hint: None,
    };
    vec![
        ToolDef::new(
            "street_open",
            "Start (or restart) the street scene. Optional: seed, backend (auto|dx12|vulkan|metal), restart. Other tools open it implicitly.",
            json!({"type": "object", "properties": {"seed": {"type": "integer"}, "backend": {"type": "string"}, "restart": {"type": "boolean"}}}),
        ),
        ToolDef::new(
            "street_close",
            "Stop the street scene.",
            json!({"type": "object", "properties": {}}),
        ),
        ToolDef::new(
            "street_set",
            "Place the camera and set wind, weather and hour. The camera stays pinned where it is put. Every other tool accepts the same arguments and applies them first.",
            with(json!({})),
        ),
        ToolDef::new(
            "street_capture",
            "Capture frames from the running scene: first frame (or a left-to-right strip with return_frames) plus a motion heat map.",
            with(merged(&shot, json!({
                "frames": {"type": "integer", "minimum": 1, "maximum": 16},
                "interval_ms": {"type": "integer", "minimum": 0, "maximum": 2000},
                "return_frames": {"type": "boolean"}
            }))),
        ).with_annotations(ro.clone()),
        ToolDef::new(
            "street_motion",
            "Measure motion over a frame sequence by block matching: per-block sway amplitude (px, and metres with distance_m), dominant frequency in Hz, how coherent neighbouring blocks are (1 = everything moves together like jelly, 0 = independent), and an amplitude map. Keep the camera still; crop to the tree for speed.",
            with(merged(&shot, json!({
                "frames": {"type": "integer", "minimum": 4, "maximum": 64},
                "interval_ms": {"type": "integer", "minimum": 0, "maximum": 1000},
                "block": {"type": "integer", "minimum": 8, "maximum": 128},
                "distance_m": {"type": "number", "description": "camera-to-subject distance for px to metres"}
            }))),
        ).with_annotations(ro.clone()),
        ToolDef::new(
            "street_stats",
            "Frame-time statistics over `seconds` (default 3) of the live window: mean FPS, p50/p95/p99/max ms, frames over the 120 Hz and 60 Hz budgets, GPU scene time, and the eight slowest frames with when and where they happened. `tour` flies the camera through the whole street while measuring (use 45 s for one pass) to find hitches in motion.",
            with(json!({"seconds": {"type": "number", "minimum": 0.2, "maximum": 60}, "tour": {"type": "boolean"}})),
        ).with_annotations(ro),
    ]
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

type BlockTrack = Option<Vec<(f32, f32)>>;

fn luma_at(luma: &[f32], w: u32, x: i32, y: i32) -> f32 {
    luma[(y as u32 * w + x as u32) as usize]
}

fn track_single_block(
    lumas: &[Vec<f32>],
    (w, h): (u32, u32),
    bounds: (i32, i32, i32, i32),
) -> BlockTrack {
    const RANGE: i32 = 4;
    let (x0, y0, x1, y1) = bounds;
    let n = ((x1 - x0) * (y1 - y0)) as f32;
    let (mut sum, mut sq) = (0., 0.);
    for y in y0..y1 {
        for x in x0..x1 {
            let v = luma_at(&lumas[0], w, x, y);
            sum += v;
            sq += v * v;
        }
    }
    // Textureless blocks (sky, plain walls) cannot be matched reliably.
    if sq / n - (sum / n) * (sum / n) < 60. {
        return None;
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
                        sad +=
                            (luma_at(&lumas[0], w, x, y) - luma_at(frame, w, x + dx, y + dy)).abs();
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
    Some(track)
}

fn summarize_tracks(
    tracks: &[BlockTrack],
    duration: f64,
) -> (Vec<f64>, Vec<f64>, Vec<Option<f64>>) {
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
        let amplitude = f64::from(hi - lo);
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
    amplitudes.sort_by(f64::total_cmp);
    frequencies.sort_by(f64::total_cmp);
    (amplitudes, frequencies, block_amplitude)
}

fn compute_neighbour_coherence(tracks: &[BlockTrack], bx: u32, by: u32) -> Option<f64> {
    let mut coherence = Vec::new();
    for j in 0..by {
        for i in 0..bx.saturating_sub(1) {
            let (a, b) = (
                &tracks[(j * bx + i) as usize],
                &tracks[(j * bx + i + 1) as usize],
            );
            if let (Some(a), Some(b)) = (a, b) {
                let demean = |t: &[(f32, f32)]| -> Vec<f32> {
                    let m = t.iter().map(|p| p.0).sum::<f32>() / t.len() as f32;
                    t.iter().map(|p| p.0 - m).collect()
                };
                let (a, b) = (demean(a), demean(b));
                let dot: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
                let norm = (a.iter().map(|x| x * x).sum::<f32>()
                    * b.iter().map(|x| x * x).sum::<f32>())
                .sqrt();
                if norm > 4. {
                    coherence.push(f64::from(dot / norm));
                }
            }
        }
    }
    (!coherence.is_empty()).then(|| coherence.iter().sum::<f64>() / coherence.len() as f64)
}

fn render_motion_heatmap(
    base: &RgbImage,
    block_amplitude: &[Option<f64>],
    (bx, by): (u32, u32),
    block: u32,
    peak: f64,
) -> RgbImage {
    let (w, h) = base.dimensions();
    let mut map = base.clone();
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
    map
}

/// Tracks each textured block through the sequence by SAD block matching (±4 px per
/// step) and summarises the resulting displacement tracks.
fn motion(images: &[RgbImage], step_seconds: f64, block: u32) -> (Value, RgbImage) {
    let (w, h) = images[0].dimensions();
    let lumas: Vec<Vec<f32>> = images.iter().map(luma).collect();
    let (bx, by) = ((w / block).max(1), (h / block).max(1));
    let mut tracks = Vec::with_capacity((bx * by) as usize);
    for j in 0..by {
        for i in 0..bx {
            let (x0, y0) = ((i * block) as i32, (j * block) as i32);
            let (x1, y1) = (
                (x0 + block as i32).min(w as i32),
                (y0 + block as i32).min(h as i32),
            );
            tracks.push(track_single_block(&lumas, (w, h), (x0, y0, x1, y1)));
        }
    }
    let duration = step_seconds * (images.len() - 1) as f64;
    let (amplitudes, frequencies, block_amplitude) = summarize_tracks(&tracks, duration);
    let coherence = compute_neighbour_coherence(&tracks, bx, by);
    let quantile =
        |v: &[f64], q: f64| (!v.is_empty()).then(|| v[((v.len() - 1) as f64 * q).round() as usize]);
    let textured = tracks.iter().flatten().count();
    let peak = amplitudes.last().copied().unwrap_or(0.).max(1.);
    let map = render_motion_heatmap(&images[0], &block_amplitude, (bx, by), block, peak);
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
        "neighbour_coherence_mean": coherence,
        "note": "frequency needs at least 1.5 periods in the recording (null otherwise): use more frames for slow sway"
    });
    (report, map)
}

fn png(image: &RgbImage, max_width: u32) -> Result<ContentBlock, Failure> {
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
    Ok(ContentBlock::Image {
        mime_type: "image/png".to_owned(),
        data: base64_encode(&bytes),
    })
}

fn main() -> Result<(), Failure> {
    let (mode, rest) = parse_transport_args(std::env::args().skip(1))?;
    if !rest.is_empty() {
        return Err(format!("unexpected arguments: {}", rest.join(" ")).into());
    }
    let handler = Arc::new(MutexMcpHandler::new(Server::default()));
    serve(handler, mode)?;
    Ok(())
}
