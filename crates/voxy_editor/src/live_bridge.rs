//! Live editor window status receipt and on-demand framebuffer PNG capture bridge.

use crate::App;
use image::ImageEncoder as _;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_STATUS_BYTES: usize = 512 * 1024;
const STATUS_INTERVAL: Duration = Duration::from_millis(250);
const STALE_AFTER_SECS: f64 = 3.0;

#[derive(Debug, Default)]
pub(crate) struct LiveBridge {
    last_status_at: Option<Instant>,
    status_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveEditorStatus {
    pub version: u32,
    pub pid: u32,
    pub updated_at_unix_seconds: f64,
    pub frames: u64,
    pub playing: bool,
    pub simulation_ticks: u64,
    pub viewport_size: [u32; 2],
    pub color_format: String,
    pub project_path: Option<String>,
    pub scene_path: Option<String>,
    pub selected_object_id: Option<String>,
    pub camera: crate::EditorCamera,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_attribution: Option<crate::FrameDomainAttribution>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRequest {
    pub request_id: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureReceipt {
    pub request_id: String,
    pub capture: String,
    pub width: u32,
    pub height: u32,
    pub blake3: String,
    pub frames: u64,
    pub simulation_ticks: u64,
    pub png_path: String,
}

#[must_use]
pub fn live_dir_for_project(project_root: Option<&Path>) -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("VOXY_EDITOR_LIVE_DIR") {
        let path = PathBuf::from(dir);
        if !path.as_os_str().is_empty() {
            return Some(path);
        }
    }
    project_root.map(|root| root.join(".voxy-editor-live"))
}

pub fn valid_request_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn read_status(dir: &Path) -> Option<(LiveEditorStatus, bool)> {
    let bytes = fs::read(dir.join("status.json")).ok()?;
    let status: LiveEditorStatus = serde_json::from_slice(&bytes).ok()?;
    let now = unix_seconds();
    let fresh = now >= status.updated_at_unix_seconds
        && (now - status.updated_at_unix_seconds) <= STALE_AFTER_SECS;
    Some((status, fresh))
}

impl LiveBridge {
    pub(crate) fn cleanup(&mut self) {
        if let Some(path) = self.status_path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

impl App {
    pub(crate) fn poll_live_bridge(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.window.is_none() {
            return Ok(());
        }
        let project_root = self
            .build
            .project
            .is_some()
            .then(|| self.authoring.authoring_project.root().to_path_buf());
        let Some(dir) = live_dir_for_project(project_root.as_deref()) else {
            return Ok(());
        };
        let due = self
            .live_bridge
            .last_status_at
            .is_none_or(|t| t.elapsed() >= STATUS_INTERVAL);
        if due {
            self.write_live_status(&dir)?;
            self.live_bridge.last_status_at = Some(Instant::now());
        }
        self.service_live_capture_requests(&dir)?;
        Ok(())
    }

    fn write_live_status(&mut self, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let Some(graphics) = &self.graphics else {
            return Ok(());
        };
        let (viewport_size, color_format) = match (&self.window, &graphics.host) {
            (Some(win), Some(host)) => {
                let s = win.inner_size();
                (
                    [s.width.max(1), s.height.max(1)],
                    format!("{:?}", host.color_format()),
                )
            }
            _ => return Ok(()),
        };
        fs::create_dir_all(dir)?;
        let selected_object_id = self.instances.get(self.selected).and_then(|node| {
            self.instances
                .iter()
                .position(|n| n == node)
                .and_then(|idx| self.object_ids.get(idx))
                .map(|id| id.0.clone())
        });
        let status = LiveEditorStatus {
            version: 1,
            pid: std::process::id(),
            updated_at_unix_seconds: unix_seconds(),
            frames: self.frames,
            playing: self.play.playing.is_some(),
            simulation_ticks: self.play.simulation_ticks,
            viewport_size,
            color_format,
            project_path: self
                .build
                .project
                .as_ref()
                .map(|(p, _)| p.display().to_string()),
            scene_path: self
                .authoring
                .scene_path
                .as_ref()
                .map(|p| p.display().to_string()),
            selected_object_id,
            camera: self.camera.clone(),
            frame_attribution: Some(self.frame_attribution.snapshot()),
        };
        let status_path = dir.join("status.json");
        let encoded = serde_json::to_vec(&status)?;
        voxy_assets::save_atomic_file(&status_path, &encoded, MAX_STATUS_BYTES)
            .map_err(|e| format!("save live status: {e}"))?;
        self.live_bridge.status_path = Some(status_path);
        Ok(())
    }

    fn service_live_capture_requests(
        &mut self,
        dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let req_dir = dir.join("requests");
        let Ok(entries) = fs::read_dir(&req_dir) else {
            return Ok(());
        };
        let cap_dir = dir.join("captures");
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let Ok(raw) = fs::read(&path) else {
                continue;
            };
            let _ = fs::remove_file(&path);
            let Ok(req) = serde_json::from_slice::<CaptureRequest>(&raw) else {
                continue;
            };
            if !valid_request_id(&req.request_id) {
                continue;
            }
            fs::create_dir_all(&cap_dir)?;
            if let Ok((png, receipt)) = self.capture_live_png(&req, &cap_dir) {
                let png_path = cap_dir.join(format!("{}.png", req.request_id));
                let meta_path = cap_dir.join(format!("{}.meta.json", req.request_id));
                let _ = voxy_assets::save_atomic_file(&png_path, &png, 16 * 1024 * 1024);
                if let Ok(meta_bytes) = serde_json::to_vec(&receipt) {
                    let _ =
                        voxy_assets::save_atomic_file(&meta_path, &meta_bytes, MAX_STATUS_BYTES);
                }
            }
        }
        Ok(())
    }

    fn capture_live_png(
        &mut self,
        req: &CaptureRequest,
        cap_dir: &Path,
    ) -> Result<(Vec<u8>, CaptureReceipt), Box<dyn std::error::Error>> {
        let (device, queue, format, default_w, default_h, msaa4) = {
            let graphics = self.graphics.as_ref().ok_or("missing graphics")?;
            let host = graphics.host.as_ref().ok_or("missing host surface")?;
            let size = self.window.as_ref().ok_or("missing window")?.inner_size();
            (
                graphics.device.clone(),
                graphics.queue.clone(),
                host.color_format(),
                size.width.max(1),
                size.height.max(1),
                self.msaa4,
            )
        };
        let width = req.width.unwrap_or(default_w).clamp(64, 2048);
        let height = req.height.unwrap_or(default_h).clamp(64, 2048);
        let samples = if msaa4 { 4 } else { 1 };
        let color = create_target_tex(
            &device,
            width,
            height,
            samples,
            format,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let depth = create_target_tex(
            &device,
            width,
            height,
            samples,
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let resolve = msaa4.then(|| {
            create_target_tex(
                &device,
                width,
                height,
                1,
                format,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            )
        });
        let overlay_depth = msaa4.then(|| {
            create_target_tex(
                &device,
                width,
                height,
                1,
                wgpu::TextureFormat::Depth32Float,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            )
        });
        let color_view = color.create_view(&Default::default());
        let depth_view = depth.create_view(&Default::default());
        let resolve_view = resolve.as_ref().map(|t| t.create_view(&Default::default()));
        let overlay_depth_view = overlay_depth
            .as_ref()
            .map(|t| t.create_view(&Default::default()));
        self.draw_frame(Some(voxy_render::SceneViewTargets {
            color: &color_view,
            depth: &depth_view,
            xray_depth: None,
            resolve: resolve_view.as_ref(),
            overlay_depth: overlay_depth_view.as_ref().unwrap_or(&depth_view),
        }))?;
        let read_tex = resolve.as_ref().unwrap_or(&color);
        let png = encode_texture_to_png(&device, &queue, read_tex, format, [width, height])?;
        let receipt = CaptureReceipt {
            request_id: req.request_id.clone(),
            capture: "live_window".to_owned(),
            width,
            height,
            blake3: blake3::hash(&png).to_hex().to_string(),
            frames: self.frames,
            simulation_ticks: self.play.simulation_ticks,
            png_path: cap_dir
                .join(format!("{}.png", req.request_id))
                .display()
                .to_string(),
        };
        Ok((png, receipt))
    }
}

pub(crate) fn create_target_tex(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    sample_count: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("voxy_editor live/offscreen capture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

pub(crate) fn encode_texture_to_png(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    format: wgpu::TextureFormat,
    size: [u32; 2],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let [width, height] = size;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let unpadded = width * 4;
    let padded = unpadded.div_ceil(align) * align;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("voxy_editor png readback"),
        size: u64::from(padded) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        texture.size(),
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    rx.recv()??;
    let view = buffer.slice(..).get_mapped_range()?;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    let swap_rb = matches!(
        format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    );
    for row in 0..height as usize {
        let start = row * padded as usize;
        let slice = &view[start..start + unpadded as usize];
        if swap_rb {
            for px in slice.chunks_exact(4) {
                rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
            }
        } else {
            rgba.extend_from_slice(slice);
        }
    }
    drop(view);
    buffer.unmap();
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png).write_image(
        &rgba,
        width,
        height,
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(png)
}

fn unix_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}
