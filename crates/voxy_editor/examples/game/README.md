# Native playable scene

Run from the workspace root:

```sh
cargo run -p voxy_editor -- --manifest crates/voxy_editor/examples/game/assets.json player --scene crates/voxy_editor/examples/game/game.scene.json
```

Press **F6** or click **Play**. Use **Left/Right** to move and **Space** to jump
over the small step. **W/S** moves in depth. Press **F6** again to restore the
exact authored scene. The camera is the editor's existing fixed orthographic
projection; this is a small playable integration fixture, not a camera system.

The Player owns a `game.character.v1` authoring descriptor. Floor and Step own
`game.box.v1` descriptors. Their OBJ geometry and collision shapes are explicit
separate data. Geometry has its final dimensions and object scales are one.
Character velocity and grounding are runtime-only and are not saved.

In the inspector, **C** adds/removes Character, **B** adds/removes Box Collider
(the two are mutually exclusive), and **I** switches between transform and physics
numeric fields. Click a value, type it and press Enter. Undo/redo includes component
changes and duplication. Physics owners and their ancestors must have identity
rotation/scale; translated parents and inherited activity are supported. Adding
a dynamic body above another physics shape is rejected. Characters collide with
active static boxes, not with one another. Initial collider penetration is rejected;
place the character above the floor before Play. The native adapter currently
exposes one logical keyboard; the input core supports independent device IDs. There is no arbitrary rigid body solver
or serialized behavior factory in this fixture.

F5 saves to the `--scene` path. To experiment without modifying this checked-in
fixture, copy this directory and open the copy.

Repeatable real-window acceptance:

```sh
cargo build -p voxy_editor
python3 tools/test_gameplay_window.py --binary target/debug/voxy_editor
```

The harness opens a temporary copy and verifies presented GPU frames, movement
and a complete quick-tap jump through the same key mapping as window events,
activity, lifecycle, queued deletion/creation/component writes, slot generation
reuse, resource cache retirement, and exact Stop restoration. It requires a native
window/GPU and fails on timeout; a headless run cannot substitute for this gate.

Physics/backend regressions:

```sh
cargo test -p voxy_gameplay --test integration
```
