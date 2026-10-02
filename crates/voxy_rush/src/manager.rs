//! Persistent, thread-local object scripts and transactional handler commands.
use glam::{Quat, Vec3};
use rush::{
    CancellationToken, ExecutionLimits, HostObject, HostRegistration, OwnedScriptInstance,
    RuntimeError, ScriptState, Value, ValueType,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
};
use voxy_input::InputMap;
use voxy_scene::{NodeId, SceneGraph, SceneGraphError, Transform};

#[derive(Debug)]
pub struct ScriptDiagnostic {
    pub file: PathBuf,
    pub handler: String,
    pub error: RuntimeError,
}
impl std::fmt::Display for ScriptDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (line, column) = self
            .error
            .location
            .as_ref()
            .map_or((1, 1), |p| (p.line, p.column));
        write!(
            f,
            "{}:{line}:{column}: {} [{}]",
            self.file.display(),
            self.error.message,
            self.handler
        )?;
        for frame in &self.error.stack {
            write!(
                f,
                "\n  at {} ({}:{}:{})",
                frame.function,
                frame
                    .module
                    .as_deref()
                    .map_or_else(|| self.file.display().to_string(), str::to_owned),
                frame.line,
                frame.column
            )?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct ScriptEvent {
    pub target: Option<NodeId>,
    pub name: String,
    pub payload: rush::StateValue,
}
#[derive(Debug)]
enum Command {
    Transform(NodeId, Transform),
    Spawn(String, Transform),
    Destroy(NodeId),
}
#[derive(Default, Debug)]
struct Context {
    nodes: HashMap<NodeId, (Rc<NodeId>, Transform, String)>,
    input: HashMap<String, (f64, bool)>,
    commands: Vec<Command>,
    max_commands: usize,
    accepting: bool,
}
impl Context {
    fn refresh(&mut self, scene: &SceneGraph) {
        self.nodes.retain(|id, _| scene.local(*id).is_ok());
        for (id, transform, _) in scene.nodes() {
            let entry = self
                .nodes
                .entry(id)
                .or_insert_with(|| (Rc::new(id), transform, String::new()));
            entry.1 = transform;
            entry.2 = scene.name(id).unwrap_or_default().into();
        }
    }
    fn id(&self, value: &Value<'_>) -> Result<NodeId, String> {
        let Value::HostObject(handle) = value else {
            return Err("Expected Voxy object".into());
        };
        let id = handle.upgrade::<NodeId>().ok_or("Expired Voxy object")?;
        self.nodes
            .contains_key(&id)
            .then_some(*id)
            .ok_or("Stale or foreign Voxy object".into())
    }
    fn push(&mut self, command: Command) -> Result<(), String> {
        if !self.accepting {
            return Err("Scene writes require a lifecycle handler".into());
        }
        if self.commands.len() >= self.max_commands {
            return Err("Script command limit exceeded".into());
        }
        self.commands.push(command);
        Ok(())
    }
}
fn vector(value: &Value<'_>, n: usize) -> Result<Vec<f32>, String> {
    let Value::List(values) = value else {
        return Err("Expected numeric list".into());
    };
    if values.len() != n {
        return Err(format!("Expected {n} components"));
    }
    values
        .iter()
        .map(|v| match v {
            Value::Number(v) if v.is_finite() && v.abs() <= f32::MAX as f64 => Ok(*v as f32),
            _ => Err("Invalid transform component".into()),
        })
        .collect()
}
fn registrations(context: &Rc<RefCell<Context>>) -> Vec<HostRegistration> {
    let object = ValueType::HostObject("VoxyObject");
    let list = ValueType::List(Box::new(ValueType::Number));
    let mut result = Vec::new();
    for (get, set, field, length) in [
        ("position", "set_position", 0, 3),
        ("rotation", "set_rotation", 1, 4),
        ("scale", "set_scale", 2, 3),
    ] {
        let c = context.clone();
        result.push(HostRegistration::new(
            get,
            vec![object.clone()],
            list.clone(),
            move |args, _| {
                let c = c.borrow();
                let id = c.id(&args[0])?;
                let t = c.nodes[&id].1;
                let values = match field {
                    0 => t.translation.to_array().to_vec(),
                    1 => t.rotation.to_array().to_vec(),
                    _ => t.scale.to_array().to_vec(),
                };
                Ok(Value::List(
                    values
                        .into_iter()
                        .map(|v| Value::Number(v as f64))
                        .collect(),
                ))
            },
        ));
        let c = context.clone();
        result.push(HostRegistration::new(
            set,
            vec![object.clone(), list.clone()],
            ValueType::Null,
            move |args, _| {
                let mut c = c.borrow_mut();
                let id = c.id(&args[0])?;
                let v = vector(&args[1], length)?;
                let mut t = c.nodes[&id].1;
                match field {
                    0 => t.translation = Vec3::new(v[0], v[1], v[2]),
                    1 => t.rotation = Quat::from_xyzw(v[0], v[1], v[2], v[3]),
                    _ => t.scale = Vec3::new(v[0], v[1], v[2]),
                };
                t.matrix().map_err(|e| e.to_string())?;
                c.push(Command::Transform(id, t))?;
                c.nodes.get_mut(&id).unwrap().1 = t;
                Ok(Value::Null)
            },
        ));
    }
    let c = context.clone();
    result.push(HostRegistration::new(
        "find",
        vec![ValueType::String],
        ValueType::List(Box::new(object.clone())),
        move |args, _| {
            let Value::String(name) = &args[0] else {
                unreachable!()
            };
            let c = c.borrow();
            let mut nodes: Vec<_> = c.nodes.values().filter(|n| &n.2 == name).collect();
            nodes.sort_by_key(|n| format!("{:?}", n.0));
            Ok(Value::List(
                nodes
                    .into_iter()
                    .map(|n| Value::HostObject(HostObject::new("VoxyObject", &n.0)))
                    .collect(),
            ))
        },
    ));
    let c = context.clone();
    result.push(HostRegistration::new(
        "destroy",
        vec![object],
        ValueType::Null,
        move |args, _| {
            let mut c = c.borrow_mut();
            let id = c.id(&args[0])?;
            c.push(Command::Destroy(id))?;
            Ok(Value::Null)
        },
    ));
    let c = context.clone();
    result.push(HostRegistration::new(
        "spawn",
        vec![ValueType::String, list],
        ValueType::Null,
        move |args, _| {
            let Value::String(name) = &args[0] else {
                unreachable!()
            };
            let v = vector(&args[1], 3)?;
            c.borrow_mut().push(Command::Spawn(
                name.clone(),
                Transform {
                    translation: Vec3::new(v[0], v[1], v[2]),
                    ..Transform::default()
                },
            ))?;
            Ok(Value::Null)
        },
    ));
    for (name, pressed) in [("input", false), ("pressed", true)] {
        let c = context.clone();
        result.push(HostRegistration::new(
            name,
            vec![ValueType::String],
            if pressed {
                ValueType::Bool
            } else {
                ValueType::Number
            },
            move |args, _| {
                let Value::String(name) = &args[0] else {
                    unreachable!()
                };
                let c = c.borrow();
                let value = c
                    .input
                    .get(name)
                    .ok_or_else(|| format!("Unknown input action: {name}"))?;
                Ok(if pressed {
                    Value::Bool(value.1)
                } else {
                    Value::Number(value.0)
                })
            },
        ));
    }
    result
}
struct Entry {
    owner: NodeId,
    file: PathBuf,
    source: String,
    attempted_source: Option<String>,
    script: OwnedScriptInstance,
    selected: Vec<String>,
    started: bool,
    paused: bool,
    destroyed: bool,
}
/// Own on the scene thread. Call fixed_update for each physics tick and update
/// once per frame. Each successful hook commits at a command barrier.
pub struct ScriptManager {
    context: Rc<RefCell<Context>>,
    entries: Vec<Entry>,
    scene_guard: Option<voxy_scene::SceneCommands>,
    pub diagnostics: Vec<ScriptDiagnostic>,
    pub command_errors: Vec<SceneGraphError>,
    pub limits: ExecutionLimits,
    max_instances: usize,
}
impl ScriptManager {
    pub fn new(max_instances: usize, max_commands: usize) -> Self {
        Self {
            context: Rc::new(RefCell::new(Context {
                max_commands,
                ..Context::default()
            })),
            entries: Vec::new(),
            scene_guard: None,
            diagnostics: Vec::new(),
            command_errors: Vec::new(),
            limits: ExecutionLimits::new(100_000),
            max_instances,
        }
    }
    fn instantiate(
        &self,
        owner: NodeId,
        source: &str,
    ) -> Result<OwnedScriptInstance, RuntimeError> {
        let handle = self.context.borrow().nodes[&owner].0.clone();
        OwnedScriptInstance::with_host(
            source,
            self.limits,
            CancellationToken::default(),
            vec![(
                "self".into(),
                Value::HostObject(HostObject::new("VoxyObject", &handle)),
            )],
            vec![],
            registrations(&self.context),
            vec![],
        )
    }
    pub fn attach(
        &mut self,
        scene: &mut SceneGraph,
        owner: NodeId,
        file: impl AsRef<Path>,
        selected: Vec<String>,
    ) -> Result<(), String> {
        if self.entries.len() >= self.max_instances {
            return Err("Script instance limit exceeded".into());
        }
        self.validate_scene(scene)?;
        if self.entries.iter().any(|e| e.owner == owner) {
            return Err("Object already has a script".into());
        }
        scene.local(owner).map_err(|e| e.to_string())?;
        let file = file.as_ref().to_path_buf();
        if file.extension().is_none_or(|e| e != "r") {
            return Err("Expected .r script".into());
        }
        let source = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
        self.context.borrow_mut().refresh(scene);
        let compiled = self.instantiate(owner, &source);
        // Top-level code initializes variables; scene writes require a lifecycle hook.
        self.context.borrow_mut().commands.clear();
        let compiled = compiled.and_then(|mut script| {
            let names: Vec<_> = selected.iter().map(String::as_str).collect();
            let state = script.export_state(&names)?;
            script.restore_state(&state)?;
            Ok(script)
        });
        let script = compiled.map_err(|error| {
            let d = ScriptDiagnostic {
                file: file.clone(),
                handler: "compile".into(),
                error,
            };
            let message = d.to_string();
            self.diagnostics.push(d);
            message
        })?;
        self.entries.push(Entry {
            owner,
            file,
            source,
            attempted_source: None,
            script,
            selected,
            started: false,
            paused: false,
            destroyed: false,
        });
        Ok(())
    }
    fn invoke(
        &mut self,
        index: usize,
        scene: &mut SceneGraph,
        name: &str,
        args: &[Value<'static>],
    ) {
        if self.entries[index].paused {
            return;
        }
        if name == "on_destroy" {
            if self.entries[index].destroyed {
                return;
            }
            self.entries[index].destroyed = true;
        }
        self.context.borrow_mut().refresh(scene);
        self.context.borrow_mut().commands.clear();
        let entry = &mut self.entries[index];
        let exists = entry.script.with_instance(|s| s.get(name).is_some());
        if !exists {
            return;
        }
        self.context.borrow_mut().accepting = true;
        let result = entry.script.call_values(name, args, self.limits);
        self.context.borrow_mut().accepting = false;
        if let Err(error) = result {
            entry.paused = true;
            self.context.borrow_mut().commands.clear();
            self.diagnostics.push(ScriptDiagnostic {
                file: entry.file.clone(),
                handler: name.into(),
                error,
            });
            return;
        }
        let commands = std::mem::take(&mut self.context.borrow_mut().commands);
        let victims: Vec<_> = self.entries.iter().enumerate().filter(|(_,e)| commands.iter().any(|command| matches!(command,Command::Destroy(id) if descendant(scene,e.owner,*id)))).map(|(i,_)| i).collect();
        let mutations: Vec<_> = commands
            .into_iter()
            .map(|command| match command {
                Command::Transform(id, t) => voxy_scene::SceneMutation::SetLocal(id, t),
                Command::Spawn(name, t) => voxy_scene::SceneMutation::SpawnNamed(name, t),
                Command::Destroy(id) => voxy_scene::SceneMutation::RemoveSubtree(id),
            })
            .collect();
        if let Err(error) = scene.apply_atomic(&mutations) {
            self.entries[index].paused = true;
            self.command_errors.push(error.cause);
            self.report_command_error(index, name, error.to_string());
            return;
        }
        // Destruction is observable only after a successful structural commit.
        // Each destruction hook has its own independent atomic command batch.
        for victim in victims {
            self.invoke(victim, scene, "on_destroy", &[]);
        }
    }
    fn report_command_error(&mut self, index: usize, handler: &str, message: String) {
        let entry = &self.entries[index];
        let offset = entry.source.find(&format!("fn {handler}")).unwrap_or(0);
        let prefix = &entry.source[..offset];
        let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
        let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        let mut span = rush::parse(&entry.source).module.span;
        span.start = offset;
        span.end = offset;
        self.diagnostics.push(ScriptDiagnostic {
            file: entry.file.clone(),
            handler: handler.into(),
            error: RuntimeError {
                module: None,
                span,
                message: format!("Scene transaction rejected: {message}"),
                location: Some(rush::SourceLocation { line, column }),
                stack: vec![rush::CallFrame {
                    function: handler.into(),
                    module: None,
                    span,
                    line,
                    column,
                }],
            },
        });
    }
    fn tick(&mut self, scene: &mut SceneGraph, delta: f64, fixed: bool) -> Result<(), String> {
        self.validate_scene(scene)?;
        if !delta.is_finite() || delta < 0.0 {
            return Err("Invalid delta".into());
        }
        for index in 0..self.entries.len() {
            let owner = self.entries[index].owner;
            if scene.local(owner).is_err() {
                self.invoke(index, scene, "on_destroy", &[]);
                continue;
            }
            if !scene.active_in_hierarchy(owner).unwrap_or(false) {
                continue;
            }
            if self.entries[index].destroyed {
                continue;
            }
            if !self.entries[index].started {
                self.entries[index].started = true;
                self.invoke(index, scene, "start", &[]);
            }
            if scene.local(owner).is_ok() && !self.entries[index].destroyed {
                self.invoke(
                    index,
                    scene,
                    if fixed { "fixed_update" } else { "update" },
                    &[Value::Number(delta)],
                );
            }
        }
        self.entries.retain(|e| scene.local(e.owner).is_ok());
        Ok(())
    }
    pub fn update(&mut self, scene: &mut SceneGraph, delta: f64) -> Result<(), String> {
        self.tick(scene, delta, false)
    }
    pub fn fixed_update(&mut self, scene: &mut SceneGraph, delta: f64) -> Result<(), String> {
        self.tick(scene, delta, true)
    }
    pub fn set_input(&mut self, input: &InputMap, names: &[&str]) -> Result<(), String> {
        let values = names
            .iter()
            .map(|name| {
                input
                    .state(name)
                    .map(|s| ((*name).into(), (s.value as f64, s.pressed)))
                    .ok_or_else(|| format!("Unknown input action: {name}"))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        self.context.borrow_mut().input = values;
        Ok(())
    }
    /// Physics/scene adapters supply typed data as an on_event record.
    pub fn event(&mut self, scene: &mut SceneGraph, event: ScriptEvent) -> Result<(), String> {
        self.validate_scene(scene)?;
        let payload = event.payload.to_value()?;
        let record = Value::Record(
            [
                (String::from("name"), Value::String(event.name)),
                (String::from("payload"), payload),
            ]
            .into(),
        );
        for i in 0..self.entries.len() {
            if event.target.is_none_or(|id| id == self.entries[i].owner)
                && scene
                    .active_in_hierarchy(self.entries[i].owner)
                    .unwrap_or(false)
            {
                if !self.entries[i].started {
                    self.entries[i].started = true;
                    self.invoke(i, scene, "start", &[]);
                }
                if !self.entries[i].destroyed && scene.local(self.entries[i].owner).is_ok() {
                    self.invoke(i, scene, "on_event", &[record.clone()]);
                }
            }
        }
        Ok(())
    }
    pub fn save(&mut self, owner: NodeId) -> Result<ScriptState, String> {
        let e = self
            .entries
            .iter_mut()
            .find(|e| e.owner == owner)
            .ok_or("No script")?;
        e.script
            .export_state(&e.selected.iter().map(String::as_str).collect::<Vec<_>>())
            .map_err(|e| e.message)
    }
    pub fn restore(&mut self, owner: NodeId, state: &ScriptState) -> Result<(), String> {
        let e = self
            .entries
            .iter_mut()
            .find(|e| e.owner == owner)
            .ok_or("No script")?;
        if state.keys().any(|key| !e.selected.contains(key)) {
            return Err("Unselected state variable".into());
        }
        e.script.restore_state(state).map_err(|e| e.message)
    }
    /// Poll contents rather than timestamps; failed candidates never replace live instances.
    pub fn reload_changed(&mut self, scene: &SceneGraph) -> Vec<String> {
        let mut errors = Vec::new();
        if self.entries.iter().any(|e| scene.local(e.owner).is_err()) {
            return vec!["Reload requires live owners in the bound scene".into()];
        }
        for i in 0..self.entries.len() {
            let file = self.entries[i].file.clone();
            let result = (|| -> Result<(), String> {
                let source = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
                if source == self.entries[i].source {
                    self.entries[i].attempted_source = None;
                    return Ok(());
                }
                if self.entries[i].attempted_source.as_ref() == Some(&source) {
                    return Ok(());
                }
                self.entries[i].attempted_source = Some(source.clone());
                self.context.borrow_mut().refresh(scene);
                let candidate = self.instantiate(self.entries[i].owner, &source);
                self.context.borrow_mut().commands.clear();
                let mut candidate = candidate.map_err(|error| {
                    let d = ScriptDiagnostic {
                        file: file.clone(),
                        handler: "reload".into(),
                        error,
                    };
                    let message = d.to_string();
                    self.diagnostics.push(d);
                    message
                })?;
                let selected = self.entries[i].selected.clone();
                let names: Vec<_> = selected.iter().map(String::as_str).collect();
                let transfer = self.entries[i]
                    .script
                    .export_state(&names)
                    .and_then(|state| candidate.restore_state(&state));
                transfer.map_err(|error| {
                    let diagnostic = ScriptDiagnostic {
                        file: file.clone(),
                        handler: "reload_state".into(),
                        error,
                    };
                    let message = diagnostic.to_string();
                    self.diagnostics.push(diagnostic);
                    message
                })?;
                self.entries[i].script = candidate;
                self.entries[i].source = source;
                self.entries[i].attempted_source = None;
                self.entries[i].paused = false;
                Ok(())
            })();
            if let Err(e) = result {
                errors.push(e);
            }
        }
        errors
    }
    pub fn paused(&self, owner: NodeId) -> bool {
        self.entries.iter().any(|e| e.owner == owner && e.paused)
    }
}

fn descendant(scene: &SceneGraph, mut node: NodeId, root: NodeId) -> bool {
    loop {
        if node == root {
            return true;
        }
        match scene.parent(node) {
            Ok(Some(parent)) => node = parent,
            _ => return false,
        }
    }
}

impl std::fmt::Debug for ScriptManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptManager")
            .field("instances", &self.entries.len())
            .field("diagnostics", &self.diagnostics)
            .finish_non_exhaustive()
    }
}
impl ScriptManager {
    fn validate_scene(&mut self, scene: &mut SceneGraph) -> Result<(), String> {
        self.scene_guard
            .get_or_insert_with(|| voxy_scene::SceneCommands::new(scene, 0))
            .apply(scene)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    /// Reads the existing bounded scene event channel and reports lost events.
    pub fn read_events(
        &mut self,
        scene: &mut SceneGraph,
        channel: &voxy_scene::EventChannel<ScriptEvent>,
        cursor: &mut voxy_scene::EventCursor,
    ) -> Result<u64, String> {
        self.validate_scene(scene)?;
        let read = channel.read(cursor).map_err(|e| e.to_string())?;
        for event in read.events {
            self.event(scene, event.clone())?;
        }
        Ok(read.missed)
    }
    pub fn save_file(&mut self, owner: NodeId, path: impl AsRef<Path>) -> Result<(), String> {
        let state = self.save(owner)?;
        let bytes = serde_json::to_vec_pretty(&state).map_err(|e| e.to_string())?;
        use std::io::Write;
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_SAVE: AtomicU64 = AtomicU64::new(0);
        let path = path.as_ref();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let name = path
            .file_name()
            .ok_or("Save path requires a file name")?
            .to_string_lossy();
        let temp = parent.join(format!(
            ".{name}.rush-save-{}-{}.tmp",
            std::process::id(),
            NEXT_SAVE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        let result = (|| -> std::io::Result<()> {
            if let Ok(metadata) = std::fs::metadata(path) {
                file.set_permissions(metadata.permissions())?;
            }
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            std::fs::rename(&temp, path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temp);
        }
        result.map_err(|e| e.to_string())
    }
    pub fn restore_file(&mut self, owner: NodeId, path: impl AsRef<Path>) -> Result<(), String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let state = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        self.restore(owner, &state)
    }
    /// Explicit shutdown barrier; invoke before dropping the scene.
    pub fn clear(&mut self, scene: &mut SceneGraph) -> Result<(), String> {
        self.validate_scene(scene)?;
        for i in 0..self.entries.len() {
            self.invoke(i, scene, "on_destroy", &[]);
        }
        self.entries.clear();
        Ok(())
    }
    #[cfg(feature = "physics-events")]
    pub fn character_events<O>(
        &mut self,
        scene: &mut SceneGraph,
        owner: NodeId,
        step: &physics::CharacterStep<O>,
        obstacle: impl Fn(&O) -> rush::StateValue,
    ) -> Result<(), String> {
        for contact in &step.contacts {
            self.event(
                scene,
                ScriptEvent {
                    target: Some(owner),
                    name: "physics.contact".into(),
                    payload: rush::StateValue::Record(
                        [
                            (
                                "normal".into(),
                                rush::StateValue::List(
                                    contact
                                        .normal
                                        .iter()
                                        .map(|n| rush::StateValue::Number(*n as f64))
                                        .collect(),
                                ),
                            ),
                            ("obstacle".into(), obstacle(&contact.obstacle)),
                        ]
                        .into(),
                    ),
                },
            )?;
        }
        self.event(
            scene,
            ScriptEvent {
                target: Some(owner),
                name: "physics.character_step".into(),
                payload: rush::StateValue::Record(
                    [
                        ("grounded".into(), rush::StateValue::Bool(step.grounded)),
                        ("stepped_up".into(), rush::StateValue::Bool(step.stepped_up)),
                    ]
                    .into(),
                ),
            },
        )
    }
}
