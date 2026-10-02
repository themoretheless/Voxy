# Rush object behavior in Voxy

`ScriptManager` owns one persistent `OwnedScriptInstance` per `NodeId`, on the
scene thread. It uses the tokenizer `release` dependency pinned in Cargo.lock;
no sibling checkout is needed. `PositionScript` remains available for stateless
position expressions.

```sh
cargo run -p voxy_rush --example behavior
cargo run -p voxy_app --example rush_scene
cargo test -p voxy_rush --features physics-events
```

The native demo uses **A/D** to move, **F** to create a cube and **E** to send
`scene.reward` (the player doubles its scale). Space pauses. Its `.r` file is
reloaded when contents change. Pass another `.r` path as a command-line argument.
The demo renders spawned scene objects as cubes; mesh/prefab selection is a
separate host concern. An in-window diagnostics panel shows file, line, column, call stack and instance
status. F1 toggles it, PgUp/PgDn select error history, and Up/Down scroll long
stacks. The panel keeps at most 32 messages; stderr also receives diagnostics.
The UI uses Voxy glyph rasterization with a platform font; set `VOXY_UI_FONT`
to a local TTF path when the platform has no standard font installed.

Attach a `.r` file with `attach(scene, owner, path, selected_state_names)`.
Call `fixed_update` once per physics tick and `update` once per rendered frame.
`start()` runs once, on the first active tick/event. `update(delta)` and
`fixed_update(delta)` retain mutable globals. Inactive objects skip these hooks.
`on_destroy()` runs once after a successful subtree-removal commit, external
removal synchronization, or explicit `clear(scene)`. Its owner may already be
invalid; destruction hooks can still find/change other live scene objects. Call clear before dropping the scene.
Paused instances do not execute further hooks, including destruction.

Scene API:

- `self` is an opaque host object backed by a generational, scene-specific Voxy
  `NodeId`. Numeric IDs cannot impersonate handles; removed/reused objects expire.
- `position(object)`, `rotation(object)`, `scale(object)` return numeric lists;
  corresponding `set_position`, `set_rotation`, `set_scale` functions take lists.
  Rotation is a unit quaternion `[x,y,z,w]`. Nonfinite transforms are rejected.
- `find(name)` returns matching handles in deterministic order.
- `spawn(name, [x,y,z])` queues creation of a root object and returns null.
  Use `find` in a subsequent handler to obtain its handle.
- `destroy(object)` queues subtree removal, including destruction hooks.

Reads use a handler snapshot, with read-your-writes for transform setters.
Writes require a lifecycle/event handler. Commands remain queued until that
handler succeeds. Runtime errors discard its entire queue and pause the instance;
script variable mutations before the error are retained. Successful handlers
commit through `SceneGraph::apply_atomic`: the entire ordered batch is first
validated against a component-free shadow of the hierarchy, including capacity,
handle generations and free-slot order. A failed preflight leaves every live
transform, object and component intact, emits a source diagnostic and pauses the
handler instance. No IDs are consumed. Destruction hooks are dispatched only
after the batch commits, and their commands form separate atomic batches. No parallel dispatch is implied.
Instance and per-handler command admission are bounded; execution limits are
configurable through `limits`.

Input/event adapters:

- `set_input(&InputMap, action_names)` snapshots existing Voxy named actions.
  Scripts use `input(name)` and `pressed(name)`. The host calls `finish_frame`
  after dispatch, so all object scripts observe the same input transition.
- `event(scene, ScriptEvent)` calls `on_event({name, payload})` for a target or
  all active instances. `read_events` consumes Voxy's bounded `EventChannel`
  using an independent cursor and returns the number of missed events.
- Enable `physics-events` to pass a real `physics::CharacterStep<O>` to
  `character_events`. Character contacts emit `physics.contact` with `normal`
  and a host-selected portable `obstacle` payload. The step also emits
  `physics.character_step` with `grounded` and `stepped_up`. The adapter accepts
  the same step type returned by voxel character physics. The caller invokes it
  after physics finishes; there is no global physics event bus.

`save`/`save_file` export only selected variables as portable Rush `ScriptState`
(JSON for files). File saves write/sync a private sibling temporary file before
replacing the destination, so failed writes preserve the previous save. `restore`/`restore_file` reject unselected names and validate the
whole state before changing bindings. Host handles/callables are not portable.
Persist stable application IDs and resolve them through the host after loading.

`reload_changed` initializes a candidate, transfers the selected state, and only
then replaces the live instance. Existing started status is preserved; start
is not replayed. Invalid syntax, initialization errors or incompatible state
leave the old instance running. A successful reload resumes a paused instance.
An unchanged failed candidate is not retried every frame; edit it to retry.

Native integration is `SceneApp::with_script(path, selected_names)`.
`with_script_character()` additionally enables the live fixed-step character
controller used by `rush_scene`: a unit collider, static floor and the created
cubes use the shared `physics::step_character`/`sweep_box` solver. Scene positions
are converted through the physics anchor, including negative coordinates. Real
contacts are sent to Rush after each successful physics step; `player.r` records
them in its `contacts` variable. Visual scale is independent of the unit collider.
The default character-free integration preserves arbitrary scripted transforms. Other
scene owners can use the manager directly with the same input/events/physics
adapters. Hot reload is polled while the scene is paused as well as during play. Successful
recovery records a visible resume message and retains earlier errors in history.
