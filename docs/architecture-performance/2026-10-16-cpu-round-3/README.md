# CPU palette publication, round 3 — 16 October 2026

## Retained changes

- The fixed-step actor path reuses its combined palette across ordinary frames
  and catch-up ticks. Companion joints append directly into this buffer instead
  of allocating a world-space palette, transforming it into another vector, then
  copying it into the combined palette. The actor inverse is computed once per
  tick. Matrix multiplication order is unchanged. Renderer publication still
  occurs once after the fixed-step loop and retains its validation.
- Last-presented pose admission reuses the previous successful validation only
  for immutable meshes with at least 1,024 vertices and byte-identical model and
  complete joint palette. A changed pose follows the existing admission path.
  Repeated publication still replaces the generation token, so stale prepared
  frames remain rejected. Reset clears the reusable pose. No geometry or
  simulation accuracy changes are involved.

The first change removes allocation/copy work during simulation pose preparation;
this round does not change the physics integrator or behavior lifecycle.
The palette retains its peak capacity (64 bytes per matrix); temporal reuse uses
only the existing last-presented pose and adds no second palette/vertex cache.

## Paired CPU measurements

Two complete series of five pairs, same executable/thread per subsystem, with
before/after order reversed every pair. Both paths execute the same compiled
geometry and simulation fixture. Binaries were copied out of the shared Cargo
output before measurements; hashes are recorded in each `summary.json`.
No window, surface, device, or GPU workload is created by these benchmarks.

| Operation | Before CPU p50 | After CPU p50 | Change |
| --- | ---: | ---: | ---: |
| Combined palette, actor prefix + dog | 1.630 µs | 0.980 µs | −39.9% |
| Combined palette, actor prefix + dog + cat | 3.334 µs | 1.865 µs | −44.1% |
| Repeat admitted pose, synthetic 100k vertices / one joint | 883.438 µs | <0.1 µs | Avoids the vertex scan |
| Control: changing pose, same 100k mesh | 884.542 µs | 883.521 µs | −0.12% |
| Control: changing pose, three vertices | 0.04956 µs | 0.05046 µs | +1.80% |

Numbers are medians of ten per-phase p50 values, **not frame p99**. The tiny
repeated-pose time is close to timer resolution; the exact logged value of
0.042 µs should not be interpreted as a general publication latency. Larger
palettes add byte-comparison work. No application-wide speedup is inferred.

Palette phases have 34 samples, discard three warmups, and batch 64 preparations
per sample. Both companions first execute 20 real fixed ticks in the same seeded
world. The reference uses the original allocating palette path; the candidate
retains capacity and appends directly. Output bytes are compared outside timing.
History phases use the original `presented` implementation as a test-only oracle,
31 samples after three warmups, with 256 operations per sample for tiny meshes.
The large changing-pose control alternates model translations each call.

The first history series had severe control outliers (some per-pair ratios up to
3.1), while its repeated-pose result and both palette cases were consistent.
The full series was repeated and **all ten pairs retained**, including outliers.
Combined control medians are within 3%, but these measurements cannot qualify
frame tails or the four-scenario p99 acceptance gate. `pairs.csv`, raw logs,
`repeat/`, and `combined-pairs.csv` preserve the evidence.

## Verification

- Animal/gait/fur tests: 10 passed, three manual benchmarks ignored. New regression
  checks byte-identical palettes for both species, initialized/uninitialized coat,
  moving simulation, nonidentity actor transform, prefix preservation and stable
  allocation on reuse. Existing rollback/recovery and fur attachment tests pass.
- Skinned/LOD/history CPU suites: 15 passed, three manual benchmarks ignored.
  New history regression checks valid changed/repeated poses against the original,
  stale prepared-frame rejection, invalid palettes, overflowing positions,
  preservation of last good pose and reset/re-admission.
- CPU report collectors: four tests passed, including rejection of incomplete,
  duplicate and nonfinite input.
- Release client tests compile the real fixed-step publication path. A separate
  development compile check and final history regression receipt are saved here.
- Workspace formatting is blocked by existing/concurrently edited files;
  strict client Clippy stops at 15 diagnostics in renderer files outside this
  round's changes. Full logs are retained. `git diff --check` passes.

GPU smoke, visual comparisons, long GPU measurements and full workspace tests
were not run in this CPU-only round. The 120 FPS / 8.33 ms p99 goal remains
unqualified. Remaining measured work includes ~0.884 ms for admitting a changed
100k-vertex temporal pose in this synthetic fixture; palette preparation with
both companions is now ~1.87 µs, so it is a small absolute frame-budget item.
The next useful investigation is changed-pose temporal admission and the cost
of actual simulation/extraction under a short headless frame trace.

Reproduce after building release test executables (use the actual returned paths,
copy them outside the shared build directory if other builds are active):

```sh
python3 tools/performance/paired_palette_history_cpu.py \
  --app-tests /path/to/voxy_app-test-executable \
  --render-tests /path/to/voxy_render-test-executable \
  --output /tmp/voxy-palette-history-pairs
```
