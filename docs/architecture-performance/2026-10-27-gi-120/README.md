# GI and absolute street_walk 120 Hz qualification, 2026-10-27

**Not qualified. No GI candidate in this campaign is promoted to the default renderer.**
The unchanged-quality 1920×1080 full-frame p99 target is 8.33 ms. No idle window,
process suspension, power change, dropped sample, retry of a control, or load
correction is used. Offscreen GPU completion is measured, not display scanout.

## Source and shipped scope

Frozen baseline: `8acd97edce5fe2024fe173a2ba792dad27e087fe`.
`frozen-binaries.json` records SHA-256 hashes of immutable local executables.
`original-gi-queue-experiment.patch` contains the rejected experimental queue,
its native GPU correctness diagnostics, and the measurement instrumentation.
Apply it to that baseline to reproduce the experimental binary. It is **not**
a production optimization. The only retained runtime changes are successive
host observations of exact GPU submission completion and gating verbose ray
phase logging behind `VOXY_STREET_GPU_TIMING_TRACE`.

The new completion interval samples include event-loop gaps between frames;
existing completed-frame service samples include current CPU work through the
GPU fence. Neither is a measurement of presentation latency or display refresh.
Both clocks must meet the absolute p99 budget. Clock instrumentation is shared
by A and B in the final campaign.

## Rejected hypotheses

All effects, 1920×1080 internal resolution, Temporal AA, ray reach (60 m), original
80-step recurrence, eight crossing refinements, materials and scene geometry are
retained in candidate measurements. Effect-disable runs are diagnostics only.

| Candidate | Evidence | Decision |
| --- | --- | --- |
| Dedicated copy of GI march | `specialize-*` | No reproducible gain; substantial baseline drift |
| Existing transmission-depth specialization | `transmission-*` | No reproducible full-frame gain |
| Conservative eight-step world bounds | `batch-*`, `rejected-aabb-batch.patch` | Native rejection oracle passed; slower |
| Cached affine eight-step bounds | `line-*`, `batch-line.patch` | Native rejection oracle passed; slower/noisy |
| Four-step bound hierarchy | `four-*`, `batch-four.patch` | Native rejection oracle passed; slower |
| Early normal-only surface evaluation | `normal-first-*`, `before-surface.patch` | No stable gain |
| Exact R32Float depth copy | `float-depth-*`, `float-depth.patch` | Native copy exact; no full-frame gain |
| Sorting original GI rays | `original-gi-queue-experiment.patch`, `final-pairs/` | Transport exact, raster GI parity failed; not eligible |

