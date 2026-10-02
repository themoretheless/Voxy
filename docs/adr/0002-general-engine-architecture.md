# ADR 0002: Capability coverage with independent engine architecture

Status: accepted direction; implementation and performance validation pending.
Date: 2026-10-01

## Requirement

Cover Stride3D/Unity3D/Godot capabilities without copying their architecture. Their feature
catalogs define user outcomes, not internal abstractions. Voxy must support
editable reusable scenes, simulation, assets, rendering, input, UI, audio,
navigation, scripting, networking, editor and all requested platforms.
ADR 0001's single-writer, snapshots, versioned jobs and bounded work remain valid.
Its V1 exclusions do not exclude capabilities from this expanded goal.

## Decision

Separate authoring, authoritative simulation and presentation. A scene tree is
an authoring and transform relationship, not the storage layout for every
simulation subsystem. Parent relationships do not imply ownership of GPU handles,
physics solver memory, audio streams or assets. Each domain owns its data and
exports explicit handles, commands and snapshots.

- Authoring: versioned scene documents, stable asset/object identifiers, prefab
  references/overrides, component schemas and migrations. Editor transactions
  support undo. Loading validates and resolves all references before commit.
- Simulation: generational entity handles and typed component tables. Frequently
  processed data uses contiguous storage; hierarchy metadata stays separate.
  Systems declare read/write access and phase dependencies. Structural edits use
  command buffers applied at explicit barriers. Conflicting access cannot run
  concurrently. Independent systems may run concurrently after validation.
- Transforms/activity: dirty propagation with cached world transforms and effective
  activity. Cycle rejection and stale-handle protection remain invariants. No
  full ancestor walk per component per frame in the final large-scene path.
- Presentation: immutable extracted frame snapshots and stable resource handles.
  Rendering consumes cameras, drawables, lights and effects without a mutable
  gameplay-world reference. GPU uploads and residency have explicit budgets.
- Assets: asynchronous dependency graph, stable IDs plus content revisions,
  bounded tasks, cancellation and stale-result rejection. Import formats are
  adapters; runtime resources do not depend on editor object representations.
- Physics/audio/navigation: domain-specific compact stores and batch processing.
  Scene components configure these domains. Collision/event results cross an
  explicit phase boundary. No callbacks mutating a solver while it iterates.
- Scripting: an adapter to the command/event API, with observable lifecycle and
  quotas. Rust behaviors are a convenience adapter, not the universal system
  dispatch mechanism. No mandatory per-object virtual update call for bulk work.
- Editor: consumes the same schemas and commands as tools/runtime, with a distinct
  authoring world and play world. Undo edits authoring transactions; it must not
  rewind arbitrary GPU resources or live platform callbacks.
- Platform: input and window/device lifecycle feed explicit events. Fixed ticks,
  frame updates, pause and application suspension are separate policies.
- Persistence/networking: explicit versioned schemas and entity-reference mapping;
  Rust TypeId, pointers and runtime slot indices never form durable IDs.

## Existing code and migration

### LOD ownership and publication boundary

The importer owns bounded source observations and a verified immutable CPU
certificate. EditorAsset retains that certificate through Arc; geometry errors are
derived from its witnesses, never accepted from a simplifier metric. The .vmodel
recipe is an importer adapter and does not become a gameplay component schema.

ModelGraphics owns either one SceneGeometry or one SceneLodGeometry bundle, so a
LOD model has no additional standalone base allocation. Shared vertex/material
streams count once; all level index buffers count toward residency. An upload is
staged within the available old-plus-new peak budget and replaces the published
GPU model only on success. Failed import retains the previous CPU revision; failed
GPU admission retains the previous GPU publication and records a deferred attempt.

Selection history belongs to the instance/view, outside the shared model resource.
SceneLodHistory weak bundle identity discards history on replacement without
retaining previous GPU resources. Level selection changes a draw reference; it
does not change authoring geometry, picking, collision or asset ownership. The
editor's initial policy is one geometric error pixel with 15 percent hysteresis.
Legacy identity projection and invalid camera/bounds use base geometry. Native
presented-frame and reload tests validate this path; material/image equivalence,
streamed level eviction and a user-facing quality control remain separate work.

