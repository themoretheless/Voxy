# Voxy: nine-direction delivery goal

Status: active. No category is complete merely because a crate/API/example exists.
Completion requires the existing editor and standalone game to consume the same
contracts, with reproducible integration acceptance. This document is the current
checkpoint for the goal requested on 2026-10-02; older coverage notes remain evidence
for their stated bounded scenarios, not blanket acceptance.

| Direction | Inspected existing foundation | Required delivery/acceptance still open |
| --- | --- | --- |
| Architecture | ADR 0001/0002; domain-owned stores, authoring IDs, generations, extraction, asset tickets and serial schedule | Trace editor/game update phases and resource lifetime end to end; remove duplicated adapters where consumers genuinely overlap |
| Scenes/prefabs | Transactional SceneDocument, registry, atomic bounded scene IO; persistent nested composition added below | Prefab creation/instantiation/revert UI; background dependency reload and standalone publication; migration policy |
| Editor | Model selection, gizmos, material/physics/audio fields, shared history, isolated play/stop and keyboard panel focus | Generic registered component inspector; remaining prefab workflows and combined native authoring/game acceptance |
| Assets | Manifest IDs, bounded observations, import workers, immutable publication, artifact cache, OBJ/static glTF/texture and observed WAV import/reload; bounded audio settings editor | Settings editing for remaining imports, cache quotas, prefab dependency imports/cache and safe reload of all required resource types |
| Graphics | Existing scene renderer, lights/shadow/skinning modules, native certified LOD | Combined editor/game materials/light/shadow/transparency/clip animation/culling acceptance; standalone module tests alone do not close this |
| Game loop | FrameLoop, FixedSceneSimulation, BehaviorRunner, named input | Project-owned game logic adapter, lifecycle/input integration and identical standalone/editor semantics |
| Physics | General physics foundation and scene-bound CharacterPhysics/affine static boxes | General scene dynamic bodies, triggers, queries and collision events; character interactions with the general physics world |
| Audio/UI | Saved sources/listener/buses, spatial mixer, filtered WAV import, asynchronous device/import and live reload; audio settings editing and editor text/focus integration | Native visual acceptance and saved game interface/text/focus integration |
| Build/diagnostics | Cargo workflows and research/native acceptance evidence | General standalone project export/resource packaging, CPU/GPU timings and useful user-facing errors |

## First implementation increment: persistent prefab composition

`voxy_scene::PrefabSceneDocument` retains authored scene objects, stable asset IDs,
instance identities, nesting and overrides. `instance_object_id` uses length framing
to derive durable child identities independent of source object ordering and runtime
slots. Expansion produces an ordinary SceneDocument; it does not introduce another
runtime graph, simulation store or loader. The old Clone-based Prefab remains a
programmatic template API and does not own persistent prefab documents.

Typed component codecs explicitly declare reference discovery and remapping through
`register_with_reference_remap`. Equal ordinary strings and asset IDs are not rewritten.
Matched source-local object IDs are remapped; unmatched IDs remain external references
and must resolve in the final composed document. Existing reference-only codecs fail
closed if instance references need rewriting. A faulty remapper is rejected.

Instance overrides replace name/activity/TRS and add, replace or remove registered
components. Unknown targets and removal of absent components fail instead of silently
ignoring stale overrides. Overrides are source-local, including derived nested IDs.
The caller keeps authored composition separately from runtime scene capture: capturing
a flat LoadedScene does not reconstruct lost instance links.

All nested assets enter an explicit dependency set. The resolver is supplied by the
caller; asset observations/source revision validation remain the asset pipeline's
responsibility. This initial core API does not yet integrate that resolver into the
native editor catalog. Cycles and depth/instance/object limits are checked during
expansion. The existing graph loader validates final schemas, transforms, parents,
identity collisions and references before publication.

`read_prefab_scene_file` reuses bounded scene-file reads. `save_prefab_scene_file`
validates expansion and uses the existing same-filesystem temporary write/sync/rename
implementation. It saves source links and overrides intact. Failed validation leaves
the previous file intact; post-rename durability errors retain the existing explicit
committed flag.

Reproduction:

```
cargo test -p voxy_scene --lib --tests --examples
cargo run -p voxy_scene --example persistent_prefab
cargo clippy -p voxy_scene --all-targets --no-deps -- -D warnings
```

Validation uses an isolated `/tmp/voxy-goal-scene-target` build directory to avoid
interfering with ongoing unrelated workspace builds. The example passed nested
instances, independent health overrides, target remapping and authoring save/load.
All existing scene tests passed, as did four persistent prefab regressions. Strict
all-target scene Clippy, formatting and focused whitespace checks passed on the
final source, including the additional source-reorder/broken-remapper regression.
No native editor prefab acceptance is claimed yet. Next step: connect composition
and dependency observations to existing editor authoring load/save and transactions.

## Editor prefab publication and authoring increment

The existing editor scene loader now expands PrefabSceneDocument through its
ordinary registry/history/restore pipeline. Scene JSON, logical-ID manifest and
transitive prefab files enter ImportInputs; all captured sources are revalidated
before the candidate reaches the live graph. A failed load retains current graph,
instance identities, history and GPU publication. Manifest model choices exclude
prefab files, so model imports and the resource selector do not treat scene assets
as meshes. Legacy flat scene files outside the model project remain supported.

Scene saves compute property/component overrides against the observed expansion,
reverse only codec-declared object links, preserve nested instance links and reuse
atomic scene file IO. A stale source snapshot rejects saving until reload, leaving
the previous file intact. The resolver used for save consumes captured immutable
prefab documents; there is no second resource catalog or scene graph implementation.
Small authoring transform edits use exact comparison rather than a tolerance.

The native acceptance `python3 tools/test_native_prefab_editor.py` opens the real
editor with a temporary manifest-backed project containing two nested instances.
Presented frame 3 passed edit/save/load/undo/redo and rejected corrupted dependency
publication; frame 6 followed retained presentation and successful dependency
restoration; frame 13 on the final binary followed fixed simulation and Stop restoring authored data.
The saved file contains two instance links and exactly one independent override,
without flattened source objects. This is presented-frame/authoring proof, not a
pixel/image-equivalence claim. Evidence is `native-prefab-editor-2026-10-02.json`.

Validation: 23 editor library tests passed, including actual project file IO,
observed dependencies and failing reload/save retention. Five scene prefab
regressions passed, including inverse reference remapping during capture. Strict
editor/scene library+test Clippy passed; final binary/retests are refreshed after
lint-only source repairs and passed again.

At this earlier increment, structural overrides and composition-aware history were
still pending; the next increment below implements them. Remaining prefab delivery:
editor UI to create/instantiate/revert instances, background source reload using
existing catalog tickets/watch integration, and standalone composition/export. These are incomplete and the full
nine-direction goal stays active.

## Structural prefab overrides and composition-aware history

ObjectOverride now records deletion and an explicit parent wrapper (including a
null parent for becoming a root). Expansion applies these through the existing
scene composition path. Final graph validation still rejects dangling parents,
component references and cycles. The existing editor Delete/Parent commands save
these edits without baking instance children as independent authored objects.

Capture compares edits against instance_baseline: the top-level scene's instance
overrides are cleared for that baseline, while nested asset-authored overrides
remain in their source assets. Captured overrides are recomputed, so undoing a
saved deletion resurrects the linked object and reverting a transform clears the
obsolete override. The baseline is authoring comparison data, not a separately
validated runtime graph; actual publication always validates the final composition.

SceneHistory snapshots now hold optional application-owned serialized metadata
beside the flat document. Composition, source definitions and dependency documents
move together during undo/redo. Metadata-only composition changes create history
versions even if flattened rows match. Document plus metadata count against the
same bounded serialized-byte budget; no GPU resource or worker handle enters
history. Saving refreshes the current serialization metadata without creating a
phantom undo step or discarding redo. A quota check runs before file publication.

Editor saves use the historical composition/dependency snapshot associated with
that history version, not whichever composition happened to load most recently.
Current source observations and historical dependency definitions are checked
before save. Missing/changed historical dependencies reject the save and require
reload, rather than silently publishing stale source definitions.

Tests cover linked deletion/save/undo/resave, metadata-only composition load/undo,
exact property reversal, external reparenting, deletion/resurrection, dangling
reference rejection, bounded metadata, stale transactions and redo preservation.
The native prefab acceptance now includes structural Delete/save/load/undo/save
before damaged dependency retention and presented Play/Stop. Evidence is
`native-prefab-structural-editor-2026-10-02.json`; final checks are recorded below.

Remaining delivery includes prefab asset creation/instantiation/revert UI,
background dependency reload through existing catalog tickets, standalone
composition/export and the other directions in the original goal. These remain
open; this increment is not full completion of scenes/prefabs or the editor.

External parent overrides record their scope explicitly, preventing an external
scene ID equal to a source-local ID from accidentally choosing the instance-local
parent. This collision has a dedicated roundtrip regression.

Final verification of this increment: 24 current editor library tests passed;
seven persistent-prefab regressions and two metadata-history regressions passed,
alongside the existing scene library/integration checks. Strict scene/editor
library+test Clippy, formatting and focused whitespace checks passed. The freshly
rebuilt native binary passed structural save/undo and last-good dependency
retention at frame 3, presented recovery at frame 6, and isolated Play/Stop at
frame 13. Its SHA-256 and saved authoring document are in the structural native
acceptance JSON above. No nine-direction completion claim is made.

