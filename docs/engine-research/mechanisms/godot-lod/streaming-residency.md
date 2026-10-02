# Optional LOD index residency

`SceneLodGeometry` now supports a certified streaming constructor which uploads
only base geometry and shares the importer's immutable `Arc<CertifiedLodIndexSet>`.
Existing eager constructors retain their behaviour. Vertex, normal and material
streams remain immutable and shared; only optional index-buffer residency changes
through exclusive access to the bundle.

The owner can calculate `desired_level_for_camera` independently of residency,
then call `SceneRenderer::ensure_lod_level`. This validates device identity, level
and buffer limits, and admits the index bytes against an available bundle budget
before allocating. Rejection leaves all resident buffers and accounting unchanged.
An already resident level is a no-op. Index payloads come from the retained verified
certificate, so a caller cannot accidentally upload a level from another revision.

Selection and history report the actual resident level. If the desired level is
missing, selection uses the nearest resident finer level, retaining the certified
geometric-error bound. Near-plane fallback still uses base geometry. Streamed
optional levels can be evicted and reloaded; base eviction is rejected. Accounting
counts shared streams once and only currently owned index buffers. It is logical
buffer residency, excluding driver/in-flight retention and allocation overhead.

The resource owner must gather every instance/view requirement before eviction,
subtract other resources from the global budget, and retain old-plus-new revision
admission. The editor now uses base-only certified upload, collects desired levels
for every unpartitioned instance in all current render views before mutation, then
reconciles the shared model resources. Required sets are unioned by asset identity;
one instance/view cannot evict a level requested by another. Part geometry
keeps its existing separate storage and does not request whole-model LOD.

Reconciliation plans admission before allocating or evicting optional levels.
Mandatory residency includes base, outline and part buffers plus game UI,
editor-panel and gizmo geometry. Candidates are considered in rounds: one desired
level per model before a second level for that model, starting with its coarsest
requested level. Within a round, greater index reduction per byte wins; equal
candidates prefer already resident levels, then stable asset identity. Optional
levels outside this admitted set are evicted before missing admitted levels load.
This also reconciles a reduced budget instead of preserving all requested buffers
regardless of capacity. Deferred requests remain pending and selection uses a finer
resident fallback. Base buffers remain pinned even below mandatory residency.
Reload retains the old-plus-new revision peak admission.

This policy distributes sufficient capacity across models before extra levels,
but does not guarantee eventual admission when capacity cannot cover all models.
It has no age rotation or view importance weighting. Animated LOD, native
streaming/multi-view acceptance and multi-view temporal composition
remain open.

Editor smoke checks validate the exact sum of base/outline/part allocations and
certified index sizes for resident levels rather than requiring every level to
be resident.

The no-op-device regression covers base-only accounting, shared CPU certificate
identity, shared GPU streams, exact index-budget admission, rejection preserving
base, resident-load no-op, pinned base, repeated eviction, finer fallback, reloading
and rejection of a foreign device. This validates resource semantics, not physical
GPU memory release or native frame presentation.

Verification: 33 renderer library tests matching `lod` pass, including camera,
history, certification/archive and new streaming-resource regressions; one GPU
test is ignored. Editor library compilation, formatting of the changed renderer
module, whitespace and four-owner/32-package boundary checks pass. Logs:
`/tmp/voxy-lod-streaming-final.log` and
`/tmp/voxy-lod-streaming-final-editor.log`. This does not prove editor streaming
integration or native GPU presentation.

Initial editor integration verification (before priority planning): 82 editor library tests pass (two GPU tests
ignored); application library/binary compilation and boundary/whitespace checks
pass. The new no-op-device regression holds two required levels for one model,
defers another model at the exact global budget including unrelated storage,
evicts an unrequested level and admits the deferred request, then evicts/reloads
optional levels while retaining both bases. Logs:
`/tmp/voxy-editor-streaming-final-tests.log` and `/tmp/voxy-editor-streaming-app.log`.
This provides resource/admission regression proof, not native visual acceptance.
Exact accounting verification also passes in the 82-test editor run at
`/tmp/voxy-editor-streaming-accounting-tests.log`; the regression rejects missing
outline storage rather than accepting a numeric allocation range. The application
check is at `/tmp/voxy-editor-streaming-accounting-check.log`. After adding panel
and gizmo geometry to the global live ledger, editor compilation passes at
`/tmp/voxy-editor-streaming-ui-accounting.log`. Transform uniforms, textures and
driver/in-flight overhead are outside this geometry-buffer ledger; image admission
retains its separate existing budget.

