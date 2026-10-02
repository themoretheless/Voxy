# Closed grasp surface classification through the shared BVH

`TriangleMesh::contains_closed_surface` traverses its existing BVH using
oriented ray crossings. Nonfinite queries fail; ambiguous near-edge, vertex,
parallel or nonfinite intersection arithmetic returns `None`. The caller must
validate closed consistently oriented topology, including reversed cavity shells.
`TriangleMesh::new` itself does not certify this topology. Crossing count is
oriented rather than parity, retaining reversed cavities and winding multiplicity.

`GraspObject::distance` uses this query after its unchanged exact nearest-surface
query. Ambiguities fall back to the existing full solid-angle winding sum. Mesh
construction, topology validation, bounds shortcut and distance magnitude remain
unchanged. No second spatial tree, query cache or per-query allocation is added.

Initial isolated verification compiled the actual hair module directly and
passed eight tests (seven existing contact/direct-solver regressions plus a
closed tetrahedron inside/outside/vertex/nonfinite test). Additional persistent
cavity and ambiguity regressions have now been added to the physics source.
The application test compares classification against full winding at 5,000
points in the actual grasp handle bounds and additional face-normal probes
one micrometre on either side of its surface. It also tests nonfinite input and
a boundary vertex; existing imported mesh cavity/topology tests are included.

The application suite completed: three imported grasp mesh tests passed, zero
failed/ignored. All 5,256 handle probes were unambiguous and matched the full
winding reference. In this debug run on this machine, classification alone took
90.237625 ms through the BVH versus 231.066 ms for the winding sum (about 2.56x).
This excludes nearest-distance queries and does not claim frame-rate or complete
solver speedup. The expanded isolated module suite also completed: ten tests
passed, including nested cavity and vertex ambiguity regressions. Logs:
`/tmp/voxy-hand-bvh-classification-tests.log` and
`/tmp/voxy-hair-bvh-isolated.log`. The two earlier full grasp processes are
separate old-executable baselines and do not validate the new complete solver.

The updated goal prioritizes the engine's essential feature path. After closing
this check, next work is scene-owned animation playback in editor Play using
existing model revision ownership and GPU skinning, followed by scene/prefab
persistence, fixed game lifecycle and standalone builds. Detailed prototype
thumb tuning is not a prerequisite for that integration, and remains open.
