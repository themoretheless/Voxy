# Original inline GI: rejected optimization experiments, 2026-10-27


**Outcome: 120 FPS is not qualified; no experimental GI change is promoted.**
The complete 14-run exact-cache campaign passed both A/A controls but regressed:
paired GPU ratio CI95 [1.00996, 1.05392], full-frame p99 ratio CI95
[1.00660, 1.08138]. Candidate service p99 spans 23.44–25.98 ms.
The MRT follow-up fails native image bounds and its short A/A control drifts.
The retained production change adds correctness captures and marks their timing
ineligible. All GI algorithms, quality settings and runtime resource paths stay
at the baseline implementation.

Baseline commit: `0bfc8df31`. Final decision is recorded in `qualification.json`.
The tested renderer retains original raster GI, random directions, 80-step
recurrence, 60 m reach, eight crossing refinements, radiance/history tests,
1920×1080 internal/output resolution, Temporal AA, materials and geometry.
No idle window, process suspension, load correction, reduced quality setting,
trimmed samples, compute GI or sorted ray queue is used.

## Selected query change

`exact-pixel-cache.patch` retains the last exact level-zero depth texel and
removes the additional 8×8 minimum-depth lookup, its cache state and branch.
A tile minimum is no larger than any represented valid forward-Z texel. A
sample rejected by that bound cannot pass the exact depth comparison. Omitting
the rejection bound can issue extra exact reads, but does not skip any original
sample or refinement. Depth stays Depth32Float. All ray/colour arithmetic
expressions remain unchanged. The texture is immutable during a sampling pass;
the cache is local to one query and resets between rays/frames. GPU submission
ordering and CPU/GPU completion synchronization are unchanged.

The shared original march also serves SSR. Full-frame results include that
shared-query effect; they do not isolate an exclusive inline-GI GPU interval.
The existing depth-bound generation pass is retained for this experiment.
Performance must be measured: fewer bound instructions can trade against more
level-zero reads and a different compiler register allocation.

## Screening and rejected variants

`zero-weight.patch` repeats a previously investigated idea in a narrower form:
skip GI for exactly zero diffuse contribution. Frame-120 final-output PFM
matched A/A and A/B bitwise, but timing controls drifted from 28.64 to 23.26 ms
median. No speed claim follows. This dynamic guard is **absent** in the selected
candidate. The earlier similar experiment is documented in
`../2026-10-26-street-120/zero-indirect.patch` and was not promoted.

`pixel-first.patch` consults the exact texel cache before the minimum-depth
bound; short screening did not establish a gain. `pixel-only.patch` combines
zero-weight admission with exact texel caching. Its first display-only image
oracle failed strict parity; raw reports remain in `pixel-only-quality-route`.
`exact-pixel-cache.patch` isolates depth caching, without the zero-weight branch.
`screen.py` runs short ABBA diagnostics only, never absolute-120 qualification.

## Correctness: native source stages, not display-only identity

The old `snapshot` helper exports the final linear display output as Float32
PFM values decoded from an RGBA16Float surface. This is **after** the display
transform, not scene HDR before TAA. The early display-only comparison cannot
establish precision of GI before post-processing. `compare_pfms.py` and the
retained `pixel-only-quality-route/numeric-comparison.json` quantify it.

The added `VOXY_STREET_HDR_CHECK_DIR` captures both final-output snapshots and
actual native renderer stages at frames 120, 600, 1200, 1800 and 2520. It uses
`capture_hdr_stages`, preserving source bits and the actual GPU temporal
uniform. Reports mark `correctness_capture_active=true`; the performance
qualifier rejects them. No readbacks occur in timed performance runs.

`quality-route/quality.json` compares A/A and A/B over complete deterministic
routes, tied to exact binary SHA-256 hashes. Native source-stage hashes and
formats are in each `frame-*/stages.json`. Strict bit parity is reported
separately and is **false**. Even A/A has small colour differences at some
phases; these are retained, not assumed to be caused by GI. Their root cause
has not been established.

Before capturing the selected native-stage proof, `quality-policy.json` set
these numerical limits: scene HDR before TAA may exceed the per-channel A/A
range by at most one source half-float ULP; post-processing colour stages by
at most two. Motion/depth must stay inside the exact A/A range. Values must
be finite, GPU temporal uniforms identical, and scene A/A changed-channel
coverage at most 0.1%. This is a representation-bound check, not a claim of
bit identity or a universal perceptual-quality metric. No depth/input precision
or sampling quality is reduced to obtain it.

The selected candidate passes these declared native bounds at all five phases.
Before TAA, A/B changes 7, 6, 4, 9 and 2 of 8,294,400 channels respectively.
Maximum excess beyond the local A/A envelope is 0.0001220703125; zero channels
exceed the allowed source-ULP bound. Motion/depth and temporal uniforms match.
Raw readback data remain local, ignored by Git along with PNG/PFM captures;
formats, hashes, per-stage statistics and unsuccessful controls are versioned.

## Full-cycle paired performance protocol

`run_pairs.py` freezes executables and runs A/A, five alternating AB/BA pairs,
then final A/A. Each run renders one full 20-second route: 2,400 measured frames
after 180 warmup frames, one fixed 1/120 simulation step per rendered frame.
Each offscreen frame waits for its exact GPU submission including postprocessing.
GPU timestamps are measured independently. CPU service through GPU completion
and successive host-completion intervals are retained separately; neither is
display scanout qualification.

