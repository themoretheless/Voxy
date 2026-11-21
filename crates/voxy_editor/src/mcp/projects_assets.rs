//! Project lifecycle and asset manifest management tools for `voxy_mcp`.

use super::state::{
    EngineState, MAX_DOC_BYTES, ProjectSession, SCENE_CAPACITY, ToolOutcome, read_bounded,
    resolve_project_rel,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use voxy_assets::{AssetId, AssetLocations, SourcePath};
use voxy_mcp::{ToolAnnotations, ToolDef};
use voxy_scene::{ComponentRegistry, PrefabSceneDocument, SceneDocument, SceneHistory};

pub(crate) fn tool_defs() -> Vec<ToolDef> {
    let mut defs = project_tool_defs();
    defs.extend(asset_tool_defs());
    defs
}

fn project_tool_defs() -> Vec<ToolDef> {
    let ro = ToolAnnotations {
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(true),
        open_world_hint: Some(false),
    };
    vec![
        ToolDef::new(
            "project_create",
            "Create a new Voxy project directory (project.voxy, assets.json, main.scene.json) and open it.",
            json!({
                "type": "object",
                "properties": {
                    "directory": { "type": "string" },
                    "name": { "type": "string" },
                    "open": { "type": "boolean", "default": true }
                },
                "required": ["directory", "name"],
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "project_open",
            "Open an existing project.voxy file into the session, resetting unsaved history.",
            json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"],
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "project_info",
            "Inspect currently active project metadata, scene revision, and open paths.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        )
        .with_annotations(ro.clone()),
        ToolDef::new(
            "project_validate",
            "Validate strict project assets, scene hierarchy, and gameplay descriptors.",
            json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "additionalProperties": false
            }),
        )
        .with_annotations(ro),
        ToolDef::new(
            "project_update_descriptor",
            "Atomically update active project.voxy fields (name, startup_scene, asset_manifest).",
            json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "startup_scene": { "type": "string" },
                    "asset_manifest": { "type": "string" }
                },
                "additionalProperties": false
            }),
        ),
    ]
}

