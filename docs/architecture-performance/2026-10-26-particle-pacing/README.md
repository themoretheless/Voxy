# Particle 120 Hz frame-pacing investigation, 2026-10-26

Fixed overdue-frame catch-up and bounded the paced queue in the **offscreen measurement harness**. Particle shaders, production particle rendering and shared-memory ownership are unchanged. The final upload A/B meets the p99 ratio bound: **1.00181, 95% CI 0.99568–1.00770**, below 1.10. One A/A control fails its p95 stability check, so strict qualification remains **false**. This is not an unconditional application 120-FPS claim.

## Cause and repair

The old scheduler targets `block_start + frame_index * period` even after a long delay; every overdue target then runs immediately, permitting catch-up bursts with three outstanding submissions. Previous particle traces contain **542 before / 446 after** CPU start gaps shorter than half a 120 Hz period, interleaved with **144 / 130** gaps longer than two periods.

The largest prior completion gap was 392.867 ms, alongside 350.835 ms CPU encode/submit and 151.751 ms GPU service. Across the 67 largest gaps, median GPU service was 1.661 ms versus 1.390 ms overall; 31 overlap a host-delay metric dominating the gap, and 28 have GPU service over four times the median. These categories overlap. Evidence supports both host and GPU contention and a confirmed scheduler catch-up defect; it does not prove a particle-kernel-specific deterministic stall. `prior-tail-analysis.json` retains the full diagnostic values.

Final repair: retain the original target grid for ordinary lateness. Rebase the next target to one period after the actual start **only when its existing target would leave less than half a period**. This prevents an overdue burst without accumulating small wake-up delays into a lower nominal rate. Paced mode limits outstanding submissions to one, waiting for that exact submission index; throughput mode retains depth three. A deterministic regression covers a 12-period interruption, normal operation, small lateness with no grid drift, and a nearly overdue next target.

Raw GPU-end gaps remain unfiltered, unclipped and unnormalized. Original fixed-grid deadline misses remain in `FULL` rows. Extra `PACE` rows retain assigned deadlines, lateness, queue bounds and assigned-deadline misses. `analyze.py` also measures **scheduled-target-to-completion latency**, including queue-capacity waits, so reducing queue depth cannot silently hide wait time outside the existing CPU-start timer.

This fixes the scheduling of the benchmark. It cannot guarantee the removal of OS preemption or other GPU workloads from a production application.

## Fixed experiment and correctness

Clean checkout of a9a488cd9 received only `full_frame_upload_bench.rs`. Frozen final binary SHA256: `faa40b0726c3283bbb05d7e0b168f634bdd4f621c9a8aaf634315e17524c16c3`.

**349 default tests passed**, including the scheduling regression; both physical fluid generated/snapshot tests passed, exercising data parity, queued consumers and overflow; both full-frame image-oracle smoke tests passed. Every timed run validates byte-exact 1920×1080 images at two seeds and GPU error scopes. No game window was opened. Rust formatting, Python AST parsing and diff whitespace checks pass.

Same complete optical depth/thickness/filter/composition passes over an opaque scene, 4096 particles, unchanged camera uploads and deterministic fixture data. Six runs in each candidate matrix, under observed concurrent load with **no idle admission, task pauses, noisy-sample rejection or optional stopping**:

1. Control: identical slice uploads and bounded pacing on both sides.
2. Pacer: identical slice uploads; legacy pacing before, bounded pacing after. This isolates scheduling changes.
3. Upload: bounded pacing both sides; slice input before, direct shared generation after.

Each group: two repeats × 18 alternating pairs × two 48-frame blocks = **36 pairs / 3456 frames per implementation**, plus unmeasured warm-up. Bootstrap: 10,000 resamples of paired metric ratios, deterministic seed 20261026, two-sided 95% interval. Median metric upper bounds +5%, cadence p99 upper bound +10%. `criteria-final.json` fixes the final candidate before its matrix; different binaries/matrices are not pooled.

## Final results under observed load

Cells show paired median ratio changes and the two-sided 95% ratio interval.

