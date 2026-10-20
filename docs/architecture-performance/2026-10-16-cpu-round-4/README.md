# Changing-pose CPU admission and simulation, round 4 — 16 October 2026

## Retained optimization

`SkinnedMesh::validate_temporal_pose` previously rebuilt the blended skin matrix
and its product with the model for every vertex, even when adjacent vertices had
identical joint indices and weights. It now reuses that product within a call,
using the existing immutable mesh coherence policy (at least 25% adjacent repeats).
Both joints and model can change on every call; nothing is reused across poses.
Meshes without sufficient repetition use the original position scan.

The operation still validates the complete palette/model first and checks every
position in vertex order. Matrix multiplication order is unchanged. It remains
position-only admission: singular normal transforms are not newly rejected.
No threads, allocation, resource formats or physics/lifecycle changes were added.
Existing render-pose grouping/certification and other concurrent work are preserved.

## Paired measurements

Two complete series of five same-process pairs per case, reversing before/after
order on alternate pairs. All ten pairs are included. The reference executes the
original scan; the candidate uses the production coherence policy. Each phase has
54 samples, discards three warmups, and reports the median of the remaining 51.
Every sample changes all 16 joint transforms and the model transform. Synthetic
meshes contain 100,000 vertices. This is CPU admission, with no device/window/GPU.

| Layout | Before CPU p50 | After CPU p50 | Change |
| --- | ---: | ---: | ---: |
| Same influences across 64 adjacent vertices | 675.229 µs | 121.959 µs | −81.9% |
| Mixed layout with 25% adjacent repeats | 693.584 µs | 529.708 µs | −23.6% |
| Alternating influences, original-path control | 677.542 µs | 697.563 µs | +2.95% |

Values are medians of ten per-phase p50 values, **not frame p99**. Other builds
were active on the host. The first series had severe outliers even on the unchanged
control; the whole series was repeated, and both series are retained. The combined
control median is just within 3%; this is narrow, noisy evidence for median CPU
cost and cannot establish a frame-tail nonregression. No application-wide speedup
or 120 FPS qualification is claimed. The old round-3 history timing used another
fixture/binary and is not used as the baseline here.

Raw logs are `temporal.log`, `coat.log`, and their counterparts in `repeat/`.
`pairs.csv` contains all 100 phase values; `summary.json` includes aggregate values
and SHA256 hashes of the saved binaries. Binaries were copied outside the shared
Cargo target directory before measurement, so concurrent builds cannot replace them.

## Rejected simulation candidate

The coat simulation computes an identical rest orientation separately for every
guide attached to a bone. A candidate computed each used bone orientation once
per step. Across 240 consecutive poses for both species, complete strand debug
snapshots (including previous/scratch/rest data) matched the original, and invalid
timestep rejection remained atomic. Each benchmark phase uses those same 240
poses, excludes the first 20 steps, and compares final full snapshots outside timing.

| Coat simulation | Before CPU p50 | Candidate CPU p50 | Change |
| --- | ---: | ---: | ---: |
| Dog | 11.334 µs | 11.188 µs | −1.29% |
| Cat | 11.563 µs | 11.542 µs | −0.18% |

The candidate fails the 5% retention threshold and **was removed from production**.
`rejected-frame-cache.patch` preserves the experiment and its two tests for review
and reproduction; it is not applied. This round deliberately makes no retained
claim of faster physics integration. Roughly 11–12 µs remains in coat stepping
on this fixture; profiling the strand solver/collision iterations is a more useful
next simulation investigation than caching the rest orientation alone.

## Validation

- Final animal/gait/fur suite: 10 passed, three manual benchmarks ignored.
- Final skinned/LOD/history CPU suite: 17 passed, four manual benchmarks ignored.
  New oracle regressions cover coherent/mixed/alternating influences, changing
  weights, cancellation, zero/negative scale, overflowing positions, invalid
  palettes, and exact position results against the original scan. Temporal
  admission still permits collapsed normal transforms when positions are finite.
- The rejected simulation candidate also passed its complete-history regression
  before removal (11 animal tests passed at that intermediate state).
- Release client tests compile the production integration; a separate client
  development compile check receipt is retained.
- Strict renderer Clippy stops at 15 diagnostics outside this round's admission
  changes. Workspace formatting also encounters existing/concurrent unformatted
  files. Logs are retained; `git diff --check` passes.

Full workspace tests, GPU smoke/visual checks, frame p99, chunk publication and
surface behavior are not requalified by this CPU-only round. Remaining measured
admission cost is about 0.530 ms for the mixed 100k fixture. Next-stage budgets
should be based on a short headless simulation/extraction trace and that changing
pose cost; a whole-frame budget cannot be assigned from these isolated timings.

Build release test executables and run the following exact ignored CPU test
(with the actual executable path returned by Cargo; it creates no GPU/window):

```sh
/path/to/voxy_render-tests --ignored --exact \
  skinned::temporal_admission_tests::paired_temporal_cpu_bench --nocapture
```

For the rejected experiment, inspect/apply `rejected-frame-cache.patch` in an
isolated checkout of this source state, build the app test executable, then run
`animals::fur::frame_cache_tests::paired_coat_cpu_bench` with the same flags.
