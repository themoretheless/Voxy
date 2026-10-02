## Partially covered primary frames

`DepthMotionPass::with_jittered_depth` now separates the exact primary raster
camera from current/previous unjittered motion cameras. Depth reconstructs with
the jittered inverse; both motion projections use unjittered world coordinates.
The relative-world proof now rasterizes with clip jitter (0.125, 0.125) and checks
a 2x previous-camera X scale: foreground UV motion is independently expected
-0.4375/-0.1875 by column, with zero background. This catches jitter leaking into
motion during projection changes. Metal and Linux Vulkan/OpenGL pass; all four
Linux temporal probes also pass. Native DLSS evaluation and jittered skeletal
overlay coverage still require further integration/verification.

`PreviousPositionPass::new_relative` stores previous correspondence as offsets
from a finite caller-supplied origin; `position_origin` exposes it for
`PrimaryMotionPass::for_previous_positions_relative`. This avoids storing large
world coordinates in the OpenGL half-float position target. The new
`relative_motion` GPU proof uses world origin (100000, 200000, 0), verifies
normalized UV motion 0.125 on geometry and zero background to 0.0001, and checks
that the unshifted GL path rejects half-range overflow. Metal and Linux
Vulkan/OpenGL pass; the Linux script also reruns the existing three probes.
Linux remains llvmpipe software rendering. This addresses map storage range,
not arbitrary large-world precision: source positions and reconstruction still
use f32, and the test displacement/coordinates are exactly representable.

`sh tools/linux/temporal-motion-smoke.sh` now passes all three probes on Linux
Vulkan and OpenGL: primary/background correspondence, static/deformed motion
composition and the eight-frame mixed voxel/skeletal native-window lifecycle.
Both use Mesa llvmpipe CPU software rendering; this is not NVIDIA GPU evidence.
OpenGL required two explicit portability fixes: comparison-sampler depth recovery
(24 iterations, absolute depth error up to 2^-24, exact clear-zero handling) and
RGBA16Float previous-position rendering because this GL adapter rejects RGBA32
attachments. That map's small-coordinate test allows 0.0005 normalized UV error;
direct raster motion and other planes retain 0.0001 checks. Large-coordinate
accuracy needs further work; half-range overflow is rejected. Vulkan/Metal retain
direct nearest depth sampling and RGBA32 correspondence. Native X11 examples use
a display-aware instance. Metal's native window test and focused Clippy also pass.

Resident chunk uploads now track `camera_anchor` and reset camera/pose history
and motion publication only after a successful change of that coordinate origin.
Reuploads with the same anchor retain history; invalid uploads retain both the
previous anchor and published frame. The native regression now covers eight
resident frames, testing a successful anchor change, same-anchor upload and an
overflowing-origin upload with a different requested anchor. All pixel, coverage,
reset and metadata checks pass on macOS. Animation loops within the test's visible
region so the original visibility/occlusion assertions stay exercised.

`Renderer::render_scene` now resets resident camera/pose history and invalidates
its published motion metadata before using the shared primary depth. Previously
the resident motion output could remain visible across this unrelated render
path. The native regression now performs six resident frames, with an empty
general-scene presentation before the last one, and verifies output invalidation,
history reset and zero reset-frame vectors on return. Native macOS execution,
renderer/example Clippy and diff checks pass.

The native macOS motion proof now contains an opaque voxel quad in front of the
weighted skeletal triangle. All five frames require more than 20 occluded
skeletal pixels and more than 100 visible skeletal pixels. Readback checks that
voxel coverage keeps static-world camera motion while visible skeletal coverage
uses interpolated deformation correspondence; exterior/reset zeros and frame
metadata remain checked. The window execution passes, establishing mixed
resident voxel/skeletal coverage and occlusion, not general multi-object motion.

`RasterMotionPass::encode_over` preserves an existing compatible RG16 target,
overwriting only Equal-depth geometry coverage. A dedicated 4x4 GPU composition
proof checks +0.1 motion on moving geometry, preserved -0.1 static camera motion
beside it, and zero background. The resident renderer now initializes motion
from its sampleable reverse-Z primary depth (clear 0), then overlays skeletal
correspondence into that same output. The weighted native-window pixel/reset
proof passes with this combined output. This includes static opaque voxel
coverage in principle; a mixed voxel/skeletal native-window pixel proof and
moving non-skeletal object correspondence remain pending.

`DepthMotionPass` supplies static-world camera motion over all primary opaque
depth coverage, without a normals map or reconstructed-surface storage buffer.
It unprojects current depth and reprojects through the previous camera, producing
RG16 backward UV vectors. Caller-supplied clear depth identifies background;
reset writes zero. The Metal half-covered-frame probe checks camera motion and
background at all 16 pixels with conventional clear depth 1; reset is also
checked. Reverse-Z clear depth 0 and integration as the resident scene's base
motion layer remain pending runtime verification. Moving objects must overwrite
the static-world baseline with their own correspondence.

`RasterMotionPass::next_frame` shares the compiled render pipeline while creating
independent pose buffers, bindings and output texture for the next frame.
The resident renderer uses it when a prior published output is available;
reset/recreation starts with a fresh pipeline. The five-frame native macOS
weighted-deformation/camera-motion pixel proof and renderer/example Clippy pass.
This removes repeated pipeline construction from uninterrupted motion frames;
no frame-time improvement has been measured. Vertex buffers, bindings and output
textures are still recreated and require further resource reuse work.

The resident window proof now uses two joints with different UNORM16 weights
per vertex, including an out-of-plane bone rotation. It independently projects
the paired triangle, computes barycentric correspondence at each pixel and
checks all interior GPU motion vectors, exterior zeros and reset zeros.
Explicit presentation-ID/reset assertions are now present in the actual
example; the previous documented ID claim preceded those assertions because
the earlier edit did not match the formatted source. Metadata invalidation on
resize/suspension is also checked. The weighted-deformation run and Clippy pass.

`Renderer::skinned_motion_output` now exposes the last presented skeletal motion
texture together with its monotonically increasing resident presentation ID and
the exact reset-history flag used by its motion pass. Acquisition skips and
suspension do not advance the counter; resize/reset/replacement invalidate the
published metadata. The window proof checks IDs 1 through 5, reset flags and all
motion pixels in the same run. Renderer/example Clippy and native macOS execution
pass. These IDs belong to the resident renderer, not a separate scene surface or
SDK frame token, and consumers must explicitly align those frame domains.

The resident window proof now moves the camera and skeleton together on its
last frame; GPU readback still matches projection through the last presented
camera/pose pair. History tests, renderer/example Clippy and the window run pass.
Current and presented joint palettes reuse their existing storage during updates
instead of allocating a new vector each frame. Paired-vertex/resource/pipeline
recreation in the opt-in motion pass remains a separate pending optimization.

`Renderer::set_skinned_motion_enabled(true)` now automatically builds and encodes
`RasterMotionPass` after primary geometry/depth, publishing the RG16 output only
after presentation. `skinned_motion_texture` exposes the last presented texture;
reset, mesh replacement and resize clear it. Acquisition skips retain the prior
presented output. It covers the resident opaque skeletal mesh only; other scene
geometry has zero motion, so this is not a complete DLSS scene input.
The native-window proof reads all 320x240 pixels for five frames, checking finite
vectors against projected pose displacement, populated moving coverage and
zero output after reset. It passed on macOS. The opt-in implementation currently
recreates CPU paired vertices, GPU resources and pipeline each frame; caching
and broader scene integration remain pending.

`RasterMotionPass` directly rasterizes paired current/previous world positions
into RG16Float backward UV motion, Equal-tested against the primary depth with
no depth writes. Perspective interpolation precedes fragment reprojection;
background/reset, behind-camera and unrepresentable motion write zero.
The Metal probe checks its output against the independent analytical deformation
and the existing correspondence-map/surface reconstruction path for all pixels.
This removes the need for a reconstructed surface buffer for the direct geometry
path. Automatic resident-renderer motion output and native SDK consumption are
still pending.

`Renderer::prepare_skinned_motion_camera` exposes current/last-presented
unjittered camera matrices using the primary renderer's reverse-Z projection.
Camera history commits after presentation and resets alongside skeletal history,
including invalid temporal poses. The native-window regression now checks both
history-valid flags before and after all five frames. A run ended on persistent
OS occlusion; a fresh run with window focus passed. Renderer/example Clippy and
skeletal-history tests pass. Automatic motion texture rendering remains pending.

