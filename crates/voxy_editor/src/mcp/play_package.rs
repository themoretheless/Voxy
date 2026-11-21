//! HeadlessGame Play simulation (single-thread actor), package export, and MCP client calls.

use super::state::{EngineState, ToolOutcome};
use crate::{GameHost, HeadlessGame, ModelSource};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use voxy_mcp::{CancelToken, RequestContext, ToolDef};
use voxy_scene::ComponentRegistry;
use winit::keyboard::KeyCode;

enum PlayCommand {
    Step {
        ticks: u32,
        dt: f64,
        delay_ms: Option<u64>,
        cancel: CancelToken,
        progress_tx: mpsc::Sender<u32>,
        reply: mpsc::Sender<Result<Value, String>>,
    },
    Input {
        key: Option<(KeyCode, bool)>,
        pointer: Option<([f32; 2], Option<[f32; 2]>, bool)>,
        focused: Option<bool>,
        reply: mpsc::Sender<Result<Value, String>>,
    },
    Snapshot {
        animation_node_id: Option<String>,
        reply: mpsc::Sender<Result<Value, String>>,
    },
    TransitionScene {
        scene_path: String,
        reply: mpsc::Sender<Result<Value, String>>,
    },
    Stop {
        reply: mpsc::Sender<Result<Value, String>>,
    },
}

#[derive(Debug)]
pub(crate) struct PlayWorker {
    tx: Mutex<mpsc::Sender<PlayCommand>>,
    join: Mutex<Option<JoinHandle<()>>>,
    initial_summary: Value,
}

impl PlayWorker {
    pub(crate) fn spawn(
        source: PathBuf,
        from_package: bool,
        registry: Arc<ComponentRegistry>,
    ) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel();
        let (init_tx, init_rx) = mpsc::channel();
        let join = thread::spawn(move || {
            let host = GameHost::new((*registry).clone());
            let open_res = if from_package {
                HeadlessGame::open_package(&source, host)
            } else {
                HeadlessGame::open_project(&source, host)
            };
            let mut game = match open_res {
                Ok(g) => g,
                Err(e) => {
                    let _ = init_tx.send(Err(e.to_string()));
                    return;
                }
            };
            let ticks = game.fixed_ticks();
            let objects = match game.snapshot() {
                Ok(s) => s.objects.len(),
                Err(e) => {
                    let _ = game.stop();
                    let _ = init_tx.send(Err(e.to_string()));
                    return;
                }
            };
            let summary = json!({
                "running": true,
                "sourceKind": if from_package { "package" } else { "project" },
                "sourcePath": source.display().to_string(),
                "activeScenePath": game.active_scene_path(),
                "fixedTicks": ticks,
                "objectCount": objects,
            });
            let _ = init_tx.send(Ok(summary));
            run_play_actor(&mut game, &source, rx);
            let _ = game.stop();
        });
        let initial_summary = init_rx
            .recv()
            .map_err(|_| "play worker exited before initialization".to_owned())??;
        Ok(Self {
            tx: Mutex::new(tx),
            join: Mutex::new(Some(join)),
            initial_summary,
        })
    }

    pub(crate) fn step(
        &self,
        ctx: &RequestContext,
        ticks: u32,
        dt: f64,
        delay_ms: Option<u64>,
    ) -> Result<Value, String> {
        let (progress_tx, progress_rx) = mpsc::channel();
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(PlayCommand::Step {
            ticks,
            dt,
            delay_ms,
            cancel: ctx.cancel_token().clone(),
            progress_tx,
            reply: reply_tx,
        })?;
        loop {
            while let Ok(done) = progress_rx.try_recv() {
                ctx.report_progress(
                    f64::from(done),
                    Some(f64::from(ticks)),
                    Some("stepping simulation"),
                );
            }
            if let Ok(res) = reply_rx.recv_timeout(Duration::from_millis(10)) {
                while let Ok(done) = progress_rx.try_recv() {
                    ctx.report_progress(
                        f64::from(done),
                        Some(f64::from(ticks)),
                        Some("stepping simulation"),
                    );
                }
                return res;
            }
        }
    }

    pub(crate) fn snapshot(&self, animation_node_id: Option<&str>) -> Result<Value, String> {
        let (tx, rx) = mpsc::channel();
        self.send(PlayCommand::Snapshot {
            animation_node_id: animation_node_id.map(str::to_owned),
            reply: tx,
        })?;
        rx.recv().map_err(|_| "play worker stopped".to_owned())?
    }

    pub(crate) fn stop(self) -> Result<Value, String> {
        let (tx, rx) = mpsc::channel();
        let _ = self.send(PlayCommand::Stop { reply: tx });
        let res = rx
            .recv()
            .unwrap_or_else(|_| Err("play worker exited".to_owned()));
        if let Ok(mut guard) = self.join.lock()
            && let Some(join) = guard.take()
        {
            let _ = join.join();
        }
        res
    }

    fn send(&self, cmd: PlayCommand) -> Result<(), String> {
        self.tx
            .lock()
            .map_err(|_| "play command channel poisoned".to_owned())?
            .send(cmd)
            .map_err(|_| "play session is not running".to_owned())
    }
}

