# Stride3D / Unity / Godot capability coverage

Goal: cover the capabilities of Stride3D, Unity3D and Godot in Voxy with an independently
designed architecture, without inheriting poor architectural decisions. Coverage means a usable
public API, integration in the engine, a reproducible example, and verification.
A module name or a cross-target compilation alone does not establish coverage.
This initial category inventory is not an exhaustive feature-by-feature audit.

Reference catalogs (reviewed 2026-10-01):
- https://doc.stride3d.net/latest/en/manual/index.html
- https://docs.godotengine.org/en/stable/about/list_of_features.html
- https://docs.unity.com/en-us/engine/6000.6/manual/get-started/first-time-user/key-concepts
- https://docs.unity.com/en-us/engine/6000.6/manual/working-with-gameobjects/game-objects

| Area | Current evidence in Voxy | Remaining work |
| --- | --- | --- |
| Scene objects | `voxy_scene`: generational handles, hierarchy, cached transforms/activity, versioned scene documents and bounded file IO | General scene ownership/lifecycle integration and schema migration |
| Components | Typed node components and requirement joins; component revisions; transactional ComponentTable; revision-validated AssociatedData; BehaviorRunner and serial SchedulePlan | General render/physics adapters, additional tracked dependencies, typed access enforcement and safe parallel dispatch |
| Prefabs / reusable scenes | In-memory cloned hierarchies; persistent scene codecs validate registered object references | Persistent prefab assets, overrides and reference remapping across instances |
| Assets | `voxy_assets`: stable IDs, immutable versions, load tickets, dependency stamps and last-good reload publication | Importer integration, residency/unload policy, payload budgets and glTF pipeline |
| 3D rendering | Mesh, texture, camera, depth, overlay and rendering work recorded in engine-development.md | Audit PBR, lights/shadows, transparency, LOD and postprocessing separately |
| 2D | Sprite/overlay renderer modules | Scene components, tilemaps, dedicated 2D physics, sorting and pixel conventions |
| Animation | `voxy_animation`, skeletal renderer | Audit clips, blending, animation graph, property tracks and event delivery |
| Physics | Kinematics, planar contacts, voxel adapter and specialized simulation modules | General rigid bodies, colliders, joints, triggers, collision layers and scene integration audit |
| Input | `voxy_input`: named actions, bindings, edges and analog dead zones; native scene adapter | Gamepad/device integration, rebinding UI and complete focus acceptance |
| UI | `voxy_ui`: pointer capture/focus, layout and scrolling; `voxy_text`: shaping/glyph atlas; native/GPU examples | Full widgets, paragraph layout/bidi/fallback, text editing, accessibility and editor integration |
| Audio | `voxy_audio`: voices/buses/gain ramps, PCM WAV/streaming/resampling and basic spatial pan; CPAL output adapter | Compressed decoding, scene lifecycle integration, device reconnect/formats and measured callback guarantees |
| Navigation | Bounded grid routing and incremental BFS with revision cancellation; scene agent example | Navmesh, footprints, avoidance, off-mesh links and rendered integration |
| Gameplay scripting | Rust BehaviorRunner lifecycle and bounded event channels | Serialized behavior registry, external scripting decision and debugging tools |
| Networking | No multiplayer crate in workspace | Transport, authority, replication, interpolation and reconnect tests |
| Editor | Native bounded OBJ/glTF/GLB editor: perspective/orthographic camera, exact triangle picking, projected gizmos, editable imported hierarchy, material/physics inspector, persistence and isolated Play/Stop | General project/gameplay adapters, bounds framing, lighting widgets, full widgets/accessibility, additional object/component types and compact history |
| Platforms / XR | Desktop, web, mobile, XR and GPU backend crates | Preserve existing hardware/device acceptance gates; see engine-development.md |
| Production tooling | Cargo build/test workflows | Profiler, diagnostics, export packaging, asset builds and CI platform matrix |

Execution order:
1. Scene/component lifecycle, reusable scene assets, serialization and integrated example.
2. Shared input actions, UI/text and audio for a small playable game.
3. Physics scene components, animation control and navigation.
4. Asset pipeline and editor with undo/play mode.
5. Rendering/2D gaps, multiplayer and remaining platform acceptance.

First increment: typed components attach to existing SceneGraph nodes. One value
per Rust type; replacement returns the old value. Foreign/stale handles are rejected;
subtree deletion drops owned components; reused slots cannot recover old data.
Components require Send + Sync and queries visit live nodes in slot order.
Transform remains the built-in hierarchy state: an attached Transform value does
not replace the node's local transform. This API is not yet an ECS scheduler,
serialized component registry, or an application behavior lifecycle.

Validation: `cargo test -p voxy_scene` (7 tests),
`cargo clippy -p voxy_scene --all-targets -- -D warnings`, `cargo fmt -p voxy_scene`.
Full workspace verification is pending. The overall goal remains open.

Second increment: `Prefab` / `PrefabNode` instantiate independent reusable
hierarchies under an optional existing parent. Topology, local transforms and
capacity are checked before mutation. Component prototypes require Clone; this
is an in-memory template, not yet an imported or serialized scene asset. Internal
NodeId references are copied as ordinary values and are not remapped. User Clone
and Drop panics are outside the transactional error guarantee.

Verified: 10 scene tests, all-target strict scene Clippy and
`cargo run -p voxy_scene --example prefab`. The example exercises independent
health values, inherited label transforms and subtree cleanup without a GPU.
Next: persistent scene format and registered component codecs with reference
remapping; then lifecycle and application scene integration.

Third increment: node names and local/effective activity, with explicit
`active_components` queries. Disabling a parent preserves children's local flags;
reparenting immediately changes effective activity. Inactive objects retain their
transforms/components and remain accessible to ordinary queries. Prefabs preserve
names and local activation. Names need not be unique; lookup returns all matches.
This does not yet suppress rendering or physics in applications automatically;
those consumers must use effective activity when integrated.

Verified: 13 scene tests, strict all-target Clippy, updated prefab gameplay example.

Fourth increment: `Behavior` and `BehaviorRunner` provide awake, enable, start,
frame/fixed updates, disable and destroy in attachment order. Awake executes at
attachment even when inactive; start precedes the first update of either kind and
runs once. Effective hierarchy activity gates update dispatch. Removed owners
receive cleanup at sync; stale handles cannot be updated. Behaviors can edit the
scene from hooks. Changes are observed at dispatch boundaries, and an explicit
clear dispatches cleanup before the runner is discarded. Drop alone does not
invoke lifecycle hooks. User panics are not recovered. The scheduler is intended
for its original scene and is not a serialized behavior or scripting registry.

Verified: 16 scene tests, strict all-target scene Clippy, formatting, and prefab
example with fixed-step health regeneration, disable/resume and deletion cleanup.
Application event-loop integration, persistent scenes and script tooling remain.

Architecture direction: [ADR 0002](adr/0002-general-engine-architecture.md)
separates authoring, simulation and presentation. Existing HashMap components,
mutable callbacks and the hardwired demo scene are bootstrap adapters, not the
final storage/scheduler/extraction architecture. Follow the ADR migration order;
feature catalogs do not prescribe the internal design.

Native integration increment: SceneApp dispatches behaviors after base animation,
fixed ticks at 60 Hz, activity synchronization while paused and shutdown cleanup.
World draws obey inherited activity; overlays remain independent. V toggles the
world subtree. `cargo run -p voxy_app --example lifecycle_scene -- --smoke` passed
on Apple M4 Max / Metal with 120 presented frames, enable=2, disable=2, start=1,
updates=104, destroy=1. This verifies lifecycle dispatch and presentation, not
pixel-level visibility or a general extracted rendering world. Primary-object
removal remains unsupported by this demo. Strict app Clippy was attempted but
failed on 91 pre-existing dependency diagnostics in physics; it did not pass.

A subsequent `--no-deps` strict app Clippy run also failed: 120 diagnostics in the
current application crate, including existing demo/style issues. These failures
remain an open verification gate; successful native smoke does not replace it.

Scene persistence increment: version-1 JSON documents use durable ObjectId strings,
named component schemas and strict top-level/object fields. Registry codecs load
and capture typed values. Registered reference visitors reject missing component
references on load/save; runtime handles are resolved separately through LoadedScene.
Parent order is unrestricted; duplicate IDs, missing parents, cycles, invalid
transforms, capacity overflow and unknown/malformed schemas fail loading. A private
staging graph leaves the existing world untouched on failure. Capture rejects
unregistered data and unidentified spawned objects instead of silently dropping it.

Verified: 20 scene tests and file roundtrip example
`cargo run -p voxy_scene --example scene_document` produces
`target/game-roundtrip.scene.json` and restores edited health with fresh runtime
handles. Prefab overrides, schema migration execution, arbitrary external asset
references, input byte budgets and durable atomic filesystem writes remain pending.
Only explicitly registered reference visitors can validate references inside custom
component data. This format is an authoring boundary, not the final simulation store.

Simulation storage increment: ComponentTable<T> owns contiguous typed values,
dense owner handles and sparse generation-aware indices independently of node
metadata. Batched mutable queries borrow the graph immutably, preventing structural
changes during processing; deleted handles are skipped and never expose old values.
Deletion cleanup occurs at synchronize barriers; swap removal may change row order.
SceneCommands provides a bounded insertion-order structural queue, per-command
results and explicit apply boundaries. Batches are not all-or-nothing transactions.

The batch_gameplay example migrates health from authoring node components to the
typed table, updates it in a batch, gates updates on hierarchy activity, explicitly
extracts state for save/load and deletes objects through a command barrier.
Verified: 23 scene tests, strict all-target Clippy and
`cargo run -p voxy_scene --example batch_gameplay`.
Declared system access/phase scheduling, general extraction adapters, cached
hierarchy state and benchmark measurements remain pending. No throughput claim
is made for this storage increment.

Hierarchy cache increment: world matrices and effective activity live in node
metadata. Spawn composes from cached parent state; transform/activity changes and
reparenting iteratively refresh only the affected subtree. Queries now read cached
values after handle validation. Equal-value writes skip propagation. Overflow is
cached as an error and recovers after valid ancestor edits. Fresh slots initialize
fresh cache state. Matrix composition uses parent-world times local, so floating
point grouping differs from the former reverse ancestor walk.

Verified: 24 scene tests, strict all-target scene Clippy, batch_gameplay acceptance
and real lifecycle_scene Metal smoke (120 frames, start=1, enable=2, disable=2,
destroy=1). A mixed hierarchy test compares cached values to full recomputation
across edits, reparent and handle reuse. Existing deep hierarchy and overflow tests
also pass. Current invalidation is eager; command-barrier coalescing and measured
1k/10k/100k scene benchmarks remain pending. There is no measured speedup claim.

Input increment: independent voxy_input crate defines named actions, physical
control IDs, bounded configuration and normalized weighted digital/analog bindings.
Press/release edges accumulate until finish_frame, including complete taps between
frames. Multiple bindings do not release an action while another holds it.
Rebinding observes existing held controls; invalid bindings preserve configuration.
Focus loss and device disconnect clear the appropriate physical controls.
Unbound controls are ignored to keep tracked state bounded by configuration.
Batch gameplay now uses pause_world action to gate inherited activity.