Native macOS presentation proof now passes:
`cargo run -p voxy_render --example skinned_history_surface --offline`.
Five actual presented resident-mesh frames check correspondence before/after
presentation, a prepared intermediate pose that is never rendered, resize,
suspension (zero-size surface) and resumption. This verifies the resident
renderer commits the pose after presentation and resets history on these
resource transitions. It does not verify native DLSS processing or automatic
motion-texture output, and does not exercise OS-driven acquisition failures.

The resident animated mesh in `Renderer` now retains current joints/model and
`SkinnedMotionHistory`. Successful `Renderer::render` calls commit the pose after
`queue.present`; timeout, occlusion and suspension do not commit. Resize, lost,
outdated and validation-failed surfaces reset history. Mesh replacement starts
fresh. `prepare_skinned_motion` exposes correspondence and its history-valid bit;
`reset_skinned_motion` handles camera cuts/teleports explicitly. Existing finite
projective pose rendering remains accepted, but unsupported temporal poses reset
history and are rejected by correspondence preparation.
Resident-history regression and renderer Clippy pass. This wiring has not yet
been verified with a native window presentation test; automatic RG motion output
in the resident renderer and native DLSS consumption remain unfinished.

`SkinnedMotionHistory` retains an owned mesh and the last explicitly presented
joint palette/model. `prepare` creates paired vertices without advancing history;
`presented` validates before committing, and `reset` makes the next pair identical
with `history_valid=false`. Separate instances are required for independent mesh
instances/VR eyes. Callers must commit only after successful presentation.
Unit coverage verifies skipped poses, invalid palette retention and reset.
The Metal correspondence probe now prepares a skipped intermediate pose before
the tested pose; GPU motion still matches the last presented palette.
Scene-loop presentation callbacks still need to call this API.

`SkinnedMesh::previous_position_vertices` now expands the existing indexed mesh
using current/previous joint palettes and model transforms. It follows the
renderer's four UNORM16 influence weights and `model * skin * position` order,
rejecting invalid palettes, nonaffine matrices and overflowing positions.
The standalone Metal proof now feeds this preparation into `PreviousPositionPass`
and `PrimaryMotionPass`, checking analytically varying motion on the foreground.
A CPU regression covers two weighted joints, different model transforms,
indexed expansion and invalid inputs. Pose preparation is CPU-side; retaining
the last presented skeleton and connecting this path to the scene frame loop
still remain to be implemented.

`PreviousPositionPass` now rasterizes paired current/previous world vertices
into RGBA32Float correspondence at current camera coverage, using Equal depth
against the opaque primary depth without depth writes. Perspective-correct
interpolation supplies previous positions; uncovered pixels clear to W=0.
The deformation proof now uses this GPU producer instead of a CPU-uploaded map.
Run `cargo run -p voxy_render --example previous_position_probe --offline` for
the standalone Metal proof without app/physics dependencies. Animated-pose
vertex pairing still needs integration with the skinned scene renderer.

`PrimaryMotionPass::for_previous_positions` now accepts RGBA16/32Float
previous world positions indexed by current primary pixels (XYZ position,
W=1 valid correspondence). This enables a deformation-aware motion consumer;
callers still must rasterize correspondence from their previous animated mesh.
A previous frame's position image is not sufficient. Invalid correspondence,
history reset and primary background produce zero motion.
The Metal probe uploads a varying deformation `(x+0.2*y, y+0.1*x, z)` and
checks analytical UV motion `(0.1*y, -0.05*x)` on all foreground samples,
including a deliberately invalid correspondence and background zeros.
This passed on M4 Max; skeletal correspondence production and NVIDIA execution
remain unfinished.

The ray probe now includes an independent 4x4 half-covered opaque frame.
GPU readback verifies all 16 pixels: foreground IDs are 0, background IDs are
`u32::MAX`, reconstructed foreground positions/normals match the rasterized
plane, and all eight surface components are zero on background pixels.
Per-object temporal motion is +0.1 backward UV in X on the foreground and
exactly zero on the background. This proof passed on Apple M4 Max / Metal.
The same partial frame now also verifies camera-only motion (-0.1, -0.05),
combined camera/object motion (0, -0.05), and history reset (0, 0) on all
foreground pixels. Background remains zero in every case. The combined case
checks cancellation of horizontal camera/object motion rather than treating
one source as a replacement for the other. Motion-history unit tests also
verify skipped frames and invalid updates preserve the last presented matrix.
An additional GPU plane checks object rotation by 90 degrees and nonuniform
scale, with both current and previous models nonidentity. Independently derived
previous world coordinates are `(-4*y, 0.25*x, z)`, giving backward UV motion
`((-4*y-x)/2, (y-0.25*x)/2)` per foreground pixel. All 16 pixels passed Metal
readback, including zero background. This checks inverse-current composition
and does not establish skeletal/deformation correspondence.
Renderer-only strict Clippy and diff checks passed. The combined probe Clippy
currently reports precision casts and function length in the independently
modified `face.rs`; it is not a passing gate for this snapshot.
This verifies temporal input coverage, not native NVIDIA FG/MFG execution.

## Rasterized opaque object IDs for temporal motion

Guide allocation now owns an R32Uint primary object-ID texture. The separate
`ReconstructionGuidePass::encode_object_ids` pass draws caller-supplied mesh/ID
pairs at Equal depth against the primary depth, without depth writes or changes
to normal/albedo/F0 guides. It clears background to `u32::MAX` and rejects that
reserved value for object draws. This adds a separate single-attachment pass,
not another simultaneous RR MRT attachment. Inputs must use identical world
geometry/camera/depth coverage as the primary opaque pass.

The Metal proof rasterizes left/right primary-plane meshes with IDs 0 and 1,
then attempts to overwrite them with full-coverage triangles at world Z 0.4 and
0.6 (IDs 77 and 88), differing from primary Z 0.5. Equal depth must reject both
extra draws. It also verifies rejecting reserved `u32::MAX` before encoding.
The resulting texture feeds `PrimaryMotionPass::for_objects`; readback checks
backward X motion -0.1 for left pixels and +0.1 for right pixels in all 16 samples
and both lighting cases. The explicit-map unknown-ID proof also runs.
Renderer/probe Clippy (`--no-deps`), package formatting and diff checks passed.
This verifies preserving IDs against draws that disagree with the existing
primary depth, including an occluded surface. Larger multi-object scenes,
skeletal/deformation correspondence and native NVIDIA temporal evaluation
remain unverified or unfinished.

## Independent object motion selected per primary pixel

`PrimaryMotionPass::for_objects` accepts an input-resolution R32Uint object-ID
map and a table of current/previous affine model pairs. CPU preparation validates
models and uploads `previous_model * inverse(current_model)` per object; the
fragment shader selects the transform by ID and projects the previous world
position. Unknown IDs write zero instead of indexing outside the table. Camera-
only and single-object entry points retain their existing behavior. ID maps must
match the opaque primary depth and current transformed world positions.

The Metal proof uploads IDs [0,0,1,99] across each row and checks opposite motion
for two transforms: backward UV X [-0.1,-0.1,+0.1,0] in all four rows. The last
column exercises unknown-ID handling. Existing camera/object reset, guide,
lighting and reflection checks also passed in both light cases. Renderer/probe
Clippy with `--no-deps`, package formatting and diff checks passed. Full-workspace
formatting encountered a concurrently edited documentation-comment error in the
hair-render example. This proof uses an explicit ID texture; scene object-ID
rasterization, skeletal/deformation correspondence and NVIDIA runtime execution
remain unfinished.

## Single-object affine motion on GPU primary surfaces

`PrimaryMotionPass::for_object` combines previous camera projection with
`previous_model * inverse(current_model)` to reproject current world positions.
It validates finite nonsingular affine model matrices, retaining the existing
camera/reset/normalized UV convention. The input buffer must describe one object
with that current transform; independent objects need separate batches or a
future per-pixel motion mapping. Skeletal/deformation motion is not represented.

