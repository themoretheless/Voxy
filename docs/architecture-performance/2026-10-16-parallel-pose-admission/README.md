# Parallel render-pose admission (16 October 2026)

## Finding

The 15 October `motion` baseline showed CPU simulation including pose publication
dominating the frame (p50 35.7 ms). CPU micro-timings of the simulation stages
on the main-app companion setup (release, same host):

| Stage (per fixed tick) | Time |
| --- | ---: |
| Companion tick (physics + gait + fur), each | 17–125 µs |
| Gait update | 0.3–0.7 µs |
| Fur coat step | 16–56 µs |
| Companion palette / shadow spheres | 1–3 µs / <0.5 µs |
| `SkinnedMesh::validate_render_pose`, actor + dog + cat mesh (1,277,328 vertices) | **28–51 ms** |

`Renderer::update_skinned_pose` runs `validate_render_pose` once per presented
frame. The companion mesh has 283,544 distinct influence sets, and only 49% of
adjacent vertices share them, so the coherent cache helps little. The full
CPU admission of every vertex therefore accounts for most of the measured
simulation/publication time. Animal simulation itself is in the microseconds.

## Change

`validate_render_pose` keeps its exact semantics but splits meshes of at least
131,072 vertices into contiguous chunks (at least 32,768 vertices each, at most
`available_parallelism` chunks) on scoped threads. Each chunk runs the unchanged
sequential admission (`admit_render_vertices`, coherent or plain path). Results
are combined in chunk order, so the returned error is the one the sequential
vertex-order scan would return. Smaller meshes keep the single-threaded path.
No GPU write happens before admission succeeds, as before.

## Paired measurement

Five alternating before/after runs of saved release test binaries, measuring the
real companion mesh, mean of 20 admissions per run:

| Run | Before | After |
| --- | ---: | ---: |
| 1 | 47.3 ms | 8.3 ms |
| 2 | 43.8 ms | 19.1 ms |
| 3 | 51.1 ms | 8.5 ms |
| 4 | 47.5 ms | 9.9 ms |
| 5 | 38.8 ms | 9.8 ms |
| **Median** | **47.3 ms** | **9.8 ms (0.21×)** |

The host carried a load average of about 27 from unrelated compilers during the
measurement, so absolute values are noisy. The alternating pairs still keep the
ratio meaningful. This is an isolated CPU measurement, **not frame p99**. A
paired native run (`tools/performance/paired_frames.py`) is still required
before claiming a frame-rate improvement or the 120 FPS target.

## Validation

- `cargo test --release -p voxy_render --lib skinned`: 13 passed, 1 ignored (GPU),
  including the new `parallel_admission_matches_sequential_first_error`. That test
  covers chunked meshes on both paths, an early error that must win over a
  different later-chunk error, a late-only error and a valid pose.
- `cargo test --release -p voxy_app --bin voxy_app animals`: 7 passed.
- `cargo test --release -p voxy_model --test skinned_normal_gpu`: 1 passed, 2 ignored.
- `rustfmt --check` passes on the changed file.

## Next candidates (not implemented)

- Avoid re-admitting the whole mesh every frame: fur guide joints only translate
  bones (`Mat4::from_translation(delta) * bone`), so per-joint certificates could
  replace the per-vertex scan.
- Move skinning admission off the frame thread or onto the GPU entirely.

## Round 2: certified influence groups (16 October 2026)

The main client never enables skinned motion or skinned LOD, so the only per-frame
per-vertex pass is `validate_render_pose`. `SkinnedMesh::new` now groups meshes of
at least 131,072 vertices by identical `(joints, weights)`, keeping a group only
when influence sets repeat at least twice on average. Each group stores its largest
absolute position component and whether any nonzero authored normal has an
inadmissible magnitude. The real companion mesh has 1,273,902 vertices and
283,766 groups.

For each pose, every group is **certified** from its blended model-times-skin
matrix, without reading vertices:

- position: finite matrix, and `max|W| * (3 * max|p| + 1) <= f32::MAX / 4`, so no
  transformed position can overflow;
- normal: the existing `NormalTransform::new` succeeds, and the scaled determinant
  is at least 1/1024. With scaled entries at most 1, the transformed normal then has
  a max-norm of at least about 1.9e-4, far above the rounding error (below 1.5e-5),
  so it cannot round to zero or become nonfinite.

If every group is certified, the pose is admitted. Otherwise the previous exact path
runs (parallel per-vertex scan) and returns the same first error as before.
Certification is conservative, never semantic: it can only skip work that would
have succeeded.

Rejected alternative: a per-group table of matrices plus a per-vertex lookup scan
was 25% faster single-threaded, but gave nothing at 16 threads (2.57 vs 2.60 ms),
because the vertex scan is memory-bound.
Moving admission off the frame thread was not done: the remaining cost is about
1.1 ms, and deferring admission would break the "no GPU write before validation"
guarantee of `update_skinned_pose`.

### Paired measurement

`cpu_companion_tick_and_pose_bench` (existing `voxy_app` benchmark, real mesh,
median of 12 timed admissions per run), 7 alternating runs of saved release test
binaries (`before` = parallel per-vertex scan from round 1):

| | Before | After |
| --- | ---: | ---: |
| Median of run medians | 2158 µs | 1128 µs (0.52×) |

Raw values are in `paired-certified.csv`. The host load average was about 17 from
unrelated work. Compared with the original sequential scan from the first table
(about 47 ms under heavier load, 15 ms single-threaded on a quieter host), admission
is now roughly 1.1 ms. Companion tick checksums are unchanged before/after.

### Validation

- `grouped_admission_matches_per_vertex_scan` compares against the per-vertex oracle
  for scattered (non-adjacent) influence repeats, zero normals, singular,
  near-singular (certification fallback), reflected and overflowing poses, a late
  overflowing vertex, worker counts 1/3/8, and shared index views. It asserts that
  certification is sound and that all three branches are exercised (certified
  valid, uncertified valid, uncertified invalid).
- `voxy_render` skinned: 14 passed, 2 ignored. `voxy_app` animals: 9 passed,
  2 ignored (manual benchmarks). `voxy_model` `skinned_normal_gpu`: 1 passed,
  2 ignored. `rustfmt --check` passes; Clippy reports nothing in `skinned.rs`.

Still not claimed: frame p99 or 120 FPS. That needs a native paired run with
`tools/performance/paired_frames.py`.
