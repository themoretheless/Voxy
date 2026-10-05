# Imported character collision admission

The existing body_motion_snapshot example accepts `--cesium --contact`. It extracts world-space collision vertices from the same ModelAsset pose and scene_meshes path used for display, retains all primitive triangle indices, binds the finite surface to all continuum regions, and samples the actual imported obstacle pose alongside the bone palette on the same frame clock. No collision proxy or second animation runner is used.

Actual Metal probe: Apple M4 Max; 3273 vertices, 4672 triangles. Full contact binding rejected the existing pad placement with `closed surface contact gap` before rendering a contact snapshot. The pads overlap the original character mesh. This is an unsuccessful admission result, not a successful collision animation.

The extraction regression samples phases 0, .25, .5, .75, 1, verifies unchanged topology, valid finite geometry, moving vertices and exact equality to displayed world positions. It also requires the current overlapping setup to reject without changing the energy receipts.

Remaining: define attachment-region contact exclusions or replace overlapping character regions with the deformable surface, preserving stable source triangle ownership. Do not remove arbitrary intersecting triangles or weaken the separation barrier to force acceptance. Existing textured GIFs predate contact integration and remain non-contact demonstrations.