Metal readback verified all 16 pixels for current object translation +0.3 X and
previous translation +0.1 X: backward UV motion [-0.1,0], with zero motion under
history reset. Current and previous cameras were identical in that case, so the
motion comes from object transforms. Existing camera-only motion, both lighting
cases and all guide/radiance proofs also passed. Strict renderer/probe Clippy
with `--no-deps` passed; the app dependency still warns about unused `set_grasp`.
Native NVIDIA temporal evaluation remains unverified.

## Camera-only motion for reconstructed primary surfaces

`PrimaryMotionPass` projects valid static world positions with current/previous
unjittered view-projection matrices and writes RG16Float backward UV motion
(previous minus current, top-left Y convention). Primary reconstruction may use
the actual jittered depth camera. History reset, invalid/background/behind-camera
samples and unrepresentable vectors write zero. The Streamline 2.14.1 constants
header specifies motion scale as normalization factors; normalized UV motion
uses scale [1,1]. This pass covers camera motion on static world geometry, not
moving objects, skeletal animation or deformation.

Metal readback checked all 16 pixels with a translated previous camera: UV motion
[-0.1,-0.05], and zero motion with the same camera difference under history reset,
in both lighting cases. Renderer and ray-probe strict Clippy with `--no-deps`
passed; the app dependency currently emits a separate unused `set_grasp` warning.
Motion texture ownership can be retained for native consumers like the existing
RR import path, but a temporal NVIDIA scene execution is not yet demonstrated.

## Primary guide setup without reference-ray dependency

`ReconstructionGuidePass::primary_inputs` supplies the primary material/depth
pass with a zero-initialized, input-resolution R32 distance placeholder. Wgpu
initializes its contents before sampling. Primary normals/albedos/F0 and depth
can therefore be rasterized before any reflection queries. After reconstructing
surfaces and tracing reflections, `ReconstructionDistancePass` replaces the
final RR distance guide from actual GPU hits.

The varying-material Metal probe now encodes its complete GPU guide/surface/
reflection/direct-light/HDR composition chain before dispatching CPU-sampled
reference rays. Reference outputs are used only for comparison/readback, and are
not bound by that chain. This validates removing the previous bootstrap
reference-distance dependency. Renderer-only strict Clippy (`--no-deps`) passed. Probe strict Clippy (`--no-deps`) now passes after narrow face-probe fixes
(explicit descriptor defaults, integer visibility validation and separated
image/decoding helpers). Checking all dependencies still reports physics
diagnostics. The face probe also ran on Apple M4 Max: three poses, 741270-741272
triangles and 2347950 shadow/ambient segments per pose, with more than 1.6 million
occluded segments per pose. The generated `/tmp/voxy-face-ray-preview.png` was
visually inspected; it shows all three facial poses with hardware-ray shadows.
This proves that scene probe on Metal, not NVIDIA SDK execution. The placeholder currently
uses an input-resolution allocation; a dedicated geometry-only shader variant
could avoid it. General temporal scene wiring and NVIDIA execution are still
unfinished.

## Direct point-light shading from GPU primary surfaces

`SurfaceLightingJob` consumes the reconstructed primary storage buffer and the
matching linear diffuse-guide texture. It computes N dot L, inverse-square
Lambertian radiance and the opaque shadow ray on the GPU, writing RGBA16Float
for HDR composition. It supports one point light per job, intensity [0,65504],
positive bias and perspective-derived world surfaces. Invalid/background/map
pixels and near-singular light distances write black; RGB output saturates at
65504. The caller must preserve the same device and primary pixel mapping.

The Metal 4x4 varying-material probe now uses GPU-derived positions/normals and
raster diffuse/F0 for both primary lighting contributions: direct point-light
radiance and mirror reflection. It checks HDR composition against analytic
per-pixel Lambertian/Schlick expectations in visible and occluded-light cases,
as well as world surfaces, F0, traced distance and the final RR distance guide.
Strict Clippy passed. CPU-generated ray samples remain only as an independent reference; primary
guide setup now uses a zero-initialized distance placeholder. Rough/indirect transport, general scene temporal wiring,
and NVIDIA RR/FG execution still require implementation or hardware validation.

## Per-pixel primary material F0 for GPU reflection

Guide allocation now also owns a sampleable/renderable RGBA16Float material-F0
texture, distinct from the four NVIDIA RR guides. `encode_material_f0` redraws
the same opaque geometry with Equal depth testing and no depth writes into this
single attachment. It stores linear mix(0.04,baseColor,metallic), alpha one;
background remains alpha zero. A separate pass avoids adding bytes to the four
existing MRT attachments.

`SurfaceReflectionJob::with_material_map` reads F0 per primary pixel and computes
Schlick reflection weights. Invalid/out-of-range map samples are skipped; rough
primary pixels remain unsupported. The map must match primary depth/normal/camera.
The Metal probe uses three distinct vertex base colors, with analytic barycentric
material interpolation over a constant-depth triangle. It verifies all 16
spatially varying raster F0 values, reflected HDR colors and direct-light plus
reflection composition in both light cases. Its uniform fallback F0 is
deliberately black, so matching nonblack reflected color establishes that the
material texture is used. Direct-light diffuse reflectance varies with the same
primary material. Depth/normal and ray/guide distances are also checked. Strict
Clippy passed. This proves interpolated per-pixel material data in this scene;
textured assets, multiple objects, rough transport and NVIDIA runtime evaluation
remain unfinished.

## Update final RR hit-distance guide from GPU primary rays

`ReconstructionDistancePass` copies traced R32Float distances through a
single-attachment render pass into the selected R32Float/R16Float RR guide.
It preserves normal/albedo targets and depth, does not redraw primary geometry,
and rejects source/output aliasing, mismatched dimensions and invalid usages.
The R16 path requires distances within 65504; zero is the miss convention.
Retained views keep the exact source and target allocations alive, so rebuilding
or resizing guides requires rebuilding this pass.

The Metal perspective-scene probe now updates its final hit-distance guide from
`SurfaceReflectionJob` after GPU surface reconstruction and ray traversal. It
checks the resulting guide and traced distance against the reference at all 16
pixels, together with GPU-reflection HDR composition in both direct-light cases.
Strict Clippy passed. The initial material pass still supplies bootstrap distance
data for setup; final RR distance/color come from GPU-derived primary reflection.
This does not prove a native NVIDIA RR frame or complete engine integration.

## Reflection from GPU-reconstructed primary surfaces

`SurfaceReflectionJob` consumes `PrimarySurfaceJob`'s storage buffer directly.
For each valid zero-roughness primary pixel it computes a perspective-camera
mirror direction, traverses the scene TLAS, records world-space hit distance,
and multiplies closest-hit primitive emission by Schlick Fresnel throughput.
Background/rough/degenerate-camera samples write black and zero distance. The
current path accepts one primary material for the entire surface buffer;
per-pixel materials and rough-lobe sampling remain necessary for general scenes.

The Metal 4x4 perspective probe compares GPU-derived reflection HDR color and
distance against independently constructed CPU primary samples, for every pixel
in both light-visibility cases. HDR composition now consumes the GPU-derived
reflection texture. The bootstrap material pass uses reference distances; the final guide is now
updated from GPU-derived distances before readback. Strict
Clippy passed. Full scene temporal integration and NVIDIA execution remain
unverified.

## GPU primary-surface reconstruction from depth

`PrimarySurfaceJob` unprojects depth pixel centers using the inverse of the exact
jittered view-projection matrix and wgpu's [0,1] clip depth. It flips texture Y
into clip Y and writes a row-major storage buffer with two vec4 values per pixel:
world position/valid flag and normalized world normal/roughness. Clear-depth,
invalid depth/normal and nonfinite reconstructed samples write invalid zeros.
Both conventional and reverse depth are supported through an explicit clear
value. Source depth must be sampleable Depth32Float; normal/roughness must be
sampleable RGBA16Float or RGBA32Float at the same dimensions.

The Metal perspective-scene probe runs reconstruction after the material/depth
pass and verifies all 16 world positions, validity flags and normals in both
lighting cases. Strict Clippy passed. This makes GPU surface data available;
ray dispatch still uses the earlier CPU-generated samples until a subsequent
GPU ray-generation pass consumes this buffer. NVIDIA runtime evaluation remains
unverified.

## Import composed ray-traced HDR color into the scene RR bridge

