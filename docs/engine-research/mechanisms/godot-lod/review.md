# Godot indexed LOD: pinned mechanism review

Repository: godotengine/godot. Commit: `084a2caa05119b625a99b6b51d44b459a26362de`.
`sources.json` records nine downloaded source/license files and SHA-256 digests.
Each saved source retains its original MIT copyright/permission notice. No Godot
runtime code was integrated into Voxy. This review is not a build/performance test.

## Observed source behavior

- `scene/resources/mesh.cpp:1510-1517` reads serialized LOD pairs consisting of
  an edge length and index data into surface metadata. The mesh vertex data is
  stored separately. `add_surface` passes the LOD collection to the server.
- `servers/rendering/renderer_rd/storage_rd/mesh_storage.cpp:419-435` creates
  the base index buffer and all supplied LOD index buffers while adding a surface.
  Each LOD retains its index count, byte size, edge length and index-array handle.
  The shared surface vertex/attribute buffers are outside this loop.
- `mesh_storage.h:485-505` selects a LOD using edge length multiplied by model
  scale and divided by the supplied distance threshold. It falls back to base
  indices if no alternative is eligible. The selected index count is returned.
  These lines alone do not establish how perspective/orthographic callers derive
  the distance threshold, nor validate the ordering of importer-generated errors.
- `mesh_storage.cpp:539-543` frees LOD index buffers when clearing the surface.
  This is surface lifetime cleanup, not selective LOD streaming/eviction.

## Adaptation decision for Voxy

Separate immutable vertex/attribute allocation ownership from index variants.
Instanced scene owners should select a draw variant without cloning the complete
geometry allocation. Keep authored transforms, materials, physics colliders and
selection identity independent of the visual LOD. Preserve renderer-device checks
and transactional publication of every replacement.

Indexed LOD is a draw-work feature, not a guaranteed vertex-memory reduction.
Voxy's SceneGeometry owns five buffer handles. Uploading each LOD through
upload_mesh duplicates vertex streams; SceneLodGeometry instead shares those
handles and gives immutable access to separate index variants.
Count shared vertex storage once and all resident index variants in geometry
admission; selecting a coarse LOD must not pretend unused resident buffers vanished.

For pressure-driven memory reduction, separately support nonresident index
variants and/or independently decimated vertex representations. Keep at least
one valid representation per admitted visible object and avoid dropping the last
working representation before a replacement is admitted. Record quality error and
resident bytes independently; never treat triangle count as byte accounting.

## Required acceptance before implementation is called complete

1. Index validation rejects out-of-range/non-triangle input before GPU creation;
   invalid variant publication retains the prior draw representation.
2. Several scene instances choose different variants over identical vertex handles.
3. Measured GPU buffer sizes count shared vertices once and every live index buffer.
4. Perspective, orthographic, nonuniform scale, near-plane and nonfinite-input
   tests cover the projected-error contract; transitions use explicit hysteresis.
5. Native readback/presented-frame checks verify visual variant changes with
   unchanged material, UV, authoring, picking and physics ownership.
6. Pressure tests distinguish draw-index selection from actual residency release,
   preserve last-good draws and recover after resources are released.

The importer/error path is reviewed below. Remaining source review: renderer
callers of mesh_surface_get_lod and camera-specific threshold derivation. No speedup,
quality equivalence or complete Godot LOD parity is claimed.

## Current Voxy implementation

`LodPolicy` validates ordered errors/counts and selects a level from conservative
projected error with a relative hysteresis band. Coarsening requires target *
(1 - hysteresis); refinement tolerates target * (1 + hysteresis). Per-instance,
per-view previous-level state belongs to the caller. Invalid/nonfinite inputs
return errors; selection never implies residency release.

Orthographic pixel scale uses viewport height and vertical span. Perspective uses
minimum view-space depth and maximum XY radial extent, including the projection
Jacobian's off-axis depth-error term. Bounds must cover source/approximation
positions. Near-plane crossing returns None and requires exact base geometry.
The helpers assume square pixels and a symmetric vertical-FOV perspective.

`LodIndexSet` owns immutable triangle variants, derives actual index counts,
validates their shared vertex domain and retains checked logical u32 index bytes
for all levels. Supplied error metadata is validated for order/finiteness, not
proved against a simplifier. Logical bytes exclude capacity/alignment/driver data.

`SceneRenderer::upload_lod_mesh` validates source geometry, exact base indices,
vertex domain and all buffer sizes before upload. `SceneLodGeometry` shares
position/normal/material-coordinate/parameter GPU handles and owns distinct index
buffers. It retains its own metadata and `select` returns index plus borrowed
geometry for the existing SceneDraw path. Only shared references escape, so
update/clear cannot mutate shared streams. Bundle allocation_bytes counts shared
streams once plus every index buffer; summing per-level allocation_bytes would
double count shared streams. Replacement requires a new immutable bundle.

## Verified evidence and pending checks

