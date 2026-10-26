# Original street input repeatability gate

BVH8 remains opt-in. Minimum 120 FPS and the full motion cycle are not qualified.
Current complete geometry has 117857 instances and 3480232 instanced triangles.

## Observed original-control failure

`artifacts/original-street-realism-2026-10-25/wide-full-street-v1/qualification/`
contains two successful original-path processes. All native camera, lighting and
material uniform words matched, but four SH words of probe 82 differed before any
source query. This is not evidence of BVH8 precision loss: no wide candidate ran.
It also cannot be ignored when qualifying a complete rendered frame.

`artifacts/original-street-realism-2026-10-25/projection-operands-v1/` retains
lightweight, in-projector observations from two further complete original scene
runs. All 2232 face reductions and subsequent SH accumulations in each run agree
bit for bit with independent IEEE-f32 reconstruction. For probe 82, all 109350
sample records reconstruct all 9216 thread-partial RGB words exactly. Between
runs, thread partials changed at probe/face 197/2, 264/3 and 297/3. Uniform records
matched. Main image words also changed at frames 3 and 4. The cause is not fixed.

The observer does not add a GPU dispatch or readback between faces. It preserves
the canonical production shader file and inserts observation statements into an
explicit diagnostic variant. A single readback occurs after baking completes.
Replay of six complete historical F16 source faces, repeated 20 times, produced
all 120 original coefficient results bit for bit. Expected coefficients remain
CPU-only. These are correctness checks, not frame-rate measurements.

## Diagnostic interface

Set both `VOXY_STREET_PROBE_PROJECTOR_TRACE_INDEX` (an in-grid probe index) and
`VOXY_STREET_PROBE_PROJECTOR_TRACE_OUTPUT` (a fresh directory) before beginning
baking. The optional storage allocation is charged to the existing device memory
budget and rejected if it exceeds device binding limits. It never truncates a
bake. With these variables absent, the canonical shader and three bindings apply;
no trace storage is allocated.

Schema 2 additionally records ordered integer hashes of each thread's actual
radiance RGB operands and weight/normal operands for every probe. Different hashes
prove different observed operand sequences; equal hashes are not a bitwise input
identity proof. The selected probe still retains all actual sample words. The
independent auditor verifies its hashes against those complete native records.

```sh
python3 tools/scene-research/audit_probe_projection_operands.py CAPTURE_DIR \
  --compare OTHER_CAPTURE_DIR --output proof.json
```

The auditor reconstructs thread sums from recorded operands for the selected
probe, and reductions/SH accumulation for every probe. It applies no epsilon and
explicitly rejects unsupported non-normal sample domains rather than silently
assuming a floating-point flush policy. Capture directories and frozen executable
identities are retained under `projection-operands-v2/`; consult its `status.json`
and terminal `process.json` records for current progress.

## Remaining gates

1. Identify and fix the original scene's varying inputs; diagnostics alone are
   not a fix, and additional stable runs would not prove one.
2. Compare current original and admitted BVH8 full-frame native outputs with
   identical verified inputs, including all probe coefficients and source data.
3. Measure isolated, alternating original/candidate runs without observations or
   source audits; retain rejected controls and separate GPU stages from whole
   frame completion.
4. Qualify the complete motion cycle and the actual 120-Hz workload. The existing
   offscreen fixed-30-step stress batch is not one ordinary 120-Hz frame. No
   presented-frame-rate claim may be derived from that diagnostic workload.

## Subsequent evidence and material corrections

Schema-2 original controls in `projection-operands-v2/` differed at probe 225,
face 2, thread 35: two RGB operand hashes changed, while all uniform words and
weight/normal hashes matched. Both complete reductions and accumulations still
passed the independent arithmetic audit. This proves an observed radiance operand
sequence changed; it does not identify which upstream rendering operation caused
it.

