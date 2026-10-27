# Original GI: dependency/cache diagnostics, 2026-10-27

Base: `6672bbf95`. Apple M4 Max / Metal, 1920×1080 Temporal AA, complete
street geometry, original Float32 materials, one 1/120-second simulation step
per completed render. No idle admission, other-task pause, load correction,
quality setting reduction, trimmed frames or retry of noisy controls.

## Ablation and unsuccessful shader experiments

`profile.py` freezes the executable and runs baseline, disabled-GI, disabled
counters, then reverse controls. These are diagnostics, never candidates or
120-FPS qualification. GPU means with GI disabled: 6.51–7.29 ms, baseline
14.06–19.19 ms. This identifies material cost of GI but not an exclusive GI
interval: passes overlap and the unchanged baseline drifts by 36%. Disabling
GPU timestamps still leaves service p99 18.17–19.46 ms. Exact per-frame GPU
submission waits remain enabled, independent of timestamp collection.

`prefetch4.patch` and `prefetch-ahead.patch` speculate original depth reads,
then admit results in original order. The R32Float compute fixture compares
65,536 original queries (16,348 positive hits), with zero differing Float32
words. The actual fragment route nevertheless fails native image bounds.
Four-read prefetch changes 2,331–3,693 pre-TAA channels at four checkpoints and
3,571 at frame 120; its maximum scene difference reaches 1.474609375.
One-read lookahead also fails the frame-120 rejection screen. No promotion.

`compare-refinement.patch` uses exact nearest Depth32 comparison only for
binary refinements; `compare-march.patch` extends it to marching. The native
predicate fixture checks 16,588,800 finite comparisons and a later 22,809,600
edge comparisons (including NaN/Inf), with zero mismatch pixels. Frame 120 is
bit-exact. Refinement-only native scene bounds pass all five phases, but two
values in the previous TAA history exceed the post-process bound. Its full
quality proof is therefore rejected. The march ABBA timing control drifts
13% and establishes no speed claim.

`quad-cache.patch` gathers exact 2×2 Depth32Float words into a local cache.
All 2,073,600 native fixture values match textureLoad bitwise. The full scene
still fails image bounds (3,070–11,908 changed pre-TAA channels). No promotion.
The finite native read/predicate fixtures do not establish full rendering
parity. In particular, this checkout compiles compute kernels in safe math,
while original raster GI uses its existing fast-math contract. No causal
explanation for the remaining fragment-image differences is established here.

## Counterbalanced full-frame refinement-cache experiment

The production ray body, direction generation, reach, sample count and eight
refinements remain byte-for-byte unchanged. The existing
`ARCHIVE_REFINE_CACHE` specialization is tested with two prebuilt pipeline sets.
AA creates identical baseline sets; AB/BA use the exact existing cache in set B.
Only one render occurs per simulation step; histories are not forked, copied
or reset between arms. The default renderer allocates no alternate pipelines.

`crossover/protocol.json` was declared before this campaign. An eight-frame
ABBA BAAB pattern balances both phases of every-other-frame GPU sampling.
BA complements every replay frame. Two AA controls measure internal label
bias; four complete AB/BA routes estimate relative service/cadence p99 and GPU
mean. Two separate BB single-pipeline routes test the absolute deadline.
External start/end load drift is retained and reported, not used to normalize
samples. Mixed runs are explicitly ineligible for single-pipeline completion
qualification; the separate BB route must meet the deadline on its own.

Timestamp records now carry the producer replay ordinal and actual variant,
not the later CPU collection frame. Pending warmup counters are drained before
clearing measurement arrays; final counters are drained without another draw.
The smoke has GPU producer IDs 181–191, paired with six measured samples and
no warmup leak. The qualifier rejects wrong producer IDs, duplicate stamps,
wrong AA policy, missing GPU data, false quantiles, truncated routes, readback
contamination, mixed/single-arm mislabelling, replay mismatch, wrong executable
or asset identity, quality failure and a missed separate BB deadline.

The eight completed full routes pass both internal AA controls. GPU ratio
CI95: [0.974993, 0.980275]; service p99 ratio CI95: [0.975969, 0.991965]; host
completion p99 ratio CI95: [0.975707, 0.983464]. Thus the timed cache path saves
about 2–2.5% GPU time under this protocol. **It is not promoted:** its full
native image check fails, and separate BB service p99 is 23.3873 / 21.4804 ms.
`crossover/qualification.json` records both failures. Relative savings are not
120-FPS qualification, and offscreen completion is not display scanout.

Materials for this campaign are a read-only APFS copy-on-write snapshot;
`assets-manifest.json` retains hashes and provenance mtimes. Frozen executable
and asset hashes are checked by the run and quality harnesses. Binary texture
readbacks and PNG/PFM files remain local and ignored. All failing controls,
quality checks, native fixtures, source patches and raw timing arrays remain
available. Nothing is deleted to make a control pass.

