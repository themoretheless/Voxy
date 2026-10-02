# Blender Studio realistic female character

Source: [Blender Human Base Meshes v1.4.1](https://www.blender.org/download/demo-files/#assets), official [download](https://download.blender.org/demo/asset-bundles/human-base-meshes/human-base-meshes-bundle-v1.4.1.zip).
License: **CC0 1.0 Universal**, as specified by Blender's Human Base Meshes asset-bundle listing. [CC0 legal text](https://creativecommons.org/publicdomain/zero/1.0/legalcode).
Original realistic female body artist: Dan Ulrich; collection published by Blender Studio and community contributors.

The model is an anatomical adult mannequin, not a likeness of an identified person. The source asset has no hair, photographic textures, clothing or animation rig. The example uses neutral opaque shading; it should not be described as photorealistic.

Derived files:

- `body.obj`: evaluated level-one anatomical mesh, converted to Y-up metres and centered vertically.
- `eye-l.obj`, `eye-r.obj`: separate eye geometry with the same coordinate transform.
- `full-body-skin.json`: reduced full-body shell, tangential reference directions and complete displacement bindings; 636 nodes, 1268 triangles, 42,342 bindings. Generated without modifying the detailed OBJ.
- `abdomen-skin.json`: actual level-zero abdominal patch, boundary pins and barycentric bindings into the detailed body. It contains no personal measurements.

To reproduce, download/extract the official source and run the installed Blender:

```sh
Blender --background --factory-startup --disable-autoexec --python tools/export_female_character.py -- /path/to/human_base_meshes_bundle.blend assets/characters/blender-female
```

The export disables auto-execution of embedded scripts. Neutral material shading is applied by the native example, not baked into the source model. Redistribution and adaptation are covered by the source CC0 grant.

Official ZIP SHA-256: `811f43accbb31a88266d932f8f5563b2d13586fca0ba2693aad1f5fe582b3515`.

## Runtime rig

The native `female` example adds a procedural 46-joint humanoid skeleton in
`crates/voxy_app/src/female_rig.rs`. Body vertices receive four normalized
anatomically restricted influences, including separate hip, knee and ankle bands.
Each finger has three additional joints. Local finger bending uses continuous
weights across the shared webbing and is applied before the original body binding;
it preserves the existing wrist/palm transition. Finger motion trails the wrist,
with a smaller thumb angle. All thirty finger joints have an isolated real-mesh
articulation regression; the complete clip also checks triangle areas and edge stretch.
A six-second looping clip brings the arms forward, flexes the elbows and turns
the head. A small spine/chest turn anticipates the gesture; shoulder, elbow and
wrist motion starts in sequence. The elbow hinge axes follow the
upper-arm direction in the source A-pose; a quintic gesture envelope adds rest
and hold phases with smooth acceleration.
Ordinary skeletal and full-physics gesture playback keep the root stationary;
the separate secondary-motion demonstration retains its prescribed vertical
excitation. A full-model regression samples the foot vertices throughout the
ordinary clip and checks that the gesture does not move their support positions.
Weights blend only connected anatomical regions. The full-body shell is solved in world space around moving skeletal attachment
targets. Its displacement relative to the posed support is added after render
skinning; normals are reconstructed from the final deformed surface.
This is an approximate runtime rig, not artist-authored Blender weights or IK.

Run `cargo run --release -p voxy_app --example female_render -- --hands --sequence`
for a camera-tracked close-up of the articulated hand across the loop.

Run `cargo run --release -p voxy_app --example female_animation` for the skeletal preview without the nonlinear skin solver.
Run `cargo run -p voxy_app --example female` for the full physical character. Space pauses both animation and
skin simulation. The source OBJ remains in its original bind pose.

## Simulated hair

The native examples use the library's implicit Cosserat guide rods, including
material-frame bending/twist, density-derived mass, actual animated triangle
contacts and capsule self-contact. Roots bind to the animated scalp vertices.
See [hair mechanics](../../../docs/hair.md) for parameters, validation and limits.
Each of the 203 physical guides carries 20 rendered follower bundles. Follower
offsets are transported by the guide material frames; followers are not
independent physical rods. The rest groom and follower offsets are projected
outside the anatomical surface before simulation.
Front guides form a short fringe; side and back guides retain their longer
lengths. Guide paths are truncated by measured arc length before resampling;
scalp-to-envelope transitions cannot extend the requested fringe length.
Rendered bundles have a 0.6 mm base diameter tapered toward the tips,
while the physical guide material retains its 80 micrometre fibre diameter.
The thicker display geometry improves coverage in full-body views and is not
the physical contact radius or an independently simulated bundle model.

Run `cargo run --release -p voxy_app --example female_animation` for the
skeletal preview. In this mode the rest groom follows the head rigidly and
neither hair nor skin physics advances. Run the `female` example for both
physical solvers. Space pauses playback in either mode.

The full six-second physical hair regression is
`cargo test --release -p voxy_app --lib female_hair::tests::complete_rig_loop_preserves_hair_and_scalp_bindings -- --nocapture`.
It checks moving scalp bindings, finite states and guide strain throughout the
cycle. This solver check does not establish native presentation frame rate.

Character presentation uses four-sample MSAA color/depth attachments, resolved
into the window image. Temporal inputs retain single-sample buffers. Offscreen
`skin_render` uses the same coverage smoothing by default; `--no-msaa` renders
the single-sample comparison. This changes raster coverage, not hair mechanics.

## Additional face features

The facial atlas also contains subtle forehead creases, crow's feet, nasolabial
folds and small superficial vessel markings. Wrinkles contribute shallow
bind-space relief. A modest view-dependent vertex highlight adds skin/lip sheen;
this remains an illustrative material, without subsurface scattering or a full
PBR skin shader.

`female_features.rs` adds 180 eyebrow hairs, 64 upper lashes and 220 short vellus
hairs. Roots attach to actual body vertices after facial, skeletal and shell
deformation. Lash directions follow the blink and strand offsets follow the
head rotation. Separate rounded tiles form two eight-tooth arches, together
with an approximate tongue and dark rear oral surface. The lower arch and tongue
follow the jaw. These are simple visual meshes, not anatomical dental assets or
simulated oral tissue.

The sealed lip strip in the render mesh is replaced with clipped upper/lower
surfaces. Their neutral area matches the original strip; the jaw separates their
shared rim smoothly. This construction runs with constant animation topology,
without editing the source OBJ or adding oral geometry to the physical shell.
