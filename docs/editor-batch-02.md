# Priority batch 02: items 11–20

This batch is accepted for the bounded native OBJ editor + swept-character
playable project described below. This is end-to-end acceptance of items 11–20
in that scope, not complete Unity/Godot/Stride parity or a general rigid-body engine.

| Item | Acceptance | Current state |
| --- | --- | --- |
| 11. Scene/render adapter | Extract active scene model components and composed transforms, reconcile backend resources by generational owner, verify rendered output through edits/deletion/reuse | Accepted: native active/interpolated extraction, generational GPU owner cache; native deletion/reuse acceptance |
| 12. Scene/simulation adapter | Explicit runtime state and scene synchronization with parent/activity/deletion semantics | Accepted: scene-local swept-character runtime + static-box backend, translated parents, activity, teleport/reset and deletion |
| 13. Object/component CRUD | Usable validated creation, edit and removal through public API and editor | Accepted: public typed component CRUD, queued spawn/name/insertion/replacement/removal; editor Character/Box numeric fields and history |
| 14. Activity | Local flags, inherited activity and subsystem transitions agree | Accepted: inherited activity in scene/physics; native disable/re-enable agrees with rendering and lifecycle |
| 15. Persistent identity | Durable object IDs and scene-local handles remain distinct through reload/clone/delete | Accepted: opaque SceneId and generational NodeId distinct from durable ObjectId; reload and reused-owner adapter tests |
| 16. Lifecycle | Start/update/stop/destroy obey ownership and structural barriers | Accepted: bounded admission, lifecycle sequence through native disable/re-enable/delete, terminal Stop with exact authoring restoration |
| 17. Safe commands | Structural mutations execute at explicit barriers with bounded admission and failure semantics | Accepted: bounded owner-thread command barrier with typed CRUD/spawn results and per-command failures; legacy mutable Behavior hooks remain serial |
| 18. Input actions | Runtime consumes named actions with focus/device edge semantics | Accepted: named keyboard movement/jump, quick taps retained until a successful fixed tick, once-only catch-up consumption, focus/device core regressions |
| 19. Fixed simulation tick | Bounded catch-up and explicit interpolation/dropped-time behavior | Accepted: 60 Hz/max 8, separate dropped-time reports, local TRS interpolation, fixed-system pose capture and explicit failure/reset policy |
| 20. Playable project | One runnable project uses the preceding integration with repeatable acceptance | Accepted: examples/game durable fixture, standalone native editor, public character_scene example, repeatable real-window acceptance |

Architecture boundary for item 11: SceneExtraction owns a backend-independent
projection; the renderer owns GPU geometry and transform buffers. Model references
are projected per object, geometry remains shared per logical asset. Extraction
fails atomically on count overflow or invalid composed transforms. Backend errors
must be handled before drawing a retained projection whose owners may have been
removed. Component clone costs are explicitly outside the instance-count budget.

Initial item 11 validation: 43 scene tests and 10 editor tests passed; strict
Clippy passed for both libraries. Native manifest reload/relocation/scene-file
acceptance passed after the editor was switched to the extraction bridge.
Inactive owners emit no instances; stale/deleted owner cache entries are pruned
before submission, and newly allocated generations get independent transforms.
Broader scene/simulation integration and a combined playable project are pending.

Simulation architecture (2026-10-01): voxy_time owns the existing clock policy;
voxy_runtime re-exports SimulationClock/TimeFrame for compatibility. TimeDrop
reports discarded real elapsed and scaled backlog separately. SceneSimulation
binds to a scene identity, has explicit behavior/command/catch-up limits, validates
foreign/invalid frames before draining commands, invokes existing lifecycle, and
rejects attach/advance after Stop. Stop dispatches cleanup once and discards queued
structural edits. Native Play uses this adapter and restores authoring after
cleanup. The inspector displays authoring values and explains that Stop is needed
to edit. Renderer interpolation is implemented through a separate presentation projection (see below).

Reproduction: `cargo run -p voxy_scene --example fixed_scene` uses public APIs to
verify 60 ticks, inherited transforms/activity, barrier deletion, empty render
extraction and Stop. Editor regression advances Play through 100 10-ms frames,
checks 60 ticks and unchanged authoring history, then verifies exact Stop restore.

Verified simulation gates: 46 scene + 11 editor + 4 clock tests passed; strict
Clippy across all targets of these three crates passed; voxy_runtime library
check passed with compatibility exports. The public fixed_scene example passed.
Native smoke executed 14 fixed ticks across 32 presented Play/Stop frames, then
confirmed a presented restored scene; full reload/relocation/scene acceptance
passed with 257 native frames. This does not close items 12/19 broadly: general
physics integration remains pending; interpolation was added in the subsequent change.

Presentation interpolation: SceneSimulation retains local poses before and after
its last fixed tick. Native Play extracts render matrices by interpolating local
translation/scale and quaternion rotation, then composing the hierarchy. It does
not decompose world matrices or mutate simulation/authoring poses. New generations,
changed parents/activity and external pose edits snap to current data; callers can
explicitly reset_interpolation after teleports. Variable-update edits are presented
at their current pose. Pose snapshots are bounded by scene capacity; this change
makes no allocation/performance claims. Interpolation adds one fixed-tick of normal
presentation latency. Picking still uses current simulation transforms.