- Six standalone policy/index/projection tests passed.
- Cargo render-library integration check passed.
- The first real-adapter test passed buffer sharing, index counts, unique bytes,
  base-topology rejection and GPU validation checks. Its binary predates readback;
  this is not a pixel acceptance result.
- Current-source explicit GPU test renders square/base versus triangle/coarse in
  8x8 offscreen targets, reads pixels and requires nonzero coarse coverage plus
  larger base coverage. Compilation's fallible mapped-range API issue was fixed;
  repeated execution passed (1 test). GPU/readback waits are bounded to ten seconds.
- Strict renderer library/tests Clippy failed on 23 diagnostics across modules.
  Two LOD texture-view default expressions were made explicit; unrelated
  diagnostics remain and the overall strict gate has not passed.
- The focused compile-fail doctest protecting immutable level access passed.
- A focused noop rejection test for foreign device/domain passed. It cannot prove
  hardware drawing. Hardware acceptance is opt-in via `--ignored`.
- Corpus audit verifies three mechanism manifests/nine pinned source files. Four
  isolated digest/URL/filename/duplicate corruptions were rejected; normal and
  python -O audits agree. Integrity does not establish review depth or parity.

Run hardware acceptance with `cargo test -p voxy_render --lib
 gpu_lod_upload_shares_streams_and_counts_all_index_buffers -- --ignored`.

Remaining: connect conservative bounds to native camera
and per-instance/view history, preserve materials/UV/picking/physics in presented
frames, integrate unique byte admission and transactional replacement in editor
residency, test multiple simultaneous variants, and implement/import meaningful
error-bounded simplification. Index streaming or separate coarse vertex domains
are required for actual pressure-driven vertex-memory reduction. None of these
is proven by the square/triangle fixture. Overall Godot/Unity/Stride parity remains
unachieved; no speedup or quality equivalence is claimed.

## Admission estimate addition

Production-only strict renderer Clippy passed; the broader test-code strict gate
still has unrelated diagnostics. lod_mesh_allocation_bytes validates source/domain
and exact base topology, counts shared streams once and all index bytes, and
checks total overflow before upload. upload_lod_mesh uses this preflight contract.
The real-GPU regression now compares the estimate to measured bundle bytes.
The focused pair of rejection/hardware tests and production Clippy have been
restarted for this code change after prior runs completed; results are pending.
Editor residency does not yet consume this estimate for LOD bundles.

## Transactional replacement addition

replace_lod_mesh stages a complete immutable replacement and assigns the target
only after upload succeeds. The rejection regression now checks that invalid
replacement preserves the previous shared vertex handle and total byte count.
Callers must admit old-plus-new peak bytes before replacement; the helper does
not implement an editor/global memory policy or catch GPU allocation failures.
The noop rejection/replacement regression passed, including retention of the old
vertex handle and total bytes. Authoring and picking identity are not altered
by this API.

Final admission/replacement focused execution completed successfully: both tests
passed, including real-GPU pixel readback and estimated-versus-actual bundle
allocation bytes. Production strict Clippy passed after the documentation fix.

Admission/replacement production strict Clippy passed after the documentation fix.
SceneCamera now exposes lod_pixel_scale from caller-provided view-space AABB and
actual viewport dimensions. It validates the existing camera, accounts for
viewport/projection aspect mismatch with the larger horizontal/vertical scale,
and delegates conservative radial/depth perspective bounds or orthographic spans.
Perspective near-plane crossing returns None. The camera/aspect/near-plane regression
and strict production Clippy passed. Per-instance history and native editor
draw/residency integration remain unimplemented.

SceneCamera::lod_bounds transforms eight object AABB corners into view space using
f64, rounds the resulting f32 extrema outward, and bounds affine error
amplification by sqrt(matrix one-norm * infinity-norm). Rotation, nonuniform scale
and shear are included; projective model transforms and nonfinite domains are
rejected. The returned error_scale is an object-to-view amplification bound for
LodPolicy selection. Caller bounds must enclose source/approximation geometry.
Build check and the shear/domain regression passed; native editor and
per-instance history integration remain open.

SceneLodGeometry::select_for_camera connects transformed bounds, viewport and
policy to borrowed GPU geometry. Near-plane fallback selects base geometry while
still validating policy, error scale and previous-level state. History must be
reset when a bundle is replaced. Strict production Clippy passed. The focused
workspace test was blocked by concurrent physics code referring to an absent
Error::WorkBudgetExceeded; a public-API acceptance project without renderer
dev-dependencies passed the camera-selection regression.


## Per-instance/view history

SceneLodHistory owns a previous selection and a weak bundle identity. Successful
selection publishes both; invalid input leaves them unchanged. Selecting from a
replacement bundle discards previous hysteresis even if the old bundle still
exists, preventing an invalid previous index after a level-count change. Dropping
a bundle makes previous() return None, and history does not retain GPU resources.
Explicit reset supports view cuts or reassignment. Callers own one history per
instance/view; no global map or authoring component is introduced.

