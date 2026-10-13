# Frame rate of the voxel app (2026-10-13)

Headless harness: `xvfb-run -a -s "-screen 0 640x360x24" target/release/voxy_app --autopilot`
on the container's software Vulkan driver (llvmpipe, LLVM 20, four vCPUs). The autopilot
runs a fixed 600 ticks at 60 Hz, so one run is 10 s of real time and the presented frame
count is the frame rate.

## Startup

The app did not start at all on this driver: `Renderer::new` requested
`EXPERIMENTAL_RAY_QUERY` whenever the adapter advertised it, and wgpu 30 refuses that
feature unless the device opts into experimental features (an `unsafe` call the render
crate denies). Every adapter that advertises ray query (llvmpipe, current desktop GPUs)
failed device creation with `ExperimentalFeaturesNotEnabled`. The request is removed;
`has_ray_query` still reports the capability for devices created with the opt-in (the ray
probe crate does that). After the fix the autopilot passes.

## Where the frame goes

| measurement | result |
| --- | ---: |
| 640×360, surface default (fifo) | 835 / 811 / 840 frames per 10 s |
| 1280×720, surface default | 850 frames per 10 s |
| 640×360, `--present-mode immediate` | 897 |
| 640×360, `--present-mode mailbox` | 919 |
| callgrind, whole process | app code 3.3% of instructions; llvmpipe + LLVM JIT + JIT'd code 95% |
| strace `-c`, whole process | 97% of syscall time in `futex` (main thread waiting on driver threads) |

Probes of the driver bound:

| probe | frames per 10 s |
| --- | ---: |
| 160×90 window | 814 |
| 640×360, `LP_NUM_THREADS=1` | 437 |
| 640×360, `LP_NUM_THREADS=2` | 743 |
| 640×360, four driver threads | 811 to 882 |

The count does not depend on resolution but does scale with the driver's thread count,
so the per-frame cost is the software driver's fixed per-pass work, not pixels and not
the app.

The frame rate is set by the software driver, not by the app: the app's own code is a few
percent of the instructions, resolution barely matters, and presentation pacing moves the
count by less than 10%. On a hardware GPU the same loop will be bounded by vertical sync
unless an uncapped present mode is chosen, which is what the new option provides.

## Per-tick work in the frame handler

A second callgrind run collected only inside the app's window-event handler (40 frames,
599 fixed ticks, 7 water ticks) and ranked the app's own per-frame work: the skinned pose
validation that runs every fixed tick (30%), the water planner's sampled-cell lookups
(25%), then the character step, the vehicle step and the world commit.

- `SkinnedMesh::validate_render_pose` multiplied the model matrix by each vertex's blended
  skin twice (position test, normal test); the product is computed once and both tests
  read it, bitwise the same result.
- The water planner's sampled-cell map was a B-tree keyed by voxel position although only
  lookups reach it: the plan sorts its writes and collects touched chunks into a set, so
  the map's order never reaches the result. It is now a hash map with the workspace's
  deterministic hasher.

Paired (same harness, same binary flags; callgrind instruction counts are deterministic,
frame counts are two runs each):

| measurement | before | after | ratio |
| --- | ---: | ---: | ---: |
| frame handler, instructions over the run | 141.3 M | 106.3 M | 0.75 |
| `validate_render_pose` (599 calls) | 43.4 M | 39.3 M | 0.91 |
| `step_liquid` (7 calls) | 47.6 M | 16.6 M | 0.35 |
| `Sampler::current` | 36.0 M | 9.4 M | 0.26 |
| `voxel_queries` water_tick (5 rounds, before binary from `be884bf7`) | 3.30 ms | 1.42 ms | 0.43 |
| presented frames per 10 s, 640×360, surface default | 811 / 835 / 840 / 845 / 882 | 1151 / 1213 | ~1.40 |

`voxel_queries` checksums (raycast, sweep, water) are identical before and after;
physics_voxel tests 43 and voxy_render lib tests 211 pass.

## What changed

- `GraphicsOptions::present` (`PresentPreference::{Default, Fifo, Mailbox, Immediate}`)
  applied at surface configuration when the surface supports it; the app parses
  `--present-mode auto|fifo|mailbox|immediate`. Default behaviour is unchanged.
- The ray-query request at device creation is gone (startup fix above).
- One model-times-skin product per vertex in the per-tick pose validation; hashed
  water sampler cells (above).

## Gates

`cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check`
clean; `voxy_app` graphics option parser tests 2 passed; headless autopilot passed in
all three present modes.