SceneGraph's per-node HashMap is a bootstrap API, not the final hot-loop layout.
BehaviorRunner's mutable-scene hooks are a single-thread convenience adapter;
keep their semantics testable while moving core systems to declared access and
commands. SceneApp's rotating cube, hardwired primary object and direct resource
references remain a demonstration, not a general application-world contract.
A behavior cannot delete its primary demo object until rendering accepts an
extracted scene. Do not extend the demo struct with one Option field per feature.

Migration sequence:
1. Define schemas, durable IDs, reference resolution and transactional scene IO.
2. Add typed table storage, structural commands and an explicit phase scheduler;
   migrate a real gameplay system, measuring baseline against the bootstrap API.
3. Cache hierarchy transforms/activity with invalidation tests for edits/reparent.
4. Extract renderer snapshots; drive scene examples through systems and snapshots.
5. Integrate assets, input/UI/audio, physics/animation/navigation and the editor
   through these boundaries, then networking and remaining platform acceptance.
Each step keeps existing runnable examples working. Avoid rewriting every domain
at once or introducing abstractions without an actual migrated consumer.

## Acceptance evidence

Correctness: invalid handles, cycles, reference remapping, transactional load,
command ordering, read/write conflict validation, callback/system changes during
iteration, stale asset completions, undo and simulation isolation.

Performance: instrumented 1k/10k/100k mixed scenes with deep and wide hierarchies,
component churn, 0/1/100 percent transform changes, inactive subtrees and asset
streaming. Record tick/extraction p50/p95/p99, allocations and peak memory on
named hardware. Compare cached transforms and typed storage against existing
implementations before making performance claims. Report budget overruns.

Operational: headless simulation tests, real GPU pixel/frame checks, versioned
save/load roundtrips and actual device/platform gates. Narrow unit tests do not
prove complete engine capability coverage or an ideal architecture.

## Tradeoffs

Typed domain stores require reference mapping and extraction code. Command
barriers delay effects until a defined phase. Schema registration adds authoring
work. These costs buy inspectable ownership and scheduling; they still require
measurement. There is no universally ideal architecture independent of workload.
This decision defines explicit criteria for improving Voxy toward the requested
architecture instead of equating familiar engine APIs with quality.

### Enforced package boundaries (2026-10-02)

Prefab source identity is validated before instance ID framing at every expansion
level, including authoring baselines. Previously an empty source object ID became
a nonempty derived instance ID and passed final validation. A regression failed
on the previous implementation and passes after rejecting empty/duplicate source
IDs. All eight persistent-prefab integration tests pass, including nested instance
save/load, reference remapping, deletion/reparent/resurrection and override capture.
This proves the identity fix, not complete editor prefab acceptance.

The headless editor prefab integration also exercises Play/Stop on two nested
instances after saving overrides. Runtime name/activity edits are discarded;
pre-Play and runtime handles expire across the transitions. Durable IDs, source
metadata and on-disk bytes survive, F5 while playing cannot save runtime edits,
and the next undo/redo still reaches the original authoring edit. The extended
integration test passes. This is application-path proof without a native window.

Flat project scenes now use the same observed-source validation before saving as
composed scenes. Previously their null composition metadata selected an unchecked
write that could overwrite an external scene edit. The regression reproduces that
failure, then verifies two successive own saves, rejection with exact external
bytes and live scene retained, and reload followed by a successful save. Own writes
refresh observations. Legacy flat paths outside the model project remain unobserved;
this is not an atomic compare-and-swap against a writer racing after validation.
Validation: 70 editor library tests passed, two GPU tests remained ignored in this
run; package boundaries and whitespace checks passed.

Atomic scene replacement now copies the existing destination permissions onto
the private replacement before its sync/rename. Unix regression verifies mode
0640 survives replacement. A forced rename failure against a populated directory
reports `committed=false`, removes the temporary sibling and retains the original
contents. Ownership/extended attributes are not copied. All 60 scene library tests
pass; these checks do not prove recovery from power loss or concurrent-writer CAS.

Legacy flat scenes outside the asset project now retain a bounded exact-content
revision. The same bytes used for parsing establish the digest, successful saves
record the expected serialized bytes, and failed loads leave the previous revision
intact. The editor integration verifies repeated own saves, rejection of even a
formatting-only external change with disk/live scene preserved, and refusal to save
over a malformed external file after failed reload. All 70 editor library tests
pass (two GPU tests ignored in this run). Reads are capped at 1 MiB. This closes
the unobserved legacy-file gap but still permits a writer race after validation;
it is not filesystem compare-and-swap or a cross-process lock.