## Prefab instance revert in the existing editor panel

The Revert instance button resets the selected top-level linked instance's
property, component, deletion and parent overrides against its captured source.
Unsaved edits in sibling instances are captured and retained. Composition and
expanded graph enter the same SceneHistory transaction, so undo/redo restores
both source links and edits. Save/load continues through the observed-input
composition path; the source prefab file is never modified by revert.

Changed input observations or mismatching historical dependency definitions
reject revert before history publication. Authored flat objects are rejected even
if their opaque ID resembles an instance-derived ID. Deleted objects require
another surviving object of that instance to select; UI representation of fully
deleted instances remains open.

Prefab asset creation and placement UI, background dependency reload,
standalone composition/export, and completion of the other original engine
categories remain open. This is incremental progress, not goal completion.

Verification: all 24 editor library tests pass, including sibling override
isolation and rejection without history changes on damaged dependency. Strict
editor library/test Clippy and focused whitespace checks pass. The rebuilt native
editor renders panels in prefab acceptance mode and verifies the button's region
hit-test before dispatch. Revert/save/load/undo/redo and structural resurrection
pass at frame 3; recovery after dependency failure at frame 6; Play/Stop at frame
13. Binary hash and captured authoring output:
`native-prefab-revert-editor-2026-10-02.json`. No pixel-equivalence claim is made.

## Linked prefab placement in editor panels

The editor exposes a bounded project prefab choice and Place prefab. Manifest
projects discover logical IDs from their existing AssetLocations table; direct
projects discover root-level .prefab files with a bounded directory scan. Choice
refresh is explicit; rendering uses cached choices and performs no discovery IO.

Placement captures current unsaved edits, appends a stable prefab instance ID,
and stages expansion through the same observed-input loader used by scene load.
Existing input observations and historical dependency definitions must still
match. A shared editor document validator checks model references, imported node
indices, world transforms, physics, cameras, materials and lights for both scene
load and placement. The graph, linked composition and dependencies enter the
existing SceneHistory transaction together. Undo/redo and save/load retain links;
failed expansion/validation leaves the authoring graph and history unchanged.

Placement retains the source's root pose. Asset creation UI, richer asset picking,
background dependency reload, standalone composition/export and the remaining
original engine requirements are still open. Placement alone does not complete
the scenes/prefabs category or the overall goal.

Final placement verification: 24 editor library tests and strict editor
library/test Clippy pass. The fresh native binary passes hit-tested selection of
nested prefab, placement/save/load/undo/redo, revert, structural resurrection,
last-good retention and presented recovery, followed by Play/Stop. Placement and
revert pass at frame 3, recovery at frame 6 and Play/Stop at frame 13. Evidence,
binary SHA-256 and saved linked composition are recorded in
`native-prefab-placement-editor-2026-10-02.json`. Focused whitespace checks pass.

## Create a reusable subtree template through editor panels

Create prefab captures the selected subtree as an independent .prefab template,
detaches its root parent, and validates typed references through the existing
scene registry. References outside the exported subtree reject creation rather
than becoming dangling IDs. The original scene/history and source prefab assets
are unchanged. The captured template keeps the current component values and
opaque object IDs; existing prefab links inside the copied subtree are expanded
into the new independent template. Preserving nested source links during export
remains a separate open authoring requirement.

Creation reserves an unused project filename with create_new and never overwrites
an existing asset. Manifest projects register the new logical ID through the same
AssetLocations table and atomic manifest writer; input revision validation rejects
changed manifests. File and manifest publication are separate operations. If
manifest publication fails, the newly written file remains recoverable and its
path is included in the error. Creating an asset does not delete it on scene undo;
placement uses the existing linked-instance SceneHistory operation. Created assets
are selected in the existing prefab picker and can be placed immediately.

Full link-preserving export, background dependency reload, standalone
composition/export and other original engine categories remain open. The overall
goal is not complete.

Creation verification: all 24 editor library tests pass, including asset
registration, unchanged scene/history during creation, template reinstantiation,
save/load and placement undo. Strict library/test Clippy and native binary build
pass. Native creation verification is incomplete: repeated terminal runs,
including a foreground focus attempt, produced zero presented frames and
SkippedOccluded from surface acquisition, then timed out. Diagnostics now report
presented frame count and last surface outcome. The failure record and tested
binary hash are in `native-prefab-creation-editor-2026-10-02.json`; it explicitly
records incomplete native verification. Earlier placement/revert native evidence
remains separate and is not proof of the new creation command. Retry native
creation when surface acquisition permits presentation; other goal work can
continue meanwhile.

## Linked subtree templates

Create prefab now preserves complete selected source instances instead of baking
their expanded rows. Current edits are first captured through the existing
instance baseline/override mechanism. Complete instances and their nested asset
IDs/overrides enter the new template; ordinary authored rows remain ordinary
rows. A selected linked root detaches from an excluded authored parent while
keeping its source link. The original composition and asset definitions remain
unchanged. This supersedes the earlier independent-copy behavior for complete
instance selections.

Before any file is created, the template is expanded through observed project
inputs and compared by object identity against the selected authored document.
Differences in objects, parentage, properties or registered references reject
publication. This also rejects dependency changes that would alter the exported
state and detects reference remapping ambiguity rather than silently changing
links. Partial instance selections currently become independent authored rows;
preserving complete nested branches within a partially selected enclosing
instance remains open.

Tests cover an authored parent with two nested linked instances, independent
translation/name overrides, detached linked-root export, and the existing
creation/registration/reinstantiation/save/history scenario. Current verification
results are recorded after the native attempt below.

Verification: 25 editor library tests pass; strict library/test Clippy and native
binary build pass. The terminal native attempt again presented zero frames and
returned SkippedOccluded, then timed out. Its actual error log and binary hash are
recorded in `native-prefab-linked-export-2026-10-02.json` with an explicit
incomplete-verification status. Native linked-export acceptance remains open;
previous successful placement/revert evidence is not reused as proof of export.
The overall nine-direction goal remains active; other implementation and
verification work can continue independently of this surface state.

## Saved scenes in the standalone executable

voxy_app --model-manifest assets.json mesh --scene scene.json --game launches
saved flat or composed scenes directly in play mode. It reuses the existing
project composition loader, import catalog/worker, SceneSimulation fixed-step
loop and physics/input adapter. The window omits authoring panels, selection
outline, authoring shortcuts and preview spin behavior. Source model imports must
finish before runtime start; failed resources report their logical asset ID and
import error. The scene file is required and is never saved by the game mode.

--game-check runs the same loader/import/start/fixed-step/stop path for 120 steps
without opening a window. It is a CPU/integration diagnostic, not graphical
proof. --game-native-smoke separately requires presented frames, simulation ticks
and GPU residency of required model resources, or fails with surface/timeout
diagnostics. tools/test_standalone_prefabs.py invokes the real executable and
checks nested composition, model import, fixed steps, failure on malformed prefab
and model inputs, and unchanged authoring bytes. Its optional --native requests
actual window presentation and records pass/fail separately.

This is the initial standalone scene path. Resource packaging, general authored
game behavior loading, audio/UI integration and the rest of the original nine
categories remain open. The shared runtime implementation currently resides in
the editor library; moving common ownership into a runtime module without a
second loader or simulation remains an architectural follow-up.

Standalone verification: 26 editor library tests passed, including 120 fixed
steps without preview spin and rejection of authoring shortcuts in game mode.
Strict editor library/test Clippy passes; voxy_app builds (its existing unrelated
library warnings remain). Actual executable checks pass for two nested instances,
120 fixed steps, malformed prefab/model rejection and unchanged scene bytes.
`standalone-prefab-game-2026-10-02.json` contains the final binary hash and outputs.
Startup model failures now close/join owned workers before propagating errors.

The separate native attempt prepared two scene draws but presented zero frames,
returned SkippedOccluded and timed out; its earlier tested binary hash/log are in
`standalone-prefab-native-game-2026-10-02.json`, with native.passed=false. The final
cleanup-only rebuild was verified by CPU execution; graphical verification still
requires an actual presented window. No standalone graphics, packaging or overall
nine-direction completion claim is made.

## Immutable resource package foundation

voxy_assets::ResourcePackage captures root-relative sources through ImportInputs,
revalidates revisions before publication, stores explicit BLAKE3 integrity hashes
and provides the same read(id, byte_limit) contract as FileInputs. The provider has
no filesystem fallback. Paths, source count, payload bytes, serialized document
bytes, format version and hashes are validated. Duplicate paths are rejected both
on capture and during untrusted package decoding. Public construction is limited
to capture/from_bytes so serde decoding cannot bypass integrity validation.

This initial format stores source bytes in JSON and is not a compressed or cooked
asset format. Connecting dependency-closure export and package reads to the
standalone loader is still required. Packaging is therefore not yet complete;
all remaining original engine requirements and graphical verification stay open.

Package verification: all 45 voxy_assets library tests and strict library/test
Clippy pass. New package coverage checks roundtrip/read limits, payload/count/
serialized-document quotas, malformed versions, traversal paths, duplicate JSON
entries, duplicate captured sources, altered payload integrity and input revisions
changing during capture. Focused whitespace checks pass. The package provider is
not yet connected to standalone launch; dependency-closure export and execution
without the original project remain required evidence before packaging can be
considered complete.

## Dependency-closure package export