impl Drop for PlayWorker {
    fn drop(&mut self) {
        if let Ok(tx) = self.tx.lock() {
            let (reply, _) = mpsc::channel();
            let _ = tx.send(PlayCommand::Stop { reply });
        }
        if let Ok(mut guard) = self.join.lock()
            && let Some(join) = guard.take()
        {
            let _ = join.join();
        }
    }
}

fn run_play_actor(game: &mut HeadlessGame, source: &Path, rx: mpsc::Receiver<PlayCommand>) {
    let mut viewport = [1280.0_f32, 720.0_f32];
    while let Ok(cmd) = rx.recv() {
        match cmd {
            PlayCommand::Step {
                ticks,
                dt,
                delay_ms,
                cancel,
                progress_tx,
                reply,
            } => {
                let res = step_game(game, ticks, dt, delay_ms, &cancel, &progress_tx);
                let _ = reply.send(res);
            }
            PlayCommand::Input {
                key,
                pointer,
                focused,
                reply,
            } => {
                let res = apply_game_input(game, &mut viewport, key, pointer, focused);
                let _ = reply.send(res);
            }
            PlayCommand::Snapshot {
                animation_node_id,
                reply,
            } => {
                let _ = reply.send(capture_game_snapshot(
                    game,
                    source,
                    animation_node_id.as_deref(),
                ));
            }
            PlayCommand::TransitionScene { scene_path, reply } => {
                let res = game
                    .transition_scene(&scene_path)
                    .map_err(|e| e.to_string())
                    .and_then(|()| {
                        let scene = game.snapshot().map_err(|e| e.to_string())?;
                        Ok(json!({
                            "transitioned": true,
                            "activeScenePath": game.active_scene_path(),
                            "fixedTicks": game.fixed_ticks(),
                            "objectCount": scene.objects.len(),
                        }))
                    });
                let _ = reply.send(res);
            }
            PlayCommand::Stop { reply } => {
                let _ = reply.send(Ok(
                    json!({ "stopped": true, "finalTicks": game.fixed_ticks() }),
                ));
                break;
            }
        }
    }
}

fn step_game(
    game: &mut HeadlessGame,
    ticks: u32,
    dt: f64,
    delay_ms: Option<u64>,
    cancel: &CancelToken,
    progress_tx: &mpsc::Sender<u32>,
) -> Result<Value, String> {
    let mut executed = 0u32;
    for step in 1..=ticks {
        if cancel.is_cancelled() {
            return Err(format!("play_step cancelled after {executed} ticks"));
        }
        game.advance(dt).map_err(|e| e.to_string())?;
        executed = step;
        if step == ticks || step % 8 == 0 || delay_ms.is_some() {
            let _ = progress_tx.send(step);
        }
        if let Some(d) = delay_ms
            && d > 0
        {
            thread::sleep(Duration::from_millis(d.min(100)));
        }
    }
    Ok(json!({
        "stepped": executed,
        "fixedTicks": game.fixed_ticks(),
        "audioStats": game.audio_stats(),
        "frameAttribution": game.frame_attribution(),
    }))
}

fn apply_game_input(
    game: &mut HeadlessGame,
    viewport: &mut [f32; 2],
    key: Option<(KeyCode, bool)>,
    pointer: Option<([f32; 2], Option<[f32; 2]>, bool)>,
    focused: Option<bool>,
) -> Result<Value, String> {
    if let Some(f) = focused {
        game.set_focused(f);
    }
    if let Some((code, pressed)) = key {
        game.key(code, pressed).map_err(|e| e.to_string())?;
    }
    if let Some((point, vp, pressed)) = pointer {
        let size = vp.unwrap_or(*viewport);
        *viewport = size;
        game.pointer(point, size, pressed)
            .map_err(|e| e.to_string())?;
    }
    Ok(json!({ "applied": true, "fixedTicks": game.fixed_ticks() }))
}