Inherited dependencies now resolve through root `workspace.dependencies`, including
renamed packages in optional and target-specific build dependencies. Missing
inheritance fails the check instead of silently omitting the edge. Five focused
regression tests cover inherited aliases, transitive violations, missing entries,
direct aliases and the intentional development-dependency exclusion. The current
gate passes for four owners and 32 packages; no existing inherited violation was
found. This strengthens the gate, not a claim that runtime ownership is complete.
Run the regression checks with
`python3 -m unittest discover -s tools -p test_engine_boundaries.py`.

The inventory follows declared workspace members (including glob patterns) and
recursively reads local production/build path dependencies outside that inventory.
It no longer assumes every package lives immediately under `crates/`. A fixture
proves detection of scene -> nonmember bridge -> editor through an inherited
path and target-specific build dependency. Missing member patterns and ambiguous
local package names fail closed. Seven regression tests and the current 32-package
gate pass. Registry/git dependency source graphs and Cargo patch replacements are
not resolved by this manifest-only check.

`python3 tools/check_engine_boundaries.py` checks production and build dependencies,
including optional and target-specific edges and their transitive workspace paths.
Development dependencies remain available for composition tests and examples.
Assets cannot depend on scene/render/runtime/editor; scene cannot depend on
render/runtime/editor; render cannot depend on scene/runtime/editor; the voxel
runtime cannot depend on render/editor. A violation reports the dependency chain.
The current check passes for four owner layers across 32 packages. Negative
checks exercised indirect violations, cycles, package aliases and target/optional
edges. This is a package-layer gate, not proof of runtime borrowing or ownership.

The initial audit found no production consumer of `SchedulePlan`. The character
physics path now shares a validated two-phase plan between editor play and the
headless character example: synchronize after behavior hooks, then step physics.
`SceneSimulation::advance_scheduled` reuses the existing frame command barrier,
fixed-tick lifecycle and pose capture; typed failures retain the system name and
original error, and stop downstream systems and ticks. The built-in plan is
validated once and retained without per-frame rebuilding. Scene access is now enforced by the scoped runner described below; access to
other resources remains declarative. Enforce all domain access before parallel dispatch.
Consequently architecture acceptance remains incomplete; a standalone scheduler
and this dependency gate do not close item 2.

Phase migration validation: `cargo test -p voxy_scene --lib` passed 55 tests;
`cargo test -p voxy_scene --test scheduled` passed the barrier/typed-failure and
zero-tick dispatch regression. `cargo run -p voxy_gameplay --example character_scene`
passed swept movement, quick-tap jump, interpolation, durable reload, typed
component removal and lifecycle Stop with the shared phase plan. The existing
quick-tap/catch-up integration test now uses this same scheduled path.
`cargo check -p voxy_editor --lib` passed after integration and after dispatch
consolidation. The migrated quick-tap integration test passed; the full gameplay
integration suite passed all 11 tests, including unknown-system rejection without
scene/physics/input mutation.

Character phase dispatch is now owned by `CharacterPhysics::run_scheduled_system`
rather than repeated editor/example name switches. It explicitly accepts only
synchronization and stepping; unknown phase names fail before touching scene,
physics or input. A regression checks that rejected dispatch preserves transform,
body count and the pending jump edge. This does not imply generic scoped resource
access; the integration still executes serially with whole-scene mutable access.


### Scoped scene capabilities

`SchedulePlan::run_scene` now lends a private `SceneSystemAccess` capability for
one callback. An absent `scene` declaration rejects reads and writes; read access
exposes only `&SceneGraph`; mutable access requires a scene write declaration.
`SceneSimulation::advance_scoped` uses this path at each fixed tick. Editor and
headless character consumers now synchronize with read access and step with write
access. The domain dispatcher rejects missing write access before physics/input
mutation. Regression tests cover undeclared reads, read-only mutation, granted
writes and a read-only character step preserving pose and pending jump input.

This enforces access to the supplied scene, not arbitrary captured resources or
interior-mutability effects in user components. Legacy behavior hooks and
`advance_with`/`advance_scheduled` still expose the whole mutable scene serially.
Physics and input captures still need typed domain capabilities before parallel
execution. Architecture acceptance therefore remains incomplete.

