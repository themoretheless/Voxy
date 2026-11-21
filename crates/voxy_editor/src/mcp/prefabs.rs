//! Non-destructive prefab authoring tools preserving instances and overrides.

use super::projects_assets::open_scene_session;
use super::state::{EngineState, MAX_DOC_BYTES, ProjectSession, ToolOutcome, resolve_project_rel};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use voxy_mcp::{ToolAnnotations, ToolDef};
use voxy_scene::{
    ComponentRegistry, DocumentError, ObjectId, ObjectOverride, PrefabInstance,
    PrefabSceneDocument, has_parent_override_cycle, read_prefab_scene_file, save_prefab_scene_file,
};

pub(crate) fn tool_defs() -> Vec<ToolDef> {
    let ro = ToolAnnotations {
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(true),
        open_world_hint: Some(false),
    };
    vec![
        ToolDef::new(
            "prefab_scene_get",
            "Read raw PrefabSceneDocument (instances + overrides preserved) and its expanded scene preview.",
            json!({
                "type": "object",
                "properties": { "scene_path": { "type": "string" } },
                "additionalProperties": false
            }),
        )
        .with_annotations(ro),
        ToolDef::new(
            "prefab_instance_add",
            "Instantiate a prefab asset ID non-destructively into a scene file and save atomically.",
            json!({
                "type": "object",
                "properties": {
                    "scene_path": { "type": "string" },
                    "id": { "type": "string" },
                    "asset": { "type": "string" },
                    "parent": { "type": ["string", "null"] },
                    "overrides": { "type": "object" }
                },
                "required": ["id", "asset"],
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "prefab_instance_remove",
            "Remove a prefab instance by ID from a scene file without touching other instances.",
            json!({
                "type": "object",
                "properties": {
                    "scene_path": { "type": "string" },
                    "id": { "type": "string" }
                },
                "required": ["id"],
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "prefab_override_set",
            "Set or replace an ObjectOverride on a target object inside a prefab instance.",
            json!({
                "type": "object",
                "properties": {
                    "scene_path": { "type": "string" },
                    "instance_id": { "type": "string" },
                    "target_id": { "type": "string" },
                    "override": { "type": "object" }
                },
                "required": ["instance_id", "target_id", "override"],
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "prefab_override_clear",
            "Clear a single target ObjectOverride or all overrides on a prefab instance.",
            json!({
                "type": "object",
                "properties": {
                    "scene_path": { "type": "string" },
                    "instance_id": { "type": "string" },
                    "target_id": { "type": "string" }
                },
                "required": ["instance_id"],
                "additionalProperties": false
            }),
        ),
    ]
}

pub(crate) fn dispatch(
    state: &mut EngineState,
    name: &str,
    arguments: Value,
) -> Result<ToolOutcome, String> {
    match name {
        "prefab_scene_get" => prefab_scene_get(state, arguments),
        "prefab_instance_add" => prefab_instance_add(state, arguments),
        "prefab_instance_remove" => prefab_instance_remove(state, arguments),
        "prefab_override_set" => prefab_override_set(state, arguments),
        "prefab_override_clear" => prefab_override_clear(state, arguments),
        _ => Err(format!("unknown prefab tool: {name}")),
    }
}

pub(crate) fn read_active_prefab_doc(
    session: &ProjectSession,
    registry: &ComponentRegistry,
    scene_path_override: Option<&str>,
) -> Result<(PathBuf, PrefabSceneDocument, voxy_scene::SceneDocument), String> {
    let target = match scene_path_override {
        Some(rel) => resolve_project_rel(&session.project.root, rel, true)?,
        None => session.scene_path.clone(),
    };
    let doc = read_prefab_scene_file(&target, MAX_DOC_BYTES).map_err(|e| e.to_string())?;
    let root = session.project.root.clone();
    let asset_map = asset_source_map(session);
    let expanded = doc
        .expand(registry, crate::scene_limits::PREFABS, |asset_id| {
            resolve_prefab_asset(&root, &asset_map, asset_id)
        })
        .map_err(|e| e.to_string())?;
    Ok((target, doc, expanded.document))
}

fn asset_source_map(session: &ProjectSession) -> BTreeMap<String, String> {
    session
        .project
        .assets
        .bindings()
        .map(|(id, src)| (id.0.clone(), src.as_str().to_owned()))
        .collect()
}

fn resolve_prefab_asset(
    root: &Path,
    asset_map: &BTreeMap<String, String>,
    asset_id: &str,
) -> Result<PrefabSceneDocument, DocumentError> {
    let source = asset_map
        .get(asset_id)
        .ok_or_else(|| DocumentError::Invalid(format!("unknown prefab asset {asset_id}")))?;
    let bytes = fs::read(root.join(source))
        .map_err(|e| DocumentError::Invalid(format!("read prefab {asset_id}: {e}")))?;
    serde_json::from_slice(&bytes).map_err(Into::into)
}

fn save_and_refresh(
    state: &mut EngineState,
    target_path: &Path,
    doc: &PrefabSceneDocument,
) -> Result<(), String> {
    let registry = state.registry.clone();
    let (descriptor_path, scene_rel) = {
        let session = state.require_project()?;
        let root = session.project.root.clone();
        let asset_map = asset_source_map(session);
        save_prefab_scene_file(
            target_path,
            doc,
            &registry,
            crate::scene_limits::PREFABS,
            MAX_DOC_BYTES,
            |asset_id| resolve_prefab_asset(&root, &asset_map, asset_id),
        )
        .map_err(|e| format!("save prefab scene: {e}"))?;
        (
            session.project.descriptor_path.clone(),
            session.scene_rel.clone(),
        )
    };
    let active_matches = state
        .project
        .as_ref()
        .is_some_and(|p| p.scene_path == target_path);
    if active_matches {
        let reopened = crate::project::open_for_editor(&descriptor_path, &registry)
            .map_err(|e| e.to_string())?;
        state.project = Some(open_scene_session(
            reopened,
            target_path.to_path_buf(),
            scene_rel,
            &registry,
        )?);
    }
    Ok(())
}

fn prefab_scene_get(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        scene_path: Option<String>,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let session = state.require_project()?;
    let (target, doc, expanded) =
        read_active_prefab_doc(session, &state.registry, args.scene_path.as_deref())?;
    Ok(ToolOutcome::ok(json!({
        "scenePath": target.display().to_string(),
        "authored": doc,
        "expanded": expanded,
    })))
}

fn prefab_instance_add(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        scene_path: Option<String>,
        id: String,
        asset: String,
        #[serde(default)]
        parent: Option<String>,
        #[serde(default)]
        overrides: BTreeMap<ObjectId, ObjectOverride>,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let (target, mut doc, _) = {
        let session = state.require_project()?;
        read_active_prefab_doc(session, &state.registry, args.scene_path.as_deref())?
    };
    if doc.instances.iter().any(|i| i.id.0 == args.id) {
        return Err(format!("duplicate prefab instance ID: {}", args.id));
    }
    if has_parent_override_cycle(&args.overrides) {
        return Err("parent override cycle detected".to_owned());
    }
    doc.instances.push(PrefabInstance {
        id: ObjectId(args.id.clone()),
        asset: args.asset.clone(),
        parent: args.parent.map(ObjectId),
        overrides: args.overrides,
    });
    save_and_refresh(state, &target, &doc)?;
    Ok(ToolOutcome::ok(json!({
        "addedInstance": args.id,
        "asset": args.asset,
        "instanceCount": doc.instances.len(),
    }))
    .with_updates(&["voxy://project/prefabs", "voxy://project/scene"]))
}

fn prefab_instance_remove(
    state: &mut EngineState,
    arguments: Value,
) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        scene_path: Option<String>,
        id: String,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let (target, mut doc, _) = {
        let session = state.require_project()?;
        read_active_prefab_doc(session, &state.registry, args.scene_path.as_deref())?
    };
    let before = doc.instances.len();
    doc.instances.retain(|i| i.id.0 != args.id);
    if doc.instances.len() == before {
        return Err(format!("unknown prefab instance ID: {}", args.id));
    }
    save_and_refresh(state, &target, &doc)?;
    Ok(ToolOutcome::ok(json!({
        "removedInstance": args.id,
        "instanceCount": doc.instances.len(),
    }))
    .with_updates(&["voxy://project/prefabs", "voxy://project/scene"]))
}

fn prefab_override_set(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        scene_path: Option<String>,
        instance_id: String,
        target_id: String,
        #[serde(rename = "override")]
        patch: ObjectOverride,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let (target, mut doc, _) = {
        let session = state.require_project()?;
        read_active_prefab_doc(session, &state.registry, args.scene_path.as_deref())?
    };
    let instance = doc
        .instances
        .iter_mut()
        .find(|i| i.id.0 == args.instance_id)
        .ok_or_else(|| format!("unknown prefab instance: {}", args.instance_id))?;
    instance
        .overrides
        .insert(ObjectId(args.target_id.clone()), args.patch);
    if has_parent_override_cycle(&instance.overrides) {
        return Err("parent override cycle detected".to_owned());
    }
    save_and_refresh(state, &target, &doc)?;
    Ok(ToolOutcome::ok(json!({
        "instanceId": args.instance_id,
        "targetId": args.target_id,
    }))
    .with_updates(&["voxy://project/prefabs", "voxy://project/scene"]))
}

fn prefab_override_clear(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        scene_path: Option<String>,
        instance_id: String,
        target_id: Option<String>,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let (target, mut doc, _) = {
        let session = state.require_project()?;
        read_active_prefab_doc(session, &state.registry, args.scene_path.as_deref())?
    };
    let instance = doc
        .instances
        .iter_mut()
        .find(|i| i.id.0 == args.instance_id)
        .ok_or_else(|| format!("unknown prefab instance: {}", args.instance_id))?;
    if let Some(target_id) = &args.target_id {
        if instance
            .overrides
            .remove(&ObjectId(target_id.clone()))
            .is_none()
        {
            return Err(format!("no override present for target {target_id}"));
        }
    } else {
        instance.overrides.clear();
    }
    save_and_refresh(state, &target, &doc)?;
    Ok(ToolOutcome::ok(json!({
        "clearedInstance": args.instance_id,
        "targetId": args.target_id,
    }))
    .with_updates(&["voxy://project/prefabs", "voxy://project/scene"]))
}