Priority planning verification: all 82 editor library tests pass (two GPU tests
ignored). The global regression now requires both models to receive their first
requested optional level before a second level is admitted. It covers eviction
after reduced demand, complete optional eviction, full-budget restoration,
budget contraction with unchanged demand, repeated reconciliation and recovery.
Log: `/tmp/voxy-lod-priority-editor.log`. Native presentation and physical GPU
release remain unverified.

The priority renderer check completed: 33 LOD tests passed, one GPU test was
ignored (`/tmp/voxy-lod-priority-render.log`). Application library and binary
compilation passed (`/tmp/voxy-lod-priority-app.log`, existing 15 warnings).
The insufficient-capacity regression also passed: an equally useful resident
level on a later-sorting asset beats a new allocation, and zero configured
capacity evicts optional levels while retaining mandatory bases and recording
both missing requests (`/tmp/voxy-lod-priority-pressure.log`).

The efficiency extension passed the three focused GPU-model tests
(`/tmp/voxy-lod-priority-efficiency.log`): with one optional index buffer of
capacity, a certified six-index to three-index reduction on the later-sorting
asset replaces an already resident zero-saving level on the earlier asset.
This verifies efficiency precedes residency ties using real certified geometry.

Initial multi-view gathering foundation (before editor wiring): `gpu_model::collect_lod_requests` accepts
instance/view requests with each view's camera, physical viewport and previous
selection. It unions asset/level requirements without mutating geometry or
selection history, before the exclusive reconciliation step. At that stage the one-view editor used this path; simultaneous rendering and
per-view transform ownership were added by the editor integration below. Editor library compilation and architectural boundary checks passed
(`/tmp/voxy-lod-view-union-check.log`).

Three focused GPU-model tests passed (`/tmp/voxy-lod-view-union-tests.log`).
The extended regression uses one certified quad shared by orthographic views at
different scales: their union retains levels 0 and 1 independent of view order,
reconciliation fits the sole optional buffer, and independent view histories
repeatedly select six versus three indices. This proves request/selection
semantics, not simultaneous viewport presentation.

Multi-view encoding foundation: `SceneRenderer::encode_views` records disjoint
physical-pixel regions in one single-sample color/depth pass. It clears once,
sets viewport/scissor per region, and uses the same opaque/transparent/X-ray/UI
recording path as single-view encoding. SceneView borrows geometry and textures;
transforms remain separately owned per view. Empty/overflowing/out-of-bounds or
overlapping rectangles are rejected before recording. Color/depth dimensions
and sample counts are checked; callers supply base-mip views. Overlapping views
need independent depth and composition rather than shared depth.

The no-op-device region/recording test passed
(`/tmp/voxy-disjoint-views-tests.log`) and renderer compilation passed
(`/tmp/voxy-disjoint-views-check.log`). At this foundation stage the API was not connected to native editor layout,
input or per-view state; editor wiring is described below. Temporal composition
still remains open.

Explicit real-GPU acceptance passed for multi-view encoding
(`/tmp/voxy-disjoint-views-gpu.log`): every pixel in a 64x32 offscreen target
matches a 16-pixel-wide red overlay view, a 16-pixel black gap and a 32-pixel-wide
blue opaque view. Both views borrow one geometry buffer with separate transforms.
This proves one view does not clear the other and that oversized geometry remains
within its viewport. The existing real-GPU LOD sharing/count/pixel test also
passed (`/tmp/voxy-disjoint-views-existing-lod-gpu.log`), validating the shared
recording-path refactor for single-view encoding. Application library/binary
compilation passed with the existing 15 warnings
(`/tmp/voxy-disjoint-views-app.log`). Offscreen GPU proof does not close native
window presentation or editor multi-view integration.

