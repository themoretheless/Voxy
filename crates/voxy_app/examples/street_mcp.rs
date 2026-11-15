//! Stdio MCP server that lets an agent look at the street walk scene.
//!
//! `street_capture` launches `street_walk` (built next to this binary) with an exact camera,
//! weather and time, captures a short frame sequence and returns the frames as PNG images
//! (optionally cropped and downscaled) plus a motion heat map: the per-pixel maximum colour
//! change across the sequence, so what sways and by how much is visible in one image.
use image::{GenericImageView, ImageEncoder, RgbImage, imageops::FilterType};
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        // Notifications carry no id and get no response.
        let Some(id) = request.get("id").cloned() else {
            continue;
        };
        let result = match request["method"].as_str().unwrap_or("") {
            "initialize" => json!({
                "protocolVersion": request["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "voxy-street", "version": "0.1.0"}
            }),
            "ping" => json!({}),
            "tools/list" => json!({"tools": [tool_schema()]}),
            "tools/call" => match call(&request["params"]) {
                Ok(content) => json!({"content": content, "isError": false}),
                Err(e) => {
                    json!({"content": [{"type": "text", "text": e.to_string()}], "isError": true})
                }
            },
            other => {
                let error = json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": format!("unknown method {other}")}});
                let _ = writeln!(out, "{error}");
                let _ = out.flush();
                continue;
            }
        };
        let _ = writeln!(
            out,
            "{}",
            json!({"jsonrpc": "2.0", "id": id, "result": result})
        );
        let _ = out.flush();
    }
}

fn tool_schema() -> Value {
    let v3 = json!({"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3});
    json!({
        "name": "street_capture",
        "description": "Render the original street walk scene from an exact camera and return a frame sequence plus a motion heat map (max per-pixel change across frames; bright = moving). Street runs along +Z, carriageway centre x=0, pavements |x|~3.5-7, trees at |x|~5.7-6.1, eye height ~1.7 m. Defaults: 4 frames 150 ms apart, wind from the scene panel defaults.",
        "inputSchema": {"type": "object", "properties": {
            "eye": v3, "look": v3,
            "frames": {"type": "integer", "minimum": 1, "maximum": 12},
            "interval_ms": {"type": "integer", "minimum": 16, "maximum": 2000},
            "wind_speed": {"type": "number", "description": "m/s"},
            "wind_direction": {"type": "number", "description": "degrees"},
            "wind_gust": {"type": "number", "minimum": 0, "maximum": 1},
            "hour": {"type": "number", "description": "local solar hour; enables the day cycle"},
            "crop": {"type": "array", "items": {"type": "integer"}, "minItems": 4, "maxItems": 4, "description": "x,y,w,h in the 1920x1080 frame"},
            "max_width": {"type": "integer", "minimum": 160, "maximum": 1920, "description": "downscale returned images to this width (default 960)"},
            "return_frames": {"type": "boolean", "description": "include every frame (default: first frame + heat map only)"},
            "seed": {"type": "integer"}
        }}
    })
}

fn call(params: &Value) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    if params["name"] != "street_capture" {
        return Err(format!("unknown tool {}", params["name"]).into());
    }
    let a = &params["arguments"];
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
    for entry in std::fs::read_dir(&out_dir)? {
        let _ = std::fs::remove_file(entry?.path());
    }
    let frames = a["frames"].as_u64().unwrap_or(4).clamp(1, 12);
    let mut cmd = Command::new(&street);
    cmd.args(["--sdr", "--snapshot"])
        .arg(out_dir.join("cap.png"));
    cmd.args(["--capture-frames", &frames.to_string()]);
    cmd.args([
        "--capture-interval-ms",
        &a["interval_ms"].as_u64().unwrap_or(150).to_string(),
    ]);
    let vec3 = |v: &Value| -> Option<String> {
        let v = v.as_array()?;
        (v.len() == 3).then(|| {
            v.iter()
                .map(|c| c.as_f64().unwrap_or(0.).to_string())
                .collect::<Vec<_>>()
                .join(",")
        })
    };
    if let Some(eye) = vec3(&a["eye"]) {
        cmd.args(["--eye", &eye]);
    }
    if let Some(look) = vec3(&a["look"]) {
        cmd.args(["--look", &look]);
    }
    for (key, flag) in [
        ("wind_speed", "--wind-speed"),
        ("wind_direction", "--wind-direction"),
        ("wind_gust", "--wind-gust"),
        ("hour", "--hour"),
        ("seed", "--seed"),
    ] {
        if let Some(v) = a[key].as_f64() {
            cmd.args([flag, &v.to_string()]);
        }
    }
    let output = cmd.output()?;
    let log = String::from_utf8_lossy(&output.stderr);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&out_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "png"))
        .collect();
    paths.sort();
    if paths.is_empty() {
        let tail: Vec<&str> = log.lines().rev().take(25).collect();
        return Err(format!(
            "no frames captured (exit {:?}):\n{}",
            output.status.code(),
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        )
        .into());
    }
    let crop = a["crop"].as_array().map(|c| {
        c.iter()
            .map(|v| v.as_u64().unwrap_or(0) as u32)
            .collect::<Vec<_>>()
    });
    let max_width = a["max_width"].as_u64().unwrap_or(960) as u32;
    let images: Vec<RgbImage> = paths
        .iter()
        .map(|p| load(p, crop.as_deref()))
        .collect::<Result<_, _>>()?;
    let (w, h) = images[0].dimensions();
    let mut heat = RgbImage::new(w, h);
    let mut peak = 0u8;
    let mut moving = 0usize;
    for y in 0..h {
        for x in 0..w {
            let mut change = 0u8;
            for pair in images.windows(2) {
                let (p, q) = (pair[0].get_pixel(x, y).0, pair[1].get_pixel(x, y).0);
                let d = (0..3).map(|i| p[i].abs_diff(q[i])).max().unwrap_or(0);
                change = change.max(d);
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
    let meta = json!({
        "frames": paths.len(), "size": [w, h], "peak_change": peak,
        "moving_fraction": moving as f64 / (w * h) as f64,
        "camera": log.lines().find(|l| l.contains("grounded spawn")).unwrap_or(""),
        "files": paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>()
    });
    let mut content = vec![json!({"type": "text", "text": serde_json::to_string_pretty(&meta)?})];
    let all = a["return_frames"].as_bool().unwrap_or(false);
    if all && images.len() > 1 {
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
        content.push(
            json!({"type": "text", "text": "motion heat map (orange = changed between frames):"}),
        );
        content.push(png(&heat, max_width)?);
    }
    Ok(content)
}

fn load(path: &Path, crop: Option<&[u32]>) -> Result<RgbImage, Box<dyn std::error::Error>> {
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

fn png(image: &RgbImage, max_width: u32) -> Result<Value, Box<dyn std::error::Error>> {
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
