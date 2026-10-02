# Native static 3D project

```sh
cargo run -p voxy_editor -- --manifest crates/voxy_editor/examples/scene3d/assets.json assembly --scene crates/voxy_editor/examples/scene3d/scene.json
```

Copy this directory before experimenting with Save: **F5 writes the supplied scene file**.
The original GLB and checker texture are generated locally by `python3 generate.py`.

- Right drag: orbit; middle drag: pan; wheel over viewport: zoom.
- F7: perspective/orthographic; F8: legacy view; F: focus selected origin.
- G: save the current camera as the scene's single camera descriptor; F5 saves to disk.
- H: expand an unexpanded glTF resource into editable nodes. Mesh and texture data remain in the shared resource, while nodes own transforms and durable part indices.
- M: material inspector (RGB tint, opaque alpha 1, Lit 0/1). Texture status is shown below.
- L: add/remove a world-direction light on the selected object. First active light is used.
- I: physics inspector; C/B: character/box component; transform fields edit collider pose and scale.
- F6: Play/Stop; arrow keys: movement; W/S: depth; Space: jump. Stop restores authored poses, including the intentionally overlapping spawn.
- D: duplicate selected node; P: parent; Z/Y: undo/redo; F9: load.

This fixture includes two textured cube nodes, a rotated/scaled collider, a floor,
and an initially overlapping character to exercise bounded runtime recovery.
The saved camera is separate from model transforms. Viewport gestures do not
change the document until G is pressed.

Supported import: static triangle glTF/GLB, finite TRS nodes, multiple material primitives per
mesh node, opaque base-color factors and PNG/JPEG textures. External normal
relative paths are tracked as dependencies; traversal/data URIs, sparse accessors,
skins, animations, morphs, required extensions and other texture channels are
rejected. The editor imports all declared nodes, rather than selecting among
multiple glTF scenes. Hierarchy changes with existing part references retain the
last good resource; remove/reimport the expanded container explicitly.

Lighting is double-sided flat Lambert, with one directional light and ambient
0.2. No shadows, PBR, smooth normals or blended transparency are claimed.
Texture addressing and all six glTF minification filters are imported. Mip-filtered
textures upload a generated mip chain. Repeated mesh primitives share GPU geometry;
identical image/sampler combinations share GPU texture bindings. Different sampler
combinations use separate views and bindings over the same GPU image allocation.
Base-only views expose one level even when another material requires a mip chain.

Characters remain axis-aligned under translation-only ancestors. Static box
colliders accept rotation, nonzero scale and inherited shear. Affine-box scenes
use continuous SAT, four slide iterations and a short ground snap; the axis-
aligned backend additionally retains its existing step-up behavior. This is
kinematic character motion, not a rigid-body simulation or moving-platform API.
Runtime overlap recovery has 16 iterations and fails explicitly if unresolved.

Native acceptance:

```sh
python3 tools/test_scene3d_window.py
```

Requires an actual window/GPU and presented frames. It checks both camera
projections/picking, texture publication, saved camera/scene, runtime motion and
recovery, Stop and reload. It does not substitute a headless pass.

Identical decoded images also share GPU storage across model resources in the
same graphics context. A versioned content key covers dimensions and RGBA pixels;
models hold the allocation, and the cache holds weak references. Storage includes
a full mip chain so later materials can reuse it; base-only views still expose
one level. Run `python3 tools/test_scene3d_window.py --shared-images` to exercise
two separate GLB files with shared images and different sampler configurations.

GPU model residency follows scene references. Removing the last reference releases
its model allocation; restored references rebuild from the CPU catalog. Inactive
references release residency; effective activity includes every ancestor. Active
containers alone do not retain GPU models when every renderable part is hidden. The shared-image native test also verifies this
remove/restore path. The image-storage budget defaults to 256 MiB; set
`VOXY_GPU_IMAGE_BUDGET_BYTES` to override it. This accounts for unique RGBA8
textures and their full mip chains, excluding geometry, driver overhead and
in-flight GPU reclamation. Candidates exceeding the budget are deferred while
last-good models keep rendering. Removed scene references release residency.

Model geometry has a separate 256 MiB default budget, configurable through
`VOXY_GPU_GEOMETRY_BUDGET_BYTES`. Preflight includes all five geometry buffers,
selection outlines and aggregate fallback geometry, deduplicating instanced
primitives within a resource. Replacement admission includes the previous live
model. UI/gizmo buffers, driver overhead and in-flight GPU reclamation are outside
this logical model-geometry budget. Rejection preserves the previous model.
