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

## Ordered compute-pass timing, 2026-10-10

`ComputeJob::encode_repeated_steps_with_timestamps` now exposes pass-boundary
queries for ordered resident dispatches sharing one compute pass. It uses the same
encoder as ordinary repeated dispatches. Both single and repeated timestamp APIs
reject disabled `TIMESTAMP_QUERY` before recording a pass; query ownership, type
and index validity remain the caller's wgpu contract.

Actual Metal validation compares all QR factor bytes for nine varying operators
with and without instrumentation, decodes every factor, resolves positive GPU
intervals and records the device timestamp period. The disabled-feature unit test
verifies that ordinary encoding remains usable after rejection. The renderer unit
suite passes 212 tests (34 ignored). Evidence and full source/binary hashes are in
`artifacts/articulated-jump-2026-10-10/repeated-qr-gpu-pass-timestamps/`.

These intervals measure selected QR passes under uncontrolled concurrent load.
They do not measure equality solves, CPU assembly, whole-trajectory throughput,
full-frame GPU time or rendered FPS. The full-density trajectory qualification is separate from these pass timings;
its terminal and subsequent diagnostic results are recorded below.


## Original physical-operator experiments, 2026-10-10

All results below use actual Apple M4 Max/Metal execution. The principal captured
operator has 174 original rows, 14 systems and 1764 coordinates, with original
physical tolerance 1e-14. The timestamp sample selects its 39-row active equality;
it is not a timestamp of the complete 174-row projection. Experiments retain
physical admission and report zero native fallback for this operator. Tests and
shader fixtures remain opt-in; the production shader and allocation defaults
are unchanged. Concurrent load was uncontrolled in every comparison.

| Experiment | Observed result | Decision |
| --- | --- | --- |
| Workgroup current-vector cache | Nine paired QR intervals; median 6.276542 ms original vs 16.337750 ms candidate, zero faster pairs; full storage bits preserved | Reject |
| Per-invocation current-vector cache | Median 5.570083 ms original vs 6.331959 ms candidate, zero faster pairs; full storage bits preserved | Reject |
| Resident ordered QR columns, single dispatch | Nine paired QR intervals, 39 original dispatches vs one candidate dispatch; median 9.673875 ms vs 8.570459 ms, nine faster pairs; full storage bits preserved | Microbenchmark result only |
| Same resident QR in complete physical projection | Seven warmed alternating pairs; median 0.813243375 s original vs 0.813444792 s candidate; four faster pairs, physical response/reaction bits preserved | Do not adopt: no measured net benefit |
| Geometric workspace capacity | Cold allocations 63 to 8; warmed allocations 31 to zero per projection. Seven warmed alternating pairs: median 0.509193917 s vs 0.495104458 s, candidate faster in only two pairs; physical response/reaction bits preserved | Keep experimental; resource reuse demonstrated, timing inconclusive |

Do not compare absolute times across rows: these are separate loaded experiments.
The geometric comparison uses separate warmed pools, so a candidate enlarged
buffer cannot improve the baseline pool. Its strict-budget regression verifies
exact-size retry after an enlarged reservation fails, oversized rejection without
losing cached storage, bounded logical-payload readback and explicit retirement.
The cache limits remain 32 completed jobs and 8 MiB. The enlarged capacity never
changes the shader's logical operator or the physical acceptance tolerance.

Host attribution of the first complete projection's equality callbacks records
66 equality batches, 119 equality-task submissions and 31079740 staging payload
bytes. In the baseline cold diagnostic invocation, device.poll waiting accounts
for 496.603509 ms of 586.908043 ms attributed callback stages (84.61%). This waiting
includes kernel execution, queue contention and completion/readback; it does not
separate their costs. Callback stage totals exclude active-set work between
callbacks and cannot be presented as whole-projection or frame timing. The later
cache-disabled replay in the fixture is excluded from this attribution.

Evidence directories under `artifacts/articulated-jump-2026-10-10/`:
`qr-workgroup-vector-experiment/`, `qr-lane-vector-experiment/`,
`qr-resident-columns-experiment/`, `qr-resident-one-dispatch/`,
`qr-resident-full-projection/`, `geometric-equality-workspaces/`, and
`qr-resident-stage-profile/`. Each retains raw output and measured summaries;
source/binary hashes accompany the experiments. These are bounded operator
experiments, not full-model, CUDA, non-Metal, full-trajectory or FPS qualification.

## Full trajectory qualification is still failing

The contact-owner isolation candidate's full 469-guide x 20-segment run requested
720 frames with unchanged position/quaternion gates of 1e-6 m and 5e-5. It exited
101 at frame 3: maximum position difference 0.00022248771285753033 m, quaternion
component difference 0.006089104733330297, worst position at guide 232 point 20.
Frames 1 and 2 passed; no native fallback occurred. See
`contact-owner-isolation-full-capture/terminal-summary.json` and the complete log.
This candidate is not trajectory-qualified despite passing isolated operators.