Scoped-path validation: 55 scene library tests passed; both scheduled scene tests
passed; all 12 gameplay integration tests passed, including read-only character
step rejection with pose/input preservation. Editor library cargo check passed.
The headless character example also passed movement, jump, interpolation, durable
reload and lifecycle checks through the scoped scene path.

### Domain operation admission

The scoped plan now retains all resource declarations, not only the scene grant.
Its invocation capability supports read/write admission checks for named domains.
Character synchronization requires physics write access; stepping preflights
physics write, input write and scene write before touching any of those domains.
Missing and read-only physics/input declarations reject stepping while preserving
live character state, pose and pending jump input. The scene-read-only regression
supplies valid physics/input grants so it continues to isolate the scene boundary.

These checks constrain the owned character dispatcher. They do not prevent an
arbitrary callback from mutating independently captured resources through another
API. Typed domain borrowing and migration of legacy behavior hooks remain required
before any concurrent execution or a claim of complete access enforcement.
Domain admission validation passed: both scheduled scene tests, all 13 gameplay
integration tests and editor library cargo check. The package-boundary gate and
diff whitespace check also passed. No parallel-access or performance claim is made.

### Audio frame phase

Editor play now runs audio through a validated frame plan after fixed simulation.
The plan reads the final scene and writes `audio.playback`; the audio owner checks
both grants before reconciliation, mixer progression or device output. Zero-tick
frames still reconcile scene/audio state, matching the prior lifecycle. No scene
write capability is granted. Audio declaration denial tests verify no changes to
frame/peak/remainder counters or scene activity, followed by successful permitted
rendering. This keeps frame audio separate from fixed physics frequency.
Audio phase validation: editor library tests passed 56, failed 0, ignored 1.
The new scoped-audio denial test and existing play/audio progression/Stop test
passed. The ignored test is the unrelated real-GPU glyph overlay test requiring
an adapter and local TrueType font. Package boundaries and diff checks passed.

### Render extraction phase

Editor draw and the headless character example now project model instances through
one validated extraction plan after simulation. The scene capability is read-only;
`render.extraction` write permission admits mutation of the owned projection.
The pose resolver receives the same immutable scene input, including simulation
interpolation, rather than capturing a mutable scene. No GPU object enters the
scene package. Missing grants reject before staging/resolver work. Resolver and
capacity errors continue to retain the prior complete publication. A regression
checks denied callbacks, denied mutation, successful publication and typed resolver
failure without replacement. Direct refresh APIs remain serial compatibility paths;
this migration does not yet mean all renderer light/camera reads use a snapshot.
Extraction phase validation passed: all 56 scene library tests, 56 editor library
tests (the unrelated real-GPU glyph overlay test remains ignored), and the actual
headless character example. Package layering and diff checks passed.

### Presentation style snapshot

Editor extraction stages an owned camera, one active directional light and bounded
per-model material values alongside model poses. Invalid style/camera input rejects
before pose publication; the style snapshot publishes only after pose extraction
succeeds. GPU transform/material updates consume that snapshot rather than querying
material or light components from the scene. Transform residency reconciliation
uses snapshot owner membership, so inactive owners release their transform entries.
Editor camera navigation remains independently owned; its value is copied once for
projection and LOD selection in the same frame. No performance claim is made for
the additional bounded snapshot allocation; allocation reuse and broader light/
material/shadow extraction remain part of architecture performance acceptance.
Style snapshot validation passed: editor cargo check and 57 editor library tests,
including snapshot independence/capacity/invalid-material rejection. One unrelated
real-GPU glyph overlay test remains ignored. No new native visual claim is made
for this change; the earlier LOD visual evidence predates this snapshot migration.

Presentation migration native acceptance passed on actual presented frames: snapshot camera distance and owner/material membership, near/far LOD, residency rejection retaining prior geometry, and budget recovery. See `docs/engine-research/mechanisms/godot-lod/native-snapshot-certification-2026-10-02.json` for binary/archive hashes and output. This strengthens native integration evidence; it does not establish image equivalence or complete architecture acceptance.

### Measured scene scaling checkpoint

