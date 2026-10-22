# Complete offscreen frame qualification, 2026-10-22

This extends the 2026-10-21 CPU input microbenchmarks with **1920×1080 full owner rendering passes**, GPU completion and offscreen frame cadence on Apple M4 Max. It does not measure display Present/vsync or the complete street_walk gameplay scene.

## Fixed workload and timing

- 4096 vegetation instances: opaque checker geometry, actual botanical shadow-map, shadow receiver and colour/depth pipelines. The correctness fixture's triangle mesh is instanced 4096 times; this is a controlled rendering workload, not a realistic tree-geometry/voxel-world FPS claim.
- 4096 liquid particles: opaque checker geometry, actual screen-space depth/thickness, bilateral filtering and composition. Particle struct generation includes radius computation; physics simulation is outside the frame benchmark.
- Before uses the existing producer Vec → renderer CPU snapshot → shared-ring encode copy. After generates final structs directly into `voxy_gpu_core` shared snapshots. Cameras and all other settings are identical; normal shared camera uploads remain enabled. A/A runs the old slice path on both independent objects.
- Every case checks byte-exact 1080p images at two deterministic animation/particle seeds before timings. A test-only clock freezes wind to the same simulation time on both objects. Production wind clock behavior is unchanged.
- One unmeasured warm-up pair, nine measured alternating AB/BA pairs, four 24-frame blocks per implementation per pair, repeated twice: 18 pairs / 1728 frames per implementation per owner/protocol.
- Queue depth is capped at three using submission-index waits; waits for queue capacity are recorded separately from CPU encode/submit. There is no artificial whole-device drain every three frames. Query resolve/readback occurs only between the 24-frame blocks and is outside frame timestamps.
- Throughput mode submits without start deadlines. Paced mode schedules CPU starts at 120 Hz and records missed completion deadlines. `gpu_gap` is the interval between successive hardware GPU-end timestamps within a block; block boundaries containing query readbacks are excluded. This is **offscreen completion pacing**, not monitor scanout pacing.
- The M4 adapter exposes stage-boundary timestamp queries but not encoder-level timestamps. Matched one-workgroup compute marker passes read the actual colour target and store a checksum, forcing resource dependencies before/after the full render. Independent empty markers could overlap graphics and were discarded during setup. Marker transitions/dispatches are included equally on both paths.
- CPU encode/submit, GPU service timestamps, callback-observed CPU-start-to-GPU-completion latency, submission/start intervals, hardware completion intervals and deadline misses are separate raw trace columns. Host callbacks can coalesce in unpaced mode; hardware timestamps supply the completion cadence. CPU time includes callback registration but excludes queue-capacity waits and pacing sleeps.

## Admission and predeclared criteria

`criteria.json` fixes the sampling and practical noninferiority bounds before qualification: upper 95% paired bootstrap ratio ≤1.05 for median CPU encode/submit, GPU duration, completion latency and completion cadence; paced cadence p99 ≤1.10, and deadline-miss rate increase ≤0.5 percentage points. These bounds do not prove an exact zero-percent difference.

`qualify.py` freezes/checks a test executable digest, requires five consecutive seconds with AGX device utilization ≤10% and no live competing GPU/test executable or active compiler, runs all A/A controls before A/B, and records process/AGX samples throughout. Idle compiler wrappers and the idle parent ollama service are logged without counting as active work. Stopped processes remain visible in provenance. A newly active rival invalidates the attempt; only the controller's own child receives SIGTERM. Post-run GPU drain is checked outside our GPU work. Failed/unstable/contaminated attempts are retained.

The user explicitly authorized temporarily pausing street_walk and then authorized coordination with its owning chat. Initial signals, an intervening external resume, and the coordinated pause acknowledgement are retained in `paused-street-walk.json`, `street-walk-pause-ack.json`, and the observer logs. The paused diagnostic renderer and both controllers were resumed after measurement. The diagnostic repeat overlapping that pause is not performance-qualified.

## Validation evidence

The authoritative live attempt manifest is `session-2/manifest.json`; the controller writes `session-2/status.json` on completion/error. `session-1` was stopped before measurements to correct a false classification of an idle ollama service. Setup capability/marker experiments are in the retained smoke logs and are not qualification results.

The frozen executable passed **335 default renderer tests**, two generated-upload Metal parity tests and three existing camera/vegetation/fluid Metal tests. The full-resolution smoke test passed both owners' exact image oracles. See `default-tests.log`, `generated-metal.log`, `previous-metal.log` and `smoke.log`.

Final accepted paired results and noninferiority decisions are in `session-2/summary.json`; a complete sampling matrix does not by itself establish noninferiority.

## Final result

The sampling matrix is **complete**, but the overall no-regression qualification is **not passed** (`session-2/summary.json`, `qualified=false`). All 16 accepted runs contain exact full-image parity. Ten rejected attempts remain in the manifest, including contention, an unstable A/A control and post-run drain failures. No threshold was relaxed and no additional pairs were selected after seeing the final confidence intervals.

