# Production readiness: current evidence and first implementation

Date: 2026-10-26. The engine is not production ready. This register implements
selected portions of ENG-00 through ENG-07 in [the roadmap](engine-roadmap-2026-10-25.md); it does not
replace the complete engine objective. Existing dirty rendering/scene work is
preserved. Source presence below is not an execution qualification.

| Area | Current evidence | Status and next acceptance gate |
|---|---|---|
| Project bootstrap | `voxy_editor/src/project.rs`, CLI create/validate | **Partial, tested**: creation, validation, relocation and native editor entry without a model. Visible-window acceptance is pending. |
| Scenes/prefabs | `voxy_scene/src/document.rs`, `prefab_document.rs`, `history.rs`, `transaction.rs` | **Project-owner nested workflow passed**: edit/save/reopen, overrides/inheritance, failed dependency retention, relocation and source-free package. Native authoring UI and the complete prefab feature set remain unqualified. |
| Asset identity | `voxy_assets/src/manifest.rs`, `locations.rs` | **Project import/reload partly verified**: the existing catalog retains morph data, validates named playback bindings, rejects bad defaults and keeps the last good asset. General format/material coverage, dependency rebuild and cache/resource limits remain incomplete. |
| Play/Stop | `voxy_editor/src/play_session.rs`, `voxy_scene/src/simulation.rs` | **Headless project/host integration passed**: Play/standalone share initialization, input, fixed simulation and codecs; Stop restores authoring. Native presentation and the complete authoring/gameplay workflow remain unqualified. |
| Packaging | `voxy_assets/src/package.rs`, editor `export_game_package`, `packaged_game.rs` | **Project package tested**: descriptor/export/isolated loader, external glTF buffers and complete morph-weight traces survive source-tree deletion. Unchanged RobotExpressive project/package state matches after 240 ticks. Clean-machine native delivery remains pending. |
| Runtime physics/animation | Existing `physics`, `voxy_animation`, scene/gameplay adapters | **Headless character/locomotion/morph integration passed**: input, collisions, jump/landing, clip transitions, per-revision weight fades and package replay. Morph overflow preserves body/clock/frame state. General graph authoring, morph retarget mapping and native foot/contact quality remain open. |
| Rendering | Frozen near-first query/source and street HDR proofs under `artifacts/original-street-realism-2026-10-25/` | **Bounded precision and GPU morph owner gates passed**: weight-only updates, CPU pixel parity and all 19 original imported primitives checked by readback. Full native scene/motion quality and minimum 120 FPS remain unqualified. |
| UI/audio/navigation | Existing domain crates and editor adapters | **Headless UI/audio scenario passed**: button callback, focus/input capture and offline spatial PCM in a source-free package. Native audio, full HUD/menu, navigation and save/load remain unqualified. |
| Editor authoring | Existing model-based entry in `voxy_editor/src/main.rs` | **Headless owner workflow passed**: empty initialization, object creation, undo/redo, save and relocated reopen. Native rendering and interactive controls still need acceptance. |
| Delivery/platforms | Existing platform crates and packaging tools | **Unverified here**: clean-machine desktop execution, recovery, platform-specific testing and release artifacts. |

## Implemented project boundary

A project directory contains `project.voxy` (version/name/manifest/start scene),
the EXISTING asset manifest format and the EXISTING scene document format.
An empty project has zero assets and zero scene objects; no dummy model is added.
Descriptor paths and asset sources are relative to the project root (the
descriptor directory), matching the existing editor input provider. Canonical resolution rejects paths/symlinks leaving the project root.
Opening uses a caller-supplied component registry and checks scene structure and
registered component decoding. The CLI uses built-in editor/gameplay registrations;
custom game components must supply their registry through the API. Cross-document
component-to-asset reference validation, asset decoding and build profiles remain
future work; checking file existence is not an import qualification.

Creation exclusively claims a NEW directory and writes files with `create_new`.
The descriptor is published last by a no-overwrite hard link to a flushed private
file, serving as the readiness marker. Failure removes only created files; it
never recursively deletes an occupied directory. The directory itself is visible
while creation runs: this is NOT an atomic directory rename or a claim of
crash-durable publication on every filesystem. Unsupported hard links fail
explicitly. The project command enters the existing native editor; visible-window validation
remains pending. Project standalone and descriptor-based packaging now share the
existing authoring/play owner; this does not qualify native rendering or deployment.

```sh
cargo +nightly-2026-05-27 run -p voxy_editor -- --create-project /absolute/new-game "My Game"
cargo +nightly-2026-05-27 run -p voxy_editor -- --validate-project /absolute/new-game/project.voxy
cargo +nightly-2026-05-27 run -p voxy_editor -- --project /absolute/new-game/project.voxy
cargo +nightly-2026-05-27 run -p voxy_editor -- --project /absolute/new-game/project.voxy --game-check
cargo +nightly-2026-05-27 run -p voxy_editor -- --build-project /absolute/new-game/project.voxy /absolute/game.vpak
cargo +nightly-2026-05-27 run -p voxy_editor -- --package /absolute/game.vpak --game-check
```

Create/validate return before window/GPU initialization; `--project` opens the native editor. The parent directory must
exist. Existing model/manifest editor commands remain available.

## Validation

- Nine project tests pass. Two exercise the actual editor owner without
  a window: object creation, undo/redo, save, relocated reopen, and nested manifest
  paths resolved against the project root. The original four cover: reopen after relocation, existing-directory
  preservation, malformed/missing/unsupported documents, absent source assets,
  existing-file preservation, blank-name rejection and symlink escape rejection.
- The actual editor CLI created and validated an empty project from cwd `/`,
  refused a second creation without replacement, and validated after relocation.
- `cargo clippy -p voxy_editor --lib --bin voxy_editor -- -D warnings` passes with
  the pinned nightly and isolated target. Two redundant u64 conversions in prior
  rendering diagnostics were removed without changing buffer limits.
- Full editor library suite: 201 passed, 17 ignored. Ignored checks are not
  counted as qualification.
- All-targets editor Clippy passes with `-D warnings`; the native editor binary
  builds. The real CLI validates the starter from cwd `/`.
- The empty hierarchy exposes `Create object (D)` through the existing creation
  action rather than requiring a model selection.
- Project Play and headless standalone produce identical motion after 120 fixed
  ticks. Stop restores the authoring document and rejects the former runtime handles.
- Project export includes descriptor, manifest, all declared sources and observed
  model/audio/font dependencies. The test uses a skeletal glTF with an external
  buffer not separately listed in the manifest. Its package runs after the entire
  source tree is removed; existing output is preserved and corruption rejected.
- Application-owned component codecs are exercised across editing, Play, project
  GameCheck, build and packaged GameCheck. Missing codecs fail explicitly; custom
  update callbacks/behaviors are not qualified by a custom-data component test.
- Real CLI acceptance from cwd `/`: create, validate, simulate a rotating model,
  build, reject overwrite, delete its source tree and simulate the isolated package.
  The project/package final documents are identical after 120 ticks.
- No native window, clean-machine distribution or FPS qualification is inferred
  from headless checks. No GPU performance benchmark was run for these changes.

## Next implementation, in dependency order

1. Validate native-window rendering and controls for `--project` with the empty
   starter. The constructor and actual authoring-owner roundtrip are tested; no
   visible editor window was launched in this qualification.
2. Extend the project scenario to prefab editing, import/reload failure recovery
   with useful authored components.
3. Extend the shared project runtime/package scenario to input, collision,
   UI callbacks, spatial audio and user game behavior. Existing common initialization
   and custom-data codecs are qualified only for the scenarios listed above.
4. Deliver a small interactive packaged game and verify it without its source assets.
   Only then close the first vertical slice and extend the remaining domain gates.

The generated [starter descriptor](../examples/empty-project/project.voxy) was
created and validated by the actual final CLI from cwd `/`. Logs, CLI outcomes
and source/binary SHA-256 identities are retained in
`production-readiness-evidence-2026-10-26/`. It is an empty authoring project,
openable through `--project`; visible rendering is unverified and it is not yet
a useful interactive game. The [motion example](../examples/project-motion/project.voxy)
and `artifacts/project-runtime-2026-10-26/motion.vpak` retain the CLI-verified
rotating-model sample and its exact package; `--project ... --game` and
`--package ... --game` select native launch (not visually verified here).

## Runtime/package contract and evidence

`project_runtime::prepare_game` is shared by the legacy model entry and the new
project entry. GameCheck uses offline audio and the existing fixed-step simulation;
it returns before creating an event loop, window or GPU. Native Game/GameSmoke
use the existing viewport and live-device path. `run_project_with_components`,
`export_project_package_with_components` and `run_packaged_game_with_components`
accept application codecs; executable game code is supplied by the host, never
serialized into the resource package.

The outer resource package remains unchanged. Version-1 model launch records stay
supported; version-2 launch records contain only the root project descriptor path.
Both envelopes reject traversal, nested bootstrap paths, unknown fields and
unsupported versions. The package loader materializes a private owned temporary
root and uses the ordinary project loader, with no source-directory fallback.
Export checks captured descriptor/manifest bytes and all existing observed import
revisions before creating output, and refuses existing output paths. It retains
all declared source files, including unused bindings, to preserve the current
project-open contract. This is not dead-asset pruning or an executable/app bundle.

Evidence: `project-runtime-full-tests.log`, `project-runtime-clippy.log`,
`project-runtime-build.log`, `project-runtime-cli-proof.json` and its runnable
Python script under `production-readiness-evidence-2026-10-26/`. The CLI proof
records the actual binary SHA-256, package SHA-256 and outputs. Simulation ticks
are not rendered frames or a real-time throughput measurement.

## Application behavior host (26 October, subsequent implementation)

`GameHost` combines the component registry, optional `BehaviorSetup` and UI action
setup in one contract. `run_project_with_host`, `export_project_package_with_host`
and `run_packaged_game_with_host` all use this contract. Every Play attaches fresh
behavior instances to the detached runtime graph using the existing bounded
`SceneSimulation`; no second scheduler or serialized executable code is added.
The data package still requires the matching host executable. Version-1 legacy
model packages reject behavior-host setup explicitly; project packages support it.

Behavior setup runs after the runtime graph and simulation are created. Its awake
edits are included in reset interpolation history. A returned setup error stops
partially attached behaviors, clears runtime owners and restores authoring; a
subsequent Play can retry. Panic recovery for arbitrary game hooks, runtime
registration of new component types, and dynamic attachment after startup are not
qualified by this change.

Two integration tests cover two Play/Stop cycles, standalone, package build and
source-free packaged launch with an application-defined movement component and
behavior. They check awake/start/fixed/destroy counts, actual final positions,
fresh instances on replay, authoring restoration, UI setup invocation, failed
setup cleanup and retry. The build validates attachment without advancing time.
UI setup invocation is tested here, not user clicks or gameplay UI dispatch.
Full editor library suite: **203 passed, 17 ignored** for this source state.

The external example `crates/voxy_editor/examples/project_game.rs` uses only public
APIs and owns its saved component schema and Rust behavior. Its sample project is
`examples/host-game/project.voxy`. Commands (native window modes are opt-in):

```sh
cargo +nightly-2026-05-27 run -p voxy_editor --example project_game -- --check examples/host-game/project.voxy
cargo +nightly-2026-05-27 run -p voxy_editor --example project_game -- --editor examples/host-game/project.voxy
cargo +nightly-2026-05-27 run -p voxy_editor --example project_game -- --build examples/host-game/project.voxy /absolute/new-host-game.vpak
cargo +nightly-2026-05-27 run -p voxy_editor --example project_game -- --package-check /absolute/new-host-game.vpak
```

This advances ENG-02/ENG-06. The full production roadmap remains incomplete;
input, collisions, prefab workflow, native rendering, hardware/device behavior,
platform deployment and performance still need their combined acceptance gates.

Public-example verification also passed using the built `project_game` executable
from cwd `/`: 120 fixed ticks produce identical movement in project and package;
the source tree is removed before package launch. Invalid behavior configuration
fails the build without publishing output. The exact sample package is retained
at `artifacts/project-host-2026-10-26/host-game.vpak`; it requires the example host
binary, since its executable Rust behavior is intentionally not stored in assets.
All-targets Clippy passes with warnings denied. Evidence, exact binary hash and
source identities: `game-host-cli-proof.json`, `game-host-qualification.json` and
`game-host-*.log` beside the preceding evidence. These are headless lifecycle and
simulation results, not native-device or performance qualification.

## Combined project: input, character collision, UI and audio

`HeadlessGame` is a public embeddable session around the SAME editor/play app
owner. It opens projects or verified project packages, delivers keyboard and
pointer edges, advances the existing simulation, captures durable-ID snapshots
and reports offline audio statistics. It creates no window or GPU. Key delivery
uses game UI focus first, and releases clear prior gameplay bindings as in native
routing. Focus loss cancels capture and releases gameplay input. Nonfinite/negative
time, nonfinite pointer coordinates and invalid viewport sizes fail before
advancing the scene. This is an integration/replay API, not a second solver.

The existing package extraction code is shared by native/package runs and these
sessions. The extracted project remains owned until app workers stop; regression
coverage verifies that it is removed after session shutdown. Per-process atomic
suffixes prevent same-timestamp extraction collisions during parallel tests.

`examples/interactive-project/project.voxy` contains a character, floor, wall,
UI button and a generated spatial WAV source/listener. It follows the character
solver's Y-up contract. The button's `sample.dismiss` callback is provided by
`crates/voxy_editor/examples/interactive_project.rs` through `GameHost`.

Two integration tests compare the complete scene document, jump maximum and PCM
frame-count/peak across editor Play, project standalone and source-free package.
The same 180-step input replay checks wall and floor nonpenetration every step,
jump/landing, UI command-barrier publication and Stop restoring authoring. Another
scenario checks focus loss, canceled pointer capture, keyboard UI consumption and
rejected invalid input/time without changing ticks or scene state.

The public example binary independently replays the project and package from cwd
`/` and checks identical results after deleting the private source tree. The
measured scenario result is x≈0.8 at the wall, final Y≈0.1 on the floor, maximum
Y≈0.5345, 180 fixed steps, 144000 mixed PCM frames and a nonzero peak. These are
simulation and mixer results, not screen pixels, device audio, elapsed FPS or a
qualification of the original high-detail street scene.

```sh
cargo +nightly-2026-05-27 run -p voxy_editor --example interactive_project -- --check examples/interactive-project/project.voxy
cargo +nightly-2026-05-27 run -p voxy_editor --example interactive_project -- --build examples/interactive-project/project.voxy /absolute/new-interactive.vpak
cargo +nightly-2026-05-27 run -p voxy_editor --example interactive_project -- --package-check /absolute/new-interactive.vpak
```

Explicit `--editor`/`--game` and `--package` modes select native windows; they have
not been launched for this qualification. Native controls use left/right arrows,
W/S and Space, with the blue UI button receiving pointer input. The sample does
not yet provide follow-camera authoring, animated character assets, a complete
menu/HUD or a full game. Terrain/voxel systems, general rigid-body physics,
navigation, networking and the other roadmap gates remain outstanding.

Evidence: `interactive-cli-proof.json` and its runnable script,
`interactive-qualification.json`, plus `interactive-*.log` in the evidence
folder. The retained exact resource package is
`artifacts/interactive-project-2026-10-26/interactive.vpak`; use the matching
`interactive_project` host executable for its callback registration.

## Project-backed nested prefabs

Project opening previously parsed only `SceneDocument`, rejecting `instances`
before the editor's existing prefab loader could run. It now accepts the existing
`PrefabSceneDocument` format, resolves stable prefab asset IDs through the existing
manifest and calls the core prefab expansion/scene validator. The returned
`OpenProject.scene` is the expanded runtime view; the editor keeps and saves the
original composition with its instance links and overrides.

The validator uses bounded observed inputs (130 entries / 16 MiB total, scene and
prefab reads capped at the existing 4 MiB document limit), and revalidates captured
bytes before publishing its view. Project validation and editor load/save/place/
revert now share the existing 4096-object, 128-instance and depth-16 prefab limits.
These values are centralized, not reduced from the former editor limits. Physical
source paths still use the project-root boundary; no second asset catalog or new
scene/prefab file format was introduced.

Two project-level regressions verify nested instances, nonflattened save, member
and transform overrides, undo/redo, inherited source edits, stable object IDs,
corrupt-dependency retention and recovery, relocation, Play/Stop and source-free
packaged simulation. Invalid cycles, missing logical prefab assets, stale override
targets, unsupported versions and excessive nesting fail before publication.

The retained `examples/prefab-interactive-project` wraps the interactive project in
two levels of linked prefabs. The startup scene overrides the nested player's
speed using an existing component-member override. Actual CLI binaries validated
it after relocation from cwd `/`, applied that override and reproduced identical
180-step character/UI/offline-audio results in its package after source deletion.
The runtime sample locates its uniquely named player/button after prefab ID
remapping; this sample convention is not a general reference-binding API.

```sh
cargo +nightly-2026-05-27 run -p voxy_editor -- --validate-project examples/prefab-interactive-project/project.voxy
cargo +nightly-2026-05-27 run -p voxy_editor --example interactive_project -- --check examples/prefab-interactive-project/project.voxy
cargo +nightly-2026-05-27 run -p voxy_editor --example interactive_project -- --build examples/prefab-interactive-project/project.voxy /absolute/new-prefab-game.vpak
cargo +nightly-2026-05-27 run -p voxy_editor --example interactive_project -- --package-check /absolute/new-prefab-game.vpak
```

Evidence is retained as `prefab-cli-proof.json`, its runnable Python script,
`prefab-qualification.json` and `prefab-*.log` beside earlier evidence. The exact
package is `artifacts/prefab-project-2026-10-26/prefab-interactive.vpak`. Native
prefab UI authoring, prefab variants/apply/conflict workflows, animated character
and camera integration, native-device validation, performance and the remaining
engine roadmap are still outstanding.

## Smooth follow camera: interpolated presentation, durable target

`game.follow-camera.v1` is a gameplay descriptor with a durable `ObjectId` target,
world-space target offset and positive finite half-life. Its reference visitor
and remapper use the existing scene/prefab codecs: nested instances rewrite the
target ID, and missing authoring targets are rejected by the scene loader.
The companion `editor.camera.v1` owns projection, distance and orbit orientation.

The editor adapter samples `SceneSimulation::render_world`, the same interpolated
world pose used by render extraction. It updates the presentation view after the
accepted simulation frame and before preparing render styles. It never writes the
followed object's physics transform or the saved camera component. `HeadlessGame`
exposes this view through `presentation_camera()` for reproducible replay checks.

`FollowSmoother` keeps f64 presentation history and integrates a first-order
response to the piecewise-linear sampled target. Stable small-argument coefficients
avoid cancellation; the weighted form retains a representable tiny prior-target
contribution instead of forcing it to zero. Tests compare a linear target with
its analytic response over 30/60/120/160/240/1000 simulation sample rates, reject
invalid inputs without changing history, and preserve tiny residuals for extreme
half-lives. These sample rates are not measured render FPS.

Project-level tests check camera movement on frames with no physics tick, an
independent check of the interpolated-pose-to-filter boundary, a finite projection
matrix, exact source-free package view traces, prefab target remapping, Stop
restoration and fresh smoothing history on a new Play. A missing runtime target
reports a named error and retains the last view. Multiple active follow cameras
fail explicitly until camera selection is implemented; failed startup restores
authoring and clears the partial simulation.

The retained `examples/follow-camera-project` includes a camera inside the nested
interactive-world prefab. Actual CLI validation and the interactive example check
its remapped target and identical camera/game/UI/offline-audio result after
source deletion. Commands:

```sh
cargo +nightly-2026-05-27 run -p voxy_editor -- --validate-project examples/follow-camera-project/project.voxy
cargo +nightly-2026-05-27 run -p voxy_editor --example interactive_project -- --check examples/follow-camera-project/project.voxy
cargo +nightly-2026-05-27 run -p voxy_editor --example interactive_project -- --build examples/follow-camera-project/project.voxy /absolute/new-camera-game.vpak
cargo +nightly-2026-05-27 run -p voxy_editor --example interactive_project -- --package-check /absolute/new-camera-game.vpak
```

This qualifies the follow-target smoothing path only. Native visual smoothness,
mouse orbit smoothing, camera collision/occlusion, target selection UI, camera
blends/cuts and a follow-component inspector are not qualified or completed by
this step. The descriptor is currently authored in the existing scene/prefab
format. Character animation, the full engine roadmap and target render FPS remain
outstanding. No native window, device audio or GPU benchmark was launched.

Camera checks on the current implementation: **209 editor tests passed, 17
ignored; 3 camera-math tests passed; all-targets Clippy passed** for editor and
gameplay. Native editor and public interactive example binaries build, and the
source-free camera CLI proof passes. Records: `camera-qualification.json`,
`camera-cli-proof.json`, its runnable script and `camera-*.log`.

The first full gameplay run finished with 180 passed / 1 failed: its old extreme
half-life test expected exact zero where the stable calculation retains a
representable residual. The expectation was corrected against the analytic
coefficient, and all three focused camera tests pass. The first failure and
diagnostic are retained. A full gameplay recheck is running separately under
session 30338, writing `/tmp/voxy-camera-gameplay-recheck.log`; its snapshot in
`camera-qualification.json` is not a terminal or green-suite claim. Inspect the
actual session/process before continuing. The exact sample package is retained at
`artifacts/follow-camera-2026-10-26/follow-camera.vpak`.


### Terminal result of the gameplay recheck

The previously running session 30338 has completed with exit code 0: **181 passed,
0 failed, 0 ignored**. The terminal log is `camera-gameplay-recheck.log`, and
`camera-qualification.json` now records the passed result. The earlier failed
expectation and its diagnostic remain retained rather than overwritten. There is
no pending gameplay process from that recheck.

## Follow-camera editor authoring

The existing generic component inspector already exposes follow half-life and
three offset fields; it is reused rather than replaced. Component pages now have
`Add follow camera` / `Remove follow camera`. Adding opens the existing hierarchy
as a target picker and atomically adds a valid companion viewport camera if one
is absent. The existing `/target` field has a `Pick` button and displays the target
name instead of requiring users to type a prefab-expanded ID.

Hierarchy choice actions carry tokens of durable object IDs and the current picker
epoch, not row indices. A choice from an earlier canceled/reopened picker cannot
affect the new request, even for the same owner and unchanged component.
Picker requests bind the original camera owner and expected follow descriptor;
stale requests/choices fail without publishing a history edit. Escape, viewport
selection, other authoring actions, reload/undo and Play cancel pending selection.
A companion view is retained when follow is removed. Replacing another follow
owner's view through the existing G shortcut is rejected before mutation.

All committed changes use the existing component codecs, scene validation,
history and prefab capture. Nested target references are saved in the correct
source-local scope and remapped on reopen. Tests exercise create/pick, numeric
field input, invalid half-life and missing target rejection, add/remove undo/redo,
save/reopen, reordered hierarchy candidates, nested member override save/reset,
stale/canceled/Play-interrupted actions and panel action-region construction.
These are headless authoring-owner/layout tests, not a visual native-window test.

Two related authoring safeguards were added. Object-ID allocation now accounts
for both `model-` and `object-` IDs after reopen. Subtree deletion validates a
candidate document and commits history before replacing the live graph, so a
registered reference to the subtree rejects the action without invalidating live
handles. The previous delete path did roll back failed commits; this improves
preflight and handle preservation, not recovery from a previously unhandled data
loss. Existing prefab tombstone/undo coverage remains in the full editor suite.

Follow target override reset and undo are also covered through the existing
component-field reset action after saving and reopening the nested project.
Reference overrides use source-local IDs in the file and resolve to the intended
expanded target after reload; the tests check both scopes. Native rendering and
real pointer/keyboard presentation of these controls still need visual acceptance.

Final camera-authoring checks: **212 editor tests passed, 17 ignored; all-targets
Clippy passed; the native editor binary builds**. The gameplay source files match
the qualified full recheck: **181 passed, no failures or ignored tests**. Evidence
is retained as `camera-authoring-qualification.json` and `camera-authoring-*.log`.
No live native-window presentation or performance result is inferred from these
owner/action/layout checks. The full roadmap remains incomplete.

## Locomotion requests over the existing animation/physics owner

`editor.model-locomotion.v1` maps idle/moving/rising/falling states to exact unique
clip names, with a movement threshold and transition duration. It uses accepted
character velocity/grounding from the preceding physical tick and the pending
jump edge. A visual rig may be on the body or a descendant; the nearest character
ancestor supplies its motion state. Imported hierarchy-part nodes are rejected as
logical playback owners. The initial unstepped state requests idle.

This is a locomotion request adapter, not a replacement animation player or a
complete Animator graph. Requests modify only the runtime `ModelAnimation` clip
name/transition duration. Existing ModelPlayback crossfades and AnimationRuntime's
character transaction own clocks, accepted skeletal poses, root-motion admission
and physics. The adapter validates every configured clip before applying requests
and restores its changed requests if the animation/character tick fails. It does
not consume jump input early or clear authored root-motion settings.

Play validates bindings before detaching the authoring graph. Asset publication
also validates all locomotion clip bindings, including inactive choices, before
accepting a model reload. Removing a currently unplayed jump clip rejects the
candidate and retains the last good imported model; restoring it recovers. Stop
restores the original authoring descriptors. Generic component fields can edit
existing locomotion descriptors; an add-component/clip-choice UI is still pending.

`HeadlessGame::animation_snapshot` exposes the accepted immutable frame's clip,
phase, transition weight and skin matrices by durable object ID. Two integration
tests compare full accepted frame traces in project and source-free package,
exercise a short press/release jump, prove an intermediate crossfade and changing
skin matrices, check Stop restoration, and verify failed-tick request rollback,
unchanged physical pose/animation serial and an unconsumed jump edge.

The tests and public `locomotion_project` CLI proof use an explicitly SYNTHETIC
regression rig with named copies of an existing test clip. This qualifies selection,
transaction and package wiring only; it does not prove realistic idle/walk/jump
motion, foot contact, positive locomotion/root-motion transitions or visual quality.
The general GraphPlayer/state-machine integration remains part of ENG-06.

### Unmodified upstream acceptance asset: import not yet supported

RobotExpressive was obtained from the official three.js repository at the pinned
commit recorded in `locomotion-upstream-provenance.json`. The upstream README
credits Tomás Laulhé / Don McCurdy and states CC0 1.0. Original model and license
hashes are retained; `locomotion-fetch-upstream.py` recovers/verifies the same bytes
without overwriting different files.

Actual import fails with `expected one mesh instance`. Inspection of that same
GLB finds multiple mesh instances, two skins, three morph-enabled primitives and
14 weight-animation channels. The current ModelAsset parser explicitly rejects
these capabilities. They were NOT removed, flattened or disabled to get a green
example. The original model and README remain under
`artifacts/locomotion-2026-10-26/upstream-robot/`; metadata, the exact failure and
required capabilities are retained in `locomotion-upstream-import-gap.json`.

The next required import work is full mesh-instance/skin binding support plus
morph geometry and animated weights through accepted CPU/GPU playback. Only then
can this upstream model be accepted for the character project. The public CLI
example is a regression/inspection consumer, not a ready realistic character demo.
The original street quality/precision/FPS requirements remain unchanged.

Validation evidence: `locomotion-cli-proof.json`, its runnable script,
`locomotion-qualification.json`, `locomotion-*.log` in the evidence directory.
No native window or GPU performance benchmark was launched for this step.

Final controller checks: **214 editor tests passed, 17 ignored; all-targets Clippy
passed; editor and public example build**. The scene integration test separately
requests all four names (idle/move/rise/fall); the CLI regression additionally
covers a shared jump clip for rising/falling. Exact accepted-frame traces match
in the source-free package. Neither fixture establishes the visual semantics of
those synthetic clips. The pinned upstream model import remains a reproduced
missing-capability case and is the next required implementation target.


## Property-curve interpolation prerequisite for morph animation

The existing `voxy_animation::PropertyTrack` now accepts explicit STEP and
CUBICSPLINE channels through `new_with_interpolation`. Incoming/outgoing
derivatives are retained per key with the same scalar/vector/color schema.
Hermite evaluation scales derivatives by segment duration, preserves overshoot
and uses f64 polynomial intermediates before returning the existing f32 property
value. This does not change the animation clock or introduce another player.
The legacy constructor retains linear numeric and held boolean behavior.

Invalid tangent counts, schemas, nonfinite tangents and unsupported boolean
interpolation are rejected. `PropertyClip::sample` rejects nonfinite interpolated
outputs rather than publishing them. Regression tests cover non-unit key spans,
exact key boundaries, negative loop time, overshoot, vector/color channels,
invalid derivative data and finite inputs whose cubic result overflows f32.

Validation: 4 focused property tests and all 330 animation library tests passed;
all-targets animation Clippy with `-D warnings` passed. Evidence is retained as
`property-interpolation-qualification.json` and `property-interpolation-*.log`.

This is a curve-layer prerequisite only. Morph geometry import, animated glTF
weight-channel decoding, accepted crossfade/interrupt snapshots and CPU/GPU
deformation integration remain required. Original RobotExpressive acceptance,
native visuals, frame-rate qualification and full engine readiness remain open.


## Local morph geometry evaluator: integration still pending

`voxy_model::MorphGeometry` owns an immutable source primitive and validated
position/normal target streams. Explicit evaluation applies additive deltas in
source-local coordinates before the existing skinning path, retaining topology,
UVs, colors, skin indices/UNORM16 weights and material configuration. Normal
deltas require base normals and remain unnormalized until subsequent normal
transformation. Negative and greater-than-one weights are preserved. Tangent
morphs are not represented yet; import must continue rejecting unsupported data.

The evaluator accumulates original f32 source deltas/weights in f64. Its wide
position output retains those results; renderer geometry converts at the existing
f32 boundary and rejects overflow. Target count/schema, finite data and a total
delta-vector budget are validated. Re-evaluation always starts from immutable
source geometry. `SceneMesh::with_deformed_geometry` preserves explicit material
coordinates and freezes implicit source positions as bind coordinates.

Focused regressions cover static and skinned geometry, an independently computed
nonuniform-scale/translation result proving morph-before-skin order, source
immutability, normal deltas, influence preservation, budgets and overflow.
This is an explicit CPU/reference geometry API, not an enabled default CPU
replacement for GPU animation. No shader, accepted playback snapshot or glTF
weight-channel publication is claimed. Default import still rejects morph data;
RobotExpressive full import/render and the full engine roadmap remain open.

See `morph-geometry-qualification.json` for exact check results and limitations.


## glTF morph import and unchanged upstream CPU reference

`MorphModelAsset::parse` now uses the SAME bounded `ModelAsset` importer with
an explicit morph-capable result wrapper. Its base asset stays private so a
consumer cannot accidentally feed undeformed geometry into skeletal-only
playback. Default `ModelAsset::parse` remains strict until accepted-frame and GPU
consumers preserve the complete morph payload. This is staged implementation,
not completion of the default runtime/editor import gate.

The importer retains source-local position/normal deltas, verifies target counts
across a mesh's primitives, and applies node weights over mesh weights over zero
defaults. Weight channels are decoded with the glTF target-count multiplier,
including CUBICSPLINE's per-key incoming/value/outgoing blocks. Per-component
tracks reuse `PropertyTrack` STEP/LINEAR/CUBICSPLINE; clip durations include both
skeletal and weight channels. Source sampler times and derivatives are retained.
No second playback clock is introduced. Explicit CPU sampling uses the same
supplied clip/time for both channels before the existing skinning owner.

Delta vectors are admitted before collection against a separate aggregate cap
equal to `ModelLimits.vertices`; scalar animation outputs consume the existing
key budget. Property-track/key caps also apply. Unsupported tangent targets,
sparse accessors, required extensions and existing unsupported material features
continue failing explicitly. No source mesh, target or animation is removed to
pass import. Static CPU geometry now retains authored/morphed normals through
inverse-transpose transformation, as well as material bind coordinates.

New import regressions cover initial-weight precedence, negative/overshooting
weights, separate mesh instances sharing source geometry, STEP and cubic layout,
normal morphing under nonuniform scale, explicit weight overrides, wrong counts,
budgets and unsupported tangent targets.

The public `voxy_model` example `morph_import` and runnable
`morph-import-proof.py` qualify the pinned original RobotExpressive GLB from cwd
`/`, with its original SHA-256 unchanged. The explicit CPU path imports 14 mesh
instances, 19 primitives (three with morph targets), a 160-entry combined palette
and all 14 clips. It evaluates 61 times per clip, 854 total samples, with finite
geometry. The exact source/binary hashes and per-clip results are retained in
`morph-import-proof.json`. No native window or GPU workload is launched.

IMPORTANT acceptance boundary: all 14 weight channels in this original GLB are
constant zero. Its ordinary clip sampling demonstrates preserved channel data
and skeletal movement, not animated facial expressions. To verify the original
shape deltas, the example separately supplies an additional weight of one for
each imported shape WITHOUT modifying the source file or its animation curves.
The three probes change 564, 564 and 494 vertices respectively. Changing weight
animation and cubic derivatives are covered by the explicit synthetic regression
fixtures; they are not attributed to the original asset.