Optimized CPU probes on Apple M4 Max / 128 GiB cover 1k/10k/100k balanced, wide
and chained scenes. Snapshot comparison now alternates execution order; both
paths reuse the same projection buffers. At 100k owners, legacy/scoped projection
medians were about 2.20 ms in the paired run and 1.41 ms in a second runtime-only
run. No access-check speedup is established. All-node sequential transform writes
measured 61.6/26.0 ms versus batch 5.10/4.09 ms in those respective runs. Peak
whole-process RSS was 136265728 bytes. Shared host load was uncontrolled.

Raw data, source/binary hashes, hardware, methodology and limits are recorded in
`docs/architecture-performance/2026-10-02-scene-scaling/report.json`. These are
isolated operations with u64 payloads and 21 samples, not whole-frame latency or
production tail/allocator acceptance. AngularMotion still issues per-owner writes
through Behavior hooks; migration to a bounded batch domain system is the next
measured intervention. Component churn, inactive mixes and complete tick/extraction
allocation/latency acceptance remain open.

### Angular motion batch migration

Editor Play captures bounded angular-motion descriptors in a separate domain owner,
not per-object Behavior attachments. Each fixed tick stages active surviving owners
and publishes their local rotations with one `set_locals` barrier before character
physics. Captured descriptors preserve legacy play semantics; deleted/reused handles
are skipped and inactive motion resumes when enabled. Stop discards runtime state.
The old explicit behavior adapter remains available but is not also bound in editor
Play. Invalid ticks/scenes and missing scene/domain writes reject before publication.

Validated plan composition preserves ordered barriers and resource grants, rejects
duplicate dispatch names and respects a total system bound. The editor composes
motion followed by character phases; typed physics failures remain intact.

Validation passed 19 gameplay library tests, 57 editor library tests (one unrelated
GPU font test ignored), and all three schedule tests. Editor authored-motion/fixed
loop/Stop tests pass on the new path. An optimized alternating-order benchmark on
Apple M4 Max measured 100k active rotating owners at 29.69 ms median for Behavior
versus 5.31 ms for the batch path. Root/middle/last world matrices are checked every
round; lifecycle/hierarchy/deletion equivalence has a separate regression test.
Raw measurements and hashes: `docs/architecture-performance/2026-10-02-angular-motion/report.json`.
This is one shared-host CPU domain probe, not total frame latency; editor admission
remains 128 authored motions and allocation/whole-frame acceptance remains open.


### Live motion lifecycle correction

The first batch implementation retained descriptors captured at startup. A lifecycle
regression exposed that runtime component removal did not stop rotation. The batch
now reconciles the detached play scene's live components before every fixed step,
using bounded reusable staging. Changes/additions/removals apply to simulation;
authoring history remains separate. Invalid descriptors, capacity overflow or new
physics-owned hierarchy conflicts reject before publishing any transform changes.
The earlier capture-only timing checkpoint is historical, not the current cost.

Live-path validation passed 21 gameplay library tests and 57 editor library tests
(one unrelated GPU text test ignored). Tests cover updated axis/rate, component
removal, admission after startup, overflow and new physics conflicts retaining poses,
and resource denial without duplicate ticks. Current optimized 100k-owner medians:
legacy 28.64 ms, reconciled batch 10.08 ms. Full data, production-source snapshot,
binary hash and limitations are in
`docs/architecture-performance/2026-10-02-angular-motion-live/report.json`.

### Model draw submission snapshot boundary

Frame styles now own bounded ModelPart values for active drawable owners. Model part selection and LOD eligibility consume this snapshot. Selection outlines and gizmo placement consume the extracted drawable/pose rather than querying live scene components, activity or world matrices. LOD history retirement uses current drawable membership. UI draw-owner checks and authoring panels remain separate live-scene consumers; the entire editor is not claimed scene-independent.

Validation: 57 editor library tests passed (one unrelated GPU text test ignored), including ModelPart snapshot independence after source changes. A freshly rebuilt native LOD smoke passed actual presented frames, snapshot consistency, near/far selection, last-good budget rejection and recovery. Hashes/output are stored in `docs/engine-research/mechanisms/godot-lod/native-parts-snapshot-certification-2026-10-02.json`. The native fixture exercises an OBJ model without hierarchy parts; hierarchical part behavior is covered by existing import/editor tests and the new snapshot test, not a new GPU pixel comparison.

### UI owner visibility snapshot