Bounds experiments use conservative rejection only: all potential hits retain
original samples and refinements. Native oracle reports retain their tested
sample counts, accepted groups and zero false rejections. They establish only
that tested rejection domain, not an unrestricted proof. Floating-point bound
construction follows the need for explicit rounding margins described by
[pbrt, Managing Rounding Error](https://www.pbr-book.org/4ed/Shapes/Managing_Rounding_Error).
Prototype source and failed first attempts remain available in this directory.

## Correctness investigation of sorted original GI

The queue stores full Float32 original-ray inputs in 32-byte records, buckets
only scheduling keys, traces the original query, and coherently resolves by
original pixel ID. The GPU oracle recomputes in original ID order and checks
all result bits, duplicate/missing/out-of-range IDs and header count. A persistent
failure word catches errors from earlier rendered frames. It passed 191 active
frames at 2,073,600 pixels/frame in the short native campaigns. Extra oracle work
is excluded from performance campaigns.

However, equality with compute GI does **not** establish equality with raster
GI. Raw RGBA32Float readback before HDR/TAA found 43,017 different words out of
8,294,400 and maximum radiance error 956.25 in `queue-exit-oracle.json`. This
failure is retained, not hidden by half-float rounding or TAA.

`input-tuple-{1,2,3,4}.json` isolates world position, surface normal, biased
origin and direction. World and normal match bitwise; origin and direction
initially differ. Explicit FMA made origin match (`canonical-tuple-3.json`) but
direction still differed. Generating rays in the raster guide pass made the
standalone direction diagnostic match (`fragment-tuple-4.json`), yet complete
GI still has 43,048 differing words (`fragment-tuple-0.json`). This implicates
stage/compiler arithmetic behavior beyond transport; no claim of a verified
Metal assembly root cause is made. No failed candidate is enabled by default.
Diagnostic tuple runs produce diagnostic images and are never performance data.
Raw raster comparisons cover the last completed frame, whereas the queue
transport oracle covers every rendered frame of its invocation.

`line-no-gi.json` and `line-no-rays.json` are lower-work diagnostics, not acceptable
quality modes: even their observed p99 was 14.88 and 15.35 ms respectively.
This does not prove a hardware impossibility; it shows GI alone is insufficient
to explain the observed complete-frame deadline misses in this workload/load.

## Final paired campaign

`run_pairs.py` freezes executables, sanitizes renderer environment variables,
runs sequential A/A, five alternating AB/BA pairs, then A/A. Every run requests
one full 20-second deterministic route: 2,400 measured frames after 180 warmup
frames. Both arms use the same frozen executable. A selects the original inline
renderer; B explicitly selects the experimental original-GI queue and atomic
scatter. No precision oracle or tuple diagnostics run inside timed samples.

`analyze_pairs.py` checks workload, GPU timestamp availability, raw quantiles,
physics/weather/water replay identity, both A/A controls, paired bootstrap CI,
service p99, completion-interval p99 and strict raw-GI parity. Thresholds were
fixed before the final campaign: A/A service p50/p99 and completion-interval p99 ±10%, GPU mean ±5%; paired p99 CI
upper bound ≤1.05; every B service and cadence p99 ≤8.33 ms; raw GI parity must
pass. Rejected controls stay rejected. `qualification.json` is authoritative.

## Validation

`tests-final.log`: `voxy_render --lib`, 354 passed, 84 ignored, zero failed.
This does not count ignored tests as passed. Native proof reports are separate
from the unit suite. Build logs and exploratory failures are preserved. Large
local PNG/PFM captures are ignored by Git; hashes/metrics and small raw reports
are versioned. See final delivery validation logs for the retained telemetry.

The timed A executable also contains disabled experimental shader branches;
therefore it is the instrumented inline control in this frozen experiment,
not a claim of machine-code identity with the clean baseline executable.
Neither arm is used to claim a production optimization.

## Final observed result

Both A/A controls passed; all 14 runs and deterministic replay checks were valid.
Absolute 120 Hz and relative p99 nonregression both failed. Paired p99 ratio
95% bootstrap CI: `[0.968141, 1.110858]`. Strict raw-GI parity also failed.

| Pair | Before service p99 ms | Queue service p99 ms | Queue completion p99 ms | GPU mean ratio B/A |
| --- | ---: | ---: | ---: | ---: |
| 0 | 23.351 | 23.258 | 23.382 | 1.0223 |
| 1 | 16.729 | 20.175 | 20.282 | 1.1047 |
| 2 | 18.374 | 17.633 | 17.785 | 1.0268 |
| 3 | 17.507 | 16.952 | 17.154 | 1.0453 |
| 4 | 17.287 | 17.028 | 17.121 | 1.0315 |

The queue increases GPU mean time in every pair. No performance claim is
accepted. These controls bound local repeatability at the beginning/end; they
do not establish an idle GPU or remove drift between pairs. Raw measurements
and both unsuccessful and successful screening controls are retained.

Final retained-source validation: `tests-delivery-render.log` has 353 passed,
84 ignored, zero failed; `tests-delivery-app.log` has 26 passed, 2 ignored,
zero failed. `tests-qualifier.log` has 13 passed, including rejection of missing
GPU timing, nonfinite or missing completion intervals, failed GI parity,
incomplete replay, false quantiles and deadline misses. `build-delivery.log`
confirms the release example build. The earlier 354-test suite includes the
experimental queue shape test, which is retained in the rejected patch only.

`retained-image-comparison.json`: frame-120 full linear Float32 RGB PFM is
byte-identical for baseline A/A and baseline versus retained telemetry (all
three SHA-256 digests equal). This checks the retained changes only and does
not rehabilitate the failed GI candidate. Captures run after the timing matrix.
