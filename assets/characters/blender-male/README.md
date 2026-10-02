# Blender Studio realistic male reference body

Source: [Blender Human Base Meshes v1.4.1](https://www.blender.org/download/demo-files/),
[official ZIP](https://download.blender.org/demo/asset-bundles/human-base-meshes/human-base-meshes-bundle-v1.4.1.zip).
License: CC0 1.0 Universal, per the official asset-bundle listing.
ZIP SHA-256: `811f43accbb31a88266d932f8f5563b2d13586fca0ba2693aad1f5fe582b3515`.

The source object is `GEO-body_male_realistic`; this is separate male anatomy,
not a breast-size edit of the female source. Export normalizes height to 1.64 m
for compatibility with the parameter library's reference height, converts to
Y-up metres and centers vertically. This normalization is not population data.
The source reference and evaluated normals are retained in `body.obj`.

Reproduce using Blender background mode with auto-execution disabled:

```sh
Blender --background --factory-startup --disable-autoexec --python tools/export_female_character.py -- SOURCE_BLEND assets/characters/blender-male male
Blender --background --factory-startup --disable-autoexec --python tools/export_full_body_skin.py -- SOURCE_BLEND assets/characters/blender-male/body.obj assets/characters/blender-male/full-body-skin.json male
python3 tools/refine_body_landmarks.py assets/characters/blender-male/body.obj assets/characters/blender-male/body-refined.obj
python3 tools/bind_refined_body.py assets/characters/blender-male/body-refined.obj assets/characters/blender-male/body.obj assets/characters/blender-male/full-body-skin.json assets/characters/blender-male/body-refined-skin.json
```

Binding preparation requires NumPy. The runtime mesh has 44,880 vertices and
89,756 triangles, with complete displacement bindings to its own 636-node,
1,268-triangle shell. Appearance uses shared illustrative landmarks and
procedural skin materials. These are not physiologically calibrated.

```sh
cargo run --release -p voxy_app --example female_render -- --male --chest --no-surface-diffusion --body-preset assets/characters/blender-male/presets/nipple-cold.json --snapshot /tmp/voxy-male-chest.png
```

The existing example name is retained; `--male` selects the separate source.
Male hair rendering/simulation is disabled in this constructor. Rig, face,
regional tissue properties, pressure and contact models still need independent
male validation. The `body_model` parameter selects the source through MCP and the viewer reload
path; native desktop presentation of switching is not yet visually verified.
This is an adult anatomical mannequin with neutral rendering, not a likeness
of an identified person or a validated medical model.

The default male constructor now uses `body-repaired-render-candidate.obj` and
`body-repaired-skin-candidate.json`. The candidate suffix/header and earlier
audit `adopted:false` fields describe preparation history; the current
constructor is authoritative. Original `body-refined.obj` and bindings remain
for provenance and regeneration. Stock female/male switching uses composed
`female-to-repaired-male-film-map.json` / `repaired-male-to-female-film-map.json`.
Static and sampled-pose/morph geometry audits are documented in
`docs/body-development-status.md`; exhaustive dynamics/parameter validation
is still incomplete.
