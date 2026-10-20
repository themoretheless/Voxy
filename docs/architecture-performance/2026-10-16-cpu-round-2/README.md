# CPU simulation and pose admission, round 2 (16 October 2026)

## Findings and retained changes

1. `Companion::tick_with_bodies` deeply cloned the whole companion and coat on
   every tick to keep failures atomic. It now reuses one private boxed candidate,
   copies the current state into its existing storage, and swaps the complete
   candidate into place only on success. Failed candidates are restored from the
   published state before reuse. `Companion::clone` intentionally excludes this
   work cache; `CoatDynamics::clone_from` and `Strand::clone_from` reuse nested Vec
   allocations while copying all simulation/history/configuration fields.
   This retains one extra candidate coat per animal in exchange for eliminating
   repeated deep-allocation churn. It does not make every part of the tick
   allocation-free and does not change physics frequency or solver iterations.
2. Mixed companion geometry narrowly missed the existing 50% adjacent-influence
   threshold, so pose publication repeatedly rebuilt identical deformation and
   normal transforms. Cache selection now starts at 25% identical adjacent
   influences. The constructor still computes this once from immutable vertices;
   shared-index/LOD views retain the classification. The original plain path is
   retained below that threshold. Per-vertex admission, numeric checks, error
   order and the existing parallel chunking remain unchanged by this patch.
   Concurrent work later added influence-group tables for large meshes; that path
   is preserved and is outside these measurements. The retained pose result
   covers the non-grouped 100,000-vertex path.

## Controlled CPU comparisons

Apple M4 Max. Two complete same-process series, five alternating before/after
pairs each: **all ten pairs retained**, including noisy tails from the first
series. Each operation is measured on the same thread and compiled fixture.
No window or GPU device is created by these benchmarks.

| Operation | Before p50 | After p50 | Ratio |
| --- | ---: | ---: | ---: |
| Real dog tick: collision, gait, fur, atomic publication | 19.313 µs | 14.583 µs | 0.755 |
| Real cat tick: collision, gait, fur, atomic publication | 18.959 µs | 14.354 µs | 0.757 |
| Admit 100,000 vertices with 25% adjacent repeated influences | 2196.104 µs | 1738.417 µs | 0.792 |
| Unchanged control: highly coherent influences | 489.126 µs | 490.230 µs | 1.002 |
| Unchanged control: alternating influences | 2198.105 µs | 2194.896 µs | 0.999 |

Numbers are the median of per-run CPU p50, **not application frame p99**. Each
animal phase executes 220 fixed ticks, excluding the first 20; its before path
uses the original clone-per-tick transaction wrapper around the same `tick_inner`.
Each phase starts from the same seeded world/spawn and uses the same deterministic target rule.
All ten final-state checksums per variant match for each species. The pose kernel
uses 31 samples after three warmups per phase, identical palettes/vertices, and
compares the old 50% policy with the new policy in the same compiled production
`admit_render_vertices` function. The synthetic pose test measures sequential
admission; large-mesh worker scheduling is not requalified here.

Median run CPU-tick p99 also decreased when both series are combined:

| Operation | Before run p99 | After run p99 |
| --- | ---: | ---: |
| Dog | 24.167 µs | 17.209 µs |
| Cat | 21.438 µs | 16.875 µs |

The first dog series had an adverse noisy tail, so the entire series was repeated
and both sets were retained rather than selecting a favorable subset. These tail
metrics are medians of per-run p99 values, not a pooled percentile.

Raw logs, per-pair CSV, executable hashes and per-series summaries are here and in
`repeat/`; `combined-pairs.csv` and `combined-summary.json` combine both series.
`paired/` is the earlier cross-process attempt. It is explicitly **non-qualifying**:
background compilers moved even unchanged pose controls by large amounts. Its
apparent speedups are excluded from the table above. Live model geometry also
changed during this session, so old/new application binaries were not compared
on different models. The retained tick comparison shares one compiled fixture.

## Validation and limits

- Animal/gait/fur suites: 9 passed, 1 manual benchmark ignored (another filtered).
  New regression compares every committed state/pose with clone-per-tick ticks,
  injects a failure after mutating the coat and phase, verifies rollback, then
  checks five further ticks for matching history. Cloned snapshots exclude caches.
- Strand-related library suites: 21 passed, 3 ignored. New regression verifies
  reuse of all five strand allocations and matching subsequent velocity history.
- Strand integration: 3 passed, including sphere collisions and atomic rejection.
- Final skinned/LOD suites: 14 passed, 3 ignored (including concurrent grouped-path
  regression). Exact-normal and error-order oracle
  tests now include mixed 25% layouts as well as coherent and plain paths.
- Main-client compile check passes; five Python report/comparison tests pass.
  Strict library Clippy still fails on 14 existing renderer diagnostics, with none
  pointing to the changed pose/strand files. No full workspace/visual/FPS qualification is
  claimed; this round makes no claim about GPU uploads or the 120 FPS goal.

Build CPU test executables, save their returned paths, then reproduce the paired
benchmarks (the two manual tests create no GPU/window):

```sh
cargo test --release -p voxy_app --bin voxy_app --no-run
cargo test --release -p voxy_render --lib --no-run
python3 tools/performance/paired_cpu_same_process.py --app-tests /path/to/voxy_app-test-executable --render-tests /path/to/voxy_render-test-executable --output /tmp/voxy-cpu-pairs
```

The earlier `paired_companion_cpu.py` runner remains available for cross-process
observations; use the same-process runner above for the retained results.