Window composition foundation: `SceneSurface::render_scene_views` acquires one
frame and delegates to `SceneRenderer::encode_view_frame`. View-local content
is followed by a full-target overlay pass so editor panels/game UI do not inherit
the last view's viewport/scissor. Global draws must be overlays. Acquisition
skips and device/surface errors use the existing custom-frame lifecycle; only
successful presentation publishes the frame ID, and single-camera temporal
inputs are invalidated. At the initial composition stage, MSAA and X-ray world views returned the typed
`UnsupportedViewConfiguration` error. MSAA support was subsequently added below;
Separate-depth X-ray composition was added subsequently below.
Those composition paths and native editor acceptance remain open. Renderer library
compilation passed (`/tmp/voxy-view-frame-check.log`).

View-frame acceptance passed: both targeted tests, including the explicitly
enabled real-GPU test, passed (`/tmp/voxy-view-frame-tests.log`). The same target
is rendered without global UI and then with a green top-eight-row band covering
all 64 columns, including the gap and both view regions. All 2048 pixels per frame
match the expected values; this confirms global UI does not inherit view-local
clipping and does not clear the views. The no-op test rejects a non-overlay global
draw before encoding. Application library/binary compilation passed with 15
existing warnings (`/tmp/voxy-view-frame-app.log`), as did boundary/whitespace
checks. Actual multi-view window acquisition remains unverified while native
window presentation is blocked. These checks preceded the editor wiring below.

Editor side-by-side integration: F4 toggles two views while authoring. The
current camera is retained; a second camera starts with the alternate projection.
Pointer selection, wheel zoom and camera-drag start choose the physical view;
changing active views swaps the active/inactive camera values without altering
either view's camera state. Full-window panel/game UI hit handling precedes
view-local editing. Picking and gizmo hits receive local coordinates and that
view's dimensions; an object drag retains its viewport origin so subsequent
physical cursor events cannot jump the object. Changing views cancels a pending
object drag. F4 collapsing to one view retains the active camera. Layout/cameras
remain editor presentation state, separate from authoring history/documents.

Rendering prepares both view matrices before GPU borrowing, owns transforms and
LOD histories by `(view, NodeId)`, and unions all instance/view requests before
reconciling shared model residency. Gizmo transforms are separate per view;
geometry, outlines and textures stay shared. Disappearing view/owner state is
pruned. Two views use `render_scene_views` and whole-window UI composition; the
one-view/Play/standalone path retains `render_scene`. View regions and pointer
hits share a physical-pixel layout, including odd widths, DPI-scaled dimensions
and narrow/suspended windows. No native visual acceptance is claimed: new editor
binary presentation, multi-view temporal composition, animated LOD and
per-view persistent layout remain open.

Editor integration verification: all 84 editor library tests passed, with two
GPU tests ignored (`/tmp/voxy-editor-split-final-tests.log`). The new App-level
regression enables F4, selects the right view, changes only that camera, starts
and previews an object drag in local coordinates without a jump, cancels it by
switching views, and collapses to the active camera without changing the authored
document. Layout tests cover physical boundaries, scaled/odd widths, invalid
cursors and tiny sizes. Application library/binary compilation passed with 15
existing warnings (`/tmp/voxy-editor-split-final-app.log`). Four-owner/32-package
boundary and whitespace checks passed. This is code/headless integration proof;
it does not certify native multi-view presentation or interactive visual quality.

Multi-view four-sample support: `encode_views_msaa4` uses the same validated
region/recording path as single-sample views, clears four-sample color/depth once
and resolves all views together. `encode_view_frame_msaa4` then composes full-
window UI at single-sample resolution using the separate overlay depth target.
The surface chooses the matching path when its MSAA targets are enabled. Invalid
sample counts, mismatched target dimensions and missing four-sample pipelines
fail before recording. At that stage X-ray and per-view temporal composition
remained open; X-ray support is described below.

