# Editor batch 03: camera and static 3D project (21–30)

This batch connects the existing scene camera, asset catalog, scene documents and
native renderer to a bounded static-3D authoring workflow. It is not full engine
parity. Further renderer/import/physics limits are explicit below.

| Priority | Implemented behavior |
| --- | --- |
| 21 | Perspective and orthographic viewport through the existing `SceneCamera`; validated durable `editor.camera.v1` descriptor, separate from authoring transforms. G publishes the current view as the single saved camera. |
| 22 | Right-drag orbit, middle-drag pan, viewport wheel zoom, F selected-origin focus, F7 projection toggle and F8 compatibility view. Focus loss/cursor exit cancel gestures. |
| 23 | Near/far unprojected picking segment through the same camera; projected world-axis handles, camera-aware plane/axis translation and existing history cancellation/undo. Panels retain screen coordinates. |
| 24 | Root-scoped static OBJ/glTF/GLB import via observed inputs, bounded geometry/images, external buffer/image dependencies and last-good reload. |
| 25 | H expands imported TRS hierarchy into editable nodes with durable part references. Existing duplication reuses the model resource. Live part compatibility rejects changed hierarchy rather than silently rebinding edited owners. |
| 26 | Base-color factor/vertex colors and PNG/JPEG textures; `editor.material.v1` opaque RGB tint and Lit inspector fields with undo/persistence, texture status. |
| 27 | `editor.light.v1` world-direction/intensity descriptor, first active directional light, GPU flat Lambert shading with world-space face normals and ambient. L adds/removes the descriptor. |
| 28 | Static box transforms accept rotation/scale/shear. Continuous SAT distinguishes empty AABB corners and preserves oblique normals during character sliding. Dynamic characters still require translation-only ancestry. |
| 29 | Opt-in runtime depenetration before character movement; maximum 16 recovery iterations, all-body publication remains atomic, authoring poses are preserved. Existing strict-admission API remains available. |
| 30 | Original textured GLB/OBJ scene, standalone launch, scene persistence, Play/Stop/reload and native acceptance harness. |

## Ownership and architecture

The camera is editor view state; mouse navigation does not mutate scene history.
G explicitly snapshots it into a registered scene component. Renderer transforms
receive camera × extracted world pose. Selection rays and gizmo projection use
that same camera. Panels keep their own identity transform.

Decoded resource publication retains geometry, image payloads and dependency
observations together. IO/decoding stays on the existing import worker; GPU
publication remains on the native owner. Rejected hierarchy candidates retain
observations through `ImportedAsset::into_failed`, keeping last-good resources.
Scene instances store logical asset IDs and durable part indices, separate from
runtime node generations. Containers use `u32::MAX` and draw no model geometry.

The existing renderer uniform's first 144 bytes retain their camera/motion ABI.
An optional tail contains world matrix, tint, light and material flags. Legacy
shaders ignore the tail; editor shading passes its values from vertex to fragment
without expanding the transform binding's stage visibility. UI atlas alpha is
preserved; opaque glTF materials ignore sampled alpha.

Axis-aligned worlds keep the existing swept character backend. Affine static
boxes use continuous SAT against an axis-aligned character, including box faces,
world axes and edge cross axes. Positions stay anchored in f64 during recovery
and motion; published authoring transforms remain f32. Recovery and solver
failure do not consume input edges or partially publish other body poses.

## Bounds and remaining scope

128 scene/imported nodes, 16 buffers/images, 34 observed inputs, 32 MiB aggregate
source budget, 16 MiB per input, 65,536 aggregate mesh vertices and 196,608 indices.
Images are limited to 2048 dimensions and 32 MiB combined decoded RGBA payloads.
Multiple material primitives are emitted as identity children of their imported
node; all declared nodes are imported. The 128-node limit includes these children.
See the example README for unsupported glTF/material features.

Lighting is one directional light, flat double-sided Lambert and ambient; PBR,
shadows, smooth normals, transparency and lighting widgets remain future work.
Focus frames the selected origin, not its complete bounds. Arbitrary rigid bodies,
character contacts, moving platforms, affine step-up and gamepad integration
remain outside this batch. General reusable prefab overrides are not implied by
imported hierarchy expansion.

## Verification

69 focused tests passed: editor 17, gameplay 2 unit + 10 integration, assets 40.
Scoped strict Clippy passed for these three crates with all targets and no
dependency lint escalation. Scene-renderer focused tests passed 6/6; the expanded
`voxy_render` library suite passed 64/65, with the unrelated compute no-op device
equality test failing its `assert_eq!(owner, other)` setup. The full workspace
suite is not claimed.

Final native 3D acceptance: 25 ticks / 36 presented frames. Existing native
gameplay: 34 ticks / 72 frames; OBJ/manifest reload: 269 frames, including
corrupt-source last-good retention, rename, persistence and Stop restoration.
The material WGSL compiled on the native GPU. Python harness compilation and
`git diff --check` passed.