The public-API acceptance tests passed camera fallback, invalid history/scale,
replacement with fewer levels, error retention, dropped-bundle observation and
reset. The current two-test acceptance run additionally passed independent
histories on the same bundle around a hysteresis threshold. Strict renderer
production Clippy also passed after the concurrent reflection edit settled.
The acceptance project compiles the actual public renderer API without workspace
dev-dependencies; it uses the compatible dependency versions resolved offline
by its own lockfile, and does not substitute for full workspace test acceptance.
Native asset/importer variant data and editor
selection/residency integration are still absent; this history API alone does not
establish presented-frame parity.


## Pinned importer and simplifier metric review

ImporterMesh::generate_lods (importer_mesh.cpp:570-844) handles indexed triangle
surfaces. At 658-682 it merges coincident vertices only after UV/UV2 proximity,
tangent handedness, normal-angle and color comparisons. Simplification attributes
at 725-752 are normal xyz and color rgb, not an explicit UV error metric. Indices
are remapped through vertex_inverse_remap before publication (831-842), so the
render variants reference the original streams.

The loop at 755-824 targets approximately half the current triangle count, keeps
at least 12 target indices, locks borders, initially permits disconnected-component
pruning and uses regularization for deformable meshes. A zero result with pruning
is retried without pruning; insufficient reduction or excessive current_error stops
the chain. The stopping rules and deformation flag do not prove animated quality
or absence of UV/material artifacts.

Module registration (register_types.cpp:44-46) binds the function pointers directly
to meshopt_simplifyWithAttributes and meshopt_simplifyScale. The pinned library is
meshoptimizer 1.2. Its header (518-537, 605-611) describes attribute-weighted error
and conversion between relative and absolute units using mesh extent. The
simplifier evaluates normalized quadrics (simplifier.cpp:776-783), combines geometry
and attribute contributions (1384-1428), retains maximum accepted collapse metric
(1622), and reports sqrt(result_error) * error_scale (2621-2623). These inspected
operations do not constitute a surface Hausdorff-error certificate.

Godot additionally sets current_error = max(current_error * 1.5, step_error)
(importer_mesh.cpp:815), described there as an arbitrary growth factor, and stores
max(current_error * scale, CMP_EPSILON2) as lod.distance (839). This is monotonic
selection metadata; it is not an independently checked bound against the original
surface. Even if step errors were geometric bounds, max(previous*1.5, step) would
not generally imply the sum bound for successive approximations. Voxy must keep
optimizer cost, switching metadata and certified geometric error distinct. Reusing
Godot's factor as a strict target_pixels guarantee would be unsupported.

## Reproducible CPU probe

Run `python3 docs/engine-research/mechanisms/godot-lod/run_importer_probe.py`.
It copies pinned sources into a temporary directory, compiles them with clang++
C++17/O2, and exercises the nondeforming attribute simplifier separately from Voxy.
All source/license notices remain in the saved files. No upstream C++ runtime code
is linked into a Voxy crate. This requires a local clang++ installation.

The fixture is a 17x17 analytic hill with analytic normals and constant color,
already sharing vertices. It reproduces the importer options, reduction/error loop
and border lock, but not Godot vertex merging, skinning, import dispatch or rendering.
It asserts finite nonnegative metrics, triangle counts, in-domain output indices,
all boundary vertices retained, unchanged source position/attribute arrays and more
than one published level. Assertions throw and remain active under optimization.

The checked-in importer-probe-2026-10-02.csv was produced with Apple clang 21.0.0,
arm64-apple-darwin25.6.0. Three levels reduced 1536 indices to 768, 384 and 192
(512 to 256, 128 and 64 triangles). The mesh extent scale is one, so relative and
object metrics coincide for this fixture. No wall-clock performance, Hausdorff
bound, image/UV equivalence, or native-editor acceptance is claimed.

The corpus integrity audit passed with three manifests and fifteen pinned files.
The 500 accepted identities still have root/README classification depth; these
mechanism reviews do not turn that corpus into 500 completed architecture audits.


## Voxy geometric witness verifier

certify_lod_error accepts two triangle surfaces in independent vertex domains,
plus one witness per source triangle in each direction. Each witness identifies
one target triangle and three barycentric points, using nonnegative integer
weights summing to 2^24. Invalid topology indices, nonfinite positions, incomplete
coverage, invalid weights and overflowing target indices are rejected.

For a source triangle with corners p_i and witnessed target points q_i, every
source point p = sum(lambda_i*p_i) has a target point q = sum(lambda_i*q_i) in
the same target triangle. The convex combination gives
norm(p-q) <= sum(lambda_i*norm(p_i-q_i)) <= max_i(norm(p_i-q_i)).
Thus the corner displacement bound covers the whole triangle, including its
interior. Requiring the reverse direction bounds both surfaces, including removed
islands and added geometry. Degenerate triangles remain valid convex sets.
A witness need not be the closest correspondence; poor witnesses give a looser
bound rather than an unsupported smaller one.

