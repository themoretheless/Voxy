# CPU pose admission and strand simulation (15 October 2026)

Two changes, without native windows or long GPU runs:

- Pose admission reuses the model-times-skin matrix and normal cofactor transform
  across consecutive vertices with identical canonical joint indices and UNORM16
  weights. The immutable mesh selects this path when at least half of adjacent
  vertices share influences. Incoherent meshes retain the original loop. Shared
  index/LOD views preserve the same classification. Every vertex still undergoes
  position and authored-normal admission, in the original error order.
- Strand steps copy the current particles into retained scratch storage and rotate
  the previous/current/scratch buffers only after all computed positions are
  finite. This removes the per-step allocation and previous-position copy without
  changing the solver, timestep, iteration count, damping, or collision order.
  Failure preserves the published positions and previous state. The tradeoff is
  one retained scratch allocation (24 bytes per particle, plus a Vec header).

## Paired CPU measurements

Five alternating before/after runs of separate saved release binaries on the same
Apple M4 Max host. Each pose run reports the median of 30 timed admissions after
three warmups. Each strand run reports the median of 180 steps after 20 warmups.
The table is the median of the five per-run medians; it is **not frame p99**.

| Operation | Before | After | After/before |
| --- | ---: | ---: | ---: |
| Admit 100,000 vertices, influence runs of 64 | 2717.58 µs | 647.00 µs | 0.238 |
| Admit 100,000 vertices, alternating influences | 2715.29 µs | 2728.21 µs | 1.005 |
| Step 96 strands, 3 particles each | 34.58 µs | 20.50 µs | 0.593 |
| Step 96 strands, 32 particles each | 397.00 µs | 338.92 µs | 0.854 |

The pose fixture uses two nonzero weights and 16 rotated joint matrices. The
strand fixture uses 8 constraint iterations, stiffness 650, damping 0.0001,
a moving root and gravity/wind; colliders are empty. Trajectory checksums match
before/after for both strand lengths in all five pairs. Background compilers from
other work were observed on this shared host, so small differences (including
+0.5% for alternating weights) are treated as noise.

`paired-final.csv` retains the per-run data; `summary.json` includes binary SHA256
hashes. `paired.csv` is the first candidate measurement: its incoherent-path
regression caused restoration of the original normal helper before the final
comparison. Only `paired-final.csv` describes the retained implementation.

These are isolated CPU measurements. Full companion collision/gait performance,
application frame p99, and the 120 FPS target have not been requalified. No GPU
or visual performance improvement is claimed.

## Validation

- Renderer skinned suites: 12 passed, 1 ignored. New oracle comparison covers
  coherent/incoherent layouts, rotations, zero normals, singular, reflected,
  anisotropic and extreme scales, nonfinite palettes, and shared index views.
  Prepared normals match the original helper bit for bit, including errors.
- Physics strand-related library suites: 20 passed, 3 ignored.
- Strand integration suite: 3 passed (atomic rejection, wind/restoration,
  attachment inertia and sphere collision).
- New regressions verify recovery after numeric overflow and reuse of exactly
  the same three allocations across 100 steps.
- Main client compile check passes. Changed Rust sources/examples pass formatting
  and diff whitespace checks. Strict library Clippy still fails on 14 existing
  renderer diagnostics; none point to the changed pose/strand files.

Reproduce the CPU-only workloads:

```sh
cargo run --release -p voxy_render --example cpu_pose_bench
cargo run --release -p physics --example strand_step_bench
cargo test -p voxy_render --lib skinned
cargo test -p physics --lib strand
cargo test -p physics --test strands
```