Still required: GPU morph upload/deformation and resource admission, immutable
accepted morph snapshots sharing the existing clocks and fade/interruption
state, root-motion transaction rollback, reload/package wiring, editor controls
and native visual acceptance. The CPU reference path does not qualify default
RobotExpressive playback, frame-rate targets or full engine readiness.

A related curve bug was fixed while qualifying import: linear interpolation
between finite extreme f32 property values could overflow its intermediate
difference and silently hold the earlier key. Numeric property blending now
uses f64 intermediates before the existing f32 output boundary. A regression
checks opposite maximum scalar/vector/color endpoints at their zero midpoint.
This is a correctness change; no performance or universal exactness claim is made.

Validation for this import increment: full model library 41 passed before the
final linear-property arithmetic hardening; final focused morph regressions
6 passed after that fix; final animation library 331 passed; editor library
215 passed, 17 ignored. All-targets animation/model Clippy with `-D warnings`
passed. The final-source upstream reference proof was rebuilt and rerun from
cwd `/`; its source/binary digests are recorded. Check logs and exact boundaries
are retained in `morph-import-qualification.json` and `morph-import-*.log`.


## Morph properties in the existing Animator frame and transitions

`Animator::bind_property_clips` attaches immutable property clips/defaults by
EXACT skeletal clip Arc identity, not clip name. Binding validates rig, duration,
loop/clamp policy, schema, duplicate identities and clip budgets before changing
state. Missing channels retain the complete authored default schema. Rebinding
during a fade is rejected; source-revision reload integration remains pending.

`AnimatorFrame.properties` now carries the immutable property snapshot next to
pose/palette/root metadata. Property evaluation uses the existing f64 target and
source clocks, the same fade weight, and a captured property source on interrupted
fades. Completion samples only the continuing target. No new animation clock or
independent player is introduced. An unbound Animator retains an empty property
set and avoids property curve evaluation.

PropertyTrack/PropertyClip checked clock sampling preserves f64 key-boundary
comparisons and Hermite interpolation time instead of narrowing to f32. The
property output remains the existing typed f32 value. PropertyClip phase sampling
now respects clamp behavior rather than unconditionally wrapping every clip.
Invalid property interpolation rejects the same candidate as invalid skeletal
pose/palette; `advance_wall` commits its staged Animator only after both succeed.
Immediate and phased transitions preflight target properties before mutation.
Retargeting preserves frame properties while transporting skeletal transforms.

Prepared rigid fades retain the same property owner, accepted snapshot and frozen
source snapshot identities. Receipt matching includes these identities/state, and
accepted wall-time prefixes reconstruct property and skeletal frames together
from the original pre-tick Animator. This qualifies that Animator/plan layer, not
the editor's not-yet-enabled morph import/publication consumer.

`MorphModelAsset::animator` binds imported curves/defaults to this existing
Animator. Its CPU `scene_meshes(frame)` consumes frame pose AND properties without
re-sampling clocks. Missing/foreign property schemas fail instead of silently
falling back to zero/default morph weights. Static sampling APIs remain explicit
reference/authoring consumers. Default ModelAsset import still rejects morph until
GPU and editor accepted-frame/resource consumers preserve the complete payload.

Regressions cover live fades, two interruptions, zero-time continuity, completion,
unchanged earlier snapshots, unanimated defaults, STEP immediately before/at an
f64 key boundary, overflow/unknown-revision failure preserving clocks and values,
rigid-prefix properties and stale receipt rejection. Model integration verifies
imported animated weights deform the same frame and rejects a property-less frame.

The unchanged RobotExpressive reference additionally passed 112 Animator frames
across interrupted transitions between all 14 source clips. Its source morph
channels remain constant zero; explicit shape probes and synthetic animated-weight
regressions retain their distinct acceptance roles. No GPU/native image or FPS
claim is made. Final evidence: `morph-frame-proof.json` and
`morph-frame-qualification.json`. Full animation library 335 passed; editor library
215 passed, 17 ignored; focused property playback 4 passed and model integration
3 passed. Strict all-targets Clippy passed for animation/model/editor/demos/gameplay.

Next required integration: GPU morph payload and pre-skin deformation in the
existing SceneSkinner path with memory/device admission, then editor accepted
frame extraction, reload/package rollback, preview controls and native acceptance.
The broad engine goal remains open.


## Exact morph deformation in the existing SceneSkinner GPU path

`SkinnedMorphMesh` validates immutable source-local position/normal deltas and
weights without quantization, normalization or target truncation (explicit cap:
256). SceneSkinner now uploads shared morph deltas and creates independently
weighted instances. It uses a separate lazy morph pipeline only for these
instances; the existing non-morph shader/layout remains unchanged. Morph deltas
are applied before the original palette/normal transform in the same dispatch.
Mixed ordinary/morph instances can share a prepared compute pass.

The local morph sum uses exact integer arithmetic over original finite binary32
inputs/products: 36 signed radix-65536 digits in units 2^-298, followed by one
ties-to-even binary32 rounding. The digit capacity covers 256 complete source
products plus the original component. Integer term decoding reuses the existing
source-affine helper prefix; the ray-specific arena/solver is not included.
CPU preflight uses the same source convention; physical/reference positions use
an exact source sum rounded directly to binary64. This replaces the intermediate
f64 accumulation only in the new explicit morph feature. Existing f32 palette,
normal transformation and skinning behavior remains unchanged. It does not make
the complete rendering pipeline exact arithmetic.

Source buffers, delta buffers, instance weights, palette and scene geometry are
admitted against logical/device budgets. Source-size arithmetic is checked.
Invalid weights/counts/overflow and unsupported device limits reject before
queue writes. Plain skeletal entrypoints reject morph sources rather than
silently ignoring their weights. Prepared tokens borrow both palette and weights.
Batch submission rejects duplicate instance owners before any writes, preventing
two poses from overwriting one shared palette/weight buffer before dispatch.
Previously accepted instances remain owned on admission/preflight failures.
CPU preflight currently evaluates the exact local deformation; its cost is NOT
yet qualified for the target frame budget.

MorphGeometry shares the renderer's immutable delta type and exact numerical
convention. MorphModelAsset exposes an explicit skin-bound morph source adapter
for SceneSkinner. Single-mesh unskinned morph data still needs a rigid GPU binding;
the adapter rejects that case instead of dropping source transforms/features.
The original multi-mesh asset already supplies those bindings.

Physical GPU acceptance on Apple M4 Max/Metal:

- Two independently weighted instances match exact local position results,
  including `1e30 + 1 - 1e30 == 1`, negative and >1 weights; UVs and normal
  deformation/normalization are checked by GPU buffer readback.
- 41 arithmetic cases cover cancellation, intermediate products beyond f32
  range, subnormals, signed underflow, ties/sticky bits and 256 nonzero terms.
  Actual GPU bits and CPU binary32/binary64 results match an INDEPENDENT Python
  Fraction/quotient-remainder rounding oracle. This is recorded coverage and a
  capacity argument, not universal hardware qualification.
- The unchanged RobotExpressive GLB's three morph primitives pass 12 skeletal
  frames across Idle/Walking/Jump transitions with explicit shape overrides:
  20,904 posed vertices checked, maximum world-position error
  4.76837158203125e-7 against the CPU path (limit 1e-5 for existing f32 skinning).
  These overrides do not alter its original constant-zero weight channels.
  The example checks all morph-bearing primitives; it does not render/qualify a
  full-model image or an editor session.
- The existing ordinary skeletal GPU integration test still passes independent
  instance/readback, CPU color-image parity and last-good-pose retention checks.

Full renderer library: 358 passed, 86 ignored. Model morph regressions: 6 passed.
Editor library: 215 passed, 17 ignored. Strict all-targets render/model/editor
Clippy passed. Four pre-existing needless raw-string delimiter warnings were
removed without changing shader string bodies; their body digests are retained.
The foreign-skinner integration test now uses the constructor instead of a struct
literal, preserving its identity-rejection check with the new lazy pipeline owner.

Reproduce headless correctness with `morph-gpu-proof.py`; it builds/runs physical
readback tests and the public `morph_gpu` example from cwd `/`, checks original
asset/code hashes, and runs the rational oracle. Evidence is retained in
`morph-gpu-proof.json`, `morph-gpu-qualification.json`, oracle payload and logs.
No native game/editor window or FPS benchmark was launched for this increment.

STILL OPEN: enable the ordinary importer/editor only after accepted-frame GPU
extraction/reload/fallback preserve all morph data; source-free package replay
and rollback; static morph rigid bindings; morph-aware LOD/culling admission;
editor controls; full-model native visual acceptance and measured scene FPS.
The minimum 120 FPS / aspirational >160 FPS requirements remain unqualified.
The engine-wide objective remains open.


## Ordinary morph project/runtime/GPU-owner/package integration

The complete morph payload now belongs to the SAME `ModelAsset` via immutable
shared import data. `MorphModelAsset` is a compatibility/convenience wrapper over
that owner, not a second asset or player. `parse_animated` retains deltas, defaults,
source clip identities and property curves. The ordinary editor importer now uses
it, including morph-only meshes without skins/clips. Legacy `ModelAsset::parse`
intentionally remains strict for untouched pose-only consumers. Pose-only posed
mesh/surface APIs reject a morph asset instead of silently dropping weights.

Public frame-aware APIs include bind frames, inclusive authored-phase frames,
CPU posed meshes, GPU source adapters and explicit-weight wide surfaces. Wide
surfaces apply exact local source sums directly in f64 before wide skinning.
Public animation-vector reordering resolves property clips by retained Arc
identity, not a stale parallel index. Mutated imported morph source geometry/rig
or foreign clip identities fail explicitly. Current material selection is retained
when immutable source geometry is deformed.

Ordinary ModelPlayback binds imported curves to the existing Animator, including
bind-pose defaults. Reload registry entries retain their OWN defaults/curves for
old source revisions. New target revisions can register during an active/frozen
fade without replacing the displayed source; unused revisions are pruned and
registration remains budget bounded. A changed property curve/default cannot
masquerade as an unchanged skeletal-only revision. Registry changes invalidate
staged fade receipts. Changed delta bases are not interpreted with old indexed
weight sources: they require a new playback owner; morph retargeting across rigs
requires an explicit shape mapping and is currently rejected.

`AnimatedModels` consumes the same accepted frame weights. Its unchanged-frame
check now includes properties, so a weight-only animation updates GPU streams.
Morph sources/instances use SceneSkinner's shared delta and per-instance weight
buffers; all primitive poses validate before one prepared compute batch. An
explicit CPU fallback uses the complete frame, retaining geometry and material
coordinates. Bind previews and authored timeline previews also retain weights,
including the final authored key of looping clips. GPU resources and old accepted
render frames remain live when a replacement/schema/resource admission fails.

The simulation preparation path admits finite posed morph geometry before clock/
body publication, including physically accepted fade prefixes and corrected foot
frames. An extreme finite weight stream that overflows deformation rejects that
tick while retaining accepted body transform, animation phase/serial and frame.
This is separate from GPU-memory failure, which retains presentation resources
while the authoritative simulation follows its existing contract.

Integrated regression acceptance:

- A clearly SYNTHETIC rig with changing weight channels follows idle/move/rise/
  fall requests and crossfades through ordinary project import and ModelPlayback.
  Exact full accepted traces, including morph weights, match a package after
  deleting its source directory. The fixture does not establish realistic motion.
- Weight-only source reload enters a phase-preserving fade instead of retaining
  stale property curves. Malformed initial-weight counts keep the last good
  catalog asset AND accepted frame; repaired data recovers.
- Morph overflow preserves accepted pose/clock/body state. Shared-asset tests
  cover inclusive preview endpoints, source identity/reordering, geometry-layout
  guards, changed-basis rejection and explicit-weight f64 surfaces.
- Physical GPU owner tests verify weight-only output changes, exact CPU-fallback
  color-pixel parity on the bounded fixture, failed replacement retention and
  owner cleanup. These are headless tests, not native editor interaction.

The unchanged original RobotExpressive now passes ordinary project/controller
execution, build and source-free package execution from cwd `/`. After 240 fixed
ticks their scene and accepted animation/morph states match exactly: 160 palette
entries, 19 primitives and three morph-bearing weight vectors. Source bytes remain
at the pinned original digest. Its own weight channels are constant zero; dynamic
weight behavior is established by the explicit regression above, not attributed
to the upstream animation. The character collision setup in this proof is a
synthetic controller harness, not visual locomotion/root-motion/foot-contact
acceptance for the original rig.

A physical GPU test ALSO runs that original asset through the ordinary App import
and accepted ModelPlayback into AnimatedModels: all 19 primitives use the GPU,
including mixed normal/morph pipeline batches. Three frames/21,642 vertices match
the CPU reference within the existing f32 skinning limit 1e-5; observed maximum
error 4.76837158203125e-7 on Apple M4 Max/Metal. This is full-owner buffer readback,
not a full-model image, native editor or FPS result.

Validation: model full library 42 passed before the final changed-basis guard
refinement, then final focused morph tests 7 passed. Animation full library
336 passed. Final editor library 218 passed, 19 ignored; the two new physical
GPU owner tests were separately run and passed. Strict all-targets Clippy passed
for animation/render/model/editor/demos/gameplay. Evidence: reproducible
`morph-runtime-proof.py`, `morph-ordinary-owner-proof.py`, their JSON/logs and
`morph-ordinary-qualification.json`. Source-free package artifact is retained as
`artifacts/morph-runtime-2026-10-26/original-robot-regression.vpak`.

Still required for this domain: morph authoring controls and persistent overrides,
explicit cross-rig shape mapping, morph-aware LOD/culling/picking acceptance,
full native visual/interaction review and measured scene performance. Morph LOD
certificates are explicitly rejected before import/publication until they cover
the deformation; no silent simplification is used. Broad import/material coverage
and the rest of ENG-00–13 remain incomplete.


## Single unskinned morph mesh: common GPU owner qualified

The importer now binds a rigid morph mesh to its existing authored scene node.
It retains the original node hierarchy and animation without adding bones or a
second playback clock. Non-morph static import keeps its existing path. The
complete CPU fallback remains available for unsupported GPU capabilities.

Explicit synthetic regressions cover animated parents, mirrored nonuniform node
scale, position and normal targets, flat-normal tags and explicit-weight f64
surfaces. A no-skin/no-clips/no-ModelAnimation-component project preserves authored
weights through the ordinary runtime and a package after deleting its source tree.
The physical GPU owner test requires a real GPU primitive, verifies CPU color-pixel
parity, weight-only updates and last-good retention on the tested failures.

`rigid-morph-proof.py` and `rigid-morph-proof.json` record the checked code digests
and reproduce four focused checks plus the unchanged original RobotExpressive
ordinary-owner/source-free-package regression. Synthetic fixtures are identified
as synthetic; the upstream model is not stripped or rewritten.

This closes the single-unskinned-morph GPU binding gap only. Morph authoring and
persistent overrides, explicit cross-rig mapping, deformation-aware LOD/culling/
picking, full native visual acceptance and scene FPS qualification remain open.
The ENG-00–13 objective is not complete.


## Morph authoring groundwork: atomic frame edit API

`ModelAsset::set_frame_morph_weights` replaces all weights for an existing source
node in a complete accepted frame. All of that node's primitives share the same
property keys. The immutable asset, pose/palette and playback clock are retained;
values are not clamped or normalized. Candidate geometry is fully admitted before
publication. The regression verifies changed output, unchanged source defaults
and palette, and atomic rejection of nonfinite/count/node/overflow errors.

Focused model morph suite: 9 passed; strict model all-targets Clippy passed.
Evidence: `morph-authoring-core-qualification.json` and adjacent logs. This is an
API foundation only: scene override persistence, inspector controls, undo/redo,
preview/runtime/package integration and native authoring remain unimplemented or
unqualified for overrides. Earlier proof digests remain historical snapshots and
must be refreshed when qualifying the subsequent integrated path.


## Persistent authored morph overrides through the ordinary component owner

`ModelAnimation.morph_weights` is a default-empty, serialized map from an exact
unique authored node name to its complete weight vector. Legacy scene documents
without the field still decode. Overrides are absolute, preserve negative and
above-one finite weights, and apply after animation blending in the existing
`prepare_displayed_frame` path used by runtime and timeline preview. No second
clock or mutable shared model is introduced. Missing/ambiguous nodes, wrong
counts and overflowing deformation reject candidate frames. The common property
inspector exposes the complete map, including an empty map, and individual scalar
members; it uses the existing codec/history transaction rather than new UI state.

The synthetic integration test edits the map through the actual panel action,
undoes/redoes it, rejects empty weight vectors without document changes, saves,
reopens, runs locomotion transitions and builds a package. After removing the
source tree, every accepted override trace and the final scene match exactly.
Final editor library: 221 passed, 20 ignored. Strict all-targets editor/model
Clippy passed. Evidence: `morph-overrides-qualification.json` and adjacent logs.

This provides data-field editing, not a dedicated visual morph slider/target
picker. Native interaction, visible timeline/viewport deformation, reset-to-
animation UX, override behavior under node-renaming reloads, morph-aware LOD and
cross-rig shape mapping still require acceptance or implementation. No GPU/FPS
qualification was run for this change. Existing proof digests refer to their own
historical checked snapshots; empty-map serialization changes newly saved data.


## Morph source identity, discovered defaults and resume authoring

Fixed an integration defect: glTF source-node indices and topologically ordered
palette indices are distinct. Named overrides now resolve through immutable morph
source metadata, after checking unique authored-name identity. `morph_nodes()`
returns each source owner once, with its name/default vector, independent of the
number of primitives. A regression deliberately reorders the source nodes so the
morph owner index differs from the palette index, verifies the right owner, and
rejects missing/non-morph/ambiguous names.

The component inspector adds `Import morph defaults` and `Resume morph animation`.
The former discovers named morph owners and populates editable vectors without
manual name entry; unnamed/ambiguous owners fail before scene mutation. The
latter clears overrides so authored channels resume. Actions carry a document
identity token and reject stale owners; both use existing authoring history.
The integration test covers import, stale-token rejection, resume, resumed preview
versus sampled animation, undo, map editing, save/reopen and exact source-free
locomotion/package replay. It awaits asynchronous import through the existing
bounded tick loop; the first test attempt without that wait is retained.

Final focused model morph suite: 10 passed. Full editor: 221 passed, 20 ignored.
Strict all-targets model/editor Clippy passed. Evidence:
`morph-discovery-qualification.json` and adjacent logs. Buttons were exercised via
panel actions without opening a native window; readability/layout, dedicated
sliders and target labels, unnamed-node authoring and reload-renaming UX remain
open. No new GPU image or performance qualification is inferred.


## Ordinary authoring viewport now requests saved morph overrides

Fixed an integration gap: outside Play or an explicit timeline preview, viewport
request generation previously skipped animated owners entirely, so saved morph
overrides could be accepted in data/runtime while the ordinary editor displayed
imported bind geometry. Request generation now includes owners with authored
overrides and sends a complete bind-pose-plus-overrides frame to the existing
AnimatedModels owner. Timeline preview and accepted game frames retain precedence.
Removing overrides removes this authoring request and returns the ordinary bind
presentation. Candidate errors preserve the existing rendered owner.

AnimatedModels caches one authoring frame per generational owner, exact model Arc
revision and settings. Unchanged repaints reuse the Arc; settings/revision changes
replace it only after full frame admission. Synchronization prunes inactive owners;
explicit clear releases the cache. Models are weakly referenced by the cache, and
no new animation clock or GPU deformation system is introduced.

The no-GPU regression verifies bind palette/override values, cache reuse, owner
isolation, settings/revision invalidation, invalid-input retention and cleanup.
The first fixture mistakenly expected finite MAX weights with small source deltas
to overflow; those values are representable and remain accepted. That failed test
log is retained; the cache rejection regression now uses nonfinite input, while
existing model tests retain actual overflowing-deformation coverage.

Final full editor library: 222 passed, 20 ignored; strict all-targets editor Clippy
passed. Evidence: `morph-viewport-qualification.json` and adjacent logs. This checks
frame/cache behavior and implements native request wiring; no native viewport
image, new physical-GPU acceptance or FPS measurement was performed. Picking,
bounds/culling, per-target authoring UX and remaining ENG gates stay open.


## Picking follows complete deformed owner geometry

Model picking now evaluates all complete posed primitives instead of the imported
preview mesh. When a rendered owner exists, it supplies its accepted model Arc
revision and accepted frame together, including an older admitted revision retained
on failed publication. Without that owner, picking uses the accepted runtime
frame, selected timeline preview, or bind pose with authored morph overrides.
Expanded ModelPart picking retains its previous per-part contract. Triangle/depth
intersection and active-hierarchy checks reuse the existing picker; there is no
static-bounds early rejection that could discard an outlying morph target.

The synthetic App regression moves a triangle outside imported geometry, proves
that the imported mesh misses its new position, then verifies actual model picking
hits the deformed position and misses the old one. Both projection types, mirrored
nonuniform scene scale, timeline preview, accepted standalone frame and inactive
owner exclusion are covered. The first expanded test incorrectly retained an
authoring NodeId across Play; the corrected fixture fetches the runtime handle,
preserving the existing generation-invalidating lifecycle contract.

Full editor: 223 passed, 20 ignored; strict all-targets editor Clippy passed.
Evidence: `morph-picking-qualification.json` and adjacent logs. Native clicks,
visual comparison under GPU failure/reload, deformation-aware bounds/culling and
morph LOD remain unqualified. No new FPS claim follows from click-time CPU geometry
evaluation. This is an ENG-05/08 integration increment, not complete engine parity.


## Focus uses deformed display bounds, rather than the owner origin

Inspection of the animated-owner draw branch found no static-AABB frustum reject
to repair: it submits the admitted geometry. This does not establish an optimized
culling implementation. Conservative GPU bounds/culling remain a separate gate;
CPU reference bounds alone are not a certificate for GPU rounding differences.

The existing F action had a concrete authoring defect: it targeted the owner's
origin even when all visible geometry was elsewhere. It now computes the center
of all displayed primitive vertices in world space, accumulating bounds in f64,
and preserves the camera distance. Picking and focus share
`displayed_model_meshes`, which selects the accepted displayed model/frame pair,
then current runtime/preview/authored geometry when no presented owner exists.
Objects without model geometry retain origin focus. Camera state is changed only
after validating its limits; this is center focus, not automatic zoom-to-fit.

The existing synthetic App regression now exercises F with morph displacement,
mirrored nonuniform owner scale and both projections, checks the geometry center
instead of the origin, and verifies unchanged distance. All prior picking,
preview/runtime and inactive-object assertions remain. Full editor: 223 passed,
20 ignored; strict all-targets editor Clippy passed. Evidence:
`morph-focus-qualification.json` and adjacent logs. Bounds are computed on explicit
click/focus interactions, not per repaint. Native visuals, zoom-to-fit, conservative
GPU frustum/occlusion culling, morph LOD and FPS qualification remain open.


## F fits displayed geometry into the active rendered viewport

F now adjusts distance as well as center. `ViewportCamera::fit_bounds` keeps yaw,
pitch and projection type, derives distance from all eight world-AABB corners in
the current camera basis/aspect, and verifies the actual projection before
publication. It respects near/far planes and the existing camera limits. Invalid
or unframeable bounds fail without changing the camera. The App uses physical
active-view region dimensions, including split views; headless calls use 800x600.
Objects without model geometry retain origin focus.

Core regressions cover both projection types, portrait/landscape/wide aspects,
orbit orientation, degenerate point bounds and failure rollback. The existing
morph App regression now checks that every deformed vertex projects inside the
viewport after F, with mirrored nonuniform owner scale. It supersedes the prior
center-only increment's unchanged-distance behavior. Full editor: 225 passed,
20 ignored; strict all-targets editor Clippy passed. Evidence:
`focus-fit-qualification.json` and adjacent logs.

This is headless projection/interaction qualification. Native visual readability,
panel-overlay safe areas, split-view user interaction, extreme coordinate precision,
conservative GPU culling and FPS remain unqualified. The algorithm runs only on
explicit focus, not every frame; no geometry is simplified to achieve framing.


## Focus fitting accounts for editor panels and active split region

`ViewportCamera::fit_bounds_in_rect` fits world bounds into a physical-pixel
subrectangle. It adjusts target laterally as well as distance when the free area
is off center, preserving orbit orientation/projection. Perspective constraints
include each corner's depth; orthographic constraints use projected extents.
Actual corner projections are checked before publication. Invalid/empty or
unframeable rectangles fail without committing the candidate camera.

Side-panel dimensions now have one owner used by panel construction, pointer
viewport admission and focus. `safe_view_rect` intersects their logical extents,
converted with the window DPI, with the active physical render region. It handles
odd split widths and rejects fully obscured views. Focus uses this rectangle when
editor panels exist; standalone/no-panel focus retains full-view fitting.

New CPU tests project every bound corner inside off-center rectangles for both
projection types and rotated cameras, including rectangles excluding the render
center. Layout tests verify 1x/2x DPI, odd split regions and hidden views. Existing
App morph-focus/picking regressions remain passing. Full editor: 227 passed,
20 ignored; strict all-targets editor Clippy passed. Evidence:
`focus-safe-qualification.json` and adjacent logs. Native screenshot/input review,
fractional-DPI device execution, extreme coordinate cases, GPU culling and scene
performance remain unqualified; this does not close the full ENG-08 stage.


## Animation panel GPU image and hit-overlap correction

The ignored physical-GPU panel test now includes the two morph commands and checks
all seven animation controls against every other hit region, rather than only
scalar fields. It reproduced an overlap: timeline at y=334..358 intersected the
Character action starting at y=354; the morph commands also occupied lower actions.
Component-mode lower actions now begin after the morph controls, and keyboard hints
move below that group. Shorter morph/animation labels avoid clipped wording.

The native SceneRenderer rendered the actual panel geometry and glyph atlas to a
1024x640 offscreen target on Apple M4 Max/Metal. All seven controls have glyph
pixels, hit regions do not overlap, and the captured image was inspected after
fixing overlapping hints and clipped labels. `animation-panel-gpu.png` retains the
final image; `animation-panel-initial-failed.log` preserves the reproduced defect.
The test was separately run and passed; it remains ignored in routine headless CI.

Full editor: 227 passed, 20 ignored before final label shortening; final physical
GPU test and strict all-targets Clippy passed after that shortening. Evidence:
`animation-panel-qualification.json` and adjacent logs. This qualifies one panel
image and bounded hit layout, not native-window input, other panel sizes/DPI,
a full engine scene or FPS. ENG-08 and broad production readiness remain partial.


## Original upstream asset regression refreshed after authoring integration

The ordinary project/package/GPU-owner proof was rerun against the current model,
playback, runtime and rendered-owner source digests after the morph authoring,
viewport and interaction changes. The original RobotExpressive bytes remain at
the pinned SHA-256. Project and package scene/animation states match exactly after
240 controller ticks with 160 palette entries and all 19 primitives, including
three morph streams. The source tree is removed before packaged execution.
The original authored weight curves remain constant zero; dynamic weight behavior
is established separately by the synthetic regressions, not attributed to this rig.

Physical GPU owner qualification passed again: all 19 original primitives use the
GPU; three frames/21,642 vertices match the CPU reference with maximum position
error 4.76837158203125e-7 (existing limit 1e-5). The separate weight-only GPU test
also passed CPU color-pixel parity and failed-replacement retention.

The driver now retains new packages by full content SHA-256, preserving the older
`original-robot-regression.vpak` byte-for-byte instead of treating changed scene
serialization as permission to replace it. Previous proof JSON/logs are archived
under `evidence-history/` by digest before current results are refreshed. Current
package: `artifacts/morph-runtime-2026-10-26/original-robot-c06ec15b350618c8d13bbc941f63e4f7962f56d5f83ffca0f0dbd24082d6c80c.vpak`.

Evidence: `current-original-regression-qualification.json`, refreshed runtime and
ordinary-owner proofs/logs. All checked source digests and package hashes were
revalidated after completion. This is updated real-asset integration evidence,
not a complete native image, editor interaction, realistic foot contact, clean-
machine deployment or 120/160-FPS result. The full ENG-00–13 objective remains open.


## Game-save integration inventory: full checkpoint remains open

The existing SceneDocument snapshot is registered scene data, not a complete game
save. The ENG-07 checkpoint contract must capture all active owners at one accepted
tick and prepare a replacement session before publication:

| Required state | Current owner | Remaining integration gate |
|---|---|---|
| Durable objects/components/references | SceneDocument / ComponentRegistry | One scene/SceneSimulation file and staged pair implemented; outer game/asset/project identity remains open |
| Controller state | CharacterPhysics | CharacterPhysicsCheckpoint captures exact bits and resolves durable IDs; core/bounded-codec tests passed; whole-session integration remains open |
| Skeletal/morph playback | AnimationRuntime / ModelPlayback / Animator | Persist clocks, source revisions, fades/frozen sources, events and accepted frames without duplicate emissions |
| Fixed-step scheduler | PlaySession / SceneSimulation | Exact clock, interpolation-history and command-queue participants implemented; external tick counter and whole-session publication remain open |
| Behaviors/game state | BehaviorRunner / GameHost | Opt-in codecs and lifecycle restore now wired through GameHost project/build/package paths; whole-session envelope/publication remains open |
| Liquid/angular/support state | SceneLiquidRuntime / angular_motion | Preserve full physical state and dependency identity, rather than rebuilding a rest state |
| UI/input/camera/audio | Current game owners | Define capture/restore and external-device/input policy; avoid replaying transient actions |
| Disk slots/migrations/public API | Not integrated | Bounded reads, version/project/asset compatibility, atomic files, last-good retention, source-free package replay |

No combination of scene snapshots and the controller checkpoint alone closes this
contract. Native save/load controls and clean-machine delivery remain separate gates.


## Controller checkpoint implementation: focused pass, broad checks in progress

CharacterPhysics now captures a versioned, deterministic checkpoint keyed by
caller-supplied durable ObjectId values. It stores the complete cached RuntimeBody,
including anchored AABB, f64 velocity/orientation/edge bases, grounded state,
published f32 pose, cached descriptor and solver policy. Floating-point fields use
integer bit representations in JSON; no normalization, narrowing or re-derivation
of physical state occurs on restore. Matching scene data must be restored by the
caller at the same accepted boundary. This is not a full game checkpoint.

Restore resolves NEW handles, validates target scene/descriptors, version/budgets,
finite and nondegenerate state and unique identity/handle bindings, and prepares
the full state map before replacing the live controller. Rejected input leaves
controller state and scene unchanged. Inactive cached owners are included; external
scene/other-owner restoration is deliberately not inferred from physics state.

Two focused tests passed: JSON restore into a reordered/new scene followed by
180 exact steps through falling and landing, and rejection/rollback with malformed
versions, nonfinite state, duplicate identities and missing bindings. A sub-ULP
f64 velocity bit pattern survives JSON unchanged. The same two tests passed within
the currently running full gameplay suite. At this update, two existing liquid
support tests are still executing in live session 38935; editor and strict Clippy
are queued after it. Do not count these broad checks as passed or restart based on
a stale log. Qualification JSON records the pending state and process log path.

Remaining: one checkpoint envelope and bounded/atomic disk slots, matching scene
and asset identity, animation/simulation/behavior/liquid/UI/input/camera/audio
participants, package replay and native save/load controls. ENG-07 remains open.


## Controller checkpoint broader validation completed; bounded codec added

The previously live session 38935 completed successfully, including both long
liquid/support regressions: 183 gameplay tests passed, then 227 editor tests passed
with 20 ignored, and strict gameplay/editor all-targets Clippy passed. No pending
process remains from that run. After adding the bounded codec wrapper, all three
checkpoint-focused tests and final strict Clippy passed again.

`CharacterPhysicsCheckpoint::from_json` rejects the input byte budget before
parsing, then admits body count, version/policy, identities and every decoded
physical state. `to_json` rejects invalid state and refuses an oversized returned
record. The outer save owner still must bound file IO before producing input text;
the codec does not claim a physical allocation/RSS bound for serde parsing or
encoding. Unknown fields, oversized byte/body inputs and nonfinite bit patterns
are covered. Direct serde callers remain responsible for their own IO limits;
restore independently validates checkpoint state before controller publication.

Evidence: refreshed `character-checkpoint-qualification.json`, full gameplay/editor
logs, final codec/Clippy logs. The old pending report is preserved in
`evidence-history/`. Full gameplay qualification predates the final codec wrapper;
the final wrapper has focused coverage, not a claimed second full gameplay run.
A full game-save envelope, atomic disk slots, matching asset/scene identity,
simulation/animation/behavior/liquid/UI/input/camera/audio state and package/native
restore remain required. This checkpoint subdomain does not complete ENG-07.


