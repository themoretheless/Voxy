//! MCP Resources, Resource Templates, Prompts, and Completions for `voxy_mcp`.

use super::prefabs::read_active_prefab_doc;
use super::projects_assets::{asset_row, project_summary};
use super::render::live_status_json;
use super::scene_ops::scene_snapshot_outcome;
use super::schemas::{all_schemas, find_schema};
use super::state::EngineState;
use crate::live_bridge::live_dir_for_project;
use serde_json::{Value, json};
use voxy_mcp::{
    CompleteParams, CompletionRef, CompletionValues, ContentBlock, GetPromptResult, PromptArgument,
    PromptDef, PromptMessage, ResourceContents, ResourceDef, ResourceTemplateDef, RpcError,
    base64_encode,
};

pub(crate) fn list_resources(state: &EngineState) -> Vec<ResourceDef> {
    let mut list = vec![
        res(
            "voxy://engine/schemas",
            "schemas",
            "All 30 built-in component schemas.",
            "application/json",
        ),
        res(
            "voxy://engine/capabilities",
            "capabilities",
            "Voxy MCP engine capabilities and limits.",
            "application/json",
        ),
    ];
    if state.project.is_some() {
        list.push(res(
            "voxy://project/descriptor",
            "project-descriptor",
            "Active project.voxy descriptor and summary.",
            "application/json",
        ));
        list.push(res(
            "voxy://project/assets",
            "project-assets",
            "Manifest assets with sizes and BLAKE3 hashes.",
            "application/json",
        ));
        list.push(res(
            "voxy://project/scene",
            "project-scene",
            "Active session SceneDocument, revision, and hash.",
            "application/json",
        ));
        list.push(res(
            "voxy://project/prefabs",
            "project-prefabs",
            "Raw authored PrefabSceneDocument and expanded preview.",
            "application/json",
        ));
    }
    if state.play.is_some() {
        list.push(res(
            "voxy://play/state",
            "play-state",
            "Live HeadlessGame simulation snapshot.",
            "application/json",
        ));
    }
    list.push(res(
        "voxy://editor/live-status",
        "editor-live-status",
        "Live voxy_editor window bridge status.",
        "application/json",
    ));
    if state.latest_render.is_some() {
        list.push(res(
            "voxy://renders/latest",
            "latest-render",
            "Latest PNG rendered by offscreen or live capture.",
            "image/png",
        ));
    }
    list
}

fn res(uri: &str, name: &str, description: &str, mime: &str) -> ResourceDef {
    ResourceDef {
        uri: uri.to_owned(),
        name: name.to_owned(),
        title: None,
        description: Some(description.to_owned()),
        mime_type: Some(mime.to_owned()),
        size: None,
    }
}

pub(crate) fn list_resource_templates() -> Vec<ResourceTemplateDef> {
    vec![
        tpl(
            "voxy://schemas/component/{schema}",
            "component-schema",
            "Metadata and default sample JSON for one component schema.",
            "application/json",
        ),
        tpl(
            "voxy://scene/node/{node_id}",
            "scene-node",
            "Single SceneObject and its components by ObjectId.",
            "application/json",
        ),
        tpl(
            "voxy://assets/item/{asset_id}",
            "asset-item",
            "Single manifest asset entry with size and BLAKE3 hash.",
            "application/json",
        ),
        tpl(
            "voxy://play/animation/{node_id}",
            "play-animation",
            "Active Play skeletal animation snapshot for an ObjectId.",
            "application/json",
        ),
    ]
}

fn tpl(uri_template: &str, name: &str, description: &str, mime: &str) -> ResourceTemplateDef {
    ResourceTemplateDef {
        uri_template: uri_template.to_owned(),
        name: name.to_owned(),
        title: None,
        description: Some(description.to_owned()),
        mime_type: Some(mime.to_owned()),
    }
}

pub(crate) fn read_resource(
    state: &EngineState,
    uri: &str,
) -> Result<Vec<ResourceContents>, RpcError> {
    if let Some(res) = read_static_resource(state, uri)? {
        return Ok(res);
    }
    if let Some(res) = read_templated_resource(state, uri)? {
        return Ok(res);
    }
    Err(RpcError::resource_not_found(uri))
}

