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