| Comparison | GPU service median | Completion median | Completion cadence median | Completion cadence p99 |
|---|---|---|---|---|
| control | -0.07% [0.99131, 1.00582] | -0.93% [0.96605, 1.01124] | -0.10% [0.99721, 1.00071] | -1.76% [0.88705, 1.01822] |
| pacer | -0.51% [0.98549, 1.00399] | -0.08% [0.99238, 1.00774] | +0.01% [0.99939, 1.00083] | +0.10% [0.99192, 1.00541] |
| upload | -0.26% [0.97901, 1.00675] | -0.48% [0.98801, 1.00445] | -0.01% [0.99888, 1.00032] | +0.18% [0.99568, 1.00770] |

The isolated pacing comparison removes **7→0** catch-up starts; minimum observed start interval changes **0.272→6.224 ms**. Median completion cadence ratio is **1.00014**, CI **0.99939–1.00083**, so the provisional candidate's ordinary-lateness drift is eliminated. Pacing p99 ratio is **1.00104**, CI **0.99192–1.00541**: no material increase in this comparison, but no demonstrated absolute reduction either.

For direct shared particle uploads, p99 hardware completion cadence is **8.950→8.943 ms** (medians of per-pair p99 values); paired ratio CI upper **+0.77%**, below +10%. All upload median ratio bounds also pass. Both implementations have zero catch-up starts. Assigned-deadline misses are **26→9 / 3456**; original fixed-grid misses are **186→40 / 3456**. Other raw tails and end-to-end scheduled latency remain available; no missed deadline was erased by the new schedule.

**Control limitation:** control-r1 cadence p95 ratio is 0.94033, outside the ±5% A/A stability band; control-r2 passes. Aggregate control p99 CI upper is +1.82%, but this does not retroactively waive the failed per-run control. `schedule-validation.json` therefore records `relative_p99_qualified=false` even though the observed upload p99 bound passes. No additional attempts were selected to replace this noisy control. General load-independent non-regression and absolute 120 Hz deadlines remain unproven.

### Actual deadline/queue telemetry

| Group | Mode | Catch-up starts | Assigned-deadline misses | Original-grid misses | Scheduled-to-completion p99, ms |
|---|---|---:|---:|---:|---:|
| control | before | 0 | 558 | 1312 | 50.134 |
| control | after | 0 | 471 | 1404 | 39.080 |
| pacer | before | 7 | 22 | 22 | 3.442 |
| pacer | after | 0 | 10 | 51 | 3.409 |
| upload | before | 0 | 26 | 186 | 4.696 |
| upload | after | 0 | 9 | 40 | 3.304 |

## Retained provisional candidate

`session` and `relative-scheduler-v1.rs` retain the first full matrix. That version rebased every frame to its actual start. It removed bursts but accumulated normal wake-up lateness, producing **+2.19% median completion intervals** despite passing its relative +5% bound. It was rejected for that deterministic schedule drift. Final v2 retains the grid for small delays; `session-final` is a completely new fixed matrix. No data was dropped or pooled to obtain a better interval. Both versions and their control failures remain inspectable.

## Reproduction

Build a release library-test executable and freeze a copy. The executable contains both schedulers and both upload paths:

```sh
cargo test --release -p voxy_render --lib --no-run
python3 docs/architecture-performance/2026-10-26-particle-pacing/measure.py \
  --binary /absolute/path/to/frozen-test-executable --output /absolute/path/to/new-session
python3 docs/architecture-performance/2026-10-26-particle-pacing/analyze.py /absolute/path/to/new-session
```

`measure.py --output PATH --summarize-only` recomputes the bootstrap. Plain and gzip frame traces are both supported. Frame and client traces are compressed losslessly; recomputation is checked against the original summary. Source digests, build logs, validation logs and raw process/AGX observations are retained. This experiment does not wait for a free GPU or pause other work.

PACE schema: `PACE,path,phase,pair,block,mode,frame,queue_depth,assigned_target_us,start_lateness_us,assigned_deadline_miss`. Absolute microseconds share the FULL host epoch; GPU timestamps remain a separate hardware clock.