The following acceptance supersedes the earlier pending second-batch status.
Broader rigid-body, device, camera and component-inspector work remains outside
this bounded project acceptance.

Interpolation validation: 48 scene tests and 11 editor tests passed. Regressions
cover local hierarchy composition under rotated/nonuniform/reflected parents,
last-tick catch-up, fractional no-tick frames, reset, external edits, reparenting,
stale handles, reused generations and Stop. Native reload/relocation/scene smoke
passed with 248 presented frames, including 14 fixed ticks and a presented Stop
restoration. A subsequent regression correction skips unchanged local poses to
avoid unnecessary quaternion normalization drift; scene tests passed afterward.

## Final integrated acceptance (2026-10-01)

`voxy_gameplay` owns the scene/backend bridge; `physics` remains scene- and
renderer-independent. CharacterBody/BoxCollider are durable authoring descriptors.
CharacterPhysics owns velocity and grounded state separately, keyed by generational
owner and bound to opaque SceneId. All active body solver steps and local-pose
publication are prepared before committing any physics state or consuming input.
Deletion or component detachment releases runtime state at structural barriers,
including zero-tick frames. Inactive owners freeze; external position/descriptor
changes reset velocity/grounding. Dynamic parents and nonidentity scale/rotation
on physics ancestry are explicitly unsupported, and initial static penetration
is rejected rather than silently freezing. Native Play validates admission before
detaching the authoring scene.

The static collision adapter uses the existing swept-box primitive and character
controller. Time-zero separating/tangent contacts are excluded; only f64 rebase
roundoff is corrected using a scale-dependent tolerance. Genuine authored
penetrations remain errors. Bodies collide with static boxes, not one another.
Scene coordinates are bounded to magnitude 1e6, descriptor extents to 1e4, and
body/collider counts to the configured admission limits. There is no performance
or allocation-free claim.

SceneSimulation.advance_with executes the owner-provided fixed system after
behavior dispatch and before pose capture. A system error reports completed ticks,
consumes the scheduled clock frame, skips remaining ticks/variable update and
resets interpolation to the current scene. This is not whole-frame rollback:
earlier behavior mutations and completed ticks remain committed. CharacterPhysics
itself publishes its multi-body step atomically. External typed SceneCommands use
explicit frame barriers and report per-command outcomes; successful Spawn handles
are returned with their command index for the next barrier. Legacy Behavior hooks
still receive mutable scene access and must remain serial.

Native keyboard events feed move_x/move_z/jump actions. Releases, focus changes and
the observed keyboard's removal reach the adapter. The frontend exposes one logical
keyboard; the input core supports independent device IDs. Input edges are cleared
only after a successful physics tick, preserving a complete tap on zero-tick render
frames and consuming it once across catch-up. Focus loss cancels pending presses
and preserves release edges. Actual gamepad hardware integration is not claimed.

The inspector adds/removes Character and Box Collider (mutually exclusive), toggles
transform/physics numeric fields, validates edits and includes the descriptors in
save/load, duplication and undo/redo. Invalid descriptor/affine edits restore the
last authored document. Play does not save runtime velocity or grounding; Stop
cleans lifecycle and restores the exact authoring document. Runtime-deleted
selection handles no longer abort rendering. Authoring gizmos are hidden in Play.

Runnable fixture: `crates/voxy_editor/examples/game/README.md`. Headless public API:
`cargo run -p voxy_gameplay --example character_scene` passed actual motion, quick-tap
jump over the step, interpolation, durable reload, component removal and Stop.
Native: `python3 tools/test_gameplay_window.py --binary target/debug/voxy_editor`
passed 34 fixed ticks / 73 presented frames, input/physics movement and jump,
activity, queued deletion/creation/component writes, exact lifecycle sequence,
generation reuse, GPU cache retirement and a presented authoring restoration.
The harness uses a temporary fixture copy and checks the file remains unchanged.
The same key translator handles window events and deterministic smoke input; the
harness does not claim physical keyboard/gamepad hardware testing.

Verification: 53 scene + 13 editor + 8 gameplay integration + 5 input tests passed
(79 total). Strict all-target Clippy passed for the four selected crates with
`--no-deps`; full dependency linting is not claimed (pre-existing physics lint
failures remain outside this change). The old native manifest reload/relocation,
scene-file and authoring acceptance passed 258 presented frames, including 14 fixed
Play ticks / 32 Play/Stop frames. `git diff --check` and Python syntax checks passed.

Further scope: arbitrary rigid bodies, rotated/scaled physical shapes, moving
platforms/character contacts, automatic depenetration, camera controls, generic
component widgets/serialized behavior factories, gamepad/rebinding UI, and the
remaining feature batches. The overall engine coverage goal remains open.

Manual native UI inspection: opened a separate temporary copy of the game fixture
and viewed the scene tree, player/floor/step geometry and the physics inspector
(Half X/Y/Z 0.040, Speed 0.600, Gravity -2.400, Jump speed 0.900). Native F6 entered
Play with authoring-only inspector hints and hidden gizmos; F6 Stop restored the
visible editor/gizmo state. The demo was left open in authoring mode. macOS
occlusion produced stale screenshots until a native zoom/resize refreshed the
surface; gameplay motion acceptance relies on the explicit presented-frame
harness above, not on stale captures. No checked-in authoring file was modified
by this UI inspection.