The original sidewalk material evaluated its footprint derivative inside
`if side_slab > 0`, where the mask depends on fragment world coordinates. This
violates the uniform-control-flow requirement for defined derivative results
([WGSL derivatives](https://www.w3.org/TR/WGSL/#derivatives)). The footprint now
executes before that guard; no material formula or detail term was removed. The
new production-IR regression failed on the old shader and passed after the move.
The local Naga 30.0.1 dependency sets
`DISABLE_UNIFORMITY_REQ_FOR_FRAGMENT_STAGE = true`, so its successful shader
validation alone cannot establish this property.

`sidewalk-footprint-hoist-v1/` retains two completed full-scene controls after
that correction. They still varied at probes/faces 64/2, 108/3 and 158/4, so the
correction did not eliminate the original repeatability problem. BVH8 was again
not launched; the strict A/A gate rejected advancement.

The next frozen candidate in `material-fine-footprints-v1/` explicitly selects
fine-quad derivatives for the original material footprints (including bump,
normal variance and wetness). Plain derivatives may select fine or coarse
results under WGSL; the explicit choice retains the detailed material terms and
F32 arithmetic. Twenty-one archive-scene tests pass, including the two new
production-IR checks. Consult that candidate's pipeline and process records;
neither root-cause closure nor 120 FPS is claimed.

If this experiment still varies, repeating target-probe captures alone is not a
useful next optimization gate. A legitimate baked-lighting asset with complete
scene/material/image/IBL/weather dependency validation, or a same-process paired
query diagnostic sharing the actual verified producer inputs, can establish
identical inputs for traversal qualification. Either requires preserving the
existing scene owner, restoring mutable query history where applicable, exact
native storage, and explicit reporting that the baker's variability remains open.
Ignoring SH differences or applying a tolerance is not an acceptable substitute.

## Current source refresh and exact interpolation regression

The shared checkout advanced to `0a318afc2` while preparing the fine-material
binary. This includes additional rendering changes and geometry expansion; it
is not an isolated before/after comparison of derivative choices. The pinned
binary now exports 119909 instances and 3544272 triangles. The previous 3480232
triangle reference was rejected, not silently reused. Fresh source data and
qualification live under `material-fine-current-source-v1/`. Its independent
serialized BVH8 proof checks all 463273 original leaf records and 586976 retained
bound/subtree associations; the worst pending frontier is 64. All 27 query shader
literals remain byte-identical in this binary and the earlier source-qualified
GPU executable.

The retained current-source control and its repeat both completed, but their
radiance operand hashes differ (including probes 88, 118, 126, 127, 235 and 251).
Neither the divergent-derivative correction nor explicit fine derivatives has
established repeatable full baking. No full-scene BVH8 candidate ran. Pipeline
statuses now record terminal rejection rather than leaving a stale running state.

Separately, the shared update factored the eight-corner interpolation into nested
bilinear stages, changing its F32 rounding sequence. An actual Metal regression
compared production interpolation against the original eight-term loop on 131072
inputs: 43379 of 524288 native output words differed. Production now retains
unrolling while multiplying each complete original corner weight and summing in
the original order. The same inputs and coefficients subsequently matched all
524288 output words bit for bit. Raw before/after output pairs, frozen test
executables and input digests are retained in
`probe-sampling-original-order-v1/`. These are sampling fixtures, not a full-scene
or FPS qualification. Shadow normalization also retains the original `/ 12.`
operation instead of multiplying by a rounded reciprocal. Other concurrent
scene/rendering work is preserved.

The useful next traversal gate is a same-process diagnostic on actual renderer
producer buffers: provision original and BVH8 certificate pipelines over one
immutable source owner, preserve any mutable seed/history state, evaluate the
same actual rays, and compare protected outputs plus actual GI/SSR consumers.
This avoids attributing independent baker variation to traversal. It must remain
separate from the later uninstrumented full-cycle frame-rate run; no input
precision, unknown-case handling, source predicate or full fallback may be
removed. Another series of independent target-probe captures is not a substitute.

## Same-process traversal and native consumer gate

The opt-in `VOXY_STREET_WORLD_RAY_COMPARE_WIDE=1` diagnostic now executes the
original certificate traversal followed by BVH8 on the same actual GPU-produced
ray buffer and immutable source owner. Both paths use the same exact-integer
pipeline and existing full fallback. Protected ray inputs, hit/parameter words,
material semantics, UVs and all world-position extension words are compared as
integers on GPU, with no tolerance. Only explicitly documented queue padding and
route/provenance metadata are excluded. History seeds, work statistics, timing
profiles and any-hit modes are rejected for this diagnostic. The first mismatch
retains the complete original/current coefficient and result records.

`same-producer-wide-v1/source-matrix/proof.json` records 24288 actual protected
result comparisons across four source configurations and four policy/order pairs
each, including separate native seeding, known UV28 materials and the unready
vertex-cache fallback. All comparisons passed, with nonzero admitted and matched
hit counts. The independent source-rational and vertex-cache audits also passed.
This is bounded source-query evidence, not the full street or an FPS result.
Actual Metal comparator negative controls cover changed low words, IDs, material
semantics, ties, request tails and permitted metadata; the v3 run additionally
checks the complete captured failure record.

The renderer diagnostic also runs the actual GI/reflection resolve PSO on the
original query snapshots and then the candidate queries, into the SAME actual
RGBA32Float output texture. It copies native bytes after each pass and compares
all four words of every tile pixel with integer loads, preserving subnormals and
signed zero. The original resolve has a private audit buffer; the candidate
output remains for opaque rendering. Row padding is excluded, tile coverage is
validated as a complete non-overlapping partition, nonfinite outputs fail, and
only the final cumulative proof is read back. No per-tile CPU fence or generated
expected image is involved. This doubled diagnostic must never be timed as the
production candidate.

Seven actual Metal synthetic HDR comparator cases passed (equal, last-pixel low
bit, signed zero, subnormal, equal NaN, equal infinity, unused row padding). The
same run does NOT qualify a scene. The full-street gate is running separately in
`same-producer-street-HDR-v1/`, with a pinned executable, exported current source,
settings and retained process lifecycle. That source has 119998 instances and
3545900 triangles; its independent CPU conversion checks all 463519 leaves and
587276 retained bound/subtree associations, with a maximum pending frontier of
64. The serialized conversion is lossless; actual full-street query/consumer
proof remains pending until its report passes. Scene additions made concurrently
in the shared checkout are retained and are not attributed to traversal work.

The additional `--offscreen-render-every-step` benchmark option advances exactly
one 1/120-second physics step per rendered frame. It preserves the old 30-step
stress batch default and is rejected outside a timed offscreen benchmark. Reports
separate complete rendered 120-Hz cycles from physics-only cycle counts, require
at least 2400 measured frames and matching physics-step counts, and use actual
completed-frame wall time for the minimum-120 test. Adding this measurement mode
is not a claim that the renderer meets that threshold.

### Full-street semantic-count rejection and correction

The first paired full-street run is now terminal (exit 1). Its actual consumer
HDR outputs matched all 49766400 pixels / 199065600 native words, but the query
gate correctly rejected a protected word-19 difference: original ties=1,
BVH8 ties=0 at the first retained ray. Both paths selected source slot 5633,
primitive 9 with identical parameter and position pairs. No comparator mask or
tolerance was changed to admit this result.

`same-producer-street-HDR-v1/independent-failing-ray.json` independently decodes
the original exported vertex/instance words and uses exact rational arithmetic
for ray/box and triangle tests. Of 112 source triangles in 127 visited nodes, the
nearest geometric event is uniquely slot 5633/9 at 53546752/237969945. Three
farther events at slots 4171/4, 4173/5 and 4179/5 share 31785268/139418835. The
shared exact nearest update incremented a tie counter for a temporary farther
winner and never cleared it upon replacement by a strictly closer event. Thus
the counter was accidentally traversal-order-dependent rather than the number
of ties of the final nearest result.

The common exact update now clears superseded ties on a strict nearer result;
true equal-distance ties, their deterministic source-ID ordering, all original
source operands and exact arithmetic remain. A Metal control-flow regression
extracts the two production update/save functions verbatim, with explicitly
mocked small exact arithmetic. The old update fails the farther/farther/nearer
sequence (ties 1 instead of 0); the corrected update passes all seven ordering
and final-tie cases. This limited control-flow proof is separate from geometry
qualification. A real three-surface source regression was also added to the
material query matrix, requiring final ties=0 in both policies and orders.

The fresh `same-producer-street-HDR-v2/` executable contains the corrected shader
literal and exports a byte-identical geometry manifest. It reruns the extended
source-oracle matrix before the complete query/HDR gate. The previous rejection,
raw first-failure records and all artifacts are retained. Neither successful HDR
comparison alone nor this counter fix qualifies 120 FPS.

### Corrected fixture and bounded scheduling candidate

The v2 source-matrix child exited 101: the new regression appended its three
meshes after creating the immutable GPU scene. Its CPU fixture dump contained
surfaces absent from that GPU owner. The test preparation is corrected by adding
those meshes before scene creation; existing IDs remain stable. Rejected v2
artifacts are preserved and its dependent street run correctly did not start.
The v3 pinned query executable now prepares all 42 source instances / 126 cached
vertices. Its first geometric/order-false pair passed 1559 protected record
comparisons with 1066 matched hits, including the transient-tie regression;
remaining policy/order/configuration results are pending. The dependent full
street starts only after all four matrix children terminate successfully and
their independent audits pass. No producer is restarted because cold Metal
compilation takes several minutes.

`benchmark_cycle.rs` now owns the additional completed-frame qualification gate.
Both tests passed: an 80-render/2400-physics-step batch cannot qualify, and a
rendered 2400-step cycle requires every actual completed frame within 1/120 s.
Incomplete cycles, mismatched steps, diagnostic capture, wall/presented mode,
nonfinite/nonpositive durations and a single over-budget frame fail this
completed-offscreen gate. These pure qualification tests are not runtime FPS
proof.

The optional `wide_source_near_first` candidate orders every existing BVH8 child
using the same projected-centroid scheduling key as the binary path. Descending
keys on the LIFO stack visit nearer hints first. It changes no AABB, leaf, source
coefficient, material predicate, exact arithmetic or complete fallback. The
original comparison pipeline explicitly disables this candidate. Flags require
wide scheduling, default false, and are exposed as
`VOXY_STREET_WORLD_RAY_WIDE_NEAR_FIRST` and
`VOXY_RAY_MATERIAL_SIDES_WIDE_NEAR_FIRST`; proof reports record the effective flag.
The actual extracted WGSL scheduling block passes Naga validation. Its bounded
Metal control test is pinned and queued after the earlier owned GPU pipeline;
source-query, full-street fidelity and performance acceptance are still pending.
Release binaries are pinned in `wide-near-first-candidate-v1/`. Its newly exported
source manifest matches v3 byte for byte, including all original payload digests.
No candidate gain or 120-FPS qualification is claimed.

Primary research checked for this candidate:

- [PBRT 4, BVH traversal](https://pbr-book.org/4ed/Primitives_and_Intersection_Acceleration/Bounding_Volume_Hierarchies):
  visiting a nearer child first can tighten the closest-hit range before other
  nodes are tested. Voxy retains its existing conservative range proof rather
  than using the scheduling key as an intersection distance.
- [Ylitie, Karras and Laine, HPG 2017](https://research.nvidia.com/sites/default/files/publications/ylitie2017hpg-paper.pdf),
  sections 3.2 and 3.4: wide-node sorting has a cost, and build-time octant ordering
  can reduce that overhead. This warns against assuming the insertion candidate
  is faster. A fixed octant permutation is a possible later measured alternative;
  it is not implemented or accepted here. The paper's compressed bounds and
  NVIDIA performance results do not prove Voxy precision or Apple Metal speed.

### Serial qualification handoff

The v3 `paired-unknown-uv20` and `paired-native-unknown-uv20` configurations have
both finished with exit 0 and their independent source/cache audits passed:
12472 protected records in total, including 7872 matched hits. The native-known
UV28 configuration is still live; do not replace a cold-compiling Metal child
based on elapsed time alone.

`wide-near-first-candidate-v1/qualify-after-baseline.py` now waits for the owned
baseline full-street proof AND a terminal successful scheduling-block Metal
control. It then runs the real source-oracle matrix with the effective
near-first flag explicitly checked, followed by the complete same-process street
query/HDR gate. It never starts a GPU producer while those dependencies are
active, and a failed prerequisite prevents candidate qualification. All three
pinned binary SHA-256 identities and controller Python syntax were rechecked.
These are running qualification dependencies, not FPS or completed-cycle proof.

### V3 bounded source gate passed

All four v3 source configurations are terminal with exit 0, including unready
vertex-cache fallback. Reinspection of their process records, independent
rational-source/cache audits and all sixteen same-producer GPU proofs confirms
24944 protected record comparisons, 24928 source-rational query checks, 16810
exact hit-position checks and 16 partial hit-to-inactive checks. No expected
geometry results were uploaded. The aggregate `source-matrix-qualified.json`
records these scopes and SHA-256 identities of each independent audit.

The dependent current-street capture has now started with its pinned binary.
Its process is live; source/HDR fidelity remains pending, and the candidate
scheduling-block test and near-first qualification still wait behind it. The
bounded source gate does not establish full-street fidelity, photorealism,
complete rendered motion or any FPS threshold.

### V3 complete-street query/HDR gate passed

The pinned v3 street process is terminal with exit 0. At 3840x2160 over three
query-producing frames, all 49766400 protected query records match the original
binary scheduling and all 49766400 actual RGBA32Float GI/SSR pixels (199065600
native words) match with zero nonfinite channels. There are 20681459 explicitly
admitted records and 7888405 matched hits, so the gate is not vacuous. The former
tie-count mismatch is gone without masking word 19 or relaxing any comparison.
The independent source cache audit checked all 42780672 words of 3565056 world
vertex records and 13439776 original GPU instance bytes, with zero tolerance;
6571 nonrepresentable cache vertices retain the complete original fallback.
See `same-producer-street-HDR-v3/proof.json` and `independent-vertex-proof.json`.
This is four rendered frames / 120 physics steps, not a complete motion cycle.
Its doubled diagnostic frame times are not production FPS measurements.

The production scheduling-block Metal control also passed all 28 frontier cases
with ordering disabled and all 28 enabled; prior stack entries and every child
were retained. The dependent near-first source matrix has started. Its pinned
binary/source identity and new ordering still require their own complete source
and street gate, followed by performance qualification.
