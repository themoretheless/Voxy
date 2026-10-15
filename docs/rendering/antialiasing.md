# Polygon antialiasing on Metal

`ArchiveSceneRenderer::set_antialiasing` selects `Off`, `Spatial`, `Temporal`, or
`MetalFx`. The street-walk example defaults to `Temporal`; set `VOXY_STREET_AA`
to `off`, `spatial`, `temporal`, or `metalfx` to compare. Unsupported MetalFX returns
an error and leaves the previous mode intact. It never silently substitutes TAA.
The existing separate `SceneRenderer` MSAA 4x path is unchanged.

## Temporal AA

The immutable polygon renderer applies an eight-phase Halton camera jitter,
composes geometry and sky into a linear RGBA16F target, then accumulates history
before the display transform. Guides reconstruct world positions from the current
jittered depth and project into the previous unjittered camera. Motion is backward
UV displacement, excluding jitter. Sky has rotation-only motion. The TAA resolve bilinearly reconstructs the current colour onto the stable output
pixel grid and accumulates history there. Its guides reconstruct the corresponding
unjittered camera ray; static-camera history therefore stays at the same pixel
instead of being repeatedly resampled with jitter. MetalFX receives guides in its
jittered input grid and handles reconstruction internally.

History uses four bilinear taps, rejecting each tap independently by previous
surface depth. A current 3x3 RGB box clips stale colours; luminance disagreement
reduces history weight. First frames, mode switches, explicit camera cuts and large
camera jumps reject history. `reset_temporal_history` also restarts jitter. New
renderer dimensions create new targets and invalid history. No history is reused
across a renderer replacement.

History tracks consecutive submitted render frames, including offscreen frames;
its camera matrices correspond to those frames rather than wall-clock time or
presentation count. Skip `draw` entirely when no frame will be rendered. Headroom
changes preserve history because accumulation is in linear HDR before tonemapping.

## MetalFX

On macOS 13 or newer with a supported device, `MetalFx` invokes Apple's `MTLFXTemporalScaler` through objc2 bindings on
a dedicated wgpu-owned Metal command buffer. This is temporal AA at **native 1:1 render
resolution**, not a lower-resolution upscaling mode. It receives RGBA16F linear
colour, Depth32F depth, RG16F unjittered backward UV motion, pixel jitter, positive
width/height motion-vector scales and a history reset flag. Output is a retained
RGBA16F shader-write texture; the final pass applies the same SDR/EDR transform.

The native bridge borrows one texture at a time, retaining its native handle and
releasing the wgpu snatch guard before borrowing another. wgpu owns submission,
resource lifetime and command-buffer completion. Scene, native MetalFX and display
command buffers are submitted together, in order, on the same queue; native and
wgpu recording APIs are never mixed on one encoder. A preceding compute pass registers
the native resources with wgpu tracking and initializes the destination; it skips
TAA accumulation in MetalFX mode. All temporary targets use the GPU memory budget.

## Full-scene validation

Run `cargo test -p voxy_render --test archive_antialiasing` on a real Metal adapter.
Tests cover shader validation, SDR/HDR compilation, rendered silhouettes through
all four modes, and static-camera motion remaining zero despite changing jitter.

The window-independent qualification renders the complete source geometry once
loaded, runs all three modes, and saves HDR PFM data plus SDR PNG previews:

```sh
cargo build -p voxy_app --example polygon_aa_validation
MTL_DEBUG_LAYER=1 target/debug/examples/polygon_aa_validation /tmp/voxy-polygon-aa
python3 tools/render-research/qualify_polygon_temporal.py /tmp/voxy-polygon-aa
```

The Python image analysis needs NumPy and Pillow. It checks nonblank finite images,
consistent geometry/headroom, and bounded differences from Off after cuts/resets.
Pixel differences are diagnostic values, not perceptual quality rankings. The
headless route does not step actor physics because its camera path is independent;
all polygon geometry, materials, shadows and rendering passes remain active.

For optional windowed validation on the same original polygon street scene:

```sh
cargo build -p voxy_app --example street_walk
VOXY_STREET_AA=temporal VOXY_STREET_AA_CAPTURE=/tmp/voxy-aa-temporal \
  target/debug/examples/street_walk --offscreen-benchmark
VOXY_STREET_AA=metalfx VOXY_STREET_AA_CAPTURE=/tmp/voxy-aa-metalfx \
  target/debug/examples/street_walk --offscreen-benchmark
```

To run all three modes on one loaded scene, start with `VOXY_STREET_AA=off`,
set `VOXY_STREET_AA_CYCLE=1`, and point the capture directory at a child named
`off`. Sibling `temporal` and `metalfx` directories are created automatically.
Validation fixes render headroom at 2.0 and weather time at zero so window focus
and system display headroom changes cannot alter comparisons. Normal interactive
rendering continues to follow live display headroom.

Each mode renders 160 frames at 1920x1080, saves consecutive static frames plus
rotation, translation, camera-cut and explicit-reset captures, and writes a JSON
report with the adapter, triangle count and completed-frame durations. Use `--bistro`
for the full imported scene instead. No scene geometry is simplified for this check.

## Limits

The archive path owns immutable geometry. Depth-derived guides correctly describe
camera motion on static meshes; animated/deforming objects need previous object
transforms or skinning positions before this implementation can be reused for them.
Transparent surfaces currently use underlying opaque depth for reprojection and
have no dedicated reactive mask. The RGB clipping/luminance rejection reduces
trails but does not prove correct translucent motion. Spatial AA can soften detail;
temporal methods can still exhibit ghosting/disocclusion artefacts. Image-quality
claims must be bounded by the captured scenes and routes.