voxy_app --model-manifest assets.json mesh --scene scene.json --export-game game.vpak
validates the saved composition and imports required models, then collects source
observations from the scene and each model publication. This includes nested
prefab sources, manifest, external glTF buffers/images and explicit model recipe
inputs without a second format-specific dependency scanner. Duplicate observations
must agree. ResourcePackage capture revalidates all sources; captured observations
must still match the validated publications before file creation. A reserved
__voxy_game.json entry records scene path and logical model/manifest identity.
Direct-file sources use the same exporter with a direct-model launch descriptor.
Existing output files reject export and are never overwritten.

Scene load and export share configured_app setup and worker cleanup. Export closes
and joins workers on success/failure. Output uses create_new/write/sync; transactional
crash-safe publication of a new output is still a follow-up. Package execution
without the original project is not yet connected, so packaging is incomplete.
Audio/game-script dependency capture must accompany their future scene integration.

Verification: 26 editor library tests passed, strict editor library/test Clippy
passes, and voxy_app builds with existing unrelated library warnings. The actual
executable checks now use two nested instances, an OBJ and a textured glTF with
external BIN/PNG. The package contains exactly the expected eight source files
plus launch descriptor, preserves every source byte and logical launch identity,
and rejects repeat export without altering the existing file. 120 fixed steps and
malformed prefab/model failures also pass. Binary hash and outputs are in
standalone-prefab-export-2026-10-02.json. No graphical or package-runtime completion
claim is made; the original nine-direction goal remains active.

## Package execution without the source project

voxy_app --game-package game.vpak starts a saved game from a verified package;
--game-check uses the same start/fixed-step/stop diagnostic and
--game-native-smoke separately requests graphical presentation. Package input is
read with a serialized-byte cap and validated for schema, quotas, paths and hashes
before materialization. Launch descriptors reject traversal and root-changing
bootstrap paths. A private owned temporary directory holds only packaged source
bytes. No original project path or fallback source directory enters game loading.

The ordinary project loader/import workers/renderer/game loop run against that
isolated root, so package support does not add a second scene or model decoder.
The temporary project is removed after game exit or error. Window/event-loop errors
now also attempt worker cleanup before returning. Source packages are materialized
on disk rather than streamed directly from the in-memory reader; cooked/compressed
formats and direct package streaming remain optimizations, not claims here.

The actual-executable integration deletes all original scene/manifest/model/image/
buffer files before packaged execution. It also alters packaged model bytes and
requires integrity rejection, preserving the known-good package for subsequent
runs. Graphical verification and the other original engine categories remain
open; evidence follows below.

Verification: 27 editor library tests pass, including invalid launch path/root
rejection and owned temporary-project cleanup. Strict editor and asset library/
test Clippy pass. The actual final voxy_app binary builds and loads the package
with every original source file removed: OBJ, textured glTF with external BIN/PNG,
three objects, two nested instances and 120 fixed steps pass. Modified packaged
model bytes reject with an integrity error. Binary hash and full outputs are in
standalone-package-runtime-2026-10-02.json.

The separate package-native attempt prepared three scene draws but returned
SkippedOccluded, presented zero frames and timed out. The same final binary hash
and native.passed=false are in standalone-package-native-2026-10-02.json. Package
CPU/integration execution is proven; graphical rendering is still unverified.
Other original requirements, including audio/UI integration, general game logic,
remaining graphics/physics features and complete engine diagnostics, remain open.
The full nine-direction goal stays active.

## Owner shutdown and lifecycle failure isolation

Editor App owns simulation plus import/watch workers through one shutdown path.
Shutdown no longer returns early after a simulation stop error: both producers
are closed before either join, and all join results are collected. Lifecycle stop
panics are caught at this termination boundary and retain their text in the error;
worker panic payloads also retain their text. Failed termination does not resume a
partially stopped runtime. Drop invokes the same idempotent cleanup as a fallback
for early returns. Explicit normal shutdown remains the ordinary path; no second
resource owner or scheduler is introduced.

Tests force a foreign simulation/scene rejection and an on_destroy panic, then
verify import/watch handles have both been consumed and repeat cleanup succeeds.
This addresses shutdown ownership/diagnostics only. General game logic loading,
audio/UI integration, complete graphics/physics coverage and the outstanding
native presentation checks remain open in the original nine-direction goal.

Owner-shutdown verification: all 29 editor library tests and strict library/test
Clippy pass. voxy_app builds with existing unrelated library warnings. The final
binary again passes actual-executable source import, dependency-closure export,
execution from package after deleting original sources, 120 fixed steps, malformed
prefab/model rejection and damaged package integrity rejection. Its hash/output
are recorded in standalone-package-owner-cleanup-2026-10-02.json. Focused whitespace
checks pass. No new graphical presentation claim is made; native verification and
the remaining nine-direction requirements stay open.

## Saved behavior and common component capture

The registered game.angular-motion.v1 descriptor now binds to the existing
SceneSimulation fixed-update/lifecycle path. Inactive owners do not advance;
negative angular rates are supported. Invalid axes/rates and rotation writers on
physics owners or their ancestors reject before play starts. Authored behavior
suppresses the editor preview Spin, and stopping play restores the authored scene.
There is no additional simulation scheduler.

Editor capture now uses ComponentRegistry codecs instead of a second manual list
of serializable components. This fixes loss of the new descriptor during scene,
history and prefab capture. The existing ModelInstance-to-editor.model.v1 bridge
still publishes the authoritative logical asset ID; runtime resources remain
outside the saved document.

Verification: 30 editor library and 53 scene tests passed during this change;
the final gameplay run passes 3 unit and 10 integration tests. Strict scene,
gameplay and editor library/test Clippy passes. The actual voxy_app builds and
executes 120 fixed steps with two saved behavior instances, both from source and
from the exported package after deleting all original inputs. The measured final
rotations match the expected one-radian rotation. Dependency closure, malformed
inputs and package integrity checks pass; binary hash and full outputs are in
standalone-authored-behavior-2026-10-02.json. git diff --check passes.

This is one built-in saved behavior, not completion of general game-module or
script loading. Its dedicated inspector editing, general logic binding, remaining
audio/UI, graphics, physics and diagnostics requirements, and native presentation
verification remain open. The full nine-direction goal remains active.

## Behavior inspector authoring

The existing native inspector now offers add/remove angular motion and a behavior
field mode with axis X/Y/Z and radians per second. These actions use the same
authoring commit/history and prepublication validators. Invalid zero axes retain
the previous descriptor; a conflicting physics writer is rolled back through the
same history restore path. Panel authoring remains disabled during play.

All 31 editor library tests pass, including panel-action field editing, negative
speed persistence, invalid-axis preservation, physics-conflict rollback and
remove/undo/redo. Strict editor library/test Clippy and git diff --check pass.
This verifies inspector operations through the actual action handlers, not native
frame presentation. The outstanding graphical check and general game logic,
audio/UI, graphics, physics and diagnostics requirements remain open.

## Registered-component duplication

Editor duplication no longer keeps a second per-type component list. The common
ComponentRegistry copies registered data through staged encode/decode before
changing the destination. Runtime-only components are excluded; durable object
references keep their existing targets. The editor keeps the existing runtime
model ownership bridge and assigns a distinct object identity and copied activity.

All 32 editor library tests and 53 scene library tests pass; strict scene/editor
library/test Clippy passes. The editor regression verifies saved behavior and
inactive-state preservation plus distinct IDs and undo/redo. This addresses
component-copy completeness, not full nested-prefab duplication or completion of
the nine-direction goal. Native presentation and the remaining requirements stay
open.

The additional registry test passes: durable reference targets survive copying,
unregistered runtime state is absent, and stale source rejection leaves the
destination unchanged. This test was compiled after the broad 53-test scene run.

## Transform inspector transaction validation

Numeric transform fields now use the existing commit_authoring transaction instead
of directly committing a document to history. This removes a validation bypass:
unsupported character scale/rotation could previously enter history through the
inspector despite rejection through other authoring operations. Failed edits now
restore the prior authored graph without adding a history entry. Valid translation
continues to support undo/redo.

All 33 editor library tests pass, including invalid character scale and rotation
through actual field-action handlers, unchanged graph/history after rejection,
and accepted translation undo/redo. Strict editor library/test Clippy and
git diff --check pass. These are transaction checks, not native presentation proof;
the full nine-direction goal and remaining integration requirements stay active.

## Durable scene audio descriptors and extraction

The gameplay component registry now includes game.audio-source.v1 and
game.audio-listener.v1. Source data saves a logical asset name, bus, gain, looping,
spatial mode and attenuation range; runtime voices are not serialized. Immutable
scene extraction includes generational owner identity, world position, hierarchy
activity and an optional active listener with world right vector. Inactive sources
remain in the snapshot for future pause/resume reconciliation; deleted sources
disappear. Multiple active listeners, invalid geometry/source data and source
capacity reject explicitly. Editor document validation uses this extraction.

All 33 editor and 5 gameplay library tests pass, with strict gameplay/editor
library/test Clippy and whitespace checks. Audio tests cover registered scene
loading, parent movement, immutable old snapshots, inactive hierarchy, removal,
generation reuse, capacity, invalid gain and competing listeners.

This establishes saved scene data and the extraction boundary only. Connecting
logical audio assets to the existing importer/cache, reconciling these snapshots
with the existing mixer/device, inspector audio editing, package dependency closure
and actual game audio playback remain required. No new device or native-editor
presentation proof is claimed. The original nine-direction goal remains active.