Each row below combines 18 alternating pairs / 1728 frames per implementation. Values are median paired percentage changes (after/before); confidence intervals use 10,000 paired bootstrap resamples. The practical noninferiority bound is +5% for median metrics, +10% for paced GPU cadence p99.

| Path | Protocol | CPU encode/submit | GPU duration | GPU completion latency | GPU completion cadence p99 | Decision |
|---|---|---:|---:|---:|---:|---|
| instances | paced | -15.44% | +0.45% | -1.59% | +1.21% | pass |
| instances | throughput | -28.18% | +3.04% | +2.90% | -2.46% | inconclusive / bound not met |
| particles | paced | -2.99% | -0.09% | -1.13% | -0.95% | inconclusive / bound not met |
| particles | throughput | -4.28% | +0.16% | +0.11% | -0.23% | pass |

- Instances throughput: completion median CI is **−1.61% to +5.06%**, and median hardware completion cadence is **+4.35%**, CI **+1.20% to +5.89%**. The latter is evidence of a small cadence slowdown in this workload, despite substantially lower CPU cost. It must not be reported as universally regression-free.
- Particles paced: GPU median CI is **−6.27% to +5.40%**, crossing the +5% bound. Completion p99 is also imprecise (CI **−1.42% to +24.27%**), even though its point estimate is only +0.73%; no tail-latency improvement is established.
- At 120 Hz, completion cadence p99 changes are +1.21% for instances (CI −0.97% to +2.61%) and −0.95% for particles (CI −1.49% to +0.75%). Completion deadline misses are **0→0 / 1728** for instances and **20→11 / 1728** for particles. These are offscreen deadlines, not presentation guarantees.
- All eight admitted A/A controls met their predeclared per-run stability bounds. Aggregated A/A instances paced still shows CPU CI up to +5.46% and misses 0→9; this exposes environmental/scheduling noise. A/A stability admission is not the same as demonstrating zero measurement error.

Correctness: 335 default tests passed; two generated-upload and three previous upload Metal tests passed; both full-resolution smoke tests passed. All timed runs passed their exact before/after image oracle and GPU validation scopes. Queue-depth-three completion tracking exercises retirement while multiple submissions are outstanding. This qualification does not replace separate lifecycle tests for canceled encoders/fallback uploads, and it makes no whole-game or display FPS claim.

### Quiet window and release

User authorized the street_walk pause, cross-chat coordination, then a common build/benchmark pause and an idle UI window. Coordination acknowledgements, observer samples and exact PID/start/command signal identities are retained. Automatic build restarts and an automatic safety-deadline resume contaminated several attempts; these were terminated by the controller and excluded. The controller was resumed with the same frozen executable and preserved accepted runs. The stale terminal status from its first timeout was replaced with running status during resume, then final complete status; it never signified a complete qualification before all 16 runs.

The temporary build holders are preserved as `voxy-hold-build-window*.py` for signal audit only; they are not invoked by the normal reproducer. All recorded still-matching stopped build processes received SIGCONT on release; `resume-verification.json` found none left stopped. The two street_walk controllers, renderer and preview pair were identity-checked and resumed (`resumed-street-walk.json`). Six owning chats received an explicit release message. Pause-overlapping timing from those other jobs is contaminated; their files were preserved.

The executable SHA256 is `e502004916c4392eca3f1b7f84eeb580c894df1ee189fea56522d0c5db97628d`. `source-hashes.json` freezes reviewed sources; `source-drift.json` records any subsequent workspace drift without changing that provenance. The old and direct paths were measured in the same frozen executable. Current workspace changes from other tasks are not automatically covered by these measurements.

## Reproduction

```sh
cargo test --release -p voxy_render --lib --no-run
# Freeze a copy of the executable printed by Cargo, then use a NEW output path:
python3 docs/architecture-performance/2026-10-22-full-frame-shared-generation/qualify.py \
  --binary /absolute/path/to/frozen-test-bench --output /absolute/path/to/new-session
python3 docs/architecture-performance/2026-10-22-full-frame-shared-generation/summarize.py \
  /absolute/path/to/new-session
```

No game window is opened. No production upload implementation is changed in this qualification; additions are the test harness, full-frame fixtures and deterministic test-only wind clock.

Post-run tooling correction: `qualify.py` now writes `status=running` immediately on entry/resume, preventing peers from observing an old timeout status while a resumed controller is active. Sampling, admission and statistical criteria are unchanged; this post-run script digest difference is retained in `source-drift.json`. The other drift is concurrent edits to `lib.rs`; benchmark helper, both fixtures and upload implementations retain their frozen source hashes. Python sources parse successfully, Rust formatting checks pass with edition 2024, and `git diff --check` passes.
