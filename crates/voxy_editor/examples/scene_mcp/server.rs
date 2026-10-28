use base64::Engine as _;
use image::ImageEncoder as _;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    io::Read as _,
    path::{Path, PathBuf},
};
use voxy_scene::{ComponentRegistry, SceneDocument, SceneHistory, SceneObject};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const LIMIT: usize = 8 * 1024 * 1024;
const CAPACITY: usize = 512;

pub struct Server {
    path: PathBuf,
    scene_path: PathBuf,
    manifest_path: PathBuf,
    registry: ComponentRegistry,
    history: SceneHistory,
    saved_scene: SceneDocument,
    disk: Vec<u8>,
    descriptor: Vec<u8>,
    manifest: Vec<u8>,
    assets: BTreeSet<String>,
    revision: u64,
    initialized: bool,
}
fn read(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > LIMIT {
        return Err("file exceeds 8 MiB".into());
    }
    Ok(bytes)
}
fn digest(scene: &SceneDocument) -> Result<String> {
    Ok(blake3::hash(&serde_json::to_vec(scene)?)
        .to_hex()
        .to_string())
}
fn decode<T: serde::de::DeserializeOwned>(v: Value) -> Result<T> {
    Ok(serde_json::from_value(v)?)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Guard {
    expected_revision: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Apply {
    expected_revision: u64,
    edits: Vec<Edit>,
}
#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Edit {
    Transform {
        node_id: String,
        translation: Option<[f32; 3]>,
        rotation: Option<[f32; 4]>,
        scale: Option<[f32; 3]>,
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
    Create {
        object: SceneObject,
    },
    Remove {
        node_id: String,
    },
    Reparent {
        node_id: String,
        parent: Option<String>,
    },
    Active {
        node_id: String,
        active: bool,
    },
}
fn object<'a>(scene: &'a mut SceneDocument, id: &str) -> Result<&'a mut SceneObject> {
    scene
        .objects
        .iter_mut()
        .find(|o| o.id.0 == id)
        .ok_or_else(|| format!("unknown node: {id}").into())
}
impl Server {
    pub fn open(path: &Path) -> Result<Self> {
        if !path.is_absolute() {
            return Err("project path must be absolute".into());
        }
        let path = path.canonicalize()?;
        let registry = voxy_editor::editor_component_registry()?;
        let project = voxy_editor::project::open(&path, &registry)?;
        let scene_path = project
            .root
            .join(&project.descriptor.startup_scene)
            .canonicalize()?;
        let manifest_path = project
            .root
            .join(&project.descriptor.asset_manifest)
            .canonicalize()?;
        let disk = read(&scene_path)?;
        // Refuse composed prefab documents: saving an expanded view would lose authoring data.
        let scene: SceneDocument = serde_json::from_slice(&disk)?;
        let assets = project
            .assets
            .bindings()
            .map(|(id, _)| id.0.clone())
            .collect();
        let history = SceneHistory::new(scene.clone(), &registry, CAPACITY, 32, 8 * LIMIT)?;
        let result = Self {
            descriptor: read(&path)?,
            manifest: read(&manifest_path)?,
            path,
            scene_path,
            manifest_path,
            registry,
            history,
            saved_scene: scene,
            disk,
            assets,
            revision: 0,
            initialized: false,
        };
        result.validate(result.history.current())?;
        Ok(result)
    }
    fn guard(&self, revision: u64) -> Result<()> {
        if revision != self.revision {
            return Err(format!(
                "stale revision {revision}; current is {}; call scene_get",
                self.revision
            )
            .into());
        }
        Ok(())
    }
    fn check_project(&self) -> Result<()> {
        if read(&self.path)? != self.descriptor || read(&self.manifest_path)? != self.manifest {
            return Err(
                "project descriptor or manifest changed externally; reopen the MCP session".into(),
            );
        }
        Ok(())
    }
    fn validate(&self, scene: &SceneDocument) -> Result<()> {
        if serde_json::to_vec(scene)?.len() > LIMIT {
            return Err("scene exceeds 8 MiB".into());
        }
        let loaded = scene.load(&self.registry, CAPACITY)?;
        for o in &scene.objects {
            // Force world transform composition too, catching ancestor overflow.
            loaded
                .graph
                .world_matrix(loaded.resolve(&o.id).ok_or("missing node")?)?;
            if let Some(v) = o.components.get("editor.model.v1") {
                if !v.as_str().is_some_and(|id| self.assets.contains(id)) {
                    return Err("unknown model asset".into());
                }
            }
            if let Some(v) = o.components.get("editor.material.v1") {
                let m: voxy_editor::SceneMaterial = decode(v.clone())?;
                if m.tint[3] != 1.
                    || !m
                        .tint
                        .iter()
                        .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
                {
                    return Err("material requires finite [0,1] RGB and alpha=1".into());
                }
            }
            if let Some(v) = o.components.get("editor.camera.v1") {
                let _: voxy_editor::EditorCamera = decode(v.clone())?;
                let target: [f32; 3] = decode(v["target"].clone())?;
                let distance = v["distance"].as_f64().ok_or("camera distance")?;
                let yaw = v["yaw"].as_f64().ok_or("camera yaw")?;
                let pitch = v["pitch"].as_f64().ok_or("camera pitch")?;
                if !target.iter().all(|x| x.is_finite() && x.abs() <= 1e6)
                    || !distance.is_finite()
                    || !(0.01..=10000.).contains(&distance)
                    || !yaw.is_finite()
                    || !pitch.is_finite()
                    || pitch.abs() > 1.5
                {
                    return Err("invalid camera bounds".into());
                }
            }
        }
        Ok(())
    }
    fn state(&self) -> Result<Value> {
        Ok(
            json!({"project":self.path,"revision":self.revision,"sceneHash":digest(self.history.current())?,"dirty":self.history.current()!=&self.saved_scene,"scene":self.history.current(),"liveEditorApplied":null,"session":"headless authoring; explicit save; offscreen render"}),
        )
    }
    fn apply(&mut self, args: Apply) -> Result<Value> {
        self.guard(args.expected_revision)?;
        if args.edits.is_empty() || args.edits.len() > 128 {
            return Err("apply requires 1..128 edits".into());
        }
        let mut candidate = self.history.current().clone();
        for edit in args.edits {
            match edit {
                Edit::Transform {
                    node_id,
                    translation,
                    rotation,
                    scale,
                } => {
                    let o = object(&mut candidate, &node_id)?;
                    if let Some(v) = translation {
                        o.translation = v;
                    }
                    if let Some(v) = rotation {
                        o.rotation = v;
                    }
                    if let Some(v) = scale {
                        o.scale = v;
                    }
                }
                Edit::Material { node_id, tint, lit } => {
                    object(&mut candidate, &node_id)?
                        .components
                        .insert("editor.material.v1".into(), json!({"tint":tint,"lit":lit}));
                }
                Edit::Camera {
                    node_id,
                    target,
                    distance,
                    yaw,
                    pitch,
                    perspective,
                } => {
                    object(&mut candidate,&node_id)?.components.insert("editor.camera.v1".into(),json!({"legacy":false,"perspective":perspective,"target":target,"distance":distance,"yaw":yaw,"pitch":pitch}));
                }
                Edit::Create { object } => candidate.objects.push(object),
                Edit::Reparent { node_id, parent } => {
                    object(&mut candidate, &node_id)?.parent = parent.map(voxy_scene::ObjectId);
                }
                Edit::Active { node_id, active } => {
                    object(&mut candidate, &node_id)?.active = active;
                }
                Edit::Remove { node_id } => {
                    object(&mut candidate, &node_id)?;
                    let mut removed = BTreeSet::from([node_id]);
                    loop {
                        let before = removed.len();
                        for o in &candidate.objects {
                            if o.parent.as_ref().is_some_and(|p| removed.contains(&p.0)) {
                                removed.insert(o.id.0.clone());
                            }
                        }
                        if removed.len() == before {
                            break;
                        }
                    }
                    candidate.objects.retain(|o| !removed.contains(&o.id.0));
                }
            }
        }
        self.validate(&candidate)?;
        if self.history.commit(candidate, &self.registry)? {
            self.revision += 1;
        }
        self.state()
    }
    fn snapshot(&self) -> Result<Value> {
        self.check_project()?;
        let scene = self.history.current();
        let loaded = scene.load(&self.registry, CAPACITY)?;
        let cameras: Vec<_> = scene
            .objects
            .iter()
            .filter(|o| {
                o.components.contains_key("editor.camera.v1")
                    && loaded
                        .graph
                        .active_in_hierarchy(loaded.resolve(&o.id).unwrap())
                        .unwrap_or(false)
            })
            .collect();
        if cameras.len() != 1 || cameras[0].components["editor.camera.v1"]["legacy"] != false {
            return Err("render requires exactly one active non-legacy camera".into());
        }
        for o in &scene.objects {
            for key in o.components.keys() {
                if !["editor.model.v1", "editor.material.v1", "editor.camera.v1"]
                    .contains(&key.as_str())
                {
                    return Err(
                        format!("offscreen OBJ preview does not support component {key}").into(),
                    );
                }
            }
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("voxy-scene-render-{}-{stamp}", std::process::id()));
        fs::create_dir(&directory)?;
        // Render a private frozen project, including mesh bytes. No live-file rereads
        // can change the scene or geometry represented by this receipt.
        let project = voxy_editor::project::open(&self.path, &self.registry)?;
        let used: BTreeSet<_> = scene
            .objects
            .iter()
            .filter_map(|o| o.components.get("editor.model.v1").and_then(Value::as_str))
            .collect();
        let mut assets = Vec::new();
        let mut hashes = serde_json::Map::new();
        let mut total = 0;
        for (id, source) in project.assets.bindings() {
            if !used.contains(id.0.as_str()) {
                continue;
            }
            let source = project.root.join(source.as_str()).canonicalize()?;
            if !source.starts_with(&project.root)
                || source.extension().and_then(|s| s.to_str()) != Some("obj")
            {
                return Err("offscreen preview supports project-local OBJ only".into());
            }
            let bytes = read(&source)?;
            total += bytes.len();
            if total > 8 * LIMIT {
                return Err("snapshot mesh budget exceeded".into());
            }
            let filename = format!("mesh-{}.obj", assets.len());
            fs::write(directory.join(&filename), &bytes)?;
            hashes.insert(
                id.0.clone(),
                json!(blake3::hash(&bytes).to_hex().to_string()),
            );
            assets.push(json!({"asset":id.0,"source":filename}));
        }
        self.check_project()?;
        fs::write(
            directory.join("assets.json"),
            serde_json::to_vec(&json!({"version":1,"assets":assets}))?,
        )?;
        fs::write(directory.join("scene.json"), scene.to_json()?)?;
        fs::write(
            directory.join("project.voxy"),
            serde_json::to_vec(
                &json!({"version":1,"name":"Frozen MCP render","asset_manifest":"assets.json","startup_scene":"scene.json"}),
            )?,
        )?;
        let rgb = super::render::render(&directory.join("project.voxy"), None)?;
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png).write_image(
            &rgb,
            1024,
            1024,
            image::ExtendedColorType::Rgb8,
        )?;
        let path = directory.join("render.png");
        fs::write(&path, &png)?;
        fs::write(directory.join("scene.json"), scene.to_json()?)?;
        let metadata = json!({"path":path,"revision":self.revision,"sceneHash":digest(scene)?,"meshHashes":hashes,"capture":"offscreen","width":1024,"height":1024,"liveEditorApplied":null,"supportedGeometry":"project-local OBJ","material":"editor Lambert tint, not PBR"});
        fs::write(
            directory.join("receipt.json"),
            serde_json::to_vec_pretty(&metadata)?,
        )?;
        Ok(
            json!({"content":[{"type":"text","text":metadata.to_string()},{"type":"image","mimeType":"image/png","data":base64::engine::general_purpose::STANDARD.encode(&png)}],"structuredContent":metadata,"isError":false}),
        )
    }
    fn call(&mut self, params: &Value) -> Result<Value> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Call {
            #[serde(default)]
            _meta: Option<Value>,
            name: String,
            #[serde(default = "empty")]
            arguments: Value,
        }
        fn empty() -> Value {
            json!({})
        }
        let call: Call = decode(params.clone())?;
        let value = match call.name.as_str() {
            "scene_get" => {
                let _: Empty = decode(call.arguments)?;
                self.state()?
            }
            "scene_apply" => self.apply(decode(call.arguments)?)?,
            "scene_undo" | "scene_redo" => {
                let args: Guard = decode(call.arguments)?;
                self.guard(args.expected_revision)?;
                let changed = if call.name == "scene_undo" {
                    self.history.undo()
                } else {
                    self.history.redo()
                };
                if changed {
                    self.revision += 1;
                }
                self.state()?
            }
            "scene_save" => {
                let args: Guard = decode(call.arguments)?;
                self.guard(args.expected_revision)?;
                self.check_project()?;
                if read(&self.scene_path)? != self.disk {
                    return Err(
                        "scene changed externally; save refused; reopen the MCP session".into(),
                    );
                }
                self.validate(self.history.current())?;
                voxy_scene::save_scene_file(
                    &self.scene_path,
                    self.history.current(),
                    &self.registry,
                    CAPACITY,
                    LIMIT,
                )?;
                self.disk = read(&self.scene_path)?;
                self.saved_scene = self.history.current().clone();
                self.state()?
            }
            "scene_render" => {
                let args: Guard = decode(call.arguments)?;
                self.guard(args.expected_revision)?;
                return self.snapshot();
            }
            _ => return Err("unknown scene tool".into()),
        };
        Ok(
            json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":value,"isError":false}),
        )
    }
    pub fn dispatch(&mut self, request: &Value) -> Value {
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let error = |code, message: &str| json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}});
        if request["jsonrpc"] != "2.0"
            || !request["method"].is_string()
            || !(id.is_string() || id.is_number() || id.is_null())
        {
            return error(-32600, "Invalid Request");
        }
        let params = request.get("params").cloned().unwrap_or(json!({}));
        let result = match request["method"].as_str().unwrap() {
            "initialize" => {
                let requested = params["protocolVersion"].as_str().unwrap_or("2025-06-18");
                let version = if ["2024-11-05", "2025-03-26", "2025-06-18"].contains(&requested) {
                    requested
                } else {
                    "2025-06-18"
                };
                self.initialized = true;
                json!({"protocolVersion":version,"capabilities":{"tools":{}},"serverInfo":{"name":"voxy-scene","version":"0.1.0"},"instructions":"Call scene_get first. Use its expected_revision for atomic scene_apply, undo/redo, save and render. Edits stay in session memory until scene_save; do not exit with unsaved work. scene_render returns an inline PNG and exact scene/mesh hashes from a frozen OBJ project. No live editor-window synchronization. Materials support opaque tint and Lambert lighting; unsupported render components fail explicitly."})
            }
            "ping" => json!({}),
            "tools/list" if self.initialized => tools(),
            "tools/call" if self.initialized => match self.call(&params) {
                Ok(v) => v,
                Err(e) => json!({"content":[{"type":"text","text":e.to_string()}],"isError":true}),
            },
            "tools/list" | "tools/call" => return error(-32000, "Initialize first"),
            _ => return error(-32601, "Method not found"),
        };
        json!({"jsonrpc":"2.0","id":id,"result":result})
    }
}

