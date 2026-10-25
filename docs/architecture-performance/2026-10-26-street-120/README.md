# Street full-frame 120 FPS qualification — target not achieved

2026-10-26, Apple M4 Max / Metal, observed desktop load. **No renderer optimization from this campaign is enabled in production.** The absolute acceptance criterion is full GPU-completed frame p99 ≤ **8.33 ms**, not a relative upload improvement. The tested candidates do not satisfy it.

## Workload and clocks

Frozen source: `fde63f9201b589db922e9c30ee73cda40fbe962b`. Physical and internal render sizes are 1920×1080, Temporal AA, original geometry (3,544,272 triangles / 119,909 instances), original materials and HDR environment, all effects enabled for qualification. Physics, weather and water retain their fixed-step simulation. No other task or application was paused; no idle-GPU admission or retry policy was used.

The old offscreen mode advances 0.25 simulation seconds, i.e. 30 physics ticks, per rendered frame. That stress replay cannot represent one rendered frame per 120 Hz simulation tick. The **measurement-only** [harness patch](qualification-harness.patch) advances exactly 1/120 second per completed frame and labels that clock explicitly. This is not an engine performance optimization. The primary checkout's concurrently edited application sources were not overwritten.

Each full run warms up 180 frames, then renders one complete 20-second movement cycle: **2,400 measured frames and 2,400 physics ticks**. Each frame waits for its own GPU submission, including postprocessing. The primary p99 comes from untrimmed `completed_frame_samples_seconds`, clock `current_completed_frame`. GPU timestamp service time is reported separately. Stage intervals can overlap on Metal and must not be summed as exclusive costs. These are offscreen completion measurements, not display scanout or a paced 120 Hz presentation certificate.

## Bottleneck localization and rejected candidates

The short effect-mask sweep is diagnostic only, not an accepted quality setting or a paired causal estimate. With all effects, GPU mean was 13.95 ms; disabling GI gave 6.49 ms, whereas individually disabling SSR, AO or sun shadows gave 13.25, 13.01 and 12.56 ms. Physics averaged approximately 0.08 ms per rendered tick. This identifies GI as the first GPU optimization target. Raw masks and results are retained in `profile-fx-*` and [diagnostic-summary.json](diagnostic-summary.json).

The subsequent frozen-binary probes tested early GI evaluation, ray-refinement caching, compute GI, compute GI+SSR, depth-bound bypass and projection hoisting. They did not bring full frames near 8.33 ms. External load also changed: an unchanged baseline moved from 15.37 to 20.21 ms median, with **identical HDR output**. Those short probes must not be treated as qualified speedup comparisons.

The candidate selected for the full paired campaign skips GI and probe sampling only when `diffuse * (1 - transmission)` is exactly zero. It preserves all nonzero contributions, original ray steps and refinements. See [zero-indirect.patch](zero-indirect.patch). Both candidate HDR captures match the baseline **bit for bit** over 6,220,800 linear RGB float components. However, its dynamic branch has an execution cost even when no work is skipped. The complete paired result rejects its promotion.

[Projection hoisting](rejected-clip-projection.patch) changes Float32 rounding and ray decisions: 361,672 HDR components differ, maximum absolute linear HDR error 0.75. It was rejected. Compute GI and refinement caching also fail the strict exact-image oracle. [snapshot-comparison.json](snapshot-comparison.json) records all hashes, finite checks and differences. Local `.pfm` and `.png` captures remain beside the report, excluded from Git to avoid checking in approximately 325 MB of image data.

An [unbuilt transmission specialization](unbuilt-specialized-transmission.patch) was abandoned before benchmarking: the street windows use transmission **0.92**, so a specialization for transmission exactly 1 cannot remove their GI. It has no correctness or performance qualification and is not applied.

## Full paired campaign

The fixed schedule is A/A → A/B → B/A → A/B → B/A → A/B → A/A. A is the original renderer and B the dynamic zero-contribution candidate. There are no overlapping benchmark children, replaced runs, discarded frames or tail normalization. [The manifest](full-cycle/manifest.json) pins binary SHA256 hashes and records every run and its wall-clock interval; per-run process snapshots retain observed competing CPU work.

Acceptance rules were fixed before this series: A/A median and p99 within ±10%, GPU mean within ±5%; upper paired p99 95% bootstrap ratio ≤1.05; **every candidate full-cycle p99 ≤8.33 ms**. Bootstrap resamples the five paired run ratios, not individual correlated frames: 10,000 samples, seed 20261026. The small five-pair campaign cannot bound all future desktop interference.

Final numerical results are in [qualification.json](qualification.json). The five before/after p99 pairs in milliseconds are:

| Pair | Original | Zero-contribution candidate |
|---|---:|---:|
| A/B 0 | 17.442 | 18.977 |
| B/A 1 | 17.750 | 18.120 |
| A/B 2 | 18.447 | 18.097 |
| B/A 3 | 17.630 | 17.597 |
| A/B 4 | 17.768 | 18.044 |

The paired geometric p99 ratio is **1.020071**, 95% CI **[0.994673, 1.054777]**. Its upper bound exceeds 1.05. The absolute threshold fails in all five candidate runs. No 120 FPS or nonregression claim is made; the candidate patch remains an artifact rather than a production change.

Both A/A controls pass. Start ratios (second/first): median 0.983380, p99 0.959648, GPU mean 1.008640. End ratios: median 0.989923, p99 0.997990, GPU mean 0.991688. Stable controls do not turn the failed A/B result into a win.

## Correctness and reproduction

The candidate renderer passes **349 default renderer tests** (79 explicitly ignored) and the qualifier passes **8 tests**, including rejection of altered physics replay, missing controls, reduced quality, shortened routes and quantiles inconsistent with raw samples. All measured replays have the same `camera-weather-water-v2` BLAKE3 digest. Actual HDR comparison is separate from this CPU state hash. No rejected candidate is presented as a validated runtime improvement.

Build the clean source plus the harness overlay for A, then additionally apply `zero-indirect.patch` for B. Freeze the executables; do not build during the campaign. Point `--materials` to the existing original-street material/HDR directory. The material assets are not duplicated into the checkout.

The initial missing-assets failure (`baseline-stress.log`) and the successful old 30-tick stress replay (`baseline-stress-assets.log`) are retained as diagnostics, not qualifying runs. Use `VOXY_STREET_MATERIALS`, not `--assets`, for the original street material root. The successful final harness build log and `frozen-binaries.json` identify the measured executables; the earlier no-op harness build log is not build evidence.

```sh
python3 run_pairs.py --before /absolute/before --after /absolute/after \
  --materials /absolute/assets/environments/original-street/materials \
  --output /absolute/new-campaign
python3 analyze_pairs.py /absolute/new-campaign
python3 -m unittest discover -s . -p 'test_*.py'
```

The runner refuses to overwrite an existing campaign. The analyzer verifies full-cycle sample counts and quantiles against raw completed-frame data and rejects failed controls independently of absolute latency. This campaign leaves the larger 120 FPS goal **unfinished**; achieving it requires a substantially faster GI path with preserved rendering correctness.
