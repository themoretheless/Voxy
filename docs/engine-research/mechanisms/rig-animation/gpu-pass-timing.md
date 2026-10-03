# GPU skin-pass timing

SceneSkinner optionally accepts compute-pass boundary timestamps through
`encode_prepared_pose_profiled`. Ordinary callers retain the existing unprofiled
entry point. TIMESTAMP_QUERY is required only for the profiled path. The caller
owns and resolves the query set; query type, indices and device must satisfy wgpu
validation. An unavailable feature is rejected before palette writes.

The upstream reference-rig regression requests exactly the available timestamp
feature, resolves two timestamps after each dispatch, reads unsigned ticks, and
multiplies their difference by the queue timestamp period. Positions and normals
are still compared with the CPU result for all 65 samples under a GPU validation
scope. Unavailable features report `supported=false` and no measurement.

On Apple M4 Max/Metal, encoder timestamps are unavailable but compute-pass
boundary timestamps are available. The 370-vertex, 22-palette-node rig measures
median 18.875 us and p95 19.416 us over 65 poses. The period is 1 ns. This is a
single compute-pass interval, including its pass boundaries, not full-frame GPU
time, FPS or a large-rig throughput claim. The debug host timings and GPU timings
are distinct; transfer/readback, CPU preflight, shading and presentation are not
included in this GPU interval. Initial samples were retained.

LOD cache publication now waits for successful camera selection and optional
geometry admission. Rejected world/camera requests preserve accepted positions,
levels and history. The GPU/CPU fallback regression compares nonuniform scaled,
rotated, translated world results with a separate certificate calculation.

Evidence is in `artifacts/rig-gpu-timestamps-2026-10-03/`. The older native-frame
profile is a separate hashed snapshot preceding this transaction-order change.
Full render-pass timestamps, complex rigs and non-Metal/CUDA verification remain
open work.