## Exact simulation clock checkpoint and honest tail-latency telemetry

SimulationClock exports nine integer words: version, fixed-step bits and exact
scale/target/spring velocity/remainder/damping/last-drop bits. It introduces no
serde dependency in voxy_time; an outer format can serialize integer words.
Restore validates version, exact step, finite/safe numeric state, remainder,
damping and drop invariants before replacing the clock. SceneSimulation's ten-word
wrapper additionally binds the catch-up work budget and rejects foreign/stopped
scenes. This restores only the clock; interpolation pose history, behaviors,
commands, input and the external game tick counter remain separate participants.

A 720-frame continuation with speed targets, pause/resume and overloaded/zero
elapsed inputs matches uninterrupted scheduling, alpha and complete clock bits.
A JSON roundtrip/new-scene test matches 120 further scheduling frames, including
dropped real/scaled time, and verifies budget, foreign-scene, corrupt and stopped
rejection. No failed restore changes the live clock.

The full time suite exposed an existing telemetry defect: floor((n-1)*p/100)
excluded a single maximum from p99 in a 13-sample window. p95/p99 now use integer
nearest rank ceil(n*p/100)-1; lower-median behavior is unchanged. Singleton, short
and ring-replacement tests passed. The reproduced failure log is retained; this
changes the helper's statistic, not engine performance or prior external benchmark
raw data. Current source search found no production consumer of this helper.

Final validation: time 8 passed, scene 75 passed, editor 227 passed/20 ignored;
strict all-targets Clippy for time/scene/gameplay/editor passed. Evidence:
`clock-checkpoint-qualification.json` and adjacent logs. Full game-save integration,
disk slots and all other runtime owners remain open; ENG-07 is not complete.


## Scene presentation checkpoint preserves interpolation after loading

SceneSimulation now captures/restores its exact clock words, f32 interpolation
alpha and previous/current local TRS histories. Pose owners and historical parent
references use durable ObjectId values; float values use integer bit patterns.
New scene handles are resolved consistently across both maps with duplicate/alias
rejection. Deleted-node or deleted-parent cache entries are omitted only where
they cannot affect render interpolation or the following fixed-tick capture.

Restore validates version, clock configuration, history capacities, identity maps,
alpha and local transforms into candidate data. A presentation-only candidate
(with empty behavior/command owners) runs the existing cached world composer over
all live nodes before publication. Finite local values that overflow after parent
composition are rejected; no user hooks or scene writes occur in that check.
Only then are the live clock, histories and alpha replaced together. Original
behaviors and pending commands remain untouched and must be separate participants.

The JSON/new-reordered-scene regression verifies the first interpolated world
matrix bit-for-bit (and proves it differs from the authoritative current pose),
then 120 further moving/rotating-child frames and scheduling results. Additional
checks cover dead historical parent pruning over zero/following ticks and atomic
rejection of invalid pose/alpha/identity/alias/composed overflow. Full scene:
78 passed; editor: 227 passed, 20 ignored; strict scene/time/gameplay/editor
all-targets Clippy passed. Evidence: `presentation-checkpoint-qualification.json`
and adjacent logs.

This is a scene-presentation participant, not a whole game save. Matching scene
and asset identity, external tick count, behaviors/commands, animation, physical
systems, UI/input/camera/audio and bounded atomic disk slots remain required.
Native save/load interaction and scene performance remain unqualified.


## Gameplay input participant and HeadlessGame consumer

InputMap now captures versioned bindings, dead zones, exact analog/control value
bits, focus and held/pressed/released state. It preserves quick physical taps and
logical activations pending a successful fixed-step consumer. Public checkpoint
records are codec-neutral, keeping voxy_input dependency-free; an outer save must
provide bounded file encoding and adapter/project compatibility checks.

Restore builds a candidate under the existing map's capacity policy, rejects
versions, duplicates, unbound controls and invalid numbers, then uses the original
recompute path to verify saved values/holds/edges against bindings and focus. It
publishes only the complete validated map. Focus loss, device disconnect and frame
consumption keep their existing behavior; no saved input event is replayed through
native event dispatch. UI pointer capture and UI focus remain separate owners.

HeadlessGame exposes input_checkpoint/restore_input_checkpoint as explicit
participant operations, not whole-session save/load. The integration test keeps
a quick jump and held movement through restore into an otherwise equal game,
then compares scenes and input for 120 subsequent fixed steps, proving the jump
was consumed and not lost. Core tests cover analog input, logical taps, focus,
disconnect, finish_frame, invalid/inconsistent data and capacity rollback.

Final input library: 8 passed; editor: 228 passed, 20 ignored; strict input/gameplay/
editor all-targets Clippy passed. Evidence: `input-checkpoint-qualification.json`
and adjacent logs. Full checkpoint envelope, byte-bounded atomic slots, adapter
compatibility, UI and remaining participants are still required. This does not
qualify native device reconnect, full game loading or FPS.


## Explicit Rust behavior save contract and candidate factories

The whole-session path audit found no behavior-state contract. Behavior now has
an opt-in checkpoint method whose default explicitly refuses unsupported types.
BehaviorState carries a durable codec name and opaque payload bytes; the enclosing
JSON does not reinterpret those bytes. A JSON convenience helper is available,
but behavior codecs own version migration and precise numeric/private-state
encoding. No executable code is serialized or instantiated without a registered
host factory.

BehaviorCheckpoint retains attachment order, durable owners and enabled/started
flags. Bounded JSON admission checks encoded bytes, count/version/schema and
missing identities. BehaviorCheckpointRegistry stages new instances using either
a registered serde type or a custom factory with readonly scene/ObjectId context.
Unknown codecs, failed payloads and unresolved/aliased owners reject preparation.
Factories receive no mutable scene access; user factory side effects/panics cannot
be rolled back and must not touch external live state.

SceneSimulation captures this participant and installs a prepared runner only
into a fresh empty simulation. Existing live behaviors are never silently replaced
or dropped. Preparation invokes no awake/start/enable hooks; saved lifecycle flags
resume naturally on subsequent sync/update. Pending commands remain a separate
participant and are not inferred from behavior state.

Four new tests cover JSON/new-scene restoration of two behaviors in order over
120 further fixed/variable updates, active/inactive transitions, no repeated
awake/start, unsupported types/codecs, missing owners, corrupt payloads, budgets,
nonempty target protection, and a custom codec rebinding its internal target to a
new runtime handle while preserving opaque bytes. Full scene: 82 passed; editor:
228 passed, 20 ignored; strict scene/gameplay/editor Clippy passed. Evidence:
`behavior-checkpoint-qualification.json` and adjacent logs.

The original intent remains a complete save envelope and staged session load,
not a collection of checkpoint APIs. This behavior contract closes a prerequisite;
GameHost codec setup, pending commands, animation/other participants, one matching
scene/asset envelope and atomic disk slots remain open. ENG-07 is not complete.


## GameHost carries behavior codecs through the ordinary project/package path

GameHost now owns a shared immutable BehaviorCheckpointRegistry. Project/editor,
headless game, export preflight and project-package launch install that same host
capability rather than retaining only component codecs and setup hooks. Legacy
model packages explicitly reject nonempty behavior checkpoint registries, since
that path has no project-host contract; default empty registries remain compatible.
No factory/code is serialized into the resource package.

HeadlessGame exposes explicit behavior-participant capture/restore using durable
object IDs. Restore uses the host registry and the fresh-empty-runner gate; it does
not replace live behaviors or imply restoration of scene/time/physics. The new
integration test shares the configured registry with an App, captures behavior
state through JSON, builds the project, removes its source directory, then restores
into a packaged game with codecs but without startup behavior attachment. The next
120 scene snapshots and behavior records match the original run exactly. Unknown
codecs and nonempty target runners fail without changing their prior checkpoint.

The public project_game example now opts its Move behavior into a versioned codec,
preserving speed through f32 bits and checking its saved range in the factory.
`--behavior-check` runs 120 headless ticks and checks participant JSON roundtrip.
It was built and run from cwd `/`: exit 0, 278 encoded bytes, fixed_ticks=120.
This CLI is explicitly participant-only (`full_game_save=false`), not a load of
all runtime systems.

Final scene: 82 passed; editor: 229 passed, 20 ignored; strict scene/gameplay/editor
all-targets Clippy passed after example changes. Evidence:
`host-checkpoint-qualification.json`, focused/full logs and real example CLI result.
The root save envelope, staged whole-session publication, atomic disk slots,
commands, animation and remaining physical/UI/audio participants remain open.
Native input/window and FPS qualification are unchanged.


## Pending scene commands preserve order, component intent and barrier failures

SceneCommands now snapshots version/capacities, every queued structural/name/
component command and the previous barrier's spawn journal. Live handles become
ObjectId references, consistently rebound in the target scene. Handles already
invalid in the source become a guaranteed-invalid scene-zero sentinel, preserving
ordinary per-command validation/error ordering without resurrecting stale nodes.
Transform fields use raw f32 bits, including invalid transforms intentionally
waiting to fail at apply. The target uses the original apply/coalescing path.

Typed insert/remove snapshots use the EXISTING ComponentRegistry's exact type
mapping and codecs. Registered decoder callbacks rebuild ordinary typed queue
operations, so component revisions/index invalidation and error semantics retain
the existing implementation. Unregistered pending components explicitly reject
capture without consuming them. Queue/scene capacity mismatch, unresolved/aliased
identities, unknown schemas, payload errors, versions and byte/count limits reject
candidate preparation without replacing the live queue or writing the scene.
Insert and Remove have distinct tagged wire operations: an inserted null-valued
component must never deserialize as removal. The initial failing regression is
retained alongside the repaired test.

The component roundtrip test exposed a separate f64 parser precision defect:
0x723f4114006c08c7 lost one low bit through the default JSON parser. voxy_scene now
requests serde_json's float_roundtrip feature. A bounded 1024-pattern regression
covers the actual queued component encode/decode/apply path, not a string-only
format test. This is numerical preservation, not a performance claim or universal
component-codec proof; user codecs still own their serialized representation.

Four tests verify new-scene JSON command order, mixed successful/error results,
registered component operations, spawn journal, removed-slot reuse, null insertion
versus removal, no double application, rollback and floating-point preservation.
Final scene: 86 passed; editor: 229 passed, 20 ignored; strict scene/gameplay/editor
Clippy passed. Evidence: `command-checkpoint-qualification.json`, current logs and
the initial f64/null failure logs. Whole-session envelope, staged publication,
animation/other participants and bounded atomic disk slots remain required.


## One file for scene data and the complete SceneSimulation owner

SceneSimulationCheckpoint now combines registered SceneDocument data, exact arena
allocation metadata, clock/presentation history, opted-in behavior instances and
pending commands. SceneSimulation::checkpoint captures matching LoadedScene data;
prepare/load_file returns a fully prepared NEW LoadedScene/SceneSimulation pair.
The existing live pair is not borrowed mutably during preparation. This is the
scene/simulation owner, not the full App/PlaySession or every gameplay subsystem.

SceneDocument alone sorts durable objects and loses allocation holes, LIFO free
order, slot generations, sibling ordering and deleted-ID reservations. The new
SceneAllocationCheckpoint restores those metadata only inside a private fresh
scene before any handles/factories are published. Components remain owned values
moved from the validated document load; no generic component clone or second
serialization registry is added. Parent/child sets, unique identities, free slots
and reserved IDs validate before node movement. SceneId stays fresh, so old handles
remain invalid even when slot/generation layout is retained. Retired identities
stay unavailable for reassignment without being rebound to live objects.

save_file validates every participant into a private candidate before reusing the
EXISTING same-directory atomic scene writer. load_file bounds reads at max_bytes+1
before parsing/preparing. Step/catch-up/behavior/queue/scene-capacity compatibility
is enforced. Pre-publication failures retain the previous file; post-rename
filesystem durability errors retain the existing committed=true reporting. Outer
encoded byte limits are not a claim of a dedicated physical heap/RSS budget.

The comprehensive test writes/reloads a real file with two allocation holes,
nondefault sibling order, a reserved deleted identity, a running behavior,
interpolation and pending typed writes/spawns/invalid transform. The first rendered
world matrix matches bit-for-bit; 120 following frames match scene data, node
iteration order after LIFO slot reuse, behavior state, interpolation and command
results. Old source handles fail in the new scene. Additional tests verify
malformed layout/configuration, unknown codecs, byte limits and last-good file/
live-pair retention. The source's pending writes are not consumed during save.

Final scene: 89 passed; editor: 229 passed, 20 ignored; strict scene/time/gameplay/
editor Clippy passed. Evidence: `scene-save-qualification.json` and adjacent logs.
The initial Clippy clone-assignment correction is retained in its diagnostic log.

Remaining for the requested whole-game save/load: App/PlaySession publication,
physics and input participant bundling, animation/morph clocks/fades/events,
liquid/angular/support state, UI/focus/camera/audio, asset/project compatibility,
external tick count and user-facing slots/migrations. Native editor/game loading,
clean-machine packages and FPS are unqualified. ENG-07 and ENG-00–13 stay open.

## Gameplay input file codec

The gameplay layer now exposes `encode_input_checkpoint` and
`decode_input_checkpoint`, leaving voxy_input independent of serde/file formats.
The versioned representation denies unknown fields, uses integer float bits,
bounds input text before parsing, and admits the candidate through the original
InputMap validation under caller-supplied action/binding limits. File-read limits
and platform adapter compatibility remain the enclosing game owner's contract.

Two focused codec tests and strict all-target gameplay Clippy passed. The existing
headless game integration now writes and reads a real JSON file before restoring
pending quick-jump input; the following 120 scene/input frames match. This test
starts both sessions at the same physics state; it does not prove whole-game load.
Evidence: `production-readiness-evidence-2026-10-26/input-file-qualification.json`
and its three logs. No full-suite, native UI, or FPS claim is made by this gate.

## Staged gameplay checkpoint: scene, simulation, character, input and tick count

`voxy_gameplay::GameplayCheckpoint` combines the existing participants in one
versioned JSON envelope. `prepare` compares a caller-supplied compatibility key,
admits input, prepares the scene/simulation pair, then restores character state
against fresh handles. Only the complete owned `PreparedGameplayCheckpoint` is
returned; no existing live pair is borrowed mutably. Admission limits are supplied
by the loader. The caller must bound file IO and derive a key covering project,
assets, executable schemas and platform adapter; this API does not derive identity.
Custom behavior factories must obey their existing no-external-side-effects contract.

Two new tests passed, including mid-fall restore, pending quick-tap input, landing
and another jump over 240 varying-delta frames. Scene/simulation JSON, solver
state, input and external tick counters match throughout. A failure during the
late physics participant and mismatched compatibility leave the source unchanged.
The scene suite passed 89 tests and strict gameplay all-target Clippy passed.

A runnable public consumer is `crates/voxy_gameplay/examples/gameplay_save.rs`:
`cargo run -p voxy_gameplay --example gameplay_save`. The actual run returned
`GAMEPLAY CHECKPOINT PASS bytes=3050 following_frames=240 ticks=292 full_app_save=false`.
It checks the whole supported checkpoint after every continuation frame. This is
a window-free JSON roundtrip, not file durability, native interaction or FPS proof.
Logs and source hashes: `production-readiness-evidence-2026-10-26/gameplay-save-qualification.json`.

Still open: native App/PlaySession ownership/publication, animation transitions and
events, audio playback/mixer, UI focus and capture, follow camera, angular/liquid
owners, derived project/resource compatibility and complete user-facing save slots.
The current unit is deliberately scoped to its explicit participants and does not
close ENG-07 or any other complete engine milestone.

## Animator playback checkpoint

`Animator::checkpoint` and `AnimatorCheckpoint::{to_json,from_json,prepare}`
now retain target/source clip clocks, speed, selected motion joint, active fade
time, interrupted-fade frozen local pose, frozen properties, accepted properties,
and each bound property revision's defaults. Floating-point data use integer bits.
The enclosing game supplies exact immutable skeletal/property clip revision keys
and resolvers; display names and process pointers are not persisted identities.
Capture rejects keys that alias different source clips. Prepare validates the
rig, clocks, finite nonsingular pose, binding schemas and sampled properties,
evaluating a clone before returning the new animator. Parsing is byte bounded;
underlying file IO and actual project/asset compatibility remain caller duties.

Three checkpoint tests cover reconstruction against independently recreated assets,
interrupted fades with morph/property sampling, pause/resume, subsequent switching,
and 240 exact continuation frames; an ordinary unbound fade into clamped playback
matches another 120 frames. Invalid clocks, missing revisions, ambiguous capture
keys and byte limits fail without replacing source playback. The complete animation
suite passed 339 tests; editor passed 229 with 20 ignored. Strict all-target Clippy
for animation and editor passed. Evidence and source hashes are retained in
`production-readiness-evidence-2026-10-26/animator-checkpoint-qualification.json`.

This is the Animator participant. ModelPlayback event tracks/pending events/contact
intervals, AnimationRuntime owner bindings/feet/root references/cached displayed
frames, native App staging and the complete gameplay envelope integration remain
open. It does not prove whole-game save/load, native image correctness or FPS.

## ModelPlayback checkpoint: event delivery and contact history

The editor's existing ModelPlayback owner now has a staged JSON participant
containing its Animator checkpoint, event tracks/windows, undelivered occurrences,
root motion joint, target and moving-source contact intervals, and frozen-source
pose plus accepted tick fraction. Restore rebuilds a separate playback against
the caller's verified model and exact clip/property revision resolver. The model
identity, asset archive and publication of the full animation owner remain outer
contracts; absent old revisions are rejected rather than silently rebound by name.

PoseCheckpoint is shared with Animator's frozen source format (the serialized
array representation remains unchanged). Its preparation validates joint count,
local TRS and composed skin palette before returning a pose. Model participant
validation covers byte/version budgets, missing tracks/resources, event limits,
invalid intervals/fractions, foreign source rigs and conflicting source modes.
There is no arbitrary normalized-time cutoff on short clips' loop spans.

Two new tests passed. A saved interrupted fade with pending events is restored
against a freshly parsed model; 240 following frames, palettes, root motion and
complete checkpoint JSON match, with queued events drained exactly once. Another
test retains a moving source contact interval, admits bind playback and rejects
corrupt fractions and unknown event clips without modifying the original. Full
validation: animation 339 passed; editor 231 passed, 20 ignored; strict all-target
Clippy for both crates and git diff check passed. Logs and hashes are retained in
`production-readiness-evidence-2026-10-26/model-playback-checkpoint-qualification.json`.

Still required: AnimationRuntime owners, model/retarget identity, foot placement
history, root references, cached accepted frames and pending motion/rotation data,
then transactional integration with native App and the other save participants.
This is not a full App save or native UI/FPS qualification.

## Durable support contact and FootRuntime history

SupportAnchorCheckpoint retains ObjectId plus exact local-coordinate/sign bits and
face index. Preparation rebinds to new scene handles, without reprojection. Normal
support resolution retains ownership of deciding whether changed/inactive geometry
still supports the anchor. Deleted/unidentified support owners are currently an
explicit capture/restore error; no contact is silently dropped. FootContactCheckpoint
retains both the anchor and awaiting-swing hysteresis, with bounded JSON codecs.

FootRuntimeCheckpoint retains authored settings, accepted contact states/pivots,
pending pivot requests, actual reference geometry in deterministic point order,
last contact weights and shared frozen-pose keys/weights. Geometry is captured from
the active runtime, including overrides installed after binding. Frozen poses must
be resolved through the enclosing transaction's shared identity table, preserving
the same Arc used by restored playback; independent content copies are insufficient
for interruption/contact history. Duplicate keys for different captured poses fail.
The outer shared-pose archive and full animation transaction are not yet implemented.

Imported models restore through ModelAsset's authored-name resolver. Native rigs
restore through their skeleton names. The initial imported-name failure is retained
in the evidence; rebuilding every foot through skeleton names was incorrect. The
new path uses the same constructors as the respective original owner. Candidate
validation rejects malformed pivots, distances, weights, missing support/snapshot
identities and conflicting reference geometry/pivot ownership. Existing owners are
never borrowed mutably by preparation.

Four new tests passed: ObjectId rebinding and moving platforms over 180 steps;
awaiting-swing/replant policy and corrupt anchors; complete foot-history JSON
roundtrip against a fresh model with pending pivot/geometry/shared snapshot and
180 contact-history updates; native binding and malformed history rejection. These
are contact/history continuation tests, not a full restored animated IK/game run.
Full animation suite: 339 passed. Editor: 233 passed, 20 ignored. Gameplay contact
filter: 10 passed; support filter: 29 passed. Strict all-target Clippy for gameplay,
animation and editor passed. Evidence/hashes: `production-readiness-evidence-2026-10-26/foot-checkpoint-qualification.json`.

Remaining: full AnimationRuntime checkpoint (owner identities, retarget bindings,
root reference enclosures, shared immutable snapshots, cached accepted frames and
pending motion/rotation data), integration with App/PlaySession and other gameplay
participants. No native interaction, rendered-image or FPS qualification is claimed.

## Shared frozen-pose archive across animation participants

PoseSnapshotBuilder interns live Arc allocations, retaining them during capture to
prevent address reuse. The versioned PoseSnapshotArchive serializes stable IDs,
caller-supplied immutable rig revision keys and exact pose bits. Repeated references
share one entry; equal-valued distinct allocations remain separate. Byte/count
budgets, unique IDs, rig resolution and all poses/palettes are admitted before
PreparedPoseSnapshots is returned. A failed later entry cannot modify an already
prepared table. Actual rig/project identity must still be verified by the caller.

Animator and ModelPlayback checkpoint variants can now capture/restore frozen pose
references through this shared archive. Inline checkpoint APIs remain available;
restore rejects conflicting inline/key representations and missing/foreign shared
snapshots. FootRuntime already consumes shared keys. The new combined consumer
test writes one JSON envelope for archive/playback/foot history, rebuilds the model,
and verifies one Arc is shared by the restored Animator source, ModelPlayback's
frozen accepted tick and foot contact blend history. The retained frozen contact
weight differs from the latest blended weight, so an erroneous pointer-identity
reset is observable. Following 240 frames, including further interruptions, retain
matching poses, palettes, root motion, contact weights and event delivery.

Two new tests passed. Full animation suite: 340 passed. Editor: 234 passed,
20 ignored. Strict all-target animation/editor Clippy and git diff check passed.
The initial test-only wildcard lint failure and corrected final run are retained.
Evidence/hashes: `production-readiness-evidence-2026-10-26/shared-snapshot-qualification.json`.
This is shared animation/contact continuation, not full restored character IK,
file durability, native frame presentation or FPS qualification.

Remaining for AnimationRuntime: complete owner/asset/retarget bindings, root
reference enclosures, cached accepted frames and pending motions/rotations, then
transactional publication with scene/physics/input and all remaining App owners.
The new archive removes one concrete integration dependency; whole-game save and
the full ENG-00..13 roadmap remain incomplete.

## Accepted AnimatorFrame checkpoint after owner corrections

AnimatorFrameCheckpoint retains the already accepted local pose, all animated
properties, cached skin matrices, root displacement/joint and transition weight.
Its versioned JSON encodes floats as integer bits. Capture verifies the source
pose against the supplied target skeleton. Preparation validates the pose and
requires exact palette agreement with that pose on the current backend before
returning the stored frame, without advancing an Animator or reapplying IK. The
enclosing owner must supply the verified target rig revision; a pose's joint
count or a passing palette comparison is not an asset identity certificate.

Two new tests passed: exact roundtrip including subnormal and signed-zero property
values, vector/color/bool properties, mirrored/nonuniform pose scales and root
metadata; rejection of a one-bit palette mismatch, invalid displacement/joint/
transition weight, byte budgets and inconsistent live cached palettes. The existing
physical foot-placement integration now saves an accepted frame after swept
character/IK correction, restores against a freshly imported model and verifies
identical complete frame JSON and solved sole-coordinate bits. It still exercises
its original rejection/clock-preservation checks.

Full validation: animation 342 passed, editor 234 passed with 20 ignored; strict
all-target Clippy for both crates and git diff check passed. Logs and hashes:
`production-readiness-evidence-2026-10-26/accepted-frame-checkpoint-qualification.json`.
This is CPU accepted-frame state qualification, not native GPU image/FPS proof.

RootReference inspection confirms it stores RootRigidEnclosure and
RootUniformScaleEnclosure, including outward interval bounds. A future codec must
retain their proof-bearing bounds exactly; reconstructing an ordinary transform
is not a completed checkpoint for these objects. Root references, pending certified
rotation paths, full AnimationRuntime owner/asset/retarget identities and App/game
transactional publication remain open.

## Exact root-reference interval checkpoint

RootRigidEnclosureCheckpoint and RootUniformScaleCheckpoint retain each binary64
endpoint as integer bits. RootReferenceCheckpoint combines the scale, body-to-world
and authored-to-body enclosures. Preparation returns the original interval bounds
without recentering, normalization, recomputation or additional widening. Invalid
versions, nonfinite/reversed intervals and rotation boxes containing no unit
quaternion are rejected. Scale zero remains a valid scale-enclosure value as in
from_scale; existing use sites retain their invertibility checks.

Rotation-box admission is exact: each nonnegative finite binary64 endpoint is an
integer multiple of 2^-1074; its squared magnitude is represented with BigUint on
the 2^-2148 lattice. The sum of per-component minimum squares and maximum squares
must bracket one. Squared norm is continuous on the closed connected box, so this
checks nonempty intersection with the unit sphere. Represented rotations remain
restricted to this intersection, as required by enclosure operations. This is an
interval-state validity check, not authentication of source/asset provenance; the
enclosing game must still verify the reference's identity. No hot-path enclosure
operation was changed; exact integer admission runs on checkpoint preparation.

Three new tests passed. Roundtrip and 180 subsequent body-reference transports
produce identical endpoint bits and transformed-point bounds; signed scale products
and exact identity intervals are retained. The adversarial point quaternion
(smallest subnormal,0,0,1), whose exact squared norm is 1+2^-2148 while the binary64
calculation rounds to one, is rejected. Reversed/NaN/zero quaternion boxes and
nonfinite scale are rejected. RootReference JSON roundtrip preserves all three
fields. The existing accepted-physics retarget test now roundtrips real captured
and transported references, including source-axis mapping and reflected parents.

Full validation: animation 344 passed; editor 235 passed, 20 ignored; strict
all-target Clippy for both crates and git diff check passed. Source hashes/logs:
`production-readiness-evidence-2026-10-26/root-reference-checkpoint-qualification.json`.

The earlier root-reference codec gap is now implemented at the participant level.
Still open: pending certified rotation paths, complete AnimationRuntime owner/asset/
retarget composition and shared archive integration, then App/PlaySession atomic
publication with all other active runtime owners. No whole-game, native image or
FPS qualification follows from these interval checks.

## Replay-verified root trajectory checkpoint

RootRigidPathCheckpoint stores a flat construction program and a lossless equality
record of every stored path field. Preparation reruns existing validated native
constructors against the caller's exact immutable curve revisions and returns a
path only if the whole record matches. The record includes duration/endpoint,
ordered rotation shapes and coefficients, left/right frames, cached angular bounds,
screw fields/initial poses, additive/pivot coefficients, and translation-cut times,
source enclosures and errors. It is never decoded as an executable certificate;
modified words fail equality against the trusted compiler's result.

Program nodes support source curves, ordered twists, explicit-time twists, retiming,
coordinate changes and spatial append. Backward-only indices avoid recursive input
graphs; node, per-path span, aggregate retained-span and encoded byte limits are
caller supplied. Each curve revision is resolved once. Curve key identity and file
read limits remain enclosing game contracts. No program tracking or extra state is
added to ordinary runtime paths. Screw span construction was factored into the same
native helper used by existing twist and outward integrators and checkpoint replay.

checkpoint_timed_screws captures canonical screw paths with exact stored start/end
times, without accumulating rounded durations. Noncanonical/nonscrew paths require
a matching construction program and fail rather than being flattened. A replay
program must actually be reproducible; arbitrary integration callbacks or their
original-field discretization/error proof obligations are not persisted by this
path-state participant. RootRigidApproximation's separate error provenance remains
an outer participant when used.

Two new tests passed: ordered twists with retime/transform/append and rejection of
corrupt words, invalid references and budgets; independently rebuilt cubic and STEP
curves across loop seams with exact retained coefficients/cut errors, matching
samples, velocities, projection and speed bounds, plus missing/changed resources
and altered cut-record rejection. The existing outward-integrator test now verifies
JSON replay of real refined screw spans, preserving every start/end clock and
matching samples/velocities. Full validation: animation 346 passed; editor 235
passed, 20 ignored; strict all-target Clippy and git diff check passed.
Evidence/hashes: `production-readiness-evidence-2026-10-26/root-path-checkpoint-qualification.json`.

Remaining: automatic recipe receipts at ModelPlayback trajectory creation,
AnimationRuntime pending rotation tuple/owner mapping, full owner/resource/retarget
composition and App transactional publication. This closes the core reproducible
path checkpoint API, not complete AnimationRuntime/game save or FPS qualification.

## Automatic single-clip root path receipts in ModelPlayback

Animator's new advance-with-receipt API retains the immutable source clip, motion
joint, selected axes and exact f64 start/end clock operands used by the existing
root path compiler. It does not derive times by multiplying normalized phases.
The legacy Animator entrypoint retains its previous path and allocation behavior.
ModelPlayback's rotating advance uses the receipt API and adopts the new receipt
only after its existing publication callback succeeds. Ordinary nonrotating ticks
clear it; certified-fade adoption clears the unrelated previous single-clip receipt
instead of retaining an incorrect source. One receipt is retained per owner.

RootRigidMotionCheckpoint stores the identified source plus replay-verified path
checkpoint. Capture can compare the owner's actual queued path against the receipt
program, rejecting every stored-field mismatch. Preparation verifies source/rig,
requires exact equality of the retained input program, then uses native replay and
full path-word equality before returning receipt/path. This prevents independently
edited receipt clocks from being accepted alongside a previous valid path record.
ModelPlayback checkpoint capture/restore now includes this participant, including
old immutable source revisions when retained. Asset/project identity and access to
those revisions remain the enclosing host's contract.

Two new editor tests passed. A real imported turning model captures its actual
root path, survives late consumer rejection unchanged, restores against a fresh
model, and matches complete playback/path checkpoint JSON through 120 following
f64 ticks. Missing source and independently altered clock metadata reject the
candidate. The receipt-focused filter passed five tests, including existing physical
fade/accepted-prefix regressions. Full animation: 346 passed. Editor: 237 passed,
20 ignored. Strict all-target animation/editor Clippy and git diff check passed.
Evidence/hashes: `production-readiness-evidence-2026-10-26/root-receipt-qualification.json`.

Still open: certified-fade integration receipts and their original-field error
provenance; AnimationRuntime pending rotation tuple/owner composition and archive
of immutable model/retarget revisions; full App/PlaySession save publication with
other active owners. These tests do not qualify native UI/image presentation or FPS.

## Original-source certified fade checkpoint and physical consumer

RootRigidFadeCheckpoint records the private pre-tick Animator checkpoint, exact
wall/axes, original preparation and integration capacities, common-frame/scale
enclosures and tolerance bits. It also retains lossless equality records for every
path field, approximation origin/angular errors and whole-domain velocity enclosures
with wall times. Preparation restores the source state and reruns the existing
original-source/common-similarity compiler; all result records must match exactly.
Serialized error data never becomes an executable certificate. Caller limits bound
encoded bytes, compiler span capacities and retained field count; resource revision
identity remains the host's contract.

An independent prepared unit contains initial animator, plan, certified motion and
validated compile context. prepare_from_animator instead links to an already staged
owner after exact initial-state comparison, preserving property-binding/frozen-pose
Arc identity required by matches_animator. RootRigidFadePlan now retains its original
preparation capacity so replay does not infer it from the integration capacity.
Mutable diagnostic paths/weights remain diagnostic; the original-source compiler's
private Animator snapshot remains authoritative, as before.

PreparedModelFadeMotion retains the small original-source compile context and can
checkpoint and prepare this participant. The existing physical fade tests now
serialize/read/rebuild the original-source prepared motion before generating the
physical request. The loaded path is therefore the one used for collision admission,
accepted-prefix event delivery and late-failure rollback. The legacy authored/path
bind_motion variant explicitly has no original-source replay context and rejects
this checkpoint operation; it is not silently treated as the original-source proof.

One new animation test exercises ordinary completion tails and interrupted frozen
sources against freshly built curves. It compares path/error/field bits and accepted
wall times/frames, verifies linking to the staged owner, and rejects missing sources,
different initial speed and corrupted path/error/field records. The three physical
fade tests passed with the restored certificate in the request flow. Full animation:
347 passed. Editor: 237 passed, 20 ignored. Strict all-target Clippy and git diff check
passed. Evidence/hashes: `production-readiness-evidence-2026-10-26/fade-checkpoint-qualification.json`.