The editor now exposes smoothing on F10 independently of split views (F4).
`SceneRenderer::enable_msaa4` stages pipelines from its currently published shader
and layouts, rejecting a foreign device and publishing only after validation.
It retains single-sample resources and caches the four-sample pipelines, so
repeated enable is a no-op. Disabling surface MSAA drops optional targets; either
quality change invalidates temporal accumulation. The preference is presentation
state and does not modify authored documents. Unsupported enable errors use the
existing recoverable keyboard-operation error path.

Initial real-GPU MSAA acceptance passed (`/tmp/voxy-multiview-msaa-gpu.log`): two
views, the empty gap and global UI retain the same exact pixel values with one
or four samples, while both diagonal view edges gain fractional coverage only
in the four-sample case. Current editor verification: 84 tests passed, two GPU
tests ignored (`/tmp/voxy-multiview-msaa-editor.log`); the App test exercises F10
and document preservation alongside F4/camera/drag routing. Application library/
binary compilation passed with 15 existing warnings
(`/tmp/voxy-multiview-msaa-app.log`), as did architectural boundary/whitespace
checks. Native editor smoothing and multi-view presentation are still unverified.

Final real-GPU acceptance passed both targeted tests
(`/tmp/voxy-multiview-msaa-final-gpu.log`). Before enabling MSAA, the test uploads
geometry, textures and transforms and publishes a shader that swaps red/blue
channels. All six single-/four-sample frames (plain views, global UI and diagonal
edges) preserve that shader's pixel behavior, proving enable uses current shader
state and compatible retained resources. A foreign device is rejected, repeated
enable is a no-op, shader revision remains unchanged, and malformed sample
targets are rejected without poisoning subsequent encoding. This remains
offscreen GPU evidence, not native F10/F4 visual acceptance.

Isolated multi-view X-ray: `SceneViewTargets` supplies the borrowed world color/
depth, optional resolve, distinct X-ray depth and single-sample overlay depth.
`encode_view_frame_targets` validates regions, target sizes/sample counts, depth
formats and nonaliasing before recording. When internals exist it records world
opaque/transparent content, then internals with their own cleared depth, then
view-local and full-window UI. One- and four-sample modes use the same stage
recorder; four-sample color is retained/resolved between passes. World depth is
never cleared by internals/UI. Opaque internals self-occlude independent of draw
order; transparent internals still need caller sorting. Lower-level single-pass
`encode_views` retains legacy stage semantics; callers needing self-occluding
X-ray use the composed target API. Existing frame entrypoints delegate to the
common target implementation.

The surface lazily owns one extra depth target per active sample mode, shared
spatially by disjoint views rather than allocating it per camera. Resize replaces
that target with the other attachments. Native editor multi-view presentation
now uses this path; it no longer rejects X-ray geometry. The old unsupported-view
error variant was removed. Per-view temporal composition, animated LOD and native
visual acceptance remain open. This does not add an editor X-ray material control.

Initial acceptance passed three targeted tests, including both explicitly
enabled real-GPU tests (`/tmp/voxy-multiview-xray-gpu.log`). Two views submit near/
far internals in opposite orders behind a closer opaque wall; both retain the
near internal color with single- and four-sample targets and global UI. Readback
also proves every single-sample world-depth pixel remains at the wall depth.
Missing or aliased X-ray depth fails before commands and does not poison later
encoding. Existing multi-view/MSAA/current-shader pixel regressions also pass.
All 84 editor library tests pass (two GPU tests ignored), and application library/
binary compilation passes with 15 existing warnings. Logs:
`/tmp/voxy-multiview-xray-editor.log`, `/tmp/voxy-multiview-xray-app.log`.

Final acceptance passed all three targeted tests again
(`/tmp/voxy-multiview-xray-final-gpu.log`). The legacy single-view X-ray path now
uses the same draw-stage recorder, and its explicit GPU color/world-depth
readback case passes alongside the composed one-/four-sample view cases. This
checks compatibility after consolidating layer recording; it does not prove
native window presentation. Formatting of changed renderer/test modules and
whitespace checks pass.

