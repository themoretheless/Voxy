# Short main-client CPU replay — 21 October 2026

## Profile and retained change

A new optional main-client entry point, `--cpu-replay OUTPUT.csv`, returns before
creating an event loop, window, surface or GPU device. It calls the current
production routines for seeded world creation, actor/companion mesh bootstrap,
character physics, actor animation, sequential companion ticks with neighbouring
bodies, persistent palette assembly, render-pose admission, CPU pose snapshot
publication, last-presented temporal admission, and shadow-sphere extraction.

This is a replay of that CPU chain with separate orchestration, **not the entire
application**. Water/chunk work, vehicles/projectiles, GPU collision backends, LOD
preparation, GPU uploads, execution and surface/display behavior are excluded.
Geometry setup is outside timing; runtime simulation/pose work uses real geometry
and the production physics/animation functions. No rendering resolution or image
quality conclusion can be drawn from this device-free replay.

The initial motion trace used 1,990,328 vertices. Temporal admission consumed 79%
of total measured CPU time, render admission another 21%, while companion
simulation was below 1%. See `baseline-motion.csv/json`. The final paired reference
series has the same ranking: median per-run mean temporal cost 3857 µs/frame,
render admission 934 µs/frame, total 4818 µs/frame. Half the nominal 120 Hz replay
frames have no fixed tick; these values include those frames and are not per-tick
costs.

The retained optimization caches one scalar maximum authored position component
(at least 1) during the existing mesh-construction validation pass. A shared-index
view retains that scalar. Each changed temporal pose first receives the complete
original finite/affine palette validation, then a sufficient finite-position bound
from the current model, palette and immutable geometry. If the conservative bound
cannot certify the pose, the original exact scan runs. Small meshes keep their
original scan. No normal constraint, physics timestep, solver iteration, matrix
multiplication order, vertex data, or GPU publication contract changes.

[The arithmetic proof](temporal-position-bound.md) explains the deliberately loose
2^20 margin and fallback. The certificate introduces no worker threads or per-frame
allocation. The original repeated-identical-pose history fast path is preserved.

## Paired CPU results

Five AB/BA pairs for each of `idle`, `motion`, and `catchup`; the entire disputed
stress scenario was repeated for another five pairs. Both complete stress series
are included, not a selected subset. Each phase starts afresh from seed 42, warms
120 frames and measures 240 frames. Normal scheduling is 120 Hz with physics at
60 Hz, not wall-clock pacing. Catchup executes two fixed ticks per frame. Both
variants use exactly the same geometry, palette/control trajectory and settings.
Geometry hashes and cumulative hashes of all per-frame states agree for every scenario across variants/runs.

Metric: median of per-run **CPU replay p99**, not presented/display frame p99.
Timers measure elapsed wall time of covered CPU stages, including CPU worker
scheduling, rather than summed core CPU time.

| CPU replay scenario | Pairs | Before | After | Change |
| --- | ---: | ---: | ---: | ---: |
| idle | 5 | 11.131 ms | 1.887 ms | 83.0% lower |
| motion | 5 | 10.248 ms | 2.107 ms | 79.4% lower |
| catchup | 10 | 9.379 ms | 2.213 ms | 76.4% lower |

The predefined comparison uses median run p99: motion must improve >=5%, other
cases must not regress >3%. The complete first series passes that CPU gate, and
the repeated stress series also has a lower median. This is evidence for a local
CPU improvement, **not proof of stable tails or the original four-scenario goal**.

Tail limitations are material: after combining both stress series, pooled CPU p99
is **12.888 ms versus 11.861 ms** (about +8.7%). Two first-series stress pairs were
adverse; the later complete series improved. Across 2400 candidate stress frames,
76 exceeded 8.33 ms and the maximum was 34.162 ms. Every adverse value remains in
`combined-pairs.csv` and the raw logs. Therefore pooled stress-tail nonregression
and stable <=8.33 ms operation remain unqualified. Host load varied and unrelated
work continued; no unrelated process was stopped. No claim of 120 FPS is made.

For normal motion, candidate pooled CPU p99 is 4.854 ms and max 6.878 ms across
1200 measured frames; none exceeded 8.33 ms. Idle candidate max is 2.887 ms.
These statements concern only the replay's covered CPU stages, not a GPU frame.

After the change, temporal admission averages about 1.28 µs/frame in motion;
render admission is about 867 µs/frame out of 892 µs total (medians of run means,
which need not add exactly). **Render admission is now the dominant measured
stage (~97%)**. Its existing per-call group-certification worker startup/scheduling
is the next CPU investigation, particularly for stress tails. Gait/fur/palette
micro-optimizations are a lower priority on this measured path. An exact remaining
whole-frame/GPU budget cannot be inferred here.