Remaining: other/custom fade compiler variants and their proof recipes when exposed
as save participants; complete AnimationRuntime owner/resource/retarget/pending
motion composition and App/PlaySession atomic publication. The original-source
certificate gap is implemented for the production bind path above; whole-game saves,
native image presentation and FPS remain unqualified.

## Complete current AnimationRuntime checkpoint participant

The composite participant now records current animation owners, immutable model
revision keys, compiled retarget bindings, playback, accepted frames, foot history,
root references and pending translation/rotation queues. One shared pose archive
preserves frozen-source Arc identity across Animator, playback and foot state.
Retargeted foot snapshots are validated against the source rig. Restore constructs
a separate candidate against caller-resolved resource revisions and the staged scene.

ModelPlayback checkpoint schema is now v2: event tracks retain their actual immutable
clip revision after model rebind, instead of resolving only by current clip index.
Version 1 is rejected explicitly; released-format migration is not qualified.
Runtime ModelInstance serialization is opt-in through
register_game_checkpoint_components; the authoring registry is unchanged.

The combined test checkpoints scene/simulation, character physics, input and animation
with real foot IK and an interrupted fade; after fresh model/scene restoration, full
gameplay and animation records match for 120 varying-duration frames. Another test
covers mixed translation/rotation queues, retargeting and inactive/resumed owners.
These are CPU integration checks using separate participant records, not an atomic
App save or native visual/performance proof. Retained final logs report animation
348 passed, editor 241 passed / 20 ignored and strict all-target Clippy passed.
Evidence: production-readiness-evidence-2026-10-26/animation-owner-qualification.json.

This supersedes earlier statements that the current AnimationRuntime owner
composition itself is missing. Remaining: persist and verify resource revision
archives; compose App/PlaySession publication with UI/audio/camera/angular/liquid
owners; implement save slots and compatibility policy; validate native standalone
save/load. The full ENG-00 through ENG-13 objective remains open.

## Follow-camera presentation history checkpoint

FollowSmoother now captures both the filtered position and previous target as
exact f64 integer words, with a versioned serde record and independent validated
prepare. Unsupported versions and nonfinite values are rejected. A JSON roundtrip
regression compares every output and history bit for 720 subsequent varying-duration
frames, including zero elapsed time. Four focused follow-camera tests and strict
gameplay library Clippy passed; retained logs are follow-smoother-checkpoint-*.log.
This is the smoothing participant only: editor FollowRuntime owner/target identity,
view state and whole PlaySession publication still require composition. No visible
camera continuity or FPS qualification is claimed by this CPU test.

## FollowRuntime owner and presentation view participant

FollowRuntimeCheckpoint now composes durable camera/target ObjectIds, the retained
FollowCamera descriptor, exact FollowSmoother history and the current ViewportCamera.
Preparation resolves handles against the staged scene, validates target liveness,
owner typed components and descriptor compatibility, then returns an independent
runtime/view pair. It never publishes into a live App itself. Missing references
are explicit errors. The record uses the existing viewport serde representation.

The regression independently opens two fresh scenes, advances both to one simulation
boundary, JSON-restores only the camera participant into the second and compares
view plus complete participant records for 120 further varying-duration frames.
This verifies camera rebinding/continuation, not whole-session save/load. Full editor
validation: 242 passed, 20 ignored; strict library Clippy and diff check passed.
Evidence: follow-runtime-checkpoint-qualification.json and accompanying retained logs.
The earlier missing FollowRuntime/view participant gap is implemented; atomic
App/PlaySession publication and the other active owners remain open.

## UI pointer and keyboard router checkpoint foundations

voxy_ui now exposes codec-neutral PointerCheckpoint and FocusCheckpoint records.
They retain painter/traversal order, pointer position, capture/focus/pressed targets
and held activation, including cancellation while a physical key remains down.
Prepare builds independent bounded routers and rejects invalid geometry, duplicate
IDs, unknown/disabled captured targets and impossible pressed/held relationships.
No serde dependency was added to the platform-neutral router crate.

All 10 UI tests and strict all-target Clippy passed. The new continuation regression
checks repeat suppression, release click, traversal cancellation and held-after-
cancellation behavior against restored routers. Evidence: ui-router-checkpoint-
qualification.json and retained test/Clippy logs. Remaining: SceneUiRuntime durable
ObjectId binding and extracted snapshot, bounded wire format, session publication
and native device-state synchronization. These records alone do not close UI saves.

## Durable SceneUiRuntime checkpoint

SceneUiCheckpoint composes the accepted UI snapshot, painter order, ObjectId-to-widget
bindings, monotonic next widget ID, pointer position/capture/held state and keyboard
focus/pressed/held state. Geometry uses integer f32 words; descriptors use the existing
serde representation. Bounded JSON input/output and caller element capacity apply.
Prepare validates identities, scene-handle liveness, snapshot geometry, text budget
and router invariants before returning an independent runtime. It preserves accepted
snapshot state rather than extracting potentially newer scene descriptors on load.
The next normal refresh reconciles pending action changes and cancels old captures.

A regression JSON-restores held keyboard/pointer state into different scene handles,
checks old accepted actions, next-refresh cancellation and allocator equality, and
rejects missing owners, insufficient capacity/bytes and invalid held/identity data.
It also exposed an existing repeated pointer-down bug: SceneUiRuntime now retains
keyboard focus on repeat instead of clearing it when the router emits no new press.

Validation: 9 gameplay UI tests, 242 editor tests (20 ignored), 10 UI tests passed;
strict all-target gameplay/UI Clippy and diff check passed. Initial failed regression
log is retained alongside final evidence in scene-ui-checkpoint-qualification.json.
Remaining: App/PlaySession publication, action-code compatibility, native input
synchronization and worker/font/GPU resource composition. No native/FPS claim.

## Exact owner-thread PCM mixer checkpoint

voxy_audio now exposes codec-neutral MixerCheckpoint, VoiceCheckpoint and
GainCheckpoint. They retain voice slots including holes (floating accumulation
order), serial allocation, sample cursors, looping/paused state, bus membership and
every f64 gain-ramp bit plus remaining frames. Clip references are caller-supplied
immutable revision keys; conflicting keys for different PCM allocations are rejected.
Prepare resolves clips once per key, checks rate/cursor/identity/gain/limit invariants
and builds a fresh mixer owner. Saved serials resolve to new live handles; old
VoiceIds remain invalid. No serde or device dependency was added to voxy_audio.

The regression compares every PCM sample bit and complete state after 120 variable
blocks, covering paused resume, active voice/channel/bus ramps, looping, one-shot
completion and empty slots; invalid resources/cursors/gains/capacity reject.
Validation: 16 audio tests, 242 editor tests (20 ignored), strict all-target audio
Clippy and diff check passed. Evidence: mixer-checkpoint-qualification.json.

Remaining: host bounded wire codec and SceneAudioRuntime source ObjectId/publication
revision/completed-source state; immutable PCM archive composition; native callback
queues and device synchronization; App/PlaySession publication. This CPU mixer test
does not establish complete game-audio save/load or native audible continuity.

## SceneAudioRuntime durable owner checkpoint

SceneAudioCheckpoint now composes a bounded JSON mixer wire record, ObjectId source
owners, accepted AudioSource descriptors, publication revision numbers, voice
references and authored bus membership. Preparation resolves exact clip keys,
rebinds scene handles, validates output sample-rate compatibility and source/bus
limits, rejects duplicate/aliased/orphaned references and returns a separate owner.
Pending scene edits are reconciled by the next normal synchronize, not applied
implicitly during load. Clip resource identity remains the external resolver contract.

Mixer allocated-reference APIs preserve a completed one-shot tombstone that remains
in SourceVoice until the next synchronization; old process VoiceIds are never
reactivated. The regression saves at exactly that boundary, JSON-restores into fresh
scene handles with a clip decoded from the existing PCM archive, then checks full
owner records and every PCM bit over 120 variable blocks. Unchanged publication
synchronization never calls the clip resolver or restarts the completed source.

Validation: 11 gameplay audio tests, 16 audio tests, 242 editor tests (20 ignored),
strict all-target audio/gameplay Clippy and diff check passed. Evidence:
scene-audio-checkpoint-qualification.json and retained raw logs.
Remaining: complete immutable resource archive composition, AudioPlay wall/sample
clock and stream/worker state, native device/queued PCM synchronization, atomic
App/PlaySession publication. This proves the scene audio owner participant only.

## Audio sample clock and pending PCM pump participants

AudioClockCheckpoint records rate, modulo-60 numerator remainder, rendered-frame
counter and peak bits. Restore validates version, remainder, peak and output rate
before changing clock fields; it does not restore the mixer. AudioPlay now uses
one checked sample-budget calculation in native and offline paths. The offline
path previously hardcoded 800 samples per fixed step regardless of rate; it now
renders the exact derived count in bounded blocks and preserves the remainder.
At 48 kHz the existing 800-sample behavior is unchanged. A JSON clock regression
uses 44,101 Hz and compares 720 following variable-step advances, also checking
that invalid restore leaves all clock fields unchanged.

PcmPumpCheckpoint captures the already-rendered block as f32 words and submission
cursor. Independent bounded prepare validates normalized finite PCM and cursor.
A one-frame simulated queue forces backpressure: restored and original pumps emit
identical sample bits and retain identical renderer counts for 120 submissions,
without rerendering the saved unsent suffix. Already-submitted device PCM is a
separate owner and is deliberately not claimed to be restored by this test.

Validation: 243 editor tests (20 ignored), 5 device tests, strict all-target editor
and device Clippy and diff check passed. Evidence: audio-clock-pump-qualification.json.
Remaining: compose clock/mixer/pump with immutable publications and reload state
into AudioPlay; coordinate native queued output; complete App/PlaySession publication.

## Quiescent asset catalog checkpoint prerequisite

AssetCatalogCheckpoint now records the global revision allocator, original capacity
policy and each entry's current revision, published revision, status and external
immutable publication key. Prepare rebuilds a separate catalog with a fresh process
identity, validates entry/key/revision invariants and resolves each payload key once.
Preserving publication numbers is necessary for audio continuation: rebuilding by
ordinary request/complete would relabel resources and restart retained source voices.

The checkpoint boundary explicitly requires no pending imports; it returns
DependencyNotReady rather than dropping or pretending to serialize running worker
recipes/dependencies. Host save-barrier scheduling is still required. Failed and
dirty entries retain last-good resources and their actual published revisions.
The regression checks failed replacement, missing publication, dirty invalidation,
old-ticket rejection and identical subsequent revision allocation after restore.
All 50 assets tests and strict all-target assets Clippy passed; diff check passed.
Evidence: catalog-checkpoint-qualification.json.

This is a codec-neutral prerequisite, not a complete resource archive or AudioPlay
checkpoint. Remaining: bounded wire representation, immutable payload/input archives,
actual worker save barrier, AudioPlay and whole App/PlaySession composition.

## Accepted imported publication archive and composite audio playback

ImportedAsset now has bounded binary checkpoint encoding of its accepted typed
payload and original ImportInputs observations/capacity policy. The type-specific
codec remains caller supplied. Restore validates magic/version/length, the combined
BLAKE3 checksum, each source digest and aggregate input limits before decoding the
payload. It rebuilds captured reads without reopening source files. The checksum
detects corruption, not trusted authorship or equivalence between source and decoder.
Failed import-attempt diagnostics remain other owners; this archive contains accepted
publications. The archive regression verifies bit changes, truncation/limits and
cached source reads. A catalog/audio integration test restores archived PCM with
last-good publication revisions after failed reload and matches 120 PCM blocks.

AudioPlaybackCheckpoint composes catalog records, imported-publication archives,
actual runtime PCM revisions, SceneAudioCheckpoint and AudioClockCheckpoint in one
bounded JSON participant. Actual voice PCM is archived separately from the latest
catalog publication, preserving the boundary where reload published new data but
playback has not synchronized yet. Prepare validates aggregate clip/input limits
and returns independently staged catalog/runtime/clock state. It does not publish
into a live App or silently manufacture reload/device state. Native output capture
explicitly requires an as-yet-unimplemented device queue save barrier.

The composite test starts with old voice PCM, a newer catalog publication and a
subsequent failed reload, restores into fresh scene handles, compares initial PCM
bits and complete canonical checkpoint JSON, then compares complete records after
120 variable-step advances. Full assets: 51 passed; editor: 244 passed, 20 ignored;
strict all-target editor/assets Clippy and diff check passed. Evidence:
audio-playback-archive-qualification.json with retained logs and source hashes.

Remaining: AudioReload worker/queued/watcher state and real save-barrier scheduling,
native submitted device queue and pending-pump composition, whole App/PlaySession
publication, standalone/native user workflow. Composite playback is implemented;
the full AudioPlay owner and full engine objective remain incomplete.

## Reload dependency and polling baseline participants

SourceDependenciesCheckpoint retains the exact published and failed-attempt source
edges per output plus capacity policy. This cannot be reconstructed from accepted
publication archives alone: a failed reload can introduce a new missing dependency.
Independent prepare checks identities, duplicates and aggregate union-edge limits.
The regression preserves both invalidation routes and subsequent replacement cleanup.

SourcePollerCheckpoint retains sorted sources, distinct unknown/unavailable/content
fingerprints, round-robin cursor and polling policy. Restore never reads files or
pretends an unknown baseline was observed. A 120-call continuation test compares
change lists and complete state under varying check counts, content changes and
unavailability/recovery; malformed ordering/cursors and admission limits reject.

All 53 assets tests, strict all-target assets Clippy and diff check passed. Evidence:
reload-foundations-qualification.json. These are codec-neutral owner records only.
Remaining: asynchronous worker snapshot transfer, AudioReload queue/scheduling and
wire representation, real save barrier and App/PlaySession composition.

## Ordered background source-poll snapshot transfer

SourcePollWorker now accepts a bounded Checkpoint command on the same FIFO as scans.
request_checkpoint is nonblocking and may queue after an outstanding scan; a full
command channel returns Busy. New scans are fenced until try_checkpoint consumes
the snapshot. Prior scan results remain on their original independent result
channel and must still be applied by the host before capturing dependent owners.
new_with_checkpoint validates policy and starts a fresh worker at the saved baseline
and round-robin cursor. close drops both result channels so snapshot transfer cannot
strand shutdown behind a full receiver. No native window or audio device is opened.

The real-worker regression queues a snapshot while a scan result is outstanding,
checks fencing and retained scan delivery, then compares original/restored workers
across 16 file-change checks including recovery of a missing file. Final polling
records agree and both worker threads are joined. Assets: 54 passed; editor: 244
passed, 20 ignored; strict all-target assets Clippy and diff check passed. Evidence:
poll-worker-checkpoint-qualification.json and retained logs.

Remaining: AudioReload must orchestrate draining/applying scan/import completions
and retaining queued outputs/cadence before composing its checkpoint; this worker
API alone is not the full game save barrier. Native audio queue and App/PlaySession
publication also remain open.

## AudioReload nonblocking save barrier and staged reconstruction

AudioReload now has explicit begin/poll/resume checkpoint state. While fenced,
normal poll drains scan invalidations before import completions using the existing
publication path, but dispatches no new scan or import. Once both workers and the
catalog are quiescent, it requests the ordered watcher snapshot. The retained
record contains outputs, queued outputs, published/attempted source dependencies,
watch baseline/cursor and remaining scan delay. Repeated reads return the same
record; resume does not abandon an outstanding snapshot and preserves the captured
cadence rather than letting save duration consume it.

Prepare validates IDs, queue/output relationships, last-good publications and policy
bounds before invoking explicit project recipe/worker factories. It returns a
fresh reload owner. The caller must provide matching project recipes and a watcher
constructed from the supplied poll snapshot; there is no claim of portable thread
or OS handle serialization.

The real-file/worker regression captures during an outstanding scan, verifies
queue retention/no dispatch while held, restores fresh workers and checks equal
subsequent PCM/input observations and publication revision. It also begins a second
barrier while an import is in flight and verifies that completion is applied before
capture, leaving no pending catalog ticket. All threads are joined. Editor: 245
passed, 20 ignored; strict all-target editor Clippy and diff check passed. Evidence:
audio-reload-barrier-qualification.json.

Remaining: bounded reload wire codec and composition with playback archives; host
request/poll/publish workflow; native device queues; global App/PlaySession save/load.
The barrier is implemented at the reload owner, not yet invoked by a user save slot.

## Offline AudioPlay owner JSON composition

AudioOwnerCheckpoint now combines the playback/resource archive and optional reload
checkpoint under one bounded JSON record. Its request/poll/resume methods use the
existing nonblocking reload fence before capturing the catalog and playback state.
Prepare stages playback, validates the reload record, starts new workers through
explicit host factories, restores the clock and returns an independent AudioPlay.
Native output is explicitly rejected until a real device queue barrier is available.

AudioReload now owns the original import recipe map as well as output IDs. Both
initial construction paths pass those recipes through. The checkpoint serializes
them rather than rebuilding them from potentially newer scene authoring descriptors.
Editor-owned serde adapters preserve codec-neutral native IDs, published/attempted
source edges, distinct poll fingerprints/cursor and delay. Duplicate recipe keys
are rejected instead of silently overwriting. Semantic checks precede worker startup.

The integrated test captures during a scan that queues a reload, JSON-roundtrips the
whole owner, restores into fresh scene handles with fresh import/watch workers, then
executes the retained queue. Publication revisions match, followed by 120 advances
with identical complete playback/resource records. The existing reload barrier test
also roundtrips its wire record before worker reconstruction. Editor: 246 passed,
20 ignored; strict all-target editor Clippy and diff check passed. Evidence:
audio-owner-composite-qualification.json with logs/hashes and initial compile failure.

Remaining: AudioSession host request/load/publication and global PlaySession
transaction/save slots; native submitted queue and pending-pump composition; real
standalone/native audible acceptance. The offline owner codec/preparation is tested,
but no user save/load entry point is claimed by this milestone.

## AudioSession checkpoint request and staged publication

AudioSession now exposes internal request/poll/resume checkpoint methods and bounded
JSON session records, including the explicit no-playback case. The supported boundary
is offline with no pending start/preparation/device transition or running retired
worker. Native device sessions reject until the queue barrier exists. Checkpoint
close/cancellation state is tracked by the session.

load_checkpoint validates the target session and version, stages the complete audio
owner with the AuthoringProject's actual saved-recipe import worker and exact-baseline
watcher factories, then replaces live playback. Failure before publication retains
the old playback. Old worker owners are closed into the existing retired-join path;
completed handles are reaped before subsequent loads so repeated loads do not retain
an ever-growing handle list. The frame-critical path never joins active file IO.

The integration regression imports a real PCM16 WAV, saves a running session,
rejects a missing-owner load while verifying playback can still advance, and loads
into different scene handles over another running session. It joins retired workers,
compares 120 following frame/peak traces and complete final playback/resource records,
then closes/joins all remaining threads. Editor: 247 passed, 20 ignored; strict
all-target editor Clippy and diff check passed. Evidence:
audio-session-checkpoint-qualification.json and retained logs/source hashes.

Remaining: App's whole-scene/PlaySession transaction and user save-slot/file workflow;
native device queue/pump fencing and restoration; native audible acceptance. This
closes internal offline AudioSession staging/publication, not whole-game saves.

## Audio staging separated from admitted publication

AudioSessionCheckpoint.prepare now returns PreparedAudioSession without touching
the destination. Explicit discard closes staged worker channels and returns join
handles for outer-owner retirement; fallback Drop also closes channels without
waiting. This permits another participant to fail after audio preparation without
publishing audio prematurely.

AudioSession.checkpoint_publication validates the destination/save boundary and
completed retired workers, then returns an exclusive-borrow publication permit.
Its publish step consumes the prepared candidate and replaces playback without a
fallible return. The borrow prevents destination transitions between admission and
publication. Existing audio-only load_checkpoint delegates to this protocol; an App
coordinator can instead stage all participants before acquiring/adopting the permit.

The existing real-WAV session regression now additionally prepares/discards an audio
candidate after a simulated late outer-participant failure, verifies unchanged live
counters, and rejects publication during an active save. Staged workers are joined;
the 120-step successful continuation remains checked. Editor: 247 passed, 20 ignored;
strict all-target editor Clippy and diff check passed. Evidence:
audio-staged-publication-qualification.json.

Remaining: the App coordinator itself, the other live owner/resource gaps, native
audio device queue and user save slots. This is an admission/publication primitive,
not a claim that the whole game already commits atomically.

## AngularMotionBatch continuation policy

AngularMotionCheckpoint records version and admission capacity. Source inspection
confirmed that accumulated rotation belongs to SceneGraph transforms; motions,
staging and edit arrays are scratch overwritten by reconcile/fixed_step before use.
Prepare therefore binds fresh scratch storage to the staged SceneId rather than
serializing redundant descriptors or obsolete NodeIds. Descriptor admission remains
at its original fixed-step boundary, preserving pending invalid-edit error behavior.

The JSON continuation regression restores an already rotated pose into fresh scene
handles, checks matching failure/no pose change for a pending invalid descriptor,
then compares quaternion bits over 120 varying-duration steps with speed and
activity changes. Foreign scene/version/capacity rejection is checked. All 5
angular behavior tests and strict all-target gameplay Clippy passed; diff check
passed. Evidence: angular-checkpoint-qualification.json.

Remaining: compose this participant with PlaySession and implement remaining liquid
and resource owners, then global App save/load. No frame-rate or native claim.

## Native liquid pulse-source checkpoint

PulsedEmitterCheckpoint records every current emitter field: immutable pulse
schedule, elapsed time, aperture sample counter, full particle/optional thermal/phase
template, direction/source velocity, density, particle volume and nozzle radius.
Floating values are integer words. Prepare validates immutable pulse/clock/version
and caller pulse-count policy; public mutable settings retain their existing
advance-time admission behavior, including pending invalid settings. No normalization
or sample-counter reset is introduced.

The source continuation fixture uses a nonzero circular nozzle and extreme direction
scale, restores the emitter mid-pulse and compares complete source words, particles
(position/velocity/mass bits), exchange ledgers and fluid state over 120 intervals
including the idle tail. Both fluid fixtures begin from a clone of the same accepted
fluid: the test does NOT serialize or prove loading Liquid itself. All 10 emission
tests and strict all-target physics Clippy passed; diff check passed. Evidence:
liquid-source-checkpoint-qualification.json.

Remaining: bounded outer wire format, the full Liquid owner (including constitutive,
thermal/species/phase, boundary and transported histories), SceneLiquidRuntime body
and source identity composition, global PlaySession loading. This milestone must
not be reported as completed fluid saves or production fluid qualification.

## Native particle base and Maxwell memory checkpoint blocks

LiquidParticleCheckpoint records every particle position/velocity/mass word and
material index, material coefficients, smoothing/particle radii, gravity and all
core work/capacity policies. prepare_base checks caller limits and reconstructs via
Liquid::new, preserving bits without bypassing its configuration/material/particle
admission. This base deliberately does not claim to contain the other active Liquid
extensions; it must remain staged until those blocks are restored.

PolymerCheckpoint records Maxwell laws, per-particle conformation tensors, particle
material ordering and accumulated relaxation heat. Atomic restore clones the target
candidate, validates laws with the existing Maxwell setters, restores memory through
set_conformations and checks energy before committing. Invalid tensors or layout/
heat/limits leave the destination unchanged. Other owner state remains untouched.

A stressed two-particle fluid is advanced, then rebuilt from particle/base and
polymer records rather than cloning the accepted fluid. Every particle and polymer
word and step report matches for 120 following steps, including relaxation heat.
Corrupt memory, policy and particle material-index cases reject. One focused
continuation regression and strict all-target physics Clippy passed; diff check
passed. The polymer-name filter selected that same test, not an additional suite.
Evidence: liquid-particle-polymer-qualification.json.

Remaining: complete other constitutive/thermal/phase/species/boundary histories,
full bounded wire codec and SceneLiquidRuntime source/rigid-body composition, then
PlaySession publication. This is not complete fluid-world serialization.

## Shear/yield and structural rheology checkpoint block

RheologyCheckpoint captures shear-thinning coefficients, yield stresses, thixotropy
models, per-particle structural fractions and material ordering as exact integer
words. Atomic restoration validates all counts/coefficients/fractions with the native
validators, stages all four linked arrays together and evaluates constitutive
properties before replacing the target. This avoids evaluating an inconsistent
intermediate combination of old structural memory and restored shear/yield laws.
The thixotropy coefficient validator is shared inside the liquid module rather than
duplicated; public configuration behavior is unchanged.

The continuation test evolves structural memory, rebuilds from the particle block,
restores rheology and Maxwell memory, then compares every particle/rheology/polymer
word and step report for 120 following steps. Out-of-range structure, nonfinite
yield stress and caller policy rejection leave the target unchanged. One new
continuation test and the existing adaptive-thixotropy substep regression passed;
strict all-target physics Clippy and diff check passed. Evidence:
liquid-rheology-qualification.json. No full physics suite is claimed.

Remaining: thermal/phase/species and other constitutive responses, boundaries and
rigid-body coupling, bounded complete liquid wire format and SceneLiquidRuntime/
PlaySession composition. This native block is not whole-fluid file serialization.

## Temperature and dissolved concentration checkpoint block

ThermalFieldCheckpoint captures per-particle temperature/concentration, per-material
specific heat/conductivity/diffusivity/mixing groups and particle material ordering.
An absent transport owner yields None. Restore checks version/layout/caller limits
then uses the existing atomic configure_transport admission, preserving every f64
word. Existing phase state is rejected by that API rather than silently erased;
phase/species/response histories remain separate required outer stages.

The regression evolves heat and solute exchange, rebuilds the particle base, restores
thermal fields and Maxwell memory, then compares particle/thermal/polymer words,
step reports and thermal-energy/dissolved-mass totals for 120 following steps.
Both the evolving thermal state and polymer relaxation heat are active. Invalid
temperature, concentration, heat capacity and policy data reject without modifying
the target. All 3 transport tests (one new continuation and two existing numerical
edge cases), strict all-target physics Clippy and diff check passed. Evidence:
liquid-thermal-checkpoint-qualification.json.

Remaining: phase/saturation/species and mechanical response laws, boundaries and
rigid-body state, complete bounded liquid archive and SceneLiquidRuntime/PlaySession
publication. No complete fluid-save or native performance claim is made here.

## Phase and prescribed-pressure saturation checkpoint block

PhaseCheckpoint records per-material transition coefficients/high-phase material,
per-particle fractions and material ordering. Optional SaturationCheckpoint retains
curves, prescribed pressures and the exact cached transition temperature per particle
(including None for fixed-temperature materials). Restore checks caller limits,
shared native model validators, latent-heat compatibility, pressure domains and
recomputed cache words. Accepted cache words remain intact; no configure_saturation
energy remap is run as a side effect of loading.

A cloned transport candidate receives the restored phase only after model/fraction/
field consistency checks; finite energy and constitutive evaluation precede commit.
The PhaseChange coefficient validator is shared with existing configure_phase_change
rather than duplicated. Thermal/species layout and coefficients must be staged
consistently by the outer complete-world transaction.

The new regression rebuilds particle/thermal/phase state for fixed transitions and
a mix of pressure-dependent/fixed materials, then compares every particle/thermal/
phase record and total energy over 120 steps per case with heating and pressure
changes. Corrupt fractions/cache words/policy reject atomically. Validation: one
new continuation test, 13 existing liquid_phase tests, 5 liquid_saturation tests,
strict all-target physics Clippy and diff check passed. Evidence:
liquid-phase-checkpoint-qualification.json.

Remaining: species/reactions and thermal mechanical response laws, boundaries and
coupled bodies, bounded full-world archive, SceneLiquidRuntime/PlaySession loading.
These native blocks still do not constitute a complete liquid save file.

## Complete current native species owner checkpoint

SpeciesCheckpoint now covers all current SpeciesState fields: schema/fractions,
reaction indices/coefficient/chemical energies, optional thermal kinetics/adaptive
accuracy, mixture component mechanics/viscosity closure and component heat capacities.
Material ordering and caller species/name-byte/particle/material/reaction-attempt
limits are checked. Native configuration admission validates fractions, chemistry
and mixture laws on a private candidate. Heat capacities are installed after native
coefficient checks without configure_species_heat_capacities energy remapping, since
the restored thermal fields already contain the saved capacity's temperature.
Finite chemical/thermal/species totals are checked before replacing the live block.

The continuation regression exercises constant-rate chemistry and thermal kinetics
with custom tolerances, mixture properties and heat capacities. It reconstructs from
particle/thermal/species records, preserves initial field bits and compares every
particle/thermal/species word, species totals and chemical energy over 120 following
steps per case. Invalid fractions/reaction indices and policy limits reject atomically.
Validation: one new continuation test; 25 existing integration tests across species,
reaction, mixture heat and mixture properties; strict all-target physics Clippy and
diff check passed. Evidence: liquid-species-checkpoint-qualification.json, including
retained initial compile failure and final passing logs.

Remaining: other thermal mechanical/gas responses, boundaries and coupled bodies,
full bounded liquid archive, SceneLiquidRuntime/PlaySession publication. Complete
current species-block coverage does not establish whole-liquid serialization.

## Coupled temperature/solute mechanical response checkpoint

FieldResponseCheckpoint records all PropertyResponse coefficients, optional solute
end-member properties and ArrheniusViscosity coefficients together, with exact
reference-material words. Restore checks layout/caller limits and shared native
coefficient validators, rejects incompatible exponential/Arrhenius or mixture/solute
closures, stages both arrays together and evaluates effective coefficients before
commit. This avoids failing an intermediate public-setter configuration while the
final saved response pair is valid. The native Arrhenius validator is now shared
with the existing configure method without changing its admission rules.

The regression covers exponential and activated viscosity with expansion/solute
response, seeds the destination with the opposite family, restores from particle/
thermal/response blocks, then compares moving particle/thermal state and exact
effective density/sound-speed/viscosity bits over 120 steps per case. Invalid
coefficients, conflicting laws and capacity reject without modifying the target.
One new continuation test, 14 existing liquid_properties integration tests, strict
all-target physics Clippy and diff check passed. Evidence:
liquid-field-response-qualification.json.

Remaining: gas EOS, boundaries and coupled rigid bodies, complete bounded liquid
wire archive, SceneLiquidRuntime/PlaySession composition. This block does not close
whole-fluid saves or native performance/visual acceptance.

## Gas EOS and energy/integration policy checkpoint

LiquidEvolutionCheckpoint captures gas EOS coefficients, reference materials, density
formulation, pressure-work/viscous-heating flags, all three viscous integrator tags
and the tensile-pressure floor. Restoration stages a candidate and uses native EOS,
energy-mode, integrator and floor admission, including gas cv/phase/geometry rules.
The no-gas block can restore default policies without requiring absent thermal fields.
No field-temperature remap or loss of f64 coefficient words is introduced.

The regression covers gas and heated-liquid cases under Sequential, Symmetric and
Midpoint viscosity integration (six cases). It rebuilds particle/thermal/evolution
records, then checks exact particle/thermal/policy words, step reports and transport
energy for 120 steps per case. Invalid EOS, enum and gas-incompatible energy flags
reject without changing the target. One new continuation test and 28 existing
integration tests across liquid_gas, liquid_pressure_work and liquid_dissipation
passed; strict all-target physics Clippy and diff check passed. Evidence:
liquid-evolution-checkpoint-qualification.json.

Remaining Liquid fields still require explicit treatment: surface/interface/wall
responses, sampled/reflecting boundaries and body coupling, particle population
metadata and scratch-state audit. Full bounded archive, SceneLiquidRuntime rigid
owner/source composition and PlaySession publication remain open. This is not a
complete fluid-world save or a production physics qualification.

## Surface/interface and container-wall response checkpoint

SurfaceInteractionCheckpoint records cohesion strengths, the symmetric canonical
positive inter-material penalty map, optional wall-adhesion coefficients and exact
reference material words. Restoration validates caller/layout/map bounds, stages
replacement through the existing native setters and clears obsolete interface
entries before committing. Container geometry remains an explicit caller input,
as in the existing Liquid.step API; it is not falsely attributed to this owner.

The continuation regression rebuilds particles near container walls, restores all
response arrays and verifies removal of a deliberately stale interface entry. Every
particle/response word, step report and interface-energy bit agrees for 120 following
steps with active wall attraction and material overlap response. Duplicate pairs,
invalid wall range/strength and caller map capacity reject atomically. One new
continuation test and nine existing adhesion/surface/interface integration tests
passed; strict all-target physics Clippy and diff check passed. Evidence:
liquid-surface-checkpoint-qualification.json.

Remaining: sampled/reflecting boundaries, coupled rigid bodies and particle population
metadata/scratch audit, complete bounded liquid archive and SceneLiquidRuntime/
PlaySession composition. Surface configuration coverage is not whole-world loading.

## Sampled/reflecting boundary checkpoint and derived grid audit