`SceneRayReconstruction::import_radiance` accepts the separate lighting output
(for example `RadianceComposition::output()`) and combines it with a temporal
candidate's depth, RG16 motion, presentation ID and history-reset requirement.
The original raster color is not imported by this overload. Existing `import`
continues to use the candidate's own color through the same validation/import
path. All eight native RR texture owners still transfer to the existing
preparation/evaluation/fence lifecycle.

The caller must match primary surfaces, camera/jitter and pixel grid across
composed radiance, depth, motion and guides; formats alone cannot establish that
semantic correspondence. Encode lighting and composition before preparation,
submit them on the serialized registered queue before SDK evaluation, and retain
runtime/output consumers until GPU completion. Strict Windows GNU all-target
Clippy passed against Streamline v2.14.1 with `scene-dx12`. This verifies the
compiled bridge API, not NVIDIA runtime execution or a full temporal scene.

## Ray-shadowed HDR point-light contribution

`PointLightSample` pairs a world-space shadow segment with its unoccluded
Lambertian contribution: linear diffuse reflectance / pi, positive N dot L,
and point-light radiant intensity divided by squared distance. Diffuse must
already exclude energy allocated to other lobes. Invalid reflectance, intensity,
geometry and values outside finite binary16 radiance are rejected.
`DirectLightingJob` runs opaque ray visibility followed by RGBA16Float lighting,
and can feed `RadianceComposition` on the same GPU stream.

The 4x4 perspective-camera probe now uses a half-metallic zero-roughness primary
surface and a point light between that surface and its reflected emitter. It
checks combined point-light plus mirror HDR colors against inverse-square
Lambertian and Schlick expectations for every pixel, alongside ray/guide hit
distances. The integration probe runs both unoccluded direct light and a light behind
the emitter triangle. All 16 occluded pixels retain the mirror contribution
while losing the direct contribution; both distance planes remain unchanged.
CPU tests also cover inverse-square falloff, back-facing normals and invalid
light/material inputs. The separate visibility smoke verifies finite-range rays.
Multiple lights, diffuse indirect transport, rough sampling and NVIDIA RR/FG
execution remain unfinished.

## HDR lighting composition before reconstruction

`RadianceComposition` combines primary lighting and BSDF-weighted reflected
radiance into a separate RGBA16Float texture using ordered GPU texture loads.
It leaves linear HDR values unexposed and untone-mapped, ignores input alpha,
and sets alpha to one. RGB sums saturate at 65504 to avoid binary16 overflow.
Inputs must share device, dimensions, primary-surface mapping and valid finite
nonnegative RGB. Unsupported formats, layers, multisampling and missing sampling
usage are rejected before pipeline construction.

The Metal integration probe composes primary surface emission [0.25,0.125,0.0625]
with perspective-camera mirror reflection, then reads back all 16 HDR colors and
both ray/guide distance planes. This demonstrates color composition, not full
scene lighting: direct lights, diffuse transport, rough reflection sampling and
temporal/native NVIDIA RR evaluation still need integration.

## Coherent primary mirror samples and RR guide proof

`MirrorSurfaceSample` derives an ideal-mirror ray and Schlick material throughput
from the same world position, normal and camera. `create_mirror_job` consumes
those paired samples to avoid independently populated ray/weight arrays.
The 4x4 Metal integration probe uses one perspective camera for the primary
material raster pass and reflected rays, zero roughness in both paths, and an
emissive triangle with HDR RGB [4,1,0.5]. Readback checks all 16 shaded HDR colors
and both the original ray distances and RR guide distances from one ordered GPU
submission. This is a reflection-only scene test; complete lighting, temporal
scene integration and NVIDIA RR/FG execution remain unfinished.

## Mirror material Fresnel weights

`ReconstructionMaterial::mirror_throughput` produces the reflection-only
Schlick Fresnel weight from linear base color, metallic fraction, world normal
and surface-to-camera direction. Dielectric F0 is 0.04 and metal F0 is base color.
Directions are normalized; the current two-sided convention uses abs(N dot V).
Nonzero roughness is rejected: this delta-mirror helper does not implement
rough GGX sampling or transmission. The formula follows the
[Enterprise PBR shading specification](https://github.com/DassaultSystemes-Technology/EnterprisePBRShadingModel/blob/master/spec-2019x.md.html).

CPU tests cover normal/grazing endpoints, an oblique dielectric angle, degenerate
input and rejection of rough materials. The Metal emission probe now obtains
its per-ray weights from actual material objects before GPU evaluation. Full
scene shading and NVIDIA RR/FG runtime validation remain unfinished.

## Primary BSDF throughput for reflected emission

`create_weighted_radiance_job` applies an explicit per-pixel RGB throughput to
single-bounce emitted radiance on the GPU. The caller supplies the primary
material's sampling weight (Fresnel for an ideal delta mirror, or BSDF times
cosine divided by PDF for a sampled lobe). The RR specular-albedo guide is not
used as that weight. Weights are limited to finite [0,1] components to preserve
the validated RGBA16Float emission range; input alpha is ignored. Unweighted
jobs retain their previous behavior.

Metal GPU readback on Apple M4 Max verified different RGB weights per ray,
including zero contribution, HDR values above one, misses and unchanged hit
distances. The renderer still needs primary material/rough-lobe sampling and
other lighting terms for a complete noisy-color buffer. NVIDIA SDK evaluation
and RTX 5090 frame generation remain unverified on this host.

## Emissive reflected incident radiance

`SpecularDistancePipeline::create_radiance_job` now writes both closest-hit
R32 distances and RGBA16Float incident radiance. The linear RGB emission table
has one entry per shared BLAS primitive; all instances use that table. Misses
write black, alpha is always one. Invalid table sizes, negative/nonfinite RGB,
and values above binary16 maximum 65504 are rejected. Alpha in the input table
is ignored.

The Metal ray probe on Apple M4 Max verified distinct HDR colors for nearest and
farther triangles, black for miss/range rejection, and existing distance/guide
integration checks. Strict Clippy also passed. This is ideal-mirror incoming
emission, not final primary-surface shading: material BSDF, other lighting and
scene integration remain necessary before using it as RR noisy color. These
checks do not execute NVIDIA DLSS or frame generation.

## Specular ray-query hit-distance texture

`SpecularRay::from_surface` reflects the camera-to-surface direction around a
normalized world normal. The primary position remains the ray origin; bias is
t-min, so reported distances start at that surface. `SpecularDistancePipeline`
requires enabled experimental ray queries and dispatches one closest-hit opaque
triangle query per row-major ray into a sampleable/readable R32Float texture.
Misses write zero. Job creation validates ray count, ranges, normalized direction,
dimensions and storage/dispatch limits. Primary world positions/normals must still
come from the matching scene frame; glossy multi-sample rays are not implemented.

Two CPU construction tests passed. The Metal M4 Max isolated ray probe read back
four actual intersections: nearest distance 2, geometric miss 0, range-limited
miss 0 and distance 4 after t-min skips the nearer triangle. This proves the
query/texture path. The isolated Metal probe now also feeds a real 4x4 specular
ray texture into the material MRT pass in the same command encoder, exercising
the storage-write-to-sampling dependency. World primary points, normals and
camera match the rasterized plane; both source and guide hit-distance readbacks
match all 16 analytically expected world-space distances. This connects real
intersections to guide output, but not yet the SDK RR evaluation or general scene
assets. NVIDIA execution remains unverified.

## DX12 Ray Reconstruction resource ownership

`voxy_streamline::rr_dx12` imports eight owned leases for noisy color, HW depth,
RG16/RG32 motion, packed float normal/roughness, linear diffuse/specular albedo,
scalar float specular hit distance and output. Import rejects unsupported formats,
mismatched render sizes, missing usages and aliased native resources. Configure
RR with packed-normal mode and matching HDR/camera matrices before use.
RR requires HDR: native option validation, the Rust camera/options constructor
and RR texture import now reject SDR before SDK evaluation. Native regression
coverage confirms rejection clears optimal-size output and does not change SDK
options; Rust camera tests also cover SDR rejection. This follows section 5
of the linked 2.14.1 guide.
`prepare` establishes shader-read inputs/UAV output; `evaluate` sets constants,
tags until-evaluate inputs and records RR; consuming `submit` retains all eight
leases through the native fence. Borrowed `inputs`/`output` expose the same leases
for reconciliation barriers. Keep owners on SDK failure and preserve runtime
lifetime through completion, as with the low-level SR path.

