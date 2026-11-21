//! Offscreen SceneRenderer capture and live editor window capture bridge tools.

use super::state::{EngineState, HeadlessGpu, RenderedFrame, ToolOutcome};
use crate::live_bridge::{
    CaptureReceipt, CaptureRequest, create_target_tex, encode_texture_to_png, live_dir_for_project,
    read_status,
};
use crate::{App, EditorCamera, Graphics};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use voxy_mcp::{ContentBlock, RequestContext, ToolAnnotations, ToolDef, base64_encode};

pub(crate) fn tool_defs() -> Vec<ToolDef> {
    let ro = ToolAnnotations {
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(true),
        open_world_hint: Some(false),
    };
    vec![
        ToolDef::new(
            "viewport_render_offscreen",
            "Render the active session scene (including unsaved edits, glTF/GLB/OBJ/materials/lights/fog) offscreen via Voxy/Metal to PNG.",
            json!({
                "type": "object",
                "properties": {
                    "width": { "type": "integer", "minimum": 64, "maximum": 2048, "default": 512 },
                    "height": { "type": "integer", "minimum": 64, "maximum": 2048, "default": 512 },
                    "camera": { "type": "object" }
                },
                "additionalProperties": false
            }),
        )
        .with_annotations(ro.clone()),
        ToolDef::new(
            "editor_window_status",
            "Check whether a live voxy_editor window is running for the active project and inspect its live viewport state.",
            json!({
                "type": "object",
                "properties": { "live_dir": { "type": "string" } },
                "additionalProperties": false
            }),
        )
        .with_annotations(ro.clone()),
        ToolDef::new(
            "editor_window_capture",
            "Capture a live PNG frame directly from a running voxy_editor interactive/game window.",
            json!({
                "type": "object",
                "properties": {
                    "live_dir": { "type": "string" },
                    "width": { "type": "integer", "minimum": 64, "maximum": 2048 },
                    "height": { "type": "integer", "minimum": 64, "maximum": 2048 },
                    "timeout_ms": { "type": "integer", "minimum": 100, "maximum": 15000, "default": 3000 }
                },
                "additionalProperties": false
            }),
        )
        .with_annotations(ro),
    ]
}

pub(crate) fn dispatch(
    state: &mut EngineState,
    ctx: &RequestContext,
    name: &str,
    arguments: Value,
) -> Result<ToolOutcome, String> {
    match name {
        "viewport_render_offscreen" => viewport_render_offscreen(state, ctx, arguments),
        "editor_window_status" => editor_window_status(state, arguments),
        "editor_window_capture" => editor_window_capture(state, ctx, arguments),
        _ => Err(format!("unknown render tool: {name}")),
    }
}

fn ensure_headless_gpu(state: &mut EngineState) -> Result<&HeadlessGpu, String> {
    if state.gpu.is_none() {
        let instance = voxy_render::GraphicsOptions::default().create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
            .map_err(|e| format!("no headless GPU adapter available: {e}"))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
            .map_err(|e| format!("request headless GPU device failed: {e}"))?;
        state.gpu = Some(HeadlessGpu {
            instance,
            adapter,
            device,
            queue,
        });
    }
    Ok(state.gpu.as_ref().expect("gpu initialized"))
}