fn read_static_resource(
    state: &EngineState,
    uri: &str,
) -> Result<Option<Vec<ResourceContents>>, RpcError> {
    match uri {
        "voxy://engine/schemas" => {
            let schemas = all_schemas();
            Ok(Some(vec![ResourceContents::json(
                uri,
                &json!({ "count": schemas.len(), "schemas": schemas }),
            )]))
        }
        "voxy://engine/capabilities" => Ok(Some(vec![ResourceContents::json(
            uri,
            &json!({
                "protocol": "2025-06-18",
                "transports": ["stdio", "streamable-http"],
                "componentSchemaCount": all_schemas().len(),
                "sceneCapacity": super::state::SCENE_CAPACITY,
                "subsystems": ["projects", "assets", "scenes", "components", "prefabs", "play", "packages", "offscreen_render", "live_window_capture"],
            }),
        )])),
        "voxy://project/descriptor" => {
            let session = state.require_project().map_err(RpcError::invalid_params)?;
            Ok(Some(vec![ResourceContents::json(
                uri,
                &project_summary(session),
            )]))
        }
        "voxy://project/assets" => {
            let session = state.require_project().map_err(RpcError::invalid_params)?;
            let assets: Vec<Value> = session
                .project
                .assets
                .bindings()
                .map(|(id, src)| asset_row(&session.project.root, &id.0, src.as_str()))
                .collect();
            Ok(Some(vec![ResourceContents::json(
                uri,
                &json!({ "assets": assets }),
            )]))
        }
        "voxy://project/scene" => {
            let outcome = scene_snapshot_outcome(state).map_err(RpcError::invalid_params)?;
            Ok(Some(vec![ResourceContents::json(uri, &outcome.structured)]))
        }
        "voxy://project/prefabs" => {
            let session = state.require_project().map_err(RpcError::invalid_params)?;
            let (_, authored, expanded) = read_active_prefab_doc(session, &state.registry, None)
                .map_err(RpcError::invalid_params)?;
            Ok(Some(vec![ResourceContents::json(
                uri,
                &json!({ "authored": authored, "expanded": expanded }),
            )]))
        }
        "voxy://play/state" => {
            let play = state.require_play().map_err(RpcError::invalid_params)?;
            let snap = play.snapshot(None).map_err(RpcError::invalid_params)?;
            Ok(Some(vec![ResourceContents::json(uri, &snap)]))
        }
        "voxy://editor/live-status" => {
            let root = state.project.as_ref().map(|p| p.project.root.as_path());
            let value = live_dir_for_project(root).map_or_else(
                || json!({ "liveWindowConnected": false }),
                |d| live_status_json(&d),
            );
            Ok(Some(vec![ResourceContents::json(uri, &value)]))
        }
        "voxy://renders/latest" => {
            let frame = state
                .latest_render
                .as_ref()
                .ok_or_else(|| RpcError::resource_not_found(uri))?;
            Ok(Some(vec![ResourceContents::blob(
                uri,
                "image/png",
                base64_encode(&frame.png),
            )]))
        }
        _ => Ok(None),
    }
}

fn read_templated_resource(
    state: &EngineState,
    uri: &str,
) -> Result<Option<Vec<ResourceContents>>, RpcError> {
    if let Some(schema) = uri.strip_prefix("voxy://schemas/component/") {
        let entry = find_schema(schema).ok_or_else(|| RpcError::resource_not_found(uri))?;
        return Ok(Some(vec![ResourceContents::json(
            uri,
            &serde_json::to_value(entry).unwrap_or(Value::Null),
        )]));
    }
    if let Some(node_id) = uri.strip_prefix("voxy://scene/node/") {
        let session = state.require_project().map_err(RpcError::invalid_params)?;
        let obj = session
            .current_scene()
            .objects
            .iter()
            .find(|o| o.id.0 == node_id)
            .ok_or_else(|| RpcError::resource_not_found(uri))?;
        return Ok(Some(vec![ResourceContents::json(
            uri,
            &serde_json::to_value(obj).unwrap_or(Value::Null),
        )]));
    }
    if let Some(asset_id) = uri.strip_prefix("voxy://assets/item/") {
        let session = state.require_project().map_err(RpcError::invalid_params)?;
        let source = session
            .project
            .assets
            .source(&voxy_assets::AssetId(asset_id.into()))
            .ok_or_else(|| RpcError::resource_not_found(uri))?;
        return Ok(Some(vec![ResourceContents::json(
            uri,
            &asset_row(&session.project.root, asset_id, source.as_str()),
        )]));
    }
    if let Some(node_id) = uri.strip_prefix("voxy://play/animation/") {
        let play = state.require_play().map_err(RpcError::invalid_params)?;
        let snap = play
            .snapshot(Some(node_id))
            .map_err(RpcError::invalid_params)?;
        return Ok(Some(vec![ResourceContents::json(
            uri,
            &json!({ "nodeId": node_id, "animation": snap["animation"] }),
        )]));
    }
    Ok(None)
}