Frame extraction now captures bounded active UiElement owner identities. UI draw
submission uses this immutable membership instead of scene validity/activity
queries. Removing UiElement immediately excludes retained GPU UI geometry even
when the scene object remains alive; a reused slot has a new generational identity
and cannot authorize an old draw. Disabled input elements remain visible, while
inactive ancestor subtrees do not. Regression checks snapshot independence,
capacity, parent inactivity, component removal and handle reuse. Render trace
positions also use extracted model poses instead of live scene transforms.

UI preparation/picking and authoring remain intentional scene consumers before
submission. The native graphics submission block itself now consumes presentation
snapshots, GPU resource ownership and editor controls; this does not close broader
simulation access or complete architecture acceptance.
UI visibility validation passed: 58 editor library tests (one unrelated real-GPU
text test ignored), editor cargo check, package-boundary gate and diff checks.
No new GPU pixel/visual equivalence claim is made for this membership change.

### Shared UI publication input

The UI publisher now gives its already-extracted owned layout snapshot to the text
import plan. Stale-completion comparison, mesh preparation and font worker shaping
therefore share one layout publication instead of extracting layout a second time.
The import plan still observes fonts from all UiElement descriptors, including
inactive ones, preserving existing aggregate font/input quotas and dependencies.
The standalone UiTextPlan constructor delegates to the same snapshot preparation.

Validation passed 58 editor library tests (one unrelated real-GPU text test ignored),
including inactive-font accounting, font publication/reload and retained-input budget
checks. Package layering and diff checks passed. No speedup or new visual claim is
made. UI import observation and input dispatch remain pre-submission scene consumers.

### Native game UI pointer entrypoint

Play and standalone left-button window events now route through a persistent
SceneUiRuntime adapter instead of the editor's Play early return. Both press and
release revalidate current descriptors, activity, generation and viewport; invalid
layout cancels capture and reports an input diagnostic without terminating the
window. Focus loss, cursor exit and Play/Stop cancel activation. Pointer coordinates
use the same physical viewport dimensions as the UI publication path.

Validation: 60 editor library tests passed, one real-GPU text test ignored.
Regression tests exercise the actual App pointer_press Play path and Stop, plus
component removal, disabling, invalid layout and capture cancellation. This is
source/event-adapter and CPU evidence, not a new native mouse/visual demonstration.
Actions currently reach diagnostics only: application gameplay handlers, keyboard
focus routing and matching hit targets to asynchronous displayed UI publication
remain open. The UI feature and broader architecture are not complete.

### Native game UI keyboard entrypoint

Play/standalone keyboard routing now offers Tab/Shift-Tab traversal and combined
Enter/Space activation to the game UI before authoring-panel and gameplay bindings.
Unfocused activation keys pass through to gameplay. Held activation keys are tracked
as one mask: autorepeat cannot activate and only the final release emits an action.
Traversal cancels an in-flight target without resetting traversal position. Live
component reconciliation occurs on each key edge; removal and focus loss cancel
activation. Consumed releases still clear earlier gameplay bindings, preventing a
key held before acquiring UI focus from leaving gameplay input latched.

New regression coverage checks forward/reverse traversal, repeat, simultaneous
activation keys, focus changes during capture, removal, cancellation, and the App
Play keyboard entrypoint. Gameplay handler delivery, visible focus indication and
input/display publication coherence remain open; actions still reach diagnostics.
Validation: 61 editor library tests passed; one real-GPU text test ignored.
Package layering and diff checks passed. Evidence is CPU/event-adapter coverage;
no new native keyboard or GPU visual demonstration is claimed.

### UI input/display publication coherence

UiLive now retains a separately acknowledged presented snapshot. GPU upload alone
cannot advance it; only RenderOutcome::Presented does. Native pointer and keyboard
admission compare the current bounded UI extraction, including viewport and complete
descriptors, against that presented snapshot. During asynchronous layout changes,
failed preparation, or an unpresented upload, input capture is cancelled until the
matching layout is displayed. Switching admitted snapshots cancels a held target
even when no input event occurred between publications. Focus loss and Stop clear
admission. Headless adapter tests have no display and continue direct live routing.

This uses atomic whole-layout admission: an unrelated layout change temporarily
suspends all UI targets until presentation. It establishes correctness before any
per-element concurrency optimization. Regression coverage includes initial upload
without presentation, acknowledgement, retained previous layout, viewport change,
component removal, capture cancellation across publications and restored input.
Gameplay action handlers and visible focus styling remain open. CPU tests do not
prove native input/display timing or GPU pixel correctness.
Validation: 63 editor library tests passed; one real-GPU text test ignored.
Package boundary and diff checks passed.