LiquidBoundaryCheckpoint captures sample positions/volumes, prescribed velocities,
static pressure extrapolation, reflecting bounds and no-slip selection, with the
exact smoothing-radius identity needed by the grid. Restore stages replacement
through native geometry/policy admission, clearing incompatible old boundary modes
first. configure_boundaries deterministically rebuilds grid cells and sample indices;
the continuation test explicitly compares the rebuilt grid with the original.

Capture/restore rejects an active temporary boundary-body transaction. Source
inspection confirms finish_boundary_body takes that coupling out before public
publication; actual persistent rigid-body state belongs to the outer owner.
Four regression cases cover moving sampled walls, static extrapolation, reflecting
free-slip and no-slip. Translated sample geometry is restored from its exact words;
120 following steps compare particle/boundary records, grids and force diagnostics.
Invalid geometry/velocity/version/capacity reject atomically. One new regression,
43 existing liquid_boundary tests and 16 liquid_reflecting tests passed; strict
all-target physics Clippy and diff check passed. Evidence:
liquid-boundary-checkpoint-qualification.json.

Further audit: suspension_heat_buffer and suspension_heat_correction are compensated
energy inventory, NOT discardable scratch (suspension heat accumulation and droplet
split/merge carry them). They still require a checkpoint, together with the explicit
droplet population. carrier_droplet_stage is a temporary stage flag reset before
public publication. These findings supersede any assumption that all those buffers
are temporary. Outer rigid-body state, full bounded liquid archive and SceneLiquidRuntime/
PlaySession composition remain open.

## Compensated heat inventory and droplet population checkpoint

PopulationHeatCheckpoint retains separate high/low deferred heat words and explicit
droplet eligibility, preserving None versus an all-false mask. Restore validates
layout/limits, finite signed inventory and required thermal ownership, then replaces
the block without depositing or renormalizing heat. In-flight carrier/body stages
are rejected. Signed rounding residues are retained instead of clamped away.

The regression stores a nonzero below-ULP correction, rebuilds particle/thermal/
population state and follows 120 deposit/motion steps for three population modes.
It explicitly forces deferred heat into representable temperature mid-cycle and
checks that the high buffer clears while exact subsequent records agree. Invalid
correction/mask/limits reject atomically. The initial tiny-mass fixture exceeded
SubstepBudget; final unit-mass fixtures preserve solver/work caps and execute the
full cycle. One new continuation test, 16 existing suspension/population integration
tests, strict all-target physics Clippy and diff check passed. Evidence:
liquid-population-heat-qualification.json, retaining the rejected fixture log.

A source-derived liquid-owner-field-audit.json now classifies every current Liquid
field into its native record block, derived grid or temporary transaction state.
This is an inventory check, not full save qualification. Remaining: compose all
blocks with correct cross-feature validation, implement the bounded exact wire
format, qualify combined continuation, then restore outer rigid-body/source owners
and SceneLiquidRuntime/PlaySession.

## Complete current native Liquid checkpoint composition

LiquidCheckpoint composes every persistent native record block and reconstructs an
independent complete Liquid. Capture uses an exhaustive Rust field pattern over all
33 current owner fields, forcing future additions to receive an explicit save
decision. Native physical queries reject an incompatible current state before
emitting a record. Derived boundary grids are reconstructed; temporary carrier/body
transactions remain excluded. Preparation requires exact whole-record equality after
reconstruction, not only approximate particle positions.

Liquid-only staging paths now share structural parsers/native coefficient validators
with standalone restore but defer coupled energy/property queries until all blocks
are installed. Existing public standalone restore retains its prior atomic checks.
This matters for a demonstrated valid case: material heat capacity 1e308 with species
capacity 1 and stored temperatures 10/20 has finite actual energy, while early base-
material evaluation overflows. The complete prepare succeeds without changing bits
or remapping temperature. No serde or binary dependency was added to physics.

Five complete-owner continuation scenarios exercise rich chemistry/rheology/Maxwell/
boundary combinations, the corresponding expansion case, phase/saturation, gas with
reflecting/no-slip boundaries, and nonthermal droplet populations. Complete records
and full native Liquid values match after 120 steps per case. Additional tests cover
the heat-reference counterexample and capture-time rejection of an incompatible
model. Initial rich-fixture failure exposed the existing Maxwell restriction against
changing reference density; it remains an explicit engine capability gap, not a
solved feature. Accepted model combinations are tested separately.

Validation: 35 liquid unit tests and 70 existing integration tests across thixotropy,
reaction, mixture properties/heat, properties, species, viscoelasticity and phase;
strict all-target physics Clippy and diff check passed. Evidence:
liquid-composite-qualification.json, raw rejected/passing logs and refreshed field
audit. These results cover native records; they do not establish universal physical
validity or a game save file.

Remaining: bounded exact wire codec, broader cross-feature qualification, outer
rigid-body/source owner records and SceneLiquidRuntime/PlaySession/App publication,
then native user save/load acceptance and the rest of ENG-00 through ENG-13.

## Bounded exact native Liquid wire format

Added `VOXYLQ01`, explicit little-endian integer-word encoding of every native
LiquidCheckpoint block, without new dependencies or floating-point conversion.
Record destructuring is exhaustive so field additions require a codec decision.
`to_bytes` limits output bytes; `from_bytes` independently limits input bytes,
cumulative requested heap payload and dynamic values, including nested vectors
and UTF-8 strings. Length checks precede vector reservation. Allocation failures
are reported. Caller must run `prepare` for physical and component-version admission.
Allocator overhead and subsequent physical preparation are outside the decoder
payload budget. This native record wire has no integrity checksum; some corruptions
can form another valid record, so outer archive integrity remains necessary.

Five full-owner scenarios now encode/decode before exact 120-step continuation.
The overflowing-base-heat-reference counterexample also crosses the wire. New tests
exercise actual temporary-file write/read, every truncated prefix, trailing bytes,
root version/magic, forged lengths, invalid option/bool tags, invalid UTF-8 and shared
budgets for nested empty vectors/strings. Final liquid suite: 37 passed. Strict
all-target physics Clippy and diff check passed. See liquid-wire-qualification.json
and its retained logs. No FPS, native UI or full game-save claim is established.

Next: exact outer rigid-body/source records and SceneLiquidRuntime composition,
then atomic PlaySession/App publication and native save/load acceptance. The full
ENG-00 through ENG-13 program remains open.

## Complete native SceneLiquidRuntime owner checkpoint

SceneLiquidCheckpoint captures the native Liquid plus every persistent source/body
field: emitters and previous positions, physical motion/spin, original principal
templates, published poses, collider transform paths, mass distributions/properties,
original rigid frames, point-load recipes and elapsed phase. Durable ObjectIds
replace scene handles in all bindings. Preparation builds an independent owner in
a fresh scene and validates accepted descriptors, ownership, poses and collider
paths before returning it. Missing/ambiguous identities and caller source/body/
pulse/binding limits reject. Exhaustive native-owner field patterns guard additions.

Source iteration is now explicitly ordered rather than re-sorted by fresh handles.
New construction retains its old NodeId ordering; restore retains captured order.
A fresh-scene regression deliberately reverses source handles, warms up 7 steps,
then follows 120 steps with moving/temporarily inactive nonzero-radius sources,
compound rigid geometry, original COM/principal frame, nonzero spin and changing
point forces. Complete native checkpoint records and fluid/dynamics agree throughout.

This test found an existing particle-exchange bug: deferred heat arrays did not
follow additions/removals. Both high/low words now follow survivors exactly, new
particles receive zero entries, and removed inventory appears as a compensated
separate exchange ledger. Film-event aggregate receipts carry that ledger too.
A dedicated sub-ULP test checks survivor/removal words and atomic rejection.
The rejected initial fixture also reached its 64-particle cap because each active
interval emits a partial sample; the final unchanged 120-step workload uses a
sufficient explicit 512-particle policy. Solver precision/work rules are unchanged.

Evidence: scene-liquid-owner-qualification.json and retained passing/rejected logs.
38 liquid unit tests, the new scene-owner continuation, existing source/point-load
regressions, exchange/suspension/population and film-event/lifecycle integrations
passed; strict all-target physics/gameplay Clippy and diff check passed.

This is a native complete-owner snapshot, not a full scene save file. Exact frame/
geometry values are currently cloned into native records. Next is its bounded file
codec/integrity and outer PlaySession/App atomic composition, then native acceptance.
The complete ENG-00..13 objective remains open.

## Exact bounded SceneLiquidCheckpoint file codec

VOXYSL01 now encodes the complete scene-liquid owner: durable bindings, native
Liquid blob, source descriptors/emitters/history, contact motion/spin, original
principal templates, published transforms, collider paths, mass recipes/properties,
original rigid frame, point-load recipes and elapsed phase. All floating-point
values cross the wire as integer bits; no JSON conversion or reconstruction from
current pose is used. The rigid frame explicitly serializes its 27 f64 words and
3 f32 scale words with exhaustive field classification. Native validators admit
frame finiteness/invertibility, unit principal quaternions and physical inertia.

The 48-byte envelope contains magic, exact payload length and BLAKE3. It rejects
corruption before decoding. This digest provides no authentication/provenance.
Outer decoding has byte/cumulative heap-payload/value limits; embedded Liquid
uses a separate caller-supplied bounded decoder budget. Aggregate exposure is the
sum of these explicit budgets, plus allocator overhead; preparation has its own
physical policies. UTF-8 strings, integer conversion, tags, nested lengths and
trailing bytes are checked. Exact record macros destructure all fields.

Preparation now also rejects inconsistent rigid participant presence, nonfinite
time/history, invalid template geometry, mass/energy/frame failures and incompatible
source material slots or emitter recipes. It does not publish into the live owner.

The full continuation regression now writes/reads an actual temporary file, checks
exact encoded-byte roundtrip, restores fresh/reversed handles, and runs 120 steps
with moving/inactive sources and time-dependent point forces. Its initial body is
rotated and nonuniformly scaled so retaining the original frame is exercised.
Adversarial tests reject every prefix and single-byte corruption, recomputed-digest
forged length/version, independent resource limits, invalid tags and nested budget
exhaustion. Signed zero is preserved. Invalid body presence/time/source slots reject.

Validation: 3 checkpoint/codec tests and 4 existing rigid-frame tests passed; strict
all-target gameplay Clippy and diff check passed. Evidence is retained in
scene-liquid-wire-qualification.json and its raw logs. These are file/CPU state
checks, not native UI, GPU performance or complete game-save qualification.

Next: atomic PlaySession/App staging/publication, whole-game file integration and
native save/load acceptance, followed by the remaining full-engine ENG-00..13 work.

## UI callback state and queued-event checkpoint contract

PlaySession composition audit found an uncovered owner: UiActionHandlers stored
arbitrary FnMut closures with private state and queued events. Restoring only
fresh setup would silently discard that state. Added explicit opt-in UiActionHandler
with read-only capture, versioned schema/opaque bytes and a registry of independent
restore factories. Existing register/dispatch behavior remains available; legacy
closures explicitly reject save without consuming queued work. User codecs own
payload precision/migrations and must capture every private field.

UiActionCheckpoint preserves handler names/states, FIFO accepted events through
ObjectId bindings and all three runtime capacities. Decode/capture/prepare apply
caller byte/event/handler/command policies. New-scene preparation resolves queued
owners and callback-private references, rejects aliased/stale IDs, invokes no
dispatch and requires exact re-capture of each factory result. Binding failures
cannot be silently ignored. Factories/capture must not mutate external live state;
arbitrary side effects remain outside rollback guarantees. Accepted events that
expire before dispatch remain queued and produce the same ordinary dispatch error.

GameHost now retains UI codec registries across project/editor/headless/build and
project-package paths. The builtin voxy.ui.hide callback uses the same state codec;
GameHost::new includes it, and applications extend that registry before sharing.
Legacy model packages reject custom UI codecs they cannot retain. Public headless
participant capture/restore stages callbacks before replacing this participant.
It does not restore scene/commands, pointer/focus, audio or the complete game.

The native regression warms 23 barriers, saves three accepted events, restores a
new scene with reversed handles and continues 120 barriers, comparing full callback
records/scene state. It covers expiration, intentional callback failures, exact
private numeric words, budgets, malformed/duplicate codecs, wrong factories and
legacy closure rejection. A real project/package regression delivers 23 pointer
clicks, leaves one accepted activation pending, captures JSON, builds a package,
deletes source files and restores into the package with host codecs. The next 120
input/barrier cycles match. Missing custom codecs preserve the current live owner.
Initial host rejection exposed the unsaved builtin closure and is retained as evidence.

Validation: 6 gameplay UI-action tests; full editor suite 248 passed, 20 ignored;
strict all-target gameplay/editor Clippy and diff check passed. See
ui-action-checkpoint-qualification.json and raw logs. No native window/GPU/FPS
claim follows from these CPU/headless checks. Complete PlaySession/App composition,
whole-game atomic file publication and native save/load acceptance remain open,
as does the full ENG-00..13 program.

## 27 October: complete PlaySession participant composition

PlaySessionCheckpoint now composes all 12 current PlaySession fields through an
exhaustive native-owner pattern. It stores the authoring Stop document, complete
SceneSimulation checkpoint, animations, angular policy, ticks, optional follow
state, optical integer words, complete scene-liquid bytes, optional character
state, exact input and UI callback/event state. Primary viewport camera is carried
as a coupled host output. The platform Instant is explicitly a frame-pacing epoch
rebased after load; actual accumulator/interpolation clocks remain in the existing
SceneSimulation record. Host capture is intended at a frame boundary.

PreparedPlaySession owns one fresh LoadedScene and all matching runtime owners.
Preparation validates compatibility, Stop-document schemas, original scene allocator
and pending commands, resource-backed animation state, participant policies, liquid
integrity, physics/input and finally UI codecs before returning the unit. No live
scene or PlaySession is borrowed mutably. A late error drops the candidate. Caller
resource keys/factories remain explicit contracts; arbitrary external side effects
of application codecs are outside the rollback guarantee.

SceneCheckpointView is a read-only adapter for hosts with separate graph/ID indexes,
so capture does not move/clone a graph or change its handles. LoadedScene document
and allocation capture now delegate to the same view implementation. SceneSimulation
adds checkpoint_view rather than duplicating its save format. A regression compares
owned/borrowed records including retired ID reservations and rejects missing, aliased,
duplicate/empty and foreign bindings.

The combined fixture warms 17 variable-time frames, retains pending input edges, a
scene command and UI event, roundtrips the complete JSON and restores a fresh scene.
It contains an active skeleton clip, character/floor, angular owner, nonzero-radius
liquid source, exact optics, stateful Rust behavior and UI callback. Complete JSON
records agree immediately and after each of 120 further variable-time frames with
a landing/jump cycle. A late missing UI codec preserves source JSON exactly; wrong
compatibility, optical NaNs, corrupt liquid bytes and byte limits reject. Active
follow, retarget/IK and finite liquid rigid bodies are not exercised by this combined
fixture; prior component proofs are not promoted to a full cross-product claim.

Validation: full scene unit suite 90 passed; full editor suite 249 passed, 20 ignored;
strict all-target scene/editor Clippy and diff check passed. Evidence lives in
production-readiness-evidence-2026-10-27/play-session-qualification.json and raw logs.

This is staged PlaySession composition, not an end-user whole-game save/load path.
App resource/audio/UI routing/viewport/authoring owners, atomic whole-App publication
and old-owner retirement, complete-game file IO/integrity, compatibility derivation
and native interactive acceptance remain open, together with the rest of ENG-00..13.

## 27 October: complete native GameUiInput and accepted layout checkpoint

GameUiInputCheckpoint now captures every current adapter field through exhaustive
owner destructuring: SceneUiRuntime, physical Enter/Space activation bits and the
separate admitted immutable layout. Physical key bits are intentionally independent
of logical router hold: Tab can cancel an activation while both keys remain down.
Loading does not synthesize releases or normalize that valid state away.

Added reusable SceneUiSnapshotCheckpoint for accepted CPU layouts. It reuses the
existing bounded geometry/binding parser with canonical noninteractive metadata,
retaining painter order, descriptor/text data, exact rectangle/clip words and
durable owner identities. It rejects hidden pointer/focus state in an immutable
layout. Preparation retains the saved accepted layout despite current pending
descriptor edits, rather than extracting a different layout at load time. Normal
subsequent admission reconciles changes and cancels stale interaction. No GPU
presentation acknowledgement or font-resource continuity is inferred from this
CPU record. UiLive remains a separate owner.

Two fresh-handle cases follow 120 input cycles each after saving concurrent Enter/
Space holds, with and without pointer capture. Complete adapter JSON agrees after
every cycle across releases, resize, action edits, disabled/re-enabled controls and
visible-layout re-admission. Additional cases cover Tab cancellation, empty admitted
layout, unknown versions/key bits, policies, malformed immutable state and failed
capture preservation. A shared immutable-layout regression preserves painter order,
Unicode text and old descriptors with reversed target handles; missing/aliased owners
and noncanonical IDs reject.

Validation: full editor suite 252 passed, 20 ignored (7 native UI input tests);
4 gameplay UI layout tests passed; strict all-target gameplay/editor Clippy and diff
check passed. Evidence: production-readiness-evidence-2026-10-27/
game-ui-input-qualification.json and raw logs.

A source-derived app-owner-field-audit.json inventories all 61 current App fields
with explicit staged/open/host-policy dispositions and source hash. This inventory
is not whole-App save qualification. Next major owners are UiLive accepted visuals/
fonts/workers and the accepted model/resource catalog with import/reload barriers,
then identity/order/viewport/authoring integration, whole-App atomic publication and
complete-game file/native acceptance. The complete ENG-00..13 objective remains open.

## 27 October: exact accepted text records and UiLive CPU generation retention

UiLive previously dropped UiTextPreparation after GPU upload, leaving only layout
metadata plus GPU draw handles. A save could not recover the actual accepted font/
glyph pixels independently of later file changes. It now retains the successful
ImportedAsset publication separately for uploaded and last-acknowledged UI generations.
The presented CPU resource advances on every frame acknowledgement even when layout
descriptors compare equal; font contents can change while that layout stays equal.
Failed import/upload paths retain previous CPU publications. Close drops both owners.
This retains parsed fonts/runs longer than before; quantitative peak CPU/backend
allocation overhead is not qualified by the current tests.

Added codec-neutral GlyphAtlasCheckpoint, TextRunCheckpoint and TextFontCheckpoint.
They preserve shelf packing/capacity, exact alpha bytes, placed glyph region/origin/
cluster words, total advance, original face-zero source bytes and raster limits.
Exhaustive field patterns classify native atlas/run/font/shaper/raster owners. Parsed
font tables reconstruct from original bytes; accepted text runs restore directly,
without re-shaping or re-rasterizing them. Admission checks versions, limits, checked
geometry arithmetic, finite coordinates and region/entry consistency.

Regressions restore an atlas then follow 120 identical insertions; restore a font and
120 Unicode edits across ligatures, combining marks, Hebrew, Arabic and whitespace;
compare exact accepted run records/pixels; and reject malformed/budget-excess state.
A real import test changes font source bytes without changing layout and verifies
old/new publication Arc lifetime through acknowledgement and close.

An explicitly requested ignored test ran on Apple M4 Max / Metal, rejecting a CPU
adapter. The production worker/watcher/GPU-upload path published three actual font
resource versions: corrupt input retained the previous texture and CPU publication;
repair produced different glyph pixels; restoring the original font reused the
original texture and exact run words. CPU generation acknowledgement was checked
independently of layout equality. The test exercises offscreen upload and wgpu
validation; it performs no rendered-pixel readback or native swapchain presentation
and establishes no FPS claim.

Validation: 6 text tests; full editor suite 258 passed, 20 ignored; the explicit GPU
regression passed; strict all-target text/editor Clippy and diff check passed. See
production-readiness-evidence-2026-10-27/accepted-ui-text-qualification.json and logs.
The source-derived ui-live-owner-field-audit.json inventories all 12 current UiLive
fields and their remaining archive/barrier policies; it is not save qualification.

Remaining: bounded composed UiTextPreparation/UiLive archive with original observations
and shared run/publication identities; worker/watcher drain/save/resume; deferred GPU
cache retry condition and native transfer fence; then App resource/file/publication
integration and full-engine ENG-00..13 acceptance.

## 27 October: bounded accepted UI text publication file archive

Added a UiTextPreparation payload codec inside the existing VOXYIMP1 checksummed
ImportedAsset envelope. It retains all original parsed-font source bytes/policies,
exact accepted atlas/glyph data, label origin/clip/style words, painter order and
durable owner identities. Run IDs intern actual Arc allocation identity: equal
font/content/size references stay shared, while distinct equal-content runs remain
distinct. Native record mirrors destructure all current fields. Count, glyph, unique
atlas-pixel, source-font, payload and outer archive budgets are caller controlled.

ImportedAsset::from_checkpoint_bytes_with_inputs extends the existing parser with
read-only already-validated original observations. Its old API delegates unchanged.
UiTextPreparation restores its observation map from that exact input index, retaining
shared observation byte Arcs rather than re-reading sources or duplicating a second
ledger. Every archived face must match an original observed byte source. Accepted
runs restore directly; font parsing reconstructs only the independent font owner.

The real-file regression imports four labels and two physically distinct equal-font
files, saves an archive, deletes the entire original source directory, rebinds fresh
reversed scene handles and compares exact re-encoded bytes. Shared runs stay shared;
equal-but-distinct runs stay distinct. 120 changing viewport/clip/color CPU draw builds
match vertex/UV/color words, indices and atlas records. Missing/aliased identities,
policies, malformed references/coordinates/clusters, unobserved fonts and unused
runs reject. An adversarial clip case exposed a missing sum-overflow check and was
fixed. The initial fixture violated the existing one-source-per-asset manifest rule;
it now uses two physical files with identical bytes without changing that rule.
Both rejected logs are retained.

The explicit GPU regression ran on Apple M4 Max/Metal, encoded the actual accepted
publication, deleted its font source, decoded it, built/uploaded UI meshes and reused
the original texture with matching index count. Font-failure/recovery checks still
pass. This is offscreen GPU upload/validation, not rendered-pixel readback or native
swapchain presentation; no FPS claim follows.

Validation: full assets suite 55 passed; full editor suite 265 passed, 20 ignored;
2 focused archive tests and the explicit GPU test passed; strict all-target assets/
editor Clippy and diff check passed. Evidence:
production-readiness-evidence-2026-10-27/ui-text-publication-qualification.json.
The payload is integer-word JSON inside the shared archive; complete allocator/font
parser peak overhead remains unmeasured.

Remaining: multi-publication UiLive bundle/Arc aliasing; worker/watcher/source/cadence
barrier; native retry/presentation transfer. Current decoding requires live scene
owners; last-good visuals referencing destroyed UI owners need an explicit identity
history/tombstone policy rather than silently dropping them. These limitations, App
resource/publication/file/native integration and full ENG-00..13 remain open.


## 27 October: historical UI owners without scene resurrection

Added explicit reserved identity lookup and historical restore paths for UI layout,
accepted text publications and input adapters. Retired owners restore as unique
nonlive handles in scene-zero namespace, with the fresh owning scene ID carried
in generation. Live allocation never issues scene zero. Ordinary scene access and
strict UI restore still reject dead owners. Historical lookup is opt-in and scoped
to the saved scene identity index; it cannot resolve unknown or foreign owners.

The scene regression checks capacities 4 and usize::MAX, exact checkpoint roundtrip,
unique retired identities, fresh slot reuse and failed access/mutation of historical
handles. An initial test restored the MAX source with capacity 4; the loader correctly
rejected it. Correcting the fixture to preserve capacity yields 91 scene tests passed.
The rejected log is retained; no production capacity check was relaxed.

Existing accepted editor run: 267 passed, 20 ignored; explicit offscreen GPU test: 1
passed on Apple M4 Max/Metal, including archived text whose owner was removed and
whose source font was deleted. Input tests cover releasing held keys/pointer without
activating the replacement object. These prior accepted runs preceded only the test
capacity correction. Strict scene/gameplay/editor all-target Clippy passed in that
run; diff check passes after correction. GPU evidence proves upload/validation, not
rendered pixels, native presentation or FPS.

Evidence: production-readiness-evidence-2026-10-27/ui-historical-owner-qualification.json.
Remaining: composed UiLive publications with Arc sharing, worker/watcher drain/save/
resume, GPU retry and presentation transfer, cross-scene history policy and complete
App atomic restoration. ENG-03–13 and native/full-engine qualification remain open.


## 27 October: composed uploaded and visible UI publication archive

Added a bounded UiPublicationsCheckpoint participant for all five persistent CPU
publication fields: attempted layout, uploaded layout/resource and previously
visible layout/resource. The exhaustive UiLive capture inventories the seven
operational fields separately; this is not a complete operational UiLive save.
PreparedUiPublications stages only private CPU owners and exact mesh reconstruction.
It performs no source reads, worker dispatch, live-owner installation or native ACK.

The VOXYUIP1 envelope contains versioned layout/reference metadata and at most two
existing independent VOXYIMP1 resource archives, with exact body length and BLAKE3
corruption detection. It bounds total archive bytes, layout bytes/elements and
aggregate input bytes, atlas pixels, glyphs and run allocations. Publication IDs
intern actual Arc allocations. Explicit run allocation IDs preserve sharing within
and between distinct publications; distinct equal data remains distinct. Reuse
requires exact run checkpoint agreement and the current per-run admission policy.
Shared run pools are committed only after the full typed decode succeeds.

Four native regressions cover a real file roundtrip after deleting the original
font directory; two generations sharing some but not all equal-content runs; fresh
scene handles and a retired owner without scene resurrection; 120 changing draw
cycles for both generations; exact re-encoded archive bytes; shared versus distinct
equal publications; malformed/unused/aliased references, late missing owners,
corrupt/truncated/trailing data, aggregate budgets and shared-run disagreement.
The initial timestamp-only temporary fixture collided between concurrent tests;
adding a fixture-local atomic serial fixes isolation without changing production
semantics. The rejected log is retained.

Fresh full editor suite: 271 passed, 20 ignored. The explicit GPU regression passed
on Apple M4 Max/Metal. It captures the actual production UiLive owner between upload
of a new font revision and frame acknowledgement of that revision, deletes the font
source, restores both revisions from the file and uploads both through upload_on.
Textures match the two retained GPU versions and the original live publication
pointers remain unchanged. This is GPU upload/validation, not rendered-pixel
readback, swapchain presentation or an FPS result. Strict all-target editor Clippy
passes. Evidence: production-readiness-evidence-2026-10-27/ui-publications-qualification.json.

Remaining: worker/watcher drain/capture/resume, dependency and scan cadence records,
GPU cache retry rebinding and atomic native presentation transfer; then complete App
resource/file/publication integration. Independent resource envelopes duplicate
shared run records in the file, while restored run allocations stay shared. Exact
parsed-font/allocator peak heap overhead is not measured. ENG-03–13 remain open.


## 27 October: UiLive import/watch barrier and composed portable save file

Added a nonblocking Draining/Requested/Ready protocol to the native UiLive owner.
Normal invalidation-before-publication handling drains an outstanding import and
scan; no new imports/scans are dispatched while held. Finished retired workers are
reaped before Ready, while unfinished workers remain owned. Import/upload errors
are surfaced through the normal path and leave the barrier available for retry or
cancel. Cancelling an ordered watcher request consumes its response before opening
dispatch, avoiding an orphan fence/Busy loop.

UiReloadCheckpoint preserves both published and failed-attempt dependency edges,
the exact ordered watcher fingerprint baseline/cursor, optional remaining scan delay
and original deferred GPU retry tuple. Instant is never serialized. Resume/restore
rebases the remaining delay. Restore explicitly binds deferred retry to the caller's
final staged cache state instead of comparing an old process epoch with a new cache.
Validation precedes construction of an idle watcher; restore takes no fresh file
baseline and does not perform source reads. The unchanged native asset checkpoint
adapters are now shared by audio and UI through asset_checkpoint_wire.rs.

A composed UiLiveCheckpoint is capturable only at Ready and the caller's App frame
boundary. VOXYUIL1 combines the existing UI publication archive and bounded reload
JSON under exact lengths and a BLAKE3 corruption check. PreparedUiLive owns private
CPU publications and an idle watcher, exposes explicit worker teardown, and performs
no native install or frame acknowledgement. The current 13-field ownership inventory
is recorded in ui-live-save-owner-field-audit.json; this supersedes the earlier
12-field inventory for this source version.

Six new CPU regressions cover scan drain, failed dependency persistence, exact JSON,
held dispatch, restored file changes against the original baseline, Requested-phase
cancellation, unfinished retired ownership, empty state, invalid records before
factory invocation, explicit deferred rebind, Ready-only composed capture, envelope
corruption/budgets and late operational failure preserving the live source.

Fresh full editor suite: 277 passed, 20 ignored. Strict all-target editor Clippy,
scoped rustfmt and diff check pass. The explicit Apple M4 Max/Metal regression
starts a save with an actual outstanding UiTextPlan import, drains it and the
watcher, writes the composed file, removes the font, restores/uploads its meshes
and retains the original live CPU/GPU pointers. A changed viewport cannot dispatch
new work while held. Resume observes the missing source, retains last-good content,
can save that failed state, and recovers after source repair. GPU validation passes.
This establishes upload/barrier behavior, not rendered pixels, swapchain acceptance
or FPS. Evidence: production-readiness-evidence-2026-10-27/ui-live-save-qualification.json.

Remaining: transactional GPU cache/resource staging, validated native presentation
transfer and complete App commit; background full-archive encoding and disk-slot
publication; cross-scene history policy and whole-session acceptance. File encoding
is bounded synchronous CPU work; the barrier itself does not wait for IO. Parsed
font/allocator peak heap overhead is unmeasured. Full ENG-03–13 remain open.


## 27 October: transactional UI GPU cache and validated native publication

UI and focus uploads now fork the existing weak ResidencyCache index. All meshes
and textures are admitted/uploaded privately; synchronous failure leaves the live
index and epoch unchanged. The native UiLive completion path commits this cache
only after the source publication record succeeds. Existing resource sharing and
texture content identities remain unchanged; no second catalog/cache owner is
introduced. Forking copies weak index entries, not image payloads or strong owners.

Added an exclusive UiPublicationPermit over UiLive and the actual Graphics UI
fields. It stages the saved uploaded publication against the current native
geometry inventory, including old UI/focus allocations for peak admission. Three
wgpu error scopes cover validation, internal and allocation errors. They are popped
synchronously before any await, so cancellation cannot leave a scope open. A
Pending candidate exposes async validation, and only the validated Ready type can
publish. Late CPU failure, GPU validation failure and explicit discard retain live
owners. The final swap is infallible, resets derived old focus/native-ready state,
rebinds deferred retry to the final cache and returns all old worker handles for
retirement outside the frame loop. Submitted GPU references remain wgpu-owned.

A host-only awaiting_first_presentation flag retains archived CPU-visible history
while withholding it from native input/focus until the new surface frame is ACKed.
Normal reload continues to expose the still-visible old publication. This flag is
a native eligibility condition, not portable proof of a past physical frame; the
current ownership inventory now includes 14 fields in ui-atomic-owner-field-audit.json.

The new offscreen GPU regression exposed an existing fractional clipping defect:
(origin + size) - origin could exceed size and produce crop endpoint >1 for a
fully visible glyph. The first GPU run and a CPU regression both reproduced
SpriteBatch::InvalidSprite at origin0.1/viewport128. UI clipping now intersects in
glyph-local [0,1] coordinates, preserving exact uncut atlas endpoints without
loosening Sprite admission or reducing image quality. Red/green logs are retained.

Fresh full editor suite: 280 passed, 21 ignored. Both explicit UI GPU tests passed
on Apple M4 Max/Metal; strict all-target editor Clippy, scoped fmt and diff check
pass. The new test saves the actual worker/watcher publication, removes its font,
then checks peak-budget failures, late CPU rejection, an actual scoped invalid-buffer
GPU error, pending cancellation, validated discard and successful publication.
Failed attempts retain the old cache state, CPU/resource pointers, focus and rendered
UI pixels. An active old import remains owned until commit and is then returned
for teardown. Source and restored UI draw pixels match exactly; the previous target
pixels differ. Exact 128x128 RGBA readbacks, digests and lossless PNGs are retained
in ui-atomic-readback/. The images were inspected and contain the expected rendered
label and clipping, not an empty target.

Evidence: production-readiness-evidence-2026-10-27/ui-atomic-qualification.json.
The Graphics adapter is implemented/compiled; GPU tests exercise its common target
with real offscreen GPU objects, not a native window. Remaining: complete App
coordinator for scene/gameplay/input/audio/model/UI ownership and asynchronous load
scheduling, native swapchain acceptance, full save-file publication and engine-wide
ENG-03–13 gates. No FPS or backend in-flight/heap peak-memory claim follows. OOM
and internal scopes are registered, but destructive OOM/device-loss injection was
not performed.


## 27 October: authoring history, detached gesture epochs and App order participant