Verified: three input tests and strict all-target input/scene Clippy. Native
keyboard/mouse/gamepad adapters, dead zones, modifiers, configurable persistence
and integration with the actual window input path remain pending.

Native input integration: SceneInput is a thin winit keyboard adapter for named
pause/toggle_world actions. SceneApp consumes action transitions, replacing its
Space/V key-specific switches. with_action_keys configures bindings; focus and
platform suspension clear held controls. Physical codes are runtime-only enum
codes, not persistent configuration identifiers. Tests cover remapping, repeated
held events, release/repress and focus reset. Device/gamepad, mouse axes and
persisted key-name configurations remain pending. Keyboard semantics are unit
verified; automated smoke does not prove human keyboard delivery by the OS.

Analog processing increment: per-action symmetric scalar dead zones reject low
magnitude drift and rescale the remaining interval to [-1,1]. Threshold updates
recompute current values and transitions; rebinding preserves calibration.
Invalid thresholds leave calibration unchanged. This is scalar action processing,
not radial two-axis stick calibration or a hardware gamepad adapter.
Verified: four voxy_input tests and strict all-target Clippy.

Scheduling increment: SchedulePlan validates bounded system lists, names, resource
access declarations, dependency references and phase order before execution.
Topological batches separate declared read/write conflicts; read/read systems can
share a batch. Cycles and later-phase dependencies are rejected. The executor
runs serially in deterministic batch order and stops at the first system failure.
Previously executed effects are not rolled back. Batch gameplay now uses the plan
for regeneration followed by validation.

Verified: 26 scene tests, strict all-target scene Clippy and batch_gameplay run.
Declarations describe intended access; callbacks are not yet restricted to scoped
resources. Parallel execution is deliberately unimplemented until access is
actually enforced. This is a validated serial phase executor and planning layer,
not proof of a safe parallel ECS scheduler. Worker execution, command-barrier
integration and benchmark measurements remain pending.

Events increment: EventChannel<T> bounds retained event count, assigns channel-local
sequence numbers and supports independent consumer cursors. Emission returns an
evicted payload when full; reads report missed events instead of silently hiding
lag. Replay starts at oldest retained data; clear retains sequence continuity.
Foreign cursors and exhausted sequence space are rejected. Reads borrow payloads
and prevent simultaneous mutation, without invoking gameplay callbacks reentrantly.
The event_gameplay example applies damage through one reader while an independent
HUD-data reader observes the same events once.

Verified: 28 scene tests, strict all-target Clippy and event_gameplay acceptance.
This is an in-process single-writer event buffer, not a network/reliable delivery
protocol. Capacity bounds event count, not heap bytes owned by arbitrary payloads.
Read batches allocate a bounded reference vector. Actual rendered HUD, script
signals, threaded producer queues and resource-scoped scheduling remain pending.

Navigation increment: independent voxy_navigation provides bounded four-connected
uniform-cost shortest paths, mutable obstacles and route revalidation. Breadth-first
search has deterministic neighbor order, grid-cell allocation limits and an explicit
expansion budget. Invalid/blocked endpoints, proven unreachable goals and exhausted
budgets are separate errors. GridPath revisions are local metadata, not global IDs.
Scene navigation_gameplay follows a route, replans around an inserted obstacle and
updates the agent transform until it reaches the goal.

Verified: three navigation tests (including all 1,764 endpoint pairs on an empty
7x6 grid against Manhattan distance), strict navigation/scene all-target Clippy,
and navigation_gameplay acceptance. This does not cover 3D navmesh, variable cost,
agent radius, local avoidance, off-mesh links or incremental asynchronous search.
Scratch is bounded by grid size but allocated per search; budgets bound expansions,
not wall-clock time. Agent movement in the example is discrete/headless.

Incremental navigation: GridSearch keeps its frontier and parent scratch between
advance calls, limiting expansions per call. Pending differs from terminal
unreachability; completed jobs reject further advances. Grid identity plus revision
rejects stale/foreign topology before continuing; dropping a job cancels it.
One-shot find_path now uses this same algorithm. The scene agent example budgets
four expansions per tick and restarts only after a confirmed stale job.

Verified: five navigation tests, strict navigation/scene Clippy and the updated
agent example. Sliced routes match one-shot routes and tests check per-call expansion
limits. Creation still allocates grid-sized scratch; final path reconstruction is
linear in route length and is not separately budgeted. These are expansion limits,
not a guarantee on per-tick milliseconds; 3D navigation and agent footprints remain.

Asset publication increment: independent voxy_assets catalogs stable AssetId values,
request revisions and immutable Arc snapshots. Owner-thread completion rejects
foreign/superseded/removed/repeated tickets. Failed reloads expose a failure state
while retaining the last good version. Removal does not invalidate snapshots already
held by consumers. Entry and current-pending limits are checked before mutation.
Superseded external workers still need caller cancellation/accounting; catalog
limits do not bound their count or payload heap bytes.

Verified: two asset lifecycle tests and strict asset all-target Clippy. The existing
OBJ importer is exercised by `cargo run -p voxy_render --example asset_reload`,
using background decode and explicit owner publication, stale results, failed reload
and retry. This does not yet implement a filesystem watcher, dependency graph,
content-hash cache, persistent resource manifest or automatic GPU replacement.

Asset dependency increment: AssetDependencies declares stable IDs with bounded
asset/edge counts, validates dependency edits before mutation and rejects direct
or indirect cycles. affected returns changed assets and transitive dependents once,
ordered dependencies-first with deterministic lexical tie breaking. A diamond
graph does not duplicate rebuilds. The OBJ reload example now computes a source
change rebuild plan and dispatches the model import through the existing catalog.

Verified: five asset tests, strict asset Clippy and background OBJ acceptance.
The graph computes plans; callers still own dispatch, worker cancellation and
failure propagation. Dependency-version stamps in import tickets, filesystem
watching, graph persistence and GPU replacement remain pending. Current graph
queries scan bounded metadata; no large-catalog performance claim is made.

Dependency-version publication: request_with_dependencies captures ready revisions
from the same catalog. prepare_import captures immutable dependency snapshots with
the ticket on the owner thread. Completion rejects changed, removed or non-ready
dependencies, releases the current pending slot and preserves the last good output
while marking that load failed. Invalid dependency requests do not supersede an
existing load. Unchanged old snapshots remain valid worker inputs, but their results
cannot publish after their dependency revisions change.

Verified: seven asset tests, strict asset Clippy and OBJ reload regression example.
Cross-catalog/heterogeneous imports, transitive dependency stamp capture and automatic
invalidation of already published outputs remain pending. The dependency graph's
rebuild plan must still be dispatched by its owner; publication validation is not
a filesystem watcher or automatic rebuild scheduler.

Authoring history increment: SceneHistory validates proposed versioned documents
before commit, supports undo/redo and branching and bounds retained versions plus
serialized document bytes. Invalid edits preserve current/undo/redo state; no-op
commits preserve redo. Oldest undo snapshots are evicted under configured limits.
Play simulation and GPU resources are not members of the history and are not
rewound by authoring edits. Restore uses durable IDs to load a fresh runtime scene.

Verified: 30 scene tests, strict all-target scene Clippy and authoring_history
example, which edits/saves a scene, undoes/redoes it and keeps separate play health
unchanged. This is snapshot-based authoring history, not yet a visual editor,
property inspector, gizmos or compact command-delta history. Byte limits measure
serialized representation, not total allocator overhead. Full-document cloning,
serialization and staging validation need large-scene measurement/optimization.

Scene file increment: bounded UTF-8 document reads and validated save-through-sibling
file with create_new, write/flush and atomic rename. Unix additionally syncs the
parent directory; a post-rename durability error explicitly reports committed=true.
Pre-publication failure cleans up the private temporary file and preserves the old
destination. Authoring_history now uses this save/load API instead of direct write.

Verified on macOS: 31 scene tests, strict scene Clippy and authoring_history file
roundtrip. Tests cover replacement, invalid document/quota preservation, bounded
reads and temporary-file cleanup. Power-loss testing, Windows directory durability,
permission preservation, locking/conflict detection for multiple editor writers
and recovery journals remain pending. IO assumes local filesystem rename semantics;
no cross-platform/network-filesystem crash guarantee is claimed.

Editor selection increment: SceneGraph bounds picking transforms normalized world
rays into local coordinates without renormalizing local directions, preserving world
hit distances under scale/rotation. It filters effective activity and layer masks,
returns nearest hits with deterministic ties and reports skipped singular objects.
LoadedScene.identity maps a live hit back to durable authoring IDs. The
picking_authoring example selects a scene object, edits its name and undoes the edit.

Verified: full 33-test scene run plus three focused picking tests after adding
rotated/reflected nearest-hit coverage, strict all-target Clippy and the authoring
selection example. This is headless bounds selection, not yet mouse/camera
unprojection, triangle picking, spatial acceleration, gizmos or visual editor UI.
Selection uses node-level bootstrap components; later frame extraction/spatial
index integration remains part of the architecture migration.

Camera selection increment: PickRay.from_viewport unprojects relative top-left
viewport coordinates with 0..1 clip depth. It supports finite perspective and
orthographic matrices without importing renderer/window ownership into the scene
module. The ray begins at the near plane and ends at the far plane; distance is
relative to that near-plane origin. Degenerate viewports, out-of-range cursors and
singular/nonfinite/infinite-far transforms are rejected.

Verified: four picking tests, strict scene Clippy and the renderer's camera_picking
example using actual SceneCamera perspective/orthographic matrices for center hits
and corner misses. This checks camera integration without GPU submission or native
mouse event delivery. DPI/viewport-offset mapping, editor clicks and gizmos remain.

Viewport routing increment: PickViewport converts physical window coordinates into
logical viewport coordinates with a supplied DPI scale and viewport offset. Outside
clicks return no ray; right/bottom borders are exclusive so adjacent editor panels
do not both receive a boundary click. Scale/rectangle/coordinate validation rejects
invalid inputs. The actual camera example checks a Retina-scale offset viewport.

Verified: five picking tests (1x/1.5x/2x DPI and boundary routing), strict scene
Clippy and perspective/orthographic camera example. Native cursor event integration
and visible selection remain pending; these tests verify coordinate mapping only.

Measured architecture probe: `cargo bench -p voxy_scene --bench scene_scaling`
now runs optimized 1k/10k/100k binary-tree scenes, 3 warmups and 21 samples per
workload. Raw results: scene-scaling-2026-10-01.csv and
scene-scaling-2026-10-01-repeat.csv. Hardware: Apple M4 Max, 128 GiB RAM,
macOS host, current Cargo bench profile. Numbers are isolated CPU timings, not
whole-game frame times; percentiles from 21 samples are coarse descriptive values.