Native direct-entry observation (2026-10-02): the isolated
`native_editor_review` example launched as the actual `CFBundleExecutable`, with
the same binary name and safe standard-library exec for log redirection. Frozen
binary SHA256: `f6eca7b8ae9f2999d9cb7de5d6d8a98045fb8e10c7b4bdd2a80ebf7049cab09c`.
Project/log retained at
`/var/folders/jx/ps1h62vx3gdgly_ddwly4mqr0000gn/T/voxy-native-editor-review-96506-1790967332332833000/native.log`.
The log records `Presented` and NSWindow occlusion state 8194, and native window
screenshots visibly show the quad, selection outline, transform gizmo, scene
panel and inspector. Later observations alternate with state 8192 and
`Occluded(true)`. F4 attempts did not visibly change the layout or produce the
accepted-key trace. This proves a basic native editor frame, not interactive
split-view/MSAA acceptance or native LOD streaming: the quad has no LOD chain.
It does not establish the launcher as the cause of earlier blank frames;
matched launch controls remain necessary for that attribution. Opt-in input
tracing now records authoring keys rejected by the existing unpresented-panel
guard, without weakening that guard. The frozen reviewed binary predates this
additional diagnostic.

Diagnostic review binary SHA256
`9522ab6363123be94f28dd0ee48018c3a7c83829b6bccbaf9bb40c60e5c559b0`
was built successfully and launched in a separate direct-entry bundle. Its log
at `/var/folders/jx/ps1h62vx3gdgly_ddwly4mqr0000gn/T/voxy-native-editor-review-6272-1790968059171031000/native.log`
records `Presented`, followed by `Occluded(true)` and both
`EDITOR KEY REJECTED frames=121 key=F4 reason=panels-not-presented` and the
equivalent F10 rejection. Thus those attempts delivered physical keys but were
blocked by presentation admission. Native screenshots again show the basic
editor frame; split/MSAA interaction remains unverified. The harness now accepts
`--model PATH --smoke`, forwarded unchanged across direct-executable reentry;
smoke requires an explicit certified recipe and uses existing presented-frame
LOD acceptance. For argumentless LaunchServices entry the explicit bundle
environment `VOXY_REVIEW_MODEL` and `VOXY_REVIEW_LOD_SMOKE=1` feed the same parser.
Projects/logs remain isolated and retained. This harness extension alone is not
a successful native LOD run.

Current native static-LOD acceptance passed with the extended direct-entry
harness, binary SHA256
`78ccdf1403f70e0a5a18a8ea2d615359254f4cd3d2367256aed0bce26db174f4`.
The explicit recipe `artifacts/lod-acceptance-2026-10-02/model.vmodel` selected
coarse level 1 (256 triangles) at presented frame 3 and base level 0 (512
triangles) at frame 6. Setting the geometry budget to zero rejected ordinary
retry/publication with `live=31812 additional=31004 limit=0`, retained the exact
previous base geometry, and passed presented-frame resident-byte validation.
At frame 9 the restored budget had cleared the deferred admission and the
renderer had presented subsequent frames. The existing smoke state machine
exited normally on success; PID 8488 is no longer running. Retained raw evidence
and fixture/binary hashes are in
`artifacts/lod-acceptance-2026-10-02/native-direct-review.log` and
`native-direct-report.json`; previous reports are preserved.
This closes native static near/far and failed-admission recovery for this
fixture, not manual split/MSAA acceptance, visual equivalence, animated LOD or
measurement of physical GPU memory. Streaming under competing models and
independent views still needs native acceptance.

Native split-view LOD acceptance subsequently passed on the same certified
fixture, binary SHA256
`2c25d57466217271654c0f0c5965799278b9143607cdab6e7a3f3ce7b7f5b4d2`.
The extended existing `LodSmoke` state machine first repeats failed-admission
and recovery checks, then invokes ordinary F4/F10 editor actions between
presented frames. At frame 13 two 640x1360 regions choose `[0]` and `[1]`; at
frame 16 MSAA4 preserves both choices; swapping the camera distances produces
`[1]` and `[0]` at frame 19. Model geometry remains 34076 logical bytes in all
three split phases: views share residency instead of duplicating the bundle.
At frame 22 disabling MSAA4 and collapsing the views restores base selection
and verifies that no secondary-view LOD history survives. The successful smoke
exits normally and PID 11927 is no longer running. Retained evidence:
`artifacts/lod-acceptance-2026-10-02/native-split-review.log` and
`native-split-report.json`.
This proves native composition and independent per-view selection for one
asset with automated editor actions. It does not prove manual keyboard/pointer
delivery, image equivalence, competing-asset admission, animated LOD or per-view
temporal effects. Those remain separate acceptance work.

