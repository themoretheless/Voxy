# Textured Fox rig acceptance

The original Khronos Fox exposed two import gaps: animated materials with a base
color texture were rejected, and missing NORMAL attributes were rejected. The
asset is now loaded unchanged: 24 skin joints, 26 palette nodes, 1728 vertices,
576 triangles, a 1024x1024 texture and Survey/Walk/Run clips. Its revision, binary
hash and original attribution are retained under examples/assets/fox.

ModelPrimitive retains base-color image/sampler metadata. The renderer parser
performs no image I/O. Editor import resolves images through the existing bounded,
observed input path, including external PNG dependencies. The existing residency
cache and texture-binding implementation are shared with static materials.
Opaque TEXCOORD_0 materials are supported; other maps and textured alpha modes
reject explicitly. This does not constitute complete glTF PBR support.

A zero normal stream denotes missing authored normals. The editor shader derives
flat normals from the current posed triangle via world-position derivatives,
which respects the glTF missing-NORMAL rule even with differing corner weights:
https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html . CPU baked and authoring
preview meshes preserve that stream. Other custom shaders must implement this
fallback; zero-normal agreement is not a surface angular-error certificate.
The legacy skinned shader avoids normalizing zero and has a geometry fallback;
its source and NOOP pipeline are checked, but legacy GPU pixel output is unproven.

Animation requests use the GPU-published model revision. Each accepted owner
retains its matching material bindings and storage roots. Failed revision or
budget admission preserves old pose, geometry and texture together. Storage
roots keep the weak residency-cache entry alive until the owner's accepted
material is replaced or removed. A real-GPU regression checks this lifetime,
missing-material rejection and successful retirement. Source geometry/palettes
remain independent per owner while image bindings are shared.

Three-clip GPU acceptance samples 65 poses per clip, checks CPU positions and
normal streams, renders with the editor shader in sRGB, and requires visible
textured pixels on every frame. Maximum position error is 1.9073486e-5 in asset
units. All three midpoint images were read back and visually inspected in a side
view. Native release acceptance checks all three inspector clip choices, Run
playback, independently paused owner, shared image binding, undo/redo and Stop:
335200 logical animation bytes during Play, zero after Stop, authoring restored.
No native-window screenshot or pointer-driven UI acceptance is claimed.

216 ordinary tests, five renderer GPU regressions and four editor GPU regressions
passed. Shader source validation covers 50 files / 55 variants / 94 entrypoints.
Final native binary is a separate hashed snapshot preceding later legacy shader
source editing and formatting; native scene-material acceptance is unchanged.
Evidence and images: artifacts/fox-animation-2026-10-03/.

Subsequent local work adds separate bind-pose authoring draws for multiple
primitives, each using its published texture binding. Additional preview buffers
are preflighted and counted as mandatory geometry; single-primitive models reuse
the existing base geometry. A two-color primitive regression verifies ordering,
color preservation and exact allocation accounting with the NOOP device. Native
visual acceptance now covers two independently bound textures before Play, Run
selected through the inspector, and Stop through actual pointer input. The
magenta/cyan viewport masks match exactly before Play and after Stop; both remain
visible during Run. See artifacts/rig-material-preview-2026-10-03/report.json and
its three inspected screenshots. The deterministic fixture generator splits
the pinned Fox triangles and adds a diagnostic framing parent. This proves the
material path and authoring restoration, not full glTF material support.

The same acceptance exposed implicit demo Spin attached to ordinary scene roots
on Play. That behavior has been removed: authored AngularMotion and explicit
registered gameplay behavior remain the sources of motion. Fixed-loop regression
now requires an unconfigured object to retain its rotation; the separate authored
motion regression continues to require its configured two radians of rotation.

Limits remain: rich material maps,
alpha modes, influence counts above four, morphs, complex humanoids, retargeting,
blending/root motion, full native-frame profiling and non-Metal/CUDA verification.