## Scene audio mixer ownership bridge

SceneAudioRuntime now owns the existing voxy_audio Mixer and reconciles immutable
scene snapshots by generational NodeId. It updates spatial gains and source gain,
pauses inactive sources or spatial sources without a listener, removes deleted
voices, and retains completed one-shot sources without restarting them each sync.
Playback identity changes (asset/bus/looping) replace the voice. Clip resolution,
sample-rate, capacity, descriptor, duplicate-owner and spatial geometry checks
finish before playback changes, preserving last-good voices on asset failure.
Bus gain and caller-owned PCM rendering delegate to the existing mixer; no device
callback, new asset registry or simulation scheduler is introduced.

Six gameplay unit tests and strict library/test Clippy pass. The bridge regression
checks actual PCM channels after scene movement, cursor preservation across pause,
last-good playback after missing replacement asset, one-shot completion and owner
deletion. Whitespace checks pass. This remains an offline owner bridge: editor/game
runtime wiring, same-asset revision reload, native output, shared importer/cache and
package dependency integration remain required. The full goal stays active.

## Audio publication revision reconciliation

SceneAudioRuntime accepts publication revision lookup from the caller's asset
owner. A changed publication for the same logical asset replaces playback only
after clip and geometry preparation succeeds. Unchanged publications preserve
cursor and avoid repeated clip resolution. Lookup/decode failures leave the
existing voices intact. The non-versioned synchronize entrypoint delegates to the
same reconciliation path using revision zero; no parallel cache is introduced.
Callers must supply revision and clip from the same immutable publication snapshot.

Seven gameplay library tests and strict library/test Clippy pass. The additional
PCM regression verifies unchanged-version cursor preservation, failed same-ID
reload retaining the prior next sample, failed publication lookup, and successful
new-version sample replacement. Whitespace checks pass. This does not yet prove
filesystem watching, shared importer wiring or actual editor/game device playback;
these and package audio closure remain open in the full nine-direction goal.

## Audio shared import and publication bridge

import_wav_asset now uses the common ImportInputs observation/revalidation path,
bounded WAV decode and existing filtered resampling. AudioImportSettings specifies
target rate and input/frame/filter-work budgets. synchronize_audio_assets captures
immutable clips and publication revisions from the existing AssetCatalog, then
feeds the scene mixer through its publication reconciliation path. It introduces
no second cache or registry.

AssetCatalog now exposes snapshot_with_revision and retains the successful
publication revision separately from current request revision. Dirty, loading or
failed requests cannot relabel last-good data as a new publication. The same
owner operation returns both the revision and Arc snapshot.

All 45 asset and 8 gameplay library tests and strict library/test Clippy pass.
The integration regression publishes decoded WAV through the shared catalog,
verifies scene PCM survives malformed reload with unchanged publication revision,
then verifies successful replacement changes both revision and output samples.
Changing source bytes during import rejects revalidation. Whitespace checks pass.

This is shared library integration; editor/play startup and shutdown, background
file worker/watch wiring, persistent audio cache, package audio dependency closure,
inspector controls and actual device/game playback remain required. The complete
nine-direction goal remains active.

## Editor play-session audio integration

Play now preflights saved audio descriptors and imports distinct WAV assets from
the existing project root/manifest. A generalized shared WAV import helper accepts
prior manifest observations, retaining both manifest and audio source provenance
through final input revalidation. A play-owned AudioPlay holds the common catalog
and scene mixer, with 128 sources, 16 buses, 48 kHz output, bounded input/decode/work
and aggregate decoded-frame limits. Fixed simulation steps synchronize the detached
runtime graph and render PCM; peak/frame counters provide offline evidence.
Stop and App shutdown release the session. Missing audio rejects before play changes
the authored graph/history.

All 34 editor and 8 gameplay library tests, strict library/test Clippy and whitespace
checks pass. The editor regression loads a real temporary WAV, verifies missing
asset rejection before play, 800 PCM frames and expected peak for one fixed step,
then session release and exact authored-document restoration on stop.

PCM is currently consumed for offline validation/metering, not sent to a device.
Startup audio decode is synchronous; background file workers/watch, persistent
cache and aggregate retained-input budgeting still need integration. Audio package
closure, inspector controls and actual standalone/device/native-editor proof also
remain open. This is not completion of audio or the full nine-direction goal.

## Audio package dependency closure

Game export now merges immutable observations from play-session audio publications
with the existing scene/model/prefab observations. Conflicting shared manifest or
source revisions reject before package capture. The ordinary package capture and
final source-byte revalidation include WAV inputs; game-check prints PCM frame/peak
evidence without claiming device output.

Strict editor library/test Clippy and voxy_app build pass (existing unrelated app
warnings remain). The actual executable verification includes two saved looping
audio sources, WAV manifest mapping, 120 fixed steps and exactly 96,000 mixed PCM
frames at peak 0.125. The same output is verified from the exported package after
all original sources are deleted. WAV bytes are checked in the dependency closure
along with OBJ, textured glTF/BIN/PNG and nested prefabs. Existing malformed inputs,
package corruption and saved behavior checks still pass. Evidence and binary hash
are recorded in standalone-audio-package-2026-10-02.json. Whitespace checks pass.
Native output, background audio import/watch/cache, inspector controls and the
remaining original engine requirements are still open; the full goal stays active.

## Audio retained-input budget

Play startup now admits each distinct decoded asset through one aggregate budget:
16 MiB retained source/manifest observation bytes and 480,000 decoded PCM frames.
Repeated manifest observations retained separately by different publications count
separately. Limits are checked before candidate publication; arithmetic overflow
and either quota rejection leave budget counters unchanged. Per-candidate decode
and input bounds still apply, so this retained-data cap is not a claim about all
transient allocations, GPU/device queues or whole-process memory.

All 35 editor library tests, strict library/test Clippy and whitespace checks pass.
The new budget test checks exact-cap admission, independent input and PCM excess,
and arithmetic overflow without counter mutation. Existing WAV play/stop and
scene transaction regressions pass. Native output, background audio import/cache,
inspector controls and the remaining original requirements stay open; goal active.

## Scene runtime native PCM output proof

The existing device queue pump now exposes PcmPump for caller-owned PCM rendering.
MixerPump delegates to this common implementation, retaining its API and mixer
ownership; queue backpressure preserves unsent suffixes and zero budgets preserve
the playback clock. SceneAudioRuntime can therefore render into the native output
adapter without handing its mixer to a second owner.

Two device tests and strict all-target device Clippy pass; strict scene_audio_output
example Clippy passes. The actual native example ran through CoreAudio at 48 kHz:
4,800 scene-runtime frames submitted, 4,800 received by the device callback, zero
callback errors. It then removes the scene owner and stops the runtime. This is
device-delivery evidence for the scene bridge, not an acoustic listening test or
proof of editor/game output wiring. The latter, background import/watch/cache,
inspector controls and remaining full-goal requirements remain open.

## Play-session native output wiring

Editor play with an existing window and standalone graphical game modes now select
native audio output; game-check/export retain offline PCM. AudioPlay owns device,
sender and the shared PcmPump alongside its existing runtime. Device sample rate
drives WAV import and the fixed-step frame budget, with fractional remainder retained.
Queue Full preserves pending PCM without blocking; closed/invalid output and callback
errors propagate. Stop and App shutdown drop the complete session/device.

All 35 normal editor library tests pass. The explicit native App play/stop test
passes on CoreAudio: callback supplied 800 frames, zero errors, then device-session
release and exact authored-scene restoration. Full native-test output is saved in
editor-play-audio-device-2026-10-02.log. Strict editor library/test Clippy and
whitespace checks pass after representing output policy as Offline/Native modes.

Native initialization remains synchronous and this test took 105.71 seconds.
A sampled stack located the delay in AudioUnitInitialize/AudioAnalytics; no
real-time startup guarantee is proven. Nonblocking startup/device ownership,
background import/watch/cache, inspector controls, standalone native output proof
and remaining graphics/UI/physics/diagnostics requirements remain open. This does
not resolve the outstanding native-window presentation check or complete the goal.

## Audio source inspector fields

The existing component-field selector now cycles behavior/audio/transform modes.
For saved audio sources, the inspector edits logical asset ID, gain, bus 0..15,
looping/spatial 0/1 and near/far attenuation. The top resource row becomes the audio
asset text field and shows its pending text. Numeric and text edits use the same
authoring validation/history path as other components. Invalid asset IDs, gains,
fractional buses, toggles and attenuation ranges leave the current scene intact.

All 36 editor library tests and strict library/test Clippy pass. The added regression
uses actual panel/field handlers for all seven properties, verifies saved IDs and
toggles, invalid-value preservation, and asset-ID undo/redo. Whitespace checks pass.
Audio-source/listener creation/removal controls and native visual proof remain
open, alongside nonblocking device startup, background import/watch/cache and the
remaining original goal requirements. The full goal remains active.

## Inspector audio component creation/removal

Audio field mode now exposes add/remove source and add/remove listener actions in
the existing component control rows. Source creation selects the first sorted WAV
logical ID from the existing manifest, or a bounded project-root file discovery;
the asset field allows choosing another ID. No WAV candidates rejects without a
scene mutation. Audio/prefab discovery now share one extension-filtered implementation.
Listener changes use common scene validation and history; a second active listener
rolls back rather than silently replacing the first. Inactive-listener policy stays
defined by the existing scene extractor.

