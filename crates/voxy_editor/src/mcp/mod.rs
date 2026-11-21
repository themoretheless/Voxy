//! Comprehensive Voxy Engine & Editor MCP server covering projects, assets, scenes,
//! all 30 component schemas, prefabs, HeadlessGame Play, packages, and offscreen/live rendering.

mod catalog;
mod play_package;
mod prefabs;
mod projects_assets;
mod render;
mod scene_ops;
mod schemas;
mod state;

use serde_json::Value;
use state::{EngineState, ToolOutcome};
use std::path::Path;
use std::sync::{Arc, Mutex};
use voxy_mcp::{
    CallToolResult, CompleteParams, CompletionValues, ContentBlock, DEFAULT_PAGE_SIZE,
    GetPromptResult, LogLevel, McpHandler, PromptDef, RequestContext, ResourceContents,
    ResourceDef, ResourceTemplateDef, RpcError, ServerConfig, ToolDef, parse_transport_args, serve,
};

#[derive(Debug)]
pub struct EngineMcpServer {
    state: Mutex<EngineState>,
    config: ServerConfig,
    tools: Vec<ToolDef>,
}

impl EngineMcpServer {
    pub fn new(initial_project: Option<&Path>, page_size: Option<usize>) -> Result<Self, String> {
        let registry = Arc::new(crate::editor_component_registry().map_err(|e| e.to_string())?);
        let mut engine_state = EngineState::new(Arc::clone(&registry));
        if let Some(path) = initial_project {
            engine_state.project = Some(projects_assets::open_project_session(path, &registry)?);
        }
        let mut tools = projects_assets::tool_defs();
        tools.extend(scene_ops::tool_defs());
        tools.extend(prefabs::tool_defs());
        tools.extend(play_package::tool_defs());
        tools.extend(render::tool_defs());
        let config = ServerConfig::new("voxy-engine", "0.1.0")
            .with_title("Voxy Engine & Editor MCP")
            .with_instructions(
                "Unified Voxy Engine & Editor MCP (`2025-06-18`). Covers projects, asset manifests, scene graphs + undo/redo/save, all 30 registered components, non-destructive prefab composition, HeadlessGame Play stepping/inputs/snapshots, .voxypkg package export, offscreen Metal viewport renders (`capture: \"offscreen\"`), and live voxy_editor window capture (`capture: \"live_window\"`).",
            )
            .with_page_size(page_size.unwrap_or(DEFAULT_PAGE_SIZE));
        Ok(Self {
            state: Mutex::new(engine_state),
            config,
            tools,
        })
    }

    fn apply_outcome(
        ctx: &RequestContext,
        tool_name: &str,
        outcome: ToolOutcome,
    ) -> CallToolResult {
        ctx.log(
            LogLevel::Info,
            "voxy.mcp",
            serde_json::json!({ "tool": tool_name, "updated": outcome.updated_uris }),
        );
        for uri in &outcome.updated_uris {
            ctx.notify_resource_updated(uri);
        }
        if outcome.list_changed {
            ctx.notify_resources_list_changed();
        }
        let summary_text = serde_json::to_string_pretty(&outcome.structured)
            .unwrap_or_else(|_| outcome.structured.to_string());
        let mut content = vec![ContentBlock::Text { text: summary_text }];
        content.extend(outcome.extra_content);
        CallToolResult {
            content,
            structured_content: Some(outcome.structured),
            is_error: false,
        }
    }
}

impl McpHandler for EngineMcpServer {
    fn config(&self) -> ServerConfig {
        self.config.clone()
    }

    fn list_tools(&self) -> Vec<ToolDef> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        ctx: &RequestContext,
        name: &str,
        arguments: Value,
    ) -> Result<CallToolResult, String> {
        if name == "client_bridge_probe" {
            let outcome = play_package::run_client_bridge_probe(ctx, arguments)?;
            return Ok(Self::apply_outcome(ctx, name, outcome));
        }
        let outcome = {
            let mut guard = self
                .state
                .lock()
                .map_err(|_| "engine MCP state poisoned".to_owned())?;
            match name {
                "project_create"
                | "project_open"
                | "project_info"
                | "project_validate"
                | "project_update_descriptor"
                | "asset_list"
                | "asset_inspect"
                | "asset_register"
                | "asset_remove" => projects_assets::dispatch(&mut guard, name, arguments)?,
                "component_schemas"
                | "scene_get"
                | "scene_open_file"
                | "scene_apply"
                | "scene_undo"
                | "scene_redo"
                | "scene_save"
                | "scene_recover_autosave"
                | "scene_query" => scene_ops::dispatch(&mut guard, name, arguments)?,
                "prefab_scene_get"
                | "prefab_instance_add"
                | "prefab_instance_remove"
                | "prefab_override_set"
                | "prefab_override_clear" => prefabs::dispatch(&mut guard, name, arguments)?,
                "play_start"
                | "play_step"
                | "play_input"
                | "play_snapshot"
                | "play_transition_scene"
                | "play_stop"
                | "package_export_project"
                | "package_export_game" => {
                    play_package::dispatch(&mut guard, ctx, name, arguments)?
                }
                "viewport_render_offscreen" | "editor_window_status" | "editor_window_capture" => {
                    render::dispatch(&mut guard, ctx, name, arguments)?
                }
                _ => return Err(format!("unknown tool: {name}")),
            }
        };
        Ok(Self::apply_outcome(ctx, name, outcome))
    }

    fn list_resources(&self) -> Vec<ResourceDef> {
        self.state
            .lock()
            .map_or_else(|_| Vec::new(), |g| catalog::list_resources(&g))
    }

    fn list_resource_templates(&self) -> Vec<ResourceTemplateDef> {
        catalog::list_resource_templates()
    }

    fn read_resource(
        &self,
        _ctx: &RequestContext,
        uri: &str,
    ) -> Result<Vec<ResourceContents>, RpcError> {
        let guard = self
            .state
            .lock()
            .map_err(|_| RpcError::new(-32603, "engine MCP state poisoned"))?;
        catalog::read_resource(&guard, uri)
    }

    fn list_prompts(&self) -> Vec<PromptDef> {
        catalog::list_prompts()
    }

    fn get_prompt(
        &self,
        _ctx: &RequestContext,
        name: &str,
        arguments: Value,
    ) -> Result<GetPromptResult, RpcError> {
        catalog::get_prompt(name, arguments)
    }

    fn complete(
        &self,
        _ctx: &RequestContext,
        params: &CompleteParams,
    ) -> Result<CompletionValues, RpcError> {
        let guard = self
            .state
            .lock()
            .map_err(|_| RpcError::new(-32603, "engine MCP state poisoned"))?;
        Ok(catalog::complete(&guard, params))
    }
}