App composition exposed a missing durable participant: SceneHistory retained only
in-process current/undo/redo versions and detached SceneEdit epoch Arcs. Replacing
it with a fresh history would discard undo/redo and could accidentally authorize
a stale gesture. Added SceneHistoryCheckpoint using the existing native history
owner/validation, not a second history implementation. Exhaustive native field
mirrors retain documents, metadata, auxiliary resource drafts, exact byte counters,
both stack orders and all policies. Restore validates every committed version and
recomputes/checks its original retained-byte charge. It neither evicts nor trims
snapshots to make the checkpoint fit.

The caller includes owned detached edits in the same checkpoint. Epoch zero is the
current Arc allocation; other canonical IDs preserve actual sharing between stale
edits, even when undo returns an equal document. Restore issues fresh allocations,
so old original transactions cannot attach to the restored owner. Drafts remain
allowed to be invalid. Integer float words preserve NaN payloads, signed zero and
infinity; committing the restored invalid draft still fails native validation.
Draft object count is a separate caller policy from live scene capacity. The core
JSON writer bounds emitted bytes incrementally; copied draft/allocator heap peak is
not qualified. The enclosing file container must provide its own integrity check.

Added the App authoring-history participant for ordered object IDs/handles, selected
index, next object ID and all ModelDrag fields plus its exact detached edit. It
rebinds IDs through the prepared scene's reserved identity index without sorting,
clamping selection, reviving removed nodes or normalizing float values. Capture
checks that caller identity bindings match the actual App graph/parallel arrays.
PreparedAuthoringHistory owns private state tagged with its target SceneId; the
enclosing App coordinator must install it with that matching staged scene. This
participant does not silently publish a partial App. EditorError now preserves the
native DocumentError as a typed transparent error.

Four core tests cover 120 mixed continuation operations, both history stacks and
metadata/auxiliary quotas, shared/stale epochs, exact invalid draft float words and
adversarial records. Three real App tests create an actual project, preserve a
nontrivial object order/selection, stage a fresh scene and active gesture, then run
120 actual editing/undo/redo/resource-draft steps with matching exact checkpoint
JSON. Stale commits still reject; cancelling a restored drag returns its original
transform without changing history. Late malformed gesture data/reference rejects
without consuming the original live edit.

Fresh full scene suite: 95 passed; editor: 283 passed, 21 ignored; strict scene/editor
all-target Clippy, scoped rustfmt and diff check pass. These are CPU/headless App
checks; no new GPU/window/audio-device tests or FPS qualification were performed.
Evidence: production-readiness-evidence-2026-10-27/authoring-history-qualification.json.

Remaining: complete App coordinator, model/resource archives and reload barriers,
native audio queue transfer, remaining authoring preview/source and App viewport/
retarget/panel state, runtime identity registration, cross-scene history scope and
whole-session publication/file/native acceptance. ENG-03–13 remain open.


## 27 October: exact accepted mesh, skin and decoded-image file checkpoints

Added native SceneMeshCheckpoint, SkinnedMeshCheckpoint and ImageCheckpoint as
resource-archive foundations for the remaining model/App participant. This is not a
complete EditorAsset/ModelAsset archive and does not replace the existing importer.
Exhaustive field mirrors capture all accepted geometry/source streams and classify
process-local validation/coherence/group/bound caches as derived. Scene mesh records
retain positions, UV/color/index order, material parameters and the exact presence
and words of authored normals/material coordinates. Skin records retain source
attributes and integer UNORM16 weights/joint indices; native reconstruction must
match accepted attributes instead of silently canonicalizing archived inactive IDs.
Decoded image dimensions/RGBA bytes restore without re-decoding PNG/JPEG sources.

VOXYGEO1 is typed little-endian binary with exact lengths and a BLAKE3 body checksum.
Counts/strides, byte/vertex/index/joint policies and image geometry are checked before
large decoded allocations. Record parsing is not native publication: prepare applies
the original SceneMesh/SkinnedMesh/ImageAsset admission. No cache validity flag, LOD
quality claim or GPU handle is trusted from the file. Output size is checked up front;
restore checks its encoded-size quota without allocating a second full encoded file.

Five tests cover exact roundtrips for absent/prepared/explicit streams, subnormal and
signed-zero words, integer weights, 120 pose/temporal results, dense source admission,
exact pixels/mips and malformed policies/data. Rechecksummed hostile count/stride
overflow, reserved flags and impossible image headers reject independently of checksum
corruption. An actual editor import consumer saves accepted Fox base/primitive/image
streams, removes the entire source directory and restores those files. 120 posed
geometry results and all image/mip bytes match. Its skeleton/animation owner remains
separately retained and is explicitly outside this geometry-only file qualification.

Fresh full render suite: 363 passed, 86 ignored; editor: 284 passed, 21 ignored. Strict
render/editor all-target Clippy, scoped formatting and diff check pass. No new explicit
GPU/window/readback/FPS qualification was run. See production-readiness-evidence-
2026-10-27/geometry-archive-qualification.json and retained logs. The final root-module
order correction is formatting-only. Native constructor/allocator peak overhead is
not a qualified physical memory budget.

Remaining: complete model resource archive including skeletons/clips/morph data and
actual shared ownership, original observations/catalog state and LOD witness policy;
model worker/watcher drain and atomic native CPU/GPU publication; then App-wide
composition with scene/gameplay/input/authoring/UI/audio and disk/native/platform
acceptance. Full ENG-03–13 remain open.


## 27 October: exact skeleton and animation asset reconstruction

AnimationAssetCheckpoint now captures native skeleton/clip resources separately from
playback state. Actual rig/name/outer-clip Arc identity tables preserve aliases and
distinct equal allocations, including distinct clip Arc wrappers sharing clone data.
All 15 AnimationClip fields are mirrored: original keys, interpolation, tangents,
constants, root curve and lazy motion/rotation/rigid cells with exact reservations,
initialized masks and cached errors. Native constructors rebuild the data; exact
f64 coefficient/error receipts and a full recapture must match. No normalization
of accepted bind quaternions, approximate compiler fallback or budget reset is used.
Capture requires the caller's asset-query barrier; observed partial reservations
or changing masks reject. This participant does not implement the global barrier.

Four focused tests cover aliases, exact 120-phase continuation, aggregate budget
exhaustion/cached failures/cold future cells, malformed records and quotas, empty
archives and accepted near-unit bind quaternion bits. The real Fox consumer now
saves geometry, images, skeleton and all clips, removes its source directory and
restores from files. Every clip matches exact local poses, skin matrix words and
posed geometry at 120 phases. Material metadata is still retained by the reference
model; this is not yet complete source-free ModelAsset restoration.

Fresh animation suite: 352 passed; editor: 284 passed, 21 ignored. After adding a
final local-pose assertion the focused consumer and strict all-target animation/
editor Clippy were rerun successfully. Scoped rustfmt passes. Initial rejected
Clippy pointer-cast and formatting logs are retained alongside accepted results.
No new GPU/window/audio-device or frame-rate qualification was performed.
Evidence: production-readiness-evidence-2026-10-27/animation-assets-qualification.json.

Still open: morph/material/node metadata and full model resource sharing, original
observations and LOD witness policy, model import/watch barrier, atomic CPU/GPU
publication and whole-App/native audio composition. ENG-03–13 remain open.


## 27 October: primitive metadata and local morph resource archives

Added ModelPrimitiveCheckpoint and MorphGeometryCheckpoint in the native model
crate, using bounded VOXYMRP1 binary envelopes with exact length/type and BLAKE3
body integrity. A primitive retains its native geometry record, exact color float
words, optional image index and every TextureSampling field (including independent
mip filtering and anisotropy). Image-index resolution and resource aliasing remain
the enclosing model owner's responsibility. No GPU validity is inferred from
serialized sampler metadata.

Morph records include the immutable source primitive and ordered optional position/
normal delta streams, retaining signed zero and subnormals. Restore verifies native
geometry then validates vertex counts before delta-vector allocation, aggregate
vector and byte policies and native MorphGeometry admission. Parsing an envelope
alone is not semantic admission. No weights are clamped or normalized and no frame
is accumulated onto an already deformed source. Physical peak heap is not qualified.

Four tests cover static/skinned sources, exact metadata, 120 phases with negative
weights, large cancelling terms and f64 results, a real archive file, actual glTF
morph import with original buffers dropped, tight policies and checksummed hostile
counts/tags/nonfinite values. The editor Fox consumer now saves/restores full
primitive records, and uses the restored color for source-free posed geometry.
Skeletons/clips/images are restored as in the preceding animation qualification.

This does not yet archive MorphImportData as a whole: default property schemas,
weight clips, source-node bindings, clip mirrors and sharing still require model
composition. LOD witnesses, original observations, worker/watch barriers and atomic
CPU/GPU/App publication remain open. ENG-03–13 are not closed. Current validation
results, initial rejected tests, source hashes and limitations are retained in
production-readiness-evidence-2026-10-27/morph-assets-qualification.json. No new
explicit GPU/window/audio-device/FPS checks were run.

Final validation for the morph/primitive checkpoint: all 49 model tests passed,
including the long animal topology test; editor 284 passed and 21 ignored; strict
model/editor all-target Clippy, scoped rustfmt and diff check passed. The initial
new importer test used the non-morph parse entrypoint and failed; switching the
test to the existing native MorphModelAsset importer corrected the test. Both
rejected and final complete-suite logs are retained; no importer contract was
weakened. The final source owner inventory is model-resource-owner-field-audit.json.


## 27 October: exact property curve/default resource ownership

PropertyAssetCheckpoint now captures immutable PropertyClip and PropertySet data
for the pending complete model/morph owner. Native field mirrors cover all four
clip fields, all four track fields, both key fields and set entries. Scalar/vector/
color values and derivatives are stored as exact f32 words; booleans retain their
type. Identity tables retain actual name, track-array, key-array and tangent-array
Arc sharing, including partial sharing between otherwise distinct clip track arrays
and distinct equal allocations. No clocks are introduced or reset by this archive.

Restore applies original PropertyTrack/PropertyClip/PropertySet constructors and
checks an exact canonical recapture before returning prepared resources. The legacy
PropertyTrack::new boolean Linear contract is retained rather than accidentally
rejected by the stricter explicit-mode constructor. Sorting is validated; unknown
versions/fields, bad references/modes, invalid source values and unused table records
reject. Byte, clip/set, aggregate track/key/tangent and string policies are bounded;
this is not a physical peak-heap qualification. JSON integrity belongs to the future
enclosing model archive, as for the skeletal resource participant.

Three regressions cover exact source words, every property value kind, all three
interpolation modes, shared/independent/partially shared allocations, empty arrays
and defaults, legacy boolean behavior and 120 repeated sample cycles, malformed
records and policies without changing original resources. The actual imported
glTF morph consumer now serializes/restores weight clips and default property sets,
drops original JSON/buffers, and evaluates restored geometry with weights sampled
from restored property resources at 120 phases. Node-binding metadata and enclosing
model ownership still come from the reference model and are not yet archived.

The remaining composition must join primitive/morph records, skeletal resources,
these property resources, source-node bindings/default arrays and the original clip
identity mapping, plus model joint-name/mesh fields and sharing. LOD witness/source
observations and worker/watch/native CPU/GPU/App publication remain open. ENG-03–13
remain open. Evidence and fresh validation: production-readiness-evidence-2026-10-27/
property-assets-qualification.json. No explicit GPU/window/audio-device/FPS checks
were run for this source-resource change.

Final property-resource validation: animation 355 passed, model 49 passed, editor
284 passed and 21 ignored. Focused archive tests (3) and actual morph consumer (1)
passed; strict animation/model/editor all-target Clippy, scoped rustfmt and diff
check passed. Source hashes and all logs, including the initial iterator-mutability
compile rejection, are retained in property-assets-qualification.json.


## 27 October: native skin mesh set with original buffer/cache sharing

The model composition audit identified an additional ownership prerequisite:
SceneMesh owns Vec streams, while SkinnedMesh clones share vertex/index buffers
and immutable InfluenceGroups. Independent primitive restoration loses those
relationships. SkinnedMeshSetCheckpoint now captures actual Arc identities in
separate source-stream/cache tables. Equal but independent allocations are retained
as independent; LOD-style shared vertices with distinct index buffers and distinct
vertices sharing indices are representable. Process addresses are never serialized.

The additive VOXYGEO1 kind 4 envelope retains exact source words, explicit lengths
and BLAKE3 body integrity. Aggregate unique vertex/index, table/mesh, byte and joint
policies are checked. Separate native reconstruction vertex/index budgets bound
repeated vertex/rig/cache/index combinations before native allocation. Exact repeated
instances reuse validated native meshes; index-only variants use native
with_shared_indices instead of revalidating/copying every vertex for each instance.
These are source/work policies, not a physical peak-heap qualification.

All six SkinnedMesh fields are captured/classified. Native constructors rebuild
coherent/max-position data and influence groups; their values and all five group
fields are checked against exact word-derived BLAKE3 receipts before shared caches
are installed. Serialized flags/digests are not independent admission authority.
Final recapture checks canonical tables and rejects unused/inconsistent records.
No LOD quality witness or pose proof is accepted from this source-resource file.

Four regressions cover a real dense-mesh file (131073 vertices per source, two
independent vertex buffers), shared versus independent groups and streams, 120
native admission phases, small exact posed geometry, empty archives, malformed
source/cache claims, rechecksummed hostile counts and tight policies. 128 repeated
instances fit a single source reconstruction budget; a distinct joint-count job
consumes its own budget. The actual Fox import consumer now writes a skin-set file,
removes sources, restores its skin streams into the prepared primitive records,
and compares all clips at 120 phases using the restored palettes and materials.

This closes the native skin-set prerequisite, not the complete ModelAsset owner.
Joint/morph source-node metadata, cross-participant name sharing, skeletal/property
clip joins, LOD witnesses, source observations and whole worker/watch/CPU/GPU/App
publication remain open. ENG-03–13 remain open. Evidence and logs are retained in
production-readiness-evidence-2026-10-27/skin-set-qualification.json. No new explicit
GPU/window/audio-device/FPS qualification was run.

Final native skin-set validation: render 367 passed and 86 ignored; editor 284
passed and 21 ignored. All four new native tests are included in the full render
result. Strict render/editor all-target Clippy, scoped rustfmt and diff check pass.
The initial test-only missing Vec3 import rejection is retained with final logs
and source hashes. The model owner remains explicitly incomplete.


## 27 October: complete native ModelAsset resource graph file

ModelAssetCheckpoint now composes the native geometry/morph, skeletal/property and
shared skin-set participants into VOXYMDL1. This is the complete CPU ModelAsset
owner, including all eight fields, all five MorphImportData fields, four binding
fields and actual sharing between multiple outer ModelAsset Arcs. GPU handles,
images, LOD, imported scene nodes, observations and catalog generations are owned
by EditorAsset/import/App and are explicitly outside this type and qualification.

The graph retains joint names, mesh-joint mapping, mesh instance count and skinned
policy; optional morph source rig, ordered source-clip identities, weight/default
curves, source-node names/indices/default weight arrays and morph source geometry.
Outer model aliases, shared MorphImportData, skeleton rigs, clip/cache groups,
property arrays, skin streams/groups and names are retained by actual native Arc
identity. Independent equal resources are not merged. Explicit name bindings join
skeletal and property participants with model metadata; incorrect external text,
merged/split aliases and binding counts reject. Existing unbound v1 component JSON
continues to omit the new optional binding field. No process address is serialized.

The file has exact body lengths and BLAKE3 integrity, bounded strict JSON metadata
and typed length-prefixed participant blocks. Root/model/morph IDs must be in
canonical first-use order with no unreachable owners; primitive/morph blocks have
unique canonical ownership, so a small manifest cannot repeatedly decode one large
block into many independent primitives. Table, metadata, file and participant limits
are applied. Native constructors and exact component receipts validate each staged
resource; skin source words must agree before shared streams are installed. Private
mesh-joint/name-count and morph source/property-clip cardinality invariants reject
rather than exposing invalid native indexing. Final recapture validates the entire
resource graph. This is pure preparation, not an App publication transaction.

Four model tests cover a real complete archive file with shared/independent owners,
cross-field names and skin buffers, stateless 120-phase continuation, source clip
identity lookup and preserved duplicate-clip Animator rejection, late failures,
malformed integrity/counts/references, reuse amplification, tight policies, legacy
skin, unskinned static transforms and empty graphs. Two component regressions cover
name binding joins and old unbound JSON shape. The actual Fox editor import consumer
saves the complete ModelAsset file, deletes sources, restores the model and uses
its own public pose/palette/primitive API for every clip at 120 phases. Images/base
EditorAsset mesh remain separately qualified participants.

The standalone CPU example model_checkpoint writes and reloads model-graph-file/
model.voxymdl, drops original GLB bytes before prepare and supplies no original
resource resolver. Its three model references and three clips pass 1080 exact
pose/weight/posed-mesh comparisons and canonical graph recapture after continuation.
The 443069-byte Fox model file and its digest/receipt are retained alongside logs.
It contains model texture indices/samplers; decoded image ownership is external.

Fresh final suites: animation 357 passed, model 53 passed, editor 284 passed and
21 ignored. Strict animation/model/editor all-target Clippy (including the example),
scoped rustfmt and diff checks pass. Initial rejected compile/test/Clippy attempts
are retained. The duplicate-clip test now compares original/restored native errors
instead of expecting the pre-existing binding contract to accept duplicate source
identities. No importer/Animator validation was weakened. No explicit GPU/window/
audio-device/FPS qualification was run. See model-graph-qualification.json and
model-graph-owner-field-audit.json under production-readiness-evidence-2026-10-27.

Remaining: complete EditorAsset graph (images/nodes/LOD witness and original import
observations), catalog/revision joins, model worker/watch barriers and atomic native
CPU/GPU/App publication, native audio-device transfer and whole-session disk/native/
platform acceptance. Physical peak memory and cross-platform compiled coefficient
compatibility are not qualified by these logical policies and CPU runs. ENG-03–13
remain open; the full engine goal is not complete.


## 27 October: retained and reverified native static LOD proofs

The EditorAsset resource audit found that CertifiedLodIndexSet discarded its
constructor witnesses. Exact static LOD restoration therefore could not reproduce
the accepted proof/error envelope from the native owner alone. It now retains
whole-triangle or subdivision witnesses after native certification, moving the
original vectors into owned storage rather than copying every witness. Geometry,
index levels and error computation use the same native certification functions.
This adds resident CPU proof storage; physical old-plus-new heap budgets are not
qualified by this change. GPU index-byte accounting remains its documented logical
index-buffer scope.

CertifiedLodCheckpoint captures exact position words, index ordering, retained
proof kind/directions/cells and exact f64 level errors/count/index-byte receipts.
Restoration invokes native whole/subdivision constructors again and requires exact
recapture. No error scalar is trusted as a quality proof. Native subdivision depth/
coverage checks, cumulative position/level/index/cell and encoded-byte policies
apply. JSON parsing alone is not native admission; file integrity belongs to the
future enclosing EditorAsset envelope. Base-only objects preserve their verifier
kind too. Ordinary import proposal formats and the existing decoder are unchanged.

Three new regressions cover whole/subdivided real-file restoration with signed zero
and subnormal positions, 120 policy continuation steps with hysteresis and both
levels exercised, exact native receipts, invalid/forged source/proof/error/count
records and policies, unknown fields, and empty variant arrays. The actual vmodel
import/dependency/last-good consumer now also writes its accepted LOD checkpoint,
stops workers, deletes OBJ/certificate/recipe sources and restores the checkpoint
with matching native selection results for 120 phases.

A standalone CPU example retains whole.json, subdivided.json and receipt.json in
certified-lod-files/ under this evidence directory. It reloads both files through
native certification and exercises both levels over 120 steps each. Its two error
receipts differ by three f64 ULPs, illustrating why converting witness kinds or
loosening the receipt comparison would lose the exact accepted envelope. Each
restored receipt equals its own original bit-for-bit. This is static geometric
quality only; material, UV, normal, deformation and GPU arithmetic guarantees are
not implied.

Fresh full render suite: 370 passed and 86 ignored; editor: 284 passed and 21 ignored.
Strict render/editor all-target Clippy including the new example, scoped rustfmt
and diff checks pass. Initial new-test select-argument and semicolon Clippy failures
are retained with accepted logs. The final semicolon cleanup is test-only. No new
explicit GPU/window/audio-device/FPS qualification was run. Evidence: production-
readiness-evidence-2026-10-27/certified-lod-qualification.json.

Remaining: skeletal LOD source checkpoint and its actual mesh sharing with model
resources, complete EditorAsset image/node/LOD/resource graph and import observations,
catalog/revision joins, worker/watch barriers and atomic CPU/GPU/App publication.
Whole-session native/platform acceptance and physical memory qualification remain
open. ENG-03–13 and the full engine goal remain open.


## 27 October: skeletal LOD source file and explicit model-mesh join

SkinnedLodCheckpoint now preserves the complete native single-source data: exact
skin attributes/indices/joint count and derived mesh/cache receipts through the
native skin-set participant, plus original whole/subdivision witnesses and their
exact rest verification receipt through CertifiedLodCheckpoint. Native current
constructors produce one witness kind per source; base-only sources have no stored
kind discriminator and use a canonical Whole certificate with no invented variants.
Source itself stores no pose/error cache; every future prepare still recomputes
posed f32 positions and certifies the candidate pose with the native verifier.

Standalone prepare builds a new owned source. prepare_with_mesh accepts the
enclosing owner's explicit Arc<SkinnedMesh> binding, compares every source/cache
word via a canonical singleton skin-set record and preserves that exact Arc on
success. source_mesh exposes immutable ownership for this join. Equal positions
with different skin influences reject. Static proof preparation reuses one native
variant decoder; source positions and base indices must match before native
skeletal construction. Exact recapture checks the accepted source. Input/output
bytes, geometry and witness counts/depth policies apply; source counts are admitted
before cloning witness arrays. Physical peak heap/old-plus-new residency and global
publication are outside this participant. The enclosing archive supplies JSON
integrity; component parsing alone is not native admission.

Three new tests cover whole/subdivided file restoration, standalone and explicitly
shared mesh identity, 120 deformed poses and nonuniform instance transforms, exact
posed geometry/positions/f64 errors, changing pose envelopes, near/far selection
with hysteresis and both levels exercised, malformed source/proof/policy records,
invalid future poses retaining an earlier prepared value and base-only sources.
The actual skeletal vmodel import/last-good consumer now saves both complete model
and skeletal LOD files, stops workers, removes all four original recipe/model/
buffer/certificate files and restores the LOD against the restored model's mesh.
Their vertex/index buffers remain shared and all 120 pose/level/geometry outputs
match the original owners. No original source resolver is supplied to either file
restore. The explicit mesh binding follows the existing one-primitive LOD importer
contract, not a heuristic content search.

The standalone skinned_lod_checkpoint example retains whole/subdivided source JSON
files and a digest receipt in skinned-lod-files/. Each file is independently
restored and checked over 120 poses with 240 exact posed-geometry comparisons,
changing error envelopes and an explicit mesh-Arc join. Full render: 373 passed,
86 ignored; editor: 284 passed, 21 ignored. Strict render/editor all-target Clippy
including the new example, scoped rustfmt and diff checks pass. No new explicit
GPU/window/audio-device/FPS qualification was run. See production-readiness-
evidence-2026-10-27/skinned-lod-qualification.json.

Remaining: composition of all EditorAsset fields, images/nodes/source provenance
and resource identity tables, catalog/revision joins, import/watch barriers and
atomic native CPU/GPU/App publication. Multiple wrapper/source alias tables belong
to that enclosing graph; this codec prepares a single native source at a time.
Whole-session disk/native/platform and physical-memory acceptance remain open.
ENG-03–13 and the full engine goal remain open.


## 27 October: imported node hierarchy and decoded image participant

ImportedNodesCheckpoint captures all eight ImportedNode fields and the ordered
ImageAsset vector: names, original parent indices/order, exact local TRS words,
optional complete SceneMesh streams, image references, all six TextureSampling
fields, use_mips and original geometry_key tuples. Near-unit accepted quaternion
words, signed zero and subnormals survive without renormalization. Images retain
width/height/RGBA via the native decoded-image checkpoint and regenerate identical
mips without PNG/JPEG sources. Static node/image buffers are owned Vec values,
not hidden shared Arc allocations.

Preparation validates topological parent and image domains, native local transform
admission and sampler tags, bounded strict JSON metadata and byte/node/image/name/
aggregate geometry/pixel policies. Parsed source counts are admitted before native
resource construction; this is not proof of parser/native peak heap bounds. The
actual GPU uploader caches shared geometry by geometry_key, so mesh-bearing nodes
require a key and repeated keys must bind byte-identical complete mesh data. The
checkpoint rejects ambiguous keys before publishing any resources. GPU sampler
capability admission is separate; desired anisotropy is preserved, not clamped.

Two new regressions cover exact file/node/image/mip data, 120 hierarchical instance
transforms, parent/image/transform/sampler/native payload corruption, key collisions
and aggregate limits, preserving original live values. The existing actual
multi-material glTF test now writes/loads the participant for all six glTF min
filters after removing its GLB. It checks original geometry grouping, exact
recapture and 120 hierarchy steps for each filter (720 comparisons). Six accepted
node/image JSON files are retained in node-image-files/ with source hashes/logs in
node-image-qualification.json. A test-only explicit evidence-directory variable
uses create_new output and does not overwrite earlier qualification files.

Fresh full editor suite: 286 passed, 21 ignored; strict editor all-target Clippy,
scoped rustfmt and diff checks pass. After adding the test-only evidence emitter,
the real consumer and strict all-target Clippy were rerun successfully. Initial
closure error-type inference compile failure is retained. No new explicit GPU/
window/audio-device/FPS checks were run.

This is an internal participant for the pending EditorAsset coordinator; its
module explicitly permits dead code while production composition is unfinished,
matching the existing authoring checkpoint foundation convention. It is exercised
by native importer/CPU consumers, not yet wired to a complete App save command.
Remaining: combine all six EditorAsset fields and resource aliases (including LOD
source/model skin joins), original import observations, catalog/revision state,
model worker/watch barriers and atomic CPU/GPU/App publication. Physical memory
and whole-session disk/native/platform acceptance remain open. ENG-03–13 and the
full engine goal remain open.


## 27 October: explicit enclosing skin collection for model/LOD graph composition

ModelAssetCheckpoint now exposes source_skin_meshes in its canonical unique-model/
morph traversal order and prepare_with_skin_meshes for the enclosing EditorAsset
resource graph. The outer owner can capture one native SkinnedMeshSet across model
and LOD sources, then supply the model's original prefix to restore its main/morph
geometry. Both source words and the complete local vertex/index/group alias table
must match the embedded model skin record. Equal data does not authorize merging
independent buffers or splitting originally shared ones. Standalone prepare keeps
its former native reconstruction path and wire format is unchanged. Original model
resources remain untouched on external-binding failure.

The actual skeletal vmodel consumer now captures the model skin prefix plus its
LOD source in a shared skin-set file, removes original recipe/model/buffer/proof
sources, restores the shared set once and binds both restored model and LOD to it.
Vertex/index addresses agree across those owners, with exact 120-phase position,
error/count and posed-geometry continuation. An initial integration test failed
because its model consumer still called standalone prepare; it was corrected to
use the explicit shared prefix. The pointer assertion remains and passes, rather
than weakening identity validation. The rejected log is retained.

One new model regression covers shared model/morph buffers and rejects split and
merged equal source allocations, wrong collection size and unchanged original
recapture. A readonly shares_source_with query on SkinnedLodMesh distinguishes
actual Source Arc sharing from equal independent sources even when they use the
same mesh Arc; one native regression proves that distinction for outer resource
identity tables. No source address is serialized.

Fresh suites: model 54 passed; render 374 passed and 86 ignored; editor 286 passed
and 21 ignored. Strict render/model/editor all-target Clippy, scoped rustfmt and
diff checks pass. These are CPU/resource qualification checks; no explicit GPU/
window/audio-device/FPS qualification was run. Evidence: production-readiness-
evidence-2026-10-27/model-skin-join-qualification.json.

Remaining: complete six-field EditorAsset graph envelope and wrapper/source alias
tables, original import observations/catalog revision joins, worker/watch barriers
and atomic CPU/GPU/App publication, then whole-session/native/platform and physical
old-plus-new memory acceptance. ENG-03–13 and the full engine goal remain open.


## 27 October: complete CPU EditorAsset graph file

EditorAssetCheckpoint now composes all six EditorAsset fields into VOXYEDA1:
base SceneMesh, optional static certified LOD, optional animated ModelAsset,
optional skeletal LOD, imported nodes and decoded images. Explicit tables preserve
outer asset/model/static-LOD/skeletal-wrapper identities, inner skeletal Source
sharing, mesh Arc sharing and global vertex/index/influence-cache relationships.
Equal independent resources remain independent. A single native skin-set spans
models/morph sources and LOD mesh handles; the model prefix and each LOD mesh Arc
are rebound explicitly. Both partial-sharing cases (vertices only or indices only)
are qualified, without guessing identity from equal content. Owned static Vec mesh/
image data remains owned per distinct asset.

The envelope has exact body length/BLAKE3 integrity, bounded strict JSON metadata
and length-prefixed typed participant blocks. Canonical first-use rooted resource
tables reject missing/unreachable references; blocks have unique ownership and no
unused aliases. File/metadata/count/node/image/pixel/static-geometry and nested
participant limits apply. Static LOD/base geometry identity and model image-index
domains are validated. Native constructors revalidate every component; final exact
recapture checks the complete graph. The output is detached CPU assets, not a
publication into the live catalog, workers, GPU draws or App. Physical parser/native
peak memory and old-plus-new residency are not qualified.

Four new native tests cover a real graph file with repeated assets, distinct asset
clones, shared/independent equal models, shared static LOD, distinct wrappers over
one skeletal Source, independent Sources sharing one mesh Arc, independent model/
mesh resources, owned images, partial buffer sharing, 120 pose/error/geometry
continuations, malformed references/integrity/native blocks/policies and empty
graphs. Original inputs remain unchanged on failure. The actual importer consumers
now restore complete files after removing sources: Fox uses the restored model's
pose/palette/primitive API for every clip at 120 phases; skeletal vmodel uses its
restored model/LOD shared buffers for 120 poses; static vmodel validates restored
proof/base identity; six glTF sampler modes compare their complete node/image
resources and 720 hierarchy continuation steps.

Ten accepted files are retained in editor-asset-files/: Fox, shared identity graph,
skeletal/static LOD and six static sampler cases. Test-only evidence output uses
create_new, preserving earlier files. Final evidence-producing full editor suite:
290 passed, 21 ignored. Strict editor all-target Clippy, scoped rustfmt and diff
checks pass. Initial unused imports were removed before strict qualification.
No explicit GPU/window/audio-device/FPS qualification was run. Source hashes,
artifact hashes and logs: production-readiness-evidence-2026-10-27/
editor-asset-qualification.json.

The internal graph codec explicitly permits dead code while production App/catalog
composition is pending. Original ImportInputs observations/dependencies, revision
IDs and watcher/worker state are outside EditorAsset itself and remain to be joined
with this file. Remaining: catalog/provenance adapters, ordered reload barriers,
atomic CPU/GPU/App installation, native audio queue transfer and whole-session
disk/native/platform/physical-memory acceptance. ENG-03–13 and the full engine
goal remain open.

## 27 October: original-input EditorAsset publication graph

Added internal `import/publication_checkpoint.rs`: VOXYEIP1 wraps a single
EditorAssetCheckpoint graph and one native VOXYIMP1 original-input envelope per
distinct ImportedAsset publication. Repeated outer Arc references remain shared;
distinct publications retain shared model/LOD/buffer resources through the global
resource graph. Original observations, digests and recorded input policies are
restored without a filesystem provider. No content deduplication is introduced.
Canonical first-use IDs, unused publications, resource slot mismatch, corruption
and current admission policies are checked before returning detached owners.

A real animated GLB import restores after its source directory is deleted,
retains original observations and publication/model aliases, and re-encodes to
identical bytes. Empty graphs, corruption, tighter limits and reordered IDs are
covered. Full editor suite: 292 passed, 21 ignored, zero failed; final focused
suite: two passed; final all-target editor Clippy with -D warnings passed.
Evidence: production-readiness-evidence-2026-10-27/editor-publication-qualification.json.

This remains an internal detached CPU codec, not an App save/load command.
Catalog generations/status/last-good binding, worker/watch barriers and atomic
CPU/GPU publication still need integration. Byte admission is not physical peak
memory qualification. Native workflow, displayed output, FPS and ENG-03–13
remain open.

## 27 October: quiescent original-model catalog file

Internal import/catalog_checkpoint.rs now composes native catalog revisions,
statuses, last-good publications, SourceDependencies and the complete original
EditorAsset publication graph in VOXYEIC1. Preparation uses fresh native catalog
identity and returns detached catalog/source owners. Published dependency IDs
must match original input observations; failed-attempt dependency edges remain
separate and intact. Pending imports reject capture. Recorded policies and
source limits are checked before resource preparation. Audio and models reuse
one CatalogWire adapter, moved without changing its schema.