All 37 editor library tests, strict library/test Clippy and whitespace checks pass.
The new regression checks missing-candidate preservation, discovered ID, source
remove/undo, duplicate source, competing-listener rollback and listener remove with
undo/redo. This is authoring/discovery validation, not WAV decoding or native visual
proof. Nonblocking device startup, background import/watch/cache and the remaining
original engine requirements remain open; the full goal stays active.

## Audio artifact cache integration

Audio import now optionally uses the existing ArtifactCache with a build key from
all observed source/manifest bytes, target sample rate, input/frame/filter budgets,
importer version and PCM/filter format version. The editor project enables this
cache under .voxy-cache. Cached clips retain current input observations and finish
through the ordinary source revalidation boundary. Cache absence, envelope errors
or invalid PCM payloads fall back to source decode; cache write failures do not
replace an otherwise valid import with a failure.

VOXYPCM1 encodes bounded stereo normalized f32 frames with explicit rate/count.
Decode rejects malformed length/version, excessive counts, zero rate, nonfinite
or out-of-range samples. Generic cache envelope/key/content integrity checks remain
owned by ArtifactCache; no new resource registry or parallel cache owner exists.

All 15 audio, 9 gameplay and 37 editor library tests and strict library/test Clippy
pass. Regressions cover PCM roundtrip/limits/truncation/NaN, repeated cached imports,
source drift on a cache hit and corrupt artifact repair with unchanged decoded data.
Whitespace checks pass. Aggregate disk-cache eviction/quota, editable persisted
import settings, background/watch integration, nonblocking device startup and the
remaining original goal requirements are still open. The full goal remains active.

## Saved audio import settings dependencies

AudioSource optionally references an import_settings logical asset. Legacy scenes
omit it and retain defaults. Strict version-1 AudioImportConfig JSON saves input,
decoded-frame and filter-work limits; settings cannot exceed engine caps. Project
resolution uses the same manifest/direct SourcePath rules and observes settings
bytes alongside manifest and WAV. This changes the common build key and includes
the settings dependency automatically in package export. Device rate remains the
runtime target, rather than a saved value that could mismatch native output.
Conflicting settings references for the same logical audio asset reject startup.

All 37 editor and 10 gameplay library tests and strict library/test Clippy pass.
Tests cover settings version/unknown-field/cap checks and rejected play with a
configured input limit too small, preserving the authored document. The actual
voxy_app builds and passes source/package execution with the settings JSON in the
dependency closure and all original files removed: 96,000 PCM frames at peak 0.125.
Evidence is in standalone-audio-settings-2026-10-02.json. Whitespace checks pass.
Editing the settings reference/config through a dedicated import inspector,
background/watch integration, nonblocking device startup and the remaining full
goal requirements remain open. No new native-window presentation claim is made.

## Audio settings-reference inspector

Audio inspector now includes a Settings text row for the saved import-settings
logical ID, with pending text displayed during editing. Clearing it restores
default import settings by removing the optional reference from serialized source
data. Both changes use the existing component validation and authoring history.

All 38 editor library tests, strict library/test Clippy and whitespace checks pass.
The added regression exercises field handlers to save an ID, clear to defaults,
undo to the configured reference and redo to defaults. This edits the reference;
dedicated editing/creation of the settings JSON remains open. Native visual proof,
background/watch import, nonblocking device startup and the remaining original
engine requirements are still unfinished. The full goal remains active.

## Nonblocking native initialization owner

OutputDeviceWorker now creates, starts and destroys the native stream exclusively
on one worker. Nonblocking poll returns an OutputConnection with sample rate,
bounded sender and shared counters, never transferring the stream to the scene
owner. close requests cancellation without waiting and suppresses late successful
publication. Explicit final join collects panic failures and waits for ownership
cleanup; Drop uses the same fallback. Native APIs cannot be forcibly interrupted,
so final join may still wait for ongoing initialization.

Four device tests and strict all-target device Clippy pass. A controlled blocked
initializer verifies poll/close return before release, late publication suppression,
and destruction on the owning thread. Initialization failure and repeated shutdown
are checked. The updated scene native example passes through this worker: 48 kHz,
4,800 submitted and callback-supplied frames, zero errors; example Clippy passes.
This establishes the async device boundary. App still needs pending-play state and
worker integration to remove its existing synchronous open path. The original
nine-direction goal and all remaining requirements stay active.

## Pending play and asynchronous native device integration

App now owns OutputDeviceWorker and a pending-play request. Native play requests
spawn initialization and return before device readiness, keeping the authored scene
and simulation stopped. tick polls readiness without waiting and starts play with
the latest validated scene once a connection arrives. A repeated play/stop request
cancels pending play; it sends close without joining in the UI path. Finished
workers are joined before reuse; final App shutdown closes the device worker and
joins it through the existing collected-error cleanup. A three-minute pending
deadline reports failure. The window title exposes preparation and cancellation.

AudioPlay now receives OutputConnection and the common PCM pump; it no longer
opens/starts/destroys native streams. Stop drops the session and requests worker
termination. Device initialization and destruction remain on their owning worker.

All 39 editor library tests, strict library/test Clippy and whitespace checks pass.
Cancellation regression proves pending state does not advance simulation or mutate
scene/history. The explicit native App test verifies pending state before readiness,
automatic play transition, 800 callback-supplied frames without errors, stop cleanup
and exact authored-document restoration. It passed in 41.92 seconds; full output is
editor-pending-play-audio-2026-10-02.log. No native initialization-time bound is
claimed. Final shutdown can still wait for an uninterruptible native API, and WAV
decode on readiness remains synchronous pending background import integration.
Watch/cache quota/settings editing, native visual proof and remaining full-goal
requirements stay open. The full nine-direction goal remains active.

## Shared observed WAV decoder for background imports

`decode_wav_observed` now accepts the caller's `ImportInputs`. The synchronous
cached importer uses this same decoder and retains final input revalidation;
`AssetImportWorker` can invoke it and own final revalidation/publication without
nesting a second import transaction. Cache keys, decoded limits and fallback
behavior are unchanged. A worker test checks decoding on a different thread and
retention of the WAV observation. Editor play still calls synchronous preparation;
connecting asynchronous preparation and cancellation remains required.

## Editor play WAV import runs on the common worker

Native editor/game play now waits for both the device connection and an
`AudioPreparation` using the existing `AssetImportWorker<Clip>` and
`AssetCatalog<ImportedAsset<Clip>>`. Manifest resolution, settings JSON, cache
reads/writes, WAV decoding/resampling and per-import dependency revalidation run
on that worker. The owner polls completions and applies aggregate admission before
publication. Shared dependencies must agree across the prepared assets.

Preparation records logical asset/settings recipes, checks the current scene on
each poll, and discards obsolete work before restarting. Gain/position/activity
changes use the latest graph when playback starts. Cancellation closes worker
queues immediately and retains a single join handle; another import waits for
that worker to finish. UI joins only a finished thread. Shutdown joins outstanding
work. No runtime graph, authoring history or simulation starts before readiness.
Offline game-check/package export retain the same synchronous decoder and catalog.

All 39 editor library tests passed, including background import, settings recipe
change and cancellation after a job was submitted. Strict Clippy for editor and
gameplay library/tests with `--no-deps` passed. The native play test exercised the
asynchronous path: callback supplied 800 frames, errors=0, followed by stop and
identical authoring restoration (`editor-background-audio-play-2026-10-02.log`).
This is native callback evidence, not a graphical presentation or audible-listening
claim. Final shutdown may still wait for native initialization/active file IO.

Fresh `voxy_app` build (`eabca8969a02f859dd14da67225bd694cb33cb8ee0e7dd1594f18c9c8b9650c1`)
passed the standalone/package verifier: both source and packaged runs completed
120 ticks, three objects and two nested instances; both rendered 96,000 offline
PCM frames at peak 0.125. The 11-file package ran after original sources were
removed, and damaged OBJ bytes failed integrity verification. Evidence:
`standalone-background-audio-2026-10-02.json`. App build retains 15 existing warnings.

## Live play audio reload through common observed asset contracts

`AudioPlay` now owns one typed catalog, its import worker and a common source-poll
worker for the play session. Native readiness transfers the existing preparation
worker into this owner. Offline preparation uses the identical project decoder.
Manifest/settings/WAV observations seed `SourceDependencies`; failed attempts retain
published inputs and add failed reads, so creation of a newly referenced missing
file can recover. Tick applies known source invalidations before accepting import
results, calls `complete_observed`, then synchronizes mixer publication revisions.
Malformed audio/settings and missing files report an asset-specific error while
last-good clip and publication revision remain playable. Source scan cadence is
200 ms with one outstanding scan and one outstanding import, bounded source/edge
counts and per-file input limits. Polling cannot observe changes that revert between
scans and does not promise a hard wall-clock IO bound.

`SourcePoller::with_observations` and `SourcePollWorker::new_with_observations` seed
initial fingerprints from completed imports. The first unchanged scan therefore
neither republishes a clip nor restarts a consumed one-shot; changes after import
remain observable. Existing callers that start from unknown fingerprints keep their
initial-invalidation behavior. This extends the existing poller, not a second watcher.

