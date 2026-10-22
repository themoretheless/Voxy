# Render-pose CPU versus waiting — 22 October 2026

## Retained change and measured cause

**The CPU-replay pooled-p99 regression is removed in all three measured scenarios.**
An initial five alternating AB/BA pairs per scenario and full repeats of the
disputed idle/stress series pass both the median-run and pooled-p99 gates. All
50 phases are included (10 idle pairs, 5 motion pairs, 10 stress pairs). No window, event loop, surface, GPU device or GPU workload was
created by this investigation. Physical displayed-frame/120-FPS qualification
remains outside these CPU-only results.

The new [clock interface](profiling-interface.md) measures scheduled caller CPU,
process CPU including workers, caller/worker computation, launch latency and
spawn/join wall versus CPU separately. A fresh, natively lifecycle-monitored stress
diagnostic pair found 518,549 immutable distinct influence groups. Every baseline
frame certified all of them using a 16-thread budget; no exact vertex fallback ran.

| Pose admission diagnostic, medians | Before | After |
| --- | ---: | ---: |
| elapsed wall time | 8.388 ms | 0.048 ms |
| scheduled process CPU (all workers) | 23.319 ms | 0.048 ms |
| scheduled caller CPU | 1.594 ms | 0.048 ms |
| join wall time | 4.980 ms | 0 |
| caller CPU inside join | 0.112 ms | 0 |
| max launch-request-to-worker-entry delay | 4.094 ms | 0 |

For the baseline's three worst elapsed frames, median pose wall time was 27.628 ms,
join wall 26.275 ms, join CPU only 0.128 ms, and process CPU 24.546 ms. Thus the
large tail is predominantly off-CPU time at the worker lifecycle/join boundary,
not a matching rise in arithmetic. Join clocks cannot distinguish kernel thread
exit, wakeup delay and caller descheduling; no narrower scheduler attribution is
claimed. In an earlier diagnostic the off-CPU gap occurred inside caller work
instead, also preserved as `baseline-catchup.*`.

The excessive arithmetic is repeated normal/position certification for distinct
weights sharing the same active joint family. The retained implementation proves
a conservative interval certificate for **every normalized convex blend** within
such a family. When all families are certified, normal admission requires no
worker creation, joining or per-weight scan. Metadata is immutable, bounded to
1024 families, and disabled unless it amortizes at least 32 influence groups per
family. Uncertain bounds, extreme matrices/positions, bad authored normals or
diverse meshes retain the original group certificate and exact vertex-order
fallback. Current matrices and palettes are checked anew every call.

[The sufficient-bound argument and oracle coverage](family-certificate.md) explain
rounding, underflow and determinant margins. No vertex positions, normals,
palette data, physics steps, asset formats or GPU writes are approximated or
reordered. In the monitored optimized diagnostic, 239/240 poses avoided workers;
one uncertain pose retained the original path. No full vertex scan ran. The
fallback is intentional and remains capable of producing occasional long frames.

## Uninstrumented paired qualification

The copied frozen executables use the same source snapshot, with the before binary
captured after adding clocks and before the family optimization. Both qualification
variants call the ordinary admission entry point with clocks disabled. Seed 42,
1,990,328 vertices, fixed input and synthetic schedule, 120 warmup frames and 240
measured frames per phase. Geometry, indices and cumulative state checksums match
across all 50 phases. The first series had one adverse idle pair and one
adverse stress pair; both entire scenarios were repeated without discarding any
initial phase. `summarize_repeated.py` validates and combines the complete series.
No phase was dropped; every observed process lifecycle was
valid with no stop/continue events. Raw CSV/JSON, metadata and binary SHA-256 hashes
are preserved under `paired/` and `environment.json`.

| Scenario | Pairs | Median run p99, ms | Pooled p99, ms | Pooled change | Frames >8.33 ms | After max, ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| idle | 10 | 13.001 → 0.489 | 20.733 → 0.514 | -97.5% | 274 → 5 | 234.778 |
| motion | 5 | 35.724 → 0.921 | 61.448 → 2.058 | -96.7% | 221 → 2 | 10.223 |
| catchup | 10 | 10.565 → 1.065 | 28.627 → 4.357 | -84.8% | 249 → 14 | 51.151 |

Idle and stress pools contain 2400 frames per variant; motion pools contain 1200.
The combined stress pooled p99 improves 84.8%; all control p99 values improve.
Across all cases, frames >8.33 ms decrease from 744 to 21 out of 6000 per variant.
`combined-summary.json` includes all-series p50/p95/p99, maxima and >16.67 ms
counts. `pooled-details.json` preserves the initial matrix's additional stage
statistics; every phase in both series retains CPU-load/lifecycle metadata.

**Not all stalls are solved.** The worst optimized idle frame was 234.778 ms in
companion simulation, worse than that scenario's baseline maximum of 77.867 ms.
Another optimized idle frame spent 109.691 ms in render admission. All are included
in pooled calculations and maxima; they were not filtered. Strong concurrent
external activity was present (one-minute load at phase starts approximately
22.3–220.6 on 16 logical CPUs). The pooled result qualifies this
CPU replay under that load; it is not an uncontended system or displayed-frame
qualification. The historical +8.7% pooled difference cannot be uniquely attributed
post hoc because those old phases lack per-thread CPU and lifecycle records. The
current costly path and its waiting tail were reproduced, measured and removed
from certified poses, rather than inferred solely from historical aggregate p99.

## Verification and preservation

- Final CPU renderer/skinned suite: 24 passed, 4 ignored. Includes all 65,536
  two-joint weight pairs, four influences, cancellation, extreme scales, reflected /
  singular matrices, shared-index views, changed poses, exact error ordering and
  disabled-family-cache fallback. Clock test: 1 passed.
- Collector/lifecycle suite: 15 passed. Stopped samples are invalidated and killed
  without automatic resume; complete CSV cannot override invalid lifecycle data.
- Release client build and release workspace check passed on the frozen snapshot.
- Strict renderer Clippy remains blocked by six pre-existing `dead_code` errors in
  `world_nearest_wide.rs`; Clippy with only that category allowed passed. No unrelated
  renderer experiment was edited to satisfy this check.
- Changed Rust sources pass rustfmt checking. Full workspace tests, GPU smoke and
  image comparisons were not run under the user's CPU-only constraint.
- `implementation.patch` isolates this turn's skinned changes from prior dirty work;
  the qualified source manifest and binary hashes retain the frozen comparison.
  Owned implementation files still match the qualified snapshot. Other live edits
  were preserved. No commit/push or unrelated process suspension was performed.

The external shared-build controller paused this task's build, then resumed it;
owned build signal provenance is retained separately. No paused build interval was
counted as benchmark timing. Managed replay diagnostics and qualification observed
no process stops. Initial unmonitored diagnostics are explicitly supplemental.