## Rejected threaded candidate

The first attempt reused full render group certification in temporal admission.
It passed correctness tests and reduced typical work, but launched another set of
scoped workers. Median CPU replay p99 rose from 10.831 to 19.663 ms for idle and
11.539 to 16.275 ms for motion. It failed the CPU gate and was replaced.
`rejected-threaded/paired/` retains all five pairs per scenario and its executable
hashes. It is not the final optimization or a qualifying measurement.

## Rejected reusable-worker experiments

Two private Rayon pool variants were also measured independently against the
finite-bound candidate, each with five AB/BA pairs per scenario. Both passed the
21-test correctness suite (including concurrent valid/reflected/overflowing poses)
but failed the performance gate and were removed, together with their new Rayon
dependency. Existing optional Rayon use elsewhere in the workspace is unchanged.

- `rejected-pool-per-group/pool-paired/`: per-group parallel iterator; median run
  p99 ratios were 2.837 idle, 1.169 motion and 1.535 stress. A following overall
  comparison was intentionally stopped after this complete independent matrix
  failed; partial phases are retained and are not qualifying.
- `rejected-pool-chunks/pool-paired/`: contiguous chunks with sequential inner
  loops; ratios were 1.016 idle, 1.124 motion and 1.489 stress.

The final production code retains the original render-certification threading.
Worker reuse is therefore a **rejected tested hypothesis**, not a retained speedup.
Further investigation should profile scheduling and per-group certificate arithmetic
under controlled load rather than assuming any thread pool improves the tails.

## Provenance and validation

A source snapshot was copied before profiling, including then-current tracked
changes and required untracked source files. Parallel working-tree edits did not
enter the builds. `before-source-manifest.json`, `after-source-manifest.json` and
`environment.json` identify the source and environment; `optimization.patch`
contains the actual change from the captured reference. Apple M4 Max, 16 logical
CPUs, rustc 1.98.0-nightly (d1fc603d1 2026-05-26), release profile. Saved executable
SHA256 hashes are in each series summary. Binaries were copied outside Cargo's
shared output directory before runs and checked for replacement by the collector.

The two manifests differ only in skinned.rs and a cfg(test)-only vegetation test
fixture. That pre-existing captured fixture needed explicit collection type
annotations to compile; `test-fixture-compatibility.patch` records this isolated
fix. It was not applied to the live parallel work. It changes no production replay
code, geometry or timed behavior.

- Release renderer skinned/LOD/history suites: **20 passed**, four manual tests
  ignored. New tests exercise certified acceptance, uncertain valid fallback,
  overflow rejection, reflection, collapsed normals, short/nonfinite palettes,
  shared indices and changing valid-invalid-valid inputs. Another oracle test
  covers 125 combinations of position/joint/model scale exponents and mixed,
  cancelling weights, spanning 2^-120 through 2^120.
- Release animal/gait/fur suites: **15 passed**, three manual benchmarks ignored.
- Python performance report/comparison suites: **10 passed**. Replay collector
  checks complete CSV coverage, finite timings, percentiles against raw rows,
  matching fixture/state hashes, and executable stability. It rejects incomplete
  or inconsistent results.
- Snapshot workspace release compile check passed. Strict renderer library Clippy
  (`-D warnings`) passed. New replay source formatting and changed-file whitespace
  checks passed. The whole workspace test suite/formatting/Clippy, live parallel
  changes, GPU smoke/visual tests and display behavior were not requalified.

Receipt logs are retained beside this report. Local candidates are uncommitted;
no push or cleanup was requested in this optimization round.

## Reproduce

```sh
cargo build --release -p voxy_app --bin voxy_app
target/release/voxy_app --cpu-replay /tmp/cpu-motion.csv --cpu-replay-scenario motion
```

Use `idle` for stationary input or `catchup` for the two-tick CPU stress case.
Normal launch behavior is retained. Capture an immutable reference and candidate
binary from identical source/fixture snapshots, then run sequentially after checks:

```sh
python3 tools/performance/paired_cpu_replay.py \
  --before /path/to/reference --after /path/to/candidate --output /tmp/cpu-replay-pairs
python3 tools/performance/paired_cpu_replay.py \
  --before /path/to/reference --after /path/to/candidate \
  --scenarios catchup --output /tmp/cpu-replay-repeat-stress
```

Keep every series; do not overwrite disputed data. The partial-scenario report has
no whole-matrix gate. `combined-summary.json` reports both median run p99 and pooled
p99, plus maxima and threshold counts, so improved medians cannot hide stress tails.