fn capture_game_snapshot(
    game: &HeadlessGame,
    source: &Path,
    animation_node_id: Option<&str>,
) -> Result<Value, String> {
    let scene = game.snapshot().map_err(|e| e.to_string())?;
    let camera = game.presentation_camera();
    let char_rep = game.character_replication_snapshot().ok();
    let animation = match animation_node_id {
        Some(id) => game
            .animation_snapshot(&voxy_scene::ObjectId(id.into()))
            .map_err(|e| e.to_string())?,
        None => None,
    };
    Ok(json!({
        "sourcePath": source.display().to_string(),
        "activeScenePath": game.active_scene_path(),
        "fixedTicks": game.fixed_ticks(),
        "audioStats": game.audio_stats(),
        "camera": camera,
        "hasCharacterPhysics": char_rep.as_ref().is_some_and(|c| c.physics.is_some()),
        "animation": animation,
        "frameAttribution": game.frame_attribution(),
        "scene": scene,
    }))
}

pub(crate) fn tool_defs() -> Vec<ToolDef> {
    let mut defs = play_tool_defs();
    defs.extend(package_tool_defs());
    defs
}

fn play_tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "play_start",
            "Start a HeadlessGame Play simulation from the active project, a project.voxy path, or a .voxypkg package.",
            json!({
                "type": "object",
                "properties": { "project_path": { "type": "string" }, "package_path": { "type": "string" } },
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "play_step",
            "Advance HeadlessGame by N fixed ticks (cancellable via notifications/cancelled with _meta.progressToken progress).",
            json!({
                "type": "object",
                "properties": {
                    "ticks": { "type": "integer", "minimum": 1, "maximum": 3600, "default": 1 },
                    "step_seconds": { "type": "number", "exclusiveMinimum": 0, "maximum": 0.2, "default": 0.0166666667 },
                    "tick_delay_ms": { "type": "integer", "minimum": 0, "maximum": 100 }
                },
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "play_input",
            "Deliver keyboard press/release, pointer click/move, or focus change to the active HeadlessGame session.",
            json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string" },
                    "pressed": { "type": "boolean" },
                    "pointer": {
                        "type": "object",
                        "properties": {
                            "point": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2 },
                            "viewport": { "type": "array", "items": { "type": "number" }, "minItems": 2, "maxItems": 2 },
                            "pressed": { "type": "boolean" }
                        },
                        "required": ["point", "pressed"],
                        "additionalProperties": false
                    },
                    "focused": { "type": "boolean" }
                },
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "play_snapshot",
            "Capture fixed tick count, scene document, presentation camera, audio stats, character physics, and optional skeletal animation snapshot.",
            json!({
                "type": "object",
                "properties": { "animation_node_id": { "type": "string" } },
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "play_transition_scene",
            "Transition the active HeadlessGame Play session to another .scene.json in the project or .voxypkg package while synchronizing active background workers.",
            json!({
                "type": "object",
                "properties": { "scene_path": { "type": "string" } },
                "required": ["scene_path"],
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "play_stop",
            "Stop the active HeadlessGame Play session and join resource workers cleanly.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
    ]
}

fn package_tool_defs() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "package_export_project",
            "Export a validated Voxy project into a standalone immutable .voxypkg distribution.",
            json!({
                "type": "object",
                "properties": { "project_path": { "type": "string" }, "output_path": { "type": "string" } },
                "required": ["output_path"],
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "package_export_game",
            "Export a model + scene pair into an immutable game package.",
            json!({
                "type": "object",
                "properties": {
                    "model_path": { "type": "string" },
                    "manifest_path": { "type": "string" },
                    "asset": { "type": "string" },
                    "scene_path": { "type": "string" },
                    "output_path": { "type": "string" }
                },
                "required": ["scene_path", "output_path"],
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "client_bridge_probe",
            "Invoke MCP server->client capabilities (`roots`, `sampling`, `elicitation`, or `ping`) cleanly without holding server locks.",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["roots", "sampling", "elicitation", "ping"] },
                    "payload": { "type": "object" }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
        ),
    ]
}

pub(crate) fn dispatch(
    state: &mut EngineState,
    ctx: &RequestContext,
    name: &str,
    arguments: Value,
) -> Result<ToolOutcome, String> {
    match name {
        "play_start" => play_start(state, arguments),
        "play_step" => play_step(state, ctx, arguments),
        "play_input" => play_input(state, arguments),
        "play_snapshot" => play_snapshot(state, arguments),
        "play_transition_scene" => play_transition_scene(state, arguments),
        "play_stop" => play_stop(state, arguments),
        "package_export_project" => package_export_project(state, ctx, arguments),
        "package_export_game" => package_export_game(ctx, arguments),
        _ => Err(format!("unknown play/package tool: {name}")),
    }
}

pub(crate) fn run_client_bridge_probe(
    ctx: &RequestContext,
    arguments: Value,
) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        action: String,
        #[serde(default)]
        payload: Option<Value>,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    match args.action.as_str() {
        "ping" => {
            ctx.ping_client()?;
            Ok(ToolOutcome::ok(json!({ "action": "ping", "ok": true })))
        }
        "roots" => {
            let roots = ctx.list_roots()?;
            let mut projects = Vec::new();
            for root in &roots {
                if let Some(dir) = root.uri.strip_prefix("file://") {
                    let candidate = Path::new(dir).join("project.voxy");
                    if candidate.is_file() {
                        projects.push(candidate.display().to_string());
                    }
                }
            }
            Ok(ToolOutcome::ok(json!({
                "action": "roots",
                "roots": roots,
                "discoveredProjects": projects,
            })))
        }
        "sampling" => {
            let params = args.payload.unwrap_or_else(|| {
                json!({
                    "messages": [{ "role": "user", "content": { "type": "text", "text": "Inspect active Voxy scene." } }],
                    "maxTokens": 128
                })
            });
            let reply = ctx.create_sampling_message(params)?;
            Ok(ToolOutcome::ok(
                json!({ "action": "sampling", "reply": reply }),
            ))
        }
        "elicitation" => {
            let params = args.payload.unwrap_or_else(|| {
                json!({
                    "message": "Confirm Voxy project action",
                    "requestedSchema": {
                        "type": "object",
                        "properties": { "confirm": { "type": "boolean" } },
                        "required": ["confirm"]
                    }
                })
            });
            let reply = ctx.create_elicitation(params)?;
            Ok(ToolOutcome::ok(
                json!({ "action": "elicitation", "reply": reply }),
            ))
        }
        other => Err(format!("unsupported client_bridge_probe action: {other}")),
    }
}

fn play_start(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        project_path: Option<String>,
        package_path: Option<String>,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    if args.project_path.is_some() && args.package_path.is_some() {
        return Err("specify either project_path or package_path, not both".to_owned());
    }
    if let Some(existing) = state.play.take() {
        existing.stop()?;
    }
    let (source, from_package) = if let Some(pkg) = args.package_path {
        (PathBuf::from(pkg), true)
    } else {
        let p = match args.project_path {
            Some(s) => PathBuf::from(s),
            None => state.require_project()?.project.descriptor_path.clone(),
        };
        (p, false)
    };
    let worker = PlayWorker::spawn(source, from_package, Arc::clone(&state.registry))?;
    let summary = worker.initial_summary.clone();
    state.play = Some(worker);
    Ok(ToolOutcome::ok(summary).with_updates(&["voxy://play/state"]))
}

fn play_step(
    state: &mut EngineState,
    ctx: &RequestContext,
    arguments: Value,
) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        #[serde(default = "one")]
        ticks: u32,
        #[serde(default = "default_dt")]
        step_seconds: f64,
        #[serde(default)]
        tick_delay_ms: Option<u64>,
    }
    const fn one() -> u32 {
        1
    }
    const fn default_dt() -> f64 {
        1.0 / 60.0
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    if !(1..=3600).contains(&args.ticks) {
        return Err("ticks must be in 1..=3600".to_owned());
    }
    let res = state
        .require_play()?
        .step(ctx, args.ticks, args.step_seconds, args.tick_delay_ms)?;
    Ok(ToolOutcome::ok(res).with_updates(&["voxy://play/state"]))
}

fn play_input(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct PointerArgs {
        point: [f32; 2],
        viewport: Option<[f32; 2]>,
        pressed: bool,
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        key: Option<String>,
        pressed: Option<bool>,
        pointer: Option<PointerArgs>,
        focused: Option<bool>,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let key = match &args.key {
        Some(name) => Some((parse_key_code(name)?, args.pressed.unwrap_or(true))),
        None => None,
    };
    let pointer = args.pointer.map(|p| (p.point, p.viewport, p.pressed));
    let play = state.require_play()?;
    let (tx, rx) = mpsc::channel();
    play.send(PlayCommand::Input {
        key,
        pointer,
        focused: args.focused,
        reply: tx,
    })?;
    let res = rx.recv().map_err(|_| "play worker stopped".to_owned())??;
    Ok(ToolOutcome::ok(res))
}

fn play_snapshot(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        animation_node_id: Option<String>,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let snap = state
        .require_play()?
        .snapshot(args.animation_node_id.as_deref())?;
    Ok(ToolOutcome::ok(snap))
}

fn play_transition_scene(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        scene_path: String,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let play = state.require_play()?;
    let (tx, rx) = mpsc::channel();
    play.send(PlayCommand::TransitionScene {
        scene_path: args.scene_path,
        reply: tx,
    })?;
    let res = rx.recv().map_err(|_| "play worker stopped".to_owned())??;
    Ok(ToolOutcome::ok(res).with_updates(&["voxy://play/state"]))
}

fn play_stop(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Empty {}
    let _: Empty = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let play = state
        .play
        .take()
        .ok_or_else(|| "no play session running".to_owned())?;
    let res = play.stop()?;
    Ok(ToolOutcome::ok(res).with_updates(&["voxy://play/state"]))
}

fn package_export_project(
    state: &mut EngineState,
    ctx: &RequestContext,
    arguments: Value,
) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        project_path: Option<String>,
        output_path: String,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let source = match args.project_path {
        Some(p) => PathBuf::from(p),
        None => state.require_project()?.project.descriptor_path.clone(),
    };
    let output = PathBuf::from(&args.output_path);
    ctx.report_progress(0.25, Some(1.0), Some("validating project closure"));
    crate::export_project_package_with_components(&source, &output, (*state.registry).clone())
        .map_err(|e| e.to_string())?;
    ctx.report_progress(1.0, Some(1.0), Some("package written"));
    let bytes = std::fs::read(&output).map_err(|e| e.to_string())?;
    Ok(ToolOutcome::ok(json!({
        "exported": true,
        "projectPath": source.display().to_string(),
        "outputPath": output.display().to_string(),
        "bytes": bytes.len(),
        "blake3": blake3::hash(&bytes).to_hex().to_string(),
    })))
}

fn package_export_game(ctx: &RequestContext, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        model_path: Option<String>,
        manifest_path: Option<String>,
        asset: Option<String>,
        scene_path: String,
        output_path: String,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let source = match (args.model_path, args.manifest_path, args.asset) {
        (Some(file), None, None) => ModelSource::File(PathBuf::from(file)),
        (None, Some(manifest), Some(asset)) => ModelSource::Manifest {
            path: PathBuf::from(manifest),
            asset: voxy_assets::AssetId(asset),
        },
        _ => return Err("supply either model_path OR (manifest_path + asset)".to_owned()),
    };
    let output = PathBuf::from(&args.output_path);
    ctx.report_progress(0.25, Some(1.0), Some("exporting game package"));
    crate::export_game_package(&source, Path::new(&args.scene_path), &output)
        .map_err(|e| e.to_string())?;
    ctx.report_progress(1.0, Some(1.0), Some("package written"));
    let bytes = std::fs::read(&output).map_err(|e| e.to_string())?;
    Ok(ToolOutcome::ok(json!({
        "exported": true,
        "outputPath": output.display().to_string(),
        "bytes": bytes.len(),
        "blake3": blake3::hash(&bytes).to_hex().to_string(),
    })))
}

fn parse_key_code(name: &str) -> Result<KeyCode, String> {
    match name {
        "ArrowLeft" | "Left" => Ok(KeyCode::ArrowLeft),
        "ArrowRight" | "Right" => Ok(KeyCode::ArrowRight),
        "ArrowUp" | "Up" => Ok(KeyCode::ArrowUp),
        "ArrowDown" | "Down" => Ok(KeyCode::ArrowDown),
        "Space" | " " => Ok(KeyCode::Space),
        "Enter" | "Return" => Ok(KeyCode::Enter),
        "Escape" | "Esc" => Ok(KeyCode::Escape),
        "Tab" => Ok(KeyCode::Tab),
        "KeyW" | "W" | "w" => Ok(KeyCode::KeyW),
        "KeyA" | "A" | "a" => Ok(KeyCode::KeyA),
        "KeyS" | "S" | "s" => Ok(KeyCode::KeyS),
        "KeyD" | "D" | "d" => Ok(KeyCode::KeyD),
        "KeyQ" | "Q" | "q" => Ok(KeyCode::KeyQ),
        "KeyE" | "E" | "e" => Ok(KeyCode::KeyE),
        "Digit0" | "0" => Ok(KeyCode::Digit0),
        "Digit1" | "1" => Ok(KeyCode::Digit1),
        "Digit2" | "2" => Ok(KeyCode::Digit2),
        "Digit3" | "3" => Ok(KeyCode::Digit3),
        "F1" => Ok(KeyCode::F1),
        "F5" => Ok(KeyCode::F5),
        "F6" => Ok(KeyCode::F6),
        "F9" => Ok(KeyCode::F9),
        "F10" => Ok(KeyCode::F10),
        _ => Err(format!("unsupported key code `{name}`")),
    }
}