f32 positions times 24-bit dyadic weights are exactly representable in f64.
Subsequent coordinate sums/subtractions, squared norm sums and square roots are
rounded outward; singleton identities preserve exact zero. Input f32 ranges keep
nonzero witness distances within normal f64 range. The verifier runs in linear
surface/witness work and trusts neither a declared optimizer cost nor a producer's
closest-point calculation. It does not establish winding, topology equivalence,
UV/normal/color/material or deformation quality.

Five standalone Rust tests passed: identical/translated surfaces, invalid and
missing witness rejection, reverse coverage of a removed island, fractional and
extreme coordinates, and a square-to-triangle surface reduction with a bound
covering its missing corner/interior. Strict renderer library Clippy passed.
`python3 tools/test_lod_certificate_numeric.py` passed 256 deterministic cases
(seed 20261002), including signed zeros, subnormals, maximum finite f32 values and
mixed signs/exponents. Python Fraction independently checked the exact squared
witness displacement against the returned f64 bound, with zero underestimates.
This sampled arithmetic check is not an exhaustive formal numerical proof.

Remaining: produce useful witnesses from actual simplification, bind verified
errors to immutable variant indices/position identity, preserve material seams,
and integrate importer artifacts, editor draw selection and residency accounting.
The public verifier alone does not make imported assets or presented frames LOD-ready.


## Binding verified error to immutable geometry

CertifiedLodIndexSet owns exact f32 position bits and immutable base/variant indices.
CertifiedLodVariant supplies indices plus both witness directions; the constructor
accepts no declared error value. Each variant is verified directly against the
original surface, and its published error is the monotonic maximum of verified
bounds so far. That envelope remains conservative without treating heuristic
inflation or previous-variant metrics as a proof. Empty/invalid base data, nonfinite
coordinates, invalid witnesses and increasing index counts are rejected. Temporary
witnesses are dropped after verification; positions and indices remain owned.

SceneRenderer::upload_certified_lod_mesh checks position bits and existing base
index/domain/device validation before buffer creation. The immutable GPU bundle
reports has_certified_geometric_errors only through this validated path; ordinary
upload reports false. This flag covers object-space geometry only, not material,
animation or all projected-camera arithmetic. Certified replacement stages a full
new bundle and publishes it only on success. certified_lod_mesh_allocation_bytes
validates source identity before using the same unique-stream/all-index byte
accounting; old-plus-new peak admission is still caller-owned.

Thirteen standalone policy/certificate/artifact tests passed, including computed
square-to-triangle bounds, changed coordinates and signed-zero source identity,
shortened coordinate domains, missing reverse witnesses and invalid base data.
The focused package run passed all three artifact/source/last-good tests,
including estimated-versus-actual bytes and rejection during admission.
Production strict Clippy passed after that addition. These noop checks
do not replace real-GPU readback or native importer/editor acceptance.

Remaining: generate witnesses from actual importer/simplifier output, serialize
position/variant artifacts with bounded import work, then connect certified bundles
to editor asset residency and per-instance/view selection history. No imported
asset or presented frame is made LOD-ready by the constructor alone.


## Bounded reference witness producer and OBJ path

generate_lod_witnesses validates both surfaces and admits exactly
2 * source_triangle_count * target_triangle_count candidate pairs before output
allocation. Checked overflow or a smaller caller budget returns WorkBudgetExceeded.
The bounded reference search tries each target triangle, projects each source
corner into its interior/edges, quantizes nonnegative barycentric coefficients to
2^24, and selects the triangle with the smallest maximum proposed displacement.
Thin/degenerate cases use edge proposals when the plane solve is unsuitable.
The proposals are not trusted distance certificates: the independent symmetric
verifier derives the final bound from the quantized witnesses.

Sixteen standalone tests passed across policy, certificate, artifact and producer,
including the exact pair-budget boundary, identity, degenerate targets, removed
islands, thin/extreme coordinates, and producer-to-immutable-artifact integration.
The existing 256-case exact-rational certificate arithmetic check still passed.
Strict renderer library and certify_obj_lod example Clippy passed.

Run the actual OBJ parsing/remapping/certification path with:

```sh
cargo run -p voxy_render --example certify_obj_lod -- crates/voxy_render/examples/assets/quad.obj crates/voxy_render/examples/assets/quad-coarse.obj 4
```

The example bounds input reads using ObjLimits, requires candidate position/UV/color/
normal bits to match original rendered vertices, remaps indices to the original
vertex domain, generates witnesses, and independently constructs a certified
artifact. The run passed: 2 triangles to 1, bound 0.7071067811865479, 36 logical
index bytes. A pair budget of 3 and a candidate UV mismatch both rejected with
exit status 1. Exact matching preserves input vertex identity, not interpolated
UV/normal/image equivalence across changed triangles. This is an offline path;
it does not import a LOD chain into the native editor or generate the coarse mesh.

## Measured reference-producer limitation

```sh
cargo run -p voxy_render --example certify_obj_lod -- docs/engine-research/mechanisms/godot-lod/flat-grid-base.obj docs/engine-research/mechanisms/godot-lod/flat-grid-coarse.obj 512
```

