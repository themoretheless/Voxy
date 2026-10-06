# Initial mechanism decisions — pinned source evidence

This is a partial review, not an evaluation of 500 engines. No external code has
been imported. Classification evidence and architectural mechanism evidence are
separate. Runtime/performance superiority remains unproven.

## Bevy: derive safety constraints before parallel dispatch

Reviewed commit `52c3ec0d5ecec0cdf6f4d2fdfd0895267d64740f`,
[`crates/bevy_ecs/src/schedule/executor/multi_threaded.rs`](https://github.com/bevyengine/bevy/blob/52c3ec0d5ecec0cdf6f4d2fdfd0895267d64740f/crates/bevy_ecs/src/schedule/executor/multi_threaded.rs).
Initialization computes pairwise system access incompatibilities and separately
records condition-access conflicts. `can_run` checks running systems, exclusive
access and non-Send execution constraints. Dispatch uses unsafe world access,
with explicit safety arguments tied to those checks. This file alone does not
prove all access declarations are correct; query/parameter construction requires
another review.

**Adapt for Voxy:** access enforcement must cover predicates/conditions as well
as system bodies. Keep worker eligibility separate from dependency readiness.
Cache validated conflict metadata when a schedule changes, rather than checking
string resource names during every worker dispatch.

**Do not copy wholesale:** Voxy's SchedulePlan currently executes serially and
its string declarations do not restrict a closure's actual access. Sending these
closures to workers would not inherit Bevy's safety proof. Require typed/scoped
borrows or physically disjoint owned domain data before enabling concurrency.
Tests must attempt undeclared writes, condition/body conflicts, exclusive access,
structural changes at barriers, worker failures and stale entity use. Benchmark
small-system dispatch overhead and large-system throughput before choosing an
executor. No parallel speedup is claimed by this review.

## Godot: separate high-level scene objects from rendering resources

Reviewed commit `084a2caa05119b625a99b6b51d44b459a26362de`,
[`servers/rendering/rendering_server.h`](https://github.com/godotengine/godot/blob/084a2caa05119b625a99b6b51d44b459a26362de/servers/rendering/rendering_server.h).
RenderingServer exposes a virtual interface using RID resource handles, including
texture creation/update/proxies. It also declares singleton access and render
thread checks. The header identifies interface boundaries, not all resource
ownership, completion or deletion semantics; implementations need separate review.

**Adapt for Voxy:** retain domain-owned resource handles and extracted presentation
data independent of scene hierarchy. Make resource operations explicit at the
renderer boundary, and distinguish CPU handle validity from GPU completion.

**Prefer Voxy's own lifetime model:** per-instance renderer ownership permits
multiple windows/worlds without introducing a new mandatory global singleton.
Validate cross-renderer handle rejection, deletion with queued draws, device loss
and instance isolation before calling this boundary complete. The reviewed
header does not establish a performance problem with Godot singletons; this is a
Voxy ownership choice, not an unsupported condemnation of another engine.

## Fyrox and Lumix: explicit edit groups and ownership boundaries

Fyrox commit `a445c62352682747be85f17e2cda8331a544ad44`,
[`editor/src/command/mod.rs`](https://github.com/FyroxEngine/Fyrox/blob/a445c62352682747be85f17e2cda8331a544ad44/editor/src/command/mod.rs):
CommandTrait separates execute/revert/finalize. CommandGroup executes forward,
reverts in reverse, and finalizes owned commands. CommandStack explicitly stores
undo position and clears the redo branch on new commands. This is source review,
not proof that every concrete command is reversible or failure-atomic.

Lumix commit `54b83b2f30230d7be9cc67b75122b13d0482f29f`,
[`src/editor/world_editor.h`](https://github.com/nem0/LumixEngine/blob/54b83b2f30230d7be9cc67b75122b13d0482f29f/src/editor/world_editor.h):
IEditorCommand exposes execute/undo/merge; WorldEditor exposes begin/end/lock group
boundaries and batch entity transforms. The interface establishes intent; merge
implementations and undo performance require a separate implementation review.

**Adapted in Voxy:** SceneHistory now starts detached SceneEdit previews. All
intermediate gesture changes become one validated commit and one undo entry.
Dropping the preview cancels it without changing current history or redo. An
instance-specific Arc epoch rejects stale/foreign previews, including an
undo/redo ABA sequence returning to the same document. Invalid candidates keep
history intact. Four history tests include these contracts. This borrows the
edit-group mechanism, not foreign command implementations.

**Still required:** compact diffs/inverse operations for large scenes, a real
editor gizmo consuming previews, gesture naming and measured history memory.
Current preview/full history snapshots copy documents; serialized-byte limits
apply to committed history and are not a bound on detached preview allocations.
We do not claim this snapshot representation outperforms either engine.

Fyrox resource review at the same commit:
[`fyrox-resource/src/state.rs`](https://github.com/FyroxEngine/Fyrox/blob/a445c62352682747be85f17e2cda8331a544ad44/fyrox-resource/src/state.rs)
and [`manager.rs`](https://github.com/FyroxEngine/Fyrox/blob/a445c62352682747be85f17e2cda8331a544ad44/fyrox-resource/src/manager.rs)
separate Unloaded/Pending/LoadError/Ok and expose typed request/try_request,
registry initialization and shared synchronized manager state. The examined
state enum includes Unloaded even though its nearby overview lists three states;
classification uses actual code. Voxy retains single-owner publication and
immutable versions; explicit load status is already present. Async waiting,
unloaded-handle residency policy and type/schema mismatch reporting still need
integration. Lock contention superiority cannot be inferred from Arc/Mutex use.

Lumix [`src/engine/resource_manager.h`](https://github.com/nem0/LumixEngine/blob/54b83b2f30230d7be9cc67b75122b13d0482f29f/src/engine/resource_manager.h)
separates type managers from a hub, reload/unreferenced cleanup policy and deferred
load hooks. Adopt the separation of importing and residency policy; Voxy's
captured dependency stamps already protect publication against changed inputs.
This header alone does not prove thread-safety or asynchronous lifetime behavior.

## Halley: separate load deduplication from explicit reload

Reviewed commit `053cb8c725b29ed2daa4a72aa2e953f2b1e98d2a`,
[`src/engine/core/src/resources/resource_collection.cpp`](https://github.com/amzeratul/halley/blob/053cb8c725b29ed2daa4a72aa2e953f2b1e98d2a/src/engine/core/src/resources/resource_collection.cpp).
`doGet` first returns a cached resource, then tracks a per-asset loading claim
under a separate mutex and waits for another loader. Disk loading happens after
claim acquisition. `reload` explicitly constructs a new resource and calls
`reloadResource` on the existing resource, preserving its asset index. `unload`
removes the collection's shared reference and map entry. This file does not prove
all concrete resource reload methods are atomic or all reader access synchronized.
No lock-contention, throughput or reload-latency measurement was performed.

**Adapt for Voxy:** distinguish an idempotent demand for an already available or
in-flight asset from an explicit new import revision. Repeated consumers should
not accidentally supersede each other's work. Retain the existing explicit
request/revision path for reload and changed dependencies; demand deduplication
must not hide changes or return another catalog's ticket.

**Keep Voxy's publication contract:** immutable Arc versions keep consumers on a
stable snapshot while the owner publishes a replacement. Do not replace that
contract with in-place resource mutation merely to preserve identity. Stable
AssetId and a particular published version have different lifetimes. A future
demand API needs tests for duplicate demand, failed-load retry policy, reload
supersession, dependency changes and detached snapshot survival. Halley's blocking
wait path is not adopted for Voxy's frame loop; asynchronous completion and budget
policy remain separate work. No external source implementation was copied.

Implemented adaptation: `AssetCatalog::demand` starts work only for absent assets,
returns Pending for an existing load, Ready with its immutable Arc version, or a
sticky Failed status. It does not authorize duplicate workers. Explicit request
and prepare_import remain the revision/retry/dependency-aware paths. The eight
asset tests pass, including duplicate demand, failed reload, explicit retry and
snapshot survival; strict all-target asset Clippy passes. The audio asset_playback
consumer now requests one background WAV worker, issues a second demand while
pending, retrieves the published clip through demand, and verifies a failed reload
does not automatically retry. Its offline mixer also verifies old/new voice
versions. Example execution and strict example Clippy pass; this does not prove
native audio-device playback or dependency-aware demand deduplication.

## Halley: record secondary importer reads at the IO boundary

At the same pinned commit, reviewed
[`iasset_importer.h`](https://github.com/amzeratul/halley/blob/053cb8c725b29ed2daa4a72aa2e953f2b1e98d2a/src/tools/tools/include/halley/plugin/iasset_importer.h),
[`asset_collector.cpp`](https://github.com/amzeratul/halley/blob/053cb8c725b29ed2daa4a72aa2e953f2b1e98d2a/src/tools/tools/src/assets/asset_collector.cpp)
and [`import_assets_task.cpp`](https://github.com/amzeratul/halley/blob/053cb8c725b29ed2daa4a72aa2e953f2b1e98d2a/src/tools/tools/src/assets/import_assets_task.cpp).
Importer input separates file bytes/metadata from options and target asset type.
The collector's additional-file reader searches source roots, records each found
path and last-write timestamp once, then reads bytes. Import tasks collect these
additional inputs even when an importer throws; generated additional assets are
queued separately from emitted output resources and files.

**Adapt for Voxy:** capture secondary reads through an importer-owned input
provider, instead of relying on a hand-maintained list beside direct filesystem
reads. Return dependency observations on failure as well as success, so a repaired
source can trigger retry. Distinguish input discovery from the published dependency
DAG: discovering a path does not make a decoded resource ready.

**Required stronger boundary:** Voxy's existing revision stamps protect decoded
catalog inputs, not arbitrary files read during import. A filesystem provider must
capture byte snapshots plus content digests, importer/schema version, options and
target configuration for reproducible derived outputs. Timestamp-before-read alone
does not establish that bytes stayed unchanged; no such guarantee is inferred for
Halley. Define bounded input counts/bytes and allowed source roots; detect input
changes before committing dependency metadata and payload together. Atomic output
publication, failure recovery and watcher-driven retries are not proven by these
three reviewed files or by current Voxy asset tests. This mechanism remains a
planned adaptation; no foreign code was imported.

## Godot: keep resource identity separate from mutable source location

Inspected pinned commit `084a2caa05119b625a99b6b51d44b459a26362de`,
[resource_uid.cpp](https://github.com/godotengine/godot/blob/084a2caa05119b625a99b6b51d44b459a26362de/core/io/resource_uid.cpp)
and [resource_uid.h](https://github.com/godotengine/godot/blob/084a2caa05119b625a99b6b51d44b459a26362de/core/io/resource_uid.h).
The implementation stores numeric IDs separately from UTF-8 source paths; add_id
registers a mapping and set_id changes its path. Random creation checks registered
IDs; path-derived creation seeds a generator with project name, lowercase path and
file MD5. That path-derived recipe is not an identity to recompute on every rename.
Cache save/load serializes mappings. These files establish the indirection and
cache mechanism, not complete editor rename behavior or cache crash consistency.

Adapt the stable-ID/location separation through an explicit project-owned
AssetLocations table. Callers issue persistent logical AssetId values; SourcePath
is a distinct validated portable root-relative type. Bind/relocate preflight count
and logical ID/path byte budgets and reject path ownership collisions. Relocation
cleans the old reverse mapping and preserves the logical identity atomically in
memory. Do not generate new IDs from content/path on each reload. No foreign code
was copied, and no performance superiority over Godot is claimed.

Implemented the bounded bidirectional registry and tests for identity preservation,
reverse mapping cleanup, collision/capacity rejection and path syntax. Persistence,
ID issuance, physical file rename, filesystem case/alias handling, importer
location snapshots and source-index relocation remain pending. Source paths in
existing ImportInputs still use AssetId keys via an explicit bridge; the stable
registry alone does not make current native examples rename-safe.


## Godot indexed LOD: vertex ownership and residency are separate

A pinned three-file source review is recorded in
[godot-lod/review.md](mechanisms/godot-lod/review.md), with immutable source URLs
and digests in its sources.json. Adapt separate vertex ownership/index variants;
do not count a draw-LOD choice as released vertex memory. The reviewed RD storage
creates all supplied index variants during surface upload. Voxy still needs shared
vertex allocation ownership, projected-error selection and residency gates before
this mechanism is implemented. This partial review does not alter the corpus's
README/root classification depth or imply complete LOD parity.


## Stride content lifetime: canonical identity and reload boundaries

The user-provided stride3d/stride repository is reviewed at its saved pinned commit
in [stride-content/review.md](mechanisms/stride-content/review.md). Four downloaded
files have digest/license evidence. Adapt canonical identity and explicit resource
dependencies while retaining Voxy's immutable revisions and transactional GPU
publication. Stride's in-place Reload is a different identity contract; no complete
failure rollback, reference-cycle or performance claim is inferred from this review.


## Stride prefab inheritance: member granularity remains required

[The pinned prefab review](mechanisms/stride-prefabs/review.md) examines editor-side
source/instance identity, deletion mappings, insertion after source changes and
transform/component invariants. Retain Voxy's validated immutable publication.
The identified whole-component inheritance gap is now addressed by explicit
object-member overrides, with codec normalization, reference remapping, reset
capture and compatibility for existing whole-component records. Eleven prefab
tests and 71 editor tests pass, including speed override/source-axis inheritance.
Numeric inspector difference indicators and Reset controls now use the existing
history and focus router. Their integration test covers reset/undo/redo, clean
save, stale-source rejection and an intentional legacy-to-member split. Generic
component controls, stable collection-item addressing and native acceptance remain
required. Four pinned source/license files were verified.

The subsequent [generic component inspector](mechanisms/stride-prefabs/generic-component-inspector.md)
adds registered scalar editing through the existing codec/history path. The
[identified collection mechanism](mechanisms/stride-prefabs/identified-collections.md)
adds explicit codec declarations and ID-addressed additions, deletions, member
edits and ordering, with source inheritance and reference remapping. Collection
topology controls and native inspector acceptance remain required. Voxy retains
data publication rather than adopting Stride's second mutable property graph.

## Stride entity processors: eligibility and lifecycle failure

[The pinned processor review](mechanisms/stride-processors/review.md) examines
component requirements, associated-data replacement, reentrant lifecycle calls and
manager Update/Draw dispatch. Adapt explicit eligibility and cache ownership;
retain typed access, generational handles and structural barriers in Voxy.
Callback-failure cleanup and incremental-membership benchmarks remain required;
these source paths do not establish thread-safe parallel scheduling.

## Derived component data: explicit strategies, not mandatory caching

The [local CPU measurements](measurements/associated-data-2026-10-02.md) do not
support universal reuse gains for cheap 64-byte factories. Retain direct typed
requirements for infrequent work, transactional ComponentTable rebuild for cheap
rows, and AssociatedData for identity-preserving/selective construction. Do not
automatically migrate all systems to associated-data caching or promise frame
speedups from these probes. Membership scans and row-table staging remain in both
synchronization paths; component revision checks and Arc ownership add costs.

AssociatedData consumers use revision-validated query/get rather than treating a
failed synchronization's previous snapshot as current. Dependencies outside its
two component types require explicit invalidation. A system depending on hierarchy
transforms, asset versions or other tables must include their changes in that
contract. Any future automatic strategy needs representative costly factories,
change rates, memory peaks and isolated repeated runs with equivalent results.


## Imported LOD error must retain its meaning

The pinned Godot importer uses meshoptimizer's attribute-weighted quadric metric,
then an arbitrary 1.5 growth factor for switching metadata. Its inspected error
path does not prove a maximum surface deviation against the original mesh.
Keep optimizer cost and switching thresholds distinct from a conservative geometric
bound. Voxy's strict pixel target is conditional on that bound; it does not certify
normals, UVs, colors, materials or animation. Any importer using estimated errors
must expose that weaker contract rather than silently promising a hard pixel cap.
See mechanisms/godot-lod/review.md and the reproducible pinned CPU probe.

## Bevy deferred-command storage and ownership

A separate [focused command review](mechanisms/bevy-commands/review.md) pins Bevy
v0.19.1 source and licenses. It distinguishes deferred visibility from immediate
read-only parameter access, early entity allocation from post-barrier handles, and
queue admission from execution transactions. The useful storage-lifecycle property
was adapted as safe Vec draining in Voxy; no packed unsafe code or parallel execution
was copied, and no speedup is claimed. This does not change corpus classification
or establish a whole-engine architecture review.

## Stride animation and skeleton publication

The [pinned animation source review](mechanisms/stride-animation/review.md) examines render-hook clip advancement, instance evaluation scratch, mutable hierarchical world updates, and mesh-specific blend-matrix publication. Adapt explicit ordering and scratch ownership; retain Voxy fixed physical time, immutable checked rig identity, and one staged source pose for skin/contact/supports. These source paths do not establish physics/render clock equality, callback failure rollback, or GPU publication atomicity. Three source files plus MIT license are pinned and digest-verified; no code or secondary evaluator was imported.