fn tools() -> Value {
    let vector = |n| json!({"type":"array","items":{"type":"number"},"minItems":n,"maxItems":n});
    let id = json!({"type":"string","minLength":1});
    let op = |tag: &str, props: Value, required: Vec<&str>| {
        let mut p = props.as_object().unwrap().clone();
        p.insert("op".into(), json!({"const":tag}));
        let mut req = vec!["op"];
        req.extend(required);
        json!({"type":"object","properties":p,"required":req,"additionalProperties":false})
    };
    let edits = json!({"type":"array","minItems":1,"maxItems":128,"items":{"oneOf":[
        op("transform",json!({"node_id":id,"translation":vector(3),"rotation":vector(4),"scale":vector(3)}),vec!["node_id"]),
        op("material",json!({"node_id":id,"tint":vector(4),"lit":{"type":"boolean"}}),vec!["node_id","tint","lit"]),
        op("camera",json!({"node_id":id,"target":vector(3),"distance":{"type":"number","minimum":0.01,"maximum":10000},"yaw":{"type":"number"},"pitch":{"type":"number","minimum":-1.5,"maximum":1.5},"perspective":{"type":"boolean"}}),vec!["node_id","target","distance","yaw","pitch","perspective"]),
        op("create",json!({"object":{"type":"object","description":"Native SceneObject: id,parent,name,active,translation,rotation,scale,components"}}),vec!["object"]),
        op("remove",json!({"node_id":id}),vec!["node_id"]),
        op("reparent",json!({"node_id":id,"parent":{"type":["string","null"]}}),vec!["node_id","parent"]),
        op("active",json!({"node_id":id,"active":{"type":"boolean"}}),vec!["node_id","active"])
    ]}});
    let guard = json!({"type":"object","properties":{"expected_revision":{"type":"integer","minimum":0}},"required":["expected_revision"],"additionalProperties":false});
    let mut tools = vec![
        json!({"name":"scene_get","description":"Read session scene, revision, dirty state and scene hash. Does not imply application in a live editor.","inputSchema":{"type":"object","properties":{},"additionalProperties":false},"annotations":{"readOnlyHint":true}}),
        json!({"name":"scene_apply","description":"Apply 1..128 scene edits atomically in session memory; validates hierarchy and components. remove removes subtree. Save explicitly.","inputSchema":{"type":"object","properties":{"expected_revision":{"type":"integer","minimum":0},"edits":edits},"required":["expected_revision","edits"],"additionalProperties":false}}),
    ];
    for (name, description) in [
        ("scene_undo", "Undo one atomic batch in memory."),
        ("scene_redo", "Redo one batch in memory."),
        (
            "scene_save",
            "Atomically save to the selected project's startup scene; refuse observed external changes.",
        ),
        (
            "scene_render",
            "Render current unsaved session scene to PNG through Voxy/Metal. Returns inline image, scene hash and local path. Local OBJ only; offscreen, no live-window claim.",
        ),
    ] {
        tools.push(json!({"name":name,"description":description,"inputSchema":guard,"annotations":{"readOnlyHint":name=="scene_render"}}));
    }
    json!({"tools":tools})
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        root: PathBuf,
        server: Server,
    }
    impl Fixture {
        fn new() -> Self {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir()
                .join(format!("voxy-scene-test-{}-{stamp}", std::process::id()));
            fs::create_dir(&root).unwrap();
            fs::write(root.join("project.voxy"),r#"{"version":1,"name":"MCP test","asset_manifest":"assets.json","startup_scene":"main.scene.json"}"#).unwrap();
            fs::write(
                root.join("assets.json"),
                r#"{"version":1,"assets":[{"asset":"triangle","source":"triangle.obj"}]}"#,
            )
            .unwrap();
            fs::write(
                root.join("triangle.obj"),
                "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            )
            .unwrap();
            let scene = json!({"version":1,"objects":[{"id":"root","parent":null,"name":"root","active":true,"translation":[0,0,0],"rotation":[0,0,0,1],"scale":[1,1,1],"components":{"editor.model.v1":"triangle"}}]});
            fs::write(root.join("main.scene.json"), scene.to_string()).unwrap();
            let server = Server::open(&root.join("project.voxy")).unwrap();
            Self { root, server }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
    #[test]
    fn batch_failure_is_atomic_and_save_detects_external_edits() {
        let mut f = Fixture::new();
        let before = f.server.state().unwrap();
        assert!(f.server.call(&json!({"name":"scene_apply","arguments":{"expected_revision":0,"edits":[{"op":"transform","node_id":"root","translation":[3,0,0]},{"op":"material","node_id":"root","tint":[2,0,0,1],"lit":true}]}})).is_err());
        assert_eq!(before, f.server.state().unwrap());
        f.server.call(&json!({"name":"scene_apply","arguments":{"expected_revision":0,"edits":[{"op":"material","node_id":"root","tint":[0.2,0.4,0.6,1],"lit":false}]}})).unwrap();
        assert!(f.server.state().unwrap()["dirty"].as_bool().unwrap());
        assert!(
            f.server
                .call(&json!({"name":"scene_save","arguments":{"expected_revision":0}}))
                .is_err()
        );
        fs::write(&f.server.scene_path, b"externally edited").unwrap();
        assert!(
            f.server
                .call(&json!({"name":"scene_save","arguments":{"expected_revision":1}}))
                .is_err()
        );
        assert_eq!(
            fs::read(&f.server.scene_path).unwrap(),
            b"externally edited"
        );
    }
    #[test]
    fn undo_redo_and_cycle_rejection_use_native_history() {
        let mut f = Fixture::new();
        let initial = f.server.history.current().clone();
        f.server.call(&json!({"name":"scene_apply","arguments":{"expected_revision":0,"edits":[{"op":"transform","node_id":"root","translation":[3,0,0]}]}})).unwrap();
        f.server
            .call(&json!({"name":"scene_undo","arguments":{"expected_revision":1}}))
            .unwrap();
        assert_eq!(&initial, f.server.history.current());
        assert_eq!(f.server.revision, 2);
        f.server
            .call(&json!({"name":"scene_redo","arguments":{"expected_revision":2}}))
            .unwrap();
        let before = f.server.state().unwrap();
        assert!(f.server.call(&json!({"name":"scene_apply","arguments":{"expected_revision":3,"edits":[{"op":"reparent","node_id":"root","parent":"root"}]}})).is_err());
        assert_eq!(before, f.server.state().unwrap());
        f.server
            .call(&json!({"name":"scene_save","arguments":{"expected_revision":3}}))
            .unwrap();
        let reopened = Server::open(&f.root.join("project.voxy")).unwrap();
        assert_eq!(reopened.history.current(), f.server.history.current());
    }
    #[test]
    fn protocol_initialization_and_unknown_fields() {
        let mut f = Fixture::new();
        assert_eq!(
            f.server
                .dispatch(&json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))["error"]["code"],
            -32000
        );
        f.server.dispatch(&json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}));
        assert_eq!(
            f.server
                .dispatch(&json!({"jsonrpc":"2.0","id":3,"method":"tools/list"}))["result"]["tools"]
                .as_array()
                .unwrap()
                .len(),
            6
        );
        assert!(
            f.server
                .call(&json!({"name":"scene_get","arguments":{"unexpected":true}}))
                .is_err()
        );
        assert!(f.server.call(&json!({"name":"scene_apply","arguments":{"expected_revision":0,"edits":[{"op":"camera","node_id":"root","target":[0,0,0],"distance":0,"yaw":0,"pitch":0,"perspective":true}]}})).is_err());
    }
}
