# Direct shared generation follow-up, 2026-10-25

Cyclic shared-slot selection is implemented and correctness checks pass. The complete **observed-load** A/A + A/B matrix is saved in `session-observed-load`: instances pass both protocol gates; particles at 120 Hz pass the GPU median gate that was previously uncertain, but their p99 completion-cadence interval remains too wide. The overall performance qualification remains **false**. This is not a claim that every regression criterion has closed.

The user explicitly requested proceeding and pushing **without requiring a free GPU window**. `--allow-contention` therefore records concurrent clients and AGX samples without waiting for idle or terminating attempts on competition. It does not change workloads, fixed sample count, noninferiority thresholds, or correctness checks. Completed runs in that mode are not isolation-qualified runs. No other task was paused for this final matrix.

## Implementation and correctness

`voxy_gpu_core::SharedUploadRing::prepare` starts each generated-snapshot search at a cyclic slot, scanning all three resident allocations before using staging overflow. This distributes successive CPU writes and GPU fetches rather than always preferring the first ready allocation. Atomic READY acquisition still prevents reuse of CPU/GPU-retained snapshots. Cancellation, producer failure/unwind recovery, completion retention, Apple unified-memory admission, unsupported-device fallback and GPU allocation accounting remain unchanged. Packing/bindings remain in `voxy_render`.

This was an experimentally motivated candidate; the measurements compare the old producer/snapshot path against the new cyclic direct-generation path, **not** first-ready direct generation against cyclic direct generation in isolation. They do not prove a cache-coherence mechanism for the original slowdown.

Validation on the frozen candidate: **352 renderer + 31 core default tests passed**; three physical core Metal tests passed, including 12 cyclic allocation identities with byte readback, multiple consumers, abandoned encoders, overflow, failed/panicking producers and managed ownership. Six generated renderer tests and both full-resolution smoke tests passed. Every completed timed run passed its byte-exact 1080p image oracle and GPU validation scopes. Rust formatting, Python AST parsing and `git diff --check` passed.

## Fixed paired protocol

Same 1920×1080 production owner passes and 4096 dynamic inputs as 2026-10-22: botanical shadow map/receiver/colour/depth over opaque checker geometry, or liquid depth/thickness/bilateral/composition. Instance fixture uses a triangle mesh per instance; this is not the complete gameplay scene. Same shared camera uploads on both paths. Baseline materializes producer Vec and renderer snapshot; candidate generates final structs into shared snapshots.

Two repeats × 18 alternating pairs × two 48-frame blocks per implementation: **36 pairs, 3456 frames per implementation per owner/protocol**. Eight A/A controls precede eight A/B runs; all eight controls passed their fixed per-run stability bounds. No noisy run was removed or additional samples selected after seeing the final intervals. Original strict session is separate and is not pooled.

Queue depth ≤3 uses specific-submission waits, recording backpressure separately. Matched colour-dependent compute markers bracket production passes, giving GPU service timestamps; callbacks provide CPU-start-to-completion latency. Hardware completion gaps provide offscreen cadence. Throughput and 120 Hz scheduled starts are separate protocols. Query readback occurs between blocks outside frame timestamps. This is not display Present/vsync pacing or a complete street_walk FPS test.

Median practical noninferiority upper 95% CI ≤1.05; paced hardware completion-cadence p99 upper CI ≤1.10; completion-deadline miss increase ≤0.5 percentage points. Deterministic 10,000-resample bootstrap of the 36 paired ratios. These criteria are preserved from the previous gate; workload sampling was expanded before the confirmation series.

## Observed-load results

Cells show median paired percentage change (after/before), not the ratio of independently pooled medians.

| Path | Protocol | CPU encode/submit | GPU service | GPU completion latency | GPU cadence median | GPU cadence p99 | Gates |
|---|---|---:|---:|---:|---:|---:|---|
| instances | paced | -15.50% | +1.47% | -5.93% | -0.11% | -1.29% | pass under observed load |
| instances | throughput | -26.63% | -4.56% | -9.47% | -9.09% | -22.74% | pass under observed load |
| particles | paced | -1.24% | -0.21% | -8.17% | +0.56% | +0.81% | not all passed |
| particles | throughput | -1.45% | +2.46% | -0.03% | +2.54% | +11.00% | not all passed |