A subsequent frozen diagnostic run remains separate in
`frame-three-rod-232-full-capture/`. Its phase/projection tracing selects guide
232 at frame 3. Its operator capture inadvertently retained exporter defaults
(guide 53, 16 operators per backend); those operators must not be described as
complete guide-232 data. Sixteen paired guide-53 islands pass raw row/load audits
and retain rod/entry-owner order and stored shapes, but their numeric inputs differ.
They contain no guide 232 and do not establish the cause of its divergence.

An opt-in in-process failing-frame replay is now compiled in the qualification
fixture. It retains complete mutable hair state, collider motion/caches and
root targets, restores failed end states afterwards, and still fails the original
gates. A bounded CPU checkpoint regression restores the complete mutable-state
fingerprint and repeats the next step identically; actual full-model GPU failure
replay remains unexecuted. Evidence: `inprocess-divergence-replay/`.

Host active-set selection and original f64 residual/backward-error checks remain
in the physics/app pipeline. Moving them to GPU requires preserving original
physical validation, failure semantics and precision; removing waits by skipping
checks is not an acceptable optimization. Rendered >160 FPS, full 720-frame
agreement, production walking/running integration, and hardware-wide support
remain unproven and are not implied by any measurement above.

Native interactive jump admission was investigated separately in
`artifacts/articulated-jump-2026-10-10/runtime-strain-refinement/`. The original
launch stopped after publishing 0.05 simulated seconds with guide 53 exceeding
the unchanged 5% axial-strain limit. Bounded retries from the original admitted
state helped but were insufficient alone. The application now samples its
animated body collider over the same substep interval as the scalp roots.
With that policy, the native-physics interactive run published 2.30 simulated
seconds and exited 0 without logged worker/scene errors, crossing first takeoff
(0.56), landing (1.06), and recovery (1.65). All 469 guides, 20 segments per guide,
and 2,363,760 GPU-generated follower vertices were retained. It did not complete
the six-second loop, and it does not qualify the independent GPU physical solver.

Timed collider sampling now reuses collider/vertex workspaces between substeps.
Fresh and reused paths match complete geometry, velocities, BVH and feature-normal
state exactly over 128 deforming intervals, with stable underlying buffers and
atomic rejection of degenerate interval boundaries. Current checks passed:
370 physics unit tests (39 ignored), 22 hair/rebase integration tests, and the
eight-step full-groom coupled skin/hair regression. The sampled-policy application
hair suite also passed 11 tests (2 ignored), including its legacy-asset 360-step
arm-rig loop. The interactive launch predates workspace reuse; no FPS gain is
claimed from either the stack sample or these correctness tests.

## Contact pipeline migration boundary, 2026-10-11

Current source audit: `gpu_hair_response_dispatch.rs` submits resident RHS waves, starts snapshot reads, waits on the submission, then decodes checked CPU outputs. `contact_safe_motion.rs` still builds candidate motions, filters broad-phase pairs and performs continuous trajectory admission on CPU. `voxy_cuda/src/lib.rs` exposes terrain, gravity, projectile, voxel, box sweep, water and tissue-search modules; it does not expose a hair-contact backend. Existing GPU pass intervals therefore cannot establish a GPU-resident hair pipeline or CUDA hair support.

The next migration unit must have one solver-owned lifetime for guide state, candidate envelopes, active contact rows, response factors and convergence/error status. Keep immutable old geometry distinct from speculative candidate geometry. Retain capacities outside captured execution; never record initialization that overwrites history on every replay. This ownership reference is pinned Newton SolverVBD construction/capture guidance, not adoption of its VBD constitutive law. PPF resident driver sequencing is a second scheduling reference.

Move broad-phase candidate generation first as a conservative superset with stable original segment identities; separately compare every candidate set to CPU geometry. Keep whole-path admission authoritative until the accelerated predicate has its own equivalence qualification, including degeneracy, overflow and failure propagation. A hardware backend may propose motion but cannot publish a pose before all required physical gates pass. Removing host waits requires device ownership of downstream consumers and explicit bounded status propagation; deleting a wait around the current CPU decoder would violate the data dependency.

Qualification must use the full469-guide groom and original material/contact limits over the720-frame coupled authored trajectory, then separately measure the rendered scene with moving camera, CPU/GPU frame times and tail latency. The currently running CPU trajectory is correctness work only. CUDA compilation or the existing non-hair kernels do not prove NVIDIA hair execution; Metal pass tests do not prove cross-backend support.