A real animated GLB is imported, its source directory removed, replacement
failed, and the resulting archive written/read from disk. Restoration keeps the
original inputs and last-good revision, rejects old tickets, continues revision
allocation, permits successful replacement and retains published data when dirty.
Malformed source joins and tighter policies reject without live mutation.
Full editor suite: 294 passed, 21 ignored; final file suite: two passed; final
all-target editor Clippy -D warnings passed. An intermediate evidence run hit
a test temporary-directory collision; atomic sequence suffixes fixed it and
the failing log is retained alongside successful final checks.
Evidence: production-readiness-evidence-2026-10-27/model-catalog-qualification.json.

This is detached CPU restoration, not complete App save/load. Model worker and
watcher barriers, queued reloads/recipes, atomic CPU/GPU commit, physical memory,
displayed output and FPS are not qualified. ENG-03–13 remain open.

## 27 October: model worker/watch fence in native App tick

Added app/resource_checkpoint.rs with an internal nonblocking Draining /
Requested / Ready fence. Existing source scan invalidations run before accepted
import completions using ordinary App::tick. New periodic scans and queued
model import dispatch stop while fenced. After scan/import/catalog quiescence,
the owner requests and consumes an ordered native watcher checkpoint. Capture
combines the model catalog/source/publication archive with queued reloads,
available outputs, current logical resource, original Direct/Manifest recipe,
watcher fingerprints/cursor and remaining poll delay. The new App fields retain
the import recipe and current fence state. Cancellation consumes an outstanding
watcher snapshot before releasing the fence; resume rebases the paused delay.

Three tests use actual headless App and file-backed worker threads: active
import draining, active scan draining, frozen dispatch with queue retention and
resume, requested snapshot cancellation, and manifest import capture after OBJ
and manifest deletion. Restored detached catalog inputs retain both original
files. Full editor suite: 297 passed, 21 ignored, zero failed. All-target Clippy
-D warnings, scoped rustfmt and diff checks pass. The initial Clippy failure on
unchecked Instant subtraction in tests is retained; checked_sub fixed it.
Evidence: production-readiness-evidence-2026-10-27/model-barrier-qualification.json.

The fence and capture API is internal. Fresh-worker restoration, user-facing
save/load, whole App atomic scene/CPU/GPU commit and physical presentation
acknowledgement remain open. No peak-memory or displayed FPS qualification is
claimed; ENG-03–13 remain incomplete.

## 27 October: fresh model workers from original-resource file

ModelResourcesCheckpoint now provides bounded checksummed VOXYEMR1 file
encoding and detached preparation of the catalog/source owners, saved recipe,
logical output/current/queue identities, idle import worker and optional native
watcher. All metadata, catalog and watcher policies validate before spawning.
The ordinary App constructor and restoration share InputRecipe::spawn_worker;
the decoder is not duplicated. Preparation reads no source model or manifest.
Stopped importer/absent watcher remain absent, including capture after shutdown.
Poll delay stays a duration until future App publication, so staging time is not
charged against the paused source polling cadence.

A real manifest project is imported, original OBJ/manifest removed, then
resources.voxyemr written and read. Fresh workers retain the exact watcher
fingerprints/cursor. A missing-file retry keeps last-good data; recreating a
manifest selecting new.obj allows the restored decoder to publish that new
source. Original live App publication remains unchanged. Tests also reject
corruption, wire bounds, duplicate outputs, bad manifest paths, bad watcher
cursor and invalid delay, and preserve stopped producers.
Full editor suite: 298 passed, 21 ignored, zero failed. Final file suite: four
passed. Final all-target editor Clippy -D warnings and scoped formatting/diff
checks passed. File artifact and log hashes:
production-readiness-evidence-2026-10-27/model-resource-restore-qualification.json.

Prepared owners are not yet committed into an existing App. Whole-scene,
authoring/runtime/UI/audio/resource identity joins, atomic CPU/GPU publication,
physical presentation acknowledgement and user-facing full save/load remain
open. Physical memory and displayed FPS are not qualified; ENG-03–13 remain
incomplete.

## 27 October: native detached GPU model batch staging

Source review found normal model reload publishes the CPU catalog before GPU
upload. A failed upload retains old GPU geometry, so CPU/GPU versions can differ.
This remains open; the new batch is a prerequisite, not a claim that the normal
reload path is already atomic.

ModelGraphics::upload now delegates to upload_on(device, queue); native surface
and offscreen preparation use one implementation, including geometry, certified
LOD, node/image sampling and animated preview resources. gpu_model_batch.rs
stages every requested model and exact CPU publication Arc into a forked cache.
Peak geometry admission includes old live resources plus earlier candidates;
accepted accounting replaces only the supplied old model inventory. Validation,
Internal and OOM scopes pop synchronously; a batch becomes Ready only after
asynchronous device-error validation. Failure never publishes the candidate.

An explicit real Apple M4 Max/Metal test passed with three models (including an
animated GLB and two different textures): aggregate peak budget failure, late
owner rejection, injected device validation error, successful staging, exact
CPU Arc retention, unchanged original cache, geometry inventory and dropping a
pending candidate without stranding scopes. Full editor suite: 298 passed,
22 ignored; the new ignored GPU test was run explicitly and passed. All-target
Clippy -D warnings, Rust 2024 formatting and diff checks passed. A first build
used the wrong nested test path; the explicit path fixed it, and the compiler
log is retained. Evidence: model-gpu-batch-qualification.json in the evidence
directory.

No whole App owner swap, destination epoch binding, render/readback pixels,
physical memory, displayed frame or FPS is qualified. Atomic scene/runtime/
UI/audio/model restoration and the normal CPU-before-GPU reload ordering still
need integration. ENG-03–13 remain open.

## 27 October: normal App reload waits for GPU admission

The normal reload path no longer installs a required visible model's CPU result
before GPU upload. App::tick stages into the detached batch and retains a
Pending model publication while nonblocking device polling resolves error scopes.
The old CPU publication and GPU model remain live. New imports are held until
the candidate settles. Source invalidations run first; device/cache/allocation
inventory, imported hierarchy and locomotion are revalidated before publication.
The shared complete_transaction admits catalog/source state, then swaps GPU
model map/cache under the same exclusive borrow. All fallible checks precede
that swap. GPU rejection is recorded as a failed import with original observed
inputs, keeping last-good CPU/GPU data. Stale tickets cannot install GPU state.
Headless/non-visible resources continue through the CPU-only path.

The model save barrier now explicitly waits for Pending GPU work, even if a
ticket was invalidated. Shutdown cancels the future and settles its ticket; it
retains observed inputs for the failed attempt. App has one new Pending owner
field. Existing hierarchy/locomotion checks were moved into one method, reused
after asynchronous staging rather than duplicated. Device/cache changes during
staging reject the candidate; this does not attempt to merge stale GPU caches.

Full editor suite: 298 passed, 23 ignored. A new explicit real Apple M4 Max/Metal
test passed without a window: the same transaction helper used by App leaves
CPU unchanged during staging, switches CPU/GPU versions together, rejects stale
tickets without changing cache or accepted publication, retains last-good on
failure/deferred error, blocks a save fence on pending work and settles shutdown.
Final all-target editor Clippy -D warnings and scoped formatting/diff checks
passed. Initial Pending Debug build error and an incorrect test fence setup are
fixed; their logs are retained. Evidence:
production-readiness-evidence-2026-10-27/native-model-reload-qualification.json.

This fixes the normal reload ordering in code and qualifies the shared commit
helper with real GPU resources. It does not qualify a native displayed frame or
render/readback pixels. Whole saved-App restoration, scene/runtime/animation/
UI/audio/model identity joins, physical memory and FPS remain open. ENG-03–13
remain incomplete.

## 27 October: current and retained animation model roots share one archive

EditorAssetCheckpoint now supports explicit external ModelAsset roots in the
same native ModelAsset/skin graph as current assets and skeletal LOD resources.
The optional zero-root metadata is omitted, retaining existing plain archive
encoding. Legacy asset/publication prepare rejects external roots rather than
silently dropping them; explicit prepare_with_models returns all roots. The
publication and catalog layers carry the graph through unchanged. Native
AnimationRuntime enumerates actual model/animation-model Arc roots in sorted
owner order, including inactive owners. App resource capture includes them.

The graph test preserves current-asset/external aliases, repeated roots, distinct
equal allocations and partial shared model/LOD buffers. It also covers an
external-only graph, root-count corruption and native reference caps. In a real
App import test, an inactive owner retains model A after the catalog publishes
a distinct model B sharing its source data. The resource archive restores both
outer identities and their shared clip Arc. Native AnimationRuntimeCheckpoint
restores the frozen A pose exactly; after activation both original and restored
runtimes advance 120 steps against their current catalog B models, comparing
exact pose checkpoints. File artifacts and source-free detached preparation
are retained. Full editor suite: 300 passed, 23 ignored. Final expanded runtime
replay and all-target Clippy -D warnings pass. Initial Serde predicate and
exhaustive-match Clippy findings were fixed; the failure log is retained.
Evidence: production-readiness-evidence-2026-10-27/retained-models-qualification.json.

This joins native ModelAsset roots, not every playback source allocation.
Standalone clips/property allocations held by fades still need a global join;
historical GPU texture content and retired EditorAsset import observations are
not recoverable from a ModelAsset alone. The test provides explicit runtime
source resolvers. Whole saved-App scene/animation/UI/audio/resource composition
and publication, physical memory, displayed-frame and FPS qualification remain
open. ENG-03–13 are incomplete.

## 27 October: playback sources and combined native animation resource file

ModelAssetCheckpoint now accepts standalone skeletal clips and property source
values in the same name/rig/compiled-cache graph as model-owned data. Explicit
prepare_sources APIs return those roots, including external skin-buffer binding;
legacy model-only prepare rejects nonempty playback roots. Zero-root metadata
is omitted. EditorAsset/publication/catalog graphs carry the same roots onward.
The editor registry records clip Arc identity and property Arc identity
separately, preserving independent equal outer property owners and shared
property tracks. Source enumeration delegates to native ModelPlayback checkpoint
serialization, covering its retained fades, contacts, events and motion sources.
App capture validates the save fence before runtime queries.

VOXYMAC1 ModelAnimationCheckpoint composes the existing AnimationRuntimeCheckpoint
and ModelResourcesCheckpoint in one checksummed bounded file. Prepared resources
provide canonical model/source resolvers; there is no second animation runtime.
The runtime loader had rejected valid stable-ID restoration when new NodeIds
were allocated in a different order. It now checks unique target nodes rather
than old ordering. Bundle graph checks compare actual allocation sets, allowing
owner traversal to change while retaining all resource aliases. Collapsed
object resolution remains an error.

Tests: native model source graph file round-trip; four external clip roots and
property values over 120 phases with exact f64 skin matrix words; repeated and
independent clip/property owners; shared tracks; malformed counts/caps and a
source-only graph. The App integration authors an accepted changed clip revision,
starts the native reload fade, saves runtime plus sources into one file, reads
it back and prepares against a fresh two-owner scene allocated in reverse order.
Both owner poses, fade weight and clip phase match over 120 steps through fade
completion. Invalid source bindings, owner collapse and pre-fence capture reject.
No external original-clip resolver is supplied on load.

Full editor suite: 302 passed, 23 ignored. Model package: 55 unit tests plus four
integration tests passed, 12 integration tests ignored. Final focused resource
suite: six passed. Final all-target Clippy -D warnings for editor/model and
scoped formatting/diff checks pass. The owner-order regression, first fixture/API
errors and Clippy findings are fixed and their logs retained. Artifacts and hashes:
production-readiness-evidence-2026-10-27/playback-source-bundle-qualification.json.

This is a detached CPU model/animation participant. Its caller still supplies
the scene and stable ObjectId mapping. Whole saved-App scene/PlaySession/UI/audio/
authoring/resource publication, historical GPU texture/source provenance,
physical memory, complete morph/retarget/physics scenarios and displayed FPS
remain open. ENG-03–13 are incomplete.

## 27 October: GPU owners retain original CPU texture publications

ModelGraphics now retains its canonical catalog ImportedAsset Arc after successful
transactional publication or ordinary residency upload. Viewport requests pass
that source to AnimatedModels; accepted owners retain it beside GPU textures.
Failure retains the old source, while successful texture-only updates refresh
the source even on the unchanged-geometry path. Pixel data is not cloned per
frame. Shared current geometry/model identity cannot identify which historical
image array belongs to a material, so the whole immutable publication is retained.

Catalog checkpoint adds optional retained publication roots in the same global
publication/EditorAsset/model graph. Current and repeated historical outer Arc
aliases and original input bytes/policies are preserved. App resource capture
collects both static and animated GPU source roots and rejects missing provenance.
A retained-only graph and malformed prefix/tail split are tested.

Explicit Apple M4 Max/Metal tests passed: canonical CPU/GPU publication Arc
association, and historical material restoration. The latter imports GLB data
with observed raw image input, accepts a red material, publishes green data
sharing its model, then rejects a bad pose. The old GPU owner remains red. After
source directory deletion, the archive restores old image/input bytes and outer
aliases; uploading via a fresh residency cache reproduces the exact original
64x64 RGBA buffer. Green differs at 253 pixels. Successful material-only update
selects green and releases the last original historical publication. Raw
reference/current/restored buffers and hashes are retained.
Full editor suite: 303 passed, 24 ignored; two explicit real-GPU tests passed.
Final all-target Clippy -D warnings, scoped formatting and diff checks pass.
Initial raw-update call-site compile errors and clone_from Clippy finding were
fixed; their logs are retained. Evidence:
production-readiness-evidence-2026-10-27/retained-texture-qualification.json.

This preserves live GPU-owner source payloads. The archive's retained root array
does not yet encode stable owner-to-publication roles or complete posed/LOD
GPU-owner state. Readback supplies an accepted reference frame externally.
Whole saved-App publication, physical presented-frame acknowledgement, retained
CPU heap admission/peak memory and FPS remain open. ENG-03–13 are incomplete.

## 27 October: stable GPU source roles in the native model-animation bundle

GPU source publication roots now carry explicit roles: AssetId for static model
uploads and ObjectId for live animated owners. Capture uses one ordered root
collection for both the role table and the original publication archive. Detached
preparation resolves ObjectIds against the supplied target scene and tags the
prepared roles with its SceneId. Missing identities, collapsed owner resolution,
duplicate IDs, noncanonical indices, nonanimated owner publications and aggregate
role-limit violations reject. Historical model publications need not equal the
current catalog version. An archive with retained roots but absent role metadata
rejects rather than guessing roles from array position.

The native VOXYMAC1 bundle includes optional role metadata alongside its existing
resource and animation participants. The CPU composition regression uses an
explicit LiveSources fixture without a window/GPU surface: a current catalog
publication and a historical owner publication resolve into a fresh scene with
changed NodeIds while preserving their distinct images and shared allocations.
The separate real GPU test on Apple M4 Max / Metal loads the historical texture
through the restored owner binding; the 64x64 RGBA readback matches the original
byte for byte, while the replacement differs at 253 pixels. This is offscreen
texture/publication evidence, not displayed frame pacing.

Fresh validation: voxy_editor 305 passed, 24 ignored; all-target Clippy with
-D warnings passed; the explicit ignored GPU material test passed separately.
The native bundle, role packet, raw readbacks, hashes, logs and qualification
record are retained under production-readiness-evidence-2026-10-27/
(gpu-source-role-qualification.json, gpu-source-role-files and
 gpu-source-role-bundle-files).

Whole-App atomic restore, full accepted GPU pose/LOD state, retired epoch-only
GPU owners, native audio-device queue restoration, physical peak memory and
presented FPS remain open. ENG-03–13 remain incomplete.

## 27 October: original accepted GPU frame values in model-animation preparation

The accepted render frame can lag the newest simulation frame after a rejected
GPU update. AnimatedModels now captures each live owner's accepted native
AnimatorFrameCheckpoint, ModelAnimation settings and u64 ticks independently of
playback resampling. Capture validates original publication/model identity and
stable owner IDs; aggregate encoded frame bytes are bounded. The existing native
VOXYMAC1 resource/runtime envelope carries this optional participant. Preparation
requires matching target SceneId and publication-owner count, rejects missing or
collapsed identities, and prepares the original frame against the historical
publication's model skeleton. Prepared frames remain detached; no live App swap
or presentation acknowledgement is claimed.

The real GPU regression now writes the accepted-frame participant into its
on-disk qualification packet, restores it against a fresh scene's stable role,
and submits that restored frame through the existing native AnimatedModels path.
The restored 64x64 texture/geometry readback exactly matches the pre-failure
image, while the replacement changes 253 pixels (Apple M4 Max / Metal). This
replaces the earlier test's externally supplied original frame. A CPU regression
checks native frame JSON equality, full u64 ticks, original model allocation,
invalid settings/version, missing owners and byte limits. The existing combined
bundle regression also joins an explicit CPU accepted-frame fixture through the
native VOXYMAC1 envelope.

Validation: full voxy_editor suite 306 passed, 24 ignored; explicit real GPU test
passed; enhanced combined-bundle test passed; final all-target Clippy -D warnings
and scoped formatting passed. Source hashes, logs, native envelope, qualification
packet and raw images are under production-readiness-evidence-2026-10-27/
accepted-render-frame-qualification.json, accepted-render-frame-files and
accepted-render-bundle-files.

Shared accepted-frame Arc identity across owners, accepted LOD certificate/cache
and per-view history, retired epoch-only owners, full App transaction/native audio
and displayed performance remain open. ENG-03–13 are not complete.

## 27 October: accepted frame aliases and staged skeletal LOD reconstruction

AcceptedFrames version 2 interns frames by original Arc identity. Shared owners
restore to one frame Arc; independent equal frames remain separate. Canonical
first-use indices and no-unused-frame rules reject ambiguous graphs. Version 1
inline records remain readable. Preparation validates every owner's target rig,
including owners sharing a stored frame. The CPU regression covers shared versus
independent frame references, out-of-order indices and unused frame resources.

The accepted owner participant now retains skeletal LOD source roles, per-view
selection history, resident non-base level inventory and its prepared certificate.
World/joint/position words are stored as f32 bits; measured error levels retain
f64 bits. Preparation re-verifies the certificate against the historical source
and rejects any exact mismatch, invalid level, missing resident selection or
noncanonical history. Capture validates that the accepted LOD source belongs to
the retained original publication.

The normal camera selection path and restore use one LodUpload staging helper.
restore_accepted_lod prepares every retained level before replacing the accepted
level map, per-view history and certificate. Peak admission includes live old
geometry plus all staged levels. A rejected budget leaves the prior owner intact.
The enclosing App transaction and asynchronous device-error scopes remain caller
responsibilities; this method is not a whole-App commit.

The explicit offscreen test now decodes the original publication through the native
VOXYEIP1 archive into an independent resource graph, restores accepted LOD state,
and rebuilds both native GPU-skinned and CPU-fallback geometry. Two camera views
retain levels 1 and 0 respectively, including a nonuniform world certificate.
All four 64x64 original/restored RGBA pairs match exactly and contain 496 nonblack
pixels each. Missing residency and corrupted certificate positions reject.
The separate historical-texture GPU test also passes with version-2 frame refs.

Validation: editor suite 306 passed, 24 ignored; alias regression passed after the
final primitive-sort correction; both explicit GPU tests passed; final all-target
Clippy -D warnings and scoped formatting/diff checks passed. Initial Clippy rejected
stable primitive sorts; its log is retained alongside the correction's result.
Evidence: production-readiness-evidence-2026-10-27/accepted-lod-owner-qualification.json,
accepted-lod-native-source-files and accepted-lod-material-files.

Full App publication/presentation ACK, retired epoch-only owners, authoring weak
frame caches, complete static GPU LOD state, native audio device queues and physical
memory/FPS qualification remain open. ENG-03–13 remain incomplete.

## 27 October: combined detached GPU models, materials, accepted frames and LOD

The native UploadTarget now stages one combined restore candidate from prepared
stable publication roles and accepted frame records. It forks ResidencyCache once,
uploads model roles, retains each exact original publication, restores per-owner
accepted settings/frame/ticks, and rebuilds retained skeletal LOD levels/history.
Historical owner materials are uploaded through the native model_texture storage
and binding path without allocating unrelated fallback model geometry. Owners
sharing one publication reuse staged material bindings; native animation source
sharing remains owned by AnimatedModels.

Admission includes old live geometry plus every candidate allocation. The ready
cache inventory excludes replaced old geometry and includes all staged models and
animated geometry. Scope validation covers OutOfMemory, Internal and Validation.
All scopes pop synchronously on success, early rejection and cancellation; no live
model/cache swap occurs here. PendingModels exposes the candidate only after its
asynchronous checks succeed. Final App stale-candidate checks, commit, presentation
acknowledgement and input handoff remain open.

The existing real historical-texture test now uses this combined candidate after
native file restoration into a fresh scene. A constrained geometry budget rejects
without changing live cache state/image inventory. Successful detached preparation
also leaves the old cache unchanged and reproduces the accepted image byte for
byte. The skeletal LOD test independently decodes its original publication graph,
stages the combined owner/LOD candidate and reproduces both camera images; each
64x64 image contains 496 nonblack pixels. CPU fallback remains separately checked.
A new explicit real GPU test injects a late device validation error, a late CPU
rejection, cancellation and a foreign target scene; the live cache remains intact
and an enclosing validation scope is clean after a later successful preparation.

Fresh validation: editor suite 306 passed, 25 ignored; three explicit real GPU tests
passed on Apple M4 Max / Metal; all-target Clippy -D warnings and scoped format/diff
checks passed. Evidence: production-readiness-evidence-2026-10-27/
composed-model-gpu-qualification.json, composed-model-gpu-files,
composed-model-lod-files and composed-gpu-*.log.

The combined candidate covers the model/animation portion, not all Graphics/App
fields. Static LOD residency/history, retired epoch-only owners, authoring caches,
UI/audio/non-model Graphics composition, physical memory, displayed FPS and the
rest of ENG-03–13 remain open.

## 27 October: one CPU/runtime/GPU prepared participant and destination guards

PreparedModelAnimation now stages its existing CPU resource catalog, source graph,
original import/watch workers and AnimationRuntime alongside the combined GPU
candidate. PendingModelAnimation retains these CPU owners during asynchronous GPU
validation; ReadyModelAnimation exposes them with the validated GPU participant.
This is composition of the existing runtime, not a second saved simulation.

Ready GPU participants retain the original destination device/queue, cache identity
and state, live-image byte inventory and old/replaced geometry inventory. Callers
can revalidate these immediately before exclusive commit. ResidencyCache now has a
native identity distinct for every fork, so replacement by an equal-counter cache
is detected. The normal deferred-upload retry and pending model reload paths use
the same identity rule. A whole-App CPU scene/history/publication epoch check is
still required; SceneId alone does not detect edits within the same scene.

The combined native VOXYMAC1 regression now has an active animation owner and a
separate older accepted GPU frame. After native envelope decoding into a fresh
scene, CPU and GPU preparation retain the exact original publication/model joins.
The restored runtime continues 120 steps with matching phase bits, skin palettes
and properties; its accepted GPU frame remains unchanged until an actual new
render publication. An explicit GPU test rejects foreign scenes, another device,
equal-state replacement caches and changed geometry inventories. Existing late
validation/cancellation and normal native reload transaction checks also pass.

Fresh validation: voxy_editor 306 passed, 26 ignored; three explicit real GPU tests
passed on Apple M4 Max / Metal; all-target Clippy -D warnings and scoped format/diff
checks passed. The first compile exposed two manually constructed Pending test
fixtures missing the new cache identity; fixtures were updated to their real cache
identity and the rejected log is retained. Evidence and source hashes:
production-readiness-evidence-2026-10-27/cpu-gpu-model-transaction-qualification.json,
cpu-gpu-model-runtime-files and cpu-gpu-*.log.

Actual full-App commit, root CPU epoch protection, first presented-frame/input
handoff, static LOD/non-model Graphics, UI/audio/device integration and physical
memory/FPS qualification remain open. ENG-03–13 are not complete.

## 27 October: native scene/history/draft observations for App restore preparation

SceneGraph now exposes an opaque process-local SceneChangeToken covering writes,
including names and transforms. The existing structure_revision retains its
structural-only semantics so transform/name edits do not invalidate those caches.
Token revision exhaustion fails rather than wrapping; scene identity distinguishes
fresh graphs. Atomic batch validation failures preserve the live token. Allocation
checkpoint reconstruction invalidates the token before slot changes. Registered
component copying now bumps structural and full write observations; previously it
extended the component map without notifying structural consumers.

SceneHistory exposes its existing native commit/undo/redo epoch as an opaque
HistoryChangeToken. Separate SceneEdit tokens change on mutable draft access and
are unique across detached edits. These process-local observation tokens are not
persisted schema revisions and do not change historical document/epoch archives.
Shared-reference interior component mutation remains outside SceneGraph tracking;
publish those changes via component_mut as documented.

App-aware model preparation now carries a SceneAuthoringGuard alongside the
CPU/runtime/GPU pending participant. It compares scene/history/draft tokens,
stable-object and node order, selection, next object ID and exact drag scalar bits.
The ready participant can revalidate this source observation separately from the
GPU destination. The destination guard now also detects changes within one target
SceneId, including a transform edit followed by restoring the same transform.
The App guard regression covers history undo/redo returning to equal content,
mutable draft access, signed-zero drag changes, selection/identity changes and
same-structure transform reverts. The real combined CPU/GPU regression accepts
an unchanged source after 120 CPU animation steps, then rejects a source rename;
this guard deliberately covers scene/authoring, not animation clocks.

Validation: voxy_scene 97 unit and 32 integration tests passed; voxy_editor 307
passed, 26 ignored; two explicit GPU checks passed on Apple M4 Max / Metal; all-target
Clippy -D warnings for both crates and scoped formatting/diff checks passed. Evidence:
production-readiness-evidence-2026-10-27/scene-authoring-restore-guard-qualification.json,
scene-authoring-*.log and scene-authoring-guard-files.

Runtime/source/UI/audio/host epoch guards, full App publication and first presented
frame/input handoff remain open, together with static LOD/non-model Graphics,
native device audio and physical memory/FPS qualification. ENG-03–13 are incomplete.

## 27 October: animation, catalog, dependency-index and ordered-fence freshness

Native AnimationRuntime now exposes a process-local write observation separate
from saved animation serial/time. Adopted staged branches, mutable references,
event registration/draining, accepted pose/fade operations, synchronization and
clear invalidate it; reads preserve it. Clones initially share an observation,
then writes use unique checked process-wide tokens so equal serials on divergent
branches cannot hide a change. Fresh checkpoint preparation gets a new token;
the durable runtime checkpoint format and exact playback state are unchanged.

AssetCatalog and SourceDependencies likewise expose opaque mutation observations.
Catalog request/completion/invalidation/removal and source mapping publication/
removal invalidate them; native checkpoint restoration creates a distinct
observation even when every saved value and retained Arc is equal. Existing ticket,
persistent revision and original source-input semantics remain separate. Stale
completion and rejected source admission preserve observations when no state changes.

App-aware model preparation retains a ModelRuntimeGuard in addition to its scene/
authoring guard. It observes native animation/catalog/index tokens, import recipe,
current/output/queued IDs, worker presence and the exact ordered Ready fence epoch.
Resuming and re-establishing an equal-data fence therefore rejects the older
candidate. ReadyModelAnimation exposes explicit model-origin revalidation alongside
scene/authoring and GPU-destination revalidation; these do not yet form the complete
App commit.

Regressions cover independent runtime branches, event mutation without scene edits,
read-only stability, stale tickets, successful/failed completions, equal-data native
restoration, source-index replacement and Ready-fence replacement. The real native
CPU/GPU bundle test first accepts the unchanged origin, continues both animation
runtimes for 120 matching steps, then proves that model-origin validation rejects
while the scene/authoring observation can still match.

Validation: voxy_assets 57 passed; voxy_editor 309 passed, 26 ignored; explicit combined
GPU test passed on Apple M4 Max / Metal; all-target Clippy -D warnings for assets/editor
and scoped format/diff checks passed. Evidence and volatile-field dispositions:
production-readiness-evidence-2026-10-27/model-runtime-origin-qualification.json,
model-runtime-origin-files and model-origin-*.log.

Non-animation PlaySession time/input/physics observations and freeze, UI/audio/host,
non-model Graphics/static LOD, full-App publication and first-presentation handoff
remain open. No displayed FPS or physical-memory result is claimed. ENG-03–13 are
incomplete.

## 27 October: complete PlaySession and original model resources in one native file

VOXYPGR1 composes the existing complete PlaySessionCheckpoint with the existing
ModelResourcesCheckpoint, stable GPU source roles and optional accepted render
frames. Its one animation authority is play.animations; the resource participant
contains original model/clip/property publications, not another runtime checkpoint.
Capture and detached preparation reuse the shared animation/resource identity
validation used by VOXYMAC1. Native scene/simulation, physics, input, angular motion,
follow state, liquids/optics, callbacks and Stop document remain owned by the
original PlaySession participant. Model import/watch workers prepare idle from the
same original resource archive.

PlayResourceGuard observes scene/authoring and model epochs, the host pacing Instant
and a BLAKE3 fingerprint of the bounded complete native file. Revalidation recaptures
native state only at restore barriers, not every render frame. Thus changes to
non-animation state do not need a duplicate simulation or lossy approximations.
This is an observation, not a full root freeze or publication implementation.

A rich existing PlaySession fixture is temporarily moved into the App owner for
native capture. The file owns its real model resource graph. After the original
source directory is deleted, it prepares into an empty target directory without
using old model Arcs; restored catalog and animation roots share the fresh model.
Missing UI callback codecs reject. The complete native PlaySession JSON matches
before continuation and after each of 120 additional variable-time frames, including
behavior/UI callback state, physics, liquid, angular motion, input and animation.
Independent origin checks reject a zero-fixed-step clock advance with unchanged
scene token, host pacing epoch change, input event, physics policy change, queued
UI event and queued scene command.

The first test run used max_sources=256 below the native watcher policy of 257;
capacity admission correctly rejected it. The fixture now admits the actual policy;
the rejected log/archive are retained. Fresh validation: voxy_editor 310 passed,
26 ignored; all-target Clippy -D warnings and scoped format/diff checks passed.
The existing combined model GPU test also passes on Apple M4 Max / Metal after
sharing the resource validation helper. This does not qualify GPU state for the new
full Play file: the rich CPU regression explicitly has no GPU roles.

Evidence: production-readiness-evidence-2026-10-27/full-play-resources-qualification.json,
full-play-resource-clock-files and full-play-resources-*.log. Whole-App authoring,
extraction/host/UI/audio composition, full play freeze, first-presentation handoff,
physical memory/FPS and ENG-03–13 remain open.

## 27 October: full Play/resources participant stages its accepted model GPU state

PreparedPlayResources now stages the existing native GPU restore directly against
its restored LoadedScene, retaining the complete game and idle resource workers
through asynchronous GPU validation. ReadyPlayResources carries that one original
PlaySession with the validated GPU candidate and rechecks both the running source
PlayResourceGuard and destination cache/device/scene observations before an outer
exclusive commit. It does not create another AnimationRuntime or move partial state
into the live App.

Full Play capture uses a typed PlayGpuSources aggregate for original publications
and accepted renderer frames. The normal host path fills it from Graphics; the
windowless regression fills it from an actual native AnimatedModels renderer and
ModelGraphics upload on Apple M4 Max / Metal, using the same capture path. The
VOXYPGR1 file contains one runtime at play.animations, GPU source roles and an older
accepted bind frame with ticks=731. After source-directory removal, native decoding
reconstructs independent models shared across the catalog, simulation and render
owner. Accepted rendering remains distinct from the newer simulation frame.

The restored model-only 64x64 RGBA readback exactly matches the original and contains
946 nonblack pixels. Projection is derived from the model snapshot for this test;
this is not a complete scene/window presentation result. The native PlaySession
still matches the control over 120 additional variable-time frames. Budget rejection
and cancellation leave the source CPU file and cache state unchanged. Changing
source input rejects origin validation; exact input restoration revalidates it.
The enclosing GPU validation scope remains clean after all three staging outcomes.

Validation: voxy_editor 310 passed, 27 ignored; the explicit full Play GPU regression
passed; all-target Clippy -D warnings and scoped format/diff checks passed. Evidence:
production-readiness-evidence-2026-10-27/full-play-gpu-qualification.json,
full-play-gpu-verified-files and full-play-gpu-*.log.

This is a running-source prepared participant. Idle/startup origin admission and
an actual full-App freeze/publication, authoring/extraction/non-model Graphics,
UI/audio composition and first-presentation/input handoff remain open. Physical
peak memory and displayed FPS are unqualified. ENG-03–13 remain incomplete.

## 27 October: idle-source admission for full Play GPU preparation