`analyze_pairs.py` checks raw quantiles, full resolution, identical simulation
replay, untouched effects, GPU timing availability, absence of image capture,
quality-proof binary identity and A/A controls. Thresholds: A/A service p50/p99
and completion p99 ±10%, GPU mean ±5%; paired p99 CI upper bound ≤1.05; every
candidate service and completion p99 ≤8.33 ms. Code promotion additionally
requires the native quality bound and confirmed GPU reduction (paired GPU
ratio CI upper bound <1). Thresholds are not adjusted to accept noisy runs.
All runs remain available, including failures. Relative parity or GPU savings
do not establish the absolute 120-Hz target.

## Validation

`tests-final-render.log`: 353 passed, 84 ignored, zero failed.
`tests-final-app.log`: 26 passed, 2 ignored, zero failed.
`tests-native-quality.log`: bounds accept one source ULP and reject two,
nonfinite values, excessive A/A noise and mismatched GPU uniforms.
`tests-qualifier.log`: rejection of shortened workloads, wrong replay, missing
metrics, false quantiles, failed quality proof, wrong executable identity,
deadline misses and readback-contaminated timing. Ignored tests are not passes.


## Conservative tile fast-forward experiments

`tile-skip-v1.patch`, `tile-skip-scalar.patch` and `tile-long-run.patch`
try to avoid intermediate original samples only when a conservative projected
ray interval proves every skipped point stays in the same minimum-depth tile
and in front of its depth bound. The original recurrence, maximum steps,
reach and crossing refinement remain unchanged. Native Metal fixtures check
262,144 inputs across four resolutions. The first helper checked 532,139
original points (62,213 accepted hints), with zero false skips. The scalar
helper checked 527,419 points (61,231 hints), also with zero false skips.
These finite fixtures are not an exhaustive proof of all projections.

The four short ABBA campaigns are screening only. Array-plane, fast-forward,
scalar/unrolled and long-run variants did not establish an improvement;
observed GPU costs were higher. None is enabled in the shipped renderer.
The patches retain the failed implementations and the native fixture tests.

## Paired history experiments

`history-ssbo.patch` co-locates original half-float RGB and Float32 depth in a
12-byte record. The codec passes 65,536 native inputs including finite half
patterns, signed zeros/subnormals and random native depth, with zero channel
word mismatches. It adds a full-screen GPU packing pass. This is not sufficient
for renderer equivalence: transparent shading reads background after the
current opaque update, while historical depth remains previous-frame depth.
The first SSBO reader changed that colour epoch and is rejected.

The MRT prototype instead specializes only opaque fragments. Transparent
fragments retain the entire original shader/resource interface. Two managed
RGBA32Uint history textures alternate read/write ownership in queue order;
CPU capture waits for exact GPU completion. The extra allocation at 1080p is
66,355,200 bytes plus a 4-byte completion marker when timestamps are enabled.
Other devices and insufficient limits/budget retain the original resources.
A timestamped dependency marker includes final history stores in the GPU span.
Original background/depth copies remain for transparent surfaces and water.

The initial MRT trial failed a real-frame bit check: RGB differed in 890,774,
889,745 and 890,211 pixels; depth was identical. The first differing native
values revealed target conversion rounded toward zero, whereas quantizeToF16
rounded to nearest. The corrected helper selects the adjacent finite half
value toward zero. Its native Metal fixture tests 65,536 Float32 inputs near
signed half boundaries, subnormals, signed zero, and untouched clear pixels:
all four decoded channel word mismatch counts are zero. The real frame-120
check also passes for all 2,073,600 pixels. This is device-specific conversion
validation; it does not establish whole-route GI parity by itself.

The short MRT ABBA screen has GPU means 13.0641, 13.0944, 13.1282, 16.2012 ms.
Its A/A drift fails the declared 5% control; neither speedup nor regression is
qualified. Candidate service p99 values are 18.0491 and 18.0590 ms, above the
8.33-ms target. All failed startup/conversion logs are retained. No samples
are removed to make controls pass.


## MRT full-route rejection

`mrt-quality-route/quality.json` checks five complete-route phases against two
baseline runs. Physics/weather/water replay and actual GPU temporal uniforms
match. Every packed history channel is bit-exact at all five checkpoints.
However, scene HDR before TAA changes 240, 466, 141, 471 and 253 channels;
86, 166, 3, 162 and 96 exceed the declared one-ULP plus pixelwise A/A envelope.
Maximum absolute scene difference is 0.0457763671875. This fails the native
quality gate even though the codec itself is exact. The cause of the remaining
renderer differences is not established; compiler effects are not proven.
`mrt-qualification.json` therefore rejects promotion. A second full performance
campaign for this already quality-rejected candidate was not performed; the
14-run campaign above belongs to exact caching, not MRT.

`history-mrt-rtz.patch` retains the complete opt-in prototype, memory accounting,
completion-tail timestamps, native fixture tests and diagnostic mismatch checks.
Applying it is an experiment, not a qualified production upgrade. Production
source restores the original renderer. PNG/PFM and native binary readbacks
remain local and ignored; JSON stage metadata retains formats and hashes.

Final Python validation: 23 original native-quality/qualification tests and 18
MRT qualification tests pass. The native MRT fixture and native codec fixture
both pass on this M4 Max; these do not override the failed image gate.


## Retained source validation

`tests-shipped-render.log`: 353 passed, 84 ignored, zero failed.
`tests-shipped-app-minimal.log`: 26 passed, 2 ignored, zero failed.
`build-shipped.log`: release example compiled successfully. These runs use the
restored original renderer plus only the 16-line capture-harness change.
`retained-smoke-proof.json` identifies the final executable and checks that a
native-stage capture is produced and the report marks it timing-ineligible.
The short smoke is not part of the performance qualification.