static STDIO_QUIET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Routes diagnostic stdout banners to stderr when running inside an MCP server.
pub fn set_stdio_quiet(quiet: bool) {
    STDIO_QUIET.store(quiet, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn stdio_quiet() -> bool {
    STDIO_QUIET.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn run_cli<I, S>(args: I) -> Result<(), Box<dyn std::error::Error>>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    set_stdio_quiet(true);
    let (mode, rest) = parse_transport_args(args)?;
    let mut project_path: Option<String> = None;
    let mut page_size: Option<usize> = None;
    let mut iter = rest.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--page-size" => {
                let raw = iter
                    .next()
                    .ok_or("--page-size requires a positive integer")?;
                page_size = Some(raw.parse::<usize>()?);
            }
            "--help" | "-h" => {
                eprintln!(
                    "usage: voxy_mcp [project.voxy] [--http 127.0.0.1:port | --http-port port] [--page-size N]"
                );
                return Ok(());
            }
            _ if !arg.starts_with('-') && project_path.is_none() => project_path = Some(arg),
            other => return Err(format!("unexpected argument `{other}`").into()),
        }
    }
    let server = Arc::new(EngineMcpServer::new(
        project_path.as_deref().map(Path::new),
        page_size,
    )?);
    serve(server, mode)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use voxy_mcp::Router;

    #[test]
    fn engine_mcp_project_scene_prefab_play_and_resources_work() {
        let dir = std::env::temp_dir().join(format!(
            "voxy-engine-mcp-unit-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let server = Arc::new(EngineMcpServer::new(None, None).unwrap());
        let router = Router::new(server);
        assert_eq!(
            router.dispatch_sync(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))["error"]["code"],
            -32000
        );
        let _ = router.dispatch_sync(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "initialize",
            "params": { "protocolVersion": "2025-06-18" }
        }));
        let schemas = router.dispatch_sync(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "resources/read",
            "params": { "uri": "voxy://engine/schemas" }
        }));
        let parsed: Value =
            serde_json::from_str(schemas["result"]["contents"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(parsed["count"], 30);

        std::fs::create_dir_all(&dir).unwrap();
        let proj_dir = dir.join("proj");
        let created = router.dispatch_sync(&json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "project_create",
                "arguments": { "directory": proj_dir.to_string_lossy(), "name": "Unit MCP" }
            }
        }));
        assert_eq!(created["result"]["isError"], false);

        let apply = router.dispatch_sync(&json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "scene_apply",
                "arguments": {
                    "expected_revision": 0,
                    "edits": [{
                        "op": "create",
                        "object": {
                            "id": "node-1",
                            "parent": null,
                            "name": "Node 1",
                            "active": true,
                            "translation": [1.0, 2.0, 3.0],
                            "rotation": [0.0, 0.0, 0.0, 1.0],
                            "scale": [1.0, 1.0, 1.0],
                            "components": { "game.audio-bus.v1": { "bus": 1, "gain": 0.5 } }
                        }
                    }]
                }
            }
        }));
        assert_eq!(apply["result"]["isError"], false);
        assert_eq!(apply["result"]["structuredContent"]["revision"], 1);

        let save = router.dispatch_sync(&json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": { "name": "scene_save", "arguments": { "expected_revision": 1 } }
        }));
        assert_eq!(save["result"]["isError"], false);

        let comp = router.dispatch_sync(&json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "completion/complete",
            "params": {
                "ref": { "type": "ref/resource", "uri": "voxy://schemas/component/{schema}" },
                "argument": { "name": "schema", "value": "audio" }
            }
        }));
        let vals = comp["result"]["completion"]["values"].as_array().unwrap();
        assert!(vals.iter().any(|v| v == "game.audio-bus.v1"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