At 100k objects, all-node eager transform edits took median 29.064 ms initially
and 28.793 ms on repeat; root edits took 1.377/1.424 ms and 1-percent leaf edits
0.035/0.036 ms. Cached all-object reads took 0.443/0.431 ms. This establishes a
large all-node-edit cost requiring coalesced transform propagation.

Typed table versus node HashMap updates varied: first medians 1.201/1.075 ms,
repeat 0.468/0.892 ms respectively. Do not claim a robust relative speedup or name
the responsible cache mechanism from these two runs. Both paths update u64 values,
check effective activity and use black_box; layouts differ and no allocations or
hardware counters were measured. Strict benchmark Clippy passes.
Next architecture step: command-barrier dirty-subtree coalescing with correctness
oracles and matched before/after benchmarks; additional deep/wide/churn/memory
workloads remain required by ADR 0002.

Transform coalescing increment: set_locals validates an entire transform batch
before mutation, applies last-write-wins duplicates and refreshes only maximal dirty
subtrees. Slot-indexed scratch replaces hashing per node; scratch allocation is
proportional to total scene slots, so sparse batching has an allocation tradeoff.
SceneCommands coalesces contiguous valid transform writes while preserving each
command's error; structural/activation operations flush these groups before running.
Reparent/removal ordering remains explicit. The batch API is atomic on input errors;
the command queue remains individually validated rather than globally transactional.

Verified: 38 scene tests, strict all-target Clippy and optimized benchmark.
Raw matched-run results: scene-scaling-batched-2026-10-01.csv. At 100k objects on
Apple M4 Max, eager all-node edits measured p50 28.823 ms / p95 31.368 ms;
batched edits measured p50 5.553 ms / p95 7.479 ms. Both update the same binary-tree
transforms; batch timing includes scratch and input-vector updates. This is an
isolated workload result, not a universal frame-speed claim. Deep sparse batches,
reusable scratch, memory allocation profiling and activation coalescing remain.

### Deep and wide hierarchy batch probe

Added chain and star hierarchies at 1k/10k/100k nodes to the optimized
`scene_scaling` benchmark. Samples include a single leaf batch and all-node
batch; eager all-node edits on the chain are deliberately excluded because
that operation revisits descendants quadratically. Raw before/after data:
`scene-scaling-hierarchy-before-2026-10-01.csv` and
`scene-scaling-hierarchy-after-2026-10-01.csv`.

A single-element `set_locals` now delegates to `set_local`, preserving input
validation and descendant refresh while avoiding scene-sized scratch buffers
and ancestor traversal. Multi-element batches still allocate slot-sized scratch
and may walk long ancestor chains for separated sparse edits. These probes are
isolated CPU operations, not renderer/game-frame or allocation measurements.
Verified with 39 scene tests and strict all-targets Clippy.

### Audio core foundation

`voxy_audio` provides a dependency-free owner-thread stereo PCM mixer with
immutable shared clips, explicit matching sample rates, fixed voice/bus counts,
bus gain, pause/resume, looping, completion reclamation and non-reusable
mixer-specific voice identifiers. Caller-owned output is overwritten; rendering
allocates no buffers. Muting advances playback, while pausing preserves cursors.
Input samples and gains are validated, and summed output is hard-clamped.
Three tests cover block partition equivalence, loop boundaries, pause/resume,
completion, capacity, foreign/stale identifiers, bus mute and saturation.

This is a foundation rather than audible playback: device adapters, decoding,
streaming, resampling, spatial attenuation/panning, effects, gain ramps and
cross-thread bounded command transport remain missing. Clip payload memory is
not globally budgeted. Dropping the last clip reference may free memory during
render, so this owner-thread API does not claim realtime callback safety.
Validation: `cargo test -p voxy_audio` and strict all-targets Clippy.

Audio gain automation now supports linear bus and voice ramps in output frames.
Zero-frame changes are immediate; retargeting begins at the current gain. Bus
ramps advance even when silent, while paused voices freeze their own ramps.
Empty render blocks advance neither clock. The first emitted frame takes the
first ramp step, with an exact target on its final step. Rendering now sums in
f64 before normalized f32 clipping and shares one bus clock across all voices.
Five audio tests include ramp block-partition equivalence and pause/retarget
semantics. Device playback and spatial audio remain open; per-frame voice scans
also need performance measurement before a realtime callback integration.

Spatial audio now has a pure geometry function plus independently ramped voice
channel gains. For a mono source duplicated into stereo, it computes normalized
equal-power pan from the listener right vector and linear near/far attenuation.
Calculations use f64 differences/norms to avoid overflow from finite f32 world
coordinates. Zero distance centers the source; invalid ranges/orientation and
nonfinite geometry are rejected. Both channel gains validate before mutation.
Tests cover rotation, distance, silence outside range, centered energy and actual
mixer channel routing. This is stereo positional playback math, not HRTF,
front/back localization, occlusion, Doppler or audible device verification.
Scene extraction and device adapters remain separate unfinished integrations.

Audio assets can now import bounded in-memory RIFF/WAVE PCM16 mono/stereo via
`decode_wav`. Unknown chunks and odd-byte padding are handled; inconsistent RIFF
length, missing/duplicate format or data, invalid rates/alignment, truncation and
unsupported encodings fail explicitly. Mono expands to dual-mono for spatial
routing. The decoded frame limit is checked before PCM allocation. Tests cover
signed sample extremes, mono/stereo layout, an odd ancillary chunk, all truncated
prefixes of a valid file, malformed headers and frame-cap refusal. This does not
bound caller-owned file bytes, decode compressed formats, stream or resample.

`cargo run -p voxy_audio --example asset_playback` integrates background WAV
import, owner-thread `AssetCatalog<Clip>` publication and the PCM mixer. A failed
reload preserves the last good clip; a superseded result is rejected. Existing
voices retain their immutable decoded samples across replacement, while a newly
started voice uses the new version. Assertions compare actual rendered samples
before and after switching voices. The generated one-frame WAV fixture is fully
decoded by the production importer. This is offline integration proof, not a
filesystem watcher, streaming service or audio-device run.

`PcmStream` adds a preallocated bounded stereo PCM FIFO for single-owner adapter
staging. Complete input blocks are validated and accepted atomically; overflow
returns capacity refusal so producers can retain and retry their blocks. Reads
preserve sample order, overwrite unavailable output with silence, and report
supplied/missing frames plus saturating lifetime underrun telemetry. Clear drops
queued frames for seeking without erasing telemetry. Tests cover wraparound,
backpressure, underrun, wrong rates, invalid samples and clear semantics.
No cross-thread synchronization, streaming codec, resampling or audio-device
integration is implied. Capacity bounds queued frames, not external producers.

`cargo run -p voxy_audio --example scene_playback` demonstrates immutable source
extraction from the scene hierarchy into a separately owned mixer. Parent motion
moves the source from left to right, while an older extracted value still plays
at its original position. Parent deactivation pauses the source cursor; resume
continues it. Subtree deletion stops the voice and returns capacity, and a reused
scene slot does not revive the removed handle. Actual output samples are checked.
This example chooses pause-on-inactive and stop-on-delete policies; it is not yet
an engine-wide audio source registry, listener component, autonomous lifecycle
system, device output or threaded snapshot transport. Scene dependencies are
confined to audio example/dev dependencies, not the audio core library.

Native output now lives in separate `voxy_audio_device` using CPAL 0.15.3
(local documented API). It opens the default stereo f32 configuration, exposes
the actual device sample rate, keeps stream lifetime explicit and accepts
normalized stereo frames through a bounded nonblocking producer interface.
Full/closed queues and invalid PCM are reported separately. The callback drains
frames or writes silence and reports supplied/missing frames and host-error
counts. Standard sync_channel internals do not provide hard realtime guarantees;
format negotiation, resampling, reconnect, platform matrix and mixer pumping
remain incomplete. This dependency is isolated from the pure audio core.

Native macOS smoke `cargo run -p voxy_audio_device --example device_smoke`
passed on the current default device: 24000 Hz, 1024 supplied silent frames,
3776 silence-underflow frames, zero stream errors. This proves live callback
execution and queue draining, not subjective audible quality or all platforms.
Strict all-targets Clippy passed. Submission test covers invalid/full/closed
paths without requiring hardware.

Offline `Clip::resample_linear` enables matching the device rate with a checked
output-frame cap and integer rational phase (avoiding cumulative phase drift).
Same-rate conversion shares PCM; other rates use linear interpolation, ceil
output duration and a held final endpoint. Tests cover known interpolated
samples, constant signals, identity storage and cap/zero-rate refusal. This is
explicitly not an anti-aliasing resampler: downsampling quality remains open.

`cargo run -p voxy_audio_device --example mixer_output` passed on the actual
macOS default output: generated quiet 440 Hz source at 48000 Hz, converted to
24000 Hz, gain-ramped through the mixer, all 2400 frames drained by CoreAudio,
zero host errors. Output contains verified nonzero PCM before submission; this
is native transfer proof rather than measured speaker fidelity. The example
prefills a bounded queue; continuous pumping, quality resampling and reconnect
remain unfinished.

`MixerPump` adds bounded nonblocking continuous feeding with one reusable render
block. A full output queue retains the unsent block suffix, so retries neither
skip samples nor rerender/advance the mixer twice. A zero-frame budget does not
advance the mixer. Owner changes take effect after already-rendered/queued audio;
rate pairing is still the caller's responsibility. A deterministic capacity-one
test checks the exact looping PCM sequence across repeated full-queue retries
and closed-queue handling. Two adapter tests and strict Clippy pass.

Native `continuous_output` passed at 24000 Hz: 2400 mixer frames transferred,
zero host errors, using 128-frame render blocks and a 512-frame queue (smaller
than the complete signal). The run exercises actual backpressure and bounded
retry until completion. This is continuous owner-thread pumping, not a dedicated
worker, hard realtime queue, underrun-free guarantee or device reconnection.

Offline filtered conversion now adds a 65-tap Hann-windowed sinc kernel with
cutoff at 90% of the smaller Nyquist frequency, normalized DC response and
extended endpoints. Frame and kernel-evaluation limits are checked before
allocation/work. Filter overshoot is explicitly clipped to normalized PCM.
Tests for 48->24 kHz cover constant levels, Nyquist alternating-sample alias
suppression and 1 kHz passband / 18 kHz stopband sine RMS (interior samples).
Fixed 65-tap support does not prove a universal attenuation specification across
extreme ratios, boundary transients or all frequencies. Conversion remains
offline, not a streaming resampler.

Native continuous output was rerun with filtered conversion: 2400 frames
received at 24000 Hz, zero host errors. Both native mixer examples now use the
filtered API with an explicit work budget; the cheap linear API remains named
and available for callers intentionally choosing it.

### UI pointer routing foundation

Independent `voxy_ui` routes one primary pointer over bounded painter-ordered
logical hit regions. Press captures the top enabled widget; release clicks only
if that captured widget is still topmost under the pointer. Pointer leave keeps
capture but prevents click; focus loss cancels. Disabled overlays occlude lower
widgets. Atomic region replacement rejects invalid geometry/duplicate IDs and
cancels removed/disabled captures. Held state prevents repeated down events from
capturing a replacement widget before release. Half-open edges avoid ambiguous
adjacent region boundaries. Three tests and strict all-targets Clippy pass.
This is routing logic, not rendered UI: layout, text, keyboard focus, accessibility,
multiple pointers, clipping and native event adapters remain missing. Caller must
supply stable distinct lifetime IDs; ID reuse during capture is not supported.