## Validation so far

Four scheduler invariant tests, 18 crossover qualifier tests and 15 legacy
qualifier tests pass. Native prefetch, comparison and quad fixtures pass in
their explicitly limited scopes above. The release crossover smoke passes.
Default rendering still uses the original, uncached raster path.

## Original GI arithmetic contract and stage crossover

The default raster GI compiles with Metal fast arithmetic, while the existing
`metal-safe-compute` policy forced every compute kernel to precise arithmetic
and disabled contraction. This is a reproducible cause of disagreement in the
standalone original-GI reference experiment, not proof of the cause of every
previous full-shader image difference. An explicit Apple Silicon / Metal /
integrated-GPU opt-in (`VOXY_STREET_GI_RASTER_MATH`) lets only the existing
`cs_opaque_gi` entry match its source raster arithmetic. Compensated paired DDA
still takes precedence and unrelated compute entry points remain precise.
The actual backend policy file is also exercised by the render unit tests.

With the original safe arithmetic, both repeated calibrations disagree with
the same-history original fragment reference in 57,919 of 8,294,400 Float32
words at frame 120 (maximum absolute difference 4214.0625). Matching raster
arithmetic gives zero differing words. The full-route, same-history native
reference then matches in **all 41,472,000 Float32 words** at frames 120, 600,
1200, 1800 and 2520, with 280,836–301,839 covered reference pixels per phase;
all values are finite. This is raw GI reference parity, not final image parity.

The complete final-image check **fails** the unchanged native quality policy
at all five phases. At frame 120 the scene-before-TAA target differs in 3,737
channels, 3,305 outside the allowed quantization/A/A envelope; maximum absolute
difference is 0.350830078125. The AA scene target itself is exact. Thus matching
the reference query does not establish equivalence of its embedding into the
complete forward shader. No unsupported causal claim about compiler RNG
reassociation, derivatives or surface admission is made. This path is not
promoted and default rendering keeps original inline GI. No ray count, maximum
distance, refinement count, source format or image tolerance was reduced.

The same counterbalanced schedule now supports original-inline versus original
compute GI with `VOXY_STREET_GI_STAGE_CROSSOVER=1`. Probe baking always uses the
original path; A frames do not encode compute GI or resolve its unwritten GPU
queries. Both query mapping and decoding use only the active range. Producer
frame identities, labels and actual selected variants are retained. Mixed
frames cannot claim single-path absolute qualification. Correctness captures
and precision-reference passes are excluded from performance.

`math-match/stage` retains the preregistered eight-route protocol, frozen binary
and asset identities, full raw frame samples, native quality policy and raw
GI proof. The analysis requires both full-image quality and exact raw GI proof
with sufficient nonzero coverage. Twenty-eight validator tests exercise the
negative gates as well as the passing fixture. Native binary readbacks remain
local and ignored. The first quality analysis failed because its policy file
was absent in the new folder; copying the unchanged policy and analyzing the
already completed captures produced the recorded failure without rerunning or
selecting captures.

The new AA original path was additionally compared with the retained original
fragment binary's two full-route captures. The unchanged native quality bound
passes all five phases (`math-match/default-original-compatibility.json`);
scene-before-TAA is exact at four phases and has one changed channel at phase
1200 within the existing bound. This reuses frozen captures and does not claim
a fresh default-mode or cross-platform runtime qualification.

All eight new full-route runs completed without rejected/retried rows or an
idle-window request. Both internal AA controls pass. GPU timestamp-interval
ratio CI95 is [0.8077769753555537, 0.8344082231665644]; completed service p99 ratio CI95 is
[0.7897592587463583, 0.8451964271875996]; host-completion interval p99 ratio CI95 is
[0.7979156708629342, 0.9012547022096453]. This establishes a roughly 16.6–19.2%
GPU-interval saving for the experimental stage move under this protocol,
without treating overlapping stages as exclusive work. Separate BB full-route
service p99 is 14.627458 / 14.661333 ms; completion-interval p99 is
15.062792 / 15.063209 ms. **Both absolute 120-FPS qualification and candidate
promotion fail.** The quality gate fails despite exact raw query-reference
parity. No faster path was enabled by default.

The BB GPU guide/query intervals average about 1.40 / 3.32 ms. These intervals
help localize remaining cost but must not be summed with overlapping shading
intervals as an exclusive frame decomposition. Full-frame fence measurements
remain the absolute deadline authority. The next unresolved correctness issue
is equivalence of the complete shader embedding, not standalone query math.

Final validation: `cargo test -p voxy_render --lib` passes 359 tests (84
explicitly ignored); `cargo test -p voxy_app --example street_walk` passes
26 tests (2 explicitly ignored); 28 stage-qualification validator tests pass.
The release executable used by every new stage campaign has SHA256
`f3ad1b3cc6b2be09ab9290ef889d5fccdd489813414055a3a2a50afff4f9600f`.