Editor regression validation after extending the native smoke passes all 84
library tests (two ignored GPU tests), recorded in
`/tmp/voxy-native-split-lod-editor-tests.log`. The test process initially waited
before test execution (sample showed only `_dyld_start`), then completed normally;
it was not restarted. Native review example compilation, changed-file rustfmt
checks and whitespace validation also pass.

Native competing-asset acceptance passed (2026-10-02), frozen binary SHA256
`7a546bc243a10f06916d675333ecfb5eb861b2972102637cdbae96deb116e348`.
The harness accepts an explicit manifest asset and copies a supplied scene seed
into its retained isolated project. Two separate logical model resources A/B
use the same certified topology fixture, with distinct GPU bundles. After the
existing near/far, failed-upload and split/MSAA phases, presented frame 24
confirms both coarse levels resident. Mandatory geometry is 62816 bytes; each
optional level is 3072 bytes. Contracting the ordinary residency budget to
65888 bytes admits A and defers B. At presented frame 27 total live geometry
equals that limit exactly, B draws base level 0, and exactly one request remains
pending. Each model's resident allocations are checked against its imported
certificate, and the whole graphics ledger must equal recorded live bytes.
Restoring the original 268435456-byte budget admits B again: frame 30 reports
68960 live bytes and no pending requests. PID 18373 exited normally after the
successful smoke. Raw log and fixture/binary hashes are retained in
`artifacts/lod-competition-2026-10-02/native-review.log` and `native-report.json`.
This closes native budget competition/fallback/recovery for two resource
identities on this fixture. It does not measure total physical GPU memory,
certify manual input, animated LOD, image equivalence or different-topology
priority tradeoffs. The latter policy cases remain covered by separate CPU/
no-op-device regressions.

Regression validation for competing-asset smoke: all 84 editor library tests
pass, with two GPU tests ignored by the default library invocation
(`/tmp/voxy-native-lod-competition-editor-tests.log`). The native example build,
changed-file rustfmt checks and whitespace checks also pass. The successful
native run above separately exercises real window presentation and MSAA.

Animated LOD quality foundation (2026-10-02): `SkinnedLodMesh` retains one shared
immutable skeletal mesh and complete bidirectional variant witnesses. Static
rest-pose error is not reused as a deformation guarantee. `prepare` evaluates
the existing CPU skinning position calculation for the exact palette/model,
then re-verifies those witnesses over the resulting world-space f32 positions.
The immutable `PreparedSkinnedLod` binds pose, positions, monotonic error envelope
and full deformed bounds together. Failed preparation never changes a previous
snapshot. Preparation shares skeletal attributes and index variants; work is
linear in vertices and retained witnesses, without nearest-surface searching.
Import validation temporarily copies variants through the existing static
certificate constructor. Runtime cost has not been benchmarked.

Static and skeletal camera selection now use the same
`LodPolicy::select_for_camera`, preserving projection validation, near-plane
base fallback and per-view caller-owned hysteresis. The posed API can explicitly
bake a selected level into existing scene geometry with identity transform.
This verifies the CPU-skinned geometric surface for one pose, not all future
animation, normals/UVs, image equivalence or GPU floating-point identity.
It does not yet connect animated index residency and selection to GPU skinning,
editor import/play or topology-aware temporal history. Animated LOD remains open.