### UI actions to application logic and scene barrier

The public gameplay UiActionHandlers registry accepts bounded named Rust callbacks
and bounded queued UiActionEvent values. Queue and dispatch are bound to SceneId;
current generation/activity/action/UiElement ancestry is revalidated before callback
execution. Duplicate registrations, full queues, unknown actions and expired owners
report errors. Callbacks read the scene and stage changes in existing SceneCommands.
Failed callbacks discard their staged scene commands; successful commands append
atomically to the simulation's existing frame queue. SceneCommands::append checks
scene identity and total capacity before moving anything. This admission guarantee
does not make command application transactional, or roll back captured callback
state on errors.

The native editor/standalone event path now enqueues pointer and keyboard actions
instead of only logging them. Dispatch occurs before SceneSimulation advance, whose
existing barrier applies commands even when elapsed time yields zero fixed ticks.
A built-in namespaced voxy.ui.hide handler sets the event owner inactive. Play creates
a fresh scene-bound registry; Stop drops it. Library clients can register application
closures through UiActionHandlers; exposing custom handler configuration through the
native launcher/editor inspector remains open, as do scripting and focus styling.
Concurrent native input integration now selects registered callbacks first and
otherwise retains the existing named InputMap activation path. The dispatcher
addition preserves that routing; callbacks are not registered for arbitrary input
names. Native custom-handler configuration and input pulse semantics should be
validated together before treating the gameplay UI integration as complete.
The combined editor regression caught named UI pulses remaining pressed in a scene
without physics owners. The character.step phase now consumes player.input edges
also in that case, after checking its write grant. Zero-tick frames retain pending
pulses. This fixes real input lifecycle independently of registered command handlers.
Validation after the input-lifecycle fix: 65 editor tests passed (one real-GPU text
check ignored), 23 gameplay tests and 57 scene tests passed. The App regression
routes voxy.ui.hide through pointer release, retains the active owner before the
barrier, hides it on a zero-tick advance and restores authoring on Stop. Handler
failure, capacity admission, foreign scene and expired owner tests passed. These
are CPU/event-path checks; no new native visual acceptance is claimed.

### Visible game UI focus

Keyboard/pointer focus now produces a bounded four-strip inward focus ring from the
admitted presented UI snapshot. The ring uses the visible clipped rectangle, handles
tiny controls, and cannot authorize a removed owner. An unpresented replacement
snapshot suppresses the old ring. Focus cancellation clears the visual target.
The ring draws after game UI using the existing overlay transform/white texture,
before authoring panels. GPU geometry is cached by owner/viewport/clip and included
in resident geometry accounting. Upload follows the existing staged peak-budget
path; rejected focus geometry is cleared rather than retained on another target,
and retried only when its key or budget state changes.

Regression coverage checks focus admission/cancellation/publication mismatch and
inward geometry bounds including one-pixel controls. Native visual acceptance is
still required; CPU geometry and window draw wiring alone do not prove appearance.
Validation: 67 editor tests passed, one real-GPU text test ignored; package layering
and diff checks passed. No fresh native screenshot or input demonstration yet.

### Native UI visual acceptance checkpoint

A freshly built lod_viewport binary was frozen into a dedicated Voxy UI Review
bundle and launched through the native computer-use API with two colored UI controls.
Screenshots were actually inspected: Tab first focused the blue control, window focus
loss removed its ring, two Tab events after raising focused the green control, and
Shift-Tab/Enter hid the blue control. Stop removed game UI and restored the authoring
gizmo; a subsequent Play restored both controls. A real pointer click also hid the
blue control. This confirms the native event/render/command path for that fixture.

Fixture, binary identity, observation scope and native log are archived in
`docs/native-ui/2026-10-02-review/report.json`. Screenshots are in the review tool
outputs, not archived image files. This is manual visual evidence, not automated
pixel comparison, text/font acceptance, budget-failure demonstration or packaged
standalone acceptance. The broader editor/gameplay/engine parity goal stays open.

### Observed UI font reload

Native UI publication now retains a SourceDependencies index and the existing
SourcePollWorker. The first completed observed attempt seeds the watcher; subsequent
imports reuse it, preserving fingerprints and avoiding repeated initial-invalidation
loops. A bounded background scan is requested every 200 ms. Known source changes
invalidate attempted layout before accepting worker completion, including when the
layout descriptor itself is unchanged. Source read/hash work stays off the owner
frame loop. Missing/corrupt input observations remain watched for recovery.