- Instances throughput: median cadence **−9.09%**, CI **−12.20% to −3.79%**; completion median **−9.47%**, CI **−18.53% to −3.50%**. The prior +4.35% median-cadence slowdown is not reproduced in this confirmation. Unpaced p99 remains noisy (upper ratio CI 1.533) and was never a gating metric.
- Instances 120 Hz: GPU median CI upper +2.12%, completion median upper −2.11%, cadence p99 upper +0.65%; all gates passed. Completion deadline misses 927→821 / 3456.
- Particles 120 Hz: **GPU median ratio 0.9979, CI 0.9813–1.0126**, closing the old GPU-median noninferiority uncertainty in this observed-load series. Completion median improved 8.17%, CI −17.46% to −3.07%. However cadence p99 ratio 1.0081 has CI **0.8750–1.1608**, failing the +10% tail bound. Completion deadline misses 1385→1181 / 3456. This is not stable absolute 120 FPS: background load produced many missed deadlines.
- Particles throughput: GPU/completion/cadence median confidence intervals are too wide for the +5% bounds; no universal regression-free claim is made for that mode either.

All observations, controls and failed bounds are retained. Passing A/A medians does not eliminate the effects of changing concurrent load; the large AB tail/deadline values show that limitation directly. Overall `qualified=false` is intentional, despite the instance gates and the particle paced GPU-median gate passing.

## Provenance and earlier strict attempt

Frozen candidate executable SHA256: `6538376c163bd5b4785173e1d5a28b84e99362e853820c12c733d2271fdabfb9`; same executable contains baseline and candidate. `source-hashes.json` captures measured Rust sources; `delivery-source-hashes.json` captures delivered scripts and sources after adding the user-authorized observed-load mode. `shared-upload-before.rs`, `shared-upload-candidate.rs` and `candidate.patch` preserve the implementation comparison.

`session-1` is the earlier strict attempt: one accepted A/A and three rejected post-drain attempts, followed by idle-admission timeout. Candidate was temporarily removed pending qualification, then re-enabled at the user's explicit instruction to proceed/push without the free-window requirement. No samples from that session enter the final summary.

Before that instruction, an authorized bounded quiet-window holder and owning-task watchdogs recorded exact SIGSTOP/SIGCONT identities. All own pauses were released and verification found no matching stopped process. The owning GPU task independently verified its Metal compiler through its launchctl pid-domain; ack/signal records are retained. `hold-window.py` is audit evidence only, not an implicit reproduction dependency. All timing spanning those pauses in other tasks is contaminated; their source/data were preserved. The final observed-load series used no pauses.

Process/AGX observer streams and client snapshots are stored losslessly as `.jsonl.gz` / `-clients.json.gz` to keep the repository small. Frame traces are also losslessly gzip-compressed; `qualify.py::parse` accepts both plain and compressed traces. Recomputing the complete summary from compressed traces reproduced every JSON field exactly.

## Reproduction

Build the test executable, freeze a copy, then choose a new output directory:

```sh
cargo test --release -p voxy_render --lib --no-run
python3 docs/architecture-performance/2026-10-25-direct-shared-followup/qualify.py \
  --binary /absolute/path/to/frozen-test-executable \
  --output /absolute/path/to/new-session --allow-contention
python3 docs/architecture-performance/2026-10-25-direct-shared-followup/summarize.py \
  /absolute/path/to/new-session
```

Omit `--allow-contention` to require the strict idle/competition gate. Never pool different binary digests. Raw sample counts and validation errors fail closed; correctness failures are not waived by the contention flag.

## Clean delivery validation

A detached checkout of `0a318afc2` received only the two Rust files in this delivery. Its release build passed, followed by **348 renderer + 31 core default tests**, **3 core Metal tests**, **6 generated renderer tests**, and **2 full-frame smoke image-oracle tests**. See `clean-candidate-build.log`, `clean-*-tests.log`, `clean-smoke.log`, and `clean-validation.json` for executable digests. Thus compilation/correctness of the delivered changes does not depend on other tasks' uncommitted edits. The longer paired performance series used the separately frozen shared-workspace executable whose hash and source inventory are recorded above; it is not the same binary as the clean delivery test build.
