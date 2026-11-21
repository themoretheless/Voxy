//! Scene authoring, component CRUD across all 30 schemas, history, save, and spatial queries.

use super::projects_assets::open_scene_session;
use super::schemas::filter_schemas;
use super::state::{
    EngineState, MAX_DOC_BYTES, SCENE_CAPACITY, ToolOutcome, json_merge_patch, read_bounded,
    resolve_project_rel,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;
use voxy_mcp::{ToolAnnotations, ToolDef};
use voxy_scene::{ComponentRegistry, SceneDocument, SceneObject, save_scene_file};

pub(crate) fn tool_defs() -> Vec<ToolDef> {
    let ro = ToolAnnotations {
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(true),
        open_world_hint: Some(false),
    };
    let rev_guard = json!({
        "type": "object",
        "properties": { "expected_revision": { "type": "integer", "minimum": 0 } },
        "required": ["expected_revision"],
        "additionalProperties": false
    });
    vec![
        ToolDef::new(
            "component_schemas",
            "List all 30 built-in Voxy component schemas with categories, descriptions, and valid JSON samples.",
            json!({
                "type": "object",
                "properties": {
                    "category": { "type": "string" },
                    "query": { "type": "string" }
                },
                "additionalProperties": false
            }),
        )
        .with_annotations(ro.clone()),
        ToolDef::new(
            "scene_get",
            "Read the active scene document, revision, dirty state, assets, and BLAKE3 sceneHash.",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        )
        .with_annotations(ro.clone()),
        ToolDef::new(
            "scene_open_file",
            "Switch active scene editing to any .scene.json file inside the open project.",
            json!({
                "type": "object",
                "properties": { "scene_path": { "type": "string" } },
                "required": ["scene_path"],
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "scene_apply",
            "Atomically apply 1..128 hierarchy, transform, duplicate, and component edits (component_set, component_patch, component_remove, create, duplicate, remove, reparent, rename, active, transform, material, camera, light).",
            json!({
                "type": "object",
                "properties": {
                    "expected_revision": { "type": "integer", "minimum": 0 },
                    "edits": { "type": "array", "minItems": 1, "maxItems": 128, "items": { "type": "object" } }
                },
                "required": ["expected_revision", "edits"],
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "scene_undo",
            "Undo the last atomic edit batch in scene history.",
            rev_guard.clone(),
        ),
        ToolDef::new(
            "scene_redo",
            "Redo the previously undone batch in scene history.",
            rev_guard.clone(),
        ),
        ToolDef::new(
            "scene_save",
            "Atomically save the active plain scene file (refuses external modifications or composed prefab flattening).",
            rev_guard,
        ),
        ToolDef::new(
            "scene_recover_autosave",
            "Recover or discard a detected crash-recovery autosave journal for the active scene.",
            json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["recover", "discard"] }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
        ),
        ToolDef::new(
            "scene_query",
            "Query scene objects by component schema, name substring, active status, or 3D sphere proximity with computed world positions.",
            json!({
                "type": "object",
                "properties": {
                    "schema": { "type": "string" },
                    "name_contains": { "type": "string" },
                    "active_only": { "type": "boolean" },
                    "center": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 },
                    "radius": { "type": "number", "minimum": 0 }
                },
                "additionalProperties": false
            }),
        )
        .with_annotations(ro),
    ]
}

pub(crate) fn dispatch(
    state: &mut EngineState,
    name: &str,
    arguments: Value,
) -> Result<ToolOutcome, String> {
    match name {
        "component_schemas" => component_schemas(arguments),
        "scene_get" => scene_get(state, arguments),
        "scene_open_file" => scene_open_file(state, arguments),
        "scene_apply" => scene_apply(state, arguments),
        "scene_undo" => scene_undo_redo(state, arguments, true),
        "scene_redo" => scene_undo_redo(state, arguments, false),
        "scene_save" => scene_save(state, arguments),
        "scene_recover_autosave" => scene_recover_autosave(state, arguments),
        "scene_query" => scene_query(state, arguments),
        _ => Err(format!("unknown scene tool: {name}")),
    }
}

fn component_schemas(arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        category: Option<String>,
        query: Option<String>,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let schemas = filter_schemas(args.category.as_deref(), args.query.as_deref());
    Ok(ToolOutcome::ok(json!({
        "count": schemas.len(),
        "schemas": schemas,
    })))
}

fn scene_get(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Empty {}
    let _: Empty = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    scene_snapshot_outcome(state)
}

pub(crate) fn scene_snapshot_outcome(state: &EngineState) -> Result<ToolOutcome, String> {
    let session = state.require_project()?;
    let assets: Vec<String> = session
        .project
        .assets
        .bindings()
        .map(|(id, _)| id.0.clone())
        .collect();
    Ok(ToolOutcome::ok(json!({
        "revision": session.revision,
        "dirty": session.dirty(),
        "scenePath": session.scene_rel.display().to_string(),
        "isPrefabComposed": session.is_prefab_composed,
        "hasAutosaveRecovery": session.pending_autosave.is_some(),
        "autosaveError": session.autosave_error,
        "sceneHash": session.scene_hash()?,
        "assets": assets,
        "scene": session.current_scene(),
    })))
}

fn scene_open_file(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        scene_path: String,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let registry = state.registry.clone();
    let current = state.require_project()?;
    let abs = resolve_project_rel(&current.project.root, &args.scene_path, true)?;
    let reopened_project =
        crate::project::open_for_editor(&current.project.descriptor_path, &registry)
            .map_err(|e| e.to_string())?;
    let next_session = open_scene_session(
        reopened_project,
        abs,
        PathBuf::from(&args.scene_path),
        &registry,
    )?;
    state.project = Some(next_session);
    Ok(scene_snapshot_outcome(state)?
        .with_updates(&["voxy://project/scene", "voxy://project/prefabs"]))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevGuard {
    expected_revision: u64,
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum SceneEditOp {
    Transform {
        node_id: String,
        translation: Option<[f32; 3]>,
        rotation: Option<[f32; 4]>,
        scale: Option<[f32; 3]>,
    },
    Rename {
        node_id: String,
        name: String,
    },
    Active {
        node_id: String,
        active: bool,
    },
    Create {
        object: SceneObject,
    },
    Duplicate {
        node_ids: Vec<String>,
        offset: Option<[f32; 3]>,
    },
    Remove {
        node_id: String,
    },
    Reparent {
        node_id: String,
        parent: Option<String>,
    },
    ComponentSet {
        node_id: String,
        schema: String,
        value: Value,
    },
    ComponentPatch {
        node_id: String,
        schema: String,
        patch: Value,
    },
    ComponentRemove {
        node_id: String,
        schema: String,
    },
    Material {
        node_id: String,
        tint: [f32; 4],
        lit: bool,
    },
    Camera {
        node_id: String,
        target: [f32; 3],
        distance: f32,
        yaw: f32,
        pitch: f32,
        perspective: bool,
    },
    Light {
        node_id: String,
        direction: [f32; 3],
        color: [f32; 3],
        intensity: f32,
    },
}

fn scene_apply(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        expected_revision: u64,
        edits: Vec<SceneEditOp>,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    if args.edits.is_empty() || args.edits.len() > 128 {
        return Err("edits must contain 1..=128 items".to_owned());
    }
    let registry = state.registry.clone();
    let session = state.require_project_mut()?;
    if args.expected_revision != session.revision {
        return Err(format!(
            "stale revision {}, expected {}",
            args.expected_revision, session.revision
        ));
    }
    let mut candidate = session.current_scene().clone();
    for op in args.edits {
        apply_single_op(&mut candidate, op, &registry)?;
    }
    validate_candidate(&candidate, &registry)?;
    session
        .history
        .commit(candidate, &registry)
        .map_err(|e| e.to_string())?;
    session.revision += 1;
    session.pending_autosave = None;
    session.autosave_error = None;
    sync_mcp_autosave(session)?;
    Ok(scene_snapshot_outcome(state)?.with_updates(&["voxy://project/scene"]))
}

fn sync_mcp_autosave(session: &super::state::ProjectSession) -> Result<(), String> {
    if !session.dirty() {
        let _ = crate::autosave::remove_journal(&session.scene_path);
        return Ok(());
    }
    let scene_file_name = session
        .scene_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("scene.json")
        .to_owned();
    let base_scene_blake3 = Some(blake3::hash(&session.disk_bytes).to_hex().to_string());
    let journal = crate::autosave::AutosaveJournal {
        version: 1,
        scene_file_name,
        base_scene_blake3,
        saved_at_unix_ms: crate::autosave::current_unix_ms(),
        payload: crate::autosave::AutosavePayload::Plain {
            document: session.current_scene().clone(),
        },
    };
    crate::autosave::write_journal(&session.scene_path, &journal, MAX_DOC_BYTES)?;
    Ok(())
}

fn find_obj<'a>(doc: &'a mut SceneDocument, id: &str) -> Result<&'a mut SceneObject, String> {
    doc.objects
        .iter_mut()
        .find(|o| o.id.0 == id)
        .ok_or_else(|| format!("unknown node: {id}"))
}

fn duplicate_op(
    doc: &mut SceneDocument,
    node_ids: Vec<String>,
    offset: Option<[f32; 3]>,
    registry: &ComponentRegistry,
) -> Result<(), String> {
    let selected_ids: Vec<voxy_scene::ObjectId> =
        node_ids.into_iter().map(voxy_scene::ObjectId).collect();
    let clip =
        voxy_scene::SceneClipboard::capture(doc, &selected_ids).map_err(|e| e.to_string())?;
    let mut next_id = 1_u64;
    let pasted = clip
        .paste(
            doc,
            registry,
            SCENE_CAPACITY,
            offset.unwrap_or([0.0; 3]),
            &mut next_id,
        )
        .map_err(|e| e.to_string())?;
    *doc = pasted.document;
    Ok(())
}

fn remove_subtree_op(doc: &mut SceneDocument, node_id: String) -> Result<(), String> {
    let _ = find_obj(doc, &node_id)?;
    let mut doomed = vec![node_id];
    let mut cursor = 0;
    while cursor < doomed.len() {
        let parent_id = doomed[cursor].clone();
        for obj in &doc.objects {
            if obj.parent.as_ref().is_some_and(|p| p.0 == parent_id) && !doomed.contains(&obj.id.0)
            {
                doomed.push(obj.id.0.clone());
            }
        }
        cursor += 1;
    }
    doc.objects.retain(|o| !doomed.contains(&o.id.0));
    Ok(())
}

fn apply_single_op(
    doc: &mut SceneDocument,
    op: SceneEditOp,
    registry: &ComponentRegistry,
) -> Result<(), String> {
    match op {
        SceneEditOp::Transform {
            node_id,
            translation,
            rotation,
            scale,
        } => {
            let obj = find_obj(doc, &node_id)?;
            if let Some(t) = translation {
                obj.translation = t;
            }
            if let Some(r) = rotation {
                obj.rotation = r;
            }
            if let Some(s) = scale {
                obj.scale = s;
            }
        }
        SceneEditOp::Rename { node_id, name } => find_obj(doc, &node_id)?.name = name,
        SceneEditOp::Active { node_id, active } => find_obj(doc, &node_id)?.active = active,
        SceneEditOp::Create { object } => doc.objects.push(object),
        SceneEditOp::Duplicate { node_ids, offset } => {
            duplicate_op(doc, node_ids, offset, registry)?;
        }
        SceneEditOp::Remove { node_id } => remove_subtree_op(doc, node_id)?,
        SceneEditOp::Reparent { node_id, parent } => {
            find_obj(doc, &node_id)?.parent = parent.map(voxy_scene::ObjectId);
        }
        SceneEditOp::ComponentSet {
            node_id,
            schema,
            value,
        } => {
            find_obj(doc, &node_id)?.components.insert(schema, value);
        }
        SceneEditOp::ComponentPatch {
            node_id,
            schema,
            patch,
        } => {
            let obj = find_obj(doc, &node_id)?;
            let entry = obj.components.entry(schema).or_insert_with(|| json!({}));
            json_merge_patch(entry, patch);
        }
        SceneEditOp::ComponentRemove { node_id, schema } => {
            find_obj(doc, &node_id)?.components.remove(&schema);
        }
        SceneEditOp::Material { node_id, tint, lit } => {
            find_obj(doc, &node_id)?.components.insert(
                "editor.material.v1".into(),
                json!({ "tint": tint, "lit": lit }),
            );
        }
        SceneEditOp::Camera {
            node_id,
            target,
            distance,
            yaw,
            pitch,
            perspective,
        } => {
            find_obj(doc, &node_id)?.components.insert(
                "editor.camera.v1".into(),
                json!({ "legacy": false, "target": target, "distance": distance, "yaw": yaw, "pitch": pitch, "perspective": perspective }),
            );
        }
        SceneEditOp::Light {
            node_id,
            direction,
            color,
            intensity,
        } => {
            find_obj(doc, &node_id)?.components.insert(
                "editor.light.v1".into(),
                json!({ "direction": direction, "color": color, "intensity": intensity }),
            );
        }
    }
    Ok(())
}

fn validate_candidate(doc: &SceneDocument, registry: &ComponentRegistry) -> Result<(), String> {
    let loaded = doc
        .load(registry, SCENE_CAPACITY)
        .map_err(|e| format!("scene validation failed: {e}"))?;
    for id in &doc.objects {
        let node = loaded.resolve(&id.id).ok_or("missing loaded node")?;
        let world = loaded.graph.world_matrix(node).map_err(|e| e.to_string())?;
        if !world.is_finite() {
            return Err(format!("non-finite world matrix on {}", id.id.0));
        }
    }
    Ok(())
}

fn scene_undo_redo(
    state: &mut EngineState,
    arguments: Value,
    undo: bool,
) -> Result<ToolOutcome, String> {
    let guard: RevGuard = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let session = state.require_project_mut()?;
    if guard.expected_revision != session.revision {
        return Err(format!(
            "stale revision {}, expected {}",
            guard.expected_revision, session.revision
        ));
    }
    let changed = if undo {
        session.history.undo()
    } else {
        session.history.redo()
    };
    if !changed {
        return Err(if undo {
            "nothing to undo".to_owned()
        } else {
            "nothing to redo".to_owned()
        });
    }
    session.revision += 1;
    sync_mcp_autosave(session)?;
    Ok(scene_snapshot_outcome(state)?.with_updates(&["voxy://project/scene"]))
}

fn scene_save(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    let guard: RevGuard = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let registry = state.registry.clone();
    let session = state.require_project_mut()?;
    if guard.expected_revision != session.revision {
        return Err(format!(
            "stale revision {}, expected {}",
            guard.expected_revision, session.revision
        ));
    }
    if session.is_prefab_composed {
        return Err(
            "active scene uses prefab composition; save via prefab_* tools to avoid flattening"
                .to_owned(),
        );
    }
    if read_bounded(&session.scene_path, MAX_DOC_BYTES)? != session.disk_bytes {
        return Err("scene file changed externally on disk; reopen before saving".to_owned());
    }
    let current = session.current_scene().clone();
    save_scene_file(
        &session.scene_path,
        &current,
        &registry,
        SCENE_CAPACITY,
        MAX_DOC_BYTES,
    )
    .map_err(|e| e.to_string())?;
    session.disk_bytes = read_bounded(&session.scene_path, MAX_DOC_BYTES)?;
    session.saved_scene = current;
    session.pending_autosave = None;
    session.autosave_error = None;
    let _ = crate::autosave::remove_journal(&session.scene_path);
    Ok(scene_snapshot_outcome(state)?.with_updates(&["voxy://project/scene"]))
}

fn scene_recover_autosave(
    state: &mut EngineState,
    arguments: Value,
) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        action: String,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let registry = state.registry.clone();
    let session = state.require_project_mut()?;
    match args.action.as_str() {
        "discard" => {
            session.pending_autosave = None;
            session.autosave_error = None;
            crate::autosave::remove_journal(&session.scene_path)?;
        }
        "recover" => {
            let recovery = session
                .pending_autosave
                .take()
                .ok_or_else(|| "no pending autosave recovery journal".to_owned())?;
            let candidate = recovery.document().clone();
            validate_candidate(&candidate, &registry)?;
            session
                .history
                .commit(candidate, &registry)
                .map_err(|e| e.to_string())?;
            session.revision += 1;
            session.autosave_error = None;
        }
        other => {
            return Err(format!(
                "expected action 'recover' or 'discard', got '{other}'"
            ));
        }
    }
    Ok(scene_snapshot_outcome(state)?.with_updates(&["voxy://project/scene"]))
}

fn scene_query(state: &mut EngineState, arguments: Value) -> Result<ToolOutcome, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Args {
        schema: Option<String>,
        name_contains: Option<String>,
        active_only: Option<bool>,
        center: Option<[f32; 3]>,
        radius: Option<f32>,
    }
    let args: Args = serde_json::from_value(arguments).map_err(|e| e.to_string())?;
    let session = state.require_project()?;
    let loaded = session
        .current_scene()
        .load(&state.registry, SCENE_CAPACITY)
        .map_err(|e| e.to_string())?;
    let active_only = args.active_only.unwrap_or(false);
    let mut matches = Vec::new();
    for obj in &session.current_scene().objects {
        if active_only && !obj.active {
            continue;
        }
        if let Some(schema) = &args.schema
            && !obj.components.contains_key(schema)
        {
            continue;
        }
        if let Some(substr) = &args.name_contains
            && !obj
                .name
                .to_ascii_lowercase()
                .contains(&substr.to_ascii_lowercase())
        {
            continue;
        }
        let world_pos = loaded
            .resolve(&obj.id)
            .and_then(|node| loaded.graph.world_matrix(node).ok())
            .map_or(obj.translation, |m| m.w_axis.truncate().to_array());
        if let (Some(c), Some(r)) = (args.center, args.radius) {
            let dx = world_pos[0] - c[0];
            let dy = world_pos[1] - c[1];
            let dz = world_pos[2] - c[2];
            if dx * dx + dy * dy + dz * dz > r * r {
                continue;
            }
        }
        matches.push(json!({
            "id": obj.id.0,
            "name": obj.name,
            "parent": obj.parent.as_ref().map(|p| &p.0),
            "active": obj.active,
            "worldPosition": world_pos,
            "schemas": obj.components.keys().collect::<Vec<_>>(),
        }));
    }
    Ok(ToolOutcome::ok(json!({
        "count": matches.len(),
        "objects": matches,
    })))
}
