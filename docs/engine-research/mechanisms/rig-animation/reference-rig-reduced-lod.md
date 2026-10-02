# Reduced reference-rig LOD

The pinned research meshoptimizer source produces a position-only shared-vertex
candidate for the unchanged CC-BY-4.0 RiggedFigure: 768 base indices become 510
indices (256 triangles become 170). Vertex/skin data is not rewritten. Its own
relative error estimate is only producer metadata, not a Voxy quality proof.

`certify_skinned_lod` now accepts optional subdivision depth and uses the bounded
indexed witness producer. At depth two, the actual reference candidate needs
84,508 triangle tests and 152,354 node visits; its archive is 284,354 bytes.
The observed editor importer re-verifies source positions/topology, skin domain
and dependencies as before. Runtime preparation still certifies the exact pose.

`profile_skinned_lod` exercises 129 evenly spaced clip times at identity and
nonuniform world scale (1.2, 0.8, 2.0), giving 258 preparations/selections.
Finite conservative geometric bounds range from 0.0576843528 to 0.0746217814
world units. At the one-pixel policy and 600x600 viewport, all near orthographic
views (span two) use base and all far views (span 200) use the reduced level.
This is sampled geometric/camera evidence, not a continuous animation bound or
normal/UV/material/perceptual error certificate.

Release CPU preparation on the current Apple M4 Max/macOS host measures a
945.875 microsecond median and 1,022.167 microsecond p95. These are wall times for
pose-specific certificate creation, excluding offline search and whole-frame
rendering; they do not establish FPS, GPU timestamps or scaling to large rigs.
Debug preparation took a 38,846.291 microsecond median and is retained separately.

The native ordinary editor loads the actual reduced `.vmodel` recipe. Two
owners use 510 indices in the far view (81,216 logical animation bytes); a
perspective near-plane crossing restores 768 base indices and retires 4,080
optional bytes (77,136 remain). Stop at frame 17 retires all animation bytes and
restores the authoring document. The camera is deliberately wide in the far
acceptance phase; native visual legibility/screenshots remain unverified.

Artifacts, source/license metadata, certificate, recipe and per-sample CSVs are
in `artifacts/reference-rig-reduced-lod-2026-10-03/`.
Reproduction starts with `tools/prepare_reference_rig_lod.py --output NEW_DIR`,
then `certify_skinned_lod BASE.glb VARIANT.txt PROOF.lod 2` and the release
`profile_skinned_lod BASE.glb PROOF.lod OUTPUT.csv` example. The native smoke now
reads base/reduced counts from the accepted source instead of fixed test counts.

Production acceptance still needs visual/material/normal checks, broader rigs,
continuous-time guards or conservative interval coverage, frame-level profiling
and additional hardware backends. The complete engine goal remains active.