Candidates must fit the play-session aggregate 16 MiB retained-input / 480,000 PCM
frame budget after replacement. The publication admission also checks 960,000 old
plus candidate PCM frames. Import decoding/resampling and watcher scratch have
separate per-import/per-file bounds; that admission is not a total process RSS limit.
Stop closes both queues immediately and collects their handles for later joining.
Native UI waits only for finished retiring workers before another preparation;
headless synchronous startup/shutdown can join active file IO. No audio worker
publishes directly into the scene or owns the native stream.

Validation: 46 asset tests and all 40 editor tests passed, including resident-budget
rejection with unchanged retained Arc/revision; malformed WAV, malformed settings,
source deletion and repairs with actual PCM; unchanged one-shot across scans;
manifest remap to a missing WAV, retained PCM and recovery after creating that
newly observed path; authoring restoration on stop. Strict library/test Clippy for
assets/editor with `--no-deps` passed. Native play callback supplied 800 frames,
errors=0, then stop restored authoring. Evidence is in
`asset-observed-watch-tests-2026-10-02.log`, `editor-audio-watch-tests-2026-10-02.log`
and `editor-audio-watch-native-2026-10-02.log`.

Fresh standalone binary SHA256:
`ac2e75a284bd65cf600b857924d3f34ec8ad3d64f7af339362691671ca75dc50`.
The first verifier process hit its 30-second timeout and was killed; no matching
process remained. Re-running the unchanged binary passed source/package runs with
120 ticks, three objects, two nested instances, 96,000 PCM frames at peak 0.125,
11 packaged files, deleted original sources and damaged-package rejection. Cause
of the initial startup delay remains unproven; no startup latency guarantee or
native graphical presentation is claimed. Evidence:
`standalone-audio-watch-2026-10-02.json`. App build retains 15 existing warnings.

Remaining audio/UI work includes mixer bus authoring, import-settings editing,
cache quotas, additional required formats and native UI/text/focus acceptance;
other original engine directions remain active.

## Saved mixer bus authoring and scene/runtime application

`game.audio-bus.v1` registers `AudioBus { bus, gain }` in the same component codec
registry as sources/listeners. Bus IDs are 0..15, gains finite 0..1. Each active bus
has one owner; inactive hierarchy owners do not override playback. Invalid saved
values and competing active owners reject authoring commit/play extraction. This
is configuration for the existing `voxy_audio::Mixer`, not another mixer/resource
system. Removing/deactivating an authored bus restores unity gain for that bus;
manual runtime bus settings without a scene-owned override remain supported.

`SceneAudioSnapshot` owns bus settings. `SceneAudioRuntime` preflights all bus,
source, clip and revision values before applying any gains, then removes absent
voices and updates source state. Gain changes preserve voice cursors. A failed
clip preflight preserves old bus gain and PCM as well as the last-good clip.

The inspector mode cycle now includes Mixer. Add/remove audio bus and numeric
bus/gain fields use the common authoring commit and undo/redo pipeline. Addition
selects an unowned active bus. Registry-based serialization/duplication captures
this component; duplicating an active owner of the same bus rejects and restores
previous authoring state. Editor tests cover saved JSON load, numeric range
rejection, undo/redo, duplicate rollback, removal undo and play/stop restoration.

All 41 editor and 12 gameplay library tests passed, including actual PCM gain/cursor
and failed-preflight checks. Strict editor/gameplay library/test Clippy with
`--no-deps` passed. Native play with bus gain 0.5 produced runtime peak 0.125,
callback supplied 800 frames, errors=0; stop restored the bus descriptor. Evidence:
`scene-audio-bus-tests-2026-10-02.log`, `scene-audio-bus-native-2026-10-02.log`.

Standalone verifier now saves a bus on the root object and routes both nested
sources through it. Source and packaged execution completed 120 ticks and 96,000
PCM frames with peak 0.0625, rather than the previous unity-bus 0.125. Packaging,
original-source removal and damaged-package rejection remain checked. Binary
SHA256 `6bc1be22dbb7e37e938bf12b515d35d7b13b4b5426fbb455787f54aaad0de0b4`;
evidence `standalone-scene-bus-2026-10-02.json`. App build has 15 existing warnings.

A fresh visual review app was launched separately, preserving existing LOD windows.
UI automation could inspect the older LOD window, but binding the new window timed
out. PID 98560 remained live; samples showed dyld launch preparation blocked in
file open before main, with no engine log yet. Current inspector visual acceptance
therefore remains unproven. See `editor-mixer-window-startup-2026-10-02.log`.
Do not mistake the older visible LOD window for the new inspector build. Import
settings editing, cache quotas, remaining UI/text/focus integration and all other
open original requirements remain active.

## Bounded audio import-settings persistence and shared asset authoring writes

`AudioImportConfig::from_json` now provides the runtime/authoring codec: a 4096-byte
JSON limit, strict fields/version and validated engine limits before accepting the
settings. `AuthoringProject` uses this codec inside the existing observed audio
import. `AudioImportConfig::save_file` validates first, then writes the canonical
pretty JSON through `voxy_assets::save_atomic_file`.

The existing asset manifest saver now calls the same atomic-file writer, retaining
its public location/IO error mapping. The shared writer enforces encoded bytes,
creates a unique sibling with create_new, writes and syncs, closes it and renames.
Any error before rename preserves the old destination; temporary cleanup is
attempted. As with the previous manifest API, trusted path ownership, portable
replacement semantics and directory power-loss durability remain caller/platform
boundaries. This is blocking authoring IO and is not device-callback work. Scene
saves retain their own stronger committed/directory-sync contract.

Validation: 47 asset tests, 41 editor tests and 13 gameplay tests passed. Coverage
includes actual manifest saves through the new writer, settings save/read round
trip, invalid setting preservation of the previous file, JSON byte bounds, failed
rename preserving a directory and no temporary leftovers in the exercised case.
The play audio-watch test now repairs invalid settings with save_file and observes
successful reload on the existing worker/poller pipeline. Strict assets/gameplay/
editor library/test Clippy with --no-deps and focused whitespace checks passed.
Evidence: `audio-settings-atomic-tests-2026-10-02.log`.

The settings editor UI and its transaction/history connection remain required;
these codec/writer APIs do not by themselves satisfy that editor acceptance.
The previously launched visual-review process PID 98560 remained live with an empty
engine log, 12+ minutes after launch. Do not restart it merely because UI observation
timed out; obtain current process/window state first. Its fresh inspector visual
acceptance and all remaining original engine requirements stay open.

## Audio import-settings editor and common undo history

The audio inspector now opens/reloads the selected logical settings asset and
edits the input-byte, decoded-frame and filter-work limits. Integer parsing
preserves values beyond f32 precision. Edits are staged; F5 explicitly saves
settings and F9 reloads them while this inspector mode is selected.

Drafts live in the existing SceneHistory auxiliary snapshot, share its byte quota,
epochs and undo/redo stack, and do not enter scene JSON. Scene/composition edits
preserve the drafts. Undo changes the draft; disk writes occur only on Save.
Observed manifest/source digests reject external edits and manifest remapping
until explicit reload. Digests of this editor's own successful writes allow
undo-after-save followed by another save. This preflight is not an atomic
compare-and-swap against concurrent external writers between checking and rename.
Open/save perform bounded synchronous authoring IO; playback decoding retains its
existing worker pipeline.

Validation: 42 editor and 55 scene tests passed, including common history epochs,
quota rollback, exact integer edits, F5/F9, staged-versus-saved values, undo/redo,
external-change rejection and manifest relocation. Strict library/test Clippy
passed. Evidence: audio-settings-editor-tests-2026-10-02.log and
audio-settings-editor-clippy-2026-10-02.log. The live visual-review process was
still present after 32 minutes; this does not prove the new inspector rendered.
Native visual acceptance and the full nine-direction goal remain open.

The freshly rebuilt voxy_app also passed the standalone verifier: 120 fixed
steps, nested instances, observed model/texture/audio/settings package closure,
execution after deleting source assets, expected bus-mixed PCM peak 0.0625 and
damaged-payload rejection. Evidence:
audio-settings-editor-standalone-2026-10-02.json. The build emitted existing app
warnings; focused scene/editor Clippy remained clean. This is headless runtime
acceptance and does not substitute for an editor-window check.

## Editor panel keyboard focus consumes the existing UI focus owner

The editor now depends on voxy_ui and routes authoring panel Tab/Shift+Tab and
Enter/Space through its existing FocusRouter. PanelFocus binds that router to the
same action rectangles already rendered and hit by the pointer. It is an adapter,
not another focus state machine. Monotonic widget IDs bind scene rows to ObjectId
and fields to selected ObjectId/inspector mode. The adapter retains only current
visible targets (maximum 256), reconciles atomically, and updates a surviving row's
index when hierarchy order changes. Removing a pressed owner, changing inspector
context, or window focus loss cancels activation. Before dispatch, the native
adapter rejects stale hierarchy/selection/mode targets until a frame rebuilds them.

Enter and Space combine into one held activation; releasing one while the other
is held does not click. Repeated events cannot introduce activation after focus
loss. Focus uses a rendered cyan border in the existing panel sprite mesh, pointer
press sets the same focus, and text editing keeps its existing Enter/Escape flow.
Panels show the traversal/activation keys. Standalone/playing input retains its
current game adapter; this increment does not claim a saved game UI system.