The checked-in planar unit-square grids have 32 and 8 triangles and identical
surfaces. The whole-source-triangle/single-target-triangle witness constraint
reported 0.2500000000000001 despite exact geometric surface distance zero.
reference-witness-probe-2026-10-02.json records this conservative but loose result.
A coarse triangle covers multiple fine triangles, so its reverse witness cannot
use all those target triangles at once. This limitation must not be presented as
a useful high-quality simplification bound or a scalable final importer.

Next: support verifiable subdivisions covering each source triangle so separate
cells can map to different target triangles, then accelerate candidate lookup
with a spatial index while retaining the same independent coverage/error checks.
Real simplification output, import caching/serialization, material-boundary policy,
editor draw/residency integration and presented-frame proof remain required.


## Verified subdivision coverage

LodSubdivisionWitness stores a depth and exactly 4^depth canonical cells for each
original source triangle. The verifier independently reconstructs the dyadic
midpoint subdivision; caller-supplied arbitrary cell shapes are not accepted.
Every original triangle and every cell must have a target witness in both surface
directions. Missing/extra cells, missing triangles and depth greater than 12 reject.
The depth limit keeps all source barycentric coefficients exact within the 2^24
integer format. Cells partition the original triangle in exact barycentric space.

Evaluation uses f32 proxy corners for compatibility with the original witness
verifier. barycentric_proxy bounds their distance from exact source-cell corners
with outward-rounded arithmetic. The maximum corner radius bounds the entire
real cell against its proxy triangle by convex interpolation. The verifier adds
that radius outward to the proxy-to-target triangle bound. Coverage therefore does
not acquire gaps when proxy coordinates round or collapse. Geometry/material,
image, winding and deformation claims remain separate.

generate_subdivided_lod_witnesses is a bounded uniform reference producer; its
work admission includes 2*N*M*4^depth candidate pairs before allocation. It does
not yet implement adaptive per-cell refinement or accelerated target lookup.
CertifiedLodIndexSet::new_subdivided verifies coverage directly against the original
surface and binds derived errors to exact owned positions/indices. Its output uses
the existing certified GPU admission/upload/replacement path.

Run the refined OBJ path with:

```sh
cargo run -p voxy_render --example certify_obj_lod -- docs/engine-research/mechanisms/godot-lod/flat-grid-base.obj docs/engine-research/mechanisms/godot-lod/flat-grid-coarse.obj 2048 1
```

The actual run passed: 32 to 8 triangles, 480 logical index bytes, conservative
bound 5.622719070614828e-16 instead of the whole-triangle bound 0.2500000000000001.
These planar surfaces coincide exactly; the remaining tiny value is conservative
arithmetic padding, not measured geometry distortion. A budget of 2047 and depth
13 both rejected with exit status 1. The saved subdivided-witness-probe JSON records
the fixture/result; this is not a renderer benchmark or native presented-frame test.

Twenty standalone tests passed across policy, certificates, producers and immutable
artifacts. New checks cover canonical identity subdivision, incomplete/extra cells,
depth limits, the planar retriangulation improvement, constructor integration and
retention of a removed-island error under subdivision. The 256-case Python Fraction
oracle now independently checks both exact witness displacement and f32 proxy
quantization radius; no underestimates occurred. This is sampled arithmetic evidence,
not an exhaustive formal proof. Final renderer/example strict Clippy passed.

Remaining: choose refinement depth from requested error/budget, implement spatial
candidate acceleration, process actual simplifier output and preserve material
boundaries, cache/serialize certified artifacts, then integrate native editor
residency/selection and verify presented frames. Uniform refinement does not by
itself complete the import pipeline or engine parity.


## Indexed witness search

generate_indexed_lod_witnesses builds temporary balanced median-split triangle
BVHs for both input surfaces. Nodes contain conservative boxes of their original
vertex coordinates. Search compares a downward-rounded maximum corner-to-box
squared-distance lower bound with an outward-rounded, independently verified
candidate upper bound. Nearer nodes are visited first; pruned nodes cannot improve
that bound. Leaf proposals still use quantized barycentric points, and final
symmetric subdivision coverage verification remains separate from search.

LodSearchBudget independently limits indexed triangle cardinality, output cell
cardinality, actual candidate triangle tests and query node visits across both
directions. Cardinality admission precedes index/output construction; query limits
are charged as work occurs. Index construction is bounded by admitted cardinality,
not included in the query node counter. Budgets do not claim exact resident bytes
or elapsed-time limits. Failure returns no partial output/artifact. No persistent
index or process-wide mutable history is introduced.

Run the indexed OBJ path with:

```sh
cargo run -p voxy_render --example certify_obj_lod -- docs/engine-research/mechanisms/godot-lod/flat-grid-base.obj docs/engine-research/mechanisms/godot-lod/flat-grid-coarse.obj 640 1 indexed
```