fn asset_tool_defs() -> Vec<ToolDef> {
    let ro = ToolAnnotations {
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(true),
        open_world_hint: Some(false),
    };
    vec![
        ToolDef::new(
            "asset_list",
            "List all assets registered in the active project manifest with size, kind, and BLAKE3 digest.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        )
        .with_annotations(ro.clone()),
        ToolDef::new(
            "asset_inspect",
            "Inspect detailed metadata, BLAKE3 digest, and imported model/scene structure for a project asset.",
            json!({
                "type": "object",
                "properties": { "asset": { "type": "string" } },
                "required": ["asset"],
                "additionalProperties": false
            }),
        )
        .with_annotations(ro),
        ToolDef::new(
            "asset_register",
            "Bind or update a project-relative source file in assets.json and reload the active project.",
            json!({
                "type": "object",
                "properties": {
                    "asset": { "type": "string" },
                    "source": { "type": "string" },
                    "replace": { "type": "boolean", "default": true }
                },
                "required": ["asset", "source"],
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "asset_remove",
            "Remove an asset ID from assets.json and reload the active project.",
            json!({
                "type": "object",
                "properties": { "asset": { "type": "string" } },
                "required": ["asset"],
                "additionalProperties": false
            }),
        ),
    ]
}

pub(crate) fn open_project_session(
    path: &Path,
    registry: &ComponentRegistry,
) -> Result<ProjectSession, String> {
    if !path.is_absolute() {
        return Err("project path must be absolute".to_owned());
    }
    let project = crate::project::open_for_editor(path, registry).map_err(|e| e.to_string())?;
    let scene_rel = project.descriptor.startup_scene.clone();
    let scene_path = project
        .root
        .join(&scene_rel)
        .canonicalize()
        .map_err(|e| format!("resolve startup scene: {e}"))?;
    open_scene_session(project, scene_path, scene_rel, registry)
}

pub(crate) fn open_scene_session(
    project: crate::project::OpenProject,
    scene_path: PathBuf,
    scene_rel: PathBuf,
    registry: &ComponentRegistry,
) -> Result<ProjectSession, String> {
    let disk_bytes = read_bounded(&scene_path, MAX_DOC_BYTES)?;
    let disk_text = std::str::from_utf8(&disk_bytes).map_err(|e| format!("scene UTF-8: {e}"))?;
    let (prefab_doc, _) = PrefabSceneDocument::from_json_migrated(disk_text, registry)
        .map_err(|e| format!("parse scene: {e}"))?;
    let is_prefab_composed = !prefab_doc.instances.is_empty();
    let expanded = if is_prefab_composed {
        let root = project.root.clone();
        let asset_map: std::collections::BTreeMap<String, String> = project
            .assets
            .bindings()
            .map(|(id, src)| (id.0.clone(), src.as_str().to_owned()))
            .collect();
        prefab_doc
            .expand(registry, crate::scene_limits::PREFABS, |asset_id| {
                let source = asset_map.get(asset_id).ok_or_else(|| {
                    voxy_scene::DocumentError::Invalid(format!("unknown prefab asset {asset_id}"))
                })?;
                let target = root.join(source);
                let raw = fs::read_to_string(&target).map_err(|e| {
                    voxy_scene::DocumentError::Invalid(format!("read prefab {asset_id}: {e}"))
                })?;
                PrefabSceneDocument::from_json_migrated(&raw, registry).map(|(doc, _)| doc)
            })
            .map_err(|e| format!("expand prefab scene: {e}"))?
            .document
    } else {
        SceneDocument {
            version: prefab_doc.version,
            objects: prefab_doc.objects,
        }
    };
    let history = SceneHistory::new(
        expanded.clone(),
        registry,
        SCENE_CAPACITY,
        64,
        crate::scene_limits::HISTORY_BYTES,
    )
    .map_err(|e| e.to_string())?;
    Ok(ProjectSession {
        project,
        scene_path,
        scene_rel,
        history,
        saved_scene: expanded,
        disk_bytes,
        is_prefab_composed,
        revision: 0,
        camera_override: None,
    })
}

pub(crate) fn dispatch(
    state: &mut EngineState,
    name: &str,
    arguments: Value,
) -> Result<ToolOutcome, String> {
    match name {
        "project_create" => project_create(state, arguments),
        "project_open" => project_open(state, arguments),
        "project_info" => project_info(state, arguments),
        "project_validate" => project_validate(state, arguments),
        "project_update_descriptor" => project_update_descriptor(state, arguments),
        "asset_list" => asset_list(state, arguments),
        "asset_inspect" => asset_inspect(state, arguments),
        "asset_register" => asset_register(state, arguments),
        "asset_remove" => asset_remove(state, arguments),
        _ => Err(format!("unknown project/asset tool: {name}")),
    }
}

fn project_create(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        directory: String,
        name: String,
        #[serde(default = "yes")]
        open: bool,
    }
    const fn yes() -> bool {
        true
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let path = crate::project::create(Path::new(&args.directory), &args.name)
        .map_err(|e| format!("project_create: {e}"))?;
    let abs = path.canonicalize().unwrap_or(path);
    if args.open {
        let session = open_project_session(&abs, &state.registry)?;
        state.project = Some(session);
        let summary = project_summary(state.require_project()?);
        return Ok(ToolOutcome::ok(json!({
            "created": abs.display().to_string(),
            "opened": true,
            "project": summary,
        }))
        .with_updates(&[
            "voxy://project/descriptor",
            "voxy://project/assets",
            "voxy://project/scene",
        ])
        .with_list_changed());
    }
    Ok(ToolOutcome::ok(json!({
        "created": abs.display().to_string(),
        "opened": false,
    })))
}

fn project_open(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        path: String,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let session = open_project_session(Path::new(&args.path), &state.registry)?;
    state.project = Some(session);
    let summary = project_summary(state.require_project()?);
    Ok(ToolOutcome::ok(summary)
        .with_updates(&[
            "voxy://project/descriptor",
            "voxy://project/assets",
            "voxy://project/scene",
        ])
        .with_list_changed())
}

fn project_info(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Empty {}
    let _: Empty = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    Ok(ToolOutcome::ok(project_summary(state.require_project()?)))
}

fn project_validate(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        path: Option<String>,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let target = match args.path {
        Some(p) => PathBuf::from(p),
        None => state.require_project()?.project.descriptor_path.clone(),
    };
    let opened = crate::project::open(&target, &state.registry).map_err(|e| e.to_string())?;
    let loaded = opened
        .scene
        .load(&state.registry, SCENE_CAPACITY)
        .map_err(|e| format!("scene load failed: {e}"))?;
    let gameplay_check = voxy_gameplay::validate_game_descriptors(&loaded.graph, SCENE_CAPACITY)
        .err()
        .map(|e| e.to_string());
    Ok(ToolOutcome::ok(json!({
        "valid": gameplay_check.is_none(),
        "path": target.display().to_string(),
        "name": opened.descriptor.name,
        "migrated": opened.migrated,
        "objectCount": opened.scene.objects.len(),
        "assetCount": opened.assets.bindings().count(),
        "gameplayIssue": gameplay_check,
    })))
}

fn project_update_descriptor(
    state: &mut EngineState,
    arguments: Value,
) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        name: Option<String>,
        startup_scene: Option<String>,
        asset_manifest: Option<String>,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let registry = state.registry.clone();
    let session = state.require_project_mut()?;
    let mut descriptor = session.project.descriptor.clone();
    if let Some(name) = args.name {
        descriptor.name = name;
    }
    if let Some(startup) = args.startup_scene {
        descriptor.startup_scene = PathBuf::from(startup);
    }
    if let Some(manifest) = args.asset_manifest {
        descriptor.asset_manifest = PathBuf::from(manifest);
    }
    session
        .project
        .save_descriptor(&descriptor, &registry)
        .map_err(|e| e.to_string())?;
    let reopened = open_project_session(&session.project.descriptor_path, &registry)?;
    *session = reopened;
    Ok(ToolOutcome::ok(project_summary(session))
        .with_updates(&["voxy://project/descriptor", "voxy://project/scene"])
        .with_list_changed())
}

pub(crate) fn project_summary(session: &ProjectSession) -> Value {
    json!({
        "name": session.project.descriptor.name,
        "root": session.project.root.display().to_string(),
        "descriptorPath": session.project.descriptor_path.display().to_string(),
        "startupScene": session.project.descriptor.startup_scene.display().to_string(),
        "activeScene": session.scene_rel.display().to_string(),
        "assetManifest": session.project.descriptor.asset_manifest.display().to_string(),
        "migrated": session.project.migrated,
        "revision": session.revision,
        "dirty": session.dirty(),
        "isPrefabComposed": session.is_prefab_composed,
        "objectCount": session.current_scene().objects.len(),
        "assetCount": session.project.assets.bindings().count(),
    })
}

fn asset_list(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Empty {}
    let _: Empty = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let session = state.require_project()?;
    let items: Vec<Value> = session
        .project
        .assets
        .bindings()
        .map(|(id, source)| asset_row(&session.project.root, &id.0, source.as_str()))
        .collect();
    Ok(ToolOutcome::ok(json!({ "assets": items })))
}

fn asset_inspect(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        asset: String,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let session = state.require_project()?;
    let source = session
        .project
        .assets
        .source(&AssetId(args.asset.clone()))
        .ok_or_else(|| format!("unknown asset ID: {}", args.asset))?;
    let mut row = asset_row(&session.project.root, &args.asset, source.as_str());
    let abs = session.project.root.join(source.as_str());
    if let Some(ext) = abs.extension().and_then(|e| e.to_str()) {
        match ext.to_ascii_lowercase().as_str() {
            "obj" | "gltf" | "glb" => {
                let sp = SourcePath::new(source.as_str()).map_err(|e| e.to_string())?;
                let mut inputs = voxy_assets::ImportInputs::new(
                    crate::import::MODEL_INPUTS,
                    crate::import::MODEL_INPUT_BYTES,
                );
                let provider = voxy_assets::FileInputs::new(&session.project.root)
                    .map_err(|e| e.to_string())?;
                if let Ok(imported) = crate::import::load_from(
                    &sp,
                    &|id, limit| provider.read(id, limit),
                    &mut inputs,
                ) {
                    row["model"] = json!({
                        "nodes": imported.nodes.len(),
                        "vertices": imported.mesh.vertices().len(),
                        "indices": imported.mesh.indices().len(),
                        "textures": imported.images.len(),
                        "animations": imported.animated.as_ref().map_or(0, |a| a.animations.len()),
                    });
                }
            }
            "json" if abs.exists() => {
                if let Ok(raw) = read_bounded(&abs, MAX_DOC_BYTES)
                    && let Ok(doc) = serde_json::from_slice::<PrefabSceneDocument>(&raw)
                {
                    row["scene"] = json!({
                        "objects": doc.objects.len(),
                        "prefabInstances": doc.instances.len(),
                    });
                }
            }
            _ => {}
        }
    }
    Ok(ToolOutcome::ok(row))
}

fn asset_register(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        asset: String,
        source: String,
        #[serde(default = "yes")]
        replace: bool,
    }
    const fn yes() -> bool {
        true
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let registry = state.registry.clone();
    let session = state.require_project()?;
    let root = session.project.root.clone();
    let descriptor_path = session.project.descriptor_path.clone();
    let manifest_path = session.manifest_path();
    let resolved = resolve_project_rel(&root, &args.source, true)?;
    let _ = resolved;
    let source_path = SourcePath::new(args.source.clone()).map_err(|e| format!("{e:?}"))?;
    let mut next = AssetLocations::new(
        crate::import::MANIFEST_ENTRIES,
        crate::import::MANIFEST_BYTES,
    );
    let mut updated = false;
    for (id, src) in session.project.assets.bindings() {
        if id.0 == args.asset {
            if !args.replace {
                return Err(format!("asset `{}` already exists", args.asset));
            }
            updated = true;
            next.bind(id.clone(), source_path.clone())
                .map_err(|e| format!("{e:?}"))?;
        } else {
            next.bind(id.clone(), src.clone())
                .map_err(|e| format!("{e:?}"))?;
        }
    }
    if !updated {
        next.bind(AssetId(args.asset.clone()), source_path)
            .map_err(|e| format!("{e:?}"))?;
    }
    save_manifest_and_reopen(state, &descriptor_path, &manifest_path, &next, &registry)?;
    Ok(ToolOutcome::ok(json!({
        "registered": args.asset,
        "source": args.source,
        "replaced": updated,
    }))
    .with_updates(&["voxy://project/assets"])
    .with_list_changed())
}

fn asset_remove(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        asset: String,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let registry = state.registry.clone();
    let session = state.require_project()?;
    let descriptor_path = session.project.descriptor_path.clone();
    let manifest_path = session.manifest_path();
    let mut next = AssetLocations::new(
        crate::import::MANIFEST_ENTRIES,
        crate::import::MANIFEST_BYTES,
    );
    let mut removed = false;
    for (id, src) in session.project.assets.bindings() {
        if id.0 == args.asset {
            removed = true;
        } else {
            next.bind(id.clone(), src.clone())
                .map_err(|e| format!("{e:?}"))?;
        }
    }
    if !removed {
        return Err(format!("unknown asset `{}`", args.asset));
    }
    save_manifest_and_reopen(state, &descriptor_path, &manifest_path, &next, &registry)?;
    Ok(ToolOutcome::ok(json!({ "removed": args.asset }))
        .with_updates(&["voxy://project/assets"])
        .with_list_changed())
}

fn save_manifest_and_reopen(
    state: &mut EngineState,
    descriptor_path: &Path,
    manifest_path: &Path,
    locations: &AssetLocations,
    registry: &ComponentRegistry,
) -> Result<(), String> {
    let encoded = locations
        .to_json(crate::import::MANIFEST_BYTES)
        .map_err(|e| format!("encode manifest: {e:?}"))?;
    voxy_assets::save_atomic_file(
        manifest_path,
        encoded.as_bytes(),
        crate::import::MANIFEST_BYTES,
    )
    .map_err(|e| format!("save manifest: {e}"))?;
    state.project = Some(open_project_session(descriptor_path, registry)?);
    Ok(())
}

pub(crate) fn asset_row(root: &Path, id: &str, source: &str) -> Value {
    let full = root.join(source);
    let ext = full
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let (exists, bytes, digest) = match fs::read(&full) {
        Ok(data) => (
            true,
            Some(data.len()),
            Some(blake3::hash(&data).to_hex().to_string()),
        ),
        Err(_) => (false, None, None),
    };
    json!({
        "asset": id,
        "source": source,
        "kind": ext,
        "exists": exists,
        "bytes": bytes,
        "blake3": digest,
    })
}
