# GPU skeletal LOD stream ownership

`SceneSkinner::create_lod_level` validates a `SkinnedLodMesh` against the instance's
exact vertices, skin attributes, base indices and palette size before allocating
one selected index buffer. The level exposes only a shared geometry reference;
its vertex, normal, material-coordinate and material-parameter buffers are the
same buffers as the instance's base geometry. The existing compute pass therefore
updates all resident levels without another skin dispatch or posed vertex copy.

The instance owns deformation state and must remain alive while these levels are
used. Logical admission counts the instance's shared streams once, then each
level's `index_allocation_bytes`. Source mismatches, foreign skinners, nonexistent
levels and failed admission return before altering the accepted instance.

The renderer API supplies GPU residency support. The editor prepares the exact
current pose with `SkinnedLodMesh::prepare`, selects against the current
camera/world transform and maintains independent instance/view history.
An immutable rest certificate alone cannot justify animated projected error.
The editor admits optional levels before publishing instance/view history and
retains previously accepted history on failure. GPU levels share deforming
streams; the explicit CPU route rebuilds selected posed levels after a pose
change. Source replacement resets optional residency and view history.
Optional index levels selected by any live view stay pinned. Unselected
levels retire after the complete view-selection pass; closed views retire
before admission. Source replacement, Stop and owner removal retire all levels.

Validation: the GPU regression uses a certified two-triangle source and a
one-triangle variant. Both geometries share vertex/normal/material streams,
while the variant adds exactly 12 index bytes. Compute-deformed positions read
through the variant buffer match the CPU reference. Admission one byte below
the requirement, invalid level, changed source and foreign skinner are rejected.
One GPU test passes; 126 ordinary renderer tests pass, six hardware tests are
ignored in that ordinary suite. Logs are retained in
`artifacts/scene-skeletal-lod-streams-2026-10-03/`.

The editor's native skeletal LOD fixture now proves reduced/base camera
selection and optional residency retirement. See
`artifacts/editor-skeletal-lod-native-2026-10-03/report.json` for measured scope.