UI keyboard routing now has a separate `FocusRouter`: bounded enabled traversal
order, forward/reverse wrapping, explicit pointer-driven focus assignment and
logical activation press/release. Held repeats never generate extra presses.
Changing focus or removing the activation target cancels it; the held key cannot
activate the next widget until released. Window focus loss clears both states.
Invalid order/focus assignments leave previous state intact. Five UI tests and
strict all-targets Clippy pass. Enter/Space physical mapping, Tab-repeat policy,
focus scopes/modals, text entry, accessibility and native/rendered integration
remain unfinished. Adapters must combine physical keys into correct logical
edges and supply only enabled focusable widgets in traversal order.

UI geometry now has `layout_linear` for horizontal/vertical rows with padding,
gaps, fixed lengths and proportional flexible lengths. The output is the same
logical `HitRegion` geometry intended for both rendering and input, avoiding
separate handwritten click rectangles. Item caps, duplicate IDs, invalid weights
and insufficient space fail explicitly; there is no implicit shrinking/scrolling.
Tests cover axis transposition, weighted division, disabled metadata and overflow.
Nested trees, content sizing, text shaping, grid/wrap, clipping, scrolling and a
rendered/native UI remain missing.

Offline `responsive_menu` passes across widths 320/640/1000: layout-derived
regions drive pointer clicks and keyboard focus/traversal without separate
coordinates. Six UI tests and strict all-targets Clippy pass. It verifies logical
routing after resize, not visual rendering or native mouse/keyboard injection.

`Sprite::from_logical_rect` bridges logical top-left pixel rectangles to
clip-space overlay geometry with identity MVP. Viewport and rectangle validation
rejects invalid/nonfinite values. DPI stays outside this conversion: input/layout
use logical coordinates while the render target scales normalized geometry.
`ui_geometry` builds the actual production SpriteBatch mesh from `layout_linear`,
checks vertex bounds mapped back to logical pixel origins and verifies matching
pointer targets at widths 320/640. This is geometry extraction verification,
not yet GPU pixels or a native interactive UI. The render-to-UI dependency is
example-only; core rendering accepts generic rectangles without widget ownership.

`cargo run -p voxy_render --example ui_pixels` now verifies actual GPU output
from layout-derived SpriteBatch geometry. Three target configurations cover
64 logical pixels at 1x/2x scale and resized 128 logical pixels at 1x. Texture
readback asserts clear/background, red/green controls and a blue painter-overlay
at known logical positions. The same disabled overlay occludes pointer presses
on underlying controls. Wgpu validation scope reports no errors. The 2x image
was visually inspected and saved as `ui-pixels-2x-2026-10-01.png`.
This proves GPU geometry/coordinate consistency, not native interactive widgets,
text, accessibility, clipping or keyboard event integration.

`voxy_ui_winit` isolates native window event routing from UI core. It projects
physical cursor coordinates with retained window scale, reprojects after DPI
changes, maps primary mouse capture to pointer focus, combines held Enter/Space
into one activation edge, and routes Tab/Shift-Tab and focus loss cancellation.
Repeated physical key-down does not duplicate activation; releasing one of two
held activation keys does not prematurely click. Region reconciliation emits
cancellation dispatches. Tests cover chords, DPI reprojection and window focus
loss through the adapter. This is event adapter code, not yet a native rendered
interactive window or accessibility/IME/text integration.

`cargo run -p voxy_render --example ui_window` now provides a native rendered UI
demo with three responsive colored controls. Hover, pointer capture, keyboard
focus and toggled state change colors; pointer release or Enter/Space release
updates the control and window title. Tab/Shift-Tab traverse controls. Shared
layout feeds both rendering and winit routing. `--smoke` presented 60 frames on
the actual native surface and exited successfully; strict example Clippy passed.
This proves native startup/presentation, while human/OS pointer-key interaction
has not yet been exercised in this run. Controls lack text/accessibility and the
demo rebuilds CPU layout/mesh each redraw; it is not the final retained UI engine.

Native UI interaction was exercised through computer-use on the temporary
`VoxyUI.app` bundle of the actual `ui_window` example. Observed title transitions:
Enter after Tab changed the first toggle true->false; a real pointer click on the
second control changed its toggle true->false; dragging from the first control
and releasing in the inter-control gap left the title/state unchanged.
Shift-Tab followed by Space selected/activated the final control and changed the
last toggle true->false. The final screenshot showed all controls untoggled with
the final keyboard focus color and title `[false, false, false]`. The demo window
was closed after verification. This establishes real OS mouse/key integration
for those paths; resize interaction, text/accessibility and broader platforms
still need independent evidence.

UI clipping now intersects logical hit regions with panel bounds, rejects invalid
rectangles and removes empty/touching intersections. Identity/enable metadata is
retained for surviving regions. The pointer test proves hidden portions cannot
capture. `Sprite::cropped` separately preserves the textured subrectangle via UV
interpolation and rotated center adjustment rather than stretching the image.
Geometry example checks crop bounds/UVs. GPU `ui_pixels` now uses a clipped blue
overlay and asserts the original red/green pixels survive outside its left/right
clip edges at 1x, 2x and resized targets. Seven UI tests, geometry/GPU examples
pass. UI strict all-targets Clippy passes; render strict Clippy is blocked by
a float-array comparison in `scene.rs:1104`, outside this clipping change.
This is axis-aligned panel intersection;
arbitrary rotated masks, stencil clips and nested retained clip propagation
remain unfinished.

### Text raster foundation

Independent `voxy_text` wraps fontdue Unicode glyph rasterization with explicit
font-byte, raster-size and per-glyph pixel limits. Metrics are checked before
bitmap allocation; unsupported characters return MissingGlyph rather than a
silent fallback. Output includes alpha coverage, baseline bearing and advance,
while whitespace can have an empty bitmap. The local-font `glyphs` example checks
Latin/Cyrillic/whitespace coverage and cap refusal without distributing the
proprietary system font. Font parsing is byte-bounded but internal parser memory
is not globally accounted. This is glyph rasterization, not Unicode shaping:
ligatures, bidi, script shaping, fallback, multiline layout, atlas packing,
GPU text rendering and IME remain unfinished.

Text now has a fixed-size append-only `GlyphAtlas` with checked texture pixel
and glyph-entry budgets. Shelf placement adds one transparent pixel around each
bitmap, preserves published coordinates and refuses full/oversized/malformed
inserts without changing pixels or packing cursors. Empty whitespace consumes
no atlas entry. Synthetic tests check exact copied pixels, gutters, row wrap,
capacity, malformed input and failure stability. The real-font glyph example
now packs Latin/Cyrillic glyphs into one atlas. Three text tests, local-font
example and strict all-targets Clippy pass. Atlas key caching, eviction, multiple
pages, upload diffs and GPU text drawing remain unfinished.

`text_pixels` now uploads the alpha glyph atlas as RGBA coverage, draws Latin A,
Cyrillic Ж and descender g through the actual SpriteBatch/SceneRenderer, and reads
GPU output back. Baseline bearing and per-glyph atlas UV rectangles are applied.
More than 1000 bitmap sample locations per target match CPU coverage in RGB
(with one byte rounding tolerance) and opaque composited alpha at 1x/2x. Wgpu
validation scope is clean; strict example Clippy passes. The 2x output was
visually inspected and saved as `text-pixels-2x-2026-10-01.png`. Runtime font is
supplied externally rather than distributed. This proves glyph drawing/atlas
orientation and baseline, not shaping, paragraph layout or native labeled UI.

`ShapeFont` adds rustybuzz shaping of a single directional/script run with
explicit LTR/RTL or guessed direction. Output preserves glyph IDs, UTF-8 byte
clusters, advances and offsets; `RasterFont::rasterize_indexed` rasterizes those
IDs from the same font face. The local Arial example proves AV kerning,
precomposed/decomposed accent glyph equivalence, valid Cyrillic source clusters
and shaped glyph rasterization. Input byte caps apply before shaping; output
caps apply after the library returns, so internal shaper work/memory is not a
hard bounded task. Missing glyphs fail explicitly. Paragraph bidi/itemization,
language/features API, fallback, line breaking and shaped GPU string integration
remain unfinished; font faces are reparsed for each shaping call at present.

`TextFont` now owns matching shaper/rasterizer face-zero sources and prepares a
private staged `TextRun` containing immutable atlas, placed glyph rectangles,
UTF-8 clusters and run advance. Shaped offsets/advances and font bearings are
converted to top-left baseline coordinates; whitespace advances without drawing.
Preparation failure cannot mutate an already-held run. Tests cover baseline/Y
signs and the local-font example proves prior output survives an atlas-cap error.
Each run currently owns a fresh atlas and repeated glyphs use separate entries;
shared caching and resource lifecycle are not implemented yet.

GPU `shaped_text_pixels` renders the prepared `AV é Жg` run at 1x/2x, checks bright
coverage/background and clean wgpu validation, and saves a preview. The 2x image
was visually inspected and saved as `shaped-text-pixels-2x-2026-10-01.png`.
Strict example Clippy passes. This proves shaped-run integration, while exact
coverage comparisons remain in the separate integer-position glyph test.
Paragraph shaping, line breaks and native UI labels remain open.

Prepared text now deduplicates glyph rasterization and atlas placement within a
run by glyph ID. Because a run has one font face and raster size, this key is
unambiguous; each instance retains its own shaped offset, source cluster and pen
position. Whitespace metrics are cached too without consuming atlas regions.
The local-font shaping example proves `A A A A` yields four independently placed
instances sharing one atlas region in a 32x32 texture that fits only one raster.
This supersedes the earlier separate-entry limitation for repeated glyphs within
a run. Cross-run/shared font-size caching and eviction remain unfinished.

Native `ui_window` optionally accepts a runtime font path and now renders shaped
bilingual labels (Play/Играть, Pause/Пауза, Settings/Настройки). Prepared runs,
atlases and textures are created once at window initialization. Redraw only
positions/clips glyph sprites inside each control, preserving UVs; controls and
labels use the same viewport geometry. Separate label atlases/draws are currently
used, so a global text batching cache remains unfinished.

Font-enabled native smoke presented 61 frames and strict example Clippy passed.
The real window screenshot was visually inspected: all Latin/Cyrillic labels
were legible inside the controls. Computer-use click on Play followed by Tab and
Enter activated Play/Pause, verified by title `[true, true, false]` and hover/focus
colors while labels remained visible. The window was closed after verification.
This does not yet expose the labels as accessible native control nodes.