Two new regressions cover a vertex driven off an initially covered surface by
one bone: rest-pose error is near zero, posed error exceeds one world unit,
the near view refines and the far view remains coarse. Model scale amplifies
the bound; snapshots share their source, retain exact palettes and reject
invalid palettes/projective transforms/overflow and malformed witnesses without
damaging last-good data. Near-plane crossing and invalid previous levels are
also checked. Final shared-policy validation passes 35 LOD tests (one GPU test
ignored), including static camera/residency/last-good regressions, in
`/tmp/voxy-skinned-lod-shared-policy-tests.log`. Earlier focused two-test log:
`/tmp/voxy-skinned-lod-tests-fixed.log`. Changed-file formatting, whitespace and
production dependency boundaries pass (4 owners, 32 packages).

Skeletal GPU index residency and renderer integration (2026-10-02): the existing
resident animated-mesh renderer now accepts `upload_skinned_lod`, optional
`ensure_skinned_lod_level`/`evict_skinned_lod_level`, pose-specific selection and
atomic `update_skinned_lod_pose`. One vertex, joint-palette and object buffer
serve all levels; only optional index buffers are allocated. CPU `SkinnedMesh`
vertices/indices are immutable shared arrays, so temporal meshes for different
index topologies share skeletal attributes as well. Existing plain skeletal
upload clears LOD metadata only after a successful replacement.

Admission checks the old-plus-new skeletal allocation peak for replacement and
current-plus-new index bytes for optional loads before allocation. Missing LODs
fall back to the closest resident finer level; base is pinned. Optional eviction
of an active level first binds base and resets topology-dependent correspondence.
Stats count shared skeletal buffers once plus resident indices, excluding other
renderer allocations, attachments, driver and in-flight resources. This is
resource admission, not a second global priority planner or proof of physical
GPU release. Editor-wide animated admission is still to be integrated.

Atomic pose updates verify the new pose and select a resident level before
writing palette/model buffers. Same-level updates preserve motion history;
index-topology changes replace its indexed mesh and invalidate temporal outputs/
camera history. Independent legacy palette/model updates verify posed data for
LOD sources and conservatively bind base. Renderer selection derives camera and
viewport from the actual renderer configuration, sharing orthographic extents
with its shader uniform calculation. After successful surface acquisition it
re-evaluates the stored policy, covering camera movement and resize without
advancing last-presented pose history on an occluded/timeout frame.

The no-op-device test validates exact bytes, base-only admission, foreign-device
rejection, optional quota rejection/load/no-op/eviction, shared GPU streams and
shared CPU attributes, active-level protection, old-plus-new rejection and
topology-sized correspondence reset. A real GPU test submits five frames through
the actual skeletal pipeline: rest base and rest coarse match color/depth on the
test fixture, moving one bone increases base depth, restoring the pose restores
the original frame, and optional eviction recovers base. Vertex/palette/object
buffer identities stay unchanged; validation scopes report no error. These
readbacks cover this fixture, not arbitrary image equivalence or a numerical
error bound for GPU skinning arithmetic.

Final skeletal validation passes all 11 filtered tests, including the explicitly
enabled real GPU test, with zero ignored:
`/tmp/voxy-skinned-lod-gpu-final-tests.log` (36.59s). Earlier dedicated real-GPU
log: `/tmp/voxy-skinned-lod-real-gpu-tests.log`. Changed module formatting,
whitespace and production dependency boundaries pass. Animated editor import/
Play, multiple resident animated instances/views, global animated residency and
native editor presentation remain open; this does not mark animated LOD complete.

Application library/binary compatibility check passes after this integration
(`/tmp/voxy-skinned-lod-gpu-app-check.log`, 38.79s, 15 existing warnings).

### Animated import and instance pose boundary

Editor glTF/GLB import now retains the existing `ModelAsset` skeleton and clips,
with a bind-pose preview. VMODEL accepts a skeletal glTF base and preserves the
subdivision witnesses needed to recertify each deformed pose. The bounded archive
parser is shared with static LOD; source positions must match bit-for-bit and base
indices must match exactly. Rest-pose metadata is never published as static LOD
for a skeletal asset. External glTF buffers and the certificate participate in
ordinary dependency observation and last-good publication.