Buffer conventions follow the official
[Streamline 2.14.1 RR guide](https://github.com/NVIDIA-RTX/Streamline/blob/v2.14.1/docs/ProgrammingGuideDLSS_RR.md).
This native bridge cross-compiles; the renderer still needs matching material
G-buffers and noisy ray-traced color. `SceneSurface::enable_two_channel_motion_vectors`
now produces RG16Float motion, preserves the HDR color mode and retains that
format through resize. Its stationary opaque-motion/depth readback and history
failure/resize reset passed on Metal M4 Max with moving X-ray geometry.
Linux Mesa llvmpipe Vulkan and OpenGL runs also passed HDR/opaque-motion/depth
readback and failure/resize history reset with RG16Float enabled and moving
transparent/X-ray layers. These are CPU software drivers, not NVIDIA GPU proof.
The SR/FG scene bridge also accepts RG16Float motion. No RR scene or NVIDIA
GPU execution has been verified, and optional guide buffers are not imported yet.
`voxy_render::RayReconstructionGuides` allocates three linear RGBA16Float
normal/roughness and albedo targets plus scalar R32Float specular hit distance.
It validates dimensions, render/sample/readback usages and four-attachment
limits; resize replaces the set only after validation. A Metal M4 Max smoke
passed four-target MRT clear, rejected-resize preservation and valid resize.
These targets do not yet receive matching scene material/ray data.
`ReconstructionMaterial` validates linear base color, metallic and roughness,
then produces packed normalized world normals, diffuse reflectance and
view-dependent specular reflectance using the guide's EnvBRDFApprox2 model.
The 48-byte POD sample contains three aligned vec4 values for GPU upload.
CPU tests passed for metallic endpoints, grazing-angle dependence, invalid
parameters and degenerate directions. Shader upload/rasterization of these
samples and real reflection hit distances remain unfinished.
`ReconstructionGuidePass` now rasterizes validated world-space triangle lists
with per-vertex linear base color/metallic/roughness. Its fragment shader
normalizes interpolated normals and evaluates view-dependent specular
reflectance against the current camera. All four MRT outputs are written;
specular distance is sampled from a separate matching R32Float ray-result
texture. It does not substitute primary depth for reflection distance.
The Metal GPU smoke compared all 16 pixels of normal/roughness, diffuse,
specular and copied hit-distance guides against expected values, including
the CPU BRDF model. That test supplies a constant distance pattern, not ray
intersections. Connecting scene assets and actual specular rays remains needed.
The guide allocation now prefers R32Float distances and selects R16Float when
the adapter cannot render to R32Float. `ReconstructionGuidePass::for_guides`
matches that attachment format. OpenGL llvmpipe exposes R32Float sampling/storage
but not rendering; its R16 path passed all four guide readbacks and CPU BRDF
comparison. Vulkan llvmpipe also passed using R32Float. These are software CPU
drivers. R16 users must constrain distances to the binary16 finite range (65504)
and account for its lower precision; real specular rays remain unconnected.

## SceneSurface DX12 bridge

`SceneRayReconstruction::import` connects an HDR temporal scene candidate with
its `RayReconstructionGuides` and RG16Float motion to the eight-resource RR
owner. It preserves candidate presentation ID/reset requirements, validates
formats/sizes/usages/aliasing, and prepares all scene/material/ray resources
before native submission. Consuming `submit_with_reconciliation` rejects an
omitted required reset, records RR, exposes the retained resources for exit-state
barriers, then transfers owners to the exact wgpu direct queue's native fence.
SDK/callback failures retain textures/recorder conservatively. Configure matching
packed HDR RR options and keep the runtime alive through completion.

Import cannot establish that color contains ray-traced radiance or that guides
describe the same primary surfaces; those remain explicit unsafe caller
requirements. Existing raster HDR color alone is insufficient RR input. This
bridge passes Windows cross-Clippy but has no NVIDIA scene execution proof yet.

On a Windows NVIDIA host, set `VOXY_DLSS_SR=1` and run `dx12_probe` with the
absolute signed interposer DLL path to enable twelve real SR/DLAA evaluations. The probe
enumerates DX12 adapters and selects one accepted by the SDK SR support query
using its exact DXGI LUID, logging rejected adapters. An unsupported system or
`VOXY_DLSS_SR=1` with `--queue-only` fails before device creation/evaluation.
The probe
configures Quality at 1024x768, then resizes to 800x600 and switches to DLAA
at 800x600, followed by HDR Quality at 800x600 with RGBA16Float color/output.
The HDR input is (4, 1, 0.5), explicitly exceeding the SDR range. Float readback
decodes binary16 channels and rejects nonfinite or mismatched values with an
8% relative plus 0.02 absolute tolerance. It recreates textures only after the preceding configuration's
native/readback work completes, validates DLAA input/output size equality,
uses the SDK-recommended render size, and initializes
RGBA8 color, Depth32Float depth and RGBA16Float zero motion, acquires a frame
token for each frame, records SR on the imported direct queue and
waits for native completion. The sequence is reset, preserved history with a
previous camera transform, then reset again for each configuration, using increasing SDK frame indices
and reused textures. Each output is cleared before evaluation so stale pixels
cannot satisfy readback checks. SDK/device ownership is retained on evaluation
failure or uncertain GPU completion. After completion it copies the output to
CPU memory and checks RGB at five interior points against the uniform input
color with a 16/255 per-channel tolerance for SDR, ignoring alpha. This detects an empty
or grossly incorrect result; it does not measure reconstruction quality. No FG
presentation is attempted. This path cross-compiles but
has not executed on NVIDIA hardware. Tagged resource states follow NVIDIA's
[DLSS integration guide](https://github.com/NVIDIA-RTX/Streamline/blob/main/docs/ProgrammingGuideDLSS.md);
the probe uses a fresh list and performs no later draws requiring CL-state restoration.
The readback validator also runs without DX12 via
`cargo test -p voxy_streamline --example dx12_probe --locked --offline`.
Four CPU tests cover padded rows/ignored alpha, corruption at each sample and
RGB channel, nonfinite/clipped/negative HDR, empty output and invalid/truncated
layouts. These tests passed on macOS; they verify the validator, not SDK output.

Enable `voxy_streamline/scene-dx12` for native imports of `voxy_render::TemporalFrame`.
`scene_dx12::SceneSuperResolution::import` retains scene color/depth/motion and
processed output, preserving the candidate presentation ID and reset-history
requirement. It validates HDR/color/depth/motion formats and output format before
HAL import. Call `prepare` from the surface preparation hook; after submission,
`evaluate` requires camera reset when the surface invalidated history. Transfer
`into_resources` into native submission ownership for GPU completion retention.
Alternatively, consuming `submit` records SR and transfers all four textures to
the imported wgpu queue's private completion-fence guard. It rejects omitted
history reset before recording and conservatively retains textures on SDK or
command-list closing failure. Restore wgpu-tracked states before subsequent
wgpu access; keep the SDK runtime live until SR completion. FG presentation
requires its separate resource lifetime guard.
Use `submit_with_reconciliation` when native barriers are needed after SDK
evaluation: its callback records on the same list before closing and submission.
Restore each resource's prepared state using the actual SDK exit state; plain
`submit` requires SDK evaluation to already preserve those states. Callback
failure retains the recorder and texture owners rather than releasing possible
SDK dependencies.
The reconciliation callback receives `SuperResolutionTextures` borrowing the
exact retained leases. `restore_prepared_states(recorder, exit_states)` accepts
the actual color/depth/motion/output DX12 exit states, restores three shader-read
inputs and the UAV output, and records an output UAV barrier. It does not infer
SDK exit states or synchronize a different queue. These resources are also
available through `SuperResolutionResources::textures` for manual recording.
SR import now rejects aliased native resources before SDK evaluation, sharing
the same input-size/identity validation with manual recording. Plain scene
`submit` records an output UAV barrier after evaluation under its prepared-state
contract; the reconciliation variant lets the callback restore those states.
`dx12_probe --queue-only` also exercises the four-resource SR ownership path:
it checks aliased-output rejection, prepares all four textures, transitions them
to copy-source and restores the prepared states with an output UAV barrier,
retains them through the private native fence, then compares all four readbacks.
These are synthetic RGBA textures; this tests native ownership/state handoff,
not DLSS input format support or SDK evaluation. The Windows binary is built
from macOS; runtime results require executing it on a DX12 host.
`scene_dx12::import_frame_generation` imports the same scene inputs for FG tagging.
`SceneFrameGeneration::import` preserves the candidate presentation ID and
reset-history requirement with those owned inputs. Call `prepare` in the scene
preparation hook, then consuming `tag` after its submission with the matching
SDK frame/viewport/constants. If history was invalidated, omitted camera reset
is rejected before SDK tagging. The returned `TaggedFrameGenerationInputs`
guard uses the existing presentation/fence reclamation contract; presentation
identity and SDK token matching remain explicit caller responsibilities.

Configure SDK HDR, dimensions, frame token and viewport before evaluation. Inputs
and output must use the registered DX12 device; proxies/presentation and resource
lifetime contracts still apply. Temporal color includes transparent/X-ray layers,
but depth/motion describe opaque geometry. This bridge cross-compiles with Windows
Clippy; native DLSS/FG scene execution is not yet verified on NVIDIA hardware.

## DXGI presentation with Reflex markers

Unsafe `StreamlineFrame::present_dxgi` queries `GetCurrentBackBufferIndex`,
opens `PresentStart`, invokes actual DXGI `Present`, then attempts `PresentEnd`
even if DXGI fails. Invalid sync intervals and test-only presentation are
rejected before markers. Start-marker failure prevents presentation. The result
preserves the DXGI HRESULT and the end-marker error independently; DXGI status
such as occlusion must not be counted as a visibly presented/generated frame.
If the end marker fails, unsafe `finish_present_marker(&mut report)` retries
only that marker and preserves the original HRESULT. A report with a successful
end marker causes no additional SDK call. The report must belong to that exact
live frame. Submit
markers must already surround actual engine queue work with this frame token.
The caller must provide a properly interposed live swapchain, initialized frame
resources, synchronization and SDK lifetimes. This helper does not initialize
render targets or enable FG and has not executed on Windows/NVIDIA hardware.

## Owned Frame Generation inputs

`dx12::FrameGenerationResources` imports renderer depth, motion vectors and
HUD-less color as owned DX12 texture leases. Depth/motion dimensions must match;
HUD-less color can use output resolution. Import rejects resource aliases and
missing texture-binding usage. `prepare` establishes wgpu shader-read tracker
states; submit it before SDK consumption. `tag` sets camera constants and tags
all three resources through presentation with the same frame token/viewport.
It does not invoke FG evaluation: interpolation occurs through presentation.

Successful tagging returns `TaggedFrameGenerationInputs`. After presentation,
unsafe `track_completion(&state)` can bind the exact SDK fence/value to that input
set and return `TrackedFrameGenerationInputs`. Its safe `is_complete` observes
CPU fence completion, and `try_reclaim` transfers completed resources once.
Device-removal sentinel values are errors. Completed tracked guards release
normally on drop; pending/error guards conservatively retain resources. Binding
requires all other GPU users to have completed and no future tag consumption;
the SDK fence snapshot must correspond to this exact presentation.

Keep an untracked guard until SDK
input processing and all GPU consumers complete, then use unsafe `reclaim` to
reuse resources. GPU queue waits alone do not prove CPU completion. Dropping an
unreclaimed guard or encountering ambiguous tagging failure conservatively
retains resource references for process lifetime. Production frame loops must
reclaim completed guards to avoid accumulating resources. Format/camera validity,
proxy presentation, frame markers and GPU hardware validation remain required.

## Frame Generation state query

`StreamlineRuntime::frame_generation_state(viewport)` returns SDK-reported
maximum generated frames, minimum output dimension, Dynamic MFG and VSync
availability, estimated VRAM, raw runtime status, presentation count, and the
borrowed inputs-processing completion fence/value. This query belongs on the
present thread and consumes the SDK presentation counter; configuration also
queries SDK state. The fence is SDK-owned and must not be released as an owned
COM reference. Respect SDK queue-parallelism rules before reusing FG inputs.
`dx12::wait_frame_generation_inputs(queue, &state)` and its `WgpuQueue` method
provide a GPU-side wait before subsequent queue work overwrites those inputs.
They reject a null fence with a nonzero value, the reserved maximum fence value,
and device removal. Completed waits are skipped. The CPU is not synchronized:
keep tagged resources and SDK alive until GPU work completes. Producer ordering
is required to prevent a queue from waiting on its own future signal.
This snapshot reports SDK state and does not enable frame generation.

## DXGI proxy and native interface probe

`voxy_streamline` exposes automatic, manual, and manual-factory-proxy DX12
initialization. Select the mode before graphics interface creation. Existing
wgpu-owned interfaces must not be retroactively upgraded.

On Windows with signed Streamline plugins installed, run:

```sh
cargo run -p voxy_streamline --features wgpu-dx12 --example dxgi_proxy_probe -- C:\SDK\sl.interposer.dll
```

Set `VOXY_STREAMLINE_SDK` to the SDK header directory for building. The probe
creates a fresh factory, upgrades it immediately, obtains its native interface,
checks COM identity against the original factory, releases local COM owners,
enumerates DXGI adapters and selects one only if SDK support queries accept
both Frame Generation and Reflex (reporting individual rejection reasons),
creates an upgraded DX12 device and direct queue, creates a 64x64 composition
swapchain through the proxy factory, validates its first buffer dimensions,
and shuts down the SDK after releasing swapchain and factory. Runtime retains
an owned device reference until SDK shutdown. Failed upgrade preserves ambiguous references and the
module until process exit. This probe creates a swapchain without attaching it to a compositor or
presenting/generated frames. The device is upgraded before queue creation; native swapchain extraction is
queried to inspect factory interception. FG presentation still needs integration. The Windows executable is cross-compiled locally; SDK execution remains
unverified without a Windows/NVIDIA runtime.

# Native Streamline Frame Generation adapter

This C++ adapter uses NVIDIA's official Streamline 2.14.1 structures, avoiding a
handwritten Rust layout for the versioned SDK ABI. It queries live generation
limits, validates Reflex/fixed/dynamic requests and preserves SDK error codes.
It is not yet connected to the Rust renderer. No NVIDIA DLL is loaded here.

Obtain the official SDK from https://github.com/NVIDIA-RTX/Streamline/releases/tag/v2.14.1.
The header source tested is commit `2122257e0fce486f91b385aa63b9a09b0a34b363`.
Point CMake at its root:

```sh
cmake -S native/streamline -B target/streamline -DVOXY_STREAMLINE_SDK=/path/to/Streamline
cmake --build target/streamline
ctest --test-dir target/streamline --output-on-failure
```

After Streamline initialization and native device registration, the integration
must resolve `slDLSSGGetState` and `slDLSSGSetOptions` through
`slGetFeatureFunction`. Use `resolve_frame_generation` with that resolver to populate `FrameGenerationApi`;
resolution clears the table on any SDK error or null function. Keep
the SDK loaded throughout its lifetime. Call configuration on the present thread;
`reflex_active` must reflect actual SDK activation, not a UI preference.

Tests use fake SDK functions and verify no settings mutation on rejection, live
count limits, dynamic availability, viewport forwarding and SDK error retention.
They ran on macOS; a Windows x64 test EXE was cross-compiled and linked with MinGW,
but was not executed. NVIDIA runtime behavior is unverified.

Still required: Rust FFI ownership, signed DLL loading, SDK initialization and
shutdown, feature/adapter discovery, device/resource interop, per-frame constants,
resource tagging, Reflex markers, proxy swap chain, GPU fences and RTX runtime
validation. DLSS 5 neural rendering is a separate SDK feature, not implemented by
this Frame Generation adapter.

Reflex adapter resolves state/options/sleep and PCL marker functions using their
correct feature IDs. It forwards SDK-owned frame tokens and accepts low-latency
or boost modes plus a microsecond frame cap. Token creation, actual state checks
and marker placement in the engine loop are still required. Unit tests cover
option forwarding, invalid mode, missing callback and absent resolver; they do
not execute NVIDIA sleep/markers or measure latency.

`acquire_frame_token` forwards optional indexing to the core SDK function and
returns only SDK-owned pointers. Do not delete tokens. Failure clears the output.
Token tests use a fake SDK token subclass only inside the test implementation.

`ReflexFrame` enforces the ordinary rendered-frame marker sequence using one
borrowed token. Invoke methods around actual simulation, submission and present.
Do not emit these markers for generated frames or late-warp work; those require
the corresponding separate SDK path. Failed SDK calls do not advance its stage.

`Session` initializes with the SDK version, registers a D3D device once, and
shuts down after successful initialization. Call `close` explicitly to inspect
shutdown errors before unloading the SDK; the destructor only attempts cleanup.
The caller must serialize process-global session ownership and keep the module
loaded. DLL ownership is not implemented here.

Optional Vulkan registration uses official sl_helpers_vk.h and Khronos headers:
set VOXY_STREAMLINE_VULKAN=ON and VOXY_VULKAN_HEADERS=/path/to/Vulkan-Headers/include.
The caller supplies real device/instance/physical-device and SDK queue allocation
metadata; the adapter checks non-null handles and single registration only. It
does not create queues, enable required extensions or hook Vulkan presentation.

WindowsModule owns the interposer DLL handle, accepts only absolute paths without
embedded NUL, verifies the NVIDIA signature with official sl_security.h, and
loads dependencies from the DLL directory/system directory. Required core exports
are resolved before any function table is exposed. Declare the module before
Session so it outlives session cleanup; do not unload after a failed explicit
shutdown. The loader is cross-compiled/linked, not yet Windows-runtime tested.

Prefer WindowsRuntime over separately managed loader/session objects. It owns
them in destruction order and exposes feature/token functions only after device
registration. Shutdown failure retains the OS module reference until process exit
to avoid dangling SDK code references. Tokens and feature tables still borrow the
runtime; the engine must stop GPU/SDK work before destroying it.

For Windows x64 cross-compilation add
`-DCMAKE_TOOLCHAIN_FILE=cmake/mingw-x64.cmake` to configuration. Use a separate
build directory. Tests now link WindowsRuntime and include relative-path/embedded
NUL rejection cases; run that EXE on Windows to execute the Windows-only checks.
Cross-compilation alone does not execute them.

Windows startup probe from Rust:
`cargo run -p voxy_streamline --features wgpu-dx12 --example dx12_probe -- C:\SDK\sl.interposer.dll`
Set VOXY_STREAMLINE_SDK at build time. This probe reports feature support and
checks device registration/shutdown; it does not render DLSS or generated frames.
# Native frame evaluation

Manual interface upgrading is now exposed through the exception-contained C
`voxy_streamline_upgrade_interface` and unsafe Rust runtime method. They forward
the mutable COM slot directly and preserve SDK pointer mutations/errors. Callers
must follow the immediate-after-creation and reference ownership rules. Existing
wgpu-owned interfaces cannot be retrofitted safely by this API alone. Windows
Clippy/C++ build checks pass; actual swapchain proxy creation remains pending.

The native loader resolves `slUpgradeInterface`; `Session::upgrade_interface`
exposes the official manual-hooking entrypoint for newly created D3D/DXGI
interfaces. It requires an initialized D3D session and a non-null interface slot,
and preserves SDK results and pointer replacement. Caller owns COM/reference
semantics and must invoke it immediately after creation as the SDK specifies.
Host tests cover pre-init/post-close rejection, null slot rejection, replacement
and SDK errors. Windows compilation passes. This is not yet connected to wgpu
factory/swapchain creation and does not prove FG presentation interposition.

`SuperResolutionResources` owns the four SR texture leases, checks input size and
binding usages at import, prepares shader-read/UAV tracker states in wgpu, calls
the grouped SR evaluation method, and transfers all leases into native fence
ownership at submission. This removes the need to assemble the retention list by
hand for this path. The unsafe contracts still require common device ownership,
serialized queue order, matching camera/options and correct SDK state restoration.
The integrated resource path passes Windows Clippy; NVIDIA execution and renderer
startup wiring remain pending.

`Recorder::evaluate_super_resolution` groups camera constants, color/depth/motion
input tags, output tag and SR evaluation on the same frame token/viewport/list.
It rejects misaligned input sizes and aliased resources before SDK calls. Tags
use until-evaluate lifecycle with the recording command list. This unsafe helper
requires initialized inputs in shader-read state, correctly sized UAV output and
external queue/state/lifetime protection. Windows Clippy passed; actual SDK frame
evaluation and SceneSurface wiring remain pending.

`ProcessedColorTarget` allocates a linear RGBA8 UNORM or RGBA16 float texture with
storage, sampling, render attachment and copy usages for processing/composition.
Zero and device-limit-exceeding dimensions are rejected before allocation.
The RGBA8 output was exercised by byte-exact Metal blit readback (all 16 pixels),
including zero-size rejection. HDR allocation/processing, tone mapping and native
DLSS writes remain unverified; producer initialization is required before use.

The windowed temporal smoke now composites `TemporalFrame::color` to the actual
presentation view with `TextureBlit` from the post-submit callback, rather than
only clearing that view. `SceneSurface::color_format` exposes its presentation
view format for pipeline creation. Apple M4 Max/Metal execution passed callback
alignment, composition submission and injected failure/reset recovery. This is
the temporal color composition path; replacing its source with evaluated DLSS
output remains pending. No swapchain pixel readback was performed in this probe.

`voxy_render::TextureBlit` provides fullscreen nearest-sampled color composition
from a single-sample float 2D texture into a color target. It has no depth, alpha
blend or HDR tone mapping; format-defined color conversion still applies.
`blit_smoke` passed on Apple M4 Max/Metal: all 16 pixels of a 2x nearest-scaled
four-color image matched byte-exact readback, proving orientation and scaling for
RGBA8 UNORM. This supplies an output composition pass; DLSS evaluation/output
ownership is not yet connected to it.

The submitted surface hook now also receives the presentation texture view, so
consumers can queue composition of processed output before present. The windowed
smoke submits an additional clear pass from this hook on successful frames; it
passed on Apple M4 Max/Metal with consumer-failure recovery. This verifies a
post-scene rendering insertion point, not DLSS output itself or pixel readback.

The temporal surface probe supports strict `VOXY_TEMPORAL_BACKEND` selection
(`auto`, `metal`, `dx12`, `vulkan`, `gl`) and uses winit's owned display handle.
It passed both Vulkan and OpenGL runs on Linux Mesa 25.0.7 llvmpipe (CPU software
rendering), including injected consumer-error recovery. Early redraw events no
longer overwrite initialization errors. A Windows target check also passed before
the display-handle adjustment. These are surface lifecycle results, not NVIDIA
hardware, shader output comparisons or DLSS execution results.

`cargo run -p voxy_render --example temporal_surface_smoke --offline` now provides
a physics-independent windowed test. On Apple M4 Max/Metal it passed preparation,
submission and publication ID/reset agreement, injected consumer failure (no
published temporal inputs), and reset recovery on the next successful frame.
Readiness skips are bounded and do not count as checked frames. This verifies the
surface hook/error lifecycle on Metal, not NVIDIA SDK evaluation or DX12 interop.

The submitted SceneSurface temporal hook is fallible. Returning an error skips
presentation, keeps the presented-frame counter unchanged and invalidates temporal
history for the next attempt. `RendererError::TemporalConsumer` can carry an SDK
failure message. Already-submitted GPU commands are not cancelled, so SDK/native
owners must still protect resources through completion. The basic render APIs
continue using a successful no-op hook. App runtime verification remains blocked
by the missing physics skin shell module in the current workspace.

The general renderer now exposes `SceneSurface::render_scene_with_temporal_hooks`:
one callback prepares resources in the scene encoder, and a second receives the
same candidate temporal inputs plus device/queue after submission, before present.
This is the insertion point for standalone native SDK work ordered after scene
passes. It does not guarantee CPU completion or implement DLSS output composition.
The existing single-callback API delegates with a no-op submitted callback.
Focused renderer Clippy passes. Workspace formatting is currently blocked by an
unrelated missing `crates/physics/src/skin/shell.rs`; package formatting succeeds.

`Recorder::uav_barrier` records a resource-specific UAV ordering barrier with no
state change. The texture probe now exercises shader-read -> UAV, a UAV barrier,
then copy-source -> shader-read before readback. Native resource references are
released after barrier recording; the supplied texture lease protects GPU use.
Windows Clippy passes. This adds a test path, not a verified Windows GPU result.

Plugin selection and adapter support queries now expose the official SDK's
`kFeatureDLSS_NR` (1004) via C selector/feature bit 16 and Rust
`StreamlineFeature::NeuralRendering` / `StreamlineFeatures::neural_rendering`.
This only requests the NR plugin and queries SDK support on the selected adapter;
it does not implement NR options, model loading or frame evaluation. The default
startup probe leaves NR disabled so an unavailable NR plugin does not prevent
testing SR/RR/FG startup. Operational DLSS 5 remains unverified.

When built with `--features scene-dx12`, `dx12_probe --queue-only` also runs
the scene FG HDR resource preparation/readback check. It converts four RGBA32
HDR pixels into the candidate's retained RGBA16 color, checks all 16 channels,
preserves frame ID/reset, and accepts output-color width four with depth/motion
width two. This path imports real DX12 leases and encodes the production scene
candidate preparation but does not tag an SDK frame or generate presentations.
GPU and map-callback waits have five-second bounds. Failed GPU completion retains
the candidate conservatively. SDK headers remain required to build; queue-only
does not require loading NVIDIA DLLs. Require the `FG SCENE RESOURCES PASS:`
marker for runtime acceptance; compilation alone is insufficient.

The scene FG fixture captures GPU validation errors across resource creation,
preparation and readback. Its PASS marker is emitted only after the scope reports
no validation error and all readback checks succeed. It closes the scope on
ordinary error returns as well, reporting both GPU validation and readback context.

Run `dx12_probe.exe --queue-only` to test DX12 queue/texture interoperability
without loading or initializing NVIDIA DLLs. Supplying an absolute signed
interposer path additionally exercises SDK startup, adapter feature queries,
device registration and shutdown. Both modes perform the texture byte check;
neither renders DLSS/FG frames. SDK headers remain a build dependency for this
example. Windows Clippy and executable linking passed; runtime results remain
unverified here.

`dx12::prepare_texture` uses wgpu 30's public native-interoperability
`CommandEncoder::transition_resources` API to update tracker state and encode a
full-texture barrier for shader reads or storage read/write. `TextureAccess` maps
those two states to the same DX12 state bits used by wgpu HAL. Required texture
usage is checked. Submit that encoder before native work on the same serialized
queue; native work must restore the handoff state before wgpu resumes. The helper
does not initialize texture data, submit work or wait. GPU validation is pending.

The Windows `dx12_probe` example now additionally imports the renderer's native
queue, records a texture state roundtrip, submits it and requires its private
fence to complete within five seconds before SDK shutdown. A timeout/device error
fails the probe. This tests queue ownership/submission when run on Windows; it
does not evaluate DLSS. It writes a 64x4 RGBA texture through wgpu, prepares
shader-read state, transitions natively to copy-source and back, then checks all
1024 readback bytes after completion. Windows Clippy and cross-linking are
checked here; the probe has not been executed on Windows in this environment.

`Recorder::transition_texture` records a DX12 transition for all subresources of
an imported texture. Identical before/after states are skipped. The caller must
know actual states, retain resources until completion and restore states before
wgpu resumes; wgpu trackers are not modified by this native command. The Windows
COM union's temporary resource reference is explicitly released after recording.
Windows compilation/Clippy checks this path; GPU barrier validation is pending.

`dx12::WgpuQueue::from_wgpu` derives the native device and direct queue together
from one wgpu HAL device and retains the wgpu device owner. Its recorder/submission
methods use that pair, allowing native commands to share the renderer's actual
queue without accessing its private command encoder. Caller must still serialize
submissions and reconcile native resource states with wgpu tracking. The API is
unsafe for those remaining obligations. Windows Clippy passes; GPU execution and
automatic renderer integration remain pending.

`RecordedCommands::submit` now executes once on a direct DX12 queue and signals
a private fence. `Submission` retains commands, allocator and supplied texture
leases. Nonblocking `is_complete` queries completion and reports device removal.
Pending/error drops conservatively retain dependencies for process lifetime,
including after a signal failure following execution. This intentionally favors
GPU lifetime safety over reclaiming unproven work. Caller still supplies complete
resource ownership, correct barriers, queue ordering and separate retention for
later SDK/presentation uses. Windows Clippy passes; no Windows GPU execution has
been performed.

Rust `dx12::Recorder` now creates its own direct command allocator/list on a
supplied registered device, exposes a scoped recording pointer for barriers/tags,
and records frame evaluation. `finish` closes the list and transfers list plus
allocator into `RecordedCommands`. There is no allocator reuse. Submission and
fences remain external unsafe obligations; keep owners/resources through GPU
completion. This avoids relying on wgpu's private command-list field, but does
not yet implement engine queue ordering/resource-state handoff. Windows Clippy
checks the native creation/close/evaluation path; runtime verification is pending.

`Dx12TextureLease::retain_until_queue_done` consumes resource leases and keeps
them inside the wgpu queue completion callback. Register only after all uses
have been submitted and SDK use/presentation has ended; continue polling the
device. Later submissions and external queues are not covered by this callback.

Current wgpu 30 DX12 HAL exposes native texture/device handles but its encoder's
`ID3D12GraphicsCommandList` field is private with no public accessor. Therefore
the raw command-list evaluation API is not yet wired into the engine's wgpu
encoder. A supported native recording/submission path and resource-state
integration remain necessary before claiming operational DLSS rendering.

With Rust `wgpu-dx12`, unsafe `Dx12TextureLease::from_wgpu` obtains a texture's
native DX12 resource and retains both a wgpu texture clone and a COM reference.
`tag` builds a full-size descriptor for a typed buffer role. Only single-layer,
single-sample 2D textures are accepted. Explicit texture/device destruction and
native/wgpu state synchronization remain caller responsibilities; keep the lease
through SDK use and GPU completion. This is resource access, not an automatic
barrier/fence integration. Windows Clippy verifies the actual wgpu 30 HAL API;
NVIDIA runtime execution remains pending.

RR configuration is exposed by `voxy_streamline_configure_rr` and Rust
`StreamlineRuntime::configure_ray_reconstruction`. `RayReconstructionOptions::from_view`
constructs SDK matrices from a rigid engine camera. Native validation also checks
both matrix products against identity using double arithmetic and a relative
roundoff tolerance, before calling SDK functions. Tests cover translated inverse
pairs, mismatched inverses, singular matrices, nonfinite values and malformed SDK
render-size recommendations. This configures RR; it does not generate its buffers
or execute a GPU frame.

`voxy_streamline_frame_tag_dx12` / unsafe Rust `StreamlineFrame::tag_dx12`
accept up to twelve distinct texture tags (depth, motion, HUD-less color,
scaling input/output, normals, roughness, diffuse/specular albedo, packed
normal/roughness and diffuse/specular hit distance). Rust exposes SDK role values
through `TextureRole`; C++ compile-time checks guard the added numeric ABI values.
Native validation rejects duplicates, invalid lifecycles,
unspecified states, empty or overflowing extents, and volatile tags without a
command list. Descriptors are translated into official SDK resource structures
and submitted with the same frame token. Resource ownership and GPU fences
remain the caller's responsibility. The renderer does not yet generate the RR
material/hit-distance textures or configure RR options. This bridge is compiled
for Windows, not GPU-runtime verified.

`voxy_streamline_frame_evaluate_dx12` and Rust
`StreamlineFrame::evaluate_dx12` record SR/DLAA or RR using the frame's actual
SDK token and a viewport input structure. FG and Reflex selectors are rejected;
FG needs presentation integration. The Rust entrypoint is unsafe: callers must
supply a live recording command list on the registered device, matching tags
and constants, resource states/barriers, submission synchronization and GPU
lifetime protection. No command list is submitted by this method.

Windows C++ compilation/linking and Windows Rust Clippy passed against the
official 2.14.1 headers. Execution with NVIDIA plugins, resource tagging and
engine command-list integration remain pending.