UI adds `ScrollPanel` with logical two-axis clamped content offsets. Invalid
extents/deltas leave prior state intact; resizing preserves and clamps offset.
Reveal moves minimally to expose a content-local target, aligning oversized
items to their leading edge. Project translates content into viewport space and
returns the same clipped rectangle for hit/render geometry. Tests cover bounds,
resize, invalid inputs, reveal and actual pointer targeting after projection.
Nine UI tests pass. This is the scroll model; native wheel routing, inertia,
scrollbar controls, focus-follow integration and virtualized lists remain open.

`WindowUi` now normalizes native wheel events into optional logical scroll deltas
in `UiDispatch`. Physical pixel deltas divide by window DPI; line deltas use
configurable logical pixels per line (default 32). Signs match ScrollPanel's
positive offset/content-left-up convention. The owning panel decides whether to
consume the wheel dispatch, permitting later nested scrolling policy. Unfocused
windows do not route wheel input; invalid/nonfinite conversion yields no delta.
Tests prove equivalent pixel movement at 1x/2x and line-wheel application to the
actual scroll model. Both adapter tests and strict all-targets Clippy pass.
This is native event mapping plus deterministic model integration, not yet an
OS-wheel-tested scrolling window, inertia, nested chaining or scrollbars.

Native `ui_window --scroll [font-path]` now projects a 600-logical-pixel content
column into the window. Backgrounds and glyphs share the visible clip; glyph
positions derive from the original scrolled control so clipping does not recenter
labels. Hidden geometry is cleared and toggles retain stable IDs. Eleven core /
adapter tests and strict example Clippy pass; native smoke presented 60 frames.
OS wheel interaction remains unverified: computer-use events failed to affect the
window and the native automation pipe subsequently closed. Focus-follow, inertia,
nested wheel chaining, accessibility and scrollbar controls remain open.

External mechanism research now informs authoring gestures: SceneHistory exposes
`begin_edit` / `commit_edit` with detached SceneEdit previews. Intermediate drag
changes remain outside history; committing validates the final document once and
creates one undo entry. Cancel by dropping the preview. History-instance epochs
reject stale and foreign previews, including undo/redo ABA. Four focused history
tests pass. This is the transaction core, not a completed native editor/gizmo or
compact history storage; detached snapshots are not included in retained-byte
accounting. Source-review rationale and pinned references live in
`docs/engine-research/decisions.md`.

The existing `authoring_history` consumer now runs five detached drag previews,
loads each preview into an isolated scene, commits once, and checks that one undo
restores the original authoring document while the independently loaded play
world keeps its health at 40. The example passes. This is headless gesture
integration; native gizmo interaction and renderer preview extraction remain open.

File input provenance increment: ImportInputs records bounded immutable byte
snapshots and BLAKE3 digests, caching failed reads for an import attempt.
FileInputs resolves relative source paths beneath a canonical root and reads only
up to the supplied byte limit plus a sentinel. ImportInputs::finish validates
current bytes and packages the decoded value with its observations in
ImportedAsset<T>; AssetCatalog can publish this pair as one immutable version.
The file_import example changes a same-length source between decode and finish,
rejects that output while retaining the previous version, then retries and checks
both the replacement value/digest and the surviving old snapshot.

Verified: 25 asset tests, strict all-target asset Clippy and
`cargo run -p voxy_assets --example file_import`. This is a source-file import
boundary, not an incremental watcher or complete asset build system. The portable
provider assumes trusted project directories and does not prevent concurrent
symlink replacement. Validation observes files; it does not lock them through
publication. Versioned build keys cover sorted source identities/digests, importer
identity/version, target and caller-supplied canonical option bytes. Output caching,
watcher revisions and atomic dependency-DAG
publication remain pending. Retained
input snapshots are bounded per attempt, not by total catalog residency bytes.

Audio provenance integration: the asset_playback example now publishes
ImportedAsset<Clip> from a background WAV worker. Its bounded immutable-memory
provider captures the exact decoded WAV bytes; reload replaces clip and provenance
together. Assertions verify different source snapshots/build keys, rejection of
failed/stale imports, and continued playback of the original clip by an existing
voice. A newly started voice uses the replacement. The example and strict example
Clippy pass; this is offline mixing, not an audio-device or file-watcher test.

Rejected publication ownership: `finish_observed` returns a `RejectedImport<T>`
containing the decoded value, captured inputs and validation error. A caller can
retain these for diagnostics instead of losing observations when publication
fails. Tests cover missing inputs, changed content and successful publication;
file_import verifies retained provenance after a real same-length file change.
The decoder can run through `decode_observed`, which returns `FailedImport<E>`
with all captured reads on failure. This does not implement watcher scheduling.

Decode failure integration: file_import now feeds UTF-8 decoding through
`decode_observed`. After publishing a valid replacement, it writes invalid UTF-8
bytes, rejects the reload and checks that the failed attempt retains those exact
bytes while the catalog keeps its last good value and releases pending capacity.
The closure API owns each attempt; it does not catch panics, enforce decoder time
limits or install a global error-retention store. Eighteen asset tests, the file
example and strict all-target asset Clippy pass.

Source invalidation index: SourceDependencies records source reads by compiled
output, separately from the compiled asset DAG. Published results replace the
source set; a failed attempt watches its missing/read-failed inputs alongside the
last good version's sources. The next failed attempt replaces the previous failed
set, and a successful publication clears it. Capacity failures preserve the index.
Changed sources yield sorted deduplicated outputs, which AssetDependencies expands
into a transitive rebuild order. The owner must validate current tickets before
recording results. This is an invalidation index, not an OS watcher, job scheduler
or job scheduler. Joint catalog/index publication is provided below; undeclared compiled outputs still need
registration in the DAG. Input admission failures without recorded IDs cannot be
watched through this index.

Joint source/resource publication: `AssetCatalog<ImportedAsset<T>>::complete_observed`
validates the current ticket before admitting source mappings and publishing the
version under exclusive owner borrows. Superseded/foreign/completed results cannot
change the source index. Changed compiled dependencies settle the current ticket
as failed without recording obsolete reads. Source-index capacity rejection keeps
both structures unchanged and leaves the ticket pending; the owner must retry or
explicitly settle it because that API consumes the result. Tests cover stale results,
failed reads, replacement, index capacity and changed compiled dependencies. The
file_import example uses this operation for success and failure. This is in-memory
owner atomicity, not durable transactions, OS file locking or compiled-DAG editing.
The owner must consistently pair one catalog with its source index.

Content polling and reload loop: SourcePoller reconciles the bounded source index
and checks at most a caller-selected number of distinct files per invocation in
round-robin order. Each provider read receives a per-file byte limit. BLAKE3 content
changes and availability transitions invalidate sources; unavailable errors coalesce
until recovery. Newly registered sources emit an initial invalidation rather than
establishing a silent baseline that could miss edits after import. Surviving source
observations persist across reconciliation, and overflow leaves the scan unchanged.
Unit tests cover unchanged files, same-length changes, disappearance/recovery,
oversized reads, fair scan advancement and source-set replacement. The real-file
example polls edits, maps them to outputs, imports and jointly publishes results,
recovering from invalid UTF-8 and retaining old snapshots. Twenty-five tests, the
file example and strict all-target Clippy pass. This portable content poller must
run outside frame-critical work: file/hash time has no wall-clock bound, edits
reverted between polls are invisible, no OS event adapter or background service
is installed, and the example dispatches synchronous imports explicitly.

Background source polling: SourcePollWorker owns a dedicated file/hash thread and
capacity-one request/result channels, admitting exactly one unconsumed scan. The
owner submits source-ID snapshots and retrieves changes with nonblocking channel
operations; it maps returned IDs through the current source index. Busy and source
capacity rejection are explicit. `close` returns a join handle so shutdown waits
can stay outside frame-critical work. Dropping detaches a closing worker; neither
path cancels or time-bounds an already blocked filesystem read. The real-file
background_watch example verifies missing-file recovery, equal-length content
changes, deletion/recreation, one outstanding scan, capacity rejection and joined
shutdown with an outstanding scan. Twenty-five asset tests and strict all-target
Clippy pass. This is an owner-triggered service, not a periodic daemon or OS event
watcher; background imports and application runtime wiring remain pending. ID
counts and file bytes are bounded separately; variable-length IDs have no global
byte-residency budget.

Background decoding pipeline: AssetImportWorker<T> runs a caller-owned typed
decoder on a dedicated thread with one unconsumed job/completion. It captures
bounded file inputs, retains reads on decode failure and revalidates successful
input bytes before returning an ImportedAsset. The owner retains the submitted
ticket and alone calls complete_observed. The background_import example joins
content polling, source invalidation, threaded UTF-8 import and owner publication;
it verifies a distinct decoder thread, busy rejection, superseded completion
rejection, corrupted-file recovery and survival of old Arc snapshots. Input byte
limits do not bound decoded output size or decoder CPU time. Decoder panics
disconnect the worker; owner-side recovery must settle/retry retained tickets.
Compiled dependency snapshots, worker pools, periodic scheduling and application
runtime integration remain pending. Joined shutdown can wait on active IO/decoding.

Rendered file hot reload: asset_hot_reload_pixels connects SourcePollWorker and
AssetImportWorker<ObjAsset> to a real temporary OBJ source and owner-thread joint
publication, then uploads the resulting SceneMesh versions to SceneRenderer. On
Apple M4 Max/Metal, real GPU readback proves failed import leaves pixels identical,
corrected geometry moves the quad by 16 pixels, and a held old Arc version renders
identically after replacement. Decoder execution is asserted off the owner thread;
OBJ input/output counts are explicitly limited. The four-state PNG at
/tmp/voxy-asset-hot-reload.png was viewed. This is an offscreen integration example,
not native app runtime wiring, GPU residency replacement scheduling or a continuous
editor viewport. The owner harness waits for completion outside a frame loop.

Verification limitation for the rendered example: strict Clippy is blocked by
existing dependency diagnostics in physics (219 errors) and a doc_markdown error
in voxy_render/src/surface_lighting.rs. The example compiled and ran successfully;
GPU assertions passed and git diff --check passed. No strict Clippy pass is claimed
for this render integration.

Retained publication retries: `complete_pending` borrows an optional completed
import and consumes it only after ticket and source-index admission succeeds.
Capacity errors preserve the exact decoded value and captured byte allocations,
allowing the owner to relieve pressure and retry without reading or decoding again.
Absent results are rejected explicitly; stale results remain available for
diagnostics but must never be reassigned to a newer ticket. The consuming
complete_observed convenience API keeps its existing release-on-error behavior.
A regression test retries an index-capacity rejection on the same ticket and
verifies shared captured bytes survive publication.

Threaded compiled dependencies: AssetImportWorker::new_with_dependencies passes
immutable compiled snapshots from AssetCatalog::prepare_import to the decoder.
submit_import bounds snapshot counts separately from file observations and copies
Arc references, while dependency-bearing tickets submitted without snapshots are
rejected. A synchronized worker test holds an old dependency, replaces it in the
catalog, verifies decoding still uses the old value and rejects the obsolete
publication; a fresh prepared import uses the replacement and publishes correctly.
Resources now require Send + Sync for shared cross-thread snapshots. The public
prepared-import mapping must remain unmodified before submission; key validation
rejects missing/extra IDs but does not authenticate caller-substituted Arc values.
Compiled dependency contents are not yet incorporated into file build keys, and
typed heterogeneous importer routing and dependency-aware worker scheduling remain
pending.

