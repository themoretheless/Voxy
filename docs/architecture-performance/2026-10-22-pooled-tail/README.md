# Pooled CPU tail investigation — 22 October 2026

> Follow-up: the [CPU/wait investigation](../2026-10-22-pose-cpu-wait/README.md)
> subsequently retained a verified family certificate and reduced pooled p99.
> Results below describe the earlier rejected candidates and remain unchanged.

## Outcome

**The stress pooled-p99 regression is not fixed or performance-qualified.** No
renderer optimization from this investigation is retained. The confirmed reporting
bug is fixed: the paired collector now requires **both** median run p99 and pooled
p99 to pass the target gain (5%) and control nonregression (3%) thresholds. Previously
its `cpu_gate` checked only the median of run p99 values. A regression test covers
rare slow runs which leave that median improved while worsening pooled p99.

The old 10-pair stress result was 11.861125 → 12.887875 ms pooled p99 (+8.7%).
Reanalysis of every retained frame attributes a median 96.7% of the optimized
variant's worst 1% frames to render-pose admission. Temporal admission is already
cheap. The 76 frames over 8.33 ms occur at 72 distinct trajectory indices across
10 identical runs, with at most two occurrences at any one index. The largest
per-index median is 1.959 ms. This localizes a non-repeatable elapsed-time tail,
not a reproducible bad pose. It **does not prove** whether scheduling, thread
creation/join, or other system activity caused the original regression.

## CPU-only experiments

Fresh source snapshot; copied immutable executables; no event loop, window, surface
or GPU created. Same production CPU replay, seed 42, 1,990,328 vertices, 120 warmup
frames and 240 measured frames per phase. All completed before/after fixture and
state checksums agree. The replay scope/exclusions remain those of the
[previous CPU report](../2026-10-21-cpu-replay/README.md).

1. Four scoped workers: rejected diagnostic. Completed pairs and partial third
   round retained under `cap4/`. Catchup round 2 after reached 386.315 ms in render
   admission. The owning agent intentionally sent SIGTERM to its own process group
   84924 after severe regressions; no remaining phase was counted as complete.
2. Sequential group certification: rejected single diagnostic, not qualification.
   Render median 35.670 ms and p99 118.286 ms (`serial-diagnostic.csv/json`).
3. Dynamic blocks of 256 groups, same scoped worker budget: complete five alternating
   AB/BA pairs for each scenario, retained under `dynamic/`. Exact group predicates,
   palette checks and original per-vertex fallback remain unchanged. Rejected.

| Scenario | Median run p99 before → after, ms | Pooled p99 before → after, ms | Pooled change |
| --- | ---: | ---: | ---: |
| idle | 6.956 → 2.554 | 14.172 → 12.689 | −10.5% |
| motion | 4.338 → 2.553 | 13.171 → 18.128 | +37.6% |
| catchup | 4.281 → 3.341 | 17.266 → 109.035 | +531.5% |

The dynamic candidate would pass the old median-only gate despite these tails.
The new gate rejects it. These values describe this CPU replay under contention;
they are neither physical GPU frame times nor a causal ranking of the candidates.

Concurrent external Rust builds, Git operations and other clients were active;
system load averages were roughly 34–47 on 16 logical CPUs. See sanitized process
and load captures. No unrelated process was stopped or reprioritized. Own builds
and correctness tests ran outside timed comparisons. The strongly varying phase
results cannot establish a clean ≥5% optimization or isolate the historical
scheduler cause. An uncontended CPU control interval and process/worker CPU versus
elapsed-time instrumentation are required before claiming that cause is eliminated.
Reducing worker count alone and changing static ranges to a shared queue are not
validated fixes.

## Validation and evidence

- Collector suite: 11 passed (`collector-tests.log`), including the hidden pooled
  regression case. All existing CSV, percentile, geometry and state guards remain.
- Rejected dynamic candidate: 21 CPU renderer/skinned tests passed, 4 ignored;
  new oracle cases cover block boundaries 255/256, 511/512, final group 1024 and
  worker budgets 1/3/8/16 (`candidate-renderer-tests.log`). No GPU tests launched.
- Release client builds succeeded for the frozen baseline and diagnostic candidates.
- Raw CSV/JSON, per-pair ratios, immutable binary hashes and complete dynamic
  summary retained. Source manifests distinguish measured source from added tests.
- Candidate patches are evidence only, not applied to production. The working
  renderer file still matches the captured baseline SHA-256.
- Full workspace tests/Clippy and 120-FPS GPU qualification were not rerun for this
  collector-only retained change. No commit or push performed.