Validation: 85 editor library tests pass (2 ignored), 37 filtered renderer LOD
tests pass (2 ignored), and 4 model tests pass. The observed import test corrupts
and repairs a certificate through the actual worker/catalog loop: corruption
retains the previous complete asset Arc, repair publishes a new revision.
Logs: `/tmp/voxy-editor-animated-lod-import-regressions.log`,
`/tmp/voxy-skinned-lod-archive-tests.log`,
`/tmp/voxy-animated-model-regressions.log`.

`ModelAsset::sample_pose` provides explicit clip selection and finite-time
validation without shared mutable playback state. Each owner can evaluate its
own clock and instance transform before pose certification. This API does not
connect Play to the GPU skeletal pipeline: the editor's separate SceneRenderer
still renders the imported bind preview. Multiple animated GPU owners, editor
global animated admission, temporal integration, and native animated presentation
remain required. The imported triangle test has identical base/optional topology
and proves publication/pose identity, not triangle reduction.

The checked sampling addition passes all 5 model tests and the extended observed
editor import test (1 pass, no ignored). The latter evaluates two owners at
different times/transforms and compares their certified positions against the
existing CPU skinning path before exercising corruption/recovery. Logs:
`/tmp/voxy-model-sampled-instances-tests.log` and
`/tmp/voxy-editor-sampled-lod-instances-tests.log`. Changed-file formatting,
whitespace, and production dependency boundaries pass.

### GPU deformation for the editor scene renderer

`SceneSkinner` now provides a compute deformation pass into the existing
`SceneGeometry` vertex/normal streams. `SceneSkinSource` shares immutable skin
attributes across owners; `SceneSkinInstance` owns its palette and output
buffers. The result is borrowed through ordinary `SceneDraw`, so this introduces
no separate scene material, shadow, viewport or MSAA rendering pipeline.
Deformation must be encoded before drawing, and commands using an instance must
be submitted before another palette update. Instance transforms remain in
`SceneTransform`; imported palettes already contain node hierarchy transforms.

Source and instance admission count logical buffer bytes before allocation.
Shared source bytes must be included once in the caller's live ledger; instance
bytes include ordinary scene buffers, palette and parameters. This remains
logical ownership accounting, not physical driver/in-flight memory measurement.
Unsupported compute devices return an explicit error and can use the existing
CPU pose-baking path. Invalid poses are checked before queue writes; bind-space
material coordinates remain fixed. Instance bindings belong to their creating
skinner, preventing incompatible implicit pipeline layouts from reaching GPU
submission.

The real-GPU test compares two independently sampled instances against CPU
positions and checks UV/color preservation, quota rejection and shared source
ownership. The extended test draws GPU and CPU outputs in adjacent viewports
through the actual scene pipeline: pixel pairs match and the triangle covers
more than 50 pixels. Invalid palettes preserve the previous output.
The extended real-GPU acceptance passes (1 test, no ignored) in
`/tmp/voxy-scene-gpu-skinning-final-tests.log`; 9 scene regression tests pass in
`/tmp/voxy-scene-skinning-scene-regressions.log`. Subsequent final checks cover
bind-coordinate preservation and foreign-skinner rejection.

This is the scene renderer integration primitive, not completed editor playback.
Remaining production rig work includes scene-owned playback/transition settings,
binding to Play's clock and lifecycle, skeletal LOD index views and global
admission, temporal pose history, unsupported-import diagnostics, and native
multi-character acceptance. Existing Animator crossfade/root-motion facilities
must be audited and integrated rather than duplicated. Bone masks, layered
animation and rig tooling require their own end-to-end acceptance.

Final current-source real-GPU verification passes with foreign-owner rejection,
bind-coordinate preservation, unchanged output after invalid palettes, and
side-by-side scene pixel equality: 1 pass, zero ignored, 38.18s in
`/tmp/voxy-scene-gpu-skinning-owner-final-tests.log`. Full editor library regression
passes 85 tests with 2 ordinary GPU tests ignored in
`/tmp/voxy-scene-skinning-editor-regressions.log`. Changed-module formatting,
whitespace and production dependency boundaries pass. No native animated editor
or CUDA hardware acceptance is implied by these results.