Dependency-aware rebuild scheduling: RebuildPlan snapshots affected compiled jobs
and their graph dependencies. It claims ready jobs up to an explicit concurrency
limit, waits for owner-reported publication before admitting dependents, supports
claim deferral under worker backpressure, and marks transitive pending dependents
Blocked after failure. Invalid transitions preserve state. Dependencies outside the
plan still require ready catalog versions. The dependency_rebuild example threads
a source -> material/mesh -> scene diamond, verifies changed source values reach
the scene, preserves an old scene snapshot, and proves a broken source blocks
dependent imports while last-good output remains available. Twenty-nine asset
tests pass. This is a bounded per-batch scheduling core; overlapping change batches,
heterogeneous worker routing, app wiring and automatic retry policy remain pending.
Last-good catalog values may remain Ready when a plan marks them Blocked; consumers
must inspect plan diagnostics to distinguish retained values from fresh outputs.

Overlapping change invalidation: AssetCatalog::invalidate atomically preflights a
bounded resident affected set, advances distinct revisions, releases current
pending slots and marks resources Dirty while retaining immutable last-good data.
Dirty demand does not start workers, and Dirty versions cannot satisfy prepared
compiled dependencies. Superseded completion tickets cannot publish. Unknown IDs
and revision exhaustion preserve the entire pre-invalidation state. The threaded
diamond example now invalidates resident affected outputs before dispatch; after
a broken base import, the retained scene is explicitly Dirty instead of appearing
fresh/Ready. Thirty-one asset tests, the diamond example and strict all-target
asset Clippy pass. External worker CPU/IO is not cancelled, and the owner must
replace/reconcile old scheduling plans and settle worker completions. This provides
the catalog transition for overlapping edits, not complete automatic batch merging.

Overlapping batch replacement: RebuildPlan::replacement merges every Pending or
Running job with new changed roots and expands through the current compiled DAG.
It leaves the old plan untouched, deduplicates repeated changes and does not retry
unrelated failed/blocked outputs automatically. Sorted outputs identify the full
resident set to invalidate before replacement dispatch. A catalog integration test
starts an import, adds an unrelated change, replaces the batch, invalidates old
tickets, rejects its late result and runs the complete dependency plan to fresh
outputs without losing previously unfinished work. The owner must still drain
old worker completions and avoid reporting them into the replacement plan. Removed
outputs/graph edits require explicit lifecycle handling; this is snapshot-batch
coalescing, not continuous automatic runtime wiring.

Asynchronous rebuild attempt identity: claim_ready allocates a non-wrapping unique
process-local RebuildClaim. finish_claim/defer_claim validate the token as well as
the job state, preventing late completions from a replaced plan or an earlier
deferred attempt from changing a new job with the same resource ID. Duplicate
settlement is rejected. Token exhaustion returns the job to Pending. The threaded
diamond example uses token settlement; legacy ID-only methods remain available
for local owner-controlled scheduling and do not provide this asynchronous check.
A regression test covers replacement, deferral/reclaim, foreign-plan and duplicate
completion rejection. Catalog tickets independently enforce resource publication.

Worker-carried rebuild claims: submit_rebuild validates that the claim resource
matches the prepared import, copies the attempt token into the bounded worker job
and returns it in ImportCompletion. The threaded diamond settles the returned
token, avoiding a separately maintained resource-to-attempt association. A worker
test rejects mismatched submission before enqueue, verifies the returned token
settles the original plan and publishes through the catalog independently. Plain
submit/submit_import return no rebuild token. Claims do not authenticate catalog
ownership or current plan state at submission; owner ticket publication and
finish_claim remain the respective validation boundaries.

Native live model viewport: asset_window opens a SceneSurface and draws an OBJ
while a dedicated source-poll thread checks content every 200 ms and a separate
import thread decodes bounded geometry. Redraw takes nonblocking completions,
publishes only current tickets and uploads accepted geometry on the owner thread.
Changed inputs invalidate in-flight requests; one reload flag coalesces edits until
the worker is available. Failed imports retain the previous GPU geometry and show
a failure title; corrected input replaces it. The external real-file smoke harness
observed publication at frame 0, failure at frame 21 with last_good=true, recovery
at frame 93 and a successful exit after 94 presented native frames. Worker joins
run after the event loop exits. Run with `cargo run -p voxy_render --example
asset_window -- /absolute/path/model.obj`. OBJ inputs are limited to 4096 bytes
and 64 attributes/vertices/triangles. This is a native renderer example, not the
procedural voxy_app world's model integration; GPU upload occurs in the frame loop
and has no measured latency budget. Native window screenshots were not captured.

Native example checks: `cargo clippy -p voxy_render --example asset_window
--no-deps -- -D warnings` passes, as does strict all-target voxy_assets Clippy.
This focused renderer check excludes dependency linting; the broader renderer
strict check retains the separately documented dependency failures.

The final asset_window source was rebuilt successfully and its native smoke rerun
passed: first publication at frame 0, corrupt-file failure at frame 22, recovery
at frame 93 and exit after 94 presented frames. An intervening physics compile
error had been fixed in the current external work before this successful build;
the final evidence is from the rebuilt executable, not the older binary.

Native edit-burst acceptance: the window processes ready source invalidations
before accepting worker completions in a frame, rejecting already-known obsolete
results before uploading geometry. tools/test_asset_window_reload.py is a
reproducible external-source harness: after initial publication it corrupts the
file, confirms retained geometry, performs three rapid valid edits and verifies
convergence to the final first-vertex position without later rollback. The rebuilt
example passed 191 native frames (failure at 23, final geometry at 70); focused
renderer no-deps strict Clippy and diff checks pass. The smoke window remains open
for one second after its last publication to observe late results. Intermediate
valid edits may be published if observed; unobserved edits between polls remain
invisible. This does not eliminate the external-file race after validation or
prove general OS-watcher event ordering.

Stable identity/location foundation: AssetLocations keeps logical AssetId bindings
separate from validated SourcePath locations. Relocation preserves logical
references and removes the old reverse mapping; collisions and metadata-capacity
failures preserve both directions. Source paths reject traversal, absolute paths,
ambiguous separators and unsupported portable syntax. The byte budget counts
logical ID/path UTF-8 lengths, not exact map allocation overhead; keys are lexical
and do not detect filesystem case aliases. Godot's pinned ResourceUID indirection
informed this change (see engine-research/decisions.md). Persistence, random ID
issuance, filesystem rename transactions and worker/watch relocation wiring remain
pending; current file reload examples still identify their output by the path.

Located worker imports: new_with_locations supplies a captured SourcePath
independently of the logical output ID, and submit_at/submit_rebuild_at copy that
location into the bounded worker job and completion. Located decoders reject
submissions without a location. source_relocation performs a real file rename
while an earlier job is outstanding, invalidates the output ticket, rejects the
old completion, imports the new path into the same logical resource and replaces
source invalidation mappings after publication. Old output snapshots survive.
The owner explicitly coordinates physical rename, location-table change and
catalog invalidation; this is not a durable joint rename transaction, automatic
filesystem-rename detection or independent location-revision validation. Source
lookup changes do not mutate a queued job's captured location.

Persistent identity manifest: AssetLocations serializes deterministic ID-sorted
version-1 JSON through a capped writer, validates a capped document into a detached
registry, and rejects unknown fields/schema, duplicate IDs/paths, invalid source
paths and metadata limits. load_manifest reads only the byte cap plus one sentinel.
save_manifest creates an exclusive temporary file beside the target, writes and
syncs it, then renames; errors clean up the temporary file, and preflight failures
leave an existing manifest untouched. File tests cover real save/reload after
relocation, failed capacity admission, failed rename into a nonempty directory and
temporary cleanup. source_relocation saves/reloads the location table before its
new-path import. Forty asset tests pass. No directory fsync/power-loss guarantee or
cross-platform replace semantics are claimed; IDs still require caller issuance
and physical rename is not committed jointly with the registry.

Manifest-driven native resources: asset_window accepts `--manifest
/absolute/path/assets.json logical-id` and resolves the selected output's SourcePath
from captured manifest bytes inside the import worker. Manifest and OBJ reads share
the provenance validation; the owner performs no manifest file IO in redraw.
Source polling includes both observed files, so replacing the manifest after a
physical rename remaps the same logical resource and drops old source observations
after successful publication. The source index/poller reserve four source keys to
retain last-good and failed attempt reads through relocation. Manifest reads are
capped at 65536 bytes (128 bindings/65536 logical metadata bytes), OBJ reads at
4096, and total attempt input bytes at 69632.

`python3 tools/test_asset_window_reload.py --manifest` passes on the rebuilt native
example: 240 frames, retained last-good geometry through failures at frames 23/71/95,
final remapped geometry at frame 119 with ID logical-quad unchanged. The harness
renames the real OBJ, atomically replaces the manifest and leaves the new file
invalid for a scan before correction. Direct-file acceptance also passes (194
frames). Focused renderer no-deps strict Clippy and diff checks pass. This responds
to manifest updates; renaming a file without updating its binding is still a
missing-source error, not automatic filesystem rename identity discovery.

Main application model mode: native model control moved into the high-level
voxy_editor crate, whose production dependencies point to renderer/assets rather
than making the renderer library own winit/editor orchestration. The existing
asset_window renderer example is a thin compatibility entry (editor dependency
only in render dev-dependencies). voxy_app now dispatches --model PATH and
--model-manifest MANIFEST LOGICAL_ID to that shared viewport. Main binary build
passes with existing unrelated dead-code warnings; focused editor strict no-deps
Clippy passes. Rebuilt main-app acceptance passes 178 native frames for manifest
rename/corrupt-source recovery and 175 for direct OBJ edit bursts. The previous
example entry also passed after extraction. The harness supports --app --binary
target/debug/voxy_app. This adds a real main-application mode, not model entities
in the procedural terrain world or a full scene editor/inspector.

### Scene-bound native model instance

The native model viewport now owns a `SceneGraph` node with a typed
`ModelInstance` component referencing the logical `AssetId`. GPU transforms
come from the node's world matrix on each redraw; resource publication changes
geometry without replacing the node. A focused editor test edits translation
and scale, invalidates an import, rejects its obsolete result, and verifies that
a subsequent failed import preserves the node, reference and world transform.
`cargo test -p voxy_editor --lib` and editor all-target Clippy with warnings denied
passed. The native manifest relocation smoke test presented 122 frames and
retained last-good geometry through corrupt source and rename recovery.
This remains a single-model authoring viewport; interactive gizmos, multiple
instances and placement in the procedural world are still missing.

### Shared model geometry with independent instances

The native viewport supports up to 128 model instances referencing the same
logical asset and GPU geometry. Each instance has its own scene node and GPU
transform. D duplicates the selected instance, Tab cycles selection, and arrow
keys move it in viewport coordinates. Active-in-hierarchy controls draw inclusion.
This is keyboard editing; picking, gizmos, deletion, undo and scene persistence
are not integrated in this viewport yet.

