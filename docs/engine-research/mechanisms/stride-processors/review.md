# Stride entity processors: pinned mechanism review

Commit: `a7fa31ced680c7d4a919f1fe233051cf508f6060`. The source manifest covers
EntityProcessor.cs and EntityManager.cs with pinned URLs and SHA-256 digests.
Original MIT notices are retained; see ../stride-content/LICENSE.md.
No Stride build/tests or performance comparison were performed.

## Observed contracts

EntityProcessor declares a main component type and additional required types.
Accept and dependency checks use assignability; dependency-type results are
cached. The generic processor maps main component objects to associated data.
ProcessEntityComponent adds/removes membership as requirements change and replaces
associated data when IsAssociatedDataValid fails. The default validity check
regenerates data and compares it; subclasses may override it.

Adding membership invokes GenerateComponentData and OnEntityComponentAdding before
inserting the data into the map. A per-entity reentrancy set suppresses recursive
addition while those callbacks run. Removing invokes OnEntityComponentRemoved
before deleting the map entry. The reviewed addition branch does not wrap these
callbacks in a finally block; cleanup after thrown callback exceptions needs
separate verification, rather than an assumed rollback guarantee.

EntityManager.Update and Draw iterate enabled processors and invoke their
respective hooks with profiling scopes. Pending processor registration is deferred
until top-level entity addition. These paths do not prove safe parallel execution
or all mutation behavior of collection iteration; collection/manager helpers and
flexible ProcessorManager require further review.

## Voxy adaptation

Adopt explicit component requirements and stable membership-to-associated-data
ownership. Keep predicate evaluation and worker access declarations distinct;
component matching alone is not proof of thread-safe system execution. Validate
cache membership against generational NodeIds, component replacement and inherited
activity, which can change eligibility without deleting the authoring component.

Keep structural changes queued at declared barriers and publish replacement
associated data only after successful construction. Lifecycle failure should leave
the previous valid state or a documented detached state, not a stranded reentrancy
marker. Do not migrate callback-driven scene mutations or reflection assignability
into Voxy's typed Rust component API merely to copy this implementation's shape.

Required verification: dependent component remove/reinsert/replacement; parent
activity changes; stale handle reuse; callback reentrancy and injected failures;
processor removal cleanup; deterministic barriers and predicate/body access
conflicts. Measure full-scan versus cached-membership costs before choosing an
incremental cache. No speedup or complete Stride processor parity is claimed.

## Voxy implementation progress

SceneGraph now exposes `active_components_with<A, B>` for allocation-free borrowed
iteration of owners satisfying two typed requirements and inherited activity.
It reads current graph membership instead of caching object references, preserving
Rust borrow exclusion and generational node validity. Identical component types
produce shared references to the same component, rather than mutable aliasing.

Integration checks cover missing/add/replacement/remove/reinsert requirements,
parent deactivation, slot reuse after deletion, and same-type/stable-order joins.
Associated-data lifecycle, membership caching and parallel scheduling remain
separate work; this API does not claim complete processor parity or speedups.

Validation: `cargo test -p voxy_scene --test component_requirements` passed both
integration tests; `cargo clippy -p voxy_scene --lib --test component_requirements
--no-deps -- -D warnings` passed. `git diff --check` also passed.

`ComponentTable::rebuild_active_with<A, B, E>` now stages a complete replacement
for the current active requirement join at a structural barrier. The factory can
inspect the previous generation-matched row through a shared borrow. Any factory
error drops staged resources and preserves every published row; successful
publication replaces rows and retires deleted, inactive or no-longer-matching
owners. Foreign scenes are rejected before factory invocation. This reuses the
existing dense/sparse storage instead of introducing a reflection processor layer.

The operation rebuilds all matching rows and temporarily holds both generations;
it is not an incremental cache or a bounded-memory streaming solution. Factory
external side effects and interior mutations of shared resources are outside its
rollback guarantee. A failed rebuild preserves a previous snapshot, which must
not be treated as proof of current component values. Existing `query_mut` still
filters dead/foreign generations and optionally activity; consumers requiring
current requirements must publish a successful rebuild before dispatch.

