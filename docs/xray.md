# X-ray reveal

Run `cargo run -p voxy_app --example xray` to use the imported
`assets/characters/blender-female/body.obj` as the exterior. `F` switches between
full-body and pelvic close-up views. Pass `--abstract` for the procedural shell.
`X` switches between opaque exterior and blue ghost shell with an orange internal
muscle ring. `S` sets simulation speed to 15%; Space pauses; left/right arrows
orbit the model; Esc exits. `--smoke` exercises both modes, slow motion and window
resizing before exiting after 120 presented frames.

The muscle surface follows the existing fixed-step XPBD sphincter sample in
`physics::tissue`; the default exterior is the imported body surface. The internal ring is
scaled and placed inside the lower pelvis as an illustrative attachment, not
an organ imported from the OBJ. The abstract mode uses an ellipsoid. This
sample does not include an anatomical pelvis, skeletal damage or medical calibration.

For other models, upload exterior and internal meshes independently, then call
`SceneGeometry::set_depth_mode`:

- `Opaque`: normal depth testing and depth writes.
- `Transparent`: world depth testing, alpha blending, no depth writes.
- `Xray`: draws through world occluders with alpha blending and no world-depth
  writes; `SceneSurface` uses isolated depth for internal self-occlusion.

`SceneSurface` automatically allocates and caches an isolated X-ray depth target
only when a frame includes revealed geometry. Resize invalidates that target.
Opaque internals then self-occlude independently of draw order while world depth
remains unchanged. UI is composed last.

For offscreen rendering use `SceneRenderer::encode_with_xray_depth` and provide a
separate `Depth32Float` view with the same dimensions as your color target. Do not
alias it with world depth. The simpler `encode` retains its painter-order X-ray
behavior for compatibility.

`SceneRenderer::encode` draws opaque objects, transparent shells, X-ray internals,
then UI overlays. Each layer preserves caller order. Sort transparent meshes and
triangles back to front for the current camera; `SceneMesh::sort_back_to_front(model_to_view)` provides stable triangle-centroid
sorting with a finite, invertible affine matrix (camera looks down -Z). It rejects
invalid transforms without changing indices. It cannot resolve intersecting
triangles or order separate meshes. The sample sorts its shell every
frame and uses view-dependent rim opacity. Vertex colors use straight linear RGBA;
use a white texture for untextured models. X-ray is deliberately visible through
all scene occluders. Use it only for selected internals, not ordinary world meshes.

Transparent and X-ray geometry is excluded from the surface motion/depth inputs:
these passes are visual layers, not reliable temporal reconstruction geometry.

`cargo run -p voxy_render --example xray_smoke` reads pixels and depth back from
the real GPU to verify ordinary occlusion, through-body reveal, transparent
blending, and unchanged depth for the two non-writing modes.

## Reusable effect controller

```rust
use voxy_render::{XrayEffect, XrayStyle};

let mut effect = XrayEffect::new(XrayStyle {
    transition_seconds: 0.3,
    ..Default::default()
})?;
effect.set_enabled(true);
effect.advance(real_delta_seconds)?;

shell_geometry.set_depth_mode(effect.shell_depth_mode());
internal_geometry.set_depth_mode(effect.internal_depth_mode());
// Apply these colors when building/updating mesh vertices:
let shell_rgba = effect.shell_color(original_shell_rgba, fresnel_weight);
let internal_rgba = effect.internal_color(original_internal_rgba);
```

Use original colors on each update to avoid accumulating tint. The controller
preserves source transparency, supports HDR nonnegative RGB, clamps Fresnel weight,
and validates styles and time deltas before mutating state. Zero duration switches
instantly. Reversing `set_enabled` mid-transition preserves the current amount.
Advance reveal with real time and physics with scaled time for slow motion.

For semitransparent internals, sort back to front even with isolated depth. All
X-ray objects in a frame share this isolated depth buffer and can occlude each
other; independent object groups require separate offscreen passes. This renderer
uses explicit internal geometry, not volumetric scanning or physical X-ray transport.

## Regional reveal

`XrayRegion::sphere(center, radius, feather)` defines a local internal selection.
The smoothstep feather runs inside the sphere radius. Nonfinite positions and
invalid region parameters are rejected; large finite coordinates are supported.

```rust
use glam::{Mat4, Vec3};
use voxy_render::{XrayRegion, SceneDepthMode};

effect.set_region(Some(XrayRegion::sphere(Vec3::ZERO, 0.4, 0.08)?));
let reveal = effect.reveal_mesh(&original_internal_mesh, model_to_region)?;
reveal_geometry.update(queue, &reveal)?;
reveal_geometry.set_depth_mode(SceneDepthMode::Xray);
```

`reveal_mesh` leaves the source unchanged and samples the mask at vertices.
Use sufficiently fine tessellation: a sphere inside a single large triangle will
not appear if all its vertices lie outside. Boundaries interpolate across triangle
faces; this is not per-fragment clipping. The region affects `reveal_color_at` and
`reveal_mesh`, while ordinary `internal_color` and shell tint remain global.
Reveal-layer alpha fades with activation and regional weight. Keep ordinary
internal geometry as a separate world draw if you need it outside the reveal or
when disabled. The default shader discards zero-alpha fragments before depth
writes; custom scene shaders must implement the same discard policy.

Press `L` in the native demo to restrict the internal ring to a feathered local
sphere. The outer shell remains globally transparent in this demonstration.