The editor regression test duplicates an edited node, moves only its duplicate,
then invalidates and fails resource imports: both transforms and logical resource
references remain intact. Native smoke exercises the same edit commands before
rendering two instances; manifest rename/corrupt-source recovery completed with
240 presented frames and unchanged world matrices. Editor all-target Clippy
with warnings denied passed. Geometry is shared through repeated draws, not GPU
instanced batching; no performance comparison is claimed.

### Native authoring undo/redo

The model viewport now uses the existing `SceneHistory` for duplication and
translation edits (Z undo, Y redo). History is bounded to 64 versions and 1 MiB
of serialized authoring data. Restoring authoring data rebuilds scene handles
and the instance transform bindings, while retaining the imported resource and
shared geometry. Selection is clamped when undo removes a duplicate. A new edit
after undo discards redo. History currently stores flat instance authoring data;
all instances refer to the viewport's selected model resource, and there is no
mixed-resource scene save/load in this mode yet.

The editor regression test covers independent transforms, duplicate undo/redo,
movement undo/redo, failed/stale reload and redo-branch invalidation. Native smoke
also undoes/redoes before rendering and after recovered publication, comparing
resource Arc identity to ensure authoring undo does not roll back imports.

### Mouse instance selection

Native cursor/left-button events route through `PickViewport` and `SceneGraph::pick`.
Bounds are computed from the current last-good imported mesh on each click and
attached to the current instance nodes. Undo-restored nodes therefore receive
fresh bounds; failed imports retain selection bounds for the rendered old mesh.
Physical cursor and physical window dimensions use the same coordinate space,
so selection remains independent of display scale. Outside/missing-model clicks
retain selection. Nearest active bounds hit wins; this is AABB selection, not
triangle-accurate surface selection or gizmo hit testing.

Two editor tests passed, including translated selection, overlap/depth, inactive
instances and regenerated undo/redo handles. Editor all-target Clippy with
warnings denied passed. A new native smoke assertion exercises selection after
resource recovery and undo/redo, but the native rebuild was blocked by concurrent
physics changes accessing private ImagePair fields; that runtime assertion has
not yet been verified. Earlier native frame counts do not verify this addition.

### Instance deletion and empty authoring scenes

Delete/Backspace removes the selected native viewport instance through
SceneGraph::remove_subtree and commits an authoring history version. Removing
the final instance is supported: draw lists remain empty, movement/selection
keys are harmless, and D creates an instance again. Undo/redo restores deleted
instances, including transitions into and out of an empty scene. GPU transform
bindings are rebuilt after deletion or restoration; the shared resource and
geometry are retained.

Viewport ObjectIds are now allocated independently of list positions and remain
unchanged for surviving/restored objects. The allocation counter is not rewound
by undo, preventing a new branch from reusing a deleted object's identity within
this session. IDs are not yet persisted across viewport sessions.

Three editor tests passed, covering empty-scene editing, deletion undo/redo,
identity preservation/non-reuse, picking and import-independent authoring
history. Native example rebuild remains blocked by concurrent physics ImagePair
field visibility errors; no new native execution is claimed for deletion.

### Standalone editor runtime and native acceptance follow-up

`cargo build -p voxy_editor --bin voxy_editor` now builds the native authoring
application directly, using the same viewport library as the main app and
renderer compatibility example. This preserves the production editor/render
boundary and avoids renderer-example physics dev-dependencies. It does not fix
or certify the concurrent physics build errors reported above.

The standalone editor compiled; all-target editor Clippy with warnings denied
and three editor regression tests passed. Native manifest relocation smoke
presented 223 frames, retained the last-good model through invalid source,
recovered the renamed logical resource, selected it after undo/redo, and verified
delete-to-empty plus restoration of object identities. Selection in this smoke
is invoked through the viewport's selection method; actual OS mouse event
injection and visual selection feedback remain unverified. Native deletion
acceptance checks restore the scene after the render loop; empty-scene rendering
has not been visually inspected.

Reproduce enhanced acceptance with:
`python3 tools/test_asset_window_reload.py --binary target/debug/voxy_editor --manifest --editor-checks`.

### Transactional mouse translation

The native model viewport now previews translation while dragging a selected
bounds hit with the left button. Motion converts physical pixel displacement to
the viewport's current identity-camera coordinates; Z is unchanged. This is
plane translation, not a perspective-camera or axis gizmo. A detached SceneEdit
captures the gesture's authoring epoch. Mouse release commits one history entry
for all intermediate motions; Escape, focus loss, cursor exit, resize or keyboard
editing cancels and restores the original transform. Cancellation preserves
redo, and no-op gestures use existing history no-op handling. Resource publication
remains outside authoring history.

Four editor tests passed, including repeated previews producing one undo step,
cancellation preserving redo, empty scenes, picking and stale imports. Native
smoke contains post-render drag transaction/undo/cancel acceptance checks;
these call the gesture methods directly, not OS mouse event injection. Visual
preview and actual pointer event delivery remain to be inspected.

### Real macOS input inspection: unresolved visual update

A temporary local macOS app wrapper launched the standalone editor for CUA
inspection. Its native window screenshot showed the expected white quad on black.
A real left click changed the AX window title to selected instance 1/1, and
Right/D/Right keyboard input changed the title to instance 2/2. The screenshots
viewed after those actions still showed the original quad rectangle; a visible
translation or duplicated extent was not demonstrated. CUA drag attempts also
failed to demonstrate motion. One earlier invocation reported App quit without
a captured reason; the relaunched app accepted click and keyboard input.

This is unresolved evidence of either a viewport presentation/update problem or
an observation/input delivery problem. The native smoke assertions operate on
authoring state and presented-frame counts; they do not prove edited instance
pixels changed. Do not treat the earlier smoke results as visual proof of mouse
translation, keyboard translation or multi-instance presentation. Next validation
must correlate input, updated world matrices, redraw/present and actual pixels.
The QA window was closed after inspection; the temporary wrapper is outside the
repository, and the source OBJ was not modified.

### Input-to-presentation tracing and GPU transform reproduction

Set `VOXY_EDITOR_TRACE_INPUT=1` to log edit input and the next presentation's
frame number, draw count, outcome and world translations. Disabled by default.
Real macOS Right input at frame 1054 produced one Presented draw with translation
(0.05,0,0.5); D at frame 1065 produced two Presented draws at (0.05,0,0.5) and
(0.2,0,0.5). The viewed screenshot showed the new duplicate's right extent but
the original instance's left edge remained at its initial location. Thus input,
scene matrices and continued presentation are confirmed, while repeated GPU
transform updates still require direct readback evidence.

`cargo run -p voxy_editor --example transform_pixels` is a new actual-GPU
reproduction using one geometry and one transform buffer: update, 16-pixel
translation, reset. It asserts a shifted centroid and exact reset-image equality.
The check uses editor dev-dependencies without the renderer's physics examples.

Direct GPU reproduction passed on Apple M4 Max / Metal: centroids were
31.5, 47.5, 31.5; reset pixels exactly matched the original frame. This proves
SceneTransform::update works for the offscreen render/readback path on this
adapter. It does not yet explain or fix the differing native window observation;
continue testing surface presentation and screenshot/input timing separately.

### Occlusion explains the inspected stale native frame

Follow-up native tracing showed four Right inputs changing X from 0 to 0.2
while every traced render returned SkippedOccluded at presented frame 157. After
presentation resumed (a subsequent Presented trace at frame 180 with X=0.2), the
viewed native screenshot showed the quad's left edge at 448 pixels rather than
its original 320: exactly the 128-pixel shift for X=0.2 in a 1280-pixel image.
Thus the inspected unchanged frame was not proof of a GPU-transform defect.
Keyboard translation has now been visually observed after actual presentation.
Undo returned the world matrices to X=0, but its input again occurred while
occluded; visual undo/drag remains unverified.

The GPU reproduction was strengthened to 120 consecutive submissions per pose
without CPU waits between submissions. It passed on Metal with centroids
31.5,47.5,31.5 and exact reset-image equality. Native input tracing now retains a
pending edit through skipped frames and logs outcome transitions until Presented,
rather than consuming the trace on an occluded frame. It does not force rendering
of an occluded surface or change platform presentation semantics.

### Native viewport scene persistence

The standalone editor accepts `--scene PATH`. Existing files are bounded-read
and validated before opening the window; new paths configure an explicit save
location. F5 uses the existing atomic scene-file replacement; F9 validates a
candidate before committing it as an undoable authoring version. Startup loading
starts a fresh history. IO failures are shown in the window title and leave the
editor running. IO is synchronous on these explicit commands, not background;
no frame latency guarantee is claimed.

SceneDocument v1 stores stable ObjectIds, names, activity, all transform fields
and an `editor.model.v1` string schema referencing the selected logical resource.
The viewport rejects hierarchy, missing/extra components or a different model
reference rather than flattening/substituting silently. Its source manifest/OBJ
is still supplied through the CLI; model files are not embedded. Mixed model
resources and hierarchical viewport editing remain future work. Loaded model-N
IDs advance the allocation counter, preventing subsequent duplication from
reusing a saved identity. Bounds are reconstructed from the live model.

Five editor tests passed, including real-file save/load into a fresh App owner,
translation/rotation/scale, names/activity/resource IDs, ID allocation after load,
invalid JSON/foreign resource rejection preserving state and undo after rejection.
Editor all-target Clippy with warnings denied passed.
Native manifest reload with `--editor-checks --scene-test` passed with 117
presented frames. Post-render acceptance saved a real scene file, deleted an
instance and loaded the file, asserting exact authoring-document equality. The
harness inspected the saved JSON and required two persisted objects. This verifies
file/state integration; it does not demonstrate actual F5/F9 OS key delivery or
visually inspect the reloaded frame.

### Priority batch 01: selection presentation and axis editing

User requested execution in groups of ten; the first priority batch is tracked
in `editor-batch-01.md` and remains open. Amber selected-instance bounds and RGB
translation handles are derived GPU geometry. Selection now intersects actual
triangles for the identity-camera depth segment, avoiding bounding-box false
positives hiding objects behind. Inactive/singular instances are skipped and
nearest intersections win. It does not implement texture-alpha selection.

Red/green handles constrain X/Y; the blue center maps vertical drag to depth.
All use the existing transaction/undo/cancel path. They are world-axis handles
for the current flat scene and identity camera; parent-space conversion and
perspective-camera interaction remain to be added with hierarchy integration.
Eight editor tests passed, including actual pointer handler routing for all axes,
unchanged other axes, undo and resource Arc preservation. Actual Metal GPU
readback verified amber outline and all three handle colors through 120 async
submissions per pose and exact pixel restoration after transform reset.
Native window inspection showed the amber outline and red/green/blue handles.
A live gesture logged preview at frame 249 and commit at frame 287 with Y=-0.311;
the viewed moved frame matched that vertical displacement. A later cancellation
kept the committed transform. An actual Z key restored Y=0 in authoring state,
but its redraw was SkippedOccluded, so visible undo remains pending. A separate
temporary bundle packaging experiment only reached an empty initial window and
did not improve this acceptance result; it is not a supported packaging claim.
The normal standalone native reload/persistence acceptance passed on the final
implementation with 247 presented frames. Batch 01 remains open.