The current run passed with 40 indexed triangles, 160 output cells, 640 triangle
tests and 608 query node visits, compared with 2048 all-pairs reference candidates.
The verified error remained 5.622719070614828e-16 with 32-to-8 triangle reduction.
The example sets separate cardinality caps (2,000,000 index triangles and 1,000,000
cells) and a query node cap of 16 times its triangle-test argument. Indexed mode's
argument counts actual triangle tests; reference mode admits total candidate pairs.
Budget 640 succeeded and 639 rejected. These are work-count results on this fixture,
not a measured wall-clock speedup or a universal search-complexity claim.

Twenty-two standalone tests passed. New tests cover reduced candidate count,
all four admission/query limits, exact versus one-less measured work limits,
full-result publication on success and retention of removed-island error.
The 256-case rational oracle now additionally verifies the search-box lower bound
never exceeds exact corner-to-box squared distance. No arithmetic violations were
observed. This sampled check is not exhaustive formal numerical verification.
Strict renderer library/example Clippy passed after a documentation-only fix in a
concurrently edited GGX light export. The existing certificate and proxy checks
remain enabled in the same independent oracle.

Remaining: process real simplifier output rather than hand-authored/grid candidates,
choose subdivision refinement from desired error and work budgets, cache/serialize
certified immutable artifacts and preserve material boundaries, then integrate
native editor draw/residency/selection and prove presented-frame behavior. Temporary
BVH search alone does not complete LOD or broad engine-feature parity.


## Actual pinned simplifier output through Voxy certification

The research probe now optionally exports base.obj and every generated LOD after
its domain/border/source-stream checks. OBJ float output uses max_digits10 so
positions, UVs and retained normal values round-trip to their original f32 bits.
run_importer_probe.py accepts --export-dir and rejects existing exported names;
the C++ exporter independently refuses to overwrite each mesh. This adds a
research bridge, not an upstream C++ runtime dependency to a Voxy crate.

The reproducible end-to-end runner is:

```sh
cargo build -p voxy_render --example certify_obj_lod
python3 tools/test_meshopt_lod_certification.py
```

It verifies every pinned source digest, builds/runs the separate upstream producer,
imports each resulting OBJ through the supplied actual certificate executable,
remaps exact vertex attributes, generates indexed subdivision witnesses and
constructs immutable certified artifacts. Every level is checked against the
original 512-triangle surface, not the preceding approximation. The report records
the executable SHA-256 and generated OBJ digests; it identifies the binary rather
than inferring its source revision. Rebuild the example when source changes.

All six level/depth runs passed. For this analytic hill fixture:

| Variant triangles | Upstream object metric | Verified bound, depth 1 | Verified bound, depth 3 |
| --- | --- | --- | --- |
| 256 | 0.00512914546 | 0.047309662758212655 | 0.011827417662687037 |
| 128 | 0.0128934653 | 0.07107694608035177 | 0.024069325320259214 |
| 64 | 0.288550735 | 0.22569487582418177 | 0.12489521205556399 |

The certified bounds are conservative upper bounds, not measured exact Hausdorff
distances. A bound larger than the upstream metric does not prove that the metric
underestimated actual surface deviation. The two quantities have different
contracts and cannot be substituted for each other. Higher subdivision tightened
the verified bounds on this fixture, with more work; that is not a universal
monotonic/performance claim. The coarsest variant retains its larger verified
error and must be selected using that value at the relevant view/scale.

The runner additionally checked imported base/variant index counts, complete cell
cardinality, finite bounds and work limits. Repeating export into the same output
directory rejected without modifying any OBJ digest. The corpus audit still passed
1428 pinned records and three mechanism manifests/fifteen source files; the 500
accepted identities remain root/README classification depth. Saved results are
meshopt-certification-2026-10-02.json.

This fixture does not cover arbitrary model imports, Godot vertex merging/skinning,
material seam policy, deformation, native rendering or image equivalence. The
simplifier remains a pinned research executable; the example consumes its output,
not an automatic shipped import service. Remaining: choose/import the simplifier
backend, bounded refinement policy, certified artifact cache/serialization and
native editor residency/draw integration with presented-frame acceptance.


## Versioned certificate artifact format

encode_lod_archive/decode_lod_archive add the VOXYLCD1 little-endian binary format:
exact f32 position bits, base/variant u32 indices, and complete subdivision witness
records (depth, cell count, target triangle and nine integer weights). No optimizer
metric or trusted error scalar is stored. Encoding produces a bounded certificate
proposal; decoding re-verifies complete geometry coverage and derives errors before
publishing CertifiedLodIndexSet. Semantic corruption can therefore reject even when
the byte stream is structurally valid.

LodArchiveLimits independently caps total byte length, positions, levels including
base, cumulative indices and cumulative cells across every level/both directions.
The decoder checks payload lengths before collection/allocation, rejects unsupported
magic/version, missing/extra cells and trailing bytes, and has checked count/offset
arithmetic. These are cardinality/input-byte limits, not exact process RSS accounting.
The codec performs no filesystem publication or global cache mutation.