observe_play_restore_origin now captures a source guard from either running Play
or the idle editor. Idle observation uses the existing scene capture, model runtime/
resource checkpoint, input codec and native optional physics/angular/follow/liquid/
UI checkpoint APIs. It does not start a dummy simulation or change the target game
file format. The exhaustive original PlaySession destructure documents all twelve
source-field decisions; optional preview participants are observed, not discarded.
A partial idle/Play transition rejects rather than assuming either state.

Idle drawing updates simulation_time as a host frame timer without advancing a
simulation. That timer-only refresh therefore does not invalidate idle admission;
running Play retains its existing pacing-epoch check. Native scene/model/fence
observations and the complete bounded idle fingerprint still detect actual source
input, preview physics/configuration, mode, camera and resource state changes.

The full native GPU regression now also returns to the parked idle editor source,
observes it without starting Play, and stages the same archived game into a fresh
scene. The earlier running-origin candidate remains allocated; its geometry is
included in peak admission while staging the idle-origin candidate. Both prepared
frames match the 64x64 reference byte for byte (946 nonblack pixels). Idle input
changes and partial mode changes reject; exact input restoration and timer-only
refresh revalidate. The original idle App remains without a simulation throughout
preparation. This is detached preparation with explicit offscreen GPU owners, not
an editor menu/load command or SceneSurface presentation result.

Fresh validation: voxy_editor 311 passed, 27 ignored; the expanded real GPU test
passed on Apple M4 Max / Metal; all-target Clippy -D warnings and scoped format/diff
checks passed. Initial compilation required mapping the existing input codec error
into EditorError; its rejected log is retained. Evidence and source-field decisions:
production-readiness-evidence-2026-10-27/idle-play-origin-qualification.json,
idle-play-render-refresh-files and idle-play-*.log.

Actual idle/startup App publication, a global game/UI/audio freeze, menu integration,
authoring/extraction/non-model Graphics composition and first-presentation/input
handoff remain open. Physical memory/FPS and ENG-03–13 are not qualified.


## 27 October: coordinated model/UI/game/Offline-audio restore barrier

App now coordinates the existing model and UI import/watch fences, allows their
normal drain to finish, then freezes game updates and requests the native Offline
audio-owner checkpoint. Frozen draw does not consume pending input edges, advance
animation/game ticks, mix Offline audio or reconcile away the accepted audio
snapshot. Play mode changes are rejected while the coordinator is held.

Cancellation consumes every issued watcher acknowledgement before unfreezing,
releases only fences acquired by this operation, and rebases paused wall time.
Cancellation during the unfrozen drain leaves the original pacing epoch intact.
Snapshot byte-budget rejection is recoverable: cancellation resumes the original
game and a subsequent checkpoint succeeds. App.restore_barrier is transient host
coordination, not serialized game state; shutdown clears it before existing worker
teardown. Native output audio remains explicitly rejected before acquiring other
participants because its device queue barrier is not implemented.

Validation: five focused coordinator tests (including real WAV/Offline audio and
headless draw), full editor suite 316 passed / 27 ignored, all-target Clippy with
-D warnings, scoped rustfmt and tracked-source diff checks passed. The composed
Play/resources GPU test passed on Apple M4 Max/Metal with the coordinator held;
reference, running-origin restore and idle-origin restore readbacks match exactly
(64x64 RGBA, 946 nonblack pixels). See
production-readiness-evidence-2026-10-27/coordinated-barrier-qualification.json
and coordinated-play-gpu-files for evidence.

These are internal coordination and offscreen/headless checks. Full App atomic
publication, menu commands, first native presentation/input handoff, rich native
UI/window acceptance and native device audio remain open. No physical memory or
displayed FPS qualification follows. ENG-03 through ENG-13 remain incomplete.


## 27 October: one immutable coordinated Play/model/UI/Offline-audio capture

App.coordinated_play_checkpoint requires the completed common restore barrier
and captures the existing PlayResourcesCheckpoint, complete UiLiveCheckpoint and
AudioSessionCheckpoint into one owned participant group. Model/runtime capture
continues to use the original shared animation/resources graph; UI includes its
native publications and reload checkpoint, not just reload metadata. It does not
start another simulation, publish a target or change participant formats.

The rich Play/resources regression compares the group's game archive byte for
byte with the original native capture, checks UI encoding and exact audio JSON,
and verifies that UI-budget rejection leaves game ticks and the frozen source
intact before a successful retry. Existing source-free continuation and detached
GPU checks remain in the same test path. Editor: 316 passed / 27 ignored; strict
all-target Clippy and scoped format/diff checks passed. Explicit GPU test passed
on Apple M4 Max/Metal; all three 64x64 readbacks match (946 nonblack pixels).
Evidence: production-readiness-evidence-2026-10-27/coordinated-capture-qualification.json.

This is a typed capture group, not an enclosing full-session disk transaction or
atomic App publication. Cross-owner preparation, authoring/extraction/non-model
Graphics, menu/first-native-presentation/input handoff and native device audio
remain open, along with physical memory/FPS qualification and ENG-03 through ENG-13.


## 27 October: detached coordinated game/UI/Offline-audio preparation

CoordinatedPlayCheckpoint.prepare now owns one prepared Play/resources unit and
constructs the candidate AuthoringProject from its original recipe and shared
component registry. Native UI and audio preparation resolve durable identities
against that unit's LoadedScene. No live App owner is installed during preparation.
PreparedCoordinatedPlay closes model/UI/audio workers on drop without blocking;
explicit discard returns join handles for teardown outside frame-critical code.

The rich test prepares all participants, verifies fresh scene identity and native
Play recapture, discards and joins workers, injects an unsupported audio version
that rejects after game/UI preparation, retries successfully and verifies source
ticks/freeze remain intact. Another preparation succeeds after the source project
directory is deleted, using an empty target root. The coordinated fixture has no
active audio source or presented UI text; richer individual owner tests remain
separate and do not prove full native-window composition.

Final editor suite: 316 passed, 27 ignored; strict all-target Clippy, scoped
rustfmt and tracked-source diff checks passed. Explicit GPU regression passed on
Apple M4 Max/Metal; reference/running/idle 64x64 readbacks match exactly, with 946
nonblack pixels. Evidence: production-readiness-evidence-2026-10-27/coordinated-prepare-qualification.json.

Full App publication, authoring/extraction/non-model graphics composition, an
enclosing full-session disk format, menu commands, first native presentation/input
transfer and native output-device audio still remain open. No physical memory or
displayed FPS claim; ENG-03 through ENG-13 remain incomplete.


## 27 October: coordinated candidate source freshness

Each common restore barrier now has a process-local unique Arc epoch.
CoordinatedOrigin combines that epoch with the existing running/idle native
PlayResourceGuard and hashes of native UiLive and freshly polled Offline audio
checkpoints. The current audio owner is observed instead of trusting the cached
barrier snapshot. prepare_guarded binds the observation to the detached candidate;
validate_origin rejects a missing observation or changed source before publication.
This is an observation API, not a new persistent runtime or a commit permit.

The composed rich regression verifies initial admission, rejection after input
mutation or native audio-mode switch, revalidation after exact restoration, and
permanent rejection after barrier cancellation/reacquisition. The real WAV-owner
regression directly advances native audio during a frozen barrier: the fresh
fingerprint changes while cached audio remains unchanged. No App is installed.

Validation: 316 editor tests passed, 27 ignored; strict all-target Clippy and
scoped format/diff checks passed. GPU test passed on Apple M4 Max/Metal with exact
reference/running/idle readback equality (946 nonblack pixels). A rejected GPU
run reused an existing evidence directory and failed create_new; the log and
original evidence were preserved, then the test passed in a distinct directory.
Evidence: production-readiness-evidence-2026-10-27/coordinated-origin-qualification.json.

Final borrow-scoped App publication, window/input routing and first native frame
handoff, authoring/extraction/non-model Graphics and native device audio remain
open. Physical memory/FPS and ENG-03 through ENG-13 remain unqualified.


## 27 October: native UI publication under the completed source barrier

The original UI publication API rejects any held save barrier. Coordinated App
restoration needs to retain its completed source fence until final acceptance,
so UiLive now exposes checkpoint_publication_at_barrier. It delegates to the
existing exclusive native GPU staging/publication implementation and admits only
a Ready barrier without active import/scan or unfinished retired work. The ordinary
publication path continues to reject held save barriers.

The new real-GPU regression drains the source UI normally before restoring target
scene descriptors and deleting source fonts. It verifies absent/Draining rejection,
ordinary-path refusal of Ready, and retention of Ready after geometry/image budget
faults, late CPU/GPU faults and pending/validated candidate discard. Successful
publication retires the old watcher, replaces owner/cache resources and leaves
restored focus/input unavailable until explicit frame_presented acknowledgement.
128x128 saved/restored RGBA matches exactly and differs from the old target.
The original ordinary publication regression still passes, including retirement
of a genuinely active old import worker.

Final checks: 316 editor tests passed, 28 ignored; both GPU regressions explicitly
passed on Apple M4 Max/Metal; strict all-target Clippy and scoped format/diff passed.
The first frozen test omitted owner-side polling for an already-issued watcher
scan and hit its deadline; the rejected log is retained alongside corrected runs.
Evidence: production-readiness-evidence-2026-10-27/ui-frozen-publication-qualification.json.

This closes one participant's barrier/publication conflict, not the whole App
transaction. Shared model/UI GPU cache composition, authoring/extraction/non-model
Graphics, audio commit, native first-presentation/input handoff and full ENG-03
through ENG-13 remain open. No native-window, physical memory or FPS claim.


## 27 October: Offline audio publication under the completed source fence

AudioSession now provides checkpoint_publication_at_barrier alongside its original
ordinary publication API. It polls the native current owner checkpoint without
waiting and admits only a completed source barrier. Missing/unready state and
serialization-budget errors retain the original owner and fence. Both routes reuse
the same exclusive publication permit; ordinary loading still rejects held saves.
Successful owner replacement clears the old session checkpoint_requested flag and
retires the original owner's workers. Dropping a permit does not release the fence.

A new real-WAV test compares the full source checkpoint before/after permit discard,
checks byte-budget rejection and retained frames/fence, then publishes the candidate
without an intervening resume. 120 continuation steps match the original native
playback state and exact peak bits; final playback checkpoints match. The original
audio-only load test remains unchanged in behavior and also passes.

Final editor suite: 317 passed, 28 ignored; strict all-target Clippy, scoped rustfmt
and tracked-source diff checks passed. Evidence:
production-readiness-evidence-2026-10-27/audio-frozen-publication-qualification.json.

This is Offline AudioSession owner publication, not native callback/device queue
transfer or the whole App transaction. Joint model/UI cache composition, authoring,
extraction, non-model Graphics and native first-presentation/input handoff remain
open. No new GPU/window/memory/FPS claim; ENG-03 through ENG-13 remain incomplete.


## 27 October: combined model/UI GPU cache staging

UiPublicationPermit.stage_after_models now consumes a validated native ReadyModels
owner and continues that candidate's cache for UI upload. The group owns models
through asynchronous UI validation/discard. Successful publication installs one
combined cache and returns model owners without a second cache that could overwrite
the shared index. The enclosing host must still install those returned owners with
its other infallible swaps; this does not publish a whole App.

The model adapter revalidates original device/queue/cache identity/state and native
source inventory, measures actual model/animation geometry, checks candidate cache
policy/inventory and computes old-source-plus-new-model peak admission. UI staging
adds new UI to that peak; final accounting replaces old UI geometry. PreparedUiLive
now retains a process-local target SceneChangeToken, so a foreign or edited target
rejects before combined upload. This token does not change durable UI formats.

The real GPU fixture combines an imported model, an explicit native textured mesh
part and original text UI. It verifies foreign-scene rejection, a changed source
cache identity, aggregate peak rejection and validated candidate discard, then
publishes the joint cache. UI pixels match the saved 128x128 reference exactly.
Native accounting: source UI geometry 2408 bytes, new model geometry 10432, final
12840, combined staging peak 15248; limit 15247 rejects. UI image inventory remains
1398104 bytes and the model texture adds 4 bytes in the final shared cache.
These are logical native allocation inventories, not measured physical memory.

Validation: 317 editor tests passed, 29 ignored; strict all-target Clippy and scoped
format/diff passed. Combined GPU test passed on Apple M4 Max/Metal; the original
ordinary UI GPU publication regression also passed. Initial large-error Clippy,
unboxing compilation and mistaken imported-node fixture failures are retained.
Evidence: production-readiness-evidence-2026-10-27/model-ui-cache-qualification.json
and model-ui-cache-scene-verified.

Full CPU/App/authoring/extraction/audio/non-model Graphics publication, native
first-presentation/input transfer, native audio callback queue, physical memory/FPS
and ENG-03 through ENG-13 remain open. Shared test uses static model GPU owners;
accepted animation and LOD have separate evidence, not joint UI qualification.


## 27 October: native UI input adapter joins coordinated preparation

CoordinatedPlayCheckpoint now captures the existing GameUiInputCheckpoint with
reserved historical identities, alongside Play/model resources, UiLive and audio.
PreparedCoordinatedPlay restores this adapter against the same LoadedScene using
existing UI element/layout-byte limits. The source freshness hash now includes the
current native adapter, so focus/capture/admitted-layout changes invalidate an
already-prepared candidate. There is no second UI input runtime or new adapter codec.

The rich fixture explicitly admits its current native UI snapshot, focuses a real
button and holds Enter before capture. Candidate routing re-encodes identically
after durable-ID rebinding. After explicit snapshot admission, release emits one
click on the restored button; a second release emits none. Cancelling the source
adapter rejects the guarded candidate; exact native checkpoint restoration
revalidates. Existing source-free game continuation and GPU paths also pass. This
fixture supplies an acknowledged snapshot directly; it does not prove actual
native-window presentation or final App handoff.

Final suite: 317 editor tests passed, 29 ignored; strict all-target Clippy and
scoped format/diff passed. Full Play/resource GPU test passed on M4 Max/Metal with
exact reference/running/idle pixels. An initial missing test import and a parallel
audio-fixture timestamp collision are retained in rejected logs. The audio fixture
now adds a process-local atomic counter, and the complete parallel suite passed.
Evidence: production-readiness-evidence-2026-10-27/coordinated-ui-input-qualification.json.

Whole-App publication, OS device/modifier routing and first presented-frame input
transfer remain open, as do authoring/extraction/non-model Graphics composition,
native audio callback queues and physical memory/FPS qualification. ENG-03 through
ENG-13 remain incomplete.


## 27 October: coordinated Play/model/UI/audio/input disk file

CoordinatedPlayCheckpoint now has a bounded VOXYCGR1 transport: a 72-byte header
contains four checked section lengths and BLAKE3 integrity over the length table
and native participant payloads. It embeds existing PlayResources, UiLive,
AudioSession JSON and GameUiInput JSON codecs without introducing another runtime.
Independent section caps and total encoded-file caps apply. read_file bounds reads
through opened-file metadata and max+1 before decoding. save_file encodes before
using the existing trusted-path atomic writer; its directory-durability/platform
overwrite limits remain those of that original writer.

The rich regression verifies real disk roundtrip/re-encoding, preservation of an
existing file after budget rejection, and rejection of corruption, trailing bytes,
resealed overflowing section length, resealed invalid UTF8 and section-budget
violations. A current file also includes the exact accepted model GPU payload.
After removal of the source project, it is reread into an empty target root and
all coordinated owners prepare; its decoded game drives the original GPU staging
and full 120-frame continuation checks. The retained accepted file is 160216 bytes.
UI input is nonempty (held Enter/focus), but UiLive/audio are empty in this rich
fixture; nonempty native owners retain their separate tests.

Final editor suite: 317 passed, 29 ignored; strict all-target Clippy and scoped
format/diff passed. Actual GPU path passed on Apple M4 Max/Metal with identical
reference/running/idle readbacks (946 nonblack pixels). Initial local error
conversion compilation and obsolete-import Clippy failures are retained. Evidence:
production-readiness-evidence-2026-10-27/coordinated-file-qualification.json and
coordinated-file-accepted-gpu-files.

This is a coordinated participant file, not complete App/session persistence.
Authoring/extraction/non-model Graphics composition, menu/atomic host publication,
native first-presentation/input handoff, native audio callback queues and physical
memory/FPS qualification remain open. ENG-03 through ENG-13 remain incomplete.


## 27 October: nonempty audio in the coordinated source-free file

The rich coordinated-file regression now creates an actual WAV, attaches a looping
AudioSource to a durable scene object and prepares/advances native Offline AudioPlay
before entering the common barrier. Both the initial and current file contain
nonempty native audio playback/resources/reload state alongside game/model and
held UI input. Ordered barrier polling now resolves actual audio owner identities.

After deleting the source project, the file prepares all participants into an empty
target root. The prepared audio owner is published into an independent native
Offline AudioSession against that same restored scene. Original/restored native
audio owners advance through 120 mixed tick counts; frame counts, exact peak bits
and final complete playback JSON match. Workers are closed/joined before discard.
Existing source-free game continuation and actual accepted GPU frame checks remain.
The audio and game continuation controls are separate; this does not qualify a
single installed App lifecycle or synchronized native output device.

Validation: 317 editor tests passed, 29 ignored; strict all-target Clippy and scoped
format/diff passed. Full Play/resources GPU test passed on Apple M4 Max/Metal with
exact reference/running/idle pixels. Evidence and current section sizes:
production-readiness-evidence-2026-10-27/coordinated-file-audio-qualification.json
and coordinated-file-audio-gpu-files.

This supersedes the earlier rich-fixture empty-audio limit; UiLive presented text
is still empty in this fixture and has separate native GPU tests. Whole App
publication, authoring/extraction/non-model Graphics, menu/first-presented-frame
input handoff and native audio callback/device queues remain open. No physical
memory/FPS or complete ENG-03 through ENG-13 qualification is claimed.


## 27 October: nonempty text/audio coordinated file and exact arena admission

A new explicit GPU regression captures original published/presented text UI and
font resources into the same coordinated file as native scene/game and WAV audio.
After the model, WAV and font source files are removed, every participant prepares
into an empty project directory. Restored native audio playback JSON matches. The
same decoded UI participant feeds the original native GPU publication tests; saved,
restored and combined-model/UI 128x128 RGBA match exactly and differ from the old
live target. Font/glyph alias history, rollback/fault/discard and frame-ACK gates
remain exercised. The fixture temporarily moves its existing scene into App for
capture; this is not a full editor authoring/extraction or menu load workflow.

Complete preparation exposed a native scene defect: SceneSimulationCheckpoint
loaded the scene at the caller's budget capacity, then demanded exact agreement
with the saved allocator. A 4-slot source consequently failed under a ceiling of
16. It now checks saved capacity against the ceiling and loads at the exact saved
capacity. The original allocation validation is retained. A regression verifies
exact checkpoint equality under a larger ceiling, rejection under a smaller one,
and identical future slot/generation allocation and capacity exhaustion. No scene
format changes or additional capacity are introduced into the restored runtime.

The coordinated file encoder also applies the remaining aggregate allowance to
each native participant and returns before encoding subsequent sections if the
header/first section exhausts the budget. Header-only and first-section-only
limits are checked through the composed regression; previous file preservation
and corruption/section-limit gates remain. This is encoded-buffer admission, not
a measured total temporary heap or physical GPU-memory guarantee.

Validation: 317 editor tests passed / 30 ignored; voxy_scene 98 unit + 32 integration
tests passed; strict all-target Clippy for both packages and scoped format/diff
checks passed. Explicit GPU scenario passed on M4 Max/Metal. Rejected allocation
configuration and initial regression compilation logs are retained. Evidence:
production-readiness-evidence-2026-10-27/coordinated-text-file-qualification.json and
coordinated-text-budget-gpu-files.

Remaining: complete App publication, authoring/extraction/non-model Graphics and
OS input ownership, native first presentation/menu integration, native output
audio callbacks and physical memory/FPS qualification. Prepared UI group checking
and explicit offscreen UI publication are separate; they do not prove a complete
installed application lifecycle. ENG-03 through ENG-13 remain incomplete.


## 27 October: authoring history and coordinated game preparation

App.editor_checkpoint now captures the original AuthoringHistoryCheckpoint
alongside the coordinated Play/model/UI/audio/input checkpoint under the completed
common barrier. EditorCheckpoint.prepare first constructs the detached coordinated
game, then rebinds native authoring history/order/selection/draft state against
that same restored LoadedScene. A history error drops only the detached candidate
and uses its existing worker cleanup; the live App/history are not mutated.

The new regression creates a real project, duplicates objects, performs undo and
restores the authoring view, then enters the normal native Play path. It captures
and prepares both owners, verifies identical game/authoring target SceneId with
fresh node handles and retained order/selection, and compares complete native
history checkpoints across redo/undo operations. A missing authoring order identity
rejects after game preparation; the source checkpoint and frozen state survive.

Validation: 318 editor tests passed, 30 ignored; strict all-target Clippy and scoped
rustfmt passed. Evidence:
production-readiness-evidence-2026-10-27/editor-history-compose-qualification.json.

This is typed owner composition. Authoring history is not yet persisted in the
coordinated file, and remaining AuthoringSession project/source revisions, preview
and settings fields are separate. Complete App/extraction/non-model Graphics
publication, native menu/first-presented-frame input handoff, native device audio
and physical memory/FPS remain open, along with full ENG-03 through ENG-13.


## 27 October: one native game + authoring history disk file

EditorCheckpoint now supports a VOXYEDR1 envelope with two bounded sections: the
unchanged coordinated game file and original AuthoringHistoryCheckpoint JSON.
Checked lengths and BLAKE3 cover the section table and payloads. Encoding passes
the remaining aggregate allowance to each native codec. File reads are bounded
and writes delegate to the existing trusted-path atomic writer after encoding.
The underlying VOXYCGR1 and history formats remain unchanged.

The real-project Play/history regression now saves and reloads this file before
preparing the fresh game/authoring pair. It verifies exact re-encoding and original
redo/undo checkpoint equality, order/selection and fresh scene handles. Corrupt
payloads, overflowing lengths, trailing data and total/read budgets reject; a
failed budget save preserves the original bytes. Late invalid authoring identity
preparation still preserves the original frozen App and history. Evidence:
production-readiness-evidence-2026-10-27/editor-history-file-qualification.json
and editor-history-file/editor.voxyedr.

Validation: 318 editor tests passed, 30 ignored; strict all-target Clippy and scoped
format/diff passed. This fixture has no active GPU/font/audio owner; earlier native
coordinated-participant tests cover those richer inner payloads separately. No new
GPU/window/device-audio or memory/FPS qualification follows.

Remaining: other AuthoringSession source metadata/project paths/revisions, previews
and settings ownership, full App/extraction/non-model Graphics publication, menu
and first native presentation/input handoff, native audio callback queues and
full ENG-03 through ENG-13. This file is not complete editor-session persistence.


## 27 October: native source observation on the editor/history candidate

EditorCheckpoint.prepare_guarded now retains CoordinatedOrigin in its original
prepared game owner. PreparedEditorCheckpoint.validate_origin rejects a mismatched
prepared history/game target SceneId before delegating the original source game,
model, UI/input/audio and native scene/history/draft observation. An unguarded
candidate remains rejected rather than silently acquiring publication permission.

The real-project Play/file/history regression now admits an observed candidate,
rejects a deliberately mismatched history target scene, and rejects source redo.
Undoing back to the original history values still rejects the stale candidate: the
native HistoryChangeToken epoch cannot be resurrected by an ABA value reversal.
Separate complete native history checkpoint comparisons remain intact.

Validation: 318 editor tests passed, 30 ignored; strict all-target Clippy and scoped
rustfmt passed. Evidence:
production-readiness-evidence-2026-10-27/editor-history-origin-qualification.json.

This is source revalidation, not a borrow-scoped whole-App commit. Remaining
AuthoringSession metadata/project/source revision/preview/settings owners, final
App/extraction/non-model Graphics publication, native menu/first-frame input
handoff, native output audio and physical memory/FPS remain open. Full ENG-03
through ENG-13 remains incomplete.


## 27 October: authoring metadata and project source observations

SceneAuthoringGuard now also observes original settings_written hashes, scene path,
stored SceneRevision path/digest receipt, prefab inventory/choice and pending
preview seek owner/phase words. SceneRevision gains Clone/Eq only; its existing
filesystem validation and serialization behavior are unchanged. Metadata can now
invalidate a coordinated/editor candidate without any SceneGraph mutation.

AuthoringProject gains a process-local observation identity. with_registry creates
a fresh owner; worker_project preserves the original identity and registry. The
guard holds both identity and component-registry Arcs and compares allocation
identity, so replacing a project or registry is detected while the worker alias
remains valid. No persistent project ID or duplicate project runtime is introduced.

A new native-App regression checks each metadata mutation while the scene change
token remains identical, including +0/-0 pending seek words and stored revision
replacement. It also accepts the worker alias and rejects registry/new-project
replacement. Final editor suite: 319 passed, 30 ignored; strict all-target Clippy
and scoped format/diff passed. Evidence:
production-readiness-evidence-2026-10-27/authoring-metadata-guard-qualification.json.

This is source observation only. These metadata fields are not yet archived or
installed by the editor file; completed animation preview and imported authoring
source provenance still require their own decisions. Stored revision comparison
is not a fresh filesystem validation. Full App/publication/menu/first-frame input
handoff, native device audio, physical memory/FPS and ENG-03 through ENG-13 remain
open. No new GPU/window qualification is claimed.


## 27 October: bounded local authoring metadata in editor files

AuthoringHistoryCheckpoint v2 now includes original settings-write hashes, scene
path/SceneRevision receipt, prefab inventory/selection and pending preview owner/
rectangle. Existing editor envelopes transport this native JSON without changing
their outer format. v1 remains readable with explicit metadata absence; v2 requires
the metadata participant. Native prepared metadata rebinds reserved object IDs,
retains exact f32 rectangle words and installs the original fields infallibly.
Archive bytes bound metadata; asset inventory is independent of scene draft count.

SceneRevisionCheckpoint retains original path/hash and prepares without filesystem
reads. A regression deletes the source receipt file before restoring metadata,
verifies source/target authoring JSON equality and fresh pending-preview handles,
then checks missing file rejection, original-byte recreation acceptance and altered
byte rejection through the unchanged native SceneRevision.validate. Legacy v1,
v2 missing metadata and mismatched revision paths are checked explicitly. Paths
retain trusted local host-path semantics; relocation/Save As is not qualified.

Final editor suite: 320 passed, 30 ignored; strict all-target Clippy and scoped
format/diff passed. The editor envelope roundtrip also exports native authoring v2.
Initial AssetId serde compilation, former v2-invalid expectation and empty-project
fixture indexing failures are retained; logical ID encoding, unknown-v3 rejection
and normal editor object creation corrected those cases. Evidence:
production-readiness-evidence-2026-10-27/authoring-metadata-file-qualification.json
and authoring-metadata-file/editor.voxyedr.

Remaining: completed animation-preview model/frame state and imported AuthoredScene
provenance, full project/AuthoringSession/App/extraction/non-model Graphics owner
composition, native menu/first-presentation/input handoff, native audio callbacks
and physical memory/FPS qualification. No new GPU/window qualification follows;
ENG-03 through ENG-13 remain incomplete.


## 27 October: original imported authoring source and prefab provenance

AuthoringHistoryCheckpoint v3 now archives the original optional ImportedAsset of
AuthoredScene through the existing bounded imported-publication codec. It retains
source/expanded documents, nested dependency documents, original accepted input
bytes/hashes and input policy. Decode uses the original bounded prefab expander
with only archived dependencies and the supplied registry, verifies exact expanded
document encoding and dependency inventory, and performs no provider/file reads.
The original imports remain distinct from dirty authoring history and live game.

A nested root/nested/leaf fixture removes every source file before restoring and
compares complete native payload and input observations. Correctly checksummed
archives with a wrong expanded document or missing dependency reject semantically;
raw corruption also rejects. Legacy v2 source absence remains explicitly
unspecified and does not erase an existing source, while v3 declared absence
clears it. An explicit enum carries that distinction to native installation.
The editor envelope regression exports v3 with an actual imported root input.

Final suite: 321 editor tests passed, 30 ignored; strict all-target Clippy and scoped
format/diff passed. Initial local InputError conversion and Clippy findings are
retained. Evidence:
production-readiness-evidence-2026-10-27/authoring-source-qualification.json and
authoring-source-file/editor.voxyedr.

Remaining: imported-source freshness observation and completed animation-preview
resources, full project/App/extraction/non-model Graphics publication, native
menu/first-presentation/input handoff and native audio callbacks. Local path
semantics are retained; no Save As relocation, new GPU/window or physical memory/
FPS result is claimed. Full ENG-03 through ENG-13 remains incomplete.


## 27 October: original authoring-source content freshness

CoordinatedOrigin now retains an explicit optional fingerprint of the accepted
original AuthoredScene imported archive and its byte policy. Revalidation rebuilds
that native bounded archive, including payload, dependencies, input bytes/hashes
and saved input policy, and rejects changed content before accepting the source.
The existing pure native dependency-expansion checks are reused without provider
reads or a second source representation. This automatically covers editor guarded
candidates through their original coordinated source observation.

The real-project Play/history regression now removes the source publication,
installs a valid different imported prefab source, and verifies both operations
invalidate the candidate while scene/history tokens remain identical. Returning
the original accepted publication revalidates. This compares archive content, not
allocation identity; it does not establish a new filesystem baseline. Native
SceneRevision/file validation remains a separate operation.

Validation: 321 editor tests passed, 30 ignored; strict all-target Clippy and scoped
rustfmt passed. The initial explicit-type-path test compilation rejection is
retained. Evidence:
production-readiness-evidence-2026-10-27/authoring-source-origin-qualification.json.

Remaining: complete animation-preview ownership and observation, full project/App/
extraction/non-model Graphics publication, menu/first-presented-frame input
handoff, native output audio and physical memory/FPS qualification. No new
GPU/window qualification follows; ENG-03 through ENG-13 remain incomplete.


## 27 October: immutable preview observation and accepted frame participant

Native Preview now provides a process-local observation of owner, exact f64 phase
words and strong original model/source/frame Arc identities. SceneAuthoringGuard
includes it, so changed previews invalidate coordinated/editor preparation even
when scene and game clocks are unchanged. Private settings/profile are rebuilt
together with the accepted frame. A guard retains old allocations to avoid Arc
identity reuse while the observation is live.

PreviewCheckpoint captures original owner identity, canonical external target/
source model keys, phase words, native settings/profile and AnimatorFrameCheckpoint.
Preparation resolves the original roots, validates clip/joint/profile bindings and
reconstructs the accepted frame against its target skeleton without resampling or
advancing any runtime. Canonical root keys must preserve original Arc aliasing.

A real imported animated-triangle fixture verifies same-pose recomputation yields
an identical frame checkpoint but invalidates the source observation, signed-zero
phase and removal invalidate, and scene/game-clock/event state remains unchanged.
Native preview checkpoint JSON restores to a fresh owner with exact frame JSON and
model/source aliasing; missing roots and inconsistent resolved aliases reject.
Final editor suite: 322 passed, 30 ignored; strict all-target Clippy and scoped
format/diff passed. Evidence:
production-readiness-evidence-2026-10-27/preview-checkpoint-qualification.json.

This participant still needs joining to the original model/resource graph and
shared accepted-frame Arc pool before inclusion in the editor file. It does not
qualify full App publication, native menu/first-presentation/input handoff, GPU
preview reconstruction, native device audio or physical memory/FPS. Full ENG-03
through ENG-13 remains incomplete.


## 27 October: preview roots in the original native model graph

Preview now exposes canonical original target/source ModelAsset roots, retaining
one root when both roles share an Arc. The existing EditorAssetCheckpoint external
model-root graph is used directly; no alternate model/resource encoder is added.
PreviewCheckpoint restores its accepted frame through freshly decoded graph roots.

The ordinary preview regression captures the native model graph, restores fresh
model allocations and rebinds a fresh scene owner while matching complete accepted
AnimatorFrame checkpoint JSON exactly. The retarget regression also archives its
two distinct original target/source roots, restores both fresh allocations and
validates the original profile and exact accepted target frame. All three native
preview tests pass. The exported single-root graph and preview JSON are retained
in preview-shared-model-graph; tests do not re-import source GLB during restore.

Validation: 322 editor tests passed, 30 ignored; strict all-target Clippy and scoped
format/diff passed. Initial asset-only prepare test compilation is retained; the
correct original prepare_with_models path exposes external roots. Evidence:
production-readiness-evidence-2026-10-27/preview-model-graph-qualification.json.

This validates the original graph adapter, not one combined runtime/catalog/preview
root transaction. Original root union and cross-owner preview/GPU/runtime frame Arc
pooling must still be composed before inclusion in the editor file. Full App
publication/menu/first-presentation/input handoff, native output audio, physical
memory/FPS and ENG-03 through ENG-13 remain open. No new GPU/window claim follows.