## Native editor batch 01 integration (2026-10-01)

The supported OBJ authoring viewport now keeps per-object logical resource
references and a per-resource GPU geometry/outline table. Manifest projects can
cycle the selected object's model with R or by clicking the inspector model row;
duplicates retain their source resource. Independent file reload preserves other
resource versions, while changing the shared manifest invalidates its consumers.
The bounded resource list is captured at project open; reopening is required to
discover newly added manifest IDs. Last-good models survive failed imports.

Interactive windows display a scrollable scene tree and a numeric TRS inspector.
Click a property, enter a finite number and press Enter; Escape cancels. Model
references and local activity can also be changed in the inspector. Parent/detach
allows choosing a parent in the tree; invalid cycles preserve the scene. Dragging
world axes converts deltas through the parent's inverse transform. Deleting a
parent deletes its subtree, and undo restores the same persistent object IDs.
Scene files persist supported object names, activity, TRS, hierarchy and model IDs.

F6/Play creates a detached runtime graph; a demonstration rotates active roots.
Authoring edits and saves are disabled during Play, and Stop restores the exact
authoring document without adding runtime changes to undo history. This is a
Play/Stop isolation demonstration, not a general gameplay/physics integration.
The inspector displays authored values during Play. Fonts currently use system
Arial or DejaVu Sans and labels use ASCII fallback; native accessibility and
full text editing remain future UI work. Identity-camera gizmos remain scoped to
this viewport.

Stride is already among the pinned engine research candidates
(`stride3d/stride`, commit `a7fa31ced680c7d4a919f1fe233051cf508f6060`).
Adding it to the coverage target does not turn source triage into a completed
architectural review or imply feature parity.

## Shared fixed-step scene simulation (2026-10-01)

The existing SimulationClock moved from voxy_runtime into dependency-free
voxy_time; voxy_runtime retains its public compatibility exports. SceneSimulation
owns behavior lifecycle, bounded command admission and fixed scheduling for one
scene identity. Explicit SimulationLimits bound behaviors, commands and catch-up
steps. Foreign/invalid advances preserve queued commands; rejected behavior
admission invokes no Awake. Stop dispatches cleanup once and is terminal.

Native Play now uses 60 Hz fixed ticks with up to eight catch-up steps. The clock
caps admitted real elapsed at 100 ms and separately reports discarded real time
and scaled simulation backlog. A relative step tolerance prevents phantom ticks
for zero elapsed with tiny fixed steps. The editor advances a detached runtime
graph and cleans up before restoring the exact authoring document.

The public `fixed_scene` example verifies 60 ticks, inherited motion/activity,
command deletion, render extraction and Stop. Render interpolation, generic
physics adapters, behavior serialization and full input/gameplay integration
remain pending; the native Spin behavior is still a demonstration.


### Native scene/playable integration, batch 02 (2026-10-01)

Items 11–20 are accepted in the bounded OBJ/character project documented in
`editor-batch-02.md`. The native editor now persists Character/BoxCollider descriptors,
exposes numeric physics fields and undo/redo, consumes named keyboard actions at
fixed ticks, and runs the existing swept character solver through `voxy_gameplay`.
Runtime motion/grounding stay separate from authoring. Active/interpolated render
extraction, lifecycle, typed queued CRUD, owner deletion/reuse and exact Stop restore
have a combined native acceptance (34 ticks / 73 presented frames). The previous
reload/manifest/scene editor acceptance passed 258 presented frames. This does not
claim arbitrary rigid-body, affine-collider, camera, gamepad or generic-inspector
parity. See `crates/voxy_editor/examples/game/README.md` for the runnable fixture.


## Native static 3D integration: batch 03 (21–30)

`docs/editor-batch-03.md` records the bounded implementation: the existing scene
camera now drives native render, picking and gizmos; navigation snapshots can be
saved as a scene descriptor. Static glTF/GLB resources bring editable TRS
hierarchies, opaque base-color textures/material overrides and flat directional
lighting into the native project. Affine static boxes use continuous SAT for
kinematic character sliding and bounded runtime overlap recovery.

The original `crates/voxy_editor/examples/scene3d` project exercises import,
texture publication, both camera projections, persistence, physics and Play/Stop
restoration. Its native acceptance passed 25 fixed ticks and 36 presented frames
on the final binary. Existing gameplay acceptance passed 34 ticks/72 frames;
OBJ/manifest reload acceptance passed 269 frames. These are separate bounded
scenarios, not complete importer/renderer/physics parity.


## Static import/render follow-up

Native editor glTF import now supports several material primitives per node,
with stable source parents and inherited transforms. All six minification filters
retain their independent mip behavior. Instances share GPU primitive geometry and
matching texture bindings within a resource revision. Different sampler variants
still allocate separate textures. Native multi-material acceptance passed 25 ticks
and 60 presented frames; the original scene passed 25 ticks and 59 frames.
Editor tests passed 18/18, renderer tests 65/65 and strict editor Clippy passed.
See the batch 03
follow-up for boundaries and reproducible commands. This closes these concrete
import/render gaps; broad Unity/Godot/Stride parity remains ongoing.


Sampler variants now share the same GPU image allocation within each model
revision. Per-material views preserve base-only versus mip-filtered behavior.
The extended native variant passed 25 ticks / 59 frames and asserts shared
image identity across different sampler configurations. Cross-resource image
residency and full material/PBR parity remain outside this implementation.


Identical decoded images now share storage across model resources within one GPU
context. Weak cache entries do not retain unloaded model revisions; versioned
content identity includes dimensions and pixels. Full mip storage supports both
base-only and mip consumers without allocation replacement. Native cross-resource
acceptance passed 25 ticks / 60 presented frames. Global residency budgets,
streaming/eviction policy and full material parity remain future work.


GPU models now follow scene resource references. Removing the final reference
releases model residency; Undo/restored references upload from the CPU catalog.
Inactive references remain resident. Native shared-image eviction/restoration
passed 25 ticks / 36 frames; the independent Delete/Undo/Redo and OBJ reload
scenario passed 135 frames. This establishes lifetime-driven eviction; byte
budgets and pressure-driven admission remain future work.


GPU images now have preflight byte admission (256 MiB default, configurable via
`VOXY_GPU_IMAGE_BUDGET_BYTES`). Unique full mip allocations count once across
resources. Rejected GPU candidates preserve previous models and defer residency;
released scene references permit retries. Native acceptance at an exact 84-byte
limit passed 25 ticks / 60 frames, including shared residency and pressure
rejection. Editor tests 19/19 and strict Clippy passed. This is an image-storage
budget; geometry, driver/in-flight memory, LOD and pressure-driven eviction of
still-referenced resources remain outstanding.


Deferred GPU uploads now wait for a changed source revision, image-cache membership
or budget instead of repeating image hashing every frame. The source identity is
weak; cache membership is tracked separately from aggregate bytes. Byte accounting
and resource lifetime are preserved. Geometry budgeting and pressure-driven LOD
remain outstanding.


Geometry accounting now reads all five actual GPU buffer sizes and deduplicates
shared primitive/outline allocations. Native static 3D measured 31,632 unique
bytes against 49,696 with repeated handles counted, passing 25 ticks / 59 frames.
This is measured geometry accounting; geometry admission limits remain pending.


Model geometry now has preflight byte admission with a 256 MiB default ceiling
(`VOXY_GPU_GEOMETRY_BUDGET_BYTES`). Unique primitives, outlines and fallback
geometry are counted, with old live revisions retained during replacement.
Geometry occupancy/limit changes wake deferred uploads. Native acceptance passed
25 ticks / 58 frames, including predicted-versus-actual buffer sizes and rejection
under geometry pressure. Driver/UI/in-flight allocations and pressure-driven
LOD/eviction of referenced models remain outside this policy.


GPU residency now follows effectively active renderable references, releasing
hidden models and unshared images. Active hierarchy containers alone do not pin
GPU geometry when every leaf part is hidden. Reactivation rebuilds from the CPU
catalog; hidden import completions avoid GPU upload. Native acceptance passed
25 ticks / 62 frames. This is activity-driven eviction; frustum/occlusion
streaming, residency hysteresis and visible-model LOD remain outstanding.


Typed processor requirements now use `SceneGraph::active_components_with<A, B>`
and the existing `ComponentTable::rebuild_active_with<A, B, E>`. Associated rows
are built in a separate table and published together only on success; failed
construction preserves previous rows and releases staged resources. Membership
includes inherited activity and generational ownership. This covers transactional
associated-data publication, not incremental processor caching or parallel
execution. The factory must keep external side effects outside the transaction.
See `engine-research/mechanisms/stride-processors/review.md` for source evidence,
tradeoffs and verification scope.


Associated-data lifecycle now includes revision-based factory reuse and validated
published-row queries. Component mutable access conservatively advances an opaque
token; shared interior mutations and dependencies outside the two requirements
need explicit invalidation. Three associated-data integration tests, two revision
regressions and the scene library tests passed; library/test/benchmark strict
Clippy passed. A runnable queued gameplay example exercised unchanged reuse,
failed construction without stale dispatch, repair and activity retirement. Its
initial run passed; a float-comparison lint was fixed and repeat example validation
is pending. Local benchmarks do not establish universal speedup for cheap rows;
full rebuild remains a valid explicit strategy. Incremental membership journaling,
external dependency tracking and enforced parallel access remain outstanding.


## Nine-direction goal and persistent prefab core (2026-10-02)

The full delivery goal remains active; its current acceptance checklist is
`engine-goal-2026-10-02.md`. Persistent prefab composition now uses the existing
SceneDocument loader and component registry, with nested stable instance identities,
authoring overrides, typed reference remapping, bounded expansion and shared atomic
file IO. The runnable persistent_prefab example saves/restores links and overrides.
Core tests do not establish editor prefab workflows or asset-revision integration;
these are the next required integration steps before the scene/prefab item closes.

Native editor now loads persistent nested prefab composition with observed source
inputs and saves ordinary property/component edits as overrides. The real native
acceptance exercised two manifest-backed nested instances, edit/save/load/history,
corrupted dependency retention, restoration and isolated Play/Stop across 13 frames
on the final binary.
See engine-goal-2026-10-02.md for evidence and remaining prefab UI/structural,
background-reload, history-metadata and standalone integration work. This does not
close the full scenes/prefabs direction or the nine-direction goal.

Prefab structural overrides now retain deletion and parent changes during saving.
Composition metadata is part of bounded SceneHistory snapshots, so undo/redo also
restores source links across different scene compositions. Save captures against
source baselines and refuses changed historical dependencies. The extended native
prefab acceptance additionally deletes/saves/undoes/resaves a linked instance.
See engine-goal-2026-10-02.md for the current evidence and open integration work.