Twenty-six standalone tests passed, including byte-by-byte truncation rejection,
trailing data/version/forged count rejection, nonfinite coordinate and weight
corruption, exact versus one-less limits, cumulative cross-direction cell/index
admission, round-trip source bits, derived errors and immutable indices. Strict
renderer library and example Clippy passed; a borrowed-slice cleanup resolved the
example's needless_pass_by_value diagnostic.

The indexed certify_obj_lod path now encodes its generated proposal and returns the
re-verified decoded artifact. The actual pinned meshoptimizer runner passed all six
level/depth combinations through this path. Verified bounds stayed unchanged. On
the hill fixture, archive sizes were 139436/1982636 bytes for 256 triangles at
refinement depths 1/3, 116780/1652780 for 128 triangles and 105452/1487852 for 64.
These include CPU proof/geometry data and are not GPU residency bytes. Results and
executable identity are in meshopt-archive-certification-2026-10-02.json.

Use the existing ImportInputs::build_key for disk-cache lookup: it already includes
observed source digests, importer identity/version, target and canonical option
bytes. Include simplifier/verifier/schema versions and all refinement/material
options in those identities/options. The existing method explicitly identifies
inputs rather than guaranteeing deterministic execution or an output cache; do not
replace it with a parallel key scheme or treat it as a completed cache service.

Remaining: atomic bounded disk storage/retrieval keyed by the existing build identity,
source/parameter invalidation and last-good publication through the asset pipeline,
a shipped simplifier/refinement backend, and native editor residency/draw/quality
acceptance. This artifact codec and in-memory round trip do not constitute that
completed disk cache or native LOD integration.

## Bounded disk artifact storage foundation

voxy_assets::ArtifactCache now stores generic artifact bytes under the existing
32-byte import build key. VOXYCA01 envelopes bind the key, payload BLAKE3 digest
and format version. Reads enforce the configured payload cap using a bounded
reader plus one sentinel byte, reject non-file entries, wrong keys, bad versions
and damaged content. Format-specific decoding and certificate verification remain
the importer's responsibility; envelope integrity does not certify geometry.

Publication writes a unique create_new temporary file in the cache directory,
syncs it and renames it to the key path. Failed publication cleans the temporary
file and does not delete the previous artifact. Replacement follows the host
filesystem's rename semantics; unsupported replacement returns an IO error.
Directory crash durability, hostile concurrent directory swaps, global eviction
and aggregate disk/RSS accounting are outside this API's contract.

Four actual-disk tests passed: round trip and replacement, oversize write preserving
the previous entry, corruption/foreign-key rejection, source/options build-key
invalidation retaining old entries, failed rename scratch cleanup and oversize read
rejection. Strict voxy_assets library Clippy passed with warnings denied.

This is the storage foundation, not an integrated LOD import cache. Remaining:
capture OBJ source snapshots through ImportInputs, load and re-verify an archive on
a matching build key, build/store on a miss, then connect last-good asset publication
and native editor residency/draw acceptance. The shipped simplifier/refinement
backend also remains open.

## OBJ certificate cache integration

certify_obj_lod now accepts an optional cache_dir after indexed mode. Both OBJ
files are captured as bounded immutable ImportInputs snapshots; build_key includes
their identities/digests, parser/search/verifier/archive version identity, portable
target and canonical budget/depth/search options. Matching entries are decoded and
certified again, then checked against exact current imported position bits and both
index arrays before use. Misses generate, encode, re-verify and publish. Hits skip
witness search, but still parse inputs and verify geometry. Reference mode rejects
cache_dir explicitly because its direct witness representation is not archived.

tools/test_lod_disk_cache.py exercises the actual built executable and filesystem:
miss/hit identical output, no search counters on hits, depth and source-byte
invalidation, retained older entries, corrupt envelope rejection without rewriting,
and no pending files. Saved evidence identifies the executable SHA256 in
disk-cache-certification-2026-10-02.json. These are a flat-grid fixture and an offline
example; native editor import publication/residency/frame acceptance remain open.
Corrupt cache entries currently fail the attempt explicitly instead of automatic
rebuild/eviction. Import version identity must be bumped when semantics change.

## Editor certificate import boundary

The native editor importer accepts an explicit .vmodel JSON recipe:
{"version":1,"base":"base.obj","certificate":"proof.lod"}. Unknown fields,
unsupported versions, non-OBJ bases and recursive recipes are rejected. Relative
paths use the existing project-scoped FileInputs and SourcePath checks. Recipe,
base and certificate all enter the existing ImportInputs dependency observations.
The recipe has a 4096-byte interpretation cap; provider reads remain bounded by
the existing 16 MiB per-file and import-attempt total limits.

The raw VOXYLCD1 archive is decoded with byte/position/level/cumulative-index/cell
caps and re-verified. The renderer's CPU allocation preflight then checks its exact
base position bits/index identity against the imported OBJ before an immutable
Arc<CertifiedLodIndexSet> is attached to EditorAsset. Publication retains the
existing source revalidation and catalog ticket boundary; these changes do not
publish partial certificate results or change the catalog's replacement policy.

