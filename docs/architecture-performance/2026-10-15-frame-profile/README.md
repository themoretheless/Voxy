# Native frame profiling

Both native clients accept `--profile PATH.csv`, `--profile-scenario NAME`,
`--profile-warmup SECONDS` (default 10), and `--profile-seconds SECONDS` (default 60).
The main app supports `idle`, `motion`, and `water`; `street_walk` supports `street`.
Profiling uses a physical 1920×1080 window and automatic fixed-tick input. Normal
launch settings are unchanged. Water releases a natural support block at tick 720
and fires periodically. The street follows a repeating 20-second forward/back path
with its fixed default seed; this is traversal of the resident scene, **not an asset
streaming benchmark**. It freezes the day cycle. Keep quality settings/environment
identical between versions.

```sh
cargo build --release -p voxy_app --bin voxy_app --example street_walk
target/release/voxy_app --backend metal --present-mode immediate --profile /tmp/idle.csv --profile-scenario idle
target/release/voxy_app --backend metal --present-mode immediate --profile /tmp/motion.csv --profile-scenario motion
target/release/voxy_app --backend metal --present-mode immediate --profile /tmp/water.csv --profile-scenario water
target/release/examples/street_walk --profile /tmp/street.csv --profile-scenario street
```

Reports contain attempted-frame intervals, CPU timings, surface acquisition,
presentation/skip observations and nearest-rank p50/p95/p99. A JSON summary sits
beside the CSV. A separate `.gpu.csv` records asynchronously collected GPU samples;
these belong to earlier submissions, not the CPU frame collecting them. Main-app
GPU measurements cover the primary voxel pass only. Street measurements cover the
existing scene timestamp span plus opaque/transparent passes; external postprocessing
is excluded. Unsupported metrics remain empty/null. Upload byte counts, peak memory,
full GPU frame time, display scanout and actual missed refreshes are not measured.
Main-app simulation time includes pose publication and its GPU upload calls.

Warmup starts after initialization; intervals crossing its boundary are excluded.
Closing the window early fails the profiling run. Surface skips are included and
prevent qualification. Keep the window visible, disable capture/validation modes,
and do not run benchmarks concurrently. A software adapter cannot qualify hardware
performance. `frame_budget_met` only describes measured CPU frame intervals; it is
not complete 120 FPS qualification.

## Paired comparisons

Save separate instrumented executables for both versions, then run:

```sh
python3 tools/performance/paired_frames.py --before /tmp/before/voxy_app --after /tmp/after/voxy_app --street-before /tmp/before/street_walk --street-after /tmp/after/street_walk --focus water --output /tmp/voxy-pairs
```

The runner executes five alternating before/after pairs for each scenario, hashes
executables, retains raw reports/logs, and compares median run p99. Acceptance needs
at least 5% improvement in the chosen scenario and no regression above 3% elsewhere.
The complete matrix takes about 47 minutes plus initialization. A partial/missing
report, process failure or timeout stops the comparison.

No performance optimization is qualified merely by adding this harness. Large-scene
streaming, bounded incremental chunk publication, full GPU pass coverage, memory
telemetry and visual regression captures still require separate implementation and
measurements before claiming the complete 120 FPS plan achieved.

## Observed baseline and verification (15 October 2026)

One complete `motion` run on Apple M4 Max / Metal, 10 s warmup + 60 s measurement:

| Metric | p50 | p99 |
| --- | ---: | ---: |
| Attempted frame interval | 46.67 ms | 80.55 ms |
| Simulation including pose publication | 35.74 ms | 61.65 ms |
| CPU render including presentation | 10.90 ms | 18.78 ms |
| Surface acquisition | 0.034 ms | 0.049 ms |
| Primary voxel GPU pass (56 delayed samples) | 1.25 ms | 6.56 ms |

Raw data is in `baseline/`. This is one observation in a dirty, concurrently edited
checkout, not a paired result or an accepted optimization. The other attempted
scenarios did not produce complete reports. The user subsequently requested no
long native GPU runs. The 120 FPS goal remains unachieved; the first measured
bottleneck is CPU simulation/publication, whose finer stage counters were added
after this observation. The draw/upload cost of incremental chunk publication
has not yet been isolated. No unmeasured performance rewrite is enabled.

The main client suite passed 37 tests before the final extra report test. The
street suite passed 18 tests, ignored one, and failed its existing geometry
assertion (`every frontage owns a distinct facade`: 48 versus expected 24).
Three stale test initializations were updated for the existing `Ageing` API;
the geometry assertion was preserved. Python comparison tests passed three cases.
Workspace strict Clippy fails in existing renderer code (14 diagnostics), and
workspace formatting fails on existing files. Full workspace tests and complete
GPU/visual qualification are not claimed.