fn viewport_render_offscreen(
    state: &mut EngineState,
    ctx: &RequestContext,
    arguments: Value,
) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        #[serde(default = "default_dim")]
        width: u32,
        #[serde(default = "default_dim")]
        height: u32,
        camera: Option<EditorCamera>,
    }
    const fn default_dim() -> u32 {
        512
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let width = args.width.clamp(64, 2048);
    let height = args.height.clamp(64, 2048);
    ctx.report_progress(0.1, Some(1.0), Some("preparing headless app"));

    let (opened_descriptor, active_scene, scene_hash, session_cam, registry) = {
        let session = state.require_project()?;
        (
            session.project.descriptor_path.clone(),
            session.current_scene().clone(),
            session.scene_hash()?,
            args.camera.or_else(|| session.camera_override.clone()),
            state.registry.clone(),
        )
    };
    let _ = &ensure_headless_gpu(state)?.instance;
    let gpu = state.gpu.as_ref().expect("gpu initialized");
    let opened = crate::project::open_for_editor(&opened_descriptor, &registry)
        .map_err(|e| e.to_string())?;
    let mut app = App::from_open_project_with_model_package(&opened, registry, None)
        .map_err(|e| e.to_string())?;
    let render_res = (|| -> Result<Vec<u8>, String> {
        app.install_authored_scene(active_scene, None, None, None, true)
            .map_err(|e| e.to_string())?;
        if let Some(cam) = session_cam {
            app.camera = cam;
        } else if let Some((_, cam)) = app.scene.active_components::<EditorCamera>().next() {
            app.camera = cam.clone();
        }
        let renderer =
            voxy_render::SceneRenderer::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm);
        app.graphics = Some(
            Graphics::new(
                gpu.adapter.clone(),
                gpu.device.clone(),
                gpu.queue.clone(),
                renderer,
                None,
                None,
                (256 * 1024 * 1024, 256 * 1024 * 1024),
            )
            .map_err(|e| e.to_string())?,
        );
        ctx.report_progress(0.4, Some(1.0), Some("waiting for GPU model uploads"));
        wait_for_app_models(&mut app, ctx)?;
        ctx.report_progress(0.8, Some(1.0), Some("encoding offscreen frame"));
        render_offscreen_target_png(&mut app, width, height)
    })();
    let _ = app.stop_workers();
    let png = render_res?;
    ctx.report_progress(1.0, Some(1.0), Some("render complete"));

    let digest = blake3::hash(&png).to_hex().to_string();
    let b64 = base64_encode(&png);
    state.latest_render = Some(RenderedFrame { png });
    Ok(ToolOutcome::ok(json!({
        "capture": "offscreen",
        "width": width,
        "height": height,
        "blake3": digest,
        "sceneHash": scene_hash,
    }))
    .with_block(ContentBlock::Image {
        data: b64,
        mime_type: "image/png".to_owned(),
    })
    .with_block(ContentBlock::ResourceLink {
        uri: "voxy://renders/latest".to_owned(),
        name: "latest-render".to_owned(),
        title: Some("Latest Voxy Viewport Render".to_owned()),
        description: Some("PNG image rendered by voxy_mcp".to_owned()),
        mime_type: Some("image/png".to_owned()),
    })
    .with_updates(&["voxy://renders/latest"]))
}