Validation: 47 editor and 9 voxy_ui tests passed. The integration case builds real
font-backed panels across every inspector mode and exercises the native key
adapter into authoring duplicate and field editing, combined-key release, repeat
suppression and stale-target cancellation. Additional cases cover stable identity
through row reindexing, transactional invalid orders, mode changes and window
focus cancellation. Strict focused Clippy passed. Logs:
editor-panel-focus-tests-2026-10-02.log and
editor-panel-focus-clippy-2026-10-02.log.

The existing mixer-review process PID 98560 remained live after 40 minutes. CUA
inventory confirmed the running bundle; binding it failed with AppleEvent timeout
(-1712), so it was not restarted and no native visual acceptance is claimed.
The full goal remains active, including actual game interface/text/focus,
remaining physics/graphics/game logic and complete diagnostics requirements.

## Shared panel pointer targeting and Play/Stop input ownership

Panels no longer implement a separate rectangle-search algorithm: their rendered
regions are clipped to the viewport and supplied to voxy_ui::PointerRouter with
the same monotonic WidgetIds used by FocusRouter. Disabled authoring actions
occlude lower regions during Play but cannot produce a target; Stop stays enabled.
The native pointer adapter rejects stale hierarchy/selection/inspector context
before dispatch, matching the keyboard guard. Existing editor pointer actions
still activate on press; this is not a new game UI capture system.

Play/Stop and preparation cancellation now clear field/parenting interactions and
held panel focus before changing input consumers. A Space press captured by an
authoring button cannot click after returning from a game session. Regression
acceptance extends the real-panels/native-adapter test with combined Play/Stop,
unchanged authoring state after delayed key release, shared pointer hit regions,
outside-viewport rejection, disabled Duplicate and enabled Stop while playing,
and stale pointer context rejection before the next rendered frame.

Native window verification remains open. The known mixer-review process PID
98560 was still live after 47 minutes; it was not restarted solely for an
observation timeout. The overall goal remains active.

Final verification for this increment: 47 editor and 9 UI tests passed; strict
editor/UI library/test Clippy and focused source whitespace checks passed. The
voxy_app build succeeded with its existing 15 app warnings. Its standalone
verifier passed source and packaged execution for 120 fixed steps, three objects,
two nested prefab instances, observed source closure, PCM peak 0.0625, original
source removal and damaged package rejection. The gameplay result is headless;
it does not prove rendered UI or a complete game UI system. Evidence:
editor-shared-panel-tests-2026-10-02.log,
editor-shared-panel-clippy-2026-10-02.log and
editor-shared-panel-standalone-2026-10-02.json.

## Saved game UI layout and shared routing foundation

The common gameplay registry now includes game.ui-element.v1. UiElement owns a
normalized screen-space rectangle, color, painter layer, enabled state, optional
bounded named action and optional UiText (logical font asset ID, Unicode content,
size and color). Descriptors reject unknown fields, invalid/nonfinite geometry,
colors, controls and per-element text/action/font-ID bounds. Text is currently a
single directional run; multiline/paragraph behavior is not claimed.

extract_scene_ui builds owned snapshots from scene identities. Rectangles compose
through UI ancestors, including through intermediate ordinary 3D nodes; 3D poses
do not affect screen layout. Every ancestor and the viewport clip the visible
rectangle. Inactive scene ancestry excludes display/input; disabled UI ancestry
propagates disabled controls. Painter order is authored layer then stable slot
order. Admission counts inactive descriptors and limits aggregate text to 64 KiB,
with checked composed geometry and a validated logical viewport up to 16384 pixels.

SceneUiRuntime binds these owned snapshots to the existing PointerRouter and
FocusRouter. Live NodeId generations receive monotonic WidgetIds. Deletion,
inactivation, hiding or action replacement cancels the old capture; changed action
names cannot receive a release captured for their previous meaning. Disabled
visual regions still occlude pointer input. Failed layout/descriptor admission
preserves the last snapshot; callers must handle errors before using a removed
old owner. Events carry the current owner handle and bounded action string, with
no hidden secondary scene/logic store.

validate_game_descriptors now shares behavior/audio/UI preflight between authoring
commits and Play. Stop restores authoring state without validating mutated runtime
UI first. GameCheck consumes the same layout/routing contract and reports
GAME UI LAYOUT counts after checking each visible enabled button's keyboard event.
The standalone verifier fixture now includes saved UI on both nested instances.

Required next integration remains explicit: observed font import/cache/package
closure and reload, Unicode text preparation, GPU UI drawing in native editor Play
and standalone game, native event routing into game UI and logical actions into
fixed-step gameplay, UI authoring inspector and native acceptance. This foundation
and its headless layout checks do not complete the game UI/text requirement.

Validation for saved UI foundation: 48 editor and 18 gameplay tests passed;
strict gameplay/editor library/test Clippy passed. Cases cover registered Unicode
JSON round-trip, nested logical rectangles through a 3D intermediary, ancestral
clipping/disabled/activity, inactive count and aggregate text admission, disabled
pointer occlusion, generation reuse, changed action capture cancellation,
last-good refresh retention, authoring rollback/undo/redo and isolated Play/Stop.

The freshly built standalone verifier passed two saved nested UI buttons in both
source and packaged GameCheck, 120 fixed ticks, existing model/audio/dependency
closure and original source removal. Malformed UI size rejects with
"invalid scene UI descriptor"; corrupted package integrity still rejects. The
fixture has no text/font payload, so this proves no font packaging or text draw.
Evidence: scene-ui-layout-tests-2026-10-02.log,
scene-ui-layout-clippy-2026-10-02.log and
scene-ui-layout-standalone-2026-10-02.json. The full goal remains active.

## Observed UI font import, Unicode preparation and package dependencies

Font import now uses AuthoringProject's common observed project manifest/source
resolver (renamed from audio-specific helpers). Each font read is scoped through
FileInputs and capped at 4 MiB, with at most two observations (manifest plus font).
ImportedAsset<TextFont> captures immutable inputs and finishes through the existing
revalidation contract. decode_ui_font_observed delegates to voxy_text::TextFont;
it introduces no separate parser, shaper or rasterizer.

UiTextPreparation is currently blocking headless GameCheck/export work. It admits
at most eight logical fonts and 16 MiB of retained imported source bytes, including
repeated manifest snapshots and repeated font content stored at distinct paths. The staging peak can include
one additional bounded font candidate. These are input/run budgets, not a claim
about total parser/RSS allocation. Dependencies must agree across imports, and all
font inputs are revalidated after glyph preparation. Inactive text references are
imported and packaged because gameplay may activate those labels later.

prepare_ui_text uses the existing directional Unicode shaper/rasterizer. It
validates snapshot/count/source limits before staging and builds local TextRuns
with 4096 text bytes, 1024 glyphs and a 512x256 alpha atlas per unique run. Equal
logical font/content/size runs share an immutable Arc; their owner, placement,
clip and tint stay separate. Admission caps placed glyphs at 8192 and unique alpha
atlas pixels at 16 MiB. Empty/clipped labels do not produce partial publication.
Missing glyph/font, malformed font or budget failure rejects the candidate; an
existing caller-owned prepared publication stays intact.

GameCheck reports GAME UI TEXT font/run/glyph counts after actual observed font
parsing and Unicode preparation. Export merges the font observations into the
same ResourcePackage closure used for models, audio and prefab documents, rejects
inconsistent revisions, and retains the package's existing write/integrity rules.
The standalone fixture now includes one project-scoped font and Cyrillic text in
both nested instances; it verifies glyph preparation after removing every source
file and rejects an invalid font.

Still open: background native font/text preparation on the existing import worker,
live reload/cache integration, GPU upload/drawing and failure retention, native UI
input into fixed-step logic, UI component authoring, and native visual acceptance.
GameCheck/export font preparation is not native Play rendering acceptance.

Final font/text validation: 51 editor and 18 gameplay tests passed. Strict focused
library/test Clippy passed. The font integration exercises actual scoped manifest
resolution, Unicode combining clusters, nonempty alpha coverage, immutable shared
runs with separate placements, invalid-font candidate rejection retaining old
prepared data, changed-input revalidation failure, inactive-label dependency
retention, count admission before reads and retained input bytes for repeated font
content stored at distinct paths. Manifest itself forbids duplicate bindings of
one source path; the budget fixture respects that existing identity invariant.

The final voxy_app build passed (existing 15 app warnings). Source and packaged
standalone checks both report one font, two text runs and 12 Cyrillic glyphs,
plus two nested UI buttons and 120 fixed ticks. The resource package contains 12
files including fonts/ui.ttf, runs after deleting all sources, and rejects damaged
fonts/UI/package payloads. Focused whitespace checks passed. Evidence:
ui-font-tests-2026-10-02.log, ui-font-focused-tests-2026-10-02.log,
ui-font-clippy-2026-10-02.log and ui-font-standalone-2026-10-02.json.

The live mixer-review process was still present after 73 minutes; there is no new
window/GPU UI visual proof. Full UI delivery and the original nine-direction goal
remain active.

## Shared background UI font/glyph decoder and owner publication

UiTextPlan captures owned scene UI snapshot data and the bounded logical font set.
Its AssetImportWorker callback receives no SceneGraph, window, native stream or GPU
resource. Font reads, parsing, Unicode shaping and atlas preparation run on the
existing generic asset worker. The result type and decoder are shared with the
headless path. AuthoringProject now exposes one worker-project/input-provider
factory reused by audio and UI; its project manifest/source resolver stays common.

