# Textured imported rig with FEM pads

The existing --cesium snapshot mode now preserves primitive boundaries, UVs, base color factors, texture image indices, samplers and mipmap selection. Its decoded images use the same shared bounded GLB path as editor import; decoded images, model data and GPU textures retain separate owners. Per-frame geometry remains CPU skinning through ModelAsset::scene_meshes, followed by the existing SceneRenderer. Diagnostic geometric vertex lighting modulates the original material colors; this is not a full PBR demonstration.

ModelAsset::decode_embedded_images checks source bytes, image count, image-view bounds against both the declared buffer and actual BIN data, codec/dimension limits and cumulative decoded storage plus each image decoder's pixel budget. It performs no external reads. GLTF/GLB imports with external image resources retain the existing editor provider/dependency path.

Evidence:
- 3 render-model tests passed: real pinned asset import/animation, images/material index correspondence, source/count/dimension/pixel admission, external URI rejection, invalid view rejection and shared budget exhaustion across duplicate image entries.
- All 180 editor tests passed with normally ignored GPU checks included, no failures or ignored tests.
- Actual Apple M4 Max / Metal offscreen run rendered 41 frames over the two-second imported clip, with validation scope clean. Four-pose strip visually inspected; actual texture and moving UVs are visible. GIF is assembled directly from those PNGs.
- Physics CSV is byte-identical to the previous neutral-material run. Frame stepping receipts are unchanged. No physics parameters or admission tolerances were changed.
- Simulation plus render/readback/save took 9.128099 seconds in this single run; realtime remains unproved.
- git diff --check clean for changed source.

Run:

```sh
CARGO_TARGET_DIR=/tmp/voxy-moving-supports-final-20261005 cargo run --release -p voxy_app --example body_motion_snapshot -- OUTPUT.png FRAME_DIRECTORY --cesium
```

Asset provenance and attribution are retained in assets/animation/cesium-man/LICENSE.md and SOURCE.md. This remains a neutral diagnostic pad demo: character surface contact and whole-character soft deformation are absent. Native interactive rendering, full PBR, other physical GPU backends and realtime performance were not qualified by this run.