Manually inspected native screenshots: textured cubes/floor in perspective,
expanded scene tree, selected cube outline/gizmo and RGB/Lit/texture inspector.
A separate temporary-copy demo remains open in authoring mode. macOS captures
occasionally showed occluded/stale surfaces; native window zoom refreshed the
surface before visual verification.

The runnable project and controls are in
`crates/voxy_editor/examples/scene3d/README.md`.

## Import/render follow-up

Static glTF meshes now accept multiple material primitives. Raw source node
indices remain stable; material children inherit their parent transform. Repeated
source mesh primitives share GPU geometry and selection outlines. Matching image,
sampler and mip configurations share texture bindings within a model revision;
different sampler configurations still allocate separate GPU textures.

All six glTF minification modes preserve independent base and mip filters.
Mip-filtered images upload their generated mip chain; non-mip modes retain a
single level. Existing renderer callers keep their previous default behavior.
The no-op compute test now checks foreign device contexts and accepts clones of
the owning device, preserving the actual device ownership boundary.

Editor tests: 18/18; full renderer library suite: 65/65, including ownership and
invalid anisotropic mip-filter validation. Strict editor Clippy with all targets
passed. Native baseline:
25 fixed ticks / 59 presented frames. Two-material native variant: 25 ticks / 60
frames. Both verify GPU sharing/mip publication, camera picking, persistence and
Play/Stop/reload. The variant is generated in a temporary fixture by
`python3 tools/test_scene3d_window.py --multi-material`.

## Shared image storage across sampler variants

The renderer can create a material binding over an existing image allocation,
validating device ownership and the exposed mip count before creating GPU objects.
A model revision uploads each referenced image once, with a mip chain when any
material needs it. Base-only and mip-filtered materials then use distinct views
and samplers over that same allocation. Identical configurations still share their
complete binding. The cache remains scoped to immutable model publication; this
does not claim cross-resource content-addressed GPU residency.

The native two-material fixture now gives the second material nearest/base-only
sampling while the first uses mip filtering. Acceptance requires identical GPU
texture identity across these distinct configurations and passed 25 fixed ticks /
59 presented frames, including camera picking, persistence and Play/Stop/reload.

Follow-up validation: renderer library tests 65/65, strict editor Clippy on all
targets passed, scoped rustfmt and `git diff --check` passed. Ownership and
unavailable mip-range rejection are checked before GPU publication.

## Cross-resource image residency

GPU image storage is now shared across model resources in one graphics context.
The versioned BLAKE3 key covers decoded RGBA pixels and dimensions under the fixed
RGBA8 sRGB/mip policy. Each image allocation has a complete generated mip chain,
including base-only consumers, so later mip consumers reuse the same allocation.
Material views keep their authored level restriction. This adds the usual mip
storage overhead to base-only images; it avoids mutable allocation upgrades.

Each published model revision retains strong image owners. The context cache
stores only weak references and prunes dead entries on publication, preserving
last-good revisions and releasing allocation ownership when their models drop.
This is context-local residency, not a process-global cache or GPU device sharing.
Native `--shared-images` acceptance passed 25 fixed ticks / 60 presented frames
and verifies image identity across independent GLB resources before persistence,
Play/Stop and resource reload.

Cross-resource validation: editor library tests 19/19, including last-owner
residency release; strict editor Clippy on all targets, scoped rustfmt, Python
harness compilation and `git diff --check` passed.

## Scene-owned GPU residency

Before drawing, the editor reconciles GPU models with scene ModelInstance
references. Unreferenced models are dropped and dead weak image entries pruned.
Returning references rehydrate GPU geometry and bindings from the existing CPU
catalog; file import is not required. Inactive scene references retain residency
so activation is immediate. CPU catalog/history retention is a separate policy.

Native shared-image acceptance now removes the final reference to a second model,
asserts its GPU entry is gone, restores the reference and asserts rehydration.
It passed 25 ticks / 36 presented frames. Existing OBJ/manifest acceptance passed
135 frames, including Delete, Undo/Redo, instance independence, persistence,
corrupt-source last-good retention and source rename. Editor tests passed 19/19;
strict editor Clippy, Python harness compilation and `git diff --check` passed.
A GPU byte budget and admission/eviction under pressure remain unimplemented.

## GPU image admission budget

The graphics context now admits unique decoded image storage under a configurable
byte ceiling (256 MiB default, `VOXY_GPU_IMAGE_BUDGET_BYTES` override). Preflight
includes complete mip chains, deduplicates candidate content keys and counts live
weak-cache allocations once. It runs before candidate GPU geometry/image creation.
Replacement admission includes the old model's live images, preserving the peak
needed for last-good publication; shared images need no additional bytes.

