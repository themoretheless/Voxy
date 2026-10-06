# Stride animation publication and skeleton ownership

Pinned repository: `stride3d/stride`, commit
`a7fa31ced680c7d4a919f1fe233051cf508f6060`. `sources.json` records
three source files plus the original MIT license with SHA-256 digests.
This is a focused source review; Stride was not built or benchmarked.

## Observed behavior

`AnimationProcessor` assigns order -500. Its Draw hook dispatches component
work through `Dispatcher.ForEach`, advances enabled animation times using the
render context's warped elapsed time, builds/computes blend operations, and
applies the result through an instance-owned AnimationUpdater. Crossfade weights
and removal of finished clips follow evaluation. Operation lists are pooled;
component removal releases evaluators and intermediate results. The file alone
does not establish failure atomicity or Dispatcher cleanup after exceptions.

`SkeletonUpdater.Initialize` uses the skeleton node-array reference as its
identity check, retains transformation capacity, and copies initial local TRS,
parent indices and flags. UpdateMatrices traverses nodes in array order and uses
already available parent world matrices. The inspected path does not validate
arbitrary parent ordering or cycles. Rendering eligibility and negative-scale
state propagate through the hierarchy; world override flags bypass composition.
This is a mutable hierarchy contract, not an immutable pose revision.

`ModelRenderProcessor.Draw` checks mesh/material changes and updates render
records. It copies mesh-specific blend matrices, world transforms, bounds and
negative-scale state into each material-pass render mesh. These are separate
render records; this source does not establish a transactional GPU palette upload
or that physics and rendering consume exactly the same pose clock.

## Voxy decisions

Adapt instance-owned evaluation scratch, explicit dependency order, parent-first
hierarchy evaluation, and retaining validated source mesh/resource ownership.
Do not adopt render-hook elapsed time as the authority for physical supports:
Voxy's fixed physical time must produce one admitted pose used by source skin,
contact staging and support targets before publication. Display may interpolate
that state without advancing the physical clock.

Retain immutable rig identity and checked Pose64 evaluation rather than accepting
mutable node-array identity as sufficient revision validation. Keep the existing
SceneSkinner source/instance ownership and candidate publication; introducing a
second mutable skeleton graph would duplicate authority. Retaining capacity or
pooling scratch is a future measured optimization, not a claimed speedup.

Current evidence: startup sampling is covered by native imported-pose tests;
three-step full-volume support checkpoints exactly match the native clip's source
positions. The two-second native capture and contact regression suite are still
running. This does not prove complete fixed/render interpolation, all GPU skinning
paths, anatomical attachment, full-clip physics, or Stride feature parity.

## Remaining verification

Review Stride ModelComponent/AnimationUpdater and Dispatcher implementations
before claiming complete phase order, bounds lifecycle, pool failure cleanup or
parallel safety. In Voxy, qualify paused/resumed play, variable display rate,
multiple independent instances, stale/foreign pose rejection and device recovery
against one physical clock. Use existing owners and APIs rather than adding a
parallel evaluator or render-specific animation player.