Fonts are staged in one derived UI bundle with one ImportInputs owner (maximum
nine observations and 16 MiB), rather than separate per-font observation stores.
Repeated manifest reads use the same captured snapshot. Decoder observations in
UiTextPreparation share immutable InputSnapshot Arc bytes with the ImportedAsset
inputs; successful worker completion revalidates all sources after glyph work.
This replaces the previous per-font repeated-manifest accounting for this bundle.
Per-font read/parser and aggregate glyph/atlas limits remain in force.

GameCheck/export now call prepare_import: submit a scoped catalog ticket, poll the
worker, publish through AssetCatalog, close the producer and join before returning.
This wrapper deliberately waits because these consumers are headless. The raw
UiTextPlan worker API supports nonblocking try_result and close returning a join
handle, but the native Play readiness state has not yet adopted it. Decoder errors
preserve failed observations; an existing catalog publication retains its exact
Arc and publication revision while the failed request is reported.

Native next requirements remain: readiness/cancellation integrated with audio and
Play/Stop; validate the captured authoring recipe and bind/rebind UI owners across
the fresh runtime graph; source invalidation ordering/live reload; owner-only GPU
uploads and drawing; input into fixed-step logic; native presentation acceptance.
Worker snapshots contain generational NodeIds and must not be reused as fresh
runtime identities without that barrier. No native UI/frame proof is claimed here.

Verification: 52 editor and 18 gameplay tests passed; strict editor library/test
Clippy and focused whitespace checks passed. The threaded publication case decodes
Cyrillic glyphs through the real file-backed worker, then rejects a damaged font
while retaining the previous publication Arc/revision and failed read observations.
The fresh voxy_app build and standalone source/package verifier passed one font,
two runs, 12 glyphs, 120 fixed ticks, 12-file closure, source removal and corrupted
font/UI/package rejection. Existing app warnings remain. Logs:
ui-worker-tests-2026-10-02.log, ui-worker-focused-tests-2026-10-02.log,
ui-worker-clippy-2026-10-02.log and ui-worker-standalone-2026-10-02.json.
The original nine-direction goal remains active.


### UI overlay CPU geometry and publication checks

The shared prepared Unicode runs now retain their font/content/size recipe.
The editor builds overlay SceneMesh geometry through the existing SpriteBatch,
with backgrounds and glyphs in scene painter order. Glyph quads intersect the
current ancestor/viewport clip and crop atlas UVs rather than stretching text.
Layout, viewport and tint changes reuse the same immutable TextRun; deleted
owner generations, changed text recipes and missing prepared labels reject
the candidate before publication. This is CPU geometry staging, not GPU
upload or a native rendered acceptance result.

Validation: 54 editor and 18 gameplay tests passed; strict Clippy for both
libraries and tests passed. Focused real-font coverage checks clipped vertices
and UVs, shared run identity after relocation/tint, stale owners/text and
background painter order. The game-check path now also builds UI meshes.
GPU upload/drawing, native action dispatch and the combined editor visual
acceptance remain pending. The original nine-direction goal remains active.

Fresh voxy_app build and the actual standalone source/package verifier passed.
Both runs staged four UI draws and 12 visible glyph quads, two buttons, one
font, two text runs and 120 fixed ticks. The 12-file package ran after source
deletion; corrupt font/UI/package diagnostics and overwrite preservation
remain covered. This verifier stages CPU geometry and does not present GPU
pixels. Evidence: ui-mesh-tests-2026-10-02.log,
ui-mesh-clippy-2026-10-02.log and ui-mesh-standalone-2026-10-02.json.


### Real GPU overlay pixel verification

The UI geometry produced by ui_draw now has an explicit offscreen GPU
acceptance test using the existing SceneRenderer overlay pipeline, common
texture/geometry/transform upload APIs and a real requested GPU adapter.
It uploads the shaped W glyph coverage atlas, clips its geometry to eight
logical pixels in a 128x128 target and reads the actual rendered RGBA bytes.
Pixels outside the clip remain the blue background; covered pixels show the
green glyph, including partial alpha. Transparent atlas regions preserve the
background. No extra UI renderer or pipeline was introduced.

Command: cargo test -p voxy_editor --lib
gpu_overlay_preserves_glyph_alpha_and_clip -- --ignored --nocapture.
Passed: covered=105, partial=54, transparent=407; no GPU validation errors.
This opt-in test requires a real GPU and local TrueType font, and fails rather
than silently skipping when those are unavailable. Strict editor library/test
Clippy passed. Evidence: ui-gpu-pixels-2026-10-02.log and
ui-gpu-clippy-2026-10-02.log.

This proves actual GPU pixel compatibility of the prepared UI meshes. It does
not prove native gameplay UI publication, resource-budget rollback or editor
window presentation. Those integrations remain required, along with the
original nine-direction completion audit. The goal remains active.


### Native gameplay UI publication through the shared graphics owner

UiLive now requests the existing bounded font/text importer from the native
Play/standalone rendering path. It polls without joining active workers in the
frame, rejects completions for changed scene/viewport snapshots, then stages
all overlay geometry and atlas resources before replacing the previous draw
set. Stopped/replaced imports close their producer and keep join ownership;
finished panic collection does not lose unfinished handles. Shutdown drains
all remaining handles.

Atlas textures use the SAME ResidencyCache as imported model images. The
cache accepts validated ImageAsset RGBA output, deduplicates immutable content
and preflights unique incoming mip storage. Model geometry accounting now
includes live UI buffers; UI admission includes both old live buffers and the
new candidate. A failed candidate retains the old publication. Deferred
admission retries when common cache state/budgets change. Native renderer
overlays use the existing SceneRenderer/material.wgsl and identity transform,
with active/generational owner checks. UI appears before editor panels.

Verification: 55 editor tests passed (one GPU test opt-in), strict editor
library/test Clippy passed, raw RGBA overflow/limits/straight-alpha test passed.
The real GPU test uses the editor material.wgsl and the actual UiLive upload
helper: repeated publication shares the same atlas Arc; old geometry plus new
geometry rejects at peak budget; additional image storage rejects at the live
image budget; the retained publication still renders correctly afterward.
Pixels: covered=105, partial=54, transparent=407; no validation errors.

Fresh voxy_app build and tools/test_standalone_prefabs.py --native passed.
The verifier exported 12 package files, deleted project sources, performed the
source/package loader and corruption checks, then opened the native packaged
game. Native output: GAME UI GPU PUBLISHED draws=4; GAME NATIVE PASS frames=18
ticks=7. The native acceptance gate now waits for a current UI publication and
a Presented frame containing it, in addition to all required models and fixed
ticks. No screenshot or editor Play/Stop visual review is claimed by this run.

Evidence: ui-live-tests-2026-10-02.log, ui-live-clippy-2026-10-02.log,
ui-live-gpu-2026-10-02.log, ui-live-rgba-2026-10-02.log and
ui-live-native-2026-10-02.json. Broad render library/test Clippy also ran and
failed with 12 diagnostics in other rendering modules (see
ui-live-render-clippy-limitations-2026-10-02.log); this is not a passing gate.
The app build retains its 15 warnings.

Pending: native game UI action routing into the fixed input lifecycle; source
watch/recovery for changed fonts; avoiding re-shaping when only viewport/tint
changes; visual editor Play/Stop acceptance and the other full original
requirements. All nine directions remain part of the active goal.


### UI activation reaches the existing gameplay input and scene barrier

Native GameUiInput is present in the current worktree and validates the
acknowledged displayed snapshot before authorizing live targets. Its emitted
actions now route through activate_game_ui. Registered application handlers
use the existing UiActionHandlers event queue and SceneCommands barrier;
other named actions use the existing InputMap. No parallel pointer router or
gameplay-input queue was added.

InputMap::activate records a completed digital activation: pressed/released
edges survive render-only frames, physical held values are preserved, focus
loss cancels pending presses, and the fixed consumer clears edges once. The
player map admits its three physical actions plus 128 UI action names per Play
session; UI-only names have empty physical bindings. Stop rebuilds the player
map and cancels UI captures. Capacity failures surface without falling through
to gameplay keyboard handling; pointer release reports operation errors without
closing the application.

An initial integration run found unconsumed edges in scenes with no physics
components. The current worktree handles character.step's input write grant
and clears edges even without a CharacterPhysics instance. Reverification
passed: Enter/Space combined edges, pulse retention over a zero-step advance,
once-per-fixed-step consumption, pointer menu activation and Stop cleanup. A
registered voxy.ui.hide callback retains scene activity until the command
barrier, then deactivates its owner without creating a logical input action.

Evidence: ui-actions-integration-2026-10-02.log (16 UI editor tests passed,
one GPU test opt-in); ui-actions-suite-2026-10-02.log (63 editor, 23 gameplay
and 6 input tests passed in the earlier full run). The focused integration
includes the subsequently added application-route tests. Strict input/gameplay
library Clippy passed: ui-actions-core-clippy-2026-10-02.log. Broad editor and
integration-test Clippy is NOT claimed passing; current focus-ring/graphics
changes and other diagnostics remain in
ui-actions-broad-clippy-limitations-2026-10-02.log.

Fresh app build and source/package/native verification passed again after the
action integration: native UI published four draws and presented 12 frames
with eight fixed ticks, after original sources were removed. Evidence:
ui-actions-native-2026-10-02.json. This is native presentation evidence, not
an OS-input visual editor Play/Stop acceptance. Font watching/recovery, visual
editor acceptance, full gameplay module loading and the remaining original
requirements remain open. The complete nine-direction goal stays active.