Final associated-data validation: `cargo test -p voxy_scene --lib --test
table_publication` passed 53 library tests and 3 integration tests. The latter
inject a late factory failure, verify old rows survive and staged resources drop,
then verify activity, requirement removal, generational reuse and foreign-scene
rejection. Focused strict Clippy (`--lib --test table_publication --no-deps -- -D
warnings`), formatting checks on the changed table/test files and
`git diff --check` passed. The corpus audit still reports 1,428 pinned identity
records and 500 accepted identities (52 derived); their catalog review depth
remains root/README only, distinct from this bounded Stride mechanism review.

The native Cargo benchmark now probes direct requirements, transactional rebuild
and published dense scans at 1k/10k/100k slots, including sparse requirements and
inactive owners. See `../../measurements/processor-requirements-2026-10-02.md`
for raw evidence and limitations. The measured sparse case supports reusing
published data for repeated dispatch, but reuse must track component values as
well as membership: unrestricted mutable access currently has no revision
contract. No incremental-cache or full-frame speedup is claimed.

Component change tracking foundation: opaque `ComponentRevision` tokens now change
on insertion/replacement, document decoding and obtaining mutable component access.
Read-only access preserves the token; absence returns None; reinsertions receive
fresh process-wide tokens. Equality is the supported comparison; tokens are not
serialized identities. Exhaustion panics before reuse rather than wrapping.
Mutable access marks conservatively even without a write. Interior mutation via
shared references is outside tracking and must be explicitly published through
mutable access. Eligibility still requires current hierarchy activity and NodeId
validation. Incremental associated-data reuse is not implemented by these tokens.
Focused revision test and strict Clippy are running; no completed verification
claim is made for this addition yet.

The initial component revision integration test passed, and focused strict Clippy
passed after updating document decoding. A second regression now checks failed
structural preflight, successful transform-only transactions, missing-type mutable
access and foreign-owner rejection preserve an existing component revision. The
expanded library/integration run and repeat strict Clippy are pending shared Cargo
build access; these new checks are not yet recorded as passing.

Revision foundation final verification: 53 scene library tests and both revision
integration tests passed; repeated strict Clippy passed.

`AssociatedData<A, B, T>` now uses the existing transactional table and immutable
Arc-owned rows to skip factory calls when both requirement revisions match.
Synchronization still scans membership and stages the row table; this is factory
reuse, not an incremental membership journal or a measured scan speedup. Failed
construction leaves published rows untouched. `get` revalidates ownership,
activity, component presence and revisions, hiding stale rows after a failure.
Explicit invalidation hides all rows until successful publication and handles
external dependencies or interior mutation not captured by A/B tokens. Temporary
inactivity between barriers does not imply lifecycle retirement; successful
synchronization while inactive retires that row. Factory side effects remain
outside rollback. New integration and strict Clippy verification are pending.

Associated-data regression coverage now includes a late factory failure after
reusing one published Arc and constructing one candidate: drop counters check
candidate cleanup, preservation of published resources, eventual replacement and
final retirement. Foreign synchronization must not invoke the factory. The test
binary is running; strict Clippy flagged pointer casts in identity assertions,
which were replaced by `std::ptr::from_ref`; repeat verification remains pending.

Both associated-data integration tests passed, including late-failure resource
cleanup, factory reuse, invalidation and generational replacement. The subsequent
change only replaces pointer casts in assertions; repeat strict Clippy is still
waiting for shared build access.

The pointer-assertion cleanup passed repeat strict Clippy. Published associated
data now has a shared `query` iterator backed by `ComponentTable::query`: it scans
stored rows instead of scene slots and revalidates current eligibility and both
component revisions. Foreign scenes are rejected before iterator creation;
invalidated tables yield no data. Shared table and scene borrows prevent mutation
while iterating. Dense row order is storage order, not a durable entity order.
The new regression covers simultaneous revision change, missing requirement,
inactivity, owner deletion and foreign-scene rejection. Its test and strict Clippy
runs are pending; no new dispatch performance result is claimed.

Published query strict Clippy passed. The Cargo benchmark has been extended with
unchanged factory reuse, revision-validated associated iteration and paired 1%
matching-owner churn workloads (full rebuild versus factory reuse). Mutations are
included in both churn timings; ceil(eligible/100) owners change per operation.
The reuse workload asserts its factory count and its final checksum matches the
current graph. These fixtures use cheap integer-copy factories, so reuse overhead
may outweigh construction savings. No performance conclusion is recorded until
the optimized run completes. The expanded integration run and benchmark are live.

