# Legacy material skinning normal transport

The legacy vertex shader previously multiplied authored normals by the forward
world deformation. Nonuniform scale and shear therefore changed their direction
incorrectly. It now uses the inverse transpose of object.model times the blended
skin matrix, implemented with scaled cofactors and determinant sign. Zero authored
normals retain the posed-face shading fallback.

A real GPU probe compiles the production shader source and invokes its normal
function from a compute entry point. Twelve cases cover reflection, shear,
anisotropic scales at ordinary, 1e-10 and 1e10 magnitudes, and missing normals.
An independent double-precision matrix inverse provides the oracle. On Apple M4
Max Metal maximum direction error is 8.5600654e-8; the old forward operation has
maximum error 1.2534771 on these same fixtures. All 130 ordinary renderer tests
also pass. Logs and source hashes: artifacts/rig-legacy-normal-2026-10-03/.

The helper probe verifies the shader function; the full synthetic raster acceptance below extends this evidence.
Legacy upload, palette updates, model updates and atomic LOD pose updates now
validate complete world-space positions and inverse-transpose normals before GPU
writes. Validation reuses the mesh already owned by temporal history; no additional
resident mesh or independent pose owner is introduced. Singular blends are rejected
even when every individual bone is invertible. Rejected admission leaves history
unchanged. Affine and overflow checks share existing temporal/normal calculations.

Admission regressions plus 91 editor and 131 renderer ordinary tests and all 14
renderer GPU tests pass. Logs: artifacts/rig-legacy-admission-2026-10-03/.
The guard ordering is inspected in source; native-window rejected-update pixel
identity, arbitrary imported-rig legacy raster parity, single-sided mirrored winding and non-Metal
hardware acceptance remain unverified. Admission scans vertices on the CPU;
its native-frame performance impact has not been measured.


## Full legacy raster and shared publication

GpuSkinnedMesh.write_pose admits the complete candidate before writing either
palette or object uniform. Ordinary palette/model updates and LOD publication use
this same method and commit their CPU pose after success. Temporal history remains
the source mesh owner; no duplicate resident CPU pose store is added.

A 128x128 real Metal pass uses the actual production vertex/fragment pipeline,
material texture, depth test and indexed draw. Independently double-precision-baked
triangle positions and inverse-transpose normals produce exactly the same pixels:
1733 visible pixels, zero differing channels. The wrong forward-normal reference
changes 5199 color channels. Rejected singular-model and nonfinite-palette updates
preserve the actual rendered pixels; a following valid update changes the frame.
This is a synthetic offscreen fixture, not native-window or complete imported-rig
pixel acceptance. Early fixture failures are retained: different normal directions
can have identical quantized illumination, so the final fixture explicitly checks
a significant lighting difference before testing the wrong-normal reference.

Release CPU admission profiling on this host uses actual Fox and RiggedFigure clips,
208 measured samples each after 32 warmup samples. Fox (1728 vertices) median/p95:
37.208/42.417 microseconds; RiggedFigure (370 vertices): 11.542/11.667 microseconds.
Sampling, palette construction, GPU writes and presentation are excluded; these
numbers do not establish full frame cost or FPS. Artifacts:
artifacts/rig-legacy-raster-2026-10-03/.

Final regression: 222 ordinary tests and all 15 renderer GPU tests passed.


## Fragment normalization and flat-face orientation

Unit vertex normals do not remain unit after raster interpolation. The legacy
fragment shader now scales and normalizes the interpolated direction before
Lambert lighting. Editor Lambert already normalized its fragment normal. For
missing authored normals, legacy screen-space derivatives now use the order that
matches the CCW front face with downward framebuffer Y.

An independent CPU oracle evaluates barycentric normal interpolation, normalization,
Lambert illumination, the production 32-level quantizer and sRGB encoding for 3092
interior pixels. Metal matches exactly (maximum channel error zero); the old
unnormalized operation differs at 2950 of these pixels. Samples within 1e-4 of a
quantizer threshold are excluded to avoid ambiguous floating-point rounding.

Flat fallback pixels equal authored CCW face pixels under identity and a rotated
nonuniform model, with zero differing channels. The original reversed derivative
order differed by 11070 channels. All 222 ordinary and 15 renderer GPU tests pass.
Artifacts: artifacts/rig-fragment-normal-2026-10-03/. This does not certify extreme
triangle derivative ranges, other hardware or native-window presentation.