pub(crate) fn list_prompts() -> Vec<PromptDef> {
    vec![
        PromptDef {
            name: "inspect_project".into(),
            title: Some("Inspect Voxy Project".into()),
            description: Some(
                "Structured prompt to audit project assets, scenes, and validation status.".into(),
            ),
            arguments: vec![PromptArgument {
                name: "project_path".into(),
                description: Some("Absolute path to project.voxy".into()),
                required: false,
            }],
        },
        PromptDef {
            name: "author_component".into(),
            title: Some("Author Scene Component".into()),
            description: Some(
                "Compose a valid component_set / component_patch edit on a scene node.".into(),
            ),
            arguments: vec![
                PromptArgument {
                    name: "schema".into(),
                    description: Some("Component schema ID".into()),
                    required: true,
                },
                PromptArgument {
                    name: "node_id".into(),
                    description: Some("Target scene ObjectId".into()),
                    required: true,
                },
            ],
        },
        PromptDef {
            name: "debug_play_session".into(),
            title: Some("Debug Play Simulation".into()),
            description: Some(
                "Step HeadlessGame physics/animations/audio and inspect snapshots.".into(),
            ),
            arguments: vec![PromptArgument {
                name: "ticks".into(),
                description: Some("Fixed ticks per step".into()),
                required: false,
            }],
        },
        PromptDef {
            name: "prefab_override_workflow".into(),
            title: Some("Prefab Override Workflow".into()),
            description: Some(
                "Instantiate and override prefab properties without flattening composition.".into(),
            ),
            arguments: vec![
                PromptArgument {
                    name: "prefab_asset".into(),
                    description: Some("Prefab AssetId".into()),
                    required: true,
                },
                PromptArgument {
                    name: "instance_id".into(),
                    description: Some("New or existing PrefabInstance ID".into()),
                    required: true,
                },
            ],
        },
    ]
}

pub(crate) fn get_prompt(name: &str, arguments: Value) -> Result<GetPromptResult, RpcError> {
    let arg = |k: &str, default: &str| {
        arguments
            .get(k)
            .and_then(Value::as_str)
            .unwrap_or(default)
            .to_owned()
    };
    let (description, text) = match name {
        "inspect_project" => {
            let p = arg("project_path", "<active project>");
            (
                "Inspect a Voxy project via MCP resources and validation tools",
                format!(
                    "Open `{p}` (or read `voxy://project/descriptor`, `voxy://project/assets`, `voxy://project/scene`), run `project_validate`, and render `viewport_render_offscreen` to verify geometry and materials."
                ),
            )
        }
        "author_component" => {
            let schema = arg("schema", "editor.material.v1");
            let node = arg("node_id", "root");
            let sample = find_schema(&schema).map_or_else(|| json!({}), |s| s.sample);
            (
                "Author a validated component on a scene object",
                format!(
                    "Inspect `voxy://schemas/component/{schema}` (sample `{sample}`), call `scene_get` for `expected_revision`, then call `scene_apply` with `op: \"component_set\"`, `node_id: \"{node}\"`, `schema: \"{schema}\"`."
                ),
            )
        }
        "debug_play_session" => {
            let ticks = arg("ticks", "60");
            (
                "Step and inspect HeadlessGame Play simulation",
                format!(
                    "Call `play_start`, optionally deliver inputs via `play_input`, advance `{ticks}` ticks with `play_step`, inspect `play_snapshot` (including `audioStats` and transforms), and close with `play_stop`."
                ),
            )
        }
        "prefab_override_workflow" => {
            let asset = arg("prefab_asset", "prop");
            let inst = arg("instance_id", "prop-1");
            (
                "Author non-destructive prefab instances and overrides",
                format!(
                    "Read `prefab_scene_get`, add instance `{inst}` from asset `{asset}` with `prefab_instance_add`, apply per-target deltas via `prefab_override_set`, and verify the authored `instances` stay unflattened."
                ),
            )
        }
        _ => return Err(RpcError::invalid_params(format!("unknown prompt: {name}"))),
    };
    Ok(GetPromptResult {
        description: Some(description.to_owned()),
        messages: vec![PromptMessage {
            role: "user".to_owned(),
            content: ContentBlock::Text { text },
        }],
    })
}

pub(crate) fn complete(state: &EngineState, params: &CompleteParams) -> CompletionValues {
    let prefix = &params.argument.value;
    match (&params.target, params.argument.name.as_str()) {
        (_, "schema") => CompletionValues::from_candidates(
            all_schemas().into_iter().map(|s| s.schema.to_owned()),
            prefix,
        ),
        (_, "node_id") => {
            let ids: Vec<String> = state
                .project
                .as_ref()
                .map(|p| {
                    p.current_scene()
                        .objects
                        .iter()
                        .map(|o| o.id.0.clone())
                        .collect()
                })
                .unwrap_or_default();
            CompletionValues::from_candidates(ids, prefix)
        }
        (_, "asset_id" | "prefab_asset") => {
            let ids: Vec<String> = state
                .project
                .as_ref()
                .map(|p| {
                    p.project
                        .assets
                        .bindings()
                        .map(|(id, _)| id.0.clone())
                        .collect()
                })
                .unwrap_or_default();
            CompletionValues::from_candidates(ids, prefix)
        }
        (CompletionRef::Prompt { .. }, "action") => CompletionValues::from_candidates(
            ["roots", "sampling", "elicitation", "ping"].map(str::to_owned),
            prefix,
        ),
        (CompletionRef::Prompt { .. }, "category") => CompletionValues::from_candidates(
            [
                "editor", "render", "scene", "physics", "gameplay", "audio", "ui",
            ]
            .map(str::to_owned),
            prefix,
        ),
        _ => CompletionValues::default(),
    }
}