fn wait_for_app_models(app: &mut App, ctx: &RequestContext) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut idle_ticks = 0u32;
    loop {
        if ctx.is_cancelled() {
            return Err("offscreen render cancelled".to_owned());
        }
        app.tick().map_err(|e| e.to_string())?;
        let required = app.required_gpu_assets();
        let all_ready = required.iter().all(|id| {
            app.graphics
                .as_ref()
                .is_some_and(|g| g.models.contains_key(id))
        });
        if all_ready && app.pending_model.is_none() {
            return Ok(());
        }
        let busy = app
            .imports
            .as_ref()
            .is_some_and(voxy_assets::AssetImportWorker::is_busy)
            || app.pending_model.is_some()
            || !app.reload.is_empty();
        idle_ticks = if busy { 0 } else { idle_ticks + 1 };
        if idle_ticks > 25 || Instant::now() >= deadline {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn render_offscreen_target_png(app: &mut App, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let (device, queue) = {
        let g = app.graphics.as_ref().ok_or("missing graphics")?;
        (g.device.clone(), g.queue.clone())
    };
    let color = create_target_tex(
        &device,
        width,
        height,
        1,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = create_target_tex(
        &device,
        width,
        height,
        1,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let color_view = color.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    app.draw_frame(Some(voxy_render::SceneViewTargets {
        color: &color_view,
        depth: &depth_view,
        xray_depth: None,
        resolve: None,
        overlay_depth: &depth_view,
    }))
    .map_err(|e| e.to_string())?;
    encode_texture_to_png(
        &device,
        &queue,
        &color,
        wgpu::TextureFormat::Rgba8Unorm,
        [width, height],
    )
    .map_err(|e| e.to_string())
}

fn resolve_live_dir(state: &EngineState, explicit: Option<&str>) -> Result<PathBuf, String> {
    if let Some(dir) = explicit {
        return Ok(PathBuf::from(dir));
    }
    let root = state.project.as_ref().map(|p| p.project.root.as_path());
    live_dir_for_project(root)
        .ok_or_else(|| "open a project or specify live_dir / VOXY_EDITOR_LIVE_DIR".to_owned())
}

fn editor_window_status(state: &EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        live_dir: Option<String>,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let dir = resolve_live_dir(state, args.live_dir.as_deref())?;
    Ok(ToolOutcome::ok(live_status_json(&dir)))
}

pub(crate) fn live_status_json(dir: &std::path::Path) -> Value {
    match read_status(dir) {
        Some((status, fresh)) => json!({
            "liveDir": dir.display().to_string(),
            "liveWindowConnected": fresh,
            "fresh": fresh,
            "status": status,
        }),
        None => json!({
            "liveDir": dir.display().to_string(),
            "liveWindowConnected": false,
            "fresh": false,
            "status": null,
        }),
    }
}

fn editor_window_capture(
    state: &mut EngineState,
    ctx: &RequestContext,
    arguments: Value,
) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        live_dir: Option<String>,
        width: Option<u32>,
        height: Option<u32>,
        #[serde(default = "default_timeout")]
        timeout_ms: u64,
    }
    const fn default_timeout() -> u64 {
        3000
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let dir = resolve_live_dir(state, args.live_dir.as_deref())?;
    let Some((_, fresh)) = read_status(&dir) else {
        return Err(format!(
            "no live voxy_editor window detected at {}",
            dir.display()
        ));
    };
    if !fresh {
        return Err(format!(
            "live editor status at {} is stale; window may be closed",
            dir.display()
        ));
    }
    state.capture_seq += 1;
    let request_id = format!("mcp-{}-{}", std::process::id(), state.capture_seq);
    let req_dir = dir.join("requests");
    let cap_dir = dir.join("captures");
    fs::create_dir_all(&req_dir).map_err(|e| e.to_string())?;
    let req_path = req_dir.join(format!("{request_id}.req.json"));
    let req_body = serde_json::to_vec(&CaptureRequest {
        request_id: request_id.clone(),
        width: args.width,
        height: args.height,
    })
    .map_err(|e| e.to_string())?;
    voxy_assets::save_atomic_file(&req_path, &req_body, 4096).map_err(|e| e.to_string())?;

    let png_path = cap_dir.join(format!("{request_id}.png"));
    let meta_path = cap_dir.join(format!("{request_id}.meta.json"));
    let deadline = Instant::now() + Duration::from_millis(args.timeout_ms.clamp(100, 15000));
    loop {
        if ctx.is_cancelled() {
            let _ = fs::remove_file(&req_path);
            return Err("live window capture cancelled".to_owned());
        }
        if meta_path.is_file() && png_path.is_file() {
            let png = fs::read(&png_path).map_err(|e| e.to_string())?;
            let meta_bytes = fs::read(&meta_path).map_err(|e| e.to_string())?;
            let receipt: CaptureReceipt =
                serde_json::from_slice(&meta_bytes).map_err(|e| e.to_string())?;
            let b64 = base64_encode(&png);
            state.latest_render = Some(RenderedFrame { png });
            return Ok(ToolOutcome::ok(json!({
                "capture": receipt.capture,
                "width": receipt.width,
                "height": receipt.height,
                "blake3": receipt.blake3,
                "frames": receipt.frames,
                "simulationTicks": receipt.simulation_ticks,
                "pngPath": receipt.png_path,
            }))
            .with_block(ContentBlock::Image {
                data: b64,
                mime_type: "image/png".to_owned(),
            })
            .with_updates(&["voxy://renders/latest", "voxy://editor/live-status"]));
        }
        if Instant::now() >= deadline {
            let _ = fs::remove_file(&req_path);
            return Err(format!(
                "timed out waiting for live window capture {request_id}"
            ));
        }
        std::thread::sleep(Duration::from_millis(15));
    }
}