Source reads are recorded as a failed attempt until both text preparation and GPU
admission succeed. This preserves previous published dependencies and geometry on
failed replacements; only successful GPU publication replaces dependency ownership.
Budget retry and file-change retry remain separate triggers. Close retires import
and watch workers through the existing join/reap ownership path. The shared project
source-watcher constructor retains the audio watcher's previous capacities.

A regression drives actual background content polling through modification, missing
file and restoration, asserting invalidation while published/presented layout remains
intact. Existing font decode/Unicode/invalid-font and publication tests provide CPU
preparation coverage. A new native font-reload visual demonstration remains open;
the prior two-color native review does not verify this change.
Validation: 68 editor library tests passed, one real-GPU text check ignored;
package boundaries and diff checks passed.

### Native text baseline and interrupted reload review

A fresh binary rendered two Arial text labels (Reload: Hello 123) on the native UI
fixture after Play and raising/zooming the window. The subsequent corrupt-font
experiment was interrupted by a restarted native window, so no retained-publication
comparison or recovery acceptance can be claimed. The fixture font was restored.
Binary/font identities and the limited evidence are in
`docs/native-ui/2026-10-02-font-review/report.json`. Native reload acceptance remains
open; the background polling/CPU import regressions remain the proven scope.

### Production UI publication on an offscreen GPU

UiLive's existing native wrapper now delegates its publication/upload path to a
surface-independent poll_gpu method; it still uses the same renderer, import worker,
source watcher, dependency admission, geometry/texture budget and draw publication.
No native event or surface-present logic was replaced. An explicit real-adapter test
uses this production path: Arial publishes, corrupted font bytes retain the original
GPU atlas/presented layout, Courier replaces it, and Arial restoration reuses the
original resident atlas. GPU validation scope completes without errors.

Both normally ignored GPU checks were explicitly run successfully. The glyph pixel
check counted 105 covered, 54 partial-alpha and 407 transparent pixels while checking
clip/background preservation. The reload check verifies resources and publication,
not full-frame pixel equivalence or native surface timing. Default editor validation
passed 68 tests with those two GPU checks skipped in that run. Commands/results and
scope are archived in `docs/native-ui/2026-10-02-gpu-reload/report.json`.
Native visual reload acceptance still remains open; this offscreen result does not
retroactively complete the interrupted native-window experiment.

### Application UI callbacks through native launchers

The public UiActionSetup factory accepts captured Rust configuration and registers
fresh callback instances into each Play's scene-bound UiActionHandlers. Native
run_model_viewport_with_ui_actions and run_packaged_game_with_ui_actions route setup
through the existing editor/standalone loader rather than building a second loop.
The ordinary entrypoints retain their default setup. Packages declare action names;
compiled application code supplies callbacks. The example ui_callbacks binds app.hide
using the existing command barrier.

Setup occurs before committing Play ownership or starting simulation. Registration
failure discards the candidate registry and leaves Play/simulation inactive; the
serialized authoring document remains unchanged. Callback state is recreated each
session, while explicitly captured setup configuration persists. This is Rust-host
integration, not an inspector scripting editor or arbitrary package executable loader.
Regression coverage exercises captured setup, fresh per-session counter state,
command dispatch, Stop restoration and a duplicate built-in registration rejecting
Play without leaving a session behind.
Validation: 69 editor tests passed (two environment-dependent GPU tests skipped in
this run), public ui_callbacks example cargo check passed, and the updated package
launch path's traversal/nested-bootstrap rejection regression passed. Boundaries and
diff checks passed. No new native custom-callback or packaged-game launch acceptance
is claimed; the Play lifecycle/dispatch regression is headless App coverage.

UI dispatcher ownership admission now checks both the read scene and destination
SceneCommands identity before draining events or invoking callbacks. Previously a
foreign destination failed only during append, after captured callback state could
already change. The regression verifies zero callback invocations on rejection,
preservation of the pending event and exactly one invocation after retrying with
the correct queue. Validation: 24 gameplay library tests passed; package boundaries
and diff checks passed. This does not roll back callback state for application errors
or capacity failures, whose documented semantics remain unchanged.
