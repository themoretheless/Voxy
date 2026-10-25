# Bracketed particle p99 confirmation, 2026-10-26

New, independent confirmation of the unchanged 120 Hz offscreen direct-upload path. It does not replace the previous campaign's failed A/A. Renderer code and practical limits are unchanged. Strict **relative** qualification requires a complete matrix, correct byte-exact images/GPU validation, valid pacing, a single pinned executable, BOTH A/A controls stable, and all upload ratio bounds passed. It is not an absolute 120-FPS or display-Present certificate.

## Preregistered experiment

`criteria.json` and the hashed `qualify.py` precede collection. Exactly four runs: control-r1 → upload-r1 → upload-r2 → control-r2. This brackets upload comparisons with endpoint controls rather than selecting a replacement for the old failed control. No idle admission, competing-task pauses, retries, noisy-run rejection or optional stopping. All observed clients/AGX samples are retained.

Each run: one warm-up pair, 18 measured alternating pairs, two 48-frame blocks per mode. Each group has 36 pairs / 3456 frames per mode. Two deterministic 1080p byte-exact image oracles and GPU error scopes validate each run. Paced queue depth one and minimum half-period CPU start separation are checked against FULL/PACE records, with one-to-one frame keys and fixed sample counts.

A/A tolerances, unchanged: CPU and completion medians ±10%; GPU-service/cadence medians ±5%; cadence p95 ±5%; cadence and CPU-start p99 ±10%. Upload median upper two-sided 95% ratio CI ≤1.05; hardware completion-cadence p99 ≤1.10. Bootstrap: 10,000 resamples of 36 paired ratios, seed 20261026. Original-grid and assigned-deadline misses remain visible.

Frozen renderer executable SHA256: `faa40b0726c3283bbb05d7e0b168f634bdd4f621c9a8aaf634315e17524c16c3`. It was built from the clean checkout used for 4785e251d; the current benchmark source still matches its measured source digest. No renderer change requires a new binary. Current validation reran **349 default tests** and both physical Metal fluid generated/snapshot parity tests; all passed. Seven qualifier tests exercise successful qualification and rejection of failed controls, exceeded p99, missing/duplicate controls, wrong hashes, reduced samples and invalid correctness/scheduling.

## Result: strict relative qualification PASSED

`session/summary.json` records `complete=true`, `controls_stable=true`, `relative_p99_qualified=true` and `qualified=true`. All four prescribed runs completed on the pinned binary. Both endpoint controls and all five upload confidence bounds passed; no run or frame was replaced or dropped.

### A/A endpoint controls

Values below are after/before metric ratios; the final column is the unchanged allowed deviation from 1.

| Metric | Before-upload control | After-upload control | Allowed |
|---|---:|---:|---:|

| cpu_p50 | 0.988905 | 0.985413 | ±10% |
| gpu_p50 | 1.002151 | 1.006547 | ±5% |
| completion_p50 | 0.994382 | 1.001621 | ±10% |
| gpu_gap_p50 | 0.999939 | 1.000302 | ±5% |
| gpu_gap_p95 | 0.996373 | 0.998716 | ±5% |
| gpu_gap_p99 | 0.995734 | 0.990180 | ±10% |
| start_gap_p99 | 0.999622 | 0.999159 | ±10% |

### Direct shared upload versus slice upload

36 alternating pairs / 3456 frames per mode. Absolute columns are medians of per-pair statistics in microseconds; paired ratio is the median of paired ratios, not the quotient of independently pooled medians.

| Metric | Before, µs | After, µs | Paired ratio | Two-sided 95% CI | Upper limit |
|---|---:|---:|---:|---|---:|
| cpu_p50 | 295.573 | 296.281 | 0.996910 | [0.962730, 1.033015] | 1.05 |
| gpu_p50 | 1425.604 | 1433.073 | 1.003353 | [0.997262, 1.011559] | 1.05 |
| completion_p50 | 2200.979 | 2202.677 | 1.003244 | [0.994016, 1.012120] | 1.05 |
| gpu_gap_p50 | 8347.927 | 8350.459 | 0.999608 | [0.998710, 1.000669] | 1.05 |
| gpu_gap_p99 | 10151.617 | 10018.826 | 0.997661 | [0.961314, 1.032269] | 1.10 |

The p99 cadence point ratio is **0.997661**; 95% CI **0.961314–1.032269**. Its upper bound is **+3.23%**, below +10%. Thus the remaining relative p99 criterion is satisfied in this new confirmation. GPU completion latency and GPU-service medians also remain within their +5% bounds.

### Deadline and load limits

- control: catch-up starts 0→0; assigned-deadline misses 4→0 / 3456; original-grid misses 10→0 / 3456.
- upload: catch-up starts 0→0; assigned-deadline misses 448→418 / 3456; original-grid misses 1196→1193 / 3456.

Both controls bound this campaign; they do not prove constant external load on every frame. The upload group's roughly 10 ms p99 and retained deadline misses prevent an absolute ≤8.33 ms / 120-FPS claim. Qualification is **relative offscreen noninferiority under observed load**, with no isolation or display scanout claim. Hardware GPU-end cadence, GPU-service time and host callback completion remain separate metrics. No normalized/clipped tail or noise-filtered subset is used.

The [previous campaign](../2026-10-26-particle-pacing/README.md) remains failed at its historical A/A p95 check. Its raw results and `relative_p99_qualified=false` were not edited. This is a new fixed confirmation, not replacement of the old failed control or pooling until a favourable result. No implementation change or threshold adjustment was made between these campaigns.

## Reproduction

Use the pinned executable, or rebuild the matching source and establish a new binary identity before starting a different campaign. This confirmation intentionally rejects a changed executable.

```sh
python3 docs/architecture-performance/2026-10-26-particle-p99-qualification/test_qualification.py
python3 docs/architecture-performance/2026-10-26-particle-p99-qualification/qualify.py \
  --binary /tmp/voxy-particle-pacing-20261026-final --output /absolute/path/to/new-session
python3 docs/architecture-performance/2026-10-26-particle-p99-qualification/qualify.py \
  --output /absolute/path/to/new-session --summarize-only
```

Frame logs and process/AGX observations are gzip-compressed losslessly. Plain/compressed logs share the same parser; recalculation from compressed traces reproduces the entire summary exactly. Fixed inputs, binary digest, source digests, unit/Metal/default-test logs and the complete ordered manifest are retained. No active task was paused or messaged for this confirmation.