This is CPU import integration. ModelGraphics currently draws the base mesh; it
does not yet upload the attached LOD bundle or select a level from the view. The
recipe consumes a raw certificate archive, not the VOXYCA01 cache envelope. A
production archive export/import workflow, GPU bundle residency, view selection
and actual presented-frame acceptance remain required.

The focused actual-file editor test passed: three observed dependencies, decoded
two-level certificate, changed proof rejecting final source revalidation, damaged
proof rejecting import, modified base coordinates rejecting identity binding and
unsupported recipe version rejection. Editor library check, strict library Clippy,
focused formatting and whitespace checks passed. This does not prove last-good GPU
replacement or native LOD selection, which are still outside the implemented path.

## Native editor LOD publication and camera selection

ModelGraphics now owns a ModelGeometry enum: an ordinary single mesh or one
certified SceneLodGeometry bundle. LOD has no duplicated standalone base buffer.
Its unique vertex/material streams and every level's indices enter geometry
admission/accounting; selection outlines retain their separate base geometry.
Staged upload keeps the existing old-plus-new peak admission policy and publishes
only after success. Failed admission retains the previous GPU model.

The editor camera exposes the same SceneCamera used to form its view projection.
Each drawable's model AABB is transformed conservatively for LOD selection;
per-instance/view weak bundle history provides 15 percent hysteresis at the initial
one-pixel geometric-error target. Legacy identity projection or invalid selection
uses base geometry. Picking and collision stay tied to authoring geometry. Selected
draw references change without changing resource residency or scene components.

tools/test_native_lod_editor.py builds a temporary hill OBJ fixture with the pinned
upstream research simplifier, runs the real certificate executable, extracts the
raw archive from its versioned cache envelope and opens the actual native editor.
On the 512/256-triangle fixture, presented frame 3 selected coarse level 1, frame 6
selected base level 0, and frame 9 was presented after a deliberate zero-budget
upload rejection while preserving the previous geometry. Resident/preflight bytes
agreed at 34076 including outline. Native log and binary/archive identities are
saved in native-editor-certification-2026-10-02.json. This proves presented draw
selection and admission retention, not pixel-image or material equivalence.

All 22 editor library tests passed. The extended certificate import test also
passed an actual asynchronous App reload: corrupted proof retained the same CPU
publication Arc, and restoring proof published a fresh certified revision.
Strict editor library/example Clippy passed; a separate renderer dependency's
unused import warning was observed during the build. The archive fixture remains
a research/export harness: an interactive simplifier/import settings UI, attribute
quality acceptance, multi-view selection ownership and streamed level eviction
remain open. Architecture ownership contracts are recorded in ADR 0002.

## Native acceptance strengthened on 2026-10-02

Budget rejection now exercises App::synchronize_gpu_residency with the published
model still resident. It asserts that the ordinary deferred publication path
records a geometry-budget error and retains the exact base geometry. Restoring
the budget must clear that deferral before the final presented-frame check.
The rebuilt native executable passed far level 1, near level 0, rejection at
34076 live bytes with a zero budget, and recovery by presented frame 9. The fresh
binary/archive hashes and log replace the native acceptance evidence above.
Editor library tests passed 22/22; renderer library tests passed 103/103 with
the real-GPU acceptance test separately opt-in.

The separately enabled real-GPU LOD test also passed: base/coarse index buffers
share vertex/material handles, allocation totals match actual buffer sizes, and
rendered pixel counts differ as expected between full square and coarse triangle.
A dependency-inclusive strict Clippy run was blocked by 669 existing diagnostics
in the unrelated physics crate; the editor-only gate is recorded separately.

Strict editor library and lod_viewport example Clippy with --no-deps passed.
Focused rustfmt and whitespace checks also passed.

## Native visual review — 2026-10-02

Inspected the native editor with a static hill fixture: 512 triangles near the camera (distance 5), 256 triangles far away (distance 32). Both surfaces remain visible, with no visible holes. Corrupting the raw certificate shows the previous model and an import failure title; restoring it recovers the loaded title and geometry. Screenshots, hashes and native log are recorded in `native-visual-review-2026-10-02.json`. This validates this fixture, not arbitrary material or animated-mesh equivalence.

The review exposed a scene authoring defect: standalone cameras/lights required a model. Optional model ownership now survives save/load; unknown model references still reject transactionally. Editor library tests: 24 passed; strict library/example Clippy passed.

## Native snapshot migration validation — 2026-10-02

Fresh editor/verifier builds passed `tools/test_native_lod_editor.py`. Presented frames 3/6/9 validate snapshot camera distance and extracted owner/material membership, far 256/near 512 triangle selection, 34076 resident bytes, zero-budget rejection preserving previous geometry, and recovery after restoring budget. Binary/archive hashes and native output are saved in `native-snapshot-certification-2026-10-02.json`. This is native frame integration evidence; no pixel comparison or new manual visual inspection is claimed.