Expanded associated-data integration verification completed: all three tests
passed, including published-query eligibility/revision filtering. Focused strict
Clippy for the library and integration test also passed. Benchmark execution and
its separate strict Clippy run remain pending.

The expanded reuse benchmark completed all 84 workload summaries and assertions.
See `../../measurements/associated-data-2026-10-02.md`. Cheap factories did not show
a universal reuse speedup; full rebuild remains an explicit valid choice. Reuse
preserves resource identity and avoids expensive factories, but still scans and
stages membership. Query now compares revisions on its already-validated dense
row instead of repeating owner/activity/table lookup via get. Tests and rerun
measurements for this change are pending; no speedup is claimed.

Benchmark strict Clippy flagged only an oversized probe function; preparation and
associated-data workloads were separated into helpers without changing measured
operations. A combined library/test/benchmark strict Clippy run is pending.
The direct-row query integration binary is running. Strategy selection and the
external-dependency invalidation contract are now recorded in decisions.md.

Direct-row query regression completed successfully: all three associated-data
integration tests passed after the lookup simplification. Combined strict Clippy
for library, integration and refactored benchmark remains pending.

Runnable integration example: `cargo run -p voxy_scene --example
associated_gameplay`. Radius/density requirements derive a disk mass after queued
component changes. Assertions cover unchanged factory reuse, rejection of invalid
radius without stale dispatch, corrected replacement and active-owner retirement.
SceneCommands remains individually validated/ordered, not atomic; the example
checks every command result before synchronizing. The associated-data publication
is a separate transaction. Execution and example strict Clippy are pending shared
Cargo access; no runtime completion claim is made yet.

TableBuildError now implements Display, and implements std::error::Error when its
factory error does, preserving the factory cause through source(). This permits
standard application error propagation without discarding owner/cause context.
The combined strict Clippy, gameplay run and example Clippy remain confirmed live
and waiting for shared build access; no additional pass is asserted.

Combined strict Clippy completed successfully for the scene library,
associated_data integration test and processor_requirements benchmark. The runnable
gameplay example and its focused Clippy still await shared build access.

Gameplay example execution completed successfully with all assertions. Example
strict Clippy flagged exact f32 comparison; the assertion now checks relative
error within f32::EPSILON. Repeat execution and focused strict Clippy are live.
The top-level capability table now lists the shipped requirement/revision/derived
row APIs and retains unimplemented dependency tracking and parallel enforcement
as gaps, rather than claiming complete Stride processor parity.

Gameplay example repeat strict Clippy passed after the floating-point assertion
fix. A new external-dependency regression models immutable asset value replacement:
its owner explicitly invalidates the derived-data consumer, a failed preparation
keeps reads hidden, and successful retry publishes data from the new dependency.
Stable subsequent synchronization must skip the factory. This proves the explicit
invalidation protocol only when the owner notifies correctly; automatic dependency
tracking and asset-catalog wiring are still absent. Expanded tests/Clippy are live.

Gameplay repeat execution also completed successfully after the assertion fix;
all queued-change, failed-build, repair and activity checks passed.

External-dependency regression completed: all four associated-data integration
tests passed, and focused strict library/test Clippy passed.

A concrete cross-crate example (`cargo run -p voxy_scene --example
asset_derived_data`) now connects AssetCatalog immutable snapshots to explicit
AssociatedData invalidation. Failed reload keeps the same last-good Arc and skips
factory reconstruction; successful publication changes the Arc, hides old derived
data, and publishes replacement; removal invalidates the consumer. The previous
strong snapshot remains alive during identity comparison, so pointer reuse cannot
alias these compared versions. Request revisions are not treated as publication
versions. This is an explicit adapter example, not automatic dependency discovery
or a generic engine-wide asset binding. Its execution and strict Clippy are live.

AssetCatalog example execution and focused strict Clippy completed successfully.
Processor retirement now has explicit AssociatedData::clear, backed by
ComponentTable::clear: published rows release immediately without deleting
components, repeated clearing is harmless, and future synchronization has no old
rows. This closes the resource-retention choice when an unavailable dependency
cannot rebuild. Its drop-count/rebuild regression and strict Clippy are pending.