Rejected candidates leave existing GPU models available; missing models defer
upload and can retry after scene-owned residency is released. Error diagnostics
are deduplicated during retries. Geometry, bindings, driver overhead and submitted
GPU work awaiting reclamation are outside this logical image budget. CPU catalog
publication remains separate from GPU residency. Pressure-driven LOD/streaming and
comprehensive GPU memory accounting remain future work.

Image-budget validation: editor tests 19/19 and strict editor Clippy passed.
Native acceptance with `VOXY_GPU_IMAGE_BUDGET_BYTES=84` passed 25 fixed ticks /
60 presented frames. Two independent resources share the exact 84-byte 4x4 mip
allocation; a different-image candidate is rejected while those bytes are live.
Eviction/restoration, persistence and Play/Stop/reload continue to pass.
Scoped rustfmt, Python harness compilation and `git diff --check` passed.

## Deferred upload scheduling

A deferred upload now records a weak source-revision identity and the image cache
state (membership epoch and configured budget). Unchanged state skips repeated
candidate preflight each frame. New source publication, image insertion/pruning,
or a changed budget permits retry. Membership changes trigger retry even when
aggregate byte usage happens to remain equal. The deferral does not retain CPU
source revisions. Preflight hashes each referenced source image once, then
coalesces equal content keys across different image indices.

Deferred-upload validation: editor library tests 20/20. Native shared-image
acceptance at 84 bytes passed 25 fixed ticks / 62 presented frames, including
budget rejection, image sharing, eviction/restoration and Play/Stop/reload.

Strict editor Clippy on all targets and scoped rustfmt / `git diff --check` passed
for deferred-upload scheduling.

## Geometry allocation accounting

`SceneGeometry::allocation_bytes` reports the actual sizes of all five geometry
buffers: vertices, indices, normals, material coordinates and material parameters.
The metric includes retained capacity and buffer alignment; CPU normal/coordinate
caches, driver overhead and pending GPU reclamation are excluded. Model accounting
counts shared primitive/outline handles once plus aggregate fallback geometry.
This supplies the measured basis for admission policy; a geometry byte ceiling
is not yet enforced.

Native geometry accounting measured 31,632 unique logical GPU buffer bytes versus
49,696 bytes when repeated instance handles are counted repeatedly. The scene
passed 25 ticks / 59 presented frames at the existing 84-byte image limit, including
budget rejection, sharing, eviction and Play/Stop. Strict editor Clippy passed.
Editor files passed scoped rustfmt and `git diff --check` passed. Whole-file
renderer rustfmt reported concurrent material-layer changes outside this work.

## Geometry admission

Model uploads now preflight their logical geometry bytes under a separate
256 MiB default ceiling (`VOXY_GPU_GEOMETRY_BUDGET_BYTES`). The renderer supplies
the mesh buffer-size calculation; model preflight includes fallback geometry,
selection outlines and unique primitive allocations. Live model byte totals
include the old revision during replacement. Admission happens before creating
new geometry/image buffers, preserving prior models on refusal.

Deferred uploads observe both live geometry bytes and the configured geometry
limit, in addition to image residency. Releasing model residency or changing the
limit permits a retry. UI/gizmo allocations, driver overhead and submitted work
awaiting reclamation are excluded; referenced-model eviction/LOD remains pending.

Native geometry admission passed 25 fixed ticks / 58 presented frames. It verifies
that predicted geometry bytes equal all actual uploaded buffer sizes, rejects an
additional model at a fully occupied geometry limit before buffer creation, and
continues existing shared-image, eviction and Play/Stop acceptance.

Geometry-admission validation: editor tests 20/20, strict editor Clippy on all
targets, scoped editor rustfmt, Python harness compilation and `git diff --check`
passed. An intermediate parallel skinning const-constructor compilation failure
was already corrected in the current source before these successful reruns.

## Activity-driven eviction

GPU model residency now follows effectively active, renderable scene references.
Inactive ancestors hide all descendants; active imported containers without mesh
parts do not retain a model when all leaf geometry is disabled. Eviction releases
model geometry and unshared images while retaining authoring/CPU catalog data.
Reactivation uploads from the CPU catalog. Import completion also avoids uploading
models with no active renderable reference. This supersedes the earlier policy of
retaining every inactive scene reference.

This is activity-driven eviction, not frustum/occlusion streaming or referenced
visible-model LOD under pressure. Repeated activity toggles may cause uploads;
residency hysteresis and asynchronous GPU publication remain future work.

Native activity-eviction acceptance passed 25 fixed ticks / 62 presented frames.
It verifies model removal on deactivation, rehydration on activation, existing
shared-image and geometry-budget acceptance, and authoring restoration on Stop.

Activity-eviction validation: editor library tests 21/21, including inherited
activity and active-container/hidden-leaf residency. Strict editor Clippy on all
targets, scoped rustfmt, harness compilation and `git diff --check` passed.
